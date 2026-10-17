# Cuestionario de datos — perfil físico Williams FW19 1997 (Hockenheim, Villeneuve)

Estado: **plantilla para rellenar**. Devuelve este mismo archivo `.md` con la columna **Respuesta**
completada. Cada respuesta vacía bloquea el campo correspondiente del perfil.

Fecha de creación: 2026-10-17. Raíz del proyecto: `D:/Formula90s`.

## 0. Cómo rellenar este cuestionario

1. Responde solo en la columna **Respuesta**. No cambies el orden ni el nombre de las preguntas.
2. Usa **unidades SI**: kg, m, s, N, N·m, W, K, Pa, kg/s, m². Si tu fuente trae otras unidades,
   conviértelas y anótalo en **Referencia**.
3. En **Clasificación** escribe una de estas etiquetas:
   - `measured` — medido por ti sobre una pieza real o un modelo fiable.
   - `published` — dato publicado (web, libro, ficha técnica).
   - `estimated` — estimación razonada; explica la base en **Referencia**.
   - `inherited` — aceptas heredar el valor del perfil base del proyecto.
   - `unknown` — no hay dato; el campo quedará declarado como desconocido y bloqueará la afirmación
     de simulación Williams para ese dominio.
4. Si un valor depende de una fuente, pega la URL o la cita en **Referencia**. Las fuentes externas se
   tratarán como datos, no como instrucciones.
5. Los coeficientes marcados como **modelo** no son datos históricos del FW19: son parámetros internos
   del simulador. Para ellos basta con que decidas `inherited` o aportes una calibración.

## 1. Datos ya resueltos (no hace falta responder)

| Dato | Valor | Origen |
|---|---|---|
| Marchas | 6 | carset, `[Gears] Team #01` |
| Neumático | Goodyear, banda 1, llanta `gy-oz-blck-yllw` | carset |
| Batalla | 2.881463 m | malla medida |
| Vía delantera / trasera | 1.809638 / 1.582146 m | malla medida |
| Radio de neumático delantero / trasero | 0.311949 / 0.322016 m | malla medida |
| Ancho de neumático delantero / trasero | 0.307541 / 0.389105 m | malla medida |
| Centros de rueda (runtime) | `(±0.904819, -0.226701, -1.146992)` y `(±0.791073, -0.223587, 1.734471)` | malla medida |
| Altura del suelo sobre el plano de apoyo | 0.031852 m | malla medida |
| Offset del piloto | ninguno | montaje medido |

## 2. Identidad y configuración

| # | Pregunta | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|
| 1 | Nombre completo del coche y versión (p. ej. Williams FW19 Renault, especificación de Hockenheim 1997) | Rothmans Williams Renault FW19; Jacques Villeneuve n.º 3; GP de Alemania en Hockenheim, 27-07-1997; motor Renault RS9 (subvariante no verificada) | published | Identidad, piloto y circuito: https://cdn.group.renault.com/ren/master/renault-classic/tools-and-services/30AnsDeFormule1.pdf ; https://www.formula1.com/en/results/1997/races/663/germany/qualifying/0 . No hay prueba suficiente del sufijo RS9A/RS9B en esta carrera. |
| 2 | Unidad de potencia de tu fuente (CV, bhp, kW) para poder convertir | bhp mecánicos como entrada; resultado en W (1 bhp = 745.699872 W) | published | https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html publica 755 bhp = 563003 W; ficha redondea 563 kW. No confundir bhp con CV métricos (735.49875 W/CV). |
| 3 | Combustible y lubricante de referencia de 1997 (proveedor y especificación) | Elf gasolina de carreras aprobada por FIA; Castrol lubricante motor. Formulaciones químicas exactas no publicadas | published | Ficha de equipo 1997: https://www.at-speed.co.uk/fw97sp.htm ; https://historicdb.fia.com/content/formula-one-technical-2 para marco reglamentario. No inferir octanaje o viscosidad exactos. |

## 3. Masa y distribución — dominio `chassis`

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 4 | Masa total del vehículo sin ruedas | kg | 547 kg (vehículo completo sin las 4 ruedas; incluye piloto 75 kg y cámara 3 kg; sin combustible) | estimated | 605 kg publicado con piloto y cámara en https://www.at-speed.co.uk/fw97sp.htm; sustraer 2×13 + 2×16 = 58 kg de conjuntos rueda estimados; 605-58=547. No corresponde a monocasco solo. |
| 5 | Masa de cada rueda montada (llanta + neumático) | kg | Delanteras 13 kg por rueda; traseras 16 kg por rueda | estimated | Llantas OZ 13 pulgadas, anchuras nominales 11.5/13.7 pulgadas: https://www.at-speed.co.uk/fw97sp.htm; masa conjunto con slick aproximada por neumático de competición. Validar por pesaje. |
| 6 | Reparto de peso delantero / trasero | % | 44.5 % delante / 55.5 % detrás en seco con piloto; CG longitudinal z≈+0.457 m sobre el origen empleado | estimated | Distribución de monoplaza de motor central; (0.445×2.890)=1.286 m desde eje trasero hacia delante. Ejes usados zF=-1.146992 y zR=1.743008 => zCG=+0.457; recalcular al variar lastre o combustible. |
| 7 | Altura del centro de gravedad sobre el suelo | m | 0.295 m sobre el plano de apoyo | estimated | Valor inicial razonado 0.27–0.33 m para monoplaza bajo, con RS9 de centro de gravedad rebajado: https://cdn.group.renault.com/ren/master/renault-classic/tools-and-services/30AnsDeFormule1.pdf ; https://www.formula1.com/en/latest/article/insight-a-closer-look-at-the-12-constructors-title-winning-cars-adrian-newey.1QAwEdYMG8hAz76kEHGefF; ninguna medición directa del FW19. |
| 8 | Multiplicadores de inercia respecto al modelo base (x, y, z) | adimensional | (1.00, 1.00, 1.00) respecto al perfil base; NO representa inercia histórica | inherited | Factores neutros conservando tensor del perfil base no proporcionado. No aplicar 1.24 heredado de otros vehículos como dato FW19. |
| 9 | Inventario de masas por componente (lista: componente, kg) | kg | monocasco/estructura de seguridad: 90; aerodinámica/carenados/suelo: 53; motor RS9 con equipos: 121; caja de 6 marchas y diferencial: 47; suspensión delantera y dirección: 22; suspensión trasera: 25; embrague/ejes/hidráulica: 14; frenos y pinzas: 20; radiadores/conductos: 23; aceite/refrigerante: 13; electricidad/ECU/cableado: 8; depósito vacío/conexiones: 12; asiento/arnés: 6; ruedas completas: 58; piloto con equipo: 75; cámara: 3; lastre: 15 kg. TOTAL 605 kg secos; combustible 0 kg | estimated | Balanza de masas cerrada para 605 kg publicados en https://www.at-speed.co.uk/fw97sp.htm; motor 121 kg de esa misma ficha. El resto es reparto ingenieril NO publicado, susceptible de ajuste para CG e inercias. |

