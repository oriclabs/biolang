use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection};
use std::sync::{Arc, Mutex};

// ── Handle wrapper ───────────────────────────────────────────────

/// Wrapper around a SQLite connection stored in `Value::CompiledClosure`.
/// The `path` field is kept for display purposes.
#[derive(Debug)]
pub struct SqliteDb {
    pub conn: Mutex<Connection>,
    pub path: String,
}

fn extract_db(val: &Value) -> Result<&SqliteDb> {
    match val {
        Value::CompiledClosure(any) => any.downcast_ref::<SqliteDb>().ok_or_else(|| {
            BioLangError::runtime(ErrorKind::TypeError, "expected a SQLite database handle", None)
        }),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("expected a SQLite database handle, got {}", val.type_of()),
            None,
        )),
    }
}

fn biolang_to_sql(val: &Value) -> SqlValue {
    match val {
        Value::Nil => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
        Value::Int(n) => SqlValue::Integer(*n),
        Value::Float(f) => SqlValue::Real(*f),
        Value::Str(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(format!("{other}")),
    }
}

fn sql_to_biolang(val: &SqlValue) -> Value {
    match val {
        SqlValue::Null => Value::Nil,
        SqlValue::Integer(n) => Value::Int(*n),
        SqlValue::Real(f) => Value::Float(*f),
        SqlValue::Text(s) => Value::Str(s.clone()),
        SqlValue::Blob(b) => Value::Str(format!("<blob {} bytes>", b.len())),
    }
}

fn sql_err(e: rusqlite::Error) -> BioLangError {
    BioLangError::runtime(ErrorKind::IOError, format!("SQLite error: {e}"), None)
}

// ── Registration ─────────────────────────────────────────────────

pub fn sqlite_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("sqlite", Arity::Range(0, 1)),
        ("sql", Arity::Range(2, 3)),
        ("sql_insert", Arity::Exact(3)),
        ("sql_tables", Arity::Exact(1)),
        ("sql_schema", Arity::Exact(2)),
        ("sql_close", Arity::Exact(1)),
        ("is_db", Arity::Exact(1)),
    ]
}

pub fn is_sqlite_builtin(name: &str) -> bool {
    matches!(
        name,
        "sqlite" | "sql" | "sql_insert" | "sql_tables" | "sql_schema" | "sql_close" | "is_db"
    )
}

pub fn call_sqlite_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "sqlite" => builtin_sqlite(args),
        "sql" => builtin_sql(args),
        "sql_insert" => builtin_sql_insert(args),
        "sql_tables" => builtin_sql_tables(args),
        "sql_schema" => builtin_sql_schema(args),
        "sql_close" => builtin_sql_close(args),
        "is_db" => builtin_is_db(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown sqlite builtin '{name}'"),
            None,
        )),
    }
}

// ── sqlite(path?) ────────────────────────────────────────────────

fn builtin_sqlite(args: Vec<Value>) -> Result<Value> {
    let path = if args.is_empty() {
        ":memory:".to_string()
    } else {
        match &args[0] {
            Value::Str(s) => s.clone(),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "sqlite() path must be a string",
                    None,
                ))
            }
        }
    };

    let conn = Connection::open(&path).map_err(sql_err)?;
    // Enable WAL mode for better concurrent read performance
    conn.execute_batch("PRAGMA journal_mode=WAL;").ok();

    let db = SqliteDb {
        conn: Mutex::new(conn),
        path: path.clone(),
    };
    Ok(Value::CompiledClosure(Arc::new(db)))
}

// ── sql(db, query, params?) ──────────────────────────────────────

fn builtin_sql(args: Vec<Value>) -> Result<Value> {
    let db = extract_db(&args[0])?;
    let query = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sql() query must be a string",
                None,
            ))
        }
    };

    let params: Vec<SqlValue> = if args.len() > 2 {
        match &args[2] {
            Value::List(items) => items.iter().map(biolang_to_sql).collect(),
            other => vec![biolang_to_sql(other)],
        }
    } else {
        Vec::new()
    };

    let conn = db.conn.lock().unwrap();

    // Detect if this is a SELECT/PRAGMA/EXPLAIN (returns rows) or a write statement
    let trimmed = query.trim_start().to_uppercase();
    let is_query = trimmed.starts_with("SELECT")
        || trimmed.starts_with("PRAGMA")
        || trimmed.starts_with("EXPLAIN")
        || trimmed.starts_with("WITH");

    if is_query {
        let mut stmt = conn.prepare(&query).map_err(sql_err)?;

        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        let rows_result: std::result::Result<Vec<Vec<Value>>, _> = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                let mut vals = Vec::with_capacity(col_count);
                for i in 0..col_count {
                    let sv: SqlValue = row.get(i)?;
                    vals.push(sql_to_biolang(&sv));
                }
                Ok(vals)
            })
            .map_err(sql_err)?
            .collect();

        let rows = rows_result.map_err(sql_err)?;
        Ok(Value::Table(Table::new(col_names, rows)))
    } else {
        let changes = conn
            .execute(&query, params_from_iter(params.iter()))
            .map_err(sql_err)?;
        Ok(Value::Int(changes as i64))
    }
}

