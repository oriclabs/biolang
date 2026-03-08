use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;

// ── Registration ─────────────────────────────────────────────────

pub fn notify_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("notify", Arity::Exact(1)),
        ("slack", Arity::Range(1, 2)),
        ("teams", Arity::Range(1, 2)),
        ("telegram", Arity::Range(1, 3)),
        ("discord", Arity::Range(1, 2)),
        ("email", Arity::Range(3, 4)),
    ]
}

pub fn is_notify_builtin(name: &str) -> bool {
    matches!(
        name,
        "notify" | "slack" | "teams" | "telegram" | "discord" | "email"
    )
}

pub fn call_notify_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "notify" => builtin_notify(args),
        "slack" => builtin_slack(args),
        "teams" => builtin_teams(args),
        "telegram" => builtin_telegram(args),
        "discord" => builtin_discord(args),
        "email" => builtin_email(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown notify builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn env_var(name: &str) -> Result<String> {
    std::env::var(name).map_err(|_| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("environment variable {name} is not set"),
            None,
        )
    })
}

fn post_json(url: &str, body: &str) -> Result<String> {
    let agent = crate::http::shared_agent();
    let resp = agent
        .post(url)
        .set("Content-Type", "application/json")
        .send_string(body)
        .map_err(|e| {
            BioLangError::runtime(
                ErrorKind::IOError,
                format!("notification failed: {e}"),
                None,
            )
        })?;
    let status = resp.status();
    let text = resp.into_string().unwrap_or_default();
    if status >= 200 && status < 300 {
        Ok(text)
    } else {
        // Single retry
        let resp2 = agent
            .post(url)
            .set("Content-Type", "application/json")
            .send_string(body)
            .map_err(|e| {
                BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("notification failed after retry: {e}"),
                    None,
                )
            })?;
        let status2 = resp2.status();
        if status2 >= 200 && status2 < 300 {
            Ok(resp2.into_string().unwrap_or_default())
        } else {
            Err(BioLangError::runtime(
                ErrorKind::IOError,
                format!("notification failed with status {status2} (first attempt: {status})"),
                None,
            ))
        }
    }
}

/// Extract a message string from a Value. If it's a Record, format it as
/// structured fields; otherwise convert to string.
fn extract_message(val: &Value) -> String {
    match val {
        Value::Str(s) => s.clone(),
        Value::Record(map) => format_record_message(map),
        other => format!("{other}"),
    }
}

/// Format a record into a readable notification message.
fn format_record_message(map: &HashMap<String, Value>) -> String {
    let mut parts = Vec::new();
    if let Some(Value::Str(title)) = map.get("title") {
        parts.push(title.clone());
    }
    if let Some(Value::Str(status)) = map.get("status") {
        parts.push(format!("Status: {status}"));
    }
    if let Some(Value::Str(msg)) = map.get("message") {
        parts.push(msg.clone());
    }
    // Append any "fields" record
    if let Some(Value::Record(fields)) = map.get("fields") {
        for (k, v) in fields {
            parts.push(format!("{k}: {v}"));
        }
    }
    if parts.is_empty() {
        // Fall back to listing all keys
        for (k, v) in map {
            parts.push(format!("{k}: {v}"));
        }
    }
    parts.join("\n")
}

// ── Slack ────────────────────────────────────────────────────────

