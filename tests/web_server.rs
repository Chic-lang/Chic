use assert_cmd::cargo::cargo_bin_cmd;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tempfile::{TempDir, tempdir};

mod common;

fn host_target() -> String {
    target_lexicon::HOST.to_string()
}

fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind probe")
        .local_addr()
        .expect("addr")
        .port()
}

struct SampleProject {
    _guard: TempDir,
    artifact: PathBuf,
    _manifest: PathBuf,
}

static SAMPLE: Lazy<Mutex<SampleProject>> = Lazy::new(|| {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let web_pkg = repo_root.join("packages").join("web");
    let guard = tempdir().expect("temp dir");
    let root = guard.path().to_path_buf();
    let manifest = root.join("manifest.yaml");
    let artifact = root.join("web_sample");
    write_sample_project(&root, &web_pkg);
    build_sample(&manifest, &artifact);
    Mutex::new(SampleProject {
        _guard: guard,
        artifact,
        _manifest: manifest,
    })
});

fn sample_artifact() -> PathBuf {
    SAMPLE
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .artifact
        .clone()
}

fn write_sample_project(root: &PathBuf, dependency_path: &PathBuf) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_pkg = repo_root.join("packages").join("runtime.native");
    let manifest = format!(
        r#"
package:
  name: web-sample
  namespace: Web.Sample

build:
  kind: exe

sources:
  - path: .
    namespace_prefix: Web.Sample

dependencies:
  std:
    path: {}
  chic.web:
    path: {}

toolchain:
  runtime:
    kind: native
    package: runtime.native
    abi: rt-abi-1
    path: {}
"#,
        repo_root.join("packages").join("std").display(),
        dependency_path.display(),
        runtime_pkg.display()
    );

    let program = r#"
namespace Web.Sample;

    import Chic.Web;
    import Std.Async;
    import Std;
    import Std.Strings;
    import Std.IO;

public class Program
{
    private static CancellationTokenSource _cts;

    public static int Main()
    {
        _cts = CancellationTokenSource.Create();
        var builder = WebApplication.CreateBuilder();
        let portText = Environment.GetEnvironmentVariable("CHIC_WEB_TEST_PORT");
        if (!string.IsNullOrEmpty(portText))
        {
            builder.Urls = "http://127.0.0.1:" + portText;
        }
        let app = builder.Build();

        app.Use(CreateShortCircuitMiddleware);
        app.Use(CreateHeaderStampMiddleware);

        app.MapGet("/", Hello);
        app.MapPost("/echo", Echo);
        app.MapPost("/chunked", Echo);
        app.MapGet("/route/{id}", RouteValue);
        app.MapGet("/query", QueryValue);
        app.MapGet("/error", ThrowError);
        app.MapPost("/shutdown", Shutdown);

        let runTask = app.RunAsync(_cts.Token());
        Std.Async.Runtime.BlockOn(runTask);
        return 0;
    }

    private static Task Hello(HttpContext context)
    {
        return context.Response.WriteStringAsync("hello");
    }

    private static Task Echo(HttpContext context)
    {
        let bytes = context.Request.Body.ReadAllBytes();
        let span = ReadOnlySpan<byte>.FromArray(ref bytes);
        let text = Utf8String.FromSpan(span);
        return context.Response.WriteStringAsync(text);
    }

    private static Task RouteValue(HttpContext context)
    {
        var value = "missing";
        if (context.Request.RouteValues.TryGetValue("id", out var captured))
        {
            value = captured;
        }
        return context.Response.WriteStringAsync(value);
    }

    private static Task QueryValue(HttpContext context)
    {
        let value = context.Request.Query.GetValueOrDefault("q", "none");
        return context.Response.WriteStringAsync(value);
    }

    private static Task ThrowError(HttpContext context)
    {
        throw new Std.InvalidOperationException("boom");
    }

    private static Task Shutdown(HttpContext context)
    {
        _cts.Cancel();
        return context.Response.WriteStringAsync("bye");
    }

    private static RequestDelegate CreateShortCircuitMiddleware(RequestDelegate next)
    {
        var middleware = new ShortCircuitMiddleware(next);
        return middleware.Invoke;
    }

    private static RequestDelegate CreateHeaderStampMiddleware(RequestDelegate next)
    {
        var middleware = new HeaderStampMiddleware(next);
        return middleware.Invoke;
    }
}
"#
    .to_string();

    let short_circuit = r#"
namespace Web.Sample;

    import Chic.Web;
    import Std.Async;

public sealed class ShortCircuitMiddleware
{
    private RequestDelegate _next;

    public init(RequestDelegate next)
    {
        _next = next;
    }

    public Task Invoke(HttpContext context)
    {
        if (context.Request.Path == "/short-circuit")
        {
            context.Response.StatusCode = 418;
            return context.Response.WriteStringAsync("short");
        }
        return _next(context);
    }
}
"#
    .to_string();

    let header_stamp = r#"
