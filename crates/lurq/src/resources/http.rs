//! Reading what is at an address: the part of the loader the feature `http` adds.

use std::{
  collections::VecDeque,
  io::{self, Read},
  ops::Range,
  sync::{
    Arc, Weak,
    atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
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
  let agent = agents.lend(address);
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
/// requests opened, and a request is lent the agent that asked its host last, so it goes over one of them, with
/// no new handshake.
/// As many are kept as the loader's pool is large ([`Loader::pool_size`](super::Loader::pool_size)); one given
/// back beyond that is let go, with its connections.
pub(super) struct Agents {
  /// The agents nobody has, each with the host it asked last, the one given back last at the end.
  idle: Mutex<Vec<(ureq::Agent, Box<str>)>>,
  size: AtomicUsize,
  /// How many were made, for a test that they are made once.
  made: AtomicUsize,
  /// The hosts a connection was opened to ahead ([`connect`]), and when.
  connected: Mutex<Vec<(Box<str>, Instant)>>,
  /// Each host its readers ask, while one of them lives.
  hosts: Mutex<Vec<(Box<str>, Weak<Host>)>>,
}

impl Agents {
  pub(super) fn new(size: usize) -> Arc<Self> {
    Arc::new(Self {
      idle: Mutex::new(Vec::new()),
      size: AtomicUsize::new(size),
      made: AtomicUsize::new(0),
      connected: Mutex::new(Vec::new()),
      hosts: Mutex::new(Vec::new()),
    })
  }

  /// What the readers of the host of `address` share: one for each host, while one of its readers lives.
  fn host(&self, address: &str) -> Arc<Host> {
    let name = host_of(address);
    let mut hosts = self.hosts.lock();
    hosts.retain(|(_, host)| host.strong_count() > 0);
    if let Some(host) = hosts
      .iter()
      .find(|(had, _)| **had == *name)
      .and_then(|(_, host)| host.upgrade())
    {
      return host;
    }
    let host = Arc::new(Host::default());
    hosts.push((name.into(), Arc::downgrade(&host)));
    host
  }

  pub(super) fn resize(&self, size: usize) {
    self.size.store(size, Ordering::Release);
    self.idle.lock().truncate(size);
  }

  /// How many agents were made since the loader was.
  pub(super) fn made(&self) -> usize {
    self.made.load(Ordering::Acquire)
  }

  /// An agent for a request to `address`, given back when the lent one is dropped: the one that asked the host
  /// of the address last, which holds a connection to it, or else the one given back last, or a new one where
  /// none is idle.
  pub(super) fn lend(&self, address: &str) -> Lent<'_> {
    let host = host_of(address);
    let idle = {
      let mut idle = self.idle.lock();
      let at = idle
        .iter()
        .rposition(|(_, asked)| **asked == *host)
        .or_else(|| idle.len().checked_sub(1));
      at.map(|at| idle.remove(at).0)
    };
    let agent = idle.unwrap_or_else(|| {
      self.made.fetch_add(1, Ordering::AcqRel);
      ureq::Agent::config_builder().http_status_as_error(false).build().into()
    });
    Lent {
      agents: self,
      agent: Some(agent),
      host: host.into(),
    }
  }
}

/// The host an address is at, with how it is asked: `https://example.com:8443`.
fn host_of(address: &str) -> &str {
  let after = address.find("://").map_or(0, |at| at + 3);
  let end = address[after..]
    .find(['/', '?', '#'])
    .map_or(address.len(), |at| after + at);
  &address[..end]
}

pub(super) struct Lent<'a> {
  agents: &'a Agents,
  agent: Option<ureq::Agent>,
  host: Box<str>,
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
      idle.push((agent, std::mem::take(&mut self.host)));
    }
  }
}

/// Opens a connection to the host of `address` ahead of the first request to it, on a thread of its own: it
/// asks for the head of what is at the address (`HEAD`), which is an answer with nothing in it, and the agent
/// keeps the connection for the request that follows. A host asked a moment ago is not asked again.
pub(super) fn connect(agents: &Arc<Agents>, address: String, headers: Vec<(Arc<str>, Arc<str>)>, timeout: Duration) {
  {
    let host = host_of(&address);
    let mut connected = agents.connected.lock();
    connected.retain(|(_, at)| at.elapsed() < CONNECTED_FOR);
    if connected.iter().any(|(had, _)| **had == *host) {
      return;
    }
    connected.push((host.into(), Instant::now()));
  }
  let agents = agents.clone();
  let _ = std::thread::Builder::new()
    .name("lurq-connect".to_owned())
    .spawn(move || {
      let asked = Instant::now();
      let agent = agents.lend(&address);
      let mut request = agent.head(&address);
      for (name, value) in &headers {
        request = request.header(name.as_ref(), value.as_ref());
      }
      let answered = request
        .config()
        .timeout_global(Some(timeout.min(CONNECT_WITHIN)))
        .build()
        .call();
      let ms = asked.elapsed().as_millis() as u64;
      match answered {
        Ok(response) => {
          let status = response.status().as_u16();
          // A connection goes back to its agent once its answer is read to its end.
          let _ = io::copy(&mut response.into_body().into_reader(), &mut io::sink());
          tracing::debug!(target: "lurq::resources", address = %address, status, ms, "connected ahead");
        }
        Err(error) => {
          tracing::debug!(target: "lurq::resources", address = %address, ms, "no connection ahead: {error}");
        }
      }
    });
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
/// How many requests a reader has on their way at most: one reads on in order, and another is made beside it
/// only for a read that waits for bytes the first is not bringing.
const REQUESTS: usize = 3;
/// How much of the first block is there when a reader is opened: enough to tell where the index of an MP4 is.
const OPENED_WITH: usize = 1024;
/// The most one read of a connection takes.
const CHUNK: usize = 32 * 1024;
/// How much of the start of an address is kept once its reader is dropped, and for how long.
const START_KEPT: u64 = 2 << 20;
const START_KEPT_FOR: Duration = Duration::from_secs(5 * 60);
/// How long a connection opened ahead is taken to be there, so that its host is not asked again: an agent
/// lets go of a connection nothing went over for a quarter of a minute.
const CONNECTED_FOR: Duration = Duration::from_secs(10);
/// The longest opening a connection ahead may take.
const CONNECT_WITHIN: Duration = Duration::from_secs(10);

/// The start of the addresses read last, kept once their readers are dropped: a reader of one of them starts
/// with it, and asks the network for nothing until a read goes past it. A player closed and made again a
/// moment later (a page of a pager gone and come back) opens at once.
pub(super) struct Starts {
  kept: Mutex<VecDeque<Start>>,
  /// Of how many addresses.
  most: AtomicUsize,
}

/// The start of what is at an address: its first blocks, whole and in order, as `cut` cuts them, with how long
/// all of it is and what it is, and when it was read.
#[derive(Clone)]
struct Start {
  key: u64,
  cut: Cut,
  read: Instant,
  len: Option<u64>,
  content_type: Option<Arc<str>>,
  blocks: Vec<Arc<[u8]>>,
}

impl Starts {
  pub(super) fn new(most: usize) -> Arc<Self> {
    Arc::new(Self {
      kept: Mutex::new(VecDeque::new()),
      most: AtomicUsize::new(most),
    })
  }

  pub(super) fn resize(&self, most: usize) {
    self.most.store(most, Ordering::Release);
    let mut kept = self.kept.lock();
    while kept.len() > most {
      kept.pop_front();
    }
  }

  /// Forgets the start kept under `key`: `true` where there was one.
  pub(super) fn forget(&self, key: u64) -> bool {
    let mut kept = self.kept.lock();
    let before = kept.len();
    kept.retain(|start| start.key != key);
    kept.len() != before
  }

  fn keep(&self, start: Start) {
    let mut kept = self.kept.lock();
    kept.retain(|had| had.key != start.key);
    kept.push_back(start);
    while kept.len() > self.most.load(Ordering::Acquire) {
      kept.pop_front();
    }
  }

  /// The start kept under `key`, where it was cut as `cut` cuts and is not too old.
  fn of(&self, key: u64, cut: Cut) -> Option<Start> {
    let mut kept = self.kept.lock();
    kept.retain(|start| start.read.elapsed() < START_KEPT_FOR);
    kept.iter().find(|start| start.key == key && start.cut == cut).cloned()
  }
}

/// How what is at an address is cut into the blocks it is asked for in. The first two are `first` long and each
/// one after them twice the one before, up to `most`; from there on they are all `most`. What opens a file reads
/// its start, and on a slow connection a small first block is there sooner; what plays it reads on, and larger
/// blocks are fewer requests. With `first` no smaller than `most` every block is `most` long.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cut {
  pub first: u64,
  pub most: u64,
}

impl Cut {
  /// Blocks of one size.
  #[cfg(test)]
  pub(super) fn even(size: u64) -> Self {
    Self {
      first: size,
      most: size,
    }
  }

  fn sizes(self) -> (u64, u64) {
    let most = self.most.max(1);
    (self.first.clamp(1, most), most)
  }

  /// Where the block at `index` starts, and how long it is.
  fn block(self, index: u64) -> (u64, u64) {
    let (first, most) = self.sizes();
    let (mut start, mut size) = (0, first);
    for at in 0..index {
      if size == most {
        // All of one size from here on.
        return (start + (index - at) * most, most);
      }
      start += size;
      if at > 0 {
        size = (size * 2).min(most);
      }
    }
    (start, size)
  }

