//! host-32 先進坑: 親 x64 example エントリ（最小スケルトン）。
//!
//! `cargo run -p pilot --example shiori-host-32`（x64）で起動する親プロセス側の
//! プレースホルダ。サブフォルダ `main.rs` ゆえ Cargo の auto-discovery で
//! example 名 `shiori-host-32` として解決される（要件 7.1）。
//!
//! 本タスク（1.1）は scaffolding のみ。helper 起動・WM_COPYDATA IPC・SHIORI/3.0
//! 組立/parse などの実体は後続タスクで実装する（design.md「File Structure Plan」参照）。
//! 葉ノード隔離（命綱・要件 7.2）: コードは本フォルダ配下のみ。production クレートへの
//! inbound 依存を作らない。

// IpcChannel の WM_COPYDATA プロトコルを親/helper で共有する単一ソース
// （design.md §150–153 / §168・物理共有は #[path] 取り込みが標準）。
// 本タスク（1.2）では規約モジュールをコンパイルに取り込むところまで（実走は後続タスク）。
#[path = "ipc.rs"]
mod ipc;

fn main() {
    println!("pilot shiori-host-32 (x64 parent): skeleton placeholder");
    // 共有プロトコルが親ターゲットへ取り込まれていることの最小確認（design.md §372）。
    let _ = ipc::DEFAULT_TIMEOUT;
}
