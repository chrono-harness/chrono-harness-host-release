use chrono_host_release::{
    execution::Environment,
    identity::{Channel, VerifiedCi, resolve},
};

const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TREE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn delivery(channel: &str) -> Environment {
    [
        ("CHRONO_RELEASE_CHANNEL", channel),
        ("CHRONO_RELEASE_SOURCE", SOURCE),
        ("CHRONO_RELEASE_CI_RUN_ID", "1234"),
        ("CHRONO_RELEASE_CI_ATTEMPT", "2"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}

#[test]
fn ordinary_build_uses_the_plan_and_observed_source() {
    let result = resolve("v0.1.0-beta.29", SOURCE, TREE, &Environment::new()).unwrap();
    assert_eq!(result.channel, Channel::Build);
    assert_eq!(result.version, "v0.1.0-beta.29");
    assert_eq!(result.source_commit, SOURCE);
    assert_eq!(result.source_tree, TREE);
    assert_eq!(result.verified_ci, None);
}

#[test]
fn preview_version_is_deterministic_and_binds_the_full_source_and_ci_attempt() {
    let environment = delivery("preview");
    let result = resolve("v0.1.0-beta.29", SOURCE, TREE, &environment).unwrap();
    assert_eq!(result.channel, Channel::Preview);
    assert_eq!(result.version, format!("v0.1.0-dev.1234.2.g{SOURCE}"));
    assert_eq!(
        result.verified_ci,
        Some(VerifiedCi {
            run_id: 1234,
            attempt: 2
        })
    );
    assert_eq!(
        result,
        resolve("v0.1.0-beta.29", SOURCE, TREE, &environment).unwrap()
    );
    for (key, value) in [
        ("CHRONO_RELEASE_CI_RUN_ID", "1235"),
        ("CHRONO_RELEASE_CI_ATTEMPT", "3"),
    ] {
        let mut changed = environment.clone();
        changed.insert(key.into(), value.into());
        assert_ne!(
            result.version,
            resolve("v0.1.0-beta.29", SOURCE, TREE, &changed)
                .unwrap()
                .version
        );
    }
    let other = "cccccccccccccccccccccccccccccccccccccccc";
    let mut changed = environment;
    changed.insert("CHRONO_RELEASE_SOURCE".into(), other.into());
    assert_ne!(
        result.version,
        resolve("v0.1.0-beta.29", other, TREE, &changed)
            .unwrap()
            .version
    );
}

#[test]
fn stable_requires_an_explicit_matching_final_version() {
    let mut environment = delivery("stable");
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v1.2.3".into());
    let result = resolve("v1.2.3", SOURCE, TREE, &environment).unwrap();
    assert_eq!(result.channel, Channel::Stable);
    assert_eq!(result.version, "v1.2.3");
    assert_eq!(result.verified_ci.unwrap().run_id, 1234);
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v1.2.4".into());
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v1.2.3-beta.1".into());
    assert!(resolve("v1.2.3-beta.1", SOURCE, TREE, &environment).is_err());
}

#[test]
fn published_channels_reject_missing_mismatched_or_ambiguous_identity() {
    for key in [
        "CHRONO_RELEASE_SOURCE",
        "CHRONO_RELEASE_CI_RUN_ID",
        "CHRONO_RELEASE_CI_ATTEMPT",
    ] {
        let mut environment = delivery("preview");
        environment.remove(std::ffi::OsStr::new(key));
        assert!(
            resolve("v1.2.3", SOURCE, TREE, &environment).is_err(),
            "{key}"
        );
    }
    for key in ["CHRONO_RELEASE_CI_RUN_ID", "CHRONO_RELEASE_CI_ATTEMPT"] {
        for value in ["0", "01", "-1", "1x", "18446744073709551616"] {
            let mut environment = delivery("preview");
            environment.insert(key.into(), value.into());
            assert!(
                resolve("v1.2.3", SOURCE, TREE, &environment).is_err(),
                "{key}={value}"
            );
        }
    }
    let mut environment = delivery("preview");
    environment.insert("CHRONO_RELEASE_SOURCE".into(), "dev".into());
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    environment = delivery("preview");
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v9.9.9".into());
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    assert!(resolve("v1.2.3", SOURCE, TREE, &delivery("unknown")).is_err());
    for version in ["1.2.3", "v1.2", "v01.2.3", "v1.two.3", "v1.2.3.4"] {
        assert!(
            resolve(version, SOURCE, TREE, &delivery("preview")).is_err(),
            "{version}"
        );
    }
}

#[test]
fn source_observation_must_be_complete_lowercase_oids() {
    for source in ["dev", "a", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"] {
        assert!(resolve("v1.2.3", source, TREE, &Environment::new()).is_err());
        assert!(resolve("v1.2.3", SOURCE, source, &Environment::new()).is_err());
    }
}

const WORKFLOW: &str = ".github/workflows/chrono-release.yml";
const WORKFLOW_COMMIT: &str = "cccccccccccccccccccccccccccccccccccccccc";
const BLOB: &str = "dddddddddddddddddddddddddddddddddddddddd";

fn workflow_environment(commit: &str) -> Environment {
    [
        ("GITHUB_REPOSITORY", "owner/repository"),
        ("GITHUB_WORKFLOW_SHA", commit),
        (
            "GITHUB_WORKFLOW_REF",
            "owner/repository/.github/workflows/chrono-release.yml@refs/heads/dev",
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}

#[derive(Default)]
struct WorkflowCommands {
    replies: std::collections::VecDeque<Result<chrono_host_release::publication::Reply, String>>,
    calls: Vec<(String, Vec<String>)>,
}

impl WorkflowCommands {
    fn blob(&mut self, blob: &str) {
        self.replies
            .push_back(Ok(chrono_host_release::publication::Reply {
                exit: 0,
                stdout: format!("{blob}\n").into_bytes(),
            }));
    }
}

impl chrono_host_release::publication::Commands for WorkflowCommands {
    fn execute(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<chrono_host_release::publication::Reply, String> {
        self.calls.push((program.into(), args.to_vec()));
        self.replies.pop_front().expect("unexpected command")
    }

    fn body(&mut self, _: &serde_json::Value) -> Result<String, String> {
        panic!("workflow identity must not write an API request")
    }
}

fn expected_blob_query(commit: &str) -> (String, Vec<String>) {
    (
        "git".into(),
        vec![
            "--no-replace-objects".into(),
            "rev-parse".into(),
            "--verify".into(),
            "--end-of-options".into(),
            format!("{commit}:{WORKFLOW}"),
        ],
    )
}

#[test]
fn workflow_from_the_source_commit_needs_only_one_fixed_blob_read() {
    let mut commands = WorkflowCommands::default();
    commands.blob(BLOB);
    let result = chrono_host_release::identity::verify_workflow(
        WORKFLOW,
        SOURCE,
        &workflow_environment(SOURCE),
        &mut commands,
    )
    .unwrap();
    assert_eq!(result.workflow_commit, SOURCE);
    assert_eq!(result.source_commit, SOURCE);
    assert_eq!(result.path, WORKFLOW);
    assert_eq!(result.blob, BLOB);
    assert_eq!(commands.calls, vec![expected_blob_query(SOURCE)]);
}

#[test]
fn unrelated_branch_progress_is_allowed_when_the_workflow_blob_is_unchanged() {
    let mut commands = WorkflowCommands::default();
    commands.blob(BLOB);
    commands.blob(BLOB);
    let result = chrono_host_release::identity::verify_workflow(
        WORKFLOW,
        SOURCE,
        &workflow_environment(WORKFLOW_COMMIT),
        &mut commands,
    )
    .unwrap();
    assert_eq!(result.workflow_commit, WORKFLOW_COMMIT);
    assert_eq!(result.source_commit, SOURCE);
    assert_eq!(result.blob, BLOB);
    assert_eq!(
        commands.calls,
        vec![
            expected_blob_query(WORKFLOW_COMMIT),
            expected_blob_query(SOURCE)
        ]
    );
}

#[test]
fn changed_workflow_is_rejected_with_both_fixed_identities() {
    let mut commands = WorkflowCommands::default();
    commands.blob(BLOB);
    commands.blob(TREE);
    let error = chrono_host_release::identity::verify_workflow(
        WORKFLOW,
        SOURCE,
        &workflow_environment(WORKFLOW_COMMIT),
        &mut commands,
    )
    .unwrap_err();
    for expected in [
        "E_RELEASE_IDENTITY",
        "workflow",
        WORKFLOW_COMMIT,
        SOURCE,
        WORKFLOW,
    ] {
        assert!(error.contains(expected), "{error}");
    }
    assert_eq!(commands.calls.len(), 2);
}

#[test]
fn missing_or_mismatched_github_bindings_fail_before_any_command() {
    for key in [
        "GITHUB_REPOSITORY",
        "GITHUB_WORKFLOW_SHA",
        "GITHUB_WORKFLOW_REF",
    ] {
        let mut environment = workflow_environment(WORKFLOW_COMMIT);
        environment.remove(std::ffi::OsStr::new(key));
        let mut commands = WorkflowCommands::default();
        assert!(
            chrono_host_release::identity::verify_workflow(
                WORKFLOW,
                SOURCE,
                &environment,
                &mut commands,
            )
            .is_err(),
            "{key}"
        );
        assert!(commands.calls.is_empty());
    }
    for (key, value) in [
        ("GITHUB_WORKFLOW_SHA", "dev"),
        (
            "GITHUB_WORKFLOW_SHA",
            "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        ),
        ("GITHUB_REPOSITORY", "other/repository"),
        (
            "GITHUB_WORKFLOW_REF",
            "owner/repository/.github/workflows/other.yml@refs/heads/dev",
        ),
        (
            "GITHUB_WORKFLOW_REF",
            "owner/repository/.github/workflows/chrono-release.yml@",
        ),
    ] {
        let mut environment = workflow_environment(WORKFLOW_COMMIT);
        environment.insert(key.into(), value.into());
        let mut commands = WorkflowCommands::default();
        assert!(
            chrono_host_release::identity::verify_workflow(
                WORKFLOW,
                SOURCE,
                &environment,
                &mut commands,
            )
            .is_err(),
            "{key}={value}"
        );
        assert!(commands.calls.is_empty());
    }
    for path in [
        "../workflow.yml",
        ".github/workflows/../other.yml",
        "workflow.yml",
        "",
    ] {
        let mut commands = WorkflowCommands::default();
        assert!(
            chrono_host_release::identity::verify_workflow(
                path,
                SOURCE,
                &workflow_environment(WORKFLOW_COMMIT),
                &mut commands,
            )
            .is_err(),
            "{path}"
        );
        assert!(commands.calls.is_empty());
    }
}

#[test]
fn git_failure_timeout_and_incomplete_blob_observations_cannot_pass() {
    use chrono_host_release::publication::Reply;
    for reply in [
        Err("original Git timed out".into()),
        Ok(Reply {
            exit: 128,
            stdout: format!("{BLOB}\n").into_bytes(),
        }),
        Ok(Reply {
            exit: -9,
            stdout: format!("{BLOB}\n").into_bytes(),
        }),
        Ok(Reply {
            exit: 0,
            stdout: vec![],
        }),
        Ok(Reply {
            exit: 0,
            stdout: vec![255],
        }),
        Ok(Reply {
            exit: 0,
            stdout: format!("{BLOB}\n{BLOB}\n").into_bytes(),
        }),
        Ok(Reply {
            exit: 0,
            stdout: format!("{BLOB}").into_bytes(),
        }),
        Ok(Reply {
            exit: 0,
            stdout: b"HEAD\n".to_vec(),
        }),
    ] {
        let mut commands = WorkflowCommands::default();
        commands.replies.push_back(reply);
        assert!(
            chrono_host_release::identity::verify_workflow(
                WORKFLOW,
                SOURCE,
                &workflow_environment(WORKFLOW_COMMIT),
                &mut commands,
            )
            .is_err()
        );
        assert_eq!(commands.calls, vec![expected_blob_query(WORKFLOW_COMMIT)]);
    }
}

#[test]
fn release_identifiers_and_digests_keep_their_distinct_wire_formats() {
    use chrono_host_release::verified_recipe::{delivery, same_text};

    for width in [39, 40, 41, 63, 64, 65] {
        let nonzero = format!("{}a", "0".repeat(width - 1));
        let zero = "0".repeat(width);
        assert_eq!(
            delivery::valid_release_oid(nonzero.as_bytes()),
            width == 40 || width == 64
        );
        assert!(!delivery::valid_release_oid(zero.as_bytes()));
        assert_eq!(delivery::valid_digest(&zero), width == 64);
        assert_eq!(delivery::valid_digest(&nonzero), width == 64);
    }
    for bad in ["A", "g", ":", "\0"] {
        let value = format!("{}{bad}", "0".repeat(63));
        assert!(!delivery::valid_release_oid(value.as_bytes()));
        assert!(!delivery::valid_digest(&value));
    }
    for job in ["a.B_1-2", ".", ".."] {
        assert!(delivery::job_id(job.as_bytes()));
    }
    for bad in ["", "/", "é", "a b"] {
        assert!(!delivery::job_id(bad.as_bytes()));
    }
    assert!(!delivery::name(b"."));
    assert!(!delivery::name(b".."));
    assert!(same_text("原始回执\0", "原始回执\0"));
    assert!(!same_text("é", "e\u{301}"));
}

#[test]
fn source_release_inventory_selects_the_largest_public_canonical_version() {
    use chrono_host_release::template_run::latest_stable;
    use serde_json::json;

    let row = |tag: &str, draft, preview, date: Option<&str>| json!({"tag_name":tag,"draft":draft,"prerelease":preview,"published_at":date});
    let select = |rows: Vec<serde_json::Value>| {
        latest_stable(serde_json::to_vec(&rows).unwrap().as_slice()).unwrap()
    };
    let mut rows = vec![
        row("v9.9.9", true, false, Some("date")),
        row("v8.0.0", false, true, Some("date")),
        row("v7.0.0", false, false, None),
        row("v6.0.0", false, false, Some("")),
    ];
    for invalid in [
        "8.0.0",
        "V8.0.0",
        "v09.0.0",
        "v1.2",
        "v1.2.3.4",
        "v1.2.3+build",
        "v1.2.3-rc.1",
        "v１.2.3",
        "v1.2.3\n",
    ] {
        rows.push(row(invalid, false, false, Some("date")));
    }
    assert_eq!(select(Vec::new()), None);
    assert_eq!(select(rows.clone()), None);
    let large = format!("v2.{}.0", "9".repeat(5000));
    let larger = format!("v2.1{}.0", "0".repeat(5000));
    for tag in ["v1.99.999", "v2.0.0", &larger, &large, &larger] {
        rows.push(row(tag, false, false, Some("date")));
    }
    assert_eq!(select(rows.clone()).as_deref(), Some(larger.as_str()));
    rows.reverse();
    assert_eq!(select(rows).as_deref(), Some(larger.as_str()));
}

#[test]
fn draft_notes_use_the_full_original_github_body() {
    use chrono_host_release::template_run::publication;
    use serde_json::{Value, json};

    let expected = "Build origins\n\n```json\n{}\n```";
    for (release, matches) in [
        (json!({"body": expected}), true),
        (json!({"body": format!("{expected}\n")}), false),
        (json!({"body": null}), false),
        (json!({"body": 1}), false),
        (json!({}), false),
    ] {
        let request = json!({"kind":"notes","expected":expected,"release":release});
        let result = publication(serde_json::to_vec(&request).unwrap().as_slice()).unwrap();
        assert_eq!(result, Value::Bool(matches));
    }
}

#[test]
fn consumer_acceptance_checks_the_last_matching_row_from_the_requested_attempt() {
    use chrono_host_release::template_run::acceptance;
    use serde_json::json;
    let sha = "a".repeat(40);
    let accepted = json!({"name":"consumer","head_sha":sha,"conclusion":"success",
        "steps":[{"name":"exercise","conclusion":"success"}]});
    let mut rejected = accepted.clone();
    rejected["conclusion"] = json!("failure");
    let unrelated = json!({"name":"consumer-extra"});
    for (jobs, success) in [
        (json!([rejected, accepted, unrelated]), true),
        (json!([accepted, rejected, unrelated]), false),
        (json!([unrelated, {"name":1}]), false),
    ] {
        let request = json!({"sha":sha,"step":"exercise","required_jobs":["consumer"],"jobs":jobs});
        assert_eq!(
            acceptance(serde_json::to_vec(&request).unwrap().as_slice()).is_ok(),
            success
        );
    }
}

#[test]
fn platform_origins_reuse_only_the_earlier_successful_build_of_the_same_run() {
    use chrono_host_release::template_run::binary_origins;
    use serde_json::{Value, json};
    let sha = "a".repeat(40);
    let workflow_sha = "b".repeat(40);
    let repo = "owner/release";
    let build = |platform: &str, attempt: u64, id: u64| {
        let file = "c".repeat(64);
        let archive = "d".repeat(64);
        json!({
            "selected_run":5,"attempt":attempt,"artifact_id":id,"archive":archive,
            "file":file,"downloaded":file,
            "run":{"id":5,"run_attempt":attempt,"repository":{"full_name":repo},
                "head_repository":{"full_name":repo},"event":"workflow_run","path":".github/workflows/release.yml",
                "head_branch":"dev","head_sha":workflow_sha},
            "jobs":[{"id":id+100,"run_id":5,"run_attempt":attempt,"name":format!("Build {platform}"),
                "conclusion":"success","head_sha":workflow_sha,"steps":[
                    {"name":"Build and exercise the release binary","conclusion":"success"},
                    {"name":"Upload the exercised binary","conclusion":"success"}]}],
            "artifact":{"id":id,"expired":false,"size_in_bytes":1,"name":format!("{platform}-{sha}-{attempt}"),
                "workflow_run":{"id":5,"head_sha":workflow_sha},"digest":format!("sha256:{archive}")}
        })
    };
    let valid = json!({"repository":repo,"binary":"product","source":sha,"workflow":"check.yml",
        "check_id":3,"check_attempt":2,"check":{"id":3,"run_attempt":2,"repository":{"full_name":repo},
        "head_repository":{"full_name":repo},"head_branch":"dev","head_sha":sha,"event":"push",
        "path":".github/workflows/check.yml","status":"completed","conclusion":"success"},
        "workflow_sha":workflow_sha,"run_id":5,"current_attempt":2,
        "macos":build("macos-aarch64",2,21),"linux":build("linux-x86_64",1,20)});
    let submit = |v: &Value| binary_origins(serde_json::to_vec(v).unwrap().as_slice());
    let result = submit(&valid).unwrap();
    assert_eq!(result["builds"]["product-linux-x86_64"]["attempt"], 1);
    assert_eq!(result["builds"]["product-macos-aarch64"]["job"], 121);
    let built = valid["linux"]["jobs"][0].clone();
    let mut earlier = built.clone();
    earlier["id"] = json!(888);
    let mut duplicates = valid.clone();
    duplicates["linux"]["jobs"] = json!([earlier, built]);
    assert_eq!(
        submit(&duplicates).unwrap()["builds"]["product-linux-x86_64"]["job"],
        120
    );
    duplicates["linux"]["jobs"] = json!([built, earlier]);
    assert_eq!(
        submit(&duplicates).unwrap()["builds"]["product-linux-x86_64"]["job"],
        888
    );
    for (path, bad) in [
        ("/linux", valid["macos"].clone()),
        ("/check/id", json!(4)),
        ("/check/run_attempt", json!(1)),
        ("/check/head_sha", json!(workflow_sha)),
        ("/linux/attempt", json!(3)),
        ("/linux/selected_run", json!(4)),
        ("/linux/run/repository/full_name", json!("another/repo")),
        ("/linux/run/event", json!("workflow_dispatch")),
        ("/linux/run/run_attempt", json!(2)),
        ("/linux/jobs/0/head_sha", json!(sha)),
        ("/linux/jobs/0/run_attempt", json!(2)),
        ("/linux/jobs/0/steps/1/conclusion", json!("skipped")),
        ("/linux/artifact/id", json!(21)),
        (
            "/linux/artifact/digest",
            json!(format!("sha256:{}", "e".repeat(64))),
        ),
        ("/linux/downloaded", json!("a".repeat(64))),
        ("/linux/artifact/expired", json!(true)),
    ] {
        let mut changed = valid.clone();
        *changed.pointer_mut(path).unwrap() = bad;
        assert!(submit(&changed).is_err(), "{path}");
    }
}

#[test]
fn workflow_rejects_the_first_invalid_input_before_reading_a_git_blob() {
    use chrono_host_release::identity::verify_workflow;
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let mut env = workflow_environment(WORKFLOW_COMMIT);
    env.insert("GITHUB_WORKFLOW_REF".into(), OsString::from_vec(vec![0xff]));
    let mut commands = WorkflowCommands::default();
    for (path, expected) in [
        (
            "../x.yml",
            "E_RELEASE_IDENTITY: invalid relative path: \"../x.yml\"",
        ),
        (
            ".github/workflows/release.yaml",
            "E_RELEASE_IDENTITY: workflow path must be a declared GitHub workflow",
        ),
        (
            WORKFLOW,
            "E_RELEASE_IDENTITY: non UTF-8 GITHUB_WORKFLOW_REF",
        ),
    ] {
        assert_eq!(
            verify_workflow(path, "bad-oid", &env, &mut commands).unwrap_err(),
            expected
        );
        assert!(commands.calls.is_empty());
    }
    env.insert("GITHUB_WORKFLOW_REF".into(), "wrong/repository@v".into());
    assert_eq!(
        verify_workflow(WORKFLOW, "bad-oid", &env, &mut commands).unwrap_err(),
        "E_RELEASE_IDENTITY: missing or mismatched GitHub workflow identity"
    );
    assert!(commands.calls.is_empty());
}

#[test]
fn a_pushed_tag_checks_only_the_branch_required_by_its_exact_grammar() {
    use chrono_host_release::identity::check_pushed_tag;
    use std::collections::VecDeque;
    fn run(tag: &str, replies: &[(&[&str], i32, &str)]) -> Result<(), String> {
        let mut calls: VecDeque<_> = replies.iter().collect();
        let result = check_pushed_tag(tag, |args| {
            let (expected, status, stdout) = calls.pop_front().expect("unexpected Git invocation");
            assert_eq!(args, *expected);
            Ok((*status, (*stdout).into()))
        });
        assert!(calls.is_empty());
        result
    }
    let head: &[&str] = &["rev-parse", "HEAD"];
    let main: &[&str] = &["rev-parse", "refs/remotes/origin/main"];
    let ancestry: &[&str] = &[
        "merge-base",
        "--is-ancestor",
        SOURCE,
        "refs/remotes/origin/dev",
    ];
    for tag in ["v1.2.3", "v00.2.003"] {
        run(tag, &[(head, 0, SOURCE), (main, 0, SOURCE)]).unwrap();
        assert!(run(tag, &[(head, 0, SOURCE), (main, 0, TREE)]).is_err());
    }
    for tag in ["v1.2.3-alpha.1", "v1.2.3--", "v1.2.3-."] {
        run(tag, &[(head, 0, SOURCE), (ancestry, 0, "")]).unwrap();
        for code in [1, 128] {
            assert!(run(tag, &[(head, 0, SOURCE), (ancestry, code, "")]).is_err());
        }
    }
    for tag in [
        "",
        "v1.2",
        "v1.2.3-",
        "v1.2.3.4",
        "v1.2.3-a/b",
        "v.2.3",
        "1.2.3",
        "v1.2.3+build",
    ] {
        assert!(run(tag, &[]).is_err(), "{tag}");
    }
    assert!(run("v1.2.3", &[(head, 128, "")]).is_err());
    assert!(run("v1.2.3", &[(head, 0, "bad")]).is_err());
}

#[test]
fn template_version_policy_preserves_absent_and_malformed_fields() {
    use chrono_host_release::template_run::promotion;
    use serde_json::{Value, json};
    let evaluate = |config: Value| {
        let request = json!({"kind":"version-policy", "config":config});
        promotion(serde_json::to_vec(&request).unwrap().as_slice())
    };
    assert_eq!(evaluate(json!({"kind":"binary"})).unwrap(), false);
    assert_eq!(
        evaluate(json!({"kind":"binary","version_policy":"explicit"})).unwrap(),
        false
    );
    assert_eq!(
        evaluate(json!({"kind":"copier","version_policy":"next_patch"})).unwrap(),
        true
    );
    assert_eq!(
        evaluate(json!({"kind":"binary","version_policy":"next_patch"})).unwrap_err(),
        "next_patch requires a Copier source product"
    );
    for bad in [Value::Null, json!(true), json!(["next_patch"]), json!("")] {
        assert_eq!(
            evaluate(json!({"kind":"copier","version_policy":bad})).unwrap_err(),
            "unsupported version policy"
        );
    }
}

#[test]
fn template_event_draft_and_main_observations_drive_writes() {
    use chrono_host_release::template_run::{promotion, publication};
    use serde_json::{Value, json};
    let route = |event_name, event: Value| {
        let query = json!({"kind":"event","repository":"owner/project","event_name":event_name,"event":event});
        promotion(serde_json::to_vec(&query).unwrap().as_slice())
    };
    let event =
        json!({"repository":{"full_name":"owner/project"},"workflow_run":{"head_branch":"dev"}});
    assert_eq!(route("workflow_run", event.clone()).unwrap(), "dev");
    assert!(route("workflow_dispatch", event).is_err());
    let tag = |refs| {
        promotion(
            serde_json::to_vec(&json!({"kind":"tag","tag":"v1.2.0","refs":refs}))
                .unwrap()
                .as_slice(),
        )
        .unwrap()
    };
    assert_eq!(tag(json!([{"ref":"refs/tags/v1.2.0-alpha.1"}])), false);
    assert_eq!(tag(json!([{"ref":"refs/tags/v1.2.0"}])), true);
    let published = |q: Value| publication(serde_json::to_vec(&q).unwrap().as_slice());
    let mut q = json!({"kind":"draft","verify":false,"origins":true,"expected":"wanted","release":{"body":"other"}});
    assert_eq!(published(q.clone()).unwrap(), true);
    q["verify"] = json!(true);
    assert!(published(q).is_err());
    let source = "a".repeat(40);
    let mut q = json!({"kind":"main","source":source,"reference":{"ref":"refs/heads/main","object":{"type":"commit","sha":source}}});
    assert_eq!(published(q.clone()).unwrap(), false);
    q["reference"]["object"]["sha"] = json!("b".repeat(40));
    assert_eq!(published(q.clone()).unwrap(), true);
    q["reference"]["ref"] = json!("refs/heads/other");
    assert!(published(q).is_err());
}

#[test]
fn publication_preserves_provider_order_and_sorted_missing_names() {
    use chrono_host_release::template_run::publication;
    use serde_json::{Value, json};

    let sha = "a".repeat(40);
    let digest = "b".repeat(64);
    let decide = |rows: Value| {
        let request = json!({
            "kind":"state", "source":sha, "tag":"v1.2.0", "prerelease":false,
            "assets":{"z":digest,"tool":digest,"a":digest},
            "reference":{"ref":"refs/tags/v1.2.0","object":{"type":"commit","sha":sha}},
            "release":{"id":1,"tag_name":"v1.2.0","prerelease":false,"draft":true,"assets":rows}
        });
        publication(serde_json::to_vec(&request).unwrap().as_slice())
    };
    let asset = json!({"name":"tool","state":"uploaded","digest":format!("sha256:{digest}")});
    assert_eq!(
        decide(json!([asset])).unwrap(),
        json!({"action":"resume","create_tag":false,"missing":["a","z"]})
    );
    let bad_bytes =
        json!({"name":"tool","state":"uploaded","digest":format!("sha256:{}", "c".repeat(64))});
    for (rows, first_error) in [
        (
            json!([asset, asset, {}]),
            "unexpected or duplicate existing asset",
        ),
        (json!([{}, asset, asset]), "missing asset name"),
        (
            json!([bad_bytes, {"name":"other"}]),
            "existing release asset differs from selected bytes: tool",
        ),
        (
            json!([{"name":"other"}, bad_bytes]),
            "unexpected or duplicate existing asset",
        ),
    ] {
        assert_eq!(decide(rows).unwrap_err(), first_error);
    }
}

#[test]
fn installation_channels_resolve_the_selected_release_before_its_assets() {
    use chrono_host_release::template_run::resolve_install;
    use serde_json::{Value, json};
    let repository = "org/product";
    let digest = format!("sha256:{}", "7".repeat(64));
    let row = |tag: &str, preview: bool| {
        let assets = ["my-linux", "my-mac", "other-command"].map(|name| json!({
            "name":name,"state":"uploaded","digest":digest,
            "browser_download_url":format!("https://github.com/{repository}/releases/download/{tag}/{name}")
        }));
        json!({"tag_name":tag,"draft":false,"published_at":"2026-01-01T00:00:00Z","prerelease":preview,"assets":assets})
    };
    let call = |selector: &str, rows: &[Value]| {
        let request = json!({"repository":repository,"selector":selector,"assets":{"linux-x64":"my-linux","macos-arm64":"my-mac"},"releases":rows});
        resolve_install(serde_json::to_vec(&request).unwrap().as_slice())
    };
    let mut draft = row("v20.0.0", false);
    draft["draft"] = json!(true);
    let mut unpublished = row("v30.0.0", false);
    unpublished["published_at"] = Value::Null;
    let releases = vec![
        row("v2.0.0-beta.8", true),
        row("v2.0.0-alpha.22", true),
        row("v1.10.0", false),
        row("v2.0.0-rc.10", true),
        row("v2.0.0-rc.2", true),
        row("v1.9.99", false),
        row("v3.0.0-rc.01", true),
        row("v4.0.0-rc.1", false),
        row("v5.0.0", true),
        draft,
        unpublished,
    ];
    for (selector, tag) in [
        ("preview", "v2.0.0-rc.10"),
        ("stable", "v1.10.0"),
        ("v1.9.99", "v1.9.99"),
    ] {
        let result = call(selector, &releases).unwrap();
        assert_eq!(result["tag"], tag);
        assert_eq!(result["version"], &tag[1..]);
        assert_eq!(
            result["platforms"]["linux-x64"]["url"],
            format!("https://github.com/{repository}/releases/download/{tag}/my-linux")
        );
        assert_eq!(result["platforms"]["macos-arm64"]["checksum"], digest);
    }
    let dated = |tag: &str, preview, published: &str| {
        let mut release = row(tag, preview);
        release["published_at"] = json!(published);
        release
    };
    let mut previews = vec![
        dated("v0.1.0-alpha.24", true, "2026-11-25T08:00:54Z"),
        dated("v0.1.0-bootstrap.1", true, "2026-11-22T16:54:29Z"),
    ];
    assert_eq!(
        call("preview", &previews).unwrap()["tag"],
        "v0.1.0-alpha.24"
    );
    previews.insert(1, dated("v0.1.0-alpha.25", true, "2026-11-25T10:05:31Z"));
    assert_eq!(
        call("preview", &previews).unwrap()["tag"],
        "v0.1.0-alpha.25"
    );
    let stable = [
        dated("v2.0.0", false, "2026-11-24T00:00:00Z"),
        dated("v1.9.1", false, "2026-11-25T00:00:00Z"),
    ];
    assert_eq!(call("stable", &stable).unwrap()["tag"], "v2.0.0");
    assert!(call("v20.0.0", &releases).is_err());
    assert!(call("v0.0.1", &releases).is_err());
    assert!(call("stable", &[row("v2.0.0-alpha.7", true)]).is_err());
    let mut bad = row("v3.0.0", false);
    bad["assets"].as_array_mut().unwrap().remove(0);
    assert!(
        call("stable", &[row("v1.0.0", false), bad])
            .unwrap_err()
            .contains("missing asset my-linux")
    );
    let mut duplicate = row("v1.0.0", false);
    let assets = duplicate["assets"].as_array_mut().unwrap();
    assets.push(assets[1].clone());
    assert_eq!(
        call("v1.0.0", &[duplicate]).unwrap_err(),
        "v1.0.0: duplicate asset my-mac"
    );
    for (field, value) in [
        ("digest", "sha256:bad"),
        ("state", "new"),
        (
            "browser_download_url",
            "https://github.com/org/other/download",
        ),
    ] {
        let mut bad = row("v1.0.0", false);
        bad["assets"][1][field] = json!(value);
        assert!(call("v1.0.0", &[bad]).is_err());
    }
}

#[test]
fn mise_installation_reads_all_original_pages_and_emits_one_tool() {
    use chrono_host_release::template_run::resolve_mise_tool;
    use serde_json::json;
    let tmp = tempfile::tempdir().unwrap();
    let checksum = format!("sha256:{}", "f".repeat(64));
    let tag = "v5.4.3-beta.2";
    let prefix = format!("https://github.com/x/y/releases/download/{tag}");
    let assets=["one","two"].map(|a|json!({"name":a,"browser_download_url":format!("{prefix}/{a}"),"state":"uploaded","digest":checksum}));
    let raw = json!([{"tag_name":tag,"draft":false,"prerelease":true,"published_at":"2026-02-10","assets":assets}]);
    let first = tmp.path().join("first");
    let last = tmp.path().join("last");
    std::fs::write(&first, b"[]").unwrap();
    std::fs::write(&last, serde_json::to_vec(&raw).unwrap()).unwrap();
    let pages = [
        first.to_str().unwrap().to_owned(),
        last.to_str().unwrap().to_owned(),
    ];
    let actual = resolve_mise_tool("tool", "x/y", "preview", "one", "two", &pages).unwrap();
    assert_eq!(
        actual,
        format!(
            "[tools.\"http:tool\"]\nversion = \"5.4.3-beta.2\"\nbin = \"tool\"\n[tools.\"http:tool\".platforms]\nlinux-x64 = {{ url = \"{prefix}/one\", checksum = \"{checksum}\" }}\nmacos-arm64 = {{ url = \"{prefix}/two\", checksum = \"{checksum}\" }}\n"
        )
    );
    assert!(resolve_mise_tool("tool", "x/y", "stable", "one", "two", &pages).is_err());
}

#[test]
fn github_release_pagination_uses_the_final_complete_response() {
    use chrono_host_release::template_run::release_page_state;
    let next = "lInK: <https://api.github.com/items?page=3>; rel=\"next\", <https://api.github.com/items?page=9>; rel=\"last\"";
    assert!(release_page_state(&format!("HTTP/2 200\r\n{next}\r\n\r\n")).unwrap());
    assert!(!release_page_state(&format!("HTTP/1.1 100 Continue\r\n{next}\r\n\r\nHTTP/2 200\r\nLink: <https://api.github.com/items?page=3>; rel=\"prev\"\r\n\r\n")).unwrap());
    assert!(!release_page_state("HTTP/2 200\nContent-Type: application/json\n\n").unwrap());
    for bad in [
        "",
        "HTTP/2 429\n\n",
        "HTTP/bad 200\n\n",
        "HTTP/2 200\r\n",
        "HTTP/2 200\nLink: <target>; rel=\"next\"\n",
        "HTTP/2 200\nLink broken\n\n",
        "HTTP/2 200\nLink:\n\n",
        "HTTP/2 200\nLink: <url without terminator\n\n",
        "HTTP/2 200\nLink: <target>; rel=\"next\n\n",
        "HTTP/2 200\nLink: <target>; rel=\"prev\",\n\n",
    ] {
        assert!(release_page_state(bad).is_err(), "{bad:?}");
    }
}
