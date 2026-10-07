//! Minimal streamable-HTTP MCP client for end-to-end tests: JSON-RPC over
//! POST, replies as JSON or SSE, one connection per request.
#![allow(dead_code)]

use std::{
  io::{BufRead, BufReader, Read, Write},
  net::TcpStream,
  time::Duration,
};

use serde_json::{Value, json};

pub struct Client {
  port: u16,
  token: String,
  session: Option<String>,
  next_id: u64,
}

impl Client {
  pub fn new(port: u16, token: String) -> Self {
    Self {
      port,
      token,
      session: None,
      next_id: 0,
    }
  }

  pub fn port(&self) -> u16 {
    self.port
  }

  pub fn token(&self) -> &str {
    &self.token
  }

  fn connect(&self) -> TcpStream {
    let stream = TcpStream::connect(("127.0.0.1", self.port)).expect("connect to the MCP server");
    stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    stream
  }

  fn session_header(&self) -> String {
    self
      .session
      .as_ref()
      .map(|id| format!("mcp-session-id: {id}\r\n"))
      .unwrap_or_default()
  }

  fn post(&mut self, body: &Value) -> Option<Value> {
    let mut stream = self.connect();
    let payload = body.to_string();
    write!(
      stream,
      "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\n\
       Accept: application/json, text/event-stream\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
      self.port,
      self.token,
      self.session_header(),
      payload.len()
    )
    .unwrap();
    let mut reader = BufReader::new(stream);
    let mut chunked = false;
    let mut length = None;
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains(" 200 ") || line.contains(" 202 "), "HTTP status: {line}");
    loop {
      line.clear();
      reader.read_line(&mut line).unwrap();
      let Some((name, value)) = line.trim_end().split_once(':') else {
        break;
      };
      let (name, value) = (name.to_ascii_lowercase(), value.trim());
      match name.as_str() {
        "mcp-session-id" => self.session = Some(value.to_owned()),
        "transfer-encoding" => chunked = value.eq_ignore_ascii_case("chunked"),
        "content-length" => length = value.parse::<usize>().ok(),
        _ => {}
      }
    }
    let wanted = body.get("id")?.clone();
    let mut received = Vec::new();
    loop {
      if chunked {
        line.clear();
        reader.read_line(&mut line).unwrap();
        let size = usize::from_str_radix(line.trim(), 16).expect("chunk size");
        if size == 0 {
          break;
        }
        let mut chunk = vec![0; size + 2];
        reader.read_exact(&mut chunk).unwrap();
        received.extend_from_slice(&chunk[..size]);
      } else {
        let mut all = vec![0; length.unwrap_or(0)];
        reader.read_exact(&mut all).unwrap();
        received = all;
      }
      let text = String::from_utf8_lossy(&received);
      let messages = text
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .chain(text.trim_start().starts_with('{').then_some(text.as_ref()))
        .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok());
      if let Some(reply) = messages.into_iter().find(|message| message.get("id") == Some(&wanted)) {
        return Some(reply);
      }
      assert!(chunked, "no JSON-RPC reply in {text}");
    }
    panic!("stream ended without a reply to {wanted}");
  }

  pub fn initialize(&mut self) {
    self.request(
      "initialize",
      json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "lurq-test", "version": "0"}}),
    );
    self.post(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
  }

  pub fn request(&mut self, method: &str, params: Value) -> Value {
    self.next_id += 1;
    let reply = self
      .post(&json!({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}))
      .expect("requests get replies");
    reply
      .get("result")
      .cloned()
      .unwrap_or_else(|| panic!("{method} failed: {reply}"))
  }

  /// Call a tool and return its text content.
  pub fn tool(&mut self, name: &str, arguments: Value) -> String {
    let result = self.request("tools/call", json!({"name": name, "arguments": arguments}));
    assert_ne!(result["isError"], true, "{name} failed: {result}");
    result["content"][0]["text"].as_str().unwrap_or_default().to_owned()
  }

  /// Call a tool that answers JSON.
  pub fn tool_json(&mut self, name: &str, arguments: Value) -> Value {
    serde_json::from_str(&self.tool(name, arguments)).expect("the tool answers JSON")
  }

  /// End the session (`DELETE /mcp`), as a client that is done does.
  pub fn close(&mut self) {
    let session = self.session.take().expect("an initialized session");
    let mut stream = self.connect();
    write!(
      stream,
      "DELETE /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nmcp-session-id: {session}\r\n\
       Content-Length: 0\r\nConnection: close\r\n\r\n",
      self.port, self.token,
    )
    .unwrap();
    let mut status = String::new();
    BufReader::new(stream).read_line(&mut status).unwrap();
    assert!(status.contains(" 20"), "DELETE status: {status}");
  }
}
