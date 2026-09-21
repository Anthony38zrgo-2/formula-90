from pathlib import Path
import sys
import json
import hashlib
import numpy
from scipy import signal
import matplotlib
matplotlib.use('Agg')
from matplotlib import pyplot

analysis_directory = Path(__file__).resolve().parent
sys.path.insert(0, str(analysis_directory / 'python-packages'))
import soundfile
import pyloudnorm

measurements = json.loads((analysis_directory / 'measurements.json').read_text())
source_directory = Path(measurements['source_directory'])
family_candidates = {'98_int_idle.wav': 32.75, '98_int_low.wav': 61.86, '98_int_med.wav': 68.35, '98_int_high_1.wav': 93.15, '98_int_max_5.wav': 127.75}
details = {}


def decibels(value):
    return float(20 * numpy.log10(max(abs(value), 1e-15)))


def envelope_statistics(samples, sample_rate):
    frame_length = round(0.1 * sample_rate)
    hop_length = round(0.01 * sample_rate)
    frames = numpy.lib.stride_tricks.sliding_window_view(samples, frame_length)[::hop_length]
    levels = 10 * numpy.log10(numpy.maximum(numpy.mean(frames ** 2, axis=1), 1e-30))
    trim_length = max(1, round(len(levels) * 0.1))
    levels = levels[trim_length:-trim_length]
    return {'percentile_10': float(numpy.percentile(levels, 10)), 'percentile_50': float(numpy.percentile(levels, 50)), 'percentile_75': float(numpy.percentile(levels, 75)), 'percentile_90': float(numpy.percentile(levels, 90)), 'range_90_minus_10': float(numpy.percentile(levels, 90) - numpy.percentile(levels, 10))}


for measurement in measurements['measurements']:
    filename = measurement['filename']
    channels, sample_rate = soundfile.read(source_directory / filename, always_2d=True)
    samples = channels.mean(axis=1)
    centered = samples - samples.mean()
    values, counts = numpy.unique(samples, return_counts=True)
    most_frequent = numpy.argsort(counts)[-5:][::-1]
    repeated_values = [{'value': float(values[index]), 'count': int(counts[index]), 'level_decibels_full_scale': decibels(values[index])} for index in most_frequent]
    repeated_edges = numpy.r_[False, (numpy.diff(samples) == 0) & (numpy.abs(samples[:-1]) > 0.5)]
    repeated_edges |= numpy.r_[repeated_edges[1:], False]
    frequencies, spectrum = signal.welch(centered, sample_rate, nperseg=min(16384, len(centered)), detrend=False)
    region_without_offset = float(spectrum[frequencies < 20].sum() / spectrum.sum() * 100)
    meter = pyloudnorm.Meter(sample_rate)
    filtered = samples.copy()
    for filter_stage in meter._filters.values():
        filtered = filter_stage.apply_filter(filtered)
    loudness = float(meter.integrated_loudness(samples)) if len(samples) / sample_rate >= 0.4 else None
    record = {'most_frequent_sample_values': repeated_values, 'samples_participating_in_high_amplitude_plateaus_percent': float(numpy.mean(repeated_edges) * 100), 'sub_20_hertz_power_percent_after_mean_removal': region_without_offset, 'mono_integrated_loudness': loudness, 'mono_ungated_weighted_level': float(-0.691 + 10 * numpy.log10(numpy.mean(filtered ** 2))), 'final_5_millisecond_level_decibels_full_scale': decibels(numpy.sqrt(numpy.mean(samples[-round(sample_rate * 0.005):] ** 2))), 'peak_time_seconds': float(numpy.argmax(numpy.abs(samples)) / sample_rate)}
    spectral_candidates = []
    power_levels = 10 * numpy.log10(spectrum + 1e-30)
    peak_indices, properties = signal.find_peaks(power_levels, prominence=8)
    for peak_index, prominence in zip(peak_indices, properties['prominences']):
        if frequencies[peak_index] >= 8000:
            spectral_candidates.append({'frequency_hertz': float(frequencies[peak_index]), 'prominence_decibels': float(prominence), 'density_decibels_full_scale_per_hertz': float(power_levels[peak_index])})
    record['high_frequency_narrow_peak_candidates'] = sorted(spectral_candidates, key=lambda entry: entry['density_decibels_full_scale_per_hertz'], reverse=True)[:5]
    if measurement['role'] == 'engine_loop':
        family = family_candidates[filename]
        peaks = measurement['spectral']['dominant_peaks']
        harmonic_evidence = []
        for peak in peaks:
            order = round(peak['frequency_hertz'] / family)
            harmonic_evidence.append({'frequency_hertz': peak['frequency_hertz'], 'order': order, 'deviation_hertz': peak['frequency_hertz'] - order * family})
        record['candidate_family_hertz'] = family
        record['candidate_harmonic_evidence'] = harmonic_evidence
        record['whole_band_envelope'] = envelope_statistics(centered, sample_rate)
        record['band_envelopes'] = {}
        for lower, upper in ((80, 150), (300, 600), (2500, 5000), (8000, 16000)):
            coefficients = signal.butter(3, [lower, upper], btype='bandpass', fs=sample_rate, output='sos')
            filtered_band = signal.sosfiltfilt(coefficients, centered)
            record['band_envelopes'][f'{lower}-{upper}'] = envelope_statistics(filtered_band, sample_rate)
        frame_length = round(0.08 * sample_rate)
        frames = numpy.lib.stride_tricks.sliding_window_view(centered, frame_length)[::round(0.01 * sample_rate)]
        frame_means = frames.mean(axis=1)
        record['local_80_millisecond_mean_10_90'] = numpy.percentile(frame_means, [10, 90]).tolist()
    details[filename] = record

figure, axes = pyplot.subplots(2, 2, figsize=(14, 8))
for row_index, filename in enumerate(('98_int_high_1.wav', '98_int_max_5.wav')):
    samples, sample_rate = soundfile.read(source_directory / filename)
    differences = numpy.diff(samples)
    candidate = numpy.where((differences == 0) & (numpy.abs(samples[:-1]) > 0.5))[0][10]
    beginning = max(0, candidate - round(sample_rate * 0.003))
    ending = min(len(samples), beginning + round(sample_rate * 0.012))
    axes[row_index, 0].plot(numpy.arange(beginning, ending) / sample_rate * 1000, samples[beginning:ending])
    axes[row_index, 0].set(title=filename, xlabel='Source time (ms)', ylabel='Amplitude')
    axes[row_index, 1].hist(samples, bins=150)
    axes[row_index, 1].set(title='Amplitude histogram: repeated ceiling values', xlabel='Amplitude', ylabel='Sample count')
figure.tight_layout()
figure.savefig(analysis_directory / 'figures' / 'baked_plateau_evidence.png', dpi=140)
pyplot.close(figure)
(analysis_directory / 'diagnostic_details.json').write_text(json.dumps(details, indent=2), encoding='utf-8')
for filename, record in details.items():
    print(filename, json.dumps({key: value for key, value in record.items() if key not in ('candidate_harmonic_evidence', 'band_envelopes')}, separators=(',', ':')))

assert all(hashlib.sha256((source_directory / entry['filename']).read_bytes()).hexdigest() == entry['sha256_before'] for entry in measurements['measurements'])
