# Brief: areka-P0-host32-shiori-load

> **⚠ スコープ拡大（2026-07-02・/kiro-discovery 再入）**: 本仕様は当初「host-32 flat-C load 層の結線」だけだったが、設計ディスカッションで **IShiori ABI の根幹的欠陥（`load_dir` 欠落）** が判明。開発者判断により **IShiori / IShioriHost / IShiori コンストラクタ関数の ABI 契約是正まで本仕様に畳む**（分割せず 1 仕様完結）。理由: `load_dir` の根幹性・所有権・lifecycle(Drop) の推論は host-32 flat-C 層と IShiori COM 層を**地続きに貫く**ため、**context 継続性**を優先。1-feature=1-branch=1-PR より継続性を上位に置く開発者決定。名前 `host32-shiori-load` は拡大後スコープをやや過小表現するが、継続性のため改名しない。
>
> M1 `areka-P0-emo2-boot`「① SHIORI 通信層エンジン host-32」トラック。go ゲートは先進坑 `pilot-shiori-host-32`（✅ go 済 2026-07-01）で充足済み。

## Problem

**二層にまたがる欠落を一体で埋める。**

1. **host-32 flat-C load 層（当初スコープ）**: x64 areka は emo2 の 32bit SHIORI DLL を in-proc ロードできない。上流 `areka-P0-host32-ipc` が bytes-over-wire transport を完成させたが、helper 側は `respond()` echo stub（`crates/shiori-host32-helper/src/main.rs:54–56`）にすぎず実 DLL を触らない。`MsgTag::Load(2)` は定義済みだが未処理。
2. **IShiori ABI 層（拡大スコープ）**: 内部正準 ABI `IShiori`（`crates/shiori-abi`・完了済み `areka-P0-shiori-com` 由来）の `Load(host)` が **`load_dir` を欠く**。これは SHIORI3 互換以前の**根幹的欠陥**——`load_dir` は「ゴースト記述・辞書・シェルが実際にどこにあるか」を指す **per-instance のリソース根**であり、これを個別保持できるからこそ各ゴーストが独立して動く。`load_dir` を持たない SHIORI は自分の資源の在り処を知らない＝定義矛盾。加えて raw ポインタ露出による不要な `unsafe`、及び RAII で足りる `Unload` メソッドの冗長が同居する。

## Current State

- **完了済み・凍結（触らない）**: `shiori-host32-ipc`（WM_COPYDATA transport＝`MsgTag{Hello,Load,Request,Response,Unload}` / framing / `ResponseSlot` / `send_request`）。**この wire/framing/`MsgTag` 定義は本仕様でも改変しない。**
- **拡張対象（host-32 flat-C 層）**: `shiori-host32-helper` の `respond()` echo stub・`handle_message()`（`Load`/`Unload` 未結線・FFI 足場皆無）。`shiori-host32-host::spawn(helper_exe, ghostdir, parent_hwnd)` は ghostdir を **cwd のみ**で運ぶ（明示 arg 化されていない）。
- **是正対象（IShiori ABI 層・`crates/shiori-abi`）**:
  - `interface.rs:79` `unsafe fn Load(&self, host: *mut c_void) -> HRESULT` — **dir 欠落・unsafe・raw ポインタ**。
  - `interface.rs:88` `unsafe fn Unload(&self) -> HRESULT` — **RAII で代替可能ゆえ冗長**。
  - `interface.rs:109` `unsafe fn Request(&self, input: *const HSTRING, out_response, out_token) -> HRESULT` — raw ポインタ・out-param。
  - `IShioriHost`（`Raise`/`Complete`・raw ポインタ・unsafe）。
  - IShiori コンストラクタ関数（factory／`shiori_create` 系入口）の ABI。
  - `ergonomic.rs` の `ShioriExt`（`load(host)` dir 欠落）・mock/test・consumer（`areka-P0-shiori-protocol` / `-reference`）が是正の波及先。
- **知見 donor（参照のみ・コピペ禁止）**: 先進坑 `crates/pilot/examples/shiori-host-32/shiori_proxy.rs`（FFI/所有権/charset 実証・README 学び #4–#6）。

## Desired Outcome

**下から上まで `load_dir` が per-instance で貫通し、teardown が RAII で一貫する。**

- host-32: x64 親が実 i686 helper 越しに SHIORI DLL を `LoadLibraryW` → `load(load_dir)` 成功（`true`）まで駆動し無クラッシュ観測。helper 内に `load`/`unload`/`request` 3 fn ポインタを保持する常設 `ShioriByteProxy` が立つ。
- IShiori: `Load(host, load_dir)` が per-instance の load_dir を COM 契約面で受け、`Unload` は消え Drop(RAII) が teardown を持ち、メソッドは型付き COM 引数で安全化される。コンストラクタ関数の ABI も是正される。

