//! # writing — writing_mode 宣言の解釈（純粋層）
//!
//! balloon descript の `writing_mode` 転記値（2層マージ後勝ち解決済み・生文字列）を
//! `WritingMode`（`HorizontalTb`／`VerticalRl`／`VerticalLr`）へ解決し、
//! 方向写像と M2 予約キー名（`text_orientation`／`text_combine_upright`）の記録を担う。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。
//!
//! ## M2 予約キー（記録のみ・実装しない・R5.7）
//!
//! CSS 借用の snake_case 予約キー名を定数として記録するに留め、M1 では実挙動を
//! 一切実装しない:
//!
//! - [`RESERVED_KEY_TEXT_ORIENTATION`]（`text_orientation`・欧文の向き）
//! - [`RESERVED_KEY_TEXT_COMBINE_UPRIGHT`]（`text_combine_upright`・縦中横）

use areka_parsers::balloon::BalloonModel;

/// `writing_mode` の CSS 語彙 `horizontal_tb` に対応する受理値（R5.1）。
const VALUE_HORIZONTAL_TB: &str = "horizontal_tb";
/// `writing_mode` の CSS 語彙 `vertical_rl` に対応する受理値（R5.1）。
const VALUE_VERTICAL_RL: &str = "vertical_rl";
/// `writing_mode` の CSS 語彙 `vertical_lr` に対応する受理値（R5.1）。
const VALUE_VERTICAL_LR: &str = "vertical_lr";

/// M2 予約キー: 欧文の向き `text_orientation`（記録のみ・実装しない・R5.7）。
pub const RESERVED_KEY_TEXT_ORIENTATION: &str = "text_orientation";
/// M2 予約キー: 縦中横 `text_combine_upright`（記録のみ・実装しない・R5.7）。
pub const RESERVED_KEY_TEXT_COMBINE_UPRIGHT: &str = "text_combine_upright";

/// `writing_mode` 宣言の解決結果（3 語彙→3 方向の 1:1 写像・R5.1/R5.5）。
///
/// 語彙（snake_case・CSS `writing-mode` 借用）と方向の対応は 1:1:
///
/// | 語彙 | variant | 方向（行内／行送り） |
/// |---|---|---|
/// | `horizontal_tb` | [`HorizontalTb`](Self::HorizontalTb) | 左→右／上→下 |
/// | `vertical_rl` | [`VerticalRl`](Self::VerticalRl) | 上→下／右→左 |
/// | `vertical_lr` | [`VerticalLr`](Self::VerticalLr) | 上→下／左→右 |
///
/// 軸読み替え（折返し軸・スクロール軸・書字開始角）の適用は layout 層の領分
/// （design.md「軸読み替え正準表」）。DirectWrite への設定写像
/// （ReadingDirection／FlowDirection）は COM 層 draw.rs が本型を消費して行う。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WritingMode {
    /// 横書き（行内 左→右・行送り 上→下）。SSP 互換の既定（マーカー無し・R5.3）。
    #[default]
    HorizontalTb,
    /// 日本語縦書き（行内 上→下・行送り 右→左）。
    VerticalRl,
    /// 縦書き左送り（行内 上→下・行送り 左→右）。
    VerticalLr,
}

