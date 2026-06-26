//! `IShiori` raw COM インターフェイス定義（脳側が実装する唯一の内部 COM 境界）。
//!
//! windows-core 0.62 のカスタム COM 定義技法（research §8 / §2 で確認）に従い、
//! `#[interface(IID)]` 属性で `unsafe trait IShiori: IUnknown` を定義する。これにより
//! vtable（`IShiori_Vtbl`）・`windows_core::Interface` 実装・IID・実装用トレイト
//! `IShiori_Impl` が生成される。メソッドは生 `unsafe fn -> HRESULT` 形を保ち、
//! 自動エルゴノミクス（`Result` 返し）は付かない（design.md §ABI Layer → IShiori,
//! 要件 1.1/1.2/1.3/1.4）。Rust 風の安全 API（`Result<RequestOutcome, ShioriError>`）は
//! 別レイヤ（`ergonomic.rs`・本層では未実装）が提供する（D4）。
//!
//! ## 役割と境界（design.md §ABI Layer → IShiori）
//! - load/unload/request の 3 操作を raw COM vtable として公開し、実装種別
//!   （native / 過去互換）の分岐を面に持たない（要件 1.4）。
//! - 全文字列引数・戻り値は `HSTRING`（UTF-16）。プロセス内取り回しは WinRT 非依存・
//!   `RoInitialize` 不要（要件 4.1）。in-proc 直 vtable 呼び出しのため OOP 自動
//!   マーシャリングは発生しない（要件 4.3）。content は不透明 HSTRING であり、
//!   正準 json-rpc の採用は上位の設計判断で本層ではパースしない（要件 2.1/3.1）。
//! - 未ロード時の request 拒否（要件 2.4）等の状態は実装側（脳）が保持する。ABI は
//!   状態フィールドを持たず、拒否を HRESULT 契約として定める。
//!
//! ## HSTRING 所有権規約（議題1・design.md §IShiori Pre/Postconditions・要件 5.4）
//! 本規約は流動契約（D7）下でも初版で固定する**不変条件**である。
//! - `[in]`（`*const HSTRING`）= **借用**。callee は呼び出し中の読み取りのみ可で、
//!   解放してはならない。呼び出し後に内容を保持する場合は callee 側で clone する。
//! - `[out]`（`*mut HSTRING`）= **callee（脳実装）が確保し caller（ergonomic 層）が
//!   解放**する move-out セマンティクス。callee は成功（`S_OK`）時に out-param へ
//!   HSTRING を書き込み、その所有権を caller へ移す。caller は Drop で解放する
//!   （windows-core 標準 move セマンティクス）。
//!
//! ## Request の応答契約（議題1/議題3・design.md §IShiori, 要件 3.1〜3.5/2.4）
//! `Request` は**同期呼び出し**で、結果を HRESULT で 3 分岐する:
//! - 即時応答: `S_OK` を返し `out_response` に応答 HSTRING を move-out（`out_token` は未使用）。
//! - 遅延: [`crate::error::SHIORI_S_PENDING`]（**成功**コード）を返し `out_token` に
//!   有効な相関トークンを書き込む（`out_response` は空＝未書き込みで解放対象なし）。
//!   後続の完了応答は `IShioriHost::Complete`（task 2.2）で配送する。
//! - 失敗: error HRESULT（例: 未ロード時は [`crate::error::SHIORI_E_NOT_LOADED`]・要件 2.4）を
//!   返す。失敗時はいずれの out-param も未書き込み。
//!
//! ## 非ブロッキング/単一 in-flight 前提（議題3・design.md §State Management）
//! `Load`/`Unload`/`Request` は**即時に戻る**こと（非ブロッキング）。重い処理は
//! `SHIORI_S_PENDING` ＋後続 `Complete` で後送りする。in-proc 直 vtable は areka の
//! 呼び出しスレッド上で実行されるためブロックは本体凍結を招き安全に打ち切れない。
//! areka は同時に高々 1 リクエストのみ発行する（`Deferred` 中は対応する `Complete`
//! 受領／取消まで次の `Request` を出さない）。脳は逐次 request 前提でよい。

