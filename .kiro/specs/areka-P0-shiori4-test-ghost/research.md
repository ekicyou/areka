# ギャップ分析 — areka-P0-shiori4-test-ghost

> 調査日: 2026-07-18 ／ 対象: 確定済み requirements.md（要件 1〜7）と既存コードベースの差分
> 目的: 「境界を踏む決定論テストゴースト」を最小手数で立ち上げる基盤の実装戦略を、決定でなく選択肢として提示する。
> 注記: 本ドキュメントは分析であり最終決定ではない（要件討議への入力）。正典はゴースト仕様＝ukadoc、内部 SHIORI4/IShiori 規約は `shiori-abi` rustdoc。

---

## 1. 現状調査（Current State）

### 1.1 資産マップ（要件が差し込む既存シーム）

| 資産 | 所在 | 役割 | 本 spec との関係 |
|---|---|---|---|
| IShiori COM ABI | `crates/shiori-abi/src/interface.rs` | `IShioriFactory::CreateInstance`（生成＋load 融合）／`IShiori::Get`(即時/遅延2分岐 HRESULT)/`Notify`／`IShioriHost::Raise/Complete/GetProperty/SetProperty` | **無改変で消費**（要件 2.1・Adjacent）。3 IID 固定・vtable 健全性は単体檻あり。 |
| ReferenceBrain＋ReferenceFactory＋`shiori_factory` C 入口 | `crates/areka/src/reference_brain.rs` | `#[implement(IShiori)]` の**エコー脳**＋`#[unsafe(no_mangle)] extern "system" fn shiori_factory(out) -> HRESULT` | **種**として活用（要件 2）。ただし現状は content 不解析エコー（後述の乖離）。**areka は bin-only ＝ cdylib 非在**。 |
| ShioriHostSink | `crates/areka/src/shiori_host.rs` | areka 本体側 `#[implement(IShioriHost)]` sink（Raise/Complete/GetProperty/SetProperty・メールボックス・property ストア・単一 in-flight 突合枠） | InProc アダプタが CreateInstance に渡す host に必要。**bin-only 内に居る**。 |
| ShioriSession | `crates/areka/src/shiori_session.rs` | in-proc アクティベーション経路（factory→create→IShiori 保持）＋利用規律（単一 in-flight／遅延タイムアウト（注入時刻）／Drop teardown） | **InProc アダプタの中核ロジックが既に実在**。ただし `SessionRequest`(Immediate/Deferred) を返し、`ShioriBackend`(Result<Option<String>>) へは未接続。**bin-only 内に居る**。 |
| ShioriBackend トレイト | `crates/areka-kanade/src/shiori/real.rs` | shiori アクターが駆動する backend 抽象。`get(id,refs,status)->Result<Option<String>,RequestError>`／`notify`／`unload()->Result<ExitKind,ShutdownError>`／`status()->HelperStatus` | **無改変で実装**（要件 3・Adjacent）。エラー語彙が host32 型（後述の型結合）。 |
| ShioriWiring 列挙＋boot() | `crates/areka-ghost/src/runtime.rs` | 現状 2 変種 `Helper{helper_exe}`／`Custom(Box<dyn FnOnce()->Result<Box<dyn ShioriBackend>,String>>)`。`boot()` が変種→connect closure を組む | **第3変種 `InProc` を追加**（要件 3.1）。connect closure は shiori アクタースレッド上で一度だけ実行（`!Send` 前提の座）。 |
| real_connect（Helper 結線） | `crates/areka-ghost/src/shiori_wiring.rs` | `mount.shiori`(dir/file)→窓生成→spawn(i686 helper)→HELLO→LOAD ack | InProc 版の**対になる新規結線**を並置（helper 不要版）。 |
| x64 SHIORI/3.0 codec | `crates/shiori-host32-host/src/shiori3.rs` | `build_request(ShioriRequest)->Vec<u8>`／`parse_response(&[u8],Charset)->Result<ParsedResponse,ShioriError>`。UTF-8 固定・Status 行・不透明転記 | **InProc アダプタで再利用可**（要件 3.2・不透明搬送）。純粋・`windows` 非依存。 |
| Shiori3Client | `crates/shiori-host32-host/src/client.rs` | codec を Helper 窓へ束ねる client（`ShioriConnection` が使う）。id/refs/status→build_request→…→parse_response→区別語彙写像 | **写像規律の参照見本**（InProc は窓でなく IShiori::Get へ差し替える同型）。 |
| flat-C LoadLibrary 見本 | `crates/shiori-host32-helper/src/shiori_proxy.rs` | i686 `LoadLibraryW`→`GetProcAddress("load"/"unload"/"request")`→transmute→呼出→Drop(FreeLibrary) | **x64 COM 版（`shiori_factory` 解決）の設計テンプレ**。半構築非露出・所有権規約が精密。 |
| 32bit 決定論 fixture | `crates/shiori-host32-testdll/`（`[lib] name="shiori" crate-type=["cdylib"]`→`shiori.dll`） | flat-C `load/unload/request`・request line/ID を parse し固定応答（200/204/400）を選択 | **x64/COM 版が作るべき兄弟**（そのまま残置・要件 6・Adjacent）。 |
| ScriptedShioriBackend／RecordingSink／spine e2e | `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` | `ShioriWiring::Custom` 上の台本 fake backend＋記録 sink＋S1〜S6 決定論 e2e（Tick 注入・sleep 不使用） | **併存維持**（要件 1.6/6.2/6.4）。**新 e2e の駆動技法の直接見本**（boot→Tick retry→発火列照合→shutdown）。 |
| マウント解決 | `crates/areka-parsers/src/package/{model,resolve}.rs` | `resolve(ghost_root,DefaultEncoding)->MountModel{shiori:ShioriMount{dir,file:Option},shell,...}` | **fixture が満たすべき契約**（要件 4）。`shiori,<file>` 無し＝`file:None`・推測しない。 |
| emo2 依存 smoke | `crates/areka/tests/{smoke_boot_loop_exit,emo2_real_run}.rs` | `CARGO_BIN_EXE_areka` 子プロセス＋ログ marker grep。emo2 fixture 指定 | **乗り換え可否の仕分け対象**（要件 6.3）。※実は SHIORI 境界を決定論的に踏んでいない（wire 成立止まり・下記）。 |

