//! Reading what is at an address: the part of the loader the feature `http` adds.

use std::{io::Read, sync::Arc, time::Duration};

use super::LoadError;

/// Reads all of what is at `address`, with `headers` on the request. It fails where the server answers an
/// error, where it takes longer than `timeout`, and where there is more than `max_bytes`, which is known from
/// the length the server says or as soon as more has come; where `cut`, it reads the first `max_bytes` and ends
/// there with them. `told` hears how far it has come after each read and stops it by answering `false`.
pub(super) fn fetch(
  address: &str,
  headers: &[(Arc<str>, Arc<str>)],
  timeout: Duration,
  max_bytes: u64,
  cut: bool,
  mut told: impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  let agent: ureq::Agent = ureq::Agent::config_builder()
    .timeout_global(Some(timeout))
    .build()
    .into();
  let mut request = agent.get(address);
  for (name, value) in headers {
    request = request.header(name.as_ref(), value.as_ref());
  }
  let response = request.call().map_err(error_of)?;
  let body = response.into_body();
  let total = body.content_length();
  if !cut && total.is_some_and(|total| total > max_bytes) {
    return Err(LoadError::TooLarge { limit: max_bytes });
  }
  let mut reader = body.into_reader();
  let mut bytes = Vec::with_capacity(total.unwrap_or(0).min(max_bytes) as usize);
  let mut chunk = vec![0u8; 64 * 1024];
  loop {
    let read = reader.read(&mut chunk).map_err(|error| match error.kind() {
      std::io::ErrorKind::TimedOut => LoadError::TimedOut,
      _ => LoadError::Io(error.to_string()),
    })?;
    if read == 0 {
      return Ok(bytes);
    }
    bytes.extend_from_slice(&chunk[..read]);
    if bytes.len() as u64 >= max_bytes && cut {
      bytes.truncate(usize::try_from(max_bytes).unwrap_or(usize::MAX));
      return Ok(bytes);
    }
    if bytes.len() as u64 > max_bytes {
      return Err(LoadError::TooLarge { limit: max_bytes });
    }
    if !told(bytes.len() as u64, total) {
      return Err(LoadError::Cancelled);
    }
  }
}

fn error_of(error: ureq::Error) -> LoadError {
  match error {
    ureq::Error::StatusCode(404) => LoadError::NotFound,
    ureq::Error::StatusCode(status) => LoadError::Status(status),
    ureq::Error::Timeout(_) => LoadError::TimedOut,
    other => LoadError::Io(other.to_string()),
  }
}

#[cfg(test)]
mod tests {
  use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
  };

  use super::*;

  /// A server on this machine that answers one request with `response`, and gives back what it was sent.
  fn serve_once(response: Vec<u8>) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/picture.png", listener.local_addr().unwrap());
    let served = thread::spawn(move || {
      let (mut stream, _) = listener.accept().unwrap();
      let mut request = Vec::new();
      let mut chunk = [0u8; 1024];
      while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
          break;
        }
        request.extend_from_slice(&chunk[..read]);
      }
      stream.write_all(&response).unwrap();
      String::from_utf8_lossy(&request).into_owned()
    });
    (address, served)
  }

  fn ok(body: &[u8]) -> Vec<u8> {
    let mut response = format!(
      "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
      body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
  }

  const SECOND: Duration = Duration::from_secs(5);

  #[test]
  fn what_is_at_an_address_is_read_with_its_headers_and_its_progress() {
    let body = vec![7u8; 200_000];
    let (address, served) = serve_once(ok(&body));
    let headers = [(Arc::from("X-Asked-By"), Arc::from("lurq"))];
    let mut steps = Vec::new();
    let bytes = fetch(&address, &headers, SECOND, 1 << 20, false, |loaded, total| {
      steps.push((loaded, total));
      true
    })
    .unwrap();
    assert_eq!(bytes, body);
    assert_eq!(steps.last(), Some(&(200_000, Some(200_000))));
    assert!(served.join().unwrap().to_lowercase().contains("x-asked-by: lurq"));
  }

  #[test]
  fn an_answer_that_is_an_error_is_the_error() {
    let (address, served) =
      serve_once(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec());
    assert_eq!(
      fetch(&address, &[], SECOND, 1 << 20, false, |_, _| true),
      Err(LoadError::NotFound)
    );
    served.join().unwrap();
    let (address, served) = serve_once(b"HTTP/1.1 503 Busy\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec());
    assert_eq!(
      fetch(&address, &[], SECOND, 1 << 20, false, |_, _| true),
      Err(LoadError::Status(503))
    );
    served.join().unwrap();
  }

  #[test]
  fn more_than_the_limit_is_not_read() {
    let (address, served) = serve_once(ok(&[1u8; 4096]));
    assert_eq!(
      fetch(&address, &[], SECOND, 1024, false, |_, _| true),
      Err(LoadError::TooLarge { limit: 1024 })
    );
    served.join().unwrap();
  }

  #[test]
  fn a_read_of_the_start_ends_with_the_start_and_no_error() {
    let body: Vec<u8> = (0..4096u32).map(|n| n as u8).collect();
    let (address, served) = serve_once(ok(&body));
    assert_eq!(
      fetch(&address, &[], SECOND, 1000, true, |_, _| true),
      Ok(body[..1000].to_vec())
    );
    served.join().unwrap();
    let (address, served) = serve_once(ok(&body[..10]));
    assert_eq!(
      fetch(&address, &[], SECOND, 1000, true, |_, _| true),
      Ok(body[..10].to_vec())
    );
    served.join().unwrap();
  }

  #[test]
  fn a_load_that_is_stopped_ends() {
    let (address, served) = serve_once(ok(&[1u8; 300_000]));
    assert_eq!(
      fetch(&address, &[], SECOND, 1 << 20, false, |_, _| false),
      Err(LoadError::Cancelled)
    );
    served.join().unwrap();
  }
}
