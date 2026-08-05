#pragma once
#include <algorithm>
#include <cmath>
namespace formula90s::physics {
inline double steering_rate(double speed_ratio, double low, double high) { return std::lerp(low, high, std::clamp(speed_ratio, 0.0, 1.0)); }
inline double apply_drag(double speed, double drag, double rolling, double dt) { double result=speed-speed*std::abs(speed)*drag*dt; double step=rolling*dt; return std::abs(result)<=step?0.0:result-(result>0?step:-step); }
inline double clamp_speed(double speed,double forward_max,double reverse_max){return std::clamp(speed,-reverse_max,forward_max);}
inline double engine_rpm(double speed_mps,double ratio,double final_drive,double idle,double maximum){double wheel=std::abs(speed_mps)*60.0/(2.0*3.14159265358979323846*0.33);return std::clamp(wheel*ratio*final_drive,idle,maximum);}
inline int choose_automatic_gear(int gear,double rpm,double throttle,double up,double down,bool can_shift){if(!can_shift||gear<1)return gear;if(rpm>=up&&gear<6)return gear+1;if(rpm<=down&&gear>1&&throttle<0.95)return gear-1;return gear;}
}

