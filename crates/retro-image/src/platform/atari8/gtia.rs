//! GTIA player/missile graphics and colour priority.
//!
//! Sources:
//! - De Re Atari ch. 4, player-missile graphics: HPOS, SIZE, GRAF and PRIOR
//!   registers (<https://www.atariarchives.org/dere/chapt04.php>), and
//!   App. E, GTIA modes (<https://www.atariarchives.org/dere/chaptE.php>);
//!   Atari Player-Missile Graphics in BASIC ch. 2, pixel widths
//!   (<https://www.atariarchives.org/pmgraphics/chapter2.php>); Altirra
//!   Hardware Reference Manual, GTIA priority
//!   (<https://www.virtualdub.org/downloads/Altirra%20Hardware%20Reference%20Manual.pdf>).
//! - The priority logic is the GTIA's: every colour register whose signal
//!   survives the priority equations below is ORed into the output, which
//!   is what makes conflicting PRIOR settings show mixed or black colours.
//!   The equations were checked by black-box probing of `recoil2png` with
//!   hand-made Graph2Font MCH files: every PRIOR value (0-63), every
//!   combination of the four players over background and each playfield
//!   colour, and every missile combination with and without the
//!   fifth-player bit. The output geometry (HPOS 0x2C at pixel 0, colour
//!   clocks 2 pixels wide) was observed the same way.

/// Player colour registers (COLPM0-3), playfield registers (COLPF0-3) and
/// the background (COLBK).
#[derive(Clone, Copy)]
pub(super) struct Colors {
    pub player: [u8; 4],
    pub playfield: [u8; 4],
    pub background: u8,
}

/// The colour value GTIA shows for `players` (bits 0-3 = P0-P3) over
/// `playfield` (bits 0-3 = PF0-PF3) under `prior`.
pub(super) fn resolve(prior: u8, players: u8, playfield: u8, colors: &Colors) -> u8 {
    let selected = SIGNALS[signal_index(prior, players, playfield)];
    let registers = [
        colors.player[0],
        colors.player[1],
        colors.player[2],
        colors.player[3],
        colors.playfield[0],
        colors.playfield[1],
        colors.playfield[2],
        colors.playfield[3],
        colors.background,
    ];
    registers
        .iter()
        .enumerate()
        .filter(|(n, _)| selected >> n & 1 != 0)
        .fold(0, |color, (_, value)| color | value)
}

/// Index into [`SIGNALS`]: the PRIOR bits the equations read (0-3 and the
/// multicolour player bit 5), the players and the playfield.
fn signal_index(prior: u8, players: u8, playfield: u8) -> usize {
    usize::from(prior & 15 | (prior >> 5 & 1) << 4)
        | usize::from(players & 15) << 5
        | usize::from(playfield & 15) << 9
}

/// [`signals`] for every input, so a pixel costs one lookup.
static SIGNALS: [u16; 1 << 13] = {
    let mut table = [0; 1 << 13];
    let mut index = 0;
    while index < table.len() {
        let prior = (index & 15 | (index >> 4 & 1) << 5) as u8;
        table[index] = signals(prior, (index >> 5 & 15) as u8, (index >> 9 & 15) as u8);
        index += 1;
    }
    table
};

/// Which colour registers show: bit 0-3 = players, 4-7 = playfields,
/// 8 = background.
// Every signal is written as "present and not blocked by ...", which reads
// better than the minimised forms Clippy suggests.
#[allow(clippy::nonminimal_bool)]
const fn signals(prior: u8, players: u8, playfield: u8) -> u16 {
    const fn bit(value: u8, n: u8) -> bool {
        value >> n & 1 != 0
    }
    let (p0, p1, p2, p3) = (
        bit(players, 0),
        bit(players, 1),
        bit(players, 2),
        bit(players, 3),
    );
    let (f0, f1, f2, f3) = (
        bit(playfield, 0),
        bit(playfield, 1),
        bit(playfield, 2),
        bit(playfield, 3),
    );
    let (pri0, pri1, pri2, pri3) = (bit(prior, 0), bit(prior, 1), bit(prior, 2), bit(prior, 3));
    let multicolor = bit(prior, 5);
    let (p01, p23, pf01, pf23) = (p0 || p1, p2 || p3, f0 || f1, f2 || f3);
    let (pri01, pri12, pri23, pri03) = (pri0 || pri1, pri1 || pri2, pri2 || pri3, pri0 || pri3);

    let sp0 = p0 && !(pf01 && pri23) && !(pri2 && pf23);
    let sp1 = p1 && !(pf01 && pri23) && !(pri2 && pf23) && (!p0 || multicolor);
    let sp2 = p2 && !p01 && !(pf23 && pri12) && !(pf01 && !pri0);
    let sp3 = p3 && !p01 && !(p2 && !multicolor) && !(pf23 && pri12) && !(pf01 && !pri0);
    let sf3 = f3 && !(p23 && pri03) && !(p01 && !pri2);
    let sf0 = f0 && !(p23 && pri0) && !(p01 && pri01) && !sf3;
    let sf1 = f1 && !(p23 && pri0) && !(p01 && pri01) && !sf3;
    let sf2 = f2 && !(p23 && pri03) && !(p01 && !pri2) && !sf3;
    let sb = !p01 && !p23 && !pf01 && !pf23;

    let flags = [sp0, sp1, sp2, sp3, sf0, sf1, sf2, sf3, sb];
    let mut mask = 0;
    let mut n = 0;
    while n < flags.len() {
        if flags[n] {
            mask |= 1 << n;
        }
        n += 1;
    }
    mask
}

