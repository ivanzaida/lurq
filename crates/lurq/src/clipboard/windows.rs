//! Immediate Windows text writes with a persistent, thread-owned HWND.
//!
//! The owner survives each Copy return. The normal native UI event loop pumps
//! its thread's messages; no delayed rendering, worker or clipboard retry is
//! introduced. Owner and open-session resources must remain on this thread.

mod memory;
mod native;
#[cfg(test)]
mod tests;

use std::cell::RefCell;

trait Platform {
  type Owner;
  type Data;
  type Session: Session<Data = Self::Data>;

  fn prepare(&self, text: &str) -> Option<Self::Data>;
  fn owner(&self) -> Option<Self::Owner>;
  fn open(&self, owner: &Self::Owner) -> Option<Self::Session>;
}

trait Session {
  type Data;

  fn empty(&mut self) -> bool;
  fn put(&mut self, data: Self::Data) -> bool;
  fn close(&mut self) -> bool;
}

struct Writer<P: Platform> {
  platform: P,
  owner: Option<P::Owner>,
}

impl<P: Platform> Writer<P> {
  fn new(platform: P) -> Self {
    Self { platform, owner: None }
  }

  fn write(&mut self, text: &str) -> bool {
    // No clipboard mutation precedes allocation, encoding or unlock success.
    let Some(data) = self.platform.prepare(text) else {
      return false;
    };
    if self.owner.is_none() {
      self.owner = self.platform.owner();
    }
    let Some(owner) = self.owner.as_ref() else {
      return false;
    };
    let Some(mut session) = self.platform.open(owner) else {
      return false;
    };
    let written = session.empty() && session.put(data);
    let closed = session.close();
    written && closed
  }
}

thread_local! {
  // One owner per calling thread, reused across writes and destroyed only at
  // thread exit. It is never moved to another thread or dropped after Copy.
  static WRITER: RefCell<Writer<native::Native>> = RefCell::new(Writer::new(native::Native));
}

pub(super) fn copy(text: &str) -> bool {
  WRITER
    .try_with(|writer| writer.try_borrow_mut().is_ok_and(|mut writer| writer.write(text)))
    .unwrap_or(false)
}
