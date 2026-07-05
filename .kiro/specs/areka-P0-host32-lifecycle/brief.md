# Brief: areka-P0-host32-lifecycle

> **種別**: 本坑（main）。① shiori トラック**最終ユニット**（逐次チェーン: pilot✅ → ipc✅ → shiori-load✅ → request✅ → **本ユニット**）。
> **調査日**: 2026-07-05（request 完了後の実地 API 調査済み）。

## Problem

host-32 は「load 貫通・request 往復」まで完成したが、**常駐運転の健全性が未証明**——ゴーストは分単位・時間単位で生き続け毎秒 request を受ける存在であり、①helper の長時間 msg loop 生存 ②周期 request 連打への耐性 ③helper 異常（crash/強制終了）の**検出と観測可能な報告** ④clean shutdown の全経路、が実証されていない。ここが埋まらないと kanade（毎秒 pump の運行）が砂上に立つ。

## Current State

- **request 完了時の資産（実シンボル）**: `Shiori3Client::get/notify`（同期往復・`RequestError{Handshake/Timeout/Ipc/Shiori}` 区別語彙）／`spawn(parent_hwnd, load_dir, shiori_name) -> HelperHandle`／**`HelperHandle::poll_exit()/poll_exit_kind()`（非ブロッキング）＋ `ExitKind::{Clean, Abnormal(i32), Terminated}`**＝死活検出の seam は既に存在（監視ループ化は未実装）。
- タイムアウト: `AREKA_SHIORI_REQUEST_TIMEOUT_MS`（既定 60s・0=無限 debug）・LOAD_ACK 30s。
- 常駐系の未実装確認済み: 周期運転試験・crash 監視ループ・異常時の上位報告経路は無し（TODO も未設置＝本ユニットが定義する）。
- teardown: `ShioriByteProxy` Drop（courtesy unload＋FreeLibrary）＝load 層で確立済み。

## Desired Outcome

実 i686 helper が**長時間運転（周期 request 連打）で健全**であり、**異常終了が検出・区別・報告**され、**clean shutdown の全経路**（通常終了・異常後の後始末）が決定的に通る。

**✔ 観測（単一 pass/fail）**: 実 i686 helper（testdll fixture）で (a) **N 秒運転**＝周期 request 連打（OnSecondChange 相当の頻度・イベント意味論なしのダミー ID で可）→全往復成功→clean unload・`ExitKind::Clean` (b) **強制 kill 注入**→監視が `Abnormal/Terminated` を検出し観測可能なエラーとして上位へ報告（ハング・無限待ちなし）。（実 pasta は env-gate 追験）

## Approach

1. **常駐監視**: `HelperHandle::poll_exit` を用いた**死活監視**を host 側に常設（request 前後 or 周期チェック——design 判断。専用監視スレッドは actor 化の先取りをしない＝親窓スレッド内の poll で足るか design で確定）。
2. **異常の語彙統一**: helper 死亡時の request 失敗（`Ipc`/`Timeout`）と `ExitKind` を突合し、呼び手（将来 kanade）が「helper が死んだ・応答しないだけ・SHIORI がエラーを返した」を**単一の語彙で区別**できる報告型へ整理（`RequestError` の拡張 or 包む型は design 判断）。
3. **周期運転試験**: 連打ハーネス（決定的・実時間 sleep 最小化）＋長時間相当の反復で leak/handle 枯渇/ResponseSlot 巻き込みなしを確認。
4. **shutdown 全経路**: 通常（unload→プロセス終了→Clean 確認）／異常後（`Abnormal` 検出→ハンドル後始末・二重 kill 安全）。**再起動戦略は持たない**（検出と報告まで。自動再起動の判断は kanade/ghost の M2 領分）。

## kanade との境界（申し開き・重要）

- **本ユニット＝host32 層の常駐健全性の証明＋死活報告 API**。周期 request は「耐性試験の負荷」であって**イベント意味論を持たない**（ID はダミーで可）。
- **kanade＝実イベントの運行表**（OnSecondChange の Reference 構成・発火順序・Value 配送）。本ユニットの成果（死活語彙・監視 seam）を kanade がそのまま消費する——**報告型は本 brief が正本**（kanade brief は消費・再定義しない）。

## 通信モデル

- 親窓＝pump スレッド専有（request brief と同旨）。本ユニットは **areka-actor 非依存**（先行可）——ただし死活報告 API は将来 shiori アクターの inbox 処理から呼ばれる前提で `Send` な所有データ・非ブロッキングに切る。

## ukadoc 必読

- 本ユニットは**イベント意味論非依存**（ukadoc 参照は最小）。OnSecondChange の Reference 詳細・発火規律は **kanade の領分**（kanade brief の必読リスト参照）。unload の作法は shiori-load 完了時の確立済み契約（courtesy unload）を踏襲。

## Scope

- **In**: 死活監視（poll_exit 常設化）／異常語彙の統一報告型／周期運転・強制 kill 注入試験／shutdown 全経路／（env-gate）実 pasta 長時間追験。
- **Out**: イベントカタログ・OnSecondChange の意味論・boot/close 運行（**kanade**）／自動再起動・縮退戦略（M2・判断は上位）／IPC フレーム変更（凍結）／SAORI。

## Boundary Candidates

- 死活監視（poll 常設）／報告型（語彙統一・純粋）／試験ハーネス（連打・kill 注入）の三片。

## Upstream / Downstream

- **Upstream**: `areka-P0-host32-request` ✅（Shiori3Client・RequestError・HelperHandle/ExitKind）。
- **Downstream**: `areka-P0-kanade`（毎秒 pump が本ユニットの健全性保証と死活語彙の上に立つ）／`ghost-setup`（終了系列）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-host32-request`（HelperHandle/ExitKind seam の常設化）。
- **Adjacent**: `shiori-host32-ipc`（凍結・不改変）。

## Constraints

- Rust 2024・tokio 禁止。helper=i686（**PowerShell ビルド必須**）・i686 テストのサイレントスキップ禁止。
- **ログ無し失敗経路の禁止**（error!＋Err 戻り値・panic は致命限定＋直前ログ＝開発者指示 2026-07-04）。
- 実時間 sleep に依存しない決定的テスト（request の先例踏襲）。
