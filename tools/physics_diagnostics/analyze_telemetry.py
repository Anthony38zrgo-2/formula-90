#!/usr/bin/env python3
"""
analyze_telemetry.py — F1 2030 Telemetry Parser
================================================
Reads a generated telemetry CSV file, computes statistics, and automatically
detects anomalous physics behavior (e.g. chassis bottoming-out, extreme G-forces).

Usage:
    python analyze_telemetry.py <path_to_csv>
"""

import argparse
import sys
import os
import pandas as pd

# Ensure UTF-8 output on Windows for emojis
if sys.stdout.encoding.lower() != 'utf-8':
    sys.stdout.reconfigure(encoding='utf-8')

def analyze_telemetry(csv_path: str):
    if not os.path.exists(csv_path):
        print(f"❌ Error: Archivo no encontrado -> {csv_path}")
        sys.exit(1)

    print("\n" + "█"*60)
    print(f"  F1 2030 — TELEMETRY ANALYSIS")
    print(f"  File: {os.path.basename(csv_path)}")
    print("█"*60)

    try:
        df = pd.read_csv(csv_path)
    except Exception as e:
        print(f"❌ Error leyendo CSV: {e}")
        sys.exit(1)

    if df.empty:
        print("⚠️  El archivo CSV está vacío.")
        sys.exit(0)

    # Convertir a numérico si hay problemas de tipo
    for col in df.columns:
        df[col] = pd.to_numeric(df[col], errors='coerce')

    # Basic stats
    duration_s = (df['Time_ms'].max() - df['Time_ms'].min()) / 1000.0
    max_speed = df['Speed_kmh'].max()
    max_rpm = df['RPM'].max()
    
    max_lat_g = df['Lat_G'].abs().max()
    max_long_g = df['Long_G'].abs().max()
    min_long_g = df['Long_G'].min()  # Deceleración
    
    max_fl_comp = df['FL_Comp'].max()
    max_fr_comp = df['FR_Comp'].max()
    max_rl_comp = df['RL_Comp'].max()
    max_rr_comp = df['RR_Comp'].max()
    
    max_front_slip = df['Front_Slip'].max()
    max_rear_slip = df['Rear_Slip'].max()

    print("\n📊 ESTADÍSTICAS GENERALES:")
    print(f"  Duración sesión : {duration_s:.1f} segundos ({len(df)} muestras)")
    print(f"  Velocidad Máx   : {max_speed:.1f} km/h")
    print(f"  RPM Máximas     : {max_rpm:.0f} RPM")
    
    print("\n💥 FUERZAS G (Picos Absolutos):")
    print(f"  Lateral Máx     : {max_lat_g:.2f} G")
    print(f"  Frenada Máx     : {min_long_g:.2f} G (Longitudinal mínima)")

    print("\n⚙️  SUSPENSIÓN (Compresión Máxima registrada):")
    print(f"  Front Left  : {max_fl_comp:.1f} mm")
    print(f"  Front Right : {max_fr_comp:.1f} mm")
    print(f"  Rear Left   : {max_rl_comp:.1f} mm")
    print(f"  Rear Right  : {max_rr_comp:.1f} mm")

    print("\n🛞 DESLIZAMIENTO DE NEUMÁTICOS (Picos):")
    print(f"  Eje Delantero : {max_front_slip:.2f}")
    print(f"  Eje Trasero   : {max_rear_slip:.2f}")

    print("\n" + "="*60)
    print("  🚨 DIAGNÓSTICO AUTOMÁTICO DE ANOMALÍAS")
    print("="*60)
    
    anomalies = 0

    # Detección de Bottom-Out (Chasis chocando)
    # Suponemos que 150mm es el recorrido total en f1_2026_car.tscn
    if max_fl_comp >= 148.0 or max_fr_comp >= 148.0:
        print("  ⚠️  [BOTTOM-OUT FRONTAL DETECTADO]: La suspensión delantera colapsó por completo (>148mm). El chasis está golpeando el suelo.")
        anomalies += 1
    if max_rl_comp >= 198.0 or max_rr_comp >= 198.0:
        print("  ⚠️  [BOTTOM-OUT TRASERO DETECTADO]: La suspensión trasera colapsó por completo (>198mm). El chasis está golpeando el suelo.")
        anomalies += 1

    # Detección de Colisiones / Gs absurdos
    if max_lat_g > 4.5 and max_speed < 100:
        print(f"  ⚠️  [IMPACTO RÍGIDO]: Se detectaron {max_lat_g:.1f}G laterales a baja velocidad. Esto indica un choque del chasis contra un bordillo, no fuerzas de neumáticos.")
        anomalies += 1
    if min_long_g < -6.0:
        print(f"  ⚠️  [IMPACTO LONGITUDINAL]: Deceleración de {min_long_g:.1f}G. Choque frontal severo contra la pista o muro.")
        anomalies += 1

    # Detección de Wheel Spin / Bloqueo
    if max_rear_slip > 15.0 and max_speed > 20:
        print("  ⚠️  [WHEEL SPIN TRASERO MASIVO]: Pérdida total de tracción en el eje trasero (Slip > 15).")
        anomalies += 1
    if max_front_slip > 25.0:
        print("  ⚠️  [SUBVIRAJE MASIVO / BLOQUEO FRONTAL]: Deslizamiento extremo en ruedas delanteras. Ruedas bloqueadas o sin peso.")
        anomalies += 1

    if anomalies == 0:
        print("  ✅  No se detectaron anomalías mecánicas graves en esta sesión.")
    
    print("="*60 + "\n")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Analiza CSV de Telemetría")
    parser.add_argument("csv_path", type=str, help="Ruta al archivo CSV a analizar")
    args = parser.parse_args()
    
    analyze_telemetry(args.csv_path)