  /// The block that holds the byte at `offset`.
  fn index_of(self, offset: u64) -> u64 {
    let (first, most) = self.sizes();
    let (mut index, mut start, mut size) = (0, 0, first);
    while offset >= start + size {
      if size == most {
        return index + (offset - start) / most;
      }
      start += size;
      if index > 0 {
        size = (size * 2).min(most);
      }
      index += 1;
    }
    index
  }
}

/// Where the index of an MP4 file is, when it is not at its start: the offset of its `moov` box, read from the
/// boxes at the start of the file. A file written for the network has it before its pictures (`mdat`) and
/// needs nothing more to open; one that has it after them is opened only once its end is read too, which a
/// decoder asks for after the start, one request after the other. `None` for what is not an MP4, for one whose
/// index is at its start, and where the start read does not say.
fn index_at_the_end(start: &[u8]) -> Option<u64> {
  let mut at = 0usize;
  let mut seen_type = false;
  // A few boxes at most stand before the pictures: the type, a free one, the index or the pictures.
  for _ in 0..8 {
    let header = start.get(at..at + 8)?;
    let short = u64::from(u32::from_be_bytes(header[..4].try_into().ok()?));
    let kind = &header[4..8];
    let size = match short {
      // To the end of the file: nothing is after it.
      0 => return None,
      1 => u64::from_be_bytes(start.get(at + 8..at + 16)?.try_into().ok()?),
      short => short,
    };
    match kind {
      b"ftyp" | b"styp" => seen_type = true,
      b"moov" => return None,
      // The pictures come first: the index is what follows them.
      b"mdat" if seen_type => return (at as u64).checked_add(size),
      b"free" | b"skip" | b"wide" | b"mdat" => {}
      _ if !seen_type => return None,
      _ => {}
    }
    at = usize::try_from((at as u64).checked_add(size)?).ok()?;
  }
  None
}

/// How many requests the readers of one host have on their way at once, besides those a read waits for: a host
/// gives one client only so much a second, however many connections it has.
const HOST_REQUESTS: usize = 3;
/// The most a request of a reader that is not in front asks for: it ends soon, and what is in front has the
/// connection next.
const BEHIND_MOST: u64 = 256 << 10;
/// The rank of a reader nobody ranked: in front, so that it reads as it did alone.
const IN_FRONT: u8 = 3;

/// How much a request is wanted: one a read waits for before one that brings what nobody waits for, and then
/// by its reader's rank ([`Reader::set_rank`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
  waited: bool,
  reader: u8,
}

/// What the readers of one host share: the requests on their way to it, and the readers that have one to make
/// and no room for it. A host gives one client only so much a second in all, so what one reader asks for takes
/// from the others: what is most wanted goes first, and a request that is less stands still while one that is
/// more is on its way.
#[derive(Default)]
pub(super) struct Host {
  state: Mutex<HostState>,
}

#[derive(Default)]
struct HostState {
  running: Vec<Rank>,
  waiting: Vec<(Weak<Ranges>, Rank)>,
}

impl Host {
  /// Whether a request of `rank` may go now: there is room, or a read waits for it.
  fn admit(&self, rank: Rank) -> bool {
    let mut state = self.state.lock();
    let room = state.running.len() < HOST_REQUESTS || rank.waited;
    if room {
      state.running.push(rank);
    }
    room
  }

  /// `ranges` has a request of `rank` to make once there is room.
  fn wait_for_room(&self, ranges: &Arc<Ranges>, rank: Rank) {
    let mut state = self.state.lock();
    state
      .waiting
      .retain(|(had, _)| had.strong_count() > 0 && !std::ptr::eq(had.as_ptr(), Arc::as_ptr(ranges)));
    state.waiting.push((Arc::downgrade(ranges), rank));
  }

  /// A request of `rank` ended: the reader most wanted of those that wait for room, to be roused.
  fn release(&self, rank: Rank) -> Option<Arc<Ranges>> {
    let mut state = self.state.lock();
    if let Some(at) = state.running.iter().position(|running| *running == rank) {
      state.running.swap_remove(at);
    }
    state.waiting.retain(|(ranges, _)| ranges.strong_count() > 0);
    let most = (0..state.waiting.len()).max_by_key(|at| state.waiting[*at].1)?;
    state.waiting.swap_remove(most).0.upgrade()
  }

  /// Whether a request of a higher rank than `rank` that a read waits for is on its way to this host, or waits
  /// for room.
  fn outranked(&self, rank: Rank) -> bool {
    let state = self.state.lock();
    state
      .running
      .iter()
      .chain(state.waiting.iter().map(|(_, rank)| rank))
      .any(|other| other.waited && *other > rank)
  }
}

/// What is at an address, read by ranges as it is asked for, with its headers on every request: what a decoder
/// reads as it plays.
///
/// A read is given its bytes as they come, and does not wait for the block they are in to be whole. Its
/// requests go one after the other over one connection, each from where the one before ended: the block a read
/// waits for first, then, where it reads ahead, what follows the last read, and that only while no read waits.
/// A second request is made beside the first only for a read that waits for bytes the first is not bringing (a
/// decoder sent to another place), and the first, where nobody waits for what it brings, stands still until
/// that read has its bytes: a host that gives one client so much a second in all gives it to what is waited
/// for.
pub(super) struct HttpReader(Arc<Ranges>);

struct Ranges {
  address: String,
  headers: Vec<(Arc<str>, Arc<str>)>,
  timeout: Duration,
  cut: Cut,
  /// The agent of all its requests: the connection one of them ended on is the one the next goes over.
  agent: ureq::Agent,
  blocks: Mutex<Blocks>,
  /// Bytes came, a request ended, or a read began or ended its wait.
  came: Condvar,
  /// Whether what follows a read is fetched before it is asked for ([`Reader::read_ahead`]), and the block
  /// read last.
  eager: AtomicBool,
  last: AtomicU64,
  /// How far it fetches before it is asked for, whether or not it reads ahead ([`Reader::read_ahead_to`]).
  ahead_to: AtomicU64,
  /// The host it asks, which it shares with the other readers of that host, and its rank among them.
  host: Arc<Host>,
  rank: AtomicU8,
  /// Where its start is kept once it is dropped, under what, and when that start was read.
  starts: Arc<Starts>,
  key: u64,
  since: Instant,
}

#[derive(Default)]
struct Blocks {
  /// How long it is, once an answer said.
  len: Option<u64>,
  content_type: Option<Arc<str>>,
  /// An answer came: its length and its type are what the server said.
  answered: bool,
  /// The blocks there are, whole or as far as they have come, the one read last at the back.
  held: VecDeque<Held>,
  /// The blocks a request is bringing.
  coming: Vec<u64>,
  /// The block each read that waits is waiting for, and where in it the read is.
  waited: Vec<u64>,
  waited_at: Vec<(u64, u64)>,
  /// The blocks whose request failed, why, and whether it was a request no read had asked for: told to the
  /// reads that waited for them, and not brought ahead again until a read asks.
  failed: Vec<(u64, LoadError, bool)>,
  /// The blocks brought ahead whatever is read: the index of an MP4 that has it at its end.
  wanted: Vec<u64>,
  /// How many requests are on their way, and whether one of them brings what no read had asked for.
  working: usize,
  ahead: bool,
  /// A read was made: until then nothing is brought ahead of what opening it asks for.
  began: bool,
  /// The reader was dropped: nothing more is asked for.
  closed: bool,
}

/// A block: its index, and its bytes from `from` bytes into it. A block is brought from its start, but for one a
/// read waits for that was sent into it (a decoder sent to another place): that is brought from the read's
/// place, and what is before it is not there.
struct Held {
  index: u64,
  from: u64,
  bytes: Bytes,
}

enum Bytes {
  /// As far as they have come.
  Coming(Vec<u8>),
  /// All of them.
  Whole(Arc<[u8]>),
}

impl Bytes {
  fn len(&self) -> usize {
    match self {
      Self::Coming(bytes) => bytes.len(),
      Self::Whole(bytes) => bytes.len(),
    }
  }
}

impl Blocks {
  fn held_at(&self, index: u64) -> Option<usize> {
    self.held.iter().position(|held| held.index == index)
  }

  /// Whether all of the block at `index` is there, or all of it from where it was brought.
  fn whole(&self, index: u64) -> bool {
    self
      .held
      .iter()
      .any(|held| held.index == index && matches!(held.bytes, Bytes::Whole(_)))
  }

  /// How far into the block at `index` its bytes are there.
  fn filled(&self, index: u64) -> u64 {
    self
      .held_at(index)
      .map_or(0, |at| self.held[at].from + self.held[at].bytes.len() as u64)
  }

  /// Where in the block at `index` the reads that wait for it are, the first of them: where a request for it
  /// starts.
  fn waited_from(&self, index: u64) -> u64 {
    self
      .waited_at
      .iter()
      .filter(|(waited, _)| *waited == index)
      .map(|(_, within)| *within)
      .min()
      .unwrap_or(0)
  }

  /// Why the request for the block at `index` failed, and whether no read had asked for it.
  fn failed_of(&self, index: u64) -> Option<(LoadError, bool)> {
    self
      .failed
      .iter()
      .find(|(failed, ..)| *failed == index)
      .map(|(_, error, ahead)| (error.clone(), *ahead))
  }

