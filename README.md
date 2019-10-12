no-std test harness

## Unsorted TODOs

- [ ] Pre-test constructors and teardown may be relevant for embedded platforms that set up peripherals?
- [ ] Reorganize crate because the `Test` prefix is unnecessary for a lot of items only pulled in by the macro. Test description types could go in a submodule?
- [ ] Assertion macros that try/throw a Result instead of panic
  - Also unwrap/expect macros
- [ ] Support should_panic under the following conditions
  - It's the only test being run (or can be sorted so it runs last?)
  - The crate is allowed control over the panic handler (feature flag?)
- [ ] Clean up and pull the panic handler out of the test fn
  - ... and catch panics for all tests, not just `should_panic` ones
- [ ] Write meta tests that can assert failures are working
- [ ] Should `run_tests` just return normally instead of exiting?
