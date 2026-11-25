//! Release configuration rejects duplicate JSON members before decoding typed declarations.
#[cfg(verus_keep_ghost)]
use super::same_text;
#[cfg(not(verus_keep_ghost))]
use crate::verified_recipe::same_text;
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub const DUPLICATE_HEADER: &str = "duplicate entry with key ";

/// Serde supplies the message and location. The literal decoder supplies the original member name.
pub fn duplicate_member<'a>(
    header: Option<&str>, member: Option<&'a str>, location: Option<&'a str>,
) -> (out: Option<(&'a str, &'a str)>)
    ensures out == match (header, member, location) {
        (Some(label), Some(key), Some(at)) if label@ == DUPLICATE_HEADER@ => Some((key, at)),
        _ => None,
    },
{
    match (header, member, location) {
        (Some(label), Some(key), Some(at)) if same_text(label, DUPLICATE_HEADER) => Some((key, at)),
        _ => None,
    }
}
}

#[cfg(not(verus_keep_ghost))]
pub fn json(bytes: &[u8]) -> Result<Value, String> {
    let value: serde_yaml_ng::Value =
        serde_json::from_slice(bytes).map_err(|error: serde_json::Error| {
            let message = error.to_string();
            let (body, location) = message.rsplit_once(" at line ").unzip();
            let (header, quoted) = body
                .and_then(|text| text.split_at_checked(DUPLICATE_HEADER.len()))
                .unzip();
            let member = quoted.and_then(|text| litrs::StringLit::parse(text).ok());
            if let Some((key, at)) =
                duplicate_member(header, member.as_ref().map(|key| key.value()), location)
            {
                return format!("duplicate JSON member {key} at line {at}");
            }
            message
        })?;
    serde_json::to_value(value).map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_value(json(bytes)?).map_err(|e| e.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub fn digest<T: serde::Serialize>(value: &T) -> Result<String, String> {
    Ok(crate::files::sha256(
        &serde_jcs::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
