Actualmente el banco de sonidos de v10 se consume desde:
D:\Formula90s\game\audio\formula_one_2030_grand_prix_sampler

El problema es que no es un folder estandarizado , el folder que contiene todos los sonidos del v10 + 2 nuevos de off y on mid es :
D:\Formula90s\game\sounds\banks\v10-v2-bank

Este banco es el que debe ser el canon.  
Primero verificar que todos los audios esten alineados al formato requerido por el sampler.
Luego planificar la introduccion de 2 samples intermedios (mid) tanto en on como en off.

genera un AGENTS.md en /games/ , este debe indicar explicitamente que todos los bancos de sonido, tanto el de motores , como el commons , deben estar en  
D:\Formula90s\game\sounds\banks\v10-v2-bank

tambien indicar en el AGENTS.md cual es el formato de sampleo.

# Plan de sprint: controles Xbox, ajustes de entrada y vibración por superficie

Estado: especificación actualizada con las aclaraciones del usuario; implementación integrada en HEAD 918ad875; rebuild canónico pendiente.

## Procedencia e inventario previo

- Rama inspeccionada: `main-clean`.
- HEAD inspeccionado: `b55d56eb592d12176fc60856cbafab7e469284eb`.
- Checkout preexistente sucio: `game/BUILD_SOURCE`, diez DLL de `game/addons/formula90s/bin/` y `game/assets/models/pit_crew/racer/source/` sin seguimiento. No tocar, respaldar, limpiar, cambiar de rama ni incluir estos cambios en el trabajo de esta feature sin una decisión explícita.
- Entrada canónica de ejecución: `scripts/run_f1_94.ps1`. Antes de una futura ejecución se deberá respetar la comprobación BUILD/HEAD exigida por el protocolo.

## Contrato funcional solicitado

| Mando Xbox                     | Acción                                 |
| ------------------------------ | -------------------------------------- |
| A                              | Subir marcha                           |
| X                              | Bajar marcha                           |
| Y                              | Abrir/cerrar panel de boxes            |
| B                              | Confirmar selección del panel de boxes |
| RT                             | Acelerador                             |
| LT                             | Freno                                  |
| Analógico izquierdo horizontal | Dirección                              |

La dirección, el freno y el acelerador deben tener curvas de respuesta configurables. El contravolante solicitado es la velocidad de inversión/retorno de la dirección; debe configurarse de forma común para teclado y mando y aplicarse a todos los perfiles, sin valores específicos por vehículo. Toda la configuración de entrada, incluidas las asignaciones y los ajustes de teclado preexistentes, debe residir en un JSON. El mando debe vibrar al pasar por pianos, circular sobre tierra, hierba, grava o arena, y acercarse al límite de agarre mecánico delantero o trasero. La vibración fuera del asfalto será leve y diferenciable de las demás respuestas.

## Hallazgos de arquitectura (baseline previo a la implementación)

- `game/scripts/input/input_bindings.gd` es el autoload que registra las acciones en `InputMap`; `game/project.godot` lo carga antes que la escena. Hoy contiene las teclas y el mando codificados en GDScript.
- `game/scripts/input/vehicle_rust_input_controller.gd` consume las acciones y solo aplica un exponente local al acelerador. Entrega valores normalizados a la física. La lógica actual invierte acelerador/freno cuando la marcha es reversa; conservar ese contrato al cambiar el origen de entrada.
- `game/scripts/runtime/pit_stop_controller.gd` consume las acciones de abrir, navegar y confirmar el panel; la cruceta ya navega el panel. El panel tiene una restricción de sesión/pista/vehículo que no debe alterarse por el nuevo mapeo.
- Conflictos actuales del mando: A es freno de mano y confirmar boxes, B conmuta transmisión, X sube marcha y Y baja marcha. Retirar las asignaciones incompatibles del mando; mantener freno de mano y cambio automático/manual en sus asignaciones actuales de teclado.
- `get_wheel_surface_types()` entrega un código por rueda: 0 asfalto, 1 piano, 2 tierra, 3 hierba, 4 grava, 5 arena, 6 muro, 7 metal. El controlador visual ya lee esa API para el efecto de piano; la vibración debe leer el dato de superficie sin depender del efecto visual.
- La simulación actual tiene parámetros `countersteer_speed` y `countersteer_assist` por vehículo y una ayuda física activable por máscara. El requerimiento confirmado es la velocidad de inversión/retorno de la dirección, común a todos los perfiles y configurada en el JSON global de input; no es la ayuda de corrección de derrape `countersteer_assist`. La implementación debe separar esa velocidad de los valores por vehículo y no presentar la ayuda física como el contravolante solicitado.

## Backlog propuesto y orden de implementación

