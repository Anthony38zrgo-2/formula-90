#pragma once
#include <godot_cpp/classes/node.hpp>
namespace godot { class EngineAudioController : public Node { GDCLASS(EngineAudioController, Node)
    double rpm=0, load=0;
protected: static void _bind_methods();
public: void set_engine_state(double p_rpm,double p_load){rpm=p_rpm;load=p_load;} void notify_gear_shift(int gear){emit_signal("gear_shift",gear);} }; }

