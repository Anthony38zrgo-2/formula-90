#![allow(dead_code)]

pub const ABI_VERSION: u32 = 2;
pub const MAX_BLOCK_SAMPLES: u32 = 4096;
pub const MAX_EVENTS_PER_BLOCK: u32 = 512;
pub const BUILD_SOURCE_LEN: u32 = 40;
pub const MAX_CYLINDERS: u32 = 5;
pub const MAX_BANKS: u32 = 2;
pub const MAX_CHANNELS: u32 = 2;
pub const MIN_CONFIG_SIZE: u32 = 24;

pub const ERR_OK: i32 = 0;
pub const ERR_NULL_ARGUMENT: i32 = 1;
pub const ERR_INVALID_ABI: i32 = 2;
pub const ERR_BUILD_MISMATCH: i32 = 3;
pub const ERR_UNSUPPORTED_SAMPLE_RATE: i32 = 4;
pub const ERR_BLOCK_TOO_LARGE: i32 = 5;
pub const ERR_EVENT_OVERFLOW: i32 = 6;
pub const ERR_NONFINITE_INPUT: i32 = 7;
pub const ERR_INVALID_STATE: i32 = 8;
pub const ERR_NOT_SUPPORTED: i32 = 9;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DspConfig {
    pub struct_size: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub simulated_cylinders: u8,
    pub bank_count: u8,
    pub flags: u8,
    pub half_block_offset_deg: f32,
    pub idle_rpm: f32,
    pub max_rpm: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DspEvent {
    pub sample_offset: u32,
    pub cylinder: u8,
    pub bank: u8,
    pub reserved: u16,
    pub crank_phase_deg: f32,
    pub pressure: f32,
    pub pressure_derivative: f32,
    pub energy: f32,
    pub cycle_variation: f32,
    pub event_pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DspEventBlock {
    pub stream_block_id: u64,
    pub event_count: u32,
    pub block_samples: u32,
    pub events: [DspEvent; 512],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DspControls {
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
    pub tc_cut: f32,
    pub master_gain: f32,
    pub lod: u8,
    pub bypass: u8,
    pub reserved0: u8,
    pub reserved1: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DspDiagnostics {
    pub struct_size: u32,
    pub blocks_processed: u32,
    pub samples_processed: u32,
    pub events_received: u32,
    pub events_dropped: u32,
    pub nonfinite_outputs: u32,
    pub nonfinite_inputs: u32,
    pub rt_violations: u32,
    pub last_error_code: u32,
    pub reserved: u32,
}

const _: () = {
    assert!(std::mem::size_of::<DspConfig>() == 24);
    assert!(std::mem::align_of::<DspConfig>() == 4);
    assert!(std::mem::size_of::<DspEvent>() == 32);
    assert!(std::mem::align_of::<DspEvent>() == 4);
    assert!(std::mem::size_of::<DspEventBlock>() == 16 + 512 * 32);
    assert!(std::mem::align_of::<DspEventBlock>() == 8);
    assert!(std::mem::size_of::<DspControls>() == 24);
    assert!(std::mem::align_of::<DspControls>() == 4);
    assert!(std::mem::size_of::<DspDiagnostics>() == 40);
    assert!(std::mem::align_of::<DspDiagnostics>() == 4);
};
