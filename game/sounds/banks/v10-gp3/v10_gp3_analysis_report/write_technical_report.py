from pathlib import Path
import json
import math
import sys
import hashlib
import html

analysis_directory = Path(__file__).resolve().parent
sys.path.insert(0, str(analysis_directory / 'python-packages'))
import markdown

analysis = json.loads((analysis_directory / 'measurements.json').read_text())
details = json.loads((analysis_directory / 'diagnostic_details.json').read_text())
measurements = {entry['filename']: entry for entry in analysis['measurements']}
loop_order = ['98_int_idle.wav', '98_int_low.wav', '98_int_med.wav', '98_int_high_1.wav', '98_int_max_5.wav']
event_order = [name for name in measurements if name not in loop_order]
ordered_names = loop_order + event_order
paragraphs = []


def append(content):
    paragraphs.append(content.strip() + '\n')


def number(value, digits=2):
    return 'N/A' if value is None else f'{value:.{digits}f}'


def table(headers, rows):
    append('| ' + ' | '.join(headers) + ' |\n| ' + ' | '.join('---' for unused in headers) + ' |\n' + '\n'.join('| ' + ' | '.join(str(value) for value in row) + ' |' for row in rows))


append('''# V10 GP3 source-bank analysis and proposed remaster

Analysis date: 20 September 2026. Scope: all 13 audio files in `D:\\Formula90s\\game\\sounds\\banks\\v10-gp3`, recursively inventoried. No source audio, sampler configuration, or repository files were modified. No remastered audio was generated.

**The priority is level calibration, DC correction, and loop preparation. The bank does not justify a global mastering preset.** High and maximum have strong evidence of baked clipping/limiting, idle has marked tonal amplitude modulation, and low/medium have unsafe raw loop joins. Broad treble suppression, denoising, pitch correction, harmonic replacement, and additional limiting would risk changing the sound more than the measurements justify.

These are measurement-based engineering recommendations, not an approved listening result. “Harsh,” “unwanted,” and “cleaner” require a level-matched listening comparison. A narrow spectral peak alone is not a defect in an engine recording.

## Provenance and measurement definitions

Repository branch: `main-clean`; HEAD: `ce768b53f5c0a068253b42f7489cd89f93433059`. The checkout was already dirty, including staged source WAV additions and unrelated audio/runtime work. Its initial inventory is recorded in `repository-status-before.txt`; nothing was staged, committed, reset, rebuilt, or run in Godot. Every WAV was hashed before and after analysis. All 13 SHA-256 values are unchanged.

The existing sampler manifest identifies five engine loops and eight one-shot events. All sources are 44,100 Hz, 16-bit PCM. Backfires and limiter are effectively dual mono: channel correlation is at least 0.999997. Engine layers and gear events are mono. Stereo file LUFS therefore reads approximately 3.01 LU higher than an averaged mono playback signal; this is a channel-count effect, not extra engine loudness. Both file LUFS and mono comparisons are provided.

- Peak: maximum absolute decoded sample, across channels. RMS: unweighted root mean square, averaged over samples and channels; 0 dBFS denotes amplitude 1. Crest factor: peak minus RMS.
- Estimated true peak: 8× SciPy polyphase reconstruction, Kaiser beta 12, line extension at edges. This is an oversampled estimate, not a certified BS.1770 true-peak meter. Reported hundredths aid comparison, not a claim of that much absolute accuracy.
- Integrated LUFS: pyloudnorm default K weighting, 400 ms blocks, 75% overlap, standard absolute/relative gates. Files under 400 ms are N/A, without padding or repetition. Values for the other short files remain descriptive, not programme-loudness certification.
- Short-event alternatives: mono whole-clip ungated K-weighted energy and maximum 100 ms K-weighted level. These are **not** standard integrated or momentary LUFS. They are useful within a consistent event family.
- Envelope dynamic range: P95 minus P10 of 20 ms RMS windows, 5 ms hop. Sustained variability: P90 minus P10 of 100 ms RMS, 10 ms hop, central 80% of the file. These are defined local descriptors, **not EBU LRA**. No stable programme LRA is claimed for 0.194–5.054 s assets.
- Spectrum: Welch Hann windows up to 16,384 samples, 50% overlap, no detrending, approximately 2.69 Hz bins for long files. Band shares describe this Welch estimate; for short transients its window weighting differs from whole-file energy. Sub-bass is rechecked after subtracting the mean to separate offset from rumble.
- Pitch: 160 ms windows/20 ms hop; normalized autocorrelation at 12 kHz, plus interpolated tracking of the dominant spectral line within ±4%. Pitch variation is P90–P10 in cents over the central 80%. A tracked harmonic or autocorrelation period is not automatically the physical firing fundamental.
- Noise: minimum/low-percentile envelopes, tails, spectra and spectrograms. There is no isolated noise-only reference. Noise floor and signal-to-noise ratio cannot be separated reliably from combustion texture, tonal modulation, and fades.

Method references: [ITU-R BS.1770](https://www.itu.int/rec/R-REC-BS.1770), [EBU metering and LRA limitations](https://tech.ebu.ch/news/2016/08/ebu-loudness-changes), [pyloudnorm implementation](https://github.com/csteinmetz1/pyloudnorm/blob/master/pyloudnorm/meter.py), [SciPy Welch](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.welch.html), and [SciPy polyphase resampling](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.resample_poly.html).
''')
append('Python ' + analysis['python'] + '; libraries: ' + ', '.join(f'{name} {version}' for name, version in analysis['libraries'].items()) + '. Scripts and machine-readable measurements are included alongside this report.')

