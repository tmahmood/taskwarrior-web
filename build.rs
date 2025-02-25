use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");
    match Command::new("tailwindcss")
        .args(["-i", "frontend/css/style.css", "-o", "dist/style.css"])
        .status() {
        Ok(vv) => {
            vv.success()
        }
        Err(_e) => {
            println!("cargo::warning=Failed to process css");
            panic!("Failed to process css")
        }
    };
    if !Command::new("rollup")
        .args(["-c", "frontend/rollup.config.js"])
        .status().unwrap().success() {
        panic!("Failed to process css")
    }
    // adding templates
    if !Command::new("cp")
        .args(["-r", "frontend/templates", "dist/"])
        .status().unwrap().success() {
        println!("cargo::warning=Failed to copy files");
        panic!("Failed to copy template files")
    }
}
