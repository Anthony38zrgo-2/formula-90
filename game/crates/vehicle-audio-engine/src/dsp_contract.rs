//! Allocation-free ABI v2 event transport.
//!
//! This module deliberately contains no crank clock, firing order, jitter or
//! pressure model. `CylState` is the sole mechanical authority and records each
//! real combustion through [`EventBuilder::record_physical`]. The second bank
//! is derived from that same event using the configured half-block offset and
//! an absolute sample deadline.

pub const F90_DSP_ABI_VERSION: u32 = 2;
pub const MAX_BLOCK_SAMPLES: usize = 4096;
pub const MAX_EVENTS_PER_BLOCK: usize = 512;
const MAX_PENDING_EVENTS: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct F90DspEvent {
    pub sample_offset: u32,
    pub cylinder: u8,
    pub bank: u8,
    pub reserved: u16,
    pub crank_phase_deg: f32,
    pub pressure: f32,
    /// Pressure units per second: `delta_pressure * sample_rate`.
    pub pressure_derivative: f32,
    pub energy: f32,
    pub cycle_variation: f32,
    pub event_pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct F90DspEventBlock {
    pub stream_block_id: u64,
    pub event_count: u32,
    pub block_samples: u32,
    pub events: [F90DspEvent; MAX_EVENTS_PER_BLOCK],
}

impl Default for F90DspEventBlock {
    fn default() -> Self {
        Self {
            stream_block_id: 0,
            event_count: 0,
            block_samples: 0,
            events: [F90DspEvent::default(); MAX_EVENTS_PER_BLOCK],
        }
    }
}

#[derive(Clone, Copy, Default)]
struct PendingEvent {
    absolute_sample: u64,
    event: F90DspEvent,
}

/// Preallocated packet sink. The historical name is retained to minimize API
/// churn, but this type no longer builds or predicts mechanical events.
pub struct EventBuilder {
    block: Box<F90DspEventBlock>,
    pending: [PendingEvent; MAX_PENDING_EVENTS],
    pending_count: usize,
    absolute_block_start: u64,
    next_block_id: u64,
    overflowed: bool,
}

impl EventBuilder {
    pub fn new(_sample_rate: f64, _seed: u64) -> Self {
        Self {
            block: Box::new(F90DspEventBlock::default()),
            pending: [PendingEvent::default(); MAX_PENDING_EVENTS],
            pending_count: 0,
            absolute_block_start: 0,
            next_block_id: 0,
            overflowed: false,
        }
    }

    pub fn begin_block(&mut self, block_samples: u32) {
        self.block.stream_block_id = self.next_block_id;
        self.block.block_samples = block_samples;
        self.block.event_count = 0;
        self.overflowed = false;

        let end = self.absolute_block_start + block_samples as u64;
        let mut write = 0usize;
        for read in 0..self.pending_count {
            let mut pending = self.pending[read];
            if pending.absolute_sample < end {
                pending.event.sample_offset = pending
                    .absolute_sample
                    .saturating_sub(self.absolute_block_start)
                    as u32;
                self.push_sorted(pending.event);
            } else {
                self.pending[write] = pending;
                write += 1;
            }
        }
        self.pending_count = write;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_physical(
        &mut self,
        sample_offset: u32,
        cylinder: u8,
        crank_phase_deg: f32,
        pressure: f32,
        pressure_derivative: f32,
        energy: f32,
        cycle_variation: f32,
        bank_b_delay: f32,
    ) {
        let base = F90DspEvent {
            sample_offset,
            cylinder,
            bank: 0,
            reserved: 0,
            crank_phase_deg: crank_phase_deg.rem_euclid(720.0),
            pressure,
            pressure_derivative,
            energy,
            cycle_variation,
            event_pad: 0.0,
        };
        self.push_sorted(base);

        let delay = bank_b_delay.max(0.0).round() as u64;
        let absolute_sample = self.absolute_block_start + sample_offset as u64 + delay;
        let mut bank_b = base;
        bank_b.bank = 1;
        bank_b.crank_phase_deg = (crank_phase_deg + 72.0).rem_euclid(720.0);
        let block_end = self.absolute_block_start + self.block.block_samples as u64;
        if absolute_sample < block_end {
            bank_b.sample_offset = (absolute_sample - self.absolute_block_start) as u32;
            self.push_sorted(bank_b);
        } else if self.pending_count < MAX_PENDING_EVENTS {
            self.pending[self.pending_count] = PendingEvent {
                absolute_sample,
                event: bank_b,
            };
            self.pending_count += 1;
        } else {
            self.overflowed = true;
        }
    }

    fn push_sorted(&mut self, event: F90DspEvent) {
        let count = self.block.event_count as usize;
        if count >= MAX_EVENTS_PER_BLOCK {
            self.overflowed = true;
            return;
        }
        let key = (event.sample_offset, event.bank, event.cylinder);
        let mut index = count;
        while index > 0 {
            let prev = self.block.events[index - 1];
            if (prev.sample_offset, prev.bank, prev.cylinder) <= key {
                break;
            }
            self.block.events[index] = prev;
            index -= 1;
        }
        self.block.events[index] = event;
        self.block.event_count += 1;
    }

    pub fn finish_block(&mut self) -> &F90DspEventBlock {
        self.absolute_block_start += self.block.block_samples as u64;
        self.next_block_id = self.next_block_id.wrapping_add(1);
        &self.block
    }

    pub fn overflowed(&self) -> bool {
        self.overflowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_match_c_abi() {
        assert_eq!(std::mem::size_of::<F90DspEvent>(), 32);
        assert_eq!(std::mem::size_of::<F90DspEventBlock>(), 16 + 512 * 32);
        assert_eq!(std::mem::align_of::<F90DspEventBlock>(), 8);
    }

    #[test]
    fn physical_event_and_delayed_bank_cross_block_exactly_once() {
        let mut sink = EventBuilder::new(44_100.0, 0);
        sink.begin_block(256);
        sink.record_physical(255, 3, 432.0, 0.8, 120.0, 0.7, -0.02, 10.0);
        let first = sink.finish_block();
        assert_eq!(first.event_count, 1);
        assert_eq!(
            (first.events[0].sample_offset, first.events[0].bank),
            (255, 0)
        );

        sink.begin_block(256);
        let second = sink.finish_block();
        assert_eq!(second.event_count, 1);
        assert_eq!(
            (second.events[0].sample_offset, second.events[0].bank),
            (9, 1)
        );
        assert_eq!(second.events[0].pressure_derivative, 120.0);
    }

    #[test]
    fn complete_order_is_stable_without_heap_sort() {
        let mut sink = EventBuilder::new(44_100.0, 0);
        sink.begin_block(128);
        sink.record_physical(20, 4, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0);
        sink.record_physical(20, 1, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0);
        let block = sink.finish_block();
        let expected = [(20, 0, 1), (20, 0, 4), (20, 1, 1), (20, 1, 4)];
        for (event, key) in block.events[..block.event_count as usize]
            .iter()
            .zip(expected)
        {
            assert_eq!((event.sample_offset, event.bank, event.cylinder), key);
        }
    }
}