append('## 1. Levels, dynamics, DC, and clipping')
table(['File', 's / ch', 'Peak dBFS', 'Est. TP dBFS', 'RMS dBFS', 'File LUFS-I', 'Crest dB', 'Envelope DR dB'], [[name, f"{entry['duration_seconds']:.3f} / {entry['channels']}", number(entry['sample_peak_decibels_full_scale']), number(entry['estimated_true_peak_decibels_full_scale']), number(entry['root_mean_square_decibels_full_scale']), number(entry['integrated_loudness_loudness_units_full_scale']), number(entry['crest_factor_decibels']), number(entry['dynamic_range_20_millisecond_95_minus_10_decibels'])] for name in ordered_names for entry in [measurements[name]]])
append('All files have **zero digital-rail samples** at −1 or +32767/32768. This does not mean that the sources have never been clipped. The high layer has a repeated ceiling around ±0.931; maximum has many short flat tops around ±0.99. These are below the current encoding rails.')
table(['File', 'DC signed (% FS, first ch)', 'DC magnitude dBFS', 'Samples ≥−0.1 dBFS (%)', '≥−1 dBFS (%)', 'High-level plateau participation (%)', 'Longest plateau (samples)'], [[name, number(entry['direct_current_offset_signed'][0] * 100, 4), number(entry['direct_current_offset_decibels_full_scale'][0]), number(entry['near_clip_minus_point_one_decibel_percent'], 3), number(entry['near_clip_minus_one_decibel_percent'], 3), number(details[name]['samples_participating_in_high_amplitude_plateaus_percent'], 3), entry['longest_high_amplitude_constant_run_samples']] for name in ordered_names for entry in [measurements[name]]])
append('Plateau participation counts samples in exact repeated-value runs with absolute amplitude >0.5, on the mono signal. Isolated 2–3 sample coincidences in other files are not sufficient clipping evidence. In high, 7,596 samples equal −0.931091 and 7,233 equal +0.931061; the longest flat run is 37 samples (0.84 ms). Maximum reaches an estimated +0.16 dBFS reconstructed peak. Its original peaks cannot be recovered by reducing gain or applying EQ. Retain this saturation as part of the baseline character; any de-clipping experiment must be a separate, auditioned variant.')
append('![Baked plateau evidence](figures/baked_plateau_evidence.png)')

