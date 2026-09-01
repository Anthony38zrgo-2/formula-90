#include "f90_audio_dsp.h"

#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>

static int g_failures = 0;

static void check(bool cond, const char* name) {
    if (cond) {
        std::printf("PASS %s\n", name);
    } else {
        std::printf("FAIL %s\n", name);
        g_failures += 1;
    }
}

static f90_dsp_config make_config() {
    f90_dsp_config cfg{};
    cfg.struct_size = sizeof(f90_dsp_config);
    cfg.sample_rate = 44100;
    cfg.channels = 2;
    cfg.simulated_cylinders = 5;
    cfg.bank_count = 2;
    cfg.flags = 0;
    cfg.half_block_offset_deg = 72.0f;
    cfg.idle_rpm = 1000.0f;
    cfg.max_rpm = 15000.0f;
    return cfg;
}

static f90_dsp_controls make_controls() {
    f90_dsp_controls ctl{};
    ctl.rpm = 12000.0f;
    ctl.throttle = 0.8f;
    ctl.load = 0.7f;
    ctl.tc_cut = 0.4f;
    ctl.master_gain = 1.0f;
    ctl.lod = 0;
    ctl.bypass = 0;
    return ctl;
}

static f90_dsp_event make_event(uint32_t offset, uint8_t cylinder, float pressure) {
    f90_dsp_event e{};
    e.sample_offset = offset;
    e.cylinder = cylinder;
    e.bank = 0;
    e.crank_phase_deg = 0.0f;
    e.pressure = pressure;
    // ABI v2 is physical: pressure_derivative is pressure-units/second.
    // Use the same order of magnitude as the real V10 runtime instead of tiny
    // normalized values that would hide unit-contract regressions.
    e.pressure_derivative = pressure * 30000.0f;
    e.energy = 1.0f;
    e.cycle_variation = 0.0f;
    return e;
}

static f90_dsp_event_block make_block(f90_dsp_event* events, uint32_t count, uint32_t samples) {
    static uint64_t next_stream_block_id = 1;
    f90_dsp_event_block blk{};
    blk.stream_block_id = next_stream_block_id++;
    blk.block_samples = samples;
    blk.event_count = count;
    for (uint32_t i = 0; i < count && i < F90_DSP_MAX_EVENTS_PER_BLOCK; ++i) {
        blk.events[i] = events[i];
    }
    return blk;
}

static bool buffers_zero(const float* a, const float* b, uint32_t n) {
    for (uint32_t i = 0; i < n; ++i) {
        if (a[i] != 0.0f || b[i] != 0.0f) {
            return false;
        }
    }
    return true;
}

// --- Fase 5 helpers: render a sequence of absolute-offset events into outL/outR ---
static void render_seq(f90_dsp_handle h, const f90_dsp_event* events, uint32_t count,
                      uint32_t total, float* outL, float* outR) {
    f90_dsp_reset(h);
    f90_dsp_controls c = make_controls();
    uint32_t base = 0;
    while (base < total) {
        uint32_t bs = (total - base) > 1024u ? 1024u : (total - base);
        f90_dsp_event be[F90_DSP_MAX_EVENTS_PER_BLOCK];
        uint32_t n = 0;
        for (uint32_t i = 0; i < count && i < F90_DSP_MAX_EVENTS_PER_BLOCK; ++i) {
            uint32_t off = events[i].sample_offset;
            if (off >= base && off < base + bs) {
                be[n] = events[i];
                be[n].sample_offset = off - base;
                ++n;
            }
        }
        f90_dsp_event_block b = make_block(be, n, bs);
        float l[1024];
        float r[1024];
        f90_dsp_process(h, &b, &c, l, r, bs);
        for (uint32_t i = 0; i < bs; ++i) {
            outL[base + i] = l[i];
            outR[base + i] = r[i];
        }
        base += bs;
    }
}

