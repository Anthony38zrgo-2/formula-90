#include "formula90s_tools/wav_analyzer.hpp"
#include <iostream>
int main(int argc, char **argv) {
    if (argc != 2) { std::cerr << "Usage: analyze_audio <file.wav>\n"; return 2; }
    const auto result = formula90s::tools::analyze_wav(argv[1]);
    std::cout << formula90s::tools::wav_analysis_json(result, argv[1]);
    if (!result.valid) std::cerr << "ERROR: " << result.error << '\n';
    return result.valid ? 0 : 1;
}
