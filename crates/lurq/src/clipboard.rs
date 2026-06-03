pub fn copy_to_clipboard(text: impl AsRef<str>) -> bool {
  let Ok(mut clipboard) = arboard::Clipboard::new() else {
    return false;
  };
  clipboard.set_text(text.as_ref().to_owned()).is_ok()
}

pub fn set_clipboard_text(text: impl AsRef<str>) -> bool {
  copy_to_clipboard(text)
}

pub fn read_from_clipboard() -> Option<String> {
  let mut clipboard = arboard::Clipboard::new().ok()?;
  clipboard.get_text().ok()
}

pub fn clipboard_text() -> Option<String> {
  read_from_clipboard()
}

pub fn clear_clipboard() -> bool {
  copy_to_clipboard("")
}
