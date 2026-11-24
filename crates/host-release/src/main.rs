#[cfg(verus_keep_ghost)]
use super::same_text;
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::verified_recipe::same_text;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
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
    match args.as_slice() {
        [command] if command == "template-acceptance-v1" => {
            chrono_host_release::template_run::acceptance(std::io::stdin().lock())?;
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
        [command] if command == "template-release-v1" => {
            chrono_host_release::template_run::release(std::io::stdin().lock())?;
            Ok(0)
        },
        [command] if command == "template-run-v1" => {
            chrono_host_release::template_run::check(std::io::stdin().lock())?;
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
        _ => Err("usage: chrono-host-release ROOT ABSENT_OUTPUT [--unit ID | --collect | --export-frontend | --deliver CONFIG | --install CONFIG NATIVE | --plan-preview CONFIG REQUEST | --select-preview CONFIG COLLECTION]\n       chrono-host-release roadmap-v1 | template-run-v1 | template-acceptance-v1 | template-next-version-v1 | template-release-v1 | template-consumer-pin-v1".into()),
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
