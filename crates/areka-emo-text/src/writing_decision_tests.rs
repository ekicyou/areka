//! # `WritingDirectionDecision` の決定点檻（純粋層・兄弟テストファイル）
//!
//! 守っているもの——「このスコープはどちら向きに書くのか」を答える**唯一の決定点**
//! （[`WritingDirectionDecision`]）の、次の 4 つの振る舞いを逐語で固定する。
//!
//! 1. **正典キー `vertical` の分類 4 分岐**——未宣言／`0`／`1`／不正値。不正値
//!    （`0`・`1` 以外および空文字列）は `warn!` ちょうど 1 件（本仕様 1.6／1.7）。
//! 2. **2 キーの共存規則**——単独宣言 5 形・一致併記・不一致併記・未知値／不正値の
//!    合流。採用結果（`mode`）だけでなく**採用出所**（`source`）と**矛盾併記の有無**
//!    （`conflicting`）まで固定する（本仕様 2.2〜2.5・2.7・10.3）。
//! 3. **記録水準の非対称**——値の破損は `warn!`・意図的な矛盾併記は `debug!`
//!    （本仕様の裁定 1）。両者を**別々に**数え、混同したら赤にする。
//! 4. **`.vertical` の導出 3 値**——横書き `0`／`vertical_rl` `1`／`vertical_lr` `1`
//!    （本仕様 7.1）。
//!
//! さらに、既存 API [`WritingMode::resolve`] が本決定点への薄い委譲であること
//! （戻り値が `mode()` と一致すること・本仕様 2.3）を代表入力集合で固定する。
//!
//! ## 0 件主張の規律（恒真の禁止）
//!
//! 「`warn` が 0 件」「`debug` が 0 件」という主張は、捕捉窓が死んでいても成立して
//! しまう。そこで本檻は捕捉窓の**内側**で対照イベント（`error!` 1 件）を発行し、
//! その 1 件が数えられていることを 0 件主張と必ず同時に確認する（[`assert_capture_alive`]）。
//! 対照の伴わない 0 件主張は本ファイルに置かない。
//!
//! ## 実行条件
//!
//! 実 DPI モニタ・実 GPU・実ゴースト・実窓を一切要さない純粋層の檻であり、同一入力に
//! 対して戻り値もログ件数も常に同一である（本仕様 10.6）。`windows` 系 crate を
//! import しない（本ファイル自身が `lib.rs` の構造檻
//! `pure_layer_modules_have_no_windows_imports` の走査対象に列挙されている）。

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use log_capture_kit::{LevelCounts, count_levels};

use super::{
    DirectionSource, VerticalDecl, WritingDirectionDecision, WritingMode, WritingModeDecl,
};

/// 対照イベント専用の宛先（本番コードがここへ発火することは無い）。
const CONTROL_TARGET: &str = "areka_emo_text::writing_decision_tests::control";

/// テスト用 `BalloonModel` 生成ヘルパ（`vertical` と `writing_mode` 以外は全成分未指定）。
///
/// `writing.rs` の既存インラインテストの `model` を、正典キー `vertical` の生値も
/// 取れる形へ広げたもの。`BalloonModel::new` は `vertical_raw` を持たないため、
/// 転記層と同じ additive ビルダ `with_vertical_raw` で相乗りさせる（2 層マージは
/// `balloon::parse` が既に確定させている前提＝本層はキー間の優劣だけを見る）。
fn model(vertical: Option<&str>, writing_mode: Option<&str>) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        writing_mode.map(str::to_owned),
        None,
    )
    .with_vertical_raw(vertical.map(str::to_owned))
}

/// 共有のログ捕捉窓の中で決定を行い、（決定, レベル別件数）を返す。
///
/// 件数の集計は硬化機構の唯一の定義元 `log-capture-kit` の [`count_levels`] に委ねる
/// （`warn` と `debug` を**別々に**数えるため、既存 `resolve_counting_warns` のように
/// `warn` へ潰さない）。窓の内側で対照イベントを 1 件発行し、[`assert_capture_alive`]
/// で捕捉が生きていることを示せるようにする。
fn decide(
    vertical: Option<&str>,
    writing_mode: Option<&str>,
) -> (WritingDirectionDecision, LevelCounts) {
    count_levels(|| {
        tracing::error!(
            target: CONTROL_TARGET,
            "捕捉窓の対照イベント（この 1 件が数えられないなら同窓の 0 件主張は無効）"
        );
        WritingDirectionDecision::resolve(&model(vertical, writing_mode))
    })
}

