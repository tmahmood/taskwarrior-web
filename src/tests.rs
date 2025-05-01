use tera::Function;

use super::*;

#[test]
fn test_get_timer() {
    let get_timer_func = get_timer();
    let time_seconds: HashMap<String, tera::Value> = HashMap::from([
        ("date".to_string(), serde_json::to_value("20250501T035408Z").unwrap())
    ]);

    let result = get_timer_func.call(&time_seconds);
    assert_eq!(result.is_ok(), true);
    let result_value = result.unwrap(); 
    assert_eq!(result_value, serde_json::to_value("00:00:52").unwrap());

    let time_seconds: HashMap<String, tera::Value> = HashMap::from([
        ("date".to_string(), serde_json::to_value("20250501T033408Z").unwrap())
    ]);

    let result = get_timer_func.call(&time_seconds);
    assert_eq!(result.is_ok(), true);
    let result_value = result.unwrap(); 
    assert_eq!(result_value, serde_json::to_value("00:20:52").unwrap());
    
    let time_seconds: HashMap<String, tera::Value> = HashMap::from([
        ("date".to_string(), serde_json::to_value("20250501T023408Z").unwrap())
    ]);

    let result = get_timer_func.call(&time_seconds);
    assert_eq!(result.is_ok(), true);
    let result_value = result.unwrap(); 
    assert_eq!(result_value, serde_json::to_value("01:20:00").unwrap());
}

#[test]
fn test_get_datetime_iso() {
    let get_datetime_iso_func = get_datetime_iso();
    let datetime: HashMap<String, tera::Value> = HashMap::from([
        ("datetime".to_string(), serde_json::to_value("20250501T035408Z").unwrap())
    ]);
    let result = get_datetime_iso_func.call(&datetime);
    assert_eq!(result.is_ok(), true);
    let result_value = result.unwrap(); 
    assert_eq!(result_value, serde_json::to_value("2025-05-01T03:54:08+00:00").unwrap());

    let datetime: HashMap<String, tera::Value> = HashMap::from([
        ("datetime".to_string(), serde_json::to_value("2025-05-01T03:54:08Z").unwrap())
    ]);
    let result = get_datetime_iso_func.call(&datetime);
    assert_eq!(result.is_ok(), true);
    let result_value = result.unwrap(); 
    assert_eq!(result_value, serde_json::to_value("2025-05-01T03:54:08+00:00").unwrap());
}
