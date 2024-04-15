use super::*;

#[test]
fn modifying_existing_task_query() {
    let p = Params {
        query: None,
        priority: Some("H".to_string()),
        q: None,
        f: None,
        report: None,
        status: None,
        uuid: None,
    };
    let mut task_query = TaskQuery::new(p);
    task_query.update(Params {
        query: None,
        priority: None,
        q: None,
        f: None,
        report: None,
        status: Some("pending".to_string()),
        uuid: None,
    });
    assert_eq!(
        &task_query.build().join(" "),
        "task priority:H status:pending export"
    )
}

#[test]
fn with_priority_string_with_status() {
    let p = Params {
        query: None,
        priority: Some("H".to_string()),
        q: None,
        f: None,
        report: None,
        status: Some("pending".to_string()),
        uuid: None,
    };
    let mut task_query = TaskQuery::new(p);
    assert_eq!(
        &task_query.build().join(" "),
        "task priority:H status:pending export"
    )
}

#[test]
fn with_priority_string_with_no_status() {
    let p = Params {
        query: None,
        priority: Some("H".to_string()),
        q: None,
        f: None,
        report: None,
        status: None,
        uuid: None,
    };
    let mut task_query = TaskQuery::new(p);
    assert_eq!(
        &task_query.build().join(" "),
        "task priority:H export next"
    )
}

#[test]
fn with_empty_search_param() {
    let p = Params {
        query: None,
        priority: None,
        q: None,
        f: None,
        report: None,
        status: None,
        uuid: None,
    };
    let task_query = TaskQuery::new(p);
    assert_eq!(
        &task_query.build().join(" "),
        "task export next"
    )
}

#[test]
fn when_containing_status() {
    let p = Params {
        query: None,
        priority: None,
        q: None,
        f: None,
        report: None,
        status: Some("completed".to_string()),
        uuid: None,
    };
    let query = TaskQuery::new(p).build();
    assert_eq!(
        &query.join(" "),
        "task status:completed export"
    )
}
