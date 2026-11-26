//! Public registration views consumed by release recipes and preview selection.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub enum Reply {
    Decode(Vec<u8>),
    Exit(i32),
    Diagnostic(Vec<u8>),
}

pub fn registration_reply(code: i32, stdout: Vec<u8>, stderr: Vec<u8>) -> (reply: Reply)
    ensures match reply {
        Reply::Decode(bytes) => code == 0 && bytes == stdout,
        Reply::Exit(status) => code != 0 && stderr.len() == 0 && status == code,
        Reply::Diagnostic(bytes) => code != 0 && stderr.len() > 0 && bytes == stderr,
    },
{
    if code == 0 { Reply::Decode(stdout) }
    else if stderr.is_empty() { Reply::Exit(code) }
    else { Reply::Diagnostic(stderr) }
}

/// A decoded error loses exactly one trailing LF. A lone LF remains an actual
/// diagnostic, with empty text after removal.
pub fn diagnostic_length(bytes: &[u8]) -> (n: usize)
    ensures n == if bytes.len() > 0 && bytes@[bytes.len() - 1] == 10 {
        bytes.len() - 1 } else { bytes.len() as int },
{
    let len = bytes.len();
    if len > 0 && bytes[len - 1] == 10 { len - 1 } else { len }
}
}

#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{collections::BTreeMap, path::Path};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub owner: String,
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<Value>,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn query<T: DeserializeOwned>(
    view: &str,
    projects: &Value,
    runner_directory: &Path,
    run: impl FnOnce(
        &str,
        &str,
        BTreeMap<String, String>,
        &[u8],
    ) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
) -> Result<T, String> {
    let bytes = serde_json::to_vec(projects).map_err(|error| error.to_string())?;
    let path = runner_directory
        .to_str()
        .ok_or("invalid runner directory")?;
    let env = BTreeMap::from([("PATH".into(), path.into())]);
    let (code, stdout, stderr) = run("chrono-judge-registration", view, env, &bytes)?;
    match registration_reply(code, stdout, stderr) {
        Reply::Decode(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid {view} response: {error}")),
        Reply::Exit(status) => Err(format!("chrono-judge-registration {view} exited {status}")),
        Reply::Diagnostic(bytes) => {
            let mut message = String::from_utf8(bytes).map_err(|error| error.to_string())?;
            message.truncate(diagnostic_length(message.as_bytes()));
            Err(message)
        }
    }
}
