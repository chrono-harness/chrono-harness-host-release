#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
fn compiler_fixture_directory(distinct: bool) -> (path: Option<&'static str>)
    ensures match path {
        Some(name) => distinct && name@ == "active"@,
        None => !distinct,
    },
{
    if distinct { Some("active") } else { None }
}
}

#[path = "failure_case.rs"]
pub(crate) mod failure_case;
