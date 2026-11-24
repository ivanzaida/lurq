//! Styled spans of a text input: kept in the app's spans signal, moved with
//! every edit, recorded in the undo history with the text, and reported to
//! the app after every change the input makes.
use std::ops::Range;

use super::*;
use crate::components::{
  InlineObject, LinkClick, ParagraphKind, ParagraphStyles, RichText, SelectionStyle, SpanStyle, StyledChange,
  StyledEdit, StyledSpan, StyledText, TextMark, styled_text as spans_ops,
};

type ChangeHandler = Arc<dyn Fn(&StyledChange) + Send + Sync>;
type CopyFormat = Arc<dyn Fn(&StyledText) -> String + Send + Sync>;
type LinkHandler = Arc<dyn Fn(&LinkClick) + Send + Sync>;

/// The spans in canonical form for one version of the value and of the
/// spans signal, so reading them copies nothing until either changes.
#[derive(Clone)]
struct SpansCache {
  value_version: u64,
  spans_version: u64,
  spans: Arc<Vec<StyledSpan>>,
}

/// What makes an input styled. Its hooks come from the builder on every
/// render; the rest is runtime state kept across renders.
#[derive(Clone, Default)]
pub(super) struct StyledState {
  spans: Option<Signal<Vec<StyledSpan>>>,
  /// The spans signal's version the input last wrote or read, to notice an
  /// app's write and lay the input out again.
  synced_version: u64,
  cache: Option<SpansCache>,
  /// What each paragraph is, where the input has kinds of paragraphs, and how each kind is drawn.
  paragraphs: Option<Signal<Vec<ParagraphKind>>>,
  paragraph_styles: Option<Arc<ParagraphStyles>>,
  /// The paragraphs signal's version the input last wrote or read.
  paragraphs_synced: u64,
  /// The things in the text that are not text, where the input has them: see [`InlineObject`].
  objects: Option<Signal<Vec<InlineObject>>>,
  /// The objects signal's version the input last wrote or read.
  objects_synced: u64,
  /// The objects the input's child nodes were built for, a child for each, in order. An edit moves the objects
  /// before the app builds the children again: until then a child is told from the others by what it shows.
  object_nodes: Arc<[InlineObject]>,
  /// The style the next typed text takes, set by the app, while the caret
  /// stays at the offset it was set at with nothing selected.
  pending: Option<(usize, SpanStyle)>,
  /// Whether an object selected alone has a handle that a drag makes it wider or narrower by.
  resizable_objects: bool,
  /// The drag of that handle in progress.
  resize: Option<ObjectResize>,
  /// Whether the pointer is on the handle.
  handle_hovered: bool,
  on_change: Vec<ChangeHandler>,
  copy_format: Option<CopyFormat>,
  /// A copy keeps the Markdown of the selection beside its text, and a paste that has Markdown is inserted with
  /// its marks and kinds.
  rich_clipboard: bool,
  on_link_click: Vec<LinkHandler>,
  /// The look of the marks of this input, in place of the theme's.
  text_marks: Option<Arc<crate::app::theme::ThemeTextMarks>>,
  /// Whether a press on the box of an item of a checklist checks it or unchecks it.
  checks_on_press: bool,
}

impl StyledState {
  pub(super) fn is_bound(&self) -> bool {
    self.spans.is_some()
  }

  pub(super) fn copy_runtime_state_from(&mut self, old: &Self) {
    if let (Some(new), Some(previous)) = (&self.spans, &old.spans)
      && new.id() == previous.id()
    {
      self.synced_version = old.synced_version;
      self.cache = old.cache.clone();
    }
    if let (Some(new), Some(previous)) = (&self.paragraphs, &old.paragraphs)
      && new.id() == previous.id()
    {
      self.paragraphs_synced = old.paragraphs_synced;
    }
    if let (Some(new), Some(previous)) = (&self.objects, &old.objects)
      && new.id() == previous.id()
    {
      self.objects_synced = old.objects_synced;
    }
    self.pending = old.pending.clone();
    // A drag of an object's handle goes on over the renders its own changes ask for.
    self.resize = old.resize.clone();
    self.handle_hovered = old.handle_hovered;
  }
}

/// How near the corner of an object the pointer takes its handle, and the least an object is made.
const OBJECT_HANDLE_REACH: f32 = 8.0;
const OBJECT_LEAST_WIDTH: f32 = 16.0;

/// A drag of the handle of an object: the byte of the object, where the drag began in the text and how wide the
/// object was then, the most it can be, and whether the drag changed it yet.
#[derive(Clone)]
struct ObjectResize {
  at: usize,
  from_x: f32,
  from_width: f32,
  most: f32,
  changed: bool,
}

/// The spans and the typing style captured before an edit changes the text.
pub(super) struct SpansBeforeEdit {
  spans: Arc<Vec<StyledSpan>>,
  style: SpanStyle,
  /// The kinds of the paragraphs and where each paragraph was, where the input has kinds.
  paragraphs: Option<(SnapshotParagraphs, Vec<Range<usize>>)>,
  /// The objects of the text, where the input has them.
  objects: Option<SnapshotObjects>,
}

/// The spans a snapshot of the undo history keeps, shared.
pub(super) type SnapshotSpans = Arc<Vec<StyledSpan>>;

/// The kinds of the paragraphs a snapshot of the undo history keeps, shared.
pub(super) type SnapshotParagraphs = Arc<Vec<ParagraphKind>>;

/// The objects a snapshot of the undo history keeps, shared.
pub(super) type SnapshotObjects = Arc<Vec<InlineObject>>;