  /// One read fewer waits for the block at `index`.
  fn waits_no_more(&mut self, index: u64) {
    if let Some(at) = self.waited.iter().position(|waited| *waited == index) {
      self.waited.swap_remove(at);
    }
    if let Some(at) = self.waited_at.iter().position(|(waited, _)| *waited == index) {
      self.waited_at.swap_remove(at);
    }
  }
}

/// What a reader is opened of, and how.
pub(super) struct Asked {
  /// What its start is kept under once it is dropped.
  pub key: u64,
  pub address: String,
  pub headers: Vec<(Arc<str>, Arc<str>)>,
  /// How long each of its requests may take.
  pub timeout: Duration,
  pub cut: Cut,
  /// Whether it reads ahead, where the one that opens it knows: `Some(true)` from its first request on, which
  /// then asks for what is read ahead with the first block; `Some(false)` not until it is told to. `None` as a
  /// reader does by itself: its first request asks for the first block, and it reads ahead once it is read.
  pub read_ahead: Option<bool>,
}

/// What one request brings: the blocks from `first` to `last`, and whether no read had asked for them. Where
/// nothing of `first` is there, it is brought from `skip` bytes into it.
struct Run {
  first: u64,
  last: u64,
  ahead: bool,
  skip: u64,
}

impl Run {
  fn holds(&self, index: u64) -> bool {
    (self.first..=self.last).contains(&index)
  }
}

impl HttpReader {
  /// A reader of what `asked` says. It asks for the first block and returns once the answer has come and the
  /// block has begun to: the status, the length and the type are known then, and a server that refuses it
  /// fails here. The rest of the block is read on as it comes.
  ///
  /// Where `starts` keeps the start of what is under its key, read by a reader dropped a moment ago, that is
  /// what it starts with, and nothing is asked for until a read goes past it.
  ///
  /// Where the first block says the file is an MP4 with its index at its end, the blocks of the index are
  /// asked for at once, beside the first block: a decoder that opens it finds them there, or on their way.
  pub(super) fn open(agents: &Agents, starts: Arc<Starts>, asked: Asked) -> Result<Self, LoadError> {
    let Asked {
      key,
      address,
      headers,
      timeout,
      cut,
      read_ahead,
    } = asked;
    let agent = (*agents.lend(&address)).clone();
    let host = agents.host(&address);
    let kept = starts.of(key, cut);
    let mut blocks = Blocks {
      began: read_ahead == Some(true),
      ..Blocks::default()
    };
    let since = kept.as_ref().map_or_else(Instant::now, |start| start.read);
    if let Some(start) = kept {
      tracing::debug!(target: "lurq::resources", address = %address, blocks = start.blocks.len(), "its start was kept");
      blocks.len = start.len;
      blocks.content_type = start.content_type;
      blocks.answered = true;
      blocks.held = start
        .blocks
        .into_iter()
        .enumerate()
        .map(|(index, bytes)| Held {
          index: index as u64,
          from: 0,
          bytes: Bytes::Whole(bytes),
        })
        .collect();
    }
    let ranges = Arc::new(Ranges {
      address,
      headers,
      timeout,
      cut,
      agent,
      blocks: Mutex::new(blocks),
      came: Condvar::new(),
      eager: AtomicBool::new(read_ahead != Some(false)),
      last: AtomicU64::new(0),
      ahead_to: AtomicU64::new(0),
      host,
      rank: AtomicU8::new(IN_FRONT),
      starts,
      key,
      since,
    });
    let reader = Self(ranges.clone());
    let mut blocks = ranges.blocks.lock();
    blocks.waited.push(0);
    blocks.waited_at.push((0, 0));
    let start = loop {
      let there = blocks.held_at(0).map(|at| &blocks.held[at].bytes);
      match there {
        Some(Bytes::Whole(bytes)) => break bytes.to_vec(),
        Some(Bytes::Coming(bytes)) if blocks.answered && bytes.len() >= OPENED_WITH => break bytes.clone(),
        _ => {}
      }
      if blocks.answered && blocks.len == Some(0) {
        break Vec::new();
      }
      if !blocks.coming.contains(&0) {
        if let Some((error, _)) = blocks.failed_of(0) {
          return Err(error);
        }
        ranges.rouse(&mut blocks);
        if !blocks.coming.contains(&0) {
          return Err(blocks.failed_of(0).map_or(LoadError::Cancelled, |(error, _)| error));
        }
      }
      ranges.came.wait(&mut blocks);
    };
    blocks.waits_no_more(0);
    if let Some(index) = index_at_the_end(&start)
      && blocks.len.is_some_and(|len| index < len)
    {
      tracing::debug!(target: "lurq::resources", address = %ranges.address, at = index, "the index is at the end: read ahead");
      let first = ranges.cut.index_of(index);
      blocks.wanted = (first..=first + READ_AHEAD).collect();
    }
    ranges.rouse(&mut blocks);
    drop(blocks);
    ranges.came.notify_all();
    Ok(reader)
  }
}

impl Drop for HttpReader {
  /// Its requests end where they are, and its start is kept for the reader that opens the address next.
  fn drop(&mut self) {
    let ranges = &self.0;
    let mut blocks = ranges.blocks.lock();
    blocks.closed = true;
    let mut start = Vec::new();
    let mut bytes = 0;
    for index in 0.. {
      let Some(Held {
        from: 0,
        bytes: Bytes::Whole(block),
        ..
      }) = blocks.held_at(index).map(|at| &blocks.held[at])
      else {
        break;
      };
      bytes += block.len() as u64;
      if bytes > START_KEPT {
        break;
      }
      start.push(block.clone());
    }
    let (len, content_type) = (blocks.len, blocks.content_type.clone());
    drop(blocks);
    ranges.came.notify_all();
    if !start.is_empty() {
      ranges.starts.keep(Start {
        key: ranges.key,
        cut: ranges.cut,
        read: ranges.since,
        len,
        content_type,
        blocks: start,
      });
    }
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
    if buf.is_empty() {
      return Ok(0);
    }
    let ranges = &self.0;
    let index = ranges.cut.index_of(offset);
    let within = (offset - ranges.cut.block(index).0) as usize;
    let mut blocks = ranges.blocks.lock();
    blocks.began = true;
    let mut waited = false;
    loop {
      if blocks.len.is_some_and(|len| offset >= len) {
        return Ok(0);
      }
      if let Some(at) = blocks.held_at(index) {
        let from = blocks.held[at].from as usize;
        if within < from {
          // Before where the block was brought from: once nothing brings it, it is let go and brought again from
          // this read's place.
          if !blocks.coming.contains(&index) {
            blocks.held.remove(at);
            continue;
          }
        } else if from + blocks.held[at].bytes.len() > within {
          let within = within - from;
          // The block read last is the last to be let go.
          let held = blocks.held.remove(at).expect("the block just found");
          let count = (held.bytes.len() - within).min(buf.len());
          let whole = match &held.bytes {
            Bytes::Whole(bytes) => Some(bytes.clone()),
            Bytes::Coming(bytes) => {
              buf[..count].copy_from_slice(&bytes[within..within + count]);
              None
            }
          };
          blocks.held.push_back(held);
          ranges.last.store(index, Ordering::Relaxed);
          // What follows it is asked for, where it reads ahead and nothing is on its way.
          ranges.rouse(&mut blocks);
          drop(blocks);
          if let Some(bytes) = whole {
            buf[..count].copy_from_slice(&bytes[within..within + count]);
          }
          return Ok(count);
        } else if matches!(blocks.held[at].bytes, Bytes::Whole(_)) {
          // A block shorter than it was cut: the end of all of it.
          return Ok(0);
        }
      }
      if !blocks.coming.contains(&index) {
        match blocks.failed_of(index) {
          // The request made for a read that waited for it failed: that is what the read is told.
          Some((error, false)) if waited => return Err(io::Error::other(error)),
          // Otherwise the read asks again: one that did not wait for the request that failed, and one that
          // waited for a request no read had asked for.
          Some(_) => blocks.failed.retain(|(failed, ..)| *failed != index),
          None => {}
        }
      }
      blocks.waited.push(index);
      blocks.waited_at.push((index, within as u64));
      ranges.rouse(&mut blocks);
      if !blocks.coming.contains(&index)
        && let Some((error, _)) = blocks.failed_of(index)
      {
        // No thread to ask on.
        blocks.waits_no_more(index);
        return Err(io::Error::other(error));
      }
      // A request that brings something else hears that a read waits.
      ranges.came.notify_all();
      ranges.came.wait(&mut blocks);
      blocks.waits_no_more(index);
      waited = true;
    }
  }

