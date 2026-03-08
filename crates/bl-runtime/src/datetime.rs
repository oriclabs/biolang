use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};

use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Utc, Weekday};

pub fn datetime_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("now", Arity::Exact(0)),
        ("timestamp", Arity::Exact(0)),
        ("timestamp_ms", Arity::Exact(0)),
        ("date_format", Arity::Exact(2)),
        ("date_parse", Arity::Exact(2)),
        ("date_add", Arity::Exact(3)),
        ("date_diff", Arity::Exact(3)),
        ("year", Arity::Exact(1)),
        ("month", Arity::Exact(1)),
        ("day", Arity::Exact(1)),
        ("weekday", Arity::Exact(1)),
    ]
}

pub fn is_datetime_builtin(name: &str) -> bool {
    matches!(
        name,
        "now" | "timestamp"
            | "timestamp_ms"
            | "date_format"
            | "date_parse"
            | "date_add"
            | "date_diff"
            | "year"
            | "month"
            | "day"
            | "weekday"
    )
}

pub fn call_datetime_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "now" => {
            let ts = Utc::now().to_rfc3339();
            Ok(Value::Str(ts))
        }
        "timestamp" => {
            let secs = Utc::now().timestamp();
            Ok(Value::Int(secs))
        }
        "timestamp_ms" => {
            let ms = Utc::now().timestamp_millis();
            Ok(Value::Int(ms))
        }
        "date_format" => {
            let date_str = require_str(&args[0], "date_format")?;
            let fmt = require_str(&args[1], "date_format")?;
            let dt = parse_datetime(date_str, "date_format")?;
            Ok(Value::Str(dt.format(fmt).to_string()))
        }
        "date_parse" => {
            let s = require_str(&args[0], "date_parse")?;
            let fmt = require_str(&args[1], "date_parse")?;
            let ndt = NaiveDateTime::parse_from_str(s, fmt).map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("date_parse() failed: {e}"),
                    None,
                )
            })?;
            let dt: DateTime<Utc> = DateTime::from_naive_utc_and_offset(ndt, Utc);
            Ok(Value::Str(dt.to_rfc3339()))
        }
        "date_add" => {
            let date_str = require_str(&args[0], "date_add")?;
            let amount = require_int(&args[1], "date_add")?;
            let unit = require_str(&args[2], "date_add")?;
            let dt = parse_datetime(date_str, "date_add")?;
            let duration = match unit {
                "days" => Duration::days(amount),
                "hours" => Duration::hours(amount),
                "minutes" => Duration::minutes(amount),
                "seconds" => Duration::seconds(amount),
                other => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "date_add() unknown unit '{other}', expected days/hours/minutes/seconds"
                        ),
                        None,
                    ))
                }
            };
            let result = dt + duration;
            Ok(Value::Str(result.to_rfc3339()))
        }
        "date_diff" => {
            let d1 = require_str(&args[0], "date_diff")?;
            let d2 = require_str(&args[1], "date_diff")?;
            let unit = require_str(&args[2], "date_diff")?;
            let dt1 = parse_datetime(d1, "date_diff")?;
            let dt2 = parse_datetime(d2, "date_diff")?;
            let diff = dt1 - dt2;
            let result = match unit {
                "days" => diff.num_days(),
                "hours" => diff.num_hours(),
                "minutes" => diff.num_minutes(),
                "seconds" => diff.num_seconds(),
                other => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "date_diff() unknown unit '{other}', expected days/hours/minutes/seconds"
                        ),
                        None,
                    ))
                }
            };
            Ok(Value::Int(result))
        }
        "year" => {
            let s = require_str(&args[0], "year")?;
            let dt = parse_datetime(s, "year")?;
            Ok(Value::Int(dt.year() as i64))
        }
        "month" => {
            let s = require_str(&args[0], "month")?;
            let dt = parse_datetime(s, "month")?;
            Ok(Value::Int(dt.month() as i64))
        }
        "day" => {
            let s = require_str(&args[0], "day")?;
            let dt = parse_datetime(s, "day")?;
            Ok(Value::Int(dt.day() as i64))
        }
        "weekday" => {
            let s = require_str(&args[0], "weekday")?;
            let dt = parse_datetime(s, "weekday")?;
            let name = match dt.weekday() {
                Weekday::Mon => "Monday",
                Weekday::Tue => "Tuesday",
                Weekday::Wed => "Wednesday",
                Weekday::Thu => "Thursday",
                Weekday::Fri => "Friday",
                Weekday::Sat => "Saturday",
                Weekday::Sun => "Sunday",
            };
            Ok(Value::Str(name.to_string()))
        }
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("unknown datetime builtin: {name}"),
            None,
        )),
    }
}

fn parse_datetime(s: &str, func: &str) -> Result<DateTime<Utc>> {
    // Try RFC 3339 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try common naive format
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::from_naive_utc_and_offset(ndt, Utc));
    }
    // Try date-only
    if let Ok(nd) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let ndt = nd.and_hms_opt(0, 0, 0).unwrap();
        return Ok(DateTime::from_naive_utc_and_offset(ndt, Utc));
    }
    Err(BioLangError::runtime(
        ErrorKind::TypeError,
        format!("{func}() cannot parse date '{s}' — expected ISO 8601 / RFC 3339 / YYYY-MM-DD"),
        None,
    ))
}

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Str, got {}", other.type_of()),
            None,
        )),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Int, got {}", other.type_of()),
            None,
        )),
    }
}
