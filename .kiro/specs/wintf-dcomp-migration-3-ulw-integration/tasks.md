# 実装計画: wintf-dcomp-migration-3-ulw-integration

## タスク概要

Phase 3 — UpdateLayeredWindow 統合。合成済みビットマップを ULW でウィンドウに転送し、alpha 透過 + クリックスルーを実現する。

---

## 実装タスク

### Phase 3A: 前提検証（Phase 3 開始前）

- [ ] 1. (P) WS_EX_LAYERED 動作検証
  - 最小構成の `WS_EX_LAYERED` ウィンドウを作成し、WM_PAINT 発火動作を確認する
  - `pptDst=None` での UpdateLayeredWindow 呼び出し時にウィンドウ位置が維持されるかを確認する
  - alpha=0 ピクセルのクリックスルー動作を確認する
  - 検証結果を design.md §5.2 の設計分岐に反映する
  - _Requirements: 4.3, 7.1_
  - _Dependencies: Phase 2 完了_

### Phase 3B: ULW コア実装

- [ ] 2. present_layered_window 関数の実装
  - `com/ulw.rs` に `present_layered_window(hwnd, hdc_src, size)` を実装する
  - `BLENDFUNCTION` を `AC_SRC_OVER, SourceConstantAlpha=255, AC_SRC_ALPHA` で構成する
  - Task 1 の検証結果に基づき `pptDst` の扱いを決定する
  - _Requirements: 2.1, 2.2, 2.3_
  - _Dependencies: Task 1_

- [ ] 3. ulw_present_system の実装
  - `ecs/graphics/compositor_systems.rs` に `ulw_present_system` を追加する
  - ダーティフラグチェック → `present_layered_window` 呼び出し → dirty=false の基本フローを実装する
  - ULW 失敗時は `tracing::warn!` + dirty 維持（次フレーム再試行）を実装する
  - _Requirements: 1.1, 1.2, 1.4, 6.1, 6.2, 6.3_
  - _Dependencies: Task 2_

### Phase 3C: ウィンドウスタイル・ハンドラ更新

- [ ] 4. (P) WS_EX_LAYERED 切替
  - `ecs/window.rs` の `WindowStyle::default()` を `WS_EX_LAYERED` に変更する
  - `areka/src/main.rs` の Shell/Balloon の `ex_style` を `WS_EX_LAYERED` に変更する
  - `WS_EX_TOOLWINDOW | WS_EX_TOPMOST` は維持する
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Dependencies: Task 3_

- [ ] 5. WM_PAINT / WM_ERASEBKGND / WM_SIZE ハンドラ更新
  - Task 1 の検証結果に基づき WM_PAINT ハンドラを更新する（BeginPaint/EndPaint 最小ペア or 不要化）
  - WM_ERASEBKGND ハンドラで `LRESULT(1)` を返す
  - WM_SIZE の既存フロー（ECS リアクティブ）が ULW 方式で正しく動作することを確認する
  - _Requirements: 4.1, 4.2, 4.3, 5.1, 5.2_
  - _Dependencies: Task 1, Task 4_

### Phase 3D: Schedule 登録・検証

- [ ] 6. (P) world.rs CommitComposition ステージ更新
  - CommitComposition ステージの `commit_composition` を `ulw_present_system` に置換する
  - _Requirements: 1.3_
  - _Dependencies: Task 3_

- [ ] 7. Phase 3 完了検証
  - ULW 透過ウィンドウ表示が動作することを確認する
  - alpha=0 クリックスルーが動作することを確認する
  - WM_SIZE リサイズが正常動作することを確認する
  - ULW 失敗時のログ出力 + 再試行が動作することを確認する
  - 全 example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）の動作を確認する
  - `cargo test` 全テストパスを確認する
  - _Requirements: 7.1, 7.2, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_
  - _Dependencies: Task 4, Task 5, Task 6_

---

## 依存関係サマリー

```
Task 1 (P) ──→ Task 2 ──→ Task 3 ──┬→ Task 4 (P) ──→ Task 5 ──┬→ Task 7
                                   └→ Task 6 (P) ─────────────┘

並列実行可能: Task 1 (P) — 即座開始 | Task 4 (P) & Task 6 (P) — Task 3 完了後
```

## 要件カバレッジサマリー

| 要件                           | タスク |
| ------------------------------ | ------ |
| Req 1 (ulw_present_system)     | 3, 6   |
| Req 2 (present_layered_window) | 2      |
| Req 3 (WS_EX_LAYERED)          | 4      |
| Req 4 (WM_PAINT/ERASEBKGND)    | 1, 5   |
| Req 5 (WM_SIZE)                | 5      |
| Req 6 (ULW失敗)                | 3      |
| Req 7 (クリックスルー)         | 1, 7   |
| Req 8 (Phase 3検証)            | 7      |
| Req 9 (テスト互換性)           | 4, 7   |
| Req 10 (前提検証)              | 1      |

全10要件がタスクにマッピング済み。
