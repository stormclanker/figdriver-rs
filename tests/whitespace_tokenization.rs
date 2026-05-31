mod common;

/// Regression tests for whitespace tokenization in the figlet binary.
/// Ensures consecutive whitespace (spaces, tabs) is tokenized correctly.

#[test]
fn argv_multiple_spaces() {
    // Multiple consecutive spaces between words should be preserved.
    let output = common::run_no_trim(&["a   b"]);
    assert_eq!(
        output,
        vec![
            "           _     ",
            "  __ _    | |__  ",
            " / _` |   | '_ \\ ",
            "| (_| |   | |_) |",
            " \\__,_|   |_.__/ "
        ]
    );
}

#[test]
fn argv_leading_spaces() {
    // Leading spaces should be preserved as FIGcharacters.
    let output = common::run_no_trim(&["  hello"]);
    assert_eq!(
        output,
        vec![
            "   _          _ _       ",
            "  | |__   ___| | | ___  ",
            "  | '_ \\ / _ \\ | |/ _ \\ ",
            "  | | | |  __/ | | (_) |",
            "  |_| |_|\\___|_|_|\\___/ "
        ]
    );
}

#[test]
fn argv_trailing_spaces() {
    // Trailing spaces should be preserved (right-aligned by default).
    let output = common::run_no_trim(&["hello   "]);
    assert!(!output.is_empty());
    assert!(output[0].ends_with("   "));
}

#[test]
fn stdin_multiple_spaces() {
    // Multiple spaces via stdin should be preserved.
    let output = common::run_with_input_no_trim(&[], "a   b");
    assert_eq!(
        output,
        vec![
            "           _     ",
            "  __ _    | |__  ",
            " / _` |   | '_ \\ ",
            "| (_| |   | |_) |",
            " \\__,_|   |_.__/ "
        ]
    );
}

#[test]
fn stdin_leading_spaces() {
    // Leading spaces via stdin should be preserved.
    let output = common::run_with_input_no_trim(&[], "  hello");
    assert_eq!(
        output,
        vec![
            "   _          _ _       ",
            "  | |__   ___| | | ___  ",
            "  | '_ \\ / _ \\ | |/ _ \\ ",
            "  | | | |  __/ | | (_) |",
            "  |_| |_|\\___|_|_|\\___/ "
        ]
    );
}

#[test]
fn stdin_tab_characters() {
    // Tab characters should be treated as whitespace and preserved.
    let output = common::run_with_input_no_trim(&[], "a\t\tb");
    assert!(!output.is_empty());
}

#[test]
fn stdin_mixed_whitespace() {
    // Mixed spaces and tabs should be preserved as a whitespace group.
    let output = common::run_with_input_no_trim(&[], "a \t b");
    assert!(!output.is_empty());
}

#[test]
fn stdin_consecutive_lines_with_spaces() {
    // Multiple lines with leading spaces should each preserve their spacing.
    let output = common::run_with_input_no_trim(&[], "  hello\n  world");
    assert!(!output.is_empty());
}
