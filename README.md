# unit test for compile-time regression in rustc

This is a small repro for the compile time regression as exposed in https://github.com/rust-lang/rust/issues/91128.

## steps to reproduce

- with rust stable 1.56, `time cargo build --release` => around 26 seconds on
  my machine
- with rust nightly, `time cargo +nightly build --release` => around 2 minutes
  on my machine
