use super::{Harness, Ticker, Waiter};

#[test]
fn offstage_stream_keeps_running_and_stops_when_removed() {
  let mut harness = Harness::mount::<Ticker>();
  harness.drive();

  harness.set_active(false);
  let went_offstage = harness.probe.iterations();
  harness.drive();
  assert!(harness.probe.iterations() > went_offstage);
  assert_eq!(harness.probe.dropped(), 0);

  harness.set_show(false);
  let removed = harness.probe.iterations();
  harness.drive();

  assert_eq!(harness.probe.iterations(), removed);
  assert_eq!(harness.probe.dropped(), 1);
}

#[test]
fn offstage_future_finishes_and_its_result_applies_once_active_again() {
  let mut harness = Harness::mount::<Waiter>();
  harness.drive();
  harness.set_active(false);

  harness.probe.release();
  harness.drive();
  harness.tree.tick_futures();
  assert_eq!(harness.probe.effects(), 1);

  harness.set_active(true);
  assert_eq!(harness.text(), Some("pending"));
  harness.tree.tick_futures();

  assert_eq!(harness.text(), Some("done:1"));
  assert_eq!(harness.probe.starts(), 1);
}