/// 捕捉窓が生きていたことを対照イベントの件数で示す（0 件主張の前提条件）。
fn assert_capture_alive(counts: &LevelCounts) {
    assert_eq!(
        counts.error, 1,
        "捕捉窓の対照イベントが数えられていない。この窓の warn／debug の 0 件主張は\
         「出なかった」ことの証拠にならない"
    );
}

// ── ⑴ 正典キー `vertical` の分類 4 分岐（本仕様 1.6／1.7） ──

/// 未宣言は `Undeclared`——`0` の宣言へ潰さない（共存規則の判定に宣言の有無が要る）。
#[test]
fn vertical_undeclared_is_classified_as_undeclared_without_records() {
    let (decision, counts) = decide(None, None);
    assert_eq!(decision.vertical_declaration(), VerticalDecl::Undeclared);
    assert_capture_alive(&counts);
    assert_eq!(counts.warn, 0, "未宣言は正常系につき warn を出さない");
    assert_eq!(counts.debug, 0, "未宣言は正常系につき debug を出さない");
}

/// `0` は「横書きの宣言」——未宣言とは別の分類として保つ。
#[test]
fn vertical_zero_is_classified_as_horizontal_without_records() {
    let (decision, counts) = decide(Some("0"), None);
    assert_eq!(decision.vertical_declaration(), VerticalDecl::Horizontal);
    assert_capture_alive(&counts);
    assert_eq!(counts.warn, 0);
    assert_eq!(counts.debug, 0);
}

/// `1` は「縦書きの宣言」（日本語縦書き＝右から左へ列送り・本仕様 2.2）。
#[test]
fn vertical_one_is_classified_as_vertical_without_records() {
    let (decision, counts) = decide(Some("1"), None);
    assert_eq!(decision.vertical_declaration(), VerticalDecl::Vertical);
    assert_capture_alive(&counts);
    assert_eq!(counts.warn, 0);
    assert_eq!(counts.debug, 0);
}

/// `0`／`1` 以外（`2`）と空文字列はいずれも `Invalid`＋`warn!` ちょうど 1 件。
///
/// 空文字列を別扱いしない（本仕様 1.7 は 1.6 と同一に扱うことを命じている）。
/// 併記が無い単独宣言なので、縮退先は正典既定の横書きになる（Flow 1 の `DefH`）。
#[test]
fn vertical_invalid_values_warn_exactly_once_and_fall_back_to_canon_default() {
    for value in ["2", ""] {
        let (decision, counts) = decide(Some(value), None);
        assert_eq!(
            decision.vertical_declaration(),
            VerticalDecl::Invalid,
            "vertical {value:?} は不正値として分類される"
        );
        assert_eq!(
            decision.mode(),
            WritingMode::HorizontalTb,
            "vertical {value:?} 単独は正典既定の横書きへ縮退する"
        );
        assert_eq!(
            decision.source(),
            DirectionSource::CanonDefault,
            "壊れた宣言は「指定なし」として合流するので採用出所は正典既定"
        );
        assert_capture_alive(&counts);
        assert_eq!(counts.warn, 1, "vertical {value:?} は warn ちょうど 1 件");
        assert_eq!(
            counts.debug, 0,
            "vertical {value:?} は矛盾併記ではないので debug を出さない"
        );
    }
}

// ── ⑵ 2 キーの共存規則（本仕様 2.2〜2.5・2.7・10.3） ──

/// 両キーとも宣言なし——正典既定の横書き・記録 0 件（本仕様 1.3）。
#[test]
fn no_declaration_uses_canon_default_without_records() {
    let (decision, counts) = decide(None, None);
    assert_eq!(decision.mode(), WritingMode::HorizontalTb);
    assert_eq!(decision.source(), DirectionSource::CanonDefault);
    assert_eq!(
        decision.writing_mode_declaration(),
        WritingModeDecl::Undeclared
    );
    assert!(!decision.conflicting());
    assert_capture_alive(&counts);
    assert_eq!(counts.warn, 0);
    assert_eq!(counts.debug, 0);
}

