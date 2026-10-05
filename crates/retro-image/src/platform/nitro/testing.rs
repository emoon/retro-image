//! Builders for the files the unit tests of the Nitro decoders read, made from
//! the layouts in the header of `nitro.rs`.

use alloc::vec::Vec;

/// A file of type `magic` with `sections` (magic and data of each).
pub(super) fn file(magic: &[u8; 4], sections: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let size = 0x10 + sections.iter().map(|(_, s)| 8 + s.len()).sum::<usize>();
    let mut out = Vec::new();
    out.extend_from_slice(magic);
    out.extend_from_slice(&[0xff, 0xfe, 0x00, 0x01]); // byte order mark, version
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&0x10u16.to_le_bytes());
    out.extend_from_slice(&(sections.len() as u16).to_le_bytes());
    for (name, data) in sections {
        out.extend_from_slice(*name);
        out.extend_from_slice(&((8 + data.len()) as u32).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

/// An NCLR whose palette section has depth code `depth` (3 = 4 bpp, 4 = 8 bpp)
/// and the colors `words`.
pub(super) fn nclr(depth: u32, words: &[u16]) -> Vec<u8> {
    let size = (words.len() * 2) as u32;
    let mut body = Vec::new();
    for field in [depth, 0, size, 0x10] {
        body.extend_from_slice(&field.to_le_bytes());
    }
    for word in words {
        body.extend_from_slice(&word.to_le_bytes());
    }
    file(b"RLCN", &[(b"TTLP", &body)])
}

/// An NCGR with depth code `depth`, `tiles` (height, width) as stored,
/// the OBJ mapping mode, character kind (1 = bitmap) and graphics `data`.
pub(super) fn ncgr(depth: u32, tiles: (u16, u16), mapping: u32, kind: u32, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&tiles.0.to_le_bytes());
    body.extend_from_slice(&tiles.1.to_le_bytes());
    for field in [depth, mapping, kind, data.len() as u32, 0x18] {
        body.extend_from_slice(&field.to_le_bytes());
    }
    body.extend_from_slice(data);
    file(b"RGCN", &[(b"RAHC", &body)])
}

/// An NSCR of `width` x `height` pixels, screen format `format` (0 text,
/// 1 affine, 2 affine extended) and entry `data`.
pub(super) fn nscr(width: u16, height: u16, format: u16, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for field in [width, height, 0, format] {
        body.extend_from_slice(&field.to_le_bytes());
    }
    body.extend_from_slice(&(data.len() as u32).to_le_bytes());
    body.extend_from_slice(data);
    file(b"RCSN", &[(b"NRCS", &body)])
}
