fn main() {
    if let Err(error) = story_project::validate_bundled_world_data()
        .and_then(|_| story_project::validate_bundled_writing_fields())
    {
        eprintln!("Bundled world data cannot ship: {error}");
        std::process::exit(1);
    }
    println!("Bundled world schemas, optional writing fields, preset defaults, the demo template and its first-draft setup validated.");
}
