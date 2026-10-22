//! The `Score` JSON boundary (plan 002 issue 11a): an issue-10-shaped file
//! still parses, validation is an `Err` and never a panic, and the new
//! mix/kind fields round-trip. These are the integration-level twins of
//! `score.rs`'s unit tests — they exercise the crate's PUBLIC surface
//! (`th_synth::score`) the way the page's `setScore` does.

use th_synth::director::{MusicDirector, SLOTS};
use th_synth::score::{kind_from_id, Score};

/// The issue-10 JSON shape: the canonical score with every 11a field
/// removed (that is exactly what issue 10's `to_json_pretty` emitted).
fn issue_10_json() -> String {
    let mut value = serde_json::to_value(Score::default()).expect("the default serializes");
    for slot in value["slots"].as_array_mut().expect("slots is an array") {
        let slot = slot.as_object_mut().expect("a slot is an object");
        slot.remove("kind");
        slot.remove("muted");
    }
    value
        .as_object_mut()
        .expect("the score is an object")
        .remove("solo");
    value.to_string()
}

#[test]
fn real_issue_10_json_parses_and_resolves_to_the_builtin_kinds() {
    let score = Score::from_json(&issue_10_json()).expect("an issue-10 score file parses");
    assert_eq!(score.solo, None, "solo defaults to null");
    for (i, slot) in score.slots.iter().enumerate() {
        assert!(slot.kind.is_none(), "slot {i} has no kind field");
        assert!(!slot.muted, "slot {i} defaults to unmuted");
    }
    // The director accepts it and keeps its 8 slots (the export path).
    let mut d = MusicDirector::new_fallback();
    d.set_score(score);
    let _ = d.tick(0.01);
    assert_eq!(d.score().slots.len(), 8);
    assert_eq!(SLOTS.len(), 8);
}

#[test]
fn validation_is_an_error_and_never_a_panic() {
    let mut v = serde_json::to_value(Score::default()).unwrap();
    v["slots"][3]["kind"] = serde_json::json!("nope");
    let err = Score::from_json(&v.to_string()).unwrap_err();
    assert!(
        err.contains("slots[3]") && err.contains("nope"),
        "the error names the slot and the bad id: {err}"
    );

    let mut v = serde_json::to_value(Score::default()).unwrap();
    v["solo"] = serde_json::json!(99);
    let err = Score::from_json(&v.to_string()).unwrap_err();
    assert!(err.contains("solo"), "the error names solo: {err}");

    // The solo bound is the slot count: 8 is out, 7 is in.
    let mut v = serde_json::to_value(Score::default()).unwrap();
    v["solo"] = serde_json::json!(8);
    assert!(Score::from_json(&v.to_string()).is_err(), "solo 8 rejected");
    let mut v = serde_json::to_value(Score::default()).unwrap();
    v["solo"] = serde_json::json!(7);
    assert!(Score::from_json(&v.to_string()).is_ok(), "solo 7 accepted");

    // Ids are exact (case-sensitive); garbage is None, never a panic.
    assert_eq!(kind_from_id("nope"), None);
    assert_eq!(kind_from_id(""), None);
    assert_eq!(kind_from_id("Heartbeat"), None, "ids are snake_case");

    // A future field is ignored, not fatal (forward compatibility).
    let mut v = serde_json::to_value(Score::default()).unwrap();
    v["future_field"] = serde_json::json!(true);
    assert!(Score::from_json(&v.to_string()).is_ok());
}

#[test]
fn muted_solo_and_kind_round_trip_through_json() {
    let mut s = Score::default();
    s.slots[2].muted = true;
    s.solo = Some(5);
    s.slots[3].kind = Some("bell".to_string());
    let back = Score::from_json(&s.to_json_pretty()).expect("round-trip");
    assert_eq!(back, s, "muted/solo/kind survive the bake text");
}
