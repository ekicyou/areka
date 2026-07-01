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

// ParentMessageWindow: x64 親の message-only 窓（HELLO ハンドシェイク＋RESPONSE 再入受領）。
// ParentDriver 境界の x64 側窓（design.md §176–212）。helper からは参照しない。本タスク（4.1）。
#[path = "parent_window.rs"]
mod parent_window;

use std::path::{Path, PathBuf};
use std::time::Duration;

use ipc::MsgTag;
use parent_window::ParentMessageWindow;
use process_host::{ExitKind, ProcessHost};

/// HWND ハンドシェイク待ちの上限（ハーネスを塞がぬよう短く・design.md §208）。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// OnBoot 1 往復（REQUEST→RESPONSE 再入受領）の上限（要件 2.3・ハングしない）。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// UNLOAD 送出のタイムアウト（clean unload 指示）。
const UNLOAD_TIMEOUT: Duration = Duration::from_secs(3);

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
    // 共有プロトコル/コーデックが親ターゲットへ取り込まれていることの最小確認（design.md §372/§376）。
    let _ = ipc::DEFAULT_TIMEOUT;
    let _ = shiori3::module_loaded();

    // ghostdir は SHIORI `load` 対象（design.md §320）。本フォルダ相対の固定パス。
    let ghostdir = Path::new("crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master");

    let helper_exe = match resolve_helper_exe() {
        Some(p) => p,
        None => {
            eprintln!(
                "[go(1)] HELPER_EXE 未設定かつ argv[1] なし。\
                 $env:HELPER_EXE に i686 helper exe を設定して再実行してください。"
            );
            std::process::exit(2);
        }
    };

    // 全体駆動（go 基準(1)・design.md §172–204）。失敗は観測可能に出して非ゼロ終了（ハングしない）。
    match drive_go_criterion_1(&helper_exe, ghostdir) {
        Ok(value) => {
            // ★ go 基準(1) の観測点（design.md §198/§203）: emo2 OnBoot の Value（起動挨拶さくら）。
            println!("=== go 基準(1) OBSERVABLE: emo2 OnBoot Value ===");
            println!("Value: {value}");
            println!("=== go 基準(1) PASS: x64 親が i686 helper 越しに OnBoot Value を受領 ===");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("=== go 基準(1) FAIL: {e} ===");
            std::process::exit(1);
        }
    }
}

