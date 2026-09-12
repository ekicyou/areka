//! `Charset`／`LabelError` の決定論テスト（タスク 1.1・要件 1.1/1.2/1.4/1.5/3.2/3.4/7.3/10.1〜10.3）。
//!
//! 窓も I/O も持たない純関数の入出力だけで成立する（x64 のみ・32bit 成果物不要）。
//! 参照値のバイト列は符号化器から導かず定数として持つ（design.md §Supporting References）。

use std::borrow::Cow;

use super::{Charset, LabelError};

/// 別名 5 綴りが同じ `Charset` へ解決し、正規名が `Shift_JIS` で一致する（要件 1.2）。
#[test]
fn shift_jis_aliases_resolve_to_the_same_charset() {
    let labels = [
        "shift_jis",
        "Shift-JIS",
        "sjis",
        "windows-31j",
        " SHIFT_JIS ",
    ];

    for label in labels {
        let cs = Charset::for_label(label).expect("Shift_JIS の別名は解決できる");
        assert_eq!(
            cs,
            Charset::SHIFT_JIS,
            "別名 {label:?} が同じ値へ解決しない"
        );
        assert_eq!(
            cs.name(),
            "Shift_JIS",
            "別名 {label:?} の正規名が一致しない"
        );
    }
}

/// 既定の 2 つ以外の文字コードも同じ経路で解決でき、実際にその文字コードで符号化される
/// （要件 1.4「既定は対応範囲ではない」＝Shift_JIS／UTF-8 だけを特別扱いしない）。
#[test]
fn charsets_beyond_the_two_constants_resolve_and_encode() {
    // 「あ」の参照値（design.md §Supporting References の定数）。
    let euc_jp = Charset::for_label("euc-jp").expect("EUC-JP は解決できる");
    assert_eq!(euc_jp.name(), "EUC-JP");
    assert_eq!(euc_jp.encode("あ").0.as_ref(), &[0xA4, 0xA2]);

    let iso_2022_jp = Charset::for_label("ISO-2022-JP").expect("ISO-2022-JP は解決できる");
    assert_eq!(iso_2022_jp.name(), "ISO-2022-JP");
    assert_eq!(
        iso_2022_jp.encode("あ").0.as_ref(),
        &[0x1B, 0x24, 0x42, 0x24, 0x22, 0x1B, 0x28, 0x42]
    );
}

/// 符号化の出力が要求と一致しないラベル（UTF-16 系・replacement 系）は
/// `NotEncodable`、Encoding Standard に無いラベルは `Unknown` で、理由が区別できる
/// （要件 1.5・10.3）。
#[test]
fn unusable_labels_are_rejected_with_distinguishable_reasons() {
    for label in ["UTF-16", "UTF-16LE", "replacement", "hz-gb-2312"] {
        assert_eq!(
            Charset::for_label(label),
            Err(LabelError::NotEncodable),
            "{label:?} は通信で使えない既知の限界として退けられるべき"
        );
    }

    assert_eq!(Charset::for_label("x-nope"), Err(LabelError::Unknown));
}

/// UTF-8 の符号化は入力の UTF-8 バイト列と同一で、複製も置換も起きない（要件 3.4）。
#[test]
fn utf8_encode_is_byte_identical_to_the_input() {
    let text = "ID: OnBoot\r\nReference0: あ\r\n";
    let (bytes, replaced) = Charset::UTF_8.encode(text);

    assert_eq!(bytes.as_ref(), text.as_bytes());
    assert_eq!(replaced, 0);
    assert!(
        matches!(bytes, Cow::Borrowed(_)),
        "UTF-8 では借用のまま返る（複製 0）"
    );
}

/// 表せない文字は 10 進の数値文字参照へ置換され、置換した文字数が返る（要件 10.1・3.2）。
#[test]
fn unmappable_char_becomes_a_decimal_numeric_character_reference() {
    let (bytes, replaced) = Charset::SHIFT_JIS.encode("\u{1F600}");

    assert_eq!(bytes.as_ref(), b"&#128512;");
    assert_eq!(replaced, 1);
}

/// 宣言された文字コードとして不正な並びは代替文字へ吸収され、置換の有無が返る（要件 10.2）。
#[test]
fn invalid_byte_sequence_decodes_to_the_replacement_char() {
    // 0x82 は Shift_JIS の 2 バイト文字の先行バイト。単独で終わるので不正な並び。
    let (text, had_errors) = Charset::SHIFT_JIS.decode(&[0x82]);

    assert!(
        text.contains('\u{FFFD}'),
        "代替文字へ吸収されていない: {text:?}"
    );
    assert!(had_errors);

    // 妥当な並びでは置換が起きない（恒真な緑にしない較正）。
    let (text, had_errors) = Charset::SHIFT_JIS.decode(&[0x82, 0xA0]);
    assert_eq!(text, "あ");
    assert!(!had_errors);
}

