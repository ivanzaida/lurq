//! A range slider in a form: submitted as its two values under its name, validated, and reset.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseButton},
  components::{
    Column, FormHandle, FormOptions, FormPrimaryButton, FormPrimaryButtonProps, FormRangeSliderInput,
    FormRangeSliderInputProps, FormValues, Text, validators,
  },
  node::Element,
};

use crate::support::{pointer_click, run_pass};

/// What the test reads of the host: what was submitted, and the form.
#[derive(Clone, Default)]
struct Probe {
  submitted: Arc<Mutex<Option<FormValues>>>,
  form: Arc<Mutex<Option<FormHandle>>>,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Probe {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Probe {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.submitted, &other.submitted)
  }
}

struct PriceForm {
  form: FormHandle,
}

impl Component for PriceForm {
  type Props = Probe;

  fn create(ctx: &mut Ctx) -> Self {
    let probe = ctx.props::<Self::Props>().clone();
    let submitted = probe.submitted.clone();
    let form = ctx
      .form(
        FormOptions::new()
          .field("price", (20, 80))
          .validate_range("price", validators::range_span(0.0, 100.0, 10.0, "At least 10 apart")),
      )
      .on_submit(move |values| {
        *submitted.lock().unwrap() = Some(values);
      });
    *probe.form.lock().unwrap() = Some(form.clone());
    Self { form }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (low, high) = self.form.range("price").get();
    let shown = format!("price={low},{high}");
    ctx.form_view(self.form.clone(), |ctx| {
      Column::new()
        .width(200.0)
        .child(
          ctx.mount::<FormRangeSliderInput>(
            FormRangeSliderInputProps::new(self.form.range_control("price"))
              .label("Price")
              .range(0, 100),
          ),
        )
        .child(ctx.mount::<FormPrimaryButton>(FormPrimaryButtonProps::new("Save")))
        .child(Text::new(&shown))
    })
  }
}

fn key(tree: &mut Tree, key: &str) {
  tree.key_down(key.to_owned(), key.to_owned(), false, false, false);
  tree.key_up(key.to_owned(), key.to_owned(), false, false, false);
  run_pass(tree);
}

fn mounted() -> (Tree, Probe) {
  let probe = Probe::default();
  let mut tree = Tree::new();
  tree.mount_root::<PriceForm>(&mut App::new(), probe.clone());
  run_pass(&mut tree);
  (tree, probe)
}

fn save(tree: &mut Tree) {
  let save = tree
    .find_element(|element| element.text_content() == Some("Save"))
    .expect("the submit button")
    .bounds();
  pointer_click(tree, save.center().0, save.center().1, MouseButton::Left);
  run_pass(tree);
}

fn shows(tree: &mut Tree, text: &str) -> bool {
  tree
    .find_element(|element| element.text_content() == Some(text))
    .is_some()
}

#[test]
fn a_range_slider_submits_its_two_values_under_its_name() {
  let (mut tree, probe) = mounted();
  assert!(shows(&mut tree, "price=20,80"), "the form's default");
  assert!(shows(&mut tree, "Price"));

  // The low thumb up a page, the high one down a step.
  key(&mut tree, "Tab");
  key(&mut tree, "PageUp");
  key(&mut tree, "Tab");
  key(&mut tree, "ArrowLeft");
  assert!(shows(&mut tree, "price=30,79"), "the field follows the slider");

  save(&mut tree);
  let values = probe.submitted.lock().unwrap().clone().expect("the form submits");
  assert_eq!(values.get_range("price"), Some((30.0, 79.0)));
}

#[test]
fn a_range_slider_is_validated_and_an_invalid_one_is_not_submitted() {
  let (mut tree, probe) = mounted();

  // The low thumb to the high one: no span between them.
  key(&mut tree, "Tab");
  key(&mut tree, "End");
  assert!(shows(&mut tree, "price=80,80"));
  // Out of the slider: it is validated on losing the focus.
  key(&mut tree, "Tab");
  key(&mut tree, "Tab");
  assert!(shows(&mut tree, "At least 10 apart"));

  save(&mut tree);
  assert!(
    probe.submitted.lock().unwrap().is_none(),
    "an invalid form is not submitted"
  );
}

#[test]
fn a_reset_puts_both_thumbs_back() {
  let (mut tree, probe) = mounted();
  key(&mut tree, "Tab");
  key(&mut tree, "Home");
  key(&mut tree, "Tab");
  key(&mut tree, "End");
  assert!(shows(&mut tree, "price=0,100"));

  let form = probe.form.lock().unwrap().clone().unwrap();
  assert!(form.is_field_dirty("price"));
  form.reset();
  run_pass(&mut tree);
  assert!(shows(&mut tree, "price=20,80"));
  assert!(!form.is_field_dirty("price"));
  // The thumbs went back too: a step from the low one is a step from 20, and the high one is still at 80.
  key(&mut tree, "Tab");
  key(&mut tree, "Tab");
  key(&mut tree, "ArrowRight");
  assert!(shows(&mut tree, "price=21,80"));

  form.range("price").set((40.0, 60.0));
  run_pass(&mut tree);
  form.reset_field("price");
  run_pass(&mut tree);
  assert!(shows(&mut tree, "price=20,80"));
}
