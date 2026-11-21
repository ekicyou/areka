//! host-32 IPC WM_COPYDATA ワイヤ規約（framing 部分）。
//!
//! x64/arm64 親プロセスと i686 helper プロセスの間で生バイト列を往復させる
//! transport 層のうち、**メッセージ framing・HWND 符号化・不正フレーム検出**と送受信を担う。
//! 下流と凍結共有する単一ソース。
//!
//! - `MsgTag` は `dwData` の**低 32bit のみ**に載る（跨ビットネス安全）。
//! - payload は**生バイト列**（`cbData` = 長さ・in-band のヘッダを持たない）。
//!   ポインタ・HANDLE・struct を境界を越えて共有しない。
//! - HWND は**常に u32 LE 4 バイト**でワイヤ表現する。
//! - shift/mask 評価は必ず `u64` cast で行い、i686 の `usize` = 32bit での
//!   overflow を回避する。
//!
//! 送信は `SendMessageTimeoutW` の上限時間で無限待機を構造的に排除する。
//! 旗は**送出の向き**で分ける（[`SendFlavor`]・[`send_flags`]）。

use core::cell::RefCell;
use core::time::Duration;

use windows::Win32::Foundation::{
    ERROR_SUCCESS, ERROR_TIMEOUT, GetLastError, HWND, LPARAM, LRESULT, SetLastError, WPARAM,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{
    SEND_MESSAGE_TIMEOUT_FLAGS, SMTO_ABORTIFHUNG, SMTO_NORMAL, SendMessageTimeoutW, WM_COPYDATA,
};

/// WM_COPYDATA メッセージ種別タグ（`dwData` の低 32bit に載る）。
///
/// 判別子は全て u32 の低 32bit に収まり、跨ビットネス安全である
/// （`(tag.as_u32() as u64) >> 32 == 0`）。
///
/// 本クレートはタグの規約だけを持ち、意味づけは使い手（host と helper）が行う。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgTag {
    /// helper → 親: 自身のメッセージ窓 HWND を u32 LE で通知するハンドシェイク。
    Hello = 1,
    /// 親 → helper: SHIORI DLL のロード要求（ペイロードなし・helper が ack 1 バイトを返す）。
    Load = 2,
    /// 親 → helper: 生バイト request。
    Request = 3,
    /// helper → 親: 生バイト response。
    Response = 4,
    /// 親 → helper: SHIORI DLL のアンロードと正常終了の要求（ペイロードなし・helper が ack を返す）。
    Unload = 5,
}

impl MsgTag {
    /// タグを `dwData` へ載せる u32 生値へ変換する。
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }

    /// `dwData` から取り出した u32 生値をタグへ復元する。
    ///
    /// 未知の生値は不正フレームの観測点として `Err(raw)` を返す。
    #[inline]
    pub fn try_from_u32(raw: u32) -> Result<Self, u32> {
        match raw {
            1 => Ok(MsgTag::Hello),
            2 => Ok(MsgTag::Load),
            3 => Ok(MsgTag::Request),
            4 => Ok(MsgTag::Response),
            5 => Ok(MsgTag::Unload),
            other => Err(other),
        }
    }
}

/// HWND を **u32 リトルエンディアン 4 バイト**へ符号化する。
///
/// HWND は USER ハンドルゆえ 32bit 有意であり、x64/arm64 でも下位 32bit のみをワイヤへ載せる。
/// 抽出は `u64` cast 経由で mask し、i686 の `usize` = 32bit での shift overflow を回避する。
#[inline]
pub fn encode_hwnd_le(hwnd: HWND) -> [u8; 4] {
    let raw = hwnd.0 as usize as u64;
    let low = (raw & 0xFFFF_FFFF) as u32;
    low.to_le_bytes()
}

/// u32 LE 4 バイトから HWND の 32bit 有意値を復元する。
#[inline]
pub fn decode_hwnd_le(bytes: [u8; 4]) -> u32 {
    u32::from_le_bytes(bytes)
}

