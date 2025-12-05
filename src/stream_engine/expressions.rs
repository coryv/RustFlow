use minijinja::{Environment, Error, ErrorKind, Value};
use minijinja::value::Rest;
use chrono::{Utc, TimeZone, NaiveDate, NaiveDateTime};

pub fn create_environment() -> Environment<'static> {
    let mut env = Environment::new();

    // String Functions
    env.add_function("CONCAT", concat);
    env.add_function("UPPER", upper);
    env.add_function("LOWER", lower);
    env.add_function("TRIM", trim);

    // Math Functions
    env.add_function("ADD", add);
    env.add_function("SUB", sub);
    env.add_function("MUL", mul);
    env.add_function("DIV", div);
    env.add_function("ROUND", round);

    // Date Functions
    env.add_function("NOW", now);
    env.add_function("DATE_ADD", date_add);
    env.add_function("TO_UTC", to_utc);
    env.add_function("PARSE_DATE", parse_date);
    env.add_function("UNIX_TIMESTAMP", unix_timestamp);
    env.add_function("TO_ISO", to_iso);

    env
}

fn concat(args: Rest<Value>) -> String {
    let mut result = String::new();
    for arg in args.0 {
        if let Some(s) = arg.as_str() {
            result.push_str(s);
        } else {
            result.push_str(&arg.to_string());
        }
    }
    result
}

fn upper(s: String) -> String {
    s.to_uppercase()
}

fn lower(s: String) -> String {
    s.to_lowercase()
}

fn trim(s: String) -> String {
    s.trim().to_string()
}

// Math helpers
fn to_f64(v: &Value) -> Result<f64, Error> {
    if let Ok(f) = f64::try_from(v.clone()) {
        Ok(f)
    } else if let Ok(i) = i64::try_from(v.clone()) {
        Ok(i as f64)
    } else if let Some(s) = v.as_str() {
         s.parse::<f64>().map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("Cannot parse number: {}", e)))
    } else {
        Err(Error::new(ErrorKind::InvalidOperation, format!("Expected number, got {:?}", v)))
    }
}

fn add(a: Value, b: Value) -> Result<Value, Error> {
    let num_a = to_f64(&a)?;
    let num_b = to_f64(&b)?;
    Ok(Value::from(num_a + num_b))
}

fn sub(a: Value, b: Value) -> Result<Value, Error> {
    let num_a = to_f64(&a)?;
    let num_b = to_f64(&b)?;
    Ok(Value::from(num_a - num_b))
}

fn mul(a: Value, b: Value) -> Result<Value, Error> {
    let num_a = to_f64(&a)?;
    let num_b = to_f64(&b)?;
    Ok(Value::from(num_a * num_b))
}

fn div(a: Value, b: Value) -> Result<Value, Error> {
    let num_a = to_f64(&a)?;
    let num_b = to_f64(&b)?;
    if num_b == 0.0 {
        return Err(Error::new(ErrorKind::InvalidOperation, "Division by zero"));
    }
    Ok(Value::from(num_a / num_b))
}

fn round(val: Value, precision: Option<i32>) -> Result<Value, Error> {
    let num = to_f64(&val)?;
    let p = precision.unwrap_or(0);
    let factor = 10f64.powi(p);
    Ok(Value::from((num * factor).round() / factor))
}

// Date helpers
fn now() -> String {
    Utc::now().to_rfc3339()
}

fn date_add(ts: String, amount: i64, unit: String) -> Result<String, Error> {
    // Normalize to UTC first using our helper
    let utc_ts = to_utc(ts)?;
    
    let dt = chrono::DateTime::parse_from_rfc3339(&utc_ts)
        .map_err(|e| Error::new(ErrorKind::InvalidOperation, format!("Invalid timestamp for DATE_ADD: {}", e)))?
        .with_timezone(&Utc);
    
    let new_dt = match unit.to_lowercase().as_str() {
        "year" | "years" => dt.checked_add_signed(chrono::Duration::days(amount * 365)).unwrap(),
        "day" | "days" => dt.checked_add_signed(chrono::Duration::days(amount)).unwrap(),
        "hour" | "hours" => dt.checked_add_signed(chrono::Duration::hours(amount)).unwrap(),
        "minute" | "minutes" => dt.checked_add_signed(chrono::Duration::minutes(amount)).unwrap(),
        "second" | "seconds" => dt.checked_add_signed(chrono::Duration::seconds(amount)).unwrap(),
        _ => return Err(Error::new(ErrorKind::InvalidOperation, format!("Unknown unit: {}", unit))),
    };
    
    Ok(new_dt.to_rfc3339())
}

fn to_utc(ts: String) -> Result<String, Error> {
    // Try to parse RFC3339
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&ts) {
        return Ok(dt.with_timezone(&Utc).to_rfc3339());
    }
    // Try YYYY-MM-DD HH:MM:SS (Naive)
    if let Ok(naive) = NaiveDateTime::parse_from_str(&ts, "%Y-%m-%d %H:%M:%S") {
         return Ok(Utc.from_utc_datetime(&naive).to_rfc3339());
    }
    // Try YYYY-MM-DD
    if let Ok(naive_date) = NaiveDate::parse_from_str(&ts, "%Y-%m-%d") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
        return Ok(Utc.from_utc_datetime(&naive_dt).to_rfc3339());
    }
    // Try with 'T' but no Z, e.g. "2023-12-05T12:00:00"
    if let Ok(naive) = NaiveDateTime::parse_from_str(&ts, "%Y-%m-%dT%H:%M:%S") {
          return Ok(Utc.from_utc_datetime(&naive).to_rfc3339());
    }
    
    Err(Error::new(ErrorKind::InvalidOperation, format!("Could not parse date: {}", ts)))
}

fn parse_date(ts: String, fmt: String) -> Result<String, Error> {
    if let Ok(naive) = NaiveDateTime::parse_from_str(&ts, &fmt) {
         return Ok(Utc.from_utc_datetime(&naive).to_rfc3339());
    }
    if let Ok(naive_date) = NaiveDate::parse_from_str(&ts, &fmt) {
         let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
         return Ok(Utc.from_utc_datetime(&naive_dt).to_rfc3339());
    }
    Err(Error::new(ErrorKind::InvalidOperation, format!("Failed to parse '{}' with format '{}'", ts, fmt)))
}

fn unix_timestamp(ts: String) -> Result<i64, Error> {
    // Reuse to_utc logic to normalize first
    let utc_str = to_utc(ts)?;
    let dt = chrono::DateTime::parse_from_rfc3339(&utc_str)
        .map_err(|_| Error::new(ErrorKind::InvalidOperation, "Invalid RFC3339 conversion"))?;
    Ok(dt.timestamp())
}

fn to_iso(ts: String) -> Result<String, Error> {
    to_utc(ts)
}
