#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// A failed cleanup must fail a successful test, while an existing panic
/// must keep its original diagnostic and avoid a second unwind.
pub(crate) fn propagate_cleanup_error(test_panicked: bool, cleanup_failed: bool) -> (raise: bool)
    ensures raise == (cleanup_failed && !test_panicked),
{
    cleanup_failed && !test_panicked
}
}
