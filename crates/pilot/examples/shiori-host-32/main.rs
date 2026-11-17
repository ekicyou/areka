//! host-32 先進坑: x64 親の入口。
//!
//! `cargo run -p pilot --example shiori-host-32`（x64）で起こす。i686 helper を起こし、
//! WM_COPYDATA で OnBoot を 1 往復させて Value を受け（go 基準(1)）、同じ helper を N 秒
//! 生かしてから UNLOAD で clean unload させる（go 基準(2)）。`--selftest-errors` では
//! 異常系 err(1)・err(2) を観測する。
//! 葉ノード隔離（命綱・要件 7.2）: コードは本フォルダ配下のみ。production クレートへの
//! inbound 依存を作らない。

// WM_COPYDATA の送りの決まり（親と helper が #[path] で同じファイルを取り込む）。
#[path = "ipc.rs"]
mod ipc;

// SHIORI/3.0 の組立と Value の取り出し（x64 親に閉じる）。
#[path = "shiori3.rs"]
mod shiori3;

// helper プロセスの起動と生存監視（x64 親に閉じる・要件 1.5）。
#[path = "process_host.rs"]
mod process_host;

// 親の message-only 窓（HELLO の受け・RESPONSE の再入受領・1 往復の送り）。
#[path = "parent_window.rs"]
mod parent_window;

// 検体を窓口から取得し**プロセス寿命で保持**する共有の受け口（spec: areka-P0-nar-install 要件 1.6）。
#[path = "sample_support.rs"]
mod sample_support;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ipc::MsgTag;
use parent_window::ParentMessageWindow;
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
/// helper 側 backstop（helper.rs が `run_helper` へ渡す 30s）より十分短く、ハーネス安全な短尺。
const SURVIVAL_WINDOW: Duration = Duration::from_secs(4);
/// 生存窓の間、親が helper 生死を `poll_exit_kind` で見る間隔（この刻みで N 秒を消化する）。
const SURVIVAL_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// helper の exe（環境変数 `HELPER_EXE`、無ければ `--` で始まらない最初の引数）。
fn helper_exe_from_process() -> Option<PathBuf> {
    process_host::resolve_helper_exe(
        std::env::var(process_host::HELPER_EXE_ENV).ok(),
        std::env::args().skip(1),
    )
}

