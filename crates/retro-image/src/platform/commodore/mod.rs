//! Commodore 64, VIC-20, C16/116/Plus4 and C128.
//!
//! Layout sources are listed per submodule; the platform survey is
//! `docs/formats/commodore.md`.

mod bitmap;
mod fli;
mod ifli;
mod interlace;
mod prg;
mod unpack;
mod vic2;

use crate::Format;

const C64: &str = "Commodore 64";

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
];
