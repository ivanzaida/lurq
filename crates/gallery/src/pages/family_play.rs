//! The playgrounds of the button family's pages: for each component, what a
//! page keeps of its props, and the playground they make.

use std::sync::Arc;

use lurq::{
  app::ctx::Ctx,
  core::Signal,
  node::Element,
  ui::{
    Button, ButtonGroup, ButtonGroupVariant, ButtonSize, Choice, Link, SegmentedControl, Toggle, ToggleGroup, icons,
  },
};

use crate::{
  kit::segmented_fill,
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// A size: what it is, what the code calls it, and what the panel calls it.
const SIZES: [(ButtonSize, &str, &str); 5] = [
  (ButtonSize::Xs, "Xs", "xs"),
  (ButtonSize::Sm, "Sm", "sm"),
  (ButtonSize::Md, "Md", "md"),
  (ButtonSize::Lg, "Lg", "lg"),
  (ButtonSize::Xl, "Xl", "xl"),
];
/// The size a component has unless it says otherwise.
const MD: usize = 2;

/// The control that picks a size.
fn size_control(size: &Signal<usize>) -> Element {
  let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.2, index)).collect();
  let set = size.clone();
  prop(
    "size",
    "ButtonSize",
    segmented_fill(&sizes, &size.get(), move |index| set.set(index)),
  )
}

/// The line of code that sets a size, if it is not the one a component has anyway.
fn size_line(size: usize) -> Option<String> {
  (size != MD).then(|| format!("    .size(ButtonSize::{})", SIZES[size].1))
}

/// How many of `changed` are true.
fn count(changed: &[bool]) -> usize {
  changed.iter().filter(|changed| **changed).count()
}

// --- Toggle ---------------------------------------------------------------

const TOGGLE_ICONS: [(&str, Option<&str>, &str); 4] = [
  ("None", None, ""),
  ("Bold", Some(icons::BOLD), "BOLD"),
  ("Pin", Some(icons::PIN), "PIN"),
  ("Star", Some(icons::STAR), "STAR"),
];

pub struct TogglePlay {
  playground: Playground,
  label: Signal<String>,
  icon: Signal<usize>,
  size: Signal<usize>,
  on: Signal<bool>,
  icon_only: Signal<bool>,
  disabled: Signal<bool>,
}

