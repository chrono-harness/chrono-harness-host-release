//! Release consumes the original build producer's public report protocol.
use crate::execution::Environment;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceObservation {
    pub commit: Option<String>,
    pub tree: Option<String>,
    pub dirty: Option<bool>,
    pub error: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativePlatform {
    pub system: String,
    pub machine: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkout {
    pub source: SourceObservation,
    pub platform: NativePlatform,
    pub report: String,
    pub summary: Summary,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub path: String,
    pub sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub destinations: Vec<String>,
    pub installed: Vec<Installation>,
    pub streams: Vec<Value>,
}

pub fn query<T: DeserializeOwned>(
    root: &Path,
    environment: &Environment,
    request: &Value,
    timeout_seconds: u64,
) -> Result<T, String> {
    // The reader invokes local Git and the installed registration command.
    // Git uses the user's ignore/config paths when observing checkout cleanliness.
    let mut env = BTreeMap::new();
    for key in ["PATH", "HOME", "XDG_CONFIG_HOME"] {
        if let Some(value) = environment.get(OsStr::new(key)) {
            env.insert(
                key.to_owned(),
                value
                    .to_str()
                    .ok_or_else(|| format!("non-UTF8 {key}"))?
                    .to_owned(),
            );
        }
    }
    let search = env.get("PATH").ok_or("missing PATH for bootstrap query")?;
    let exe = chrono_harness::resolve_program(root, "chrono-host-bootstrap", Some(search))?;
    let digest = chrono_harness::file_identity(&exe)?.0;
    let spec = chrono_harness::CommandSpec {
        program: exe
            .to_str()
            .ok_or("bootstrap executable path must be UTF-8")?
            .into(),
        args: vec!["original-report-v1".into()],
        env,
        timeout_seconds,
        output_limit_bytes: chrono_harness::verified_limits::MAX_OUTPUT_BYTES,
    };
    let bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
    let result = chrono_harness::run_process_observed(root, &spec, &bytes, &digest)?;
    if let Some(e) = result.failure {
        return Err(e);
    }
    if result.exit_code != 0 {
        return Err(format!(
            "bootstrap original report exited {}: {}",
            result.exit_code,
            result.stderr.trim_end()
        ));
    }
    chrono_harness::decode(&result.stdout_bytes)
}

pub fn native_platform(timeout_seconds: u64) -> Result<NativePlatform, String> {
    query(
        &std::env::current_dir().map_err(|e| e.to_string())?,
        &std::env::vars_os().collect(),
        &json!({"operation":"platform"}),
        timeout_seconds,
    )
}
