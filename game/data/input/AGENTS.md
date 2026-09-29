# input — Input profile data

Scope: the versioned JSON profile with every keyboard and gamepad action, deadzones, response curves, the common steering return speed, and vibration band settings.
Consumers: the InputBindings autoload and the vehicle input and rumble controllers.
Rules: this JSON is the only truth for bindings and input shaping; adding a consumer never moves data into GDScript. The schema version gates every parse.
