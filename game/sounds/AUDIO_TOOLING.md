# Sound Design & Síntesis Aditiva Headless — Formula-90

Guía de uso del ecosistema de audio **Python** para este proyecto. Todo corre
**headless** (sin GUI, sin DAW) y se integra con la pipeline existente en
`tools/audio/` (`dsp_common.py`, `bank_generator.py`, `bank_validator.py`).

> Entorno: `.venv` (Python 3.14). No hay `ffmpeg` ni `sox` instalados, así que
> se evitan librerías que dependen de ellos (`pydub`, `pysox`) y se usan
> alternativas autocontenidas (`pedalboard`, `numpy`/`scipy`/`soundfile`).

---

## 1. Stack disponible

| Paquete | Aporta | Depende de | Headless |
|---|---|---|---|
| `numpy` | Generación de partiales (suma de senos), envolventes, slicing | — | ✅ |
| `scipy` | FFT, filtros (`scipy.signal`: lowpass anti-alias, EQ), convolución (reverb IR) | `numpy` | ✅ |
| `soundfile` | Leer/escribir WAV/FLAC/OGG (libsndfile) sin ffmpeg | `numpy` | ✅ |
| `matplotlib` | Espectrogramas, waveform, validación visual | `numpy` | ✅ |
| `numba` | JIT: acelera la suma de miles de armónicos (aditiva) | `llvmlite` | ✅ |
| `resampy` | Resample de alta calidad (unificar SR del banco) | `numpy`/`scipy` | ✅ |
| `pedalboard` | Efectos sobre samples: reverb IR, compressor, pitch-shift, delay, distorsión, VST | autocontenido (core C++) | ✅ |
| `pyloudnorm` | Normalización EBU R128 (emparejar loudness del banco) | `numpy` | ✅ |
| `librosa` | `pitch_shift`, `time_stretch`, `tone()`, features, espectrogramas | `numba`/`pooch` (pesado) | ✅ |

**Versiones instaladas y verificadas (`.venv`, Python 3.14):**
`numpy 2.5.2`, `scipy 1.18.0`, `soundfile 0.14.0`, `matplotlib 3.11.1`,
`numba 0.67.0`, `resampy 0.4.3`, `pedalboard 0.9.24`, `pyloudnorm 0.2.0`,
`librosa 1.0.0`.

No instalados a propósito: `pydub` (necesita ffmpeg), `pysox` (necesita sox),
`ffmpeg`, `sox`, `csound`, `supercollider`, `faust`, `puredata`, `fluidsynth`,
`timidity`, `lua`. Ver `tools/audio/` para la síntesis aditiva real.

---

## 2. Flujo de trabajo recomendado (pipeline)

```
partials (numpy+numba)  ->  envolvente  ->  anti-alias (scipy)
   ->  efectos (pedalboard)  ->  resample (resampy)
   ->  loudness (pyloudnorm)  ->  export WAV (soundfile)
   ->  validación (matplotlib + bank_validator)
```

Cada paso es una función pura que `bank_generator.py` puede llamar al generar
un sample del banco `v10_vehicle`.

---

## 3. Ejemplos

### 3.1 Síntesis aditiva (núcleo) con `numpy` + `numba`

```python
import numpy as np
from numba import njit

@njit(cache=True)
def additive_synthesis(f0: float, sr: int, dur: float,
                      harmonic_amps: np.ndarray) -> np.ndarray:
    """Suma de senos: f0, 2*f0, 3*f0, ... con amplitudes por armónico."""
    n = int(dur * sr)
    t = np.arange(n) / sr
    out = np.zeros(n)
    for k in range(harmonic_amps.shape[0]):
        out += harmonic_amps[k] * np.sin(2 * np.pi * f0 * (k + 1) * t)
    return out

# Ejemplo: motor V10 estilo — armónicos pares/impares con caída 1/k
sr, dur = 44100, 2.0
amps = np.array([1.0, 0.6, 0.4, 0.25, 0.18, 0.12, 0.08, 0.05], dtype=np.float64)
sig = additive_synthesis(f0=120.0, sr=sr, dur=dur, harmonic_amps=amps)
```

