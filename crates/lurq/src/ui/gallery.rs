//! A gallery: pictures in rows of as many columns, chosen among and looked at closer.

use std::{marker::PhantomData, sync::Arc};

use super::{
  Lightbox, Picture, PictureSize, names,
  part_style::Length,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::Theme,
  },
  components::{Column, Row},
  core::Signal,
  node::{Element, FocusGroup, dimension::Dimension},
};

/// The parts of a gallery a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GalleryPart {
  /// The gallery: its rows, one under another.
  Root,
  /// A row of pictures.
  Row,
  /// The room of one picture in a row.
  Cell,
}

names!(GalleryPart {
  Root => "root",
  Row => "row",
  Cell => "cell",
});

/// The states of a gallery a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GalleryState {
  /// Its pictures can be chosen.
  Selectable,
  /// It takes nothing.
  Disabled,
}

names!(GalleryState {
  Selectable => "selectable",
  Disabled => "disabled",
});

/// What builds a gallery's nodes: a theme's in place of the default one.
pub type GalleryRenderer = Arc<dyn Fn(GalleryView) -> Element + Send + Sync>;

type Told<T> = Arc<dyn Fn(T) + Send + Sync>;

/// What a key of a gallery's picture is: what the app tells its pictures apart by.
pub trait GalleryKey: Clone + PartialEq + Send + Sync + 'static {}

impl<T: Clone + PartialEq + Send + Sync + 'static> GalleryKey for T {}

/// A gallery: pictures in rows.
///
/// ```
/// use lurq::{core::Signal, ui::{Gallery, Picture}};
///
/// let chosen = Signal::new(Vec::<usize>::new());
/// let photos = Gallery::new([
///   Picture::from_file("lake.jpg").label("The lake at dusk"),
///   Picture::from_file("ridge.jpg").label("The ridge"),
/// ])
/// .columns(4)
/// .selectable(true)
/// .selected(chosen);
/// ```
///
/// Its pictures stand in rows of [`columns`](Self::columns), each as wide as its share of the gallery and as
/// high as [`aspect_ratio`](Self::aspect_ratio) makes it, [`gap`](Self::gap) apart. A gallery that shows fewer
/// than it has ([`max_shown`](Self::max_shown)) ends in a picture under a veil that says how many more there
/// are.
///
/// A press of a picture opens it in a lightbox that holds them all, unless the gallery is told not to
/// ([`lightbox`](Self::lightbox)); then it tells the app ([`on_select`](Self::on_select)). In a gallery whose
/// pictures can be chosen ([`selectable`](Self::selectable)) a press chooses, and Enter opens. The gallery is
/// one stop of Tab, and the arrows move among its pictures.
///
/// Pictures are told apart by their place, or by a key of the app's ([`keyed`](Self::keyed)): what is chosen
/// is then a list of keys, which stays right when the pictures change.
///
/// A gallery is a component: it keeps which picture is open and which are chosen unless the app gives it
/// signals for them.
pub struct Gallery<K = usize> {
  items: Vec<(K, Picture)>,
  columns: usize,
  gap: f32,
  aspect_ratio: f32,
  size: Name,
  width: Option<Length>,
  max_shown: Option<usize>,
  more_text: Option<Arc<dyn Fn(usize) -> String + Send + Sync>>,
  selectable: bool,
  selected: Option<Signal<Vec<K>>>,
  controlled: Option<Vec<K>>,
  on_selection_change: Vec<Arc<dyn Fn(&[K]) + Send + Sync>>,
  lightbox: bool,
  open: Option<Signal<Option<usize>>>,
  on_open_change: Vec<Told<Option<usize>>>,
  on_select: Vec<Told<usize>>,
  label: Option<Arc<str>>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<K: Clone> Clone for Gallery<K> {
  fn clone(&self) -> Self {
    Self {
      items: self.items.clone(),
      columns: self.columns,
      gap: self.gap,
      aspect_ratio: self.aspect_ratio,
      size: self.size,
      width: self.width,
      max_shown: self.max_shown,
      more_text: self.more_text.clone(),
      selectable: self.selectable,
      selected: self.selected.clone(),
      controlled: self.controlled.clone(),
      on_selection_change: self.on_selection_change.clone(),
      lightbox: self.lightbox,
      open: self.open.clone(),
      on_open_change: self.on_open_change.clone(),
      on_select: self.on_select.clone(),
      label: self.label.clone(),
      disabled: self.disabled,
      states: self.states.clone(),
      rules: self.rules.clone(),
      id: self.id.clone(),
      key: self.key.clone(),
    }
  }
}

impl Gallery<usize> {
  /// A gallery of `pictures`, each told from the others by its place.
  pub fn new(pictures: impl IntoIterator<Item = Picture>) -> Self {
    Self::keyed(pictures.into_iter().enumerate())
  }

