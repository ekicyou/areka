//! host-32 先進坑: WM_COPYDATA トランスポート規約（親 x64 / helper i686 共有）。
//!
//! 本モジュールは **IpcChannel** コンポーネント（design.md §329–374）の単一ソースであり、
//! 親（`main.rs`）と helper（`helper.rs`）の双方から `#[path = "ipc.rs"] mod ipc;` で
//! 取り込まれる（design.md §150–153 / §168）。プロトコル定義を一箇所に集約することで、
//! x64⇄i686 の跨ビットネス規約のズレを構造的に防ぐ。
//!
//! 本タスク（1.2）が確立するのは **規約とビルディングブロック**のみ:
//!   - メッセージ種別タグ `MsgTag`（`dwData` の低 32bit に載る・跨ビットネス安全）
//!   - ペイロード規約（生バイト列・`cbData`=長さ・ポインタ/HANDLE/struct を載せない）
//!   - HWND の u32 LE ワイヤ表現（エンコード/デコード）
//!   - `SendMessageTimeout`（`SMTO_ABORTIFHUNG`）による送出規約（ハングしない）
//! 実走の 1 往復（応答待ち・再入受領）は後続タスク 4.1 の範囲であり本モジュールには含めない。

#![allow(dead_code)] // 本タスクは規約とビルディングブロックの確立のみ。実走呼び出しは後続タスク。

use core::cell::RefCell;
use core::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{
    SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_COPYDATA,
};

// ============================================================================
// 1. メッセージ種別タグ（MsgTag）
// ============================================================================

/// `WM_COPYDATA` の `COPYDATASTRUCT.dwData` に載せるメッセージ種別タグ。
///
/// 跨ビットネス規約（design.md §117 / §339）: `dwData` は `ULONG_PTR`（x64=64bit /
/// i686=32bit）だが、本タグは **低 32bit のみ**を使用する。`#[repr(u32)]` ゆえ判別子は
/// 必ず `u32` に収まり、`(tag as usize) >> 32 == 0` が常に成り立つ（跨ビットネス安全）。
///
/// ペイロード（生バイト列）はここには載らない — タグは「種別」のみを表し、本体は
/// `COPYDATASTRUCT.lpData`（長さ `cbData`）で渡す。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgTag {
    /// helper → 親: メッセージ窓 HWND ハンドシェイク（ペイロード = HWND の u32 LE）。
    Hello = 1,
    /// 親 → helper: `pasta.dll` ロード指示（ペイロード = ghostdir バイト列）。
    Load = 2,
    /// 親 → helper: SHIORI/3.0 リクエスト（ペイロード = 生 SHIORI3 バイト列）。
    Request = 3,
    /// helper → 親: SHIORI/3.0 応答（ペイロード = 生 SHIORI3 バイト列）。
    Response = 4,
    /// 親 → helper: `pasta.dll` アンロード指示。
    Unload = 5,
}

impl MsgTag {
    /// タグを `u32` 判別子へ。`dwData` へ載せる際の低 32bit 表現。
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    /// 受信側（inbound）: `dwData` の低 32bit から `MsgTag` を復元する。
    /// 未知の値は `Err(raw)` を返す（不正フレームの観測点）。
    pub fn try_from_u32(raw: u32) -> Result<Self, u32> {
        match raw {
            x if x == MsgTag::Hello.as_u32() => Ok(MsgTag::Hello),
            x if x == MsgTag::Load.as_u32() => Ok(MsgTag::Load),
            x if x == MsgTag::Request.as_u32() => Ok(MsgTag::Request),
            x if x == MsgTag::Response.as_u32() => Ok(MsgTag::Response),
            x if x == MsgTag::Unload.as_u32() => Ok(MsgTag::Unload),
            other => Err(other),
        }
    }
}

impl From<MsgTag> for u32 {
    #[inline]
    fn from(tag: MsgTag) -> u32 {
        tag.as_u32()
    }
}

impl TryFrom<u32> for MsgTag {
    type Error = u32;
    #[inline]
    fn try_from(raw: u32) -> Result<Self, u32> {
        MsgTag::try_from_u32(raw)
    }
}

// ============================================================================
// 2. ペイロード規約（生バイト列・cbData=長さ・ポインタ/HANDLE/struct 禁止）
// ============================================================================

/// **ペイロード規約**（design.md §338–339 / requirements 2.1）。
///
/// - WM_COPYDATA のペイロードは **生バイト列**である。固定ヘッダ・手動フレーミングは持たず、
///   メッセージ境界は OS が `COPYDATASTRUCT.cbData`（バイト長）で画定する。
/// - ペイロードには **ポインタ・HANDLE・Rust/C の struct を一切載せない**。跨ビットネスで
///   有効なのは生バイト列のみ（HWND は §3 の u32 LE 表現で別途受け渡す）。
/// - `dwData` は種別タグ（§1）専用であり、本体データは載せない。
///
/// 本定数は「ペイロードに構造を持たせない」契約の宣言的アンカーである（コードでヘッダ長を
/// 0 と固定することで、フレーミングが `cbData` のみに依存することを表現する）。
pub const PAYLOAD_HEADER_LEN: usize = 0;

