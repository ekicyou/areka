//! [`super`]（`balloon_overrides`）の決定論テスト——トークン列の形と、飛ばした値の記録。
//!
//! 入力はすべて転記層の公開入口（`areka_parsers::balloon::parse_str`）を通す。記録の件数は
//! `log-capture-kit` で数え、捕捉の窓には本番の解決（`ResolvedFont::resolve`）1 回と、窓が生きている
//! ことの対照の `error!` 1 件だけを入れる（`capture_checked`・「記録 0 件」が窓の死で恒真にならない）
//! ——記録は「解決 1 回に 1 回」（受け口の判定は `look`・色の成分不足は本モジュールの宛先）。
//!
//! | テスト | 内容 | 要件 |
//! |---|---|---|
//! | W1 | 基底の飾りは宣言されたキーだけが列になる | 8.1 |
//! | W2 | 無効表示の書体名・大きさ・色・飾りの形 | 8.2 |
//! | W3 | 色の成分不足（`r` だけ・`r`＋`g`）は列に入らず記録 1 件 | 8.3, 8.5 |
//! | W4 | 語彙外の値は列に入ったうえで記録 1 件（キーは descript のキー名） | 8.5, 9.11 |
//! | W5 | 正典どおりの宣言は記録 0 件 | 8.5 |
//! | W6 | 影の 4 キーは両列とも渡さない | 8.3 |
//! | W7 | 何も書かなければ両列とも空 | 5.4 |
//! | W8 | 縁取りは列に入って記録なし | 8.1, 8.5 |
//! | W9 | 無効表示の大きさの受け口判定が記録に映る | 8.5, 9.11 |
//! | E1 | 端から端まで: 基底の太字が既定の見た目に立つ | 5.5, 8.1, 9.10 |
//! | E2 | 端から端まで: 無効表示の太字は無効表示の見た目だけに立つ | 8.2, 9.10 |
//! | E3 | 端から端まで: 無効表示の色 3 成分は混色でなく宣言値 | 8.2, 9.10 |
//! | E4 | 端から端まで: 縁取りは状態だけ立ち他の項目は既定のまま | 9.10 |

use areka_parsers::balloon::parse_str;
use log_capture_kit::CapturedEvent;

use super::{BalloonOverrides, overrides};
use crate::actor::test_support::capture_checked;
use crate::draw::ResolvedFont;
use crate::look::{LookLayers, apply_font_tag};

/// 本モジュールの記録の宛先。
const TARGET: &str = "areka_emo_text::balloon_overrides";
/// 受け口（`look.rs` の `apply_overrides`）が飛ばした値の記録の宛先。
const LOOK: &str = "areka_emo_text::look";

/// 解析 → 列（窓の外）と、本番の解決 1 回（対照つきの窓の中）。記録は「解決 1 回に 1 回」を数える。
fn run(descript: &str) -> (BalloonOverrides, Vec<CapturedEvent>) {
    let model = parse_str(descript, None);
    let (_, events) = capture_checked(|| ResolvedFont::resolve(&model));
    (overrides(&model), events)
}

/// 書体設定の記録の `warn` だけ（色の成分不足は本モジュール・受け口の判定は `look`）。
fn warns(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN && (e.target == TARGET || e.target == LOOK))
        .collect()
}

/// `&[&[&str]]` をトークン列へ。
fn lists(src: &[&[&str]]) -> Vec<Vec<String>> {
    src.iter()
        .map(|t| t.iter().map(|s| (*s).to_owned()).collect())
        .collect()
}

/// W1: 基底の飾りは宣言されたキーだけが `[キー, 値]` の列になり、無効表示へは漏れない。
#[test]
fn font_overrides_emit_one_token_list_per_declared_decoration_key() {
    let (out, _) = run("font.underline,1\r\nfont.bold,1\r\nfont.strike,0\r\n");
    assert_eq!(
        out.font,
        lists(&[&["bold", "1"], &["strike", "0"], &["underline", "1"]])
    );
    assert!(out.disable.is_empty(), "{:?}", out.disable);

    let (all, _) = run(
        "font.bold,1\r\nfont.italic,1\r\nfont.outline,1\r\nfont.strike,1\r\nfont.underline,1\r\n",
    );
    assert_eq!(
        all.font,
        lists(&[
            &["bold", "1"],
            &["italic", "1"],
            &["outline", "1"],
            &["strike", "1"],
            &["underline", "1"],
        ])
    );
}

