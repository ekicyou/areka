//! ライフサイクルと単一 in-flight 規律の **end-to-end 結合テスト**（新 ABI 追随・要件 2.1/12.3）。
//!
//! 旧 ABI の `Load`/`Unload`/「未ロード拒否（NotLoaded）」は create 融合＋Drop teardown により消滅した
//! （要件 9.1/9.2）。本モジュールは新 ABI の **生成〜利用〜teardown の遷移** を通しで実証する:
//!
//! 1. **factory create でセッション確立**: `IShioriFactory::create` で load 完了済み `IShiori` を受領し
//!    セッションを確立する（「未ロード状態」は存在しない・要件 2.1）。
//! 2. **`get` の即時応答が受理・往復する**: Loaded/Unloaded の状態機械は無く、確立後は常に受理される。
//! 3. **drop teardown で参照不在**: セッションを drop すると保留取消→brain 参照 drop が起こり、以降は
//!    セッション参照が存在しない（型システムが検証を肩代わり・要件 2.1/12.3）。
//! 4. **`Deferred` 保留中の drop で保留取消 → 再 activate 後に正常動作**: 遅延保留が立った状態で drop
//!    すると host 側突合枠がクリアされ、再確立後に新規 get が正常に受理される（保留の持ち越しなし）。
//!
//! ## モック脳（design.md §InterfaceLayer）
//! 新 ABI の脳は `Get`（即時/遅延）＋`Notify` のみを持つ。本テストの [`StatefulBrain`] は即時/遅延を
//! 構築時に切替可能とし、host を保持する（脳→host 駆動用）。「未ロード拒否」は新 ABI に存在しない。

#![allow(non_snake_case)]

use shiori_abi::interface::{
    IShiori, IShiori_Impl, IShioriFactory, IShioriFactory_Impl, IShioriHost,
};
use shiori_abi::outcome::CorrelationToken;
use windows_core::{AsImpl, HRESULT, HSTRING, OutRef, Ref, Result as ComResult, implement};

use crate::shiori_session::{SessionError, SessionRequest, ShioriSession};

/// 遅延 get で脳が発行する固定相関トークン。
const DEFERRED_TOKEN: u64 = 0x00AA_00BB;
/// 即時応答する固定本文。
const IMMEDIATE_RESPONSE: &str = "lifecycle-immediate-body";

/// 即時/遅延を切替可能なモック脳（host を保持）。新 ABI ゆえ Load/Unload・未ロード拒否は無い。
#[implement(IShiori)]
struct StatefulBrain {
    /// 遅延モードか（`true` で `Get` が `SHIORI_S_PENDING`＋token を返す）。
    deferred: bool,
    /// `CreateInstance` で受け取った host を保持する（AddRef 相当・脳→host 駆動用）。
    #[allow(dead_code)]
    held_host: IShioriHost,
}

impl IShiori_Impl for StatefulBrain_Impl {
    unsafe fn Get(
        &self,
        _input: &HSTRING,
        out_response: &mut HSTRING,
        out_token: &mut u64,
    ) -> HRESULT {
        if self.deferred {
            *out_token = DEFERRED_TOKEN;
            shiori_abi::error::SHIORI_S_PENDING
        } else {
            *out_response = HSTRING::from(IMMEDIATE_RESPONSE);
            HRESULT(0) // S_OK
        }
    }

    unsafe fn Notify(&self, _input: &HSTRING) -> ComResult<()> {
        Ok(())
    }
}

/// `StatefulBrain` を生成するモック factory（即時/遅延を切替）。
#[implement(IShioriFactory)]
struct StatefulFactory {
    deferred: bool,
}

impl IShioriFactory_Impl for StatefulFactory_Impl {
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
        let brain: IShiori = StatefulBrain {
            deferred: self.deferred,
            held_host: host,
        }
        .into();
        out.write(Some(brain))?;
        Ok(())
    }
}

