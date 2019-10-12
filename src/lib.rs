#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "unstable", feature(try_trait))]
#![cfg_attr(feature = "unstable-test", feature(test))]

use core::fmt;
use core::marker::PhantomData;

#[cfg(feature = "alloc")]
extern crate alloc;

pub use mintest_impl::*;
pub use mintest_impl::test as mintest;

pub trait Platform {
    type Stderr: fmt::Write;

    fn exit() -> !;
    fn abort() -> !;
    fn stderr() -> Self::Stderr;

    #[cfg(not(feature = "unstable-test"))]
    fn run() -> ! {
        run_tests::<Self, _, _>(TESTS)
    }
}

#[cfg(feature = "cortex-m-semihosting")]
mod cortex_m {
    use cortex_m_semihosting::{HStderr, debug, hstderr};
    pub struct CortexMSemihostingPlatform;

    impl super::Platform for CortexMSemihostingPlatform {
        type Stderr = HStderr;

        #[inline]
        fn exit() -> ! {
            debug::exit(Ok(()));
            unreachable!()
        }

        #[inline]
        fn abort() -> ! {
            debug::exit(Err(()));
            unreachable!()
        }

        #[inline]
        fn stderr() -> HStderr {
            hstderr()
        }
    }
}

#[cfg(feature = "cortex-m-semihosting")]
pub use self::cortex_m::CortexMSemihostingPlatform;
#[cfg(feature = "cortex-m-semihosting")]
pub type DefaultPlatform = CortexMSemihostingPlatform;

#[cfg(feature = "semihosting")]
mod semihosting_platform {
    pub struct SemihostingPlatform;

    impl super::Platform for SemihostingPlatform {
        type Stderr = semihosting::CharPrinter;

        #[inline]
        fn exit() -> ! {
            semihosting::exit()
        }

        #[inline]
        fn abort() -> ! {
            semihosting::abort()
        }

        #[inline]
        fn stderr() -> semihosting::CharPrinter {
            semihosting::CharPrinter
        }
    }
}

#[cfg(feature = "semihosting")]
pub use self::semihosting_platform::SemihostingPlatform;
#[cfg(feature = "semihosting")]
pub type DefaultPlatform = SemihostingPlatform;

#[cfg(feature = "std")]
mod std_platform {
    pub use std::{self, io, process};

    pub struct WriteWrapper<W>(W);

    impl<W: io::Write> core::fmt::Write for WriteWrapper<W> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0.write_all(s.as_bytes()).map_err(|_| core::fmt::Error)
        }
    }

    pub struct StdPlatform;

    impl super::Platform for StdPlatform {
        type Stderr = WriteWrapper<io::Stderr>;

        #[inline]
        fn exit() -> ! {
            process::exit(0)
        }

        #[inline]
        fn abort() -> ! {
            process::exit(-1)
        }

        #[inline]
        fn stderr() -> WriteWrapper<io::Stderr> {
            WriteWrapper(io::stderr())
        }
    }
}

#[cfg(feature = "std")]
pub use self::std_platform::StdPlatform;
#[cfg(feature = "std")]
pub type DefaultPlatform = StdPlatform;

pub struct WriteHole;
impl core::fmt::Write for WriteHole {
    #[inline]
    fn write_str(&mut self, _: &str) -> core::fmt::Result {
        Ok(())
    }
}

#[cfg(all(not(feature = "std"), not(feature = "cortex-m-semihosting"), not(feature = "semihosting")))]
pub type DefaultPlatform = UnknownPlatform;

pub struct UnknownPlatform;
impl Platform for UnknownPlatform {
    type Stderr = WriteHole;

    #[inline]
    fn exit() -> ! {
        // what can we do here :(
        panic!("^C:q!")
    }

    #[inline]
    fn abort() -> ! {
        // core::intrinsics::abort()?
        panic!("tests failed");
    }

    #[inline]
    fn stderr() -> WriteHole {
        WriteHole
    }
}

