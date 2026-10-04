//! One admission rule for both backend-owned presentation lease graphs.
use super::MAX_PRESENTATION_BYTES;
#[derive(Default)]
pub(crate) struct TargetCharge {
  bytes: usize,
  shared: std::collections::HashSet<usize>,
}
impl TargetCharge {
  pub(crate) fn add(&mut self, bytes: usize) {
    self.bytes = self.bytes.saturating_add(bytes);
  }
  pub(crate) fn shared(&mut self, identity: usize, bytes: usize) {
    if self.shared.insert(identity) {
      self.add(bytes);
    }
  }
  pub(crate) fn bytes(&self) -> usize {
    self.bytes
  }
}
pub(crate) fn replacement_admitted(owned: usize, additional: usize, workspace: usize) -> bool {
  owned
    .checked_add(additional)
    .and_then(|n| n.checked_add(workspace))
    .is_some_and(|n| n <= MAX_PRESENTATION_BYTES as usize)
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn live_and_retired_shared_artwork_is_charged_once_but_distinct_targets_are_not() {
    let bytes = 3840 * 2160 * 4;
    let mut charge = TargetCharge::default();
    charge.add(bytes);
    charge.add(bytes); // distinct complete front and replacement
    charge.shared(100, bytes);
    charge.shared(100, bytes); // old artwork: live + retired lease
    charge.shared(101, bytes); // newly prepared artwork
    assert_eq!(charge.bytes(), 132_710_400);
    assert!(replacement_admitted(charge.bytes(), 0, 32 * 1024 * 1024));
    charge.add(bytes);
    charge.add(bytes); // two in-flight pool targets still count
    assert!(replacement_admitted(charge.bytes(), 0, 32 * 1024 * 1024));
    assert!(replacement_admitted(charge.bytes(), bytes, 32 * 1024 * 1024));
    assert!(!replacement_admitted(charge.bytes(), 2 * bytes, 32 * 1024 * 1024));
    assert!(!replacement_admitted(usize::MAX, 1, 0));
    assert!(!replacement_admitted(0, usize::MAX, 1));
  }
}
