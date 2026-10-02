//! GTIA player/missile graphics and colour priority.
//!
//! Sources:
//! - De Re Atari ch. 4 (player-missile graphics: HPOS, SIZE, GRAF and PRIOR
//!   registers) and App. E (GTIA modes); Mapping the Atari (register
//!   addresses).
//! - The priority logic is the GTIA's: every colour register whose signal
//!   survives the priority equations below is ORed into the output, which
//!   is what makes conflicting PRIOR settings show mixed or black colours.
//!   The equations were checked against `recoil2png` output for every PRIOR
//!   value (0-63), every combination of the four players over background
//!   and each playfield colour, and every missile combination with and
//!   without the fifth-player bit.

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
// Every signal is written as "present and not blocked by ...", which reads
// better than the minimised forms Clippy suggests.
#[allow(clippy::nonminimal_bool)]
pub(super) fn resolve(prior: u8, players: u8, playfield: u8, colors: &Colors) -> u8 {
    let bit = |value: u8, n: u8| value >> n & 1 != 0;
    let [p0, p1, p2, p3] = [0, 1, 2, 3].map(|n| bit(players, n));
    let [f0, f1, f2, f3] = [0, 1, 2, 3].map(|n| bit(playfield, n));
    let [pri0, pri1, pri2, pri3] = [0, 1, 2, 3].map(|n| bit(prior, n));
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

    let selected = [
        (sp0, colors.player[0]),
        (sp1, colors.player[1]),
        (sp2, colors.player[2]),
        (sp3, colors.player[3]),
        (sf0, colors.playfield[0]),
        (sf1, colors.playfield[1]),
        (sf2, colors.playfield[2]),
        (sf3, colors.playfield[3]),
        (sb, colors.background),
    ];
    selected
        .iter()
        .filter(|(on, _)| *on)
        .fold(0, |color, (_, value)| color | value)
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

impl Pmg {
    pub fn draw(&self) -> Objects {
        let mut pixels = [0; WIDTH];
        let mut fill = |hpos: u8, size: u8, bits: u8, count: u32, flag: u8| {
            let width = 2 * [1, 2, 1, 4][usize::from(size & 3)];
            let left = 2 * i32::from(hpos) - 88;
            for i in 0..count {
                if bits >> (count - 1 - i) & 1 == 0 {
                    continue;
                }
                let start = left + (i as i32) * width;
                for x in start.max(0)..(start + width).min(WIDTH as i32) {
                    pixels[x as usize] |= flag;
                }
            }
        };
        for k in 0..4 {
            fill(
                self.hpos_player[k],
                self.size_player >> (2 * k),
                self.graf_player[k],
                8,
                1 << k,
            );
            fill(
                self.hpos_missile[k],
                self.size_missile >> (2 * k),
                self.graf_missile >> (2 * k),
                2,
                0x10 << k,
            );
        }
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
