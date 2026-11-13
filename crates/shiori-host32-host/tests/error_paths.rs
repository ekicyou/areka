//! エラー経路の統合テスト（task 5.2・design.md Testing Strategy / Integration Tests 2・3・4・5）。
//!
//! 3 経路それぞれが**観測可能な形で失敗を報告**し、**親が無限待機しない**（すべて bounded）
//! ことを、公開 API のみで検証する（`#[cfg(test)]` 限定の内部観測子には依存しない）:
//!
//! 1. **ハンドシェイク timeout（要件 3.4）**: helper を spawn せず HELLO も送らない親窓で
//!    `pump_until_hello_or(短 timeout)` が `None` を返し、上限時間で bounded に抜ける。
//! 2. **応答 timeout / wedge（要件 5.2 / 5.3）**: ハンドシェイクを（loopback で）成立させた後、
//!    **応答しない相手**（親自身の HWND＝REQUEST に RESPONSE を返さない）へ `send_request` すると
//!    上限時間で `SendError::Ipc(IpcError::Timeout)` を返し、親がハングしない。
//! 3. （消した: helper の強制終了の見つけ方は `lifecycle_kill_e2e` が本物の握手で確かめ、helper が
//!    自分で終わる道の記録は `process_host_stderr_tests::real_helper_stderr_reaches_the_sink` が受ける）
//! 4. **不正フレーム隔離（要件 2.5）**: 親窓へ**実 WM_COPYDATA** で「未知タグ」フレームを
//!    **ハンドシェイク成立前**（helper_hwnd 未確定）に送り、破損フレームが HELLO と誤認されて
//!    helper_hwnd を確定させることが無い＝上位（ハンドシェイク／`ResponseSlot`）へ渡らないことを、
//!    公開 API のみで**非盲目**に観測する: 不正フレーム送出**後**でも `pump_until_hello_or(短 timeout)`
//!    が依然 `None`（＝不正フレームは HELLO と誤認されず helper_hwnd を確定させない）を返す。
//!    さらに、その不正フレーム送出後でも正当な loopback HELLO を続けて送ればハンドシェイクが成立し得る
//!    （窓が生きている＝crash していない）ことまで確認し、隔離と生存の両面を示す。
//!    framing 関数の単体テストでは覆えない **WndProc 受信経路の隔離**を実配送で確かめる。
//!
//! ### 撤回した検証（cbData 不整合の実配送注入）と単体被覆への委譲
//! 前ラウンドは「`cbData` と実長 不整合」フレームを `SendMessageW` で注入して WndProc 隔離を
//! 検証しようとしたが、これは 2 つの理由で**撤回**した:
//! 1. **UB**: `cbData`(宣言長) を実バッファ長より大きく詐称すると、受信側 `read_copydata`
//!    （`src/parent_window.rs`）が `from_raw_parts(lpData, cbData)` で**境界外読み取り（未定義動作）**を
//!    起こす。テストに UB を残さない。
//! 2. **構造上到達不能**: `read_copydata` は `cbData` バイトちょうどを slice して `classify_inbound`
//!    へ渡すため、実受信経路では常に `declared_len == data.len()` となり、`copydata_payload` の
//!    `LengthMismatch` 分岐は**実 WM_COPYDATA 受信では原理的に発火しない**（発火させるには上記 UB が
//!    必要）。よって「実配送で cbData 不整合を隔離検証する」ことは構造上できない。
//!
//! `cbData`/実長 不整合の検出は**単体レベルで既に被覆済み**であり、本統合テストはそこへ委譲する:
//! - proto 単体 `shiori-host32-ipc::framing_tests::framing_rejects_length_mismatch`
//!   （純関数 `copydata_payload` の `LengthMismatch` 分岐を直接検証）。
//! - host 単体 `shiori-host32-host::parent_window::classify_tests::length_mismatch_is_ignored_as_bad`
//!   （WndProc 判定ロジック `classify_inbound` が長さ不整合を `IgnoreBad` にすることを検証）。
//!
//! 本統合テストは**到達可能な不正フレーム＝未知タグ**の WndProc 隔離を実配送検証し、長さ不整合は
//! 上記単体被覆に委ねる。
//!
//! 窓が要る経路（1・2・4）は 1 つのテスト関数で 1 つの親窓を使い、順に検証する。
//!
//! ## 実行（PowerShell・Git Bash 不可）
//! ```powershell
//! cargo test -p shiori-host32-host --test error_paths
//! ```

