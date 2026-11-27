#[cfg(verus_keep_ghost)]
#[path = "../../host-release/src/verified_recipe.rs"]
mod policy;

#[cfg(verus_keep_ghost)]
use policy::command;

use policy::delivery;

use policy::{DependencyResult, Invocation};

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
mod originals;
#[cfg(verus_keep_ghost)]
mod query_tool;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/host-release/publication.rs"]
mod publication_fixture;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/host-release/recipe.rs"]
mod recipe;

#[path = "proof_support/host-release/support/execution.rs"]
mod execution_fixture;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/host-release/support/frontend.rs"]
mod frontend_fixture;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/host-release/staging.rs"]
mod staging_fixture;

#[cfg(verus_keep_ghost)]
#[path = "proof_support/host-release/support/release_tool.rs"]
mod tool_fixture;

verus! {
#[cfg(verus_keep_ghost)]
fn local_parent_failure_precedes_the_final_directory() {
    let result = policy::local::claim_output(
        Err::<(),u8>(9),
        || -> (s: Result<(), u8>) requires false { Ok(()) },
    );
    assert(result == Err::<(), u8>(9));
}

#[cfg(verus_keep_ghost)]
fn local_output_keeps_the_exclusive_directory_result<E>(leaf: Result<(), E>) {
    let result = policy::local::claim_output(
        Ok::<(),E>(()),
        || -> (s: Result<(), E>) ensures s == leaf { leaf },
    );
    assert(result == leaf);
}

#[cfg(verus_keep_ghost)]
fn only_git_regular_files_are_admitted(line: &[u8]) {
    let accepted = policy::committed::regular_blob_mode(line);
    let plain = [49u8, 48u8, 48u8, 54u8, 52u8, 52u8, 10u8];
    let executable = [49u8, 48u8, 48u8, 55u8, 53u8, 53u8, 10u8];
    assert(plain@ =~= seq![49u8, 48, 48, 54, 52, 52, 10]);
    assert(executable@ =~= seq![49u8, 48, 48, 55, 53, 53, 10]);
    assert(accepted == (line@ == plain@ || line@ == executable@));
}

#[cfg(verus_keep_ghost)]
fn archive_end_preserves_the_error_or_reports_the_member<T, E>(member: T, failure: E) {
    let complete = policy::transport::archive_finished::<T, E>(None);
    let extra = policy::transport::archive_finished::<T, E>(Some(Ok(member)));
    let broken = policy::transport::archive_finished::<T, E>(Some(Err(failure)));
    assert(complete == Result::<bool, E>::Ok(true));
    assert(extra == Result::<bool, E>::Ok(false));
    assert(broken == Result::<bool, E>::Err(failure));
}


#[cfg(verus_keep_ghost)]
fn bootstrap_replies_keep_original_errors_and_only_decode_complete_output() {
    use policy::bootstrap::{report_reply, ReportReply};
    let success = report_reply(None, 0, vec![91u8, 93u8], vec![10u8], true, true);
    assert(match success { ReportReply::Decode(bytes) => bytes@ == seq![91u8, 93u8], _ => false });
    let stdout_missing = report_reply(None, 0, vec![91u8, 93u8], vec![], false, true);
    assert(stdout_missing is Incomplete);
    let stderr_missing = report_reply(None, 0, vec![91u8, 93u8], vec![], true, false);
    assert(stderr_missing is Incomplete);
    let child_error = report_reply(None, 17, vec![], vec![9u8], false, false);
    assert(match child_error { ReportReply::Unsuccessful(n, bytes) => n == 17 && bytes@ == seq![9u8], _ => false });
    let observer_error = report_reply(Some(String::new()), 17, vec![], vec![], false, false);
    assert(match observer_error { ReportReply::ObservationFailure(msg) => msg@.len() == 0, _ => false });
}

#[cfg(verus_keep_ghost)]
fn stream_boundaries_select_the_captured_stream(limit: usize, out: &str, err: &str)
    requires out@ != err@,
{
    use policy::observed::stream_digests;
    expect_recipe(stream_digests(out, err, out, err));
    expect_recipe(!stream_digests(out, err, err, out));
    expect_recipe(!stream_digests(out, err, out, out));
    use policy::observed::{frame, Frame};
    let stdout = frame(79, limit, 0, limit, limit);
    let stderr = frame(69, limit, limit, 0, limit);
    let stdout_over = frame(79, 1, limit, 0, limit);
    let stderr_over = frame(69, 1, 0, limit, limit);
    let empty = frame(79, 0, limit, limit, limit);
    let terminal = frame(84, 2097152, limit, limit, limit);
    let too_large = frame(84, 2097153, 0, 0, limit);
    let unknown = frame(1, 0, 0, 0, limit);
    assert(stdout is Stdout && stderr is Stderr && empty is Stdout);
    assert(stdout_over is OutputLimit && stderr_over is OutputLimit);
    assert(terminal is Terminal && too_large is TerminalLimit && unknown is Unknown);
}

#[cfg(verus_keep_ghost)]
fn release_unit_keeps_real_exit_and_requires_complete_streams(signal: i32, code: i32)
    requires 1 <= signal <= 127, 0 <= code <= 255,
{
    use policy::process::{child_status, stream_failed};
    let stopped = child_status(Some(signal), code);
    let exited = child_status(None, code);
    assert(stopped == Some((-signal) as i32) && exited == Some(code));
    let no_signal = child_status(Some(0), 0);
    let large_signal = child_status(Some(128), 0);
    let no_exit = child_status(None, -1);
    let large_exit = child_status(None, 256);
    assert(no_signal.is_none() && large_signal.is_none() && no_exit.is_none() && large_exit.is_none());
    expect_recipe(stream_failed(false, 0));
    expect_recipe(stream_failed(true, 1));
    expect_recipe(!stream_failed(true, 0));
}



#[cfg(verus_keep_ghost)]
fn landing_requires_both_real_attempts_and_the_development_branch(run: u64, a: u64, upstream: u64, b: u64)
    requires run > 0, a > 0, upstream > 0, b > 0,
{
    use policy::template_run::landing_event;
    proof { reveal_strlit("dev"); reveal_strlit("main"); reveal_strlit("workflow_run");
        reveal_strlit("workflow_dispatch"); reveal_strlit("pull_request");
        assert("dev"@.len() != "main"@.len());
        assert("workflow_run"@[0] != "pull_request"@[0]);
        assert("workflow_dispatch"@.len() != "pull_request"@.len());
    }
    expect_recipe(landing_event("dev", "workflow_run", run, a, upstream, b));
    expect_recipe(landing_event("dev", "workflow_dispatch", run, a, upstream, b));
    expect_recipe(!landing_event("main", "workflow_run", run, a, upstream, b));
    expect_recipe(!landing_event("dev", "pull_request", run, a, upstream, b));
    expect_recipe(!landing_event("dev", "workflow_run", run, 0, upstream, b));
    expect_recipe(!landing_event("dev", "workflow_run", run, a, upstream, 0));
}

#[cfg(verus_keep_ghost)]
fn registration_keeps_only_the_applicable_stream(stdout: Vec<u8>, diagnostic: Vec<u8>, code: i32)
    requires code != 0, diagnostic.len() > 0,
{
    use policy::registrations::{Reply, registration_reply};
    let ok = registration_reply(0, stdout, diagnostic);
    assert(ok matches Reply::Decode(bytes) && bytes == stdout);
    let empty = registration_reply(code, Vec::new(), Vec::new());
    assert(empty matches Reply::Exit(status) && status == code);
}

#[cfg(verus_keep_ghost)]
fn registration_keeps_the_entire_diagnostic_through_utf8_decoding(stdout: Vec<u8>, stderr: Vec<u8>, code: i32)
    requires code != 0, stderr.len() > 0,
{
    let reply = policy::registrations::registration_reply(code, stdout, stderr);
    assert(reply matches policy::registrations::Reply::Diagnostic(bytes) && bytes == stderr);
}

#[cfg(verus_keep_ghost)]
fn diagnostic_removes_only_one_final_lf(bytes: &[u8])
{
    let n = policy::registrations::diagnostic_length(bytes);
    if bytes.len() > 0 && bytes[bytes.len() - 1] == 10 {
        assert(n + 1 == bytes.len());
        assert(bytes@.take(n as int) + seq![10u8] == bytes@);
    } else {
        assert(n == bytes.len());
    }
}


#[cfg(verus_keep_ghost)]
fn retained_identity_and_write_budget(
    before: policy::evidence::FileIdentity,
    after: policy::evidence::FileIdentity,
    bytes: &[u8],
    previous: u64,
    write_error: Option<u8>,
)
    requires previous as int + before.len as int <= u64::MAX,
{
    let mut used = previous;
    let total = previous + before.len;
    let result = policy::evidence::publish_observed(before, after, bytes, &mut used, total,
        |actual: &[u8]| -> (r: Result<(), u8>)
            requires before == after, actual@ == bytes@, actual.len() == before.len,
            ensures r == match write_error { None => Ok::<(),u8>(()), Some(error) => Err(error) },
        {
            match write_error { Some(error) => Err(error), None => Ok(()) }
        });
    let changed = before.dev != after.dev || before.ino != after.ino || before.len != after.len
        || before.mtime != after.mtime || before.mtime_nsec != after.mtime_nsec
        || bytes.len() as u64 != before.len;
    if changed {
        assert(result == Ok::<bool,u8>(false));
        assert(used == previous);
    } else if let Some(error) = write_error {
        assert(result == Err::<bool,u8>(error));
        assert(used == previous);
    } else {
        assert(result == Ok::<bool,u8>(true));
        assert(used as int == previous as int + bytes.len() as int);
    }
}

#[cfg(verus_keep_ghost)]
fn retained_bytes_and_directory_boundaries(size: u64) {
    use policy::evidence::{bytes_after_copy, group_status, nested_eligible};
    let exact = bytes_after_copy(size, 0, size);
    let full_empty = bytes_after_copy(size, size, 0);
    let full_one = bytes_after_copy(size, size, 1);
    assert(exact == Some(size) && full_empty == Some(size) && full_one.is_none());
    let regular = nested_eligible(true, true, true, true);
    let relative = nested_eligible(false, true, true, true);
    let file = nested_eligible(true, false, true, true);
    let child = nested_eligible(true, true, false, true);
    let other_name = nested_eligible(true, true, true, false);
    assert(regular && !relative && !file && !child && !other_name);
    let complete = group_status(true, true);
    let file_missing = group_status(true, false);
    let source_unavailable = group_status(false, true);
    assert(complete@ == "retained"@ && file_missing@ == "partial"@ && source_unavailable@ == "partial"@);
}

#[cfg(verus_keep_ghost)]
fn pushed_tags_separate_stable_main_from_dev_and_refuse_empty_suffixes() {
    use policy::workflow_identity::{tag_branch, pushed_dev, pushed_tag_output};
    let major=[118u8,48,49]; let minor=[50u8]; let patch=[51u8]; let suffix=[45u8,46,65];
    proof { assert(major@ =~= seq![118u8,48,49]); assert(minor@ =~= seq![50u8]); assert(patch@ =~= seq![51u8]); assert(suffix@ =~= seq![45u8,46,65]); }
    let parts: [&[u8];3]=[&major,&minor,&patch];
    let good=tag_branch(&parts,None); let preview=tag_branch(&parts,Some(&suffix));
    let bad=tag_branch(&parts,Some(&[]));
    let ancestor=pushed_dev(0); let other=pushed_dev(1); let failed=pushed_dev(128);
    assert(good is Main && preview is Dev && bad is Invalid && ancestor && !other && !failed);
    let stable = pushed_tag_output(&good, true);
    let prerelease = pushed_tag_output(&preview, true);
    let quiet_stable = pushed_tag_output(&good, false);
    let quiet_preview = pushed_tag_output(&preview, false);
    let invalid = pushed_tag_output(&bad, true);
    assert(stable@ == "false\n"@ && prerelease@ == "true\n"@);
    assert(quiet_stable@ == ""@ && quiet_preview@ == ""@ && invalid@ == ""@);
}


#[cfg(verus_keep_ghost)]
fn fixed_series_requires_preview_and_rolling_accepts_stable(core: &str, parts: &[&str], serial: &str)
    requires parts.len() == 3,
        forall|i: int| 0 <= i < parts.len() ==> delivery::decimal(parts@[i].spec_bytes(), true),
        delivery::decimal(serial.spec_bytes(), false),
{
    expect_recipe(delivery::consumer_series(false, core, parts, core, Some(serial)));
    expect_recipe(!delivery::consumer_series(false, core, parts, core, None));
    expect_recipe(delivery::consumer_series(true, core, parts, core, None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn stable_and_adopted_preview_use_the_actual_release_relations() {
    proof {
        reveal_strlit("ahead"); reveal_strlit("behind"); reveal_strlit("identical"); reveal_strlit("diverged");
        reveal_strlit("date"); reveal_strlit(""); reveal_strlit("other"); reveal_strlit("sha");
        assert("date"@.len() != ""@.len()); assert("other"@.len() != "sha"@.len());
        assert("ahead"@.len() != "behind"@.len()); assert("ahead"@.len() != "identical"@.len());
        assert("diverged"@ != "identical"@) by { assert("diverged"@[0] != "identical"@[0]); }
    }
    expect_recipe(delivery::includes_base(Some("ahead")));
    expect_recipe(!delivery::includes_base(Some("behind")));
    expect_recipe(delivery::covered_candidate(Some("behind")) == Some(true));
    expect_recipe(delivery::covered_candidate(Some("ahead")) == Some(false));
    expect_recipe(delivery::covered_candidate(Some("diverged")).is_none());
    expect_recipe(delivery::explicit_stable(Some("tag"),"tag",Some(false),Some(false),Some("date")) == Some(true));
    expect_recipe(delivery::explicit_stable(Some("tag"),"tag",Some(true),Some(false),None) == Some(false));
    expect_recipe(delivery::explicit_stable(Some("tag"),"tag",Some(false),Some(false),None).is_none());
    proof { reveal_with_fuel(delivery::serial_digits, 3); }
    expect_recipe(delivery::adopted_preview(Some("sha"),"sha","0","0",&[49],Some(1)));
    expect_recipe(!delivery::adopted_preview(Some("other"),"sha","0","0",&[49],Some(1)));
    expect_recipe(!delivery::adopted_preview(None,"sha","0","0",&[49],Some(2)));
    expect_recipe(!delivery::adopted_preview(None,"sha","0","0",&[48,49],Some(1)));
}

#[cfg(verus_keep_ghost)]
fn a_correct_upload_consumes_only_its_name<A: delivery::PendingAssets>(
    pending: &mut A, name: &str, digest: &str,
)
    requires
        old(pending).pins().contains_key(name@),
        old(pending).pins()[name@] == digest@,
        digest.spec_bytes().len() == 64,
        delivery::lower_hex(digest.spec_bytes()),
{
    let ghost original = pending.pins();
    let rows = [(Some(name), Some("uploaded"), Some(digest))];
    let result = delivery::consume_assets(pending, &rows);
    proof {
        reveal_with_fuel(delivery::remaining_after, 3);
        assert(result is Ok);
        assert(pending.pins() == original.remove(name@));
    }
}

#[cfg(verus_keep_ghost)]
fn a_duplicate_precedes_a_later_missing_name<A: delivery::PendingAssets>(
    pending: &mut A, name: &str, digest: &str,
)
    requires
        old(pending).pins().contains_key(name@),
        old(pending).pins()[name@] == digest@,
        digest.spec_bytes().len() == 64,
        delivery::lower_hex(digest.spec_bytes()),
{
    let row = (Some(name), Some("uploaded"), Some(digest));
    let rows = [row, row, (None, None, None)];
    let result = delivery::consume_assets(pending, &rows);
    proof {
        reveal_with_fuel(delivery::remaining_after, 4);
        assert(result matches Err((1, delivery::PublicationAssetError::UnexpectedName)));
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn publication_replay_requires_an_existing_ref_and_all_assets() {
    expect_recipe(delivery::publication_state(false, 0, None, None, "sha", 1) == 0);
    expect_recipe(delivery::publication_state(true, 1, Some(true), None, "sha", 1) == 1);
    expect_recipe(delivery::publication_state(true, 0, Some(true), Some("sha"), "sha", 1) == 1);
    expect_recipe(delivery::publication_state(true, 0, Some(true), None, "sha", 1) == 3);
    expect_recipe(delivery::publication_state(true, 1, Some(false), None, "sha", 0) == 2);
    expect_recipe(delivery::publication_state(true, 1, Some(false), None, "sha", 1) == 3);
    expect_recipe(delivery::publication_state(true, 0, Some(false), None, "sha", 0) == 3);
    expect_recipe(delivery::publication_state(false, 2, None, None, "sha", 0) == 3);
    expect_recipe(delivery::checksum_lines(&["one","two"], &["one","two"]));
    expect_recipe(!delivery::checksum_lines(&["one","two"], &["one"]));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn preview_prioritizes_publication_and_stable_prioritizes_semver() {
    use std::cmp::Ordering::*;
    use policy::version_order::install_precedence;
    expect_recipe(matches!(install_precedence(true, Greater, Less, Less), Greater));
    expect_recipe(matches!(install_precedence(true, Less, Greater, Greater), Less));
    expect_recipe(matches!(install_precedence(false, Greater, Less, Greater), Less));
    expect_recipe(matches!(install_precedence(false, Less, Greater, Less), Greater));
    expect_recipe(matches!(install_precedence(true, Equal, Less, Greater), Less));
    expect_recipe(matches!(install_precedence(false, Less, Equal, Greater), Less));
}

fn both_install_channels_break_joint_ties_by_original_tag(preview: bool, tag: std::cmp::Ordering) {
    use std::cmp::Ordering::*;
    let result = policy::version_order::install_precedence(preview, Equal, Equal, tag);
    assert(result == tag);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn pagination_continues_only_for_the_next_relation() {
    use policy::version_order::next_release_page;
    proof {
        reveal_strlit("next"); reveal_strlit("last"); reveal_strlit("previous");
        assert("last"@[0] != "next"@[0]);
        assert("previous"@.len() != "next"@.len());
    }
    expect_recipe(!next_release_page(&[]));
    expect_recipe(!next_release_page(&["previous", "last"]));
    expect_recipe(next_release_page(&["last", "next"]));
    expect_recipe(next_release_page(&["next", "previous"]));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn exact_install_selects_the_first_published_requested_tag() {
    use policy::version_order::exact_install;
    let rows = [
        (Some(true), Some("date"), Some("v1.0.0")),
        (None, Some("date"), Some("v1.0.0")),
        (Some(false), None, Some("v1.0.0")),
        (Some(false), Some(""), Some("v1.0.0")),
        (Some(false), Some("date"), Some("v1.0.0-alpha.1")),
        (Some(false), Some("date"), None),
        (Some(false), Some("date"), Some("v1.0.0")),
        (Some(false), Some("date"), Some("v1.0.0")),
    ];
    proof {
        reveal_strlit("date");
        reveal_strlit("");
        reveal_strlit("v1.0.0");
        reveal_strlit("v1.0.0-alpha.1");
        reveal_strlit("v1.0.1");
        assert("date"@.len() > 0);
        assert(""@.len() == 0);
        assert("v1.0.0"@.len() != "v1.0.0-alpha.1"@.len());
        assert("v1.0.0"@ != "v1.0.1"@) by { assert("v1.0.0"@[5] != "v1.0.1"@[5]); }
        assert("v1.0.0-alpha.1"@.len() != "v1.0.1"@.len());
        assert forall|i:int| 0 <= i < rows.len() implies
            (policy::version_order::public_exact(#[trigger] rows@[i], "v1.0.0") == (i == 6 || i == 7))
            && (policy::version_order::public_exact(rows@[i], "v1.0.0-alpha.1") == (i == 4))
            && !policy::version_order::public_exact(rows@[i], "v1.0.1") by {
            assert(i == 0 || i == 1 || i == 2 || i == 3 || i == 4 || i == 5 || i == 6 || i == 7);
            assert("v1.0.0"@[5] != "v1.0.1"@[5]);
        }
    }
    expect_recipe(exact_install(&rows, "v1.0.0") == Some(6));
    expect_recipe(exact_install(&rows, "v1.0.0-alpha.1") == Some(4));
    expect_recipe(exact_install(&rows, "v1.0.1").is_none());
    expect_recipe(exact_install(&[], "v1.0.0").is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn install_channels_match_the_public_release_and_version_together() {
    use policy::version_order::install_channel;
    expect_recipe(install_channel(false, false, false));
    expect_recipe(install_channel(true, true, true));
    expect_recipe(!install_channel(false, true, true));
    expect_recipe(!install_channel(true, false, true));
    expect_recipe(!install_channel(true, true, false));
    expect_recipe(!install_channel(false, false, true));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn consumer_pins_reject_bad_series_and_missing_downloads() {
    let empty: [&str; 0] = [];
    expect_recipe(!delivery::consumer_series(true, "", &empty, "", None));
    expect_recipe(!delivery::installed_download(None, "url", None));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn template_release_preserves_cardinality_and_the_consumer_pin() {
    let empty_expected: [delivery::ExpectedAsset; 0] = [];
    let empty_actual: [delivery::ObservedAsset; 0] = [];
    expect_recipe(delivery::release_assets(&empty_expected, &empty_actual));
    let expected = [("tool", None)];
    let missing = [(Some("tool"), Some("uploaded"), None)];
    expect_recipe(!delivery::release_assets(&expected, &empty_actual));
    expect_recipe(!delivery::release_assets(&empty_expected, &missing));
    expect_recipe(!delivery::release_assets(&expected, &missing));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn patch_successor_accepts_carry_and_numerical_order() {
    let nine = [57u8];
    let increased = policy::version_order::increment(&nine);
    expect_recipe(increased.len()==2 && increased[0]==49 && increased[1]==48);
    let ninety_nine = [57u8, 57u8];
    let hundred = policy::version_order::increment(&ninety_nine);
    expect_recipe(hundred.len()==3 && hundred[0]==49 && hundred[1]==48 && hundred[2]==48);
    let two = [50u8];
    let ten = [49u8, 48u8];
    let zero = [48u8];
    expect_recipe(policy::version_order::above([&two,&ten,&zero],[&two,&nine,&nine]));
    expect_recipe(!policy::version_order::above([&two,&nine,&nine],[&two,&ten,&zero]));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn select_the_last_exact_job_in_provider_order() {
    proof {
        reveal_strlit("Build"); reveal_strlit("Build again");
        assert("Build"@.len() != "Build again"@.len());
    }
    let rows = [Some("Build"), None, Some("Build again"), Some("Build"), None];
    expect_recipe(delivery::last_job_named(&rows, "Build") == Some(3));
    expect_recipe(delivery::last_job_named(&rows, "Build again") == Some(2));
    expect_recipe(delivery::last_job_named(&[], "Build").is_none());
    expect_recipe(delivery::last_job_named(&[None, Some("Build again")], "Build").is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_requires_a_named_successful_step_on_the_selected_job_and_commit() {
    proof { reveal_strlit("exercise"); reveal_strlit(""); assert("exercise"@.len() != 0); }
    expect_recipe(delivery::acceptance_declared(1, "exercise"));
    expect_recipe(!delivery::acceptance_declared(0, "exercise"));
    expect_recipe(!delivery::acceptance_declared(1, ""));
    let steps = [(Some("exercise"), Some("success"))];
    expect_recipe(delivery::acceptance_job(Some("job"), "job", Some("success"), Some("sha"), "sha", &steps, "exercise"));
    expect_recipe(!delivery::acceptance_job(Some("job"), "job", Some("success"), None, "sha", &steps, "exercise"));
    expect_recipe(!delivery::acceptance_job(Some("job"), "job", Some("success"), Some("sha"), "sha", &[], "exercise"));
}

#[cfg(verus_keep_ghost)]
fn every_run_identity_field_is_required(omitted: usize)
    requires omitted < 8,
{
    let mut fields = [Some("repo"), Some("repo"), Some("path"), Some("dev"),
                      Some("sha"), Some("push"), Some("completed"), Some("success")];
    fields[omitted] = None;
    assert(omitted == 0 || omitted == 1 || omitted == 2 || omitted == 3
        || omitted == 4 || omitted == 5 || omitted == 6 || omitted == 7);
    expect_recipe(!delivery::landed_push(&fields, "repo", "path", "dev", "sha"));
}

#[cfg(verus_keep_ghost)]
fn run_command_requires_one_complete_argument(command: String, other: String, first: String, second: String)
    requires command@ == "template-run-v1"@, other@ != "template-run-v1"@,
{
    expect_recipe(command::template_run_command(&[command]));
    expect_recipe(!command::template_run_command(&[]));
    expect_recipe(!command::template_run_command(&[other]));
    expect_recipe(!command::template_run_command(&[first, second]));
}

#[cfg(verus_keep_ghost)]
fn release_probe_rejects_wrong_exit_and_missing_text(stderr: &[u8])
{
    let marker = "usage: chrono-host-release ROOT ABSENT_OUTPUT";
    proof {
        reveal_strlit("usage: chrono-host-release ROOT ABSENT_OUTPUT");
        reveal_strlit("0"); reveal_strlit("2");
        assert("0"@[0] != "2"@[0]);
        assert(marker.spec_bytes().subrange(0, marker.spec_bytes().len() as int) =~= marker.spec_bytes());
        assert(policy::roadmap::marker_at(marker.spec_bytes(), marker.spec_bytes(), 0));
    }
    let valid = policy::release_usage_observed("2", marker.as_bytes());
    let success_exit = policy::release_usage_observed("0", stderr);
    let missing = policy::release_usage_observed("2", &[]);
    assert(valid && !success_exit && !missing);
}

#[cfg(verus_keep_ghost)]
fn installer_receives_the_complete_page_result(error: String) {
    let more = command::release_page_output(Ok(true));
    let last = command::release_page_output(Ok(false));
    let failed = command::release_page_output(Err(error));
    assert(more is Ok && more.unwrap()@ == "next"@);
    assert(last is Ok && last.unwrap()@ == "done"@);
    assert(failed is Err && failed.unwrap_err()@ == error@);
}

#[cfg(verus_keep_ghost)]
fn installer_forwards_only_its_page_headers(args: &[String])
{
    let actual = command::release_page_headers(args);
    proof {
        if args.len() != 3 {
            assert(actual.is_none());
        } else if args[0]@ == "resolve-install-v1"@ && args[1]@ == "--page-state"@ {
            assert(actual.is_some() && actual.unwrap()@ == args[2]@);
        } else {
            assert(actual.is_none());
        }
    }
}

#[cfg(verus_keep_ghost)]
fn decoder_only_relabels_the_reported_duplicate(key: &str, at: &str) {
    use policy::codec::duplicate_member;
    proof {
        reveal_strlit("duplicate entry with key "); reveal_strlit("ordinary error");
        assert("ordinary error"@.len() != "duplicate entry with key "@.len());
    }
    let record = duplicate_member(Some("duplicate entry with key "), Some(key), Some(at));
    assert(record == Some((key, at)));
    let unrelated = duplicate_member(Some("ordinary error"), Some(key), Some(at));
    let no_key = duplicate_member(Some("duplicate entry with key "), None, Some(at));
    let no_location = duplicate_member(Some("duplicate entry with key "), Some(key), None);
    assert(unrelated.is_none() && no_key.is_none() && no_location.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_requires_the_selected_successful_push() {
    proof {
        reveal_strlit("success"); reveal_strlit("failure"); reveal_strlit("pull_request"); reveal_strlit("push");
        assert("success"@.len() != "failure"@.len() || "success"@[0] != "failure"@[0]);
        assert("pull_request"@.len() != "push"@.len());
    }
    let mut fields = [Some("repo"), Some("repo"), Some("path"), Some("dev"), Some("sha"), Some("push"), Some("completed"), Some("success")];
    expect_recipe(delivery::landed_push(&fields, "repo", "path", "dev", "sha"));
    fields[7] = Some("failure");
    expect_recipe(!delivery::landed_push(&fields, "repo", "path", "dev", "sha"));
    fields[7] = Some("success");
    fields[5] = Some("pull_request");
    expect_recipe(!delivery::landed_push(&fields, "repo", "path", "dev", "sha"));
    fields[5] = Some("push");
    fields[1] = None;
    expect_recipe(!delivery::landed_push(&fields, "repo", "path", "dev", "sha"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn component_recovery_checks_the_entrypoint_paths_and_copied_bytes() {
    proof {
        reveal_strlit("--select-preview"); reveal_strlit("--plan-preview");
        reveal_strlit("bin"); reveal_strlit("other"); reveal_strlit("report"); reveal_strlit("old");
        reveal_strlit("executable.tar"); reveal_strlit("other.tar"); reveal_strlit("digest");
        assert("--plan-preview"@.len() != "--select-preview"@.len());
        assert("other"@.len() != "bin"@.len());
        assert("old"@.len() != "report"@.len());
        assert("other.tar"@.len() != "executable.tar"@.len());
        assert("other"@.len() != "digest"@.len());
    }
    expect_recipe(command::select_preview_command("--select-preview"));
    expect_recipe(!command::select_preview_command("--plan-preview"));
    expect_recipe(policy::frontend::artifact_paths("bin", "bin", "report", "report", "executable.tar"));
    expect_recipe(!policy::frontend::artifact_paths("other", "bin", "report", "report", "executable.tar"));
    expect_recipe(!policy::frontend::artifact_paths("bin", "bin", "old", "report", "executable.tar"));
    expect_recipe(!policy::frontend::artifact_paths("bin", "bin", "report", "report", "other.tar"));
    expect_recipe(policy::staging::copied_member("digest", "digest", 8, 8));
    expect_recipe(!policy::staging::copied_member("other", "digest", 8, 8));
    expect_recipe(!policy::staging::copied_member("digest", "digest", 7, 8));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn preview_reuse_binds_the_original_landing_base() {
    proof { reveal_strlit("base"); reveal_strlit("earlier"); assert("base"@.len() != "earlier"@.len()); }
    expect_recipe(policy::recipe::preview::matching_predecessor(true, "base", "base"));
    expect_recipe(!policy::recipe::preview::matching_predecessor(false, "base", "base"));
    expect_recipe(!policy::recipe::preview::matching_predecessor(true, "earlier", "base"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn milestone_release_selection_keeps_first_tie_and_last_final() {
    use policy::roadmap::{ReleaseFacts, Selection, select_release, event_or_newest};
    proof { reveal_strlit(""); reveal_strlit("event"); }
    let preview = ReleaseFacts { tag: &[118u8,49,45,112], date: &[50u8], draft: false, preview: true };
    let stable = ReleaseFacts { tag: &[118u8,49], date: &[49u8], draft: false, preview: false };
    let draft = ReleaseFacts { tag: &[118u8,49,45,100], date: &[50u8], draft: true, preview: true };
    let rows = [draft, stable, preview, preview, stable];
    proof {
        reveal_with_fuel(policy::roadmap::later, 3);
        assert(policy::roadmap::later(preview.date@,stable.date@,0));
        assert(!policy::roadmap::later(preview.date@,preview.date@,0));
        assert(!policy::roadmap::later(stable.date@,preview.date@,0));
        assert(policy::roadmap::marker_at(preview.tag@, stable.tag@, 0)) by {
            assert(preview.tag@.subrange(0,2) =~= stable.tag@);
        }
        reveal_with_fuel(policy::roadmap::choice, 7); reveal_with_fuel(policy::roadmap::later, 2); }
    expect_recipe(select_release(&rows, Selection::Newest) == Some(2));
    expect_recipe(select_release(&rows, Selection::Preview(&[118u8,49])) == Some(2));
    expect_recipe(select_release(&rows, Selection::Final(&[118u8,49])) == Some(4));
    expect_recipe(select_release(&rows, Selection::Preview(&[118u8,49,48])).is_none());
    expect_recipe(select_release(&[], Selection::Newest).is_none());
    expect_recipe(event_or_newest(Some("event"), None).is_some());
    expect_recipe(event_or_newest(Some(""), None).is_none());
}

// 同一预期在 Verus 中证明，在普通 Rust 中实际执行。
#[cfg_attr(not(verus_keep_ghost), test)]
fn roadmap_uses_only_public_releases_unassigned_open_issues_and_completed_milestones() {
    use policy::roadmap::{issue_destination,milestone_state,published,unassigned_issue};
    proof {
        reveal_strlit(""); reveal_strlit("date"); reveal_strlit("open"); reveal_strlit("closed");
        assert("closed"@.len() != "open"@.len());
    }
    expect_recipe(published(false,"date"));
    expect_recipe(!published(true,"date"));
    expect_recipe(!published(false,""));
    expect_recipe(unassigned_issue(false,"open",false));
    expect_recipe(!unassigned_issue(true,"open",false));
    expect_recipe(!unassigned_issue(false,"open",true));
    expect_recipe(!unassigned_issue(false,"closed",false));
    expect_recipe(policy::same_text(issue_destination(None,None),"Backlog"));
    expect_recipe(policy::same_text(issue_destination(Some("v2.0.0"),None),"v2.0.0"));
    expect_recipe(policy::same_text(issue_destination(Some("v2.0.0"),Some(0)),"Backlog"));
    expect_recipe(policy::same_text(milestone_state(true,0),"closed"));
    expect_recipe(policy::same_text(milestone_state(true,1),"open"));
    expect_recipe(policy::same_text(milestone_state(false,0),"open"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn first_release_block_keeps_surrounding_bytes_and_rejects_a_reversed_pair() {
    use policy::roadmap::release_block;
    #[cfg(verus_keep_ghost)]
    use policy::roadmap::{first_at, marker_at};
    broadcast use vstd::array::group_array_axioms;
    let begin = [1u8];
    let end = [2u8];
    let block = [7u8];
    let text = [9u8, 1, 2, 8, 1, 2];
    let reversed = [2u8, 1];
    proof {
        assert(text@.subrange(1, 2) =~= begin@);
        assert(text@.subrange(2, 3) =~= end@);
        assert forall|i:int| 0 <= i < 2 implies !marker_at(text@,end@,i) by {
            assert(text@.subrange(i,i+1)[0] == text@[i]);
        }
        assert(text@.subrange(0,1)[0] != begin@[0]);
        assert(first_at(text@,begin@,1) && first_at(text@,end@,2));
        assert(reversed@.subrange(0,1) =~= end@);
        assert(reversed@.subrange(1,2) =~= begin@);
        assert(reversed@.subrange(0,1)[0] != begin@[0]);
        assert(first_at(reversed@,begin@,1) && first_at(reversed@,end@,0));
    }
    let result = release_block(&text,&begin,&end,&block);
    expect_recipe(result.is_some());
    let bytes = result.unwrap();
    expect_recipe(bytes.len() == 5 && bytes[0] == 9 && bytes[1] == 7
        && bytes[2] == 8 && bytes[3] == 1 && bytes[4] == 2);
    expect_recipe(release_block(&reversed,&begin,&end,&block).is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn roadmap_updates_only_its_own_original_issue_across_release_attempts() {
    use policy::roadmap::{IssueAction,owns_failure,release_issue};
    broadcast use vstd::array::group_array_axioms;
    let prefix=[60u8,33]; let body=[60u8,33,32]; let other=[32u8,60,33];
    proof {
        assert(body@.take(2) =~= prefix@); assert(other@.take(2)[0] != prefix@[0]);
        reveal_strlit("bot"); reveal_strlit("github-actions[bot]");
        assert("bot"@.len() != "github-actions[bot]"@.len());
    }
    expect_recipe(owns_failure(false,"github-actions[bot]",&body,&prefix));
    expect_recipe(!owns_failure(true,"github-actions[bot]",&body,&prefix));
    expect_recipe(!owns_failure(false,"bot",&body,&prefix));
    expect_recipe(!owns_failure(false,"github-actions[bot]",&other,&prefix));
    let create=release_issue("completed","failure",None,"");
    let close=release_issue("completed","success",Some("open"),"");
    let reopen=release_issue("completed","timed_out",Some("closed"),"completed");
    let dismissed=release_issue("completed","failure",Some("closed"),"not_planned");
    let running=release_issue("in_progress","failure",None,"");
    let duplicate=release_issue("completed","failure",Some("open"),"");
    proof {
        reveal_strlit("completed"); reveal_strlit("in_progress"); reveal_strlit("not_planned");
        reveal_strlit("success"); reveal_strlit("failure"); reveal_strlit("timed_out");
        reveal_strlit("open"); reveal_strlit("closed");
        assert("failure"@[0] != "success"@[0]); assert("completed"@.len() != "in_progress"@.len());
        assert("completed"@.len() != "not_planned"@.len()); assert("open"@.len() != "closed"@.len());
    }
    expect_recipe(matches!(create,IssueAction::Create));
    expect_recipe(matches!(close,IssueAction::Close));
    expect_recipe(matches!(reopen,IssueAction::Reopen));
    expect_recipe(matches!(dismissed,IssueAction::None));
    expect_recipe(matches!(running,IssueAction::None));
    expect_recipe(matches!(duplicate,IssueAction::None));
}

fn expect_recipe(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_fixture_cleanup_preserves_the_original_failure() {
    expect_recipe(execution_fixture::failure_case::propagate_cleanup_error(false, true));
    expect_recipe(!execution_fixture::failure_case::propagate_cleanup_error(true, true));
    expect_recipe(!execution_fixture::failure_case::propagate_cleanup_error(false, false));
    expect_recipe(!execution_fixture::failure_case::propagate_cleanup_error(true, false));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn direct_compiler_binding_requires_the_path_and_both_identities() {
    let valid = policy::receipt::direct_compiler_binding(true, true, true);
    let wrong_path = policy::receipt::direct_compiler_binding(false, true, true);
    let wrong_cargo = policy::receipt::direct_compiler_binding(true, false, true);
    let wrong_rustc = policy::receipt::direct_compiler_binding(true, true, false);
    expect_recipe(valid && !wrong_path && !wrong_cargo && !wrong_rustc);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn promotion_requires_one_unexpired_original_with_the_selected_identity() {
    let one = delivery::single_original(Some(1), 1);
    let missing = delivery::single_original(Some(0), 0);
    let ambiguous = delivery::single_original(Some(2), 1);
    let valid = delivery::promotion_artifact(Some(30), Some(30), Some(false), Some(1), &[(Some(20), 20)], &[]);
    let wrong = delivery::promotion_artifact(Some(31), Some(30), Some(false), Some(1), &[], &[]);
    let expired = delivery::promotion_artifact(Some(30), None, Some(true), Some(1), &[], &[]);
    let unknown = delivery::promotion_artifact(None, None, Some(false), Some(1), &[], &[]);
    let empty = delivery::promotion_artifact(Some(30), None, Some(false), Some(0), &[], &[]);
    let wrong_run = delivery::promotion_artifact(Some(30), None, Some(false), Some(1), &[(Some(21),20)], &[]);
    let missing_expiry = delivery::promotion_artifact(Some(30), None, None, Some(1), &[], &[]);
    expect_recipe(one && !missing && !ambiguous && valid && !wrong && !expired && !unknown && !empty && !wrong_run && !missing_expiry);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn promotion_does_not_refresh_the_original_platform_time_window() {
    let valid = delivery::original_spans(2, 1000000001, Some(1), Some(1000000000), 1, 1);
    let late = delivery::original_spans(2, 1000000002, Some(1), Some(1000000001), 1, 1);
    let missing = delivery::original_spans(2, 3, None, Some(1), 1, 1);
    let rewritten = delivery::original_spans(2, 3, Some(1), Some(1), 1, 1);
    let reversed = delivery::original_spans(2, 1, Some(1), Some(0), 1, 1);
    let unit = delivery::original_spans(2, 1000000003, Some(1), Some(1000000002), 1, 2);
    expect_recipe(valid && !late && !missing && !rewritten && !reversed && !unit);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn promotion_requires_the_original_run_event_and_complete_workflow_head() {
    proof {
        reveal_strlit("workflow_run"); reveal_strlit("workflow_dispatch"); reveal_strlit("push");
        assert("push"@.len() != "workflow_run"@.len());
        assert("push"@.len() != "workflow_dispatch"@.len());
    }
    let head = [97u8; 40];
    let head_bytes: &[u8] = &head;
    proof {
        assert(head_bytes@ == head@);
        assert(head_bytes@.len() == 40);
        assert forall|i: int| 0 <= i < head_bytes@.len() implies head_bytes@[i] == 97 by {
            assert(head@[i] == 97);
        }
        assert(head_bytes@[0] == 97);
        assert(exists|i: int| 0 <= i < head_bytes@.len() && head_bytes@[i] != 48);
        assert(delivery::release_oid(head_bytes@));
        assert(delivery::text_pair((Some("workflow_run"), "workflow_run")));
        assert(delivery::text_pair((Some("workflow_dispatch"), "workflow_dispatch")));
    }
    let numbers = [(Some(2u64),2u64)];
    let no_numbers: &[(Option<u64>, u64)] = &[];
    let no_texts: &[(Option<&str>, &str)] = &[];
    let raw_head = Some(head_bytes);
    let valid = delivery::promotion_run(&numbers, no_texts, Some("workflow_run"), raw_head);
    let manual = delivery::promotion_run(no_numbers, no_texts, Some("workflow_dispatch"), raw_head);
    proof {
        // 显式实例化固定切片与 Option 输入；预期结果仍在下方独立断言。
        assert forall|i: int| 0 <= i < numbers.len() implies numbers@[i].0 == Some(numbers@[i].1) by {}
        assert forall|i: int| 0 <= i < no_numbers.len() implies no_numbers@[i].0 == Some(no_numbers@[i].1) by {}
        assert forall|i: int| 0 <= i < no_texts.len() implies delivery::text_pair(no_texts@[i]) by {}
        assert(match raw_head { Some(value) => delivery::release_oid(value@), None => false });
    }
    let wrong = delivery::promotion_run(&[], &[], Some("push"), Some(head_bytes));
    let missing = delivery::promotion_run(&[], &[], None, Some(head_bytes));
    let short = delivery::promotion_run(&[], &[], Some("workflow_run"), Some(&[97u8]));
    let attempt = delivery::promotion_run(&[(Some(3),2)], &[], Some("workflow_run"), Some(head_bytes));
    expect_recipe(valid);
    expect_recipe(manual);
    expect_recipe(!wrong);
    expect_recipe(!missing);
    expect_recipe(!short);
    expect_recipe(!attempt);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_missing_fields_are_not_empty_or_zero_values() {
    proof { reveal_strlit(""); }
    let text = delivery::text_fields(&[(None, "")]);
    let number = delivery::number_fields(&[(None, 0)]);
    let boolean = delivery::boolean_fields(&[(None, false)]);
    let complete = delivery::number_fields(&[(Some(0), 0), (Some(u64::MAX), u64::MAX)]);
    let changed = delivery::number_fields(&[(Some(0), 0), (Some(1), u64::MAX)]);
    expect_recipe(!text && !number && !boolean && complete && !changed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_requires_all_full_text_fields() {
    proof {
        reveal_strlit("source"); reveal_strlit("source-extra");
        assert("source"@.len() != "source-extra"@.len());
    }
    let exact = delivery::text_fields(&[(Some("source"), "source")]);
    let prefix = delivery::text_fields(&[(Some("source-extra"), "source")]);
    let missing = delivery::text_fields(&[(Some("source"), "source"), (None, "source")]);
    expect_recipe(exact && !prefix && !missing);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_decimal_ids_reject_zero_leading_zero_and_non_digits() {
    let positive = delivery::decimal_text(&[49u8,50], false);
    let component = delivery::decimal_text(&[48u8], true);
    let zero_id = delivery::decimal_text(&[48u8], false);
    let leading = delivery::decimal_text(&[48u8,49], true);
    let sign = delivery::decimal_text(&[45u8,49], false);
    let suffix = delivery::decimal_text(&[49u8,120], false);
    expect_recipe(positive && component && !zero_id && !leading && !sign && !suffix);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_public_versions_require_a_complete_draft_or_exact_existing_assets() {
    let draft = delivery::draft_state(Some(true), 99);
    let complete = delivery::draft_state(Some(false), 0);
    let incomplete = delivery::draft_state(Some(false), 1);
    let unknown = delivery::draft_state(None, 0);
    let zero = delivery::positive(Some(0));
    let maximum = delivery::positive(Some(u64::MAX));
    expect_recipe(draft == 1 && complete == 0 && incomplete == 2 && unknown == 2 && !zero && maximum);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_only_treats_the_original_explicit_404_as_absent() {
    proof {
        reveal_strlit("404"); reveal_strlit("Not Found"); reveal_strlit("403");
        assert("403"@[2] != "404"@[2]);
    }
    let success = delivery::api_outcome(0, None, None, false);
    let absent = delivery::api_outcome(1, Some("404"), Some("Not Found"), true);
    let denied = delivery::api_outcome(1, Some("403"), Some("Not Found"), true);
    let forbidden_absence = delivery::api_outcome(1, Some("404"), Some("Not Found"), false);
    let failed = delivery::api_outcome(2, Some("404"), Some("Not Found"), true);
    expect_recipe(success == 0 && absent == 1 && denied == 2 && forbidden_absence == 2 && failed == 2);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_asset_names_do_not_allow_paths_or_parent_components() {
    let name = [97u8,45,49,46,95];
    let dots = [46u8,46];
    let dot = [46u8];
    let slashed = [97u8,47,98];
    let line = [97u8,10];
    proof {
        assert(name@ =~= seq![97u8,45,49,46,95]);
        assert(dots@ =~= seq![46u8,46]);
        assert(dot@ =~= seq![46u8]);
        assert(slashed@ =~= seq![97u8,47,98]);
        assert(line@ =~= seq![97u8,10]);
    }
    let valid = delivery::name(&name);
    let parent = delivery::name(&dots);
    let current = delivery::name(&dot);
    let path = delivery::name(&slashed);
    let newline = delivery::name(&line);
    expect_recipe(valid && !parent && !current && !path && !newline);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn delivery_channel_and_requested_version_are_exact() {
    proof {
        reveal_strlit(""); reveal_strlit("build"); reveal_strlit("preview"); reveal_strlit("stable"); reveal_strlit("Preview"); reveal_strlit("v1.2.3"); reveal_strlit("v1.2.3-beta.1");
        assert("preview"@.len() != "build"@.len());
        assert("stable"@.len() != "build"@.len());
        assert("stable"@.len() != "preview"@.len());
        assert("Preview"@.len() != "build"@.len());
        assert("Preview"@.len() != "stable"@.len());
        assert("Preview"@[0] != "preview"@[0]);
        assert("v1.2.3"@.len() != "v1.2.3-beta.1"@.len());
    }
    let build = delivery::channel("");
    let preview = delivery::channel("preview");
    let stable = delivery::channel("stable");
    let unknown = delivery::channel("Preview");
    let derived = delivery::requested_version(1, "", "v1.2.3", "v1.2.3-beta.1");
    let override_version = delivery::requested_version(1, "v1.2.3", "v1.2.3", "v1.2.3");
    let final_version = delivery::requested_version(2, "v1.2.3", "v1.2.3", "v1.2.3");
    let prerelease = delivery::requested_version(2, "v1.2.3-beta.1", "v1.2.3", "v1.2.3-beta.1");
    expect_recipe(build == 0 && preview == 1 && stable == 2 && unknown == 3
        && derived && !override_version && final_version && !prerelease);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn frontend_header_requires_the_original_source_platform_and_github_role() {
    proof {
        reveal_strlit("chrono-release-frontend/v1"); reveal_strlit("other");
        reveal_strlit("commit"); reveal_strlit("other-commit");
        reveal_strlit("tree"); reveal_strlit("other-tree");
        reveal_strlit("system"); reveal_strlit("other-system");
        reveal_strlit("machine"); reveal_strlit("other-machine");
        assert("chrono-release-frontend/v1"@.len() != "other"@.len());
        assert("commit"@.len() != "other-commit"@.len());
        assert("tree"@.len() != "other-tree"@.len());
        assert("system"@.len() != "other-system"@.len());
        assert("machine"@.len() != "other-machine"@.len());
    }
    let same=policy::frontend_header("chrono-release-frontend/v1",true,"commit","tree","system","machine","commit","tree","system","machine");
    let schema=policy::frontend_header("other",true,"commit","tree","system","machine","commit","tree","system","machine");
    let role=policy::frontend_header("chrono-release-frontend/v1",false,"commit","tree","system","machine","commit","tree","system","machine");
    let commit=policy::frontend_header("chrono-release-frontend/v1",true,"other-commit","tree","system","machine","commit","tree","system","machine");
    let tree=policy::frontend_header("chrono-release-frontend/v1",true,"commit","other-tree","system","machine","commit","tree","system","machine");
    let system=policy::frontend_header("chrono-release-frontend/v1",true,"commit","tree","other-system","machine","commit","tree","system","machine");
    let machine=policy::frontend_header("chrono-release-frontend/v1",true,"commit","tree","system","other-machine","commit","tree","system","machine");
    expect_recipe(same && !schema && !role && !commit && !tree && !system && !machine);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn frontend_installation_requires_exactly_the_declared_tool_and_digest() {
    proof {
        reveal_strlit(".chrono-harness/bin/chrono-host-release"); reveal_strlit("other");
        reveal_strlit("hash"); reveal_strlit("other-hash");
        assert(".chrono-harness/bin/chrono-host-release"@.len() != "other"@.len());
        assert("hash"@.len() != "other-hash"@.len());
    }
    let same=policy::frontend_installation(1,1,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","hash","hash");
    let missing=policy::frontend_installation(1,0,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","hash","hash");
    let extra=policy::frontend_installation(2,1,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","hash","hash");
    let tool=policy::frontend_installation(1,1,"other","other","hash","hash");
    let path=policy::frontend_installation(1,1,".chrono-harness/bin/chrono-host-release","other","hash","hash");
    let hash=policy::frontend_installation(1,1,".chrono-harness/bin/chrono-host-release",".chrono-harness/bin/chrono-host-release","other-hash","hash");
    expect_recipe(same && !missing && !extra && !tool && !path && !hash);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn non_rust_verification_and_collection_do_not_require_rust() {
    let verification = policy::requires_rust(&Invocation::Verify { rust_toolchain: false });
    let collection = policy::requires_rust(&Invocation::Collect);
    expect_recipe(!verification && !collection);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn build_and_explicit_verification_require_rust() {
    let build = policy::requires_rust(&Invocation::Build);
    let verification = policy::requires_rust(&Invocation::Verify { rust_toolchain: true });
    expect_recipe(build && verification);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn executable_modes_require_a_regular_file_and_an_execute_bit() {
    proof {
        assert(493u32 & 73u32 == 73u32) by (bit_vector);
        assert(420u32 & 73u32 == 0u32) by (bit_vector);
        assert(1u32 & 73u32 == 1u32) by (bit_vector);
        assert(4095u32 & 73u32 == 73u32) by (bit_vector);
    }
    let normal = policy::executable_file(true, 493);
    let data = policy::executable_file(true, 420);
    let directory = policy::executable_file(false, 493);
    let lowest = policy::executable_file(true, 1);
    let special = policy::executable_file(true, 4095);
    let unknown = policy::executable_file(true, 4096);
    expect_recipe(normal && !data && !directory && lowest && special && !unknown);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn executable_identity_requires_the_full_digest_size_and_mode() {
    proof {
        reveal_strlit("hash"); reveal_strlit("hashx"); reveal_strlit("Hash");
        assert("hash"@.len() != "hashx"@.len());
        assert("hash"@[0] != "Hash"@[0]);
    }
    let same = policy::same_executable("hash", 7, 489, "hash", 7, 489);
    let digest = policy::same_executable("Hash", 7, 489, "hash", 7, 489);
    let prefix = policy::same_executable("hashx", 7, 489, "hash", 7, 489);
    let size = policy::same_executable("hash", 8, 489, "hash", 7, 489);
    let mode = policy::same_executable("hash", 7, 493, "hash", 7, 489);
    expect_recipe(same && !digest && !prefix && !size && !mode);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn tar_entry_requires_the_exact_regular_name_size_and_mode() {
    let name = [101u8,120,101,99,117,116,97,98,108,101];
    proof { assert(name@ =~= seq![101u8,120,101,99,117,116,97,98,108,101]); }
    let same = policy::executable_entry(&name, true, 7, 489, 7, 489);
    let link = policy::executable_entry(&name, false, 7, 489, 7, 489);
    let size = policy::executable_entry(&name, true, 8, 489, 7, 489);
    let mode = policy::executable_entry(&name, true, 7, 493, 7, 489);
    let traversal = [46u8,46,47,101,120,101,99,117,116,97,98,108,101];
    let path = policy::executable_entry(&traversal, true, 7, 489, 7, 489);
    let empty: [u8; 0] = [];
    proof { vstd::array::array_len_matches_n(&empty); }
    let absent = policy::executable_entry(&empty, true, 7, 489, 7, 489);
    expect_recipe(same);
    expect_recipe(!link);
    expect_recipe(!size);
    expect_recipe(!mode);
    expect_recipe(!path);
    expect_recipe(!absent);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn child_failure_wins_over_forwarding_retention_and_receipt_failures() {
    let ordinary = policy::result_exit(Some(41), false);
    let auxiliary = policy::result_exit(Some(41), true);
    let lowest = policy::result_exit(Some(1), true);
    let highest = policy::result_exit(Some(255), true);
    expect_recipe(ordinary == 41 && auxiliary == 41 && lowest == 1 && highest == 255);
    let success = policy::passed(Some(41), true);
    expect_recipe(!success);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_child_fails_when_evidence_cannot_be_published() {
    let success = policy::result_exit(Some(0), false);
    let failure = policy::result_exit(Some(0), true);
    let passed = policy::passed(Some(0), false);
    let rejected = policy::passed(Some(0), true);
    expect_recipe(success == 0 && failure == 2 && passed && !rejected);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_child_status_is_never_success() {
    let no_error = policy::result_exit(None, false);
    let with_error = policy::result_exit(None, true);
    let passed = policy::passed(None, false);
    expect_recipe(no_error == 2 && with_error == 2 && !passed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn child_signal_survives_auxiliary_io_failures() {
    let killed = policy::result_exit(Some(-9), false);
    let killed_with_io_failure = policy::result_exit(Some(-9), true);
    let first = policy::result_exit(Some(-1), true);
    let last = policy::result_exit(Some(-127), true);
    expect_recipe(killed == 137 && killed_with_io_failure == 137 && first == 129 && last == 255);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn old_success_cannot_hide_current_failed_cancelled_or_skipped_needs() {
    let failed = policy::selected_producer(&DependencyResult::Failure, "run", "run", 1, 2, "artifact");
    let cancelled = policy::selected_producer(&DependencyResult::Cancelled, "run", "run", 1, 2, "artifact");
    let skipped = policy::selected_producer(&DependencyResult::Skipped, "run", "run", 1, 2, "artifact");
    expect_recipe(!failed && !cancelled && !skipped);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn successful_current_needs_can_select_the_original_earlier_attempt() {
    proof { reveal_strlit("run"); reveal_strlit("artifact"); }
    let carried = policy::selected_producer(&DependencyResult::Success, "run", "run", 1, 2, "artifact");
    let same = policy::selected_producer(&DependencyResult::Success, "run", "run", 2, 2, "artifact");
    let large = policy::selected_producer(&DependencyResult::Success, "run", "run", u64::MAX, u64::MAX, "artifact");
    expect_recipe(carried && same && large);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn missing_future_or_foreign_producers_are_rejected() {
    proof {
        reveal_strlit("run"); reveal_strlit("foreign"); reveal_strlit("");
        assert("run"@.len() != "foreign"@.len());
    }
    let zero = policy::selected_producer(&DependencyResult::Success, "run", "run", 0, 2, "artifact");
    let future = policy::selected_producer(&DependencyResult::Success, "run", "run", 3, 2, "artifact");
    let foreign = policy::selected_producer(&DependencyResult::Success, "foreign", "run", 1, 2, "artifact");
    let absent_run = policy::selected_producer(&DependencyResult::Success, "", "", 1, 2, "artifact");
    let absent_artifact = policy::selected_producer(&DependencyResult::Success, "run", "run", 1, 2, "");
    expect_recipe(!zero && !future && !foreign && !absent_run && !absent_artifact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn installation_asset_identity_and_uploaded_state() {
    use delivery::{installation_asset, uploaded_download, NamedAsset};
    proof {
        reveal_strlit("linux"); reveal_strlit("mac"); reveal_strlit("uploaded"); reveal_strlit("new");
        assert("mac"@.len() != "linux"@.len()); assert("new"@.len() != "uploaded"@.len());
    }
    let names = [None, Some("mac"), Some("linux"), Some("mac")];
    expect_recipe(matches!(installation_asset(&names, "linux"), NamedAsset::Found(2)));
    expect_recipe(matches!(installation_asset(&names, "mac"), NamedAsset::Duplicate));
    expect_recipe(matches!(installation_asset(&[], "mac"), NamedAsset::Missing));
    expect_recipe(!uploaded_download(Some("new"), Some("URL"), "URL", Some("hash")));
    expect_recipe(!uploaded_download(None, Some("URL"), "URL", Some("hash")));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_a_copier_product_can_request_the_next_patch() {
    use delivery::{version_policy, VersionPolicy};
    proof {
        reveal_strlit("explicit"); reveal_strlit("next_patch"); reveal_strlit("copier"); reveal_strlit("binary");
        assert("explicit"@.len() != "next_patch"@.len());
        assert("copier"@[0] != "binary"@[0]);
    }
    expect_recipe(matches!(version_policy(Some("binary"), false, None), VersionPolicy::Explicit));
    expect_recipe(matches!(version_policy(Some("binary"), true, Some("explicit")), VersionPolicy::Explicit));
    expect_recipe(matches!(version_policy(Some("copier"), true, Some("next_patch")), VersionPolicy::NextPatch));
    expect_recipe(matches!(version_policy(Some("binary"), true, Some("next_patch")), VersionPolicy::WrongProduct));
    expect_recipe(matches!(version_policy(None, true, Some("next_patch")), VersionPolicy::WrongProduct));
    expect_recipe(matches!(version_policy(Some("copier"), true, None), VersionPolicy::Unknown));
}

fn unknown_version_policies_stop_before_release_lookup(name: &str, product: &str)
    requires name@ != "explicit"@, name@ != "next_patch"@,
{
    let result = delivery::version_policy(Some(product), true, Some(name));
    expect_recipe(matches!(result, delivery::VersionPolicy::Unknown));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_events_stop_before_unknown_sources_and_writes() {
    use delivery::{release_event, ReleaseEvent, draft_notes, DraftNotes};
    proof {
        reveal_strlit("r"); reveal_strlit("s"); reveal_strlit("main"); reveal_strlit("dev");
        reveal_strlit("topic"); reveal_strlit("workflow_run"); reveal_strlit("workflow_dispatch");
        reveal_strlit("refs/heads/main"); reveal_strlit("refs/heads/dev"); reveal_strlit("commit");
        reveal_strlit("refs/heads/topic"); reveal_strlit("notes"); reveal_strlit("other");
        assert("workflow_run"@.len() != "workflow_dispatch"@.len());
        assert("refs/heads/main"@.len() != "refs/heads/dev"@.len());
        assert("topic"@.len() != "dev"@.len() && "topic"@.len() != "main"@.len());
        assert("r"@[0] != "s"@[0] && "notes"@[0] != "other"@[0]);
    }
    expect_recipe(matches!(release_event("r",Some("r"),"workflow_run",Some("dev"),None), ReleaseEvent::Dev));
    expect_recipe(matches!(release_event("r",Some("r"),"workflow_run",Some("main"),None), ReleaseEvent::Main));
    expect_recipe(matches!(release_event("r",Some("r"),"workflow_dispatch",None,Some("refs/heads/dev")), ReleaseEvent::Dispatch));
    expect_recipe(matches!(release_event("r",Some("r"),"workflow_dispatch",None,Some("refs/heads/main")), ReleaseEvent::WrongEvent));
    expect_recipe(matches!(release_event("r",Some("r"),"workflow_run",Some("topic"),None), ReleaseEvent::WrongBranch));
    expect_recipe(matches!(release_event("r",Some("s"),"workflow_run",Some("dev"),None), ReleaseEvent::WrongRepository));
    expect_recipe(matches!(draft_notes(false,true,Some("other"),"notes"),DraftNotes::Replace));
    expect_recipe(matches!(draft_notes(true,true,Some("other"),"notes"),DraftNotes::Refuse));
    expect_recipe(matches!(draft_notes(true,true,Some("notes"),"notes"),DraftNotes::Keep));
    expect_recipe(matches!(draft_notes(true,false,None,"notes"),DraftNotes::Keep));
}

fn direct_main_precondition(head: &str, older: &str)
    requires delivery::release_oid(head.spec_bytes()), delivery::release_oid(older.spec_bytes()), head@ != older@,
{
    use delivery::{main_ref,MainRef};
    proof { reveal_strlit("refs/heads/main"); reveal_strlit("refs/heads/topic");
        assert("refs/heads/main"@.len() != "refs/heads/topic"@.len()); }
    expect_recipe(matches!(main_ref(Some("refs/heads/main"),Some("commit"),Some(head),head),MainRef::Current));
    expect_recipe(matches!(main_ref(Some("refs/heads/main"),Some("commit"),Some(older),head),MainRef::FastForward));
    expect_recipe(matches!(main_ref(Some("refs/heads/topic"),Some("commit"),Some(older),head),MainRef::Invalid));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn receipt_job_run_and_attempt_must_match_the_selected_producer() {
    proof {
        reveal_strlit("job"); reveal_strlit("other"); reveal_strlit("run"); reveal_strlit("changed");
        reveal_strlit("");
        assert("job"@.len() != "other"@.len());
        assert("run"@.len() != "changed"@.len());
    }
    let valid = policy::producer_lineage("job", "job", "run", "run", 1, 1);
    let job = policy::producer_lineage("other", "job", "run", "run", 1, 1);
    let run = policy::producer_lineage("job", "job", "changed", "run", 1, 1);
    let attempt = policy::producer_lineage("job", "job", "run", "run", 1, 2);
    let zero = policy::producer_lineage("job", "job", "run", "run", 0, 0);
    let empty_job = policy::producer_lineage("", "", "run", "run", 1, 1);
    let empty_run = policy::producer_lineage("job", "job", "", "", 1, 1);
    expect_recipe(valid && !job && !run && !attempt && !zero && !empty_job && !empty_run);
}

}

#[cfg(verus_keep_ghost)]
verus! {
fn workflow_reference_boundaries(repo: &[u8], path: &[u8], reference: &[u8], commit: &[u8])
    requires repo.len() > 0, policy::delivery::release_oid(commit@),
        policy::workflow_identity::segment(reference@, 0, repo@),
        reference.len() > repo.len(), reference@[repo.len() as int] == 47,
        policy::workflow_identity::segment(reference@, repo.len()+1, path@),
        reference.len() >= repo.len()+path.len()+2, reference@[repo.len()+path.len()+1] == 64,
{
    let accepted = policy::workflow_identity::workflow_environment(repo, path, reference, commit, commit);
    assert(accepted == (reference.len() > repo.len()+path.len()+2));
    let empty: [u8; 0] = [];
    let wrong_repository = policy::workflow_identity::workflow_environment(&empty, path, reference, commit, commit);
    assert(!wrong_repository);
    let bad_commit = policy::workflow_identity::workflow_environment(repo, path, reference, &empty, commit);
    assert(!bad_commit);
}
}
