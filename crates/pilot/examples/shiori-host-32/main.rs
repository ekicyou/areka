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
use std::time::{Duration, Instant};

use ipc::MsgTag;
use parent_window::{ParentMessageWindow, ParentShared};
use process_host::{ExitKind, HelperHandle, ProcessHost};
use std::pin::Pin;

/// HWND ハンドシェイク待ちの上限（ハーネスを塞がぬよう短く・design.md §208）。
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// OnBoot 1 往復（REQUEST→RESPONSE 再入受領）の上限（要件 2.3・ハングしない）。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// UNLOAD 送出のタイムアウト（clean unload 指示）。
const UNLOAD_TIMEOUT: Duration = Duration::from_secs(3);

/// go 基準(2) のメッセージループ生存観測窓 N（要件 5.2・design.md §214–232）。
/// helper の message loop がこの N 秒を破綻なく生存し続けることを親が観測する。
/// helper 側 backstop（`run_helper` の 30s・helper.rs §88）より十分短く、ハーネス安全な短尺。
const SURVIVAL_WINDOW: Duration = Duration::from_secs(4);
/// 生存窓の間、親が helper 生死を `poll_exit` する間隔（この刻みで N 秒を消化する）。
const SURVIVAL_POLL_INTERVAL: Duration = Duration::from_millis(250);

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

    // 全体駆動。失敗は観測可能に出して非ゼロ終了（ハングしない・要件 2.3）。
    //
    // 構成（design.md §172–232）:
    //   [go 基準(1)] setup（helper 起動＋HELLO＋OnBoot 1 往復）→ Value 受領・確認。
    //   [go 基準(2)] 同一 helper を N 秒生存させ（poll_exit で生存監視）→ UNLOAD → clean unload
    //               → 終了コード 0 を親が観測（design.md §230: 終了コード 0 ＋親側観測ログを一次記録）。
    // UNLOAD は go(1) では送らず、go(2) の生存窓の**後**に送る（設計変更点・§221「N 秒後」）。
    match drive(&helper_exe, ghostdir) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("=== FAIL: {e} ===");
            std::process::exit(1);
        }
    }
}

/// 全体駆動（go 基準(1)＋(2)）。go(1) の 1 往復成功後、同じ helper で go(2) の生存→clean unload を観測する。
fn drive(helper_exe: &Path, ghostdir: &Path) -> Result<(), String> {
    // 親 message-only 窓は go(1)/go(2) を通じて生存させる（RESPONSE 受け皿・HELLO 受け皿）。
    let parent = ParentMessageWindow::create()?;

    // --- setup + go 基準(1): 往復して Value を受領（UNLOAD はまだ送らない）---
    let mut session = drive_go_criterion_1(&parent, helper_exe, ghostdir)?;

    // ★ go 基準(1) の観測点（design.md §198/§203）: emo2 OnBoot の Value（起動挨拶さくら）。
    println!("=== go 基準(1) OBSERVABLE: emo2 OnBoot Value ===");
    println!("Value: {}", session.value);
    println!("=== go 基準(1) PASS: x64 親が i686 helper 越しに OnBoot Value を受領 ===");

    // --- go 基準(2): N 秒メッセージループ生存 → UNLOAD → clean unload → 終了コード 0 観測 ---
    drive_go_criterion_2(&mut session)?;

    Ok(())
}

/// go 基準(1) の 1 往復で確立した「生きた helper セッション」（go(2) がこれを引き継ぐ）。
///
/// 窓・handle・helper HWND を保持し、UNLOAD 未送出のまま go(2) の生存窓へ渡す。
struct HelperSession<'p> {
    /// 親 message-only 窓（RESPONSE 受け皿・go(2) 中も生存）。
    parent: &'p ParentMessageWindow,
    /// helper 子プロセスハンドル（生存監視・clean unload 待ちの単一ソース）。
    /// clean unload 待ち（`wait_clean`）で所有権を消費するため `Option`（`take` して wait へ渡す）。
    handle: Option<HelperHandle>,
    /// 親窓 HWND（UNLOAD の from）。
    parent_hwnd: windows::Win32::Foundation::HWND,
    /// helper メッセージ窓 HWND（UNLOAD / 追加 REQUEST の宛先）。
    helper_hwnd: windows::Win32::Foundation::HWND,
    /// go(1) で受領した OnBoot の Value（親が確認済み）。
    value: String,
}

