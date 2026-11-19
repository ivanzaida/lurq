//! DevTools: a window beside the inspected one that shows what it is made of.
//!
//! It is built from the board `pencil/lurq-devtools.pen` with the design system's components, in a theme of
//! its own: the design system's, whatever the inspected app made of its theme.

mod changes;
mod components;
mod context;
mod kit;
mod profiler;
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
  context::ContextState,
  kit::FILL,
  profiler::ProfilerTab,
  signals::SignalsState,
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
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

pub type DevToolsDebugOverlayCallback = Arc<dyn Fn(Option<Vec<usize>>) + Send + Sync>;
pub type DevToolsBoolCallback = Arc<dyn Fn(bool) + Send + Sync>;
pub type DevToolsPathCallback = Arc<dyn Fn(Vec<usize>) + Send + Sync>;

/// The board's bar of tabs under the title bar.
const TOP_BAR_HEIGHT: f32 = 40.0;

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
  contexts: ContextState,
  profiler: ProfilerTab,
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
  Context,
}

impl Component for DevTools {
  type Props = DevToolsProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      theme: Theme::new(),
      active_tab: ctx.signal(DevToolsTab::Components),
      components: ComponentsState::new(ctx),
      signals: SignalsState::new(ctx),
      contexts: ContextState::new(ctx),
      profiler: ProfilerTab::new(ctx),
      last_picked_revision: AtomicU64::new(0),
    }
  }

  fn on_unmounted(&self) {
    // A recording left running would keep counting against the session limit.
    self.profiler.close();
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
        DevToolsTab::Profiler => profiler::view(ctx, &self.profiler, props.profiling.clone()),
        #[cfg(feature = "persistent_storage")]
        DevToolsTab::PersistentStorage => storage::view(ctx),
        DevToolsTab::Signals => signals::view(ctx, &props.snapshot, &self.signals),
        DevToolsTab::Context => context::view(ctx, &props.snapshot, &self.contexts),
      };
      let (tab, tree, signals, contexts) = (
        self.active_tab.clone(),
        self.components.search_key.clone(),
        self.signals.list.filter_key.clone(),
        self.contexts.list.filter_key.clone(),
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
            DevToolsTab::Context => &contexts,
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
    tabs.push(Tab::new(DevToolsTab::Context, "Context"));
    Tabs::new(self.active_tab.clone(), tabs)
      .variant(TabsVariant::Line)
      .label("DevTools")
      .id("devtools-tabs")
      .style(TabsPart::List, |s| s.height(TOP_BAR_HEIGHT).padding_x(16.0).gap(24.0))
      .style(TabsPart::Tab, |s| s.height(TOP_BAR_HEIGHT))
      .style(TabsPart::Label, |s| s.font_size(13.0))
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

  /// One thing a tree painted, as words: where, and what of.
  fn said(quad: &crate::layout::quad::Quad) -> String {
    use crate::layout::quad::QuadContent;
    let what = match &quad.content {
      QuadContent::Rect { color, gradient } => format!("rect {color:?} gradient {}", gradient.is_some()),
      QuadContent::Text { text, style, .. } => format!(
        "text {text:?} {} {} {:?} {:?}",
        style.font_family, style.font_size, style.weight, style.color
      ),
      QuadContent::RichText { spans, .. } => format!("rich text of {}", spans.len()),
      QuadContent::Backdrop { blur } => format!("backdrop {blur}"),
      _ => "other".to_owned(),
    };
    format!(
      "{:.1},{:.1} {:.1}x{:.1} @{:.2} {what} radius {:?} border {:?}",
      quad.x, quad.y, quad.width, quad.height, quad.opacity, quad.border_radius, quad.border
    )
  }

  /// What a tree paints once nothing in it is on its way to another look: a tab that was left fades for a
  /// moment, by the clock, so the tree is painted until two paintings a moment apart are the same.
  fn settled(tree: &mut crate::app::Tree, app: &mut App) -> Vec<String> {
    let mut last = Vec::new();
    for _ in 0..80 {
      tree.pass_headless(app);
      let now: Vec<String> = tree.painted_quads().iter().map(said).collect();
      if now == last {
        return now;
      }
      last = now;
      std::thread::sleep(std::time::Duration::from_millis(30));
    }
    panic!("the tree did not come to rest");
  }

  /// A window to inspect that is the same at every run: a component over three elements, none of them timed.
  fn fixed_snapshot() -> DevToolsSnapshot {
    let ids = crate::core::IdGenerator::new();
    let leaves = ["Row", "Text", "Button"]
      .map(|tag| DevToolsNode::for_test(ids.next(), tag, DevToolsNodeKind::Element, vec![]))
      .to_vec();
    DevToolsSnapshot {
      root: Some(DevToolsNode::for_test(
        ids.next(),
        "Root",
        DevToolsNodeKind::Component,
        leaves,
      )),
      frame: Default::default(),
    }
  }

  /// What DevTools paints, tab after tab, in an app whose theme `dress` made its own; and what a button of the
  /// app's own paints in that theme.
  fn painted_in(dress: impl FnOnce(&Theme)) -> (Vec<(&'static str, Vec<String>)>, Vec<String>) {
    use crate::app::events::MouseButton;
    let mut app = App::new();
    dress(app.theme());

    let mut own = crate::app::Tree::new();
    own.resize(400, 200);
    own.set_root(Column::new().child(crate::ui::Button::new("Save")));
    own.pass_headless(&mut app);
    let own = own.painted_quads().iter().map(said).collect();

    let mut tree = crate::app::Tree::new();
    tree.resize(1440, 936);
    tree.mount_root::<DevTools>(
      &mut app,
      DevToolsProps {
        snapshot: fixed_snapshot(),
        snapshot_revision: 1,
        #[cfg(feature = "persistent_storage")]
        persistent_storage_revision: 0,
        picked_path: None,
        picked_revision: 0,
        system: SystemSettings::default(),
        on_debug_overlay_path: None,
        on_overlay_enabled: None,
        on_pick_inspected: None,
        on_selected_path: None,
        on_save_node_screenshot: None,
        profiling: None,
      },
    );
    for _ in 0..2 {
      tree.pass_headless(&mut app);
    }
    let mut tabs = Vec::new();
    for tab in ["Components", "Profiler", "Signals", "Context"] {
      let label = tree
        .find_element(|element: crate::node::ElementRef<'_>| element.text_content() == Some(tab))
        .map(|element| element.bounds())
        .unwrap_or_else(|| panic!("no tab {tab}"));
      let (x, y) = (label.x + label.width / 2.0, label.y + label.height / 2.0);
      tree.mouse_down(x, y, MouseButton::Left);
      tree.mouse_up(x, y, MouseButton::Left);
      // The pointer is left where no tab is hovered.
      tree.mouse_move(700.0, 500.0);
      tabs.push((tab, settled(&mut tree, &mut app)));
    }
    (tabs, own)
  }

  #[test]
  fn devtools_is_drawn_in_its_own_theme_whatever_the_app_made_of_its_own() {
    use crate::{
      app::theme::ThemeDefinition,
      node::color::Color,
      ui::{Badge, BadgePart, Button, ButtonPart, Recipe, Table, TablePart, Tabs, TabsPart, Toggle, TogglePart},
    };
    let (plain, plain_own) = painted_in(|_| {});
    let (dressed, dressed_own) = painted_in(|theme| {
      // Another look whole: its palette, its type, its radii, its shadows, its scroll bars.
      theme.apply(ThemeDefinition::legacy());
      // Roles of the app's own.
      let red = Color::from_hex("#ff0000");
      for role in [
        PaletteColor::Background,
        PaletteColor::Foreground,
        PaletteColor::Card,
        PaletteColor::Muted,
        PaletteColor::MutedForeground,
        PaletteColor::Border,
        PaletteColor::Input,
        PaletteColor::Primary,
        PaletteColor::Highlight,
        PaletteColor::Destructive,
      ] {
        theme.set_palette_color(role, red);
      }
      // Recipes of the app's own for what DevTools is built with.
      theme.set_recipe::<Button>(Recipe::new().base(ButtonPart::Root, |s| s.background(red).radius(0.0).height(64.0)));
      theme.set_recipe::<Toggle>(Recipe::new().base(TogglePart::Root, |s| s.background(red).height(64.0)));
      theme.set_recipe::<Table<()>>(Recipe::new().base(TablePart::Row, |s| s.background(red).height(80.0)));
      theme.set_recipe::<Tabs>(Recipe::new().base(TabsPart::Tab, |s| s.background(red).height(90.0)));
      theme.set_recipe::<Badge>(Recipe::new().base(BadgePart::Root, |s| s.background(red)));
      // And a renderer of the app's own.
      theme.set_renderer::<Toggle>(std::sync::Arc::new(|_| Text::new("the app's toggle").into()));
    });
    // The dress is one that shows: the app's own button is not drawn as it was.
    assert_ne!(plain_own, dressed_own, "the app's own tree takes the app's theme");
    assert!(!plain.is_empty());
    for ((tab, plain), (_, dressed)) in plain.iter().zip(&dressed) {
      assert!(plain.len() > 20, "the {tab} tab paints {} things", plain.len());
      for (at, (plain, dressed)) in plain.iter().zip(dressed).enumerate() {
        assert_eq!(plain, dressed, "the {tab} tab, thing {at}");
      }
      assert_eq!(plain.len(), dressed.len(), "the {tab} tab");
    }
  }

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
