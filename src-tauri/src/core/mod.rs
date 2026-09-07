pub mod autostart;
pub mod backup;
pub mod handle;
pub mod hotkey;
pub mod logger;
#[cfg(target_os = "macos")]
mod macos_sysproxy;
pub mod manager;
mod notification;
pub mod service;
pub mod sysopt;
pub mod timer;
pub mod tray;
pub mod updater;
pub mod validate;
pub mod win_uwp;

pub use self::{manager::CoreManager, timer::Timer, updater::SilentUpdater};
