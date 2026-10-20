//! What a clipboard holds, as a paste delivers it: the kinds it reports and
//! the checks on an image.

use std::path::PathBuf;

use lurq::app::events::{ClipboardContent, ClipboardImage, ClipboardImageError};

#[test]
fn the_kinds_follow_what_the_content_holds() {
  let empty = ClipboardContent::empty();
  assert!(empty.is_empty());
  assert!(!empty.kinds().any());

  let text = ClipboardContent::text("hello");
  assert_eq!(text.text_content(), Some("hello"));
  assert!(text.kinds().text && !text.kinds().image && !text.kinds().files);

  let image = ClipboardImage::from_rgba(1, 1, vec![0, 0, 0, 255]).unwrap();
  let both = ClipboardContent::image(image.clone()).with_text("a screenshot");
  let kinds = both.kinds();
  assert!(kinds.text && kinds.image && !kinds.files, "text and an image at once");
  assert_eq!(both.image_content(), Some(&image));

  let files = ClipboardContent::files([PathBuf::from("a.png"), PathBuf::from("b.png")]);
  assert!(files.kinds().files);
  assert_eq!(files.file_paths().len(), 2);
  let (text, image, paths) = files.into_parts();
  assert!(text.is_none() && image.is_none());
  assert_eq!(paths.len(), 2);
}

#[test]
fn empty_text_counts_as_no_text() {
  let content = ClipboardContent::text("");
  assert!(content.is_empty());
  assert_eq!(content.text_content(), None);
  assert!(!content.kinds().text);
}

#[test]
fn a_clipboard_image_checks_its_size_and_bytes() {
  let image = ClipboardImage::from_rgba(2, 2, vec![7; 16]).unwrap();
  assert_eq!((image.width(), image.height()), (2, 2));
  assert_eq!(image.rgba().len(), 16);
  assert_eq!(
    format!("{image:?}"),
    "ClipboardImage { width: 2, height: 2, bytes: 16 }"
  );

  assert_eq!(
    ClipboardImage::from_rgba(0, 2, Vec::new()).unwrap_err(),
    ClipboardImageError::EmptySize { width: 0, height: 2 }
  );
  assert_eq!(
    ClipboardImage::from_rgba(2, 2, vec![0; 15]).unwrap_err(),
    ClipboardImageError::WrongLength {
      expected: 16,
      actual: 15
    }
  );
}

#[cfg(feature = "clipboard")]
#[test]
fn a_clipboard_image_encodes_to_png() {
  let image = ClipboardImage::from_rgba(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).unwrap();
  let png = image.to_png().expect("encodes");
  assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
  #[cfg(feature = "image")]
  {
    let decoded = lurq::images::ImageData::from_bytes(&png).expect("decodes back");
    assert_eq!((decoded.width(), decoded.height()), (2, 1));
    assert_eq!(decoded.data(), image.rgba());
  }
}