append('## 2. Low-level content, amplitude stability, and transients')
table(['File', 'Quietest 20 ms dBFS', 'Time of quiet window (s)', 'P10 20 ms dBFS', 'Final 20 ms dBFS', 'Central 100 ms variation dB', 'Peak time ms', '5–95% energy span ms'], [[name, number(entry['minimum_20_millisecond_level_decibels_full_scale']), number(entry['minimum_20_millisecond_center_seconds'], 3), number(entry['low_10_percentile_20_millisecond_level_decibels_full_scale']), number(entry['last_20_millisecond_level_decibels_full_scale']), number(entry['sustained_100_millisecond_90_minus_10_decibels']), number(entry['time_of_absolute_peak_seconds'] * 1000, 1), number((entry['energy_arrival_seconds']['95'] - entry['energy_arrival_seconds']['5']) * 1000, 1)] for name in ordered_names for entry in [measurements[name]]])
append('The quiet-window figures are **signal-containing levels, not measured noise floors**. Backfire minima occur in the decaying tail. Event variation and energy span describe the intended transient and must not be flattened to the engine-loop target. The limiter is a short active burst, not a quiet recording suitable for a noise profile. Gear-down retains substantial bass energy despite its short duration.')
table(['Event', 'Mono LUFS-I', 'Mono ungated K level', 'Max 100 ms K level', 'Final 5 ms dBFS'], [[name, number(details[name]['mono_integrated_loudness']), number(details[name]['mono_ungated_weighted_level']), number(measurements[name]['maximum_100_millisecond_weighted_level']), number(details[name]['final_5_millisecond_level_decibels_full_scale'])] for name in event_order])
append('Idle varies by 4.86 dB in the sustained descriptor; low/medium/high/maximum are approximately 1.52/1.95/0.99/2.18 dB. Idle has two substantial envelope troughs near 0.4 and 1.25 s. Its 300–600 Hz band varies by 6.24 dB while 2.5–5 kHz varies by only 0.79 dB. This points to tonal amplitude modulation rather than a broadband level problem. Leave some idle unevenness intact; aggressive gain riding would raise the steady broadband component during the troughs.')
table(['Loop', '80–150 Hz range dB', '300–600 Hz range dB', '2.5–5 kHz range dB', '8–16 kHz range dB'], [[name] + [number(details[name]['band_envelopes'][region]['range_90_minus_10']) for region in ['80-150', '300-600', '2500-5000', '8000-16000']] for name in loop_order])
append('These diagnostic band envelopes use third-order Butterworth bandpasses applied forward/backward, then 100 ms RMS over the central envelope frames. Detector differences mean their thresholds must be recalibrated in the chosen processor. Low and medium show approximately 9 dB modulation in 80–150 Hz despite relatively stable broadband levels; this supports limited band-specific control over a full-band compressor if that movement is audible.')

append('## 3. Spectral balance and harmonic identity')
table(['File', '<20 Hz %', '20–80 Hz %', '80–300 Hz %', '300–1200 Hz %', '1.2–5 kHz %', '5–8 kHz %', '>8 kHz %', 'Centroid Hz', '99% rolloff Hz'], [[name, number(regions['0-20']['power_percent'], 3), number(sum(regions[key]['power_percent'] for key in ['20-40', '40-80']), 3), number(sum(regions[key]['power_percent'] for key in ['80-150', '150-300']), 2), number(sum(regions[key]['power_percent'] for key in ['300-600', '600-1200']), 2), number(sum(regions[key]['power_percent'] for key in ['1200-2500', '2500-5000']), 2), number(regions['5000-8000']['power_percent'], 3), number(sum(regions[key]['power_percent'] for key in ['8000-12000', '12000-16000', '16000-20000', '20000-24000']), 4), number(entry['spectral']['centroid_hertz'], 1), number(entry['spectral']['rolloff_99_hertz'], 1)] for name in ordered_names for entry in [measurements[name]] for regions in [entry['spectral']['regions']]])
append('Low’s apparent <20 Hz share falls from 0.752% to 0.043% after mean removal: most is DC, not rumble. Corresponding residual <20 Hz shares are idle 0.00013%, medium 0.061%, high 0.116%, maximum 0.118%. There is no justification for a common 80–100 Hz high-pass filter: it would remove real low-order engine harmonics and much of the gear-event impact.')
table(['File', 'Six strongest resolved peaks, Hz'], [[name, ', '.join(number(peak['frequency_hertz'], 1) for peak in measurements[name]['spectral']['dominant_peaks'][:6])] for name in ordered_names])
append('One-shot peaks describe transient resonances, not stable fundamentals. Pitch stability is not a meaningful pass/fail requirement for backfires, limiter interruption or gear thumps. For these events, preserve the measured attack and decay; no pitch correction is indicated.')
table(['Loop', 'Candidate harmonic spacing Hz', 'Autocorrelation period Hz / confidence', 'Tracked line Hz', 'Line P90–P10 cents', 'Line drift cents/s'], [[name, number(details[name]['candidate_family_hertz']), f"{measurements[name]['periodicity']['autocorrelation_median_hertz']:.2f} / {measurements[name]['periodicity']['autocorrelation_median_confidence']:.2f}", number(measurements[name]['periodicity']['tracked_dominant_median_hertz']), number(measurements[name]['periodicity']['tracked_dominant_10_90_cents']), number(measurements[name]['periodicity']['tracked_dominant_drift_cents_per_second'])] for name in loop_order])
append('''The approximately 32.75/61.86/68.35/93.15/127.75 Hz families progress upward, but these are acoustic periodicities, not independently measured crankshaft speed. Idle strongly favors the fifth/tenth family members near 164/328 Hz; autocorrelation therefore selects approximately 163 Hz instead of 32.75 Hz. Low and medium favor their tenth member near 619/684 Hz; high favors approximately the fifth near 466 Hz; maximum emphasizes the first near 128 Hz. This change of harmonic dominance is a large part of the bank’s identity. Do not make every layer’s spectrum identical or infer RPM from its loudest peak.

The candidate spacing is a diagnostic fit to the resolved harmonic family, with individual deviations recorded in `diagnostic_details.json`; weak/inharmonic lines and time variation prevent a unique physical-fundamental claim. The existing manifest’s comb calibration is context, not ground truth. Its own stored confidence values are low. No sample-rate or pitch rewrite is warranted from this analysis alone. Preserve slow natural modulation, and evaluate any beats at actual transition playback rates before changing reference anchors.

Above 8 kHz, even maximum carries only about 0.19% of Welch power; high carries about 0.053%. The original upper-RPM distortion produces harmonic extension that may contribute aggression. The spectra do not establish objectionable aliasing. A static recording cannot reliably distinguish aliased products from intentional harmonics or recording artifacts without a source reference or playback-rate experiment. Narrow high-frequency peaks are retained as diagnostic candidates in JSON, not automatically labeled whistles to remove.

![Engine-layer comparison](figures/engine_layer_comparison.png)
''''')