/// go 基準(1) の全体駆動（design.md §172–204）。成功で「生きた helper セッション」を返す。
///
/// **設計変更点（task 5.1）**: UNLOAD はここでは送らない。go 基準(2) の生存窓の**後**に送る
/// （design.md §221「N 秒後」）。ゆえに戻り値は Value 文字列ではなく、UNLOAD 未送出のまま
/// go(2) へ引き継ぐ `HelperSession`（helper 稼働中・handle 保持）とする。
///
/// a. 親 message-only 窓は呼び出し側（`drive`）が生成・保持（go(2) 中も生存）。
/// b. ProcessHost::spawn（real parent HWND を u32 で渡す）→ helper が HELLO を送る。
/// c. 親ループを HELLO 受領まで pump（HWND ハンドシェイク・bounded・ハングしない）。
/// d. Shiori3Codec::build_onboot → OnBoot SHIORI/3.0 bytes。
/// e. send_request（REQUEST 送出 → RESPONSE 再入受領・受け皿セル方式・design.md §209）。
/// f. parse_value → Value を取り出す（go 基準(1) 観測）。
fn drive_go_criterion_1<'p>(
    parent: &'p ParentMessageWindow,
    helper_exe: &Path,
    ghostdir: &Path,
) -> Result<HelperSession<'p>, String> {
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

    // UNLOAD はここでは送らない（設計変更点・task 5.1）。go(2) の生存窓の後に送る（§221）。
    // helper 稼働中のまま session を go(2) へ引き継ぐ。
    Ok(HelperSession {
        parent,
        handle: Some(handle),
        parent_hwnd,
        helper_hwnd,
        value,
    })
}