impl TogglePlay {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      playground: Playground::new(ctx, 1),
      label: ctx.signal("Bold".to_owned()),
      icon: ctx.signal(1),
      size: ctx.signal(MD),
      on: ctx.signal(false),
      icon_only: ctx.signal(false),
      disabled: ctx.signal(false),
    }
  }

  pub fn card(&self, ctx: &mut Ctx) -> Element {
    let (label, icon, size) = (self.label.get(), self.icon.get(), self.size.get());
    let (on, icon_only, disabled) = (self.on.get(), self.icon_only.get(), self.disabled.get());
    let (record, set) = (self.playground.recorder(), self.on.clone());
    let plain = {
      let label = label.clone();
      move || {
        let toggle = Toggle::new(&label)
          .size(SIZES[size].0)
          .on(on)
          .icon_only(icon_only)
          .disabled(disabled);
        match TOGGLE_ICONS[icon].1 {
          Some(name) => toggle.icon(name),
          None => toggle,
        }
      }
    };
    let toggle = plain().force(self.playground.forced()).on_change(move |next| {
      set.set(next);
      record(format!("on_change({next})"));
    });

    let mut code = vec![format!("Toggle::new({label:?})")];
    if TOGGLE_ICONS[icon].1.is_some() {
      code.push(format!("    .icon(icons::{})", TOGGLE_ICONS[icon].2));
    }
    code.extend(size_line(size));
    if icon_only {
      code.push("    .icon_only(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on(bold.get())".to_owned());
    code.push("    .on_change(move |on| bold.set(on))".to_owned());

    let mut tree = vec![format!(
      "root      {}   {}",
      self.playground.measured(),
      if on { "on" } else { "off" }
    )];
    if TOGGLE_ICONS[icon].1.is_some() {
      tree.push(format!("{} icon", if icon_only { "└─" } else { "├─" }));
    }
    if !icon_only {
      tree.push(format!("└─ label     {label:?}"));
    }

    let glyphs: Vec<(&str, usize)> = TOGGLE_ICONS
      .iter()
      .enumerate()
      .map(|(index, glyph)| (glyph.0, index))
      .collect();
    let pick = self.icon.clone();
    let (to_label, to_icon, to_size) = (self.label.clone(), self.icon.clone(), self.size.clone());
    let flags = [self.on.clone(), self.icon_only.clone(), self.disabled.clone()];
    self.playground.card(
      ctx,
      Spec {
        title: "Toggle",
        make: Arc::new(move || plain().into()),
        shown: toggle.into(),
        fill: false,
        forceable: true,
        facts: if on { "on".to_owned() } else { "off".to_owned() },
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop(
            "icon",
            "impl Into<Name>",
            self.playground.select(0, &glyphs, &icon, move |index| pick.set(index)),
          ),
          size_control(&self.size),
          divider(),
          flag("on", &self.on),
          flag("icon_only", &self.icon_only),
          flag("disabled", &self.disabled),
        ],
        changed: count(&[icon != 0, size != MD, on, icon_only, disabled]),
        reset: Arc::new(move || {
          to_label.set("Bold".to_owned());
          to_icon.set(1);
          to_size.set(MD);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

// --- Link -----------------------------------------------------------------

const LINK_ICONS: [(&str, Option<&str>, &str); 3] = [
  ("None", None, ""),
  ("External", Some(icons::ARROW_UP_RIGHT), "    .external()"),
  ("Arrow right", Some(icons::ARROW_RIGHT), "    .icon(icons::ARROW_RIGHT)"),
];

pub struct LinkPlay {
  playground: Playground,
  label: Signal<String>,
  icon: Signal<usize>,
  disabled: Signal<bool>,
}

impl LinkPlay {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      playground: Playground::new(ctx, 1),
      label: ctx.signal("Read the docs".to_owned()),
      icon: ctx.signal(1),
      disabled: ctx.signal(false),
    }
  }

  pub fn card(&self, ctx: &mut Ctx) -> Element {
    let (label, icon, disabled) = (self.label.get(), self.icon.get(), self.disabled.get());
    let record = self.playground.recorder();
    let plain = {
      let label = label.clone();
      move || {
        let link = Link::new(&label).disabled(disabled);
        match LINK_ICONS[icon].1 {
          Some(name) => link.icon(name),
          None => link,
        }
      }
    };
    let link = plain()
      .force(self.playground.forced())
      .on_click(move |_| record("on_click".to_owned()));
    let mut code = vec![format!("Link::new({label:?})")];
    if LINK_ICONS[icon].1.is_some() {
      code.push(LINK_ICONS[icon].2.to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .on_click(move |_| navigator.push(\"/docs\"))".to_owned());
    let mut tree = vec![
      format!("root      {}   gap 4", self.playground.measured()),
      format!(
        "{} label     {label:?}   14/500",
        if LINK_ICONS[icon].1.is_some() {
          "├─"
        } else {
          "└─"
        }
      ),
    ];
    if LINK_ICONS[icon].1.is_some() {
      tree.push("└─ icon      14 × 14".to_owned());
    }
    let glyphs: Vec<(&str, usize)> = LINK_ICONS
      .iter()
      .enumerate()
      .map(|(index, glyph)| (glyph.0, index))
      .collect();
    let pick = self.icon.clone();
    let (to_label, to_icon, to_disabled) = (self.label.clone(), self.icon.clone(), self.disabled.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Link",
        make: Arc::new(move || plain().into()),
        shown: link.into(),
        fill: false,
        forceable: true,
        facts: String::new(),
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop(
            "icon",
            "impl Into<Name>",
            self.playground.select(0, &glyphs, &icon, move |index| pick.set(index)),
          ),
          divider(),
          flag("disabled", &self.disabled),
        ],
        changed: count(&[icon != 0, disabled]),
        reset: Arc::new(move || {
          to_label.set("Read the docs".to_owned());
          to_icon.set(1);
          to_disabled.set(false);
        }),
      },
    )
  }
}

// --- Segmented control ----------------------------------------------------

const VIEWS: [(&str, &str, &str); 3] = [
  ("List", icons::LIST, "LIST"),
  ("Table", icons::TABLE, "TABLE"),
  ("Board", icons::DASHBOARD, "DASHBOARD"),
];

pub struct SegmentedPlay {
  playground: Playground,
  chosen: Signal<usize>,
  size: Signal<usize>,
  with_icons: Signal<bool>,
  icon_only: Signal<bool>,
  full_width: Signal<bool>,
  disabled: Signal<bool>,
}

impl SegmentedPlay {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      playground: Playground::new(ctx, 0),
      chosen: ctx.signal(0),
      size: ctx.signal(MD),
      with_icons: ctx.signal(false),
      icon_only: ctx.signal(false),
      full_width: ctx.signal(false),
      disabled: ctx.signal(false),
    }
  }

  pub fn card(&self, ctx: &mut Ctx) -> Element {
    let (chosen, size) = (self.chosen.get().min(VIEWS.len() - 1), self.size.get());
    let (with_icons, icon_only) = (self.with_icons.get() || self.icon_only.get(), self.icon_only.get());
    let (full_width, disabled) = (self.full_width.get(), self.disabled.get());
    let (record, set) = (self.playground.recorder(), self.chosen.clone());
    let choices = VIEWS.iter().enumerate().map(|(index, (label, glyph, _))| {
      let choice = Choice::new(index, label);
      if with_icons {
        choice.icon(*glyph).icon_only(icon_only)
      } else {
        choice
      }
    });
    let control = SegmentedControl::controlled(chosen, choices, move |index: usize| {
      set.set(index);
      record(format!("chosen: {}", VIEWS[index].0));
    })
    .size(SIZES[size].0)
    .full_width(full_width)
    .disabled(disabled)
    .force(self.playground.forced());

    let mut code = vec!["SegmentedControl::new(view, [".to_owned()];
    for (label, _, constant) in VIEWS {
      if with_icons {
        // A choice with an icon is two lines: it would not fit the card on one.
        code.push(format!("    Choice::new(View::{label}, {label:?})"));
        code.push(format!(
          "        .icon(icons::{constant}){},",
          if icon_only { ".icon_only(true)" } else { "" }
        ));
      } else {
        code.push(format!("    (View::{label}, {label:?}),"));
      }
    }
    code.push("])".to_owned());
    code.extend(size_line(size));
    if full_width {
      code.push("    .full_width(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    let mut tree = vec![format!("root      {}   padding 3   gap 2", self.playground.measured())];
    for (index, (label, ..)) in VIEWS.iter().enumerate() {
      tree.push(format!(
        "{} segment   {label:?}{}",
        if index + 1 == VIEWS.len() { "└─" } else { "├─" },
        if index == chosen { "   selected" } else { "" }
      ));
    }
    let (to_chosen, to_size) = (self.chosen.clone(), self.size.clone());
    let flags = [
      self.with_icons.clone(),
      self.icon_only.clone(),
      self.full_width.clone(),
      self.disabled.clone(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Segmented Control",
        make: Arc::new(move || {
          let choices = VIEWS.iter().enumerate().map(|(index, (label, glyph, _))| {
            let choice = Choice::new(index, label);
            if with_icons {
              choice.icon(*glyph).icon_only(icon_only)
            } else {
              choice
            }
          });
          SegmentedControl::controlled(chosen, choices, |_: usize| {})
            .size(SIZES[size].0)
            .full_width(full_width)
            .disabled(disabled)
            .into()
        }),
        shown: control.into(),
        fill: full_width,
        forceable: true,
        facts: format!("chosen: {}", VIEWS[chosen].0),
        event: "chosen",
        code,
        tree,
        controls: vec![
          size_control(&self.size),
          divider(),
          flag("icons", &self.with_icons),
          flag("icon_only", &self.icon_only),
          flag("full_width", &self.full_width),
          flag("disabled", &self.disabled),
        ],
        changed: count(&[size != MD, with_icons, icon_only, full_width, disabled]),
        reset: Arc::new(move || {
          to_chosen.set(0);
          to_size.set(MD);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

// --- Button group ---------------------------------------------------------

const GROUP_VARIANTS: [(ButtonGroupVariant, &str); 4] = [
  (ButtonGroupVariant::Outline, "Outline"),
  (ButtonGroupVariant::Primary, "Primary"),
  (ButtonGroupVariant::Secondary, "Secondary"),
  (ButtonGroupVariant::Destructive, "Destructive"),
];
const UNITS: [&str; 4] = ["Years", "Months", "Weeks", "Days"];

pub struct ButtonGroupPlay {
  playground: Playground,
  variant: Signal<usize>,
  size: Signal<usize>,
  /// How many buttons the group has, less two.
  more: Signal<usize>,
  disabled: Signal<bool>,
}

impl ButtonGroupPlay {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      playground: Playground::new(ctx, 1),
      variant: ctx.signal(0),
      size: ctx.signal(MD),
      more: ctx.signal(1),
      disabled: ctx.signal(false),
    }
  }

  pub fn card(&self, ctx: &mut Ctx) -> Element {
    let (variant, size) = (self.variant.get(), self.size.get());
    let (buttons, disabled) = (self.more.get().min(2) + 2, self.disabled.get());
    let record = self.playground.recorder();
    let mut group = ButtonGroup::new()
      .variant(GROUP_VARIANTS[variant].0)
      .size(SIZES[size].0);
    for unit in UNITS.into_iter().take(buttons) {
      let record = record.clone();
      group = group.child(
        Button::new(unit)
          .disabled(disabled)
          .on_click(move |_| record(format!("on_click: {unit}"))),
      );
    }
    let mut code = vec!["ButtonGroup::new()".to_owned()];
    if variant != 0 {
      code.push(format!(
        "    .variant(ButtonGroupVariant::{})",
        GROUP_VARIANTS[variant].1
      ));
    }
    code.extend(size_line(size));
    for unit in UNITS.into_iter().take(buttons) {
      code.push(format!(
        "    .child(Button::new({unit:?}){}.on_click(..))",
        if disabled { ".disabled(true)" } else { "" }
      ));
    }
    let mut tree = vec![format!(
      "root      {}   {}",
      self.playground.measured(),
      if variant == 0 { "a line around" } else { "gap 1" }
    )];
    for (index, unit) in UNITS.into_iter().take(buttons).enumerate() {
      let last = index + 1 == buttons;
      tree.push(format!(
        "{} button    {unit:?}   padding 0 14",
        if last { "└─" } else { "├─" }
      ));
      if variant == 0 && !last {
        tree.push("├─ separator 1 wide".to_owned());
      }
    }
    let variants: Vec<(&str, usize)> = GROUP_VARIANTS
      .iter()
      .enumerate()
      .map(|(index, variant)| (variant.1, index))
      .collect();
    let (pick, more) = (self.variant.clone(), self.more.clone());
    let (to_variant, to_size, to_more, to_disabled) = (
      self.variant.clone(),
      self.size.clone(),
      self.more.clone(),
      self.disabled.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Button Group",
        make: Arc::new(move || {
          let mut group = ButtonGroup::new()
            .variant(GROUP_VARIANTS[variant].0)
            .size(SIZES[size].0);
          for unit in UNITS.into_iter().take(buttons) {
            group = group.child(Button::new(unit).disabled(disabled));
          }
          group.into()
        }),
        shown: group.into(),
        fill: false,
        forceable: false,
        facts: format!("{buttons} buttons"),
        event: "on_click",
        code,
        tree,
        controls: vec![
          prop(
            "variant",
            "ButtonGroupVariant",
            self
              .playground
              .select(0, &variants, &variant, move |index| pick.set(index)),
          ),
          size_control(&self.size),
          prop(
            "children",
            "Button",
            segmented_fill(
              &[("2", 0usize), ("3", 1), ("4", 2)],
              &self.more.get().min(2),
              move |count| more.set(count),
            ),
          ),
          divider(),
          flag("disabled", &self.disabled),
        ],
        changed: count(&[variant != 0, size != MD, disabled]),
        reset: Arc::new(move || {
          to_variant.set(0);
          to_size.set(MD);
          to_more.set(1);
          to_disabled.set(false);
        }),
      },
    )
  }
}

// --- Toggle group ---------------------------------------------------------

const MARKS: [(&str, &str, &str); 3] = [
  ("Bold", icons::BOLD, "BOLD"),
  ("Italic", icons::ITALIC, "ITALIC"),
  ("Underline", icons::UNDERLINE, "UNDERLINE"),
];

pub struct ToggleGroupPlay {
  playground: Playground,
  /// Whether any number of toggles are on, in place of one.
  multiple: Signal<bool>,
  one: Signal<usize>,
  many: Signal<Vec<usize>>,
  size: Signal<usize>,
  icon_only: Signal<bool>,
  disabled: Signal<bool>,
}

impl ToggleGroupPlay {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      playground: Playground::new(ctx, 0),
      multiple: ctx.signal(false),
      one: ctx.signal(0),
      many: ctx.signal(vec![0]),
      size: ctx.signal(MD),
      icon_only: ctx.signal(true),
      disabled: ctx.signal(false),
    }
  }

  pub fn card(&self, ctx: &mut Ctx) -> Element {
    let (multiple, size) = (self.multiple.get(), self.size.get());
    let (icon_only, disabled) = (self.icon_only.get(), self.disabled.get());
    let on: Vec<usize> = if multiple {
      self.many.get()
    } else {
      vec![self.one.get().min(MARKS.len() - 1)]
    };
    let record = self.playground.recorder();
    let (one, many) = (self.one.clone(), self.many.clone());
    let choices = MARKS
      .iter()
      .enumerate()
      .map(|(index, (label, glyph, _))| Choice::new(index, label).icon(*glyph).icon_only(icon_only));
    let group = ToggleGroup::controlled(on.clone(), choices, move |index: usize| {
      if multiple {
        many.update(|on| match on.iter().position(|held| *held == index) {
          Some(at) => {
            on.remove(at);
          }
          None => on.push(index),
        });
      } else {
        one.set(index);
      }
      record(format!("pressed: {}", MARKS[index].0));
    })
    .size(SIZES[size].0)
    .disabled(disabled);

    let mut code = vec![if multiple {
      "ToggleGroup::multiple(marks, [".to_owned()
    } else {
      "ToggleGroup::new(mark, [".to_owned()
    }];
    for (label, _, constant) in MARKS {
      code.push(format!("    Choice::new(Mark::{label}, {label:?})"));
      code.push(format!(
        "        .icon(icons::{constant}){},",
        if icon_only { ".icon_only(true)" } else { "" }
      ));
    }
    code.push("])".to_owned());
    code.extend(size_line(size));
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    let mut tree = vec![format!("root      {}   a line around", self.playground.measured())];
    for (index, (label, ..)) in MARKS.iter().enumerate() {
      let last = index + 1 == MARKS.len();
      tree.push(format!(
        "{} toggle    {label:?}{}",
        if last { "└─" } else { "├─" },
        if on.contains(&index) { "   on" } else { "" }
      ));
      if !last {
        tree.push("├─ separator 1 wide".to_owned());
      }
    }
    let names: Vec<&str> = MARKS
      .iter()
      .enumerate()
      .filter_map(|(index, mark)| on.contains(&index).then_some(mark.0))
      .collect();
    let mode = self.multiple.clone();
    let (to_mode, to_one, to_many, to_size, to_icons, to_disabled) = (
      self.multiple.clone(),
      self.one.clone(),
      self.many.clone(),
      self.size.clone(),
      self.icon_only.clone(),
      self.disabled.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Toggle Group",
        make: {
          let on = on.clone();
          Arc::new(move || {
            let choices = MARKS
              .iter()
              .enumerate()
              .map(|(index, (label, glyph, _))| Choice::new(index, label).icon(*glyph).icon_only(icon_only));
            ToggleGroup::controlled(on.clone(), choices, |_: usize| {})
              .size(SIZES[size].0)
              .disabled(disabled)
              .into()
          })
        },
        shown: group.into(),
        fill: false,
        forceable: false,
        facts: if names.is_empty() {
          "on: nothing".to_owned()
        } else {
          format!("on: {}", names.join(", "))
        },
        event: "pressed",
        code,
        tree,
        controls: vec![
          prop(
            "bound to",
            "Signal<T> or Signal<Vec<T>>",
            segmented_fill(&[("one", false), ("any number", true)], &multiple, move |many| {
              mode.set(many)
            }),
          ),
          size_control(&self.size),
          divider(),
          flag("icon_only", &self.icon_only),
          flag("disabled", &self.disabled),
        ],
        changed: count(&[multiple, size != MD, !icon_only, disabled]),
        reset: Arc::new(move || {
          to_mode.set(false);
          to_one.set(0);
          to_many.set(vec![0]);
          to_size.set(MD);
          to_icons.set(true);
          to_disabled.set(false);
        }),
      },
    )
  }
}
