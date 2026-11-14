//! The page of the toolbar: its playground with a width to pick, the board's
//! bar of marks, a narrow bar whose controls go into its menu, a table's bar,
//! and a command bar of labelled groups.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::RadiusSize},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    Button, ButtonSize, ButtonVariant, Choice, InputSize, Length, SearchField, SearchFieldPart, Select, SelectSize,
    Tabs, TabsPart, TabsVariant, Tag, TagVariant, Toggle, ToggleGroup, Toolbar, ToolbarCommand, ToolbarGroup,
    ToolbarItem, ToolbarPart, ToolbarVariant, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The variants the playground offers, as the code names them.
const VARIANTS: [(&str, ToolbarVariant); 3] = [
  ("Surface", ToolbarVariant::Surface),
  ("Plain", ToolbarVariant::Plain),
  ("Floating", ToolbarVariant::Floating),
];

/// The sizes the playground offers.
const SIZES: [(&str, ButtonSize); 3] = [("Xs", ButtonSize::Xs), ("Sm", ButtonSize::Sm), ("Md", ButtonSize::Md)];

/// The widths the playground offers the bar: its own, and narrower ones that its controls overflow.
const WIDTHS: [(&str, Option<f32>); 5] = [
  ("Its own", None),
  ("360", Some(360.0)),
  ("280", Some(280.0)),
  ("200", Some(200.0)),
  ("140", Some(140.0)),
];

/// The marks of a text that are on, as the bars of the page hold them.
#[derive(Clone)]
struct Marks {
  bold: Signal<bool>,
  italic: Signal<bool>,
  underline: Signal<bool>,
}

impl Marks {
  /// A toggle of the bar for one mark, its icon alone, with its shortcut.
  fn toggle(
    mark: &Signal<bool>,
    label: &str,
    icon: &str,
    keys: &str,
    record: impl Fn(String) + Send + Sync + 'static,
  ) -> ToolbarItem {
    let set = mark.clone();
    let name = label.to_owned();
    ToolbarItem::from(
      Toggle::new(label)
        .icon(icon)
        .icon_only(true)
        .on(mark.get())
        .on_change(move |on| {
          set.set(on);
          record(format!("{name}: {on}"));
        }),
    )
    .shortcut(keys)
  }
}

/// The board's bar: the marks, a line, List, Link and Image, each telling `record` what it did.
fn formatting(marks: &Marks, record: impl Fn(String) + Clone + Send + Sync + 'static) -> Toolbar {
  let button = |label: &'static str, icon: &str, keys: &str| {
    let record = record.clone();
    ToolbarItem::from(
      Button::new(label)
        .icon(icon)
        .icon_only(true)
        .on_click(move |_| record(format!("{label}: pressed"))),
    )
    .shortcut(keys)
  };
  Toolbar::new()
    .label("Formatting")
    .child(Marks::toggle(
      &marks.bold,
      "Bold",
      icons::BOLD,
      "Ctrl+B",
      record.clone(),
    ))
    .child(Marks::toggle(
      &marks.italic,
      "Italic",
      icons::ITALIC,
      "Ctrl+I",
      record.clone(),
    ))
    .child(Marks::toggle(
      &marks.underline,
      "Underline",
      icons::UNDERLINE,
      "Ctrl+U",
      record.clone(),
    ))
    .child(Toolbar::separator())
    .child(button("List", icons::LIST, "Ctrl+Shift+8"))
    .child(button("Link", icons::LINK, "Ctrl+K"))
    .child(button("Image", icons::IMAGE, "Ctrl+Shift+I"))
}

/// What a bar on the page is shown in: `width` wide, or as wide as it is.
fn sized(width: Option<f32>, bar: Element) -> Element {
  match width {
    Some(width) => Column::new().width(width).overflow_visible().child(bar).into(),
    None => Row::new().overflow_visible().child(bar).into(),
  }
}

pub struct ToolbarPage {
  frame: Frame,
  playground: Playground,
  marks: Marks,
  variant: Signal<usize>,
  size: Signal<usize>,
  width: Signal<usize>,
  overflow: Signal<bool>,
  label: Signal<String>,
  /// The table's bar: what its search field holds, its density, its columns.
  search: Signal<String>,
  density: Signal<u8>,
  columns: Signal<u8>,
}

impl ToolbarPage {
  /// The playground: the board's bar as the panel's props make it, as wide as the width picked.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (variant_name, variant) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (size_name, size) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (width_name, width) = WIDTHS[self.width.get().min(WIDTHS.len() - 1)];
    let (overflow, label) = (self.overflow.get(), self.label.get());
    let plain = {
      let (marks, label, record) = (self.marks.clone(), label.clone(), self.playground.recorder());
      move || {
        formatting(&marks, record.clone())
          .variant(variant)
          .size(size)
          .overflow(overflow)
          .label(&label)
      }
    };
    let shown = sized(width, plain().mount(ctx));

