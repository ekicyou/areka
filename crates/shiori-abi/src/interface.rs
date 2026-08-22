//! `IShioriFactory` / `IShiori` / `IShioriHost` raw COM インターフェイス定義。
//!
//! 3 つの内部 COM 境界を型付き引数・pub メソッド・新 IID で定義する（design.md §InterfaceLayer）:
//! - `IShioriFactory`: 生成＋load 融合。`CreateInstance` で load 完了済み `IShiori` を move-out する
//!   唯一の生成面（脳側が実装・R8.1）。
//! - `IShiori`: 脳側が実装する痩身境界。`Get`（同期 request の即時/遅延 2 分岐）＋`Notify`（片道通知）。
//!   `Load`/`Unload` は**存在しない**（create 融合＋Drop teardown・R9.1/9.2）。
//! - `IShioriHost`: areka 本体が実装し脳へ渡す単一 sink。能動通知（`Raise`）・遅延応答（`Complete`）に
//!   加えプロパティアクセス（`GetProperty`/`SetProperty`）を持つ（R10.1）。
//!
//! ## windows-core 型付き COM 契約面（design.md §InterfaceLayer・R11.1/11.3）
//! windows-core 0.62 の `#[interface(IID)]` で `unsafe trait T: IUnknown` を定義する。これにより
//! vtable（`T_Vtbl`）・`windows_core::Interface` 実装・IID・実装用トレイト `T_Impl` が生成される。
//! メソッド引数を**型付き**（`&HSTRING` 借用／`&mut HSTRING`／[`Ref`]／[`OutRef`]）で宣言することで
//! メソッド本体から raw ポインタ操作を排し、マクロ固定の `unsafe` を**空洞化**する（R11.3）。
//! メソッドは `pub` 宣言し、マクロが `#vis` を生成呼出ラッパへ伝播することで consumer の
//! vtable 直呼びハックを全廃可能にする（R12.5・SafeSurface 層で活用）。
//!
//! ## HSTRING 所有権規約（design.md §InterfaceLayer・R5.4）
//! - `[in]`（`&HSTRING`）= **借用**。callee は呼び出し中の読み取りのみ可で解放してはならない。
//!   呼び出し後に内容を保持する場合は callee 側で clone する。
//! - `[out]`（`&mut HSTRING`）= **callee が確保し caller が解放**する move-out。callee は成功時に
//!   out-param へ HSTRING を書き込み、その所有権を caller へ移す（caller が Drop で解放）。
//! - interface `[in]`（[`Ref`]`<'_, T>`）= 借用。callee が保持する場合は clone（AddRef）する。
//! - interface `[out]`（[`OutRef`]`<'_, T>`）= callee が move-out（caller が Release）。
//!
//! ## Get の応答契約（design.md §InterfaceLayer・R9.1）
//! `Get` は**同期呼び出し**で、成功 2 値ゆえ HRESULT を生返しする（`Result<T≠()>` は vtable 不可・
//! `.ok()` は成功コードを潰すため生返し必須）:
//! - 即時応答: `S_OK` を返し `out_response` に応答 HSTRING を move-out（`out_token` は未使用）。
//! - 遅延: [`crate::error::SHIORI_S_PENDING`]（**成功**コード）を返し `out_token` に有効な相関トークンを
//!   書き込む（`out_response` は空＝未書き込みで解放対象なし）。後続の完了応答は
//!   [`IShioriHost::Complete`] で配送する。
//! - 失敗: error HRESULT を返す。失敗時はいずれの out-param も未書き込み。
//!
//! GET/NOTIFY は SHIORI/3.0 の GET SHIORI/3.0 / NOTIFY SHIORI/3.0 に意味対応する（R9.3）。

// COM 規約に従い PascalCase メソッド名を用いる。`#[interface]` は生成 vtable/実装トレイト面にも
// PascalCase を伝播するため、本 ABI モジュール全体で non_snake_case を許可する。
#![allow(non_snake_case)]

use windows_core::{HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, OutRef, Ref, Result, interface};