  fn buffered(&self) -> Vec<Range<u64>> {
    let cut = self.0.cut;
    let mut held: Vec<Range<u64>> = self
      .0
      .blocks
      .lock()
      .held
      .iter()
      .filter(|held| held.bytes.len() > 0)
      .map(|held| {
        let start = cut.block(held.index).0 + held.from;
        start..start + held.bytes.len() as u64
      })
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

  fn read_ahead(&self, ahead: bool) {
    if self.0.eager.swap(ahead, Ordering::Relaxed) == ahead || !ahead {
      return;
    }
    // What follows the last read is asked for now: what starts to play does not wait for its second block.
    let mut blocks = self.0.blocks.lock();
    self.0.rouse(&mut blocks);
  }

  fn set_rank(&self, rank: u8) {
    if self.0.rank.swap(rank, Ordering::Relaxed) >= rank {
      return;
    }
    // Ranked higher, it may have room now that it had not.
    let mut blocks = self.0.blocks.lock();
    self.0.rouse(&mut blocks);
  }

  fn read_ahead_to(&self, end: u64) {
    if self.0.ahead_to.swap(end, Ordering::Relaxed) >= end {
      return;
    }
    let mut blocks = self.0.blocks.lock();
    self.0.rouse(&mut blocks);
  }
}

impl Ranges {
  /// Whether the block at `index` is to be asked for: not all there, not on its way, and not past the end.
  fn missing(&self, blocks: &Blocks, index: u64) -> bool {
    !blocks.whole(index)
      && !blocks.coming.contains(&index)
      && blocks.len.is_none_or(|len| self.cut.block(index).0 < len)
  }

  /// What the next request brings, taken: the block a read waits for; where none does, the index of an MP4
  /// that has it at its end; and where that is there and `in_turn`, the first block after the last read that is
  /// not there, where it reads ahead. The blocks after it go in the same request while they are within what is
  /// read ahead and the request asks for no more than the largest block: the small first blocks are one
  /// request to a reader that reads ahead, and one each to one that does not.
  fn job(&self, blocks: &mut Blocks, in_turn: bool) -> Option<Run> {
    if blocks.closed {
      return None;
    }
    let eager = self.eager.load(Ordering::Relaxed);
    let last = self.last.load(Ordering::Relaxed);
    // The block that holds the last byte it is told to fetch ahead.
    let until = match self.ahead_to.load(Ordering::Relaxed) {
      0 => None,
      end => Some(self.cut.index_of(end - 1)),
    };
    // A block whose request failed is asked for again by the next read of it, and not before.
    let fresh = |index: &u64| self.missing(blocks, *index) && blocks.failed_of(*index).is_none();
    let waited = blocks.waited.iter().copied().find(fresh);
    let (first, ahead) = match waited {
      Some(index) => (index, false),
      // What nobody waits for is brought once nothing is waited for, one request at a time and in order.
      None if !blocks.ahead && blocks.waited.iter().all(|index| blocks.whole(*index)) => {
        let wanted = blocks.wanted.iter().copied().find(fresh);
        let next = || (1..=READ_AHEAD).map(|ahead| last + ahead).find(fresh);
        // As far as it is told to fetch ahead, past what reading ahead brings.
        let further = || until.and_then(|until| (last + 1..=until).find(fresh));
        let in_order = || {
          (in_turn && blocks.began)
            .then(|| eager.then(next).flatten().or_else(further))
            .flatten()
        };
        (wanted.or_else(in_order)?, true)
      }
      None => return None,
    };
    let reach = last.max(first) + READ_AHEAD;
    let (_, most) = self.cut.sizes();
    // A reader behind others of its host asks for little at a time: its request ends soon, and the host's
    // connection goes to what is wanted more.
    let most = if ahead && self.rank.load(Ordering::Relaxed) < IN_FRONT {
      most.min(BEHIND_MOST)
    } else {
      most
    };
    // A read sent into a block waits for its place, not for the start of the block: the block is brought from
    // there. Measured on a Mac over a video of a slow host, a seek waited for up to 852 KB before its place,
    // 1.3 s of the host's speed, before the bytes it was sent to.
    let skip = if ahead || blocks.held_at(first).is_some() {
      0
    } else {
      blocks.waited_from(first)
    };
    let mut run = Run {
      first,
      last: first,
      ahead,
      skip,
    };
    let mut bytes = self.cut.block(first).1;
    loop {
      let next = run.last + 1;
      let size = self.cut.block(next).1;
      let asked_for = (eager && blocks.began && next <= reach)
        || blocks.wanted.contains(&next)
        || (blocks.began && until.is_some_and(|until| next <= until));
      if !asked_for
        || bytes + size > most
        || !self.missing(blocks, next)
        || blocks.held_at(next).is_some()
        || blocks.failed_of(next).is_some()
      {
        break;
      }
      bytes += size;
      run.last = next;
    }
    blocks.coming.extend(run.first..=run.last);
    blocks.working += 1;
    blocks.ahead |= run.ahead;
    Some(run)
  }

  /// Starts the requests there is something for: one, which goes on to what is next by itself, and another
  /// beside it only for a read that waits for what no request is bringing, and for an index at the end.
  fn rouse(self: &Arc<Self>, blocks: &mut Blocks) {
    while blocks.working < REQUESTS {
      let idle = blocks.working == 0;
      let Some(run) = self.job(blocks, idle) else {
        return;
      };
      let rank = self.rank_of(&run);
      // The host has room for it, or it waits for room, and is roused when a request to the host ends.
      if !self.host.admit(rank) {
        self.untake(blocks, &run);
        self.host.wait_for_room(self, rank);
        return;
      }
      let (ranges, first, last) = (self.clone(), run.first, run.last);
      let spawned = std::thread::Builder::new()
        .name("lurq-read".to_owned())
        .spawn(move || ranges.work(run, rank));
      if let Err(error) = spawned {
        self.untake(
          blocks,
          &Run {
            first,
            last,
            ahead: !rank.waited,
            skip: 0,
          },
        );
        blocks
          .failed
          .extend((first..=last).map(|index| (index, LoadError::Io(error.to_string()), false)));
        let next = self.host.release(rank);
        Self::rouse_other(next, self);
        return;
      }
    }
  }

  /// The rank of a request that brings `run`.
  fn rank_of(&self, run: &Run) -> Rank {
    Rank {
      waited: !run.ahead,
      reader: self.rank.load(Ordering::Relaxed),
    }
  }

  /// Gives back what `run` took of `blocks`: it is not asked for after all.
  fn untake(&self, blocks: &mut Blocks, run: &Run) {
    blocks.coming.retain(|index| !run.holds(*index));
    blocks.working -= 1;
    blocks.ahead &= !run.ahead;
  }

  /// Rouses `next`, a reader the host has room for now, unless it is `this`, which the caller rouses itself.
  fn rouse_other(next: Option<Arc<Self>>, this: &Arc<Self>) {
    if let Some(next) = next
      && !Arc::ptr_eq(&next, this)
    {
      // Its own blocks are taken on another turn: this reader's are held here.
      std::thread::spawn(move || {
        let mut blocks = next.blocks.lock();
        next.rouse(&mut blocks);
      });
    }
  }

  /// Brings `run`, and then what is next, one request after the other, until there is nothing to bring.
  fn work(self: Arc<Self>, mut run: Run, mut rank: Rank) {
    loop {
      let brought = self.bring(&run);
      let mut blocks = self.blocks.lock();
      blocks.coming.retain(|index| !run.holds(*index));
      blocks.working -= 1;
      blocks.ahead &= !run.ahead;
      let went_well = brought.is_ok();
      if let Err(error) = brought {
        tracing::debug!(target: "lurq::resources", address = %self.address, "a range failed: {error}");
        for index in run.first..=run.last {
          if !blocks.whole(index) {
            blocks.failed.push((index, error.clone(), run.ahead));
          }
        }
      }
      // The request ended: the host has room for the reader most wanted of those that wait for it, this one
      // among them.
      // What it was let go with, whatever its reader's rank is now.
      let roused = self.host.release(rank);
      // After a request that failed only what a read waits for is asked for: a server that is down is not asked
      // again and again for what nobody waits for.
      let mut next = self.job(&mut blocks, went_well);
      if let Some(taken) = &next {
        rank = self.rank_of(taken);
        if !self.host.admit(rank) {
          self.untake(&mut blocks, taken);
          self.host.wait_for_room(&self, rank);
          next = None;
        }
      }
      drop(blocks);
      self.came.notify_all();
      Self::rouse_other(roused, &self);
      match next {
        Some(next) => run = next,
        None => return,
      }
    }
  }

  /// Whether a request reads on. Not once the reader is dropped. One that brings what no read waits for stands
  /// still while a read waits for what another request brings, for as long as half of its time: the server
  /// sends it no more, and what it gives this client goes to the bytes that are waited for.
  fn goes_on(&self, run: &Run, asked: Instant) -> bool {
    let mut blocks = self.blocks.lock();
    loop {
      if blocks.closed {
        return false;
      }
      // A read of this reader waits for what another of its requests brings, or a request of a higher rank to
      // the same host is on its way, or waits for room.
      let waits_here = blocks
        .waited
        .iter()
        .any(|index| blocks.coming.contains(index) && !blocks.whole(*index));
      let gives_way = !blocks.waited.iter().any(|index| run.holds(*index))
        && (waits_here || self.host.outranked(self.rank_of(run)))
        && asked.elapsed() < self.timeout / 2;
      if !gives_way {
        return true;
      }
      self.came.wait_for(&mut blocks, Duration::from_millis(20));
    }
  }

