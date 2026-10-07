//! KiSS sets: the `.cnf` configuration file of a doll, drawn as its first
//! set from the cels and palettes it names.
//!
//! Sources:
//! - "KISS/GS" by ITO Takayuki (public domain, see `kiss.rs`), section 5, the
//!   configuration file: `(width,height)` of the screen (448 x 320 if absent),
//!   `%` palette files numbered in order, `#mark.fix cel [*palette] [:sets]`
//!   cel entries (cels of the same mark form an object, earlier entries are
//!   in front of later ones, `*` picks a palette file, `:` the sets that draw
//!   the cel), `$group x,y ...` set lines (one per set, positions by object
//!   mark, `*` for an object that is not there, continued on lines starting
//!   with a blank), `;` comments. The cel header's offsets place a cel
//!   relative to its object.
//! - Checked on the 10 free dolls of the corpus (`corpus/extra/misc-computers/kiss`):
//!   the first set of each draws a believable doll, wardrobe or scene with
//!   its clothes in front of the body.
//!
//! Choices of this crate: only the first set is drawn, with its palette
//! group, from the first palette file where a cel names none; the screen
//! shows color 0 of the first palette file where no cel covers it, which
//! matches the backgrounds of the dolls checked (the border color of the `[`
//! line is not used); a missing cel is skipped, a missing palette file turns
//! its cels gray, and a set that draws nothing is not an image. File names
//! are tried as written and in lower case, upper case and the two mixes,
//! since KiSS does not tell the cases apart and archives do not agree.

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::{full_palette, read_cel, read_palette};
use crate::image::{CLEAR, check_size};
use crate::{Companions, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;
const DEFAULT_SIZE: (usize, usize) = (448, 320);
const MAX_SIDE: usize = 4096;
/// Cel entries, set positions and palette files read from a file.
const MAX_ENTRIES: usize = 4096;
/// Cel pixels drawn in all, so that a file cannot ask for endless work.
const MAX_PIXELS: usize = 1 << 27;

/// What the configuration file says that matters for the first set.
struct Set {
    width: usize,
    height: usize,
    palette_files: Vec<String>,
    cels: Vec<Entry>,
    /// The palette group of the first set line.
    group: usize,
    /// Where each object (by mark) is in the first set; `None` if it is not.
    positions: Vec<Option<(i64, i64)>>,
}

struct Entry {
    mark: usize,
    file: String,
    palette: usize,
    /// Whether the first set draws the cel.
    in_first_set: bool,
}

pub(super) fn decode_set(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let set = parse(data);
    check_size(set.width, set.height)?;
    if set.width > MAX_SIDE || set.height > MAX_SIDE {
        return Err(FAIL);
    }
    let palettes: Vec<Option<Vec<u32>>> = set
        .palette_files
        .iter()
        .map(|name| {
            let kcf = fetch(companions, name)?;
            read_palette(&kcf, set.group).map(|colors| full_palette(Some(colors), 8))
        })
        .collect();
    let background = palettes
        .first()
        .and_then(Option::as_ref)
        .map_or(CLEAR, |palette| 0xff00_0000 | palette[0]);
    let fill = core::iter::repeat(background);
    let mut canvas = Image::from_argb(set.width as u32, set.height as u32, fill)?;
    let (mut drawn, mut pixels) = (0, 0usize);
    // Earlier entries are in front, so the last one goes down first.
    for entry in set.cels.iter().rev().filter(|entry| entry.in_first_set) {
        let Some(&Some((x, y))) = set.positions.get(entry.mark) else {
            continue;
        };
        let Some(cel) = fetch(companions, &entry.file).and_then(|data| read_cel(&data).ok()) else {
            continue;
        };
        pixels = pixels.saturating_add(cel.width * cel.height);
        if pixels > MAX_PIXELS {
            return Err(FAIL);
        }
        let (x, y) = (x + cel.x as i64, y + cel.y as i64);
        match palettes.get(entry.palette).and_then(Option::as_ref) {
            Some(palette) => cel.draw(&mut canvas, x, y, palette),
            None => {
                let gray = cel
                    .index_bits()
                    .map_or_else(Vec::new, |bits| full_palette(None, bits));
                cel.draw(&mut canvas, x, y, &gray);
            }
        }
        drawn += 1;
    }
    match drawn {
        0 => Err(FAIL),
        _ => Ok(canvas),
    }
}

/// A file the set names, as written or in the other usual spellings.
fn fetch(companions: &dyn Companions, name: &str) -> Option<Vec<u8>> {
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    let join = |stem: String, extension: String| match extension.is_empty() {
        true => stem,
        false => format!("{stem}.{extension}"),
    };
    [
        name.to_owned(),
        name.to_ascii_lowercase(),
        name.to_ascii_uppercase(),
        join(stem.to_ascii_lowercase(), extension.to_ascii_uppercase()),
        join(stem.to_ascii_uppercase(), extension.to_ascii_lowercase()),
    ]
    .iter()
    .find_map(|spelling| companions.get_named(spelling))
}

fn parse(data: &[u8]) -> Set {
    let mut set = Set {
        width: DEFAULT_SIZE.0,
        height: DEFAULT_SIZE.1,
        palette_files: Vec::new(),
        cels: Vec::new(),
        group: 0,
        positions: Vec::new(),
    };
    // Whether the first set line, which may continue on later lines, is open.
    let (mut seen_set, mut in_set) = (false, false);
    for line in data.split(|&b| b == b'\n') {
        let line = line.split(|&b| b == b';').next().unwrap_or_default();
        let line = line.trim_ascii_end();
        match line.first() {
            None => {}
            Some(b'(') => {
                let mut numbers = numbers(&line[1..]);
                if let (Some(width), Some(height)) = (numbers.next(), numbers.next()) {
                    (set.width, set.height) = (width, height);
                }
                in_set = false;
            }
            Some(b'%') => {
                let name = line[1..].trim_ascii();
                if set.palette_files.len() < MAX_ENTRIES {
                    set.palette_files
                        .push(String::from_utf8_lossy(name).to_string());
                }
                in_set = false;
            }
            Some(b'#') => {
                if let Some(entry) = cel_entry(&line[1..])
                    && set.cels.len() < MAX_ENTRIES
                {
                    set.cels.push(entry);
                }
                in_set = false;
            }
            Some(b'$') if !seen_set => {
                (seen_set, in_set) = (true, true);
                let mut tokens = line[1..]
                    .split(u8::is_ascii_whitespace)
                    .filter(|t| !t.is_empty());
                set.group = tokens.next().and_then(|t| numbers(t).next()).unwrap_or(0);
                positions(tokens, &mut set.positions);
            }
            Some(b' ' | b'\t') if in_set => {
                positions(
                    line.split(u8::is_ascii_whitespace)
                        .filter(|t| !t.is_empty()),
                    &mut set.positions,
                );
            }
            Some(_) => in_set = false,
        }
    }
    set
}

/// `mark.fix cel [*palette] [:sets...]`, the part of a `#` line after the `#`.
fn cel_entry(line: &[u8]) -> Option<Entry> {
    let tokens: Vec<&[u8]> = line
        .split(u8::is_ascii_whitespace)
        .filter(|t| !t.is_empty())
        .collect();
    let mark = numbers(tokens.first()?).next()?;
    let file = String::from_utf8_lossy(tokens.get(1)?).to_string();
    let (mut palette, mut in_first_set) = (0, true);
    for (i, token) in tokens.iter().enumerate().skip(2) {
        if let Some(number) = token.strip_prefix(b"*") {
            // The palette number may follow after a blank.
            let number = if number.is_empty() {
                tokens.get(i + 1).copied()
            } else {
                Some(number)
            };
            palette = number.and_then(|n| numbers(n).next()).unwrap_or(0);
        } else if token.starts_with(b":") {
            in_first_set = tokens[i..]
                .iter()
                .flat_map(|t| numbers(t))
                .any(|set| set == 0);
            break;
        }
    }
    Some(Entry {
        mark,
        file,
        palette,
        in_first_set,
    })
}

/// Object positions: `x,y` pairs, and `*` for an object that is not there.
fn positions<'a>(tokens: impl Iterator<Item = &'a [u8]>, out: &mut Vec<Option<(i64, i64)>>) {
    for token in tokens {
        if out.len() == MAX_ENTRIES {
            break;
        }
        out.push(
            token
                .iter()
                .position(|&b| b == b',')
                .and_then(|comma| signed(&token[..comma]).zip(signed(&token[comma + 1..]))),
        );
    }
}