## Approach

2 ワークストリームを **`load_dir` 根幹性・所有権・Drop lifecycle** の一貫原則で束ねる。

**WS-A: host-32 flat-C load 層**（当初スコープ・下記 Confirmed Decisions D2–D5 に従う）
1. helper: `MsgTag::Load` をトリガとして結線（`InboundAction::LoadDll`・echo stub 置換）。
2. `ShioriByteProxy`（i686・`unsafe` 集約）: `LoadLibraryW(load_dir\<SHIORI名>)` → `GetProcAddress`×3 → cdecl fn ポインタ保持。
3. `load(load_dir)`: ANSI(CP_ACP) 符号化 → `GlobalAlloc(GMEM_FIXED)` → 入力 HGLOBAL は DLL 解放。
4. load-ack = `MsgTag::Response` 1byte bool（凍結 wire 上・新タグ不要）。
5. 最小 SHIORI DLL fixture（i686 cdylib）で成功／失敗を決定的 E2E 観測。

**WS-B: IShiori ABI 是正**（拡大スコープ・下記 D1/D6/D7 に従う・**Option B factory 採用**）
1. **factory 新設**: `shiori_factory(out) -> IShioriFactory`（module entry・旧 `shiori_create` を置換）＋ `IShioriFactory::create(load_dir: &HSTRING, host: Ref<IShioriHost>, out: OutRef<IShiori>)`＝**生成＋load 融合**の per-ghost instance 生成。factory=backend（native/host-32 互換/mock）、instance=ゴースト。
2. **`IShiori` 痩身**: `Request(input: &HSTRING) -> Result<RequestOutcome>` のみ。**`Load`/`Unload` メソッド消滅**（load は factory.create に融合・teardown は Drop(RAII)）。
3. **`unsafe` 除去＝型付き COM 引数**: IF 借用は `Ref<'_, T>`（`&IShioriHost` は二重ポインタゆえ不可）／OUT は `OutRef<'_, T>`／文字列は `&HSTRING`。生 `*mut c_void` は避けられる範囲で避ける。
4. **`IShioriHost` 役割拡充＋安全化**: `Raise`/`Complete` に加え **`GetProperty(key)->HSTRING`／`SetProperty(key,value)`（共有変数/プロパティアクセス・同期 brain→host）を新設**。`key`=SSP プロパティシステムの dotted パス（ukadoc `list_propertysystem` 準拠）。sink は共同所有ゆえ host は `Ref` 渡し（callee clone）。
5. teardown は Drop(RAII) 全層一貫（IShiori impl Drop → `ShioriByteProxy::Drop` の courtesy unload+FreeLibrary）。
6. consumer（`shiori-protocol`/`-reference`・`ShioriExt`・`ShioriSession::activate`→factory 経由へ・mock/test）の波及更新。

## Confirmed Decisions（本ディスカッションで確定・継続性の核）

- **D1 [根幹] `load_dir` は SHIORI の根幹**: per-instance のリソース根（記述/辞書/シェルの所在）。個別保持で各ゴースト独立。エンジン実体が areka 内部共有でもインスタンス毎 load_dir で独立動作。**全層で必須**（SHIORI3 互換以前の原理）。※誤って盾にした記憶 `areka-engine-construction-model`（「dir は construction に隠す」解釈）は**誤り＝訂正対象**。
- **D2 [供給] 起動パラメーター**: helper へ `load_dir`＋`SHIORI 名` を**明示 arg＋env fallback**（`parent_hwnd` と同規約・env 例 `HOST32_LOAD_DIR`/`HOST32_SHIORI_NAME`）で。LOAD メッセージは**トリガ化**（wire でパスを運ばない）。`spawn` 起動パラメーター契約を拡張（凍結 wire には及ばない）。
- **D2' [cwd] helper プロセス cwd = load_dir**: 伺か慣習（SHIORI は自ディレクトリを cwd 前提に相対 I/O・SAORI 参照）ゆえ `spawn` が子 cwd を load_dir に設定**維持**。ただし helper は load_dir の**値**を明示 arg から取得（cwd 推測しない）。cwd と load 引数は同一 ghost/master で整合。
- **D3 [名前] SHIORI 名は descript.txt `shiori,<ファイル名>`**（既定 `shiori.dll`・ukadoc `descript_ghost#shiori,ファイル名`）。emo2 の `pasta.dll` は一例。名前解決（descript 参照）は親／`package-mount` の領分・helper は受け取るのみ。
- **D4 [fixture] host-32 トラック所有の最小 SHIORI DLL**（i686 cdylib `shiori-host32-testdll`・`crate-type=["cdylib"]`・`[lib] name="shiori"`）を主役。`load→false` 強制で失敗パス決定的テスト化。本物 emo2 `pasta.dll` は `HOST32_PASTA_DLL` env-gated 任意 confidence（pilot go 済が根拠・CI 必須にしない）。`crates/pilot` 非依存（葉ノード隔離）。
- **D5 [ack] load-ack = `MsgTag::Response` 1byte bool**（`[1]`=成功）。親 `send_request(MsgTag::Load, &[], t)` の再入 RESPONSE 経路にそのまま乗る（凍結 wire 不改変・実コードで裏取り済）。
- **D6 [IShiori ABI] factory 採用（Option B）＋ sink 拡充**:
  - **factory**: `shiori_factory() -> IShioriFactory`／`IShioriFactory::create(load_dir, host) -> IShiori`（生成＋load 融合・旧 `shiori_create` 置換）。factory=backend・instance=per-ghost（load_dir で独立）。→ **`IShiori::Load` は無駄ゆえ廃止**（create に融合）。
  - **`IShiori`**: `Request` のみ（`Load`/`Unload` 消滅・teardown=Drop）。
  - **`IShioriHost`**: `Raise`/`Complete`＋**`GetProperty`/`SetProperty` 新設**（共有変数アクセス＝sink の正当な役割・ukadoc プロパティシステム準拠）。
  - **安全化**: `unsafe` 除去・IF 借用は `Ref<'_,T>`（`&T` は二重ポインタで不可）・OUT は `OutRef<'_,T>`・文字列 `&HSTRING`。C エクスポート入口（`shiori_factory`）は `extern "system"`＋raw out-param 維持。
  - host 渡しは `Ref`（sink 共同所有ゆえ callee clone・by-value 不採用）。
