//! Control-rate adaptation only; never advances or changes vehicle physics.
use vehicle_physics_engine::{PowertrainState, VehicleConfig};
use vehicle_audio_engine::ffi::{VehicleAudioTelemetryV3, VEHICLE_AUDIO_ABI_VERSION};
use v10_engine_synth::ShiftPhase;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MechanicalAudioState {
    pub load: f32,
    pub torque: f32,
    pub clutch: f32,
    pub tc: f32,
    pub limiter: bool,
    pub shifting: bool,
    pub downshift: bool,
}

impl MechanicalAudioState {
    pub fn from_physics(p: &PowertrainState, config: &VehicleConfig) -> Self {
        let torque = (p.engine_torque / config.max_torque.max(1.0)).clamp(-1.0, 1.0) as f32;
        // Clutch torque already contains engagement. Preserve braking magnitude.
        // TC acts downstream on positive drive torque in this physics model;
        // reflect that reduction once in acoustic delivered load, not in torque.
        let mut load = if p.clutch_engagement > 1e-4 {
            (p.clutch_torque.abs() / config.max_torque.max(1.0)).clamp(0.0, 1.0)
        } else { 0.0 };
        if p.engine_torque > 0.0 { load *= 1.0 - p.tc_cut_ratio_smoothed.clamp(0.0, 1.0); }
        Self { load: load as f32, torque,
            clutch: p.clutch_engagement.clamp(0.0, 1.0) as f32,
            tc: p.tc_cut_ratio_smoothed.clamp(0.0, 1.0) as f32,
            limiter: p.is_rev_limited, shifting: p.shift_timer > 0.0,
            downshift: p.target_gear < p.current_gear }
    }
}

#[derive(Default)]
pub(crate) struct AudioTelemetryAdapter {
    previous: Option<(f32, f32)>,
    rpm_dot: f32,
    throttle_dot: f32,
    was_shifting: bool,
    downshift: bool,
    recovery_s: f32,
}

impl AudioTelemetryAdapter {
    #[allow(clippy::too_many_arguments)]
    pub fn update(&mut self, rpm: f64, idle: f64, max: f64, throttle: f32,
        speed: f64, gear: i32, slip: f32, dt: f32, physical: MechanicalAudioState
    ) -> VehicleAudioTelemetryV3 {
        // Invalid timing resets derivatives; first observation never invents a spike.
        let valid_dt = dt.is_finite() && dt > 0.0 && dt <= 0.1;
        if let Some((last_rpm, last_throttle)) = self.previous.filter(|_| valid_dt) {
            let alpha = 1.0 - (-dt / 0.020).exp();
            self.rpm_dot += alpha * ((rpm as f32 - last_rpm) / dt - self.rpm_dot);
            self.throttle_dot += alpha * ((throttle - last_throttle) / dt - self.throttle_dot);
        } else { self.rpm_dot = 0.0; self.throttle_dot = 0.0; }
        self.previous = Some((rpm as f32, throttle));
        // Cut is physical. Recovery is a named 40 ms acoustic transition, not
        // an invented transmission phase. No synthetic downshift blip is inferred.
        let phase = if physical.shifting {
            self.downshift = physical.downshift;
            self.recovery_s = 0.0;
            if self.downshift { ShiftPhase::DownshiftCut } else { ShiftPhase::UpshiftCut }
        } else {
            if self.was_shifting { self.recovery_s = 0.040; }
            let phase = if self.recovery_s > 0.0 {
                if self.downshift { ShiftPhase::DownshiftRecovery } else { ShiftPhase::UpshiftRecovery }
            } else { ShiftPhase::None };
            self.recovery_s = (self.recovery_s - if valid_dt { dt } else { 0.0 }).max(0.0);
            phase
        };
        self.was_shifting = physical.shifting;
        VehicleAudioTelemetryV3 {
            schema_version: VEHICLE_AUDIO_ABI_VERSION,
            struct_size: std::mem::size_of::<VehicleAudioTelemetryV3>() as u32,
            rpm, idle_rpm: idle, max_rpm: max, throttle,
            normalized_engine_load: physical.load, normalized_engine_torque: physical.torque,
            torque_sign: if physical.torque > 0.025 { 1 } else if physical.torque < -0.025 { -1 } else { 0 },
            rpm_derivative: self.rpm_dot, throttle_derivative: self.throttle_dot,
            speed_kph: speed, gear, slip, shift_phase: phase as i32,
            clutch_engagement: physical.clutch, tc_cut_ratio: physical.tc,
            rev_limiter_active: u32::from(physical.limiter),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_mapping_keeps_signed_torque_and_applies_tc_once() {
        let config = VehicleConfig::default();
        let max = config.max_torque;
        let mut p = PowertrainState::new(&config);
        p.clutch_engagement = 1.0;
        p.clutch_torque = max * 0.5;
        p.engine_torque = max * 0.5;
        p.tc_cut_ratio_smoothed = 0.4;
        let drive = MechanicalAudioState::from_physics(&p, &config);
        assert!((drive.torque - 0.5).abs() < 1e-6);
        assert!((drive.load - 0.3).abs() < 1e-6);

        p.engine_torque = -max * 0.5;
        p.clutch_torque = -max * 0.5;
        let coast = MechanicalAudioState::from_physics(&p, &config);
        assert!((coast.torque + 0.5).abs() < 1e-6);
        assert!((coast.load - 0.5).abs() < 1e-6);

        p.clutch_engagement = 0.0;
        p.clutch_torque = 0.0;
        let free_rev = MechanicalAudioState::from_physics(&p, &config);
        assert_eq!(free_rev.load, 0.0);
        assert_eq!(free_rev.clutch, 0.0);
    }

    #[test]
    fn shift_cut_has_only_physical_cut_then_acoustic_recovery() {
        let mut adapter = AudioTelemetryAdapter::default();
        let up = MechanicalAudioState {
            shifting: true,
            ..MechanicalAudioState::default()
        };
        let cut = adapter.update(9_000.0, 1_000.0, 15_000.0, 0.8, 100.0, 4, 0.0, 1.0 / 120.0, up);
        assert_eq!(cut.shift_phase, ShiftPhase::UpshiftCut as i32);
        let settled = adapter.update(9_000.0, 1_000.0, 15_000.0, 0.8, 100.0, 4, 0.0, 1.0 / 120.0, MechanicalAudioState::default());
        assert_eq!(settled.shift_phase, ShiftPhase::UpshiftRecovery as i32);
        assert_eq!(settled.torque_sign, 0);

        let down_limiter = MechanicalAudioState {
            shifting: true,
            downshift: true,
            limiter: true,
            ..MechanicalAudioState::default()
        };
        let packet = adapter.update(
            9_000.0, 1_000.0, 15_000.0, 0.2, 100.0, 3, 0.0, 1.0 / 120.0,
            down_limiter,
        );
        assert_eq!(packet.shift_phase, ShiftPhase::DownshiftCut as i32);
        assert_eq!(packet.rev_limiter_active, 1);
    }
}