namespace Web.Sample;

    import Chic.Web;
    import Std.Async;

public sealed class HeaderStampMiddleware
{
    private RequestDelegate _next;

    public init(RequestDelegate next)
    {
        _next = next;
    }

    public Task Invoke(HttpContext context)
    {
        context.Response.Headers.Set("X-Before", "yes");
        let task = _next(context);
        Std.Async.Runtime.BlockOn(task);
        context.Response.Headers.Set("X-After", "done");
        return TaskRuntime.CompletedTask();
    }
}
"#
    .to_string();

    common::write_sources(
        root,
        &[
            ("manifest.yaml", &manifest),
            ("Program.ch", &program),
            ("ShortCircuitMiddleware.ch", &short_circuit),
            ("HeaderStampMiddleware.ch", &header_stamp),
        ],
    );
}

fn build_sample(manifest: &PathBuf, output: &PathBuf) {
    cargo_bin_cmd!("chic")
        .env("CHIC_LOG_LEVEL", "error")
        .arg("build")
        .arg(manifest)
        .args([
            "--backend",
            "llvm",
            "--target",
            host_target().as_str(),
            "-o",
            output.to_str().expect("utf8 output"),
        ])
        .assert()
        .success();
}

fn wait_for_server(port: u16) {
    for _ in 0..40 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("server did not start on port {}", port);
}

#[derive(Debug)]
struct ParsedResponse {
    status: u16,
    body: String,
    headers: HashMap<String, String>,
}

fn send_request(stream: &mut TcpStream, request: &str) -> ParsedResponse {
    stream.write_all(request.as_bytes()).expect("write request");
    stream.flush().expect("flush request");
    read_response(stream)
}

fn read_response(stream: &mut TcpStream) -> ParsedResponse {
    let mut buffer = Vec::new();
    let mut header_end = None;
    let mut temp = [0u8; 1024];
    while header_end.is_none() {
        let n = stream.read(&mut temp).expect("read response");
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..n]);
        if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = Some(pos + 4);
        }
    }
    let header_end = header_end.expect("header terminator");
    let raw_headers = String::from_utf8_lossy(&buffer[..header_end]);
    let mut headers = HashMap::new();
    for line in raw_headers.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let status_line = raw_headers.lines().next().expect("status line");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<u16>().ok())
        .expect("status code");
    let content_length = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut temp).expect("read body");
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }
    let body = body[..content_length].to_vec();
    let text = String::from_utf8_lossy(&body).to_string();
    ParsedResponse {
        status,
        body: text,
        headers,
    }
}

fn shutdown_server(port: u16) {
    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
        let request = "POST /shutdown HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(request.as_bytes());
    }
}

struct ServerDropGuard<'a> {
    child: &'a mut Child,
    port: u16,
}

impl<'a> ServerDropGuard<'a> {
    fn new(child: &'a mut Child, port: u16) -> Self {
        Self { child, port }
    }
}

impl Drop for ServerDropGuard<'_> {
    fn drop(&mut self) {
        shutdown_server(self.port);
        match self.child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(_) => return,
        }
        if self.child.wait_timeout(Duration::from_secs(2)).is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn spawn_server(binary: &PathBuf, port: u16) -> Child {
    std::process::Command::new(binary)
        .env("CHIC_WEB_TEST_PORT", port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server")
}

fn header(headers: &HashMap<String, String>, name: &str) -> Option<String> {
    headers.get(&name.to_ascii_lowercase()).cloned()
}

#[test]
#[ignore = "Web package and networking stdlib deps are currently broken; tracked separately from the LSP refactor PR"]
fn http11_server_handles_basic_routes_and_keep_alive() {
    let artifact = sample_artifact();
    let port = find_free_port();

    let mut server = spawn_server(&artifact, port);
    let _server_guard = ServerDropGuard::new(&mut server, port);
    wait_for_server(port);

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to server");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");

    let request1 = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    let response1 = send_request(&mut stream, request1);
    assert_eq!(response1.status, 200);
    assert_eq!(response1.body, "hello");

    let echo_body = "ping";
    let request2 = format!(
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{}",
        echo_body.len(),
        echo_body
    );
    let response2 = send_request(&mut stream, &request2);
    assert_eq!(response2.status, 200);
    assert_eq!(response2.body, echo_body);

    shutdown_server(port);
}

#[test]
#[ignore = "Web package and networking stdlib deps are currently broken; tracked separately from the LSP refactor PR"]
fn chunked_request_bodies_are_consumed() {
    let artifact = sample_artifact();
    let port = find_free_port();
    let mut server = spawn_server(&artifact, port);
    let _server_guard = ServerDropGuard::new(&mut server, port);
    wait_for_server(port);

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");

    let request = "POST /chunked HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n4\r\ntest\r\n0\r\n\r\n";
    let response = send_request(&mut stream, request);
    assert_eq!(response.status, 200);
    assert_eq!(response.body, "test");

    shutdown_server(port);
}

