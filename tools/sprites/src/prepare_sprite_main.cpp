#include "formula90s_tools/sprite_toolkit.hpp"
#include <cstdio>
#include <iostream>
using namespace formula90s_tools;
int main(int argc,char**argv){try{
 auto cli=parse_args(argc,argv);std::map<std::string,std::string>a;if(cli.count("config"))a=read_simple_config(cli["config"]);for(const auto&e:cli)a[e.first]=e.second;
 if(!a.count("input")||!a.count("output"))throw std::runtime_error("usage: prepare_sprite --input FILE --output PNG [--config YAML] [--force] [--dry-run]");
 PrepareOptions o;o.input=a["input"];o.output=a["output"];o.metadata=a.count("metadata")?a["metadata"]:o.output.string()+".json";
 auto set_string=[&](const char*k,std::string&v){if(a.count(k))v=a[k];};set_string("orientation",o.orientation);set_string("vehicle_id",o.vehicle_id);set_string("vehicle-id",o.vehicle_id);set_string("background_mode",o.background_mode);set_string("background-mode",o.background_mode);set_string("palette_mode",o.palette_mode);set_string("palette-mode",o.palette_mode);
 auto set_int=[&](const char*k,int&v){if(a.count(k))v=std::stoi(a[k]);};set_int("tolerance",o.tolerance);set_int("edge_softness",o.edge_softness);set_int("edge-softness",o.edge_softness);set_int("padding",o.padding);set_int("canvas_width",o.canvas_width);set_int("canvas-width",o.canvas_width);set_int("canvas_height",o.canvas_height);set_int("canvas-height",o.canvas_height);
 if(a.count("background-color"))o.background_color=parse_hex_color(a["background-color"]);
 if(a.count("crop-rect")){int x,y,w,h;if(std::sscanf(a["crop-rect"].c_str(),"%d,%d,%d,%d",&x,&y,&w,&h)!=4)throw std::runtime_error("crop-rect must be x,y,w,h");o.source_crop={x,y,w,h};}
 o.force=cli.count("force");o.dry_run=cli.count("dry-run");auto r=prepare_sprite(o);if(o.dry_run){std::cout<<"DRY RUN valid output "<<r.image.width<<"x"<<r.image.height<<"\n";return 0;}write_png(o.output,r.image,o.force);write_prepare_metadata(o,r);std::cout<<"Prepared "<<o.output<<" sha256="<<r.source_sha256<<"\n";return 0;
 }catch(const std::exception&e){std::cerr<<"ERROR: "<<e.what()<<"\n";return 2;}}
