#!/usr/bin/env python3
"""
analyze_suspension.py — F1 2030 Physics Diagnostics
=====================================================
Analyzes suspension balance, natural frequencies, damping ratios,
bump-stop risk, and rollover susceptibility for a given vehicle config.

Uses real parameters from f1_2026_car.tscn by default.

Usage:
    python analyze_suspension.py
    python analyze_suspension.py --mass 708 --rear-spring 0.20 --rear-damping 0.78

Dependencies: numpy, scipy, matplotlib
"""

import argparse
import sys
import numpy as np

if sys.stdout.encoding.lower() != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')
    
# Import parser
import os
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
from tscn_parser import update_config_from_tscn

import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec
from scipy.signal import lti, step

# ─────────────────────────────────────────────────────────────────────────────
# DEFAULT CONFIG — mirrored from f1_2026_car.tscn
# ─────────────────────────────────────────────────────────────────────────────
DEFAULT = dict(
    vehicle_mass               = 708.0,    # kg  (includes driver)
    front_weight_distribution  = 0.46,     # 0..1 (fraction on front axle)
    center_of_gravity_height   = 0.28,     # m   (wheel center + offset: 0.38 → ~0.28 effective ride height CG)
    wheelbase                  = 3.24,     # m   (dist between axles: |z_front - z_rear| = 1.62*2)
    front_track_width          = 1.573,    # m   (wheel X offset * 2)
    rear_track_width           = 1.473,    # m

    # Front suspension
    front_spring_length        = 0.15,     # m   (max travel)
    front_resting_ratio        = 0.50,     # compression at rest (0..1)
    front_damping_ratio        = 0.75,     # critical damping ratio (GEVP param)
    front_bump_damp_mult       = 1.3,
    front_rebound_damp_mult    = 1.1,
    front_bump_stop_mult       = 3.0,

    # Rear suspension
    rear_spring_length         = 0.20,     # m
    rear_resting_ratio         = 0.25,
    rear_damping_ratio         = 0.78,
    rear_bump_damp_mult        = 2.0,
    rear_rebound_damp_mult     = 1.8,
    rear_bump_stop_mult        = 1.0,      # ← LOW. Likely rollover source.

    # Tire stiffness (N/m equivalent)
    tire_stiffness_road        = 22000.0,  # N/m
    tire_stiffness_curb        = 9000.0,   # N/m

    front_tire_radius          = 0.370,    # m
    rear_tire_radius           = 0.370,    # m
    front_wheel_mass           = 6.0,      # kg
    rear_wheel_mass            = 7.5,      # kg
)

G = 9.81  # m/s²

# ─────────────────────────────────────────────────────────────────────────────
# CALCULATIONS
# ─────────────────────────────────────────────────────────────────────────────

