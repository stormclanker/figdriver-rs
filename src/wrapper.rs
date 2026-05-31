use crate::Error;
use crate::Smusher;

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Right,
    Center,
}

/// Render smushed ASCII-art characters with word wrapping.
///
/// Wrapper receives string or character input and renders the corresponding
/// FIGcharacters if the output text fits inside the maximum width specified on
/// creation. The wrapper will flush the output buffer earlier if the line is
/// too long, thus producing multiple "lines" of output text.
pub struct Wrapper<'a> {
    sm            : Smusher<'a>,    // the FIGcharacter smusher
    buffer        : String,         // buffer to keep our input text
    pending_space : Option<String>, // accumulated whitespace to commit with next word
    just_flushed  : bool,           // true after a wrap flush, suppresses leading space/whitespace
    pub width     : usize,          // terminal width
    pub align     : Align,          // text alignment
}

impl<'a> Wrapper<'a> {
    /// Create a new wrapper using the specified Smusher and terminal width.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn foo() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a smusher using the specified FIGfont
    /// let font = figdriver::FIGfont::from_path("small.flf")?;
    /// let mut sm = figdriver::Smusher::new(&font);
    ///
    /// // Create a line wrapper using our smusher and maximum width of 80 columns
    /// let mut wr = figdriver::Wrapper::new(sm, 80);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(sm: Smusher<'a>, width: usize) -> Self {
        Wrapper{
            sm,
            width,
            buffer        : String::new(),
            align         : Align::Left,
            pending_space : None,
            just_flushed  : false,
        }
    }

    /// Clear the output buffer.
    pub fn clear(&mut self) {
        self.sm.clear();
        self.buffer.clear();
        self.pending_space = None;
        self.just_flushed = false;
    }

    /// Retrieve the output buffer lines.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn foo() -> Result<(), Box<dyn std::error::Error>> {
    /// // Create a new wrapper
    /// let mut font = figdriver::FIGfont::from_path("small.flf")?;
    /// let mut wr = figdriver::Wrapper::new(figdriver::Smusher::new(&font), 80);
    ///
    /// // Add a string to the output buffer
    /// wr.push_str("hello")?;
    ///
    /// // Get and print the current output buffer contents
    /// for line in &wr.get() {
    ///     println!("{}", line);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&mut self) -> Vec<String> {
        if self.len() > self.width {
            self.sm.trim(self.width);
        }

        let mut v = self.sm.get_raw();

        // Pad the block to its own widest line (figlet convention), then replace
        // hardblanks with spaces (figlet never trims output).
        let max_w = v.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        for line in &mut v {
            let len = line.chars().count();
            if len < max_w {
                line.extend(std::iter::repeat_n(' ', max_w - len));
            }
        }
        self.sm.replace_hardblanks(&mut v);

        let w = self.width.saturating_sub(max_w);

        match self.align {
            Align::Left   => v.to_vec(),
            Align::Center => add_pad(&v, w / 2 + w % 2),
            Align::Right  => add_pad(&v, w),
        }
    }

    /// Get the length in sub-characters of the current output buffer.
    pub fn len(&self) -> usize {
        self.sm.len()
    }

    /// Verify whether the output buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.sm.is_empty()
    }

    /// Add a string to the output buffer.
    ///
    /// # Errors
    ///
    /// If adding the string results in a line wider than the maximum number of columns,
    /// the string is not added to the output buffer and a LineFull error is returned.
    pub fn push_str(&mut self, s: &str) -> Result<(), Error> {
        let rendered = self.sm.push_str(s);

        if self.sm.len() > self.width {
            self.sm.clear();
            self.sm.push_str(&self.buffer);
            return Err(Error::LineFull)
        }

        self.buffer.push_str(&rendered);
        Ok(())
    }

    /// Add a character to the output buffer.
    ///
    /// # Errors
    ///
    /// If adding the character results in a line wider than the maximum number of columns,
    /// the character is not added to the output buffer and a LineFull error is returned.
    pub fn push(&mut self, ch: char) -> Result<(), Error> {
        let rendered = self.sm.push(ch);

        if self.sm.len() > self.width {
            self.sm.clear();
            self.sm.push_str(&self.buffer);
            return Err(Error::LineFull)
        }

        if rendered {
            self.buffer.push(ch);
        }
        Ok(())
    }

    /// Add a string to the output buffer, wrapping it if necessary.
    ///
    /// If the new string causes the output to be wider than the maximum width, the current
    /// buffer contents (if any) will be passed to the flush callback, the buffer will be
    /// cleared, and the new string will be added to the buffer. If the string is wider
    /// than the output buffer, it will be wrapped at character level.
    ///
    /// Explicit newline characters ('\n') in the input force a flush of the current
    /// buffer, starting a new output line (figfont.txt lines 1617-1621).
    pub fn wrap_str(&mut self, s: &str, flush: &dyn Fn(&[String])) {
        // Handle explicit newlines per spec (figfont.txt lines 1617-1621):
        // When input contains newlines, flush the current buffer as a complete line
        // and continue wrapping subsequent text on a new line.
        // Normalize CRLF and bare CR to LF for cross-platform compatibility.
        let normalized = s.replace("\r\n", "\n").replace("\r", "\n");
        if normalized.contains('\n') {
            let segments: Vec<&str> = normalized.split('\n').collect();
            let num_segments = segments.len();

            for (i, segment) in segments.iter().enumerate() {
                if i > 0 {
                    // Flush current buffer as a complete line after newline
                    if !self.is_empty() {
                        flush(&self.get());
                        self.clear();
                    }
                    // Consecutive newlines produce blank lines
                    if segment.is_empty() && i < num_segments - 1 {
                        flush(&self.get());
                        self.clear();
                    }
                }

                if segment.is_empty() {
                    continue;
                }

                self.wrap_segment(segment, flush);
            }
            return;
        }

        self.wrap_segment(s, flush);
    }

    /// Internal wrapper logic for a single segment (no newlines).
    fn wrap_segment(&mut self, s: &str, flush: &dyn Fn(&[String])) {
        let empty = s.trim().is_empty();

        // Handle whitespace tokens per spec (figfont.txt lines 1623-1633):
        // - At wrap points: discard all blanks until next non-blank character
        // - At input start or after linebreak: preserve blanks as FIGcharacters
        // - Between words: accumulate whitespace, commit when next word arrives
        if empty {
            if self.just_flushed {
                return;
            }
            if self.buffer.is_empty() {
                self.push_str(s).ok();
                return;
            }
            self.pending_space = Some(self.pending_space.clone().unwrap_or_default() + s);
            return;
        }

        let after_wrap = std::mem::replace(&mut self.just_flushed, false);
        let space = self.pending_space.take();

        // Commit accumulated whitespace if we're not at a wrap point.
        // Save buffer state before spaces so that, if the subsequent word push
        // fails, the flush outputs content without trailing whitespace (spec
        // says blanks at wrap points are discarded).
        if let Some(ref sp) = space {
            if !after_wrap && !self.buffer.is_empty() {
                let pre_space_buffer = self.buffer.clone();
                if self.push_str(sp).is_err() {
                    flush(&self.get());
                    self.clear();
                    self.push_str(sp).ok();
                }
                if self.push_str(s).is_err() {
                    self.sm.clear();
                    self.sm.push_str(&pre_space_buffer);
                    flush(&self.get());
                    self.clear();
                    if self.push_str(s).is_err() {
                        self.wrap_word(s, flush);
                        self.just_flushed = false;
                        return;
                    }
                }
            }
        } else if self.push_str(s).is_err() {
            if !self.buffer.is_empty() {
                flush(&self.get());
                self.clear();
            }
            if self.push_str(s).is_err() {
                self.wrap_word(s, flush);
                self.just_flushed = false;
                return;
            }
        }
    }

    /// Add a word to the output buffer, breaking it if necessary.
    ///
    /// Add this word to the output character by character. If a new character causes the
    /// output to be wider than the maximum width, the current buffer contents (if any) will
    /// be passed to the flush callback, the buffer will be cleared, and the new character
    /// will be added to the buffer. If the character is wider than the maximum width, it
    /// will be added without any additional processing.
    pub fn wrap_word(&mut self, word: &str, flush: &dyn Fn(&[String])) {
        for c in word.chars() {
            if self.push(c).is_err() {
                if !self.buffer.is_empty() {
                    flush(&self.get());
                    self.clear();
                    self.just_flushed = true;
                }
                if self.sm.push(c) {
                    self.buffer.push(c);
                }
            }
        }
    }
}