/// 脳側が実装する生成面（生成＋load 融合・design.md §InterfaceLayer, R8.1/8.3/11.1）。
///
/// IID（dev 用・流動契約下でリリースまで変動可・**リリース時に凍結**）: 本タスクで v4 採番した
/// `7E072AA3-2288-444E-A42F-779428A9545A`。
///
/// [`CreateInstance`](IShioriFactory::CreateInstance) が `ReferenceBrain` 相当の脳を構築し、
/// load 完了済みの `IShiori` を `out`（[`OutRef`]）へ move-out する唯一の生成経路。旧
/// `IShiori::Load`/`Unload` はこの融合により消滅した（R9.2）。
#[interface("7E072AA3-2288-444E-A42F-779428A9545A")]
pub unsafe trait IShioriFactory: IUnknown {
    /// 脳を生成し、load 完了済みの `IShiori` を `out` へ move-out する（R8.1/8.3）。
    ///
    /// 生成＋load 融合。成功 1 値ゆえ `Result<()>` 可（生成物は `out` の [`OutRef`] 経由で move-out）。
    ///
    /// - `load_dir`: `[in]` ロードディレクトリ（借用・callee 保持は clone）。
    /// - `shiori_name`: `[in]` SHIORI 名（借用・callee 保持は clone）。
    /// - `host`: `[in]` areka 本体が実装する sink（[`Ref`]`<'_, IShioriHost>` 借用・callee 保持は clone/AddRef）。
    /// - `out`: `[out]` load 完了済み `IShiori`（[`OutRef`] へ callee move-out・caller が Release）。
    ///   失敗時は out 未書込（半構築非露出・R8.6）。
    ///
    /// # 戻り値
    /// 成功 = `Ok(())`（`out` に move-out 済み） / 失敗 = error（`out` 未書込）。
    pub unsafe fn CreateInstance(
        &self,
        load_dir: &HSTRING,
        shiori_name: &HSTRING,
        host: Ref<'_, IShioriHost>,
        out: OutRef<'_, IShiori>,
    ) -> Result<()>;
}

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

/// areka 本体が実装し脳へ渡す単一 sink（design.md §InterfaceLayer, R10.1）。
///
/// IID（dev 用・流動契約下でリリースまで変動可・**リリース時に凍結**）: 本タスクで v4 採番した
/// `15E93FE3-7635-496C-9F0F-7B07027EAF98`（旧 IShioriHost IID `03BB53C6-...` からは再採番）。
///
/// 能動通知（[`Raise`](IShioriHost::Raise)）・遅延応答（[`Complete`](IShioriHost::Complete)）に加え、
/// プロパティアクセス（[`GetProperty`](IShioriHost::GetProperty)/[`SetProperty`](IShioriHost::SetProperty)）を
/// 単一 sink が受ける（R10.1）。sink は create 時に `Ref` 渡し（callee clone）で共同所有され、
/// インスタンス生存中 callback 可能。`GetProperty` はストアから同期即答する（R10.3）。
#[interface("15E93FE3-7635-496C-9F0F-7B07027EAF98")]
pub unsafe trait IShioriHost: IUnknown {
    /// 能動通知（wakeup）。脳が areka へ自発的に通知内容を届ける（R10.1）。
    ///
    /// - `script`: `[in]` 通知内容（不透明 HSTRING）。**借用**——host は呼び出し中の読み取りのみ可で
    ///   解放しない。保持時は host 側で clone する。
    pub unsafe fn Raise(&self, script: &HSTRING) -> Result<()>;

    /// 遅延リクエストの完了応答を配送する（R10.1）。
    ///
    /// [`IShiori::Get`] が [`crate::error::SHIORI_S_PENDING`] を返した際に発行した相関トークンで
    /// 元 request と突き合わせる。
    ///
    /// - `token`: 相関トークン（`u64`）。`Get` が遅延時に `out_token` へ書き込んだ値。
    /// - `response`: `[in]` 応答内容（不透明 HSTRING・借用・保持時は clone）。
    ///
    /// # 戻り値
    /// - 成功 = `Ok(())`。
    /// - 突合不能/stale トークン（未知）= [`crate::error::SHIORI_E_UNKNOWN_TOKEN`] 由来の error。
    pub unsafe fn Complete(&self, token: u64, response: &HSTRING) -> Result<()>;

