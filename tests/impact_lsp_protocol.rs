use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command as StdCommand, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use url::Url;

struct ChildGuard {
    child: Child,
    stdout_thread: Option<JoinHandle<()>>,
}

impl ChildGuard {
    fn new(child: Child, stdout_thread: JoinHandle<()>) -> Self {
        Self {
            child,
            stdout_thread: Some(stdout_thread),
        }
    }
}

impl std::ops::Deref for ChildGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl std::ops::DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let mut still_running = false;
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                still_running = true;
            }
            Err(_) => {
                still_running = true;
            }
        }
        if still_running {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(handle) = self.stdout_thread.take() {
            let _ = handle.join();
        }
    }
}

fn write_message(stdin: &mut ChildStdin, payload: &Value) {
    let body = payload.to_string();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdin
        .write_all(header.as_bytes())
        .expect("write header to LSP server");
    stdin
        .write_all(body.as_bytes())
        .expect("write body to LSP server");
    stdin.flush().expect("flush LSP stdin");
}

fn read_message(stdout: &mut BufReader<ChildStdout>) -> Option<Value> {
    let mut content_length = None;
    let mut header_line = String::new();
    loop {
        header_line.clear();
        let bytes = stdout.read_line(&mut header_line).ok()?;
        if bytes == 0 {
            return None;
        }
        let trimmed = header_line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(raw_len) = trimmed.strip_prefix("Content-Length:") {
            let len = raw_len
                .trim()
                .parse::<usize>()
                .expect("parse content length");
            content_length = Some(len);
        }
    }
    let len = content_length?;
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn spawn_lsp() -> (ChildGuard, ChildStdin, Receiver<Value>) {
    let binary = assert_cmd::cargo_bin!("impact-lsp");
    let mut child = StdCommand::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn impact-lsp");
    let stdin = child.stdin.take().expect("capture stdin");
    let stdout = child.stdout.take().expect("capture stdout");
    let (tx, rx) = mpsc::channel();
    let stdout_thread = thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Some(value) = read_message(&mut reader) {
            if tx.send(value).is_err() {
                break;
            }
        }
    });
    (ChildGuard::new(child, stdout_thread), stdin, rx)
}

#[test]
fn initialize_and_publish_diagnostics() {
    let (mut child, mut stdin, rx) = spawn_lsp();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {},
            "trace": "off"
        }
    });
    write_message(&mut stdin, &init_request);
    let init_response = loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(message) if message.get("id") == Some(&json!(1)) => break message,
            Ok(_) => continue,
            Err(err) => panic!("initialize response missing: {err}"),
        }
    };
    assert_eq!(init_response.get("id"), Some(&json!(1)));
    assert!(init_response.get("result").is_some());

    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_message(&mut stdin, &initialized);

    let dir = tempdir().expect("create temp dir");
    let file_path = dir.path().join("sample.ch");
    let uri = Url::from_file_path(&file_path).expect("file URI");
    let bad_source = "namespace Test;\nfn main() { let =; }";
    let did_open = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri.as_str(),
                "languageId": "chic",
                "version": 1,
                "text": bad_source,
            }
        }
    });
    write_message(&mut stdin, &did_open);

    let mut diagnostics: Option<Value> = None;
    let diag_deadline = Instant::now() + Duration::from_secs(30);
    while diagnostics.is_none() && Instant::now() < diag_deadline {
        if let Ok(message) = rx.recv_timeout(Duration::from_millis(200)) {
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
            {
                diagnostics = message
                    .get("params")
                    .map(|params| params["diagnostics"].clone());
            }
        }
    }
    let diags = diagnostics.expect("publishDiagnostics not received");
    assert!(
        diags.as_array().map_or(false, |array| !array.is_empty()),
        "expected diagnostics for malformed source"
    );

    let hover_request = json!({
        "jsonrpc": "2.0",
        "id": 99,
        "method": "textDocument/hover",
        "params": {
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": 0, "character": 0 }
        }
    });
    write_message(&mut stdin, &hover_request);
    let mut hover_response: Option<Value> = None;
    let hover_deadline = Instant::now() + Duration::from_secs(3);
    while hover_response.is_none() && Instant::now() < hover_deadline {
        if let Ok(message) = rx.recv_timeout(Duration::from_millis(200)) {
            if message.get("id") == Some(&json!(99)) {
                hover_response = Some(message);
            }
        }
    }
    let hover = hover_response.expect("hover response");
    assert!(
        hover.get("result").is_some(),
        "hover response should include a result field"
    );

    let goto_definition = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": 0, "character": 0 }
        }
    });
    write_message(&mut stdin, &goto_definition);
    let mut definition_response: Option<Value> = None;
    let def_deadline = Instant::now() + Duration::from_secs(3);
    while definition_response.is_none() && Instant::now() < def_deadline {
        if let Ok(message) = rx.recv_timeout(Duration::from_millis(200)) {
            if message.get("id") == Some(&json!(100)) {
                definition_response = Some(message);
            }
        }
    }
    let definition = definition_response.expect("definition response");
    assert!(
        definition.get("result").is_some(),
        "definition response should include a result field"
    );

    let shutdown = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
        "params": null
    });
    write_message(&mut stdin, &shutdown);
    let exit = json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": {}
    });
    write_message(&mut stdin, &exit);
    let _ = child.wait().expect("wait for server");
}