/// W2: 無効表示の書体名は候補列（カンマ分割・trim・記述順）、大きさ・色・飾りは `\f` と同じ形。
#[test]
fn disable_overrides_emit_name_height_color_and_decorations() {
    let (out, _) = run("disable.font.name,A, B\r\ndisable.font.height,20\r\n\
         disable.font.color.r,1\r\ndisable.font.color.g,2\r\ndisable.font.color.b,3\r\n\
         disable.font.italic,1\r\n");
    assert_eq!(
        out.disable,
        lists(&[
            &["name", "A", "B"],
            &["height", "20"],
            &["color", "1", "2", "3"],
            &["italic", "1"],
        ])
    );
    assert!(out.font.is_empty(), "{:?}", out.font);
}

/// W3: 色の 3 成分が揃わなければ列に入れず、キー `disable.font.color` で 1 度だけ記録する。
#[test]
fn disable_color_with_missing_components_is_dropped_and_warned_once() {
    let (only_r, events) = run("disable.font.color.r,1\r\n");
    assert!(only_r.disable.is_empty(), "{:?}", only_r.disable);
    let w = warns(&events);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].target, TARGET);
    assert_eq!(w[0].field_str("key"), Some("disable.font.color"));
    assert_eq!(w[0].field_str("value"), Some("1,,"));

    let (out, events) = run("disable.font.color.r,1\r\ndisable.font.color.g,2\r\n");
    assert!(out.disable.is_empty(), "{:?}", out.disable);
    let w = warns(&events);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].field_str("key"), Some("disable.font.color"));
    assert_eq!(w[0].field_str("value"), Some("1,2,"));
}

/// W4: 語彙外の値は落とさず列に入れ、受け口が飛ばす理由で 1 度だけ記録する。
/// 記録のキーは descript のキー名（`font.bold`／`disable.font.bold`）で、基底と無効表示を取り違えない。
#[test]
fn out_of_vocabulary_values_are_passed_through_and_warned_once() {
    let (out, events) = run("font.bold,yes\r\n");
    assert_eq!(out.font, lists(&[&["bold", "yes"]]));

    // 理由の語は受け口自身に言わせる（語を写さない）。
    let base = LookLayers::default();
    let expected = apply_font_tag(&mut base.default.clone(), &base, &["bold", "yes"])
        .expect_err("受け口は `yes` を飛ばすはず")
        .reason;

    let w = warns(&events);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].target, LOOK);
    assert_eq!(w[0].field_str("key"), Some("font.bold"));
    assert_eq!(w[0].field_str("value"), Some("yes"));
    assert_eq!(w[0].field_str("reason"), Some(expected));

    // 基底と無効表示に同じ値を書くと記録は 2 件で、キーで書いた行を区別できる。
    let (_, events) = run("font.bold,yes\r\ndisable.font.bold,yes\r\n");
    let keys: Vec<_> = warns(&events).iter().map(|e| e.field_str("key")).collect();
    assert_eq!(keys, [Some("font.bold"), Some("disable.font.bold")]);
}

/// W5: 正典どおりの宣言（基底・無効表示の全形）は記録を 1 件も出さない。
///
/// 窓が生きていることは同じ窓の対照の `error!` で確かめる（`run`）。発行点が生きていることは W4／W9。
#[test]
fn canonical_declarations_produce_no_warnings() {
    let (out, events) = run(
        "font.bold,1\r\nfont.italic,0\r\nfont.strike,1\r\nfont.underline,default\r\n\
         disable.font.name,Meiryo, Yu Gothic UI\r\ndisable.font.height,14\r\n\
         disable.font.color.r,128\r\ndisable.font.color.g,128\r\ndisable.font.color.b,128\r\n\
         disable.font.bold,1\r\ndisable.font.underline,0\r\n",
    );
    assert_eq!(out.font.len(), 4, "{:?}", out.font);
    assert_eq!(out.disable.len(), 5, "{:?}", out.disable);
    assert_eq!(warns(&events).len(), 0, "{events:?}");
}

/// W6: 影の 4 キーは基底・無効表示とも列にしない（零の判定）。飾りは同じ入力から列になる（対照）。
#[test]
fn shadow_keys_are_never_forwarded() {
    let (out, _) = run(
        "font.bold,1\r\nfont.shadowcolor.r,1\r\nfont.shadowcolor.g,2\r\nfont.shadowcolor.b,3\r\n\
         font.shadowstyle,offset\r\n\
         disable.font.bold,1\r\ndisable.font.shadowcolor.r,1\r\ndisable.font.shadowcolor.g,2\r\n\
         disable.font.shadowcolor.b,3\r\ndisable.font.shadowstyle,outline\r\n",
    );
    assert_eq!(out.font, lists(&[&["bold", "1"]]));
    assert_eq!(out.disable, lists(&[&["bold", "1"]]));
}