/// Palette slot of an ANTIC mode 4 pixel: 0 background, 1-3 playfield
/// 0-2, 4 playfield 3, which an inverse character shows for value 3.
pub(super) fn antic4_register(value: u8, inverse: bool) -> usize {
    if value == 3 && inverse {
        4
    } else {
        usize::from(value)
    }
}

/// The playfield bit (PF0 = bit 0 ... PF3 = bit 3) of a palette slot
/// from [`antic4_register`]; the background has none.
pub(super) fn playfield_bit(register: usize) -> u8 {
    match register {
        0 => 0,
        n => 1 << (n - 1),
    }
}

/// Splits one pixel's object flags (see [`Objects::pixels`]) over the
/// players and playfield bits: with PRIOR bit 4 (fifth player) the missiles
/// together act as playfield 3, otherwise as extra players.
pub(super) fn add_objects(prior: u8, objs: u8, players: u8, playfield: u8) -> (u8, u8) {
    let players = players | objs & 0x0f;
    if prior & 0x10 == 0 {
        (players | objs >> 4, playfield)
    } else if objs & 0xf0 != 0 {
        (players, playfield | 8)
    } else {
        (players, playfield)
    }
}

/// Player and missile shapes on one scanline, in output pixels of a
/// 336-pixel-wide picture whose playfield starts at HPOS 0x2C.
pub(super) struct Objects {
    /// Bits 0-3: players 0-3; bits 4-7: missiles 0-3.
    pub pixels: [u8; WIDTH],
}

pub(super) const WIDTH: usize = 336;

/// One scanline's PMG registers.
pub(super) struct Pmg {
    pub hpos_player: [u8; 4],
    pub hpos_missile: [u8; 4],
    /// SIZEP0-3 packed two bits per player, player 0 lowest.
    pub size_player: u8,
    pub size_missile: u8,
    pub graf_player: [u8; 4],
    pub graf_missile: u8,
}

/// One player or missile: it starts where its HPOS says and then shows
/// `bits`, most significant first, each as wide as its size says.
pub(super) struct Shape {
    /// First output pixel, negative left of the picture.
    pub left: i32,
    size: u8,
    bits: u8,
    count: u32,
    flag: u8,
}

impl Shape {
    fn new(hpos: u8, size: u8, bits: u8, count: u32, flag: u8) -> Self {
        Shape {
            left: 2 * i32::from(hpos) - 88,
            size,
            bits,
            count,
            flag,
        }
    }

    /// Adds the shape to `pixels`. `resized` are later size writes, as
    /// (first output pixel, size) in order, which stretch the bits still to
    /// come.
    pub fn draw(&self, pixels: &mut [u8; WIDTH], resized: &[(i32, u8)]) {
        let mut resized = resized.iter().peekable();
        let mut size = self.size;
        let mut start = self.left;
        for i in 0..self.count {
            while let Some(&(_, new)) = resized.next_if(|&&(x, _)| x <= start) {
                size = new;
            }
            let width = 2 * [1, 2, 1, 4][usize::from(size & 3)];
            if self.bits >> (self.count - 1 - i) & 1 != 0 {
                for x in start.max(0)..(start + width).min(WIDTH as i32) {
                    pixels[x as usize] |= self.flag;
                }
            }
            start += width;
        }
    }
}

impl Pmg {
    /// The four players, then the four missiles.
    pub fn shapes(&self) -> [Shape; 8] {
        core::array::from_fn(|n| {
            let k = n % 4;
            if n < 4 {
                Shape::new(
                    self.hpos_player[k],
                    self.size_player >> (2 * k),
                    self.graf_player[k],
                    8,
                    1 << k,
                )
            } else {
                Shape::new(
                    self.hpos_missile[k],
                    self.size_missile >> (2 * k),
                    self.graf_missile >> (2 * k),
                    2,
                    0x10 << k,
                )
            }
        })
    }

    pub fn draw(&self) -> Objects {
        let mut pixels = [0; WIDTH];
        self.shapes()
            .iter()
            .for_each(|shape| shape.draw(&mut pixels, &[]));
        Objects { pixels }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: Colors = Colors {
        player: [0x12, 0x24, 0x48, 0x8a],
        playfield: [0x56, 0x66, 0x76, 0x96],
        background: 0xb4,
    };

    #[test]
    fn priority_selects_and_mixes() {
        // PRIOR 1: players above playfield.
        assert_eq!(resolve(0x01, 0b0001, 0b0001, &COLORS), 0x12);
        // PRIOR 4: playfield above players.
        assert_eq!(resolve(0x04, 0b0001, 0b0001, &COLORS), 0x56);
        // PRIOR 0: player 2 and PF2 both pass and are ORed.
        assert_eq!(resolve(0x00, 0b0100, 0b0100, &COLORS), 0x48 | 0x76);
        // Nothing: background.
        assert_eq!(resolve(0x00, 0, 0, &COLORS), 0xb4);
    }

    #[test]
    fn draws_players_and_missiles() {
        let pmg = Pmg {
            hpos_player: [0x30, 0, 0, 0],
            hpos_missile: [0x40, 0, 0, 0],
            size_player: 1,
            size_missile: 3,
            graf_player: [0x81, 0, 0, 0],
            graf_missile: 0b10,
        };
        let objects = pmg.draw();
        assert_eq!(objects.pixels[8..12], [1; 4]);
        assert_eq!(objects.pixels[12], 0);
        assert_eq!(objects.pixels[36..40], [1; 4]);
        assert_eq!(objects.pixels[40..48], [0x10; 8]);
    }
}