#[test]
fn typecheck_diagnostics_and_definition_navigation() {
    println!("starting typecheck_diagnostics_and_definition_navigation");
    let (mut child, mut stdin, rx) = spawn_lsp();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {},
            "trace": "off"
        }
    });
    write_message(&mut stdin, &init_request);
    let _ = loop {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(message) if message.get("id") == Some(&json!(1)) => break message,
            Ok(_) => continue,
            Err(err) => panic!("initialize response missing: {err}"),
        }
    };
    println!("lsp initialised for typecheck test");

    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_message(&mut stdin, &initialized);

    let dir = tempdir().expect("create temp dir");
    let file_path = dir.path().join("sample.ch");
    let uri = Url::from_file_path(&file_path).expect("file URI");
    let source = r#"namespace RefDiagnostics;

public ref int Alias(in int value)
{
    return ref value;
}

public int Main()
{
    return Alias(5);
}
"#;
    let did_open = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri.as_str(),
                "languageId": "chic",
                "version": 1,
                "text": source,
            }
        }
    });
    write_message(&mut stdin, &did_open);

    let mut diagnostics: Option<Value> = None;
    let start = Instant::now();
    while diagnostics.is_none() && start.elapsed() < Duration::from_secs(30) {
        if let Ok(message) = rx.recv_timeout(Duration::from_millis(200)) {
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
            {
                diagnostics = message
                    .get("params")
                    .map(|params| params["diagnostics"].clone());
            }
        }
    }
    let diags = diagnostics.expect("publishDiagnostics not received");
    println!("received diagnostics: {diags:?}");
    assert!(
        diags.as_array().map_or(false, |array| !array.is_empty()),
        "expected typecheck diagnostics for bad return type"
    );

    let definition_request = json!({
        "jsonrpc": "2.0",
        "id": 200,
        "method": "textDocument/definition",
        "params": {
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": 9, "character": 11 }
        }
    });
    write_message(&mut stdin, &definition_request);
    let mut definition_response: Option<Value> = None;
    let deadline = Instant::now() + Duration::from_secs(3);
    while definition_response.is_none() && Instant::now() < deadline {
        if let Ok(message) = rx.recv_timeout(Duration::from_millis(200)) {
            if message.get("id") == Some(&json!(200)) {
                definition_response = Some(message);
            }
        }
    }
    let definition = definition_response.expect("definition response");
    println!("definition response: {definition:?}");
    let result = definition
        .get("result")
        .cloned()
        .unwrap_or_else(|| json!(null));
    let location_uri = result
        .get("uri")
        .and_then(Value::as_str)
        .expect("definition uri missing");
    assert!(
        location_uri.ends_with("sample.ch"),
        "definition should resolve to the opened file"
    );
    let def_line = result["range"]["start"]["line"]
        .as_u64()
        .expect("definition line missing");
    assert_eq!(def_line, 3, "definition should point to the function decl");

    let shutdown = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown",
        "params": null
    });
    write_message(&mut stdin, &shutdown);
    let exit = json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": {}
    });
    write_message(&mut stdin, &exit);
    println!("shutdown sent");
    let _ = child.wait().expect("wait for server");
}
