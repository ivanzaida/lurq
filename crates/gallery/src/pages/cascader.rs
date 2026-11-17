//! The page of the cascader: its playground, the board's open cascader, a choice of any level, columns that open
//! under the pointer, children brought on demand, in a field, its states, and its props.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, Timeout},
  },
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Cascader, CascaderPart, CascaderSize, Field, Interaction, TreeItem},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

type Id = &'static str;

/// Places by their continent and their country.
fn places() -> Vec<TreeItem<Id>> {
  vec![
    TreeItem::new("asia", "Asia").items([
      TreeItem::new("japan", "Japan").items([TreeItem::new("tokyo", "Tokyo"), TreeItem::new("osaka", "Osaka")]),
      TreeItem::new("korea", "Korea").item(TreeItem::new("seoul", "Seoul")),
    ]),
    TreeItem::new("europe", "Europe").items([
      TreeItem::new("poland", "Poland").items([TreeItem::new("warsaw", "Warsaw"), TreeItem::new("krakow", "Kraków")]),
      TreeItem::new("ukraine", "Ukraine").items([
        TreeItem::new("kyiv", "Kyiv"),
        TreeItem::new("lviv", "Lviv"),
        TreeItem::new("odesa", "Odesa"),
        TreeItem::new("kharkiv", "Kharkiv"),
      ]),
      TreeItem::new("spain", "Spain").items([
        TreeItem::new("madrid", "Madrid"),
        TreeItem::new("barcelona", "Barcelona").disabled(true),
      ]),
    ]),
    TreeItem::new("americas", "Americas").items([
      TreeItem::new("peru", "Peru").item(TreeItem::new("lima", "Lima")),
      TreeItem::new("canada", "Canada").item(TreeItem::new("toronto", "Toronto")),
    ]),
  ]
}

/// The board's places: three continents, three countries of Europe, three cities of Ukraine.
fn board() -> Vec<TreeItem<Id>> {
  let country = |id: Id, label: &str, cities: &[(Id, &str)]| {
    TreeItem::new(id, label).items(cities.iter().map(|(id, label)| TreeItem::new(*id, label)))
  };
  vec![
    TreeItem::new("asia", "Asia").item(country("japan", "Japan", &[("tokyo", "Tokyo")])),
    TreeItem::new("europe", "Europe").items([
      country("poland", "Poland", &[("warsaw", "Warsaw")]),
      country(
        "ukraine",
        "Ukraine",
        &[("kyiv", "Kyiv"), ("lviv", "Lviv"), ("odesa", "Odesa")],
      ),
      country("spain", "Spain", &[("madrid", "Madrid")]),
    ]),
    TreeItem::new("americas", "Americas").item(country("peru", "Peru", &[("lima", "Lima")])),
  ]
}

