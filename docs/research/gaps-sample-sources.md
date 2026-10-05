# Sample sources: where to find real files for retro image formats, and how to mine them

Cross-cutting research for the corpus (`corpus/`, git-ignored, never redistributed; see
[CLEANROOM.md](../../CLEANROOM.md)). The format-gap surveys say which formats lack samples; this note says
where files come from and how to pull them without hurting anyone's server. Surveyed 2026-10-05.
No RECOIL source and no GPL/LGPL decoder source was read. Every tool below is either my own script
(stdlib Python or shell, listed in the appendix) or an off-the-shelf program that was only run, never read.

Confidence tags: **(tested)** I ran it this session and saw the result; **(fetched)** I fetched the
page or listing and read it; **(snippet)** a search snippet only; **(memory)** from background
knowledge, not checked.

## 0. What matters most

1. Wayback `id_` URLs plus the CDX API work and are enough to mine whole dead sites by extension
   (tested). Hosts whose DNS no longer resolves or that refuse connections were enumerated (and sampled for the first, third and fourth): `garbo.uwasa.fi` (30,564
   non-HTML URLs, 11,599 zips), `wuarchive.wustl.edu` (`/graphics` 6,273, `/systems/atari` 909,
   `/systems/mac` 9,935, `/systems/amiga` 38,638), `www.amstrad.eu` (the CPCScene FTP mirror: 1,244 URLs)
   and `hp.vector.co.jp` (13,577 `.lzh`/`.zip`). Full counts in 2.3.
2. archive.org lets you pull one file out of a zip or ISO without downloading the container
   (tested): `https://archive.org/download/<id>/<big.zip>/<inner/path>` (also for `.iso`; one
   level only). Combined with a small Range-request zip lister (appendix), a 15 GB TOSEC zip can be
   listed with 5 requests and one 100 KB inner zip fetched.
3. Four JSON/index sources give direct file URLs by platform, no scraping: the ZXArt API (19,507
   Spectrum-family pictures in 34 type/extension combinations, including `.sxg`, `.hrg`, `.specscii`, `.ss4`), the Demozoo
   API (per-platform graphics lists), the 16colo.rs API (about 5,500 ANSI/ASCII packs) and Aminet's
   `INDEX` (78,876 packages, 11,889 of them `pix/`) (tested).
4. `ggnkua/Atari_ST_Sources` holds pictures we have not used: 185,404 files including
   2,452 `.PI1`, 1,758 `.IMG`, 1,147 `.ANI`, 817 `.PC1`, 691 `.NEO`, 97 `.TNY`. A blobless sparse clone
   fetched all 691 `.NEO` files in about 5 seconds (tested). Licence: none stated.
5. Atari ST TOSEC zip → Floppyshop picture disk → FAT12 → 16 of 19 files accepted by the current
   `retro-image` binary (tested end to end). 345 more Floppyshop Picture/Clip Art disks are in the
   same zip.
6. cd.textfiles.com is bigger than its use so far: 520 CDs, 4.43 million files, 504 GB, including
   several Atari ST CDs (`suzybatari1/2`, `crawlycrypt1/2`, `atarilibrary`, `806atari`), Amiga coverdisc
   sets (461,581 + 173,620 files) and a Japanese Fujitsu collection (fetched).
7. rsync works from the sandbox for `ftp.funet.fi`, `ftp.scene.org` (1.5 TB, `graphics/` alone is
   8,704 files) and `ftp.sunet.se` (tested). Prefer it over HTTP crawling for bulk.
8. Not yet mined in the C64 archive: `CSDB_discmags` (485 MB zip, 9,617 members, 5,094 `.d64`),
   `CSDB_c128`, `CSDB_bbs` (tested listing). Previous waves took graphics, tools, demos, misc.
9. Gotchas that cost me time: the CDX `digest` is not always the SHA-1 of the payload on 2000-era
   captures; `statuscode:200` alone lets HTML directory listings through; domain-wide regex filters on
   huge hosts time out; DOS-era zips (shrink/implode) defeat Python's `zipfile` but not Info-ZIP `unzip`;
   several hosts I assumed dead still answer (see 2.3); Wayback answers 503 or refuses connections when
   several agents share one IP. Sections 2.5 and 2.6.

## 1. Method and caveats

- Everything was fetched from the research sandbox on 2026-10-05. Several other agents were active on
  the same outbound IP, which is probably why Wayback returned 503 "Temporarily Offline" on about one early CDX call in four.
- Roughly 70 MB came over the wire (Aminet `INDEX` 7 MB, ZXArt JSON 12 MB, an ISO listing 6 MB, a sparse
  clone 38 MB, small stuff). That is above the 50 MB I was asked to stay under, mostly the clone, which I deleted.
  Sample files themselves were a few MB.
- Nothing was written into the repo's `corpus/`. All scratch work is in the session scratchpad. (The
  scratchpad `dl/` directory is shared with other agents; I did not touch their files.)
- My own test requests carried a `User-Agent` that included the git commit address. That was my mistake;
  the scripts in the appendix now read a contact string from `CORPUS_CONTACT` and default to none.
