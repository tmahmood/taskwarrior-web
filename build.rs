use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");
    println!("cargo:rerun-if-changed=templates/");
    if !Command::new("tailwindcss")
        .args(["-i", "frontend/css/style.css", "-o", "dist/style.css", "-c", "frontend/tailwind.config.js"])
        .status().unwrap().success() {
        panic!("Failed to process css")
    }
    if !Command::new("rollup")
        .args(["-c", "frontend/rollup.config.js"])
        .status().unwrap().success() {
        panic!("Failed to process css")
    }
}
