//! 製品 [`ReferenceBrain`](crate::reference_brain) × [`ShioriSession`](crate::shiori_session)
//! の **end-to-end 結合テスト**（task 5.1・要件 6.4/6.5/6.6）。
//!
//! 既存の [`shiori_e2e_tests`](crate::shiori_e2e_tests) /
//! [`shiori_lifecycle_e2e_tests`](crate::shiori_lifecycle_e2e_tests) は **モック脳**
//! （`MockBrain`/`DeferringBrain`）を立てて `ShioriSession` の利用規律を検証していた。
//! 本モジュールはそれらと**重複させず**、`shiori_create` で取得した **本物の製品
//! `ReferenceBrain`** を `ShioriSession` 越しに駆動し、製品脳とセッションの統合が
//! デモ経路（即時→遅延+Complete→Raise→unload の数往復）で破綻なく成立することを
//! 決定的に（実時間 `sleep` に非依存・タイムアウトは `expire_if_elapsed` へ
//! [`Duration`] を注入）検証する。
//!
//! ## モック e2e との差分（重複回避）
//! - モック脳は `Request`/`Complete`/`Raise` の挙動を固定値で偽装していた。本テストは
//!   **製品 `Request` のエコー往復**（即時応答が受信 content と厳密一致）と
//!   **製品 `arm_defer_next`/`complete_pending`/`fire_raise`**（製品の相関トークン採番・
//!   保持 host への vtable 直呼び）という実装そのものを通す。
//! - したがって本テストは「製品脳とセッションの配線」が壊れていれば落ちる意味のある検証であり、
//!   モック e2e が既に押さえた sink 単体挙動・モック利用規律の再表明は避ける。
//!
//! ## 製品脳の取得とハンドル保持パターン（デモと同一）
//! [`ShioriSession::activate`] は `IShiori` を **move** で取り込み、脳アクセサを公開しない。
//! 遅延武装・能動通知（`arm_defer_next`/`complete_pending`/`fire_raise`）を駆動するため、
//! `activate` 前に脳を `clone`（AddRef）して型付きハンドルを確保し、`AsImpl` で
//! [`ReferenceBrain`] へダウンキャストしてから session を起こす。終了時は session を
//! `unload`→drop し、ハンドルを drop して `IShiori` を解放する（リーク/二重解放なし）。

use core::ffi::c_void;
use core::ptr;
use std::time::Duration;

use shiori_abi::error::SHIORI_E_UNKNOWN_TOKEN;
use shiori_abi::interface::IShiori;
use windows_core::{AsImpl, HSTRING, Interface};

use crate::reference_brain::{ReferenceBrain, shiori_create};
use crate::shiori_host::HostMessage;
use crate::shiori_session::{SessionError, SessionRequest, ShioriSession};

/// 不透明 OnBoot 様の固定 content（解析・スキーマ検証せず往復させるだけ・要件 1.4/8.1）。
const ONBOOT_CONTENT: &str = "\\0\\h\\s[0]OnBoot日本語opaque😶\\e";
/// 2 往復目で使う別の不透明 content（即時エコーの往復を区別して観測する）。
const SECOND_CONTENT: &str = "\\0\\h\\s[10]GetName別opaque🌱\\e";
/// 遅延完了で製品脳が後送りする応答本文（既知の不透明 HSTRING）。
const DEFERRED_RESPONSE: &str = "\\h\\s[0]deferred応答😶\\e";
/// 能動通知で製品脳が届けるさくらスクリプト様の本文（既知の不透明 HSTRING）。
const RAISE_SCRIPT: &str = "\\h\\s[0]wakeup-from-reference-brain";

