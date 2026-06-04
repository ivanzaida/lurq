use lurq::components::{FormHandle, FormOptions, validators};

#[test]
fn dirty_signal_tracks_field_changes_against_defaults() {
  let form = FormHandle::new(FormOptions::new().field("name", "Ada"));
  let name = form.string("name");
  let name_dirty = form.dirty("name");

  assert!(!form.is_dirty());
  assert!(!form.is_field_dirty("name"));
  assert!(!name_dirty.get());

  name.set("Grace".to_owned());

  assert!(form.is_dirty());
  assert!(form.is_field_dirty("name"));
  assert!(name_dirty.get());

  name.set("Ada".to_owned());

  assert!(!form.is_dirty());
  assert!(!form.is_field_dirty("name"));
  assert!(!name_dirty.get());
}

#[test]
fn touched_state_can_be_marked_and_cleared() {
  let form = FormHandle::new(FormOptions::new());
  let touched = form.touched("email");

  assert!(!form.is_touched());
  assert!(!form.is_field_touched("email"));
  assert!(!touched.get());

  form.mark_touched("email");

  assert!(form.is_touched());
  assert!(form.is_field_touched("email"));
  assert!(touched.get());

  form.clear_touched("email");

  assert!(!form.is_touched());
  assert!(!form.is_field_touched("email"));
  assert!(!touched.get());
}

#[test]
fn reset_restores_defaults_and_clears_form_state() {
  let form = FormHandle::new(
    FormOptions::new()
      .field("name", "Ada")
      .field("age", 36.0)
      .field("active", true),
  );
  let name = form.string("name");
  let age = form.number("age");
  let active = form.bool("active");
  let name_dirty = form.dirty("name");
  let name_touched = form.touched("name");
  let name_error = form.error("name");

  name.set("Grace".to_owned());
  age.set(40.0);
  active.set(false);
  form.mark_touched("name");
  form.set_error("name", "Invalid name");

  assert!(form.is_dirty());
  assert!(name_dirty.get());
  assert!(name_touched.get());
  assert_eq!(name_error.get().as_deref(), Some("Invalid name"));

  form.reset();

  assert_eq!(name.get(), "Ada");
  assert_eq!(age.get(), 36.0);
  assert!(active.get());
  assert!(!form.is_dirty());
  assert!(!form.is_touched());
  assert!(!name_dirty.get());
  assert!(!name_touched.get());
  assert_eq!(name_error.get(), None);
  assert!(form.errors().is_empty());
}

#[test]
fn reset_field_only_resets_that_field() {
  let form = FormHandle::new(
    FormOptions::new()
      .field("name", "Ada")
      .field("email", "ada@example.com"),
  );
  let name = form.string("name");
  let email = form.string("email");
  let name_dirty = form.dirty("name");
  let email_dirty = form.dirty("email");
  let name_touched = form.touched("name");

  name.set("Grace".to_owned());
  email.set("grace@example.com".to_owned());
  form.mark_touched("name");
  form.set_error("name", "Invalid name");

  form.reset_field("name");

  assert_eq!(name.get(), "Ada");
  assert_eq!(email.get(), "grace@example.com");
  assert!(!name_dirty.get());
  assert!(email_dirty.get());
  assert!(!name_touched.get());
  assert!(form.error("name").get().is_none());
  assert!(form.is_dirty());
}

#[test]
fn submit_marks_registered_fields_touched() {
  let form = FormHandle::new(FormOptions::new().field("email", "ada@example.com"));
  let touched = form.touched("email");

  form.submit(Default::default());

  assert!(form.is_field_touched("email"));
  assert!(touched.get());
}

#[test]
fn submit_attempted_signal_tracks_submit_and_reset() {
  let form = FormHandle::new(FormOptions::new());
  let submit_attempted = form.submit_attempted();

  assert!(!form.has_submit_attempted());
  assert!(!submit_attempted.get());

  form.submit(Default::default());

  assert!(form.has_submit_attempted());
  assert!(submit_attempted.get());

  form.reset();

  assert!(!form.has_submit_attempted());
  assert!(!submit_attempted.get());
}

#[test]
fn string_field_groups_value_validation_state_and_reset() {
  let form = FormHandle::new(
    FormOptions::new()
      .field("email", "ada@example.com")
      .validate_string("email", validators::required("Email is required")),
  );
  let email = form.string_field("email");
  let value = email.value();
  let dirty = email.dirty();
  let touched = email.touched();
  let error = email.error();

  assert_eq!(email.name(), "email");
  assert_eq!(value.get(), "ada@example.com");
  assert!(!email.is_dirty());
  assert!(!email.is_touched());

  value.set(String::new());
  email.mark_touched();

  assert!(email.is_dirty());
  assert!(dirty.get());
  assert!(email.is_touched());
  assert!(touched.get());
  assert!(!email.validate());
  assert_eq!(error.get().as_deref(), Some("Email is required"));

  email.reset();

  assert_eq!(value.get(), "ada@example.com");
  assert!(!dirty.get());
  assert!(!touched.get());
  assert_eq!(error.get(), None);
}

#[test]
fn typed_field_constructors_reuse_form_signals() {
  let form = FormHandle::new(FormOptions::new().field("age", 36.0).field("active", true));
  let age = form.number_field("age");
  let active = form.bool_field("active");

  age.value().set(40.0);
  active.value().set(false);

  assert_eq!(form.number("age").get(), 40.0);
  assert!(!form.bool("active").get());
  assert!(age.is_dirty());
  assert!(active.is_dirty());
}
