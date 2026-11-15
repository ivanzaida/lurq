//! The page of the volume control: its playground, the control bound to the page's signals, the glyph of each
//! level, the collapsed form, and the white form over a picture.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::RadiusSize},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::{Element, Gradient, GradientStop, color::Color},
  ui::{MediaVolume, MediaVolumeSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::End)
}

/// What stands in for a picture: the board's sunset, from the top down, with what is on it at its bottom.
fn picture(width: f32, height: f32) -> Row {
  let stops = [
    ("#132544", 0.0),
    ("#1E3A8A", 0.3),
    ("#7C3AED", 0.58),
    ("#F472B6", 0.78),
    ("#F59E0B", 0.9),
    ("#3B1212", 1.0),
  ]
  .map(|(color, at)| GradientStop::at(Color::from_hex(color), at));
  Row::new()
    .size(width, height)
    .padding_horizontal(12.0)
    .padding_bottom(12.0)
    .rounded(RadiusSize::Lg)
    .background_gradient(Gradient::linear(180.0, stops))
    .align_items(Alignment::End)
    .justify(Justify::Start)
}

/// The sizes, as the playground offers them.
const SIZES: [(&str, MediaVolumeSize); 2] = [("Md", MediaVolumeSize::Md), ("Lg", MediaVolumeSize::Lg)];

/// The lengths of the slider the playground offers: the recipe's, and the two the players use.
const LENGTHS: [(&str, Option<f32>); 3] = [("72 (the recipe's)", None), ("64", Some(64.0)), ("96", Some(96.0))];

pub struct MediaVolumePage {
  frame: Frame,
  playground: Playground,
  volume: Signal<f32>,
  muted: Signal<bool>,
  size: Signal<usize>,
  length: Signal<usize>,
  collapsed: Signal<bool>,
  on_media: Signal<bool>,
  disabled: Signal<bool>,
  label: Signal<String>,
  mute_label: Signal<String>,
  unmute_label: Signal<String>,
  // The control the page's own signals drive.
  live: Signal<f32>,
  live_muted: Signal<bool>,
}

impl MediaVolumePage {
  /// The playground: the control as the panel's props make it, bound to the page's volume and muted.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let size = self.size.get().min(SIZES.len() - 1);
    let length = self.length.get().min(LENGTHS.len() - 1);
    let (muted, collapsed, on_media, disabled) = (
      self.muted.get(),
      self.collapsed.get(),
      self.on_media.get(),
      self.disabled.get(),
    );
    let (label, mute_label, unmute_label) = (self.label.get(), self.mute_label.get(), self.unmute_label.get());
    let volume = self.volume.get();
    let plain = {
      let (label, mute_label, unmute_label) = (label.clone(), mute_label.clone(), unmute_label.clone());
      move |volume: f32| {
        let mut control = MediaVolume::controlled(volume, |_| {})
          .muted_controlled(muted, |_| {})
          .size(SIZES[size].1)
          .collapsed(collapsed)
          .on_media(on_media)
          .disabled(disabled)
          .label(&label)
          .mute_label(&mute_label)
          .unmute_label(&unmute_label);
        if let Some(length) = LENGTHS[length].1 {
          control = control.length(length);
        }
        control
      }
    };
    let record = self.playground.recorder();
    let (change, end, mute) = (record.clone(), record.clone(), record);
    let mut control = MediaVolume::new(self.volume.clone())
      .muted(self.muted.clone())
      .size(SIZES[size].1)
      .collapsed(collapsed)
      .on_media(on_media)
      .disabled(disabled)
      .label(&label)
      .mute_label(&mute_label)
      .unmute_label(&unmute_label)
      .force(self.playground.forced())
      .on_change(move |to| change(format!("on_change({to})")))
      .on_change_end(move |to| end(format!("on_change_end({to})")))
      .on_muted_change(move |next| mute(format!("on_muted_change({next})")));
    if let Some(length) = LENGTHS[length].1 {
      control = control.length(length);
    }
    let playground = &self.playground;
    let shown = if on_media {
      Shown::built(move |ctx| {
        let mounted = control.mount(ctx);
        picture(280.0, 96.0)
          .overflow_visible()
          .child(playground.measure(mounted))
          .into()
      })
      .measured()
    } else {
      Shown::built(move |ctx| control.mount(ctx))
    };

