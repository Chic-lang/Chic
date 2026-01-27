//! Minimal JSON-RPC 2.0 + LSP stdio framing.
//!
//! We intentionally implement only the subset required by `impact-lsp` so we can
//! remove the `lsp-server` / `lsp-types` crate dependencies.

#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io::{BufRead, Write};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

#[derive(Clone, Debug)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    pub params: Value,
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

#[derive(Clone, Debug)]
pub enum IncomingMessage {
    Request(Request),
    Notification(Notification),
    Response,
}

pub fn send_notification<T: Serialize>(
    writer: &mut impl Write,
    method: &str,
    params: &T,
) -> Result<(), String> {
    let params_value = serde_json::to_value(params)
        .map_err(|err| format!("failed to serialise notification params: {err}"))?;
    let mut obj = Map::<String, Value>::new();
    obj.insert("jsonrpc".into(), Value::String("2.0".into()));
    obj.insert("method".into(), Value::String(method.into()));
    obj.insert("params".into(), params_value);
    write_message(writer, &Value::Object(obj))
}

pub fn send_response(writer: &mut impl Write, id: RequestId, result: Value) -> Result<(), String> {
    let mut obj = Map::<String, Value>::new();
    obj.insert("jsonrpc".into(), Value::String("2.0".into()));
    obj.insert("id".into(), serde_json::to_value(id).unwrap_or(Value::Null));
    obj.insert("result".into(), result);
    write_message(writer, &Value::Object(obj))
}

pub fn send_error_response(
    writer: &mut impl Write,
    id: RequestId,
    code: i32,
    message: String,
) -> Result<(), String> {
    let mut err_obj = Map::<String, Value>::new();
    err_obj.insert(
        "code".into(),
        Value::Number(serde_json::Number::from(code as i64)),
    );
    err_obj.insert("message".into(), Value::String(message));

    let mut obj = Map::<String, Value>::new();
    obj.insert("jsonrpc".into(), Value::String("2.0".into()));
    obj.insert("id".into(), serde_json::to_value(id).unwrap_or(Value::Null));
    obj.insert("error".into(), Value::Object(err_obj));
    write_message(writer, &Value::Object(obj))
}

pub fn read_message(reader: &mut impl BufRead) -> Result<Option<IncomingMessage>, String> {
    let body = match read_frame(reader)? {
        Some(body) => body,
        None => return Ok(None),
    };

    let value: Value =
        serde_json::from_slice(&body).map_err(|err| format!("invalid JSON-RPC body: {err}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "JSON-RPC message must be an object".to_string())?;

    let method = obj
        .get("method")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let id = obj.get("id").cloned();

    match (method, id) {
        (Some(method), Some(id)) => {
            let id: RequestId = serde_json::from_value(id)
                .map_err(|err| format!("invalid JSON-RPC request id: {err}"))?;
            let params = obj.get("params").cloned().unwrap_or(Value::Null);
            Ok(Some(IncomingMessage::Request(Request {
                id,
                method,
                params,
            })))
        }
        (Some(method), None) => {
            let params = obj.get("params").cloned().unwrap_or(Value::Null);
            Ok(Some(IncomingMessage::Notification(Notification {
                method,
                params,
            })))
        }
        (None, Some(_id)) => Ok(Some(IncomingMessage::Response)),
        (None, None) => Err("JSON-RPC message missing both method and id".to_string()),
    }
}

fn read_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| format!("failed to read LSP header line: {err}"))?;
        if read == 0 {
            return Ok(None);
        }

        if line == "\r\n" {
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            let parsed: usize = value
                .trim()
                .parse()
                .map_err(|err| format!("invalid Content-Length value: {err}"))?;
            content_length = Some(parsed);
        }
    }

    let len = content_length.ok_or_else(|| "missing Content-Length header".to_string())?;
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .map_err(|err| format!("failed to read LSP body: {err}"))?;
    Ok(Some(buf))
}

fn write_message(writer: &mut impl Write, message: &Value) -> Result<(), String> {
    let body =
        serde_json::to_vec(message).map_err(|err| format!("failed to serialise JSON: {err}"))?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())
        .map_err(|err| format!("failed to write LSP header: {err}"))?;
    writer
        .write_all(&body)
        .map_err(|err| format!("failed to write LSP body: {err}"))?;
    writer
        .flush()
        .map_err(|err| format!("failed to flush LSP output: {err}"))?;
    Ok(())
}
