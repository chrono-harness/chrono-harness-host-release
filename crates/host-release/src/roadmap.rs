//! Pure decisions consumed by the generated GitHub issue-roadmap workflow.
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub fn published(draft: bool, date: &str) -> (keep: bool)
    ensures keep == (!draft && date@.len() > 0),
{ !draft && !date.is_empty() }

pub fn unassigned_issue(pull: bool, state: &str, has_milestone: bool) -> (assign: bool)
    ensures assign == (!pull && state@ == "open"@ && !has_milestone),
{ !pull && <&str>::exec_eq(state, "open") && !has_milestone }

pub fn milestone_state(final_release: bool, open_issues: u64) -> (state: &'static str)
    ensures state@ == (if final_release && open_issues == 0 { "closed"@ } else { "open"@ }),
{ if final_release && open_issues == 0 { "closed" } else { "open" } }

pub fn owns_failure(pull: bool, login: &str, body: &[u8], marker: &[u8]) -> (owned: bool)
    ensures owned == (!pull && login@ == "github-actions[bot]"@ &&
        marker.len() > 0 && marker.len() <= body.len() && body@.take(marker.len() as int) == marker@),
{
    if pull || !<&str>::exec_eq(login,"github-actions[bot]") || marker.len() == 0 || body.len() < marker.len() {
        return false;
    }
    let mut i = 0;
    while i < marker.len()
        invariant i <= marker.len() <= body.len(),
            forall|j:int| 0 <= j < i ==> body@[j] == marker@[j],
        decreases marker.len() - i,
    {
        if body[i] != marker[i] {
            assert(body@.take(marker.len() as int)[i as int] != marker@[i as int]);
            return false;
        }
        i += 1;
    }
    assert(body@.take(marker.len() as int) =~= marker@);
    true
}

#[cfg_attr(not(verus_keep_ghost), derive(Serialize))]
#[cfg_attr(not(verus_keep_ghost), serde(rename_all = "kebab-case"))]
pub enum IssueAction { None, Close, Reopen, Create }

pub enum ReleaseResult { Success, Failure, Unrelated }

pub fn release_result(status: &str, conclusion: &str) -> (result: ReleaseResult)
    ensures
        (result is Success) == (status@ == "completed"@ && conclusion@ == "success"@),
        (result is Failure) == (status@ == "completed"@ && (conclusion@ == "failure"@ || conclusion@ == "timed_out"@)),
{
    proof {
        reveal_strlit("success"); reveal_strlit("failure"); reveal_strlit("timed_out");
        assert("success"@[0] != "failure"@[0]);
        assert("success"@.len() != "timed_out"@.len());
    }
    if !<&str>::exec_eq(status, "completed") { return ReleaseResult::Unrelated; }
    if <&str>::exec_eq(conclusion, "success") { return ReleaseResult::Success; }
    if <&str>::exec_eq(conclusion, "failure") || <&str>::exec_eq(conclusion, "timed_out") {
        return ReleaseResult::Failure;
    }
    ReleaseResult::Unrelated
}

/// A failed or timed-out release owns one issue across all attempts of the same run.
pub fn release_issue(
    status: &str, conclusion: &str, issue: Option<&str>, reason: &str,
) -> (action: IssueAction)
    ensures
        (action is Close) == (status@ == "completed"@ && conclusion@ == "success"@
            && issue is Some && issue.unwrap()@ == "open"@),
        (action is Create) == (status@ == "completed"@ && (conclusion@ == "failure"@ || conclusion@ == "timed_out"@)
            && issue is None),
        (action is Reopen) == (status@ == "completed"@ && (conclusion@ == "failure"@ || conclusion@ == "timed_out"@)
            && issue is Some && issue.unwrap()@ == "closed"@ && reason@ != "not_planned"@),
{
    match release_result(status, conclusion) {
        ReleaseResult::Success => {
            if let Some(state) = issue {
                if <&str>::exec_eq(state, "open") { return IssueAction::Close; }
            }
        }
        ReleaseResult::Failure => match issue {
            None => return IssueAction::Create,
            Some(state) if <&str>::exec_eq(state, "closed") && !<&str>::exec_eq(reason, "not_planned") => return IssueAction::Reopen,
            _ => (),
        },
        ReleaseResult::Unrelated => (),
    }
    IssueAction::None
}
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Query {
    Published {
        releases: Vec<Release>,
    },
    Unassigned {
        issues: Vec<Issue>,
    },
    Milestone {
        final_release: bool,
        open_issues: u64,
    },
    ReleaseRun {
        status: String,
        conclusion: Option<String>,
    },
    ReleaseIssue {
        status: String,
        conclusion: String,
        run_id: u64,
        issues: Vec<Issue>,
    },
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Release {
    draft: bool,
    published_at: Option<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct User {
    login: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Issue {
    number: u64,
    state: String,
    state_reason: Option<String>,
    pull_request: Option<serde_json::Value>,
    milestone: Option<serde_json::Value>,
    user: Option<User>,
    body: Option<String>,
}

#[cfg(not(verus_keep_ghost))]
pub fn run(input: impl std::io::Read, output: impl std::io::Write) -> Result<i32, String> {
    use serde_json::{json, to_writer};
    let query: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    let result = match query {
        Query::Published { releases } => json!(
            releases
                .iter()
                .enumerate()
                .filter_map(|(index, r)| {
                    published(r.draft, r.published_at.as_deref().unwrap_or("")).then_some(index)
                })
                .collect::<Vec<_>>()
        ),
        Query::Unassigned { issues } => json!(
            issues
                .iter()
                .filter(|i| unassigned_issue(
                    i.pull_request.is_some(),
                    &i.state,
                    i.milestone.is_some()
                ))
                .map(|i| i.number)
                .collect::<Vec<_>>()
        ),
        Query::Milestone {
            final_release,
            open_issues,
        } => json!(milestone_state(final_release, open_issues)),
        Query::ReleaseRun { status, conclusion } => json!(!matches!(
            release_result(&status, conclusion.as_deref().unwrap_or("")),
            ReleaseResult::Unrelated
        )),
        Query::ReleaseIssue {
            status,
            conclusion,
            run_id,
            issues,
        } => {
            let marker = format!("<!-- chrono-harness release-run {run_id} -->");
            let issue = issues.iter().find(|i| {
                owns_failure(
                    i.pull_request.is_some(),
                    i.user.as_ref().map_or("", |u| u.login.as_str()),
                    i.body.as_deref().unwrap_or("").as_bytes(),
                    marker.as_bytes(),
                )
            });
            let action = release_issue(
                &status,
                &conclusion,
                issue.map(|i| i.state.as_str()),
                issue.and_then(|i| i.state_reason.as_deref()).unwrap_or(""),
            );
            json!({"action":action,"issue":issue.map(|i| i.number)})
        }
    };
    to_writer(output, &result).map_err(|e| e.to_string())?;
    Ok(0)
}
