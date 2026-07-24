//! Task 2.5 — 語彙台帳・key パーサ 決定論単体テスト群（consolidated cage）。
//!
//! design.md「Testing Strategy → Unit Tests」項 1（key パーサ）・項 2（語彙台帳）が
//! 列挙する検証基準を、**単一の自己完結モジュール**として第一級化する。個々の不変量は
//! 各生産ファイルの `#[cfg(test)] mod tests`（key.rs／vocab/flat.rs／vocab/dotted.rs／
//! vocab/shiori_resource.rs／vocab/mod.rs）でも檻化済みだが、Task 2.5 の成果物は「この
//! 決定論単体テスト群が criterion ごとに自明な 1 群として存在すること」ゆえ、基準ごとに
//! 1 テストへ写して集約する（criterion → assertion の写像を doc で明示）。
//!
//! すべて x64 純粋・no I/O・no std::env・no clock（本モジュールは const 表と
//! [`parse_dotted`] の純関数のみを参照する）。R9.1（全判断分岐の決定論檻）・R9.2（x64 純粋）。

#![cfg(test)]

use crate::key::{parse_dotted, KeyParseError, PropPath, PathSeg, Selector};
use crate::vocab::dotted::{
    DOTTED_ROOTS, EXT_EVENT_GET, EXT_EVENT_SET, GENERIC_PROP_NAMES, SET_EFFECTIVE,
};
use crate::vocab::flat::{FLAT_VOCAB, SYNTAX_RECORDS};
use crate::vocab::shiori_resource::SHIORI_RESOURCE_IDS;
use crate::vocab::{BackingLayer, DegradePolicy, SetSemantics};

// ============================================================================
// 基準群 A: セレクタ 5 形の受理と PropPath 構造の一致（R1.3・design UnitTests 1）
//   各形が「ちょうどこの構造」へ解釈されることを 1 テストへ集約。
//   （個別檻: key.rs form1..form5_*）
// ============================================================================

#[test]
fn criterion_selector_five_forms_exact_proppath() {
    // ①括弧名選択: ghostlist(名前) → name="ghostlist", ByName("名前")
    assert_eq!(
        parse_dotted("ghostlist(名前)").unwrap(),
        PropPath {
            segs: vec![PathSeg {
                name: "ghostlist".into(),
                selector: Some(Selector::ByName("名前".into())),
            }]
        },
        "①括弧名選択"
    );

    // ②.index(ID) → 末尾セグメント name="index", ByIndex(3)
    assert_eq!(
        parse_dotted("activeghostlist(x).index(3)").unwrap(),
        PropPath {
            segs: vec![
                PathSeg { name: "activeghostlist".into(), selector: Some(Selector::ByName("x".into())) },
                PathSeg { name: "index".into(), selector: Some(Selector::ByIndex(3)) },
            ]
        },
        "②.index(ID)"
    );

    // ③.current → name="current", selector=None
    assert_eq!(
        parse_dotted("currentghost.current").unwrap(),
        PropPath {
            segs: vec![
                PathSeg { name: "currentghost".into(), selector: None },
                PathSeg { name: "current".into(), selector: None },
            ]
        },
        "③.current"
    );

    // ④.count → name="count", selector=None
    assert_eq!(
        parse_dotted("ghostlist.count").unwrap(),
        PropPath {
            segs: vec![
                PathSeg { name: "ghostlist".into(), selector: None },
                PathSeg { name: "count".into(), selector: None },
            ]
        },
        "④.count"
    );

    // ⑤数値括弧: scope(0) → name="scope", ByIndex(0)
    assert_eq!(
        parse_dotted("areka.window.scope(0).x").unwrap(),
        PropPath {
            segs: vec![
                PathSeg { name: "areka".into(), selector: None },
                PathSeg { name: "window".into(), selector: None },
                PathSeg { name: "scope".into(), selector: Some(Selector::ByIndex(0)) },
                PathSeg { name: "x".into(), selector: None },
            ]
        },
        "⑤数値括弧"
    );
}

// ============================================================================
// 基準群 B: 不正形の KeyParseError 決定論（R1.3・R9.1・design UnitTests 1）
//   空セグメント／括弧不閉／非数値 index の 3 系統 ＋ 同一入力→同一 Ok/Err。
//   （個別檻: key.rs err_*・determinism_*）
// ============================================================================