/// 復号は宣言された文字コードだけで行い、先頭の BOM で文字コードを切り替えない
/// （BOM は 1 文字として残る・要件 10.2 の経路と同じ寛容さ）。
#[test]
fn decode_does_not_let_a_bom_override_the_declared_charset() {
    let (text, had_errors) = Charset::UTF_8.decode(b"\xEF\xBB\xBFA");
    assert_eq!(text, "\u{FEFF}A");
    assert!(!had_errors);

    // UTF-8 の BOM を Shift_JIS として宣言された応答に混ぜても UTF-8 へ切り替わらない。
    let (text, _) = Charset::SHIFT_JIS.decode(b"\xEF\xBB\xBF\x82\xA0");
    assert!(text.ends_with('あ'));
    assert_ne!(text, "\u{FEFF}あ", "UTF-8 として読み直してはならない");
}

/// 正規名は `Charset` ヘッダに書く綴りで、定数と解決結果で一致する（要件 3.2）。
#[test]
fn canonical_names_match_the_header_spelling() {
    assert_eq!(Charset::UTF_8.name(), "UTF-8");
    assert_eq!(Charset::SHIFT_JIS.name(), "Shift_JIS");
    assert_eq!(
        Charset::for_label("utf8").map(Charset::name),
        Ok("UTF-8"),
        "別名からも同じ綴りが取れる"
    );
}

// ---------------------------------------------------------------------------
// `CharsetNegotiator`／`CharsetPolicy` の決定論テスト
// （タスク 1.2・要件 2.2/2.3/4.2/4.4〜4.7/5.1/7.1/7.2/7.4/9.3/10.3）。
//
// ログは `log_capture_kit::capture` で観測する（呼出スレッドで同期発火するため捕捉できる。
// 捕捉窓は番兵イベントで自己較正されるので、件数 0 の主張が「捕捉できていないだけ」には
// ならない）。
// ---------------------------------------------------------------------------

use log_capture_kit::CapturedEvent;

use super::{CharsetNegotiator, CharsetPolicy};

/// 交渉のログ（`target: "shiori-charset"`）だけを取り出す。
fn charset_events(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == "shiori-charset")
        .collect()
}

/// 指定した `event` 名のログだけを取り出す。
fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    charset_events(events)
        .into_iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// 規則表 1 行目: 非強制・ヘッダ省略 → 継承のみ（状態不変・ログ 0・要件 4.3）。
#[test]
fn negotiate_without_header_inherits_and_logs_nothing() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    let ((), events) = log_capture_kit::capture(|| neg.note_response(None, false));

    assert_eq!(neg.current(), Charset::SHIFT_JIS);
    assert_eq!(charset_events(&events).len(), 0, "{events:?}");
}

/// 規則表 2 行目: 非強制・同じ文字コードのヘッダ → 状態不変・ログ 0（要件 4.6「変わらない
/// 応答ではログを出さない」）。別名の綴りで来ても「同じ」と判定する。
#[test]
fn negotiate_with_the_same_charset_changes_nothing_and_logs_nothing() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("windows-31j"), false));

    assert_eq!(neg.current(), Charset::SHIFT_JIS);
    assert_eq!(charset_events(&events).len(), 0, "{events:?}");
}

/// 規則表 3 行目: 非強制・解決できる別の文字コード → 採用し、切替の詳細ログ 1 行
/// （要件 4.2・4.6・9.3 前半「採用後の `current()` が変わる」）。
#[test]
fn negotiate_with_a_different_charset_switches_and_logs_once() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("UTF-8"), false));

    assert_eq!(neg.current(), Charset::UTF_8, "採用が状態に現れていない");
    assert_eq!(neg.policy(), CharsetPolicy::Negotiate(Charset::UTF_8));

    let switched = named(&events, "charset_switched");
    assert_eq!(switched.len(), 1, "{events:?}");
    assert_eq!(switched[0].level, tracing::Level::DEBUG);
    assert_eq!(switched[0].field_str("from"), Some("Shift_JIS"));
    assert_eq!(switched[0].field_str("to"), Some("UTF-8"));
    assert_eq!(
        charset_events(&events).len(),
        1,
        "余計なログがある: {events:?}"
    );
}

