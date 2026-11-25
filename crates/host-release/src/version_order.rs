//! Decimal components are canonical SemVer numbers without a machine-integer limit.
use std::cmp::Ordering;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use vstd::slice::slice_to_vec;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

verus! {

/// GitHub supplies the relation names of the final release-page response.
pub fn next_release_page(relations: &[&str]) -> (more: bool)
    ensures more == (exists|i: int| 0 <= i < relations.len() && (#[trigger] relations@[i])@ == "next"@),
{
    let mut i = 0;
    while i < relations.len()
        invariant i <= relations.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] relations@[j])@ != "next"@,
        decreases relations.len() - i,
    {
        if super::same_text(relations[i], "next") { return true; }
        i += 1;
    }
    false
}

/// The public channel must agree with both GitHub and the SemVer prerelease component.
pub fn install_channel(preview: bool, github_prerelease: bool, version_prerelease: bool) -> (eligible: bool)
    ensures eligible == (preview == github_prerelease && preview == version_prerelease),
{
    preview == github_prerelease && preview == version_prerelease
}

/// Exact installations accept both stable and preview tags in their original page order.
pub type InstallRelease<'a> = (Option<bool>, Option<&'a str>, Option<&'a str>);

pub open spec fn public_exact(row: InstallRelease, requested: &str) -> bool {
    row.0 == Some(false) && match (row.1, row.2) {
        (Some(date), Some(tag)) => date@.len() > 0 && tag@ == requested@,
        _ => false,
    }
}

pub fn exact_install(rows: &[InstallRelease], requested: &str) -> (found: Option<usize>)
    ensures match found {
        Some(i) => i < rows.len() && public_exact(rows@[i as int], requested)
            && forall|j: int| 0 <= j < i ==> !public_exact(#[trigger] rows@[j], requested),
        None => forall|j: int| 0 <= j < rows.len() ==> !public_exact(#[trigger] rows@[j], requested),
    },
{
    let mut i = 0;
    while i < rows.len()
        invariant i <= rows.len(),
            forall|j: int| 0 <= j < i ==> !public_exact(#[trigger] rows@[j], requested),
        decreases rows.len() - i,
    {
        if let (Some(draft), Some(date), Some(tag)) = rows[i] {
            if super::roadmap::published(draft, date) && super::same_text(tag, requested) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

pub open spec fn earlier(a: Seq<u8>, b: Seq<u8>) -> bool {
    a.len() < b.len() || (a.len() == b.len()
        && exists|i: int| 0 <= i < a.len() && a[i] < b[i]
            && forall|j:int| 0 <= j < i ==> a[j] == b[j])
}

pub fn compare(a: &[u8], b: &[u8]) -> (order: i8)
    ensures (order == -1) == earlier(a@, b@), (order == 0) == (a@ == b@),
            (order == 1) == earlier(b@, a@), -1 <= order <= 1,
{
    if a.len() < b.len() { return -1; }
    if a.len() > b.len() { return 1; }
    let mut i = 0;
    while i < a.len()
        invariant a.len() == b.len(), i <= a.len(),
            forall|j:int| 0 <= j < i ==> a@[j] == b@[j],
        decreases a.len() - i,
    {
        if a[i] != b[i] {
            proof {
                assert forall|k:int| 0 <= k < a.len() && (forall|j:int| 0 <= j < k ==> a@[j] == b@[j])
                    && a@[k] != b@[k] implies k == i by {
                    if k > i { assert(a@[i as int] == b@[i as int]); }
                }
                if a@[i as int] < b@[i as int] { assert(earlier(a@, b@)); } else { assert(earlier(b@, a@)); }
            }
            return if a[i] < b[i] { -1 } else { 1 };
        }
        i += 1;
    }
    assert(a@ =~= b@);
    0
}

pub open spec fn core_above(a: [&[u8];3], b: [&[u8];3]) -> bool {
    earlier(b[0]@, a[0]@) || (a[0]@ == b[0]@ && (earlier(b[1]@, a[1]@)
        || (a[1]@ == b[1]@ && earlier(b[2]@, a[2]@))))
}

pub fn above(a: [&[u8];3], b: [&[u8];3]) -> (yes: bool)
    ensures yes == core_above(a,b),
{
    let major=compare(a[0],b[0]);
    if major != 0 { return major > 0; }
    let minor=compare(a[1],b[1]);
    if minor != 0 { return minor > 0; }
    compare(a[2],b[2]) > 0
}

/// GitHub JSON and the leading-v split supply the observations. The original
/// tag enters the ordering only when its publication fields and digits qualify.
pub fn stable_key<'a>(
    tag: &str, parts: &[&'a str], draft: bool, prerelease: bool, date: &str,
) -> (key: Option<[&'a [u8]; 3]>)
    ensures key.is_some() == (tag.spec_bytes().len() > 0 && tag.spec_bytes()[0] == 118
        && !draft && !prerelease && date@.len() > 0 && parts.len() == 3
        && forall|i: int| 0 <= i < parts.len() ==> super::delivery::decimal(parts@[i].spec_bytes(), true)),
        key.is_some() ==> forall|i:int| 0 <= i < 3 ==> key.unwrap()[i]@ == parts@[i].spec_bytes(),
{
    let raw = tag.as_bytes();
    if raw.is_empty() || raw[0] != 118 || prerelease || !super::roadmap::published(draft, date)
        || !super::delivery::version_numbers(parts) {
        return None;
    }
    Some([parts[0].as_bytes(), parts[1].as_bytes(), parts[2].as_bytes()])
}

proof fn earlier_transitive(a: Seq<u8>, b: Seq<u8>, c: Seq<u8>)
    requires earlier(a,b), earlier(b,c),
    ensures earlier(a,c),
{
    if a.len() == c.len() {
        let i = choose|i:int| 0 <= i < a.len() && a[i] < b[i]
            && forall|j:int| 0 <= j < i ==> a[j] == b[j];
        let k = choose|k:int| 0 <= k < b.len() && b[k] < c[k]
            && forall|j:int| 0 <= j < k ==> b[j] == c[j];
        let first = if i <= k { i } else { k };
        assert(0 <= first < a.len() && a[first] < c[first]);
        assert forall|j:int| 0 <= j < first implies a[j] == c[j] by { }
        assert(earlier(a,c));
    }
}

proof fn above_transitive(a: [&[u8];3], b: [&[u8];3], c: [&[u8];3])
    requires core_above(a,b), core_above(b,c),
    ensures core_above(a,c),
{
    if earlier(c[0]@,b[0]@) && earlier(b[0]@,a[0]@) {
        earlier_transitive(c[0]@,b[0]@,a[0]@);
    } else if earlier(c[1]@,b[1]@) && earlier(b[1]@,a[1]@) {
        earlier_transitive(c[1]@,b[1]@,a[1]@);
    } else if earlier(c[2]@,b[2]@) && earlier(b[2]@,a[2]@) {
        earlier_transitive(c[2]@,b[2]@,a[2]@);
    }
}

/// A supplied page may be empty or contain no published canonical tag.
/// Global input order and page boundaries do not affect the maximum version.
pub fn latest_stable(keys: &[Option<[&[u8];3]>]) -> (best: Option<usize>)
    ensures
        best.is_none() == (forall|j:int| 0 <= j < keys.len() ==> keys@[j].is_none()),
        best.is_some() ==> best.unwrap() < keys.len() && keys@[best.unwrap() as int].is_some(),
        best.is_some() ==> forall|j:int| 0 <= j < keys.len() && keys@[j].is_some()
            ==> !core_above(keys@[j].unwrap(),keys@[best.unwrap() as int].unwrap()),
{
    let mut best: Option<usize> = None;
    let mut i = 0;
    while i < keys.len()
        invariant i <= keys.len(),
            best.is_none() == (forall|j:int| 0 <= j < i ==> keys@[j].is_none()),
            best.is_some() ==> best.unwrap() < i && keys@[best.unwrap() as int].is_some(),
            best.is_some() ==> forall|j:int| 0 <= j < i && keys@[j].is_some()
                ==> !core_above(keys@[j].unwrap(),keys@[best.unwrap() as int].unwrap()),
        decreases keys.len() - i,
    {
        if let Some(key) = keys[i] {
            if let Some(previous) = best {
                let old = keys[previous].unwrap();
                if above(key, old) {
                    proof {
                        assert forall|j:int| 0 <= j < i && keys@[j].is_some()
                            implies !core_above(keys@[j].unwrap(), key) by {
                            if core_above(keys@[j].unwrap(), key) {
                                above_transitive(keys@[j].unwrap(), key, old);
                            }
                        }
                    }
                    best = Some(i);
                }
            } else {
                best = Some(i);
            }
        }
        i += 1;
    }
    best
}

// The returned prefix is unchanged, the last non-nine grows by one, and
// trailing nines become zeros. An all-nines input gains one leading digit.
pub open spec fn digit_carry(a:Seq<u8>, b:Seq<u8>, i:int) -> bool {
    0 <= i < a.len() && a[i]<57 && b[i] as int==a[i] as int+1
        && (forall|j:int| 0<=j<i ==> a[j]==b[j])
        && (forall|j:int| i<j<a.len() ==> a[j]==57 && b[j]==48)
}

pub open spec fn decimal_successor(a: Seq<u8>, b: Seq<u8>) -> bool {
    (b.len()==a.len() && exists|i:int| #[trigger] digit_carry(a,b,i))
    || (b.len()==a.len()+1 && b[0]==49
        && forall|j:int| 0<=j<a.len() ==> a[j]==57 && b[j+1]==48)
}

pub fn increment(a: &[u8]) -> (out: Vec<u8>)
    requires super::delivery::decimal(a@,true),
    ensures decimal_successor(a@,out@),
{
    let mut i = a.len();
    let mut digits = slice_to_vec(a);
    while i > 0
        invariant i<=a.len(), digits.len()==a.len(), super::delivery::decimal(a@,true),
            forall|j:int| 0<=j<i ==> digits@[j]==a@[j],
            forall|j:int| i<=j<a.len() ==> a@[j]==57 && digits@[j]==48,
        decreases i,
    {
        i -= 1;
        if a[i] < 57 {
            digits[i] = a[i]+1;
            assert(digit_carry(a@, digits@, i as int));
            assert(decimal_successor(a@, digits@));
            return digits;
        }
        digits[i] = 48;
    }
    let mut carried=vec![49u8];
    carried.extend_from_slice(&digits);
    assert(carried@ =~= seq![49u8] + digits@);
    carried
}
}

verus! {

/// GitHub timestamp and tag ordering are supplied by Rust; SemVer supplies the version comparison.
pub fn install_precedence(preview: bool, date: Ordering, version: Ordering, tag: Ordering) -> (out: Ordering)
    ensures out == if preview && date != Ordering::Equal { date }
        else if version != Ordering::Equal { version }
        else if date != Ordering::Equal { date }
        else { tag },
{
    match (preview, date, version) {
        (true, Ordering::Less, _) => Ordering::Less,
        (true, Ordering::Greater, _) => Ordering::Greater,
        (_, _, Ordering::Less) => Ordering::Less,
        (_, _, Ordering::Greater) => Ordering::Greater,
        (_, Ordering::Less, _) => Ordering::Less,
        (_, Ordering::Greater, _) => Ordering::Greater,
        _ => tag,
    }
}

}
