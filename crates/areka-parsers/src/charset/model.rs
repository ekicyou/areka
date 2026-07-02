//! 既定エンコード指定モデル（下流共有 I/O 契約）。
//!
//! `decode` へ呼び出し側が渡す ANSI／UTF-8 の 2 値選択を表す公開型
//! `DefaultEncoding` を定義する。これがクロスエンジン契約の片側であり、
//! 本 spec が正本を所有する（`Ansi` の写像先コードページ等の意味変更は破壊的）。
//!
//! 設計規律（design.md「Service Interface」）:
//! - `#[non_exhaustive]` により variant 追加は後方互換。
//! - 最小派生（`Clone` / `Copy` / `Debug` / `PartialEq` / `Eq`）のみ。
//! - `Ansi→SHIFT_JIS` / `Utf8→UTF_8` の写像は `decode` 側の責務（D6）であり、
//!   本モジュールは enum 定義のみを持つ。

/// 既定エンコード指定（呼び出し側が ANSI / UTF-8 を選択）。
/// SHIORI/4 ゴーストはエンジンが `Utf8` を渡す。環境非依存（R2.3/R3.1）。
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefaultEncoding {
    /// 旧環境互換の ANSI。areka では CP932（Shift_JIS）へ固定写像する（D6）。
    Ansi,
    /// UTF-8。SHIORI/4 の既定。
    Utf8,
}
