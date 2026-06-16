use std::{
  collections::HashMap,
  fmt::{Display, Formatter},
  path::Path,
  sync::Arc,
};

use parking_lot::RwLock;
use redb::{Database, ReadableDatabase, TableDefinition};

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("persistent_values");

const TYPE_BOOL: u8 = 1;
const TYPE_STRING: u8 = 2;
const TYPE_I8: u8 = 3;
const TYPE_I16: u8 = 4;
const TYPE_I32: u8 = 5;
const TYPE_I64: u8 = 6;
const TYPE_ISIZE: u8 = 7;
const TYPE_U8: u8 = 8;
const TYPE_U16: u8 = 9;
const TYPE_U32: u8 = 10;
const TYPE_U64: u8 = 11;
const TYPE_USIZE: u8 = 12;
const TYPE_F32: u8 = 13;
const TYPE_F64: u8 = 14;
const TYPE_I128: u8 = 15;
const TYPE_U128: u8 = 16;
const TYPE_CHAR: u8 = 17;

#[derive(Clone)]
pub struct PersistentStorage {
  backend: PersistentStorageBackend,
}

#[derive(Clone)]
enum PersistentStorageBackend {
  Memory(Arc<RwLock<HashMap<String, Vec<u8>>>>),
  Redb(Arc<Database>),
}

#[derive(Debug)]
pub enum PersistentStorageError {
  Backend(String),
}

impl Display for PersistentStorageError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Backend(message) => write!(f, "{message}"),
    }
  }
}

impl std::error::Error for PersistentStorageError {}

pub trait PersistentValue: Sized {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self>;
}

pub trait IntoPersistentValue {
  fn encode_persistent_value(self) -> Vec<u8>;
}

impl Default for PersistentStorage {
  fn default() -> Self {
    Self::memory()
  }
}

impl PersistentStorage {
  pub fn memory() -> Self {
    Self {
      backend: PersistentStorageBackend::Memory(Arc::new(RwLock::new(HashMap::new()))),
    }
  }

  pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistentStorageError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
      && !parent.as_os_str().is_empty()
    {
      std::fs::create_dir_all(parent).map_err(backend_error)?;
    }

    let db = Database::create(path).map_err(backend_error)?;
    let storage = Self {
      backend: PersistentStorageBackend::Redb(Arc::new(db)),
    };
    storage.ensure_table()?;
    Ok(storage)
  }

  pub fn value<T: PersistentValue>(&self, key: &str) -> Option<T> {
    self.try_value(key).ok().flatten()
  }

  pub fn try_value<T: PersistentValue>(&self, key: &str) -> Result<Option<T>, PersistentStorageError> {
    let Some(bytes) = self.raw_value(key)? else {
      return Ok(None);
    };
    Ok(T::decode_persistent_value(&bytes))
  }

  pub fn set_value<T: IntoPersistentValue>(&self, key: &str, value: T) -> Result<(), PersistentStorageError> {
    let bytes = value.encode_persistent_value();
    match &self.backend {
      PersistentStorageBackend::Memory(values) => {
        values.write().insert(key.to_owned(), bytes);
        Ok(())
      }
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_write().map_err(backend_error)?;
        {
          let mut table = txn.open_table(TABLE).map_err(backend_error)?;
          table.insert(key, bytes.as_slice()).map_err(backend_error)?;
        }
        txn.commit().map_err(backend_error)
      }
    }
  }

  pub fn remove_value(&self, key: &str) -> Result<(), PersistentStorageError> {
    match &self.backend {
      PersistentStorageBackend::Memory(values) => {
        values.write().remove(key);
        Ok(())
      }
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_write().map_err(backend_error)?;
        {
          let mut table = txn.open_table(TABLE).map_err(backend_error)?;
          table.remove(key).map_err(backend_error)?;
        }
        txn.commit().map_err(backend_error)
      }
    }
  }

  fn ensure_table(&self) -> Result<(), PersistentStorageError> {
    let PersistentStorageBackend::Redb(db) = &self.backend else {
      return Ok(());
    };

    let txn = db.begin_write().map_err(backend_error)?;
    {
      let _table = txn.open_table(TABLE).map_err(backend_error)?;
    }
    txn.commit().map_err(backend_error)
  }

  fn raw_value(&self, key: &str) -> Result<Option<Vec<u8>>, PersistentStorageError> {
    match &self.backend {
      PersistentStorageBackend::Memory(values) => Ok(values.read().get(key).cloned()),
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_read().map_err(backend_error)?;
        let table = txn.open_table(TABLE).map_err(backend_error)?;
        Ok(
          table
            .get(key)
            .map_err(backend_error)?
            .map(|value| value.value().to_vec()),
        )
      }
    }
  }
}

