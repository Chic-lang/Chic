#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "harness/mod.rs"]
mod harness;

use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread;

use fixtures::fixture;
use harness::{Category, ExecHarness, HarnessBackend};

fn codegen_exec_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_EXEC").unwrap_or(false)
}

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().is_ok()
}

fn perf_enabled() -> bool {
    env_flag_truthy("CHIC_ENABLE_CODEGEN_PERF").unwrap_or(false)
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    std::env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

fn llvm_harness() -> ExecHarness {
    ExecHarness::new(HarnessBackend::Llvm, Category::Happy)
}

#[test]
fn http_client_gets_body_over_tcp() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_gets_body_over_tcp (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_gets_body_over_tcp (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = b"http-ok";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program =
        fixture!("http/http_client_headers_read.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("stream-body"),
        "expected streaming body, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_posts_json() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping http_client_posts_json (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_posts_json (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = b"json-ok";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program = fixture!("http/http_client_post_json.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("json-ok"),
        "expected json response body, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_resolves_base_address() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_resolves_base_address (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_resolves_base_address (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = b"base-ok";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program =
        fixture!("http/http_client_base_address.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("base-ok"),
        "expected base address body, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_respects_timeout() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_respects_timeout (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_respects_timeout (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
            let body = b"slow";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program = fixture!("http/http_client_timeout.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("timeout"),
        "expected timeout, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_enforces_buffer_limit() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_enforces_buffer_limit (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_enforces_buffer_limit (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = b"0123456789";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program =
        fixture!("http/http_client_buffer_limit.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("buffer-limit"),
        "expected buffer limit failure, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_put_sends_body() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping http_client_put_sends_body (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_put_sends_body (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            let mut content_length = 0usize;
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header_text = String::from_utf8_lossy(&data[..pos]);
                    for line in header_text.lines() {
                        if let Some(rest) = line.strip_prefix("Content-Length: ") {
                            content_length = rest.trim().parse().unwrap_or(0);
                        }
                    }
                    let body_start = pos + 4;
                    while data.len() - body_start < content_length {
                        let read_more = stream.read(&mut buf).unwrap_or(0);
                        if read_more == 0 {
                            break;
                        }
                        data.extend_from_slice(&buf[..read_more]);
                    }
                    let body = &data[body_start..body_start + content_length];
                    let response_body: &[u8] = if body == b"put-body" {
                        b"put-ok"
                    } else {
                        b"put-bad"
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                        response_body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(response_body);
                    break;
                }
            }
        }
    });

    let program = fixture!("http/http_client_put.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("put-ok"),
        "expected put body echo, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_patch_sends_body() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_patch_sends_body (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_patch_sends_body (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            let mut content_length = 0usize;
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let header_text = String::from_utf8_lossy(&data[..pos]);
                    for line in header_text.lines() {
                        if let Some(rest) = line.strip_prefix("Content-Length: ") {
                            content_length = rest.trim().parse().unwrap_or(0);
                        }
                    }
                    let body_start = pos + 4;
                    while data.len() - body_start < content_length {
                        let read_more = stream.read(&mut buf).unwrap_or(0);
                        if read_more == 0 {
                            break;
                        }
                        data.extend_from_slice(&buf[..read_more]);
                    }
                    let body = &data[body_start..body_start + content_length];
                    let response_body: &[u8] = if body == b"patch-body" {
                        b"patch-ok"
                    } else {
                        b"patch-bad"
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                        response_body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.write_all(response_body);
                    break;
                }
            }
        }
    });

    let program = fixture!("http/http_client_patch.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("patch-ok"),
        "expected patch body echo, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_reuses_connection_for_keep_alive() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_reuses_connection_for_keep_alive (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_reuses_connection_for_keep_alive (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            handle_simple_request(&mut stream, b"reuse-one");
            handle_simple_request(&mut stream, b"reuse-two");
        }
    });

    let program = fixture!("http/http_client_reuse.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("reuse-one|reuse-two"),
        "expected pooled responses, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_detects_incomplete_body() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_detects_incomplete_body (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_detects_incomplete_body (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nshrt";
            let _ = stream.write_all(response);
        }
    });

    let program =
        fixture!("http/http_client_incomplete_body.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("incomplete"),
        "expected incomplete body handling, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_supports_head_and_options() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_supports_head_and_options (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_supports_head_and_options (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        let mut handled = 0usize;
        for stream in listener.incoming() {
            if handled >= 2 {
                break;
            }
            if let Ok(mut stream) = stream {
                loop {
                    if handled >= 2 {
                        break;
                    }
                    if let Some(method) = read_request_method(&mut stream) {
                        handled += 1;
                        if method == "HEAD" {
                            let response =
                                b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nX-Head: yes\r\n\r\n";
                            let _ = stream.write_all(response);
                        } else if method == "OPTIONS" {
                            let body = b"options-ok";
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                                body.len()
                            );
                            let _ = stream.write_all(response.as_bytes());
                            let _ = stream.write_all(body);
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    });

    let program =
        fixture!("http/http_client_head_options.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("yes|options-ok"),
        "expected HEAD header and options body, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_respects_canceled_token() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_respects_canceled_token (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_respects_canceled_token (clang not available)");
        return Ok(());
    }

    let program = r#"
namespace Exec;

import Std.Net.Http;

public static class Program
{
    public static int Main()
    {
        var cts = Std.Async.CancellationTokenSource.Create();
        cts.Cancel();
        var client = new HttpClient();
        try
        {
            var response = client.GetAsync("http://127.0.0.1:1/", cts.Token()).Scope();
            return 1;
        }
        catch (Std.TaskCanceledException)
        {
            return 0;
        }
    }
}
"#;

    let harness = llvm_harness();
    let artifact = match harness.build_executable(program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn http_client_cancel_pending_requests() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_cancel_pending_requests (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_cancel_pending_requests (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            // Do not respond; client should cancel.
            let _ = stream.read(&mut [0u8; 128]);
        }
    });

    let program =
        fixture!("http/http_client_cancel_pending.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("canceled"),
        "expected cancelation, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_delete_works() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!("skipping http_client_delete_works (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)");
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_delete_works (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = b"delete-ok";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program = fixture!("http/http_client_delete.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("delete-ok"),
        "expected delete response, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_get_stream_reads_length() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_get_stream_reads_length (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_get_stream_reads_length (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = b"123456789";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program = fixture!("http/http_client_get_stream.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("9"),
        "expected stream length, got `{stdout}`"
    );
    Ok(())
}

#[test]
fn http_client_rejects_exact_unsupported_version() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_rejects_exact_unsupported_version (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_rejects_exact_unsupported_version (clang not available)");
        return Ok(());
    }

    let program = fixture!("http/http_client_bad_version.ch").replace("{{PORT}}", "1");
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    assert_eq!(output.status.code(), Some(0));
    Ok(())
}

#[test]
fn http_client_response_headers_read_streams_body() -> Result<(), Box<dyn Error>> {
    if !codegen_exec_enabled() {
        eprintln!(
            "skipping http_client_response_headers_read_streams_body (set CHIC_ENABLE_CODEGEN_EXEC=1 to enable)"
        );
        return Ok(());
    }
    if !clang_available() {
        eprintln!("skipping http_client_response_headers_read_streams_body (clang not available)");
        return Ok(());
    }

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let mut data = Vec::new();
            loop {
                let read = stream.read(&mut buf).unwrap_or(0);
                if read == 0 {
                    break;
                }
                data.extend_from_slice(&buf[..read]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let body = b"stream-body";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.write_all(body);
        }
    });

    let program = fixture!("http/http_client_basic.ch").replace("{{PORT}}", &port.to_string());
    let harness = llvm_harness();
    let artifact = match harness.build_executable(&program, None) {
        Ok(artifact) => artifact,
        Err(err) => return err.into_test_result(&harness),
    };

    let output = Command::new(artifact.output.path()).output()?;
    server.join().expect("server thread");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("http-ok"),
        "stdout should contain server body, got `{stdout}`"
    );
    Ok(())
}

fn handle_simple_request(stream: &mut TcpStream, body: &[u8]) {
    let mut buf = [0u8; 1024];
    let mut data = Vec::new();
    loop {
        let read = stream.read(&mut buf).unwrap_or(0);
        if read == 0 {
            return;
        }
        data.extend_from_slice(&buf[..read]);
        if data.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
}

fn read_request_method(stream: &mut TcpStream) -> Option<String> {
    let mut buf = [0u8; 1024];
    let mut data = Vec::new();
    loop {
        let read = stream.read(&mut buf).ok()?;
        if read == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..read]);
        if data.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let line_end = data.windows(2).position(|w| w == b"\r\n")?;
    let line = String::from_utf8_lossy(&data[..line_end]);
    let method = line.split_whitespace().next()?.to_string();
    Some(method)
}
