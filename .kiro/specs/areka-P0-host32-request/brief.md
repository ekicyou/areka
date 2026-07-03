# Brief: areka-P0-host32-request

> **種別**: 本坑（main）。① shiori トラック第3ユニット（逐次チェーン: pilot✅go → host32-ipc✅ → host32-shiori-load✅ → **本ユニット** → host32-lifecycle）。
> **調査日**: 2026-07-03（コード深掘り＋ukadoc 正典）。

## Problem

x64 areka は 32bit `pasta.dll` の load までは貫通した（shiori-load 完了）が、**request が呼べない＝ゴーストが一言も喋れない**。SHIORI/3.0 wire の組立/解析（x64 側）と、helper の `request()` エクスポート実呼出（i686 側）が欠けている。

## Current State

- **凍結境界 `shiori-host32-ipc` は既に REQUEST 対応済み**: `MsgTag::Request(3)`/`Response(4)` 定義済み・`send_request()` 往復（REQUEST→RESPONSE・ResponseSlot）実装済み（`lib.rs:44-55`/`108-143`）。**本ユニットで IPC フレーム形式・タグの追加は不要**（ペイロード＝不透明バイト列）。
- **helper**: `request` エクスポートは**解決済み・未呼出**（`shiori_proxy.rs:66` に「下流 host32-request が追加する」と TODO 明記）。`respond()` は echo スタブ（`main.rs:69`・置換予定と明記済み）。
- **HGLOBAL 所有権契約は確立済み**: 入力 HGLOBAL＝callee（DLL）が free／応答 HGLOBAL＝caller（helper）が free（`shiori-host32-testdll:lib.rs:61-73` が契約を固定）。シグネチャ `unsafe extern "cdecl" fn request(req: HGLOBAL, len: *mut usize) -> HGLOBAL`。
- **x64 側 codec の設計指針は既定**: completed shiori-load design §Shiori3Codec が「SHIORI/3.0 の組立と `Value:` parse は **x64 親側に閉じる**（helper はバイト proxy に徹する）」と規定。pilot `shiori3.rs` の `build_onboot`/`parse_value` が実証済み donor 知見（**コピペ禁止・知見参照**＝二坑規律）。
- **shiori-abi は是正済み**: `IShioriFactory` 融合 create＋`IShiori::Get(input)→HSTRING response`／`Notify` 分離（07-02 完了）。

## Desired Outcome

x64 が SHIORI/3.0 request バイト列を組み立て → WM_COPYDATA wire 越しに 32bit `pasta.dll` の `request()` を駆動 → response バイト列から `Value` を受領する。

**✔ 観測（単一 pass/fail）**: (a) testdll 決定的テスト（固定 SHIORI/3.0 response → host が `Value` 抽出）green ＋ (b) 実 emo2 `pasta.dll` OnBoot `Value` 受領（env-gated・`HOST32_TESTDLL_DLL` 方式踏襲）。

## Approach

1. **helper 側（i686）**: `respond()` の echo を `ShioriByteProxy::request()` 実呼出へ置換。request バイト列→GlobalAlloc(GMEM_FIXED)→DLL へ（callee-free）／返却 HGLOBAL→バイト列コピー→**caller が GlobalFree**→RESPONSE frame で返送。
2. **host 側（x64）**: `Shiori3Codec` 新設 — request build（`GET SHIORI/3.0`/`NOTIFY SHIORI/3.0`・**CRLF**・`Charset`/`Sender`/`ID`/`Reference0..N`・**空行終端**）＋ response parse（status 200/204/311/312/400/500・`Value`・`ErrorLevel`/`ErrorDescription`）。ID/References は**汎用ビルダ**（イベント個別知識を持たない）。
3. **charset**: emo2＝UTF-8 固定で主実装（fixture descript `charset,UTF-8`）。response `Charset` ヘッダ省略時は request 側を継承（ukadoc）。**Shift_JIS はシームのみ**（encoding_rs 依存導入済・実装しない＝emo2 未使用・最小実装規律）。
4. **testdll fixture 拡張**: `request` スタブ（現在 null 返却）を「受領 request を検証し固定 SHIORI/3.0 response を返す」fixture へ拡張（決定的観測・所有権契約の実地検証を兼ねる）。
5. **IShiori への装着**: codec＋transport を `IShiori::Get` 実装として host32-host に載せるか（SHIORI4⇄SHIORI3 変換＝IShiori 下の x64 過去互換アダプタ・記憶 areka-shiori-layer-naming）は **design で判断**。HSTRING⇄バイト列の変換点はプロセスを跨がない（HGLOBAL=32bit ローカル/HSTRING=x64 ローカル）。

## クロスユニット契約（後続を詰ませない事前考慮・2026-07-03）

