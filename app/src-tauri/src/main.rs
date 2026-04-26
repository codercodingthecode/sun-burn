// Prevents console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sun_burn_app_lib::run();
}