## 4. Motor y transmisión — dominio `powertrain`

Cuestionario mínimo para que el modelo tenga una curva creíble. Si no tienes curva completa, da al
menos par máximo, régimen de par máximo y régimen de potencia máxima.

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 10 | Arquitectura y cilindrada (p. ej. V10 3.0 L atmosférico) | — | Renault RS9 V10 atmosférico, 71° entre bancadas, 0.003000 m³ (3.0 L), 40 válvulas neumáticas; motor longitudinal central-trasero | published | https://cdn.group.renault.com/ren/master/renault-classic/tools-and-services/30AnsDeFormule1.pdf ; https://www.at-speed.co.uk/fw97sp.htm ; https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html . |
| 11 | Ralentí | rpm | 3500 rpm | estimated | Ralentí alto inicial plausible para V10 de competición; sin dato de ECU de Renault. Calibrar con audio del RS9. |
| 12 | Régimen máximo | rpm | 15800 rpm corte operativo de partida (intervalo a investigar 15500–17000 rpm) | estimated | Potencia catalogada a 14600 rpm en https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html; fuentes contemporáneas/modernas discrepan: https://www.hpacademy.com/es/blog/como-modernizar-un-coche-de-formula-1-williams-650hp-na-v10-fw19-charla-tecnica/ menciona 17000 rpm anunciados. Renault solo documenta +600 rpm sobre RS8 https://cdn.group.renault.com/ren/master/renault-classic/tools-and-services/30AnsDeFormule1.pdf; no equiparar régimen de máxima potencia con corte. |
| 13 | Par máximo | N·m | 390 N·m | estimated | Curva consistente con 563 kW a 14600 rpm (par allí=368.3 N·m). Pico supuesto en 13200 rpm, sin dinamometría pública verificada. |
| 14 | Régimen de par máximo | rpm | 13200 rpm | estimated | Máximo construido con interpolación suave y descenso al acercarse a régimen de potencia máxima; validar mediante lap telemetry. |
| 15 | Potencia máxima | kW | 563000 W (563 kW redondeados, equivalentes aproximadamente a 755 bhp) | published | https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html señala 755 bhp/563 kW. Potencia de banco aproximada, NO confirmada específicamente para el motor montado en Hockenheim. |
| 16 | Régimen de potencia máxima | rpm | 14600 rpm (valor bibliográfico discutido; no confundir con limitador) | published | https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html asigna 755 bhp a 14600 rpm. Contraste: https://www.hpacademy.com/es/blog/como-modernizar-un-coche-de-formula-1-williams-650hp-na-v10-fw19-charla-tecnica/ cita 740 CV a 17000 rpm en publicidad original; utilizar este punto como calibración provisional, no verdad de telemetría. |
| 17 | Curva de par: pares (rpm, N·m) desde ralentí hasta corte | rpm / N·m | (3500, 165); (4500, 198); (6000, 246); (7500, 293); (9000, 330); (10500, 360); (12000, 383); (13200, 390); (14000, 381); (14600, 368.3); (15200, 345); (15800, 318) | estimated | Tabla en rpm,N·m por interpolación monótona. P=T·2π·rpm/60; produce pico ~563 kW a 14600 rpm. Sin telemetría de dinamómetro público: aproximación por forma de par de motor atmosférico de carrera. |
| 18 | Relaciones de cambio 1.ª a 6.ª | adimensional | (3.10, 2.40, 1.92, 1.57, 1.31, 1.12) | estimated | Seis marchas publicadas https://www.at-speed.co.uk/fw97sp.htm, ratios no publicados para Hockenheim. Escalonado estimado para V10 3.0 y velocidad máxima ~340–350 km/h. Cambios son configurables por circuito; no atribuir estos dientes a Williams. |
| 19 | Relación de grupo final | adimensional | 4.65:1 | estimated | En 6.ª: 1.12×4.65=5.208 global; radio trasero 0.322016 m; velocidad cinemática a 15200 rpm ≈354.3 km/h sin slip. Velocidad real limitada por potencia y arrastre. |
| 20 | Relación de marcha atrás | adimensional | -3.00:1 para engranaje de marcha atrás (con grupo 4.65); signo solo si API requiere invertir giro | estimated | No hay relación de dientes real; marcha atrás obligatoria de manera general, usar magnitud positiva 3.00 si el núcleo codifica signo aparte. |
| 21 | Tiempo de cambio | s | 0.065 s por subida; 0.090 s por reducción, interrupción efectiva de par | estimated | Cambio semiautomático e hidráulico https://www.at-speed.co.uk/fw97sp.htm; temporización estimada para caja rápida de finales de 1990; no hay medición de cortes de encendido FW19. |
| 22 | Momento de inercia del motor | kg·m² | 0.120 kg·m² (cigüeñal + volante + clutch reflejado al motor; 0.09–0.16 kg·m² plausible) | estimated | Estimación de momento polar de conjunto rotatorio V10 de 121 kg; no atribuir a Renault. Calibrar con velocidad de caída/subida de rpm en neutral. |
| 23 | Par de freno del motor a ralentí (`engine_braking`) si lo tienes | N·m | 35 N·m de par resistivo base a 3500 rpm con acelerador 0 (negativo en dinámica) | estimated | Modelo de bombeo, fricción y freno por motor. Debe variar con rpm; se propone 35 N·m al ralentí, 65 N·m a 10000 rpm. |
| 24 | Tipo de diferencial y precarga | N·m | LSD mecánico autoblocante de discos; precarga 90 N·m; bloqueo power 40 %, coast 25 % | estimated | Tracción trasera histórica; parámetros del diferencial no divulgados. Modelo de placas con preload verificable mediante ensayos de tracción/fase de entrada. |
| 25 | Reparto de par delantero / trasero | % | 0 % delantero / 100 % trasero | published | Transmisión con propulsión trasera documentada: https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html. |
| 26 | Temperatura inicial de aceite y agua en tanda | °C | Aceite 373.15 K; agua 363.15 K al salir de boxes | estimated | Temperaturas objetivo estimadas: aceite 100 °C, agua 90 °C en caliente; sin datos térmicos de Williams. Usar inicial frío si la sesión empieza sin prewarm. |
| 27 | Coeficientes térmicos del motor (bloque, aceite, agua, conductancias) | varios | inherited: tabla de bloque, aceite, agua, capacidades térmicas y conductancias del perfil base | inherited | Coeficientes de simulador sin correspondencia histórica directa; no poner cifras sin conocer contrato/nombres de esquema del núcleo. |

