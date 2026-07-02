# Gap Analysis: areka-P0-host32-shiori-load

- **作成日**: 2026-07-02（スコープ拡大後の要件 R1–R13 に対する新規ギャップ分析）
- **入力**: `requirements.md`（確定・R1–R13）／`brief.md`（Confirmed Decisions D1–D7・locked ABI）／`.kiro/steering/`／既存コードベース実査
- **注記**: 旧 research.md（WS-A 単独スコープ時代・§8–12 を含む）は陳腐化により削除済み。本書は **WS-A（host-32 flat-C load 層）と WS-B（IShiori ABI 是正）の両ワークストリーム**を対象とする書き下ろしである。D1–D7 は確定事項として再審しない——本書は「どう実現するか」の分析と選択肢の提示に徹する。

---

## 1. 現状調査（Current State Investigation)

### 1.1 WS-A 資産: host-32 3 クレート

#### `crates/shiori-host32-ipc`（完了・凍結——改変対象外の確認のみ）
- `MsgTag{Hello=1, Load=2, Request=3, Response=4, Unload=5}`・`copydata_payload`（framing 検証）・`ResponseSlot`（再入 RESPONSE）・`send_copydata`・`send_request` を公開（`src/lib.rs:44/124/213/270/325`）。
- **重要**: `send_request` は**タグを引数に取る汎用形**（`parent.send_request(MsgTag::Request, payload, timeout)` の既存呼出で確認）。したがって **R5 の親側「`send_request(MsgTag::Load, &[], timeout)` → 1 byte ack 受領」は凍結 wire の既存 API そのままで成立**し、ipc への変更は一切不要。D5 の実コード裏取りは成立している。

#### `crates/shiori-host32-host`（拡張対象）
- `process_host.rs` の `spawn(helper_exe: &Path, ghostdir: &Path, parent_hwnd: u32)`（125 行）: 現状 `ghostdir` は **`current_dir(ghostdir)` として cwd のみ**で運び、明示 arg に載らない。`parent_hwnd` は「arg1=10進 u32＋env `HOST32_PARENT_HWND`」の二重供給（`PARENT_HWND_ENV` 定数・`spawn_command` テスト seam あり）。
- **R3 とのギャップ**: `load_dir`・SHIORI 名の「明示 arg＋env fallback」が **Missing**。cwd=load_dir（D2'）は既存挙動を**維持**すればよい。
- **破壊的変更の波及先（実測）**: `tests/echo_roundtrip.rs:132`・`tests/error_paths.rs:300` の 2 箇所が `spawn(&helper_exe, &ghostdir, parent_hwnd)` を直呼び。シグネチャ変更はこの 2 テストの吸収で完結する（他に呼出なし）。
- `parent_window.rs` に `ParentMessageWindow::create / pump_until_hello_or / send_request` が既在——LOAD E2E の親側足場はほぼ流用可能。

#### `crates/shiori-host32-helper`（拡張対象）
- `main.rs`: `respond()` echo stub（54–56 行・「下流の差し替え点」と明記済み）。`classify_inbound()` は `Load` を `InboundAction::IgnoreKnown` に分類（現状「既知だが無視」・67–72 行）——**R4.1 のトリガ結線が Missing**。
- `HelperShared` は `parent_hwnd: u32`＋`Cell<u64>` 観測カウンタのみ。DLL プロキシ保持スロットが **Missing**（`ShioriByteProxy` は非 `Copy` ゆえ `Cell` 不可→ `RefCell<Option<Proxy>>` が必要。single UI thread 前提なので `RefCell` で足りる）。
- 起動パラメーター取得は `parent_hwnd_from_env()`（arg1 優先→env fallback）のみ。`load_dir`／SHIORI 名の取得が **Missing**（同型のパターンを増設すればよい）。
- `Cargo.toml`: windows features は `Win32_System_DataExchange / Win32_UI_WindowsAndMessaging / Win32_Foundation` のみ。**`Win32_System_LibraryLoader`（LoadLibraryW/GetProcAddress）・`Win32_System_Memory`（GlobalAlloc/GlobalFree）・`Win32_Globalization`（WideCharToMultiByte/CP_ACP）が Missing**（workspace 側 features に Memory/Globalization は無いが、member 側 features は additive なので helper ローカル追記で足りる）。

#### 参照専用: `crates/pilot/examples/shiori-host-32/shiori_proxy.rs`（コピペ禁止・知見のみ）
先進坑で go 済みの一次記録。再実装時に踏襲すべき確定知見（すべて pasta 実ソースで裏取り済みと記録）:
- flat-C 署名: `load(hdir: HGLOBAL, len: usize) -> bool`／`unload() -> bool`／`request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`（cdecl・戻り値は **Rust `bool` 1 byte**・`request` len は in/out）。
- HGLOBAL 規約: `GlobalAlloc(GMEM_FIXED=0)` でハンドル＝先頭ポインタ・**load/request の入力 HGLOBAL は DLL(callee) が解放**（ホスト側解放＝二重解放）・request 応答はホスト解放（本仕様では request 非呼出）。
- charset 非対称: `load` の dir は **ANSI(CP_ACP)**（`WideCharToMultiByte`）・request は UTF-8（下流）。
- `GetProcAddress` 結果の fn ポインタ `transmute`・`Drop` で `FreeLibrary`・失敗を enum（`LoadLibraryFailed`/`EntryNotFound`/`LoadFailed`…）で観測可能に返す構造。
- **i686 落とし穴の記録**: pilot の ipc.rs は `usize >> 32` overflow で i686 の `cargo test` がビルド不能だった——R13.3（dwData 演算は u64 幅）の実証的根拠。production ipc クレートは対処済み。

#### fixture crate（新設対象）
`shiori-host32-testdll` は**存在しない**（crates/ 配下実査: areka / areka-parsers / dola / pilot / shiori-abi / shiori-host32-{helper,host,ipc} / wintf）。**Missing（新設）**。

#### `vendors/pasta` submodule
`git submodule status` → `-048d646c...`（**未展開**）。ワークスペース `Cargo.toml` に `[patch.crates-io] pasta_core = { path = "vendors/pasta/crates/pasta_core" }` があるため、**未展開のままでは workspace の cargo 解決自体が壊れる**。実装前の `git submodule update --init` は R13.5（署名バイト正確確認）に加えて**ビルド前提**でもある（Constraint・最優先の準備タスク候補）。

### 1.2 WS-B 資産: shiori-abi と consumer 波及面

