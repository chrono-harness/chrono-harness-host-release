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
            chrono_harness::facts::full_oid(oid)?;
        }
        let search = env
            .get(OsStr::new("PATH"))
            .map(|p| p.to_str().ok_or("non-UTF8 Git search path"))
            .transpose()?;
        let git = chrono_harness::resolve_program(root, "git", search)?;
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
        let mut git = Command::new(&self.git);
        git.current_dir(self.root)
            .env_clear()
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
        chrono_harness::process_fds::forward_command(&mut git, &[])?;
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

    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        chrono_harness::relative_path(path)?;
        // A tree path can name a symlink or submodule. Git formats just its
        // mode here, so path quoting and whitespace cannot change the verdict.
        let mode = self.run(&["ls-tree", "--format=%(objectmode)", &self.tree, "--", path])?;
        if !matches!(mode.as_slice(), b"100644\n" | b"100755\n") {
            return Err(format!(
                "release input is not a regular file in the fixed tree: {path}"
            ));
        }
        self.run(&["cat-file", "blob", &format!("{}:{path}", self.tree)])
    }
}
