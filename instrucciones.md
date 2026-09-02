# Backlog autoritativo — V10 Rust GF509 en Godot

Fecha: 2026-09-01  
Workspace: `D:\Formula90s`  
Rama observada: `fix/v10-audio-physical-boundary`  
HEAD observado: `4c6854081c21dc8cdc6ea1f761935120848090ea`  
Script canónico: `run_f1_94.ps1`

## Objetivo

Reproducir GF509 en tiempo real dentro de Godot. `v10-engine-synth` reemplazará sólo la fuente continua del motor. `vehicle-audio-engine` conservará el mezclador, one-shots, backfires, impactos, scraping, superficies/neumáticos y protección final.

```text
Godot: rpm · throttle · load · gear · dt
                    │
                    ▼
 v10-engine-synth: V10Engine + AcousticScene
                    + ThreeZoneSampleLayer
                    │ motor continuo estéreo
                    ▼
 vehicle-audio-engine
   ├── one-shots / backfires
   ├── impactos / scraping
   ├── superficies / neumáticos
   └── mezcla y protección final
                    │ PCM
                    ▼
 VehicleAudioControllerNative → bus Vehicle → Godot
```

## Reglas y seguridad

- Integrar Rust con Rust; no añadir C++ ni Faust.
- No rediseñar el timbre GF509 durante la integración.
- No reimplementar ni duplicar one-shots en `v10-engine-synth`.
- No depender en runtime de `reports/`, `D:\ASETS` ni rutas absolutas.
- Mantener el motor continuo anterior como fallback explícito hasta el human gate.
- No versionar renders, stems, DLL, `target` o caches salvo autorización.
- Aplicar fail-fast; no crear smoke tests redundantes.

Antes de modificar:

```powershell
git branch --show-current
git rev-parse HEAD
git status --short
```

El worktree ya contiene numerosos cambios y artefactos ajenos. No limpiar, resetear, cambiar de rama ni sobrescribirlos. Stagear rutas explícitas; `git add -A` está prohibido.

No ejecutar Godot como evidencia hasta cumplir:

```text
game/BUILD_SOURCE == git rev-parse HEAD
```

Al redactar este backlog, `BUILD_SOURCE` contiene `7bcde9ab5ed0ef787107fab23e8265ba8102ea39` y no coincide con HEAD. Una reconstrucción completa debe limpiar únicamente outputs/caches autorizados de Cargo, SCons y `game/.godot`, usar `run_f1_94.ps1` y confirmar nuevamente la paridad.

Pipeline:

```text
sprint planning → backlog item → implement → review → human gate → sprint retrospective → done
```

## Backlog ordenado

### INT-00 — Congelar baseline y contrato — P0

- Identificar commit y configuración exactos de GF509.
- Registrar sample rate, seed, parámetros, ganancias y hashes SHA-256 de low/med/max preparados.
- Conservar como referencia 7499 RPM y el sweep 5000–14500 RPM con lift-and-coast.
- Definir entrada: `rpm`, `throttle`, `load`, `gear`, `dt`; fase de cigüeñal sólo si existe una fuente autoritativa.
- Definir salida: estéreo `f32`, sample rate, frames máximos por bloque y semántica de reset.
- Resolver procedencia/licencia de los samples antes de empaquetarlos.

Gate: tests actuales del V10 pasan, baseline reproducible y contrato documentado. Detenerse si los samples no pueden distribuirse.

### INT-01 — API integrable de `v10-engine-synth` — P0

- Resolver su workspace aislado de forma compatible con el workspace/build canónico.
- Añadir dependencia de path explícita desde `vehicle-audio-engine`.
- Exponer una API mínima: construir, actualizar telemetría, renderizar bloque y resetear.
- Separar CLI, WAV, stems y análisis de la biblioteca runtime.
- Evitar estado global, I/O y allocations durante render; conservar seed determinista.

Gate: `cargo check` del workspace canónico y tests de ambos crates pasan. Una prueba contractual cubre tamaños de bloque variables, muestras finitas y continuidad de frontera.

### INT-02 — Empaquetar `ThreeZoneSampleLayer` — P0

- Ubicar low/med/max dentro de `game/` y accesibles por el runtime instalado.
- Crear manifest versionado: clave, RPM nativa, sample rate, canales, loops, gain y hash.
- Resolver paths desde el proyecto/configuración, nunca mediante rutas absolutas.
- Cargar, validar y preparar buffers fuera del callback.
- Fallar con mensaje accionable si falta un sample o formato/hash/loop es inválido.

Gate: los tres samples cargan desde una copia limpia; retirar uno falla inmediatamente; no quedan referencias runtime a `reports/` o `D:\ASETS`.

### INT-03 — Fuente continua dentro de `vehicle-audio-engine` — P0