/// 製品 `shiori_create` で `IShiori` を取得し、`(session, handle)` を構築するヘルパ。
///
/// `handle` は `activate` で move される `IShiori` とは別に AddRef 保持した型付き参照で、
/// テストから製品脳の `arm_defer_next`/`complete_pending`/`fire_raise` を駆動するために
/// `as_impl` でダウンキャストして使う。`timeout` は遅延完了タイムアウト（決定的判定用）。
fn make_session_with_handle(timeout: Duration) -> (ShioriSession, IShiori) {
    let mut out: *mut c_void = ptr::null_mut();
    // 製品の純粋C コンストラクタを直呼びして refcount 1 の IShiori を得る（in-tree シンボル直呼び）。
    let hr = unsafe { shiori_create(&mut out) };
    assert!(hr.is_ok(), "shiori_create は成功時 S_OK を返すこと, got 0x{:08X}", hr.0);
    assert!(!out.is_null(), "成功時は out へ非 NULL の IShiori を書き出すこと");

    // 手渡された単一参照を adopt（from_raw は AddRef しない）。
    let brain = unsafe { IShiori::from_raw(out) };
    // activate は brain を move するため、駆動用に AddRef した型付きハンドルを別途確保する。
    let handle = brain.clone();
    let session = ShioriSession::activate_with_timeout(brain, timeout)
        .expect("製品脳へのアクティベーションは成功すること");
    (session, handle)
}

/// 製品脳ハンドルから [`ReferenceBrain`] 実体参照を取り出す（製品脳駆動用）。
///
/// # Safety
/// `handle` は本テストで `shiori_create` が生成した `ReferenceBrain` 実体の COM ポインタ。
fn brain_of(handle: &IShiori) -> &ReferenceBrain {
    unsafe { AsImpl::<ReferenceBrain>::as_impl(handle) }
}

/// (1) フル往復: 即時→遅延+Complete→Raise→unload を製品脳越しに数往復通す（要件 6.4/6.5）。
///
/// - 即時: 製品 `Request` が受信 content を厳密エコーで返すこと（不透明往復の実証）。
/// - 遅延: 製品 `arm_defer_next`→`request` で `Deferred(token)` を受け保留に入り、製品
///   `complete_pending` の発火→`poll_completions` で `Completed{token,response}` を drain し
///   保留が解除されること（受け皿 drain＋単一 in-flight 解放）。
/// - 能動通知: 製品 `fire_raise` の発火→`poll_completions` で `Raised(script)` を drain すること。
/// - unload: 後始末が Ok であること。
/// - 反復: 遅延解消後に再び即時 request が通り（別 content をエコー往復）、製品脳とセッションの
///   配線が反復しても破綻しないこと。
#[test]
fn full_roundtrip_immediate_deferred_raise_unload_through_session() {
    let (mut session, handle) = make_session_with_handle(ShioriSession::DEFAULT_TIMEOUT);
    let brain = brain_of(&handle);

    // --- 往復1: 即時応答（製品 Request のエコー往復・不透明 content 厳密一致）。
    let onboot = HSTRING::from(ONBOOT_CONTENT);
    let outcome = session.request(&onboot).expect("即時 request は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(onboot.clone()),
        "製品脳の即時応答は受信 content の不解釈エコー（厳密一致）であること（要件 6.4）"
    );
    assert!(!session.is_pending(), "即時応答は保留枠を立てないこと");

    // --- 往復2: 遅延応答（製品トークン採番＋保持 host への Complete vtable 直呼び）。
    brain.arm_defer_next();
    let outcome = session.request(&onboot).expect("遅延 request は Ok");
    let token = match outcome {
        SessionRequest::Deferred(t) => t,
        other => panic!("遅延武装後の request は Deferred を返すこと, got {other:?}"),
    };
    assert!(session.is_pending(), "遅延後は保留状態（単一 in-flight）であること");
    assert_eq!(
        session.pending_token(),
        Some(token),
        "セッション保留トークンは製品脳が採番したトークンと一致すること"
    );

    // 製品脳が保持 host へ Complete(token, response) を vtable 直呼びで発火する。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    let hr = brain.complete_pending(&response);
    assert!(hr.is_ok(), "製品 complete_pending は host から S_OK を受け取ること, got 0x{:08X}", hr.0);

    // poll で Completed を drain し、保留が解除されること（受け皿 drain＋解放・要件 6.4）。
    let drained = session.poll_completions();
    assert_eq!(
        drained,
        vec![HostMessage::Completed {
            token,
            response: response.clone(),
        }],
        "遅延 Completed が製品トークン突合・内容一致で drain されること（要件 6.4）"
    );
    assert!(!session.is_pending(), "Complete 受領（poll）で保留が解除されること（単一 in-flight 解放）");

    // --- 往復3: 能動通知（製品 fire_raise → host へ Raise vtable 直呼び）。
    let script = HSTRING::from(RAISE_SCRIPT);
    let hr = brain.fire_raise(&script);
    assert!(hr.is_ok(), "製品 fire_raise は host から S_OK を受け取ること, got 0x{:08X}", hr.0);
    let drained = session.poll_completions();
    assert_eq!(
        drained,
        vec![HostMessage::Raised(script)],
        "能動通知 Raise が製品脳→host で drain されること（要件 6.4）"
    );

    // --- 往復4: 反復（遅延解消後の即時 request が別 content をエコー往復で通る）。
    let second = HSTRING::from(SECOND_CONTENT);
    let outcome = session.request(&second).expect("反復の即時 request は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(second),
        "反復後も製品脳の即時応答が別 content を厳密エコーすること（配線の反復健全性）"
    );

    // --- 後始末: unload が Ok であること。
    session.unload().expect("unload は成功すること（要件 6.5）");

    drop(session);
    drop(handle);
}

