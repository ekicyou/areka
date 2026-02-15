# 要件定義書: wintf-dcomp-migration-1-d2d1-composition

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 1「D2D1合成スタック構築」を担当する。現行の DComp パイプライン（`IDCompositionDevice3::Commit` による合成）を温存したまま、新しい D2D1 合成描画スタックを独立モジュールとして構築し、既存のウィジェット描画システムが出力する `GraphicsCommandList`（`ID2D1CommandList`）を per-window 合成ビットマップに統合描画する能力を確立する。

### コンテキスト

現行パイプラインでは各ウィジェットの描画結果は `IDCompositionSurface` を介して DComp Visual ツリーに送られ、`IDCompositionDevice3::Commit()` でハードウェア合成される。本仕様では、DComp 合成に代わる D2D1 ソフトウェア合成パスを新規に構築し、`ID2D1Bitmap1` 上に全ウィジェットを1枚の合成結果として描画する。この合成結果は後続 Phase で `UpdateLayeredWindow` に渡されるが、Phase 1 では合成完了と HBITMAP 転送までをスコープとする。

### 本子仕様のスコープ

- `ecs/graphics/compositor.rs` 新規作成: WindowD3D11Compositor コンポーネント
- `ecs/graphics/compositor_systems.rs` 新規作成: compositor_init_system, composite_render_system
- `com/ulw.rs` 新規作成（部分）: transfer_to_hbitmap ユーティリティ
- `ecs/layout/arrangement.rs` 拡張: GlobalArrangement に `global_opacity` フィールド追加
- `ecs/layout/systems.rs` 拡張: `propagate_global_arrangements` に Opacity 累積ロジック追加

### Non-Goals

- `world.rs` への新システム登録（Phase 2 で実施）
- DComp パイプラインの変更・無効化（Phase 2 で実施）
- `UpdateLayeredWindow` 呼び出し（Phase 3 で実施）
- `WS_EX_LAYERED` ウィンドウスタイル変更（Phase 3 で実施）
- 旧 DComp コード削除（Phase 4 で実施）
- `GraphicsCore` からの DComp 初期化除去（Phase 2 以降）

---

## Requirements

### Requirement 1: WindowD3D11Compositor コンポーネント

**Objective:** 開発者として、ウィンドウごとの D2D1 合成描画リソース群を統合管理する ECS コンポーネントが欲しい。これにより DComp Visual ツリーに依存しない per-window 合成描画基盤が得られる。

_Parent: Req 3.1, 6.1_

#### Acceptance Criteria

1. The `WindowD3D11Compositor` shall 以下の4リソースを統合管理する:
   - 合成描画先ビットマップ（`ID2D1Bitmap1`, `D2D1_BITMAP_OPTIONS_TARGET` フラグ付き）
   - CPU ステージングビットマップ（`ID2D1Bitmap1`, `D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW` フラグ付き）
   - HBITMAP（`CreateDIBSection` による PBGRA32 形式 top-down DIB）
   - MemoryDC（`CreateCompatibleDC` によるメモリデバイスコンテキスト）

2. The `WindowD3D11Compositor` shall リソースライフサイクル管理 API として以下を提供する:
   - 全リソースの一括作成（コンストラクタ）
   - 新サイズでの全リソース再作成（resize）
   - 全リソースの無効化（invalidate、リソース解放は Drop に委譲）
   - リソース有効性の判定（is_valid）
   - リソース世代の追跡（generation カウンタ）

3. The `WindowD3D11Compositor` shall 管理する全4リソースが常に同一サイズ・同一ピクセルフォーマット（PBGRA32）を維持することを保証する

4. The `WindowD3D11Compositor` shall bevy_ecs の SparseSet ストレージ戦略を使用する（ウィンドウエンティティ数が少ないため）

5. The `WindowD3D11Compositor` shall `ecs/graphics/compositor.rs` に配置される

### Requirement 2: 合成描画システム（composite_render_system）

**Objective:** 開発者として、全エンティティの `GraphicsCommandList` を z-order + transform + opacity で per-window 合成ビットマップに描画するシステムが欲しい。これにより DComp Visual ツリーの階層合成を D2D1 ソフトウェア合成で代替できる。

_Parent: Req 3.1, 3.2, 3.3, 3.4_

