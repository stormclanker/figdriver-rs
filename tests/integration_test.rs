use std::path;

macro_rules! new_smusher {
    ( $a: ident, $b: expr ) => {
        let path = env!("CARGO_MANIFEST_DIR").to_owned() + &path::MAIN_SEPARATOR.to_string() + $b;
        let font = figdriver::FIGfont::from_path(path).unwrap();
        let mut $a = figdriver::Smusher::new(&font);
        $a.mode = font.layout;
    }
}

fn dummy(_: &[String]) {
}

#[test]
fn line_full() {
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 8);
    assert!(!wr.push_str("this").is_err());
    assert!(!wr.push_str(" ").is_err());
    assert!(!wr.push_str("is").is_err());
    assert!(!wr.push_str(" ").is_err());
    assert!(wr.push_str("a").is_err());
    assert_eq!(wr.get(), vec!["this is "]);
}

#[test]
fn line_wrap() {
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 8);
    [ "this", " ", "is", " ", "a", " ", "test" ].iter().for_each(|x| wr.wrap_str(&x, &dummy));
    assert_eq!(wr.get(), vec!["a test"]);
}

#[test]
fn wrap_align_left() {
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 12);
    wr.align = figdriver::Align::Left;
    [ "this", " ", "is", " ", "a", " ", "new", " ", "test" ].iter().for_each(|x| wr.wrap_str(&x, &dummy));
    assert_eq!(wr.get(), vec!["new test"]);
}

#[test]
fn wrap_align_center() {
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 12);
    wr.align = figdriver::Align::Center;
    [ "this", " ", "is", " ", "a", " ", "new", " ", "test" ].iter().for_each(|x| wr.wrap_str(&x, &dummy));
    assert_eq!(wr.get(), vec!["  new test"]);
}

#[test]
fn wrap_align_right() {
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 12);
    wr.align = figdriver::Align::Right;
    [ "this", " ", "is", " ", "a", " ", "new", " ", "test" ].iter().for_each(|x| wr.wrap_str(&x, &dummy));
    assert_eq!(wr.get(), vec!["    new test"]);
}

#[test]
fn standard_font_char() {
    new_smusher!(sm, "fonts/standard.flf");
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push('A').is_err());
    assert_eq!(wr.get(), vec![r"    _    ",
                              r"   / \   ",
                              r"  / _ \  ",
                              r" / ___ \ ",
                              r"/_/   \_\",
                              r"         "]);
}

#[test]
fn smushing() {
    new_smusher!(sm, "fonts/small.flf");
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Smushy").is_err());
    assert_eq!(wr.get(), vec![r" ___              _        ",
                              r"/ __|_ __ _  _ __| |_ _  _ ",
                              r"\__ \ '  \ || (_-< ' \ || |",
                              r"|___/_|_|_\_,_/__/_||_\_, |",
                              r"                      |__/ "]);
}

#[test]
fn kerning() {
    new_smusher!(sm, "fonts/small.flf");
    sm.mode = figdriver::SMUSH_KERN;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Kerning").is_err());
    assert_eq!(wr.get(), vec![r" _  __                 _             ",
                              r"| |/ / ___  _ _  _ _  (_) _ _   __ _ ",
                              r"| ' < / -_)| '_|| ' \ | || ' \ / _` |",
                              r"|_|\_\\___||_|  |_||_||_||_||_|\__, |",
                              r"                               |___/ "]);
}

#[test]
fn overlap() {
    new_smusher!(sm, "fonts/standard.flf");
    sm.mode = 0;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Over Write").is_err());
    assert_eq!(wr.get(), vec![r"  ___                __        __    _ _       ",
                              r" / _ \__   _____ _ __\ \      / _ __(_| |_ ___ ",
                              r"| | | \ \ / / _ | '__|\ \ /\ / | '__| | __/ _ \",
                              r"| |_| |\ V |  __| |    \ V  V /| |  | | ||  __/",
                              r" \___/  \_/ \___|_|     \_/\_/ |_|  |_|\__\___|",
                              r"                                               "]);
}

#[test]
fn full_width() {
    new_smusher!(sm, "fonts/small.flf");
    sm.full_width = true;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Full width").is_err());
    assert_eq!(wr.get(), vec![r"  ___          _   _              _      _   _     _    ",
                              r" | __|  _  _  | | | |   __ __ __ (_)  __| | | |_  | |_  ",
                              r" | _|  | || | | | | |   \ V  V / | | / _` | |  _| | ' \ ",
                              r" |_|    \_,_| |_| |_|    \_/\_/  |_| \__,_|  \__| |_||_|",
                              r"                                                        "]);
}

