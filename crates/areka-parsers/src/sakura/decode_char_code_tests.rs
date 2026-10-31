//! `\_u[…]`・`\_m[…]`（文字の番号の埋め込み）の読み手の兄弟テスト（script-small-tags
//! 要件 1.1〜1.3・1.5・1.7〜1.9・2.3・2.5・8.1）。
//!
//! 固定すること:
//! - 出せる番号は本文の字（`Text`）になり、隣の本文と 1 つの `Text` にまとまる。
//! - 埋めた字は読み直さない（`\_m[0x5C]` は `\` の 1 字）。
//! - 上位と下位のサロゲートの組は 1 字（範囲は 2 つを覆う）、片割れは捨てる。
//! - 捨てた番号は `CharCodeDropped` で、隣の本文とまとめない。

use super::{CharCodeDrop, Instruction, SurfaceArg, parse, parse_noted};

fn text(s: &str) -> Instruction {
    Instruction::Text(s.to_owned())
}

fn dropped(raw: &str, reason: CharCodeDrop) -> Instruction {
    Instruction::CharCodeDropped {
        raw: raw.to_owned(),
        reason,
    }
}

#[test]
fn printable_codes_become_text() {
    for (input, want) in [
        (r"\_u[0x3000]", "\u{3000}"),
        (r"\_u[8942]", "\u{22EE}"),
        (r"\_u[0x1F600]", "\u{1F600}"),
        (r"\_m[65]", "A"),
        (r"\_m[0x41]", "A"),
        (r"\_m[0x3042]", "あ"),
        (r"\_u[ 0x22EE]", "\u{22EE}"),
        (r"\_u[0x22EE,1]", "\u{22EE}"),
    ] {
        assert_eq!(parse(input), vec![text(want)], "{input}");
    }
}

#[test]
fn embedded_chars_are_not_read_again() {
    assert_eq!(parse(r"\_m[0x5C]"), vec![text("\\")]);
    assert_eq!(parse(r"\_m[0x25]"), vec![text("%")]);
    assert_eq!(parse(r"\_m[0x5C]n"), vec![text("\\n")]);
}

#[test]
fn embedding_joins_neighbouring_text_into_one() {
    let input = r"本文\_u[0x3000]本文";
    let reads = parse_noted(input);
    assert_eq!(reads.len(), 1, "{reads:?}");
    assert_eq!(reads[0].instruction, text("本文\u{3000}本文"));
    assert_eq!(reads[0].span, 0..input.len());
    assert!(reads[0].notes.is_empty());

    assert_eq!(parse(r"e\_u[0x0301]"), vec![text("e\u{301}")]);
    assert_eq!(parse(r"\_m[65]\_m[66]"), vec![text("AB")]);
}

#[test]
fn surrogate_pair_becomes_one_char_covering_both() {
    for input in [
        r"\_u[0xD83D]\_u[0xDE00]",
        r"\_m[0xD83D]\_u[0xDE00]",
        r"\_u[55357]\_m[0xde00]",
    ] {
        let reads = parse_noted(input);
        assert_eq!(reads.len(), 1, "{input}: {reads:?}");
        assert_eq!(reads[0].instruction, text("\u{1F600}"), "{input}");
        assert_eq!(reads[0].span, 0..input.len(), "{input}");
    }
}

#[test]
fn lone_surrogates_are_dropped() {
    use CharCodeDrop::LoneSurrogate;
    assert_eq!(
        parse(r"\_u[0xD83D]"),
        vec![dropped(r"\_u[0xD83D]", LoneSurrogate)]
    );
    assert_eq!(
        parse(r"\_m[0xDE00]"),
        vec![dropped(r"\_m[0xDE00]", LoneSurrogate)]
    );
    // 上位の後が本文・下位でない埋め込み・逆の順は組にならない。
    assert_eq!(
        parse(r"\_u[0xD83D]a"),
        vec![dropped(r"\_u[0xD83D]", LoneSurrogate), text("a")]
    );
    assert_eq!(
        parse(r"\_u[0xDE00]\_u[0xD83D]"),
        vec![
            dropped(r"\_u[0xDE00]", LoneSurrogate),
            dropped(r"\_u[0xD83D]", LoneSurrogate),
        ]
    );
    // 上位が 2 つ続けば先の 1 つだけが片割れで、後ろは下位と組になる。
    assert_eq!(
        parse(r"\_u[0xD83D]\_u[0xD83D]\_u[0xDE00]"),
        vec![dropped(r"\_u[0xD83D]", LoneSurrogate), text("\u{1F600}")]
    );
    // 上位の後に捨てる番号が来ても、その番号は自分の理由で捨てる。
    assert_eq!(
        parse(r"\_u[0xD83D]\_u[0]"),
        vec![
            dropped(r"\_u[0xD83D]", LoneSurrogate),
            dropped(r"\_u[0]", CharCodeDrop::Zero),
        ]
    );
}

#[test]
fn unprintable_and_unreadable_codes_are_dropped_with_reason() {
    use CharCodeDrop::*;
    for (input, reason) in [
        (r"\_u[0]", Zero),
        (r"\_m[0x00]", Zero),
        (r"\_u[0x09]", Control),
        (r"\_m[0x7F]", Control),
        (r"\_u[0x110000]", OutOfRange),
        (r"\_m[0xFFFFFFFFFF]", OutOfRange),
        (r"\_u[abc]", Unreadable),
        (r"\_m[0x]", Unreadable),
        (r"\_u[-1]", Unreadable),
    ] {
        assert_eq!(parse(input), vec![dropped(input, reason)], "{input}");
    }
}

#[test]
fn dropped_codes_are_not_joined_and_keep_their_span() {
    let input = r"a\_u[0]b";
    let reads = parse_noted(input);
    let got: Vec<_> = reads.iter().map(|r| r.instruction.clone()).collect();
    assert_eq!(
        got,
        vec![text("a"), dropped(r"\_u[0]", CharCodeDrop::Zero), text("b")]
    );
    assert_eq!(&input[reads[1].span.clone()], r"\_u[0]");
}

#[test]
fn many_embeddings_keep_written_order() {
    assert_eq!(
        parse(r"\_u[0x41]\s[0]\_m[0x42]\_u[0]\_u[0x43]末"),
        vec![
            text("A"),
            Instruction::Surface(SurfaceArg::new("0".to_owned())),
            text("B"),
            dropped(r"\_u[0]", CharCodeDrop::Zero),
            text("C末"),
        ]
    );
}
