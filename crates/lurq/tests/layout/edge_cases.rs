use lurq::{
  app::Runtime,
  layout::{Alignment, Constraints, Size, StackAlignment, layout_kind::FrameConstraints, quad::QuadContent},
  node::{Element, color::Color, dimension::Dimension, padding::Padding},
};

fn rt() -> Runtime {
  Runtime::new()
}

// --- Zero-size scenarios ---

#[test]
fn leaf_with_no_constraints_is_zero() {
  let mut rt = rt();
  let node = Element::new();
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}

#[test]
fn leaf_with_tight_zero_constraints() {
  let mut rt = rt();
  let node = Element::new();
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(0.0, 0.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 0.0);
}

#[test]
fn row_of_zero_size_children() {
  let mut rt = rt();
  let node = Element::row_with(
    10.0,
    Alignment::Start,
    vec![Element::new(), Element::new(), Element::new()],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 20.0); // spacing only: 10*2
  assert_eq!(result.size.height, 0.0);
}

#[test]
fn column_of_zero_size_children() {
  let mut rt = rt();
  let node = Element::column_with(5.0, Alignment::Start, vec![Element::new(), Element::new()]);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 300.0))).unwrap();
  assert_eq!(result.size.width, 0.0);
  assert_eq!(result.size.height, 5.0);
}

// --- Tight constraints override child preference ---

#[test]
fn frame_overrides_outer_constraints() {
  let mut rt = rt();
  // Frame sets its own tight constraints, overriding the outer ones
  let node = Element::new().frame(FrameConstraints {
    width: Some(lurq::node::dimension::Dimension::Px(500.0)),
    height: Some(lurq::node::dimension::Dimension::Px(500.0)),
    ..Default::default()
  });
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(100.0, 80.0))).unwrap();
  assert_eq!(result.size.width, 500.0);
  assert_eq!(result.size.height, 500.0);
}

#[test]
fn max_frame_constrains_inner_frame() {
  let mut rt = rt();
  // Outer frame with max_width limits the inner frame
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(500.0)),
      height: Some(lurq::node::dimension::Dimension::Px(500.0)),
      ..Default::default()
    })
    .frame(FrameConstraints {
      max_width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      max_height: Some(lurq::node::dimension::Dimension::Px(80.0)),
      ..Default::default()
    });
  rt.set_root(node);
  let result = rt
    .compute_layout(Constraints::loose(Size::new(1000.0, 1000.0)))
    .unwrap();
  assert!(result.size.width <= 100.0);
  assert!(result.size.height <= 80.0);
}

#[test]
fn min_constraints_expand_leaf() {
  let mut rt = rt();
  let node = Element::new();
  let c = Constraints {
    min_width: 50.0,
    max_width: 200.0,
    min_height: 30.0,
    max_height: 200.0,
  };
  rt.set_root(node);
  let result = rt.compute_layout(c).unwrap();
  assert_eq!(result.size.width, 50.0);
  assert_eq!(result.size.height, 30.0);
}

// --- Flex edge cases ---

#[test]
fn flex_all_children_are_flex() {
  let mut rt = rt();
  let node = Element::row_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new().flex(1.0),
      Element::new().flex(1.0),
      Element::new().flex(1.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  for child in &result.children {
    assert!((child.result.size.width - 100.0).abs() < 0.01);
  }
}

#[test]
fn flex_single_child_takes_all_space() {
  let mut rt = rt();
  let node = Element::row_with(0.0, Alignment::Start, vec![Element::new().flex(1.0)]);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(400.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 400.0);
}

#[test]
fn flex_zero_remaining_space() {
  let mut rt = rt();
  let node = Element::row_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(400.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
      Element::new().flex(1.0),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(400.0, 100.0))).unwrap();
  assert_eq!(result.children[1].result.size.width, 0.0);
}

#[test]
fn flex_with_large_spacing_eats_space() {
  let mut rt = rt();
  let node = Element::row_with(
    100.0,
    Alignment::Start,
    vec![Element::new().flex(1.0), Element::new().flex(1.0)],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  // 200 - 100 spacing = 100 available, split 50/50
  assert_eq!(result.children[0].result.size.width, 50.0);
  assert_eq!(result.children[1].result.size.width, 50.0);
}

#[test]
fn flex_factor_very_small() {
  let mut rt = rt();
  let node = Element::row_with(
    0.0,
    Alignment::Start,
    vec![Element::new().flex(0.001), Element::new().flex(999.999)],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(1000.0, 100.0))).unwrap();
  assert!(result.children[0].result.size.width < 2.0);
  assert!(result.children[1].result.size.width > 998.0);
}

