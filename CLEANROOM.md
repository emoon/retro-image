# Clean-room policy

retro-image decodes the same retro image formats as RECOIL
(<https://recoil.sourceforge.net>), which is GPL-2.0-or-later. retro-image is
MIT-licensed. Translating or paraphrasing GPL code would make retro-image a
derivative work, so every decoder must be written independently.

## Never read

- RECOIL source in any form: the Fusion original, generated C/C++/C#/Java/JS/Python/Swift/TS,
  release tarballs, forks, ports, or patches attached to its bug tracker.
- Any other GPL/LGPL decoder (TRSE, SevenuP, view64, abydos, GrafX2, Altirra/Atari800 source,
  format198x, ...). The full list is in each platform file's "To avoid" section in
  [docs/formats/](docs/formats/README.md).
- Code derived from RECOIL even if permissively licensed (e.g. stb_gemras).
- Code with no licence, or an unverified one. Its prose documentation is fine.

## Allowed

- Public format documentation, hardware manuals, magazine articles, author-written specs.
- Permissively licensed code (MIT/BSD/zlib/Apache/CC0), e.g. Deark, GoDot, CiderPress II.
- Reverse engineering of sample files.
- Running the RECOIL binary (`recoil2png`) as a black box and comparing its output pixels.
  This is what the oracle test does.

## Record provenance

Every decoder module states where its layout knowledge came from (links to docs,
"reverse engineered from samples", or "palette observed from recoil2png output").

## Sample files

Sample images are copyrighted by their artists. They live in `corpus/` (git-ignored)
and are never committed. Tests search it recursively: RECOIL's sample set at the top
level, files collected from public archives under `corpus/extra/<platform group>/`
(each group has a `MANIFEST.tsv` recording where every file came from).
Get RECOIL's sample set with:

```sh
mkdir -p corpus && cd corpus
curl -LO https://recoil.sourceforge.net/examples.zip && unzip examples.zip && rm examples.zip
```