/// `vertical` 単独 2 形——`0`→横書き・`1`→`VerticalRl`（本仕様 2.2）・記録 0 件。
#[test]
fn vertical_alone_adopts_canon_key_without_records() {
    for (value, expected) in [
        ("0", WritingMode::HorizontalTb),
        ("1", WritingMode::VerticalRl),
    ] {
        let (decision, counts) = decide(Some(value), None);
        assert_eq!(decision.mode(), expected, "vertical,{value} の解決結果");
        assert_eq!(
            decision.source(),
            DirectionSource::CanonKey,
            "vertical 単独宣言は正典キーを採用する"
        );
        assert!(!decision.conflicting());
        assert_capture_alive(&counts);
        assert_eq!(counts.warn, 0, "vertical,{value} は warn を出さない");
        assert_eq!(counts.debug, 0, "vertical,{value} は debug を出さない");
    }
}

/// `writing_mode` 単独 3 形——現行と 1 ビットも変わらない（本仕様 2.3）・記録 0 件。
#[test]
fn writing_mode_alone_adopts_extension_key_without_records() {
    for (value, expected) in [
        ("horizontal_tb", WritingMode::HorizontalTb),
        ("vertical_rl", WritingMode::VerticalRl),
        ("vertical_lr", WritingMode::VerticalLr),
    ] {
        let (decision, counts) = decide(None, Some(value));
        assert_eq!(decision.mode(), expected, "writing_mode,{value} の解決結果");
        assert_eq!(
            decision.writing_mode_declaration(),
            WritingModeDecl::Declared(expected)
        );
        assert_eq!(
            decision.source(),
            DirectionSource::ExtensionKey,
            "writing_mode 単独宣言は拡張キーを採用する"
        );
        assert!(!decision.conflicting());
        assert_capture_alive(&counts);
        assert_eq!(counts.warn, 0, "writing_mode,{value} は warn を出さない");
        assert_eq!(counts.debug, 0, "writing_mode,{value} は debug を出さない");
    }
}

/// 一致併記——その方向を採り、**記録は warn も debug も 0 件**（本仕様 2.4）。
///
/// 採用出所は方向が一致していても `ExtensionKey` である。優先順位は方向の一致・
/// 不一致に依らず一定であり、一致・不一致は**記録の有無だけ**を変える。
#[test]
fn agreeing_declarations_resolve_without_any_record() {
    for (vertical, writing_mode, expected) in [
        ("0", "horizontal_tb", WritingMode::HorizontalTb),
        ("1", "vertical_rl", WritingMode::VerticalRl),
    ] {
        let (decision, counts) = decide(Some(vertical), Some(writing_mode));
        assert_eq!(
            decision.mode(),
            expected,
            "vertical,{vertical} ＋ writing_mode,{writing_mode} の解決結果"
        );
        assert_eq!(
            decision.source(),
            DirectionSource::ExtensionKey,
            "両キーが有効宣言なら方向一致でも採用出所は拡張キー"
        );
        assert!(
            !decision.conflicting(),
            "同じ方向を意味する併記は矛盾ではない"
        );
        assert_capture_alive(&counts);
        assert_eq!(counts.warn, 0, "一致併記は warn を出さない");
        assert_eq!(counts.debug, 0, "一致併記は debug を出さない");
    }
}

