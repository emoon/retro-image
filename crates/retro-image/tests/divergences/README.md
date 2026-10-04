# Divergences from RECOIL

RECOIL is the baseline, not the definition of correct: when it crashes on,
rejects, or misrenders a valid file, retro-image decodes it properly and the
file is recorded here. The oracle test then checks our output against the
recorded fingerprint instead of RECOIL's.

One `.tsv` file per platform group (`amiga-apple-misc.tsv`, `atari8.tsv`,
`atari-st.tsv`, `coco3.tsv`, `commodore.tsv`, `msx-japanese.tsv`, `sinclair-cpc-misc.tsv`,
`textmode.tsv`, `riscos-ql.tsv`, `thomson.tsv`, `zx-snapshots.tsv`), so work on different platforms
doesn't conflict.

Formats RECOIL doesn't support at all (e.g. ANSI art, RISC OS sprites, Thomson)
are recorded here too, since RECOIL rejects every file. Their evidence is a
reference render from another tool or program: a permissively licensed decoder run
as a black box (e.g. Deark), an emulator, or the original program. Say which one and
whether it matched pixel for pixel; after visual review only, say that.

Only add an entry after reviewing our output, with independent evidence: a spec
reference, a render from an emulator or the original program, or a clear
explanation of RECOIL's error. Never add one just to make a failure go away. The
oracle's failure message prints `ours: WxH HASH` for copying.

Columns (tab-separated), `#` starts a comment line:

    corpus id <TAB> WxH <TAB> fnv1a64 <TAB> evidence
