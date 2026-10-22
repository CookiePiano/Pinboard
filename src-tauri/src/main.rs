// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
mod linux_startup;

fn main() {
    #[cfg(target_os = "linux")]
    linux_startup::configure();
    pinboard_lib::run()
}
