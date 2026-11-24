use chrono_host_release::{bootstrap, committed::Committed, execution::Environment};
use serde_json::{Value, json};
use std::{ffi::OsString, fs, os::unix::ffi::OsStringExt, path::Path};

fn mock(root: &Path, name: &str) -> Environment {
    fs::copy(
        env!("CARGO_BIN_EXE_chrono-release-query-mock"),
        root.join(name),
    )
    .unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_chrono-release-query-mock"),
        root.join("chrono-harness"),
    )
    .unwrap();
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
    let releases = json!([
        {"draft":true, "tag_name":"v1.0.0-r", "prerelease":true, "published_at":"2026-03-01"},
        {"draft":false, "tag_name":"v1.0.0-a", "prerelease":true, "published_at":"2026-01-01"},
        {"draft":false, "tag_name":"v1.0.0-b", "prerelease":true, "published_at":"2026-01-01"},
        {"draft":false, "tag_name":"v1.0.01-a", "prerelease":true, "published_at":"2026-02-01"},
        {"draft":false, "tag_name":"v1.0.0", "prerelease":false, "published_at":"2026-01-05"},
        {"draft":false, "tag_name":"v1.0.0", "prerelease":false, "published_at":"2026-01-04"}
    ]);
    for (event, tag) in [
        (json!(null), "v1.0.01-a"),
        (json!(""), "v1.0.01-a"),
        (json!("v2.0.0"), "v2.0.0"),
    ] {
        assert_eq!(
            query(
                json!({"operation":"release-selection","event_tag":event,"milestones":["v1.0.0","v2.0.0"],"releases":releases})
            ),
            {
                let mut titles = json!({"v1.0.0":{"final":5,"preview":1},"v2.0.0":{"final":null,"preview":null}});
                if tag == "v1.0.01-a" {
                    titles["v1.0.01"] = json!({"final":null,"preview":3});
                }
                json!({"tag":tag,"milestones":titles})
            }
        );
    }
    assert_eq!(
        query(json!({"operation":"release-selection","milestones":[],"releases":[]})),
        json!({"tag":null,"milestones":{}})
    );
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

#[test]
fn roadmap_release_block_preserves_text_outside_the_first_pair() {
    use chrono_host_release::verified_recipe::roadmap::run;
    let begin = "<!-- chrono-harness release -->";
    let end = "<!-- /chrono-harness release -->";
    let block = format!("{begin}\n测试版 🍃\n正式版\n{end}");
    let request = |original: &str, lines: &[&str], output: &mut Vec<u8>| {
        let body =
            json!({"operation":"release-block","milestone":7,"original":original,"lines":lines});
        run(serde_json::to_vec(&body).unwrap().as_slice(), output)
    };
    for (original, lines, expected) in [
        (String::new(), vec![], format!("{begin}\n{end}")),
        (
            "前言\r\n".into(),
            vec!["测试版 🍃", "正式版"],
            format!("前言\r\n\n\n{block}"),
        ),
        (
            format!("🍃{begin}{begin}old{end}尾{begin}原样{end}"),
            vec!["测试版 🍃", "正式版"],
            format!("🍃{block}尾{begin}原样{end}"),
        ),
        (block.clone(), vec!["测试版 🍃", "正式版"], block.clone()),
    ] {
        let mut output = Vec::new();
        assert_eq!(request(&original, &lines, &mut output).unwrap(), 0);
        assert_eq!(serde_json::from_slice::<String>(&output).unwrap(), expected);
    }
    for original in [
        begin.to_string(),
        end.to_string(),
        format!("{end}{begin}{end}"),
    ] {
        let mut output = Vec::new();
        assert_eq!(
            request(&original, &[], &mut output).unwrap_err(),
            "Incomplete release block in milestone 7"
        );
        assert!(output.is_empty());
    }
}

#[test]
fn installation_is_bound_to_this_launchers_entire_stdout_and_the_installed_bytes() {
    use chrono_host_release::{
        delivery::verify_installation_stdout,
        identity::{Channel, Identity},
    };
    use sha2::{Digest, Sha256};
    use std::{collections::BTreeSet, os::unix::fs::PermissionsExt};
    let dir = tempfile::tempdir().unwrap();
    let host = dir.path();
    let rel = ".chrono-harness/bin/tool";
    let file = host.join(rel);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"tool").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    let hash = format!("{:x}", Sha256::digest(b"tool"));
    let members = BTreeSet::from(["tool".to_string()]);
    let tools = json!({"tool":{"sha256":hash,"size":4}});
    let identity = Identity {
        channel: Channel::Build,
        version: "v1.2.3".into(),
        source_commit: "a".repeat(40),
        source_tree: "b".repeat(40),
        verified_ci: None,
    };
    let adopted = json!({"version":identity.version,"lock":".chrono-harness/distribution.json","bootstrap":".chrono-harness/install.sh"});
    let report = json!({"schema":"chrono-install-result/v1","version":identity.version,"source_commit":identity.source_commit,"source_tree":identity.source_tree,"platform":"macos-aarch64","installed":[{"name":"tool","size":4,"sha256":hash,"path":rel}]});
    let verify = |channel, input: &str| {
        verify_installation_stdout(
            input.as_bytes(),
            host,
            &Identity {
                channel,
                ..identity.clone()
            },
            "macos-aarch64",
            &members,
            tools.as_object().unwrap(),
        )
    };
    let one = report.to_string();
    let both = format!("{adopted}\n{report}\n");
    assert_eq!(verify(Channel::Build, &one).unwrap(), report);
    assert_eq!(verify(Channel::Preview, &both).unwrap(), report);
    assert_eq!(verify(Channel::Stable, &both).unwrap(), report);
    fs::create_dir_all(host.join(".chrono-harness/state")).unwrap();
    fs::write(host.join(".chrono-harness/state/distribution.json"), &one).unwrap();
    for (channel, input) in [
        (Channel::Build, "".into()),
        (Channel::Build, both.clone()),
        (Channel::Stable, one.clone()),
        (Channel::Stable, format!("{report}\n{adopted}")),
        (Channel::Preview, both.clone() + "extra"),
        (Channel::Preview, format!("{{}}\n{report}")),
        (Channel::Build, format!("{{\"schema\":null,{}", &one[1..])),
    ] {
        assert!(verify(channel, &input).is_err(), "{input}");
    }
    for (field, other) in [
        ("/schema", json!("unknown")),
        ("/version", json!("v2.0.0")),
        ("/source_commit", json!("c".repeat(40))),
        ("/source_tree", json!("d".repeat(40))),
        ("/platform", json!("other")),
        ("/installed/0/path", json!("elsewhere")),
        ("/installed/0/sha256", json!("f".repeat(64))),
        ("/installed/0/size", json!(0)),
        ("/installed/0/name", json!("different")),
        ("/installed", json!([])),
    ] {
        let mut changed = report.clone();
        *changed.pointer_mut(field).unwrap() = other;
        assert!(
            verify(Channel::Build, &changed.to_string()).is_err(),
            "{field}"
        );
    }
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(verify(Channel::Preview, &both).is_err());
    fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(&file, b"changed").unwrap();
    assert!(verify(Channel::Build, &one).is_err());
}
