from pathlib import Path
import sys

analysis_directory = Path(__file__).resolve().parent
sys.path.insert(0, str(analysis_directory / 'python-packages'))

import csv
import hashlib
import importlib.metadata
import json
import platform
import struct
import numpy
import scipy
from scipy import signal
import soundfile
import pyloudnorm
import matplotlib
matplotlib.use('Agg')
from matplotlib import pyplot

source_directory = Path('D:/Formula90s/game/sounds/banks/v10-gp3')
manifest_path = Path('D:/Formula90s/game/audio/formula_one_2030_grand_prix_sampler/manifest.json')
manifest = json.loads(manifest_path.read_text())
loop_definitions = {entry['source_filename']: entry for entry in manifest['loops']}
frequency_regions = [(0, 20), (20, 40), (40, 80), (80, 150), (150, 300), (300, 600), (600, 1200), (1200, 2500), (2500, 5000), (5000, 8000), (8000, 12000), (12000, 16000), (16000, 20000), (20000, 24000)]
figure_directory = analysis_directory / 'figures'
figure_directory.mkdir(exist_ok=True)


def amplitude_decibels(values):
    return 20 * numpy.log10(numpy.maximum(numpy.abs(values), 1e-15))


def power_decibels(values):
    return 10 * numpy.log10(numpy.maximum(values, 1e-30))


def root_mean_square(values):
    return float(numpy.sqrt(numpy.mean(numpy.square(values))))


def framed_values(samples, sample_rate, duration_seconds, hop_seconds):
    frame_length = max(1, round(duration_seconds * sample_rate))
    hop_length = max(1, round(hop_seconds * sample_rate))
    if len(samples) < frame_length:
        return samples[numpy.newaxis, :], numpy.array([len(samples) / (2 * sample_rate)])
    frames = numpy.lib.stride_tricks.sliding_window_view(samples, frame_length)[::hop_length]
    times = (numpy.arange(len(frames)) * hop_length + frame_length / 2) / sample_rate
    return frames, times


def wave_chunks(path):
    payload = path.read_bytes()
    position = 12
    chunks = []
    embedded_loops = []
    while position + 8 <= len(payload):
        chunk_name = payload[position:position + 4].decode('ascii', errors='replace')
        chunk_length = struct.unpack_from('<I', payload, position + 4)[0]
        chunks.append({'name': chunk_name, 'bytes': chunk_length})
        if chunk_name == 'smpl' and chunk_length >= 36:
            loop_count = struct.unpack_from('<I', payload, position + 8 + 28)[0]
            for loop_index in range(loop_count):
                loop_position = position + 8 + 36 + loop_index * 24
                if loop_position + 24 <= position + 8 + chunk_length:
                    embedded_loops.append(list(struct.unpack_from('<6I', payload, loop_position)))
        position += 8 + chunk_length + chunk_length % 2
    return chunks, embedded_loops


def integrated_loudness(samples, sample_rate):
    if len(samples) < round(sample_rate * 0.4):
        return None
    return float(pyloudnorm.Meter(sample_rate).integrated_loudness(samples))


def weighted_samples(samples, sample_rate):
    meter = pyloudnorm.Meter(sample_rate)
    filtered = samples.copy()
    for filter_stage in meter._filters.values():
        filtered = filter_stage.apply_filter(filtered)
    return filtered


