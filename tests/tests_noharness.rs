#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "unstable-test"))]
include!("common/tests.rs");

fn main() {
    // invalid configuration otherwise
    #[cfg(not(feature = "unstable-test"))]
    DefaultPlatform::run();
}
