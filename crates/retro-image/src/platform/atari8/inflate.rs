//! zlib/DEFLATE decompression, used by Graph2Font's `G2FZLIB` files.
//!
//! Sources: RFC 1950, "ZLIB Compressed Data Format Specification version
//! 3.3" (<https://www.rfc-editor.org/rfc/rfc1950>) and RFC 1951, "DEFLATE
//! Compressed Data Format Specification version 1.3"
//! (<https://www.rfc-editor.org/rfc/rfc1951>). Written from the RFCs alone.
//!
//! Nothing else in the crate needs it yet; if another format does, this
//! module can move to `codec`.

use alloc::vec::Vec;

/// Decompresses a zlib stream (RFC 1950) of at most `limit` bytes. `None`
/// for anything malformed, a preset dictionary, an Adler-32 mismatch or
/// output beyond `limit`.
pub(super) fn zlib(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    let (&cmf, rest) = data.split_first()?;
    let (&flg, body) = rest.split_first()?;
    // Method 8 (deflate), window up to 32K, header check, no dictionary.
    if cmf & 0x0f != 8 || cmf >> 4 > 7 || (u16::from(cmf) << 8 | u16::from(flg)) % 31 != 0 {
        return None;
    }
    if flg & 0x20 != 0 {
        return None;
    }
    let mut input = Bits::new(body);
    let out = inflate(&mut input, limit)?;
    let trailer = input.aligned_rest();
    let expected = crate::bytes::be32(trailer, 0)?;
    (adler32(&out) == expected).then_some(out)
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &byte in chunk {
            a += u32::from(byte);
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    b << 16 | a
}

/// Least-significant-bit-first reader (RFC 1951 section 3.1.1).
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn bit(&mut self) -> Option<u32> {
        let byte = self.data.get(self.pos / 8)?;
        let bit = u32::from(byte >> (self.pos % 8)) & 1;
        self.pos += 1;
        Some(bit)
    }

    /// `count` bits, first bit least significant.
    fn bits(&mut self, count: u32) -> Option<u32> {
        let mut value = 0;
        for i in 0..count {
            value |= self.bit()? << i;
        }
        Some(value)
    }

    /// The bytes after the current partial byte.
    fn aligned_rest(&self) -> &'a [u8] {
        self.data.get(self.pos.div_ceil(8)..).unwrap_or(&[])
    }
}

/// A canonical Huffman code (RFC 1951 section 3.2.2), decoded by walking
/// code lengths one bit at a time.
struct Huffman {
    /// Number of codes of each length 0-15.
    counts: [u16; 16],
    /// Symbols ordered by code length, then by symbol value.
    symbols: Vec<u16>,
}

impl Huffman {
    /// `None` if the lengths oversubscribe the code space.
    fn new(lengths: &[u8]) -> Option<Self> {
        let mut counts = [0u16; 16];
        for &length in lengths {
            counts[usize::from(length)] += 1;
        }
        counts[0] = 0;
        let mut left = 1i32;
        for &count in &counts[1..] {
            left = left * 2 - i32::from(count);
            if left < 0 {
                return None;
            }
        }
        let mut symbols = Vec::with_capacity(lengths.len());
        for length in 1..16 {
            for (symbol, &l) in lengths.iter().enumerate() {
                if usize::from(l) == length {
                    symbols.push(symbol as u16);
                }
            }
        }
        Some(Self { counts, symbols })
    }

