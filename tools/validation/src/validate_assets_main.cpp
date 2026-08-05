#include "formula90s_tools/sprite_toolkit.hpp"
#include "formula90s_tools/wav_analyzer.hpp"
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <sstream>
int main(int argc, char **argv) {
    const std::filesystem::path root = argc > 1 ? argv[1] : ".";
    int errors = 0, warnings = 0;
    try {
        const auto sheet = formula90s_tools::load_image(root / "game/assets/sprites/vehicles/v10/v10_sheet.png");
        if (sheet.width != 4096 || sheet.height != 128 || sheet.rgba.empty()) { std::cerr << "ERROR sprite sheet canvas/alpha\n"; ++errors; }
        bool transparent = false, visible = false;
        for (std::size_t i = 3; i < sheet.rgba.size(); i += 4) { transparent = transparent || sheet.rgba[i] == 0; visible = visible || sheet.rgba[i] > 8; }
        if (!transparent || !visible) { std::cerr << "ERROR sprite sheet alpha content\n"; ++errors; }
    } catch (const std::exception &e) { std::cerr << "ERROR sprite sheet: " << e.what() << '\n'; ++errors; }
    std::ifstream sprite_metadata_file(root / "game/assets/sprites/vehicles/v10/frames/rear.json"); std::stringstream sprite_metadata; sprite_metadata << sprite_metadata_file.rdbuf();
    if (!sprite_metadata_file || sprite_metadata.str().find("7607fc1c5be1a63acb0d6345dd499001ae8294f9adce68f6ed7f2359b245cec1") == std::string::npos || sprite_metadata.str().find("\"anchor\": {\"x\": 0.5, \"y\": 1.0}") == std::string::npos) { std::cerr << "ERROR sprite metadata/hash/anchor\n"; ++errors; }
    const auto metadata_path = root / "game/assets/audio/engines/v10_prototype/generation_metadata.json";
    std::ifstream metadata_file(metadata_path); std::stringstream metadata; metadata << metadata_file.rdbuf();
    if (!metadata_file || metadata.str().find("\"sample_rate\": 44100") == std::string::npos) { std::cerr << "ERROR audio metadata\n"; ++errors; }
    constexpr std::array<const char *, 7> files{"engine_idle.wav","engine_low.wav","engine_mid.wav","engine_high.wav","engine_redline.wav","gear_up.wav","gear_down.wav"};
    for (const char *name : files) {
        const auto analysis = formula90s::tools::analyze_wav(root / "game/assets/audio/engines/v10_prototype" / name);
        if (!analysis.valid || analysis.channels != 1 || analysis.sample_rate != 44100 || analysis.clipping || analysis.silence || std::abs(analysis.dc_offset) > 0.01) {
            std::cerr << "ERROR invalid audio: " << name << '\n'; ++errors;
        }
        if (std::string(name).find("engine_") == 0 && analysis.endpoint_difference > 0.08) { std::cerr << "WARNING loop edge: " << name << '\n'; ++warnings; }
    }
    std::cout << "Validation: " << errors << " errors, " << warnings << " warnings\n";
    return errors ? 1 : 0;
}
