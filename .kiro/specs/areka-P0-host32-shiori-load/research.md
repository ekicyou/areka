# ギャップ分析 (Gap Analysis): areka-P0-host32-shiori-load

> 対象: 確定済み `requirements.md`（要件 1〜6）と既存コードベースの実装ギャップ。
> 本書は **情報提示（options）** であって最終決定ではない。設計判断項目は末尾に列挙し、要件ディスカッションへ供給する。
> 言語: ja（spec.json.language）。日付: 2026-07-02。

---

## 1. 分析サマリ（3〜5 点）

- **上流 transport は完全に凍結・利用可能**: `shiori-host32-ipc`（`MsgTag{Hello,Load,Request,Response,Unload}`・`send_copydata`/`send_request`/`ResponseSlot`/framing）、`shiori-host32-host`（`ProcessHost::spawn`/`ParentMessageWindow`/HELLO handshake/再入 RESPONSE）は完動。**本ユニットは transport を一切改変せず、helper 側で `MsgTag::Load` を結線し echo stub を差し替えるだけ**。seam は `crates/shiori-host32-helper/src/main.rs:54-56` の `fn respond(req)->req.to_vec()` と `handle_message()`（現状 `MsgTag::Request` のみ処理）。
- **欠落能力は helper 内の FFI プロキシ一式**: helper には `LoadLibraryW`/`GetProcAddress`/`GlobalAlloc`/charset(`WideCharToMultiByte`) の足場が **皆無**。`ShioriByteProxy`（モジュールハンドル所有＋3 fn ポインタ保持＋HGLOBAL 所有権規約＋ANSI 符号化）を新設する必要がある。加えて helper の `Cargo.toml` に windows crate の FFI feature（`Win32_System_LibraryLoader`/`Win32_System_Memory`/`Win32_Globalization`）が未追加＝依存ギャップ。
- **知見 donor は検証済みだがコピペ禁止**: pilot `crates/pilot/examples/shiori-host-32/shiori_proxy.rs` が FFI シーケンス・所有権・charset 非対称を実証済み（go 済 2026-07-01）。**production クレートは `crates/pilot` へ inbound 依存禁止（葉ノード隔離）**＝知見のみ参照し一から掘り直す。
- **fixture は worktree に存在するが供給経路が未確定**: 実 emo2 `pasta.dll`（3.4MB・PE i686）は `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/pasta.dll` に**実在**。ただし host32 crate テストがこのパスを参照すると葉ノード隔離違反。共有 fixture の置き場／取り込み方が **真のギャップ**（設計判断 c）。
- **リサーチフラグ（build health）**: `vendors/pasta` submodule が **本 worktree で未展開（空ディレクトリ）**。かつ workspace `Cargo.toml` の `[patch.crates-io] pasta_core = { path = "vendors/pasta/crates/pasta_core" }` が空 path を指すため、**workspace 全体 cargo が失敗しうる**。`git submodule update --init` が前提（[[Harness shell quirks]] と整合）。ABI バイト正確源（`pasta_shiori/src/windows.rs`）も未検証状態＝pilot README 学び #4 の記録に依拠するしかない。

---

## 2. 現状調査（Requirement → Asset マップ）

