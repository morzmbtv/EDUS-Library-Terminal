//! Native Configurator transport. The Win32 executable is separate so protocol
//! tests run without elevation; production UAC requirements are never disabled.
pub mod ipc;
mod server_identity;
