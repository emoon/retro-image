//! Morton (Z-order) pixel addressing: bit interleaving of the x and y
//! coordinates, <https://en.wikipedia.org/wiki/Z-order_curve>.
//!
//! This file holds no format knowledge. Which axis takes the even bits, and
//! whether the order covers a whole texture or only an 8x8 tile, is each
//! format's business and is stated by the decoder that calls it.

/// Index of the pixel at (`x`, `y`) in a Z-order block: the bits of `x` take
/// the even bit positions of the result and the bits of `y` the odd ones.
/// Swap the arguments for a format that puts `y` first. Only the low 16 bits
/// of each coordinate are used.
pub(crate) fn morton_index(x: u32, y: u32) -> u32 {
    spread_bits(x) | spread_bits(y) << 1
}

/// Moves bit `n` of the low 16 bits of `v` to bit `2 * n`.
fn spread_bits(v: u32) -> u32 {
    let mut v = v & 0xffff;
    v = (v | v << 8) & 0x00ff_00ff;
    v = (v | v << 4) & 0x0f0f_0f0f;
    v = (v | v << 2) & 0x3333_3333;
    v = (v | v << 1) & 0x5555_5555;
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_takes_the_even_bits_and_y_the_odd_bits() {
        assert_eq!(morton_index(0, 0), 0);
        assert_eq!(morton_index(1, 0), 1);
        assert_eq!(morton_index(0, 1), 2);
        assert_eq!(morton_index(1, 1), 3);
        assert_eq!(morton_index(2, 0), 4);
        assert_eq!(morton_index(7, 7), 63);
        assert_eq!(morton_index(0xffff, 0xffff), 0xffff_ffff);
    }
}
