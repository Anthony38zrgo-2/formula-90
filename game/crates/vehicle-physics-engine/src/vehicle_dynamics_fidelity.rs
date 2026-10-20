use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VehicleDynamicsFidelity {
    #[default]
    Full,
    Simplified,
}

impl VehicleDynamicsFidelity {
    pub fn is_simplified(self) -> bool {
        self == Self::Simplified
    }
}
