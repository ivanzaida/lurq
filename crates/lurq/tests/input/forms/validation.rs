use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::components::{
  FormData, FormErrors, FormHandle, FormOptions, FormValue, FormValues, ValidationResult, validators,
};

#[test]
fn invalid_submit_blocks_on_submit_and_updates_error_signal() {
  let submits = Arc::new(AtomicUsize::new(0));
  let invalid = Arc::new(Mutex::new(None::<FormErrors>));
  let form = FormHandle::new(FormOptions::new().validate_string("email", validators::required("Email is required")))
    .on_submit({
      let submits = submits.clone();
      move |_| {
        submits.fetch_add(1, Ordering::SeqCst);
      }
    })
    .on_invalid({
      let invalid = invalid.clone();
      move |errors| {
        *invalid.lock().unwrap() = Some(errors);
      }
    });
  let email_error = form.error("email");

  let mut data = FormData::new();
  data.append("email", "");
  form.submit(data);

  assert_eq!(submits.load(Ordering::SeqCst), 0);
  assert_eq!(email_error.get().as_deref(), Some("Email is required"));
  assert_eq!(
    invalid
      .lock()
      .unwrap()
      .as_ref()
      .and_then(|errors| errors.first("email")),
    Some("Email is required")
  );
}

#[test]
fn valid_submit_clears_errors_and_calls_on_submit() {
  let submitted = Arc::new(Mutex::new(None::<FormValues>));
  let form = FormHandle::new(
    FormOptions::new()
      .validate_string("email", validators::required("Email is required"))
      .validate_string("email", validators::email("Enter a valid email")),
  )
  .on_submit({
    let submitted = submitted.clone();
    move |values| {
      *submitted.lock().unwrap() = Some(values);
    }
  });
  let email_error = form.error("email");
  form.set_error("email", "Old error");

  let mut data = FormData::new();
  data.append("email", "ada@example.com");
  form.submit(data);

  assert_eq!(email_error.get(), None);
  assert!(form.errors().is_empty());
  assert_eq!(
    submitted
      .lock()
      .unwrap()
      .as_ref()
      .and_then(|values| values.get_string("email")),
    Some("ada@example.com")
  );
}

#[test]
fn validate_field_updates_only_that_field() {
  let form = FormHandle::new(
    FormOptions::new()
      .validate_string("email", validators::required("Email is required"))
      .validate_string("password", validators::required("Password is required")),
  );
  let email = form.string("email");
  let email_error = form.error("email");
  let password_error = form.error("password");

  assert!(!form.validate());
  assert_eq!(email_error.get().as_deref(), Some("Email is required"));
  assert_eq!(password_error.get().as_deref(), Some("Password is required"));

  email.set("ada@example.com".to_owned());

  assert!(form.validate_field("email"));
  assert_eq!(email_error.get(), None);
  assert_eq!(password_error.get().as_deref(), Some("Password is required"));
}

#[test]
fn number_validator_receives_optional_number() {
  let form = FormHandle::new(
    FormOptions::new()
      .field("age", FormValue::from(17.0))
      .validate_number("age", validators::range(18.0, 120.0, "Age must be at least 18")),
  );
  let age = form.number("age");
  let age_error = form.error("age");

  assert!(!form.validate());
  assert_eq!(age_error.get().as_deref(), Some("Age must be at least 18"));

  age.set(21.0);

  assert!(form.validate());
  assert_eq!(age_error.get(), None);
}

#[test]
fn custom_validator_can_compare_fields() {
  let form = FormHandle::new(
    FormOptions::new()
      .field("password", "secret123")
      .field("confirm", "secret")
      .validate_string("confirm", |confirm, values| {
        if values.get_string("password") == Some(confirm) {
          ValidationResult::valid()
        } else {
          ValidationResult::invalid("Passwords must match")
        }
      }),
  );
  let confirm = form.string("confirm");
  let confirm_error = form.error("confirm");

  assert!(!form.validate_field("confirm"));
  assert_eq!(confirm_error.get().as_deref(), Some("Passwords must match"));

  confirm.set("secret123".to_owned());

  assert!(form.validate_field("confirm"));
  assert_eq!(confirm_error.get(), None);
}

#[test]
fn set_errors_replaces_all_field_errors_and_updates_signals() {
  let form = FormHandle::new(FormOptions::new());
  let email_error = form.error("email");
  let password_error = form.error("password");

  form.set_error("email", "Old error");
  form.set_errors(
    FormErrors::new()
      .with("email", "Email already exists")
      .with("password", "Password is too short"),
  );

  assert_eq!(email_error.get().as_deref(), Some("Email already exists"));
  assert_eq!(password_error.get().as_deref(), Some("Password is too short"));
  assert_eq!(form.errors().message_count(), 2);
}

#[test]
fn set_field_errors_preserves_other_fields_and_uses_first_message_for_signal() {
  let form = FormHandle::new(FormOptions::new());
  let email_error = form.error("email");
  let password_error = form.error("password");

  form.set_error("password", "Password is too short");
  form.set_field_errors("email", ["Email already exists", "Email domain is blocked"]);

  assert_eq!(email_error.get().as_deref(), Some("Email already exists"));
  assert_eq!(password_error.get().as_deref(), Some("Password is too short"));

  let errors = form.errors();
  assert_eq!(errors.get("email").map(|messages| messages.len()), Some(2));
  assert_eq!(errors.message_count(), 3);
}

#[test]
fn clear_errors_for_removes_selected_fields() {
  let form = FormHandle::new(FormOptions::new());
  let email_error = form.error("email");
  let password_error = form.error("password");

  form.set_errors(
    FormErrors::new()
      .with("email", "Email already exists")
      .with("password", "Password is too short"),
  );
  form.clear_errors_for(["email"]);

  assert_eq!(email_error.get(), None);
  assert_eq!(password_error.get().as_deref(), Some("Password is too short"));
  assert_eq!(form.errors().first("email"), None);
  assert_eq!(form.errors().first("password"), Some("Password is too short"));
}
