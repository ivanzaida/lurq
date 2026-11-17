//! The arrangement of a dock layout as a person leaves it: a tree of splits whose leaves are groups of panels shown
//! as tabs, an area of documents among them, the groups that float, the panels hidden at an edge and the one
//! maximized; a value the app holds, stores as text and gives back.

use std::{collections::HashSet, fmt, str::FromStr};

use crate::ui::Orientation;

/// The least share of a split a child keeps: a child is never squeezed to nothing.
const MIN_SHARE: f32 = 0.05;

/// How far the shares of a split may sum from 1 and still be left as they are.
const SHARE_SLACK: f32 = 1e-4;

/// The deepest nesting `DockState::parse` reads.
const MAX_DEPTH: usize = 64;

/// An edge: of the whole layout, of a group, or the strip a panel is hidden into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DockSide {
  /// The left edge.
  Left,
  /// The right edge.
  Right,
  /// The top edge.
  Top,
  /// The bottom edge.
  Bottom,
}

impl DockSide {
  /// Every side, in the order left, right, top, bottom.
  pub const ALL: [DockSide; 4] = [DockSide::Left, DockSide::Right, DockSide::Top, DockSide::Bottom];

  /// The way a split runs that puts things on this side of each other: a row for left and right, a column for top
  /// and bottom.
  pub fn orientation(self) -> Orientation {
    match self {
      DockSide::Left | DockSide::Right => Orientation::Horizontal,
      DockSide::Top | DockSide::Bottom => Orientation::Vertical,
    }
  }

  /// Whether the side is at the start of its split: left or top.
  pub fn is_start(self) -> bool {
    matches!(self, DockSide::Left | DockSide::Top)
  }

  fn name(self) -> &'static str {
    match self {
      DockSide::Left => "left",
      DockSide::Right => "right",
      DockSide::Top => "top",
      DockSide::Bottom => "bottom",
    }
  }

  fn from_name(name: &str) -> Option<Self> {
    DockSide::ALL.into_iter().find(|side| side.name() == name)
  }
}

/// The arrangement of a dock layout: what is docked, what floats and what is maximized. Panels are named by their
/// ids; the layout draws each from the app's own. Every change made through its methods leaves it valid (see
/// `make_valid`); one built by hand, or read back from text, is brought to the rules by `valid` with the panels the
/// app knows.
///
/// ```
/// use lurq::ui::{DockNode, DockState};
///
/// let state = DockState::new(
///   DockNode::row([
///     DockNode::group(["explorer"]),
///     DockNode::column([
///       DockNode::documents(DockNode::group(["blur.rs", "theme.rs"])),
///       DockNode::group(["terminal", "problems", "output"]),
///     ])
///     .shares([0.7, 0.3]),
///   ])
///   .shares([0.25, 0.75]),
/// );
/// assert!(state.in_documents("theme.rs"));
/// assert_eq!(DockState::parse(&state.to_text()), Some(state));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockState {
  /// What is docked: `None` where nothing is.
  pub root: Option<DockNode>,
  /// Groups that float over the layout, the last in front.
  pub floating: Vec<DockFloat>,
  /// The panel whose group fills the layout, where one is maximized.
  pub maximized: Option<String>,
}

impl crate::app::component::DevtoolsInspectable for DockState {}

/// A part of the docked tree.
#[derive(Clone, Debug, PartialEq)]
pub enum DockNode {
  /// Children side by side or one above another.
  Split(DockSplit),
  /// Panels shown as tabs.
  Group(DockGroup),
  /// The area of documents: at most one in a state (the first is kept). Its groups draw document tabs.
  Documents(DockDocuments),
}

/// Children laid along a row or down a column, each with its share of the length.
#[derive(Clone, Debug, PartialEq)]
pub struct DockSplit {
  /// A row (`Horizontal`: children side by side) or a column (`Vertical`).
  pub orientation: Orientation,
  /// What it holds, in order.
  pub children: Vec<DockNode>,
  /// Each child's share of the split's length, as many as children, summing to 1.
  pub shares: Vec<f32>,
}

/// Panels shown as tabs, one at a time.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockGroup {
  /// The panels shown as its tabs, in order.
  pub panels: Vec<String>,
  /// The one shown: one of `panels`, or `None` only where `panels` is empty.
  pub shown: Option<String>,
  /// Panels of this group hidden into the strip at an edge; put back, each comes back here.
  pub hidden: Vec<DockHidden>,
}

/// A panel hidden into the strip at an edge of the layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DockHidden {
  /// The panel's id.
  pub panel: String,
  /// The edge whose strip shows it.
  pub side: DockSide,
}

/// The area of documents: where documents open, drawn with document tabs, and what the layout shows where it is
/// empty.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DockDocuments {
  /// What the area holds: `None` where it is empty. Never itself a documents area.
  pub node: Option<Box<DockNode>>,
}

/// A group floating over the layout.
#[derive(Clone, Debug, PartialEq)]
pub struct DockFloat {
  /// Its panels; its `hidden` is always empty.
  pub group: DockGroup,
  /// Its top left corner in the layout, in logical pixels.
  pub position: (f32, f32),
  /// Its width and height in logical pixels: `None` for the shown panel's own size.
  pub size: Option<(f32, f32)>,
}

/// Where a panel (or a group) goes.
#[derive(Clone, Debug, PartialEq)]
pub enum DockPlace {
  /// A tab of the group that holds the panel `of` (docked, hidden in it, or floating), after its tabs.
  Tab { of: String },
  /// A new group beside the group that holds `of`, on `side` of it (within the documents area where `of` is in
  /// it). Where `of` floats it is the same as `Tab`; where `of` is hidden, as `Edge` at the side it is hidden at.
  Beside { of: String, side: DockSide },
  /// A new group along `side` of the whole layout (outside the documents area).
  Edge(DockSide),
  /// A tab of the documents area: of its first group, or the area's only group where it is empty. A state with no
  /// documents area gets one, at the right of what is docked.
  Documents,
  /// Floating at `position`, `size` or its own.
  Float {
    position: (f32, f32),
    size: Option<(f32, f32)>,
  },
  /// Hidden into the strip at `side`: where it was docked it keeps that place (put back returns it there); where it
  /// floated or was not placed, it is put in a new group along that edge first and hidden there.
  Hidden(DockSide),
}