- Crear una abstracción interna como `EngineContinuousSource`.
- Adaptar la fuente actual como `Legacy`.
- Implementar `V10Gf509Source`, propietario de `V10Engine`, `AcousticScene` y `ThreeZoneSampleLayer`.
- Añadir selección `legacy | v10_gf509` y fallback sólo ante error de inicialización, nunca silenciosamente durante render.
- Insertar el V10 exactamente donde entra el motor continuo, antes de one-shots y protección final.
- Desactivar la fuente tonal legacy cuando se seleccione V10.
- Mantener triggers y rutas actuales de one-shots/beds sin duplicarlos.

Gate: con V10 silenciado siguen sonando one-shots pero legacy entrega silencio exacto; un shift/backfire dispara una voz; modo legacy conserva el comportamiento previo.

### INT-04 — Telemetría y tiempo real Godot — P0

- Trazar `VehicleAudioControllerNative → formula90-core → VehicleAudioEngine` y reutilizar el estado coherente existente.
- Separar frecuencia de física de frecuencia de audio e interpolar parámetros dentro del bloque.
- Si no existe fase autoritativa, mantener fase sample-accurate interna; no reiniciarla por frame.
- Definir pausa, respawn, cambio de coche, neutral, stall y pérdida del nodo físico.
- Alimentar el `AudioStreamGenerator` y bus `Vehicle` existentes.

Gate: RPM fija sin modulación al ritmo de física; sweep sin clicks/reinicios; lift-and-coast conserva inercia tonal; reset no deja estado o voces colgadas.

### INT-05 — Configuración y observabilidad — P1

- Configuración versionada para fuente, manifest, gain y fallback.
- Mantener parámetros GF509 centralizados; exponer sólo controles de integración.
- Medir fuente activa, RPM recibida/renderizada, peak pre-limitador, reducción, tiempo de render, underruns y errores de assets.
- Añadir diagnóstico `V10 solo | one-shots/beds solos | mezcla`.
- No loguear desde el callback; publicar snapshots/contadores fuera de él.

Gate: Godot confirma que la fuente activa es `v10_gf509` y permite aislar rutas sin recompilar.

### INT-06 — Presupuesto de tiempo real — P1

- Medir Release con tamaños de bloque reales: mediana, p95 y peor caso, V10 solo y mezcla completa.
- Verificar cero allocations, locks bloqueantes, disco y logs dentro de `render`.
- Preasignar/reutilizar buffers.
- Perfilar antes de optimizar. No volver a duplicar un banco de cinco cilindros ni eliminar identidad de cilindro sin human gate.

Gate: margen suficiente bajo el deadline de audio, cero underruns y sin degradación audible frente al baseline GF509.

### INT-07 — Rebuild seguro y validación en juego — P0

- Revisar el diff y ejecutar tests dirigidos.
- Verificar HEAD justo antes del build.
- Limpiar sólo outputs/caches autorizados y ejecutar `run_f1_94.ps1`.
- Confirmar `BUILD_SOURCE == HEAD` y procedencia de las DLL cargadas.
- Probar idle/5000, 7499, zona media, 14500, lift-and-coast, shifts, backfires, impactos y superficies.

Human gate: comparar GF509 en juego contra el baseline offline y confirmar que one-shots/beds siguen correctos. El refinamiento tonal posterior será otro backlog.

### INT-08 — Documentación y commits atómicos — P1

- Actualizar `docs/` con ownership, flujo, assets, configuración y fallback.
- Documentar preparación/licencia/hash de samples y lectura de métricas.
- Registrar tests, benchmark y human gate sin añadir WAV generados.
- Revisar `git diff` y `git diff --cached`; stagear sólo rutas explícitas.
- Separar commits de contrato, assets/config, integración y documentación.
- Completar review y retrospectiva.

## Definición de terminado

- GF509 responde en tiempo real a RPM, throttle y load.
- Low/med/max participan desde assets empaquetados.
- One-shots y beds funcionan una sola vez y no fueron reimplementados.
- No hay clicks de bloque, underruns, clipping ni allocations en callback.
- El fallback recupera legacy sin afectar eventos.
- BUILD/HEAD coinciden y el usuario aprueba el A/B dentro del juego.

## Tests mínimos con valor

1. Bloques variables sin NaN, discontinuidad ni allocation conocida.
2. Seleccionar V10 silencia exactamente la fuente continua legacy.
3. Shift/backfire dispara una sola vez y sigue audible con V10 silenciado.
4. Manifest valida los tres assets; ausencia/corrupción falla rápido.
5. Reset elimina estado anterior y conserva determinismo.
6. El script canónico rechaza BUILD/HEAD fuera de paridad.

## Fuera de alcance

- Rediseñar GF509 o añadir capas, micrófonos, C++, Faust o nuevos samples.
- Reescribir one-shots.
- Ocultar errores mediante EQ/limitación master.
- Hacer push o distribuir samples sin autorización expresa.

## Primer bloque recomendado

Ejecutar sólo `INT-00` e `INT-01`, hacer review y detenerse ante fallo de compilación, ambigüedad de procedencia/licencia o conflicto con cambios dirty. Después avanzar a `INT-02` e `INT-03`.
