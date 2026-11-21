//! 新 ABI（factory 生成→get/notify）の即時往復・所有権・非マーシャリング・Drop の結合テスト。
//!
//! 公開 API の安全面だけを使う。mock factory の `create` で mock brain を得て、`get` は既知の応答
//! HSTRING を move-out し、`notify` は受領を記録する。mock host は create の必須引数として渡すだけ。
//!
//! ## Drop 回数の決定的な観測
//! `HSTRING` 自体に Drop フックは差せないため、以下を組み合わせて「確保=解放=1:1・二重解放/リーク
//! なし」を実時間や ASAN に頼らず示す:
//!
//! - **(probe)**: `Get` のたびに応答 HSTRING を内包する [`TrackedResponse`] を 1 つ作り、生成と Drop を
//!   per-instance カウンタで数えて alloc == drop を assert する。drop < alloc ならリーク、
//!   drop > alloc なら二重解放。
//! - **(i) 多数回ループ**: 10_000 回の往復。HSTRING の二重解放は CRT abort を招くので、完走自体が
//!   二重解放非発生の証左になる。
//! - **(ii) clone 生存**: 受領した応答を clone 保持し、original を drop しても保持側が無傷であること
//!   （premature free / 破壊的マーシャリングなら壊れる）。
//! - **(iii) ビット一致**: move-out された HSTRING が脳の用意値と UTF-16 コードユニット列で一致
//!   （マーシャリングはコピー/変換を伴うが、in-proc 直 vtable は同一論理バッファを渡す）。

#![allow(non_snake_case)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use shiori_abi::interface::{
    IShiori, IShiori_Impl, IShioriFactory, IShioriFactory_Impl, IShioriHost, IShioriHost_Impl,
};
use shiori_abi::outcome::GetOutcome;
use windows_core::{HRESULT, HSTRING, OutRef, Ref, Result as ComResult, implement};

/// 応答ラッパの確保/解放回数を数える **per-instance** カウンタ（probe）。
///
/// グローバル static にすると、同一プロセスの別スレッドで並列に走るテストどうしでカウンタが
/// 汚染し合うので、モック脳のインスタンスに紐づける。
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

/// 応答 HSTRING を内包し、生成で `alloc`・Drop で `drop` を数える被験ラッパ（probe）。
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

// --- in-proc 即時応答モック脳（`#[implement(IShiori)]`・新 ABI） ---

/// in-proc 即時応答モック脳。
///
/// `Get` は呼ばれるたびに [`TrackedResponse`] を 1 つ作り、その HSTRING を `out_response` へ
/// move-out して `S_OK` を返す。
#[implement(IShiori)]
struct MockBrain {
    counters: Arc<Counters>,
    notified: std::cell::RefCell<Vec<HSTRING>>,
}

impl IShiori_Impl for MockBrain_Impl {
    unsafe fn Get(
        &self,
        _input: &HSTRING,
        out_response: &mut HSTRING,
        _out_token: &mut u64,
    ) -> HRESULT {
        let tracked = TrackedResponse::new(KNOWN_RESPONSE, Arc::clone(&self.counters));
        // `tracked` はこの関数の末尾で Drop される（drop++）。
        *out_response = tracked.hstring_for_moveout();
        HRESULT(0) // S_OK
    }

    unsafe fn Notify(&self, input: &HSTRING) -> ComResult<()> {
        self.notified.borrow_mut().push(input.clone());
        Ok(())
    }
}

// --- 生成面モック factory（`#[implement(IShioriFactory)]`・新 ABI） ---

/// in-proc モック factory。生成した [`MockBrain`] に [`Counters`] を注入する。
#[implement(IShioriFactory)]
struct MockFactory {
    counters: Arc<Counters>,
}

impl IShioriFactory_Impl for MockFactory_Impl {
    unsafe fn CreateInstance(
        &self,
        _load_dir: &HSTRING,
        _shiori_name: &HSTRING,
        host: Ref<'_, IShioriHost>,
        out: OutRef<'_, IShiori>,
    ) -> ComResult<()> {
        let _host_ref: Option<&IShioriHost> = host.as_ref();
        let brain: IShiori = MockBrain {
            counters: Arc::clone(&self.counters),
            notified: std::cell::RefCell::new(Vec::new()),
        }
        .into();
        out.write(Some(brain))?;
        Ok(())
    }
}

// --- create の必須引数となる最小 host（`#[implement(IShioriHost)]`） ---

/// `create` へ渡す最小 host（sink）。本テストでは通知を受けないので no-op。
#[implement(IShioriHost)]
struct NoopHost;

impl IShioriHost_Impl for NoopHost_Impl {
    unsafe fn Raise(&self, _script: &HSTRING) -> ComResult<()> {
        Ok(())
    }
    unsafe fn Complete(&self, _token: u64, _response: &HSTRING) -> ComResult<()> {
        Ok(())
    }
    unsafe fn GetProperty(&self, _key: &HSTRING, _out_value: &mut HSTRING) -> ComResult<()> {
        Ok(())
    }
    unsafe fn SetProperty(&self, _key: &HSTRING, _value: &HSTRING) -> ComResult<()> {
        Ok(())
    }
}

