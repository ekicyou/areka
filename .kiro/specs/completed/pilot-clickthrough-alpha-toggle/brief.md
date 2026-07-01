# Brief: pilot-clickthrough-alpha-toggle（先進坑 / pilot・使い捨て）

> **種別**: 先進坑（pilot）。spec 名は `pilot-` 接頭辞で先進坑と明示（命名規約・`two-tunnel.md`）。成果物はコードではなく**知見**（go／違う／直す ＋ 学び）。
> 配置: `crates/pilot/examples/pilot-clickthrough-alpha-toggle/`（`main.rs` ＋ README 3 幕）。
> 規律の正本: `.kiro/steering/two-tunnel.md`。一次記録は example の README（＋本 PoC では `REPORT.md`）。

## Problem

Windows デスクトップマスコットは「キャラクター描画領域だけクリックを受け、透明領域は背面アプリ（別プロセス）へクリックを透過させる」のが中核要件。既存の `areka`/`wintf` はこれを **ULW（UpdateLayeredWindow）** の alpha-0 ピクセル OS 自動透過で実現済みだが、ULW は CPU ビットマップ方式で **DComp スワップチェーン合成と併用不可**——別プロセス透過のために 3D（DComp/GPU 合成）描画を諦める踏み絵になっている。**DComp 描画を捨てられない**のが至上要件。`WS_EX_TRANSPARENT` 動的トグルは **他社 3D マスコット採用の実証済み手段**で、DComp 描画を維持したまま透過を成立させ得る。本先進坑はその**実現可能性を使い捨てで先に潰す**（本実装は十分な検証・エンバグ対応を要する前提）。

## Current State

- `wintf` は ULW/DComp 切替式（`CompositionMode` enum・ULW 既定・生成時固定）を実装済み。別プロセス透過は ULW の alpha-0 自動透過に依存。
- `tech.md` line 83 / `roadmap.md` line 30 は別プロセス透過を **「実質 ULW 一択」** と断定する。しかしこの結論は **HTTRANSPARENT・SetWindowRgn・ULW** の 3 択比較で、**`WS_EX_TRANSPARENT` 動的トグル方式（winit `set_cursor_hittest` 相当・プロセス境界を越える第 4 の手）を検討していない**。
- DComp 経路は ULW のような自動αヒットテストを持たないため、DComp で別プロセス透過を成立させるには別手段が必要。`WS_EX_TRANSPARENT` トグルがその候補。
- 同等の「αマスク連動で自動切替する」Rust クレートは現時点で存在しない（`winit::set_cursor_hittest` / `tauri::set_ignore_cursor_events` は API はあるが自動切替はしない）。自前実装する。

## Desired Outcome

`WS_EX_TRANSPARENT` を αマスクに応じて動的に付け外しする方式が、**別プロセスへのクリック透過を成立させること**を使い捨ての先進坑で実証し、開発者（人間）が **go／違う／直す** を判定できる状態にする。「クリックスルーの状態切替が正しく動くこと」「不透明領域が実際にクリックできること」が核心。

## Approach

先進坑（pilot・使い捨て）として独立 example を作る。`_template` をコピーして着手。

- 透過したトップモストのウィンドウを生成。全域透明・中央に「不透明な四角領域」を定義。
- 別スレッド（`event_listener` ＋ `std::thread`・**tokio 禁止**）が **16ms 周期**でカーソル位置を取得し、αマスク関数（仮実装・画面中央 (960,540) 半径 200px の円の外を透明扱い）に問い合わせる。
- 円内＝クリックスルー OFF、円外＝クリックスルー ON を動的に切替。**状態変化したフレームでのみ** `SetWindowLongPtr(GWL_EXSTYLE)` ＋ `SetWindowPos(SWP_FRAMECHANGED)` を呼ぶ（毎フレーム呼び出し禁止・前回状態と比較）。
- 切替が起こるたびにログ出力。四角領域のクリックでもログ出力＋四角の色をトグル変更。
- `main` 冒頭で `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` を必ず呼ぶ。
- `WS_EX_LAYERED` は付けない（`WS_EX_TRANSPARENT` 単独で別プロセス透過が成立）。`WM_NCHITTEST` を自前ハンドルしない（HTTRANSPARENT は別プロセスに届かず無意味）。
- HWND は `!Send` だが `unsafe impl Send for AppState` でラップしてよい（Win32 慣例）。

