//! The arrangement of a dock layout: the helpers that build one, reading it, each place a panel or a group goes,
//! selecting, closing, hiding and putting back, maximizing, shares and floating groups, the rules that make it
//! valid, and its text form.

use lurq::ui::{
  DockDocuments, DockFloat, DockGroup, DockHidden, DockNode, DockPlace, DockSide, DockSplit, DockState,
  DockStateParseError, Orientation,
};

fn group(panels: &[&str]) -> DockNode {
  DockNode::group(panels.iter().copied())
}

fn tab(of: &str) -> DockPlace {
  DockPlace::Tab { of: of.to_owned() }
}

fn beside(of: &str, side: DockSide) -> DockPlace {
  DockPlace::Beside {
    of: of.to_owned(),
    side,
  }
}

fn float_at(x: f32, y: f32) -> DockPlace {
  DockPlace::Float {
    position: (x, y),
    size: None,
  }
}

fn hidden_group(panel: &str, side: DockSide) -> DockNode {
  DockNode::Group(DockGroup {
    panels: Vec::new(),
    shown: None,
    hidden: vec![DockHidden {
      panel: panel.to_owned(),
      side,
    }],
  })
}

fn documents() -> DockNode {
  DockNode::documents(group(&["blur.rs", "theme.rs"]))
}

fn bottom() -> DockNode {
  group(&["terminal", "problems", "output"])
}

/// The explorer at the left; the documents above the terminal's group at the right.
fn sample() -> DockState {
  DockState::new(
    DockNode::row([
      group(&["explorer"]),
      DockNode::column([documents(), bottom()]).shares([0.7, 0.3]),
    ])
    .shares([0.25, 0.75]),
  )
}

fn shares_of(node: Option<&DockNode>) -> Vec<f32> {
  match node {
    Some(DockNode::Split(split)) => split.shares.clone(),
    _ => panic!("not a split: {node:?}"),
  }
}

fn assert_shares(node: Option<&DockNode>, expected: &[f32]) {
  let shares = shares_of(node);
  assert_eq!(shares.len(), expected.len(), "{shares:?}");
  for (share, expected) in shares.iter().zip(expected) {
    assert!((share - expected).abs() < 1e-5, "{shares:?} against {expected:?}");
  }
}

// The helpers.

#[test]
fn helpers_build_what_they_say() {
  assert_eq!(
    DockNode::row([group(&["a"]), group(&["b"])]),
    DockNode::Split(DockSplit {
      orientation: Orientation::Horizontal,
      children: vec![group(&["a"]), group(&["b"])],
      shares: vec![0.5, 0.5],
    })
  );
  let column = DockNode::column([group(&["a"]), group(&["b"]), group(&["c"])]);
  let DockNode::Split(split) = &column else {
    panic!("a column is a split");
  };
  assert_eq!(split.orientation, Orientation::Vertical);
  assert_eq!(split.shares, vec![1.0 / 3.0; 3]);
  assert_eq!(
    DockNode::group(["a", "b"]),
    DockNode::Group(DockGroup {
      panels: vec!["a".into(), "b".into()],
      shown: Some("a".into()),
      hidden: Vec::new(),
    })
  );
  let DockNode::Group(shown) = DockNode::group(["a", "b"]).shown("b") else {
    panic!("a group");
  };
  assert_eq!(shown.shown.as_deref(), Some("b"));
  assert_eq!(
    DockNode::documents(None),
    DockNode::Documents(DockDocuments { node: None })
  );
  assert_eq!(
    DockNode::documents(group(&["a"])),
    DockNode::Documents(DockDocuments {
      node: Some(Box::new(group(&["a"]))),
    })
  );
  // Each ignored where it does not apply.
  assert_eq!(group(&["a"]).shares([1.0]), group(&["a"]));
  assert_eq!(
    DockNode::row([group(&["a"])]).shown("a"),
    DockNode::row([group(&["a"])])
  );
  assert_eq!(
    DockNode::row([group(&["a"]), group(&["b"])]).shares([0.3, 0.7]),
    DockNode::Split(DockSplit {
      orientation: Orientation::Horizontal,
      children: vec![group(&["a"]), group(&["b"])],
      shares: vec![0.3, 0.7],
    })
  );

  let state = DockState::new(group(&["a"]));
  assert_eq!(state.root, Some(group(&["a"])));
  assert!(state.floating.is_empty() && state.maximized.is_none());
  assert_eq!(DockState::empty().root, None);
  assert_eq!(DockState::default(), DockState::empty());
  assert_eq!(DockSide::ALL.len(), 4);
}

// Reading.

