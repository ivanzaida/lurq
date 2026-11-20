//! Reading what is at an address: the part of the loader the feature `http` adds.

use std::{
  collections::{HashSet, VecDeque},
  io::{self, Read},
  ops::Range,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
  time::{Duration, Instant},
};

use parking_lot::{Condvar, Mutex};

use super::{LoadError, Reader};

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
  agents: &Agents,
  address: &str,
  headers: &[(Arc<str>, Arc<str>)],
  timeout: Duration,
  max_bytes: u64,
  cut: bool,
  mut told: impl FnMut(u64, Option<u64>) -> bool,
) -> Result<Vec<u8>, LoadError> {
  let agent = agents.lend();
  let response = ask(&agent, address, headers, None, timeout, &mut told)?;
  let body = response.into_body();
  let total = body.content_length();
  if !cut && total.is_some_and(|total| total > max_bytes) {
    return Err(LoadError::TooLarge { limit: max_bytes });
  }
  let mut reader = body.into_reader();
  let mut bytes = Vec::with_capacity(total.unwrap_or(0).min(max_bytes) as usize);
  let mut chunk = vec![0u8; 64 * 1024];
  loop {
    let read = reader.read(&mut chunk).map_err(read_failed)?;
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

/// The loader's HTTP agents, made once and lent to its loads and readers: an agent keeps the connections its
/// requests opened, so the next request to the same host lent it goes over one of them, with no new handshake.
/// As many are kept as the loader's pool is large ([`Loader::pool_size`](super::Loader::pool_size)); one given
/// back beyond that is let go, with its connections.
pub(super) struct Agents {
  idle: Mutex<Vec<ureq::Agent>>,
  size: AtomicUsize,
  /// How many were made, for a test that they are made once.
  made: AtomicUsize,
}

impl Agents {
  pub(super) fn new(size: usize) -> Arc<Self> {
    Arc::new(Self {
      idle: Mutex::new(Vec::new()),
      size: AtomicUsize::new(size),
      made: AtomicUsize::new(0),
    })
  }

  pub(super) fn resize(&self, size: usize) {
    self.size.store(size, Ordering::Release);
    self.idle.lock().truncate(size);
  }

  /// How many agents were made since the loader was.
  pub(super) fn made(&self) -> usize {
    self.made.load(Ordering::Acquire)
  }

  /// An agent for a request, given back when the lent one is dropped: the one given back last, which most likely
  /// holds a connection to the host asked again, or a new one where none is idle.
  pub(super) fn lend(&self) -> Lent<'_> {
    let agent = self.idle.lock().pop().unwrap_or_else(|| {
      self.made.fetch_add(1, Ordering::AcqRel);
      ureq::Agent::config_builder().http_status_as_error(false).build().into()
    });
    Lent {
      agents: self,
      agent: Some(agent),
    }
  }
}

pub(super) struct Lent<'a> {
  agents: &'a Agents,
  agent: Option<ureq::Agent>,
}

impl std::ops::Deref for Lent<'_> {
  type Target = ureq::Agent;

  fn deref(&self) -> &ureq::Agent {
    self.agent.as_ref().expect("an agent until it is given back")
  }
}

impl Drop for Lent<'_> {
  fn drop(&mut self) {
    let mut idle = self.agents.idle.lock();
    if idle.len() < self.agents.size.load(Ordering::Acquire)
      && let Some(agent) = self.agent.take()
    {
      idle.push(agent);
    }
  }
}