#### Acceptance Criteria

1. The `composite_render_system` shall ウィンドウに属する全エンティティを `Children` 関係の depth-first pre-order で走査し、z-order に従って合成ビットマップに描画する

2. The `composite_render_system` shall 各エンティティの `GlobalArrangement.transform` をデバイスコンテキストに適用した上で、対応する `GraphicsCommandList` を合成ビットマップに描画する

3. When `Visual.is_visible` が `false` のエンティティを処理する時, the `composite_render_system` shall そのエンティティとその children の描画を完全にスキップする

4. When `GlobalArrangement.global_opacity` が 1.0 未満のエンティティを処理する時, the `composite_render_system` shall D2D1 レイヤー機構（`PushLayer` または同等 API）で opacity を適用してから描画する

5. The `composite_render_system` shall 合成描画完了後、合成ビットマップからステージングビットマップへピクセルデータをコピーする

6. The `composite_render_system` shall ウィンドウ内のいずれかのエンティティでコンポーネント変更（`GraphicsCommandList`、`GlobalArrangement`、`Visual` のいずれか）が検出された場合のみ、ウィンドウ全体の再合成を実行する

7. The `composite_render_system` shall 既存のウィジェット描画システム群（`draw_rectangles`, `draw_labels`, `draw_typewriters`, `draw_bitmap_sources`）を一切変更せず、それらが出力した `GraphicsCommandList` を合成入力として消費する

8. The `composite_render_system` shall `ecs/graphics/compositor_systems.rs` に配置される

### Requirement 3: 合成リソース初期化システム（compositor_init_system）

**Objective:** 開発者として、HWND を持つウィンドウエンティティに自動的に `WindowD3D11Compositor` を作成・アタッチするシステムが欲しい。これにより合成リソースのライフサイクルが ECS フレームワーク内で自動管理される。

_Parent: Req 3.1, 6.1_

#### Acceptance Criteria

1. When `WindowHandle` が新たにアタッチされたエンティティが検出された時, the `compositor_init_system` shall そのエンティティに `WindowD3D11Compositor` を作成・挿入する

2. The `compositor_init_system` shall `GraphicsCore` リソースから `ID2D1DeviceContext` を取得し、合成リソースの作成に使用する

3. When ウィンドウサイズが前フレームから変更されたことを検出した時, the `compositor_init_system` shall `WindowD3D11Compositor` のリサイズ処理を呼び出す

4. When `GraphicsCore` の generation カウンタと `WindowD3D11Compositor` の generation カウンタが不一致の時, the `compositor_init_system` shall `WindowD3D11Compositor` を再作成する

5. The `compositor_init_system` shall `ecs/graphics/compositor_systems.rs` に配置される

### Requirement 4: GlobalArrangement Opacity 累積

**Objective:** 開発者として、DComp `Visual.SetOpacity()` の代替として、親→子の Opacity 階層累積を `GlobalArrangement` で自動処理したい。これにより合成描画システムが各エンティティの最終的な透明度を直接参照できる。

_Parent: Req 3.6_

#### Acceptance Criteria

1. The `GlobalArrangement` shall `global_opacity: f32` フィールドを持ち、初期値は `1.0`（完全不透明）とする

2. The `propagate_global_arrangements` shall 各エンティティの `global_opacity` を `parent.global_opacity * child.opacity` として計算する

3. The `propagate_global_arrangements` shall 計算後の `global_opacity` を `[0.0, 1.0]` 範囲にクランプする

4. The Opacity 累積 shall 既存の `propagate_global_arrangements` システム内の transform 伝播と同一パスで実行される（追加の走査コストを発生させない）

5. The `global_opacity` フィールド追加 shall 既存の `GlobalArrangement` を参照するテスト・システムに回帰を起こさない

### Requirement 5: D2D → HBITMAP 転送ユーティリティ

**Objective:** 開発者として、D2D1 ステージングビットマップから GDI HBITMAP への高速ピクセル転送関数が欲しい。これにより Phase 3 での `UpdateLayeredWindow` 呼び出しの前提条件が整う。

_Parent: Req 3.1（合成パイプラインの一部として）_

#### Acceptance Criteria

