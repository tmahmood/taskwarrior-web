use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=public/css/style.css");
    println!("cargo:rerun-if-changed=templates/*");
    Command::new("tailwindcss")
        .args(
            [
                "-i",
                "public/css/style.css",
                "-o",
                "dist/style.css"
            ]
        ).status().expect("Failed to compile css");
}