append('## 4. Adjacent RPM-layer comparisons')
append('Each raw source was analytically resampled to the existing manifest’s transition-center RPM, then trimmed by 50 ms at each edge before measurement. Resampling was band-limited FFT interpolation in memory; no audio was saved. These are source-to-source comparisons, not validation of the prepared runtime loops, filters, envelopes, or mixer. Existing calibrated gains were excluded so that source inconsistencies remain visible.')
table(['Transition', 'Center RPM', 'Playback rates A / B', 'B−A LU', 'B−A RMS dB', 'Existing relative gain B/A dB'], [[comparison['first'] + ' → ' + comparison['second'], number(comparison['boundary_revolutions_per_minute'], 0), f"{comparison['summaries'][0]['playback_rate']:.3f} / {comparison['summaries'][1]['playback_rate']:.3f}", number(comparison['second_minus_first_loudness_units']), number(comparison['summaries'][1]['root_mean_square_decibels_full_scale'] - comparison['summaries'][0]['root_mean_square_decibels_full_scale']), number(comparison['calibrated_gain_difference_decibels'])] for comparison in analysis['adjacent_layer_comparisons']])
table(['Transition', '80–150 Hz ΔdB', '150–300 Hz ΔdB', '300–600 Hz ΔdB', '600–1200 Hz ΔdB', '1.2–2.5 kHz ΔdB', '2.5–5 kHz ΔdB'], [[comparison['first'] + ' → ' + comparison['second']] + [number(comparison['loudness_matched_region_difference_decibels'][region]) for region in ['80-150', '150-300', '300-600', '600-1200', '1200-2500', '2500-5000']] for comparison in analysis['adjacent_layer_comparisons']])
append('''Band differences above are **after compensating each pair’s LUFS difference**, at the common RPM. They are diagnostic contrasts, not required EQ gains. Large ratios against an almost absent band, especially idle’s low-order energy, do not justify equally large boosts/cuts.

- Idle → low: overall loudness already matches closely at the boundary, but low adds bass/low-order harmonics and loses upper content. This harmonic-envelope transition is more important than the native-pitch loudness difference. Do not boost idle’s missing bass by 21 dB.
- Low → medium: about +1.28 LU before calibration; modest level adjustment may suffice, with conservative control of the moving 124/137 Hz components if needed.
- Medium → high: about +5.73 LU before calibration. Correct gain first. The residual +2.90 dB in 300–600 Hz supports a small optional cut around high’s 466 Hz source harmonic, not a broadband treble cut.
- High → maximum: approximately −0.49 LU overall, yet maximum adds +8.67 dB in the 80–150 Hz boundary band. Its strong 128 Hz source component should be retained, with at most shallow dynamic control if it makes the handoff boomy.

The existing manifest already attenuates high relative to medium by 4.47 dB, and encodes a designed 1.5 dB transition-level progression. Do not add the proposed source trims on top of old calibration without recomputing that calibration. Matching source storage levels and preserving the runtime RPM loudness curve are separate tasks.

Across 32 circular relative offsets per pair, equal-RMS equal-power midpoint estimates vary by roughly −0.64 to +0.60 dB. These whole-overlap estimates do not bound short-time comb filtering, beating, or peak overshoot. Both level and phase behavior must be measured through the actual runtime crossfade at its start, center and end. Keep additional bus headroom for correlated overlap and event voices.''')

