use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::span::Span;
use bl_core::value::Value;
use std::collections::HashMap;

pub fn add(a: Value, b: Value, span: Option<Span>) -> Result<Value> {
    match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x.wrapping_add(*y))),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 + y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x + *y as f64)),
        (Value::Str(x), Value::Str(y)) => Ok(Value::Str(format!("{x}{y}"))),
        (Value::List(x), Value::List(y)) => {
            let mut result = x.clone();
            result.extend(y.clone());
            Ok(Value::List(result))
        }
        _ => Err(BioLangError::type_error(
            format!("cannot add {} and {}", a.type_of(), b.type_of()),
            span,
        )),
    }
}

pub fn sub(a: Value, b: Value, span: Option<Span>) -> Result<Value> {
    match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x.wrapping_sub(*y))),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x - y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 - y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x - *y as f64)),
        _ => Err(BioLangError::type_error(
            format!("cannot subtract {} from {}", b.type_of(), a.type_of()),
            span,
        )),
    }
}

pub fn mul(a: Value, b: Value, span: Option<Span>) -> Result<Value> {
    match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x.wrapping_mul(*y))),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x * y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 * y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x * *y as f64)),
        (Value::Str(s), Value::Int(n)) | (Value::Int(n), Value::Str(s)) => {
            Ok(Value::Str(s.repeat(*n as usize)))
        }
        _ => Err(BioLangError::type_error(
            format!("cannot multiply {} and {}", a.type_of(), b.type_of()),
            span,
        )),
    }
}

pub fn div(a: Value, b: Value, span: Option<Span>) -> Result<Value> {
    match (&a, &b) {
        (Value::Int(_), Value::Int(0)) | (Value::Float(_), Value::Int(0)) => {
            Err(BioLangError::runtime(
                ErrorKind::DivisionByZero,
                "division by zero",
                span,
            ))
        }
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x / y)),
        (Value::Float(x), Value::Float(y)) => {
            if *y == 0.0 {
                Err(BioLangError::runtime(
                    ErrorKind::DivisionByZero,
                    "division by zero",
                    span,
                ))
            } else {
                Ok(Value::Float(x / y))
            }
        }
        (Value::Int(x), Value::Float(y)) => {
            if *y == 0.0 {
                Err(BioLangError::runtime(
                    ErrorKind::DivisionByZero,
                    "division by zero",
                    span,
                ))
            } else {
                Ok(Value::Float(*x as f64 / y))
            }
        }
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x / *y as f64)),
        _ => Err(BioLangError::type_error(
            format!("cannot divide {} by {}", a.type_of(), b.type_of()),
            span,
        )),
    }
}

pub fn modulo(a: Value, b: Value, span: Option<Span>) -> Result<Value> {
    match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => {
            if *y == 0 {
                Err(BioLangError::runtime(
                    ErrorKind::DivisionByZero,
                    "modulo by zero",
                    span,
                ))
            } else {
                Ok(Value::Int(x % y))
            }
        }
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x % y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 % y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x % *y as f64)),
        _ => Err(BioLangError::type_error(
            format!("cannot modulo {} by {}", a.type_of(), b.type_of()),
            span,
        )),
    }
}

pub fn negate(a: Value, span: Option<Span>) -> Result<Value> {
    match a {
        Value::Int(n) => Ok(Value::Int(-n)),
        Value::Float(f) => Ok(Value::Float(-f)),
        _ => Err(BioLangError::type_error(
            format!("cannot negate {}", a.type_of()),
            span,
        )),
    }
}

pub fn less(a: &Value, b: &Value) -> Result<bool> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(x < y),
        (Value::Float(x), Value::Float(y)) => Ok(x < y),
        (Value::Int(x), Value::Float(y)) => Ok((*x as f64) < *y),
        (Value::Float(x), Value::Int(y)) => Ok(*x < (*y as f64)),
        (Value::Str(x), Value::Str(y)) => Ok(x < y),
        _ => Ok(false),
    }
}

pub fn greater(a: &Value, b: &Value) -> Result<bool> {
    less(b, a)
}

pub fn less_equal(a: &Value, b: &Value) -> Result<bool> {
    Ok(!greater(a, b)?)
}

pub fn greater_equal(a: &Value, b: &Value) -> Result<bool> {
    Ok(!less(a, b)?)
}

pub fn get_field(object: &Value, field: &str, span: Option<Span>) -> Result<Value> {
    match object {
        Value::Record(map) | Value::Map(map) => {
            map.get(field).cloned().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("no field '{field}' on record"),
                    span,
                )
            })
        }
        Value::Table(t) => {
            if let Some(col_idx) = t.col_index(field) {
                let values: Vec<Value> = t.rows.iter().map(|r| {
                    r.get(col_idx).cloned().unwrap_or(Value::Nil)
                }).collect();
                Ok(Value::List(values))
            } else {
                Err(BioLangError::runtime(
                    ErrorKind::NameError,
                    format!("no column '{field}' on table"),
                    span,
                ))
            }
        }
        _ => Err(BioLangError::type_error(
            format!("cannot access field on {}", object.type_of()),
            span,
        )),
    }
}

pub fn get_field_opt(object: &Value, field: &str) -> Value {
    match object {
        Value::Record(map) | Value::Map(map) => {
            map.get(field).cloned().unwrap_or(Value::Nil)
        }
        Value::Nil => Value::Nil,
        _ => Value::Nil,
    }
}

pub fn get_index(object: &Value, index: &Value, span: Option<Span>) -> Result<Value> {
    match (object, index) {
        (Value::List(list), Value::Int(i)) => {
            let idx = if *i < 0 {
                (list.len() as i64 + i) as usize
            } else {
                *i as usize
            };
            list.get(idx).cloned().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!("index {i} out of bounds for list of length {}", list.len()),
                    span,
                )
            })
        }
        (Value::Map(map), Value::Str(key)) | (Value::Record(map), Value::Str(key)) => {
            Ok(map.get(key).cloned().unwrap_or(Value::Nil))
        }
        (Value::Str(s), Value::Int(i)) => {
            let idx = if *i < 0 {
                (s.len() as i64 + i) as usize
            } else {
                *i as usize
            };
            s.chars()
                .nth(idx)
                .map(|c| Value::Str(c.to_string()))
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::IndexOutOfBounds,
                        format!("index {i} out of bounds for string of length {}", s.len()),
                        span,
                    )
                })
        }
        _ => Err(BioLangError::type_error(
            format!(
                "cannot index {} with {}",
                object.type_of(),
                index.type_of()
            ),
            span,
        )),
    }
}

pub fn set_field(object: &mut Value, field: &str, value: Value, span: Option<Span>) -> Result<()> {
    match object {
        Value::Record(ref mut map) | Value::Map(ref mut map) => {
            map.insert(field.to_string(), value);
            Ok(())
        }
        _ => Err(BioLangError::type_error(
            format!("cannot set field on {}", object.type_of()),
            span,
        )),
    }
}

pub fn make_record(keys: Vec<String>, values: Vec<Value>) -> Value {
    let mut map = HashMap::new();
    for (k, v) in keys.into_iter().zip(values.into_iter()) {
        map.insert(k, v);
    }
    Value::Record(map)
}
