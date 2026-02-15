# 要件定義書: wintf-dcomp-migration-1-d2d1-composition

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 1「D2D1合成スタック構築」を担当する。DComp パイプラインを温存したまま、新しい D2D1 合成描画スタックを独立モジュールとして構築し、既存の GraphicsCommandList を合成描画する能力を確立する。

### 本子仕様のスコープ

- `ecs/graphics/compositor.rs` 新規作成: WindowD3D11Compositor コンポーネント
- `ecs/graphics/compositor_systems.rs` 新規作成: compositor_init_system, composite_render_system
- `com/ulw.rs` 新規作成（部分）: transfer_to_hbitmap ユーティリティ
- `ecs/layout/` 拡張: GlobalArrangement に global_opacity 追加
- `ecs/layout/systems.rs` 拡張: Opacity 累積ロジック

### Non-Goals

- world.rs への新システム登録（Phase 2 で実施）
- DComp パイプラインの変更・無効化（Phase 2 で実施）
- UpdateLayeredWindow 呼び出し（Phase 3 で実施）
- WS_EX_LAYERED ウィンドウスタイル変更（Phase 3 で実施）
- 旧コード削除（Phase 4 で実施）

---

## Requirements

### Requirement 1: WindowD3D11Compositor コンポーネント

**Objective:** 開発者として、ウィンドウごとの合成描画リソース（合成ビットマップ + ステージング + HBITMAP）を統合管理するコンポーネントが欲しい。

_Parent: Req 3.1, 6.1_

#### Acceptance Criteria

1. The `WindowD3D11Compositor` shall 以下の4リソースを統合管理する:
   - 合成描画先ビットマップ（`ID2D1Bitmap1`, `D2D1_BITMAP_OPTIONS_TARGET`）
   - CPUステージングビットマップ（`ID2D1Bitmap1`, `D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW`）
   - HBITMAP（`CreateDIBSection`, PBGRA32形式, top-down DIB）
   - MemoryDC（`CreateCompatibleDC`）

2. The `WindowD3D11Compositor` shall `Option<WindowD3D11CompositorInner>` パターンでリソースライフサイクルを管理する:
   - `new()`: 全リソース作成
   - `resize()`: 全リソース再作成
   - `invalidate()`: `inner = None`（リソース解放はDrop）
   - `is_valid()`: リソース有効性判定
   - `generation: u32`: リソース世代管理

3. The 全リソース shall 同一サイズ・同一ピクセルフォーマット（PBGRA32）を維持する

4. The `WindowD3D11Compositor` shall SparseSet ストレージ戦略を使用する（ウィンドウ数が少ない前提）

### Requirement 2: 合成描画システム（composite_render_system）

**Objective:** 開発者として、全エンティティの GraphicsCommandList を z-order ソートで per-window 合成ビットマップに描画するシステムが欲しい。

_Parent: Req 3.1, 3.2, 3.3, 3.4_

#### Acceptance Criteria

1. The `composite_render_system` shall ウィンドウに属する全エンティティを `Children` 関係の depth-first pre-order で走査し、z-order に従って合成描画する

2. The `composite_render_system` shall 各エンティティの `GlobalArrangement.transform` で `SetTransform` し、`GraphicsCommandList` を `DrawImage` する

3. The `composite_render_system` shall `GlobalArrangement.global_opacity` が 0.0 のエンティティ（`Visual.is_visible == false` を含む）を描画スキップする

4. The `composite_render_system` shall `global_opacity < 1.0` の場合に `PushLayer`（または同等のD2D API）で opacity を適用する

5. The `composite_render_system` shall 合成描画完了後、`CopyFromBitmap` で composition_bitmap → staging_bitmap にコピーする

6. The `composite_render_system` shall ウィンドウ内のいずれかのエンティティで `Changed<GraphicsCommandList>` || `Changed<GlobalArrangement>` || `Changed<Visual>` の場合のみウィンドウ全体を再合成する（ダーティ判定）

7. The `composite_render_system` shall 既存のウィジェット描画システム群（draw_rectangles, draw_labels, draw_bitmap_sources）を一切変更せずに、その出力である `GraphicsCommandList` を消費する

### Requirement 3: 合成リソース初期化システム（compositor_init_system）

**Objective:** 開発者として、HWND 付きウィンドウエンティティに自動的に WindowD3D11Compositor を作成・アタッチするシステムが欲しい。

_Parent: Req 3.1, 6.1_

#### Acceptance Criteria

1. The `compositor_init_system` shall `Added<WindowHandle>` かつ `Without<WindowD3D11Compositor>` のエンティティに WindowD3D11Compositor を作成・挿入する

2. The `compositor_init_system` shall `GraphicsCore` から `ID2D1DeviceContext` を取得してリソース作成に使用する

