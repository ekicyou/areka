//! task 2.2 の `InProcHost` 4 メソッド単体檻（design.md §InProcHost・要件 3.1/7.4）。
//!
//! 全て決定論（DLL/COM アパートメント不要）で、`IShioriHost` 型付き面を通して駆動する
//! （interface.rs `host_sink_all_methods_dispatch` と同律）。
//! - SetProperty → GetProperty 往復（格納値の move-out）。
//! - 欠落 key の GetProperty → `SHIORI_E_PROPERTY_NOT_FOUND`（out_value 未書込）。
//! - Raise → `Ok(())`（M1 InProc に消費者なし・warn 記録のみ）。
//! - Complete（任意トークン）→ `SHIORI_E_UNKNOWN_TOKEN`（deferred 非対応＝pending 枠なし・要件 7.4）。

use super::*;
use shiori_abi::error::{SHIORI_E_PROPERTY_NOT_FOUND, SHIORI_E_UNKNOWN_TOKEN};
use shiori_abi::interface::IShioriHost;

/// SetProperty → GetProperty 往復で格納値が move-out されること（プロパティ単純往復・要件 7.4）。
#[test]
fn set_then_get_property_roundtrips() {
    let host: IShioriHost = InProcHost::new().into();

    let key = HSTRING::from("path.to.key");
    let value = HSTRING::from("some-value");
    unsafe { host.SetProperty(&key, &value) }.expect("SetProperty は Ok であること");

    let mut out_value = HSTRING::new();
    unsafe { host.GetProperty(&key, &mut out_value) }.expect("GetProperty は Ok であること");
    assert_eq!(out_value, value, "設定した値が move-out されること");
}

/// 欠落 key の GetProperty は `SHIORI_E_PROPERTY_NOT_FOUND` で失敗し out_value を書かないこと
/// （欠落 key・design.md §InProcHost）。
#[test]
fn get_missing_property_returns_property_not_found() {
    let host: IShioriHost = InProcHost::new().into();

    let missing = HSTRING::from("no.such.key");
    // 未書込の観測用に非空の番兵値を置き、失敗経路で不変であることを確かめる。
    let sentinel = HSTRING::from("__unwritten__");
    let mut out_value = sentinel.clone();
    let err = unsafe { host.GetProperty(&missing, &mut out_value) }
        .expect_err("欠落 key の GetProperty は error であること");
    assert_eq!(
        err.code(),
        SHIORI_E_PROPERTY_NOT_FOUND,
        "欠落 key は SHIORI_E_PROPERTY_NOT_FOUND であること"
    );
    assert_eq!(out_value, sentinel, "欠落 key では out_value を書き込まないこと");
}

/// Raise は消費者不在でも `Ok(())` を返すこと（warn 可視化・握りつぶさない・要件 7.4）。
#[test]
fn raise_returns_ok_without_consumer() {
    let host: IShioriHost = InProcHost::new().into();

    let script = HSTRING::from("\\h\\s[0]hello");
    unsafe { host.Raise(&script) }.expect("Raise は受領して Ok を返すこと（消費者なし）");
}

/// Complete は任意トークンで `SHIORI_E_UNKNOWN_TOKEN`（deferred 非対応・pending 枠なし・要件 7.4）。
#[test]
fn complete_any_token_returns_unknown_token() {
    let host: IShioriHost = InProcHost::new().into();

    let response = HSTRING::from("response-body");
    let err = unsafe { host.Complete(12345, &response) }
        .expect_err("deferred 非対応ゆえ Complete は error であること");
    assert_eq!(
        err.code(),
        SHIORI_E_UNKNOWN_TOKEN,
        "任意トークンの Complete は SHIORI_E_UNKNOWN_TOKEN であること"
    );
}
