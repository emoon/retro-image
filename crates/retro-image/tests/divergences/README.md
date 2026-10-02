# Divergences from RECOIL

RECOIL is the baseline, not the definition of correct: when it crashes on,
rejects, or misrenders a valid file, retro-image decodes it properly and the
file is recorded here. The oracle test then checks our output against the
recorded fingerprint instead of RECOIL's.

One `.tsv` file per platform group (`amiga-apple-misc.tsv`, `atari8.tsv`,
`atari-st.tsv`, `commodore.tsv`, `msx-japanese.tsv`, `sinclair-cpc-misc.tsv`), so
work on different platforms doesn't conflict.

Only add an entry after reviewing our output, with independent evidence: a spec
reference, a render from an emulator or the original program, or a clear
explanation of RECOIL's error. Never add one just to make a failure go away. The
oracle's failure message prints `ours: WxH HASH` for copying.

Columns (tab-separated), `#` starts a comment line:

    corpus id <TAB> WxH <TAB> fnv1a64 <TAB> evidence
