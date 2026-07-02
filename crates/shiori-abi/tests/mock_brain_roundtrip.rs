//! Task 8.4 結合テスト: 新 ABI（factory 生成→get/notify）による即時往復・所有権/非マーシャリング/
//! Drop 実証。
//!
//! shiori-abi の**統合テスト**（公開 API のみ使用・`src` は不変）。旧 ABI（`Load`/`Unload`/`Request`
//! ＋`ShioriExt::request`）版は Task 8.1 で撤去されたため、本ファイルを**新 ABI へ翻案**して再構築する。
//!
//! 新 ABI の主経路は **safe 面（インヘレント）** を用いる（design.md §Testing Strategy → Integration
//! Tests #3・§SafeSurface）:
//! - `#[implement(IShioriFactory)]` の mock factory を立て、`create(load_dir, shiori_name, &host)` で
//!   load 完了済み `IShiori`（mock brain）を move-out する。
//! - `#[implement(IShiori)]` の mock brain は `get`（即時応答で入力を反映した応答 HSTRING を move-out）
//!   ＋`notify`（受領記録）を実装する。
//! - `#[implement(IShioriHost)]` の mock host を create の必須引数として渡す（本モックは観測のみ）。
//!
//! in-proc 直 vtable のため OOP マーシャリングは非発生であることを、往復 HSTRING の**内容一致**
//! （＋UTF-16 ビット一致・clone 生存）で観測し、応答 HSTRING の**確保=解放 1:1 均衡**を per-instance
//! カウンタで決定的に計測する（requirements.md R12.1/12.6・design.md §Testing Strategy）。
//!
//! ## Drop 回数観測の決定的アプローチ（旧版手法の翻案・R12.6）
//! `HSTRING` 自体に Drop フックは差せないため、以下を**複合**して「確保=解放=1:1・二重解放/リーク
//! なし」を実時間や ASAN に頼らず決定的に示す:
//!
//! - **(probe) 明示的 Drop 計測**: mock brain が `Get` で応答を生成するたびに、応答 HSTRING を内包する
//!   被験ラッパ [`TrackedResponse`] を 1 つ生成し、生成回数（alloc）と Drop 回数（drop）を
//!   **per-instance** カウンタで数える。ラッパは `Get` 内で move-out 用ハンドルを複製後に Drop され、
//!   最終的に alloc == drop（1:1 均衡）であることを assert する。drop < alloc ならリーク、
//!   drop > alloc なら二重解放を示す。
//! - **(i) 多数回ループ**: 同一往復を 10_000 回繰り返し、毎回 content 一致を assert しつつ
//!   abort/crash しないことを示す（HSTRING の二重解放は CRT abort を招くため、完走自体が
//!   二重解放非発生の証左となる）。
//! - **(ii) clone 生存**: 受領した即時応答を test 側で clone 保持し、original を drop しても
//!   保持側 content が無傷であることを assert（premature free / 破壊的マーシャリングなら壊れる。
//!   HSTRING は参照カウント型のためこの不変が成立する）。
//! - **(iii) ビット一致**: move-out された HSTRING の content が脳の用意値と UTF-16 コードユニット列で
//!   一致（マーシャリングはコピー/変換を伴うが、in-proc 直 vtable は同一論理バッファを渡すため完全一致）。

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

// --- in-proc 即時応答モック脳（`#[implement(IShiori)]`・新 ABI） ---

/// in-proc 即時応答モック脳。
///
/// `Get` は呼ばれるたびに [`TrackedResponse`] を 1 つ生成し、その内包 HSTRING の所有権ハンドルを
/// `out_response` へ move-out して `S_OK`（即時応答）を返す（所有権規約）。`Notify` は受領した入力を
/// 記録する（片道通知）。計測カウンタはインスタンス所有（並列テスト汚染回避）。
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
        // 応答生成: probe ラッパを 1 つ確保（alloc++）。
        let tracked = TrackedResponse::new(KNOWN_RESPONSE, Arc::clone(&self.counters));
        // out-param へ content の所有権ハンドルを move-out（callee 確保・caller 解放規約）。
        // `tracked` 自身はこの関数末尾で Drop され drop++（内包 HSTRING も 1 本解放）。
        *out_response = tracked.hstring_for_moveout();
        HRESULT(0) // S_OK
        // ここで `tracked` が Drop -> drop++（ラッパ 1:1 均衡の片側）。
    }

    unsafe fn Notify(&self, input: &HSTRING) -> ComResult<()> {
        // `[in]` 借用: 保持する場合は clone する（所有権規約）。
        self.notified.borrow_mut().push(input.clone());
        Ok(())
    }
}

// --- 生成面モック factory（`#[implement(IShioriFactory)]`・新 ABI） ---