    /// プロパティストアから値を同期即答する（R10.1/10.3）。
    ///
    /// - `key`: `[in]` プロパティキー（dotted パス・借用）。
    /// - `out_value`: `[out]` 値（callee が確保し caller が解放する move-out）。
    ///
    /// # 戻り値
    /// - 成功 = `Ok(())`（`out_value` に move-out）。
    /// - 欠落 key = [`crate::error::SHIORI_E_PROPERTY_NOT_FOUND`] 由来の error（`out_value` 未書込）。
    pub unsafe fn GetProperty(&self, key: &HSTRING, out_value: &mut HSTRING) -> Result<()>;

    /// プロパティストアへ値を即書きする（R10.1）。
    ///
    /// - `key`: `[in]` プロパティキー（借用）。
    /// - `value`: `[in]` 設定値（借用・保持は clone）。
    pub unsafe fn SetProperty(&self, key: &HSTRING, value: &HSTRING) -> Result<()>;
}

#[cfg(test)]
mod tests {
    //! 最小モック実装による vtable 健全性の単体証明。
    //!
    //! 3 interface（`IShioriFactory`/`IShiori`/`IShioriHost`）を `#[implement]` した小さな
    //! struct を立て、混在署名（[`Ref`]/[`OutRef`]/`&HSTRING`/`&mut`/`Result<()>`/`-> HRESULT`）の
    //! interface が `#[implement]` と組んで vtable dispatch まで通ることをコンパイル＋実行で実証する。
    //! 併せて 3 IID の固定（相互不一致・旧 2 IID 不一致）を検証する。

    use super::*;
    use windows_core::implement;

    // --- IShiori（脳・Get/Notify）の最小モック ---

    /// 即時応答時に move-out する固定文字列。
    const IMMEDIATE_RESPONSE: &str = "pong";
    /// 遅延応答時に発行する固定相関トークン。
    const PENDING_TOKEN: u64 = 0xDEAD_BEEF;
    /// 欠落 key を示すテストローカル HRESULT（customer bit 立ち・失敗）。error.rs の
    /// `SHIORI_E_PROPERTY_NOT_FOUND` は Task 8.2 で追加予定のため、本タスクでは error.rs へ
    /// 依存せずローカル定数で dispatch を実証する（値は 8.2 の採番 `0xA0A1_0004` と一致）。
    const TEST_PROPERTY_NOT_FOUND: HRESULT = HRESULT(0xA0A1_0004u32 as i32);

    /// vtable 健全性検証用の最小モック脳。
    ///
    /// `Get` は `defer` フラグで即時（`S_OK`＋`out_response` move-out）と遅延
    /// （[`crate::error::SHIORI_S_PENDING`]＋`out_token`）の 2 分岐を切り替える。`Notify` は受領を記録する。
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
                crate::error::SHIORI_S_PENDING
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

    // --- IShioriHost（sink・Raise/Complete/GetProperty/SetProperty）の最小モック ---

    /// vtable 健全性検証用の最小モック host（areka 本体側 sink の代役）。
    ///
    /// 単一 sink が能動通知・遅延応答・プロパティアクセスの全てを受けることを 1 struct で証明する。
    #[implement(IShioriHost)]
    struct MockHost {
        raised: std::cell::RefCell<Vec<HSTRING>>,
        completed: std::cell::RefCell<Vec<(u64, HSTRING)>>,
        properties: std::cell::RefCell<std::collections::HashMap<String, HSTRING>>,
        valid_token: u64,
    }

    impl IShioriHost_Impl for MockHost_Impl {
        unsafe fn Raise(&self, script: &HSTRING) -> Result<()> {
            self.raised.borrow_mut().push(script.clone());
            Ok(())
        }

        unsafe fn Complete(&self, token: u64, response: &HSTRING) -> Result<()> {
            if token != self.valid_token {
                // 突合不能/stale トークンは host が SHIORI_E_UNKNOWN_TOKEN 由来の error を返す。
                return Err(crate::error::SHIORI_E_UNKNOWN_TOKEN.into());
            }
            self.completed.borrow_mut().push((token, response.clone()));
            Ok(())
        }

        unsafe fn GetProperty(&self, key: &HSTRING, out_value: &mut HSTRING) -> Result<()> {
            match self.properties.borrow().get(&key.to_string()) {
                Some(v) => {
                    // 存在時: 値を out へ move-out（callee 確保・caller 解放）。
                    *out_value = v.clone();
                    Ok(())
                }
                // 欠落 key は PROPERTY_NOT_FOUND（out_value 未書込）。
                None => Err(TEST_PROPERTY_NOT_FOUND.into()),
            }
        }

