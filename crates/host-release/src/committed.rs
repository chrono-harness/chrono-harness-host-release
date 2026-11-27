//! Read release recipes and workflow inputs from the exact previous release tree.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// Git returns exactly one LF-terminated mode for a regular tree entry.
pub fn regular_blob_mode(line: &[u8]) -> (accepted: bool)
    ensures accepted == (line@ == seq![49u8, 48, 48, 54, 52, 52, 10]
                      || line@ == seq![49u8, 48, 48, 55, 53, 53, 10]),
{
    let accepted = line.len() == 7
        && line[0] == 49u8 && line[1] == 48u8 && line[2] == 48u8 && line[6] == 10u8
        && ((line[3] == 54u8 && line[4] == 52u8 && line[5] == 52u8)
         || (line[3] == 55u8 && line[4] == 53u8 && line[5] == 53u8));
    proof {
        assert((line@ == seq![49u8, 48, 48, 54, 52, 52, 10])
            == (line@ =~= seq![49u8, 48, 48, 54, 52, 52, 10]));
        assert((line@ == seq![49u8, 48, 48, 55, 53, 53, 10])
            == (line@ =~= seq![49u8, 48, 48, 55, 53, 53, 10]));
    }
    accepted
}
}

#[cfg(not(verus_keep_ghost))]
use crate::execution::Environment;
#[cfg(not(verus_keep_ghost))]
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(not(verus_keep_ghost))]
pub struct Committed<'a> {
    root: &'a Path,
    env: &'a Environment,
    git: PathBuf,
    tree: String,
}

#[cfg(not(verus_keep_ghost))]
impl<'a> Committed<'a> {
    pub fn new(
        root: &'a Path,
        env: &'a Environment,
        commit: &str,
        tree: &str,
    ) -> Result<Self, String> {
        for oid in [commit, tree] {
            Self::validate_oid(oid)?;
        }
        let search = env
            .get(OsStr::new("PATH"))
            .map(|p| p.to_str().ok_or("non-UTF8 Git search path"))
            .transpose()?;
        let git = crate::files::resolve_program(root, "git", search)?;
        let source = Self {
            root,
            env,
            git,
            tree: tree.into(),
        };
        if source.run(&["rev-parse", "--verify", &format!("{commit}^{{tree}}")])?
            != format!("{tree}\n").as_bytes()
        {
            return Err("immutable producer tree differs".into());
        }
        Ok(source)
    }

    fn run(&self, args: &[&str]) -> Result<Vec<u8>, String> {
        let mut git = if let Some(descriptors) = std::env::var_os(crate::observed::DESCRIPTORS) {
            let search = self.env.get(OsStr::new("PATH")).and_then(|v| v.to_str());
            let owner = crate::files::resolve_program(self.root, "chrono-harness", search)?;
            let mut command = Command::new(owner);
            command.arg("exec-with-descriptors-v1").arg(&self.git);
            command
                .env_clear()
                .env(crate::observed::DESCRIPTORS, descriptors);
            command
        } else {
            let mut command = Command::new(&self.git);
            command.env_clear();
            command
        };
        git.current_dir(self.root)
            .envs(
                ["PATH", "HOME", "XDG_CONFIG_HOME"]
                    .into_iter()
                    .filter_map(|key| self.env.get(OsStr::new(key)).map(|value| (key, value))),
            )
            .args([
                "--no-optional-locks",
                "--no-replace-objects",
                "--literal-pathspecs",
            ])
            .args(args);
        let result = git.output().map_err(|e| e.to_string())?;
        if !result.status.success() {
            return Err(format!(
                "release source Git failed ({}): {}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            ));
        }
        Ok(result.stdout)
    }

    fn validate_oid(oid: &str) -> Result<(), String> {
        if crate::verified_recipe::delivery::valid_release_oid(oid.as_bytes()) {
            Ok(())
        } else {
            Err("expected full nonzero lowercase commit OID".into())
        }
    }
    pub fn read_at(&self, treeish: &str, path: &str) -> Result<Vec<u8>, String> {
        Self::validate_oid(treeish)?;
        crate::files::relative_path(path)?;
        // A tree path can name a symlink or submodule. Git formats just its
        // mode here, so path quoting and whitespace cannot change the verdict.
        let mode = self.run(&["ls-tree", "--format=%(objectmode)", treeish, "--", path])?;
        if !regular_blob_mode(&mode) {
            return Err(format!(
                "release input is not a regular file in the fixed tree: {path}"
            ));
        }
        self.run(&["cat-file", "blob", &format!("{treeish}:{path}")])
    }
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        self.read_at(&self.tree, path)
    }
}
