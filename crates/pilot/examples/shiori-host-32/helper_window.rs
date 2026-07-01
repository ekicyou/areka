//! host-32 先進坑: **HelperMessageWindow**（design.md §415–443・i686 helper 専用）。
//!
//! `wintf-winmsg-executor` 0.0.5 で **message-only 窓**を生成し、メッセージループを回す
//! （窓持ち SHIORI 対応・要件 5.1）。起動時に親へ HELLO（自 HWND を u32 LE）を 1st
//! WM_COPYDATA で送り（design.md §424）、WndProc で REQUEST を受領して `ShioriByteProxy`
//! を駆動、応答を 2nd WM_COPYDATA で親へ返す（design.md §425）。UNLOAD 受領でループを
//! 停止し clean unload する（要件 5.2, 5.3・go 基準(2)＝design.md §214–232）。
//!
//! ## 本ファイルが担う範囲（HELPER 側のみ・境界 = HelperMessageWindow）
//! 親（x64）側の driver / parent message window / 親駆動の完全な往復（task 4.1）は**対象外**。
//! 本ファイルは helper 側の窓・WndProc・ループ生存＋清掃終了と、それを**単独で**実証する
//! **in-process loopback セルフテスト**（stand-in parent 窓が HELLO を実 WM_COPYDATA で受領・
//! helper HWND を復号確認 → N 秒ループ生存 → UNLOAD 清掃終了）までを持つ。
//!
//! ## デッドロック回避（design.md §210）
//! WndProc は REQUEST に対し **単一の応答**（RESPONSE を `send_copydata` で 1 通）を返すだけで、
//! それ以上の跨プロセス `SendMessage` を発行せず即 return する。往復は single-in-flight
//! （REQUEST 待ち ⊃ RESPONSE 受領 WndProc）で厳密な入れ子となり、循環待ちが構造的に発生しない。
//!
//! ## pasta を起動時に eager ロードする選択（design.md §425 の前提）
//! REQUEST を WndProc 内で即座に処理できるよう、pasta.dll は**窓生成前に eager ロード**する
//! （`load_dll` + `shiori_load(ghostdir)`）。REQUEST ごとの遅延ロードは往復レイテンシと
//! 状態機械（Started→Pumping）を複雑にするため採らない。UNLOAD/終了時に `shiori_unload` →
//! `FreeLibrary`（`ShioriEntries` Drop）で清掃する。
//!
//! ## リスク（design.md §443・本先進坑で初確認）
//! `wintf-winmsg-executor` の **i686 実行時挙動**はビルド実証済だが実走は本坑で初。窓生成/
//! ループが i686 実機で回らなければ raw Win32 ループへの後退（Revalidation Trigger）を人間が
//! 判断する。本ファイルは後退実装を持たず、観測結果を正直に出す（`--selftest-loop` 参照）。

#![allow(dead_code)] // 一部フィールド/経路は selftest・後続タスクからのみ使用。

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Duration, Instant};

use wintf_winmsg_executor::util::{Window, WindowMessage, WindowType};
use wintf_winmsg_executor::{FilterResult, MessageLoop};
use windows::Win32::Foundation::{HWND, LRESULT};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::WM_COPYDATA;

use crate::ipc::{self, MsgTag};
use crate::shiori_proxy::{ShioriByteProxy, ShioriEntries};

/// helper 状態機械（design.md §436）。`Started → Pumping → Unloading → CleanExit`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperState {
    Started,
    Pumping,
    Unloading,
    CleanExit,
}

/// WndProc と外側（ループ制御・観測）で共有する helper 窓の状態。
///
/// `wintf-winmsg-executor` の `Window<S>` は `state: S` を窓と同居させ、WndProc へ
/// `Pin<&S>` で渡す（donor 参照・GWLP_USERDATA 手詰め不要）。single-in-flight ゆえ
/// 内部可変は `Cell`/`RefCell` で足りる（reader スレッド不要・design.md §438）。
pub struct HelperShared {
    /// 親のメッセージ窓 HWND（u32 ワイヤ値）。HELLO 送出先＆RESPONSE 返送先。
    parent_hwnd: u32,
    /// eager ロード済み pasta エントリ。Drop で `FreeLibrary`（design.md §425 の前提）。
    entries: RefCell<Option<ShioriEntries>>,
    /// UNLOAD を受領したら true。外側ループがこれを見て `msg_loop.quit()` する。
    want_quit: Cell<bool>,
    /// 状態機械の現在値（観測用）。
    state: Cell<HelperState>,
    /// 観測カウンタ: 受領した REQUEST 数。
    requests_handled: Cell<u64>,
    /// 観測カウンタ: 返送した RESPONSE 数。
    responses_sent: Cell<u64>,
    /// 観測カウンタ: 未知タグ受領数（crash させず記録・design.md §425）。
    unknown_tags: Cell<u64>,
    /// 応答送出タイムアウト（既定 = `ipc::DEFAULT_TIMEOUT`）。
    reply_timeout: Duration,
}