### 3.2 Envolvente (ADSR simple)

```python
def adsr(n: int, sr: int, a=0.02, d=0.10, s=0.7, r=0.30) -> np.ndarray:
    env = np.ones(n)
    na, nd, nr = int(a*sr), int(d*sr), int(r*sr)
    ns = n - na - nd - nr
    env[:na]  = np.linspace(0, 1, na)
    env[na:na+nd] = np.linspace(1, s, nd)
    env[na+nd:na+nd+ns] = s
    env[na+nd+ns:] = np.linspace(s, 0, nr)
    return env

sig = sig * adsr(len(sig), sr)
```

### 3.3 Anti-alias (lowpass con `scipy`)

```python
from scipy.signal import butter, sosfilt

def anti_alias(x: np.ndarray, sr: int, cutoff: float) -> np.ndarray:
    nyq = 0.5 * sr
    sos = butter(4, cutoff / nyq, btype="low", output="sos")
    return sosfilt(sos, x)

sig = anti_alias(sig, sr, cutoff=min(sr/2 - 1000, 8000))
```

### 3.4 Efectos sobre samples existentes (`pedalboard`)

```python
import soundfile as sf
from pedalboard import Pedalboard, Compressor, Reverb, PitchShift

data, sr = sf.read("game/sounds/banks/v10_vehicle/engine_idle.wav")
board = Pedalboard([
    Compressor(threshold_db=-18, ratio=4),
    Reverb(room_size=0.35, wet_level=0.25),
    PitchShift(semitones=-2),   # bajar tono del motor
])
processed = board(data, sr)
sf.write("engine_idle_proc.wav", processed, sr)
```

### 3.5 Unificar sample-rate (`resampy`)

```python
import resampy
y_up = resampy.resample(data, sr_old=22050, sr_new=44100)
```

### 3.6 Loudness consistente en todo el banco (`pyloudnorm`)

```python
import pyloudnorm as pyln

def normalize_to_target(data: np.ndarray, sr: int, target_lufs=-14.0) -> np.ndarray:
    meter = pyln.Meter(sr)
    loud = meter.integrated_loudness(data)
    if not np.isfinite(loud):
        return data
    return pyln.normalize.loudness(data, loud, target_lufs)

processed = normalize_to_target(processed, sr, target_lufs=-14.0)
```

### 3.7 Análisis / validación (`matplotlib` + `librosa`)

```python
import matplotlib.pyplot as plt
import librosa.display

S = librosa.amplitude_to_db(np.abs(librosa.stft(processed)), ref=np.max)
librosa.display.specshow(S, sr=sr, y_axis="log")
plt.title("espectrograma"); plt.colorbar(format="%+2.0f dB")
plt.savefig("engine_idle_proc_spec.png", dpi=120)
plt.close()
```

---

## 4. Integración con `tools/audio/`

- `dsp_common.py` — poner aquí los helpers (`additive_synthesis`, `adsr`,
  `anti_alias`, `normalize_to_target`). Es el módulo compartido.
- `bank_generator.py` — orquesta: genera/modifica cada sample del banco y lo
  escribe con `soundfile`.
- `bank_validator.py` / `tests/test_bank_determinism.py` — el bake debe ser
  **determinista**: fijar semillas y no usar ruido aleatorio sin seed, o los
  tests de determinismo fallarán.

Convención de banco: `game/sounds/banks/v10_vehicle/*.wav` (engine layers,
impactos, shifts, surfaces). Mantener SR y loudness objetivo iguales en todos.

---

## 5. Referencia rápida (CLI)

```powershell
# Instalar en el venv del proyecto
.venv\Scripts\python.exe -m pip install pedalboard numba resampy pyloudnorm librosa

# Verificar versiones instaladas
.venv\Scripts\python.exe -m pip list | Select-String "numpy|scipy|soundfile|pedalboard|numba|resampy|pyloudnorm|librosa|matplotlib"

# Correr la pipeline / validación existente
.venv\Scripts\python.exe -m pytest tools/audio/tests
```