        unsafe fn SetProperty(&self, key: &HSTRING, value: &HSTRING) -> Result<()> {
            self.properties
                .borrow_mut()
                .insert(key.to_string(), value.clone());
            Ok(())
        }
    }

    /// テスト用 host のインスタンス化ヘルパ。
    fn mock_host(valid_token: u64) -> IShioriHost {
        MockHost {
            raised: std::cell::RefCell::new(Vec::new()),
            completed: std::cell::RefCell::new(Vec::new()),
            properties: std::cell::RefCell::new(std::collections::HashMap::new()),
            valid_token,
        }
        .into()
    }

    // --- IShioriFactory（生成面）の最小モック ---

    /// vtable 健全性検証用の最小モック factory。
    ///
    /// `CreateInstance` は `MockBrain` を構築し `out`（[`OutRef`]）へ move-out する。`host` の
    /// [`Ref`] 借用が dispatch まで通ること（interface 引数の型付き受け渡し）を実証する。
    #[implement(IShioriFactory)]
    struct MockFactory {
        defer: bool,
    }

    impl IShioriFactory_Impl for MockFactory_Impl {
        unsafe fn CreateInstance(
            &self,
            _load_dir: &HSTRING,
            _shiori_name: &HSTRING,
            host: Ref<'_, IShioriHost>,
            out: OutRef<'_, IShiori>,
        ) -> Result<()> {
            // `host` は Ref 借用。生存中は AddRef された参照を得られる（本モックは観測のみ）。
            let _host_ref: Option<&IShioriHost> = host.as_ref();
            let brain: IShiori = MockBrain {
                defer: self.defer,
                notified: std::cell::RefCell::new(Vec::new()),
            }
            .into();
            // load 完了済み brain を out へ move-out（callee 確保・caller Release）。
            out.write(Some(brain))?;
            Ok(())
        }
    }

    /// factory 経由 create → 即時 Get の混在署名 dispatch がコンパイル＋実行で通ること。
    /// （`Ref`＋`OutRef`＋`&HSTRING`＋`&mut`＋`Result<()>`＋`-> HRESULT` の混在実証・設計判断 (a)。）
    #[test]
    fn factory_create_and_immediate_get_dispatch_works() {
        let factory: IShioriFactory = MockFactory { defer: false }.into();
        let host = mock_host(42);

        // CreateInstance: host を Ref 借用で渡し、brain を OutRef で受ける。
        let load_dir = HSTRING::from("C:/ghost/master");
        let shiori_name = HSTRING::from("reference");
        let mut out: Option<IShiori> = None;
        unsafe {
            factory
                .CreateInstance(&load_dir, &shiori_name, (&host).into(), (&mut out).into())
                .expect("CreateInstance は Ok であること");
        }
        let brain = out.expect("out に brain が move-out されていること");

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
    }

    /// 遅延 Get: `SHIORI_S_PENDING`＋`out_token` の分岐が dispatch すること。
    #[test]
    fn deferred_get_returns_pending_with_token() {
        let factory: IShioriFactory = MockFactory { defer: true }.into();
        let host = mock_host(42);

        let mut out: Option<IShiori> = None;
        unsafe {
            factory
                .CreateInstance(
                    &HSTRING::from("dir"),
                    &HSTRING::from("name"),
                    (&host).into(),
                    (&mut out).into(),
                )
                .expect("CreateInstance は Ok であること");
        }
        let brain = out.expect("out に brain が move-out されていること");

        let input = HSTRING::from("GET SHIORI/3.0...");
        let mut out_response = HSTRING::new();
        let mut out_token: u64 = 0;
        let hr = unsafe { brain.Get(&input, &mut out_response, &mut out_token) };
        assert_eq!(
            hr,
            crate::error::SHIORI_S_PENDING,
            "遅延 Get は SHIORI_S_PENDING を返すこと: 0x{:08X}",
            hr.0
        );
        assert_eq!(
            out_token, PENDING_TOKEN,
            "out_token に相関トークンが書かれること"
        );
    }

