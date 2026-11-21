//! host-32 先進坑: 32bit helper バイナリのエントリ。
//!
//! i686 ターゲット（`i686-pc-windows-msvc`）でビルドする独立 example。サブフォルダの
//! `helper.rs` は Cargo の auto-discovery 対象外なので、`pilot/Cargo.toml` に
//! `[[example]] name = "shiori-host-32-helper"` を明示して独立ターゲットにする。
//!
//! 観測（i686）: `cargo build -p pilot --example shiori-host-32-helper --target i686-pc-windows-msvc`。
//!
//! 値は引数だけで受け取る（環境変数は読まない）。モード:
//! - `--selftest-load [ghostdir]`: pasta.dll を読み `load(ghostdir)` を呼ぶ。
//! - `--selftest-loop [secs]`: 同じプロセスの親役の窓と HELLO〜UNLOAD を往復する。
//! - `<ghostdir> <parent_hwnd>`: 本番。親が `argv[1]=ghostdir`・`argv[2]=parent_hwnd`（10 進）で起こす。
//!
//! 終了コード: 0＝成功・2＝`--selftest-load` の失敗か窓／HELLO の失敗・3＝`--selftest-loop` の失敗・
//! 4＝UNLOAD が来ないまま上限で抜けた・5＝UNLOAD で抜けたが pasta を読めていなかった。
//! ghostdir を省くと検体 emo2 の ghost/master を使う。
//!
//! 葉ノード隔離（命綱）: コードは本フォルダ配下のみ。

// 親と同じ ipc.rs を #[path] で共有し、跨ビットネスの WM_COPYDATA 規約を 1 か所に置く。
#[path = "ipc.rs"]
mod ipc;

// pasta.dll の動的ロードは helper（i686）専用。x64 の main.rs には載せない。
#[path = "shiori_proxy.rs"]
mod shiori_proxy;

#[path = "helper_window.rs"]
mod helper_window;

// 検体はプロセス寿命で保持する。返るパスの寿命は保持している値の寿命なので、関数内の一時値にはしない。
#[path = "sample_support.rs"]
mod sample_support;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest-load") {
        let ghostdir = args
            .iter()
            .position(|a| a == "--selftest-load")
            .and_then(|i| args.get(i + 1))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(sample_support::emo2_ghost_master);
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

    if args.iter().any(|a| a == "--selftest-loop") {
        let secs = args
            .iter()
            .position(|a| a == "--selftest-loop")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(3); // ハーネスを塞がないよう短く。
        match helper_window::selftest_loop(secs) {
            Ok(()) => {
                println!(
                    "selftest_loop OK: HELLO 受領＋{secs}s ループ生存＋clean unload（要件 5.1/5.2/5.3）"
                );
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("selftest_loop FAILED: {e}");
                std::process::exit(3);
            }
        }
    }

    // 引数の並びは process_host.rs::spawn と同じ（argv[1]=ghostdir・argv[2]=parent_hwnd）。
    if let Some(parent_hwnd) = args.get(2).and_then(|s| s.trim().parse::<u32>().ok()) {
        let ghostdir = std::path::PathBuf::from(&args[1]);
        // UNLOAD が来なくてもハーネスを塞がないよう上限を置く。
        let code =
            helper_window::run_helper(parent_hwnd, &ghostdir, std::time::Duration::from_secs(30));
        std::process::exit(code);
    }

    println!("pilot shiori-host-32-helper (i686 helper)");
    println!("  --selftest-load [ghostdir]   pasta.dll を読み load(ghostdir) を呼ぶ");
    println!("  --selftest-loop [secs]       同じプロセスで HELLO〜UNLOAD を往復する（既定 3 秒）");
    println!(
        "  <ghostdir> <parent_hwnd>     本番。親が argv[1]=ghostdir argv[2]=parent_hwnd（10 進）で起こす"
    );
    println!("  ghostdir を省くと検体 emo2 の ghost/master");
    println!("  終了コード: 0=成功 2=selftest-load か窓/HELLO の失敗 3=selftest-loop の失敗");
    println!("              4=UNLOAD が来ず上限で抜けた 5=UNLOAD で抜けたが pasta を読めていない");
}