// --- Stacking modifiers ---

#[test]
fn multiple_modifiers_chain() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(50.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .background(Color::new(255, 0, 0, 255))
    .padding(Padding::all(Dimension::Px(10.0)))
    .offset(5.0, 5.0);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  // offset wraps padding wraps background wraps frame
  assert_eq!(result.size.width, 70.0); // 50 + 10*2
  assert_eq!(result.size.height, 70.0);
  assert_eq!(result.children[0].offset.x, 5.0); // offset
  assert_eq!(result.children[0].offset.y, 5.0);
}

#[test]
fn align_modifier_is_passthrough_in_flex() {
  let mut rt = rt();
  let node = Element::row_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(50.0)),
          height: Some(lurq::node::dimension::Dimension::Px(50.0)),
          ..Default::default()
        })
        .align(Alignment::Center),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  // align doesn't affect sizing, just passes through
  assert_eq!(result.children[0].result.size.width, 50.0);
  assert_eq!(result.children[1].offset.x, 50.0);
}

// --- Stack edge cases ---

#[test]
fn stack_single_child() {
  let mut rt = rt();
  let node = Element::stack_with(
    StackAlignment::BottomEnd,
    vec![Element::new().frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(80.0)),
      ..Default::default()
    })],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 80.0);
  assert_eq!(result.children[0].offset.x, 0.0);
  assert_eq!(result.children[0].offset.y, 0.0);
}

#[test]
fn stack_all_same_size_children() {
  let mut rt = rt();
  let f = || {
    Element::new().frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(100.0)),
      ..Default::default()
    })
  };
  let node = Element::stack_with(StackAlignment::Center, vec![f(), f(), f()]);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 100.0);
  for child in &result.children {
    assert_eq!(child.offset.x, 0.0);
    assert_eq!(child.offset.y, 0.0);
  }
}

// --- Padding edge cases ---

#[test]
fn padding_larger_than_constraints() {
  let mut rt = rt();
  let node = Element::new().padding(Padding::all(Dimension::Px(100.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(50.0, 50.0))).unwrap();
  // Padding subtracts from constraints, clamped to 0
  assert_eq!(result.size.width, 50.0);
  assert_eq!(result.size.height, 50.0);
}

#[test]
fn nested_padding_accumulates() {
  let mut rt = rt();
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(10.0)),
      height: Some(lurq::node::dimension::Dimension::Px(10.0)),
      ..Default::default()
    })
    .padding(Padding::all(Dimension::Px(5.0)))
    .padding(Padding::all(Dimension::Px(5.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 30.0); // 10 + 5*2 + 5*2
  assert_eq!(result.size.height, 30.0);
}

// --- Quad edge cases ---

#[test]
fn quads_skip_modifier_wrapper_nodes() {
  let mut rt = rt();
  // padding modifier itself has no color, so no quad for it
  let node = Element::new()
    .frame(FrameConstraints {
      width: Some(lurq::node::dimension::Dimension::Px(100.0)),
      height: Some(lurq::node::dimension::Dimension::Px(50.0)),
      ..Default::default()
    })
    .background(Color::new(255, 0, 0, 255))
    .padding(Padding::all(Dimension::Px(10.0)));
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  // Only the background node produces a quad, not the padding wrapper
  assert_eq!(quads.len(), 1);
  assert_eq!(quads[0].x, 10.0);
  assert_eq!(quads[0].y, 10.0);
}

#[test]
fn quads_multiple_visible_children() {
  let mut rt = rt();
  let node = Element::column_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(100.0)),
          height: Some(lurq::node::dimension::Dimension::Px(50.0)),
          ..Default::default()
        })
        .background(Color::new(255, 0, 0, 255)),
      Element::text("middle"),
      Element::new()
        .frame(FrameConstraints {
          width: Some(lurq::node::dimension::Dimension::Px(100.0)),
          height: Some(lurq::node::dimension::Dimension::Px(50.0)),
          ..Default::default()
        })
        .background(Color::new(0, 0, 255, 255)),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 3);
  assert!(matches!(quads[0].content, QuadContent::Rect { .. }));
  assert!(matches!(quads[1].content, QuadContent::Text { .. }));
  assert!(matches!(quads[2].content, QuadContent::Rect { .. }));
}

