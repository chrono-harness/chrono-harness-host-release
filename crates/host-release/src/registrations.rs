//! Public registration views consumed by release recipes and preview selection.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

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

pub(crate) fn query<T: DeserializeOwned>(
    view: &str,
    projects: &Value,
    run: impl FnOnce(&str, &str, &[u8]) -> Result<(i32, Vec<u8>, Vec<u8>), String>,
) -> Result<T, String> {
    let bytes = serde_json::to_vec(projects).map_err(|error| error.to_string())?;
    let (code, stdout, stderr) = run("chrono-judge-registration", view, &bytes)?;
    if code != 0 {
        let message = String::from_utf8(stderr).map_err(|error| error.to_string())?;
        return Err(if message.is_empty() {
            format!("chrono-judge-registration {view} exited {code}")
        } else {
            message.strip_suffix('\n').unwrap_or(&message).to_owned()
        });
    }
    serde_json::from_slice(&stdout).map_err(|error| format!("invalid {view} response: {error}"))
}