- **D7 [teardown] Drop(RAII) 全層一貫**: teardown は best-effort（どうせ畳む）。helper ハング等はプロセス lifecycle（kill・下流 `host32-lifecycle`）で処理し戻り値では扱わない。reload-in-place は areka=1 helper=1 ゴースト＝再生成で足る（YAGNI）。

## Scope

- **In**:
  - **[WS-A]** helper の `Load` トリガ結線／`ShioriByteProxy`（3 エクスポート解決・`load` 呼出・ANSI 符号化・HGLOBAL 所有権・Drop courtesy unload）／`spawn` 起動パラメーター拡張（load_dir・SHIORI 名 arg＋env・cwd=load_dir）／最小 SHIORI DLL fixture crate／LOAD E2E（成功・失敗・無クラッシュ）。
  - **[WS-B]** `IShiori`/`IShioriHost` の ABI 是正（`Load(host, load_dir)`・`unsafe` 除去・`Unload` 削除→Drop・型付き COM 引数）／IShiori コンストラクタ関数 ABI 是正／`ergonomic`（`ShioriExt`）・mock/test の追随／consumer（`shiori-protocol`/`-reference`）の波及更新。
- **Out**:
  - `request` の**呼出**・SHIORI/3.0 build/marshal・Value parse・request の UTF-8 charset（→ `areka-P0-host32-request`）。※`request` fn ポインタの解決は本仕様。
  - 常駐メッセージループ生存・`OnSecondChange` poll・crash 監視の lifecycle（→ `areka-P0-host32-lifecycle`）。※teardown の Drop courtesy unload は本仕様。
  - **WM_COPYDATA transport（`shiori-host32-ipc` の wire/framing/`ResponseSlot`/HELLO/timeout/`MsgTag` 定義）の改変**（凍結）。
  - native x64 脳の実装本体・里々/YAYA・SAORI・native x64 化（M2 以降）。※IShiori ABI 是正は native 脳にも将来効くが、native 実装は本仕様外。

## Boundary Candidates

- **層の分割線**: WS-A（i686 helper flat-C）⟷ WS-B（x64 IShiori COM）。両者を `load_dir`／Drop lifecycle で結ぶが、crate 境界（`shiori-host32-*` ⟷ `shiori-abi`）で責務分離。
- **charset 分割線**: IShiori=`HSTRING`(UTF-16) ／ 過去互換アダプタが ANSI(CP_ACP) 変換 ／ helper flat-C=ANSI。
- **teardown 分割線**: Drop で全層一貫・恒常 lifecycle は下流。
- **ABI 波及境界**: IShiori consumer（`shiori-protocol`/`-reference`/mock）の更新範囲を design で網羅。

## Out of Boundary

- WM_COPYDATA transport の改変（上流凍結）。
- `request` 呼出・SHIORI/3.0 セマンティクス・常駐 lifecycle。
- native x64 脳実装・M2 互換面拡大。

## Upstream / Downstream

