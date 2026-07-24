//! 製品 [`ReferenceFactory`](crate::reference_brain::ReferenceFactory)/
//! [`ReferenceBrain`](crate::reference_brain::ReferenceBrain) × [`ShioriSession`](crate::shiori_session)
//! の **end-to-end 結合テスト**（新 ABI 追随・要件 1.3/12.1/12.6）。
//!
//! 既存の [`shiori_e2e_tests`](crate::shiori_e2e_tests) /
//! [`shiori_lifecycle_e2e_tests`](crate::shiori_lifecycle_e2e_tests) は **モック脳** を立てて
//! `ShioriSession` の利用規律を検証していた。本モジュールはそれらと**重複させず**、`shiori_factory`
//! で取得した **本物の製品 factory/brain** を `ShioriSession` 越しに駆動し、製品脳とセッションの統合が
//! デモ経路（即時→遅延+complete→raise→notify→drop teardown）で破綻なく成立することを決定的に
//! （実時間 `sleep` に非依存・タイムアウトは `expire_if_elapsed` へ [`Duration`] を注入）検証する。
//!
//! ## モック e2e との差分（重複回避）
//! - モック脳は `Get`/`Complete`/`Raise` の挙動を固定値で偽装していた。本テストは **製品 `Get` の
//!   エコー往復**（即時応答が受信 content と厳密一致）と **製品 `arm_defer_next`/`complete_pending`/
//!   `fire_raise`**（製品の相関トークン採番・保持 host への safe メソッド呼出）という実装そのものを通す。
//! - また **`load_dir`/`shiori_name` の貫通観測**（D1・要件 1.3）を製品 factory 経由で実証する。
//!
//! ## 製品脳の取得とハンドル保持パターン
//! [`shiori_factory`] で `IShioriFactory` を得て、host（sink）を明示生成し、`factory.create` で
//! 製品脳を生成する。session は脳を move で保持するため、遅延武装・能動通知（`arm_defer_next`/
//! `complete_pending`/`fire_raise`）を駆動するハンドルを `clone`（AddRef）で確保し、`AsImpl` で
//! [`ReferenceBrain`] へダウンキャストする。終了時は session を drop（保留取消→brain drop）する。

use core::ffi::c_void;
use core::ptr;
use std::time::Duration;

use shiori_abi::error::ShioriError;
use shiori_abi::interface::{IShiori, IShioriFactory, IShioriHost};
use windows_core::{AsImpl, HSTRING, Interface};

use crate::reference_brain::{ReferenceBrain, shiori_factory};
use crate::shiori_host::{HostMessage, ShioriHostSink};
use crate::shiori_session::{SessionError, SessionRequest, ShioriSession};

/// 不透明 OnBoot 様の固定 content（解析・スキーマ検証せず往復させるだけ・不透明）。
const ONBOOT_CONTENT: &str = "\\0\\h\\s[0]OnBoot日本語opaque😶\\e";
/// 2 往復目で使う別の不透明 content（即時エコーの往復を区別して観測する）。
const SECOND_CONTENT: &str = "\\0\\h\\s[10]GetName別opaque🌱\\e";
/// 遅延完了で製品脳が後送りする応答本文（既知の不透明 HSTRING）。
const DEFERRED_RESPONSE: &str = "\\h\\s[0]deferred応答😶\\e";
/// 能動通知で製品脳が届けるさくらスクリプト様の本文（既知の不透明 HSTRING）。
const RAISE_SCRIPT: &str = "\\h\\s[0]wakeup-from-reference-brain";
/// D1 貫通観測に用いる load_dir。
const LOAD_DIR: &str = "C:/ghost/master";
/// D1 貫通観測に用いる shiori_name。
const SHIORI_NAME: &str = "reference";