/// (2) 単一 in-flight: 遅延保留中の次 request が `RequestInFlight` で拒否され、解消後は通ること（要件 6.5）。
///
/// 製品脳の遅延武装で保留に入れ、保留中の `request` が拒否されること（`matches!`）を確認。
/// 製品 `complete_pending`＋`poll` で解消したのち、次 request が成立することまで通す。
#[test]
fn single_in_flight_rejects_second_request_until_cleared() {
    let (mut session, handle) = make_session_with_handle(ShioriSession::DEFAULT_TIMEOUT);
    let brain = brain_of(&handle);

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let outcome = session.request(&onboot).expect("遅延 request は Ok");
    let token = match outcome {
        SessionRequest::Deferred(t) => t,
        other => panic!("遅延武装後の request は Deferred を返すこと, got {other:?}"),
    };
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // 単一 in-flight: 保留中の次 request は RequestInFlight で拒否される（要件 6.5）。
    let err = session
        .request(&onboot)
        .expect_err("保留中の次 request は拒否されること");
    assert!(
        matches!(err, SessionError::RequestInFlight),
        "拒否理由は RequestInFlight であること, got {err:?}"
    );
    assert!(session.is_pending(), "拒否されても保留は継続すること");

    // 解消: 製品 complete_pending＋poll で保留を解く。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    assert!(brain.complete_pending(&response).is_ok(), "complete_pending は S_OK");
    let drained = session.poll_completions();
    assert_eq!(
        drained,
        vec![HostMessage::Completed { token, response }],
        "Completed が drain されること"
    );
    assert!(!session.is_pending(), "解消後は保留が解除されること");

    // 解消後は次 request が成立すること（即時エコー）。
    let outcome = session.request(&onboot).expect("解消後は次 request 可能");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(onboot),
        "解消後の即時 request が成立すること（単一 in-flight 解放）"
    );

    session.unload().expect("unload は成功すること");
    drop(session);
    drop(handle);
}