  /// Keeps `bytes`, which are at `at`, in the blocks they are of, and wakes the reads that wait for them.
  fn put(&self, mut at: u64, mut bytes: &[u8]) {
    let mut blocks = self.blocks.lock();
    while !bytes.is_empty() {
      let index = self.cut.index_of(at);
      let (start, size) = self.cut.block(index);
      let place = match blocks.held_at(index) {
        Some(place) => place,
        None => {
          // The start is kept whatever was read since, as far as the largest block is long (the first
          // megabyte): a file's start says what it is, and a media that goes on from its start at its end, or
          // is sent there, starts from it at once. Nor is a block let go that is on its way or waited for.
          let (_, most) = self.cut.sizes();
          if blocks.held.len() >= HELD_BLOCKS
            && let Some(oldest) = blocks.held.iter().position(|held| {
              self.cut.block(held.index).0 >= most
                && !blocks.coming.contains(&held.index)
                && !blocks.waited.contains(&held.index)
            })
          {
            blocks.held.remove(oldest);
          }
          blocks.held.push_back(Held {
            index,
            from: at - start,
            bytes: Bytes::Coming(Vec::with_capacity((start + size - at) as usize)),
          });
          blocks.held.len() - 1
        }
      };
      let room = (start + size - at) as usize;
      let (now, rest) = bytes.split_at(bytes.len().min(room));
      let len = blocks.len;
      let held = &mut blocks.held[place];
      let from = held.from;
      if let Bytes::Coming(there) = &mut held.bytes {
        // From where it ends, and nothing twice: a block has one request at a time.
        if start + from + there.len() as u64 == at {
          there.extend_from_slice(now);
        }
        let end = start + from + there.len() as u64;
        if end >= start + size || len.is_some_and(|len| end >= len) {
          held.bytes = Bytes::Whole(Arc::from(std::mem::take(there)));
        }
      }
      at += now.len() as u64;
      bytes = rest;
    }
    drop(blocks);
    self.came.notify_all();
  }

  /// All of it ends at `at`: an answer ended there, or said there is nothing from there on.
  fn ends_at(&self, at: u64) {
    let mut blocks = self.blocks.lock();
    let len = blocks.len.map_or(at, |len| len.min(at));
    blocks.len = Some(len);
    blocks.answered = true;
    for held in &mut blocks.held {
      let (start, _) = self.cut.block(held.index);
      let from = held.from;
      if let Bytes::Coming(there) = &mut held.bytes
        && start + from + there.len() as u64 >= len
      {
        held.bytes = Bytes::Whole(Arc::from(std::mem::take(there)));
      }
    }
    drop(blocks);
    self.came.notify_all();
  }

  /// Asks for the blocks of `run`, from where what is there of them ends, and keeps what comes as it comes.
  fn bring(&self, run: &Run) -> Result<(), LoadError> {
    let end = {
      let (start, size) = self.cut.block(run.last);
      start + size
    };
    loop {
      let from = {
        let blocks = self.blocks.lock();
        if blocks.closed {
          return Ok(());
        }
        // The first block of a run may be there in part, from a request that ended on its way.
        let mut index = run.first;
        while index < run.last && blocks.whole(index) {
          index += 1;
        }
        let skip = if index == run.first && blocks.held_at(index).is_none() {
          run.skip
        } else {
          0
        };
        let at = self.cut.block(index).0 + blocks.filled(index).max(skip);
        if at >= end || blocks.whole(run.last) || blocks.len.is_some_and(|len| at >= len) {
          return Ok(());
        }
        at
      };
      let range = format!("bytes={from}-{}", end - 1);
      let asked = Instant::now();
      let response = match ask(
        &self.agent,
        &self.address,
        &self.headers,
        Some(&range),
        self.timeout,
        &mut |_, _| !self.blocks.lock().closed,
      ) {
        Ok(response) => response,
        // Past the end, where its length was not known yet.
        Err(LoadError::Status(416)) => {
          self.ends_at(from);
          return Ok(());
        }
        Err(error) => return Err(error),
      };
      let ranged = response.status().as_u16() == 206;
      let header = |name: &str| response.headers().get(name).and_then(|value| value.to_str().ok());
      let said = if ranged {
        total_of(header("content-range"))
      } else {
        response.body().content_length()
      };
      let content_type =
        header("content-type").map(|kind| Arc::<str>::from(kind.split(';').next().unwrap_or(kind).trim()));
      {
        let mut blocks = self.blocks.lock();
        blocks.len = said.or(blocks.len);
        blocks.content_type = content_type.or(blocks.content_type.take());
        blocks.answered = true;
      }
      self.came.notify_all();
      let mut body = response.into_body().into_reader();
      if !ranged {
        // A server that does not read ranges answers with all of it, from its start: what is before the place
        // is read past, once for each request.
        io::copy(&mut (&mut body).take(from), &mut io::sink()).map_err(read_failed)?;
      }
      let mut at = from;
      let mut chunk = vec![0u8; CHUNK];
      while at < end {
        if !self.goes_on(run, asked) {
          return Ok(());
        }
        let most = (end - at).min(CHUNK as u64) as usize;
        let read = body.read(&mut chunk[..most]).map_err(read_failed)?;
        if read == 0 {
          break;
        }
        self.put(at, &chunk[..read]);
        at += read as u64;
      }
      // Each request with what it asked for and how long it took: what a slow start is made of.
      tracing::debug!(
        target: "lurq::resources",
        address = %self.address,
        range = %range,
        bytes = at - from,
        ms = asked.elapsed().as_millis() as u64,
        "range"
      );
      if at >= end {
        // A connection goes back to its agent once its answer is read to its end, and the part asked for is
        // the whole answer: what is left is nothing, read to see the end.
        if ranged {
          io::copy(&mut body, &mut io::sink()).map_err(read_failed)?;
        }
        return Ok(());
      }
      if !ranged || at == from {
        // The answer ended before what was asked for: all of it ends there.
        self.ends_at(at);
        return Ok(());
      }
      // A server may answer a range with less of it than was asked: the rest is asked for again.
    }
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
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
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

  /// A host on this machine that gives one client so many bytes a second in all, however many connections the
  /// client has, takes a pause over each new connection, as a handshake does, and keeps a connection for the
  /// request after. It serves `body` by the ranges it is asked for.
  struct Capped {
    address: String,
    /// How many connections were opened to it.
    connections: Arc<AtomicUsize>,
    /// What it was asked for, in the order the requests came: a range, or `HEAD`.
    asked: Arc<Mutex<Vec<String>>>,
  }

  fn serve_capped(body: Vec<u8>, rate: u64, pause: Duration) -> Capped {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/clip.mp4", listener.local_addr().unwrap());
    let host = Capped {
      address,
      connections: Arc::default(),
      asked: Arc::default(),
    };
    let (body, connections, asked) = (Arc::new(body), host.connections.clone(), host.asked.clone());
    // When the bytes sent so far are all through: the next ones go after that, whichever connection sends them.
    let through = Arc::new(Mutex::new(Instant::now()));
    thread::spawn(move || {
      for stream in listener.incoming() {
        let (Ok(mut stream), body, asked, through) = (stream, body.clone(), asked.clone(), through.clone()) else {
          return;
        };
        connections.fetch_add(1, Ordering::SeqCst);
        thread::spawn(move || {
          thread::sleep(pause);
          loop {
            let mut request = Vec::new();
            let mut chunk = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
              match stream.read(&mut chunk) {
                Ok(read) if read > 0 => request.extend_from_slice(&chunk[..read]),
                _ => return,
              }
            }
            let request = String::from_utf8_lossy(&request).to_lowercase();
            let len = body.len();
            if request.starts_with("head ") {
              asked.lock().push("HEAD".to_owned());
              let head = format!("HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {len}\r\n\r\n");
              if stream.write_all(head.as_bytes()).is_err() {
                return;
              }
              continue;
            }
            let range = request
              .lines()
              .find_map(|line| line.strip_prefix("range: bytes="))
              .unwrap_or("0-")
              .to_owned();
            asked.lock().push(range.clone());
            let (from, to) = range.split_once('-').unwrap();
            let from: usize = from.parse().unwrap();
            let to = to.parse::<usize>().map_or(len - 1, |to| to.min(len - 1));
            if from >= len {
              let head = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\n\r\n"
              );
              if stream.write_all(head.as_bytes()).is_err() {
                return;
              }
              continue;
            }
            let head = format!(
              "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nContent-Range: bytes {from}-{to}/{len}\r\nContent-Length: {}\r\n\r\n",
              to + 1 - from
            );
            if stream.write_all(head.as_bytes()).is_err() {
              return;
            }
            for piece in body[from..=to].chunks(2048) {
              let at = {
                let mut through = through.lock();
                *through = (*through).max(Instant::now()) + Duration::from_secs_f64(piece.len() as f64 / rate as f64);
                *through
              };
              thread::sleep(at.saturating_duration_since(Instant::now()));
              if stream.write_all(piece).is_err() {
                return;
              }
            }
          }
        });
      }
    });
    host
  }

  fn numbered(len: usize) -> Vec<u8> {
    (0..len).map(|at| (at % 251) as u8).collect()
  }

  /// A block small enough that a test reads many.
  const SMALL: u64 = 4096;

  fn asked(address: &str, headers: Vec<(Arc<str>, Arc<str>)>, cut: Cut) -> Asked {
    Asked {
      key: 0,
      address: address.to_owned(),
      headers,
      timeout: SECOND,
      cut,
      read_ahead: None,
    }
  }

  /// A reader of what is at `address`, with agents of its own and no start kept.
  fn opening(address: &str, headers: Vec<(Arc<str>, Arc<str>)>, cut: Cut) -> Result<HttpReader, LoadError> {
    HttpReader::open(&Agents::new(4), Starts::new(0), asked(address, headers, cut))
  }

