#![cfg(all(feature = "unstable-test", feature = "test"))]
#![feature(custom_test_frameworks)]
#![test_runner(mintest::runner)]
#![no_std]

use mintest::mintest;

#[test]
fn empty_test() {
}

#[test]
#[ignore]
fn test_ignore() {
    panic!("even the test crate should ignore this one!")
}

#[test]
#[should_panic]
fn test_panic_attr() {
    panic!("whee")
}

#[cfg(feature = "linkme")]
#[mintest]
fn mixed_test() {
}

/* TODO
#[test]
#[mintest]
fn double_mixer() {
}
*/