/// u32 の 32bit 有意値を、当該プロセスのポインタ幅の `HWND` へ復元する。
///
/// x64/arm64 では上位 32bit を zero-extend する（`value as usize` 経由）。
#[inline]
pub fn hwnd_from_u32(value: u32) -> HWND {
    HWND(value as usize as *mut core::ffi::c_void)
}

/// 受信 WM_COPYDATA フレームを検証し payload を取り出す純粋 framing 関数。
///
/// COPYDATASTRUCT そのものではなく、実窓なしで単体検証できるように
/// `(dwData 生値, cbData 宣言長, 実データ)` から検証済み payload を返す。
///
/// - `dw_data`: `COPYDATASTRUCT.dwData`（種別タグの生値）。低 32bit のみ有意。
/// - `declared_len`: `COPYDATASTRUCT.cbData`（宣言された payload 長）。
/// - `data`: `COPYDATASTRUCT.lpData` が指す実バイト列。
///
/// 検証:
/// - 未知タグ（`try_from_u32` が `Err`）→ `Err`。
/// - 宣言長 `cbData` と実 payload 長の不整合 → `Err`。
///
/// `dw_data` の型は `usize`（`COPYDATASTRUCT.dwData` は跨ビットネスで幅が異なる）。
/// 低 32bit のみを `u64` cast 経由で取り出すため、i686 の `usize` = 32bit でも overflow しない。
pub fn copydata_payload(
    dw_data: usize,
    declared_len: usize,
    data: &[u8],
) -> Result<(MsgTag, &[u8]), FramingError> {
    let raw_tag = ((dw_data as u64) & 0xFFFF_FFFF) as u32;
    let tag = MsgTag::try_from_u32(raw_tag).map_err(FramingError::UnknownTag)?;

    if declared_len != data.len() {
        return Err(FramingError::LengthMismatch {
            declared: declared_len,
            actual: data.len(),
        });
    }

    Ok((tag, data))
}

/// 受領した WM_COPYDATA の LPARAM から `(dwData 生値, cbData 宣言長, payload コピー)` を取り出す
/// （host の親窓と helper の窓が共に使う）。`lparam` が空なら `None`（呼び手が記録して捨てる）。
///
/// # Safety
/// `lparam` は 0 か、WM_COPYDATA の配送中に OS が渡した `*const COPYDATASTRUCT` であること。
/// その `lpData` が空でなければ `cbData` バイト分読めること（OS が受け手の側へ写した領域）。
/// どちらも WndProc が WM_COPYDATA を処理している間だけ有効なので、その中で呼ぶこと。
pub unsafe fn read_copydata(lparam: LPARAM) -> Option<(usize, usize, Vec<u8>)> {
    let cds = lparam.0 as *const COPYDATASTRUCT;
    if cds.is_null() {
        return None;
    }
    // SAFETY: 呼び手の約束により lparam は有効な COPYDATASTRUCT を指す。
    let cds = unsafe { &*cds };
    let dw_data = cds.dwData;
    let len = cds.cbData as usize;
    let payload = if len == 0 || cds.lpData.is_null() {
        Vec::new()
    } else {
        // SAFETY: 呼び手の約束により lpData は cbData バイト読める。
        unsafe { std::slice::from_raw_parts(cds.lpData as *const u8, len).to_vec() }
    };
    Some((dw_data, len, payload))
}

/// framing レベルの不正フレーム表現。
///
/// 送信・往復層の [`IpcError`] へは `IpcError::CorruptFrame` として `From` で持ち上げる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramingError {
    /// `dwData` の低 32bit が既知の [`MsgTag`] でない（未知タグ）。
    UnknownTag(u32),
    /// 宣言長 `cbData` と実 payload 長が不整合。
    LengthMismatch { declared: usize, actual: usize },
}

