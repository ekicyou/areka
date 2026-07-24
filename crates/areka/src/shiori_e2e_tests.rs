//! 遅延応答と push 経路の **end-to-end 結合テスト**（新 ABI 追随・要件 10.1/12.1/12.6）。
//!
//! `shiori_host.rs`（sink）/`shiori_session.rs`（session）の inline 単体テストと**重複させず**、
//! design.md §System Flows WS-B の遅延経路を 1 シナリオで通す:
//!
//! 1. モック脳の `Get` が `SHIORI_S_PENDING`＋既知 token を返す（遅延）。
//! 2. areka（[`ShioriSession`]）が `Deferred(token)` を受け、host 突合枠へ token をセットして保留する。
//! 3. **後で脳自身が**、`CreateInstance` で受け取り保持した host に対し safe `complete(token, response)`
//!    を呼ぶ（脳→host・vtable 直呼び廃止・pub 化した安全面メソッド）。
//! 4. areka sink が token を突合枠と突き合わせて応答をメールボックスへ配送し、保留を解除する。
//! 5. `poll_completions` で `Completed{token,response}` を取り出し内容一致を確認する。
//!
//! あわせて、脳の能動通知 `host.raise(script)` がメールボックスへ届き内容一致すること、
//! stale/未知トークンの `host.complete` が
//! [`ShioriError::UnknownToken`](shiori_abi::error::ShioriError) で拒否されることを 1 ハーネスで検証する。
//!
//! ## モック脳（脳→host を safe メソッドで駆動できる版）
//! `CreateInstance` で受けた host を保持し、`fire_complete`/`fire_raise` で保持 host の
//! snake_case 安全面（`complete`/`raise`）を呼ぶ [`DeferringBrain`]／[`DeferringFactory`] を用意する。
//! vtable 直呼びヘルパ（`call_*`）はメソッド pub 化により全廃した（要件 12.5）。

#![allow(non_snake_case)]

use shiori_abi::error::ShioriError;
use shiori_abi::interface::{
    IShiori, IShiori_Impl, IShioriFactory, IShioriFactory_Impl, IShioriHost,
};
use shiori_abi::outcome::CorrelationToken;
use windows_core::{AsImpl, HRESULT, HSTRING, OutRef, Ref, Result as ComResult, implement};

use crate::shiori_host::HostMessage;
use crate::shiori_session::{SessionRequest, ShioriSession};

/// 遅延 request で脳が発行する固定相関トークン。
const DEFERRED_TOKEN: u64 = 0x0051_0052;
/// 遅延完了で脳が後から届ける応答本文。
const DEFERRED_RESPONSE: &str = "deferred-e2e-body";
/// 脳が能動通知で届けるさくらスクリプト相当の本文。
const RAISE_SCRIPT: &str = "\\h\\s[0]wakeup-from-brain";

/// `Get` で `SHIORI_S_PENDING`＋既知 token を返し、後から保持 host へ safe `complete`/`raise` を
/// 発火できるモック脳（脳→host を駆動する end-to-end 版）。
#[implement(IShiori)]
struct DeferringBrain {
    /// `CreateInstance` で受け取った host を保持する（AddRef 相当の COM 参照）。脳→host 駆動に用いる。
    held_host: IShioriHost,
}

impl DeferringBrain {
    /// 保持 host へ safe `complete(token, response)` を発火する（脳→host・後送り完了）。
    fn fire_complete(&self, token: u64, response: &HSTRING) -> Result<(), ShioriError> {
        self.held_host.complete(CorrelationToken(token), response)
    }

    /// 保持 host へ safe `raise(script)` を発火する（脳→host・能動通知）。
    fn fire_raise(&self, script: &HSTRING) -> Result<(), ShioriError> {
        self.held_host.raise(script)
    }
}

impl IShiori_Impl for DeferringBrain_Impl {
    unsafe fn Get(
        &self,
        _input: &HSTRING,
        _out_response: &mut HSTRING,
        out_token: &mut u64,
    ) -> HRESULT {
        // 遅延: 相関トークンを out_token へ書き、out_response は空のまま。後で complete で応答する。
        *out_token = DEFERRED_TOKEN;
        shiori_abi::error::SHIORI_S_PENDING
    }

    unsafe fn Notify(&self, _input: &HSTRING) -> ComResult<()> {
        Ok(())
    }
}

/// `DeferringBrain` を生成するモック factory（host を脳へ保持させる）。
#[implement(IShioriFactory)]
struct DeferringFactory;

