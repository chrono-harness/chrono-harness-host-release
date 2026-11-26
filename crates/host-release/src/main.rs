#[cfg(verus_keep_ghost)]
use super::same_text;
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::verified_recipe::same_text;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn template_run_command(args: &[String]) -> (selected: bool)
    ensures selected == (args.len() == 1 && args@[0]@ == "template-run-v1"@),
{
    args.len() == 1 && same_text(args[0].as_str(), "template-run-v1")
}

pub fn pushed_tag_command(args: &[String]) -> (requested: Option<bool>)
    ensures
        (requested == Some(false)) == (args.len() == 2 && args@[0]@ == "check-pushed-tag"@),
        (requested == Some(true)) == (args.len() == 3 && args@[0]@ == "check-pushed-tag"@
            && args@[2]@ == "--prerelease"@),
{
    if args.len() < 2 || !same_text(args[0].as_str(), "check-pushed-tag") {
        None
    } else if args.len() == 2 {
        Some(false)
    } else if args.len() == 3 && same_text(args[2].as_str(), "--prerelease") {
        Some(true)
    } else {
        None
    }
}

/// Only the exact installer's paging command selects a response-header file.
pub fn release_page_headers(args: &[String]) -> (headers: Option<&str>)
    ensures
        headers.is_some() == (args.len() == 3 && args@[0]@ == "resolve-install-v1"@
            && args@[1]@ == "--page-state"@),
        headers.is_some() ==> headers.unwrap()@ == args@[2]@,
{
    if args.len() == 3 && same_text(args[0].as_str(), "resolve-install-v1")
        && same_text(args[1].as_str(), "--page-state") {
        Some(args[2].as_str())
    } else {
        None
    }
}

/// The generated installer consumes these two complete tokens or the original parser error.
pub fn release_page_output(observed: Result<bool, String>) -> (out: Result<&'static str, String>)
    ensures match observed {
        Ok(true) => out is Ok && out.unwrap()@ == "next"@,
        Ok(false) => out is Ok && out.unwrap()@ == "done"@,
        Err(error) => out is Err && out.unwrap_err()@ == error@,
    },
{
    match observed {
        Ok(true) => Ok("next"),
        Ok(false) => Ok("done"),
        Err(error) => Err(error),
    }
}

pub fn select_preview_command(flag: &str) -> (selected: bool)
    ensures selected == (flag@ == "--select-preview"@),
{
    same_text(flag, "--select-preview")
}
}
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::{delivery, execution, frontend, local, recipe::Scope};
#[cfg(not(verus_keep_ghost))]
use std::{path::Path, process::ExitCode};

