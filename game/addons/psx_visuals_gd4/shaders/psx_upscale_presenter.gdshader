shader_type canvas_item;

#include "res://addons/psx_visuals_gd4/shaders/psx.gdshaderinc"

uniform sampler2D screen_texture : hint_screen_texture, filter_linear_mipmap;
uniform bool use_sharp_bilinear = true;
uniform bool use_crt_scanlines = false;
uniform float scanline_opacity : hint_range(0.0, 1.0) = 0.15;
uniform float composite_bleed : hint_range(0.0, 0.2) = 0.02;

// Sharp Bilinear sampling helper
vec4 sample_sharp_bilinear(sampler2D tex, vec2 uv, vec2 tex_size) {
	vec2 tex_coord = uv * tex_size - vec2(0.5);
	vec2 i = floor(tex_coord);
	vec2 f = fract(tex_coord);
	vec2 df = fwidth(tex_coord);

	if (df.x <= 0.00001 || df.y <= 0.00001) {
		return texture(tex, uv);
	}

	// Sharp bilinear step calculation
	vec2 sharp_f = clamp((f - 0.5 + df) / (2.0 * df), 0.0, 1.0);
	vec2 sample_uv = (i + 0.5 + sharp_f) / tex_size;

	return texture(tex, sample_uv);
}

void fragment() {
	vec2 tex_size = vec2(textureSize(TEXTURE, 0));
	if (tex_size.x <= 1.0 || tex_size.y <= 1.0) {
		tex_size = vec2(320.0, 240.0);
	}

	vec4 base_color;
	if (use_sharp_bilinear) {
		base_color = sample_sharp_bilinear(TEXTURE, UV, tex_size);
		if (composite_bleed > 0.001) {
			vec2 offset = vec2(composite_bleed / tex_size.x, 0.0);
			vec4 r_col = sample_sharp_bilinear(TEXTURE, UV + offset, tex_size);
			vec4 b_col = sample_sharp_bilinear(TEXTURE, UV - offset, tex_size);
			base_color.r = mix(base_color.r, r_col.r, 0.4);
			base_color.b = mix(base_color.b, b_col.b, 0.4);
		}
	} else {
		base_color = texture(TEXTURE, UV);
	}

	// Apply 16-bit color quantization and authentic PSX Bayer Dithering
	int bit_depth = (psx_bit_depth > 0) ? psx_bit_depth : 5;
	vec3 dithered = dither(base_color, FRAGCOORD.xy, bit_depth).rgb;

	// Optional CRT scanlines
	if (use_crt_scanlines && scanline_opacity > 0.001) {
		float scanline = sin(UV.y * tex_size.y * PI * 2.0);
		float scanline_factor = 1.0 - (scanline * 0.5 + 0.5) * scanline_opacity;
		dithered *= scanline_factor;
	}

	COLOR = vec4(dithered, 1.0);
}
