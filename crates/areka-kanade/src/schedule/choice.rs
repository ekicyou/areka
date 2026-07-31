//! choice — 選択確定（`\q` 選択肢）の判定純関数（設計 C3 の純関数部分）。
//!
//! 本モジュールは選択確定にまつわる**判断分岐だけ**を副作用なしの純関数として置く層である
//! （ログも出さない・時刻も config も引数で受け取る）。状態型・Reference 構築・状態機械への
//! 配線は本層の外（設計 C3 の `schedule/events.rs` 側・C4 の `schedule/steady.rs` 側）が持つ。
//!
//! # 本層が確定する 2 つの写像
//! 1. [`plan_cascade`]: 選択肢 ID → 発火段列（[`CascadePlan`] の 3 分岐・Req2.1/2.2/2.5/2.7）。
//! 2. [`choice_deadline`]: タイムアウト指令 → 選択待ちの期限（DD-8 の 3 値語彙・Req7.6/7.7）。
//!
//! いずれも同一入力に対して常に同一の結果を返す（Req2.5）。外部状態を読まないため、
//! 呼び手が既定値・起点時刻を渡す形にしてある（config への依存を純関数へ持ち込まない）。

use crate::msg::MonotonicMs;

/// 選択肢 ID から決まる発火段列（設計 C3・「カスケード則の正典裁定」1/7）。
///
/// 段列の実発行は C4（steady 調停）が担い、本 enum は「どの列を辿るか」だけを表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// C4（タスク 4.x）が steady 調停から本判定を呼ぶまでは非テストビルドで未使用となる。
#[allow(dead_code)]
pub(crate) enum CascadePlan {
    /// `On` 始まり ID → 当該 ID と同名イベントの**任意名 1 段のみ**（Req2.1）。
    ///
    /// `OnChoiceSelectEx`／`OnChoiceSelect` を先行発火しない（裁定 1・provenance=areka_discretion。
    /// 正典の OnID 記述は先行段の有無に沈黙し、emo2 実物は直接発火に依存する）。
    Named,
    /// 正典形（`On` 始まりでない ID）→ `OnChoiceSelectEx` 先行・204 なら `OnChoiceSelect` 後続（Req2.2）。
    ///
    /// 先行段が応答スクリプトを返したら後続段を発行しない（裁定 2・Req2.4）。
    Canonical,
    /// M1 未対応カテゴリ（`script:` 前置）→ イベントを発行せず、警告のうえ選択待ちの解決のみ行う（Req2.7）。
    ///
    /// 裁定 7（provenance=areka_discretion の明示縮退）。警告と解決は C4 の責務であり、
    /// 本層は「未対応である」という判定だけを返す。
    Unsupported,
}

/// 選択肢 ID から発火段列を一意に決める（Req2.1/2.2/2.5/2.7・設計 C3）。
///
/// # 判定規則（設計 C3 の記述をそのまま実装。この 3 分岐で全 ID を尽くす）
/// 1. `id.starts_with("script:")` → [`CascadePlan::Unsupported`]
/// 2. `id.starts_with("On")` → [`CascadePlan::Named`]
/// 3. それ以外 → [`CascadePlan::Canonical`]
///
/// 判定順序は規則 1 を先に置く（`script:` は小文字始まりゆえ規則 2 とは実際には交差しないが、
/// 未対応カテゴリの判定が常に最優先であることを裁定 7 の明示として構造で固定する）。
///
/// # 境界の読み
/// - `"On"` 単独は `starts_with("On")` に一致するため [`CascadePlan::Named`]。任意名イベント
///   `On` を逐語発火する形であり、正典形へは落とさない。
/// - `"on..."`（小文字始まり）は一致しないため [`CascadePlan::Canonical`]。判定は大文字小文字を
///   区別する（正典のイベント名は `On` 始まりで大小を区別するため）。
/// - `""`（空文字列）はいずれの接頭辞にも一致しないため [`CascadePlan::Canonical`]。
/// - 比較はバイト列の接頭辞一致であり、後続が非 ASCII でも UTF-8 境界を割らない
///   （`"On"` / `"script:"` はともに ASCII のみで構成される）。
///
/// 外部状態を読まないため、同一 ID に対して常に同一の [`CascadePlan`] を返す（Req2.5）。
// C4（タスク 4.x）が steady 調停から本判定を呼ぶまでは非テストビルドで未使用となる。
#[allow(dead_code)]
pub(crate) fn plan_cascade(id: &str) -> CascadePlan {
    if id.starts_with("script:") {
        CascadePlan::Unsupported
    } else if id.starts_with("On") {
        CascadePlan::Named
    } else {
        CascadePlan::Canonical
    }
}

