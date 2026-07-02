# Requirements Document

## Project Description (Input)

M1 `areka-P0-emo2-boot` の「① SHIORI 通信層エンジン host-32」トラック第2ユニット。上流 `areka-P0-host32-ipc`（bytes-over-wire transport・完了済）が残した `MsgTag::Load` seam の上に、実 `pasta.dll` の実行時ロードを結線する。x64 areka は emo2 の 32bit `pasta.dll` を in-proc ロードできない（ビット幅不一致）ため、上流が生バイト往復 transport（helper spawn／WM_COPYDATA framing／HELLO handshake／再入 RESPONSE／timeout）を完成させたが、helper 側は現在 `respond(req) -> req.to_vec()` の echo stub にすぎず実 SHIORI DLL を触らない。`MsgTag::Load(2)` はワイヤ互換のため定義済みだが未処理（helper では現在無視）。この seam を埋めない限り emo2 の脳は一度も動かない。本ユニットは helper に `ShioriByteProxy` を新設して echo stub を差し替え、実 i686 helper 越しに実 emo2 `pasta.dll` を `LoadLibraryW` → `load(ghostdir)` 成功（`true`）まで、クラッシュ無しで駆動できることを観測可能にする。

## Introduction

本仕様は host-32 トラックの第2ユニットとして、上流 `areka-P0-host32-ipc` が凍結した WM_COPYDATA transport の上に「実 SHIORI DLL のロード」というセマンティクスを載せる。到達目標は、x64 親プロセスが実 i686 helper プロセス越しに実 emo2 `pasta.dll` を **`LoadLibraryW` → 3 エクスポート解決 → `load(ghostdir)` を呼び出し、成功結果（`true`）をクラッシュ無しで観測できる**ことである。これにより下流ユニット（`request` 呼出・常駐 lifecycle）が載る常設プロキシの足場が確立する。

本ユニットは **transport 層を一切改変しない**。`MsgTag::Load` は上流で定義済みであり、本ユニットはその結線（helper 側の受領・処理）と、helper 内 FFI プロキシの新設、`load` の呼出のみを担う。`request` の呼出、`unload` の恒常呼出、SHIORI/3.0 セマンティクス、常駐 lifecycle は明示的に対象外である。

## Boundary Context

- **In scope（本ユニットが担う観測可能な振る舞い）**:
  - helper の echo stub（`respond`）置換＝`MsgTag::Load` の受領・処理の結線。
  - helper 内 FFI プロキシ（`ShioriByteProxy`）による `pasta.dll` の `LoadLibraryW` ロードと、`load`／`unload`／`request` の **3 エクスポートの解決**（fn ポインタ保持・モジュールハンドル所有）。
  - `load(ghostdir)` の**呼出**: ghostdir を ANSI(CP_ACP/Shift_JIS) 符号化し、そのバイト結果を `load` へ渡して bool 結果を観測する。load 入力バッファの所有権規約（DLL 側が解放）を守る。
  - x64 親 → helper の LOAD 入力契約（`pasta.dll` の所在と ghostdir の受け渡し）と、`load` の bool 結果を親へ返す ack。
  - 観測指標: 実 emo2 `pasta.dll` を実 i686 helper 越しに `load` 成功（`true`）・無クラッシュで E2E に観測。
- **Out of scope（本ユニットが所有しないもの）**:
  - `request` の**呼出**・SHIORI/3.0 の組立/marshal・`Value` parse・request の UTF-8 charset（→ 下流 `areka-P0-host32-request`）。※`request` fn ポインタの**解決**は本ユニット、**呼出はしない**。
  - 常駐メッセージループの生存監視・`OnSecondChange` poll・`unload` の**恒常呼出**・crash 監視の lifecycle（→ 下流 `areka-P0-host32-lifecycle`）。※`unload` fn ポインタの解決は本ユニット。テスト後始末の courtesy `unload`／`FreeLibrary` は許容だが常駐 lifecycle は所有しない。
  - transport 層（spawn／WM_COPYDATA framing／`ResponseSlot`／HELLO／timeout）の改変（`areka-P0-host32-ipc` 完了済・`MsgTag::Load` は定義済みで本ユニットは結線するのみ）。
  - 里々/YAYA・SAORI・native x64 化（M2 以降）。
