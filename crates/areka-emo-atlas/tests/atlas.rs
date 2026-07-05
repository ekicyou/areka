//! 統合テスト入口（束ね役）。
//!
//! structure.md のテスト慣行に従い、`tests/` 配下は本ファイルを唯一の入口とし、
//! ドメイン別テストモジュールを `#[path]` 宣言で束ねる。
//!
//! 主軸は各モジュール in-source の `#[cfg(test)]`。正規化以降（normalize/trim/pack/
//! table）はメモリ PBGRA 入力で COM init 不要の純粋テスト、WIC 腕テストのみ
//! `CoInitializeEx` を要する。
//!
//! （本タスクは雛形。テストモジュールは後続タスクで追加する。）
