from pathlib import Path
import argparse
import hashlib
import json
import struct
import sys
import uuid
import wave

import numpy

repository_directory = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(repository_directory / 'tools'))
from common.output_policy import validate_output_path


def write_document(destination, document):
    validate_output_path(repository_directory, destination, 'promote')
    destination.write_text(json.dumps(document, indent=2) + '\n', encoding='utf-8', newline='\n')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--metadata', type=Path)
    parser.add_argument('--use-recovered-links', action='store_true')
    arguments = parser.parse_args()
    destination_directory = repository_directory / 'game/sounds/banks/v8-bank'
    original_metadata_path = destination_directory / 'original_event_metadata.json'
    if arguments.metadata:
        write_document(original_metadata_path, json.loads(arguments.metadata.read_text(encoding='utf-8')))
    metadata = json.loads(original_metadata_path.read_text(encoding='utf-8'))
    write_document(original_metadata_path, metadata)
    source_bank = repository_directory / 'game/sounds/banks/cim_2008_ferrari/cim_2008_ferrari.bank'
    if source_bank.is_file() and not arguments.use_recovered_links:
        bank_bytes = source_bank.read_bytes()
        assert hashlib.sha256(bank_bytes).hexdigest() == metadata['sha256']
        instrument_chunks = []

        def collect_chunks(start, end):
            position = start
            while position + 8 <= end:
                identifier = bank_bytes[position:position + 4]
                length = struct.unpack_from('<I', bank_bytes, position + 4)[0]
                assert position + 8 + length <= end
                if identifier == b'WAIB':
                    instrument_chunks.append(position)
                if identifier == b'LIST':
                    collect_chunks(position + 12, position + 8 + length)
                position += 8 + length + length % 2

        collect_chunks(12, struct.unpack_from('<I', bank_bytes, 4)[0] + 8)
        assert len(instrument_chunks) == len(metadata['instances']) == 72
        recovered_instances = []
        for position, original_instance in zip(instrument_chunks, metadata['instances']):
            instrument_guid = str(uuid.UUID(bytes_le=bank_bytes[position + 8:position + 24]))
            assert str(uuid.UUID(bytes_le=bank_bytes[position + 24:position + 40])) == original_instance['wave_guid']
            assert bank_bytes[position + 40:position + 44] == b'INST'
            if original_instance['anchor'] is not None:
                assert struct.unpack_from('<f', bank_bytes, position + 117)[0] == original_instance['anchor']
            assert uuid.UUID(original_instance['bus_guid']).bytes_le in bank_bytes[position + 48:position + 171]
            instance = dict(original_instance)
            instance['instrument_guid'] = instrument_guid
            instance['instrument_offset'] = position
            instance['recovered_controllers'] = [controller for controller in metadata['controllers'] if controller['target_guid'] in (instrument_guid, instance['bus_guid'])]
            recovered_instances.append(instance)
    else:
        recovered_document = json.loads((destination_directory / 'recovered_automation.json').read_text())
        assert recovered_document['source_bank_sha256'] == metadata['sha256']
        assert recovered_document['all_instrument_sample_identifiers_anchors_and_bus_links_verified_against_bank']
        recovered_instances = recovered_document['engine_instances']
        original_instances = {instance['index']: instance for instance in metadata['instances']}
        for instance in recovered_instances:
            original_instance = original_instances[instance['index']]
            assert all(instance[key] == value for key, value in original_instance.items())
            instance['recovered_controllers'] = [controller for controller in metadata['controllers'] if controller['target_guid'] in (instance['instrument_guid'], instance['bus_guid'])]
    recovered_instances_by_index = {instance['index']: instance for instance in recovered_instances}
    controllers_by_identifier = {controller['guid']: controller for controller in metadata['controllers']}
    selected_identifiers = ('5cb018b3-751f-472a-9741-4bd5d97614ff', 'dac9a39a-679a-4297-abda-9f3ceb852323')
    selected_indices = {entry['instance_index'] for identifier in selected_identifiers for entry in controllers_by_identifier[identifier]['matched_instances']}
    source_inventory = json.loads((destination_directory / 'source_inventory.json').read_text())
    source_inventory['runtime_manifest_ready'] = True
    source_inventory['original_automation_status'] = 'Instrument curve links recovered from WAIB GUIDs. Original control points and instance anchors preserved; tangent interpolation, pitch modulation and spatial behavior reconstructed approximately.'
    write_document(destination_directory / 'source_inventory.json', source_inventory)
    samples_by_name = {sample['original_sample_name']: sample for sample in source_inventory['samples']}
    derived_by_name = {}
    selected_voices = []
    for instance in recovered_instances:
        if instance['index'] not in selected_indices:
            continue
        sample = samples_by_name[instance['sample']]
        if instance['sample'] not in derived_by_name:
            source_path = destination_directory / sample['filename']
            assert hashlib.sha256(source_path.read_bytes()).hexdigest() == sample['sha256']
            with wave.open(str(source_path), 'rb') as source_wave:
                assert (source_wave.getnchannels(), source_wave.getsampwidth(), source_wave.getframerate()) == (1, 2, 44100)
                samples = numpy.frombuffer(source_wave.readframes(source_wave.getnframes()), dtype='<i2').astype(numpy.float64)
            samples = samples[sample['loop_start_frame']:sample['loop_end_frame_exclusive']]
            seam_frames = min(221, len(samples) // 8)
            positions = numpy.linspace(0.0, 1.0, seam_frames)
            weights = positions * positions * (3.0 - 2.0 * positions)
            samples[-seam_frames:] = samples[-seam_frames:] * (1.0 - weights) + samples[:seam_frames] * weights
            samples = numpy.rint(samples[seam_frames:]).astype('<i2')
            derived_path = destination_directory / (instance['sample'] + '_prepared_loop.wav')
            validate_output_path(repository_directory, derived_path, 'promote')
            with wave.open(str(derived_path), 'wb') as destination_wave:
                destination_wave.setnchannels(1)
                destination_wave.setsampwidth(2)
                destination_wave.setframerate(44100)
                destination_wave.writeframes(samples.tobytes())
            normalized_samples = samples.astype(numpy.float64) / 32768.0
            derived_by_name[instance['sample']] = {
                'filename': derived_path.name,
                'sha256': hashlib.sha256(derived_path.read_bytes()).hexdigest(),
                'frames': len(samples),
                'level': float(20 * numpy.log10(max(1e-12, numpy.sqrt(numpy.mean(normalized_samples ** 2))))),
                'peak': float(20 * numpy.log10(max(1e-12, numpy.max(numpy.abs(normalized_samples))))),
                'seam_overlap_frames': seam_frames,
            }
        derived = derived_by_name[instance['sample']]
        throttle_controller = next(controller for controller in instance['recovered_controllers'] if controller['parameter'] == 'throttle')
        throttle_points = throttle_controller['keyframes']
        throttle_curve = []
        for lower, upper in zip(throttle_points, throttle_points[1:]):
            for position in numpy.linspace(0.0, 1.0, 16, endpoint=False):
                throttle_curve.append({'parameter_value': float(lower['x'] + (upper['x'] - lower['x']) * position), 'amplitude': float(10 ** ((lower['value'] + (upper['value'] - lower['value']) * position) / 20))})
        throttle_curve.append({'parameter_value': throttle_points[-1]['x'], 'amplitude': 10 ** (throttle_points[-1]['value'] / 20)})
        selected_voices.append({
            'instrument_guid': instance['instrument_guid'],
            'filename': derived['filename'],
            'sha256': derived['sha256'],
            'reference_revolutions_per_minute': instance['anchor'],
            'loop_start_frame': 0,
            'loop_end_frame_exclusive': derived['frames'],
            'revolutions_per_minute_curves': [[{'parameter_value': point['x'], 'amplitude': point['value']} for point in controller['keyframes']] for controller in instance['recovered_controllers'] if controller['parameter'] == 'rpms'],
            'throttle_curve': throttle_curve,
        })
    preserved_directory = repository_directory / 'game/sounds/banks/v10-v2-bank'
    manifest = json.loads((preserved_directory / 'manifest.json').read_text())
    for record in manifest['events'] + [manifest['transmission_whine']]:
        for field in ('source_filename', 'derived_filename'):
            source_path = preserved_directory / record[field]
            destination = destination_directory / record[field]
            validate_output_path(repository_directory, destination, 'promote')
            destination.write_bytes(source_path.read_bytes())
    transition_bounds = [(7000.0, 8400.0), (10000.0, 10600.0), (12600.0, 13800.0), (15162.99805, 16000.0)]
    representative_indices = [39, 18, 36, 7, 43]
    powered_loops = []
    for index, instance_index in enumerate(representative_indices):
        instance = recovered_instances_by_index[instance_index]
        derived = derived_by_name[instance['sample']]
        sample = samples_by_name[instance['sample']]
        coverage_start = 4500.0 if index == 0 else transition_bounds[index - 1][0]
        coverage_end = 18000.0 if index == 4 else transition_bounds[index][1]
        powered_loops.append({
            'id': 'recorded_engine_voice_' + str(index + 1), 'role': 'engine_loop',
            'source_filename': sample['filename'], 'source_sha256': sample['sha256'],
            'derived_filename': derived['filename'], 'derived_sha256': derived['sha256'],
            'derived_frames': derived['frames'], 'derived_rms_dbfs': derived['level'], 'derived_peak_dbfs': derived['peak'],
            'comb_spacing_hz': 0.0, 'reference_revolutions_per_minute': instance['anchor'],
            'zone_revolutions_per_minute': [4500.0, 8400.0, 10600.0, 13800.0, 17000.0][index],
            'calibrated_gain': 1.0, 'valid_playback_rate_min': coverage_start / instance['anchor'], 'valid_playback_rate_max': coverage_end / instance['anchor'],
            'active_coverage_revolutions_per_minute': [coverage_start, coverage_end],
            'loop_start_frame': 0, 'loop_end_frame_exclusive': derived['frames'], 'loop_crossfade_frames': derived['seam_overlap_frames'],
        })
    manifest['loops'] = powered_loops
    manifest['transitions'] = [{'from_loop_id': powered_loops[index]['id'], 'to_loop_id': powered_loops[index + 1]['id'], 'start_revolutions_per_minute': lower, 'end_revolutions_per_minute': upper, 'center_revolutions_per_minute': (lower + upper) / 2, 'half_width_revolutions_per_minute': (upper - lower) / 2, 'blend_law': 'smoothstep', 'gain_law': 'equal_power', 'measured_level_compensation_db': 0.0} for index, (lower, upper) in enumerate(transition_bounds)]
    manifest['coast_loops'] = []
    manifest['coast_transitions'] = []
    manifest['bank_id'] = 'ferrari_2008_recorded_engine_bank'
    manifest['preparation_tool'] = 'tools/audio/build_ferrari_recorded_engine_bank.py'
    manifest['preparation_tool_revision'] = 1
    manifest['source_inventory_sha256'] = hashlib.sha256((destination_directory / 'source_inventory.json').read_bytes()).hexdigest()
    manifest['recorded_engine_automation'] = {'interpolation': 'linear_reconstruction', 'voices': selected_voices}
    write_document(destination_directory / 'manifest.json', manifest)
    write_document(destination_directory / 'recovered_automation.json', {
        'source_bank_sha256': metadata['sha256'],
        'all_instrument_sample_identifiers_anchors_and_bus_links_verified_against_bank': True,
        'engine_instances': [instance for instance in recovered_instances if instance['event_id'] in (2, 6)],
        'recovered_revolutions_per_minute_controller_links': sum(controller['parameter'] == 'rpms' for instance in recovered_instances for controller in instance['recovered_controllers']),
        'selected_exterior_rear_voice_indices': sorted(selected_indices),
        'selected_throttle_controller_identifiers': selected_identifiers,
        'reconstruction_limits': ['Exact original points and instance anchors; linear interpolation replaces unresolved original tangent semantics.', 'Throttle values treated as decibels, supported by extracted -42 and -6.052174 values but not independently verified through FMOD.', 'Pitch ratio follows simulation RPM divided by instance anchor; no FMOD pitch modulation or spatial effects reproduced.', 'Rear exterior perspective selected for the mono runtime; interior and front branches remain archived.', 'Five millisecond overlap removes unverified full-sample loop seams; source WAV files remain unchanged.'],
        'preserved_non_engine_manifest_fields': ['events', 'event_groups', 'event_trigger_policy', 'transmission_whine', 'event_selection_seed'],
    })
    for filename, contents in {
        'README.md': '# Ferrari 2008: candidato V8 conservado\n\nEl manifest usa diez voces de las ramas exteriores traseras ON/OFF originales. Las anclas y ventanas RPM se recuperan por CTRL.target_guid -> WAIB.instrument_guid; las curvas de acelerador se recuperan por bus. Caja, whine, backfires, limitador y politica de eventos son copias exactas del banco V10 previo. Commons permanece separado.\n\nLimites: interpolacion lineal reconstruida, acelerador interpretado como decibelios y pitch proporcional a RPM/ancla; no se afirma equivalencia exacta con FMOD. No se reproducen automatizaciones espaciales ni de pitch originales. Se selecciona una perspectiva exterior trasera mono, sin conmutacion de camara. Las fuentes interiores y frontales permanecen disponibles. El solapamiento de cinco milisegundos de los loops preparados es nuevo, no metadata original.\n\nRegenerar con `python tools/audio/build_ferrari_recorded_engine_bank.py`. Usa las fuentes WAV verificadas y los enlaces recuperados incluidos; si el bank original esta disponible, vuelve a verificar los enlaces binarios. Metadatos originales: original_event_metadata.json. Grafo y limites: recovered_automation.json. La activacion modifica solamente profiles/geometric/audio.json y el soporte de automatizacion de motor del sampler.\n',
        'AGENTS.md': '# Ferrari recorded engine bank\n\nScope: inactive engine-only V8 candidate for the geometric F1 2030 profile. Regenerate runtime manifest, prepared loops and recovered automation using tools/audio/build_ferrari_recorded_engine_bank.py. Original WAVs remain mono PCM16 at 44100 Hz; source_inventory.json records provenance. Preserve every non-engine asset, event group and trigger policy from v10-v2-bank byte-for-byte. Never claim exact FMOD interpolation: the current reconstruction uses original control points and anchors with approximate linear interpolation and proportional pitch. Keep source perspective and newly authored seam processing explicit.\n',
    }.items():
        destination = destination_directory / filename
        validate_output_path(repository_directory, destination, 'promote')
        destination.write_text(contents, encoding='utf-8', newline='\n')
    print(json.dumps({'engine_voices': len(selected_voices), 'prepared_recordings': len(derived_by_name), 'destination': str(destination_directory)}, indent=2))


if __name__ == '__main__':
    main()
