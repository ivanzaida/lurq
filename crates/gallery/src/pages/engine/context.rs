//! The page of context: a value a component provides to everything mounted under it, which those read by its type
//! with no props between. Its playground is a provider of a locale and two children that read it; its sections
//! are the demo's Context page: a theme switched inside a part of the page, and the provider of a locale with a
//! button that switches it, beside children that no provider is over.
//!
//! The demo switched its own window's theme. Here the switch is the stage's: a theme of its own that what is in
//! the stage is built in, and the gallery's theme stays as it is.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::MouseEvent,
    theme::{PaletteColor, RadiusSize, Theme},
    window::Appearance,
  },
  components::{Column, Row},
  core::{ReactiveContext, Signal},
  layout::text_style::FontWeight,
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant},
};

use super::kit::{sample, samples, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, line, mono, props_table, quiet, segmented, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// What the provider gives the children under it: a locale's name, its greeting and how it writes a date.
#[derive(Clone, Hash, PartialEq, Eq, lurq::DevtoolsInspectable)]
struct LocaleContext {
  locale: &'static str,
  greeting: &'static str,
  date_format: &'static str,
}

const LOCALES: [LocaleContext; 3] = [
  LocaleContext {
    locale: "en-US",
    greeting: "Hello!",
    date_format: "MM/DD/YYYY",
  },
  LocaleContext {
    locale: "en-GB",
    greeting: "Hello there!",
    date_format: "DD/MM/YYYY",
  },
  LocaleContext {
    locale: "de-DE",
    greeting: "Hallo!",
    date_format: "DD.MM.YYYY",
  },
];
/// The choices of the playground's selects: an index into [`LOCALES`], and the way the value is provided.
const LOCALE_CHOICES: [(&str, usize); 3] = [("en-US", 0), ("en-GB", 1), ("de-DE", 2)];
const WAYS: [(&str, bool); 2] = [("create_context", true), ("provide", false)];
const APPEARANCES: [(&str, bool); 2] = [("Light", false), ("Dark", true)];

/// What a child shows when nothing above it provides a locale.
const UNAVAILABLE: LocaleContext = LocaleContext {
  locale: "unknown",
  greeting: "Unavailable",
  date_format: "Unavailable",
};

/// What a provider of the locale is given: which locale, and whether it provides a reactive context or a plain
/// value.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct ProviderProps {
  /// An index into [`LOCALES`].
  locale: usize,
  reactive: bool,
}

/// A component that provides the locale its props name to what it mounts, and mounts the two children.
struct LocaleProvider {
  context: ReactiveContext<LocaleContext>,
}

impl Component for LocaleProvider {
  type Props = ProviderProps;

  fn create(ctx: &mut Ctx) -> Self {
    let ProviderProps { locale, .. } = ctx.props::<ProviderProps>().clone();
    Self {
      context: ctx.create_context(LOCALES[locale.min(LOCALES.len() - 1)].clone()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let ProviderProps { locale, reactive } = ctx.props::<ProviderProps>().clone();
    let value = LOCALES[locale.min(LOCALES.len() - 1)].clone();
    if reactive {
      // Set to the same value it tells nothing: the hash is the same.
      self.context.set(value.clone());
    } else {
      ctx.provide(value.clone());
    }
    Column::new()
      .width(FILL)
      .spacing(8.0)
      .child(mono(&format!("Provider: locale = {:?}", value.locale), 13.0).width(FILL))
      .child(ctx.mount::<LocaleChild>(ChildProps {
        name: "Child A",
        greeting: true,
        reactive,
      }))
      .child(ctx.mount::<LocaleChild>(ChildProps {
        name: "Child B",
        greeting: false,
        reactive,
      }))
  }
}

/// What a child that reads the locale is given: its name, which part of the locale it shows, and whether it
/// reads a reactive context or a plain value.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct ChildProps {
  name: &'static str,
  /// The greeting where true, the format of a date where false.
  greeting: bool,
  reactive: bool,
}

/// A child that reads the locale from the context, or says it is unavailable where nothing provides it.
struct LocaleChild;

impl Component for LocaleChild {
  type Props = ChildProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let ChildProps {
      name,
      greeting,
      reactive,
    } = ctx.props::<ChildProps>().clone();
    let locale = if reactive {
      ctx.consume_context::<LocaleContext>().map(|context| context.get())
    } else {
      ctx.use_context::<LocaleContext>()
    }
    .unwrap_or(UNAVAILABLE);
    let said = if greeting {
      format!("locale: {:?}, greeting: {:?}", locale.locale, locale.greeting)
    } else {
      format!("locale: {:?}, date format: {:?}", locale.locale, locale.date_format)
    };
    Column::new()
      .width(FILL)
      .spacing(2.0)
      .padding_horizontal(12.0)
      .padding_vertical(8.0)
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Background)
      .border(line())
      .child(text(name, 12.0, FontWeight::SemiBold).color(PaletteColor::Primary))
      .child(mono(&said, 12.0).width(FILL))
  }
}