  fn asked_by_lurq() -> Vec<(Arc<str>, Arc<str>)> {
    vec![(Arc::from("X-Asked-By"), Arc::from("lurq"))]
  }

  fn asking(address: &str) -> Result<HttpReader, LoadError> {
    opening(address, asked_by_lurq(), Cut::even(SMALL))
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
    // A read sent into a block asks for it from its place: what is before that in the block is not waited for.
    let sent_to = [2 * SMALL + 17, body.len() as u64 - 5];
    assert!(
      blocks.iter().all(|from| from % SMALL == 0 || sent_to.contains(from)),
      "every request asks for a block from its start, or from where a read was sent into it: {asked:?}"
    );
    assert_eq!(
      blocks.iter().filter(|from| **from == 0).count(),
      1,
      "a block that is held is not asked for again: {asked:?}"
    );
  }

  #[test]
  fn the_first_blocks_are_small_and_the_ones_after_them_grow_to_the_largest() {
    let cut = Cut { first: 256, most: 1024 };
    // 256, 256, 512, and 1024 from there on.
    let blocks: Vec<(u64, u64)> = (0..6).map(|index| cut.block(index)).collect();
    assert_eq!(
      blocks,
      [
        (0, 256),
        (256, 256),
        (512, 512),
        (1024, 1024),
        (2048, 1024),
        (3072, 1024)
      ]
    );
    for (index, (start, size)) in blocks.iter().enumerate() {
      for offset in [*start, start + size / 2, start + size - 1] {
        assert_eq!(cut.index_of(offset), index as u64, "the byte at {offset}");
      }
    }
    assert_eq!(
      cut.index_of(1024 * 100 + 5),
      3 + 99,
      "far in, where every block is the largest"
    );
    assert_eq!(cut.block(3 + 99), (1024 * 100, 1024));

    // Of one size, as before.
    let even = Cut::even(4096);
    assert_eq!((even.block(0), even.block(3)), ((0, 4096), (12288, 4096)));
    assert_eq!(even.index_of(12288 + 17), 3);
    // A first block larger than the largest is the largest.
    let wide = Cut {
      first: 4096,
      most: 1024,
    };
    assert_eq!((wide.block(0), wide.block(2)), ((0, 1024), (2048, 1024)));
  }

  #[test]
  fn an_address_cut_into_growing_blocks_is_read_whole_and_asks_for_its_start_first_in_a_small_block() {
    let body = numbered(5000);
    let (address, asked, _) = serve_file(body.clone(), true);
    let cut = Cut { first: 256, most: 1024 };
    let reader = opening(&address, asked_by_lurq(), cut).unwrap();
    assert_eq!(*asked.lock(), ["0-255"], "its start, in the small first block");
    assert_eq!(read(&reader, 0, body.len()).unwrap(), body);
    // Across the blocks of different sizes.
    for at in [250, 500, 1020, 2040, 4990] {
      assert_eq!(
        read(&reader, at, 16).unwrap(),
        &body[at as usize..(at as usize + 16).min(body.len())],
        "at {at}"
      );
    }
    // Read on, the two blocks after the first are one request, as long as the largest block.
    let asked = asked.lock().clone();
    for range in ["256-1023", "1024-2047", "2048-3071"] {
      assert!(asked.iter().any(|asked| asked == range), "{range} of {asked:?}");
    }
  }

  /// The start of an MP4 file: its type, then `boxes`, each a name and how long it says it is.
  fn mp4_start(boxes: &[(&[u8; 4], u32)]) -> Vec<u8> {
    let mut start = Vec::new();
    start.extend_from_slice(&24u32.to_be_bytes());
    start.extend_from_slice(b"ftypisom");
    start.extend_from_slice(&[0; 12]);
    for (name, size) in boxes {
      start.extend_from_slice(&size.to_be_bytes());
      start.extend_from_slice(*name);
    }
    start
  }

  #[test]
  fn where_the_index_of_an_mp4_is_after_its_pictures_is_read_from_its_start() {
    // Written for the network: the index first.
    assert_eq!(index_at_the_end(&mp4_start(&[(b"moov", 900), (b"mdat", 50_000)])), None);
    // The pictures first: the index is what follows them.
    assert_eq!(index_at_the_end(&mp4_start(&[(b"mdat", 50_000)])), Some(24 + 50_000));
    let mut after_free = mp4_start(&[(b"free", 8)]);
    after_free.extend_from_slice(&50_000u32.to_be_bytes());
    after_free.extend_from_slice(b"mdat");
    assert_eq!(index_at_the_end(&after_free), Some(24 + 8 + 50_000));
    // A long box says its length in the eight bytes after its name.
    let mut long = mp4_start(&[(b"mdat", 1)]);
    long.extend_from_slice(&(6_000_000_000u64).to_be_bytes());
    assert_eq!(index_at_the_end(&long), Some(24 + 6_000_000_000));
    // Not an MP4, and a start too short to say.
    assert_eq!(index_at_the_end(b"GIF89a, and then what a picture holds"), None);
    assert_eq!(index_at_the_end(&mp4_start(&[])[..20]), None);
  }

  #[test]
  fn an_mp4_with_its_index_at_its_end_has_its_end_asked_for_as_it_opens() {
    // Its pictures take the file to 9000, and its index is the 1000 bytes after them.
    let mut body = mp4_start(&[(b"mdat", 9000 - 24)]);
    body.resize(10_000, 7);
    let (address, asked, _) = serve_file(body, true);
    let cut = Cut { first: 256, most: 1024 };
    let reader = opening(&address, asked_by_lurq(), cut).unwrap();
    // The block of the index is on its way, or there, with nothing read past the start yet.
    let until = Instant::now() + SECOND;
    while !asked.lock().iter().any(|range| range == "8192-9215") {
      assert!(Instant::now() < until, "the end was not asked for: {:?}", asked.lock());
      thread::sleep(Duration::from_millis(5));
    }
    drop(reader);
  }

  #[test]
  fn told_not_to_read_ahead_it_asks_for_what_is_read_and_no_more_until_it_is_told_to() {
    let body = numbered(10 * SMALL as usize);
    let (address, asked, _) = serve_file(body.clone(), true);
    let reader = asking(&address).unwrap();
    reader.read_ahead(false);
    assert_eq!(read(&reader, 100, 8).unwrap(), &body[100..108]);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(*asked.lock(), ["0-4095"], "its first block, and nothing after it");

    // Told to, what follows its last read is asked for at once, with nothing read.
    reader.read_ahead(true);
    let until = Instant::now() + SECOND;
    while asked.lock().len() < 3 {
      assert!(Instant::now() < until, "nothing was read ahead: {:?}", asked.lock());
      thread::sleep(Duration::from_millis(5));
    }
    let mut ahead = asked.lock().clone();
    ahead.sort();
    assert_eq!(ahead, ["0-4095", "4096-8191", "8192-12287"]);
  }

  #[test]
  fn an_address_opened_without_the_header_it_needs_fails_with_what_the_server_answered() {
    let (address, ..) = serve_file(numbered(1000), true);
    let refused = opening(&address, Vec::new(), Cut::even(SMALL));
    assert_eq!(refused.err(), Some(LoadError::Status(403)));
  }

