#include "formula90s/register_types.hpp"
#include "formula90s/core/game_bootstrap.hpp"
#include "formula90s/core/reset_manager.hpp"
#include "formula90s/camera/arcade_chase_camera.hpp"
#include "formula90s/presentation/directional_vehicle_sprite.hpp"
#include "formula90s/presentation/directional_sprite_validation_controller.hpp"
#include "formula90s/presentation/vehicle_visual_3d_config.hpp"
#include "formula90s/presentation/vehicle_visual_3d_controller.hpp"
#include "formula90s/presentation/engine_audio_controller.hpp"
#include "formula90s/presentation/vehicle_audio_controller_native.hpp"
#include "formula90s/audio/engine_audio_config.hpp"
#include "formula90s/ui/main_menu_controller.hpp"
#include "formula90s/ui/debug_hud_controller.hpp"
#include "formula90s/ui/static_minimap_controller.hpp"
#include "formula90s/vehicle/f1_94_rust_vehicle.hpp"
#include "formula90s/sim/f90_sim_bridge.hpp"
#include "formula90s/core/f90_core.hpp"
#include <godot_cpp/core/defs.hpp>
#include <godot_cpp/godot.hpp>
using namespace godot;
void initialize_formula90s_module(ModuleInitializationLevel level) {
	if (level != MODULE_INITIALIZATION_LEVEL_SCENE) return;
	GDREGISTER_CLASS(GameBootstrap);
	GDREGISTER_CLASS(ResetManager);
	GDREGISTER_CLASS(ArcadeChaseCamera);
	GDREGISTER_CLASS(DirectionalVehicleSprite);
	GDREGISTER_CLASS(DirectionalSpriteValidationController);
	GDREGISTER_CLASS(VehicleVisual3DConfig);
	GDREGISTER_CLASS(VehicleVisual3DController);
	GDREGISTER_CLASS(EngineAudioConfig);
	GDREGISTER_CLASS(EngineAudioController);
	GDREGISTER_CLASS(VehicleAudioControllerNative);
	GDREGISTER_CLASS(MainMenuController);
	GDREGISTER_CLASS(DebugHudController);
	GDREGISTER_CLASS(StaticMinimapController);
	GDREGISTER_CLASS(F194RustVehicle);
	GDREGISTER_CLASS(F90SimBridge);
	GDREGISTER_CLASS(F90Core);
}
void uninitialize_formula90s_module(ModuleInitializationLevel level) {}
extern "C" {
GDExtensionBool GDE_EXPORT formula90s_library_init(GDExtensionInterfaceGetProcAddress address,
		GDExtensionClassLibraryPtr library, GDExtensionInitialization *initialization) {
	GDExtensionBinding::InitObject init(address, library, initialization);
	init.register_initializer(initialize_formula90s_module);
	init.register_terminator(uninitialize_formula90s_module);
	init.set_minimum_library_initialization_level(MODULE_INITIALIZATION_LEVEL_SCENE);
	return init.init();
}
}