#[doc(hidden)]
pub mod internal {
    pub use core;
    #[cfg(feature = "linkme")]
    pub use linkme::distributed_slice;
    pub use super::DefaultPlatform;
}

pub type TestResult = Result<(), TestError>;

#[derive(Clone)]
pub enum TestFn {
    Static(fn(TestContext) -> TestResult),
    Plain(fn()),
}

pub trait IntoTestResult {
    fn into_test_result(self) -> TestResult;
}

impl IntoTestResult for () {
    #[inline]
    fn into_test_result(self) -> TestResult {
        Ok(self)
    }
}

// NOTE: would impl for T: Try but blankets are mean :(

impl<T: Into<()>, E: Into<TestError>> IntoTestResult for Result<T, E> {
    #[inline]
    fn into_test_result(self) -> TestResult {
        self.map(Into::into).map_err(Into::into)
    }
}

impl<T: Into<()>> IntoTestResult for Option<T> {
    #[inline]
    fn into_test_result(self) -> TestResult {
        self.map(Into::into).ok_or_else(|| TestError::none_error())
    }
}

pub enum TestError {
    Debug(&'static dyn fmt::Debug),
    Display(&'static dyn fmt::Display),
    #[cfg(feature = "std")]
    Panic(Box<dyn std::any::Any + Send + 'static>),
}

pub struct TestContext<'a> {
    pub test: &'a Test,
    pub index: usize,
    pub total: usize,
    _phantom: PhantomData<&'a ()>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TestExpected {
    Success,
    Fail,
    Panic,
}

#[derive(Debug, Copy, Clone)]
pub enum TestStatus {
    Enable,
    Skip(Option<&'static str>),
    Disable,
}

#[derive(Clone)]
pub struct Test {
    pub status: TestStatus,
    pub name: &'static str,
    pub test: TestFn,
    pub expected: TestExpected,
    //failure_handler: Option<fn(TestError)>, // TODO
}

impl AsRef<Test> for Test {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<'a> fmt::Debug for TestContext<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("TestContext")
            .field("test", self.test)
            .field("index", &self.index)
            .field("total", &self.total)
            .finish()
    }
}

impl fmt::Debug for TestFn {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let (name, ptr) = match self {
            TestFn::Plain(p) => ("TestFn::Plain", p as *const _ as *const ()),
            TestFn::Static(p) => ("TestFn::Static", p as *const _ as *const ()),
        };
        fmt.debug_tuple(name)
            .field(&ptr)
            .finish()
    }
}

impl fmt::Debug for Test {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Test")
            .field("name", &self.name)
            .field("status", &self.status)
            .field("expected", &self.expected)
            .field("test", &self.test)
            //.field("failure_handler", &self.failure_handler.as_ref().map(|_| "<FN>"))
            .finish()
    }
}

impl fmt::Debug for TestError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TestError::Display(d) => fmt.debug_tuple("TestError")
                .field(&format_args!("{}", d))
                .finish(),
            TestError::Debug(d) => fmt.debug_tuple("TestError")
                .field(d)
                .finish(),
            #[cfg(feature = "std")]
            TestError::Panic(panic) => fmt.debug_tuple("TestError")
                .field(&if let Some(panic) = panic.downcast_ref::<String>() {
                    &panic[..]
                } else if let Some(panic) = panic.downcast_ref::<&'static str>() {
                    panic
                } else {
                    "panic"
                }).finish(),
        }
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TestError::Display(d) => fmt::Display::fmt(d, fmt),
            TestError::Debug(d) => fmt::Debug::fmt(d, fmt),
            #[cfg(feature = "std")]
            TestError::Panic(panic) => fmt.write_str(if let Some(panic) = panic.downcast_ref::<String>() {
                &panic[..]
            } else if let Some(panic) = panic.downcast_ref::<&'static str>() {
                panic
            } else {
                "panic"
            }),
        }
    }
}

