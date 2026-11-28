#[cfg(verus_keep_ghost)]
use super::policy;
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::verified_recipe as policy;
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg(verus_keep_ghost)]
fn compiler_fixture_directory(distinct: bool) -> (path: Option<&'static str>)
    ensures match path {
        Some(name) => distinct && name@ == "active"@,
        None => !distinct,
    },
{
    if distinct { Some("active") } else { None }
}

pub(crate) fn expect_recipe(actual: bool)
    requires actual,
{
    if !actual {
        panic!();
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_completion_keeps_the_first_failure_and_original_platform_span() {
    use policy::execution::{unit_completion, UnitFailure};
    let child = unit_completion(true, true, None, 100, Some(90), 1, 1);
    expect_recipe(child.failure matches Some(UnitFailure::Child) && child.platform_span.is_none());
    let operation = unit_completion(false, true, None, 100, Some(90), 1, 1);
    expect_recipe(operation.failure matches Some(UnitFailure::Operation));
    let timestamp = unit_completion(false, false, None, 100, Some(90), 1, 1);
    expect_recipe(timestamp.failure matches Some(UnitFailure::Timestamp));
    let operation = unit_completion(false, true, Some(2_000_000_000), 101, Some(100), 1, 1);
    expect_recipe(operation.failure matches Some(UnitFailure::Operation)
        && operation.platform_span == Some(1_999_999_900));
    let child = unit_completion(true, true, Some(2_000_000_000), 101, Some(100), 1, 1);
    expect_recipe(child.failure matches Some(UnitFailure::Child)
        && child.platform_span == Some(1_999_999_900));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_completion_admits_exact_limits_and_rejects_backwards_or_late_finishes() {
    use policy::execution::{unit_completion, UnitFailure};
    let exact = unit_completion(false, false, Some(2_000_000_100), 1_000_000_100, Some(100), 1, 2);
    expect_recipe(exact.failure.is_none() && exact.platform_span == Some(2_000_000_000));
    let both_late = unit_completion(false, false, Some(2_000_000_101), 1_000_000_100, Some(100), 1, 2);
    expect_recipe(both_late.failure matches Some(UnitFailure::UnitSpan)
        && both_late.platform_span == Some(2_000_000_001));
    let platform_late = unit_completion(false, false, Some(2_000_000_101), 1_000_000_101, Some(100), 1, 2);
    expect_recipe(platform_late.failure matches Some(UnitFailure::PlatformSpan));
    let unit_backwards = unit_completion(false, false, Some(99), 100, Some(90), 1, 1);
    expect_recipe(unit_backwards.failure matches Some(UnitFailure::UnitSpan)
        && unit_backwards.platform_span == Some(9));
    let platform_backwards = unit_completion(false, false, Some(100), 99, Some(101), 1, 1);
    expect_recipe(platform_backwards.failure matches Some(UnitFailure::PlatformSpan)
        && platform_backwards.platform_span.is_none());
    let only_unit = unit_completion(false, false, Some(u64::MAX), 1, None, u64::MAX, 0);
    expect_recipe(only_unit.failure.is_none() && only_unit.platform_span.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn release_receipt_write_failure_preserves_the_original_exit_or_fails_success() {
    use policy::execution::unit_exit;
    expect_recipe(unit_exit(None, false, true) == 0);
    expect_recipe(unit_exit(None, false, false) == 2);
    expect_recipe(unit_exit(None, true, true) == 2);
    expect_recipe(unit_exit(None, true, false) == 2);
    expect_recipe(unit_exit(Some(37), true, false) == 37);
    expect_recipe(unit_exit(Some(-15), true, false) == 143);
}

}

#[cfg(verus_keep_ghost)]
#[path = "failure_case.rs"]
pub(crate) mod failure_case;
