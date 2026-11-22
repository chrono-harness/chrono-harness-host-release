#[cfg(verus_keep_ghost)]
use super::policy::native_delivery as delivery;

#[cfg(verus_keep_ghost)]
use super::policy::publication;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn original_run_and_attempt_must_both_be_positive_and_canonical() {
    let valid = publication::publication_attempt(4567, 3);
    let absent_run = publication::publication_attempt(0, 3);
    let absent_attempt = publication::publication_attempt(4567, 0);
    let number = delivery::canonical_positive(3, true);
    let padded = delivery::canonical_positive(3, false);
    let zero = delivery::canonical_positive(0, true);
    if !(valid && !absent_run && !absent_attempt && number && !padded && !zero) { panic!(); }
}
}