def spectral_measurements(samples, sample_rate):
    frequencies, spectrum = signal.welch(samples, sample_rate, window='hann', nperseg=min(16384, len(samples)), noverlap=None, detrend=False)
    frequency_step = frequencies[1] - frequencies[0]
    total_power = float(numpy.sum(spectrum) * frequency_step)
    regions = {}
    for lower, upper in frequency_regions:
        selection = (frequencies >= lower) & (frequencies < min(upper, sample_rate / 2 + frequency_step))
        region_power = float(numpy.sum(spectrum[selection]) * frequency_step)
        regions[f'{lower}-{upper}'] = {'power_percent': 100 * region_power / max(total_power, 1e-30), 'level_decibels_full_scale': float(power_decibels(region_power))}
    candidates, properties = signal.find_peaks(power_decibels(spectrum), prominence=6, distance=max(1, round(15 / frequency_step)))
    candidates = [int(index) for index in candidates if frequencies[index] >= 20]
    candidates.sort(key=lambda index: spectrum[index], reverse=True)
    dominant_peaks = []
    spectrum_levels = power_decibels(spectrum)
    for peak_index in candidates[:15]:
        widths = signal.peak_widths(spectrum, [peak_index], rel_height=0.5)[0]
        dominant_peaks.append({'frequency_hertz': float(frequencies[peak_index]), 'density_decibels_full_scale_per_hertz': float(spectrum_levels[peak_index]), 'half_power_width_hertz': float(widths[0] * frequency_step)})
    cumulative = numpy.cumsum(spectrum) / max(numpy.sum(spectrum), 1e-30)
    positive = (frequencies >= 20) & (frequencies <= min(20000, sample_rate / 2))
    return {'regions': regions, 'centroid_hertz': float(numpy.sum(frequencies * spectrum) / numpy.sum(spectrum)), 'rolloff_95_hertz': float(frequencies[numpy.searchsorted(cumulative, 0.95)]), 'rolloff_99_hertz': float(frequencies[numpy.searchsorted(cumulative, 0.99)]), 'flatness_decibels': float(power_decibels(numpy.exp(numpy.mean(numpy.log(spectrum[positive] + 1e-30))) / numpy.mean(spectrum[positive]))), 'dominant_peaks': dominant_peaks}, frequencies, spectrum


