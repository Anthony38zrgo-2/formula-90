pub use track_domain::{ControlPoint, Point2};

#[derive(Debug, Clone, PartialEq)]
pub struct CenterlineSample {
    pub station: f64,
    pub position: Point2,
    pub tangent: Point2,
    pub curvature: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GeometryError {
    TooFewPoints,
    NonFinitePoint,
    ZeroLengthSegment,
    InvalidTolerance,
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::TooFewPoints => "closed centerline needs at least three points",
            Self::NonFinitePoint => "centerline contains a non-finite point",
            Self::ZeroLengthSegment => "centerline contains a zero-length segment",
            Self::InvalidTolerance => "Bezier flattening tolerance must be finite and positive",
        };
        formatter.write_str(message)
    }
}

pub fn flatten_cubic(
    start: &Point2,
    control_a: &Point2,
    control_b: &Point2,
    end: &Point2,
    tolerance: f64,
) -> Result<Vec<Point2>, GeometryError> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(GeometryError::InvalidTolerance);
    }
    for point in [start, control_a, control_b, end] {
        if !point.x.is_finite() || !point.z.is_finite() {
            return Err(GeometryError::NonFinitePoint);
        }
    }
    let mut output = Vec::new();
    flatten_cubic_recursive(start, control_a, control_b, end, tolerance, &mut output);
    Ok(output)
}

impl std::error::Error for GeometryError {}

pub fn closed_centerline_length(points: &[ControlPoint]) -> Result<f64, GeometryError> {
    let lengths = segment_lengths(points)?;
    Ok(lengths.iter().sum())
}

pub fn sample_closed_centerline(
    points: &[ControlPoint],
    station: f64,
) -> Result<CenterlineSample, GeometryError> {
    let lengths = segment_lengths(points)?;
    let total: f64 = lengths.iter().sum();
    let target = station.rem_euclid(total);
    let mut accumulated = 0.0;

    for (index, length) in lengths.iter().enumerate() {
        if target <= accumulated + length || index == lengths.len() - 1 {
            let next = (index + 1) % points.len();
            let ratio = ((target - accumulated) / length).clamp(0.0, 1.0);
            let start = point(&points[index]);
            let end = point(&points[next]);
            let position = lerp(&start, &end, ratio);
            let tangent = normalize(&Point2 {
                x: end.x - start.x,
                z: end.z - start.z,
            });
            let previous = point(&points[(index + points.len() - 1) % points.len()]);
            let previous_tangent = normalize(&Point2 {
                x: start.x - previous.x,
                z: start.z - previous.z,
            });
            let curvature = cross(&previous_tangent, &tangent) / length;
            return Ok(CenterlineSample {
                station: target,
                position,
                tangent,
                curvature,
            });
        }
        accumulated += length;
    }

    unreachable!("a valid closed centerline always selects a segment")
}

pub fn resample_closed_polyline(
    points: &[Point2],
    spacing: f64,
) -> Result<Vec<Point2>, GeometryError> {
    if points.len() < 3 {
        return Err(GeometryError::TooFewPoints);
    }
    if !spacing.is_finite() || spacing <= 0.0 {
        return Err(GeometryError::NonFinitePoint);
    }
    let mut closed = points.to_vec();
    if closed.first() != closed.last() {
        closed.push(closed[0].clone());
    }
    let lengths = open_segment_lengths(&closed)?;
    let total: f64 = lengths.iter().sum();
    let count = ((total / spacing).round() as usize).max(4);
    let mut cumulative = Vec::with_capacity(lengths.len() + 1);
    cumulative.push(0.0);
    for length in &lengths {
        cumulative.push(cumulative.last().unwrap() + length);
    }

    let mut output = Vec::with_capacity(count);
    for index in 0..count {
        let target = total * index as f64 / count as f64;
        let mut segment = 0;
        while segment + 1 < cumulative.len() - 1 && cumulative[segment + 1] < target {
            segment += 1;
        }
        let start = cumulative[segment];
        let length = lengths[segment];
        let ratio = ((target - start) / length).clamp(0.0, 1.0);
        output.push(lerp(&closed[segment], &closed[segment + 1], ratio));
    }
    Ok(output)
}

