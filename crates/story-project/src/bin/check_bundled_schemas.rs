fn main() {
    if let Err(error) = story_project::validate_bundled_world_data()
        .and_then(|_| story_project::validate_bundled_library_examples())
    {
        eprintln!("Bundled world data cannot ship: {error}");
        std::process::exit(1);
    }
    println!("Bundled world schemas, preset defaults, location templates and node-library examples validated.");
}