| Requirement | 必要な技術要素 | 既存資産 | ギャップ種別 |
|---|---|---|---|
| **R1** LOAD メッセージの結線（helper 受領） | `handle_message()` に `MsgTag::Load` 分岐追加。LOAD payload から dll 所在＋ghostdir を取り出す。framing 不整合は crash させず記録のみ。transport 凍結 seam 不改変。 | `handle_message()`／`classify_inbound()`（`main.rs:79-176`）が既存。現状 `Load` は `InboundAction::IgnoreKnown(Load)`（`main.rs:308` テストが「無視」を固定）。`copydata_payload` で framing 検証済み。 | **Constraint**（既存分類器を拡張）＋**Missing**（Load 分岐の実処理・payload デコード） |
| **R2** pasta.dll ロード＋3 エクスポート解決（ShioriByteProxy） | `LoadLibraryW`→`GetProcAddress`×3→`transmute` で cdecl fn ポインタ保持。モジュールハンドル所有。ロード/解決失敗は crash させず失敗返却。`request`/`unload` は解決のみ・呼ばない。 | helper に FFI 足場 **皆無**。pilot `shiori_proxy.rs:79-101`（`load_dll`）が完全実証（知見のみ）。 | **Missing**（新規 `ShioriByteProxy` 型・helper Cargo feature 追加） |
| **R3** load(ghostdir) 呼出＋charset 符号化 | ghostdir を ANSI(CP_ACP/Shift_JIS) へ `WideCharToMultiByte`→`GlobalAlloc(GMEM_FIXED)`→`load(h,len)`→Rust `bool`(1byte) 解釈。入力 HGLOBAL は DLL 解放（二重解放禁止）。 | pilot `shiori_proxy.rs:106-120`（`shiori_load`）＋`ansi_encode`（`186-221`）＋`global_alloc_copy`（`172-184`）が実証（知見のみ）。 | **Missing**（helper に新規実装） |
| **R4** load 結果の親への ack | bool 結果を親へ返送。親が成功/未成功を判別・crash せず観測。REQUEST/RESPONSE ワイヤ形式を改変しない範囲で。 | `send_copydata`/`ParentMessageWindow` は `Response` を再入受領し `ResponseSlot` へ store（`parent_window.rs:188-194`）。親側は `Load`/`Request`/`Unload` を `IgnoreKnown`。 | **Constraint**（凍結ワイヤ上で ack をどう載せるか＝設計判断 b）＋**Missing**（helper→親 ack 送出・親側判別） |
| **R5** 実バイナリ E2E ロード観測 | 実 i686 helper＋実 emo2 pasta.dll で `load` 成功（true）・無 crash を E2E 観測。teardown の courtesy unload/FreeLibrary は許容。 | `tests/echo_roundtrip.rs` が実 helper spawn＋実 WM_COPYDATA 往復の型を確立（helper exe 解決・HelperGuard・poll_exit_kind 無 crash 確認）。fixture pasta.dll は worktree に実在（pilot 配下）。 | **Missing**（LOAD E2E テスト新規）＋**Unknown**（fixture 供給経路＝設計判断 c）＋**Constraint**（teardown 範囲＝設計判断 d） |
| **R6** 32bit 可搬性＋ビルド健全性 | i686-pc-windows-msvc でビルド・実行。dwData/ULONG_PTR は u64 評価。cdecl `extern "C"` で 3 シグネチャ整合。pilot へ inbound 依存なし。 | ipc/helper は既に i686 でビルド・往復実証済（README 学び #2/#3）。`copydata_payload` は `(dw_data as u64) & 0xFFFF_FFFF`（`lib.rs:130`）で overflow 回避済。 | **Constraint**（既存規約を踏襲）＋**Missing**（cdecl fn 型定義・PowerShell ビルド前提の遵守） |

### 既存アーキテクチャの規約（踏襲すべき）

- **純ロジック分離パターン**: `classify_inbound(dw_data, declared_len, data) -> InboundAction` の enum で「窓・FFI から切り離した純関数」を単体テストし、WndProc は結果を見て副作用を行う。**Load 分岐もこの `InboundAction` に新バリアントを足す形が整合的**（`main.rs:63-85`）。
- **観測カウンタ**: `HelperShared` の `Cell<u64>` 群（hellos_sent/requests_handled/responses_sent/bad_frames）。load 観測もこの型に足すのが自然。
- **RAII**: `Window<S>` が Drop で `DestroyWindow`。pilot `ShioriEntries` は Drop で `FreeLibrary`。teardown の courtesy unload/FreeLibrary は Drop に載せられる（設計判断 d）。
- **エラー方針**: transport は「一様な失敗報告」（`IpcError::Timeout`/`SendFailed`/`CorruptFrame`）。proxy の失敗（LoadLibraryFailed/EntryNotFound/LoadFailed/EncodeAnsiFailed/GlobalAllocFailed）は crash せず観測可能な失敗として扱う（pilot `ProxyError` 参照・要件 2.3/2.4/4.4）。
- **依存方向**: helper は `shiori-host32-ipc`（proto）のみ一方向依存。host へも pilot へも依存しない。

### 確定 ABI（pilot README 学び #4・shiori_proxy.rs の記録に依拠）

- `load(hdir: HGLOBAL, len: usize) -> bool`（Rust bool 1byte・Win32 BOOL でない）
- `unload() -> bool`
- `request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`（len は in/out）
- HGLOBAL=`GlobalAlloc(GMEM_FIXED=0)`（生ポインタ＝ハンドル・GlobalLock 不要・IPC を跨がない）
- 入力 HGLOBAL は DLL(callee) 解放／request 応答 HGLOBAL はホスト解放
- charset 非対称: `load` の dir は **ANSI(CP_ACP/Shift_JIS)**・`request` は UTF-8（本ユニットは load のみ）
- シンボルは `#[unsafe(no_mangle)] pub extern "C"` ＝装飾なし C 名（`GetProcAddress(b"load\0")` 等）