fn main() {
    // ghostdir は SHIORI `load` 対象（design.md §320）。検体の窓口から**絶対パス**で得る
    // （実走は絶対パスが定石＝相対だと pasta.dll の LOAD が 0x8007007E で落ちる）。
    let ghostdir = sample_support::emo2_ghost_master();
    let ghostdir = ghostdir.as_path();

    // --- 異常系セルフテストモード（design.md §521 IPC / §528 helper 異常終了）---
    // `--selftest-errors` で 2 つの異常系を観測する:
    //   (1) IPC タイムアウト（要件 2.3）: 無応答 target へ send_copydata が bounded に Timeout を返す。
    //   (2) helper 強制終了検出（要件 1.4/2.4）: 実 i686 helper を kill し親が異常終了を検出。
    // いずれも **ハングしない**（bounded）ことが本質。両方 PASS で exit 0。
    if std::env::args().any(|a| a == "--selftest-errors") {
        match drive_selftest_errors(ghostdir) {
            Ok(()) => {
                println!(
                    "=== --selftest-errors: 両異常系 PASS（IPC Timeout ＋ helper 異常終了検出・いずれもハングせず） ==="
                );
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("=== --selftest-errors FAIL: {e} ===");
                std::process::exit(1);
            }
        }
    }

    let helper_exe = match helper_exe_from_process() {
        Some(p) => p,
        None => {
            eprintln!(
                "[go(1)] HELPER_EXE 未設定かつ `--` で始まらない引数なし。\
                 $env:HELPER_EXE に i686 helper exe を設定して再実行してください。"
            );
            std::process::exit(2);
        }
    };

    // 全体駆動。失敗は観測可能に出して非ゼロ終了（ハングしない・要件 2.3）。
    //
    // 構成（design.md §172–232）:
    //   [go 基準(1)] setup（helper 起動＋HELLO＋OnBoot 1 往復）→ Value 受領・確認。
    //   [go 基準(2)] 同一 helper を N 秒生存させ（poll_exit_kind で生存監視）→ UNLOAD → clean unload
    //               → 終了コード 0 を親が観測（design.md §230: 終了コード 0 ＋親側観測ログを一次記録）。
    // UNLOAD は go(1) では送らず、go(2) の生存窓の**後**に送る（§221「N 秒後」）。
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

    // go(1) の観測記録を保持（go(2) 後の一次記録サマリで再掲・design.md §230）。
    let request_roundtrip = session.request_roundtrip;
    let response_bytes = session.response_bytes;
    let value_bytes = session.value_bytes;

    // --- go 基準(2): N 秒メッセージループ生存 → UNLOAD → clean unload → 終了コード 0 観測 ---
    let go2 = drive_go_criterion_2(&mut session)?;

    // === 一次記録サマリ（design.md §230: 終了コード 0 ＋親側観測ログを一次記録）===
    // 数値を自己完結の 1 ブロックに集約する。go 判定そのものは開発者の人間判断（要件 6.4/6.5）。
    println!("=== GO 検証記録 (実 pasta.dll / emo2) ===");
    println!(
        "go(1): Value={} bytes 受領 [PASS] / RESPONSE={} bytes / REQUEST→RESPONSE 同期往復={} ms (block-on-reply)",
        value_bytes,
        response_bytes,
        request_roundtrip.as_millis()
    );
    println!(
        "go(2): survived {:.2}s (poll={} 回 alive / probe: 生存後 REQUEST {}) → clean unload kind={:?} [PASS]",
        go2.survived.as_secs_f64(),
        go2.polls,
        if go2.post_survival_response_bytes.is_some() {
            "応答あり"
        } else {
            "応答なし"
        },
        go2.exit_kind,
    );
    println!(
        "load→spawn_actor: design.md §495 が load 時 actor スレッドを仮説化したが利用可能ソースでは未確認\
         （vendors/pasta に該当シンボル無し・pasta_shiori/src/shiori.rs は single-threaded と明記／\
         かつ実ロードは prebuilt emo2 pasta.dll でその内部は vendored source から検証不能）\
         ⇒ 挙動証拠のみに格下げ: helper が go(1) 要求＋go(2) 生存後要求（実 2 応答）を処理し clean unload\
         ＝リクエスト処理ループが生存窓を跨いで稼働（代理証拠・x64 親は helper 内スレッド数を直接観測できない）"
    );
    println!("（go 判定は README・人間判断・要件 6.5）===");

    Ok(())
}

/// go 基準(2) の観測記録（一次記録サマリ用・数値のみ）。
struct Go2Record {
    /// N 秒生存窓で実際に生存した実時間。
    survived: Duration,
    /// 生存窓中の poll_exit_kind 回数（すべて alive=None）。
    polls: u64,
    /// 生存後の追加 REQUEST に helper が返した応答バイト数（None=応答なし）。
    /// Some ＝ pasta の actor ループが生存後も稼働継続していることの親可視な証拠。
    post_survival_response_bytes: Option<usize>,
    /// 終わり方の分類（Clean＝終了コード 0 を期待）。
    exit_kind: ExitKind,
}

/// go 基準(1) の 1 往復で確立した「生きた helper セッション」（go(2) がこれを引き継ぐ）。
///
/// 窓・handle・helper HWND を保持し、UNLOAD 未送出のまま go(2) の生存窓へ渡す。
struct HelperSession<'p> {
    /// 親 message-only 窓（RESPONSE 受け皿・go(2) 中も生存）。
    parent: &'p ParentMessageWindow,
    /// helper 子プロセスハンドル（生存監視・clean unload 待ちの単一ソース）。
    /// clean unload 待ち（`wait_kind`）で所有権を消費するため `Option`（`take` して wait へ渡す）。
    handle: Option<HelperHandle>,
    /// 親窓 HWND（UNLOAD の from）。
    parent_hwnd: windows::Win32::Foundation::HWND,
    /// helper メッセージ窓 HWND（UNLOAD / 追加 REQUEST の宛先）。
    helper_hwnd: windows::Win32::Foundation::HWND,
    /// go(1) で受領した OnBoot の Value（親が確認済み）。
    value: String,
    /// go(1) 観測記録: OnBoot REQUEST→RESPONSE 同期往復に要した実時間（block-on-reply 実証）。
    request_roundtrip: Duration,
    /// go(1) 観測記録: 受領した RESPONSE バイト数（実 pasta 応答・run ごとに変動しうる）。
    response_bytes: usize,
    /// go(1) 観測記録: 受領した Value のバイト数（さくらスクリプト本体・run ごとに変動しうる）。
    value_bytes: usize,
}

