use std::{fmt, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogOperation {
  OpenFile,
  OpenFiles,
  OpenFolder,
  SaveFile,
}

impl FileDialogOperation {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::OpenFile => "open_file",
      Self::OpenFiles => "open_files",
      Self::OpenFolder => "open_folder",
      Self::SaveFile => "save_file",
    }
  }

  pub(crate) fn parse(value: &str) -> Result<Self, FileDialogError> {
    match value {
      "open_file" => Ok(Self::OpenFile),
      "open_files" => Ok(Self::OpenFiles),
      "open_folder" => Ok(Self::OpenFolder),
      "save_file" => Ok(Self::SaveFile),
      _ => Err(FileDialogError::WrongOperation),
    }
  }
}

/// Advisory native picker metadata, never a substitute for app validation.
#[derive(Clone, Debug)]
pub struct FileDialogRequest {
  pub operation: FileDialogOperation,
  pub title: String,
  pub filters: Vec<(String, Vec<String>)>,
  pub suggested_name: Option<String>,
}

impl FileDialogRequest {
  pub fn new(operation: FileDialogOperation) -> Self {
    Self {
      operation,
      title: String::new(),
      filters: Vec::new(),
      suggested_name: None,
    }
  }

  pub(crate) fn validate(&self) -> Result<(), FileDialogError> {
    if self.title.len() > 4096
      || self.filters.len() > 32
      || self.suggested_name.as_ref().is_some_and(|name| name.len() > 4096)
      || self.filters.iter().any(|(name, extensions)| {
        name.len() > 256 || extensions.len() > 32 || extensions.iter().any(|ext| ext.len() > 64)
      })
    {
      return Err(FileDialogError::InvalidMetadata);
    }
    Ok(())
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDialogSelection {
  pub paths: Vec<PathBuf>,
  /// Explicit MCP intent, not proof that a target exists or is safe to replace.
  /// The application must enforce overwrite/TOCTOU/atomic-write policy.
  pub overwrite: bool,
}

impl FileDialogSelection {
  pub(crate) fn validate(&self, operation: FileDialogOperation) -> Result<(), FileDialogError> {
    let max = if operation == FileDialogOperation::OpenFiles {
      64
    } else {
      1
    };
    if self.paths.is_empty()
      || self.paths.len() > max
      || self.paths.iter().any(|path| {
        !path.is_absolute() || path.as_os_str().len() > 32768 || path.as_os_str().as_encoded_bytes().contains(&0)
      })
      || (operation != FileDialogOperation::SaveFile && self.overwrite)
    {
      return Err(FileDialogError::InvalidSelection);
    }
    Ok(())
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogError {
  Unavailable,
  Cancelled,
  WindowClosed,
  StaleRequest,
  WrongOperation,
  TooManyPending,
  InvalidMetadata,
  InvalidSelection,
}

impl fmt::Display for FileDialogError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{self:?}")
  }
}
impl std::error::Error for FileDialogError {}
