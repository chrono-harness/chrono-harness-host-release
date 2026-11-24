//! Read release recipes and workflow inputs from the exact previous release tree.
use crate::execution::Environment;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

pub struct Committed<'a> {
    root: &'a Path,
    env: &'a Environment,
    git: PathBuf,
    tree: String,
}

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
        if !matches!(mode.as_slice(), b"100644\n" | b"100755\n") {
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
