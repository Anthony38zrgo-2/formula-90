# Reporte Acústico Comparativo: Renault R24 V10 (2004) vs. Formula-90 V10 Synth

**Ubicación del Archivo de Referencia**: `D:\Formula90s\docs\audio-design\F1 Classic Onboard_ Bahrain Grand Prix 2004, Alonso v Webber.mp3`  
**Vehículo y Motor de Referencia**: Renault R24 (Chasis R24-02 / Fernando Alonso, GP de Bahrein 2004), Motor Renault RS24 V10 a 72°, ~18,500 - 19,000 RPM, aspiración natural.  
**Señal de Comparación**: `reports/audio/sweep_5000_15000_10s.wav` y stems físicos diagnósticos (`reports/audio/sweep_5000_15000_10s_stems/`).  
**Herramientas Utilizadas**: Python (`miniaudio`, `scipy.signal.welch`, `scipy.signal.spectrogram`, `numpy`, `matplotlib`).

---

## 1. Gráficas Generadas de Comparativa Espectral

Se generaron dos figuras de alta resolución en la misma carpeta:
1. `renault_r24_vs_f90_spectral_comparison.png` — Comparativa directa de Densidad Espectral de Potencia (PSD) en escala logarítmica (20 Hz a 16 kHz).
2. `renault_r24_vs_f90_spectrogram.png` — Comparativa visual de espectrogramas temporales (Aceleración real en Bahrein vs. Barrido 5,000 -> 15,000 RPM de la simulación).

---

## 2. Distribución de Energía por Bandas Acústicas

Se calculó la potencia espectral integrada normalizada en 7 bandas críticas del espectro de audio:

| Banda Acústica | Renault R24 (Referencia) | Simulación Híbrida (F90) | Simulación Física Pura | Escena Acústica (`scene_mix`) | Diagnóstico del Desbalance |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **Sub-bass (20 – 80 Hz)** | **0.71%** | 0.31% | 0.34% | 0.21% | Control adecuado en ambas. |
| **Bass / Chassis (80 – 250 Hz)** | **1.53%** | **37.73%** | **48.22%** | **36.44%** | **Exceso crítico en F90 (+36%)**. El micrófono de cámara onboard (T-cam) no captura retumbos graves de cárter. F90 tiene demasiada energía en los modos de cárter (86-196 Hz) y presión directa. |
| **Low-Mid / Engine Roar (250 – 800 Hz)** | **33.16%** | **23.90%** | 12.73% | 24.56% | F90 está ~10% por debajo. Falta presencia del gruñido de escape y el semiórden de banco (Order 0.5). |
| **Mid / Fundamental Firing (800 – 1600 Hz)** | **46.97%** | **21.96%** | 32.81% | 26.13% | **Déficit crítico en F90 (-25%)**. En el R24 real, casi la mitad de toda la energía acústica está concentrada en el grito fundamental de encendido ($f_0$). En F90 está diluida por el exceso de graves. |
| **High-Mid / Airbox Bite (1600 – 3500 Hz)** | **13.36%** | **10.31%** | 2.26% | 7.70% | Buena aproximación en el híbrido gracias a los samples, pero la física cruda carece de mordida de inducción de aire. |
| **Presence / Harmonics (3500 – 7000 Hz)** | **2.29%** | **1.11%** | 0.03% | 0.47% | El R24 real tiene saturación de preamplificador y raspado de choque que aporta brillo metálico controlado. |
| **Treble / Air (7000 – 15000 Hz)** | **1.50%** | **0.61%** | 0.00% | 0.51% | Caída natural por absorción de aire. |

---

## 3. Jerarquía de Órdenes Armónicos en el Renault RS24 V10

Analizando un instante de aceleración a fondo sostenido a 14,600 RPM ($f_0 = 1,216.6\text{ Hz}$):

* **Orden 1.0 ($f_0 = 1,216.6\text{ Hz}$ — Fundamental de los 10 cilindros)**: **0.0 dB** (Pico dominante absoluto).
* **Orden 0.5 ($608.3\text{ Hz}$ — Fundamental del banco de 5 cilindros)**: **-5.2 dB**.
  * *Hallazgo clave*: La pulsación de cada banco individual de 5 cilindros es casi tan fuerte como el encendido total (sólo 5.2 dB por debajo). Esto le otorga al V10 ese sonido gutural y asimétrico característico de "rasgado", evitando sonar como un zumbador estéril o un sintetizador continuo.
* **Orden 2.0 ($2,433.3\text{ Hz}$ — 2º armónico)**: **-14.9 dB**.
  * En el coche real, el 2º armónico decae 15 dB de forma natural respecto a la fundamental.
* **Orden 1.5 y 2.5 (Intermodulación de bancos a $1,825\text{ Hz}$ y $3,042\text{ Hz}$)**: **-22.5 dB** y **-19.3 dB**.
* **Orden 3.0 ($3,650\text{ Hz}$)**: **-24.9 dB**.
* **Orden 4.0 ($4,867\text{ Hz}$)**: **-30.7 dB**.

---

## 4. Dinámica, Factor de Cresta y Saturación de Micrófono

| Parámetro | Renault R24 (Referencia) | Formula-90 Híbrido Actual | Diagnóstico |
| :--- | :---: | :---: | :--- |
| **Factor de Cresta (Peak / RMS)** | **10.34 dB** | **15.68 dB** | El audio real está **mucho más saturado y comprimido** por la presión sonora extrema sobre la cápsula del micrófono en el T-cam y el preamplificador de transmisión. F90 tiene transitorios demasiado picudos y vacíos entre ciclos. |
| **Comportamiento en Lift-off / Overrun** | Caída de RMS mínima (-1.2 dB), transición a quejido de transmisión y raspado de retención. | Caída abrupta de nivel al bajar carga a 0.10. | En el R24 real, al soltar el acelerador la sonoridad no desaparece: el motor de combustión reduce su volumen pero el arrastre, la retención de escape y el quejido de la caja de cambios mantienen la cabina llena de sonido. |

---

## 5. Las 5 Brechas Principales (Gaps) Identificadas

1. **Exceso masivo de frecuencias graves (80–250 Hz)**:
   * En la simulación actual, casi el **38–48% de la energía** proviene del retumbo de baja frecuencia de los resonadores del bloque/cárter y la presión directa.
   * En un onboard real de F1 (micrófono en la toma de aire/roll-hoop), las frecuencias por debajo de 250 Hz son mínimas (1.5%).
2. **Déficit de la Frecuencia Fundamental de Encendido (800–1600 Hz)**:
   * El grito del V10 real está dominado en un **47%** por el orden 1.0 ($f_0$). En F90 representa apenas el **22%**.
3. **Falta de Presencia del Orden 0.5 (Desbalance de Bancos 5-a-1)**:
   * En el R24, el banco de 5 cilindros aporta una componente a $f_0 / 2$ a sólo -5.2 dB del tono principal, produciendo la "textura rasgada" del V10.
4. **Falta de Compresión y Saturación Dinámica de Cápsula**:
   * El sonido real tiene un factor de cresta de 10.3 dB (sonido como una pared sólida y densa de energía). F90 tiene 15.7 dB, sonando más limpio y "quirúrgico", pero menos visceral y rugiente.
5. **Comportamiento en Lift and Coast (Desaceleración)**:
   * Al levantar el acelerador, la simulación física apaga casi por completo la excitación acústica, mientras que en el coche real la deceleración produce resonancia de vacío en el colector, chisporroteo y retención acústica.
