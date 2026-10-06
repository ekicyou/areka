//! 診断可能なエラー型（`BakeError` / `DecodeError`）と継続方針。
//!
//! 要件 **R2**。
//!
//! bake パイプラインの脱落エントリを診断可能に集約するエラー型を定義する。デコード
//! 失敗は当該エントリを索引表に載せず（`resolve` が `None`）記録して継続し（1 element の
//! 失敗が全体を止めない・R2.2）、`bake` は `BakeResult.errors: Vec<BakeError>` へ集約して
//! 返す。正規化は失敗しない（spec: areka-P0-self-alpha-declaration）。失敗は error! ログ ＋
//! Err 戻り値で扱い、安易な panic を避ける（記憶 areka-log-first-no-silent-failure）。
//!
//! `DecodeError` の正本は [`crate::decode`]（パス保持・不在/破損）で、ここでは
//! 集約型 `BakeError` の一腕として参照するのみ（再定義・移設はしない）。

use crate::decode::DecodeError;

/// bake パイプラインで 1 エントリが脱落した原因（デコード）。
///
/// 当該エントリを索引表から除外し（`resolve` → `None`）、他エントリの処理は
/// 継続する（R2.2）。どのエントリで何が起きたかを診断可能に保持する。
#[derive(Debug)]
pub enum BakeError {
    /// デコード失敗（不在・破損）。`DecodeError` が失敗パスを保持する（R2.2）。
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

impl From<DecodeError> for BakeError {
    fn from(e: DecodeError) -> Self {
        BakeError::Decode(e)
    }
}
