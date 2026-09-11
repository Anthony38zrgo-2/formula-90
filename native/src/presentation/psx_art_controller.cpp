#include "formula90s/presentation/psx_art_controller.hpp"

#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/classes/file_access.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/viewport.hpp>
#include <godot_cpp/classes/control.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <godot_cpp/variant/color.hpp>
#include <algorithm>
#include <utility>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace godot {

void PsxArtController::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_preset_path", "path"), &PsxArtController::set_preset_path);
	ClassDB::bind_method(D_METHOD("get_preset_path"), &PsxArtController::get_preset_path);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "preset_path", PROPERTY_HINT_FILE, "*.json"), "set_preset_path", "get_preset_path");

	ClassDB::bind_method(D_METHOD("set_auto_apply", "auto"), &PsxArtController::set_auto_apply);
	ClassDB::bind_method(D_METHOD("get_auto_apply"), &PsxArtController::get_auto_apply);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "auto_apply"), "set_auto_apply", "get_auto_apply");

	ClassDB::bind_method(D_METHOD("set_target_viewport_path", "path"), &PsxArtController::set_target_viewport_path);
	ClassDB::bind_method(D_METHOD("get_target_viewport_path"), &PsxArtController::get_target_viewport_path);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "target_viewport_path"), "set_target_viewport_path", "get_target_viewport_path");

	ClassDB::bind_method(D_METHOD("set_target_presenter_path", "path"), &PsxArtController::set_target_presenter_path);
	ClassDB::bind_method(D_METHOD("get_target_presenter_path"), &PsxArtController::get_target_presenter_path);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "target_presenter_path"), "set_target_presenter_path", "get_target_presenter_path");

	ClassDB::bind_method(D_METHOD("set_override_engine_fps", "override"), &PsxArtController::set_override_engine_fps);
	ClassDB::bind_method(D_METHOD("get_override_engine_fps"), &PsxArtController::get_override_engine_fps);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "override_engine_fps"), "set_override_engine_fps", "get_override_engine_fps");

	ClassDB::bind_method(D_METHOD("load_preset", "path"), &PsxArtController::load_preset);
	ClassDB::bind_method(D_METHOD("apply_preset"), &PsxArtController::apply_preset);

	ClassDB::bind_method(D_METHOD("get_current_profile_name"), &PsxArtController::get_current_profile_name);
	ClassDB::bind_method(D_METHOD("get_internal_width"), &PsxArtController::get_internal_width);
	ClassDB::bind_method(D_METHOD("get_internal_height"), &PsxArtController::get_internal_height);
	ClassDB::bind_method(D_METHOD("get_upscale_mode"), &PsxArtController::get_upscale_mode);
	ClassDB::bind_method(D_METHOD("get_bit_depth"), &PsxArtController::get_bit_depth);
	ClassDB::bind_method(D_METHOD("get_vertex_snap_distance"), &PsxArtController::get_vertex_snap_distance);
	ClassDB::bind_method(D_METHOD("get_affine_texture_strength"), &PsxArtController::get_affine_texture_strength);
	ClassDB::bind_method(D_METHOD("is_preset_loaded"), &PsxArtController::is_preset_loaded);

	ADD_SIGNAL(MethodInfo("preset_applied", PropertyInfo(Variant::STRING, "profile_name")));
}

PsxArtController::PsxArtController() {
}

PsxArtController::~PsxArtController() {
	unload_dll();
}

void PsxArtController::_ready() {
	if (auto_apply_ && !preset_path_.is_empty()) {
		load_preset(preset_path_);
	}
}

void PsxArtController::set_preset_path(const String &p_path) {
	preset_path_ = p_path;
}

String PsxArtController::get_preset_path() const {
	return preset_path_;
}

void PsxArtController::set_auto_apply(bool p_auto) {
	auto_apply_ = p_auto;
}

bool PsxArtController::get_auto_apply() const {
	return auto_apply_;
}

void PsxArtController::set_target_viewport_path(const NodePath &p_path) {
	target_viewport_path_ = p_path;
}

NodePath PsxArtController::get_target_viewport_path() const {
	return target_viewport_path_;
}

void PsxArtController::set_target_presenter_path(const NodePath &p_path) {
	target_presenter_path_ = p_path;
}

NodePath PsxArtController::get_target_presenter_path() const {
	return target_presenter_path_;
}

void PsxArtController::set_override_engine_fps(bool p_override) {
	override_engine_fps_ = p_override;
}

bool PsxArtController::get_override_engine_fps() const {
	return override_engine_fps_;
}

bool PsxArtController::load_dll() {
	if (dll_handle_ != nullptr) {
		return true;
	}

#ifdef _WIN32
	Array candidates;
	candidates.append("res://addons/formula90s/bin/psx_art_plugin.windows.template_release.x86_64.dll");
	candidates.append("res://addons/formula90s/bin/psx_art_plugin.windows.template_debug.x86_64.dll");
	candidates.append("res://addons/formula90s/bin/psx_art_plugin.dll");
	candidates.append("game/addons/formula90s/bin/psx_art_plugin.windows.template_release.x86_64.dll");
	candidates.append("game/addons/formula90s/bin/psx_art_plugin.windows.template_debug.x86_64.dll");
	candidates.append("game/addons/formula90s/bin/psx_art_plugin.dll");
	candidates.append("game/crates/target/release/psx_art_plugin.dll");
	candidates.append("game/crates/target/debug/psx_art_plugin.dll");

	HMODULE hDll = nullptr;
	String loaded_path = "";
	ProjectSettings *ps = ProjectSettings::get_singleton();

	for (int i = 0; i < candidates.size(); ++i) {
		String res_path = candidates[i];
		String global_path = ps ? ps->globalize_path(res_path) : res_path;
		CharString cs = global_path.utf8();
		hDll = LoadLibraryA(cs.get_data());
		if (hDll) {
			loaded_path = std::move(global_path);
			break;
		}
	}

	if (!hDll) {
		UtilityFunctions::printerr("[PsxArtController] Could not find or load psx_art_plugin.dll!");
		return false;
	}

	dll_handle_ = (void *)hDll;
	fn_parse_preset_ = (FnPsxArtParsePreset)GetProcAddress(hDll, "psx_art_parse_preset");
	fn_get_dither_ = (FnPsxArtGetDitherMatrix)GetProcAddress(hDll, "psx_art_get_dither_matrix");
	fn_version_ = (FnPsxArtVersion)GetProcAddress(hDll, "psx_art_version");

	if (!fn_parse_preset_) {
		UtilityFunctions::printerr("[PsxArtController] Missing psx_art_parse_preset symbol in DLL!");
		unload_dll();
		return false;
	}

	return true;
#else
	UtilityFunctions::printerr("[PsxArtController] Only Windows runtime supported in this build.");
	return false;
#endif
}

