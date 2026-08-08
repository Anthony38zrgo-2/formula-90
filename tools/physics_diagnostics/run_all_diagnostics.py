#!/usr/bin/env python3
"""
run_all_diagnostics.py — F1 2030 Physics Diagnostics
=====================================================
Runs all diagnostic tools in sequence (no-plot mode) and
prints a unified summary report. Useful for quick AI triage.

Usage:
    python run_all_diagnostics.py
    python run_all_diagnostics.py --plot
"""
import subprocess
import sys

if sys.stdout.encoding.lower() != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')
import argparse
import os

# Ensure UTF-8 output on Windows for emojis and special characters
if sys.stdout.encoding.lower() != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')

# Make sure sibling scripts are importable
sys.path.insert(0, os.path.dirname(__file__))

import analyze_suspension as susp_mod
import analyze_powertrain as pt_mod
import analyze_aero_and_weight as aero_mod
import analyze_tires as tire_mod
from tscn_parser import update_config_from_tscn

def main():
    parser = argparse.ArgumentParser(description="F1 2030 Full Physics Diagnostic Suite")
    parser.add_argument("--plot", action="store_true", help="Show matplotlib charts after each analysis")
    args = parser.parse_args()

    no_plot = not args.plot

    print("\n" + "█"*60)
    print("  F1 2030 — FULL PHYSICS DIAGNOSTIC SUITE")
    print("  Using: f1_2026_car.tscn defaults (V10 config)")
    print("█"*60)

    # ── Suspension ──────────────────────────────────────────────────────────
    cfg_s = susp_mod.DEFAULT.copy()
    cfg_s = update_config_from_tscn(cfg_s, "game/scenes/vehicles/f1_2026_car.tscn")
    r_s   = susp_mod.analyze(cfg_s)
    susp_mod.print_report(cfg_s, r_s)
    if not no_plot:
        step_resp = susp_mod.simulate_step_response(cfg_s, r_s)
        susp_mod.plot_results(cfg_s, r_s, step_resp)

    # ── Powertrain ──────────────────────────────────────────────────────────
    cfg_p = pt_mod.DEFAULT.copy()
    cfg_p = update_config_from_tscn(cfg_p, "game/scenes/vehicles/f1_2026_car.tscn")
    r_p   = pt_mod.analyze(cfg_p)
    pt_mod.print_report(cfg_p, r_p)
    if not no_plot:
        pt_mod.plot_results(cfg_p, r_p)

    # ── Aero & Weight ────────────────────────────────────────────────────────
    cfg_a = aero_mod.DEFAULT.copy()
    cfg_a = update_config_from_tscn(cfg_a, "game/scenes/vehicles/f1_2026_car.tscn")
    r_a   = aero_mod.analyze(cfg_a)
    aero_mod.print_report(cfg_a, r_a)
    if not no_plot:
        aero_mod.plot_results(cfg_a, r_a)

    # ── Tires ────────────────────────────────────────────────────────────────
    cfg_t = tire_mod.DEFAULT.copy()
    cfg_t = update_config_from_tscn(cfg_t, "game/scenes/vehicles/f1_2026_car.tscn")
    r_t   = tire_mod.analyze(cfg_t)
    tire_mod.print_report(cfg_t, r_t)
    if not no_plot:
        tire_mod.plot_results(cfg_t, r_t)

    # ── CONSOLIDATED RISK SUMMARY ────────────────────────────────────────────
    print("\n" + "="*60)
    print("  ⚠️  CONSOLIDATED RISK FLAGS")
    print("="*60)

    flags = []
    if r_s["freq_ratio"] > 1.10:
        flags.append(f"[SUSPENSION] Rear freq ({r_s['f_rear']:.2f}Hz) >> Front ({r_s['f_front']:.2f}Hz) → curb bounce/rollover risk")
    if cfg_s["rear_bump_stop_mult"] < 1.5:
        flags.append(f"[SUSPENSION] rear_bump_stop_mult={cfg_s['rear_bump_stop_mult']:.1f} — chassis may bottom-out on curbs")
    if r_s["rollover_g_rear"] < 2.8:
        flags.append(f"[SUSPENSION] Rear rollover threshold {r_s['rollover_g_rear']:.2f}G — close to F1 cornering envelope")
    if r_t["stiffness_drop_ratio"] < 0.50:
        flags.append(f"[TIRES] Road→Curb stiffness drop {r_t['stiffness_drop_ratio']:.2f}x — sudden instability on curb entry")
    if r_p["peak_power_hp"] < 700:
        flags.append(f"[POWERTRAIN] Peak power {r_p['peak_power_hp']:.0f}HP — below expected V10 range (750-850HP)")
    if r_a["rear_lift_off_g"] < 1.5:
        flags.append(f"[AERO/WEIGHT] Rear axle unloads at {r_a['rear_lift_off_g']:.2f}G braking — instability under trail braking")

    if flags:
        for f in flags:
            print(f"   ⚠️  {f}")
    else:
        print("   ✅  No critical risk flags detected.")

    print("\n" + "="*60 + "\n")


if __name__ == "__main__":
    main()