// ============================================================================
// 3. HWND ワイヤ表現（u32 LE）
// ============================================================================

/// HWND を **u32 リトルエンディアン** 4 バイトへエンコードする（design.md §117 / §339）。
///
/// HWND は USER ハンドルゆえ **32bit 有意**であり、x64 側は上位 32bit を zero/sign-extend
/// しているにすぎない。跨ビットネスで安全に運べるのは下位 32bit のみなので、ワイヤ表現は
/// 常に u32 LE（4 バイト固定）とする。ポインタ幅（x64=8 / i686=4）をそのまま載せてはならない。
#[inline]
pub fn encode_hwnd_le(hwnd: HWND) -> [u8; 4] {
    // HWND(*mut c_void) の数値をいったん usize で取り出し、下位 32bit のみワイヤへ。
    let raw = hwnd.0 as usize as u32;
    raw.to_le_bytes()
}

/// u32 LE 4 バイトから HWND の 32bit 有意値（USER ハンドル）を復元する。
///
/// 復元側がポインタ幅 64bit の x64 親であっても、まず 32bit 値として読み、その後
/// `HWND(value as usize as *mut _)` で zero-extend する（design.md §339）。
#[inline]
pub fn decode_hwnd_le(bytes: [u8; 4]) -> u32 {
    u32::from_le_bytes(bytes)
}

/// `decode_hwnd_le` で得た 32bit 値を、当該プロセスのポインタ幅の `HWND` へ復元する。
/// x64 では上位 32bit を 0 で埋める（zero-extend）。
#[inline]
pub fn hwnd_from_u32(value: u32) -> HWND {
    HWND(value as usize as *mut core::ffi::c_void)
}

// ============================================================================
// 4. SendMessageTimeout 送出規約（ハングしない）
// ============================================================================

/// IPC 送出の失敗種別（design.md §365）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// `timeout` 内に `SendMessageTimeout` が返らなかった（要件 2.3: ハングしない）。
    Timeout,
    /// 送出自体が失敗した（`SendMessageTimeout` が 0 を返す等）。
    SendFailed,
    /// 送出先ウィンドウが消滅していた（peer プロセス終了）。
    PeerGone,
}

/// 送出のデフォルトタイムアウト規約（要件 2.3）。後続タスクが必要に応じて上書きする。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// **送出規約**: `SendMessageTimeout`（`SMTO_ABORTIFHUNG` ＋タイムアウト）で WM_COPYDATA を
/// 1 通送る薄いラッパ（design.md §340 / requirements 2.3）。
///
/// - `SMTO_ABORTIFHUNG`: 送出先がハング中なら即座に打ち切る（ハングしない契約）。
/// - `timeout`: ミリ秒粒度で `SendMessageTimeoutW` に渡す。期限超過は `Err(Timeout)`。
/// - ペイロードは生バイト列（§2）。`cbData` に長さ、`dwData` に種別タグ（低 32bit）を載せる。
///
/// 本ラッパは「片道送出」の規約確立まで。応答（helper→親の 2nd WM_COPYDATA）待ちは
/// 後続タスク 4.1 の範囲であり、本関数は戻り値を待たない（`Ok(())` で送出成功のみ表す）。
///
/// # Safety
/// `SendMessageTimeoutW` は FFI。`target` は有効な HWND（ハンドシェイク確定済）であること、
/// `payload` は呼び出し中生存していること（`COPYDATASTRUCT.lpData` がこれを指す）が前提。
pub fn send_copydata(
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), IpcError> {
    // 生バイト列を指す COPYDATASTRUCT を組み立てる。lpData は payload を指すだけで、
    // ポインタ自体はワイヤに載らない（OS が cbData バイトを marshal する）。
    let cds = COPYDATASTRUCT {
        dwData: tag.as_u32() as usize, // ★ 低 32bit のみ使用（跨ビットネス安全）
        cbData: payload.len() as u32,  // ★ メッセージ境界 = cbData（手動フレーミング不要）
        lpData: payload.as_ptr() as *mut core::ffi::c_void,
    };

    let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;

    let mut result: usize = 0;
    // SAFETY: target は有効 HWND 前提。&cds は本呼び出し中生存。SMTO_ABORTIFHUNG ＋
    // timeout により無期限ブロックしない（要件 2.3）。
    let ret: LRESULT = unsafe {
        SendMessageTimeoutW(
            target,
            WM_COPYDATA,
            WPARAM(self_hwnd.0 as usize),
            LPARAM(&cds as *const COPYDATASTRUCT as isize),
            SMTO_ABORTIFHUNG,
            timeout_ms,
            Some(&mut result as *mut usize),
        )
    };

    if ret.0 == 0 {
        // 0 = タイムアウト or 失敗。GetLastError で区別可能だが先進坑では Timeout 既定。
        Err(IpcError::Timeout)
    } else {
        Ok(())
    }
}

