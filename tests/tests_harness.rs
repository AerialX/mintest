#![cfg(all(feature = "unstable-test", not(all(not(feature = "linkme"), feature = "test"))))]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable-test", feature(custom_test_frameworks))]
#![cfg_attr(feature = "unstable-test", test_runner(mintest::runner))]

include!("common/tests.rs");
