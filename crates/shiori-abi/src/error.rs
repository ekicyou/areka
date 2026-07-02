//! SHIORI ABI のエラー型・HRESULT 規約。
//!
//! - [`ShioriError`]: ergonomic 層が返す `thiserror` 派生エラー型（design.md §Error Handling, 3.6/7.2）。
//! - カスタム HRESULT 定数 [`SHIORI_S_PENDING`] / [`SHIORI_E_UNKNOWN_TOKEN`] /
//!   [`SHIORI_E_PROPERTY_NOT_FOUND`]: 成功の 2 値分岐（即時 `S_OK` / 遅延 `SHIORI_S_PENDING`）と
//!   失敗カテゴリを COM 規約（HRESULT）で表現する（design.md §Error Strategy/§Error Categories,
//!   要件 2.3/3.5/3.6/7.1/7.2/7.3/8.6）。
//! - [`hresult_to_shiori_error`]: 失敗 HRESULT を [`ShioriError`] 変種へマップする変換関数。
//!
//! ## HRESULT ビットレイアウト（32bit）と採番根拠
//! `HRESULT` は 32bit:
//! - bit 31 (`0x8000_0000`) = severity（1=失敗 / 0=成功）
//! - bit 29 (`0x2000_0000`) = customer bit（アプリケーション定義コードでセット）
//! - bit 16..=26          = FACILITY
//! - bit 0..=15           = code
//!
//! 本クレートは Microsoft 予約と衝突しないアプリケーション専用コードとするため、
//! 全カスタム HRESULT で **customer bit をセット**する（design.md §Error Strategy）。
//! FACILITY は予約済み値（FACILITY_ITF=0x4 等）を避け、SHIORI 専用に `0xA1` を採番する
//! （customer bit セット下では FACILITY は自由採番可能。0xA1 は他の areka 内 FACILITY と
//! 重複しないよう一意に選定）。code はカテゴリごとに 0x0001 から連番。

use windows_core::HRESULT;

/// SHIORI 専用 FACILITY 値（customer bit 前提の自由採番。一意性のため `0xA1` を採番）。
const FACILITY_SHIORI: i32 = 0xA1;

/// customer bit（bit 29）。アプリケーション定義 HRESULT を示す。
const CUSTOMER_BIT: i32 = 0x2000_0000;

/// severity bit（bit 31）。失敗 HRESULT を示す。`i32` では最上位ビットのため負値となる。
const SEVERITY_FAILURE: i32 = 0x8000_0000u32 as i32;

/// カスタム成功コードを構築する（severity=0・customer bit セット）。
const fn make_shiori_success(code: i32) -> HRESULT {
    HRESULT(CUSTOMER_BIT | (FACILITY_SHIORI << 16) | code)
}

/// カスタム失敗コードを構築する（severity=1・customer bit セット）。
const fn make_shiori_failure(code: i32) -> HRESULT {
    HRESULT(SEVERITY_FAILURE | CUSTOMER_BIT | (FACILITY_SHIORI << 16) | code)
}

/// 遅延応答を示すカスタム**成功** HRESULT（D1: `S_OK`=即時 / `SHIORI_S_PENDING`=遅延）。
///
/// severity=0（成功）・customer bit セット。値 `0x20A1_0001`。
/// ergonomic 層はこのコードを受けて `GetOutcome::Deferred` を構築する（要件 3.3/3.5）。
pub const SHIORI_S_PENDING: HRESULT = make_shiori_success(0x0001);

/// 突合不能/stale な相関トークンを示す失敗 HRESULT（host 側が返す。design.md §Error Categories）。
/// 値 `0xA0A1_0003`。
pub const SHIORI_E_UNKNOWN_TOKEN: HRESULT = make_shiori_failure(0x0003);

/// 欠落プロパティ key を示す失敗 HRESULT（host 側の `GetProperty` が返す。design.md §確定設計判断 (b)(f)）。
/// 値 `0xA0A1_0004`。暗黙の空値で続行せず、欠落を判別可能にする（design.md §Error Categories）。
pub const SHIORI_E_PROPERTY_NOT_FOUND: HRESULT = make_shiori_failure(0x0004);