### 1.2 規約・パターン（Conventions）

- **層と依存方向**: `shiori-abi`（ABI 定義のみ）← `shiori-host32-host`（x64 codec/client、`RequestError`/`ExitKind`/`ShutdownError`/`HelperStatus` の**語彙源**）／`areka-kanade`（`ShioriBackend` トレイト・host32 型を消費）／`areka-ghost`（boot 結線・`ShioriWiring`）／`areka`（bin・脳/sink/session の実体はここに集中）。
- **COM エクスポート規約**: `extern "system"`（`#[unsafe(no_mangle)]` で C リンケージ・記憶 windows-com-export-calling-convention）。
- **失敗経路のログ規律**: 安易な panic 禁止・失敗は `error!`＋`Err` 戻り（記憶 areka-log-first-no-silent-failure・要件 3.5）。
- **決定論テスト**: sleep 不使用・注入時刻（Tick）のみ・env-gate は本質的非決定のみ（記憶 deterministic-test-coverage-mandate・要件 1.2/1.3/5.4）。
- **bin-crate 内部テスト配置**: areka は no-lib ゆえ内部項目テストは in-crate `#[cfg(test)]`、バイナリ起動型は `tests/`（記憶 areka-bin-crate-internal-tests-in-crate）。
- **fixture 原則**: emo2 は最小適合 fixture であって書式の聖典ではない・正典は ukadoc（記憶 areka-ghost-boot-descript-not-install）。
- **偽装境界の前進**: 本 spec は「偽装境界（純 x64 fake 注入）」を**実 DLL 境界まで前進**させるもの（記憶 prefer-x64-fake-boundary-tests-not-x86 と同思想の一歩先）。

