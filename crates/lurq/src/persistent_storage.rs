use std::{
  collections::HashMap,
  fmt::{Display, Formatter},
  path::Path,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use parking_lot::RwLock;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

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
const TYPE_STRUCT: u8 = 18;
const STRUCT_FIELD_NAMES_MARKER: &[u8] = b"LURQ_STRUCT_FIELDS_V1";

#[derive(Clone)]
pub struct PersistentStorage {
  backend: PersistentStorageBackend,
  revision: Arc<AtomicU64>,
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

pub struct PersistentWrite {
  key: String,
  value: Vec<u8>,
}

impl PersistentWrite {
  pub fn new<T: IntoPersistentValue>(key: impl AsRef<str>, value: T) -> Self {
    Self {
      key: key.as_ref().to_owned(),
      value: value.encode_persistent_value(),
    }
  }
}

pub trait IntoPersistentWrite {
  fn into_persistent_write(self) -> PersistentWrite;
}

impl IntoPersistentWrite for PersistentWrite {
  fn into_persistent_write(self) -> PersistentWrite {
    self
  }
}

impl<K, T> IntoPersistentWrite for (K, T)
where
  K: AsRef<str>,
  T: IntoPersistentValue,
{
  fn into_persistent_write(self) -> PersistentWrite {
    PersistentWrite::new(self.0, self.1)
  }
}

pub struct PersistentReadBatch {
  values: HashMap<String, Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentStorageSnapshotEntry {
  pub key: String,
  pub type_name: String,
  pub full_type_name: String,
  pub value: String,
  pub byte_len: usize,
}

impl PersistentReadBatch {
  pub fn value<T: PersistentValue>(&self, key: &str) -> Option<T> {
    self.values.get(key).and_then(|bytes| T::decode_persistent_value(bytes))
  }

  pub fn values<T, I, K>(&self, keys: I) -> Vec<Option<T>>
  where
    T: PersistentValue,
    I: IntoIterator<Item = K>,
    K: AsRef<str>,
  {
    keys.into_iter().map(|key| self.value(key.as_ref())).collect()
  }

  pub fn contains_key(&self, key: &str) -> bool {
    self.values.contains_key(key)
  }
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
      revision: Arc::new(AtomicU64::new(0)),
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
      revision: Arc::new(AtomicU64::new(0)),
    };
    storage.ensure_table()?;
    Ok(storage)
  }

  pub fn revision(&self) -> u64 {
    self.revision.load(Ordering::Relaxed)
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

  pub fn read_bulk<I, K>(&self, keys: I) -> Result<PersistentReadBatch, PersistentStorageError>
  where
    I: IntoIterator<Item = K>,
    K: AsRef<str>,
  {
    let keys = keys.into_iter().map(|key| key.as_ref().to_owned()).collect::<Vec<_>>();
    let values = self.raw_values(&keys)?;
    Ok(PersistentReadBatch {
      values: keys
        .into_iter()
        .zip(values)
        .filter_map(|(key, value)| value.map(|value| (key, value)))
        .collect(),
    })
  }

  pub fn read_bulk_values<T, I, K>(&self, keys: I) -> Result<Vec<Option<T>>, PersistentStorageError>
  where
    T: PersistentValue,
    I: IntoIterator<Item = K>,
    K: AsRef<str>,
  {
    let keys = keys.into_iter().map(|key| key.as_ref().to_owned()).collect::<Vec<_>>();
    Ok(self.read_bulk(&keys)?.values::<T, _, _>(&keys))
  }

  pub fn set_value<T: IntoPersistentValue>(&self, key: &str, value: T) -> Result<(), PersistentStorageError> {
    let bytes = value.encode_persistent_value();
    match &self.backend {
      PersistentStorageBackend::Memory(values) => {
        values.write().insert(key.to_owned(), bytes);
        self.bump_revision();
        Ok(())
      }
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_write().map_err(backend_error)?;
        {
          let mut table = txn.open_table(TABLE).map_err(backend_error)?;
          table.insert(key, bytes.as_slice()).map_err(backend_error)?;
        }
        txn.commit().map_err(backend_error)?;
        self.bump_revision();
        Ok(())
      }
    }
  }

  pub fn write_bulk<I, E>(&self, entries: I) -> Result<(), PersistentStorageError>
  where
    I: IntoIterator<Item = E>,
    E: IntoPersistentWrite,
  {
    let entries = entries
      .into_iter()
      .map(IntoPersistentWrite::into_persistent_write)
      .collect::<Vec<_>>();

    match &self.backend {
      PersistentStorageBackend::Memory(values) => {
        let mut values = values.write();
        for entry in entries {
          values.insert(entry.key, entry.value);
        }
        self.bump_revision();
        Ok(())
      }
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_write().map_err(backend_error)?;
        {
          let mut table = txn.open_table(TABLE).map_err(backend_error)?;
          for entry in entries {
            table
              .insert(entry.key.as_str(), entry.value.as_slice())
              .map_err(backend_error)?;
          }
        }
        txn.commit().map_err(backend_error)?;
        self.bump_revision();
        Ok(())
      }
    }
  }

  pub fn remove_value(&self, key: &str) -> Result<(), PersistentStorageError> {
    match &self.backend {
      PersistentStorageBackend::Memory(values) => {
        values.write().remove(key);
        self.bump_revision();
        Ok(())
      }
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_write().map_err(backend_error)?;
        {
          let mut table = txn.open_table(TABLE).map_err(backend_error)?;
          table.remove(key).map_err(backend_error)?;
        }
        txn.commit().map_err(backend_error)?;
        self.bump_revision();
        Ok(())
      }
    }
  }

  pub fn snapshot(&self) -> Result<Vec<PersistentStorageSnapshotEntry>, PersistentStorageError> {
    self
      .raw_snapshot()
      .map(|entries| entries.into_iter().map(snapshot_entry).collect())
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

  fn raw_values(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>, PersistentStorageError> {
    match &self.backend {
      PersistentStorageBackend::Memory(values) => {
        let values = values.read();
        Ok(keys.iter().map(|key| values.get(key).cloned()).collect())
      }
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_read().map_err(backend_error)?;
        let table = txn.open_table(TABLE).map_err(backend_error)?;
        keys
          .iter()
          .map(|key| {
            table
              .get(key.as_str())
              .map_err(backend_error)
              .map(|value| value.map(|value| value.value().to_vec()))
          })
          .collect()
      }
    }
  }

  fn raw_snapshot(&self) -> Result<Vec<(String, Vec<u8>)>, PersistentStorageError> {
    let mut entries = match &self.backend {
      PersistentStorageBackend::Memory(values) => values
        .read()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<_>>(),
      PersistentStorageBackend::Redb(db) => {
        let txn = db.begin_read().map_err(backend_error)?;
        let table = txn.open_table(TABLE).map_err(backend_error)?;
        let mut entries = Vec::new();
        for entry in table.iter().map_err(backend_error)? {
          let (key, value) = entry.map_err(backend_error)?;
          entries.push((key.value().to_owned(), value.value().to_vec()));
        }
        entries
      }
    };
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
  }

  fn bump_revision(&self) {
    self.revision.fetch_add(1, Ordering::Relaxed);
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

fn snapshot_entry((key, value): (String, Vec<u8>)) -> PersistentStorageSnapshotEntry {
  let byte_len = value.len();
  let (type_name, full_type_name, value) = describe_persistent_value(&value);
  PersistentStorageSnapshotEntry {
    key,
    type_name,
    full_type_name,
    value,
    byte_len,
  }
}

fn describe_persistent_value(bytes: &[u8]) -> (String, String, String) {
  match bytes.first().copied() {
    Some(TYPE_BOOL) => describe_as::<bool>("bool", bytes),
    Some(TYPE_STRING) => describe_string(bytes),
    Some(TYPE_I8) => describe_as::<i8>("i8", bytes),
    Some(TYPE_I16) => describe_as::<i16>("i16", bytes),
    Some(TYPE_I32) => describe_as::<i32>("i32", bytes),
    Some(TYPE_I64) => describe_as::<i64>("i64", bytes),
    Some(TYPE_ISIZE) => describe_as::<isize>("isize", bytes),
    Some(TYPE_U8) => describe_as::<u8>("u8", bytes),
    Some(TYPE_U16) => describe_as::<u16>("u16", bytes),
    Some(TYPE_U32) => describe_as::<u32>("u32", bytes),
    Some(TYPE_U64) => describe_as::<u64>("u64", bytes),
    Some(TYPE_USIZE) => describe_as::<usize>("usize", bytes),
    Some(TYPE_F32) => describe_as::<f32>("f32", bytes),
    Some(TYPE_F64) => describe_as::<f64>("f64", bytes),
    Some(TYPE_I128) => describe_as::<i128>("i128", bytes),
    Some(TYPE_U128) => describe_as::<u128>("u128", bytes),
    Some(TYPE_CHAR) => describe_char(bytes),
    Some(TYPE_STRUCT) => {
      describe_struct(bytes).unwrap_or_else(|| ("struct".to_owned(), "struct".to_owned(), "<invalid>".to_owned()))
    }
    Some(tag) => (
      "raw".to_owned(),
      "raw".to_owned(),
      format!("{} bytes, tag {}", bytes.len(), tag),
    ),
    None => ("raw".to_owned(), "raw".to_owned(), "empty".to_owned()),
  }
}

fn describe_as<T>(type_name: &str, bytes: &[u8]) -> (String, String, String)
where
  T: PersistentValue + Display,
{
  (
    type_name.to_owned(),
    type_name.to_owned(),
    T::decode_persistent_value(bytes)
      .map(|value| value.to_string())
      .unwrap_or_else(|| "<invalid>".to_owned()),
  )
}

fn describe_string(bytes: &[u8]) -> (String, String, String) {
  (
    "String".to_owned(),
    "String".to_owned(),
    String::decode_persistent_value(bytes)
      .map(|value| format!("{value:?}"))
      .unwrap_or_else(|| "<invalid utf-8>".to_owned()),
  )
}

fn describe_char(bytes: &[u8]) -> (String, String, String) {
  (
    "char".to_owned(),
    "char".to_owned(),
    char::decode_persistent_value(bytes)
      .map(|value| format!("{value:?}"))
      .unwrap_or_else(|| "<invalid>".to_owned()),
  )
}

fn describe_struct(bytes: &[u8]) -> Option<(String, String, String)> {
  let mut offset = 1;
  let type_name = read_len_bytes(bytes, &mut offset)?;
  let type_name = String::from_utf8(type_name.to_vec()).ok()?;
  let field_count = read_u32(bytes, &mut offset)?;
  let mut fields = Vec::with_capacity(field_count as usize);
  for index in 0..field_count {
    let field = read_len_bytes(bytes, &mut offset)?;
    let (_, _, value) = describe_persistent_value(field);
    fields.push((index, value));
  }

  let labels = read_optional_struct_field_names(bytes, &mut offset, field_count as usize)?;
  if offset != bytes.len() {
    return None;
  }

  let labels = labels.unwrap_or_else(|| {
    (0..field_count)
      .map(|index| format!("field{index}"))
      .collect::<Vec<_>>()
  });
  let fields = fields
    .into_iter()
    .zip(labels)
    .map(|((_, value), label)| format!("{label}: {value}"))
    .collect::<Vec<_>>()
    .join(", ");
  let short_type = type_name.rsplit("::").next().unwrap_or(&type_name).to_owned();

  Some((short_type.clone(), type_name, format!("{short_type} {{ {fields} }}")))
}

fn read_optional_struct_field_names(
  bytes: &[u8],
  offset: &mut usize,
  field_count: usize,
) -> Option<Option<Vec<String>>> {
  if *offset == bytes.len() {
    return Some(None);
  }

  if !bytes.get(*offset..)?.starts_with(STRUCT_FIELD_NAMES_MARKER) {
    return None;
  }
  *offset += STRUCT_FIELD_NAMES_MARKER.len();

  let stored_field_count = read_u32(bytes, offset)? as usize;
  if stored_field_count != field_count {
    return None;
  }

  let mut names = Vec::with_capacity(field_count);
  for _ in 0..field_count {
    let name = read_len_bytes(bytes, offset)?;
    names.push(String::from_utf8(name.to_vec()).ok()?);
  }
  Some(Some(names))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Option<u32> {
  let end = offset.checked_add(4)?;
  let value = u32::from_le_bytes(bytes.get(*offset..end)?.try_into().ok()?);
  *offset = end;
  Some(value)
}

fn read_len_bytes<'a>(bytes: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
  let len = read_u32(bytes, offset)? as usize;
  let end = offset.checked_add(len)?;
  let value = bytes.get(*offset..end)?;
  *offset = end;
  Some(value)
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

#[doc(hidden)]
pub mod derive_support {
  use super::{
    IntoPersistentValue, PersistentValue, STRUCT_FIELD_NAMES_MARKER, TYPE_STRUCT, read_optional_struct_field_names,
  };

  pub fn begin_struct(type_name: &str, field_count: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.push(TYPE_STRUCT);
    push_len_bytes(&mut bytes, type_name.as_bytes());
    bytes.extend_from_slice(&(field_count as u32).to_le_bytes());
    bytes
  }

  pub fn push_field<T: IntoPersistentValue>(bytes: &mut Vec<u8>, value: T) {
    let field = value.encode_persistent_value();
    push_len_bytes(bytes, &field);
  }

  pub fn push_field_names(bytes: &mut Vec<u8>, names: &[&str]) {
    bytes.extend_from_slice(STRUCT_FIELD_NAMES_MARKER);
    bytes.extend_from_slice(&(names.len() as u32).to_le_bytes());
    for name in names {
      push_len_bytes(bytes, name.as_bytes());
    }
  }

  fn push_len_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u32).to_le_bytes());
    bytes.extend_from_slice(value);
  }

  pub struct DecodeCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
    field_count: usize,
    remaining_fields: usize,
  }

  impl<'a> DecodeCursor<'a> {
    pub fn new(bytes: &'a [u8], expected_type: &str, expected_fields: usize) -> Option<Self> {
      if bytes.first().copied()? != TYPE_STRUCT {
        return None;
      }
      let mut cursor = Self {
        bytes,
        offset: 1,
        field_count: 0,
        remaining_fields: 0,
      };
      let stored_type = cursor.read_len_bytes()?;
      if stored_type != expected_type.as_bytes() {
        return None;
      }
      let stored_fields = cursor.read_u32()? as usize;
      if stored_fields != expected_fields {
        return None;
      }
      cursor.field_count = stored_fields;
      cursor.remaining_fields = stored_fields;
      Some(cursor)
    }

    pub fn read_field<T: PersistentValue>(&mut self) -> Option<T> {
      if self.remaining_fields == 0 {
        return None;
      }
      let bytes = self.read_len_bytes()?;
      self.remaining_fields -= 1;
      T::decode_persistent_value(bytes)
    }

    pub fn finish(mut self) -> Option<()> {
      if self.remaining_fields != 0 {
        return None;
      }
      read_optional_struct_field_names(self.bytes, &mut self.offset, self.field_count)?;
      if self.offset == self.bytes.len() {
        Some(())
      } else {
        None
      }
    }

    fn read_u32(&mut self) -> Option<u32> {
      let end = self.offset.checked_add(4)?;
      let bytes = self.bytes.get(self.offset..end)?;
      self.offset = end;
      Some(u32::from_le_bytes(bytes.try_into().ok()?))
    }

    fn read_len_bytes(&mut self) -> Option<&'a [u8]> {
      let len = self.read_u32()? as usize;
      let end = self.offset.checked_add(len)?;
      let bytes = self.bytes.get(self.offset..end)?;
      self.offset = end;
      Some(bytes)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{PersistentStorage, PersistentWrite};

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

  #[test]
  fn memory_bulk_roundtrips_values_in_key_order() {
    let storage = PersistentStorage::memory();

    storage
      .write_bulk([("first", 1_u32), ("second", 2_u32), ("third", 3_u32)])
      .unwrap();

    let batch = storage.read_bulk(["third", "missing", "first"]).unwrap();
    let values = batch.values::<u32, _, _>(["third", "missing", "first"]);

    assert_eq!(values, vec![Some(3), None, Some(1)]);
  }

  #[test]
  fn memory_bulk_writes_mixed_value_types() {
    let storage = PersistentStorage::memory();

    storage
      .write_bulk([
        PersistentWrite::new("name", "Ada"),
        PersistentWrite::new("count", 2_u64),
        PersistentWrite::new("enabled", true),
      ])
      .unwrap();

    assert_eq!(storage.value::<String>("name"), Some("Ada".to_owned()));
    assert_eq!(storage.value::<u64>("count"), Some(2));
    assert_eq!(storage.value::<bool>("enabled"), Some(true));
  }

  #[test]
  fn memory_bulk_reads_mixed_value_types() {
    let storage = PersistentStorage::memory();

    storage
      .write_bulk([
        PersistentWrite::new("name", "Ada"),
        PersistentWrite::new("count", 2_u64),
        PersistentWrite::new("enabled", true),
      ])
      .unwrap();

    let batch = storage.read_bulk(["name", "count", "enabled", "missing"]).unwrap();

    assert_eq!(batch.value::<String>("name"), Some("Ada".to_owned()));
    assert_eq!(batch.value::<u64>("count"), Some(2));
    assert_eq!(batch.value::<bool>("enabled"), Some(true));
    assert_eq!(batch.value::<bool>("missing"), None);
  }

  #[test]
  fn snapshot_lists_keys_sorted_with_decoded_values() {
    let storage = PersistentStorage::memory();

    storage.set_value("name", "Ada").unwrap();
    storage.set_value("count", 42_u32).unwrap();
    storage.set_value("enabled", true).unwrap();

    let snapshot = storage.snapshot().unwrap();

    assert_eq!(
      snapshot
        .iter()
        .map(|entry| (entry.key.as_str(), entry.type_name.as_str(), entry.value.as_str()))
        .collect::<Vec<_>>(),
      vec![
        ("count", "u32", "42"),
        ("enabled", "bool", "true"),
        ("name", "String", "\"Ada\""),
      ]
    );
    assert_eq!(storage.revision(), 3);
  }
}
