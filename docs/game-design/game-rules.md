# F1 2030: Reglas del Juego

1. **Simulación Híbrida:** La simulación general utiliza GEVP (Godot Easy Vehicle Physics) sobre RigidBody3D, apuntando a un estilo de conducción *Simcade*.
2. **Prioridad Física:** Los valores y parámetros deben derivar de lógicas del mundo real (masa en kg, curvas de par, fuerza aerodinámica dependiente de la velocidad al cuadrado) en lugar de multiplicadores arbitrarios, siempre que sea posible.
3. **Mecánicas del Jugador:** Ajustar el Brake Bias y gestionar el acelerador en ausencia de Control de Tracción son las habilidades primarias.