def periodicity_measurements(samples, sample_rate, dominant_frequency):
    analysis_rate = 12000
    divisor = numpy.gcd(sample_rate, analysis_rate)
    resampled = signal.resample_poly(samples, analysis_rate // divisor, sample_rate // divisor)
    frames, times = framed_values(resampled, analysis_rate, 0.16, 0.02)
    periodic_frequencies = []
    confidences = []
    for frame in frames:
        centered = frame - numpy.mean(frame)
        autocorrelation = signal.fftconvolve(centered, centered[::-1], mode='full')[len(frame) - 1:]
        minimum_lag = round(analysis_rate / 2000)
        maximum_lag = min(round(analysis_rate / 25), len(frame) - 2)
        autocorrelation /= max(autocorrelation[0], 1e-30)
        peak_positions, unused = signal.find_peaks(autocorrelation[minimum_lag:maximum_lag])
        peak_positions += minimum_lag
        if len(peak_positions) == 0:
            periodic_frequencies.append(float('nan'))
            confidences.append(0)
            continue
        best_value = max(autocorrelation[peak_positions])
        selected = peak_positions[autocorrelation[peak_positions] >= max(0.35, 0.9 * best_value)]
        selected_lag = int(selected[0] if len(selected) else peak_positions[numpy.argmax(autocorrelation[peak_positions])])
        interpolation = 0.5 * (autocorrelation[selected_lag - 1] - autocorrelation[selected_lag + 1]) / (autocorrelation[selected_lag - 1] - 2 * autocorrelation[selected_lag] + autocorrelation[selected_lag + 1])
        periodic_frequencies.append(float(analysis_rate / (selected_lag + interpolation)))
        confidences.append(float(autocorrelation[selected_lag]))
    original_frames, original_times = framed_values(samples, sample_rate, 0.16, 0.02)
    transform_length = 2 ** int(numpy.ceil(numpy.log2(original_frames.shape[1] * 4)))
    transform_frequencies = numpy.fft.rfftfreq(transform_length, 1 / sample_rate)
    spectrum = numpy.abs(numpy.fft.rfft(original_frames * numpy.hanning(original_frames.shape[1]), n=transform_length, axis=1))
    selected_band = numpy.where((transform_frequencies > dominant_frequency * 0.96) & (transform_frequencies < dominant_frequency * 1.04))[0]
    tracked = []
    for frame_spectrum in spectrum:
        peak_index = int(selected_band[numpy.argmax(frame_spectrum[selected_band])])
        logarithms = numpy.log(frame_spectrum[peak_index - 1:peak_index + 2] + 1e-20)
        interpolation = 0.5 * (logarithms[0] - logarithms[2]) / (logarithms[0] - 2 * logarithms[1] + logarithms[2])
        tracked.append(float((peak_index + interpolation) * sample_rate / transform_length))
    tracked = numpy.array(tracked)
    central = (original_times >= len(samples) / sample_rate * 0.1) & (original_times <= len(samples) / sample_rate * 0.9)
    if not numpy.any(central):
        central = numpy.ones(len(tracked), dtype=bool)
    center_frequency = float(numpy.median(tracked[central]))
    cents = 1200 * numpy.log2(tracked[central] / center_frequency)
    confident_periods = numpy.array(periodic_frequencies)[numpy.array(confidences) >= 0.65]
    result = {'autocorrelation_median_hertz': float(numpy.nanmedian(periodic_frequencies)), 'autocorrelation_median_confidence': float(numpy.median(confidences)), 'autocorrelation_confident_frame_percent': float(numpy.mean(numpy.array(confidences) >= 0.65) * 100), 'autocorrelation_confident_frequency_10_90_hertz': numpy.percentile(confident_periods, [10, 90]).tolist() if len(confident_periods) else None, 'tracked_dominant_median_hertz': center_frequency, 'tracked_dominant_10_90_cents': float(numpy.percentile(cents, 90) - numpy.percentile(cents, 10)), 'tracked_dominant_drift_cents_per_second': float(numpy.polyfit(original_times[central], cents, 1)[0]) if sum(central) > 1 else None, 'periodicity_frequency_hertz': periodic_frequencies, 'periodicity_confidence': confidences, 'times_seconds': original_times.tolist(), 'tracked_dominant_frequency_hertz': tracked.tolist()}
    return result


def analyze_file(path):
    before_hash = hashlib.sha256(path.read_bytes()).hexdigest()
    channels, sample_rate = soundfile.read(path, dtype='float64', always_2d=True)
    information = soundfile.info(path)
    samples = numpy.mean(channels, axis=1)
    duration = len(samples) / sample_rate
    loop = path.name in loop_definitions
    peak = float(numpy.max(numpy.abs(channels)))
    average = root_mean_square(channels)
    oversampled = signal.resample_poly(channels, 8, 1, axis=0, window=('kaiser', 12.0), padtype='line')
    estimated_true_peak = float(numpy.max(numpy.abs(oversampled)))
    frames, times = framed_values(samples, sample_rate, 0.02, 0.005)
    frame_levels = amplitude_decibels(numpy.sqrt(numpy.mean(frames ** 2, axis=1)))
    stable_frames, stable_times = framed_values(samples, sample_rate, 0.1, 0.01)
    stable_levels = amplitude_decibels(numpy.sqrt(numpy.mean(stable_frames ** 2, axis=1)))
    sustained = (stable_times >= duration * 0.1) & (stable_times <= duration * 0.9)
    sustained_levels = stable_levels[sustained]
    if len(sustained_levels) == 0:
        sustained_levels = stable_levels
    quietest = int(numpy.argmin(frame_levels))
    weighted = weighted_samples(samples, sample_rate)
    short_frames, short_times = framed_values(weighted, sample_rate, min(0.1, duration), 0.01)
    short_loudness = -0.691 + power_decibels(numpy.mean(short_frames ** 2, axis=1))
    spectral, frequencies, spectrum = spectral_measurements(samples, sample_rate)
    chunks, embedded_loops = wave_chunks(path)
    rail_samples = (channels <= -1.0) | (channels >= 32767 / 32768)
    differences = numpy.diff(samples)
    same_value = numpy.diff(channels, axis=0) == 0
    elevated_same = same_value & (numpy.abs(channels[:-1]) >= 0.5)
    changes = numpy.diff(numpy.r_[False, elevated_same[:, 0], False].astype(int))
    run_lengths = numpy.where(changes == -1)[0] - numpy.where(changes == 1)[0] + 1
    edge_length = max(1, round(sample_rate * 0.02))
    seam_jump = abs(samples[0] - samples[-1])
    edge_correlation = float(numpy.corrcoef(samples[:edge_length], samples[-edge_length:])[0, 1])
    edge_spectrum_first = spectral_measurements(samples[:max(edge_length, round(sample_rate * 0.1))], sample_rate)[0]
    edge_spectrum_last = spectral_measurements(samples[-max(edge_length, round(sample_rate * 0.1)):], sample_rate)[0]
    cumulative_energy = numpy.cumsum(samples ** 2)
    cumulative_energy /= max(cumulative_energy[-1], 1e-30)
    energy_times = {str(percent): float(numpy.searchsorted(cumulative_energy, percent / 100) / sample_rate) for percent in (5, 50, 95)}
    result = {'filename': path.name, 'sha256_before': before_hash, 'sample_rate_hertz': sample_rate, 'channels': information.channels, 'encoding': information.subtype, 'frame_count': len(samples), 'duration_seconds': duration, 'role': 'engine_loop' if loop else 'one_shot', 'wave_chunks': chunks, 'embedded_loops': embedded_loops, 'sample_peak_decibels_full_scale': float(amplitude_decibels(peak)), 'estimated_true_peak_decibels_full_scale': float(amplitude_decibels(estimated_true_peak)), 'root_mean_square_decibels_full_scale': float(amplitude_decibels(average)), 'integrated_loudness_loudness_units_full_scale': integrated_loudness(channels if information.channels > 1 else samples, sample_rate), 'whole_clip_ungated_weighted_level': float(-0.691 + power_decibels(numpy.mean(weighted ** 2))), 'maximum_100_millisecond_weighted_level': float(numpy.max(short_loudness)), 'crest_factor_decibels': float(amplitude_decibels(peak / average)), 'dynamic_range_20_millisecond_95_minus_10_decibels': float(numpy.percentile(frame_levels, 95) - numpy.percentile(frame_levels, 10)), 'sustained_100_millisecond_90_minus_10_decibels': float(numpy.percentile(sustained_levels, 90) - numpy.percentile(sustained_levels, 10)), 'sustained_100_millisecond_standard_deviation_decibels': float(numpy.std(sustained_levels)), 'sustained_100_millisecond_median_decibels_full_scale': float(numpy.median(sustained_levels)), 'sustained_100_millisecond_90_percentile_decibels_full_scale': float(numpy.percentile(sustained_levels, 90)), 'direct_current_offset_signed': numpy.mean(channels, axis=0).tolist(), 'direct_current_offset_decibels_full_scale': amplitude_decibels(numpy.mean(channels, axis=0)).tolist(), 'digital_rail_sample_count': int(numpy.sum(rail_samples)), 'near_clip_minus_point_one_decibel_percent': float(numpy.mean(numpy.abs(channels) >= 10 ** (-0.1 / 20)) * 100), 'near_clip_minus_one_decibel_percent': float(numpy.mean(numpy.abs(channels) >= 10 ** (-1 / 20)) * 100), 'high_amplitude_repeated_sample_count': int(numpy.sum(elevated_same)), 'longest_high_amplitude_constant_run_samples': int(max(run_lengths, default=0)), 'minimum_20_millisecond_level_decibels_full_scale': float(numpy.min(frame_levels)), 'minimum_20_millisecond_center_seconds': float(times[quietest]), 'low_10_percentile_20_millisecond_level_decibels_full_scale': float(numpy.percentile(frame_levels, 10)), 'first_20_millisecond_level_decibels_full_scale': float(amplitude_decibels(root_mean_square(samples[:edge_length]))), 'last_20_millisecond_level_decibels_full_scale': float(amplitude_decibels(root_mean_square(samples[-edge_length:]))), 'time_of_absolute_peak_seconds': float(numpy.argmax(numpy.abs(samples)) / sample_rate), 'energy_arrival_seconds': energy_times, 'seam': {'jump_decibels_full_scale': float(amplitude_decibels(seam_jump)), 'jump_relative_to_internal_difference_99_percentile': float(seam_jump / max(numpy.percentile(numpy.abs(differences), 99), 1e-15)), 'first_sample': float(samples[0]), 'last_sample': float(samples[-1]), 'first_last_20_millisecond_correlation': edge_correlation, 'last_minus_first_20_millisecond_level_decibels': float(amplitude_decibels(root_mean_square(samples[-edge_length:]) / max(root_mean_square(samples[:edge_length]), 1e-15))), 'first_100_millisecond_centroid_hertz': edge_spectrum_first['centroid_hertz'], 'last_100_millisecond_centroid_hertz': edge_spectrum_last['centroid_hertz']}, 'spectral': spectral}
    if loop:
        dominant_frequency = spectral['dominant_peaks'][0]['frequency_hertz']
        result['periodicity'] = periodicity_measurements(samples, sample_rate, dominant_frequency)
        result['runtime_reference'] = loop_definitions[path.name]
    else:
        result['periodicity'] = None
    if information.channels > 1:
        result['stereo_correlation'] = float(numpy.corrcoef(channels.T)[0, 1])
        result['stereo_side_relative_to_mid_decibels'] = float(amplitude_decibels(root_mean_square((channels[:, 0] - channels[:, 1]) / 2) / max(root_mean_square(samples), 1e-15)))
    figure, axes = pyplot.subplots(3, 2, figsize=(15, 11))
    figure.suptitle(f'{path.name} | {sample_rate} Hz | {information.subtype} | {duration:.3f} s', fontsize=16)
    axes[0, 0].plot(numpy.arange(len(samples)) / sample_rate, samples, linewidth=0.4)
    axes[0, 0].set(xlabel='Time (s)', ylabel='Amplitude', title='Source waveform', ylim=(-1.08, 1.08))
    axes[0, 1].plot(times, frame_levels, label='20 ms RMS', linewidth=0.8)
    axes[0, 1].plot(stable_times, stable_levels, label='100 ms RMS')
    axes[0, 1].legend()
    axes[0, 1].set(xlabel='Time (s)', ylabel='dBFS', title='Envelope: silence/tails are not a noise estimate')
    axes[1, 0].semilogx(frequencies[1:], power_decibels(spectrum[1:]), linewidth=0.8)
    axes[1, 0].set(xlabel='Frequency (Hz)', ylabel='dBFS / Hz', title='Welch power spectral density', xlim=(20, sample_rate / 2), ylim=(-130, 0))
    spectrogram_frequencies, spectrogram_times, spectrogram_power = signal.spectrogram(samples, sample_rate, nperseg=2048, noverlap=1536)
    axes[1, 1].pcolormesh(spectrogram_times, spectrogram_frequencies, power_decibels(spectrogram_power), vmin=-105, vmax=-25, shading='auto', cmap='magma')
    axes[1, 1].set(xlabel='Time (s)', ylabel='Frequency (Hz)', title='Spectrogram (fixed -105 to -25 dBFS/Hz)', ylim=(0, sample_rate / 2))
    edge_view = round(0.004 * sample_rate)
    axes[2, 0].plot(numpy.arange(-edge_view, 0) / sample_rate * 1000, samples[-edge_view:], label='Tail')
    axes[2, 0].plot(numpy.arange(edge_view) / sample_rate * 1000, samples[:edge_view], label='Head')
    axes[2, 0].axvline(0, color='red', linewidth=0.7)
    axes[2, 0].set(xlabel='Time relative to raw join (ms)', ylabel='Amplitude', title='Raw boundary (not a prepared loop)')
    if loop:
        periodicity = result['periodicity']
        axes[2, 1].plot(periodicity['times_seconds'], periodicity['tracked_dominant_frequency_hertz'])
        axes[2, 1].set(xlabel='Time (s)', ylabel='Frequency (Hz)', title='Dominant line tracking; not automatically engine fundamental')
    else:
        axes[2, 1].plot(numpy.arange(len(samples)) / sample_rate, cumulative_energy)
        axes[2, 1].set(xlabel='Time (s)', ylabel='Cumulative energy', title='Transient energy arrival')
    figure.tight_layout()
    figure.savefig(figure_directory / (path.stem + '.png'), dpi=130)
    pyplot.close(figure)
    result['sha256_after'] = hashlib.sha256(path.read_bytes()).hexdigest()
    return result, samples, frequencies, spectrum


measurements = []
waveforms = {}
spectra = {}
for source_path in sorted(source_directory.rglob('*')):
    if source_path.suffix.lower() not in ('.wav', '.flac', '.ogg', '.aif', '.aiff', '.mp3'):
        continue
    result, samples, frequencies, spectrum = analyze_file(source_path)
    measurements.append(result)
    waveforms[source_path.name] = samples
    spectra[source_path.name] = (frequencies, spectrum)
    print(f"{source_path.name}: peak={result['sample_peak_decibels_full_scale']:.2f}, RMS={result['root_mean_square_decibels_full_scale']:.2f}, loudness={result['integrated_loudness_loudness_units_full_scale']}, crest={result['crest_factor_decibels']:.2f}", flush=True)

comparison_results = []
for transition in manifest['transitions']:
    first_definition = next(entry for entry in manifest['loops'] if entry['id'] == transition['from_loop_id'])
    second_definition = next(entry for entry in manifest['loops'] if entry['id'] == transition['to_loop_id'])
    boundary = transition['center_revolutions_per_minute']
    rendered = []
    summaries = []
    for definition in (first_definition, second_definition):
        filename = definition['source_filename']
        measurement = next(entry for entry in measurements if entry['filename'] == filename)
        sample_rate = measurement['sample_rate_hertz']
        playback_rate = boundary / definition['reference_revolutions_per_minute']
        output_length = round(len(waveforms[filename]) / playback_rate)
        shifted = signal.resample(waveforms[filename], output_length)
        trim = round(sample_rate * 0.05)
        shifted = shifted[trim:-trim]
        spectral, unused_frequencies, unused_spectrum = spectral_measurements(shifted, sample_rate)
        summaries.append({'filename': filename, 'playback_rate': playback_rate, 'loudness': integrated_loudness(shifted, sample_rate), 'root_mean_square_decibels_full_scale': float(amplitude_decibels(root_mean_square(shifted))), 'spectral': spectral})
        rendered.append(shifted)
    loudness_difference = summaries[1]['loudness'] - summaries[0]['loudness']
    broad_differences = {region: summaries[1]['spectral']['regions'][region]['level_decibels_full_scale'] - summaries[0]['spectral']['regions'][region]['level_decibels_full_scale'] - loudness_difference for region in summaries[0]['spectral']['regions']}
    common_length = min(len(rendered[0]), len(rendered[1]))
    first_normalized = rendered[0][:common_length] / root_mean_square(rendered[0][:common_length])
    second_normalized = rendered[1][:common_length] / root_mean_square(rendered[1][:common_length])
    correlations = [float(numpy.corrcoef(first_normalized, numpy.roll(second_normalized, offset))[0, 1]) for offset in numpy.linspace(0, common_length - 1, 32).astype(int)]
    comparison_results.append({'first': first_definition['source_filename'], 'second': second_definition['source_filename'], 'boundary_revolutions_per_minute': boundary, 'summaries': summaries, 'second_minus_first_loudness_units': loudness_difference, 'loudness_matched_region_difference_decibels': broad_differences, 'correlation_32_offsets_min_max': [min(correlations), max(correlations)], 'equal_power_midpoint_change_decibels_min_max': [float(power_decibels(1 + min(correlations))), float(power_decibels(1 + max(correlations)))], 'calibrated_gain_difference_decibels': float(amplitude_decibels(second_definition['calibrated_gain'] / first_definition['calibrated_gain']))})

figure, axes = pyplot.subplots(2, 1, figsize=(15, 10))
for filename in loop_definitions:
    measurement = next(entry for entry in measurements if entry['filename'] == filename)
    frequencies, spectrum = spectra[filename]
    axes[0].semilogx(frequencies[1:], power_decibels(spectrum[1:]) - measurement['root_mean_square_decibels_full_scale'], label=filename, linewidth=0.8)
    axes[1].plot([f'{lower}-{upper}' for lower, upper in frequency_regions], [measurement['spectral']['regions'][f'{lower}-{upper}']['power_percent'] for lower, upper in frequency_regions], marker='o', label=filename)
axes[0].set(xlabel='Frequency (Hz)', ylabel='Relative power density (dB/Hz)', xlim=(20, 22050), ylim=(-100, 0), title='Engine layers normalized by total RMS: narrow lines are often intentional harmonics')
axes[0].legend()
axes[1].set(xlabel='Frequency region (Hz)', ylabel='Percent total power', title='Spectral balance at native source pitch')
axes[1].tick_params(axis='x', rotation=35)
axes[1].legend()
figure.tight_layout()
figure.savefig(figure_directory / 'engine_layer_comparison.png', dpi=140)
pyplot.close(figure)

versions = {package: importlib.metadata.version(package) for package in ('numpy', 'scipy', 'soundfile', 'matplotlib', 'pyloudnorm')}
output = {'python': platform.python_version(), 'libraries': versions, 'source_directory': str(source_directory), 'runtime_manifest_sha256': hashlib.sha256(manifest_path.read_bytes()).hexdigest(), 'measurements': measurements, 'adjacent_layer_comparisons': comparison_results}
(analysis_directory / 'measurements.json').write_text(json.dumps(output, indent=2, allow_nan=False), encoding='utf-8')
scalar_keys = [key for key, value in measurements[0].items() if isinstance(value, (str, int, float)) or value is None]
with (analysis_directory / 'measurements.csv').open('w', newline='', encoding='utf-8') as handle:
    writer = csv.DictWriter(handle, fieldnames=scalar_keys)
    writer.writeheader()
    for measurement in measurements:
        writer.writerow({key: measurement.get(key) for key in scalar_keys})
assert len(measurements) == 13
assert all(entry['sha256_before'] == entry['sha256_after'] for entry in measurements)
print('All 13 source hashes unchanged. Analysis complete.', flush=True)