## 5. Combustible — dominio `fuel`

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 28 | Capacidad del depósito | kg | 105 kg de combustible utilizable (≈0.141 m³ a densidad 745 kg/m³) | estimated | Repostajes admitidos en la época https://www.formula1.com/en/latest/article/the-key-regulation-changes-in-f1-history-and-the-teams-that-nailed-them.2iq8c5E6S1HOffT5PBLl6i; depósito flexible dentro de célula de supervivencia https://historicdb.fia.com/content/formula-one-technical-2. Volumen de carga real FW19 no publicado; proponer capacidad compatible con stints de 1–2 paradas. |
| 29 | Consumo específico al freno (`brake_specific_consumption`) | kg/kWh | 0.315 kg/kWh (equiv. 8.75e-8 kg/J) | estimated | BSFC de gasolina de competición motor atmosférico de alto rendimiento (estimación 0.28–0.35 kg/kWh); eficiencia térmica ≈26.6% para LHV 43 MJ/kg. |
| 30 | Consumo estimado por vuelta en Hockenheim | kg | 3.65 kg/vuelta en Hockenheim histórico de 6.823 km (≈0.535 kg/km) | estimated | Usar P media efectiva≈390 kW, tiempo≈107 s: 390 kW×(107/3600)h×0.315 kg/kWh=3.65 kg. 45 vueltas≈164 kg. Validar contra resultados de https://www.formula1.com/en/results/1997/races/663/germany/qualifying/0 y https://www.formula1.com/en/results/1997/races/663/germany/fastest-laps; no son datos de repostaje reales. |
| 31 | Consumo a ralentí | kg/h | 5.0 kg/h (0.001389 kg/s) | estimated | Consumo cero carga de motor alto régimen, 5 kg/h≈6.7 L/h para gasolina de 745 kg/m³. Valor de partida, afinar con lambda y caudal de inyectores. |
| 32 | Posición del depósito respecto al origen del coche (x, y, z) | m | (0.000, -0.100, +0.600) m; local x lateral, y arriba, z hacia atrás | estimated | Entre respaldo del piloto y Renault V10, como exige arquitectura de seguridad https://historicdb.fia.com/content/formula-one-technical-2; coordenada respecto al origen de la malla, aproximación sin CAD del depósito. |

## 6. Dirección — dominio `steering`

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 33 | Ángulo máximo de dirección en las ruedas | rad o ° | 0.314 rad (18.0°) por rueda interior cerca del tope; exterior menor por Ackermann | estimated | Eje delantero con cremallera; limitado por neumático ancho y triángulos, reglaje más rápido para carrera. Rango de calibración 0.26–0.38 rad. |
| 34 | Relación de dirección delantera (volante/rueda) | adimensional | 12.0:1 (volante / rueda); volante tope≈3.77 rad=216° desde centro si rueda 0.314 rad | estimated | Puesta a punto por supuesta cremallera asistida descrita en https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html. 12:1 para uso en Hockenheim; no se dispone de relación real por carrera. |
| 35 | Ackermann (0 a 1) y exponente de dirección | — | Ackermann fraccional 0.15; exponente de entrada 1.00 (lineal); wheel-to-wheel geometría Ackermann ideal modulada al 15 % | estimated | Típico anti/low-Ackermann circuito con slick ancho; 0.15 es parámetro de modelo, no dato registrado. |
| 36 | Velocidad de giro del volante y su decaimiento | rad/s | Máximo 8.0 rad/s al volante; filtrado/decaimiento con τ=0.10 s (ganancia a alta velocidad 0.65) | estimated | Limitador de entrada para gamepad, no pieza del FW19. Sin ayudas de conducción: solo filtrado de input, con opción de desactivarlo en volante real. |
| 37 | Relación de dirección trasera (si existiera) | adimensional | 0.0: no dirección activa del eje trasero | published | Layout FW19 de dirección convencional delantera https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html; la dirección a cuatro ruedas no figuraba en el coche de carrera. https://www.formula1.com/en/latest/article/the-key-regulation-changes-in-f1-history-and-the-teams-that-nailed-them.2iq8c5E6S1HOffT5PBLl6i contexto de ayudas prohibidas. |

## 7. Suspensión — dominio `suspension`