> ⚠ **ABI 再確認フラグ**: 上記は pilot README＋`shiori_proxy.rs` コメントの二次記録。一次源 `vendors/pasta/crates/pasta_shiori/src/windows.rs`（行番号 50/63/76 等）は **submodule 未展開のため本分析では未検証**。設計フェーズで `git submodule update --init` 後にバイト正確を再確認すること。

---

## 3. 実装アプローチの選択肢

### アプローチ A: 既存 helper へ ShioriByteProxy を新規モジュール追加（推奨・Option B 寄り）

`crates/shiori-host32-helper` 内に `shiori_proxy.rs`（新ファイル）を追加し、`main.rs` の `handle_message()`/`classify_inbound()` に Load 分岐を結線する。

- **構成**:
  - 新 `mod shiori_proxy`: `ShioriByteProxy`（module handle＋3 fn ポインタ）／`ProxyError`／`ansi_encode`／`global_alloc_copy`。`unsafe` 境界をこの型に集約（pilot §489 の方針を踏襲）。
  - `main.rs`: `InboundAction` に `LoadDll(LoadPayload)` バリアント追加。`classify_inbound` が `MsgTag::Load` を新バリアントへ写像。WndProc が proxy を構築・`load` 呼出・bool 結果を親へ ack 送出。
  - proxy を helper のプロセス生存期間で保持（`HelperShared` に `RefCell<Option<ShioriByteProxy>>` 等）＝下流 request/lifecycle が載る常設プロキシの足場。
- **トレードオフ**:
  - ✅ 既存の純ロジック分離パターン・観測カウンタ・RAII をそのまま活かせる。
  - ✅ helper 単一クレート内に閉じ、依存方向（ipc のみ）を維持。葉ノード隔離を自然に満たす。
  - ✅ 下流ユニット（request/lifecycle）が同 proxy を共有できる足場になる。
  - ❌ helper Cargo.toml へ windows feature 3 種追加が必要（意図的依存追加）。
  - ❌ `main.rs` が肥大化する懸念 → proxy は別ファイル分離で緩和。

### アプローチ B: proxy を独立クレート `shiori-host32-proxy` として切り出す

FFI プロキシを新クレートにし、helper がそれを依存する。

- **トレードオフ**:
  - ✅ proxy の単体テストがクレート境界で明確。下流でも import しやすい。
  - ❌ **i686 専用クレートを workspace に足すと、x64 ビルド時に扱いが煩雑**（cfg/target 分離が必要・[[areka マルチアーキ]] の「crate 境界で分離」原則はあるが、proxy は helper と同じ i686 なので helper 内で足りる）。
  - ❌ 現時点で共有の必要が薄い（YAGNI）。brief も「trait 抽象は設けない・YAGNI」を明言。下流 request/lifecycle が同 helper 内に載るなら分離不要。
  - ❌ 過剰設計リスク（spec 工場回避の教義に反する）。

### アプローチ C: ハイブリッド（proxy は helper 内・fixture は共有クレート/ディレクトリへ）

proxy 実装はアプローチ A、**fixture 供給だけを別立て**にする。

- **構成案（fixture 供給・設計判断 c の具体化）**:
  - (c-1) test fixtures 専用ディレクトリ（例 `crates/shiori-host32-helper/tests/fixtures/emo2/` もしくは workspace ルート `fixtures/emo2/`）へ pasta.dll を配置し、pilot 配下とは別に管理。
  - (c-2) 環境変数（例 `HOST32_PASTA_DLL` / `HOST32_GHOSTDIR`）でテスト時にパスを注入（`echo_roundtrip.rs` の `HOST32_HELPER_EXE` 方式と同型）。fixture をリポジトリに二重取り込みしない。
  - (c-3) build script で pilot fixture を target 配下へコピー（ただし production→pilot 参照になり葉ノード隔離のグレーゾーン＝非推奨）。
- **トレードオフ**:
  - ✅ 葉ノード隔離を守りつつ実バイナリ E2E を可能にする。
  - ✅ 環境変数注入（c-2）は既存 `echo_roundtrip.rs` の実績パターンと一貫。
  - ❌ fixture の二重管理（c-1）か、CI での env 設定手間（c-2）が発生。

**推奨の方向性（決定ではない）**: proxy 実装は **アプローチ A**（helper 内・別ファイル・trait なし）。fixture は **C の c-1 か c-2**（葉ノード隔離を厳守できる方）を要件ディスカッションで確定。

---

## 4. 実装複雑度とリスク

- **Effort: M（3〜7 日）**
  - 根拠: FFI プロキシ・charset・HGLOBAL 所有権は pilot で実証済み（知見あり）＝アルゴリズム的に既知だが、helper への新規結線・i686 ビルド往復・LOAD wire 契約設計・fixture 供給・E2E テスト整備が絡む。純粋な拡張より広い。
