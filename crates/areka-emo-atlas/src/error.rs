//! 診断可能なエラー型（`BakeError` / `DecodeError`）と継続方針。
//!
//! デコード失敗は当該エントリを索引表に載せず（`resolve` が `None`）継続し（1 element の
//! 失敗が全体を止めない）、`bake` は `BakeResult.errors: Vec<BakeError>` へ集めて返す。
//! 読めなかった絵の記録は呼び手が出す。
//!
//! `DecodeError` の正本は [`crate::decode`] で、ここでは `BakeError` の一腕として参照するのみ。

use crate::decode::DecodeError;

/// bake パイプラインで 1 エントリが脱落した原因（デコード）。
///
/// 当該エントリは索引表から除外され（`resolve` → `None`）、他エントリの処理は継続する。
#[derive(Debug)]
pub enum BakeError {
    /// デコード失敗（不在・破損）。`DecodeError` が失敗パスを保持する。
    Decode(DecodeError),
}

impl std::fmt::Display for BakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BakeError::Decode(e) => write!(f, "bake: decode failed: {e}"),
        }
    }
}

impl std::error::Error for BakeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BakeError::Decode(e) => Some(e),
        }
    }
}
