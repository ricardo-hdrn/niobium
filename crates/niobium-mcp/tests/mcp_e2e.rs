// End-to-end tests for the Niobium MCP server.
//
// These tests launch the actual binary, send JSON-RPC messages over stdio,
// and verify the responses. They test the real MCP protocol flow without
// needing the Flutter UI (only schema storage tools are exercised).

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

/// Path to the compiled binary (built by `cargo test`).
fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    path.push("niobium");
    path
}

/// Send a sequence of JSON-RPC messages to the niobium binary and collect responses.
/// Messages are sent with small delays between them to ensure sequential processing.
/// Each call gets its own data directory to avoid parallel test interference.
fn run_mcp_session(messages: &[Value]) -> Vec<Value> {
    let binary = binary_path();

    let data_dir = tempfile::tempdir().expect("failed to create temp dir");

    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("NIOBIUM_DATA_DIR", data_dir.path())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to start {}: {e}", binary.display()));

    let stdin = child.stdin.as_mut().expect("failed to open stdin");

    for msg in messages {
        let line = serde_json::to_string(msg).unwrap();
        writeln!(stdin, "{line}").expect("failed to write to stdin");
        // Small delay between messages to ensure sequential processing
        std::thread::sleep(Duration::from_millis(200));
    }

    // Give the server time to process the last message
    std::thread::sleep(Duration::from_secs(2));
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to read stdout");

    let stdout = String::from_utf8_lossy(&output.stdout);

    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn init_message() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "e2e-test", "version": "0.1" }
        }
    })
}

fn initialized_notification() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    })
}

fn tool_call(id: u64, name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments }
    })
}

fn tools_list(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/list",
        "params": {}
    })
}

/// Extract the text content from a tools/call result.
fn extract_tool_text(response: &Value) -> String {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_handshake() {
    let responses = run_mcp_session(&[init_message()]);

    assert!(!responses.is_empty(), "should get at least one response");

    let resp = &responses[0];
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    assert!(resp["result"]["capabilities"]["tools"].is_object());
    assert!(
        resp["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("Niobium")
    );
}

#[test]
fn test_tools_list_returns_all_tools() {
    let responses = run_mcp_session(&[init_message(), initialized_notification(), tools_list(2)]);

    // Find the tools/list response (id=2)
    let tools_resp = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("missing tools/list response");

    let tools = tools_resp["result"]["tools"]
        .as_array()
        .expect("tools should be array");

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"show_form"), "missing show_form");
    assert!(
        tool_names.contains(&"show_confirmation"),
        "missing show_confirmation"
    );
    assert!(tool_names.contains(&"show_output"), "missing show_output");
    assert!(tool_names.contains(&"save_form"), "missing save_form");
    assert!(tool_names.contains(&"list_forms"), "missing list_forms");
    assert!(
        tool_names.contains(&"show_saved_form"),
        "missing show_saved_form"
    );
    assert_eq!(tool_names.len(), 6, "expected exactly 6 tools");
}

#[test]
fn test_save_and_list_forms() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "email": { "type": "string", "format": "email" }
        },
        "required": ["name", "email"]
    });

    let responses = run_mcp_session(&[
        init_message(),
        initialized_notification(),
        tool_call(
            2,
            "save_form",
            json!({
                "name": "contact",
                "schema": schema,
                "description": "Contact information form"
            }),
        ),
        tool_call(3, "list_forms", json!({})),
    ]);

    // Find save_form response
    let save_resp = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("missing save_form response");
    let save_text = extract_tool_text(save_resp);
    let save_data: Value = serde_json::from_str(&save_text).unwrap();
    assert_eq!(save_data["name"], "contact");
    assert_eq!(save_data["version"], 1);

    // Find list_forms response
    let list_resp = responses
        .iter()
        .find(|r| r["id"] == 3)
        .expect("missing list_forms response");
    let list_text = extract_tool_text(list_resp);
    let list_data: Value = serde_json::from_str(&list_text).unwrap();
    let forms = list_data.as_array().expect("list should be array");
    assert_eq!(forms.len(), 1);
    assert_eq!(forms[0]["name"], "contact");
    assert_eq!(forms[0]["description"], "Contact information form");
}

#[test]
fn test_save_form_auto_versions() {
    let responses = run_mcp_session(&[
        init_message(),
        initialized_notification(),
        tool_call(
            2,
            "save_form",
            json!({
                "name": "survey",
                "schema": { "type": "object", "properties": { "q1": { "type": "string" } } },
                "description": "v1"
            }),
        ),
        tool_call(
            3,
            "save_form",
            json!({
                "name": "survey",
                "schema": { "type": "object", "properties": { "q1": { "type": "string" }, "q2": { "type": "string" } } },
                "description": "v2 with extra question"
            }),
        ),
        tool_call(4, "list_forms", json!({})),
    ]);

    // v1
    let r2 = responses.iter().find(|r| r["id"] == 2).unwrap();
    let d2: Value = serde_json::from_str(&extract_tool_text(r2)).unwrap();
    assert_eq!(d2["version"], 1);

    // v2
    let r3 = responses.iter().find(|r| r["id"] == 3).unwrap();
    let d3: Value = serde_json::from_str(&extract_tool_text(r3)).unwrap();
    assert_eq!(d3["version"], 2);

    // list returns only latest
    let r4 = responses.iter().find(|r| r["id"] == 4).unwrap();
    let list: Value = serde_json::from_str(&extract_tool_text(r4)).unwrap();
    let forms = list.as_array().unwrap();
    assert_eq!(forms.len(), 1);
    assert_eq!(forms[0]["version"], 2);
    assert_eq!(forms[0]["description"], "v2 with extra question");
}

#[test]
fn test_list_forms_empty() {
    let responses = run_mcp_session(&[
        init_message(),
        initialized_notification(),
        tool_call(2, "list_forms", json!({})),
    ]);

    let resp = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("missing list_forms response");
    let text = extract_tool_text(resp);
    let data: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(data.as_array().unwrap().len(), 0);
}

#[test]
fn test_show_form_rejects_non_object_schema() {
    let responses = run_mcp_session(&[
        init_message(),
        initialized_notification(),
        tool_call(
            2,
            "show_form",
            json!({
                "schema": "not an object"
            }),
        ),
    ]);

    let resp = responses
        .iter()
        .find(|r| r["id"] == 2)
        .expect("missing show_form response");
    // Should be an error
    assert!(
        resp.get("error").is_some(),
        "should return error for non-object schema, got: {resp}"
    );
}
