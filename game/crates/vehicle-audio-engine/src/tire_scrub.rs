//! Deterministic tyre-scrub control signal derived from per-wheel physics.

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TireScrubMode {
    #[default]
    None = 0,
    Lateral = 1,
    Wheelspin = 2,
    Lockup = 3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TireScrubTarget {
    pub gain: f32,
    pub pitch: f32,
    pub mode: TireScrubMode,
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Compute one continuous scrub voice from four signed longitudinal slip ratios,
/// lateral slip angles, suspension contact fractions and normal loads.
pub fn compute_tire_scrub_target(
    slip_ratio: [f32; 4],
    slip_angle_rad: [f32; 4],
    contact_fraction: [f32; 4],
    normal_force_n: [f32; 4],
    speed_kph: f32,
    surface: &str,
) -> TireScrubTarget {
    let speed_gate = smoothstep(5.0, 20.0, speed_kph.abs());
    if speed_gate <= 0.0 {
        return TireScrubTarget::default();
    }

    let surface_gain = match surface {
        "asphalt" => 1.0,
        "rumble" => 0.70,
        "grass" | "sand" => 0.22,
        _ => 0.50,
    };
    let mut best = 0.0f32;
    let mut mode = TireScrubMode::None;
    let mut sum = 0.0f32;

    for i in 0..4 {
        let contact = contact_fraction[i].clamp(0.0, 1.0);
        let load = smoothstep(150.0, 3500.0, normal_force_n[i].max(0.0));
        let authority = contact * load;
        if authority <= 0.0 {
            continue;
        }

        let lateral = smoothstep(0.04, 0.18, slip_angle_rad[i].abs()) * authority;
        let wheelspin = smoothstep(0.08, 0.35, slip_ratio[i]) * authority;
        let lockup = smoothstep(0.12, 0.70, -slip_ratio[i]) * authority;
        let (severity, candidate) = if lockup >= wheelspin && lockup >= lateral {
            (lockup, TireScrubMode::Lockup)
        } else if wheelspin >= lateral {
            (wheelspin, TireScrubMode::Wheelspin)
        } else {
            (lateral, TireScrubMode::Lateral)
        };
        sum += severity;
        if severity > best {
            best = severity;
            mode = candidate;
        }
    }

    // The loudest wheel defines the event, with a restrained lift when multiple
    // tyres scrub together (four-wheel slide or braking lockup).
    let intensity = (best + 0.10 * (sum - best).max(0.0)).clamp(0.0, 1.0);
    let gain = intensity * speed_gate * surface_gain;
    if gain <= 1e-4 {
        return TireScrubTarget::default();
    }
    let pitch = match mode {
        TireScrubMode::Lockup => 0.82 + 0.08 * intensity,
        TireScrubMode::Lateral => 0.95 + 0.10 * intensity,
        TireScrubMode::Wheelspin => 1.05 + 0.15 * intensity,
        TireScrubMode::None => 1.0,
    };
    TireScrubTarget { gain, pitch, mode }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTACT: [f32; 4] = [1.0; 4];
    const LOAD: [f32; 4] = [3500.0; 4];

    #[test]
    fn normal_running_is_silent() {
        assert_eq!(
            compute_tire_scrub_target([0.02; 4], [0.01; 4], CONTACT, LOAD, 100.0, "asphalt"),
            TireScrubTarget::default()
        );
    }

    #[test]
    fn airborne_and_walking_speed_are_silent() {
        assert_eq!(
            compute_tire_scrub_target([1.0; 4], [1.0; 4], [0.0; 4], LOAD, 100.0, "asphalt").gain,
            0.0
        );
        assert_eq!(
            compute_tire_scrub_target([1.0; 4], [1.0; 4], CONTACT, LOAD, 4.0, "asphalt").gain,
            0.0
        );
    }

    #[test]
    fn distinguishes_lateral_wheelspin_and_lockup() {
        let lateral =
            compute_tire_scrub_target([0.0; 4], [0.20; 4], CONTACT, LOAD, 80.0, "asphalt");
        let spin = compute_tire_scrub_target([0.40; 4], [0.0; 4], CONTACT, LOAD, 80.0, "asphalt");
        let lock = compute_tire_scrub_target([-0.80; 4], [0.0; 4], CONTACT, LOAD, 80.0, "asphalt");
        assert_eq!(lateral.mode, TireScrubMode::Lateral);
        assert_eq!(spin.mode, TireScrubMode::Wheelspin);
        assert_eq!(lock.mode, TireScrubMode::Lockup);
        assert!(lock.pitch < lateral.pitch && lateral.pitch < spin.pitch);
    }

    #[test]
    fn loose_surface_is_strongly_attenuated() {
        let road = compute_tire_scrub_target([0.5; 4], [0.0; 4], CONTACT, LOAD, 80.0, "asphalt");
        let grass = compute_tire_scrub_target([0.5; 4], [0.0; 4], CONTACT, LOAD, 80.0, "grass");
        assert!(grass.gain < road.gain * 0.25);
    }
}