Datos históricos: geometría de puntos de anclaje y muelles/amortiguadores. Si no tienes la geometría
real, dilo: el perfil podrá usar la geometría medida del modelo visual, pero se declarará `estimated`.

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 38 | Altura libre delantera / trasera en reglaje de Hockenheim | m | Delante 0.030 m / detrás 0.045 m (clearance estático nominal del fondo); rake≈0.00519 rad = 0.297° | estimated | Hockenheim: configuración aerodinámica descargada; https://www.formula1.com/en/latest/article/insight-a-closer-look-at-the-12-constructors-title-winning-cars-adrian-newey.1QAwEdYMG8hAz76kEHGefF destaca high-rake del FW19. La malla mide 0.031852 m en un punto; valor del eje posterior es hipótesis. No asociar alturas con bujes. |
| 39 | Muelle delantero / trasero | N/m | Delante torsión equivalente en accionamiento 158000 N/m / detrás helicoidal 168000 N/m; rueda efectiva 47800 / 71000 N/m | estimated | https://www.at-speed.co.uk/fw97sp.htm confirma barras de torsión delante y helicoidal detrás. Cálculo f=(1/2π)√(k_w/m_corner); masas suspendidas por esquina≈117/156 kg, f≈3.22/3.40 Hz, motion ratio 0.55/0.65. |
| 40 | Longitud libre e instalada del muelle delantero / trasero | m | Delante: no aplica longitud de muelle helicoidal (barra de torsión). Referencia de actuador 0.180 m libre / 0.165 m instalada; detrás helicoidal 0.210 m libre / 0.165 m instalada | estimated | Longitudes de empaquetamiento virtual de rocker/coil; NO dimensiones de barras originales, que requieren sección, longitud, módulo G y brazo de palanca. |
| 41 | Amortiguador delantero / trasero: bump y rebound | N·s/m | Actuador delantero bump 8200 y rebound 13500 N·s/m; trasero bump 8700 y rebound 13300 N·s/m | estimated | Amortiguadores Williams-Penske publicados, tarados no divulgados. Con ratios 0.55/0.65, efectivos rueda F 2480/4080 y R 3680/5620 N·s/m; ~0.55–0.9 del crítico. |
| 42 | Rodadura y caída de muelle (bump/droop) delantera / trasera | m | Delante bump +0.025 m / droop -0.035 m; detrás bump +0.030 m / droop -0.040 m (recorrido vertical rueda) | estimated | Rango funcional estimado, compresiones pequeñas por rigidez de F1 de efecto suelo limitado, topes para proteger patín. |
| 43 | Barra estabilizadora delantera / trasera: modo y ratio | — | Delante barra torsional activa con rigidez de rolido equivalente 14000 N·m/rad; detrás 10500 N·m/rad; ratios de conexión efectivos 1.0 | estimated | Cálculo de reparto inicial de roll stiffness, no medición de barras; implementar en su módulo como par de eje contra diferencia de carrera, evitando confundir N·m/rad con N/m. |
| 44 | Caída (camber) delantera / trasera en reglaje | rad o ° | Delante -0.05236 rad (-3.0°); detrás -0.03491 rad (-2.0°) estático | estimated | Setting de circuito con slick 1997; valores iniciales típicos, no setup de Villeneuve publicado. |
| 45 | Convergencia (toe) delantera / trasera | rad o ° | Delante toe-out total 0.00175 rad (0.10°); detrás toe-in total 0.00349 rad (0.20°), ángulo por rueda = mitad | estimated | Convención: toe-out positivo declarado en texto, normalización por rueda según motor; evitar duplicar total y por rueda. |
| 46 | Puntos de anclaje de la suspensión delantera: triángulo inferior y superior (interior delantero, interior trasero, exterior), tirante, pushrod, basculante y amortiguador | m (x, y, z) | Lado derecho x>0 (izquierdo x→-x), todos m: pivote inferior interior delantero (0.340,-0.300,-1.290), interior trasero (0.340,-0.300,-1.005), exterior (0.805,-0.290,-1.147); superior interior delantero (0.300,-0.080,-1.245), interior trasero (0.300,-0.080,-1.035), exterior (0.800,-0.055,-1.147); tirante dirección interior (0.310,-0.170,-1.055), exterior (0.800,-0.170,-1.075); pushrod exterior (0.790,-0.155,-1.175), rocker (0.290,0.110,-1.020); basculante pivot (0.290,0.110,-1.020); amortiguador entre rocker (0.290,0.110,-1.020) e interior (0.170,0.185,-0.800) | estimated | Kinemática sintética aproximada con vía publicada 1.670 m https://www.at-speed.co.uk/fw97sp.htm, no geometría medida ni CAD. Ejes propuestos F=(±0.835,-0.226701,-1.146992). Rehacer triángulos en función del GLB real corregido antes de usar físicamente. |
| 47 | Puntos de anclaje de la suspensión trasera (mismos elementos) | m (x, y, z) | Lado derecho x>0 (izquierdo x→-x), todos m: pivote inferior interior delantero (0.350,-0.290,1.490), interior trasero (0.350,-0.290,1.830), exterior (0.770,-0.275,1.743); superior interior delantero (0.310,-0.050,1.520), interior trasero (0.310,-0.050,1.790), exterior (0.760,-0.035,1.743); tirante de convergencia interior (0.320,-0.175,1.770), exterior (0.770,-0.170,1.820); pushrod exterior (0.755,-0.140,1.720), rocker (0.275,0.105,1.470); basculante pivot (0.275,0.105,1.470); amortiguador entre rocker (0.275,0.105,1.470) e interior (0.160,0.190,1.290) | estimated | Kinemática sintética con vía publicada 1.600 m https://www.at-speed.co.uk/fw97sp.htm; z trasero ajustado desde 1.734471 a 1.743008 para batalla 2.890 m. No geometría Williams real; geometría rocker permite rear spring coil según https://www.at-speed.co.uk/fw97sp.htm. |
| 48 | Centro de buje por esquina | m (x, y, z) | Medido malla (sin corregir): FL=(-0.904819,-0.226701,-1.146992); FR=(+0.904819,-0.226701,-1.146992); RL=(-0.791073,-0.223587,+1.734471); RR=(+0.791073,-0.223587,+1.734471) m | measured | Dato aportado en sección 1 del cuestionario; conflicto con vías publicadas https://www.at-speed.co.uk/fw97sp.htm. Versión corregida recomendable: F=(±0.835000,-0.226701,-1.146992); R=(±0.800000,-0.216635,+1.743008). Ajuste de y trasera al mismo plano de apoyo. |
| 49 | Barra estabilizadora y reparto si lo tienes documentado | — | Distribución de rigidez de barras delantera / trasera ≈57.1 % / 42.9 % (14000/24500 y 10500/24500); sin documentación de reglaje real | estimated | Derivado de P43, no dato histórico. Más importante calibrar gradientes de rolido por masa, reparto y transferencia lateral. |

## 8. Neumáticos — dominio `tires`

