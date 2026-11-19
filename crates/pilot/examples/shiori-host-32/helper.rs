//! host-32 先進坑: 32bit helper バイナリのエントリ。
//!
//! i686 ターゲット（`i686-pc-windows-msvc`）でビルドする独立 example。サブフォルダの
//! `helper.rs` は Cargo の auto-discovery 対象外ゆえ、`pilot/Cargo.toml` に明示
//! `[[example]] name = "shiori-host-32-helper"` を宣言して独立ターゲット化する
//! （design.md §161・要件 7.5 のプロセス分離＝ターゲット別ビルド）。
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
// 専用。`wintf-winmsg-executor` の窓/ループを本境界へ閉じる。
#[path = "helper_window.rs"]
mod helper_window;

// 検体を窓口から取得し**プロセス寿命で保持**する共有の受け口（spec: areka-P0-nar-install 要件 1.6）。
// 退避経路が返すパスの寿命は保持している値の寿命なので、関数内の一時値にはしない。
#[path = "sample_support.rs"]
mod sample_support;

fn main() {
    // i686 セルフテスト観測（go 基準(1) precursor・requirements 3.3）:
    //   shiori-host-32-helper.exe --selftest-load <ghostdir>
    // pasta.dll を動的ロード→3 エントリ解決→load(ghostdir) 無 crash 完了→unload を実行し、
    // 結果を標準出力へ出す（親が観測）。ghostdir 省略時は検体 emo2 の ghost/master を既定。
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

    // in-process loopback セルフテスト（go 基準(2) helper 側の核・要件 5.1/5.2/5.3）:
    //   shiori-host-32-helper.exe --selftest-loop <secs>
    // stand-in parent 窓が HELLO を実 WM_COPYDATA で受領・helper HWND を復号確認 →
    // N 秒ループ生存 → UNLOAD 清掃終了、までを単独実証する。
    if args.iter().any(|a| a == "--selftest-loop") {
        let secs = args
            .iter()
            .position(|a| a == "--selftest-loop")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(3); // 既定 3 秒（ハーネスを塞がぬよう短く）。
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

    // 本番 helper 実行経路（親が起動・design.md §415–443）:
    //   argv[1]=ghostdir・argv[2]=parent_hwnd（u32 10 進）＝process_host.rs::spawn の並び。
    // pasta を eager ロードし窓を立て HELLO を送り、UNLOAD までループを回して clean unload する。
    if let Some(parent_hwnd) = args.get(2).and_then(|s| s.trim().parse::<u32>().ok()) {
        let ghostdir = std::path::PathBuf::from(&args[1]);
        // UNLOAD が来なくても暴走しないよう上限（先進坑・ハーネス保護）。親駆動時は UNLOAD で早期停止。
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
