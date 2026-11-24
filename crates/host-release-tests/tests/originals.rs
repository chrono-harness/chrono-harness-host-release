use chrono_host_release::{bootstrap, committed::Committed, execution::Environment};
use serde_json::{Value, json};
use std::{ffi::OsString, fs, os::unix::ffi::OsStringExt, path::Path};

fn mock(root: &Path, name: &str) -> Environment {
    fs::copy(
        env!("CARGO_BIN_EXE_chrono-release-query-mock"),
        root.join(name),
    )
    .unwrap();
    if name == "chrono-host-bootstrap" {
        fs::copy(
            env!("CARGO_BIN_EXE_chrono-release-query-mock"),
            root.join("chrono-harness"),
        )
        .unwrap();
    }
    [
        ("PATH".into(), root.as_os_str().to_owned()),
        ("GH_TOKEN".into(), "unused".into()),
        ("GIT_INDEX_FILE".into(), "outside".into()),
        ("UNRELATED".into(), OsString::from_vec(vec![255])),
    ]
    .into_iter()
    .collect()
}

fn replies(root: &Path, commands: &[(&[&str], Value)]) {
    let map: serde_json::Map<String, Value> = commands
        .iter()
        .map(|(args, reply)| (serde_json::to_string(args).unwrap(), reply.clone()))
        .collect();
    fs::write(root.join("mock.json"), serde_json::to_vec(&map).unwrap()).unwrap();
}

#[test]
fn report_query_preserves_stdin_diagnostics_and_isolates_unrelated_environment() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = mock(root, "chrono-host-bootstrap");
    let request = json!({"operation":"streams","report":"raw\nbytes"});
    for (exit, stdout, stderr, expected) in [
        (0, "[]", "", None),
        (0, "invalid", "", Some("expected value")),
        (2, "", "original changed\n", Some("original changed")),
    ] {
        replies(
            root,
            &[(
                &["original-report-v1"],
                json!({"stdin":request, "stdout":stdout,"stderr":stderr,"exit":exit}),
            )],
        );
        let result: Result<Vec<Value>, _> = bootstrap::query(root, &env, &request, 20);
        if let Some(expected) = expected {
            assert!(result.unwrap_err().contains(expected));
        } else {
            assert!(result.unwrap().is_empty());
        }
    }
}

#[test]
fn bootstrap_report_rejects_changed_or_incomplete_process_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = mock(root, "chrono-host-bootstrap");
    let request = json!({"operation":"streams","report":"bound"});
    for (fault, expected) in [
        ("digest", "stream identity differs"),
        ("child", "process identity differs"),
        ("launcher", "process identity differs"),
        ("truncated", "fill whole buffer"),
        ("trailing", "trailing bytes"),
    ] {
        replies(
            root,
            &[(
                &["original-report-v1"],
                json!({"stdin":request,"stdout":"[]","stderr":"","exit":0,"fault":fault}),
            )],
        );
        let result: Result<Vec<Value>, _> = bootstrap::query(root, &env, &request, 20);
        let error = result.unwrap_err();
        assert!(error.contains(expected), "{fault}: {error}");
    }
}

#[test]
fn release_reads_only_a_regular_file_from_the_requested_git_tree() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let env = mock(root, "git");
    let commit = "1".repeat(40);
    let tree = "2".repeat(40);
    let path = "inputs/[literal] with spaces";
    let revision = format!("{commit}^{{tree}}");
    let object = format!("{tree}:{path}");
    let flags = [
        "--no-optional-locks",
        "--no-replace-objects",
        "--literal-pathspecs",
    ];
    let rev = [flags.as_slice(), &["rev-parse", "--verify", &revision]].concat();
    let list = [
        flags.as_slice(),
        &["ls-tree", "--format=%(objectmode)", &tree, "--", path],
    ]
    .concat();
    let blob = [flags.as_slice(), &["cat-file", "blob", &object]].concat();
    replies(root, &[(&rev, json!({"stdout":format!("{commit}\n")}))]);
    assert!(Committed::new(root, &env, &commit, &tree).is_err());
    for mode in [
        "", "120000\n", "160000\n", "040000\n", "100644\n", "100755\n",
    ] {
        let mut commands = vec![
            (rev.as_slice(), json!({"stdout":format!("{tree}\n")})),
            (list.as_slice(), json!({"stdout":mode})),
        ];
        let regular = matches!(mode, "100644\n" | "100755\n");
        if regular {
            commands.push((blob.as_slice(), json!({"stdout":"fixed bytes\n"})));
        }
        replies(root, &commands);
        let reader = Committed::new(root, &env, &commit, &tree).unwrap();
        let read = reader.read(path);
        if regular {
            assert_eq!(read.unwrap(), b"fixed bytes\n");
        } else {
            assert!(read.unwrap_err().contains("not a regular file"));
        }
    }
}

#[test]
fn roadmap_queries_use_github_objects_and_return_the_owned_issue_number() {
    use chrono_host_release::verified_recipe::roadmap::run;
    let query = |request: Value| -> Value {
        let mut output = Vec::new();
        assert_eq!(
            run(
                serde_json::to_vec(&request).unwrap().as_slice(),
                &mut output
            )
            .unwrap(),
            0
        );
        serde_json::from_slice(&output).unwrap()
    };
    let marker = "<!-- chrono-harness release-run 123 -->";
    let bot = json!({"login":"github-actions[bot]"});
    let history = json!([
        {"number":1,"state":"open","user":{"login":"reporter"},"body":marker},
        {"number":2,"state":"open","user":bot,"body":marker,"pull_request":{}},
        {"number":3,"state":"open","user":bot,"body":"<!-- chrono-harness release-run 1230 -->"},
        {"number":4,"state":"open","user":bot,"body":format!("{marker}\n\noriginal run")}
    ]);
    assert_eq!(
        query(
            json!({"operation":"release-issue","status":"completed","conclusion":"success","run_id":123,"issues":history})
        ),
        json!({"action":"close","issue":4})
    );
    assert_eq!(
        query(json!({"operation":"release-run","status":"in_progress","conclusion":null})),
        json!(false)
    );
    assert_eq!(
        query(
            json!({"operation":"published","releases":[{"draft":false,"published_at":null},{"draft":true,"published_at":"time"},{"draft":false,"published_at":"time"}]})
        ),
        json!([2])
    );
    assert_eq!(
        query(
            json!({"operation":"unassigned","issues":[{"number":5,"state":"open","milestone":null,"author_association":"MEMBER"},{"number":6,"state":"open","milestone":{"number":1}}]})
        ),
        json!([5])
    );
    let mut output = Vec::new();
    assert!(
        run(
            br#"{"operation":"milestone","final_release":true,"open_issues":-1}"#.as_slice(),
            &mut output
        )
        .is_err()
    );
    assert!(output.is_empty());
}