#[test]
fn quads_offset_accumulates_through_nesting() {
  let mut rt = rt();
  let node = Element::column_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(100.0)),
        ..Default::default()
      }),
      Element::row_with(
        0.0,
        Alignment::Start,
        vec![
          Element::new().frame(FrameConstraints {
            width: Some(lurq::node::dimension::Dimension::Px(30.0)),
            height: Some(lurq::node::dimension::Dimension::Px(30.0)),
            ..Default::default()
          }),
          Element::new()
            .frame(FrameConstraints {
              width: Some(lurq::node::dimension::Dimension::Px(30.0)),
              height: Some(lurq::node::dimension::Dimension::Px(30.0)),
              ..Default::default()
            })
            .background(Color::new(0, 255, 0, 255))
            .padding(Padding::all(Dimension::Px(5.0))),
        ],
      ),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  let quads = rt.resolve_quads(&result);
  assert_eq!(quads.len(), 1);
  // column offset y=100, row offset x=30, padding offset x=5 y=5
  assert_eq!(quads[0].x, 30.0 + 5.0);
  assert_eq!(quads[0].y, 100.0 + 5.0);
}

// --- Unbounded constraints ---

#[test]
fn row_with_unbounded_constraints() {
  let mut rt = rt();
  let node = Element::row_with(
    10.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(200.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::unbounded()).unwrap();
  assert_eq!(result.size.width, 310.0);
  assert_eq!(result.size.height, 50.0);
}

#[test]
fn column_with_unbounded_constraints() {
  let mut rt = rt();
  let node = Element::column_with(
    10.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(80.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::unbounded()).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 140.0);
}

// --- Large trees ---

#[test]
fn many_children_in_row() {
  let mut rt = rt();
  let children: Vec<Element> = (0..100)
    .map(|_| {
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(10.0)),
        height: Some(lurq::node::dimension::Dimension::Px(10.0)),
        ..Default::default()
      })
    })
    .collect();
  let node = Element::row_with(1.0, Alignment::Start, children);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::unbounded()).unwrap();
  assert_eq!(result.size.width, 1099.0); // 100*10 + 99*1
  assert_eq!(result.size.height, 10.0);
  assert_eq!(result.children.len(), 100);
}

#[test]
fn many_children_in_column() {
  let mut rt = rt();
  let children: Vec<Element> = (0..100)
    .map(|_| {
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(10.0)),
        height: Some(lurq::node::dimension::Dimension::Px(10.0)),
        ..Default::default()
      })
    })
    .collect();
  let node = Element::column_with(2.0, Alignment::Start, children);
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::unbounded()).unwrap();
  assert_eq!(result.size.width, 10.0);
  assert_eq!(result.size.height, 1198.0); // 100*10 + 99*2
}

// --- Mixed flex and fixed ---

#[test]
fn flex_between_two_fixed() {
  let mut rt = rt();
  let node = Element::row_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
      Element::new().flex(1.0),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(50.0)),
        height: Some(lurq::node::dimension::Dimension::Px(50.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(300.0, 100.0))).unwrap();
  assert_eq!(result.children[0].result.size.width, 50.0);
  assert_eq!(result.children[1].result.size.width, 200.0);
  assert_eq!(result.children[2].result.size.width, 50.0);
  assert_eq!(result.children[1].offset.x, 50.0);
  assert_eq!(result.children[2].offset.x, 250.0);
}

#[test]
fn two_fixed_one_flex_column() {
  let mut rt = rt();
  let node = Element::column_with(
    0.0,
    Alignment::Start,
    vec![
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(40.0)),
        ..Default::default()
      }),
      Element::new().flex(1.0),
      Element::new().frame(FrameConstraints {
        width: Some(lurq::node::dimension::Dimension::Px(100.0)),
        height: Some(lurq::node::dimension::Dimension::Px(40.0)),
        ..Default::default()
      }),
    ],
  );
  rt.set_root(node);
  let result = rt.compute_layout(Constraints::tight(Size::new(100.0, 200.0))).unwrap();
  assert_eq!(result.children[0].result.size.height, 40.0);
  assert_eq!(result.children[1].result.size.height, 120.0);
  assert_eq!(result.children[2].result.size.height, 40.0);
}