#### `crates/shiori-abi`（是正対象）
- `interface.rs`: `#[interface(IID)] unsafe trait IShiori: IUnknown`——`unsafe fn Load(&self, host: *mut c_void)`（79 行・**load_dir 欠落**）／`unsafe fn Unload(&self)`（88 行・RAII 冗長）／`unsafe fn Request(&self, input: *const HSTRING, out_response: *mut HSTRING, out_token: *mut u64)`（109 行・raw out-param）。`IShioriHost` は `Raise`/`Complete`（raw ポインタ・`GetProperty`/`SetProperty` は **Missing**）。IID は dev-stage 流動（再採番可と doc 明記済み）。
- `ergonomic.rs`: `ShioriExt`（`load(host)`/`unload`/`request`）が **vtable 直呼び**（`(Interface::vtable(self).Load)(self.as_raw(), ..)`）で raw 層へ到達し `Result` 化。HRESULT 3 分岐（S_OK→`Immediate`／`SHIORI_S_PENDING`→`Deferred`／失敗→`ShioriError`）のマッピングロジックは新 `Get` にほぼ流用可能。
- `error.rs`: `SHIORI_S_PENDING`（成功・0x20A1_0001）／`SHIORI_E_NOT_LOADED`／`SHIORI_E_UNKNOWN_TOKEN`＋`ShioriError`。**create 融合後は「未ロード状態の IShiori」が契約上存在しなくなる**ため `SHIORI_E_NOT_LOADED`／`ShioriError::NotLoaded`／`LoadFailed` の存廃・改名（例: `CreateFailed`）が波及点。
- `outcome.rs`: `RequestOutcome{Immediate,Deferred}`・`CorrelationToken`・`CorrelationTokenAllocator`——遅延応答機構は新 ABI でもそのまま生きる（brief 開放質問: `GetOutcome` 改名検討のみ）。
- `tests/mock_brain_roundtrip.rs`（260 行）: `ShioriExt::request` 経由の HSTRING 無マーシャリング往復・alloc/drop 計測。新 ABI へ書換え対象。

#### `crates/areka` consumer（波及更新対象・実測マップ）
| ファイル | 現状 | 新 ABI での主な変化 |
|---|---|---|
| `reference_brain.rs` | `#[implement(IShiori)] ReferenceBrain`＋`IShiori_Impl for ReferenceBrain_Impl`（Load で host AddRef 保持・`loaded: AtomicBool`・未ロード拒否）＋ C 入口 `shiori_create`（214 行・`extern "system"`＋`#[unsafe(no_mangle)]`） | `Get`/`Notify` 実装へ痩身（Load/Unload 消滅・「未ロード状態」自体が消え `loaded` フラグ不要化）。**reference `IShioriFactory` 実装＋`shiori_factory` 入口を新設**（`shiori_create` は残置しない）。host 保持は construction 時に確定 |
| `shiori_session.rs` | `ShioriSession::activate(brain: IShiori)` が `ShioriExt::load(host)` を呼ぶ／`unload()` メソッドが保留取消＋`Unload` | **factory 経由の生成へ移行**（`activate` は `IShioriFactory::create(load_dir, shiori_name, host)` 相当を受ける形へ）。`unload()` → **Drop teardown**（保留取消は `Drop` impl へ）。単一 in-flight・タイムアウト・`poll_completions` の規律はそのまま生存 |
| `shiori_host.rs` | `ShioriHostSink`（`#[implement(IShioriHost)]`・突合枠 `Mutex<Option<Token>>`＋mailbox `Mutex<VecDeque>`・**投函して即返す**遅延モデル） | `GetProperty`/`SetProperty` 追加。**R10.3 の同期応答は mailbox 投函モデルでは満たせない**→ sink 内プロパティストア等の同期経路が必要（→ §6 設計判断 (b)） |
| `shiori_demo.rs` | `shiori_create`→`ShioriSession::activate`→即時/遅延/Raise/unload をデモ駆動 | factory 生成・Get/Notify・Drop teardown へ追随 |
| `shiori_e2e_tests.rs`／`shiori_lifecycle_e2e_tests.rs`／`shiori_reference_e2e_tests.rs`／各モジュール内 `#[cfg(test)]` | vtable 直呼びヘルパ（`call_load`/`call_request`/`call_raise`/`call_complete`）多数 | 新 ABI へ機械的追随。**面が型付き＋メソッド可視化されれば vtable 直呼びヘルパ群は大幅に不要化できる**（§2 の可視性知見） |

#### 依存グラフ上の独立性（WS-A ⟷ WS-B）
`shiori-host32-*` は `shiori-abi` に依存せず、`shiori-abi`／`areka` は `shiori-host32-*` に依存しない（Cargo.toml 実査）。**両 WS はコード上完全に独立**であり、結合点は原則（D1/D7）と本仕様ドキュメントのみ（→ §6 設計判断 (e)）。

### 1.3 既存の規約・慣行（踏襲すべきパターン）
- **設定供給**: 「arg 優先・env fallback・env キーは `pub const` で契約固定」（`PARENT_HWND_ENV` パターン）＋ `spawn_command` の下位 seam でプロセス非依存テスト。
- **テスト**: 純関数切り出し（`classify_inbound`/`ExitKind::classify`）→単体、loopback（1 窓制約に集約）→窓結線、実プロセス E2E（`resolve_helper_exe`: env 優先→target-dir 探索→**見つからなければ明確 panic で fail・無言スキップ禁止**）の三層。R7.5 の「silent skip 禁止」は既存慣行と一致。
- **COM 実装**: `#[implement(X)]`＋`X_Impl for T_Impl`、`AsImpl` ダウンキャスト、raw メソッド private ゆえ他クレートから vtable 直呼び——この「直呼びハック」は新 ABI で解消余地あり（§2）。
- **ビルド**: i686 は PowerShell 必須（steering/memory 記録・Git Bash link.exe トラップ）。`cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` → x64 側テスト、の 2 段が echo_roundtrip の確立手順。

---

## 2. 技術調査（KEY）: windows-core 0.62.2 `#[interface]` の表現力——実装可否の証拠ベース判定

R11／brief 開放質問 1 の核心。**windows-interface 0.59.3（windows-core 0.62.2 が使用するマクロ実体）のソースを直接実査**した結果（`c:\rust\cargo\registry\src\...\windows-interface-0.59.3\src\lib.rs`）:

### 2.1 判定表

