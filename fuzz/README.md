# Fuzzing

Needs nightly and `cargo install cargo-fuzz`. Run from the workspace root:

```sh
cargo +nightly fuzz run decode fuzz/corpus/decode corpus -- -max_total_time=600
```

`fuzz/corpus/decode` collects generated inputs (git-ignored); `corpus` is the sample
image set (see CLEANROOM.md), used read-only as seeds. Crashes land in
`fuzz/artifacts/decode/`; reproduce one with `cargo +nightly fuzz run decode <file>`,
then add a regression unit test for it in the decoder's module.

The `format` target fuzzes one decoder per input (first bytes pick the format and
a companion file), which finds far more than `decode`. Seed it from the sample set:

```sh
cargo run --release --example fuzz_seeds -- corpus fuzz/corpus/format
cargo +nightly fuzz run format fuzz/corpus/format -- -max_len=262144 \
  -rss_limit_mb=2048 -malloc_limit_mb=1024 -timeout=5
```

The memory and time limits make header-driven huge allocations and slow inputs
count as crashes.

The `simd` target runs every pixel primitive in `crates/retro-image/src/simd.rs` at
each SIMD level the CPU supports and checks it against the scalar reference:

```sh
cargo +nightly fuzz run simd fuzz/corpus/simd -- -max_total_time=300 -max_len=4096
```

`cargo test` also runs `crates/retro-image/tests/robustness.rs`, a deterministic
truncation/mutation pass that works on stable.
