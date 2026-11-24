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
