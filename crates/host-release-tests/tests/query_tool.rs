//! Respond only to the exact command declared by the test in its temporary cwd.
use serde_json::Value;
use std::{
    env, fs,
    io::{Read, Write},
    process::ExitCode,
};
fn main() -> ExitCode {
    let replies: Value = serde_json::from_slice(&fs::read("mock.json").unwrap()).unwrap();
    let args: Vec<_> = env::args().skip(1).collect();
    let key = serde_json::to_string(&args).unwrap();
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
