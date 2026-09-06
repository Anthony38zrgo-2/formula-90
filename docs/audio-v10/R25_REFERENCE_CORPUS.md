# Renault R25 reference corpus for AUD-01

Inventory date: 2026-09-04. The captures below were found in the ignored source
directory `docs/audio-design/V10-curated` of the original checkout. They were
read and hashed without modifying that checkout. The packaged runtime metadata
records matching hashes for the four sources from which current loop assets
were derived.

All `int` captures are provisionally classified as **interior/onboard
perspective** based only on the asset naming. Operating-state labels in the
table are provisional for the same reason. Exact microphone placement,
recorder processing, vehicle speed, gear and track location are unknown. RPM
values are spectral estimates, not telemetry, unless explicitly replaced by a
documented telemetry source.

| Capture | Operating state | Bytes | SHA-256 | RPM evidence | Current use |
|---|---|---:|---|---|---|
| `01_23_r25_int_on_mid.wav` | on-throttle, mid | 293492 | `b3db1abdd061a840a8d3075a25a8c4ccf89df26fac3f57edb007218b687db8fd` | 5122.5 estimated; loop 5122.6674 | low zone |
| `01_01_r25_int_on_midhigh.wav` | on-throttle, mid-high | 467460 | `4ee9329dc9fe82810691c8c5cf49b34d600d67b5a22875e0c0a05b2b2793999f` | 5965.5 estimated; loop 5965.8045 | medium zone |
| `01_22_r25_int_on_high.wav` | on-throttle, high | 871304 | `8ed4cc45b4b6c2f4cf0d90cd5c69dfaae2fe2bfbcf4fcdd87d0ba94026655104` | 8731.5 estimated; loop 8731.4726 | high zone |
| `01_08_r25_int_off_mid.wav` | off-throttle, mid | 461156 | `5fb1bca595860415c2bde95408829656962bfeb79350c92a7249f3e8cc1ca8ed` | 8172.0 estimated; loop 8172.2718 | lift/coast layer |
| `01_06_r25_int_off_downshift.wav` | off-throttle/downshift | 335564 | `91d9f1f6cb8554fb1bf21864306715a9db2b8dce8007b70f9ef733e1b4e524a9` | unknown | reference only |
| `01_29_r25_int_on_upshift.wav` | on-throttle/upshift | 726108 | `6a5a453a10c69edcf6af79327e2eb1986aaaea8d63cf5578d63374a5622a9265` | unknown | reference only |

`gear_down_alt.wav` and `gear_up_alt.wav` are excluded from the R25 corpus:
their names do not establish vehicle or recording provenance.

## Uncertainty register

- Spectral RPM anchors assume the dominant firing order is five times shaft
  frequency. Harmonic misidentification remains possible and must be checked
  against pitch continuity and any future synchronized video/telemetry.
- The labels `mid`, `midhigh`, and `high` describe the source files; they are not
  calibrated operating ranges.
- Unknown recorder EQ, limiting, compression, wind, transmission noise and
  cockpit resonances prevent direct amplitude matching.

## Listening protocol and human gate

First audition each complete capture and record the exact sample/time window
whose operating state is sufficiently steady or whose shift transient is
unambiguous. Render matching RPM/state windows from GF509: held 5123, 5966,
8172 and 8731 RPM; an acceleration through all anchors; lift/coast around 8172
RPM; one upshift and one downshift. Compare each against the selected window after
integrated-loudness matching, with peak limiting disabled unless it is present
in both candidates. Randomize A/B labels and use identical headphones/output
gain.

The reviewer records, per window: preferred candidate; tonal pitch/brightness;
combustion pulse definition; intake/exhaust balance; rasp/aliasing; mechanical
and transmission content; transient timing; and any loop/crossfade artifact.
Acceptance requires completed human notes for all four states plus both shift
directions. This document intentionally records no preference before listening.