1. The `transfer_to_hbitmap()` shall ステージング `ID2D1Bitmap1` のピクセルデータをマップし、DIBSection メモリへコピーし、アンマップする

2. When D2D1 Map の pitch と DIBSection の stride（`width * 4`）が異なる時, the `transfer_to_hbitmap()` shall 行単位のコピーを行う

3. When pitch と stride が一致する時, the `transfer_to_hbitmap()` shall 単一の連続メモリコピーで転送を最適化する

4. The `transfer_to_hbitmap()` shall `com/ulw.rs` モジュールに配置され、ECS 非依存の純粋ユーティリティ関数として実装する

5. If Map 操作が失敗した場合, the `transfer_to_hbitmap()` shall `windows::core::Result` でエラーを返却する

### Requirement 6: リサイズ対応

**Objective:** 開発者として、ウィンドウサイズ変更時に合成ビットマップ群が適切に再作成され、次フレームから正しい描画が行われることを保証したい。

_Parent: Req 3.5_

#### Acceptance Criteria

1. When ウィンドウサイズが変更された時, the `WindowD3D11Compositor` shall 全4リソース（合成ビットマップ、ステージングビットマップ、HBITMAP、MemoryDC）を新サイズで再作成する

2. The リサイズ処理完了後, the `composite_render_system` shall 次フレームで新サイズに基づく正しい合成描画を実行する

3. If リサイズ時のリソース作成が失敗した場合, the `WindowD3D11Compositor` shall リソースを無効化（invalidate）し、`tracing::error` でエラー詳細をログ出力する

4. The リサイズ処理 shall 0×0 サイズのウィンドウに対してリソース作成を試行しない

### Requirement 7: デバイスロスト対応

**Objective:** 開発者として、既存の `GraphicsCore` デバイスロストフローと整合性のある `WindowD3D11Compositor` の自動再初期化が欲しい。

_Parent: Req 5.4（間接）, Req 10.1_

#### Acceptance Criteria

1. When `GraphicsCore` が invalidate された時, the `WindowD3D11Compositor` shall 自身も invalidate される

2. The `compositor_init_system` shall `GraphicsCore` と `WindowD3D11Compositor` の generation カウンタの不一致を検出し、`WindowD3D11Compositor` を新しいデバイスリソースで再作成する

3. The デバイスロスト後の復旧 shall 既存の `HasGraphicsResources.set_changed()` トリガーメカニズムと整合する

4. The デバイスロスト復旧 shall ユーザー操作なしで自動的に完了し、次の正常フレームで合成描画が再開される

### Requirement 8: Phase 1 検証基準

**Objective:** 開発者として、Phase 1 の完了を客観的に判定できる検証基準が欲しい。

_Parent: Req 10.1, 10.2_

#### Acceptance Criteria

1. The `WindowD3D11Compositor::new()` shall 全4リソースを正しく作成できること（unit test で検証）

2. The `composite_render_system` shall 複数の `GraphicsCommandList` を z-order + transform + opacity で正しく合成描画できること（integration test で検証）

3. The `global_opacity` 累積 shall 多段階層で正確に動作すること（unit test: parent 0.8 × child 0.5 = 0.4）

4. The `transfer_to_hbitmap()` shall pitch/stride 不一致パターンを含む転送を正しく実行できること（unit test で検証）

5. The 新モジュール群 shall `cargo test` で全テストがパスし、既存テストへの回帰がないこと

6. The 新パイプライン shall DComp パイプラインと共存状態で `cargo build` が成功すること

---

## 要件カバレッジサマリー

| 子仕様要件 | 親要件 | 概要 |
|-----------|--------|------|
| Req 1 | 3.1, 6.1 | WindowD3D11Compositor コンポーネント定義 |
| Req 2 | 3.1-3.4 | composite_render_system（合成描画） |
| Req 3 | 3.1, 6.1 | compositor_init_system（リソース初期化） |
| Req 4 | 3.6 | GlobalArrangement Opacity 累積 |
| Req 5 | 3.1 | D2D → HBITMAP 転送ユーティリティ |
| Req 6 | 3.5 | リサイズ対応 |
| Req 7 | 5.4, 10.1 | デバイスロスト対応 |
| Req 8 | 10.1, 10.2 | Phase 1 検証基準 |