/// 送信・往復トランスポート層の構造化エラー。
///
/// **一様な失敗報告**: peer の生死やハングを distinct なバリアントで区別せず、
/// 期限切れは `Timeout`、到達不能・ハング peer の中断（`SMTO_ABORTIFHUNG`）は
/// `SendFailed` に集約する（distinct な PeerGone は設けない）。
/// ハング打ち切りが起きうるのは**要求方向だけ**である（[`send_flags`]）。
/// peer の生死は送信結果に混ぜず、上位の `ExitKind`（ProcessHost）で観測する。
///
/// [`FramingError`] は `CorruptFrame` へ `From` で持ち上げ、呼び出し側が `?` で一様に扱う。
#[derive(thiserror::Error, Debug)]
pub enum IpcError {
    /// 応答が上限時間内に返らなかった: `SendMessageTimeoutW` が上限時間で
    /// 打ち切られた（戻り 0 かつ last error が `ERROR_TIMEOUT`）か、往復の応答が受け皿に
    /// 届かなかった（[`send_request`]）。
    #[error("ipc request timed out")]
    Timeout,
    /// `SendMessageTimeoutW` が期限切れ以外の理由で 0 を返した（宛先の窓が無い・
    /// 要求方向の送出でハング peer が `SMTO_ABORTIFHUNG` で中断された、など）。
    #[error("ipc send failed")]
    SendFailed,
    /// 受信フレームが framing 規約に整合しない（未知タグ・長さ不整合など）。
    #[error("corrupt ipc frame: {0}")]
    CorruptFrame(FramingError),
}

impl From<FramingError> for IpcError {
    #[inline]
    fn from(err: FramingError) -> Self {
        IpcError::CorruptFrame(err)
    }
}

impl core::fmt::Display for FramingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FramingError::UnknownTag(raw) => write!(f, "unknown message tag {raw:#x}"),
            FramingError::LengthMismatch { declared, actual } => {
                write!(f, "length mismatch: declared {declared}, actual {actual}")
            }
        }
    }
}

impl core::error::Error for FramingError {}

/// single-in-flight の応答受け皿。
///
/// UI スレッド固定かつ往復は同時に 1 つゆえ、内部同期は [`RefCell`] で足りる（`Mutex` 不要）。
/// 応答 WndProc が再入で [`store`](Self::store) し、[`send_request`] が [`take`](Self::take)
/// する。両者は同一の `&ResponseSlot` を参照する。
///
/// 状態機械: `clear` → 送信ブロック中に WndProc が `store` → 復帰後 `take` で 1 回消費
/// （再 `take` は `None`）。
#[derive(Debug, Default)]
pub struct ResponseSlot {
    inner: RefCell<Option<Vec<u8>>>,
}

impl ResponseSlot {
    /// 空の受け皿を生成する。
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(None),
        }
    }

    /// 受け皿を空にする（前回往復の残骸を排除・single-in-flight の前提確立）。
    #[inline]
    pub fn clear(&self) {
        *self.inner.borrow_mut() = None;
    }

    /// 応答バイト列を格納する（再入 WndProc から呼ばれる）。
    ///
    /// single-in-flight 前提ゆえ、既存値があれば最新格納で上書きする。
    #[inline]
    pub fn store(&self, bytes: Vec<u8>) {
        *self.inner.borrow_mut() = Some(bytes);
    }

    /// 格納済み応答を 1 回だけ取り出す（消費後は空・再 `take` は `None`）。
    #[inline]
    pub fn take(&self) -> Option<Vec<u8>> {
        self.inner.borrow_mut().take()
    }
}

/// `SendMessageTimeoutW` へ渡す `Duration` をミリ秒 `u32` に飽和変換する。
///
/// `u32::MAX` ミリ秒（約 49.7 日）で上限をクランプする。
#[inline]
fn timeout_millis(timeout: Duration) -> u32 {
    timeout.as_millis().min(u32::MAX as u128) as u32
}

/// 送出の**向き**。
///
/// 向きによって `SendMessageTimeoutW` へ渡す旗が変わる。どちらの向きにどの旗が付くかは
/// [`send_flags`] が単独で決める（呼び出し側に旗を書かせない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendFlavor {
    /// 要求方向（ホスト → helper）。相手はメッセージループを常時回している。
    Request,
    /// 応答方向（helper → ホスト）。相手は自分の `SendMessageTimeoutW` の中でこの応答を
    /// 待っており、待っている間はメッセージを取り出さない。
    Response,
}