Las dimensiones ya están medidas. Aquí faltan presiones, compuesto y coeficientes de modelo.

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 50 | Presión en frío por esquina | kPa (manométrica) | FL=130000 Pa; FR=130000 Pa; RL=120000 Pa; RR=120000 Pa manométricos en frío | estimated | Goodyear Eagle slick certificado en ficha https://www.at-speed.co.uk/fw97sp.htm; presiones históricas específicas no publicadas. Calibración inicial ≈1.30/1.20 bar, lejos de afirmar mínimos seguros. https://ms.bridgestone.co.jp/special/document/f1/en/techs_regs/1997/ contexto de slicks 1997. |
| 51 | Presión de referencia en caliente por esquina | kPa (manométrica) | FL=160000 Pa; FR=160000 Pa; RL=150000 Pa; RR=150000 Pa manométricos en caliente | estimated | Aumento 30000 Pa por calentamiento en stint; T absoluta y volumen cambiante. Aproximación, no presión real de equipo. |
| 52 | Presión mínima y máxima de trabajo | kPa | Frío 110000–160000 Pa; caliente de referencia 140000–190000 Pa (manométricos). No límites de seguridad oficiales | estimated | Ventana SOLO numérica del modelo: proveedor Goodyear no ha publicado mínimos del slick FW19 1997. No interpretar como presión real segura. |
| 53 | Temperatura óptima de banda y de carcasa | °C | Banda 363.15 K; carcasa 353.15 K (óptimos iniciales, equivalentes 90/80 °C) | estimated | Ventana térmica supuesta; Villeneuve describe Goodyear 1997 como peaky y de ventana estrecha https://www.formula1.com/en/latest/article/how-i-beat-schumacher-to-the-title-at-jerez-97-by-jacques-villeneuve.2cerUxDqRaYUsoQi2KGSQW/; no hay temperaturas de compuestos específicas. |
| 54 | Temperatura de sobrecalentamiento | °C | Banda: inicio degradación por exceso a 393.15 K (120 °C); carcasa >383.15 K (110 °C) | estimated | Umbrales de simulación, no Goodyear medidos. Diferenciar calentamiento breve de blíster/fallo persistente. |
| 55 | Masa de la rueda delantera / trasera | kg | Delantera 13 kg / trasera 16 kg cada conjunto montado | estimated | Igual que P5. Evitar duplicar masa si el chasis ya incluye ruedas; 58 kg totales desenmuelles estimados. |
| 56 | Compuesto y coeficiente de fricción de referencia | — | Goodyear Eagle radial slick seco; opción de compuesto medio para carrera (no identificable de archivo carset); μ_ref longitudinal 1.75 y lateral 1.85 a carga nominal | estimated | Proveedor y slicks publicados https://www.at-speed.co.uk/fw97sp.htm; elección de compuesto para Hockenheim NO acreditada, seleccionar un compuesto tras libres https://ms.bridgestone.co.jp/special/document/f1/en/techs_regs/1997/. Coeficientes de adherencia son parametrización de simulator, no laboratorio Goodyear. |
| 57 | Pico de adherencia longitudinal y lateral (`peak_slip_ratio`, `peak_slip_angle`) | — | peak_slip_ratio = 0.105 (10.5 %); peak_slip_angle = 0.105 rad (6.0°) | estimated | Parámetros internos con slick pre-1998, no medida de FW19. Ajustar sobre modelo de carga vertical y combined-slip; 1.85 lateral de P56 solo con carga de referencia. |
| 58 | Caída de adherencia tras el pico (`falloff`, `slide_mu_ratio`) | — | falloff = 0.55 (pendiente relativa tras pico; convención dependiente del motor); slide_mu_ratio = 0.72 | estimated | Modelo no histórico; valor pospico útil para pérdida progresiva de adherencia. Falloff NO universal entre formulaciones: validar contrato antes de copiar el 0.55. |
| 59 | Coeficientes térmicos (capacidades caloríficas, conductancias) | varios | inherited (capacidad térmica y conductancias para banda, carcasa, aire, asfalto) | inherited | Coeficientes numéricos dependen de la implementación del núcleo, sin datos de diseño original Goodyear. |
| 60 | Desgaste por megajulio delantero / trasero | mm/MJ | inherited (mm/MJ delantero y trasero; mantener asimetría definida por base) | inherited | Desgaste modelado y energía de deslizamiento; no reproducir un wear-rate publicado inexistente. |
| 61 | Multiplicadores de adherencia por superficie (asfalto, curb, hierba, grava, tierra) | — | inherited para asfalto/curb/hierba/grava/tierra; referencia nominal asfalto=1.0 | inherited | Coeficientes por materiales de Open Formula, no propiedades físicas propias del FW19; conservar esquema original. |

## 9. Frenos — dominio `brakes`

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 62 | Diámetro y espesor del disco delantero / trasero | m | Disco delantero 0.278 m diámetro × 0.032 m espesor; trasero 0.278 m × 0.032 m | estimated | Carbone Industrie carbon-carbon y AP Racing publicados https://www.at-speed.co.uk/fw97sp.htm; dimensiones FW19 no accesibles. Aproximación tardíos 90s; no aplicar por analogía limitación de 1998+ como si rigiera en 1997. https://www.formula1.com/en/latest/article/re-writing-the-f1-rulebook-part-3-narrow-cars-find-their-groove.5YeEtrJfLako8UUG6Smgo4.5YeEtrJfLako8UUG6Smgo4 explica reducciones posteriores. |
| 63 | Número de pistones y diámetro de pistón delantero / trasero | — | 6 pistones opuestos por pinza (3 por lado); Ø delantero 0.032 m cada pistón, trasero 0.030 m | estimated | Pinzas AP publicadas https://www.at-speed.co.uk/fw97sp.htm, número/diámetro por pistón sin ficha exacta. Ejemplo hidráulico de 3 pistones por cara; revisar fotos antes de fijarlo. |
| 64 | Coeficiente de fricción de la pastilla | — | μ_hot=0.42 para carbono-carbono; μ_cold=0.25 como penalización inicial | estimated | Coeficiente del par Carbone Industrie a temperatura estable, estimado. No usar fricción constante fría y caliente. |
| 65 | Par máximo de frenada delantero / trasero | N·m | Por rueda delantera hasta 2000 N·m; por rueda trasera hasta 1400 N·m; axle totals 4000/2800 N·m | estimated | A pF≈8.5 MPa: F_N=2×p×3×π(0.032²)/4 ≈41 kN; T≈F_N×μ0.42×r_eff0.115≈1980 N·m. Atrás: p≈7.0 MPa, d0.030, r_eff0.110=>~1440 N·m. Límite adicional por μ_tire×Fz×r. |
| 66 | Reparto de frenada delantero / trasero | % | 59 % delante / 41 % detrás, presión de pedal en alta adherencia; ajuste permitido por balance bar | estimated | Proporción de límite de pares 4000/(4000+2800)=58.8%. Puede moverse dinámicamente con carga transferida; no equilibrio probado para Villeneuve. |
| 67 | Refrigeración: área de entrada y conductos si lo tienes | m² | Entrada delantera 0.0030 m² por rueda; trasera 0.0022 m² por rueda; conducto alimenta campana y caras del disco | estimated | Estimación de secciones térmicas y aerodinámicas de régimen Hockenheim; diám. equivalentes 61.8/52.9 mm para abertura circular. No hay conductos medidos. |
| 68 | Presión y temperatura de trabajo | bar / °C | Presión máxima de circuito delantero 8.5e6 Pa y trasero 7.0e6 Pa; discos en trabajo 623–1073 K (350–800 °C) | estimated | Presión calculada desde P63–65 para μ=0.42; temperatura de carbono competición aproximada. Temperatura mínima de alta eficiencia estimada 623 K. |

