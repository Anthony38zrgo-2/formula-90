#!/usr/bin/env python3
"""
analyze_tires.py — F1 2030 Physics Diagnostics
===============================================
Analyzes tire lateral/longitudinal force capacity, sensitivity to surface
stiffness (Road vs Curb), slip angle behavior and rollover susceptibility.

Uses real parameters from f1_2026_car.tscn by default.

Usage:
    python analyze_tires.py
    python analyze_tires.py --curb-stiffness 12000

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
# DEFAULT CONFIG — from f1_2026_car.tscn
# ─────────────────────────────────────────────────────────────────────────────
DEFAULT = dict(
    vehicle_mass               = 708.0,
    front_weight_distribution  = 0.46,
    center_of_gravity_height   = 0.28,    # m
    front_track_width          = 1.573,   # m
    rear_track_width           = 1.473,   # m
    wheelbase                  = 3.24,    # m

    # Tire geometry
    front_tire_radius          = 0.370,   # m
    rear_tire_radius           = 0.370,   # m
    front_tire_width_mm        = 245.0,   # mm
    rear_tire_width_mm         = 365.0,   # mm
    contact_patch              = 0.22,    # m (GEVP param)
    front_wheel_mass           = 6.0,     # kg
    rear_wheel_mass            = 7.5,     # kg

    # Surface stiffnesses (N/m)
    tire_stiffness_road        = 22000.0,
    tire_stiffness_curb        = 9000.0,

    # Friction coefficients
    coef_friction_road         = 2.4,
    coef_friction_curb         = 1.2,
    coef_friction_gravel       = 0.6,
    coef_friction_grass        = 0.5,

    braking_grip_multiplier    = 1.15,

    # Suspension
    front_spring_length        = 0.15,
    rear_spring_length         = 0.20,
    front_resting_ratio        = 0.50,
    rear_resting_ratio         = 0.25,
)

G = 9.81


def analyze(cfg: dict) -> dict:
    m  = cfg["vehicle_mass"]
    fd = cfg["front_weight_distribution"]

    W_front_axle  = m * G * fd
    W_rear_axle   = m * G * (1.0 - fd)
    W_front_corner = W_front_axle / 2.0
    W_rear_corner  = W_rear_axle  / 2.0

    # ── Maximum lateral force per corner (Fy_max = μ * Fz) ──────────────
    def lat_force(Fz, mu): return mu * Fz
    def long_force(Fz, mu, brake_mult): return mu * brake_mult * Fz

    surfaces = {
        "Road":   (cfg["coef_friction_road"],   cfg["tire_stiffness_road"]),
        "Curb":   (cfg["coef_friction_curb"],   cfg["tire_stiffness_curb"]),
        "Gravel": (cfg["coef_friction_gravel"],  5000.0),
        "Grass":  (cfg["coef_friction_grass"],   3000.0),
    }

    surface_results = {}
    for surf, (mu, k_tire) in surfaces.items():
        Fy_front = lat_force(W_front_corner, mu)
        Fy_rear  = lat_force(W_rear_corner,  mu)
        Flong_front = long_force(W_front_corner, mu, cfg["braking_grip_multiplier"])
        Flong_rear  = long_force(W_rear_corner,  mu, cfg["braking_grip_multiplier"])

        # Max lateral G before sliding
        max_lat_g_front = Fy_front / (m * G / 4.0)
        max_lat_g_rear  = Fy_rear  / (m * G / 4.0)

        # Tire stiffness impulse on a 5cm bump (curb step)
        bump_height = 0.05   # m (typical curb)
        impulse_force = k_tire * bump_height  # N (simplified spring model)

        surface_results[surf] = dict(
            mu=mu, k_tire=k_tire,
            Fy_front=Fy_front, Fy_rear=Fy_rear,
            Flong_front=Flong_front, Flong_rear=Flong_rear,
            max_lat_g_front=max_lat_g_front,
            max_lat_g_rear=max_lat_g_rear,
            impulse_force=impulse_force,
        )

    # ── Slip angle model (simplified Pacejka-like) ────────────────────────
    # F_y = μ * Fz * sin(C * arctan(B * α)) — simplified version
    alpha_deg = np.linspace(0, 20, 200)
    alpha_rad = np.deg2rad(alpha_deg)

    B, C = 10.0, 1.3   # stiffness factor, shape factor (typical F1)
    Fy_front_slip = cfg["coef_friction_road"] * W_front_corner * np.sin(C * np.arctan(B * alpha_rad))
    Fy_rear_slip  = cfg["coef_friction_road"] * W_rear_corner  * np.sin(C * np.arctan(B * alpha_rad))

    peak_alpha_front = alpha_deg[np.argmax(Fy_front_slip)]
    peak_alpha_rear  = alpha_deg[np.argmax(Fy_rear_slip)]

    # ── Rollover sensitivity (track width / 2 / CG height) ────────────────
    rollover_g_front = (cfg["front_track_width"] / 2.0) / cfg["center_of_gravity_height"]
    rollover_g_rear  = (cfg["rear_track_width"]  / 2.0) / cfg["center_of_gravity_height"]

    # ── Curb stiffness ratio (Road vs Curb — sharp drop means instability) ──
    stiffness_drop_ratio = cfg["tire_stiffness_curb"] / cfg["tire_stiffness_road"]

    return dict(
        W_front_corner=W_front_corner, W_rear_corner=W_rear_corner,
        surface_results=surface_results,
        alpha_deg=alpha_deg,
        Fy_front_slip=Fy_front_slip, Fy_rear_slip=Fy_rear_slip,
        peak_alpha_front=peak_alpha_front, peak_alpha_rear=peak_alpha_rear,
        rollover_g_front=rollover_g_front, rollover_g_rear=rollover_g_rear,
        stiffness_drop_ratio=stiffness_drop_ratio,
    )


def print_report(cfg: dict, r: dict):
    print("\n" + "="*60)
    print("  F1 2030 — TIRE DIAGNOSTIC REPORT")
    print("="*60)

    print(f"\n⚖️  CORNER LOADS")
    print(f"   Front corner   : {r['W_front_corner']:.1f} N")
    print(f"   Rear corner    : {r['W_rear_corner']:.1f} N")

    print(f"\n🏁 SURFACE GRIP & LATERAL FORCE CAPACITY")
    print(f"   {'Surface':<10} {'μ':<8} {'k_tire N/m':<14} {'Fy_R (N)':<12} {'Max Lat G':<12} {'Curb Impulse (5cm)'}")
    print(f"   {'-'*70}")
    for surf, d in r["surface_results"].items():
        print(f"   {surf:<10} {d['mu']:<8.1f} {d['k_tire']:<14.0f} {d['Fy_rear']:<12.0f} {d['max_lat_g_rear']:<12.2f} {d['impulse_force']:.0f} N")

    print(f"\n📐 SLIP ANGLE PEAKS (Road surface)")
    print(f"   Front peak grip at: {r['peak_alpha_front']:.1f}°")
    print(f"   Rear  peak grip at: {r['peak_alpha_rear']:.1f}°")

    print(f"\n🔄 ROLLOVER THRESHOLD")
    print(f"   Front          : {r['rollover_g_front']:.2f} G")
    print(f"   Rear           : {r['rollover_g_rear']:.2f} G")

    ratio = r["stiffness_drop_ratio"]
    print(f"\n⚠️  STIFFNESS DROP Road→Curb : {ratio:.2f}x")
    if ratio < 0.50:
        print(f"   ⚠️  Very sharp stiffness change — tires will feel a sudden 'drop' on curbs, causing rear bounce")
    else:
        print(f"   ✅  Stiffness transition is reasonable")
    print()


def plot_results(cfg: dict, r: dict):
    fig = plt.figure(figsize=(14, 8), facecolor="#0f0f0f")
    fig.suptitle("F1 2030 — Tire Diagnostic", color="white", fontsize=14, fontweight="bold")
    gs = gridspec.GridSpec(2, 3, figure=fig, hspace=0.4, wspace=0.38)
    text_kw = dict(color="white")

    surfaces = list(r["surface_results"].keys())
    lat_g_rear  = [r["surface_results"][s]["max_lat_g_rear"]  for s in surfaces]
    impulses    = [r["surface_results"][s]["impulse_force"]    for s in surfaces]
    k_tires     = [r["surface_results"][s]["k_tire"]           for s in surfaces]
    colors = ["#00d4ff", "#ff8800", "#aaaaaa", "#00ff88"]

    # 1) Max lateral G per surface
    ax1 = fig.add_subplot(gs[0, 0])
    ax1.set_facecolor("#1a1a1a")
    bars = ax1.bar(surfaces, lat_g_rear, color=colors)
    ax1.set_title("Max Lateral G — Rear Corner", **text_kw)
    ax1.set_ylabel("G", **text_kw)
    ax1.tick_params(colors="white"); ax1.spines[:].set_color("#444")
    for bar, v in zip(bars, lat_g_rear):
        ax1.text(bar.get_x() + bar.get_width()/2, v + 0.03, f"{v:.2f}", ha="center", color="white", fontsize=9)

    # 2) Tire stiffness per surface
    ax2 = fig.add_subplot(gs[0, 1])
    ax2.set_facecolor("#1a1a1a")
    bars2 = ax2.bar(surfaces, k_tires, color=colors)
    ax2.set_title("Tire Stiffness per Surface (N/m)", **text_kw)
    ax2.set_ylabel("N/m", **text_kw)
    ax2.tick_params(colors="white"); ax2.spines[:].set_color("#444")
    for bar, v in zip(bars2, k_tires):
        ax2.text(bar.get_x() + bar.get_width()/2, v + 100, f"{v:.0f}", ha="center", color="white", fontsize=8)

    # 3) Curb impulse force (5cm bump)
    ax3 = fig.add_subplot(gs[0, 2])
    ax3.set_facecolor("#1a1a1a")
    bars3 = ax3.bar(surfaces, impulses, color=colors)
    ax3.set_title("Bump Impulse Force (5cm curb, N)", **text_kw)
    ax3.set_ylabel("N", **text_kw)
    ax3.tick_params(colors="white"); ax3.spines[:].set_color("#444")
    for bar, v in zip(bars3, impulses):
        ax3.text(bar.get_x() + bar.get_width()/2, v + 30, f"{v:.0f}", ha="center", color="white", fontsize=8)

    # 4) Slip angle curve — front
    ax4 = fig.add_subplot(gs[1, :2])
    ax4.set_facecolor("#1a1a1a")
    ax4.plot(r["alpha_deg"], r["Fy_front_slip"], color="#00d4ff", linewidth=2, label="Front")
    ax4.plot(r["alpha_deg"], r["Fy_rear_slip"],  color="#ff4444", linewidth=2, label="Rear")
    ax4.axvline(r["peak_alpha_front"], color="#00d4ff", linestyle=":", alpha=0.7)
    ax4.axvline(r["peak_alpha_rear"],  color="#ff4444", linestyle=":", alpha=0.7)
    ax4.set_title("Lateral Force vs Slip Angle (Road)", **text_kw)
    ax4.set_xlabel("Slip Angle (°)", **text_kw); ax4.set_ylabel("Lateral Force (N)", **text_kw)
    ax4.legend(facecolor="#222", labelcolor="white")
    ax4.tick_params(colors="white"); ax4.spines[:].set_color("#444")

    # 5) Rollover G
    ax5 = fig.add_subplot(gs[1, 2])
    ax5.set_facecolor("#1a1a1a")
    bars5 = ax5.bar(["Front", "Rear"], [r["rollover_g_front"], r["rollover_g_rear"]], color=["#00d4ff", "#ff4444"])
    ax5.axhline(y=3.0, color="#ffaa00", linestyle="--", linewidth=1, label="F1 corner ~3G")
    ax5.set_title("Rollover Threshold (G)", **text_kw)
    ax5.set_ylabel("G", **text_kw)
    ax5.legend(facecolor="#222", labelcolor="white", fontsize=8)
    ax5.tick_params(colors="white"); ax5.spines[:].set_color("#444")
    for bar, v in zip(bars5, [r["rollover_g_front"], r["rollover_g_rear"]]):
        ax5.text(bar.get_x() + bar.get_width()/2, v + 0.04, f"{v:.2f}G", ha="center", color="white", fontsize=10)

    out = "tire_report.png"
    plt.savefig(out, dpi=120, bbox_inches="tight", facecolor=fig.get_facecolor())
    print(f"📊 Chart saved → {out}")
    plt.show()


def main():
    parser = argparse.ArgumentParser(description="F1 2030 Tire Analyzer")
    parser.add_argument("--curb-stiffness", type=float, default=DEFAULT["tire_stiffness_curb"],  dest="tire_stiffness_curb")
    parser.add_argument("--road-stiffness", type=float, default=DEFAULT["tire_stiffness_road"],  dest="tire_stiffness_road")
    parser.add_argument("--no-plot",        action="store_true")
    args = parser.parse_args()

    cfg = DEFAULT.copy()
    cfg["tire_stiffness_curb"] = args.tire_stiffness_curb
    cfg["tire_stiffness_road"] = args.tire_stiffness_road

    r = analyze(cfg)
    print_report(cfg, r)
    if not args.no_plot:
        plot_results(cfg, r)


if __name__ == "__main__":
    main()
