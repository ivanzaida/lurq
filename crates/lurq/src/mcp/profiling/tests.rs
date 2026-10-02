use super::*;
use std::collections::HashSet;

fn shared() -> McpShared {
  McpShared::new(
    HashSet::from([Scope::Observe]),
    HashSet::new(),
    "test-token".into(),
    "test".into(),
    None,
  )
}

fn json(output: McpToolResult) -> Value {
  match output.unwrap() {
    McpToolOutput::Json(value) => value,
    _ => panic!("expected JSON"),
  }
}

#[test]
fn profiling_tools_are_observe_read_only_and_report_feature_availability() {
  let tools = registered_tools();
  assert_eq!(tools.len(), 3);
  assert!(tools.iter().all(|tool| tool.scope == Scope::Observe && tool.read_only));
  let availability = json(execute(&shared(), BuiltinTool::ProfileRead, &json!({})));
  assert_eq!(
    availability["build"]["features"]["perf_profile"],
    cfg!(feature = "perf_profile")
  );
  #[cfg(not(feature = "perf_profile"))]
  {
    assert_eq!(availability["status"], "feature_disabled");
    assert!(
      execute(&shared(), BuiltinTool::ProfileStart, &json!({}))
        .err()
        .unwrap()
        .contains("perf_profile")
    );
  }
}

#[test]
fn unavailable_profiling_tools_are_rechecked_without_tree_access() {
  let shared = shared();
  shared.remove_scope(&Scope::Observe);
  assert!(execute(&shared, BuiltinTool::ProfileRead, &json!({})).is_err());
  shared.add_scope(Scope::Observe);
  shared.deny_tool(TOOL_NAMES[1]);
  assert!(execute(&shared, BuiltinTool::ProfileRead, &json!({})).is_err());
  shared.allow_tool(TOOL_NAMES[1]);
  shared.set_enabled(false);
  assert!(execute(&shared, BuiltinTool::ProfileRead, &json!({})).is_err());
}

#[cfg(feature = "perf_profile")]
#[test]
fn profiling_mcp_independent_ids_validation_and_revocation_do_not_affect_host_sessions() {
  let shared = shared();
  let handle = shared.profiling.read().unwrap().clone();
  let host = handle.start(Default::default()).unwrap().id;
  let first = json(execute(&shared, BuiltinTool::ProfileStart, &json!({})))["id"].clone();
  let second = json(execute(&shared, BuiltinTool::ProfileStart, &json!({"max_samples":2})))["id"].clone();
  let ended = json(execute(&shared, BuiltinTool::ProfileEnd, &json!({"id":second})));
  assert_eq!(ended["finalized"], true);
  assert_eq!(ended["id"], second);
  assert_eq!(
    json(execute(&shared, BuiltinTool::ProfileRead, &json!({"id":first})))["finalized"],
    false
  );
  assert!(execute(&shared, BuiltinTool::ProfileEnd, &json!({"id":second})).is_err());
  assert!(execute(&shared, BuiltinTool::ProfileEnd, &json!({"id":"profile_999"})).is_err());
  assert!(
    execute(
      &shared,
      BuiltinTool::ProfileRead,
      &json!({"id":format!("profile_{}", host.0)})
    )
    .is_err()
  );
  for limit in [json!(0), json!(241), json!(-1), json!(1.5), json!("2")] {
    assert!(execute(&shared, BuiltinTool::ProfileStart, &json!({"max_samples":limit})).is_err());
  }
  shared.remove_scope(&Scope::Observe);
  assert!(shared.profile_sessions.lock().unwrap().is_empty());
  assert!(handle.read(host).is_ok());
  shared.add_scope(Scope::Observe);
  assert!(execute(&shared, BuiltinTool::ProfileRead, &json!({"id":first})).is_err());
  handle.end(host).unwrap();
}