/// 不一致併記——拡張キー `writing_mode` を採用し、`debug!` ちょうど 1 件・warn 0 件。
///
/// 記録水準の非対称は意図的である（本仕様の裁定 1）——矛盾併記は areka を知る作者の
/// 意図的な状態であって値の破損ではないため、警告にはしない。
///
/// 3 組目 `vertical,1` ＋ `writing_mode,vertical_lr` も**異なる方向**である。本仕様 2.2 が
/// `vertical,1` を `VerticalRl` へ逐語固定しており、`vertical_lr` はそれとは別の方向
/// （列送りが左向き）だからである。「どちらも縦書きだから一致」とは扱わない。
#[test]
fn conflicting_declarations_adopt_extension_key_with_exactly_one_debug() {
    for (vertical, writing_mode, expected) in [
        ("0", "vertical_rl", WritingMode::VerticalRl),
        ("1", "horizontal_tb", WritingMode::HorizontalTb),
        ("1", "vertical_lr", WritingMode::VerticalLr),
    ] {
        let (decision, counts) = decide(Some(vertical), Some(writing_mode));
        assert_eq!(
            decision.mode(),
            expected,
            "vertical,{vertical} ＋ writing_mode,{writing_mode} は拡張キーの方向を採る"
        );
        assert_eq!(decision.source(), DirectionSource::ExtensionKey);
        assert!(
            decision.conflicting(),
            "vertical,{vertical} ＋ writing_mode,{writing_mode} は異なる方向の併記である"
        );
        assert_capture_alive(&counts);
        assert_eq!(
            counts.debug, 1,
            "矛盾併記は debug ちょうど 1 件（採らなかった正典キーの生値と採った拡張キーの生値）"
        );
        assert_eq!(
            counts.warn, 0,
            "矛盾併記は値の破損ではないので warn へ格上げしない"
        );
    }
}

/// `writing_mode` 未知値 ＋ `vertical` 宣言——`vertical` を採用（本仕様 2.7）。
///
/// 未知値は「指定なし」として合流するため、これは矛盾併記ではない（`debug` 0 件）。
/// warn は未知値の 1 件だけである。
#[test]
fn unknown_writing_mode_falls_back_to_declared_vertical() {
    let (decision, counts) = decide(Some("1"), Some("diagonal_bt"));
    assert_eq!(decision.mode(), WritingMode::VerticalRl);
    assert_eq!(
        decision.writing_mode_declaration(),
        WritingModeDecl::Unknown
    );
    assert_eq!(
        decision.source(),
        DirectionSource::CanonKey,
        "未知値は指定なし扱いなので正典キーが採られる"
    );
    assert!(
        !decision.conflicting(),
        "有効宣言が片方だけなので矛盾併記ではない"
    );
    assert_capture_alive(&counts);
    assert_eq!(
        counts.warn, 1,
        "未知の writing_mode 値は warn ちょうど 1 件"
    );
    assert_eq!(counts.debug, 0, "矛盾併記ではないので debug は出ない");
}

/// `vertical` 不正値 ＋ `writing_mode` 有効宣言——拡張キーを採用（設計 DD6 の合流）。
///
/// 前項と対称の形である。壊れた `vertical` は「指定なし」として合流するので、
/// これも矛盾併記ではない（`debug` 0 件）。warn は不正値の 1 件だけ。
#[test]
fn invalid_vertical_falls_back_to_declared_writing_mode() {
    let (decision, counts) = decide(Some("2"), Some("vertical_rl"));
    assert_eq!(decision.mode(), WritingMode::VerticalRl);
    assert_eq!(decision.vertical_declaration(), VerticalDecl::Invalid);
    assert_eq!(
        decision.source(),
        DirectionSource::ExtensionKey,
        "不正値は指定なし扱いなので拡張キーが採られる"
    );
    assert!(
        !decision.conflicting(),
        "有効宣言が片方だけなので矛盾併記ではない"
    );
    assert_capture_alive(&counts);
    assert_eq!(counts.warn, 1, "vertical の不正値は warn ちょうど 1 件");
    assert_eq!(counts.debug, 0, "矛盾併記ではないので debug は出ない");
}

/// 両キーとも壊れている——正典既定の横書きへ縮退し、warn は 2 件（各キー 1 件ずつ）。
#[test]
fn both_declarations_broken_fall_back_to_canon_default_with_two_warns() {
    let (decision, counts) = decide(Some("2"), Some("diagonal_bt"));
    assert_eq!(decision.mode(), WritingMode::HorizontalTb);
    assert_eq!(decision.source(), DirectionSource::CanonDefault);
    assert!(!decision.conflicting());
    assert_capture_alive(&counts);
    assert_eq!(counts.warn, 2, "壊れた宣言 2 つでそれぞれ 1 件ずつ");
    assert_eq!(counts.debug, 0);
}

// ── ⑶ `.vertical` の導出 3 値（本仕様 7.1・設計 DD7） ──