---

## 6. Notas / límites

- Sin `ffmpeg`/`sox`: para MP3 u otros formatos no-WAV usa `soundfile` (FLAC/OGG
  sí soportados) o instala el binario en `.tools/`.
- `librosa` descarga modelos la primera vez vía `pooch` (requiere red). Para
  offline puro, usar `numpy`/`scipy` directamente.
- `numba` tarda en compilar la primera vez (`cache=True` acelera las siguientes).
- Para síntesis en **tiempo real dentro de Godot**, portar el núcleo DSP a
  Rust/C++ GDExtension (ver `vehicle_audio_controller_native`); el mismo algoritmo
  sirve para el CLI headless y para el motor.

---

## 7. Síntesis modular (`tools/audio/synthesis/`)

Paquete headless donde **cada etapa es un módulo propio** y un orquestador las
encadena. Añadir DSP nuevo = crear un stage, sin tocar el orquestador.

**Contratos** (`protocols.py`): `Source.render() -> ndarray` y
`Stage.process(x, sr) -> (x, sr)` (el stage devuelve el sr por si re-muestrea).

**Etapas disponibles:**
| Módulo | Clase | Etapa |
|---|---|---|
| `source.py` | `AdditiveSource` / `NoiseSource` | genera el buffer (suma de partiales / ruido con semilla) |
| `envelope.py` | `Envelope` | ADSR |
| `filters.py` | `AntiAliasFilter` / `HighPassFilter` / `BandPassFilter` / `NotchFilter` | IIR scipy |
| `effects.py` | `EffectsChain` | reverb/comp/pitch/dist (pedalboard) |
| `resample.py` | `Resampler` | resampy |
| `loudness.py` | `LoudnessNormalizer` | EBU R128 (pyloudnorm) |
| `io_audio.py` | `WaveReader` / `WaveWriter` | lee sample existente / escribe WAV |
| `analyze.py` | `Analyzer` | espectrograma/waveform PNG (headless) |

**Orquestador** (`orchestrator.py`): `EngineSynth(source, stages)` — la única
clase que importa a las demás. `.add(stage)` encadena fluido; `.render()` corre
la cadena; `.render_to_wav(path)` escribe.

```python
from tools.audio.synthesis import (EngineSynth, AdditiveSource, Envelope,
    AntiAliasFilter, EffectsChain, LoudnessNormalizer, Resampler, WaveWriter)

synth = EngineSynth(
    AdditiveSource(f0=120.0, duration_s=1.0,
                   harmonic_amplitudes=[1.0, 0.6, 0.4, 0.25, 0.15]),
    [Envelope(0.01, 0.05, 0.7, 0.1),
     AntiAliasFilter(8000.0),
     EffectsChain(reverb=0.3, compressor=True, pitch_semitones=-2.0),
     LoudnessNormalizer(-14.0),
     Resampler(44100)],
)
synth.render_to_wav("game/sounds/banks/v10_vehicle/engine_test.wav")
```

**Modificar un sample existente** (p.ej. bajar tono y normalizar):
```python
from tools.audio.synthesis import EngineSynth, WaveReader, EffectsChain, LoudnessNormalizer, WaveWriter
EngineSynth(WaveReader("game/sounds/banks/v10_vehicle/engine_idle.wav"),
            [EffectsChain(pitch_semitones=-2.0),
             LoudnessNormalizer(-14.0),
             WaveWriter("engine_idle_low.wav")]).render_to_wav("engine_idle_low.wav")
```

Tests: `tools/audio/tests/test_synthesis.py` (determinismo, resample, roundtrip,
efectos + analyzer).

---

## 8. Remasterizar un sonido del banco (ej. `shift_up` / `shift_down`)

Diagnóstico: estos one-shots estaban fuertemente limitados en banda — energía
> 8 kHz = 0.0 (sordos), con un solo click frontal amortiguado y sin evolución
mecánica. Estrategia: **híbrida y no destructiva** — conservar el cuerpo original
y sumarle el detalle que le faltaba.

