//! host-32 IPC WM_COPYDATA ワイヤ規約（framing 部分）。
//!
//! x64/arm64 親プロセスと i686 helper プロセスの間で生バイト列を往復させる
//! transport 層のうち、**メッセージ framing・HWND 符号化・不正フレーム検出**を担う。
//! 下流と凍結共有する単一ソース。
//!
//! 設計制約（design.md §272-336・要件 2.1-2.5 / 7.2 / 7.3）:
//! - `MsgTag` は `dwData` の**低 32bit のみ**に載る（跨ビットネス安全）。
//! - payload は**生バイト列**（`cbData` = 長さ・固定ヘッダ長 0 = [`PAYLOAD_HEADER_LEN`]）。
//!   ポインタ・HANDLE・struct を境界を越えて共有しない。
//! - HWND は**常に u32 LE 4 バイト**でワイヤ表現する。
//! - shift/mask 評価は必ず `u64` cast で行い、i686 の `usize` = 32bit での
//!   overflow を回避する（要件 7.2）。
//!
//! 送信プリミティブ（`send_copydata` / `send_request`）・`ResponseSlot`・
//! `IpcError` も本モジュールに同居する（跨ビットネス共有の単一ソース）。
//! 送信関数は `SendMessageTimeoutW`（`SMTO_ABORTIFHUNG` ＋上限時間）で
//! 無限待機を構造的に排除する（要件 5.3）。実往復の再入受領は実窓を要すため
//! 統合タスクで検証する（本モジュールは型・timeout 契約・slot 消費の確立まで）。

use core::cell::RefCell;
use core::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::WindowsAndMessaging::{SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_COPYDATA};

/// 跨プロセス payload の固定ヘッダ長。
///
/// 本規約では payload は生バイト列そのものであり、長さ以外の in-band ヘッダを
/// 持たない（要件 2.4）。`cbData` が payload 長そのものであることを明示する定数。
pub const PAYLOAD_HEADER_LEN: usize = 0;

/// WM_COPYDATA メッセージ種別タグ（`dwData` の低 32bit に載る）。
///
/// 判別子は全て u32 の低 32bit に収まり、跨ビットネス安全である
/// （`(tag.as_u32() as u64) >> 32 == 0`・要件 7.2）。
///
/// 本ユニットが直接扱うのは `Hello` / `Request` / `Response`（echo）である。
/// `Load` / `Unload` はワイヤ互換のため定義するが、本ユニットでは未処理で
/// 下流ユニットが結線する。
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgTag {
    /// helper → 親: 自身のメッセージ窓 HWND を u32 LE で通知するハンドシェイク。
    Hello = 1,
    /// pasta.dll ロード要求（本ユニット未処理・下流で結線）。
    Load = 2,
    /// 親 → helper: 生バイト request。
    Request = 3,
    /// helper → 親: 生バイト response。
    Response = 4,
    /// pasta.dll アンロード要求（本ユニット未処理・下流で結線）。
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
    /// 未知の生値は不正フレームの観測点として `Err(raw)` を返す（要件 2.5）。
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

/// HWND を **u32 リトルエンディアン 4 バイト**へ符号化する（要件 2.2）。
///
/// HWND は USER ハンドルゆえ 32bit 有意であり、ワイヤは常に u32 LE。
/// x64/arm64 では HWND は 64bit だが下位 32bit のみをワイヤへ載せる。
/// 抽出は `u64` cast 経由で mask し、i686 の `usize` = 32bit での
/// shift overflow を回避する（要件 7.2）。
#[inline]
pub fn encode_hwnd_le(hwnd: HWND) -> [u8; 4] {
    // HWND(*mut c_void) の数値を usize で取り出し、u64 へ widen して下位 32bit を mask。
    let raw = hwnd.0 as usize as u64;
    let low = (raw & 0xFFFF_FFFF) as u32;
    low.to_le_bytes()
}

/// u32 LE 4 バイトから HWND の 32bit 有意値を復元する（要件 2.2・2.3）。
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
/// 検証（要件 2.5・不正フレーム検出）:
/// - 未知タグ（`try_from_u32` が `Err`）→ `Err`。
/// - 宣言長 `cbData` と実 payload 長の不整合 → `Err`。
///
/// `dw_data` の型は `usize`（`COPYDATASTRUCT.dwData` は跨ビットネスで幅が異なる）。
/// 低 32bit のみを `u64` cast 経由で取り出してタグ判定するため、i686 の
/// `usize` = 32bit でも overflow しない（要件 7.2）。
pub fn copydata_payload(
    dw_data: usize,
    declared_len: usize,
    data: &[u8],
) -> Result<(MsgTag, &[u8]), FramingError> {
    // dwData の低 32bit のみが種別タグとして有意（要件 2.1 / 7.2）。
    let raw_tag = ((dw_data as u64) & 0xFFFF_FFFF) as u32;
    let tag = MsgTag::try_from_u32(raw_tag).map_err(FramingError::UnknownTag)?;

    // 宣言長 (cbData) と実 payload 長が一致しないフレームは破損として拒否する
    // （要件 2.5・PAYLOAD_HEADER_LEN = 0 ゆえ cbData = payload 長）。
    if declared_len != data.len() {
        return Err(FramingError::LengthMismatch {
            declared: declared_len,
            actual: data.len(),
        });
    }

    Ok((tag, data))
}

/// framing レベルの不正フレーム表現（要件 2.5）。
///
/// 本ユニットは framing ローカルな軽量エラーに留める。下流タスクの
/// `IpcError`（thiserror enum）へは、この列挙の各バリアントを
/// `IpcError::CorruptFrame` 等へ `From` で持ち上げて統合する想定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramingError {
    /// `dwData` の低 32bit が既知の [`MsgTag`] でない（未知タグ）。
    UnknownTag(u32),
    /// 宣言長 `cbData` と実 payload 長が不整合。
    LengthMismatch { declared: usize, actual: usize },
}

