# Requirements Document

## Project Description (Input)

M1 `areka-P0-emo2-boot` の「① SHIORI 通信層エンジン host-32」トラック第 3 ユニット（逐次チェーン: pilot✅ → `host32-ipc`✅ → `host32-shiori-load`✅ → **本ユニット** → `host32-lifecycle`）。

**問題**: x64 areka は 32bit `pasta.dll` の load までは貫通した（`host32-shiori-load` 完了）が、**request が呼べない＝ゴーストが一言も喋れない**。SHIORI/3.0 wire の組立/解析（x64 側）と、helper の `request()` エクスポート実呼出（i686 側）が欠けている。

**現状**: 凍結境界 `shiori-host32-ipc` は既に REQUEST/RESPONSE 対応済み（`MsgTag::Request(3)`/`Response(4)` 定義済み・`send_request()` 往復実装済み・ペイロード＝不透明バイト列）ゆえ IPC フレーム形式・タグの追加は不要。helper の `request` エクスポートは解決済み・未呼出（`respond()` は echo スタブ）。HGLOBAL 所有権契約（入力＝callee free／応答＝caller free）は `host32-shiori-load` で確立済み。x64 側 codec の設計指針（SHIORI/3.0 の組立と `Value:` parse は x64 親側に閉じる・helper はバイト proxy に徹する）は完了済み `host32-shiori-load` design で既定。

**あるべき姿**: x64 が SHIORI/3.0 request バイト列を組み立て → WM_COPYDATA wire 越しに 32bit `pasta.dll` の `request()` を駆動 → response バイト列から `Value` を受領する。単一 pass/fail 観測は (a) testdll 決定的テスト（固定 SHIORI/3.0 response → host が `Value` 抽出）と (b) 実 emo2 `pasta.dll` OnBoot `Value` 受領（env-gated）。

## Introduction

本仕様の到達目標は「**x64 areka がイベント（ID＋References）を SHIORI/3.0 request として組み立て、凍結 wire 越しに 32bit SHIORI DLL の `request()` を駆動し、返却された SHIORI/3.0 response から `Value`（さくらスクリプト本体）を受領できる**」ことである。

これは 2 層にまたがる欠落を埋める:

- **helper 側（i686）**: `respond()` の echo スタブを、常設プロキシ（`host32-shiori-load` で確立）が保持する `request` エクスポートの実呼出へ置換する。request バイト列を HGLOBAL として DLL へ渡し（callee-free）、返却 HGLOBAL からバイト列をコピーして解放し（caller-free）、RESPONSE フレームで返送する。
- **host 側（x64）**: SHIORI/3.0 の request 組立（GET/NOTIFY・CRLF・空行終端）と response 解析（ステータスコード・`Value`・エラー情報）を担う純粋 codec を新設し、既存の request 往復トランスポート（`host32-shiori-load` 完了）と結線する。

本仕様は上流 `shiori-host32-ipc` の WM_COPYDATA transport（wire/framing/`MsgTag`/`ResponseSlot`/timeout）を一切改変しない（凍結）。イベントカタログ・送出タイミング・SHIORI イベント循環（下流 `kanade`）、常駐メッセージループ・crash 監視・`unload` 常用系（下流 `host32-lifecycle`）、Shift_JIS の実装（シームのみ）、SAORI（emo2 未使用）は明示的に対象外である。

## Boundary Context

