#[cfg(verus_keep_ghost)]
use super::policy::same_text;

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
fn expected_test_threads(owner: &str) -> (out: Option<&'static str>)
    ensures
        if owner@ == "worktree-tests"@ {
            out is Some && out.unwrap()@ == "--test-threads=8"@
        } else if owner@ == "judge-workflow-tests"@ || owner@ == "pr-tests"@
            || owner@ == "cache-tests"@ {
            out is Some && out.unwrap()@ == "--test-threads=2"@
        } else { out is None },
{
    if same_text(owner, "worktree-tests") {
        Some("--test-threads=8")
    } else if same_text(owner, "judge-workflow-tests")
        || same_text(owner, "pr-tests")
        || same_text(owner, "cache-tests") {
        Some("--test-threads=2")
    } else { None }
}

fn expect_thread_argument(actual: Option<&str>, expected: Option<&str>)
    requires match (actual, expected) {
        (Some(a), Some(e)) => a@ == e@,
        (None, None) => true,
        _ => false,
    },
{
    match (actual, expected) {
        (Some(a), Some(e)) => { if !same_text(a, e) { panic!(); } },
        (None, None) => (),
        _ => panic!(),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_thread_arguments_keep_all_four_exact_owners() {
    proof {
        reveal_strlit("worktree-tests"); reveal_strlit("judge-workflow-tests");
        reveal_strlit("pr-tests"); reveal_strlit("cache-tests");
        assert("worktree-tests"@.len() == 14);
        assert("judge-workflow-tests"@.len() == 20);
        assert("pr-tests"@.len() == 8); assert("cache-tests"@.len() == 11);
    }
    expect_thread_argument(expected_test_threads("worktree-tests"), Some("--test-threads=8"));
    expect_thread_argument(expected_test_threads("judge-workflow-tests"), Some("--test-threads=2"));
    expect_thread_argument(expected_test_threads("pr-tests"), Some("--test-threads=2"));
    expect_thread_argument(expected_test_threads("cache-tests"), Some("--test-threads=2"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unknown_case_unicode_and_nul_owners_do_not_gain_thread_arguments() {
    proof {
        reveal_strlit("worktree-tests"); reveal_strlit("judge-workflow-tests");
        reveal_strlit("pr-tests"); reveal_strlit("cache-tests");
        reveal_strlit(""); reveal_strlit("Cache-tests"); reveal_strlit("cache-tests\0");
        reveal_strlit("Worktree-tests"); reveal_strlit("worktree-tests\0");
        reveal_strlit("缓存/λ");
        assert("worktree-tests"@.len() == 14);
        assert("judge-workflow-tests"@.len() == 20);
        assert("pr-tests"@.len() == 8); assert("cache-tests"@.len() == 11);
        assert(""@.len() == 0); assert("Cache-tests"@.len() == 11);
        assert("cache-tests\0"@.len() == 12); assert("Worktree-tests"@.len() == 14);
        assert("worktree-tests\0"@.len() == 15); assert("缓存/λ"@.len() == 4);
        assert("Worktree-tests"@[0] != "worktree-tests"@[0]);
        assert("Cache-tests"@[0] != "cache-tests"@[0]);
    }
    expect_thread_argument(expected_test_threads(""), None);
    expect_thread_argument(expected_test_threads("Cache-tests"), None);
    expect_thread_argument(expected_test_threads("cache-tests\0"), None);
    expect_thread_argument(expected_test_threads("Worktree-tests"), None);
    expect_thread_argument(expected_test_threads("worktree-tests\0"), None);
    expect_thread_argument(expected_test_threads("缓存/λ"), None);
}
}
