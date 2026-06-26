//! ライフサイクルと単一 in-flight 規律の **end-to-end 結合テスト**
//! （task 5.3・要件 2.1/2.2/2.4）。
//!
//! task 4.2 の inline 単体テスト（`deferred_request_blocks_next_request`,
//! `unload_cancels_pending`, timeout 系）と task 5.2 の遅延 push e2e は、個々の挙動
//! （単一 in-flight 拒否・タイムアウト放棄・遅延完了配送）を切り出して検証している。
//! 本モジュールはそれらと**重複させず**、design.md
//! §System Flows →「ライフサイクル（load/unload と sink 受け渡し）」の
//! **状態遷移を通しで** 1〜数本のシナリオで実証する:
//!
//! 1. **未ロード状態（Load 前）の `Request` 拒否**: 脳がまだ `Load` されていない（`Unloaded`）
//!    状態で `request` すると `SHIORI_E_NOT_LOADED` →
//!    [`ShioriError::NotLoaded`](shiori_abi::error::ShioriError::NotLoaded) で拒否される（要件 2.4・
//!    state diagram `Unloaded --> Unloaded: Request 拒否`）。
//! 2. **`Load`→`Request`→`Unload` の完全な遷移**: アクティベーション（`Load`）で `Loaded` へ遷移し、
//!    `request` が受理されて即時応答が往復し、`Unload` で `Unloaded` へ戻る（要件 2.1/2.2、
//!    state diagram `Unloaded --> Loaded --> Unloaded`）。
//! 3. **未ロード状態（Unload 後）の `Request` 拒否**: `Unload` 後（再び `Unloaded`）に `request`
//!    すると再び `NotLoaded` で拒否される（要件 2.4・遷移が往復で成立すること）。
//! 4. **`Deferred` 保留中の `Unload` で保留取消 → 再 Load 後に正常動作**: 遅延 request で保留が
//!    立った状態で `unload` すると保留枠がクリアされ（要件 2.2・議題3）、再アクティベーション後に
//!    新規 request が正常に受理される（保留の持ち越しがないこと）。
//!
//! ## 状態を保持するモック脳（design.md §State Management）
//! design.md §State Management は「ライフサイクル状態（`Unloaded`/`Loaded`）は**脳実装側が保持**し、
//! ABI は状態フィールドを持たず未ロード拒否を HRESULT 契約として定める」と規定する。よって本テストの
//! モック脳 [`StatefulBrain`] は内部に [`AtomicBool`] のロード状態を持ち、未ロード時の `Request` を
//! `SHIORI_E_NOT_LOADED` で拒否する（4.2 の `MockBrain` は常にロード済み前提で未ロード経路を持たない
//! ため、ここで状態遷移を検証するには別モックが要る）。即時/遅延は構築時に切替可能とする。

#![allow(non_snake_case)]

use core::cell::RefCell;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use shiori_abi::error::{SHIORI_E_NOT_LOADED, SHIORI_S_PENDING, ShioriError};
use shiori_abi::ergonomic::ShioriExt;
use shiori_abi::interface::{IShiori, IShiori_Impl, IShioriHost};
use shiori_abi::outcome::CorrelationToken;
use windows_core::{AsImpl, HRESULT, HSTRING, Interface, implement};

use crate::shiori_session::{SessionError, SessionRequest, ShioriSession};

/// 遅延 request でステートフル脳が発行する固定相関トークン。
const DEFERRED_TOKEN: u64 = 0x00AA_00BB;
/// `Loaded` 状態で即時応答する固定本文。
const IMMEDIATE_RESPONSE: &str = "lifecycle-immediate-body";

/// ロード状態を内部に保持し、未ロード時の `Request` を `SHIORI_E_NOT_LOADED` で拒否するモック脳。
///
/// design.md §State Management の「状態は脳実装側が保持」に従い、`AtomicBool` で `Unloaded`/`Loaded`
/// を持つ。`Load` で `Loaded`、`Unload` で `Unloaded` へ遷移する。`Request` は:
/// - 未ロード（`Unloaded`）→ `SHIORI_E_NOT_LOADED`（要件 2.4）。
/// - ロード済み（`Loaded`）かつ即時モード → `S_OK`＋応答 HSTRING を move-out（要件 3.2）。
/// - ロード済みかつ遅延モード → `SHIORI_S_PENDING`＋token（要件 3.3）。
#[implement(IShiori)]
struct StatefulBrain {
    /// ロード状態（`true`=`Loaded`）。`Load`/`Unload` で遷移し、`Request` 受理可否を決める。
    loaded: AtomicBool,
    /// 遅延モードか（`true` で `Request` が `SHIORI_S_PENDING`＋token を返す）。
    deferred: bool,
    /// `Load` で受け取った host を保持する（議題2・AddRef 相当の COM 参照）。`Unload` で Release。
    held_host: RefCell<Option<IShioriHost>>,
}