def analyze(cfg: dict) -> dict:
    m      = cfg["vehicle_mass"]
    fd     = cfg["front_weight_distribution"]
    h_cg   = cfg["center_of_gravity_height"]
    wb     = cfg["wheelbase"]
    tw_f   = cfg["front_track_width"]
    tw_r   = cfg["rear_track_width"]

    # Weight distribution per axle and per corner
    W_front_axle  = m * G * fd             # N
    W_rear_axle   = m * G * (1.0 - fd)    # N
    W_front_corner = W_front_axle / 2.0
    W_rear_corner  = W_rear_axle  / 2.0

    # ── Spring rates from GEVP resting_ratio
    # k = W_corner / (spring_length * resting_ratio)
    # resting_ratio = compression fraction at static equilibrium
    k_front = W_front_corner / (cfg["front_spring_length"] * cfg["front_resting_ratio"])
    k_rear  = W_rear_corner  / (cfg["rear_spring_length"]  * cfg["rear_resting_ratio"])

    # ── Sprung mass per corner (exclude unsprung wheel mass)
    m_front_corner = (m / 2.0 * fd)   - cfg["front_wheel_mass"]
    m_rear_corner  = (m / 2.0 * (1.0 - fd)) - cfg["rear_wheel_mass"]

    # ── Natural frequencies (Hz)
    omega_front = np.sqrt(k_front / m_front_corner)
    omega_rear  = np.sqrt(k_rear  / m_rear_corner)
    f_front = omega_front / (2 * np.pi)
    f_rear  = omega_rear  / (2 * np.pi)

    # ── Critical damping coefficients
    c_crit_front = 2.0 * np.sqrt(k_front * m_front_corner)
    c_crit_rear  = 2.0 * np.sqrt(k_rear  * m_rear_corner)
    c_actual_front = cfg["front_damping_ratio"] * c_crit_front
    c_actual_rear  = cfg["rear_damping_ratio"]  * c_crit_rear

    # ── Static compression at rest (how much travel is used)
    static_comp_front = cfg["front_spring_length"] * cfg["front_resting_ratio"]
    static_comp_rear  = cfg["rear_spring_length"]  * cfg["rear_resting_ratio"]
    travel_left_front = cfg["front_spring_length"] - static_comp_front
    travel_left_rear  = cfg["rear_spring_length"]  - static_comp_rear

    # ── Bump-stop clearance analysis
    # Extra bump force at bump-stop = spring_force * bump_stop_multiplier
    bump_stop_force_front = k_front * cfg["front_spring_length"] * cfg["front_bump_stop_mult"]
    bump_stop_force_rear  = k_rear  * cfg["rear_spring_length"]  * cfg["rear_bump_stop_mult"]

    # ── Rollover threshold (lateral G before two-wheel lift)
    # Tipping point: G_lat = (track_width / 2) / h_cg
    rollover_g_front = (tw_f / 2.0) / h_cg
    rollover_g_rear  = (tw_r / 2.0) / h_cg

    # ── Load transfer under braking (1G deceleration)
    delta_load_brake_1g = (m * G * 1.0 * h_cg) / wb   # N transferred front
    extra_load_rear_pct = (delta_load_brake_1g / W_rear_axle) * 100

    # ── Frequency balance assessment
    freq_ratio = f_rear / f_front
    # Ideal: rear slightly lower than front (0.85–1.05) to avoid pitch resonance
    freq_warning = ""
    if freq_ratio < 0.85:
        freq_warning = "⚠️  REAR TOO SOFT vs FRONT — bounce/pitch resonance risk"
    elif freq_ratio > 1.10:
        freq_warning = "⚠️  REAR TOO STIFF vs FRONT — sharp rear response, rollover risk on curbs"
    else:
        freq_warning = "✅  Frequency balance acceptable"

    return dict(
        W_front_axle=W_front_axle, W_rear_axle=W_rear_axle,
        W_front_corner=W_front_corner, W_rear_corner=W_rear_corner,
        k_front=k_front, k_rear=k_rear,
        m_front_corner=m_front_corner, m_rear_corner=m_rear_corner,
        omega_front=omega_front, omega_rear=omega_rear,
        f_front=f_front, f_rear=f_rear,
        c_crit_front=c_crit_front, c_crit_rear=c_crit_rear,
        c_actual_front=c_actual_front, c_actual_rear=c_actual_rear,
        static_comp_front=static_comp_front, static_comp_rear=static_comp_rear,
        travel_left_front=travel_left_front, travel_left_rear=travel_left_rear,
        bump_stop_force_front=bump_stop_force_front,
        bump_stop_force_rear=bump_stop_force_rear,
        rollover_g_front=rollover_g_front, rollover_g_rear=rollover_g_rear,
        delta_load_brake_1g=delta_load_brake_1g,
        extra_load_rear_pct=extra_load_rear_pct,
        freq_ratio=freq_ratio, freq_warning=freq_warning,
    )


def simulate_step_response(cfg: dict, r: dict):
    """Simulates suspension step response (e.g., hitting a curb) via LTI system."""
    t = np.linspace(0, 2.0, 1000)

    results = {}
    for axle, (k, c, m_c, label) in enumerate([
        (r["k_front"], r["c_actual_front"], r["m_front_corner"], "Front"),
        (r["k_rear"],  r["c_actual_rear"],  r["m_rear_corner"],  "Rear"),
    ]):
        # 2nd order LTI: m*x'' + c*x' + k*x = F
        sys = lti([1.0 / m_c], [1.0, c / m_c, k / m_c])
        _, y = step(sys, T=t)
        results[label] = (t, y)
    return results


