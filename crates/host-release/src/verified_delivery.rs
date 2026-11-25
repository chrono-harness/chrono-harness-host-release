//! 发布入口的实际字段判断。适配器只供应外部解码的原值，不供应预先算好的通过标志。
use super::same_text;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

pub open spec fn text_pair(pair: (Option<&str>, &str)) -> bool {
    match pair.0 { Some(actual) => actual@ == pair.1@, None => false }
}

/// 每个输入保留缺失／错型观察；缺字段不能与空文本混同。
pub fn text_fields(fields: &[(Option<&str>, &str)]) -> (valid: bool)
    ensures valid <==> forall|i: int| 0 <= i < fields.len() ==> text_pair(fields@[i]),
{
    let mut i = 0;
    while i < fields.len()
        invariant i <= fields.len(),
            forall|j: int| 0 <= j < i ==> text_pair(fields@[j]),
        decreases fields.len() - i,
    {
        match fields[i].0 {
            Some(actual) => {
                if !same_text(actual, fields[i].1) { return false; }
            },
            None => { return false; },
        }
        i += 1;
    }
    true
}

/// The identity and outcome of the GitHub run selected to publish a commit.
pub fn landed_push(fields: &[Option<&str>; 8], repository: &str, workflow_path: &str, branch: &str, sha: &str) -> (valid: bool)
    ensures valid == (
        text_pair((fields@[0], repository)) && text_pair((fields@[1], repository))
        && text_pair((fields@[2], workflow_path)) && text_pair((fields@[3], branch))
        && text_pair((fields@[4], sha)) && text_pair((fields@[5], "push"))
        && text_pair((fields@[6], "completed")) && text_pair((fields@[7], "success"))),
{
    let pairs = [(fields[0], repository), (fields[1], repository), (fields[2], workflow_path),
        (fields[3], branch), (fields[4], sha), (fields[5], "push"),
        (fields[6], "completed"), (fields[7], "success")];
    let valid = text_fields(&pairs);
    assert(valid == (text_pair(pairs@[0]) && text_pair(pairs@[1]) && text_pair(pairs@[2])
        && text_pair(pairs@[3]) && text_pair(pairs@[4]) && text_pair(pairs@[5])
        && text_pair(pairs@[6]) && text_pair(pairs@[7]))) by {
        if !valid {
            let i = choose|i:int| 0 <= i < pairs.len() && !text_pair(pairs@[i]);
            assert(i == 0 || i == 1 || i == 2 || i == 3 || i == 4 || i == 5 || i == 6 || i == 7);
        }
    }
    valid
}