- **Risk: Medium**
  - 根拠（High でない理由）: 跨ビットネス往復・i686 での wintf 稼働・pasta FFI 駆動は pilot で go 済（設計最大の賭けは成立済み）。
  - 根拠（Low でない理由）:
    - `unsafe` FFI（transmute・生ポインタ・二重解放禁止）の正しさは production 品質で再実装する必要がある。
    - **ABI 一次源（vendors/pasta）が本 worktree で未検証**＝二次記録依拠のリスク（要 submodule 展開）。
    - fixture 供給の葉ノード隔離を破らず E2E を通す設計が非自明。
    - LOAD wire payload 契約（dll path/ghostdir をどこに載せ何で符号化するか）が凍結 seam の上に新規セマンティクスを載せる＝設計要注意。

---

## 5. リサーチフラグ（設計フェーズへ持ち越す "Research Needed"）

1. **vendors/pasta submodule 展開**: 本 worktree では空。`git submodule update --init` を実行し、`crates/pasta_shiori/src/windows.rs` で `load`/`unload`/`request` の ABI（バイト正確・シンボル名・戻り値型）を一次確認する。未展開だと `[patch.crates-io] pasta_core` の path 解決失敗で workspace cargo が壊れうる（build health）。
2. **helper Cargo.toml の windows feature 追加**: `Win32_System_LibraryLoader`（LoadLibraryW/GetProcAddress）・`Win32_System_Memory`（GlobalAlloc/GlobalFree）・`Win32_Globalization`（WideCharToMultiByte/CP_ACP）が現状未宣言。追加が必要（意図的依存追加として設計で明記）。
3. **i686 ビルド前提の遵守**: PowerShell 必須（Git Bash の GNU link.exe トラップ）。`usize`=32bit ゆえ dwData/ULONG_PTR 演算は u64 評価（`copydata_payload` は既遵守）。共有モジュールは `cargo test --target i686-pc-windows-msvc` も回す。
4. **pasta.dll の内部前提を置かない**（README 学び #5）: 依存してよいのは観測可能な契約（`load` の同期 bool 返却・無 crash）のみ。内部スレッド等の仮説には依存しない（要件 5.3 が明文化）。
5. **LOAD payload の wire 符号化**: dll path/ghostdir を UTF-8 か UTF-16 で wire に載せ、helper が各 API 用に何へ transcode するか（`LoadLibraryW`=UTF-16／`load` dir=ANSI）。`ProcessHost::spawn` が既に ghostdir を helper の cwd＋起動引数として運ぶため、dll path/ghostdir を **spawn 引数と LOAD payload のどちらに載せるか** を含む（設計判断 a）。

---

## 6. 設計判断項目（要件ディスカッションへ供給）

> 以下は「情報と選択肢」であり最終決定ではない。requirements.md/spec.json は改変しない。

1. **(a) load 入力の運び先 — ✅ 決着（2026-07-02・要件ディスカッション #1）**
   - **決定**: load 入力（**load_dir と SHIORI 名**）は helper の**プロセス起動パラメーター（明示的コマンドライン引数＋env fallback）**で供給する。`spawn` がこれらを渡す（`parent_hwnd` と同じ arg＋env 規約）。**LOAD メッセージは wire でパスを運ばず、ロード実行のトリガに純化**する。→ requirements R1.2/R1.5・R2.1・R3.1・Boundary 反映済み。
   - **根拠（ukadoc 正典）**: SHIORI は `descript.txt` の `shiori,<ファイル名>`（既定 `shiori.dll`・`descript_ghost#shiori,ファイル名`）で**名前を与えられる**存在。`pasta.dll` はその一例。ゆえに起動引数に load_dir だけでなく **SHIORI 名**が必須。名前解決（descript 参照）は親／`package-mount` の領分で、helper は名前を受け取るのみ。
   - **現状訂正**: 本 §6 初版は「ghostdir は既に spawn が cwd＋arg で運んでいる」と記したが、実コード（`process_host.rs:130-135`）では **ghostdir は `current_dir` のみ・arg 化されていない**（arg1＋env は `parent_hwnd`）。よって cwd 依存をやめ load_dir を明示 arg 化するのが本決定の実装差分。
   - **spawn 契約拡張**: `spawn(helper_exe, ghostdir, parent_hwnd)` → load_dir と SHIORI 名を明示 arg（＋env fallback）で追加。これは上流 `shiori-host32-host` の**起動パラメーター契約の拡張**であり、凍結された WM_COPYDATA wire/framing/`MsgTag` 定義は改変しない。
   - **transcode（helper 内）**: `LoadLibraryW`＝UTF-16（`load_dir\<SHIORI名>` を結合）／`load` の dir 引数＝ANSI(CP_ACP)。wire 符号化論点は消滅（パスは wire を通らない）。起動引数の Rust `OsString`/`PathBuf` からそのまま各 API へ変換。