/// タイムアウト指令から選択待ちの期限を写す（DD-8・Req7.6/7.7・設計 F3）。
///
/// # 引数
/// - `display_end`: 起点＝当該トークの表示完了時刻（duration 権威から dispatcher が換算済み・Req7.2）。
/// - `timeout_directive_secs`: バリアの生の指令値（`BarrierKind::WaitForChoice{timeout}`）。
/// - `default_ms`: 未指定時に委譲する既定値。呼び手が `KanadeConfig::choice_timeout_default_ms`
///   を渡す（config 依存を純関数へ持ち込まないための引数化）。
///
/// # 写像（DD-8 の 3 値語彙。入口値は単一で、既定値かどうかで規律を変えない・Req7.7）
/// | 指令 | 意味 | 戻り値 |
/// |---|---|---|
/// | `None` | 未指定（既定へ委譲） | `Some(display_end + default_ms)` |
/// | `Some(v)`, `v <= 0.0` | 無効化 | `None`（無期限・計測を開始しない・Req7.6） |
/// | `Some(v)`, `v > 0.0` | 明示秒指定 | `Some(display_end + v*1000 ms)` |
///
/// 戻り値の `None` は「期限なし＝無期限に選択待ちを継続する」を表す。引数の `None`（未指定）とは
/// 意味が異なる点に注意（引数 `None` は既定値加算へ写る）。
///
/// # 端数と極端値
/// - 秒→ミリ秒は `v * 1000.0` を [`f64::round`]（四捨五入・端数 0.5 は絶対値の大きい側）で丸める。
///   `0.0005` 秒のような既に正の微小値は 0ms へ丸まり得るが、戻り値は `Some` のまま（即時期限）で
///   あり、無期限（`None`）とは区別される。
/// - `f64::INFINITY` や u64 を超える巨大値は飽和キャストと [`u64::saturating_add`] で
///   [`u64::MAX`] に留まり、オーバーフローしない。
/// - `f64::NAN` は DD-8 の 3 値語彙の外にあるため、無効化と同じ無期限（`None`）へ畳む。
///   NaN は `v <= 0.0` にも `v > 0.0` にも一致しない（順序が部分的）ため、
///   [`f64::is_nan`] による明示判定を無効化アームへ併記して決定論を構造で固定する。
pub(crate) fn choice_deadline(
    display_end: MonotonicMs,
    timeout_directive_secs: Option<f64>,
    default_ms: u64,
) -> Option<MonotonicMs> {
    let elapsed_ms = match timeout_directive_secs {
        // 未指定＝既定へ委譲（DD-8）。
        None => default_ms,
        // 0／負値＝無効化＝無期限（Req7.6・DD-8）。語彙外の NaN も同じ側へ畳む。
        Some(v) if v.is_nan() || v <= 0.0 => return None,
        // 明示秒指定（DD-8）。飽和キャストにより INFINITY・巨大値は u64::MAX へ留まる。
        Some(v) => (v * 1000.0).round() as u64,
    };
    Some(MonotonicMs(display_end.0.saturating_add(elapsed_ms)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === plan_cascade: 3 分岐の全網羅（Req2.1/2.2/2.5/2.7） ===

    /// Req2.1: `On` 始まりの任意名 ID は任意名 1 段（emo2 実物 `menu.pasta` の依存形）。
    #[test]
    fn plan_cascade_named_for_on_prefixed_arbitrary_id() {
        assert_eq!(plan_cascade("Onおしゃべり頻度メニュー"), CascadePlan::Named);
        assert_eq!(plan_cascade("OnFoo"), CascadePlan::Named);
        // 正典固定 ID と同名でも `On` 始まりゆえ任意名 1 段（逐語発火・裁定 1）。
        assert_eq!(plan_cascade("OnChoiceSelect"), CascadePlan::Named);
        assert_eq!(plan_cascade("OnChoiceSelectEx"), CascadePlan::Named);
        // Req2.9: スケジューラ起源の恒久禁止 ID も、選択起源では逐語で 1 段に載せる。
        assert_eq!(plan_cascade("OnTalk"), CascadePlan::Named);
        assert_eq!(plan_cascade("OnHour"), CascadePlan::Named);
    }

    /// 境界: `"On"` 単独も接頭辞一致＝Named（正典形へ落とさない）。
    #[test]
    fn plan_cascade_named_for_bare_on() {
        assert_eq!(plan_cascade("On"), CascadePlan::Named);
    }

    /// 境界: 非 ASCII 後続でも接頭辞判定が割れない（UTF-8 境界を割らない）。
    #[test]
    fn plan_cascade_named_for_on_with_non_ascii_tail() {
        assert_eq!(plan_cascade("Onほにゃらら"), CascadePlan::Named);
    }

    /// Req2.2 境界: 小文字始まりは `On` に一致しない＝正典形（Ex 先行・無印後続）。
    #[test]
    fn plan_cascade_canonical_for_lowercase_on_prefix() {
        assert_eq!(plan_cascade("on_menu"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("on"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("oN"), CascadePlan::Canonical);
    }

    /// 境界: 2 文字目が大文字（`"ONFOO"`）は `starts_with("On")` に一致せず正典形。
    #[test]
    fn plan_cascade_canonical_for_all_uppercase_on() {
        assert_eq!(plan_cascade("ONFOO"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("ON"), CascadePlan::Canonical);
    }

    /// Req2.2: 通常の非 `On` 始まり ID と空文字列はいずれも正典形。
    #[test]
    fn plan_cascade_canonical_for_plain_and_empty_id() {
        assert_eq!(plan_cascade("choice1"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("メニュー"), CascadePlan::Canonical);
        assert_eq!(plan_cascade(""), CascadePlan::Canonical);
    }

    /// Req2.7・裁定 7: `script:` 前置は M1 未対応カテゴリ。
    #[test]
    fn plan_cascade_unsupported_for_script_prefix() {
        assert_eq!(plan_cascade("script:\\e"), CascadePlan::Unsupported);
        // 前置のみ（後続が空）でも未対応カテゴリ。
        assert_eq!(plan_cascade("script:"), CascadePlan::Unsupported);
        // 未対応判定は `On` 判定より優先する（構造上の順序固定）。
        assert_eq!(plan_cascade("script:OnFoo"), CascadePlan::Unsupported);
    }

    /// 境界: 前置が不完全／大文字違い／途中出現なら未対応ではない（正典形へ落ちる）。
    #[test]
    fn plan_cascade_canonical_for_near_miss_script_prefix() {
        assert_eq!(plan_cascade("script"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("Script:x"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("xscript:y"), CascadePlan::Canonical);
    }

    /// Req2.5: 同一入力に対して常に同一の列を導く（副作用も内部状態も持たない）。
    #[test]
    fn plan_cascade_is_deterministic_for_repeated_input() {
        for id in [
            "Onおしゃべり頻度メニュー",
            "On",
            "on_menu",
            "script:\\e",
            "choice1",
            "",
        ] {
            let first = plan_cascade(id);
            for _ in 0..8 {
                assert_eq!(plan_cascade(id), first, "同一入力 {id:?} が別の列を導いた");
            }
        }
    }

    // === choice_deadline: DD-8 の 3 値語彙（Req7.6/7.7） ===

    const DEFAULT_MS: u64 = 30_000;

    /// DD-8: `None`＝未指定→既定値加算。
    #[test]
    fn choice_deadline_none_delegates_to_default() {
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), None, DEFAULT_MS),
            Some(MonotonicMs(31_000))
        );
        // 起点 0 でも規律は同一。
        assert_eq!(
            choice_deadline(MonotonicMs(0), None, DEFAULT_MS),
            Some(MonotonicMs(30_000))
        );
    }

    /// Req7.7: 既定値以外の `default_ms` でも同一規律で動く（入口値は単一）。
    #[test]
    fn choice_deadline_none_uses_supplied_default_verbatim() {
        assert_eq!(
            choice_deadline(MonotonicMs(500), None, 1_234),
            Some(MonotonicMs(1_734))
        );
    }

    /// Req7.6・DD-8: `0` は無効化＝無期限。
    #[test]
    fn choice_deadline_zero_is_indefinite() {
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(0.0), DEFAULT_MS),
            None
        );
        // 負のゼロも 0 と同値（`-0.0 > 0.0` は偽）。
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(-0.0), DEFAULT_MS),
            None
        );
    }

    /// Req7.6・DD-8: 負値（正典の `-1`）は無効化＝無期限。
    #[test]
    fn choice_deadline_negative_is_indefinite() {
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(-1.0), DEFAULT_MS),
            None
        );
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(-0.001), DEFAULT_MS),
            None
        );
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(f64::NEG_INFINITY), DEFAULT_MS),
            None
        );
    }

    /// DD-8: 正値は明示秒指定＝起点＋秒*1000ms。
    #[test]
    fn choice_deadline_positive_adds_seconds() {
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(5.0), DEFAULT_MS),
            Some(MonotonicMs(6_000))
        );
        // 既定値は正値指定に一切関与しない（別の default_ms でも結果不変）。
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(5.0), 999_999),
            Some(MonotonicMs(6_000))
        );
    }

    /// 端数: 秒→ms は四捨五入で丸める（丸め規則の固定）。
    #[test]
    fn choice_deadline_rounds_fractional_seconds_half_away_from_zero() {
        // 0.5 秒 → 500ms（割り切れる小数）。
        assert_eq!(
            choice_deadline(MonotonicMs(0), Some(0.5), DEFAULT_MS),
            Some(MonotonicMs(500))
        );
        // 1.2345 秒 → 1234.5ms → 1235ms（0.5 は絶対値の大きい側へ）。
        assert_eq!(
            choice_deadline(MonotonicMs(0), Some(1.2345), DEFAULT_MS),
            Some(MonotonicMs(1_235))
        );
        // 1.2344 秒 → 1234.4ms → 1234ms（切り下げ側）。
        assert_eq!(
            choice_deadline(MonotonicMs(0), Some(1.2344), DEFAULT_MS),
            Some(MonotonicMs(1_234))
        );
    }

    /// 端数境界: 0ms へ丸まる微小な正値も `Some`（即時期限）であり無期限とは区別される。
    #[test]
    fn choice_deadline_tiny_positive_is_immediate_not_indefinite() {
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(1e-9), DEFAULT_MS),
            Some(MonotonicMs(1_000))
        );
    }

    /// 極端値: 無限大・巨大値・起点飽和のいずれもオーバーフローせず u64::MAX に留まる。
    #[test]
    fn choice_deadline_saturates_on_extreme_values() {
        assert_eq!(
            choice_deadline(MonotonicMs(0), Some(f64::INFINITY), DEFAULT_MS),
            Some(MonotonicMs(u64::MAX))
        );
        assert_eq!(
            choice_deadline(MonotonicMs(0), Some(1e300), DEFAULT_MS),
            Some(MonotonicMs(u64::MAX))
        );
        // 起点側が既に最大でも加算で回り込まない。
        assert_eq!(
            choice_deadline(MonotonicMs(u64::MAX), Some(1.0), DEFAULT_MS),
            Some(MonotonicMs(u64::MAX))
        );
        assert_eq!(
            choice_deadline(MonotonicMs(u64::MAX), None, DEFAULT_MS),
            Some(MonotonicMs(u64::MAX))
        );
    }

    /// 語彙外の NaN は無期限（`None`）へ畳む（決定論的な既定の読み）。
    #[test]
    fn choice_deadline_nan_is_indefinite() {
        assert_eq!(
            choice_deadline(MonotonicMs(1_000), Some(f64::NAN), DEFAULT_MS),
            None
        );
    }

    /// 同一入力に対して常に同一の期限を返す（副作用なし・実時刻を読まない）。
    #[test]
    fn choice_deadline_is_deterministic_for_repeated_input() {
        for directive in [None, Some(0.0), Some(-1.0), Some(2.5)] {
            let first = choice_deadline(MonotonicMs(1_000), directive, DEFAULT_MS);
            for _ in 0..8 {
                assert_eq!(
                    choice_deadline(MonotonicMs(1_000), directive, DEFAULT_MS),
                    first,
                    "同一入力 {directive:?} が別の期限を導いた"
                );
            }
        }
    }
}
