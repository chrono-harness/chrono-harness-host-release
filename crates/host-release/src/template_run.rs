//! Read the selected GitHub run and its expected identity from the publisher.
use crate::verified_recipe::delivery::{acceptance_declared, acceptance_job, landed_push};
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    repository: String,
    workflow: String,
    branch: String,
    sha: String,
    run: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Acceptance {
    sha: String,
    required_jobs: Vec<String>,
    step: String,
    jobs: Vec<Value>,
}

fn read<T: serde::de::DeserializeOwned>(mut input: impl Read) -> Result<T, String> {
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    serde_json::from_value(crate::codec::json(&bytes)?).map_err(|e| e.to_string())
}

pub fn check(input: impl Read) -> Result<(), String> {
    let request: Request = read(input)?;
    let run = request.run;
    let fields = [
        run["repository"]["full_name"].as_str(),
        run["head_repository"]["full_name"].as_str(),
        run["path"].as_str(),
        run["head_branch"].as_str(),
        run["head_sha"].as_str(),
        run["event"].as_str(),
        run["status"].as_str(),
        run["conclusion"].as_str(),
    ];
    let workflow_path = format!(".github/workflows/{}", request.workflow);
    if landed_push(
        &fields,
        &request.repository,
        &workflow_path,
        &request.branch,
        &request.sha,
    ) {
        Ok(())
    } else {
        Err(format!(
            "unexpected GitHub run for {} on {} at {}",
            workflow_path, request.branch, request.sha
        ))
    }
}