/// 規則表 4 行目: 非強制・解決できないヘッダ → 採用せず現在の文字コードを継続し、
/// 初回は警告 1 行（ラベル・理由・継続する文字コード・要件 4.4・10.3）。
#[test]
fn negotiate_with_an_unresolvable_label_keeps_current_and_warns() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("x-nope"), false));

    assert_eq!(neg.current(), Charset::SHIFT_JIS, "採用してはならない");

    let unresolved = named(&events, "charset_label_unresolved");
    assert_eq!(unresolved.len(), 1, "{events:?}");
    assert_eq!(unresolved[0].level, tracing::Level::WARN);
    assert_eq!(unresolved[0].field_str("label"), Some("x-nope"));
    assert_eq!(unresolved[0].field_str("reason"), Some("unknown"));
    assert_eq!(unresolved[0].field_str("kept"), Some("Shift_JIS"));

    // UTF-16 系は「通信で使えない既知の限界」として理由で区別される（要件 10.3）。
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);
    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("UTF-16"), false));
    let unresolved = named(&events, "charset_label_unresolved");
    assert_eq!(unresolved.len(), 1, "{events:?}");
    assert_eq!(unresolved[0].field_str("reason"), Some("not_encodable"));
    assert_eq!(neg.current(), Charset::SHIFT_JIS);
}

/// 規則表 5 行目: 強制・ヘッダ省略 → 状態不変・ログ 0（要件 2.2）。
#[test]
fn forced_without_header_changes_nothing_and_logs_nothing() {
    let mut neg = CharsetNegotiator::new(Charset::UTF_8, true);

    let ((), events) = log_capture_kit::capture(|| neg.note_response(None, false));

    assert_eq!(neg.current(), Charset::UTF_8);
    assert_eq!(neg.policy(), CharsetPolicy::Force(Charset::UTF_8));
    assert_eq!(charset_events(&events).len(), 0, "{events:?}");
}

/// 規則表 6 行目: 強制・同じ文字コードのヘッダ → 状態不変・ログ 0（食い違っていないので
/// 知らせることが無い）。
#[test]
fn forced_with_the_same_charset_changes_nothing_and_logs_nothing() {
    let mut neg = CharsetNegotiator::new(Charset::UTF_8, true);

    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("utf8"), false));

    assert_eq!(neg.current(), Charset::UTF_8);
    assert_eq!(charset_events(&events).len(), 0, "{events:?}");
}

/// 規則表 7 行目: 強制・それ以外のヘッダ（別の文字コード／解決できないラベルの双方）→
/// 復号にも採用にも用いず、初回のみ詳細ログ 1 行（要件 2.2・4.5）。
#[test]
fn forced_ignores_a_mismatching_header_and_logs_only_the_first_time() {
    let mut neg = CharsetNegotiator::new(Charset::UTF_8, true);

    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("Shift_JIS"), false));

    assert_eq!(neg.current(), Charset::UTF_8, "強制中に採用してはならない");
    let ignored = named(&events, "charset_forced_ignores_header");
    assert_eq!(ignored.len(), 1, "{events:?}");
    assert_eq!(ignored[0].level, tracing::Level::DEBUG);
    assert_eq!(ignored[0].field_str("forced"), Some("UTF-8"));
    assert_eq!(ignored[0].field_str("header"), Some("Shift_JIS"));
    assert_eq!(charset_events(&events).len(), 1, "{events:?}");

    // 2 回目以降は綴りが変わっても（解決できないラベルでも）ログを出さない＝「初回のみ」。
    let ((), events) = log_capture_kit::capture(|| {
        neg.note_response(Some("EUC-JP"), false);
        neg.note_response(Some("x-nope"), false);
    });
    assert_eq!(neg.current(), Charset::UTF_8);
    assert_eq!(charset_events(&events).len(), 0, "{events:?}");
}

/// 同じ解決できないラベルを 3 回受けると警告 1・詳細 2（要件 4.4・7.4——毎秒のイベントで
/// 警告が氾濫しない）。
#[test]
fn the_same_unresolvable_label_warns_once_then_falls_to_debug() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    let ((), events) = log_capture_kit::capture(|| {
        for _ in 0..3 {
            neg.note_response(Some("x-nope"), false);
        }
    });

    let unresolved = named(&events, "charset_label_unresolved");
    assert_eq!(
        unresolved.len(),
        3,
        "後退のたびに記録は残る（要件 7.1）: {events:?}"
    );
    let warns = unresolved
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .count();
    let debugs = unresolved
        .iter()
        .filter(|e| e.level == tracing::Level::DEBUG)
        .count();
    assert_eq!((warns, debugs), (1, 2), "{events:?}");
}

