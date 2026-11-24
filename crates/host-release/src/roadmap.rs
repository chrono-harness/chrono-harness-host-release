//! Pure decisions consumed by the generated GitHub issue-roadmap workflow.
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use vstd::slice::slice_subrange;

verus! {

fn has_publication(draft: bool, has_date: bool) -> (keep: bool)
    ensures keep == (!draft && has_date),
{ !draft && has_date }

pub fn published(draft: bool, date: &str) -> (keep: bool)
    ensures keep == (!draft && date@.len() > 0),
{ has_publication(draft, !date.is_empty()) }

/// GitHub supplies UTC publication timestamps in a fixed ISO 8601 representation.
pub open spec fn later(a: Seq<u8>, b: Seq<u8>, i: int) -> bool
    decreases (a.len() - i) as nat,
{
    if i < 0 || i >= a.len() { false }
    else if i >= b.len() || a[i] > b[i] { true }
    else { a[i] == b[i] && later(a, b, i + 1) }
}

fn newer_stamp(a: &[u8], b: &[u8]) -> (yes: bool)
    ensures yes == later(a@, b@, 0),
{
    let mut i = 0usize;
    while i < a.len()
        invariant i <= a.len(), later(a@, b@, 0) == later(a@, b@, i as int),
        decreases a.len() - i,
    {
        if i >= b.len() || a[i] != b[i] { return i >= b.len() || a[i] > b[i]; }
        i += 1;
    }
    false
}

#[derive(Clone, Copy)]
pub struct ReleaseFacts<'a> {
    pub tag: &'a [u8],
    pub date: &'a [u8],
    pub draft: bool,
    pub preview: bool,
}

#[derive(Clone, Copy)]
pub enum Selection<'a> {
    Newest,
    Final(&'a [u8]),
    Preview(&'a [u8]),
}

pub open spec fn eligible(r: ReleaseFacts, kind: Selection) -> bool {
    !r.draft && r.date@.len() > 0 && match kind {
        Selection::Newest => true,
        Selection::Final(tag) => !r.preview && r.tag@ == tag@,
        Selection::Preview(tag) => r.preview && tag.len() < r.tag.len()
            && marker_at(r.tag@, tag@, 0) && r.tag@[tag.len() as int] == 45,
    }
}

fn accepts(r: ReleaseFacts, kind: Selection) -> (ok: bool)
    ensures ok == eligible(r, kind),
{
    if !has_publication(r.draft, !r.date.is_empty()) { return false; }
    match kind {
        Selection::Newest => true,
        Selection::Final(tag) => {
            if r.preview || r.tag.len() != tag.len() { return false; }
            let same = matches_at(r.tag, tag, 0);
            assert(r.tag@.subrange(0, tag.len() as int) =~= r.tag@);
            same
        },
        Selection::Preview(tag) => r.preview && tag.len() < r.tag.len()
            && matches_at(r.tag, tag, 0) && r.tag[tag.len()] == 45u8,
    }
}

// The stable tag map uses the last identical final. Newest/preview retain the
// first occurrence on a tied publication timestamp, including across API pages.
pub open spec fn choice(rows: Seq<ReleaseFacts>, kind: Selection, n: nat) -> Option<usize>
    decreases n,
{
    if n == 0 { None } else {
        let i = (n - 1) as int;
        let prev = choice(rows, kind, (n - 1) as nat);
        if eligible(rows[i], kind) && match prev {
            None => true,
            Some(k) => kind is Final || later(rows[i].date@, rows[k as int].date@, 0),
        } { Some(i as usize) } else { prev }
    }
}

pub fn select_release(rows: &[ReleaseFacts], kind: Selection) -> (best: Option<usize>)
    ensures best == choice(rows@, kind, rows.len() as nat),
            best.is_some() ==> best.unwrap() < rows.len(),
{
    let mut best: Option<usize> = None;
    for i in 0..rows.len()
        invariant best == choice(rows@, kind, i as nat),
                  best.is_some() ==> best.unwrap() < i,
    {
        let row = rows[i];
        let preferred = accepts(row, kind) && match best {
            None => true,
            Some(k) => matches!(kind, Selection::Final(_))
                || newer_stamp(row.date, rows[k].date),
        };
        if preferred { best = Some(i); }
        assert(best == choice(rows@, kind, (i+1) as nat));
    }
    best
}

pub fn event_or_newest<'a>(event: Option<&'a str>, newest: Option<&'a str>) -> (tag: Option<&'a str>)
    ensures tag == match event { Some(s) if s@.len() > 0 => event, _ => newest },
{
    match event { Some(s) if !s.is_empty() => Some(s), _ => newest }
}

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

pub open spec fn marker_at(text: Seq<u8>, marker: Seq<u8>, index: int) -> bool {
    0 <= index && index + marker.len() <= text.len()
        && text.subrange(index, index + marker.len()) == marker
}

pub open spec fn first_at(text: Seq<u8>, marker: Seq<u8>, index: int) -> bool {
    marker_at(text, marker, index)
        && forall|earlier: int| 0 <= earlier < index ==> !marker_at(text, marker, earlier)
}

fn matches_at(text: &[u8], marker: &[u8], index: usize) -> (found: bool)
    requires index <= text.len(), marker.len() <= text.len() - index,
    ensures found == marker_at(text@, marker@, index as int),
{
    let mut offset = 0;
    while offset < marker.len()
        invariant offset <= marker.len(), index <= text.len(), marker.len() <= text.len() - index,
            forall|j: int| 0 <= j < offset ==> text@[index + j] == marker@[j],
        decreases marker.len() - offset,
    {
        if text[index + offset] != marker[offset] {
            assert(text@.subrange(index as int, index + marker.len())[offset as int] != marker@[offset as int]);
            return false;
        }
        offset += 1;
    }
    assert(text@.subrange(index as int, index + marker.len()) =~= marker@);
    true
}