### 8.1 Etapas nuevas (una por archivo)
- `tools/audio/synthesis/shift_transient.py` → `ShiftTransient`: clack metálico de
  engrane (partiales inharmónicos ~2.6 kHz, decay 10 ms) en t=0; para `down` añade
  un blip ascendente de gas a ~40 ms.
- `tools/audio/synthesis/metal_sheen.py` → `MetalSheen`: ruido rosa band-pass 4–9 kHz
  con envolvente rápida = el "aire" metálico que faltaba.
- `tools/audio/synthesis/remaster_shift.py` → `remaster_shift(src, out, kind)`:
  orquesta `WaveReader → HighPass(140) → Gain(0.7) → MetalSheen → ShiftTransient →
  EffectsChain(comp+dist+reverb) → LoudnessNormalizer(-16) → Finalize(pico 0.92) →
  WaveWriter`. Todo determinista (seeds).

### 8.2 Uso
```python
from tools.audio.synthesis import remaster_shift
remaster_shift("game/sounds/banks/v10_vehicle/shift_up.wav",
               "game/sounds/banks/v10_vehicle/shift_up.wav",  # o un _remaster.wav
               kind="up")
```

### 8.3 ⚠️ El `bank_manifest.json` ES OBLIGATORIO ACTUALIZARLO
El audio del vehículo lo carga **Rust** (`game/crates/vehicle-audio-engine/src/bank.rs`),
no Godot `ResourceLoader`. `VehicleSoundBank::load` itera `bank_manifest.json`, lee
cada WAV por el stem del campo `file` y **compara el sha256 del archivo contra
`files[].sha256`**; además valida `mono / 16-bit / 44.1 kHz`.

Consecuencia: si reemplazas un WAV del banco **sin actualizar su `sha256` en el
manifest, el loader falla con `BankError::ShaMismatch` y se rompe TODO el audio del
vehículo**. No hace falta tocar código Rust (carga por nombre de archivo), pero **sí**
hay que reescribir la entrada del manifest para ese `file`:

- `sha256` → sha256 de los bytes del nuevo WAV
  (`hashlib.sha256(open(path,'rb').read()).hexdigest()`).
- `duration_s` → `len(pcm) / 44100`.
- `peak`, `loudness_dbfs`, `dc_offset` → medidos del nuevo WAV (el contrato exige
  `peak < 0.99` y `|dc| < 0.005`).
- **Conservar** `synthesis.source_file` y `synthesis.source_sha256` (los tests
  `test_manifest_contains_required_fields` / `test_bank_matches_declared_source_spec`
  los exigen) y la substring `"derived from original"` en `provenance`.
- Se puede añadir `synthesis.remaster = {...}` con la receta; no rompe ningún test.

El helper `tools/audio/bank_validator.py` (`validate_bank`) marca hash/formato
incorrectos como **error**; corrélo tras el cambio para confirmar.

### 8.4 `.import` (Godot) — no afecta al runtime
El loader Rust lee los WAV crudos; los `.import` solo los usa el editor Godot. Al
reemplazar `shift_up.wav`/`shift_down.wav` se borraron los `.import` obsoletos
(`shift_up.wav.import`, `shift_down.wav.import`) para que el editor los regenere.
Los originales renombrados a `shift_up_backup.wav` / `shift_down_backup.wav` quedan
como WAV no listados en el manifest: `validate_bank` los reporta como *warning*
(`file.untracked`), no como error, y `test_loop_seam_continuity` los ignora.

### 8.5 Verificación
```powershell
# 1) Suite Python del banco (formato + sha256 vs manifest + validador)
.venv\Scripts\python.exe -m pytest tools/audio/tests -q
# 2) Loader Rust real contra el banco en disco (carga + sha256 + formato)
cargo test -p vehicle_audio_engine --test bank_integration
```
Resultado obtenido: **15 passed** (Python) y **6 passed** (Rust, incluido
`bank_loads_and_has_expected_entries`). La energía > 8 kHz pasó de 0.0 % a
**~0.98 %** (`shift_up`) / **~0.93 %** (`shift_down`).

---

## 9. Remasterizar los 8 impactos del banco

