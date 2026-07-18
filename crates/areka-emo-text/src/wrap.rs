//! # wrap — budoux_newline 宣言の折返しモード解釈（純粋層）
//!
//! balloon descript の `budoux_newline` 転記値（2層マージ後勝ち解決済み・生文字列）を
//! `WrapMode`（`CharByChar`／`BudouxWordWrap`）へ解決する。討議 #1 決定（2026-07-18）で
//! 受理値は bool 2 値（ON＝`1`/`true`・OFF＝`0`/`false`）に閉じるが、解決結果型は将来の
//! ワードラップ戦略名（禁則強化・欧文ハイフネーション等）を variant 追加で第一級化しうる
//! enum シームとする。
//!
//! `writing.rs` の `WritingMode::resolve` と同一姿勢: 欠落/OFF は正常系（ログなし・R1.3）、
//! 未知値は値を含む `warn!` ＋ OFF フォールバック（縮退継続・R1.4・log-first）。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。

use areka_parsers::balloon::BalloonModel;

/// `budoux_newline` の ON 受理値（`1`・R1.2）。
const VALUE_ON_ONE: &str = "1";
/// `budoux_newline` の ON 受理値（`true`・R1.2）。
const VALUE_ON_TRUE: &str = "true";
/// `budoux_newline` の OFF 受理値（`0`・R1.3）。
const VALUE_OFF_ZERO: &str = "0";
/// `budoux_newline` の OFF 受理値（`false`・R1.3）。
const VALUE_OFF_FALSE: &str = "false";

/// `budoux_newline` 宣言の解決結果（折返しモード・将来の戦略名第一級化シーム・討議 #1）。
///
/// 本 spec の受理語彙は bool 2 値だが、型は将来のワードラップ戦略名（禁則強化・欧文
/// ハイフネーション等）を variant 追加で第一級化しうる enum とする。折返し計画
/// （`WrapPlan`）への写像は layout 層の領分。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WrapMode {
    /// 従来の文字単位折返し（既定・キー無し/`0`/`false`/未知値フォールバック・R1.3）。
    #[default]
    CharByChar,
    /// budoux 分かち書き境界ワードラップ（`1`/`true`・R1.2）。
    BudouxWordWrap,
}

