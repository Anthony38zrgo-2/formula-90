"""Formula90s vehicle policy: opt-in, config-driven measurements.

This layer composes generic predicate evidence into vehicle wording; it never
changes generic semantics and never encodes FIA limits or legal-compliance
claims. Datums, sections, thresholds, axes and units live only in the config.

Measurement kinds:
    datum_distance              distance between two plane datums along their axis
    max_extent                  maximum signed extent along an axis in a section
    max_abs_extent              maximum absolute extent along an axis in a section
    forward_extent_beyond_datum maximum forward distance beyond a plane datum
    behind_extent_beyond_datum  maximum backward distance beyond a plane datum
    radial_about                maximum radial distance around a point datum
                                in the plane perpendicular to a chosen axis
"""
import math

from .. import contracts
from .. import hashing
from .. import predicates

MEASUREMENT_KINDS = (
    'datum_distance',
    'max_extent',
    'max_abs_extent',
    'forward_extent_beyond_datum',
    'behind_extent_beyond_datum',
    'radial_about',
)

NOT_MEASURABLE = contracts.STATUS_NOT_MEASURABLE


# ---------------------------------------------------------------------------
# Config schema validation
# ---------------------------------------------------------------------------

def validate_config(config):
    """Validate the vehicle-measure config schema; raise ConfigError."""
    convention = contracts.require_dict(
        config.get('coordinate_convention'), 'coordinate_convention'
    )
    convention_model = {}
    for role in ('forward', 'right', 'up'):
        convention_model[role] = contracts.convention_axis(
            convention, role, 'coordinate_convention'
        )
    contracts.require_str(config.get('units'), 'units')

    datums = contracts.require_dict(config.get('datums', {}), 'datums')
    datum_model = {}
    for name, datum in datums.items():
        datum_model[name] = _validate_datum(datum, name)

    sections = config.get('sections', {})
    if sections is None:
        sections = {}
    sections = contracts.require_dict(sections, 'sections')
    for name, section in sections.items():
        section = contracts.require_dict(section, 'sections.%s' % name)
        contracts.parse_axis(section.get('axis'), 'sections.%s.axis' % name)
        for bound in ('min', 'max'):
            if bound in section:
                contracts.require_finite_number(
                    section[bound], 'sections.%s.%s' % (name, bound)
                )

    sources = _resolve_sources(config)
    source_ids = [source['id'] for source in sources]

    measurements = contracts.require_list(
        config.get('measurements'), 'measurements'
    )
    measurement_model = []
    for index, measurement in enumerate(measurements):
        measurement_model.append(
            _validate_measurement(
                measurement, index, datums, sections, source_ids
            )
        )
    if not measurement_model:
        raise contracts.ConfigError('measurements must not be empty')
    return {
        'convention': convention_model,
        'units': contracts.require_str(config.get('units'), 'units'),
        'datums': datum_model,
        'sections': sections,
        'sources': sources,
        'measurements': measurement_model,
    }


def _validate_datum(datum, name):
    datum = contracts.require_dict(datum, 'datums.%s' % name)
    has_axis = isinstance(datum.get('axis'), str)
    has_point = isinstance(datum.get('point'), list)
    if has_axis == has_point:
        raise contracts.ConfigError(
            'datums.%s must define exactly one of "axis" or "point"' % name
        )
    if has_axis:
        axis = contracts.parse_axis(datum['axis'], 'datums.%s.axis' % name)
        value = contracts.require_finite_number(
            datum['value'], 'datums.%s.value' % name
        )
        return {'kind': 'plane', 'axis': axis, 'value': value}
    point = datum['point']
    if len(point) != 3:
        raise contracts.ConfigError('datums.%s.point must have 3 values' % name)
    for index, value in enumerate(point):
        contracts.require_finite_number(value, 'datums.%s.point[%d]' % (name, index))
    return {'kind': 'point', 'point': list(point)}


def _resolve_sources(config):
    if 'source' in config:
        expected = config.get('expected_sha256')
        if expected is not None:
            expected = contracts.require_str(expected, 'expected_sha256')
        sources = [{
            'id': 'default',
            'path': contracts.require_str(config['source'], 'source'),
            'expected_sha256': expected,
        }]
    elif 'sources' in config:
        sources = []
        entries = contracts.require_list(config['sources'], 'sources')
        for index, entry in enumerate(entries):
            entry = contracts.require_dict(entry, 'sources[%d]' % index)
            source_id = contracts.require_str(entry.get('id'), 'sources[%d].id' % index)
            path = contracts.require_str(entry.get('path'), 'sources[%d].path' % index)
            expected = entry.get('expected_sha256')
            if expected is not None:
                expected = contracts.require_str(
                    expected, 'sources[%d].expected_sha256' % index
                )
            sources.append({'id': source_id, 'path': path, 'expected_sha256': expected})
    else:
        raise contracts.ConfigError('config must define "source" or "sources"')
    source_ids = [source['id'] for source in sources]
    if len(source_ids) != len(set(source_ids)):
        raise contracts.ConfigError('source ids must be unique')
    return sources


