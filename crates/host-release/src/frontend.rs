//! 发布工具自己的生产与消费证据；复用 bootstrap 原报告及现有单执行物运输。
#[cfg(verus_keep_ghost)]
use super::same_text;
#[cfg(not(verus_keep_ghost))]
use crate::verified_recipe::same_text;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn artifact_paths(executable: &str, expected_executable: &str, report: &str, expected_report: &str, archive: &str) -> (valid: bool)
    ensures valid == (executable@ == expected_executable@ && report@ == expected_report@ && archive@ == "executable.tar"@),
{
    same_text(executable, expected_executable) && same_text(report, expected_report)
        && same_text(archive, "executable.tar")
}
}
#[cfg(not(verus_keep_ghost))]
use crate::bootstrap::{self, Checkout, NativePlatform, SourceObservation, Summary};
#[cfg(not(verus_keep_ghost))]
use crate::{
    execution::Environment,
    receipt::{self, Kind, Lineage, Selection},
    recipe::{Loaded, NativeFrontend},
    transport::{self, ExecutableIdentity},
    verified_recipe as policy,
};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, ffi::OsStr, fs, io::Write, path::Path};

#[cfg(not(verus_keep_ghost))]
const INSTALLED: &str = ".chrono-harness/bin/chrono-host-release";

#[cfg(not(verus_keep_ghost))]
fn error(message: impl std::fmt::Display) -> String {
    format!("E_RELEASE_FRONTEND: {message}")
}

#[cfg(not(verus_keep_ghost))]
fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value[field]
        .as_str()
        .ok_or_else(|| error(format!("missing {field}")))
}

#[cfg(not(verus_keep_ghost))]
fn variable<'a>(environment: &'a Environment, name: &str) -> Result<&'a str, String> {
    environment
        .get(OsStr::new(name))
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error(format!("missing {name}")))
}

