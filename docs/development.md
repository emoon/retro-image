# Development

```sh
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Clean-room rules

RECOIL is GPL, so nobody working on retro-image reads its source, or the source of any
other GPL decoder. The `recoil2png` binary is fine to run as a black box. The full rules
are in [CLEANROOM.md](../CLEANROOM.md), and every decoder module lists its sources (see
[sources.md](sources.md)).

## Corpus tests

Most of the testing happens against real pictures, not unit tests. The pictures are
copyrighted, so the corpus isn't in the repository; [CLEANROOM.md](../CLEANROOM.md)
shows how to fetch RECOIL's example set into `corpus/`.

- `tests/oracle.rs` decodes every corpus file and compares it with `recoil2png`. RECOIL
  is a baseline, not the definition of correct. Where we deliberately differ, the
  difference and the evidence for it go in `crates/retro-image/tests/divergences/`.
- `tests/robustness.rs` feeds truncated and mutated files to every decoder and fails on
  any panic. It takes about a minute and a half. For fuzzing, see
  [fuzz/README.md](../fuzz/README.md).

Both tests skip the corpus silently when it isn't at `<workspace>/corpus`. In a git
worktree, set `RETRO_IMAGE_CORPUS=/path/to/corpus`.

## Adding a format

Follow [adding-a-format.md](adding-a-format.md). The per-platform research notes are in
[research/](research/README.md).

## Generated documents

`docs/formats.md` and `docs/coverage.md` are generated. Regenerate them after changing
the format registry:

```sh
scripts/formats.py > docs/formats.md

curl -sLo /tmp/recoil-formats.html https://recoil.sourceforge.net/formats.html
scripts/coverage.py /tmp/recoil-formats.html > docs/coverage.md
```
