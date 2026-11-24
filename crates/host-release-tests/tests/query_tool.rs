//! Respond only to the exact command declared by the test in its temporary cwd.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{Read, Write},
    process::ExitCode,
};
fn main() -> ExitCode {
    let replies: Value = serde_json::from_slice(&fs::read("mock.json").unwrap()).unwrap();
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["observed-command-v1"] {
        let mut request = Vec::new();
        std::io::stdin().read_to_end(&mut request).unwrap();
        let split = request.iter().position(|b| *b == b'\n').unwrap();
        let header: Value = serde_json::from_slice(&request[..split]).unwrap();
        let spec = &header["command"];
        assert_eq!(
            std::path::Path::new(spec["program"].as_str().unwrap())
                .file_name()
                .unwrap(),
            "chrono-host-bootstrap"
        );
        for name in ["GH_TOKEN", "GIT_INDEX_FILE", "UNRELATED"] {
            assert!(spec["env"].get(name).is_none());
        }
        assert_eq!(header["timeout"], json!({"secs":20,"nanos":0}));
        let reply = &replies[serde_json::to_string(&spec["args"]).unwrap()];
        assert_eq!(
            serde_json::from_slice::<Value>(&request[split + 1..]).unwrap(),
            reply["stdin"]
        );
        let stdout = reply["stdout"].as_str().unwrap().as_bytes();
        let stderr = reply["stderr"].as_str().unwrap().as_bytes();
        let hash = |b: &[u8]| format!("{:x}", Sha256::digest(b));
        let mut receipt = json!({"argv":spec["args"],"cwd":env::current_dir().unwrap(),"launcher_pid":std::process::id(),"child_pid":78,"executable":spec["program"],"sha256":header["expected_sha256"],"environment_digest":"env","exit_signal":null,"exit_code":reply["exit"],"failure":null,"stdout_complete":true,"stderr_complete":true,"stdout_size":stdout.len(),"stderr_size":stderr.len(),"stdout_sha256":hash(stdout),"stderr_sha256":hash(stderr)});
        match reply["fault"].as_str() {
            Some("digest") => receipt["stdout_sha256"] = json!("changed"),
            Some("child") => receipt["child_pid"] = json!(0),
            Some("launcher") => receipt["launcher_pid"] = json!(0),
            _ => {}
        }
        let terminal = serde_json::to_vec(&receipt).unwrap();
        let mut out = std::io::stdout().lock();
        for (kind, bytes) in [(b'O', stdout), (b'E', stderr), (b'T', terminal.as_slice())] {
            out.write_all(&[kind]).unwrap();
            out.write_all(&(bytes.len() as u64).to_be_bytes()).unwrap();
            if kind == b'T' && reply["fault"] == "truncated" {
                out.write_all(&bytes[..bytes.len() - 1]).unwrap();
            } else {
                out.write_all(bytes).unwrap();
            }
        }
        if reply["fault"] == "trailing" {
            out.write_all(b"x").unwrap();
        }
        return ExitCode::SUCCESS;
    }
    let args = if args.first().map(String::as_str) == Some("exec-with-descriptors-v1") {
        assert_eq!(
            std::path::Path::new(&args[1]),
            env::current_dir().unwrap().join("git")
        );
        assert!(env::var_os("CHRONO_PROCESS_FDS").is_some());
        &args[2..]
    } else {
        &args[..]
    };
    let key = serde_json::to_string(args).unwrap();
    let reply = replies
        .get(&key)
        .unwrap_or_else(|| panic!("unexpected command {args:?}"));
    if let Some(expected) = reply.get("stdin") {
        let mut bytes = Vec::new();
        std::io::stdin().read_to_end(&mut bytes).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), *expected);
    }
    for key in ["GH_TOKEN", "GIT_INDEX_FILE", "UNRELATED"] {
        assert!(env::var_os(key).is_none(), "unexpected {key}");
    }
    std::io::stdout()
        .write_all(reply["stdout"].as_str().unwrap().as_bytes())
        .unwrap();
    std::io::stderr()
        .write_all(reply["stderr"].as_str().unwrap_or("").as_bytes())
        .unwrap();
    ExitCode::from(reply["exit"].as_u64().unwrap_or(0) as u8)
}