/// A component that says which appearance the theme it is built in has: the theme is a context too.
struct ThemeReader;

impl Component for ThemeReader {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let dark = ctx.theme().appearance() == Appearance::Dark;
    mono(
      &format!("ctx.theme().appearance(): {}", if dark { "Dark" } else { "Light" }),
      12.0,
    )
    .width(FILL)
  }
}

/// A theme of the design in one appearance.
fn theme_in(appearance: Appearance) -> Theme {
  let theme = Theme::new();
  theme.set_preferred_appearance(Some(appearance));
  theme
}

/// A small card in the theme it is built in: its name, two of its colors, and a primary button.
fn preview(title: &str, theme: &Theme) -> Column {
  let hex = |role: PaletteColor| theme.palette_color(role).to_hex();
  Column::new()
    .width(220.0)
    .max_width(FILL)
    .spacing(8.0)
    .padding(16.0)
    .rounded(RadiusSize::Lg)
    .background(PaletteColor::Background)
    .foreground(PaletteColor::Foreground)
    .border(line())
    .child(text(title, 16.0, FontWeight::Bold))
    .child(quiet(
      &format!(
        "background {}  card {}",
        hex(PaletteColor::Background),
        hex(PaletteColor::Card)
      ),
      11.0,
    ))
    .child(Row::new().child(Button::new("Primary").size(ButtonSize::Sm)))
}

pub struct ContextPage {
  frame: Frame,
  playground: Playground,
  /// An index into [`LOCALES`].
  locale: Signal<usize>,
  reactive: Signal<bool>,
  provided: Signal<bool>,
  /// The locale of the section's provider, which its button switches.
  section_locale: Signal<usize>,
  /// Whether the stage of the theme's section is dark, and the two themes it is built in.
  dark: Signal<bool>,
  light_theme: Theme,
  dark_theme: Theme,
}