/// go 基準(2) の駆動（要件 1.3/5.2/5.4・design.md §214–232）。
///
/// 前提: `session` は go(1) で確立した稼働中の helper（UNLOAD 未送出）。
/// 手順:
///   1. **生存窓**: helper を N 秒（`SURVIVAL_WINDOW`）生かし、`ProcessHost::poll_exit` で
///      刻みごとに生存（`None`）を確認する。途中で終了していれば go(2) 失敗（loop が破綻＝要件 5.2 不成立）。
///   2. 生存窓の後に UNLOAD を送出（design.md §221「N 秒後」）。
///   3. `ProcessHost::wait_clean` で終了コードを取得し、**0 / `ExitKind::Clean`** を親が観測する
///      （design.md §230: 終了コード 0 ＋親側観測ログを一次記録・要件 1.3/5.4）。非ゼロなら失敗。
///
/// 全ステップは bounded: 生存窓は N（`SURVIVAL_WINDOW`）＋刻み、UNLOAD は `UNLOAD_TIMEOUT`。
/// clean unload 待ち（`wait_clean`）は helper が UNLOAD で即 clean 終了するため実質即返り、
/// 最悪でも helper 側 backstop（`run_helper` 30s・helper.rs §88）が上限を保証する（ハングしない）。
fn drive_go_criterion_2(session: &mut HelperSession) -> Result<(), String> {
    println!(
        "=== go 基準(2): メッセージループ {SURVIVAL_WINDOW:?} 生存 → clean unload を観測 ==="
    );

    // --- 1. 生存窓: N 秒間 helper が生きていることを poll_exit で刻みごとに観測（要件 5.2）---
    let started = Instant::now();
    let mut polls = 0u64;
    let handle = session
        .handle
        .as_mut()
        .ok_or_else(|| "go 基準(2) 内部エラー: helper handle が既に消費済み".to_string())?;
    while started.elapsed() < SURVIVAL_WINDOW {
        // 生死監視（IPC レイヤと直交・要件 2.4/1.2）。稼働中なら None。
        if let Some(kind) = ProcessHost::poll_exit_kind(handle) {
            // 生存窓の途中で helper が終了した＝ループが N 秒生存できなかった（要件 5.2 不成立）。
            let elapsed = started.elapsed();
            return Err(format!(
                "go 基準(2) 不成立: helper が生存窓 {SURVIVAL_WINDOW:?} を待たず {elapsed:?} で終了した \
                 （メッセージループが破綻・{kind:?}・UNLOAD 未送出）"
            ));
        }
        polls += 1;
        std::thread::sleep(SURVIVAL_POLL_INTERVAL);
    }
    let survived = started.elapsed();
    println!(
        "[go(2)] helper survived {:.2}s（poll_exit={} 回 すべて alive=None・メッセージループ生存・要件 5.2）",
        survived.as_secs_f64(),
        polls
    );

    // 任意: 生存後もループが REQUEST を捌けることを示す（bounded・失敗しても致命ではない）。
    let _ = mid_window_probe(session);

    // --- 2. 生存窓の後に UNLOAD 送出（design.md §221「N 秒後」・要件 5.3）---
    match ipc::send_copydata(
        session.helper_hwnd,
        session.parent_hwnd,
        MsgTag::Unload,
        &[],
        UNLOAD_TIMEOUT,
    ) {
        Ok(()) => println!("[go(2)] UNLOAD 送出 OK（N 秒生存後・clean unload 指示）"),
        Err(e) => {
            // UNLOAD 送出失敗は致命ではない（helper は backstop で自律停止する）が、観測して記録。
            eprintln!("[go(2)] UNLOAD 送出失敗（helper は backstop 上限で自律停止・観測のみ）: {e:?}");
        }
    }

    // --- 3. clean unload を待って終了コード 0 を親が観測（要件 1.3/5.4・design.md §230）---
    //     helper 子ハンドルを消費して wait（`session.handle` を take で取り出す）。
    let handle = session
        .handle
        .take()
        .ok_or_else(|| "go 基準(2) 内部エラー: helper handle が既に消費済み".to_string())?;
    match ProcessHost::wait_clean(handle) {
        Ok(code) => {
            let kind = ExitKind::classify(Some(code));
            println!("[go(2)] helper exited code={code} kind={kind:?}（親が終了コードを観測）");
            if kind.is_clean() {
                println!(
                    "=== go 基準(2) PASS: helper survived {:.2}s → clean unload → exit code=0 kind=Clean を親が観測（要件 1.3/5.2/5.4・design.md §230） ===",
                    survived.as_secs_f64()
                );
                Ok(())
            } else {
                Err(format!(
                    "go 基準(2) 不成立: helper が clean(0) で終了しなかった（code={code} {kind:?}）"
                ))
            }
        }
        Err(e) => Err(format!("go 基準(2) 不成立: clean unload 待ちに失敗（{e}）")),
    }
}

/// 生存窓の後、helper のメッセージループがまだ REQUEST を捌けることを示す任意プローブ（bounded）。
/// 失敗しても go(2) は致命扱いしない（生存＋clean unload が本質・これは「まだ動く」補足観測）。
fn mid_window_probe(session: &HelperSession) -> Result<(), ()> {
    let onboot = shiori3::build_onboot(std::path::Path::new(""));
    let shared: Pin<&ParentShared> = session.parent.shared();
    match ipc::send_request(
        session.helper_hwnd,
        session.parent_hwnd,
        MsgTag::Request,
        &onboot,
        REQUEST_TIMEOUT,
        shared.response_slot(),
    ) {
        Ok(resp) => {
            println!(
                "[go(2)] 生存後の追加 REQUEST に helper が応答（{} バイト）＝ループは生存後も稼働中",
                resp.len()
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("[go(2)] 追加 REQUEST は失敗（補足観測のみ・致命ではない）: {e:?}");
            Err(())
        }
    }
}
