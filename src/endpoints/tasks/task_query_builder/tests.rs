use crate::endpoints::tasks::read_task_file;
use super::*;

#[test]
fn modifying_existing_task_query() {
    let p = Params {
        priority: Some("H".to_string()),
        report: None,
        ..Params::default()
    };
    let mut task_query = TaskQuery::new(p);
    task_query.update(Params {
        report: None,
        status: Some("pending".to_string()),
        ..Params::default()
    });
    assert_eq!(
        &task_query.as_filter_text().join(" "),
        "priority:H status:pending"
    )
}

#[test]
fn with_priority_string_with_status() {
    let p = Params {
        priority: Some("H".to_string()),
        report: None,
        status: Some("pending".to_string()),
        ..Params::default()
    };
    let mut task_query = TaskQuery::new(p);
    assert_eq!(
        &task_query.as_filter_text().join(" "),
        "priority:H status:pending"
    )
}

#[test]
fn with_priority_string_with_no_status() {
    let p = Params {
        priority: Some("H".to_string()),
        report: None,
        ..Params::default()
    };
    let mut task_query = TaskQuery::new(p);
    assert_eq!(
        &task_query.as_filter_text().join(" "),
        "priority:H next"
    )
}

#[test]
fn with_empty_search_param() {
    let p = Params {
        report: None,
        ..Params::default()
    };
    let task_query = TaskQuery::new(p);
    assert_eq!(
        &task_query.as_filter_text().join(" "),
        "next"
    )
}

#[test]
fn when_containing_status() {
    let p = Params {
        report: None,
        status: Some("completed".to_string()),
        ..Params::default()
    };
    let query = TaskQuery::new(p).as_filter_text();
    assert_eq!(
        &query.join(" "),
        "status:completed"
    )
}

#[test]
fn task_by_uuid() {
    let mut p = Params::default();
    let test_uuid = "794618dd-7a41-4aca-ab2e-70cc4a04b5e6".to_string();
    p.filter = Some(test_uuid);
    let t = TaskQuery::new(p);
    println!("{:?}", t);
    println!("{:?}", t.as_filter_text());
    let mut tasks = read_task_file(t, false).unwrap();
    println!("{:#?}", tasks);
}

#[test]
fn adding_new_task() {
    let mut p = Params {
        task_entry: Some("A task is being added".to_string()),
        report: None,
        ..Params::default()
    };
    let t = TaskQuery::new(p.clone());
    let tcmd = t.as_filter_text();
    assert_eq!(
        "add A task is being added",
        tcmd.join(" ")
    );
    p.project = Some("+Tag1".to_string());
    let tcmd = t.as_filter_text();
    assert_eq!(
        "add A task is being added +Tag1",
        tcmd.join(" ")
    );
}