void PsxArtController::unload_dll() {
#ifdef _WIN32
	if (dll_handle_) {
		FreeLibrary((HMODULE)dll_handle_);
		dll_handle_ = nullptr;
	}
#endif
	fn_parse_preset_ = nullptr;
	fn_get_dither_ = nullptr;
	fn_version_ = nullptr;
}

bool PsxArtController::load_preset(const String &p_path) {
	if (!load_dll()) {
		return false;
	}

	Ref<FileAccess> file = FileAccess::open(p_path, FileAccess::READ);
	if (file.is_null()) {
		UtilityFunctions::printerr("[PsxArtController] Failed to open preset JSON at: " + p_path);
		return false;
	}

	String json_content = file->get_as_text();
	CharString utf8_json = json_content.utf8();

	CPsxArtConfigNative config{};
	int32_t result = fn_parse_preset_(utf8_json.get_data(), &config);
	if (result != 0) {
		UtilityFunctions::printerr("[PsxArtController] Failed to parse/validate preset JSON (code: " + String::num_int64(result) + ")");
		return false;
	}

	current_config_ = config;
	preset_path_ = p_path;
	current_profile_name_ = p_path.get_file().get_basename();
	is_loaded_ = true;

	return apply_preset();
}

bool PsxArtController::apply_preset() {
	if (!is_loaded_) {
		return false;
	}

	Object *rs = Engine::get_singleton()->get_singleton("RenderingServer");
	if (rs) {
		int safe_bit_depth = std::clamp((int)current_config_.bit_depth, 1, 8);
		float safe_snap = std::max(current_config_.vertex_snap_distance, 0.0F);
		float safe_affine = std::clamp(current_config_.affine_texture_strength, 0.0F, 1.0F);

		// 1. Maintain 100% backward compatibility with existing shaders & global uniforms
		rs->call("global_shader_parameter_set", "psx_bit_depth", safe_bit_depth);
		rs->call("global_shader_parameter_set", "psx_snap_distance", safe_snap);
		rs->call("global_shader_parameter_set", "psx_affine_strength", safe_affine);

		Color fog_col(
			std::clamp(current_config_.fog_color[0], 0.0F, 1.0F),
			std::clamp(current_config_.fog_color[1], 0.0F, 1.0F),
			std::clamp(current_config_.fog_color[2], 0.0F, 1.0F),
			std::clamp(current_config_.fog_color[3], 0.0F, 1.0F)
		);
		rs->call("global_shader_parameter_set", "psx_fog_color", fog_col);
		rs->call("global_shader_parameter_set", "psx_fog_near", std::max(current_config_.fog_near, 0.0F));
		rs->call("global_shader_parameter_set", "psx_fog_far", std::max(current_config_.fog_far, 0.1F));
	}

	// 2. Adjust Viewport resolution
	if (!target_viewport_path_.is_empty()) {
		Node *vp_node = get_node_or_null(target_viewport_path_);
		if (vp_node) {
			if (current_config_.upscale_mode == 3) {
				// NativeHiresPsx mode: Viewport scales to native window
			} else {
				vp_node->set("size", Vector2i((int)current_config_.internal_width, (int)current_config_.internal_height));
			}
		}
	}

	// 3. Adjust TextureRect Presenter filter mode
	if (!target_presenter_path_.is_empty()) {
		Node *pres_node = get_node_or_null(target_presenter_path_);
		if (pres_node) {
			Control *ctrl = Object::cast_to<Control>(pres_node);
			if (ctrl) {
				if (current_config_.upscale_mode == 0) {
					// PixelPerfect raw nearest (0 = inherit, 1 = nearest)
					ctrl->set_texture_filter(Control::TEXTURE_FILTER_NEAREST);
				} else {
					// SharpBilinear / CrtSmooth / NativeHires (2 = linear)
					ctrl->set_texture_filter(Control::TEXTURE_FILTER_LINEAR);
				}
			}
		}
	}

	// 4. Adjust Engine Framerate Cap
	if (override_engine_fps_ && current_config_.fps_cap > 0) {
		Engine::get_singleton()->set_max_fps((int)current_config_.fps_cap);
	}

	UtilityFunctions::print("[PsxArtController] Applied preset: " + current_profile_name_ +
		" (" + String::num_int64(current_config_.internal_width) + "x" + String::num_int64(current_config_.internal_height) +
		", BitDepth=" + String::num_int64(current_config_.bit_depth) +
		", Snap=" + String::num(current_config_.vertex_snap_distance, 3) +
		", Affine=" + String::num(current_config_.affine_texture_strength, 2) + ")");

	emit_signal("preset_applied", current_profile_name_);
	return true;
}

} // namespace godot