/// 向きに応じた `SendMessageTimeoutW` の旗を返す純関数。
///
/// - [`SendFlavor::Request`] → `SMTO_ABORTIFHUNG`（相手が応答なしなら待たずに打ち切る）。
/// - [`SendFlavor::Response`] → `SMTO_NORMAL`（旗なし＝上限時間だけで送る）。応答の宛先は
///   自分の要求の中でこの応答を待っている相手であり、「応答なし」判定はこの向きでは
///   誤検知にしかならない。
#[inline]
#[must_use]
pub const fn send_flags(flavor: SendFlavor) -> SEND_MESSAGE_TIMEOUT_FLAGS {
    match flavor {
        SendFlavor::Request => SMTO_ABORTIFHUNG,
        SendFlavor::Response => SMTO_NORMAL,
    }
}

/// 生バイト payload を WM_COPYDATA として**要求方向**へ片道送出する。
///
/// 旗は [`send_flags`]`(`[`SendFlavor::Request`]`)`＝`SMTO_ABORTIFHUNG` ＋上限時間。送出先が
/// ハング中でも上限時間で復帰する（無限待機の構造的排除）。戻り 0 のうち期限切れは
/// [`IpcError::Timeout`]、それ以外（失敗 / hung peer）は [`IpcError::SendFailed`] とする。
///
/// 応答方向は [`send_copydata_response`] を使う（旗が異なる）。
///
/// - `dwData`: `tag.as_u32()`（低 32bit のみ有意・跨ビットネス安全）。
/// - `cbData`: `payload.len()`（メッセージ境界＝固定ヘッダ長 0）。
/// - `lpData`: `payload` を指す。ポインタ自体はワイヤに載らず OS が `cbData` バイトを marshal する。
/// - `wParam`: 送信側自身の HWND（`self_hwnd`）。
///
/// # 前提
/// `SendMessageTimeoutW` は FFI。`target` は有効な HWND（ハンドシェイク確定済）であること、
/// `payload` は本呼び出し中生存すること（`SendMessage` は同期ゆえ復帰まで生存が保証される）。
pub fn send_copydata(
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), IpcError> {
    send_copydata_with(
        SendFlavor::Request,
        target,
        self_hwnd,
        tag,
        payload,
        timeout,
    )
}

/// 生バイト payload を WM_COPYDATA として**応答方向**へ片道送出する。
///
/// [`send_copydata`] との違いは旗だけ——`SMTO_ABORTIFHUNG` を付けず、上限時間のみで送る。
/// 応答の宛先は自分の `SendMessageTimeoutW` の中でこの応答を待っている相手であり、往復の間は
/// メッセージを取り出さない（手空き時の周期的な保守は往復の外でしか走らない）ため、往復が
/// 長引けば OS からは「応答なし」に見えうるが、この向きではその判定を送出の打ち切り理由にしない。
/// 時間上限は呼び出し側が渡す（helper は `REPLY_TIMEOUT`＝5 秒）。
///
/// # 前提
/// [`send_copydata`] の前提（有効 `target`・`payload` 生存）を引き継ぐ。
pub fn send_copydata_response(
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), IpcError> {
    send_copydata_with(
        SendFlavor::Response,
        target,
        self_hwnd,
        tag,
        payload,
        timeout,
    )
}