| 表現したい形 | マクロ上の可否 | 証拠（マクロソース） |
|---|---|---|
| 非 `unsafe` trait 宣言 | **不可** | parse が `input.parse::<syn::Token![unsafe]>()?` を必須要求（461 行）——`unsafe trait` 固定 |
| 非 `unsafe` メソッド（呼出面・`_Impl` 実装面） | **不可** | 生成コードが両面とも `unsafe fn` をハードコード（呼出ラッパ 141/148 行・`_Impl` trait 175 行）。宣言側で `unsafe` を書かなくても生成面は `unsafe fn` になる |
| `Ref<'_, T>`（interface 借用引数） | **可** | 引数型の最終セグメント名 `Ref`/`OutRef` を特別検出（`borrow_type()` 744–761 行）。vtable には宣言型のまま載る（`Ref` は `#[repr(transparent)]` で ABI=raw ポインタ・windows-core `ref.rs:5-6`）。呼出面は `P0: Param<T>` ジェネリックになり **`&IShioriHost` をそのまま渡せる**。実装面は `Ref<'_,T>` を受け、`ok()/as_ref()/cloned()` の**安全メソッド**で扱える |
| `OutRef<'_, T>`（interface out 引数） | **可** | 同上。実装面は安全な `OutRef::write(value) -> Result<()>`（windows-core `out_ref.rs:16`）で move-out できる。呼出面は `OutParam<T>` ジェネリック |
| `&HSTRING`（文字列 in 引数） | **可** | 非 Ref/OutRef 型は素通し。`&HSTRING` は HSTRING（ポインタサイズ・repr(transparent)）への参照＝ABI 上 `*const HSTRING` と同一。**実装面で raw deref の `unsafe` ブロックが不要になる** |
| `&mut HSTRING`／`&mut u64`（値 out 引数） | **可** | 同上（ABI= `*mut`）。callee は `*out = value` の安全代入で move-out できる |
| `-> Result<()>` | **可** | `is_result()` 検出時、vtable slot は `-> HRESULT` に落ち、実装 thunk は `.into()`（`From<Result<T>> for HRESULT`・windows-result `hresult.rs:147`）、呼出ラッパは `.ok()` |
| `-> Result<T>`（T≠()、例 `Result<IShiori>`/`Result<HSTRING>`） | **不可（コンパイルエラー）** | 呼出ラッパが `(vtable...)(..).ok()` を生成するが `HRESULT::ok()` の戻りは **`Result<()>` 固定**（windows-result `hresult.rs:34`）→ 宣言戻り値 `Result<T>` と型不一致。**値を返す Result 直返しは vtable 面では表現できない** |
| `Result<RequestOutcome>`（Rust enum 直返し） | **不可** | 上記に加え、`RequestOutcome` は ABI 表現を持たない Rust enum。vtable を渡れない |
| 3 値 HRESULT 分岐（S_OK／`SHIORI_S_PENDING`／失敗） | **`Result<()>` では不可・`-> HRESULT` なら可** | `.ok()` は**全成功コードを `Ok(())` へ潰す**ため、成功 2 値（即時/遅延）の判別が消える。`Get` の vtable 面は `-> HRESULT` 生返しを維持する必要がある |

### 2.2 帰結: 「二層構造」は新 ABI でも必然（ただし unsafe の質が変わる）

1. **vtable 面（`#[interface]`）**: `unsafe trait`＋`unsafe fn` は除去不能。ただし引数を `Ref<T>`/`OutRef<T>`/`&HSTRING`/`&mut HSTRING` へ型付けすることで、**実装体・呼出体の中身から raw ポインタ操作（deref・`core::ptr::write`・`from_raw_borrowed`）がほぼ消える**。「`unsafe` は署名に残るが空洞化する」のが到達可能な最良点。これは R11.3 の「最小 unsafe の局所化・契約面は型付き署名」の想定どおりであり、**R11 は fallback 条項込みで実現可能**（zero-unsafe の硬性要求でないことが要件側で既に手当て済み）。
2. **安全面（ergonomic 相当の薄いラッパ）**: `Result<IShiori>` 直返し（R8/`create`）・`Result<RequestOutcome>`（R9/`Get`）・`Result<HSTRING>`（R10/`GetProperty`）は**この層が提供する**。既存 `ergonomic.rs` の HRESULT→`RequestOutcome` マッピングはそのまま流用できる。→ **R12.5（ShioriExt 存廃）への含意: 「拡張トレイト」という形は再検討対象でも、「安全面レイヤ」という機能は消せない**。配置の選択肢は §4.2-C。
3. **副次的な改善機会（可視性）**: 現行 interface はメソッド可視性が private のため、consumer は `(Interface::vtable(x).Complete)(x.as_raw(), ..)` の直呼びハックを多用している（reference_brain/shiori_host/shiori_session の計 10 箇所超）。新定義でメソッドに `pub` を付ければ生成呼出ラッパが公開され、**直呼びハックを全廃できる**（マクロは `#vis` をラッパへ伝播・141/148 行）。
4. **spike の要否**: 上記はマクロソースから静的に証明済み。残る実証は「`Ref<IShioriHost>`＋`OutRef<IShiori>`＋`&HSTRING`＋`Result<()>`／`-> HRESULT` 混在の interface が `#[implement]` と組んで vtable dispatch まで通る」1 点のみで、**本仕様タスク内の最初の ABI スケルトンタスク（既存 interface.rs の vtable 健全性テストと同型）で吸収できる規模**。独立 spike spec は不要と判断する（→ §6 設計判断 (a)）。

### 2.3 locked ABI（brief D6）の vtable 面への具体的な落とし込み案

brief の署名は「契約面（利用者が見る形）」であり、vtable 面は次の分解が必要（各要素は §2.1 で可否確認済み）:

```text
[C 入口・唯一の raw 例外（R11.2）]
#[unsafe(no_mangle)] pub unsafe extern "system"
fn shiori_factory(out: *mut *mut c_void) -> HRESULT          // 既存 shiori_create:214 と同パターン

[vtable 面（#[interface]・unsafe は署名のみ・中身は型付き）]
IShioriFactory::CreateInstance(load_dir: &HSTRING, shiori_name: &HSTRING,
                               host: Ref<'_, IShioriHost>,
                               out: OutRef<'_, IShiori>) -> Result<()>   // 成功1値なので Result<()> 可
IShiori::Get(input: &HSTRING, out_response: &mut HSTRING,
             out_token: &mut u64) -> HRESULT                 // 成功2値（S_OK/PENDING）ゆえ HRESULT 生返し必須
IShiori::Notify(input: &HSTRING) -> Result<()>               // 片道・成功1値
IShioriHost::Raise(script: &HSTRING) -> Result<()>
IShioriHost::Complete(token: u64, response: &HSTRING) -> Result<()>     // UNKNOWN_TOKEN は Err 側
IShioriHost::GetProperty(key: &HSTRING, out_value: &mut HSTRING) -> Result<()>  // 欠落 key の扱いは design
IShioriHost::SetProperty(key: &HSTRING, value: &HSTRING) -> Result<()>

[安全面（薄いラッパ・配置は §4.2-C の選択肢）]
create(load_dir, shiori_name, &host) -> Result<IShiori>      // OutRef 受け皿を隠蔽
get(input) -> Result<RequestOutcome>                          // HRESULT 3分岐 → enum 復元（既存 ergonomic 流用）
get_property(key) -> Result<HSTRING>
```