- **In scope（本仕様が担う観測可能な振る舞い）**:
  - **[helper]** `request` エクスポートの実呼出（echo スタブ置換）。request バイト列を `GlobalAlloc(GMEM_FIXED)` の HGLOBAL として DLL へ渡し（callee-free）、返却 HGLOBAL からバイト列をコピー後 caller が `GlobalFree`。応答は既存 RESPONSE 経路で親へ返送。
  - **[host]** SHIORI/3.0 request 組立（`GET`／`NOTIFY`・request line・`Charset`/`Sender`/`ID`/`Reference0..N` ヘッダ・CRLF 行区切り・空行終端・UTF-8）と response 解析（ステータスコード 200/204/311/312/400/500・`Value`・`ErrorLevel`/`ErrorDescription`）。ID／References は汎用ビルダ（イベント個別知識を持たない）。
  - **[host 出口 API]** 下流の呼び手（`kanade`）が消費する形の request API: 「イベント（ID＋References）を渡す→`Value`（あれば）が返る」。`GET`＝応答待ち・`NOTIFY`＝投げきり。codec/wire/HGLOBAL の内部を呼び手へ漏らさない。呼び手が区別できるエラー語彙（wire タイムアウト・SHIORI エラー応答・helper 死活）を返す。
  - **[testdll]** `request` スタブ（現在 null 返却）を「受領 request を検証し固定 SHIORI/3.0 response を返す」fixture へ拡張（決定的観測＋所有権契約の実地検証）。
  - **[E2E]** testdll による決定的 request 往復（成功・`Value` 抽出・所有権規約）＋ env-gated 実 emo2 `pasta.dll` OnBoot `Value` 受領。
  - **[IShiori 装着]** codec＋transport を `IShiori::Get` 実装として host32 互換 backend に載せる方針の可否は design 判断（本仕様は出口 API の形を要件で固定し、装着方式そのものは design に委ねる）。
- **Out of scope（本仕様が所有しないもの）**:
  - イベントカタログ・送出イベント名の網羅・送出タイミング・SHIORI イベント循環の駆動（→ 下流 `areka-P0-kanade`）。本仕様は OnBoot 一件を観測に用いるのみで、codec は汎用 ID で任意イベントを送れることを示す。
  - 常駐メッセージループ生存・`OnSecondChange` ポーリング・crash 監視・`unload` の恒常呼出（→ 下流 `areka-P0-host32-lifecycle`）。
  - `IShiori::Get` の遅延応答（`SHIORI_S_PENDING`＋token＋`IShioriHost::Complete`）の実装（pasta の SHIORI/3.0 wire は同期応答ゆえ本仕様は同期で足る・型シームのみ・実装しない）。
  - Shift_JIS の request/response 実符号化の実装（シームのみ・emo2＝UTF-8 固定ゆえ実装しない）。
  - WM_COPYDATA transport（`shiori-host32-ipc` の wire/framing/`MsgTag`/`ResponseSlot`/timeout）の改変（上流完了・凍結）。
  - ゴースト mount からの `load_dir` 解決・descript 解析（`package-mount`／`parsers` 済）／応答 `Value`（さくらスクリプト）の解釈・再生（`sakura`／`kanade`）／SAORI（emo2 未使用）／native x64 脳の実装本体。
- **Adjacent expectations（隣接仕様への期待）**:
  - **上流 `shiori-host32-ipc`（完了・凍結）**: `MsgTag{Request=3,Response=4}`／`send_request`（REQUEST 送出→再入 RESPONSE 受領→`ResponseSlot` 消費）／`SMTO_ABORTIFHUNG`＋timeout を提供済み。本仕様は不透明バイト列のペイロードとしてこれをそのまま使う。
  - **上流 `areka-P0-host32-shiori-load`（完了）**: helper 内常設プロキシ（`ShioriByteProxy`）が `load`/`unload`/`request` の 3 fn ポインタを解決保持済み・`load` 済み。x64 側 `ParentMessageWindow::send_request(tag, payload, timeout)` が request 往復トランスポートを提供済み。本仕様はこの 2 点の上に request 呼出と codec を増分する。
  - **`shiori-abi`（是正済み・完了）**: `IShiori::Get(input)→応答`／`Notify(input)→片道`、`GetOutcome`（即時 HSTRING／遅延 token）、`IShioriFactory::CreateInstance` が定義済み。host-32 互換 backend の factory 実装は未在。本仕様の host 出口 API はこの `IShiori::Get`/`Notify` の意味論に写像可能な形に切る（HSTRING⇄バイト列変換はプロセスを跨がない＝HGLOBAL は 32bit ローカル・HSTRING は x64 ローカル）。
  - **参照専用 `pilot-shiori-host-32`（go 済・コピペ禁止）**: `build_onboot`/`parse_value` が SHIORI/3.0 wire の donor 知見（request line・ヘッダ・CRLF・空行終端・`Value:` 抽出）。production クレートは `crates/pilot` へ inbound 依存しない（葉ノード隔離）。
  - **ukadoc 正典**: SHIORI/3.0 の request/response 書式（`GET`/`NOTIFY SHIORI/3.0`・`Charset`/`Sender`/`ID`/`Reference*`・CRLF・空行終端・ステータスコード・`ErrorLevel`/`ErrorDescription`〔SSP 拡張〕）と DLL 共通仕様（GMEM_FIXED・`request` 署名・callee/caller の解放責務）。emo2 fixture は最小サンプルにすぎず正典は ukadoc。
  - **下流 `areka-P0-kanade`（後続）**: 本仕様の host 出口 API を SHIORI イベント循環の request 送出としてそのまま消費する。API はブロッキング呼出可（専用スレッド前提）・引数/戻り値は `Send` な所有データ・親窓 pump スレッドと干渉しない形。channel 化は kanade 結線時（本 API を包むだけで済むことが受入基準）。
  - **下流 `areka-P0-host32-lifecycle`（後続）**: 本仕様のエラー語彙（タイムアウト・SHIORI エラー・helper 死活）を crash 監視・縮退判断の受け皿として共有する。
  - **未確定で design が埋める項目**: 送出ヘッダ最小集合と受信寛容集合の確定（`SecurityLevel`/`SenderType`/`SecurityOrigin`/`X-SSTP-PassThru`・未知ヘッダ寛容・応答側 `Reference*`/`Marker` の tolerate）／codec を `IShiori::Get` へ装着する具体構造。これらは design フェーズで確定する。