#[test]
fn criterion_malformed_key_parse_errors() {
    // 空入力。
    assert_eq!(parse_dotted(""), Err(KeyParseError::Empty));
    // 空セグメント（連続ドット／先頭／末尾／括弧前空名）。
    assert_eq!(parse_dotted("a..b"), Err(KeyParseError::EmptySegment { at: 1 }));
    assert_eq!(parse_dotted(".a"), Err(KeyParseError::EmptySegment { at: 0 }));
    assert_eq!(parse_dotted("a."), Err(KeyParseError::EmptySegment { at: 1 }));
    assert_eq!(parse_dotted("(x)"), Err(KeyParseError::EmptySegment { at: 0 }));
    // 括弧不閉。
    assert_eq!(parse_dotted("foo(bar"), Err(KeyParseError::UnclosedParen { at: 0 }));
    // 非数値 index（index(...) は数値必須）＋ 数値括弧オーバーフロー。
    assert_eq!(parse_dotted("index(abc)"), Err(KeyParseError::BadIndex { at: 0 }));
    assert_eq!(parse_dotted("scope(99999999999999)"), Err(KeyParseError::BadIndex { at: 0 }));
}

#[test]
fn criterion_parse_is_deterministic_same_input_same_result() {
    // 成功系: 同一入力 → 同一 Ok。
    for input in ["areka.window.scope(2).y", "ghostlist(名前)", "system.baseware"] {
        assert_eq!(parse_dotted(input), parse_dotted(input), "Ok 決定論: {input}");
        assert!(parse_dotted(input).is_ok());
    }
    // 失敗系: 同一入力 → 同一 Err（エラー種別・位置とも一致）。
    for input in ["", "a..b", "foo(bar", "index(abc)"] {
        assert_eq!(parse_dotted(input), parse_dotted(input), "Err 決定論: {input}");
        assert!(parse_dotted(input).is_err());
    }
}

// ============================================================================
// 基準群 C: 台帳件数檻（R1.1/1.2/1.4/3.4・design UnitTests 2）
//   フラット 26／ルート枝 10／汎用名 17／Resource 159／SET 有効群全項の網羅。
//   （個別檻: 各 vocab ファイルの *_has_N_entries・set_effective_covers_all_*）
// ============================================================================

#[test]
fn criterion_ledger_counts_exact() {
    assert_eq!(FLAT_VOCAB.len(), 26, "フラット 26");
    assert_eq!(DOTTED_ROOTS.len(), 10, "ルート枝 10");
    assert_eq!(GENERIC_PROP_NAMES.len(), 17, "汎用名 17");
    assert_eq!(SHIORI_RESOURCE_IDS.len(), 159, "Resource 159");
}

#[test]
fn criterion_set_effective_full_group_coverage() {
    use std::collections::BTreeSet;
    // design.md 確定の SET 有効群 全 21 項（基本 3＋mousecursor 10＋seriko.cursor/tooltip 4＋menu 4）。
    let required: [&str; 21] = [
        "surface.num", "animation.num", "seriko.defaultsurface",
        "mousecursor", "mousecursor.text", "mousecursor.wait",
        "mousecursor.hand", "mousecursor.grip", "mousecursor.arrow",
        "balloon.mousecursor", "balloon.mousecursor.text",
        "balloon.mousecursor.wait", "balloon.mousecursor.arrow",
        "seriko.cursor.path", "seriko.cursor.name",
        "seriko.tooltip.text", "seriko.tooltip.name",
        "menu", "sakura.bind.menu", "kero.bind.menu", "char*.bind.menu",
    ];
    let keys: BTreeSet<&str> = SET_EFFECTIVE.iter().map(|(k, _)| *k).collect();
    let exp: BTreeSet<&str> = required.iter().copied().collect();
    // 過不足なし（欠落も余剰も許さない＝全項の網羅）。
    assert_eq!(keys, exp, "SET 有効群 全項の網羅");
    // 全項 RuntimeCommand（StoreWrite は正準語彙外の自由 key 用ゆえ SET 有効群には現れない）。
    assert!(
        SET_EFFECTIVE.iter().all(|(_, s)| *s == SetSemantics::RuntimeCommand),
        "SET 有効群は全て RuntimeCommand"
    );
}

// ============================================================================
// 基準群 D: username のみ ConsumerDefault（R3.1・design UnitTests 2）
//   （個別檻: flat.rs only_username_is_consumer_default）
// ============================================================================