/// [`send_copydata`]／[`send_copydata_response`] の共通本体（違いは旗だけ）。
///
/// # 前提
/// [`send_copydata`] の前提（有効 `target`・`payload` 生存）を引き継ぐ。
fn send_copydata_with(
    flavor: SendFlavor,
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), IpcError> {
    let cds = COPYDATASTRUCT {
        dwData: tag.as_u32() as usize,
        cbData: payload.len() as u32,
        lpData: payload.as_ptr() as *mut core::ffi::c_void,
    };
    let timeout_ms = timeout_millis(timeout);

    let mut result: usize = 0;
    // 戻り 0 のとき SendMessageTimeoutW は last error を必ずしも設定しないので、前の値を消しておく。
    // SAFETY: スレッド局所のエラー値を書くだけ。
    unsafe { SetLastError(ERROR_SUCCESS) };
    // SAFETY: target は有効 HWND 前提。&cds は本呼び出し中生存し、その lpData が指す
    // payload も同期送信中生存する。timeout_ms により無期限ブロックしない。
    let ret: LRESULT = unsafe {
        SendMessageTimeoutW(
            target,
            WM_COPYDATA,
            WPARAM(self_hwnd.0 as usize),
            LPARAM(&cds as *const COPYDATASTRUCT as isize),
            send_flags(flavor),
            timeout_ms,
            Some(&mut result as *mut usize),
        )
    };

    // hung peer の中断は last error 0 のまま返るので SendFailed に落ちる。
    if ret.0 == 0 {
        // SAFETY: 直前の SendMessageTimeoutW が残したスレッド局所のエラー値を読むだけ。
        if unsafe { GetLastError() } == ERROR_TIMEOUT {
            return Err(IpcError::Timeout);
        }
        return Err(IpcError::SendFailed);
    }
    Ok(())
}

/// 1 往復送信（再入受領・single-in-flight）。
///
/// [`send_copydata`] で REQUEST を同期送出してブロックする間に、Windows が helper の
/// RESPONSE（2nd WM_COPYDATA）を親 WndProc へ再入配送する。WndProc は `slot.store()` して
/// 即 return する（跨プロセス SendMessage を発行しない）。復帰後に受け皿が空（上限内未受領）なら
/// [`IpcError::Timeout`]。
///
/// `slot` は親の応答 WndProc が参照するものと同一 `&ResponseSlot`。往復は同時に 1 つ。
///
/// # 前提
/// [`send_copydata`] の前提（有効 `target`・`payload` 生存）を引き継ぐ。
pub fn send_request(
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
    slot: &ResponseSlot,
) -> Result<Vec<u8>, IpcError> {
    slot.clear();

    send_copydata(target, self_hwnd, tag, payload, timeout)?;

    slot.take().ok_or(IpcError::Timeout)
}

#[cfg(test)]
mod send_timeout_tests;

#[cfg(test)]
mod slot_error_tests {
    use super::*;

    #[test]
    fn slot_clear_then_take_is_none() {
        let slot = ResponseSlot::new();
        slot.clear();
        assert_eq!(slot.take(), None);
    }

    #[test]
    fn slot_store_then_take_is_some_once() {
        let slot = ResponseSlot::new();
        slot.store(b"resp".to_vec());
        assert_eq!(slot.take(), Some(b"resp".to_vec()));
    }

    #[test]
    fn slot_take_after_consume_is_none() {
        let slot = ResponseSlot::new();
        slot.store(b"resp".to_vec());
        let _ = slot.take();
        assert_eq!(slot.take(), None, "consumed slot must be empty");
    }

    #[test]
    fn slot_clear_evicts_stale() {
        let slot = ResponseSlot::new();
        slot.store(b"stale".to_vec());
        slot.clear();
        assert_eq!(slot.take(), None, "clear must evict stale residue");
    }

    #[test]
    fn slot_store_overwrites_latest() {
        let slot = ResponseSlot::new();
        slot.store(b"first".to_vec());
        slot.store(b"second".to_vec());
        assert_eq!(slot.take(), Some(b"second".to_vec()));
    }

    #[test]
    fn ipc_error_variants_display() {
        let timeout = IpcError::Timeout;
        let send_failed = IpcError::SendFailed;
        assert!(!format!("{timeout}").is_empty());
        assert!(!format!("{send_failed}").is_empty());
        assert!(!format!("{timeout:?}").is_empty());
    }

    #[test]
    fn framing_error_lifts_into_ipc_error() {
        let fe = FramingError::UnknownTag(9);
        let ie: IpcError = fe.into();
        assert!(matches!(ie, IpcError::CorruptFrame(_)));
    }
}

#[cfg(test)]
mod framing_tests {
    use super::*;

