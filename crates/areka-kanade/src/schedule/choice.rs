//! choice — 選択確定（`\q` 選択肢）の判定純関数（の純関数部分）。
//!
//! 本モジュールは選択確定にまつわる**判断分岐だけ**を副作用なしの純関数として置く層である
//! （ログも出さない・時刻も config も引数で受け取る）。状態型・Reference 構築・状態機械への
//! 配線は本層の外（`schedule/events.rs` ・`schedule/steady.rs` 側）が持つ。
//!
//! # 本層が確定する 2 つの写像
//! 1. [`plan_cascade`]: 選択肢 ID → 発火段列（[`CascadePlan`] の 3 分岐）。
//!    `script:` の後ろの台本は [`script_body`] が取り出す。
//! 2. [`choice_deadline`]: タイムアウト指令 → 選択待ちの期限（3 値語彙）。
//!
//! いずれも同一入力に対して常に同一の結果を返す。外部状態を読まないため、
//! 呼び手が既定値・起点時刻を渡す形にしてある（config への依存を純関数へ持ち込まない）。

use super::events;
use dola::{TimeDelta, TimePoint, TimeSpan};

/// 選択肢 ID から決まる発火段列。
///
/// 段列の実発行は（steady 調停）が担い、本 enum は「どの列を辿るか」だけを表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CascadePlan {
    /// `On` 始まり ID → 当該 ID と同名イベントの**任意名 1 段のみ**。
    ///
    /// `OnChoiceSelectEx`／`OnChoiceSelect` を先行発火しない（
    /// 正典の OnID 記述は先行段の有無に沈黙し、emo2 実物は直接発火に依存する）。
    Named,
    /// 正典形（`On` 始まりでない ID）→ `OnChoiceSelectEx` 先行・204 なら `OnChoiceSelect` 後続。
    ///
    /// 先行段が応答スクリプトを返したら後続段を発行しない。
    Canonical,
    /// `script:` で始まる ID → SHIORI のイベントを起こさず、`script:` の後ろの台本を
    /// 新しいトークとして実行する。
    ///
    /// 台本の取り出しは [`script_body`]、トークの開始と記録は調停側（`steady.rs`）の責務であり、
    /// 本層は「台本を実行する」という判定だけを返す。
    Script,
}

/// 選択肢の ID が `script:` で始まるとき、その後ろの台本を返す（後ろが空なら `Some("")`）。
///
/// 判定はバイト列の前方一致で、大文字小文字を区別する（`script`・`Script:x`・`xscript:y`・
/// 空文字列は `None`）。`"script:"` の綴りはここ 1 か所にだけ置く。
pub(crate) fn script_body(id: &str) -> Option<&str> {
    id.strip_prefix("script:")
}

/// 選択肢 ID から発火段列を一意に決める。
///
/// # 判定規則（この 3 分岐で全 ID を尽くす）
/// 1. [`script_body`] が `Some` → [`CascadePlan::Script`]
/// 2. [`events::is_allowed_choice_event`] が真（`On` 接頭） → [`CascadePlan::Named`]
/// 3. それ以外 → [`CascadePlan::Canonical`]
///
/// 判定順序は規則 1 を先に置く（`script:` は小文字始まりゆえ規則 2 とは実際には交差しないが、
/// `script:` の判定が常に先に効くことを構造で固定する）。
///
/// 規則 2 の境界（`"On"` 単独・大小文字・空文字列）は [`events::is_allowed_choice_event`] の規則に
/// 従う（境界の檻も `events_tests.rs` の `is_allowed_choice_event_accepts_only_on_prefixed_names_verbatim`）。
///
/// 外部状態を読まないため、同一 ID に対して常に同一の [`CascadePlan`] を返す。
pub(crate) fn plan_cascade(id: &str) -> CascadePlan {
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1
    if script_body(id).is_some() {
        CascadePlan::Script
    } else if events::is_allowed_choice_event(id) {
        CascadePlan::Named
    } else {
        CascadePlan::Canonical
    }
}