/// Text inserted with its own spans, with the kinds of its paragraphs where it brings them, and with the objects
/// that are in it, their bytes counted from its start.
pub(super) struct InsertedText<'a> {
  pub(super) styled: &'a StyledText,
  pub(super) kinds: Option<&'a [ParagraphKind]>,
  pub(super) objects: &'a [InlineObject],
}

impl TextInputState {
  pub(crate) fn set_spans_signal(&self, spans: Signal<Vec<StyledSpan>>) {
    let mut inner = self.inner.lock();
    if inner
      .styled
      .spans
      .as_ref()
      .is_some_and(|bound| bound.id() == spans.id())
    {
      return;
    }
    inner.styled.synced_version = spans.version();
    inner.styled.cache = None;
    inner.styled.spans = Some(spans);
    drop(inner);
    self.mark_layout_dirty_keeping_scroll();
  }

  /// Binds the kinds of the input's paragraphs and how each kind is drawn.
  pub(crate) fn set_paragraphs(&self, kinds: Signal<Vec<ParagraphKind>>, styles: ParagraphStyles) {
    let mut inner = self.inner.lock();
    let same_signal = inner
      .styled
      .paragraphs
      .as_ref()
      .is_some_and(|bound| bound.id() == kinds.id());
    let same_styles = inner
      .styled
      .paragraph_styles
      .as_ref()
      .is_some_and(|held| **held == styles);
    if same_signal && same_styles {
      return;
    }
    if !same_signal {
      inner.styled.paragraphs_synced = kinds.version();
      inner.styled.paragraphs = Some(kinds);
    }
    if !same_styles {
      inner.styled.paragraph_styles = Some(Arc::new(styles));
    }
    drop(inner);
    self.mark_layout_dirty_keeping_scroll();
  }

  /// Binds the objects of the input's text. `built` are those its child nodes were built for, a child for each,
  /// in order.
  pub(crate) fn set_objects(&self, objects: Signal<Vec<InlineObject>>, built: Vec<InlineObject>) {
    let mut inner = self.inner.lock();
    let same_signal = inner
      .styled
      .objects
      .as_ref()
      .is_some_and(|bound| bound.id() == objects.id());
    let same_nodes = inner.styled.object_nodes.as_ref() == built.as_slice();
    if !same_signal {
      inner.styled.objects_synced = objects.version();
      inner.styled.objects = Some(objects);
    }
    if !same_nodes {
      inner.styled.object_nodes = built.into();
    }
    drop(inner);
    if !(same_signal && same_nodes) {
      self.mark_layout_dirty_keeping_scroll();
    }
  }

  /// The objects of the current text, in its order, shared; `None` for an input without objects.
  pub(crate) fn current_objects(&self) -> Option<SnapshotObjects> {
    let signal = {
      let inner = self.inner.lock();
      if inner.mask.is_some() || inner.styled.spans.is_none() {
        return None;
      }
      inner.styled.objects.clone()?
    };
    let value = self.shared_value();
    Some(Arc::new(signal.with_untracked(|objects| {
      spans_ops::normalize_objects(&value, objects.clone())
    })))
  }

  /// The objects of the text the input shows: the current ones, moved over a composition in progress.
  pub(crate) fn shown_objects(&self) -> Option<SnapshotObjects> {
    let objects = self.current_objects()?;
    let Some(composed) = self.composed_text() else {
      return Some(objects);
    };
    let value = self.shared_value();
    let (start, end) = self.selection_or_caret(&value);
    let inserted = composed.len() + (end - start) - value.len();
    Some(Arc::new(spans_ops::splice_objects(&objects, start..end, inserted)))
  }

  /// For each object the input shows, in order, which of the input's child nodes was built for it: the next
  /// one, in the order they were built, that shows the same thing. `None` for an object no child was built for
  /// yet; a child no object is left for is in none.
  pub(crate) fn object_children(&self) -> Vec<Option<usize>> {
    let Some(objects) = self.shown_objects() else {
      return Vec::new();
    };
    let built = self.inner.lock().styled.object_nodes.clone();
    let mut next = 0;
    objects
      .iter()
      .map(|object| {
        let found = next + built[next..].iter().position(|node| node.same_thing(object))?;
        next = found + 1;
        Some(found)
      })
      .collect()
  }

  /// The bytes of each object of the current text: what is one thing to the caret in a styled input.
  pub(super) fn object_ranges(&self) -> Vec<Range<usize>> {
    self
      .current_objects()
      .map(|objects| objects.iter().map(InlineObject::range).collect())
      .unwrap_or_default()
  }

  /// The object whose box is at `(x, y)`, in the coordinates the caret is placed by: its bytes.
  pub(super) fn object_at_point(&self, x: f32, y: f32) -> Option<Range<usize>> {
    let objects = self.current_objects().filter(|objects| !objects.is_empty())?;
    let layout = self.token_layout()?;
    let (x, y) = {
      let inner = self.inner.lock();
      (x + inner.scroll_x, y + inner.scroll_y())
    };
    let room = layout
      .objects
      .iter()
      .find(|room| x >= room.x && x < room.x + room.width && y >= room.y && y < room.y + room.height)?;
    objects
      .iter()
      .find(|object| object.at == room.byte)
      .map(InlineObject::range)
  }

  pub(crate) fn set_resizable_objects(&self, resizable: bool) {
    self.inner.lock().styled.resizable_objects = resizable;
  }

