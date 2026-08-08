#!/usr/bin/env python3
"""
analyze_aero_and_weight.py — F1 2030 Physics Diagnostics
=========================================================
Analyzes aerodynamic drag, downforce, weight transfer under braking/cornering,
and static vs dynamic load distribution.

Uses real parameters from f1_2026_car.tscn by default.

Usage:
    python analyze_aero_and_weight.py
    python analyze_aero_and_weight.py --cd 1.1 --frontal-area 1.6

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
    vehicle_mass               = 708.0,   # kg
    front_weight_distribution  = 0.46,
    center_of_gravity_height   = 0.28,    # m (effective ride CG)
    wheelbase                  = 3.24,    # m
    front_track_width          = 1.573,   # m
    rear_track_width           = 1.473,   # m

    coefficient_of_drag        = 0.95,    # Cd
    frontal_area               = 1.50,    # m²
    air_density                = 1.225,   # kg/m³

    # Approximated downforce coefficient for a modern F1 car (no ERS)
    # F_downforce = 0.5 * rho * Cl * A * V²
    # For realistic feel, Cl ≈ 3.5x the Cd for a high-downforce F1
    coefficient_of_lift        = 3.50,    # Cl (negative lift = downforce)
)

G = 9.81


def analyze(cfg: dict) -> dict:
    m     = cfg["vehicle_mass"]
    fd    = cfg["front_weight_distribution"]
    h_cg  = cfg["center_of_gravity_height"]
    wb    = cfg["wheelbase"]
    tw_f  = cfg["front_track_width"]
    tw_r  = cfg["rear_track_width"]

    speeds_kmh = np.array([0, 50, 100, 150, 200, 250, 300, 320, 340])
    speeds_ms  = speeds_kmh / 3.6

    rho = cfg["air_density"]
    Cd  = cfg["coefficient_of_drag"]
    Cl  = cfg["coefficient_of_lift"]
    A   = cfg["frontal_area"]

    drag      = 0.5 * rho * Cd * A * speeds_ms**2   # N
    downforce = 0.5 * rho * Cl * A * speeds_ms**2   # N (adds to normal load)
    drag_kn   = drag / 1000
    df_kn     = downforce / 1000

    static_weight = m * G                            # N
    static_front  = static_weight * fd               # N
    static_rear   = static_weight * (1.0 - fd)       # N

    # ── Load transfer under braking ──────────────────────────────────────
    decel_g_range = np.linspace(0, 5.0, 200)
    lt_brake      = (m * G * decel_g_range * h_cg) / wb   # N shift to front
    front_load_brake = static_front + lt_brake
    rear_load_brake  = static_rear  - lt_brake
    rear_lift_off_g  = static_rear / ((m * G * h_cg) / wb)  # G where rear goes 0

    # ── Load transfer in cornering ────────────────────────────────────────
    # Lateral transfer: ΔF = m * ay * h_cg / track_width
    lat_g_range   = np.linspace(0, 5.0, 200)
    lt_front_lat  = (m * G * lat_g_range * h_cg) / tw_f
    lt_rear_lat   = (m * G * lat_g_range * h_cg) / tw_r

    rollover_front_g = (tw_f / 2.0) / h_cg
    rollover_rear_g  = (tw_r / 2.0) / h_cg

    # ── Downforce effect on effective weight at speeds ────────────────────
    effective_weight_at_speed = static_weight + downforce

    return dict(
        speeds_kmh=speeds_kmh, speeds_ms=speeds_ms,
        drag=drag, drag_kn=drag_kn, downforce=downforce, df_kn=df_kn,
        static_weight=static_weight, static_front=static_front, static_rear=static_rear,
        decel_g_range=decel_g_range, lt_brake=lt_brake,
        front_load_brake=front_load_brake, rear_load_brake=rear_load_brake,
        rear_lift_off_g=rear_lift_off_g,
        lat_g_range=lat_g_range,
        lt_front_lat=lt_front_lat, lt_rear_lat=lt_rear_lat,
        rollover_front_g=rollover_front_g, rollover_rear_g=rollover_rear_g,
        effective_weight_at_speed=effective_weight_at_speed,
    )


def print_report(cfg: dict, r: dict):
    print("\n" + "="*60)
    print("  F1 2030 — AERO & WEIGHT DIAGNOSTIC REPORT")
    print("="*60)

    print(f"\n⚖️  STATIC WEIGHT")
    print(f"   Total          : {r['static_weight']:.0f} N  ({cfg['vehicle_mass']:.0f} kg)")
    print(f"   Front axle     : {r['static_front']:.0f} N  ({cfg['front_weight_distribution']*100:.0f}%)")
    print(f"   Rear axle      : {r['static_rear']:.0f} N  ({(1-cfg['front_weight_distribution'])*100:.0f}%)")

    print(f"\n💨 AERODYNAMIC FORCES")
    print(f"   {'Speed (km/h)':<14} {'Drag (N)':<12} {'Drag (kN)':<12} {'Downforce (kN)':<16} {'Eff. Weight (N)'}")
    print(f"   {'-'*65}")
    for i, spd in enumerate(r["speeds_kmh"]):
        print(f"   {spd:<14.0f} {r['drag'][i]:<12.0f} {r['drag_kn'][i]:<12.2f} {r['df_kn'][i]:<16.2f} {r['effective_weight_at_speed'][i]:.0f}")

    print(f"\n🛑 BRAKING LOAD TRANSFER")
    print(f"   Rear lift-off at: {r['rear_lift_off_g']:.2f} G deceleration")

    print(f"\n🔄 ROLLOVER THRESHOLD")
    print(f"   Front          : {r['rollover_front_g']:.2f} G")
    print(f"   Rear           : {r['rollover_rear_g']:.2f} G")
    print()


def plot_results(cfg: dict, r: dict):
    fig = plt.figure(figsize=(15, 8), facecolor="#0f0f0f")
    fig.suptitle("F1 2030 — Aero & Weight Diagnostic", color="white", fontsize=14, fontweight="bold")
    gs = gridspec.GridSpec(2, 3, figure=fig, hspace=0.4, wspace=0.35)
    text_kw = dict(color="white")

    # 1) Drag vs Speed
    ax1 = fig.add_subplot(gs[0, 0])
    ax1.set_facecolor("#1a1a1a")
    ax1.plot(r["speeds_kmh"], r["drag_kn"], color="#ff4444", linewidth=2)
    ax1.set_title("Drag Force vs Speed", **text_kw)
    ax1.set_xlabel("km/h", **text_kw); ax1.set_ylabel("kN", **text_kw)
    ax1.tick_params(colors="white"); ax1.spines[:].set_color("#444")

    # 2) Downforce vs Speed
    ax2 = fig.add_subplot(gs[0, 1])
    ax2.set_facecolor("#1a1a1a")
    ax2.plot(r["speeds_kmh"], r["df_kn"], color="#00d4ff", linewidth=2)
    ax2.axhline(y=r["static_weight"]/1000, color="#ffaa00", linestyle="--", linewidth=1, label="Car weight")
    ax2.set_title("Downforce vs Speed", **text_kw)
    ax2.set_xlabel("km/h", **text_kw); ax2.set_ylabel("kN", **text_kw)
    ax2.legend(facecolor="#222", labelcolor="white", fontsize=8)
    ax2.tick_params(colors="white"); ax2.spines[:].set_color("#444")

    # 3) Front/Rear load under braking
    ax3 = fig.add_subplot(gs[0, 2])
    ax3.set_facecolor("#1a1a1a")
    ax3.plot(r["decel_g_range"], r["front_load_brake"]/1000, color="#00ff88", label="Front axle", linewidth=2)
    ax3.plot(r["decel_g_range"], r["rear_load_brake"]/1000,  color="#ff4444", label="Rear axle",  linewidth=2)
    ax3.axhline(y=0, color="white", linestyle="--", linewidth=0.8)
    ax3.axvline(x=r["rear_lift_off_g"], color="#ffaa00", linestyle=":", linewidth=1, label=f"Rear lift-off {r['rear_lift_off_g']:.1f}G")
    ax3.set_title("Load Transfer — Braking", **text_kw)
    ax3.set_xlabel("Deceleration (G)", **text_kw); ax3.set_ylabel("Axle Load (kN)", **text_kw)
    ax3.legend(facecolor="#222", labelcolor="white", fontsize=8)
    ax3.tick_params(colors="white"); ax3.spines[:].set_color("#444")

    # 4) Lateral load transfer front
    ax4 = fig.add_subplot(gs[1, 0])
    ax4.set_facecolor("#1a1a1a")
    ax4.plot(r["lat_g_range"], r["lt_front_lat"]/1000, color="#00d4ff", linewidth=2)
    ax4.axvline(x=r["rollover_front_g"], color="#ff4444", linestyle="--", linewidth=1.2, label=f"Rollover {r['rollover_front_g']:.1f}G")
    ax4.set_title("Front Lateral Load Transfer", **text_kw)
    ax4.set_xlabel("Lateral G", **text_kw); ax4.set_ylabel("Transfer (kN)", **text_kw)
    ax4.legend(facecolor="#222", labelcolor="white", fontsize=8)
    ax4.tick_params(colors="white"); ax4.spines[:].set_color("#444")

    # 5) Lateral load transfer rear
    ax5 = fig.add_subplot(gs[1, 1])
    ax5.set_facecolor("#1a1a1a")
    ax5.plot(r["lat_g_range"], r["lt_rear_lat"]/1000, color="#ff8800", linewidth=2)
    ax5.axvline(x=r["rollover_rear_g"], color="#ff4444", linestyle="--", linewidth=1.2, label=f"Rollover {r['rollover_rear_g']:.1f}G")
    ax5.set_title("Rear Lateral Load Transfer", **text_kw)
    ax5.set_xlabel("Lateral G", **text_kw); ax5.set_ylabel("Transfer (kN)", **text_kw)
    ax5.legend(facecolor="#222", labelcolor="white", fontsize=8)
    ax5.tick_params(colors="white"); ax5.spines[:].set_color("#444")

    # 6) Effective total weight at speed
    ax6 = fig.add_subplot(gs[1, 2])
    ax6.set_facecolor("#1a1a1a")
    ax6.plot(r["speeds_kmh"], r["effective_weight_at_speed"]/1000, color="#00ff88", linewidth=2)
    ax6.axhline(y=r["static_weight"]/1000, color="#ffaa00", linestyle="--", linewidth=1, label=f"Static {r['static_weight']/1000:.1f}kN")
    ax6.set_title("Effective Total Normal Load vs Speed", **text_kw)
    ax6.set_xlabel("km/h", **text_kw); ax6.set_ylabel("kN", **text_kw)
    ax6.legend(facecolor="#222", labelcolor="white", fontsize=8)
    ax6.tick_params(colors="white"); ax6.spines[:].set_color("#444")

    out = "aero_weight_report.png"
    plt.savefig(out, dpi=120, bbox_inches="tight", facecolor=fig.get_facecolor())
    print(f"📊 Chart saved → {out}")
    plt.show()


def main():
    parser = argparse.ArgumentParser(description="F1 2030 Aero & Weight Analyzer")
    parser.add_argument("--cd",           type=float, default=DEFAULT["coefficient_of_drag"])
    parser.add_argument("--frontal-area", type=float, default=DEFAULT["frontal_area"])
    parser.add_argument("--mass",         type=float, default=DEFAULT["vehicle_mass"])
    parser.add_argument("--no-plot",      action="store_true")
    args = parser.parse_args()

    cfg = DEFAULT.copy()
    cfg["coefficient_of_drag"] = args.cd
    cfg["frontal_area"]        = args.frontal_area
    cfg["vehicle_mass"]        = args.mass

    r = analyze(cfg)
    print_report(cfg, r)
    if not args.no_plot:
        plot_results(cfg, r)


if __name__ == "__main__":
    main()