append('## 5. Raw loop boundaries')
table(['Loop', 'End→start jump dBFS', 'Jump / internal P99 step', 'Tail−head 20 ms level dB', '20 ms tail/head correlation'], [[name, number(measurements[name]['seam']['jump_decibels_full_scale']), number(measurements[name]['seam']['jump_relative_to_internal_difference_99_percentile']), number(measurements[name]['seam']['last_minus_first_20_millisecond_level_decibels']), number(measurements[name]['seam']['first_last_20_millisecond_correlation'], 3)] for name in loop_order])
append('''There are no embedded `smpl` loop markers in the source WAVs. These figures test the full-file raw join, not the loops prepared by the existing build script. Medium’s 0.816 FS jump is 9.57 times its internal P99 adjacent-sample difference; low’s 0.230 FS jump is 3.23 times. They need deliberate loop construction. High/maximum have smaller endpoint jumps, but opposite-phase or mismatched waveform neighborhoods can still create a spectral seam. Idle and maximum have negative 20 ms edge correlations, so a blind overlap can create a dip.

For a future derivative, search start/end windows for similar waveform phase, slope, local RMS, and harmonic spectrum after DC correction. Evaluate 10–30 ms overlaps as an initial search range, extending only if necessary for the low-order period. This range covers roughly one or more prominent cycles without a long amplitude fade; it is not a guaranteed recipe. Pick the overlap law from correlation: linear weights suit coherent aligned material, equal-power weights suit decorrelated material. Check the interior splice as well as the final wrap. Preserve as much stable material and natural modulation as possible. Require no visible exceptional derivative jump, ≤0.5 dB local envelope disturbance as an engineering target, and no click or periodic dip in repeated listening. Smaller endpoint numbers alone do not certify a seamless loop.

The eight events are not loops. Backfires and gear-up reach zero endpoints; gear-down starts near −62 dBFS and ends at zero. Preserve their tails. The limiter ends at +0.07431 FS (approximately −22.6 dBFS), so an immediate stop to silence creates a real endpoint step. In a future derivative, trial a 3–5 ms fade to zero at its tail, then verify that the final burst is retained; this is an edge repair, not compression of its interior bursts.''')

