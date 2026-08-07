//! Minimal but real MCP JSON-RPC 2.0 over stdio (tools only).
//! Spec: https://modelcontextprotocol.io — initialize / tools/list / tools/call / notifications.

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::tools::ToolCtx;

pub fn serve_stdio(ctx: ToolCtx) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let lines = stdin.lock().lines();

    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                write_msg(
                    &mut stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": format!("parse error: {e}") }
                    }),
                )?;
                continue;
            }
        };
        // Notifications have no id — ack by ignoring response.
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));

        if method == "notifications/initialized" || method.starts_with("notifications/") {
            continue;
        }

        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": "aevum-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "Aevum Unify trust path. Tools gate real effects via temporal graph authorization. Untrusted remote memories cannot authorize until promoted."
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(crate::tools::list_tools_value()),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                match crate::tools::dispatch(&ctx, name, &args) {
                    Ok(text) => Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    })),
                    Err(e) => Ok(json!({
                        "content": [{ "type": "text", "text": format!("ERROR: {e}") }],
                        "isError": true
                    })),
                }
            }
            "" => Err((-32600, "invalid request".into())),
            other => Err((-32601, format!("method not found: {other}"))),
        };

        if id.is_none() {
            continue;
        }
        match result {
            Ok(r) => write_msg(
                &mut stdout,
                json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            )?,
            Err((code, message)) => write_msg(
                &mut stdout,
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": code, "message": message }
                }),
            )?,
        }
    }
    Ok(())
}

fn write_msg(out: &mut impl Write, v: Value) -> io::Result<()> {
    let s = serde_json::to_string(&v).unwrap();
    writeln!(out, "{s}")?;
    out.flush()
}