/// go 基準(1) の全体駆動（design.md §172–204）。成功で「生きた helper セッション」を返す。
///
/// UNLOAD はここでは送らない。go 基準(2) の生存窓の**後**に送る
/// （design.md §221「N 秒後」）。ゆえに戻り値は Value 文字列ではなく、UNLOAD 未送出のまま
/// go(2) へ引き継ぐ `HelperSession`（helper 稼働中・handle 保持）とする。
///
/// a. 親 message-only 窓は呼び出し側（`drive`）が生成・保持（go(2) 中も生存）。
/// b. ProcessHost::spawn（real parent HWND を u32 で渡す）→ helper が HELLO を送る。
/// c. 親ループを HELLO 受領まで pump（HWND ハンドシェイク・bounded・ハングしない）。
/// d. Shiori3Codec::build_onboot → OnBoot SHIORI/3.0 bytes。
/// e. parent.request（REQUEST 送出 → RESPONSE 再入受領・受け皿セル方式・design.md §209）。
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
    let onboot = shiori3::build_onboot();
    println!("[go(1)] OnBoot 組立 OK（{} バイト）", onboot.len());

    // e. REQUEST 送出 → RESPONSE を受け皿セルで再入受領（design.md §209・要件 2.2/4.3）。
    //    親はここで SendMessageTimeout にブロックし、待機中に helper の RESPONSE を
    //    親 WndProc が再入受領して受け皿セルへ格納する（デッドロック回避・§210）。
    //    ★ 観測記録（design.md §492/§495・research.md §6）:
    //      `parent.request` 直前〜直後の実時間を計測する。ここは SendMessageTimeout で
    //      helper の RESPONSE 再入受領まで**親が同期ブロック**する区間そのもの。非自明な
    //      往復時間の後に**同一同期呼び出し内で**有効な Value が返る＝親は pasta の応答を
    //      待った（GET=block-on-reply）ことの実証（fire-and-forget ではない）。
    let req_started = Instant::now();
    let response = match parent.request(helper_hwnd, &onboot, REQUEST_TIMEOUT) {
        Ok(Some(r)) => r,
        Err(ipc::IpcError::Timeout) => {
            return Err(format!(
                "OnBoot の送りが {REQUEST_TIMEOUT:?} で時間切れ: helper が時間内に REQUEST を捌かなかった\
                 （helper の窓が詰まっている）"
            ));
        }
        Err(ipc::IpcError::SendFailed(code)) => {
            return Err(format!(
                "OnBoot を送れなかった（Win32 エラー {code}）: HELLO の HWND が違うか helper の窓が無い"
            ));
        }
        Ok(None) => {
            return Err(
                "OnBoot は送れたが RESPONSE が受け皿に届かなかった: helper の標準エラーを見る\
                 （pasta の読み込みや request の失敗）。それでなければ再入受領が i686↔x64 で不成立\
                 （design.md §210 Revalidation Trigger＝named pipe 後退を人間判断）"
                    .to_string(),
            );
        }
    };
    let request_roundtrip = req_started.elapsed();
    println!("[go(1)] RESPONSE 再入受領 OK（{} バイト）", response.len());
    println!(
        "[go(1)] REQUEST→RESPONSE 同期往復 {} ms（block-on-reply: 親は pasta 応答まで同期ブロック・design.md §492）",
        request_roundtrip.as_millis()
    );

    // f. Value を parse（design.md §198・要件 4.2）。
    let value = shiori3::parse_value(&response)
        .ok_or_else(|| "RESPONSE に Value: が無い（emo2 OnBoot 応答の parse 失敗）".to_string())?;
    if value.is_empty() {
        return Err("Value: が空（起動挨拶さくらスクリプトが空）".to_string());
    }

    // UNLOAD はここでは送らない。go(2) の生存窓の後に送る（§221）。
    // helper 稼働中のまま session を go(2) へ引き継ぐ。
    let value_bytes = value.len();
    Ok(HelperSession {
        parent,
        handle: Some(handle),
        parent_hwnd,
        helper_hwnd,
        value,
        request_roundtrip,
        response_bytes: response.len(),
        value_bytes,
    })
}

