#include "formula90s_tools/wav_analyzer.hpp"
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iostream>
static void u16(std::ofstream &f, unsigned v){char b[2]{char(v),char(v>>8)};f.write(b,2);} static void u32(std::ofstream &f,unsigned v){char b[4]{char(v),char(v>>8),char(v>>16),char(v>>24)};f.write(b,4);}
int main(){const auto p=std::filesystem::temp_directory_path()/"formula90s_wav_test.wav";std::ofstream f(p,std::ios::binary);f.write("RIFF",4);u32(f,36+8820);f.write("WAVEfmt ",8);u32(f,16);u16(f,1);u16(f,1);u32(f,44100);u32(f,88200);u16(f,2);u16(f,16);f.write("data",4);u32(f,8820);for(int i=0;i<4410;++i)u16(f,unsigned(short(std::sin(i*.1)*16000)));f.close();auto a=formula90s::tools::analyze_wav(p);std::filesystem::remove(p);bool ok=a.valid&&a.sample_rate==44100&&a.channels==1&&std::abs(a.duration-.1)<.001&&!a.clipping&&!a.silence;std::cout<<(ok?"PASS":"FAIL")<<" wav_analysis\n";return ok?0:1;}
