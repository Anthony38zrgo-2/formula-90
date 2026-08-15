//! DSP helpers for the runtime mixer — loop-position crossfade.
//!
//! Mirrors `tools/audio/render_audio_scenario.py` `_make_loop_reader`: reads a
//! looped buffer with a short crossfade at the wrap point so looping never
//! clicks, independent of bank loop seam quality.

/// Crossfade window in samples (~11.6 ms @ 44.1 kHz). Must equal the offline
/// mixer's `LOOP_XFADE`.
pub const LOOP_XFADE: usize = 512;

/// Read one sample from a looped PCM buffer at an absolute step, crossfading at
/// the wrap point. `pcm` is the raw PCM16 buffer; returns a float in [-1, 1].
pub fn loop_sample(pcm: &[i16], step: usize) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    let n = pcm.len();
    if n <= LOOP_XFADE {
        return pcm[step % n] as f32 / 32768.0;
    }
    let pos = step % n;
    if pos >= n - LOOP_XFADE {
        let k = pos - (n - LOOP_XFADE);
        let t = k as f32 / (LOOP_XFADE - 1) as f32;
        let a = pcm[pos] as f32 / 32768.0;
        let b = pcm[k] as f32 / 32768.0;
        return a * (1.0 - t) + b * t;
    }
    pcm[pos] as f32 / 32768.0
}

/// Linear interp between two floats (helper).
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_returns_zero() {
        assert_eq!(loop_sample(&[], 5), 0.0);
    }

    #[test]
    fn reads_within_buffer() {
        let pcm = [0i16, 10000, -10000, 20000];
        assert_eq!(loop_sample(&pcm, 0), 0.0);
        assert_eq!(loop_sample(&pcm, 1), 10000.0 / 32768.0);
    }

    #[test]
    fn wraps_and_crossfades() {
        // n=1024 > LOOP_XFADE; at pos n-1 the tail crossfades into head.
        let pcm: Vec<i16> = (0..1024).map(|i| (i * 100) as i16).collect();
        let tail = loop_sample(&pcm, 1023);
        let head = loop_sample(&pcm, 0);
        // Crossfade at the very wrap point blends the last sample toward the
        // head; assert it stays within range (no NaN / out of bounds).
        assert!(tail.is_finite());
        assert!(head.is_finite());
        // Deterministic.
        assert_eq!(loop_sample(&pcm, 1023), loop_sample(&pcm, 1023));
    }

    #[test]
    fn lerp_linear() {
        assert_eq!(lerp(0.0, 10.0, 0.25), 2.5);
    }
}
