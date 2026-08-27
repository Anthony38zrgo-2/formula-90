# AUDIO-001 — mapa de migración del dominio de audio

> Estado: **AUDIO-010 APROBADO; MIGRACIÓN DE AUDIO COMPLETADA**
> Alcance: mapa histórico y estado final de la migración del dominio. El flujo
> vigente separa fuentes, tooling, preview y runtime.

## 1. Resultado buscado

Separar el dominio de audio en tres responsabilidades simples:

```text
source-assets/audio/  ->  tools/audio/  ->  scratch/audio/
        fuentes              build           preview
                                                   |
                                                   | human gate + promote
                                                   v
                                      game/sounds/banks/
                                              runtime
```

- `source-assets/audio/` contiene muestras originales o reemplazos editables.
- `tools/audio/` contiene recetas, validadores y generadores reproducibles.
- `scratch/audio/` contiene cualquier banco candidato.
- `game/sounds/` contiene solamente audio aprobado que el juego carga.

La migración no intenta diseñar ahora una plataforma genérica de audio. El
primer corte termina cuando `v10_vehicle` puede construirse fuera del runtime,
escucharse y promoverse explícitamente sin romper F1-94.

## 2. Inventario observado

### Fuentes legacy

`assets-lowpoly-python/sounds/` contiene 39 archivos (38 WAV y 1 MP3;
4.112.859 bytes):

| Grupo | Ejemplos | Clasificación |
|---|---|---|
| Motor y vehículo | `98_int_idle.wav`, `98_int_max_5.wav`, `ALT_int_backfire.wav` | Fuente editable |
| Ambiente y evento | `Ambient/ambient.mp3`, público, semáforo, screenshot | Fuente editable |
| Impactos y superficies | `GP_hit*.WAV`, grass, sand, rumble, scrape | Fuente editable |

Son inputs de authoring, no recursos cargados directamente por Godot. Por eso
pertenecen en `source-assets/audio/`, no junto al script Python que les dio
origen histórico.

### Tooling

`tools/audio/` ya agrupa generación, validación, remaster, escenarios,
synthesis y sus pruebas. Se conserva como frontera de tooling. Sus rutas por
defecto se corregirán en un item posterior; AUDIO-001 no modifica conducta.

### Runtime

`game/sounds/` contiene únicamente responsabilidades de ejecución:

- `sound_mixer_config.json`: política de mezcla cargada por runtime;
- `banks/v10_vehicle/`: banco canónico de 27 voces consumido por Rust.

Las fuentes legacy viven en `source-assets/audio/legacy-f1-1998/` y las once
fuentes de reemplazo en `source-assets/audio/replacements/f1_94/`. Los previews,
candidatos y backups transaccionales son locales a `scratch/`; no forman parte
del runtime.

## 3. Clasificación y destino

| Origen actual | Acción | Destino propuesto | Motivo |
|---|---|---|---|
| `assets-lowpoly-python/sounds/` | MOVED | `source-assets/audio/legacy-f1-1998/` | Fuentes originales, fuera del runtime |
| `tools/audio/` | KEEP + REFACTOR | `tools/audio/` | Ya representa una sola responsabilidad |
| `tools/audio/bank_config.yaml` | KEEP + REVIEW | `tools/audio/` | Receta de build; confirmar que sea ejecutable y no solo documental |
| `game/sounds/sound_mixer_config.json` | KEEP | `game/sounds/` | Política de mezcla usada en ejecución |
| `game/sounds/banks/v10_vehicle/` | KEEP | mismo destino | Banco runtime aprobado |
| `game/sounds/banks/new sounds/` | MOVED | `source-assets/audio/replacements/f1_94/` | 11 fuentes originales preservadas por hash |
| `*_backup.wav` dentro del banco | RETIRED | `scratch/promote-backups/` durante cutover | El runtime contiene solo las 27 voces declaradas |
| `*.import` | REGENERATE, no mover | junto al recurso runtime que Godot importe | Artefactos del editor, nunca fuentes |
| `game/sounds/AUDIO_TOOLING.md` | MOVED | `docs/audio-tooling.md` | Documentación fuera del runtime y marcada con el flujo vigente |
| `game/scripts/audio/vehicle_audio_controller.gd` | RETIRED | — | La mezcla y playback canónicos pertenecen a Rust/F90Core |
| `game/crates/vehicle-audio-engine/` | KEEP + THIN | mismo destino | Motor runtime y validación del banco |

`legacy-f1-1998` es un nombre de procedencia, no una API definitiva. Conserva
el árbol y los nombres originales durante el primer movimiento para que los
hashes permitan demostrar que no se alteró contenido.

## 4. Lectores, escritores y acoplamientos

### Productores

- `tools/audio/bank_generator.py` lee fuentes legacy y escribe por defecto en
  `scratch/audio/v10_vehicle`.
- `tools/audio/promote_f1_94_replacement_sounds.py` exige fuente y banco
  explícitos; no conserva un destino runtime por defecto.
- `tools/audio/remaster_lib.py` usa `scratch/audio/v10_vehicle` como banco de
  trabajo predeterminado.

La única escritura del banco runtime ocurre mediante `promote_content.ps1`
después del human gate.

### Consumidores

- `F90Core` y el adaptador nativo cargan
  `res://sounds/banks/v10_vehicle` y `sound_mixer_config.json`.