/// (3) 決定的タイムアウト: 注入経過時間で保留枠を放棄でき、未満では継続すること（要件 6.5・実時間非依存）。
///
/// 100ms タイムアウトで activate し、製品脳の遅延で保留に入れる。99ms 経過注入では放棄せず保留継続、
/// 100ms 経過注入で放棄して保留解除。実時間 `sleep` は一切使わず `expire_if_elapsed` へ Duration を注入する。
#[test]
fn deterministic_timeout_expires_pending_via_injected_duration() {
    let timeout = Duration::from_millis(100);
    let (mut session, handle) = make_session_with_handle(timeout);
    let brain = brain_of(&handle);

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let _ = session.request(&onboot).expect("遅延 request は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // タイムアウト未満の経過注入では放棄しない（決定的・実時間非依存）。
    assert!(
        !session.expire_if_elapsed(Duration::from_millis(99)),
        "タイムアウト未満では保留を放棄しないこと"
    );
    assert!(session.is_pending(), "未超過では保留が継続すること");

    // タイムアウト到達の経過注入で保留枠を放棄する（要件 6.5）。
    assert!(
        session.expire_if_elapsed(Duration::from_millis(100)),
        "タイムアウト到達で保留枠を放棄すること"
    );
    assert!(!session.is_pending(), "放棄後は保留状態が解除されること");

    session.unload().expect("unload は成功すること");
    drop(session);
    drop(handle);
}

/// (4) タイムアウト後の stale Complete: 放棄後に遅れて来た製品 Complete が
/// `SHIORI_E_UNKNOWN_TOKEN` で弾かれること（要件 6.5/6.6）。
///
/// (3) と同じくタイムアウトで保留枠を放棄させたあと、製品脳に保持されたままの遅延完了を
/// `complete_pending` で発火する。セッションが host 側突合枠を放棄時にクリアしているため、
/// 遅れて来た Complete は突合不能で `SHIORI_E_UNKNOWN_TOKEN` を返す。
#[test]
fn stale_complete_after_timeout_is_rejected_by_host() {
    let timeout = Duration::from_millis(100);
    let (mut session, handle) = make_session_with_handle(timeout);
    let brain = brain_of(&handle);

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let _ = session.request(&onboot).expect("遅延 request は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // タイムアウトで保留枠を放棄（host 側突合枠もクリアされる）。
    assert!(
        session.expire_if_elapsed(timeout),
        "タイムアウト到達で保留枠を放棄すること"
    );
    assert!(!session.is_pending(), "放棄後は保留が解除されること");

    // 製品脳は遅延完了を保持したまま。放棄後に発火すると host 側突合枠が空のため弾かれる。
    let response = HSTRING::from(DEFERRED_RESPONSE);
    let hr = brain.complete_pending(&response);
    assert_eq!(
        hr, SHIORI_E_UNKNOWN_TOKEN,
        "タイムアウト放棄後の stale Complete は SHIORI_E_UNKNOWN_TOKEN で弾かれること（要件 6.5/6.6）, got 0x{:08X}",
        hr.0
    );
    // stale Complete は投函されないため poll は空。
    assert!(
        session.poll_completions().is_empty(),
        "弾かれた stale Complete はメールボックスへ投函されないこと"
    );

    session.unload().expect("unload は成功すること");
    drop(session);
    drop(handle);
}

/// (5) Unload 後始末（失敗注入アナログ）: 遅延保留中に unload すると保留が取消されること（要件 6.6）。
///
/// 製品脳で遅延保留状態に持ち込み、`unload` を呼ぶと成功し、保留が解除される（セッションが
/// areka 側・host 側双方の保留枠をクリアする）。これは遅延が outstanding でも session レベルの
/// cleanup が保留を確実に解放することを、本物の製品脳越しに実証する（要件 6.6 のクリーンアップ規律）。
#[test]
fn unload_cleans_up_outstanding_deferred_pending() {
    let (mut session, handle) = make_session_with_handle(ShioriSession::DEFAULT_TIMEOUT);
    let brain = brain_of(&handle);

    let onboot = HSTRING::from(ONBOOT_CONTENT);
    brain.arm_defer_next();
    let _ = session.request(&onboot).expect("遅延 request は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // 遅延が outstanding のまま unload しても成功し、保留が取消されること（後始末規律・要件 6.6）。
    session.unload().expect("遅延保留中の unload も成功すること");
    assert!(
        !session.is_pending(),
        "unload は outstanding な遅延保留を取消すこと（session レベルの cleanup）"
    );

    drop(session);
    drop(handle);
}