impl WritingMode {
    /// `BalloonModel` の転記値（2層マージ後勝ち解決済み・R5.2）から有効値を解釈する。
    ///
    /// - マーカー無し（`None`）→ [`HorizontalTb`](Self::HorizontalTb)（SSP 互換既定・
    ///   正常系につきログなし・R5.3）。
    /// - 未知の値 → `warn!`（値を含む）＋ [`HorizontalTb`](Self::HorizontalTb) へ
    ///   フォールバック（縮退継続・R5.4）。
    /// - 語彙は snake_case 完全一致（trim は parser の kv 層で済んでいる前提・R5.1）。
    pub fn resolve(model: &BalloonModel) -> WritingMode {
        match model.writing_mode() {
            None => WritingMode::HorizontalTb,
            Some(VALUE_HORIZONTAL_TB) => WritingMode::HorizontalTb,
            Some(VALUE_VERTICAL_RL) => WritingMode::VerticalRl,
            Some(VALUE_VERTICAL_LR) => WritingMode::VerticalLr,
            Some(unknown) => {
                tracing::warn!(
                    value = unknown,
                    "未知の writing_mode 値のため horizontal_tb へフォールバックする（受理語彙: horizontal_tb / vertical_rl / vertical_lr）"
                );
                WritingMode::HorizontalTb
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

    use super::{RESERVED_KEY_TEXT_COMBINE_UPRIGHT, RESERVED_KEY_TEXT_ORIENTATION, WritingMode};

    /// テスト用 BalloonModel 生成ヘルパ（writing_mode 以外は全成分未指定）。
    fn model(writing_mode: Option<&str>) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(None, None),
            WordWrapPoint::new(None, None),
            ValidRect::new(None, None, None, None),
            Font::new(None, None, FontColor::new(None, None, None)),
            writing_mode.map(str::to_owned),
            None,
        )
    }

    /// WARN イベント数を数える最小 Subscriber（決定論的なログ檻・state.rs の檻パターン踏襲）。
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
    fn resolve_counting_warns(writing_mode: Option<&str>) -> (WritingMode, usize) {
        let warns = Arc::new(AtomicUsize::new(0));
        let subscriber = WarnCounter {
            warns: Arc::clone(&warns),
        };
        let mode = tracing::subscriber::with_default(subscriber, || {
            WritingMode::resolve(&model(writing_mode))
        });
        (mode, warns.load(Ordering::SeqCst))
    }

    // ── R5.1/R5.5: 3 語彙の受理と方向写像 1:1（CSS 語彙 snake_case→3 variant 単射） ──

    #[test]
    fn horizontal_tb_resolves_to_horizontal_tb() {
        assert_eq!(
            WritingMode::resolve(&model(Some("horizontal_tb"))),
            WritingMode::HorizontalTb
        );
    }

    #[test]
    fn vertical_rl_resolves_to_vertical_rl() {
        assert_eq!(
            WritingMode::resolve(&model(Some("vertical_rl"))),
            WritingMode::VerticalRl
        );
    }

    #[test]
    fn vertical_lr_resolves_to_vertical_lr() {
        assert_eq!(
            WritingMode::resolve(&model(Some("vertical_lr"))),
            WritingMode::VerticalLr
        );
    }

    /// 方向写像は 1:1（R5.5）——3 語彙は互いに異なる variant へ写る（単射）。
    #[test]
    fn three_vocabulary_values_map_injectively() {
        let modes = [
            WritingMode::resolve(&model(Some("horizontal_tb"))),
            WritingMode::resolve(&model(Some("vertical_rl"))),
            WritingMode::resolve(&model(Some("vertical_lr"))),
        ];
        assert_ne!(modes[0], modes[1]);
        assert_ne!(modes[0], modes[2]);
        assert_ne!(modes[1], modes[2]);
    }

    /// 既知 3 語彙の受理は警告を出さない（warn は未知値専用・R5.1/R5.4）。
    #[test]
    fn known_values_resolve_without_warning() {
        for value in ["horizontal_tb", "vertical_rl", "vertical_lr"] {
            let (_, warns) = resolve_counting_warns(Some(value));
            assert_eq!(warns, 0, "value {value:?} は warn なしで受理される");
        }
    }

    // ── R5.3: マーカー無し→横書き既定（SSP 互換） ──

    #[test]
    fn missing_marker_defaults_to_horizontal_tb() {
        let (mode, warns) = resolve_counting_warns(None);
        assert_eq!(mode, WritingMode::HorizontalTb);
        // 未指定は正常系（SSP 互換既定）——warn は記録しない。
        assert_eq!(warns, 0);
    }

    /// 型の Default も横書き（design.md の #[default] 正準と一致）。
    #[test]
    fn writing_mode_default_is_horizontal_tb() {
        assert_eq!(WritingMode::default(), WritingMode::HorizontalTb);
    }

    // ── R5.4: 未知値→warn ログ＋横書きフォールバック ──

    #[test]
    fn unknown_value_falls_back_to_horizontal_tb_with_warn() {
        // parsers 側の未知値素通しテストと同じ値（diagonal_bt）で連続性を持たせる。
        let (mode, warns) = resolve_counting_warns(Some("diagonal_bt"));
        assert_eq!(mode, WritingMode::HorizontalTb);
        assert_eq!(warns, 1, "未知値はちょうど 1 回 warn を記録する");
    }

    /// 語彙は snake_case 完全一致（R5.1）——ハイフン形（CSS 原表記）や大文字・空白は未知値扱い。
    #[test]
    fn near_miss_values_are_unknown_and_fall_back() {
        for value in ["vertical-rl", "VERTICAL_RL", "vertical_rl ", ""] {
            let (mode, warns) = resolve_counting_warns(Some(value));
            assert_eq!(
                mode,
                WritingMode::HorizontalTb,
                "value {value:?} は横書きへフォールバックする"
            );
            assert_eq!(warns, 1, "value {value:?} は warn を記録する");
        }
    }

    // ── R5.2: 2層マージ後の転記値を読むだけ（マージ済み単一値で解決） ──

    /// 2層マージは balloon-parse 側で解決済み——本層は `parse` が転記した後勝ち単一値を
    /// 読むだけで、画像別上書きの結果がそのまま有効値になる。
    #[test]
    fn resolves_from_two_layer_merged_transcription() {
        use areka_parsers::balloon::parse;
        use std::collections::BTreeMap;

        let descript: BTreeMap<String, String> =
            [("writing_mode".to_owned(), "horizontal_tb".to_owned())].into();
        let image: BTreeMap<String, String> =
            [("writing_mode".to_owned(), "vertical_rl".to_owned())].into();
        let merged = parse(&descript, Some(&image));
        assert_eq!(WritingMode::resolve(&merged), WritingMode::VerticalRl);
    }

    // ── R5.7: M2 予約キーは名前の記録のみ（実挙動なし） ──

    #[test]
    fn m2_reserved_key_names_are_recorded_as_constants() {
        assert_eq!(RESERVED_KEY_TEXT_ORIENTATION, "text_orientation");
        assert_eq!(RESERVED_KEY_TEXT_COMBINE_UPRIGHT, "text_combine_upright");
    }
}
