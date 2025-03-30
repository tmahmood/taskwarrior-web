use std::{fs, process::Command};

fn main() {
    let tailwindcss_bin = fs::canonicalize("./").expect("Cannot determine absolute path to current directory");
    let tailwindcss_bin = tailwindcss_bin.join("tailwindcss");
    if !tailwindcss_bin.exists() {
        panic!("Cannot find tailwindcss binary in {}", tailwindcss_bin.to_string_lossy())
    }

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

    if !Command::new(tailwindcss_bin)
        .args(["-i", "frontend/css/style.css", "-o", "dist/style.css"])
        .status()
        .unwrap()
        .success()
    {
        panic!("Failed to process css")
    }

    if !Command::new("frontend/node_modules/rollup/dist/bin/rollup")
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
