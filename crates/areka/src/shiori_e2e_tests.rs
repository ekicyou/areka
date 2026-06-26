//! 遅延応答と push 経路の **end-to-end 結合テスト**（task 5.2・要件 3.3/6.1/6.3/6.4/6.5）。
//!
//! task 4.1（[`ShioriHostSink`](crate::shiori_host::ShioriHostSink)）/4.2
//! （[`ShioriSession`](crate::shiori_session::ShioriSession)）の inline 単体テストは、
//! `Complete`/`Raise` をテストコードから直接 vtable 駆動して個々の挙動を検証していた。
//! 本モジュールはそれらと**重複させず**、design.md §System Flows →「リクエスト 即時/遅延/失敗」
//! の遅延経路を 1 シナリオで通す:
//!
//! 1. モック脳の `Request` が `SHIORI_S_PENDING`＋既知 token を返す（遅延）。
//! 2. areka（[`ShioriSession`]）が `Deferred(token)` を受け、host 突合枠へ token をセットして保留する。
//! 3. **後で脳自身が**、`Load` で受け取り保持した host に対し
//!    `host.Complete(token, response)` を呼ぶ（脳→host・vtable 直呼び・Implementation Notes）。
//! 4. areka sink が token を突合枠と突き合わせて応答をメールボックスへ配送し、保留を解除する。
//! 5. `poll_completions` で `Completed{token,response}` を取り出し内容一致を確認する（要件 3.3/6.4）。
//!
//! あわせて、脳の能動通知 `host.Raise(script)` がメールボックスへ届き内容一致すること（要件 6.1/6.3/6.5）、
//! stale/未知トークンの `host.Complete` が
//! [`SHIORI_E_UNKNOWN_TOKEN`](shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN) で拒否されること（議題3）
//! を 1 ハーネスで検証する。
//!
//! ## モック脳（脳→host を駆動できる版）
//! 既存 4.2 テストの `MockBrain` は host を保持するだけだったが、本テストでは脳が
//! **テストの指示で後から** `host.Complete`/`host.Raise` を発火できる必要がある。そこで
//! `Load` で受けた host を保持し、`fire_complete`/`fire_raise` で保持 host を vtable 直呼びする
//! [`DeferringBrain`] を用意する（脳→host は raw メソッドが ABI 定義モジュール private のため
//! `Interface::vtable(host).<slot>(host.as_raw(), ..)` で呼ぶ・tasks.md §Implementation Notes）。

#![allow(non_snake_case)]

use core::cell::RefCell;
use core::ptr;

use shiori_abi::error::{SHIORI_E_UNKNOWN_TOKEN, SHIORI_S_PENDING};
use shiori_abi::interface::{IShiori, IShiori_Impl, IShioriHost};
use shiori_abi::outcome::CorrelationToken;
use windows_core::{AsImpl, HRESULT, HSTRING, Interface, implement};

use crate::shiori_host::{HostMessage, ShioriHostSink};
use crate::shiori_session::{SessionRequest, ShioriSession};

/// 遅延 request で脳が発行する固定相関トークン。
const DEFERRED_TOKEN: u64 = 0x0051_0052;
/// 遅延完了で脳が後から届ける応答本文。
const DEFERRED_RESPONSE: &str = "deferred-e2e-body";
/// 脳が能動通知で届けるさくらスクリプト相当の本文。
const RAISE_SCRIPT: &str = "\\h\\s[0]wakeup-from-brain";

/// `Request` で `SHIORI_S_PENDING`＋既知 token を返し、後から保持 host へ
/// `Complete`/`Raise` を発火できるモック脳（脳→host を駆動する end-to-end 版）。
///
/// `Load` で受け取った host を AddRef 保持し（議題2）、`fire_complete`/`fire_raise` で
/// 保持 host を vtable 直呼びする。`Unload` で host を Release する。
#[implement(IShiori)]
struct DeferringBrain {
    /// `Load` で受け取った host を保持する（AddRef 相当の COM 参照）。脳→host 駆動に用いる。
    held_host: RefCell<Option<IShioriHost>>,
}

impl DeferringBrain {
    fn new() -> Self {
        Self {
            held_host: RefCell::new(None),
        }
    }

    /// 保持 host へ vtable 直呼びで `Complete(token, response)` を発火する（脳→host・後送り完了）。
    ///
    /// # Panics
    /// host 未保持（`Load` 前 or `Unload` 後）に呼ぶと panic（テスト前提違反）。
    fn fire_complete(&self, token: u64, response: &HSTRING) -> HRESULT {
        let held = self.held_host.borrow();
        let host = held.as_ref().expect("host must be held (Load called, not Unloaded)");
        // 脳→host は raw メソッドが ABI 定義モジュール private のため vtable 直呼び（Implementation Notes）。
        // Safety: `host` は areka が所有する有効な IShioriHost、`response` は呼び出し中有効。
        unsafe { (Interface::vtable(host).Complete)(host.as_raw(), token, response as *const HSTRING) }
    }

    /// 保持 host へ vtable 直呼びで `Raise(script)` を発火する（脳→host・能動通知）。
    ///
    /// # Panics
    /// host 未保持に呼ぶと panic（テスト前提違反）。
    fn fire_raise(&self, script: &HSTRING) -> HRESULT {
        let held = self.held_host.borrow();
        let host = held.as_ref().expect("host must be held (Load called, not Unloaded)");
        // Safety: `host` は有効な IShioriHost、`script` は呼び出し中有効。
        unsafe { (Interface::vtable(host).Raise)(host.as_raw(), script as *const HSTRING) }
    }
}