  /// One more picture, after the others.
  pub fn picture(mut self, picture: Picture) -> Self {
    self.items.push((self.items.len(), picture));
    self
  }

  /// More pictures, after the others.
  pub fn pictures(mut self, pictures: impl IntoIterator<Item = Picture>) -> Self {
    for picture in pictures {
      self = self.picture(picture);
    }
    self
  }
}

impl<K: GalleryKey> Gallery<K> {
  /// A gallery of pictures, each with the key the app tells it by: what is chosen is a list of those keys.
  pub fn keyed(items: impl IntoIterator<Item = (K, Picture)>) -> Self {
    Self {
      items: items.into_iter().collect(),
      columns: 4,
      gap: 8.0,
      aspect_ratio: 92.0 / 68.0,
      size: PictureSize::Sm.into(),
      width: None,
      max_shown: None,
      more_text: None,
      selectable: false,
      selected: None,
      controlled: None,
      on_selection_change: Vec::new(),
      lightbox: true,
      open: None,
      on_open_change: Vec::new(),
      on_select: Vec::new(),
      label: None,
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// One more picture with its key, after the others.
  pub fn item(mut self, key: K, picture: Picture) -> Self {
    self.items.push((key, picture));
    self
  }

  /// How many pictures stand in a row: 4 unless told.
  pub fn columns(mut self, columns: usize) -> Self {
    self.columns = columns.max(1);
    self
  }

  /// The room between pictures, across and down: 8 unless told.
  pub fn gap(mut self, gap: f32) -> Self {
    self.gap = gap.max(0.0);
    self
  }

  /// The width of a picture's room to its height: 92 to 68 unless told.
  pub fn aspect_ratio(mut self, ratio: f32) -> Self {
    if ratio.is_finite() && ratio > 0.0 {
      self.aspect_ratio = ratio;
    }
    self
  }

  /// The form of its pictures ([`PictureSize`]): the small one unless told.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// How wide the gallery is: all the room there is unless told.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// The most pictures it shows. With more than that, the last one shown lies under a veil that says how
  /// many more there are, itself among them.
  pub fn max_shown(mut self, max: usize) -> Self {
    self.max_shown = Some(max.max(1));
    self
  }

  /// What the veil says, from how many more pictures there are: "+5" unless told.
  pub fn more_text(mut self, text: impl Fn(usize) -> String + Send + Sync + 'static) -> Self {
    self.more_text = Some(Arc::new(text));
    self
  }

  /// Whether its pictures can be chosen: a press chooses one, and takes it out again.
  pub fn selectable(mut self, selectable: bool) -> Self {
    self.selectable = selectable;
    self
  }

  /// The keys of the pictures that are chosen, to read and to set. Without it the gallery keeps them.
  pub fn selected(mut self, selected: Signal<Vec<K>>) -> Self {
    self.selected = Some(selected);
    self
  }

  /// The keys of the pictures that are chosen, as the app says them each time: the gallery asks for another
  /// list with `on_change` and shows what it is told.
  pub fn selected_controlled(mut self, keys: Vec<K>, on_change: impl Fn(&[K]) + Send + Sync + 'static) -> Self {
    self.controlled = Some(keys);
    self.on_selection_change.push(Arc::new(on_change));
    self
  }

  /// Called when a picture is chosen or taken out, with the keys of all that are chosen now.
  pub fn on_selection_change(mut self, handler: impl Fn(&[K]) + Send + Sync + 'static) -> Self {
    self.on_selection_change.push(Arc::new(handler));
    self
  }

  /// Whether a picture opens in a lightbox that holds them all: it does unless told.
  pub fn lightbox(mut self, lightbox: bool) -> Self {
    self.lightbox = lightbox;
    self
  }

  /// Which picture is open in the lightbox, counted from 0, and `None` while it is closed: to read and to
  /// set. Without it the gallery keeps it.
  pub fn open(mut self, open: Signal<Option<usize>>) -> Self {
    self.open = Some(open);
    self
  }

  /// Called when a picture is opened, another is turned to, or the lightbox is closed.
  pub fn on_open_change(mut self, handler: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
    self.on_open_change.push(Arc::new(handler));
    self
  }

  /// Called on a press of a picture, with its place, in a gallery that neither chooses nor opens.
  pub fn on_select(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// What it is called to tooling that reads the tree.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Half the opacity on its pictures; it takes nothing.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Turns a state a recipe's rules name on or off.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The style of one part, over the theme's.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Rules over the theme's.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The gallery mounted in `ctx`: what converting it into an element does once the render that returns it
  /// has returned.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = GalleryProps(self);
    match key {
      Some(key) => ctx.mount_keyed::<GalleryHost<K>>(&key, props),
      None => ctx.mount::<GalleryHost<K>>(props),
    }
  }
}

/// A gallery in a tree is its component, declared: the component that renders the tree mounts it.
impl<K: GalleryKey> From<Gallery<K>> for Element {
  fn from(gallery: Gallery<K>) -> Self {
    let key = gallery.key.clone();
    let props = GalleryProps(gallery);
    match key {
      Some(key) => Element::component_keyed::<GalleryHost<K>>(key, props),
      None => Element::component::<GalleryHost<K>>(props),
    }
  }
}

/// A gallery to mount.
struct GalleryProps<K>(Gallery<K>);

impl<K: Clone> Clone for GalleryProps<K> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

/// Its pictures and its handlers are made anew by every render of what holds it.
impl<K> PartialEq for GalleryProps<K> {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl<K> crate::app::component::DevtoolsInspectable for GalleryProps<K> {}

/// The component of a gallery: what it keeps between renders.
struct GalleryHost<K> {
  /// The keys of the pictures chosen, where the app names no signal for them.
  selected: Signal<Vec<K>>,
  /// Which picture is open, where the app names no signal for it.
  open: Signal<Option<usize>>,
  /// Whether the lightbox is open: what the lightbox itself is given.
  shown: Signal<bool>,
  _keys: PhantomData<fn() -> K>,
}

impl<K: GalleryKey> Component for GalleryHost<K> {
  type Props = GalleryProps<K>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      selected: ctx.signal(Vec::new()),
      open: ctx.signal(None),
      shown: ctx.signal(false),
      _keys: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let gallery = ctx.props::<GalleryProps<K>>().0.clone();
    let theme = current_theme();
    let total = gallery.items.len();
    let open_signal = gallery.open.clone().unwrap_or_else(|| self.open.clone());
    let opened = open_signal.get().filter(|at| *at < total);
    let selected_signal = gallery.selected.clone().unwrap_or_else(|| self.selected.clone());
    let chosen = gallery.controlled.clone().unwrap_or_else(|| selected_signal.get());
    // The lightbox is open while a picture is.
    if self.shown.get_untracked() != opened.is_some() {
      self.shown.set(opened.is_some());
    }

    // Opens the picture at a place, turns to another, or closes the lightbox.
    let show: Told<Option<usize>> = {
      let (open, shown, told) = (open_signal.clone(), self.shown.clone(), gallery.on_open_change.clone());
      Arc::new(move |at: Option<usize>| {
        if open.get_untracked() != at {
          open.set(at);
        }
        if shown.get_untracked() != at.is_some() {
          shown.set(at.is_some());
        }
        told.iter().for_each(|tell| tell(at));
      })
    };
    // Chooses the picture with a key, or takes it out.
    let choose: Arc<dyn Fn(&K, bool) + Send + Sync> = {
      let (signal, controlled, told) = (
        selected_signal.clone(),
        gallery.controlled.is_some(),
        gallery.on_selection_change.clone(),
      );
      let chosen = chosen.clone();
      Arc::new(move |key: &K, on: bool| {
        let mut next = chosen.clone();
        next.retain(|held| held != key);
        if on {
          next.push(key.clone());
        }
        if !controlled {
          signal.set(next.clone());
        }
        told.iter().for_each(|tell| tell(&next));
      })
    };

    let shown_count = gallery.max_shown.map_or(total, |max| max.min(total));
    // The last one shown stands for itself and the rest.
    let more = (total > shown_count).then(|| total - shown_count + 1);
    let mut cells = Vec::with_capacity(shown_count);
    for (at, (key, picture)) in gallery.items.iter().enumerate().take(shown_count) {
      let stands_for_more = more.filter(|_| at + 1 == shown_count);
      let mut cell = picture
        .clone()
        .in_grid()
        .size(gallery.size)
        .disabled(gallery.disabled)
        .key(format!("picture-{at}"));
      if let Some(id) = &gallery.id {
        cell = cell.id(format!("{id}-{at}"));
      }
      let opens = gallery.lightbox;
      if let Some(more) = stands_for_more {
        let text = match &gallery.more_text {
          Some(text) => text(more),
          None => format!("+{more}"),
        };
        cell = cell.count(&text);
      }
      if gallery.selectable && stands_for_more.is_none() {
        let (choose, key) = (choose.clone(), key.clone());
        cell = cell
          .selectable(true)
          .selected(chosen.contains(&key))
          .on_change(move |on| choose(&key, on));
      } else if opens {
        let show = show.clone();
        cell = cell.on_click(move |_: MouseEvent| show(Some(at)));
      } else if !gallery.on_select.is_empty() {
        let told = gallery.on_select.clone();
        cell = cell.on_click(move |_: MouseEvent| told.iter().for_each(|tell| tell(at)));
      }
      // In a gallery that chooses, Enter is what opens.
      let enter = (opens && gallery.selectable && !gallery.disabled).then(|| show.clone());
      cells.push((Element::from(cell), enter.map(|show| (show, at))));
    }

    let viewer = (gallery.lightbox && total > 0).then(|| {
      let (turn, close) = (show.clone(), show.clone());
      let mut viewer = Lightbox::new(
        self.shown.clone(),
        gallery.items.iter().map(|(_, picture)| picture.clone()),
      )
      .index_controlled(opened.unwrap_or(0), move |at| turn(Some(at)))
      .on_dismiss(move || close(None))
      .key("viewer");
      if let Some(label) = &gallery.label {
        viewer = viewer.label(label);
      }
      if let Some(id) = &gallery.id {
        viewer = viewer.id(format!("{id}-viewer"));
      }
      Element::from(viewer)
    });

    let case = Case {
      variant: None,
      size: Some(gallery.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: gallery.states.clone(),
    }
    .state(GalleryState::Selectable, gallery.selectable)
    .state(GalleryState::Disabled, gallery.disabled);
    let view = GalleryView {
      recipe: theme.resolve::<Gallery<K>>(&case, Some(&gallery.rules)),
      cells,
      viewer,
      columns: gallery.columns,
      gap: gallery.gap,
      aspect_ratio: gallery.aspect_ratio,
      width: gallery.width,
      selectable: gallery.selectable,
      label: gallery.label.clone(),
      id: gallery.id.clone(),
      key: gallery.key.clone(),
    };
    match theme.renderer::<Gallery<K>>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// A gallery as a renderer sees it: its pictures, made, and how they are laid.
pub struct GalleryView {
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  /// The pictures shown, in order, each with what Enter does on it where that is not what a press does.
  cells: Vec<(Element, Option<(Told<Option<usize>>, usize)>)>,
  /// The lightbox that holds them all, which takes no room.
  viewer: Option<Element>,
  pub columns: usize,
  pub gap: f32,
  pub aspect_ratio: f32,
  pub width: Option<Length>,
  pub selectable: bool,
  label: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl GalleryView {
  /// The style of a part.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// How the default renderer builds a gallery: rows of as many rooms, each as wide as its share and as high
  /// as the ratio makes it, with a picture in it; and the lightbox, which takes no room.
  pub fn render(self) -> Element {
    let full = Dimension::Pct(100.0);
    let width = self.width.and_then(|width| width.resolve(self.theme())).unwrap_or(full);
    let (row_part, cell_part) = (self.part(GalleryPart::Row), self.part(GalleryPart::Cell));
    let mut root = Column::new()
      .part(&self.part(GalleryPart::Root))
      .width(width)
      // The width said is the most the gallery takes: in a narrower room its pictures are smaller.
      .max_width(full)
      .spacing(self.gap)
      // One stop of Tab, and the arrows move among the pictures.
      .focus_group(FocusGroup::both().one_stop(true))
      .describe("a11y_role", if self.selectable { "listbox" } else { "list" });
    if let Some(label) = &self.label {
      root = root.describe("a11y_name", label.clone());
    }
    if self.selectable {
      root = root.describe("a11y_multiselectable", "true");
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    let columns = self.columns.max(1);
    let mut cells = self.cells.into_iter().peekable();
    while cells.peek().is_some() {
      let mut row = Row::new().part(&row_part).width(full).spacing(self.gap);
      for _ in 0..columns {
        // A row the pictures do not fill keeps its rooms, so that the ones it has are as wide as the others.
        let mut room = Row::new()
          .part(&cell_part)
          .flex(1.0)
          .min_width(0.0)
          .aspect_ratio(self.aspect_ratio);
        if let Some((picture, enter)) = cells.next() {
          if let Some((show, at)) = enter {
            room = room.on_key_down(move |event: KeyboardEvent| {
              // A key is told to every element; this one is the picture's that has the focus.
              if event.key == "Enter" && event.focus_within {
                event.prevent_default();
                event.stop_propagation();
                show(Some(at));
              }
            });
          }
          room = room.child(picture);
        }
        row = row.child(room);
      }
      root = root.child(row);
    }
    if let Some(viewer) = self.viewer {
      root = root.child(viewer);
    }
    root.into()
  }
}

impl<K: Send + Sync + 'static> Themed for Gallery<K> {
  const NAME: &'static str = "gallery";
  type Renderer = GalleryRenderer;

  /// The design's gallery (`pencil/lurq-design-system.pen`, Gallery): rows of four pictures of 92 by 68, 8
  /// apart across and down, each a small picture with the corners of one. What a picture looks like, chosen
  /// or under the veil of a count, is the picture's own recipe.
  fn recipe() -> Recipe {
    Recipe::new()
  }
}
