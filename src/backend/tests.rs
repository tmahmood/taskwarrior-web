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
use crate::backend::task::{Task, get_replica};
use crate::core::app::AppState;
use crate::endpoints::tasks::task_add;
use crate::get_random_appstate;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use taskchampion::{Operations, Uuid};
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

#[test]
fn test_task_json_start_and_end() {
    // 1. Task JSON with only `start` timestamp (no `end`)
    let json_only_start = r#"{
        "id": 1,
        "uuid": "00000000-0000-0000-0000-000000000001",
        "description": "Task with only start",
        "start": "20261120T120000Z"
    }"#;
    let task1: Task = serde_json::from_str(json_only_start).expect("Failed to deserialize task JSON");
    assert!(task1.start.is_some());
    assert!(task1.end.is_none());

    // 2. Task JSON with both `start` and `end` timestamps
    let json_start_and_end = r#"{
        "id": 2,
        "uuid": "00000000-0000-0000-0000-000000000002",
        "description": "Task with start and end",
        "start": "20261120T120000Z",
        "end": "20261120T130000Z"
    }"#;
    let task2: Task = serde_json::from_str(json_start_and_end).expect("Failed to deserialize task JSON");
    assert!(task2.start.is_some());
    assert_eq!(task2.end.as_deref(), Some("20261120T130000Z"));

    // Also verify for crate::endpoints::tasks::Task model
    let ep_task1: crate::endpoints::tasks::Task =
        serde_json::from_str(json_only_start).expect("Failed to deserialize endpoint task JSON");
    assert_eq!(ep_task1.start.as_deref(), Some("20261120T120000Z"));
    assert!(ep_task1.end.is_none());

    let ep_task2: crate::endpoints::tasks::Task =
        serde_json::from_str(json_start_and_end).expect("Failed to deserialize endpoint task JSON");
    assert_eq!(ep_task2.start.as_deref(), Some("20261120T120000Z"));
    assert_eq!(ep_task2.end.as_deref(), Some("20261120T130000Z"));
}

#[tokio::test]
async fn test_task_from_taskchampion_start_and_end() -> anyhow::Result<()> {
    let (_temp_dir, app_state) = setup_test_cfg();
    let mut replica = get_replica(&app_state.task_storage_path).await?;
    let mut ops = Operations::new();

    // 1. TaskChampion task with only `start` property set
    let uuid_only_start = Uuid::new_v4();
    let mut tc_task1 = replica.create_task(uuid_only_start, &mut ops).await?;
    tc_task1.set_description("tc task with only start".into(), &mut ops)?;
    tc_task1.set_value("start", Some("1700000000".into()), &mut ops)?;

    let task1 = Task::from(tc_task1);
    assert!(task1.start.is_some());
    assert!(task1.end.is_none());

    // 2. TaskChampion task with both `start` and `end` properties set
    let uuid_start_end = Uuid::new_v4();
    let mut tc_task2 = replica.create_task(uuid_start_end, &mut ops).await?;
    tc_task2.set_description("tc task with start and end".into(), &mut ops)?;
    tc_task2.set_value("start", Some("1700000000".into()), &mut ops)?;
    tc_task2.set_value("end", Some("1700003600".into()), &mut ops)?;

    let task2 = Task::from(tc_task2);
    assert!(task2.start.is_some());
    assert_eq!(task2.end.as_deref(), Some("1700003600"));

    Ok(())
}
