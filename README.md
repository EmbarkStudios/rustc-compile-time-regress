# unit test for compile-time regression in rustc

## steps to reproduce

- with rust stable 1.56, `time cargo build --release` => around 26 seconds on
  my machine
- with rust nightly, `time cargo +nightly build --release` => around 2 minutes
  on my machine
