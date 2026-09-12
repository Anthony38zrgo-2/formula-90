//! Fase 4 — Runtime wrapper that connects the real C++ post-combustion DSP DLL
//! (`f90_audio_dsp.dll`, ABI v2) into the Rust `vehicle-audio-engine`.
//!
//! Design rules (from `instrucciones.md` §4.1):
//! - The DLL is loaded and every symbol resolved **outside** the audio callback.
//! - Before `f90_dsp_create`, the ABI version and the 40-byte BUILD_SOURCE are
//!   verified. Any failure is reported through [`DspRuntimeError`]; nothing panics.
//! - The instance is destroyed exactly once (`Drop`, guarded so it runs even on
//!   unwind).
//! - The per-block entrypoint ([`DspRuntime::process_block`]) performs **no**
//!   DLL loads, no logging, and no heap allocations: the `f90_dsp_event_block`,
//!   the `f90_dsp_controls`, and the L/R output buffers are all preallocated.
//! - This type is intentionally **not** `Send`/`Sync`: it owns a raw C++ instance
//!   pointer and is meant to live on a single audio/render thread. Marking it
//!   `Send`/`Sync` would let the handle escape to other threads where the C++
//!   instance (which is *not* internally locked) could be re-entered. The single
//!   owner-thread assumption is exactly what the runtime already guarantees for
//!   the mixer, so we keep the default (non-`Send`/`Sync`) semantics.

use crate::dsp_contract::{EventBuilder, F90DspEvent, F90DspEventBlock};

use std::ffi::{c_void, CStr, CString};
use std::path::{Path, PathBuf};

// --- ABI constants (must mirror f90_audio_dsp.h) -----------------------------
pub const F90_DSP_ABI_VERSION: u32 = 2;
const MAX_BLOCK_SAMPLES: usize = 4096;
const MAX_EVENTS_PER_BLOCK: usize = 512;
const BUILD_SOURCE_LEN: usize = 40;

// --- Build-source gating -----------------------------------------------------
// `game/BUILD_SOURCE` is the canonical 40-byte git HEAD the C++ DLL was built
// from (see AGENTS.md RUNTIME PARITY). In a canonical build this equals
// `git rev-parse HEAD`. We embed it at compile time and compare against what the
// DLL reports via `f90_dsp_build_source()` / `f90_dsp_check_build_source()`.
const BUILD_SOURCE_RAW: &str = include_str!("../../../BUILD_SOURCE");

fn expected_build_source() -> &'static str {
    let s = BUILD_SOURCE_RAW.trim();
    if s.len() >= BUILD_SOURCE_LEN {
        &s[..BUILD_SOURCE_LEN]
    } else {
        s
    }
}

// --- C-compatible ABI mirror structs ----------------------------------------
// Rust and C++ now declare the same fields and offsets. `process_block` still
// performs an explicit field copy (rather than transmute) so reserved/padding
// bytes are always initialized and ABI drift remains locally auditable.