/// `.vertical` は実際に適用されている書字方向から一意に定まる。
///
/// **`vertical_lr`（areka 拡張の縦書き左送り）も `1`** である——正典 SSP の語彙は
/// 縦横 2 値であり、列送りの向きを区別しないため、縦書き 2 モードは同じ `1` へ写る。
/// 本仕様はこの値を publish しない（実導出は追跡 spec
/// `areka-P0-currentghost-property-tree` が所有する）。
#[test]
fn vertical_property_value_maps_three_modes_to_two_canon_values() {
    for (writing_mode, expected_mode, expected_value) in [
        ("horizontal_tb", WritingMode::HorizontalTb, 0u8),
        ("vertical_rl", WritingMode::VerticalRl, 1u8),
        ("vertical_lr", WritingMode::VerticalLr, 1u8),
    ] {
        let decision = WritingDirectionDecision::resolve(&model(None, Some(writing_mode)));
        assert_eq!(decision.mode(), expected_mode);
        assert_eq!(
            decision.vertical_property_value(),
            expected_value,
            "writing_mode,{writing_mode} の .vertical 導出値"
        );
    }
}

/// 正典キー `vertical` の生値と `.vertical` の導出値は、有効宣言のときに一致する。
///
/// `vertical,1` → `1`・`vertical,0` → `0`。壊れた宣言は横書きへ縮退するので `0`。
#[test]
fn vertical_property_value_round_trips_declared_canon_key() {
    for (declared, expected_value) in [("0", 0u8), ("1", 1u8), ("2", 0u8), ("", 0u8)] {
        let decision = WritingDirectionDecision::resolve(&model(Some(declared), None));
        assert_eq!(
            decision.vertical_property_value(),
            expected_value,
            "vertical,{declared:?} の .vertical 導出値"
        );
    }
}

// ── ⑷ 既存 API の委譲同値（本仕様 2.3） ──

/// [`WritingMode::resolve`] は決定点への薄い委譲であり、戻り値は常に `mode()` と一致する。
///
/// 代表入力集合は共存規則の全腕（単独・一致併記・不一致併記・未知値／不正値の合流・
/// 両キー未宣言）を含む。ここが崩れると、既存の本番唯一の呼出（`actor.rs` の
/// `ResolvedBalloonText::resolve` 内）が決定点と別の答えを返すことになる。
#[test]
fn writing_mode_resolve_delegates_to_decision_mode() {
    const CASES: &[(Option<&str>, Option<&str>)] = &[
        (None, None),
        (Some("0"), None),
        (Some("1"), None),
        (Some("2"), None),
        (Some(""), None),
        (None, Some("horizontal_tb")),
        (None, Some("vertical_rl")),
        (None, Some("vertical_lr")),
        (None, Some("diagonal_bt")),
        (Some("0"), Some("horizontal_tb")),
        (Some("1"), Some("vertical_rl")),
        (Some("0"), Some("vertical_rl")),
        (Some("1"), Some("horizontal_tb")),
        (Some("1"), Some("vertical_lr")),
        (Some("1"), Some("diagonal_bt")),
        (Some("2"), Some("vertical_rl")),
        (Some("2"), Some("diagonal_bt")),
    ];

    for (vertical, writing_mode) in CASES {
        let via_api = WritingMode::resolve(&model(*vertical, *writing_mode));
        let via_decision =
            WritingDirectionDecision::resolve(&model(*vertical, *writing_mode)).mode();
        assert_eq!(
            via_api, via_decision,
            "vertical={vertical:?} writing_mode={writing_mode:?} で委譲が一致しない"
        );
    }
}

/// 決定論——同一入力に対し、戻り値もログ件数も同一である（本仕様 10.6）。
#[test]
fn decision_is_deterministic_in_value_and_record_counts() {
    for (vertical, writing_mode) in [
        (None, None),
        (Some("1"), None),
        (Some("2"), None),
        (Some("1"), Some("vertical_lr")),
        (Some("1"), Some("diagonal_bt")),
    ] {
        let (first_decision, first_counts) = decide(vertical, writing_mode);
        let (second_decision, second_counts) = decide(vertical, writing_mode);
        assert_eq!(
            first_decision, second_decision,
            "vertical={vertical:?} writing_mode={writing_mode:?} の決定が 2 回で異なる"
        );
        assert_eq!(
            first_counts, second_counts,
            "vertical={vertical:?} writing_mode={writing_mode:?} のログ件数が 2 回で異なる"
        );
    }
}
