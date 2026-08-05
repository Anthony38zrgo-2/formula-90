#include "formula90s/presentation/engine_audio_controller.hpp"
using namespace godot;
void EngineAudioController::_bind_methods(){ClassDB::bind_method(D_METHOD("set_engine_state","rpm","load"),&EngineAudioController::set_engine_state);ClassDB::bind_method(D_METHOD("notify_gear_shift","gear"),&EngineAudioController::notify_gear_shift);ADD_SIGNAL(MethodInfo("gear_shift",PropertyInfo(Variant::INT,"gear")));}