// COM 規約に従い PascalCase メソッド名（`Load`/`Unload`/`Request`）を用いる。
// `#[interface]` は trait への非 doc 属性を拒否し、生成 vtable/実装トレイト面にも
// PascalCase が伝播するため、本 ABI モジュール全体で non_snake_case を許可する
// （design.md のメソッド面・引数順を厳密に保つための意図的な許可）。
#![allow(non_snake_case)]

use windows_core::{HRESULT, HSTRING, IUnknown, IUnknown_Vtbl, interface};

/// 脳側が実装する唯一の内部 COM 境界（design.md §ABI Layer → IShiori, 要件 1.1/1.2/1.3/1.4）。
///
/// IID（dev 用・流動契約 D7 によりリリースまで変動可。**リリース時に凍結**する）:
/// `E7887AB4-525D-4520-9474-577528758C79`（本タスクで v4 採番）。
///
/// 詳細な契約（HSTRING 所有権規約・Request の 3 分岐・非ブロッキング/単一 in-flight 前提）は
/// モジュールレベル doc を参照。
#[interface("E7887AB4-525D-4520-9474-577528758C79")]
pub unsafe trait IShiori: IUnknown {
    /// 脳をロードし、areka 本体が実装する sink を渡す機会（要件 2.1/2.3, 6.2）。
    ///
    /// `host` は `IShioriHost`（task 2.2 で定義）の raw COM ポインタを表す。本タスクでは
    /// `IShioriHost` 型が未定義のため `*mut core::ffi::c_void` で受ける。task 2.2 後に
    /// `*mut core::ffi::c_void`（vtable 互換のためレイアウトは不変）のまま、上位レイヤで
    /// 型付きラッパ越しに `IShioriHost` として解釈する設計余地を残す（design.md §IShiori
    /// Dependencies）。脳は受け取った host を AddRef して `Unload` まで保持してよい（議題2）。
    ///
    /// # 戻り値
    /// 成功 = `S_OK` / 失敗 = error HRESULT（要件 2.3）。
    ///
    /// # Safety
    /// `host` は呼び出し側（areka）が所有する有効な `IShioriHost` raw ポインタであること。
    unsafe fn Load(&self, host: *mut core::ffi::c_void) -> HRESULT;

    /// 脳をアンロードし、終了処理の機会を与える（要件 2.2）。
    ///
    /// 即時に戻ること（非ブロッキング）。`Unload` 後、脳は保持していた host への
    /// `Raise`/`Complete` を呼んではならない（議題2）。
    ///
    /// # 戻り値
    /// 成功 = `S_OK` / 失敗 = error HRESULT。
    unsafe fn Unload(&self) -> HRESULT;

    /// 同期 request。即時/遅延/失敗を HRESULT で 3 分岐する（要件 3.1〜3.5, 2.4）。
    ///
    /// - `input`: `[in]` 正準 content（json-rpc 相当の不透明 HSTRING）。**借用**——
    ///   callee は読み取りのみで解放しない（所有権規約・モジュール doc 参照）。
    /// - `out_response`: `[out]` 即時応答（`S_OK` 時のみ有効）。callee が確保し
    ///   caller が解放する move-out（所有権規約・モジュール doc 参照）。
    /// - `out_token`: `[out]` 相関トークン（[`crate::error::SHIORI_S_PENDING`] 時のみ有効）。
    ///
    /// # 戻り値
    /// - `S_OK`: 即時応答（`out_response` に move-out、`out_token` 未使用）。
    /// - [`crate::error::SHIORI_S_PENDING`]: 遅延（`out_token` に有効トークン、`out_response` 空）。
    /// - error HRESULT: 失敗（例: 未ロードは [`crate::error::SHIORI_E_NOT_LOADED`]・要件 2.4）。
    ///   失敗時はいずれの out-param も未書き込み。
    ///
    /// 非ブロッキング: 重い処理は遅延応答へ回し即時に戻ること（議題3・モジュール doc 参照）。
    ///
    /// # Safety
    /// `input` は呼び出し中有効な `*const HSTRING`。`out_response`/`out_token` は
    /// caller が確保した書き込み可能な領域を指すこと。
    unsafe fn Request(
        &self,
        input: *const HSTRING,
        out_response: *mut HSTRING,
        out_token: *mut u64,
    ) -> HRESULT;
}