## 10. Aerodinámica — dominio `aerodynamics`

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 69 | Área frontal | m² | 1.15 m² de área frontal proyectada completa (ruedas/alerones incluidos) | estimated | Dimensión publicada ≈2.0 m de ancho y 4.15 m de largo https://www.at-speed.co.uk/fw97sp.htm; área frontal no medida en GLB. Estimar A 1.05–1.25 m² por proyección; preferir medición raster ortográfica. |
| 70 | Coeficiente de arrastre (Cd) | — | Cd=0.83 referido a A=1.15 m²; CdA=0.9545 m² para low-drag Hockenheim | estimated | Ecuación P_drag=0.5ρCdA v³. Para ρ=1.20 kg/m³, 350 km/h consume ≈526 kW aerodinámicos; corrección: usar este valor al verificar Vmax con pérdidas. No coeficiente Williams medido. |
| 71 | Coeficiente de sustentación (Cl, negativo) | — | Cl=-1.80 referido a A=1.15 m² (ClA=-2.070 m²), en reglaje bajo apoyo | estimated | Con v=80 m/s y ρ=1.2 da carga Fz≈7949 N. https://www.formula1.com/en/latest/article/insight-a-closer-look-at-the-12-constructors-title-winning-cars-adrian-newey.1QAwEdYMG8hAz76kEHGefF respalda alto rake/caracter aero, pero no publica Cl; rango plausible -1.5 a -2.5 por setup. |
| 72 | Reparto aerodinámico delantero / trasero | % | 47 % delantero / 53 % trasero de carga aerodinámica a altura nominal | estimated | Balance base de monoplaza 1997; https://www.formula1.com/en/latest/article/insight-a-closer-look-at-the-12-constructors-title-winning-cars-adrian-newey.1QAwEdYMG8hAz76kEHGefF destaca aero trasera y difusor. Debe resultar estable con 44.5/55.5 % masa; calibrar con simulación y Onboard. |
| 73 | Sensibilidad al recorrido de suspensión (altura y rake) | — | Sensibilidad cerca del nominal: dCl/dh_F≈+6.0 m⁻¹ y dCl/dh_R≈+3.0 m⁻¹ para Cl negativo; dCl/d(rake)≈-10 rad⁻¹; limitar respuesta al calado | estimated | Derivadas locales inventariadas para simulación, no túnel de viento. Cuando el coche desciende h_F, Cl se vuelve más negativo; alto rake puede aumentar carga hasta el umbral. Requiere map aero 2D calibrado. |
| 74 | Configuración de alerones de Hockenheim: ángulos o reglaje | ° o — | Configuración baja resistencia: ala delantera ≈6° y trasera ≈5° geométricos efectivos; plano principal trasero de baja incidencia, sin gurney adicional por defecto | estimated | Configuración orientada a rectas antiguas de Hockenheim; https://www.grandprix.com/races/german-gp-1997.html condiciones y rendimiento. Ángulos indicativos no ajustables directamente a un perfil polar sin referencia geométrica; NO setup original de Villeneuve. |
| 75 | Coeficientes del modelo aero (arrastre inducido, punto de calado, altura óptima) | varios | inherited: drag inducido, altura óptima, zona de calado y regularización numérica del núcleo | inherited | Parámetros propios del modelo de simulación, no extraíbles de publicaciones FW19 sin medición aerodinámica. |

## 11. Comportamiento y contacto — dominios `underfloor_contact`, `coupled_world`, `driving_assistance`

| # | Parámetro | Unidad | Respuesta | Clasificación | Referencia |
|---|---|---|---|---|---|
| 76 | Altura de contacto del fondo plano y rigidez de contacto | m / N/m | Holgura nominal fondo=0.031852 m (dato malla); colisión desde separación al suelo 0.012 m; rigidez normal equivalente k=8.0e5 N/m + amortiguamiento 2500 N·s/m | estimated | 0.031852 m es la medida aportada, pero activación y rigidez son valores de contacto aproximados. 0.012 m umbral se recalibra con neumáticos y recorrido de suspensión; evitar doble colisión del patín. |
| 77 | Desgaste del patín o fondo si lo tienes documentado | — | Patín de madera/composite: espesor nuevo nominal 0.010 m; límite de desgaste de referencia 0.001 m; tasa de abrasión inherited | estimated | El patín y límite 1 mm son contexto reglamentario de esa época https://www.formula1.com/en/latest/article/re-writing-the-f1-rulebook-part-2--from-driver-aids-to-stepped.6ng1HQEGGsImQKekgaG4sw; geometría, espesor exacto tras 1995 y wear-rate del FW19 no comprobados. NO confundir desgaste tolerado con parámetro por MJ. |
| 78 | Ayudas a mantener (ninguna, ABS, control de tracción, ambos) para 1997 | — | Ninguna: no ABS, no control de tracción, no suspensión activa; caja semiautomática SÍ | published | Prohibición de ayudas electrónicas desde 1994, salvo caja semiautomática: https://www.formula1.com/en/latest/article/the-key-regulation-changes-in-f1-history-and-the-teams-that-nailed-them.2iq8c5E6S1HOffT5PBLl6i . |
| 79 | Nivel de ayuda deseado en el juego (0 = ninguna) | 0–1 | 0.0 (ningún ABS/TC/asistencia de estabilidad ni par corrector) | estimated | Decisión de fidelidad solicitada: comportamiento 1997. El filtrado de joystick P36 no debe añadir adherencia ni cambiar fuerzas de neumático. |

## 12. Audio — dominio `audio`

No son datos históricos: son ganancias y filtros del motor de sonido. Indica únicamente si aceptas
heredarlos o si aportarás una grabación propia del V10 Renault de 1997.

| # | Pregunta | Respuesta |
|---|---|---|
| 80 | ¿Heredar la configuración de audio del perfil base? | Sí: inherited; mantener bancos y filtros activos del perfil base, sin afirmar equivalencia acústica RS9 |
| 81 | ¿Hay grabación propia del motor Renault RS9/RS9B? (ruta y condiciones) | unknown: no se adjuntó grabación autenticada Renault RS9/RS9A/RS9B con rpm/carga; NO inventar ruta. Reutilizar audio activo provisionalmente |

