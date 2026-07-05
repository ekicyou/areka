//! 合成コアの構造化エラー型 `ComposeError`（thiserror）。
//!
//! 失敗は `error` ログ＋戻り値で表現し、`Err` は surface 不在・定義層皆無の退化データに
//! 限定する。全 element 全透明・空の有効 bind 集合などの非退化な空結果は失敗とせず、
//! surface 外形どおりの全透明 `ComposedSurface` を正常返却する（議題2裁定）。
//!
//! # 3 分類（design「Error Handling」表・議題2裁定・要件 6.6/10.5）
//!
//! 命令ゼロの導出（[`crate::plan::build_plan`]）は次の 3 状態を厳密に区別する:
//!
//! - **対象 surface 不在** → [`ComposeError::SurfaceNotFound`]（`error` ログ＋`Err`・要件 10.5）。
//! - **surface 存在＋描画可能命令ゼロ**（全 element 全透明・空の有効 bind 集合など）→ **正常系**。
//!   空の命令列と**非ゼロの外形**を `Ok` で返す（`Err` にしない・要件 6.6・議題2裁定）。
//! - **定義層が皆無で外形 0×0** の退化データのみ → [`ComposeError::EmptyComposition`]（`error`
//!   ログ＋`Err`・要件 10.5・議題2裁定: これが唯一の真の失敗となる退化ケース）。

/// 合成コア（plan 段）の構造化エラー。失敗経路はログ＋`Err` で表現する（パニックしない・要件 1.4/10.5）。
///
/// `#[non_exhaustive]` ゆえ将来 variant 追加でも呼び手の `match` は網羅性エラーにならない。
/// 「描画可能命令ゼロだが定義層あり」は本 enum のいずれの variant でもなく **正常系**（全透明
/// 返却）である点に注意（議題2裁定・要件 6.6）。
#[non_exhaustive]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ComposeError {
    /// 合成対象 surface が World に存在しない（要件 10.5）。命令/外形の算出前に返す。
    #[error("surface {0} not found")]
    SurfaceNotFound(u32),
    /// 定義層が皆無で外形 0×0 の退化データのみ（議題2裁定）。
    /// 「描画可能命令ゼロだが定義層あり」は正常系＝全透明返却であり本 variant を使わない。
    #[error("surface {0} has no layers at all (extent 0x0)")]
    EmptyComposition(u32),
}
