//! Movable UTF16 storage: only a successful clipboard transfer relinquishes it.

use std::{ffi::c_void, marker::PhantomData, rc::Rc};

use ::windows::Win32::{
  Foundation::{GetLastError, HGLOBAL, NO_ERROR, SetLastError},
  System::Memory::{GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalSize},
};

// The generated Result bindings treat zero as an error for both functions.
// Win32 instead defines zero + NO_ERROR as successful final unlock, and NULL
// as successful free. Preserve these raw return values explicitly.
#[link(name = "kernel32")]
unsafe extern "system" {
  #[link_name = "GlobalUnlock"]
  fn unlock_memory(memory: *mut c_void) -> i32;
  #[link_name = "GlobalFree"]
  fn free_memory(memory: *mut c_void) -> *mut c_void;
}

pub(super) struct Data {
  handle: Option<HGLOBAL>,
  #[cfg(test)]
  units: usize,
  _thread: PhantomData<Rc<()>>,
}

impl Data {
  pub(super) fn prepare(text: &str) -> Option<Self> {
    let units = text.encode_utf16().count().checked_add(1)?;
    let bytes = units.checked_mul(size_of::<u16>())?;
    if bytes > isize::MAX as usize {
      return None;
    }
    // SAFETY: allocates this call's movable storage; its sole owner is Data.
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes) }.ok()?;
    let data = Self {
      handle: Some(handle),
      #[cfg(test)]
      units,
      _thread: PhantomData,
    };
    // SAFETY: the allocation remains owned and no clipboard transfer occurred.
    if unsafe { GlobalSize(handle) } < bytes {
      return None;
    }
    let lock = data.lock()?;
    // SAFETY: checked allocation holds exactly the encoded units plus NUL.
    unsafe {
      for (index, value) in text.encode_utf16().chain(std::iter::once(0)).enumerate() {
        lock.pointer.add(index).write(value);
      }
    }
    lock.finish().then_some(data)
  }

  pub(super) fn handle(&self) -> HGLOBAL {
    self.handle.expect("untransferred clipboard allocation")
  }

  pub(super) fn transfer(mut self) {
    self.handle = None;
  }

  fn lock(&self) -> Option<Lock<'_>> {
    // SAFETY: Data owns this valid, untransferred allocation.
    let pointer = unsafe { GlobalLock(self.handle()) }.cast::<u16>();
    (!pointer.is_null()).then_some(Lock {
      data: self,
      pointer,
      locked: true,
    })
  }
}

impl Drop for Data {
  fn drop(&mut self) {
    if let Some(handle) = self.handle.take() {
      // SAFETY: only allocations not transferred to Windows reach this path.
      if !unsafe { free_memory(handle.0) }.is_null() {
        tracing::warn!("untransferred clipboard allocation could not be freed");
      }
    }
  }
}

struct Lock<'a> {
  data: &'a Data,
  pointer: *mut u16,
  locked: bool,
}

impl Lock<'_> {
  fn finish(mut self) -> bool {
    self.locked = false;
    unlock(self.data.handle())
  }
}

fn unlock(handle: HGLOBAL) -> bool {
  // SAFETY: called exactly once for a matching successful GlobalLock; clearing
  // last error distinguishes successful zero from an actual unlock failure.
  unsafe {
    SetLastError(NO_ERROR);
    unlock_memory(handle.0) != 0 || GetLastError() == NO_ERROR
  }
}

impl Drop for Lock<'_> {
  fn drop(&mut self) {
    if self.locked {
      let _ = unlock(self.data.handle());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn windows_clipboard_real_memory_encodes_surrogates_and_empty_nul() {
    for (text, expected) in [("A😀", vec![0x41, 0xd83d, 0xde00, 0]), ("", vec![0])] {
      let data = Data::prepare(text).expect("synthetic allocation");
      let lock = data.lock().expect("owned memory lock");
      // SAFETY: this test borrows its own live, locked allocation only.
      let actual = unsafe { std::slice::from_raw_parts(lock.pointer, data.units) };
      assert_eq!(actual, expected);
      assert!(lock.finish());
      drop(data);
    }
  }
}
