/*
 * Copyright 2026 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
use crate::NewTask;
use crate::core::app::AppState;
use crate::endpoints::tasks::task_add;
use crate::get_random_appstate;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use taskchampion::Uuid;
use tempfile::TempDir;

/// sets up a taskwarrior configuration file for testing.
fn setup_test_cfg() -> (TempDir, AppState) {
    let (temp_dir, app_state) = get_random_appstate();
    (temp_dir, app_state)
}

#[tokio::test]
async fn check_hook_file_execution() -> anyhow::Result<()> {
    let (_temp_dir, app_state) = setup_test_cfg();
    let hooks_dir = app_state.task_hooks_path.as_ref().unwrap();
    let hook_file = hooks_dir.join("on-add.task");
    let gen_file = hooks_dir.join("on_add_hook_called");
    let content = format!("#!/bin/bash\ntouch \"{}\"", gen_file.display());
    fs::create_dir_all(hooks_dir)?;
    {
        let mut file = File::create(&hook_file)?;
        file.write_all(content.as_bytes())?;
        let mut perms = fs::metadata(&hook_file)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_file, perms)?;
    }
    let mut file = File::open(&hook_file)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    assert_eq!(s, content);

    let task_name = Uuid::new_v4();
    let task = NewTask {
        description: task_name.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some("priority:H".into()),
    };
    task_add(&task, &app_state).await?;
    assert!(gen_file.exists());
    Ok(())
}
