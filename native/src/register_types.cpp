#include "formula90s/register_types.hpp"
#include "formula90s/core/game_bootstrap.hpp"
#include "formula90s/camera/arcade_chase_camera.hpp"
#include "formula90s/ui/main_menu_controller.hpp"
#include "formula90s/vehicle/f1_94_rust_vehicle.hpp"
#include "formula90s/core/f90_core.hpp"
#include "formula90s/presentation/psx_art_controller.hpp"
#include <godot_cpp/core/defs.hpp>
#include <godot_cpp/godot.hpp>
using namespace godot;
void initialize_formula90s_module(ModuleInitializationLevel level) {
	if (level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
	GDREGISTER_CLASS(GameBootstrap);
	GDREGISTER_CLASS(ArcadeChaseCamera);
	GDREGISTER_CLASS(MainMenuController);
	GDREGISTER_CLASS(F194RustVehicle);
	GDREGISTER_CLASS(F90Core);
	GDREGISTER_CLASS(PsxArtController);
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