/// 製品 `shiori_factory`→`create` で製品脳を得て、`(session, handle, host)` を構築するヘルパ。
///
/// `handle` は session が move する `IShiori` とは別に AddRef 保持した型付き参照で、テストから製品脳の
/// `arm_defer_next`/`complete_pending`/`fire_raise`/`notifications`/`load_dir` を駆動・観測するために
/// `as_impl` でダウンキャストして使う。`timeout` は遅延完了タイムアウト（決定的判定用）。
/// `host` は脳と session が共有する sink（脳→host の配送を session が観測するため明示保持）。
fn make_session_with_handle(timeout: Duration) -> (ShioriSession, IShiori, IShioriHost) {
    let mut out: *mut c_void = ptr::null_mut();
    // 製品の純粋C コンストラクタを直呼びして refcount 1 の IShioriFactory を得る（in-tree シンボル直呼び）。
    let hr = unsafe { shiori_factory(&mut out) };
    assert!(hr.is_ok(), "shiori_factory は成功時 S_OK を返すこと, got 0x{:08X}", hr.0);
    assert!(!out.is_null(), "成功時は out へ非 NULL の IShioriFactory を書き出すこと");
    let factory = unsafe { IShioriFactory::from_raw(out) };

    // host（sink）を明示生成し、脳と session で共有する。
    // sink は sylphya 委譲済み（第 2 ストア撤去・Task 9.1/9.3）。hermetic な偽 IO sink（既知 asker）。
    let host: IShioriHost = crate::shiori_host::spawn_test_sylphya_sink().sink.into();
    let brain = factory
        .create(&HSTRING::from(LOAD_DIR), &HSTRING::from(SHIORI_NAME), &host)
        .expect("製品脳の create は成功すること");
    // session は brain を move するため、駆動用に AddRef した型付きハンドルを別途確保する。
    let handle = brain.clone();
    let session = ShioriSession::from_parts(brain, host.clone()).with_timeout(timeout);
    (session, handle, host)
}

/// 製品脳ハンドルから [`ReferenceBrain`] 実体参照を取り出す（製品脳駆動用）。
///
/// # Safety
/// `handle` は本テストで `shiori_factory`→`create` が生成した `ReferenceBrain` 実体の COM ポインタ。
fn brain_of(handle: &IShiori) -> &ReferenceBrain {
    unsafe { AsImpl::<ReferenceBrain>::as_impl(handle) }
}

/// (0) `load_dir`/`shiori_name` の貫通観測（D1・要件 1.3）: 製品 factory 経由で create した脳が
/// load_dir/shiori_name を保持し観測可能であること。
#[test]
fn create_propagates_and_observes_load_dir_shiori_name() {
    let (session, handle, host) = make_session_with_handle(ShioriSession::DEFAULT_TIMEOUT);
    let brain = brain_of(&handle);
    assert_eq!(
        brain.load_dir(),
        &HSTRING::from(LOAD_DIR),
        "load_dir が製品脳へ貫通し観測可能であること（D1・要件 1.3）"
    );
    assert_eq!(
        brain.shiori_name(),
        &HSTRING::from(SHIORI_NAME),
        "shiori_name が製品脳へ貫通し観測可能であること（要件 1.3）"
    );
    drop(session);
    drop(handle);
    drop(host);
}