---

## 2. 要件別フィージビリティと差分（Requirement-to-Asset Map）

タグ: **[欠落]**＝新規作成が必要 ／ **[制約]**＝既存構造が形を規定 ／ **[未知]**＝設計フェーズで要研究。

### 要件 1（最優先: テスト作者のエルゴノミクス）
- 既存 `GhostBootOptions`/`ShioriWiring`/`boot()` へ **fixture パス＋in-proc 選択のみ**で乗る（要件 1.1）。**[制約]** `GhostBootOptions` は `shiori: ShioriWiring` を1フィールドで受けるため、InProc 変種を足せば「別 boot 入口を新設しない」を自然に満たす。
- 常設 `cargo test --workspace`・env-gate/i686/helper/pasta/別プロセス**なし**（要件 1.2）。**[制約]** 新 cdylib が同一 workspace の x64 成果物として `cargo test` 依存ビルドに載る必要（後述の「cdylib のテスト時ビルド保証」＝**[未知]**）。
- Tick のみで時間前進（要件 1.3）＝ spine e2e の retry ループ技法が既に確立（`TickerMode::Disabled`＋dispatcher/kanade へ Tick 注入）。**[欠落]** 新 e2e 自体。
- SHIORI 交信列（id・NOTIFY/GET・順序）と cue sink 出力の**双方**を assert（要件 1.4）。**[欠落]** InProc 経路では現状 backend 呼出の記録装置が無い（Custom+Scripted は fake ゆえ記録できるが、InProc は実 DLL＝backend 内で記録できない）。→ **交信列の観測点をどこに置くか**が設計論点（下記 D-3）。
- `ShioriWiring::Custom`/`ScriptedShioriBackend` の併存（要件 1.6）＝現状のまま。

### 要件 2（x64 SHIORI4 テスト DLL）
- **[欠落・中核]** 新 cdylib crate（命名候補 `shiori4-testdll`）。`shiori_factory`(extern "system")→`IShioriFactory`/`IShiori` の決定論脳。
- **[制約・重大] areka bin-only 問題**: 種となる `ReferenceBrain`/`ReferenceFactory`/`shiori_factory`/`ShioriHostSink`/`ShioriSession` は**すべて bin-only の `areka` crate 内**に居り、cdylib からも `areka-ghost` からも依存できない。→ **リファレンス脳機構の lib 化（carve-out）が本 spec の隠れた最大工数**（下記 Option 群の主軸）。
- **[乖離] 台本 vs エコー**: ReferenceBrain は content 不解析の**純エコー**（即時 Get は input をそのまま返す）。要件 2.2 は「正典イベント（OnInitialize/OnFirstBoot/OnBoot/basewareversion/OnSecondChange/OnClose）ごとに単一 authoritative 台本から固定応答」＝**ID による分岐が必要**。これは `shiori-host32-testdll` の `parse_request`（request line＋ID 抽出）→`select_response` と**同型**。よって新脳は「HSTRING content を UTF-8 として読み ID を抜き、固定 SHIORI/3.0 応答 HSTRING を返す」＝エコー脳の**別種**（要件 2.5 の「独自スキーマを発明しない」とは両立＝SHIORI/3.0 の ID 行を読むだけで意味づけ分岐を増やさない）。
- 未知/未台本 ID → 204 相当（要件 2.3）／malformed → fail-visible・panic せず（要件 2.4）＝testdll の 400 選択と同型。