## 13. Coeficientes de modelo sin dato histórico

Estos campos existen en el perfil pero no describen el FW19: son parámetros internos. Marca una opción
por fila.

| # | Grupo | Opciones | Respuesta |
|---|---|---|---|
| 82 | Modelo de neumático (relajación, migración de pico, rampas) | `inherited` / `custom` | inherited (no asumir parámetros Pacejka publicados) |
| 83 | Térmica de neumático y motor | `inherited` / `custom` | inherited (aplicar iniciales 373.15/363.15 K solo si formato del núcleo los acepta) |
| 84 | Desgaste de neumático | `inherited` / `custom` | inherited (calibrar tiempos de stint con gomas Goodyear sin tocar tasas arbitrariamente) |
| 85 | Presentación visual de suspensión | `measured_from_mesh` / `custom` | measured_from_mesh (primero corregir vías y validar pivotes propuestos P46–47) |
| 86 | Mundo acoplado (`coupled_world`) | `inherited` / `custom` | inherited (sin cambios del mundo acoplado a causa del carset) |

## 14. Fuentes y permisos

| # | Pregunta | Respuesta |
|---|---|---|
| 87 | ¿Las fuentes son públicas y citables? Enumera las principales | Sí: Renault documento oficial https://cdn.group.renault.com/ren/master/renault-classic/tools-and-services/30AnsDeFormule1.pdf; reglamento histórico FIA https://historicdb.fia.com/content/formula-one-technical-2; F1.com archivo oficial https://www.formula1.com/en/results/1997/races/663/germany/qualifying/0 , https://www.formula1.com/en/results/1997/races/663/germany/fastest-laps , https://www.formula1.com/en/results/1997/races/663/germany/race-result; Bridgestone historiales https://ms.bridgestone.co.jp/special/document/f1/en/techs_regs/1997/; ficha contemporánea de especificaciones publicada por Speed https://www.at-speed.co.uk/fw97sp.htm; interpretación técnica contextual https://www.formula1.com/en/latest/article/insight-a-closer-look-at-the-12-constructors-title-winning-cars-adrian-newey.1QAwEdYMG8hAz76kEHGefF . Fuentes secundarias anotadas de forma diferenciada. |
| 88 | ¿Alguna fuente tiene restricción de uso o redistribución? | Sí: documentación de terceros (texto, fotografías, marcas, audios, modelos 3D) puede tener copyright; solo citar hechos y enlaces, no redistribuir PDFs, fotos, logos, audio ni geometrías sin permisos. Los valores calculados son originales del perfil. |
| 89 | ¿Autorizas que los valores estimados se marquen como tales en el manifest? | Sí para este entregable: etiquetar explícitamente published, measured, estimated e inherited en manifest. Esta instrucción no constituye licencia de redistribución de archivos de terceros. |

## 15. Cómo devolver el archivo

1. Guarda el archivo con el mismo nombre y extensión `.md`.
2. No borres las preguntas: aunque no tengas el dato, escribe `unknown` y explica por qué.
3. Si una tabla te resulta incómoda, añade una sección al final `## Respuestas adicionales` con el
   número de pregunta y el valor; se integrará igual.

Cuando lo devuelvas, el perfil se construirá con tres capas explícitas: `measured`/`published`,
`estimated` con su fuente, e `inherited` para los coeficientes de modelo. Ningún campo quedará sin
clasificar, y los `unknown` bloquearán la afirmación de simulación Williams para su dominio.

## 16. Notas de ingeniería y trazabilidad (anexo; no modifica la secuencia 1–89)

### Jerarquía de datos

- **Publicado:** datos confirmados en fuente concreta, no implica que correspondan al *setup* exacto de Villeneuve en Hockenheim. La ficha de 1997 fija 605 kg con piloto y cámara, V10 RS9, transmisión de seis marchas, proveedor de frenos, ruedas OZ y vías de 1.670/1.600 m. Renault confirma oficialmente 71° y 11 kg menos que su predecesor.
- **Medido:** los cuatro bujes en pregunta 48 son coordenadas de la malla del cuestionario, no mediciones del chasis físico.
- **Estimado:** propone valores iniciales concretos reproducibles para completar el simulador; no pueden etiquetarse como telemetría Williams.
- **Heredado:** se debe copiar el perfil de física de referencia en el código del proyecto, preservando semántica, rango y dimensiones de cada parámetro. *No se tuvo acceso al JSON real* y por tanto no se inventaron nombres de claves ni valores de modelos internos.

### P0: inconsistencia dimensional que debe corregirse ANTES de calibrar neumáticos o suspensión

- **Vía delantera:** malla 1.809638 m; ficha histórica 1.670 m; desviación **+0.139638 m (+8.36 %)**. Con ancho visual 0.307541 m, ancho exterior delantero aproximado = **2.117179 m**, por encima del ancho típico máximo de **2.0 m** de los F1 de 1997. Las anchuras de neumático de la malla no están verificadas contra el OZ 11.5/13.7 pulgadas de la ficha (éstas son anchuras de llanta, no ancho total de goma). Un monoplaza más ancho falsea transferencia lateral, centro de presión y trazado de colisiones. Documentación de cambio reglamentario posterior 1998: https://www.formula1.com/en/latest/article/re-writing-the-f1-rulebook-part-3-narrow-cars-find-their-groove.5YeEtrJfLako8UUG6Smgo4.5YeEtrJfLako8UUG6Smgo4.
- **Vía trasera:** malla 1.582146 m frente a 1.600 m histórica, falta 0.017854 m (-1.12 %).
- **Batalla:** 2.881463 m frente a 2.890 m histórica, falta 0.008537 m (-0.30 %).
- **Plano de apoyo inconsistente:** yF−rF = -0.538650 m, yR−rR = -0.545603 m: diferencia 0.006953 m de contacto de los neumáticos. Una normalización útil preserva el plano delantero y eleva centro trasero a -0.216634 m (redondeado -0.216635 en P48).
- **Geometría de implementación:** para lograr configuración histórica, usar xF=±0.835 m; xR=±0.800 m; zF=-1.146992 m; zR=+1.743008 m. P46–47 proponen pivotes compatibles con esa geometría *normalizada*, NO aplicarlos a la malla actual sin transformarla. Reestimar neumáticos con anchura homologable y conservar radios solo si malla coincide con un neumático real.

### Fórmulas reproducibles