impl IShiori_Impl for DeferringBrain_Impl {
    unsafe fn Load(&self, host: *mut core::ffi::c_void) -> HRESULT {
        // 受け取った raw host を型付き COM 参照として AddRef 保持する（議題2）。
        // Safety: host は areka が所有する有効な IShioriHost raw ポインタ（呼び出し中有効）。
        let borrowed: Option<&IShioriHost> = unsafe { IShioriHost::from_raw_borrowed(&host) };
        *self.held_host.borrow_mut() = borrowed.cloned();
        HRESULT(0) // S_OK
    }

    unsafe fn Unload(&self) -> HRESULT {
        // 保持 host を Release（drop）。以後 host を呼ばない（議題2）。
        *self.held_host.borrow_mut() = None;
        HRESULT(0) // S_OK
    }

    unsafe fn Request(
        &self,
        _input: *const HSTRING,
        _out_response: *mut HSTRING,
        out_token: *mut u64,
    ) -> HRESULT {
        // 遅延: 相関トークンを out_token へ書き、out_response は空のまま。後で Complete で応答する。
        unsafe { ptr::write(out_token, DEFERRED_TOKEN) };
        SHIORI_S_PENDING
    }
}

/// モック脳の実体参照を取り出す（脳→host 発火の駆動用）。
///
/// # Safety
/// `brain` は本テストで `DeferringBrain` から構築した COM ポインタであること。
fn brain_inner(brain: &IShiori) -> &DeferringBrain {
    unsafe { AsImpl::<DeferringBrain>::as_impl(brain) }
}

/// end-to-end: 遅延 request → 脳が後で `host.Complete` → areka が token 突合で配送 → poll で取り出し。
///
/// あわせて同一シナリオ内で `host.Raise`（能動通知）の配送、stale/未知トークンの
/// `host.Complete` 拒否（`SHIORI_E_UNKNOWN_TOKEN`）まで 1 本で通す（要件 3.3/6.1/6.3/6.4/6.5・議題3）。
#[test]
fn deferred_completion_and_push_delivered_end_to_end() {
    // --- アクティベーション: areka が脳へ Load で sink を渡す（脳が host を保持）。
    let brain: IShiori = DeferringBrain::new().into();
    let mut session = ShioriSession::activate(brain.clone()).expect("activate は成功すること");

    // --- 遅延 request: 脳が SHIORI_S_PENDING+token を返し、areka は Deferred で保留する。
    let content = HSTRING::from("hrequest-content");
    let outcome = session.request(&content).expect("遅延 request は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Deferred(CorrelationToken(DEFERRED_TOKEN)),
        "遅延はトークン付き Deferred を返すこと（要件 3.3）"
    );
    assert!(session.is_pending(), "遅延後は保留状態（単一 in-flight）であること");

    // --- 能動通知: 脳が保持 host へ Raise を発火し、areka sink のメールボックスへ届くこと（要件 6.3）。
    let script = HSTRING::from(RAISE_SCRIPT);
    let raise_hr = brain_inner(&brain).fire_raise(&script);
    assert!(raise_hr.is_ok(), "脳→host->Raise は S_OK: 0x{:08X}", raise_hr.0);

    // --- stale/未知トークン: 突合枠(DEFERRED_TOKEN)と不一致の Complete は弾かれること（議題3）。
    let bogus = HSTRING::from("bogus");
    let bogus_hr = brain_inner(&brain).fire_complete(DEFERRED_TOKEN ^ 0xFFFF, &bogus);
    assert_eq!(
        bogus_hr, SHIORI_E_UNKNOWN_TOKEN,
        "未知トークンの Complete は SHIORI_E_UNKNOWN_TOKEN で拒否されること"
    );
    assert!(
        session.is_pending(),
        "未知トークンの Complete は保留枠を消費しないこと（突合不能）"
    );

    // --- 遅延完了: 脳が突合枠と一致する token で Complete を発火し、areka が応答を配送する（要件 6.4）。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    let complete_hr = brain_inner(&brain).fire_complete(DEFERRED_TOKEN, &response);
    assert!(
        complete_hr.is_ok(),
        "一致トークンの Complete は S_OK: 0x{:08X}",
        complete_hr.0
    );

    // --- 配送確認: poll で Raised と Completed を FIFO で取り出し、token 突合と内容一致を確認する。
    let drained = session.poll_completions();
    assert_eq!(
        drained,
        vec![
            HostMessage::Raised(HSTRING::from(RAISE_SCRIPT)),
            HostMessage::Completed {
                token: CorrelationToken(DEFERRED_TOKEN),
                response: HSTRING::from(DEFERRED_RESPONSE),
            },
        ],
        "Raise 能動通知と遅延 Completed が token 突合・内容一致で配送されること（要件 3.3/6.3/6.4/6.5）"
    );

    // --- 保留解除: 一致 Complete の受領で保留が解け、次 request が許可されること（議題3・要件 3.3）。
    assert!(
        !session.is_pending(),
        "遅延完了の token 突合で保留が解除されること（単一 in-flight の解放）"
    );
    assert!(
        session.request(&content).is_ok(),
        "遅延完了後は次 request が可能になること"
    );
}
