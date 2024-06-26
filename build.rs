use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");
    let css_build_success = match Command::new("tailwindcss")
        .args(["-i", "frontend/css/style.css", "-o", "dist/style.css", "-c", "frontend/tailwind.config.js"])
        .status() {
        Ok(vv) => {
            vv.success()
        }
        Err(e) => {
            false
        }
    };
    if !css_build_success {
        if let Err(e) = Command::new("./tailwindcss")
            .args(["-i", "frontend/css/style.css", "-o", "dist/style.css", "-c", "frontend/tailwind.config.js"])
            .status() {
            panic!("Failed to process css")
        }
    };
    if !Command::new("rollup")
        .args(["-c", "frontend/rollup.config.js"])
        .status().unwrap().success() {
        panic!("Failed to process css")
    }
}
