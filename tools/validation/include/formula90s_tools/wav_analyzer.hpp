#pragma once

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace formula90s::tools {
struct WavAnalysis {
    bool valid = false;
    std::uint32_t sample_rate = 0;
    std::uint16_t channels = 0, bits_per_sample = 0;
    std::uint64_t frame_count = 0;
    double duration = 0.0, peak = 0.0, peak_dbfs = -120.0, rms = 0.0, dc_offset = 0.0;
    double endpoint_difference = 0.0;
    bool clipping = false, has_nan = false, has_infinity = false, silence = true;
    std::vector<float> samples;
    std::string error;
};
WavAnalysis analyze_wav(const std::filesystem::path &path);
std::string wav_analysis_json(const WavAnalysis &analysis, const std::filesystem::path &path);
}
