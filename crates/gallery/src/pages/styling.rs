//! How a component of the design system is styled: the four levels, each live, with its code, and the order they
//! are laid in. Every scene is in a theme of its own, a copy of the gallery's, so what it changes reaches what is
//! in the scene and nothing of the gallery around it.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, RadiusSize, Theme, ThemeDefinition, TypographyStyle},
  },
  components::{Column, Grid, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, border::Border, color::Color},
  ui::{
    Badge, Button, ButtonPart, ButtonPartRenderer, ButtonVariant, Checkbox, Interaction, Link, Progress, Recipe,
    Switch, Tab, Tabs,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, code, muted, phone, quiet, segmented, styled},
};

/// Where the docs are.
const DOCS: &str = "https://ivanzaida.github.io/lurq";

/// The colors the palette's primary role can be given.
const PRIMARIES: [(&str, Option<&str>); 4] = [
  ("design", None),
  ("violet", Some("#7c3aed")),
  ("teal", Some("#0d9488")),
  ("orange", Some("#ea580c")),
];

/// What a recipe patched gives every button.
const SHAPES: [(&str, usize); 4] = [("design", 0), ("pill", 1), ("square", 2), ("tall", 3)];

/// A theme of a scene's own: a copy of the gallery's look as the page was opened, in the gallery's appearance.
struct Scoped {
  theme: Theme,
  base: ThemeDefinition,
}

impl Scoped {
  fn new(ctx: &mut Ctx) -> Self {
    let base = ctx.theme().definition();
    Self {
      theme: Theme::from_definition(base.clone()),
      base,
    }
  }

  /// `build` in the scene's theme, in the appearance the gallery is in.
  fn show(&self, ctx: &mut Ctx, build: impl FnOnce(&mut Ctx) -> Element) -> Element {
    self.theme.set_preferred_appearance(Some(ctx.theme().appearance()));
    ctx.in_theme(&self.theme, build)
  }
}

pub struct StylingPage {
  frame: Frame,
  palette: Scoped,
  primary: Signal<usize>,
  recipe: Scoped,
  shape: Signal<usize>,
  one: Signal<bool>,
  forced_hover: Signal<bool>,
  states: Scoped,
  pinned: Signal<bool>,
  layers: Scoped,
  theme_layer: Signal<bool>,
  rules_layer: Signal<bool>,
  style_layer: Signal<bool>,
  renderer: Scoped,
  own_label: Signal<bool>,
}

/// A row of a scene: what does not fit goes on to the next line.
fn scene() -> Row {
  Row::new()
    .width(FILL)
    .spacing(12.0)
    .wrap()
    .align_items(Alignment::Center)
}

/// A line under what a scene shows, as wide as the scene and wrapping in it.
fn note(text: &str) -> Column {
  Column::new().width(FILL).child(quiet(text, 12.0).width(FILL))
}

/// A switch of a scene's controls, bound to `on`.
fn toggle(label: &str, on: &Signal<bool>) -> Switch {
  Switch::new(label).bind(on.clone())
}

/// The controls of a demo over what it shows, a line under them.
fn controlled(controls: impl Into<Element>, shown: Element) -> Element {
  Column::new()
    .width(FILL)
    .spacing(20.0)
    .child(controls)
    .child(shown)
    .into()
}

/// What a part renderer of the label draws: the label after a mark, in the label's own style.
fn marked_label() -> ButtonPartRenderer {
  Arc::new(|view| view.part(ButtonPart::Label).text(&format!("» {}", view.label)).into())
}

