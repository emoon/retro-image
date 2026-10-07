//! Desktop integration generated from the format list: a shared-mime-info
//! package and a freedesktop `.thumbnailer` entry, so file managers (e.g.
//! flea) classify retro images as images and thumbnail them with this CLI.
//!
//! Sources:
//! - MIME package XML (`mime-info`, `mime-type`, `comment`, `generic-icon`,
//!   `glob` with `weight`, default weight 50) and the
//!   `~/.local/share/mime/packages` location: Shared MIME-info Database
//!   specification,
//!   <https://specifications.freedesktop.org/shared-mime-info/latest/>.
//! - `.thumbnailer` entries (`[Thumbnailer Entry]` with `TryExec`, `Exec`
//!   using `%i` input, `%o` output and `%s` size, `MimeType`, installed in
//!   `~/.local/share/thumbnailers`): freedesktop has no official
//!   specification for these files; the convention (from GNOME, shared by
//!   Xfce's Tumbler and PCManFM) is described in the Tumbler documentation,
//!   <https://docs.xfce.org/xfce/tumbler/available_plugins>, and the Arch
//!   wiki, <https://wiki.archlinux.org/title/File_manager_functionality>.
//!   Where the thumbnails end up is the Thumbnail Managing Standard,
//!   <https://specifications.freedesktop.org/thumbnail/latest/>.
//!
//! Install with:
//!
//! ```sh
//! retro-image --mime-xml > ~/.local/share/mime/packages/retro-image.xml
//! update-mime-database ~/.local/share/mime
//! retro-image --thumbnailer > ~/.local/share/thumbnailers/retro-image.thumbnailer
//! ```

use std::collections::BTreeSet;
use std::fmt::Write;

/// The single MIME type covering every supported extension.
const MIME_TYPE: &str = "image/x-retro-image";

/// Below shared-mime-info's default of 50, so established types for shared
/// extensions (e.g. `.pic`, `.scr`, `.img`) keep priority.
const GLOB_WEIGHT: u32 = 30;

/// Extensions left out of the package because ordinary files carry them too
/// (`my.cnf`, `report.tpl`, Minecraft's `.mcr`) and no system type claims them,
/// so the glob would retype those files as retro images. The formats still
/// decode by name; they just aren't claimed by the desktop.
const TOO_GENERIC: &[&str] = &[
    "anim", "cnf", "fix", "gl", "icon", "icons", "imag", "mcr", "srm", "tem", "tpl", "vms",
];

/// Every extension of every format except [`TOO_GENERIC`], lower-case, sorted
/// and deduplicated.
fn extensions() -> BTreeSet<String> {
    retro_image::formats()
        .flat_map(|f| f.extensions().iter())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| !TOO_GENERIC.contains(&e.as_str()))
        .collect()
}

/// One line per format: platform, name, extensions, flags.
pub fn format_list() -> String {
    let mut out = String::new();
    for f in retro_image::formats() {
        let mut flags = Vec::new();
        if f.has_signature() {
            flags.push("signature");
        }
        if f.uses_companions() {
            flags.push("companions");
        }
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}",
            f.platform(),
            f.name(),
            f.extensions().join(","),
            flags.join(",")
        );
    }
    out
}

pub fn mime_xml() -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <mime-info xmlns=\"http://www.freedesktop.org/standards/shared-mime-info\">\n",
    );
    let _ = writeln!(out, "  <mime-type type=\"{MIME_TYPE}\">");
    out.push_str("    <comment>Retro computer image</comment>\n");
    out.push_str("    <generic-icon name=\"image-x-generic\"/>\n");
    for ext in extensions() {
        let _ = writeln!(
            out,
            "    <glob pattern=\"*.{}\" weight=\"{GLOB_WEIGHT}\"/>",
            xml_escape(&ext)
        );
    }
    out.push_str("  </mime-type>\n</mime-info>\n");
    out
}

pub fn thumbnailer() -> String {
    format!(
        "[Thumbnailer Entry]\n\
         TryExec=retro-image\n\
         Exec=retro-image -i %i -o %o -s %s\n\
         MimeType={MIME_TYPE};\n"
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_xml_lists_every_extension_once_and_escapes() {
        let xml = mime_xml();
        assert!(xml.contains("<glob pattern=\"*.scr\" weight=\"30\"/>"));
        assert_eq!(xml.matches("pattern=\"*.scr\"").count(), 1);
        assert!(!xml.contains("*.b&w"), "ampersands must be escaped");
    }

    #[test]
    fn generic_extensions_are_not_claimed() {
        let xml = mime_xml();
        for ext in TOO_GENERIC {
            assert!(
                retro_image::formats().any(|f| f.extensions().contains(ext)),
                "{ext} is on the denylist but no format uses it"
            );
            assert!(
                !xml.contains(&format!("pattern=\"*.{ext}\"")),
                "{ext} is claimed"
            );
        }
    }

    #[test]
    fn thumbnailer_runs_the_cli_with_size() {
        assert!(thumbnailer().contains("Exec=retro-image -i %i -o %o -s %s"));
    }
}
