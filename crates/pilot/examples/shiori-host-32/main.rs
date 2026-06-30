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

// SHIORI/3.0 ワイヤコーデック（x64 親側に閉じる・helper からは参照しない）。
// design.md Shiori3Codec §376–411 / research.md §5.4。本タスク（2.1）では
// 親ターゲットへ取り込むところまで（ParentDriver からの実呼び出しは後続タスク）。
#[path = "shiori3.rs"]
mod shiori3;

// ProcessHost: helper プロセスの起動と生存監視（design.md §285–327・x64 親に閉じる）。
// helper（i686）からは取り込まない（プロセス分離・要件 1.5）。本タスク（2.2）で実装。
#[path = "process_host.rs"]
mod process_host;

use std::path::{Path, PathBuf};

use process_host::{ExitKind, ProcessHost};

/// helper exe パスを解決する（design.md §324 確定）:
/// 環境変数 `HELPER_EXE`（無ければ第 1 CLI 引数）。どちらも無ければ `None`。
fn resolve_helper_exe() -> Option<PathBuf> {
    if let Ok(p) = std::env::var(process_host::HELPER_EXE_ENV) {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    std::env::args().nth(1).map(PathBuf::from)
}

fn main() {
    println!("pilot shiori-host-32 (x64 parent): skeleton placeholder");
    // 共有プロトコルが親ターゲットへ取り込まれていることの最小確認（design.md §372）。
    let _ = ipc::DEFAULT_TIMEOUT;
    // SHIORI3 コーデックが親ターゲットへ取り込まれていることの最小確認（design.md §376）。
    let _ = shiori3::module_loaded();

    // ── 2.2 スライス: ProcessHost による helper 起動 → 終了コード取得 ──────────────
    // （メッセージ窓・WM_COPYDATA 往復・HELLO ハンドシェイクは未実装。後続タスク 4.1。）
    // 親 HWND はここではプレースホルダ 0（実 HWND ハンドシェイクは task 4.1・要件外）。
    let parent_hwnd: u32 = 0;
    // ghostdir は SHIORI `load` 対象（design.md §320）。本フォルダ相対の固定パス。
    let ghostdir = Path::new("crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master");

    match resolve_helper_exe() {
        Some(helper_exe) => {
            println!("[2.2] helper_exe = {}", helper_exe.display());
            println!("[2.2] ghostdir   = {}", ghostdir.display());
            println!("[2.2] parent_hwnd(placeholder) = {parent_hwnd}");
            match ProcessHost::spawn(&helper_exe, ghostdir, parent_hwnd) {
                Ok(handle) => match ProcessHost::wait_clean(handle) {
                    Ok(code) => {
                        let kind = ExitKind::classify(Some(code));
                        // 観測点: 親が helper の終了コードを取得し clean/異常を分類できる
                        // （要件 1.1/1.2/1.3/1.4・design.md §326）。
                        println!("[2.2] helper exited: code={code} kind={kind:?}");
                    }
                    Err(e) => eprintln!("[2.2] wait_clean failed: {e}"),
                },
                Err(e) => eprintln!("[2.2] spawn failed: {e}"),
            }
        }
        None => {
            // 観測の前提が無い場合は配線のみ示して終了（テスト/CI で env 未設定でも落とさない）。
            println!(
                "[2.2] HELPER_EXE not set and no argv[1]; \
                 set $env:HELPER_EXE to the i686 helper exe to observe the real spawn."
            );
        }
    }
}