#[test]
fn reading_finds_each_panel() {
  let mut state = sample();
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert!(state.show("outline", float_at(10.0, 20.0)));

  assert!(state.contains("explorer") && state.contains("problems") && state.contains("outline"));
  assert!(!state.contains("ghost"));
  assert_eq!(
    state.panels(),
    [
      "explorer", "blur.rs", "theme.rs", "terminal", "output", "problems", "outline"
    ]
  );
  assert_eq!(state.is_hidden("problems"), Some(DockSide::Bottom));
  assert_eq!(state.is_hidden("terminal"), None);
  assert!(state.is_floating("outline") && !state.is_floating("terminal"));
  assert_eq!(
    state.hidden(),
    vec![DockHidden {
      panel: "problems".into(),
      side: DockSide::Bottom,
    }]
  );
  assert_eq!(state.group_of("problems").unwrap().panels, ["terminal", "output"]);
  assert_eq!(state.group_of("outline").unwrap().panels, ["outline"]);
  assert!(state.group_of("ghost").is_none());
  assert!(state.in_documents("blur.rs") && !state.in_documents("terminal") && !state.in_documents("outline"));
}

#[test]
fn node_at_goes_through_splits_and_the_documents_area() {
  let state = sample();
  assert_eq!(state.node_at(&[]), state.root.as_ref());
  assert_eq!(state.node_at(&[0]), Some(&group(&["explorer"])));
  assert!(matches!(state.node_at(&[1]), Some(DockNode::Split(_))));
  assert_eq!(state.node_at(&[1, 0]), Some(&documents()));
  assert_eq!(state.node_at(&[1, 0, 0]), Some(&group(&["blur.rs", "theme.rs"])));
  assert_eq!(state.node_at(&[1, 0, 1]), None);
  assert_eq!(state.node_at(&[0, 0]), None);
  assert_eq!(state.node_at(&[5]), None);
  assert_eq!(DockState::empty().node_at(&[]), None);
}

// Tabs.

#[test]
fn tab_moves_a_panel_after_the_tabs_of_another_group() {
  let mut state = sample();
  assert!(state.show("explorer", tab("terminal")));
  // The explorer's group goes, and the row of one folds.
  assert_eq!(
    state.root,
    Some(
      DockNode::column([
        documents(),
        group(&["terminal", "problems", "output", "explorer"]).shown("explorer"),
      ])
      .shares([0.7, 0.3])
    )
  );

  let mut state = sample();
  assert!(state.show("search", tab("explorer")));
  assert_eq!(
    state.node_at(&[0]),
    Some(&group(&["explorer", "search"]).shown("search"))
  );
}

#[test]
fn tab_that_changes_nothing() {
  let mut state = sample();
  assert!(!state.show("explorer", tab("explorer")));
  assert!(!state.show("explorer", tab("ghost")));
  assert!(!state.show("search", tab("ghost")));
  assert!(!state.contains("search"));
  // The last tab, shown, moved to after its own group's tabs.
  assert!(state.select("output"));
  assert!(!state.show("output", tab("terminal")));
  // Not the last: it goes to the end.
  assert!(state.show("terminal", tab("output")));
  assert_eq!(
    state.group_of("terminal").unwrap().panels,
    ["problems", "output", "terminal"]
  );
  assert_eq!(state.group_of("terminal").unwrap().shown.as_deref(), Some("terminal"));
}

// Beside.

#[test]
fn beside_in_a_split_of_the_same_orientation_takes_half_the_share() {
  let mut state = sample();
  assert!(state.show("outline", beside("explorer", DockSide::Right)));
  let DockNode::Split(row) = state.root.as_ref().unwrap() else {
    panic!("a row");
  };
  assert_eq!(row.children[0], group(&["explorer"]));
  assert_eq!(row.children[1], group(&["outline"]));
  assert_eq!(row.shares, vec![0.125, 0.125, 0.75]);

  let mut state = sample();
  assert!(state.show("outline", beside("explorer", DockSide::Left)));
  let DockNode::Split(row) = state.root.as_ref().unwrap() else {
    panic!("a row");
  };
  assert_eq!(row.children[0], group(&["outline"]));
  assert_eq!(row.children[1], group(&["explorer"]));
  assert_eq!(row.shares, vec![0.125, 0.125, 0.75]);

  let mut state = sample();
  assert!(state.show("outline", beside("terminal", DockSide::Bottom)));
  assert_eq!(
    state.node_at(&[1]),
    Some(&DockNode::column([documents(), bottom(), group(&["outline"])]).shares([0.7, 0.15, 0.15]))
  );
}

#[test]
fn beside_across_the_split_makes_a_new_one() {
  let mut state = sample();
  assert!(state.show("outline", beside("explorer", DockSide::Top)));
  assert_eq!(
    state.node_at(&[0]),
    Some(&DockNode::column([group(&["outline"]), group(&["explorer"])]))
  );
  assert_eq!(shares_of(state.root.as_ref()), vec![0.25, 0.75]);

  let mut state = sample();
  assert!(state.show("outline", beside("explorer", DockSide::Bottom)));
  assert_eq!(
    state.node_at(&[0]),
    Some(&DockNode::column([group(&["explorer"]), group(&["outline"])]))
  );

  let mut state = sample();
  assert!(state.show("outline", beside("terminal", DockSide::Right)));
  assert_eq!(
    state.node_at(&[1, 1]),
    Some(&DockNode::row([bottom(), group(&["outline"])]))
  );
}

