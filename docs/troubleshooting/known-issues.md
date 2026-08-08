# Known Issues

Registro de problemas actuales o patrones de error activos en el proyecto. 

*Instrucciones de uso: Consulta este documento en DIAGNOSTIC MODE antes de intentar solucionar problemas recurrentes.*

---

## 1. Oscilación de Cámara en Alta Velocidad

**Symptom:**
La cámara (`ArcadeChaseCamera` en C++ o equivalentes) tiembla violentamente cuando el vehículo toma curvas prolongadas a muy alta velocidad.

**Likely causes:**
Conflictos en el orden de procesamiento físico (Interpolación de RigidBody3D de GEVP chocando con el `_process` de la cámara) o estabilización redundante en múltiples ejes.

**Checks:**
- Revisar si `test_field.tscn` tiene activada interpolación asíncrona.
- Verificar el script de GEVP que pasa los transforms.

**Do not:**
Añadir más capas de `lerp()` matemático sin antes identificar qué sistema es dueño de la rotación absoluta del frame actual.

**Related commits:**
- `8c39a10`, `2da7f5f`, `9642eed`, `b8901c3`