impl HelperShared {
    fn new(parent_hwnd: u32, entries: Option<ShioriEntries>) -> Self {
        Self {
            parent_hwnd,
            entries: RefCell::new(entries),
            want_quit: Cell::new(false),
            state: Cell::new(HelperState::Started),
            requests_handled: Cell::new(0),
            responses_sent: Cell::new(0),
            unknown_tags: Cell::new(0),
            reply_timeout: ipc::DEFAULT_TIMEOUT,
        }
    }
}

/// 受領した `WM_COPYDATA` の生バイト payload を安全に取り出す（`cbData` で境界画定・要件 2.1）。
///
/// # Safety
/// `lparam` は WM_COPYDATA の LPARAM で、`*const COPYDATASTRUCT` を指していること
/// （OS が WM_COPYDATA 配送時に保証）。`lpData` は `cbData` バイト有効。
unsafe fn copydata_payload(lparam: windows::Win32::Foundation::LPARAM) -> Option<(u32, Vec<u8>)> {
    let cds = lparam.0 as *const COPYDATASTRUCT;
    if cds.is_null() {
        return None;
    }
    // SAFETY: WM_COPYDATA 契約により lparam は有効な COPYDATASTRUCT を指す。
    let cds = unsafe { &*cds };
    // dwData は低 32bit のみ有意（ipc.rs §1 の跨ビットネス規約）。
    let tag_raw = (cds.dwData & 0xFFFF_FFFF) as u32;
    let len = cds.cbData as usize;
    let payload = if len == 0 || cds.lpData.is_null() {
        Vec::new()
    } else {
        // SAFETY: lpData は cbData バイト有効（OS が marshal 済みの受信側コピー）。
        unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, len).to_vec() }
    };
    Some((tag_raw, payload))
}

/// WndProc: WM_COPYDATA を tag で分岐する（design.md §425）。非ブロッキング（応答は 1 通のみ）。
///
/// `sender_hwnd` は WM_COPYDATA の WPARAM（送信元窓 HWND・`send_copydata` が self_hwnd を載せる）。
/// RESPONSE は原則 `parent_hwnd`（起動時に確定）へ返すが、sender が有効ならそれを優先する。
fn handle_message(s: &HelperShared, msg: &WindowMessage) -> Option<LRESULT> {
    if msg.msg != WM_COPYDATA {
        return None; // 非対象は DefWindowProc へ（closure が None を返すと lib が委譲）。
    }

    // SAFETY: WM_COPYDATA の lparam は COPYDATASTRUCT を指す（OS 契約）。
    let (tag_raw, payload) = match unsafe { copydata_payload(msg.lparam) } {
        Some(v) => v,
        None => return Some(LRESULT(0)),
    };

    match MsgTag::try_from_u32(tag_raw) {
        Ok(MsgTag::Request) => {
            s.state.set(HelperState::Pumping);
            s.requests_handled.set(s.requests_handled.get() + 1);
            // REQUEST を pasta へ駆動（eager ロード済み entries）。
            let response = {
                let guard = s.entries.borrow();
                match guard.as_ref() {
                    Some(e) => ShioriByteProxy::shiori_request(e, &payload).ok(),
                    None => None,
                }
            };
            // RESPONSE を親へ 1 通だけ返す（それ以上の跨プロセス SendMessage 不可・§210）。
            if let Some(bytes) = response {
                let target = ipc::hwnd_from_u32(s.parent_hwnd);
                let self_hwnd = msg.hwnd;
                match ipc::send_copydata(
                    target,
                    self_hwnd,
                    MsgTag::Response,
                    &bytes,
                    s.reply_timeout,
                ) {
                    Ok(()) => {
                        s.responses_sent.set(s.responses_sent.get() + 1);
                    }
                    Err(e) => {
                        eprintln!("[helper] RESPONSE 送出失敗: {e:?}");
                    }
                }
            } else {
                eprintln!("[helper] REQUEST 処理失敗（応答なし・観測点）");
            }
            Some(LRESULT(0))
        }
        Ok(MsgTag::Unload) => {
            // ループ停止指示。実 unload は外側（run_helper_loop 後）で清掃する。
            s.state.set(HelperState::Unloading);
            s.want_quit.set(true);
            Some(LRESULT(0))
        }
        Ok(MsgTag::Load) => {
            // 本先進坑では pasta を eager ロード済みゆえ LOAD は no-op（観測のみ）。
            Some(LRESULT(0))
        }
        Ok(other) => {
            // HELLO/RESPONSE は helper が受ける想定外（親 → helper では来ない）。記録のみ。
            eprintln!("[helper] 想定外タグ受領（無視）: {other:?}");
            Some(LRESULT(0))
        }
        Err(raw) => {
            // 未知タグは crash させず記録（design.md §425・観測点）。
            s.unknown_tags.set(s.unknown_tags.get() + 1);
            eprintln!("[helper] 未知タグ受領（無視）: {raw:#x}");
            Some(LRESULT(0))
        }
    }
}

