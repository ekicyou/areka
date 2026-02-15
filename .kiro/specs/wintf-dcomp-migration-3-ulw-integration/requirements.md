# 要件定義書: wintf-dcomp-migration-3-ulw-integration

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 3「UpdateLayeredWindow 統合」を担当する。Phase 2 で D2D1 合成パイプラインに切り替わった描画結果を、UpdateLayeredWindow（ULW）経由でウィンドウに転送し、alpha 透過とクリックスルーを実現する。

### コンテキスト

Phase 2 完了時点で、`world.rs` は D2D1 合成パイプライン（`compositor_init_system`, `composite_render_system`）で動作し、各ウィンドウの `WindowD3D11Compositor` が MemoryDC/HBITMAP へのピクセル転送を完了している。本 Phase では、この合成済みビットマップを `UpdateLayeredWindow` API で OS に転送し、WS_EX_LAYERED ウィンドウの alpha 透過表示とクリックスルーを実現する。

### 本子仕様のスコープ

- `ecs/graphics/compositor_systems.rs` 追加: `ulw_present_system` 関数
- `com/ulw.rs` 追加: `present_layered_window` 関数
- `ecs/world.rs` 変更: CommitComposition ステージに `ulw_present_system` を登録
- `ecs/window.rs` 変更: `WindowStyle::default()` の `ex_style` を `WS_EX_LAYERED` に変更
- `ecs/window_proc/handlers.rs` 変更: WM_PAINT / WM_ERASEBKGND / WM_SIZE ハンドラ更新
- `areka/src/main.rs` 変更: Shell / Balloon ウィンドウの `ex_style` 更新

### Non-Goals

- DComp コードの物理的削除（Phase 4 で実施）
- 新規 ECS コンポーネントの追加（Phase 1-2 で実装済みコンポーネントを使用）
- ウィジェット描画システムの変更

### 前提条件

- Phase 2 完了: `world.rs` が D2D1 合成パイプラインで動作
- `WindowD3D11Compositor` が各ウィンドウエンティティに初期化済み
- `composite_render_system` が毎フレーム HBITMAP への転送を完了
- DComp API 呼び出しがゼロ（Phase 2 で検証済み）

---

## Requirements

### Requirement 1: ulw_present_system の実装

**Objective:** 開発者として、合成済みビットマップを UpdateLayeredWindow で毎フレーム転送する ECS システムが欲しい。これにより DComp Commit に代わるウィンドウ表示メカニズムが確立される。

_Parent: Req 4.1, 4.4_

#### Acceptance Criteria

1. The `ulw_present_system` shall `WindowD3D11Compositor` の HBITMAP/MemoryDC を使用して `UpdateLayeredWindow(hwnd, hdcDst, &ptDst, &size, hdcSrc, &ptSrc, 0, &blend, ULW_ALPHA)` を呼び出す
2. The `ulw_present_system` shall `BLENDFUNCTION { BlendOp: AC_SRC_OVER, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA }` を使用する
3. The `ulw_present_system` shall `world.rs` の `CommitComposition` ステージに登録され、旧 `commit_composition` システムを完全に置換する
4. When `WindowD3D11Compositor` のダーティフラグが立っていない時, the `ulw_present_system` shall 当該ウィンドウの ULW 呼び出しをスキップする

### Requirement 2: present_layered_window 関数の実装

**Objective:** 開発者として、ULW 呼び出しを抽象化した COM ラッパー関数が欲しい。Win32 API の詳細を隠蔽し、安全な呼び出しインターフェースを提供する。

_Parent: Req 4.1_

#### Acceptance Criteria

1. The `present_layered_window` 関数 shall HWND, MemoryDC, ウィンドウサイズを引数に取り、`UpdateLayeredWindow` の Win32 API 呼び出しを実行する
2. The `present_layered_window` 関数 shall `ptDst` にウィンドウのスクリーン座標を使用し、`ptSrc` に `(0, 0)` を使用する
3. The `present_layered_window` 関数 shall `windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow` を使用する
4. If `UpdateLayeredWindow` が失敗した場合, the `present_layered_window` 関数 shall `windows::core::Result` でエラーを返す
5. The `present_layered_window` 関数 shall `com/ulw.rs` に配置される

### Requirement 3: WS_EX_LAYERED ウィンドウスタイル切替

