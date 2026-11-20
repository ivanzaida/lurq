//! A [`Reader`] as the stream Media Foundation reads: what the app's loader reads by ranges, with its headers, is
//! given to the decoder as `SHCreateMemStream` gives bytes. `MFCreateMFByteStreamOnStream` makes the byte stream
//! of it, and reads it on its own threads, so a read that waits for the network does not wait on the media's.

use std::sync::Arc;

use parking_lot::Mutex;
use windows::{
  Win32::{
    Foundation::{E_NOTIMPL, S_OK, STG_E_ACCESSDENIED, STG_E_INVALIDFUNCTION, STG_E_READFAULT, STG_E_SEEKERROR},
    System::Com::{
      ISequentialStream_Impl, IStream, IStream_Impl, LOCKTYPE, STATFLAG, STATSTG, STGC, STGM_READ, STGTY_STREAM,
      STREAM_SEEK, STREAM_SEEK_CUR, STREAM_SEEK_END, STREAM_SEEK_SET,
    },
  },
  core::{HRESULT, Ref, Result, implement},
};

use crate::resources::Reader;

#[implement(IStream)]
struct ReaderStream {
  reader: Arc<dyn Reader>,
  at: Mutex<u64>,
}

/// A stream of what `reader` reads, from its start.
pub(super) fn stream_of(reader: Arc<dyn Reader>) -> IStream {
  ReaderStream {
    reader,
    at: Mutex::new(0),
  }
  .into()
}

impl ISequentialStream_Impl for ReaderStream_Impl {
  fn Read(&self, pv: *mut core::ffi::c_void, cb: u32, pcbread: *mut u32) -> HRESULT {
    if pv.is_null() {
      return STG_E_INVALIDFUNCTION;
    }
    // SAFETY: the caller gives a buffer of `cb` bytes at `pv`.
    let buf = unsafe { std::slice::from_raw_parts_mut(pv.cast::<u8>(), cb as usize) };
    let mut at = self.at.lock();
    // A reader gives what it has at once; the stream gives all that was asked for, or what is left before the
    // end: the byte stream made of it takes fewer as the end.
    let mut filled = 0;
    let read = loop {
      if filled == buf.len() {
        break S_OK;
      }
      match self.reader.read_at(*at + filled as u64, &mut buf[filled..]) {
        Ok(0) => break S_OK,
        Ok(read) => filled += read,
        Err(error) => {
          tracing::debug!(target: "lurq::media", "a read of the stream failed: {error}");
          break STG_E_READFAULT;
        }
      }
    };
    *at += filled as u64;
    if !pcbread.is_null() {
      // SAFETY: where the caller gives a place for the count, it is one `u32`.
      unsafe { *pcbread = filled as u32 };
    }
    read
  }

  fn Write(&self, _pv: *const core::ffi::c_void, _cb: u32, _pcbwritten: *mut u32) -> HRESULT {
    STG_E_ACCESSDENIED
  }
}

impl IStream_Impl for ReaderStream_Impl {
  fn Seek(&self, dlibmove: i64, dworigin: STREAM_SEEK, plibnewposition: *mut u64) -> Result<()> {
    let mut at = self.at.lock();
    let from = match dworigin {
      STREAM_SEEK_SET => 0,
      STREAM_SEEK_CUR => *at,
      STREAM_SEEK_END => self.reader.len().ok_or(STG_E_SEEKERROR)?,
      _ => return Err(STG_E_INVALIDFUNCTION.into()),
    };
    *at = from.checked_add_signed(dlibmove).ok_or(STG_E_SEEKERROR)?;
    if !plibnewposition.is_null() {
      // SAFETY: where the caller gives a place for the position, it is one `u64`.
      unsafe { *plibnewposition = *at };
    }
    Ok(())
  }

  fn SetSize(&self, _libnewsize: u64) -> Result<()> {
    Err(STG_E_ACCESSDENIED.into())
  }

  fn CopyTo(&self, _pstm: Ref<IStream>, _cb: u64, _pcbread: *mut u64, _pcbwritten: *mut u64) -> Result<()> {
    Err(E_NOTIMPL.into())
  }

  fn Commit(&self, _grfcommitflags: &STGC) -> Result<()> {
    Ok(())
  }

  fn Revert(&self) -> Result<()> {
    Ok(())
  }

  fn LockRegion(&self, _liboffset: u64, _cb: u64, _dwlocktype: &LOCKTYPE) -> Result<()> {
    Err(STG_E_INVALIDFUNCTION.into())
  }

  fn UnlockRegion(&self, _liboffset: u64, _cb: u64, _dwlocktype: u32) -> Result<()> {
    Err(STG_E_INVALIDFUNCTION.into())
  }

  fn Stat(&self, pstatstg: *mut STATSTG, _grfstatflag: &STATFLAG) -> Result<()> {
    if pstatstg.is_null() {
      return Err(STG_E_INVALIDFUNCTION.into());
    }
    // SAFETY: the caller gives one `STATSTG` to fill. No name is given, so there is nothing for it to free.
    unsafe {
      *pstatstg = STATSTG {
        r#type: STGTY_STREAM.0 as u32,
        cbSize: self.reader.len().unwrap_or(0),
        grfMode: STGM_READ,
        ..Default::default()
      };
    }
    Ok(())
  }

  fn Clone(&self) -> Result<IStream> {
    Ok(
      ReaderStream {
        reader: self.reader.clone(),
        at: Mutex::new(*self.at.lock()),
      }
      .into(),
    )
  }
}
