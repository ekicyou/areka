# Brief: areka-P0-host32-ipc（本坑 / main・M1 M-boot / SHIORI 通信層 host-32 トラック）

> **種別**: 本坑（main）。M1 `areka-P0-emo2-boot` の **① SHIORI 通信層エンジン host-32** トラックの**先頭ユニット**。
> **ゲート**: 先進坑 `pilot-shiori-host-32`（**✅ go 済み 2026-07-01**）が host-32 トラックを gate ＝ **解禁済み・着手可**。pilot 一次記録は `crates/pilot/examples/shiori-host-32/`（README/REPORT・3 幕）。
> **規律（二坑）**: pilot コードは**隔離・参照のみ**。**コピペ donor 禁止・クリーンに掘り直す**（README/REPORT の検証結果を参照）。
> **並走性**: 別プロセス＝天然のアクター境界ゆえ **parser/wintf トラックと非衝突・安全並走**（Line 3 の頭）。

## Problem

x64 areka が emo2 の 32bit `pasta.dll` を駆動するには、**x64↔i686 の IPC・ハンドシェイク・プロセス生存管理の transport 層**が要る。pilot で feasibility は実証済み（耐力壁突破）ゆえ、本坑でその transport をクリーンに実装する。本ユニットは **bytes-over-wire 層まで**（中身の SHIORI/3.0 や DLL ロードは下流ユニット）。

## Current State（調査済み・参照元と接続先）

- **pilot（隔離・参照のみ・`crates/pilot/examples/shiori-host-32/`）**:
  - `ipc.rs`（共有）: WM_COPYDATA プロトコル（`MsgTag`・**u32 LE の HWND** wire 表現・raw byte payload・`cbData` 長）。
  - `main.rs`（x64 親）: ParentDriver・spawn・lifecycle・orchestration。
  - `parent_window.rs`（x64）: ParentMessageWindow＋**ResponseSlot（再入 RESPONSE 受信）**。
  - `process_host.rs`（x64）: helper spawn・非ブロッキング `poll_exit()`・ExitKind（Clean/Abnormal）分類。
  - `helper.rs`/`helper_window.rs`（i686）: HelperMessageWindow（`wintf-winmsg-executor` 0.0.5）・msg loop・WndProc。
  - 依存: `wintf-winmsg-executor` 0.0.5（両 target・i686 実証済）／`windows`（DataExchange/Memory/Globalization）／`windows-core`／`event-listener` 5。
- **x64 SHIORI ABI（接続先・残す基盤）**: `crates/shiori-abi`（`IShiori`/`IShioriHost` COM・HSTRING・IID 既定義）。host-32 は **x64 側でこの ABI を実装**し、IPC 越しに i686 proxy へ marshalling する（本ユニットは bytes transport まで・ABI 実装本体は下流と design で結線）。
- **実証済み申し送り（README/REPORT・本坑が担ぐ）**:
  - **WM_COPYDATA 一方向で GO**: x64 は `SendMessageTimeout`（ブロック）、i686 の**再入 RESPONSE WndProc** が payload を ResponseSlot へ配送し即 return。以降のクロスプロセス SendMessage を出さない＝**デッドロック無し**（single-in-flight・厳密ネスト）。named pipe は push 要時のみ（本ユニットは不要）。
  - **跨ビットネスは raw bytes only**: HGLOBAL=i686 local／HSTRING=x64 local／**HWND=u32 LE**（USER ハンドルは 32bit 有効）。
  - **PowerShell build 必須**（Git Bash の GNU `link.exe` が MSVC link を遮蔽）。i686 は `usize=32bit`＝`(x as usize)>>32` は overflow → **u64 cast**。
  - `wintf-winmsg-executor` 0.0.5 は i686 で GO（**raw Win32 fallback 不要**）。

## Desired Outcome

x64 親が i686 helper を spawn → **HELLO handshake（HWND を u32 LE で交換）** → WM_COPYDATA で **往復 echo**（request bytes 送出 → response bytes 受領）→ timeout/wedge 検出 → プロセス生存監視（clean/abnormal 分類）。roadmap ✔「**往復 echo**」を無 crash で観測できる。

