//! Release stream and executable identities use the original file bytes.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub(crate) open spec fn hex_digit(value: int) -> int {
    if value < 10 { 48 + value } else { 87 + value }
}

fn digit(value: u8) -> (ascii: u8)
    requires value < 16,
    ensures ascii as int == hex_digit(value as int),
{
    if value < 10 { 48u8 + value } else { 97u8 + (value - 10) }
}

/// Encode the 32 digest bytes in wire order, including leading zeroes.
pub(crate) fn digest_ascii(digest: &[u8; 32]) -> (ascii: [u8; 64])
    ensures forall|i: int| 0 <= i < 32 ==>
        #[trigger] ascii@[2*i] as int == hex_digit(digest@[i] as int / 16)
        && ascii@[2*i+1] as int == hex_digit(digest@[i] as int % 16),
{
    let mut ascii = [0u8; 64];
    let mut i: usize = 0;
    while i < 32
        invariant i <= 32,
            forall|j: int| 0 <= j < i ==>
                #[trigger] ascii@[2*j] as int == hex_digit(digest@[j] as int / 16)
                && ascii@[2*j+1] as int == hex_digit(digest@[j] as int % 16),
        decreases 32 - i,
    {
        ascii[2*i] = digit(digest[i] / 16);
        ascii[2*i+1] = digit(digest[i] % 16);
        i += 1;
    }
    ascii
}
}

#[cfg(not(verus_keep_ghost))]
fn digest_text(digest: [u8; 32]) -> String {
    String::from_utf8(digest_ascii(&digest).to_vec()).expect("lowercase hex is ASCII")
}

#[cfg(not(verus_keep_ghost))]
use sha2::{Digest, Sha256};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub fn sha256(data: &[u8]) -> String {
    digest_text(Sha256::digest(data).into())
}
#[cfg(not(verus_keep_ghost))]
pub fn copy_hashed(mut input: impl Read, mut output: impl Write) -> Result<(String, u64), String> {
    let mut buffer = [0; 65536];
    let mut hash = Sha256::new();
    let mut total = 0u64;
    loop {
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        hash.update(&buffer[..n]);
        total = total.checked_add(n as u64).ok_or("stream size overflow")?;
    }
    Ok((digest_text(hash.finalize().into()), total))
}
#[cfg(not(verus_keep_ghost))]
pub fn file_identity(path: &Path) -> Result<(String, u64), String> {
    copy_hashed(
        fs::File::open(path).map_err(|e| e.to_string())?,
        std::io::sink(),
    )
}
#[cfg(not(verus_keep_ghost))]
pub fn resolve_program(
    root: &Path,
    program: &str,
    search: Option<&str>,
) -> Result<PathBuf, String> {
    let root = std::path::absolute(root).map_err(|e| e.to_string())?;
    let error = || format!("executable not found: {program}");
    if program.contains('/') {
        let candidate = root.join(program);
        return candidate.is_file().then_some(candidate).ok_or_else(error);
    }
    let path = search
        .map(std::ffi::OsString::from)
        .or_else(|| std::env::var_os("PATH"))
        .ok_or("PATH is missing")?;
    std::env::split_paths(&path)
        .map(|dir| root.join(dir).join(program))
        .find(|candidate| candidate.is_file())
        .ok_or_else(error)
}
/// Check the runner wire's lexical relative path before using native host filesystem APIs.
#[cfg(not(verus_keep_ghost))]
pub fn relative_path(relative: &str) -> Result<(), String> {
    if relative.is_empty()
        || relative == "."
        || relative.starts_with("./")
        || relative.starts_with('/')
        || relative.split('/').any(|part| part == "..")
        || relative
            .bytes()
            .any(|b| matches!(b, 0 | b'\n' | b'\r' | b'\\'))
    {
        return Err(format!("invalid relative path: {relative:?}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn no_symlink_parents(root: &Path, relative: &str) -> Result<PathBuf, String> {
    relative_path(relative)?;
    let mut path = root.to_owned();
    for part in Path::new(relative).components() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("symlink path is not allowed: {}", path.display()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}

/// Prepare one release output, accepting only physical directories and regular destinations.
#[cfg(not(verus_keep_ghost))]
pub fn prepare_publication(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let target = no_symlink_parents(root, relative)?;
    match fs::symlink_metadata(&target) {
        Ok(m) if !m.is_file() => {
            return Err(format!(
                "publication target is not a file: {}",
                target.display()
            ));
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.to_string()),
        _ => {}
    }
    fs::create_dir_all(target.parent().ok_or("missing publication parent")?)
        .map_err(|e| e.to_string())?;
    no_symlink_parents(root, relative)
}