impl StatefulBrain {
    fn new(deferred: bool) -> Self {
        Self {
            loaded: AtomicBool::new(false),
            deferred,
            held_host: RefCell::new(None),
        }
    }

    /// 現在ロード済みか（状態遷移の観測用）。
    fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::SeqCst)
    }
}

impl IShiori_Impl for StatefulBrain_Impl {
    unsafe fn Load(&self, host: *mut core::ffi::c_void) -> HRESULT {
        // 受け取った raw host を型付き COM 参照として AddRef 保持し、`Loaded` へ遷移する（議題2）。
        // Safety: host は areka が所有する有効な IShioriHost raw ポインタ（呼び出し中有効）。
        let borrowed: Option<&IShioriHost> = unsafe { IShioriHost::from_raw_borrowed(&host) };
        *self.held_host.borrow_mut() = borrowed.cloned();
        self.loaded.store(true, Ordering::SeqCst);
        HRESULT(0) // S_OK
    }

    unsafe fn Unload(&self) -> HRESULT {
        // 保持 host を Release（drop）し、`Unloaded` へ戻す（議題2・Unload 後は host を呼ばない）。
        *self.held_host.borrow_mut() = None;
        self.loaded.store(false, Ordering::SeqCst);
        HRESULT(0) // S_OK
    }

    unsafe fn Request(
        &self,
        _input: *const HSTRING,
        out_response: *mut HSTRING,
        out_token: *mut u64,
    ) -> HRESULT {
        if !self.loaded.load(Ordering::SeqCst) {
            // 未ロード状態（`Unloaded`）の Request は受理しない（要件 2.4・state diagram）。
            return SHIORI_E_NOT_LOADED;
        }
        if self.deferred {
            // 遅延: 相関トークンを out_token へ。out_response は未書き込み（空）。
            unsafe { ptr::write(out_token, DEFERRED_TOKEN) };
            SHIORI_S_PENDING
        } else {
            // 即時: callee 確保の HSTRING を out_response へ move-out（所有権規約・要件 3.2）。
            unsafe { ptr::write(out_response, HSTRING::from(IMMEDIATE_RESPONSE)) };
            HRESULT(0) // S_OK
        }
    }
}

/// モック脳の実体参照を取り出す（ロード状態の観測用）。
///
/// # Safety
/// `brain` は本テストで `StatefulBrain` から構築した COM ポインタであること。
fn brain_inner(brain: &IShiori) -> &StatefulBrain {
    unsafe { AsImpl::<StatefulBrain>::as_impl(brain) }
}

/// 未ロード状態（Load 前）の `request` が `NotLoaded` で拒否され、`Load`→`Request`→`Unload` の
/// 完全な遷移が成立し、`Unload` 後（再び未ロード）の `request` が再び `NotLoaded` で拒否されること。
///
/// state diagram（design.md §System Flows → ライフサイクル）の
/// `Unloaded --Request拒否--> Unloaded`, `Unloaded --Load--> Loaded`, `Loaded --Request--> Loaded`,
/// `Loaded --Unload--> Unloaded` を 1 シナリオで往復通しする（要件 2.1/2.2/2.4）。
#[test]
fn lifecycle_transition_and_unloaded_request_is_rejected() {
    let brain: IShiori = StatefulBrain::new(false).into();

    // --- (Unloaded) Load 前: 脳は未ロード。ShioriExt 越しの request は NotLoaded で拒否される（要件 2.4）。
    assert!(
        !brain_inner(&brain).is_loaded(),
        "アクティベーション前は Unloaded であること"
    );
    let content = HSTRING::from("hrequest-before-load");
    let err = brain
        .request(&content)
        .expect_err("未ロード（Load 前）の request は拒否されること");
    assert!(
        matches!(err, ShioriError::NotLoaded),
        "Load 前 request の拒否理由は NotLoaded であること, got {err:?}"
    );

    // --- (Unloaded --Load--> Loaded) アクティベーションで Load を呼び Loaded へ遷移する（要件 2.1）。
    let mut session =
        ShioriSession::activate(brain.clone()).expect("activate（Load）は成功すること");
    assert!(
        brain_inner(&brain).is_loaded(),
        "Load 後は Loaded へ遷移していること（要件 2.1）"
    );

    // --- (Loaded --Request--> Loaded) ロード済みでは request が受理され即時応答が往復する（要件 3.1/3.2）。
    let outcome = session
        .request(&content)
        .expect("Loaded 状態の request は受理されること");
    assert_eq!(
        outcome,
        SessionRequest::Immediate(HSTRING::from(IMMEDIATE_RESPONSE)),
        "Loaded 状態で即時応答が内容一致で返ること"
    );
    assert!(!session.is_pending(), "即時応答は保留枠を立てないこと");

    // --- (Loaded --Unload--> Unloaded) Unload で Unloaded へ戻す（要件 2.2）。
    session.unload().expect("unload は成功すること");
    assert!(
        !brain_inner(&brain).is_loaded(),
        "Unload 後は Unloaded へ戻っていること（要件 2.2）"
    );

    // --- (Unloaded) Unload 後: 再び未ロード。request は再び NotLoaded で拒否される（要件 2.4・往復成立）。
    let err = brain
        .request(&content)
        .expect_err("未ロード（Unload 後）の request は拒否されること");
    assert!(
        matches!(err, ShioriError::NotLoaded),
        "Unload 後 request の拒否理由は NotLoaded であること, got {err:?}"
    );
}