2. **(b) load-ack の形（凍結 REQUEST/RESPONSE ワイヤを改変しない）**
   - 前提更新（(a) 決着の影響）: パスは起動パラメーターで供給済みゆえ、**LOAD は payload を持たない（空）トリガ**になった。残る論点は「トリガの送出手段」と「bool 結果 ack の載せ方」のみ。
   - 論点: `load` の bool 結果を親へどう返すか。transport の framing/MsgTag は改変不可。
   - 選択肢: (i) `MsgTag::Response` に 1 バイト（0/1）を載せて既存の再入 RESPONSE 経路（`ResponseSlot`）で受ける＝新タグ不要・`send_request`(REQUEST→RESPONSE) の往復にそのまま乗る。(ii) LOAD を `send_request` 相当で送り、その RESPONSE を ack とみなす。
   - 現状: 親側 WndProc は `Response` を `StoreResponse`→slot へ store（`parent_window.rs:188`）。`Load` は `IgnoreKnown`。**LOAD を親が送る手段**（`send_request(MsgTag::Load, ...)` を許すか）も論点＝親側 send は transport の `send_request` が任意タグを取れる（`parent_window.rs:320`）ため追加なしで可能。

3. **(c) test-fixture の供給 — ✅ 決着（2026-07-02・要件ディスカッション #2）**
   - **決定**: 本物 emo2 `pasta.dll`（3.4MB）を主 fixture にせず、**host-32 トラックが所有する自作の最小 SHIORI DLL fixture**（i686 cdylib・flat-C ABI 実装・既定名 `shiori.dll`・数KB）を**主役**にする。本物 `pasta.dll` は**任意・env-gated（`HOST32_PASTA_DLL`）の confidence** に格下げ（CI 必須ゲートにしない）。→ requirements R5 全面改訂で反映。
   - **根拠**: このユニットの観測は「`load`→bool・無クラッシュ」のみで、helper の FFI パスは DLL 中身に非依存＝簡易 DLL で同一パスを網羅。本物 `pasta.dll` の実ロード可否は先進坑 `pilot-shiori-host-32` が go 済（2026-07-01）＝耐力壁は突破済み。ロードマップ done-line「✔ load 成功・無crash」も本物指定なし。
   - **利点**: ① fixture 肥大なし（数KB）② `crates/pilot` 非参照＝**葉ノード隔離を自然遵守**（旧 c-1/c-2/c-3 のジレンマ消滅）③ `load`→`false`／ロード失敗を故意に模擬でき、**R2.3/R2.4/R4.4 の失敗パスを決定的にテスト化**（本物 DLL では困難）④ `request`/OnBoot 最小対応を持たせれば下流 `host32-request`/`-lifecycle` が再利用できる共有資産。
   - **design 送りの実装論点**: i686 cdylib を workspace にどう組むか（helper 同様の i686 専用ビルド扱い・別 crate/example/`build.rs` で C dll 生成 等）、fixture ghost dir の最小構成（`shiori.dll`＋必要なら `descript.txt`）。本物 pasta の任意検証は env 注入（設定済みで欠落なら明示 fail・無言スキップ禁止）。

4. **(d) teardown unload/FreeLibrary を本ユニットの検証で行使するか**
   - 論点: load 成功観測後、テスト後始末で `unload`/`FreeLibrary` を呼ぶか、load-only 観測に留めるか。常駐 lifecycle は下流所有を維持。
   - 選択肢: (i) courtesy `unload`＋`FreeLibrary` を `ShioriByteProxy` の Drop に載せる（pilot `ShioriEntries` の Drop 方式）＝プロセス/DLL リーク防止・RAII 一貫。(ii) load-only 観測に留め teardown はプロセス終了に委ねる（helper プロセスごと落とす）。
   - 制約: 要件 5.4 は courtesy を「許容だが常駐 lifecycle の所有ではない」と明記。`unload` fn ポインタの**解決**は本ユニット（要件 2.6）。恒常呼出は下流。

---

## 7. 次ステップ

- 上記リサーチフラグ（特に vendors/pasta 展開と ABI 一次確認）と設計判断 a〜d を要件ディスカッションで詰める。
- 確定後 `/kiro-design areka-P0-host32-shiori-load` で設計文書を生成する。