## Requirements

### Requirement 1: host 側 SHIORI/3.0 request 組立（汎用ビルダ）

**Objective:** As a x64 host-32 ホスト層, I want イベント（ID と Reference 群）から SHIORI/3.0 request バイト列を組み立てる汎用ビルダ, so that イベント個別の知識を持たずに任意の SHIORI イベントを wire 形式で送出できる

#### Acceptance Criteria

1. When 呼び手が応答を要するイベント（ID と任意個の Reference）を与える, the host-32 SHIORI codec shall request line `GET SHIORI/3.0` から始まる request バイト列を組み立てる。
2. When 呼び手が片道イベント（通知のみ）を与える, the host-32 SHIORI codec shall request line `NOTIFY SHIORI/3.0` から始まる request バイト列を組み立てる。
3. The host-32 SHIORI codec shall request の各行を CR+LF（0x0D 0x0A）で区切り、ヘッダ部の終端を空行（連続する CR+LF）で示す。
4. The host-32 SHIORI codec shall request に `Charset`・`Sender`・`ID`（イベント名）ヘッダを含め、与えられた Reference 群を `Reference0`・`Reference1`・…（0 起点連番）ヘッダとして順に含める。
5. The host-32 SHIORI codec shall イベント名を汎用の `ID` 値として受け取り、特定イベント（OnBoot 等）に固有の分岐や既定 Reference を埋め込まない。
6. While 本仕様の対象範囲（emo2）では, the host-32 SHIORI codec shall request を UTF-8 で符号化し、`Charset` ヘッダに UTF-8 を宣言する。
7. Where Shift_JIS 対応が将来含まれる場合, the host-32 SHIORI codec shall charset 切替の拡張シームのみを備え、本仕様では Shift_JIS の実符号化を実装しない。

### Requirement 2: host 側 SHIORI/3.0 response 解析

**Objective:** As a x64 host-32 ホスト層, I want SHIORI/3.0 response バイト列からステータス・`Value`・エラー情報を解析する, so that 呼び手へ「応答さくらスクリプト（あれば）」と「区別可能な失敗」を返せる

#### Acceptance Criteria

1. When response がステータスコード 200 を示す, the host-32 SHIORI codec shall `Value` ヘッダの値（さくらスクリプト本体）を応答結果として抽出する。
2. When response がステータスコード 204 を示す, the host-32 SHIORI codec shall 「応答なし（`Value` 不在）」を成功として区別可能に返す。
3. The host-32 SHIORI codec shall response の行区切りとして CR+LF を受理し、ステータス行の後続ヘッダを「ヘッダ名: 値」形式として解析する。
4. If response がステータスコード 400 または 500 を示す, then the host-32 SHIORI codec shall それを SHIORI エラー応答として呼び手に区別可能な形で返す。
5. When response に `ErrorLevel`／`ErrorDescription` ヘッダ（SSP 拡張）が存在する, the host-32 SHIORI codec shall それらを保持してエラー情報として呼び手が参照可能にする。
6. Where response の `Charset` ヘッダが省略されている, the host-32 SHIORI codec shall request 側で用いた charset を継承して解析する（ukadoc 準拠）。
7. When response にステータスコード 311 または 312（OnTeach 系）が現れる, the host-32 SHIORI codec shall それを解析上落とさず区別可能に扱う（本仕様は OnTeach 循環を駆動しないが codec は当該コードを許容する）。
8. When response に本仕様が明示的に扱わないヘッダが含まれる, the host-32 SHIORI codec shall それを理由に解析を失敗させない（未知ヘッダ寛容）。