/// W7: 何も書かなければ両列とも空（受け口の呼び出しは従来と同じ）。
#[test]
fn no_declarations_yield_two_empty_lists() {
    let (out, events) = run("");
    assert_eq!(out, BalloonOverrides::default());
    assert_eq!(warns(&events).len(), 0, "{events:?}");
}

/// W8: 縁取り（受け口では語彙のみ）は列に入り、記録しない。
#[test]
fn outline_is_forwarded_without_warning() {
    let (out, events) = run("font.outline,1\r\ndisable.font.outline,1\r\n");
    assert_eq!(out.font, lists(&[&["outline", "1"]]));
    assert_eq!(out.disable, lists(&[&["outline", "1"]]));
    assert_eq!(warns(&events).len(), 0, "{events:?}");
}

/// W9: 無効表示の大きさ `0` は列に入ったうえで受け口の「正でない」判定が記録 1 件に映り、
/// `20` は記録 0 件（spec `areka-P0-balloon-font-descript-keys` の research.md §14——相対指定は転記層が `u32` で読むため配線へ届かない）。
#[test]
fn disable_height_rejected_by_receiver_is_forwarded_and_warned_once() {
    let (zero, events) = run("disable.font.height,0\r\n");
    assert_eq!(zero.disable, lists(&[&["height", "0"]]));
    let w = warns(&events);
    assert_eq!(w.len(), 1, "{w:?}");
    assert_eq!(w[0].target, LOOK);
    assert_eq!(w[0].field_str("key"), Some("disable.font.height"));
    assert_eq!(w[0].field_str("value"), Some("0"));

    let (twenty, events) = run("disable.font.height,20\r\n");
    assert_eq!(twenty.disable, lists(&[&["height", "20"]]));
    assert_eq!(warns(&events).len(), 0, "{events:?}");
}

/// 解析 → 本番の解決関数（端から端まで）。
fn looks_of(descript: &str) -> LookLayers {
    ResolvedFont::resolve(&parse_str(descript, None)).looks
}

/// E1: `font.bold,1` は既定の見た目の太字を立て、無効表示（既定の複製）にも写る。
/// 配線を空の列へ戻すと赤になる。
#[test]
fn resolve_lifts_declared_bold_into_the_default_look() {
    let looks = looks_of("font.bold,1\r\n");
    assert!(looks.default.bold, "{:?}", looks.default);
    assert!(looks.disable.bold, "{:?}", looks.disable);
}

/// E2: `disable.font.bold,1` は無効表示の見た目だけに立ち、既定の見た目は太字にならない。
#[test]
fn resolve_lifts_disable_bold_into_the_disable_look_only() {
    let looks = looks_of("disable.font.bold,1\r\n");
    assert!(looks.disable.bold, "{:?}", looks.disable);
    assert!(!looks.default.bold, "{:?}", looks.default);
}

/// E3: 無効表示の色は 3 成分が揃えば宣言値（混色ではない）、揃わなければ宣言なしと同じ混色のまま。
#[test]
fn resolve_uses_declared_disable_color_instead_of_the_mix() {
    let mixed = looks_of("").disable.color;
    assert_ne!(mixed, (10, 20, 30), "対照が成り立たない入力");

    let declared = looks_of(
        "disable.font.color.r,10\r\ndisable.font.color.g,20\r\ndisable.font.color.b,30\r\n",
    );
    assert_eq!(declared.disable.color, (10, 20, 30));

    let partial = looks_of("disable.font.color.r,10\r\n");
    assert_eq!(partial.disable.color, mixed);
}

/// E4: `font.outline,1` は `outline` だけを立て、`TextLook` の他の項目は宣言なしと同じ（語彙のみ）。
#[test]
fn resolve_keeps_outline_vocabulary_only() {
    let plain = looks_of("");
    let looks = looks_of("font.outline,1\r\n");
    assert!(looks.default.outline, "{:?}", looks.default);

    let mut expected = plain.default.clone();
    expected.outline = true;
    assert_eq!(looks.default, expected);
}
