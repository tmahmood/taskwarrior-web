/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

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