  #[test]
  fn a_read_the_server_refuses_on_the_way_says_what_it_answered() {
    let body = numbered(40 * SMALL as usize);
    let (address, _, refusing) = serve_file(body.clone(), true);
    let reader = asking(&address).unwrap();
    refusing.store(true, Ordering::SeqCst);
    let error = read(&reader, 30 * SMALL, 8).unwrap_err();
    assert_eq!(LoadError::from(error), LoadError::Status(403));
    // The next read of it asks again.
    refusing.store(false, Ordering::SeqCst);
    let at = 30 * SMALL as usize;
    assert_eq!(read(&reader, at as u64, 8).unwrap(), &body[at..at + 8]);
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

  /// The bytes a request asked for: from, and to.
  fn span(range: &str) -> (u64, u64) {
    let (from, to) = range.split_once('-').unwrap();
    (from.parse().unwrap(), to.parse().unwrap())
  }

  #[test]
  fn a_read_has_its_bytes_as_they_come_and_does_not_wait_for_its_block() {
    // A block of 256 KiB from a host that gives 256 KiB a second is whole a second after it was asked for.
    let block = 256 << 10;
    let body = numbered(300 << 10);
    let host = serve_capped(body.clone(), block, Duration::ZERO);
    let started = Instant::now();
    let reader = opening(&host.address, Vec::new(), Cut::even(block)).unwrap();
    assert_eq!(reader.len(), Some(body.len() as u64), "opened, it knows how long it is");
    assert_eq!(read(&reader, 0, 16 << 10).unwrap(), &body[..16 << 10]);
    let there = reader.buffered();
    assert!(
      there.len() == 1 && there[0].start == 0 && there[0].end < block,
      "the read had its bytes with its block still on its way: {there:?}"
    );
    assert!(
      started.elapsed() < Duration::from_millis(600),
      "opened and read in {:?}",
      started.elapsed()
    );
    // Further in the block, a read waits for its own bytes and no more.
    let at = 100 << 10;
    assert_eq!(read(&reader, at as u64, 8).unwrap(), &body[at..at + 8]);
    assert!(reader.buffered()[0].end < block);
    assert_eq!(read(&reader, 0, body.len()).unwrap(), body);
  }

  #[test]
  fn a_reader_that_plays_asks_in_order_over_one_connection() {
    let body = numbered(600 << 10);
    let host = serve_capped(body.clone(), 1 << 20, Duration::from_millis(50));
    let cut = Cut {
      first: 64 << 10,
      most: 256 << 10,
    };
    let reader = HttpReader::open(
      &Agents::new(4),
      Starts::new(0),
      Asked {
        read_ahead: Some(true),
        ..asked(&host.address, Vec::new(), cut)
      },
    )
    .unwrap();
    assert_eq!(read(&reader, 0, body.len()).unwrap(), body);
    let asked = host.asked.lock().clone();
    assert_eq!(
      asked[0], "0-262143",
      "the small first blocks are one request, as long as the largest block"
    );
    let mut at = 0;
    for range in &asked {
      let (from, to) = span(range);
      assert_eq!(
        from, at,
        "each request goes on from where the one before ended: {asked:?}"
      );
      at = to + 1;
    }
    assert_eq!(
      host.connections.load(Ordering::SeqCst),
      1,
      "all of them over the connection the first opened: {asked:?}"
    );
  }

  #[test]
  fn opened_to_be_played_later_it_asks_for_what_is_read_and_no_more() {
    let body = numbered(600 << 10);
    let host = serve_capped(body.clone(), 8 << 20, Duration::ZERO);
    let cut = Cut {
      first: 64 << 10,
      most: 256 << 10,
    };
    let reader = HttpReader::open(
      &Agents::new(4),
      Starts::new(0),
      Asked {
        read_ahead: Some(false),
        ..asked(&host.address, Vec::new(), cut)
      },
    )
    .unwrap();
    assert_eq!(read(&reader, 100, 8).unwrap(), &body[100..108]);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(*host.asked.lock(), ["0-65535"], "its first block, alone");
    // Opened by what does not say, its first request is the first block too, and it reads ahead once it is read.
    let reader = opening(&host.address, Vec::new(), cut).unwrap();
    thread::sleep(Duration::from_millis(150));
    assert_eq!(*host.asked.lock(), ["0-65535", "0-65535"]);
    assert_eq!(read(&reader, 100, 8).unwrap(), &body[100..108]);
    let until = Instant::now() + SECOND;
    while host.asked.lock().len() < 3 {
      assert!(
        Instant::now() < until,
        "nothing was read ahead: {:?}",
        host.asked.lock()
      );
      thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(host.asked.lock()[2], "65536-262143");
  }

  #[test]
  fn opened_to_be_played_later_it_fetches_as_far_as_it_is_told_and_no_further() {
    let body = numbered(1 << 20);
    let host = serve_capped(body.clone(), 8 << 20, Duration::ZERO);
    let cut = Cut::even(64 << 10);
    let reader = HttpReader::open(
      &Agents::new(4),
      Starts::new(0),
      Asked {
        read_ahead: Some(false),
        ..asked(&host.address, Vec::new(), cut)
      },
    )
    .unwrap();
    assert_eq!(read(&reader, 100, 8).unwrap(), &body[100..108]);
    // Told to have its first 200 KB: the blocks up to the one that holds it, and no more.
    reader.read_ahead_to(200 << 10);
    let until = Instant::now() + SECOND;
    while !reader.buffered().iter().any(|range| range.end >= 200 << 10) {
      assert!(
        Instant::now() < until,
        "nothing was fetched ahead: {:?}",
        host.asked.lock()
      );
      thread::sleep(Duration::from_millis(5));
    }
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
      *host.asked.lock(),
      ["0-65535", "65536-131071", "131072-196607", "196608-262143"]
    );
    // Told less, it fetches nothing more.
    reader.read_ahead_to(100 << 10);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(host.asked.lock().len(), 4);
  }

  #[test]
  fn a_read_sent_to_another_place_is_asked_for_beside_the_block_on_its_way() {
    // A block is whole half a second after it was asked for.
    let block: u64 = 256 << 10;
    let body = numbered(2 << 20);
    let host = serve_capped(body.clone(), 2 * block, Duration::ZERO);
    let reader = opening(&host.address, Vec::new(), Cut::even(block)).unwrap();
    assert_eq!(read(&reader, 0, 1024).unwrap(), &body[..1024]);
    let far = 6 * block as usize + 100;
    let started = Instant::now();
    assert_eq!(read(&reader, far as u64, 1024).unwrap(), &body[far..far + 1024]);
    assert!(
      started.elapsed() < Duration::from_millis(300),
      "it did not wait for the block on its way: {:?}",
      started.elapsed()
    );
    assert!(reader.buffered()[0].end < block, "which is still on its way");
    assert_eq!(host.connections.load(Ordering::SeqCst), 2);
    assert_eq!(
      host.asked.lock()[1],
      format!("{far}-{}", 7 * block - 1),
      "from the read's place"
    );
  }

  #[test]
  fn a_block_brought_from_a_reads_place_is_brought_again_for_a_read_before_it() {
    let body = numbered(4 * SMALL as usize);
    let (address, asked, _) = serve_file(body.clone(), true);
    let reader = opening(&address, asked_by_lurq(), Cut::even(SMALL)).unwrap();
    let into = 2 * SMALL as usize + 500;
    assert_eq!(read(&reader, into as u64, 64).unwrap(), &body[into..into + 64]);
    assert!(
      reader
        .buffered()
        .iter()
        .any(|range| range.start == into as u64 && range.end == 3 * SMALL),
      "what is there of the block starts at the read's place: {:?}",
      reader.buffered()
    );
    let before = 2 * SMALL as usize + 10;
    assert_eq!(read(&reader, before as u64, 64).unwrap(), &body[before..before + 64]);
    let asked = asked.lock().clone();
    assert!(asked.contains(&format!("{into}-{}", 3 * SMALL - 1)), "{asked:?}");
    assert!(asked.contains(&format!("{before}-{}", 3 * SMALL - 1)), "{asked:?}");
  }

  /// A reader of `address` among the other readers of `agents`, ranked `rank`.
  fn ranked(agents: &Agents, address: &str, rank: u8) -> HttpReader {
    let reader = HttpReader::open(
      agents,
      Starts::new(0),
      asked(address, asked_by_lurq(), Cut::even(SMALL)),
    )
    .unwrap();
    reader.set_rank(rank);
    reader
  }

  #[test]
  fn a_read_that_waits_has_a_request_whatever_else_goes_to_its_host() {
    let host = Host::default();
    let behind = Rank {
      waited: false,
      reader: 1,
    };
    for _ in 0..HOST_REQUESTS {
      assert!(host.admit(behind));
    }
    assert!(!host.admit(behind), "no room for more of what nobody waits for");
    assert!(
      host.admit(Rank {
        waited: true,
        reader: 0,
      }),
      "a read waits for this one"
    );
  }

  #[test]
  fn the_reader_ranked_highest_of_those_that_wait_for_room_goes_first() {
    let (address, ..) = serve_file(numbered(100), true);
    let agents = Agents::new(4);
    let (low, high) = (ranked(&agents, &address, 1), ranked(&agents, &address, 5));
    let host = agents.host(&address);
    assert!(Arc::ptr_eq(&host, &low.0.host), "one host for both");
    let ahead = |reader: u8| Rank { waited: false, reader };
    // The readers have brought their first block, and ask for nothing more.
    let until = Instant::now() + SECOND;
    while !host.state.lock().running.is_empty() {
      assert!(Instant::now() < until, "the readers did not end their requests");
      thread::sleep(Duration::from_millis(5));
    }
    for _ in 0..HOST_REQUESTS {
      host.admit(ahead(1));
    }
    host.wait_for_room(&low.0, ahead(1));
    host.wait_for_room(&high.0, ahead(5));
    let first = host.release(ahead(1)).expect("a reader that waits");
    assert!(Arc::ptr_eq(&first, &high.0), "the one ranked higher");
    let next = host.release(ahead(1)).expect("the other");
    assert!(Arc::ptr_eq(&next, &low.0));
  }

  #[test]
  fn a_request_ranked_lower_stands_still_while_one_ranked_higher_brings_what_a_read_waits_for() {
    let (address, ..) = serve_file(numbered(100), true);
    let agents = Agents::new(4);
    let (behind, front) = (ranked(&agents, &address, 1), ranked(&agents, &address, 5));
    let host = agents.host(&address);
    let ahead = || Run {
      first: 3,
      last: 3,
      ahead: true,
      skip: 0,
    };
    assert!(behind.0.goes_on(&ahead(), Instant::now()), "nothing else is on its way");
    let waited = front.0.rank_of(&Run {
      first: 5,
      last: 5,
      ahead: false,
      skip: 0,
    });
    assert!(host.admit(waited));
    let went = Arc::new(AtomicBool::new(false));
    let standing = {
      let (ranges, went) = (behind.0.clone(), went.clone());
      thread::spawn(move || {
        let on = ranges.goes_on(&ahead(), Instant::now());
        went.store(true, Ordering::SeqCst);
        on
      })
    };
    thread::sleep(Duration::from_millis(120));
    assert!(!went.load(Ordering::SeqCst), "it stands still");
    host.release(waited);
    assert!(standing.join().unwrap(), "and reads on once that request ended");
  }

  #[test]
  fn the_readers_of_two_hosts_do_not_wait_for_each_other() {
    let (one, ..) = serve_file(numbered(100), true);
    let (other, ..) = serve_file(numbered(100), true);
    let agents = Agents::new(4);
    let (a, b) = (ranked(&agents, &one, 1), ranked(&agents, &other, 5));
    assert!(!Arc::ptr_eq(&a.0.host, &b.0.host));
    assert!(b.0.host.admit(Rank {
      waited: true,
      reader: 5,
    }));
    assert!(!a.0.host.outranked(Rank {
      waited: false,
      reader: 1,
    }));
  }

  #[test]
  fn a_request_nobody_waits_for_stands_still_while_a_read_waits_for_another() {
    let (address, ..) = serve_file(numbered(100), true);
    let reader = asking(&address).unwrap();
    let ranges = reader.0.clone();
    let ahead = || Run {
      first: 3,
      last: 3,
      ahead: true,
      skip: 0,
    };
    assert!(
      ranges.goes_on(&ahead(), Instant::now()),
      "nothing is waited for: it reads on"
    );
    // A read waits for a block another request brings.
    {
      let mut blocks = ranges.blocks.lock();
      blocks.waited.push(7);
      blocks.coming.push(7);
    }
    let went = Arc::new(AtomicBool::new(false));
    let standing = {
      let (ranges, went) = (ranges.clone(), went.clone());
      thread::spawn(move || {
        let on = ranges.goes_on(&ahead(), Instant::now());
        went.store(true, Ordering::SeqCst);
        on
      })
    };
    thread::sleep(Duration::from_millis(120));
    assert!(!went.load(Ordering::SeqCst), "it stands still");
    let waited_for = Run {
      first: 7,
      last: 7,
      ahead: false,
      skip: 0,
    };
    assert!(
      ranges.goes_on(&waited_for, Instant::now()),
      "the request that brings what is waited for reads on"
    );
    let long_ago = Instant::now().checked_sub(SECOND * 3 / 5).unwrap();
    assert!(
      ranges.goes_on(&ahead(), long_ago),
      "past half of its time it reads on whatever is waited for"
    );
    // The read has its bytes.
    ranges.blocks.lock().waited.clear();
    assert!(standing.join().unwrap(), "and it reads on");
    drop(reader);
    assert!(
      !ranges.goes_on(&ahead(), Instant::now()),
      "the reader is dropped: its requests end"
    );
  }

  #[test]
  fn a_read_that_waited_for_what_was_read_ahead_asks_again_itself_where_that_failed() {
    let body = numbered(40 * SMALL as usize);
    let (address, ..) = serve_file(body.clone(), true);
    let reader = Arc::new(asking(&address).unwrap());
    // A request the read did not ask for, and then one made for it.
    for (index, ahead) in [(30_u64, true), (35, false)] {
      let at = (index * SMALL) as usize;
      // As far as the reader knows, a request is bringing the block.
      reader.0.blocks.lock().coming.push(index);
      let waiting = {
        let reader = reader.clone();
        thread::spawn(move || read(&reader, at as u64, 8))
      };
      let until = Instant::now() + SECOND;
      while !reader.0.blocks.lock().waited.contains(&index) {
        assert!(Instant::now() < until, "the read does not wait");
        thread::sleep(Duration::from_millis(2));
      }
      // It failed.
      {
        let mut blocks = reader.0.blocks.lock();
        blocks.coming.retain(|coming| *coming != index);
        blocks.failed.push((index, LoadError::TimedOut, ahead));
      }
      reader.0.came.notify_all();
      let read = waiting.join().unwrap();
      if ahead {
        assert_eq!(read.unwrap(), &body[at..at + 8], "asked again, by the read");
      } else {
        assert_eq!(LoadError::from(read.unwrap_err()), LoadError::TimedOut);
      }
    }
  }

  #[test]
  fn the_start_of_an_address_is_kept_for_the_reader_that_opens_it_next() {
    let block = SMALL as usize;
    let body = numbered(40 * block);
    let (address, ranges, _) = serve_file(body.clone(), true);
    let (agents, starts) = (Agents::new(4), Starts::new(2));
    let open = |key: u64, cut: Cut| {
      HttpReader::open(
        &agents,
        starts.clone(),
        Asked {
          key,
          read_ahead: Some(false),
          ..asked(&address, asked_by_lurq(), cut)
        },
      )
      .unwrap()
    };
    let cut = Cut::even(SMALL);
    let reader = open(1, cut);
    assert_eq!(read(&reader, 0, 3 * block).unwrap(), &body[..3 * block]);
    assert_eq!(read(&reader, 10 * SMALL, 8).unwrap(), &body[10 * block..10 * block + 8]);
    drop(reader);
    let before = ranges.lock().len();
    assert_eq!(before, 4);

    // Opened again: its start is there, with how long it is and what it is, and nothing is asked for.
    let again = open(1, cut);
    assert_eq!(again.len(), Some(body.len() as u64));
    assert_eq!(again.content_type().as_deref(), Some("video/mp4"));
    assert_eq!(
      again.buffered(),
      vec![0..3 * SMALL],
      "the blocks from its start, and not one further in"
    );
    assert_eq!(read(&again, 0, 3 * block).unwrap(), &body[..3 * block]);
    assert_eq!(ranges.lock().len(), before, "opened and read with no request");
    // What follows the start is asked for.
    assert_eq!(read(&again, 3 * SMALL, 8).unwrap(), &body[3 * block..3 * block + 8]);
    assert_eq!(ranges.lock().last().unwrap(), "12288-16383");
    drop(again);

    // Another address has no start, and neither has this one cut another way.
    drop(open(2, cut));
    assert_eq!(ranges.lock().last().unwrap(), "0-4095");
    drop(open(1, Cut::even(2 * SMALL)));
    assert_eq!(ranges.lock().last().unwrap(), "0-8191");
    // Forgotten, it is asked for again.
    let asked_so_far = ranges.lock().len();
    drop(open(2, cut));
    assert_eq!(ranges.lock().len(), asked_so_far, "kept until it is forgotten");
    assert!(starts.forget(2));
    assert!(!starts.forget(2));
    drop(open(2, cut));
    assert_eq!(ranges.lock().len(), asked_so_far + 1);
    // Of no more addresses than it is told: the one kept longest ago is let go.
    drop(open(3, cut));
    drop(open(4, cut));
    let asked_so_far = ranges.lock().len();
    drop(open(2, cut));
    assert_eq!(ranges.lock().len(), asked_so_far + 1, "two were kept after it");
    // And of none, told so.
    starts.resize(0);
    let asked_so_far = ranges.lock().len();
    drop(open(2, cut));
    drop(open(2, cut));
    assert_eq!(ranges.lock().len(), asked_so_far + 2);
  }

  #[test]
  fn a_request_is_lent_the_agent_that_asked_its_host_last() {
    assert_eq!(host_of("https://one.example/a/b.mp4?x=1"), "https://one.example");
    assert_eq!(host_of("http://127.0.0.1:8080"), "http://127.0.0.1:8080");
    assert_eq!(host_of("https://one.example?x=/y"), "https://one.example");
    let agents = Agents::new(4);
    // Two requests at once, to two hosts: an agent each.
    let (one, two) = (
      agents.lend("https://one.example/a"),
      agents.lend("https://two.example/b"),
    );
    drop(one);
    drop(two);
    assert_eq!(agents.made(), 2);
    let idle =
      |agents: &Agents| -> Vec<String> { agents.idle.lock().iter().map(|(_, host)| host.to_string()).collect() };
    // The agent that asked the host, and not the one given back last.
    let again = agents.lend("https://one.example/another");
    assert_eq!(idle(&agents), ["https://two.example"]);
    drop(again);
    // A host nobody asked takes the one given back last, and makes no agent.
    let third = agents.lend("https://three.example/c");
    assert_eq!(idle(&agents), ["https://two.example"]);
    drop(third);
    assert_eq!(idle(&agents), ["https://two.example", "https://three.example"]);
    assert_eq!(agents.made(), 2);
  }

  #[test]
  fn a_connection_opened_ahead_is_the_one_the_first_request_goes_over() {
    // A host that takes a fifth of a second over a new connection.
    let body = numbered(100 << 10);
    let host = serve_capped(body.clone(), 8 << 20, Duration::from_millis(200));
    let agents = Agents::new(4);
    connect(&agents, host.address.clone(), Vec::new(), SECOND);
    // Asked a moment ago, the host is not asked again.
    connect(&agents, host.address.clone(), Vec::new(), SECOND);
    let until = Instant::now() + SECOND;
    while agents.idle.lock().is_empty() {
      assert!(Instant::now() < until, "no connection was opened ahead");
      thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(*host.asked.lock(), ["HEAD"]);

    let started = Instant::now();
    let reader = HttpReader::open(
      &agents,
      Starts::new(0),
      asked(&host.address, Vec::new(), Cut::even(64 << 10)),
    )
    .unwrap();
    assert_eq!(read(&reader, 0, 1024).unwrap(), &body[..1024]);
    assert!(
      started.elapsed() < Duration::from_millis(150),
      "opened with no new connection: {:?}",
      started.elapsed()
    );
    assert_eq!(host.connections.load(Ordering::SeqCst), 1);
    assert_eq!(*host.asked.lock(), ["HEAD", "0-65535"]);
  }
}
