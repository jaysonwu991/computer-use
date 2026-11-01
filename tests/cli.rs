//! End-to-end checks of the binary: CLI surface, the stdio MCP handshake and
//! the Oxide configuration writer.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_oxide-computer-use")
}

fn run(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(binary())
        .args(args)
        .output()
        .expect("the binary runs");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

#[test]
fn tools_lists_every_registered_tool() {
    let (stdout, _, ok) = run(&["tools"]);
    assert!(ok);
    let names: Vec<&str> = stdout.lines().collect();
    assert_eq!(names.len(), 11, "unexpected tool list: {stdout}");
    assert!(names.contains(&"list_apps"));
    assert!(names.contains(&"get_app_state"));
    assert!(names.contains(&"set_value"));
}

#[test]
fn the_mcp_server_answers_initialize_and_tools_list() {
    let mut child = Command::new(binary())
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the MCP server starts");
    let mut stdin = child.stdin.take().expect("stdin");
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
    )
    .unwrap();
    drop(stdin);
    let output = child.wait_with_output().expect("the server exits");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let responses: Vec<Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is one JSON packet"))
        .collect();
    assert_eq!(
        responses.len(),
        2,
        "notifications are not answered: {stdout}"
    );
    assert_eq!(responses[0]["id"], json!(1));
    assert!(responses[0]["result"]["capabilities"]["tools"].is_object());
    assert_eq!(responses[1]["id"], json!(2));
    assert!(responses[1]["result"]["tools"]
        .as_array()
        .is_some_and(|tools| tools.len() == 11));
}

#[test]
fn install_oxide_prints_a_stdio_server_entry() {
    let (stdout, _, ok) = run(&["install-oxide", "--print"]);
    assert!(ok);
    let value: Value = serde_json::from_str(&stdout).expect("print emits JSON");
    let entry = &value["mcpServers"]["computer-use"];
    assert_eq!(entry["args"], json!(["mcp"]));
    assert!(
        entry["command"]
            .as_str()
            .is_some_and(|command| command.ends_with("oxide-computer-use")),
        "unexpected command: {entry}"
    );
}

#[test]
fn an_unknown_tool_is_reported_as_an_error_result() {
    let (stdout, stderr, ok) = run(&["call", "no_such_tool"]);
    assert!(!ok, "a failed call exits nonzero");
    assert!(stderr.contains("unknown tool"), "{stderr}");
    assert!(stdout.contains("\"isError\":true"), "{stdout}");
}