/// What `DockState::from_str` returns for text that is not the text form of a dock state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DockStateParseError;

impl fmt::Display for DockStateParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("not the text form of a dock state")
  }
}

impl std::error::Error for DockStateParseError {}

impl DockNode {
  /// A split of `children` side by side, each with an equal share.
  pub fn row(children: impl IntoIterator<Item = DockNode>) -> Self {
    Self::split(Orientation::Horizontal, children)
  }

  /// A split of `children` one above another, each with an equal share.
  pub fn column(children: impl IntoIterator<Item = DockNode>) -> Self {
    Self::split(Orientation::Vertical, children)
  }

  fn split(orientation: Orientation, children: impl IntoIterator<Item = DockNode>) -> Self {
    let children: Vec<DockNode> = children.into_iter().collect();
    let shares = equal_shares(children.len());
    DockNode::Split(DockSplit {
      orientation,
      children,
      shares,
    })
  }

  /// A group of `panels` as tabs, in order, showing the first.
  pub fn group<S: Into<String>>(panels: impl IntoIterator<Item = S>) -> Self {
    let panels: Vec<String> = panels.into_iter().map(Into::into).collect();
    DockNode::Group(DockGroup {
      shown: panels.first().cloned(),
      panels,
      hidden: Vec::new(),
    })
  }

  /// The area of documents holding `node`, or empty where it is `None`.
  pub fn documents(node: impl Into<Option<DockNode>>) -> Self {
    DockNode::Documents(DockDocuments {
      node: node.into().map(Box::new),
    })
  }

  /// The shares of a split's children (ignored on anything else); brought to the rules by `make_valid`.
  pub fn shares(mut self, shares: impl IntoIterator<Item = f32>) -> Self {
    if let DockNode::Split(split) = &mut self {
      split.shares = shares.into_iter().collect();
    }
    self
  }

  /// The panel a group shows (ignored on anything else).
  pub fn shown(mut self, panel: &str) -> Self {
    if let DockNode::Group(group) = &mut self {
      group.shown = Some(panel.to_owned());
    }
    self
  }

  /// The child at `index`: of a split, or the node of a documents area at 0.
  fn child(&self, index: usize) -> Option<&DockNode> {
    match self {
      DockNode::Split(split) => split.children.get(index),
      DockNode::Documents(area) if index == 0 => area.node.as_deref(),
      _ => None,
    }
  }

  fn child_mut(&mut self, index: usize) -> Option<&mut DockNode> {
    match self {
      DockNode::Split(split) => split.children.get_mut(index),
      DockNode::Documents(area) if index == 0 => area.node.as_deref_mut(),
      _ => None,
    }
  }

  /// The path below this node to the first node `hit` takes, in tree order, this one first.
  fn find(&self, path: &mut Vec<usize>, hit: &impl Fn(&DockNode) -> bool) -> bool {
    if hit(self) {
      return true;
    }
    let mut index = 0;
    while let Some(child) = self.child(index) {
      path.push(index);
      if child.find(path, hit) {
        return true;
      }
      path.pop();
      index += 1;
    }
    false
  }

  /// Every group below this node in tree order, each with whether it is in the documents area.
  fn collect_groups<'a>(&'a self, in_documents: bool, groups: &mut Vec<(&'a DockGroup, bool)>) {
    match self {
      DockNode::Group(group) => groups.push((group, in_documents)),
      DockNode::Split(split) => {
        for child in &split.children {
          child.collect_groups(in_documents, groups);
        }
      }
      DockNode::Documents(area) => {
        if let Some(node) = &area.node {
          node.collect_groups(true, groups);
        }
      }
    }
  }
}

impl DockGroup {
  /// Whether the group names `panel`, as a tab or hidden.
  fn holds(&self, panel: &str) -> bool {
    self.panels.iter().any(|name| name == panel) || self.hidden.iter().any(|hidden| hidden.panel == panel)
  }

  /// A group of the one panel, showing it.
  fn of_one(panel: &str) -> Self {
    DockGroup {
      panels: vec![panel.to_owned()],
      shown: Some(panel.to_owned()),
      hidden: Vec::new(),
    }
  }

  /// Takes the tab `panel` out; where it was shown the group shows the one after it, else the one before.
  fn remove_tab(&mut self, panel: &str) -> bool {
    let Some(at) = self.panels.iter().position(|name| name == panel) else {
      return false;
    };
    self.panels.remove(at);
    if self.shown.as_deref() == Some(panel) {
      self.shown = self.panels.get(at).or_else(|| self.panels.last()).cloned();
    }
    true
  }

  /// Puts the tabs and the hidden panels of `other` after its own, showing the one `other` shows.
  fn add_tabs(&mut self, other: DockGroup) {
    self.panels.extend(other.panels);
    self.hidden.extend(other.hidden);
    if other.shown.is_some() {
      self.shown = other.shown;
    }
  }
}

/// Where a group is: docked at a path, or floating at an index.
#[derive(Clone, Debug, PartialEq)]
enum Location {
  Docked(Vec<usize>),
  Floating(usize),
}

impl DockState {
  /// A state of `root` docked, nothing floating and nothing maximized.
  pub fn new(root: DockNode) -> Self {
    Self {
      root: Some(root),
      floating: Vec::new(),
      maximized: None,
    }
  }

  /// A state of nothing.
  pub fn empty() -> Self {
    Self::default()
  }

  /// Every docked group in tree order, each with whether it is in the documents area.
  fn docked_groups(&self) -> Vec<(&DockGroup, bool)> {
    let mut groups = Vec::new();
    if let Some(root) = &self.root {
      root.collect_groups(false, &mut groups);
    }
    groups
  }

  /// Whether the state names `panel`: docked, hidden, or floating.
  pub fn contains(&self, panel: &str) -> bool {
    self.group_of(panel).is_some()
  }

  /// Every panel the state names, in tree order (each group's tabs, then its hidden ones) then floating.
  pub fn panels(&self) -> Vec<String> {
    let groups = self
      .docked_groups()
      .into_iter()
      .map(|(group, _)| group)
      .chain(self.floating.iter().map(|float| &float.group));
    let mut panels = Vec::new();
    for group in groups {
      panels.extend(group.panels.iter().cloned());
      panels.extend(group.hidden.iter().map(|hidden| hidden.panel.clone()));
    }
    panels
  }