    /// host sink の全 4 面（Raise/Complete/GetProperty/SetProperty）が dispatch すること。
    /// 未知トークンの `Complete` と欠落 key の `GetProperty` が error を返すことも観測する。
    #[test]
    fn host_sink_all_methods_dispatch() {
        let host = mock_host(42);

        // Raise（能動通知・Result<()>）。
        let script = HSTRING::from("\\h\\s[0]hello");
        unsafe { host.Raise(&script) }.expect("Raise は Ok であること");

        // Complete（有効トークン）。
        let response = HSTRING::from("response-body");
        unsafe { host.Complete(42, &response) }.expect("有効トークンの Complete は Ok であること");

        // Complete（未知トークン）は error。
        let err = unsafe { host.Complete(999, &response) }
            .expect_err("未知トークンの Complete は error であること");
        assert_eq!(
            err.code(),
            crate::error::SHIORI_E_UNKNOWN_TOKEN,
            "未知トークンは SHIORI_E_UNKNOWN_TOKEN であること"
        );

        // SetProperty → GetProperty（同期即答・move-out）。
        let key = HSTRING::from("path.to.key");
        let value = HSTRING::from("some-value");
        unsafe { host.SetProperty(&key, &value) }.expect("SetProperty は Ok であること");
        let mut out_value = HSTRING::new();
        unsafe { host.GetProperty(&key, &mut out_value) }.expect("GetProperty は Ok であること");
        assert_eq!(out_value, value, "設定した値が move-out されること");

        // 欠落 key の GetProperty は error（out_value 未書込）。
        let missing = HSTRING::from("no.such.key");
        let mut out_missing = HSTRING::new();
        let err = unsafe { host.GetProperty(&missing, &mut out_missing) }
            .expect_err("欠落 key の GetProperty は error であること");
        assert_eq!(
            err.code(),
            TEST_PROPERTY_NOT_FOUND,
            "欠落 key は PROPERTY_NOT_FOUND であること"
        );
    }

    /// 3 interface の IID が本タスクで採番した v4 GUID に固定されていること（回帰防止）。
    /// **3 本相互不一致**＋**旧 2 IID（旧 IShiori/旧 IShioriHost）との不一致**を assert する。
    #[test]
    fn iids_are_fixed_and_mutually_distinct() {
        use windows_core::{GUID, Interface};

        let factory_iid = <IShioriFactory as Interface>::IID;
        let shiori_iid = <IShiori as Interface>::IID;
        let host_iid = <IShioriHost as Interface>::IID;

        // 採番値への固定。
        assert_eq!(
            factory_iid,
            GUID::from_u128(0x7E072AA3_2288_444E_A42F_779428A9545A),
            "IShioriFactory の IID は採番値に固定されていること"
        );
        assert_eq!(
            shiori_iid,
            GUID::from_u128(0x327998D4_8491_4912_BA85_6B8F52831B14),
            "IShiori の IID は採番値に固定されていること"
        );
        assert_eq!(
            host_iid,
            GUID::from_u128(0x15E93FE3_7635_496C_9F0F_7B07027EAF98),
            "IShioriHost の IID は採番値に固定されていること"
        );

        // 3 本相互不一致。
        assert_ne!(factory_iid, shiori_iid, "Factory と Shiori の IID は不一致");
        assert_ne!(factory_iid, host_iid, "Factory と Host の IID は不一致");
        assert_ne!(shiori_iid, host_iid, "Shiori と Host の IID は不一致");

        // 旧 2 IID との不一致（旧 IShiori・旧 IShioriHost。旧 factory は存在しない）。
        let old_shiori = GUID::from_u128(0xE7887AB4_525D_4520_9474_577528758C79);
        let old_host = GUID::from_u128(0x03BB53C6_6496_47FB_B1B7_84356A94A9C7);
        assert_ne!(
            shiori_iid, old_shiori,
            "新 IShiori IID は旧 IShiori IID と不一致"
        );
        assert_ne!(
            host_iid, old_host,
            "新 IShioriHost IID は旧 IShioriHost IID と不一致"
        );
        assert_ne!(
            factory_iid, old_shiori,
            "新 Factory IID は旧 IShiori IID と不一致"
        );
        assert_ne!(
            factory_iid, old_host,
            "新 Factory IID は旧 IShioriHost IID と不一致"
        );
    }
}
