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

/// A texture for [`btx0`]: its name, its `TEXIMAGE_PARAM` without the data
/// offset, its data, and for the compressed format its block attributes.
pub(super) struct Tex<'a> {
    pub name: &'a str,
    pub param: u32,
    pub data: &'a [u8],
    pub attributes: &'a [u8],
}

fn dict(entries: &[Vec<u8>], names: &[&str]) -> Vec<u8> {
    let (count, unit) = (entries.len(), entries[0].len());
    let size = 8 + 4 + count * 4 + 4 + count * unit + count * 16;
    let mut out = alloc::vec![0, count as u8];
    out.extend_from_slice(&(size as u16).to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes());
    out.extend_from_slice(&((0xc + count * 4) as u16).to_le_bytes());
    out.resize(8 + 4 + count * 4, 0); // the search tree, not read
    out.extend_from_slice(&(unit as u16).to_le_bytes());
    out.extend_from_slice(&((4 + count * unit) as u16).to_le_bytes());
    for entry in entries {
        out.extend_from_slice(entry);
    }
    for name in names {
        let mut padded = name.as_bytes().to_vec();
        padded.resize(16, 0);
        out.extend_from_slice(&padded);
    }
    out
}

/// A BTX0 file with `textures` and `palettes` (a name and 16-bit colors).
pub(super) fn btx0(textures: &[Tex], palettes: &[(&str, &[u16])]) -> Vec<u8> {
    let (mut plain, mut compressed, mut attributes, mut colors) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut texture_entries = Vec::new();
    for texture in textures {
        let compressed_format = texture.param >> 26 & 7 == 5;
        let block = if compressed_format {
            &mut compressed
        } else {
            &mut plain
        };
        let offset = block.len();
        block.extend_from_slice(texture.data);
        block.resize(block.len().next_multiple_of(8), 0);
        if compressed_format {
            attributes.resize(offset / 2, 0);
            attributes.extend_from_slice(texture.attributes);
        }
        let mut entry = (texture.param | (offset / 8) as u32).to_le_bytes().to_vec();
        entry.extend_from_slice(&[0, 0, 0, 0x80]);
        texture_entries.push(entry);
    }
    let mut palette_entries = Vec::new();
    for (_, words) in palettes {
        let mut entry = ((colors.len() / 8) as u16).to_le_bytes().to_vec();
        entry.extend_from_slice(&[0, 0]);
        palette_entries.push(entry);
        colors.extend(words.iter().flat_map(|w| w.to_le_bytes()));
        colors.resize(colors.len().next_multiple_of(8), 0);
    }
    let texture_dict = dict(
        &texture_entries,
        &textures.iter().map(|t| t.name).collect::<Vec<_>>(),
    );
    let palette_dict = dict(
        &palette_entries,
        &palettes.iter().map(|p| p.0).collect::<Vec<_>>(),
    );
    let texture_dict_at = 0x3c;
    let palette_dict_at = texture_dict_at + texture_dict.len();
    let plain_at = palette_dict_at + palette_dict.len();
    let compressed_at = plain_at + plain.len();
    let attributes_at = compressed_at + compressed.len();
    let colors_at = attributes_at + attributes.len();
    let mut chunk = alloc::vec![0u8; 0x3c];
    chunk[..4].copy_from_slice(b"TEX0");
    let set16 = |chunk: &mut Vec<u8>, at: usize, v: usize| {
        chunk[at..at + 2].copy_from_slice(&(v as u16).to_le_bytes())
    };
    let set32 = |chunk: &mut Vec<u8>, at: usize, v: usize| {
        chunk[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes())
    };
    set16(&mut chunk, 0xe, texture_dict_at);
    set32(&mut chunk, 0x14, plain_at);
    set16(&mut chunk, 0x1e, texture_dict_at);
    set32(&mut chunk, 0x24, compressed_at);
    set32(&mut chunk, 0x28, attributes_at);
    set32(&mut chunk, 0x34, palette_dict_at);
    set32(&mut chunk, 0x38, colors_at);
    for part in [
        &texture_dict,
        &palette_dict,
        &plain,
        &compressed,
        &attributes,
        &colors,
    ] {
        chunk.extend_from_slice(part);
    }
    let size = chunk.len();
    set32(&mut chunk, 4, size);
    let mut file = b"BTX0\xff\xfe\x01\x00".to_vec();
    file.extend_from_slice(&((0x14 + size) as u32).to_le_bytes());
    file.extend_from_slice(&0x10u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&0x14u32.to_le_bytes());
    file.extend_from_slice(&chunk);
    file
}
