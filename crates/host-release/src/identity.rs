//! 同一次发布的版本、固定源码及原 CI 身份；不读取会漂移的分支。
#[cfg(verus_keep_ghost)]
use super::delivery as policy;
#[cfg(not(verus_keep_ghost))]
use crate::verified_recipe::delivery as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {
/// GitHub's existing pushed tag grammar accepts leading zeros and an ASCII
/// prerelease suffix made of letters, digits, dots or hyphens.
pub enum TagBranch { Main, Dev, Invalid }

/// Output for an admitted pushed tag. The ordinary admission command is silent.
pub fn pushed_tag_output(branch: &TagBranch, requested: bool) -> (line: &'static str)
    ensures
        (requested && branch is Main) ==> line@ == "false\n"@,
        (requested && branch is Dev) ==> line@ == "true\n"@,
        (!requested || branch is Invalid) ==> line@ == ""@,
{
    match branch {
        TagBranch::Main if requested => "false\n",
        TagBranch::Dev if requested => "true\n",
        _ => "",
    }
}

pub open spec fn digit(b: u8) -> bool { 48 <= b <= 57 }
pub open spec fn preview_char(b: u8) -> bool {
    digit(b) || 65 <= b <= 90 || 97 <= b <= 122 || b == 46 || b == 45
}

fn tag_component(value: &[u8], start: usize, preview: bool) -> (valid: bool)
    ensures valid == (start < value.len() && forall|i: int| start <= i < value.len()
        ==> if preview { preview_char(#[trigger] value@[i]) } else { digit(#[trigger] value@[i]) }),
{
    if start >= value.len() { return false; }
    let mut i = start;
    while i < value.len()
        invariant start < value.len(), start <= i <= value.len(),
            forall|j: int| start <= j < i ==>
                if preview { preview_char(#[trigger] value@[j]) } else { digit(#[trigger] value@[j]) },
        decreases value.len() - i,
    {
        let b = value[i];
        if !((48 <= b && b <= 57) || (preview
            && ((65 <= b && b <= 90) || (97 <= b && b <= 122) || b == 46 || b == 45))) {
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn numeric_component(value: Seq<u8>, start: int) -> bool {
    start < value.len() && forall|i: int| start <= i < value.len() ==> digit(#[trigger] value[i])
}

pub open spec fn tag_core(parts: Seq<&[u8]>) -> bool {
    parts.len() == 3 && parts[0]@.len() > 0 && parts[0]@[0] == 118
        && numeric_component(parts[0]@,1) && numeric_component(parts[1]@,0) && numeric_component(parts[2]@,0)
}

pub open spec fn preview_suffix(suffix: Option<&[u8]>) -> bool {
    match suffix {
        Some(s) => s.len() > 0 && forall|i: int| 0 <= i < s.len() ==> preview_char(#[trigger] s@[i]),
        None => false,
    }
}

pub fn tag_branch(parts: &[&[u8]], suffix: Option<&[u8]>) -> (branch: TagBranch)
    ensures (branch is Main) == (tag_core(parts@) && suffix.is_none()),
            (branch is Dev) == (tag_core(parts@) && preview_suffix(suffix)),
            (branch is Invalid) == (!tag_core(parts@) || (suffix.is_some() && !preview_suffix(suffix))),
{
    if parts.len() != 3 || parts[0].len() == 0 || parts[0][0] != 118
        || !tag_component(parts[0],1,false) || !tag_component(parts[1],0,false)
        || !tag_component(parts[2],0,false) { return TagBranch::Invalid; }
    match suffix {
        None => TagBranch::Main,
        Some(s) if tag_component(s,0,true) => TagBranch::Dev,
        Some(_) => TagBranch::Invalid,
    }
}

pub fn pushed_main(source: &str, main: &str) -> (admit: bool)
    ensures admit == (policy::release_oid(source.spec_bytes()) && source@ == main@),
{
    policy::valid_release_oid(source.as_bytes()) && policy::text_fields(&[(Some(source),main)])
}

pub fn pushed_dev(ancestor_status: i32) -> (admit: bool)
    ensures admit == (ancestor_status == 0),
{ ancestor_status == 0 }

pub open spec fn segment(all: Seq<u8>, at: int, part: Seq<u8>) -> bool {
    0 <= at && at + part.len() <= all.len()
        && forall|i: int| 0 <= i < part.len() ==> all[at+i] == part[i]
}

fn matches_at(all: &[u8], at: usize, part: &[u8]) -> (ok: bool)
    ensures ok == segment(all@, at as int, part@),
{
    if at > all.len() || part.len() > all.len() - at { return false; }
    let mut i = 0usize;
    while i < part.len()
        invariant
            i <= part.len(), at <= all.len(), part.len() <= all.len()-at,
            forall|j: int| 0 <= j < i ==> all@[at as int+j] == part@[j],
        decreases part.len()-i,
    {
        if all[at+i] != part[i] { return false; }
        i += 1;
    }
    true
}

pub fn workflow_path(path: &[u8]) -> (ok: bool)
    ensures ok == (segment(path@, 0, ".github/workflows/".spec_bytes())
        && segment(path@, path.len() - ".yml".spec_bytes().len(), ".yml".spec_bytes())),
{
    let suffix = ".yml".as_bytes();
    path.len() >= suffix.len() && matches_at(path, 0, ".github/workflows/".as_bytes())
        && matches_at(path, path.len()-suffix.len(), suffix)
}

pub fn fixed_revisions(workflow: &[u8], source: &[u8]) -> (ok: bool)
    ensures ok == (policy::release_oid(workflow@) && policy::release_oid(source@)),
{
    policy::valid_release_oid(workflow) && policy::valid_release_oid(source)
}

/// GitHub supplies repository/path@ref. The ref must have a nonempty suffix.
pub fn workflow_environment(repo: &[u8], path: &[u8], reference: &[u8], workflow: &[u8], source: &[u8]) -> (ok: bool)
    ensures ok == (repo.len() > 0 && policy::release_oid(workflow@) && policy::release_oid(source@)
        && segment(reference@, 0, repo@) && reference.len() > repo.len()
        && reference@[repo.len() as int] == 47
        && segment(reference@, repo.len()+1, path@)
        && reference.len() > repo.len()+path.len()+2
        && reference@[repo.len()+path.len()+1] == 64),
{
    if repo.len() == 0 || !fixed_revisions(workflow, source)
        || !matches_at(reference, 0, repo) || reference.len() <= repo.len()
        || reference[repo.len()] != 47 { return false; }
    let start = repo.len()+1;
    if !matches_at(reference, start, path) { return false; }
    let end = start+path.len();
    reference.len()-end > 1 && reference[end] == 64
}
}

#[cfg(not(verus_keep_ghost))]
use crate::execution::Environment;
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::ffi::OsStr;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg(not(verus_keep_ghost))]
pub enum Channel {
    Build,
    Preview,
    Stable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct VerifiedCi {
    pub run_id: u64,
    pub attempt: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Identity {
    pub channel: Channel,
    pub version: String,
    pub source_commit: String,
    pub source_tree: String,
    pub verified_ci: Option<VerifiedCi>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct WorkflowIdentity {
    pub path: String,
    pub workflow_commit: String,
    pub source_commit: String,
    pub blob: String,
}

#[cfg(not(verus_keep_ghost))]
fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_IDENTITY: {message}")
}

#[cfg(not(verus_keep_ghost))]
fn text<'a>(environment: &'a Environment, name: &str) -> Result<&'a str, String> {
    environment
        .get(OsStr::new(name))
        .map(|v| v.to_str().ok_or_else(|| error(format!("non UTF-8 {name}"))))
        .unwrap_or(Ok(""))
}

/// GitHub 的工作流修订可以前移；只有同一路径的完整字节相同才可消费本候选。
#[cfg(not(verus_keep_ghost))]
pub fn verify_workflow(
    path: &str,
    source_commit: &str,
    environment: &Environment,
    commands: &mut impl crate::publication::Commands,
) -> Result<WorkflowIdentity, String> {
    crate::files::relative_path(path).map_err(error)?;
    if !workflow_path(path.as_bytes()) {
        return Err(error("workflow path must be a declared GitHub workflow"));
    }
    let repository = text(environment, "GITHUB_REPOSITORY")?;
    let workflow_commit = text(environment, "GITHUB_WORKFLOW_SHA")?;
    let workflow_ref = text(environment, "GITHUB_WORKFLOW_REF")?;
    if !workflow_environment(
        repository.as_bytes(),
        path.as_bytes(),
        workflow_ref.as_bytes(),
        workflow_commit.as_bytes(),
        source_commit.as_bytes(),
    ) {
        return Err(error("missing or mismatched GitHub workflow identity"));
    }
    verify_workflow_commits(path, source_commit, workflow_commit, commands)
}

/// 只读核对由 GitHub 原运行 API 取得的工作流提交与固定候选。
#[cfg(not(verus_keep_ghost))]
pub(crate) fn verify_workflow_commits(
    path: &str,
    source_commit: &str,
    workflow_commit: &str,
    commands: &mut impl crate::publication::Commands,
) -> Result<WorkflowIdentity, String> {
    crate::files::relative_path(path).map_err(error)?;
    if !workflow_path(path.as_bytes())
        || !fixed_revisions(workflow_commit.as_bytes(), source_commit.as_bytes())
    {
        return Err(error("invalid fixed workflow identity"));
    }
    let mut blob = |commit: &str| -> Result<String, String> {
        let reply = commands
            .execute(
                "git",
                &[
                    "--no-replace-objects".into(),
                    "rev-parse".into(),
                    "--verify".into(),
                    "--end-of-options".into(),
                    format!("{commit}:{path}"),
                ],
            )
            .map_err(error)?;
        if reply.exit != 0 {
            return Err(error(format!(
                "workflow Git observation exited {} for {commit}:{path}",
                reply.exit
            )));
        }
        let value = reply
            .stdout
            .strip_suffix(b"\n")
            .filter(|oid| crate::verified_recipe::delivery::valid_release_oid(oid))
            .ok_or_else(|| error("incomplete workflow blob identity"))?;
        String::from_utf8(value.to_vec()).map_err(error)
    };
    let workflow_blob = blob(workflow_commit)?;
    let source_blob = if workflow_commit == source_commit {
        workflow_blob.clone()
    } else {
        blob(source_commit)?
    };
    if workflow_blob != source_blob {
        return Err(error(format!(
            "executing workflow {workflow_commit}:{path} differs from candidate {source_commit}:{path}; dispatch from the fixed candidate ref"
        )));
    }
    Ok(WorkflowIdentity {
        path: path.into(),
        workflow_commit: workflow_commit.into(),
        source_commit: source_commit.into(),
        blob: workflow_blob,
    })
}

#[cfg(not(verus_keep_ghost))]
fn positive(environment: &Environment, name: &str) -> Result<u64, String> {
    let value = text(environment, name)?;
    if !policy::decimal_text(value.as_bytes(), false) {
        return Err(error(format!("{name} must be a positive decimal identity")));
    }
    value.parse().map_err(error)
}

#[cfg(not(verus_keep_ghost))]
fn core(version: &str) -> Result<&str, String> {
    let core = version.split('-').next().unwrap_or("");
    let numbers: Vec<_> = core.strip_prefix('v').unwrap_or("").split('.').collect();
    if !policy::version_numbers(&numbers) {
        return Err(error(
            "release plan needs a vMAJOR.MINOR.PATCH version core",
        ));
    }
    Ok(core)
}

#[cfg(not(verus_keep_ghost))]
pub fn resolve(
    planned_version: &str,
    source_commit: &str,
    source_tree: &str,
    environment: &Environment,
) -> Result<Identity, String> {
    if !crate::verified_recipe::delivery::valid_release_oid(source_commit.as_bytes())
        || !crate::verified_recipe::delivery::valid_release_oid(source_tree.as_bytes())
    {
        return Err(error(
            "source observation must contain complete lowercase commit/tree OIDs",
        ));
    }
    let selected_channel = policy::channel(text(environment, "CHRONO_RELEASE_CHANNEL")?);
    let channel = match selected_channel {
        0 => Channel::Build,
        1 => Channel::Preview,
        2 => Channel::Stable,
        _ => return Err(error("unknown release channel")),
    };
    let mut identity = Identity {
        channel,
        version: planned_version.into(),
        source_commit: source_commit.into(),
        source_tree: source_tree.into(),
        verified_ci: None,
    };
    if identity.channel == Channel::Build {
        return Ok(identity);
    }
    if !policy::text_fields(&[(
        Some(text(environment, "CHRONO_RELEASE_SOURCE")?),
        source_commit,
    )]) {
        return Err(error(
            "requested source differs from the observed fixed commit",
        ));
    }
    let ci = VerifiedCi {
        run_id: positive(environment, "CHRONO_RELEASE_CI_RUN_ID")?,
        attempt: positive(environment, "CHRONO_RELEASE_CI_ATTEMPT")?,
    };
    let version_core = core(planned_version)?;
    let requested_version = text(environment, "CHRONO_RELEASE_VERSION")?;
    match identity.channel {
        Channel::Preview => {
            if !policy::requested_version(
                selected_channel,
                requested_version,
                version_core,
                planned_version,
            ) {
                return Err(error(
                    "preview version is derived from its CI and source identities",
                ));
            }
            identity.version = format!(
                "{version_core}-dev.{}.{}.g{source_commit}",
                ci.run_id, ci.attempt
            );
        }
        Channel::Stable => {
            if !policy::requested_version(
                selected_channel,
                requested_version,
                version_core,
                planned_version,
            ) {
                return Err(error(
                    "stable delivery needs an explicit final version matching the fixed plan",
                ));
            }
        }
        Channel::Build => unreachable!("build returned before publication bindings"),
    }
    identity.verified_ci = Some(ci);
    Ok(identity)
}

/// Git argument vectors are fixed. The host only dispatches the GitHub release
/// build after this command returns success.
#[cfg(not(verus_keep_ghost))]
pub fn check_pushed_tag(
    tag: &str,
    git: impl FnMut(&[&str]) -> Result<(i32, String), String>,
) -> Result<(), String> {
    admitted_tag_branch(tag, git).map(|_| ())
}

#[cfg(not(verus_keep_ghost))]
pub fn admitted_tag_branch(
    tag: &str,
    mut git: impl FnMut(&[&str]) -> Result<(i32, String), String>,
) -> Result<TagBranch, String> {
    let (core, suffix) = match tag.split_once('-') {
        Some((core, suffix)) => (core, Some(suffix.as_bytes())),
        None => (tag, None),
    };
    let parts: Vec<_> = core.splitn(4, '.').map(str::as_bytes).collect();
    let branch = tag_branch(&parts, suffix);
    if matches!(branch, TagBranch::Invalid) {
        return Err(format!("Unsupported release tag: {tag}"));
    }
    let mut revision = |reference: &str| {
        let (exit, raw) = git(&["rev-parse", reference])?;
        if exit != 0 {
            return Err(format!("git rev-parse {reference} exited {exit}"));
        }
        let oid = raw.trim_end_matches('\n').to_owned();
        if !policy::valid_release_oid(oid.as_bytes()) {
            return Err(format!("invalid Git OID for {reference}"));
        }
        Ok(oid)
    };
    let source = revision("HEAD")?;
    match branch {
        TagBranch::Main => {
            let main = revision("refs/remotes/origin/main")?;
            if pushed_main(&source, &main) {
                Ok(branch)
            } else {
                Err("stable release source differs from main".into())
            }
        }
        TagBranch::Dev => {
            let (exit, _) = git(&[
                "merge-base",
                "--is-ancestor",
                &source,
                "refs/remotes/origin/dev",
            ])?;
            if pushed_dev(exit) {
                Ok(branch)
            } else {
                Err(format!("git merge-base --is-ancestor exited {exit}"))
            }
        }
        TagBranch::Invalid => unreachable!(),
    }
}
