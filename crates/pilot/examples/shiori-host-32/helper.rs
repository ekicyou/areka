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

    println!("pilot shiori-host-32-helper (i686 helper): skeleton placeholder");
    // 共有プロトコルが helper ターゲットへ取り込まれていることの最小確認。
    let _ = ipc::DEFAULT_TIMEOUT;
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
