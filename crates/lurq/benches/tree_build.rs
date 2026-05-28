use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lurq::{
  components::{Column, Rect, Row, Spacer, Stack, Text},
  layout::{Alignment, StackAlignment},
  node::Element,
};

fn build_flat(count: usize) -> Element {
  Column::with(
    0.0,
    Alignment::Start,
    (0..count).map(|_| Rect::new(100.0, 20.0).fill("#334155")),
  )
  .into()
}

fn build_deep(depth: usize) -> Element {
  let mut el: Element = Rect::new(10.0, 10.0).fill("#3b82f6").into();
  for _ in 0..depth {
    el = Column::new().child(el).pad(4.0).into();
  }
  el
}

fn build_wide_row(count: usize) -> Element {
  Row::with(
    4.0,
    Alignment::Center,
    (0..count).map(|i| -> Element {
      if i % 3 == 0 {
        Text::new(&format!("Label {i}")).into()
      } else if i % 3 == 1 {
        Rect::new(60.0, 30.0).fill("#22c55e").rounded(4.0).into()
      } else {
        Spacer::new().into()
      }
    }),
  )
  .into()
}

fn build_stacked(count: usize) -> Element {
  Stack::with(
    StackAlignment::Center,
    (0..count).map(|i| {
      let s = (count - i) as f32 * 20.0;
      Rect::new(s, s).fill("#3b82f6").rounded(4.0)
    }),
  )
  .into()
}

fn bench_tree_build(c: &mut Criterion) {
  let mut group = c.benchmark_group("tree_build");

  for count in [10, 50, 100, 500, 1000] {
    group.bench_with_input(BenchmarkId::new("flat_rects", count), &count, |b, &count| {
      b.iter(|| build_flat(count));
    });
  }

  for depth in [5, 10, 20, 50, 100] {
    group.bench_with_input(BenchmarkId::new("deep_nesting", depth), &depth, |b, &depth| {
      b.iter(|| build_deep(depth));
    });
  }

  for count in [10, 50, 100, 500] {
    group.bench_with_input(BenchmarkId::new("wide_row", count), &count, |b, &count| {
      b.iter(|| build_wide_row(count));
    });
  }

  for count in [5, 10, 20, 50] {
    group.bench_with_input(BenchmarkId::new("stacked", count), &count, |b, &count| {
      b.iter(|| build_stacked(count));
    });
  }

  group.finish();
}

criterion_group!(benches, bench_tree_build);
criterion_main!(benches);