---

## 3. 要件 → 資産マップ（ギャップタグ付き）

| 要件 | 既存資産 | ギャップ | タグ |
|---|---|---|---|
| R1 load_dir 貫通 | spawn の cwd 供給（暗黙）・IShiori には皆無 | 明示 arg 化（WS-A）・create 引数化（WS-B）・欠落時の決定的失敗 | **Missing** |
| R2 Drop 一貫 | pilot `ShioriEntries::Drop`（FreeLibrary のみ・unload 呼出なし）／`ShioriSession::unload` 明示メソッド | proxy Drop への courtesy unload 追加・`Unload`/`unload()` の削除→Drop 移行 | **Missing** |
| R3 起動パラメーター | `PARENT_HWND_ENV` パターン・`spawn_command` seam | load_dir/SHIORI 名の arg＋env・helper 側取得・欠落時失敗 | **Missing**（パターン既在で低リスク） |
| R4 LOAD トリガ＋DLL ロード | `classify_inbound`（Load=IgnoreKnown）・pilot proxy 知見一式 | `InboundAction` 新 variant・proxy 本実装・helper への常設保持（`RefCell`）・windows features 3 種追加 | **Missing** |
| R5 load-ack | `send_request` タグ汎用・`ResponseSlot` 再入経路（凍結のまま使える） | helper 側「Load 受領→load 実行→Response[1]/[0] 返送」の結線のみ | **Missing**（wire 変更ゼロを確認済み） |
| R6 失敗パス | pilot `ProxyError` 分類の設計知見 | 各失敗態様→ack[0]・プロセス生存維持の実装＋E2E | **Missing** |
| R7 fixture＋E2E | `resolve_helper_exe` 探索・panic-not-skip 慣行・`ParentMessageWindow` | testdll crate 新設・E2E 本体・`HOST32_PASTA_DLL` gate | **Missing** |
| R8 IShioriFactory＋entry | `shiori_create` の C 入口パターン・`#[interface]`/`#[implement]` 技法 | factory interface・新 IID・`shiori_factory`・create 融合 | **Missing** |
| R9 Get/Notify 痩身 | `Request` 3 分岐実装・`RequestOutcome` | Get/Notify 分離・Load/Unload 削除・Notify の新設 | **Missing** |
| R10 プロパティ | `ShioriHostSink`（mailbox 投函モデル） | GetProperty/SetProperty 新設・**同期応答経路**（mailbox では満たせない） | **Missing＋Unknown**（詳細 semantics は design 送り・R10.5 で明示済み） |
| R11 型付き安全化 | §2 実査結果 | vtable 面の型付き化＋安全面ラッパ。非 unsafe 署名は**マクロ制約により不可**→ R11.3 fallback 適用 | **Constraint**（実現形が確定・§2） |
| R12 consumer 波及 | §1.2 の波及マップ（8 ファイル＋abi 内 3 ファイル） | 全面追随（機械的だが量がある）・ABI 証明 = reference/mock | **Missing**（範囲は特定済み） |
| R13 開発制約 | PowerShell 規律・u64 演算（ipc 対処済）・葉ノード隔離・`resolve_helper_exe` 慣行 | `vendors/pasta` submodule **未展開**（cargo 解決の前提でもある） | **Constraint** |

---

## 4. 実装アプローチの選択肢

### 4.1 WS-A（host-32 flat-C load 層）

#### A-1: spawn 契約の拡張形（→ §6 設計判断 (c)）
- **案 a（位置引数拡張）**: `spawn(helper_exe, load_dir, shiori_name, parent_hwnd)`——arg1=parent_hwnd（現行 helper の読み取り互換）・arg2=load_dir・arg3=shiori_name＋env 3 種＋cwd=load_dir。既存 2 呼出の吸収は機械的。最小差分。
- **案 b（構成 struct）**: `SpawnConfig { helper_exe, load_dir, shiori_name, parent_hwnd }` を導入し `spawn(&SpawnConfig)`。将来の起動パラメーター増加（下流 lifecycle 等）に強いが、本仕様単独では YAGNI 気味。
- どちらでも `PARENT_HWND_ENV` と同格の `pub const`（例 `LOAD_DIR_ENV = "HOST32_LOAD_DIR"` / `SHIORI_NAME_ENV = "HOST32_SHIORI_NAME"`）で契約固定し、`spawn_command` seam・「arg 優先 env fallback」の既存慣行を踏襲する。

#### A-2: helper 側の結線（Option A: 既存構造への拡張が自然）
- `classify_inbound` に `Ok((MsgTag::Load, _)) => InboundAction::TriggerLoad`（ペイロード無視・R4.1）を追加——純関数のまま単体検証可能という既存設計をそのまま活かす。
- `HelperShared` に `proxy: RefCell<Option<ShioriByteProxy>>`＋起動パラメーター（load_dir/shiori_name）を追加。WndProc 内で Load 受領→`ShioriByteProxy` 確立→`load()` 同期呼出→`send_copydata(parent, Response, &[1|0])`。
- **留意（設計で明示すべき）**: `load()` は親の `SendMessageTimeoutW` ハンドラ内で同期実行される。DLL の load 所要時間は親側 timeout（呼出ごとに指定可・凍結 API）で吸収する——「timeout は Load 用に長めを許容する」ことを E2E とドキュメントで固定するのが安全。`LoadLibraryW` を WndProc 内で呼ぶこと自体は問題ない（DllMain 内ではないため loader lock 衝突なし）。
- `shiori_proxy` は helper クレート内の新モジュールとする（案: `crates/shiori-host32-helper/src/shiori_proxy.rs`）。下流 request/lifecycle も同 proxy を使うため、将来クレート分離の余地はあるが、現時点では helper 内モジュールが最小（Option A）。

