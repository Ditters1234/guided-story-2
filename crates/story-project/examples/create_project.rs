use std::path::Path;
use story_project::{Project, World};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .expect("Usage: create_project <new-folder> [title]");
    let project = Project::create(
        Path::new(path),
        args.get(2)
            .cloned()
            .unwrap_or("Fictional test world".into()),
        World::default(),
        uuid::Uuid::new_v4().to_string(),
    )
    .expect("Could not create isolated project");
    println!("Created {}", project.folder_path());
}