#[test]
fn beside_within_the_documents_area() {
  let mut state = sample();
  assert!(state.show("lib.rs", beside("theme.rs", DockSide::Right)));
  assert_eq!(
    state.node_at(&[1, 0]),
    Some(&DockNode::documents(DockNode::row([
      group(&["blur.rs", "theme.rs"]),
      group(&["lib.rs"]),
    ])))
  );
  assert!(state.in_documents("lib.rs"));
}

#[test]
fn beside_itself_floating_or_hidden() {
  let mut state = sample();
  // Alone in its group: nothing.
  assert!(!state.show("explorer", beside("explorer", DockSide::Right)));
  assert!(!state.show("explorer", beside("ghost", DockSide::Right)));
  // With others: beside what is left of its group.
  assert!(state.show("problems", beside("problems", DockSide::Right)));
  assert_eq!(
    state.node_at(&[1, 1]),
    Some(&DockNode::row([group(&["terminal", "output"]), group(&["problems"])]))
  );

  // Beside a floating panel: a tab of its group.
  let mut state = sample();
  assert!(state.show("outline", float_at(0.0, 0.0)));
  assert!(state.show("search", beside("outline", DockSide::Left)));
  assert_eq!(state.floating[0].group.panels, ["outline", "search"]);

  // Beside a hidden panel: along the edge it is hidden at.
  let mut state = sample();
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert!(state.show("search", beside("problems", DockSide::Left)));
  let DockNode::Split(column) = state.root.as_ref().unwrap() else {
    panic!("a column");
  };
  assert_eq!(column.orientation, Orientation::Vertical);
  assert_eq!(column.children[1], group(&["search"]));
  assert_eq!(column.shares, vec![0.75, 0.25]);
}

// Edges.

#[test]
fn edge_of_each_side() {
  let mut state = sample();
  assert!(state.show("a", DockPlace::Edge(DockSide::Left)));
  assert_eq!(state.node_at(&[0]), Some(&group(&["a"])));
  assert_eq!(shares_of(state.root.as_ref()), vec![0.25, 0.1875, 0.5625]);

  let mut state = sample();
  assert!(state.show("a", DockPlace::Edge(DockSide::Right)));
  assert_eq!(state.node_at(&[2]), Some(&group(&["a"])));
  assert_eq!(shares_of(state.root.as_ref()), vec![0.1875, 0.5625, 0.25]);

  let mut state = sample();
  assert!(state.show("a", DockPlace::Edge(DockSide::Top)));
  assert_eq!(
    state.root,
    Some(DockNode::column([group(&["a"]), sample().root.unwrap()]).shares([0.25, 0.75]))
  );

  let mut state = sample();
  assert!(state.show("a", DockPlace::Edge(DockSide::Bottom)));
  assert_eq!(
    state.root,
    Some(DockNode::column([sample().root.unwrap(), group(&["a"])]).shares([0.75, 0.25]))
  );

  let mut state = DockState::empty();
  assert!(state.show("a", DockPlace::Edge(DockSide::Bottom)));
  assert_eq!(state.root, Some(group(&["a"])));
}

// The documents area.

#[test]
fn documents_is_a_tab_of_its_first_group() {
  let mut state = sample();
  assert!(state.show("lib.rs", DockPlace::Documents));
  assert_eq!(
    state.node_at(&[1, 0, 0]),
    Some(&group(&["blur.rs", "theme.rs", "lib.rs"]).shown("lib.rs"))
  );
  // Moved there from the docked part.
  assert!(state.show("terminal", DockPlace::Documents));
  assert!(state.in_documents("terminal"));
  assert_eq!(state.group_of("problems").unwrap().panels, ["problems", "output"]);
}

#[test]
fn documents_where_the_area_is_empty_or_missing() {
  let mut state = DockState::new(DockNode::row([group(&["explorer"]), DockNode::documents(None)]));
  assert!(state.show("lib.rs", DockPlace::Documents));
  assert_eq!(state.node_at(&[1]), Some(&DockNode::documents(group(&["lib.rs"]))));

  let mut state = DockState::new(group(&["explorer"]));
  assert!(state.show("lib.rs", DockPlace::Documents));
  assert_eq!(
    state.root,
    Some(DockNode::row([group(&["explorer"]), DockNode::documents(group(&["lib.rs"]))]).shares([0.75, 0.25]))
  );

  let mut state = DockState::empty();
  assert!(state.show("lib.rs", DockPlace::Documents));
  assert_eq!(state.root, Some(DockNode::documents(group(&["lib.rs"]))));

  // The last document closed: the area stays, empty.
  assert!(state.close("lib.rs"));
  assert_eq!(state.root, Some(DockNode::documents(None)));
}

// Floating.