/// (1) フル往復: 即時→遅延+complete→raise→notify→drop teardown を製品脳越しに数往復通す（要件 12.1）。
#[test]
fn full_roundtrip_immediate_deferred_raise_notify_teardown_through_session() {
    let (mut session, handle, host) = make_session_with_handle(ShioriSession::DEFAULT_TIMEOUT);
    let brain = brain_of(&handle);

    // --- 往復1: 即時応答（製品 Get のエコー往復・不透明 content 厳密一致）。
    let onboot = HSTRING::from(ONBOOT_CONTENT);
    let outcome = session.get(&onboot).expect("即時 get は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(onboot.clone()),
        "製品脳の即時応答は受信 content の不解釈エコー（厳密一致）であること（要件 12.1）"
    );
    assert!(!session.is_pending(), "即時応答は保留枠を立てないこと");

    // --- 往復2: 遅延応答（製品トークン採番＋保持 host への safe complete）。
    brain.arm_defer_next();
    let outcome = session.get(&onboot).expect("遅延 get は Ok");
    let token = match outcome {
        SessionRequest::Deferred(t) => t,
        other => panic!("遅延武装後の get は Deferred を返すこと, got {other:?}"),
    };
    assert!(session.is_pending(), "遅延後は保留状態（単一 in-flight）であること");
    assert_eq!(
        session.pending_token(),
        Some(token),
        "セッション保留トークンは製品脳が採番したトークンと一致すること"
    );

    // 製品脳が保持 host へ safe complete(token, response) を発火する。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    brain
        .complete_pending(&response)
        .expect("製品 complete_pending は host から Ok を受け取ること");

    // poll で Completed を drain し、保留が解除されること（受け皿 drain＋解放・要件 12.1）。
    let drained = session.poll_completions();
    assert_eq!(
        drained,
        vec![HostMessage::Completed {
            token,
            response: response.clone(),
        }],
        "遅延 Completed が製品トークン突合・内容一致で drain されること（要件 12.1）"
    );
    assert!(!session.is_pending(), "Complete 受領（poll）で保留が解除されること（単一 in-flight 解放）");

    // --- 往復3: 能動通知（製品 fire_raise → host へ safe raise）。
    let script = HSTRING::from(RAISE_SCRIPT);
    brain.fire_raise(&script).expect("製品 fire_raise は host から Ok を受け取ること");
    let drained = session.poll_completions();
    assert_eq!(
        drained,
        vec![HostMessage::Raised(script)],
        "能動通知 raise が製品脳→host で drain されること（要件 12.1）"
    );

    // --- 往復4: 片道通知（製品 notify → 受領ログへ記録・応答なし・要件 9.3 追随）。
    let notify_content = HSTRING::from("NOTIFY SHIORI/3.0 OnOtherGhostBooted");
    session.notify(&notify_content).expect("notify は Ok（片道）");
    assert_eq!(
        brain.notifications(),
        vec![notify_content],
        "製品脳の Notify 受領ログに記録されること（片道性の観測）"
    );

    // --- 往復5: 反復（遅延解消後の即時 get が別 content をエコー往復で通る）。
    let second = HSTRING::from(SECOND_CONTENT);
    let outcome = session.get(&second).expect("反復の即時 get は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(second),
        "反復後も製品脳の即時応答が別 content を厳密エコーすること（配線の反復健全性）"
    );

    // --- 後始末: drop teardown（保留取消→brain drop）。
    drop(session);
    drop(handle);
    drop(host);
}

/// (2) 単一 in-flight: 遅延保留中の次 get が `RequestInFlight` で拒否され、解消後は通ること（要件 12.1）。
#[test]
fn single_in_flight_rejects_second_get_until_cleared() {
    let (mut session, handle, host) = make_session_with_handle(ShioriSession::DEFAULT_TIMEOUT);
    let brain = brain_of(&handle);

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let outcome = session.get(&onboot).expect("遅延 get は Ok");
    let token = match outcome {
        SessionRequest::Deferred(t) => t,
        other => panic!("遅延武装後の get は Deferred を返すこと, got {other:?}"),
    };
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // 単一 in-flight: 保留中の次 get は RequestInFlight で拒否される（要件 12.1）。
    let err = session
        .get(&onboot)
        .expect_err("保留中の次 get は拒否されること");
    assert!(
        matches!(err, SessionError::RequestInFlight),
        "拒否理由は RequestInFlight であること, got {err:?}"
    );
    assert!(session.is_pending(), "拒否されても保留は継続すること");

    // 解消: 製品 complete_pending＋poll で保留を解く。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    brain.complete_pending(&response).expect("complete_pending は Ok");
    let drained = session.poll_completions();
    assert_eq!(
        drained,
        vec![HostMessage::Completed { token, response }],
        "Completed が drain されること"
    );
    assert!(!session.is_pending(), "解消後は保留が解除されること");

    // 解消後は次 get が成立すること（即時エコー）。
    let outcome = session.get(&onboot).expect("解消後は次 get 可能");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(onboot),
        "解消後の即時 get が成立すること（単一 in-flight 解放）"
    );

    drop(session);
    drop(handle);
    drop(host);
}

