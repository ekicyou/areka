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
//! `IpcError` は下流タスクの領分ゆえ本モジュールには含まない。

use windows::Win32::Foundation::HWND;

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