- **Adjacent expectations（隣接仕様への期待）**:
  - **上流 `areka-P0-host32-ipc`（完了・改変不可）**: `MsgTag{Hello,Load,Request,Response,Unload}`／WM_COPYDATA framing（`dwData` 低 32bit タグ・`cbData`=生バイト長・固定ヘッダ長 0）／HWND の u32 LE 符号化／`send_copydata`・`send_request`・`ResponseSlot`／`SMTO_ABORTIFHUNG`+timeout／`ProcessHost::spawn(helper_exe, ghostdir, parent_hwnd)`（ghostdir を helper の作業ディレクトリとして、parent_hwnd を helper 起動引数として既に運ぶ）を提供する。本ユニットはこれらを利用し、変更しない。
  - **参照専用 `pilot-shiori-host-32`（go 済・コピペ禁止）**: FFI シーケンス／charset 非対称／HGLOBAL 所有権規約の一次記録（README 学び #4–#6）。知見のみ参照し、コードは隔離する（葉ノード隔離＝production クレートは `crates/pilot` へ inbound 依存しない）。
  - **正本 `vendors/pasta`（`crates/pasta_shiori`）と `doc/COMPAT_ARCHITECTURE.md`**: flat-C ABI と過去互換経路のバイト正確源。

## Requirements

### Requirement 1: LOAD メッセージの結線（helper 側受領）

**Objective:** As a host-32 helper プロセス, I want 親から送られた `MsgTag::Load` メッセージを受領して処理する, so that echo stub のままでは決して起きなかった実 DLL ロードを開始できる

#### Acceptance Criteria

1. When helper が `MsgTag::Load` の WM_COPYDATA フレームを受領し, the host-32 helper shall そのフレームを `pasta.dll` ロード要求として処理へ回す（現行の「既知だが無応答（無視）」扱いをやめる）。
2. When helper が LOAD 要求を処理する, the host-32 helper shall LOAD payload から `load` に必要な入力（`pasta.dll` の所在と ghostdir）を取り出す。
3. If 受領した LOAD フレームが framing 規約（既知タグ・宣言長と実長の一致）に整合しない, then the host-32 helper shall クラッシュせず不正フレームとして記録のみ行い, 上位へ伝播させない。
4. The host-32 helper shall LOAD の処理において上流 transport の凍結 seam（WM_COPYDATA の framing・`MsgTag` 定義・HWND 符号化）を改変しない。

### Requirement 2: pasta.dll のロードと 3 エクスポート解決（ShioriByteProxy）

**Objective:** As a host-32 helper プロセス, I want `pasta.dll` を実行時にロードし `load`／`unload`／`request` の 3 エクスポートを解決して保持する, so that 以降の下流ユニットが載る常設プロキシの足場が立つ

#### Acceptance Criteria

1. When LOAD 要求で `pasta.dll` の所在が与えられる, the ShioriByteProxy shall その DLL を実行時にロードし, ロード成功時にモジュールハンドルを所有する。
2. When DLL ロードに成功した, the ShioriByteProxy shall `load`／`unload`／`request` の 3 エクスポートを解決し, 呼出可能な形で保持する。
3. If `pasta.dll` のロードに失敗する, then the ShioriByteProxy shall クラッシュせず失敗として扱い, 親が「load 未成功」を観測できるようにする。
4. If 3 エクスポートのいずれかが解決できない, then the ShioriByteProxy shall クラッシュせず失敗として扱い, 親が「load 未成功」を観測できるようにする。
5. The ShioriByteProxy shall `request` エクスポートの fn ポインタを解決・保持するが, 本ユニットでは `request` を呼び出さない。
6. The ShioriByteProxy shall `unload` エクスポートの fn ポインタを解決・保持するが, 常駐 lifecycle としての `unload` 恒常呼出は行わない。

### Requirement 3: load(ghostdir) の呼出と charset 符号化

