//! DevTools: a window beside the inspected one that shows what it is made of.
//!
//! It is built from the board `pencil/lurq-devtools.pen` with the design system's components, in a theme of
//! its own: the design system's, whatever the inspected app made of its theme.

mod components;
mod kit;
mod legacy;
mod signals;
mod snapshot;
#[cfg(feature = "persistent_storage")]
mod storage;

use std::sync::{
  Arc,
  atomic::{AtomicU64, Ordering},
};

pub use snapshot::{DevToolsNode, DevToolsSnapshot, FrameProfileSnapshot};

pub use self::snapshot::DevToolsSnapshot as Snapshot;
use self::{
  components::{ComponentsState, Inspected},
  kit::FILL,
  legacy::{profiler, profiler_analysis},
  signals::SignalsState,
};
use crate::{
  app::{
    component::Component,
    ctx::{Ctx, Interval},
    events::KeyboardEvent,
    profiler::ProfilingHandle,
    theme::{PaletteColor, Theme},
    window::SystemSettings,
  },
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{Tab, Tabs, TabsPart, TabsVariant, WindowChrome, icons},
};

/// The icon font of the tab not rebuilt yet (`legacy`): it goes with it.
static LUCIDE_TTF: &[u8] = include_bytes!("../../../assets/lucide.ttf");

pub type DevToolsDebugOverlayCallback = Arc<dyn Fn(Option<Vec<usize>>) + Send + Sync>;
pub type DevToolsBoolCallback = Arc<dyn Fn(bool) + Send + Sync>;
pub type DevToolsPathCallback = Arc<dyn Fn(Vec<usize>) + Send + Sync>;

/// The board's bar of tabs under the title bar.
const TOP_BAR_HEIGHT: f32 = 40.0;

pub fn load_fonts(app: &mut crate::app::App) {
  app.load_font(LUCIDE_TTF.to_vec());
  app.register_font("lucide", "lucide");
}

#[derive(Clone, crate::DevtoolsInspectable)]
pub struct DevToolsProps {
  #[devtools_ignore]
  pub snapshot: DevToolsSnapshot,
  #[devtools_ignore]
  pub snapshot_revision: u64,
  #[cfg(feature = "persistent_storage")]
  #[devtools_ignore]
  pub persistent_storage_revision: u64,
  pub picked_path: Option<Vec<usize>>,
  pub picked_revision: u64,
  /// What a person set for the whole system, as the inspected window knows it: DevTools is light or dark with
  /// it, in its own theme.
  #[devtools_ignore]
  pub system: SystemSettings,
  #[devtools_ignore]
  pub on_debug_overlay_path: Option<DevToolsDebugOverlayCallback>,
  #[devtools_ignore]
  pub on_overlay_enabled: Option<DevToolsBoolCallback>,
  #[devtools_ignore]
  pub on_pick_inspected: Option<DevToolsBoolCallback>,
  #[devtools_ignore]
  pub on_selected_path: Option<DevToolsPathCallback>,
  #[devtools_ignore]
  pub on_save_node_screenshot: Option<DevToolsPathCallback>,
  /// The inspected tree's profiling handle, which the Profiler tab records on.
  #[devtools_ignore]
  pub profiling: Option<ProfilingHandle>,
}

impl DevToolsProps {
  fn inspected(&self) -> Inspected {
    Inspected {
      on_debug_overlay_path: self.on_debug_overlay_path.clone(),
      on_overlay_enabled: self.on_overlay_enabled.clone(),
      on_pick_inspected: self.on_pick_inspected.clone(),
      on_selected_path: self.on_selected_path.clone(),
      on_save_node_screenshot: self.on_save_node_screenshot.clone(),
    }
  }
}

impl PartialEq for DevToolsProps {
  fn eq(&self, other: &Self) -> bool {
    self.snapshot_revision == other.snapshot_revision
      && self.picked_revision == other.picked_revision
      && self.system == other.system
      && {
        #[cfg(feature = "persistent_storage")]
        {
          self.persistent_storage_revision == other.persistent_storage_revision
        }
        #[cfg(not(feature = "persistent_storage"))]
        {
          true
        }
      }
  }
}

pub struct DevTools {
  /// The theme DevTools is drawn in: the design system's, light or dark with the system. It is not the
  /// inspected app's, so nothing the app does to its own theme reaches the tool that inspects it.
  theme: Theme,
  active_tab: Signal<DevToolsTab>,
  components: ComponentsState,
  signals: SignalsState,
  profiler: Signal<profiler::ProfilerState>,
  /// Rereads a running recording's status; runs only while one does.
  profiler_status_ticker: Interval,
  last_picked_revision: AtomicU64,
}