impl StylingPage {
  /// The four levels, a card each, and the order they are laid in.
  fn levels(&self) -> Element {
    let level = |number: &str, title: &str, says: &str, call: &str, docs: &str, anchor: &str| {
      card()
        .padding(20.0)
        .spacing(8.0)
        .child(muted(number, TypographyStyle::Label))
        .child(styled(title, TypographyStyle::Title, |_| {}))
        .child(muted(says, TypographyStyle::Description).width(FILL))
        .child(Row::new().width(FILL).child(code(call).width(FILL)))
        .child(Link::new(docs).href(&format!("{DOCS}/{anchor}")).external(true))
    };
    let grid = Grid::with_columns(if phone() { 1 } else { 2 })
      .gap(16.0)
      .width(FILL)
      .child(level(
        "1 · the theme's tokens",
        "Tokens",
        "What blue is and how round Md is: the palette's roles and the scales. A role changed, every component \
         that is drawn with it follows.",
        "theme.set_palette_color(PaletteColor::Primary, ..)",
        "Theme",
        "theme/",
      ))
      .child(level(
        "2 · a component's recipe in the theme",
        "Recipes",
        "How each part of one kind of component looks, by variant, size, state, under the pointer, by density and \
         breakpoint. Patched, every one of that kind follows.",
        "theme.patch_recipe::<Button>(|r| ..)",
        "Recipes",
        "design-system/#recipes",
      ))
      .child(level(
        "3 · one instance",
        "Rules and style",
        "What one instance lays over its recipe: rules with selectors (.rules), a part's style (.style), a state of \
         its own (.state), a case drawn with nothing pointing at it (.force).",
        ".style(ButtonPart::Root, |s| ..)",
        "One instance",
        "design-system/#one-instance-one-place-one-subtree",
      ))
      .child(level(
        "4 · a renderer",
        "Renderers",
        "What nodes a component is built of, for a look a recipe cannot reach: all of it, or one part, with the \
         behavior still lurq's.",
        "theme.set_part_renderer::<Button, _>(..)",
        "Renderers",
        "design-system/#renderers",
      ));
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(grid)
      .child(
        card()
          .padding(20.0)
          .spacing(8.0)
          .child(styled("The order", TypographyStyle::Title, |_| {}))
          .child(
            muted(
              "Later over earlier: the theme's recipe (the design's, then what the theme patched), then the \
               rules of the place (.rules), then the instance's own (.style). The tokens are what all of them \
               are drawn with; a renderer is what draws. .state and .force say which case of the rules holds.",
              TypographyStyle::Description,
            )
            .width(FILL),
          )
          .child(
            Row::new()
              .width(FILL)
              .spacing(12.0)
              .wrap()
              .child(
                Link::new("Part styles")
                  .href(&format!("{DOCS}/design-system/#part-styles"))
                  .external(true),
              )
              .child(
                Link::new("What a theme changes")
                  .href(&format!("{DOCS}/design-system/#what-a-theme-changes"))
                  .external(true),
              )
              .child(Link::new("Tokens").href(&format!("{DOCS}/theme/")).external(true)),
          ),
      )
      .into()
  }

  /// A role of the palette changed: every component drawn with it follows.
  fn palette(&self, ctx: &mut Ctx) -> Element {
    let chosen = self.primary.get().min(PRIMARIES.len() - 1);
    let (theme, base, primary) = (
      self.palette.theme.clone(),
      self.palette.base.clone(),
      self.primary.clone(),
    );
    let controls = segmented(
      &PRIMARIES
        .iter()
        .enumerate()
        .map(|(index, (name, _))| (*name, index))
        .collect::<Vec<_>>(),
      &chosen,
      move |index| {
        primary.set(index);
        theme.apply(base.clone());
        if let Some(hex) = PRIMARIES[index].1 {
          theme.set_palette_color(PaletteColor::Primary, Color::from_hex(hex));
        }
      },
    );
    let shown = self.palette.show(ctx, |_| {
      scene()
        .child(Button::new("Save"))
        .child(Switch::new("Sync").on(true))
        .child(Checkbox::new("Remember me").checked(true))
        .child(Badge::new("New"))
        .child(Column::new().width(160.0).max_width(FILL).child(Progress::new(0.6)))
        .child(Tabs::controlled(
          0,
          [Tab::new(0, "Overview"), Tab::new(1, "Activity")],
          |_| {},
        ))
        .into()
    });
    controlled(controls, shown)
  }

