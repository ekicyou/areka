# Gap Analysis: areka-P0-host32-request

> 調査日: 2026-07-03 / 対象: 確定済み requirements.md（7 要件）＋ brief.md ＋ steering ＋ 既存コード深掘り
> 種別: 本坑（main）① shiori トラック第3ユニット（pilot✅ → host32-ipc✅ → host32-shiori-load✅ → **本ユニット** → host32-lifecycle）
> 位置づけ: gap 分析は「情報提供・複数案提示」に徹する。最終決定は要件ディスカッション／design に委ねる。

## 1. サマリ（3–5行）

- **上流資産は request 往復まで完全に貫通済み**。凍結境界 `shiori-host32-ipc` は `MsgTag::Request/Response` ＋ `send_request()`（再入 RESPONSE・`ResponseSlot`・`SMTO_ABORTIFHUNG`）を実装済み。host 側 `ParentMessageWindow::send_request(tag, payload, timeout)` も稼働。**本ユニットは新規 transport を一切書かず、既存 seam の上へ (a) helper の request 実呼出、(b) x64 純 codec、(c) 出口 API、(d) testdll fixture 拡張の4点を増分する**だけで足りる。
- **欠落は明快に2箇所**。① helper `main.rs::respond()` が echo スタブ（`req.to_vec()`）のまま＝`ShioriByteProxy` が保持する `request` fn を呼んでいない（`shiori_proxy.rs:66/93` に「下流 host32-request が消費」と TODO 明記・`request` フィールドは `#![allow(dead_code)]` 下で保持のみ）。② x64 側に SHIORI/3.0 codec（build/parse）が皆無。
- **donor 知見が揃っている**。pilot `shiori3.rs`（`build_onboot`/`parse_value`）と pilot `shiori_proxy.rs::shiori_request`（in/out `len`・応答 HGLOBAL caller-free）が SHIORI/3.0 ワイヤと HGLOBAL request 契約の実証済みミニチュア。**コピペ禁止・知見参照のみ**（二坑規律・`crates/pilot` へ inbound 依存禁止＝R7.4）。
- **HGLOBAL 所有権契約は request 経路では未実地検証**。load 経路は callee-free で確立済みだが、request は「入力＝callee-free／応答 HGLOBAL＝caller-free（`GlobalFree`）」の非対称契約が新規に効く。testdll fixture が現在 null 返却ゆえ、応答 HGLOBAL を新規 `GlobalAlloc(GMEM_FIXED)` で返す実装へ拡張しないと caller-free 経路が検証できない。
- **最大の未確定は「codec を IShiori へどう装着するか」**。要件は出口 API の**形**（GET=応答待ち/NOTIFY=投げきり・`Send` 引数戻り・内部型非露出）を固定するが、装着方式（`IShiori::Get` 実装として host32 互換 backend factory に載せるか、当面は plain な host メソッドに留めるか）を design 判断へ明示委譲している（Boundary Context「IShiori 装着」）。

## 2. 現状調査（既存資産の棚卸し）

### 2.1 凍結境界 `shiori-host32-ipc`（`crates/shiori-host32-ipc/src/lib.rs`・改変禁止＝R7.1）

- `MsgTag { Hello=1, Load=2, Request=3, Response=4, Unload=5 }`（低32bit占有・跨ビットネス安全）。**Request/Response は定義・実装済み**。
- `copydata_payload()` framing 検証（未知タグ・長さ不整合を `FramingError` で観測）。
- `send_copydata()`（片道 `SMTO_ABORTIFHUNG`＋timeout）／`send_request()`（`slot.clear → send → slot.take`・single-in-flight）。
- `ResponseSlot`（`RefCell<Option<Vec<u8>>>`・clear/store/take）／`IpcError { Timeout, SendFailed, CorruptFrame }`。
- **ペイロードは不透明バイト列**。本ユニットは SHIORI/3.0 バイト列をこのペイロードに載せるだけ＝wire/framing/timeout を一切触らない。

### 2.2 host 側 `shiori-host32-host`（x64/arm64・codec 追加先）

