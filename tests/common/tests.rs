use mintest::{*, test};

#[test]
fn empty_test() {
}

#[test(name = "empty_test_name_customized")]
fn empty_test_name() {
}

#[test]
fn test_ok() -> TestResult {
    OK
}

#[test(name = "test_skip", skip)]
fn test_skip() -> TestResult {
    err(&"don't pay attention to this")
}

#[test]
#[ignore]
fn test_ignore() {
    panic!("even the test crate should ignore this one!")
}

#[test(skip = "broken")]
fn test_skip_reason() -> TestResult {
    err(&"don't pay attention to this")
}

#[test(should_fail)]
fn test_err() -> TestResult {
    err(&"expected")
}

#[test(should_fail)]
fn test_err_macro() -> TestResult {
    err!("expected")
}

#[cfg(feature = "unstable")]
#[test(should_fail)]
fn test_option_try() -> TestResult {
    None::<usize>?;
    Ok(())
}

#[test(should_fail)]
fn test_none() -> Option<()> {
    None
}

#[test]
fn test_some() -> Option<()> {
    Some(())
}

#[test(should_panic)]
fn test_panic() {
    panic!("whee")
}

#[test]
#[should_panic]
fn test_panic_attr() {
    panic!("whee")
}

#[test(disable)]
fn test_disable() {
    panic!()
}

#[test(disable, no_compile)]
fn test_no_compile() {
    compile_error!("no_compile?")
}

#[test(skip, no_compile)]
fn test_no_compile_skip() {
    compile_error!("no_compile?")
}

/*
TODO: support this
#[test]
#[mintest(should_panic)]
fn test_multi_attr() {
    panic!()
}*/

#[mintest(should_panic)]
#[test]
fn test_multi_attr_reverse() {
    panic!()
}
