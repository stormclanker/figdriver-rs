use super::InputEncoding;
use super::iso2022::{Iso2022Decoder, Iso2022Settings};

/// Iterator that decodes raw bytes into character codes according to the given encoding.
/// Yields one `i32` code per `next()` call.
pub(super) struct EncodingDecoder<'a> {
    bytes: &'a [u8],
    encoding: InputEncoding,
    pos: usize,
    /// For HZ encoding: are we currently in two-byte mode?
    hz_two_byte: bool,
    /// For ISO 2022 encoding: the stateful decoder.
    iso2022_decoder: Option<Iso2022Decoder<'a>>,
}

impl<'a> EncodingDecoder<'a> {
    pub(super) fn new(bytes: &'a [u8], encoding: InputEncoding, iso2022: Option<&'a Iso2022Settings>) -> Self {
        let iso2022_decoder = if encoding == InputEncoding::ISO2022 {
            let mut decoder = iso2022.map(|s| s.build_decoder()).unwrap_or_default();
            decoder.set_input(bytes);
            Some(decoder)
        } else {
            None
        };
        Self {
            bytes,
            encoding,
            pos: 0,
            hz_two_byte: false,
            iso2022_decoder,
        }
    }
}

impl Iterator for EncodingDecoder<'_> {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        match self.encoding {
            InputEncoding::Default  => self.next_utf8(),
            InputEncoding::Latin1   => self.next_latin1(),
            InputEncoding::UTF8     => self.next_utf8(),
            InputEncoding::Dbcs     => self.next_dbcs(),
            InputEncoding::ShiftJIS => self.next_shiftjis(),
            InputEncoding::HZ       => self.next_hz(),
            InputEncoding::ISO2022  => self.iso2022_decoder.as_mut().and_then(|d| d.next()),
        }
    }
}

impl EncodingDecoder<'_> {
    fn next_latin1(&mut self) -> Option<i32> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let b = self.bytes[self.pos];
        self.pos += 1;
        Some(b as i32)
    }

    fn next_utf8(&mut self) -> Option<i32> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let remaining = &self.bytes[self.pos..];
        let first = remaining[0];

        let (code, consumed) = if first < 0x80 {
            (first as i32, 1)
        } else if first < 0xC0 {
            (128, 1)
        } else if first < 0xE0 {
            if remaining.len() >= 2 && (remaining[1] & 0xC0) == 0x80 {
                let hi = (remaining[0] & 0x1F) as u32;
                let lo = (remaining[1] & 0x3F) as u32;
                let cp = (hi << 6) | lo;
                if cp >= 0x80 {
                    (cp as i32, 2)
                } else {
                    (128, 1)
                }
            } else {
                (128, 1)
            }
        } else if first < 0xF0 {
            if remaining.len() >= 3
                && (remaining[1] & 0xC0) == 0x80
                && (remaining[2] & 0xC0) == 0x80
            {
                let hi = (remaining[0] & 0x0F) as u32;
                let mid = (remaining[1] & 0x3F) as u32;
                let lo = (remaining[2] & 0x3F) as u32;
                let cp = (hi << 12) | (mid << 6) | lo;
                if cp >= 0x800 && !(0xD800..=0xDFFF).contains(&cp) {
                    (cp as i32, 3)
                } else {
                    (128, 1)
                }
            } else {
                (128, 1)
            }
        } else if first < 0xF8 {
            if remaining.len() >= 4
                && (remaining[1] & 0xC0) == 0x80
                && (remaining[2] & 0xC0) == 0x80
                && (remaining[3] & 0xC0) == 0x80
            {
                let hi = (remaining[0] & 0x07) as u32;
                let mid1 = (remaining[1] & 0x3F) as u32;
                let mid2 = (remaining[2] & 0x3F) as u32;
                let lo = (remaining[3] & 0x3F) as u32;
                let cp = (hi << 18) | (mid1 << 12) | (mid2 << 6) | lo;
                if (0x10000..=0x10FFFF).contains(&cp) {
                    (cp as i32, 4)
                } else {
                    (128, 1)
                }
            } else {
                (128, 1)
            }
        } else {
            (128, 1)
        };

        self.pos += consumed;
        Some(code)
    }

    fn next_dbcs(&mut self) -> Option<i32> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let b = self.bytes[self.pos];

        if b < 0x80 {
            self.pos += 1;
            Some(b as i32)
        } else if self.pos + 1 < self.bytes.len() {
            let hi = b;
            let lo = self.bytes[self.pos + 1];
            self.pos += 2;
            Some(((hi as i32) << 8) | (lo as i32))
        } else {
            self.pos += 1;
            Some(b as i32)
        }
    }

    fn next_shiftjis(&mut self) -> Option<i32> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let b = self.bytes[self.pos];

        let is_high_byte = (0x80..=0x9F).contains(&b) || (0xE0..=0xEF).contains(&b);

        if is_high_byte && self.pos + 1 < self.bytes.len() {
            let hi = b;
            let lo = self.bytes[self.pos + 1];
            self.pos += 2;
            Some(((hi as i32) << 8) | (lo as i32))
        } else {
            self.pos += 1;
            Some(b as i32)
        }
    }

    fn next_hz(&mut self) -> Option<i32> {
        loop {
            if self.pos >= self.bytes.len() {
                return None;
            }

            if self.bytes[self.pos] == b'~' {
                self.pos += 1;
                if self.pos >= self.bytes.len() {
                    // Lone tilde at end of input: consume silently
                    continue;
                }

                match self.bytes[self.pos] {
                    b'{' => {
                        self.pos += 1;
                        self.hz_two_byte = true;
                        continue;
                    }
                    b'}' => {
                        self.pos += 1;
                        self.hz_two_byte = false;
                        continue;
                    }
                    b'~' => {
                        self.pos += 1;
                        return Some('~' as i32);
                    }
                    _ => {
                        // All other ~X sequences are removed from input
                        self.pos += 1;
                        continue;
                    }
                }
            }

            if self.hz_two_byte {
                if self.pos + 1 < self.bytes.len() {
                    let hi = self.bytes[self.pos];
                    let lo = self.bytes[self.pos + 1];
                    self.pos += 2;
                    return Some(((hi as i32) << 8) | (lo as i32));
                } else {
                    let b = self.bytes[self.pos];
                    self.pos += 1;
                    self.hz_two_byte = false;
                    return Some(b as i32);
                }
            } else {
                let b = self.bytes[self.pos];
                self.pos += 1;
                return Some(b as i32);
            }
        }
    }
}