#### A-3: fixture crate `shiori-host32-testdll`（Option B: 新設一択・D4 確定）
- `crate-type=["cdylib"]`・`[lib] name="shiori"`→ 出力 `shiori.dll`。flat-C 3 エクスポート（`#[unsafe(no_mangle)] pub extern "C"`）。`load` は受領 HGLOBAL を `GlobalFree`（callee 解放規約の忠実な再現＝ホスト側二重解放バグの検出器を兼ねる）。
- 失敗強制: env（例 `HOST32_TESTDLL_LOAD_FAIL=1`）で `load`→`false`。**E2E は spawn 前に親プロセスで env を set→子が継承**する形で決定的に制御できる（`Command::env` でも可）。
- `request` は本仕様では呼ばれないがエクスポート解決対象（R4.2）なので、最小 stub（null 返し等）で実装する。
- 依存は `windows`（Foundation/Memory）のみ・`crates/pilot` 非依存（R7.7）。workspace members のワイルドカードで自動参加するが、x64 ビルドでも無害（使われない x64 dll ができるだけ）。
- **成果物解決（→ §6 設計判断 (d)）**: `resolve_helper_exe` と同型の「env `HOST32_TESTDLL_DLL`（仮）優先 → `target/i686-pc-windows-msvc/{debug,release}/shiori.dll` 探索 → 見つからなければ明確 panic」。E2E の load_dir は (i) 一時 dir へ dll をコピーして `load_dir\shiori.dll` を成立させる案（cwd=load_dir の伺か慣習も同時に検証できる・推奨寄り）と、(ii) target dir を直接 load_dir に使う案（コピー不要だが load_dir の意味が薄まる）がある。

#### A-4: E2E テストの置き場
`crates/shiori-host32-host/tests/shiori_load_e2e.rs`（仮名）として echo_roundtrip と並置——親窓・spawn・pump の既存部品を最大限流用（Option A）。検証系列: 成功 ack[1]／env 強制失敗 ack[0]／dll 不在 ack[0]／エクスポート欠落 dll（testdll の feature か別名ビルドで用意するか、もしくは「不在」で代表させるか——欠落エクスポートの決定的 fixture をどこまで作るかは design で確定）／全ケース後の helper 生存（`poll_exit_kind`→None）。`HOST32_PASTA_DLL` 設定時のみ実 pasta 追験＋不在なら明示 fail（R7.4/7.5）。

### 4.2 WS-B（IShiori ABI 是正）

#### B-1: interface 定義の置き換え（Option A: `shiori-abi` 内での全面書換え）
`interface.rs` を §2.3 の vtable 面で書き換える（旧 `Load`/`Unload`/`Request` は残置しない・IID 3 本再採番）。新規クレート分離の理由はない——`shiori-abi` の「最小依存 ABI クレート」という位置づけは変わらない。

#### B-2: 安全面レイヤの配置（→ §6 設計判断の材料・R12.5）
`Result<IShiori>`/`Result<RequestOutcome>`/`Result<HSTRING>` 直返しは vtable に載らない（§2.1）ため、安全面をどこに置くかの選択肢:
- **案 a（インヘレント第 2 impl）← ✅ 採用（discussion #1・wintf 手法）**: `#[interface]` 生成の `unsafe fn Get(..)` と別名で `impl IShiori { pub fn get(..) -> Result<RequestOutcome> }` を同クレートに追加（複数 inherent impl は合法・PascalCase=vtable / snake_case=安全面で命名衝突なし）。**利用側は `use` 追加なしでメソッドが見える**のが利点。`ShioriExt` トレイトは廃止する。
- **案 b（拡張トレイト存続）**: 現行 `ShioriExt` 型式を `Get`/`Notify`/`create` 対応へ改める。差分最小だが、トレイト import の儀式が残り、安全化後の存在意義が薄い（brief 開放質問 4 の問題意識どおり）。
- **案 c（newtype ラッパ）**: `Shiori(IShiori)` 等で完全に面を分離。安全性の見通しは最良だが COM ポインタとの相互変換の儀式が増え、consumer 差分も最大。
- いずれでも中身は既存 `ergonomic.rs` の HRESULT マッピング流用。**メソッド `pub` 化による vtable 直呼びハック全廃**（§2.2-3）は案に依らず適用可能。

#### B-3: `ShioriHostSink` の GetProperty 同期応答（→ §6 設計判断 (b)）
R10.3「brain が `Get` 処理中に呼んでも**同期**で値応答」の実現案:
- **案 a（sink 内蔵ストア）**: `Mutex<HashMap<String, HSTRING>>`（または `RwLock`）を sink に内蔵し、`GetProperty` はストアから即答・`SetProperty` は即書き。areka 側は事前/随時にストアを更新する。**再入安全性の分析**: brain の `GetProperty` 呼び戻しは areka スレッド上の `Get` 呼出中に同一スレッドで起きるが、areka が `Get` を呼ぶ時点でストアのロックを保持していなければデッドロックしない（「`Get` 呼出中はストアロックを持たない」を規約化すれば成立）。実装は最小・M1 の最小 key 集合と相性が良い。
- **案 b（同期コールバック委譲）**: sink が areka ECS へ問い合わせるコールバックを保持。常に最新値を返せるが、**`Get` 再入中に ECS世界へ同期アクセスする経路は areka の並行モデル（render/window=UI スレッド固定・actor 間 channel）と衝突しやすく**、デッドロック・借用競合の危険が高い。M1 では過剰。
- **案 c（ハイブリッド）**: 内蔵ストア＋「areka が talk 駆動前にスナップショット同期する」規約。案 a の運用形を明文化したもの。
- いずれも key 名前空間（SSP dotted パス）・欠落 key の応答（Err か空 HSTRING か）・スレッド安全性の確定は design 送り（R10.5 で要件側が明示済み）。

#### B-4: セッション層の Drop 移行
`ShioriSession::unload()` を廃し `impl Drop for ShioriSession`（保留取消→brain drop）へ。**留意**: 現行 `unload()` は `Result` を返すが Drop は失敗を返せない——D7「teardown は best-effort・戻り値で扱わない」がこれを正当化する（R2.3 と整合）。テストの「unload 後の拒否」系は「drop 後は参照が存在しない」へ書き換わり、型システムが検証を肩代わりする。

#### B-5: reference factory の形
`#[implement(IShioriFactory)] ReferenceFactory` が `create` で `ReferenceBrain` を構築し host を保持させて返す。native reference は `load_dir`/`shiori_name` を「検証または無視」できる（D6 単一 create）——reference としては**受領値を保持して観測可能にする**（E2E で貫通を証明する材料になる）のが正解見本として有益。`shiori_factory` C 入口は `shiori_create` の実装パターン（E_POINTER 防御・refcount 1 move-out・writes-on-success）をそのまま移植。

#### 採用推奨の方向性（決定は design へ）
両 WS とも **Hybrid（Option C）**: 既存構造への拡張（helper 結線・spawn 拡張・sink 拡充・session 移行）＋新設（testdll crate・IShioriFactory・安全面レイヤ）。全面新設や別クレート化を要する箇所はない。