static double goertzel_energy(const float* x, int n, double freq, double sr) {
    const double w = 2.0 * 3.141592653589793 * freq / sr;
    const double cw = std::cos(w);
    const double coeff = 2.0 * cw;
    double s1 = 0.0, s2 = 0.0;
    for (int i = 0; i < n; ++i) {
        const double s0 = static_cast<double>(x[i]) + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    return s1 * s1 + s2 * s2 - coeff * s1 * s2;
}

static double band_energy(const float* x, int n, double sr, double f0, double f1) {
    double e = 0.0;
    const double step = 200.0;
    for (double f = f0; f <= f1; f += step) {
        e += goertzel_energy(x, n, f, sr);
    }
    return e;
}

static double total_energy(const float* x, int n) {
    double e = 0.0;
    for (int i = 0; i < n; ++i) {
        e += static_cast<double>(x[i]) * static_cast<double>(x[i]);
    }
    return e;
}


static double rms(const float* x, int n) {
    if (n <= 0) return 0.0;
    return std::sqrt(total_energy(x, n) / static_cast<double>(n));
}

static double max_abs(const float* x, int n) {
    double m = 0.0;
    for (int i = 0; i < n; ++i) {
        const double a = std::fabs(static_cast<double>(x[i]));
        if (a > m) m = a;
    }
    return m;
}

static double saturated_fraction(const float* x, int n, double threshold) {
    if (n <= 0) return 0.0;
    int saturated = 0;
    for (int i = 0; i < n; ++i) {
        if (std::fabs(static_cast<double>(x[i])) >= threshold) {
            ++saturated;
        }
    }
    return static_cast<double>(saturated) / static_cast<double>(n);
}

static bool buffers_bit_equal(const float* a, const float* b, uint32_t n) {
    for (uint32_t i = 0; i < n; ++i) {
        if (std::memcmp(&a[i], &b[i], sizeof(float)) != 0) {
            return false;
        }
    }
    return true;
}

// Render a sequence of absolute-offset events into outL, partitioning the total
// span into blocks of `block_size` (state is continuous across sub-blocks).
static void render_to_l(f90_dsp_handle h, const f90_dsp_event* events, uint32_t count,
                        uint32_t total, uint32_t block_size, const f90_dsp_controls& ctl,
                        float* outL) {
    f90_dsp_reset(h);
    uint32_t base = 0;
    while (base < total) {
        uint32_t bs = block_size < (total - base) ? block_size : (total - base);
        f90_dsp_event be[F90_DSP_MAX_EVENTS_PER_BLOCK];
        uint32_t n = 0;
        for (uint32_t i = 0; i < count; ++i) {
            uint32_t off = events[i].sample_offset;
            if (off >= base && off < base + bs) {
                be[n] = events[i];
                be[n].sample_offset = off - base;
                ++n;
            }
        }
        f90_dsp_event_block b = make_block(be, n, bs);
        float l[4096];
        float r[4096];
        f90_dsp_process(h, &b, &ctl, l, r, bs);
        std::memcpy(outL + base, l, bs * sizeof(float));
        base += bs;
    }
}

int main() {
    std::printf("STARTUP REACHED\n");
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    const f90_dsp_config cfg = make_config();
    check(sizeof(f90_dsp_config) == 24, "sizeof config 24");
    check(sizeof(f90_dsp_event) == 32, "sizeof event 32");
    check(sizeof(f90_dsp_event_block) == 16 + 512 * 32, "sizeof event_block 16400");
    check(sizeof(f90_dsp_controls) == 24, "sizeof controls 24");
    check(sizeof(f90_dsp_diagnostics) == 40, "sizeof diagnostics 40");
    check(alignof(f90_dsp_event_block) == 8, "alignof event_block 8");

    check(f90_dsp_abi_version() == F90_AUDIO_DSP_ABI_VERSION, "abi_version 2");
    const char* src = f90_dsp_build_source();
    check(std::strlen(src) == F90_DSP_BUILD_SOURCE_LEN, "build_source len 40");
    check(f90_dsp_check_build_source(src) == F90_DSP_OK, "check_build_source ok");
    const char wrong[41] = "0000000000000000000000000000000000000000";
    check(f90_dsp_check_build_source(wrong) == F90_DSP_ERR_BUILD_MISMATCH, "check_build_source mismatch");
    check(f90_dsp_check_build_source(nullptr) == F90_DSP_ERR_NULL_ARGUMENT, "check_build_source null");

    f90_dsp_handle h = nullptr;
    check(f90_dsp_create(nullptr, &h) == F90_DSP_ERR_NULL_ARGUMENT, "create null cfg");
    check(f90_dsp_create(&cfg, nullptr) == F90_DSP_ERR_NULL_ARGUMENT, "create null out");

    f90_dsp_config bad = cfg;
    bad.sample_rate = 48000;
    check(f90_dsp_create(&bad, &h) == F90_DSP_ERR_UNSUPPORTED_SAMPLE_RATE, "create 48k rejected");
    bad = cfg;
    bad.simulated_cylinders = 6;
    check(f90_dsp_create(&bad, &h) == F90_DSP_ERR_NOT_SUPPORTED, "create 6 cylinders rejected");
    bad = cfg;
    bad.flags = 1;
    check(f90_dsp_create(&bad, &h) == F90_DSP_ERR_NOT_SUPPORTED, "create flags rejected");
    bad = cfg;
    bad.struct_size = 16;
    check(f90_dsp_create(&bad, &h) == F90_DSP_ERR_INVALID_ABI, "create struct_size 16 rejected");

    check(f90_dsp_create(&cfg, &h) == F90_DSP_OK, "create ok");

    f90_dsp_event evs[2];
    evs[0] = make_event(100, 0, 0.5f);
    evs[1] = make_event(300, 1, 0.5f);
    f90_dsp_event_block blk = make_block(evs, 2, 1024);
    f90_dsp_controls ctl = make_controls();
    float left[4096];
    float right[4096];

    check(f90_dsp_process(h, &blk, &ctl, left, right, 1024) == F90_DSP_OK, "process nominal");
    check(!buffers_zero(left, right, 1024), "process produces signal");
    check(buffers_bit_equal(left, right, 1024), "dual-mono exact");

    f90_dsp_event bank1_evs[1];
    bank1_evs[0] = make_event(500, 2, 0.5f);
    bank1_evs[0].bank = 1;
    f90_dsp_event_block bank1_blk = make_block(bank1_evs, 1, 1024);
    float bl[4096];
    float br[4096];
    check(f90_dsp_process(h, &bank1_blk, &ctl, bl, br, 1024) == F90_DSP_OK, "bank1 explicit processed");
    check(!buffers_zero(bl, br, 1024), "bank1 explicit produces signal");
    float lref[4096];
    float rref[4096];
    std::memcpy(lref, left, 1024 * sizeof(float));
    std::memcpy(rref, right, 1024 * sizeof(float));

    ctl.bypass = 1;
    blk.stream_block_id = bank1_blk.stream_block_id + 1;
    check(f90_dsp_process(h, &blk, &ctl, left, right, 1024) == F90_DSP_OK, "process bypass ok");
    check(buffers_zero(left, right, 1024), "bypass silence");
    ctl.bypass = 0;

    f90_dsp_handle h2 = nullptr;
    check(f90_dsp_create(&cfg, &h2) == F90_DSP_OK, "create second ok");
    f90_dsp_event_block blk2 = make_block(evs, 2, 1024);
    float l2[4096];
    float r2[4096];
    f90_dsp_controls ctl2 = make_controls();
    check(f90_dsp_process(h2, &blk2, &ctl2, l2, r2, 1024) == F90_DSP_OK, "process second");
    check(std::memcmp(lref, l2, 1024 * sizeof(float)) == 0, "determinism fresh instances");

    check(f90_dsp_reset(h) == F90_DSP_OK, "reset ok");
    check(f90_dsp_process(h, &blk, &ctl, left, right, 1024) == F90_DSP_OK, "process after reset");
    check(std::memcmp(left, lref, 1024 * sizeof(float)) == 0, "determinism after reset");

    f90_dsp_event_block big{};
    big.stream_block_id = 1;
    big.block_samples = 1024;
    big.event_count = F90_DSP_MAX_EVENTS_PER_BLOCK + 1;
    for (uint32_t i = 0; i < F90_DSP_MAX_EVENTS_PER_BLOCK; ++i) {
        big.events[i] = make_event(i * 2, static_cast<uint8_t>(i % 5), 0.2f);
    }
    check(f90_dsp_process(h, &big, &ctl, left, right, 1024) == F90_DSP_ERR_EVENT_OVERFLOW, "overflow rc");
    f90_dsp_diagnostics diag{};
    diag.struct_size = sizeof(diag);
    f90_dsp_get_diagnostics(h, &diag);
    check(diag.events_dropped >= 1, "overflow dropped counted");

    f90_dsp_event oob_evs[1];
    oob_evs[0] = make_event(5000, 0, 0.5f);
    f90_dsp_event_block oob_blk = make_block(oob_evs, 1, 1024);
    check(f90_dsp_reset(h) == F90_DSP_OK, "reset before failure paths");
    check(f90_dsp_process(h, &oob_blk, &ctl, left, right, 1024) == F90_DSP_ERR_INVALID_STATE,
          "out-of-range rejected");
    check(buffers_zero(left, right, 1024), "out-of-range silence");

    f90_dsp_event bad_evs[2];
    bad_evs[0] = make_event(500, 0, 0.5f);
    bad_evs[1] = make_event(200, 1, 0.5f);
    f90_dsp_event_block bad_blk = make_block(bad_evs, 2, 1024);
    check(f90_dsp_process(h, &bad_blk, &ctl, left, right, 1024) == F90_DSP_ERR_INVALID_STATE, "non-monotonic rc");
    check(buffers_zero(left, right, 1024), "non-monotonic silence");

    f90_dsp_controls nan_ctl = make_controls();
    nan_ctl.rpm = std::numeric_limits<float>::quiet_NaN();
    f90_dsp_event_block none = make_block(nullptr, 0, 1024);
    check(f90_dsp_process(h, &none, &nan_ctl, left, right, 1024) == F90_DSP_ERR_NONFINITE_INPUT, "nan controls rc");
    check(buffers_zero(left, right, 1024), "nan controls output zero");
    f90_dsp_get_diagnostics(h, &diag);
    check(diag.nonfinite_inputs >= 1, "nan inputs counted");

    check(f90_dsp_process(h, &none, &ctl, left, right, 5000) == F90_DSP_ERR_BLOCK_TOO_LARGE, "frames too large");
    f90_dsp_event_block mismatch = make_block(nullptr, 0, 512);
    check(f90_dsp_process(h, &mismatch, &ctl, left, right, 1024) == F90_DSP_ERR_INVALID_STATE, "samples mismatch");

    // ABI v2 ordering is the complete tuple, not sample_offset alone.
    f90_dsp_reset(h);
    f90_dsp_event tuple_bad[2];
    tuple_bad[0] = make_event(20, 4, 0.5f);
    tuple_bad[0].bank = 1;
    tuple_bad[1] = make_event(20, 0, 0.5f);
    tuple_bad[1].bank = 0;
    f90_dsp_event_block tuple_bad_block = make_block(tuple_bad, 2, 128);
    check(f90_dsp_process(h, &tuple_bad_block, &ctl, left, right, 128) == F90_DSP_ERR_INVALID_STATE,
          "complete tuple order rejected");

    f90_dsp_reset(h);
    f90_dsp_event nonfinite_event[1];
    nonfinite_event[0] = make_event(20, 0, 0.5f);
    nonfinite_event[0].energy = std::numeric_limits<float>::infinity();
    f90_dsp_event_block nonfinite_block = make_block(nonfinite_event, 1, 128);
    check(f90_dsp_process(h, &nonfinite_block, &ctl, left, right, 128) == F90_DSP_ERR_NONFINITE_INPUT,
          "nonfinite event rejected");

    f90_dsp_reset(h);
    f90_dsp_event_block id_first = make_block(nullptr, 0, 128);
    check(f90_dsp_process(h, &id_first, &ctl, left, right, 128) == F90_DSP_OK,
          "stream id first after reset accepted");
    f90_dsp_event_block id_repeat = id_first;
    check(f90_dsp_process(h, &id_repeat, &ctl, left, right, 128) == F90_DSP_ERR_INVALID_STATE,
          "stream id repeat rejected");
    id_repeat.stream_block_id -= 1;
    check(f90_dsp_process(h, &id_repeat, &ctl, left, right, 128) == F90_DSP_ERR_INVALID_STATE,
          "stream id decreasing rejected");
    f90_dsp_reset(h);
    check(f90_dsp_process(h, &id_repeat, &ctl, left, right, 128) == F90_DSP_OK,
          "stream id baseline restarts after reset");

    // ---- Fase 3: pruebas temporales (3.5) ----
    // 1. Evento en offset 17: 0..16 cero, primer sample afectado = 17.
    {
        f90_dsp_handle ht = nullptr;
        check(f90_dsp_create(&cfg, &ht) == F90_DSP_OK, "temporal create offset17");
        f90_dsp_event e[1];
        e[0] = make_event(17, 0, 0.5f);
        f90_dsp_event_block b = make_block(e, 1, 256);
        float l[512];
        float r[512];
        f90_dsp_controls c = make_controls();
        check(f90_dsp_process(ht, &b, &c, l, r, 256) == F90_DSP_OK, "temporal process offset17");
        bool pre_zero = true;
        for (uint32_t i = 0; i < 17; ++i) {
            if (l[i] != 0.0f || r[i] != 0.0f) pre_zero = false;
        }
        check(pre_zero, "temporal offset17 samples 0..16 zero");
        check(l[17] != 0.0f || r[17] != 0.0f, "temporal offset17 sample 17 affected");
        f90_dsp_destroy(ht);
    }
    // 2. Eventos en offsets 17 y 93: ambos disparan en su sitio.
    {
        f90_dsp_handle ht = nullptr;
        check(f90_dsp_create(&cfg, &ht) == F90_DSP_OK, "temporal create 17and93");
        f90_dsp_event e[2];
        e[0] = make_event(17, 0, 0.5f);
        e[1] = make_event(93, 1, 0.5f);
        f90_dsp_event_block b = make_block(e, 2, 256);
        float l[512];
        float r[512];
        f90_dsp_controls c = make_controls();
        check(f90_dsp_process(ht, &b, &c, l, r, 256) == F90_DSP_OK, "temporal process 17and93");
        bool pre_zero = true;
        for (uint32_t i = 0; i < 17; ++i) {
            if (l[i] != 0.0f || r[i] != 0.0f) pre_zero = false;
        }
        check(pre_zero, "temporal 17and93 samples 0..16 zero");
        check((l[17] != 0.0f || r[17] != 0.0f) && (l[93] != 0.0f || r[93] != 0.0f),
              "temporal 17and93 both offsets fire");
        f90_dsp_destroy(ht);
    }
    // 3. Dos eventos mismo offset, bancos distintos: ambos se procesan.
    {
        f90_dsp_handle ht = nullptr;
        check(f90_dsp_create(&cfg, &ht) == F90_DSP_OK, "temporal create bothbanks");
        f90_dsp_event one[1];
        one[0] = make_event(50, 0, 0.5f);
        one[0].bank = 0;
        f90_dsp_event both[2];
        both[0] = make_event(50, 0, 0.5f);
        both[0].bank = 0;
        both[1] = make_event(50, 0, 0.5f);
        both[1].bank = 1;
        float l1[1024];
        float r1[1024];
        float l2b[1024];
        float r2b[1024];
        f90_dsp_controls c = make_controls();
        f90_dsp_event_block b1 = make_block(one, 1, 1024);
        f90_dsp_process(ht, &b1, &c, l1, r1, 1024);
        f90_dsp_reset(ht);
        f90_dsp_event_block b2 = make_block(both, 2, 1024);
        f90_dsp_process(ht, &b2, &c, l2b, r2b, 1024);
        float max1 = 0.0f, max2 = 0.0f;
        for (uint32_t i = 0; i < 1024; ++i) {
            if (std::fabs(l1[i]) > max1) max1 = std::fabs(l1[i]);
            if (std::fabs(l2b[i]) > max2) max2 = std::fabs(l2b[i]);
        }
        check(!buffers_zero(l2b, r2b, 1024), "temporal bothbanks produce signal");
        check(max2 > max1 * 1.5f, "temporal bothbanks louder than one");
        f90_dsp_destroy(ht);
    }
    // 4. Evento en el ultimo sample: no aparece al inicio.
    {
        f90_dsp_handle ht = nullptr;
        check(f90_dsp_create(&cfg, &ht) == F90_DSP_OK, "temporal create last");
        f90_dsp_event e[1];
        e[0] = make_event(255, 0, 0.5f);
        f90_dsp_event_block b = make_block(e, 1, 256);
        float l[512];
        float r[512];
        f90_dsp_controls c = make_controls();
        check(f90_dsp_process(ht, &b, &c, l, r, 256) == F90_DSP_OK, "temporal process last");
        bool pre = true;
        for (uint32_t i = 0; i < 255; ++i) {
            if (l[i] != 0.0f || r[i] != 0.0f) pre = false;
        }
        check(pre, "temporal last samples 0..254 zero");
        check(l[255] != 0.0f || r[255] != 0.0f, "temporal last sample affected");
        f90_dsp_destroy(ht);
    }
    // 5. Banco B en el bloque siguiente: no se pierde ni se desfasa.
    {
        f90_dsp_handle ht = nullptr;
        check(f90_dsp_create(&cfg, &ht) == F90_DSP_OK, "temporal create nextblock");
        f90_dsp_controls c = make_controls();
        f90_dsp_event e1[1];
        e1[0] = make_event(1000, 0, 0.5f);
        e1[0].bank = 1;
        float l1b[2048];
        float r1b[2048];
        f90_dsp_event_block b1 = make_block(e1, 1, 1024);
        check(f90_dsp_process(ht, &b1, &c, l1b, r1b, 1024) == F90_DSP_OK,
              "temporal nextblock b1");
        bool pre1 = true;
        for (uint32_t i = 0; i < 1000; ++i) {
            if (l1b[i] != 0.0f) pre1 = false;
        }
        check(pre1, "temporal nextblock b1 samples 0..999 zero");
        check(l1b[1000] != 0.0f, "temporal nextblock b1 event at 1000");
        // Isolate block2: clear the ringing tail from block1 so we can verify a
        // deferred bank-B event is placed at its own offset (not lost, not at 0).
        check(f90_dsp_reset(ht) == F90_DSP_OK, "temporal nextblock reset");
        f90_dsp_event e2[1];
        e2[0] = make_event(5, 0, 0.5f);
        e2[0].bank = 1;
        float l2b[2048];
        float r2b[2048];
        f90_dsp_event_block b2 = make_block(e2, 1, 1024);
        check(f90_dsp_process(ht, &b2, &c, l2b, r2b, 1024) == F90_DSP_OK,
              "temporal nextblock b2");
        bool pre2 = true;
        for (uint32_t i = 0; i < 5; ++i) {
            if (l2b[i] != 0.0f) pre2 = false;
        }
        check(pre2, "temporal nextblock b2 samples 0..4 zero");
        check(l2b[5] != 0.0f, "temporal nextblock b2 event at 5");
        f90_dsp_destroy(ht);
    }
    // 6. Particionar 2048 como 2048 / 2x1024 / 4x512 / 8x256 da igual salida.
    {
        f90_dsp_event seq[12];
        for (uint32_t i = 0; i < 12; ++i) {
            seq[i] = make_event(80 + i * 160, static_cast<uint8_t>(i % 5), 0.4f + 0.1f * (i % 3));
            seq[i].bank = static_cast<uint8_t>(i % 2);
        }
        const uint32_t total = 2048;
        f90_dsp_handle ha = nullptr, hb = nullptr, hc = nullptr, hd = nullptr;
        check(f90_dsp_create(&cfg, &ha) == F90_DSP_OK, "temporal bs create a");
        check(f90_dsp_create(&cfg, &hb) == F90_DSP_OK, "temporal bs create b");
        check(f90_dsp_create(&cfg, &hc) == F90_DSP_OK, "temporal bs create c");
        check(f90_dsp_create(&cfg, &hd) == F90_DSP_OK, "temporal bs create d");
        float out1[4096] = {0}, out2[4096] = {0}, out3[4096] = {0}, out4[4096] = {0};
        f90_dsp_controls c = make_controls();
        render_to_l(ha, seq, 12, total, 2048, c, out1);
        render_to_l(hb, seq, 12, total, 1024, c, out2);
        render_to_l(hc, seq, 12, total, 512, c, out3);
        render_to_l(hd, seq, 12, total, 256, c, out4);
        check(buffers_bit_equal(out1, out2, total), "temporal bs 2048 vs 2x1024");
        check(buffers_bit_equal(out1, out3, total), "temporal bs 2048 vs 4x512");
        check(buffers_bit_equal(out1, out4, total), "temporal bs 2048 vs 8x256");
        f90_dsp_destroy(ha);
        f90_dsp_destroy(hb);
        f90_dsp_destroy(hc);
        f90_dsp_destroy(hd);
    }
    // 7. Bypass durante un bloque y luego off: no reproduce un ataque viejo.
    {
        f90_dsp_handle ht = nullptr;
        check(f90_dsp_create(&cfg, &ht) == F90_DSP_OK, "temporal create bypass");
        f90_dsp_event e[1];
        e[0] = make_event(50, 0, 0.5f);
        f90_dsp_controls cb = make_controls();
        cb.bypass = 1;
        float l[1024];
        float r[1024];
        f90_dsp_event_block bev = make_block(e, 1, 1024);
        check(f90_dsp_process(ht, &bev, &cb, l, r, 1024) == F90_DSP_OK,
              "temporal bypass b1");
        f90_dsp_event_block empty = make_block(nullptr, 0, 1024);
        f90_dsp_controls c2 = make_controls();
        c2.bypass = 0;
        float l2b[1024];
        float r2b[1024];
        check(f90_dsp_process(ht, &empty, &c2, l2b, r2b, 1024) == F90_DSP_OK,
              "temporal bypass b2 empty");
        float mx = 0.0f;
        for (uint32_t i = 0; i < 1024; ++i) {
            if (std::fabs(l2b[i]) > mx) mx = std::fabs(l2b[i]);
        }
        check(mx < 0.05f, "temporal bypass no stale attack");
        f90_dsp_destroy(ht);
    }
    // 8. Sesion larga: sin violaciones RT ni salida no finita.
    {
        f90_dsp_handle ht = nullptr;
        check(f90_dsp_create(&cfg, &ht) == F90_DSP_OK, "temporal create long");
        f90_dsp_controls c = make_controls();
        f90_dsp_event e[1];
        e[0] = make_event(10, 0, 0.5f);
        float l[2048];
        float r[2048];
        bool ok = true;
        f90_dsp_event_block bev = make_block(e, 1, 1024);
        for (int i = 0; i < 1000; ++i) {
            bev.stream_block_id += 1;
            if (f90_dsp_process(ht, &bev, &c, l, r, 1024) != F90_DSP_OK) {
                ok = false;
                break;
            }
        }
        f90_dsp_diagnostics d{};
        d.struct_size = sizeof(d);
        f90_dsp_get_diagnostics(ht, &d);
        check(ok, "temporal long session ok");
        check(d.rt_violations == 0, "temporal long no rt violations");
        check(d.nonfinite_outputs == 0, "temporal long no nonfinite outputs");
        f90_dsp_destroy(ht);
    }

    // ---- Fase 5: Faust post-combustion timbre ----
    // (a) NO events -> output must be EXACT silence (no free-running hiss).
    {
        f90_dsp_handle fph = nullptr;
        check(f90_dsp_create(&cfg, &fph) == F90_DSP_OK, "fase5 create");
        f90_dsp_event_block fpnone = make_block(nullptr, 0, 1024);
        f90_dsp_controls fc = make_controls();
        float fL[2048];
        float fR[2048];
        bool silent = true;
        for (int b = 0; b < 4; ++b) {
            fpnone.stream_block_id += 1;
            f90_dsp_process(fph, &fpnone, &fc, fL, fR, 1024);
            if (!buffers_zero(fL, fR, 1024)) silent = false;
        }
        check(silent, "fase5 no-event exact silence");
        f90_dsp_destroy(fph);
    }
    // (b)+(c) events -> non-zero output with meaningful 2.5-7 kHz energy AND low
    // (modal) energy: both banks contribute.
    {
        f90_dsp_handle fph = nullptr;
        check(f90_dsp_create(&cfg, &fph) == F90_DSP_OK, "fase5 create events");
        f90_dsp_event fevs[24];
        for (uint32_t i = 0; i < 24; ++i) {
            fevs[i] = make_event(100 + i * 80, static_cast<uint8_t>(i % 5), 0.5f);
            fevs[i].pressure_derivative = 15000.0f;
            fevs[i].bank = static_cast<uint8_t>(i % 2);
        }
        float fL[4096];
        float fR[4096];
        render_seq(fph, fevs, 24, 2048, fL, fR);
        double tot = total_energy(fL, 2048);
        double hf = band_energy(fL, 2048, 44100.0, 2500.0, 7000.0);
        double lo = band_energy(fL, 2048, 44100.0, 20.0, 400.0);
        check(tot > 0.0, "fase5 events produce signal");
        check(buffers_bit_equal(fL, fR, 2048), "fase5 dual-mono exact");
        check(hf / tot > 0.01, "fase5 2.5-7kHz band non-trivial");
        check(lo > 0.0, "fase5 modal low-freq energy present");
        check((hf > 0.0 && lo > 0.0), "fase5 both banks (modal + HF) contribute");
        f90_dsp_destroy(fph);
    }
    // (d) Runtime-scale derivatives must retain headroom and transient dynamics.
    // This regression test catches the old physical-units -> audio amplitude bug,
    // where almost the whole stem lived against the limiter.
    {
        f90_dsp_handle fh = nullptr;
        check(f90_dsp_create(&cfg, &fh) == F90_DSP_OK, "fase5 create dynamics");
        f90_dsp_event ev[96];
        for (uint32_t i = 0; i < 96; ++i) {
            ev[i] = make_event(20 + i * 42, static_cast<uint8_t>(i % 5), 0.9f);
            ev[i].pressure_derivative = 27000.0f;
            ev[i].bank = static_cast<uint8_t>(i % 2);
        }
        float l[8192];
        float r[8192];
        render_seq(fh, ev, 96, 4096, l, r);
        const double peak = max_abs(l, 4096);
        const double signal_rms = rms(l, 4096);
        const double crest = signal_rms > 0.0 ? peak / signal_rms : 0.0;
        const double hot = saturated_fraction(l, 4096, 0.95);
        check(peak < 0.95, "fase5 runtime-scale derivative keeps peak headroom");
        check(hot < 0.001, "fase5 runtime-scale derivative not saturation wall");
        check(crest > 1.20, "fase5 runtime-scale derivative retains transient crest");
        f90_dsp_destroy(fh);
    }

    // (e) event rate / strength tracks the content: a higher-RPM train yields
    // higher HF energy than an idle train.
    {
        f90_dsp_handle fhi = nullptr, fidle = nullptr;
        check(f90_dsp_create(&cfg, &fhi) == F90_DSP_OK, "fase5 create hi");
        check(f90_dsp_create(&cfg, &fidle) == F90_DSP_OK, "fase5 create idle");
        f90_dsp_event fhiEvs[256];
        f90_dsp_event fidleEvs[64];
        for (uint32_t i = 0; i < 256; ++i) {
            fhiEvs[i] = make_event(20 + i * 16, static_cast<uint8_t>(i % 5), 0.8f);
            fhiEvs[i].pressure_derivative = 27000.0f;
        }
        for (uint32_t i = 0; i < 64; ++i) {
            fidleEvs[i] = make_event(20 + i * 64, static_cast<uint8_t>(i % 5), 0.2f);
            fidleEvs[i].pressure_derivative = 6000.0f;
        }
        float fLhi[8192];
        float fRhi[8192];
        float fLidle[8192];
        float fRidle[8192];
        render_seq(fhi, fhiEvs, 256, 4096, fLhi, fRhi);
        render_seq(fidle, fidleEvs, 64, 4096, fLidle, fRidle);
        double hf_hi = band_energy(fLhi, 4096, 44100.0, 2500.0, 7000.0);
        double hf_idle = band_energy(fLidle, 4096, 44100.0, 2500.0, 7000.0);
        check(hf_hi > hf_idle * 1.5, "fase5 higher-RPM train yields higher HF energy");
        f90_dsp_destroy(fhi);
        f90_dsp_destroy(fidle);
    }

    const auto t0 = std::chrono::steady_clock::now();
    uint64_t total_samples = 0;
    bool stress_ok = true;
    for (int i = 0; i < 500; ++i) {
        f90_dsp_event_block stress = make_block(nullptr, 0, 4096);
        if (f90_dsp_process(h, &stress, &ctl, left, right, 4096) != F90_DSP_OK) {
            stress_ok = false;
            break;
        }
        total_samples += 4096;
    }
    const auto t1 = std::chrono::steady_clock::now();
    const double seconds = std::chrono::duration<double>(t1 - t0).count();
    check(stress_ok, "stress ok");
    std::printf("stress: %llu samples in %.3f s\n", (unsigned long long)total_samples, seconds);
    f90_dsp_get_diagnostics(h, &diag);
    check(diag.rt_violations == 0, "no rt violations");
    check(diag.nonfinite_outputs == 0, "no nonfinite outputs");

    f90_dsp_diagnostics zero_diag{};
    zero_diag.struct_size = sizeof(zero_diag);
    check(f90_dsp_get_diagnostics(nullptr, &zero_diag) == F90_DSP_ERR_NULL_ARGUMENT, "diag null handle");
    check(f90_dsp_reset(nullptr) == F90_DSP_ERR_NULL_ARGUMENT, "reset null handle");
    f90_dsp_destroy(nullptr);
    f90_dsp_destroy(h);
    f90_dsp_destroy(h2);

    if (g_failures == 0) {
        std::printf("ALL TESTS PASSED\n");
        return 0;
    }
    std::printf("%d FAILURES\n", g_failures);
    return 1;
}
