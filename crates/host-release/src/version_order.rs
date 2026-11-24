//! Decimal components are canonical SemVer numbers without a machine-integer limit.
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use vstd::slice::slice_to_vec;

verus! {

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