impl fmt::Display for TestExpected {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(match self {
            TestExpected::Success => "success",
            TestExpected::Fail => "failure",
            TestExpected::Panic => "panic",
        })
    }
}

impl<S: fmt::Display> From<&'static S> for TestError {
    #[inline]
    fn from(s: &'static S) -> Self {
        TestError::Display(s as &_)
    }
}

#[cfg(feature = "unstable")]
impl From<core::option::NoneError> for TestError {
    #[inline]
    fn from(n: core::option::NoneError) -> Self {
        TestError::none_error()
    }
}

impl TestError {
    pub fn none_error() -> Self {
        TestError::Display(&"None?")
    }
}

pub const OK: TestResult = Ok(());
pub fn ok() -> TestResult { OK }
pub fn err<S: fmt::Display>(s: &'static S) -> TestResult { Err(TestError::Display(s as &_)) }
pub fn err_debug<S: fmt::Debug>(s: &'static S) -> TestResult { Err(TestError::Debug(s as &_)) }

#[cfg(all(feature = "linkme", not(all(feature = "unstable-test", not(feature = "test")))))]
#[linkme::distributed_slice]
pub static TESTS: [Test] = [..];

pub fn test_all<I: AsRef<Test>, T: IntoIterator<Item=I>>(fmt: &mut dyn fmt::Write, tests: T) -> MainResult where
T::IntoIter: Clone {
    #[cfg(feature = "color-backtrace")]
    {
        use color_backtrace::{install_with_settings, Settings, Verbosity};
        install_with_settings(Settings::new().verbosity(Verbosity::Medium))
    }

    let test_filter = |t: &I| match t.as_ref().status {
        TestStatus::Disable => false,
        _ => true,
    };

    let tests = tests.into_iter().filter(test_filter);
    let total = tests.clone().count();
    let _ = writeln!(fmt, "running {} tests", total);

    let (mut passed, mut failed, mut skipped) = (0usize, 0usize, 0usize);
    for (index, test) in tests.enumerate() {
        let test = test.as_ref();
        let status = match test.status {
            #[cfg(not(feature = "std"))]
            TestStatus::Enable if test.expected == TestExpected::Panic =>
                TestStatus::Skip(Some("no-std but panic expected")),
            status => status,
        };

        match status {
            TestStatus::Disable => (),
            TestStatus::Enable => {
                let _ = write!(fmt, "{} ... ", test.name);
                let context = TestContext {
                    index,
                    total,
                    test,
                    _phantom: PhantomData,
                };
                let test_fn = |context| match test.test {
                    TestFn::Static(f) => f(context),
                    TestFn::Plain(f) => {
                        f();
                        Ok(())
                    },
                };
                let result = match test.expected {
                    #[cfg(feature = "std")]
                    TestExpected::Panic => {
                        use std::panic;

                        let hook = panic::take_hook();
                        panic::set_hook(Box::new(|_| ()));
                        let res = match std::panic::catch_unwind(move || (test_fn)(context)) {
                            Ok(res) => res,
                            Err(res) => Err(TestError::Panic(res)),
                        };
                        panic::set_hook(hook);
                        res
                    },
                    _ => (test_fn)(context),
                };
                match (result, test.expected) {
                    #[cfg(not(feature = "std"))]
                    (_, TestExpected::Panic) => panic!("no-std but panic expected"),
                    (Ok(()), TestExpected::Success) => {
                        passed += 1;
                        let _ = writeln!(fmt, "\x1b[34mOK\x1b[0m");
                    },
                    (Ok(()), expected) => {
                        failed += 1;
                        let _ = writeln!(fmt, "\x1b[31mFAIL: expected {} but test passed\x1b[0m", expected);
                    },
                    (Err(e), TestExpected::Success) => {
                        failed += 1;
                        let _ = writeln!(fmt, "\x1b[31mFAIL: {}\x1b[0m", e);
                    },
                    #[cfg(feature = "std")]
                    (Err(panic @ TestError::Panic(..)), TestExpected::Panic) => {
                        passed += 1;
                        let _ = writeln!(fmt, "\x1b[34mOK: {}\x1b[0m", panic);
                    },
                    #[cfg(feature = "std")]
                    (Err(e), TestExpected::Panic) => {
                        failed += 1;
                        let _ = writeln!(fmt, "\x1b[31mFAIL: expected panic, got {}\x1b[0m", e);
                    },
                    (Err(e), TestExpected::Fail) => {
                        passed += 1;
                        let _ = writeln!(fmt, "\x1b[34mOK: {}\x1b[0m", e);
                    },
                }
            },
            TestStatus::Skip(Some(reason)) => {
                skipped += 1;
                let _ = writeln!(fmt, "{} ... \x1b[33mSkipped: {}\x1b[0m", test.name, reason);
            },
            TestStatus::Skip(None) => {
                skipped += 1;
                let _ = writeln!(fmt, "{} ... \x1b[33mSkipped\x1b[0m", test.name);
            },
        }
    }

    MainResult {
        passed,
        failed,
        skipped,
    }
}

pub struct MainResult {
    passed: usize,
    failed: usize,
    skipped: usize,
}

impl MainResult {
    #[inline]
    pub fn succeeded(&self) -> bool {
        self.failed == 0
    }
}

impl fmt::Display for MainResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let result_str = if self.succeeded() {
            "ok"
        } else {
            "FAILED"
        };
        write!(f, "test result: {}. {} passed; {} failed; {} skipped", result_str, self.passed, self.failed, self.skipped)
    }
}