/// 送信・往復トランスポート層の構造化エラー（要件 5.2 / 5.3・design.md §486-506）。
///
/// **一様な失敗報告**: peer の生死やハングを distinct なバリアントで区別せず、
/// 到達不能・timeout・ハング peer の中断（`SMTO_ABORTIFHUNG`）は一律
/// `Timeout` / `SendFailed` に集約する（distinct な PeerGone は設けない）。
/// peer の生死は送信結果に混ぜず、上位の `ExitKind`（ProcessHost・別タスク）で
/// 観測する（Requirement 1 と 5 の分離）。
///
/// framing レベルの不正フレーム（[`FramingError`]）は `CorruptFrame` へ
/// `From` で持ち上げ、呼び出し側が `?` で一様に扱えるようにする
/// （task 2.1 CONCERNS 統合）。
#[derive(thiserror::Error, Debug)]
pub enum IpcError {
    /// 応答が上限時間内に返らなかった（要件 5.2）、または
    /// ハング peer が `SMTO_ABORTIFHUNG` で中断された（要件 5.3）。
    #[error("ipc request timed out or peer was hung")]
    Timeout,
    /// `SendMessageTimeoutW` が 0 を返した（送出そのものの失敗）。
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

/// single-in-flight の応答受け皿（要件 4.1 / 4.3・design.md §328-331）。
///
/// UI スレッド固定かつ往復は同時に 1 つ（single-in-flight）ゆえ、内部同期は
/// [`RefCell`] で足りる（`Mutex` 不要）。応答 WndProc が再入で [`store`](Self::store)
/// し、[`send_request`] 側が [`take`](Self::take) する共有参照モデルであり、
/// `send_request` と WndProc は同一の `&ResponseSlot` を参照する。
///
/// 状態機械: `clear`（残骸排除＝None 化）→ 送信ブロック中に WndProc が `store`
/// → 復帰後 `take` で 1 回消費（再 `take` は `None`）。
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
/// `u32::MAX` ミリ秒（約 49.7 日）で上限をクランプし、無限待機を構造的に
/// 排除する（要件 5.3）。
#[inline]
fn timeout_millis(timeout: Duration) -> u32 {
    timeout.as_millis().min(u32::MAX as u128) as u32
}

/// 生バイト payload を WM_COPYDATA として片道送出する（要件 2.1 / 5.3・design.md §315）。
///
/// `SendMessageTimeoutW` に `SMTO_ABORTIFHUNG` ＋上限時間を与え、送出先がハング中でも
/// 上限時間で復帰する（無限待機の構造的排除）。戻り 0（失敗 / timeout / hung peer）は
/// 一律 [`IpcError::SendFailed`] とする。
///
/// - `dwData`: `tag.as_u32()`（低 32bit のみ有意・跨ビットネス安全）。
/// - `cbData`: `payload.len()`（メッセージ境界＝固定ヘッダ長 0）。
/// - `lpData`: `payload` を指す。ポインタ自体はワイヤに載らず OS が `cbData` バイトを marshal する。
/// - `wParam`: 送信側自身の HWND（`self_hwnd`）。
///
/// # Safety
/// `SendMessageTimeoutW` は FFI。`target` は有効な HWND（ハンドシェイク確定済）であること、
/// `payload` は本呼び出し中生存すること（`SendMessage` は同期ゆえ復帰まで生存が保証される）。
pub fn send_copydata(
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
    // SAFETY: target は有効 HWND 前提。&cds は本呼び出し中生存し、その lpData が指す
    // payload も同期送信中生存する。SMTO_ABORTIFHUNG ＋ timeout_ms により無期限
    // ブロックしない（要件 5.3）。
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

    // 戻り 0 = 送出失敗 / timeout / hung peer の中断。一律失敗として報告する
    // （distinct PeerGone を設けない・design.md §286）。
    if ret.0 == 0 {
        return Err(IpcError::SendFailed);
    }
    Ok(())
}

/// 1 往復送信（再入受領・single-in-flight・要件 4.1 / 4.3 / 5.2・design.md §319）。
///
/// 手順:
///   1. `slot.clear()` — 受け皿を空に（single-in-flight の前提確立）。
///   2. [`send_copydata`] で REQUEST を同期送出。親はここでブロックし、待機中に
///      Windows が helper の RESPONSE（2nd WM_COPYDATA）を親 WndProc へ再入配送し、
///      WndProc が `slot.store()` して即 return する（跨プロセス SendMessage を発行しない）。
///   3. 復帰後 `slot.take()` — `Some(bytes)` なら `Ok`、`None`（上限内未受領）なら
///      [`IpcError::Timeout`]。
///
/// `slot` は親の応答 WndProc が参照するものと同一 `&ResponseSlot`。往復は同時に 1 つ。
///
/// **注意**: 実際の再入往復は実窓を要すため統合 task（4.3 / 5.1）で検証する。本関数は
/// timeout 契約と slot 消費ロジックの確立まで。
///
/// # Safety
/// [`send_copydata`] の Safety 前提（有効 `target`・`payload` 生存）を引き継ぐ。
pub fn send_request(
    target: HWND,
    self_hwnd: HWND,
    tag: MsgTag,
    payload: &[u8],
    timeout: Duration,
    slot: &ResponseSlot,
) -> Result<Vec<u8>, IpcError> {
    // ① 受け皿を空に（前回残骸排除・single-in-flight）。
    slot.clear();

    // ② REQUEST を同期送出。ブロック中に RESPONSE が再入配送され slot.store される。
    send_copydata(target, self_hwnd, tag, payload, timeout)?;

    // ③ 復帰後に受領済み応答を取り出す。未受領（None）は timeout。
    slot.take().ok_or(IpcError::Timeout)
}

#[cfg(test)]
mod slot_error_tests {
    use super::*;