const SIZES: [(&str, CascaderSize); 3] = [
  ("Sm", CascaderSize::Sm),
  ("Md", CascaderSize::Md),
  ("Lg", CascaderSize::Lg),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct CascaderPage {
  frame: Frame,
  playground: Playground,
  size: Signal<CascaderSize>,
  placeholder: Signal<String>,
  path: Signal<bool>,
  leaves_only: Signal<bool>,
  open_on_hover: Signal<bool>,
  clearable: Signal<bool>,
  full_width: Signal<bool>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  played: Signal<Option<Id>>,
  city: Signal<Option<Id>>,
  any: Signal<Option<Id>>,
  hovered: Signal<Option<Id>>,
  lazy: Signal<Option<Id>>,
  entry: Signal<Option<Id>>,
}

impl CascaderPage {
  /// The playground: the cascader as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (size, placeholder, path) = (self.size.get(), self.placeholder.get(), self.path.get());
    let (leaves_only, open_on_hover, clearable) =
      (self.leaves_only.get(), self.open_on_hover.get(), self.clearable.get());
    let (full_width, invalid, read_only, disabled) = (
      self.full_width.get(),
      self.invalid.get(),
      self.read_only.get(),
      self.disabled.get(),
    );
    let chosen = self.played.get();
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let dress = {
      let placeholder = placeholder.clone();
      move |cascader: Cascader<Id, Option<Id>>| {
        cascader
          .label("Place")
          .placeholder(&placeholder)
          .size(size)
          .path(path)
          .leaves_only(leaves_only)
          .open_on_hover(open_on_hover)
          .clearable(clearable)
          .full_width(full_width)
          .invalid(invalid)
          .read_only(read_only)
          .disabled(disabled)
          .force(forced)
      }
    };
    let build = {
      let (dress, set, record) = (dress.clone(), self.played.clone(), record.clone());
      move |ctx: &mut Ctx, chosen: Option<Id>, playground: Option<&Playground>| -> Element {
        let (set, changed, opened) = (set.clone(), record.clone(), record.clone());
        let cascader = dress(Cascader::controlled_optional(chosen, places(), move |next| {
          changed(format!("on_change({next:?})"));
          set.set(*next);
        }))
        .on_open_change(move |open| opened(format!("on_open_change({open})")));
        match playground {
          Some(playground) => playground.styled(cascader),
          None => cascader,
        }
        .mount(ctx)
      }
    };
    let shown = {
      let build = build.clone();
      let playground = &self.playground;
      Shown::built(move |ctx| build(ctx, chosen, Some(playground)))
    };
    {
      let (build, chosen) = (build.clone(), self.played.clone());
      self.playground.window_shows(move |ctx| build(ctx, chosen.get(), None));
    }
    let make = move || -> Element { dress(Cascader::controlled_optional(chosen, places(), |_| {})).into() };

    let mut code = vec![
      "// city: Signal<Option<&str>>".to_owned(),
      "Cascader::optional(city, places)".to_owned(),
      ".label(\"Place\")".to_owned(),
    ];
    if !placeholder.is_empty() {
      code.push(format!(".placeholder({placeholder:?})"));
    }
    if size != CascaderSize::Md {
      code.push(format!(".size(CascaderSize::{size:?})"));
    }
    for (on, call) in [
      (!path, ".path(false)"),
      (!leaves_only, ".leaves_only(false)"),
      (open_on_hover, ".open_on_hover(true)"),
      (clearable, ".clearable(true)"),
      (full_width, ".full_width(true)"),
      (invalid, ".invalid(true)"),
      (read_only, ".read_only(true)"),
      (disabled, ".disabled(true)"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push(".mount(ctx)".to_owned());

    let tree = vec![
      format!("trigger      {}   a select's box", self.playground.measured()),
      "├─ value     the path to what is chosen; an ellipsis where it is cut".to_owned(),
      "├─ clear     16   where it is clearable and something is chosen".to_owned(),
      "└─ chevron   16".to_owned(),
      "panel        4 under the trigger, as wide as its columns".to_owned(),
      "└─ columns   a column view; scrolls past 280".to_owned(),
      "   └─ column 124, padded 4: options of 30, in the highlight on the path".to_owned(),
    ];

    let flags = [
      self.open_on_hover.clone(),
      self.clearable.clone(),
      self.full_width.clone(),
      self.invalid.clone(),
      self.read_only.clone(),
      self.disabled.clone(),
    ];
    let (reset_size, reset_placeholder, reset_path, reset_leaves, reset_chosen) = (
      self.size.clone(),
      self.placeholder.clone(),
      self.path.clone(),
      self.leaves_only.clone(),
      self.played.clone(),
    );
    let set_size = self.size.clone();
    let controls = vec![
      prop(
        "size",
        "CascaderSize",
        crate::kit::segmented(&SIZES, &size, move |size| set_size.set(size)),
      ),
      prop("placeholder", "&str", field(&self.placeholder)),
      divider(),
      flag("path", &self.path),
      flag("leaves_only", &self.leaves_only),
      flag("open_on_hover", &self.open_on_hover),
      flag("clearable", &self.clearable),
      flag("full_width", &self.full_width),
      flag("invalid", &self.invalid),
      flag("read_only", &self.read_only),
      flag("disabled", &self.disabled),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Cascader",
        make: Arc::new(make),
        shown,
        fill: full_width,
        forceable: true,
        facts: format!("{chosen:?} chosen"),
        event: "on_change",
        code,
        tree,
        controls,
        changed: [
          size != CascaderSize::Md,
          placeholder != "Select a city",
          !path,
          !leaves_only,
          open_on_hover,
          clearable,
          full_width,
          invalid,
          read_only,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          for flag in &flags {
            flag.set(false);
          }
          reset_size.set(CascaderSize::Md);
          reset_placeholder.set("Select a city".to_owned());
          reset_path.set(true);
          reset_leaves.set(true);
          reset_chosen.set(Some("kyiv"));
        }),
      },
    )
  }
}

