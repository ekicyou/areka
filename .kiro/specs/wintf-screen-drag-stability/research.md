# ギャップ分析レポート: wintf-screen-drag-stability（拡張版）

## 分析サマリー

- ドラッグ不安定は2つの**主要根因**と複数の副次根因の組み合わせで発生
- **主要根因1 (P1)**: `Arrangement.offset` のステール化とカスケード巻き戻し — ドラッグ中に間接レイアウト再計算が走ると古い座標が復活
- **主要根因2 (S2→P2に格上げ)**: マウスキャプチャ未実装 — DPI変更でウィンドウ縮小時、マウスが外に出ると `WM_MOUSEMOVE` が届かずドラッグが途切れる（**実測で確認済み**）
- `SetCapture`/`ReleaseCapture` は windows 0.62.2 で利用可能（`Win32::UI::Input::KeyboardAndMouse`）であり、TODOは古い前提
- `WindowDragging` マーカーはライフサイクル実装済みだが、**レイアウト/WindowPos反映系で未参照** のため防御機構として機能していない
- 設計フェーズでは「原因仮説を1件ずつ有効化/無効化して評価」する実験計画が妥当

---

## 1. 調査対象と現状

### 1.1 主要経路（今回の深掘り対象）

- 入力経路: `ecs/window_proc/mouse_click.rs`, `mouse_move.rs`, `keyboard.rs`, `pointer/nchittest_cache.rs`
- 位置同期経路: `ecs/window_proc/window_pos.rs`, `ecs/window/command.rs`, `ecs/layout/systems/window_pos_systems.rs`
- レイアウト経路: `ecs/layout/systems/taffy_systems.rs`, `monitor_systems.rs`
- フレーム駆動経路: `ecs/world/mod.rs`, `ecs/world/vsync.rs`, `win_thread_mgr.rs`
- 既存検証: `tests/layout/boxstyle_coordinate_separation_test.rs`, `tests/layout/feedback_loop_convergence_test.rs`, `tests/window/multiwindow_event_test.rs`

### 1.2 実行順序（不安定性に関係する順）

`try_tick_world()`:

`Input → Update → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup → Draw → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize`

PostLayout 内:
1. `sync_window_arrangement_from_window_pos` (`Changed<WindowPos>`)
2. `sync_simple_arrangements`
3. `mark_dirty_arrangement_trees` (`Changed<Arrangement>`)
4. `propagate_global_arrangements`
5. `window_pos_sync_system` (`Changed<GlobalArrangement>`)

UISetup:
- `apply_window_pos_changes` (`Changed<WindowPos>` → `SetWindowPosCommand::enqueue`)

---

## 2. なぜなぜ分析（原因ツリー）

## 2.1 主要根因（Primary）

### P1: ステール offset による巻き戻しカスケード

**なぜ不安定になるか**

1. ドラッグ中は `WM_WINDOWPOSCHANGED` で `bypass_change_detection()` が使われ、`Changed<WindowPos>` が発火しない
2. その結果、`sync_window_arrangement_from_window_pos` が走らず `Arrangement.offset` が古いまま残る
3. ドラッグ中に別契機（`BoxStyle`/`TaffyStyle`/`Monitor`更新）で `update_arrangements_system` が走る
4. `update_arrangements_system` は Window の offset を「既存値維持」するため、古い offset を温存したまま `Changed<Arrangement>` が立つ
5. `propagate_global_arrangements` → `window_pos_sync_system` が古い座標を `WindowPos` に書き戻す
6. `apply_window_pos_changes` が古い座標で `SetWindowPos` を再送し、ウィンドウがジャンプ/巻き戻り

**コード根拠**
- `window_proc/window_pos.rs`（echo時 bypass）
- `layout/systems/window_pos_systems.rs`（GA→WindowPos, WindowPos→Arrangement）
- `layout/systems/taffy_systems.rs`（Window offset維持ロジック）

**状態**: `PARTIAL`（直接ループは防止済み、間接カスケードは未防止）

---

## 2.2 主要根因2（旧S2から格上げ）

### P2: マウスキャプチャ未実装によるイベント欠落（**実測確認済み**）

**なぜスクリーン境界で途切れるか**
- DPI変更（例: 200% → 125%）でウィンドウが縮小すると、マウスカーソルがウィンドウ領域外に出る
- `SetCapture` がないため、ウィンドウ外に出た瞬間 `WM_MOUSEMOVE` / `WM_LBUTTONUP` が届かなくなる
- ドラッグ状態が途切れる（マウスを動かしてウィンドウ内に戻ると再開する — **実測で確認**）