    const ALL_TAGS: [MsgTag; 5] = [
        MsgTag::Hello,
        MsgTag::Load,
        MsgTag::Request,
        MsgTag::Response,
        MsgTag::Unload,
    ];

    #[test]
    fn msgtag_u32_roundtrip_lossless() {
        for tag in ALL_TAGS {
            let raw = tag.as_u32();
            assert_eq!(MsgTag::try_from_u32(raw), Ok(tag));
        }
    }

    #[test]
    fn msgtag_occupies_low_32_bits_only() {
        for tag in ALL_TAGS {
            let widened = tag.as_u32() as u64;
            assert_eq!(widened >> 32, 0, "tag {tag:?} must fit low 32 bits");
        }
    }

    #[test]
    fn msgtag_unknown_is_err() {
        for raw in [0u32, 6u32, 0xFFFF_FFFF] {
            assert_eq!(MsgTag::try_from_u32(raw), Err(raw));
        }
    }

    #[test]
    fn hwnd_u32_le_roundtrip_and_byte_order() {
        for value in [0x1u32, 0x1234_5678, 0xDEAD_BEEF, u32::MAX] {
            let hwnd = hwnd_from_u32(value);
            let bytes = encode_hwnd_le(hwnd);
            assert_eq!(bytes, value.to_le_bytes(), "LE byte order for {value:#x}");
            assert_eq!(decode_hwnd_le(bytes), value, "roundtrip for {value:#x}");
        }
    }

    #[test]
    fn framing_accepts_valid_frame() {
        let payload: &[u8] = b"hello-echo";
        let dw_data = MsgTag::Request.as_u32();
        let result = copydata_payload(dw_data as usize, payload.len(), payload);
        assert_eq!(result, Ok((MsgTag::Request, payload)));
    }

    #[test]
    fn framing_rejects_length_mismatch() {
        let payload: &[u8] = b"1234";
        let dw_data = MsgTag::Request.as_u32();
        assert!(copydata_payload(dw_data as usize, payload.len() + 1, payload).is_err());
        assert!(copydata_payload(dw_data as usize, payload.len() - 1, payload).is_err());
    }

    #[test]
    fn framing_rejects_unknown_tag() {
        let payload: &[u8] = b"";
        assert!(copydata_payload(6usize, payload.len(), payload).is_err());
    }

    #[test]
    fn framing_accepts_empty_payload() {
        let payload: &[u8] = b"";
        let dw_data = MsgTag::Hello.as_u32();
        assert_eq!(
            copydata_payload(dw_data as usize, 0, payload),
            Ok((MsgTag::Hello, payload))
        );
    }
}

/// 送出の向きごとの旗の檻（応答方向を「応答なし」判定で打ち切ってはならない）。
#[cfg(test)]
mod send_flavor_tests {
    use super::*;

    #[test]
    fn response_flavor_has_no_abort_if_hung() {
        let flags = send_flags(SendFlavor::Response);
        assert_eq!(
            flags.0 & SMTO_ABORTIFHUNG.0,
            0,
            "応答方向は SMTO_ABORTIFHUNG を付けない（要件 16.1）: {flags:?}"
        );
    }

    #[test]
    fn request_flavor_keeps_abort_if_hung() {
        let flags = send_flags(SendFlavor::Request);
        assert_ne!(
            flags.0 & SMTO_ABORTIFHUNG.0,
            0,
            "要求方向は SMTO_ABORTIFHUNG を保つ（要件 16.1 後段）: {flags:?}"
        );
    }

    // 区別が名前だけの見せかけでないことの固定。
    #[test]
    fn the_two_flavors_do_not_share_the_same_flags() {
        assert_ne!(
            send_flags(SendFlavor::Request).0,
            send_flags(SendFlavor::Response).0,
            "向きの区別は旗の実差に対応する（要件 16.2）"
        );
    }

    #[test]
    fn response_flavor_is_plain_normal() {
        assert_eq!(send_flags(SendFlavor::Response).0, SMTO_NORMAL.0);
    }
}