- **Upstream**:
  - `areka-P0-host32-ipc`（✅完了・凍結 transport）。
  - `areka-P0-shiori-com`（✅完了・**本仕様が IShiori ABI を是正する対象**＝1 仕様完結ゆえ別 revisit せず本ブランチで是正）。
  - `pilot-shiori-host-32`（✅ go・参照専用）／`vendors/pasta`（flat-C ABI 正確源・要 `git submodule update --init`）／`doc/COMPAT_ARCHITECTURE.md`（正本）。
- **Downstream**:
  - `areka-P0-host32-request`（proxy の `request` 呼出）／`areka-P0-host32-lifecycle`（常駐＋unload 恒常呼出）。
  - IShiori consumer: `areka-P0-shiori-protocol`/`-reference`（是正 ABI へ追随・本仕様内で更新）。
  - x64 IShiori 過去互換アダプタ（host-32 を IShiori として提示・是正後 `Load(host, load_dir)` を消費）。

## Existing Spec Touchpoints

- **Extends（本仕様が是正する完了済み spec）**: `areka-P0-shiori-com`（IShiori/IShioriHost/コンストラクタ ABI）。**開発者決定により別 revisit せず本仕様 1 本で是正**（context 継続性優先）。
- **Adjacent**: `areka-P0-host32-ipc`（凍結・結線のみ）／`areka-P0-host32-request`・`-lifecycle`（同 proxy 共有）／`areka-P0-shiori-protocol`・`-reference`（IShiori consumer・波及更新）。

## Constraints

- **ビルド**: i686（helper・testdll）は **PowerShell 必須**（Git Bash link.exe トラップ）。`cargo test --target i686-pc-windows-msvc` も回す。
- **32bit 可搬性**: `usize`=32bit ゆえ dwData/ULONG_PTR 演算は `u64` 幅で評価。
- **flat-C ABI**: cdecl `extern "C"`・返り値 Rust `bool`(1byte)・`request` len は in/out・HGLOBAL=`GlobalAlloc(GMEM_FIXED)`・load 入力 HGLOBAL は DLL 解放。
- **charset**: `load` dir=ANSI(CP_ACP)・`WideCharToMultiByte`。IShiori 面は `HSTRING`(UTF-16)。
- **IShiori 安全化の検証**: windows-core 0.62 `#[interface]` が非 `unsafe` メソッド＋型付き引数（`HSTRING`・interface 参照）＋out-param 安全表現を許すか、design で実装可否を確定要（許さない箇所は最小 `unsafe` を局所化）。
- **ABI 波及**: IShiori 是正は完了済み consumer（`shiori-protocol`/`-reference`）へコンパイル破壊的。同一ブランチ内で追随更新（1 PR）。
- **命綱（葉ノード隔離）**: production は `crates/pilot` へ inbound 依存しない。
- **実バイナリ内部前提を置かない**（README 学び #5）: 観測契約は `load` 同期 bool＋無クラッシュのみ。
- 制約変更の正本は `doc/COMPAT_ARCHITECTURE.md`。

## Open Questions（design で決める・discovery のブロッカーではない）

1. **IShiori 安全メソッドの実装可否**: windows-core 0.62 の `#[interface]` で `unsafe` を外し型付き COM 引数（`Ref<T>`/`OutRef<T>`/`&HSTRING`）／`Request` の out-param（応答・token）を安全表現できるか。不可なら最小 `unsafe` を局所化する落としどころ。
2. **factory の詳細**: `IShioriFactory` の新規 IID 採番／`create` の out-param 形（`OutRef<IShiori>` vs raw）／host-32 x64 過去互換アダプタが**独自 factory**（別 backend）として `IShioriFactory` を実装する形。`shiori_factory` の C 入口署名確定。
6. **プロパティシステム semantics**: `GetProperty`/`SetProperty` の key 名前空間（SSP `list_propertysystem` の dotted パス・汎用 vs `ext.` 拡張）・同期契約・スレッド安全性（brain 任意スレッドから呼ばれうる）・欠落 key の扱い（error vs 空）・host-32 越しの実現（compat は sakura `\![get,property]`/イベント経由か・native は同期コールバックか）を design で確定。M1 で emo2 が使う最小 key 集合に絞る。
3. **consumer 波及範囲**: `shiori-protocol`/`-reference`/ergonomic/mock の具体的更新点を design で網羅（IShiori impl・呼出側）。
4. **ABI 一次源再確認**: 実装前に `git submodule update --init` で `vendors/pasta` を展開し flat-C 署名をバイト正確に再確認（`[patch.crates-io] pasta_core` の path 健全化も兼ねる）。
5. **記憶訂正**: `areka-engine-construction-model` の「dir を construction に隠す」解釈を D1 に沿って訂正（load_dir は load 契約に必須）。