// ── sql_insert(db, table_name, data) ─────────────────────────────

fn builtin_sql_insert(args: Vec<Value>) -> Result<Value> {
    let db = extract_db(&args[0])?;
    let table_name = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sql_insert() table_name must be a string",
                None,
            ))
        }
    };

    let conn = db.conn.lock().unwrap();

    match &args[2] {
        Value::Table(t) => {
            if t.columns.is_empty() || t.rows.is_empty() {
                return Ok(Value::Int(0));
            }

            let placeholders: Vec<&str> = (0..t.columns.len()).map(|_| "?").collect();
            let col_list = t
                .columns
                .iter()
                .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "INSERT INTO \"{}\" ({}) VALUES ({})",
                table_name.replace('"', "\"\""),
                col_list,
                placeholders.join(", ")
            );

            let tx = conn.unchecked_transaction().map_err(sql_err)?;
            let mut total = 0usize;
            {
                let mut stmt = tx.prepare(&sql).map_err(sql_err)?;
                for row in &t.rows {
                    let params: Vec<SqlValue> = row.iter().map(biolang_to_sql).collect();
                    stmt.execute(params_from_iter(params.iter()))
                        .map_err(sql_err)?;
                    total += 1;
                }
            }
            tx.commit().map_err(sql_err)?;
            Ok(Value::Int(total as i64))
        }
        Value::List(records) => {
            if records.is_empty() {
                return Ok(Value::Int(0));
            }
            // Extract column names from first record
            let first = match &records[0] {
                Value::Record(m) => m,
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "sql_insert() list items must be records",
                        None,
                    ))
                }
            };
            let cols: Vec<String> = first.keys().cloned().collect();
            let placeholders: Vec<&str> = (0..cols.len()).map(|_| "?").collect();
            let col_list = cols
                .iter()
                .map(|c| format!("\"{}\"", c.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "INSERT INTO \"{}\" ({}) VALUES ({})",
                table_name.replace('"', "\"\""),
                col_list,
                placeholders.join(", ")
            );

            let tx = conn.unchecked_transaction().map_err(sql_err)?;
            let mut total = 0usize;
            {
                let mut stmt = tx.prepare(&sql).map_err(sql_err)?;
                for rec in records {
                    let map = match rec {
                        Value::Record(m) => m,
                        _ => {
                            return Err(BioLangError::runtime(
                                ErrorKind::TypeError,
                                "sql_insert() list items must be records",
                                None,
                            ))
                        }
                    };
                    let params: Vec<SqlValue> = cols
                        .iter()
                        .map(|c| biolang_to_sql(map.get(c).unwrap_or(&Value::Nil)))
                        .collect();
                    stmt.execute(params_from_iter(params.iter()))
                        .map_err(sql_err)?;
                    total += 1;
                }
            }
            tx.commit().map_err(sql_err)?;
            Ok(Value::Int(total as i64))
        }
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sql_insert() data must be a Table or list of records",
            None,
        )),
    }
}

// ── sql_tables(db) ───────────────────────────────────────────────

fn builtin_sql_tables(args: Vec<Value>) -> Result<Value> {
    let db = extract_db(&args[0])?;
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .map_err(sql_err)?;
    let names: Vec<Value> = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(Value::Str(name))
        })
        .map_err(sql_err)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Value::List(names))
}

// ── sql_schema(db, table_name) ───────────────────────────────────

fn builtin_sql_schema(args: Vec<Value>) -> Result<Value> {
    let db = extract_db(&args[0])?;
    let table_name = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sql_schema() table_name must be a string",
                None,
            ))
        }
    };

    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "PRAGMA table_info(\"{}\")",
            table_name.replace('"', "\"\"")
        ))
        .map_err(sql_err)?;

    let columns: Vec<String> = vec![
        "cid".into(),
        "name".into(),
        "type".into(),
        "notnull".into(),
        "pk".into(),
    ];
    let rows: Vec<Vec<Value>> = stmt
        .query_map([], |row| {
            Ok(vec![
                Value::Int(row.get::<_, i64>(0)?),
                Value::Str(row.get::<_, String>(1)?),
                Value::Str(row.get::<_, String>(2)?),
                Value::Bool(row.get::<_, bool>(3)?),
                Value::Bool(row.get::<_, bool>(5)?),
            ])
        })
        .map_err(sql_err)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── sql_close(db) ────────────────────────────────────────────────

fn builtin_sql_close(_args: Vec<Value>) -> Result<Value> {
    // Connection is inside Arc<SqliteDb> — we can't consume it.
    // But we can drop it from the environment. Since BioLang is GC-free and
    // uses reference counting, the Connection drops when the last reference
    // to the Value is gone. This is a no-op hint for clarity.
    Ok(Value::Nil)
}

// ── is_db(value) ─────────────────────────────────────────────────

fn builtin_is_db(args: Vec<Value>) -> Result<Value> {
    let is = matches!(&args[0], Value::CompiledClosure(any) if any.downcast_ref::<SqliteDb>().is_some());
    Ok(Value::Bool(is))
}