/// go 基準(2) の駆動（要件 1.3/5.2/5.4・design.md §214–232）。
///
/// 前提: `session` は go(1) で確立した稼働中の helper（UNLOAD 未送出）。
/// 手順:
///   1. **生存窓**: helper を N 秒（`SURVIVAL_WINDOW`）生かし、`ProcessHost::poll_exit_kind` で
///      刻みごとに生存（`None`）を確認する。途中で終了していれば go(2) 失敗（loop が破綻＝要件 5.2 不成立）。
///   2. 生存窓の後に UNLOAD を送出（design.md §221「N 秒後」）。
///   3. `ProcessHost::wait_kind` で終わり方を取り、**`ExitKind::Clean`**（終了コード 0）を親が観測する
///      （design.md §230: 終了コード 0 ＋親側観測ログを一次記録・要件 1.3/5.4）。それ以外は失敗。
///
/// 全ステップは bounded: 生存窓は N（`SURVIVAL_WINDOW`）＋刻み、UNLOAD は `UNLOAD_TIMEOUT`。
/// clean unload 待ち（`wait_kind`）は helper が UNLOAD で即 clean 終了するため実質即返り、
/// 最悪でも helper 側 backstop（helper.rs が `run_helper` へ渡す 30s）が上限を保証する（ハングしない）。
fn drive_go_criterion_2(session: &mut HelperSession) -> Result<Go2Record, String> {
    println!("=== go 基準(2): メッセージループ {SURVIVAL_WINDOW:?} 生存 → clean unload を観測 ===");

    // --- 1. 生存窓: N 秒間 helper が生きていることを poll_exit_kind で刻みごとに観測（要件 5.2）---
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
        "[go(2)] helper survived {:.2}s（poll_exit_kind={} 回 すべて alive=None・メッセージループ生存・要件 5.2）",
        survived.as_secs_f64(),
        polls
    );

    // 任意: 生存後もループが REQUEST を捌けることを示す（bounded・失敗しても致命ではない）。
    //   ★ 観測記録: これが返せば「go(1) と go(2) 生存後」の**2 応答**が揃う。
    //     helper 内の pasta actor ループが生存窓を跨いで稼働し続けたことの親可視な代理証拠
    //     （x64 親は helper 内スレッド数を直接観測できないため・design.md §230）。
    let post_survival_response_bytes = mid_window_probe(session).ok();

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
            eprintln!(
                "[go(2)] UNLOAD 送出失敗: {e:?}（helper は上限で抜けて終了コード 4 になり、go 基準(2) は不成立になる）"
            );
        }
    }

    // --- 3. clean unload を待って終了コード 0 を親が観測（要件 1.3/5.4・design.md §230）---
    //     helper 子ハンドルを消費して wait（`session.handle` を take で取り出す）。
    let handle = session
        .handle
        .take()
        .ok_or_else(|| "go 基準(2) 内部エラー: helper handle が既に消費済み".to_string())?;
    match ProcessHost::wait_kind(handle) {
        Ok(kind) => {
            println!("[go(2)] helper exited kind={kind:?}（親が終わり方を観測）");
            if kind.is_clean() {
                println!(
                    "=== go 基準(2) PASS: helper survived {:.2}s → clean unload → exit code=0 kind=Clean を親が観測（要件 1.3/5.2/5.4・design.md §230） ===",
                    survived.as_secs_f64()
                );
                Ok(Go2Record {
                    survived,
                    polls,
                    post_survival_response_bytes,
                    exit_kind: kind,
                })
            } else {
                Err(format!(
                    "go 基準(2) 不成立: helper が clean(0) で終了しなかった（{kind:?}）"
                ))
            }
        }
        Err(e) => Err(format!("go 基準(2) 不成立: clean unload 待ちに失敗（{e}）")),
    }
}

// ============================================================================
// 異常系セルフテスト（design.md §521 IPC / §528 helper 異常終了）
//
// 2 つの異常系を x64 親側で観測する。BOTH must NOT hang（bounded）:
//   err(1) IPC タイムアウト（要件 2.3・design.md §340/§525）
//   err(2) helper 強制終了検出（要件 1.4/2.4・design.md §528/§224–226）
// ============================================================================