  /// The object that has a handle: the one the selection is, where it is one object and nothing else, in an
  /// input whose objects are resized. Its box, in the coordinates the glyphs have, and its record.
  pub(crate) fn resized_object(
    &self,
  ) -> Option<(crate::app::glyph_engine::input_tokens::InputObjectBox, InlineObject)> {
    if !self.inner.lock().styled.resizable_objects {
      return None;
    }
    let (start, end) = self.selection_bounds()?;
    let objects = self.current_objects()?;
    let object = objects
      .iter()
      .find(|object| object.at == start && object.range().end == end)?
      .clone();
    let layout = self.token_layout()?;
    let room = *layout.objects.iter().find(|room| room.byte == object.at)?;
    Some((room, object))
  }

  /// The object whose handle is at `(x, y)`, in the coordinates the caret is placed by.
  fn object_handle_at(
    &self,
    x: f32,
    y: f32,
  ) -> Option<(crate::app::glyph_engine::input_tokens::InputObjectBox, InlineObject)> {
    let (room, object) = self.resized_object()?;
    let (x, y) = {
      let inner = self.inner.lock();
      (x + inner.scroll_x, y + inner.scroll_y())
    };
    let near = |point: f32, corner: f32| (point - corner).abs() <= OBJECT_HANDLE_REACH;
    (near(x, room.x + room.width) && near(y, room.y + room.height)).then_some((room, object))
  }

  /// Starts a drag of the handle at `(x, y)`: `false` where there is none there.
  pub(super) fn begin_object_resize(&self, x: f32, y: f32) -> bool {
    self.inner.lock().styled.resize = None;
    let Some((room, object)) = self.object_handle_at(x, y) else {
      return false;
    };
    let most = self
      .token_layout()
      .map(|layout| layout.config.width)
      .filter(|width| width.is_finite() && *width < 100_000.0)
      .unwrap_or(f32::MAX);
    let mut inner = self.inner.lock();
    inner.styled.resize = Some(ObjectResize {
      at: object.at,
      from_x: x + inner.scroll_x,
      from_width: room.width,
      most,
      changed: false,
    });
    true
  }

  /// Moves a drag of a handle to `x`: the object is as much wider as the pointer went. `false` where no drag
  /// is in progress.
  pub(super) fn drag_object_resize(&self, x: f32) -> bool {
    let (resize, scroll_x) = {
      let inner = self.inner.lock();
      let Some(resize) = inner.styled.resize.clone() else {
        return false;
      };
      (resize, inner.scroll_x)
    };
    let least = OBJECT_LEAST_WIDTH.min(resize.most);
    let width = (resize.from_width + x + scroll_x - resize.from_x).clamp(least, resize.most.max(least));
    let width = width.round() as u32;
    let Some(objects) = self.current_objects() else {
      return true;
    };
    let Some(index) = objects.iter().position(|object| object.at == resize.at) else {
      return true;
    };
    // An object with no width of its own is as wide as it was laid out until the drag moves it.
    let held = objects[index].width.unwrap_or(resize.from_width.round() as u32);
    if held == width {
      return true;
    }
    if !resize.changed {
      self.push_undo_snapshot();
      if let Some(resize) = self.inner.lock().styled.resize.as_mut() {
        resize.changed = true;
      }
    }
    let mut changed = objects.to_vec();
    changed[index].width = Some(width);
    self.write_objects(changed);
    self.mark_layout_dirty_keeping_scroll();
    true
  }

  /// Ends a drag of a handle: what it changed is reported, once.
  pub(crate) fn end_object_resize(&self) {
    let Some(resize) = self.inner.lock().styled.resize.take() else {
      return;
    };
    if !resize.changed {
      return;
    }
    if let Some(spans) = self.current_spans() {
      let range = resize.at..resize.at + crate::components::OBJECT_CHARACTER.len_utf8();
      self.commit_spans(spans.to_vec(), StyledEdit::Style { range });
    }
  }

  /// Notes whether the pointer, at `point` in the coordinates the caret is placed by, is on a handle.
  pub(super) fn hover_object_handle(&self, point: Option<(f32, f32)>) {
    let on = point.is_some_and(|(x, y)| self.object_handle_at(x, y).is_some());
    self.inner.lock().styled.handle_hovered = on;
  }

  /// Whether the pointer is on the handle of an object, or drags it: what the cursor says.
  pub(crate) fn object_handle_hovered(&self) -> bool {
    let inner = self.inner.lock();
    inner.styled.handle_hovered || inner.styled.resize.is_some()
  }

  /// Writes the objects, without a report.
  fn write_objects(&self, objects: Vec<InlineObject>) {
    let Some(signal) = self.inner.lock().styled.objects.clone() else {
      return;
    };
    if signal.with_untracked(|current| *current != objects) {
      signal.set(objects);
    }
    self.inner.lock().styled.objects_synced = signal.version();
  }

  /// Puts back the objects of an undo snapshot. The spans are put back after them, which reports the change.
  pub(super) fn restore_objects(&self, objects: SnapshotObjects) {
    self.write_objects(objects.to_vec());
  }

  /// Moves the objects over an edit that replaced `range` with `inserted` bytes, among them `added`, whose
  /// bytes are counted from the start of what was inserted: before the spans are committed and the change is
  /// reported.
  fn splice_objects(&self, edit: &SpansBeforeEdit, range: &Range<usize>, inserted: usize, added: &[InlineObject]) {
    let Some(objects) = &edit.objects else {
      return;
    };
    let moved = spans_ops::splice_objects(objects, range.clone(), inserted);
    let moved = spans_ops::place_objects(moved, range.start, added);
    self.write_objects(spans_ops::normalize_objects(&self.shared_value(), moved));
  }

