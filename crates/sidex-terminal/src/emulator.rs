//! Terminal emulator — ANSI escape sequence parser using the `vte` crate.
//!
//! Processes byte streams from a PTY and updates a [`TerminalGrid`] accordingly.
//! Implements the `vte::Perform` trait to handle characters, control codes,
//! CSI sequences (cursor movement, erase, SGR), OSC sequences, and ESC sequences.

use crate::grid::{Cell, Color, TerminalGrid};

/// A terminal emulator that feeds PTY output bytes through a VTE parser
/// and updates the backing grid.
pub struct TerminalEmulator {
    grid: TerminalGrid,
    parser: vte::Parser,
    /// Template cell carrying the current SGR attributes.
    pen: Cell,
    /// Saved cursor position (for ESC 7 / ESC 8).
    saved_cursor: (u16, u16),
    /// Window title set via OSC 0 / OSC 2.
    title: String,
}

impl TerminalEmulator {
    /// Creates a new emulator backed by the given grid.
    pub fn new(grid: TerminalGrid) -> Self {
        Self {
            grid,
            parser: vte::Parser::new(),
            pen: Cell::default(),
            saved_cursor: (0, 0),
            title: String::new(),
        }
    }

    /// Feeds raw bytes from the PTY into the parser, updating the grid.
    pub fn process(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            let mut performer = Performer {
                grid: &mut self.grid,
                pen: &mut self.pen,
                saved_cursor: &mut self.saved_cursor,
                title: &mut self.title,
            };
            self.parser.advance(&mut performer, byte);
        }
    }

    /// Returns a reference to the backing grid.
    pub fn grid(&self) -> &TerminalGrid {
        &self.grid
    }

    /// Returns a mutable reference to the backing grid.
    pub fn grid_mut(&mut self) -> &mut TerminalGrid {
        &mut self.grid
    }

    /// Returns the current window title (set via OSC sequences).
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns a reference to the current pen (SGR attribute template).
    pub fn pen(&self) -> &Cell {
        &self.pen
    }
}

/// Internal performer that holds mutable references to emulator state.
/// This is separate from `TerminalEmulator` so we can pass it to the VTE parser
/// without borrowing issues (parser is on the same struct).
struct Performer<'a> {
    grid: &'a mut TerminalGrid,
    pen: &'a mut Cell,
    saved_cursor: &'a mut (u16, u16),
    title: &'a mut String,
}

#[allow(clippy::cast_possible_truncation)]
fn u16_to_u8(v: u16) -> u8 {
    (v & 0xFF) as u8
}

impl Performer<'_> {
    /// Parses a CSI SGR (Select Graphic Rendition) sequence from parameter values.
    fn handle_sgr(&mut self, params: &vte::Params) {
        let mut iter = params.iter();
        while let Some(param) = iter.next() {
            let code = param[0];
            match code {
                0 => *self.pen = Cell::default(),
                1 => self.pen.bold = true,
                3 => self.pen.italic = true,
                4 => self.pen.underline = true,
                9 => self.pen.strikethrough = true,
                22 => self.pen.bold = false,
                23 => self.pen.italic = false,
                24 => self.pen.underline = false,
                29 => self.pen.strikethrough = false,
                30..=37 => self.pen.fg = Color::Indexed(u16_to_u8(code - 30)),
                90..=97 => self.pen.fg = Color::Indexed(u16_to_u8(code - 90 + 8)),
                38 => self.parse_extended_color(&mut iter, true),
                39 => self.pen.fg = Color::Default,
                40..=47 => self.pen.bg = Color::Indexed(u16_to_u8(code - 40)),
                100..=107 => self.pen.bg = Color::Indexed(u16_to_u8(code - 100 + 8)),
                48 => self.parse_extended_color(&mut iter, false),
                49 => self.pen.bg = Color::Default,
                _ => {}
            }
        }
    }

    /// Parses `5;n` (256-color) or `2;r;g;b` (truecolor) after a 38/48 code.
    fn parse_extended_color<'b>(
        &mut self,
        iter: &mut impl Iterator<Item = &'b [u16]>,
        foreground: bool,
    ) {
        let Some(kind) = iter.next() else { return };
        match kind[0] {
            5 => {
                if let Some(idx) = iter.next() {
                    let color = Color::Indexed(u16_to_u8(idx[0]));
                    if foreground {
                        self.pen.fg = color;
                    } else {
                        self.pen.bg = color;
                    }
                }
            }
            2 => {
                let r = iter.next().map_or(0, |p| u16_to_u8(p[0]));
                let g = iter.next().map_or(0, |p| u16_to_u8(p[0]));
                let b = iter.next().map_or(0, |p| u16_to_u8(p[0]));
                let color = Color::Rgb(r, g, b);
                if foreground {
                    self.pen.fg = color;
                } else {
                    self.pen.bg = color;
                }
            }
            _ => {}
        }
    }
}

