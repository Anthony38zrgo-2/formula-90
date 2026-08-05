#include "formula90s/vehicle/physics_math.hpp"
#include <cmath>
#include <iostream>
using namespace formula90s::physics;
static int failures=0;
static void check(bool value,const char *name){if(!value){std::cerr<<"FAIL "<<name<<"\n";++failures;}else std::cout<<"PASS "<<name<<"\n";}
int main(){
 check(choose_automatic_gear(1,8300,1,8200,4300,true)==2,"first_to_second");
 check(choose_automatic_gear(6,9000,1,8200,4300,true)==6,"sixth_limit");
 check(choose_automatic_gear(4,4000,.5,8200,4300,true)==3,"automatic_downshift");
 check(choose_automatic_gear(3,6000,.5,8200,4300,true)==3,"hysteresis_band");
 check(choose_automatic_gear(2,9000,1,8200,4300,false)==2,"minimum_shift_time");
 check(std::abs(engine_rpm(0,3.1,3.7,1100,9000)-1100)<.01,"idle_rpm");
 check(std::abs(engine_rpm(200,3.1,3.7,1100,9000)-9000)<.01,"rev_limiter");
 check(steering_rate(1,1.9,.55)<steering_rate(0,1.9,.55),"speed_dependent_steering");
 check(apply_drag(20,.006,1.4,.1)<20,"drag");
 check(clamp_speed(100,79.2,9.72)==79.2&&clamp_speed(-20,79.2,9.72)==-9.72,"speed_limits_and_reverse");
 check(std::isfinite(engine_rpm(10,0,0,1100,9000)),"invalid_numeric_guard");
 std::cout<<(failures?"UNIT TESTS FAILED":"UNIT TESTS PASSED")<<"\n";return failures?1:0;
}