  /// Puts an object in place of the selection, as one undo step. `false` for an input without objects, and
  /// where an `on_input` handler refused the edit.
  pub(crate) fn insert_object(&self, object: &InlineObject, keyboard: &KeyboardEvent) -> bool {
    if self.current_objects().is_none() {
      return false;
    }
    let text = StyledText::plain(crate::components::OBJECT_CHARACTER.to_string());
    let object = InlineObject {
      at: 0,
      ..object.clone()
    };
    let insert = InsertedText {
      styled: &text,
      kinds: None,
      objects: std::slice::from_ref(&object),
    };
    self.replace_selection_inserting(&insert, keyboard)
  }

  /// The kinds of the paragraphs of the current text, one for each, shared;
  /// `None` for an input without kinds of paragraphs.
  pub(crate) fn current_paragraphs(&self) -> Option<SnapshotParagraphs> {
    let signal = {
      let inner = self.inner.lock();
      if inner.mask.is_some() || inner.styled.spans.is_none() {
        return None;
      }
      inner.styled.paragraphs.clone()?
    };
    let count = spans_ops::paragraph_ranges(&self.shared_value()).len();
    Some(Arc::new(
      signal.with_untracked(|kinds| spans_ops::normalize_kinds(kinds.clone(), count)),
    ))
  }

  /// What the input draws its paragraphs as: the kind of each paragraph of
  /// the text it shows (with a composition in progress), and the styles of
  /// the kinds.
  pub(crate) fn paragraph_display(&self) -> Option<(SnapshotParagraphs, Arc<ParagraphStyles>)> {
    let styles = self.inner.lock().styled.paragraph_styles.clone()?;
    let kinds = self.current_paragraphs()?;
    let Some(composed) = self.composed_text() else {
      return Some((kinds, styles));
    };
    let value = self.shared_value();
    let (start, end) = self.selection_or_caret(&value);
    let inserted = composed.len() + (end - start) - value.len();
    let shown = spans_ops::splice_kinds(
      &kinds,
      &spans_ops::paragraph_ranges(&value),
      &spans_ops::paragraph_ranges(&composed),
      start..end,
      inserted,
    );
    Some((Arc::new(shown), styles))
  }

  /// Writes the kinds of the paragraphs, without a report.
  fn write_paragraphs(&self, kinds: Vec<ParagraphKind>) {
    let Some(signal) = self.inner.lock().styled.paragraphs.clone() else {
      return;
    };
    if signal.with_untracked(|current| *current != kinds) {
      signal.set(kinds);
    }
    self.inner.lock().styled.paragraphs_synced = signal.version();
  }

  /// Puts back the kinds of an undo snapshot. The spans are put back after
  /// them, which reports the change.
  pub(super) fn restore_paragraphs(&self, kinds: SnapshotParagraphs) {
    self.write_paragraphs(kinds.to_vec());
  }

  /// The paragraphs the selection touches, or the one the caret is in: their
  /// indices, and where each paragraph of the text is.
  fn selected_paragraph_range(&self) -> (Range<usize>, Vec<Range<usize>>) {
    let value = self.shared_value();
    let ranges = spans_ops::paragraph_ranges(&value);
    let (start, end) = self.selection_or_caret(&value);
    let first = spans_ops::paragraph_of(&ranges, start);
    let last = spans_ops::paragraph_of(&ranges, end);
    (first..last + 1, ranges)
  }

  /// The kinds of the paragraphs the selection touches, in order.
  pub(crate) fn selected_paragraphs(&self) -> Option<Vec<ParagraphKind>> {
    let kinds = self.current_paragraphs()?;
    let (range, _) = self.selected_paragraph_range();
    Some(kinds[range].to_vec())
  }

  /// The paragraph the caret is in: its index, its kind and where its text is.
  pub(crate) fn caret_paragraph(&self) -> Option<(usize, ParagraphKind, Range<usize>)> {
    let kinds = self.current_paragraphs()?;
    let value = self.shared_value();
    let ranges = spans_ops::paragraph_ranges(&value);
    let caret = clamp_to_char_boundary(&value, self.inner.lock().caret);
    let index = spans_ops::paragraph_of(&ranges, caret);
    Some((index, kinds[index], ranges[index].clone()))
  }

  /// Makes every paragraph the selection touches a `kind`, as one undo
  /// step. `false` where they all are already, and for an input without
  /// kinds of paragraphs.
  pub(crate) fn set_paragraph_kind(&self, kind: ParagraphKind) -> bool {
    let Some(kinds) = self.current_paragraphs() else {
      return false;
    };
    let kind = kind.canonical();
    let (range, _) = self.selected_paragraph_range();
    if kinds[range.clone()].iter().all(|held| *held == kind) {
      return false;
    }
    let mut changed = kinds.to_vec();
    changed[range.clone()].fill(kind);
    self.push_undo_snapshot();
    self.write_paragraphs(changed);
    if let Some(spans) = self.current_spans() {
      self.commit_spans(spans.to_vec(), StyledEdit::Paragraphs { range });
    }
    self.mark_layout_dirty();
    true
  }

  /// Moves the kinds of the paragraphs over an edit, before its spans are
  /// committed and the change is reported.
  fn splice_paragraphs(&self, edit: &SpansBeforeEdit, range: &Range<usize>, inserted: usize) {
    let Some((kinds, before)) = &edit.paragraphs else {
      return;
    };
    let after = spans_ops::paragraph_ranges(&self.shared_value());
    let mut kinds = spans_ops::splice_kinds(kinds, before, &after, range.clone(), inserted);
    // A divider holds no text: one an edit put text in is body text.
    for (kind, paragraph) in kinds.iter_mut().zip(&after) {
      if *kind == ParagraphKind::Divider && !paragraph.is_empty() {
        *kind = ParagraphKind::Body;
      }
    }
    self.write_paragraphs(kinds);
  }