append('## 6. Proposed EQ and filtering: source-rate settings')
append('All values below are conservative starting points for a later non-destructive pass, not measured optimal settings. EQ frequencies refer to native 44.1 kHz source playback; they move with playback rate. “Conditional” means the default is bypass until a matched comparison confirms the measured feature is objectionable. There is **no justified reinforcement**: all boost gains start at 0 dB. Retain low-order body, 300–1200 Hz harmonic structure, and the original upper-RPM rasp.')
table(['Source', 'Recommendation (Hz, Q, dB)', 'Measured justification / preserve'], [
['98_int_idle.wav', 'Static EQ bypass. Conditional dynamic bell 328 Hz, Q 1.5, 0 to −1.5 dB.', '300–600 Hz carries 78.20% of power and varies 6.24 dB. Reduce only excess peaks; preserve the main 164/328/493 Hz family and idle unevenness.'],
['98_int_low.wav', 'Remove +2.078% DC. Conditional dynamic bell 124 Hz, Q 2.0, 0 to −1.5 dB.', '80–150 Hz envelope varies 9.06 dB. Preserve approximately 62/186/248/619 Hz; avoid weakening the entire bass range.'],
['98_int_med.wav', 'Remove +0.396% DC. Conditional dynamic bell 137 Hz, Q 2.0, 0 to −1.5 dB.', '80–150 Hz envelope varies 9.43 dB. Preserve 68/342/684 Hz character; fix the loop seam first.'],
['98_int_high_1.wav', 'Remove +1.183% DC and calibrate level first. Conditional bell 466 Hz, Q 1.2, −1.0 dB; do not exceed −1.5 dB initially.', 'At aligned medium→high transition, 300–600 Hz is +2.90 dB after loudness matching. Prefer dynamic 0 to −1 dB if only local peaks offend. Preserve baked saturation and approximately 934 Hz.'],
['98_int_max_5.wav', 'Remove +0.809% DC and calibrate level first. Conditional dynamic bell 128 Hz, Q 2.0, 0 to −1.5 dB.', '80–150 Hz carries 26.85% of power, varies 4.85 dB, and increases +8.67 dB at the boundary. Preserve the fundamental rather than fully equalizing this difference.'],
['500_backfire3.wav', 'EQ bypass. Optional high-pass 20 Hz, Q 0.707, 12 dB/oct if subsonic excursion matters.', '6.77% below 40 Hz, including a 32 Hz component. Preserve 73/127 Hz impact and 300–1200 Hz burst texture.'],
['500_backfire4.wav', 'EQ bypass. Optional high-pass 20 Hz, Q 0.707, 12 dB/oct.', '2.89% below 40 Hz; preserve the 135/291 Hz body. No measured need for a resonant notch.'],
['500_backfire5.wav', 'EQ bypass. Optional high-pass 20 Hz, Q 0.707, 12 dB/oct.', '1.93% below 40 Hz; 300–600 Hz is naturally dominant. Preserve 155/309/563 Hz and transient crest.'],
['500_backfire6.wav', 'EQ bypass. Optional high-pass 20 Hz, Q 0.707, 12 dB/oct.', '3.94% below 40 Hz; preserve 145 Hz thump and 600–1200 Hz crack.'],
['500_backfire7.wav', 'EQ bypass; no automatic bass cut. High-pass normally bypassed.', '29.27% in 40–80 Hz and a 46 Hz dominant peak are part of its distinct event body. This variant already has headroom.'],
['500_limiter.wav', 'EQ bypass; any high-pass trial limited to 20 Hz, Q 0.707, 12 dB/oct.', 'Preserve interruption rhythm and burst structure; treat as an event, not a smooth sustained loop.'],
['geardn.wav', 'EQ and filters bypass by default.', '50.82% of Welch power lies in 40–80 Hz, with a 52 Hz peak. A conventional 80 Hz cleanup filter would remove the event itself.'],
['gearup.wav', 'EQ and filters bypass by default.', 'Distinct 47/128/182 Hz impact with already negligible treble; no evidence supporting a brightness boost.']
])
append('''For loops, remove a constant mean before deciding whether any high-pass is useful. If residual low-frequency drift remains problematic, trial **20 Hz, Q 0.707, 12 dB/oct** only on low/medium/high/maximum. The ideal second-order response at 62 Hz is approximately −0.05 dB, so this is deliberately below their measured family spacing. Idle needs no high-pass: it has almost no sub-20 Hz energy and the weakest inferred 33 Hz family deserves protection. In a one-shot, blindly subtracting its whole-file mean can create nonzero tails; use padded filtering and inspect endpoints if correction is needed.

**Low-pass recommendation: bypass for every file initially.** There is no measured blanket hiss problem. If listening confirms objectionable upper noise specifically in high/maximum, compare a gentle **12,000 Hz low-pass, Q 0.707, 12 dB/oct**, or an **8,000 Hz high shelf, Q 0.707, −1 dB maximum** as separate alternatives; retain only one if it improves the result without dulling the V10. These are conditional trials justified by their relatively greater upper content, not mandatory processing. Do not apply them to all files or attempt to “repair aliasing” by assumption. Runtime anti-alias filtering for playback above unity is a separate resampling requirement and must be checked at the maximum actual playback rate.''')