    let mut code = vec![
      "// volume: Signal<f32>, muted: Signal<bool>".to_owned(),
      "MediaVolume::new(volume)".to_owned(),
      "    .muted(muted)".to_owned(),
    ];
    if size != 0 {
      code.push("    .size(MediaVolumeSize::Lg)".to_owned());
    }
    if let Some(length) = LENGTHS[length].1 {
      code.push(format!("    .length({length:?})"));
    }
    for (on, call) in [
      (collapsed, ".collapsed(true)"),
      (on_media, ".on_media(true)"),
      (disabled, ".disabled(true)"),
    ] {
      if on {
        code.push(format!("    {call}"));
      }
    }
    for (said, default, call) in [
      (&label, "Volume", "label"),
      (&mute_label, "Mute", "mute_label"),
      (&unmute_label, "Unmute", "unmute_label"),
    ] {
      if said != default {
        code.push(format!("    .{call}({said:?})"));
      }
    }
    code.push(if collapsed {
      "    .mount(ctx)".to_owned()
    } else {
      "    .on_change(|volume| println!(\"{volume}\"))".to_owned()
    });

    let silent = muted || volume <= 0.0;
    let glyph = if silent {
      "volume-off"
    } else if volume < 0.5 {
      "volume-low"
    } else {
      "volume"
    };
    let (button, icon) = if size == 0 { (32, 16) } else { (36, 20) };
    let slider = LENGTHS[length].1.unwrap_or(72.0);
    let mut tree = vec![
      format!("root          {}   gap 4", self.playground.measured()),
      format!("├─ button     {button} × {button} px   ghost   {glyph} {icon}"),
    ];
    tree.push(if collapsed {
      format!("└─ slider     {slider} long, under the pointer and with the focus")
    } else {
      format!("└─ slider     {slider} long   at {}", if muted { 0.0 } else { volume })
    });

    let (pick_size, pick_length) = (self.size.clone(), self.length.clone());
    let labels = [
      (self.label.clone(), "Volume"),
      (self.mute_label.clone(), "Mute"),
      (self.unmute_label.clone(), "Unmute"),
    ];
    let (to_size, to_length, to_volume) = (self.size.clone(), self.length.clone(), self.volume.clone());
    let flags = [
      self.muted.clone(),
      self.collapsed.clone(),
      self.on_media.clone(),
      self.disabled.clone(),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Volume",
        make: Arc::new(move || plain(volume).into()),
        shown,
        fill: false,
        forceable: true,
        facts: format!(
          "volume {volume:.2} · {}",
          if muted { "muted, kept" } else { "sound on" }
        ),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "size",
            "MediaVolumeSize",
            self
              .playground
              .select(0, &SIZES.map(|(name, _)| (name, name)), &SIZES[size].0, move |name| {
                pick_size.set(SIZES.iter().position(|(held, _)| *held == name).unwrap_or(0))
              }),
          ),
          prop(
            "length",
            "f32",
            self.playground.select(
              1,
              &LENGTHS.map(|(name, _)| (name, name)),
              &LENGTHS[length].0,
              move |name| pick_length.set(LENGTHS.iter().position(|(held, _)| *held == name).unwrap_or(0)),
            ),
          ),
          prop("label", "&str", field(&self.label)),
          prop("mute_label", "&str", field(&self.mute_label)),
          prop("unmute_label", "&str", field(&self.unmute_label)),
          divider(),
          flag("muted", &self.muted),
          flag("collapsed", &self.collapsed),
          flag("on_media", &self.on_media),
          flag("disabled", &self.disabled),
        ],
        changed: [
          size != 0,
          length != 0,
          muted,
          collapsed,
          on_media,
          disabled,
          label != "Volume",
          mute_label != "Mute",
          unmute_label != "Unmute",
          (volume - 0.6).abs() > f32::EPSILON,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_size.set(0);
          to_length.set(0);
          to_volume.set(0.6);
          for flag in &flags {
            flag.set(false);
          }
          for (label, default) in &labels {
            label.set((*default).to_owned());
          }
        }),
      },
    )
  }
}