  pub(crate) fn add_styled_change_handler(&self, handler: impl Fn(&StyledChange) + Send + Sync + 'static) {
    self.inner.lock().styled.on_change.push(Arc::new(handler));
  }

  pub(crate) fn set_copy_format(&self, format: impl Fn(&StyledText) -> String + Send + Sync + 'static) {
    self.inner.lock().styled.copy_format = Some(Arc::new(format));
  }

  pub(crate) fn add_link_click_handler(&self, handler: impl Fn(&LinkClick) + Send + Sync + 'static) {
    self.inner.lock().styled.on_link_click.push(Arc::new(handler));
  }

  /// Whether the input edits styled text: it has a spans signal and is not
  /// masked.
  pub(crate) fn is_styled(&self) -> bool {
    let inner = self.inner.lock();
    inner.styled.spans.is_some() && inner.mask.is_none()
  }

  /// The value. A copy: the value signal's own allocation is never shared,
  /// since `Signal::update` writes it in place.
  fn shared_value(&self) -> String {
    self.value()
  }

  /// The spans of the current text in canonical form, shared; `None` for a
  /// plain input. They are normalized again only after the value or the
  /// spans signal changed.
  pub(crate) fn current_spans(&self) -> Option<SnapshotSpans> {
    let (signal, cache) = {
      let inner = self.inner.lock();
      if inner.mask.is_some() {
        return None;
      }
      (inner.styled.spans.clone()?, inner.styled.cache.clone())
    };
    let versions = (self.value.version(), signal.version());
    if let Some(cache) = cache.filter(|cache| (cache.value_version, cache.spans_version) == versions) {
      return Some(cache.spans);
    }
    let value = self.shared_value();
    let spans = Arc::new(signal.with_untracked(|spans| spans_ops::normalize(&value, spans.clone())));
    self.inner.lock().styled.cache = Some(SpansCache {
      value_version: versions.0,
      spans_version: versions.1,
      spans: spans.clone(),
    });
    Some(spans)
  }

  pub(crate) fn styled_text(&self) -> Option<StyledText> {
    let spans = self.current_spans()?;
    Some(StyledText {
      text: self.value(),
      spans: spans.to_vec(),
    })
  }

  /// What the input draws: the text, with a composition in progress in the
  /// style typing would give it, and its spans. `None` for a plain input
  /// and for an empty one showing its placeholder, unless `empty` asks for that one too: an input whose kinds
  /// of paragraphs are drawn lays its one empty paragraph out as what it is, and its placeholder in it.
  pub(crate) fn styled_display(&self, empty: bool) -> Option<(Arc<str>, SnapshotSpans)> {
    let spans = self.current_spans()?;
    let Some(composed) = self.composed_text() else {
      if self.is_showing_placeholder() && !(empty && self.current_paragraphs().is_some()) {
        return None;
      }
      return Some((self.shared_value().into(), spans));
    };
    let value = self.shared_value();
    let (start, end) = self.selection_or_caret(&value);
    let inserted = composed.len() + (end - start) - value.len();
    let style = self.typing_style_for(&value, &spans);
    let spans = spans_ops::splice(&spans, start..end, inserted, &style);
    Some((composed.into(), Arc::new(spans)))
  }

  /// Notices an app's write of the objects signal: the input lays out again.
  fn sync_external_objects(&self) {
    let mut inner = self.inner.lock();
    let Some(version) = inner.styled.objects.as_ref().map(Signal::version) else {
      return;
    };
    if version == inner.styled.objects_synced {
      return;
    }
    inner.styled.objects_synced = version;
    drop(inner);
    self.mark_layout_dirty_keeping_scroll();
  }

  /// Notices an app's write of the spans signal: the input lays out again
  /// and drops a pending typing style.
  pub(super) fn sync_external_spans(&self) {
    self.sync_external_paragraphs();
    self.sync_external_objects();
    let mut inner = self.inner.lock();
    let Some(version) = inner.styled.spans.as_ref().map(Signal::version) else {
      return;
    };
    if version == inner.styled.synced_version {
      return;
    }
    inner.styled.synced_version = version;
    inner.styled.pending = None;
    drop(inner);
    self.mark_layout_dirty_keeping_scroll();
  }

  /// Notices an app's write of the paragraphs signal: the input lays out again.
  fn sync_external_paragraphs(&self) {
    let mut inner = self.inner.lock();
    let Some(version) = inner.styled.paragraphs.as_ref().map(Signal::version) else {
      return;
    };
    if version == inner.styled.paragraphs_synced {
      return;
    }
    inner.styled.paragraphs_synced = version;
    drop(inner);
    self.mark_layout_dirty_keeping_scroll();
  }

  /// Captures what an edit needs before it changes the text.
  pub(super) fn styled_before_edit(&self) -> Option<SpansBeforeEdit> {
    let spans = self.current_spans()?;
    let value = self.shared_value();
    let style = self.typing_style_for(&value, &spans);
    let paragraphs = self
      .current_paragraphs()
      .map(|kinds| (kinds, spans_ops::paragraph_ranges(&value)));
    Some(SpansBeforeEdit {
      spans,
      style,
      paragraphs,
      objects: self.current_objects(),
    })
  }