fn segment_lengths(points: &[ControlPoint]) -> Result<Vec<f64>, GeometryError> {
    if points.len() < 3 {
        return Err(GeometryError::TooFewPoints);
    }
    let mut coordinates: Vec<_> = points.iter().map(point).collect();
    coordinates.push(coordinates[0].clone());
    open_segment_lengths(&coordinates)
}

pub fn closed_polyline_length(points: &[Point2]) -> Result<f64, GeometryError> {
    if points.len() < 2 {
        return Err(GeometryError::TooFewPoints);
    }
    let mut coordinates = points.to_vec();
    if coordinates.first() != coordinates.last() {
        coordinates.push(coordinates[0].clone());
    }
    Ok(open_segment_lengths(&coordinates)?.iter().sum())
}

/// Authoring helpers -------------------------------------------------------
///
/// Deterministic pixel-to-meter calibration from two image points and the real
/// world distance between them. Returns pixels per meter; must be positive.
pub fn pixels_per_meter(image_distance_px: f64, real_distance_m: f64) -> Option<f64> {
    if !image_distance_px.is_finite()
        || !real_distance_m.is_finite()
        || image_distance_px <= 0.0
        || real_distance_m <= 0.0
    {
        return None;
    }
    Some(image_distance_px / real_distance_m)
}

/// World distance between two points.
pub fn world_distance(a: &Point2, b: &Point2) -> f64 {
    (b.x - a.x).hypot(b.z - a.z)
}

/// Snap a point to a regular world grid.
pub fn snap_to_grid(point: &Point2, grid_m: f64) -> Point2 {
    if grid_m <= 0.0 || !grid_m.is_finite() {
        return point.clone();
    }
    Point2 {
        x: (point.x / grid_m).round() * grid_m,
        z: (point.z / grid_m).round() * grid_m,
    }
}