#[repr(C)]
#[derive(Clone, Copy)]
struct CConfig {
    struct_size: u32,
    sample_rate: u32,
    channels: u8,
    simulated_cylinders: u8,
    bank_count: u8,
    flags: u8,
    half_block_offset_deg: f32,
    idle_rpm: f32,
    max_rpm: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CEvent {
    sample_offset: u32,
    cylinder: u8,
    bank: u8,
    reserved: u16,
    crank_phase_deg: f32,
    pressure: f32,
    pressure_derivative: f32,
    energy: f32,
    cycle_variation: f32,
    event_pad: f32,
}

#[repr(C)]
struct CEventBlock {
    stream_block_id: u64,
    event_count: u32,
    block_samples: u32,
    events: [CEvent; MAX_EVENTS_PER_BLOCK],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CControls {
    rpm: f32,
    throttle: f32,
    load: f32,
    tc_cut: f32,
    master_gain: f32,
    lod: u8,
    bypass: u8,
    reserved0: u8,
    reserved1: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CDiagnostics {
    struct_size: u32,
    blocks_processed: u32,
    samples_processed: u32,
    events_received: u32,
    events_dropped: u32,
    nonfinite_outputs: u32,
    nonfinite_inputs: u32,
    rt_violations: u32,
    last_error_code: u32,
    reserved: u32,
}

// Compile-time layout guarantees so a silent ABI drift fails the build, not the
// runtime.
const _: () = assert!(std::mem::size_of::<CConfig>() == 24);
const _: () = assert!(std::mem::size_of::<CEvent>() == 32);
const _: () = assert!(std::mem::size_of::<CEventBlock>() == 16 + 512 * 32);
const _: () = assert!(std::mem::size_of::<CControls>() == 24);
const _: () = assert!(std::mem::size_of::<CDiagnostics>() == 40);

type Handle = *mut c_void;

type AbiVersionFn = unsafe extern "C" fn() -> u32;
type BuildSourceFn = unsafe extern "C" fn() -> *const std::ffi::c_char;
type CheckBuildFn = unsafe extern "C" fn(*const std::ffi::c_char) -> i32;
type CreateFn = unsafe extern "C" fn(*const CConfig, *mut Handle) -> i32;
type ProcessFn = unsafe extern "C" fn(
    Handle,
    *const CEventBlock,
    *const CControls,
    *mut f32,
    *mut f32,
    u32,
) -> i32;
type ResetFn = unsafe extern "C" fn(Handle) -> i32;
type DiagFn = unsafe extern "C" fn(Handle, *mut CDiagnostics) -> i32;
type DestroyFn = unsafe extern "C" fn(Handle);

#[derive(Clone, Copy)]
struct Fns {
    abi_version: AbiVersionFn,
    build_source: BuildSourceFn,
    check_build: CheckBuildFn,
    create: CreateFn,
    process: ProcessFn,
    reset: ResetFn,
    get_diag: DiagFn,
    destroy: DestroyFn,
}

/// Friendly controls fed to the C++ `process()` (Rust-native view).
#[derive(Clone, Copy, Debug, Default)]
pub struct DspControls {
    pub rpm: f32,
    pub throttle: f32,
    pub load: f32,
    pub tc_cut: f32,
    pub master_gain: f32,
    pub lod: u8,
    pub bypass: bool,
}

#[derive(Debug)]
pub enum DspRuntimeError {
    DllNotFound,
    UnsupportedPlatform,
    LoadFailed(i32),
    SymbolMissing(&'static str),
    AbiMismatch { got: u32, expected: u32 },
    BuildMismatch,
    CreateFailed(i32),
    BlockTooLarge(u32),
    ProcessFailed(i32),
}

/// The single owned connection to one C++ DSP instance.
pub struct DspRuntime {
    library: *mut c_void,
    handle: Handle,
    fns: Fns,
    c_block: Box<CEventBlock>,
    c_controls: CControls,
    l_buf: Vec<f32>,
    r_buf: Vec<f32>,
    last_frames: usize,
}

// SAFETY NOTE: `DspRuntime` wraps a single-threaded C++ instance and is
// intentionally NOT `Send`/`Sync` (see module docs). We do not implement those
// traits; the default (raw-pointer-carrying) semantics keep it confined to one
// thread.

impl DspRuntime {
    /// Load the DLL from an explicit path and verify ABI + BUILD_SOURCE, then
    /// create the instance. Resolves every symbol up front (outside any
    /// callback). Never panics.
    pub fn load(path: &Path) -> Result<Self, DspRuntimeError> {
        #[cfg(windows)]
        {
            Self::load_windows(path)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(DspRuntimeError::UnsupportedPlatform)
        }
    }

    /// Resolve the deployed DLL relative to the crate manifest and a few sibling
    /// candidates. Fails with a clear diagnostic if none exist.
    pub fn load_default() -> Result<Self, DspRuntimeError> {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let candidates: [PathBuf; 4] = [
            // CARGO_MANIFEST_DIR = game/crates/vehicle-audio-engine ->
            // game/addons/formula90s/bin/...
            manifest.join("../../addons/formula90s/bin/f90_audio_dsp.dll"),
            // As literally spelled in the Fase-4 brief (harmless if absent).
            manifest.join("../../../game/addons/formula90s/bin/f90_audio_dsp.dll"),
            // From a crate-local run dir.
            PathBuf::from("game/addons/formula90s/bin/f90_audio_dsp.dll"),
            // Repo-relative fallback.
            PathBuf::from("game/addons/formula90s/bin/f90_audio_dsp.dll"),
        ];
        for c in &candidates {
            if c.exists() {
                return Self::load(c);
            }
        }
        Err(DspRuntimeError::DllNotFound)
    }

    #[cfg(windows)]
    fn load_windows(path: &Path) -> Result<Self, DspRuntimeError> {
        use std::os::windows::ffi::OsStrExt;
        // SAFETY: Win32 DLL loading. `wide` is a valid NUL-terminated UTF-16 path; symbols are
        // resolved from the freshly loaded module and transmuted to the matching
        // `unsafe extern "C"` fn pointer types declared in `Fns`. On every error path the
        // library is freed before returning.
        unsafe {
            let wide: Vec<u16> = std::ffi::OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0u16))
                .collect();
            let lib = LoadLibraryW(wide.as_ptr());
            if lib.is_null() {
                return Err(DspRuntimeError::LoadFailed(1));
            }

            let resolve = |name: &'static str| -> Result<*mut c_void, DspRuntimeError> {
                let c = CString::new(name).map_err(|_| DspRuntimeError::SymbolMissing(name))?;
                let p = GetProcAddress(lib, c.as_ptr() as *const u8);
                if p.is_null() {
                    return Err(DspRuntimeError::SymbolMissing(name));
                }
                Ok(p)
            };

            let fns = Fns {
                abi_version: std::mem::transmute(resolve("f90_dsp_abi_version")?),
                build_source: std::mem::transmute(resolve("f90_dsp_build_source")?),
                check_build: std::mem::transmute(resolve("f90_dsp_check_build_source")?),
                create: std::mem::transmute(resolve("f90_dsp_create")?),
                process: std::mem::transmute(resolve("f90_dsp_process")?),
                reset: std::mem::transmute(resolve("f90_dsp_reset")?),
                get_diag: std::mem::transmute(resolve("f90_dsp_get_diagnostics")?),
                destroy: std::mem::transmute(resolve("f90_dsp_destroy")?),
            };

            // 1) ABI gate.
            let abi = (fns.abi_version)();
            if abi != F90_DSP_ABI_VERSION {
                FreeLibrary(lib);
                return Err(DspRuntimeError::AbiMismatch {
                    got: abi,
                    expected: F90_DSP_ABI_VERSION,
                });
            }

            // 2) BUILD_SOURCE gate.
            let expected = expected_build_source();
            let cexp = match CString::new(expected) {
                Ok(c) => c,
                Err(_) => {
                    FreeLibrary(lib);
                    return Err(DspRuntimeError::BuildMismatch);
                }
            };
            let bs = (fns.check_build)(cexp.as_ptr());
            if bs != 0 {
                FreeLibrary(lib);
                return Err(DspRuntimeError::BuildMismatch);
            }

            // 3) Create the instance with the Fase-4 default config.
            let cfg = CConfig {
                struct_size: 24,
                sample_rate: 44100,
                channels: 2,
                simulated_cylinders: 5,
                bank_count: 2,
                flags: 0,
                half_block_offset_deg: 72.0,
                idle_rpm: 1000.0,
                max_rpm: 15000.0,
            };
            let mut handle: Handle = std::ptr::null_mut();
            let rc = (fns.create)(&cfg, &mut handle);
            if rc != 0 || handle.is_null() {
                FreeLibrary(lib);
                return Err(DspRuntimeError::CreateFailed(rc));
            }

            let c_block = Box::new(CEventBlock {
                stream_block_id: 0,
                event_count: 0,
                block_samples: 0,
                events: [CEvent {
                    sample_offset: 0,
                    cylinder: 0,
                    bank: 0,
                    reserved: 0,
                    crank_phase_deg: 0.0,
                    pressure: 0.0,
                    pressure_derivative: 0.0,
                    energy: 0.0,
                    cycle_variation: 0.0,
                    event_pad: 0.0,
                }; MAX_EVENTS_PER_BLOCK],
            });

            Ok(Self {
                library: lib,
                handle,
                fns,
                c_block,
                c_controls: CControls::default(),
                l_buf: vec![0.0f32; MAX_BLOCK_SAMPLES],
                r_buf: vec![0.0f32; MAX_BLOCK_SAMPLES],
                last_frames: 0,
            })
        }
    }