append('## 7. Compression, dynamic EQ, and noise reduction')
append('Default broadband compression is bypass, including all eight events and the already flattened high/maximum layers. Low and medium have under 2 dB sustained broadband variation; there is no justification for a global compressor. Do not normalize crest factor across the bank: that would flatten low/medium to resemble previously saturated sources.')
table(['Use case', 'Threshold / detector', 'Ratio', 'Attack / release', 'Makeup', 'Reduction limit / condition'], [
['Optional idle full-band alternative', '−12 dBFS, approximately 100 ms RMS detector, source before normalization; soft knee 3 dB', '1.4:1', '40 / 180 ms', '0 dB', '≤1.5 dB reduction; compare against tonal dynamic EQ, do not stack both. Chosen around its upper sustained envelope; leave troughs alone.'],
['Idle dynamic bell at 328 Hz', 'About −13.6 dBFS in a 300–600 Hz RMS detector (measured P75)', '1.5:1', '40 / 180 ms', '0 dB', '0 to −1.5 dB; do not use an upward expander to fill the troughs.'],
['Low dynamic bell at 124 Hz', 'About −21.6 dBFS in an 80–150 Hz RMS detector (P75)', '1.5:1', '30 / 120 ms', '0 dB', '0 to −1.5 dB, conditional on audible bass fluctuation.'],
['Medium dynamic bell at 137 Hz', 'About −19.4 dBFS in an 80–150 Hz RMS detector (P75)', '1.5:1', '30 / 120 ms', '0 dB', '0 to −1.5 dB, conditional on audible bass fluctuation.'],
['High dynamic alternative at 466 Hz', 'About −8.1 dBFS in a 300–600 Hz RMS detector (P75)', '1.3:1', '40 / 160 ms', '0 dB', '0 to −1 dB; use instead of the static bell. No full-band compression.'],
['Maximum dynamic bell at 128 Hz', 'About −10.3 dBFS in an 80–150 Hz RMS detector (P75)', '1.5:1', '30 / 150 ms', '0 dB', '0 to −1.5 dB. Prefer slower ≤1 dB subtractive gain automation for a global rise.'],
['Backfires, limiter, gear events', 'Bypass / threshold inactive', '1:1', 'Inactive', '0 dB', 'Use gain trim for headroom. Crest and decay are meaningful event characteristics.']
])
append('''Thresholds refer to source levels after DC correction, before final gain matching. They are not transferable unchanged after a trim: add the trim in dB to the threshold, and recalibrate against the processor’s detector. The detector bands above are wider diagnostic bands, distinct from the bell bandwidth. If a processor cannot use such an external detector, set the threshold from its own P75 measurement. Attack/release starting points are several tonal cycles long and target measured 0.1–0.4 s envelope movement rather than individual combustion pulses. No compression settings here have been auditioned or optimized.

**Prefer dynamic EQ to multiband compression** for the isolated 124/137/128 Hz movements or idle’s dominant tonal band. Dynamic EQ limits the amount of harmonic change and avoids splitting the full signal into multiple crossover bands. Multiband compression is not presently justified: no independent broad-band imbalance requires it. Global de-essing, excitation, saturation, stereo widening and transient sharpening are also unsupported.

**Noise reduction: bypass.** The low-level tails contain wanted decay, and the sustained files contain no independently identifiable silence. Do not train a denoiser on an idle trough or backfire tail, hard-gate the engine loops, or use spectral subtraction simply because the spectrogram has a broadband floor. A future confirmed fixed whistle could receive a narrow dynamic notch (center and Q measured from that line’s bandwidth), but the present evidence does not justify specifying or applying one. Baked nonlinear distortion cannot be reliably removed as “noise.”''')

append('## 8. Level targets and gain proposals')
append('For loop derivatives, a provisional common native-pitch storage target is **−16 LUFS-I mono**, with estimated true peak **≤−4 dBFS** and no mandatory limiter. This target is derived from the existing crest/headroom: low is the limiting layer, requiring approximately −15.94 LUFS or lower to leave 4 dB reconstructed-peak headroom. Rounded down, −16 LUFS achieves that for all five by gain alone. This is an asset-calibration reference, not a final playback or broadcast target. Recompute after EQ, any dynamics, and final loop construction.')
table(['Loop', 'Native mono LUFS-I', 'Initial gain to −16 LUFS, dB', 'Predicted estimated TP after gain, dBFS'], [[name, number(measurements[name]['integrated_loudness_loudness_units_full_scale']), number(-16 - measurements[name]['integrated_loudness_loudness_units_full_scale']), number(measurements[name]['estimated_true_peak_decibels_full_scale'] - 16 - measurements[name]['integrated_loudness_loudness_units_full_scale'])] for name in loop_order])
append('These are gain-only predictions, not processed-file results. At runtime, recalibrate adjacent layers at common RPM with the selected pitch shifter, seam, source trim and existing loudness progression. Aim for ≤0.5 LU unintended handoff error after accounting for the designed gain curve. Maintain source headroom; verify the bus under worst-case loop overlap and event concurrency, with a proposed final output ceiling of −1 dBTP. Do not solve a source mismatch by continually driving a bus limiter.')
append('For event assets, use an **attenuation-only estimated peak ceiling of −3 dBFS**, preserving existing quieter variants. Do not normalize every event upward to the same peak or force all events to engine LUFS. The following gain-only trims bring backfire whole-clip mono K levels to approximately −21.4 to −20.1, already a relatively narrow range without compression.')
table(['Event', 'Proposed initial trim dB', 'Predicted est. TP dBFS', 'Predicted mono ungated K level'], [[name, number(trim), number(entry['estimated_true_peak_decibels_full_scale'] + trim), number(details[name]['mono_ungated_weighted_level'] + trim)] for name in event_order for entry in [measurements[name]] for trim in [min(0, -3 - entry['estimated_true_peak_decibels_full_scale'])]])
append('Backfire 7 keeps its existing gain: raising it to the others’ near-zero raw peaks would overemphasize its dense bass. Preserve the distinct limiter/upshift/downshift mix roles and set event-to-engine balance in context. The −3 dBFS ceiling is an initial per-voice headroom choice, not proof that overlapping events cannot clip the final bus.')

