//! Petmate workspaces (`.petmate`): PETSCII screens for the C64, C128 (40
//! and 80 columns), C16/Plus4, VIC-20 and PET, saved as JSON.
//!
//! Sources:
//! - Structure: Petmate and Petmate 9, both MIT licensed,
//!   <https://github.com/nurpax/petmate> and <https://github.com/wbochar/petmate9>.
//!   Only the structure was taken from them, as documentation of the file
//!   (the `version`, `screens` and `framebufs` members, the cell fields
//!   `code`, `color`, `attr` and `transparent`, and the way the exporters
//!   in `src/utils/exporters/util.ts` draw a screen); no code was copied.
//!   The VDC attribute bits (`src/utils/vdcAttr.ts`) are alternate set
//!   `$80`, reverse `$40`, underline `$20` and blink `$10`; legacy 80-column
//!   `c128Upper`/`c128Lower` screens are VDC screens
//!   (`src/redux/workspaceMigrate.ts`). Checked on the 14 files from the
//!   petmate9 repository's `_defaults/` and `_tests/` (versions 3 and 4).
//! - JSON grammar: see [`crate::json`].
//! - Character sets: the Commodore ROM character dumps (PET, VIC-20, C16,
//!   C128; the C64 one is [`super::petscii::CHARGEN`]), 256 glyphs of 8
//!   bytes each with the reversed half included. The eight files in
//!   `petmate_fonts/` were taken unchanged from `assets/` of petmate9
//!   (`{pet,vic20,c16,c128}-charset-{upper,lower}.bin`; the repository is
//!   MIT licensed, the glyph data is Commodore's ROM content, embedded with
//!   the owner's approval as is the C64 set). The copyright notice and
//!   license text travel with them in `petmate_fonts/LICENSE-petmate9.txt`;
//!   the file structure above is from the same project. The VDC screen shows the C128
//!   upper set as glyphs 0-255 and the lower set as 256-511.
//! - Palettes: those of the platform's other formats in this crate (VIC-II,
//!   VIC, TED and VDC RGBI). The PET screen is white on black.
//!
//! The picture is the first of the workspace's `screens`, without border.
//! Character sets that are not ROM dumps (Commodore Business, DirArt and
//! custom fonts) are not supported.

use super::{c128, petscii, ted, vic2, vic20};
use crate::image::check_size;
use crate::json::Value;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// What a screen's color numbers mean.
#[derive(Clone, Copy)]
enum Machine {
    C64,
    Vic20,
    Pet,
    Ted,
    Vdc,
}

impl Machine {
    fn rgb(self, color: i64) -> u32 {
        let color = (color & 0xff) as u8;
        match self {
            Machine::C64 => vic2::rgb(color),
            Machine::Vic20 => vic20::PALETTE[usize::from(color & 15)],
            Machine::Pet if color == 1 => 0xffffff,
            Machine::Pet => 0,
            Machine::Ted => ted::rgb(color),
            Machine::Vdc => c128::rgbi(color & 15),
        }
    }
}

/// How a screen is drawn: the machine and the character ROM.
struct Font {
    machine: Machine,
    /// 256 glyphs of 8 bytes.
    glyphs: &'static [u8],
    vdc: Option<Vdc>,
}

/// An 80-column C128 screen: cells carry attribute bits and the alternate
/// set (glyphs 256-511) is the C128 lower case set.
struct Vdc {
    alternate: &'static [u8],
    /// Legacy `c128Lower` screens: every cell uses the alternate set.
    always_alternate: bool,
}

/// VDC attribute bits.
const ALTERNATE: i64 = 0x80;
const REVERSE: i64 = 0x40;
const UNDERLINE: i64 = 0x20;

static C128_UPPER: &[u8] = include_bytes!("petmate_fonts/c128-upper.bin");
static C128_LOWER: &[u8] = include_bytes!("petmate_fonts/c128-lower.bin");

fn plain(machine: Machine, glyphs: &'static [u8]) -> Font {
    Font {
        machine,
        glyphs,
        vdc: None,
    }
}