impl IShioriFactory_Impl for DeferringFactory_Impl {
    unsafe fn CreateInstance(
        &self,
        _load_dir: &HSTRING,
        _shiori_name: &HSTRING,
        host: Ref<'_, IShioriHost>,
        out: OutRef<'_, IShiori>,
    ) -> ComResult<()> {
        let host: IShioriHost = host
            .as_ref()
            .ok_or_else(|| windows_core::Error::from(windows::Win32::Foundation::E_POINTER))?
            .clone();
        let brain: IShiori = DeferringBrain { held_host: host }.into();
        out.write(Some(brain))?;
        Ok(())
    }
}

/// end-to-end: 遅延 get → 脳が後で `host.complete` → areka が token 突合で配送 → poll で取り出し。
///
/// あわせて同一シナリオ内で `host.raise`（能動通知）の配送、stale/未知トークンの `host.complete`
/// 拒否（`UnknownToken`）まで 1 本で通す（要件 10.1/12.1・議題3）。
///
/// session は brain を move で保持するため、脳駆動用に別途 create した `DeferringBrain` を用いる。
/// session と脳駆動 brain が同一 host（sink）を共有するよう、host を明示的に生成して両者へ渡す。
#[test]
fn deferred_completion_and_push_delivered_end_to_end() {
    // --- host（sink）を明示生成し、脳と session で共有する（脳→host の配送を session が観測する）。
    let factory: IShioriFactory = DeferringFactory.into();
    // sink は sylphya 委譲済み（第 2 ストア撤去・Task 9.1/9.3）。hermetic な偽 IO sink（既知 asker）。
    let host: IShioriHost = crate::shiori_host::spawn_test_sylphya_sink().sink.into();

    // factory で脳を生成（host を脳へ保持させる）。session はこの脳を move で保持する。
    let brain = factory
        .create(&HSTRING::from("dir"), &HSTRING::from("name"), &host)
        .expect("create は Ok");
    // 脳駆動用ハンドル（session が move するため clone で確保）。
    let brain_handle = brain.clone();
    let mut session = ShioriSession::from_parts(brain, host.clone());

    let brain_inner = unsafe { AsImpl::<DeferringBrain>::as_impl(&brain_handle) };

    // --- 遅延 get: 脳が SHIORI_S_PENDING+token を返し、areka は Deferred で保留する。
    let content = HSTRING::from("hrequest-content");
    let outcome = session.get(&content).expect("遅延 get は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Deferred(CorrelationToken(DEFERRED_TOKEN)),
        "遅延はトークン付き Deferred を返すこと"
    );
    assert!(session.is_pending(), "遅延後は保留状態（単一 in-flight）であること");

    // --- 能動通知: 脳が保持 host へ raise を発火し、areka sink のメールボックスへ届くこと。
    let script = HSTRING::from(RAISE_SCRIPT);
    brain_inner.fire_raise(&script).expect("脳→host->raise は Ok");

    // --- stale/未知トークン: 突合枠(DEFERRED_TOKEN)と不一致の complete は弾かれること（議題3）。
    let bogus = HSTRING::from("bogus");
    let bogus_err = brain_inner
        .fire_complete(DEFERRED_TOKEN ^ 0xFFFF, &bogus)
        .expect_err("未知トークンの complete は Err");
    assert!(
        matches!(bogus_err, ShioriError::UnknownToken),
        "未知トークンの complete は UnknownToken で拒否されること, got {bogus_err:?}"
    );
    assert!(
        session.is_pending(),
        "未知トークンの complete は保留枠を消費しないこと（突合不能）"
    );

    // --- 遅延完了: 脳が突合枠と一致する token で complete を発火し、areka が応答を配送する。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    brain_inner
        .fire_complete(DEFERRED_TOKEN, &response)
        .expect("一致トークンの complete は Ok");

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
        "raise 能動通知と遅延 Completed が token 突合・内容一致で配送されること"
    );

    // --- 保留解除: 一致 complete の受領で保留が解け、次 get が許可されること（議題3）。
    assert!(
        !session.is_pending(),
        "遅延完了の token 突合で保留が解除されること（単一 in-flight の解放）"
    );
    assert!(
        session.get(&content).is_ok(),
        "遅延完了後は次 get が可能になること"
    );

    // drop teardown（session drop→保留取消→brain drop）。
    drop(session);
    drop(brain_handle);
    drop(host);
}