    /// Reported ABI version (for diagnostics/tests).
    pub fn abi_version(&self) -> u32 {
        // SAFETY: `self.fns.abi_version` was resolved from the loaded DSP module and the
        // module is kept alive by `self.library` for the lifetime of `self`.
        unsafe { (self.fns.abi_version)() }
    }

    /// The 40-byte build source string the DLL was compiled from.
    pub fn build_source(&self) -> &str {
        // SAFETY: `build_source` returns a pointer into the DSP module's static build string
        // (NUL-terminated); the module outlives `self`. The null case is handled.
        unsafe {
            let p = (self.fns.build_source)();
            if p.is_null() {
                return "";
            }
            CStr::from_ptr(p).to_str().unwrap_or("")
        }
    }

    /// Validate an arbitrary expected build source against the loaded DLL.
    pub fn check_build_source(&self, expected: &str) -> i32 {
        if let Ok(c) = CString::new(expected) {
            // SAFETY: `c` is a valid NUL-terminated C string; the fn pointer was resolved from
            // the loaded DSP module kept alive by `self.library`.
            unsafe { (self.fns.check_build)(c.as_ptr()) }
        } else {
            -1
        }
    }

    pub fn reset(&mut self) {
        // SAFETY: `self.handle` is a live instance created by the DSP module (or null, which
        // the module tolerates) and the module is kept alive by `self.library`.
        unsafe {
            (self.fns.reset)(self.handle);
        }
    }

