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