def print_report(cfg: dict, r: dict):
    print("\n" + "="*60)
    print("  F1 2030 — SUSPENSION DIAGNOSTIC REPORT")
    print("="*60)

    print(f"\n📐 WEIGHT DISTRIBUTION")
    print(f"   Mass           : {cfg['vehicle_mass']:.0f} kg")
    print(f"   Front axle     : {r['W_front_axle']:.1f} N  ({cfg['front_weight_distribution']*100:.0f}%)")
    print(f"   Rear axle      : {r['W_rear_axle']:.1f} N  ({(1-cfg['front_weight_distribution'])*100:.0f}%)")
    print(f"   Per corner F   : {r['W_front_corner']:.1f} N")
    print(f"   Per corner R   : {r['W_rear_corner']:.1f} N")

    print(f"\n🔩 SPRING RATES (derived from resting_ratio)")
    print(f"   Front k        : {r['k_front']:.1f} N/m")
    print(f"   Rear  k        : {r['k_rear']:.1f} N/m")

    print(f"\n🎵 NATURAL FREQUENCIES")
    print(f"   Front          : {r['f_front']:.2f} Hz  (ω = {r['omega_front']:.2f} rad/s)")
    print(f"   Rear           : {r['f_rear']:.2f} Hz  (ω = {r['omega_rear']:.2f} rad/s)")
    print(f"   Ratio R/F      : {r['freq_ratio']:.3f}")
    print(f"   Assessment     : {r['freq_warning']}")

    print(f"\n💧 DAMPING")
    print(f"   Front ζ        : {cfg['front_damping_ratio']:.2f}  (c_crit={r['c_crit_front']:.1f}, c_actual={r['c_actual_front']:.1f})")
    print(f"   Rear  ζ        : {cfg['rear_damping_ratio']:.2f}  (c_crit={r['c_crit_rear']:.1f}, c_actual={r['c_actual_rear']:.1f})")
    print(f"   Front bump×    : {cfg['front_bump_damp_mult']:.1f}x  | rebound× {cfg['front_rebound_damp_mult']:.1f}x")
    print(f"   Rear  bump×    : {cfg['rear_bump_damp_mult']:.1f}x  | rebound× {cfg['rear_rebound_damp_mult']:.1f}x")

    print(f"\n📏 TRAVEL & BUMP STOP")
    print(f"   Front: static comp {r['static_comp_front']*1000:.1f}mm | travel left {r['travel_left_front']*1000:.1f}mm | bump force {r['bump_stop_force_front']:.0f}N")
    print(f"   Rear : static comp {r['static_comp_rear']*1000:.1f}mm | travel left {r['travel_left_rear']*1000:.1f}mm | bump force {r['bump_stop_force_rear']:.0f}N")
    if cfg["rear_bump_stop_mult"] < 1.5:
        print(f"   ⚠️  rear_bump_stop_mult = {cfg['rear_bump_stop_mult']:.1f}  (LOW — chassis may bottom-out on curbs → rollover risk)")

    print(f"\n🔄 ROLLOVER THRESHOLD")
    print(f"   Front : {r['rollover_g_front']:.2f} G lateral before two-wheel lift")
    print(f"   Rear  : {r['rollover_g_rear']:.2f} G lateral before two-wheel lift")

    print(f"\n🛑 LOAD TRANSFER (1G braking)")
    print(f"   Load shift     : {r['delta_load_brake_1g']:.1f} N to front")
    print(f"   Rear unloading : {r['extra_load_rear_pct']:.1f}% of rear axle weight")

    print("\n" + "="*60 + "\n")


