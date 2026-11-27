//! 发布单元的实时原流。沿用现有描述符转交；辅助 IO 失败后仍排空两流、加入原进程。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// Negative receipt exit codes encode signals; absent or invalid owner status is unknown.
pub fn child_status(signal: Option<i32>, code: i32) -> (status: Option<i32>)
    ensures status == match signal {
        Some(s) => if 1 <= s <= 127 { Some((-s) as i32) } else { None },
        None => if 0 <= code <= 255 { Some(code) } else { None },
    }, status is Some ==> -127 <= status.unwrap() <= 255,
{
    match signal {
        Some(s) if 1 <= s && s <= 127 => Some(-s),
        None if 0 <= code && code <= 255 => Some(code),
        _ => None,
    }
}

pub fn stream_failed(complete: bool, failures: usize) -> (failed: bool)
    ensures failed == (!complete || failures > 0),
{ !complete || failures > 0 }

}

#[cfg(not(verus_keep_ghost))]
use crate::verified_recipe as policy;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Debug)]
pub struct CapturedStream {
    pub bytes: Vec<u8>,
    pub path: Option<String>,
    pub failures: Vec<String>,
    pub complete: bool,
}

#[cfg(not(verus_keep_ghost))]
impl CapturedStream {
    /// 路径只引用实际写成的文件；保留失败时继续提供已观察的原字节。
    pub fn receipt(&self) -> Value {
        let mut value = json!({
            "size": self.bytes.len(),
            "sha256": crate::files::sha256(&self.bytes),
        });
        if let Some(path) = &self.path {
            value["path"] = json!(path);
        } else {
            value["status"] = json!("unavailable");
            value["error"] = json!(self.failures.join("; "));
            value["bytes"] = json!(self.bytes);
        }
        if !self.complete {
            value["status"] = json!("incomplete");
        }
        value
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Debug)]
pub struct CapturedProcess {
    pub pid: u32,
    pub argv: Vec<String>,
    pub cwd: String,
    pub started_ns: u128,
    pub finished_ns: Option<u128>,
    pub duration_seconds: f64,
    /// 与原配方一致：正常退出为 0..255，信号为负数；未知终态不填写成功。
    pub child_exit: Option<i32>,
    pub stdout: CapturedStream,
    pub stderr: CapturedStream,
    pub failures: Vec<String>,
    pub observation: Value,
}

#[cfg(not(verus_keep_ghost))]
impl CapturedProcess {
    pub fn auxiliary_failed(&self) -> bool {
        !self.failures.is_empty()
            || stream_failed(self.stdout.complete, self.stdout.failures.len())
            || stream_failed(self.stderr.complete, self.stderr.failures.len())
    }

    pub fn exit_code(&self) -> i32 {
        policy::result_exit(self.child_exit, self.auxiliary_failed())
    }

    pub fn passed(&self) -> bool {
        policy::passed(self.child_exit, self.auxiliary_failed())
    }

    pub fn receipt(&self, phase: &str, operation: Option<&str>) -> Value {
        json!({
            "phase": phase, "operation": operation,
            "argv": self.argv, "cwd": self.cwd, "pid": self.pid,
            "started_ns": self.started_ns, "finished_ns": self.finished_ns,
            "exit_code": self.child_exit,
            "stdout": self.stdout.receipt(), "stderr": self.stderr.receipt(),
            "observation": self.observation,
        })
    }
}

#[cfg(not(verus_keep_ghost))]
fn timestamp() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
struct StreamWriter<W> {
    file: Option<std::fs::File>,
    relative: String,
    forward: Option<W>,
    failures: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
impl<W: Write> StreamWriter<W> {
    fn new(output: &Path, relative: String, forward: W) -> Self {
        let mut failures = Vec::new();
        let file = match fs::create_dir_all(output.join("processes")).and_then(|()| {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output.join(&relative))
        }) {
            Ok(file) => Some(file),
            Err(error) => {
                failures.push(format!("retention: {error}"));
                None
            }
        };
        Self {
            file,
            relative,
            forward: Some(forward),
            failures,
        }
    }

    fn finish(mut self, bytes: Vec<u8>, complete: bool) -> CapturedStream {
        if let Some(file) = self.file.as_mut() {
            if let Err(error) = file.flush() {
                self.failures.push(format!("retention flush: {error}"));
                self.file = None;
            }
        }
        CapturedStream {
            bytes,
            path: (self.file.is_some() && complete).then_some(self.relative),
            failures: self.failures,
            complete,
        }
    }
}

#[cfg(not(verus_keep_ghost))]
impl<W: Write> Write for StreamWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(file) = self.file.as_mut() {
            if let Err(error) = file.write_all(bytes) {
                self.failures.push(format!("retention: {error}"));
                self.file = None;
            }
        }
        if let Some(forward) = self.forward.as_mut() {
            if let Err(error) = forward.write_all(bytes).and_then(|()| forward.flush()) {
                self.failures.push(format!("forwarding: {error}"));
                self.forward = None;
            }
        }
        // Auxiliary errors must not interrupt either original reader or replace child failure.
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(not(verus_keep_ghost))]
/// Reuse the runner's bounded process owner, deadline, inherited capabilities and stream readers.
/// Output paths remain relative to the release output, which may lie outside the source clone.
pub fn execute(
    root: &Path,
    spec: &crate::observed::CommandSpec,
    output: &Path,
    index: usize,
    timeout: std::time::Duration,
    stdout: impl Write + Send,
    stderr: impl Write + Send,
) -> Result<CapturedProcess, String> {
    let digest = crate::files::file_identity(Path::new(&spec.program))?.0;
    let mut stdout = StreamWriter::new(output, format!("processes/{index}.stdout"), stdout);
    let mut stderr = StreamWriter::new(output, format!("processes/{index}.stderr"), stderr);
    let started_ns = timestamp()?;
    let started = Instant::now();
    let original = crate::observed::run_process_observed_forwarded_for(
        root,
        spec,
        b"",
        &digest,
        timeout,
        crate::observed::ProcessForward {
            stdout: &mut stdout,
            stderr: &mut stderr,
        },
    )?;
    let child_exit = child_status(original.exit_signal, original.exit_code);
    let observation = json!({"launcher_pid":original.launcher_pid,"executable":original.executable,"sha256":original.sha256,"environment_digest":original.environment_digest,"exit_signal":original.exit_signal,"engine_exit_code":original.exit_code,"failure":original.failure,"timeout_ns":timeout.as_nanos(),"output_limit_bytes":spec.output_limit_bytes});
    let mut failures: Vec<String> = original.failure.into_iter().collect();
    if child_exit.is_none() {
        failures.push("unavailable terminal child status".into());
    }
    let finished_ns = match timestamp() {
        Ok(time) => Some(time),
        Err(error) => {
            failures.push(format!("finish timestamp: {error}"));
            None
        }
    };
    Ok(CapturedProcess {
        pid: original
            .child_pid
            .ok_or("missing original child identity")?,
        argv: original.argv,
        cwd: original
            .cwd
            .to_str()
            .ok_or("non UTF-8 release cwd")?
            .to_owned(),
        started_ns,
        finished_ns,
        duration_seconds: started.elapsed().as_secs_f64(),
        child_exit,
        stdout: stdout.finish(original.stdout_bytes, original.stdout_complete),
        stderr: stderr.finish(original.stderr_bytes, original.stderr_complete),
        failures,
        observation,
    })
}
