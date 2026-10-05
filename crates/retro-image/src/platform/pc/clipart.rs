//! What the DOS clip-art libraries (Print Shop, PrintMaster, PrintPartner)
//! share besides the sheet their pictures are laid out on
//! ([`crate::sheet`]): the pictures are 1-bit, black on white, and DOS tools
//! padded a library with zero fill or `1A` end-of-file marks. The facts are
//! those cited by `printshop.rs`, `printmaster.rs` and `printpartner.rs`.

use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

/// Whether `tail`, the bytes after a library's last picture, is only the
/// zero fill or the `1A` end-of-file marks that DOS tools left there.
pub(super) fn is_padding(tail: &[u8]) -> bool {
    tail.iter().all(|&b| b == 0 || b == 0x1a)
}

/// A picture of `width` x `height` pixels from 1-bit rows of whole bytes,
/// most significant bit leftmost, a set bit black.
pub(super) fn black_on_white(
    width: usize,
    height: usize,
    rows: &[u8],
) -> Result<Image, DecodeError> {
    check_size(width, height)?;
    Image::from_bits(
        width as u32,
        height as u32,
        rows,
        width.div_ceil(8),
        BitOrder::MsbFirst,
        [0xffffff, 0x000000],
    )
}