#[cfg(not(verus_keep_ghost))]
fn qualify(report: &Summary, identity: &ExecutableIdentity) -> Result<(), String> {
    let planned = report.destinations.first();
    let installed = report.installed.first();
    if !policy::frontend_installation(
        report.destinations.len() as u64,
        report.installed.len() as u64,
        planned.map(String::as_str).unwrap_or(""),
        installed.map(|p| p.path.as_str()).unwrap_or(""),
        installed.map(|p| p.sha256.as_str()).unwrap_or(""),
        &identity.sha256,
    ) {
        return Err(error(
            "original bootstrap installation differs from release frontend",
        ));
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn write(directory: &Path, relative: &str, raw: &[u8]) -> Result<Value, String> {
    let path = chrono_harness::no_symlink_parents(directory, relative)?;
    fs::create_dir_all(path.parent().ok_or("missing original parent")?).map_err(error)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(error)?;
    file.write_all(raw)
        .and_then(|()| file.flush())
        .map_err(error)?;
    Ok(json!({"path":relative,"size":raw.len(),"sha256":chrono_harness::sha256(raw)}))
}

#[cfg(not(verus_keep_ghost))]
fn actual(path: &Path) -> Result<ExecutableIdentity, String> {
    let parent = path.parent().ok_or("missing frontend parent")?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid frontend filename")?;
    transport::observe(parent, name)
}

#[cfg(not(verus_keep_ghost))]
fn same_content(actual: &ExecutableIdentity, expected: &ExecutableIdentity) -> bool {
    policy::same_content(&actual.sha256, actual.size, &expected.sha256, expected.size)
}

#[cfg(not(verus_keep_ghost))]
fn same(actual: &ExecutableIdentity, expected: &ExecutableIdentity) -> bool {
    policy::same_executable(
        &actual.sha256,
        actual.size,
        actual.mode,
        &expected.sha256,
        expected.size,
        expected.mode,
    )
}

/// 只导出已经成功构建并安装、且实际正在执行的工具；最后才发布清单。
#[cfg(not(verus_keep_ghost))]
pub fn export(root: &Path, output: &Path, environment: &Environment) -> Result<(), String> {
    let root = fs::canonicalize(root).map_err(error)?;
    let loaded = Loaded::load(&root)?;
    let lineage = Lineage {
        kind: Kind::Github,
        run_id: variable(environment, "CHRONO_RELEASE_RUN")?.into(),
        attempt: variable(environment, "CHRONO_RELEASE_ATTEMPT")?.into(),
        job: variable(environment, "CHRONO_RELEASE_JOB")?.into(),
    };
    lineage.validate()?;
    let declared = loaded
        .recipe
        .native_frontends
        .values()
        .find(|f| f.job == lineage.job)
        .ok_or_else(|| error("unregistered frontend producing job"))?;
    let Checkout {
        platform: native,
        source,
        report: raw,
        summary,
    } = bootstrap::query(
        &root,
        environment,
        &json!({"operation":"checkout","profile":declared.bootstrap_profile}),
        loaded.recipe.limits.unit_seconds,
    )?;
    let identity = transport::observe(&root, INSTALLED)?;
    // Cargo may execute the freshly built frontend with a different mode from
    // the bootstrap installation. The installed identity remains the source
    // for archive and consumer mode checks below.
    if !same_content(
        &actual(&std::env::current_exe().map_err(error)?)?,
        &identity,
    ) {
        return Err(error(
            "executing frontend differs from bootstrap installation",
        ));
    }
    qualify(&summary, &identity)?;
    // create_dir 拒绝任何已有输出，包括链接；失败的部分输出保留供诊断。
    fs::create_dir(output).map_err(error)?;
    let workflow = if environment
        .get(OsStr::new("GITHUB_ACTIONS"))
        .is_some_and(|value| value == "true")
    {
        let config = crate::recipe::read_json(&root, ".chrono-harness/ci/release.json")?;
        Some(crate::delivery::workflow_preflight(
            &root,
            output,
            &loaded,
            text(&config, "workflow_path")?,
            source
                .commit
                .as_deref()
                .ok_or("missing frontend source commit")?,
            environment,
        )?)
    } else {
        None
    };
    let packed = transport::pack(&root, INSTALLED, &output.join("executable.tar"))?;
    if !same(&packed.executable, &identity) {
        return Err(error("installed frontend changed during export"));
    }
    let bootstrap = write(output, &declared.bootstrap_report, raw.as_bytes())?;
    for stream in summary.streams {
        let path = receipt::original(&root, &stream)?;
        write(
            output,
            text(&stream, "path")?,
            &fs::read(path).map_err(error)?,
        )?;
    }
    let after: SourceObservation = bootstrap::query(
        &root,
        environment,
        &json!({"operation":"source"}),
        loaded.recipe.limits.unit_seconds,
    )?;
    if after != source {
        return Err(error("frontend source changed during export"));
    }
    let manifest = json!({"schema":"chrono-release-frontend/v1", "source":source,
        "system":native.system,"machine":native.machine,"lineage":lineage,
        "executable":identity.receipt(&declared.executable),"archive":packed.archive.receipt(),"bootstrap":bootstrap,"workflow":workflow});
    write(
        output,
        "frontend.json",
        &serde_json::to_vec_pretty(&manifest).map_err(error)?,
    )?;
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub fn declaration<'a>(
    loaded: &'a Loaded,
    selected: &Selection,
) -> Result<&'a NativeFrontend, String> {
    loaded
        .recipe
        .native_frontends
        .values()
        .find(|f| f.job == selected.job)
        .ok_or_else(|| error("unregistered selected frontend job"))
}

/// 单元保留自己的原工具生产者元数据；汇总不要求它来自当前 attempt。
#[cfg(not(verus_keep_ghost))]
pub fn validate_metadata(
    root: &Path,
    directory: &Path,
    declared: &NativeFrontend,
    native: &NativePlatform,
    source: &SourceObservation,
    selected: &Selection,
    timeout_seconds: u64,
) -> Result<Value, String> {
    validate_producer_metadata(
        root,
        directory,
        declared,
        native,
        source,
        selected,
        None,
        timeout_seconds,
    )
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn validate_producer_metadata(
    root: &Path,
    directory: &Path,
    declared: &NativeFrontend,
    native: &NativePlatform,
    source: &SourceObservation,
    selected: &Selection,
    revision: Option<(&str, &str)>,
    timeout_seconds: u64,
) -> Result<Value, String> {
    let manifest = crate::recipe::read_json(directory, "frontend.json")?;
    let lineage: Lineage = serde_json::from_value(manifest["lineage"].clone()).map_err(error)?;
    let declared_source: SourceObservation =
        serde_json::from_value(manifest["source"].clone()).map_err(error)?;
    lineage.validate()?;
    if &declared_source != source {
        return Err(error("frontend source observation differs"));
    }
    if !policy::frontend_header(
        text(&manifest, "schema")?,
        matches!(lineage.kind, Kind::Github),
        text(&manifest["source"], "commit")?,
        text(&manifest["source"], "tree")?,
        text(&manifest, "system")?,
        text(&manifest, "machine")?,
        source.commit.as_deref().unwrap_or(""),
        source.tree.as_deref().unwrap_or(""),
        &native.system,
        &native.machine,
    ) || selected.job != declared.job
        || !policy::producer_lineage(
            &lineage.job,
            &declared.job,
            &lineage.run_id,
            &selected.run_id,
            lineage.attempt.parse().map_err(error)?,
            selected.attempt.parse().map_err(error)?,
        )
    {
        return Err(error(
            "frontend source, platform or original production differs",
        ));
    }
    if !artifact_paths(
        text(&manifest["executable"], "path")?,
        &declared.executable,
        text(&manifest["bootstrap"], "path")?,
        &declared.bootstrap_report,
        text(&manifest["archive"], "path")?,
    ) {
        return Err(error("frontend declared artifact paths differ"));
    }
    let identity = ExecutableIdentity::from_receipt(&manifest["executable"])?;
    let raw = fs::read(receipt::original(directory, &manifest["bootstrap"])?).map_err(error)?;
    let report: Summary = bootstrap::query(
        root,
        &std::env::vars_os().collect(),
        &json!({
            "operation":"retained","profile":declared.bootstrap_profile,"platform":native,
            "source":source,"report":std::str::from_utf8(&raw).map_err(error)?,
            "directory":fs::canonicalize(directory).map_err(error)?,
            "revision":revision.map(|(commit,tree)|json!({"commit":commit,"tree":tree}))
        }),
        timeout_seconds,
    )?;
    qualify(&report, &identity)?;
    Ok(manifest)
}

/// needs 已选出原生产者；本边界另核原构建、工具字节和实际执行物。
/// collector 不执行依赖工具，传 None；真正消费者传当前执行物路径。
#[cfg(not(verus_keep_ghost))]
pub fn validate(
    root: &Path,
    directory: &Path,
    declared: &NativeFrontend,
    native: &NativePlatform,
    source: &SourceObservation,
    selected: &Selection,
    executing: Option<&Path>,
    timeout_seconds: u64,
) -> Result<Value, String> {
    let manifest = validate_metadata(
        root,
        directory,
        declared,
        native,
        source,
        selected,
        timeout_seconds,
    )?;
    let identity = ExecutableIdentity::from_receipt(&manifest["executable"])?;
    let archive = receipt::original(directory, &manifest["archive"])?;
    transport::validate(
        &archive,
        &transport::ArchiveIdentity {
            sha256: text(&manifest["archive"], "sha256")?.into(),
            size: manifest["archive"]["size"]
                .as_u64()
                .ok_or("missing archive size")?,
        },
        &identity,
    )?;
    if let Some(path) = executing {
        let expected = chrono_harness::no_symlink_parents(directory, &declared.executable)?;
        if fs::canonicalize(path).map_err(error)? != fs::canonicalize(expected).map_err(error)?
            || !same(&actual(path)?, &identity)
        {
            return Err(error(
                "actual frontend differs from selected bootstrap artifact",
            ));
        }
    }
    Ok(manifest)
}

/// 逐个列出原元数据；不扫描目录，也不复制二进制或其归档。
#[cfg(not(verus_keep_ghost))]
pub fn metadata_files(
    directory: &Path,
    timeout_seconds: u64,
) -> Result<BTreeMap<String, Value>, String> {
    let path = chrono_harness::no_symlink_parents(directory, "frontend.json")?;
    let (sha256, size) = chrono_harness::file_identity(&path)?;
    let manifest = crate::recipe::read_json(directory, "frontend.json")?;
    let raw = fs::read(receipt::original(directory, &manifest["bootstrap"])?).map_err(error)?;
    let streams: Vec<Value> = bootstrap::query(
        directory,
        &std::env::vars_os().collect(),
        &json!({"operation":"streams","report":std::str::from_utf8(&raw).map_err(error)?}),
        timeout_seconds,
    )?;
    let mut files = BTreeMap::from([(
        "frontend.json".into(),
        json!({"path":"frontend.json","sha256":sha256,"size":size}),
    )]);
    let mut insert = |value: Value| -> Result<(), String> {
        let name = text(&value, "path")?.to_owned();
        receipt::original(directory, &value)?;
        if files.insert(name, value).is_some() {
            return Err(error("duplicate frontend metadata path"));
        }
        Ok(())
    };
    insert(manifest["bootstrap"].clone())?;
    for stream in streams {
        insert(stream)?;
    }
    Ok(files)
}

#[cfg(not(verus_keep_ghost))]
pub fn retain_metadata(
    directory: &Path,
    output: &Path,
    timeout_seconds: u64,
) -> Result<BTreeMap<String, Value>, String> {
    let files = metadata_files(directory, timeout_seconds)?;
    fs::create_dir(output).map_err(error)?;
    for (name, declaration) in &files {
        let path = receipt::original(directory, declaration)?;
        let retained = write(output, name, &fs::read(path).map_err(error)?)?;
        if retained != *declaration {
            return Err(error("frontend metadata changed during retention"));
        }
    }
    Ok(files)
}