/// HelperMessageWindow のハンドル。窓（`Window<Rc 不要・S=HelperShared>`）を保持し、
/// Drop で窓破棄。`shared()` で観測状態へアクセスできる。
pub struct HelperMessageWindow {
    window: Window<HelperShared>,
}

impl HelperMessageWindow {
    /// message-only 窓を生成し、起動時に親へ HELLO（自 HWND を u32 LE）を送る（要件 5.1）。
    ///
    /// `entries` は eager ロード済み pasta（REQUEST 駆動用）。`None` でも窓は立つ
    /// （REQUEST 時に応答なしとして観測）。
    pub fn create(parent_hwnd: u32, entries: Option<ShioriEntries>) -> Result<Self, String> {
        let shared = HelperShared::new(parent_hwnd, entries);
        // new_checked = FnMut closure を RefCell で再入ガード（donor 参照）。message-only。
        let window = Window::new_checked(
            WindowType::MessageOnly,
            shared,
            move |state: Pin<&HelperShared>, msg: WindowMessage| -> Option<LRESULT> {
                handle_message(state.get_ref(), &msg)
            },
        )
        .map_err(|e| format!("message-only 窓の生成に失敗: {e:?}"))?;

        // 起動時 HELLO（1st WM_COPYDATA・自 HWND を u32 LE・design.md §424）。
        // SendMessageTimeout は同スレッド宛なら同期配送（親が別スレ/別プロセスでもタイムアウト保護）。
        let self_hwnd: HWND = window.hwnd();
        let hello_payload = ipc::encode_hwnd_le(self_hwnd);
        let target = ipc::hwnd_from_u32(parent_hwnd);
        if let Err(e) = ipc::send_copydata(
            target,
            self_hwnd,
            MsgTag::Hello,
            &hello_payload,
            ipc::DEFAULT_TIMEOUT,
        ) {
            // HELLO 送出失敗は起動失敗（親が helper HWND を得られない）。観測して返す。
            return Err(format!("HELLO 送出に失敗: {e:?}"));
        }
        window.state().state.set(HelperState::Pumping);
        Ok(Self { window })
    }

    pub fn hwnd(&self) -> HWND {
        self.window.hwnd()
    }

    pub fn shared(&self) -> Pin<&HelperShared> {
        self.window.state()
    }