  /// The side `panel` is hidden at, where it is hidden.
  pub fn is_hidden(&self, panel: &str) -> Option<DockSide> {
    self
      .docked_groups()
      .into_iter()
      .flat_map(|(group, _)| &group.hidden)
      .find(|hidden| hidden.panel == panel)
      .map(|hidden| hidden.side)
  }

  /// Whether `panel` is in a floating group.
  pub fn is_floating(&self, panel: &str) -> bool {
    self.floating.iter().any(|float| float.group.holds(panel))
  }

  /// Every hidden panel, in tree order.
  pub fn hidden(&self) -> Vec<DockHidden> {
    self
      .docked_groups()
      .into_iter()
      .flat_map(|(group, _)| group.hidden.iter().cloned())
      .collect()
  }

  /// The group that holds `panel` (docked, hidden in it, or floating).
  pub fn group_of(&self, panel: &str) -> Option<&DockGroup> {
    self
      .docked_groups()
      .into_iter()
      .map(|(group, _)| group)
      .chain(self.floating.iter().map(|float| &float.group))
      .find(|group| group.holds(panel))
  }

  /// Whether `panel` is in the documents area.
  pub fn in_documents(&self, panel: &str) -> bool {
    self
      .docked_groups()
      .into_iter()
      .any(|(group, in_documents)| in_documents && group.holds(panel))
  }

  /// The node at `path`: child indices from the root through splits, where a documents area counts as one level
  /// whose only child (index 0) is its node. The empty path is the root.
  ///
  /// ```
  /// use lurq::ui::{DockNode, DockState};
  ///
  /// let state = DockState::new(DockNode::row([
  ///   DockNode::group(["explorer"]),
  ///   DockNode::column([
  ///     DockNode::documents(DockNode::group(["blur.rs"])),
  ///     DockNode::group(["terminal"]),
  ///   ]),
  /// ]));
  /// assert_eq!(state.node_at(&[1, 0, 0]), Some(&DockNode::group(["blur.rs"])));
  /// ```
  pub fn node_at(&self, path: &[usize]) -> Option<&DockNode> {
    let mut node = self.root.as_ref()?;
    for &index in path {
      node = node.child(index)?;
    }
    Some(node)
  }

  fn node_at_mut(&mut self, path: &[usize]) -> Option<&mut DockNode> {
    let mut node = self.root.as_mut()?;
    for &index in path {
      node = node.child_mut(index)?;
    }
    Some(node)
  }

  fn path_of(&self, hit: impl Fn(&DockNode) -> bool) -> Option<Vec<usize>> {
    let mut path = Vec::new();
    self.root.as_ref()?.find(&mut path, &hit).then_some(path)
  }

  fn docked_group_mut(&mut self, path: &[usize]) -> Option<&mut DockGroup> {
    match self.node_at_mut(path)? {
      DockNode::Group(group) => Some(group),
      _ => None,
    }
  }

  /// Where the group that holds `panel` is.
  fn location_of(&self, panel: &str) -> Option<Location> {
    if let Some(path) = self.path_of(|node| matches!(node, DockNode::Group(group) if group.holds(panel))) {
      return Some(Location::Docked(path));
    }
    self
      .floating
      .iter()
      .position(|float| float.group.holds(panel))
      .map(Location::Floating)
  }

  fn group_at_mut(&mut self, location: &Location) -> Option<&mut DockGroup> {
    match location {
      Location::Docked(path) => self.docked_group_mut(path),
      Location::Floating(index) => self.floating.get_mut(*index).map(|float| &mut float.group),
    }
  }

  // Changing it.

  /// Puts `panel` at `place`: moved from where it is, or added where the state did not name it. Returns whether
  /// anything changed. A place that names a panel the state does not hold changes nothing. Moving a panel to a
  /// place in its own group that it already is changes nothing. The panel moved is shown in its new group; moving
  /// the maximized panel ends the maximize.
  ///
  /// ```
  /// use lurq::ui::{DockNode, DockPlace, DockSide, DockState};
  ///
  /// let mut state = DockState::new(DockNode::group(["explorer"]));
  /// state.show("terminal", DockPlace::Edge(DockSide::Bottom));
  /// assert_eq!(
  ///   state.root,
  ///   Some(DockNode::column([DockNode::group(["explorer"]), DockNode::group(["terminal"])]).shares([0.75, 0.25]))
  /// );
  /// ```
  pub fn show(&mut self, panel: &str, place: DockPlace) -> bool {
    let mut next = self.clone();
    if !next.put_panel(panel, place) {
      return false;
    }
    self.commit(next, &[panel.to_owned()])
  }

  /// The same as `show` for a panel the state holds; false where it does not.
  pub fn move_panel(&mut self, panel: &str, place: DockPlace) -> bool {
    self.contains(panel) && self.show(panel, place)
  }

  /// Moves the whole group that holds `of` (its panels in order, the one shown kept) to `place`. `Tab` puts its
  /// panels after the other group's tabs; `Beside`/`Edge`/`Documents`/`Float` keep it one group. `Hidden` hides each
  /// panel. The panels hidden from a docked group go with it, but to `Float`, where they stay where the group was.
  pub fn move_group(&mut self, of: &str, place: DockPlace) -> bool {
    let Some(group) = self.group_of(of) else {
      return false;
    };
    let moved: Vec<String> = group
      .panels
      .iter()
      .cloned()
      .chain(group.hidden.iter().map(|hidden| hidden.panel.clone()))
      .collect();
    let mut next = self.clone();
    if !next.put_group(of, place) {
      return false;
    }
    self.commit(next, &moved)
  }

  /// Takes `panel` out of the state wherever it is. Its group goes where it was the last; closing the maximized
  /// panel ends the maximize.
  pub fn close(&mut self, panel: &str) -> bool {
    let mut next = self.clone();
    if next.take_panel(panel).is_none() {
      return false;
    }
    self.commit(next, &[panel.to_owned()])
  }

  /// Makes `panel` the one its group shows. False where it is hidden or not held, or already shown.
  pub fn select(&mut self, panel: &str) -> bool {
    let Some(location) = self.location_of(panel) else {
      return false;
    };
    let Some(group) = self.group_at_mut(&location) else {
      return false;
    };
    if !group.panels.iter().any(|name| name == panel) || group.shown.as_deref() == Some(panel) {
      return false;
    }
    group.shown = Some(panel.to_owned());
    true
  }

