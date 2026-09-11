#![forbid(unsafe_code)]

pub mod acoustics;
pub mod config;
pub mod crank;
pub mod cylinder;
pub mod engine;
pub mod exhaust;
pub mod geometry;
pub mod runtime;
pub mod sample_layer;
pub mod scene;
pub mod thermodynamics;
pub mod wav;

pub use config::EngineConfig;
pub use engine::{EngineFrame, EngineInput, V10Engine};
pub use geometry::SliderCrank;
pub use runtime::{Gf509Runtime, Gf509RuntimeConfig, RuntimeTelemetry, ShiftPhase, TorqueSign};
pub use sample_layer::{
    SampleLayerFrame, SampleLayerInput, ThreeZoneSampleLayer, ThreeZoneSampleLayerConfig,
};
pub use scene::{
    AcousticFrame, AcousticScene, AcousticSceneConfig, AirboxPlenum, EngineCover,
    EngineMountMonocoque, MetallicStructure, UnderSeatVibration,
};
pub use thermodynamics::{
    ChamberPhase, CombustionChamber, CombustionFrame, ExhaustRunner, ExhaustValve, GasState,
    PolytropicProcess, RunnerState,
};
