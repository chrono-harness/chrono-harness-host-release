//! Release-side client of the installed process owner.
use crate::{
    codec::decode,
    files::{resolve_program, sha256},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

pub const DESCRIPTORS: &str = "CHRONO_PROCESS_FDS";
/// Upper bound of each original stream in the v1 command protocol.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Serialize)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}

#[derive(Deserialize)]
pub struct Observation {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub launcher_pid: Option<u32>,
    pub child_pid: Option<u32>,
    pub executable: PathBuf,
    pub sha256: String,
    pub environment_digest: String,
    pub exit_signal: Option<i32>,
    pub exit_code: i32,
    pub failure: Option<String>,
    pub stdout_complete: bool,
    pub stderr_complete: bool,
    stdout_size: usize,
    stderr_size: usize,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    #[serde(skip)]
    pub stdout_bytes: Vec<u8>,
    #[serde(skip)]
    pub stderr_bytes: Vec<u8>,
}

pub struct ProcessForward<'a> {
    pub stdout: &'a mut (dyn Write + Send),
    pub stderr: &'a mut (dyn Write + Send),
}

fn receive(
    mut input: impl Read,
    limit: usize,
    mut forward: ProcessForward<'_>,
) -> Result<Observation, String> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    loop {
        let mut header = [0; 9];
        input
            .read_exact(&mut header)
            .map_err(|e| format!("truncated observed command: {e}"))?;
        let size = usize::try_from(u64::from_be_bytes(header[1..].try_into().unwrap()))
            .map_err(|_| "frame length overflow")?;
        let (capture, live) = match header[0] {
            b'O' => (&mut stdout, &mut forward.stdout),
            b'E' => (&mut stderr, &mut forward.stderr),
            b'T' => {
                // The request header is limited to 1 MiB. The receipt adds the runner's observed identities.
                if size > 2 * 1024 * 1024 {
                    return Err("oversize observed terminal record".into());
                }
                let mut bytes = vec![0; size];
                input.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                let mut observed: Observation = decode(&bytes)?;
                let mut trailing = [0u8; 1];
                if input.read(&mut trailing).map_err(|e| e.to_string())? != 0 {
                    return Err("trailing bytes after observed terminal record".into());
                }
                if stdout.len() != observed.stdout_size
                    || stderr.len() != observed.stderr_size
                    || sha256(&stdout) != observed.stdout_sha256
                    || sha256(&stderr) != observed.stderr_sha256
                {
                    return Err("observed terminal stream identity differs".into());
                }
                observed.stdout_bytes = stdout;
                observed.stderr_bytes = stderr;
                return Ok(observed);
            }
            _ => return Err("unknown observed command frame".into()),
        };
        if size > limit.saturating_sub(capture.len()) {
            return Err("observed output exceeds command limit".into());
        }
        let from = capture.len();
        capture.resize(from + size, 0);
        input
            .read_exact(&mut capture[from..])
            .map_err(|e| e.to_string())?;
        live.write_all(&capture[from..])
            .and_then(|()| live.flush())
            .map_err(|e| e.to_string())?;
    }
}

pub fn run_process_observed(
    root: &Path,
    spec: &CommandSpec,
    input: &[u8],
    digest: &str,
) -> Result<Observation, String> {
    run_process_observed_for(
        root,
        spec,
        input,
        digest,
        Duration::from_secs(spec.timeout_seconds),
    )
}
pub fn run_process_observed_for(
    root: &Path,
    spec: &CommandSpec,
    input: &[u8],
    digest: &str,
    timeout: Duration,
) -> Result<Observation, String> {
    run_process_observed_forwarded_for(
        root,
        spec,
        input,
        digest,
        timeout,
        ProcessForward {
            stdout: &mut std::io::sink(),
            stderr: &mut std::io::sink(),
        },
    )
}
pub fn run_process_observed_forwarded_for(
    root: &Path,
    spec: &CommandSpec,
    input: &[u8],
    digest: &str,
    timeout: Duration,
    forward: ProcessForward<'_>,
) -> Result<Observation, String> {
    observe_with_search(root, spec, input, digest, timeout, None, forward)
}

/// A caller with an explicit host tool environment can select the installed owner independently
/// of PATH supplied to the business child.
pub(crate) fn observe_with_search(
    root: &Path,
    spec: &CommandSpec,
    input: &[u8],
    digest: &str,
    timeout: Duration,
    search: Option<&str>,
    forward: ProcessForward<'_>,
) -> Result<Observation, String> {
    let runner = resolve_program(root, "chrono-harness", search)?;
    let mut header = serde_json::to_vec(
        &serde_json::json!({"command":spec,"expected_sha256":digest,"timeout":timeout}),
    )
    .map_err(|e| e.to_string())?;
    header.push(b'\n');
    let mut child = Command::new(runner)
        .arg("observed-command-v1")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let launcher = child.id();
    let mut sender = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let mut diagnostic = child.stderr.take().unwrap();
    let (received, sent, diagnostics) = std::thread::scope(|scope| {
        let write = scope.spawn(move || {
            sender
                .write_all(&header)
                .and_then(|()| sender.write_all(input))
        });
        let read = scope.spawn(move || {
            let mut bytes = vec![];
            diagnostic.read_to_end(&mut bytes).map(|_| bytes)
        });
        let received = receive(output, spec.output_limit_bytes, forward);
        (received, write.join(), read.join())
    });
    let status = child.wait().map_err(|e| e.to_string())?;
    let diagnostics = diagnostics
        .map_err(|_| "observed diagnostic reader panicked")?
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "observed-command-v1 {status}: {}",
            String::from_utf8_lossy(&diagnostics).trim_end()
        ));
    }
    sent.map_err(|_| "observed input writer panicked")?
        .map_err(|e| e.to_string())?;
    let observed = received?;
    if observed.launcher_pid != Some(launcher)
        || observed.child_pid.is_none_or(|pid| pid == 0)
        || observed.sha256 != digest
    {
        return Err("observed process identity differs".into());
    }
    Ok(observed)
}