/// GitHub supplies jobs and release assets in provider order.
pub fn last_named(names: &[Option<&str>], limit: usize, expected: &str) -> (found: Option<usize>)
    requires limit <= names.len(),
    ensures match found {
        Some(i) => i < limit && text_pair((names@[i as int], expected))
            && forall|j: int| i < j < limit ==> !text_pair((#[trigger] names@[j], expected)),
        None => forall|j: int| 0 <= j < limit ==> !text_pair((#[trigger] names@[j], expected)),
    },
{
    let mut end = limit;
    while end > 0
        invariant end <= limit,
            forall|j: int| end <= j < limit ==> !text_pair((#[trigger] names@[j], expected)),
        decreases end,
    {
        end -= 1;
        if text_fields(&[(names[end], expected)]) { return Some(end); }
    }
    None
}

pub enum NamedAsset { Missing, Duplicate, Found(usize) }

/// The selected index must identify the only asset with the requested name.
pub fn installation_asset(names: &[Option<&str>], requested: &str) -> (result: NamedAsset)
    ensures match result {
        NamedAsset::Missing => forall|i: int| 0 <= i < names.len() ==> !text_pair((#[trigger] names@[i], requested)),
        NamedAsset::Duplicate => exists|i: int, j: int| 0 <= i < j < names.len()
            && text_pair((names@[i], requested)) && text_pair((names@[j], requested)),
        NamedAsset::Found(i) => i < names.len() && text_pair((names@[i as int], requested))
            && forall|j: int| 0 <= j < names.len() && j != i ==> !text_pair((#[trigger] names@[j], requested)),
    },
{
    let Some(i) = last_named(names, names.len(), requested) else { return NamedAsset::Missing; };
    if let Some(earlier) = last_named(names, i, requested) {
        assert(text_pair((names@[earlier as int], requested)));
        NamedAsset::Duplicate
    } else {
        NamedAsset::Found(i)
    }
}

pub open spec fn exercised_step(steps: Seq<(Option<&str>, Option<&str>)>, step: &str) -> bool {
    exists|i: int| 0 <= i < steps.len() && text_pair((steps[i].0, step)) && text_pair((steps[i].1, "success"))
}

pub fn acceptance_job(name: Option<&str>, expected: &str, conclusion: Option<&str>, head: Option<&str>, sha: &str,
    steps: &[(Option<&str>, Option<&str>)], step: &str) -> (valid: bool)
    ensures valid == (text_pair((name, expected)) && text_pair((conclusion, "success"))
        && text_pair((head, sha)) && exercised_step(steps@, step)),
{
    if !text_fields(&[(name, expected), (conclusion, "success"), (head, sha)]) { return false; }
    let mut i = 0;
    while i < steps.len()
        invariant text_pair((name, expected)), text_pair((conclusion, "success")), text_pair((head, sha)),
            i <= steps.len(), forall|j: int| 0 <= j < i ==>
            !(text_pair((steps@[j].0, step)) && text_pair((steps@[j].1, "success"))),
        decreases steps.len()-i,
    {
        if text_fields(&[(steps[i].0, step),(steps[i].1,"success")]) {
            assert(exercised_step(steps@, step)) by { assert(0 <= i < steps.len()); }
            return true;
        }
        i += 1;
    }
    false
}

pub fn acceptance_declared(names: usize, step: &str) -> (valid: bool)
    ensures valid == (names > 0 && step@ != ""@),
{ names > 0 && !same_text(step, "") }

pub fn number_fields(fields: &[(Option<u64>, u64)]) -> (valid: bool)
    ensures valid <==> forall|i: int| 0 <= i < fields.len() ==> fields@[i].0 == Some(fields@[i].1),
{
    let mut i = 0;
    while i < fields.len()
        invariant i <= fields.len(),
            forall|j: int| 0 <= j < i ==> fields@[j].0 == Some(fields@[j].1),
        decreases fields.len() - i,
    {
        if fields[i].0 != Some(fields[i].1) { return false; }
        i += 1;
    }
    true
}

pub fn boolean_fields(fields: &[(Option<bool>, bool)]) -> (valid: bool)
    ensures valid <==> forall|i: int| 0 <= i < fields.len() ==> fields@[i].0 == Some(fields@[i].1),
{
    let mut i = 0;
    while i < fields.len()
        invariant i <= fields.len(),
            forall|j: int| 0 <= j < i ==> fields@[j].0 == Some(fields@[j].1),
        decreases fields.len() - i,
    {
        if fields[i].0 != Some(fields[i].1) { return false; }
        i += 1;
    }
    true
}

pub fn single_original(count: Option<u64>, rows: usize) -> (valid: bool)
    ensures valid == (count == Some(1u64) && rows == 1),
{
    count == Some(1) && rows == 1
}

/// API 原字段直接输入；缺字段、过期或选中另一不可变工件均失败。
pub fn promotion_artifact(id: Option<u64>, selected: Option<u64>, expired: Option<bool>, size: Option<u64>,
    numbers: &[(Option<u64>, u64)], texts: &[(Option<&str>, &str)]) -> (valid: bool)
    ensures valid == (positive_spec(id) && positive_spec(size) && expired == Some(false)
        && (selected is None || id == selected)
        && (forall|i: int| 0 <= i < numbers.len() ==> numbers@[i].0 == Some(numbers@[i].1))
        && (forall|i: int| 0 <= i < texts.len() ==> text_pair(texts@[i]))),
{
    positive(id) && positive(size) && expired == Some(false)
        && (selected.is_none() || id == selected) && number_fields(numbers) && text_fields(texts)
}

/// An upload from an earlier attempt of this release run can be reused.
pub fn producing_attempt(run: u64, selected: u64, attempt: u64, current: u64) -> (valid: bool)
    ensures valid == (0 < run && selected == run && 0 < attempt <= current),
{ run > 0 && selected == run && attempt > 0 && attempt <= current }

pub open spec fn positive_spec(value: Option<u64>) -> bool {
    value is Some && value.unwrap() > 0
}

pub open spec fn lower_hex(bytes: Seq<u8>) -> bool {
    forall|i: int| 0 <= i < bytes.len() ==>
        48 <= #[trigger] bytes[i] <= 57 || 97 <= bytes[i] <= 102
}

pub open spec fn release_oid(bytes: Seq<u8>) -> bool {
    (bytes.len() == 40 || bytes.len() == 64) && lower_hex(bytes)
        && (exists|i: int| 0 <= i < bytes.len() && bytes[i] != 48)
}

/// Git 的空对象名由全零表示；发布摘要允许全零。
fn hex_content(bytes: &[u8]) -> (nonzero: Option<bool>)
    ensures
        nonzero.is_some() == lower_hex(bytes@),
        nonzero.is_some() ==> nonzero.unwrap() == (exists|i: int| 0 <= i < bytes.len() && bytes[i] != 48),
{
    let mut seen_nonzero = false;
    for i in 0..bytes.len()
        invariant
            seen_nonzero == (exists|j: int| 0 <= j < i && bytes[j] != 48),
            forall|j: int| 0 <= j < i ==> 48 <= #[trigger] bytes[j] <= 57 || 97 <= bytes[j] <= 102,
    {
        let byte = bytes[i];
        if !(48 <= byte && byte <= 57 || 97 <= byte && byte <= 102) { return None; }
        if byte != 48 { seen_nonzero = true; }
    }
    Some(seen_nonzero)
}

pub fn valid_release_oid(bytes: &[u8]) -> (valid: bool)
    ensures valid == release_oid(bytes@),
{
    (bytes.len() == 40 || bytes.len() == 64) && matches!(hex_content(bytes), Some(true))
}

pub fn valid_digest(text: &str) -> (valid: bool)
    ensures valid == (text.spec_bytes().len() == 64 && lower_hex(text.spec_bytes())),
{
    let bytes = text.as_bytes();
    bytes.len() == 64 && hex_content(bytes).is_some()
}

pub type ExpectedAsset<'a> = (&'a str, Option<&'a str>);
pub type ObservedAsset<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);

pub open spec fn matching_asset(expected: ExpectedAsset, actual: ObservedAsset) -> bool {
    text_pair((actual.0, expected.0)) && text_pair((actual.1, "uploaded"))
    && match actual.2 {
        None => false,
        Some(digest) => digest.spec_bytes().len() == 64 && lower_hex(digest.spec_bytes())
            && match expected.1 {
                Some(pin) => digest@ == pin@,
                None => expected.0@ == "SHA256SUMS"@,
            },
    }
}

/// A missing consumer pin is meaningful only for the checksums file itself.
pub fn uploaded_asset(expected: ExpectedAsset, actual: ObservedAsset) -> (valid: bool)
    ensures valid == matching_asset(expected, actual),
{
    if !text_fields(&[(actual.0, expected.0), (actual.1, "uploaded")]) { return false; }
    match actual.2 {
        None => false,
        Some(digest) => valid_digest(digest) && match expected.1 {
            Some(pin) => same_text(digest, pin),
            None => same_text(expected.0, "SHA256SUMS"),
        },
    }
}

/// The publisher owns a BTreeMap keyed by asset name. A successful take
/// returns the original pin and consumes exactly one key.
pub trait PendingAssets {
    spec fn pins(&self) -> Map<Seq<char>, Seq<char>>;

    fn take(&mut self, name: &str) -> (pin: Option<String>)
        ensures
            self.pins() == old(self).pins().remove(name@),
            match pin {
                Some(value) => old(self).pins().contains_key(name@) && value@ == old(self).pins()[name@],
                None => !old(self).pins().contains_key(name@),
            };
}

pub open spec fn pinned_row(pins: Map<Seq<char>,Seq<char>>, row: ObservedAsset) -> bool {
    match (row.0,row.2) {
        (Some(name),Some(digest)) => pins.contains_key(name@) && digest@ == pins[name@]
            && text_pair((row.1,"uploaded")) && digest.spec_bytes().len() == 64 && lower_hex(digest.spec_bytes()),
        _ => false,
    }
}

pub open spec fn remaining_after(pins: Map<Seq<char>,Seq<char>>, rows: Seq<ObservedAsset>, n: int)
    -> Map<Seq<char>,Seq<char>>
    decreases n,
{
    if n <= 0 || n > rows.len() { pins } else {
        let previous = remaining_after(pins,rows,n-1);
        match rows[n-1].0 { Some(name) => previous.remove(name@), None => previous }
    }
}

pub enum PublicationAssetError {
    MissingName,
    UnexpectedName,
    DifferentBytes,
}

pub fn consume_assets<A: PendingAssets>(pending: &mut A, rows: &[ObservedAsset])
    -> (result: Result<(),(usize,PublicationAssetError)>)
    ensures
        match result {
            Ok(()) => pending.pins() == remaining_after(old(pending).pins(),rows@,rows.len() as int)
                && forall|j:int| 0 <= j < rows.len() ==>
                    pinned_row(remaining_after(old(pending).pins(),rows@,j),#[trigger] rows@[j]),
            Err((i,cause)) => i < rows.len()
                && (forall|j:int| 0 <= j < i ==>
                    pinned_row(remaining_after(old(pending).pins(),rows@,j),#[trigger] rows@[j]))
                && match cause {
                    PublicationAssetError::MissingName => rows@[i as int].0 is None,
                    PublicationAssetError::UnexpectedName => rows@[i as int].0 is Some
                        && !remaining_after(old(pending).pins(),rows@,i as int).contains_key(rows@[i as int].0.unwrap()@),
                    PublicationAssetError::DifferentBytes => rows@[i as int].0 is Some
                        && remaining_after(old(pending).pins(),rows@,i as int).contains_key(rows@[i as int].0.unwrap()@)
                        && !pinned_row(remaining_after(old(pending).pins(),rows@,i as int),rows@[i as int]),
                },
        },
{
    let mut i=0;
    while i < rows.len()
        invariant
            i <= rows.len(),
            pending.pins() == remaining_after(old(pending).pins(),rows@,i as int),
            forall|j:int| 0 <= j < i ==> pinned_row(remaining_after(old(pending).pins(),rows@,j),#[trigger] rows@[j]),
        decreases rows.len()-i,
    {
        let row=rows[i];
        let name=match row.0 {
            Some(name)=>name, None=>return Err((i,PublicationAssetError::MissingName)),
        };
        let pin=match pending.take(name) {
            Some(value)=>value, None=>return Err((i,PublicationAssetError::UnexpectedName)),
        };
        if !uploaded_asset((name,Some(pin.as_str())),row) {
            return Err((i,PublicationAssetError::DifferentBytes));
        }
        i+=1;
    }
    Ok(())
}

/// The decoder sorts both lists by asset name and preserves every GitHub row.
pub fn release_assets(expected: &[ExpectedAsset], actual: &[ObservedAsset]) -> (valid: bool)
    ensures valid == (expected.len() == actual.len() && forall|i: int| 0 <= i < expected.len()
        ==> matching_asset(expected@[i], actual@[i])),
{
    if expected.len() != actual.len() { return false; }
    for i in 0..expected.len()
        invariant expected.len() == actual.len(),
            forall|j: int| 0 <= j < i ==> matching_asset(expected@[j], actual@[j]),
    {
        if !uploaded_asset(expected[i], actual[i]) { return false; }
    }
    true
}

/// 只接受显式原运行支持的两个触发事件与完整工作流 head。
pub fn promotion_run(numbers: &[(Option<u64>, u64)], texts: &[(Option<&str>, &str)], event: Option<&str>, head: Option<&[u8]>) -> (valid: bool)
    ensures valid == (
        (forall|i: int| 0 <= i < numbers.len() ==> numbers@[i].0 == Some(numbers@[i].1))
        && (forall|i: int| 0 <= i < texts.len() ==> text_pair(texts@[i]))
        && (text_pair((event,"workflow_run")) || text_pair((event,"workflow_dispatch")))
        && (match head { Some(value) => release_oid(value@), None => false })),
{
    number_fields(numbers) && text_fields(texts)
        && (text_fields(&[(event,"workflow_run")]) || text_fields(&[(event,"workflow_dispatch")]))
        && match head { Some(value) => valid_release_oid(value), None => false }
}

/// 保留原生产时间窗；正式晋升不会重新计算或刷新原平台起点。
pub fn original_spans(start: u64, finish: u64, platform: Option<u64>, span: Option<u64>, unit_seconds: u64, platform_seconds: u64) -> (valid: bool)
    ensures valid == (match platform {
        Some(p) => p > 0 && p <= start && start <= finish
            && finish as int - start as int <= unit_seconds as int * 1000000000
            && span == Some((finish - p) as u64)
            && finish as int - p as int <= platform_seconds as int * 1000000000,
        None => false,
    }),
{
    match platform {
        Some(p) => p > 0 && p <= start && start <= finish
            && (finish - start) as u128 <= unit_seconds as u128 * 1000000000u128
            && span == Some(finish - p)
            && (finish - p) as u128 <= platform_seconds as u128 * 1000000000u128,
        None => false,
    }
}

/// 0=成功，1=允许缺失的原生 404，2=失败。失败不能因 absent 标志变成成功。
pub fn api_outcome(exit: i32, status: Option<&str>, message: Option<&str>, absent: bool) -> (out: u8)
    ensures out == if exit == 0 { 0u8 }
        else if exit == 1 && absent && text_pair((status,"404")) && text_pair((message,"Not Found")) { 1u8 }
        else { 2u8 },
{
    if exit == 0 { 0 }
    else if exit == 1 && absent && text_fields(&[(status,"404"),(message,"Not Found")]) { 1 }
    else { 2 }
}

pub open spec fn decimal(bytes: Seq<u8>, zero: bool) -> bool {
    bytes.len() > 0
        && (bytes[0] != 48 || (zero && bytes.len() == 1))
        && forall|i: int| 0 <= i < bytes.len() ==> 48 <= #[trigger] bytes[i] <= 57
}

pub fn decimal_text(bytes: &[u8], zero: bool) -> (valid: bool)
    ensures valid == decimal(bytes@, zero),
{
    if bytes.len() == 0 || (bytes[0] == 48 && (!zero || bytes.len() != 1)) { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant i <= bytes.len(), bytes.len() > 0,
            bytes[0] != 48 || (zero && bytes.len() == 1),
            forall|j: int| 0 <= j < i ==> 48 <= #[trigger] bytes[j] <= 57,
        decreases bytes.len() - i,
    {
        if bytes[i] < 48 || bytes[i] > 57 { return false; }
        i += 1;
    }
    true
}

/// 标准 split 只解码原分量；三段和规范十进制的要求由这里拥有。
pub fn version_numbers(parts: &[&str]) -> (valid: bool)
    ensures valid == (parts.len() == 3
        && forall|i: int| 0 <= i < parts.len() ==> decimal(parts@[i].spec_bytes(), true)),
{
    if parts.len() != 3 { return false; }
    let mut i = 0;
    while i < parts.len()
        invariant parts.len() == 3, i <= parts.len(),
            forall|j: int| 0 <= j < i ==> decimal(parts@[j].spec_bytes(), true),
        decreases parts.len() - i,
    {
        if !decimal_text(parts[i].as_bytes(), true) { return false; }
        i += 1;
    }
    true
}

/// The caller decodes an optional "-alpha." suffix; release serials are positive canonical decimals.
pub fn consumer_series(rolling: bool, core: &str, parts: &[&str], planned: &str, serial: Option<&str>) -> (valid: bool)
    ensures valid == ((parts.len() == 3 && forall|i: int| 0 <= i < parts.len() ==> decimal(parts@[i].spec_bytes(), true))
        && (rolling || core@ == planned@)
        && match serial { Some(value) => decimal(value.spec_bytes(), false), None => rolling }),
{
    version_numbers(parts) && (rolling || same_text(core, planned))
        && match serial { Some(value) => decimal_text(value.as_bytes(), false), None => rolling }
}

pub fn installed_download(url: Option<&str>, expected: &str, sha256: Option<&str>) -> (valid: bool)
    ensures valid == (text_pair((url, expected)) && match sha256 {
        None => false,
        Some(value) => value.spec_bytes().len() == 64 && lower_hex(value.spec_bytes()),
    }),
{
    text_fields(&[(url, expected)]) && match sha256 {
        Some(value) => valid_digest(value), None => false,
    }
}

pub fn uploaded_download(state: Option<&str>, url: Option<&str>, expected: &str, sha256: Option<&str>) -> (valid: bool)
    ensures valid == (text_pair((state, "uploaded")) && text_pair((url, expected)) && match sha256 {
        None => false,
        Some(value) => value.spec_bytes().len() == 64 && lower_hex(value.spec_bytes()),
    }),
{
    text_fields(&[(state, "uploaded")]) && installed_download(url, expected, sha256)
}

/// channel 的数字仅为适配器对既有枚举的无状态投影；3 表示拒绝。
pub fn channel(text: &str) -> (out: u8)
    ensures out == if text@ == ""@ || text@ == "build"@ { 0u8 }
        else if text@ == "preview"@ { 1u8 }
        else if text@ == "stable"@ { 2u8 }
        else { 3u8 },
{
    if same_text(text, "") || same_text(text, "build") { 0 }
    else if same_text(text, "preview") { 1 }
    else if same_text(text, "stable") { 2 }
    else { 3 }
}

pub fn requested_version(channel: u8, requested: &str, core: &str, planned: &str) -> (valid: bool)
    ensures valid == (if channel == 1 { requested@.len() == 0 }
        else if channel == 2 { core@ == planned@ && requested@ == planned@ }
        else { false }),
{
    if channel == 1 { requested.is_empty() }
    else if channel == 2 {
        same_text(core, planned) && same_text(requested, planned)
    } else { false }
}

pub open spec fn name_byte(byte: u8) -> bool {
    (65 <= byte <= 90) || (97 <= byte <= 122) || (48 <= byte <= 57)
        || byte == 46 || byte == 95 || byte == 45
}

pub fn job_id(bytes: &[u8]) -> (valid: bool)
    ensures valid == (bytes.len() > 0
        && forall|i: int| 0 <= i < bytes.len() ==> name_byte(#[trigger] bytes[i])),
{
    if bytes.is_empty() { return false; }
    for i in 0..bytes.len()
        invariant forall|j: int| 0 <= j < i ==> name_byte(#[trigger] bytes[j]),
    {
        let b = bytes[i];
        if !((65 <= b && b <= 90) || (97 <= b && b <= 122) || (48 <= b && b <= 57)
            || b == 46 || b == 95 || b == 45) { return false; }
    }
    true
}

pub fn name(bytes: &[u8]) -> (valid: bool)
    ensures valid == (bytes.len() > 0 && bytes@ != seq![46u8] && bytes@ != seq![46u8,46]
        && forall|i: int| 0 <= i < bytes.len() ==> name_byte(#[trigger] bytes[i])),
{
    if !job_id(bytes) { return false; }
    if bytes[0] == 46 && (bytes.len() == 1 || (bytes.len() == 2 && bytes[1] == 46)) {
        proof {
            if bytes.len() == 1 { assert(bytes@ =~= seq![46u8]); }
            else { assert(bytes@ =~= seq![46u8,46]); }
        }
        return false;
    }
    true
}

pub fn positive(value: Option<u64>) -> (valid: bool)
    ensures valid == (value is Some && value.unwrap() > 0),
{
    match value { Some(value) => value > 0, None => false }
}

/// 0 = absent, 1 = same direct commit, 2 = conflicting or malformed reference.
pub fn selected_ref(present: bool, reference: Option<&str>, expected_ref: &str,
    kind: Option<&str>, oid: Option<&str>, source: &str) -> (status: u8)
    ensures status == (if !release_oid(source.spec_bytes()) { 2u8 }
        else if !present { 0u8 }
        else if text_pair((reference, expected_ref)) && text_pair((kind, "commit")) && text_pair((oid, source)) { 1u8 }
        else { 2u8 }),
{
    if !valid_release_oid(source.as_bytes()) { return 2; }
    if !present { return 0; }
    if text_fields(&[(reference, expected_ref), (kind, "commit"), (oid, source)]) { 1 } else { 2 }
}

pub enum VersionPolicy { Explicit, NextPatch, Unknown, WrongProduct }

/// Only an omitted policy defaults to the version declared in the release configuration.
pub fn version_policy(product: Option<&str>, present: bool, policy: Option<&str>) -> (out: VersionPolicy)
    ensures
        (out is Explicit) == (!present || text_pair((policy, "explicit"))),
        (out is NextPatch) == (present && text_pair((policy, "next_patch")) && text_pair((product, "copier"))),
        (out is WrongProduct) == (present && text_pair((policy, "next_patch")) && !text_pair((product, "copier"))),
        (out is Unknown) == (present && !text_pair((policy, "explicit")) && !text_pair((policy, "next_patch"))),
{
    proof {
        reveal_strlit("explicit"); reveal_strlit("next_patch");
        assert("explicit"@.len() != "next_patch"@.len());
    }
    if !present || text_fields(&[(policy, "explicit")]) { return VersionPolicy::Explicit; }
    if !text_fields(&[(policy, "next_patch")]) { return VersionPolicy::Unknown; }
    if text_fields(&[(product, "copier")]) { VersionPolicy::NextPatch } else { VersionPolicy::WrongProduct }
}

/// Decide which source can drive a release before reading its configuration.
pub enum ReleaseEvent { Dev, Main, Dispatch, WrongRepository, WrongBranch, WrongEvent }

pub fn release_event(repository: &str, actual: Option<&str>, event: &str,
    branch: Option<&str>, git_ref: Option<&str>) -> (out: ReleaseEvent)
    ensures
        (out is WrongRepository) == !text_pair((actual,repository)),
        (out is Dev) == (text_pair((actual,repository)) && event@ == "workflow_run"@ && text_pair((branch,"dev"))),
        (out is Main) == (text_pair((actual,repository)) && event@ == "workflow_run"@ && text_pair((branch,"main"))),
        (out is Dispatch) == (text_pair((actual,repository)) && event@ == "workflow_dispatch"@ && text_pair((git_ref,"refs/heads/dev"))),
        (out is WrongBranch) == (text_pair((actual,repository)) && event@ == "workflow_run"@ && !text_pair((branch,"dev")) && !text_pair((branch,"main"))),
        (out is WrongEvent) == (text_pair((actual,repository)) && event@ != "workflow_run"@ && !(event@ == "workflow_dispatch"@ && text_pair((git_ref,"refs/heads/dev")))),
{
    proof {
        reveal_strlit("workflow_run"); reveal_strlit("workflow_dispatch"); reveal_strlit("dev"); reveal_strlit("main");
        assert("workflow_run"@.len() != "workflow_dispatch"@.len());
        assert("dev"@.len() != "main"@.len());
    }
    if !text_fields(&[(actual, repository)]) { return ReleaseEvent::WrongRepository; }
    if same_text(event,"workflow_run") {
        if text_fields(&[(branch,"dev")]) { return ReleaseEvent::Dev; }
        if text_fields(&[(branch,"main")]) { return ReleaseEvent::Main; }
        return ReleaseEvent::WrongBranch;
    }
    if same_text(event,"workflow_dispatch") && text_fields(&[(git_ref,"refs/heads/dev")]) {
        ReleaseEvent::Dispatch
    } else { ReleaseEvent::WrongEvent }
}

pub enum DraftNotes { Keep, Replace, Refuse }
pub fn draft_notes(verify: bool, origins: bool, body: Option<&str>, expected: &str) -> (out: DraftNotes)
    ensures (out is Keep) == (!origins || text_pair((body,expected))),
        (out is Replace) == (origins && !text_pair((body,expected)) && !verify),
        (out is Refuse) == (origins && !text_pair((body,expected)) && verify),
{
    if !origins || text_fields(&[(body,expected)]) { DraftNotes::Keep }
    else if verify { DraftNotes::Refuse } else { DraftNotes::Replace }
}

pub enum MainRef { Current, FastForward, Invalid }
pub fn main_ref(reference: Option<&str>, kind: Option<&str>, oid: Option<&str>, target: &str) -> (out: MainRef)
    ensures match out {
        MainRef::Invalid => !text_pair((reference,"refs/heads/main")) || !text_pair((kind,"commit"))
            || !release_oid(target.spec_bytes())
            || !(oid matches Some(value) && release_oid(value.spec_bytes())),
        MainRef::Current => text_pair((reference,"refs/heads/main")) && text_pair((kind,"commit"))
            && release_oid(target.spec_bytes()) && text_pair((oid,target)),
        MainRef::FastForward => text_pair((reference,"refs/heads/main")) && text_pair((kind,"commit"))
            && release_oid(target.spec_bytes()) && (oid matches Some(value) && release_oid(value.spec_bytes()) && value@ != target@),
    },
{
    let Some(oid) = oid else { return MainRef::Invalid; };
    if !valid_release_oid(target.as_bytes()) || !valid_release_oid(oid.as_bytes())
        || !text_fields(&[(reference,"refs/heads/main"),(kind,"commit")]) { return MainRef::Invalid; }
    if same_text(oid,target) { MainRef::Current } else { MainRef::FastForward }
}

/// Missing tags can be created for a new release or a draft explicitly targeting this source.
pub fn publication_state(has_release: bool, reference: u8, draft: Option<bool>, target: Option<&str>,
    source: &str, missing: usize) -> (action: u8)
    ensures action == (if reference >= 2 { 3u8 }
        else if !has_release { 0u8 }
        else if draft == Some(true) && (reference == 1 || text_pair((target, source))) { 1u8 }
        else if draft == Some(false) && reference == 1 && missing == 0 { 2u8 }
        else { 3u8 }),
{
    if reference >= 2 { return 3; }
    if !has_release { return 0; }
    match draft_state(draft, missing) {
        1 if reference == 1 || text_fields(&[(target, source)]) => 1,
        0 if reference == 1 => 2,
        _ => 3,
    }
}

pub fn checksum_lines(expected: &[&str], observed: &[&str]) -> (valid: bool)
    ensures valid == (expected.len() == 2 && observed.len() == 2
        && expected@[0]@ == observed@[0]@ && expected@[1]@ == observed@[1]@),
{
    expected.len() == 2 && observed.len() == 2
        && same_text(expected[0], observed[0]) && same_text(expected[1], observed[1])
}

/// The compare endpoint reports head relative to base. Only these two states allow a fast-forward.
pub fn includes_base(status: Option<&str>) -> (yes: bool)
    ensures yes == (text_pair((status, "identical")) || text_pair((status, "ahead"))),
{
    text_fields(&[(status, "identical")]) || text_fields(&[(status, "ahead")])
}

/// A candidate covered by the latest stable release is terminal. A later descendant can continue.
pub fn covered_candidate(status: Option<&str>) -> (covered: Option<bool>)
    ensures covered == (if text_pair((status, "identical")) || text_pair((status, "behind")) { Some(true) }
                        else if text_pair((status, "ahead")) { Some(false) } else { None }),
{
    if text_fields(&[(status, "identical")]) || text_fields(&[(status, "behind")]) { Some(true) }
    else if text_fields(&[(status, "ahead")]) { Some(false) } else { None }
}

/// A release at the exact stable tag can finish the explicit-version plan. Drafts remain eligible for retry.
pub fn explicit_stable(tag: Option<&str>, expected: &str, draft: Option<bool>, preview: Option<bool>,
    published: Option<&str>) -> (complete: Option<bool>)
    ensures complete == (if !text_pair((tag, expected)) || draft.is_none() || preview.is_none() { None }
        else if draft == Some(true) || preview == Some(true) { Some(false) }
        else { match published { Some(s) if s@ != ""@ => Some(true), _ => None } }),
{
    if !text_fields(&[(tag, expected)]) || draft.is_none() || preview.is_none() { return None; }
    if draft == Some(true) || preview == Some(true) { return Some(false); }
    match published { Some(s) if !same_text(s,"") => Some(true), _ => None }
}

/// Digits up to end encode this positive number in canonical decimal.
pub open spec fn serial_digits(digits: Seq<u8>, end: nat, number: u64) -> bool
    decreases end,
{
    end > 0 && end <= digits.len() && number > 0 && digits[end - 1] == 48 + number % 10
        && (if end == 1 { number < 10 } else { serial_digits(digits, (end - 1) as nat, number / 10) })
}

fn serial_matches(digits: &[u8], end: usize, number: u64) -> (valid: bool)
    requires end <= digits.len(),
    ensures valid == serial_digits(digits@, end as nat, number),
    decreases end,
{
    if end == 0 || number == 0 || digits[end - 1] != 48 + (number % 10) as u8 { return false; }
    if end == 1 { number < 10 } else { serial_matches(digits, end - 1, number / 10) }
}

/// The adopted preview series and canonical serial must match the selected source run.
pub fn adopted_preview(main_check: Option<&str>, selected: &str, core: &str, version: &str,
    serial: &[u8], run_number: Option<u64>) -> (valid: bool)
    ensures valid == ((main_check.is_none() || text_pair((main_check, selected))) && core@ == version@
        && match run_number { Some(n) => serial_digits(serial@, serial.len() as nat, n), None => false }),
{
    (main_check.is_none() || text_fields(&[(main_check, selected)])) && same_text(core, version)
        && match run_number { Some(n) => serial_matches(serial, serial.len(), n), None => false }
}

pub fn draft_state(draft: Option<bool>, missing: usize) -> (action: u8)
    ensures action == match draft {
        Some(true) => 1u8,
        Some(false) if missing == 0 => 0u8,
        _ => 2u8,
    },
{
    match draft { Some(true) => 1, Some(false) if missing == 0 => 0, _ => 2 }
}

}
