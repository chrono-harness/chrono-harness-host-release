#[path = "../../host-release/src/registrations.rs"]
mod registrations;

use registrations::{Method, query};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn recipe_uses_stdin_and_preserves_action_evidence() {
    let projects = json!({"projects":[],"scripts":[]});
    let method = json!({"owner":"script:s","operation":"pack","tool":"packer","argv":["a b"],
                        "output_contract":{"schema":"output"}});
    let result: BTreeMap<String, Vec<Method>> =
        query("execution-methods", &projects, |bin, arg, input| {
            assert_eq!(
                (bin, arg),
                ("chrono-judge-registration", "execution-methods")
            );
            assert_eq!(serde_json::from_slice::<Value>(input).unwrap(), projects);
            Ok((
                0,
                serde_json::to_vec(&json!({"pack":[method]})).unwrap(),
                vec![],
            ))
        })
        .unwrap();
    assert_eq!(serde_json::to_value(&result["pack"][0]).unwrap(), method);
}

#[test]
fn preview_reports_rejected_invalid_and_unavailable_registration() {
    let projects = json!({});
    for (reply, expected) in [
        (
            Ok((2, vec![], b"invalid binding\n".to_vec())),
            "invalid binding",
        ),
        (
            Ok((7, vec![], vec![])),
            "chrono-judge-registration test-owners exited 7",
        ),
        (
            Ok((0, b"bad".to_vec(), vec![])),
            "invalid test-owners response",
        ),
        (Err("timeout".into()), "timeout"),
    ] {
        let result =
            query::<BTreeMap<String, Vec<String>>>("test-owners", &projects, |bin, arg, _| {
                assert_eq!((bin, arg), ("chrono-judge-registration", "test-owners"));
                reply
            });
        assert!(result.unwrap_err().starts_with(expected));
    }
}