  /// A recipe patched in the theme: every button follows, and nothing else.
  fn recipe(&self, ctx: &mut Ctx) -> Element {
    let chosen = self.shape.get().min(SHAPES.len() - 1);
    let (theme, shape) = (self.recipe.theme.clone(), self.shape.clone());
    let controls = segmented(&SHAPES, &chosen, move |index| {
      shape.set(index);
      theme.reset_recipe::<Button>();
      match index {
        1 => theme.patch_recipe::<Button>(|r| r.base(ButtonPart::Root, |s| s.radius(RadiusSize::Full))),
        2 => theme.patch_recipe::<Button>(|r| r.base(ButtonPart::Root, |s| s.radius(0.0))),
        3 => theme.patch_recipe::<Button>(|r| {
          r.base(ButtonPart::Root, |s| s.height(44.0).padding_x(24.0))
            .base(ButtonPart::Label, |s| s.typography(TypographyStyle::Title))
        }),
        _ => {}
      }
    });
    let shown = self.recipe.show(ctx, |_| {
      scene()
        .child(Button::new("Primary"))
        .child(Button::new("Secondary").variant(ButtonVariant::Secondary))
        .child(Button::new("Outline").variant(ButtonVariant::Outline))
        .child(Button::new("Ghost").variant(ButtonVariant::Ghost))
        .child(Switch::new("Not a button").on(true))
        .into()
    });
    controlled(controls, shown)
  }

  /// `.style(..)` on one button, and the others as the theme has them.
  fn one(&self) -> Element {
    let styled = self.one.get();
    let middle = Button::new("This one").variant(ButtonVariant::Outline);
    let middle = if styled {
      middle.style(ButtonPart::Root, |s| {
        s.background(PaletteColor::Success)
          .foreground(PaletteColor::PrimaryForeground)
          .radius(RadiusSize::Full)
      })
    } else {
      middle
    };
    controlled(
      toggle("Style the middle one", &self.one),
      scene()
        .child(Button::new("Before").variant(ButtonVariant::Outline))
        .child(middle)
        .child(Button::new("After").variant(ButtonVariant::Outline))
        .into(),
    )
  }

  /// `.rules(&recipe)` with a selector: one button turns destructive under the pointer.
  fn rules(&self) -> Element {
    let danger = Recipe::new().hovered(ButtonPart::Root, |s| {
      s.background(PaletteColor::Destructive)
        .foreground(PaletteColor::DestructiveForeground)
    });
    let forced = self.forced_hover.get().then_some(Interaction::Hovered);
    controlled(
      toggle(".force(hovered)", &self.forced_hover),
      scene()
        .child(
          Button::new("Delete")
            .variant(ButtonVariant::Outline)
            .rules(&danger)
            .force(forced),
        )
        .child(Button::new("Keep").variant(ButtonVariant::Outline))
        .child(note("Point at Delete: the rule holds under the pointer only."))
        .into(),
    )
  }

  /// A state of the app's own a theme styles, and the cases a component is drawn in with nothing pointing at it.
  fn states(&self, ctx: &mut Ctx) -> Element {
    let pinned = self.pinned.get();
    let shown = self.states.show(ctx, |_| {
      let forced = [
        ("at rest", None),
        ("hovered", Some(Interaction::Hovered)),
        ("pressed", Some(Interaction::Pressed)),
        ("focus", Some(Interaction::FocusVisible)),
      ];
      let mut row = scene().child(
        Button::new("Pinned tab")
          .variant(ButtonVariant::Outline)
          .state("pinned", pinned),
      );
      for (name, interaction) in forced {
        row = row.child(Button::new(name).variant(ButtonVariant::Secondary).force(interaction));
      }
      row.into()
    });
    controlled(toggle("pinned", &self.pinned), shown)
  }

