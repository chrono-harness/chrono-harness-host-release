//! Verify the setup command and installer selected by this host's mise config.
#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
/// A mise setup installs tools first and refreshes the rules only when selected.
pub fn setup_sequence(commands: &[String], instructions: bool) -> (accepted: bool)
    ensures accepted == (commands.len() == (if instructions { 2usize } else { 1usize })
        && commands@[0]@ == "sh .chrono-harness/install-tools.sh"@
        && (!instructions || commands@[1]@ == "mise exec -- chrono-instructions sync-rules --host-root ."@)),
{
    if commands.len() != (if instructions { 2usize } else { 1usize }) { return false; }
    commands[0] == "sh .chrono-harness/install-tools.sh".to_string() && (!instructions
        || commands[1] == "mise exec -- chrono-instructions sync-rules --host-root .".to_string())
}

pub fn mise_manifest() -> (s: &'static str) {
    include_str!("../../../.config/mise/conf.d/chrono-harness.toml")
}

pub fn tool_installer() -> &'static str {
    include_str!("../../../.chrono-harness/install-tools.sh")
}

}

#[cfg(all(test, not(verus_keep_ghost)))]
mod native {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn deployed_installer_matches_the_verified_copier_render() {
        let digest: String = Sha256::digest(tool_installer().as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        // Copier v0.1.54 rendered with this host's template answers.
        assert_eq!(
            digest,
            "666d65b9dff56772927ad3a073106ef4719e775b40e5fcdbb933b1ec6decdb64"
        );
    }

    fn setup_matches(source: &str, instructions: bool) -> bool {
        let Ok(doc) = source.parse::<toml::Table>() else {
            return false;
        };
        let instruction_tool = doc
            .get("tools")
            .and_then(toml::Value::as_table)
            .is_some_and(|tools| tools.contains_key("http:chrono-instructions"));
        let Some(setup) = doc
            .get("tasks")
            .and_then(|tasks| tasks.get("harness-setup"))
            .and_then(toml::Value::as_table)
        else {
            return false;
        };
        if instruction_tool != instructions
            || setup.keys().any(|key| key != "run" && key != "description")
        {
            return false;
        }
        let commands: Vec<String> = match setup.get("run") {
            Some(toml::Value::String(one)) => vec![one.clone()],
            Some(toml::Value::Array(steps)) => {
                let Some(all) = steps
                    .iter()
                    .map(|value| value.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                else {
                    return false;
                };
                all
            }
            _ => return false,
        };
        setup_sequence(&commands, instructions)
    }

    #[test]
    fn mise_installs_the_selected_tools_before_refreshing_instructions() {
        let original = mise_manifest();
        let selected = true;
        assert!(setup_matches(original, selected));
        assert!(!setup_matches(
            &original.replace(
                "sh .chrono-harness/install-tools.sh",
                "sh .chrono-harness/install-tools.sh --dry-run"
            ),
            selected
        ));
        assert!(!setup_matches(
            &original.replace(
                "[tasks.harness-setup]",
                "[tasks.harness-setup]\ndepends = ['other']"
            ),
            selected
        ));
        let install_only =
            "[tools]\n[tasks.harness-setup]\nrun = 'sh .chrono-harness/install-tools.sh'";
        assert!(setup_matches(install_only, false));
        assert!(!setup_matches(install_only, true));
        let reverse = "[tools.'http:chrono-instructions']\n[tasks.harness-setup]\nrun=['mise exec -- chrono-instructions sync-rules --host-root .','sh .chrono-harness/install-tools.sh']";
        assert!(!setup_matches(reverse, true));
        let extra = "[tools]\n[tasks.harness-setup]\nrun = ['sh .chrono-harness/install-tools.sh', 'another-command']";
        assert!(!setup_matches(extra, false));
    }
}
