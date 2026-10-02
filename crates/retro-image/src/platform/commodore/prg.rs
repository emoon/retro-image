//! C64 program files: a 2-byte load address followed by a memory image.
//!
//! Sources: "The first two bytes in a PRG file are the memory address that
//! the file should be loaded into (in little-endian format)",
//! <http://fileformats.archiveteam.org/wiki/Commodore_64_binary_executable>;
//! C64-Wiki `LOAD`, <https://www.c64-wiki.com/wiki/LOAD> (`,1` loads to the
//! address in the first two bytes); Peter Schepers, "Standard C64 BITMAP
//! files", <http://ist.uwaterloo.ca/~schepers/formats/BITMAP.TXT> (file
//! offset = address - load address + 2).

/// A memory image addressed by C64 address. The file's own load address is
/// ignored: `load` is the format's documented one (some tools save the same
/// layout at another address).
#[derive(Clone, Copy)]
pub(super) struct Prg<'a> {
    data: &'a [u8],
    load: usize,
}

impl<'a> Prg<'a> {
    /// `data` is the whole file including its load-address header.
    pub(super) fn new(data: &'a [u8], load: u16) -> Self {
        Self {
            data,
            load: usize::from(load),
        }
    }

    /// `len` bytes at C64 address `addr`, if the file covers them.
    pub(super) fn at(&self, addr: u16, len: usize) -> Option<&'a [u8]> {
        let start = usize::from(addr).checked_sub(self.load)? + 2;
        self.data.get(start..start + len)
    }

    /// The byte at `addr`.
    pub(super) fn byte(&self, addr: u16) -> Option<u8> {
        self.at(addr, 1).map(|b| b[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_skip_header() {
        let data = [0x00, 0x60, 1, 2, 3];
        let prg = Prg::new(&data, 0x6000);
        assert_eq!(prg.at(0x6001, 2), Some(&[2u8, 3][..]));
        assert_eq!(prg.at(0x6002, 2), None);
        assert_eq!(prg.at(0x5fff, 1), None);
    }
}