/// Persistent decoder state for ISO 2022 encoding, preserved across calls.
#[derive(Clone, Debug)]
struct Iso2022State {
    gn: [i32; 4],
    gndbl: [bool; 4],
    gl: usize,
    gr: usize,
    single_shift: Option<usize>,
}

impl Default for Iso2022State {
    fn default() -> Self {
        Self {
            gn: [0, 0x80, 0, 0],
            gndbl: [false; 4],
            gl: 0,
            gr: 1,
            single_shift: None,
        }
    }
}

/// Mutable decode context for ISO 2022, used within a single decode_bytes call.
struct Iso2022DecodeCtx {
    gn: [i32; 4],
    gndbl: [bool; 4],
    gl: usize,
    gr: usize,
    single_shift: Option<usize>,
    pos: usize,
}

impl Iso2022State {
    fn from_settings(settings: &Iso2022Settings) -> Self {
        let mut gn = [0i32; 4];
        let mut gndbl = [false; 4];
        for (i, gset) in settings.g_sets.iter().enumerate() {
            if let Some(gs) = gset {
                gn[i] = gs.base_code;
                gndbl[i] = gs.double_byte;
            }
        }
        Self {
            gn,
            gndbl,
            gl: settings.left_half,
            gr: settings.right_half,
            single_shift: None,
        }
    }
}

/// Streaming decoder that preserves state across multiple `decode_bytes` calls.
///
/// Unlike `EncodingDecoder` which creates a fresh decoder per call, this type
/// maintains decoder state (HZ two-byte flag, ISO 2022 G-register assignments)
/// so that stateful encodings work correctly across line boundaries.
#[derive(Debug)]
pub(crate) struct StreamingDecoder {
    encoding: InputEncoding,
    /// For HZ encoding: are we currently in two-byte mode?
    hz_two_byte: bool,
    /// For ISO 2022 encoding: persistent G-register state.
    iso2022_state: Iso2022State,
}

