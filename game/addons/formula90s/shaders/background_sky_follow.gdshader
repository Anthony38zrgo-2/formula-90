shader_type canvas_item;

// Sky shader: proyecta una textura de panorama en pantalla segun la direccion
// de vista de la camara. Siempre llena la pantalla sin importar la rotacion.
// Usa un mapeo equirectangular simple: U = yaw/(2*PI), V = pitch/PI.

uniform sampler2D sky_texture : source_color;
uniform vec2 camera_euler; // (pitch, yaw) en radianes

void fragment() {
	float yaw = camera_euler.y;
	float pitch = camera_euler.x;

	// Coordenadas de pantalla normalizadas [0,1]
	vec2 uv = SCREEN_UV;

	// Mapeo equirectangular: la textura cubre 360 horizontal, 180 vertical.
	// El centro de la textura (0.5, 0.5) corresponde a la direccion de vista.
	float u = uv.x + yaw / (2.0 * PI);
	float v = uv.y + pitch / PI;

	// Wrap
	u = fract(u);
	v = clamp(v, 0.0, 1.0);

	COLOR = texture(sky_texture, vec2(u, v));
}