- `parent_window.rs`: `ParentMessageWindow`。`send_request(tag, payload, timeout) -> Result<Vec<u8>, SendError>` が**ハンドシェイクゲート下の1往復**を提供済み（`SendError { Handshake, Ipc }`）。RESPONSE は WndProc の `StoreResponse` アームが再入 store。**本ユニットの GET 出口 API はこの `send_request(MsgTag::Request, <SHIORI/3.0 bytes>, timeout)` を呼び、返送 `Vec<u8>` を codec で parse する**構図。
- `process_host.rs`: `spawn`（arg/env 二重供給）・`poll_exit_kind`・`LOAD_ACK_TIMEOUT=30s`。request 用 timeout 定数はここに追加候補（load ack と別建ての先例あり）。
- `error.rs`: `SpawnError`/`HandshakeError`（複数タスクが単一責務で追記する共存ファイル・request エラー語彙もここへ追記候補）。
- `lib.rs`: 公開 re-export の集約点。codec モジュール・出口 API の公開はここに追加。
- **codec モジュールは未在**（`build_*`/`parse_*` に相当する x64 純関数群は host クレートに一切ない）。

### 2.3 helper 側 `shiori-host32-helper`（i686・request 実呼出の差替先）

- `main.rs::respond(req) -> Vec<u8> { req.to_vec() }`（**echo スタブ**・R3.1 で置換対象）。`classify_inbound` の `Reply(respond(payload))` から呼ばれ、`handle_message` の `InboundAction::Reply` アームが `MsgTag::Response` で返送。
- `shiori_proxy.rs::ShioriByteProxy`: `request: RequestFn` フィールドを**解決保持済み・未呼出**。`RequestFn = unsafe extern "cdecl" fn(req: HGLOBAL, len: *mut usize) -> HGLOBAL`。`GlobalAlloc(GMEM_FIXED)` ヘルパ `global_alloc_copy` あり（load で実績）。
- **設計上の要点**: `respond` は現在 `HelperShared`（proxy 保持者）に触れない plain fn。request 実呼出には proxy への到達が要る＝`respond` の**シグネチャ／呼び出し文脈の変更**か、`ShioriByteProxy` に `request()` メソッドを新設して `Reply` アームから駆動する**結線変更**が必要（下記 Option 参照）。
- **再入規律の既確立注意点**: `handle_message` の LOAD アームは「`s.proxy` の borrow を `send_copydata`（ブロッキング・再入可）越しに保持しない」規律を厳守済み（validation issue #1）。request 実呼出でも `proxy.borrow()` を FFI `request` 呼出中に保持することは問題ないが（FFI は跨プロセス SendMessage を発しない同期呼出）、その後の RESPONSE 返送（`send_copydata`）へ borrow を持ち越さない同型規律を守る必要がある。

### 2.4 testdll fixture `shiori-host32-testdll`（i686・request fixture 拡張先）

- `load`（入力 HGLOBAL を callee-free・二重解放検出器）／`unload`（marker 書込）は実装済み。
- `request(req, len) -> HGLOBAL`: **現在は入力を callee-free し `null`＋`len=0` を返す stub**（R6.1 で「受領検証＋固定 SHIORI/3.0 response 返却」へ拡張対象）。応答 HGLOBAL を返す経路（`GlobalAlloc(GMEM_FIXED)`＋len 書戻し）が未実装＝caller-free 契約の被検証側が空白。
- 解決規約: env `HOST32_TESTDLL_DLL` → target 探索・**silent skip 禁止**（見つからねば panic）。E2E は `HOST32_PASTA_DLL` env-gated（未設定 skip・指定 DLL 不在は明示 fail）。

### 2.5 pilot donor（`crates/pilot/examples/shiori-host-32/`・知見参照専用・コピペ禁止）

- `shiori3.rs::build_onboot(ghostdir) -> Vec<u8>`: `GET SHIORI/3.0\r\n` ＋ `ID:`/`Charset: UTF-8`/`Sender:`/`SecurityLevel: local`＋空行終端。**イベント固有**（OnBoot 決め打ち）ゆえ本ユニットの**汎用ビルダ**（R1.5「イベント個別知識を持たない」）とは要件が異なる＝donor は書式の実証、本ユニットは ID/Reference 汎用化が追加要件。
- `shiori3.rs::parse_value(response) -> Option<String>`: CRLF/LF 両対応・`Value:` を大小無視抽出。**ステータスコード分岐・ErrorLevel・204 区別が無い**＝本ユニットの R2（200/204/311/312/400/500・ErrorLevel/ErrorDescription・未知ヘッダ寛容）はこれより広い。
- `shiori_proxy.rs::shiori_request(e, req) -> Result<Vec<u8>>`: **request 実呼出の完全実証**。`len` は in/out（入力長を渡し応答長で上書き）・入力 HGLOBAL は callee-free・**応答 HGLOBAL は `hres.0 as *const u8` から `len` バイトコピー後 `GlobalFree(hres)`（caller-free）**。本ユニット helper 実装が写すべき所有権規約の正確な手本（知見のみ）。

