//! Wiebe-style combustion heat release driving the cylinder P/T gas state.
//!
//! This is a simplified single-zone burn model: instead of a flame-front
//! tracking the fuel mixture (which engine-sim does with a full gas network,
//! rejected in PHY-003), the burned mass fraction follows a Wiebe function and
//! the released heat is added to the post-compression internal energy. Pressure
//! then follows from the ideal gas law at the instantaneous chamber volume.
//!
//! The model is a pure function of crank angle: no per-sample allocation and no
//! hidden state beyond what a cylinder already tracks (age since firing).

use crate::config::EngineConfig;
use crate::geometry::SliderCrank;
use crate::thermodynamics::gas::{GasState, CV_AIR, R_SPECIFIC_AIR};
use crate::thermodynamics::polytrope::PolytropicProcess;

/// Wiebe completeness factor `a`; `X_end = 1 - exp(-a)`.
pub const WIEBE_A: f32 = 5.0;
/// Wiebe shape factor `m`.
pub const WIEBE_M: f32 = 3.0;
/// Intake manifold pressure at the start of compression, Pa.
pub const INTAKE_PRESSURE_PA: f32 = 101_325.0;
/// Intake manifold temperature, K.
pub const INTAKE_TEMPERATURE_K: f32 = 300.0;

#[derive(Clone, Copy, Debug)]
pub struct CombustionFrame {
    pub pressure_pa: f32,
    pub temperature_k: f32,
    pub burn_fraction: f32,
    pub heat_release_rate_w: f32,
}

/// A single-zone combustion chamber driven by Wiebe heat release.
#[derive(Clone, Copy, Debug)]
pub struct CombustionChamber {
    ignition_deg: f32,
    burn_duration_deg: f32,
    shape_factor: f32,
    efficiency: f32,
    fuel_energy_j: f32,
    charge: GasState,
    geometry: SliderCrank,
    polytrope: PolytropicProcess,
}

impl CombustionChamber {
    /// Build from configuration. The charge is derived from intake conditions
    /// at BDC (maximum volume) so that the polytropic compression reference is
    /// a proper ideal-gas state.
    pub fn new(config: &EngineConfig) -> Self {
        let geometry = SliderCrank::from_config(config);
        let v_bdc = geometry.instantaneous_volume_m3(180.0);
        let mass = INTAKE_PRESSURE_PA * v_bdc / (R_SPECIFIC_AIR * INTAKE_TEMPERATURE_K);
        let charge = GasState::new(INTAKE_PRESSURE_PA, INTAKE_TEMPERATURE_K, mass, v_bdc);
        Self {
            ignition_deg: config.combustion_start_deg,
            burn_duration_deg: config.combustion_rise_deg,
            shape_factor: config.combustion_shape_factor,
            efficiency: config.combustion_efficiency,
            fuel_energy_j: config.fuel_energy_per_cycle,
            charge,
            geometry,
            polytrope: PolytropicProcess::adiabatic(),
        }
    }

    /// Cumulative Wiebe burned-mass fraction at `deg` past ignition.
    pub fn burn_fraction(&self, deg_past_ignition: f32) -> f32 {
        if deg_past_ignition <= 0.0 || self.burn_duration_deg <= 0.0 {
            return 0.0;
        }
        // Clamp to the well-defined burn window to avoid overflow on exp.
        let u = (deg_past_ignition / self.burn_duration_deg).clamp(0.0, 1.0);
        1.0 - (-WIEBE_A * u.powf(self.shape_factor + 1.0)).exp()
    }

    /// Wiebe heat-release rate (`dQ/ddeg`), in Joules per degree.
    pub fn heat_release_rate_per_deg(&self, deg_past_ignition: f32, energy: f32) -> f32 {
        if deg_past_ignition <= 0.0 || self.burn_duration_deg <= 0.0 {
            return 0.0;
        }
        let u = (deg_past_ignition / self.burn_duration_deg).clamp(0.0, 1.0);
        if u >= 1.0 {
            return 0.0;
        }
        let dfrac = self.d_burn_fraction_d_deg(u);
        self.fuel_energy_j * self.efficiency * energy * dfrac
    }

    fn d_burn_fraction_d_deg(&self, u: f32) -> f32 {
        let m1 = self.shape_factor + 1.0;
        let outer = (-WIEBE_A * u.powf(m1)).exp();
        WIEBE_A * m1 / self.burn_duration_deg * u.powf(self.shape_factor) * outer
    }