#[test]
fn float_takes_the_panel_out_into_a_group_of_its_own() {
  let mut state = sample();
  assert!(state.show(
    "terminal",
    DockPlace::Float {
      position: (10.0, 20.0),
      size: Some((300.0, 200.0)),
    }
  ));
  assert_eq!(
    state.floating,
    vec![DockFloat {
      group: DockGroup {
        panels: vec!["terminal".into()],
        shown: Some("terminal".into()),
        hidden: Vec::new(),
      },
      position: (10.0, 20.0),
      size: Some((300.0, 200.0)),
    }]
  );
  // Its old group shows the one after it.
  assert_eq!(state.node_at(&[1, 1]), Some(&group(&["problems", "output"])));
  assert!(state.is_floating("terminal"));

  // A tab of a floating group.
  assert!(state.move_panel("problems", tab("terminal")));
  assert_eq!(state.floating[0].group.panels, ["terminal", "problems"]);
  assert_eq!(state.floating[0].group.shown.as_deref(), Some("problems"));
  // And back.
  assert!(state.move_panel("problems", beside("output", DockSide::Top)));
  assert!(!state.is_floating("problems"));
  assert!(!state.move_panel("ghost", float_at(0.0, 0.0)));
}

// Hidden.

#[test]
fn hidden_from_docked_keeps_its_group() {
  let mut state = sample();
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert_eq!(
    state.node_at(&[1, 1]),
    Some(&DockNode::Group(DockGroup {
      panels: vec!["terminal".into(), "output".into()],
      shown: Some("terminal".into()),
      hidden: vec![DockHidden {
        panel: "problems".into(),
        side: DockSide::Bottom,
      }],
    }))
  );
  // Again at the same side: nothing; at another: its side changes.
  assert!(!state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Right)));
  assert_eq!(state.is_hidden("problems"), Some(DockSide::Right));

  // The shown one hidden: the group shows the one after it, else the one before.
  let mut state = sample();
  assert!(state.show("terminal", DockPlace::Hidden(DockSide::Bottom)));
  assert_eq!(state.group_of("terminal").unwrap().shown.as_deref(), Some("problems"));
  assert!(state.select("output"));
  assert!(state.show("output", DockPlace::Hidden(DockSide::Bottom)));
  assert_eq!(state.group_of("output").unwrap().shown.as_deref(), Some("problems"));
  // Every tab hidden: the group stays, showing nothing.
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  let group_of = state.group_of("problems").unwrap();
  assert!(group_of.panels.is_empty() && group_of.shown.is_none() && group_of.hidden.len() == 3);
  assert!(matches!(state.node_at(&[1]), Some(DockNode::Split(_))));
}

#[test]
fn hidden_from_floating_or_new_goes_along_the_edge_first() {
  let mut state = sample();
  assert!(state.show("outline", float_at(0.0, 0.0)));
  assert!(state.show("outline", DockPlace::Hidden(DockSide::Right)));
  assert!(state.floating.is_empty());
  assert_eq!(state.node_at(&[2]), Some(&hidden_group("outline", DockSide::Right)));
  assert_eq!(shares_of(state.root.as_ref()), vec![0.1875, 0.5625, 0.25]);

  let mut state = sample();
  assert!(state.show("search", DockPlace::Hidden(DockSide::Left)));
  assert_eq!(state.node_at(&[0]), Some(&hidden_group("search", DockSide::Left)));
  assert_eq!(state.is_hidden("search"), Some(DockSide::Left));
}

#[test]
fn put_back_returns_a_hidden_panel_to_its_group_shown() {
  let mut state = sample();
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert!(state.put_back("problems"));
  assert_eq!(
    state.node_at(&[1, 1]),
    Some(&group(&["terminal", "output", "problems"]).shown("problems"))
  );
  assert!(!state.put_back("problems"));
  assert!(!state.put_back("ghost"));

  // Hidden along an edge, put back where it was put.
  let mut state = sample();
  assert!(state.show("search", DockPlace::Hidden(DockSide::Left)));
  assert!(state.put_back("search"));
  assert_eq!(state.node_at(&[0]), Some(&group(&["search"])));
}

// Groups.

#[test]
fn move_group_to_a_tab() {
  let mut state = sample();
  assert!(state.move_group("problems", tab("explorer")));
  assert_eq!(
    state.root,
    Some(
      DockNode::row([
        group(&["explorer", "terminal", "problems", "output"]).shown("terminal"),
        documents(),
      ])
      .shares([0.25, 0.75])
    )
  );
  // Into its own group, or of a panel not held: nothing.
  assert!(!state.move_group("problems", tab("explorer")));
  assert!(!state.move_group("ghost", tab("explorer")));
  assert!(!state.move_group("explorer", tab("ghost")));
  assert!(!state.move_group("explorer", beside("terminal", DockSide::Left)));
}