impl ContextPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (locale, reactive, provided) = (self.locale.get(), self.reactive.get(), self.provided.get());
    let props = ProviderProps { locale, reactive };
    let scene = move |ctx: &mut Ctx| -> Element {
      if provided {
        ctx.mount::<LocaleProvider>(props.clone())
      } else {
        // No provider above them: each child says so.
        Column::new()
          .width(FILL)
          .spacing(8.0)
          .child(quiet("No provider", 13.0))
          .child(ctx.mount::<LocaleChild>(ChildProps {
            name: "Child A",
            greeting: true,
            reactive,
          }))
          .child(ctx.mount::<LocaleChild>(ChildProps {
            name: "Child B",
            greeting: false,
            reactive,
          }))
          .into()
      }
    };
    let framed =
      move |ctx: &mut Ctx| -> Element { Column::new().width(360.0).max_width(FILL).child(scene(ctx)).into() };
    self.playground.window_shows(framed.clone());

    let value = &LOCALES[locale.min(LOCALES.len() - 1)];
    let mut code = vec!["// The provider:".to_owned()];
    if reactive {
      code.extend([
        "// in create".to_owned(),
        format!(
          "let locale = ctx.create_context(LocaleContext {{ locale: {:?}, .. }});",
          value.locale
        ),
        "// in render, or a handler".to_owned(),
        format!("locale.set(LocaleContext {{ locale: {:?}, .. }});", value.locale),
        String::new(),
        "// A child under it:".to_owned(),
        "let locale = ctx.consume_context::<LocaleContext>().map(|locale| locale.get());".to_owned(),
      ]);
    } else {
      code.extend([
        "// in render".to_owned(),
        format!("ctx.provide(LocaleContext {{ locale: {:?}, .. }});", value.locale),
        String::new(),
        "// A child under it:".to_owned(),
        "let locale = ctx.use_context::<LocaleContext>();".to_owned(),
      ]);
    }
    if !provided {
      code.push("// Nothing provides it: None, and the child says Unavailable.".to_owned());
    }
    let tree = if provided {
      vec![
        format!("LocaleProvider   {}", self.playground.measured()),
        format!("├─ LocaleChild   Child A   {:?}", value.greeting),
        format!("└─ LocaleChild   Child B   {:?}", value.date_format),
      ]
    } else {
      vec![
        format!("column   {}", self.playground.measured()),
        "├─ LocaleChild   Child A   \"Unavailable\"".to_owned(),
        "└─ LocaleChild   Child B   \"Unavailable\"".to_owned(),
      ]
    };

    let (set_locale, set_way) = (self.locale.clone(), self.reactive.clone());
    let reset = {
      let (locale, reactive, provided) = (self.locale.clone(), self.reactive.clone(), self.provided.clone());
      Arc::new(move || {
        locale.set(0);
        reactive.set(true);
        provided.set(true);
      })
    };
    let mut controls = vec![];
    if provided {
      controls.push(prop(
        "locale",
        "LocaleContext",
        self
          .playground
          .select(0, &LOCALE_CHOICES, &locale, move |value| set_locale.set(value)),
      ));
    }
    controls.push(prop(
      "how",
      "ReactiveContext<T> / T",
      self
        .playground
        .select(1, &WAYS, &reactive, move |value| set_way.set(value)),
    ));
    controls.push(divider());
    controls.push(flag("a provider above the children", &self.provided));
    self.playground.card(
      ctx,
      Spec {
        title: "Context",
        // The scene is of mounted components: the window of its own mounts them there (`window_shows`).
        make: Arc::new(|| Row::new().into()),
        shown: Shown::built(framed),
        fill: false,
        forceable: false,
        facts: if provided {
          value.locale.to_owned()
        } else {
          "no provider".to_owned()
        },
        event: "",
        code,
        tree,
        controls,
        changed: [locale != 0 && provided, !reactive, !provided]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// A part of the page in a theme of its own, which a switch makes light or dark, and a card in each theme.
  fn themes(&self, ctx: &mut Ctx) -> Column {
    let dark = self.dark.get();
    let set = self.dark.clone();
    let theme = if dark { &self.dark_theme } else { &self.light_theme };
    let stage = ctx.in_theme(theme, |ctx| {
      Column::new()
        .width(FILL)
        .spacing(10.0)
        .padding(16.0)
        .rounded(RadiusSize::Lg)
        .background(PaletteColor::Canvas)
        .foreground(PaletteColor::Foreground)
        .border(line())
        .child(text(
          &format!("Theme of this part: {}", if dark { "Dark" } else { "Light" }),
          14.0,
          FontWeight::Medium,
        ))
        .child(ctx.mount::<ThemeReader>(()))
        .child(
          Row::new()
            .spacing(8.0)
            .wrap()
            .child(Button::new("Primary").size(ButtonSize::Sm))
            .child(
              Button::new("Outline")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm),
            ),
        )
    });
    let light = ctx.in_theme(&self.light_theme, |_| preview("Light theme", &self.light_theme));
    let dark_card = ctx.in_theme(&self.dark_theme, |_| preview("Dark theme", &self.dark_theme));
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(segmented(&APPEARANCES, &dark, move |value| set.set(value)))
      .child(stage)
      .child(
        samples()
          .child(sample(dark_card, "Theme::new(), dark"))
          .child(sample(light, "Theme::new(), light")),
      )
  }

  /// The demo's provider of a locale with its button, and the two children outside any provider.
  fn locales(&self, ctx: &mut Ctx) -> Column {
    let at = self.section_locale.get();
    let switch = self.section_locale.clone();
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(
        well()
          .padding(16.0)
          .spacing(12.0)
          .child(
            Button::new("Switch locale")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_: MouseEvent| switch.set((switch.get() + 1) % LOCALES.len())),
          )
          .child(ctx.mount::<LocaleProvider>(ProviderProps {
            locale: at,
            reactive: true,
          })),
      )
      .child(
        Column::new()
          .width(FILL)
          .spacing(8.0)
          .child(quiet("Outside the provider", 12.0))
          .child(ctx.mount::<LocaleChild>(ChildProps {
            name: "Child A",
            greeting: true,
            reactive: true,
          })),
      )
  }
}