fn backend_error(error: impl Display) -> PersistentStorageError {
  PersistentStorageError::Backend(error.to_string())
}

fn tagged(tag: u8, payload: impl AsRef<[u8]>) -> Vec<u8> {
  let payload = payload.as_ref();
  let mut bytes = Vec::with_capacity(payload.len() + 1);
  bytes.push(tag);
  bytes.extend_from_slice(payload);
  bytes
}

fn fixed<const N: usize>(bytes: &[u8], tag: u8) -> Option<[u8; N]> {
  if bytes.first().copied()? != tag {
    return None;
  }
  bytes.get(1..)?.try_into().ok()
}

impl PersistentValue for bool {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    if bytes.first().copied()? != TYPE_BOOL {
      return None;
    }
    match *bytes.get(1)? {
      0 => Some(false),
      1 => Some(true),
      _ => None,
    }
  }
}

impl IntoPersistentValue for bool {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_BOOL, [u8::from(self)])
  }
}

impl PersistentValue for String {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    if bytes.first().copied()? != TYPE_STRING {
      return None;
    }
    String::from_utf8(bytes.get(1..)?.to_vec()).ok()
  }
}

impl IntoPersistentValue for String {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_STRING, self.as_bytes())
  }
}

impl IntoPersistentValue for &str {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_STRING, self.as_bytes())
  }
}

macro_rules! impl_int_persistent_value {
  ($ty:ty, $tag:ident) => {
    impl PersistentValue for $ty {
      fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
        Some(<$ty>::from_le_bytes(fixed::<{ std::mem::size_of::<$ty>() }>(
          bytes, $tag,
        )?))
      }
    }

    impl IntoPersistentValue for $ty {
      fn encode_persistent_value(self) -> Vec<u8> {
        tagged($tag, self.to_le_bytes())
      }
    }
  };
}

impl_int_persistent_value!(i8, TYPE_I8);
impl_int_persistent_value!(i16, TYPE_I16);
impl_int_persistent_value!(i32, TYPE_I32);
impl_int_persistent_value!(i64, TYPE_I64);
impl_int_persistent_value!(i128, TYPE_I128);
impl_int_persistent_value!(u8, TYPE_U8);
impl_int_persistent_value!(u16, TYPE_U16);
impl_int_persistent_value!(u32, TYPE_U32);
impl_int_persistent_value!(u64, TYPE_U64);
impl_int_persistent_value!(u128, TYPE_U128);

impl PersistentValue for isize {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    i64::from_le_bytes(fixed::<8>(bytes, TYPE_ISIZE)?).try_into().ok()
  }
}

impl IntoPersistentValue for isize {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_ISIZE, (self as i64).to_le_bytes())
  }
}

impl PersistentValue for usize {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    u64::from_le_bytes(fixed::<8>(bytes, TYPE_USIZE)?).try_into().ok()
  }
}

impl IntoPersistentValue for usize {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_USIZE, (self as u64).to_le_bytes())
  }
}

impl PersistentValue for f32 {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    Some(f32::from_le_bytes(fixed::<4>(bytes, TYPE_F32)?))
  }
}

impl IntoPersistentValue for f32 {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_F32, self.to_le_bytes())
  }
}

impl PersistentValue for f64 {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    Some(f64::from_le_bytes(fixed::<8>(bytes, TYPE_F64)?))
  }
}

impl IntoPersistentValue for f64 {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_F64, self.to_le_bytes())
  }
}

impl PersistentValue for char {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    char::from_u32(u32::from_le_bytes(fixed::<4>(bytes, TYPE_CHAR)?))
  }
}

impl IntoPersistentValue for char {
  fn encode_persistent_value(self) -> Vec<u8> {
    tagged(TYPE_CHAR, (self as u32).to_le_bytes())
  }
}

#[cfg(test)]
mod tests {
  use super::PersistentStorage;

  #[test]
  fn memory_roundtrips_primitives() {
    let storage = PersistentStorage::memory();

    storage.set_value("enabled", true).unwrap();
    storage.set_value("count", 42_i32).unwrap();
    storage.set_value("ratio", 1.5_f64).unwrap();
    storage.set_value("name", "Ada").unwrap();
    storage.set_value("initial", 'L').unwrap();

    assert_eq!(storage.value::<bool>("enabled"), Some(true));
    assert_eq!(storage.value::<i32>("count"), Some(42));
    assert_eq!(storage.value::<f64>("ratio"), Some(1.5));
    assert_eq!(storage.value::<String>("name"), Some("Ada".to_owned()));
    assert_eq!(storage.value::<char>("initial"), Some('L'));
  }

  #[test]
  fn type_mismatch_returns_none() {
    let storage = PersistentStorage::memory();

    storage.set_value("count", 42_i32).unwrap();

    assert_eq!(storage.value::<String>("count"), None);
  }
}