#[test]
fn move_group_beside_along_an_edge_and_into_the_documents() {
  let mut state = sample();
  assert!(state.move_group("explorer", beside("terminal", DockSide::Bottom)));
  assert_eq!(
    state.root,
    Some(DockNode::column([documents(), bottom(), group(&["explorer"])]).shares([0.7, 0.15, 0.15]))
  );

  let mut state = sample();
  assert!(state.move_group("output", DockPlace::Edge(DockSide::Left)));
  assert_eq!(
    state.root,
    Some(DockNode::row([bottom(), group(&["explorer"]), documents()]).shares([0.25, 0.1875, 0.5625]))
  );

  let mut state = sample();
  assert!(state.move_group("terminal", DockPlace::Documents));
  assert_eq!(
    state.root,
    Some(
      DockNode::row([
        group(&["explorer"]),
        DockNode::documents(DockNode::row([group(&["blur.rs", "theme.rs"]), bottom()])),
      ])
      .shares([0.25, 0.75])
    )
  );
}

#[test]
fn move_group_floats_and_hides() {
  let mut state = sample();
  assert!(state.select("problems"));
  assert!(state.move_group("terminal", float_at(5.0, 6.0)));
  assert_eq!(state.floating.len(), 1);
  assert_eq!(state.floating[0].group.panels, ["terminal", "problems", "output"]);
  assert_eq!(state.floating[0].group.shown.as_deref(), Some("problems"));
  assert_eq!(
    state.root,
    Some(DockNode::row([group(&["explorer"]), documents()]).shares([0.25, 0.75]))
  );
  // A floating group moved again floats at the new place, in front.
  assert!(state.show("outline", float_at(0.0, 0.0)));
  assert!(state.move_group("terminal", float_at(7.0, 8.0)));
  assert_eq!(state.floating[1].position, (7.0, 8.0));
  assert_eq!(state.floating[1].group.panels, ["terminal", "problems", "output"]);

  // Hidden: each of its panels, the group kept.
  let mut state = sample();
  assert!(state.move_group("terminal", DockPlace::Hidden(DockSide::Bottom)));
  let held = state.group_of("terminal").unwrap();
  assert!(held.panels.is_empty() && held.shown.is_none());
  assert_eq!(
    state
      .hidden()
      .into_iter()
      .map(|hidden| hidden.panel)
      .collect::<Vec<_>>(),
    ["terminal", "problems", "output"]
  );
  assert!(state.hidden().iter().all(|hidden| hidden.side == DockSide::Bottom));

  // A floating group hidden: along the edge.
  let mut state = sample();
  assert!(state.move_group("terminal", float_at(0.0, 0.0)));
  assert!(state.move_group("terminal", DockPlace::Hidden(DockSide::Right)));
  assert!(state.floating.is_empty());
  assert_eq!(state.is_hidden("output"), Some(DockSide::Right));
}

// Selecting and closing.

#[test]
fn select_shows_a_tab() {
  let mut state = sample();
  assert!(state.select("problems"));
  assert_eq!(state.group_of("problems").unwrap().shown.as_deref(), Some("problems"));
  assert!(!state.select("problems"));
  assert!(!state.select("ghost"));
  assert!(state.show("output", DockPlace::Hidden(DockSide::Bottom)));
  assert!(!state.select("output"));
}

#[test]
fn close_takes_a_panel_out_and_folds_what_is_left() {
  let mut state = sample();
  assert!(state.close("explorer"));
  assert_eq!(
    state.root,
    Some(DockNode::column([documents(), bottom()]).shares([0.7, 0.3]))
  );
  assert!(!state.close("explorer"));

  let mut state = sample();
  for panel in ["terminal", "problems", "output"] {
    assert!(state.close(panel));
  }
  assert_eq!(
    state.root,
    Some(DockNode::row([group(&["explorer"]), documents()]).shares([0.25, 0.75]))
  );

  // The shown one closed: the next is shown.
  let mut state = sample();
  assert!(state.close("terminal"));
  assert_eq!(state.group_of("problems").unwrap().shown.as_deref(), Some("problems"));

  // A hidden one and a floating one.
  assert!(state.show("output", DockPlace::Hidden(DockSide::Bottom)));
  assert!(state.close("output"));
  assert!(state.hidden().is_empty());
  assert!(state.show("outline", float_at(0.0, 0.0)));
  assert!(state.close("outline"));
  assert!(state.floating.is_empty());

  // The maximized one closed ends the maximize.
  let mut state = sample();
  assert!(state.maximize("terminal"));
  assert!(state.close("terminal"));
  assert_eq!(state.maximized, None);
}

// Maximizing.

