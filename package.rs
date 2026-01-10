extern crate built;

use std::path::Path;

fn main() {
    let template_dir = Path::new("templates");
    if !template_dir.exists() {
        panic!("Template directory 'templates/' not found!");
    }

    println!("cargo:rerun-if-changed=templates");

    built::write_built_file().expect("Failed to acquire build-time information");
}