// ============================================================================
// 5. 受け皿セル（ResponseSlot）＋ 1 往復 send_request（再入受領・task 4.1）
// ============================================================================

/// **受け皿セル**（design.md §209・議題 2 確定）。親の応答 WndProc と `send_request` が
/// 共有する single-in-flight の応答格納先。
///
/// 往復は同時に 1 つ（`Invariants: single-in-flight`・design.md §369）ゆえ内部可変は
/// `RefCell` で足りる（UI スレッド固定・reader スレッド不要）。`send_request` が送出前に
/// `clear()` し、応答 WndProc が `store()` し、`SendMessageTimeout` 復帰後に `take()` する。
///
/// **デッドロック回避の核**（design.md §210）: 応答を受ける WndProc は本セルへ payload を
/// 格納して**即 return**するのみ。それ以上の跨プロセス `SendMessage` を発行しない
/// （REQUEST 待ち ⊃ RESPONSE 受領 WndProc の厳密な入れ子 ＝ 循環待ちなし）。
#[derive(Default)]
pub struct ResponseSlot {
    inner: RefCell<Option<Vec<u8>>>,
}

impl ResponseSlot {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(None),
        }
    }

    /// 送出直前に空にする（前往復の残骸を排除・single-in-flight の前提）。
    pub fn clear(&self) {
        *self.inner.borrow_mut() = None;
    }

    /// 応答 WndProc が RESPONSE payload を格納する（再入受領・即 return）。
    pub fn store(&self, bytes: Vec<u8>) {
        *self.inner.borrow_mut() = Some(bytes);
    }

    /// `SendMessageTimeout` 復帰後に取り出す（`None` なら未受領＝Timeout）。
    pub fn take(&self) -> Option<Vec<u8>> {
        self.inner.borrow_mut().take()
    }
}

/// **1 往復送信**（design.md §353–365・IpcChannel Service Interface）。
///
/// `send_copydata`（片道）を土台に、応答（helper→親の 2nd WM_COPYDATA）を**受け皿セル方式**で
/// 再入受領する（design.md §209）。手順:
///   1. `slot.clear()` — 受け皿を空に（single-in-flight）。
///   2. `SendMessageTimeout(target, WM_COPYDATA, REQUEST)` — 親はここでブロック。
///      待機中、Windows が helper の RESPONSE(2nd WM_COPYDATA) を親 WndProc へ**再入配送**し、
///      WndProc が `slot.store()` して即 return する（跨プロセス SendMessage を発行しない・§210）。
///   3. `SendMessageTimeout` 復帰後 `slot.take()` — `Some(bytes)=>Ok`、空=>`Err(Timeout)`。
///
/// `SMTO_ABORTIFHUNG` ＋ `timeout` で wedged peer をハングさせない（要件 2.3）。往復は
/// single-in-flight（同時に 1 つ・design.md §369）。
///
/// # Safety
/// `target` は有効な HWND（ハンドシェイクで確定）。`payload` は本呼び出し中生存。
/// `slot` は親の応答 WndProc が参照する受け皿と**同一**でなければならない（再入受領の要）。
pub fn send_request(
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
    slot: &ResponseSlot,
) -> Result<Vec<u8>, IpcError> {
    // ① 受け皿を空に（前往復の残骸排除・single-in-flight）。
    slot.clear();

    // ② REQUEST を同期送出（親はここでブロック）。待機中に RESPONSE が再入配送され、
    //    親 WndProc が slot.store() する（design.md §209）。
    let cds = COPYDATASTRUCT {
        dwData: tag.as_u32() as usize, // 低 32bit のみ（跨ビットネス安全）
        cbData: payload.len() as u32,  // メッセージ境界 = cbData（手動フレーミング不要）
        lpData: payload.as_ptr() as *mut core::ffi::c_void,
    };
    let timeout_ms = timeout.as_millis().min(u32::MAX as u128) as u32;
    let mut result: usize = 0;
    // SAFETY: target 有効前提。&cds は本呼び出し中生存。SMTO_ABORTIFHUNG＋timeout でハングしない。
    let ret: LRESULT = unsafe {
        SendMessageTimeoutW(
            target,
            WM_COPYDATA,
            WPARAM(self_hwnd.0 as usize),
            LPARAM(&cds as *const COPYDATASTRUCT as isize),
            SMTO_ABORTIFHUNG,
            timeout_ms,
            Some(&mut result as *mut usize),
        )
    };

    if ret.0 == 0 {
        // 0 = REQUEST 送出自体がタイムアウト/失敗（helper が REQUEST を捌けなかった）。
        return Err(IpcError::Timeout);
    }

    // ③ 受け皿から応答を取り出す。再入受領が成立していれば Some、未受領なら Timeout。
    match slot.take() {
        Some(bytes) => Ok(bytes),
        None => Err(IpcError::Timeout),
    }
}

