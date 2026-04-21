#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

extern crate log;
extern crate simplelog;

fn main() {
    review_helper::main();
}
