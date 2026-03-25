#![windows_subsystem = "windows"]

mod config;
mod native_interop;
mod theme;
mod time_calc;
mod window;

fn main() {
    window::run();
}