- Tools run as black boxes: `curl`, `jq`, `gh`, `git`, `rsync` (GPL, only executed), `unzip`, `lha` (this
  machine's `lha` is Lhasa 0.3.1, ISC). Everything else is stdlib Python.
- "Present in corpus" figures compare file basenames against `corpus/`; a renamed file counts as absent.

## 2. Wayback Machine

### 2.1 CDX API

Endpoint: `https://web.archive.org/cdx/search/cdx?url=<pattern>&...`. Parameters I used, all (tested):

| Parameter | Use | Result seen |
|---|---|---|
| `url=host/path/*` or `matchType=prefix` | everything under a path | `www.devrs.com/` → 1,092 URLs |
| `matchType=domain` | host plus subdomains | `nesdev.parodius.com` domain → 7 index pages |
| `matchType=host` / `exact` | one host / one URL | exact URL gave its capture list |
| `output=json` | JSON array; first row is the header | |
| `fl=timestamp,original,mimetype,statuscode,digest,length` | pick columns | |
| `filter=statuscode:200` | drop 404/302 captures. `statuscode` is `-` for `warc/revisit` duplicates, so this also drops those | funet: 404 and 403 rows vanish |
| `filter=!mimetype:text/html` | drop HTML, which includes FTP/Apache directory listings stored as HTML | needed on funet, atari.org |
| `filter=original:(?i).*\.(neo\|pi1\|zip)(\?.*)?$` | regex on any field, case-insensitive with `(?i)` | worked; repeat `filter=` to AND |
| `collapse=urlkey` | one row per URL (applied after filters, so you get the first 200 capture) | |
| `collapse=digest` | one row per distinct payload digest | `c3.zip` with `from=2005&to=2012` gave 1 row |
| `from=` / `to=` | year or timestamp range | |
| `limit=N`, `limit=-N` | first N or last N rows | |
| `showNumPages=true`, `page=K` | split a huge result; each page takes 1-10 s | `geocities.com/SiliconValley/` = 962 pages |
| `showResumeKey=true`, `resumeKey=` | cursor pagination; the key is the last row after an empty row | |

Related endpoints (tested): `https://archive.org/wayback/available?url=...&timestamp=2007` (closest
capture as JSON) and `https://web.archive.org/web/timemap/json?url=...&fl=...` (all captures of one URL).

Notes on fields:

- `length` is the compressed record size in the WARC, not the payload size. For `c3.zip` the CDX said
  35,583 and the payload was 35,221 bytes. Use it to skip outliers (one list had 977 MB entries), not to predict size.
- `digest` is base32 SHA-1. On most captures it equals the SHA-1 of the payload, so you can hash local files
  the same way and skip captures you already hold. On 7 of 16 captures from May to July 2000 it did not
  match, although the bodies were valid zips (see 2.6).
- Original URLs keep case and `:80` ports (`http://www.devrs.com:80/gb/files/c3.zip`); `urlkey` is lower-cased.

### 2.2 Raw bytes with `id_`

`https://web.archive.org/web/<timestamp>id_/<original-url>` returns the stored bytes with no toolbar and no
link rewriting (tested):

| URL form | `c3.zip` (binary) | `about.html` |
|---|---|---|
| `<ts>id_/` | 35,221 B, md5 `e0f134ef...` | 1,638 B, clean |
| `<ts>/` (default replay) | identical | 3,272 B, contains `wombat`/`web-static` scripts |
| `<ts>oe_/` | identical | not tested |

So `id_` is mandatory for anything that might be HTML and harmless for the rest. A timestamp without an exact
capture answers `302` to the nearest one; use `curl -L` (tested). Download endpoints (`/download/12/`)
keep their `Content-Disposition` header on replay, which gives the real file name
(`romhacking.net/download/utilities/10/` → `ipsmaker.zip`) (tested).

### 2.3 Enumerating dead sites: counts I actually got

Command shape: `wbenum.py HOST/ prefix limit=100000 "filter=!mimetype:text/html" "filter=original:(?i).*\.(zip|lzh)$"`
(appendix). Extension histograms come from `extsum.py` (10 lines). "Dead" below means the name no longer
resolves or refuses connections when fetched directly; I checked this after the enumerations, because
several hosts I took for dead are still up.

Hosts that are dead (tested: `curl` exit 6 = no DNS, or connection refused):

| Host | Query | Rows | Breakdown | Time |
|---|---|---|---|---|
| `garbo.uwasa.fi` (Garbo DOS archive; archive.org also holds copies) | non-HTML | 30,564 | `.zip` 11,599, `.gif` 3,344, `.zoo` 253, `.z` 770, plus Linux and Windows trees; 20,844 distinct digests; `/pub/pc/screen` 107, `/pub/pc/gif*` dirs | 7 s |
| `wuarchive.wustl.edu` (345 CDX pages) | non-HTML, per tree | `/graphics` 6,273; `/systems/atari` 909; `/systems/amiga` 38,638; `/systems/mac` 9,935 | graphics: `.jpg` 1,156, `.z` 989, `.gif` 413, `.zip` 371, `.tga` 74; atari: `.arc` 388, `.zoo` 90, `.lzh` 87, `.zip` 28; amiga: `.readme` 35,589 but `.lha` only 1,219; mac: `.hqx` 8,564 | not timed |
| `www.amstrad.eu` (98 pages) | non-HTML | hit the 200,000 cap; mostly forum `.jpg` 113,673 and `.png` 74,576 | the CPCScene FTP mirror under `/uploads/fichiers/ftp_cpcscene/`: 1,244 URLs (`.pdf` 719, `.m4v` 222, `.zip` 188, `.dsk` 9; folders `Slide-Shows` 46, `Compils` 10, `Cracktros` 13, `Mags` 56) | 28 s |
| `hp.vector.co.jp` | `.lzh .zip .mag .pi ...` non-HTML | 13,577 | `.lzh` 9,008, `.zip` 4,564; 13,473 distinct digests | 118 s |
| `ftp.nvg.unit.no` | non-HTML | 510 | `.gif` 300, `.jpg` 88, `.zip` 66 | not timed |
| `sam.speccy.org`, `oric.ch`, `sharpx1.com`, `atari.archive.umich.edu` | `showNumPages` / `robots.txt` only | 1 CDX page each (umich: one `robots.txt`) | not worth enumerating | |

Hosts I enumerated first and then found to be alive or redirecting (tested):

| Host | Now | Rows | Breakdown |
|---|---|---|---|
| `nesdev.parodius.com` | redirects to `www.nesdev.org`; `tlp10.zip` still downloads there | 13,873 (all captures, 200) | `.php` 9,621, `.pl` 3,503, `.zip` 300, `.gif` 138, `.txt` 122, `.ned` 44; 9,896 distinct digests |
| `www.devrs.com` | live; `c3.zip` downloads | 1,092 | `.zip` 210, `.gif` 178, `.txt` 88, `.php` 87 |
| `noname.c64.org` | live (small page) | 52,395 non-HTML | `.php` 41,462, `.zip` 1,540, `.d64` 604, `.prg` 447, `.t64` 381, `.gz` 249, `.rar` 201 |
| `www.atari.org` | live | 31,757 | `.cgi` 28,403, `.php3` 2,437, `.zip` 39 (system disks) |
| `ftp.vector.co.jp` | HTTP 200 with an empty body | 23,823 | `.lzh` 16,822, `.zip` 7,001; `/pack/dos` 923, `/pack/x68` 40; some entries near 1 GB (21 s) |
| `www.atarimuseum.com` | answers | 3,920 non-HTML | `.jpg` 2,447, `.gif` 868, `.pdf` 179, `.zip` 92 |
| `romhacking.net/download/utilities/` | live but 403 to `curl` | 1,501 | zip 650, rar 140, 7z 61, html 604 |
| `mdfs.net/Software/` | live but 403 to `curl` | 2,005 non-HTML | `.gif` 707, `.zip` 118, `.ssd` 24, `.tap` 26 |
| `ftp.funet.fi/pub/<d>/` | live (rsync, see 3.2) | amiga 1,523 / atari 693 / cbm 4,957 / msx 540 / graphics 210 | Wayback is sparse; use rsync |
| `wuarchive.wustl.edu/pub/aminet/pix/` | dead host | 7,789 | `.jpg` 4,253, `.readme` 3,329, `.lha` 113: previews, not the archives |
| `geocities.com` | landing page only | domain-wide regex: 0 rows, timed out at 280 s; `geocities.com/SiliconValley/` pages 100 / 500 / 900 gave 21 / 130 / 46 zip rows in 0.8 / 9.8 / 5.9 s | 962 pages in all, so about 25 minutes for a full scan |

Downloads made from dead hosts (tested): 3 small `.lzh` from `hp.vector.co.jp`, 6 CPC slide-show zips from
`amstrad.eu` (each holds one `.dsk` disk image), 3 small zips from `garbo.uwasa.fi` (`.com`, `.asm`). All opened
as valid archives. The Wayback `.lzh` held `.ico` and map files, and running `retro-image` on them accepted
14 files (the `.ico`) and rejected 2 (`.map`). The CPC `.dsk` files and DOS `.com` files are counted by
`inventory.py` as unregistered extensions: the CPC slide shows need a DSK reader before their `.scr` files
can be tried. Extra downloads from the live-site captures (16 nesdev zips, 1 romhacking utility, 1 devrs zip)
were only to exercise the mechanics.

Names only, contents not inspected (tested listing): `nesdev.parodius.com` has `tilemolester-0.16.zip`,
`tlp10.zip` (Tile Layer Pro), `chr2nam.zip`, `raw2chr.zip`, `CHR_Creator.zip`, `bmp2nes.zip`, `sprite.zip`.
These are tools, so they matter for producing sample files under Wine, not as samples.

### 2.4 From listing to download plan

1. Enumerate with the non-HTML and extension filters; keep the TSV (it is the audit trail).
2. Drop rows whose digest is in the corpus digest list (`corpus_digests.py`; 4,829 corpus files hash in
   0.3 s, 4,738 distinct, so 91 corpus files are already duplicates).
3. Sort by `length` and cap (for example 20 MB per file, 500 MB per site), then download oldest-first or latest-first.
4. Download with `wbfetch.py` (1.2 s apart, retries, `id_`, HTML-wrap check, body SHA-1 recorded).
5. Unpack and test with `inventory.py` (it runs the repo's own `retro-image` over registered extensions
   and extensionless files; the rejects are the interesting ones).

### 2.5 Rate limits and politeness

What I saw (tested): CDX returned `503` with a "Temporarily Offline" HTML page on about one in four of my
early calls, with calls at least 1 s apart while other agents were also querying. Once a replay request got
`Connection refused` after a run of 16 downloads 1.2 s apart; a retry shortly afterwards worked. Doubling
back-off (5, 10, 20, 40 s) always recovered. `x-rl`/`x-na` response headers are present but I did not decode them.

Rules I would keep: one request per second overall, one Wayback job at a time across all agents sharing an
IP, back off on 429/5xx and "connection refused", never parallelise, cache every listing, identify yourself
in the `User-Agent`, and do not retry a 404. Published limits were not checked (memory): I recall IA
discouraging more than about one request per second on replay and a stricter CDX limit.

### 2.6 Gotchas

| Gotcha | Evidence | Remedy |
|---|---|---|
| `statuscode:200` still returns `text/html` for URLs that end in `.zip` (FTP listings, error pages, Simtel `?0 0 x.zip` pages) | funet: 9,184 of 18,588 rows were `text/html` | add `filter=!mimetype:text/html`; check magic after download |
| CDX digest ≠ payload SHA-1 on some old captures | 7 of 16 nesdev zips from 2000; bodies valid | hash the body yourself; treat digest as a hint |
| Default replay wraps HTML | `about.html` 3,272 B vs 1,638 B | always `id_` |
| Huge entries | `ftp.vector.co.jp` list has 977 MB records | filter on `length` locally |
| Domain-wide regex on a huge host times out | geocities, 280 s, zero rows | restrict the path prefix, or use `showNumPages` and scan `page=K` |
| URL ends in `/` or has a query | local name would be `index` | use `Content-Disposition`; `wbfetch.py` does |
| Shift-JIS file names inside `.lzh` | `lha x` wrote raw bytes (`�_�ˉ�.cal`) | rename in a second pass; `inventory.py` only sanitises zip members |
| Mixed-case and 8.3 names | `13.Ico`, `04.Ico` next to `03.ico`; FAT12 extraction gives names like `AAAAAAAA.ART` | lower-case the extension before matching; sniff by extension and signature, not by base name |
| Zip-slip paths | a test zip with `../evil/AGONY.IFF` | flatten member paths (done in `inventory.py`) |
| Extensionless Amiga and Mac files | `56Chevy` (IFF HAM) is accepted by signature only | try extensionless files too |
| DOS-era zips use shrink/reduce/implode | Garbo `colorize.zip`: Python `zipfile` says "compression method is not supported" | fall back to Info-ZIP `unzip` (done in `inventory.py`) |
| The "dead" site may still serve the file | `devrs.com/gb/files/c3.zip` and `nesdev.parodius.com/tlp10.zip` (redirect) download live | `curl -I` the original URL before queueing Wayback fetches |
| Redirect captures | `statuscode:200` drops 301/302 rows; a file served through a redirect has its capture under the target URL (memory) | enumerate the target path too, or drop the status filter and check `statuscode` in the TSV |
| Revisit rows | `statuscode` `-` with the same digest | `statuscode:200` already removes them |
| Robots exclusions | server timing header shows `exclusion.robots` | an excluded site simply returns nothing; do not look for a way round (memory) |

## 3. Mechanics that beat Wayback when they apply

### 3.1 archive.org

- Search API (tested): `https://archive.org/advancedsearch.php?q=<lucene>&fl[]=identifier&fl[]=item_size&rows=N&sort[]=downloads desc&output=json`.
  `mediatype:software` plus `title:(...)` works and `collection:softwarelibrary_c64` works. My `format:` queries
  returned 0 rows, so I used title and collection queries. File formats in metadata are named "ZIP", "ISO Image",
  "Storage Media Image", "7z".
- File lists: `https://archive.org/metadata/<id>` returns every file with name, format and size (tested).
- Inner-archive access (tested):
  - `https://archive.org/download/<id>/<file.zip>/` (trailing slash) redirects to `view_archive.php` and returns an
    HTML table of members; `.../<file.zip>/<inner/path>` returns that member. 901,120 bytes of ADF came out of a
    228 KB zip, and a 4 KB `.lha` out of an 18,801-file Aminet ISO, without fetching the container.
  - Works for `.zip` and `.iso`. A `.7z` listing worked but showed only its `.bin`/`.cue`. Zip inside zip is
    one level only: asking for `outer.zip/inner.zip/file` returned 404, so fetch the inner zip and open it locally.
  - `Range` requests on the inner file are ignored (the whole member comes back).
  - The listing page for a big ISO is large (5.9 MB for 18,801 files); save it once.
- Remote zip listing with Range (tested): `rzip.py list URL` reads the central directory in 4-5 requests.
  Worked on archive.org (15 GB Atari ST TOSEC zip: 32,818 members, 5 requests; 508 MB Assembly64 discmags zip:
  9,617 members, 4 requests) and on `hackerswithstyle.se`.
- TOSEC sets: one big zip per platform, built as zip of zips (an inner zip per tape/disk image). Sizes per platform are in section 4.
- Collections (tested), item counts: `softwarelibrary_c64` 99,557; `softwarelibrary_apple` 42,469;
  `softwarelibrary_msdos` 23,258; `softwarelibrary_atari` 15,570; `softwarelibrary_amiga` 13,204;
  `softwarelibrary_zx_spectrum` 12,305; `softwarelibrary_cpc` 3,809; `softwarelibrary_mac` 481;
  `cdromimages` 38,690; `softwarecapsules` 25,345. `softwarelibrary_msx`, `_pc98`, `_x68000`, `_atari_st` and about twenty other guesses returned 0.
- No full-text search over file names inside items; use the metadata API or the zip/ISO listings.

### 3.2 rsync mirrors (tested)

```sh
rsync --list-only rsync://ftp.scene.org/ftp/graphics/          # 8,704 files, 4.9 GB, recursive with -r
rsync --list-only rsync://ftp.funet.fi/ftp/pub/                # amiga atari cbm cpm graphics mac msdos msx pics ...
rsync -r --list-only rsync://ftp.funet.fi/ftp/pub/atari/ > atari.lst
rsync -av --include='*/' --include='*.neo' --exclude='*' rsync://host/module/path/ ./out/   # fetch by pattern (syntax not run)
```

funet recursive listings: `pub/atari` 2,473 files, 556 MB (`.zip` 719, `.lzh` 680, `.zoo` 283, `.arc` 173,
only 25 in `falcon/graphics`); `pub/mac` 13,573 files (Info-Mac mirror, 9,290 `.hqx`, 262 in `info-mac/art`);
`pub/graphics` 2,163; `pub/pics` 23,755 (railways photos, not retro); `pub/msx`, `pub/cbm`, `pub/cpm` are
nearly empty now (`ftp.funet.fi/pub/cbm/graphics/` redirects to zimmers.net). `ftp.scene.org` modules: `ftp` and `mirrors`; `graphics/` also holds 1,892 `.diz` and 4,090 `.txt` files, which are
the short descriptions that go with the zips.
`ftp.sunet.se` has only `pub` and `mirror` modules, no retro content.

### 3.3 GitHub

- Blobless sparse clone by extension (tested):
  ```sh
  git clone -q --filter=blob:none --no-checkout --depth 1 https://github.com/ggnkua/Atari_ST_Sources.git st
  cd st && git sparse-checkout init --no-cone && git sparse-checkout set '*.NEO' '*.neo' && git checkout -q
  git ls-tree -r --name-only HEAD | awk -F. 'NF>1{print tolower($NF)}' | sort | uniq -c | sort -rn   # histogram, no blobs needed
  ```
  3.7 s for the tree-only clone (5.7 MB `.git`), 1.7 s to check out 692 files (23 MB).
- Tree API `gh api repos/O/R/git/trees/HEAD?recursive=1` is cut off at 100,000 entries for big repos
  (`truncated: true`; it showed 226 `.neo` where the real count is 691). Use `git ls-tree` for those.
- Code search `gh api -X GET search/code -f q='extension:X'` works with the stored token but is noisy:
  extension collisions (`.gbr` = Gerber 317,440 hits, `.nam`, `.chr`, `.spd`, `.ctm`), only 12 `.pi1`, 5 `.ilbm`.
  Add `path:` or `repo:`; treat counts as meaningless. The limit is about 10 requests a minute (memory).
- Licence field via `gh api repos/O/R --jq .license.spdx_id`: GBTD_GBMB MIT, SGDK MIT, SuperFamiconv MIT,
  cc65 Zlib, gbdk-2020 NOASSERTION, grit GPL-2.0 (do not read), awesome-gbdev GPL-3.0 (a list of links; use facts only).
- Examples from tree listings (tested): `cc65/cc65` `samples/apple2/` has 7 `.hgr` and 6 `.dhgr` pictures;
  `gbdk-2020` has 11 `.gbr` and 5 `.gbm` (already in our corpus); `SGDK` is 4,361 files, 3,341 of them `.png`;
  `GBTD_GBMB` is the Pascal source of the tool, with no sample project.

### 3.4 Site APIs and index files

| Source | Access | What it gives | Status |
|---|---|---|---|
| ZXArt | `https://zxart.ee/api/types:zxPicture/export:zxPicture/language:eng/start:N/limit:2000/order:date,desc` | 19,507 pictures with `type`, `year`, and `originalUrl` = direct file; 23 have none | (tested), 2 files downloaded |
| Demozoo | `https://demozoo.org/api/v1/productions/?supertype=graphics&platform=<id>&limit=N`; detail `/productions/<id>/` has `download_links` | per-platform lists; downloads are mostly `files.scene.org` or party packs | (tested) |
| 16colo.rs | `https://api.16colo.rs/v1/year`, `/v1/year/1996?rows=50` → `download` = `https://16colo.rs/archive/<year>/<pack>.zip` | 5,485 packs in all, 50 per page | (tested) index and result shape; no pack zip fetched |
| CSDb | `https://csdb.dk/webservice/?type=release&id=93314&depth=2` | XML with release metadata | (tested); download links not checked |
| Pouet | `https://api.pouet.net/v1/prod/?id=1`, `/v1/front-page/latest-added/` | JSON prod records | (tested); download links not checked |
| Aminet | `https://aminet.net/INDEX` (7.2 MB) and `https://aminet.net/<dir>/<file>.lha` plus `.readme` | 78,876 packages: file, dir, size, age, description | (tested) |
| Apple II asimov mirror | `https://mirrors.apple2.org.za/ftp.apple.asimov.net/site_files.txt` (2.5 MB) | 39,663 paths; `images/` 30,292 | (tested) |
| cd.textfiles.com | `http://cd.textfiles.com/directory.html` plus per-CD folders with `.files`/`.size` | 520 CDs: name, year, file count, KB | (tested) |
| Assembly64 mirror | `https://hackerswithstyle.se/artifacts/<cat>/<name>.zip` (Range works) | CSDb, discmags, c128, bbs, games | (tested) |
| sembiance.com | `https://sembiance.com/fileFormatSamples/image/<dir>/` | 856 format folders | (tested) |
| FFmpeg samples | `https://samples.ffmpeg.org/image-samples/` | already used (Pictor) | (fetched) |

Aminet detail from the INDEX (tested): `pix/` 11,889 packages, 6.8 GB (`pix/trace` 1,505, `pix/misc` 1,270,
`pix/anim` 726, `pix/wb` 659, `pix/art` 554, `pix/3dobj` 531, `pix/map` 508). Format keywords in `pix/` descriptions:
HAM6 134, HAM8 35, DCTV 10, HAM-E 1, 24-bit 115, ANIM 466, AGA 64; no hits for SHAM, DynamicHiRes, PCHG,
DigiView by description. Each package has a `.readme` with `Short:`, `Author:`, `Type:` and often a distribution
line; a sample HAM6 320x400 file came out as a 640x400 PNG from our binary.

Demozoo graphics counts per platform (tested): C64 7,323; ZX Spectrum 5,234; Atari ST/E 3,787; Amiga OCS/ECS 2,561;
MS-DOS 1,627; Atari 8-bit 1,440; Amiga AGA 206; Amstrad CPC 191; Atari Falcon 159; C16/Plus4 82;
BK-0010 40; NES 32; ZX81 31; MSX 28; Vector-06C 18; Thomson 18; Game Boy 18; macOS 18; VIC-20 16; BBC 14;
Apple II 13; SAM 13; Amstrad Plus 12; Atari TT 11; SNES 10; GBC 8; Sharp MZ-700 7; Mega Drive 6; Master System 5;
Lynx 3; GBA 3; Oric 3; PET 2; Jaguar 2; Game Gear 2; QL 1; Archimedes 1; MZ-800 1; Classic Mac OS, Atari Portfolio, ColecoVision 0.
Platforms with zero graphics: X68000, TRS-80, Apple IIGS, C128, PC Engine, Neo Geo, Neo Geo Pocket, Enterprise, Wonderswan.
The 94 platform names include none for PC-98, PC-88, FM Towns, Dragon, CoCo or TI-99.

ZXArt picture variants (file extension, count) (tested): `.scr` 16,468 (standard) plus 80 Timex81, 34 ULA+,
18 TimexHR, 12 monochrome, 10 flash; `.atr` 1,423; `.sca` 295; `.ss4` 285; `.img` 255 (gigascreen); `.ch$` 113;
`.bsc` 72; `.bmp` 55 (ZX Evo); `.sxg` 50; `.nxi` 42; `.mg2` 42; `.specscii` 35; `.sl2` 34; `.3` 33 (tricolor);
`.mlt` 32; `.stl` 30; `.grf` 11; `.mg4` 10; `.mg8` 9; `.hlr` 7; `.s81` 6; `.hrg` 5; `.sam3` 4; `.ifl` 4; `.mg1` 4;
`.mc` 2; `.bsp` 2; `.lce` 1; `.bmc4` 1. A downloaded SAM Mode 4 `.ss4` decoded with our binary. A `.specscii`
(1,846 bytes) is rejected as "unknown format" because we register no such extension; the Spectrum survey
says true SpecSCII samples were missing, and this is 35 of them.

## 4. Targets by platform family

Status key: (fetched) page or listing read this session; (tested) I downloaded or ran something; "live" means the host answered.

### 4.1 Amiga

| Source | Content | Count / size | Access | Notes |
|---|---|---|---|---|
| Aminet `pix/` | IFF pictures and animations in `.lha` | 11,889 packages, 6.8 GB | HTTP + `INDEX` (tested) | Per-package readme gives artist and (often) distribution terms. Official site, no mirror needed |
| Fred Fish disks | 1,000 library disks as zips of `.adf` | item `commodore-amiga-collections-fred-fish`, 438 MB | archive.org inner path (tested), ADF came out | Disks are programs and docs, pictures only incidentally |
| Aminet CD sets | `aminet-set-1` ... `-9`, `aminet-52`, Walnut Creek #1, `aminet-games` | 0.6-2.6 GB each | ISO browse (tested) on set 1 disc 1: 18,801 files, 332 under `Aminet/gfx/` | 1995 content only |
| cd.textfiles.com | `Amiga Format CD Collection` 461,581 files; `Amiga ACS Coverdiscs` 173,620; `AmigaPlus` 66,373; `Zoom Release 2` 45,734; `Amiga Tools CD`; `Aminet Archive CD` 9,176; `Collection of Images from Various Amiga BBSes` 288 | see names | HTTP folders (fetched) | Best untried bulk source for pictures |
| TOSEC Amiga | `Commodore_Amiga_TOSEC_2012_04_10` 32 GB | | zip of zips | disk images; needs an OFS/FFS reader |
| scene.org graphics | artists, groups, compos | 8,704 files, 4.9 GB (2,099 zip, 133 lha) | rsync (tested) | Mixed platforms |
| Demozoo | Amiga OCS/ECS 2,561 + AGA 206 graphics | | API (tested) | Often a party pack zip |
| `ftp.funet.fi/pub/amiga` | Wayback shows 1,523 non-HTML captures (`.dms` 434, `.lha` 167); the live tree answers over HTTP and rsync | | rsync (listing of the top level only) | Games and demos by the look of it |
| Amiga Graphics Archive (`amiga.lychesis.net`) | game screenshots (not IFF) (memory) | | live | Probably not useful for decoders |

Variant leads from the Aminet index (tested): DCTV (10 in `pix/`, for example `pix/anim/VaseDCTV.lha`,
`TrainToy_DCTV.lha`), HAM8 (35), HAM-E (`pix/misc/Nishi.lha`), ANIM (466), multipalette (`gfx/conv/mp2iff24.lha`
is a tool). RGBN/RGB8 and DigiView have no useful hits (the earlier note stands: Aminet has only the datatype).
Sculpt and Imagine are 3D object formats; Aminet has `pix/3dobj` (531), `pix/imagi` (311) and `pix/real3` (206) folders, which I only counted by name.

### 4.2 Atari ST, 8-bit and TOSEC

| Source | Content | Count / size | Access | Notes |
|---|---|---|---|---|
| `ggnkua/Atari_ST_Sources` | source trees with pictures | 185,404 files: `.PI1` 2,452, `.IMG` 1,758, `.ANI` 1,147, `.PC1` 817, `.MBK` 723, `.NEO` 691, `.TGA` 314, `.IFF` 226, `.SPR` 190, `.PAC` 136, `.TNY` 97, `.PI3` 91, `.SCR` 78, `.APX` 74, `.SPC` 42, `.PC3` 42, `.PI2` 37, `.MAC` 34, `.DOO` 33, `.ICE` 25, `.ART` 21, `.SPU` 20, `.CA1` 19 | sparse clone (tested) | Licence none stated; mixed authorship. The `.ANI` count suggests NEOchrome animations |
| TOSEC Atari ST | `Atari_ST_TOSEC_2012_04_23` 14.9 GB | 32,752 inner zips: Compilations 11,504, Collections 8,149, Games 7,084, Demos 2,276, Applications 1,252, Diskmags 977, Coverdisks 902 | Range list + inner path + `fat12.py` (tested) | `Floppyshop Picture` / `Clip Art`: 345 disks. One disk gave 19 files, 16 accepted; the disk is full of 32,512-byte `.ART` files (Art Director) |
| cd.textfiles.com | `suzybatari1` 27,712 files, `suzybatari2` 26,091, `crawlycrypt1` 14,445, `crawlycrypt2` 13,809, `atarilibrary`, `geminiatari`, `806atari` 25,023 (floppy images), `scenepixelg` 4,161 | | HTTP (fetched) | `ataricompendium`, `Atari CD Collection` 133,030 files |
| archive.org | `softwarelibrary_atari` 15,570 items; `MAXONCDPDCOLLECTIONVOL1`; `public-domain-cover-disk-1..6` (Future Publishing); `fantasy_graphics_deltronics` | | search API (tested) | |
| `ftp.funet.fi/pub/atari` | 2,473 files, `falcon/graphics` 25 | | rsync (tested) | `.zoo`, `.arc`, `.lzh`: needs unpackers |
| Demozoo | ST/E 3,787, Falcon 159, TT 11 graphics | | API (tested) | |
| Sembiance | `gemMetafile` 52, `neoDrawDrawing` 11, `gemViewDither` 10, `atariGrafik` 6, ... | | HTTP (fetched) | table in section 8 |
| Atari 8-bit | `ftp.pigwa.net` (used), atarionline.pl (used), TOSEC `Atari_8_bit` 366 MB | | live | already covered by the Atari 8-bit corpus notes |
| Atari Legend, atari-forum | | | Atari Legend live (fetched); atari-forum.com did not connect over HTTPS (inconclusive) | not enumerated |

### 4.3 Commodore (C64, VIC-20, Plus/4, PET)

| Source | Content | Size | Access | Notes |
|---|---|---|---|---|
| Assembly64 mirror (`hackerswithstyle.se/artifacts/`) | `CSDB_graphics`, `_tools`, `_demos`, `_misc` (mined), `_discmags` 485 MB (9,617 members: 5,094 `.d64`, 223 `.prg`, 184 `.t64`, 57 `.d81`), `_c128` 50 MB, `_bbs` 17 MB (915 members), `_games`, `Blast_from_the_past` 49 MB | | zip + Range (tested) | `_discmags`, `_c128` and `_bbs` are not yet mined |
| archive.org `softwarelibrary_c64` | 99,557 items, disk and tape images | | search API (tested) | commercial software too |
| TOSEC C64 | 6.7 GB; Plus/4 172 MB; VIC-20 103 MB; C128 9 MB; PET 2 MB | | zip of zips | |
| `noname.c64.org` (Wayback) | `.d64` 604, `.prg` 447, `.t64` 381, `.zip` 1,540 | 52,395 URLs | `id_` (tested) listing | the host now answers with a small page; the captures hold the old downloads (early CSDb site) |
| `ftp.funet.fi/pub/cbm` (Wayback) | `.prg` 2,125, `.cvt` 384, `.lnx` 153, `.sfx` 250, `.gif` 157 | 4,957 | Wayback only | live `ftp.funet.fi/pub/cbm/graphics/` redirects to zimmers.net, where `anonftp/pub/cbm/graphics/` answers 404 (GEOS albums under `.../geos/graphics/albums/` were used earlier) |
| CSDb webservice, Demozoo | | C64 7,323 graphics on Demozoo | API (tested) | |
| Plus4World, VIC-20 Denial, Lemon64 | | | Plus4World and Denial live; Lemon64 403 to `curl` (fetched) | not enumerated |

Previous waves already concluded the twelve missing editor formats (BDP, ESH, FLM, ...) have no pictures in the
CSDb archives; the unmined folders above are the remaining place to look, with low odds.

### 4.4 ZX Spectrum family, SAM, Timex

| Source | Content | Size | Access | Notes |
|---|---|---|---|---|
| ZXArt API | see 3.4 | 19,507 files | (tested) | the best single source for ZX variants, SAM Mode 4, ZX Next |
| TOSEC `Sinclair_ZX_Spectrum` | 1.6 GB (used for TAP/TZX) | | | |
| TOSEC `Sam_Coupe` | 311 MB, one zip | | listing approach (tested) on other sets | |
| `softwarelibrary_zx_spectrum` | 12,305 items | | search API | |
| Demozoo | ZX 5,234 + ZX Enhanced 68 + ZX81 31 + SAM 13 | | API | |
| Spectrum Computing, Sinclair wiki | | | live (fetched) | |
| World of Spectrum `/pub/sinclair/` | | | 403 to `curl` (fetched); `ftp.nvg.ntnu.no/pub/` answers (not browsed) | |
| worldofsam.org | | | live | |

### 4.5 Amstrad CPC and Acorn/BBC

| Source | Content | Notes |
|---|---|---|
| archive.org `snaps_202510` | 1,470 members (used) | CPC snapshots (tested listing) |
| TOSEC `Amstrad_CPC` 265 MB; `softwarelibrary_cpc` 3,809 items | disk and tape images | zip of zips |
| cpc-power.com, cpcrulez.fr | live (fetched) | not enumerated |
| CPCWiki | 403 to `curl` | use Wayback if needed |
| Demozoo | CPC 191 + Plus 12 | API |
| TOSEC `Acorn_BBC` 219 MB, `Acorn_Archimedes` 68 MB, `Acorn_Electronic` 15 MB | | |
| stairwaytohell.com | live (fetched) | |
| mdfs.net | 403 to `curl`; Wayback has 2,005 non-HTML URLs (`.ssd` 24, `.tap` 26, `.gif` 707) (tested) | Teletext/mode 7 samples come from GitHub (already used) |
| Sembiance | `bbcMicro` 5/5 present, `acornDraw` 10/0, `bbcDisplayRAM` 1/0 | |

### 4.6 MSX, Japanese computers (PC-98, PC-88, X68000, FM Towns, FM-7, X1, MZ)

| Source | Content | Size | Access | Notes |
|---|---|---|---|---|
| `hp.vector.co.jp` (Wayback) | personal software pages, mostly `.lzh` | 13,577 archives | `id_` (tested) | the host refuses connections now; best Japanese hobby-page source found |
| `ftp.vector.co.jp` (Wayback) | `/pack/dos`, `/pack/x68`, `/pack/mac`, `/pack/win95/art` | 23,823 archives | `id_` (tested) | |
| archive.org TOSEC | `NEC_PC_9801` 2.0 GB, `NEC_PC_8801` 602 MB, `NEC_PC_9821` 8 MB, `MSX_MSX2` 614 MB, `MSX_MSX` 162 MB, `MSX_TurboR` 27 MB, `Sharp_X68000` 2.2 GB, `Sharp_X1` 9 MB, `Fujitsu_FM-7` 88 MB, `Fujitsu_FM_Towns` 74 MB, `Sharp_MZ-700` 1 MB | | zip of zips | disk images (D88, XDF, DIM); need readers |
| archive.org | `kawaii-dake-na-no` (used), `chotto-dake-yo-pc98`, `frieve_msxfan`, `MSXFANSuperProkore2`, `04_20230212_202302` (MSX CG programs from Technopolis) | 1-140 MB | metadata API (tested) | |
| archive.org CDs | `MSXMAG1-3` (MSX Magazine Revival, 264-465 MB ISOs; the ISO holds Windows installers: 87 `.exe`), `Nova_OhX1998Spring` ... `OhX2001Spring` (X68000 magazine CDs; the 1999 Spring item is two `.7z` files that each hold a bin/cue) | | | a full download is needed to look inside |
| archive.org large | `X68K_Arquivista` 4.4 GB, `Sharp_X68000_Collection` 17 GB, `pc-98_20230120` 6 GB, `Neo_Kobe_Fujitsu_FM_Towns` 94 GB | | | remote browsing not tested |
| cd.textfiles.com | `fujitsufree` (Fujitsu Free Software Collection, Japanese) 37,351 files, 1.7 GB | | HTTP (fetched) | |
| Sembiance | `fmTownsHEL` 10, `fmTownsIcons` 12, `fmTownsTK4` 7, `townsPaintII` 4, `pc88PI` 17 (4 present), `tim` 17, `tim2` 6 | | | none of the FM Towns folders is in the corpus by name |
| msx.org, generation-msx.nl, `download.file-hunter.com`, msxarchive.nl | live (fetched) | | | not enumerated |
| `vector.co.jp` | live | | | current site; `hp.` is dead, `ftp.` returns an empty page |

### 4.7 Apple II, IIGS, Macintosh

| Source | Content | Size | Access | Notes |
|---|---|---|---|---|
| asimov mirror | `images/gs` 1,868, `images/demos` 75, `pd_collections` 653, `games` 10,091, `educational` 7,001, `productivity` 2,381; `misc` 1,221 | 30,292 images of 39,663 paths | `site_files.txt` (tested) | disk images (`.dsk`, `.po`, `.2mg`); DOS 3.3/ProDOS readers needed. Graphics-looking names: `AAA Art And Graphic 0xx` series, `Apple-Boston ... Graphics #1-6`, `bones_slideshow_*` |
| archive.org | `softwarelibrary_apple` 42,469 items (LOGIC Print Shop disks PS001..PS031, 4am cracks); TOSEC `Apple_2` 240 MB, `Apple_II_GS` 395 MB | | search API | |
| `cc65/cc65` `samples/apple2/` | 7 `.hgr`, 6 `.dhgr` | small | GitHub (tested) | code is Zlib; picture copyright unclear |
| CiderPress II TestData | already used | | | Apache-2.0 |
| Macintosh | `ftp.funet.fi/pub/mac/info-mac` 13,573 files (262 in `art/`); archive.org `info-mac-archive` 5.3 GB, `InfoMac5Disk2.img`, `mac_Paint_2` (MacPaint 1.5 app); `cd.textfiles.com/Roadside Resources` (BMUG, 4,685 files); Sembiance `macPaint` 31 (4 present), `macDraw` 49 (1 present) | | rsync, archive.org | `.hqx` needs a BinHex decoder (`unar` does it) (memory) |
| Wayback of `ftp.vector.co.jp/pack/mac/` | Japanese Mac shareware | | | |

### 4.8 DOS and PC graphics, text mode, demoscene

| Source | Content | Size | Notes |
|---|---|---|---|
| cd.textfiles.com | `vgaspectrum` (VGA Spectrum) 7,211 files, VGA Spectrum II 3,285, `10000gp2` (10000 Graphics Pack 2) 11,036, `scenepixelg` 4,161, GIFs Galore (two CDs), Gif Galaxy, Venus VGA Series, `librisbritannia` (Dr. Halo and CAD clip-art folders, used) | 520 CDs, 504 GB | (fetched); folders carry `.files` counts |
| archive.org | Simtel (50 items, e.g. `Simtel_MSDOS_1996-06`), Garbo (`Garbo`, `2012.11.24.ftp-garbo-mirror`), Walnut Creek (418 items, `GifsGalore_Aug92`, `LibrisBritannia`) | | (tested) search |
| Wayback of `ftp.funet.fi/pub/simtelnet/` | about 11,000 URLs ending in `.zip`, but most were captured as `text/html` (listing pages), so the count says little | | use the live rsync instead |
| `ftp.vector.co.jp/pack/dos/art` | 127 archives | | Wayback |
| 16colo.rs | 5,500 packs (1990-): ANSI, ASCII, XBin, ADF, IDF, TND, PCB | | (tested) API; `16colo.rs` not `16colors.rs` |
| scene.org / Demozoo | MS-DOS graphics 1,627 on Demozoo | | (tested) |
| Sembiance | `pcx` etc.; see section 8 | | |
| FFmpeg samples | Pictor (used) | | |

### 4.9 Other 8-bit and small systems

| System | Sources found | Notes |
|---|---|---|
| TI-99/4A | TOSEC `Texas_Instruments_TI-99_4a` 41 MB (43.6 MB single zip); `www.whtech.com/ftp/` live index; `happy-computer-ti-99-collection` (162 MB) | Sembiance `tiareGRA` 4, `tiCalc`; no Demozoo graphics platform |
| Dragon | TOSEC `Dragon_Data_Dragon` 7 MB: 706 members, 694 inner zips, titles include `Picture Maker`, `Graphic Animator`, `Graphics System` (tested); `archive.worldofdragon.org` live | tape images (`.cas`), so the pictures are inside programs |
| CoCo | TOSEC `Tandy_TRS80_Color_Computer` 21 MB; `colorcomputerarchive.com` live (has an `Images` section); KAOS toolkit repo (MIT, used) | |
| TRS-80 | Sembiance `trs80HR` 64 (3 present), `trs80Star` 12 (9 present), `trs` 12 | |
| Oric | TOSEC `Tangerine_Oric_1_and_Atmos` 26 MB; oric.org live | Demozoo 3 graphics |
| Sharp MZ, X1 | TOSEC `Sharp_MZ-700` 1 MB, `MZ-800` empty, `X1` 9 MB; `sharpmz.org` live | no usable Demozoo or Sembiance set |
| Thomson | dcmoto.free.fr (used) | Demozoo 18 |
| Enterprise, Jupiter Ace, Memotech, Camputers Lynx | TOSEC sets of 116, 52, 47, 0 MB | no picture formats known |
| Calculators (TI-8x/9x) | TOSEC TI-82 18 MB, TI-83 36 MB, TI-89 8 MB, TI-92 13 MB | |

### 4.10 Consoles and handheld tile tools

This family is about tool project files and raw tile dumps (CHR, tilemaps, palettes); commercial ROMs would
be sample files only in a loose sense and should stay out unless a format cannot be tested otherwise.

| Source | Content | Notes |
|---|---|---|
| GitHub homebrew repos, by `git ls-tree` | NES `.nam`/`.chr`/`.nss`, Game Boy `.gbr`/`.gbm`, SNES `.chr`/`.pal`, Genesis tiles | the cheapest route; already yielded `nes-nam` and `gameboy-gbtd-gbc` groups |
| `nesdev.parodius.com` (redirects to `nesdev.org`; Wayback copy works too) | 300 zips of NES tools (Tile Molester 0.16, TLP 1.0, `chr2nam`, `raw2chr`, `CHR_Creator`, `bmp2nes`), plus the old nesdev docs | (tested) enumeration and 16 downloads |
| `forums.nesdev.org/download/file.php?id=N` | forum attachments: `id=100` a zip, `id=20000` a PNG | (tested) 4 ids; `id=5000` and `40000` were 403/404, so do not brute-force; harvest ids from thread pages |
| `www.nesdev.org` wiki | tool list, formats | live (fetched) |
| `www.devrs.com` (still live; Wayback listing) | 210 Game Boy dev zips (`c3.zip`, `gbb210.zip`, `gic.zip`, `apa.zip`, `gftilea9.zip`, ...), 87 `.php` pages | (tested) |
| `gbdev.io`, `awesome-gbdev` | link lists | live; `gbdev.io/list.html` 404 |
| `romhacking.net` | utilities, documents | 403 to `curl` (fetched); Wayback has 1,501 `/download/utilities/<id>/` captures (650 zip, 140 rar, 61 7z) (tested) |
| `www.smspower.org` | SMS/GG docs and tools | `/Development/Index` live; `/Tools/` 404 |
| `wiki.neogeodev.org` | Neo Geo | live (fetched) |
| `www.sega-16.com`, `zophar.net/utilities` | forums, tools | live (fetched) |
| `pcenginefx.com` | did not connect | |
| Atari Lynx / Jaguar | AtariAge forums (memory); Demozoo 3 and 2 graphics | not checked |
| TOSEC console sets | `NEC_PC-Engine` 3 MB, `NEC_SuperGrafx` 7 MB, `Sega_Mark_III_and_Master_System` 107 MB, `Sega_Game_Gear` 177 MB, `Sega_Megadrive_and_Genesis` 2.4 GB, `Super_Famicom_and_SES` 1.9 GB, `Bandai_Wonderswan` 119 MB | ROM sets, not tool files |
| Demozoo | NES 32, GB 18, GBC 8, SNES 10, MD 6, SMS 5, Lynx 3, GBA 3, PCE 0 | |

## 5. Pipeline

### 5.1 Stages

```
enumerate  ->  filter  ->  download  ->  verify  ->  unpack  ->  classify  ->  manifest
 CDX / API     ext, mime,   throttled,   SHA-1,      zip/lha/    retro-image   append rows
 listing       length,      id_ for      magic,      iso/fat12   accept /
               dedupe       Wayback      not HTML                reject
```

### 5.2 Commands I ran (all (tested))

```sh
export CORPUS_CONTACT='https://example.org/your-contact'   # shown in the User-Agent

# 1. enumerate a dead site by extension, then summarise
wbenum.py "hp.vector.co.jp/" prefix limit=60000 "filter=!mimetype:text/html" \
  'filter=original:(?i).*\.(mag|pi|q4|max|mki|lzh|lha|zip|d88|dsk)(\?.*)?$' > hpvector.tsv
python3 extsum.py hpvector.tsv 15

# 2. dedupe against the corpus, then fetch the small ones
python3 corpus_digests.py corpus > seen.txt
awk -F'\t' '$6<200000' hpvector.tsv | sort -t$'\t' -k6,6n > small.tsv      # field 6 = CDX length
wbfetch.py small.tsv dl/hpvector --seen seen.txt --max-files 50 --max-bytes 20000000

# 3. unpack and classify with the repo's own binary
python3 inventory.py dl/hpvector work --cli target/release/retro-image

# 4. a file inside a zip on archive.org, without fetching the zip (list, pick a member, build the URL with urllib.parse.quote)
python3 rzip.py list "https://archive.org/download/Atari_ST_TOSEC_2012_04_23/Atari_ST_TOSEC_2012_04_23.zip" > list.tsv
grep 'Floppyshop Picture' list.tsv | head -2
url=$(python3 -c "import urllib.parse as u; n='Atari ST [TOSEC]/Collections/Atari ST - Collections - Floppyshop (TOSEC-v2011-03-07_CM)/Floppyshop Picture 0165 (19xx)(Floppyshop).zip'; print('https://archive.org/download/Atari_ST_TOSEC_2012_04_23/Atari_ST_TOSEC_2012_04_23.zip/'+u.quote(n))")
curl -L -o disk.zip "$url"                          # 108 KB, one level of nesting only
unzip -o disk.zip -d disk && python3 fat12.py disk/*.st out/

# 5. an ISO on archive.org: list, then pull one file
curl -L "https://archive.org/download/aminet-set-1/<urlencoded .iso name>/" -o iso1.html     # member table
curl -L -o MerlinDCTV_05.lha "https://archive.org/download/aminet-set-1/<urlencoded .iso name>/Aminet/gfx/misc/MerlinDCTV_05.lha"
```

Results: `wbfetch.py` fetched 16 small zips in 24 s and, given a `--seen` list built from them, fetched nothing
on a rerun; `inventory.py` accepted 14 of 16 registered-extension files from the three Wayback `.lzh` and
found 6 CPC `.dsk` and 3 DOS `.com` files in the other downloads; the Floppyshop disk gave 16 accepted of 19
files. Two bugs surfaced and are fixed in the appendix: `fat12.py` failed on a boot sector whose
reserved-sector field is 0 (treat 0 as 1, as TOS does), and an early `wbfetch.py` skipped every file because
it marked the CDX digest as seen before comparing it with the body hash.

### 5.3 Manifest convention

`corpus/extra/<group>/MANIFEST.tsv` exists in two shapes: the older `path, format, source, recoil_size` and
the newer `path, format, source URL, licence, notes`. Use the newer one for anything from here on.
`wbfetch.py` writes it with the format column empty (fill it after `retro-image` classifies the file), the
`web.archive.org/web/<ts>id_/<url>` URL as source, `none stated` as licence, and notes
`sha1-b32 <hash>; <bytes> bytes[; cdx-digest-differs ...]`. For files taken from inside archives, put the
container and member in the source column the way existing rows do (`... .zip : member`). Source rows for
archive.org inner paths should name the item id, the container file and the member path.

### 5.4 Safety checklist

- Cap bytes per site and per run; refuse single files above a limit; log the plan before downloading.
- Never extract with `unzip`/`lha` into the repo; use a work directory, flatten paths (zip-slip), sanitise names.
- Do not execute anything extracted. The test only ever opens files as bytes (the DOS programs and `.exe`
  files in these archives stay inert).
- Do not run the decoder on untrusted files outside a resource limit (`timeout 10`, `ulimit -v`), because some
  inputs will trigger bugs; that is also what the fuzz setup in `fuzz/` is for.
- Keep the listing TSV and the manifest together; they are what lets you re-check provenance later.

## 6. Legal and provenance (short, not legal advice)

- `corpus/` is git-ignored and never redistributed, which is the stance in CLEANROOM.md; keep it that way.
  Do not attach sample files or decoded images to issues, PRs or docs.
- Record where each file came from (URL or container path plus timestamp for Wayback) and its stated licence;
  write `none stated` when there is none. A missing licence means all rights reserved, not public domain.
- Prefer sources that publish files for preservation or sharing (Aminet, scene.org, Demozoo, CSDb, 16colo.rs,
  the repos above). Archives that mirror commercial software (TOSEC, `softwarelibrary_*`, cd.textfiles.com
  shovelware CDs) are acceptable as local test inputs; do not redistribute or quote from them.
- Respect `robots.txt` and the archive's rate expectations; Wayback honours site exclusions, and so should we.
  Do not use rotating addresses or other tricks around 403 or 503 responses (romhacking.net, CPCWiki,
  Lemon64 and WoS answer 403 to `curl`; use their Wayback copies or ask the site).
- Some art disks (PC-98 CG collections, Atari ST art disks, the 16colors packs) contain adult material.
  Keep that out of anything shown to other people.
- Code or data copied from a permissive repository carries its licence text (see the existing memory note
  on permissive licence notices). Sample files copied only into the local corpus do not, but their licence still goes in the manifest.
- Running GPL programs such as `rsync` as black boxes is not a clean-room issue; the rule is about reading
  decoder source. If tooling must stay strictly permissive, the `rsync` steps can be replaced with HTTP listings.

## 7. Where to find samples, by format family (prioritised)

Rank 1 is the first place to look. Counts and status are from sections 2-4.

| Format family | Rank 1 | Rank 2 | Rank 3 | Verified? |
|---|---|---|---|---|
| NES tile dumps, nametables, NESST/NEXXT projects | GitHub homebrew repos via `git ls-tree` | `nesdev.parodius.com` Wayback tools and docs; nesdev forum attachments | Demozoo NES (32) | the `nes-nam` group came from repos in an earlier wave; the tree/sparse mechanism is (tested) on the ST repo; Wayback (tested); attachments partly |
| SNES `.chr`/`.cgx`/`.pal`, Tile Molester projects | GitHub (SNES homebrew, hack repos) (memory) | romhacking.net utilities via Wayback (1,501 URLs) | Demozoo SNES (10) | listing (tested); contents not inspected |
| Mega Drive / Genesis | SGDK tree (3,341 `.png`, `.res`, `.vgm`) | GitHub MD homebrew | Demozoo MD (6) | (tested) tree |
| SMS / Game Gear | smspower.org dev pages (live) | GitHub | Demozoo (5 + 2) | page live only |
| PC Engine | TOSEC PC-Engine/SuperGrafx (3 MB, 7 MB) | Demozoo (0) | | (tested) names only |
| GBA | Demozoo (3), GitHub (memory) | | | weak; hardly any retro-image formats |
| Game Boy tiles and maps (`.gbr`, `.gbm`, GB Camera) | gbdk-2020 and other GitHub repos | `devrs.com` Wayback (210 zips) | Demozoo GB/GBC (18 + 8) | (tested) |
| Neo Geo, Lynx, Jaguar | `wiki.neogeodev.org`, AtariAge (memory) | Demozoo (3, 2) | | weak |
| Amiga IFF: ILBM, HAM6/8, ANIM, DCTV | Aminet `pix/` via `INDEX` keywords | cd.textfiles.com Amiga CD sets | Aminet CDs on archive.org; scene.org; Fred Fish | (tested) |
| HAM-E, SHAM, PCHG, DHires, DigiView, RGBN | Aminet descriptions have almost none; try cd.textfiles.com and Demozoo AGA/OCS | scene.org artist trees | | (tested): sparse |
| Amiga animations / Deluxe Paint ANIM | Aminet `pix/anim` (726, 1.2 GB) | | | (tested) index |
| MSX2 screens (SC5-SC8, GL*, Graph Saurus) | TOSEC `MSX_MSX2` 614 MB; archive.org MSX-FAN items | msx.org and file-hunter downloads | Demozoo MSX (28) | listing only |
| PC-98 (Pi, MAG, Q4, MKI, ...) | `kawaii-dake-na-no` (used); `hp.vector.co.jp` Wayback; TOSEC `NEC_PC_9801` | Sembiance `pc88PI` | `ftp.vector.co.jp` | (tested) Wayback |
| X68000 | TOSEC `Sharp_X68000` 2.2 GB; `ftp.vector.co.jp/pack/x68` (40) | archive.org `Oh!X` CD 7z files | | listing only |
| FM Towns (HEL, ICN, TK4, Towns Paint II) | Sembiance `fmTownsHEL`, `fmTownsIcons`, `fmTownsTK4`, `townsPaintII` (33 files, none in corpus) | TOSEC `Fujitsu_FM_Towns` 74 MB; cd.textfiles.com `fujitsufree` | | Sembiance (fetched) |
| Apple II HGR/DHGR, Print Shop | cc65 `samples/apple2`; asimov `images/` | archive.org `softwarelibrary_apple`; TOSEC Apple_2 | Demozoo (13) | (tested) |
| Apple IIGS SHR (3200, APF, PNT) | asimov `images/gs` 1,868 | TOSEC `Apple_II_GS` 395 MB; Sembiance `a2gs*` | CiderPress II TestData (used) | (tested) index |
| Macintosh (MacPaint, PICT, clip art) | funet `info-mac/art` (262); archive.org Info-Mac | Sembiance `macPaint` 31, `macDraw` 49 | cd.textfiles.com BMUG CD | rsync (tested) |
| Atari ST (DEGAS, NEO, Tiny, ANI, SPU, ART, IMG, ...) | `ggnkua/Atari_ST_Sources` (counts in 4.2) | TOSEC Atari ST, Floppyshop picture disks (345) | cd.textfiles.com Suzy B / Crawly Crypt; scene.org; Demozoo | (tested) |
| Atari 8-bit | `ftp.pigwa.net`, atarionline.pl (used) | TOSEC `Atari_8_bit`; Demozoo (1,440) | | prior waves |
| TI-99/4A | TOSEC TI-99 (41 MB) | `whtech.com/ftp` | Sembiance `tiareGRA` | weak |
| Dragon / CoCo | TOSEC Dragon (7 MB), CoCo (21 MB) | `colorcomputerarchive.com`, `archive.worldofdragon.org` | KAOS repo (used) | Dragon list (tested) |
| SAM Coupe | ZXArt `.ss4` 285, `.sam3` 4, `.lce` 1 | TOSEC Sam_Coupe 311 MB; Demozoo 13 | Sembiance `samCoupe*` | one `.ss4` decoded (tested) |
| Oric | TOSEC Oric (26 MB) | oric.org | Demozoo (3) | listing only |
| Sharp MZ / X1 | TOSEC (1 MB, 9 MB), sharpmz.org | | | weak |
| Acorn BBC / RISC OS | TOSEC `Acorn_BBC` 219 MB, `Acorn_Archimedes` 68 MB | Wayback `mdfs.net`; stairwaytohell.com | Sembiance `acornDraw` | partly |
| DOS: PCX, GIF, BMP, TGA, Dr. Halo, ColoRIX, PIC | cd.textfiles.com (VGA Spectrum, GIF CDs, 10000 Graphics Pack) | Simtel/Garbo/Walnut Creek on archive.org | scene.org, `ftp.vector.co.jp/pack/dos` | (fetched) |
| Text mode (ANSI, XBin, ADF, IDF, TND, PCB, ...) | 16colo.rs API | scene.org `graphics/ascii` | Sembiance `ans` (used) | (tested) API |
| ZX Spectrum, Next, Timex, ZX81 | ZXArt API (19,507) | TOSEC, `softwarelibrary_zx_spectrum` | Demozoo, Sembiance `zx*` | (tested) |
| C64, VIC-20, Plus/4, C128 | Assembly64 zips (incl. unmined discmags/c128/bbs) | archive.org `softwarelibrary_c64`, TOSEC, `noname.c64.org` Wayback | CSDb webservice | (tested) listings |

## 8. Leads worth doing next

1. Mine `CSDB_discmags` (5,094 `.d64`), `CSDB_c128`, `CSDB_bbs` the way wave 6 mined the other folders.
2. Sparse-clone `ggnkua/Atari_ST_Sources` for `.PI1 .PC1 .NEO .ANI .TNY .SPU .SPC .CA1-3 .TN1-3 .DOO .APX .ART .IMG`
   and run `retro-image` over them; the `.ANI`, `.APX` and `.IMG` counts are large.
3. Pull the 345 Floppyshop Picture/Clip Art disks from TOSEC Atari ST (inner zip → `.st` → `fat12.py`).
4. Pull the ZXArt variants the registry lacks (`.sxg`, `.hrg`, `.specscii`, `.bsc`, `.mg*`, `.stl`, `.hlr`, `.grf`,
   `.s81`, `.3`, `.mlt`, `.ifl`) as a small sample per type. `.specscii` may unblock the SpecSCII question.
5. Aminet `pix/` by INDEX keyword (`ham8`, `dctv`, `24-bit`, `anim`) for the Amiga variants; roughly 200 packages cover the keyworded set.
6. cd.textfiles.com: `scenepixelg`, `suzybatari1/2`, `crawlycrypt1/2`, `fujitsufree`, `Amiga Format CD Collection` (`.files` give the size before you start).
7. Sembiance folders not yet in `corpus/` by name (counts: files / present by name):

| Folder | Files / present | Folder | Files / present |
|---|---|---|---|
| `fmTownsHEL` | 10 / 0 | `trs80HR` | 64 / 3 |
| `fmTownsIcons` | 12 / 0 | `trs` | 12 / 0 |
| `fmTownsTK4` | 7 / 0 | `tim`, `tim2` | 17 / 4, 6 / 0 |
| `townsPaintII` | 4 / 0 | `tiareGRA` | 4 / 0 |
| `gemMetafile` | 52 / 0 | `acornDraw` | 10 / 0 |
| `gemViewDither`, `gemViewMGF` | 10 / 0, 3 / 0 | `amiDrawSDW` | 12 / 0 |
| `macPaint`, `macDraw` | 31 / 4, 49 / 1 | `iffChunkyBitmap` | 3 / 0 |
| `neoDrawDrawing`, `neoPaintPattern` | 11 / 0, 2 / 0 | `zxAtr` | 9 / 0 |
| `atariGrafik`, `atariGraphDiagram` | 6 / 0, 3 / 0 | `zxCHR`, `zxGigascreen`, `zxStellar` | 14 / 2, 14 / 1, 10 / 1 |
| `a2gsPreferred`, `a2gs320x`, `a2gsSHStar` | 25 / 14, 12 / 3, 13 / 3 | `zxULAPlus`, `zx3`, `zxSCR`, `zxNXI` | 8 / 1, 8 / 1, 18 / 2, 3 / 1 |
| `pc88PI` | 17 / 4 | `timexHiColor`, `timexHiRes` | 7 / 1, 9 / 1 |
| `samCoupeMode4` | 10 / 3 | | |

   The index has 856 folders under `image/`; 189 are referenced by our manifests or local dirs. I looked only at
   the retro-looking names among the other 677. Check the manifests before fetching; "present" is by file name only.
8. `romhacking.net/download/utilities/` via Wayback if a tool is needed to produce samples (Tile Molester,
   YY-CHR, NES Screen Tool); run them as black boxes.

## 9. Verification log

Fetched or run this session (tested unless noted): Wayback CDX (all parameters in 2.1) on `garbo.uwasa.fi`,
`wuarchive.wustl.edu`, `www.amstrad.eu`, `hp.vector.co.jp`, `ftp.vector.co.jp`, `nesdev.parodius.com`, `www.devrs.com`,
`noname.c64.org`, `www.atari.org`, `www.atarimuseum.com`, `romhacking.net`, `mdfs.net`, `ftp.nvg.unit.no`, funet,
geocities; about 45 `id_` requests; archive.org advanced search, metadata API, `view_archive.php` for zip, iso
and 7z, Range-based zip listing on archive.org and `hackerswithstyle.se`; rsync listings on funet, scene.org,
sunet; `gh` tree, search and repo APIs; blobless sparse clone; Demozoo, ZXArt, 16colo.rs, CSDb, Pouet APIs;
Aminet `INDEX`, one package and readme; asimov `site_files.txt`; cd.textfiles.com `directory.html` and four
folders; Sembiance folder listings (about 55).

Liveness (HTTP status only): atarilegend, atarimania, atariarchives, whtech, colorcomputerarchive,
archive.worldofdragon, sharpmz, worldofsam, oric.org, msx.org, generation-msx, file-hunter, cpc-power,
cpcrulez, stairwaytohell, plus4world, sleepingelephant, lychesis, sinclair wiki, nesdev.org, forums.nesdev.org,
gbdev.io, sega-16, zophar, neogeodev wiki, smspower, dhs.nu, scene.org, pouet, csdb, spectrumcomputing,
`ftp.cdrom.com` (200), `www.lysator.liu.se` (200).

No DNS (`curl` exit 6): garbo.uwasa.fi, wuarchive.wustl.edu, www.amstrad.eu, sam.speccy.org, oric.ch, sharpx1.com,
atari.archive.umich.edu, ftp.nvg.unit.no; refused: hp.vector.co.jp; timeout: ftp.u-aizu.ac.jp. HTTPS failed but
HTTP answered: macintoshgarden.org, oldskool.org, atari-forum.com (301). Not reachable and not diagnosed:
`pcenginefx.com`, textfiles.com `/art/`, `amiga-pic-collection` (my guess at an item name, 404). 403 to `curl`:
lemon64, cpcwiki, mdfs.net, worldofspectrum.org `/pub/`, romhacking.net. Wrong domain: `16colors.rs`
(`16colo.rs` works).

Not verified: published Wayback rate limits; robots exclusion behaviour on a specific site; whether `rsync -av`
pattern fetching works exactly as written in 3.2 (only listings were run); contents of the nesdev and devrs
tool zips beyond names; every `memory` row above.

## Appendix: scripts (own code, stdlib only)

These are the files I ran, copied verbatim from the scratchpad. The `CORPUS_CONTACT` and `mktemp` edits were
re-tested after the change. Save them under `scripts/` (or anywhere outside `corpus/`) and `chmod +x`.

### `cdx.sh`

```sh
#!/bin/bash
# usage: cdx.sh "<query string after ?>"
# GET a Wayback CDX query; retry with doubling back-off on 429/5xx; body to stdout
q="$1"; tmp=$(mktemp); delay=8
for try in 1 2 3 4 5 6; do
  code=$(curl -sS -m 90 -o "$tmp" -w "%{http_code}" -A "retro-image-corpus-research/0.1 (${CORPUS_CONTACT:-no-contact-set})" "https://web.archive.org/cdx/search/cdx?$q")
  if [ "$code" = "200" ]; then cat "$tmp"; rm -f "$tmp"; exit 0; fi
  echo "cdx http $code (try $try), sleeping $delay" >&2; sleep $delay; delay=$((delay*2))
done
rm -f "$tmp"; exit 1
```

### `wbenum.py`

```python
#!/usr/bin/env python3
"""Enumerate Wayback CDX captures for a URL pattern, grouped by file extension.

Usage: wbenum.py URL [matchType] [extra cdx params...]   -> TSV on stdout
Own script, stdlib only. Retries on 429/503 with backoff, ~1 request/s.
"""
import sys, time, urllib.request, urllib.parse, urllib.error, json, collections, os

UA = "retro-image-corpus-research/0.1 (" + os.environ.get("CORPUS_CONTACT", "no-contact-set") + ")"   # set CORPUS_CONTACT to a URL or address you are happy to expose

def cdx(params, tries=7):
    q = urllib.parse.urlencode(params, doseq=True)
    delay = 6
    for i in range(tries):
        try:
            req = urllib.request.Request("https://web.archive.org/cdx/search/cdx?" + q, headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=120) as r:
                return r.read().decode("utf-8", "replace")
        except urllib.error.HTTPError as e:
            if e.code in (429, 503, 502, 504):
                sys.stderr.write(f"  cdx {e.code}, retry in {delay}s\n"); time.sleep(delay); delay = min(delay * 2, 120); continue
            raise
        except Exception as e:
            sys.stderr.write(f"  cdx error {e!r}, retry in {delay}s\n"); time.sleep(delay); delay = min(delay * 2, 120)
    raise SystemExit("cdx failed")

if __name__ == "__main__":
    url = sys.argv[1]
    mt = sys.argv[2] if len(sys.argv) > 2 else "prefix"
    extra = {}
    for a in sys.argv[3:]:
        k, v = a.split("=", 1)
        extra.setdefault(k, []).append(v)   # repeated keys (filter=...) accumulate
    params = {"url": url, "matchType": mt, "output": "json", "fl": "timestamp,original,mimetype,statuscode,digest,length",
              "filter": ["statuscode:200"], "collapse": "urlkey"}
    for k, v in extra.items():
        if k == "filter": params["filter"] += v
        else: params[k] = v[-1]
    body = cdx(params)
    rows = json.loads(body) if body.strip() else []
    for r in rows[1:]:
        print("\t".join(r))
```

### `extsum.py`

```python
import sys,collections,os,urllib.parse
c=collections.Counter(); b=collections.Counter()
for l in open(sys.argv[1]):
    f=l.rstrip("\n").split("\t")
    p=urllib.parse.urlparse(f[1]).path
    e=os.path.splitext(p)[1].lower()
    c[e]+=1; b[e]+=int(f[5]) if f[5].isdigit() else 0
tot=sum(c.values())
print(tot,"urls;", ", ".join(f"{e or '(none)'}:{n}" for e,n in c.most_common(int(sys.argv[2]) if len(sys.argv)>2 else 25)))
```

### `corpus_digests.py`

```python
#!/usr/bin/env python3
"""Print the CDX-style digest (base32 SHA-1) of every file under DIR, one per line.
Feed the output to wbfetch.py --seen to skip captures the corpus already holds."""
import base64, hashlib, os, sys
for root, _, files in os.walk(sys.argv[1]):
    for f in files:
        with open(os.path.join(root, f), "rb") as fh:
            print(base64.b32encode(hashlib.sha1(fh.read()).digest()).decode())
```

### `wbfetch.py`

```python
#!/usr/bin/env python3
"""Download Wayback captures listed in a wbenum.py TSV, politely and verified.

  wbfetch.py LIST.tsv OUTDIR [--max-bytes N] [--max-files N] [--min-interval S] [--seen FILE]

LIST.tsv columns: timestamp, original, mimetype, statuscode, digest, length (CDX order).
- skips digests already in --seen (one base32 SHA-1 per line; same encoding as the CDX digest)
- fetches /web/<ts>id_/<url> (raw bytes, no toolbar/link rewriting), >= --min-interval s apart
- retries 429/5xx/timeouts with exponential backoff
- rejects HTML bodies that were not captured as HTML; flags (keeps) bodies whose SHA-1 differs from the CDX digest
  (true for some 2000-era captures, where the digest covers something other than the payload)
- writes OUTDIR/<digest[:10]>_<basename> and appends OUTDIR/MANIFEST.tsv
Own code, stdlib only.
"""
import argparse, base64, hashlib, os, re, sys, time, urllib.error, urllib.parse, urllib.request

UA = "retro-image-corpus-research/0.1 (" + os.environ.get("CORPUS_CONTACT", "no-contact-set") + ")"   # set CORPUS_CONTACT to a URL or address you are happy to expose

def fetch(ts, url, tries=6):
    u = f"https://web.archive.org/web/{ts}id_/{url}"
    delay = 5
    for _ in range(tries):
        try:
            with urllib.request.urlopen(urllib.request.Request(u, headers={"User-Agent": UA}), timeout=120) as r:
                return r.read(), r.headers.get('Content-Disposition', '')
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504):
                time.sleep(delay); delay = min(delay * 2, 120); continue
            return None, f"HTTP {e.code}"
        except Exception:
            time.sleep(delay); delay = min(delay * 2, 120)
    return None, "gave up"

def safe_name(url, disp=""):
    m = re.search(r'filename="?([^";]+)', disp or "")        # download endpoints (/download/12/) name the file here
    base = m.group(1) if m else (os.path.basename(urllib.parse.unquote(urllib.parse.urlparse(url).path)) or "index")
    return re.sub(r"[^A-Za-z0-9._+-]", "_", base)[:80]   # no spaces/control/non-ASCII bytes in local names

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("list"); ap.add_argument("out")
    ap.add_argument("--max-bytes", type=int, default=50_000_000)
    ap.add_argument("--max-files", type=int, default=10**9)
    ap.add_argument("--min-interval", type=float, default=1.2)
    ap.add_argument("--seen")
    a = ap.parse_args()
    seen = set(open(a.seen).read().split()) if a.seen and os.path.exists(a.seen) else set()
    os.makedirs(a.out, exist_ok=True)
    man = open(os.path.join(a.out, "MANIFEST.tsv"), "a")
    batch = set()                      # digests already handled in this run
    total = n = 0
    last = 0.0
    for line in open(a.list, encoding="utf-8"):
        ts, url, mime, sc, dig, ln = line.rstrip("\n").split("\t")[:6]
        if dig in seen or dig in batch: continue
        batch.add(dig)
        wait = a.min_interval - (time.time() - last)
        if wait > 0: time.sleep(wait)
        last = time.time()
        body, info = fetch(ts, url)   # info = Content-Disposition on success, else an error string
        if body is None: print("FAIL", info, url, file=sys.stderr); continue
        got = base64.b32encode(hashlib.sha1(body).digest()).decode()
        if got in seen or (got != dig and got in batch): continue   # body already held (CDX digest can differ on old captures)
        batch.add(got)
        note = "" if got == dig else "; cdx-digest-differs (old capture, body kept)"
        if body[:15].lower().startswith((b"<!doctype html", b"<html")) and "html" not in mime:
            print("HTML-WRAPPED", url, file=sys.stderr); continue
        dest = f"{got[:10]}_{safe_name(url, info)}"   # name from the real body hash
        open(os.path.join(a.out, dest), "wb").write(body)
        man.write(f"{dest}\t\thttps://web.archive.org/web/{ts}id_/{url}\tnone stated\tsha1-b32 {got}; {len(body)} bytes{note}\n"); man.flush()
        total += len(body); n += 1
        print(f"ok {len(body):>9} {dest}")
        if total >= a.max_bytes or n >= a.max_files: break

if __name__ == "__main__":
    main()
```

### `rzip.py`

```python
#!/usr/bin/env python3
"""List / extract members of a remote ZIP over HTTP Range requests (stdlib only).

  rzip.py list URL            -> name<TAB>size per member (reads only the central directory)
  rzip.py get  URL NAME OUT   -> extracts one member
Needs a server that honours Range (Accept-Ranges: bytes).
"""
import io, os, sys, urllib.request, zipfile

UA = "retro-image-corpus-research/0.1 (" + os.environ.get("CORPUS_CONTACT", "no-contact-set") + ")"   # set CORPUS_CONTACT to a URL or address you are happy to expose

class HTTPFile(io.RawIOBase):
    def __init__(self, url):
        self.url, self.pos = url, 0
        req = urllib.request.Request(url, method="HEAD", headers={"User-Agent": UA})
        with urllib.request.urlopen(req, timeout=60) as r:
            self.size = int(r.headers["Content-Length"])
            if r.headers.get("Accept-Ranges", "").lower() != "bytes":
                raise SystemExit("server does not advertise Accept-Ranges: bytes")
            self.url = r.geturl()
        self.requests = 0
    def seekable(self): return True
    def readable(self): return True
    def tell(self): return self.pos
    def seek(self, off, whence=0):
        self.pos = {0: off, 1: self.pos + off, 2: self.size + off}[whence]
        return self.pos
    def read(self, n=-1):
        if n < 0 or self.pos + n > self.size: n = self.size - self.pos
        if n <= 0: return b""
        req = urllib.request.Request(self.url, headers={"User-Agent": UA, "Range": f"bytes={self.pos}-{self.pos+n-1}"})
        with urllib.request.urlopen(req, timeout=120) as r: data = r.read()
        self.requests += 1; self.pos += len(data); return data
    def readinto(self, b):
        d = self.read(len(b)); b[:len(d)] = d; return len(d)

if __name__ == "__main__":
    f = HTTPFile(sys.argv[2]); z = zipfile.ZipFile(io.BufferedReader(f, 1 << 16))
    if sys.argv[1] == "list":
        for i in z.infolist(): print(f"{i.filename}\t{i.file_size}")
        sys.stderr.write(f"{len(z.infolist())} members, {f.requests} range requests, archive {f.size} bytes\n")
    else:
        open(sys.argv[4], "wb").write(z.read(sys.argv[3]))
```

### `fat12.py`

```python
#!/usr/bin/env python3
"""Extract every file from a FAT12 floppy image (.st / .img / .dsk / MSA-expanded). Own code, stdlib only.

  fat12.py IMAGE OUTDIR      (prints one line per extracted file)

Reads the BPB (works for Atari ST and PC disks); 8.3 names only (long-name entries are skipped);
names are sanitised to ASCII and de-duplicated. Not for CP/M, ProDOS or Amiga OFS/FFS disks.
"""
import os, re, struct, sys

def main(img, out):
    d = open(img, "rb").read()
    bps, spc, rsv, nfat, nroot, tot16, _, spf = struct.unpack_from("<HBHBHHBH", d, 11)
    rsv = rsv or 1          # some ST boot sectors carry 0 here; TOS reads that as 1
    if bps not in (512, 1024, 2048) or spc == 0 or nfat == 0 or spf == 0: raise SystemExit("not a FAT12 BPB")
    fat = d[rsv * bps : (rsv + spf) * bps]
    root = (rsv + nfat * spf) * bps
    data = root + nroot * 32
    def nxt(c):
        v = struct.unpack_from("<H", fat, c * 3 // 2)[0]
        return (v >> 4) if c & 1 else (v & 0xFFF)
    def chain(c):
        while 2 <= c < 0xFF0:
            yield c; c = nxt(c)
    def clus(c): o = data + (c - 2) * spc * bps; return d[o : o + spc * bps]
    def entries(buf):
        for o in range(0, len(buf), 32):
            e = buf[o : o + 32]
            if e[0] == 0: break
            if e[0] == 0xE5 or e[11] == 0x0F or e[11] & 0x08: continue
            name = e[:8].decode("latin-1").rstrip(); ext = e[8:11].decode("latin-1").rstrip()
            yield name, ext, e[11], struct.unpack_from("<H", e, 26)[0], struct.unpack_from("<I", e, 28)[0]
    def walk(buf, rel):
        for name, ext, attr, c, size in entries(buf):
            if name in (".", ".."): continue
            fn = re.sub(r"[^A-Za-z0-9._+-]", "_", name + ("." + ext if ext else ""))
            if attr & 0x10: walk(b"".join(clus(x) for x in chain(c)), os.path.join(rel, fn)); continue
            body = b"".join(clus(x) for x in chain(c))[:size]
            p = os.path.join(out, rel, fn); os.makedirs(os.path.dirname(p), exist_ok=True)
            k = 0
            while os.path.exists(p): k += 1; p = os.path.join(out, rel, f"{k}_{fn}")
            open(p, "wb").write(body); print(f"{size:8d} {os.path.relpath(p, out)}")
    walk(d[root:data], "")

if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
```

### `inventory.py`

```python
#!/usr/bin/env python3
"""Unpack downloaded archives and sort their members by what retro-image makes of them.

  inventory.py ARCHIVE_DIR WORK_DIR [--cli PATH]

For every .zip (zipfile, no shell) and .lzh/.lha (lhasa/lha) in ARCHIVE_DIR, extract into
WORK_DIR/<archive>/, then for each member whose extension is registered in
`retro-image --list-formats`, run `retro-image -i FILE -o /dev/null`:
  accepted  -> likely real sample of a supported format
  rejected  -> registered extension, decoder says no: unknown variant, or not an image (the interesting list)
Extensionless members are tried too (signature-detected formats such as IFF).
Members with other unregistered extensions are only counted (candidates for new formats).
Zip-slip safe: member paths are flattened to a sanitised relative path under WORK_DIR.
"""
import collections, os, re, subprocess, sys, zipfile

def registered(cli):
    out = subprocess.run([cli, "--list-formats"], capture_output=True, text=True).stdout
    exts = set()
    for line in out.splitlines():
        parts = line.split("\t")
        if len(parts) >= 3: exts.update(e.strip().lower() for e in parts[2].split(",") if e.strip())
    return exts

def safe(path):   # drop drive letters, '..', absolute parts; keep it ASCII
    parts = [re.sub(r"[^A-Za-z0-9._+-]", "_", p) for p in re.split(r"[\\/]+", path) if p not in ("", ".", "..")]
    return os.path.join(*parts) if parts else "_"

def extract(arc, dest):
    os.makedirs(dest, exist_ok=True)
    if arc.lower().endswith(".zip"):
        try:
            with zipfile.ZipFile(arc) as z:
                for i in z.infolist():
                    if i.is_dir(): continue
                    t = os.path.join(dest, safe(i.filename)); os.makedirs(os.path.dirname(t), exist_ok=True)
                    open(t, "wb").write(z.read(i))
        except NotImplementedError:     # DOS-era shrink/reduce/implode: zipfile cannot, Info-ZIP unzip can
            subprocess.run(["unzip", "-o", "-qq", "-d", dest, arc], capture_output=True)
    else:
        subprocess.run(["lha", "xqw=" + dest, arc], capture_output=True)   # lhasa/lha: x=extract, q=quiet, w=dest

if __name__ == "__main__":
    adir, work = sys.argv[1], sys.argv[2]
    cli = sys.argv[sys.argv.index("--cli") + 1] if "--cli" in sys.argv else "retro-image"
    known = registered(cli)
    acc, rej, unk = [], [], collections.Counter()
    for name in sorted(os.listdir(adir)):
        if not name.lower().endswith((".zip", ".lzh", ".lha")): continue
        dest = os.path.join(work, name); 
        try: extract(os.path.join(adir, name), dest)
        except Exception as e: print("BAD ARCHIVE", name, e); continue
        for root, _, files in os.walk(dest):
            for f in files:
                ext = os.path.splitext(f)[1][1:].lower(); p = os.path.join(root, f)
                if ext in known or (ext == "" and os.path.getsize(p) < 20_000_000):   # extensionless (Amiga, Mac): rely on signatures
                    r = subprocess.run([cli, "-i", p, "-o", "/dev/null"], capture_output=True, text=True)
                    (acc if r.returncode == 0 else rej).append(p)
                else: unk[ext or "(none)"] += 1
    print(f"accepted {len(acc)}, rejected {len(rej)}")
    for p in rej: print("REJECTED", p)
    print("unregistered extensions:", ", ".join(f"{e}:{n}" for e, n in unk.most_common(25)))
```
