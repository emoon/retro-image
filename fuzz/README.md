# Fuzzing

Needs nightly and `cargo install cargo-fuzz`. Run from the workspace root:

```sh
cargo +nightly fuzz run decode fuzz/corpus/decode corpus -- -max_total_time=600
```

`fuzz/corpus/decode` collects generated inputs (git-ignored); `corpus` is the sample
image set (see CLEANROOM.md), used read-only as seeds. Crashes land in
`fuzz/artifacts/decode/`; reproduce one with `cargo +nightly fuzz run decode <file>`,
then add a regression unit test for it in the decoder's module.

`cargo test` also runs `crates/retro-image/tests/robustness.rs`, a deterministic
truncation/mutation pass that works on stable.