/// (3) 決定的タイムアウト: 注入経過時間で保留枠を放棄でき、未満では継続すること（要件 12.1・実時間非依存）。
#[test]
fn deterministic_timeout_expires_pending_via_injected_duration() {
    let timeout = Duration::from_millis(100);
    let (mut session, handle, host) = make_session_with_handle(timeout);
    let brain = brain_of(&handle);

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let _ = session.get(&onboot).expect("遅延 get は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");

    assert!(
        !session.expire_if_elapsed(Duration::from_millis(99)),
        "タイムアウト未満では保留を放棄しないこと"
    );
    assert!(session.is_pending(), "未超過では保留が継続すること");

    assert!(
        session.expire_if_elapsed(Duration::from_millis(100)),
        "タイムアウト到達で保留枠を放棄すること"
    );
    assert!(!session.is_pending(), "放棄後は保留状態が解除されること");

    drop(session);
    drop(handle);
    drop(host);
}

/// (4) タイムアウト後の stale complete: 放棄後に遅れて来た製品 complete が `UnknownToken` で
/// 弾かれること（要件 12.1）。
#[test]
fn stale_complete_after_timeout_is_rejected_by_host() {
    let timeout = Duration::from_millis(100);
    let (mut session, handle, host) = make_session_with_handle(timeout);
    let brain = brain_of(&handle);

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let _ = session.get(&onboot).expect("遅延 get は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // タイムアウトで保留枠を放棄（host 側突合枠もクリアされる）。
    assert!(
        session.expire_if_elapsed(timeout),
        "タイムアウト到達で保留枠を放棄すること"
    );
    assert!(!session.is_pending(), "放棄後は保留が解除されること");

    // 製品脳は遅延完了を保持したまま。放棄後に発火すると host 側突合枠が空のため弾かれる。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    let err = brain
        .complete_pending(&response)
        .expect_err("タイムアウト放棄後の stale complete は Err");
    assert!(
        matches!(err, ShioriError::UnknownToken),
        "タイムアウト放棄後の stale complete は UnknownToken で弾かれること, got {err:?}"
    );
    // stale complete は投函されないため poll は空。
    assert!(
        session.poll_completions().is_empty(),
        "弾かれた stale complete はメールボックスへ投函されないこと"
    );

    drop(session);
    drop(handle);
    drop(host);
}

/// (5) drop teardown（outstanding 遅延の取消）: 遅延保留中に drop すると保留が取消されること（要件 2.1/12.3）。
#[test]
fn drop_cleans_up_outstanding_deferred_pending() {
    let (mut session, handle, host) = make_session_with_handle(ShioriSession::DEFAULT_TIMEOUT);
    let brain = brain_of(&handle);

    // sink を clone で保持し、drop 後に突合枠クリアを観測できるようにする。
    let sink_impl = unsafe { AsImpl::<ShioriHostSink>::as_impl(&host) };

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let _ = session.get(&onboot).expect("遅延 get は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");
    assert!(sink_impl.pending_token().is_some(), "drop 前は host 側突合枠にトークンがあること");

    // 遅延が outstanding のまま drop すると保留が取消される（Drop teardown・要件 2.1/12.3）。
    drop(session);
    assert_eq!(
        sink_impl.pending_token(),
        None,
        "drop は outstanding な遅延保留を取消すこと（host 側突合枠クリア）"
    );

    drop(handle);
    drop(host);
}