## Approach

本坑クレート（**配置は design 議題**・pilot は `examples/` に隔離ゆえ本坑は `crates/` 直下の新クレート＝x64 host lib ＋ i686 helper bin のペア想定）に、上記 transport を **pilot 知見を参照して一から実装**（コピペ禁止）。

- **seam の原則**: IPC は「**request bytes を送る・response bytes を受ける**」まで。中身（SHIORI/3.0 の build/parse）と `LoadLibrary pasta.dll` は下流ユニットの領分。
- スレッド跨ぎ通知は `event_listener` 既存パターン（tokio 禁止）。
- **i686 build は PowerShell で**（`usize=32bit` の shift overflow に注意）。

**既存コードに触れる/新クレートを立てる前に、配置と構成を依頼者へ提示して確認を取る**。

## Scope

- **In**: helper プロセス spawn＋非ブロッキング `poll_exit`。WM_COPYDATA framing（`MsgTag`／u32 LE HWND／`cbData`／raw payload）。HELLO handshake（HWND 交換）。ResponseSlot 再入受信（デッドロック回避）。timeout/wedge 検出（`SendMessageTimeout`＋`SMTO_ABORTIFHUNG`）。プロセス生存監視（Clean/Abnormal 分類）。**往復 echo** テスト。i686 helper ビルド（PowerShell）。
- **Out**: `LoadLibraryW` pasta.dll＋`GetProcAddress`＋load/unload/request 解決（`host32-shiori-load`）。SHIORI/3.0 request build＋marshal＋Value parse＋charset（`host32-request`）。常駐 msg loop＋`OnSecondChange` poll＋unload＋crash 監視（`host32-lifecycle`）。x64 `IShiori` ABI 実装本体（下流と design で結線）。pilot コードのコピペ。

## Boundary Candidates

- helper プロセス spawn／生存監視（poll_exit・ExitKind）
- WM_COPYDATA framing（MsgTag／u32 LE HWND／cbData）
- HELLO handshake（HWND 交換）
- ResponseSlot 再入 RESPONSE 受信
- timeout/wedge 検出

## Out of Boundary

- DLL ロード（`host32-shiori-load`）・SHIORI marshalling（`host32-request`）・常駐 lifecycle（`host32-lifecycle`）。
- pilot の領分（使い捨て検証・仮 selftest）。

## Upstream / Downstream

- **Upstream**: `pilot-shiori-host-32`（**go 済・参照専用**）／`crates/shiori-abi`（x64 `IShiori` ABI）／`wintf-winmsg-executor` 0.0.5。
- **Downstream**: `areka-P0-host32-shiori-load` → `areka-P0-host32-request` → `areka-P0-host32-lifecycle`（同トラックの chain）／`areka-P0-conductor`（SHIORI イベント循環が host-32 を送受）／`areka-P0-package-mount`（`ghost/master` パスが `shiori-load` のロード先）。

## Existing Spec Touchpoints

- **Extends**: `crates/shiori-abi`／`areka-P0-shiori-com`・`-shiori-protocol`・`-shiori-protocol-split`・`-shiori-reference`（completed・x64 側 SHIORI 基盤）。
- **Adjacent**: `pilot-shiori-host-32`（隔離・参照）。他 M1 トラック（parser/wintf）とは別プロセス/別クレートゆえ**非衝突**。

## Constraints

- Rust 2024・`windows` 0.62.2・`wintf-winmsg-executor` 0.0.5・`event-listener` 5・**tokio 禁止**。
- **PowerShell build 必須**（Git Bash link.exe 遮蔽）。i686 `usize=32bit`（shift overflow は u64 cast）。
- **跨ビットネスは raw bytes only**（HGLOBAL=i686 local／HSTRING=x64 local／HWND=u32 LE）。
- **pilot コードのコピペ禁止**（クリーン再掘・README/REPORT を参照）。32bit 可搬性を崩さない。
- 不確実な Win32 API/IPC 仕様は推測で進めず質問。設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本に。
