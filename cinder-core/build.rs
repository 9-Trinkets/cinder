use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let project_dir = Path::new(&manifest_dir)
        .parent()
        .expect("cinder-core should be one level below project root");
    println!("cargo::rustc-env=CINDER_PROJECT_DIR={}", project_dir.display());
}
