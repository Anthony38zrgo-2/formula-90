#pragma once
#include <algorithm>
#include <cmath>
#include <numbers>
namespace formula90s::physics {
constexpr double WHEEL_RADIUS = 0.33;
constexpr double TWO_PI = 2.0 * std::numbers::pi_v<double>;
inline double steering_rate(double speed_ratio, double low, double high) { return std::lerp(low, high, std::clamp(speed_ratio, 0.0, 1.0)); }
inline double apply_drag(double speed, double drag, double rolling, double dt) { double result=speed-speed*std::abs(speed)*drag*dt; double step=rolling*dt; return std::abs(result)<=step?0.0:result-(result>0?step:-step); }
inline double clamp_speed(double speed,double forward_max,double reverse_max){return std::clamp(speed,-reverse_max,forward_max);}
inline bool is_stopped(double horizontal_speed_mps,double stopped_threshold_mps=0.02){return std::isfinite(horizontal_speed_mps)&&std::abs(horizontal_speed_mps)<=stopped_threshold_mps;}
inline bool can_change_direction(double horizontal_speed_mps,double stopped_time,double stopped_threshold_mps=0.02,double required_delay=0.18){return is_stopped(horizontal_speed_mps,stopped_threshold_mps)&&stopped_time>=required_delay;}
inline int manual_shift_up(int gear,int max_gear){return gear>=1?std::min(gear+1,max_gear):gear;}
inline int manual_shift_down(int gear){return gear>1?gear-1:gear;}
inline double apply_lateral_grip(double lateral_speed,double grip_rate,double dt){return lateral_speed*std::exp(-std::max(0.0,grip_rate)*std::max(0.0,dt));}
inline double engine_rpm(double speed_mps,double ratio,double final_drive,double idle,double maximum){double wheel=std::abs(speed_mps)*60.0/(TWO_PI*WHEEL_RADIUS);return std::clamp(wheel*ratio*final_drive,idle,maximum);}
inline int choose_automatic_gear(int gear,double rpm,double throttle,double up,double down,bool can_shift,int max_gear){if(!can_shift||gear<1)return gear;if(rpm>=up&&gear<max_gear)return gear+1;if(rpm<=down&&gear>1&&throttle<0.95)return gear-1;return gear;}
}
