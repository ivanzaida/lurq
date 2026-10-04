use super::*;
fn canvas(software: bool) -> CanvasHandle {
  let c = CanvasHandle::new();
  let mut s = c.inner.lock();
  s.attached = true;
  s.software = software;
  s.metrics.pixel_width = 8;
  s.metrics.pixel_height = 8;
  s.metrics.size = Size::new(8., 8.);
  if software {
    s.pixels = Pixmap::new(8, 8);
  }
  drop(s);
  c
}
fn snapshot(c: &CanvasHandle) -> Vec<u8> {
  c.snapshot().try_take().unwrap().unwrap().rgba
}
#[test]
fn complete_front_survives_partial_recording_abort_and_superseded_commit() {
  let c = canvas(true);
  let d = c.context_2d();
  d.set_fill_style("#ff0000");
  d.fill_rect(0., 0., 8., 8.);
  let before = snapshot(&c);
  let first = d.begin_presentation().unwrap();
  d.reset();
  d.set_fill_style("#0000ff");
  d.fill_rect(0., 0., 4., 8.);
  assert_eq!(snapshot(&c), before);
  let latest = d.begin_presentation().unwrap();
  assert_eq!(d.commit_presentation(first), Err(CanvasError::StateLimit));
  d.abort_presentation(latest).unwrap();
  assert_eq!(snapshot(&c), before);
  let next = d.begin_presentation().unwrap();
  d.reset();
  d.set_fill_style("#00ff00");
  d.fill_rect(0., 0., 8., 8.);
  d.commit_presentation(next).unwrap();
  assert_ne!(snapshot(&c), before);
}
#[test]
fn artwork_excludes_later_overlay_and_camera_draw_never_reads_back() {
  let c = canvas(true);
  let d = c.context_2d();
  let token = d.begin_presentation().unwrap();
  d.reset();
  d.set_fill_style("#ff0000");
  d.fill_rect(0., 0., 8., 8.);
  d.capture_artwork().unwrap();
  d.set_fill_style("#0000ff");
  d.fill_rect(0., 0., 4., 8.);
  d.commit_presentation(token).unwrap();
  assert_eq!(&snapshot(&c)[..4], &[0, 0, 255, 255]);
  let token = d.begin_presentation().unwrap();
  d.reset();
  d.draw_retained_artwork(Transform2D::translate(2., 0.)).unwrap();
  d.commit_presentation(token).unwrap();
  let pixels = snapshot(&c);
  assert_eq!(&pixels[2 * 4..3 * 4], &[255, 0, 0, 255]);
  assert_eq!(&pixels[..4], &[0, 0, 0, 0]);
  d.forget_artwork();
  let token = d.begin_presentation().unwrap();
  assert_eq!(
    d.draw_retained_artwork(Transform2D::IDENTITY),
    Err(CanvasError::StateLimit)
  );
  d.abort_presentation(token).unwrap();
}
#[test]
fn reset_preserves_transaction_barriers_and_latest_camera_order() {
  let c = canvas(false);
  let d = c.context_2d();
  let first = d.begin_presentation().unwrap();
  d.fill_rect(0., 0., 8., 8.);
  d.capture_artwork().unwrap();
  d.commit_presentation(first).unwrap();
  let latest = d.begin_presentation().unwrap();
  d.reset();
  d.draw_retained_artwork(Transform2D::IDENTITY).unwrap();
  d.commit_presentation(latest).unwrap();
  let mut batch = c.take_batch().unwrap();
  let commands = batch.take_commands();
  let latest_begin = commands
    .iter()
    .position(|c| matches!(c,gpu::Command::BeginPresentation(t, _) if *t==latest))
    .unwrap();
  assert!(matches!(commands[latest_begin + 1], gpu::Command::Clear));
  assert!(matches!(commands[latest_begin + 2], gpu::Command::RetainedArtwork(_)));
  assert!(matches!(commands[latest_begin+3],gpu::Command::CommitPresentation(t, _) if t==latest));
  assert!(
    commands[..latest_begin]
      .iter()
      .any(|c| matches!(c,gpu::Command::BeginPresentation(t, _) if *t==first))
  );
  assert!(!c.presentation_current(first));
  assert!(c.presentation_current(latest));
  batch.submit();
}
#[test]
fn presentation_requires_balanced_layers_and_detach_releases_artwork() {
  let c = canvas(true);
  let d = c.context_2d();
  let token = d.begin_presentation().unwrap();
  d.begin_layer(1., BlendMode::Normal).unwrap();
  assert_eq!(d.commit_presentation(token), Err(CanvasError::UnbalancedLayer));
  d.abort_presentation(token).unwrap();
  assert!(c.inner.lock().software_layers.is_empty());
  let recovered = d.begin_presentation().unwrap();
  d.reset();
  d.set_fill_style("#00ff00");
  d.fill_rect(0., 0., 8., 8.);
  d.commit_presentation(recovered).unwrap();
  assert_eq!(&snapshot(&c)[..4], &[0, 255, 0, 255]);
  c.detach();
  assert!(c.inner.lock().artwork_pixels.is_none());
  assert!(!c.presentation_current(token));
}
#[test]
fn four_4k_targets_fit_the_separate_presentation_allowance_without_pixel_cap_widening() {
  assert!(presentation_admitted(3840 * 2160));
  assert_eq!(3840u64 * 2160 * 16, 132_710_400);
  assert!(!presentation_admitted(MAX_PIXELS));
  assert!(!presentation_admitted(MAX_PIXELS + 1));
  assert!(!presentation_admitted(0));
  assert!(!presentation_admitted(u64::MAX));
}

#[test]
fn pending_readback_keeps_complete_front_revision_and_refused_begin_keeps_current_token() {
  let c = canvas(true);
  let d = c.context_2d();
  d.set_fill_style("#ff0000");
  d.fill_rect(0., 0., 8., 8.);
  let before = c.snapshot().try_take().unwrap().unwrap();
  let token = d.begin_presentation().unwrap();
  d.reset();
  d.set_fill_style("#0000ff");
  d.fill_rect(0., 0., 8., 8.);
  let pending = c.snapshot().try_take().unwrap().unwrap();
  assert_eq!((pending.rgba, pending.revision), (before.rgba, before.revision));
  let serial = c.inner.lock().presentation_serial;
  {
    let mut s = c.inner.lock();
    s.metrics.pixel_width = u32::MAX;
  }
  assert_eq!(d.begin_presentation(), Err(CanvasError::SurfaceTooLarge));
  assert_eq!(c.inner.lock().presentation_serial, serial);
  assert!(c.presentation_current(token));
  d.abort_presentation(token).unwrap();
}