impl Component for CascaderPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      size: ctx.signal(CascaderSize::Md),
      placeholder: ctx.signal("Select a city".to_owned()),
      path: ctx.signal(true),
      leaves_only: ctx.signal(true),
      open_on_hover: ctx.signal(false),
      clearable: ctx.signal(false),
      full_width: ctx.signal(false),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      played: ctx.signal(Some("kyiv")),
      city: ctx.signal(Some("kyiv")),
      any: ctx.signal(Some("europe")),
      hovered: ctx.signal(None),
      lazy: ctx.signal(None),
      entry: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let open = Cascader::controlled("kyiv", board(), |_| {})
      .label("Place")
      .inline(true);
    let one = Cascader::optional(self.city.clone(), places())
      .label("Place")
      .placeholder("Select a city")
      .clearable(true)
      .key("one")
      .mount(ctx);
    let any = Cascader::optional(self.any.clone(), places())
      .label("Region")
      .placeholder("Select a region")
      .leaves_only(false)
      .key("any")
      .mount(ctx);
    let hovered = Cascader::optional(self.hovered.clone(), places())
      .label("Place")
      .placeholder("Select a city")
      .open_on_hover(true)
      .key("hover")
      .mount(ctx);
    let lazy = ctx.mount::<LazyCascader>(());
    let in_field = Field::new(
      "Office",
      Cascader::optional(self.entry.clone(), places())
        .placeholder("Select a city")
        .full_width(true),
    )
    .description("Where the team works from.")
    .required(true)
    .mount(ctx);
    let state = |cascader: Cascader<Id, Id>| cascader.style(CascaderPart::Trigger, |s| s.width(200.0));
    let plain = || Cascader::controlled("kyiv", places(), |_| {});
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(state(plain()), "at rest"))
      .child(item(
        state(plain().force(Some(Interaction::Hovered))),
        "under the pointer",
      ))
      .child(item(
        state(plain().force(Some(Interaction::FocusVisible))),
        "focused from the keyboard",
      ))
      .child(item(state(plain().invalid(true)), "invalid"))
      .child(item(state(plain().read_only(true)), "read only"))
      .child(item(state(plain().disabled(true)), "disabled"));

    let rows = [
      (
        "value",
        "Signal<T>",
        "",
        "What is chosen, the first argument of `new`; `optional` takes a `Signal<Option<T>>`. `controlled` and \
         `controlled_optional` are the same held by the app.",
      ),
      (
        "items",
        "impl IntoIterator<Item = TreeItem<T>>",
        "",
        "The tree: a tree view's items. A value is one item's.",
      ),
      (
        "on_change",
        "impl Fn(&T) / Fn(&Option<T>)",
        "",
        "Told what is chosen after a choice a person makes, after the signal is set.",
      ),
      (
        "leaves_only",
        "bool",
        "true",
        "Only an option that holds nothing is chosen. Without it a press or Enter chooses any, and the panel stays \
         open on the column of one that holds others.",
      ),
      (
        "open_on_hover",
        "bool",
        "false",
        "The pointer on an option that holds others opens its column, as a submenu opens.",
      ),
      (
        "on_load",
        "Fn(&T)",
        "",
        "Told an item marked `lazy` that holds nothing yet as its column opens: where the app brings its children.",
      ),
      (
        "open",
        "Signal<bool>",
        "kept where mounted",
        "Whether the panel is open. `on_open_change(bool)` hears each change a person makes, `on_dismiss()` Escape \
         and a press outside.",
      ),
      (
        "path / separator_text",
        "bool / &str",
        "true / \" / \"",
        "The trigger says the whole path to what is chosen, or its label alone; and what stands between the steps.",
      ),
      (
        "placeholder",
        "&str",
        "",
        "What the trigger says while nothing is chosen.",
      ),
      ("size", "CascaderSize", "Md", "Sm 32, Md 36, Lg 40: a select's."),
      (
        "clearable",
        "bool",
        "false",
        "A cross before the chevron that takes what is chosen away; `clear_label` names it.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The judgment of what is chosen, in the trigger's edge.",
      ),
      (
        "read_only / disabled / full_width",
        "bool",
        "false",
        "The panel does not open; a disabled one takes no focus either; as wide as what holds it.",
      ),
      (
        "label / panel_label / empty_text / loading_label",
        "&str",
        "",
        "What it, its panel and the spinner of a column on its way are called; what an empty panel says.",
      ),
      (
        "placement / offset / collision / inline",
        "",
        "",
        "Where the panel stands, as a popover's; `dismiss_on_escape` and `dismiss_on_outside_press` too.",
      ),
      (
        "style / rules",
        "CascaderPart",
        "",
        "Trigger, Value, Placeholder, Chevron, Clear, ClearIcon, Panel, Columns, Column, Option, OptionIcon, \
         OptionLabel, OptionChevron, Empty.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Cascader"],
        title: "Cascader",
        description: "A select whose options are a tree shown as columns that open one after the other: a place \
                      by its continent, its country and its city.",
        import: Some("use lurq::ui::{Cascader, TreeItem};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Cascader is a control on the right. The cascader on the stage is live: a press opens its \
           columns, a press of a country opens its cities, and a city chooses and closes.",
          playground,
        ),
        Section::new(
          "Open",
          "The board's cascader with its panel drawn in place: the trigger says the path to what is chosen, and the \
           columns of that path are open, the options on it in the highlight.",
          self.frame.demo(
            0,
            open,
            "Cascader::controlled(value, items, on_change).inline(true)",
            &[
              "Cascader::controlled(\"kyiv\", [",
              "    TreeItem::new(\"asia\", \"Asia\").items(..),",
              "    TreeItem::new(\"europe\", \"Europe\").items([",
              "        TreeItem::new(\"poland\", \"Poland\").items(..),",
              "        TreeItem::new(\"ukraine\", \"Ukraine\").items([",
              "            TreeItem::new(\"kyiv\", \"Kyiv\"),",
              "            TreeItem::new(\"lviv\", \"Lviv\"),",
              "            TreeItem::new(\"odesa\", \"Odesa\"),",
              "        ]),",
              "        ..",
              "    ]),",
              "    ..",
              "], |city| { /* the app's city */ })",
              ".label(\"Place\")",
              ".inline(true)",
            ],
          ),
        ),
        Section::new(
          "One or none",
          "`Cascader::optional` with a `Signal<Option<&str>>`, mounted so that it keeps whether it is open and the \
           columns open in it. A press, Enter, Space or Down opens it on what is chosen; Up and Down go through a \
           column, Right into an option, Left back, Enter on a city chooses it; Escape closes it. The cross clears.",
          self.frame.demo(
            1,
            one,
            "Cascader::optional(city, items).clearable(true).mount(ctx)",
            &[
              "// city: Signal<Option<&str>>",
              "Cascader::optional(city, items)",
              "    .label(\"Place\")",
              "    .placeholder(\"Select a city\")",
              "    .clearable(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Any level",
          "With `leaves_only(false)` a continent or a country is chosen too: a press of it chooses it and opens its \
           column, and the panel stays open for a finer choice.",
          self.frame.demo(
            2,
            any,
            ".leaves_only(false)",
            &[
              "Cascader::optional(region, items)",
              "    .label(\"Region\")",
              "    .leaves_only(false)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Opened under the pointer",
          "With `open_on_hover(true)` the pointer on an option that holds others opens its column, as a submenu \
           opens; a press still does.",
          self.frame.demo(
            3,
            hovered,
            ".open_on_hover(true)",
            &[
              "Cascader::optional(city, items)",
              "    .open_on_hover(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Children on demand",
          "An item marked `lazy` with no children yet opens a column with a spinner, and `on_load` is told it; here \
           the app gives the cities a second later.",
          self.frame.demo(
            4,
            lazy,
            "TreeItem::new(..).lazy(true) + on_load",
            &[
              "// cities: Signal<Option<Vec<TreeItem<&str>>>>",
              "let later = TreeItem::new(\"later\", \"Brought later\").lazy(true);",
              "let later = match cities.get() { Some(cities) => later.items(cities), None => later };",
              "Cascader::optional(city, [.., later])",
              "    .on_load(move |_| bring_the_cities(cities.clone()))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the cascader, judges it and disables it, and a mounted field mounts it.",
          self.frame.demo(
            5,
            Column::new().width(320.0).max_width(FILL).child(in_field),
            "Field::new(label, Cascader::optional(..)).mount(ctx)",
            &[
              "Field::new(\"Office\", Cascader::optional(office, items).full_width(true))",
              "    .description(\"Where the team works from.\")",
              "    .required(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The trigger is a select's: its edge takes the ring's color under the pointer, the focus is a line of 2 \
           inside it, a judgment colors its edge, a read only one is on the muted fill and a disabled one faded.",
          self.frame.demo(
            6,
            states,
            ".invalid(true) / .read_only(true) / .disabled(true)",
            &[
              "Cascader::controlled(\"kyiv\", items, |_| {}).invalid(true)",
              "Cascader::controlled(\"kyiv\", items, |_| {}).read_only(true)",
              "Cascader::controlled(\"kyiv\", items, |_| {}).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a cascader is given.",
          props_table(&rows, &["value", "items"], self.frame.room()),
        ),
      ],
    )
  }
}

/// A cascader whose last continent's countries come a second after its column opens.
struct LazyCascader {
  city: Signal<Option<Id>>,
  countries: Signal<bool>,
  load: Timeout,
}

impl Component for LazyCascader {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let countries = ctx.signal(false);
    let set = countries.clone();
    Self {
      city: ctx.signal(None),
      countries,
      load: ctx.create_timeout(std::time::Duration::from_secs(1), move || set.set(true)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut later = TreeItem::new("oceania", "Oceania").lazy(true);
    if self.countries.get() {
      later = later.items([TreeItem::new("sydney", "Sydney"), TreeItem::new("auckland", "Auckland")]);
    }
    let load = self.load.clone();
    Cascader::optional(self.city.clone(), places().into_iter().chain([later]))
      .label("Place")
      .placeholder("Select a city")
      .on_load(move |_| load.restart())
      .mount(ctx)
  }
}