### 2.6 shiori-abi（`crates/shiori-abi/src/interface.rs`・是正済み）

- `IShiori::Get(input: &HSTRING, out_response: &mut HSTRING, out_token: &mut u64) -> HRESULT`（即時 `S_OK`＋move-out／遅延 `SHIORI_S_PENDING`＋token）／`Notify(input) -> Result<()>`（片道）。
- `IShioriFactory::CreateInstance(load_dir, shiori_name, host, out) -> Result<()>`（生成＋load 融合・load 済み `IShiori` を move-out）。
- **host-32 互換 backend の factory 実装は未在**（`ReferenceBrain` 相当の native モックのみ）。本ユニット出口 API はこの `Get`/`Notify` 意味論へ**写像可能な形**に切ることが要件（HSTRING⇄バイト列変換はプロセスを跨がない＝HGLOBAL は32bit ローカル・HSTRING は x64 ローカル）。

### 2.7 ukadoc 正典との突合（design 前提事実の確認・2026-07-03 追調査）

- `descript_ghost` の `shiori.encoding` / `shiori.forceencoding` に「**SHIORI 側から Charset ヘッダが返された場合は SHIORI 側が優先**」と明記＝R2.6「response の Charset 省略時は request 側 charset を継承」の正典裏付け（emo2 は UTF-8 固定ゆえ本ユニットは UTF-8 継承で足る）。
- SHIORI/3.0 プロトコル全文（`protocol` カテゴリに存在・GET/NOTIFY・ステータスコード・Sender/Reference・ErrorLevel/ErrorDescription〔SSP拡張〕・X-SSTP-PassThru・SenderType/SecurityOrigin）は **design 冒頭で `search_docs`(category=protocol)→`get_doc` で正典参照**する（gap 分析は書式の網羅転記を行わず design へ送る＝gap-analysis.md「Out-of-Scope: deep research は design へ」）。brief の「必読」節がこの参照先を既に指名済み。

## 3. Requirement → Asset マップ（gap タグ: Missing / Unknown / Constraint）

| 要件 | 必要能力 | 既存資産 | Gap |
|---|---|---|---|
| R1 request 組立（汎用ビルダ） | `GET/NOTIFY SHIORI/3.0`・CRLF・空行終端・`Charset/Sender/ID/Reference0..N`・UTF-8・SJIS シーム | pilot `build_onboot`（OnBoot 固定・知見のみ） | **Missing**（x64 codec 新設・**汎用 ID/Reference 化**が追加） |
| R2 response 解析 | 200/204/311/312/400/500・`Value`・`ErrorLevel`/`ErrorDescription`・CRLF・charset 継承・未知ヘッダ寛容 | pilot `parse_value`（Value 抽出のみ・知見） | **Missing**（ステータス分岐・エラー情報・寛容性が新規） |
| R3 helper request 実呼出 | echo 置換・`GlobalAlloc(GMEM_FIXED)` 入力・callee-free・応答 copy 後 caller-free・RESPONSE 返送 | `ShioriByteProxy.request` 保持済み／`global_alloc_copy`／pilot `shiori_request`（手本） | **Missing**（`respond` 差替＋proxy 結線）＋ **Constraint**（HGLOBAL 非対称契約・再入規律） |
| R4 E2E 結線（出口 API） | GET/NOTIFY 出口 API・`send_request` 越し送出→parse・同期ブロッキング可・`Send` 引数戻り・内部型非露出 | `ParentMessageWindow::send_request` | **Missing**（codec×transport 結線＋API 面）＋ **Unknown**（IShiori 装着方式＝design 判断） |
| R5 エラー語彙 | wire timeout／SHIORI エラー（400/500・ErrorLevel）／helper 死活を区別 | `SendError`/`IpcError`/`ExitKind`（部品あり） | **Missing**（SHIORI エラーを timeout と区別する新語彙・3語彙統合） |
| R6 testdll fixture 拡張＋決定的 E2E | 受領検証＋固定 response・入力 callee-free／応答 `GlobalAlloc` caller-free・helper 越し `Value` 抽出・env-gated 実 pasta | `request` stub（null 返却）／`shiori_load_e2e.rs`（LOAD 版 E2E の型） | **Missing**（fixture 応答経路＋新 request E2E テスト） |
| R7 凍結・隔離・32bit 規律（横断） | ipc 不改変・i686 PowerShell ビルド／test・silent skip 禁止・pilot 非依存・pasta 署名バイト照合・HGLOBAL 契約厳守 | 既存規律が全面確立済み（load 経路が先例） | **Constraint**（既存規律の踏襲・逸脱不可） |