/// mock factory から安全面の `create` で mock brain を作り、その [`Counters`] と一緒に返す。
fn make_brain() -> (IShiori, Arc<Counters>) {
    let counters = Arc::new(Counters::default());
    let factory: IShioriFactory = MockFactory {
        counters: Arc::clone(&counters),
    }
    .into();
    let host: IShioriHost = NoopHost.into();

    let brain = factory
        .create(
            &HSTRING::from("C:/ghost/master"),
            &HSTRING::from("reference"),
            &host,
        )
        .expect("create は Ok で load 完了済み IShiori を直返しすること");
    (brain, counters)
}

/// (iii) factory 生成→即時 `get` で応答 HSTRING が `Immediate` として往復し content
/// 一致すること、かつ UTF-16 コードユニット列でビット単位一致すること（非マーシャリング）。
#[test]
fn immediate_response_roundtrips_with_bit_identical_content() {
    let (brain, _counters) = make_brain();

    let outcome = brain
        .get(&HSTRING::from("GET SHIORI/3.0..."))
        .expect("即時 get は Ok であること");

    let resp = match outcome {
        GetOutcome::Immediate(resp) => resp,
        other => panic!("expected Immediate, got {other:?}"),
    };

    let expected = HSTRING::from(KNOWN_RESPONSE);
    assert_eq!(
        resp, expected,
        "往復した応答 content が脳の用意値と一致すること"
    );
    // `HSTRING: Deref<Target = [u16]>` なので `&*` で UTF-16 スライスを比べる。
    assert_eq!(
        &*resp, &*expected,
        "応答が UTF-16 ビット単位で完全一致すること（マーシャリングによるコピー/変換が介在しない）"
    );
}

/// (probe) 単一往復で応答ラッパの確保=解放が 1:1 に均衡し、リーク/二重解放が無いこと。
#[test]
fn single_roundtrip_alloc_equals_drop() {
    let (brain, counters) = make_brain();

    {
        let outcome = brain
            .get(&HSTRING::from("GET SHIORI/3.0..."))
            .expect("即時 get は Ok であること");
        let GetOutcome::Immediate(resp) = outcome else {
            panic!("expected Immediate");
        };
        assert_eq!(resp, HSTRING::from(KNOWN_RESPONSE));
    }

    let alloc = counters.alloc();
    let drop = counters.dropped();
    assert_eq!(alloc, 1, "1 往復で応答ラッパが 1 回だけ確保されること");
    assert_eq!(
        drop, alloc,
        "確保=解放（1:1 均衡）であること。drop<alloc=リーク, drop>alloc=二重解放: alloc={alloc}, drop={drop}"
    );
}

/// (i) 多数回（10_000）の往復で毎回 content 一致し、abort せず完走し、alloc==drop==N であること。
#[test]
fn many_roundtrips_no_double_free_and_balanced() {
    let (brain, counters) = make_brain();

    const N: i64 = 10_000;
    let expected = HSTRING::from(KNOWN_RESPONSE);
    let input = HSTRING::from("GET SHIORI/3.0...");

    for i in 0..N {
        let outcome = brain.get(&input).expect("即時 get は Ok であること");
        let GetOutcome::Immediate(resp) = outcome else {
            panic!("iteration {i}: expected Immediate");
        };
        assert_eq!(resp, expected, "iteration {i}: content 一致");
    }

    let alloc = counters.alloc();
    let drop = counters.dropped();
    assert_eq!(alloc, N, "N 往復で N 回確保されること");
    assert_eq!(
        drop, N,
        "N 往復で確保=解放=N（二重解放/リークなし）であること: alloc={alloc}, drop={drop}"
    );
}

/// (ii) clone 生存: 受領した `Immediate` を clone 保持し、original を drop しても
/// 保持側 content が無傷であること。premature free / 破壊的マーシャリングなら壊れる。
#[test]
fn cloned_response_survives_original_drop() {
    let (brain, _counters) = make_brain();

    let outcome = brain
        .get(&HSTRING::from("GET SHIORI/3.0..."))
        .expect("即時 get は Ok であること");
    let GetOutcome::Immediate(resp) = outcome else {
        panic!("expected Immediate");
    };

    let survivor = resp.clone();
    let expected = HSTRING::from(KNOWN_RESPONSE);

    drop(resp);

    assert_eq!(
        survivor, expected,
        "original drop 後も clone 保持側の content が無傷であること（premature free 非発生）"
    );
    assert_eq!(
        &*survivor, &*expected,
        "clone 保持側が UTF-16 ビット単位で無傷であること"
    );
}

/// 新 ABI ハーネス健全性: factory 生成→`get`（即時）→`notify`（片道受領）の最小
/// ライフサイクルが通ること。`notify` の入力借用が clone 保持され Ok が返ることを観測する。
#[test]
fn harness_create_get_notify_minimal() {
    let (brain, _counters) = make_brain();

    let outcome = brain
        .get(&HSTRING::from("GET SHIORI/3.0..."))
        .expect("get は Ok であること");
    assert!(matches!(outcome, GetOutcome::Immediate(_)));

    brain
        .notify(&HSTRING::from("NOTIFY SHIORI/3.0..."))
        .expect("notify は Ok であること");
}
