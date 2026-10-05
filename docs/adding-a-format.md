# Adding a format

1. Read [CLEANROOM.md](../CLEANROOM.md). Work only from the sources listed for the format in
   [docs/research/](research/README.md), from permissively licensed code, or from sample files.
2. Implement the decoder in the platform's module under `crates/retro-image/src/platform/`
   and add a `Format::new(platform, name, extensions, decoder)` entry to its `FORMATS`.
   A module may become a directory (`atari_st.rs` + `atari_st/*.rs`).
   - The core crate is `no_std` + `alloc`: use `core::`/`alloc::`, never `std::`, and take
     input only as `&[u8]`.
   - Return `DecodeError::Unrecognized` for any data the format can't accept, including
     wrong sizes or truncated data. Never panic on bad input: check lengths before indexing.
   - Several formats share extensions. Validate size or magic bytes tightly so a decoder
     doesn't claim another format's files.
   - If the decoder checks a reliable signature (magic bytes, or a header validated so
     strictly that random data can't pass), chain `.signature()` on the `Format`. It is
     then also tried for files with other or no extensions (content detection). Never
     mark headerless memory dumps or formats recognised only by size.
   - Formats with companion files (e.g. `MIC`+`COL`, `SCR`+`PAL`) register with
     `Format::with_companions` and get a `&dyn Companions` to fetch siblings by
     extension. The main file must still decode alone where the format allows (e.g.
     with a default palette): callers such as sandboxed thumbnailers only have one file.
     The oracle checks both: the file alone, and with its siblings against RECOIL
     given the same files (`<id> +companions` in divergence files). A main file that
     lists the files it needs (a scroll list) fetches them with `Companions::get_named`;
     the oracle copies every file served that way next to the main file.
   - Reuse the shared helpers instead of writing local copies: `crate::bytes`
     (`le16`/`le32`/`be16`/`be32` at an offset, `None` past the end), `Image::from_indexed`,
     `Image::from_bits` (1-bit bitmaps, `BitOrder`), `Image::scaled`, `Image::blend`,
     `codec::packbits`, and `tiles::TileLayout` (tiles stored as bit planes or packed
     pixels, and sheets of them; the module header says how to describe a new layout).
   - Keep a format's transparency as straight alpha and never composite it onto gray or
     any other background. `Image::from_argb` and `Image::set_argb` take `0xAARRGGBB`
     pixels, `Image::from_indexed_argb` and `TileLayout::sheet_argb` take a palette of
     them, `Image::draw` puts one pixel over another, and `Image::with_alpha` attaches a
     plane the decoder built itself. Tests check alpha with `Image::get_argb` and `CLEAR`.
     Before you trust a fourth byte or a mask, look at a real file: if it is zero for
     every pixel of your samples it is probably padding.
   - Platform names (`Format::platform`) reuse a name from `docs/formats.md` where one
     fits, e.g. `"Atari ST"` or `"Commodore 64"`, so they match `$RETRO_IMAGE_PLATFORMS`.
3. Start every file's `//!` doc comment with the documents its implementation is based on,
   with links (use a Wayback Machine URL when the live page is gone). Where knowledge
   came from reverse engineering, say so and name the samples or the `recoil2png`
   probing. Submodules don't inherit their parent's sources; registry files point at
   their `docs/research/` survey.
4. Add unit tests for the tricky parts (memory layout, unpacking, palette).
5. Run the oracle against RECOIL:
   `RETRO_IMAGE_PLATFORMS="Atari ST" cargo test -p retro-image --test oracle -- --nocapture`
   RECOIL is the baseline, not the definition of correct. If RECOIL crashes on, rejects
   or misrenders a valid file, decode it properly anyway. After reviewing our output,
   record it in `crates/retro-image/tests/divergences/<group>.tsv` with the evidence (spec
   reference, emulator or original-program render). The oracle's failure message prints
   the fingerprint to copy. Never record a divergence just to make a failure go away.
6. `cargo fmt --all`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`.
   `cargo test` includes `tests/robustness.rs`, which feeds truncated and mutated
   corpus files to your decoder and fails on any panic. Both corpus tests silently
   skip the corpus when it isn't at `<workspace>/corpus` (e.g. in a git worktree),
   so set `RETRO_IMAGE_CORPUS=/path/to/corpus` there.
7. For new decoders with packers or variable-length headers, also fuzz for a few minutes:
   `cargo +nightly fuzz run decode fuzz/corpus/decode corpus -- -max_total_time=300`
   (see [fuzz/README.md](../fuzz/README.md)).
8. Regenerate the format list and the coverage report (see
   [development.md](development.md#generated-documents)).
