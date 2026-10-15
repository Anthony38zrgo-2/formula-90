from pathlib import Path
import argparse
import hashlib
import json
import math
import sys
import wave

import numpy
from scipy.signal import resample_poly

repository_directory = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(repository_directory / 'tools'))
from common.output_policy import validate_output_path


def write_bytes(destination, contents):
    validate_output_path(repository_directory, destination, 'promote')
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(contents)


def write_document(destination, document):
    write_bytes(destination, (json.dumps(document, indent=2) + '\n').encode('utf-8'))


def build_bank(source_directory):
    destination_directory = repository_directory / 'game/sounds/banks/renault-interior-bank'
    metadata_path = source_directory / 'metadata_extraida.json'
    metadata = json.loads(metadata_path.read_text(encoding='utf-8'))
    inventory = []
    prepared_samples = {}
    for sample in metadata['samples']:
        if not sample['name'].startswith('renault_r26_int_'):
            continue
        assert sample['event_ids'] == [1] and len(sample['rpm_anchors']) == 1
        source_path = source_directory / sample['wav']
        source_contents = source_path.read_bytes()
        with wave.open(str(source_path), 'rb') as source_wave:
            assert source_wave.getsampwidth() == 2
            assert (source_wave.getframerate(), source_wave.getnchannels(), source_wave.getnframes()) == (sample['frequency'], sample['channels'], sample['samples'])
            channels = source_wave.getnchannels()
            source_rate = source_wave.getframerate()
            source_samples = numpy.frombuffer(source_wave.readframes(source_wave.getnframes()), dtype='<i2').astype(numpy.float64).reshape(-1, channels)
        source_mono = source_samples.mean(axis=1)
        divisor = math.gcd(source_rate, 44100)
        converted_samples = resample_poly(source_mono, 44100 // divisor, source_rate // divisor)
        assert numpy.max(numpy.abs(converted_samples)) <= 32768
        converted_samples = numpy.clip(numpy.rint(converted_samples), -32768, 32767).astype('<i2')
        converted_path = destination_directory / (sample['name'] + '.wav')
        validate_output_path(repository_directory, converted_path, 'promote')
        converted_path.parent.mkdir(parents=True, exist_ok=True)
        with wave.open(str(converted_path), 'wb') as converted_wave:
            converted_wave.setnchannels(1)
            converted_wave.setsampwidth(2)
            converted_wave.setframerate(44100)
            converted_wave.writeframes(converted_samples.tobytes())
        loop_start = round(sample.get('loop_start', 0) * 44100 / source_rate)
        loop_end = min(len(converted_samples), round((sample.get('loop_end', sample['samples'] - 1) + 1) * 44100 / source_rate))
        region = converted_samples[loop_start:loop_end].astype(numpy.float64)
        overlap_frames = min(221, len(region) // 8)
        authored_reference = sample['rpm_anchors'][0]
        cycle_count = math.floor((len(region) - overlap_frames) / 44100 * authored_reference / 120)
        derived_frames = round(cycle_count * 44100 * 120 / authored_reference)
        assert cycle_count > 0 and derived_frames + overlap_frames <= len(region)
        region = region[:derived_frames + overlap_frames]
        positions = numpy.linspace(0.0, 1.0, overlap_frames)
        weights = positions * positions * (3.0 - 2.0 * positions)
        region[-overlap_frames:] = region[-overlap_frames:] * (1.0 - weights) + region[:overlap_frames] * weights
        region = numpy.rint(region[overlap_frames:]).astype('<i2')
        derived_path = destination_directory / (sample['name'] + '_loop.wav')
        validate_output_path(repository_directory, derived_path, 'promote')
        with wave.open(str(derived_path), 'wb') as derived_wave:
            derived_wave.setnchannels(1)
            derived_wave.setsampwidth(2)
            derived_wave.setframerate(44100)
            derived_wave.writeframes(region.tobytes())
        normalized_region = region.astype(numpy.float64) / 32768.0
        root_mean_square = float(numpy.sqrt(numpy.mean(normalized_region ** 2)))
        record = {
            'source_name': sample['name'], 'original_path': str(source_path),
            'original_wave_sha256': hashlib.sha256(source_contents).hexdigest(),
            'source_filename': converted_path.name, 'source_sha256': hashlib.sha256(converted_path.read_bytes()).hexdigest(),
            'derived_filename': derived_path.name, 'derived_sha256': hashlib.sha256(derived_path.read_bytes()).hexdigest(),
            'derived_frames': len(region), 'derived_rms_dbfs': 20 * math.log10(max(root_mean_square, 1e-12)),
            'derived_peak_dbfs': 20 * math.log10(max(float(numpy.max(numpy.abs(normalized_region))), 1e-12)),
            'original_reference_revolutions_per_minute': authored_reference,
            'reference_revolutions_per_minute': cycle_count * 44100 * 120 / len(region),
            'integer_cycle_count': cycle_count, 'reference_method': 'authored_instance_anchor_integer_cycle_adjustment_not_acoustic_measurement',
            'original_loop_start_frame': sample.get('loop_start'), 'original_loop_end_frame_inclusive': sample.get('loop_end'),
            'source_loop_metadata_present': 'loop_start' in sample,
            'loop_start_frame': 0, 'loop_end_frame_exclusive': len(region), 'loop_crossfade_frames': overlap_frames,
            'seam_step': int(abs(int(region[-1]) - int(region[0]))),
            'normalized_root_mean_square': root_mean_square,
        }
        inventory.append(record)
        prepared_samples[sample['name']] = record
    assert len(inventory) == 7
    write_bytes(destination_directory / 'original_event_metadata.json', metadata_path.read_bytes())
    source_inventory = {'source_directory': str(source_directory), 'metadata_sha256': hashlib.sha256(metadata_path.read_bytes()).hexdigest(), 'claimed_source_bank_sha256': metadata['sha256'], 'original_bank_binary_available': False, 'sample_rate': 44100, 'channels': 1, 'bits_per_sample': 16, 'samples': inventory}
    write_document(destination_directory / 'source_inventory.json', source_inventory)
    previous_directory = repository_directory / 'game/sounds/banks/v10-v2-bank'
    manifest = json.loads((previous_directory / 'manifest.json').read_text())
    for asset in manifest['events'] + [manifest['transmission_whine']]:
        for key in ('source_filename', 'derived_filename'):
            write_bytes(destination_directory / asset[key], (previous_directory / asset[key]).read_bytes())
    manifest.pop('recorded_engine_automation', None)
    collections = (
        ('loops', 'transitions', 'engine_loop', ['renault_r26_int_off_idle', 'renault_r26_int_on_midlow', 'renault_r26_int_on_mid', 'renault_r26_int_on_upshift_2'], [(5500.0, 7000.0), (9500.0, 10650.0), (13000.0, 15000.0)], [4500.0, 8500.0, 11500.0, 17200.0]),
        ('coast_loops', 'coast_transitions', 'engine_coast_loop', ['renault_r26_int_off_low', 'renault_r26_int_off_mid', 'renault_r26_int_off_downshift'], [(8500.0, 10000.0), (12500.0, 14500.0)], [7250.0, 11600.0, 15000.0]),
    )
    for collection, transitions, role, names, windows, zones in collections:
        assets = []
        for index, name in enumerate(names):
            asset = dict(prepared_samples[name])
            asset['id'] = name + '_loop'
            asset['role'] = role
            asset['zone_revolutions_per_minute'] = zones[index]
            asset['presence_curve'] = []
            minimum = 4500.0 if index == 0 else windows[index - 1][0]
            maximum = 18000.0 if index == len(names) - 1 else windows[index][1]
            asset['active_coverage_revolutions_per_minute'] = [minimum, maximum]
            asset['valid_playback_rate_min'] = minimum / asset['reference_revolutions_per_minute']
            asset['valid_playback_rate_max'] = maximum / asset['reference_revolutions_per_minute']
            asset['calibrated_gain'] = 10 ** ((-17.0 - asset['derived_rms_dbfs']) / 20)
            asset['comb_spacing_hz'] = 0.0
            assets.append(asset)
        manifest[collection] = assets
        manifest[transitions] = [{'from_loop_id': assets[index]['id'], 'to_loop_id': assets[index + 1]['id'], 'start_revolutions_per_minute': minimum, 'end_revolutions_per_minute': maximum, 'center_revolutions_per_minute': (minimum + maximum) / 2, 'half_width_revolutions_per_minute': (maximum - minimum) / 2, 'blend_law': 'smoothstep', 'gain_law': 'equal_power', 'measured_level_compensation_db': 0.0} for index, (minimum, maximum) in enumerate(windows)]
    manifest.update(bank_id='renault_interior_engine_trial', preparation_tool='tools/audio/build_renault_interior_engine_bank.py', preparation_tool_revision=1, source_inventory_sha256=hashlib.sha256((destination_directory / 'source_inventory.json').read_bytes()).hexdigest())
    write_document(destination_directory / 'manifest.json', manifest)
    write_bytes(destination_directory / '.gdignore', b'')
    write_bytes(destination_directory / 'AGENTS.md', b'# Renault interior engine trial\n\nScope: user-authorized replacement of the Ferrari exterior engine layer. Use only renault_r26_int source WAVs for continuous engine voices. Preserve all non-engine assets and event settings from v10-v2-bank byte-for-byte. Regenerate with tools/audio/build_renault_interior_engine_bank.py. Original source inventory and metadata identify authored RPM anchors, inclusive loop endpoints and inferred whole-sample fallback loops. Crossfade windows, level calibration and seam overlaps are new trial authoring, not recovered FMOD behavior.\n')
    write_bytes(destination_directory / 'README.md', '# Prueba Renault R26 interior\n\nSolo las siete fuentes renault_r26_int. Cuatro voces ON/idle y tres OFF. Los archivos upshift/downshift aportan exclusivamente las regiones de bucle declaradas, con frecuencia dominante estable; no reemplazan los efectos de cambio. Los efectos, whine, backfires y limitador originales se conservan.\n\nConversion a mono PCM16/44100 Hz. Anclas RPM del JSON proporcionado; ajuste minimo para longitud de ciclos enteros. No se dispone del bank original para verificar su hash ni de sus curvas completas. Ventanas de crossfade y objetivo de nivel -17 dBFS son nuevos para esta prueba; pitch proporcional a RPM/referencia, no equivalencia FMOD. Solapamiento de cinco milisegundos para las costuras.\n\nRegenerar: python tools/audio/build_renault_interior_engine_bank.py. Fuentes originales en D:/ASETS/R26_WAV_y_Metadata/R26; el inventario guarda sus hashes.\n'.encode('utf-8'))
    print(json.dumps({'engine_sources': len(inventory), 'powered_loops': 4, 'coast_loops': 3, 'directory': str(destination_directory)}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-directory', type=Path, default=Path('D:/ASETS/R26_WAV_y_Metadata/R26'))
    build_bank(parser.parse_args().source_directory)