/// err(1) の Timeout 観測に使う短いタイムアウト（wedge の 3s より十分短い）。
const ERR_IPC_TIMEOUT: Duration = Duration::from_millis(500);
/// err(1) の wedged 窓が REQUEST 受領時にブロックする時間（Timeout より十分長い）。
const ERR_WEDGE_SLEEP: Duration = Duration::from_secs(3);
/// err(1) の Timeout 判定の上限マージン（bounded ＝ ハングしない・§340）。
/// SendMessageTimeout(500ms) が返るまでを寛容に見積もる。wedge(3s) より十分短ければ合格。
const ERR_TIMEOUT_UPPER_BOUND: Duration = Duration::from_millis(1500);
/// err(1) の Timeout 判定の下限（`ERR_IPC_TIMEOUT` − 100 ms）。これより早い Timeout は上限を守った証拠にならない。
const ERR_TIMEOUT_LOWER_BOUND: Duration = Duration::from_millis(400);
/// err(2) で helper の HELLO を待つ上限（起動確認・bounded）。
const ERR_HELLO_TIMEOUT: Duration = Duration::from_secs(5);

/// 異常系 2 種を続けて観測する。両方 PASS で `Ok(())`。
fn drive_selftest_errors(ghostdir: &Path) -> Result<(), String> {
    drive_err_ipc_timeout()?;
    drive_err_helper_abnormal_exit(ghostdir)?;
    Ok(())
}