  /// Moves the spans over an edit that replaced `range` with `inserted`
  /// bytes typed in the captured style, and reports the change.
  pub(super) fn styled_after_edit(&self, edit: Option<SpansBeforeEdit>, range: Range<usize>, inserted: usize) {
    let Some(edit) = edit else {
      return;
    };
    self.splice_paragraphs(&edit, &range, inserted);
    self.splice_objects(&edit, &range, inserted, &[]);
    let spans = spans_ops::splice(&edit.spans, range.clone(), inserted, &edit.style);
    self.commit_spans(
      spans,
      StyledEdit::Text {
        replaced: range,
        inserted,
      },
    );
  }

  /// Like [`Self::styled_after_edit`] for inserted text that brings its own
  /// spans, relative to its start.
  pub(super) fn styled_after_insert(
    &self,
    edit: Option<SpansBeforeEdit>,
    range: Range<usize>,
    inserted: &InsertedText<'_>,
  ) {
    let Some(edit) = edit else {
      return;
    };
    let (inserted, pasted, objects) = (inserted.styled, inserted.kinds, inserted.objects);
    self.splice_paragraphs(&edit, &range, inserted.text.len());
    self.splice_objects(&edit, &range, inserted.text.len(), objects);
    if let (Some(pasted), Some(kinds)) = (pasted, self.current_paragraphs()) {
      let after = spans_ops::paragraph_ranges(&self.shared_value());
      self.write_paragraphs(spans_ops::place_kinds(
        kinds.to_vec(),
        &after,
        range.start,
        inserted.text.len(),
        pasted,
      ));
    }
    let start = range.start;
    let mut spans = spans_ops::splice(&edit.spans, range.clone(), inserted.text.len(), &SpanStyle::default());
    spans.extend(
      inserted
        .spans
        .iter()
        .map(|span| StyledSpan::new(span.range.start + start..span.range.end + start, span.style.clone())),
    );
    let spans = spans_ops::normalize(&self.shared_value(), spans);
    self.commit_spans(
      spans,
      StyledEdit::Text {
        replaced: range,
        inserted: inserted.text.len(),
      },
    );
  }

  /// Puts back the spans of an undo snapshot.
  pub(super) fn restore_spans(&self, spans: SnapshotSpans) {
    self.commit_spans(spans.to_vec(), StyledEdit::Replaced);
  }

  /// Writes `spans` for the current text and reports the change to the
  /// app's change handlers, sharing the spans with them.
  fn commit_spans(&self, spans: Vec<StyledSpan>, edit: StyledEdit) {
    let (signal, handlers) = {
      let mut inner = self.inner.lock();
      let Some(signal) = inner.styled.spans.clone() else {
        return;
      };
      inner.styled.pending = None;
      (signal, inner.styled.on_change.clone())
    };
    let shared = Arc::new(spans);
    if signal.with_untracked(|current| current != shared.as_ref()) {
      signal.set(shared.to_vec());
    }
    {
      let mut inner = self.inner.lock();
      inner.styled.synced_version = signal.version();
      inner.styled.cache = Some(SpansCache {
        value_version: self.value.version(),
        spans_version: signal.version(),
        spans: shared.clone(),
      });
    }
    if handlers.is_empty() {
      return;
    }
    let change = StyledChange {
      value: self.value.clone(),
      spans: shared,
      paragraphs: self.current_paragraphs(),
      objects: self.current_objects(),
      edit,
    };
    for handler in handlers {
      handler(&change);
    }
  }

  /// The marks over the selection, or of the next typed text.
  pub(crate) fn selection_style(&self) -> Option<SelectionStyle> {
    let spans = self.current_spans()?;
    let value = self.shared_value();
    let (start, end) = self.selection_or_caret(&value);
    if start < end {
      return Some(spans_ops::summarize(&spans, value.len(), start..end));
    }
    Some(SelectionStyle::of_style(&self.typing_style_for(&value, &spans)))
  }

  /// Turns `mark` on over the selection unless all of it has it, then off;
  /// with nothing selected, for the next typed text. `false` for a plain
  /// input.
  pub(crate) fn toggle_mark(&self, mark: TextMark) -> bool {
    let Some(style) = self.selection_style() else {
      return false;
    };
    let on = style.mark(mark) != crate::components::MarkPresence::All;
    self.set_mark(mark, on)
  }

  pub(crate) fn set_mark(&self, mark: TextMark, on: bool) -> bool {
    self.restyle_selection(|style| *style = std::mem::take(style).with(mark, on))
  }

  /// Links the selection to `target`, or unlinks it with `None`. With
  /// nothing selected it changes the link the caret is in, else the next
  /// typed text.
  pub(crate) fn set_link(&self, target: Option<Arc<str>>) -> bool {
    let Some(spans) = self.current_spans() else {
      return false;
    };
    let value = self.shared_value();
    let (start, end) = self.selection_or_caret(&value);
    if start == end
      && let Some(range) = link_range(&spans, start).filter(|range| range.start < start && start < range.end)
    {
      return self.restyle_range(&spans, range, |style| style.link = target.clone());
    }
    self.restyle_selection(|style| style.link = target.clone())
  }

  /// Sets the style the next typed text takes at the caret; `None` goes
  /// back to the style of the text before the caret.
  pub(crate) fn set_pending_style(&self, style: Option<SpanStyle>) -> bool {
    if !self.is_styled() {
      return false;
    }
    let mut inner = self.inner.lock();
    inner.styled.pending = style.map(|style| (inner.caret, style));
    true
  }

  /// The selection with its spans, offsets from its start.
  pub(crate) fn selected_styled_text(&self) -> Option<StyledText> {
    let spans = self.current_spans()?;
    let value = self.shared_value();
    let (start, end) = self.selection_or_caret(&value);
    Some(StyledText {
      text: value[start..end].to_owned(),
      spans: spans_ops::slice(&spans, start..end),
    })
  }