#[test]
fn utf8_input() {
    new_smusher!(sm, "fonts/standard.flf");
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Ação! ಠ_ಠ").is_err());
    assert_eq!(wr.get(), vec![r"    _        /\/|       _    _____)      _____)",
                              r"   / \   ___|/\/_  ___ | |  /_ ___/     /_ ___/",
                              r"  / _ \ / __/ _` |/ _ \| |  / _ \       / _ \  ",
                              r" / ___ \ (_| (_| | (_) |_| | (_) |     | (_) | ",
                              r"/_/   \_\___\__,_|\___/(_)  \___/ _____ \___/  ",
                              r"         )_)                     |_____|       "]);
}

#[test]
fn right_to_left() {
    new_smusher!(sm, "fonts/small.flf");
    sm.right2left = true;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    wr.align = figdriver::Align::Right;
    assert!(!wr.push_str("ABC").is_err());
    let output = wr.get();
    // RTL + right-aligned: content is right-aligned with left padding
    assert_eq!(output[0].len(), 60);
    assert!(output[0].ends_with("___ ___   _   "));
}

#[test]
fn consecutive_blanks_collapsed_at_wrap() {
    // Multiple blanks at a wrap point should be discarded per spec.
    // With preserved whitespace, "this   is" exceeds width 8, so "this" wraps alone.
    // Then "is a" fits, but "is a test" exceeds, so "is a" wraps.
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 8);
    [ "this", "   ", "is", " ", "a", " ", "test" ].iter().for_each(|x| wr.wrap_str(x, &dummy));
    assert_eq!(wr.get(), vec!["test"]);
}

#[test]
fn leading_blanks_preserved() {
    // Blanks at the start of input should be rendered as FIGcharacters.
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 30);
    wr.wrap_str("   ", &dummy);
    wr.wrap_str("x", &dummy);
    assert_eq!(wr.get(), vec!["   x"]);
}

#[test]
fn rtl_consecutive_blanks_collapsed_at_wrap() {
    // Multiple blanks at a wrap point should be discarded in RTL mode.
    // In RTL mode with the test font, text is reversed (chars prepended left).
    new_smusher!(sm, "tests/test.flf");
    sm.right2left = true;
    let mut wr = figdriver::Wrapper::new(sm, 8);
    [ "this", "   ", "is", " ", "a", " ", "test" ].iter().for_each(|x| wr.wrap_str(x, &dummy));
    // With preserved whitespace, wrapping happens differently. "test" → RTL "tset"
    assert_eq!(wr.get(), vec!["tset"]);
}

#[test]
fn rtl_leading_blanks_preserved() {
    // Blanks at the start of input should be rendered as FIGcharacters in RTL mode.
    // In RTL, leading blanks become trailing rendered spaces, preserved in output (figlet behavior).
    new_smusher!(sm, "tests/test.flf");
    sm.right2left = true;
    let mut wr = figdriver::Wrapper::new(sm, 30);
    wr.wrap_str("   ", &dummy);
    wr.wrap_str("x", &dummy);
    // After RTL smushing: "x   " (x prepended left of 3 spaces)
    assert_eq!(wr.get(), vec!["x   "]);
}

#[test]
fn rtl_inter_word_blanks_preserved() {
    // Multiple blanks between words should be preserved in RTL mode.
    new_smusher!(sm, "tests/test.flf");
    sm.right2left = true;
    let mut wr = figdriver::Wrapper::new(sm, 30);
    [ "a", "   ", "b" ].iter().for_each(|x| wr.wrap_str(x, &dummy));
    assert_eq!(wr.get(), vec!["b   a"]);
}

#[test]
fn rtl_blank_after_wrap_discarded() {
    // Whitespace immediately after a flush should be discarded in RTL mode.
    new_smusher!(sm, "tests/test.flf");
    sm.right2left = true;
    let mut wr = figdriver::Wrapper::new(sm, 4);
    [ "this", "   ", "is", " ", "a", " ", "test" ].iter().for_each(|x| wr.wrap_str(x, &dummy));
    // "this" (4) wraps, "is a" (4) wraps, "test" (4) → RTL rendered as "tset"
    assert_eq!(wr.get(), vec!["tset"]);
}