def _validate_measurement(measurement, index, datums, sections, source_ids):
    label = 'measurements[%d]' % index
    measurement = contracts.require_dict(measurement, label)
    measurement_id = contracts.require_str(measurement.get('id'), label + '.id')
    kind = contracts.require_str(measurement.get('kind'), label + '.kind')
    if kind not in MEASUREMENT_KINDS:
        raise contracts.ConfigError(
            '%s.kind must be one of: %s' % (label, ', '.join(MEASUREMENT_KINDS))
        )
    source = measurement.get('source', source_ids[0])
    if source not in source_ids:
        raise contracts.ConfigError(
            '%s.source references unknown source: %r' % (label, source)
        )
    expected = contracts.require_dict(measurement.get('expected'), label + '.expected')
    for bound in ('min', 'max'):
        if bound in expected:
            contracts.require_finite_number(expected[bound], label + '.expected.' + bound)
    if kind == 'datum_distance':
        for key in ('from', 'to'):
            datum = contracts.require_str(measurement.get(key), label + '.' + key)
            if datum not in datums:
                raise contracts.ConfigError(
                    '%s.%s references unknown datum: %r' % (label, key, datum)
                )
    elif kind in ('max_extent', 'max_abs_extent'):
        axis = contracts.require_str(measurement.get('axis'), label + '.axis')
        if axis not in ('up', 'right', 'forward'):
            contracts.parse_axis(axis, label + '.axis')
        section = measurement.get('section')
        if section is not None:
            if not isinstance(section, str) or section not in sections:
                raise contracts.ConfigError(
                    '%s.section references unknown section: %r' % (label, section)
                )
    elif kind in ('forward_extent_beyond_datum', 'behind_extent_beyond_datum'):
        datum = contracts.require_str(measurement.get('datum'), label + '.datum')
        if datum not in datums:
            raise contracts.ConfigError(
                '%s.datum references unknown datum: %r' % (label, datum)
            )
    else:  # radial_about
        datum = contracts.require_str(measurement.get('datum'), label + '.datum')
        if datum not in datums:
            raise contracts.ConfigError(
                '%s.datum references unknown datum: %r' % (label, datum)
            )
        contracts.parse_axis(measurement.get('plane_axis'), label + '.plane_axis')
    return {
        'id': measurement_id,
        'kind': kind,
        'source': source,
        'expected': expected,
        'spec': measurement,
    }


# ---------------------------------------------------------------------------
# Measurement evaluation
# ---------------------------------------------------------------------------

def _not_measurable(reason):
    return {'status': NOT_MEASURABLE, 'measured': None, 'reason': reason}


def _compare(measured, expected):
    minimum = expected.get('min')
    maximum = expected.get('max')
    if minimum is not None and measured < minimum:
        return {'status': contracts.STATUS_FAIL, 'measured': measured, 'reason': None}
    if maximum is not None and measured > maximum:
        return {'status': contracts.STATUS_FAIL, 'measured': measured, 'reason': None}
    return {'status': contracts.STATUS_PASS, 'measured': measured, 'reason': None}