impl Component for MediaVolumePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      volume: ctx.signal(0.6),
      muted: ctx.signal(false),
      size: ctx.signal(0),
      length: ctx.signal(0),
      collapsed: ctx.signal(false),
      on_media: ctx.signal(false),
      disabled: ctx.signal(false),
      label: ctx.signal("Volume".to_owned()),
      mute_label: ctx.signal("Mute".to_owned()),
      unmute_label: ctx.signal("Unmute".to_owned()),
      live: ctx.signal(0.6),
      live_muted: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (live, live_muted) = (self.live.get(), self.live_muted.get());
    let bound = items().child(item(
      MediaVolume::new(self.live.clone()).muted(self.live_muted.clone()),
      &format!(
        "volume {live:.2}, {}",
        if live_muted {
          "muted: the volume is kept"
        } else {
          "sound on"
        }
      ),
    ));
    let at = |volume: f32, muted: bool| MediaVolume::controlled(volume, |_| {}).muted_controlled(muted, |_| {});
    let levels = items()
      .child(item(at(0.8, false), "volume · from a half up"))
      .child(item(at(0.2, false), "volume-low · under a half"))
      .child(item(at(0.8, true), "volume-off · muted, 0.8 kept"))
      .child(item(at(0.0, false), "volume-off · at nothing"))
      .child(item(at(0.6, false).disabled(true), "disabled"));
    let collapsed = items()
      .child(item(
        MediaVolume::new(self.live.clone())
          .muted(self.live_muted.clone())
          .collapsed(true)
          .mount(ctx),
        "collapsed · the slider under the pointer and with the focus",
      ))
      .child(item(
        MediaVolume::new(self.live.clone())
          .muted(self.live_muted.clone())
          .size(MediaVolumeSize::Lg)
          .collapsed(true)
          .mount(ctx),
        "collapsed · lg",
      ));
    let on_picture = |control: Element| picture(300.0, 96.0).overflow_visible().child(control);
    let over = items()
      .child(item(
        on_picture(
          MediaVolume::new(self.live.clone())
            .muted(self.live_muted.clone())
            .on_media(true)
            .size(MediaVolumeSize::Lg)
            .length(64.0)
            .into(),
        ),
        "on media · lg, as the video player's bar has it",
      ))
      .child(item(
        on_picture(
          MediaVolume::new(self.live.clone())
            .muted(self.live_muted.clone())
            .on_media(true)
            .collapsed(true)
            .mount(ctx),
        ),
        "on media · collapsed, as a narrow bar has it",
      ));
    let rows = [
      (
        "new",
        "Signal<f32>",
        "",
        "Bound to the app's volume, from 0 to 1, which a drag, a press on the slider and the keys set.",
      ),
      (
        "controlled",
        "impl Into<f64>, impl Fn(f64)",
        "",
        "In place of new: the app holds the volume and is told what a person makes it.",
      ),
      (
        "muted",
        "Signal<bool>",
        "",
        "Muted while true, which a press of the button turns. Muted, the slider stands at nothing and the volume is \
         kept; dragging it up turns the sound back on.",
      ),
      (
        "muted_controlled",
        "bool, impl Fn(bool)",
        "",
        "In place of muted: the app holds it and is told what a press makes it.",
      ),
      (
        "on_change",
        "impl Fn(f64)",
        "",
        "Called with the volume each time a person moves it. Bound, after the signal is set.",
      ),
      (
        "on_change_end",
        "impl Fn(f64)",
        "",
        "Called with the volume when a drag is let go or a key's step is made.",
      ),
      (
        "on_muted_change",
        "impl Fn(bool)",
        "",
        "Called on a press of the button with whether it is to be muted, and with false when the slider is moved \
         up from nothing while muted.",
      ),
      (
        "collapsed",
        "bool",
        "false",
        "The button alone, the slider coming while the pointer is over it or the focus is in it. Needs mount(ctx).",
      ),
      (
        "on_media",
        "bool",
        "false",
        "Over a picture: white, the same in a light and a dark theme.",
      ),
      (
        "size",
        "MediaVolumeSize",
        "Md",
        "Md: a button of 32 with 16; Lg: 36 with 20.",
      ),
      ("length", "f32", "72", "How long the slider is."),
      ("label", "&str", "\"Volume\"", "What the slider is called to tooling."),
      (
        "mute_label",
        "&str",
        "\"Mute\"",
        "What the button is called while a press would mute.",
      ),
      (
        "unmute_label",
        "&str",
        "\"Unmute\"",
        "What the button is called while a press would bring the sound back.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; takes no input and Tab passes it by.",
      ),
      (
        "style / rules / state",
        "MediaVolumePart, Recipe, Name",
        "",
        "A part restyled, the rules of the component it is in, a state the theme's recipe names.",
      ),
      (
        "force",
        "Option<Interaction>",
        "None",
        "Draws the button hovered, pressed or focused with nothing pointing at it.",
      ),
      (
        "id / key",
        "impl Into<Arc<str>>",
        "",
        "The id of the button; what tells it apart from those beside it.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Puts it in the tree keeping the pointer, the focus and a drag: what a collapsed one needs.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Volume"],
        title: "Volume",
        description: "A button that mutes and a slider of how loud. Muting keeps the volume, so that the sound comes \
                      back as loud as it was.",
        import: Some("use lurq::ui::{MediaVolume, MediaVolumeSize};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of MediaVolume is a control on the right. The control on the stage is bound to the page's \
           volume and muted: drag the slider, press the button, and the Events tab hears each change. Over a \
           picture it stands on a gradient in place of one.",
          playground,
        ),
        Section::new(
          "Bound to signals",
          "The page holds the volume and whether it is muted. Muted, the slider stands at nothing and the volume \
           is kept; moving the slider up turns the sound back on. Tab reaches the button and then the slider: Space \
           or Enter mutes, the arrows move the volume a twentieth.",
          self.frame.demo(
            0,
            bound,
            "MediaVolume::new(volume).muted(muted)",
            &[
              "// volume: Signal<f32>, muted: Signal<bool>",
              "MediaVolume::new(volume).muted(muted)",
            ],
          ),
        ),
        Section::new(
          "Levels",
          "The glyph is the loud one from a half up, the low one under it, and the crossed one while muted or at \
           nothing.",
          self.frame.demo(
            1,
            levels,
            "MediaVolume::controlled(0.2, |volume| ..)",
            &[
              "MediaVolume::controlled(0.8, |volume| set_volume(volume))",
              "MediaVolume::controlled(0.2, |volume| set_volume(volume))",
              "MediaVolume::controlled(0.8, |volume| set_volume(volume))",
              "    .muted_controlled(true, |muted| set_muted(muted))",
              "MediaVolume::controlled(0.0, |volume| set_volume(volume))",
              "MediaVolume::controlled(0.6, |volume| set_volume(volume)).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Collapsed",
          "Where a bar has no room for the slider: the button alone, and the slider beside it while the pointer is \
           over the control, the focus is in it or it is being dragged. Mounted, so that it keeps that between \
           renders.",
          self.frame.demo(
            2,
            collapsed,
            ".collapsed(true).mount(ctx)",
            &[
              "MediaVolume::new(volume)",
              "    .muted(muted)",
              "    .collapsed(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Over a picture",
          "In a video's bar: the glyph, the slider's range and its thumb white, the thumb 12, the rail white at \
           20%; the button white at 12% under the pointer and the ring of the focus white. The same in a light and \
           a dark theme, since the picture is not themed.",
          self.frame.demo(
            3,
            over,
            ".on_media(true)",
            &[
              "MediaVolume::new(volume)",
              "    .muted(muted)",
              "    .on_media(true)",
              "    .size(MediaVolumeSize::Lg)",
              "    .length(64.0)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a volume control is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
