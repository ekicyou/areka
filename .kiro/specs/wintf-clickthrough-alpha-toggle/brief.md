# Brief: wintf-clickthrough-alpha-toggle（本坑 / main・完成品）

> **種別**: 本坑（main）。通常の kiro ライフサイクル（requirements → design → tasks → impl → complete）。PR ベース squash マージで `main` へ統合。
> **ゲート**: `_Depends(confirmed): pilot-clickthrough-alpha-toggle`。先進坑の go 判定が出るまで **BLOCKED**（go 前着手は二坑規律違反）。
> 規律の正本: `.kiro/steering/two-tunnel.md`。

## Problem

本体 `wintf` の透過は ULW/DComp 切替式だが、別プロセスへのαマスク連動クリック透過は **ULW** に依存している。ULW は CPU ビットマップ方式で **DComp スワップチェーン合成と併用不可**——つまり別プロセス透過を得るために 3D（DComp/GPU 合成）描画を諦める踏み絵になっている。**至上要件は DComp 描画を捨てないこと**（開発者）。DComp 経路は ULW のような自動αヒットテストを持たないため、`WS_EX_TRANSPARENT` 動的トグル方式（**他社 3D マスコット採用の実証済み手段**）を本体に導入し、**DComp 描画を維持したまま**キャラクター描画領域のみクリック可・透明領域は背面プロセスへ透過、を実現したい。

## Current State（調査済み・接続先）

- `CompositionMode` enum（`ecs/window/components.rs`）: ULW 既定／DComp、**生成時固定・動的切替非対応**。
- `compute_ex_style()`（`runtime/window_factory.rs`）: ULW→`WS_EX_LAYERED` 保持、DComp→`WS_EX_NOREDIRECTIONBITMAP`。拡張スタイル反映は `SetWindowLongPtrW(GWL_STYLE)`＋`SetWindowPos(SWP_FRAMECHANGED)`。
- `HitTestMode::AlphaMask`（`ecs/layout/hit_test/mod.rs`）＋ `AlphaMask::is_hit` / `AlphaMask::from_pbgra32`（`ecs/widget/bitmap_source/alpha_mask.rs`）: 既存のαマスク・ピクセル単位ヒットテスト（**本体αマスク関数の素材**）。`generate_alpha_mask_system` が非同期生成。
- `VsyncEventBridge`（`runtime/tick_bridge.rs`）: `event_listener::Event`＋`DwmFlush` による VSync→UI スレッド起床（スレッド跨ぎ通知の既存パターン）。
- D2D1 staging αバッファ（`ecs/graphics/compositor.rs`）: 合成済みαを CPU 読み取り可（`D2D1_BITMAP_OPTIONS_CPU_READ`）。

## Desired Outcome

本体 `wintf` で `WS_EX_TRANSPARENT` 動的トグル方式が動作し、**本体αマスク（実描画αバッファ／`AlphaMask`）参照でキャラクター領域のみクリック可能**になる。既存機能を壊さず、既存のリリースビルドフラグ・最適化設定と互換。pilot の知見をクリーンに掘り直す（コピペ donor 禁止・README 検証結果を参照）。

## Approach

先進坑 `pilot-clickthrough-alpha-toggle` の go 判定後、その知見を参照して一から綺麗に実装する。`WS_EX_TRANSPARENT` を別スレッドのカーソル監視＋本体αマスク問い合わせで状態変化時のみ付け外しする機構を wintf に組み込む。接続先は上記 Current State の列挙箇所。スレッド跨ぎ通知は既存 `event_listener` パターンに倣う（tokio 禁止）。

**既存コードに触れる前に、対象ファイルと変更内容を依頼者に提示して確認を取る**（推測で書き換えない）。

## ULW との共存方針（開発者決定）

至上要件は **DComp 描画の維持**であり、本方式は DComp 経路に透過能力を授けるもの。本仕様が完全に有効と判断されれば **ULW ルートは破棄**する。ただし他社実績ある手段とはいえ **十分な検証期間・エンバグ対応**を置き、**当面は ULW と並走**させる（即時撤去はしない）。確定時に `tech.md` line 83 ／ `roadmap.md` line 30 の「ULW 一択」記述を更新する。`WS_EX_LAYERED` の追加・`WM_NCHITTEST` ハンドラの追加は行わない（必要と判断したら理由とともに依頼者に確認）。

## Scope

- **In**: `WS_EX_TRANSPARENT` 動的トグル機構の本体実装。本体αマスク（実描画αバッファ／`AlphaMask`）との接続。カーソル監視ワーカ＋状態変化最適化。`docs/click_through.md` 新規作成（仕組み概要・ULW/HTTRANSPARENT/Layered を採らない理由・API 使用例・既知の制約）。既存機能の非破壊・リリースビルド互換の検証。
- **Out**: ULW バックエンドの即時撤去（並走期間中は残す）。pilot コードのコピペ流用。新しい大型クレート（winit/tauri 等）の追加（提案のみ・勝手に追加しない）。`Cargo.toml` 依存の大幅追加（最小限）。

## Boundary Candidates

- 拡張スタイル動的適用層（`compute_ex_style` / `SetWindowLongPtrW(GWL_EXSTYLE)` の動的化）
- カーソル監視ワーカ＋本体αマスク問い合わせ（`event_listener` パターン）
- 状態変化検出・適用最適化（前回状態比較）
- `CompositionMode`/DComp 経路とのクリック透過責務の整理

## Out of Boundary

- pilot の領分（仮αマスク・使い捨て検証）。
- M1（emo2-boot）の各エンジントラック（別軸）。

## Upstream / Downstream

- **Upstream**: 先進坑 `pilot-clickthrough-alpha-toggle` の go 判定（前提依存）。既存 ULW/DComp 切替基盤・`event-hit-test-alpha-mask`・`VsyncEventBridge`。
- **Downstream**: areka 本体マスコット表示（キャラ領域クリック・3D 描画時の軽量透過）。将来の ULW ルート破棄判断。

## Existing Spec Touchpoints

- **Extends**: 既存 ULW/DComp 切替基盤（`CompositionMode`・`com/ulw.rs`）／`event-hit-test-alpha-mask`（αヒットテスト）。並走後に置換の可能性。
- **Adjacent**: `wintf-winmsg-executor`（ウィンドウ生成 facade）／M1 emo2-boot トラック（窓配置・render-engine）。

## Constraints

- Rust 2024・`windows` 0.62.2 系・`event_listener` 5・**tokio 禁止**。32bit 可搬性を崩さない。
- マルチモニタ・高 DPI 環境を仮定（per-monitor-v2）。
- 既存リリース最適化（`opt-level='z'`, `lto=true`）と互換。
- 既存本体コードは推測で書き換えない（変更前に対象と内容を依頼者へ提示）。
- 設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本として更新。
- 不確実な Win32 API/クレート仕様は推測で進めず質問する。