  /// The three layers of rules on one button, each on or off: what is on top wins.
  fn layers(&self, ctx: &mut Ctx) -> Element {
    let (theme_on, rules_on, style_on) = (self.theme_layer.get(), self.rules_layer.get(), self.style_layer.get());
    let winner = if style_on {
      ".style: Success"
    } else if rules_on {
      ".rules: Warning"
    } else if theme_on {
      "the theme's recipe: Info"
    } else {
      "the design's recipe"
    };
    let controls = Row::new()
      .width(FILL)
      .spacing(20.0)
      .wrap()
      .child(toggle("theme", &self.theme_layer))
      .child(toggle(".rules", &self.rules_layer))
      .child(toggle(".style", &self.style_layer));
    let shown = self.layers.show(ctx, |_| {
      let mut button = Button::new("Layered");
      if rules_on {
        button = button.rules(&Recipe::new().base(ButtonPart::Root, |s| {
          s.background(PaletteColor::Warning).foreground(PaletteColor::Foreground)
        }));
      }
      if style_on {
        button = button.style(ButtonPart::Root, |s| {
          s.background(PaletteColor::Success)
            .foreground(PaletteColor::PrimaryForeground)
        });
      }
      scene()
        .child(button)
        .child(note(&format!("drawn by {winner}; the radius stays the theme's")))
        .into()
    });
    controlled(controls, shown)
  }

  /// A renderer of one part of its own: the label.
  fn renderer(&self, ctx: &mut Ctx) -> Element {
    let shown = self.renderer.show(ctx, |_| {
      scene()
        .child(Button::new("Continue"))
        .child(Button::new("Details").variant(ButtonVariant::Outline))
        .child(Button::new("Skip").variant(ButtonVariant::Ghost))
        .into()
    });
    controlled(toggle("A label renderer", &self.own_label), shown)
  }
}

