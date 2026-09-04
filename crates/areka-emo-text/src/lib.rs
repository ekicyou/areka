//! # areka-emo-text — emo ⑥ render engine テキスト層
//!
//! **crate↔spec 名マッピング**: 本 crate `areka-emo-text` の spec/feature 名は
//! `areka-P0-emo-text-layer`（emo トラック第4の独立 crate・atlas/compose/present の
//! 単一トークン命名に倣う）。
//!
//! sakura が発火する Balloon 向け `TalkCue`（Text/NewLine/Clear）を受け、emo-present が
//! 予約したバルーン上のスロット（`emo-text-layer-slot`）に文字を実際に描く層である。
//!
//! ## 層規律（一方向・レビューエラー基準）
//!
//! crate 内は次の一方向に層を分ける。逆流はレビューエラーとして扱う。
//!
//! 1. **純粋層**（[`state`]／[`writing`]／[`region`]／[`cursor_tag`]／[`layout`]／[`canvas`]／
//!    [`viewbox`]）——
//!    `windows` 系 crate 非依存の決定論檻。純粋層モジュールに `windows` の import が
//!    現れたらレビューエラー（本 crate のテストでも構造検証する）。
//! 2. **COM 層**（[`draw`]／[`surface`]／[`viewbox_draw`]）——DirectWrite/D2D/DXGI/WUC を
//!    触る唯一の場所。UI スレッド専有。
//! 3. **結線層**（[`sink`]／[`actor`]）——sakura からの cue 受信と UI 配送・フレーム提示。
//!
//! ## 依存方向（強制）
//!
//! `areka-parsers / areka-sakura / areka-actor → areka-emo-text ← wintf`、
//! `areka-emo-atlas → areka-emo-compose → areka-emo-present → areka-emo-text`。
//! 逆方向 import（emo-present → emo-text 等）は実装・レビューでエラーとして扱う。
//!
//! ## 失敗経路のログ規律（log-first）
//!
//! 失敗は `tracing::error!`（真因文脈付き）＋ [`TextLayerError`] の `Err` 戻り値で扱う。
//! panic は用いない。縮退可能な失敗は `warn!`＋縮退継続。

pub mod actor;
pub mod canvas;
pub mod choice;
pub mod cursor_tag;
pub mod draw;
pub mod layout;
pub mod region;
pub mod segment;
pub mod sink;
pub mod state;
pub mod surface;
pub mod viewbox;
pub mod viewbox_draw;
pub mod wrap;
pub mod writing;

/// 本 crate の共通エラー型（design.md「Error Handling」正本）。
///
/// log-first 規律: 失敗は `tracing::error!`（真因文脈付き）＋本型の `Err` 戻り値で扱う。
/// panic は用いない。縮退可能な失敗は `warn!`＋縮退継続。
#[derive(Debug, thiserror::Error)]
pub enum TextLayerError {
    /// D2D/DXGI/DWrite などデバイス呼び出しの失敗
    /// （`error!`＋`Err`・当該フレームの提示を skip し次フレーム再試行）。
    #[error("device call failed: {context} (hresult={hresult:#x})")]
    Device {
        /// 失敗した COM 呼び出しの HRESULT。
        hresult: i32,
        /// 失敗箇所の文脈（呼び出し名など）。
        context: &'static str,
    },
    /// テキストスロット未接続——actor の binding 未解決
    /// （`debug!`・状態は蓄積継続・次フレーム再試行）。
    #[error("text slot not attached: {actor}")]
    SlotNotAttached {
        /// binding 未解決の actor（`ActorKey` の表示形）。
        actor: String,
    },
}

/// budouy 依存導入の spike（task 1・Req 1.2/8.1）。
///
/// budouy の**実 API 実形**をコンパイラで確定し、以降の `segment.rs` 本実装
/// （task 3.2）と Parser キャッシュ機構（task 5）の判断根拠を回帰檻として固定する。
/// 代表和文の実境界ピン（決定論・無損失）も兼ねる。
///
/// spike で確定した事実:
/// - ローダ: `budouy::model::load_default_japanese_parser() -> budouy::Parser`（**infallible**・
///   Result ではない・`vendored-models` 同梱データのロードのみ＝ネットワーク/ファイル I/O なし）。
/// - セグメント呼び出し: `Parser::parse(&self, &str) -> Vec<String>`（**所有** String の
///   チャンク列。docs.rs 表示および design.md の `Vec<&str>` 想定は誤り——実 API は owned
///   String を返す。task 3.2 の glyph-index 写像はこの owned チャンクで `chars().count()`）。
/// - `budouy::Parser` は `Sync + Send`（下の `assert_sync`/`assert_send` がコンパイルで証明）
///   → キャッシュ機構は `static PARSER: OnceLock<budouy::Parser>` を採用可能
///   （`thread_local!` への退避は不要）。
#[cfg(test)]
mod budouy_spike {
    /// コンパイル時 trait 証明: `Sync` なら `OnceLock<budouy::Parser>` static が組める。
    fn assert_sync<T: Sync>() {}
    /// コンパイル時 trait 証明: `Send`（アクター跨ぎ／将来の move の余地）。
    fn assert_send<T: Send>() {}