1. **Contrato JSON de entrada.** Definir un único archivo versionado para teclado y mando, con acciones, eventos físicos, zonas muertas, curvas de dirección/acelerador/freno, velocidad común de inversión/retorno de dirección, y ajustes de vibración. Incluir ahí la velocidad de contravolante para que rija todos los perfiles y eliminar su dependencia de valores específicos por vehículo. Precisar unidades, rangos válidos, valores predeterminados y tratamiento de JSON inválido. Mantener las constantes de nombre de acción solo como interfaz hacia consumidores existentes; registrar eventos desde el JSON en el autoload.
2. **Mapa Xbox sin acciones duplicadas.** Aplicar A/X/Y/B/RT/LT/analógico izquierdo según la tabla. Preservar la navegación del panel con la cruceta y el teclado existente. Mantener freno de mano y cambio automático/manual en teclado; no reasignarlos a otros botones del mando. Verificar que B solo confirme cuando el panel esté activo y que Y no confirme ni cambie marcha.
3. **Tratamiento de ejes y retorno de dirección.** Separar lectura, selección de dispositivo activo y transformación de valores de la entrega al vehículo. Aplicar zonas muertas y curvas con salida acotada: dirección en [-1, 1], pedales en [0, 1]. Configurar una velocidad común de inversión/retorno de dirección que se aplique a teclado y mando en todos los perfiles. Evitar que una fuente inactiva mantenga un valor residual o que teclado y mando se sumen de forma inesperada. La ayuda física de corrección de derrape queda fuera del significado del contravolante solicitado.
4. **Vibración por superficie y límite de agarre.** Crear un controlador de vibración vinculado al vehículo activo y al mando que lo controla. Muestrear superficies por rueda: piano (código 1) y fuera de asfalto (tierra 2, hierba 3, grava 4, arena 5). Dar a fuera de asfalto una respuesta leve distinguible de la vibración de piano. Incorporar una señal de proximidad al límite de agarre mecánico y distinguir pérdida de agarre del eje delantero y posterior; definir cómo obtenerla de telemetría/estado físico existente o qué dato habrá que exponer. Parametrizar intensidades, frecuencias, umbrales y transiciones en el JSON. Evitar reinicios de vibración en cada frame y detenerla al cesar el evento, desconectar el mando o liberar la escena. Sin requisito de vibración para muro, metal o pérdida de contacto con el suelo.
5. **Revisión y validación.** Añadir pruebas del cargador JSON, mapeo y ausencia de conflictos A/X/Y/B; curvas, límites y cambio de dispositivo; apertura/confirmación de boxes; y vibración simulada por secuencias de superficies. Hacer una pasada manual con el mando Xbox real y `scripts/run_f1_94.ps1` para verificar gatillos, sentido de dirección, cambios, panel y respuesta háptica. Confirmar BUILD/HEAD antes de abrir Godot. Revisar el diff y, si se autoriza un commit, preparar solo archivos propios de esta feature, inspeccionar `git diff --cached` y conservar los cambios preexistentes fuera del commit.

## Decisiones confirmadas

- El contravolante significa la velocidad de inversión/retorno de la dirección. Su ajuste es común a teclado y mando, se aplica a todos los perfiles y no debe configurarse por vehículo. No equivale a la ayuda física de corrección de derrape.
- Las funciones actuales del mando desplazadas por el nuevo mapa (freno de mano y cambio automático/manual) permanecen disponibles mediante sus controles de teclado; no necesitan nuevas asignaciones en el mando.
- Fuera del asfalto comprende tierra, hierba, grava y arena. La vibración será leve y distinta al pasar por esas superficies. También habrá una señal diferenciada al acercarse al límite de agarre mecánico delantero o posterior.
- El JSON global de entrada contendrá las curvas y ajustes del contravolante solicitado. No se añadirá configuración de contravolante por vehículo.

## Criterios de aceptación

- Las siete asignaciones Xbox solicitadas funcionan simultáneamente sin disparar las antiguas acciones de esos botones.
- El teclado conserva sus controles actuales y tanto sus asignaciones como sus ajustes de entrada se leen del JSON.
- Los tres ejes respetan sus curvas y zonas muertas configuradas; la velocidad común de inversión/retorno de la dirección funciona en teclado y mando y en todos los perfiles, sin configuración por vehículo.
- El panel se abre con Y, se navega con la cruceta y se confirma con B solo en un estado válido.
- La vibración se percibe en piano, de forma leve y diferenciable en tierra/hierba/grava/arena, y con señal distinta al aproximarse al límite mecánico delantero o posterior. Se detiene limpiamente al cesar el evento o desconectar el mando.
- El commit de la feature no incluye los binarios, `BUILD_SOURCE` ni los assets de pit crew que ya estaban sucios al iniciar esta planificación. El rebuild autorizado actualizará los artefactos generados para el HEAD actual; los assets de pit crew quedan fuera del rebuild.