fn first_marker(text: &[u8], marker: &[u8]) -> (found: Option<usize>)
    requires marker.len() > 0,
    ensures
        found.is_some() ==> first_at(text@, marker@, found.unwrap() as int),
        found.is_none() ==> forall|i: int| !marker_at(text@, marker@, i),
{
    let mut index = 0;
    while index <= text.len() && marker.len() <= text.len() - index
        invariant index <= text.len(), marker.len() > 0,
            forall|i: int| 0 <= i < index ==> !marker_at(text@, marker@, i),
        decreases text.len() - index,
    {
        if matches_at(text, marker, index) { return Some(index); }
        index += 1;
    }
    None
}

/// Replace the first ordered pair. A lone or reversed first marker is invalid.
pub fn release_block(text: &[u8], begin: &[u8], end: &[u8], block: &[u8]) -> (result: Option<Vec<u8>>)
    requires begin.len() > 0, end.len() > 0,
    ensures
        !(exists|i:int| marker_at(text@,begin@,i)) && !(exists|i:int| marker_at(text@,end@,i)) ==>
            result.is_some() && result.unwrap()@ ==
                (if text.len() == 0 { block@ } else { text@ + seq![10u8, 10u8] + block@ }),
        forall|b:int, e:int| #[trigger] first_at(text@,begin@,b) && #[trigger] first_at(text@,end@,e) ==>
            (result.is_some() == (b <= e) && (b <= e ==> result.unwrap()@ ==
                text@.take(b) + block@ + text@.skip(e + end.len()))),
        ((exists|i:int| marker_at(text@,begin@,i)) != (exists|i:int| marker_at(text@,end@,i)))
            ==> result.is_none(),
{
    let mut output = Vec::new();
    match (first_marker(text, begin), first_marker(text, end)) {
        (None, None) => {
            if text.len() > 0 {
                output.extend_from_slice(text);
                output.push(10u8);
                output.push(10u8);
                assert(output@ =~= text@ + seq![10u8, 10u8]);
            }
            output.extend_from_slice(block);
        }
        (Some(b), Some(e)) if b <= e => {
            assert(first_at(text@,end@,e as int));
            assert(e + end.len() <= text.len());
            output.extend_from_slice(slice_subrange(text, 0, b));
            output.extend_from_slice(block);
            output.extend_from_slice(slice_subrange(text, e + end.len(), text.len()));
            assert(output@ =~= text@.take(b as int) + block@ + text@.skip(e + end.len()));
        }
        _ => return None,
    }
    Some(output)
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
    ReleaseSelection {
        releases: Vec<MilestoneRelease>,
        event_tag: Option<String>,
        milestones: Vec<String>,
    },
    Unassigned {
        issues: Vec<Issue>,
    },
    Milestone {
        final_release: bool,
        open_issues: u64,
    },
    ReleaseBlock {
        milestone: u64,
        original: String,
        lines: Vec<String>,
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
struct MilestoneRelease {
    tag_name: String,
    draft: bool,
    #[serde(default)]
    prerelease: bool,
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
    let result =
        match query {
            Query::Published { releases } => json!(
                releases
                    .iter()
                    .enumerate()
                    .filter_map(|(index, r)| {
                        published(r.draft, r.published_at.as_deref().unwrap_or("")).then_some(index)
                    })
                    .collect::<Vec<_>>()
            ),
            Query::ReleaseSelection {
                releases,
                event_tag,
                milestones,
            } => {
                let rows: Vec<_> = releases
                    .iter()
                    .map(|r| ReleaseFacts {
                        tag: r.tag_name.as_bytes(),
                        date: r.published_at.as_deref().unwrap_or("").as_bytes(),
                        draft: r.draft,
                        preview: r.prerelease,
                    })
                    .collect();
                let newest =
                    select_release(&rows, Selection::Newest).map(|i| releases[i].tag_name.as_str());
                let tag = event_or_newest(event_tag.as_deref(), newest);
                let mut selected = serde_json::Map::new();
                // The release event can create its milestone during this reconciliation.
                let released_milestone = tag.and_then(|value| value.split('-').next());
                for milestone in milestones
                    .iter()
                    .map(String::as_str)
                    .chain(released_milestone)
                {
                    selected.insert(milestone.to_owned(), json!({
                    "final": select_release(&rows, Selection::Final(milestone.as_bytes())),
                    "preview": select_release(&rows, Selection::Preview(milestone.as_bytes())),
                }));
                }
                json!({"tag":tag,"milestones":selected})
            }
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
            Query::ReleaseBlock {
                milestone,
                original,
                lines,
            } => {
                let begin = "<!-- chrono-harness release -->";
                let end = "<!-- /chrono-harness release -->";
                let block = std::iter::once(begin)
                    .chain(lines.iter().map(String::as_str))
                    .chain(std::iter::once(end))
                    .collect::<Vec<_>>()
                    .join("\n");
                let bytes = release_block(
                    original.as_bytes(),
                    begin.as_bytes(),
                    end.as_bytes(),
                    block.as_bytes(),
                )
                .ok_or_else(|| format!("Incomplete release block in milestone {milestone}"))?;
                json!(String::from_utf8(bytes).map_err(|e| e.to_string())?)
            }
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
