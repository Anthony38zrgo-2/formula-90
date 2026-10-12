pub mod complete_profile;

pub use complete_profile::{
    load_complete_vehicle_profile, CompleteVehicleProfile, CompleteVehicleProfileProvenance,
    VehicleProfileCoordinationError, COMPLETE_VEHICLE_PROFILE_CONTRACT_VERSION,
};
