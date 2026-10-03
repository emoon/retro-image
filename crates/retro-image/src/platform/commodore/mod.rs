//! Commodore 64, VIC-20, C16/116/Plus4 and C128.
//!
//! Layout sources are listed per submodule; the platform survey is
//! `docs/research/commodore.md`.
//!
//! Generic C64 pictures (`.vic`, listed at
//! <http://fileformats.archiveteam.org/wiki/Commodore_graphics_formats>) are
//! memory dumps in the other formats' layouts. Which layouts occur (including
//! FLI Graph dumps up to `$7FFF` and a 33602-byte Gunpaint-layout IFLI dump)
//! was found by inspecting sample files and comparing with `recoil2png` output.

mod bitmap;
mod c128;
mod cgx;
mod charpad;
mod charset;
mod cle;
mod ecp;
mod emc;
mod flf;
mod fli;
mod godot;
mod hcb;
mod ifli;
mod interlace;
mod loadstar;
mod logo;
mod mufli;
mod mwin;
mod nufli;
mod petscii;
mod prg;
mod printfox;
mod printmaster;
mod she;
mod sprites;
mod superhires;
mod ted;
mod ufli;
mod unpack;
mod vhi;
mod vic2;
mod vic20;
mod viewer;
mod xfl;

use crate::{DecodeError, Format, Image};

type Decoder = fn(&[u8]) -> Result<Image, DecodeError>;

