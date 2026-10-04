//! When the register writes of a PowerGraphics event stream land within a
//! scanline: the 6502 cycles of the display kernel that executes the events,
//! slowed by ANTIC's DMA.
//!
//! Sources:
//! - ANTIC's cycle stealing (114 cycles per scanline, 9 refresh cycles from
//!   cycle 25 every 4 cycles, the display list fetch at cycle 9, player and
//!   missile fetches at the start of the line, one playfield fetch every 2
//!   cycles) is as in the Altirra Hardware Reference Manual
//!   (<https://www.virtualdub.org/downloads/Altirra%20Hardware%20Reference%20Manual.pdf>)
//!   and De Re Atari ch. 2 (<https://www.atariarchives.org/dere/chapt02.php>).
//! - The kernel's own timing is reverse engineered from `recoil2png`
//!   (black box): chains of COLBK writes in blank and Graphics 7.5 lines,
//!   with and without player/missile DMA, for the 32 and 40 byte widths,
//!   fitted until the simulated write positions matched in every case;
//!   notes in `docs/research/gaps-corpus-atari8.md` section 6.
//!
//! The kernel starts a scanline's events at cycle -9 (the end of WSYNC in
//! the previous line). A register write takes 4 cycles, plus 2 with a value
//! byte (event bit 5). An event on register `0x1c + n` (a wait, writing
//! nothing) takes `8 * n`, plus 2 for bit 5, 2 for bit 6 and 4 for bit 7.
//! The CPU makes progress only on cycles ANTIC does not steal.
//! A write lands on the last of its cycles, at output pixel `4 * cycle - 86`.
//! The playfield fetch of 32 and 40 bytes starts at cycle 28 and 20; the
//! start for 48 bytes was not observed, so a clock on such a line gives up
//! at cycle 12.

/// Where a write lands, in output pixels from the left of the picture. A
/// write that lands before the picture takes effect at its first pixel (as
/// observed: a player whose HPOS puts it left of the picture does not see
/// an earlier HPOS write).
pub(super) type Position = i32;

/// The cycles ANTIC steals on one scanline.
#[derive(Clone, Copy)]
pub(super) struct Dma {
    /// Playfield bytes fetched, 0 on blank lines.
    pub fetched: u32,
    /// The display list instruction has an address (LMS).
    pub lms: bool,
    /// Player and missile DMA bits of DMACTL (bits 3 and 2).
    pub players: bool,
    pub missiles: bool,
}

/// First cycle of a scanline's events.
const START: i32 = -9;
/// Cycles per scanline; a write landing later would belong to the next one.
const LINE: i32 = 114;
/// Cycles of a write, and of a wait per register step.
const WRITE: u32 = 4;
const WAIT: u32 = 8;
/// Where the playfield fetch of 48 bytes might start.
const WIDE_UNKNOWN: i32 = 12;

impl Dma {
    /// The first cycle after which the stolen cycles are not known.
    fn known_until(self) -> i32 {
        if self.fetched == 48 {
            WIDE_UNKNOWN
        } else {
            LINE
        }
    }

    /// First playfield fetch cycle.
    fn fetch_start(self) -> i32 {
        if self.fetched == 32 { 28 } else { 20 }
    }

    /// Whether the CPU is held on `cycle`.
    fn stolen(self, cycle: i32) -> bool {
        let start = self.fetch_start();
        let fetch =
            (start..start + 2 * self.fetched as i32).contains(&cycle) && (cycle - start) % 2 == 0;
        let refresh = (25..=57).contains(&cycle) && (cycle - 25) % 4 == 0;
        let list = cycle == 9 || self.lms && (10..=11).contains(&cycle);
        // Player DMA also takes the missile cycle.
        let objects = match (self.players, self.missiles) {
            (true, _) => 0..5,
            (false, true) => 0..1,
            _ => 0..0,
        };
        fetch || refresh || list || objects.contains(&cycle)
    }
}

/// The kernel's progress through one scanline.
pub(super) struct Clock {
    dma: Dma,
    cycle: i32,
}

impl Clock {
    pub fn new(dma: Dma) -> Self {
        Clock { dma, cycle: START }
    }

    /// Runs the event byte `event` (a write, or a wait on register 0x1c-0x1f)
    /// and returns the position of its write. `None` if the event runs past
    /// the scanline or into cycles whose DMA is unknown.
    pub fn run(&mut self, event: u8) -> Option<Position> {
        let register = u32::from(event & 0x1f);
        let bit = |n: u8| u32::from(event >> n & 1);
        let mut left = if register < 0x1c {
            WRITE + 2 * bit(5)
        } else {
            WAIT * (register - 0x1c) + 2 * bit(5) + 2 * bit(6) + 4 * bit(7)
        };
        while left > 0 {
            if self.cycle >= self.dma.known_until() {
                return None;
            }
            if !self.dma.stolen(self.cycle) {
                left -= 1;
            }
            self.cycle += 1;
        }
        Some((4 * (self.cycle - 1) - 86).max(0))
    }
}