/// タイムアウト指令から選択待ちの期限を写す。
///
/// # 引数
/// - `display_end`: 起点＝当該トークの表示完了時刻（duration 権威・刻みと同じ物差し）。
/// - `timeout_directive`: バリアの生の指令値（`BarrierKind::WaitForChoice{timeout}`）。
/// - `default`: 未指定時に委譲する既定値。呼び手が `KanadeConfig::choice_timeout_default`
///   を渡す（config 依存を純関数へ持ち込まないための引数化）。
///
/// # 写像（3 値語彙。入口値は単一で、既定値かどうかで規律を変えない）
/// | 指令 | 意味 | 戻り値 |
/// |---|---|---|
/// | `None` | 未指定（既定へ委譲） | `Some(display_end + default)` |
/// | `Some(d)`, `d <= 0` | 無効化 | `None`（無期限・計測を開始しない） |
/// | `Some(d)`, `d > 0` | 明示の長さ指定 | `Some(display_end + d)` |
///
/// 戻り値の `None` は「期限なし＝無期限に選択待ちを継続する」を表す。引数の `None`（未指定）とは
/// 意味が異なる点に注意（引数 `None` は既定値加算へ写る）。足し算は時刻の型の端へ張り付く。
pub(crate) fn choice_deadline(
    display_end: TimePoint,
    timeout_directive: Option<TimeDelta>,
    default: TimeSpan,
) -> Option<TimePoint> {
    match timeout_directive {
        None => Some(display_end + default),
        Some(d) if d <= TimeDelta::ZERO => None,
        Some(d) => Some(display_end + d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === plan_cascade: 3 分岐の全網羅と script_body ===

    /// `On` 始まりの任意名 ID は任意名 1 段（emo2 実物 `menu.pasta` の依存形）。
    #[test]
    fn plan_cascade_named_for_on_prefixed_arbitrary_id() {
        assert_eq!(plan_cascade("Onおしゃべり頻度メニュー"), CascadePlan::Named);
        assert_eq!(plan_cascade("OnFoo"), CascadePlan::Named);
        // 正典固定 ID と同名でも `On` 始まりゆえ任意名 1 段（逐語発火）。
        assert_eq!(plan_cascade("OnChoiceSelect"), CascadePlan::Named);
        assert_eq!(plan_cascade("OnChoiceSelectEx"), CascadePlan::Named);
        // スケジューラ起源の恒久禁止 ID も、選択起源では逐語で 1 段に載せる。
        assert_eq!(plan_cascade("OnTalk"), CascadePlan::Named);
        assert_eq!(plan_cascade("OnHour"), CascadePlan::Named);
    }

    /// 通常の非 `On` 始まり ID と空文字列はいずれも正典形。
    #[test]
    fn plan_cascade_canonical_for_plain_and_empty_id() {
        assert_eq!(plan_cascade("choice1"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("メニュー"), CascadePlan::Canonical);
        assert_eq!(plan_cascade(""), CascadePlan::Canonical);
    }

    /// `script:` で始まる ID は「台本を実行する」結論になる。
    #[test]
    fn plan_cascade_script_for_script_prefix() {
        assert_eq!(plan_cascade("script:\\e"), CascadePlan::Script);
        // 後ろが空でも同じ結論（空の扱いは調停側が決める）。
        assert_eq!(plan_cascade("script:"), CascadePlan::Script);
        // `script:` の判定は `On` の判定より先に効く。
        assert_eq!(plan_cascade("script:OnFoo"), CascadePlan::Script);
    }

    /// 境界: 前置が不完全／大文字違い／途中出現なら `script:` ではない（正典形へ落ちる）。
    #[test]
    fn plan_cascade_canonical_for_near_miss_script_prefix() {
        assert_eq!(plan_cascade("script"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("Script:x"), CascadePlan::Canonical);
        assert_eq!(plan_cascade("xscript:y"), CascadePlan::Canonical);
    }

    /// `script_body`: `script:` の後ろをそのまま返す（空なら空文字列）。
    #[test]
    fn script_body_returns_text_after_prefix() {
        assert_eq!(script_body("script:\\e"), Some("\\e"));
        assert_eq!(script_body("script:"), Some(""));
    }

    // === choice_deadline: 3 値語彙 ===

    use dola::time::testing::{at_ms, delta_ms, ms};

    const DEFAULT: TimeSpan = ms(30_000);

    /// `None`＝未指定→既定値加算。
    #[test]
    fn choice_deadline_none_delegates_to_default() {
        assert_eq!(
            choice_deadline(at_ms(1_000), None, DEFAULT),
            Some(at_ms(31_000))
        );
        // 起点 0 でも規律は同一。
        assert_eq!(
            choice_deadline(at_ms(0), None, DEFAULT),
            Some(at_ms(30_000))
        );
    }

    /// 既定値以外の `default` でも同一規律で動く（入口値は単一）。
    #[test]
    fn choice_deadline_none_uses_supplied_default_verbatim() {
        assert_eq!(
            choice_deadline(at_ms(500), None, ms(1_234)),
            Some(at_ms(1_734))
        );
    }

    /// `0` は無効化＝無期限。
    #[test]
    fn choice_deadline_zero_is_indefinite() {
        assert_eq!(
            choice_deadline(at_ms(1_000), Some(TimeDelta::ZERO), DEFAULT),
            None
        );
    }

    /// 負値（正典の `-1`）は無効化＝無期限。
    #[test]
    fn choice_deadline_negative_is_indefinite() {
        assert_eq!(
            choice_deadline(at_ms(1_000), Some(delta_ms(-1_000)), DEFAULT),
            None
        );
        assert_eq!(
            choice_deadline(at_ms(1_000), Some(TimeDelta::from_ticks(-1)), DEFAULT),
            None
        );
        assert_eq!(
            choice_deadline(at_ms(1_000), Some(TimeDelta::MIN), DEFAULT),
            None
        );
    }

    /// 正値は明示の長さ指定＝起点＋指令。
    #[test]
    fn choice_deadline_positive_adds_directive() {
        assert_eq!(
            choice_deadline(at_ms(1_000), Some(delta_ms(5_000)), DEFAULT),
            Some(at_ms(6_000))
        );
        // 既定値は正値指定に一切関与しない（別の default でも結果不変）。
        assert_eq!(
            choice_deadline(at_ms(1_000), Some(delta_ms(5_000)), ms(999_999)),
            Some(at_ms(6_000))
        );
    }

    /// 最小の正値（100ns）も `Some` であり無期限とは区別される。
    #[test]
    fn choice_deadline_smallest_positive_is_not_indefinite() {
        assert_eq!(
            choice_deadline(at_ms(1_000), Some(TimeDelta::from_ticks(1)), DEFAULT),
            Some(TimePoint::from_ticks(at_ms(1_000).as_ticks() + 1))
        );
    }

    /// 極端値: 巨大な指令・起点の端のいずれも回り込まず時刻の型の端に留まる。
    #[test]
    fn choice_deadline_saturates_on_extreme_values() {
        assert_eq!(
            choice_deadline(at_ms(0), Some(TimeDelta::MAX), DEFAULT),
            Some(TimePoint::MAX)
        );
        assert_eq!(
            choice_deadline(TimePoint::MAX, Some(delta_ms(1_000)), DEFAULT),
            Some(TimePoint::MAX)
        );
        assert_eq!(
            choice_deadline(TimePoint::MAX, None, DEFAULT),
            Some(TimePoint::MAX)
        );
    }
}
