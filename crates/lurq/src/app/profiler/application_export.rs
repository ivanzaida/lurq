//! Additive JSON export; existing frame/input/update arrays remain unchanged.
use serde_json::{Value, json};

use super::application_model::*;

pub(super) fn availability(enabled: bool) -> Value {
  json!({
    "available": enabled,
    "max_live_scopes": MAX_APPLICATION_LIVE_SCOPES,
    "max_samples_per_session": MAX_APPLICATION_SCOPES_PER_SESSION,
    "max_label_bytes": MAX_APPLICATION_LABEL_BYTES,
    "timing": "synchronous_inclusive_wall",
    "thread_cpu_time_available": false,
    "gpu_time_available": false
  })
}

fn info(scope: &ApplicationScopeInfo) -> Value {
  json!({
    "id": format!("app_scope_{}", scope.id.0),
    "parent_id": scope.parent_id.map(|id| format!("app_scope_{}", id.0)),
    "depth": scope.depth, "window": scope.window.as_ref(),
    "label": scope.label, "lane": scope.lane.name(), "started_ms": scope.started_ms
  })
}

impl ApplicationScopeReport {
  pub fn to_json(&self, observed_ms: f64) -> Value {
    let newest = self.samples.last().map(|sample| sample.completed_ms);
    let status = if !self.available {
      "feature_disabled"
    } else if !self.in_flight.is_empty() {
      "unfinished_work_observed"
    } else if !self.samples.is_empty() {
      "completed_scopes"
    } else if self.abandoned_window_closed > 0 || self.abandoned_producer_closed > 0 {
      "abandoned_work_observed"
    } else {
      "no_completed_scopes"
    };
    let refused = self.refused;
    json!({
      "availability": availability(self.available), "status": status,
      "max_live_scopes": self.max_live_scopes, "max_samples": self.max_samples,
      "started_scopes": self.started_scopes, "completed_scopes": self.completed_scopes,
      "returned_scopes": self.samples.len(), "dropped_scopes": self.dropped_scopes,
      "truncated": self.dropped_scopes > 0,
      "boundary_excluded_scopes": self.boundary_excluded_scopes,
      "abandoned_scopes": { "window_closed": self.abandoned_window_closed,
        "producer_closed": self.abandoned_producer_closed },
      "refused_starts": { "invalid_label": refused.invalid_label,
        "window_unavailable": refused.window_unavailable, "collector_closed": refused.collector_closed,
        "live_limit": refused.live_limit, "invalid_parent": refused.invalid_parent },
      "sample_age_ms": newest.map(|completed| (observed_ms - completed).max(0.)),
      "in_flight": self.in_flight.iter().map(|live| json!({
        "scope": info(&live.scope), "elapsed_so_far_ms": live.elapsed_ms,
        "unfinished": true, "excluded_from_completed_scopes": true,
        "started_before_session": live.started_before_session
      })).collect::<Vec<_>>(),
      "samples": self.samples.iter().map(|sample| json!({
        "scope": info(&sample.scope), "completed_ms": sample.completed_ms,
        "wall_timings_ms": { "total": sample.wall.as_secs_f64() * 1000. }
      })).collect::<Vec<_>>(),
      "scope_semantics": "explicit inclusive synchronous wall spans; lock/I/O waits included; may overlap UI samples and other lanes; never sum parents/children or interpret as thread CPU/GPU time",
      "parent_policy": "same collector and live parent at child entry; parent may finish before child; IDs are temporal nesting references, not a guarantee of full containment",
      "boundary_policy": "whole completed scopes started within this session; unfinished observations and pre-start completions are excluded",
      "caller_rule": "drop guards before await; declared UI/worker lanes are not inferred OS thread identities"
    })
  }
}