    // Unit Test 3（design 533・要件 4.3）: ResponseSlot clear/store/take semantics
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
        // single-in-flight 前提: 最新格納で上書き
        let slot = ResponseSlot::new();
        slot.store(b"first".to_vec());
        slot.store(b"second".to_vec());
        assert_eq!(slot.take(), Some(b"second".to_vec()));
    }

    // IpcError バリアント構築・Display/Debug（要件 5.2/5.3）
    #[test]
    fn ipc_error_variants_display() {
        let timeout = IpcError::Timeout;
        let send_failed = IpcError::SendFailed;
        assert!(!format!("{timeout}").is_empty());
        assert!(!format!("{send_failed}").is_empty());
        assert!(!format!("{timeout:?}").is_empty());
    }

    // From<FramingError> 持ち上げ（task 2.1 CONCERNS 統合）
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

    // (a) MsgTag u32 往復ロスレス（要件 2.1）
    #[test]
    fn msgtag_u32_roundtrip_lossless() {
        for tag in ALL_TAGS {
            let raw = tag.as_u32();
            assert_eq!(MsgTag::try_from_u32(raw), Ok(tag));
        }
    }

    // (b) 低 32bit 占有: (as_u32() as u64) >> 32 == 0（要件 7.2）
    #[test]
    fn msgtag_occupies_low_32_bits_only() {
        for tag in ALL_TAGS {
            let widened = tag.as_u32() as u64;
            assert_eq!(widened >> 32, 0, "tag {tag:?} must fit low 32 bits");
        }
    }