    /// Combustion state at `age_deg` past firing (TDC at `age = 0`).
    ///
    /// Pressure is `P = m * R * T / V` where the temperature is the polytropic
    /// compression temperature plus the heat-released energy divided by
    /// `m * cv`. This reduces exactly to the compression state at zero heat.
    pub fn frame(&self, age_deg: f32, energy: f32) -> CombustionFrame {
        let volume = self.geometry.instantaneous_volume_m3(age_deg);
        let comp = self
            .polytrope
            .state_at(&self.charge, volume);
        let deg_past_ignition = age_deg - self.ignition_deg;
        let frac = self.burn_fraction(deg_past_ignition);
        let heat_j = self.fuel_energy_j * self.efficiency * energy * frac;
        let temperature_k =
            comp.temperature_k + heat_j / (self.charge.mass_kg * CV_AIR);
        let frac_clamped = (deg_past_ignition as f32).max(0.0);
        let rate = self.heat_release_rate_per_deg(frac_clamped, energy);
        CombustionFrame {
            pressure_pa: self.charge.mass_kg * R_SPECIFIC_AIR * temperature_k / volume,
            temperature_k,
            burn_fraction: frac,
            heat_release_rate_w: rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chamber() -> CombustionChamber {
        CombustionChamber::new(&EngineConfig::default())
    }

    #[test]
    fn burn_fraction_starts_zero_and_reaches_completeness() {
        let c = chamber();
        assert_eq!(c.burn_fraction(0.0), 0.0);
        assert_eq!(c.burn_fraction(-5.0), 0.0);
        let end = c.burn_fraction(c.burn_duration_deg);
        assert!((end - (1.0 - (-WIEBE_A).exp())).abs() < 1.0e-4);
        assert!(end > 0.98);
    }

    #[test]
    fn heat_release_raises_pressure_and_temperature_above_compression() {
        let c = chamber();
        let low = c.frame(10.0, 0.3);
        let high = c.frame(10.0, 1.0);
        let comp = c
            .polytrope
            .state_at(&c.charge, c.geometry.instantaneous_volume_m3(10.0));
        // Compression-only baseline at the same angle.
        assert!(high.pressure_pa >= comp.pressure_pa);
        assert!(high.temperature_k > comp.temperature_k);
        assert!(high.pressure_pa > low.pressure_pa, "more energy -> more pressure");
    }

    #[test]
    fn heat_release_rate_is_crank_angle_coherent() {
        let c = chamber();
        let mut peak = f32::MIN;
        let mut peak_deg = f32::MIN;
        for deg in (0..=c.burn_duration_deg as usize).map(|d| d as f32) {
            let rate = c.heat_release_rate_per_deg(deg, 1.0);
            assert!(rate.is_finite() && rate >= 0.0);
            if rate > peak {
                peak = rate;
                peak_deg = deg;
            }
            assert!(rate >= -1.0e-6, "rate must not go negative near {deg}");
        }
        assert!(peak > 0.0);
        assert!(peak_deg > 0.0, "heat release must ramp up after ignition");
    }

    #[test]
    fn pressure_is_bounded_and_finite_over_full_cycle() {
        let c = chamber();
        let mut peak = 0.0f32;
        let mut min = f32::MAX;
        for i in 0..=720 {
            let deg = i as f32;
            let f = c.frame(deg, 1.0);
            assert!(f.pressure_pa.is_finite() && f.pressure_pa > 0.0);
            assert!(f.temperature_k.is_finite() && f.temperature_k > 0.0);
            peak = peak.max(f.pressure_pa);
            min = min.min(f.pressure_pa);
        }
        assert!(peak > min);
        assert!(peak < 5.0e7, "peak pressure must stay within physical bounds");
    }

    #[test]
    fn no_heat_before_ignition_matches_pure_compression() {
        let c = chamber();
        let pre = c.frame(-20.0, 1.0);
        let comp = c
            .polytrope
            .state_at(&c.charge, c.geometry.instantaneous_volume_m3(-20.0));
        assert!((pre.temperature_k - comp.temperature_k).abs() < 1.0e-2);
        assert!((pre.burn_fraction).abs() < 1.0e-6);
    }

    #[test]
    #[ignore]
    fn print_peak_magnitudes() {
        let c = chamber();
        for energy in [0.0f32, 0.25, 0.5, 0.76, 1.0] {
            let mut peak = 0.0f32;
            let mut peak_deg = 0.0f32;
            let mut tpeak = 0.0f32;
            for i in -200..=300 {
                let deg = i as f32;
                let f = c.frame(deg, energy);
                if f.pressure_pa > peak {
                    peak = f.pressure_pa;
                    peak_deg = deg;
                    tpeak = f.temperature_k;
                }
            }
            eprintln!(
                "energy={energy:.2} peak={:.3} MPa at {peak_deg:.0}deg T={tpeak:.0}K",
                peak / 1.0e6
            );
        }
    }
}
