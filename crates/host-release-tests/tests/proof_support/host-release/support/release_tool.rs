#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
fn compiler_fixture_allowed(distinct: bool, compiler: bool, active: bool) -> (allowed: bool)
    ensures allowed == (!distinct || !compiler || active),
{
    !distinct || !compiler || active
}
}
