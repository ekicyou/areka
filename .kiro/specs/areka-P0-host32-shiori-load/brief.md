# Brief: areka-P0-host32-shiori-load

> M1 `areka-P0-emo2-boot`「① SHIORI 通信層エンジン host-32」トラックの第2ユニット。
> 上流 `areka-P0-host32-ipc`（bytes-over-wire transport・✅完了）が残した **`MsgTag::Load` seam** の上に、実 `pasta.dll` の実行時ロードを結線する。
> 種別: 本坑（main）。go ゲートは先進坑 `pilot-shiori-host-32`（✅ go 済 2026-07-01）で充足済み＝着手可。

## Problem

x64 areka は emo2 の 32bit `pasta.dll` を **in-proc ロードできない**（ビット幅不一致）。上流 `areka-P0-host32-ipc` が「生バイト列を往復させる transport（helper spawn／WM_COPYDATA framing／HELLO handshake／再入 RESPONSE／timeout）」までを完成させたが、その helper 側は現在 `respond(req) -> req.to_vec()` の **echo stub**（`crates/shiori-host32-helper/src/main.rs:54–56`）にすぎず、実 SHIORI DLL を触らない。`MsgTag::Load(2)` はワイヤ互換のため**定義済みだが未処理**（`crates/shiori-host32-ipc` 内コメント「本ユニット未処理・下流で結線」）。この seam を埋めない限り emo2 の脳は一度も動かない。

## Current State

- **完了済み（触らない）**: `shiori-host32-ipc`（`MsgTag{Hello,Load,Request,Response,Unload}` / `send_copydata` / `send_request` / `ResponseSlot` / framing）、`shiori-host32-host`（`ProcessHost::spawn(helper_exe, ghostdir, parent_hwnd)` / `ParentMessageWindow` handshake ＋ `send_request`）。
- **stub（本ユニットの置換対象）**: `shiori-host32-helper` の `respond()` echo と `handle_message()`（現状 `MsgTag::Request` のみ処理）。`MsgTag::Load`/`Unload` は未結線。helper には `LoadLibrary`/`GetProcAddress` 足場・charset・HGLOBAL 取り扱いが**一切ない**。
- **知見 donor（参照のみ・コピペ禁止）**: 先進坑 `crates/pilot/examples/shiori-host-32/shiori_proxy.rs` が FFI シーケンスを実証済み（README 学び #4）。

## Desired Outcome

x64 親が実 i686 helper 越しに実 emo2 `pasta.dll` を **`LoadLibraryW` → `load(ghostdir)` 成功（`true`）**まで駆動し、**クラッシュ無し**で観測できる。helper 内に `pasta.dll` のモジュールハンドルと `load`/`unload`/`request` の3 fn ポインタを保持する常設プロキシが立ち、以降の下流ユニット（request 呼出・lifecycle）がその上に載れる。

## Approach

先進坑の知見を見て**一から掘り直す**（コピペ donor 禁止・二坑教義）。helper に `ShioriByteProxy` を新設し、echo stub を差し替える。trait 抽象は設けない（YAGNI・凍結 seam は WM_COPYDATA の REQUEST/RESPONSE ワイヤ形式）。

1. **helper: `MsgTag::Load` 結線** — `handle_message()` に Load 分岐を追加。LOAD payload（dll path ＋ ghostdir）を受領。
2. **`ShioriByteProxy` 構築** — `LoadLibraryW(pasta.dll path)` → `GetProcAddress` で3エクスポート解決 → `transmute` で cdecl fn ポインタ保持。
   - `load(HGLOBAL, usize) -> bool` / `unload() -> bool` / `request(HGLOBAL, *mut usize) -> HGLOBAL`（返り値は **Rust `bool`(1byte)**・Win32 BOOL ではない）。
3. **`load(ghostdir)` 呼出** — ghostdir を **ANSI(CP_ACP/Shift_JIS)** へ `WideCharToMultiByte` 符号化 → `GlobalAlloc(GMEM_FIXED)` で HGLOBAL 化 → `load(hglobal, len)`。**入力 HGLOBAL は DLL が解放**（ホストは二重解放しない）。bool 結果を観測。
4. **load ack** — bool 結果を x64 親へ返し、親が「load 成功・無crash」を観測可能にする。
5. **検証** — 実 i686 helper ＋ 実 emo2 `pasta.dll` fixture で E2E に load 成功を観測。

## Scope

- **In**:
  - helper の `respond()` echo stub 置換＝`MsgTag::Load` 分岐の結線。
  - `ShioriByteProxy`（i686）: `LoadLibraryW` ＋ `GetProcAddress` で `load`/`unload`/`request` **3エクスポートを解決**し fn ポインタを保持（モジュールハンドル所有）。
  - `load(ghostdir)` の**呼出**: ghostdir の ANSI(CP_ACP) 符号化 ＋ `GlobalAlloc(GMEM_FIXED)` HGLOBAL 化 ＋ **load 入力 HGLOBAL の所有権規約（DLL 解放）**。
  - x64 親→helper の LOAD 入力契約（dll path ＋ ghostdir の受け渡し）と、load の bool 結果 ack。
  - 観測指標: 実 emo2 `pasta.dll` を実 i686 helper 越しに `load` 成功（`true`）・無クラッシュ。