/// err(1) IPC タイムアウト（要件 2.3・design.md §340/§525）。
///
/// 別スレッド上に **無応答（wedged）** message-only 窓を立て、その WndProc が REQUEST 受領時に
/// `ERR_WEDGE_SLEEP`(3s) スリープしてブロックする（＝無応答 helper の模擬）。main スレッドから
/// 短い `ERR_IPC_TIMEOUT`(500ms) で `ipc::send_copydata` を撃ち、`Err(IpcError::Timeout)` が
/// **タイムアウト時間で**返ること（wedge の 3s を待たずに abort＝ハングしない）を観測する。
///
/// SendMessageTimeout は**別スレッド**宛なら caller を実際にブロックし `timeout_ms` を honor する
/// （同一スレッド宛は deadlock ゆえ別スレッドの wedged 窓を用いる）。SMTO_ABORTIFHUNG
/// ＋ timeout_ms が wedge を on-time に打ち切る（ipc.rs §155–206）。
fn drive_err_ipc_timeout() -> Result<(), String> {
    use std::sync::mpsc;

    println!("--- err(1) IPC タイムアウト観測（要件 2.3・design.md §340/§525）---");

    // wedged 窓を別スレッドで生成し、その HWND(u32) を main へ返す。窓・ループはそのスレッドに
    // 閉じる（窓アフィニティ）。REQUEST 受領で WndProc が 3s スリープ＝無応答を模擬する。
    let (hwnd_tx, hwnd_rx) = mpsc::channel::<u32>();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_thread = stop.clone();

    let wedge = std::thread::spawn(move || {
        // WndProc: REQUEST(WM_COPYDATA) を受けたら wedge_sleep ブロックして応答しない（無応答）。
        let window = match wintf_winmsg_executor::util::Window::new_checked(
            wintf_winmsg_executor::util::WindowType::MessageOnly,
            (),
            move |_state: Pin<&()>,
                  msg: wintf_winmsg_executor::util::WindowMessage|
                  -> Option<windows::Win32::Foundation::LRESULT> {
                if msg.msg == windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA {
                    // 無応答 helper の模擬: REQUEST を捌かず長時間ブロックする。
                    // caller（別スレッドの SendMessageTimeout）は SMTO_ABORTIFHUNG＋timeout で
                    // これを待たずに打ち切る（＝ハングしない）。
                    std::thread::sleep(ERR_WEDGE_SLEEP);
                    return Some(windows::Win32::Foundation::LRESULT(0));
                }
                None
            },
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[err(1)] wedged 窓生成に失敗: {e:?}");
                return;
            }
        };
        // HWND を main へ渡す。
        let hwnd_u32 = ipc::hwnd_to_u32(window.hwnd());
        let _ = hwnd_tx.send(hwnd_u32);

        // stop まで窓スレッドでメッセージループを回す（WndProc が呼ばれるよう）。
        // main が stop を立ててから 1 回投函する WM_NULL で起き、stop を見て抜ける。
        wintf_winmsg_executor::MessageLoop::run(|msg_loop, _msg| {
            if stop_thread.load(std::sync::atomic::Ordering::Relaxed) {
                msg_loop.quit();
            }
            wintf_winmsg_executor::FilterResult::Forward
        });
        drop(window);
    });

    // wedged 窓の HWND を受け取る（bounded・起動失敗で即エラー）。
    let wedge_hwnd_u32 = hwnd_rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "wedged 窓の HWND を受領できなかった（窓生成失敗の可能性）".to_string())?;
    let wedge_hwnd = ipc::hwnd_from_u32(wedge_hwnd_u32);
    println!(
        "[err(1)] wedged 窓（別スレッド・REQUEST で {ERR_WEDGE_SLEEP:?} ブロック）: hwnd(u32)={wedge_hwnd_u32:#010x}"
    );

    // wedged 窓のループが回り始めるまで軽く待つ。
    // ここは短い固定待ち（bounded）で十分。
    std::thread::sleep(Duration::from_millis(100));

    // self_hwnd は形式上の送信元（wedged 窓は使わない）。適当な非 0 でよいが、正当性のため
    // wedge_hwnd を流用（WPARAM に載るだけ・応答は来ないので無関係）。
    let self_hwnd = wedge_hwnd;

    // ★ 観測: 短い timeout で send_copydata。wedge(3s) を待たず timeout(500ms) 付近で
    //   Err(Timeout) が返ること＝ハングしない（要件 2.3・design.md §340）。
    let started = Instant::now();
    let result = ipc::send_copydata(
        wedge_hwnd,
        self_hwnd,
        MsgTag::Request,
        b"GET SHIORI/3.0\r\nID: OnBoot\r\n\r\n",
        ERR_IPC_TIMEOUT,
    );
    let elapsed = started.elapsed();

    // wedge スレッドを停止させる（後始末）。MessageLoop はメッセージ駆動ゆえ、stop を立てた後に
    // wedged 窓へ 1 回 PostMessage を撃って起こす。
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(wedge_hwnd),
            0, // WM_NULL
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );
    }
    let _ = wedge.join();

    // 判定: Err(Timeout) かつ 下限 ≦ elapsed ≦ 上限（timeout 近傍・wedge の 3s より十分短い）。
    match result {
        Err(ipc::IpcError::Timeout) => {
            if elapsed < ERR_TIMEOUT_LOWER_BOUND {
                return Err(format!(
                    "err(1) 不成立: Timeout は返ったが elapsed={elapsed:?} が下限 {ERR_TIMEOUT_LOWER_BOUND:?} 未満（上限まで待たずに返った）"
                ));
            }
            if elapsed > ERR_TIMEOUT_UPPER_BOUND {
                return Err(format!(
                    "err(1) 不成立: Timeout は返ったが elapsed={elapsed:?} が上限 {ERR_TIMEOUT_UPPER_BOUND:?} を超過（bounded でない）"
                ));
            }
            println!(
                "[err(1)] IPC timeout observed in {}ms (timeout={:?}, wedge={:?}, bounded, no hang) PASS",
                elapsed.as_millis(),
                ERR_IPC_TIMEOUT,
                ERR_WEDGE_SLEEP
            );
            Ok(())
        }
        Err(other) => Err(format!(
            "err(1) 不成立: 期待は IpcError::Timeout だが {other:?} が返った（elapsed={elapsed:?}）"
        )),
        Ok(()) => Err(format!(
            "err(1) 不成立: 無応答 wedge への送りが通った（elapsed={elapsed:?}・Timeout であるべき）"
        )),
    }
}