    /// UNLOAD 受領（`want_quit`）まで、または `max_run` 経過までメッセージループを回す
    /// （要件 5.1/5.2）。戻り時は窓は生存（Drop は呼び出し側）。
    ///
    /// `MessageLoop::run` のフィルタクロージャで毎メッセージ `want_quit`／経過時間を検査し、
    /// 満たせば `msg_loop.quit()`。窓が無入力で沈黙してもタイムアウトで抜けられるよう、
    /// 別スレッドから自窓へ定期 WM_NULL を撃ってループを起こす（`heartbeat`）。
    pub fn run_until_unload_or(&self, max_run: Duration) {
        let deadline = Instant::now() + max_run;
        let want_quit_ptr = self.window.state();

        // ループが GetMessage でブロックし続けないよう、定期的に自窓へメッセージを撃つ
        // ハートビート（`want_quit`／deadline を必ず再評価させる）。别スレッドから PostMessage。
        let hb_hwnd_u32 = ipc::encode_hwnd_le(self.window.hwnd());
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let heartbeat = {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let hwnd = ipc::hwnd_from_u32(u32::from_le_bytes(hb_hwnd_u32));
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    // WM_NULL(0) は無害な起こし用メッセージ。PostMessage で loop を起こすだけ。
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            Some(hwnd),
                            0, // WM_NULL
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            })
        };

        MessageLoop::run(|msg_loop, _msg| {
            if want_quit_ptr.want_quit.get() || Instant::now() >= deadline {
                msg_loop.quit();
            }
            FilterResult::Forward
        });

        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = heartbeat.join();
    }

    /// clean unload（要件 5.3）: pasta を `shiori_unload` → `FreeLibrary`（entries Drop）。
    /// 状態を `CleanExit` へ。窓自体は `self` Drop で破棄される。
    pub fn clean_unload(&self) {
        let s = self.window.state();
        if let Some(e) = s.entries.borrow().as_ref() {
            let _ = ShioriByteProxy::shiori_unload(e);
        }
        // entries を drop して FreeLibrary（Drop 実装）。
        *s.entries.borrow_mut() = None;
        s.state.set(HelperState::CleanExit);
    }
}

/// 本番 helper 実行経路（親が起動・design.md §415–443）。pasta を eager ロードし、窓を立て
/// HELLO を送り、UNLOAD までループを回して clean unload する。戻り値 = プロセス終了コード。
///
/// `parent_hwnd` は親から受けた u32 ワイヤ値。`ghostdir` は pasta.dll の在処。
pub fn run_helper(parent_hwnd: u32, ghostdir: &Path, max_run: Duration) -> i32 {
    let dll = ghostdir.join("pasta.dll");
    let entries = match ShioriByteProxy::load_dll(&dll) {
        Ok(e) => match ShioriByteProxy::shiori_load(&e, ghostdir) {
            Ok(()) => Some(e),
            Err(err) => {
                eprintln!("[helper] shiori_load 失敗（観測）: {err:?}");
                Some(e) // 窓は立てて HELLO は送る（生存監視の対象）。REQUEST は失敗観測。
            }
        },
        Err(err) => {
            eprintln!("[helper] pasta.dll ロード失敗（観測）: {err:?}");
            None
        }
    };

    let win = match HelperMessageWindow::create(parent_hwnd, entries) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[helper] 窓生成/HELLO 失敗: {e}");
            return 2;
        }
    };
    win.run_until_unload_or(max_run);
    win.clean_unload();
    0
}