impl StreamingDecoder {
    /// Create a new streaming decoder for the given encoding and ISO 2022 settings.
    pub(super) fn new(encoding: InputEncoding, iso2022: Option<&Iso2022Settings>) -> Self {
        Self {
            encoding,
            hz_two_byte: false,
            iso2022_state: iso2022.map(Iso2022State::from_settings).unwrap_or_default(),
        }
    }

    /// Decode a slice of bytes into character codes, preserving decoder state.
    pub(crate) fn decode_bytes(&mut self, bytes: &[u8]) -> Vec<i32> {
        match self.encoding {
            InputEncoding::Default | InputEncoding::UTF8 => {
                EncodingDecoder::new(bytes, InputEncoding::UTF8, None).collect()
            }
            InputEncoding::Latin1 => {
                EncodingDecoder::new(bytes, InputEncoding::Latin1, None).collect()
            }
            InputEncoding::Dbcs => {
                EncodingDecoder::new(bytes, InputEncoding::Dbcs, None).collect()
            }
            InputEncoding::ShiftJIS => {
                EncodingDecoder::new(bytes, InputEncoding::ShiftJIS, None).collect()
            }
            InputEncoding::HZ => self.decode_hz(bytes),
            InputEncoding::ISO2022 => self.decode_iso2022(bytes),
        }
    }

    fn decode_hz(&mut self, bytes: &[u8]) -> Vec<i32> {
        let mut result = Vec::new();
        let mut pos = 0;

        while pos < bytes.len() {
            if bytes[pos] == b'~' {
                pos += 1;
                if pos >= bytes.len() {
                    break;
                }
                match bytes[pos] {
                    b'{' => { pos += 1; self.hz_two_byte = true; }
                    b'}' => { pos += 1; self.hz_two_byte = false; }
                    b'~' => { pos += 1; result.push('~' as i32); }
                    _ => { pos += 1; }
                }
                continue;
            }

            if self.hz_two_byte {
                if pos + 1 < bytes.len() {
                    let hi = bytes[pos];
                    let lo = bytes[pos + 1];
                    pos += 2;
                    result.push(((hi as i32) << 8) | (lo as i32));
                } else {
                    result.push(bytes[pos] as i32);
                    pos += 1;
                    self.hz_two_byte = false;
                }
            } else {
                result.push(bytes[pos] as i32);
                pos += 1;
            }
        }

        result
    }

    fn decode_iso2022(&mut self, bytes: &[u8]) -> Vec<i32> {
        let mut ctx = Iso2022DecodeCtx {
            gn: self.iso2022_state.gn,
            gndbl: self.iso2022_state.gndbl,
            gl: self.iso2022_state.gl,
            gr: self.iso2022_state.gr,
            single_shift: self.iso2022_state.single_shift,
            pos: 0,
        };
        let mut result = Vec::new();

        while ctx.pos < bytes.len() {
            let ch = bytes[ctx.pos];
            ctx.pos += 1;

            let handled = match ch {
                0x0E => { ctx.single_shift = None; ctx.gl = 1; true }
                0x0F => { ctx.single_shift = None; ctx.gl = 0; true }
                0x8E => { ctx.single_shift = Some(2); true }
                0x8F => { ctx.single_shift = Some(3); true }
                27 => {
                    ctx.single_shift = None;
                    if ctx.pos >= bytes.len() { break; }
                    let second = bytes[ctx.pos];
                    ctx.pos += 1;
                    Self::handle_escape_iso2022(bytes, &mut ctx, second)
                }
                _ => false,
            };

            if !handled {
                if let Some(code) = Self::decode_char_iso2022(bytes, &mut ctx, ch) {
                    result.push(code);
                }
            }
        }

        self.iso2022_state.gn = ctx.gn;
        self.iso2022_state.gndbl = ctx.gndbl;
        self.iso2022_state.gl = ctx.gl;
        self.iso2022_state.gr = ctx.gr;
        self.iso2022_state.single_shift = ctx.single_shift;

        result
    }

