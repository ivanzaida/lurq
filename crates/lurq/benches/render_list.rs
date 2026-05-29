use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lurq::{
  app::runtime::Tree,
  components::{Column, Rect, Row, Text},
  layout::Alignment,
  node::Element,
};

fn sidebar_with_content(item_count: usize) -> Element {
  let sidebar: Element = Column::with(
    0.0,
    Alignment::Start,
    (0..11).map(|i| {
      Row::with(
        0.0,
        Alignment::Center,
        vec![
          Element::from(Rect::new(3.0, 38.0).fill("#3b82f6")),
          Element::from(Text::new(if i == 0 { "Layout" } else { "Item" })),
        ],
      )
      .fill(if i == 0 { "#1e3a5f" } else { "#1e293b" })
      .width(200.0)
    }),
  )
  .fill("#1e293b")
  .into();

  let content: Element = Column::with(
    8.0,
    Alignment::Start,
    (0..item_count).map(|i| {
      Row::with(
        8.0,
        Alignment::Center,
        vec![
          Element::from(Text::new(&format!("Label {i}"))),
          Element::from(Rect::new(60.0, 30.0).fill("#334155").rounded(4.0)),
          Element::from(Rect::new(200.0, 30.0).fill("#0f172a").rounded(4.0)),
        ],
      )
    }),
  )
  .pad(24.0)
  .into();

  Row::with(0.0, Alignment::Start, vec![sidebar, content])
    .fill("#0f172a")
    .size(1200.0, 800.0)
    .into()
}

fn bench_full_pass(c: &mut Criterion) {
  let mut group = c.benchmark_group("full_pass");

  for count in [10, 50, 100, 200] {
    group.bench_with_input(BenchmarkId::new("sidebar_content", count), &count, |b, &count| {
      let mut rt = Tree::new();
      rt.resize(1200, 800);
      rt.set_root(sidebar_with_content(count));
      rt.rebuild();
      b.iter(|| {
        rt.set_root(sidebar_with_content(count));
        rt.rebuild();
      });
    });
  }

  group.finish();
}

fn bench_rebuild_no_change(c: &mut Criterion) {
  let mut group = c.benchmark_group("rebuild_no_change");

  for count in [10, 50, 100] {
    group.bench_with_input(BenchmarkId::new("sidebar_content", count), &count, |b, &count| {
      let mut rt = Tree::new();
      rt.resize(1200, 800);
      rt.set_root(sidebar_with_content(count));
      rt.rebuild();
      b.iter(|| {
        rt.rebuild();
      });
    });
  }

  group.finish();
}

criterion_group!(benches, bench_full_pass, bench_rebuild_no_change);
criterion_main!(benches);