append('## 9. Non-destructive remaster chain and validation gates')
append('''1. Freeze this source inventory and preserve original WAV bytes and metadata. Write every future derivative and recipe to a separate versioned bank. Use float processing; choose mono derivatives only after preserving the demonstrated dual-mono behavior.
2. Apply constant DC correction to sustained loops. For events, avoid introducing endpoint steps; only filter demonstrated offset/rumble with boundary padding and inspect the tails.
3. Apply only accepted, sample-specific EQ. Start with all optional filters and boosts bypassed. Level-match listening before calling a change an improvement.
4. If required, use the shallow dynamic EQ controls above. Preserve attacks, idle movement, low-order harmonics and the saturated high-RPM character. No global compressor, denoiser or de-clip pass.
5. Construct and validate final engine loops after processing, using matched phase/slope/envelope windows. Warm up stateful processors on repeated material or handle their filter/envelope state circularly so an artificial start-up transient is not baked into a loop. Check all internal edits, wraps and repeated cycles. Do not loop the events.
6. Re-measure native LUFS/peaks, apply the measured gain trims, then remeasure at actual RPM transition rates and recalculate runtime gains. Retain the authored RPM progression and event roles. Avoid cumulative old/new gain compensation.
7. Quantize once at export; if keeping PCM16, use appropriate dither only at that final conversion. Retain a float intermediate. Confirm derivative clipping, DC, spectra, harmonic spacing, duration, seam and source hashes; no arbitrary peak limiting to reach loudness targets.
8. Validate actual runtime crossfades, fast and slow RPM sweeps, limiter entry, shifts, lift/backfire triggers, anti-alias behavior at the largest playback multiplier, and worst-case simultaneous voices. Compare at matched perceived loudness. A human listening gate must confirm the same V10 identity and improved continuity before any source replacement or runtime promotion.

Priority order: (1) gain/DC and low/medium loop seams, (2) high/maximum baked saturation handled conservatively, (3) idle tonal-envelope control if needed, (4) bass-band handoffs, (5) optional upper-band cleanup only if listening confirms a problem. No claim is made that an analysis-only report has completed these future processing or listening gates.
''')

append('## 10. Per-file diagnostic plots and reproducibility')
for name in ordered_names:
    append(f'- [{name}: waveform, envelope, spectrum, spectrogram, boundary and pitch/transient plot](figures/{Path(name).stem}.png)')
append('Measurements: [full structured JSON](measurements.json), [scalar CSV](measurements.csv), [diagnostic detail JSON](diagnostic_details.json). Reproduce using `analyze_source_audio.py`, then `analyze_diagnostic_details.py`, then `write_technical_report.py`. Package versions are recorded above. The analysis scripts read WAVs and create numeric/visual artifacts only; they contain no audio-export operation.')
table(['File', 'SHA-256, unchanged'], [[name, measurements[name]['sha256_before']] for name in ordered_names])

report = '\n'.join(paragraphs)
(analysis_directory / 'technical_report.md').write_text(report, encoding='utf-8')
body = markdown.markdown(report, extensions=['tables', 'fenced_code', 'toc'])
stylesheet = 'body{font:16px/1.55 system-ui,sans-serif;color:#202b38;background:#f7f9fc;margin:0}main{max-width:1400px;margin:auto;background:white;padding:44px}h1{font-size:34px}h2{margin-top:48px;color:#173a57}table{border-collapse:collapse;width:100%;font-size:13px;display:block;overflow:auto;margin:22px 0}th,td{padding:9px 11px;text-align:left;border-bottom:1px solid #dde4ec;vertical-align:top}th{background:#eaf0f7}tr:nth-child(even){background:#f8fafc}img{max-width:100%;height:auto}code{font-size:.88em;background:#eef2f6;overflow-wrap:anywhere}a{color:#0964ad}li{margin:7px 0}@media print{main{padding:0}body{background:white}h2{break-after:avoid}img, tr{break-inside:avoid}}'
(analysis_directory / 'technical_report.html').write_text('<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>V10 GP3 audio analysis</title><style>' + stylesheet + '</style><main>' + body + '</main></html>', encoding='utf-8')
print(f'Report written: {len(report.split())} words; {len(measurements)} files; {len(list((analysis_directory / "figures").glob("*.png")))} plots.')