---

## 5. 工数・リスク見積り

| 単位 | Effort | Risk | 根拠 |
|---|---|---|---|
| WS-A: spawn 拡張＋helper 取得 | S | Low | `PARENT_HWND_ENV` パターン踏襲・呼出 2 箇所の吸収のみ |
| WS-A: proxy＋LOAD 結線＋ack | M | Medium | FFI 本体は pilot 実証済みだが、コピペ禁止での再実装＋WndProc 内同期 load の timeout 設計・二重解放規約の厳守が要注意 |
| WS-A: testdll＋E2E | M | Medium | crate 新設・i686 2 段ビルド・失敗パス網羅。探索/コピー段取りは既存慣行流用で低減 |
| WS-B: interface 書換え＋安全面 | M | Medium | §2 で実装可否は確定済み（Unknown が Constraint に転化）。vtable 健全性テストの型は既在 |
| WS-B: consumer 波及（8＋3 ファイル） | M〜L | Medium | 機械的だが量がある。コンパイラ駆動で漏れは検出可能（同一ワークスペース 1 PR） |
| WS-B: sink プロパティ同期 | S〜M | Medium | 実装は小さいが再入規約の設計品質が本質（§4.2-B3） |
| 前提: submodule 展開＋ABI バイト確認 | S | Low | `git submodule update --init`＋pasta windows.rs 照合（R13.5） |

**全体: M〜L（1〜2 週間相当）／Risk Medium**。最大の不確実性だった windows-core 表現力は本調査で解消済み。残るリスクは (i) GetProperty 再入規約の設計、(ii) 波及更新の物量、(iii) i686 環境規律（PowerShell・2 段ビルド）の運用ミス。

---

## 6. 設計ディスカッションへ挙げる設計判断事項（分析であり決定ではない）

> **✅ 全項目決着（2026-07-02・設計フェーズ）**: (a) は discussion #1 で、(b)〜(g) は設計生成時に §8 の記録どおり確定し、`design.md` に反映済み。

1. **(a) windows-core 実装可否の確定形** — **✅ 決着（2026-07-02 discussion #1）**: **薄い unsafe ラッパ二層を採用（wintf 確立手法）**。vtable 面=`unsafe` PascalCase メソッド（型付き引数 `Ref`/`OutRef`/`&HSTRING` で本体を空洞化）／安全面=**snake_case のインヘレント安全メソッド**（`Get`→`get`・`Notify`→`notify`・`create`／`get_property`／`set_property`）が `Result` 値返しを担う。**`ShioriExt` トレイト形式は廃止**し snake_case インヘレントへ置換。**独立 spike 不要**——最初の ABI スケルトンタスク（既存 vtable dispatch テストと同型）で compile 実証を兼ねる。メソッド `pub` 化による vtable 直呼びハック全廃を**波及範囲に含める**（R12.5 に反映済み）。
2. **(b) GetProperty 同期応答の実現位置**: sink 内蔵プロパティストア（案 a・推奨寄り）vs areka ECS への同期コールバック（案 b・並行モデルと衝突リスク）vs ハイブリッド（案 c）。「`Get` 呼出中に areka がストアロックを保持しない」再入規約の明文化を含む。M1 最小 key 集合の確定も design 論点。
3. **(c) spawn シグネチャの進化形**: 位置引数拡張（案 a・最小差分）vs `SpawnConfig` struct（案 b・将来拡張耐性）。arg 順序（arg1=parent_hwnd 維持か）・env キー名（`HOST32_LOAD_DIR`/`HOST32_SHIORI_NAME`）・`echo_roundtrip.rs`/`error_paths.rs` の吸収方針。
4. **(d) testdll 成果物の E2E 解決**: env override 名＋target-dir 探索順（`resolve_helper_exe` 慣行踏襲）と、load_dir の成立方式（一時 dir へ dll コピー vs target dir 直指し）。「エクスポート欠落 DLL」失敗態様の fixture をどこまで作り込むか（別ビルド variant vs 不在ケースで代表）。
5. **(e) WS-A/WS-B の実装順序**: 依存グラフ上は完全独立（§1.2 末尾）→ **並行タスクグループ化が可能**。共有するのは原則（D1/D7）と `vendors/pasta` 展開タスク（WS-A の署名確認前提・cargo 解決の全体前提）のみ。順序制約は「submodule 展開 → WS-A proxy 実装」だけで、WS-B はいつでも着手できる。
6. **（追記・(b) 派生）error.rs の語彙整理**: create 融合で `SHIORI_E_NOT_LOADED`/`ShioriError::NotLoaded`/`LoadFailed` の意味が消失・変質する。削除／改名（`CreateFailed` 等）／プロパティ系エラー（欠落 key）の追加をまとめて design で確定。
7. **（追記・R9 派生）`RequestOutcome` の改名（`GetOutcome`）と reference brain の `Notify` 意味論**（応答なし片道を reference では何で観測可能にするか——受領記録の公開など）。

---

## 7. Research Needed（design フェーズへ持ち越し）

- **pasta flat-C 署名のバイト正確再確認**（R13.5）: `git submodule update --init` 後に `vendors/pasta/crates/pasta_shiori/src/windows.rs` と pilot 記録の一致を確認して固定（[patch.crates-io] の path 健全化を兼ねる・実装前必須）。
- **プロパティシステムの M1 最小 key 集合**: ukadoc プロパティシステム（`mcp__ukadoc__` で `list_propertysystem` 系を参照）から emo2 が実際に使う dotted パスを特定し、GetProperty の欠落 key 応答を決める。
- **Load 用 timeout の適正値**: WndProc 内同期 `LoadLibraryW`＋`load()` の所要（testdll は数 ms・pasta 実 DLL は actor 起動込み）を計測し、E2E と親側呼出規約の timeout を確定。
- **`#[implement]` と新 interface 群の結線最終確認**: §2 の静的証明を最初の ABI タスクでコンパイル実証（独立 spike にしない前提の確認事項）。

---

## 8. 設計フェーズ Discovery 補強と設計判断の確定（2026-07-02・design 生成時）

### 8.1 Light Discovery（Extension 型）— 実コード再検証の結果

§1 の全事実をソース直読で再確認した（差異なし）:

- `process_host.rs`: `spawn(helper_exe, ghostdir, parent_hwnd)` は `ghostdir` を `current_dir` のみで運ぶ。`PARENT_HWND_ENV`＝「arg1 優先・env fallback・10進 u32」契約と `spawn_command` seam を確認。
- `helper main.rs`: `classify_inbound` が `Load` を `InboundAction::IgnoreKnown` に分類（67–85 行）。`HelperShared` は `parent_hwnd`＋`Cell<u64>` カウンタのみ。`REPLY_TIMEOUT=5s`。`parent_hwnd_from_env()`（arg1→env）。helper `Cargo.toml` の windows features は 3 種のみ（LibraryLoader/Memory/Globalization が不足）。
- `shiori-abi`: `interface.rs` の raw 署名（`Load(*mut c_void)`/`Unload`/`Request(*const,*mut,*mut)`）、`ergonomic.rs` の `ShioriExt` vtable 直呼び＋HRESULT 3 分岐マッピング、`error.rs` の `SHIORI_S_PENDING(0x20A1_0001)`/`SHIORI_E_NOT_LOADED(0xA0A1_0002)`/`SHIORI_E_UNKNOWN_TOKEN(0xA0A1_0003)`＋FACILITY 0xA1・customer bit 規約、`outcome.rs` の `RequestOutcome`/`CorrelationToken`/`CorrelationTokenAllocator`。
- `areka` consumer: `reference_brain.rs`（`loaded: AtomicBool`・`held_host: RefCell<Option<IShioriHost>>`・`shiori_create` C 入口）、`shiori_session.rs`/`shiori_host.rs`/e2e 群の `call_raise`/`call_complete` vtable 直呼びハック（grep 実測で多数）。
- workspace `Cargo.toml`: members は `crates/*` ワイルドカード → testdll crate は配置のみで自動参加。`[patch.crates-io] pasta_core` 未展開問題も再確認。

### 8.2 §6 開放判断の確定（design.md へ反映済み）

| 項目 | 決定 | 根拠 |
|---|---|---|
| (a) 二層構造 | **✅ 済（discussion #1）**: 薄い unsafe ラッパ二層・snake_case インヘレント安全面・`ShioriExt` 廃止・メソッド `pub` 化 | wintf 確立手法・§2 静的証明 |
| (b) GetProperty 同期応答 | **案 a（sink 内蔵ストア）採用**: `Mutex<HashMap<String, HSTRING>>` を sink に内蔵し `GetProperty` は即答・`SetProperty` は即書き。**再入規約**「areka は `Get`/`Notify` 呼出中にプロパティストアのロックを保持しない」を契約化。欠落 key は **`SHIORI_E_PROPERTY_NOT_FOUND`（新設・失敗 HRESULT）** で決定的に観測（空 HSTRING 返しは「空値の key」と区別不能ゆえ却下）。M1 最小 key 集合（ukadoc プロパティシステムの dotted パス）は実装フェーズで確定する注記を design に明記 | 実装最小・並行モデル（案 b の ECS 同期コールバック）との衝突回避・R1.2 と同じ「暗黙既定値で続行しない」哲学 |
| (c) spawn シグネチャ | **案 a（位置引数拡張）採用**: `spawn(helper_exe, load_dir, shiori_name, parent_hwnd)`。子への供給は arg1=parent_hwnd（現行 helper 読み取り互換を維持）・arg2=load_dir・arg3=shiori_name＋env 3 種（`HOST32_PARENT_HWND`/`HOST32_LOAD_DIR`/`HOST32_SHIORI_NAME`）＋cwd=load_dir 維持。`SpawnConfig` struct は本仕様単独では YAGNI（下流 lifecycle でパラメーターが実際に増えた時に導入すればよい）。吸収は `echo_roundtrip.rs:132`/`error_paths.rs:300` の 2 箇所のみ | 最小差分・`PARENT_HWND_ENV` 慣行の同型拡張 |
| (d) testdll E2E 解決 | env `HOST32_TESTDLL_DLL` 優先 → `target/i686-pc-windows-msvc/{debug,release}/shiori.dll` 探索 → 不在なら明確 panic（`resolve_helper_exe` 慣行）。load_dir 成立は **一時 dir へ DLL コピー**方式を採用（`load_dir\shiori.dll` と cwd=load_dir の伺か慣習を同時検証・target dir 直指しは load_dir の意味が薄まるため却下）。**エクスポート欠落 fixture は別ビルド variant を作らない**: 欠落態様は helper クレートの i686 単体テストで `kernel32.dll`（`load` エクスポートを持たない実在 DLL）に対する解決失敗（`EntryNotFound`）として決定的に検証し、E2E の失敗パスは「DLL 不在」＋「env 強制 `load`→false」で代表する（「あらゆる ProxyError → ack[0]」は単一の写像コードパスであり E2E 2 態様で貫通が証明できる） | cargo feature variant は同一 target-dir でのアーティファクト衝突リスク・段取り複雑化に見合わない |
| (e) 実装順序 | WS-A / WS-B は**完全並行タスクグループ**（§1.2 の依存グラフ実測どおり）。順序制約は「submodule 展開 → WS-A proxy 実装（署名バイト確認 R13.5）」のみ。submodule 展開は workspace cargo 解決の全体前提でもあるため**最初のタスク**とする | §1.2 実測 |
| (f) error.rs 語彙 | `SHIORI_E_NOT_LOADED`／`ShioriError::NotLoaded` は**削除**（create 融合で「未ロード状態の IShiori」が契約上消滅）。`LoadFailed`→**`CreateFailed`**・`RequestFailed`→**`GetFailed`** に改名、**`NotifyFailed`** を新設。`SHIORI_E_PROPERTY_NOT_FOUND = make_shiori_failure(0x0004)`＋`ShioriError::PropertyNotFound` を新設。`SHIORI_E_UNKNOWN_TOKEN` は存置し、安全面での判別性向上のため **`ShioriError::UnknownToken`** 変種を新設（従来は `Com` 落ち）。`Com` は catch-all として存置 | 語彙とABI 契約の整合・判別可能性 |
| (g) 改名と Notify 観測 | `RequestOutcome`→**`GetOutcome`** に改名（`Immediate`/`Deferred` 変種・`CorrelationToken`/allocator は不変）。reference brain の `Notify` 観測は **`notifications: RefCell<Vec<HSTRING>>` 受領ログ**（`AsImpl` ダウンキャストで test から読む既存慣行）で担保 | `Get` との命名整合・応答なし片道の観測可能性 |

### 8.3 追加の設計確定（§7 持ち越し分の決着）