  /// Puts a hidden panel back in its group, after its tabs, shown.
  pub fn put_back(&mut self, panel: &str) -> bool {
    let Some(path) = self.path_of(
      |node| matches!(node, DockNode::Group(group) if group.hidden.iter().any(|hidden| hidden.panel == panel)),
    ) else {
      return false;
    };
    let Some(group) = self.docked_group_mut(&path) else {
      return false;
    };
    group.hidden.retain(|hidden| hidden.panel != panel);
    group.panels.push(panel.to_owned());
    group.shown = Some(panel.to_owned());
    true
  }

  /// Maximizes the group of `panel` (docked only; not floating, not hidden). `restore` ends it.
  pub fn maximize(&mut self, panel: &str) -> bool {
    if !self.is_docked_tab(panel) || self.maximized.as_deref() == Some(panel) {
      return false;
    }
    self.maximized = Some(panel.to_owned());
    true
  }

  /// Ends the maximize; false where nothing is maximized.
  pub fn restore(&mut self) -> bool {
    self.maximized.take().is_some()
  }

  /// The shares of the split at `path` (see `node_at`), brought to the rules. False where `path` is no split or
  /// the count differs, or nothing changed.
  pub fn set_shares(&mut self, path: &[usize], shares: &[f32]) -> bool {
    let mut next = self.clone();
    let Some(DockNode::Split(split)) = next.node_at_mut(path) else {
      return false;
    };
    if split.children.len() != shares.len() {
      return false;
    }
    split.shares = shares.to_vec();
    self.commit(next, &[])
  }

  /// The place and size of the floating group at `index`, brought to the rules. False where there is none, or
  /// nothing changed.
  pub fn set_float(&mut self, index: usize, position: (f32, f32), size: Option<(f32, f32)>) -> bool {
    let mut next = self.clone();
    let Some(float) = next.floating.get_mut(index) else {
      return false;
    };
    float.position = position;
    float.size = size;
    self.commit(next, &[])
  }

  /// Brings the floating group at `index` to the front (the end of `floating`). False where there is none, or it is
  /// in front already.
  pub fn raise_float(&mut self, index: usize) -> bool {
    if index + 1 >= self.floating.len() {
      return false;
    }
    let float = self.floating.remove(index);
    self.floating.push(float);
    true
  }

  /// Whether `panel` is a tab of a docked group.
  fn is_docked_tab(&self, panel: &str) -> bool {
    self
      .docked_groups()
      .into_iter()
      .any(|(group, _)| group.panels.iter().any(|name| name == panel))
  }

  /// Makes `next` the state where what is docked or floating differs, ending the maximize of a panel in `moved`.
  fn commit(&mut self, mut next: DockState, moved: &[String]) -> bool {
    next.make_valid(|_| true);
    if next.root == self.root && next.floating == self.floating {
      return false;
    }
    if next.maximized.as_ref().is_some_and(|panel| moved.contains(panel)) {
      next.maximized = None;
    }
    *self = next;
    true
  }

  /// Takes `panel` out of wherever it is, leaving its group (perhaps empty) in place. Returns the side it was hidden
  /// at, or `None` for a tab.
  fn take_panel(&mut self, panel: &str) -> Option<Option<DockSide>> {
    let location = self.location_of(panel)?;
    let group = self.group_at_mut(&location)?;
    if group.remove_tab(panel) {
      return Some(None);
    }
    let at = group.hidden.iter().position(|hidden| hidden.panel == panel)?;
    Some(Some(group.hidden.remove(at).side))
  }

  /// Takes the tabs of the group that holds `of` out, and its hidden panels unless `keep_hidden`; a floating group
  /// is taken whole.
  fn take_group(&mut self, of: &str, keep_hidden: bool) -> Option<DockGroup> {
    match self.location_of(of)? {
      Location::Floating(index) => Some(self.floating.remove(index).group),
      Location::Docked(path) => {
        let group = self.docked_group_mut(&path)?;
        let taken = DockGroup {
          panels: std::mem::take(&mut group.panels),
          shown: group.shown.take(),
          hidden: if keep_hidden {
            Vec::new()
          } else {
            std::mem::take(&mut group.hidden)
          },
        };
        Some(taken)
      }
    }
  }

  /// Puts `panel` at `place`; false where the place changes nothing or names a panel the state does not hold.
  fn put_panel(&mut self, panel: &str, place: DockPlace) -> bool {
    match place {
      DockPlace::Tab { of } => {
        if of == panel || !self.contains(&of) {
          return false;
        }
        self.take_panel(panel);
        self.make_valid(|_| true);
        self.add_tab_of(&of, DockGroup::of_one(panel))
      }
      DockPlace::Beside { of, side } => {
        let Some(location) = self.location_of(&of) else {
          return false;
        };
        if let Location::Floating(_) = location {
          return self.put_panel(panel, DockPlace::Tab { of });
        }
        if let Some(hidden) = self.is_hidden(&of) {
          return self.put_panel(panel, DockPlace::Edge(hidden));
        }
        // Beside its own group: the group is found by another of its tabs once the panel is out of it.
        let of = if of == panel {
          let Some(other) = self
            .group_of(panel)
            .and_then(|group| group.panels.iter().find(|name| *name != panel).cloned())
          else {
            return false;
          };
          other
        } else {
          of
        };
        self.take_panel(panel);
        self.make_valid(|_| true);
        let Some(Location::Docked(path)) = self.location_of(&of) else {
          return false;
        };
        self.insert_beside(&path, DockGroup::of_one(panel), side);
        true
      }
      DockPlace::Edge(side) => {
        self.take_panel(panel);
        self.make_valid(|_| true);
        self.insert_edge(DockGroup::of_one(panel), side);
        true
      }
      DockPlace::Documents => {
        self.take_panel(panel);
        self.make_valid(|_| true);
        self.insert_documents(DockGroup::of_one(panel), true);
        true
      }
      DockPlace::Float { position, size } => {
        self.take_panel(panel);
        self.make_valid(|_| true);
        self.floating.push(DockFloat {
          group: DockGroup::of_one(panel),
          position,
          size,
        });
        true
      }
      DockPlace::Hidden(side) => {
        if let Some(path) = self.path_of(|node| matches!(node, DockNode::Group(group) if group.holds(panel)))
          && let Some(group) = self.docked_group_mut(&path)
        {
          if let Some(hidden) = group.hidden.iter_mut().find(|hidden| hidden.panel == panel) {
            hidden.side = side;
          } else {
            group.remove_tab(panel);
            group.hidden.push(DockHidden {
              panel: panel.to_owned(),
              side,
            });
          }
          return true;
        }
        self.take_panel(panel);
        self.make_valid(|_| true);
        self.insert_edge(
          DockGroup {
            panels: Vec::new(),
            shown: None,
            hidden: vec![DockHidden {
              panel: panel.to_owned(),
              side,
            }],
          },
          side,
        );
        true
      }
    }
  }