  /// The objects in the selection, their bytes counted from its start.
  fn selected_objects(&self) -> Vec<InlineObject> {
    let Some(objects) = self.current_objects() else {
      return Vec::new();
    };
    let (start, end) = self.selection_or_caret(&self.shared_value());
    spans_ops::slice_objects(&objects, start..end)
  }

  /// What copying the selection puts on the clipboard: the app's format of
  /// the styled selection when it gave one, else the selected text, with
  /// the label of an object in its place. Nothing while the input is
  /// masked, as a password field copies nothing.
  pub(crate) fn clipboard_selection(&self) -> Option<String> {
    if self.is_masked() {
      return None;
    }
    let selected = self.selected_text()?;
    let format = self.inner.lock().styled.copy_format.clone();
    match (format, self.selected_styled_text()) {
      (Some(format), Some(styled)) => Some(format(&styled)),
      _ => Some(spans_ops::text_with_labels(&selected, &self.selected_objects())),
    }
  }

  pub(crate) fn set_rich_clipboard(&self, on: bool) {
    self.inner.lock().styled.rich_clipboard = on;
  }

  /// The Markdown of the selection, which a copy keeps beside its text: its marks and the kinds of the
  /// paragraphs it touches. Only an input with kinds of paragraphs that keeps its formatting through the
  /// clipboard has one, and never while it is masked.
  pub(crate) fn clipboard_markdown(&self) -> Option<String> {
    if !self.inner.lock().styled.rich_clipboard || self.is_masked() {
      return None;
    }
    let kinds = self.current_paragraphs()?;
    let selected = self.selected_styled_text()?;
    if selected.text.is_empty() {
      return None;
    }
    let (range, _) = self.selected_paragraph_range();
    let rich =
      RichText::new(selected.text, selected.spans, kinds[range].to_vec()).with_objects(self.selected_objects());
    Some(rich.to_markdown())
  }

  /// Pastes formatted text into an input that keeps its formatting through the clipboard: the `markdown` beside
  /// the text, or the text itself where it reads as Markdown with block marks. `None` where the paste is plain
  /// text: an input without kinds of paragraphs, a caret in a line of code, a text that is not Markdown.
  pub(crate) fn paste_rich(&self, text: &str, markdown: Option<&str>, keyboard: &KeyboardEvent) -> Option<bool> {
    if !self.inner.lock().styled.rich_clipboard || self.is_masked() {
      return None;
    }
    let kinds = self.current_paragraphs()?;
    // Code is pasted into code as it is written.
    let (touched, _) = self.selected_paragraph_range();
    if kinds[touched.start].is_code() {
      return None;
    }
    let source = match markdown {
      Some(markdown) => markdown,
      None if RichText::reads_as_markdown(text) => text,
      None => return None,
    };
    let rich = RichText::from_markdown(source);
    if rich.is_empty() {
      return None;
    }
    Some(self.replace_selection_rich(&rich, keyboard))
  }

  pub(crate) fn set_text_marks(&self, marks: Option<crate::app::theme::ThemeTextMarks>) {
    self.inner.lock().styled.text_marks = marks.map(Arc::new);
  }

  /// The look of the marks of this input where it has one of its own.
  pub(crate) fn text_marks(&self) -> Option<Arc<crate::app::theme::ThemeTextMarks>> {
    self.inner.lock().styled.text_marks.clone()
  }

  pub(crate) fn set_checks_on_press(&self, on: bool) {
    self.inner.lock().styled.checks_on_press = on;
  }

  /// Where the paragraph starts whose box of a checklist's item is under a point of the content box, where the
  /// input lets a press check it.
  fn check_under(&self, x: f32, y: f32) -> Option<usize> {
    if !self.inner.lock().styled.checks_on_press {
      return None;
    }
    let layout = self.token_layout()?;
    let (x, y) = {
      let inner = self.inner.lock();
      (x + inner.scroll_x, y + inner.scroll_y())
    };
    layout.check_at(x, y)
  }

  /// Whether a press at a point of the content box is on the box of a checklist's item that it checks.
  pub(crate) fn check_at_point(&self, x: f32, y: f32) -> bool {
    self.check_under(x, y).is_some()
  }

  /// Checks or unchecks the item of a checklist whose box is under a point of the content box, as one undo step,
  /// where the input lets a press do it. `false` where no box is there.
  pub(crate) fn press_check_at_point(&self, x: f32, y: f32) -> bool {
    let Some(start) = self.check_under(x, y) else {
      return false;
    };
    let ranges = spans_ops::paragraph_ranges(&self.shared_value());
    let index = spans_ops::paragraph_of(&ranges, start);
    self.toggle_checks(index..index + 1)
  }

  /// Checks the items of a checklist the selection touches, or unchecks them where they all are checked, as one
  /// undo step. `false` where it touches none.
  pub(crate) fn toggle_selected_checks(&self) -> bool {
    let (range, _) = self.selected_paragraph_range();
    self.toggle_checks(range)
  }

  /// Checks the items of a checklist among the paragraphs `range`, or unchecks them where they all are checked.
  fn toggle_checks(&self, range: Range<usize>) -> bool {
    let Some(kinds) = self.current_paragraphs() else {
      return false;
    };
    let range = range.start.min(kinds.len())..range.end.min(kinds.len());
    let items: Vec<usize> = range
      .clone()
      .filter(|index| matches!(kinds[*index], ParagraphKind::Checklist { .. }))
      .collect();
    if items.is_empty() {
      return false;
    }
    let checked = !items
      .iter()
      .all(|index| kinds[*index] == ParagraphKind::Checklist { checked: true });
    let mut changed = kinds.to_vec();
    for index in items {
      changed[index] = ParagraphKind::Checklist { checked };
    }
    self.push_undo_snapshot();
    self.write_paragraphs(changed);
    if let Some(spans) = self.current_spans() {
      self.commit_spans(spans.to_vec(), StyledEdit::Paragraphs { range });
    }
    self.mark_layout_dirty();
    true
  }

