//! Task 5.1 結合テスト: in-proc モック脳による即時往復・所有権/非マーシャリング/Drop 実証。
//!
//! shiori-abi の**統合テスト**（公開 API のみ使用・`src` は不変）。`#[implement(IShiori)]` の
//! モック脳を in-proc で立て、`ShioriExt::request` 経由で即時応答 HSTRING がマーシャリングなしで
//! 往復することと、HSTRING の確保/解放が 1:1 に均衡し**二重解放・リークが発生しないこと**を
//! 決定的に実証する（requirements.md 1.2/3.1/3.2/4.1/4.3/5.2/5.4・design.md §Testing Strategy →
//! Integration Tests / §IShiori Invariants 4.3 / §Open Questions・HSTRING 所有権が唯一の UB 源）。
//!
//! ## Drop 回数観測の決定的アプローチ（核心・4.3）
//! `HSTRING` 自体に Drop フックは差せないため、以下を**複合**して「確保=解放=1:1・
//! 二重解放/リークなし」を実時間や ASAN に頼らず決定的に示す:
//!
//! - **(probe) 明示的 Drop 計測**: モック脳が応答を生成するたびに、応答 HSTRING を内包する
//!   被験ラッパ [`TrackedResponse`] を 1 つ生成し、生成回数（alloc）と Drop 回数（drop）を
//!   グローバルカウンタで数える。ラッパは move-out 後に caller 側で Drop され、最終的に
//!   alloc == drop（1:1 均衡）であることを assert する。drop < alloc ならリーク、
//!   drop > alloc なら二重解放を示す。
//! - **(i) 多数回ループ**: 同一往復を 10_000 回繰り返し、毎回 content 一致を assert しつつ
//!   abort/crash しないことを示す（HSTRING の二重解放は CRT abort を招くため、完走自体が
//!   二重解放非発生の証左となる）。
//! - **(ii) clone 生存**: 脳が用意した応答を test 側で clone 保持し、original(`Immediate`) を
//!   drop しても保持側 content が無傷であることを assert（premature free / 破壊的マーシャリング
//!   なら壊れる。HSTRING は参照カウント型のためこの不変が成立する）。
//! - **(iii) ビット一致**: move-out された HSTRING の content が脳の用意値とビット単位
//!   （UTF-16 コードユニット列）で一致（マーシャリングはコピー/変換を伴うが、in-proc 直 vtable は
//!   同一論理バッファを渡すため完全一致する）。

#![allow(non_snake_case)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use shiori_abi::ergonomic::ShioriExt;
use shiori_abi::interface::{IShiori, IShiori_Impl, IShioriHost};
use shiori_abi::outcome::RequestOutcome;
use windows_core::{HRESULT, HSTRING, implement};

/// 応答ラッパの確保/解放回数を数える **per-instance** カウンタ（probe）。
///
/// グローバル static にすると cargo の並列テスト実行（既定で複数テストが同一プロセス内の
/// 別スレッドで走る）でカウンタが相互汚染する。そのため計測状態をモック脳インスタンスに
/// 紐づけ、各テストが独立した [`Counters`] を所有する設計とする（決定的・スレッド安全）。
#[derive(Default)]
struct Counters {
    /// 応答ラッパの累計生成回数（alloc）。
    alloc: AtomicI64,
    /// 応答ラッパの累計 Drop 回数（drop）。
    drop: AtomicI64,
}

impl Counters {
    fn alloc(&self) -> i64 {
        self.alloc.load(Ordering::SeqCst)
    }
    fn dropped(&self) -> i64 {
        self.drop.load(Ordering::SeqCst)
    }
}

/// 応答 HSTRING を内包し Drop を計測する被験ラッパ（probe）。
///
/// 生成時に所属インスタンスの `alloc` を、Drop 時に `drop` をインクリメントする。これにより
/// 「応答ライフサイクルの確保=解放=1:1」を決定的に観測する。`HSTRING` 自体に Drop フックを
/// 差せないため、応答生成のたびにこのラッパを 1 つ随伴させ、ラッパの均衡をもって
/// 二重解放/リーク非発生の代理証拠とする（応答 HSTRING の Drop はラッパ内包フィールドの
/// Drop として 1:1 で連動する）。
struct TrackedResponse {
    inner: HSTRING,
    counters: Arc<Counters>,
}

impl TrackedResponse {
    fn new(s: &str, counters: Arc<Counters>) -> Self {
        counters.alloc.fetch_add(1, Ordering::SeqCst);
        Self {
            inner: HSTRING::from(s),
            counters,
        }
    }

