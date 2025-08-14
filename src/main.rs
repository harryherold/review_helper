#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() -> Result<(), slint::PlatformError> {
    review_helper::main()
}