const C64: &str = "Commodore 64";
const PLUS4: &str = "Commodore 16/116/Plus4";
const C128: &str = "Commodore 128";
const VIC20: &str = "Commodore VIC-20";

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
    Format::new(C64, "Picasso 64", &["p64"], bitmap::decode_picasso_64),
    Format::new(C64, "Cheese", &["che"], bitmap::decode_cheese),
    Format::new(
        C64,
        "Rainbow Painter",
        &["rp"],
        bitmap::decode_rainbow_painter,
    ),
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
    )
    .signature(),
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
    Format::new(C64, "CFLI Designer", &["cfli"], fli::decode_cfli),
    Format::new(C64, "FLI Profi", &["fpr"], fli::decode_fli_profi),
    Format::new(C64, "Drazlace", &["drl", "dlp"], interlace::decode_drazlace),
    Format::new(
        C64,
        "Drazlace (compressed)",
        &["drl", "dlp"],
        interlace::decode_drazlace_packed,
    )
    .signature(),
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
    Format::new(C64, "Funpaint II", &["fun", "fp2"], ifli::decode_funpaint).signature(),
    Format::new(C64, "Pixel Perfect", &["pp"], ifli::decode_pixel_perfect),
    Format::new(
        C64,
        "Pixel Perfect (compressed)",
        &["ppp"],
        ifli::decode_pixel_perfect_packed,
    ),
    Format::new(C64, "ECI Graphic Editor", &["eci"], ifli::decode_eci),
    Format::new(C64, "Flash FLI", &["ffli", "ffl"], ifli::decode_ffli),
    Format::new(C64, "Big FLI", &["bfli", "bfl"], ifli::decode_bfli),
    Format::new(
        C64,
        "Micro Illustrator",
        &["mil"],
        bitmap::decode_micro_illustrator,
    ),
    Format::new(C64, "GoDot 4Bit", &["4bt"], godot::decode_4bt).signature(),
    Format::new(C64, "GoDot 4Bit clip", &["clp"], godot::decode_clp).signature(),
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
    Format::new(C64, "Loadstar SHP", &["shp"], loadstar::decode_shp),
    Format::new(C64, "Logo Painter", &["lp3"], logo::decode_logo_painter),
    Format::new(C64, "Character set", &["64c"], charset::decode_font),
    Format::new(C64, "SEUCK font", &["g"], charset::decode_seuck_font),
    Format::new(
        C64,
        "Star Painter font",
        &["zs"],
        charset::decode_star_painter_font,
    ),
    Format::new(C64, "SpritePad", &["spd"], sprites::decode_spd).signature(),
    Format::new(
        C64,
        "SpritePad (headerless)",
        &["spd"],
        sprites::decode_spd_raw,
    ),
    Format::new(C64, "SEUCK sprites", &["a"], sprites::decode_seuck),
    Format::new(C64, "Commodore Grafix", &["cgx"], cgx::decode_cgx).signature(),
    Format::new(C64, "CharPad", &["ctm"], charpad::decode_ctm).signature(),
    Format::new(C64, "C64 OS screenshot", &["pet"], petscii::decode_c64os).signature(),
    Format::new(
        C64,
        "PETSCII Editor",
        &["pet"],
        petscii::decode_petscii_editor,
    ),
    Format::with_companions(
        C64,
        "PETSCII Editor (screen + colours)",
        &["scr"],
        petscii::decode_scr_col,
    ),
    Format::new(C64, "PETSCII BOT", &["pbot"], petscii::decode_pbot),
    Format::new(
        C64,
        "Super Hires Interlace Editor",
        &["shi"],
        superhires::decode_shi,
    ),
    Format::new(
        C64,
        "Super Hires Interlace FLI Editor",
        &["sif"],
        superhires::decode_sif,
    ),
    Format::new(
        C64,
        "Super Hires FLI Editor",
        &["shf"],
        superhires::decode_shf,
    ),
    Format::new(C64, "SHF-XL Edit", &["shx"], superhires::decode_shx),
    Format::new(C64, "NUFLI Editor", &["nuf"], nufli::decode_nufli),
    Format::new(C64, "UFLI-editor", &["ufl"], ufli::decode_ufli),
    Format::new(PLUS4, "Botticelli", &["p4i"], ted::decode_p4i),
    Format::new(C128, "VDC BitMap", &["vbm", "bm"], c128::decode_vbm).signature(),
    Format::new(VIC20, "MiniPaint", &["mg"], vic20::decode_minipaint),
    Format::new(VIC20, "Best Paint", &["bp"], vic20::decode_best_paint),
    Format::with_companions(VIC20, "Picasso", &["pic0"], vic20::decode_picasso),
    Format::new(C128, "BASIC 8", &["ip", "brus", "pict"], c128::decode_brus).signature(),
    // Wave 5: C64
    Format::new(C64, "Centauri Logo-Editor", &["cle"], cle::decode_cle),
    Format::new(C64, "Hires-Editor", &["het"], bitmap::decode_hires_editor),
    Format::new(
        C64,
        "Interlace Hires Editor",
        &["ihe"],
        interlace::decode_interlace_hires_editor,
    ),
    Format::new(
        C64,
        "Multi-Lace Editor",
        &["mle"],
        interlace::decode_multi_lace,
    ),
    Format::new(
        C64,
        "Dolphin Ed",
        &["dol", "bed"],
        bitmap::decode_dolphin_ed,
    ),
    Format::new(
        C64,
        "ECI Graphic Editor (compressed)",
        &["ecp"],
        ecp::decode_ecp,
    ),
    Format::new(
        C64,
        "Face Painter",
        &["fcp", "fpt"],
        bitmap::decode_face_painter,
    ),
    // Wave 5: FLF
    Format::new(C64, "Turbo Rascal Syntax Error", &["flf"], flf::decode_c64).signature(),
    Format::new(
        VIC20,
        "Turbo Rascal Syntax Error",
        &["flf"],
        flf::decode_vic20,
    )
    .signature(),
    // Wave 5b: C64
    Format::new(C64, "EMC-editor", &["emc"], emc::decode_emc),
    Format::new(C64, "MUFLI Editor", &["muf"], mufli::decode_muf),
    Format::new(C64, "MUIFLI Editor", &["mui"], mufli::decode_mui),
    Format::new(
        C64,
        "MUFLI Editor (compressed)",
        &["mup"],
        mufli::decode_mup,
    ),
    Format::new(
        C64,
        "Art Studio window",
        &["mwi", "mwin"],
        mwin::decode_mwin,
    ),
    // Wave 6: C64 NUFLI packed
    Format::new(
        C64,
        "NUFLI Editor (compressed)",
        &["nup"],
        nufli::decode_nup,
    ),
    // Wave 6: C64 FLI Editor + HCB
    Format::new(C64, "FLI Editor", &["fed"], fli::decode_fed),
    Format::new(C64, "HCB-editor", &["hcb"], hcb::decode_hcb),
    // Wave 6: C64 PetDraw and Super Hires Editor
    Format::new(C64, "PetDraw64", &["pdr"], petscii::decode_petdraw),
    Format::new(C64, "Super Hires Editor", &["she"], she::decode_she),
    // Wave 6: C64 VHI + X-FLI
    Format::new(
        C64,
        "Vertical Hires Interlace Editor",
        &["vhi"],
        vhi::decode_vhi,
    ),
    // Wave 6: C64 X-FLI
    Format::new(C64, "X-FLI Editor", &["xfl"], xfl::decode_xfl),
    Format::new(C64, "PrintMaster", &["gra"], printmaster::decode_gra),
    // Self-displaying PRGs
    Format::new(
        C64,
        "Self-displaying PETSCII",
        &["prg"],
        petscii::decode_petscii_prg,
    )
    .signature(),
    Format::new(
        C64,
        "Koala viewer (10500 bytes)",
        &["prg"],
        viewer::decode_10500,
    )
    .signature(),
    Format::new(
        C64,
        "Koala viewer (10608 bytes)",
        &["prg"],
        viewer::decode_10608,
    )
    .signature(),
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