pub fn acceptance(input: impl Read) -> Result<(), String> {
    let request: Acceptance = read(input)?;
    if !acceptance_declared(request.required_jobs.len(), &request.step) {
        return Err("missing consumer acceptance obligations".into());
    }
    for name in &request.required_jobs {
        // GitHub returns jobs in provider order. The publisher previously used the last job of a given name.
        let job = request
            .jobs
            .iter()
            .rev()
            .find(|job| job["name"].as_str() == Some(name.as_str()))
            .ok_or_else(|| format!("missing successful consumer on {name}"))?;
        let steps: Vec<_> = job["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|step| (step["name"].as_str(), step["conclusion"].as_str()))
            .collect();
        if !acceptance_job(
            job["name"].as_str(),
            name,
            job["conclusion"].as_str(),
            job["head_sha"].as_str(),
            &request.sha,
            &steps,
            &request.step,
        ) {
            return Err(format!("missing exercised product on {name}"));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionPlan {
    floor: String,
    latest: Option<String>,
}

fn components(core: &str) -> Result<[&[u8]; 3], String> {
    let parts: Vec<_> = core.split('.').collect();
    if !crate::verified_recipe::delivery::version_numbers(&parts) {
        return Err(format!("invalid release version {core}"));
    }
    Ok([
        parts[0].as_bytes(),
        parts[1].as_bytes(),
        parts[2].as_bytes(),
    ])
}

pub fn next_version(input: impl Read) -> Result<String, String> {
    use crate::verified_recipe::version_order;
    let request: VersionPlan = read(input)?;
    let floor = components(&request.floor)?;
    let Some(version) = request.latest else {
        return Ok(request.floor);
    };
    let latest = components(&version)?;
    if version_order::above(floor, latest) {
        return Ok(request.floor);
    }
    let digits = version_order::increment(latest[2]);
    let patch = std::str::from_utf8(&digits).map_err(|e| e.to_string())?;
    let (prefix, _) = version.rsplit_once('.').ok_or("invalid release version")?;
    Ok(format!("{prefix}.{patch}"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    tag: String,
    draft: bool,
    prerelease: bool,
    assets: std::collections::BTreeMap<String, Option<String>>,
    release: Value,
}

pub fn release(input: impl Read) -> Result<(), String> {
    use crate::verified_recipe::delivery::{boolean_fields, release_assets, text_fields};
    let request: Release = read(input)?;
    let release = request.release;
    if !text_fields(&[(release["tag_name"].as_str(), &request.tag)])
        || !boolean_fields(&[
            (release["draft"].as_bool(), request.draft),
            (release["prerelease"].as_bool(), request.prerelease),
        ])
    {
        return Err("unexpected release tag, draft or prerelease status".into());
    }
    let mut actual: Vec<_> = release["assets"]
        .as_array()
        .ok_or("missing release assets array")?
        .iter()
        .map(crate::publication::observed_asset)
        .collect();
    actual.sort_unstable_by_key(|row| row.0);
    let expected = request
        .assets
        .iter()
        .map(|(name, pin)| {
            let digest = pin
                .as_deref()
                .map(|s| s.strip_prefix("sha256:").ok_or("expected a sha256 pin"))
                .transpose()?;
            Ok((name.as_str(), digest))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if release_assets(&expected, &actual) {
        Ok(())
    } else {
        Err("release assets differ in names, upload state or checksums".into())
    }
}

#[derive(Deserialize)]
struct ReleaseRow {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
}

pub fn latest_stable(input: impl Read) -> Result<Option<String>, String> {
    use crate::verified_recipe::version_order;
    let rows: Vec<ReleaseRow> = read(input)?;
    let keys: Vec<_> = rows
        .iter()
        .map(|row| {
            let parts: Vec<_> = row
                .tag_name
                .strip_prefix('v')
                .unwrap_or("")
                .split('.')
                .collect();
            version_order::stable_key(
                &row.tag_name,
                &parts,
                row.draft,
                row.prerelease,
                row.published_at.as_deref().unwrap_or(""),
            )
        })
        .collect();
    Ok(version_order::latest_stable(&keys).map(|index| rows[index].tag_name.clone()))
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum ConsumerPin {
    Copier {
        repository: String,
        version: String,
        rolling: bool,
        answers: Value,
    },
    Binary {
        repository: String,
        version: String,
        binary: String,
        tool: Value,
    },
}

fn check_pin_version(value: &str, planned: &str, rolling: bool) -> Result<(), String> {
    let (core, serial) = match value.split_once("-alpha.") {
        Some((core, serial)) => (core, Some(serial)),
        None => (value, None),
    };
    let parts: Vec<_> = core.split('.').collect();
    if crate::verified_recipe::delivery::consumer_series(rolling, core, &parts, planned, serial) {
        Ok(())
    } else {
        Err("consumer has not adopted this preview series".into())
    }
}

pub fn consumer_pin(input: impl Read) -> Result<Value, String> {
    use crate::verified_recipe::delivery::{installed_download, text_fields};
    match read(input)? {
        ConsumerPin::Copier {
            repository,
            version,
            rolling,
            answers,
        } => {
            let source = answers["_src_path"].as_str();
            let urls = [
                format!("gh:{repository}"),
                format!("https://github.com/{repository}"),
                format!("https://github.com/{repository}.git"),
            ];
            if !urls.iter().any(|url| text_fields(&[(source, url)])) {
                return Err("consumer uses another template".into());
            }
            let tag = answers["_commit"].as_str().ok_or("missing template tag")?;
            check_pin_version(
                tag.strip_prefix('v').ok_or("expected a v-prefixed tag")?,
                &version,
                rolling,
            )?;
            Ok(serde_json::json!([tag, null]))
        }
        ConsumerPin::Binary {
            repository,
            version,
            binary,
            tool,
        } => {
            let pin = tool["version"].as_str().ok_or("missing binary version")?;
            check_pin_version(pin, &version, false)?;
            if !text_fields(&[(tool["bin"].as_str(), &binary)]) {
                return Err("wrong installed executable".into());
            }
            let mut checksums = std::collections::BTreeMap::new();
            for (platform, suffix) in [
                ("linux-x64", "linux-x86_64"),
                ("macos-arm64", "macos-aarch64"),
            ] {
                let asset = format!("{binary}-{suffix}");
                let expected =
                    format!("https://github.com/{repository}/releases/download/v{pin}/{asset}");
                let platform = &tool["platforms"][platform];
                let checksum = platform["checksum"]
                    .as_str()
                    .ok_or("missing platform checksum")?;
                if !installed_download(
                    platform["url"].as_str(),
                    &expected,
                    checksum.strip_prefix("sha256:"),
                ) {
                    return Err(format!("consumer URL or checksum differs for {asset}"));
                }
                checksums.insert(asset, checksum);
            }
            Ok(serde_json::json!([format!("v{pin}"), checksums]))
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum Publication {
    Ref {
        name: String,
        source: String,
        reference: Value,
    },
    Checksums {
        expected: Vec<String>,
        lines: Vec<String>,
    },
    State {
        tag: String,
        source: String,
        prerelease: bool,
        assets: std::collections::BTreeMap<String, String>,
        reference: Value,
        release: Value,
    },
}

fn ref_status(reference: &Value, name: &str, source: &str) -> u8 {
    crate::verified_recipe::delivery::selected_ref(
        !reference.is_null(),
        reference["ref"].as_str(),
        name,
        reference["object"]["type"].as_str(),
        reference["object"]["sha"].as_str(),
        source,
    )
}

pub fn publication(input: impl Read) -> Result<Value, String> {
    use crate::verified_recipe::delivery as policy;
    use serde_json::json;
    match read(input)? {
        Publication::Ref {
            name,
            source,
            reference,
        } => {
            if ref_status(&reference, &name, &source) != 1 {
                return Err(format!(
                    "reference {name} does not identify the selected commit"
                ));
            }
            Ok(Value::Null)
        }
        Publication::Checksums {
            mut expected,
            mut lines,
        } => {
            expected.sort_unstable();
            lines.sort_unstable();
            let expected: Vec<_> = expected.iter().map(String::as_str).collect();
            let actual: Vec<_> = lines.iter().map(String::as_str).collect();
            if !policy::checksum_lines(&expected, &actual) {
                return Err("asset checksums differ from SHA256SUMS".into());
            }
            Ok(Value::Null)
        }
        Publication::State {
            tag,
            source,
            prerelease,
            assets,
            reference,
            release,
        } => {
            let version = tag.strip_prefix('v').ok_or("expected a v-prefixed tag")?;
            let (core, serial) = version
                .split_once("-alpha.")
                .map_or((version, None), |(core, serial)| (core, Some(serial)));
            let parts: Vec<_> = core.split('.').collect();
            if (prerelease && !policy::consumer_series(false, core, &parts, core, serial))
                || (!prerelease && !policy::version_numbers(&parts))
                || (!prerelease && serial.is_some())
            {
                return Err("tag does not identify the selected publication channel".into());
            }
            if assets.values().any(|hash| !policy::valid_digest(hash)) {
                return Err("invalid expected publication digest".into());
            }
            let reference = ref_status(&reference, &format!("refs/tags/{tag}"), &source);
            let mut missing = assets;
            if !release.is_null() {
                if !policy::positive(release["id"].as_u64())
                    || !policy::text_fields(&[(release["tag_name"].as_str(), &tag)])
                    || !policy::boolean_fields(&[(release["prerelease"].as_bool(), prerelease)])
                {
                    return Err("existing release identity or channel differs".into());
                }
                for asset in release["assets"]
                    .as_array()
                    .ok_or("missing release assets array")?
                {
                    let row = crate::publication::observed_asset(asset);
                    let name = row.0.ok_or("missing asset name")?;
                    let expected = missing
                        .remove(name)
                        .ok_or("unexpected or duplicate existing asset")?;
                    if !policy::uploaded_asset((name, Some(&expected)), row) {
                        return Err(format!(
                            "existing release asset differs from selected bytes: {name}"
                        ));
                    }
                }
            }
            let action = match policy::publication_state(
                !release.is_null(),
                reference,
                release["draft"].as_bool(),
                release["target_commitish"].as_str(),
                &source,
                missing.len(),
            ) {
                0 => "create",
                1 => "resume",
                2 => "replay",
                _ => return Err("release or tag conflicts with the selected publication".into()),
            };
            Ok(
                json!({"action":action, "create_tag":reference==0, "missing":missing.keys().collect::<Vec<_>>()}),
            )
        }
    }
}