/// The font a charset name selects. `columns` tells the legacy 80-column
/// C128 sets from the 40-column ones.
fn charset(name: &str, columns: usize) -> Option<Font> {
    let vdc = |always_alternate| Font {
        machine: Machine::Vdc,
        glyphs: C128_UPPER,
        vdc: Some(Vdc {
            alternate: C128_LOWER,
            always_alternate,
        }),
    };
    Some(match name {
        "upper" => plain(Machine::C64, &petscii::CHARGEN[..2048]),
        "lower" => plain(Machine::C64, &petscii::CHARGEN[2048..]),
        "c128vdc" => vdc(false),
        "c128Upper" if columns >= 80 => vdc(false),
        "c128Lower" if columns >= 80 => vdc(true),
        "c128Upper" => plain(Machine::C64, C128_UPPER),
        "c128Lower" => plain(Machine::C64, C128_LOWER),
        "c16Upper" => plain(Machine::Ted, include_bytes!("petmate_fonts/c16-upper.bin")),
        "c16Lower" => plain(Machine::Ted, include_bytes!("petmate_fonts/c16-lower.bin")),
        "vic20Upper" => plain(
            Machine::Vic20,
            include_bytes!("petmate_fonts/vic20-upper.bin"),
        ),
        "vic20Lower" => plain(
            Machine::Vic20,
            include_bytes!("petmate_fonts/vic20-lower.bin"),
        ),
        "petGfx" => plain(Machine::Pet, include_bytes!("petmate_fonts/pet-upper.bin")),
        "petBiz" => plain(Machine::Pet, include_bytes!("petmate_fonts/pet-lower.bin")),
        _ => return None,
    })
}

/// Petmate's codes for a cell without a character (non-VDC and VDC).
const TRANSPARENT: [i64; 2] = [256, 512];
const SPACE: i64 = 0x20;

/// One screen cell as stored in the file.
struct Cell {
    code: usize,
    color: i64,
    attr: i64,
}

impl Cell {
    fn parse(value: &Value, font: &Font) -> Option<Cell> {
        let code = value.get("code")?.as_int()?;
        let transparent = value.get("transparent").and_then(Value::as_bool) == Some(true);
        let code = if transparent || TRANSPARENT.contains(&code) {
            SPACE
        } else {
            code
        };
        let color = value.get("color")?.as_int()?;
        let mut attr = match value.get("attr") {
            Some(attr) => attr.as_int()? & 0xff,
            None => color & 15,
        };
        if font.vdc.as_ref().is_some_and(|vdc| vdc.always_alternate) {
            attr |= ALTERNATE;
        }
        Some(Cell {
            code: usize::try_from(code).ok().filter(|&c| c < 256)?,
            color: (0..=255).contains(&color).then_some(color)?,
            attr,
        })
    }

    /// The eight pixel rows of the glyph, as the VDC or VIC would draw them.
    fn rows(&self, font: &Font) -> [u8; 8] {
        let source = match &font.vdc {
            Some(vdc) if self.attr & ALTERNATE != 0 => vdc.alternate,
            _ => font.glyphs,
        };
        let mut rows = [0; 8];
        rows.copy_from_slice(&source[self.code * 8..self.code * 8 + 8]);
        if font.vdc.is_some() {
            if self.attr & UNDERLINE != 0 {
                rows[7] = 0xff;
            }
            if self.attr & REVERSE != 0 {
                rows = rows.map(|row| !row);
            }
        }
        rows
    }
}

/// The screen the workspace shows first, as a picture.
pub(super) fn decode_petmate(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"{\"version\":") {
        return Err(DecodeError::Unrecognized);
    }
    let root = Value::parse(data).ok_or(DecodeError::Unrecognized)?;
    render(&root).ok_or(DecodeError::Unrecognized)
}

fn render(root: &Value) -> Option<Image> {
    // Versions 3 and 4 are the ones checked.
    if !matches!(root.get("version")?.as_int()?, 3 | 4) {
        return None;
    }
    let first = root.get("screens")?.as_array()?.first()?.as_int()?;
    let frames = root.get("framebufs")?.as_array()?;
    let frame = frames.get(usize::try_from(first).ok()?)?;
    let width = usize::try_from(frame.get("width")?.as_int()?).ok()?;
    let height = usize::try_from(frame.get("height")?.as_int()?).ok()?;
    check_size(width.checked_mul(8)?, height.checked_mul(8)?).ok()?;
    let font = charset(frame.get("charset")?.as_str()?, width)?;
    let background = frame.get("backgroundColor")?.as_int()?;
    let rows = frame.get("framebuf")?.as_array()?;
    if rows.len() != height {
        return None;
    }
    let mut cells = Vec::with_capacity(width * height);
    for row in rows {
        let row = row.as_array().filter(|row| row.len() == width)?;
        for cell in row {
            cells.push(Cell::parse(cell, &font)?);
        }
    }
    let glyphs: Vec<[u8; 8]> = cells.iter().map(|cell| cell.rows(&font)).collect();
    let (pixel_width, pixel_height) = (width * 8, height * 8);
    let color = |index: usize| {
        let (x, y) = (index % pixel_width, index / pixel_width);
        let cell = y / 8 * width + x / 8;
        let set = glyphs[cell][y % 8] & (0x80 >> (x % 8)) != 0;
        font.machine
            .rgb(if set { cells[cell].color } else { background })
    };
    Some(Image::from_colors(
        pixel_width as u32,
        pixel_height as u32,
        (0..pixel_width * pixel_height).map(color),
    ))
}