- `game/crates/vehicle-audio-engine` abre el manifiesto, valida formato/hashes
  y reproduce el banco.
- pruebas de Rust y Python consumen el banco canónico y, en varios casos, las
  fuentes legacy directamente.
- `run_f1_94.ps1 -SmokeAudio` constituye el smoke runtime de cierre.

### Datos duplicados

Los nombres de bandas, RPM nativas, centros y ancho se declaran en el manifiesto;
Rust los valida y consume sin tablas runtime duplicadas. El manifiesto incluye
`role` y `playback`, y `synthesis.source_file` apunta exclusivamente a
`source-assets/audio/`.

El manifiesto del banco es la fuente efectiva de
metadatos por muestra. `sound_mixer_config.json` seguirá siendo la fuente de
ajuste de mezcla en runtime. No se duplicarán ambos propósitos en un schema
nuevo.

Los crossfades de 2048 frames (procesado offline de seam) y 512 frames
(playback/runtime) son responsabilidades distintas. No se unifican hasta que
una prueba auditiva demuestre que representan el mismo concepto.

## 5. Contrato mínimo del slice

### Preview

```text
fuentes + receta -> scratch/audio/v10_vehicle/ -> validación mínima -> escucha
```

La validación automática previa al human gate se limita a fallos baratos:

1. el comando terminó;
2. existe el manifiesto;
3. los WAV declarados existen y pueden abrirse;
4. los hashes/formatos declarados son coherentes.

Después se escucha o ejecuta el candidato. No se exige la suite FULL antes de
decidir si el resultado vale la pena.

### Promote

Solo un candidato aceptado se copia a `game/sounds/banks/v10_vehicle/` mediante
el mecanismo común de promoción. La promoción conserva backup recuperable y
registra origen/destino; no vuelve a sintetizar ni remasterizar.

### Cierre

Tras el human gate se ejecutan las pruebas de integración proporcionales al
cambio y el smoke de F1-94. FULL se reserva para cerrar el slice, no para cada
iteración auditiva.

## 6. Decisiones de seguridad

1. AUDIO-002 verificó hashes antes y después del movimiento de fuentes legacy.
2. Se preservarán nombres, mayúsculas y estructura relativa en el primer move.
3. No se moverán `.import` desde runtime a `source-assets`.
4. Las fuentes de `new sounds` se revisaron y movieron a `source-assets/audio/`
   preservando 11/11 hashes.
5. Los backups runtime se retiraron después del cutover; el rollback
   transaccional permanece en scratch/ durante el gate final.
6. Git LFS, compresión y normalización masiva quedan fuera de esta migración.
7. No se mezclará síntesis de audio nueva con el cambio de arquitectura.
8. Cada cutover mantendrá paridad BUILD/HEAD exigida por el repositorio.

## 7. Backlog ejecutable del dominio

| Orden | Item | Entrega | Human gate |
|---:|---|---|---|
| 1 | **AUDIO-002 — IMPLEMENTADO** | Fuentes legacy movidas a `source-assets/audio/legacy-f1-1998/`; 39/39 archivos y hashes idénticos | Confirmar que el inventario audible sigue intacto |
| 2 | **AUDIO-003 — IMPLEMENTADO** | Defaults de build/remaster/render en `scratch/audio/`; writers runtime legacy requieren destino explícito | Probar un build candidato |
| 3 | **AUDIO-004 — IMPLEMENTADO** | `preview_audio.ps1` construye y renderiza current/candidate bajo `scratch/audio/ab/` | Escuchar ambos WAV y decidir keep/reject/iterate |
| 4 | **AUDIO-005 — IMPLEMENTADO** | Schema v1 mínimo y validador stdlib para estructura, paths, hashes y loops | Aprobar el núcleo obligatorio sin extensiones anticipadas |
| 5 | **AUDIO-006 — IMPLEMENTADO** | `playback.native_rpm` y `engine_band` explícitos por muestra; mixer policy permanece fuera | Revisar manifiestos runtime y candidato |
| 6 | **AUDIO-007 — IMPLEMENTADO** | Loader, mixer y replay Rust usan el perfil del manifiesto; tablas runtime duplicadas retiradas | Smoke Rust + audio |
| 7 | **AUDIO-008 — IMPLEMENTADO** | Implementación GDScript paralela retirada; escenas sin referencias muertas, audio en Rust vía `F90Core`/adaptador nativo | Smoke Godot F1-94 |
| 8 | **AUDIO-009 — IMPLEMENTADO** | Banco completo de 27 voces promovido mediante `promote_content.ps1`; banco anterior preservado en backup transaccional | Smoke Godot + validar rollback disponible |
| 9 | **AUDIO-010 — IMPLEMENTADO** | 11 fuentes movidas por hash a `source-assets`; backups/documentación retirados del runtime; mapa actualizado | Gate final + FAST/FULL proporcional |

No se comienza por crear `formats/audio_bank/`: primero se mueve la fuente y se
separa preview de runtime. El formato se extrae después desde consumidores
reales, evitando burocracia y abstracciones anticipadas.

## 8. Criterio histórico de cierre de AUDIO-001

- Se identificaron fuentes, tooling, runtime, temporales y documentación.
- Cada subárbol tiene acción y destino propuesto.
- Se enumeraron productores, consumidores y duplicaciones principales.
- Se fijó una secuencia que permite fallar barato antes del human gate.
- No se movió, regeneró ni eliminó ningún asset.