/// `Deferred` 保留中に `Unload` すると保留が取消され（保留枠クリア）、再アクティベーション
/// （再 Load）後に新規 request が正常に受理されること（要件 2.2/2.4・議題3）。
///
/// 4.2 の `unload_cancels_pending` は「Unload で保留が消える」点のみを見るが、本シナリオは
/// その後の **再 Load → 新規 request の正常受理**（保留の持ち越しがなく Loaded で再び動く）まで
/// 通しで実証する（ライフサイクル遷移の往復＋保留取消の結合）。
#[test]
fn unload_cancels_pending_then_reload_serves_requests() {
    let brain: IShiori = StatefulBrain::new(true).into();
    let mut session = ShioriSession::activate(brain.clone()).expect("activate（Load）");

    // --- 遅延 request で保留枠を立てる（単一 in-flight・議題3）。
    let content = HSTRING::from("hrequest-deferred");
    let outcome = session.request(&content).expect("遅延 request は Ok");
    assert_eq!(
        outcome,
        SessionRequest::Deferred(CorrelationToken(DEFERRED_TOKEN)),
        "遅延はトークン付き Deferred を返すこと"
    );
    assert!(session.is_pending(), "遅延後は保留状態（単一 in-flight）であること");

    // --- 保留中に Unload: 保留が取消され Unloaded へ戻る（要件 2.2・議題3）。
    session.unload().expect("保留中の unload は成功すること");
    assert!(!session.is_pending(), "Unload で保留が取消されること（要件 2.2）");
    assert!(
        !brain_inner(&brain).is_loaded(),
        "Unload 後は Unloaded であること"
    );

    // --- Unload 後（未ロード）の request は NotLoaded で拒否される（保留も Loaded 状態も持ち越さない）。
    let err = session
        .request(&content)
        .expect_err("Unload 後の request は拒否されること");
    assert!(
        matches!(err, SessionError::Shiori(ShioriError::NotLoaded)),
        "Unload 後 request の拒否理由は NotLoaded であること, got {err:?}"
    );

    // --- 再アクティベーション（再 Load）: Loaded へ復帰する。
    let mut session = ShioriSession::activate(brain.clone()).expect("再 activate（再 Load）");
    assert!(
        brain_inner(&brain).is_loaded(),
        "再 Load 後は Loaded へ復帰していること"
    );
    assert!(
        !session.is_pending(),
        "再 Load 直後は保留が持ち越されていないこと（保留取消が成立していた）"
    );

    // --- 新規 request が正常に受理されること（再び遅延枠を立てられる＝Loaded で正常動作）。
    let outcome = session
        .request(&content)
        .expect("再 Load 後の新規 request は受理されること");
    assert_eq!(
        outcome,
        SessionRequest::Deferred(CorrelationToken(DEFERRED_TOKEN)),
        "再 Load 後も遅延 request が正常に受理されること"
    );
    assert!(
        session.is_pending(),
        "再 Load 後の遅延 request で保留枠が立つこと（新規枠・持ち越しでない）"
    );

    // --- 後始末: 保留中の unload で取消（リーク防止・teardown）。
    session.unload().expect("teardown unload");
}

/// 念のための単独確認: `Deferred` 保留中は次 `Request` を出さない規律が、ライフサイクル経路
/// （`ShioriSession` 越し）でも成立すること（要件 2.4・単一 in-flight・議題3）。
///
/// 4.2 の `deferred_request_blocks_next_request` は同趣旨だが、本ケースは `StatefulBrain`
/// （状態保持・未ロード拒否を持つ脳）でも同じ規律が崩れないことを通しシナリオの一部として確認する
/// （脳が「未ロードなら拒否」を持つことで規律が二重化されないこと＝areka 側が先に弾くこと）。
#[test]
fn deferred_blocks_next_request_under_lifecycle() {
    let brain: IShiori = StatefulBrain::new(true).into();
    let mut session = ShioriSession::activate(brain).expect("activate");

    let content = HSTRING::from("hrequest");
    session.request(&content).expect("遅延 request は Ok");
    assert!(session.is_pending(), "遅延後は保留状態であること");

    // 単一 in-flight: 保留中の次 request は areka 側が RequestInFlight で拒否する（脳まで到達しない）。
    let err = session
        .request(&content)
        .expect_err("保留中の次 request は拒否されること");
    assert!(
        matches!(err, SessionError::RequestInFlight),
        "保留中は RequestInFlight で拒否されること（脳の状態判定に到達する前）, got {err:?}"
    );

    // teardown.
    session.unload().expect("teardown unload");
}