/// The answer of the server at `address` to a request with `headers`, and `range` where it asks for a part of
/// it. The request and the reading of its answer end within `timeout`. A server that says it is busy is asked
/// again, twice at most and within `timeout`; `told` is asked before each pause whether to go on.
fn ask(
  agent: &ureq::Agent,
  address: &str,
  headers: &[(Arc<str>, Arc<str>)],
  range: Option<&str>,
  timeout: Duration,
  told: &mut dyn FnMut(u64, Option<u64>) -> bool,
) -> Result<ureq::http::Response<ureq::Body>, LoadError> {
  let started = Instant::now();
  let mut busy = 0;
  loop {
    let left = timeout.saturating_sub(started.elapsed());
    if left.is_zero() {
      return Err(LoadError::TimedOut);
    }
    let mut request = agent.get(address);
    for (name, value) in headers {
      request = request.header(name.as_ref(), value.as_ref());
    }
    if let Some(range) = range {
      request = request.header("Range", range);
    }
    let response = request
      .config()
      .timeout_global(Some(left))
      .build()
      .call()
      .map_err(error_of)?;
    let status = response.status().as_u16();
    if status < 400 {
      return Ok(response);
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
  }
}

fn read_failed(error: io::Error) -> LoadError {
  match error.kind() {
    io::ErrorKind::TimedOut => LoadError::TimedOut,
    _ => LoadError::Io(error.to_string()),
  }
}

/// How many blocks an [`HttpReader`] keeps: a decoder reads the start of a file again, and where it was.
const HELD_BLOCKS: usize = 16;
/// How many blocks past the one read are fetched before they are asked for, so that playing does not wait for
/// each.
const READ_AHEAD: u64 = 2;

/// What is at an address, read by ranges as it is asked for, with its headers on every request: what a decoder
/// reads as it plays.
pub(super) struct HttpReader(Arc<Ranges>);

struct Ranges {
  address: String,
  headers: Vec<(Arc<str>, Arc<str>)>,
  timeout: Duration,
  block: u64,
  /// The loader's agents: each request goes over a connection one of them kept, where it has one to the host.
  agents: Arc<Agents>,
  blocks: Mutex<Blocks>,
  /// A block that was on its way came, or failed.
  came: Condvar,
}

#[derive(Default)]
struct Blocks {
  /// How long it is, once an answer said.
  len: Option<u64>,
  content_type: Option<Arc<str>>,
  /// The blocks there are, by their index, the one read last at the back.
  held: VecDeque<(u64, Arc<[u8]>)>,
  /// The blocks on their way.
  fetching: HashSet<u64>,
}

/// What a block's request gave: its bytes, how long all of it is and what it is, where the answer said.
struct Fetched {
  bytes: Vec<u8>,
  len: Option<u64>,
  content_type: Option<Arc<str>>,
}

impl HttpReader {
  /// Asks for the first block of what is at `address`, `block` bytes, and gives a reader of it: the answer's
  /// status, length and type are known when it returns, and a server that refuses it fails here.
  pub(super) fn open(
    agents: Arc<Agents>,
    address: String,
    headers: Vec<(Arc<str>, Arc<str>)>,
    timeout: Duration,
    block: u64,
  ) -> Result<Self, LoadError> {
    let ranges = Arc::new(Ranges {
      address,
      headers,
      timeout,
      block: block.max(1),
      agents,
      blocks: Mutex::new(Blocks::default()),
      came: Condvar::new(),
    });
    ranges.block(0)?;
    Ok(Self(ranges))
  }
}

impl Reader for HttpReader {
  fn len(&self) -> Option<u64> {
    self.0.blocks.lock().len
  }

  fn content_type(&self) -> Option<Arc<str>> {
    self.0.blocks.lock().content_type.clone()
  }

  fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
    if buf.is_empty() || self.0.blocks.lock().len.is_some_and(|len| offset >= len) {
      return Ok(0);
    }
    let size = self.0.block;
    let index = offset / size;
    let block = self.0.block(index).map_err(io::Error::other)?;
    for ahead in 1..=READ_AHEAD {
      self.0.ahead(index + ahead);
    }
    let rest = block.get((offset - index * size) as usize..).unwrap_or(&[]);
    let count = rest.len().min(buf.len());
    buf[..count].copy_from_slice(&rest[..count]);
    Ok(count)
  }

  fn buffered(&self) -> Vec<Range<u64>> {
    let size = self.0.block;
    let mut held: Vec<Range<u64>> = self
      .0
      .blocks
      .lock()
      .held
      .iter()
      .map(|(index, bytes)| index * size..index * size + bytes.len() as u64)
      .collect();
    held.sort_by_key(|range| range.start);
    let mut joined: Vec<Range<u64>> = Vec::new();
    for range in held {
      match joined.last_mut() {
        Some(last) if last.end >= range.start => last.end = last.end.max(range.end),
        _ => joined.push(range),
      }
    }
    joined
  }
}

impl Ranges {
  /// The block at `index`: held, fetched by another read and waited for, or fetched now.
  fn block(&self, index: u64) -> Result<Arc<[u8]>, LoadError> {
    let mut blocks = self.blocks.lock();
    loop {
      if let Some(at) = blocks.held.iter().position(|(held, _)| *held == index) {
        let held = blocks.held.remove(at).expect("the block just found");
        let bytes = held.1.clone();
        blocks.held.push_back(held);
        return Ok(bytes);
      }
      if !blocks.fetching.contains(&index) {
        break;
      }
      self.came.wait(&mut blocks);
    }
    blocks.fetching.insert(index);
    drop(blocks);
    let fetched = self.fetch(index);
    self.keep(index, fetched)
  }

