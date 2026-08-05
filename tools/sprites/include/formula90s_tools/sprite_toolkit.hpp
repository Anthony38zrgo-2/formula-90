#pragma once
#include <cstdint>
#include <array>
#include <filesystem>
#include <map>
#include <string>
#include <vector>

namespace formula90s_tools {
struct Image {
    int width=0, height=0;
    std::vector<std::uint8_t> rgba;
    bool valid() const { return width>0 && height>0 && rgba.size()==static_cast<std::size_t>(width*height*4); }
    std::uint8_t *pixel(int x,int y){return rgba.data()+(y*width+x)*4;}
    const std::uint8_t *pixel(int x,int y)const{return rgba.data()+(y*width+x)*4;}
};
struct Rect { int x=0,y=0,width=0,height=0; bool valid()const{return width>0&&height>0;} };
struct PrepareOptions {
    std::filesystem::path input, output, metadata, diagnostic;
    std::string vehicle_id="v10", orientation="rear", background_mode="corners", palette_mode="full";
    int tolerance=20, edge_softness=0, padding=8, canvas_width=256, canvas_height=128;
    std::array<std::uint8_t,3> background_color{255,0,255};
    Rect source_crop{}; bool crop=true, force=false, dry_run=false;
};
struct PrepareResult { Image image; Rect source_bounds, content_bounds; std::string source_sha256; double removed_ratio=0; };
Image load_image(const std::filesystem::path &path);
void write_png(const std::filesystem::path &path,const Image &image,bool force);
std::string sha256_file(const std::filesystem::path &path);
std::array<std::uint8_t,3> parse_hex_color(const std::string &value);
double remove_background(Image &image,const std::string &mode,int tolerance,int edge_softness,const std::array<std::uint8_t,3> &manual={255,0,255});
Rect alpha_bounds(const Image &image,std::uint8_t threshold=8);
Image crop_image(const Image &image,const Rect &rect);
Image resize_nearest(const Image &image,int width,int height);
Image align_on_canvas(const Image &image,int width,int height,int offset_x=0,int offset_y=0);
PrepareResult prepare_sprite(const PrepareOptions &options);
std::map<std::string,std::string> parse_args(int argc,char **argv);
std::map<std::string,std::string> read_simple_config(const std::filesystem::path &path);
void write_prepare_metadata(const PrepareOptions &options,const PrepareResult &result);
std::vector<Rect> detect_content_rows(const Image &image,int tolerance=20);
int run_sprite_tests(const std::filesystem::path &temporary_directory);
}