/// go 基準(1) の全体駆動（design.md §172–204）。成功で emo2 OnBoot の `Value:` を返す。
///
/// a. 親 message-only 窓生成 → HWND 取得。
/// b. ProcessHost::spawn（real parent HWND を u32 で渡す）→ helper が HELLO を送る。
/// c. 親ループを HELLO 受領まで pump（HWND ハンドシェイク・bounded・ハングしない）。
/// d. Shiori3Codec::build_onboot → OnBoot SHIORI/3.0 bytes。
/// e. send_request（REQUEST 送出 → RESPONSE 再入受領・受け皿セル方式・design.md §209）。
/// f. parse_value → Value を取り出す（go 基準(1) 観測）。
/// g. UNLOAD 送出 → ProcessHost::wait_clean で終了コード 0 確認。
fn drive_go_criterion_1(helper_exe: &Path, ghostdir: &Path) -> Result<String, String> {
    // a. 親 message-only 窓を先に立てる（helper 起動前・HELLO 受け皿）。
    let parent = ParentMessageWindow::create()?;
    let parent_hwnd_u32 = parent.hwnd_u32();
    let parent_hwnd = ipc::hwnd_from_u32(parent_hwnd_u32);
    println!("[go(1)] 親 message-only 窓 生成: hwnd(u32)={parent_hwnd_u32:#010x}");
    println!("[go(1)] helper_exe = {}", helper_exe.display());
    println!("[go(1)] ghostdir   = {}", ghostdir.display());

    // b. helper 起動（real parent HWND を u32 で seed・design.md §182）。
    let mut handle = ProcessHost::spawn(helper_exe, ghostdir, parent_hwnd_u32)
        .map_err(|e| format!("helper 起動失敗: {e}"))?;
    println!("[go(1)] helper 起動 OK（HELLO を待機）");

    // 早期異常検出（helper が起動直後に落ちた等）。
    if let Some(kind) = ProcessHost::poll_exit_kind(&mut handle) {
        return Err(format!("helper が HELLO 前に終了した: {kind:?}"));
    }

    // c. HELLO 受領まで pump（HWND ハンドシェイク・bounded）。
    let helper_hwnd_u32 = match parent.pump_until_hello_or(HANDSHAKE_TIMEOUT) {
        Some(h) => h,
        None => {
            // ハンドシェイク不成立。helper の生死も併せて観測（要件 2.4）。
            let alive = ProcessHost::poll_exit_kind(&mut handle);
            return Err(format!(
                "HELLO ハンドシェイクが {HANDSHAKE_TIMEOUT:?} 内に成立しなかった（helper={alive:?}）"
            ));
        }
    };
    let helper_hwnd = ipc::hwnd_from_u32(helper_hwnd_u32);
    println!("[go(1)] HWND ハンドシェイク完了: helper hwnd(u32)={helper_hwnd_u32:#010x}");

    // d. OnBoot SHIORI/3.0 リクエストを組み立てる（design.md §189・要件 4.1）。
    let onboot = shiori3::build_onboot(ghostdir);
    println!("[go(1)] OnBoot 組立 OK（{} バイト）", onboot.len());

    // e. REQUEST 送出 → RESPONSE を受け皿セルで再入受領（design.md §209・要件 2.2/4.3）。
    //    親はここで SendMessageTimeout にブロックし、待機中に helper の RESPONSE を
    //    親 WndProc が再入受領して受け皿セルへ格納する（デッドロック回避・§210）。
    let shared = parent.shared();
    let slot = shared.response_slot();
    let response = ipc::send_request(
        helper_hwnd,
        parent_hwnd,
        MsgTag::Request,
        &onboot,
        REQUEST_TIMEOUT,
        slot,
    )
    .map_err(|e| {
        format!(
            "OnBoot 1 往復に失敗: {e:?}（再入受領が i686↔x64 で不成立の可能性・design.md §210 \
             Revalidation Trigger＝named pipe 後退を人間判断）"
        )
    })?;
    println!("[go(1)] RESPONSE 再入受領 OK（{} バイト）", response.len());

    // f. Value を parse（design.md §198・要件 4.2）。
    let value = shiori3::parse_value(&response)
        .ok_or_else(|| "RESPONSE に Value: が無い（emo2 OnBoot 応答の parse 失敗）".to_string())?;
    if value.is_empty() {
        return Err("Value: が空（起動挨拶さくらスクリプトが空）".to_string());
    }

    // g. UNLOAD 送出（片道）→ clean unload（design.md §199–202・要件 1.3/5.3）。
    if let Err(e) = ipc::send_copydata(
        helper_hwnd,
        parent_hwnd,
        MsgTag::Unload,
        &[],
        UNLOAD_TIMEOUT,
    ) {
        // UNLOAD 送出失敗は致命ではない（helper は max_run 上限で自律停止する）。観測のみ。
        eprintln!("[go(1)] UNLOAD 送出失敗（helper は上限で自律停止・観測のみ）: {e:?}");
    } else {
        println!("[go(1)] UNLOAD 送出 OK");
    }

    // clean unload を待って終了コードを確認（要件 1.3・go 基準(2) の一部だが往復完了の締め）。
    match ProcessHost::wait_clean(handle) {
        Ok(code) => {
            let kind = ExitKind::classify(Some(code));
            println!("[go(1)] helper exited: code={code} kind={kind:?}");
            if !kind.is_clean() {
                eprintln!("[go(1)] 警告: helper が clean(0) で終了しなかった（{kind:?}）");
            }
        }
        Err(e) => eprintln!("[go(1)] wait_clean 失敗（観測のみ）: {e}"),
    }

    Ok(value)
}