  /// Starts fetching the block at `index` on a thread of its own, where it is not there or on its way.
  fn ahead(self: &Arc<Self>, index: u64) {
    {
      let mut blocks = self.blocks.lock();
      if blocks.len.is_some_and(|len| index * self.block >= len)
        || blocks.fetching.contains(&index)
        || blocks.held.iter().any(|(held, _)| *held == index)
      {
        return;
      }
      blocks.fetching.insert(index);
    }
    let ranges = self.clone();
    let spawned = std::thread::Builder::new()
      .name("lurq-read-ahead".to_owned())
      .spawn(move || {
        let fetched = ranges.fetch(index);
        let _ = ranges.keep(index, fetched);
      });
    if spawned.is_err() {
      self.blocks.lock().fetching.remove(&index);
      self.came.notify_all();
    }
  }

  /// Keeps what fetching the block at `index` gave, and wakes the reads that wait for it. A block that failed is
  /// not kept: the next read of it asks again.
  fn keep(&self, index: u64, fetched: Result<Fetched, LoadError>) -> Result<Arc<[u8]>, LoadError> {
    let mut blocks = self.blocks.lock();
    blocks.fetching.remove(&index);
    let kept = fetched.map(|fetched| {
      if fetched.len.is_some() {
        blocks.len = fetched.len;
      }
      if fetched.content_type.is_some() {
        blocks.content_type = fetched.content_type;
      }
      let bytes: Arc<[u8]> = Arc::from(fetched.bytes);
      blocks.held.push_back((index, bytes.clone()));
      // The first block is kept whatever was read since: a file's start says what it is, and a media that goes
      // on from its start at its end, or is sent there, starts from it at once.
      if blocks.held.len() > HELD_BLOCKS
        && let Some(oldest) = blocks.held.iter().position(|(held, _)| *held != 0)
      {
        blocks.held.remove(oldest);
      }
      bytes
    });
    drop(blocks);
    self.came.notify_all();
    kept
  }

  /// The bytes of the block at `index`, fewer at the end.
  fn fetch(&self, index: u64) -> Result<Fetched, LoadError> {
    let start = index * self.block;
    let mut fetched = Fetched {
      bytes: Vec::with_capacity(self.block as usize),
      len: None,
      content_type: None,
    };
    let filled = |fetched: &Fetched| fetched.bytes.len() as u64;
    // A server may answer a range with less of it than was asked: the rest is asked for again.
    while filled(&fetched) < self.block && fetched.len.is_none_or(|len| start + filled(&fetched) < len) {
      let from = start + filled(&fetched);
      let range = format!("bytes={from}-{}", start + self.block - 1);
      let agent = self.agents.lend();
      let response = match ask(
        &agent,
        &self.address,
        &self.headers,
        Some(&range),
        self.timeout,
        &mut |_, _| true,
      ) {
        Ok(response) => response,
        // Past the end, where its length was not known yet.
        Err(LoadError::Status(416)) => break,
        Err(error) => return Err(error),
      };
      let ranged = response.status().as_u16() == 206;
      let header = |name: &str| response.headers().get(name).and_then(|value| value.to_str().ok());
      let said = if ranged {
        total_of(header("content-range"))
      } else {
        response.body().content_length()
      };
      fetched.len = said.or(fetched.len);
      fetched.content_type = header("content-type")
        .map(|kind| Arc::from(kind.split(';').next().unwrap_or(kind).trim()))
        .or(fetched.content_type);
      let mut reader = response.into_body().into_reader();
      if !ranged {
        // A server that does not read ranges answers with all of it, from its start: what is before the block
        // is read past, once for each block.
        io::copy(&mut (&mut reader).take(from), &mut io::sink()).map_err(read_failed)?;
      }
      let before = fetched.bytes.len();
      (&mut reader)
        .take(self.block - before as u64)
        .read_to_end(&mut fetched.bytes)
        .map_err(read_failed)?;
      if !ranged {
        break;
      }
      // A connection goes back to its agent once its answer is read to its end, and the part asked for is the
      // whole answer: what is left is nothing, read to see the end.
      io::copy(&mut reader, &mut io::sink()).map_err(read_failed)?;
      if fetched.bytes.len() == before {
        break;
      }
    }
    Ok(fetched)
  }
}

