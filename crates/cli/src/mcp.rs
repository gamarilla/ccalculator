//! A minimal Model Context Protocol (MCP) server over stdio.
//!
//! Speaks newline-delimited JSON-RPC 2.0. Exposes the calculator engine as MCP
//! tools so an AI assistant can evaluate expressions, convert units, and change
//! the display base — all sharing one persistent session engine. This same
//! interface is what the end-to-end tests drive.

use std::io::{BufRead, Write};

use ccalc_core::Engine;
use serde_json::{json, Value};

const PROTOCOL_VERSION: &str = "2024-11-05";

pub fn serve() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut engine = Engine::new();
    crate::persist::load_into(&mut engine);

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                write_msg(&mut stdout, &error_response(Value::Null, -32700, &format!("parse error: {e}")))?;
                continue;
            }
        };
        if let Some(resp) = handle(&mut engine, &req) {
            write_msg(&mut stdout, &resp)?;
        }
    }
    Ok(())
}

fn write_msg(out: &mut impl Write, msg: &Value) -> anyhow::Result<()> {
    out.write_all(msg.to_string().as_bytes())?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

/// Handle one JSON-RPC request, returning a response (or None for notifications).
fn handle(engine: &mut Engine, req: &Value) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "ccalc", "version": env!("CARGO_PKG_VERSION") }
            }
        })),
        "notifications/initialized" | "initialized" => None,
        "ping" => Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": tool_specs() }
        })),
        "tools/call" => Some(handle_tool_call(engine, id, req)),
        _ => id.map(|id| error_response(id, -32601, &format!("method not found: {method}"))),
    }
}

fn tool_specs() -> Value {
    json!([
        {
            "name": "evaluate",
            "description": "Evaluate a Console Calculator expression or command. Supports high-precision arithmetic, variables, user functions, unit conversion (e.g. '10 in -> cm'), base conversion, and commands (mode deg, display hex, sigfigs N, ...). State persists across calls in this session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expression": { "type": "string", "description": "The expression or command to evaluate." }
                },
                "required": ["expression"]
            }
        },
        {
            "name": "convert_units",
            "description": "Convert a value from one unit to another, e.g. value=10 from='in' to='cm'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "value": { "type": "string", "description": "Numeric value (as a string or number)." },
                    "from": { "type": "string", "description": "Source unit, e.g. 'mi/h'." },
                    "to": { "type": "string", "description": "Target unit, e.g. 'm/s'." }
                },
                "required": ["value", "from", "to"]
            }
        },
        {
            "name": "convert_base",
            "description": "Show an integer value in a given base: 'dec', 'hex', or 'bin'.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "value": { "type": "string", "description": "The value/expression to display." },
                    "base": { "type": "string", "enum": ["dec", "hex", "bin"] }
                },
                "required": ["value", "base"]
            }
        },
        {
            "name": "reset",
            "description": "Reset the calculator session, clearing user variables, functions, and units.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

fn handle_tool_call(engine: &mut Engine, id: Option<Value>, req: &Value) -> Value {
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let text = match name {
        "evaluate" => {
            let expr = args.get("expression").and_then(|v| v.as_str()).unwrap_or("");
            eval_to_text(engine, expr)
        }
        "convert_units" => {
            let value = arg_str(&args, "value");
            let from = arg_str(&args, "from");
            let to = arg_str(&args, "to");
            let line = format!("{value} {from} -> {to}");
            eval_to_text(engine, &line)
        }
        "convert_base" => {
            let value = arg_str(&args, "value");
            let base = arg_str(&args, "base");
            eval_to_text(engine, &format!("display {base}; {value}"))
        }
        "reset" => {
            *engine = Engine::new();
            "session reset".to_string()
        }
        other => {
            return error_response(id.unwrap_or(Value::Null), -32602, &format!("unknown tool: {other}"));
        }
    };

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [ { "type": "text", "text": text } ]
        }
    })
}

fn arg_str(args: &Value, key: &str) -> String {
    match args.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Evaluate a line and join all visible outputs into one text block.
fn eval_to_text(engine: &mut Engine, line: &str) -> String {
    let outs = engine.run_line(line);
    let lines: Vec<String> = outs.iter().filter_map(|ev| engine.format_eval(ev)).collect();
    if lines.is_empty() {
        "(no output)".to_string()
    } else {
        lines.join("\n")
    }
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}