#[test]
fn inter_word_blanks_preserved() {
    // Multiple blanks between words should be preserved.
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 30);
    [ "a", "   ", "b" ].iter().for_each(|x| wr.wrap_str(x, &dummy));
    assert_eq!(wr.get(), vec!["a   b"]);
}

#[test]
fn blank_after_wrap_discarded() {
    // Whitespace immediately after a flush should be discarded.
    new_smusher!(sm, "tests/test.flf");
    let mut wr = figdriver::Wrapper::new(sm, 4);
    [ "this", "   ", "is", " ", "a", " ", "test" ].iter().for_each(|x| wr.wrap_str(x, &dummy));
    assert_eq!(wr.get(), vec!["test"]);
}

#[test]
fn smush_force_with_enable_uses_font_layout() {
    // -S on a font with SMUSH_ENABLE sets mode to font.layout
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + &path::MAIN_SEPARATOR.to_string() + "fonts/small.flf";
    let font = figdriver::FIGfont::from_path(path).unwrap();
    assert!((font.layout & figdriver::SMUSH_ENABLE) != 0);
    let mut sm = figdriver::Smusher::new(&font);
    sm.mode = font.layout;
    sm.full_width = false;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Smushy").is_err());
    let output = wr.get();
    assert_eq!(output, vec![r" ___              _        ",
                            r"/ __|_ __ _  _ __| |_ _  _ ",
                            r"\__ \ '  \ || (_-< ' \ || |",
                            r"|___/_|_|_\_,_/__/_||_\_, |",
                            r"                      |__/ "]);
}

#[test]
fn smush_force_without_enable_falls_back_to_overlap() {
    // -S on a font without SMUSH_ENABLE (banner, layout=64) falls back to overlap mode 0
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + &path::MAIN_SEPARATOR.to_string() + "fonts/banner.flf";
    let font = figdriver::FIGfont::from_path(path).unwrap();
    assert!((font.layout & figdriver::SMUSH_ENABLE) == 0);
    let mut sm = figdriver::Smusher::new(&font);
    sm.mode = 0;
    sm.full_width = false;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Hi").is_err());
    let output = wr.get();
    assert!(!output[0].is_empty());
    let smushed_width = output[0].chars().count();
    let mut sm_full = figdriver::Smusher::new(&font);
    sm_full.full_width = true;
    let mut wr_full = figdriver::Wrapper::new(sm_full, 60);
    assert!(!wr_full.push_str("Hi").is_err());
    let full_width = wr_full.get()[0].chars().count();
    assert!(smushed_width < full_width);
}

#[test]
fn smush_force_overrides_full_width() {
    // -S should disable full_width mode
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + &path::MAIN_SEPARATOR.to_string() + "fonts/small.flf";
    let font = figdriver::FIGfont::from_path(path).unwrap();
    let mut sm = figdriver::Smusher::new(&font);
    sm.full_width = false;
    sm.mode = font.layout;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Hi").is_err());
    let output = wr.get();
    let smushed_width = output[0].chars().count();
    let mut sm_full = figdriver::Smusher::new(&font);
    sm_full.full_width = true;
    let mut wr_full = figdriver::Wrapper::new(sm_full, 60);
    assert!(!wr_full.push_str("Hi").is_err());
    let full_width = wr_full.get()[0].chars().count();
    assert!(smushed_width < full_width);
}

#[test]
fn smush_force_overrides_kern() {
    // -S should take precedence over kern mode
    let path = env!("CARGO_MANIFEST_DIR").to_owned() + &path::MAIN_SEPARATOR.to_string() + "fonts/small.flf";
    let font = figdriver::FIGfont::from_path(path).unwrap();
    let mut sm = figdriver::Smusher::new(&font);
    sm.mode = font.layout;
    sm.full_width = false;
    let mut wr = figdriver::Wrapper::new(sm, 60);
    assert!(!wr.push_str("Smushy").is_err());
    let smushed = wr.get();
    let mut sm_kern = figdriver::Smusher::new(&font);
    sm_kern.mode = figdriver::SMUSH_KERN;
    let mut wr_kern = figdriver::Wrapper::new(sm_kern, 60);
    assert!(!wr_kern.push_str("Smushy").is_err());
    let kerned = wr_kern.get();
    let smushed_width = smushed[0].chars().count();
    let kerned_width = kerned[0].chars().count();
    assert!(smushed_width < kerned_width);
}
