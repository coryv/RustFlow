use serde_json::Value;

pub fn loose_eq(a: &Value, b: &Value) -> bool {
    // Handle "123" == 123
    if let (Some(a_num), Some(b_num)) = (to_f64(a), to_f64(b)) {
        return (a_num - b_num).abs() < f64::EPSILON;
    }
    // Handle "true" == true
    if let (Value::String(s), Value::Bool(b_val)) = (a, b) {
        return s.parse::<bool>().unwrap_or(false) == *b_val;
    }
    if let (Value::Bool(a_val), Value::String(s)) = (a, b) {
        return *a_val == s.parse::<bool>().unwrap_or(false);
    }
    
    // Default equality
    a == b
}

pub fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}
