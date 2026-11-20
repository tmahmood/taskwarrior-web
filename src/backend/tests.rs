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
use crate::backend::task::get_replica;
use crate::core::app::AppState;
use crate::endpoints::tasks::task_add;
use crate::get_random_appstate;
use chrono::Utc;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use taskchampion::{Operations, Status, Uuid};
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

/// Reproduces the bug in `From<taskchampion::Task> for Task` where the `end`
/// field is populated from the `start` property.
///
/// A pending task that has been started has a `start` value but no `end`
/// value. Conversion to the view model must keep `end` empty.
#[tokio::test]
async fn pending_task_must_not_copy_start_into_end() -> anyhow::Result<()> {
    let (_temp_dir, app_state) = setup_test_cfg();
    let mut replica = get_replica(&app_state.task_storage_path).await?;

    let uuid = Uuid::new_v4();
    let mut ops = Operations::new();
    let mut tc_task = replica.create_task(uuid, &mut ops).await?;
    tc_task.set_description("pending task that was started".to_string(), &mut ops)?;
    tc_task.set_entry(Some(Utc::now()), &mut ops)?;
    tc_task.set_status(Status::Pending, &mut ops)?;
    // Taskwarrior/Timewarrior store timestamps as unix seconds.
    tc_task.set_value("start", Some(Utc::now().timestamp().to_string()), &mut ops)?;
    replica.commit_operations(ops).await?;

    // Re-read from storage so the test exercises the same path as a request.
    let tc_task = replica
        .get_task(uuid)
        .await?
        .expect("task should exist after commit");
    assert!(tc_task.get_value("start").is_some());
    assert!(tc_task.get_value("end").is_none());

    let task: crate::backend::task::Task = tc_task.into();
    assert!(
        task.start.is_some(),
        "the start timestamp should still be parsed"
    );
    assert_eq!(
        task.end, None,
        "a pending task has no end; conversion must not copy `start` into `end`"
    );

    Ok(())
}

/// Positive counterpart to `pending_task_must_not_copy_start_into_end`:
/// a completed task must expose its real `end` value, not its `start` value.
#[tokio::test]
async fn completed_task_must_use_end_property() -> anyhow::Result<()> {
    let (_temp_dir, app_state) = setup_test_cfg();
    let mut replica = get_replica(&app_state.task_storage_path).await?;

    let uuid = Uuid::new_v4();
    let started_at = Utc::now().timestamp() - 3600;
    let mut ops = Operations::new();
    let mut tc_task = replica.create_task(uuid, &mut ops).await?;
    tc_task.set_description("task started an hour ago".to_string(), &mut ops)?;
    tc_task.set_entry(Some(Utc::now()), &mut ops)?;
    tc_task.set_status(Status::Pending, &mut ops)?;
    tc_task.set_value("start", Some(started_at.to_string()), &mut ops)?;
    replica.commit_operations(ops).await?;

    // Completing the task sets a fresh `end` timestamp (different from `start`).
    let mut tc_task = replica
        .get_task(uuid)
        .await?
        .expect("task should exist after commit");
    let mut ops = Operations::new();
    tc_task.set_status(Status::Completed, &mut ops)?;
    replica.commit_operations(ops).await?;

    let tc_task = replica
        .get_task(uuid)
        .await?
        .expect("task should still exist after completion");
    let expected_start = tc_task
        .get_value("start")
        .expect("completed task keeps its start value")
        .to_string();
    let expected_end = tc_task
        .get_value("end")
        .expect("completed task has an end value")
        .to_string();
    assert_ne!(
        expected_start, expected_end,
        "test setup requires distinct start and end timestamps"
    );

    let task: crate::backend::task::Task = tc_task.into();
    assert_eq!(
        task.end,
        Some(expected_end),
        "conversion must read the `end` property, not `start`"
    );
    assert_ne!(
        task.end,
        Some(expected_start),
        "conversion must not expose `start` as `end`"
    );

    Ok(())
}
