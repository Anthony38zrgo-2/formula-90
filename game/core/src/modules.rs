//! Registered modules of the facade (module crate aggregator).
//!
//! Each future subsystem (climatology, AI, ...) is a separate crate whose
//! `SimModule` impl is registered here and enabled through `CoreConfig.modules`.

pub mod ai_stub;
pub mod weather_stub;