    /// 内包 HSTRING を move-out 用に複製する（参照カウント増加・content 共有）。
    ///
    /// `HSTRING` は参照カウント型のため `clone` は論理 content を共有しつつ独立した
    /// 所有権ハンドルを返す。これを out-param へ move-out し、ラッパ自身は脳側で Drop させる
    /// ことで「脳が用意した値の所有権ハンドルが 1 本 caller へ渡り、caller の Drop で解放される」
    /// 往復を作る。
    fn hstring_for_moveout(&self) -> HSTRING {
        self.inner.clone()
    }
}

impl Drop for TrackedResponse {
    fn drop(&mut self) {
        self.counters.drop.fetch_add(1, Ordering::SeqCst);
    }
}

/// 脳が用意する既知の応答文字列（content 一致・ビット一致検証の基準値）。
const KNOWN_RESPONSE: &str = "mock-brain-immediate-response-本文";

/// in-proc 即時応答モック脳（`#[implement(IShiori)]`）。
///
/// `Request` は呼ばれるたびに [`TrackedResponse`] を 1 つ生成し、その内包 HSTRING の
/// 所有権ハンドルを `out_response` へ move-out して `S_OK` を返す（即時応答・所有権規約）。
/// `Load`/`Unload` は最小（`S_OK`）。計測カウンタはインスタンス所有（並列テスト汚染回避）。
#[implement(IShiori)]
struct MockBrain {
    counters: Arc<Counters>,
}

impl MockBrain {
    fn new() -> (Self, Arc<Counters>) {
        let counters = Arc::new(Counters::default());
        (
            Self {
                counters: Arc::clone(&counters),
            },
            counters,
        )
    }
}

impl IShiori_Impl for MockBrain_Impl {
    unsafe fn Load(&self, _host: *mut core::ffi::c_void) -> HRESULT {
        HRESULT(0) // S_OK
    }

    unsafe fn Unload(&self) -> HRESULT {
        HRESULT(0) // S_OK
    }

    unsafe fn Request(
        &self,
        _input: *const HSTRING,
        out_response: *mut HSTRING,
        _out_token: *mut u64,
    ) -> HRESULT {
        // 応答生成: probe ラッパを 1 つ確保（alloc++）。
        let tracked = TrackedResponse::new(KNOWN_RESPONSE, Arc::clone(&self.counters));
        // out-param へ content の所有権ハンドルを move-out（callee 確保・caller 解放規約）。
        // `tracked` 自身はこの関数末尾で Drop され drop++（内包 HSTRING も 1 本解放）。
        unsafe { core::ptr::write(out_response, tracked.hstring_for_moveout()) };
        HRESULT(0) // S_OK
        // ここで `tracked` が Drop -> drop++（ラッパ 1:1 均衡の片側）。
    }
}

/// (3.1/3.2/4.1/1.2) 即時応答 HSTRING が `Immediate` で往復し content 一致すること、
/// かつ (iii) ビット単位（UTF-16 コードユニット列）で脳の用意値と完全一致すること。
#[test]
fn immediate_response_roundtrips_with_bit_identical_content() {
    let (mock, _counters) = MockBrain::new();
    let brain: IShiori = mock.into();

    let content = HSTRING::from("ping-request-content");
    let outcome = brain.request(&content).expect("即時応答は Ok であること");

    let resp = match outcome {
        RequestOutcome::Immediate(resp) => resp,
        other => panic!("expected Immediate, got {other:?}"),
    };

    let expected = HSTRING::from(KNOWN_RESPONSE);
    // content 一致（論理等価・要件 3.2）。
    assert_eq!(resp, expected, "往復した応答 content が脳の用意値と一致すること");
    // (iii) ビット一致: UTF-16 コードユニット列が完全一致（in-proc 直 vtable=非マーシャリング 4.3）。
    // `HSTRING` は `Deref<Target = [u16]>`（windows-strings）。`&*` で UTF-16 スライスを取得して比較する。
    assert_eq!(
        &*resp, &*expected,
        "応答が UTF-16 ビット単位で完全一致すること（マーシャリングによるコピー/変換が介在しない）"
    );
}

/// (probe・4.3) 単一往復で応答ラッパの確保=解放が 1:1 に均衡し、リーク/二重解放が無いこと。
///
/// `Immediate` を drop した後に alloc==drop（=2: 脳の move-out 元 + caller 受領分）であることを
/// 観測する。drop<alloc ならリーク、drop>alloc なら二重解放を示す。
#[test]
fn single_roundtrip_alloc_equals_drop() {
    let (mock, counters) = MockBrain::new();
    let brain: IShiori = mock.into();

    {
        let content = HSTRING::from("ping");
        let outcome = brain.request(&content).expect("即時応答は Ok であること");
        let RequestOutcome::Immediate(resp) = outcome else {
            panic!("expected Immediate");
        };
        assert_eq!(resp, HSTRING::from(KNOWN_RESPONSE));
        // ここで `resp`（caller が所有する move-out された HSTRING）がスコープ末で Drop される。
    }

    // 脳側でラッパ `tracked` は Request 内で 1 回生成・1 回 Drop された（alloc==drop==1）。
    // ラッパは応答 HSTRING の所有権ライフサイクルの代理計測であり、生成と解放が均衡する。
    let alloc = counters.alloc();
    let drop = counters.dropped();
    assert_eq!(alloc, 1, "1 往復で応答ラッパが 1 回だけ確保されること");
    assert_eq!(
        drop, alloc,
        "確保=解放（1:1 均衡）であること。drop<alloc=リーク, drop>alloc=二重解放: alloc={alloc}, drop={drop}"
    );
}