/// The tabs of the window, in the order the board has them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, crate::DevtoolsInspectable)]
pub(crate) enum DevToolsTab {
  Components,
  Profiler,
  #[cfg(feature = "persistent_storage")]
  PersistentStorage,
  Signals,
}

impl Component for DevTools {
  type Props = DevToolsProps;

  fn create(ctx: &mut Ctx) -> Self {
    let profiler = ctx.signal(profiler::ProfilerState::default());
    let profiler_status_ticker = ctx.create_interval(profiler::STATUS_INTERVAL, {
      let profiler = profiler.clone();
      move || profiler.update(profiler::ProfilerState::refresh_status)
    });
    Self {
      theme: Theme::new(),
      active_tab: ctx.signal(DevToolsTab::Components),
      components: ComponentsState::new(ctx),
      signals: SignalsState::new(ctx),
      profiler,
      profiler_status_ticker,
      last_picked_revision: AtomicU64::new(0),
    }
  }

  fn on_unmounted(&self) {
    // A recording left running would keep counting against the session limit.
    self.profiler_status_ticker.stop();
    self.profiler.update(profiler::ProfilerState::clear);
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<DevToolsProps>().clone();
    let inspected = props.inspected();
    self.theme.apply_system(&props.system);

    // A node picked in the inspected window is selected here, once for each pick.
    if props.picked_revision != self.last_picked_revision.swap(props.picked_revision, Ordering::Relaxed) {
      self.components.set_pick(false, &Inspected::default());
      if let Some(path) = props.picked_path.clone() {
        self.components.show_picked(&props.snapshot, path, &inspected);
      }
    }

    let theme = self.theme.clone();
    ctx.in_theme(&theme, |ctx| {
      let active = self.active_tab.get();
      let body = match active {
        DevToolsTab::Components => components::view(ctx, &props.snapshot, &self.components, &inspected),
        DevToolsTab::Profiler => self.profiler_tab(ctx, &props),
        #[cfg(feature = "persistent_storage")]
        DevToolsTab::PersistentStorage => storage::view(ctx),
        DevToolsTab::Signals => signals::view(ctx, &props.snapshot, &self.signals),
      };
      let (tab, tree, signals) = (
        self.active_tab.clone(),
        self.components.search_key.clone(),
        self.signals.filter_key.clone(),
      );
      let window = Column::new()
        .size(FILL, FILL)
        .background(PaletteColor::Background)
        .foreground(PaletteColor::Foreground)
        // Every key pressed in the window comes by here, whatever has the focus: the key of a tab's search.
        .on_key_down(move |event: KeyboardEvent| {
          if !kit::is_find_key(&event) {
            return;
          }
          let search = match tab.get_untracked() {
            DevToolsTab::Components => &tree,
            DevToolsTab::Signals => &signals,
            _ => return,
          };
          event.prevent_default();
          search.ask();
        })
        .child(self.top_bar())
        .child(Row::new().width(FILL).flex(1.0).child(body));
      WindowChrome::new()
        .title("lurq DevTools")
        .icon(icons::BUG)
        .content(window)
        .mount(ctx)
    })
  }
}

impl DevTools {
  /// The tabs of the window, in a row over a line.
  fn top_bar(&self) -> Element {
    let mut tabs = vec![
      Tab::new(DevToolsTab::Components, "Components"),
      Tab::new(DevToolsTab::Profiler, "Profiler"),
    ];
    #[cfg(feature = "persistent_storage")]
    tabs.push(Tab::new(DevToolsTab::PersistentStorage, "Storage"));
    tabs.push(Tab::new(DevToolsTab::Signals, "Signals"));
    Tabs::new(self.active_tab.clone(), tabs)
      .variant(TabsVariant::Line)
      .label("DevTools")
      .id("devtools-tabs")
      .style(TabsPart::List, |s| s.height(TOP_BAR_HEIGHT).padding_x(16.0).gap(24.0))
      .style(TabsPart::Tab, |s| s.height(TOP_BAR_HEIGHT))
      .style(TabsPart::Label, |s| s.font_size(13.0))
      .into()
  }