impl vte::Perform for Performer<'_> {
    fn print(&mut self, c: char) {
        self.grid.write_char(c, self.pen);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                let (row, _col) = self.grid.cursor_position();
                let (_top, bottom) = self.grid.scroll_region();
                if row >= bottom {
                    self.grid.scroll_up();
                } else {
                    self.grid.set_cursor(row + 1, self.grid.cursor_position().1);
                }
            }
            b'\r' => {
                let (row, _col) = self.grid.cursor_position();
                self.grid.set_cursor(row, 0);
            }
            0x08 => {
                let (row, col) = self.grid.cursor_position();
                if col > 0 {
                    self.grid.set_cursor(row, col - 1);
                }
            }
            b'\t' => {
                let (row, col) = self.grid.cursor_position();
                let next_tab = (col / 8 + 1) * 8;
                self.grid.set_cursor(row, next_tab.min(self.grid.cols() - 1));
            }
            0x07 => {
                log::trace!("BEL");
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let first = params.iter().next().map_or(1, |p| p[0].max(1));
        let (row, col) = self.grid.cursor_position();

        match action {
            'A' => self.grid.set_cursor(row.saturating_sub(first), col),
            'B' => self.grid.set_cursor(row + first, col),
            'C' => self.grid.set_cursor(row, col + first),
            'D' => self.grid.set_cursor(row, col.saturating_sub(first)),
            'H' | 'f' => {
                let mut piter = params.iter();
                let r = piter.next().map_or(1, |p| p[0].max(1));
                let c = piter.next().map_or(1, |p| p[0].max(1));
                self.grid.set_cursor(r - 1, c - 1);
            }
            'J' => {
                let mode = params.iter().next().map_or(0, |p| p[0]);
                match mode {
                    0 => self.grid.clear_below(),
                    1 => self.grid.clear_above(),
                    2 | 3 => self.grid.clear(),
                    _ => {}
                }
            }
            'K' => {
                let mode = params.iter().next().map_or(0, |p| p[0]);
                match mode {
                    0 => self.grid.clear_line_from_cursor(),
                    1 => self.grid.clear_line_to_cursor(),
                    2 => self.grid.clear_line(row),
                    _ => {}
                }
            }
            'm' => self.handle_sgr(params),
            'S' => {
                for _ in 0..first {
                    self.grid.scroll_up();
                }
            }
            'T' => {
                for _ in 0..first {
                    self.grid.scroll_down();
                }
            }
            'r' => {
                let mut piter = params.iter();
                let top = piter.next().map_or(1, |p| p[0].max(1));
                let bottom = piter.next().map_or(self.grid.rows(), |p| p[0].max(1));
                self.grid.set_scroll_region(top - 1, bottom - 1);
                self.grid.set_cursor(0, 0);
            }
            _ => {
                log::trace!("unhandled CSI: {action}");
            }
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }
        let code = params[0];
        if (code == b"0" || code == b"2") && params.len() >= 2 {
            *self.title = String::from_utf8_lossy(params[1]).into_owned();
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'7' => {
                *self.saved_cursor = self.grid.cursor_position();
            }
            b'8' => {
                let (r, c) = *self.saved_cursor;
                self.grid.set_cursor(r, c);
            }
            b'M' => {
                let (row, _col) = self.grid.cursor_position();
                let (top, _bottom) = self.grid.scroll_region();
                if row == top {
                    self.grid.scroll_down();
                } else {
                    self.grid
                        .set_cursor(row.saturating_sub(1), self.grid.cursor_position().1);
                }
            }
            _ => {
                log::trace!("unhandled ESC: 0x{byte:02x}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_emulator(rows: u16, cols: u16) -> TerminalEmulator {
        TerminalEmulator::new(TerminalGrid::new(rows, cols))
    }

    #[test]
    fn simple_text_output() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"Hello, world!");
        assert_eq!(emu.grid().row_text(0), "Hello, world!");
        assert_eq!(emu.grid().cursor_position(), (0, 13));
    }

    #[test]
    fn newline_and_carriage_return() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"Line1\r\nLine2");
        assert_eq!(emu.grid().row_text(0), "Line1");
        assert_eq!(emu.grid().row_text(1), "Line2");
    }

    #[test]
    fn cursor_movement_csi() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[3;5H*");
        assert_eq!(emu.grid().cell(2, 4).character, '*');
    }

    #[test]
    fn erase_display_below() {
        let mut emu = make_emulator(4, 10);
        emu.process(b"AAAAAAAAAA");
        emu.process(b"\r\nBBBBBBBBBB");
        emu.process(b"\x1b[1;6H\x1b[0J");
        assert_eq!(emu.grid().row_text(0), "AAAAA");
        assert_eq!(emu.grid().row_text(1), "");
    }

    #[test]
    fn erase_entire_line() {
        let mut emu = make_emulator(4, 10);
        emu.process(b"Hello");
        emu.process(b"\x1b[2K");
        assert_eq!(emu.grid().row_text(0), "");
    }

    #[test]
    fn sgr_bold_and_reset() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[1mBold\x1b[0m");
        assert!(emu.grid().cell(0, 0).bold);
        assert_eq!(emu.grid().cell(0, 0).character, 'B');
        assert!(!emu.pen().bold);
    }

    #[test]
    fn sgr_italic_underline_strikethrough() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[3;4;9mX");
        let cell = emu.grid().cell(0, 0);
        assert!(cell.italic);
        assert!(cell.underline);
        assert!(cell.strikethrough);
    }

    #[test]
    fn sgr_standard_foreground_color() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[31mR");
        assert_eq!(emu.grid().cell(0, 0).fg, Color::Indexed(1));
    }

    #[test]
    fn sgr_256_color() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[38;5;200mX");
        assert_eq!(emu.grid().cell(0, 0).fg, Color::Indexed(200));
    }

    #[test]
    fn sgr_truecolor() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[38;2;100;150;200mX");
        assert_eq!(emu.grid().cell(0, 0).fg, Color::Rgb(100, 150, 200));
    }

    #[test]
    fn sgr_background_truecolor() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[48;2;10;20;30mX");
        assert_eq!(emu.grid().cell(0, 0).bg, Color::Rgb(10, 20, 30));
    }

    #[test]
    fn sgr_bright_colors() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[91mX");
        assert_eq!(emu.grid().cell(0, 0).fg, Color::Indexed(9));
    }

    #[test]
    fn osc_set_title() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b]0;My Terminal\x07");
        assert_eq!(emu.title(), "My Terminal");
    }

    #[test]
    fn backspace_moves_cursor_back() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"AB\x08C");
        assert_eq!(emu.grid().row_text(0), "AC");
    }

    #[test]
    fn tab_advances_to_next_stop() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"A\tB");
        assert_eq!(emu.grid().cursor_position().1, 9);
        assert_eq!(emu.grid().cell(0, 8).character, 'B');
    }

    #[test]
    fn save_restore_cursor() {
        let mut emu = make_emulator(24, 80);
        emu.process(b"\x1b[5;10H");
        emu.process(b"\x1b7");
        emu.process(b"\x1b[1;1H");
        emu.process(b"\x1b8");
        assert_eq!(emu.grid().cursor_position(), (4, 9));
    }

    #[test]
    fn scroll_region_and_scroll_up() {
        let mut emu = make_emulator(5, 10);
        emu.process(b"\x1b[2;4r");
        assert_eq!(emu.grid().scroll_region(), (1, 3));
    }

    #[test]
    fn clear_display_mode_2() {
        let mut emu = make_emulator(4, 10);
        emu.process(b"Hello");
        emu.process(b"\r\nWorld");
        emu.process(b"\x1b[2J");
        assert_eq!(emu.grid().row_text(0), "");
        assert_eq!(emu.grid().row_text(1), "");
    }

    #[test]
    fn scrollback_on_scroll() {
        let mut emu = make_emulator(3, 10);
        emu.process(b"Line1\r\nLine2\r\nLine3\r\nLine4");
        assert!(emu.grid().scrollback_len() >= 1);
    }
}