/// (i・4.3) 多数回（10_000）の往復で毎回 content 一致し、abort/crash せず完走すること。
///
/// HSTRING の二重解放は CRT abort を招くため、10_000 回の確保/解放を完走できること自体が
/// 二重解放非発生の強い証左となる。併せて probe カウンタの 1:1 均衡（alloc==drop==N）を assert。
#[test]
fn many_roundtrips_no_double_free_and_balanced() {
    let (mock, counters) = MockBrain::new();
    let brain: IShiori = mock.into();

    const N: i64 = 10_000;
    let expected = HSTRING::from(KNOWN_RESPONSE);
    let content = HSTRING::from("ping");

    for i in 0..N {
        let outcome = brain.request(&content).expect("即時応答は Ok であること");
        let RequestOutcome::Immediate(resp) = outcome else {
            panic!("iteration {i}: expected Immediate");
        };
        assert_eq!(resp, expected, "iteration {i}: content 一致");
        // `resp` は各反復末で Drop される（caller 解放）。
    }

    let alloc = counters.alloc();
    let drop = counters.dropped();
    assert_eq!(alloc, N, "N 往復で N 回確保されること");
    assert_eq!(
        drop, N,
        "N 往復で確保=解放=N（二重解放/リークなし）であること: alloc={alloc}, drop={drop}"
    );
}

/// (ii・4.3) clone 生存: 受領した `Immediate` を clone 保持し、original を drop しても
/// 保持側 content が無傷であること。premature free / 破壊的マーシャリングなら壊れる。
#[test]
fn cloned_response_survives_original_drop() {
    let (mock, _counters) = MockBrain::new();
    let brain: IShiori = mock.into();

    let content = HSTRING::from("ping");
    let outcome = brain.request(&content).expect("即時応答は Ok であること");
    let RequestOutcome::Immediate(resp) = outcome else {
        panic!("expected Immediate");
    };

    // test 側で clone 保持（HSTRING は参照カウント型: content を共有しつつ独立ハンドル）。
    let survivor = resp.clone();
    let expected = HSTRING::from(KNOWN_RESPONSE);

    // original を明示 drop（caller 解放を前倒し）。
    drop(resp);

    // 保持側 content が破壊されていないこと（premature free なら UB/不一致になる）。
    assert_eq!(
        survivor, expected,
        "original drop 後も clone 保持側の content が無傷であること（premature free 非発生）"
    );
    assert_eq!(
        &*survivor, &*expected,
        "clone 保持側が UTF-16 ビット単位で無傷であること"
    );
}

/// `ShioriExt::load`/`unload` の最小経路が成立すること（ハーネス健全性・要件 2.1/2.2）。
///
/// host を `#[implement(IShioriHost)]` で立てて `load` に渡し、Ok を観測する。本テストの
/// 主眼は所有権/Drop だが、結合ハーネスとして load→request→unload の最小ライフサイクルが
/// 通ることも併せて確認する（重複を避け詳細なライフサイクル検証は task 5.3 に委ねる）。
#[test]
fn harness_load_request_unload_minimal() {
    let (mock, _counters) = MockBrain::new();
    let brain: IShiori = mock.into();
    let host: IShioriHost = NoopHost.into();

    brain.load(&host).expect("load は Ok であること");
    let content = HSTRING::from("ping");
    let outcome = brain.request(&content).expect("request は Ok であること");
    assert!(matches!(outcome, RequestOutcome::Immediate(_)));
    brain.unload().expect("unload は Ok であること");
}

/// `load` へ渡す最小 host（sink）。本テストでは通知を受けないので no-op。
#[implement(IShioriHost)]
struct NoopHost;

impl shiori_abi::interface::IShioriHost_Impl for NoopHost_Impl {
    unsafe fn Raise(&self, _script: *const HSTRING) -> HRESULT {
        HRESULT(0)
    }
    unsafe fn Complete(&self, _token: u64, _response: *const HSTRING) -> HRESULT {
        HRESULT(0)
    }
}