### Requirement 3: helper 側 request エクスポートの実呼出（echo 置換）

**Objective:** As a host-32 helper プロセス（i686）, I want 常設プロキシが保持する `request` エクスポートを実呼出する, so that echo スタブでは決して駆動されなかった SHIORI DLL の request を実際に走らせる

#### Acceptance Criteria

1. When helper が REQUEST フレームを受領する, the helper shall 現行の echo 応答を置換し、常設プロキシの `request` エクスポートへ受領バイト列を渡して呼び出す。
2. When helper が `request` を呼び出す, the helper shall 受領バイト列を `GlobalAlloc(GMEM_FIXED)` で確保した HGLOBAL バッファとして長さと共に渡す。
3. The helper shall `request` へ渡した入力 HGLOBAL を自ら解放しない（所有権は DLL 側へ移転・callee-free 規約・二重解放を発生させない）。
4. When `request` が応答 HGLOBAL と長さを返す, the helper shall 応答バイト列を自前バッファへコピーした上で、その応答 HGLOBAL を `GlobalFree` で解放する（caller-free 規約）。
5. When helper が応答バイト列を得る, the helper shall それを既存の RESPONSE 経路（`MsgTag::Response`）で親へ返送し、新しい `MsgTag`・framing 変更・wire 改変を伴わない。
6. The helper が呼び出す `request` の flat-C 契約 shall `vendors/pasta` を正確源とする署名（cdecl・`request(HGLOBAL, *mut len) -> HGLOBAL`・長さ引数は in/out）に一致する。
7. While REQUEST を処理していない状況でも, the helper shall 本仕様の範囲では常設プロキシの 3 エクスポートのうち `request`（と既存の `load`／Drop 時 courtesy `unload`）のみを呼び出し、`unload` の恒常呼出は行わない（下流 `host32-lifecycle` の領分）。

### Requirement 4: request 往復のエンドツーエンド結線（host 出口 API）

**Objective:** As a 下流の呼び手（kanade 相当）, I want イベントを渡すと `Value` が返る単一の request API, so that codec・wire・HGLOBAL の内部を知らずに SHIORI イベントの応答を得られる

#### Acceptance Criteria

1. The host-32 ホスト層 shall 「応答を要するイベント（ID＋References）を渡す→`Value`（あれば）が返る」GET 経路の出口 API を提供する（呼び手は request 組立・wire・HGLOBAL の内部を意識しない）。
2. The host-32 ホスト層 shall 「片道イベント（ID＋References）を投げきる」NOTIFY 経路の出口 API を提供する（応答を待たない）。
3. When GET 出口 API が呼ばれる, the host-32 ホスト層 shall request を組み立て、既存の request 往復トランスポート（`host32-shiori-load` 完了）越しに helper へ送出し、返送 response を解析して結果を呼び手へ返す。
4. The host-32 ホスト層の出口 API shall 呼び手のスレッドをブロックして同期的に応答を返してよい（専用スレッド前提・親窓 pump スレッドの再入規律と干渉しない）。
5. The 出口 API の引数と戻り値 shall スレッド跨ぎで受け渡し可能な所有データ（`Send`）とし、codec/wire/HGLOBAL の内部型を呼び手へ露出しない。
6. The 本仕様 shall `IShiori::Get` の遅延応答（`SHIORI_S_PENDING`＋token＋`Complete`）を実装せず、pasta の SHIORI/3.0 同期応答に対して同期（即時応答）で完結させる（遅延経路は型シームのみ・塞がない）。

### Requirement 5: request 経路のエラー語彙

**Objective:** As a 下流の呼び手（kanade／host32-lifecycle）, I want request 失敗の各態様を区別できるエラーとして受け取る, so that リトライ・縮退・crash 監視を同じ語彙の上で判断できる