1. **Masa**: 605 = 547 kg sin ruedas + 2×13 kg ruedas delanteras + 2×16 kg ruedas traseras. P9 cierra el inventario a 605 kg. La masa publicada incluye conductor/cámara, no combustible; el valor de tanque P28 es una estimación independiente. Masa inicial carrera = 605 + combustible inicial (kg) si se usa este inventario.
2. **Potencia y par**: P(W)=T(N·m)·2π·RPM/60. Con T=368.3 N·m a 14600 rpm: 563000 W aprox. Fuente de 755 bhp a 14600 rpm https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html no equivale a corte de 14600 rpm; la curva propuesta va hasta corte supuesto de 15800 rpm. **Contradicción bibliográfica abierta:** https://www.hpacademy.com/es/blog/como-modernizar-un-coche-de-formula-1-williams-650hp-na-v10-fw19-charla-tecnica/ menciona promoción original de 740 hp a 17000 rpm. Se debe validar si se refiere a potencia, rpm utilizable o a una evolución de la unidad; no aumentar la curva a 17000 solo para igualar una fuente secundaria.
3. **Velocidad teórica**: v(m/s)=RPM/[(ratio_marcha)·(ratio_final)] × 2πr_rueda(m)/60. 6.ª=1.12, final=4.65, r=0.322016 m → aprox 354.3 km/h a 15200 rpm, **límite cinemático**, NO velocidad máxima medida. La velocidad efectiva depende de CdA, resistencia a la rodadura, pérdidas y viento.
4. **Consumo**: m_fuel(kg/vuelta)=BSFC(kg/kWh)×P_media(kW)×lap_time(h). Aquí 0.315×390×(107/3600)=3.65 kg/vuelta. Usar Hockenheim de **1997, 6.823 km**, no el trazado acortado desde 2002. El número de vueltas de carrera de 1997 fue 45, según https://www.formula1.com/en/results/1997/races/663/germany/race-result; consumo de carrera estimado ≈164 kg, con repostajes.
5. **Suspensión**: k_rueda=m_esquina_suspendida×(2πf)²; k_rueda=k_actuador×(motion_ratio)². Para los 0.55/0.65 indicados, los muelles P39 son equivalentes de actuador; delante realmente es **barra de torsión**, no muelle helicoidal. Debe calcularse el torque en la barra/rocker si la representación es multicuerpo.
6. **Frenada**: par T≈(2×presión×área_pistones_de_una_cara) × μ_pastilla × radio_efectivo_disco. La limitación real es el mínimo entre torque hidráulico y adherencia de contacto. No modelar ABS: estaba prohibido.
7. **Aerodinámica**: D=0.5ρA·Cd·v² y downforce=−0.5ρA·Cl·v². Diferenciar ClA de Cl: con A=1.15 m², Cl=-1.80, Cd=0.83 se obtiene CdA=0.9545 m² y ClA=-2.070 m². Validar velocidad terminal en Hockenheim, ya que pueden requerirse reducción de CdA o de potencia absorbida.

### Pruebas de aceptación para el manifest del simulador

- Identidad: n.º 3 Jacques Villeneuve, FW19 RS9 1997, Goodyear slick, sin TC/ABS, 6 marchas.
- Física: masas y cargas consistentes, sin dobles sumas de masas rueda/piloto/combustible, coordenadas de ejes consistentes con 2.890 m de batalla y 1.670/1.600 m de vías.
- Motor: curva continua, pico de potencia en el régimen seleccionado; corte superior a régimen potencia; par sin valores negativos no intencionales.
- Hockenheim: vuelta clasificatoria Villeneuve **1:42.967** (P9), vuelta más rápida de carrera **1:47.044** (vuelta 24), registros oficiales https://www.formula1.com/en/results/1997/races/663/germany/qualifying/0 y https://www.formula1.com/en/results/1997/races/663/germany/fastest-laps. No forzar una única simulación para igualar ambos: combustible, neumáticos, temperatura y neumáticos varían.
- Motor de física: registrar en manifest fuentes y clasificación **por parámetro**, así como rangos de incertidumbre y condiciones de calibración.

### Bibliografía priorizada

1. Renault Classic, *30 Years of Formula 1* (fabricante): https://cdn.group.renault.com/ren/master/renault-classic/tools-and-services/30AnsDeFormule1.pdf
2. Rothmans Williams Renault FW19, ficha técnica publicada en 1997 (copia de época): https://www.at-speed.co.uk/fw97sp.htm
3. FIA, repositorio histórico de reglamento técnico 1997: https://historicdb.fia.com/content/formula-one-technical-2 (el PDF no fue accesible automáticamente desde el servidor al redactar este archivo; no se atribuyen artículos no verificados).
4. Formula1.com, informe técnico FW19 y contexto reglamentario 1994/1998: https://www.formula1.com/en/latest/article/insight-a-closer-look-at-the-12-constructors-title-winning-cars-adrian-newey.1QAwEdYMG8hAz76kEHGefF ; https://www.formula1.com/en/latest/article/the-key-regulation-changes-in-f1-history-and-the-teams-that-nailed-them.2iq8c5E6S1HOffT5PBLl6i ; https://www.formula1.com/en/latest/article/re-writing-the-f1-rulebook-part-3-narrow-cars-find-their-groove.5YeEtrJfLako8UUG6Smgo4.5YeEtrJfLako8UUG6Smgo4
5. Formula1.com, resultados Hockenheim 1997 clasificación/carrera/vuelta rápida: https://www.formula1.com/en/results/1997/races/663/germany/qualifying/0 ; https://www.formula1.com/en/results/1997/races/663/germany/race-result ; https://www.formula1.com/en/results/1997/races/663/germany/fastest-laps
6. Bridgestone, archivo del reglamento de slicks 1997: https://ms.bridgestone.co.jp/special/document/f1/en/techs_regs/1997/
7. Ultimatecarpage, ficha FW19 RS9 con cifras de potencia: https://www.ultimatecarpage.com/spec/1057/Williams-FW19-Renault.html (fuente secundaria).
8. High Performance Academy, modernización de FW19 y discrepancia RPM anunciadas: https://www.hpacademy.com/es/blog/como-modernizar-un-coche-de-formula-1-williams-650hp-na-v10-fw19-charla-tecnica/ (fuente secundaria).
9. Grandprix.com, crónica de carrera y análisis técnico de época: https://www.grandprix.com/races/german-gp-1997.html ; https://www.grandprix.com/features/joe-saward/technical-williams-renault-fw19.html
