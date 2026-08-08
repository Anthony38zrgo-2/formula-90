#!/usr/bin/env python3
"""
analyze_powertrain.py — F1 2030 Physics Diagnostics
=====================================================
Analyzes V10 engine torque curve, gear ratios, top speeds per gear,
wheel force across the RPM range, and ideal shift points.

Uses real parameters from f1_2026_car.tscn by default.

Usage:
    python analyze_powertrain.py
    python analyze_powertrain.py --max-torque 380 --max-rpm 19000

Dependencies: numpy, matplotlib
"""

import argparse
import sys
import numpy as np

if sys.stdout.encoding.lower() != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')
import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec

# ─────────────────────────────────────────────────────────────────────────────
# DEFAULT CONFIG — mirrored from f1_2026_car.tscn + vehicle.gd defaults
# ─────────────────────────────────────────────────────────────────────────────
DEFAULT = dict(
    max_torque     = 360.0,          # Nm
    max_rpm        = 18500.0,        # RPM
    idle_rpm       = 4500.0,         # RPM
    motor_brake    = 18.0,           # engine braking constant
    motor_moment   = 0.10,           # kg·m² flywheel inertia

    gear_ratios    = [2.85, 2.29, 1.89, 1.60, 1.38, 1.20],
    final_drive    = 3.60,
    reverse_ratio  = 3.00,
    shift_time     = 0.04,           # s

    rear_tire_radius = 0.370,        # m

    vehicle_mass   = 708.0,          # kg
    coefficient_of_drag = 0.95,
    frontal_area   = 1.50,           # m²
    air_density    = 1.225,          # kg/m³
)

# ─────────────────────────────────────────────────────────────────────────────
# V10 TORQUE CURVE — approximated from real atmospheric V10 (Cosworth/BMW S70)
# Defined as normalized (0..1) at key RPM points.
# ─────────────────────────────────────────────────────────────────────────────
TORQUE_CURVE_POINTS = np.array([
    [0.00,  0.20],   # idle
    [0.15,  0.50],   # low RPM plateau
    [0.30,  0.75],
    [0.45,  0.88],
    [0.55,  0.95],
    [0.65,  1.00],   # peak torque ~65% of max RPM (~12,000 RPM)
    [0.75,  0.98],
    [0.85,  0.92],
    [0.92,  0.82],
    [1.00,  0.65],   # at rev limiter
])


def torque_at_rpm(rpm: np.ndarray, cfg: dict) -> np.ndarray:
    """Returns actual torque (Nm) at given RPM array."""
    normalized_rpm = rpm / cfg["max_rpm"]
    normalized_torque = np.interp(
        normalized_rpm,
        TORQUE_CURVE_POINTS[:, 0],
        TORQUE_CURVE_POINTS[:, 1],
    )
    return normalized_torque * cfg["max_torque"]