#### Acceptance Criteria

1. If request 往復が上限時間内に応答を返さない, then the host-32 ホスト層 shall それを wire タイムアウトとして呼び手が区別できるエラーで返す（既存凍結 transport の timeout 機構に乗る・本仕様は timeout 機構を変更しない）。
2. If response が SHIORI エラー応答（ステータス 400／500・または `ErrorLevel`）を示す, then the host-32 ホスト層 shall それを SHIORI エラーとして wire タイムアウトと区別できる形で返す。
3. If helper プロセスが応答不能・死活喪失の状態にある, then the host-32 ホスト層 shall それを呼び手が区別できるエラーで返す（プロセス処分の判断は下流 `host32-lifecycle` に委ねる）。
4. The host-32 ホスト層 shall タイムアウト・SHIORI エラー・helper 死活を単一の不透明失敗へ潰さず、呼び手のリトライ／縮退判断に足る区別を保った語彙で返す。

### Requirement 6: testdll request fixture 拡張と決定的 E2E 検証

**Objective:** As a host-32 トラックの開発者, I want トラック所有の最小 SHIORI DLL fixture の `request` を「受領を検証し固定 response を返す」形へ拡張する, so that 本物 `pasta.dll` に依存せず request 往復・`Value` 抽出・所有権契約を CI で再現可能に観測できる

#### Acceptance Criteria

1. The host-32 トラックの最小 SHIORI DLL fixture shall 現行の null 返却スタブ `request` を、受領した request バイト列を検証し固定の SHIORI/3.0 response を返す実装へ拡張する。
2. When fixture の `request` が呼ばれる, the fixture shall 受領した入力 HGLOBAL を callee 側で `GlobalFree` し、応答を新規 `GlobalAlloc(GMEM_FIXED)` の HGLOBAL として長さと共に返す（DLL 共通仕様の所有権契約の実地検証）。
3. The 決定的 E2E 検証 shall 実 i686 helper プロセス越しに fixture へ SHIORI/3.0 request を送出し、返送 response から host が `Value` を抽出できることを観測する。
4. Where fixture が受領 request の内容検証を行う場合, the fixture shall request line・`ID` 等の受領内容を検証面として assert 可能にし、request が正しく組み立てられて届いたことを裏付ける。
5. Where 環境変数（例: `HOST32_PASTA_DLL`）で本物 emo2 `pasta.dll` が指定されている, the E2E 検証 shall 実 `pasta.dll` に OnBoot request を送出し `Value` を受領する追加の confidence 検証を実行する。
6. If 実 `pasta.dll` を指す環境変数が設定されているのに指定 DLL が見つからない, then the テスト shall 明示的に失敗する（silent skip を認めない）。
7. The E2E 検証 shall 本物 `pasta.dll` を CI 必須ゲートとして要求せず、決定的検証は fixture で成立させる。
8. The fixture crate shall `crates/pilot` へ依存しない（葉ノード隔離の維持）。

### Requirement 7: 凍結境界・隔離規律・32bit ビルド規律の遵守（横断）

**Objective:** As a areka 開発者, I want 凍結境界・隔離規律・32bit ビルド規律が本仕様の全作業で守られる, so that 上流資産の安定性と検証の再現性が損なわれない

#### Acceptance Criteria

1. The 本仕様 shall `shiori-host32-ipc` の wire/framing/`MsgTag`/`ResponseSlot`/timeout の定義を改変しない（凍結境界・ペイロードは不透明バイト列として利用する）。
2. The i686 成果物（helper・fixture）のビルドおよびテスト shall PowerShell 経由で実行し、`cargo test --target i686-pc-windows-msvc` を検証に含める。
3. The i686 側テスト shall fixture／helper 不在時にサイレントスキップせず、明示的に失敗する（`host32-shiori-load` 踏襲）。
4. The production クレート shall `crates/pilot` へ inbound 依存しない（先進坑コードは知見参照のみ・コピペ禁止）。
5. The helper が用いる flat-C `request` 署名 shall 実装前に `vendors/pasta`（`git submodule update --init` で展開）との一致をバイト正確に確認した上で固定される。
6. The HGLOBAL 所有権規約 shall request 経路の全境界で厳守される（入力＝callee free／応答＝caller free・二重解放を発生させない）。