**コード根拠**
- `mouse_click.rs` L165, L223, L296: SetCapture/ReleaseCapture の TODO コメント（実装なし）
- `keyboard.rs` L56: ReleaseCapture の TODO コメント（実装なし）

**状態**: `MISSING`（最優先実装項目）

---

## 2.3 副次根因（Secondary）

### S1: `WindowDragging` が防御条件として未接続

**なぜ効いていないか**
- `dispatch_drag_events` で `WindowDragging` は付与/除去される
- しかし、`window_pos_sync_system` / `apply_window_pos_changes` / `sync_window_arrangement_from_window_pos` が `Without<WindowDragging>` 等を持たない
- 結果、ドラッグ中も通常同期系が動き、P1 を誘発できる

**状態**: `MISSING`

---

### S2: `WM_CAPTURECHANGED` 未ハンドリング

**なぜ問題化するか**
- 外部要因で capture を失った時に drag state を明示的に終了できない
- `WM_CANCELMODE` 未発火のケースで状態残留リスク

**状態**: `MISSING`

---

### S3: `WM_NCHITTEST` キャッシュのキー不足（座標のみ）

**なぜ不整合が起きうるか**
- キャッシュキーが `(HWND, screen_point)` のみで、**ウィンドウ位置変化**を考慮しない
- ドラッグ中は同一 `screen_point` でもクライアント座標系が実質変化しうる
- tickまでキャッシュ保持されるため、短時間に複数回 NCHITTEST が来ると古い判定（HTCLIENT/HTTRANSPARENT）が再利用される可能性

**実装事実**
- キャッシュクリアは `try_tick_world()` 終了時のみ

**状態**: `RISK`（再現条件依存、未検証）

---

### S4: `flush_window_pos_commands()` の重複呼び出し

**なぜ影響しうるか**
- `try_tick_on_vsync()` 内で flush 実行
- `WM_WINDOWPOSCHANGED` ハンドラの末尾でも flush 実行
- 通常は2回目が no-op でも、境界条件でコマンド生成タイミングが重なると追加 SetWindowPos 実行の窓が広がる

**状態**: `RISK`

---

### S5: `try_borrow_mut()` 失敗時の入力イベントドロップ

**なぜガタつきに見えるか**
- `WM_MOUSEMOVE` は `try_borrow_mut()` 失敗時に処理できず戻る経路がある
- 高頻度メッセージ時に間欠的にサンプル欠落すると、delta累積が飛び、視覚的にカクつく

**状態**: `RISK`

---

### S6: 位置計算の情報源が `WindowPos.position` 依存

**なぜずれが増幅しうるか**
- `screen_x/screen_y = lparam(client) + WindowPos.position`
- `WindowPos.position` は echo時 bypass更新でECS change tickと切り離される
- 他系統更新とタイミング差があると、delta計算の基準が瞬間的に不連続になる可能性

**状態**: `RISK`

---

### S7: ディスプレイ構成変更フラグ経由の再レイアウト干渉

**なぜ境界操作で発火しやすいか**
- `WM_DISPLAYCHANGE` 後、`detect_display_change_system` が Monitor entity/BoxStyle を更新
- これがレイアウト再計算契機となり P1 カスケードに合流可能

**状態**: `PARTIAL`（機能として正しいがドラッグ排他なし）

---

## 2.4 設計上の非採用/未活用メッセージ（観測）

- `WM_MOVING`, `WM_MOVE`, `WM_WINDOWPOSCHANGING`, `WM_ENTERSIZEMOVE`, `WM_EXITSIZEMOVE` は ECS wndproc で未使用
- 現在は `WM_WINDOWPOSCHANGED` を中心に構成されているため、OS標準移動ループ由来の補助情報を活用していない

**状態**: `UNKNOWN`（必須ではないが診断シグナルとして有用）

---

## 3. 要件別ギャップ（拡張）

### Requirement 1: マウスキャプチャ

- AC1/AC2/AC3/AC4/AC5: **MISSING**（**P2根因: DPI変更時のウィンドウ縮小→マウス外出→イベント欠落で実測確認済み**）
- 追加ギャップ: `WM_CAPTURECHANGED` 未処理

### Requirement 2: スクリーン境界での位置安定性

- AC1: **PARTIAL**（通常追従は成立）
- AC2: **EXISTING**（直接echo防止）
- AC3: **PARTIAL**（間接再送が残る）
- AC4: **MISSING**（`WindowDragging` 未接続）
- AC5: **MISSING** → R1（マウスキャプチャ）で解決される副次的問題