/// err(2) helper 強制終了検出（要件 1.4/2.4・design.md §528/§224–226 Crashed 遷移）。
///
/// 実 i686 helper を `ProcessHost::spawn` で起動し、HELLO 到達で稼働を確認した後、
/// `ProcessHost::terminate`（`Child::kill()`）で**強制終了**する。その後 `wait_kind` で
/// 親が終了種別を観測し、**`ExitKind::Clean` ではない**（`Terminated` または非ゼロ `Abnormal`）
/// ＝異常終了を検出できること（clean unload の exit 0 と明確に区別）を確認する。すべて bounded。
fn drive_err_helper_abnormal_exit(ghostdir: &Path) -> Result<(), String> {
    println!("--- err(2) helper 強制終了検出（要件 1.4/2.4・design.md §528）---");

    let helper_exe = match helper_exe_from_process() {
        Some(p) => p,
        None => {
            return Err(
                "HELPER_EXE 未設定かつ argv に helper exe なし。err(2) には実 i686 helper が必要（$env:HELPER_EXE を設定）".to_string(),
            );
        }
    };
    println!("[err(2)] helper_exe = {}", helper_exe.display());

    // 親 message-only 窓（HELLO 受け皿）を立て、helper を起動する。
    let parent = ParentMessageWindow::create()?;
    let parent_hwnd_u32 = parent.hwnd_u32();
    let mut handle = ProcessHost::spawn(&helper_exe, ghostdir, parent_hwnd_u32)
        .map_err(|e| format!("helper 起動失敗: {e}"))?;
    println!("[err(2)] helper 起動 OK（HELLO を待機・稼働確認）");

    // HELLO 受領で稼働確認（bounded）。届かなくても kill は可能だが、稼働状態からの
    // 強制終了を観測するため HELLO を待つ。
    match parent.pump_until_hello_or(ERR_HELLO_TIMEOUT) {
        Some(h) => {
            println!("[err(2)] helper 稼働確認 OK（HELLO 受領・helper hwnd(u32)={h:#010x}）")
        }
        None => {
            // 稼働確認できずとも観測は続行（強制終了自体は成立する）。ただし記録する。
            eprintln!(
                "[err(2)] HELLO を {ERR_HELLO_TIMEOUT:?} 内に受領できず（稼働確認省略・kill は続行）"
            );
        }
    }

    // 起動直後に既に落ちていないことを確認（落ちていたら kill の意味がない）。
    if let Some(kind) = ProcessHost::poll_exit_kind(&mut handle) {
        return Err(format!(
            "err(2) 準備失敗: kill 前に helper が既に終了していた（{kind:?}）"
        ));
    }

    // ★ 強制終了（TerminateProcess 相当・要件 1.4 の「予期せぬ終了」を模擬）。
    let killed_at = Instant::now();
    ProcessHost::terminate(&mut handle).map_err(|e| format!("helper の強制終了に失敗: {e}"))?;
    println!("[err(2)] helper を強制終了（Child::kill）");

    // 親が終了種別を観測する（bounded＝wait は kill 済みゆえ即返る）。
    let kind = ProcessHost::wait_kind(handle).map_err(|e| format!("終了待ちに失敗: {e}"))?;
    let detect_elapsed = killed_at.elapsed();

    // 判定: clean(0) では**ない**こと＝異常を検出できた（要件 1.4「予期せぬ終了の検出・記録」）。
    if kind.is_clean() {
        return Err(format!(
            "err(2) 不成立: 強制終了したのに ExitKind::Clean と観測された（異常検出できず）"
        ));
    }
    let code_str = match kind {
        ExitKind::Abnormal(c) => format!("{c}"),
        ExitKind::Terminated => "none(Terminated)".to_string(),
        ExitKind::Clean => unreachable!(),
    };
    println!(
        "[err(2)] force-killed helper detected: kind={kind:?} code={code_str} (detected in {}ms, bounded, no hang) PASS",
        detect_elapsed.as_millis()
    );
    drop(parent);
    Ok(())
}

/// 生存窓の後、helper のメッセージループがまだ REQUEST を捌けることを示す任意プローブ（bounded）。
/// 失敗しても go(2) は致命扱いしない（生存＋clean unload が本質・これは「まだ動く」補足観測）。
fn mid_window_probe(session: &HelperSession) -> Result<usize, ()> {
    let onboot = shiori3::build_onboot();
    match session
        .parent
        .request(session.helper_hwnd, &onboot, REQUEST_TIMEOUT)
    {
        Ok(Some(resp)) => {
            println!(
                "[go(2)] 生存後の追加 REQUEST に helper が応答（{} バイト）＝ループは生存後も稼働中",
                resp.len()
            );
            Ok(resp.len())
        }
        Ok(None) => {
            eprintln!("[go(2)] 追加 REQUEST は送れたが応答なし（補足観測のみ・致命ではない）");
            Err(())
        }
        Err(e) => {
            eprintln!("[go(2)] 追加 REQUEST は失敗（補足観測のみ・致命ではない）: {e:?}");
            Err(())
        }
    }
}
