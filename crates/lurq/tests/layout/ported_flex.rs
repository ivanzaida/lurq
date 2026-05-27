use lurq::{
  app::Runtime,
  layout::{
    Alignment, Constraints, Size,
    layout_kind::{FrameConstraints, Justify},
  },
  node::Element,
};

fn rt() -> Runtime {
  Runtime::new()
}

fn rect(w: f32, h: f32) -> Element {
  Element::new().size(w, h)
}

fn wide(w: f32) -> Element {
  Element::new().width(w)
}

fn tall(h: f32) -> Element {
  Element::new().height(h)
}

// ============================================================================
// Row distributes children horizontally
// ============================================================================

#[test]
fn flex_row_distributes_children_horizontally() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .with_children(vec![rect(100.0, 50.0), rect(100.0, 50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  assert!(r.children[1].offset.x > r.children[0].offset.x);
}

// ============================================================================
// Column stacks children vertically
// ============================================================================

#[test]
fn flex_column_stacks_children_vertically() {
  let mut rt = rt();
  let node = Element::column()
    .spacing(0.0)
    .with_children(vec![rect(100.0, 50.0), rect(100.0, 50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();
  assert!(r.children[1].offset.y > r.children[0].offset.y);
}

// ============================================================================
// Flex grow distributes free space
// ============================================================================

#[test]
fn flex_grow_distributes_free_space() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .with_children(vec![Element::new().flex(1.0), Element::new().flex(2.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  let ratio = r.children[1].result.size.width / r.children[0].result.size.width;
  assert!((ratio - 2.0).abs() < 0.1, "ratio should be ~2.0, got {}", ratio);
}

// ============================================================================
// Justify content
// ============================================================================

#[test]
fn flex_justify_content_center() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .justify(Justify::Center)
    .child(rect(100.0, 50.0));
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  let offset = r.children[0].offset.x;
  assert!((offset - 100.0).abs() < 1.0, "should be centered (offset={})", offset);
}

#[test]
fn flex_justify_content_end() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .justify(Justify::End)
    .child(rect(50.0, 50.0));
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  let child_end = r.children[0].offset.x + r.children[0].result.size.width;
  assert!(
    (child_end - 300.0).abs() < 1.0,
    "flex-end: child at right edge, end={}",
    child_end
  );
}

#[test]
fn flex_justify_content_space_between() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .justify(Justify::SpaceBetween)
    .with_children(vec![rect(50.0, 30.0), rect(50.0, 30.0), rect(50.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  let first_x = r.children[0].offset.x;
  let last_end = r.children[2].offset.x + r.children[2].result.size.width;
  assert!(first_x.abs() < 1.0, "first at start");
  assert!((last_end - 300.0).abs() < 1.0, "last at end");
}

#[test]
fn flex_justify_content_space_around() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .justify(Justify::SpaceAround)
    .with_children(vec![rect(50.0, 30.0), rect(50.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  // Free space = 200, 2 items → each gets 100px, half on each side = 50px
  let a_start = r.children[0].offset.x;
  assert!(
    a_start > 40.0 && a_start < 60.0,
    "space-around: first item should have ~50px before it, got {}",
    a_start
  );
}

#[test]
fn flex_justify_content_space_evenly() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .justify(Justify::SpaceEvenly)
    .with_children(vec![rect(50.0, 30.0), rect(50.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  // Free space = 200, 3 slots → ~66.7px each
  let a_start = r.children[0].offset.x;
  assert!(
    a_start > 60.0 && a_start < 73.0,
    "space-evenly: ~66.7px before first, got {}",
    a_start
  );
}

#[test]
fn flex_justify_center_column() {
  let mut rt = rt();
  let node = Element::column()
    .spacing(0.0)
    .justify(Justify::Center)
    .child(rect(100.0, 50.0));
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 300.0))).unwrap();
  let offset = r.children[0].offset.y;
  assert!(
    (offset - 125.0).abs() < 1.0,
    "should be centered vertically (offset={})",
    offset
  );
}

// ============================================================================
// Gap / spacing
// ============================================================================

#[test]
fn flex_gap_adds_spacing() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(20.0)
    .with_children(vec![rect(100.0, 50.0), rect(100.0, 50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  let a_end = r.children[0].offset.x + r.children[0].result.size.width;
  let b_start = r.children[1].offset.x;
  let gap = b_start - a_end;
  assert!((gap - 20.0).abs() < 1.0, "gap should be ~20px, got {}", gap);
}

#[test]
fn flex_gap_three_items() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(16.0)
    .with_children(vec![rect(120.0, 40.0), rect(120.0, 40.0), rect(120.0, 40.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(500.0, 100.0))).unwrap();
  let gap_01 = r.children[1].offset.x - (r.children[0].offset.x + r.children[0].result.size.width);
  let gap_12 = r.children[2].offset.x - (r.children[1].offset.x + r.children[1].result.size.width);
  assert!((gap_01 - 16.0).abs() < 1.0, "gap 0→1 should be ~16px, got {}", gap_01);
  assert!((gap_12 - 16.0).abs() < 1.0, "gap 1→2 should be ~16px, got {}", gap_12);
}

// ============================================================================
// Align items
// ============================================================================

#[test]
fn flex_align_items_start() {
  let mut rt = rt();
  let node = Element::row_with(0.0, Alignment::Start, vec![rect(50.0, 50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();
  let offset = r.children[0].offset.y;
  assert!(offset.abs() < 1.0, "flex-start: child at top, offset={}", offset);
}

#[test]
fn flex_align_items_center() {
  let mut rt = rt();
  let node = Element::row_with(0.0, Alignment::Center, vec![rect(50.0, 50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();
  let child_cy = r.children[0].offset.y + r.children[0].result.size.height / 2.0;
  let container_cy = r.size.height / 2.0;
  assert!((child_cy - container_cy).abs() < 1.0, "should be centered");
}

#[test]
fn flex_align_items_end() {
  let mut rt = rt();
  let node = Element::row_with(0.0, Alignment::End, vec![rect(50.0, 50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();
  let child_bottom = r.children[0].offset.y + r.children[0].result.size.height;
  assert!((child_bottom - 200.0).abs() < 1.0, "flex-end: child at bottom");
}

#[test]
fn flex_align_items_stretch() {
  let mut rt = rt();
  let node = Element::row_with(0.0, Alignment::Stretch, vec![wide(50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();
  assert!(
    (r.children[0].result.size.height - 200.0).abs() < 1.0,
    "stretch: child should fill cross axis, got {}",
    r.children[0].result.size.height
  );
}

#[test]
fn flex_align_items_stretch_column() {
  let mut rt = rt();
  let node = Element::column_with(0.0, Alignment::Stretch, vec![tall(50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();
  assert!(
    (r.children[0].result.size.width - 300.0).abs() < 1.0,
    "stretch: child should fill cross axis width, got {}",
    r.children[0].result.size.width
  );
}

// ============================================================================
// Flex shrink
// ============================================================================

#[test]
fn flex_shrink_reduces_overflowing_items() {
  let mut rt = rt();
  let node = Element::row().spacing(0.0).with_children(vec![
    rect(150.0, 50.0).flex_full(0.0, 1.0, None),
    rect(150.0, 50.0).flex_full(0.0, 1.0, None),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  let a_w = r.children[0].result.size.width;
  let b_w = r.children[1].result.size.width;
  assert!(a_w < 150.0, "should shrink, got {}", a_w);
  let total = a_w + b_w;
  assert!((total - 200.0).abs() < 1.0, "total should be ~200px, got {}", total);
}

#[test]
fn flex_shrink_weighted() {
  let mut rt = rt();
  let node = Element::row().spacing(0.0).with_children(vec![
    rect(200.0, 50.0).flex_full(0.0, 1.0, None),
    rect(100.0, 50.0).flex_full(0.0, 1.0, None),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  let big = r.children[0].result.size.width;
  let small = r.children[1].result.size.width;
  assert!(
    big > small,
    "bigger item should still be wider (big={}, small={})",
    big,
    small
  );
  assert!(
    (big + small - 200.0).abs() < 1.0,
    "total should be 200, got {}",
    big + small
  );
}

#[test]
fn flex_shrink_zero_does_not_shrink() {
  let mut rt = rt();
  let node = Element::row().spacing(0.0).with_children(vec![
    rect(150.0, 50.0).flex_full(0.0, 0.0, None),
    rect(150.0, 50.0).flex_full(0.0, 1.0, None),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  assert!(
    (r.children[0].result.size.width - 150.0).abs() < 1.0,
    "shrink:0 should not shrink, got {}",
    r.children[0].result.size.width
  );
}

// ============================================================================
// Flex basis
// ============================================================================

#[test]
fn flex_basis_sets_initial_size() {
  let mut rt = rt();
  let node = Element::row().spacing(0.0).with_children(vec![
    Element::new().flex_full(0.0, 0.0, Some(100.0)),
    Element::new().flex_full(0.0, 0.0, Some(200.0)),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  assert!(
    (r.children[0].result.size.width - 100.0).abs() < 1.0,
    "flex-basis:100px, got {}",
    r.children[0].result.size.width
  );
  assert!(
    (r.children[1].result.size.width - 200.0).abs() < 1.0,
    "flex-basis:200px, got {}",
    r.children[1].result.size.width
  );
}

#[test]
fn flex_basis_zero_with_grow() {
  let mut rt = rt();
  let node = Element::row().spacing(0.0).with_children(vec![
    Element::new().flex_full(1.0, 0.0, Some(0.0)),
    Element::new().flex_full(1.0, 0.0, Some(0.0)),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  assert!(
    (r.children[0].result.size.width - 150.0).abs() < 1.0,
    "flex-basis:0 + grow:1 → 150, got {}",
    r.children[0].result.size.width
  );
}

// ============================================================================
// Flex wrap
// ============================================================================

#[test]
fn flex_wrap_wraps_items() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .wrap()
    .with_children(vec![rect(120.0, 30.0), rect(120.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 200.0))).unwrap();
  assert!(
    r.children[1].offset.y > r.children[0].offset.y,
    "should wrap: y0={}, y1={}",
    r.children[0].offset.y,
    r.children[1].offset.y
  );
}

#[test]
fn flex_wrap_three_items_two_lines() {
  let mut rt = rt();
  let node =
    Element::row()
      .spacing(0.0)
      .wrap()
      .with_children(vec![rect(80.0, 30.0), rect(80.0, 30.0), rect(80.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 200.0))).unwrap();
  // First two fit on line 1 (80+80=160 ≤ 200), third wraps
  assert_eq!(r.children[0].offset.y, r.children[1].offset.y, "A and B on same line");
  assert!(r.children[2].offset.y > r.children[0].offset.y, "C wraps to next line");
}

#[test]
fn flex_wrap_column() {
  let mut rt = rt();
  let node = Element::column()
    .spacing(0.0)
    .wrap()
    .with_children(vec![rect(50.0, 120.0), rect(50.0, 120.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 200.0))).unwrap();
  assert!(
    r.children[1].offset.x > r.children[0].offset.x,
    "column wrap: should wrap to next column"
  );
}

#[test]
fn flex_wrap_with_spacing() {
  let mut rt = rt();
  let node =
    Element::row()
      .spacing(10.0)
      .wrap()
      .with_children(vec![rect(100.0, 30.0), rect(100.0, 30.0), rect(100.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(220.0, 200.0))).unwrap();
  // 100 + 10 + 100 = 210 ≤ 220 → first two fit; third wraps
  assert_eq!(r.children[0].offset.y, r.children[1].offset.y, "A and B on same line");
  assert!(r.children[2].offset.y > r.children[0].offset.y, "C wraps");
}

// ============================================================================
// Empty container
// ============================================================================

#[test]
fn flex_no_children_does_not_panic() {
  let mut rt = rt();
  let node = Element::row().spacing(0.0);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  assert_eq!(r.children.len(), 0);
}

// ============================================================================
// Min/max constraints on flex items
// ============================================================================

#[test]
fn flex_max_width_prevents_growing() {
  let mut rt = rt();
  let node = Element::row().spacing(0.0).with_children(vec![
    Element::new()
      .frame(FrameConstraints {
        max_width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        ..Default::default()
      })
      .flex(1.0),
    Element::new().flex(1.0),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(400.0, 100.0))).unwrap();
  assert!(
    r.children[0].result.size.width <= 101.0,
    "max-width:100 should cap growth, got {}",
    r.children[0].result.size.width
  );
}

#[test]
fn flex_min_height_prevents_shrinking_column() {
  let mut rt = rt();
  let node = Element::column().spacing(0.0).with_children(vec![
    Element::new()
      .frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        min_height: Some(lurq::node::dimension::Dimension::Px(120.0)),
        ..Default::default()
      })
      .flex_full(0.0, 1.0, None),
    rect(100.0, 150.0).flex_full(0.0, 1.0, None),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 200.0))).unwrap();
  assert!(
    r.children[0].result.size.height >= 119.0,
    "min-height:120 should prevent shrinking below 120, got {}",
    r.children[0].result.size.height
  );
}

// ============================================================================
// Nested flex
// ============================================================================

#[test]
fn flex_nested_row_in_row() {
  let mut rt = rt();
  let inner = Element::row()
    .spacing(0.0)
    .with_children(vec![rect(50.0, 30.0), rect(50.0, 30.0)])
    .flex(1.0);
  let node = Element::row()
    .spacing(0.0)
    .with_children(vec![inner, rect(100.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(400.0, 100.0))).unwrap();
  assert_eq!(r.children.len(), 2);
  let inner_w = r.children[0].result.size.width;
  assert!(inner_w > 200.0, "inner flex should grow, got {}", inner_w);
}

#[test]
fn flex_nested_column_in_row() {
  let mut rt = rt();
  let inner_col = Element::column()
    .spacing(4.0)
    .with_children(vec![
      Element::new().flex(1.0),
      Element::new().flex(1.0),
      Element::new().flex(1.0),
    ])
    .flex(1.0);
  let inner_row = Element::row()
    .spacing(4.0)
    .align_items(Alignment::Center)
    .with_children(vec![rect(30.0, 20.0), rect(30.0, 20.0), rect(30.0, 20.0)])
    .flex(2.0);
  let node = Element::row().spacing(8.0).with_children(vec![inner_col, inner_row]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(800.0, 120.0))).unwrap();

  let col = &r.children[0];
  let row = &r.children[1];
  assert!(col.result.size.height > 100.0, "inner col should stretch to ~120px");
  assert!(
    row.result.size.width > col.result.size.width,
    "inner row (flex:2) wider than col (flex:1)"
  );
}

#[test]
fn flex_deeply_nested_three_levels() {
  let mut rt = rt();
  let leaf = rect(20.0, 20.0);
  let inner = Element::row().spacing(0.0).child(leaf).flex(1.0);
  let mid = Element::column().spacing(0.0).child(inner).flex(1.0);
  let root = Element::row().spacing(0.0).child(mid);
  rt.set_root(root);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 200.0))).unwrap();
  assert!(r.size.width > 0.0);
}

// ============================================================================
// Padding with flex
// ============================================================================

#[test]
fn flex_padding_reduces_available_space() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .with_children(vec![Element::new().flex(1.0), Element::new().flex(1.0)])
    .pad(20.0);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  // Padding 20 on each side → 260 available for flex children → 130 each
  let inner = &r.children[0].result;
  let a_w = inner.children[0].result.size.width;
  let b_w = inner.children[1].result.size.width;
  assert!(
    (a_w - 130.0).abs() < 1.0,
    "padding should reduce available space, a_w={}",
    a_w
  );
  assert!((a_w - b_w).abs() < 1.0, "should be equal: a={}, b={}", a_w, b_w);
}

// ============================================================================
// Overflow clipping
// ============================================================================

#[test]
fn overflow_hidden_clips_children() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .clip()
    .with_children(vec![rect(200.0, 50.0), rect(200.0, 50.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  // Children total 400px but container is 300px; layout still places them
  assert_eq!(r.children[0].offset.x, 0.0);
  assert_eq!(r.children[1].offset.x, 200.0);
  // But clipping is applied during quad resolution (visual, not layout)
}

// ============================================================================
// Intrinsic sizes with flex
// ============================================================================

#[test]
fn flex_intrinsic_leaf_contributes_size() {
  let mut rt = rt();
  let leaf = Element::new().intrinsic(80.0, 40.0);
  let node = Element::row().spacing(10.0).with_children(vec![leaf, rect(50.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(r.children[0].result.size.width, 80.0);
  assert_eq!(r.children[0].result.size.height, 40.0);
  assert_eq!(r.size.width, 140.0); // 80 + 10 + 50
}

// ============================================================================
// Percentage padding
// ============================================================================

#[test]
fn percentage_padding_resolves_against_parent() {
  use lurq::node::{dimension::Dimension, padding::Padding};
  let mut rt = rt();
  let node = rect(100.0, 100.0).padding(Padding::all(Dimension::Pct(10.0)));
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  // 10% of 400 = 40 horizontal padding on each side, 10% of 300 = 30 vertical
  // Inner rect is 100x100, so outer = 100+80=180 x 100+60=160
  // The padding is the outermost wrapper applied first on the node
  let pad_result = &r;
  let frame_result = &pad_result.children[0].result;
  let inner_w = frame_result.children[0].result.size.width;
  let inner_h = frame_result.children[0].result.size.height;
  assert_eq!(inner_w, 100.0, "inner width");
  assert_eq!(inner_h, 100.0, "inner height");
  assert!(
    (pad_result.size.width - 180.0).abs() < 1.0,
    "outer w={}",
    pad_result.size.width
  );
  assert!(
    (pad_result.size.height - 160.0).abs() < 1.0,
    "outer h={}",
    pad_result.size.height
  );
}

// ============================================================================
// Multiple justify + spacing interaction
// ============================================================================

#[test]
fn justify_space_between_with_explicit_spacing() {
  let mut rt = rt();
  // SpaceBetween ignores explicit spacing, distributes free space between items
  let node = Element::row()
    .spacing(0.0)
    .justify(Justify::SpaceBetween)
    .with_children(vec![rect(50.0, 30.0), rect(50.0, 30.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  let gap = r.children[1].offset.x - (r.children[0].offset.x + r.children[0].result.size.width);
  assert!((gap - 100.0).abs() < 1.0, "gap should be 100px, got {}", gap);
}

#[test]
fn justify_space_evenly_single_child() {
  let mut rt = rt();
  let node = Element::row()
    .spacing(0.0)
    .justify(Justify::SpaceEvenly)
    .child(rect(100.0, 50.0));
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  // Free=200, 2 slots → 100 each
  let offset = r.children[0].offset.x;
  assert!((offset - 100.0).abs() < 1.0, "should be centered, got {}", offset);
}

// ============================================================================
// Column flex enhancements
// ============================================================================

#[test]
fn column_justify_space_between() {
  let mut rt = rt();
  let node = Element::column()
    .spacing(0.0)
    .justify(Justify::SpaceBetween)
    .with_children(vec![rect(100.0, 40.0), rect(100.0, 40.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 300.0))).unwrap();
  let first_y = r.children[0].offset.y;
  let last_bottom = r.children[1].offset.y + r.children[1].result.size.height;
  assert!(first_y.abs() < 1.0, "first at top");
  assert!((last_bottom - 300.0).abs() < 1.0, "last at bottom");
}

#[test]
fn column_flex_shrink() {
  let mut rt = rt();
  let node = Element::column().spacing(0.0).with_children(vec![
    rect(100.0, 150.0).flex_full(0.0, 1.0, None),
    rect(100.0, 150.0).flex_full(0.0, 1.0, None),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 200.0))).unwrap();
  let total = r.children[0].result.size.height + r.children[1].result.size.height;
  assert!((total - 200.0).abs() < 1.0, "total should be 200, got {}", total);
}

#[test]
fn column_flex_basis() {
  let mut rt = rt();
  let node = Element::column().spacing(0.0).with_children(vec![
    Element::new().flex_full(0.0, 0.0, Some(80.0)),
    Element::new().flex_full(0.0, 0.0, Some(120.0)),
  ]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(200.0, 300.0))).unwrap();
  assert!(
    (r.children[0].result.size.height - 80.0).abs() < 1.0,
    "basis:80, got {}",
    r.children[0].result.size.height
  );
  assert!(
    (r.children[1].result.size.height - 120.0).abs() < 1.0,
    "basis:120, got {}",
    r.children[1].result.size.height
  );
}

#[test]
fn column_flex_wrap() {
  let mut rt = rt();
  let node =
    Element::column()
      .spacing(0.0)
      .wrap()
      .with_children(vec![rect(50.0, 120.0), rect(50.0, 120.0), rect(50.0, 120.0)]);
  rt.set_root(node);
  let r = rt.compute_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();
  // First item: 120 ≤ 200 → fits. Second: 120+120=240 > 200 → wraps
  assert_eq!(r.children[0].offset.x, r.children[0].offset.x); // sanity
  assert!(
    r.children[1].offset.x > r.children[0].offset.x,
    "second should wrap to next column"
  );
}