/// SHIORI ABI の ergonomic 層エラー型（design.md §Components and Interfaces → Ergonomic Layer, 3.6/7.2）。
///
/// raw COM 層の `HRESULT` を Rust 風の `Result` へ型化する。HRESULT は [`windows_core::HRESULT`] を内包する。
#[derive(thiserror::Error, Debug)]
pub enum ShioriError {
    /// create（factory 経由の IShiori 構築・load 融合）が失敗した（要件 2.3/8.6）。
    /// 内包 HRESULT が失敗コードを保持する。半構築を露出しない（design.md §確定設計判断 (f)）。
    #[error("create failed: {0}")]
    CreateFailed(HRESULT),
    /// `Get` 操作が失敗した（要件 3.6/9.4）。内包 HRESULT が失敗コードを保持する。
    #[error("get failed: {0}")]
    GetFailed(HRESULT),
    /// `Notify` 操作が失敗した（要件 9.4）。内包 HRESULT が失敗コードを保持する。
    #[error("notify failed: {0}")]
    NotifyFailed(HRESULT),
    /// 突合不能/stale な相関トークン（[`SHIORI_E_UNKNOWN_TOKEN`] の型付き写像）。
    /// 従来 `Com` 落ちだったのを判別可能化する（design.md §Error Categories/§確定設計判断 (f)）。
    #[error("unknown correlation token")]
    UnknownToken,
    /// 欠落プロパティ key（[`SHIORI_E_PROPERTY_NOT_FOUND`] の型付き写像）。
    /// 暗黙の空値で続行しない（design.md §確定設計判断 (b)(f)）。
    #[error("property not found")]
    PropertyNotFound,
    /// その他の COM エラー（`windows_core::Error` 由来）。
    #[error("com error: {0}")]
    Com(#[from] windows_core::Error),
}

/// 失敗 HRESULT を [`ShioriError`] へマップする（design.md §Error Categories）。
///
/// 呼び出し規約: **成功扱いの HRESULT（`S_OK` / [`SHIORI_S_PENDING`]）を渡してはならない**。
/// 成功/失敗の判別は呼び出し側（ergonomic 層）が `HRESULT::is_ok()` で行い、
/// 失敗時のみ本関数で `ShioriError` 変種へ変換する。設計の「成功の 2 値分岐
/// （`S_OK`=即時 / `SHIORI_S_PENDING`=遅延）」を尊重し、本関数は失敗側のマッピングに専念する。
///
/// マッピング:
/// - [`SHIORI_E_UNKNOWN_TOKEN`] → [`ShioriError::UnknownToken`]（design.md §Error Categories）
/// - [`SHIORI_E_PROPERTY_NOT_FOUND`] → [`ShioriError::PropertyNotFound`]（design.md §確定設計判断 (b)(f)）
/// - その他の失敗 HRESULT → 呼び出し文脈に応じ呼び出し側が [`ShioriError::CreateFailed`] /
///   [`ShioriError::GetFailed`] / [`ShioriError::NotifyFailed`] を選ぶ。文脈が無い汎用変換としては
///   [`ShioriError::Com`]（`windows_core::Error::from(hr)`）へ落とす（要件 3.6/7.2）。
///
/// # Panics
/// `hr` が成功コード（`is_ok()`）の場合は呼び出し規約違反として panic する。
pub fn hresult_to_shiori_error(hr: HRESULT) -> ShioriError {
    debug_assert!(
        hr.is_err(),
        "hresult_to_shiori_error には失敗 HRESULT のみ渡すこと: 0x{:08X}",
        hr.0
    );
    match hr {
        SHIORI_E_UNKNOWN_TOKEN => ShioriError::UnknownToken,
        SHIORI_E_PROPERTY_NOT_FOUND => ShioriError::PropertyNotFound,
        other => ShioriError::Com(windows_core::Error::from(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_core::HRESULT;

    /// `S_OK` 相当（即時応答）の HRESULT。
    const S_OK: HRESULT = HRESULT(0);

    // --- HRESULT ビット規約の単体テスト（design.md §Error Strategy） ---

    #[test]
    fn s_ok_is_success() {
        // S_OK = 即時応答（D1）。成功コード。
        assert!(S_OK.is_ok());
        assert!(!S_OK.is_err());
    }

    #[test]
    fn shiori_s_pending_is_success_with_customer_bit() {
        // 遅延は「成功」コード（severity=0）かつ customer bit セット。
        assert!(SHIORI_S_PENDING.is_ok(), "SHIORI_S_PENDING は成功コードであること");
        assert!(!SHIORI_S_PENDING.is_err());
        assert_eq!(
            SHIORI_S_PENDING.0 & CUSTOMER_BIT,
            CUSTOMER_BIT,
            "customer bit がセットされていること"
        );
        assert_eq!(
            SHIORI_S_PENDING.0 & SEVERITY_FAILURE,
            0,
            "severity ビットが立っていない（=成功）こと"
        );
        // 採番値の固定（回帰防止）。
        assert_eq!(SHIORI_S_PENDING.0, 0x20A1_0001u32 as i32);
    }

    #[test]
    fn shiori_e_unknown_token_is_failure_with_customer_bit() {
        assert!(SHIORI_E_UNKNOWN_TOKEN.is_err());
        assert_eq!(SHIORI_E_UNKNOWN_TOKEN.0 & CUSTOMER_BIT, CUSTOMER_BIT);
        assert_eq!(SHIORI_E_UNKNOWN_TOKEN.0 & SEVERITY_FAILURE, SEVERITY_FAILURE);
        assert_eq!(SHIORI_E_UNKNOWN_TOKEN.0, 0xA0A1_0003u32 as i32);
    }

    #[test]
    fn shiori_e_property_not_found_is_failure_with_customer_bit() {
        // 新設: 欠落プロパティ key を示す失敗 HRESULT（採番固定・回帰防止）。
        assert!(
            SHIORI_E_PROPERTY_NOT_FOUND.is_err(),
            "SHIORI_E_PROPERTY_NOT_FOUND は失敗コードであること"
        );
        assert_eq!(SHIORI_E_PROPERTY_NOT_FOUND.0 & CUSTOMER_BIT, CUSTOMER_BIT);
        assert_eq!(
            SHIORI_E_PROPERTY_NOT_FOUND.0 & SEVERITY_FAILURE,
            SEVERITY_FAILURE,
            "severity（失敗）ビットがセットされていること"
        );
        assert_eq!(SHIORI_E_PROPERTY_NOT_FOUND.0, 0xA0A1_0004u32 as i32);
    }

    #[test]
    fn custom_codes_are_distinct() {
        // 全カスタム定数が一意に採番されていること。
        let codes = [
            SHIORI_S_PENDING.0,
            SHIORI_E_UNKNOWN_TOKEN.0,
            SHIORI_E_PROPERTY_NOT_FOUND.0,
        ];
        for (i, a) in codes.iter().enumerate() {
            for b in &codes[i + 1..] {
                assert_ne!(a, b, "カスタム HRESULT は一意であること");
            }
        }
    }

    // --- HRESULT ⇄ ShioriError マッピングの単体テスト（要件 3.5/3.6/7.2, design.md §Testing Strategy） ---

    #[test]
    fn unknown_token_maps_to_unknown_token_variant() {
        // SHIORI_E_UNKNOWN_TOKEN は型付き変種 UnknownToken へ判別可能に写像される（従来の Com 落ちを廃止）。
        let err = hresult_to_shiori_error(SHIORI_E_UNKNOWN_TOKEN);
        assert!(
            matches!(err, ShioriError::UnknownToken),
            "SHIORI_E_UNKNOWN_TOKEN は ShioriError::UnknownToken へマップされること, got {err:?}"
        );
    }

    #[test]
    fn property_not_found_maps_to_property_not_found_variant() {
        let err = hresult_to_shiori_error(SHIORI_E_PROPERTY_NOT_FOUND);
        assert!(
            matches!(err, ShioriError::PropertyNotFound),
            "SHIORI_E_PROPERTY_NOT_FOUND は ShioriError::PropertyNotFound へマップされること, got {err:?}"
        );
    }

    #[test]
    fn generic_failure_maps_to_com_variant_preserving_hresult() {
        // 専用変種を持たない汎用失敗 HRESULT（E_FAIL 相当）は Com へ落ち、元の HRESULT を保持する。
        let e_fail = HRESULT(0x8000_4005u32 as i32);
        let err = hresult_to_shiori_error(e_fail);
        match err {
            ShioriError::Com(e) => assert_eq!(e.code(), e_fail),
            other => panic!("expected ShioriError::Com, got {other:?}"),
        }
    }

    #[test]
    fn create_failed_wraps_hresult() {
        // 文脈依存変種（CreateFailed/GetFailed/NotifyFailed）は HRESULT を内包し、保持できること。
        let hr = HRESULT(0x8000_4005u32 as i32);
        let err = ShioriError::CreateFailed(hr);
        match err {
            ShioriError::CreateFailed(got) => assert_eq!(got, hr),
            other => panic!("expected CreateFailed, got {other:?}"),
        }
    }

    #[test]
    fn get_failed_wraps_hresult() {
        let hr = HRESULT(0x8000_4005u32 as i32);
        let err = ShioriError::GetFailed(hr);
        match err {
            ShioriError::GetFailed(got) => assert_eq!(got, hr),
            other => panic!("expected GetFailed, got {other:?}"),
        }
    }

    #[test]
    fn notify_failed_wraps_hresult() {
        let hr = HRESULT(0x8000_4005u32 as i32);
        let err = ShioriError::NotifyFailed(hr);
        match err {
            ShioriError::NotifyFailed(got) => assert_eq!(got, hr),
            other => panic!("expected NotifyFailed, got {other:?}"),
        }
    }

    #[test]
    fn com_error_from_conversion() {
        // #[from] windows_core::Error 経由の変換が成立すること。
        let com_err = windows_core::Error::from(HRESULT(0x8000_4005u32 as i32));
        let err: ShioriError = com_err.into();
        assert!(matches!(err, ShioriError::Com(_)));
    }
}
