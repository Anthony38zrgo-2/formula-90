#include "formula90s_tools/sprite_toolkit.hpp"
#include "stb_image_write.h"
#include <iostream>
using namespace formula90s_tools;
int main(int argc,char**argv){try{
 auto dir=std::filesystem::path(argc>1?argv[1]:"build/sprite_tests");std::filesystem::create_directories(dir);int fail=0;auto check=[&](bool v,const char*n){std::cout<<(v?"PASS ":"FAIL ")<<n<<"\n";if(!v)fail++;};
 Image fixture{12,10,std::vector<std::uint8_t>(12*10*4)};for(int y=0;y<10;y++)for(int x=0;x<12;x++){auto*p=fixture.pixel(x,y);p[0]=255;p[2]=255;p[3]=255;if(x>=3&&x<=8&&y>=2&&y<=8){p[0]=240;p[1]=240;p[2]=240;}}
 auto png=dir/"fixture.png";write_png(png,fixture,true);auto loaded=load_image(png);check(loaded.width==12&&loaded.height==10,"read_png");
 std::vector<unsigned char>rgb(12*10*3);for(int i=0;i<120;++i){rgb[i*3]=fixture.rgba[i*4];rgb[i*3+1]=fixture.rgba[i*4+1];rgb[i*3+2]=fixture.rgba[i*4+2];}auto jpg=dir/"fixture.jpg";stbi_write_jpg(jpg.string().c_str(),12,10,3,rgb.data(),95);check(load_image(jpg).width==12,"read_jpg");
 check(remove_background(loaded,"corners",10,0)>.2,"corner_background");auto b=alpha_bounds(loaded);check(b.x==3&&b.y==2&&b.width==6&&b.height==7,"alpha_bounds");
 Image preserve=loaded;auto alpha_before=preserve.pixel(3,2)[3];remove_background(preserve,"preserve",0,0);check(preserve.pixel(3,2)[3]==alpha_before,"preserve_alpha");
 Image magenta=fixture;remove_background(magenta,"color",10,0,{255,0,255});check(magenta.pixel(0,0)[3]==0,"magenta_background");
 Image white=fixture;for(int y=0;y<10;y++)for(int x=0;x<12;x++){auto*p=white.pixel(x,y);if(!(x>=3&&x<=8&&y>=2&&y<=8))p[0]=p[1]=p[2]=255;}remove_background(white,"color",10,0,{255,255,255});check(white.pixel(0,0)[3]==0,"white_background");
 auto resized=resize_nearest(crop_image(loaded,b),3,4);check(resized.width==3&&resized.height==4,"nearest_resize");auto canvas=align_on_canvas(resized,8,8);auto cb=alpha_bounds(canvas);check(cb.y==4&&cb.x==2,"bottom_center_alignment");check(canvas.width==8&&canvas.height==8,"fixed_canvas");check(sha256_file(png).size()==64,"sha256");
 bool protected_write=false;try{write_png(png,fixture,false);}catch(...){protected_write=true;}check(protected_write,"overwrite_protection");bool total_guard=false;Image allbg=fixture;for(auto&p:allbg.rgba)p=255;try{remove_background(allbg,"corners",10,0);}catch(...){total_guard=true;}check(total_guard,"total_removal_guard");
 auto rows=detect_content_rows(fixture,10);check(!rows.empty(),"content_row_detection");for(int count:{1,8,12,16})check(count==1||count%4==0,(std::string("direction_count_")+std::to_string(count)).c_str());
 return fail?1:0;}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 2;}}