#[test]
fn criterion_username_is_sole_consumer_default() {
    let consumer_defaults: Vec<&str> = FLAT_VOCAB
        .iter()
        .filter(|e| e.degrade == DegradePolicy::ConsumerDefault)
        .map(|e| e.token)
        .collect();
    assert_eq!(consumer_defaults, vec!["username"], "ConsumerDefault は username 唯一");
    // 他は全て PassThroughRaw（NotFound はフラット語彙に出現しない）。
    for e in FLAT_VOCAB {
        if e.token != "username" {
            assert_eq!(e.degrade, DegradePolicy::PassThroughRaw, "{} は PassThroughRaw", e.token);
        }
    }
}

// ============================================================================
// 基準群 E: `%*` が解決対象外（R1.6・design UnitTests 2）
//   `%*`・`%property[...]` は SYNTAX_RECORDS 専用で解決語彙（FLAT_VOCAB）に漏れない。
//   `\%`（エスケープ）はどの表にも含まれない。
//   （個別檻: flat.rs syntax_records_are_star_and_property）
// ============================================================================

#[test]
fn criterion_star_not_a_resolution_target() {
    assert_eq!(SYNTAX_RECORDS, &["*", "property[...]"], "構文記録は %* と %property[...]");
    // 解決語彙（FLAT_VOCAB）に `*`・`property[...]` は現れない。
    assert!(FLAT_VOCAB.iter().all(|e| e.token != "*"), "%* は解決対象外");
    assert!(FLAT_VOCAB.iter().all(|e| e.token != "property[...]"), "%property[...] は解決対象外");
    // `\%`（エスケープ記法）はいずれの表にも含まれない。
    assert!(!SYNTAX_RECORDS.contains(&"\\%") && !SYNTAX_RECORDS.contains(&"%"));
    assert!(FLAT_VOCAB.iter().all(|e| e.token != "\\%" && e.token != "%"));
}

// ============================================================================
// 基準群 F: ext イベント名は予約のみで発火しない（R3.5・design UnitTests 2）
//   EXT_EVENT_GET/SET は ukadoc の予約文字列そのもの。かつ「イベント名の予約」であって
//   「解決可能な語彙」ではない——どの解決台帳（FLAT_VOCAB／DOTTED_ROOTS／GENERIC_PROP_NAMES）
//   にも現れないことで、M1 で発火しない（＝解決経路に載らない）予約専用性を型・データ両面で檻化する。
//   （発火しないことの直接証跡は grep: クレート内に property.get/property.set の dispatch/emit なし。
//    本テストは「予約語彙が解決台帳に混入しない」不変量を新規に檻化する。）
// ============================================================================

#[test]
fn criterion_ext_event_names_reserved_only_not_resolution_vocab() {
    // 予約文字列そのもの（ukadoc 準拠）。
    assert_eq!(EXT_EVENT_GET, "property.get");
    assert_eq!(EXT_EVENT_SET, "property.set");
    assert_ne!(EXT_EVENT_GET, EXT_EVENT_SET);
    // イベント名予約であって解決語彙ではない: どの解決台帳にも混入しない。
    for reserved in [EXT_EVENT_GET, EXT_EVENT_SET] {
        assert!(
            FLAT_VOCAB.iter().all(|e| e.token != reserved),
            "{reserved} はフラット解決語彙に混入しない"
        );
        assert!(!DOTTED_ROOTS.contains(&reserved), "{reserved} はルート枝でない");
        assert!(!GENERIC_PROP_NAMES.contains(&reserved), "{reserved} は汎用名でない");
    }
}

// ============================================================================
// 基準群 G: 5 層 BackingLayer 型の存在（R3.6・design key モデル＋語彙台帳 Service Interface）
//   （個別檻: vocab/mod.rs backing_layer_has_five_values）
// ============================================================================

#[test]
fn criterion_backing_layer_five_taxonomy_exists() {
    let all = [
        BackingLayer::StaticConfig,
        BackingLayer::RuntimeState,
        BackingLayer::SystemEnv,
        BackingLayer::ShioriQuery,
        BackingLayer::Persistent,
    ];
    assert_eq!(all.len(), 5, "backing 実体層は 5 層");
    // フラット台帳が実際にこの層タクソノミーを使用している（型が飾りでない）。
    assert!(
        FLAT_VOCAB.iter().any(|e| e.layer == BackingLayer::ShioriQuery),
        "username は ShioriQuery 層"
    );
    assert!(FLAT_VOCAB.iter().any(|e| e.layer == BackingLayer::StaticConfig));
    assert!(FLAT_VOCAB.iter().any(|e| e.layer == BackingLayer::SystemEnv));
    assert!(FLAT_VOCAB.iter().any(|e| e.layer == BackingLayer::RuntimeState));
}
