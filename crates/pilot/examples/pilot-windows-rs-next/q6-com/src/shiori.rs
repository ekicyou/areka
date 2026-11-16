// COM 規約に従い PascalCase メソッド名を用いる。`#[interface]` は生成 vtable/実装トレイト面にも
// PascalCase を伝播するため、本 ABI モジュール全体で non_snake_case を許可する。
#![allow(non_snake_case)]

use windows_core::{HRESULT, HSTRING, IUnknown, Result, implement, interface};

/// 脳側が実装する痩身 COM 境界（design.md §InterfaceLayer, R9.1/9.2/9.3）。
///
/// IID（dev 用・流動契約下でリリースまで変動可・**リリース時に凍結**）: 本タスクで v4 採番した
/// `327998D4-8491-4912-BA85-6B8F52831B14`（旧 IShiori IID `E7887AB4-...` からは再採番）。
///
/// `Get`（同期 request・GET SHIORI/3.0 後継）＋`Notify`（片道通知・NOTIFY SHIORI/3.0 後継）の 2 操作
/// のみ。`Load`/`Unload` は存在しない（[`IShioriFactory::CreateInstance`] の create 融合＋Drop
/// teardown で代替・R9.2）。詳細な契約（HSTRING 所有権規約・Get の 2 分岐）はモジュール doc 参照。
#[interface("327998D4-8491-4912-BA85-6B8F52831B14")]
pub unsafe trait IShiori: IUnknown {
    /// 同期 request。即時/遅延を HRESULT で 2 分岐する（R9.1・GET SHIORI/3.0 後継）。
    ///
    /// 成功 2 値（`S_OK`=即時／[`crate::error::SHIORI_S_PENDING`]=遅延+token）ゆえ HRESULT 生返し必須。
    ///
    /// - `input`: `[in]` 正準 content（不透明 HSTRING）。**借用**——callee は読み取りのみで解放しない。
    /// - `out_response`: `[out]` 即時応答（`S_OK` 時のみ有効）。callee が確保し caller が解放する move-out。
    /// - `out_token`: `[out]` 相関トークン（[`crate::error::SHIORI_S_PENDING`] 時のみ有効）。
    ///
    /// # 戻り値
    /// - `S_OK`: 即時応答（`out_response` に move-out、`out_token` 未使用）。
    /// - [`crate::error::SHIORI_S_PENDING`]: 遅延（`out_token` に有効トークン、`out_response` 空）。
    ///   後続の完了応答は [`IShioriHost::Complete`] で配送する。
    /// - error HRESULT: 失敗（out-param いずれも未書き込み）。
    pub unsafe fn Get(
        &self,
        input: &HSTRING,
        out_response: &mut HSTRING,
        out_token: &mut u64,
    ) -> HRESULT;

    /// 片道通知（応答なし・R9.1・NOTIFY SHIORI/3.0 後継）。
    ///
    /// - `input`: `[in]` 通知 content（借用・callee 保持は clone）。
    ///
    /// # 戻り値
    /// 成功 = `Ok(())` / 失敗 = error。応答は返さない（片道性）。
    pub unsafe fn Notify(&self, input: &HSTRING) -> Result<()>;
    // Load/Unload は存在しない（create 融合＋Drop teardown・R9.2）。
}

/// 遅延の成功コード（`shiori-abi` の `error.rs` と同じ値 `0x20A1_0001`。写しは `error.rs` を持たない）。
const SHIORI_S_PENDING: HRESULT = HRESULT(0x20A1_0001);

/// 即時応答時に move-out する固定文字列。
const IMMEDIATE_RESPONSE: &str = "pong";
/// 遅延応答時に発行する固定相関トークン。
const PENDING_TOKEN: u64 = 0xDEAD_BEEF;

/// vtable 健全性検証用の最小モック脳。
///
/// `Get` は `defer` フラグで即時（`S_OK`＋`out_response` move-out）と遅延
/// （[`SHIORI_S_PENDING`]＋`out_token`）の 2 分岐を切り替える。`Notify` は受領を記録する。
#[implement(IShiori)]
struct MockBrain {
    defer: bool,
    notified: std::cell::RefCell<Vec<HSTRING>>,
}

impl IShiori_Impl for MockBrain_Impl {
    unsafe fn Get(
        &self,
        _input: &HSTRING,
        out_response: &mut HSTRING,
        out_token: &mut u64,
    ) -> HRESULT {
        if self.defer {
            // 遅延: 相関トークンを out_token へ（out_response は未書き込み＝空のまま）。
            *out_token = PENDING_TOKEN;
            SHIORI_S_PENDING
        } else {
            // 即時応答: callee が確保した HSTRING を out-param へ move-out（所有権規約）。
            *out_response = HSTRING::from(IMMEDIATE_RESPONSE);
            HRESULT(0) // S_OK
        }
    }

    unsafe fn Notify(&self, input: &HSTRING) -> Result<()> {
        // `[in]` 借用: 保持する場合は clone する（所有権規約）。
        self.notified.borrow_mut().push(input.clone());
        Ok(())
    }
}

/// 写した呼び出し（試験の中の即時 `Get` と `Notify`）を bin から走らせる。
pub fn run() {
    let brain: IShiori = MockBrain {
        defer: false,
        notified: std::cell::RefCell::new(Vec::new()),
    }
    .into();

    // 即時 Get: S_OK と move-out された応答を観測する。
    let input = HSTRING::from("GET SHIORI/3.0...");
    let mut out_response = HSTRING::new();
    let mut out_token: u64 = 0;
    let hr = unsafe { brain.Get(&input, &mut out_response, &mut out_token) };
    assert!(hr.is_ok(), "即時 Get は S_OK を返すこと: 0x{:08X}", hr.0);
    assert_eq!(
        out_response,
        HSTRING::from(IMMEDIATE_RESPONSE),
        "move-out された即時応答を観測できること"
    );

    // Notify（片道・Result<()>）が dispatch すること。
    let notify_input = HSTRING::from("NOTIFY SHIORI/3.0...");
    unsafe { brain.Notify(&notify_input) }.expect("Notify は Ok であること");
    println!(
        "q6 IShiori::Get hr=0x{:08X} response={}",
        hr.0,
        out_response.display()
    );
}