#[cfg(test)]
mod tests {
    //! 最小モック実装による vtable 健全性の単体証明。
    //!
    //! `#[implement(IShiori)]` の小さな struct を立て、COM ポインタ（`IShiori`）経由で
    //! メソッドを呼び、生成された vtable が機能することを 1 件検証する。HSTRING 往復・
    //! Drop 計測等の本格的な結合検証は task 5.1 のハーネスで別途行う（本テストは最小に留める）。

    use super::*;
    use windows_core::implement;

    /// vtable 健全性検証用の最小モック脳。
    ///
    /// `Unload`/`Request` が呼ばれたことを記録し、`Request` は即時応答（`S_OK`）として
    /// `out_response` に固定 HSTRING を move-out する。
    #[implement(IShiori)]
    struct MockBrain {
        unloaded: std::cell::Cell<bool>,
    }

    impl IShiori_Impl for MockBrain_Impl {
        unsafe fn Load(&self, _host: *mut core::ffi::c_void) -> HRESULT {
            HRESULT(0) // S_OK
        }

        unsafe fn Unload(&self) -> HRESULT {
            self.unloaded.set(true);
            HRESULT(0) // S_OK
        }

        unsafe fn Request(
            &self,
            _input: *const HSTRING,
            out_response: *mut HSTRING,
            _out_token: *mut u64,
        ) -> HRESULT {
            // 即時応答: callee が確保した HSTRING を out-param へ move-out（所有権規約）。
            unsafe { core::ptr::write(out_response, HSTRING::from("pong")) };
            HRESULT(0) // S_OK
        }
    }

    /// `IShiori` が IID 付きでコンパイルし、COM ポインタ経由の vtable 呼び出しが機能すること。
    /// （観測: vtable のメソッド面・引数順が design.md と一致し、生 `unsafe fn -> HRESULT`
    /// として呼び出せる。）
    #[test]
    fn ishiori_vtable_dispatch_works() {
        let mock = MockBrain {
            unloaded: std::cell::Cell::new(false),
        };
        // `#[implement]` 生成の `From`/`into()` で COM インターフェイスポインタへ変換。
        let brain: IShiori = mock.into();

        // 即時 Request: vtable 経由で呼び、S_OK と move-out された応答を観測する。
        let input = HSTRING::from("ping");
        let mut out_response = HSTRING::new();
        let mut out_token: u64 = 0;
        let hr = unsafe { brain.Request(&input, &mut out_response, &mut out_token) };
        assert!(hr.is_ok(), "Request は S_OK を返すこと: 0x{:08X}", hr.0);
        assert_eq!(out_response, HSTRING::from("pong"), "move-out された応答を観測できること");

        // Unload: vtable 経由で呼び、副作用（記録）が反映されることを観測する。
        let hr = unsafe { brain.Unload() };
        assert!(hr.is_ok(), "Unload は S_OK を返すこと: 0x{:08X}", hr.0);
    }

    /// `IShiori` の IID が本タスクで採番した v4 GUID に固定されていること（回帰防止）。
    #[test]
    fn ishiori_iid_is_fixed() {
        use windows_core::Interface;
        let expected = windows_core::GUID::from_u128(0xE7887AB4_525D_4520_9474_577528758C79);
        assert_eq!(<IShiori as Interface>::IID, expected, "IID は採番値に固定されていること");
    }
}