def _measurement(entry, model, vertices):
    kind = entry['kind']
    spec = entry['spec']
    if kind == 'datum_distance':
        from_datum = model['datums'][spec['from']]
        to_datum = model['datums'][spec['to']]
        if from_datum['kind'] != 'plane' or to_datum['kind'] != 'plane':
            return _not_measurable('datum_distance requires plane datums')
        if from_datum['axis'][0] != to_datum['axis'][0]:
            return _not_measurable('datums must use the same physical axis')
        measured = abs(
            to_datum['value'] * to_datum['axis'][1]
            - from_datum['value'] * from_datum['axis'][1]
        )
        return _compare(measured, entry['expected'])

    if kind in ('max_extent', 'max_abs_extent'):
        axis = spec['axis']
        if axis in ('up', 'right', 'forward'):
            axis_index, sign = model['convention'][axis]
        else:
            axis_index, sign = contracts.parse_axis(axis, 'measurement.axis')
        section_name = spec.get('section')
        scope = vertices
        if section_name is not None:
            scope = predicates.section_filter(vertices, model['sections'][section_name])
        finite = [v for v in scope if v['finite']]
        if not finite:
            return _not_measurable('no finite vertices in section')
        if kind == 'max_extent':
            measured = max(v['point'][axis_index] * sign for v in finite)
        else:
            measured = max(abs(v['point'][axis_index]) for v in finite)
        return _compare(measured, entry['expected'])

    if kind == 'forward_extent_beyond_datum':
        datum = model['datums'][spec['datum']]
        if datum['kind'] != 'plane':
            return _not_measurable('forward_extent_beyond_datum requires a plane datum')
        forward_index, forward_sign = model['convention']['forward']
        datum_index, datum_sign = datum['axis']
        if forward_index != datum_index:
            return _not_measurable('datum axis must match the forward axis')
        datum_forward = datum['value'] * datum_sign * forward_sign
        distances = []
        for vertex in vertices:
            if not vertex['finite']:
                continue
            forward_coord = vertex['point'][forward_index] * forward_sign
            if forward_coord > datum_forward:
                distances.append(forward_coord - datum_forward)
        if not distances:
            return _not_measurable('no vertices beyond the datum plane')
        return _compare(max(distances), entry['expected'])

    if kind == 'behind_extent_beyond_datum':
        datum = model['datums'][spec['datum']]
        if datum['kind'] != 'plane':
            return _not_measurable('behind_extent_beyond_datum requires a plane datum')
        forward_index, forward_sign = model['convention']['forward']
        datum_index, datum_sign = datum['axis']
        if forward_index != datum_index:
            return _not_measurable('datum axis must match the forward axis')
        datum_forward = datum['value'] * datum_sign * forward_sign
        distances = []
        for vertex in vertices:
            if not vertex['finite']:
                continue
            forward_coord = vertex['point'][forward_index] * forward_sign
            if forward_coord < datum_forward:
                distances.append(datum_forward - forward_coord)
        if not distances:
            return _not_measurable('no vertices behind the datum plane')
        return _compare(max(distances), entry['expected'])

    # radial_about
    datum = model['datums'][spec['datum']]
    if datum['kind'] != 'point':
        return _not_measurable('radial_about requires a point datum')
    plane_index, _ = contracts.parse_axis(spec['plane_axis'], 'measurement.plane_axis')
    centre = datum['point']
    other = [i for i in range(3) if i != plane_index]
    radii = []
    for vertex in vertices:
        if not vertex['finite']:
            continue
        radii.append(math.sqrt(
            sum((vertex['point'][i] - centre[i]) ** 2 for i in other)
        ))
    if not radii:
        return _not_measurable('no finite vertices')
    return _compare(max(radii), entry['expected'])


def measure(model, vertices_by_source, broken_source_ids=()):
    """Evaluate every measurement over precomputed per-source vertices.

    ``model`` comes from :func:`validate_config`; ``vertices_by_source`` maps
    source id -> (vertices, non_finite_count) with vertices in world space.
    Sources in ``broken_source_ids`` fail every measurement with a
    precondition reason (never a silent pass).
    """
    results = []
    for entry in model['measurements']:
        vertices, _ = vertices_by_source[entry['source']]
        if entry['source'] in broken_source_ids:
            result = _not_measurable('source precondition not met (hash mismatch)')
        else:
            result = _measurement(entry, model, vertices)
        results.append({
            'id': entry['id'],
            'kind': entry['kind'],
            'source': entry['source'],
            'unit': model['units'],
            'status': result['status'],
            'measured': result['measured'],
            'expected': entry['expected'],
            'reason': result['reason'],
        })
    any_fail = any(r['status'] == contracts.STATUS_FAIL for r in results)
    any_not_measurable = any(r['status'] == NOT_MEASURABLE for r in results)
    if any_fail:
        status = contracts.STATUS_FAIL
        reason = 'at least one measured value violates its configured expectation'
    elif any_not_measurable:
        status = NOT_MEASURABLE
        reason = 'at least one measurement could not be evaluated from the source'
    else:
        status = contracts.STATUS_PASS
        reason = 'all measurements satisfy their configured expectations'
    return {'measurements': results, 'summary': {'status': status, 'reason': reason}}


def hash_evidence(model, resolved_paths):
    """Compute SHA-256 per source and detect expected-hash mismatches.

    ``resolved_paths`` maps source id -> Path. Returns
    (source_evidence, broken_source_ids, findings).
    """
    from .. import paths

    root = paths.find_repository_root()
    findings = []
    source_evidence = []
    broken = []
    for source in model['sources']:
        path = resolved_paths[source['id']]
        digest = hashing.sha256_file(path)
        expected = source['expected_sha256']
        match = True
        if expected is not None and expected.upper() != digest:
            match = False
            broken.append(source['id'])
            findings.append({
                'level': 'warning',
                'code': 'VEHICLE-HASH-MISMATCH',
                'message': (
                    'source %s does not match its expected SHA-256 '
                    '(measured %s)' % (source['id'], digest)
                ),
            })
        source_evidence.append({
            'id': source['id'],
            'path': paths.repo_relative(root, path),
            'sha256': digest,
            'expected_sha256': expected,
            'hash_match': match,
        })
    return source_evidence, broken, findings


def load_vertices(path):
    """Load world-space vertices for a source file (corrupt -> CorruptAssetError)."""
    try:
        return predicates.load_world_vertices(str(path))
    except ValueError as exc:
        raise contracts.CorruptAssetError(
            'GLB structural error while loading source: %s' % exc
        ) from exc
