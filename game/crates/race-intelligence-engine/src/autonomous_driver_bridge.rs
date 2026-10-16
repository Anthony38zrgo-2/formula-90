use race_intelligence_engine::local_driver::autonomous_driver::{
    AutonomousDriver, DriverObservation,
};
use race_intelligence_engine::local_driver::driver_tuning::load_driver_tuning;
use race_intelligence_engine::track::path_reader::load_racing_line_from_document;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.get(1).map(String::as_str) == Some("--validate-tuning") {
        let path = arguments.get(2).ok_or("Expected driver tuning path")?;
        load_driver_tuning(&std::fs::read_to_string(path)?)?;
        println!("AUTONOMOUS_DRIVER_TUNING_VALID path={path}");
        return Ok(());
    }
    if arguments.get(1).map(String::as_str) == Some("--write-steering-mapping") {
        let output_path = arguments.get(2).ok_or("Expected steering mapping output path")?;
        let repository_root = std::path::Path::new(arguments.get(3).ok_or("Expected repository root")?);
        let vehicle_profile = std::path::Path::new(arguments.get(4).ok_or("Expected vehicle profile path")?);
        let configuration = vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root(vehicle_profile, repository_root)?;
        let calibration = race_intelligence_engine::local_driver::steering_rack_calibration::SteeringRackCalibration::from_configuration(&configuration)?;
        std::fs::write(output_path, serde_json::to_string(&calibration.canonical_command_mapping(&configuration))?)?;
        return Ok(());
    }
    let document = std::fs::read_to_string(arguments.get(1).ok_or("Expected racing line path")?)?;
    let tuning = load_driver_tuning(&std::fs::read_to_string(
        arguments.get(3).ok_or("Expected driver tuning path")?,
    )?)?;
    let repository_root = std::path::Path::new(arguments.get(4).ok_or("Expected authorized repository root")?);
    let vehicle_profile = std::path::Path::new(arguments.get(5).ok_or("Expected vehicle profile path")?);
    let configuration = vehicle_physics_engine::vehicle_configuration::load_vehicle_configuration_from_authorized_root(vehicle_profile, repository_root)?;
    let mut driver = AutonomousDriver::new(load_racing_line_from_document(&document)?, tuning, &configuration)?;
    let listener = TcpListener::bind((
        "127.0.0.1",
        arguments.get(2).ok_or("Expected port")?.parse::<u16>()?,
    ))?;
    eprintln!("AUTONOMOUS_DRIVER_LISTENING address={}", listener.local_addr()?);
    let (mut connection, _) = listener.accept()?;
    connection.set_nodelay(true)?;
    connection.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
    connection.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;
    let mut reader = BufReader::new(connection.try_clone()?);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let response = serde_json::from_str::<DriverObservation>(&line)
            .map_err(|error| error.to_string())
            .and_then(|observation| driver.command(observation));
        let document = match response {
            Ok(command) => serde_json::json!({"success": true, "command": command}),
            Err(error) => serde_json::json!({"success": false, "error": error}),
        };
        writeln!(connection, "{document}")?;
    }
    Ok(())
}