#[test]
fn maximize_and_restore() {
  let mut state = sample();
  assert!(state.maximize("terminal"));
  assert_eq!(state.maximized.as_deref(), Some("terminal"));
  assert!(!state.maximize("terminal"));
  assert!(!state.maximize("ghost"));
  assert!(state.restore());
  assert!(!state.restore());

  assert!(state.show("outline", float_at(0.0, 0.0)));
  assert!(!state.maximize("outline"));
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert!(!state.maximize("problems"));

  // Another panel moved keeps it; the maximized one moved ends it.
  let mut state = sample();
  assert!(state.maximize("terminal"));
  assert!(state.show("explorer", DockPlace::Edge(DockSide::Right)));
  assert_eq!(state.maximized.as_deref(), Some("terminal"));
  assert!(state.show("terminal", tab("explorer")));
  assert_eq!(state.maximized, None);

  // A move that changes nothing keeps it.
  let mut state = sample();
  assert!(state.maximize("explorer"));
  assert!(!state.show("explorer", tab("explorer")));
  assert_eq!(state.maximized.as_deref(), Some("explorer"));
}

// Shares and floating groups.

#[test]
fn set_shares_of_a_split_by_its_path() {
  let mut state = sample();
  assert!(state.set_shares(&[1], &[0.4, 0.6]));
  assert_eq!(shares_of(state.node_at(&[1])), vec![0.4, 0.6]);
  assert!(!state.set_shares(&[1], &[0.4, 0.6]));
  assert!(!state.set_shares(&[1], &[0.2, 0.3, 0.5]));
  assert!(!state.set_shares(&[0], &[1.0]));
  assert!(!state.set_shares(&[9], &[0.5, 0.5]));
  // Brought to the rules.
  assert!(state.set_shares(&[], &[3.0, 1.0]));
  assert_eq!(shares_of(state.root.as_ref()), vec![0.75, 0.25]);
  assert!(state.set_shares(&[], &[f32::NAN, 1.0]));
  assert_shares(state.root.as_ref(), &[0.05, 0.95]);
}

#[test]
fn set_float_and_raise_float() {
  let mut state = sample();
  assert!(state.show("a", float_at(0.0, 0.0)));
  assert!(state.show("b", float_at(0.0, 0.0)));
  assert!(state.set_float(0, (5.0, 6.0), Some((200.0, 100.0))));
  assert_eq!(state.floating[0].position, (5.0, 6.0));
  assert_eq!(state.floating[0].size, Some((200.0, 100.0)));
  assert!(!state.set_float(0, (5.0, 6.0), Some((200.0, 100.0))));
  assert!(!state.set_float(2, (0.0, 0.0), None));
  assert!(state.set_float(0, (f32::NAN, 1.0), Some((0.5, 100.0))));
  assert_eq!(state.floating[0].position, (0.0, 0.0));
  assert_eq!(state.floating[0].size, None);

  assert!(state.raise_float(0));
  assert_eq!(state.floating[0].group.panels, ["b"]);
  assert_eq!(state.floating[1].group.panels, ["a"]);
  assert!(!state.raise_float(1));
  assert!(!state.raise_float(5));
}

// Making it valid.

#[test]
fn valid_leaves_out_unknown_panels() {
  let mut state = sample();
  assert!(state.show("outline", float_at(0.0, 0.0)));
  assert!(state.show("search", DockPlace::Hidden(DockSide::Left)));
  let state = state.valid(|panel| !matches!(panel, "problems" | "outline" | "search" | "explorer"));
  assert!(!state.contains("problems") && !state.contains("outline") && !state.contains("search"));
  assert!(state.floating.is_empty());
  // The explorer's group goes, the edge's group of the hidden one too, and the row folds.
  assert_eq!(
    state.root,
    Some(DockNode::column([documents(), group(&["terminal", "output"])]).shares([0.7, 0.3]))
  );
}

#[test]
fn valid_keeps_a_panel_named_twice_at_its_first_place() {
  let mut state = DockState::new(DockNode::row([group(&["a"]), group(&["a", "b"])]));
  state.floating.push(DockFloat {
    group: DockGroup {
      panels: vec!["b".into()],
      shown: Some("b".into()),
      hidden: Vec::new(),
    },
    position: (0.0, 0.0),
    size: None,
  });
  let state = state.valid(|_| true);
  assert_eq!(state.root, Some(DockNode::row([group(&["a"]), group(&["b"])])));
  assert!(state.floating.is_empty());
}

#[test]
fn valid_drops_empty_groups_and_folds_splits() {
  let empty = DockNode::Group(DockGroup::default());
  let state = DockState::new(DockNode::row([group(&["a"]), empty.clone()])).valid(|_| true);
  assert_eq!(state.root, Some(group(&["a"])));

  let state = DockState::new(DockNode::row([group(&["a"])])).valid(|_| true);
  assert_eq!(state.root, Some(group(&["a"])));

  let state = DockState::new(DockNode::row([])).valid(|_| true);
  assert_eq!(state.root, None);

  let state = DockState::new(DockNode::column([empty.clone(), DockNode::row([empty])])).valid(|_| true);
  assert_eq!(state.root, None);

  // A shown not one of its panels: the first.
  let state = DockState::new(group(&["a", "b"]).shown("ghost")).valid(|_| true);
  assert_eq!(state.root, Some(group(&["a", "b"])));
}