    let mut code = vec![
      "Toolbar::new()".to_owned(),
      format!("    .label({label:?})"),
      "    .child(ToolbarItem::from(Toggle::new(\"Bold\").icon(icons::BOLD).icon_only(true)).shortcut(\"Ctrl+B\"))"
        .to_owned(),
      "    .child(Toggle::new(\"Italic\").icon(icons::ITALIC).icon_only(true))".to_owned(),
      "    .child(Toggle::new(\"Underline\").icon(icons::UNDERLINE).icon_only(true))".to_owned(),
      "    .child(Toolbar::separator())".to_owned(),
      "    .child(Button::new(\"List\").icon(icons::LIST).icon_only(true))".to_owned(),
      "    .child(Button::new(\"Link\").icon(icons::LINK).icon_only(true))".to_owned(),
      "    .child(Button::new(\"Image\").icon(icons::IMAGE).icon_only(true))".to_owned(),
    ];
    if variant != ToolbarVariant::Surface {
      code.push(format!("    .variant(ToolbarVariant::{variant_name})"));
    }
    if size != ButtonSize::Sm {
      code.push(format!("    .size(ButtonSize::{size_name})"));
    }
    if overflow {
      code.push("    .overflow(true)".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root        {} · padding 4 · gap 2", self.playground.measured()),
      "├─ item      ghost button or toggle · 32 · icon 16".to_owned(),
      "├─ separator 1 × 20 · border · 4 either side".to_owned(),
      "├─ item …".to_owned(),
      "└─ more      line · More (more-horizontal) · menu of what did not fit".to_owned(),
    ];

    let (to_variant, to_size, to_width) = (self.variant.clone(), self.size.clone(), self.width.clone());
    let (to_overflow, to_label) = (self.overflow.clone(), self.label.clone());
    let (set_variant, set_size, set_width) = (self.variant.clone(), self.size.clone(), self.width.clone());
    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let sizes: Vec<(&str, usize)> = SIZES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let widths: Vec<(&str, usize)> = WIDTHS
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let make = {
      let plain = plain.clone();
      move || sized(width, plain().into())
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Toolbar",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: false,
        facts: match width {
          Some(_) => format!("{width_name} wide"),
          None => "as wide as its controls".to_owned(),
        },
        event: "on_click / on_change",
        code,
        tree,
        controls: vec![
          prop(
            "variant",
            "ToolbarVariant",
            self
              .playground
              .select(0, &variants, &self.variant.get(), move |index| set_variant.set(index)),
          ),
          prop(
            "size",
            "ButtonSize",
            self
              .playground
              .select(1, &sizes, &self.size.get(), move |index| set_size.set(index)),
          ),
          prop("label", "&str", field(&self.label)),
          divider(),
          flag("overflow", &self.overflow),
          prop(
            "width",
            "of what holds it",
            self
              .playground
              .select(2, &widths, &self.width.get(), move |index| set_width.set(index)),
          ),
        ],
        changed: [
          variant != ToolbarVariant::Surface,
          size != ButtonSize::Sm,
          !overflow,
          width.is_some(),
          label != "Formatting",
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_variant.set(0);
          to_size.set(1);
          to_width.set(0);
          to_overflow.set(true);
          to_label.set("Formatting".to_owned());
        }),
      },
    )
  }

  /// A table's bar (the board's Table Toolbar): a search field, the filters
  /// that are set, Filter, and at the end the density, the columns and Export.
  fn table(&self, ctx: &mut Ctx) -> Element {
    let density = self.density.clone();
    let columns = self.columns.clone();
    Toolbar::new()
      .label("Crates")
      .overflow(true)
      .style(ToolbarPart::Root, |s| {
        s.width(Length::Fill)
          .height(52.0)
          .padding_x(12.0)
          .gap(8.0)
          .radius(RadiusSize::Lg)
      })
      .child(
        ToolbarItem::element(
          SearchField::new(self.search.clone())
            .placeholder("Filter crates")
            .size(InputSize::Sm)
            .style(SearchFieldPart::Root, |s| s.width(220.0)),
        )
        .stays(true),
      )
      .child(ToolbarItem::element(
        Tag::new("Status: Passing").variant(TagVariant::Info).on_remove(|| {}),
      ))
      .child(ToolbarItem::element(
        Tag::new("Kind: Rust").variant(TagVariant::Info).on_remove(|| {}),
      ))
      .child(
        ToolbarItem::from(
          Button::new("Filter")
            .icon(icons::PLUS)
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Xs),
        )
        .as_given(true),
      )
      .child(Toolbar::spacer())
      .child(ToggleGroup::new(
        density,
        [
          Choice::new(2u8, "Comfortable").icon(icons::ROWS_2).icon_only(true),
          Choice::new(3u8, "Cozy").icon(icons::ROWS_3).icon_only(true),
          Choice::new(4u8, "Compact").icon(icons::ROWS_4).icon_only(true),
        ],
      ))
      .child(
        ToolbarItem::element(
          Select::new(columns.clone(), [(0u8, "Columns"), (1, "Name"), (2, "Status")]).size(SelectSize::Sm),
        )
        .in_menu(
          ToolbarCommand::new("Columns")
            .icon(icons::SLIDERS)
            .on_select(move || columns.set(0)),
        ),
      )
      .child(
        ToolbarItem::from(
          Button::new("Export")
            .icon(icons::DOWNLOAD)
            .variant(ButtonVariant::Outline),
        )
        .as_given(true),
      )
      .mount(ctx)
  }

  /// A command bar (the board's Command Bar): tabs over labelled groups.
  fn command_bar(&self, ctx: &mut Ctx) -> Element {
    let tabs = Tabs::controlled(0, [(0, "Home"), (1, "Insert"), (2, "View"), (3, "Help")], |_| {})
      .variant(TabsVariant::Line)
      .style(TabsPart::List, |s| s.height(32.0).gap(2.0).padding_x(8.0))
      .style(TabsPart::Tab, |s| s.height(32.0).padding_x(12.0));
    let command = |label: &str, icon: &str| Button::new(label).icon(icon);
    let bar = Toolbar::new()
      .label("Home")
      .variant(ToolbarVariant::Plain)
      .size(ButtonSize::Xs)
      .overflow(true)
      .child(
        ToolbarGroup::new()
          .label("Clipboard")
          .child(command("Paste", icons::PASTE))
          .child(command("Cut", icons::CUT))
          .child(command("Copy", icons::COPY)),
      )
      .child(
        ToolbarGroup::new()
          .label("Format")
          .child(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).on(true))
          .child(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true))
          .child(Toggle::new("Underline").icon(icons::UNDERLINE).icon_only(true))
          .child(Toggle::new("Strikethrough").icon(icons::STRIKETHROUGH).icon_only(true)),
      )
      .child(
        ToolbarGroup::new()
          .label("View")
          .child(command("Sidebar", icons::PANEL_LEFT))
          .child(command("Zoom in", icons::ZOOM_IN))
          .child(command("Zoom out", icons::ZOOM_OUT)),
      )
      .mount(ctx);
    Column::new()
      .width(FILL)
      .align_items(Alignment::Stretch)
      .child(tabs)
      .child(bar)
      .into()
  }
}

