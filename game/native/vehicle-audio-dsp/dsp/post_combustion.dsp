import("stdfaust.lib");

// ===========================================================================
// post_combustion — Fase 5 Faust source–filter timbre for the F90 combustion DSP.
//
// Audio inputs (set per sample from the C++ wrapper):
//   in0  excitation  : normalized pressure-derivative impulse train. The C++
//                      boundary converts ABI physical pressure-units/s to this
//                      bounded acoustic domain; zero input => exact silence.
//   in1  level       : secondary acoustic load control (0.55..1.0). Event
//                      amplitude already lives in excitation, so this must not
//                      re-apply raw throttle as a full amplitude multiplier.
//   in2  intake      : airbox enable gate (0 = off, 1 = on). Kept off by default
//                      per spec 5.2 (airbox only when intake is reactivated).
//
// Output: single bipolar post-combustion signal.
// NO free-running saw/square/oscillator or continuous noise: every resonant
// mode is driven by the impulse, and the HF residual is gated by a per-event
// decaying envelope (deterministic colored noise * event envelope).
// ===========================================================================

process(excitation, level, intake) = sig
with {
    // Remove the DC of the unipolar transient so the source is not sustained.
    excd = excitation : fi.dcblocker;

    // --- Modal bank: separable families (block/head, headers, collector, airbox) ---
    // Each is a small parallel of bandpass resonators driven by the impulse.
    // Frequency / Q / gain are fixed; they can be smoothed later if needed.
    // Bandwidths are deliberately wide so every mode fully rings out well within
    // one ~1 kHz block: a bypassed event therefore leaves no stale tail (Fase 3.3).
    blockMode = excd <: fi.resonbp(180.0, 120.0, 1.0), fi.resonbp(320.0, 140.0, 0.8) :> _;
    headerMode = excd <: fi.resonbp(650.0, 180.0, 0.7), fi.resonbp(1100.0, 220.0, 0.5) :> _;
    collectorMode = excd <: fi.resonbp(2200.0, 400.0, 0.5), fi.resonbp(3500.0, 600.0, 0.32) :> _;
    airboxMode = excd : fi.resonbp(5200.0, 1000.0, 0.4) * intake;

    // Overall modal level (the bank is resonant, so it is scaled to keep the
    // percussive combustion transient controlled).
    modal = (blockMode + headerMode + collectorMode + airboxMode) * 0.10;

    // --- HF residual (5.3): deterministic colored noise, event-triggered only ---
    // Envelope rises on |excitation| and decays; with no events it stays exactly 0.
    hfEnv = abs(excitation) : *(0.45) : + ~ (*(0.80));
    hfColor = no.noise : fi.highpass(1, 2500.0) : fi.lowpass(1, 7000.0);
    hf = hfColor * hfEnv * 0.12;

    // The wrapper already bounds excitation. This clip is only a last-resort
    // safety curve, not the primary level control; at nominal amplitudes it is
    // intentionally close to linear.
    softclip(x) = x / (1.0 + 0.25 * abs(x));
    sig = (modal + hf) * level : softclip;
};