/// Format a Value as Slack Block Kit JSON.
fn slack_payload(val: &Value, webhook: Option<&str>) -> Result<(String, String)> {
    let url = match webhook {
        Some(u) => u.to_string(),
        None => env_var("SLACK_WEBHOOK")?,
    };

    let body = match val {
        Value::Record(map) => {
            let mut blocks = Vec::new();

            if let Some(Value::Str(title)) = map.get("title") {
                blocks.push(serde_json::json!({
                    "type": "header",
                    "text": {"type": "plain_text", "text": title}
                }));
            }

            let mut fields_md = Vec::new();
            if let Some(Value::Str(status)) = map.get("status") {
                fields_md.push(format!("*Status:* {status}"));
            }
            if let Some(Value::Str(msg)) = map.get("message") {
                fields_md.push(msg.clone());
            }
            if let Some(Value::Record(fields)) = map.get("fields") {
                for (k, v) in fields {
                    fields_md.push(format!("*{k}:* {v}"));
                }
            }
            if !fields_md.is_empty() {
                blocks.push(serde_json::json!({
                    "type": "section",
                    "text": {"type": "mrkdwn", "text": fields_md.join("\n")}
                }));
            }
            if blocks.is_empty() {
                // Fallback: plain text
                serde_json::json!({"text": format_record_message(map)}).to_string()
            } else {
                serde_json::json!({"blocks": blocks}).to_string()
            }
        }
        _ => {
            let text = extract_message(val);
            serde_json::json!({"text": text}).to_string()
        }
    };

    Ok((url, body))
}

fn builtin_slack(args: Vec<Value>) -> Result<Value> {
    let (url, body) = match args.len() {
        1 => slack_payload(&args[0], None)?,
        _ => {
            let webhook = match &args[0] {
                Value::Str(s) => s.as_str(),
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "slack() first argument must be a webhook URL string",
                        None,
                    ))
                }
            };
            slack_payload(&args[1], Some(webhook))?
        }
    };

    post_json(&url, &body)?;
    Ok(Value::Nil)
}

// ── Teams ────────────────────────────────────────────────────────

fn teams_payload(val: &Value, webhook: Option<&str>) -> Result<(String, String)> {
    let url = match webhook {
        Some(u) => u.to_string(),
        None => env_var("TEAMS_WEBHOOK")?,
    };

    let body = match val {
        Value::Record(map) => {
            let title = map
                .get("title")
                .map(|v| format!("{v}"))
                .unwrap_or_default();
            let text = extract_message(val);

            let mut facts = Vec::new();
            if let Some(Value::Record(fields)) = map.get("fields") {
                for (k, v) in fields {
                    facts.push(serde_json::json!({"name": k, "value": format!("{v}")}));
                }
            }

            let mut card = serde_json::json!({
                "@type": "MessageCard",
                "@context": "http://schema.org/extensions",
                "themeColor": "7C3AED",
                "summary": if title.is_empty() { text.clone() } else { title.clone() },
                "sections": [{
                    "activityTitle": title,
                    "text": text,
                }]
            });
            if !facts.is_empty() {
                card["sections"][0]["facts"] = serde_json::json!(facts);
            }
            card.to_string()
        }
        _ => {
            let text = extract_message(val);
            serde_json::json!({
                "@type": "MessageCard",
                "@context": "http://schema.org/extensions",
                "themeColor": "7C3AED",
                "text": text
            })
            .to_string()
        }
    };

    Ok((url, body))
}

fn builtin_teams(args: Vec<Value>) -> Result<Value> {
    let (url, body) = match args.len() {
        1 => teams_payload(&args[0], None)?,
        _ => {
            let webhook = match &args[0] {
                Value::Str(s) => s.as_str(),
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "teams() first argument must be a webhook URL string",
                        None,
                    ))
                }
            };
            teams_payload(&args[1], Some(webhook))?
        }
    };

    post_json(&url, &body)?;
    Ok(Value::Nil)
}

// ── Telegram ─────────────────────────────────────────────────────