Mismo protocolo que los shifts: cuerpo original + capas de detalle, backups `_backup`,
`bank_manifest.json` actualizado (sha256 + metadatos), y el caveat de la sección 8.3
**se aplica igualmente** (sin actualización de sha256 → `ShaMismatch` → audio roto).

### 9.1 Nuevos stages (`tools/audio/synthesis/impact_stages.py`)
- `ImpactBody(f0, decay_s, gain, sub)` — thud low-mid con pitch-drop y sub opcional.
- `BoomSource(f0, f1, glide_s, decay_s, gain)` — sub con glissando descendente (explosiones).
- `CrackleLayer(t0, t1, rate, band, gain)` — pops tipo Poisson band-pass (whoosh / chispa).
- `DebrisLayer(band, depth, gain)` — ruido band-pass con tremolo lento (grind / rozadura).

Todos deterministas (seed). Reutilizan `Stage.process(x, sr) -> (x, sr)`.

### 9.2 Perfiles (`tools/audio/synthesis/remaster_impact.py`)

| Perfil | Archivos | Cadena resumida |
|---|---|---|
| `barrier` | `impact_barrier` | HPF(120) + ImpactBody(160Hz, sub) + ShiftTransient(base 2k, decay 60ms) + DebrisLayer(3-8k) + comp+dist+reverb |
| `cone` | `impact_cone` | HPF(150) + ImpactBody(320Hz) + ShiftTransient(base 2.6k, 12ms) + comp+reverb |
| `fire` | `impact_fire` | HPF(60) + BoomSource(120→40Hz) + ShiftTransient(1.8k, 20ms) + CrackleLayer(1-9k, 1.3s) + comp+dist+reverb |
| `hit` | `hit_1..4` | HPF(140) + ImpactBody(f0 varía) + ShiftTransient(base varía) + MetalSheen(0.10) + comp+reverb |
| `scrape` | `impact_scrape` | HPF(80) + ImpactBody(300Hz) + MetalSheen(0.06) + comp+reverb |

Los hits varían `body_f0` (130/150/170/190) y `clang_base` (1800/2600/2000/2300) para
una familia coherente con pesos y brillos distintos pero reconocibles.

### 9.3 Uso
```python
from tools.audio.synthesis import remaster_impact
remaster_impact("game/sounds/banks/v10_vehicle/impact_fire.wav",
                "game/sounds/banks/v10_vehicle/impact_fire.wav",
                profile="fire")
```

### 9.4 Resultados objetivos (backup → remaster)

| Archivo | low<200 | hf>8k | centroid | Cambio clave |
|---|---|---|---|---|
| `impact_fire` | 98.5% → 97.5% | 0.01% → **0.08%** | 1096 → **2308** | boom + crackle + clack: de sub sordo a explosión completa |
| `impact_hit_1` | 0.6% → **17.8%** | 0.02% → 0.15% | 1884 → 2335 | thud + clang: de clack fino a golpe con peso |
| `impact_hit_2` | 5.9% → **20.4%** | 0.3% → 0.43% | 2532 → 2917 | más peso + brillo mantenido |
| `impact_hit_3` | 19.8% → 23.2% | 0.11% → 0.3% | 1866 → **2705** | destapado (clang + MetalSheen) |
| `impact_hit_4` | 9.4% → **17.4%** | 0.11% → 0.29% | 2014 → 2690 | peso + brillo, familia cohesiva |
| `impact_barrier` | 6.9% → 6.9% | 0.41% → 0.49% | 2506 → 2739 | clang t=0 + debris grind |
| `impact_cone` | 12.6% → 3.0% | 0.06% → 0.02% | 1460 → 1036 | thock más profundo (menos hueco) |
| `impact_scrape` | 0.02% → 0.63% | 0.89% → 0.18% | 3734 → 3332 | anclado a pista (rumble low-mid) |

### 9.5 Verificación
```powershell
.venv\Scripts\python.exe -m pytest tools/audio/tests -q   # 15 passed
cargo test -p vehicle_audio_engine --test bank_integration  # 6 passed
```