### 要件 3（正規 in-proc ロード経路 `ShioriWiring::InProc`＋アダプタ）
- **[欠落]** `ShioriWiring::InProc{ dll_path }`（または `{dir, file}`）変種＋`boot()` の match 追加。
- **[欠落]** DLL ロード経路: x64 `LoadLibraryW`→`GetProcAddress("shiori_factory")`→transmute(`extern "system" fn(*mut *mut c_void)->HRESULT`)→`IShioriFactory::from_raw`→`CreateInstance(load_dir, name, host)`→`IShiori`。**[制約]** `shiori_proxy.rs`（flat-C 版）が半構築非露出・FreeLibrary teardown の精密テンプレを提供（ただし解決するシンボルと署名が COM 版で異なる）。
- **[欠落・部分既存] `IShiori`→`ShioriBackend` アダプタ**（要件 3.2）: `ShioriSession`（in-proc IShiori 駆動＋単一 in-flight＋GetOutcome 分岐＋Drop teardown）が**中核ロジックを既に実装済み**だが、(a) bin-only、(b) 返り値が `SessionRequest` で `Result<Option<String>,RequestError>` へ未写像、(c) 入出力が生 HSTRING で、id/refs/status→codec→HSTRING／HSTRING→codec→Option<String> の**両端 codec 挟み込み**が未実装。→ アダプタ＝「codec でリクエスト bytes 組立→String→HSTRING→`IShiori::Get`→HSTRING→bytes→`parse_response`→status で `Ok(Some)`/`Ok(None)`/`Err` 写像」。写像規律は `Shiori3Client::get`（client.rs:118）と `real.rs::map_get_result` 相当が参照見本。
- **[制約・型結合] `ShioriBackend` のエラー語彙が host32 型**: `unload()->Result<ExitKind,ShutdownError>`／`status()->HelperStatus`／`get(..)->Result<_,RequestError>` はいずれも `shiori-host32-host` の型。別プロセス helper が居ない InProc でも、これらを供給せねばならない。妥当な写像: `status()`→常に `HelperStatus::Running`（死活監視対象なし・要件 3.3）／`unload()`→Drop teardown 実行→`Ok(ExitKind::Clean)`／load 失敗・get/notify 失敗→`RequestError`/接続失敗（`Err(String)`）。→ **InProc アダプタは `shiori-host32-host` へ依存する**（型と codec の双方）＝命名は host32 だが x64 純関数群なので健全。
- 正規実装であり M2 native x64 SHIORI4 が同一シームに乗る（要件 3.6・7.1）。

### 要件 4（マウント可能テストゴースト fixture）
- **[欠落]** 完全ゴーストフォルダ: `ghost/master/descript.txt`（charset UTF-8＋`shiori,<testdll>` 行＋`seriko.defaultsurfacedirectoryname,master`＋`name`）＋最小 shell（`shell/master/descript.txt`＋`surfaces.txt`＋数枚 PNG）＋（必要なら）最小 balloon。
- **[未知・重要] DLL パス解決**: `resolve` が返す `mount.shiori.file` は**ファイル名のみ**、`mount.shiori.dir`＝`ghost/master`。だが実ビルド cdylib は `target/<profile>/` に出る。fixture 同梱ではない。→ **どうやって boot に実 DLL の絶対パスを渡すか**が設計論点（D-1）:
  - (a) `InProc{ dll_path }` を明示絶対パスで受け、`mount.shiori.file` は宣言整合の確認のみ（fixture の descript は名前一致のため必要だが load 元は override）。
  - (b) テストが起動前に `CARGO_*` 由来のビルド済み cdylib を fixture の `ghost/master/<name>` へコピー（shiori_proxy テストが temp load_dir へ shiori.dll をコピーする流儀）。
  - (c) fixture を temp に生成（spine e2e 流）＋cdylib をそこへコピー。
- **[制約]** `resolve` は shell dir 物理存在を要求（`ShellDirMissing`）＝shell フォルダ実在が必須。surfaces/PNG が boot→talk→close で**実消費されるか**は要研究（要件 4.2 の「過不足なく」）＝**[未知]**（seriko/emo の実描画まで踏むなら PNG 実体要・cue sink 記録止まりなら最小で足りる）。
- pasta.dll・32bit 成果物を一切含まない（要件 4.3）。

