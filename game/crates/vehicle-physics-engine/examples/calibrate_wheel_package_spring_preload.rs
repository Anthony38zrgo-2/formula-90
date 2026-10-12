use std::path::Path;
use vehicle_physics_engine::suspension_kinematics::jacobian;
use vehicle_physics_engine::{VehicleConfig, WheelIndex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile_path = std::env::args().nth(1).ok_or("Expected vehicle profile path")?;
    let configuration = VehicleConfig::from_json_path(Path::new(&profile_path))?;
    let geometry = configuration.suspension.geometric_suspension.as_ref().ok_or("Expected physical suspension geometry")?;
    for wheel in [WheelIndex::FrontLeft, WheelIndex::RearLeft] {
        let axle = geometry.axle(wheel);
        let motion_ratio = jacobian(geometry.corners.get(wheel), 0.0, 0.0, wheel.is_front()).ok_or("Suspension rest pose did not converge")?.motion_ratio;
        let wheel_load_newtons = configuration.mass_over_wheel(wheel) * 9.80665;
        let free_length_metres = axle.spring_installed_length_m + wheel_load_newtons / (axle.spring_rate_N_per_m * motion_ratio);
        println!("{wheel:?} motion_ratio={motion_ratio} wheel_load_newtons={wheel_load_newtons} spring_free_length_metres={free_length_metres}");
    }
    Ok(())
}