fn add_pad(v: &[String], pad_size: usize) -> Vec<String> {
    let p: String = (0..pad_size).map(|_| " ").collect();
    v.iter().map(|x| p.clone() + x).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FIGfont;

    macro_rules! vec_string {
        ( $($x:expr),* ) => (vec![$($x.to_string()),*])
    }

    fn test_font() -> Result<FIGfont, Error> {
        FIGfont::from_path(env!("CARGO_MANIFEST_DIR").to_owned() + "/fonts/small.flf")
    }

    #[test]
    fn test_padding() {
        assert_eq!(add_pad(&vec_string!("x", "x"), 0), vec_string!("x", "x"));
        assert_eq!(add_pad(&vec_string!("x", "x"), 4), vec_string!("    x", "    x"));
    }

    #[test]
    fn test_padding_utf8() {
        assert_eq!(add_pad(&vec_string!("á", "á"), 0), vec_string!("á", "á"));
        assert_eq!(add_pad(&vec_string!("á", "á"), 4), vec_string!("    á", "    á"));
    }

    #[test]
    fn test_newline_splits_lines() {
        use std::cell::RefCell;
        let font = test_font().unwrap();
        let sm = Smusher::new(&font);
        let mut wr = Wrapper::new(sm, 80);
        let flushed = RefCell::new(Vec::new());

        wr.wrap_str("hello\nworld", &|lines: &[String]| flushed.borrow_mut().push(lines.to_vec()));

        let flushed = flushed.borrow();
        assert_eq!(flushed.len(), 1, "newline should flush the first line");
        assert!(!flushed[0][0].is_empty(), "flushed line should contain rendered content");

        let remaining = wr.get();
        assert!(!remaining[0].is_empty(), "remaining buffer should contain rendered world");
    }

    #[test]
    fn test_consecutive_newlines_produce_blank_lines() {
        use std::cell::RefCell;
        let font = test_font().unwrap();
        let sm = Smusher::new(&font);
        let mut wr = Wrapper::new(sm, 80);
        let flushed = RefCell::new(Vec::new());

        wr.wrap_str("hello\n\nworld", &|lines: &[String]| flushed.borrow_mut().push(lines.to_vec()));

        let flushed = flushed.borrow();
        assert_eq!(flushed.len(), 2, "consecutive newlines should flush content and blank line");
        assert!(!flushed[0][0].is_empty(), "first flush should contain rendered content");
        assert!(flushed[1][0].is_empty(), "second flush should be blank line");

        let remaining = wr.get();
        assert!(!remaining[0].is_empty(), "remaining buffer should contain rendered world");
    }

    #[test]
    fn test_leading_whitespace_after_newline_preserved() {
        use std::cell::RefCell;
        let font = test_font().unwrap();
        let sm = Smusher::new(&font);
        let mut wr = Wrapper::new(sm, 80);
        let flushed = RefCell::new(Vec::new());

        wr.wrap_str("hello\n  world", &|lines: &[String]| flushed.borrow_mut().push(lines.to_vec()));

        let flushed = flushed.borrow();
        assert_eq!(flushed.len(), 1, "should flush the first line");
        assert!(!flushed[0][0].is_empty(), "flushed line should contain rendered content");

        let remaining = wr.get();
        assert!(remaining[0].starts_with("  "), "leading whitespace after newline should be preserved");
    }

    #[test]
    fn test_crlf_and_bare_cr_handled_as_newlines() {
        use std::cell::RefCell;
        let font = test_font().unwrap();

        // CRLF
        let sm = Smusher::new(&font);
        let mut wr = Wrapper::new(sm, 80);
        let flushed = RefCell::new(Vec::new());
        wr.wrap_str("hello\r\nworld", &|lines: &[String]| flushed.borrow_mut().push(lines.to_vec()));
        assert_eq!(flushed.borrow().len(), 1, "CRLF should flush the first line");

        // Bare CR
        let sm = Smusher::new(&font);
        let mut wr = Wrapper::new(sm, 80);
        let flushed = RefCell::new(Vec::new());
        wr.wrap_str("hello\rworld", &|lines: &[String]| flushed.borrow_mut().push(lines.to_vec()));
        assert_eq!(flushed.borrow().len(), 1, "bare CR should flush the first line");
    }

    #[test]
    fn test_leading_newline_no_blank_line() {
        use std::cell::RefCell;
        let font = test_font().unwrap();
        let sm = Smusher::new(&font);
        let mut wr = Wrapper::new(sm, 80);
        let flushed = RefCell::new(Vec::new());

        wr.wrap_str("\nworld", &|lines: &[String]| flushed.borrow_mut().push(lines.to_vec()));

        let flushed = flushed.borrow();
        assert_eq!(flushed.len(), 0, "leading newline should not flush blank line");

        let remaining = wr.get();
        assert!(!remaining[0].is_empty(), "remaining buffer should contain rendered world");
    }

    #[test]
    fn test_trailing_newline_no_blank_line() {
        use std::cell::RefCell;
        let font = test_font().unwrap();
        let sm = Smusher::new(&font);
        let mut wr = Wrapper::new(sm, 80);
        let flushed = RefCell::new(Vec::new());

        wr.wrap_str("hello\n", &|lines: &[String]| flushed.borrow_mut().push(lines.to_vec()));

        let flushed = flushed.borrow();
        assert_eq!(flushed.len(), 1, "trailing newline should flush the content line once");
        assert!(!flushed[0][0].is_empty(), "flushed line should contain rendered content");

        assert!(wr.is_empty(), "buffer should be empty after trailing newline");
    }
}
