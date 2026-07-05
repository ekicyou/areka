//! 合成コアの構造化エラー型 `ComposeError`（thiserror）。
//!
//! 失敗は `error` ログ＋戻り値で表現し、`Err` は surface 不在・定義層皆無の退化データに
//! 限定する。全 element 全透明・空の有効 bind 集合などの非退化な空結果は失敗とせず、
//! surface 外形どおりの全透明 `ComposedSurface` を正常返却する（議題2裁定）。