impl Component for StylingPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let page = Self {
      frame: Frame::new(ctx),
      palette: Scoped::new(ctx),
      primary: ctx.signal(0),
      recipe: Scoped::new(ctx),
      shape: ctx.signal(0),
      one: ctx.signal(true),
      forced_hover: ctx.signal(false),
      states: Scoped::new(ctx),
      pinned: ctx.signal(true),
      layers: Scoped::new(ctx),
      theme_layer: ctx.signal(true),
      rules_layer: ctx.signal(true),
      style_layer: ctx.signal(true),
      renderer: Scoped::new(ctx),
      own_label: ctx.signal(true),
    };
    // A state the app names, styled by the theme: what `.state("pinned", true)` turns on.
    page.states.theme.patch_recipe::<Button>(|r| {
      r.state("pinned", ButtonPart::Root, |s| {
        s.border(Border::inside(BorderSize::Md, PaletteColor::Primary))
      })
    });
    // The theme's layer of the layered button, while it is on.
    let layered = |theme: &Theme, on: bool| {
      theme.reset_recipe::<Button>();
      if on {
        theme.patch_recipe::<Button>(|r| {
          r.base(ButtonPart::Root, |s| {
            s.background(PaletteColor::Info)
              .foreground(PaletteColor::PrimaryForeground)
              .radius(RadiusSize::Full)
          })
        });
      }
    };
    layered(&page.layers.theme, true);
    let theme = page.layers.theme.clone();
    ctx.watch(&page.theme_layer, move |on| layered(&theme, *on));
    let theme = page.renderer.theme.clone();
    theme.set_part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Label, marked_label());
    ctx.watch(&page.own_label, move |on| {
      if *on {
        theme.set_part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Label, marked_label());
      } else {
        theme.reset_part_renderer::<Button>(ButtonPart::Label);
      }
    });
    page
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let primary = PRIMARIES[self.primary.get().min(PRIMARIES.len() - 1)];
    let palette_code: Vec<String> = match primary.1 {
      Some(hex) => vec![
        "// Every component drawn with the primary role follows.".to_owned(),
        format!("app.theme().set_palette_color(PaletteColor::Primary, Color::from_hex({hex:?}));"),
      ],
      None => vec!["// The design's primary: nothing set.".to_owned()],
    };
    let shape_code: Vec<String> = match self.shape.get() {
      1 => vec![
        "app.theme().patch_recipe::<Button>(|r| {".to_owned(),
        "    r.base(ButtonPart::Root, |s| s.radius(RadiusSize::Full))".to_owned(),
        "});".to_owned(),
      ],
      2 => vec![
        "app.theme().patch_recipe::<Button>(|r| {".to_owned(),
        "    r.base(ButtonPart::Root, |s| s.radius(0.0))".to_owned(),
        "});".to_owned(),
      ],
      3 => vec![
        "app.theme().patch_recipe::<Button>(|r| {".to_owned(),
        "    r.base(ButtonPart::Root, |s| s.height(44.0).padding_x(24.0))".to_owned(),
        "        .base(ButtonPart::Label, |s| s.typography(TypographyStyle::Title))".to_owned(),
        "});".to_owned(),
      ],
      _ => vec!["// The design's recipe: nothing patched.".to_owned()],
    };
    let one_code: Vec<String> = if self.one.get() {
      vec![
        "Button::new(\"This one\")".to_owned(),
        "    .variant(ButtonVariant::Outline)".to_owned(),
        "    .style(ButtonPart::Root, |s| {".to_owned(),
        "        s.background(PaletteColor::Success)".to_owned(),
        "            .foreground(PaletteColor::PrimaryForeground)".to_owned(),
        "            .radius(RadiusSize::Full)".to_owned(),
        "    })".to_owned(),
      ]
    } else {
      vec!["Button::new(\"This one\").variant(ButtonVariant::Outline)".to_owned()]
    };
    let mut rules_code = vec![
      "let danger = Recipe::new().hovered(ButtonPart::Root, |s| {".to_owned(),
      "    s.background(PaletteColor::Destructive)".to_owned(),
      "        .foreground(PaletteColor::DestructiveForeground)".to_owned(),
      "});".to_owned(),
      "Button::new(\"Delete\").variant(ButtonVariant::Outline).rules(&danger)".to_owned(),
    ];
    if self.forced_hover.get() {
      rules_code.push("    .force(Some(Interaction::Hovered))".to_owned());
    }
    let states_code = vec![
      "app.theme().patch_recipe::<Button>(|r| {".to_owned(),
      "    r.state(\"pinned\", ButtonPart::Root, |s| {".to_owned(),
      "        s.border(Border::inside(BorderSize::Md, PaletteColor::Primary))".to_owned(),
      "    })".to_owned(),
      "});".to_owned(),
      format!("Button::new(\"Pinned tab\").state(\"pinned\", {})", self.pinned.get()),
      "Button::new(\"hovered\").force(Some(Interaction::Hovered))".to_owned(),
    ];
    let mut layers_code = Vec::new();
    if self.theme_layer.get() {
      layers_code.extend([
        "// 1. The theme's recipe.".to_owned(),
        "theme.patch_recipe::<Button>(|r| {".to_owned(),
        "    r.base(ButtonPart::Root, |s| s.background(PaletteColor::Info).radius(RadiusSize::Full))".to_owned(),
        "});".to_owned(),
      ]);
    }
    layers_code.push("Button::new(\"Layered\")".to_owned());
    if self.rules_layer.get() {
      layers_code.push("    // 2. The rules of the place.".to_owned());
      layers_code
        .push("    .rules(&Recipe::new().base(ButtonPart::Root, |s| s.background(PaletteColor::Warning)))".to_owned());
    }
    if self.style_layer.get() {
      layers_code.push("    // 3. The instance's own.".to_owned());
      layers_code.push("    .style(ButtonPart::Root, |s| s.background(PaletteColor::Success))".to_owned());
    }
    let renderer_code: Vec<String> = if self.own_label.get() {
      vec![
        "let label: ButtonPartRenderer = Arc::new(|view| {".to_owned(),
        "    view.part(ButtonPart::Label).text(&format!(\"» {}\", view.label)).into()".to_owned(),
        "});".to_owned(),
        "app.theme().set_part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Label, label);".to_owned(),
        "// All of it: app.theme().set_renderer::<Button>(renderer)".to_owned(),
      ]
    } else {
      vec!["// The default renderer draws every part.".to_owned()]
    };

    let levels = self.levels();
    let palette = self.palette(ctx);
    let recipe = self.recipe(ctx);
    let one = self.one();
    let rules = self.rules();
    let states = self.states(ctx);
    let layers = self.layers(ctx);
    let renderer = self.renderer(ctx);
    let playground = card().padding(20.0).spacing(8.0).child(
      muted(
        "Each component's page opens with a playground, and its panel has a Styling section beside the props: the \
           parts and the states the component declares, a part restyled live, for the one on the stage \
           (.style, .rules) or for every one in the stage's own theme (patch_recipe), and the Rust of both.",
        TypographyStyle::Description,
      )
      .width(FILL),
    );
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Getting started", "Styling"],
        title: "Styling",
        description: "A component is styled at four levels: the theme's tokens, its recipe in the theme, one \
                      instance, and a renderer. Each is live here, with its code; what a scene changes stays in it.",
        import: Some("use lurq::{app::theme::*, ui::*};"),
        status: Some("Stable"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Four levels",
          "Most apps change tokens; a brand patches recipes; one place styles an instance; a look a recipe cannot \
           reach has a renderer.",
          levels,
        ),
        Section::new(
          "A palette role",
          "The primary role given another color: the button, the switch, the box, the badge, the bar and the tab's \
           line follow, as they are drawn with it.",
          self.frame.demo(
            0,
            palette,
            "theme.set_palette_color(PaletteColor::Primary, ..)",
            &lines(&palette_code),
          ),
        ),
        Section::new(
          "A recipe patched",
          "Rules laid over the button's recipe in the theme: every button follows, of every variant; the switch \
           beside them does not.",
          self
            .frame
            .demo(1, recipe, "theme.patch_recipe::<Button>(..)", &lines(&shape_code)),
        ),
        Section::new(
          "One instance: .style",
          "One part of one button styled over what the theme gives it; its neighbours keep the theme's.",
          self
            .frame
            .demo(2, one, ".style(ButtonPart::Root, ..)", &lines(&one_code)),
        ),
        Section::new(
          "One instance: .rules",
          "Rules with selectors, laid on one instance: what a component says of a button in its place. Here, a \
           destructive fill under the pointer. .force draws it so with nothing pointing at it.",
          self.frame.demo(3, rules, ".rules(&recipe)", &lines(&rules_code)),
        ),
        Section::new(
          "States: .state and .force",
          "A state the app names, which the theme styles: .state(\"pinned\", on) turns it on. And the cases of the \
           pointer and the keyboard, drawn with nothing pointing at them.",
          self
            .frame
            .demo(4, states, ".state(\"pinned\", true) · .force(..)", &lines(&states_code)),
        ),
        Section::new(
          "The order",
          "The theme's recipe, then .rules, then .style: each on or off, the one on top draws the fill.",
          self
            .frame
            .demo(5, layers, "theme → .rules → .style", &lines(&layers_code)),
        ),
        Section::new(
          "A renderer of one part",
          "The label of every button in the scene's theme drawn by a renderer of its own, in the label's style; \
           the rest of the button, and what it does, is still the default renderer's.",
          self.frame.demo(
            6,
            renderer,
            "theme.set_part_renderer::<Button, _>(ButtonPart::Label, ..)",
            &lines(&renderer_code),
          ),
        ),
        Section::new(
          "In every playground",
          "The Styling tab of a component's playground does this for any component, from the parts it declares.",
          playground,
        ),
      ],
    )
  }
}

/// Lines of code as a demo card takes them.
fn lines(code: &[String]) -> Vec<&str> {
  code.iter().map(String::as_str).collect()
}
