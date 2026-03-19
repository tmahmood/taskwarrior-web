/*
 * Copyright 2026 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use std::str::FromStr;

use crate::{NewTask, backend::task::get_replica, endpoints::tasks::run_modify_command};
use chrono::{Datelike, Days, Months, Timelike, Utc};
use taskchampion::{Status, Tag, Uuid};

use super::task_add;

#[tokio::test]
async fn test_task_add() -> anyhow::Result<()> {
    let (tmp_dir, app_state) = crate::get_random_appstate();
    let task_name = Uuid::new_v4();

    let task = NewTask {
        description: task_name.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some("priority:H".into()),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_ok());
    let mut replica = get_replica(&app_state.task_storage_path).await?;
    let tasks = replica.all_tasks().await?;
    let our_task = tasks
        .iter()
        .find(|p| p.1.get_description() == task_name.to_string());
    assert!(our_task.is_some());
    let our_task = our_task.expect("Cannot unwrap task");
    // compare the data.
    let task_map = our_task.1.clone().into_task_data();
    assert_eq!(task_map.get("project"), Some("TWK"));
    let tags: Vec<Tag> = our_task.1.get_tags().collect();
    assert!(tags.contains(&Tag::from_str("twk").unwrap()));
    assert!(tags.contains(&Tag::from_str("development").unwrap()));
    assert_eq!(task_map.get("priority"), Some("H"));

    let _ = tmp_dir.close();
    Ok(())
}

#[tokio::test]
async fn test_task_add_fail() -> anyhow::Result<()> {
    let (tmp_dir, app_state) = crate::get_random_appstate();
    let task_name = Uuid::new_v4();

    let task = NewTask {
        description: task_name.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some("priority:H due:\"".into()),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_err());
    let result = result.unwrap_err();
    assert!(!result.is_success());
    assert!(result.has_error("additional"));

    let _ = tmp_dir.close();
    Ok(())
}

#[tokio::test]
async fn test_task_modify_successful() -> anyhow::Result<()> {
    let (tmp_dir, app_state) = crate::get_random_appstate();
    let task_name = Uuid::new_v4();

    let task = NewTask {
        description: task_name.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some("priority:H".into()),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_ok());
    let mut replica = get_replica(&app_state.task_storage_path).await?;
    let tasks = replica.all_tasks().await?;
    let our_task_1 = tasks
        .iter()
        .find(|p| p.1.get_description() == task_name.to_string());
    assert!(our_task_1.is_some());
    let our_task_1 = our_task_1.expect("Cannot unwrap task");

    // create a second one.
    let task_name_2 = Uuid::new_v4();

    let task = NewTask {
        description: task_name_2.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some("priority:H".into()),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_ok());
    let tasks = replica.all_tasks().await.expect("Cannot retrieve tasks");
    let our_task_2 = tasks
        .iter()
        .find(|p| p.1.get_description() == task_name_2.to_string());
    assert!(our_task_2.is_some());

    let dt_wait = Utc::now().checked_add_days(Days::new(15)).unwrap();
    let dt_wait_str = dt_wait.format("%Y-%m-%d").to_string();
    let dt_wait = dt_wait
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let dt_due = Utc::now().checked_add_days(Days::new(60)).unwrap();
    let dt_due_str = dt_due.format("%Y-%m-%d").to_string();
    let dt_due = dt_due
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();

    // Now modify!
    let cmd_text = format!(
        "wait:{} due:{} +concert -twk \"description:This is a title with spaces\" depends:{} project:{}  status:completed",
        dt_wait_str, dt_due_str, task_name_2, "KWT"
    );
    let result = run_modify_command(*our_task_1.0, &cmd_text, &app_state).await;
    assert!(result.is_ok());
    let updated_task = replica.get_task(*our_task_1.0).await;
    assert!(updated_task.is_ok());
    let updated_task = updated_task.unwrap();
    assert!(updated_task.is_some());
    let updated_task = updated_task.unwrap();

    // Compare the data.
    assert_eq!(
        updated_task.get_description(),
        "This is a title with spaces"
    );
    assert_eq!(updated_task.get_value("project"), Some("KWT"));
    assert_eq!(updated_task.get_due(), Some(dt_due));
    assert_eq!(updated_task.get_wait(), Some(dt_wait));
    assert!(updated_task.is_waiting());
    assert_eq!(updated_task.get_dependencies().count(), 1);
    assert_eq!(updated_task.get_status(), Status::Completed);
    assert_eq!(updated_task.get_dependencies().next(), Some(task_name_2));
    let set_tags = [
        "concert",
        "development",
        "WAITING",
        "COMPLETED",
        "UNBLOCKED",
    ];
    for tag in updated_task.get_tags() {
        let tag_name = tag.to_string();
        assert!(set_tags.contains(&tag_name.as_str()));
    }

    let _ = tmp_dir.close();
    Ok(())
}

#[tokio::test]
async fn test_task_modify_breakit() -> anyhow::Result<()> {
    let (tmp_dir, app_state) = crate::get_random_appstate();
    let task_name = Uuid::new_v4();

    let task = NewTask {
        description: task_name.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some("priority:H".into()),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_ok());
    let mut replica = get_replica(&app_state.task_storage_path).await?;
    let tasks = replica.all_tasks().await.expect("Cannot retrieve tasks");
    let our_task_1 = tasks
        .iter()
        .find(|p| p.1.get_description() == task_name.to_string());
    assert!(our_task_1.is_some());
    let our_task_1 = our_task_1.expect("Cannot unwrap task");

    // Now modify!
    let cmd_text = format!(
        "wait:{} due:{} +concert -twk +dont/c -e/b +WAITING -PENDING start \"description:This is a title with spaces\" depends:{} project:{}",
        "abc",
        "def",
        String::from("ec2c596f-5fa3-442c-80ee-98b087e32bbd"),
        ""
    );
    let result = run_modify_command(*our_task_1.0, &cmd_text, &app_state).await;

    let result = result.unwrap_err();

    assert!(!result.is_success());
    assert!(result.has_error("additional"));
    let add_errors = result.fields.get("additional");
    assert!(add_errors.is_some());
    let add_errors = add_errors.unwrap();

    assert_eq!(add_errors.len(), 5);

    let mut keywords = vec!["wait", "due", "start", "Synthetic", "Synthetic"];
    for err in add_errors {
        assert_eq!(&err.field, "additional");
        let p = keywords.iter().position(|p| err.message.contains(*p));
        assert!(p.is_some());
        let p = p.unwrap();
        let _ = keywords.remove(p);
    }
    assert!(keywords.is_empty());

    let _ = tmp_dir.close();
    Ok(())
}

#[tokio::test]
async fn test_task_add_recur() -> anyhow::Result<()> {
    let (tmp_dir, app_state) = crate::get_random_appstate();
    let task_name = Uuid::new_v4();
    let dt_wait = Utc::now()
        .with_day(1)
        .unwrap()
        .checked_add_months(Months::new(1))
        .unwrap();

    let task = NewTask {
        description: task_name.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some(format!(
            "priority:H recur:monthly due:{}",
            dt_wait.format("%Y-%m-%d")
        )),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_ok());
    let mut replica = get_replica(&app_state.task_storage_path).await?;
    let tasks = replica.all_tasks().await?;
    let our_task = tasks
        .iter()
        .find(|p| p.1.get_description() == task_name.to_string());
    assert!(our_task.is_some());
    let our_task = our_task.expect("Cannot unwrap task");
    // compare the data.
    assert_eq!(our_task.1.get_status(), Status::Recurring);
    assert_eq!(our_task.1.get_value("rtype"), Some("periodic"));
    assert_eq!(our_task.1.get_value("recur"), Some("monthly"));

    let _ = tmp_dir.close();
    Ok(())
}

#[tokio::test]
async fn test_task_append_dependency() -> anyhow::Result<()> {
    let (tmp_dir, app_state) = crate::get_random_appstate();
    let task_name_prime = Uuid::new_v4();

    let task = NewTask {
        description: task_name_prime.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some("priority:H".into()),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_ok());
    let prime_task_uuid = result.unwrap();


    let mut replica = get_replica(&app_state.task_storage_path).await?;
    let tasks = replica.all_tasks().await?;
    let our_task = tasks
        .iter()
        .find(|p| p.1.get_description() == task_name_prime.to_string())
        .unwrap();

    assert_eq!(our_task.0, &prime_task_uuid);

    let task_name = Uuid::new_v4();
    let task = NewTask {
        description: task_name.clone().to_string(),
        tags: Some("+twk development".into()),
        project: Some("TWK".into()),
        filter_value: None,
        additional: Some(format!("depends:{}", our_task.0)),
    };
    let result = task_add(&task, &app_state).await;
    assert!(result.is_ok());
    let dependent_task_uuid = result.unwrap();

    let tasks = replica.all_tasks().await?;
    let dependent_task = tasks.get(&dependent_task_uuid).unwrap();
    assert_eq!(dependent_task.get_dependencies().count(), 1);
    let _ = tmp_dir.close();
    Ok(())
}