    // (c) 未知タグは Err(raw)（要件 2.5・不正フレーム観測点）
    #[test]
    fn msgtag_unknown_is_err() {
        for raw in [0u32, 6u32, 0xFFFF_FFFF] {
            assert_eq!(MsgTag::try_from_u32(raw), Err(raw));
        }
    }

    // (d) HWND u32 LE 往復（要件 2.2・7.3）
    #[test]
    fn hwnd_u32_le_roundtrip_and_byte_order() {
        for value in [0x1u32, 0x1234_5678, 0xDEAD_BEEF, u32::MAX] {
            let hwnd = hwnd_from_u32(value);
            let bytes = encode_hwnd_le(hwnd);
            assert_eq!(bytes, value.to_le_bytes(), "LE byte order for {value:#x}");
            assert_eq!(decode_hwnd_le(bytes), value, "roundtrip for {value:#x}");
        }
    }

    // framing: 正当フレームで Ok((tag, payload))（要件 2.1・2.3・2.4）
    #[test]
    fn framing_accepts_valid_frame() {
        let payload: &[u8] = b"hello-echo";
        let dw_data = MsgTag::Request.as_u32();
        let result = copydata_payload(dw_data as usize, payload.len(), payload);
        assert_eq!(result, Ok((MsgTag::Request, payload)));
    }

    // framing: cbData ≠ 実長 → Err（要件 2.5）
    #[test]
    fn framing_rejects_length_mismatch() {
        let payload: &[u8] = b"1234";
        let dw_data = MsgTag::Request.as_u32();
        // 宣言長 (cbData) が実長より大きい
        assert!(copydata_payload(dw_data as usize, payload.len() + 1, payload).is_err());
        // 宣言長 (cbData) が実長より小さい
        assert!(copydata_payload(dw_data as usize, payload.len() - 1, payload).is_err());
    }

    // framing: 未知タグ生値 → Err（要件 2.5）
    #[test]
    fn framing_rejects_unknown_tag() {
        let payload: &[u8] = b"";
        assert!(copydata_payload(6usize, payload.len(), payload).is_err());
    }

    // framing: 空 payload の正当フレーム
    #[test]
    fn framing_accepts_empty_payload() {
        let payload: &[u8] = b"";
        let dw_data = MsgTag::Hello.as_u32();
        assert_eq!(
            copydata_payload(dw_data as usize, 0, payload),
            Ok((MsgTag::Hello, payload))
        );
    }

    // PAYLOAD_HEADER_LEN=0 の明示（要件 2.4・固定ヘッダ長 0）
    #[test]
    fn payload_header_len_is_zero() {
        assert_eq!(PAYLOAD_HEADER_LEN, 0);
    }
}