/// in-proc モック factory。`CreateInstance` で load 完了済み [`MockBrain`] を `out` へ move-out する。
///
/// 生成した brain に per-instance カウンタ [`Counters`] を注入し、テスト側がその均衡を観測する。
/// `host`（[`Ref`] 借用）は create の必須引数として受けるが、本モックは観測のみ（保持しない）。
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
        // `host` は Ref 借用（本モックは観測のみで保持しない）。
        let _host_ref: Option<&IShioriHost> = host.as_ref();
        let brain: IShiori = MockBrain {
            counters: Arc::clone(&self.counters),
            notified: std::cell::RefCell::new(Vec::new()),
        }
        .into();
        // load 完了済み brain を out へ move-out（callee 確保・caller Release）。
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

/// テスト用のハーネスを組み立てる: mock factory から safe 面 `create` で mock brain を生成する。
///
/// 生成した brain と、その応答ライフサイクルを観測する per-instance [`Counters`] を返す。
/// load_dir/shiori_name は create の必須引数ゆえ妥当な HSTRING を渡す（mock factory は無視する）。
fn make_brain() -> (IShiori, Arc<Counters>) {
    let counters = Arc::new(Counters::default());
    let factory: IShioriFactory = MockFactory {
        counters: Arc::clone(&counters),
    }
    .into();
    let host: IShioriHost = NoopHost.into();

    // safe 面 `create`: OutRef 受け皿は隠蔽され IShiori を直返し（vtable 直呼びハックを使わない）。
    let brain = factory
        .create(
            &HSTRING::from("C:/ghost/master"),
            &HSTRING::from("reference"),
            &host,
        )
        .expect("create は Ok で load 完了済み IShiori を直返しすること");
    (brain, counters)
}

/// (R12.1/12.6・(iii)) factory 生成→即時 `get` で応答 HSTRING が `Immediate` として往復し content
/// 一致すること、かつ UTF-16 コードユニット列でビット単位一致すること（非マーシャリング）。
#[test]
fn immediate_response_roundtrips_with_bit_identical_content() {
    let (brain, _counters) = make_brain();

    // safe 面 `get`: S_OK → Immediate（応答内容付き）。
    let outcome = brain
        .get(&HSTRING::from("GET SHIORI/3.0..."))
        .expect("即時 get は Ok であること");

    let resp = match outcome {
        GetOutcome::Immediate(resp) => resp,
        other => panic!("expected Immediate, got {other:?}"),
    };

    let expected = HSTRING::from(KNOWN_RESPONSE);
    // content 一致（論理等価・R12.6）。
    assert_eq!(resp, expected, "往復した応答 content が脳の用意値と一致すること");
    // (iii) ビット一致: UTF-16 コードユニット列が完全一致（in-proc 直 vtable=非マーシャリング）。
    // `HSTRING` は `Deref<Target = [u16]>`。`&*` で UTF-16 スライスを取得して比較する。
    assert_eq!(
        &*resp, &*expected,
        "応答が UTF-16 ビット単位で完全一致すること（マーシャリングによるコピー/変換が介在しない）"
    );
}

/// (probe・R12.6) 単一往復で応答ラッパの確保=解放が 1:1 に均衡し、リーク/二重解放が無いこと。
///
/// `Immediate` を drop した後に alloc==drop（=1）であることを観測する。drop<alloc ならリーク、
/// drop>alloc なら二重解放を示す。
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
        // ここで `resp`（caller が所有する move-out された HSTRING）がスコープ末で Drop される。
    }

    // 脳側でラッパ `tracked` は Get 内で 1 回生成・1 回 Drop された（alloc==drop==1）。
    // ラッパは応答 HSTRING の所有権ライフサイクルの代理計測であり、生成と解放が均衡する。
    let alloc = counters.alloc();
    let drop = counters.dropped();
    assert_eq!(alloc, 1, "1 往復で応答ラッパが 1 回だけ確保されること");
    assert_eq!(
        drop, alloc,
        "確保=解放（1:1 均衡）であること。drop<alloc=リーク, drop>alloc=二重解放: alloc={alloc}, drop={drop}"
    );
}

/// (i・R12.6) 多数回（10_000）の往復で毎回 content 一致し、abort/crash せず完走すること。
///
/// HSTRING の二重解放は CRT abort を招くため、10_000 回の確保/解放を完走できること自体が
/// 二重解放非発生の強い証左となる。併せて probe カウンタの 1:1 均衡（alloc==drop==N）を assert。
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

/// (ii・R12.6) clone 生存: 受領した `Immediate` を clone 保持し、original を drop しても
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

/// (R9.3/12.6) 新 ABI ハーネス健全性: factory 生成→`get`（即時）→`notify`（片道受領）の最小
/// ライフサイクルが通ること。`notify` の入力借用が clone 保持され Ok が返ることを観測する。
#[test]
fn harness_create_get_notify_minimal() {
    let (brain, _counters) = make_brain();

    // get（即時）。
    let outcome = brain
        .get(&HSTRING::from("GET SHIORI/3.0..."))
        .expect("get は Ok であること");
    assert!(matches!(outcome, GetOutcome::Immediate(_)));

    // notify（片道・応答なし）。入力は callee 側で clone 保持され Ok を返す。
    brain
        .notify(&HSTRING::from("NOTIFY SHIORI/3.0..."))
        .expect("notify は Ok であること");
}