def analyze(cfg: dict) -> dict:
    rpm_range = np.linspace(cfg["idle_rpm"], cfg["max_rpm"], 1000)
    torque    = torque_at_rpm(rpm_range, cfg)
    power_kw  = torque * (rpm_range * 2 * np.pi / 60) / 1000.0  # kW
    power_hp  = power_kw * 1.341                                  # HP

    # ── Per-gear analysis ──────────────────────────────────────────────────
    gear_data = []
    for i, gr in enumerate(cfg["gear_ratios"]):
        total_ratio = gr * cfg["final_drive"]

        # Wheel torque (Nm) across RPM
        wheel_torque = torque * total_ratio  # simplified (no transmission losses)

        # Vehicle speed (km/h) for each RPM in this gear
        wheel_omega  = rpm_range * 2 * np.pi / 60 / total_ratio   # rad/s
        speed_ms     = wheel_omega * cfg["rear_tire_radius"]       # m/s
        speed_kmh    = speed_ms * 3.6

        # Tractive force (N)
        tractive_force = wheel_torque / cfg["rear_tire_radius"]

        # Aero drag (N) at each speed
        drag = 0.5 * cfg["air_density"] * cfg["coefficient_of_drag"] * cfg["frontal_area"] * speed_ms**2

        # Net force
        net_force = tractive_force - drag

        # Top speed in this gear (where tractive force = drag → net=0)
        # Find first crossing from positive to negative
        crossings = np.where(np.diff(np.sign(net_force)))[0]
        if len(crossings):
            top_speed = speed_kmh[crossings[0]]
        else:
            top_speed = speed_kmh[-1]

        # Peak power point RPM
        peak_power_rpm  = rpm_range[np.argmax(power_kw)]
        # Shift RPM = power peak of current gear = power peak of next gear in current gear
        # Simple rule: shift at 95% of max RPM for F1
        shift_rpm = cfg["max_rpm"] * 0.97

        gear_data.append(dict(
            gear=i+1, ratio=gr, total_ratio=total_ratio,
            speed_at_redline=speed_kmh[-1],
            top_speed=top_speed,
            peak_wheel_torque=float(np.max(wheel_torque)),
            peak_tractive_force=float(np.max(tractive_force)),
        ))

    # ── Engine braking at max RPM ──────────────────────────────────────────
    # GEVP: engine_brake_torque ≈ motor_brake * (rpm / max_rpm)
    engine_brake_torque = cfg["motor_brake"] * (cfg["max_rpm"] / cfg["max_rpm"])

    # ── Power peak ────────────────────────────────────────────────────────
    peak_power_idx = np.argmax(power_kw)
    peak_power_rpm = rpm_range[peak_power_idx]
    peak_power_kw  = power_kw[peak_power_idx]
    peak_power_hp  = power_hp[peak_power_idx]
    peak_torque_idx = np.argmax(torque)
    peak_torque_rpm = rpm_range[peak_torque_idx]

    return dict(
        rpm_range=rpm_range, torque=torque, power_kw=power_kw, power_hp=power_hp,
        gear_data=gear_data,
        peak_power_rpm=peak_power_rpm, peak_power_kw=peak_power_kw, peak_power_hp=peak_power_hp,
        peak_torque_rpm=peak_torque_rpm, peak_torque=float(np.max(torque)),
        engine_brake_torque=engine_brake_torque,
    )


def print_report(cfg: dict, r: dict):
    print("\n" + "="*60)
    print("  F1 2030 — POWERTRAIN DIAGNOSTIC REPORT")
    print("="*60)

    print(f"\n🔥 ENGINE SPEC (V10 Atmospheric)")
    print(f"   Max torque     : {cfg['max_torque']:.0f} Nm")
    print(f"   Max RPM        : {cfg['max_rpm']:.0f}")
    print(f"   Idle RPM       : {cfg['idle_rpm']:.0f}")
    print(f"   Motor brake    : {cfg['motor_brake']:.1f}")
    print(f"   Flywheel I     : {cfg['motor_moment']:.2f} kg·m²")

    print(f"\n⚡ POWER PEAKS")
    print(f"   Peak Torque    : {r['peak_torque']:.1f} Nm @ {r['peak_torque_rpm']:.0f} RPM")
    print(f"   Peak Power     : {r['peak_power_kw']:.1f} kW  ({r['peak_power_hp']:.0f} HP) @ {r['peak_power_rpm']:.0f} RPM")

    print(f"\n🛑 ENGINE BRAKING @ max RPM")
    print(f"   Brake torque   : {r['engine_brake_torque']:.1f} Nm  (motor_brake constant)")

    print(f"\n⚙️  GEAR ANALYSIS")
    print(f"   {'Gear':<6} {'Ratio':<8} {'Total':<8} {'Top Speed':<14} {'Peak Wheel Torque'}")
    print(f"   {'-'*55}")
    for g in r["gear_data"]:
        print(f"   {g['gear']:<6} {g['ratio']:<8.2f} {g['total_ratio']:<8.2f} {g['top_speed']:<14.1f} {g['peak_wheel_torque']:.0f} Nm")
    print()


