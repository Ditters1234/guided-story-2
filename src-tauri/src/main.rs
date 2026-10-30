#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|argument| argument == "--export-bindings") {
        guided_story_2_lib::export_bindings();
        return;
    }
    guided_story_2_lib::run();
}
