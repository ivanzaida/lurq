use lurq::{
  app::ctx::Ctx,
  core::{batch, Signal},
};

#[test]
fn batch_coalesces_dirty_marking() {
  let mut ctx = Ctx::new_root();
  let sig = ctx.signal(0_i32);
  assert!(ctx.is_dirty());

  batch(|| {
    sig.set(1);
    sig.set(2);
    sig.set(3);
  });
  assert_eq!(sig.get(), 3);
}

#[test]
fn batch_with_multiple_signals() {
  let mut ctx = Ctx::new_root();
  let a = ctx.signal(0_i32);
  let b = ctx.signal(0_i32);
  batch(|| {
    a.set(10);
    b.set(20);
  });
  assert_eq!(a.get(), 10);
  assert_eq!(b.get(), 20);
}

#[test]
fn empty_batch_is_noop() {
  batch(|| {});
}

#[test]
fn batch_returns_nothing_and_completes() {
  let s = Signal::new(0);
  batch(|| {
    s.set(42);
  });
  assert_eq!(s.get(), 42);
}