    fn handle_escape_iso2022(bytes: &[u8], ctx: &mut Iso2022DecodeCtx, second: u8) -> bool {
        let (base, third) = if second == b'$' {
            if ctx.pos >= bytes.len() { return true; }
            let t = bytes[ctx.pos];
            ctx.pos += 1;
            (0x200, Some(t))
        } else {
            (0x100, None)
        };
        let ch = third.unwrap_or(second);

        match (base, ch) {
            (0x100, b'N') => { ctx.single_shift = Some(2); }
            (0x100, b'O') => { ctx.single_shift = Some(3); }
            (0x100, b'n') => { ctx.gl = 2; }
            (0x100, b'o') => { ctx.gl = 3; }
            (0x100, b'~') => { ctx.gr = 1; }
            (0x100, b'}') => { ctx.gr = 2; }
            (0x100, b'|') => { ctx.gr = 3; }
            (0x100, b'(') => { Self::designate_iso2022(bytes, ctx, 0, 94); return true; }
            (0x100, b')') => { Self::designate_iso2022(bytes, ctx, 1, 94); return true; }
            (0x100, b'*') => { Self::designate_iso2022(bytes, ctx, 2, 94); return true; }
            (0x100, b'+') => { Self::designate_iso2022(bytes, ctx, 3, 94); return true; }
            (0x100, b'-') => { Self::designate_iso2022(bytes, ctx, 1, 96); return true; }
            (0x100, b'.') => { Self::designate_iso2022(bytes, ctx, 2, 96); return true; }
            (0x100, b'/') => { Self::designate_iso2022(bytes, ctx, 3, 96); return true; }
            (0x200, b'(') => { Self::designate_iso2022(bytes, ctx, 0, 9999); return true; }
            (0x200, b')') => { Self::designate_iso2022(bytes, ctx, 1, 9999); return true; }
            (0x200, b'*') => { Self::designate_iso2022(bytes, ctx, 2, 9999); return true; }
            (0x200, b'+') => { Self::designate_iso2022(bytes, ctx, 3, 9999); return true; }
            _ if base == 0x200 => {
                ctx.gn[0] = (ch as i32) << 16;
                ctx.gndbl[0] = true;
                return true;
            }
            _ => {}
        }
        true
    }

    fn designate_iso2022(bytes: &[u8], ctx: &mut Iso2022DecodeCtx, reg: usize, kind: i32) {
        if ctx.pos >= bytes.len() { return; }
        let d = bytes[ctx.pos] as i32;
        ctx.pos += 1;
        let mut d = d;
        if (kind == 94 && d == b'B' as i32)
            || ((kind == 96 || kind == 9999) && d == b'A' as i32)
        {
            d = 0;
        }
        if kind == 9999 {
            ctx.gn[reg] = d << 16;
            ctx.gndbl[reg] = true;
        } else if kind == 96 {
            ctx.gn[reg] = (d << 16) | 0x80;
            ctx.gndbl[reg] = false;
        } else {
            ctx.gn[reg] = d << 16;
            ctx.gndbl[reg] = false;
        }
    }