  /// Puts the group that holds `of` at `place`; false where the place changes nothing or names a panel the state
  /// does not hold.
  fn put_group(&mut self, of: &str, place: DockPlace) -> bool {
    let Some(from) = self.location_of(of) else {
      return false;
    };
    match place {
      DockPlace::Tab { of: target } | DockPlace::Beside { of: target, .. }
        if self.location_of(&target).is_none_or(|to| to == from) =>
      {
        false
      }
      DockPlace::Tab { of: target } => {
        let Some(group) = self.take_group(of, false) else {
          return false;
        };
        self.make_valid(|_| true);
        self.add_tab_of(&target, group)
      }
      DockPlace::Beside { of: target, side } => {
        if let Some(Location::Floating(_)) = self.location_of(&target) {
          return self.put_group(of, DockPlace::Tab { of: target });
        }
        if let Some(hidden) = self.is_hidden(&target) {
          return self.put_group(of, DockPlace::Edge(hidden));
        }
        let Some(group) = self.take_group(of, false) else {
          return false;
        };
        self.make_valid(|_| true);
        let Some(Location::Docked(path)) = self.location_of(&target) else {
          return false;
        };
        self.insert_beside(&path, group, side);
        true
      }
      DockPlace::Edge(side) => {
        let Some(group) = self.take_group(of, false) else {
          return false;
        };
        self.make_valid(|_| true);
        self.insert_edge(group, side);
        true
      }
      DockPlace::Documents => {
        let Some(group) = self.take_group(of, false) else {
          return false;
        };
        self.make_valid(|_| true);
        self.insert_documents(group, false);
        true
      }
      DockPlace::Float { position, size } => {
        let Some(group) = self.take_group(of, true) else {
          return false;
        };
        if group.panels.is_empty() {
          return false;
        }
        self.make_valid(|_| true);
        self.floating.push(DockFloat { group, position, size });
        true
      }
      DockPlace::Hidden(side) => {
        let hide = |group: &mut DockGroup| {
          let panels = std::mem::take(&mut group.panels);
          group.shown = None;
          group
            .hidden
            .extend(panels.into_iter().map(|panel| DockHidden { panel, side }));
        };
        match from {
          Location::Docked(path) => {
            let Some(group) = self.docked_group_mut(&path) else {
              return false;
            };
            hide(group);
          }
          Location::Floating(index) => {
            let mut group = self.floating.remove(index).group;
            hide(&mut group);
            self.make_valid(|_| true);
            self.insert_edge(group, side);
          }
        }
        true
      }
    }
  }

  /// Puts `group`'s panels after the tabs of the group that holds `of`, showing the one `group` shows.
  fn add_tab_of(&mut self, of: &str, group: DockGroup) -> bool {
    let Some(location) = self.location_of(of) else {
      return false;
    };
    let Some(target) = self.group_at_mut(&location) else {
      return false;
    };
    if let Location::Floating(_) = location {
      // A floating group hides nothing: what was hidden comes along as tabs.
      let mut group = group;
      group.panels.extend(group.hidden.drain(..).map(|hidden| hidden.panel));
      target.add_tabs(group);
    } else {
      target.add_tabs(group);
    }
    true
  }

  /// Puts `group` on `side` of the node at `path`: into its parent split where that runs the same way, the two
  /// sharing its share half and half; otherwise in a new split of the two in its place.
  fn insert_beside(&mut self, path: &[usize], group: DockGroup, side: DockSide) {
    let orientation = side.orientation();
    let new = DockNode::Group(group);
    if let Some((&last, parent)) = path.split_last()
      && let Some(DockNode::Split(split)) = self.node_at_mut(parent)
      && split.orientation == orientation
      && last < split.children.len()
    {
      if split.shares.len() != split.children.len() {
        split.shares = equal_shares(split.children.len());
      }
      let half = split.shares[last] / 2.0;
      split.shares[last] = half;
      let at = if side.is_start() { last } else { last + 1 };
      split.children.insert(at, new);
      split.shares.insert(at, half);
      return;
    }
    let Some(slot) = self.node_at_mut(path) else {
      return;
    };
    let old = std::mem::replace(slot, DockNode::documents(None));
    let children = if side.is_start() {
      vec![new, old]
    } else {
      vec![old, new]
    };
    *slot = DockNode::Split(DockSplit {
      orientation,
      children,
      shares: vec![0.5, 0.5],
    });
  }

  /// Puts `group` along `side` of the whole layout with a quarter of its length, the rest scaled to the other
  /// three quarters.
  fn insert_edge(&mut self, group: DockGroup, side: DockSide) {
    let orientation = side.orientation();
    let new = DockNode::Group(group);
    self.root = Some(match self.root.take() {
      None => new,
      Some(DockNode::Split(mut split)) if split.orientation == orientation => {
        if split.shares.len() != split.children.len() {
          split.shares = equal_shares(split.children.len());
        }
        for share in &mut split.shares {
          *share *= 0.75;
        }
        let at = if side.is_start() { 0 } else { split.children.len() };
        split.children.insert(at, new);
        split.shares.insert(at, 0.25);
        DockNode::Split(split)
      }
      Some(old) => {
        let (children, shares) = if side.is_start() {
          (vec![new, old], vec![0.25, 0.75])
        } else {
          (vec![old, new], vec![0.75, 0.25])
        };
        DockNode::Split(DockSplit {
          orientation,
          children,
          shares,
        })
      }
    });
  }

