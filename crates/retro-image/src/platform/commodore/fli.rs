//! C64 FLI formats: a bitmap with a separate screen RAM for each of the
//! eight lines of a character row (multicolour FLI, hires AFLI), optionally
//! with a per-line background (`$D021`) table.
//!
//! Sources (memory maps):
//! - Codebase64 "C64 Graphics File Format Specs" (CB),
//!   <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>
//! - GoDot loader pages (GD), <https://www.godot64.de/german/lstab.htm>
//! - FLI display mechanism: <https://www.cebix.net/VIC-Article.txt>
//!
//! | Format | Sources |
//! |---|---|
//! | FLI Designer (FD2, FLI) | CB "FLI Designer 1.1 & 2.0", GD FLI-Designer |
//! | FLI Graph (BML) | CB "FLI Graph 2.2", GD FLI Graph |
//! | AFLI-editor (AFL) | CB "AFLI-editor v2.0", GD AFLI |
//! | Hires FLI Designer (HFC, HFD) | CB "Hires FLI" |
//! | Hires Manager (HIM, unpacked) | CB "Hires Manager", GD HiManRaw |
//!
//! Picture heights of Hires FLI Designer (112 lines) and Hires Manager
//! (192 lines, starting at the second character row) observed from
//! `recoil2png` output.

use super::prg::Prg;
use super::vic2::{BITMAP_LEN, Background, Bitmap, FLI_BUG, Frame, SCREEN_LEN, Screens};
use crate::{DecodeError, Image};

/// Bytes of eight screen RAMs, 1024 apart.
const SCREENS_LEN: usize = 7 * 1024 + SCREEN_LEN;

/// Memory map of a FLI picture.
pub(super) struct Fli {
    pub load: u16,
    pub sizes: &'static [usize],
    pub bitmap: u16,
    pub screens: u16,
    /// Colour RAM; `None` for hires (AFLI).
    pub color: Option<u16>,
    /// Address of a 200-entry `$D021` table; black when `None`.
    pub backgrounds: Option<u16>,
    pub height: usize,
    /// Bitmap lines above the picture that are not shown.
    pub skip: usize,
}

impl Fli {
    pub(super) fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        if !self.sizes.contains(&data.len()) {
            return Err(DecodeError::Unrecognized);
        }
        self.frame(&Prg::new(data, self.load))
            .map(|frame| frame.skip_lines(self.skip).to_image(FLI_BUG))
            .ok_or(DecodeError::Unrecognized)
    }

    pub(super) fn frame(&self, prg: &Prg) -> Option<Frame> {
        let screens = Screens::Fli {
            data: prg.at(self.screens, SCREENS_LEN)?,
            stride: 1024,
        };
        let background = match self.backgrounds {
            Some(addr) => Background::PerLine(prg.at(addr, self.height)?),
            None => Background::Fixed(0),
        };
        let bitmap = Bitmap {
            bitmap: prg.at(self.bitmap, BITMAP_LEN)?,
            screens,
            color: match self.color {
                Some(addr) => prg.at(addr, SCREEN_LEN)?,
                None => &[],
            },
            background,
        };
        let height = self.skip + self.height;
        match self.color {
            Some(_) => Frame::multicolor(&bitmap, height),
            None => Frame::hires(&bitmap, height),
        }
    }
}

const FLI_DESIGNER: Fli = Fli {
    load: 0x3c00,
    sizes: &[17409, 17410],
    bitmap: 0x6000,
    screens: 0x4000,
    color: Some(0x3c00),
    backgrounds: None,
    height: 200,
    skip: 0,
};
const FLI_GRAPH: Fli = Fli {
    load: 0x3b00,
    sizes: &[17474],
    backgrounds: Some(0x3b00),
    ..FLI_DESIGNER
};
const AFLI_EDITOR: Fli = Fli {
    load: 0x4000,
    sizes: &[16385],
    bitmap: 0x6000,
    screens: 0x4000,
    color: None,
    backgrounds: None,
    height: 200,
    skip: 0,
};
const HIRES_FLI_DESIGNER: Fli = Fli {
    load: 0x4000,
    sizes: &[16386],
    bitmap: 0x4000,
    screens: 0x6000,
    color: None,
    backgrounds: None,
    height: 112,
    skip: 0,
};
const HIRES_MANAGER: Fli = Fli {
    sizes: &[16385],
    height: 192,
    skip: 8,
    ..HIRES_FLI_DESIGNER
};

pub(super) fn decode_fli_designer(data: &[u8]) -> Result<Image, DecodeError> {
    FLI_DESIGNER.decode(data)
}

pub(super) fn decode_fli_graph(data: &[u8]) -> Result<Image, DecodeError> {
    FLI_GRAPH.decode(data)
}

pub(super) fn decode_afli_editor(data: &[u8]) -> Result<Image, DecodeError> {
    AFLI_EDITOR.decode(data)
}

pub(super) fn decode_hires_fli_designer(data: &[u8]) -> Result<Image, DecodeError> {
    HIRES_FLI_DESIGNER.decode(data)
}

pub(super) fn decode_hires_manager(data: &[u8]) -> Result<Image, DecodeError> {
    HIRES_MANAGER.decode(data)
}