- **kanade（下流の呼び手）向け出口契約**: 本ユニットの host 側 API は「**イベント（ID＋References）を渡す→Value（あれば）が返る**」の形に切る（GET は応答待ち・NOTIFY は投げきり）。kanade が SHIORI イベント循環を組む際にこの API をそのまま消費できる形＝request 送出の内部（codec/wire/HGLOBAL）を kanade に漏らさない。
- **IShiori::Get の同期/遅延 seam**: `shiori-abi` は `SHIORI_S_PENDING`＋token＋`IShioriHost::Complete` の遅延応答を定義済みだが、**pasta.dll（SHIORI/3.0 wire）は同期応答**——本ユニットの実装は同期（S_OK 即時）で足る。**PENDING 経路は型シームのみ**（実装しない・塞がない）。host32-lifecycle（常駐 msg loop）で非同期化の必要が出た場合もこの seam が受け皿。
- **タイムアウト・エラーの語彙**: wire タイムアウト（DEFAULT_LOAD_TIMEOUT 5s 前例）・SHIORI エラー応答（400/500・ErrorLevel）・helper 死活を**呼び手が区別できるエラー型**で返す（kanade のリトライ/縮退判断と host32-lifecycle の crash 監視が同じ語彙に乗る）。

## ukadoc 正典要点（design の前提事実）

- request line: `GET SHIORI/3.0` ＝応答要求／`NOTIFY SHIORI/3.0` ＝通知のみ。ヘッダ `Charset`・`Sender`・`ID`（イベント名）・`Reference0..N`、行末 CRLF、空行で終端。
- response: `SHIORI/3.0 200 OK`（`Value` あり）／`204 No Content`（返答なし）／`311`/`312`（OnTeach）／`400`/`500`。`ErrorLevel`（info〜critical）/`ErrorDescription` は SSP 拡張。
- `SecurityLevel` ヘッダは SSP 系拡張（design で送出可否を判断・de-facto）。
- OnBoot: Reference0=起動 shell 名（イベントカタログの網羅は kanade 領分・本ユニットは OnBoot 一件を観測に使うのみ）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-03 総ざらい）

- **必読**: `spec_shiori3` **全文**（ステータスコード／Sender／Reference1〜／ErrorLevel・ErrorDescription〔SSP拡張〕／X-SSTP-PassThru／SenderType〔internal/external/sstp/embed/raise 等〕／SecurityOrigin）＋ `memo_shiorievent`（イベント往復の概念）。SSTP 経由の将来形は `spec_sstp`（request/response）を参照。
- **brief 未網羅→design で埋める項目**: ① **SecurityLevel** の値域（local/external）② **SenderType/SecurityOrigin/X-SSTP-PassThru** ヘッダの扱い（M1 は送出最小・受信寛容）③ **未知ヘッダ寛容**——SHIORI 応答に未知ヘッダが来ても落とさない（保持 or 読み飛ばし）を codec 契約に明記 ④ 応答側 `Reference*`・`Marker` の存在（tolerate）⑤ リソース照会系 ID（`version`/`name`/`craftman` 等の GET）——kanade 前に codec が汎用 ID で送れることを確認。
- **具体指示**: design 冒頭で `get_doc('spec_shiori3')` を読み、**送出ヘッダ最小集合（Charset/Sender/ID/Reference*・SecurityLevel）と受信寛容集合**の2表を design.md に載せること。

## Scope

- **In**: helper `request()` 実呼出（HGLOBAL 契約遵守）／x64 `Shiori3Codec`（build＋parse＋UTF-8）／testdll request fixture 拡張／env-gated 実 pasta E2E／IShiori 装着方針の design 判断。
- **Out**: イベントカタログ・送出タイミング・SHIORI イベント循環（**kanade**）／OnSecondChange ポーリング・常駐 msg loop・crash 監視・unload 常用系（**host32-lifecycle**）／Shift_JIS 実装（シームのみ）／IPC フレーム形式変更（**凍結**）／SAORI（emo2 未使用）。

## Boundary Candidates

- wire codec（x64・純粋関数群＝単体テスト可）と transport 結線（send_request 呼出）の分離
- helper 側 request 駆動（proxy 拡張）と RESPONSE 返送の分離
- testdll fixture の request 検証面（受領内容 assert）

## Out of Boundary

- ゴースト mount からの load_dir 解決（shiori-load 済）・descript 解析（parsers 済）
- 応答 Value（さくらスクリプト）の解釈・再生（sakura／kanade）

## Upstream / Downstream

- **Upstream**: `areka-P0-host32-shiori-load` ✅（load 貫通・proxy 基盤）／`shiori-host32-ipc` ✅（凍結 wire）／`shiori-abi` ✅（IShiori 是正済）。
- **Downstream**: `areka-P0-host32-lifecycle`（常駐運転）→ `areka-P0-kanade`（イベント循環が本ユニットの request API を消費）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-host32-shiori-load`（helper proxy・ack 経路を request 対応へ拡張）。
- **Adjacent**: `completed/areka-P0-host32-ipc`（凍結・不改変）／`completed/pilot-shiori-host-32`（donor 知見・コピペ禁止）。

## Constraints

- Rust 2024・`windows` 0.62.2・tokio 禁止。helper=i686-pc-windows-msvc（**PowerShell ビルド必須**）・host=x64+arm64。32bit 可搬性制約は本トラック（host-32 系）に適用。
- HGLOBAL 所有権契約厳守（入力=callee free／応答=caller free・double-free 禁止）。
- i686 テストのサイレントスキップ禁止（shiori-load 踏襲・fixture 不在は fail）。
- 設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本として更新。
