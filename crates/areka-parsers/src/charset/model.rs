//! 既定エンコード指定モデル（下流共有 I/O 契約）。
//!
//! `decode` へ呼び出し側が渡す ANSI／UTF-8 の 2 値選択を表す公開型
//! `DefaultEncoding` を定義する。これがクロスエンジン契約の片側であり、
//! 本 spec が正本を所有する（`Ansi` の写像先コードページ等の意味変更は破壊的）。
//!
//! 設計規律（design.md「Service Interface」）:
//! - `#[non_exhaustive]` により variant 追加は後方互換。
//! - 最小派生（`Clone` / `Copy` / `Debug` / `PartialEq` / `Eq`）のみ。
//! - `Ansi→SHIFT_JIS` / `Utf8→UTF_8` の固定写像（D6）を `to_encoding` に持つ。
//!   写像は環境非依存の純粋 `match`（OS ロケール不読・panic/Result なし）で、
//!   `decode`（タスク 2.3）が消費するクレート内部専用（公開契約は enum 自体）。

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

impl DefaultEncoding {
    /// 既定指定を `encoding_rs` の静的 Encoding へ固定写像する（D6）。
    ///
    /// `Ansi → SHIFT_JIS`（CP932）／`Utf8 → UTF_8`。呼び出し側が渡した指定のみで
    /// 決まる環境非依存・決定的な純粋 `match`（OS ロケールを読まない・R2.3/R3.1/R3.2）。
    /// panic・`Result` を持たない（R2.7/R6.1）。`decode`（タスク 2.3）専用の
    /// クレート内部写像であり、公開契約は enum 自体（本メソッドは公開面に出さない）。
    // 非テストビルドでは唯一の消費者 `decode`（タスク 2.3）が未実装のため未使用に見える。
    // 2.3 実装後に消費されるため許容する（テストは本メソッドを検証済み）。
    #[allow(dead_code)]
    pub(crate) fn to_encoding(self) -> &'static encoding_rs::Encoding {
        match self {
            DefaultEncoding::Ansi => encoding_rs::SHIFT_JIS,
            DefaultEncoding::Utf8 => encoding_rs::UTF_8,
        }
    }
}