### 要件 5（boot→talk→close 決定論 e2e・常設ゲート）
- **[欠落]** 新 e2e。**[制約]** 配置先は `areka-ghost/tests/ghost/`（spine e2e と同居）か新規テストファイル。InProc は areka-ghost の `ShioriWiring` を使うので areka-ghost の `tests/` が自然。
- 駆動技法は spine S1 が確立（boot→Tick retry で active slot 到達→RecordingSink 発火列照合→shutdown）。差分は **backend が実 DLL** な点＝(a) 起動系列 id 列の観測手段（要件 1.4）と (b) OnBoot talk が実 DLL の台本由来である点。
- OnBoot talk（台本どおり cue 列）が sink に届く（要件 5.2）／clean close 握手（要件 5.3）＝spine S1/S5 の観測型を踏襲。

### 要件 6（既存資産との共存・置換規律）
- env-gate 実 pasta 追験（`HOST32_PASTA_DLL`/`AREKA_EMO2_REAL_RUN`）残置（要件 6.1）／emo2-conformance-e2e spine 不侵（要件 6.2）／`ScriptedShioriBackend` 併存（要件 6.4）＝**現状維持で自動達成**。
- **[未知・仕分け] emo2 依存 smoke の乗り換え可否**（要件 6.3）: `smoke_boot_loop_exit.rs`/`emo2_real_run.rs` は `CARGO_BIN_EXE_areka` 子プロセス＋ログ grep で、**wire 成立（`wired=true`）まで**を見る＝実は SHIORI 境界を決定論的に踏んでいない（32bit helper の LOAD 失敗より前に wire 成立へ到達）。→ これらは「窓/wire plumbing の smoke」であり本 spec の「境界決定論 e2e」とは目的が異なる。仕分け候補: **残置**（役割が別）＋新 e2e を**追加**、が素直。乗り換え＝smoke を InProc fixture へ差し替えるかは討議事項。

### 要件 7（スコープ境界・M2 前方整合）
- 本番 main 結線 unchanged（要件 7.2）／descript 駆動 bitness 判別は M2 予約（要件 7.3）／`Raise` 自発・deferred は ReferenceBrain 既存檻超えない（要件 7.4）／SAORI・里々・YAYA 非対象（要件 7.5）＝**スコープ規律で達成**。
- InProc シームを M2 native x64 の正規消費点として位置づけ（要件 7.1）＝ carve-out した lib の公開面設計に前方整合性を織り込む（下記 Option）。

---

## 3. 実装アプローチ（Options）

本 spec の設計自由度の核心は「**リファレンス脳機構をどこへ carve-out し、テスト DLL とテスト脳をどう作るか**」にある。三点セット（DLL／InProc アダプタ＋シーム／fixture＋e2e）のうち、DLL とアダプタが carve-out 判断に強く依存する。

### Option A: 最小 carve-out（新 lib crate に脳機構を移設し、両者が依存）
- **構成**: 新 lib crate `shiori4-brain`（仮）を作り、`ReferenceBrain`／`ReferenceFactory`／`shiori_factory`／`ShioriHostSink`／`ShioriSession`／(必要なら)`GetOutcome` 消費機構を `areka` bin から移設。areka bin は新 lib を再エクスポート消費（既存 in-crate テストは移設先 or bin 側薄ラッパへ）。
  - 新 cdylib `shiori4-testdll` は `shiori4-brain` を dep し、**台本脳**（ID 分岐の別脳）＋`shiori_factory` を実装。
  - InProc アダプタは `areka-ghost` 内に新設し `shiori4-brain`(session/host)＋`shiori-host32-host`(codec/型) を dep。
- **トレードオフ**: ✅ M2 native 脳・reference 脳・test 脳が同一公開面に乗る前方整合／重複ゼロ。 ✅ InProc アダプタが実証済み ShioriSession をそのまま活かす。 ❌ bin→lib 移設は既存 in-crate テスト（`shiori_*_e2e_tests.rs` 群・`reference_brain.rs`/`shiori_host.rs`/`shiori_session.rs` の #[cfg(test)]）の**移設/再配線が広範**（記憶 areka-bin-crate-internal-tests-in-crate と衝突しうる＝bin 内部テストの一部が lib テストへ移る）。 ❌ 影響半径が大（areka bin の main.rs 経路も新 lib 参照へ）。

