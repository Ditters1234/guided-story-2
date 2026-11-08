fn main() {
    if let Err(error) = story_project::validate_bundled_world_data() {
        eprintln!("Bundled world data cannot ship: {error}");
        std::process::exit(1);
    }
    println!("Bundled world schemas, preset defaults and location templates validated.");
}