### Requirement 3: 同一DPI環境での安定性

- AC1/AC2: **EXISTING**
- AC3/AC4: **PARTIAL**（カスケードや入力欠落で破綻余地）
- 追加ギャップ: `NCHITTEST` キャッシュ整合、borrow失敗時ドロップ

### Requirement 4: グラフィックス安定性

- AC1: **EXISTING**（VSYNC駆動）
- AC2: **PARTIAL**（不要再送経路残る）
- AC3: **MISSING**（ドラッグ中排他ポリシー不足）
- AC4: **EXISTING**（ドラッグ単体でHasGraphicsResources変更なし）

### Requirement 5: 終了整合性

- AC1/AC2/AC3: **EXISTING**（正常系）
- AC4/AC5: **MISSING**（capture解放保証が未整備）
- 追加ギャップ: キャンセル時のロールバック方針が曖昧

---

## 4. テストカバレッジ・ギャップ

### 4.1 既存で検証されていること

- `WindowDragging` の付与/除去ライフサイクル
- `sync_window_arrangement_from_window_pos` の基本収束
- `update_arrangements_system` の Window offset維持
- マルチウィンドウ時の owner window ガード

### 4.2 未検証（今回追加した重要ギャップ）

1. **ドラッグ中 + レイアウト再計算契機** での巻き戻し再現テスト
2. **SetCaptureあり/なし比較**（境界外での `WM_LBUTTONUP` 欠落）
3. **`WM_CAPTURECHANGED` 受信時の DragEnd 保証**
4. **NCHITTESTキャッシュが移動中に古くなるケース**
5. **`try_borrow_mut` 失敗時の入力欠落耐性**
6. **重複 flush の副作用有無**

---

## 5. 実装アプローチ（設計フェーズでの試行前提）

### Option A: 既存拡張（最小差分）

- `SetCapture/ReleaseCapture` 実装（**P2根因を解決、最優先**） + `WM_CAPTURECHANGED` ハンドラ追加
- `window_pos_sync_system` / `apply_window_pos_changes` に `Without<WindowDragging>`
- 必要に応じて `sync_window_arrangement_from_window_pos` にも同フィルタ

**利点**: 変更範囲が狭く、P1/P2/S1/S2 を一気に低減

### Option B: ドラッグ専用ガード層新設

- DragGuard resource/system を作り、位置同期・描画同期の排他を集中管理

**利点**: 将来拡張に強い
**欠点**: 今回スコープでは過大

### Option C: ハイブリッド（A + 診断強化）

- Option A に加えて、`NCHITTEST` キャッシュ条件拡張と flush経路の単純化、観測ログ追加

**利点**: 真因切り分け効率が高い

---

## 6. 設計フェーズ向け「1件ずつ試行」計画（提案）

1. **実験1: SetCapture/ReleaseCapture 導入のみ**（**最優先: P2根因を解決**）
   - 目的: DPI変更時のウィンドウ縮小→マウス外出→イベント欠落を解消できるか検証
   - 期待: 200%→125% DPI 移動時のドラッグ途切れが解消
2. **実験2: `WindowDragging` フィルタ導入のみ**
   - 目的: P1巻き戻しカスケードの有無を確認
3. **実験3: Capture + フィルタ併用**
   - 目的: 主要2根因（P1/P2）の相互作用評価
4. **実験4: NCHITTESTキャッシュ条件変更**
   - 目的: 境界付近のヒットテスト由来揺らぎを検証
5. **実験5: flush経路単純化**
   - 目的: 重複SetWindowPos可能性の排除
6. **実験6: borrow失敗時の計測/再試行方針**
   - 目的: 入力ドロップの影響定量化

---

## 7. 工数・リスク

| 項目 | 評価 | 根拠 |
|------|------|------|
| 工数 | **M (3-7日)** | 実装修正は小さいが、原因分離実験と回帰確認に時間が必要 |
| リスク | **Medium** | 既存設計への追従は容易。ただしイベント順序依存の再現試験が必要 |

---

## 8. 設計フェーズへの持ち越しリサーチ

- `WM_CAPTURECHANGED` と `WM_CANCELMODE` の責務分離（重複終了の扱い）
- `NCHITTEST` キャッシュキーに「ウィンドウ位置世代」を入れるべきか
- `flush_window_pos_commands` の単一責務化（呼び出し点の一本化）
- ドラッグ中に許可するレイアウト変更の最小集合（完全凍結 vs 部分許可）
- 失敗時フォールバックの観測粒度（trace/debug/info の基準）