/// 大小文字・前後空白だけが違う同じラベルは同じ 1 件と数える（鍵の正規化。壊れた SHIORI が
/// 応答ごとに綴りを変えても警告が重複せず、警告済みの集合も際限なく育たない）。
#[test]
fn label_keys_are_normalized_so_case_variants_warn_only_once() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    let ((), events) = log_capture_kit::capture(|| {
        neg.note_response(Some("foo"), false);
        neg.note_response(Some("FOO"), false);
        neg.note_response(Some("  Foo  "), false);
    });

    let warns = named(&events, "charset_label_unresolved")
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .count();
    assert_eq!(warns, 1, "綴り違いで警告が重複している: {events:?}");

    // 別のラベルは別の 1 件として警告される（恒真な緑にしない較正）。
    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("bar"), false));
    let warns = named(&events, "charset_label_unresolved")
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .count();
    assert_eq!(warns, 1, "{events:?}");
}

/// 応答の不正な並びは採用規則と独立に記録され、文字コード名につき初回 warn・以後 debug
/// （要件 4.7・10.2・7.4）。記録される文字コード名は実際に復号に使った文字コード
/// ＝採用後の現在値。
#[test]
fn invalid_bytes_are_recorded_independently_of_adoption() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    // 採用（Shift_JIS → UTF-8）と復号エラーが同じ応答で起きる。
    let ((), events) = log_capture_kit::capture(|| neg.note_response(Some("UTF-8"), true));

    assert_eq!(neg.current(), Charset::UTF_8);
    assert_eq!(named(&events, "charset_switched").len(), 1, "{events:?}");
    let invalid = named(&events, "charset_invalid_bytes_replaced");
    assert_eq!(invalid.len(), 1, "{events:?}");
    assert_eq!(invalid[0].level, tracing::Level::WARN);
    assert_eq!(
        invalid[0].field_str("charset"),
        Some("UTF-8"),
        "復号に使った文字コードで記録する"
    );

    // 同じ文字コードの 2 回目以降は詳細ログへ落ちる。
    let ((), events) = log_capture_kit::capture(|| neg.note_response(None, true));
    let invalid = named(&events, "charset_invalid_bytes_replaced");
    assert_eq!(invalid.len(), 1, "{events:?}");
    assert_eq!(invalid[0].level, tracing::Level::DEBUG);

    // 置換が起きていない応答では記録しない。
    let ((), events) = log_capture_kit::capture(|| neg.note_response(None, false));
    assert_eq!(charset_events(&events).len(), 0, "{events:?}");
}

/// 要求で表せない文字を置換したことはイベント名につき初回 warn・以後 debug
/// （要件 3.5・7.2・7.4）。置換 0 のときは記録しない。
#[test]
fn unmappable_replacements_are_recorded_per_event_id() {
    let mut neg = CharsetNegotiator::new(Charset::SHIFT_JIS, false);

    let ((), events) = log_capture_kit::capture(|| neg.note_request("OnSecondChange", 2));

    let replaced = named(&events, "charset_unmappable_replaced");
    assert_eq!(replaced.len(), 1, "{events:?}");
    assert_eq!(replaced[0].level, tracing::Level::WARN);
    assert_eq!(replaced[0].field_str("id"), Some("OnSecondChange"));
    assert_eq!(replaced[0].field("replaced"), Some("2"));

    // 同じイベントの 2 回目は詳細ログ、別のイベントは改めて警告。
    let ((), events) = log_capture_kit::capture(|| {
        neg.note_request("OnSecondChange", 1);
        neg.note_request("OnBoot", 1);
    });
    let replaced = named(&events, "charset_unmappable_replaced");
    assert_eq!(replaced.len(), 2, "{events:?}");
    assert_eq!(replaced[0].level, tracing::Level::DEBUG);
    assert_eq!(replaced[1].level, tracing::Level::WARN);

    // 置換 0 の通常経路ではログを出さない。
    let ((), events) = log_capture_kit::capture(|| neg.note_request("OnSecondChange", 0));
    assert_eq!(charset_events(&events).len(), 0, "{events:?}");
}

/// `policy()` は強制の有無をそのまま 2 値へ写す（要件 2.2・2.3）。
#[test]
fn policy_reflects_the_forced_flag() {
    let negotiated = CharsetNegotiator::new(Charset::SHIFT_JIS, false);
    assert_eq!(
        negotiated.policy(),
        CharsetPolicy::Negotiate(Charset::SHIFT_JIS)
    );

    let forced = CharsetNegotiator::new(Charset::UTF_8, true);
    assert_eq!(forced.policy(), CharsetPolicy::Force(Charset::UTF_8));
}

/// 交渉状態は起動時のクロージャ（`Send + 'static`）へ move されるため `Send` を満たす
/// （design §State Management → Concurrency）。
#[test]
fn negotiator_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<CharsetNegotiator>();
}
