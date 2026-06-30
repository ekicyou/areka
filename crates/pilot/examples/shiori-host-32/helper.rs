//! host-32 先進坑: 32bit helper バイナリのエントリ（最小スケルトン）。
//!
//! i686 ターゲット（`i686-pc-windows-msvc`）でビルドする独立 example。サブフォルダの
//! `helper.rs` は Cargo の auto-discovery 対象外ゆえ、`pilot/Cargo.toml` に明示
//! `[[example]] name = "shiori-host-32-helper"` を宣言して独立ターゲット化する
//! （design.md §161・要件 7.5 のプロセス分離＝ターゲット別ビルド）。
//!
//! 観測（i686）: `cargo build -p pilot --example shiori-host-32-helper --target i686-pc-windows-msvc`。
//!
//! 本タスク（1.1）は scaffolding のみ。message-only 窓生成・メッセージループ・
//! `pasta.dll` 動的ロードなどの実体は後続タスクで実装する。
//! 葉ノード隔離（命綱・要件 7.2）: コードは本フォルダ配下のみ。

fn main() {
    println!("pilot shiori-host-32-helper (i686 helper): skeleton placeholder");
}
