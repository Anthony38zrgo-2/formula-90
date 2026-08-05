#include "formula90s/vehicle/physics_math.hpp"
#include "formula90s/audio/engine_dsp.hpp"
#include "formula90s/presentation/directional_sprite_math.hpp"
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
 check(!is_stopped(0.5)&&!is_stopped(0.021)&&is_stopped(0.02)&&is_stopped(0.0),"strict_stopped_threshold");
 check(!can_change_direction(0.0,0.17,0.02,0.18)&&!can_change_direction(0.03,1.0,0.02,0.18)&&can_change_direction(0.0,0.18,0.02,0.18),"direction_change_requires_stable_stop");
 check(manual_shift_up(1)==2&&manual_shift_up(6)==6&&manual_shift_down(6)==5&&manual_shift_down(1)==1,"manual_gear_limits");
 check(apply_lateral_grip(5.0,9.0,1.0/60.0)<5.0&&apply_lateral_grip(5.0,9.0,1.0/60.0)>0.0,"stable_lateral_grip");
 check(std::isfinite(engine_rpm(10,0,0,1100,9000)),"invalid_numeric_guard");
 check(std::abs(formula90s::presentation::clockwise_view_angle(0.0,1.0,0.0,1.0)-0.0)<.01,"sprite_rear_angle");
 check(std::abs(formula90s::presentation::clockwise_view_angle(-0.382683,0.923880,0.0,1.0)-337.5)<.01,"sprite_right_turn_uses_clockwise_frame");
 check(std::abs(formula90s::presentation::clockwise_view_angle(0.382683,0.923880,0.0,1.0)-22.5)<.01,"sprite_left_turn_uses_counterpart_frame");
 check(std::abs(formula90s::presentation::circular_angle_distance(359.0,0.0)-1.0)<.01,"sprite_wraparound_359_to_zero");
 check(!formula90s::presentation::should_switch_direction(12.0,10.0,3.0),"sprite_hysteresis_holds_boundary");
 check(formula90s::presentation::should_switch_direction(14.0,10.0,3.0),"sprite_hysteresis_releases_after_margin");
 using namespace formula90s::audio;
 const auto idle_weights=EngineLayerMixer::weights(0.0), mid_weights=EngineLayerMixer::weights(0.5), blend_weights=EngineLayerMixer::weights(0.625);
 check(idle_weights[0]==1.0F&&mid_weights[2]==1.0F&&blend_weights[2]>0&&blend_weights[3]>0,"dsp_layer_crossfade");
 float weight_sum=0;for(float weight:blend_weights)weight_sum+=weight;check(std::abs(weight_sum-1.0F)<.0001F,"dsp_gain_sum");
 EngineDspState dsp_state;dsp_state.normalized_rpm=2.0;check(EnginePitchProcessor::ratio(dsp_state,.7F,1.4F)==1.4F,"dsp_pitch_limits");
 dsp_state.normalized_rpm=.5;dsp_state.reverse=true;check(EnginePitchProcessor::ratio(dsp_state,.7F,1.4F)<1.05F,"dsp_reverse_pitch");
 EngineFilterProcessor filter;filter.set_sample_rate(44100);const float first=filter.process(1,.5F),second=filter.process(0,.5F);check(first>0&&second>0&&filter.state()>0,"dsp_filter_persistent_state");
 filter.set_sample_rate(48000);check(std::isfinite(filter.process(1,1)),"dsp_sample_rate_change");
 check(std::abs(EngineSaturator::process(2,.3F))<=1.01F,"dsp_saturation");
 check(EngineLimiter::process(2,.92F)==.92F&&EngineLimiter::process(NAN,.92F)==0,"dsp_limiter_and_nan");
 EngineDspChain chain;chain.set_sample_rate(44100);float peak=0;for(int i=0;i<512;++i)peak=std::max(peak,std::abs(chain.process(1.8F,dsp_state,.12F,.92F)));check(peak<=.92F&&std::isfinite(peak),"dsp_sum_without_clipping");
 std::cout<<(failures?"UNIT TESTS FAILED":"UNIT TESTS PASSED")<<"\n";return failures?1:0;
}