/// factory create でセッションを確立し、即時 get が受理・往復すること。drop teardown 後は参照不在。
///
/// 新 ABI の遷移（生成→利用→teardown）を通しで実証する。旧「未ロード拒否」は create 融合で消滅した。
#[test]
fn create_activate_immediate_get_then_drop_teardown() {
    let factory: IShioriFactory = StatefulFactory { deferred: false }.into();
    let mut session =
        ShioriSession::activate(&factory, &HSTRING::from("dir"), &HSTRING::from("name"))
            .expect("factory create でセッション確立できること（要件 2.1）");

    // 確立後は常に受理される（「未ロード状態」は無い）。即時応答が内容一致で返る。
    let content = HSTRING::from("hrequest");
    let outcome = session.get(&content).expect("確立後の get は受理されること");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(HSTRING::from(IMMEDIATE_RESPONSE)),
        "即時応答が内容一致で返ること"
    );
    assert!(!session.is_pending(), "即時応答は保留枠を立てないこと");

    // drop teardown: session を drop すると保留取消→brain 参照 drop が起こる。以降は参照不在
    // （型システムが検証を肩代わり——drop 後に session を使うコードはコンパイルできない）。
    drop(session);
}

/// `Deferred` 保留中に drop すると保留が取消され（保留枠クリア）、再 activate 後に新規 get が
/// 正常に受理されること（要件 2.1/12.3・議題3）。
#[test]
fn drop_cancels_pending_then_reactivate_serves_requests() {
    let factory: IShioriFactory = StatefulFactory { deferred: true }.into();

    // host を明示生成し、drop 後に突合枠クリアを観測できるよう clone で保持する。
    // sink は sylphya 委譲済み（第 2 ストア撤去・Task 9.1/9.3）。hermetic な偽 IO sink（既知 asker）。
    let host: IShioriHost = crate::shiori_host::spawn_test_sylphya_sink().sink.into();
    let brain = factory
        .create(&HSTRING::from("dir"), &HSTRING::from("name"), &host)
        .expect("create");
    let mut session = ShioriSession::from_parts(brain, host.clone());
    let sink_impl =
        unsafe { AsImpl::<crate::shiori_host::ShioriHostSink>::as_impl(&host) };

    // --- 遅延 get で保留枠を立てる（単一 in-flight・議題3）。
    let content = HSTRING::from("hrequest-deferred");
    let outcome = session.get(&content).expect("遅延 get は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Deferred(CorrelationToken(DEFERRED_TOKEN)),
        "遅延はトークン付き Deferred を返すこと"
    );
    assert!(session.is_pending(), "遅延後は保留状態（単一 in-flight）であること");
    assert_eq!(
        sink_impl.pending_token(),
        Some(CorrelationToken(DEFERRED_TOKEN)),
        "host 側突合枠にトークンがあること"
    );

    // --- 保留中に drop: 保留が取消され host 側突合枠がクリアされる（要件 2.1/12.3・議題3）。
    drop(session);
    assert_eq!(
        sink_impl.pending_token(),
        None,
        "drop で host 側突合枠が取消されること（保留取消・Drop teardown）"
    );

    // --- 再アクティベーション（新規 session）: 新しい host で確立し、新規 get が正常に受理される。
    let mut session =
        ShioriSession::activate(&factory, &HSTRING::from("dir"), &HSTRING::from("name"))
            .expect("再 activate");
    assert!(!session.is_pending(), "再確立直後は保留が持ち越されていないこと");

    let outcome = session
        .get(&content)
        .expect("再確立後の新規 get は受理されること");
    assert_eq!(
        outcome,
        SessionRequest::Deferred(CorrelationToken(DEFERRED_TOKEN)),
        "再確立後も遅延 get が正常に受理されること"
    );
    assert!(session.is_pending(), "再確立後の遅延 get で保留枠が立つこと（新規枠）");

    // teardown（drop で保留取消）。
    drop(session);
}

/// `Deferred` 保留中は次 `get` を出さない規律が、ライフサイクル経路（session 越し）でも成立すること
/// （要件 2.1・単一 in-flight・議題3）。
#[test]
fn deferred_blocks_next_get_under_lifecycle() {
    let factory: IShioriFactory = StatefulFactory { deferred: true }.into();
    let mut session =
        ShioriSession::activate(&factory, &HSTRING::from("dir"), &HSTRING::from("name"))
            .expect("activate");

    let content = HSTRING::from("hrequest");
    session.get(&content).expect("遅延 get は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // 単一 in-flight: 保留中の次 get は areka 側が RequestInFlight で拒否する。
    let err = session
        .get(&content)
        .expect_err("保留中の次 get は拒否されること");
    assert!(
        matches!(err, SessionError::RequestInFlight),
        "保留中は RequestInFlight で拒否されること, got {err:?}"
    );

    drop(session);
}
