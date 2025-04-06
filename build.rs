use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");
    println!("Install npm dependencies");
    if !Command::new("npm")
        .current_dir("frontend")
        .args(["install"])
        .status()
        .unwrap()
        .success()
    {
        panic!("Could not install dependencies")
    }

    if !Command::new("frontend/node_modules/.bin/tailwindcss")
        .args(["-i", "frontend/css/style.css", "-o", "dist/style.css"])
        .status()
        .unwrap()
        .success()
    {
        panic!("Failed to process css")
    }

    if !Command::new("frontend/node_modules/.bin/rollup")
        .args(["-c", "frontend/rollup.config.js"])
        .status()
        .unwrap()
        .success()
    {
        panic!("Failed to process js")
    }
    // adding templates
    if !Command::new("cp")
        .args(["-r", "frontend/templates", "dist"])
        .status()
        .unwrap()
        .success()
    {
        println!("cargo::warning=Failed to copy files");
        panic!("Failed to copy template files")
    }
}