  /// Puts `group` in the documents area: as tabs of its first group where `as_tabs`, else beside that group on its
  /// right; the area's node where it is empty. A state with no area gets one at the right of what is docked.
  fn insert_documents(&mut self, group: DockGroup, as_tabs: bool) {
    let Some(area) = self.path_of(|node| matches!(node, DockNode::Documents(_))) else {
      let documents = DockNode::documents(DockNode::Group(group));
      self.root = Some(match self.root.take() {
        None => documents,
        Some(old) => DockNode::row([old, documents]).shares([0.75, 0.25]),
      });
      return;
    };
    let mut first = area.clone();
    first.push(0);
    let found = self.node_at(&first).and_then(|node| {
      let mut path = first.clone();
      node
        .find(&mut path, &|node| matches!(node, DockNode::Group(_)))
        .then_some(path)
    });
    match found {
      Some(path) if as_tabs => {
        if let Some(target) = self.docked_group_mut(&path) {
          target.add_tabs(group);
        }
      }
      Some(path) => self.insert_beside(&path, group, DockSide::Right),
      None => {
        if let Some(DockNode::Documents(documents)) = self.node_at_mut(&area) {
          match documents.node.take() {
            None => documents.node = Some(Box::new(DockNode::Group(group))),
            Some(node) => {
              documents.node = Some(Box::new(DockNode::row([*node, DockNode::Group(group)])));
            }
          }
        }
      }
    }
  }

  // Making it valid.

  /// Brings the state to the rules, refusing nothing:
  /// 1. A panel `known` says no to is left out everywhere. A panel named twice keeps its first place (tree order, each
  ///    group's tabs before its hidden ones, then floating).
  /// 2. A group with no panels and no hidden goes; a floating group with no panels goes, and the panels it hides become
  ///    its tabs. A `shown` not one of `panels` becomes the first, or `None`.
  /// 3. A split with no children goes; a split of one child is replaced by the child; a child that is a split of the
  ///    same orientation is flattened into it, its shares multiplied in.
  /// 4. Shares: a count that differs from the children's becomes equal shares; a share not finite or below 0.05 becomes
  ///    0.05; then they are scaled to sum to 1, none below 0.05 (below `1 / n` for more than 20 children).
  /// 5. The first documents area is kept; a later one, or one inside it, is replaced by its node (or goes where empty).
  ///    An empty documents area stays: it is where the layout shows what it shows when empty.
  /// 6. A `maximized` that is not a tab of a docked group becomes `None`.
  /// 7. A floating size not finite or below 1 becomes `None`; a position not finite becomes (0, 0).
  pub fn make_valid(&mut self, known: impl Fn(&str) -> bool) {
    let mut cleaner = Cleaner {
      known: &known,
      seen: HashSet::new(),
      documents: false,
    };
    self.root = self.root.take().and_then(|node| cleaner.node(node));
    let floating = std::mem::take(&mut self.floating);
    for mut float in floating {
      let hidden = std::mem::take(&mut float.group.hidden);
      float.group.panels.extend(hidden.into_iter().map(|hidden| hidden.panel));
      cleaner.group(&mut float.group);
      if float.group.panels.is_empty() {
        continue;
      }
      let (x, y) = float.position;
      if !x.is_finite() || !y.is_finite() {
        float.position = (0.0, 0.0);
      }
      if let Some((width, height)) = float.size
        && !(width.is_finite() && height.is_finite() && width >= 1.0 && height >= 1.0)
      {
        float.size = None;
      }
      self.floating.push(float);
    }
    if let Some(panel) = &self.maximized
      && !self.is_docked_tab(panel)
    {
      self.maximized = None;
    }
  }

  /// The state brought to the rules (see `make_valid`).
  pub fn valid(mut self, known: impl Fn(&str) -> bool) -> Self {
    self.make_valid(known);
    self
  }

  // The text form.

  /// The state as text that `parse` reads back to the same state: the root node, then each floating group, then
  /// the maximized panel, separated by spaces.
  ///
  /// - A split is `row(..)` or `column(..)` of its shares and children in turn: `row(0.25 group(..) 0.75 group(..))`.
  /// - A group is `group(..)` of its panels as quoted ids, then `shown:"id"` where it shows other than the first
  ///   (`shown:none` where it shows nothing though it has panels), then each hidden one as `hidden:left:"id"`.
  /// - The documents area is `documents(..)` of its node, or `documents()`.
  /// - A floating group is `float(..)` of what a group writes, then `at:x,y`, then `size:w,h` where it has one.
  /// - The maximized panel is `maximized:"id"`.
  ///
  /// Ids are quoted with `\"` and `\\` escaped; numbers are written as Rust writes them.
  ///
  /// ```
  /// use lurq::ui::{DockNode, DockPlace, DockState};
  ///
  /// let mut state = DockState::new(DockNode::row([DockNode::group(["explorer"]), DockNode::group(["terminal"])]));
  /// state.show("outline", DockPlace::Float { position: (10.0, 20.0), size: Some((300.0, 200.0)) });
  /// assert_eq!(
  ///   state.to_text(),
  ///   r#"row(0.5 group("explorer") 0.5 group("terminal")) float("outline" at:10,20 size:300,200)"#
  /// );
  /// ```
  pub fn to_text(&self) -> String {
    let mut out = String::new();
    if let Some(root) = &self.root {
      write_node(&mut out, root);
    }
    for float in &self.floating {
      if !out.is_empty() {
        out.push(' ');
      }
      out.push_str("float(");
      let mut first = write_group_items(&mut out, &float.group);
      space(&mut out, &mut first);
      out.push_str(&format!("at:{},{}", float.position.0, float.position.1));
      if let Some((width, height)) = float.size {
        out.push_str(&format!(" size:{width},{height}"));
      }
      out.push(')');
    }
    if let Some(panel) = &self.maximized {
      if !out.is_empty() {
        out.push(' ');
      }
      out.push_str("maximized:");
      write_string(&mut out, panel);
    }
    out
  }

  /// The state `text` writes (see `to_text`), as written: `None` for text that is not the form. Bring it to the
  /// rules with `valid` and the panels the app knows.
  pub fn parse(text: &str) -> Option<DockState> {
    let tokens = tokenize(text)?;
    let mut parser = Parser {
      tokens,
      at: 0,
      depth: 0,
    };
    parser.state()
  }
}

impl fmt::Display for DockState {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.to_text())
  }
}

impl FromStr for DockState {
  type Err = DockStateParseError;

  fn from_str(text: &str) -> Result<Self, Self::Err> {
    DockState::parse(text).ok_or(DockStateParseError)
  }
}