#[cfg(not(verus_keep_ghost))]
fn run() -> Result<i32, String> {
    let args = std::env::args_os()
        .skip(1)
        .map(|s| {
            s.into_string()
                .map_err(|_| "non UTF-8 release argument".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if template_run_command(&args) {
        chrono_host_release::template_run::check(std::io::stdin().lock())?;
        return Ok(0);
    }
    if let Some(requested) = pushed_tag_command(&args) {
        use chrono_host_release::identity::{admitted_tag_branch, pushed_tag_output};
        use std::io::Write;
        let branch = admitted_tag_branch(&args[1], |args| {
            let output = std::process::Command::new("git")
                .args(args)
                .stderr(std::process::Stdio::inherit())
                .output()
                .map_err(|e| e.to_string())?;
            Ok((
                output.status.code().unwrap_or(-1),
                String::from_utf8(output.stdout).map_err(|e| e.to_string())?,
            ))
        })?;
        std::io::stdout()
            .lock()
            .write_all(pushed_tag_output(&branch, requested).as_bytes())
            .map_err(|e| e.to_string())?;
        return Ok(0);
    }
    if let Some(headers) = release_page_headers(&args) {
        let raw = std::fs::read_to_string(headers).map_err(|e| format!("{headers}: {e}"))?;
        let output = release_page_output(chrono_host_release::template_run::release_page_state(&raw))?;
        println!("{output}");
        return Ok(0);
    }
    match args.as_slice() {
        [command] if command == "template-promotion-v1" => {
            let decision = chrono_host_release::template_run::promotion(std::io::stdin().lock())?;
            println!("{}", decision);
            Ok(0)
        },
        [command] if command == "template-binary-origins-v1" => {
            let origins = chrono_host_release::template_run::binary_origins(std::io::stdin().lock())?;
            println!("{}", origins);
            Ok(0)
        },
        [command] if command == "template-acceptance-v1" => {
            chrono_host_release::template_run::acceptance(std::io::stdin().lock())?;
            Ok(0)
        },
        [command, flag, binary, repo, selector, linux, macos, pages @ ..] if command == "resolve-install-v1" && flag == "--mise-tool" => {
            let tool = chrono_host_release::template_run::resolve_mise_tool(binary,repo,selector,linux,macos,pages)?;
            print!("{tool}");
            Ok(0)
        },
        [command] if command == "resolve-install-v1" => {
            let release = chrono_host_release::template_run::resolve_install(std::io::stdin().lock())?;
            println!("{release}");
            Ok(0)
        },
        [command] if command == "template-latest-stable-v1" => {
            let tag = chrono_host_release::template_run::latest_stable(std::io::stdin().lock())?;
            println!("{}", serde_json::to_string(&tag).map_err(|e| e.to_string())?);
            Ok(0)
        },
        [command] if command == "template-next-version-v1" => {
            let version = chrono_host_release::template_run::next_version(std::io::stdin().lock())?;
            println!("{}", serde_json::to_string(&version).map_err(|e| e.to_string())?);
            Ok(0)
        },
        [command] if command == "template-consumer-pin-v1" => {
            let pin = chrono_host_release::template_run::consumer_pin(std::io::stdin().lock())?;
            println!("{}", pin);
            Ok(0)
        },
        [command] if command == "template-publication-v1" => {
            let action = chrono_host_release::template_run::publication(std::io::stdin().lock())?;
            println!("{}", action);
            Ok(0)
        },
        [command] if command == "template-release-v1" => {
            chrono_host_release::template_run::release(std::io::stdin().lock())?;
            Ok(0)
        },
        [command] if command == "roadmap-v1" => chrono_host_release::verified_recipe::roadmap::run(std::io::stdin().lock(), std::io::stdout().lock()),
        [root, output, flag, config, collection] if select_preview_command(flag) => delivery::select_preview(
            Path::new(root), Path::new(output), config, collection, &std::env::vars_os().collect(),
        ),
        [root, output, flag, config, request] if flag == "--plan-preview" => {
            chrono_host_release::recipe::preview::write_plan(
                Path::new(root), Path::new(output), config, request, &std::env::vars_os().collect()
            )?;
            Ok(0)
        },
        [root, output, flag, config, native] if flag == "--install" => delivery::install(
            Path::new(root), Path::new(output), config, native, &std::env::vars_os().collect()
        ),
        [root, output, flag, config] if flag == "--deliver" => delivery::run(
            Path::new(root), Path::new(output), config, &std::env::vars_os().collect()
        ),
        [root, output] => local::run(
            Path::new(root),
            Path::new(output),
            std::env::vars_os().collect(),
        ),
        [root, output, flag, id] if flag == "--unit" => execution::scoped(
            Path::new(root),
            Path::new(output),
            Scope::Unit(id),
            std::env::vars_os().collect(),
        ),
        [root, output, flag] if flag == "--collect" => execution::scoped(
            Path::new(root),
            Path::new(output),
            Scope::Collect,
            std::env::vars_os().collect(),
        ),
        [root, output, flag] if flag == "--export-frontend" => {
            frontend::export(Path::new(root), Path::new(output), &std::env::vars_os().collect())?;
            Ok(0)
        },
        _ => Err("usage: chrono-host-release ROOT ABSENT_OUTPUT [--unit ID | --collect | --export-frontend | --deliver CONFIG | --install CONFIG NATIVE | --plan-preview CONFIG REQUEST | --select-preview CONFIG COLLECTION]\n       chrono-host-release check-pushed-tag TAG [--prerelease] | roadmap-v1 | template-run-v1 | template-acceptance-v1 | template-next-version-v1 | template-promotion-v1 | template-release-v1 | template-consumer-pin-v1".into()),
    }
}

#[cfg(not(verus_keep_ghost))]
fn main() -> ExitCode {
    let code = match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            2
        }
    };
    ExitCode::from(u8::try_from(code).unwrap_or(2))
}
