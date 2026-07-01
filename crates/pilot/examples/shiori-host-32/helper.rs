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

// 親と同一の WM_COPYDATA プロトコル（IpcChannel）を helper ターゲットへも取り込む。
// 同一 ipc.rs を #[path] で共有することで跨ビットネス規約の単一ソース化を担保する
// （design.md §150–153 / §168 / §372）。
#[path = "ipc.rs"]
mod ipc;

// ShioriByteProxy（pasta.dll 動的ロード＋バイト proxy・design.md §445–495）は helper（i686）
// 専用ゆえ helper.rs からのみ取り込む（main.rs=x64 には載せない・unsafe FFI を本境界へ集約）。
#[path = "shiori_proxy.rs"]
mod shiori_proxy;

// HelperMessageWindow（message-only 窓＋メッセージループ・design.md §415–443）も helper（i686）
// 専用。`wintf-winmsg-executor` の窓/ループを本境界へ閉じる（task 3.2）。
#[path = "helper_window.rs"]
mod helper_window;

fn main() {
    // i686 セルフテスト観測（task 3.1・go 基準(1) precursor・requirements 3.3）:
    //   shiori-host-32-helper.exe --selftest-load <ghostdir>
    // pasta.dll を動的ロード→3 エントリ解決→load(ghostdir) 無 crash 完了→unload を実行し、
    // 結果を標準出力へ出す（親が観測）。ghostdir 省略時は fixtures の emo2 ghost/master を既定。
    // ※ cargo test 経由の観測は同 example の ipc.rs #[cfg(test)] が i686 でビルド不能
    //    （usize >> 32 overflow・本タスク境界外）なため、本実行時セルフテストでも観測できる。
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest-load") {
        let ghostdir = args
            .iter()
            .position(|a| a == "--selftest-load")
            .and_then(|i| args.get(i + 1))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(default_fixture_ghostdir);
        match shiori_proxy::selftest_load(&ghostdir) {
            Ok(msg) => {
                println!("{msg}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("selftest_load FAILED: {e:?}");
                std::process::exit(2);
            }
        }
    }

    // in-process loopback セルフテスト（task 3.2・go 基準(2) helper 側の核・要件 5.1/5.2/5.3）:
    //   shiori-host-32-helper.exe --selftest-loop <secs>
    // stand-in parent 窓が HELLO を実 WM_COPYDATA で受領・helper HWND を復号確認 →
    // N 秒ループ生存 → UNLOAD 清掃終了、までを単独実証する（実クロスプロセス親は task 4.1）。
    if args.iter().any(|a| a == "--selftest-loop") {
        let secs = args
            .iter()
            .position(|a| a == "--selftest-loop")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(3); // 既定 3 秒（ハーネスを塞がぬよう短く）。
        match helper_window::selftest_loop(secs) {
            Ok(()) => {
                println!("selftest_loop OK: HELLO 受領＋{secs}s ループ生存＋clean unload（要件 5.1/5.2/5.3）");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("selftest_loop FAILED: {e}");
                std::process::exit(3);
            }
        }
    }

    // 本番 helper 実行経路（親が起動・design.md §415–443）:
    //   親が PARENT_HWND（u32 10 進）と GHOSTDIR を env（無ければ arg[2]/arg[1]）で渡す
    //   （process_host.rs の PARENT_HWND_ENV / GHOSTDIR_ENV と同名・spawn の arg 並びに一致）。
    // pasta を eager ロードし窓を立て HELLO を送り、UNLOAD までループを回して clean unload する。
    // 実クロスプロセス往復（親駆動）は task 4.1 で完成。ここでは helper 側を起動しループを回す。
    if let Some(parent_hwnd) = read_parent_hwnd(&args) {
        let ghostdir = read_ghostdir(&args).unwrap_or_else(default_fixture_ghostdir);
        // UNLOAD が来なくても暴走しないよう上限（先進坑・ハーネス保護）。親駆動時は UNLOAD で早期停止。
        let code = helper_window::run_helper(parent_hwnd, &ghostdir, std::time::Duration::from_secs(30));
        std::process::exit(code);
    }

    println!("pilot shiori-host-32-helper (i686 helper): skeleton placeholder");
    println!("  modes: --selftest-load <ghostdir> | --selftest-loop <secs> | (env PARENT_HWND[+GHOSTDIR] で本番起動)");
    // 共有プロトコルが helper ターゲットへ取り込まれていることの最小確認。
    let _ = ipc::DEFAULT_TIMEOUT;
}

/// 親 HWND（u32 ワイヤ値）を env `PARENT_HWND`（無ければ arg[2]・10 進）から読む
/// （process_host.rs::PARENT_HWND_ENV / spawn の argv[2] と同名・同並び）。
fn read_parent_hwnd(args: &[String]) -> Option<u32> {
    if let Ok(v) = std::env::var("PARENT_HWND") {
        if let Ok(n) = v.trim().parse::<u32>() {
            return Some(n);
        }
    }
    // arg[2] = parent_hwnd（process_host.rs::spawn の argv 並び）。arg[0]=exe, arg[1]=ghostdir。
    args.get(2).and_then(|s| s.trim().parse::<u32>().ok())
}

/// ghostdir を env `GHOSTDIR`（無ければ arg[1]）から読む（process_host.rs::GHOSTDIR_ENV と同名）。
fn read_ghostdir(args: &[String]) -> Option<std::path::PathBuf> {
    if let Ok(v) = std::env::var("GHOSTDIR") {
        if !v.trim().is_empty() {
            return Some(std::path::PathBuf::from(v));
        }
    }
    args.get(1).map(std::path::PathBuf::from)
}

/// ビルド時の crate ルートから fixtures の emo2 ghostdir（pasta.dll の在処）を組み立てる。
fn default_fixture_ghostdir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("shiori-host-32")
        .join("fixtures")
        .join("emo2")
        .join("ghost")
        .join("master")
}