use std::time::Duration;

use shiori_host32_host::{ParentMessageWindow, SendError};
use shiori_host32_ipc::{IpcError, MsgTag, hwnd_from_u32, send_copydata};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_COPYDATA};

mod common;
use common::HANDSHAKE_TIMEOUT;

/// 任意 `dw_data`（種別タグ生値）の実 WM_COPYDATA フレームを `SendMessageW` で `target` へ
/// **同期配送**する（**未知タグ**の不正フレームを実配送するために使う・要件 2.5 の WndProc 隔離検証）。
///
/// proto の `send_copydata` は既知 `MsgTag` しか載せられないため、未知タグ（例 `0xDEAD`）の
/// フレームを注入するにはこの生ヘルパが要る。
///
/// **UB 回避（重要）**: `cbData` は**必ず `data.len()`（実バッファ長）ちょうど**に設定する。
/// これにより受信側 `read_copydata`（`src/parent_window.rs`）が `from_raw_parts(lpData, cbData)` を
/// 呼んでも境界外読み取りが起きない。`cbData` を実長より大きく詐称する（＝長さ不整合注入）ことは
/// **意図的に行わない**（UB を招き、かつ実受信経路では `LengthMismatch` が原理的に発火しないため。
/// ファイル冒頭「撤回した検証」参照）。長さ不整合の検出は proto/host の単体テストに委譲する。
///
/// `SendMessageW` は同期ゆえ、復帰時点で対象 WndProc の処理は完了している（観測が決定的）。
///
/// # Safety
/// `target_hwnd` は有効な HWND（本テストの親窓・生存中）であること。`data` は本呼び出し中
/// 生存する（`SendMessageW` は同期）。`cbData` は `data.len()` ちょうどゆえ受信側の
/// `from_raw_parts` は実バッファ内に収まる（境界外読み取りなし）。
unsafe fn send_raw_frame_with_tag(target_hwnd: u32, dw_data: usize, data: &[u8]) {
    let target = hwnd_from_u32(target_hwnd);
    let cds = COPYDATASTRUCT {
        dwData: dw_data,
        // UB 回避: cbData は実バッファ長ちょうど（受信側 from_raw_parts が境界内）。
        cbData: data.len() as u32,
        lpData: data.as_ptr() as *mut core::ffi::c_void,
    };
    // SAFETY: 呼び出し側前提（有効 target・data 生存）。cbData == data.len() ゆえ受信側の
    // from_raw_parts(lpData, cbData) は実バッファ内に収まる。SendMessageW は同期配送。
    unsafe {
        let _ = SendMessageW(
            target,
            WM_COPYDATA,
            Some(WPARAM(target.0 as usize)),
            Some(LPARAM(&cds as *const COPYDATASTRUCT as isize)),
        );
    }
}

// ---------------------------------------------------------------------------
// 経路 1・2・4: 窓を要する 3 経路を 1 つの親窓で順に確かめる
// ---------------------------------------------------------------------------