#[test]
fn valid_flattens_a_split_of_the_same_orientation() {
  let state = DockState::new(DockNode::row([
    group(&["a"]),
    DockNode::row([group(&["b"]), group(&["c"])]),
  ]))
  .valid(|_| true);
  assert_eq!(
    state.root,
    Some(DockNode::row([group(&["a"]), group(&["b"]), group(&["c"])]).shares([0.5, 0.25, 0.25]))
  );

  // Across the documents area it does not.
  let state = DockState::new(DockNode::row([
    group(&["a"]),
    DockNode::documents(DockNode::row([group(&["b"]), group(&["c"])])),
  ]))
  .valid(|_| true);
  assert!(matches!(state.node_at(&[1, 0]), Some(DockNode::Split(_))));
}

#[test]
fn valid_brings_shares_to_the_rules() {
  let abc = || [group(&["a"]), group(&["b"]), group(&["c"])];
  // A count that differs: equal shares.
  let state = DockState::new(DockNode::row(abc()).shares([1.0])).valid(|_| true);
  assert_shares(state.root.as_ref(), &[1.0 / 3.0; 3]);
  // Not finite or below the least: the least, then scaled, none below it.
  let state = DockState::new(DockNode::row(abc()).shares([f32::NAN, 0.0, 3.0])).valid(|_| true);
  assert_shares(state.root.as_ref(), &[0.05, 0.05, 0.9]);
  let state = DockState::new(DockNode::row(abc()).shares([f32::INFINITY, -1.0, 0.05])).valid(|_| true);
  assert_shares(state.root.as_ref(), &[1.0 / 3.0; 3]);
  // Scaled to sum to 1.
  let state = DockState::new(DockNode::row(abc()).shares([2.0, 1.0, 1.0])).valid(|_| true);
  assert_shares(state.root.as_ref(), &[0.5, 0.25, 0.25]);
  // A valid state is left as it is, to the bit.
  let once = sample().valid(|_| true);
  assert_eq!(once, sample());
  assert_eq!(once.clone().valid(|_| true), once);
  let odd = DockState::new(DockNode::row(abc()).shares([0.001, 7.0, 0.3])).valid(|_| true);
  assert_eq!(odd.clone().valid(|_| true), odd);
}

#[test]
fn valid_keeps_one_documents_area() {
  let state = DockState::new(DockNode::row([
    DockNode::documents(group(&["a"])),
    DockNode::documents(group(&["b"])),
  ]))
  .valid(|_| true);
  assert_eq!(
    state.root,
    Some(DockNode::row([DockNode::documents(group(&["a"])), group(&["b"])]))
  );
  assert!(state.in_documents("a") && !state.in_documents("b"));

  let state = DockState::new(DockNode::documents(DockNode::documents(group(&["a"])))).valid(|_| true);
  assert_eq!(state.root, Some(DockNode::documents(group(&["a"]))));

  let state = DockState::new(DockNode::row([
    DockNode::documents(group(&["a"])),
    group(&["b"]),
    DockNode::documents(None),
  ]))
  .valid(|_| true);
  assert_eq!(state.node_at(&[2]), None);
  assert_shares(state.root.as_ref(), &[0.5, 0.5]);

  // An empty area stays: at the root, or as a child of a split.
  let state = DockState::new(DockNode::documents(None)).valid(|_| true);
  assert_eq!(state.root, Some(DockNode::documents(None)));
  let row = DockNode::row([group(&["a"]), DockNode::documents(None)]);
  assert_eq!(DockState::new(row.clone()).valid(|_| true).root, Some(row));
}

#[test]
fn valid_clears_a_maximized_panel_that_is_not_a_docked_tab() {
  let mut state = sample();
  state.maximized = Some("ghost".into());
  assert_eq!(state.valid(|_| true).maximized, None);

  let mut state = sample();
  state.maximized = Some("terminal".into());
  assert_eq!(state.clone().valid(|_| true).maximized.as_deref(), Some("terminal"));
  assert_eq!(state.valid(|panel| panel != "terminal").maximized, None);
}

#[test]
fn valid_mends_a_floating_group() {
  let mut state = DockState::empty();
  state.floating = vec![
    DockFloat {
      group: DockGroup {
        panels: vec!["a".into()],
        shown: None,
        hidden: vec![DockHidden {
          panel: "b".into(),
          side: DockSide::Left,
        }],
      },
      position: (f32::INFINITY, 4.0),
      size: Some((0.5, 100.0)),
    },
    DockFloat {
      group: DockGroup::default(),
      position: (0.0, 0.0),
      size: None,
    },
    DockFloat {
      group: DockGroup {
        panels: vec!["c".into()],
        shown: Some("c".into()),
        hidden: Vec::new(),
      },
      position: (1.0, 2.0),
      size: Some((f32::NAN, 10.0)),
    },
  ];
  let state = state.valid(|_| true);
  assert_eq!(
    state.floating,
    vec![
      DockFloat {
        group: DockGroup {
          panels: vec!["a".into(), "b".into()],
          shown: Some("a".into()),
          hidden: Vec::new(),
        },
        position: (0.0, 0.0),
        size: None,
      },
      DockFloat {
        group: DockGroup {
          panels: vec!["c".into()],
          shown: Some("c".into()),
          hidden: Vec::new(),
        },
        position: (1.0, 2.0),
        size: None,
      },
    ]
  );
}