/// Return the id and squared distance of the nearest control point to `point`.
pub fn nearest_control_point(point: &Point2, points: &[ControlPoint]) -> Option<(String, f64)> {
    points
        .iter()
        .map(|control| {
            let candidate = Point2 {
                x: control.x,
                z: control.z,
            };
            let dx = candidate.x - point.x;
            let dz = candidate.z - point.z;
            (control.id.clone(), dx * dx + dz * dz)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn open_segment_lengths(points: &[Point2]) -> Result<Vec<f64>, GeometryError> {
    let mut lengths = Vec::with_capacity(points.len().saturating_sub(1));
    for pair in points.windows(2) {
        let start = &pair[0];
        let end = &pair[1];
        let dx = end.x - start.x;
        let dz = end.z - start.z;
        let length = dx.hypot(dz);
        if !length.is_finite() {
            return Err(GeometryError::NonFinitePoint);
        }
        if length <= f64::EPSILON {
            return Err(GeometryError::ZeroLengthSegment);
        }
        lengths.push(length);
    }
    Ok(lengths)
}

fn point(control: &ControlPoint) -> Point2 {
    Point2 {
        x: control.x,
        z: control.z,
    }
}

fn lerp(start: &Point2, end: &Point2, ratio: f64) -> Point2 {
    Point2 {
        x: start.x + (end.x - start.x) * ratio,
        z: start.z + (end.z - start.z) * ratio,
    }
}

fn normalize(vector: &Point2) -> Point2 {
    let length = vector.x.hypot(vector.z);
    Point2 {
        x: vector.x / length,
        z: vector.z / length,
    }
}

fn cross(left: &Point2, right: &Point2) -> f64 {
    left.x * right.z - left.z * right.x
}

/// Returns true when segments `ab` and `cd` strictly cross (endpoint touch
/// excluded), mirroring `svg_normalizer._proper_segment_intersect`.
fn proper_segment_intersect(a: &Point2, b: &Point2, c: &Point2, d: &Point2) -> bool {
    let o1 = cross_tri(a, b, c);
    let o2 = cross_tri(a, b, d);
    let o3 = cross_tri(c, d, a);
    let o4 = cross_tri(c, d, b);
    if o1 == 0.0 || o2 == 0.0 || o3 == 0.0 || o4 == 0.0 {
        return false;
    }
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn cross_tri(a: &Point2, b: &Point2, c: &Point2) -> f64 {
    (b.z - a.z) * (c.x - b.x) - (b.x - a.x) * (c.z - b.z)
}

fn point_on_segment(p: &Point2, a: &Point2, b: &Point2, tolerance: f64) -> bool {
    let cross = (b.x - a.x) * (p.z - a.z) - (b.z - a.z) * (p.x - a.x);
    if cross.abs() > tolerance {
        return false;
    }
    let length_squared = (b.x - a.x) * (b.x - a.x) + (b.z - a.z) * (b.z - a.z);
    if length_squared <= tolerance {
        return (p.x - a.x).hypot(p.z - a.z) <= tolerance;
    }
    let dot = (p.x - a.x) * (b.x - a.x) + (p.z - a.z) * (b.z - a.z);
    -tolerance <= dot && dot <= length_squared + tolerance
}

/// Detect strict crossings or touches between non-adjacent segments of a
/// closed polyline, mirroring `svg_normalizer.closed_polyline_self_intersects`.
pub fn closed_polyline_self_intersects(points: &[Point2]) -> bool {
    let n = points.len();
    if n < 4 {
        return false;
    }
    for i in 0..n {
        let a = &points[i];
        let b = &points[(i + 1) % n];
        for j in (i + 1)..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            let c = &points[j];
            let d = &points[(j + 1) % n];
            if proper_segment_intersect(a, b, c, d) {
                return true;
            }
            if point_on_segment(a, c, d, 1e-9) || point_on_segment(b, c, d, 1e-9) {
                return true;
            }
            if point_on_segment(c, a, b, 1e-9) || point_on_segment(d, a, b, 1e-9) {
                return true;
            }
        }
    }
    false
}

fn flatten_cubic_recursive(
    start: &Point2,
    control_a: &Point2,
    control_b: &Point2,
    end: &Point2,
    tolerance: f64,
    output: &mut Vec<Point2>,
) {
    let tangent_x = control_a.x - start.x;
    let tangent_z = control_a.z - start.z;
    let deviation_x = control_b.x - (3.0 * control_a.x - 2.0 * start.x);
    let deviation_z = control_b.z - (3.0 * control_a.z - 2.0 * start.z);
    let distance = (deviation_x * deviation_x + deviation_z * deviation_z).sqrt();
    let flat = distance <= tolerance
        && (tangent_x * tangent_x + tangent_z * tangent_z) <= tolerance * tolerance;
    if flat {
        output.push(end.clone());
        return;
    }

    let ab = midpoint(start, control_a);
    let bc = midpoint(control_a, control_b);
    let cd = midpoint(control_b, end);
    let abc = midpoint(&ab, &bc);
    let bcd = midpoint(&bc, &cd);
    let middle = midpoint(&abc, &bcd);
    flatten_cubic_recursive(start, &ab, &abc, &middle, tolerance, output);
    flatten_cubic_recursive(&middle, &bcd, &cd, end, tolerance, output);
}

fn midpoint(left: &Point2, right: &Point2) -> Point2 {
    Point2 {
        x: (left.x + right.x) * 0.5,
        z: (left.z + right.z) * 0.5,
    }
}

const PATH_COMMAND_PARAMS: &[(char, usize)] = &[
    ('M', 2),
    ('m', 2),
    ('L', 2),
    ('l', 2),
    ('H', 1),
    ('h', 1),
    ('V', 1),
    ('v', 1),
    ('C', 6),
    ('c', 6),
    ('Q', 4),
    ('q', 4),
    ('Z', 0),
    ('z', 0),
];

const COMMANDS: &str = "MmLlHhVvCcQqZz";

/// Parse an SVG path ``d`` string into flattened polylines (one per subpath),
/// mirroring the Python normalizer tokenizer and command handling. Cubic and
/// quadratic segments are flattened with the same recursive algorithm used by
/// `svg_normalizer.flatten_path`.
pub fn parse_path(d: &str, tolerance: f64) -> Result<Vec<Vec<Point2>>, GeometryError> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(GeometryError::InvalidTolerance);
    }
    let tokens = tokenize_path(d);
    validate_path_arity(&tokens)?;

    let mut subpaths: Vec<Vec<Point2>> = Vec::new();
    let mut current: Vec<Point2> = Vec::new();
    let mut cursor = Point2 { x: 0.0, z: 0.0 };
    let mut subpath_start = Point2 { x: 0.0, z: 0.0 };
    let mut index = 0;

    while index < tokens.len() {
        let command = tokens[index].chars().next().unwrap();
        index += 1;
        let mut params: Vec<f64> = Vec::new();
        while index < tokens.len() && !COMMANDS.contains(tokens[index].chars().next().unwrap()) {
            let value: f64 = tokens[index]
                .parse()
                .map_err(|_| GeometryError::NonFinitePoint)?;
            if !value.is_finite() {
                return Err(GeometryError::NonFinitePoint);
            }
            params.push(value);
            index += 1;
        }

        match command {
            'M' | 'm' => {
                for pair in params.chunks(2) {
                    let x = if command == 'm' {
                        cursor.x + pair[0]
                    } else {
                        pair[0]
                    };
                    let z = if command == 'm' {
                        cursor.z + pair[1]
                    } else {
                        pair[1]
                    };
                    cursor = Point2 { x, z };
                    subpath_start = cursor.clone();
                    if !current.is_empty() {
                        subpaths.push(std::mem::take(&mut current));
                    }
                    current.push(cursor.clone());
                }
            }
            'L' | 'l' => {
                for pair in params.chunks(2) {
                    let x = if command == 'l' {
                        cursor.x + pair[0]
                    } else {
                        pair[0]
                    };
                    let z = if command == 'l' {
                        cursor.z + pair[1]
                    } else {
                        pair[1]
                    };
                    cursor = Point2 { x, z };
                    current.push(cursor.clone());
                }
            }
            'H' => {
                cursor.x = params[params.len() - 1];
                current.push(cursor.clone());
            }
            'h' => {
                cursor.x += params[params.len() - 1];
                current.push(cursor.clone());
            }
            'V' => {
                cursor.z = params[params.len() - 1];
                current.push(cursor.clone());
            }
            'v' => {
                cursor.z += params[params.len() - 1];
                current.push(cursor.clone());
            }
            'C' | 'c' => {
                for chunk in params.chunks(6) {
                    let (c1, c2, end) = if command == 'c' {
                        (
                            Point2 {
                                x: cursor.x + chunk[0],
                                z: cursor.z + chunk[1],
                            },
                            Point2 {
                                x: cursor.x + chunk[2],
                                z: cursor.z + chunk[3],
                            },
                            Point2 {
                                x: cursor.x + chunk[4],
                                z: cursor.z + chunk[5],
                            },
                        )
                    } else {
                        (
                            Point2 {
                                x: chunk[0],
                                z: chunk[1],
                            },
                            Point2 {
                                x: chunk[2],
                                z: chunk[3],
                            },
                            Point2 {
                                x: chunk[4],
                                z: chunk[5],
                            },
                        )
                    };
                    current.extend(flatten_cubic(&cursor, &c1, &c2, &end, tolerance)?);
                    cursor = end;
                }
            }
            'Q' | 'q' => {
                for chunk in params.chunks(4) {
                    let (q, end) = if command == 'q' {
                        (
                            Point2 {
                                x: cursor.x + chunk[0],
                                z: cursor.z + chunk[1],
                            },
                            Point2 {
                                x: cursor.x + chunk[2],
                                z: cursor.z + chunk[3],
                            },
                        )
                    } else {
                        (
                            Point2 {
                                x: chunk[0],
                                z: chunk[1],
                            },
                            Point2 {
                                x: chunk[2],
                                z: chunk[3],
                            },
                        )
                    };
                    let c1 = Point2 {
                        x: cursor.x + 2.0 / 3.0 * (q.x - cursor.x),
                        z: cursor.z + 2.0 / 3.0 * (q.z - cursor.z),
                    };
                    let c2 = Point2 {
                        x: end.x + 2.0 / 3.0 * (q.x - end.x),
                        z: end.z + 2.0 / 3.0 * (q.z - end.z),
                    };
                    current.extend(flatten_cubic(&cursor, &c1, &c2, &end, tolerance)?);
                    cursor = end;
                }
            }
            'Z' | 'z' => {
                if current.last() != Some(&subpath_start) {
                    current.push(subpath_start.clone());
                }
                cursor = subpath_start.clone();
            }
            _ => {}
        }
    }
    if !current.is_empty() {
        subpaths.push(current);
    }
    Ok(subpaths)
}

fn tokenize_path(d: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut number = String::new();
    for character in d.replace(['\n', ','], " ").chars() {
        if COMMANDS.contains(character) {
            if !number.trim().is_empty() {
                tokens.extend(number.split_whitespace().map(str::to_owned));
                number.clear();
            }
            tokens.push(character.to_string());
        } else {
            number.push(character);
        }
    }
    if !number.trim().is_empty() {
        tokens.extend(number.split_whitespace().map(str::to_owned));
    }
    tokens
}

fn validate_path_arity(tokens: &[String]) -> Result<(), GeometryError> {
    if tokens.is_empty() || !matches!(tokens[0].chars().next(), Some('M') | Some('m')) {
        return Err(GeometryError::NonFinitePoint);
    }
    let mut command: Option<char> = None;
    let mut count = 0usize;
    for token in tokens {
        let first = token.chars().next().unwrap();
        if COMMANDS.contains(first) {
            if let Some(previous) = command {
                check_command_params(previous, count)?;
            }
            command = Some(first);
            count = 0;
        } else {
            count += 1;
        }
    }
    if let Some(previous) = command {
        check_command_params(previous, count)?;
    }
    Ok(())
}

fn check_command_params(command: char, count: usize) -> Result<(), GeometryError> {
    let step = PATH_COMMAND_PARAMS
        .iter()
        .find(|(c, _)| *c == command)
        .map(|(_, n)| *n)
        .ok_or(GeometryError::NonFinitePoint)?;
    if step == 0 {
        return if count == 0 {
            Ok(())
        } else {
            Err(GeometryError::NonFinitePoint)
        };
    }
    if count == 0 || !count.is_multiple_of(step) {
        return Err(GeometryError::NonFinitePoint);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<ControlPoint> {
        [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]
            .into_iter()
            .enumerate()
            .map(|(index, (x, z))| ControlPoint {
                id: format!("p{index}"),
                x,
                z,
                handle_in: None,
                handle_out: None,
            })
            .collect()
    }

    #[test]
    fn closed_length_includes_seam() {
        assert_eq!(closed_centerline_length(&square()).unwrap(), 40.0);
    }

    #[test]
    fn station_wraps_and_returns_position_and_tangent() {
        let sample = sample_closed_centerline(&square(), 45.0).unwrap();
        assert_eq!(sample.station, 5.0);
        assert_eq!(sample.position, Point2 { x: 5.0, z: 0.0 });
        assert_eq!(sample.tangent, Point2 { x: 1.0, z: 0.0 });
    }

    #[test]
    fn invalid_centerlines_are_rejected() {
        assert_eq!(
            closed_centerline_length(&square()[..2]),
            Err(GeometryError::TooFewPoints)
        );
        let mut invalid = square();
        invalid[1].x = invalid[0].x;
        invalid[1].z = invalid[0].z;
        assert_eq!(
            closed_centerline_length(&invalid),
            Err(GeometryError::ZeroLengthSegment)
        );
    }

    #[test]
    fn resampling_uses_fixed_arc_length_count() {
        let points = square()
            .into_iter()
            .map(|point| Point2 {
                x: point.x,
                z: point.z,
            })
            .collect::<Vec<_>>();
        let samples = resample_closed_polyline(&points, 10.0).unwrap();
        assert_eq!(samples.len(), 4);
        assert_eq!(samples[1], Point2 { x: 10.0, z: 0.0 });
    }

    #[test]
    fn cubic_flattening_is_deterministic_and_ends_at_curve_endpoint() {
        let points = flatten_cubic(
            &Point2 { x: 0.0, z: 0.0 },
            &Point2 { x: 0.0, z: 10.0 },
            &Point2 { x: 10.0, z: 10.0 },
            &Point2 { x: 10.0, z: 0.0 },
            0.1,
        )
        .unwrap();
        assert!(points.len() > 2);
        assert_eq!(points.last(), Some(&Point2 { x: 10.0, z: 0.0 }));
        assert_eq!(
            points,
            flatten_cubic(
                &Point2 { x: 0.0, z: 0.0 },
                &Point2 { x: 0.0, z: 10.0 },
                &Point2 { x: 10.0, z: 10.0 },
                &Point2 { x: 10.0, z: 0.0 },
                0.1,
            )
            .unwrap()
        );
    }

    #[test]
    fn self_intersection_detects_bowtie_but_accepts_simple_loop() {
        let simple = vec![
            Point2 { x: 0.0, z: 0.0 },
            Point2 { x: 10.0, z: 0.0 },
            Point2 { x: 10.0, z: 10.0 },
            Point2 { x: 0.0, z: 10.0 },
        ];
        assert!(!closed_polyline_self_intersects(&simple));

        let bowtie = vec![
            Point2 { x: 0.0, z: 0.0 },
            Point2 { x: 10.0, z: 10.0 },
            Point2 { x: 0.0, z: 10.0 },
            Point2 { x: 10.0, z: 0.0 },
        ];
        assert!(closed_polyline_self_intersects(&bowtie));
    }

    #[test]
    fn calibration_is_deterministic_and_rejects_bad_input() {
        assert_eq!(pixels_per_meter(100.0, 10.0), Some(10.0));
        assert_eq!(pixels_per_meter(200.0, 5.0), Some(40.0));
        assert_eq!(pixels_per_meter(0.0, 10.0), None);
        assert_eq!(pixels_per_meter(f64::NAN, 10.0), None);
    }

    #[test]
    fn world_distance_and_snapping() {
        let a = Point2 { x: 0.0, z: 0.0 };
        let b = Point2 { x: 3.0, z: 4.0 };
        assert_eq!(world_distance(&a, &b), 5.0);

        let snapped = snap_to_grid(&Point2 { x: 3.7, z: -1.2 }, 2.0);
        assert_eq!(snapped, Point2 { x: 4.0, z: -2.0 });
    }

    #[test]
    fn nearest_control_point_finds_closest() {
        let controls = vec![
            ControlPoint {
                id: "a".into(),
                x: 0.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "b".into(),
                x: 10.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
        ];
        let (id, _) = nearest_control_point(&Point2 { x: 9.0, z: 1.0 }, &controls).unwrap();
        assert_eq!(id, "b");
    }
}
