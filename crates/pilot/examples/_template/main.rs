//! 先進坑テンプレ example（雛形）。
//!
//! このフォルダ（`examples/_template/`）は新しい先進坑を即座に着手するための雛形である。
//! `_` 前置で実 spec フォルダ（`examples/<spec-name>/`）と区別する。
//!
//! 着手手順:
//! 1. `examples/<spec-name>/` を新規作成（1 仕様 = 1 フォルダ）。
//! 2. この `main.rs` と隣の `README.md` をコピーして中身を差し替える。
//! 3. `cargo run -p pilot --example <spec-name>` で実行する。
//!
//! 依存ゼロの最小コード。差し替えること。

fn main() {
    println!("pilot template: replace me");
}
