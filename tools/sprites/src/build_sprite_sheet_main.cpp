#include "formula90s_tools/sprite_toolkit.hpp"
#include <cstring>
#include <fstream>
#include <iostream>
#include <set>
#include <sstream>
using namespace formula90s_tools;
int main(int argc,char**argv){try{
 auto a=parse_args(argc,argv);if(!a.count("input-dir")||!a.count("output")||!a.count("metadata")||!a.count("config"))throw std::runtime_error("usage: build_sprite_sheet --input-dir DIR --output PNG --metadata JSON --config YAML");
 auto cfg=read_simple_config(a["config"]);if(!cfg.count("frame_order"))throw std::runtime_error("config requires frame_order");
 std::vector<std::string>requested;std::stringstream ss(cfg["frame_order"]);std::string name;while(std::getline(ss,name,',')){name.erase(0,name.find_first_not_of(" \t"));name.erase(name.find_last_not_of(" \t")+1);requested.push_back(name);}
 std::vector<Image>frames;std::vector<std::string>names;std::set<std::string>hashes;
 for(const auto&candidate:requested){auto path=std::filesystem::path(a["input-dir"])/(candidate+".png");if(!std::filesystem::exists(path)){if(cfg["missing_frame_policy"]=="error")throw std::runtime_error("missing frame: "+candidate);std::cerr<<"WARNING: missing frame "<<candidate<<"\n";continue;}auto hash=sha256_file(path);if(!hashes.insert(hash).second)std::cerr<<"WARNING: duplicate frame "<<candidate<<"\n";frames.push_back(load_image(path));names.push_back(candidate);}
 if(frames.empty())throw std::runtime_error("no frames available");int width=frames[0].width,height=frames[0].height;for(const auto&frame:frames)if(frame.width!=width||frame.height!=height)throw std::runtime_error("inconsistent frame dimensions");
 Image sheet{width*int(frames.size()),height,std::vector<std::uint8_t>(std::size_t(width)*frames.size()*height*4)};for(std::size_t i=0;i<frames.size();++i)for(int y=0;y<height;++y)std::memcpy(sheet.pixel(int(i)*width,y),frames[i].pixel(0,y),std::size_t(width)*4);
 if(a.count("dry-run")){std::cout<<"DRY RUN frames="<<frames.size()<<" sheet="<<sheet.width<<"x"<<sheet.height<<"\n";return 0;}const bool force=a.count("force");write_png(a["output"],sheet,force);if(std::filesystem::exists(a["metadata"])&&!force)throw std::runtime_error("metadata exists; pass --force");std::filesystem::create_directories(std::filesystem::path(a["metadata"]).parent_path());
 std::ofstream out(a["metadata"]);const bool mirror=cfg["allow_horizontal_mirror"]=="true";out<<"{\n  \"schema_version\": 1,\n  \"vehicle_id\": \"v10\",\n  \"direction_count\": "<<cfg["required_directions"]<<",\n  \"allow_horizontal_mirror\": "<<(mirror?"true":"false")<<",\n  \"missing_frame_policy\": \""<<cfg["missing_frame_policy"]<<"\",\n  \"frame_width\": "<<width<<", \"frame_height\": "<<height<<",\n  \"sheet_width\": "<<sheet.width<<", \"sheet_height\": "<<sheet.height<<",\n  \"frames\": [\n";
 for(std::size_t i=0;i<frames.size();++i){double angle=std::stod(cfg["angle_"+names[i]]);out<<"    {\"name\": \""<<names[i]<<"\", \"index\": "<<i<<", \"angle_degrees\": "<<angle<<", \"region\": {\"x\": "<<i*width<<", \"y\": 0, \"width\": "<<width<<", \"height\": "<<height<<"}, \"anchor\": {\"x\": 0.5, \"y\": 1.0}}"<<(i+1<frames.size()?",":"")<<"\n";}out<<"  ]\n}\n";std::cout<<"Built sheet "<<a["output"]<<" frames="<<frames.size()<<"\n";return 0;
 }catch(const std::exception&e){std::cerr<<"ERROR: "<<e.what()<<"\n";return 2;}}
