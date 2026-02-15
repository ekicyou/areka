# 要件定義: wintf-dcomp-migration-3-ulw-integration

## 概要

Phase 3 — UpdateLayeredWindow 統合。Phase 2 で D2D1 合成パイプラインに切り替わった描画結果を、UpdateLayeredWindow（ULW）経由でウィンドウに転送し、alpha 透過とクリックスルーを実現する。

---

## 要件一覧

### Requirement 1: ulw_present_system の実装

**Objective:** 開発者として、合成済みビットマップを UpdateLayeredWindow で毎フレーム転送する ECS システムが欲しい。

_Parent: Req 4.1, 4.4_

#### Acceptance Criteria

1. The `ulw_present_system` shall `WindowD3D11Compositor` の HBITMAP/MemoryDC を使用して `UpdateLayeredWindow(hwnd, hdcDst, &ptDst, &size, hdcSrc, &ptSrc, 0, &blend, ULW_ALPHA)` を呼び出す
2. The `ulw_present_system` shall `BLENDFUNCTION { BlendOp: AC_SRC_OVER, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA }` を使用する
3. The `ulw_present_system` shall world.rs の `CommitComposition` ステージに登録され、旧 `commit_composition` システムを完全に置換する
4. The `ulw_present_system` shall ダーティフラグが立っていないウィンドウの ULW 呼び出しをスキップする（無駄な転送の回避）

### Requirement 2: present_layered_window 関数の実装

**Objective:** 開発者として、ULW 呼び出しを抽象化した COM ラッパー関数が欲しい。

_Parent: Req 4.1_

#### Acceptance Criteria

1. The `com/ulw.rs` の `present_layered_window` 関数 shall HWND, MemoryDC, ウィンドウサイズを引数に取り、`UpdateLayeredWindow` の Win32 API 呼び出しを実行する
2. The 関数 shall `ptDst` にウィンドウのスクリーン座標を使用し、`ptSrc` に `(0, 0)` を使用する
3. The 関数 shall `windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow` を使用する

### Requirement 3: WS_EX_LAYERED ウィンドウスタイル切替

**Objective:** 開発者として、全ウィンドウが WS_EX_LAYERED で作成されるようにしたい。

_Parent: Req 4.2_

#### Acceptance Criteria

1. The `ecs/window.rs` の `WindowStyle::default()` shall `ex_style` を `WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` に変更する
2. The `areka/src/main.rs` shall Shell ウィンドウの `ex_style` から `WS_EX_NOREDIRECTIONBITMAP` を除去し `WS_EX_LAYERED` を設定する
3. The `areka/src/main.rs` shall Balloon ウィンドウの `ex_style` から `WS_EX_NOREDIRECTIONBITMAP` を除去し `WS_EX_LAYERED` を設定する
4. The wintf crate shall `WS_EX_TOOLWINDOW | WS_EX_TOPMOST` は維持する（既存動作の継続）

### Requirement 4: WM_PAINT / WM_ERASEBKGND ハンドラ更新

**Objective:** 開発者として、WS_EX_LAYERED 互換のメッセージハンドラが欲しい。

_Parent: Req 7.1, 7.3_

#### Acceptance Criteria

1. The `ecs/window_proc/handlers.rs` shall WM_PAINT ハンドラで `BeginPaint` / `EndPaint` の最小ペアのみを実行し、実際の描画は行わない（WS_EX_LAYERED では WM_PAINT が発火しない可能性があるが、安全のため最小実装を維持）
2. The `ecs/window_proc/handlers.rs` shall WM_ERASEBKGND ハンドラで `1` を返し、背景消去をスキップする
3. The wintf crate shall WS_EX_LAYERED での WM_PAINT 発火動作を Phase 3 実装前に検証する（research.md § Research Needed 参照）。検証結果に基づきハンドラの最終設計を確定する

### Requirement 5: WM_SIZE ハンドラ更新

**Objective:** 開発者として、リサイズ時に合成ビットマップの再作成を確実にトリガーしたい。

_Parent: Req 7.2_

#### Acceptance Criteria

1. The `ecs/window_proc/handlers.rs` shall WM_SIZE メッセージ受信時に `WindowD3D11Compositor` のリサイズフラグをトリガーする（Phase 1 で実装済みの `resize()` メソッドを活用）
2. The wintf crate shall WM_SIZE 後の次フレームで合成ビットマップが新サイズで再作成されていることを検証する

### Requirement 6: ULW 失敗時のエラーハンドリング

**Objective:** 開発者として、ULW 呼び出し失敗時に適切なリカバリが行われて欲しい。

_Parent: Req 4.5_

#### Acceptance Criteria

1. If `UpdateLayeredWindow` が失敗した場合, the `ulw_present_system` shall `tracing::warn!` でエラーを記録し、当該フレームをスキップする
2. The `ulw_present_system` shall 失敗後の次フレームで自動的に ULW 呼び出しを再試行する
3. The wintf crate shall ULW 連続失敗時にパニックしない

### Requirement 7: alpha=0 クリックスルー検証

**Objective:** 開発者として、alpha=0 ピクセル領域でマウスクリックが背後のウィンドウに透過することを確認したい。

_Parent: Req 4.3_

#### Acceptance Criteria

1. When ULW_ALPHA で描画した alpha=0 ピクセル領域をクリックした場合, the OS shall 当該クリックを背後のウィンドウに透過する（OS 標準動作の検証）
2. The Phase 3 完了検証 shall alpha=0 クリックスルー動作を実機で確認する

### Requirement 8: Phase 3 検証基準

**Objective:** 開発者として、Phase 3 完了時の品質基準を明確にしたい。

_Parent: Req 10.1_

#### Acceptance Criteria

1. The Phase 3 完了検証 shall UpdateLayeredWindow での透過ウィンドウ表示が動作することを確認する
2. The Phase 3 完了検証 shall alpha=0 ピクセル領域のクリックスルーが動作することを確認する
3. The Phase 3 完了検証 shall WM_SIZE 時のリサイズが正常動作することを確認する
4. The Phase 3 完了検証 shall ULW 失敗時のログ出力 + 次フレーム再試行が動作することを確認する
5. The Phase 3 完了検証 shall 全 example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）が ULW 方式で正常動作することを確認する
6. The Phase 3 完了検証 shall `cargo test` 全テストパスを確認する

---

## 要件トレーサビリティ（親仕様 → 子仕様）

| 親要件 | 子仕様要件 |
|--------|-----------|
| Req 4.1 (ULW呼び出し) | Req 1, Req 2 |
| Req 4.2 (WS_EX_LAYERED) | Req 3 |
| Req 4.3 (クリックスルー) | Req 7 |
| Req 4.4 (commit→ULW置換) | Req 1 |
| Req 4.5 (ULW失敗リトライ) | Req 6 |
| Req 7.1 (WM_PAINT更新) | Req 4 |
| Req 7.2 (WM_SIZE) | Req 5 |
| Req 7.3 (BeginPaint最小ペア) | Req 4 |
| Req 10.1 (検証基準) | Req 8 |
