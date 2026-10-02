# Adding a format

1. Read [CLEANROOM.md](../CLEANROOM.md). Work only from the sources listed for the format in
   [docs/formats/](formats/README.md), from permissively licensed code, or from sample files.
2. Implement the decoder in the platform's module under `crates/retro-image/src/platform/`
   and add a `Format::new(platform, name, extensions, decoder)` entry to its `FORMATS`.
   A module may become a directory (`atari_st.rs` + `atari_st/*.rs`).
   - The core crate is `no_std` + `alloc`: use `core::`/`alloc::`, never `std::`, and take
     input only as `&[u8]`.
   - Return `DecodeError::Unrecognized` for any data the format can't accept, including
     wrong sizes or truncated data. Never panic on bad input: check lengths before indexing.
   - Several formats share extensions. Validate size or magic bytes tightly so a decoder
     doesn't claim another format's files.
   - Platform names (`Format::platform`) follow RECOIL's format list, e.g. `"Atari ST"`,
     `"Commodore 64"`, so they match `$RETRO_IMAGE_PLATFORMS`.
3. Start the module doc comment with the sources the layout came from.
4. Add unit tests for the tricky parts (memory layout, unpacking, palette).
5. Run the oracle against RECOIL:
   `RETRO_IMAGE_PLATFORMS="Atari ST" cargo test -p retro-image --test oracle -- --nocapture`
6. `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.
   `cargo test` includes `tests/robustness.rs`, which feeds truncated and mutated
   corpus files to your decoder and fails on any panic.
7. For new decoders with packers or variable-length headers, also fuzz for a few minutes:
   `cargo +nightly fuzz run decode fuzz/corpus/decode corpus -- -max_total_time=300`
   (see [fuzz/README.md](../fuzz/README.md)).

Not supported yet: formats that need several files (e.g. `MIC+COL`). They need an API for
companion files, which hasn't been designed.
