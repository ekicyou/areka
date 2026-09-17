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
//!    [`viewbox`]／[`look`]／[`color`]）——
//!    `windows` 系 crate 非依存の決定論檻。純粋層モジュールに `windows` の import が
//!    現れたらレビューエラー（本 crate のテストでも構造検証する）。
//! 2. **COM 層**（[`draw`]／[`surface`]／[`viewbox_draw`]）——DirectWrite/D2D/DXGI/WUC を
//!    触る唯一の場所。UI スレッド専有。
//! 3. **結線層**（[`sink`]／[`actor`]）——sakura からの cue 受信と UI 配送・フレーム提示。
//!
//! 上の 3 つはいずれも**親ファイルの名前**であって、走査面ではない。親は `#[path]` で
//! 子モジュールを抱えており（`draw` は `draw_metrics`／`draw_line_store`／`draw_catalog`、
//! `layout` は `layout_line_ops`／`layout_styled`、`state` は `state_decoration`、
//! `viewbox_draw` は `viewbox_draw_plan`／`viewbox_draw_decoration`、`actor` は
//! `actor_decoration`）、子は親と同じ層に属する。**層規律を実際に見張る走査面は
//! `PURE_SOURCES` と `SOURCES_OUTSIDE_THE_PURE_SCAN` の 2 つの一覧**（本ファイル末尾の
//! `#[cfg(test)] mod layer_discipline` 内）で、その和が `src/*.rs` の実ファイル集合と
//! 一致することを `every_source_file_is_either_scanned_or_explicitly_excluded` が突き合わせる
//! （この段落は列挙を数えるためのものではない——数える場所は 2 つの一覧の側にある）。
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
pub mod color;
pub mod cursor_tag;
pub mod draw;
pub mod layout;
pub mod look;
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

    /// 純粋層モジュールの走査面。[`pure_layer_modules_have_no_windows_imports`] と
    /// [`every_source_file_is_either_scanned_or_explicitly_excluded`] が共有する。
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
        (
            "region_inline_limit_tests.rs",
            include_str!("region_inline_limit_tests.rs"),
        ),
        ("segment.rs", include_str!("segment.rs")),
        ("layout.rs", include_str!("layout.rs")),
        (
            "layout_cursor_center_origin_tests.rs",
            include_str!("layout_cursor_center_origin_tests.rs"),
        ),
        (
            "layout_cursor_order_tests.rs",
            include_str!("layout_cursor_order_tests.rs"),
        ),
        (
            "layout_cursor_overflow_tests.rs",
            include_str!("layout_cursor_overflow_tests.rs"),
        ),
        (
            "layout_cursor_tests.rs",
            include_str!("layout_cursor_tests.rs"),
        ),
        (
            "layout_cursor_vertical_canon_tests.rs",
            include_str!("layout_cursor_vertical_canon_tests.rs"),
        ),
        (
            "layout_cursor_vertical_tests.rs",
            include_str!("layout_cursor_vertical_tests.rs"),
        ),
        (
            "layout_cursor_wiring_tests.rs",
            include_str!("layout_cursor_wiring_tests.rs"),
        ),
        (
            "layout_hard_limit_tests.rs",
            include_str!("layout_hard_limit_tests.rs"),
        ),
        ("canvas.rs", include_str!("canvas.rs")),
        ("viewbox.rs", include_str!("viewbox.rs")),
        ("wrap.rs", include_str!("wrap.rs")),
        // areka-P0-text-decoration-canon が新設した純粋モジュール 14 本
        // （`draw_metrics.rs`／`draw_line_store.rs` は COM 層なので載せない）。
        ("color.rs", include_str!("color.rs")),
        ("color_tests.rs", include_str!("color_tests.rs")),
        ("look.rs", include_str!("look.rs")),
        ("look_tests.rs", include_str!("look_tests.rs")),
        (
            "look_font_tag_tests.rs",
            include_str!("look_font_tag_tests.rs"),
        ),
        (
            "look_font_tag_value_tests.rs",
            include_str!("look_font_tag_value_tests.rs"),
        ),
        ("state_decoration.rs", include_str!("state_decoration.rs")),
        (
            "state_decoration_tests.rs",
            include_str!("state_decoration_tests.rs"),
        ),
        (
            "state_decoration_reset_tests.rs",
            include_str!("state_decoration_reset_tests.rs"),
        ),
        ("layout_line_ops.rs", include_str!("layout_line_ops.rs")),
        ("layout_styled.rs", include_str!("layout_styled.rs")),
        (
            "layout_styled_tests.rs",
            include_str!("layout_styled_tests.rs"),
        ),
        ("viewbox_draw_plan.rs", include_str!("viewbox_draw_plan.rs")),
        (
            "viewbox_style_fingerprint_tests.rs",
            include_str!("viewbox_style_fingerprint_tests.rs"),
        ),
        (
            "viewbox_axis_tests.rs",
            include_str!("viewbox_axis_tests.rs"),
        ),
        (
            "viewbox_choice_marker_tests.rs",
            include_str!("viewbox_choice_marker_tests.rs"),
        ),
        (
            "viewbox_dirty_tests.rs",
            include_str!("viewbox_dirty_tests.rs"),
        ),
        (
            "viewbox_plan_commit_tests.rs",
            include_str!("viewbox_plan_commit_tests.rs"),
        ),
        (
            "viewbox_test_support.rs",
            include_str!("viewbox_test_support.rs"),
        ),
        // 純粋層モジュールの兄弟テスト／支援で歴史的に載っていなかったもの（タスク 9.4 で追加）。
        // いずれも `windows` 参照 0 件を実測してから移した——除外一覧に置いておく理由が無い。
        (
            "layout_segmented_tests.rs",
            include_str!("layout_segmented_tests.rs"),
        ),
        (
            "layout_test_support.rs",
            include_str!("layout_test_support.rs"),
        ),
        (
            "layout_visible_window_tests.rs",
            include_str!("layout_visible_window_tests.rs"),
        ),
        ("layout_wrap_tests.rs", include_str!("layout_wrap_tests.rs")),
        ("choice_tests.rs", include_str!("choice_tests.rs")),
        (
            "choice_style_resolve_tests.rs",
            include_str!("choice_style_resolve_tests.rs"),
        ),
        (
            "choice_decorate_tests.rs",
            include_str!("choice_decorate_tests.rs"),
        ),
        (
            "state_cue_apply_tests.rs",
            include_str!("state_cue_apply_tests.rs"),
        ),
        (
            "state_reveal_tests.rs",
            include_str!("state_reveal_tests.rs"),
        ),
        (
            "state_test_support.rs",
            include_str!("state_test_support.rs"),
        ),
    ];
    /// 純粋層の走査面へ**載せない**ファイルの明示（タスク 9.4）。
    ///
    /// ここに載るのは COM 層・結線層のモジュールとその兄弟テスト／支援である。
    /// ここへ置くことは「純粋層でない」の宣言ではなく「この検査では走査しない」の明示で、
    /// この一覧は新設ファイルが黙って走査面から外れることを防ぐためにある
    /// （[`every_source_file_is_either_scanned_or_explicitly_excluded`] が和を実ファイル
    /// 集合と突き合わせるので、どちらかへの明示的な編集が必ず要る）。
    const SOURCES_OUTSIDE_THE_PURE_SCAN: &[&str] = &[
        "actor.rs",
        "actor_choice_contract_tests.rs",
        "actor_clear_atomicity_tests.rs",
        "actor_decoration.rs",
        "actor_decoration_frame_tests.rs",
        "actor_decoration_tests.rs",
        "actor_region_warn_tests.rs",
        "actor_runtime_frame_tests.rs",
        "actor_scale_refresh_tests.rs",
        "actor_scroll_retain_tests.rs",
        "actor_test_support.rs",
        "actor_tests.rs",
        "draw.rs",
        "draw_catalog.rs",
        "draw_catalog_tests.rs",
        "draw_format_metrics_tests.rs",
        "draw_line_store.rs",
        "draw_line_store_tests.rs",
        "draw_metrics.rs",
        "draw_metrics_styled_tests.rs",
        "draw_oracle_tests.rs",
        "draw_test_support.rs",
        "sink.rs",
        "surface.rs",
        "viewbox_draw.rs",
        "viewbox_draw_choice_hover_tests.rs",
        "viewbox_draw_decoration.rs",
        "viewbox_draw_decoration_tests.rs",
        "viewbox_draw_frame_render_tests.rs",
        "viewbox_draw_live_diff_tests.rs",
        "viewbox_draw_oracle_regression_tests.rs",
        "viewbox_draw_png_dump_tests.rs",
        "viewbox_draw_scroll_retain_tests.rs",
        "viewbox_draw_test_support.rs",
    ];

    /// 層規律の構造檻: 純粋層モジュール（state/writing/region/layout/canvas/look/color）の
    /// ソースに `windows` 系 crate への依存（import／パス参照）が一切無いことを検証する。
    /// （design.md「依存方向（強制）」——純粋層に `windows` の import が現れたらレビューエラー）
    #[test]
    fn pure_layer_modules_have_no_windows_imports() {
        // 列挙は静的なので、走査面が痩せても述語そのものは緑のままになる。
        // 母数を先に固定して「黙って減る」経路を塞ぐ（増やすときは 2 箇所を明示的に編集する）。
        assert_eq!(PURE_SOURCES.len(), 54, "走査する純粋層モジュールの母数");
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
    /// 手保守の静的な列挙が「新設したのに載せない」を塞げない穴を閉じる（タスク 9.4）。
    ///
    /// [`PURE_SOURCES`] も [`SOURCES_OUTSIDE_THE_PURE_SCAN`] も人が書く一覧なので、
    /// 新しい兄弟ファイルを足しただけでは走査面が広がらず、層規律の字面検査は黙って
    /// 素通りする（`PURE_SOURCES.len()` の母数固定は「黙って減る」しか塞げない）。
    /// `src/*.rs` の**実ファイル集合**を実行時に読み、2 つの一覧の和と突き合わせる。
    #[test]
    fn every_source_file_is_either_scanned_or_explicitly_excluded() {
        use std::collections::BTreeSet;

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let actual: BTreeSet<String> = std::fs::read_dir(&dir)
            .expect("src ディレクトリが読めない")
            .map(|entry| {
                entry
                    .expect("src の項目が読めない")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.ends_with(".rs") && name != "lib.rs")
            .collect();
        // 空振り防止: 実ファイルが 0 件だと以下の包含判定はどちらも恒真になる。
        assert!(
            actual.len() > 50,
            "src/*.rs の実ファイルが {} 件しか読めていない——検査が空振りしている",
            actual.len()
        );

        let scanned: BTreeSet<&str> = PURE_SOURCES.iter().map(|(name, _)| *name).collect();
        let excluded: BTreeSet<&str> = SOURCES_OUTSIDE_THE_PURE_SCAN.iter().copied().collect();
        let both: Vec<&str> = scanned.intersection(&excluded).copied().collect();
        assert!(
            both.is_empty(),
            "同じファイルを走査面と除外の両方に置いている: {both:?}"
        );

        let listed: BTreeSet<&str> = scanned.union(&excluded).copied().collect();
        let unlisted: Vec<&str> = actual
            .iter()
            .map(String::as_str)
            .filter(|name| !listed.contains(name))
            .collect();
        assert!(
            unlisted.is_empty(),
            "src/ に在るのにどちらの一覧にも載っていない: {unlisted:?}——\
             純粋層なら PURE_SOURCES へ、そうでなければ SOURCES_OUTSIDE_THE_PURE_SCAN へ足す"
        );
        let vanished: Vec<&str> = listed
            .iter()
            .copied()
            .filter(|name| !actual.contains(*name))
            .collect();
        assert!(
            vanished.is_empty(),
            "一覧に在るのに src/ から消えている: {vanished:?}"
        );
    }
}