// The text form.

fn rich() -> DockState {
  let mut state = sample();
  assert!(state.show("problems", DockPlace::Hidden(DockSide::Bottom)));
  assert!(state.show(
    "outline",
    DockPlace::Float {
      position: (10.5, -3.0),
      size: Some((300.0, 200.25)),
    }
  ));
  assert!(state.show("search", float_at(0.0, 0.0)));
  assert!(state.show("a \"quoted\" \\ id (with: parens, commas)", tab("explorer")));
  assert!(state.show("lib.rs", beside("theme.rs", DockSide::Right)));
  assert!(state.select("theme.rs"));
  assert!(state.set_shares(&[1], &[1.0 / 3.0, 2.0 / 3.0]));
  assert!(state.maximize("terminal"));
  state
}

#[test]
fn text_round_trips_a_rich_state() {
  let state = rich();
  let text = state.to_text();
  assert_eq!(DockState::parse(&text), Some(state.clone()));
  assert_eq!(state.to_string(), text);
  assert_eq!(text.parse::<DockState>(), Ok(state));

  assert_eq!(DockState::empty().to_text(), "");
  assert_eq!(DockState::parse(""), Some(DockState::empty()));
  assert_eq!(DockState::parse("  "), Some(DockState::empty()));
}

#[test]
fn text_of_a_small_state() {
  let mut state = DockState::new(DockNode::row([
    group(&["a", "b"]).shown("b"),
    DockNode::documents(None),
  ]));
  assert!(state.show("c", DockPlace::Hidden(DockSide::Top)));
  assert!(state.maximize("a"));
  let text = state.to_text();
  assert_eq!(
    text,
    r#"column(0.25 group(hidden:top:"c") 0.75 row(0.5 group("a" "b" shown:"b") 0.5 documents())) maximized:"a""#
  );
  assert_eq!(DockState::parse(&text), Some(state));
}

#[test]
fn text_round_trips_a_state_not_yet_valid() {
  let mut state = DockState::new(
    DockNode::row([
      DockNode::Group(DockGroup {
        panels: vec!["a".into()],
        shown: None,
        hidden: Vec::new(),
      }),
      DockNode::documents(DockNode::documents(None)),
    ])
    .shares([0.1, 0.2, 0.3]),
  );
  state.floating.push(DockFloat {
    group: DockGroup::default(),
    position: (-0.5, 1e-7),
    size: None,
  });
  state.maximized = Some("ghost".into());
  let text = state.to_text();
  assert_eq!(DockState::parse(&text), Some(state));

  // Shares fewer than children, and the other way.
  let state = DockState::new(DockNode::column([group(&["a"]), group(&["b"])]).shares([1.0]));
  assert_eq!(DockState::parse(&state.to_text()), Some(state));
}

#[test]
fn text_that_is_not_the_form_is_none() {
  for garbage in [
    "row(",
    ")",
    "nonsense",
    "group",
    "group(\"a\"",
    "group(\"a\") group(\"b\")",
    "group(\"a\\x\")",
    "\"unterminated",
    "row(abc group())",
    "row(0.5 \"a\")",
    "float(\"a\")",
    "float(\"a\" at:1)",
    "float(\"a\" at:1,2 at:3,4)",
    "group(\"a\" at:1,2)",
    "group(\"a\" shown:)",
    "group(\"a\" shown:\"a\" shown:\"a\")",
    "group(hidden:up:\"a\")",
    "group(hidden:left)",
    "documents(group() group())",
    "maximized:",
    "maximized:\"a\" maximized:\"b\"",
    "group() ,",
    "box()",
    "row(0.5 group(\"a\") 0.5 group(\"b\"",
  ] {
    assert_eq!(DockState::parse(garbage), None, "{garbage}");
  }
  assert_eq!("nonsense".parse::<DockState>(), Err(DockStateParseError));
  assert!(!DockStateParseError.to_string().is_empty());

  // Deep nesting is refused, not overflowed.
  let deep = "row(".repeat(100_000);
  assert_eq!(DockState::parse(&deep), None);
  let deep = format!("{}{}", "documents(".repeat(10_000), ")".repeat(10_000));
  assert_eq!(DockState::parse(&deep), None);

  // Every cut of a rich state's text is read or refused, never a panic.
  let text = rich().to_text();
  for (at, _) in text.char_indices() {
    let _ = DockState::parse(&text[..at]);
    let _ = DockState::parse(&text[at..]);
  }
}