impl Component for ContextPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      locale: ctx.signal(0),
      reactive: ctx.signal(true),
      provided: ctx.signal(true),
      section_locale: ctx.signal(0),
      dark: ctx.signal(true),
      light_theme: theme_in(Appearance::Light),
      dark_theme: theme_in(Appearance::Dark),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "ctx.create_context",
        "T -> ReactiveContext<T>",
        "",
        "Provides a value that can change to what the component mounts. Made in create; set tells the children \
         that read it when its hash changes.",
      ),
      (
        "ctx.consume_context",
        "Option<ReactiveContext<T>>",
        "",
        "The nearest reactive context of the type above the component, None where there is none. Reading it \
         renders the component again when it is set.",
      ),
      (
        "ctx.provide",
        "T",
        "",
        "Provides a plain value by its type: in create for the component's life, in render for that render.",
      ),
      (
        "ctx.use_context",
        "Option<T>",
        "",
        "The nearest value of the type provided above the component, None where there is none.",
      ),
      (
        "ctx.in_theme",
        "&Theme, FnOnce(&mut Ctx)",
        "",
        "Builds a part of the tree in a theme of its own: a dark side bar in a light window.",
      ),
      ("ctx.theme", "&Theme", "", "The theme the component is built in."),
    ];
    let playground = self.play(ctx);
    let themes = self.themes(ctx);
    let locales = self.locales(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Context"],
        title: "Context",
        description: "A value a component provides to everything mounted under it, which those read by its type \
                      with no props between: a locale, a theme, a session.",
        import: Some("use lurq::{app::ctx::Ctx, core::ReactiveContext};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A provider of a locale with two children that read it. Choose the locale, and how it is provided: a \
           reactive context or a plain value. Without a provider above them, the children find none.",
          playground,
        ),
        Section::new(
          "Theme switching",
          "The theme is a context too. This part of the page is built in a theme of its own, light or dark, and \
           what is in it takes its colors from that: the gallery's own theme stays as it is.",
          self.frame.demo(
            0,
            themes,
            "ctx.in_theme(&theme, |ctx| ..)",
            &[
              "// In create:",
              "let dark = Theme::new();",
              "dark.set_preferred_appearance(Some(Appearance::Dark));",
              "",
              "// In render:",
              "ctx.in_theme(&dark, |ctx| {",
              "    Column::new()",
              "        .background(PaletteColor::Canvas)",
              "        .child(ctx.mount::<ThemeReader>(()))",
              "})",
              "",
              "// ThemeReader, in render:",
              "let dark = ctx.theme().appearance() == Appearance::Dark;",
            ],
          ),
        ),
        Section::new(
          "Context provide and consume",
          "The provider sets its context to the next locale at each press of the button, and both children under \
           it render again. A child outside it finds no locale and says Unavailable.",
          self.frame.demo(
            1,
            locales,
            "ctx.consume_context::<LocaleContext>()",
            &[
              "#[derive(Clone, Hash)]",
              "struct LocaleContext { locale: &'static str, greeting: &'static str, date_format: &'static str }",
              "",
              "// LocaleProvider, in create:",
              "let locale = ctx.create_context(EN_US);",
              "// in render:",
              "locale.set(LOCALES[props.locale].clone());",
              "",
              "// LocaleChild, in render:",
              "let locale = ctx",
              "    .consume_context::<LocaleContext>()",
              "    .map(|locale| locale.get())",
              "    .unwrap_or(UNAVAILABLE);",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a component's context provides and reads.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
