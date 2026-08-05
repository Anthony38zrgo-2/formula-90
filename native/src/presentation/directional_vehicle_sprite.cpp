#include "formula90s/presentation/directional_vehicle_sprite.hpp"
#include "formula90s/presentation/directional_sprite_math.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/camera3d.hpp>
#include <godot_cpp/classes/file_access.hpp>
#include <godot_cpp/classes/json.hpp>
#include <godot_cpp/classes/viewport.hpp>
#include <godot_cpp/core/math.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
using namespace godot;
namespace {double circular_distance(double a,double b){return Math::abs(Math::fmod(a-b+540.0,360.0)-180.0);}}
DirectionalVehicleSprite::DirectionalVehicleSprite(){set_billboard_mode(BaseMaterial3D::BILLBOARD_ENABLED);set_texture_filter(BaseMaterial3D::TEXTURE_FILTER_NEAREST);set_centered(true);set_offset(Vector2());set_process_priority(10);}
void DirectionalVehicleSprite::_bind_methods(){
 ClassDB::bind_method(D_METHOD("set_orientation_count","value"),&DirectionalVehicleSprite::set_orientation_count);ClassDB::bind_method(D_METHOD("get_orientation_count"),&DirectionalVehicleSprite::get_orientation_count);
 ClassDB::bind_method(D_METHOD("set_allow_mirroring","value"),&DirectionalVehicleSprite::set_allow_mirroring);ClassDB::bind_method(D_METHOD("get_allow_mirroring"),&DirectionalVehicleSprite::get_allow_mirroring);
 ClassDB::bind_method(D_METHOD("set_ground_offset","value"),&DirectionalVehicleSprite::set_ground_offset);ClassDB::bind_method(D_METHOD("get_ground_offset"),&DirectionalVehicleSprite::get_ground_offset);
 ClassDB::bind_method(D_METHOD("set_metadata_path","value"),&DirectionalVehicleSprite::set_metadata_path);ClassDB::bind_method(D_METHOD("get_metadata_path"),&DirectionalVehicleSprite::get_metadata_path);
 ClassDB::bind_method(D_METHOD("set_angular_offset","value"),&DirectionalVehicleSprite::set_angular_offset);ClassDB::bind_method(D_METHOD("get_angular_offset"),&DirectionalVehicleSprite::get_angular_offset);
 ClassDB::bind_method(D_METHOD("set_angular_hysteresis","value"),&DirectionalVehicleSprite::set_angular_hysteresis);ClassDB::bind_method(D_METHOD("get_angular_hysteresis"),&DirectionalVehicleSprite::get_angular_hysteresis);
 ClassDB::bind_method(D_METHOD("set_minimum_visual_speed","value"),&DirectionalVehicleSprite::set_minimum_visual_speed);ClassDB::bind_method(D_METHOD("get_minimum_visual_speed"),&DirectionalVehicleSprite::get_minimum_visual_speed);
 ClassDB::bind_method(D_METHOD("set_velocity_direction_influence","value"),&DirectionalVehicleSprite::set_velocity_direction_influence);ClassDB::bind_method(D_METHOD("get_velocity_direction_influence"),&DirectionalVehicleSprite::get_velocity_direction_influence);
 ADD_PROPERTY(PropertyInfo(Variant::INT,"orientation_count",PROPERTY_HINT_ENUM,"1,8,12,16"),"set_orientation_count","get_orientation_count");ADD_PROPERTY(PropertyInfo(Variant::BOOL,"allow_mirroring"),"set_allow_mirroring","get_allow_mirroring");ADD_PROPERTY(PropertyInfo(Variant::FLOAT,"ground_offset"),"set_ground_offset","get_ground_offset");
 ADD_PROPERTY(PropertyInfo(Variant::STRING,"metadata_path",PROPERTY_HINT_FILE,"*.json"),"set_metadata_path","get_metadata_path");ADD_PROPERTY(PropertyInfo(Variant::FLOAT,"first_frame_angular_offset",PROPERTY_HINT_RANGE,"-180,180,0.1"),"set_angular_offset","get_angular_offset");
 ADD_PROPERTY(PropertyInfo(Variant::FLOAT,"angular_hysteresis",PROPERTY_HINT_RANGE,"0,15,0.1"),"set_angular_hysteresis","get_angular_hysteresis");ADD_PROPERTY(PropertyInfo(Variant::FLOAT,"minimum_visual_speed",PROPERTY_HINT_RANGE,"0,10,0.1"),"set_minimum_visual_speed","get_minimum_visual_speed");ADD_PROPERTY(PropertyInfo(Variant::FLOAT,"velocity_direction_influence",PROPERTY_HINT_RANGE,"0,0.35,0.01"),"set_velocity_direction_influence","get_velocity_direction_influence");
}
void DirectionalVehicleSprite::_ready(){metadata_loaded=load_metadata();if(!metadata_loaded){frame_angles.clear();orientation_count=Math::max(1,orientation_count);for(int i=0;i<orientation_count;++i)frame_angles.push_back(360.0*i/orientation_count);set_hframes(orientation_count);set_vframes(1);set_frame(0);selected_frame=0;metadata_loaded=true;}}
bool DirectionalVehicleSprite::load_metadata(){
 frame_angles.clear();if(metadata_path.is_empty())return false;Ref<FileAccess>file=FileAccess::open(metadata_path,FileAccess::READ);if(file.is_null()){UtilityFunctions::printerr("[formula90s] sprite metadata missing: ",metadata_path);return false;}Variant parsed=JSON::parse_string(file->get_as_text());if(parsed.get_type()!=Variant::DICTIONARY){UtilityFunctions::printerr("[formula90s] invalid sprite metadata JSON");return false;}Dictionary root=parsed;allow_mirroring=bool(root.get("allow_horizontal_mirror",allow_mirroring));const int frame_height=int(root.get("frame_height",128));Array frames=root.get("frames",Array());const int frame_count=int(frames.size());for(int i=0;i<frame_count;++i){Dictionary frame=frames[i];const double fallback_angle=360.0*i/(frame_count>0?frame_count:1);frame_angles.push_back(float(frame.get("angle_degrees",fallback_angle)));}if(frame_angles.is_empty())return false;orientation_count=frame_angles.size();set_hframes(orientation_count);set_vframes(1);selected_frame=0;set_frame(selected_frame);set_offset(Vector2());Vector3 local_position=get_position();local_position.y=ground_offset+double(frame_height)*get_pixel_size()*0.5;set_position(local_position);return true;
}
int DirectionalVehicleSprite::find_nearest_frame(double angle)const{double best=1e9;int result=0;for(int i=0;i<frame_angles.size();++i){const double distance=circular_distance(angle,frame_angles[i]);if(distance<best){best=distance;result=i;}}return result;}
void DirectionalVehicleSprite::_process(double){
 if(!metadata_loaded||!is_inside_tree())return;Camera3D*camera=get_viewport()->get_camera_3d();ArcadeCarController*car=Object::cast_to<ArcadeCarController>(get_parent());if(!camera||!car)return;
 Vector3 velocity=car->get_velocity();velocity.y=0;const double speed=velocity.length();if(speed<minimum_visual_speed)return;
 Vector3 forward=-car->get_global_basis().get_column(2);forward.y=0;forward.normalize();Vector3 rear=-forward;
 if(speed>0.0001&&velocity_direction_influence>0.0){Vector3 motion_forward=velocity/speed;if(motion_forward.dot(forward)<0.0)motion_forward=-motion_forward;const double influence=Math::clamp(velocity_direction_influence*Math::clamp((speed-minimum_visual_speed)/10.0,0.0,1.0),0.0,0.35);rear=rear.lerp(-motion_forward,influence).normalized();}
 Vector3 to_camera=camera->get_global_position()-car->get_global_position();to_camera.y=0;if(to_camera.length_squared()<0.0001)return;to_camera.normalize();double angle=formula90s::presentation::clockwise_view_angle(rear.x,rear.z,to_camera.x,to_camera.z)+angular_offset;angle=Math::fmod(angle+360.0,360.0);
 if(allow_mirroring&&angle>180.0){angle=360.0-angle;set_flip_h(true);}else set_flip_h(false);const int candidate=find_nearest_frame(angle);if(candidate!=selected_frame){const double current_distance=circular_distance(angle,frame_angles[selected_frame]);const double candidate_distance=circular_distance(angle,frame_angles[candidate]);if(candidate_distance+Math::max(angular_hysteresis,0.0)<current_distance)selected_frame=candidate;}set_frame(selected_frame);
}