3. The `compositor_init_system` shall リサイズ検出時（WindowHandle のサイズ変更）に `WindowD3D11Compositor::resize()` を呼び出す

4. The `compositor_init_system` shall デバイスロスト時（generation 不一致）に WindowD3D11Compositor を再作成する

### Requirement 4: GlobalArrangement Opacity 累積

**Objective:** 開発者として、DComp Visual.SetOpacity() の代替として、親→子の Opacity 階層累積を GlobalArrangement で自動処理したい。

_Parent: Req 3.6_

#### Acceptance Criteria

1. The `GlobalArrangement` shall `global_opacity: f32` フィールドを持つ（初期値 `1.0`）

2. The `propagate_global_arrangements` shall `global_opacity = parent.global_opacity * child.opacity` を計算する

3. When `Visual.is_visible == false` の場合, the `global_opacity` shall `0.0` に設定される

4. The `global_opacity` shall `[0.0, 1.0]` 範囲にクランプされる

5. The Opacity 累積 shall 既存の `propagate_global_arrangements` システム内の transform 累積と同一のフレームで実行される

### Requirement 5: D2D → HBITMAP 転送ユーティリティ

**Objective:** 開発者として、D2D ステージングビットマップから HBITMAP への転送関数が欲しい（Phase 3 の ULW 呼び出しの前提）。

_Parent: Req 3.1（合成パイプラインの一部として）_

#### Acceptance Criteria

1. The `transfer_to_hbitmap()` shall ステージング ID2D1Bitmap1 を `Map()` し、DIBSection メモリへコピーし、`Unmap()` する

2. The `transfer_to_hbitmap()` shall D2D Map() の pitch と DIBSection の stride（`width * 4`）が異なる場合に行単位コピーを行う

3. The `transfer_to_hbitmap()` shall pitch==stride の場合は単一 memcpy で最適化可能とする

4. The `transfer_to_hbitmap()` shall `com/ulw.rs` モジュールに配置され、ECS 非依存の純粋ユーティリティ関数として実装する

### Requirement 6: リサイズ対応

**Objective:** 開発者として、ウィンドウサイズ変更時に合成ビットマップ群が適切に再作成されることを保証したい。

_Parent: Req 3.5_

#### Acceptance Criteria

1. When ウィンドウサイズが変更された時, the `WindowD3D11Compositor` shall `resize()` によって全4リソース（composition_bitmap, staging_bitmap, HBITMAP, MemoryDC）を新サイズで再作成する

2. The リサイズ処理 shall 次フレームで正しい描画を保証する

3. If リサイズ時のリソース作成が失敗した場合, the `WindowD3D11Compositor` shall 旧サイズを維持し、`tracing::error` でログ出力する

### Requirement 7: デバイスロスト対応

**Objective:** 開発者として、既存の GraphicsCore デバイスロストフローと整合する WindowD3D11Compositor の再初期化が欲しい。

_Parent: Req 5.4（間接）, Parent: Req 10.1_

#### Acceptance Criteria

1. When `GraphicsCore::invalidate()` が呼ばれた時, the `WindowD3D11Compositor` shall 自動的に invalidate される

2. The `compositor_init_system` shall `generation` カウンタの不一致を検出して WindowD3D11Compositor を再作成する

3. The デバイスロスト対応 shall 既存の `HasGraphicsResources.set_changed()` トリガーメカニズムに準拠する

### Requirement 8: Phase 1 検証基準

**Objective:** 開発者として、Phase 1 の完了を客観的に判定できる検証基準が欲しい。

_Parent: Req 10.1, 10.2_

#### Acceptance Criteria

1. The `WindowD3D11Compositor::new()` shall 全4リソースを正しく作成できること（unit test）

2. The `composite_render_system` shall GraphicsCommandList を z-order + transform + opacity で合成描画できること（integration test）

3. The `global_opacity` 累積 shall 正確に動作すること（unit test: parent 0.8 × child 0.5 = 0.4）

4. The 新パイプライン shall `taffy_flex_demo` 相当の描画結果を独立テスト環境で再現できること（E2E test）

5. The 全テスト shall `cargo test` でパスすること（既存テストへの回帰なし）

---

## 要件カバレッジサマリー

| 子仕様要件 | 親要件 | 概要 |
|-----------|--------|------|
| Req 1 | 3.1, 6.1 | WindowD3D11Compositor コンポーネント |
| Req 2 | 3.1-3.4 | composite_render_system |
| Req 3 | 3.1, 6.1 | compositor_init_system |
| Req 4 | 3.6 | GlobalArrangement Opacity 累積 |
| Req 5 | 3.1 | D2D → HBITMAP 転送ユーティリティ |
| Req 6 | 3.5 | リサイズ対応 |
| Req 7 | 5.4, 10.1 | デバイスロスト対応 |
| Req 8 | 10.1, 10.2 | Phase 1 検証基準 |