### Option B: 独立テスト DLL（脳を areka から借りず、testdll を自給自足で新規実装）
- **構成**: cdylib `shiori4-testdll` を `shiori-abi` のみ dep で自給。`#[implement(IShiori/IShioriFactory)]` の**台本脳**を新規に小さく実装（`shiori-host32-testdll` の x64/COM 版・ReferenceBrain を種にせずゼロから）。`shiori_factory` も testdll 内に独自実装。
  - InProc アダプタは `areka-ghost` 内で、`IShiori` を直接駆動する**薄いアダプタ**を新規実装（ShioriSession を借りず、単一 in-flight 等の重装備は要件が要らなければ省く＝要件 7.4「deferred は最小のみ」に整合）。host は `IShioriHost` の**最小テスト実装**を areka-ghost 内 or testdll 内に新設（ShioriHostSink を借りない）。
- **トレードオフ**: ✅ areka bin へ一切触れない＝影響半径最小・既存テスト無改変。 ✅ testdll が pilot testdll の x64 兄弟として素直（同型の parse/select）。 ❌ ShioriHostSink/ShioriSession の実証済みロジックを**再実装**（GetOutcome 分岐・host sink・Drop teardown を薄くとも再度書く）＝一部車輪の再発明（記憶 areka-cue-runtime-consolidated の反面教師）。 ❌ M2 native 脳が乗る「reference と同一公開面」は別途未整備のまま（要件 7.1 の前方整合は InProc シーム＝アダプタ側で担保し、脳側公開面は将来課題に留まる）。

### Option C: ハイブリッド（脳機構は借用移設せず、共有部分だけを薄 lib 化）
- **構成**: 移設は**最小限**に留める。(1) InProc アダプタが必要とする「`IShiori` を codec で駆動する純ロジック」を新 lib（例 `shiori4-inproc`）へ切り出し（ShioriSession の単一 in-flight 部分は要件が要求する範囲＝要件 7.4 最小のみ移植）。(2) cdylib testdll は Option B 同様に自給（脳は借りない）。(3) `ShioriHostSink` は「テスト用最小 host」を新 lib へ薄く新設（bin の ShioriHostSink は移設せず併存）。
  - つまり **DLL＝自給（B 流）** ＋ **アダプタ/host＝新薄 lib（A の縮退）**。
- **トレードオフ**: ✅ bin 影響最小＋アダプタは lib 化されて areka-ghost/将来 M2 の双方から使える。 ✅ 「正規実装（要件 3.6）」を lib 公開面で満たしつつ、bin の既存資産を壊さない。 ❌ ReferenceBrain/ShioriHostSink と「似て非なる」実装が二重化する懸念（設計で「なぜ bin 版を移設せず薄 lib を新設するのか」の正当化が要る）。 ❌ 新 crate 数が最多（testdll＋inproc-lib）。

> いずれの Option でも **fixture（データ）と e2e（テスト）は共通**で、差分は「脳/アダプタ/host がどの crate に居るか」だけ。fixture は Option 非依存で先行設計可能。

---

## 4. 工数・リスク（Effort / Risk）