**Objective:** As a host-32 helper プロセス, I want ghostdir を SHIORI が要求する文字集合で符号化して `load` を呼び出す, so that 実 emo2 の脳が正しい ghost 位置で初期化される

#### Acceptance Criteria

1. When ShioriByteProxy が `load` を呼び出す前に, the host-32 helper shall ghostdir を ANSI(CP_ACP / Shift_JIS) バイト列へ符号化する。
2. When ghostdir の符号化が完了した, the host-32 helper shall 符号化バイト列を入力バッファとして `load` を呼び出し, その bool 返り値を取得する。
3. The host-32 helper shall `load` の返り値を Rust `bool`（1 バイト）として解釈する（Win32 BOOL としては解釈しない）。
4. The host-32 helper shall `load` に渡した入力バッファの所有権を DLL 側の解放に委ね, ホスト側で二重解放しない。
5. The host-32 helper shall `load` 入力の符号化責務のみを負い, `request` の UTF-8 符号化・request 応答バッファの所有権は下流ユニットに委ねる。

### Requirement 4: load 結果の親への ack

**Objective:** As a x64 親プロセス, I want helper から `load` の成否結果を受け取る, so that 実 DLL ロードが成功したかをクラッシュ無しで観測できる

#### Acceptance Criteria

1. When helper が `load` の呼出を完了する, the host-32 helper shall その bool 結果を親へ返送する。
2. When 親が helper からの load 結果を受け取る, the x64 親 shall `load` が成功（`true`）したか未成功（`false` またはロード失敗）かを判別できる。
3. The host-32 helper shall load 結果の返送を上流 transport の凍結 seam（WM_COPYDATA の REQUEST/RESPONSE ワイヤ形式）を改変しない範囲で行う。
4. If `load` が未成功（`false`）またはロード/解決失敗であった, then the x64 親 shall その未成功をクラッシュせず観測できる。

### Requirement 5: 実バイナリを用いた E2E ロード観測

**Objective:** As a areka 開発者, I want 実 i686 helper と実 emo2 `pasta.dll` fixture を用いた E2E で load 成功を観測する, so that host-32 トラックの「脳がロードできる」ことを確証できる

#### Acceptance Criteria

1. When x64 親が実 i686 helper を起動し, LOAD 要求を送って実 emo2 `pasta.dll` を対象に `load(ghostdir)` を駆動する, the host-32 helper shall `load` を成功（`true`）させ, 親がその成功をクラッシュ無しで観測できる。
2. While 上記 E2E ロード観測が進行する, the host-32 shall いずれのプロセスもクラッシュさせない（無クラッシュを観測可能な成功条件とする）。
3. The host-32 shall 本ユニットの観測契約を `load` の同期 bool 返却と無クラッシュのみに限定し, 実バイナリの内部スレッド等の未確認の内部前提には依存しない。
4. Where テスト後始末が必要な場合, the host-32 helper shall courtesy の `unload`／`FreeLibrary` を行ってよいが, これは常駐 lifecycle の所有ではなく後始末に限る。

### Requirement 6: 32bit 可搬性とビルド健全性

**Objective:** As a areka 開発者, I want i686 helper と共有モジュールが 32bit ターゲットで健全にビルド・検証される, so that ビット幅差やビルドトラップで隠れた破綻が生じない

#### Acceptance Criteria

1. The host-32 helper shall i686-pc-windows-msvc ターゲットでビルド・実行可能である。
2. When `dwData`／`ULONG_PTR` 系の値に対しシフト/マスク演算を行う, the host-32 shall その演算を 64bit 幅で評価し, i686 の `usize`=32bit でのオーバーフローを避ける。
3. The host-32 shall FFI 呼出を cdecl（`extern "C"`）ABI で行い, `load`/`unload`/`request` の各シグネチャ（`load(buf, len) -> bool` / `unload() -> bool` / `request(buf, *mut len) -> buf`）に整合させる。
4. The host-32 shall production クレートから `crates/pilot`（先進坑）へ inbound 依存を持たない（葉ノード隔離）。
