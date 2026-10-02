//! Commodore 64, VIC-20, C16/116/Plus4 and C128.
//!
//! Layout sources are listed per submodule; the platform survey is
//! `docs/formats/commodore.md`.
//!
//! Generic C64 pictures (`.vic`, listed at
//! <http://fileformats.archiveteam.org/wiki/Commodore_graphics_formats>) are
//! memory dumps in the other formats' layouts. Which layouts occur (including
//! FLI Graph dumps up to `$7FFF` and a 33602-byte Gunpaint-layout IFLI dump)
//! was found by inspecting sample files and comparing with `recoil2png` output.

mod bitmap;
mod fli;
mod godot;
mod ifli;
mod interlace;
mod prg;
mod printfox;
mod ted;
mod unpack;
mod vic2;

use crate::{DecodeError, Format, Image};

type Decoder = fn(&[u8]) -> Result<Image, DecodeError>;

const C64: &str = "Commodore 64";
const PLUS4: &str = "Commodore 16/116/Plus4";

pub(super) static FORMATS: &[Format] = &[
    Format::new(
        C64,
        "Koala Painter",
        &["koa", "kla", "gig"],
        bitmap::decode_koala,
    ),
    Format::new(
        C64,
        "Koala Painter (compressed)",
        &["gg"],
        bitmap::decode_koala_packed,
    ),
    Format::new(
        C64,
        "Run Paint (multicolor)",
        &["rpm"],
        bitmap::decode_run_paint,
    ),
    Format::new(
        C64,
        "Interpaint (lores)",
        &["ipt", "lre"],
        bitmap::decode_interpaint_lores,
    ),
    Format::new(
        C64,
        "Create with Garfield",
        &["cwg"],
        bitmap::decode_create_with_garfield,
    ),
    Format::new(C64, "CDU-Paint", &["cdu"], bitmap::decode_cdu_paint),
    Format::new(C64, "Amica Paint", &["ami"], bitmap::decode_amica),
    Format::new(
        C64,
        "Wigmore Artist 64",
        &["a64", "wig"],
        bitmap::decode_artist_64,
    ),
    Format::new(
        C64,
        "Blazing Paddles",
        &["pi", "bpl"],
        bitmap::decode_blazing_paddles,
    ),
    Format::new(C64, "Vidcom 64", &["vid"], bitmap::decode_vidcom),
    Format::new(
        C64,
        "Image System (multicolor)",
        &["ism"],
        bitmap::decode_image_system_multi,
    ),
    Format::new(
        C64,
        "Advanced Art Studio",
        &["ocp", "mpi", "mpic"],
        bitmap::decode_advanced_art_studio,
    ),
    Format::new(C64, "Saracen Paint", &["sar"], bitmap::decode_saracen_paint),
    Format::new(C64, "Drazpaint", &["drz", "drp"], bitmap::decode_drazpaint),
    Format::new(
        C64,
        "Drazpaint (compressed)",
        &["drz", "drp"],
        bitmap::decode_drazpaint_packed,
    ),
    Format::new(
        C64,
        "Art Studio",
        &["aas", "art", "hpi"],
        bitmap::decode_art_studio,
    ),
    Format::new(
        C64,
        "Interpaint (hires)",
        &["iph", "gig", "hpi", "hre"],
        bitmap::decode_interpaint_hires,
    ),
    Format::new(
        C64,
        "Image System (hires)",
        &["ish"],
        bitmap::decode_image_system_hires,
    ),
    Format::new(C64, "Hi-Eddi", &["hed"], bitmap::decode_hi_eddi),
    Format::new(C64, "Doodle", &["dd", "ddp"], bitmap::decode_doodle),
    Format::new(
        C64,
        "Doodle (compressed)",
        &["jj"],
        bitmap::decode_doodle_packed,
    ),
    Format::new(
        C64,
        "Hires-Bitmap",
        &["hbm", "hir", "hpi", "fgs", "rpo"],
        bitmap::decode_mono,
    ),
    Format::new(
        C64,
        "Gigapaint (hires)",
        &["gih"],
        bitmap::decode_gigapaint_hires,
    ),
    Format::new(
        C64,
        "FLI Designer",
        &["fd2", "fli"],
        fli::decode_fli_designer,
    ),
    Format::new(
        C64,
        "FLI Graph",
        &["bml", "flg", "fli"],
        fli::decode_fli_graph,
    ),
    Format::new(C64, "Flip", &["fbi"], fli::decode_flip),
    Format::new(C64, "AFLI-editor", &["afl"], fli::decode_afli_editor),
    Format::new(
        C64,
        "Hires FLI Designer",
        &["hfc", "hfd"],
        fli::decode_hires_fli_designer,
    ),
    Format::new(C64, "Hires Manager", &["him"], fli::decode_hires_manager),
    Format::new(C64, "Drazlace", &["drl", "dlp"], interlace::decode_drazlace),
    Format::new(C64, "True Paint", &["mci"], interlace::decode_true_paint),
    Format::new(
        C64,
        "Hires-Interlace",
        &["hlf"],
        interlace::decode_hires_interlace,
    ),
    Format::new(C64, "Giga-CAD", &["gcd", "mon"], bitmap::decode_giga_cad),
    Format::new(C64, "Paint Magic", &["pmg"], bitmap::decode_paint_magic),
    Format::new(
        C64,
        "Hi-Pic Creator",
        &["hpc"],
        bitmap::decode_hi_pic_creator,
    ),
    Format::new(C64, "Gunpaint", &["gun", "ifl"], ifli::decode_gunpaint),
    Format::new(C64, "Funpaint II", &["fun", "fp2"], ifli::decode_funpaint),
    Format::new(C64, "Pixel Perfect", &["pp"], ifli::decode_pixel_perfect),
    Format::new(
        C64,
        "Pixel Perfect (compressed)",
        &["ppp"],
        ifli::decode_pixel_perfect_packed,
    ),
    Format::new(C64, "ECI Graphic Editor", &["eci"], ifli::decode_eci),
    Format::new(
        C64,
        "Micro Illustrator",
        &["mil"],
        bitmap::decode_micro_illustrator,
    ),
    Format::new(C64, "GoDot 4Bit", &["4bt"], godot::decode_4bt),
    Format::new(C64, "GoDot 4Bit clip", &["clp"], godot::decode_clp),
    Format::new(C64, "Printfox screen", &["bs"], printfox::decode_bs),
    Format::new(C64, "Printfox large picture", &["gb"], printfox::decode_gb),
    Format::new(C64, "Pagefox", &["pg"], printfox::decode_pg),
    Format::new(
        C64,
        "Star Painter",
        &["gr", "cs"],
        printfox::decode_star_painter,
    ),
    Format::new(C64, "Generic C64 picture", &["vic"], decode_generic),
    Format::new(PLUS4, "Botticelli", &["p4i"], ted::decode_p4i),
];

/// `.vic`: a memory dump in one of the unpacked C64 layouts, told apart by
/// file size (and load address for the 33602-byte IFLI dumps).
fn decode_generic(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() == 33602 && data[..2] != [0x00, 0x3c] {
        return ifli::decode_gunpaint_dump(data);
    }
    const LAYOUTS: &[Decoder] = &[
        bitmap::decode_koala,
        bitmap::decode_art_studio,
        bitmap::decode_advanced_art_studio,
        fli::decode_fli_designer,
        fli::decode_fli_graph,
        ifli::decode_gunpaint,
        ifli::decode_funpaint,
        ifli::decode_pixel_perfect,
        bitmap::decode_blazing_paddles,
        interlace::decode_drazlace,
    ];
    LAYOUTS
        .iter()
        .find_map(|decode| decode(data).ok())
        .ok_or(DecodeError::Unrecognized)
}
