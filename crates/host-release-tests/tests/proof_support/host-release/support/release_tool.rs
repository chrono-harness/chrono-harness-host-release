//! Release-build observations and the independent compiler fixture.
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// The release observer reads the deployed compiler input declarations.
fn source_check() -> &'static str { include_str!("../../../../../../check.sh") }

fn registration_in_both_roots(production: bool, independent: bool) -> (covered: bool)
    ensures covered == (production && independent),
{
    production && independent
}
fn compiler_fixture_allowed(distinct: bool, compiler: bool, active: bool) -> (allowed: bool)
    ensures allowed == (!distinct || !compiler || active),
{
    !distinct || !compiler || active
}
}

#[cfg(not(verus_keep_ghost))]
fn main() -> std::process::ExitCode {
    let tokens: Vec<_> = source_check().split_ascii_whitespace().collect();
    if !registration_in_both_roots(
        tokens.contains(&r#""$src/registrations.rs""#),
        tokens.contains(&r#""$included/registrations.rs""#),
    ) {
        eprintln!("compiler source checks must include both registration reply implementations");
        return std::process::ExitCode::FAILURE;
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [exit, path] = args.as_slice() else {
        eprintln!("usage: release_observation EXIT STDERR_FILE");
        return std::process::ExitCode::FAILURE;
    };
    let stderr = match std::fs::read(path) {
        Ok(stderr) => stderr,
        Err(error) => {
            eprintln!("cannot read {path:?}: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if exit.to_str().is_some_and(|exit| {
        chrono_host_release::verified_recipe::release_usage_observed(exit, &stderr)
    }) {
        std::process::ExitCode::SUCCESS
    } else {
        eprintln!("unexpected release probe result, exit {exit:?}");
        std::process::ExitCode::FAILURE
    }
}