#[test]
#[ignore = "Web package and networking stdlib deps are currently broken; tracked separately from the LSP refactor PR"]
fn middleware_short_circuits_and_stamps_headers() {
    let artifact = sample_artifact();
    let port = find_free_port();
    let mut server = spawn_server(&artifact, port);
    let _server_guard = ServerDropGuard::new(&mut server, port);
    wait_for_server(port);

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let request = "GET /short-circuit HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = send_request(&mut stream, request);
    assert_eq!(response.status, 418);
    assert_eq!(response.body, "short");
    assert_eq!(
        header(&response.headers, "x-before").as_deref(),
        Some("yes")
    );
    assert_eq!(
        header(&response.headers, "x-after").as_deref(),
        Some("done")
    );

    shutdown_server(port);
}

#[test]
#[ignore = "Web package and networking stdlib deps are currently broken; tracked separately from the LSP refactor PR"]
fn routing_and_queries_are_resolved() {
    let artifact = sample_artifact();
    let port = find_free_port();
    let mut server = spawn_server(&artifact, port);
    let _server_guard = ServerDropGuard::new(&mut server, port);
    wait_for_server(port);

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let request_route =
        "GET /route/123 HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
    let route_response = send_request(&mut stream, request_route);
    assert_eq!(route_response.status, 200);
    assert_eq!(route_response.body, "123");

    let request_query = "GET /query?q=abc HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let query_response = send_request(&mut stream, request_query);
    assert_eq!(query_response.status, 200);
    assert_eq!(query_response.body, "abc");

    shutdown_server(port);
}

#[test]
#[ignore = "Web package and networking stdlib deps are currently broken; tracked separately from the LSP refactor PR"]
fn exception_handler_surfaces_500() {
    let artifact = sample_artifact();
    let port = find_free_port();
    let mut server = spawn_server(&artifact, port);
    let _server_guard = ServerDropGuard::new(&mut server, port);
    wait_for_server(port);

    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let request = "GET /error HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let response = send_request(&mut stream, request);
    assert_eq!(response.status, 500);
    assert!(
        response.body.contains("Unhandled exception: boom"),
        "unexpected body: {}",
        response.body
    );

    shutdown_server(port);
}

fn write_protocol_probe(root: &PathBuf, dependency_path: &PathBuf, protocol: &str) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_pkg = repo_root.join("packages").join("runtime.native");
    let manifest = format!(
        r#"
package:
  name: web-probe-{protocol}
  namespace: Web.Probe

build:
  kind: exe

sources:
  - path: .
    namespace_prefix: Web.Probe

dependencies:
  chic.web:
    path: {}

toolchain:
  runtime:
    kind: native
    package: runtime.native
    abi: rt-abi-1
    path: {}
"#,
        dependency_path.display(),
        runtime_pkg.display()
    );

    let program = format!(
        r#"
namespace Web.Probe;

    import Chic.Web;
    import Std.Async;
    import Std;

public class Program
{{
    public static int Main()
    {{
        var builder = WebApplication.CreateBuilder();
        builder.Protocols = HttpProtocols.{protocol};
        let app = builder.Build();
        let cts = CancellationTokenSource.Create();
        cts.Cancel();
        try
        {{
            let _ = app.RunAsync(cts.Token());
        }}
        catch (Std.NotSupportedException)
        {{
            return 0;
        }}
        return 1;
    }}
}}
"#
    );

    common::write_sources(
        root,
        &[("manifest.yaml", &manifest), ("Program.ch", &program)],
    );
}

fn run_protocol_probe(protocol: &str) -> i32 {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let web_pkg = repo_root.join("packages").join("web");
    let probe_root = tempdir().expect("probe dir");
    let root_path = probe_root.path().to_path_buf();
    let manifest = root_path.join("manifest.yaml");
    let artifact = root_path.join(format!("probe_{protocol}"));

    write_protocol_probe(&root_path, &web_pkg, protocol);
    build_sample(&manifest, &artifact);
    let status = std::process::Command::new(&artifact)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run protocol probe");
    status.code().unwrap_or(-1)
}

#[test]
#[ignore = "Web package and networking stdlib deps are currently broken; tracked separately from the LSP refactor PR"]
fn http2_and_http3_are_gated_with_not_supported() {
    assert_eq!(run_protocol_probe("Http2"), 0);
    assert_eq!(run_protocol_probe("Http3"), 0);
}

trait WaitTimeout {
    fn wait_timeout(&mut self, timeout: Duration) -> Option<std::process::ExitStatus>;
}

impl WaitTimeout for Child {
    fn wait_timeout(&mut self, timeout: Duration) -> Option<std::process::ExitStatus> {
        for _ in 0..timeout.as_millis() {
            if let Some(status) = self.try_wait().unwrap() {
                return Some(status);
            }
            thread::sleep(Duration::from_millis(1));
        }
        None
    }
}