- **Load 用 timeout 方針**: 凍結 wire の `send_request` は呼出ごとに timeout を取るため ipc 改変不要。host クレートに **`pub const LOAD_ACK_TIMEOUT: Duration = Duration::from_secs(30)`** を推奨既定として新設し、LOAD の `send_request(MsgTag::Load, &[], LOAD_ACK_TIMEOUT)` と E2E で使用する（echo の 5s と別建て。実 DLL は actor 起動等で load が数百 ms〜数秒かかりうる・`SMTO_ABORTIFHUNG` がハング検出を担保）。実測調整は実装フェーズ。
- **helper 必須起動パラメーター欠落時**: `parent_hwnd` 前例と同じ**起動時 exit(2)**（HELLO 不達＋プロセス終了として親から決定的に観測可能）。半構成の helper を生かさない（R3.5）。
- **Load 再受領（proxy 確立済みで再度 Load）**: reload-in-place はしない（R2.4）。確立済みなら **ack[1] を冪等再送**（`load` を再呼出しない）。
- **ack ペイロード契約**: `MsgTag::Response`・厳密 1 byte・`[0x01]`=成功／`[0x00]`=失敗。定数は host/helper 各クレートでローカル定義（凍結 ipc へ定数追加はしない・E2E が両端一致を固定）。
- **新 IID 採番**: 3 interface とも実装時に新規 v4 GUID を採番（PowerShell `[guid]::NewGuid()`）し、既存慣行どおり IID 固定回帰テストで固定（旧 IID との相違も assert）。
- **プロパティ key 名前空間**: ukadoc プロパティシステムの dotted パス準拠（契約面のみ・R10.5）。M1 最小 key 集合は実装フェーズで ukadoc MCP を参照して確定。

### 8.4 Synthesis（design-synthesis 適用結果）

- **一般化**: (i) helper 起動パラメーター供給は「arg-n 優先・env fallback・`pub const` env キー」の**単一パターン**へ一般化（parent_hwnd/load_dir/shiori_name の 3 適用）。(ii) 安全面レイヤは「vtable=unsafe PascalCase／安全面=snake_case インヘレント」の**単一規約**で 3 interface（Factory/Shiori/Host）へ一様適用。(iii) D1/D7 は WS-A（proxy）と WS-B（session/brain）で同一原則の 2 実現として設計。
- **Build vs Adopt**: windows-core `#[interface]`/`#[implement]` を継続採用（§2 で制約込み実証済み・自作 vtable は不採用）。FFI は Win32 API 直（LoadLibraryW/GetProcAddress/GlobalAlloc/WideCharToMultiByte）で新規外部依存ゼロ。fixture も `windows` クレートのみ。
- **単純化**: 新設クレートは testdll の 1 個のみ（proxy は helper 内モジュール・factory/安全面は既存 shiori-abi 内）。helper に trait 抽象を導入しない（前 spec の YAGNI 判断を踏襲）。`ShioriExt` トレイトを廃止し層を 1 枚減らす。`SpawnConfig` struct を導入しない。エクスポート欠落 DLL variant を作らない。reload-in-place を作らない。

## 9. 実装フェーズ Task 1: flat-C 署名バイト正確照合（R13.5）

**実施日**: 2026-07-02（実装フェーズ着手時）。`git submodule update --init vendors/pasta` で `vendors/pasta` を展開（`048d646c` / v0.1.6-1）し、`cargo metadata --format-version 1 --no-deps` が **EXIT=0**（`[patch.crates-io] pasta_core = { path = "vendors/pasta/crates/pasta_core" }` 経路健全）を確認。

### 9.1 正確源

- `vendors/pasta/crates/pasta_shiori/src/windows.rs`（flat-C エクスポート本体）
- `vendors/pasta/crates/pasta_shiori/src/util/hglobal/mod.rs`（HGLOBAL 所有権規約の実装）

### 9.2 署名照合結果（設計 §ShioriByteProxy との差分）

| 項目 | 正確源 pasta（windows.rs） | 設計提案 | 判定 |
|---|---|---|---|
| 呼出規約 | `extern "C"` | `extern "cdecl"` | ✅ 一致（i686-pc-windows-msvc で C ABI＝cdecl。x64 も単一規約） |
| `load` | `pub extern "C" fn load(hdir: HGLOBAL, len: usize) -> bool` | `fn(hdir: HGLOBAL, len: usize) -> bool` | ✅ 完全一致 |
| `unload` | `pub extern "C" fn unload() -> bool` | `fn() -> bool` | ✅ 完全一致 |
| `request` | `pub extern "C" fn request(req: HGLOBAL, len: &mut usize) -> HGLOBAL` | `fn(req: HGLOBAL, len: *mut usize) -> HGLOBAL` | ✅ 一致（`&mut usize`≡`*mut usize` は ABI 上同一のポインタ渡し。本仕様では解決のみ・呼出しない） |
| 戻り値 bool | Rust `bool`（1 byte・Win32 BOOL ではない） | 同 | ✅ 一致（`load`/`unload` は Rust `bool` を直返し） |
| シンボル装飾 | `#[unsafe(no_mangle)]`＝無装飾（`load`/`unload`/`request` そのまま） | `GetProcAddress("load")` 等 | ✅ 一致（i686 MSVC でも no_mangle は先頭 `_` を付けない。testdll も `#[unsafe(no_mangle)]` で合わせる） |

### 9.3 HGLOBAL 所有権規約の確認（R4.5）

- `load(hdir, len)` → `RawShiori::load_impl` → `ShioriString::capture(hdir, len)`（`has_free: true`）→ **Drop 時に `GlobalFree(self.h)`**。すなわち **入力 HGLOBAL は callee（DLL 側）が解放**する。**ホスト（helper）は自ら解放してはならない**（二重解放禁止・R4.5）。testdll も同一挙動（load 入力を `GlobalFree`）を再現し、ホスト側二重解放バグの検出器とする（R7.2 系）。
- `request` 応答 HGLOBAL は `clone_from_str_nofree`（`has_free: false`）＝DLL は解放せず caller（host）が所有・解放する（下流 host32-request の領分）。
- `load` dir 文字列は `to_ansi_str()`（`Encoding::ANSI`＝JP 環境 SJIS・`MultiByteToWideChar`）で解釈 → helper は `load_dir` を **ANSI(CP_ACP)** で符号化して渡す（R4.4）。`GlobalAlloc(GMEM_FIXED)`（pasta 側 `GMEM_FIXED = 0`）と一致。

### 9.4 結論

**設計への差し戻し不要**。設計 §ShioriByteProxy / §TestDll の flat-C 3 署名・HGLOBAL 所有権規約・ANSI 符号化・GMEM_FIXED はすべて正確源とバイト正確に一致。proxy 側 fn ポインタ型は `extern "cdecl"` 宣言（pasta の `extern "C"` と i686/x64 で ABI 同一）で固定する。