/// An optionally negative decimal number, saturating.
fn signed(token: &[u8]) -> Option<i64> {
    let (negative, digits) = match token.split_first() {
        Some((b'-', digits)) => (true, digits),
        _ => (false, token),
    };
    let value = numbers(digits).next()? as i64;
    Some(if negative { -value } else { value })
}

/// The runs of decimal digits in `text` as numbers, saturating at a million.
fn numbers(text: &[u8]) -> impl Iterator<Item = usize> + '_ {
    text.split(|b| !b.is_ascii_digit())
        .filter(|digits| !digits.is_empty())
        .map(|digits| {
            digits.iter().fold(0usize, |n, &d| {
                (n * 10 + usize::from(d - b'0')).min(1_000_000)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Files(Vec<(&'static str, Vec<u8>)>);

    impl Companions for Files {
        fn get(&self, _extension: &str) -> Option<Vec<u8>> {
            None
        }

        fn get_named(&self, file_name: &str) -> Option<Vec<u8>> {
            self.0
                .iter()
                .find(|(name, _)| *name == file_name)
                .map(|(_, data)| data.clone())
        }
    }

    /// A 2x2 8-bit cel with indices `pixels` and header offsets.
    fn cel(pixels: [u8; 4], x: u16, y: u16) -> Vec<u8> {
        let mut data = b"KiSS\x20\x08\0\0".to_vec();
        for word in [2, 2, x, y] {
            data.extend_from_slice(&word.to_le_bytes());
        }
        data.resize(32, 0);
        data.extend_from_slice(&pixels);
        data
    }

    /// A 24-bit palette file of one group: black, red, green, blue.
    fn palette() -> Vec<u8> {
        let mut data = b"KiSS\x10\x18\0\0".to_vec();
        for word in [4u16, 1] {
            data.extend_from_slice(&word.to_le_bytes());
        }
        data.resize(32, 0);
        data.extend_from_slice(&[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
        data
    }

    #[test]
    fn a_set_draws_its_first_set_back_to_front_on_the_palette_background() {
        let cnf = b"; a doll\r\n(5,4)\r\n%col.kcf\r\n\
            #0.0 TOP.CEL *0 ; in front\r\n\
            #1.0 under.cel :0 1\r\n\
            #2.0 later.cel :1 2\r\n\
            $0 0,0 1,1\r\n \r\n$1 3,3 *\r\n";
        let files = Files(alloc::vec![
            // The cel names are written in other cases than the files.
            ("col.kcf", palette()),
            ("top.cel", cel([1, 0, 0, 0], 0, 0)),
            ("under.cel", cel([2, 2, 2, 2], 0, 0)),
            ("later.cel", cel([3, 3, 3, 3], 0, 0)),
        ]);
        let image = decode_set(cnf, &files).unwrap();
        assert_eq!((image.width(), image.height()), (5, 4));
        // Object 0 is at (0,0): red in front of the green object 1 at (1,1).
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(image.get(1, 0), 0x000000);
        assert_eq!(image.get(1, 1), 0x00ff00);
        assert_eq!(image.get(2, 2), 0x00ff00);
        // The later cel is in the second set only, and set 1's line is ignored.
        assert_eq!(image.get(4, 3), 0x000000);
    }

    #[test]
    fn header_offsets_and_negative_positions_move_and_clip_cels() {
        let cnf = b"(3,3)\n%col.kcf\n#0 a.cel\n$0 -2,-2\n";
        let files = Files(alloc::vec![
            ("col.kcf", palette()),
            ("a.cel", cel([1, 2, 3, 3], 1, 1)),
        ]);
        let image = decode_set(cnf, &files).unwrap();
        // The cel's top left is at (-2,-2) + (1,1), so only its last pixel,
        // index 3 (blue), shows, at (0,0).
        assert_eq!(image.get(0, 0), 0x0000ff);
        assert_eq!(image.get(1, 0), 0x000000);
        assert_eq!(image.get(0, 1), 0x000000);
    }

    #[test]
    fn a_set_that_draws_nothing_or_names_no_files_is_not_an_image() {
        let files = Files(alloc::vec![("a.cel", cel([1, 1, 1, 1], 0, 0))]);
        assert!(decode_set(b"#0 a.cel\n$0 0,0\n", &files).is_ok());
        assert!(decode_set(b"#0 b.cel\n$0 0,0\n", &files).is_err());
        assert!(decode_set(b"#0 a.cel\n$0 *\n", &files).is_err());
        assert!(decode_set(b"[mysqld]\nport = 3306\n", &files).is_err());
        assert!(decode_set(b"#0 a.cel\n$0 0,0\n", &crate::NoCompanions).is_err());
        assert!(decode_set(b"(9999,9999)\n#0 a.cel\n$0 0,0\n", &files).is_err());
    }

    #[test]
    fn numbers_and_positions_parse_signs_and_junk() {
        assert_eq!(numbers(b"12.2001").collect::<Vec<_>>(), [12, 2001]);
        assert_eq!(signed(b"-5"), Some(-5));
        assert_eq!(signed(b"x"), None);
        let mut out = Vec::new();
        positions([&b"3,-4"[..], b"*", b"7"].into_iter(), &mut out);
        assert_eq!(out, [Some((3, -4)), None, None]);
    }
}
