#include "formula90s/camera/arcade_chase_camera.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/camera3d.hpp>
#include <godot_cpp/core/math.hpp>
using namespace godot;
namespace {
double smoothing_alpha(double rate,double delta){return 1.0-Math::exp(-Math::max(rate,0.01)*delta);}
double outside_dead_zone(double value,double zone){const double magnitude=Math::abs(value);return magnitude<=zone?0.0:Math::sign(value)*(magnitude-zone);}
Vector3 clamp_length(const Vector3&value,double maximum){const double length=value.length();return length>maximum&&length>0.000001?value*(maximum/length):value;}
}
ArcadeChaseCamera::ArcadeChaseCamera(){set_process_priority(-10);}
void ArcadeChaseCamera::_bind_methods(){
#define CAMERA_PROP(name) ClassDB::bind_method(D_METHOD("set_" #name,"value"),&ArcadeChaseCamera::set_##name);ClassDB::bind_method(D_METHOD("get_" #name),&ArcadeChaseCamera::get_##name);ADD_PROPERTY(PropertyInfo(Variant::FLOAT,#name),"set_" #name,"get_" #name)
 CAMERA_PROP(distance);CAMERA_PROP(height);CAMERA_PROP(follow_damping);CAMERA_PROP(horizontal_smoothing);CAMERA_PROP(vertical_smoothing);CAMERA_PROP(look_ahead);CAMERA_PROP(horizontal_dead_zone);CAMERA_PROP(vertical_dead_zone);CAMERA_PROP(velocity_anticipation);CAMERA_PROP(inertia_strength);CAMERA_PROP(maximum_camera_offset);CAMERA_PROP(offset_smoothing);CAMERA_PROP(lateral_swing);CAMERA_PROP(turn_look_offset);CAMERA_PROP(base_fov);CAMERA_PROP(speed_fov_gain);CAMERA_PROP(heading_smoothing);CAMERA_PROP(maximum_follow_lag);
#undef CAMERA_PROP
}
void ArcadeChaseCamera::_ready(){ArcadeCarController*car=Object::cast_to<ArcadeCarController>(get_parent());if(car){const Transform3D pose=car->get_visual_transform();Vector3 forward=-pose.basis.get_column(2);forward.y=0;forward.normalize();smoothed_forward=forward;set_global_position(pose.origin-forward*distance+Vector3(0,height,0));smoothed_look_target=pose.origin+forward*look_ahead+Vector3(0,1,0);presentation_epoch=car->get_presentation_epoch();initialized=true;}}
void ArcadeChaseCamera::_process(double delta){
 ArcadeCarController*car=Object::cast_to<ArcadeCarController>(get_parent());Camera3D*camera=Object::cast_to<Camera3D>(get_node_or_null("Camera3D"));if(!car||!camera||delta<=0.0)return;
 const Transform3D visual_pose=car->get_visual_transform();Vector3 physical_forward=-visual_pose.basis.get_column(2);physical_forward.y=0;physical_forward.normalize();
 const bool discontinuity=presentation_epoch!=car->get_presentation_epoch();if(discontinuity||!initialized){presentation_epoch=car->get_presentation_epoch();smoothed_forward=physical_forward;smoothed_velocity_lead=Vector3();smoothed_inertia=Vector3();filtered_acceleration=Vector3();previous_inertia_source=Vector3();inertia_initialized=false;initialized=true;}
 Vector3 heading_blend=smoothed_forward.lerp(physical_forward,smoothing_alpha(heading_smoothing,delta));smoothed_forward=heading_blend.length_squared()>0.000001?heading_blend.normalized():physical_forward;Vector3 right(-smoothed_forward.z,0,smoothed_forward.x);
 const Vector3 car_position=visual_pose.origin;Vector3 horizontal_velocity=car->get_velocity();horizontal_velocity.y=0;Vector3 horizontal_acceleration=car->get_world_acceleration();horizontal_acceleration.y=0;
 const double dynamic_limit=Math::max(maximum_camera_offset,0.0);Vector3 lead_target=clamp_length(horizontal_velocity*Math::max(velocity_anticipation,0.0),dynamic_limit*0.35);smoothed_velocity_lead=smoothed_velocity_lead.lerp(lead_target,smoothing_alpha(offset_smoothing,delta));
 filtered_acceleration=filtered_acceleration.lerp(horizontal_acceleration,smoothing_alpha(10.0,delta));const double impulse_limit=Math::min(dynamic_limit,0.4);const Vector3 inertia_source=clamp_length(-filtered_acceleration*Math::max(inertia_strength,0.0),impulse_limit);if(inertia_initialized){const Vector3 impulse_delta=inertia_source-previous_inertia_source;if(impulse_delta.length()>0.01)smoothed_inertia=clamp_length(smoothed_inertia+impulse_delta,impulse_limit);}else inertia_initialized=true;previous_inertia_source=inertia_source;smoothed_inertia=smoothed_inertia.lerp(Vector3(),smoothing_alpha(offset_smoothing,delta));
 Vector3 dynamic_offset=clamp_length(smoothed_inertia,dynamic_limit);const double lateral_limit=Math::min(Math::max(lateral_swing,0.0),dynamic_limit);const double lateral=dynamic_offset.dot(right);dynamic_offset+=right*(Math::clamp(lateral,-lateral_limit,lateral_limit)-lateral);
 const Vector3 desired=car_position-smoothed_forward*distance+Vector3(0,height,0)+dynamic_offset;if(discontinuity)set_global_position(desired);const Vector3 error=desired-get_global_position();const Vector3 horizontal_error=right*outside_dead_zone(error.dot(right),horizontal_dead_zone)+smoothed_forward*outside_dead_zone(error.dot(smoothed_forward),horizontal_dead_zone);const Vector3 vertical_error=Vector3(0,outside_dead_zone(error.y,vertical_dead_zone),0);
 Vector3 next=get_global_position()+horizontal_error*smoothing_alpha(horizontal_smoothing,delta)+vertical_error*smoothing_alpha(vertical_smoothing,delta);Vector3 lag=next-desired;const double lag_limit=Math::max(maximum_follow_lag,0.0);if(lag_limit>0.0)lag=clamp_length(lag,lag_limit);else lag=Vector3();next=desired+lag;set_global_position(next);
 const Vector3 look_desired=car_position+smoothed_forward*look_ahead+smoothed_velocity_lead+right*(dynamic_offset.dot(right)*Math::clamp(turn_look_offset,0.0,1.0))+Vector3(0,1,0);if(discontinuity)smoothed_look_target=look_desired;else smoothed_look_target=smoothed_look_target.lerp(look_desired,smoothing_alpha(follow_damping,delta));look_at(smoothed_look_target,Vector3(0,1,0));
 const double fov_target=base_fov+speed_fov_gain*Math::clamp(car->get_speed_kph()/285.0,0.0,1.0);camera->set_fov(Math::lerp((double)camera->get_fov(),fov_target,smoothing_alpha(follow_damping,delta)));
}
