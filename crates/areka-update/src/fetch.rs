//! 取得の境界（要件 3.1）。

use crate::error::FetchError;

/// 外界へ出る唯一の口（3.1）。定義ファイルも各ファイルもここから取る。
///
/// 本物の実装は WinHTTP の 1 つ、偽の実装は `testkit::FakeFetch`（テスト側）の 1 つ。
pub trait Fetch {
    /// `url` の本文をバイト列で返す。「無い」（404）は [`FetchError::NotFound`] で区別する。
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError>;
}
