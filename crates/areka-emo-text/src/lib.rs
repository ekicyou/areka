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
//! 1. **純粋層**（[`state`]／[`writing`]／[`region`]／[`layout`]／[`canvas`]）——
//!    `windows` 系 crate 非依存の決定論檻。純粋層モジュールに `windows` の import が
//!    現れたらレビューエラー（本 crate のテストでも構造検証する）。
//! 2. **COM 層**（[`draw`]／[`surface`]）——DirectWrite/D2D/DXGI/WUC を触る唯一の場所。
//!    UI スレッド専有。
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
pub mod draw;
pub mod layout;
pub mod region;
pub mod sink;
pub mod state;
pub mod surface;
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
            ("state.rs", include_str!("state.rs")),
            ("writing.rs", include_str!("writing.rs")),
            ("region.rs", include_str!("region.rs")),
            ("layout.rs", include_str!("layout.rs")),
            ("canvas.rs", include_str!("canvas.rs")),
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
