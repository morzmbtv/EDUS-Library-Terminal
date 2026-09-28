//! Shared, local-first EDUS Library domain core.
//!
//! It deliberately has no Tauri, HTTP-server, or desktop-session dependency.
//! The local service invokes these modules without any terminal UI dependency.

pub mod card;
pub mod db;
pub mod device;
pub mod domain;
pub mod face;
pub mod identity;
pub mod runtime;
pub mod security;
pub mod services;
pub mod storage;
pub mod sync;
