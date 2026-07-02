//! 冒頭 ASCII プリスキャン（内部・非公開）。
//!
//! バイト列先頭を実エンコード非依存に走査し `charset,<name>` 宣言を抽出する
//! （D1/D2）。実装は後続タスク 2.2 で埋める。本ファイルはスケルトンであり、
//! 公開面（`mod.rs`）は本モジュールを `pub` 公開しない（内部専用）。

/// 冒頭プリスキャンで charset 宣言名を抽出する（後続タスク 2.2 で実装）。
///
/// スタブ段階では常に `None`（宣言なし相当）を返す。
#[allow(dead_code)]
pub(super) fn prescan_charset(_bytes: &[u8]) -> Option<String> {
    None
}