    pub(crate) fn diagnostics(&self) -> CDiagnostics {
        let mut d = CDiagnostics {
            struct_size: 40,
            ..CDiagnostics::default()
        };
        // SAFETY: `self.handle` is a live instance; `d` is a stack `CDiagnostics` matching the
        // C layout, written by the DSP module whose code is kept alive by `self.library`.
        unsafe {
            (self.fns.get_diag)(self.handle, &mut d);
        }
        d
    }

    pub fn events_received(&self) -> u32 {
        self.diagnostics().events_received
    }

    /// Per-block entrypoint. Converts the Rust `F90DspEventBlock` into the exact
    /// C layout (allocation-free), calls `f90_dsp_process`, and writes the L/R
    /// output into the preallocated buffers. `frames` is taken from
    /// `src.block_samples` and must be <= 4096 (the C++ hard cap). Returns the
    /// number of frames produced.
    ///
    /// No DLL loads, no logs, no allocations occur on this path.
    pub fn process_block(
        &mut self,
        src: &F90DspEventBlock,
        ctl: &DspControls,
    ) -> Result<u32, DspRuntimeError> {
        let frames = src.block_samples.min(MAX_BLOCK_SAMPLES as u32);
        if src.block_samples > MAX_BLOCK_SAMPLES as u32 {
            return Err(DspRuntimeError::BlockTooLarge(src.block_samples));
        }

        // --- convert Rust event block -> C layout (field copy, no alloc) ---
        let count = src.event_count.min(MAX_EVENTS_PER_BLOCK as u32) as usize;
        self.c_block.stream_block_id = src.stream_block_id;
        self.c_block.event_count = count as u32;
        self.c_block.block_samples = frames;
        for i in 0..count {
            let s: &F90DspEvent = &src.events[i];
            self.c_block.events[i] = CEvent {
                sample_offset: s.sample_offset,
                cylinder: s.cylinder,
                bank: s.bank,
                reserved: 0,
                crank_phase_deg: s.crank_phase_deg,
                pressure: s.pressure,
                pressure_derivative: s.pressure_derivative,
                energy: s.energy,
                cycle_variation: s.cycle_variation,
                event_pad: 0.0,
            };
        }

        self.c_controls = CControls {
            rpm: ctl.rpm,
            throttle: ctl.throttle,
            load: ctl.load,
            tc_cut: ctl.tc_cut,
            master_gain: ctl.master_gain,
            lod: ctl.lod,
            bypass: ctl.bypass as u8,
            reserved0: 0,
            reserved1: 0,
        };

        // SAFETY: `self.handle` is a live instance; `self.c_block`/`self.c_controls` are
        // `repr(C)` structs matching the DSP ABI, and `l_buf`/`r_buf` hold at least `frames`
        // writable samples. The module code is kept alive by `self.library`.
        let rc = unsafe {
            (self.fns.process)(
                self.handle,
                &*self.c_block,
                &self.c_controls,
                self.l_buf.as_mut_ptr(),
                self.r_buf.as_mut_ptr(),
                frames,
            )
        };
        self.last_frames = frames as usize;
        if rc == 0 {
            Ok(frames)
        } else {
            Err(DspRuntimeError::ProcessFailed(rc))
        }
    }