## Scope

- **In**: 上記 PoC の最小実装。試験項目 T1〜T8 の手動検証。`REPORT.md`（指定フォーマット）＋ README 3 幕の作成。
- **Out**: 本体 `wintf`/`areka` への接続（フェーズ2＝本坑の領分）。実描画αバッファ参照（PoC は仮の円判定でよい）。ULW の撤去。新しい大型クレート（winit/tauri 等）の追加。先進坑コードの production 流用（コピペ donor 禁止）。

## Boundary Candidates

- ウィンドウ生成＋透過トップモスト化（`WS_EX_TRANSPARENT` 制御の口）
- カーソル監視ワーカ（16ms 周期・別スレッド・スレッド跨ぎ通知）
- αマスク関数の差し替えシーム（仮の円判定 → 将来は実描画αバッファ）
- 状態変化検出＋拡張スタイル適用（状態変化最適化）

## Out of Boundary

- 本体αマスク関数（実描画αバッファ参照）との接続：本坑 `wintf-clickthrough-alpha-toggle` が所有。
- ULW/DComp バックエンドの改変：本坑の領分。

## go 基準（人間判断・AI 単独で判定しない）

PoC をビルド・実行し、以下を**すべて手動検証**して `REPORT.md` に記載する。検証は人間とともに実施（人間の準備確認 → エージェントがプログラム起動 → 結果をヒアリング）。

| # | 試験項目 | 期待結果 |
|---|---|---|
| T1 | 起動確認 | 透過トップモスト窓が表示される |
| T2 | 円外でのクリック透過 | 背面アプリ（デスクトップアイコン等）が反応 |
| T3 | 円内でのクリック受領 | WndProc に WM_LBUTTONDOWN が届く |
| T4 | 状態切替の発火 | 円境界をまたぐ瞬間に ON↔OFF ログ |
| T5 | 状態変化なし時の非発火 | 留まっている間は SetWindowPos 非呼び出し |
| T6 | マルチプロセス透過 | 背面ブラウザのリンクが円外クリックで開く |
| T7 | DPI 環境での座標一致 | 高 DPI（150% 等）でも円判定が見た目と一致 |
| T8 | 終了処理 | 窓を閉じるとプロセス・ワーカスレッドが正常終了 |

- **合格基準**: T1・T2・T3・T4・T6 が ✅ 必須。T5・T7・T8 は ✅ または軽微な条件付き合格（理由明記）。
- 合否を問わずレポートを作成し依頼者の判断を仰ぐ。**Claude Code 単独で「合格判定」して次フェーズに進まない。**

## Upstream / Downstream

- **Upstream**: 二坑モデル（`two-tunnel.md`）の先進坑規律。`crates/pilot` の検疫所構造（空 lib ＋ examples-only）。`pilot/Cargo.toml` の既存依存（`windows`/`windows-core`/`event-listener`・tokio 不在）。
- **Downstream**: 本坑 `wintf-clickthrough-alpha-toggle`（本 pilot の go 判定を `_Depends(confirmed):` 前提依存とする）。

## Existing Spec Touchpoints

- **Extends**: なし（先進坑は production に被依存しない葉ノード隔離）。
- **Adjacent**: 完了済み `event-hit-test-alpha-mask`（既存αヒットテスト）／ULW 切替基盤（`com/ulw.rs`・`CompositionMode`）。本坑で接続するが pilot では触れない。

## Constraints

- Rust 2024・`windows` 0.62.2 系・`event_listener` 5・**tokio 禁止**（`event_listener` ＋ `std::thread` で組む）。
- 葉ノード隔離厳守（`examples/` のみ・他クレートへ inbound ゼロ）。32bit 可搬性を崩さない。
- マルチモニタ・高 DPI 環境を仮定（プライマリのみ前提にしない）。
- Win32 API/クレート仕様で不確実な点は推測で進めず質問する。
- worktree 実行時は `git submodule update --init --recursive` を前段で要する（既知制約）。