    fn decode(&self, input: &mut Bits) -> Option<u16> {
        // `code` is the code read so far; `first` the first code of the
        // current length; `index` the position of that code in `symbols`.
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for length in 1..16 {
            code |= input.bit()? as i32;
            let count = i32::from(self.counts[length]);
            if code - first < count {
                return self.symbols.get((index + code - first) as usize).copied();
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        None
    }
}

const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
/// Order of the code length code lengths (RFC 1951 section 3.2.7).
const CODE_LENGTH_ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Decompresses raw DEFLATE blocks (RFC 1951) up to the final block.
fn inflate(input: &mut Bits, limit: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let last = input.bit()? == 1;
        match input.bits(2)? {
            0 => stored(input, &mut out, limit)?,
            1 => {
                let (literals, distances) = fixed_codes()?;
                compressed(input, &mut out, limit, &literals, &distances)?;
            }
            2 => {
                let (literals, distances) = dynamic_codes(input)?;
                compressed(input, &mut out, limit, &literals, &distances)?;
            }
            _ => return None,
        }
        if last {
            return Some(out);
        }
    }
}

/// A stored block: byte-aligned LEN, NLEN, then LEN bytes.
fn stored(input: &mut Bits, out: &mut Vec<u8>, limit: usize) -> Option<()> {
    let start = input.pos.div_ceil(8);
    let len = usize::from(crate::bytes::le16(input.data, start)?);
    let nlen = crate::bytes::le16(input.data, start + 2)?;
    if len as u16 != !nlen || out.len() + len > limit {
        return None;
    }
    out.extend_from_slice(input.data.get(start + 4..start + 4 + len)?);
    input.pos = (start + 4 + len) * 8;
    Some(())
}

fn fixed_codes() -> Option<(Huffman, Huffman)> {
    let mut lengths = [0u8; 288];
    lengths[..144].fill(8);
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    lengths[280..].fill(8);
    Some((Huffman::new(&lengths)?, Huffman::new(&[5; 30])?))
}

fn dynamic_codes(input: &mut Bits) -> Option<(Huffman, Huffman)> {
    let literal_count = input.bits(5)? as usize + 257;
    let distance_count = input.bits(5)? as usize + 1;
    let code_length_count = input.bits(4)? as usize + 4;
    let mut code_lengths = [0u8; 19];
    for &index in &CODE_LENGTH_ORDER[..code_length_count] {
        code_lengths[index] = input.bits(3)? as u8;
    }
    let code_length_code = Huffman::new(&code_lengths)?;
    let mut lengths = Vec::with_capacity(literal_count + distance_count);
    while lengths.len() < literal_count + distance_count {
        let (value, repeat) = match code_length_code.decode(input)? {
            symbol @ 0..=15 => (symbol as u8, 1),
            16 => (*lengths.last()?, 3 + input.bits(2)?),
            17 => (0, 3 + input.bits(3)?),
            18 => (0, 11 + input.bits(7)?),
            _ => return None,
        };
        for _ in 0..repeat {
            lengths.push(value);
        }
    }
    if lengths.len() != literal_count + distance_count || lengths[256] == 0 {
        return None;
    }
    let (literals, distances) = lengths.split_at(literal_count);
    Some((Huffman::new(literals)?, Huffman::new(distances)?))
}

fn compressed(
    input: &mut Bits,
    out: &mut Vec<u8>,
    limit: usize,
    literals: &Huffman,
    distances: &Huffman,
) -> Option<()> {
    loop {
        let symbol = usize::from(literals.decode(input)?);
        match symbol {
            0..=255 => {
                if out.len() >= limit {
                    return None;
                }
                out.push(symbol as u8);
            }
            256 => return Some(()),
            _ => {
                let i = symbol - 257;
                let length = usize::from(*LENGTH_BASE.get(i)?)
                    + input.bits(u32::from(LENGTH_EXTRA[i]))? as usize;
                let d = usize::from(distances.decode(input)?);
                let distance = usize::from(*DISTANCE_BASE.get(d)?)
                    + input.bits(u32::from(DISTANCE_EXTRA[d]))? as usize;
                if distance > out.len() || out.len() + length > limit {
                    return None;
                }
                let start = out.len() - distance;
                for k in 0..length {
                    out.push(out[start + k]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_block() {
        // zlib header, final stored block "abc", Adler-32.
        let data = [
            0x78, 0x01, 0x01, 0x03, 0x00, 0xfc, 0xff, b'a', b'b', b'c', 0x02, 0x4d, 0x01, 0x27,
        ];
        assert_eq!(zlib(&data, 100).unwrap(), b"abc");
        assert_eq!(zlib(&data, 2), None);
    }

    #[test]
    fn fixed_block_with_match() {
        // Python: zlib.compress(b"aaaaaaaaaa")
        let data = [
            0x78, 0x9c, 0x4b, 0x4c, 0x84, 0x01, 0x00, 0x14, 0xe1, 0x03, 0xcb,
        ];
        assert_eq!(zlib(&data, 100).unwrap(), b"aaaaaaaaaa");
    }

    #[test]
    fn rejects_bad_checksum_and_header() {
        let mut data = [
            0x78, 0x9c, 0x4b, 0x4c, 0x84, 0x01, 0x00, 0x14, 0xe1, 0x03, 0xcb,
        ];
        data[10] ^= 1;
        assert_eq!(zlib(&data, 100), None);
        assert_eq!(zlib(&[0x78, 0x9d], 100), None);
    }
}