impl WrapMode {
    /// `BalloonModel` の転記値（2層マージ後勝ち解決済み・R1.1）から有効値を解釈する。
    ///
    /// - マーカー無し（`None`）／`0`／`false` → [`CharByChar`](Self::CharByChar)
    ///   （正常系につきログなし・R1.3）。
    /// - `1`／`true` → [`BudouxWordWrap`](Self::BudouxWordWrap)（R1.2）。
    /// - 未知の値 → `warn!`（値を含む）＋ [`CharByChar`](Self::CharByChar) へ
    ///   フォールバック（縮退継続・R1.4）。
    /// - 語彙は完全一致（trim は parser の kv 層で済んでいる前提）。`TRUE`／`on`／
    ///   空文字等は未知値扱い。
    pub fn resolve(model: &BalloonModel) -> WrapMode {
        match model.budoux_newline() {
            None => WrapMode::CharByChar,
            Some(VALUE_OFF_ZERO) | Some(VALUE_OFF_FALSE) => WrapMode::CharByChar,
            Some(VALUE_ON_ONE) | Some(VALUE_ON_TRUE) => WrapMode::BudouxWordWrap,
            Some(unknown) => {
                tracing::warn!(
                    value = unknown,
                    "未知の budoux_newline 値のため OFF（文字単位折返し）へフォールバックする（受理語彙: 1 / true / 0 / false）"
                );
                WrapMode::CharByChar
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

    use super::WrapMode;

    /// テスト用 BalloonModel 生成ヘルパ（budoux_newline 以外は全成分未指定）。
    /// `budoux_newline` は `BalloonModel::new` の最終位置引数（writing_mode の次）。
    fn model(budoux_newline: Option<&str>) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(None, None),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            Font::new(None, None, FontColor::new(None, None, None)),
            None, // writing_mode
            budoux_newline.map(str::to_owned),
        )
    }

    /// WARN イベント数を数える最小 Subscriber（決定論的なログ檻・writing.rs の檻パターン踏襲）。
    struct WarnCounter {
        warns: Arc<AtomicUsize>,
    }

    impl tracing::Subscriber for WarnCounter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.warns.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    /// resolve を WARN 檻の中で実行し、（解決結果, WARN 件数）を返す。
    fn resolve_counting_warns(budoux_newline: Option<&str>) -> (WrapMode, usize) {
        let warns = Arc::new(AtomicUsize::new(0));
        let subscriber = WarnCounter {
            warns: Arc::clone(&warns),
        };
        let mode = tracing::subscriber::with_default(subscriber, || {
            WrapMode::resolve(&model(budoux_newline))
        });
        (mode, warns.load(Ordering::SeqCst))
    }

    // ── R1.2: 1/true → ON（分かち書きワードラップ）・warn なし ──

    #[test]
    fn one_resolves_to_budoux_word_wrap() {
        let (mode, warns) = resolve_counting_warns(Some("1"));
        assert_eq!(mode, WrapMode::BudouxWordWrap);
        assert_eq!(warns, 0, "`1` は warn なしで ON 受理される");
    }

    #[test]
    fn true_resolves_to_budoux_word_wrap() {
        let (mode, warns) = resolve_counting_warns(Some("true"));
        assert_eq!(mode, WrapMode::BudouxWordWrap);
        assert_eq!(warns, 0, "`true` は warn なしで ON 受理される");
    }

    // ── R1.3: 欠落/0/false → OFF（文字単位折返し・正常系・ログなし） ──

    #[test]
    fn missing_marker_defaults_to_char_by_char() {
        let (mode, warns) = resolve_counting_warns(None);
        assert_eq!(mode, WrapMode::CharByChar);
        // 未指定は正常系（既存ゴースト無害・R1.3/R1.5）——warn は記録しない。
        assert_eq!(warns, 0);
    }

    #[test]
    fn zero_resolves_to_char_by_char_without_warn() {
        let (mode, warns) = resolve_counting_warns(Some("0"));
        assert_eq!(mode, WrapMode::CharByChar);
        assert_eq!(warns, 0, "`0` は明示 OFF・正常系につき warn なし");
    }

    #[test]
    fn false_resolves_to_char_by_char_without_warn() {
        let (mode, warns) = resolve_counting_warns(Some("false"));
        assert_eq!(mode, WrapMode::CharByChar);
        assert_eq!(warns, 0, "`false` は明示 OFF・正常系につき warn なし");
    }

    // ── R1.4: 受理語彙以外 → warn ログ 1 回＋OFF フォールバック ──

    /// 完全一致のみ受理（trim は kv 層済み）——大文字形・別綴り・空白・空文字は未知値扱い。
    #[test]
    fn unknown_values_fall_back_to_char_by_char_with_single_warn() {
        for value in ["TRUE", "on", "2", " 1", ""] {
            let (mode, warns) = resolve_counting_warns(Some(value));
            assert_eq!(
                mode,
                WrapMode::CharByChar,
                "value {value:?} は OFF（文字単位折返し）へフォールバックする"
            );
            assert_eq!(warns, 1, "value {value:?} はちょうど 1 回 warn を記録する");
        }
    }

    // ── 型の Default・決定論 ──

    /// 型の Default も文字単位折返し（design.md の #[default] 正準と一致）。
    #[test]
    fn wrap_mode_default_is_char_by_char() {
        assert_eq!(WrapMode::default(), WrapMode::CharByChar);
    }

    /// 決定論: 全入力パターン（ON・OFF・欠落・未知値）で同一入力→同一出力（Invariants）。
    #[test]
    fn resolution_is_deterministic_across_all_patterns() {
        let cases: &[(Option<&str>, WrapMode, usize)] = &[
            (Some("1"), WrapMode::BudouxWordWrap, 0),
            (Some("true"), WrapMode::BudouxWordWrap, 0),
            (None, WrapMode::CharByChar, 0),
            (Some("0"), WrapMode::CharByChar, 0),
            (Some("false"), WrapMode::CharByChar, 0),
            (Some("on"), WrapMode::CharByChar, 1),
        ];
        for (input, expected_mode, expected_warns) in cases {
            // 同一入力を繰り返し評価しても結果が一致する（非決定論の排除）。
            let (m1, w1) = resolve_counting_warns(*input);
            let (m2, w2) = resolve_counting_warns(*input);
            assert_eq!(m1, m2, "input {input:?} の解決結果が非決定論的（mode）");
            assert_eq!(w1, w2, "input {input:?} の warn 件数が非決定論的");
            assert_eq!(m1, *expected_mode, "input {input:?} の解決結果が期待と不一致");
            assert_eq!(w1, *expected_warns, "input {input:?} の warn 件数が期待と不一致");
        }
    }

    // ── R1.1: 2層マージ後の転記値を読むだけ（マージ済み単一値で解決） ──

    /// 2層マージは balloon-parse 側で解決済み——本層は `parse` が転記した後勝ち単一値を
    /// 読むだけで、画像別上書きの結果がそのまま有効値になる。
    #[test]
    fn resolves_from_two_layer_merged_transcription() {
        use areka_parsers::balloon::parse;
        use std::collections::BTreeMap;

        let descript: BTreeMap<String, String> =
            [("budoux_newline".to_owned(), "0".to_owned())].into();
        let image: BTreeMap<String, String> =
            [("budoux_newline".to_owned(), "1".to_owned())].into();
        let merged = parse(&descript, Some(&image));
        assert_eq!(WrapMode::resolve(&merged), WrapMode::BudouxWordWrap);
    }
}