    /// Left-channel output of the most recent `process_block` (length =
    /// `last_frames`).
    pub fn left(&self) -> &[f32] {
        &self.l_buf[..self.last_frames]
    }

    /// Right-channel output of the most recent `process_block`.
    pub fn right(&self) -> &[f32] {
        &self.r_buf[..self.last_frames]
    }

    /// Create the fixed-capacity event sink used by `CylState`. It contains no
    /// clock or firing schedule; the sample rate/seed arguments remain only for
    /// source compatibility with the pre-refactor constructor.
    pub fn make_event_builder(sample_rate: f64, seed: u64) -> EventBuilder {
        EventBuilder::new(sample_rate, seed)
    }
}

impl Drop for DspRuntime {
    fn drop(&mut self) {
        // Destroy the C++ instance exactly once. Guard against a null handle
        // (partial init) and clear both pointers so a double-drop on unwind is
        // impossible.
        if !self.handle.is_null() {
            // SAFETY: `self.handle` is a live instance created by the DSP module; it is
            // destroyed exactly once and cleared immediately after.
            unsafe {
                (self.fns.destroy)(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
        if !self.library.is_null() {
            // SAFETY: `self.library` is a module handle returned by `LoadLibraryW`, freed
            // exactly once and cleared immediately after.
            unsafe {
                FreeLibrary(self.library);
            }
            self.library = std::ptr::null_mut();
        }
    }
}

#[cfg(windows)]
extern "system" {
    fn LoadLibraryW(lpFileName: *const u16) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
    fn FreeLibrary(hModule: *mut c_void) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn loaded() -> DspRuntime {
        DspRuntime::load_default().expect("DLL must be present and gate-clean in this build")
    }

    #[test]
    #[cfg(windows)]
    fn abi_and_build_source_gate_satisfied() {
        let dsp = loaded();
        assert_eq!(dsp.abi_version(), 2);
        assert_eq!(dsp.build_source(), expected_build_source());
        assert_eq!(dsp.check_build_source(expected_build_source()), 0);
        // A wrong build source must be rejected by the DLL.
        assert_ne!(
            dsp.check_build_source("0000000000000000000000000000000000000000"),
            0
        );
    }

    #[test]
    #[cfg(windows)]
    fn impulse_timing_matches_event_offsets_both_banks_boundary() {
        let mut dsp = loaded();
        // Manually craft a block: bank-0 @50, bank-1 @120, and a near-boundary
        // event @511 (last valid sample of a 512-frame block). The C++ must fire
        // an impulse *exactly* at each sample_offset and stay silent before the
        // first one.
        let mut blk = F90DspEventBlock::default();
        blk.stream_block_id = 0;
        blk.block_samples = 512;
        blk.event_count = 3;
        blk.events[0] = F90DspEvent {
            sample_offset: 50,
            cylinder: 0,
            bank: 0,
            pressure: 0.8,
            pressure_derivative: 0.8,
            ..F90DspEvent::default()
        };
        blk.events[1] = F90DspEvent {
            sample_offset: 120,
            cylinder: 2,
            bank: 1,
            pressure: 0.6,
            pressure_derivative: 0.6,
            ..F90DspEvent::default()
        };
        blk.events[2] = F90DspEvent {
            sample_offset: 511,
            cylinder: 4,
            bank: 0,
            pressure: 0.7,
            pressure_derivative: 0.7,
            ..F90DspEvent::default()
        };

        let ctl = DspControls {
            rpm: 9000.0,
            throttle: 1.0,
            load: 1.0,
            master_gain: 1.0,
            ..DspControls::default()
        };
        dsp.process_block(&blk, &ctl).expect("process ok");

        let l = dsp.left();
        assert_eq!(l.len(), 512);
        // Silent before the first event.
        for &v in &l[0..50] {
            assert_eq!(v, 0.0, "output must be silent before first event offset");
        }
        // Non-zero exactly at the event offsets (mirrors rust_event_impulse timing).
        assert_ne!(l[50], 0.0, "bank-0 event @50 must produce an impulse");
        assert_ne!(l[120], 0.0, "bank-1 event @120 must produce an impulse");
        assert_ne!(l[511], 0.0, "boundary event @511 must not be truncated");
        // Both banks arrived (the DLL counts each valid event).
        assert_eq!(dsp.events_received(), 3);
    }

    #[test]
    #[cfg(windows)]
    fn disabled_when_master_gain_zero() {
        let mut dsp = loaded();
        let mut blk = F90DspEventBlock::default();
        blk.block_samples = 512;
        blk.event_count = 1;
        blk.events[0] = F90DspEvent {
            sample_offset: 80,
            cylinder: 1,
            bank: 1,
            pressure: 0.9,
            ..F90DspEvent::default()
        };
        // Conservative default: gain 0 => the C++ layer contributes nothing.
        let ctl = DspControls {
            rpm: 9000.0,
            master_gain: 0.0,
            ..DspControls::default()
        };
        dsp.process_block(&blk, &ctl).expect("process ok");
        assert!(dsp.left().iter().all(|&v| v == 0.0));
        assert!(dsp.right().iter().all(|&v| v == 0.0));
        // But the event was still received (control plane alive).
        assert_eq!(dsp.events_received(), 1);
    }

    #[test]
    #[cfg(windows)]
    fn no_cross_block_loss_across_continuous_blocks() {
        let mut dsp = loaded();
        let mut builder = DspRuntime::make_event_builder(44100.0, 0xCAFE);
        let mut total_hand_members = 0u32;
        let mut first_block_received = 0u32;
        let mut second_block_received = 0u32;
        for b in 0..8u64 {
            builder.begin_block(512);
            let offset = if b == 0 { 511 } else { 10 };
            builder.record_physical(offset, (b % 5) as u8, 0.0, 0.8, 100.0, 0.7, 0.0, 2.0);
            let blk = builder.finish_block();
            total_hand_members += blk.event_count;
            let ctl = DspControls {
                rpm: 15000.0,
                master_gain: 0.5,
                ..DspControls::default()
            };
            dsp.process_block(blk, &ctl).expect("process ok");
            let rcv = dsp.events_received();
            if b == 0 {
                first_block_received = rcv;
            }
            if b == 1 {
                second_block_received = rcv - first_block_received;
            }
        }
        let diag = dsp.diagnostics();
        // Every event Rust handed to C++ was received; nothing dropped at the
        // block boundary (the EventBuilder defers past-boundary firings into the
        // next block, and C++ counts them there).
        assert_eq!(diag.events_received, total_hand_members);
        assert_eq!(diag.events_dropped, 0);
        assert!(first_block_received > 0, "block 0 must receive events");
        assert!(
            second_block_received > 0,
            "block 1 must also receive events (boundary firings deferred, not lost)"
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn noop_on_non_windows() {
        // The Fase-4 DLL path is Windows-only; on other platforms we report a
        // clean, controlled error rather than panicking.
        assert!(matches!(
            DspRuntime::load_default(),
            Err(DspRuntimeError::UnsupportedPlatform) | Err(DspRuntimeError::DllNotFound)
        ));
    }
}
