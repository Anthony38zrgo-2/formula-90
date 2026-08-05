#include "formula90s_tools/wav_analyzer.hpp"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <sstream>

namespace formula90s::tools {
namespace {
std::uint16_t u16(const unsigned char *p) { return std::uint16_t(p[0]) | (std::uint16_t(p[1]) << 8U); }
std::uint32_t u32(const unsigned char *p) { return std::uint32_t(p[0]) | (std::uint32_t(p[1]) << 8U) | (std::uint32_t(p[2]) << 16U) | (std::uint32_t(p[3]) << 24U); }
}
WavAnalysis analyze_wav(const std::filesystem::path &path) {
    WavAnalysis result;
    std::ifstream file(path, std::ios::binary);
    if (!file) { result.error = "cannot open WAV"; return result; }
    std::vector<unsigned char> bytes((std::istreambuf_iterator<char>(file)), {});
    if (bytes.size() < 44 || std::memcmp(bytes.data(), "RIFF", 4) || std::memcmp(bytes.data() + 8, "WAVE", 4)) { result.error = "invalid RIFF/WAVE header"; return result; }
    std::size_t cursor = 12, data_offset = 0, data_size = 0;
    std::uint16_t format = 0;
    while (cursor + 8 <= bytes.size()) {
        const std::uint32_t size = u32(bytes.data() + cursor + 4);
        const std::size_t payload = cursor + 8;
        if (payload + size > bytes.size()) { result.error = "truncated WAV chunk"; return result; }
        if (!std::memcmp(bytes.data() + cursor, "fmt ", 4) && size >= 16) {
            format = u16(bytes.data() + payload); result.channels = u16(bytes.data() + payload + 2);
            result.sample_rate = u32(bytes.data() + payload + 4); result.bits_per_sample = u16(bytes.data() + payload + 14);
        } else if (!std::memcmp(bytes.data() + cursor, "data", 4)) { data_offset = payload; data_size = size; }
        cursor = payload + size + (size & 1U);
    }
    if (format != 1 || result.bits_per_sample != 16 || result.channels == 0 || result.sample_rate == 0 || data_size == 0) { result.error = "only non-empty PCM16 WAV is supported"; return result; }
    const std::size_t count = data_size / 2;
    result.samples.reserve(count);
    double sum = 0.0, squares = 0.0;
    for (std::size_t i = 0; i < count; ++i) {
        const auto bits = u16(bytes.data() + data_offset + i * 2);
        const float sample = static_cast<float>(static_cast<std::int16_t>(bits)) / 32768.0F;
        result.samples.push_back(sample); sum += sample; squares += double(sample) * sample;
        result.peak = std::max(result.peak, std::abs(double(sample)));
    }
    result.frame_count = count / result.channels;
    result.duration = double(result.frame_count) / result.sample_rate;
    result.dc_offset = sum / count; result.rms = std::sqrt(squares / count);
    result.peak_dbfs = result.peak > 0.0 ? 20.0 * std::log10(result.peak) : -120.0;
    result.clipping = result.peak >= 0.9999; result.silence = result.rms < 1e-5;
    result.endpoint_difference = std::abs(result.samples.front() - result.samples[result.samples.size() - result.channels]);
    result.valid = true; return result;
}
std::string wav_analysis_json(const WavAnalysis &a, const std::filesystem::path &path) {
    std::ostringstream out; out << std::fixed << std::setprecision(8);
    out << "{\n  \"path\": \"" << path.generic_string() << "\",\n  \"valid\": " << (a.valid ? "true" : "false")
        << ",\n  \"duration\": " << a.duration << ",\n  \"sample_rate\": " << a.sample_rate << ",\n  \"channels\": " << a.channels
        << ",\n  \"peak\": " << a.peak << ",\n  \"peak_dbfs\": " << a.peak_dbfs << ",\n  \"rms\": " << a.rms
        << ",\n  \"dc_offset\": " << a.dc_offset << ",\n  \"clipping\": " << (a.clipping ? "true" : "false")
        << ",\n  \"nan\": " << (a.has_nan ? "true" : "false") << ",\n  \"infinity\": " << (a.has_infinity ? "true" : "false")
        << ",\n  \"silence\": " << (a.silence ? "true" : "false") << ",\n  \"endpoint_difference\": " << a.endpoint_difference
        << ",\n  \"possible_loop_discontinuity\": " << (a.endpoint_difference > 0.08 ? "true" : "false") << "\n}\n";
    return out.str();
}
}