/// 窓を要する 3 経路（ハンドシェイク timeout / 不正フレーム隔離 / 応答 timeout・wedge）を
/// **単一の親窓**で順に検証する（design.md Integration Tests 2・3・5・要件 3.4 / 5.2 / 5.3 / 2.5）。
///
/// すべての pump / send_request は上限時間で bounded に復帰する（無限待機しない）。
///
/// **経路 4 をハンドシェイク成立前に置く理由**: 不正フレーム隔離を**非盲目**に観測するため。
/// ハンドシェイク前（helper_hwnd 未確定）に未知タグを注入し、直後の `pump_until_hello_or` が
/// 依然 `None` を返すことで「不正フレームが HELLO と誤認されず helper_hwnd を確定させない＝
/// 上位/ハンドシェイクへ漏れない」を公開 API のみで直接観測できる。この観測は `send_request` の
/// `slot.clear()` に盲目化されない（ハンドシェイク自体が成立しないため）。
#[test]
fn window_error_paths_handshake_timeout_wedge_and_corrupt_frame_isolation() {
    // 親 message-only 窓。helper は spawn しない（HELLO は誰も送らない）。
    let parent = ParentMessageWindow::create().expect("親 message-only 窓生成に失敗");
    let parent_hwnd = parent.hwnd_u32();

    // === 経路 1: ハンドシェイク timeout（要件 3.4） ===================================
    // helper を spawn せず HELLO も来ない状態で pump は上限時間で None を返す。
    let none = parent.pump_until_hello_or(Duration::from_millis(150));
    assert_eq!(
        none, None,
        "HELLO 未受領なら pump_until_hello_or は None を返す（ハンドシェイク timeout・要件 3.4）"
    );

    // === 経路 4: 不正フレーム隔離（要件 2.5・ハンドシェイク成立前に非盲目観測） =========
    // ハンドシェイク前（helper_hwnd 未確定）に、実 WM_COPYDATA で **未知タグ**（既知 MsgTag に無い
    // dwData = 0xDEAD）フレームを親窓へ送る。cbData は実バッファ長ちょうど（UB 回避）。
    //
    // 隔離の観測（公開 API・非盲目）: 不正フレーム送出**後**でも `pump_until_hello_or(短 timeout)` は
    // 依然 None を返す。＝不正フレームは WndProc で `IgnoreBad` に分類され、HELLO と誤認されて
    // helper_hwnd を確定させることが無い（上位/ハンドシェイクへ漏れない・要件 2.5）。この観測は
    // ハンドシェイク自体が成立しないゆえ `send_request` の `slot.clear()` に盲目化されない。
    //
    // RED 実証: この未知タグ注入が誤って RecordHello 扱いされる（隔離が壊れる）と、直後の pump は
    // Some(0xDEAD の下位解釈) を返し、この None アサートが落ちる。
    let unknown_payload: &[u8] = b"corrupt-unknown-tag-payload";
    // SAFETY: parent_hwnd は本テストの生存中の親窓。unknown_payload は本呼び出し中生存。
    // cbData = data.len() ちょうどゆえ受信側 from_raw_parts は境界内（UB なし）。
    unsafe {
        send_raw_frame_with_tag(
            parent_hwnd,
            0xDEAD_usize, // 既知 MsgTag(1..=5) 以外＝未知タグ
            unknown_payload,
        );
    }
    let still_none = parent.pump_until_hello_or(Duration::from_millis(150));
    assert_eq!(
        still_none, None,
        "不正フレーム（未知タグ）は HELLO と誤認されず helper_hwnd を確定させない\
         （隔離・上位へ渡らない・要件 2.5）"
    );

    // === ハンドシェイク成立（loopback HELLO・不正フレーム後でも窓は生きている） =========
    // 経路 2 は「ハンドシェイク成立後」を前提とする。上の不正フレームで親がクラッシュしていない
    // ことを、続けて正当な HELLO を送ってハンドシェイクが成立し得ること（生存）で示す。helper
    // 不在でも、親自身の HWND を helper HWND として loopback HELLO で登録すれば後続 send_request の
    // ゲートを通せる。親自身は REQUEST に RESPONSE を返さない（Request アームは記録のみ）ゆえ
    // 「応答しない相手」。
    let hello_payload = parent_hwnd.to_le_bytes(); // helper HWND = 親自身の HWND（u32 LE）
    send_copydata(
        hwnd_from_u32(parent_hwnd),
        hwnd_from_u32(parent_hwnd),
        MsgTag::Hello,
        &hello_payload,
        Duration::from_secs(5),
    )
    .expect("HELLO loopback 送出に失敗");
    // 送出は同期ゆえ、この時点でハンドシェイク成立（helper_hwnd = 親自身）を pump が確認できる。
    // ＝不正フレーム送出後でも窓は生きており正当 HELLO を受理できる（隔離＋生存の両面）。
    assert_eq!(
        parent.pump_until_hello_or(HANDSHAKE_TIMEOUT),
        Some(parent_hwnd),
        "不正フレーム後でも loopback HELLO でハンドシェイクが成立し helper HWND（= 親自身）が確定する\
         （窓が生存＝crash していない）"
    );

    // === 経路 2: 応答 timeout / wedge（要件 5.2 / 5.3） ===============================
    // 応答しない相手（親自身）へ REQUEST を送る。親 WndProc は REQUEST を記録のみで RESPONSE を
    // store しない → slot 空 → 上限時間で Timeout。SMTO_ABORTIFHUNG ＋短 timeout で有限復帰。
    let result = parent.send_request(MsgTag::Request, b"ping", Duration::from_millis(200));
    assert!(
        matches!(result, Err(SendError::Ipc(IpcError::Timeout))),
        "無応答相手への send_request は上限時間で Timeout を返す（要件 5.2）: got {result:?}"
    );

    drop(parent);
}
