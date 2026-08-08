# Physics Diagnostics — F1 2030

Suite de herramientas de análisis físico **offline** (Python) para diagnosticar el comportamiento del vehículo antes de hacer cambios en el código del juego.

Uso previsto: la IA (Antigravity o DeepSeek) debe ejecutar estas herramientas cuando encuentre un bug de físicas recurrente, en lugar de iterar código a ciegas.

## Instalación rápida
```bash
pip install -r requirements.txt
```

## Herramientas disponibles

| Script | Qué diagnostica | Cuándo usarlo |
|---|---|---|
| `analyze_suspension.py` | Frecuencias naturales, amortiguamiento, bump-stop, umbral de vuelco | Bug de rebote, vuelco en bordillos, comportamiento nervioso |
| `analyze_powertrain.py` | Curva de torque V10, potencia, velocidades por marcha, freno motor | RPM erróneas, cambios de marcha incorrectos, falta de potencia |
| `analyze_aero_and_weight.py` | Drag, downforce, transferencia de peso al frenar y en curva | Subviraje/sobreviraje crónico, inestabilidad en frenada |
| `analyze_tires.py` | Grip por superficie, slip angle, impulso en bordillos, stiffness drop | Grip excesivo/insuficiente, comportamiento diferente en asfalto vs bordillo |
| `run_all_diagnostics.py` | Ejecuta todo y produce un resumen de flags de riesgo | Primera triage antes de empezar cualquier debugging |

## Uso básico
```bash
# Triage completo (sin gráficas — ideal para la IA)
python run_all_diagnostics.py

# Análisis de suspensión con gráficas
python analyze_suspension.py

# Probar parámetros alternativos
python analyze_suspension.py --rear-spring 0.18 --rear-damping 0.90 --rear-bump-mult 2.5
python analyze_powertrain.py --max-torque 380 --max-rpm 19000
python analyze_tires.py --curb-stiffness 12000
```

## Notas para agentes AI
- Todos los defaults están sincronizados con `f1_2026_car.tscn` (V10 config).
- Cada script detecta y reporta condiciones de riesgo con emojis `⚠️` y `✅`.
- Los gráficos se guardan como PNG en el directorio de trabajo (`suspension_report.png`, etc.).
- **No mezcles estos scripts con el runtime del juego.** Son herramientas de análisis offline puro.
