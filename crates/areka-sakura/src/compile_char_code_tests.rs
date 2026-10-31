//! 文字の番号の埋め込み `\_u[…]`・`\_m[…]` の組み立ての檻（spec: script-small-tags 要件 1.5〜1.8・2.1〜2.4・2.6・2.7）。
//!
//! 1. 埋め込みの台本を組み立てた合図の列は、同じ字を直に書いた台本の合図の列と等しい。
//! 2. 捨てた番号は理由ごとに警告を 1 行だけ残し、合図を作らず、後ろの台本は今どおり組み上がる。

use super::test_support::{command_of, compile, cue_eq};
use super::*;
use areka_parsers::sakura::parse;

fn same_cues(a: &CompiledTalk, b: &CompiledTalk) -> bool {
    let (a, b) = (a.sheet.cues(), b.sheet.cues());
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| cue_eq(x, y)) && !a.is_empty()
}

/// 埋め込みと直書きの合図の列が等しい（1.5〜1.8）。全角の空白だけの台本と、ぱすたの起動の形
/// （`\_u[0x3000]` → 重なりの指令 → `\c`）を含む。
#[test]
fn embedded_script_compiles_like_the_direct_one() {
    let pairs = [
        ("\\_u[0x3000]", "　"),
        (
            "\\0\\_u[0x3000]\\![set,zorder,0,1]\\c",
            "\\0　\\![set,zorder,0,1]\\c",
        ),
        ("本文\\_u[0x3000]本文", "本文　本文"),
        ("\\f[bold,1]\\_m[65]\\_m[0x3042]", "\\f[bold,1]Aあ"),
        ("e\\_u[0x0301]", "e\u{301}"),
        ("\\_u[0xD83D]\\_u[0xDE00]", "\u{1F600}"),
        ("\\_m[0x5C]\\_m[0x25]username", "\\\\\\%username"),
        ("\\1\\_u[8942]\\w9\\_u[0x3000]", "\\1\u{22EE}\\w9　"),
    ];
    for (embedded, direct) in pairs {
        let (a, b) = (compile(&parse(embedded)), compile(&parse(direct)));
        assert!(same_cues(&a, &b), "{embedded} と {direct}");
        assert_eq!(a.end, b.end, "{embedded}");
    }
    // 全角の空白だけの台本は本文の合図を 1 つ持つ（バルーンを開く・1.6）。
    let compiled = compile(&parse("\\_u[0x3000]"));
    assert_eq!(
        command_of(&compiled.sheet.cues()[1]),
        &CueCommand::Text("　".into())
    );
}

/// 捨てた番号は理由ごとに `char_code_dropped` の警告が 1 行（元の綴り・理由つき）、合図は 0、
/// 後ろの台本は捨てた番号が無いときと同じに組み上がる（2.1〜2.4・2.6・2.7）。
#[test]
fn dropped_char_code_warns_once_and_emits_nothing() {
    let cases = [
        ("\\_u[0]", "Zero"),
        ("\\_m[0x00]", "Zero"),
        ("\\_u[0x09]", "Control"),
        ("\\_m[0x7F]", "Control"),
        ("\\_u[0xD83D]", "LoneSurrogate"),
        ("\\_u[0xDE00]", "LoneSurrogate"),
        ("\\_u[0x110000]", "OutOfRange"),
        ("\\_u[abc]", "Unreadable"),
        ("\\_u[]", "Unreadable"),
        ("\\_m[-1]", "Unreadable"),
    ];
    for (tag, reason) in cases {
        let (alone, events) = log_capture_kit::capture(|| compile(&parse(tag)));
        let warned: Vec<_> = events
            .iter()
            .filter(|ev| ev.field_str("event") == Some("char_code_dropped"))
            .collect();
        assert_eq!(warned.len(), 1, "{tag}: {events:?}");
        assert_eq!(warned[0].level, tracing::Level::WARN, "{tag}");
        // 字句の段は `\_u[]` の引数を空の列で渡す（`\s[]` と同じ）ので、元の綴りは `\_u` に戻る。
        let raw = if tag == "\\_u[]" { "\\_u" } else { tag };
        assert_eq!(warned[0].field("raw"), Some(raw), "{tag}");
        assert_eq!(warned[0].field("reason"), Some(reason), "{tag}");
        assert!(alone.sheet.is_empty(), "{tag} は合図を作らない");

        let after = compile(&parse(&format!("{tag}い\\n\\s[1]")));
        assert!(
            same_cues(&after, &compile(&parse("い\\n\\s[1]"))),
            "{tag} の後ろ"
        );
    }
}