  /// Puts a divider after the paragraph the caret is in, or makes that paragraph one where it is empty body text,
  /// with a paragraph of body text after it that the caret is put in, as one undo step. `false` in an input
  /// without kinds of paragraphs.
  pub(crate) fn insert_divider(&self, keyboard: &KeyboardEvent) -> bool {
    let Some((index, kind, range)) = self.caret_paragraph() else {
      return false;
    };
    let empty = range.is_empty() && kind == ParagraphKind::Body;
    let (at, text, divider) = if empty {
      (range.start, "\n", index)
    } else {
      (range.end, "\n\n", index + 1)
    };
    let len = self.shared_value().len();
    if !self.controller_select(at.min(len), at.min(len)) {
      return false;
    }
    if !self.replace_selection_text(text, keyboard) {
      return false;
    }
    // The edit is the undo step: the kinds it leaves are made what they are to be without another.
    let Some(kinds) = self.current_paragraphs() else {
      return true;
    };
    let mut changed = kinds.to_vec();
    if divider + 1 < changed.len() {
      changed[divider] = ParagraphKind::Divider;
      changed[divider + 1] = ParagraphKind::Body;
    }
    self.write_paragraphs(changed);
    if let Some(spans) = self.current_spans() {
      self.commit_spans(
        spans.to_vec(),
        StyledEdit::Paragraphs {
          range: divider..divider + 2,
        },
      );
    }
    self.mark_layout_dirty();
    true
  }

  /// Reports the link under a point of the content box to the input's link
  /// handlers. `false` when no handler or no link is there.
  pub(crate) fn click_link_at_point(&self, x: f32, y: f32) -> bool {
    let handlers = self.inner.lock().styled.on_link_click.clone();
    if handlers.is_empty() {
      return false;
    }
    let Some(click) = self.link_at_point(x, y) else {
      return false;
    };
    for handler in handlers {
      handler(&click);
    }
    true
  }

  /// The link drawn under a point of the content box.
  fn link_at_point(&self, x: f32, y: f32) -> Option<LinkClick> {
    let spans = self.current_spans()?;
    let layout = self.token_layout()?;
    let (scroll_x, scroll_y) = {
      let inner = self.inner.lock();
      (inner.scroll_x, inner.scroll_y())
    };
    let cluster = layout.cluster_at(x + scroll_x, y + scroll_y)?;
    let range = link_range(&spans, cluster.start)?;
    let target = spans_ops::style_at(&spans, cluster.start).link?;
    Some(LinkClick { target, range })
  }

  fn restyle_selection(&self, change: impl Fn(&mut SpanStyle)) -> bool {
    let Some(spans) = self.current_spans() else {
      return false;
    };
    let value = self.shared_value();
    let (start, end) = self.selection_or_caret(&value);
    if start == end {
      let mut style = self.typing_style_for(&value, &spans);
      change(&mut style);
      let mut inner = self.inner.lock();
      inner.styled.pending = Some((inner.caret, style));
      return true;
    }
    self.restyle_range(&spans, start..end, change)
  }

  fn restyle_range(&self, spans: &[StyledSpan], range: Range<usize>, change: impl Fn(&mut SpanStyle)) -> bool {
    let restyled = spans_ops::restyle(spans, self.shared_value().len(), range.clone(), change);
    if restyled.as_slice() == spans {
      return false;
    }
    self.push_undo_snapshot();
    self.commit_spans(restyled, StyledEdit::Style { range });
    self.mark_layout_dirty();
    true
  }

  /// The style typed text takes over the selection or at the caret.
  fn typing_style_for(&self, value: &str, spans: &[StyledSpan]) -> SpanStyle {
    let (start, end) = self.selection_or_caret(value);
    let inner = self.inner.lock();
    if let Some((caret, style)) = &inner.styled.pending
      && start == end
      && *caret == start
    {
      return style.clone();
    }
    drop(inner);
    spans_ops::insertion_style(value, spans, start, end)
  }

  fn selection_or_caret(&self, value: &str) -> (usize, usize) {
    let inner = self.inner.lock();
    selection_range_indices(value, inner.selection_anchor, inner.caret).unwrap_or_else(|| {
      let caret = clamp_to_char_boundary(value, inner.caret);
      (caret, caret)
    })
  }
}

/// The whole link around `offset`: the spans next to each other that carry
/// the link of the span at `offset` (or ending there).
fn link_range(spans: &[StyledSpan], offset: usize) -> Option<Range<usize>> {
  let index = spans
    .iter()
    .position(|span| span.range.start <= offset && offset < span.range.end && span.style.link.is_some())
    .or_else(|| {
      spans
        .iter()
        .position(|span| span.range.end == offset && span.style.link.is_some())
    })?;
  let target = spans[index].style.link.clone();
  let mut first = index;
  while first > 0 && spans[first - 1].range.end == spans[first].range.start && spans[first - 1].style.link == target {
    first -= 1;
  }
  let mut last = index;
  while last + 1 < spans.len()
    && spans[last + 1].range.start == spans[last].range.end
    && spans[last + 1].style.link == target
  {
    last += 1;
  }
  Some(spans[first].range.start..spans[last].range.end)
}