**Objective:** 開発者として、全ウィンドウが WS_EX_LAYERED で作成されるようにしたい。ULW による描画には WS_EX_LAYERED が必須である。

_Parent: Req 4.2_

#### Acceptance Criteria

1. The `WindowStyle::default()` shall `ex_style` を `WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` に変更する
2. The `areka/src/main.rs` の Shell ウィンドウ設定 shall `ex_style` を `WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` に変更する
3. The `areka/src/main.rs` の Balloon ウィンドウ設定 shall `ex_style` を `WS_EX_NOREDIRECTIONBITMAP` から `WS_EX_LAYERED` に変更する
4. The wintf crate shall `WS_EX_TOOLWINDOW | WS_EX_TOPMOST` を維持する（既存動作の継続）

### Requirement 4: WM_PAINT / WM_ERASEBKGND ハンドラ更新

**Objective:** 開発者として、WS_EX_LAYERED 互換のメッセージハンドラが欲しい。ULW 方式では描画を UpdateLayeredWindow に委ねるため、WM_PAINT/WM_ERASEBKGND は安全な最小実装を維持する。

_Parent: Req 7.1, 7.3_

#### Acceptance Criteria

1. The WM_PAINT ハンドラ shall `BeginPaint` / `EndPaint` の最小ペアのみを実行し、実際の描画は行わない
2. The WM_ERASEBKGND ハンドラ shall `LRESULT(1)` を返し、背景消去をスキップする
3. While `WS_EX_LAYERED` が設定されている時, the wintf crate shall WM_PAINT による描画を行わず、ウィンドウ表示を UpdateLayeredWindow に委ねる

### Requirement 5: WM_SIZE ハンドラ更新

**Objective:** 開発者として、リサイズ時に合成ビットマップの再作成を確実にトリガーしたい。

_Parent: Req 7.2_

#### Acceptance Criteria

1. When WM_SIZE メッセージを受信した時, the `ecs/window_proc/handlers.rs` shall `WindowD3D11Compositor` のリサイズフラグをトリガーする（Phase 1 で実装済みの `resize()` メソッドを活用）
2. When WM_SIZE 処理後の次フレームで, the wintf crate shall 合成ビットマップが新サイズで再作成されていることを保証する

### Requirement 6: ULW 失敗時のエラーハンドリング

**Objective:** 開発者として、ULW 呼び出し失敗時に適切なリカバリが行われて欲しい。フレーム落ちは許容するがパニックは許容しない。

_Parent: Req 4.5_

#### Acceptance Criteria

1. If `UpdateLayeredWindow` が失敗した場合, the `ulw_present_system` shall `tracing::warn!` でエラーを記録し、当該フレームをスキップする
2. The `ulw_present_system` shall 失敗後の次フレームで自動的に ULW 呼び出しを再試行する
3. The wintf crate shall ULW 連続失敗時にパニックしない

### Requirement 7: alpha=0 クリックスルー動作

**Objective:** 開発者として、alpha=0 ピクセル領域でマウスクリックが背後のウィンドウに透過することを確認したい。ULW_ALPHA 方式の標準 OS 動作に依存する。

_Parent: Req 4.3_

#### Acceptance Criteria

1. When ULW_ALPHA で描画された alpha=0 ピクセル領域がクリックされた時, the OS shall 当該クリックを背後のウィンドウに透過する（OS 標準動作への依存）
2. The Phase 3 完了検証 shall alpha=0 クリックスルー動作を実機テストで確認する

### Requirement 8: Phase 3 検証基準

**Objective:** 開発者として、Phase 3 完了時の品質基準を明確にし、次 Phase への移行可否を判断したい。

_Parent: Req 10.1_

#### Acceptance Criteria

1. The Phase 3 完了検証 shall UpdateLayeredWindow での透過ウィンドウ表示が動作することを確認する
2. The Phase 3 完了検証 shall alpha=0 ピクセル領域のクリックスルーが動作することを確認する
3. The Phase 3 完了検証 shall WM_SIZE 時の合成ビットマップリサイズが正常動作することを確認する
4. The Phase 3 完了検証 shall ULW 失敗時のログ出力と次フレーム再試行が動作することを確認する
5. The Phase 3 完了検証 shall 全 example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）が ULW 方式で正常動作することを確認する
6. The Phase 3 完了検証 shall `cargo test` 全テストがパスすることを確認する

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