- **Out**:
  - `request` の**呼出**・SHIORI/3.0 build/marshal・Value parse・request の **UTF-8 charset**（→ `areka-P0-host32-request`）。※`request` fn ポインタの**解決**は本ユニット（proxy に保持）だが**呼出はしない**。
  - 常駐メッセージループ生存・`OnSecondChange` poll・`unload` の**呼出**・crash 監視の lifecycle（→ `areka-P0-host32-lifecycle`）。※`unload` fn ポインタの解決は本ユニット。テスト teardown の courtesy `unload`/`FreeLibrary`（Drop）は許容だが**常駐 lifecycle は所有しない**。
  - transport 層（spawn／WM_COPYDATA framing／ResponseSlot／HELLO／timeout）＝ `areka-P0-host32-ipc` 完了済・**改変しない**（`MsgTag::Load` は定義済みで本ユニットが**結線するのみ**）。
  - pilot コードのコピペ・再利用（README/学び参照に限る）。

## Boundary Candidates

- **helper 内 FFI プロキシ境界**: `ShioriByteProxy`（モジュールハンドル ＋ 3 fn ポインタ ＋ HGLOBAL 所有権をこの型に閉じ込める）。
- **LOAD ワイヤ payload 契約**: dll path ＋ ghostdir をどう符号化して helper へ渡すか（ipc の凍結 seam の上に載る新セマンティクス）。
- **charset 責務の分割線**: **load-path の ANSI(CP_ACP) 符号化のみ本ユニット**。request の UTF-8 は下流。
- **HGLOBAL 所有権の分割線**: **load 入力 HGLOBAL（DLL 解放）のみ本ユニット**。request 応答 HGLOBAL（ホスト解放）は下流。

## Out of Boundary

- `request` の呼出・SHIORI/3.0 セマンティクス・Value・UTF-8 charset。
- 常駐 lifecycle・`unload` の恒常呼出・`OnSecondChange`・crash 監視。
- transport／framing／handshake の改変（上流が凍結済み）。
- 里々/YAYA・SAORI・native x64 化（M2 以降）。

## Upstream / Downstream

- **Upstream**:
  - `areka-P0-host32-ipc`（✅完了）: `MsgTag::Load` seam ＋ transport を提供。本ユニットは Load を結線するのみ。
  - `pilot-shiori-host-32`（✅ go 済・参照専用）: FFI シーケンス／charset 非対称／HGLOBAL 所有権の一次記録（README 学び #4/#5/#6）。コードは隔離・コピペ禁止。
  - `vendors/pasta`（`crates/pasta_shiori`）: flat-C ABI のバイト正確確認源（`extern "C"` cdecl）。
  - `doc/COMPAT_ARCHITECTURE.md`（正本・過去互換経路 §82–88）。
- **Downstream**:
  - `areka-P0-host32-request`（同 proxy の `request` を呼び SHIORI/3.0 往復＝emo2 OnBoot Value 受領）。
  - `areka-P0-host32-lifecycle`（常駐ループ生存 ＋ `unload` 呼出 ＋ crash 監視）。
  - `areka-P0-conductor` 以降（Value を sakura-engine へ）。

## Existing Spec Touchpoints

- **Extends**: なし（新ユニット・just-in-time brief。spec 工場は回避）。
- **Adjacent**: `areka-P0-host32-ipc`（`MsgTag`/helper `handle_message` の結線点・改変せず拡張）／`areka-P0-host32-request`・`areka-P0-host32-lifecycle`（同 `ShioriByteProxy` を後続で共有）。

## Constraints

- **ビルド**: i686 helper は **PowerShell 必須**（Git Bash の GNU `link.exe` が MSVC link を遮蔽するトラップ・[[arm64-windows-build]] と同根）。共有モジュールは `cargo test --target i686-pc-windows-msvc` も回す。
- **32bit 可搬性**: i686 で `usize`=32bit ゆえ `(x as usize) >> 32` は overflow lint でコンパイルエラー → dwData/ULONG_PTR 系演算は `u64` で評価。
- **ABI**: cdecl `extern "C"`。返り値は **Rust `bool`(1byte)**（Win32 BOOL でない）。`request` の `len` は **in/out**（入力長を先に書く）。HGLOBAL=`GlobalAlloc(GMEM_FIXED)` 生ポインタ（`GlobalLock` 不要）で **IPC を跨がない**（32bit ローカル）。
- **charset**: `load` の dir は **ANSI(CP_ACP/Shift_JIS)**。Shift_JIS は windows crate の CP_ACP（`WideCharToMultiByte`）で足り、`encoding_rs` は不要。
- **実バイナリ内部前提を置かない**（README 学び #5）: 依存してよいのは観測可能な契約（`request` の block-on-reply・clean unload）のみ。内部スレッド等の仮説には依存しない。本ユニットは `load` の同期 bool 返却のみを観測契約とする。
- **命綱（葉ノード隔離）**: production クレートは `crates/pilot` へ inbound 依存しない。
- 制約変更の正本は `doc/COMPAT_ARCHITECTURE.md`。

## Open Questions（design フェーズで決める・discovery のブロッカーではない）

1. **LOAD payload の符号化**: dll path ／ ghostdir を wire で何のバイト列（UTF-8／UTF-16）として渡し、helper が各 API 用に何へ transcode するか（`LoadLibraryW`=UTF-16／`load` dir=ANSI）。`ProcessHost::spawn` は既に ghostdir を運ぶため、dll path/ghostdir を spawn 引数と LOAD payload のどちらに載せるかも含む。
2. **load ack の返し方**: bool 結果を `MsgTag::Response` に 1byte で載せるか、専用 ack 形にするか（transport は改変しない範囲で）。
3. **emo2 `pasta.dll` fixture の供給**: 実 fixture は現状 pilot の `fixtures/emo2` にある。host32 crate のテストが pilot に依存すると葉ノード隔離違反 → 共有 test fixture の置き場／取り込み方を決める。
4. **teardown unload の扱い**: load 成功観測後、テスト後始末で `unload`/`FreeLibrary` を本ユニットで呼ぶか、load-only 観測に留めるか（常駐 lifecycle は下流所有を維持しつつ）。