/// Stored as its text form, the way a string is.
#[cfg(feature = "persistent_storage")]
impl crate::persistent_storage::PersistentValue for DockState {
  fn decode_persistent_value(bytes: &[u8]) -> Option<Self> {
    let text = <String as crate::persistent_storage::PersistentValue>::decode_persistent_value(bytes)?;
    DockState::parse(&text)
  }
}

/// Stored as its text form, the way a string is.
#[cfg(feature = "persistent_storage")]
impl crate::persistent_storage::IntoPersistentValue for DockState {
  fn encode_persistent_value(self) -> Vec<u8> {
    crate::persistent_storage::IntoPersistentValue::encode_persistent_value(self.to_text())
  }
}

/// `count` equal shares.
fn equal_shares(count: usize) -> Vec<f32> {
  vec![1.0 / count.max(1) as f32; count]
}

/// Brings `shares` to rule 4 of `make_valid`, the count already right. Shares that keep it already are left as
/// they are, so a valid state stays the same to the bit.
fn normalize(shares: &mut [f32]) {
  let count = shares.len();
  if count == 0 {
    return;
  }
  let min = MIN_SHARE.min(1.0 / count as f32);
  for share in shares.iter_mut() {
    if !share.is_finite() || *share < min {
      *share = min;
    }
  }
  let sum: f32 = shares.iter().sum();
  if (sum - 1.0).abs() <= SHARE_SLACK {
    return;
  }
  // Scaled to sum to 1, those that would fall below the least held at it and the rest scaled to what is left.
  let mut held = vec![false; count];
  loop {
    let free: f32 = shares
      .iter()
      .zip(&held)
      .filter(|(_, held)| !**held)
      .map(|(share, _)| *share)
      .sum();
    let held_count = held.iter().filter(|held| **held).count();
    let room = 1.0 - min * held_count as f32;
    if free <= 0.0 {
      for share in shares.iter_mut() {
        *share = min;
      }
      return;
    }
    let scale = room / free;
    let mut more = false;
    for (share, held) in shares.iter().zip(held.iter_mut()) {
      if !*held && share * scale < min {
        *held = true;
        more = true;
      }
    }
    if !more {
      for (share, held) in shares.iter_mut().zip(&held) {
        *share = if *held { min } else { *share * scale };
      }
      return;
    }
  }
}

/// What `make_valid` walks the tree with.
struct Cleaner<'a, K: Fn(&str) -> bool> {
  known: &'a K,
  /// The panels already placed.
  seen: HashSet<String>,
  /// Whether the first documents area is placed.
  documents: bool,
}

impl<K: Fn(&str) -> bool> Cleaner<'_, K> {
  fn group(&mut self, group: &mut DockGroup) {
    let known = self.known;
    let seen = &mut self.seen;
    group.panels.retain(|panel| known(panel) && seen.insert(panel.clone()));
    group
      .hidden
      .retain(|hidden| known(&hidden.panel) && seen.insert(hidden.panel.clone()));
    if !group.shown.as_ref().is_some_and(|shown| group.panels.contains(shown)) {
      group.shown = group.panels.first().cloned();
    }
  }

  fn node(&mut self, node: DockNode) -> Option<DockNode> {
    match node {
      DockNode::Group(mut group) => {
        self.group(&mut group);
        (!group.panels.is_empty() || !group.hidden.is_empty()).then_some(DockNode::Group(group))
      }
      DockNode::Documents(area) => {
        if self.documents {
          return area.node.and_then(|node| self.node(*node));
        }
        self.documents = true;
        let node = area.node.and_then(|node| self.node(*node)).map(Box::new);
        Some(DockNode::Documents(DockDocuments { node }))
      }
      DockNode::Split(split) => self.split(split),
    }
  }

  fn split(&mut self, split: DockSplit) -> Option<DockNode> {
    let count = split.children.len();
    let mut shares = if split.shares.len() == count {
      split.shares
    } else {
      equal_shares(count)
    };
    let min = MIN_SHARE.min(1.0 / count.max(1) as f32);
    for share in &mut shares {
      if !share.is_finite() || *share < min {
        *share = min;
      }
    }
    let mut children = Vec::with_capacity(count);
    let mut kept = Vec::with_capacity(count);
    for (child, share) in split.children.into_iter().zip(shares) {
      match self.node(child) {
        None => {}
        Some(DockNode::Split(inner)) if inner.orientation == split.orientation => {
          for (child, inner_share) in inner.children.into_iter().zip(inner.shares) {
            children.push(child);
            kept.push(inner_share * share);
          }
        }
        Some(child) => {
          children.push(child);
          kept.push(share);
        }
      }
    }
    match children.len() {
      0 => None,
      1 => children.pop(),
      _ => {
        normalize(&mut kept);
        Some(DockNode::Split(DockSplit {
          orientation: split.orientation,
          children,
          shares: kept,
        }))
      }
    }
  }
}

// Writing the text form.

fn space(out: &mut String, first: &mut bool) {
  if !*first {
    out.push(' ');
  }
  *first = false;
}

fn write_string(out: &mut String, text: &str) {
  out.push('"');
  for c in text.chars() {
    if c == '"' || c == '\\' {
      out.push('\\');
    }
    out.push(c);
  }
  out.push('"');
}

fn write_node(out: &mut String, node: &DockNode) {
  match node {
    DockNode::Split(split) => {
      out.push_str(match split.orientation {
        Orientation::Horizontal => "row(",
        Orientation::Vertical => "column(",
      });
      let mut first = true;
      for index in 0..split.children.len().max(split.shares.len()) {
        if let Some(share) = split.shares.get(index) {
          space(out, &mut first);
          out.push_str(&share.to_string());
        }
        if let Some(child) = split.children.get(index) {
          space(out, &mut first);
          write_node(out, child);
        }
      }
      out.push(')');
    }
    DockNode::Group(group) => {
      out.push_str("group(");
      write_group_items(out, group);
      out.push(')');
    }
    DockNode::Documents(area) => {
      out.push_str("documents(");
      if let Some(node) = &area.node {
        write_node(out, node);
      }
      out.push(')');
    }
  }
}

