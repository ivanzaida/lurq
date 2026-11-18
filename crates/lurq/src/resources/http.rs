//! Reading what is at an address: the part of the loader the feature `http` adds.

use std::{
  io::Read,
  sync::Arc,
  time::{Duration, Instant},
};

use super::LoadError;

/// How many times more a server that says it is busy (429 Too Many Requests, 503 Service Unavailable) is asked.
/// Such an answer passes: the picture of a GitHub page, asked for the first time, is often a 429 that the next
/// request a second later is not.
const BUSY_RETRIES: u32 = 2;

/// The longest pause before a busy server is asked again, whatever its `Retry-After` says: a load waits seconds,
/// not the minutes a rate limit may name.
const BUSY_PAUSE: Duration = Duration::from_secs(2);

/// Reads all of what is at `address`, with `headers` on the request. It fails where the server answers an
/// error, where it takes longer than `timeout`, and where there is more than `max_bytes`, which is known from
/// the length the server says or as soon as more has come; where `cut`, it reads the first `max_bytes` and ends
/// there with them. A server that says it is busy is asked again, twice at most and within `timeout`. `told`
/// hears how far it has come after each read and stops it by answering `false`.
pub(super) fn fetch(
  address: &str,
  headers: &[(Arc<str>, Arc<str>)],
  timeout: Duration,
  max_bytes: u64,
  cut: bool,
  mut told: impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  let started = Instant::now();
  let mut busy = 0;
  let response = loop {
    let left = timeout.saturating_sub(started.elapsed());
    if left.is_zero() {
      return Err(LoadError::TimedOut);
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
      .timeout_global(Some(left))
      .http_status_as_error(false)
      .build()
      .into();
    let mut request = agent.get(address);
    for (name, value) in headers {
      request = request.header(name.as_ref(), value.as_ref());
    }
    let response = request.call().map_err(error_of)?;
    let status = response.status().as_u16();
    if status < 400 {
      break response;
    }
    if matches!(status, 429 | 503) && busy < BUSY_RETRIES {
      busy += 1;
      let said = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs);
      let pause = said
        .unwrap_or(Duration::from_secs(u64::from(busy)))
        .min(BUSY_PAUSE)
        .min(timeout.saturating_sub(started.elapsed()));
      tracing::debug!(target: "lurq::resources", "{address} answered {status}: asked again in {pause:?}");
      if !told(0, None) {
        return Err(LoadError::Cancelled);
      }
      std::thread::sleep(pause);
      continue;
    }
    return Err(match status {
      404 => LoadError::NotFound,
      status => LoadError::Status(status),
    });
  };
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
    let (address, served) = serve(vec![response]);
    (address, thread::spawn(move || served.join().unwrap().remove(0)))
  }

  /// A server on this machine that answers each request it is sent with the next of `responses`, and gives back
  /// what it was sent.
  fn serve(responses: Vec<Vec<u8>>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/picture.png", listener.local_addr().unwrap());
    let served = thread::spawn(move || {
      let mut requests = Vec::new();
      for response in responses {
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
        requests.push(String::from_utf8_lossy(&request).into_owned());
      }
      requests
    });
    (address, served)
  }

  fn busy(status: &str, retry_after: &str) -> Vec<u8> {
    format!("HTTP/1.1 {status}\r\nRetry-After: {retry_after}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
      .into_bytes()
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
    let (address, served) =
      serve_once(b"HTTP/1.1 500 Broken\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec());
    assert_eq!(
      fetch(&address, &[], SECOND, 1 << 20, false, |_, _| true),
      Err(LoadError::Status(500))
    );
    served.join().unwrap();
  }

  #[test]
  fn a_server_that_says_it_is_busy_is_asked_again_and_its_answer_is_read() {
    // GitHub's pictures of its pages answer 429 to a first request now and then, and the next is the picture.
    let (address, served) = serve(vec![
      busy("429 Too Many Requests", "556"),
      busy("503 Service Unavailable", "0"),
      ok(b"the picture"),
    ]);
    let started = std::time::Instant::now();
    assert_eq!(
      fetch(&address, &[], SECOND, 1 << 20, false, |_, _| true),
      Ok(b"the picture".to_vec())
    );
    assert_eq!(served.join().unwrap().len(), 3);
    let waited = started.elapsed();
    assert!(
      waited < Duration::from_secs(4),
      "a long Retry-After is not waited for: {waited:?}"
    );
  }

  #[test]
  fn a_server_that_stays_busy_is_its_status_after_two_more_asks() {
    let (address, served) = serve(vec![
      busy("503 Busy", "0"),
      busy("503 Busy", "0"),
      busy("503 Busy", "0"),
    ]);
    assert_eq!(
      fetch(&address, &[], SECOND, 1 << 20, false, |_, _| true),
      Err(LoadError::Status(503))
    );
    assert_eq!(served.join().unwrap().len(), 3, "asked three times in all");
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
