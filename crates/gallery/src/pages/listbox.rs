//! The page of the listbox: its playground, the board's renderers, many chosen, a choice that waits for Space, a
//! long list the page keys move through, thousands of options in a virtual list, and the states of an option.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::Density},
  components::{Column, Row, SelectOption},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Interaction, Listbox, ListboxPart},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

type Id = &'static str;

/// The densities, as the code names them.
const DENSITIES: [(&str, Density); 4] = [
  ("compact", Density::Compact),
  ("default", Density::Default),
  ("comfortable", Density::Comfortable),
  ("touch", Density::Touch),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// The board's renderers, with OpenGL disabled where `disabled`, under labels where `groups`, with details where
/// `details`.
fn renderers(groups: bool, disabled: bool, details: bool) -> Vec<SelectOption<Id>> {
  let option = |value: Id, label: &str, detail: &str| {
    let option = SelectOption::new(value, label);
    if details { option.detail(detail) } else { option }
  };
  let mut first = option("wgpu", "wgpu", "Every platform");
  let mut native = option("dx12", "DirectX 12", "Windows");
  if groups {
    first = first.group("Renderers");
    native = native.group("Native");
  }
  vec![
    first,
    option("gl", "OpenGL (legacy)", "Every platform").disabled(disabled),
    native,
    option("metal", "Metal", "macOS and iOS"),
    option("vulkan", "Vulkan", "Windows, Linux and Android"),
  ]
}

pub struct ListboxPage {
  frame: Frame,
  playground: Playground,
  multiple: Signal<bool>,
  follow: Signal<bool>,
  groups: Signal<bool>,
  details: Signal<bool>,
  option_disabled: Signal<bool>,
  disabled: Signal<bool>,
  density: Signal<usize>,
  played_one: Signal<Id>,
  played_many: Signal<Vec<Id>>,
  renderer: Signal<Id>,
  columns: Signal<Vec<Id>>,
  waiting: Signal<Id>,
  zone: Signal<usize>,
}

/// The time zones of the long list.
const ZONES: [&str; 24] = [
  "Pacific/Honolulu",
  "America/Anchorage",
  "America/Los_Angeles",
  "America/Denver",
  "America/Chicago",
  "America/New_York",
  "America/Halifax",
  "America/Sao_Paulo",
  "Atlantic/Azores",
  "Europe/London",
  "Europe/Lisbon",
  "Europe/Paris",
  "Europe/Berlin",
  "Europe/Kyiv",
  "Europe/Istanbul",
  "Asia/Dubai",
  "Asia/Karachi",
  "Asia/Kolkata",
  "Asia/Dhaka",
  "Asia/Bangkok",
  "Asia/Shanghai",
  "Asia/Tokyo",
  "Australia/Sydney",
  "Pacific/Auckland",
];

impl ListboxPage {
  /// The playground: the renderers as the panel's props make the listbox.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (multiple, follow) = (self.multiple.get(), self.follow.get());
    let (groups, details, option_disabled, disabled) = (
      self.groups.get(),
      self.details.get(),
      self.option_disabled.get(),
      self.disabled.get(),
    );
    let (density_name, density) = DENSITIES[self.density.get().min(DENSITIES.len() - 1)];
    let record = self.playground.recorder();
    let options = move || renderers(groups, option_disabled, details);
    // The props the panel gives either form: of one, `Listbox<Id>`, or of any number, `Listbox<Id, [Id]>`.
    fn dress<V: ?Sized + 'static>(
      listbox: Listbox<Id, V>,
      follow: bool,
      disabled: bool,
      density: Density,
    ) -> Listbox<Id, V> {
      listbox
        .label("Renderer")
        .follow_focus(follow)
        .disabled(disabled)
        .density(density)
    }
    let shown: Element = if multiple {
      dress(
        Listbox::multiple(self.played_many.clone(), options()),
        follow,
        disabled,
        density,
      )
      .on_change(move |next: &[Id]| record(format!("on_change({next:?})")))
      .force(self.playground.forced())
      .into()
    } else {
      dress(
        Listbox::new(self.played_one.clone(), options()),
        follow,
        disabled,
        density,
      )
      .on_change(move |next: &Id| record(format!("on_change({next:?})")))
      .force(self.playground.forced())
      .into()
    };
    let make = {
      let (one, many) = (self.played_one.get(), self.played_many.get());
      move || -> Element {
        if multiple {
          dress(
            Listbox::multiple(Signal::new(many.clone()), options()),
            follow,
            disabled,
            density,
          )
          .into()
        } else {
          dress(Listbox::new(Signal::new(one), options()), follow, disabled, density).into()
        }
      }
    };

    let mut code = vec![if multiple {
      "// chosen: Signal<Vec<&str>>".to_owned()
    } else {
      "// chosen: Signal<&str>".to_owned()
    }];
    code.push(
      if multiple {
        "Listbox::multiple(chosen, ["
      } else {
        "Listbox::new(chosen, ["
      }
      .to_owned(),
    );
    let detail = |text: &str| {
      if details {
        format!(".detail(\"{text}\")")
      } else {
        String::new()
      }
    };
    code.push(format!(
      "    SelectOption::new(\"wgpu\", \"wgpu\"){}{},",
      if groups { ".group(\"Renderers\")" } else { "" },
      detail("Every platform")
    ));
    code.push(format!(
      "    SelectOption::new(\"gl\", \"OpenGL (legacy)\"){}{},",
      detail("Every platform"),
      if option_disabled { ".disabled(true)" } else { "" }
    ));
    code.push(format!(
      "    SelectOption::new(\"dx12\", \"DirectX 12\"){}{},",
      if groups { ".group(\"Native\")" } else { "" },
      detail("Windows")
    ));
    code.push("    ..".to_owned());
    code.push("])".to_owned());
    code.push(".label(\"Renderer\")".to_owned());
    if !follow && !multiple {
      code.push(".follow_focus(false)".to_owned());
    }
    if density != Density::Default {
      code.push(format!(".density(Density::{density:?})"));
    }
    if disabled {
      code.push(".disabled(true)".to_owned());
    }
    code.push(
      if multiple {
        ".on_change(|chosen: &[&str]| /* use it */)"
      } else {
        ".on_change(|chosen: &&str| /* use it */)"
      }
      .to_owned(),
    );

    let tree = vec![
      format!(
        "root        {}   220, an edge of 1, radius lg",
        self.playground.measured()
      ),
      "└─ content  4 in, options 1 apart; scrolls past the root's max height".to_owned(),
      format!("   ├─ group    11 at 600, muted{}", if groups { "" } else { "   none" }),
      format!("   └─ option   at {density_name}, 8 in, radius sm"),
      format!(
        "      ├─ label    13, 500 while chosen{}",
        if details { ", its detail under it" } else { "" }
      ),
      "      └─ check    14 at the end of a chosen option".to_owned(),
    ];

    let mut controls = vec![
      prop("density", "Density", {
        let choose = self.density.clone();
        let densities: Vec<(&str, usize)> = DENSITIES
          .iter()
          .enumerate()
          .map(|(index, (name, _))| (*name, index))
          .collect();
        segmented_fill(&densities, &self.density.get(), move |value| choose.set(value))
      }),
      divider(),
      flag("multiple (Listbox::multiple)", &self.multiple),
    ];
    if !multiple {
      controls.push(flag("follow_focus", &self.follow));
    }
    controls.extend([
      flag("groups", &self.groups),
      flag("details", &self.details),
      flag("a disabled option", &self.option_disabled),
      flag("disabled", &self.disabled),
    ]);
    let resets = (
      self.multiple.clone(),
      self.follow.clone(),
      self.groups.clone(),
      self.details.clone(),
      self.option_disabled.clone(),
      self.disabled.clone(),
      self.density.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Listbox",
        make: Arc::new(make),
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: if multiple {
          format!("{} chosen", self.played_many.get().len())
        } else {
          format!("{:?} chosen", self.played_one.get())
        },
        event: "on_change",
        code,
        tree,
        controls,
        changed: [
          multiple,
          !follow && !multiple,
          !groups,
          details,
          !option_disabled,
          disabled,
          density != Density::Default,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          resets.0.set(false);
          resets.1.set(true);
          resets.2.set(true);
          resets.3.set(false);
          resets.4.set(true);
          resets.5.set(false);
          resets.6.set(1);
        }),
      },
    )
  }
}