## 4. 実装アプローチ（複数案・A/B/C）

論点は独立した2軸に分かれる。**軸① codec の置き場所／装着**（最大の未確定）と、**軸② helper request 結線の形**。それぞれ案を提示する。

### 軸① codec の置き場所と IShiori 装着（design 判断・要件は「出口 API の形」のみ固定）

#### Option ①-A: host クレートに純 codec モジュール新設＋plain な出口 API（IShiori 装着は後回し）
- **内容**: `shiori-host32-host` に `shiori3` 相当の純関数モジュール（`build_request`/`parse_response`）を新設し、`ParentMessageWindow`（または薄い上位型）に `get(id, refs) -> Result<Option<Value>, HostError>` / `notify(id, refs) -> Result<(), HostError>` を生やす。IShiori 実装（factory）装着は下流／別ユニットへ送る。
- **トレードオフ**: ✅ 最小・純関数で単体テスト容易・凍結境界非接触・R1/R2/R4/R5 を最短で満たす ✅ kanade が求める「イベント→Value」形をそのまま提供 ❌ shiori-abi の `IShiori::Get` へまだ載らない＝将来 native 脳と同一 ABI に統一する結線が別途要る（ただし要件は「写像可能な形」までしか求めない＝適合）。

#### Option ①-B: codec を `IShiori::Get`/`Notify` 実装として host32 互換 backend factory に即装着
- **内容**: `IShioriFactory` を host32 で実装し、`IShiori::Get(input: HSTRING)` 内で HSTRING→UTF-8 バイト→`send_request`→parse→HSTRING の変換を閉じる（SHIORI4⇄SHIORI3 変換＝IShiori 下の x64 過去互換アダプタ・記憶 areka-shiori-layer-naming）。
- **トレードオフ**: ✅ native 脳と同一 ABI に最初から統一・kanade は IShiori 一本で native/32bit を区別せず消費可 ❌ HSTRING⇄バイト変換・factory 生成・`SHIORI_S_PENDING` 型シーム（実装せず塞がない）・COM 実装の重量が本ユニットへ流入＝スコープ肥大。要件は「装着方式は design に委ねる／本ユニットは出口 API の形を固定」ゆえ **B を要件が要求はしていない**（過剰実装リスク）。

#### Option ①-C（推奨方向・hybrid）: 純 codec を独立させ、出口 API は plain で提供しつつ IShiori 写像点を「型シーム」として明示
- **内容**: ①-A の純 codec＋plain 出口 API を実体とし、`IShiori::Get` への写像は「変換関数の署名（`fn onto_ishiori_get(...)`相当）を doc 化・型で示すが実装しない」段階に留める。装着の実体化は下流（kanade 結線時 or lifecycle）。
- **トレードオフ**: ✅ 要件の「写像可能な形／PENDING は型シームのみ」に厳密適合・最小実装規律を守る ✅ 将来 ①-B へ無改修で拡張できる境界を残す ❌ 「型シームだけ」の線引きを design で厳密に定義しないと曖昧化。→ **design で①-A/①-C のどちらを取るか、IShiori 装着をどこまで本ユニットに含めるかを確定する**（要件ディスカッションの主要論点）。

### 軸② helper 側 request 実呼出の結線（R3）

#### Option ②-A: `ShioriByteProxy` に `request(&self, req: &[u8]) -> Option<Vec<u8>>` メソッドを新設し `Reply` アームから駆動
- **内容**: proxy に request メソッドを足し（pilot `shiori_request` の HGLOBAL 非対称契約を知見として写す・コピペ禁止）、`handle_message` の REQUEST 分岐を「proxy 未確立なら echo/エラー・確立済みなら `proxy.request(payload)`」へ変更。`respond` plain fn は撤去 or 縮退。
- **トレードオフ**: ✅ unsafe FFI が `ShioriByteProxy` に一点集約される既存方針と整合（steering unsafe 隔離）✅ load と対称の設計 ❌ REQUEST が proxy 未確立で来た場合の扱い（LOAD 前 REQUEST）を要件化する必要（現状 echo 前提の順序保証が変わる）。

#### Option ②-B: `respond` を「proxy を引数に取る自由関数」へ変え classify から proxy 経由で駆動
- **内容**: `classify_inbound` から proxy 依存を切り離したまま、`handle_message` の `Reply` アームで proxy 呼出を行う（`respond` は純変換をやめ、実呼出は WndProc 側に置く）。
- **トレードオフ**: ✅ `classify_inbound` の純粋性（単体テスト可能性）を維持 ❌ unsafe が WndProc 側へ散る＝一点集約方針とやや不整合（②-A の方が steering 適合）。