| 単位 | Effort | Risk | 一言根拠 |
|---|---|---|---|
| cdylib テスト DLL（台本脳＋`shiori_factory`） | **M** | Medium | パターンは testdll/reference_brain に既在だが、COM `#[implement]`＋cdylib 出力＋台本 ID 分岐は新規組み合わせ。 |
| リファレンス脳機構の carve-out（Option A のみ） | **L** | High | bin→lib 移設は影響半径大・既存 in-crate テスト再配線・main.rs 経路波及。Option B/C なら回避/縮退。 |
| `ShioriWiring::InProc`＋LoadLibrary ロード経路 | **S–M** | Medium | `shiori_proxy.rs`（flat-C）が精密テンプレ。差分は COM シンボル/署名＋半構築非露出＋`!Send` 座の扱い。 |
| `IShiori`→`ShioriBackend` アダプタ（両端 codec 挟み込み） | **M** | Medium | codec・写像規律は既在（Shiori3Client）。HSTRING↔bytes 変換と status 写像・Drop teardown が新規判断。 |
| fixture（descript＋最小 shell＋PNG＋DLL パス解決） | **S–M** | Medium | データは軽量だが「DLL 絶対パスを boot へどう渡すか」と「shell/PNG の実消費範囲」が未確定（D-1/要件 4.2）。 |
| boot→talk→close 決定論 e2e | **M** | Medium | spine S1/S5 の技法流用可。差分は実 DLL 境界＋交信列観測点（D-3）。 |
| 既存 smoke の仕分け（要件 6.3） | **S** | Low | 判断（残置/併存/乗換）を残すのみ・コード変更は最小。 |

全体感: **L（1〜2 週）** 級。Option A を採れば carve-out で上振れ、Option B/C なら M〜L に収まりやすい。

---

## 5. 設計フェーズへの申し送り（Research Needed／設計判断項目）

以下は要件討議・設計フェーズで解く論点（番号は討議入力用）。

> **【要件討議#1 決着（2026-07-18）】** 台本源とfixture方針が確定＝**ゴールデンスナップショット録画再生方式**: 実 emo2 `pasta.dll` の出力（正典イベントごとの実応答さくらスクリプト）を env-gate 実 pasta 経路（要件6.1）で一度観測・凍結し、テストDLLは静的 fixture として replay する。fixture の shell/balloon は emo2 実物を流用（脳のみ差替＝非決定論は SHIORI に局在）。常設ゲート深度は cue sink 受領レベル・実描画は流用資産上の opt-in 追加。これにより **D-5（fixture実消費範囲）＝流用で確定**・**D-4（台本content解析範囲）＝ID行のみ読み対応スナップショットを返す方向で確定**（残る設計詳細＝スナップショットの格納形式・採取ハーネスの自動化度は設計フェーズ）。

