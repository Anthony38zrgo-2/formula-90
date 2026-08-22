#pragma once

#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/classes/viewport.hpp>
#include <godot_cpp/classes/control.hpp>
#include <godot_cpp/variant/string.hpp>
#include <godot_cpp/variant/node_path.hpp>

namespace godot {

/// C-struct layout matching `CPsxArtConfig` in `psx-art-pluggin/src/c_abi.rs`.
struct CPsxArtConfigNative {
	uint32_t internal_width;
	uint32_t internal_height;
	uint32_t upscale_mode; // 0=PixelPerfect, 1=SharpBilinear, 2=CrtSmooth, 3=NativeHiresPsx
	uint32_t fps_cap;

	float vertex_snap_distance;
	float affine_texture_strength;

	uint32_t bit_depth;
	uint32_t dither_matrix_type; // 0=None, 1=Bayer2x2, 2=Bayer4x4, 3=Bayer8x8
	float dither_strength;

	bool fog_enabled;
	float fog_color[4];
	float fog_near;
	float fog_far;

	bool scanlines_enabled;
	float scanline_opacity;
	float composite_bleed;
};

typedef int32_t (*FnPsxArtParsePreset)(const char *json_ptr, CPsxArtConfigNative *out_config);
typedef int32_t (*FnPsxArtGetDitherMatrix)(uint32_t matrix_type, float *out_buffer, size_t max_len, size_t *out_len);
typedef uint32_t (*FnPsxArtVersion)();

class PsxArtController : public Node {
	GDCLASS(PsxArtController, Node)

private:
	String preset_path_ = "res://data/visuals/psx_authentic_94.json";
	bool auto_apply_ = true;
	NodePath target_viewport_path_;
	NodePath target_presenter_path_;
	bool override_engine_fps_ = true;

	void *dll_handle_ = nullptr;
	FnPsxArtParsePreset fn_parse_preset_ = nullptr;
	FnPsxArtGetDitherMatrix fn_get_dither_ = nullptr;
	FnPsxArtVersion fn_version_ = nullptr;

	CPsxArtConfigNative current_config_{};
	String current_profile_name_ = "";
	bool is_loaded_ = false;

	bool load_dll();
	void unload_dll();

protected:
	static void _bind_methods();

public:
	PsxArtController();
	~PsxArtController() override;

	void _ready() override;

	void set_preset_path(const String &p_path);
	String get_preset_path() const;

	void set_auto_apply(bool p_auto);
	bool get_auto_apply() const;

	void set_target_viewport_path(const NodePath &p_path);
	NodePath get_target_viewport_path() const;

	void set_target_presenter_path(const NodePath &p_path);
	NodePath get_target_presenter_path() const;

	void set_override_engine_fps(bool p_override);
	bool get_override_engine_fps() const;

	bool load_preset(const String &p_path);
	bool apply_preset();

	String get_current_profile_name() const { return current_profile_name_; }
	int get_internal_width() const { return (int)current_config_.internal_width; }
	int get_internal_height() const { return (int)current_config_.internal_height; }
	int get_upscale_mode() const { return (int)current_config_.upscale_mode; }
	int get_bit_depth() const { return (int)current_config_.bit_depth; }
	float get_vertex_snap_distance() const { return current_config_.vertex_snap_distance; }
	float get_affine_texture_strength() const { return current_config_.affine_texture_strength; }
	bool is_preset_loaded() const { return is_loaded_; }
};

} // namespace godot
