shader_type spatial;
render_mode unshaded, cull_disabled, fog_disabled;

// Sky gradient shader — degradado de cielo según hora del día (Formula-90)
// Reemplaza sky.png como fondo. El sprite llena el frustum y sigue la cámara.
// UV.y = 0 → zenit (parte superior), UV.y = 1 → suelo (parte inferior).

uniform vec3 zenith_color : source_color = vec3(0.18, 0.42, 0.82);
uniform vec3 horizon_color : source_color = vec3(0.95, 0.92, 0.82);
uniform vec3 ground_color : source_color = vec3(0.72, 0.68, 0.55);
uniform float horizon_sharpness : hint_range(0.1, 3.0) = 0.65;
uniform float ground_start : hint_range(0.0, 1.0) = 0.48;

void fragment() {
	float v = UV.y;

	vec3 col;
	if (v < ground_start) {
		float t = v / ground_start;
		col = mix(zenith_color, horizon_color, pow(t, horizon_sharpness));
	} else {
		float t = (v - ground_start) / (1.0 - ground_start);
		col = mix(horizon_color, ground_color, t);
	}

	ALBEDO = col;
	ALPHA = 1.0;
}