- **D-1（DLL パス解決）[未知]**: `InProc` に何を渡すか＝(a) 明示 `dll_path` 絶対パス／(b) `mount.shiori.dir.join(file)` を load 元にしテストが cdylib を fixture へコピー／(c) env 逃がし。`resolve` は file 名のみ返し、cdylib は `target/` に出る現実との整合が核心。`CARGO_BIN_EXE_*` は bin のみ・cdylib は同等 env が無い点に注意（テストからビルド済み cdylib パスをどう得るか＝**cdylib のテスト時ビルド保証も併せて要研究**：workspace member であっても `cargo test` が依存として自動ビルドするとは限らない＝`[dev-dependencies]` に cdylib を積む／build script／明示ビルド前提のいずれか）。
- **D-2（carve-out の是非と範囲）[未知]**: Option A/B/C の選択。「reference 脳を種に活用」（brief）と「areka bin-only・in-crate テスト」（記憶）の緊張をどう解くか。M2 native 脳の前方整合（要件 7.1）を脳側公開面で担保するか、InProc シーム側だけで足りるか。
- **D-3（SHIORI 交信列の観測点）[討議#2 決着＝(c)]**: 要件 1.4 は「送出イベント id・NOTIFY/GET・順序」を assert 可能にせよと要求。**決着＝`ShioriBackend` seam に記録デコレータ `Recorder<B: ShioriBackend>` を噛ませる方式**（InProc 実DLL backend と Custom/ScriptedShioriBackend fake の双方で同一手口・cue sink 記録装置と対をなす）。理由＝演者ごとに記録機構を増やさず判断分岐のみを檻に入れる／既存 spine e2e の `RecordedCall`・`RecordingSink` と観測面が揃う。残る設計詳細＝`Recorder` の公開 API 形状・InProc backend をデコレータで包む結線（`Custom` クロージャで InProc backend を構築し Recorder でラップする案 等）は設計フェーズ。（不採用: (a) InProc 専用フック＝再利用性低・(b) テストDLL受領ログ＝境界の向こう側観測で Custom と手口が割れる。）
- **D-4（台本脳の content 解析範囲）[制約→判断]**: 要件 2.5「独自スキーマを発明しない」と要件 2.2「イベント別固定応答」の両立線＝ID 行のみ読む（testdll と同型）。OnBoot の References（shell 名等）を台本が参照するか、ID だけで分岐するかを確定（spine S1 は OnBoot Ref0＝shell 名を送る）。
- **D-5（fixture の実消費範囲）[未知]**: 要件 4.2「過不足なく boot→talk→close が消費する要素」＝cue sink 記録止まりなら surfaces/PNG は最小ダミーで足りるが、seriko/emo の実描画まで踏むなら実 PNG 要。e2e がどこまでの層を貫くかで決まる。
- **D-6（InProc の `!Send`・スレッド座）[制約]**: connect closure は shiori アクタースレッド上で一度だけ実行（Helper の `!Send` window と同座）。ロード済み HMODULE＋COM objects（`IShiori`/`IShioriHost`）はスレッドアフィニティを持つ＝backend が `!Send` で良い（`spawn_shiori_actor` の closure 内 move で完結・既存 `ShioriConnection` と同じ扱い）。COM アパートメント（MTA/STA）前提の要否も確認（記憶 areka-wuc-runs-on-mta-thread＝areka は MTA）。
- **D-7（unload/status 写像）[制約→判断]**: InProc backend の `status()`＝常時 `Running`（要件 3.3）／`unload()`＝Drop teardown→`Ok(ExitKind::Clean)`／接続失敗＝`Err(String)`→`ShioriDown`。この写像で spine S2/S3（接続失敗・死活）シナリオが InProc でも自然に説明できるかを確認。
- **D-8（新 crate 命名）[軽微]**: brief 候補 `shiori4-testdll`（cdylib）。出力 DLL 名（`[lib] name`）と fixture の `shiori,<name>` の一致規約を確定。testdll（i686・`shiori.dll`）と名前衝突しないこと。

---

## 6. 分析サマリ（3–5 点）

- **既存パターンは大半が既在**: IShiori ABI／x64 codec（build_request/parse_response）／in-proc IShiori 駆動（ShioriSession）／LoadLibrary テンプレ（shiori_proxy）／決定論 e2e 駆動技法（spine S1〜S6）／32bit 決定論 testdll（x64 版の兄弟見本）が揃う。**ゼロから作る要素は少ない**。
- **隠れた最大工数は「bin-only 問題」**: 種となる reference 脳・host sink・session は bin-only `areka` 内に居り、cdylib/areka-ghost から借りられない。carve-out するか（Option A・影響大）、testdll/アダプタを自給するか（Option B/C・重複懸念）が本 spec の設計分水嶺（D-2）。
- **主な欠落**: (1) cdylib テスト DLL＋台本脳（ID 分岐＝reference のエコーとは別種）、(2) `ShioriWiring::InProc`＋x64 COM ロード経路＋`IShiori`→`ShioriBackend` アダプタ（両端 codec 挟み込み・status/unload 写像）、(3) マウント可能 fixture、(4) boot→talk→close 決定論 e2e。
- **未確定の要研究**: DLL 絶対パスの boot への渡し方とテスト時 cdylib ビルド保証（D-1）、SHIORI 交信列の観測点（D-3）、fixture の実消費範囲（D-5）。
- **要件 6 の共存はほぼ自動達成**、ただし emo2 smoke は実は SHIORI 境界を決定論的に踏んでおらず（wire 成立止まり）、本 spec の e2e とは目的が別＝「残置＋追加」が素直な仕分け仮説。