/// How long all of it is, from a `Content-Range` such as `bytes 0-1023/4096`: `None` where it says `*`.
fn total_of(content_range: Option<&str>) -> Option<u64> {
  content_range?.rsplit('/').next()?.trim().parse().ok()
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
    sync::atomic::{AtomicBool, Ordering},
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
    let bytes = fetch(
      &Agents::new(1),
      &address,
      &headers,
      SECOND,
      1 << 20,
      false,
      |loaded, total| {
        steps.push((loaded, total));
        true
      },
    )
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
      fetch(&Agents::new(1), &address, &[], SECOND, 1 << 20, false, |_, _| true),
      Err(LoadError::NotFound)
    );
    served.join().unwrap();
    let (address, served) =
      serve_once(b"HTTP/1.1 500 Broken\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec());
    assert_eq!(
      fetch(&Agents::new(1), &address, &[], SECOND, 1 << 20, false, |_, _| true),
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
      fetch(&Agents::new(1), &address, &[], SECOND, 1 << 20, false, |_, _| true),
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
      fetch(&Agents::new(1), &address, &[], SECOND, 1 << 20, false, |_, _| true),
      Err(LoadError::Status(503))
    );
    assert_eq!(served.join().unwrap().len(), 3, "asked three times in all");
  }

  #[test]
  fn more_than_the_limit_is_not_read() {
    let (address, served) = serve_once(ok(&[1u8; 4096]));
    assert_eq!(
      fetch(&Agents::new(1), &address, &[], SECOND, 1024, false, |_, _| true),
      Err(LoadError::TooLarge { limit: 1024 })
    );
    served.join().unwrap();
  }

  #[test]
  fn a_read_of_the_start_ends_with_the_start_and_no_error() {
    let body: Vec<u8> = (0..4096u32).map(|n| n as u8).collect();
    let (address, served) = serve_once(ok(&body));
    assert_eq!(
      fetch(&Agents::new(1), &address, &[], SECOND, 1000, true, |_, _| true),
      Ok(body[..1000].to_vec())
    );
    served.join().unwrap();
    let (address, served) = serve_once(ok(&body[..10]));
    assert_eq!(
      fetch(&Agents::new(1), &address, &[], SECOND, 1000, true, |_, _| true),
      Ok(body[..10].to_vec())
    );
    served.join().unwrap();
  }

  #[test]
  fn a_load_that_is_stopped_ends() {
    let (address, served) = serve_once(ok(&[1u8; 300_000]));
    assert_eq!(
      fetch(&Agents::new(1), &address, &[], SECOND, 1 << 20, false, |_, _| false),
      Err(LoadError::Cancelled)
    );
    served.join().unwrap();
  }

  /// A server on this machine that serves `body` as `video/mp4`, the part a `Range` asks for where `ranges`, and
  /// answers 403 to a request without `X-Asked-By: lurq`, and to every request once `refusing` is set. It gives
  /// back the ranges it was asked for.
  fn serve_file(body: Vec<u8>, ranges: bool) -> (String, Arc<Mutex<Vec<String>>>, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/clip.mp4", listener.local_addr().unwrap());
    let (body, asked) = (Arc::new(body), Arc::new(Mutex::new(Vec::new())));
    let refusing = Arc::new(AtomicBool::new(false));
    let (told, refuses) = (asked.clone(), refusing.clone());
    thread::spawn(move || {
      for stream in listener.incoming() {
        let (Ok(mut stream), body, told, refuses) = (stream, body.clone(), told.clone(), refuses.clone()) else {
          return;
        };
        thread::spawn(move || {
          let mut request = Vec::new();
          let mut chunk = [0u8; 1024];
          while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut chunk) {
              Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
              _ => return,
            }
          }
          let request = String::from_utf8_lossy(&request).to_lowercase();
          if !request.contains("x-asked-by: lurq") || refuses.load(Ordering::SeqCst) {
            let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
          }
          let range = request
            .lines()
            .find_map(|line| line.strip_prefix("range: bytes="))
            .map(str::to_owned);
          told.lock().push(range.clone().unwrap_or_default());
          let len = body.len();
          let part = range.filter(|_| ranges).and_then(|range| {
            let (from, to) = range.split_once('-')?;
            let from: usize = from.parse().ok()?;
            let to = to.parse::<usize>().map_or(len - 1, |to| to.min(len - 1));
            Some((from, to))
          });
          let head = match part {
            Some((from, _)) if from >= len => {
              format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
              )
            }
            Some((from, to)) => format!(
              "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4; codecs=avc1\r\nContent-Range: bytes {from}-{to}/{len}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
              to + 1 - from
            ),
            None => format!(
              "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
            ),
          };
          let _ = stream.write_all(head.as_bytes());
          let sent = match part {
            Some((from, _)) if from >= len => &body[..0],
            Some((from, to)) => &body[from..=to],
            None => &body[..],
          };
          let _ = stream.write_all(sent);
        });
      }
    });
    (address, asked, refusing)
  }

  fn numbered(len: usize) -> Vec<u8> {
    (0..len).map(|at| (at % 251) as u8).collect()
  }

  /// A block small enough that a test reads many.
  const SMALL: u64 = 4096;

  fn asking(address: &str) -> Result<HttpReader, LoadError> {
    HttpReader::open(
      Agents::new(4),
      address.to_owned(),
      vec![(Arc::from("X-Asked-By"), Arc::from("lurq"))],
      SECOND,
      SMALL,
    )
  }

  /// Reads `len` bytes from `at`, fewer at the end.
  fn read(reader: &HttpReader, at: u64, len: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    let mut filled = 0;
    while filled < len {
      let read = reader.read_at(at + filled as u64, &mut buf[filled..])?;
      if read == 0 {
        break;
      }
      filled += read;
    }
    buf.truncate(filled);
    Ok(buf)
  }

  #[test]
  fn an_address_is_read_by_ranges_with_its_headers_from_any_place() {
    let body = numbered(3 * SMALL as usize + 1234);
    let (address, asked, _) = serve_file(body.clone(), true);
    let reader = asking(&address).unwrap();
    assert_eq!(asked.lock().len(), 1, "opened, it asked for its first block");
    assert_eq!(reader.len(), Some(body.len() as u64));
    assert_eq!(reader.content_type().as_deref(), Some("video/mp4"));
    // Across the end of a block, near the end, and past it.
    for at in [0, SMALL - 3, 2 * SMALL + 17, body.len() as u64 - 5] {
      let at_ = at as usize;
      assert_eq!(
        read(&reader, at, 8).unwrap(),
        &body[at_..(at_ + 8).min(body.len())],
        "at {at}"
      );
    }
    assert_eq!(reader.read_at(body.len() as u64, &mut [0u8; 4]).unwrap(), 0, "the end");
    assert!(!reader.buffered().is_empty());
    let asked = asked.lock().clone();
    let blocks: Vec<u64> = asked
      .iter()
      .map(|range| range.split_once('-').and_then(|(from, _)| from.parse().ok()).unwrap())
      .collect();
    assert!(
      blocks.iter().all(|from| from % SMALL == 0),
      "every request asks for a block from its start: {asked:?}"
    );
    assert_eq!(
      blocks.iter().filter(|from| **from == 0).count(),
      1,
      "a block that is held is not asked for again: {asked:?}"
    );
  }

  #[test]
  fn an_address_opened_without_the_header_it_needs_fails_with_what_the_server_answered() {
    let (address, ..) = serve_file(numbered(1000), true);
    let refused = HttpReader::open(Agents::new(4), address, Vec::new(), SECOND, SMALL);
    assert_eq!(refused.err(), Some(LoadError::Status(403)));
  }

  #[test]
  fn a_read_the_server_refuses_on_the_way_says_what_it_answered() {
    let body = numbered(40 * SMALL as usize);
    let (address, _, refusing) = serve_file(body, true);
    let reader = asking(&address).unwrap();
    refusing.store(true, Ordering::SeqCst);
    let error = read(&reader, 30 * SMALL, 8).unwrap_err();
    assert_eq!(LoadError::from(error), LoadError::Status(403));
  }

  #[test]
  fn a_server_that_reads_no_ranges_is_read_from_its_start_to_the_place() {
    let body = numbered(2 * SMALL as usize + 99);
    let (address, ..) = serve_file(body.clone(), false);
    let reader = asking(&address).unwrap();
    let at = 2 * SMALL as usize + 10;
    assert_eq!(read(&reader, at as u64, 6).unwrap(), &body[at..at + 6]);
    assert_eq!(reader.len(), Some(body.len() as u64));
  }
}
