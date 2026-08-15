shader_type spatial;
render_mode unshaded, cull_disabled, fog_disabled;

// Cloud layer shader — nubes procedurales FBM (Formula-90)
// Genera un patrón de nubes con ruido FBM 4 octavas y scroll lateral lento.
// No requiere textura: el ruido se calcula en el fragment shader.
// quad_size escala las UV al tamaño real del sprite en unidades mundo,
// garantizando resolución de ruido consistente independientemente de la textura dummy.

uniform float cloud_scale : hint_range(0.001, 0.05) = 0.012;
uniform float cloud_coverage : hint_range(0.0, 1.0) = 0.45;
uniform float cloud_sharpness : hint_range(0.1, 5.0) = 2.5;
uniform float wind_speed : hint_range(0.0, 0.1) = 0.005;
uniform vec3 cloud_color : source_color = vec3(1.0, 1.0, 1.0);
uniform float cloud_opacity : hint_range(0.0, 1.0) = 0.6;
uniform float quad_size = 100.0;

float hash(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	f = f * f * (3.0 - 2.0 * f);
	float a = hash(i);
	float b = hash(i + vec2(1.0, 0.0));
	float c = hash(i + vec2(0.0, 1.0));
	float d = hash(i + vec2(1.0, 1.0));
	return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float amp = 0.5;
	float freq = 1.0;
	for (int i = 0; i < 4; i++) {
		v += amp * noise(p * freq);
		freq *= 2.0;
		amp *= 0.5;
	}
	return v;
}

void fragment() {
	// Scale UV by quad_size so noise resolution is in world units, not texture pixels.
	vec2 uv = UV * quad_size;
	uv.x += TIME * wind_speed * quad_size;
	float n = fbm(uv / cloud_scale);
	float edge = 1.0 / max(cloud_sharpness, 0.01);
	float cloud = smoothstep(cloud_coverage, cloud_coverage + edge, n);
	ALBEDO = cloud_color;
	ALPHA = cloud * cloud_opacity;
}