fn builtin_telegram(args: Vec<Value>) -> Result<Value> {
    let (token, chat_id, message) = match args.len() {
        1 => {
            // telegram(message) — read config from env
            let token = env_var("TELEGRAM_BOT_TOKEN")?;
            let chat_id = env_var("TELEGRAM_CHAT_ID")?;
            let msg = extract_message(&args[0]);
            (token, chat_id, msg)
        }
        2 => {
            // telegram(chat_id, message) — token from env
            let token = env_var("TELEGRAM_BOT_TOKEN")?;
            let chat_id = match &args[0] {
                Value::Str(s) => s.clone(),
                Value::Int(n) => n.to_string(),
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "telegram() chat_id must be a string or integer",
                        None,
                    ))
                }
            };
            let msg = extract_message(&args[1]);
            (token, chat_id, msg)
        }
        _ => {
            // telegram(token, chat_id, message)
            let token = match &args[0] {
                Value::Str(s) => s.clone(),
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "telegram() token must be a string",
                        None,
                    ))
                }
            };
            let chat_id = match &args[1] {
                Value::Str(s) => s.clone(),
                Value::Int(n) => n.to_string(),
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "telegram() chat_id must be a string or integer",
                        None,
                    ))
                }
            };
            let msg = extract_message(&args[2]);
            (token, chat_id, msg)
        }
    };

    let url = format!("https://api.telegram.org/bot{token}/sendMessage");

    // Telegram supports Markdown formatting
    let parse_mode = "Markdown";
    let body =
        serde_json::json!({"chat_id": chat_id, "text": message, "parse_mode": parse_mode})
            .to_string();

    post_json(&url, &body)?;
    Ok(Value::Nil)
}

// ── Discord ──────────────────────────────────────────────────────

fn discord_payload(val: &Value, webhook: Option<&str>) -> Result<(String, String)> {
    let url = match webhook {
        Some(u) => u.to_string(),
        None => env_var("DISCORD_WEBHOOK")?,
    };

    let body = match val {
        Value::Record(map) => {
            let title = map
                .get("title")
                .map(|v| format!("{v}"))
                .unwrap_or_default();

            let mut fields = Vec::new();
            if let Some(Value::Record(flds)) = map.get("fields") {
                for (k, v) in flds {
                    fields.push(serde_json::json!({
                        "name": k,
                        "value": format!("{v}"),
                        "inline": true
                    }));
                }
            }

            let mut embed = serde_json::json!({
                "color": 8076015,
            });
            if !title.is_empty() {
                embed["title"] = serde_json::json!(title);
            }
            let desc = extract_message(val);
            if !desc.is_empty() {
                embed["description"] = serde_json::json!(desc);
            }
            if !fields.is_empty() {
                embed["fields"] = serde_json::json!(fields);
            }
            serde_json::json!({"embeds": [embed]}).to_string()
        }
        _ => {
            let text = extract_message(val);
            serde_json::json!({"content": text}).to_string()
        }
    };

    Ok((url, body))
}

fn builtin_discord(args: Vec<Value>) -> Result<Value> {
    let (url, body) = match args.len() {
        1 => discord_payload(&args[0], None)?,
        _ => {
            let webhook = match &args[0] {
                Value::Str(s) => s.as_str(),
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "discord() first argument must be a webhook URL string",
                        None,
                    ))
                }
            };
            discord_payload(&args[1], Some(webhook))?
        }
    };

    post_json(&url, &body)?;
    Ok(Value::Nil)
}

// ── Email (SMTP) ─────────────────────────────────────────────────