    fn decode_char_iso2022(bytes: &[u8], ctx: &mut Iso2022DecodeCtx, ch: u8) -> Option<i32> {
        if (0x21..=0x7E).contains(&ch) {
            let gl_reg = ctx.single_shift.take().unwrap_or(ctx.gl);
            if ctx.gndbl[gl_reg] {
                if ctx.pos < bytes.len() {
                    let ch2 = bytes[ctx.pos] as i32;
                    ctx.pos += 1;
                    Some(ctx.gn[gl_reg] | ((ch as i32) << 8) | ch2)
                } else {
                    Some(ctx.gn[gl_reg] | (ch as i32))
                }
            } else {
                Some(ctx.gn[gl_reg] | (ch as i32))
            }
        } else if (0xA0..=0xFF).contains(&ch) {
            if ctx.gndbl[ctx.gr] {
                if ctx.pos < bytes.len() {
                    let ch2 = bytes[ctx.pos] as i32;
                    ctx.pos += 1;
                    Some(ctx.gn[ctx.gr] | ((ch as i32) << 8) | ch2)
                } else {
                    Some(ctx.gn[ctx.gr] | (ch as i32 & !0x80))
                }
            } else {
                Some(ctx.gn[ctx.gr] | (ch as i32 & !0x80))
            }
        } else {
            Some(ch as i32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(bytes: &[u8], encoding: InputEncoding) -> Vec<i32> {
        EncodingDecoder::new(bytes, encoding, None).collect()
    }

    fn collect_utf8(bytes: &[u8]) -> Vec<i32> {
        collect(bytes, InputEncoding::UTF8)
    }

    fn collect_dbcs(bytes: &[u8]) -> Vec<i32> {
        collect(bytes, InputEncoding::Dbcs)
    }

    fn collect_shiftjis(bytes: &[u8]) -> Vec<i32> {
        collect(bytes, InputEncoding::ShiftJIS)
    }

    fn collect_hz(bytes: &[u8]) -> Vec<i32> {
        collect(bytes, InputEncoding::HZ)
    }

    /* UTF-8 tests */

    #[test]
    fn utf8_empty_returns_none() {
        assert_eq!(collect_utf8(&[]), vec![]);
    }

    #[test]
    fn utf8_ascii_single_byte() {
        assert_eq!(collect_utf8(b"ABC"), vec![65, 66, 67]);
    }

    #[test]
    fn utf8_ascii_null() {
        assert_eq!(collect_utf8(&[0x00]), vec![0]);
    }

    #[test]
    fn utf8_2byte_sequence() {
        // U+00E9 = e-acute, encoded as 0xC3 0xA9
        assert_eq!(collect_utf8(&[0xC3, 0xA9]), vec![0xE9]);
    }

    #[test]
    fn utf8_3byte_sequence() {
        // U+0F60 = Tibetan vowel sign a, encoded as 0xE0 0xBD 0xA0
        assert_eq!(collect_utf8(&[0xE0, 0xBD, 0xA0]), vec![0xF60]);
    }

    #[test]
    fn utf8_4byte_sequence() {
        // U+1F600 = grinning face, encoded as 0xF0 0x9F 0x98 0x80
        assert_eq!(collect_utf8(&[0xF0, 0x9F, 0x98, 0x80]), vec![0x1F600]);
    }

    #[test]
    fn utf8_max_codepoint() {
        // U+10FFFF, encoded as 0xF4 0x8F 0xBF 0xBF
        assert_eq!(collect_utf8(&[0xF4, 0x8F, 0xBF, 0xBF]), vec![0x10FFFF]);
    }

    #[test]
    fn utf8_overlong_1byte_as_2byte() {
        // 0xC0 0xAF is overlong encoding of '/', lead byte rejected (128),
        // continuation byte 0xAF is bare and also yields 128
        assert_eq!(collect_utf8(&[0xC0, 0xAF]), vec![128, 128]);
    }

    #[test]
    fn utf8_overlong_2byte_as_3byte() {
        // 0xE0 0x80 0xAF is overlong encoding of '/', lead byte rejected (128),
        // remaining two continuation bytes each yield 128
        assert_eq!(collect_utf8(&[0xE0, 0x80, 0xAF]), vec![128, 128, 128]);
    }

    #[test]
    fn utf8_overlong_3byte_as_4byte() {
        // 0xF0 0x80 0x80 0xAF is overlong encoding of '/', lead byte rejected (128),
        // remaining three continuation bytes each yield 128
        assert_eq!(collect_utf8(&[0xF0, 0x80, 0x80, 0xAF]), vec![128, 128, 128, 128]);
    }

    #[test]
    fn utf8_surrogate_low_rejected() {
        // U+D800 is a surrogate, encoded as 0xED 0xA0 0x80,
        // lead byte rejected (128), two continuation bytes each yield 128
        assert_eq!(collect_utf8(&[0xED, 0xA0, 0x80]), vec![128, 128, 128]);
    }

    #[test]
    fn utf8_surrogate_high_rejected() {
        // U+DFFF is a surrogate, encoded as 0xED 0xBF 0xBF,
        // lead byte rejected (128), two continuation bytes each yield 128
        assert_eq!(collect_utf8(&[0xED, 0xBF, 0xBF]), vec![128, 128, 128]);
    }

    #[test]
    fn utf8_invalid_continuation_byte() {
        // 0xC2 followed by non-continuation byte 0x00
        assert_eq!(collect_utf8(&[0xC2, 0x00]), vec![128, 0]);
    }

    #[test]
    fn utf8_bare_continuation_byte() {
        // 0x80 is a bare continuation byte
        assert_eq!(collect_utf8(&[0x80]), vec![128]);
    }

    #[test]
    fn utf8_bare_continuation_byte_ff() {
        assert_eq!(collect_utf8(&[0xFF]), vec![128]);
    }

    #[test]
    fn utf8_truncated_2byte() {
        assert_eq!(collect_utf8(&[0xC3]), vec![128]);
    }

    #[test]
    fn utf8_truncated_3byte() {
        assert_eq!(collect_utf8(&[0xE0, 0xBD]), vec![128, 128]);
    }

    #[test]
    fn utf8_truncated_4byte() {
        assert_eq!(collect_utf8(&[0xF0, 0x9F, 0x98]), vec![128, 128, 128]);
    }

    #[test]
    fn utf8_codepoint_above_max() {
        // 0xF5 0x80 0x80 0x80 decodes to 0x110000, above U+10FFFF,
        // lead byte rejected (128), three continuation bytes each yield 128
        assert_eq!(collect_utf8(&[0xF5, 0x80, 0x80, 0x80]), vec![128, 128, 128, 128]);
    }

    #[test]
    fn utf8_mixed_valid_and_invalid() {
        // 'A' (0x41) + valid 2-byte (0xC3 0xA9 = U+00E9) + bare continuation (0x80) + 'B' (0x42)
        assert_eq!(collect_utf8(&[0x41, 0xC3, 0xA9, 0x80, 0x42]), vec![65, 0xE9, 128, 66]);
    }

    /* DBCS tests */

    #[test]
    fn dbcs_empty_returns_none() {
        assert_eq!(collect_dbcs(&[]), vec![]);
    }

    #[test]
    fn dbcs_ascii_bytes() {
        assert_eq!(collect_dbcs(b"AB"), vec![65, 66]);
    }

    #[test]
    fn dbcs_high_byte_pair() {
        assert_eq!(collect_dbcs(&[0xA1, 0xA1]), vec![0xA1A1]);
    }

    #[test]
    fn dbcs_mixed_ascii_and_high_bytes() {
        assert_eq!(collect_dbcs(&[0x41, 0xA1, 0xA1, 0x42]), vec![0x41, 0xA1A1, 0x42]);
    }

    #[test]
    fn dbcs_trailing_stray_high_byte() {
        // High byte at end of input with no low byte partner
        assert_eq!(collect_dbcs(&[0x41, 0xA1]), vec![0x41, 0xA1]);
    }

    #[test]
    fn dbcs_multiple_pairs() {
        assert_eq!(
            collect_dbcs(&[0x81, 0x40, 0x81, 0x41, 0x81, 0x42]),
            vec![0x8140, 0x8141, 0x8142]
        );
    }

    /* Shift-JIS tests */

    #[test]
    fn shiftjis_empty_returns_none() {
        assert_eq!(collect_shiftjis(&[]), vec![]);
    }

    #[test]
    fn shiftjis_ascii_bytes() {
        assert_eq!(collect_shiftjis(b"AB"), vec![65, 66]);
    }

    #[test]
    fn shiftjis_high_byte_range_1() {
        // 0x80-0x9F range
        assert_eq!(collect_shiftjis(&[0x81, 0x40]), vec![0x8140]);
    }

    #[test]
    fn shiftjis_high_byte_range_2() {
        // 0xE0-0xEF range
        assert_eq!(collect_shiftjis(&[0xEA, 0x40]), vec![0xEA40]);
    }

    #[test]
    fn shiftjis_non_high_byte_treated_as_single() {
        // 0xA0-0xDF and 0xF0-0xFF are NOT high bytes in Shift-JIS
        assert_eq!(collect_shiftjis(&[0xA0, 0xA1]), vec![0xA0, 0xA1]);
    }

    #[test]
    fn shiftjis_mixed_content() {
        // ASCII + high-byte pair + ASCII
        assert_eq!(collect_shiftjis(&[0x41, 0x81, 0x40, 0x42]), vec![0x41, 0x8140, 0x42]);
    }

    #[test]
    fn shiftjis_trailing_high_byte() {
        assert_eq!(collect_shiftjis(&[0x81]), vec![0x81]);
    }

    #[test]
    fn shiftjis_high_byte_boundaries() {
        // 0x9F is last byte of first high-byte range
        assert_eq!(collect_shiftjis(&[0x9F, 0x40]), vec![0x9F40]);
        // 0xEF is last byte of second high-byte range
        assert_eq!(collect_shiftjis(&[0xEF, 0x40]), vec![0xEF40]);
        // 0x7F is NOT a high byte
        assert_eq!(collect_shiftjis(&[0x7F, 0x40]), vec![0x7F, 0x40]);
    }

    /* HZ tests */

    #[test]
    fn hz_empty_returns_none() {
        assert_eq!(collect_hz(&[]), vec![]);
    }

    #[test]
    fn hz_ascii_default_mode() {
        assert_eq!(collect_hz(b"AB"), vec![65, 66]);
    }

    #[test]
    fn hz_enter_two_byte_mode() {
        // ~{ enters two-byte mode, then two bytes form a pair
        assert_eq!(collect_hz(&[b'~', b'{', 0xA1, 0xA1]), vec![0xA1A1]);
    }

    #[test]
    fn hz_exit_two_byte_mode() {
        // ~{ enters two-byte mode, ~} exits back to ASCII mode
        assert_eq!(
            collect_hz(&[b'~', b'{', 0xA1, 0xA1, b'~', b'}', b'A']),
            vec![0xA1A1, 65]
        );
    }

    #[test]
    fn hz_tilde_tilde_escape() {
        // ~~ produces a literal tilde
        assert_eq!(collect_hz(&[b'~', b'~']), vec![b'~' as i32]);
    }

    #[test]
    fn hz_stray_tilde_x() {
        // ~X (where X is not {, }, or ~) is removed from input per spec
        assert_eq!(collect_hz(&[b'~', b'X', b'A']), vec![65]);
    }

    #[test]
    fn hz_trailing_tilde_at_end() {
        // Lone ~ at end of input: consumed silently
        assert_eq!(collect_hz(&[b'~']), vec![]);
    }

    #[test]
    fn hz_two_byte_trailing_single_byte() {
        // In two-byte mode, odd byte at end yields single byte
        assert_eq!(
            collect_hz(&[b'~', b'{', 0xA1, 0xA2, 0xA3]),
            vec![0xA1A2, 0xA3]
        );
    }

    #[test]
    fn hz_multiple_mode_switches() {
        // Switch modes multiple times
        assert_eq!(
            collect_hz(&[
                b'A',                   // ASCII: 65
                b'~', b'{',             // Enter two-byte
                0xA1, 0xA2,             // Two-byte: 0xA1A2
                b'~', b'}',             // Exit two-byte
                b'B',                   // ASCII: 66
                b'~', b'{',             // Enter two-byte again
                0xB1, 0xB2,             // Two-byte: 0xB1B2
            ]),
            vec![65, 0xA1A2, 66, 0xB1B2]
        );
    }

    #[test]
    fn hz_stray_tilde_preserves_mode() {
        // ~{ enters two-byte, ~X is removed silently without affecting mode,
        // subsequent bytes are still decoded as two-byte pairs
        assert_eq!(
            collect_hz(&[b'~', b'{', b'~', b'X', 0xA1, 0xA2]),
            vec![0xA1A2]
        );
    }

    /* Latin1 tests */

    #[test]
    fn latin1_basic() {
        assert_eq!(collect(&[0x41, 0x42, 0x43], InputEncoding::Latin1), vec![0x41, 0x42, 0x43]);
    }

    #[test]
    fn latin1_high_bytes() {
        assert_eq!(collect(&[0xE9, 0xF1], InputEncoding::Latin1), vec![0xE9, 0xF1]);
    }

    #[test]
    fn latin1_empty() {
        assert_eq!(collect(&[], InputEncoding::Latin1), vec![]);
    }

    /* StreamingDecoder tests - cross-line state preservation */

    #[test]
    fn streaming_hz_two_byte_persists_across_lines() {
        // ~{ on line 1 should keep two-byte mode for line 2
        let mut dec = StreamingDecoder::new(InputEncoding::HZ, None);
        let line1 = dec.decode_bytes(&[b'~', b'{', 0xA1, 0xA2]);
        let line2 = dec.decode_bytes(&[0xB1, 0xB2]);
        assert_eq!(line1, vec![0xA1A2]);
        assert_eq!(line2, vec![0xB1B2]);
    }

    #[test]
    fn streaming_hz_exit_mode_persists_across_lines() {
        // ~} on line 2 should exit two-byte mode started on line 1
        let mut dec = StreamingDecoder::new(InputEncoding::HZ, None);
        let line1 = dec.decode_bytes(&[b'~', b'{', 0xA1, 0xA2]);
        let line2 = dec.decode_bytes(&[b'~', b'}', b'A', b'B']);
        assert_eq!(line1, vec![0xA1A2]);
        assert_eq!(line2, vec![65, 66]);
    }

    #[test]
    fn streaming_utf8_no_state_preservation_needed() {
        // UTF-8 has no state, streaming should work identically to per-call
        let mut dec = StreamingDecoder::new(InputEncoding::UTF8, None);
        assert_eq!(dec.decode_bytes(b"Hello"), vec![72, 101, 108, 108, 111]);
        assert_eq!(dec.decode_bytes(b"World"), vec![87, 111, 114, 108, 100]);
    }

    #[test]
    fn streaming_latin1_no_state_preservation_needed() {
        let mut dec = StreamingDecoder::new(InputEncoding::Latin1, None);
        assert_eq!(dec.decode_bytes(&[0xE9, 0xF1]), vec![0xE9, 0xF1]);
        assert_eq!(dec.decode_bytes(&[0x41, 0x42]), vec![0x41, 0x42]);
    }

    #[test]
    fn streaming_iso2022_gregister_persists_across_lines() {
        // ESC(B designates G0 as US-ASCII on line 1,
        // line 2 should still use the default G0
        let mut dec = StreamingDecoder::new(InputEncoding::ISO2022, None);
        let line1 = dec.decode_bytes(b"ABC");
        let line2 = dec.decode_bytes(b"DEF");
        assert_eq!(line1, vec![65, 66, 67]);
        assert_eq!(line2, vec![68, 69, 70]);
    }

    #[test]
    fn streaming_iso2022_esc_sequence_on_line() {
        // ESC sequence in the middle of line
        let mut dec = StreamingDecoder::new(InputEncoding::ISO2022, None);
        // ESC n switches GL to G2
        let line1 = dec.decode_bytes(&[0x1B, b'n', b'A', b'B']);
        assert_eq!(line1, vec![65, 66]);
    }

    #[test]
    fn streaming_iso2022_so_si_persists_across_lines() {
        // SO switches GL to G1 on line 1, should persist to line 2
        // Default G1 = 0x80, so 'A' -> 0x80 | 0x41 = 0xC1 = 193
        let mut dec = StreamingDecoder::new(InputEncoding::ISO2022, None);
        let line1 = dec.decode_bytes(&[0x0E, b'A']);
        let line2 = dec.decode_bytes(&[b'B']);
        assert_eq!(line1, vec![0x80 | 0x41]);
        assert_eq!(line2, vec![0x80 | 0x42]); // GL=G1 persists from line 1
    }

    #[test]
    fn streaming_iso2022_esc_n_ss2() {
        // Designate G2 with ESC * D, then ESC N invokes it for one char
        // gn[2] = 0x44 << 16 = 0x440000, so 'A' -> 0x440041, 'B' restores to G0 -> 0x42
        let mut dec = StreamingDecoder::new(InputEncoding::ISO2022, None);
        let result = dec.decode_bytes(&[0x1B, b'*', b'D', 0x1B, b'N', b'A', b'B']);
        assert_eq!(result, vec![0x440041, 0x42]);
    }

    #[test]
    fn streaming_iso2022_esc_o_ss3() {
        // Designate G3 with ESC + E, then ESC O invokes it for one char
        // gn[3] = 0x45 << 16 = 0x450000, so 'A' -> 0x450041, 'B' restores to G0 -> 0x42
        let mut dec = StreamingDecoder::new(InputEncoding::ISO2022, None);
        let result = dec.decode_bytes(&[0x1B, b'+', b'E', 0x1B, b'O', b'A', b'B']);
        assert_eq!(result, vec![0x450041, 0x42]);
    }

    #[test]
    fn streaming_iso2022_94x94_double_byte() {
        // ESC $( X designates G0 as 94x94 with designator X
        // Then 'A' 'B' should produce double-byte code 0x584142
        let mut dec = StreamingDecoder::new(InputEncoding::ISO2022, None);
        let result = dec.decode_bytes(&[0x1B, b'$', b'(', b'X', b'A', b'B']);
        assert_eq!(result, vec![0x584142]);
    }
}