// ============================================================================
// 単体テスト（観測可能な受け入れ基準・task 1.2）
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 全 `MsgTag` が低 32bit に収まる（`(tag as usize) >> 32 == 0`）こと、
    /// および u32 へ往復ロスレスであること（design.md §339 跨ビットネス規約）。
    #[test]
    fn msgtag_fits_in_low_32bits_and_roundtrips() {
        for tag in [
            MsgTag::Hello,
            MsgTag::Load,
            MsgTag::Request,
            MsgTag::Response,
            MsgTag::Unload,
        ] {
            // 低 32bit のみ占有（ULONG_PTR の上位 32bit は常に 0）。
            // 評価は u64 で行う（i686 では usize=32bit ゆえ `usize >> 32` が
            // overflow lint でコンパイルエラーになる・跨ビットネス可搬性）。
            let as_dwdata = u64::from(tag.as_u32());
            assert_eq!(
                as_dwdata >> 32,
                0,
                "{tag:?} must occupy only the low 32 bits of dwData"
            );

            // u32 値自体が u32::MAX 以下（自明だが規約の明示）。
            assert!(u64::from(tag.as_u32()) <= u64::from(u32::MAX));

            // u32 へ往復ロスレス。
            let raw = tag.as_u32();
            let back = MsgTag::try_from_u32(raw).expect("known tag must round-trip");
            assert_eq!(back, tag, "{tag:?} must round-trip through u32 losslessly");

            // From/TryFrom 経路も一致。
            let via_into: u32 = tag.into();
            assert_eq!(via_into, raw);
            assert_eq!(MsgTag::try_from(raw).unwrap(), tag);
        }
    }

    /// 未知の判別子は `Err(raw)` を返す（不正フレームの観測点）。
    #[test]
    fn msgtag_unknown_discriminant_is_err() {
        assert_eq!(MsgTag::try_from_u32(0), Err(0));
        assert_eq!(MsgTag::try_from_u32(6), Err(6));
        assert_eq!(MsgTag::try_from_u32(u32::MAX), Err(u32::MAX));
    }

    /// HWND の u32 LE エンコード/デコードがサンプル値を往復すること（design.md §339）。
    #[test]
    fn hwnd_u32_le_roundtrips() {
        // 代表的な 32bit 有意ハンドル値（USER ハンドルは 32bit 有意）。
        for sample in [0x0000_0001u32, 0x1234_5678u32, 0xDEAD_BEEFu32, u32::MAX] {
            let hwnd = hwnd_from_u32(sample);
            let bytes = encode_hwnd_le(hwnd);
            // リトルエンディアン並びの明示確認。
            assert_eq!(bytes, sample.to_le_bytes(), "LE byte order for {sample:#010x}");
            let decoded = decode_hwnd_le(bytes);
            assert_eq!(decoded, sample, "u32 LE round-trip for {sample:#010x}");
            // HWND まで戻して同値（当該プロセス幅へ zero-extend）。
            assert_eq!(hwnd_from_u32(decoded).0 as usize as u32, sample);
        }
    }

    /// ペイロード規約: 固定ヘッダ長は 0（フレーミングは cbData のみに依存）。
    #[test]
    fn payload_has_no_fixed_header() {
        assert_eq!(PAYLOAD_HEADER_LEN, 0);
    }

    /// 受け皿セル（ResponseSlot）の semantics（task 4.1・design.md §209）:
    /// clear→take=None、store→take=Some（1 回で消費）、再 take=None（single-in-flight）。
    #[test]
    fn response_slot_clear_store_take_semantics() {
        let slot = ResponseSlot::new();
        // 初期/clear 後は空（未受領 = Timeout 相当）。
        assert_eq!(slot.take(), None);
        slot.clear();
        assert_eq!(slot.take(), None);

        // WndProc が store → send_request が take で 1 回だけ取り出す。
        slot.store(b"Value: hello".to_vec());
        assert_eq!(slot.take().as_deref(), Some(&b"Value: hello"[..]));
        // 取り出し済みは空（次往復のため）。
        assert_eq!(slot.take(), None);

        // clear は残骸を排除する（前往復の応答を次往復に持ち越さない）。
        slot.store(b"stale".to_vec());
        slot.clear();
        assert_eq!(slot.take(), None);
    }
}
