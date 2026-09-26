//! ABI test for the pit-service facade calls: explicit refuel targets and a
//! fresh cold tire set applied while the orchestrator owns the entity.

use std::ffi::CString;

use formula90_core::ffi::{
    f90_core_abi_version, f90_core_create, f90_core_destroy, f90_core_replace_tires,
    f90_core_set_fuel_kg, f90_core_spawn, f90_core_step, F90CoreFrameOut, F90TriRaycastSample,
};

const DT: f64 = 1.0 / 120.0;

#[test]
fn pit_service_facade_refuels_and_replaces_tires() {
    assert_eq!(f90_core_abi_version(), 17);

    let config_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/vehicles/f1_2030/f1_2030_v10_geometric.json");
    let options = format!(
        "{{\"config_json_path\":\"{}\",\"enable_audio\":false}}",
        config_path.to_string_lossy().replace('\\', "/")
    );
    let options_c = CString::new(options).expect("options must be a valid C string");
    let handle = unsafe { f90_core_create(options_c.as_ptr(), std::ptr::null_mut(), 0) };
    assert!(!handle.is_null(), "facade creation must succeed");
    let id = f90_core_spawn(handle);
    assert_ne!(id, 0, "entity must spawn");

    assert!(f90_core_set_fuel_kg(handle, id, 37.95));
    assert!(!f90_core_set_fuel_kg(handle, id.wrapping_add(999), 10.0));
    assert!(f90_core_replace_tires(handle, id));
    assert!(!f90_core_replace_tires(handle, id.wrapping_add(999)));

    let samples = [unsafe { std::mem::zeroed::<F90TriRaycastSample>() }; 4];
    let mut frame = F90CoreFrameOut::default();
    // SAFETY: the handle comes from `f90_core_create`, the samples array holds
    // four valid PODs and `frame` points to a writable output block.
    unsafe {
        f90_core_step(
            handle,
            id,
            -297.652,
            0.35,
            -244.251,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            -2,
            0,
            DT,
            samples.as_ptr(),
            std::ptr::null(),
            &mut frame,
        );
    }
    assert!(
        (frame.fuel_remaining_kg - 37.95).abs() < 0.01,
        "the frame must report the service target, got {}",
        frame.fuel_remaining_kg
    );
    assert_eq!(frame.tire_wear_remaining_fraction, [1.0; 4]);

    // SAFETY: `handle` is still the live facade handle from `f90_core_create`.
    unsafe { f90_core_destroy(handle) };
}