/// Writes what is inside a group's parentheses; returns whether it wrote nothing.
fn write_group_items(out: &mut String, group: &DockGroup) -> bool {
  let mut first = true;
  for panel in &group.panels {
    space(out, &mut first);
    write_string(out, panel);
  }
  if group.shown.as_ref() != group.panels.first() {
    space(out, &mut first);
    out.push_str("shown:");
    match &group.shown {
      Some(shown) => write_string(out, shown),
      None => out.push_str("none"),
    }
  }
  for hidden in &group.hidden {
    space(out, &mut first);
    out.push_str("hidden:");
    out.push_str(hidden.side.name());
    out.push(':');
    write_string(out, &hidden.panel);
  }
  first
}

// Reading the text form.

#[derive(Clone, Debug, PartialEq)]
enum Token {
  Open,
  Close,
  Comma,
  Colon,
  Text(String),
  Word(String),
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
  let mut tokens = Vec::new();
  let mut chars = text.chars().peekable();
  while let Some(&c) = chars.peek() {
    match c {
      c if c.is_whitespace() => {
        chars.next();
      }
      '(' | ')' | ',' | ':' => {
        chars.next();
        tokens.push(match c {
          '(' => Token::Open,
          ')' => Token::Close,
          ',' => Token::Comma,
          _ => Token::Colon,
        });
      }
      '"' => {
        chars.next();
        let mut string = String::new();
        loop {
          match chars.next()? {
            '"' => break,
            '\\' => match chars.next()? {
              escaped @ ('"' | '\\') => string.push(escaped),
              _ => return None,
            },
            c => string.push(c),
          }
        }
        tokens.push(Token::Text(string));
      }
      _ => {
        let mut word = String::new();
        while let Some(&c) = chars.peek() {
          if c.is_whitespace() || matches!(c, '(' | ')' | ',' | ':' | '"') {
            break;
          }
          word.push(c);
          chars.next();
        }
        tokens.push(Token::Word(word));
      }
    }
  }
  Some(tokens)
}

/// What a group's (or a floating group's) parentheses hold.
struct GroupItems {
  group: DockGroup,
  position: Option<(f32, f32)>,
  size: Option<(f32, f32)>,
}

struct Parser {
  tokens: Vec<Token>,
  at: usize,
  depth: usize,
}

impl Parser {
  fn peek(&self) -> Option<&Token> {
    self.tokens.get(self.at)
  }

  fn peek_word(&self) -> Option<&str> {
    match self.peek()? {
      Token::Word(word) => Some(word),
      _ => None,
    }
  }

  fn next(&mut self) -> Option<Token> {
    let token = self.tokens.get(self.at).cloned();
    self.at += 1;
    token
  }

  fn expect(&mut self, token: Token) -> Option<()> {
    (self.next()? == token).then_some(())
  }

  fn word(&mut self) -> Option<String> {
    match self.next()? {
      Token::Word(word) => Some(word),
      _ => None,
    }
  }

  fn text(&mut self) -> Option<String> {
    match self.next()? {
      Token::Text(text) => Some(text),
      _ => None,
    }
  }

  fn number(&mut self) -> Option<f32> {
    self.word()?.parse().ok()
  }

  fn pair(&mut self) -> Option<(f32, f32)> {
    let first = self.number()?;
    self.expect(Token::Comma)?;
    Some((first, self.number()?))
  }

  fn state(&mut self) -> Option<DockState> {
    let mut state = DockState::empty();
    while self.peek().is_some() {
      match self.peek_word()? {
        "maximized" => {
          self.next();
          self.expect(Token::Colon)?;
          if state.maximized.is_some() {
            return None;
          }
          state.maximized = Some(self.text()?);
        }
        "float" => {
          self.next();
          self.expect(Token::Open)?;
          let GroupItems { group, position, size } = self.group_items(true)?;
          state.floating.push(DockFloat {
            group,
            position: position?,
            size,
          });
        }
        _ => {
          if state.root.is_some() {
            return None;
          }
          state.root = Some(self.node()?);
        }
      }
    }
    Some(state)
  }

  fn node(&mut self) -> Option<DockNode> {
    self.depth += 1;
    if self.depth > MAX_DEPTH {
      return None;
    }
    let kind = self.word()?;
    self.expect(Token::Open)?;
    let node = match kind.as_str() {
      "row" | "column" => {
        let orientation = if kind == "row" {
          Orientation::Horizontal
        } else {
          Orientation::Vertical
        };
        let mut children = Vec::new();
        let mut shares = Vec::new();
        loop {
          match self.peek()? {
            Token::Close => {
              self.next();
              break;
            }
            Token::Word(_) if self.tokens.get(self.at + 1) == Some(&Token::Open) => children.push(self.node()?),
            Token::Word(_) => shares.push(self.number()?),
            _ => return None,
          }
        }
        DockNode::Split(DockSplit {
          orientation,
          children,
          shares,
        })
      }
      "group" => DockNode::Group(self.group_items(false)?.group),
      "documents" => {
        let node = if self.peek()? == &Token::Close {
          None
        } else {
          Some(Box::new(self.node()?))
        };
        self.expect(Token::Close)?;
        DockNode::Documents(DockDocuments { node })
      }
      _ => return None,
    };
    self.depth -= 1;
    Some(node)
  }

  /// Reads what is inside a group's (or a floating group's) parentheses, and the closing one.
  fn group_items(&mut self, float: bool) -> Option<GroupItems> {
    let mut group = DockGroup::default();
    let mut shown = None;
    let mut position = None;
    let mut size = None;
    loop {
      match self.next()? {
        Token::Close => break,
        Token::Text(panel) => group.panels.push(panel),
        Token::Word(word) => {
          self.expect(Token::Colon)?;
          match word.as_str() {
            "shown" if shown.is_none() => {
              shown = Some(match self.next()? {
                Token::Text(panel) => Some(panel),
                Token::Word(word) if word == "none" => None,
                _ => return None,
              });
            }
            "hidden" => {
              let side = DockSide::from_name(&self.word()?)?;
              self.expect(Token::Colon)?;
              let panel = self.text()?;
              group.hidden.push(DockHidden { panel, side });
            }
            "at" if float && position.is_none() => position = Some(self.pair()?),
            "size" if float && size.is_none() => size = Some(self.pair()?),
            _ => return None,
          }
        }
        _ => return None,
      }
    }
    group.shown = match shown {
      Some(shown) => shown,
      None => group.panels.first().cloned(),
    };
    Some(GroupItems { group, position, size })
  }
}
