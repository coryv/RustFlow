use rust_flow::stream_engine::expressions::create_environment;
use serde_json::json;

#[test]
fn test_string_functions() {
    let env = create_environment();
    
    // CONCAT
    let res = env.render_str("{{ CONCAT('Hello', ' ', 'World') }}", &json!({})).unwrap();
    assert_eq!(res, "Hello World");

    // UPPER
    let res = env.render_str("{{ UPPER('foo') }}", &json!({})).unwrap();
    assert_eq!(res, "FOO");

    // TRIM
    let res = env.render_str("{{ TRIM('  bar  ') }}", &json!({})).unwrap();
    assert_eq!(res, "bar");
}

#[test]
fn test_math_functions() {
    let env = create_environment();
    
    // ADD
    let res = env.render_str("{{ ADD(10, 20) }}", &json!({})).unwrap();
    assert_eq!(res, "30.0"); // Render returns string, treated as float

    // ROUND
    let res = env.render_str("{{ ROUND(10.556, 2) }}", &json!({})).unwrap();
    assert_eq!(res, "10.56");
}

#[test]
fn test_date_functions() {
    let env = create_environment();
    
    // TO_UTC
    let res = env.render_str("{{ TO_UTC('2023-01-01') }}", &json!({})).unwrap();
    assert_eq!(res, "2023-01-01T00:00:00+00:00");

    // DATE_ADD (String in, String out)
    // 2023-01-01 + 1 day
    let res = env.render_str("{{ DATE_ADD('2023-01-01', 1, 'day') }}", &json!({})).unwrap();
    assert_eq!(res, "2023-01-02T00:00:00+00:00");
}