/// helper `main()` から呼べる **in-process loopback セルフテスト**（go 基準(2) helper 側の核）。
///
/// 実クロスプロセス親（task 4.1）抜きで、HELLO 送受信・N 秒ループ生存・清掃終了を単独実証する:
///   1. stand-in parent（message-only 窓）を立て、HELLO を実 WM_COPYDATA で受領・記録する。
///   2. HelperMessageWindow を生成 → HELLO（helper 自 HWND を u32 LE）を stand-in parent へ送る。
///   3. stand-in parent が HELLO 受領＋helper HWND を復号一致することを確認（要件 5.1）。
///   4. ループを `secs` 秒回して破綻せず生存することを確認（要件 5.2）。
///   5. UNLOAD 相当（want_quit）でループ停止＋清掃終了を確認（要件 5.3 → exit 0）。
///
/// 標準出力に観測を出し、全確認 OK なら `Ok(())`（`main` が exit 0）。
pub fn selftest_loop(secs: u64) -> Result<(), String> {
    // --- 1. stand-in parent（HELLO 受け皿）を立てる。受領 HWND を Cell に記録 ---
    let received_hello: std::rc::Rc<Cell<Option<u32>>> = std::rc::Rc::new(Cell::new(None));
    let parent = {
        let received = received_hello.clone();
        Window::new_checked(
            WindowType::MessageOnly,
            (),
            move |_state: Pin<&()>, msg: WindowMessage| -> Option<LRESULT> {
                if msg.msg == WM_COPYDATA {
                    // SAFETY: WM_COPYDATA 契約。
                    if let Some((tag_raw, payload)) = unsafe { copydata_payload(msg.lparam) } {
                        if MsgTag::try_from_u32(tag_raw) == Ok(MsgTag::Hello) && payload.len() == 4 {
                            let bytes = [payload[0], payload[1], payload[2], payload[3]];
                            let helper_hwnd_u32 = ipc::decode_hwnd_le(bytes);
                            received.set(Some(helper_hwnd_u32));
                        }
                    }
                    return Some(LRESULT(0));
                }
                None
            },
        )
        .map_err(|e| format!("stand-in parent 窓生成に失敗: {e:?}"))?
    };
    let parent_hwnd_u32 = ipc::encode_hwnd_le(parent.hwnd());
    let parent_hwnd_u32 = u32::from_le_bytes(parent_hwnd_u32);
    println!("[selftest] stand-in parent 窓 生成: hwnd(u32)={parent_hwnd_u32:#010x}");

    // --- 2. HelperMessageWindow を生成（HELLO は create 内で送出）---
    // pasta は selftest では不要（HELLO＋ループ生存の実証が目的）ゆえ entries=None。
    let helper = HelperMessageWindow::create(parent_hwnd_u32, None)
        .map_err(|e| format!("HelperMessageWindow 生成に失敗: {e}"))?;
    let helper_hwnd_u32 = u32::from_le_bytes(ipc::encode_hwnd_le(helper.hwnd()));
    println!("[selftest] HelperMessageWindow 生成: hwnd(u32)={helper_hwnd_u32:#010x}");

    // --- 3. HELLO 受領＋helper HWND 復号一致を確認（要件 5.1）---
    match received_hello.get() {
        Some(got) if got == helper_hwnd_u32 => {
            println!(
                "[selftest] HELLO 受領 OK: stand-in parent が helper HWND を復号一致 ({got:#010x}) \
                 = 窓生成＋HELLO 実 WM_COPYDATA 往復成功（要件 5.1）"
            );
        }
        Some(got) => {
            return Err(format!(
                "HELLO で復号した helper HWND 不一致: got={got:#010x} expected={helper_hwnd_u32:#010x}"
            ));
        }
        None => {
            return Err("stand-in parent が HELLO を受領しなかった（要件 5.1 不成立）".to_string());
        }
    }

    // --- 4. ループを secs 秒回して生存を確認（要件 5.2）---
    println!("[selftest] メッセージループを {secs} 秒運転（生存確認・要件 5.2）...");
    let started = Instant::now();
    helper.run_until_unload_or(Duration::from_secs(secs));
    let elapsed = started.elapsed();
    // deadline 到達（UNLOAD 未送出）ゆえ elapsed ≳ secs。破綻なくここへ戻れた＝ループ生存。
    if elapsed < Duration::from_secs(secs).saturating_sub(Duration::from_millis(500)) {
        return Err(format!(
            "ループが {secs}s 未満で戻った（生存せず・elapsed={elapsed:?}）"
        ));
    }
    println!(
        "[selftest] ループ生存 OK: {elapsed:?} 破綻なく回り続けた（要件 5.2・crash/deadlock なし）"
    );

    // --- 5. UNLOAD 相当でループ停止＋清掃終了（要件 5.3）---
    // want_quit を立て、ごく短時間もう一度ループへ入り即 quit を観測（UNLOAD → 停止の実演）。
    helper.shared().want_quit.set(true);
    helper.run_until_unload_or(Duration::from_secs(2)); // want_quit ゆえ即 quit で戻るはず。
    helper.clean_unload();
    if helper.shared().state.get() != HelperState::CleanExit {
        return Err("clean_unload 後の状態が CleanExit でない".to_string());
    }
    println!(
        "[selftest] UNLOAD → ループ停止 → clean unload OK（要件 5.3・state=CleanExit）"
    );

    // 窓は helper/parent の Drop で破棄。
    drop(helper);
    drop(parent);
    println!("[selftest] 窓破棄・清掃終了 OK（exit 0）");
    Ok(())
}

/// selftest 用の既定 ghostdir（本番 run_helper では親が env/arg で渡す・process_host.rs 参照）。
pub fn default_fixture_ghostdir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("shiori-host-32")
        .join("fixtures")
        .join("emo2")
        .join("ghost")
        .join("master")
}