def plot_results(cfg: dict, r: dict):
    fig = plt.figure(figsize=(15, 8), facecolor="#0f0f0f")
    fig.suptitle("F1 2030 — Powertrain Diagnostic (V10)", color="white", fontsize=14, fontweight="bold")
    gs = gridspec.GridSpec(2, 3, figure=fig, hspace=0.4, wspace=0.35)

    text_kw = dict(color="white")

    # 1) Torque curve
    ax1 = fig.add_subplot(gs[0, :2])
    ax1.set_facecolor("#1a1a1a")
    ax1.plot(r["rpm_range"], r["torque"], color="#00d4ff", linewidth=2, label="Torque (Nm)")
    ax1.plot(r["rpm_range"], r["power_hp"], color="#ff8800", linewidth=2, linestyle="--", label="Power (HP)")
    ax1.axvline(r["peak_torque_rpm"], color="#00d4ff", linestyle=":", alpha=0.6)
    ax1.axvline(r["peak_power_rpm"],  color="#ff8800", linestyle=":", alpha=0.6)
    ax1.set_title("V10 Power & Torque Curve", **text_kw)
    ax1.set_xlabel("RPM", **text_kw); ax1.set_ylabel("Nm / HP", **text_kw)
    ax1.tick_params(colors="white"); ax1.spines[:].set_color("#444")
    ax1.legend(facecolor="#222", labelcolor="white")

    # 2) Top speed per gear
    ax2 = fig.add_subplot(gs[0, 2])
    ax2.set_facecolor("#1a1a1a")
    gears = [g["gear"] for g in r["gear_data"]]
    tops  = [g["top_speed"] for g in r["gear_data"]]
    bars  = ax2.bar([f"G{g}" for g in gears], tops, color="#ff4444")
    ax2.set_title("Max Speed per Gear (km/h)", **text_kw)
    ax2.set_ylabel("km/h", **text_kw)
    ax2.tick_params(colors="white"); ax2.spines[:].set_color("#444")
    for bar, v in zip(bars, tops):
        ax2.text(bar.get_x() + bar.get_width()/2, v + 1, f"{v:.0f}", ha="center", color="white", fontsize=9)

    # 3) Wheel torque per gear (peak)
    ax3 = fig.add_subplot(gs[1, :2])
    ax3.set_facecolor("#1a1a1a")
    peak_wt = [g["peak_wheel_torque"] for g in r["gear_data"]]
    ax3.bar([f"Gear {g}" for g in gears], peak_wt, color="#00ff88")
    ax3.set_title("Peak Wheel Torque per Gear (Nm)", **text_kw)
    ax3.set_ylabel("Nm", **text_kw)
    ax3.tick_params(colors="white"); ax3.spines[:].set_color("#444")

    # 4) Gear ratio ladder
    ax4 = fig.add_subplot(gs[1, 2])
    ax4.set_facecolor("#1a1a1a")
    ratios = [g["ratio"] for g in r["gear_data"]]
    ax4.plot(gears, ratios, "o-", color="#ffaa00", linewidth=2)
    ax4.set_title("Gear Ratio Ladder", **text_kw)
    ax4.set_xlabel("Gear", **text_kw); ax4.set_ylabel("Ratio", **text_kw)
    ax4.tick_params(colors="white"); ax4.spines[:].set_color("#444")

    out = "powertrain_report.png"
    plt.savefig(out, dpi=120, bbox_inches="tight", facecolor=fig.get_facecolor())
    print(f"📊 Chart saved → {out}")
    plt.show()


def main():
    parser = argparse.ArgumentParser(description="F1 2030 Powertrain Analyzer")
    parser.add_argument("--max-torque", type=float, default=DEFAULT["max_torque"])
    parser.add_argument("--max-rpm",    type=float, default=DEFAULT["max_rpm"])
    parser.add_argument("--no-plot",    action="store_true")
    args = parser.parse_args()

    cfg = DEFAULT.copy()
    cfg["max_torque"] = args.max_torque
    cfg["max_rpm"]    = args.max_rpm

    r = analyze(cfg)
    print_report(cfg, r)
    if not args.no_plot:
        plot_results(cfg, r)


if __name__ == "__main__":
    main()