    /// budouy Parser は Sync + Send（→ `OnceLock` キャッシュ採用の根拠・task 5）。
    #[test]
    fn parser_is_sync_and_send() {
        assert_sync::<budouy::Parser>();
        assert_send::<budouy::Parser>();
    }

    /// vendored-models の既定日本語 parser が I/O なしでロードでき、
    /// 代表和文を決定論的・無損失に分かち書きする（Req 8.1: 決定論・オフライン）。
    #[test]
    fn default_japanese_parser_segments_deterministically_and_losslessly() {
        // infallible ロード（Result ではない・同梱データのロードのみ）。
        let parser = budouy::model::load_default_japanese_parser();

        let sentence = "今日はいい天気ですね";

        // parse -> Vec<String>（**所有** String のチャンク列。docs.rs/design 想定の
        // `Vec<&str>` は誤りで、実 API は owned String を返す——task 3.2 の glyph-index
        // 写像はこの owned チャンクを `chars().count()` で数える）。
        let first: Vec<String> = parser.parse(sentence);
        let second: Vec<String> = parser.parse(sentence);

        // 決定論: 同一入力 → 同一境界（8.1・オフライン CI 整合）。
        assert_eq!(first, second, "同一入力に対し分かち書き結果が非決定論的");

        // 無損失: チャンクの連結は原文字列と一致（segment.rs の写像不変条件の前提）。
        let joined: String = first.concat();
        assert_eq!(joined, sentence, "チャンク連結が原文と不一致（損失あり）");

        // 実際に 2 つ以上の塊へ分かれる代表文であることをピン（Req 1.2: 境界がセグメンテーションへ供給される）。
        assert!(
            first.len() >= 2,
            "代表和文が単一塊に潰れている（分かち書きが機能していない）: {first:?}"
        );

        // 各チャンクは非空（空 run/空塊を生まない前提）。
        assert!(
            first.iter().all(|c| !c.is_empty()),
            "空チャンクが含まれる: {first:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::TextLayerError;

    /// コンパイル時 trait 検証: 共通エラー型は std::error::Error＋Send＋Sync。
    fn assert_error_traits<E: std::error::Error + Send + Sync + 'static>() {}

    #[test]
    fn text_layer_error_is_std_error_send_sync() {
        assert_error_traits::<TextLayerError>();
    }

    #[test]
    fn device_error_displays_context_and_hresult() {
        let err = TextLayerError::Device {
            hresult: 0x8007000Eu32 as i32, // E_OUTOFMEMORY
            context: "CreateSwapChainForComposition",
        };
        assert_eq!(
            err.to_string(),
            "device call failed: CreateSwapChainForComposition (hresult=0x8007000e)"
        );
    }

    #[test]
    fn slot_not_attached_error_displays_actor() {
        let err = TextLayerError::SlotNotAttached {
            actor: "0".to_string(),
        };
        assert_eq!(err.to_string(), "text slot not attached: 0");
    }

    /// 層規律の構造檻: 純粋層モジュール（state/writing/region/layout/canvas）の
    /// ソースに `windows` 系 crate への依存（import／パス参照）が一切無いことを検証する。
    /// （design.md「依存方向（強制）」——純粋層に `windows` の import が現れたらレビューエラー）
    #[test]
    fn pure_layer_modules_have_no_windows_imports() {
        const PURE_SOURCES: &[(&str, &str)] = &[
            ("choice.rs", include_str!("choice.rs")),
            ("cursor_tag.rs", include_str!("cursor_tag.rs")),
            (
                "cursor_tag_resolve_tests.rs",
                include_str!("cursor_tag_resolve_tests.rs"),
            ),
            (
                "cursor_tag_test_support.rs",
                include_str!("cursor_tag_test_support.rs"),
            ),
            ("cursor_tag_tests.rs", include_str!("cursor_tag_tests.rs")),
            ("state.rs", include_str!("state.rs")),
            (
                "state_cursor_coord_parse_tests.rs",
                include_str!("state_cursor_coord_parse_tests.rs"),
            ),
            ("writing.rs", include_str!("writing.rs")),
            (
                "writing_decision_tests.rs",
                include_str!("writing_decision_tests.rs"),
            ),
            ("region.rs", include_str!("region.rs")),
            (
                "region_vertical_canon_tests.rs",
                include_str!("region_vertical_canon_tests.rs"),
            ),
            ("segment.rs", include_str!("segment.rs")),
            ("layout.rs", include_str!("layout.rs")),
            (
                "layout_cursor_order_tests.rs",
                include_str!("layout_cursor_order_tests.rs"),
            ),
            (
                "layout_cursor_tests.rs",
                include_str!("layout_cursor_tests.rs"),
            ),
            (
                "layout_cursor_vertical_tests.rs",
                include_str!("layout_cursor_vertical_tests.rs"),
            ),
            ("canvas.rs", include_str!("canvas.rs")),
            ("viewbox.rs", include_str!("viewbox.rs")),
            ("wrap.rs", include_str!("wrap.rs")),
        ];
        const FORBIDDEN: &[&str] = &[
            "use windows",
            "windows::",
            "windows_core",
            "windows_numerics",
            "extern crate windows",
        ];
        for (name, src) in PURE_SOURCES {
            for pat in FORBIDDEN {
                assert!(
                    !src.contains(pat),
                    "純粋層モジュール {name} に windows 依存が混入している（パターン: {pat}）"
                );
            }
        }
    }
}