impl Component for ListboxPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      multiple: ctx.signal(false),
      follow: ctx.signal(true),
      groups: ctx.signal(true),
      details: ctx.signal(false),
      option_disabled: ctx.signal(true),
      disabled: ctx.signal(false),
      density: ctx.signal(1),
      played_one: ctx.signal("wgpu"),
      played_many: ctx.signal(vec!["wgpu", "vulkan"]),
      renderer: ctx.signal("wgpu"),
      columns: ctx.signal(vec!["name", "size"]),
      waiting: ctx.signal("dx12"),
      zone: ctx.signal(13),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Listbox::new(
      self.renderer.clone(),
      [
        SelectOption::new("wgpu", "wgpu").group("Renderers"),
        SelectOption::new("dx12", "DirectX 12"),
        SelectOption::new("metal", "Metal"),
        SelectOption::new("gl", "OpenGL (legacy)").disabled(true),
      ],
    )
    .label("Renderer");
    let columns = Listbox::multiple(
      self.columns.clone(),
      [
        ("name", "Name"),
        ("size", "Size"),
        ("kind", "Kind"),
        ("modified", "Modified"),
        ("owner", "Owner"),
      ],
    )
    .label("Columns");
    let waiting = Listbox::new(
      self.waiting.clone(),
      [
        ("wgpu", "wgpu"),
        ("dx12", "DirectX 12"),
        ("metal", "Metal"),
        ("vulkan", "Vulkan"),
      ],
    )
    .label("Renderer")
    .follow_focus(false);
    let zones = Listbox::new(
      self.zone.clone(),
      ZONES.iter().enumerate().map(|(index, zone)| (index, *zone)),
    )
    .label("Time zone")
    .style(ListboxPart::Root, |s| s.width(260.0).max_height(240.0));

    let thousands = ctx.mount::<BigListbox>(());

    let state = |forced: Option<Interaction>, disabled: bool| {
      Listbox::new(
        Signal::new("wgpu"),
        [
          SelectOption::new("wgpu", "wgpu"),
          SelectOption::new("dx12", "DirectX 12"),
          SelectOption::new("gl", "OpenGL (legacy)").disabled(true),
        ],
      )
      .disabled(disabled)
      .force(forced)
      .style(ListboxPart::Root, |s| s.width(180.0))
    };
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(state(None, false), "at rest: chosen, a disabled option"))
      .child(item(state(Some(Interaction::Hovered), false), "under the pointer"))
      .child(item(
        state(Some(Interaction::FocusVisible), false),
        "focused from the keyboard",
      ))
      .child(item(state(None, true), "disabled"));

    let rows = [
      (
        "value",
        "Signal<T>",
        "",
        "What the listbox is bound to: the option of its value is chosen, and choosing another sets it. \
         Listbox::optional binds a Signal<Option<T>>, none chosen while it is None; Listbox::multiple a \
         Signal<Vec<T>>, any number chosen.",
      ),
      (
        "controlled",
        "(T, options, Fn(&T))",
        "",
        "Listbox::controlled: the app holds the value, and is told what a press or a key would choose. \
         controlled_optional takes an Option<T> and Fn(&Option<T>), controlled_multiple the values and Fn(&[T]).",
      ),
      (
        "options",
        "impl IntoIterator<Item = impl Into<SelectOption<T>>>",
        "",
        "What a select takes: `(value, label)`, a `Choice`, or a `SelectOption` with `group`, `detail`, `hint`, \
         `leading`, `trailing`, `disabled`.",
      ),
      (
        "follow_focus",
        "bool",
        "true",
        "Of one, whether the arrows choose as they move. Without it Space and Enter choose.",
      ),
      (
        "on_change",
        "Fn(&T)",
        "",
        "Told the new choice after each press or key that changes it, and not for what is already chosen: the \
         value, the Option of an optional one, the whole set of a multiple one. A second is called after the first.",
      ),
      ("label", "&str", "", "What the listbox is called, to tooling."),
      (
        "invalid / disabled / full_width",
        "bool",
        "false",
        "Its edge in the destructive color; no option chosen; as wide as what holds it.",
      ),
      (
        "density",
        "Density",
        "the theme's",
        "How high an option is: 24, 32, 40 or 44.",
      ),
      (
        "style / rules",
        "ListboxPart",
        "",
        "Root, Content, Group, Option, Leading, Label, Detail, Hint, Trailing, Check. A `max_height` on Root \
         makes the options scroll.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "For thousands of options: in a virtual list, which builds only those near the view. Give Root a height.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Listbox"],
        title: "Listbox",
        description: "A list in the page of which one option is chosen, or any number: a renderer, a time zone, \
                      the columns a table shows.",
        import: Some("use lurq::ui::Listbox;"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Listbox is a control on the right. The listbox on the stage is live: a press and the keys \
           choose, and each change is counted.",
          playground,
        ),
        Section::new(
          "One chosen",
          "The board's listbox. Tab reaches the chosen option; Up and Down move and choose, as in a radio group, \
           Home and End go to the ends, a letter to the next option that starts with it, and the disabled option \
           is passed by.",
          self.frame.demo(
            0,
            board,
            "Listbox::new(renderer, options)",
            &[
              "// renderer: Signal<&str>",
              "Listbox::new(renderer, [",
              "    SelectOption::new(\"wgpu\", \"wgpu\").group(\"Renderers\"),",
              "    SelectOption::new(\"dx12\", \"DirectX 12\"),",
              "    SelectOption::new(\"metal\", \"Metal\"),",
              "    SelectOption::new(\"gl\", \"OpenGL (legacy)\").disabled(true),",
              "])",
              ".label(\"Renderer\")",
            ],
          ),
        ),
        Section::new(
          "Many chosen",
          "Listbox::multiple, with a `Signal<Vec<_>>`. A press chooses one alone, with Ctrl it adds or takes out one, with Shift it \
           chooses from the last chosen to it. Space adds or takes out the focused option, Shift with the arrows \
           chooses on the way, and Ctrl+A chooses them all.",
          self.frame.demo(
            1,
            columns,
            "Listbox::multiple(columns, options)",
            &[
              "// columns: Signal<Vec<&str>>",
              "Listbox::multiple(columns, [",
              "    (\"name\", \"Name\"),",
              "    (\"size\", \"Size\"),",
              "    (\"kind\", \"Kind\"),",
              "    (\"modified\", \"Modified\"),",
              "    (\"owner\", \"Owner\"),",
              "])",
              ".label(\"Columns\")",
            ],
          ),
        ),
        Section::new(
          "A choice that waits",
          "Without `follow_focus` the arrows move the focus alone, and Space, Enter or a press choose: for a \
           choice that costs something as it is made.",
          self.frame.demo(
            2,
            waiting,
            ".follow_focus(false)",
            &[
              "Listbox::new(renderer, [(\"wgpu\", \"wgpu\"), (\"dx12\", \"DirectX 12\"), ..])",
              "    .label(\"Renderer\")",
              "    .follow_focus(false)",
            ],
          ),
        ),
        Section::new(
          "A long list",
          "A box with a most height scrolls its options. PageUp and PageDown move by what it shows, and the focused \
           option is kept in view.",
          self.frame.demo(
            3,
            zones,
            "style(ListboxPart::Root, |s| s.max_height(240.0))",
            &[
              "// zone: Signal<usize>",
              "Listbox::new(zone, ZONES.iter().enumerate().map(|(index, zone)| (index, *zone)))",
              "    .label(\"Time zone\")",
              "    .style(ListboxPart::Root, |s| s.width(260.0).max_height(240.0))",
            ],
          ),
        ),
        Section::new(
          "Thousands of options",
          "Ten thousand options in groups of a thousand: `mount(ctx)` puts them in the engine's virtual list, which \
           builds those near the view and opens at the chosen one. The keys reach every option, built or not, and \
           the choice follows them: End goes to the last, a letter to the next option it starts, PageDown a view \
           down.",
          self.frame.demo(
            5,
            thousands,
            "Listbox::new(value, options).mount(ctx)",
            &[
              "// item: Signal<u32>",
              "Listbox::new(item, (0..10_000u32).map(|i| {",
              "    let option = SelectOption::new(i, format!(\"Item {i}\"));",
              "    if i % 1000 == 0 { option.group(format!(\"From {i}\")) } else { option }",
              "}))",
              ".label(\"Items\")",
              ".style(ListboxPart::Root, |s| s.width(260.0).height(320.0))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws the chosen option, one under the pointer and a disabled one. One held down is veiled, \
           the focus from the keyboard is a ring inside the option, and a disabled listbox is dimmed.",
          self.frame.demo(
            4,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "Listbox::new(chosen, [",
              "    SelectOption::new(\"wgpu\", \"wgpu\"),",
              "    SelectOption::new(\"dx12\", \"DirectX 12\"),",
              "    SelectOption::new(\"gl\", \"OpenGL (legacy)\").disabled(true),",
              "])",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              ".force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a listbox is given.",
          props_table(&rows, &["value", "options"], self.frame.room()),
        ),
      ],
    )
  }
}

/// Ten thousand options, in a component of its own so that the page's other signals do not build them again.
struct BigListbox {
  item: Signal<u32>,
}

impl Component for BigListbox {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { item: ctx.signal(4321) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Listbox::new(
      self.item.clone(),
      (0..10_000u32).map(|i| {
        let option = SelectOption::new(i, format!("Item {i}"));
        if i % 1000 == 0 {
          option.group(format!("From {i}"))
        } else {
          option
        }
      }),
    )
    .label("Items")
    .style(ListboxPart::Root, |s| s.width(260.0).height(320.0))
    .mount(ctx)
  }
}