impl Component for ToolbarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      marks: Marks {
        bold: ctx.signal(true),
        italic: ctx.signal(false),
        underline: ctx.signal(false),
      },
      variant: ctx.signal(0),
      size: ctx.signal(1),
      width: ctx.signal(0),
      overflow: ctx.signal(true),
      label: ctx.signal("Formatting".to_owned()),
      search: ctx.signal(String::new()),
      density: ctx.signal(3),
      columns: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let record = |_: String| {};
    let board = formatting(&self.marks, record).mount(ctx);
    let mut narrow = Row::new()
      .spacing(32.0)
      .align_items(Alignment::Start)
      .overflow_visible();
    for width in [280.0, 200.0, 140.0] {
      narrow = narrow.child(
        Column::new()
          .spacing(8.0)
          .overflow_visible()
          .child(sized(
            Some(width),
            formatting(&self.marks, record).overflow(true).mount(ctx),
          ))
          .child(quiet(&format!("{width} wide"), 12.0)),
      );
    }
    let floating = formatting(&self.marks, record)
      .variant(ToolbarVariant::Floating)
      .mount(ctx);
    let table = self.table(ctx);
    let command_bar = self.command_bar(ctx);

    let rows = [
      (
        "new",
        "Toolbar",
        "",
        "An empty bar on a surface, its controls of the small size: 32, so the bar is 40 high.",
      ),
      (
        "child",
        "impl Into<ToolbarItem>",
        "",
        "A Button, a Toggle, a ToggleGroup, a Select, a ToolbarGroup, Toolbar::separator(), Toolbar::spacer(), or \
         any element with ToolbarItem::element.",
      ),
      (
        "mount",
        "(ctx)",
        "",
        "Puts the bar in the tree: it keeps the menu of what does not fit and where the focus left it.",
      ),
      (
        "variant",
        "ToolbarVariant",
        "Surface",
        "Surface: the background's fill and an edge. Plain: neither. Floating: the popover's fill, an edge and a \
         shadow.",
      ),
      (
        "size",
        "ButtonSize",
        "Sm",
        "The size of its buttons and toggles, which are ghost ones unless an item is as_given.",
      ),
      (
        "overflow",
        "bool",
        "false",
        "Where the bar is narrower than its controls, the last go into a menu behind More at its end.",
      ),
      ("label", "&str", "", "What the bar is called, to what reads the tree."),
      (
        "ToolbarItem::shortcut",
        "&str",
        "",
        "Keys shown in the tooltip of an icon alone and beside its item in the menu.",
      ),
      (
        "ToolbarItem::stays",
        "bool",
        "false",
        "The control never goes into the menu. A select or a field stays unless it has an item in_menu.",
      ),
      (
        "ToolbarItem::as_given",
        "bool",
        "false",
        "The button or toggle keeps its own variant, size and style.",
      ),
      (
        "ToolbarItem::in_menu",
        "ToolbarCommand",
        "",
        "What stands for a select or a field in the menu, so that it can go there.",
      ),
      (
        "ToolbarGroup::label",
        "&str",
        "",
        "A group with its label under it, as a command bar has them: it goes into the menu whole.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Navigation", "Toolbar"],
        title: "Toolbar",
        description: "The tools of what is in front of a person, in a row: the marks of a text, the actions on a \
                      table, the commands of a document. One stop of Tab, whose arrow keys move between its \
                      controls; what does not fit goes into a menu at its end.",
        import: Some("use lurq::ui::{Toolbar, ToolbarGroup, ToolbarItem, ToolbarVariant};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Toolbar is a control on the right. Pick a width to see the controls that do not fit go \
           into the menu behind More, the last first; press More to use them from there. Tab into the bar, move \
           with the arrow keys, Tab out and back: the focus comes back where it left.",
          playground,
        ),
        Section::new(
          "Groups, toggles and a line between them",
          "The board's bar: ghost buttons of 32, 2 apart, inside a padding of 4. A toggle that is on has the fill \
           of what is picked out. An icon alone has a tooltip of its label and its shortcut.",
          self.frame.demo(
            0,
            Row::new().overflow_visible().child(board),
            "Toolbar::new().child(..).child(Toolbar::separator())",
            &[
              "Toolbar::new()",
              "    .label(\"Formatting\")",
              "    .child(ToolbarItem::from(Toggle::new(\"Bold\").icon(icons::BOLD).icon_only(true).on(true))",
              "        .shortcut(\"Ctrl+B\"))",
              "    .child(Toggle::new(\"Italic\").icon(icons::ITALIC).icon_only(true))",
              "    .child(Toolbar::separator())",
              "    .child(Button::new(\"Link\").icon(icons::LINK).icon_only(true))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "What does not fit",
          "The same bar at three widths. The controls that do not fit leave the bar the last first, and More opens \
           a menu of them in the order of the bar, with their icons, their shortcuts, and a check for a toggle.",
          self.frame.demo(
            1,
            narrow,
            ".overflow(true)",
            &[
              "Toolbar::new()",
              "    .child(..)",
              "    .overflow(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Floating",
          "The popover's fill, an edge and the large shadow: a bar over what it works on.",
          self.frame.demo(
            2,
            Row::new().padding_vertical(16.0).overflow_visible().child(floating),
            ".variant(ToolbarVariant::Floating)",
            &[
              "Toolbar::new()",
              "    .variant(ToolbarVariant::Floating)",
              "    .child(..)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A table's bar",
          "A search field that stays, the filters that are set, then a spacer and the density, the columns and \
           Export at the end. Narrow the window: the filters and Export go into the menu, the search stays.",
          self.frame.demo(
            3,
            Column::new().width(FILL).overflow_visible().child(table),
            "ToolbarItem::element(search).stays(true)",
            &[
              "Toolbar::new()",
              "    .overflow(true)",
              "    .child(ToolbarItem::element(SearchField::new(query)).stays(true))",
              "    .child(ToolbarItem::element(Tag::new(\"Kind: Rust\").variant(TagVariant::Info)))",
              "    .child(Toolbar::spacer())",
              "    .child(ToggleGroup::new(density, rows))",
              "    .child(ToolbarItem::element(columns).in_menu(ToolbarCommand::new(\"Columns\")))",
              "    .child(ToolbarItem::from(Button::new(\"Export\").variant(ButtonVariant::Outline)).as_given(true))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A command bar",
          "Labelled groups under tabs: each group has its label under it and a line at its end, and goes into the \
           menu whole, under its label.",
          self.frame.demo(
            4,
            command_bar,
            "ToolbarGroup::new().label(\"Clipboard\")",
            &[
              "Toolbar::new()",
              "    .variant(ToolbarVariant::Plain)",
              "    .child(ToolbarGroup::new().label(\"Clipboard\").child(paste).child(cut).child(copy))",
              "    .child(ToolbarGroup::new().label(\"Format\").child(bold).child(italic))",
              "    .overflow(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a toolbar is given, and what its items say of themselves.",
          props_table(&rows, &["new", "child", "mount"], self.frame.room()),
        ),
      ],
    )
  }
}