#[cfg(all(feature = "alloc", feature = "test"))]
use alloc::borrow::Cow;

#[cfg(all(feature = "unstable-test", feature = "test"))]
extern crate test as test_;

#[cfg(all(feature = "unstable-test", feature = "test"))]
pub fn runner(tests: &[&test_::TestDescAndFn]) -> ! {
    let tests = tests.iter().map(|test| Test {
        status: match (test.desc.ignore, test.desc.allow_fail) {
            (_, true) => panic!("allow_fail unsupported"),
            (false, _) => TestStatus::Enable,
            (true, _) => TestStatus::Skip(Some("ignore")),
        },
        name: match &test.desc.name {
            test_::StaticTestName(name) => *name,
            test_::AlignedTestName(Cow::Borrowed(name), _) => *name,
            test_::AlignedTestName(..) | test_::DynTestName(..) =>
                panic!("dynamic test names unsupported"),
        },
        expected: match test.desc.should_panic {
            test_::ShouldPanic::No => TestExpected::Success,
            test_::ShouldPanic::Yes => TestExpected::Panic,
            test_::ShouldPanic::YesWithMessage(_) => panic!("should_panic expected message unimplemented"),
        },
        test: match test.testfn {
            test_::StaticTestFn(f) => TestFn::Plain(f),
            _ => panic!("unsupported test fn"),
        },
    });

    #[cfg(feature = "linkme")]
    run_tests::<DefaultPlatform, _, _>(TESTS.iter().map(Cow::Borrowed).chain(tests.map(Cow::Owned)));

    #[cfg(not(feature = "linkme"))]
    run_tests::<DefaultPlatform, _, _>(tests);
}

#[cfg(all(feature = "unstable-test", not(feature = "test")))]
pub fn runner(tests: &[&Test]) -> ! {
    run_tests::<DefaultPlatform, _, _>(tests);
}

pub fn run_tests<P: Platform + ?Sized, I: AsRef<Test>, T: IntoIterator<Item=I>>(tests: T) -> ! where
T::IntoIter: Clone {
    use fmt::Write;

    let mut stderr = P::stderr();
    let results = test_all(&mut stderr, tests);
    let _ = writeln!(stderr, "{}", results);
    match results.succeeded() {
        true => P::exit(),
        false => P::abort(),
    }
}

#[macro_export]
macro_rules! err {
    ($msg:literal) => {
        return $crate::err(&$msg).into()
    };
}
