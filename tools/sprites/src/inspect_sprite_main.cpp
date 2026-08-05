#include "formula90s_tools/sprite_toolkit.hpp"
#include <iostream>
using namespace formula90s_tools;
int main(int argc,char**argv){try{auto a=parse_args(argc,argv);if(!a.count("input"))throw std::runtime_error("usage: inspect_sprite --input FILE");auto im=load_image(a["input"]);std::cout<<"format="<<std::filesystem::path(a["input"]).extension().string()<<" width="<<im.width<<" height="<<im.height<<" alpha="<<(alpha_bounds(im).valid()?"yes":"no")<<" sha256="<<sha256_file(a["input"])<<"\n";auto rows=detect_content_rows(im,a.count("tolerance")?std::stoi(a["tolerance"]):20);for(std::size_t i=0;i<rows.size();i++)std::cout<<"row["<<i<<"]="<<rows[i].x<<","<<rows[i].y<<","<<rows[i].width<<","<<rows[i].height<<"\n";return rows.empty()?3:0;}catch(const std::exception&e){std::cerr<<"ERROR: "<<e.what()<<"\n";return 2;}}