> **共通制約（両案）**: 入力 HGLOBAL は callee-free（helper は解放しない）／応答 HGLOBAL は helper が copy 後 `GlobalFree`（caller-free）。RESPONSE 返送（`send_copydata`）へ `proxy.borrow()` を持ち越さない再入規律（LOAD アームの validation issue #1 と同型）。pasta `request` 署名は **`vendors/pasta` を `git submodule update --init` で展開しバイト照合してから固定**（R7.5・現ワークツリーで submodule 未展開＝実装前に展開必須）。

### testdll fixture 拡張（R6・軸に依らず必須）
- `request` stub を「受領 request line/ID を assert 可能に検証（R6.4）＋固定 SHIORI/3.0 200 response（`Value:` 入り）を `GlobalAlloc(GMEM_FIXED)` で確保し len 書戻しで返却（R6.2 caller-free 実地検証）」へ拡張。新 E2E テスト（`shiori_load_e2e.rs` の型を踏襲した `request_e2e.rs` 相当）で helper 越し `Value` 抽出＋所有権往復＋env-gated 実 pasta を観測。

## 5. 複雑度・リスク

- **Effort: M（3–7日）**。新規 transport ゼロ・純 codec と2箇所の結線＋fixture 拡張が主。codec は純関数で単体テスト容易だが、SHIORI/3.0 の寛容パース（未知ヘッダ・複数ステータス・charset 継承）と HGLOBAL request 契約の実地検証（i686 ビルド・E2E）が幅を持つ。
- **Risk: Low–Medium**。技術は既知（donor 実証済み・load 経路が先例）で凍結境界に触れないため Low 寄り。Medium 要因は (1) IShiori 装着範囲の線引き（軸①）が未確定でスコープが振れる、(2) request 経路の HGLOBAL 非対称契約（応答 caller-free）が新規に効き二重解放・リークの窓がある、(3) LOAD 前 REQUEST の順序保証・proxy 未確立時の応答語彙が要件化されていない。

## 6. design へ持ち越す Research/決定事項

1. **[軸①・最重要] codec の IShiori 装着範囲**: ①-A（plain 出口 API・装着は後回し）／①-B（IShiori::Get 即装着）／①-C（plain＋型シーム）のいずれを本ユニットに含めるか。要件は「出口 API の形を固定・装着は design」。→ design 冒頭で確定。
2. **[R1/R2] 送出ヘッダ最小集合と受信寛容集合の2表**: `SecurityLevel`（local/external）・`SenderType`（internal/external/sstp/embed/raise…）・`SecurityOrigin`・`X-SSTP-PassThru` の送出可否／受信 tolerate、応答側 `Reference*`・`Marker` の tolerate、未知ヘッダ寛容の codec 契約明記。→ design 冒頭で `get_doc`(SHIORI/3.0・category=protocol) を読み design.md に2表を載せる（brief 指示）。
3. **[R3/軸②] helper request 結線の形**: proxy メソッド新設（②-A）／WndProc 側駆動（②-B）。および **LOAD 前 REQUEST・proxy 未確立時の応答語彙**（echo 継続か・明示エラーか・R5 との整合）。
4. **[R5] エラー語彙の統合設計**: wire timeout（既存 `IpcError::Timeout`）／SHIORI エラー（400/500・ErrorLevel＝codec 由来の新語彙）／helper 死活（`ExitKind`／`SendError`）を単一 enum で区別保持する形。`error.rs` 共存ファイルへの追記か新 codec エラー型か。
5. **[R6] fixture 応答の固定内容と検証面**: 固定 SHIORI/3.0 response の具体（200＋`Value:` の中身・204 ケースの要否）、request line/ID の assert 方法、caller-free 二重解放検出の仕込み。
6. **[R7.5] pasta `request` 署名のバイト照合**: `vendors/pasta` submodule を展開し `crates/pasta_shiori/src/windows.rs` の `request` 実署名（`len: &mut usize` ≡ `*mut usize`・cdecl・応答 nofree）を再確認して固定（現ワークツリーは submodule 未展開）。
7. **[timeout] request 用 per-call timeout の既定値**: `LOAD_ACK_TIMEOUT=30s` と別建てにするか。GET は脳の思考時間を含むため長め・NOTIFY は投げきり。凍結 timeout 機構は不変（per-call 引数のみ）。