  fn profiler_tab(&self, ctx: &mut Ctx, props: &DevToolsProps) -> Element {
    let state = self.profiler.get();
    let analysis = profiler::analysis_props(&state, self.profiler.clone())
      .map(|props| ctx.mount::<profiler_analysis::ProfilerAnalysis>(props))
      .unwrap_or_default();
    Column::new()
      .child(profiler::profiler_actions(
        &state,
        self.profiler.clone(),
        props.profiling.clone(),
        self.profiler_status_ticker.clone(),
      ))
      .child(profiler::profiler_view(&state, self.profiler.clone(), analysis))
      .size(FILL, FILL)
      .flex(1.0)
      .into()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    app::{App, devtools::snapshot::DevToolsNodeKind},
    components::{Column, Text},
  };

  #[test]
  fn snapshot_collects_tree_nodes() {
    let mut app = App::new();
    let mut tree = crate::app::Tree::new();
    tree.mount_root::<SnapshotTestApp>(&mut app, ());
    assert!(tree.draw_debug_overlay_over_node(vec![0]));
    assert!(!tree.draw_debug_overlay_over_node(vec![0]));
    assert!(tree.clear_debug_overlay());
    tree.draw_perf_overlay();
    assert!(tree.perf_overlay_enabled());
    tree.tick_perf_overlay();

    let snapshot = DevToolsSnapshot::from_tree(&tree);

    assert!(snapshot.node_count() >= 2);
    assert_eq!(
      snapshot.root.as_ref().map(|node| kit::short_tag(&node.tag)),
      Some("SnapshotTestApp")
    );
    assert_eq!(
      snapshot.root.as_ref().map(|node| node.kind),
      Some(DevToolsNodeKind::Component)
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node.children.first())
        .map(|node| node.kind),
      Some(DevToolsNodeKind::Element)
    );
    let child_shape = snapshot
      .root
      .as_ref()
      .and_then(|node| node.children.first())
      .map(|node| node.shape.as_slice())
      .unwrap_or(&[]);
    assert_eq!(
      child_shape
        .iter()
        .find(|row| row.label == "layout")
        .and_then(|row| row.value.as_deref()),
      Some("Leaf")
    );
    let padding = child_shape.iter().find(|row| row.label == "padding").unwrap();
    assert_eq!(padding.value, None);
    assert_eq!(
      padding
        .children
        .iter()
        .map(|row| (row.label.as_str(), row.value.as_deref()))
        .collect::<Vec<_>>(),
      vec![
        ("left", Some("8px")),
        ("right", Some("8px")),
        ("top", Some("8px")),
        ("bottom", Some("8px")),
      ]
    );
    let hover_style = child_shape.iter().find(|row| row.label == "hover style").unwrap();
    assert_eq!(
      hover_style
        .children
        .iter()
        .find(|row| row.label == "fill")
        .and_then(|row| row.value.as_deref()),
      Some("#ff0000")
    );
    let hover_padding = hover_style.children.iter().find(|row| row.label == "padding").unwrap();
    assert_eq!(
      hover_padding
        .children
        .iter()
        .find(|row| row.label == "left")
        .and_then(|row| row.value.as_deref()),
      Some("4px")
    );
    let active_style = child_shape.iter().find(|row| row.label == "active style").unwrap();
    let active_frame = active_style.children.iter().find(|row| row.label == "frame").unwrap();
    assert_eq!(
      active_frame
        .children
        .iter()
        .find(|row| row.label == "width")
        .and_then(|row| row.value.as_deref()),
      Some("32px")
    );
    let root_effects = snapshot
      .root
      .as_ref()
      .map(|node| node.effects.as_slice())
      .unwrap_or(&[]);
    assert_eq!(root_effects.len(), 1);
    assert!(root_effects[0].id > 0);
    let keyed_child = snapshot
      .root
      .as_ref()
      .and_then(|node| {
        node
          .children
          .iter()
          .find(|child| kit::short_tag(&child.tag) == "KeyedChild")
      })
      .expect("keyed child should be captured");
    assert_eq!(keyed_child.key.as_deref(), Some("child-key"));
    let keyed_child_props = keyed_child
      .props
      .as_ref()
      .expect("keyed child props should be captured");
    assert_eq!(kit::short_tag(&keyed_child_props.type_name), "KeyedChildProps");
    assert_eq!(keyed_child_props.fields[0].name(), "title");
    assert_eq!(keyed_child_props.fields[0].type_name(), "&str");
    assert_eq!(keyed_child_props.fields[0].formatted_value(), Some("\"keyed child\""));
    assert_eq!(keyed_child_props.fields[1].name(), "count");
    assert_eq!(keyed_child_props.fields[1].type_name(), "u32");
    assert_eq!(keyed_child_props.fields[1].formatted_value(), Some("7"));
    assert_eq!(keyed_child_props.fields[2].name(), "details");
    assert_eq!(
      kit::short_tag(keyed_child_props.fields[2].type_name()),
      "KeyedChildDetails"
    );
    assert_eq!(keyed_child_props.fields[2].children()[0].name(), "enabled");
    assert_eq!(keyed_child_props.fields[2].children()[0].type_name(), "bool");
    assert_eq!(
      keyed_child_props.fields[2].children()[0].formatted_value(),
      Some("true")
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node.props.as_ref())
        .map(|props| props.fields.len()),
      Some(0)
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node.signals.first())
        .map(|signal| signal.type_name.as_ref()),
      Some("i32")
    );
    assert_eq!(
      snapshot
        .root
        .as_ref()
        .and_then(|node| node.signals.first())
        .and_then(|signal| signal.formatted_value()),
      Some("0".to_owned())
    );
    assert_eq!(snapshot.root.as_ref().map(|node| node.contexts.len()), Some(2));
  }

  #[test]
  fn a_component_says_how_many_times_it_rendered_and_an_element_does_not() {
    struct Counted;

    impl Component for Counted {
      type Props = ();

      fn create(_ctx: &mut Ctx) -> Self {
        Self
      }

      fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
        Column::new().child(Text::new("counted"))
      }
    }

    let mut app = App::new();
    let mut tree = crate::app::Tree::new();
    tree.mount_root::<Counted>(&mut app, ());

    let snapshot = DevToolsSnapshot::from_tree(&tree);
    let root = snapshot.root.as_ref().expect("the root is captured");
    assert_eq!(root.render.map(|render| render.count), Some(1));
    assert_eq!(
      root.children.first().and_then(|child| child.render),
      None,
      "an element is drawn by the render of the component it is in"
    );

    tree.rebuild();

    let snapshot = DevToolsSnapshot::from_tree(&tree);
    let render = snapshot.root.as_ref().and_then(|root| root.render);
    assert_eq!(render.map(|render| render.count), Some(2), "each render is counted");
  }

  #[test]
  fn selected_snapshot_only_collects_inspector_details_for_selected_path() {
    let mut app = App::new();
    let mut tree = crate::app::Tree::new();
    tree.mount_root::<SnapshotTestApp>(&mut app, ());

    let snapshot = DevToolsSnapshot::from_tree_for_selection(&tree, &[0]);
    let root = snapshot.root.as_ref().expect("root should be captured");
    let selected = root.children.first().expect("selected child should be captured");

    assert!(root.shape.is_empty());
    assert!(root.props.is_none());
    assert!(!root.signals.is_empty());
    assert!(!selected.shape.is_empty());
    assert!(selected.layout_box.is_none());
  }

  struct SnapshotTestApp {
    count: crate::core::Signal<i32>,
  }

  impl Component for SnapshotTestApp {
    type Props = ();

    fn create(ctx: &mut Ctx) -> Self {
      let count = ctx.signal(0);
      let effect_count = count.clone();
      ctx.on_effect(move || {
        let _ = effect_count.get();
      });
      Self { count }
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      ctx.provide(42_i32);
      let _ = ctx.use_context::<i32>();
      Column::new()
        .child(
          Text::new(&format!("child {}", self.count.get()))
            .padding(8.0)
            .hovered(|style| style.background("#ff0000").padding_left(4.0))
            .active(|style| style.width(32.0)),
        )
        .child(ctx.mount_keyed::<KeyedChild>(
          "child-key",
          KeyedChildProps {
            title: "keyed child",
            count: 7,
            details: KeyedChildDetails { enabled: true },
          },
        ))
    }
  }

  struct KeyedChild;

  #[derive(Clone, PartialEq, crate::DevtoolsInspectable)]
  struct KeyedChildProps {
    title: &'static str,
    count: u32,
    details: KeyedChildDetails,
  }

  #[derive(Clone, PartialEq, crate::DevtoolsInspectable)]
  struct KeyedChildDetails {
    enabled: bool,
  }

  impl Component for KeyedChild {
    type Props = KeyedChildProps;

    fn create(_ctx: &mut Ctx) -> Self {
      Self
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
      Text::new(ctx.props::<Self::Props>().title)
    }
  }
}
