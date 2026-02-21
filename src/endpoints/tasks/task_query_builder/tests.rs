/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use super::*;

#[test]
fn modifying_existing_task_query() {
    let p = TWGlobalState {
        query: Some("priority:H".to_string()),
        report: None,
        ..TWGlobalState::default()
    };
    let mut task_query = TaskQuery::new(p);
    task_query.update(TWGlobalState {
        report: None,
        status: Some("pending".to_string()),
        ..TWGlobalState::default()
    });
    assert_eq!(
        &task_query.as_filter_text().join(" "),
        "priority:H status:pending"
    )
}

#[test]
fn with_priority_string_with_status() {
    let p = TWGlobalState {
        query: Some("priority:H".to_string()),
        report: None,
        status: Some("pending".to_string()),
        ..TWGlobalState::default()
    };
    let task_query = TaskQuery::new(p);
    assert_eq!(
        &task_query.as_filter_text().join(" "),
        "priority:H status:pending"
    )
}

#[test]
fn with_priority_string_with_no_status() {
    let p = TWGlobalState {
        query: Some("priority:H".to_string()),
        report: None,
        ..TWGlobalState::default()
    };
    let task_query = TaskQuery::new(p);
    assert_eq!(&task_query.as_filter_text().join(" "), "next priority:H")
}

#[test]
fn with_empty_search_param() {
    let p = TWGlobalState {
        report: None,
        ..TWGlobalState::default()
    };
    let task_query = TaskQuery::new(p);
    assert_eq!(&task_query.as_filter_text().join(" "), "next")
}

#[test]
fn when_containing_status() {
    let p = TWGlobalState {
        report: None,
        status: Some("completed".to_string()),
        ..TWGlobalState::default()
    };
    let query = TaskQuery::new(p).as_filter_text();
    assert_eq!(&query.join(" "), "status:completed")
}

#[test]
fn error_condition_for_pending_tasks() {
    let query_text = r#"{
  "status": "NotSet",
  "priority": "NotSet",
  "report": "Next",
  "tags": [],
  "project": null,
  "filter": null,
  "new_entry": null,
  "custom_query": null
}"#;
    let task_query = serde_json::from_str::<TaskQuery>(query_text).unwrap();
    let result = task_query.get_query(true);
    assert_eq!(result, ["next", "export"])
}
