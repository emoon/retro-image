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
count as crashes. libfuzzer's `-timeout` is coarse, though: it missed a decode that
took 4.3 s. So the `format` target times every decode itself and fails on any that
takes over 2 s, whatever the flags say.

`fuzz_seeds` also writes a few files that random mutation will not build: FLIC, FLC
and FLH files with a large screen and tens of thousands of 6-byte BLACK or COPY
chunks, a Utah RLE file that repeats a full-row run, a KT4 file that fills the whole
screen over and over, and a KiSS set that draws one large cel in thousands of
entries. Each one costs a decoder that does work per operation, not per input byte,
seconds. The decoders now bound that work, and these seeds are the check that it
stays bounded.

The `simd` target runs every pixel primitive in `crates/retro-image/src/simd.rs` at
each SIMD level the CPU supports and checks it against the scalar reference:

```sh
cargo +nightly fuzz run simd fuzz/corpus/simd -- -max_total_time=300 -max_len=4096
```

`cargo test` also runs `crates/retro-image/tests/robustness.rs`, a deterministic
truncation/mutation pass that works on stable.