fn builtin_email(args: Vec<Value>) -> Result<Value> {
    let to = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "email() `to` must be a string",
                None,
            ))
        }
    };
    let subject = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "email() `subject` must be a string",
                None,
            ))
        }
    };
    let body = match &args[2] {
        Value::Str(s) => s.clone(),
        Value::Record(map) => format_record_message(map),
        other => format!("{other}"),
    };

    let smtp_host = env_var("SMTP_HOST")?;
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".to_string())
        .parse()
        .unwrap_or(587);
    let smtp_user = env_var("SMTP_USER")?;
    let smtp_pass = env_var("SMTP_PASS")?;
    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| smtp_user.clone());

    // Raw SMTP session over TCP. Zero extra deps.
    use std::io::{BufRead, BufReader};
    use std::net::TcpStream;

    let tcp = TcpStream::connect(format!("{smtp_host}:{smtp_port}")).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("SMTP connection failed: {e}"), None)
    })?;
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(15))).ok();
    tcp.set_write_timeout(Some(std::time::Duration::from_secs(15))).ok();

    let mut reader = BufReader::new(tcp.try_clone().unwrap());
    let mut writer = tcp;

    smtp_read(&mut reader)?; // greeting

    smtp_send(&mut writer, &mut reader, "EHLO biolang")?;
    // Drain multi-line EHLO
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap_or(0);
        if line.len() < 4 || line.as_bytes().get(3) == Some(&b' ') {
            break;
        }
    }

    smtp_send(&mut writer, &mut reader, "AUTH LOGIN")?;
    smtp_send(&mut writer, &mut reader, &base64_encode(smtp_user.as_bytes()))?;
    let auth_reply = smtp_send(&mut writer, &mut reader, &base64_encode(smtp_pass.as_bytes()))?;
    if !auth_reply.starts_with('2') {
        return Err(BioLangError::runtime(
            ErrorKind::IOError, format!("SMTP auth failed: {auth_reply}"), None,
        ));
    }

    smtp_send(&mut writer, &mut reader, &format!("MAIL FROM:<{from}>"))?;
    smtp_send(&mut writer, &mut reader, &format!("RCPT TO:<{to}>"))?;
    smtp_send(&mut writer, &mut reader, "DATA")?;

    let msg = format!(
        "From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}\r\n."
    );
    let reply = smtp_send(&mut writer, &mut reader, &msg)?;
    if !reply.starts_with('2') {
        return Err(BioLangError::runtime(
            ErrorKind::IOError, format!("SMTP send failed: {reply}"), None,
        ));
    }

    let _ = smtp_send(&mut writer, &mut reader, "QUIT");

    Ok(Value::Nil)
}

fn smtp_read(reader: &mut impl std::io::BufRead) -> Result<String> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("SMTP read error: {e}"), None)
    })?;
    Ok(line)
}

fn smtp_send(
    writer: &mut impl std::io::Write,
    reader: &mut impl std::io::BufRead,
    cmd: &str,
) -> Result<String> {
    writer
        .write_all(format!("{cmd}\r\n").as_bytes())
        .map_err(|e| {
            BioLangError::runtime(ErrorKind::IOError, format!("SMTP write error: {e}"), None)
        })?;
    writer.flush().ok();
    smtp_read(reader)
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

// ── notify() — smart router ─────────────────────────────────────

fn builtin_notify(args: Vec<Value>) -> Result<Value> {
    let provider = std::env::var("BIOLANG_NOTIFY").unwrap_or_default();

    match provider.to_lowercase().as_str() {
        "slack" => builtin_slack(args),
        "teams" => builtin_teams(args),
        "telegram" => builtin_telegram(args),
        "discord" => builtin_discord(args),
        "email" => {
            // For email via notify(), use NOTIFY_EMAIL_TO and NOTIFY_EMAIL_SUBJECT
            let to = env_var("NOTIFY_EMAIL_TO")?;
            let subject = std::env::var("NOTIFY_EMAIL_SUBJECT")
                .unwrap_or_else(|_| "BioLang Notification".to_string());
            let body_val = args.into_iter().next().unwrap_or(Value::Nil);
            let body_str = extract_message(&body_val);
            builtin_email(vec![
                Value::Str(to),
                Value::Str(subject),
                Value::Str(body_str),
            ])
        }
        "" => {
            // No provider configured — print to stderr as fallback
            let msg = extract_message(&args[0]);
            eprintln!("[notify] {msg}");
            Ok(Value::Nil)
        }
        other => Err(BioLangError::runtime(
            ErrorKind::IOError,
            format!(
                "unknown BIOLANG_NOTIFY provider '{other}' (expected: slack, teams, telegram, discord, email)"
            ),
            None,
        )),
    }
}