def plot_results(cfg: dict, r: dict, step_resp: dict):
    fig = plt.figure(figsize=(14, 8), facecolor="#0f0f0f")
    fig.suptitle("F1 2030 — Suspension Diagnostic", color="white", fontsize=14, fontweight="bold")
    gs = gridspec.GridSpec(2, 3, figure=fig, hspace=0.45, wspace=0.35)

    text_kw = dict(color="white")
    bar_colors = ["#00d4ff", "#ff4444"]

    # 1) Natural frequencies bar chart
    ax1 = fig.add_subplot(gs[0, 0])
    ax1.set_facecolor("#1a1a1a")
    bars = ax1.bar(["Front", "Rear"], [r["f_front"], r["f_rear"]], color=bar_colors)
    ax1.set_title("Natural Frequency (Hz)", **text_kw)
    ax1.set_ylabel("Hz", **text_kw)
    ax1.tick_params(colors="white"); ax1.spines[:].set_color("#444")
    for bar, v in zip(bars, [r["f_front"], r["f_rear"]]):
        ax1.text(bar.get_x() + bar.get_width()/2, v + 0.02, f"{v:.2f}", ha="center", color="white", fontsize=10)
    ideal = plt.axhline(y=2.0, color="#ffaa00", linestyle="--", linewidth=1, label="Ideal ~2Hz")
    ax1.legend(facecolor="#222", labelcolor="white", fontsize=8)

    # 2) Spring rates
    ax2 = fig.add_subplot(gs[0, 1])
    ax2.set_facecolor("#1a1a1a")
    bars2 = ax2.bar(["Front k", "Rear k"], [r["k_front"], r["k_rear"]], color=bar_colors)
    ax2.set_title("Spring Rates (N/m)", **text_kw)
    ax2.set_ylabel("N/m", **text_kw)
    ax2.tick_params(colors="white"); ax2.spines[:].set_color("#444")
    for bar, v in zip(bars2, [r["k_front"], r["k_rear"]]):
        ax2.text(bar.get_x() + bar.get_width()/2, v + 50, f"{v:.0f}", ha="center", color="white", fontsize=9)

    # 3) Damping ratios
    ax3 = fig.add_subplot(gs[0, 2])
    ax3.set_facecolor("#1a1a1a")
    damping_vals = [cfg["front_damping_ratio"], cfg["rear_damping_ratio"]]
    bars3 = ax3.bar(["Front ζ", "Rear ζ"], damping_vals, color=bar_colors)
    ax3.axhline(y=1.0, color="red", linestyle="--", linewidth=1, label="Critical damping")
    ax3.axhline(y=0.7, color="#ffaa00", linestyle="--", linewidth=1, label="Ideal (0.7)")
    ax3.set_title("Damping Ratio ζ", **text_kw)
    ax3.set_ylim(0, 1.4)
    ax3.tick_params(colors="white"); ax3.spines[:].set_color("#444")
    ax3.legend(facecolor="#222", labelcolor="white", fontsize=8)

    # 4) Step response — Front
    ax4 = fig.add_subplot(gs[1, 0])
    ax4.set_facecolor("#1a1a1a")
    t, y = step_resp["Front"]
    ax4.plot(t, y, color="#00d4ff", linewidth=1.5)
    ax4.axhline(y=1.0, color="#444", linestyle="--", linewidth=0.8)
    ax4.set_title("Step Response — Front", **text_kw)
    ax4.set_xlabel("Time (s)", **text_kw); ax4.set_ylabel("Normalized disp.", **text_kw)
    ax4.tick_params(colors="white"); ax4.spines[:].set_color("#444")

    # 5) Step response — Rear
    ax5 = fig.add_subplot(gs[1, 1])
    ax5.set_facecolor("#1a1a1a")
    t, y = step_resp["Rear"]
    ax5.plot(t, y, color="#ff4444", linewidth=1.5)
    ax5.axhline(y=1.0, color="#444", linestyle="--", linewidth=0.8)
    ax5.set_title("Step Response — Rear (curb hit sim)", **text_kw)
    ax5.set_xlabel("Time (s)", **text_kw)
    ax5.tick_params(colors="white"); ax5.spines[:].set_color("#444")

    # 6) Rollover threshold
    ax6 = fig.add_subplot(gs[1, 2])
    ax6.set_facecolor("#1a1a1a")
    bars6 = ax6.bar(["Front\nRollover G", "Rear\nRollover G"],
                    [r["rollover_g_front"], r["rollover_g_rear"]], color=bar_colors)
    ax6.axhline(y=3.0, color="#00ff88", linestyle="--", linewidth=1, label="F1 corner G ~3G")
    ax6.set_title("Rollover Threshold (G)", **text_kw)
    ax6.set_ylabel("Lateral G", **text_kw)
    ax6.legend(facecolor="#222", labelcolor="white", fontsize=8)
    ax6.tick_params(colors="white"); ax6.spines[:].set_color("#444")
    for bar, v in zip(bars6, [r["rollover_g_front"], r["rollover_g_rear"]]):
        ax6.text(bar.get_x() + bar.get_width()/2, v + 0.03, f"{v:.2f}G", ha="center", color="white", fontsize=10)

    out = "suspension_report.png"
    plt.savefig(out, dpi=120, bbox_inches="tight", facecolor=fig.get_facecolor())
    print(f"📊 Chart saved → {out}")
    plt.show()


def main():
    parser = argparse.ArgumentParser(description="Analyze F1 2030 Suspension (with dynamic .tscn loading)")
    parser.add_argument("--tscn", type=str, default="game/scenes/vehicles/f1_2026_car.tscn", help="Path to vehicle scene")
    parser.add_argument("--mass",          type=float, default=DEFAULT["vehicle_mass"])
    parser.add_argument("--rear-spring",   type=float, default=DEFAULT["rear_spring_length"],   dest="rear_spring_length")
    parser.add_argument("--rear-damping",  type=float, default=DEFAULT["rear_damping_ratio"],   dest="rear_damping_ratio")
    parser.add_argument("--rear-resting",  type=float, default=DEFAULT["rear_resting_ratio"],   dest="rear_resting_ratio")
    parser.add_argument("--rear-bump-mult",type=float, default=DEFAULT["rear_bump_stop_mult"],  dest="rear_bump_stop_mult")
    parser.add_argument("--no-plot",       action="store_true")
    args = parser.parse_args()

    cfg = DEFAULT.copy()
    
    # DYNAMIC TSCN OVERRIDE!
    # This prevents the AI from relying on stale defaults.
    tscn_path = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), args.tscn)
    cfg = update_config_from_tscn(cfg, tscn_path)

    # CLI overrides
    if args.mass: cfg['vehicle_mass'] = args.mass
    cfg["rear_spring_length"]  = args.rear_spring_length
    cfg["rear_damping_ratio"]  = args.rear_damping_ratio
    cfg["rear_resting_ratio"]  = args.rear_resting_ratio
    cfg["rear_bump_stop_mult"] = args.rear_bump_stop_mult

    r = analyze(cfg)
    print_report(cfg, r)

    if not args.no_plot:
        step_resp = simulate_step_response(cfg, r)
        plot_results(cfg, r, step_resp)


if __name__ == "__main__":
    main()
