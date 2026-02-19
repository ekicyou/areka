# Requirements Document

## Project Description (Input)
Phase 4 方針変更: DComp完全除去 → 切り替え式バックエンド実装。

透過ウィンドウのクリックスルーが必要な場合は ULW（UpdateLayeredWindow）パイプライン、通常のウィンドウUIには DComp パイプラインを使用する切り替え式アーキテクチャを実装する。

### 背景・動機
- Phase 1〜3 で DComp → ULW 移行が完了し、現在は ULW パイプラインのみがアクティブ
- DComp のシステム関数・コンポーネント・COM ラッパーはコードとして完全に残存（スケジュール登録のみ解除）
- GraphicsCore は現在も DComp デバイスを初期化している
- デスクトップマスコット（透過・クリックスルー必須）は ULW、通常ウィンドウ UI は DComp で描画する二刀流が最適
- Window エンティティ単位で合成モードを切り替え、同一アプリ内で ULW ウィンドウと DComp ウィンドウを共存させる

### 設計方針
- Window エンティティに合成モード（CompositionMode enum: ULW / DComp）を持たせ、ウィンドウ単位で描画パイプラインを切り替える
- 描画コマンド生成（GraphicsCommandList）は両パイプラインで共有
- ECS システムは CompositionMode に基づきクエリフィルタリングで分岐
- パイプライン固有のシステム・コンポーネントをモジュール分離し、保守性を確保する

## Requirements

### Requirement 1: CompositionMode によるウィンドウ単位のパイプライン選択

**Objective:** 開発者として、Window エンティティごとに描画パイプライン（ULW / DComp）を選択し、同一アプリ内で透過クリックスルーウィンドウと通常UIウィンドウを共存させたい。

#### Acceptance Criteria

1. The wintf crate shall `CompositionMode` enum を定義し、少なくとも `ULW` と `DComp` の2バリアントを含む
2. The wintf crate shall `CompositionMode` を Window エンティティのコンポーネントとして保持し、ウィンドウ生成時に指定可能とする
3. The wintf crate shall `CompositionMode` のデフォルト値を `ULW` とする（既存のデスクトップマスコット用途との後方互換性確保）
4. When `CompositionMode::ULW` が設定されている時, the wintf crate shall `WS_EX_LAYERED` スタイルを適用し、ULW パイプライン（D2D1合成 → DIBSection → UpdateLayeredWindow）で描画する
5. When `CompositionMode::DComp` が設定されている時, the wintf crate shall `WS_EX_NOREDIRECTIONBITMAP` スタイルを適用し、DComp パイプライン（IDCompositionTarget → IDCompositionVisual3 → IDCompositionSurface）で描画する

### Requirement 2: ECS システムの CompositionMode ベースクエリ分岐

**Objective:** 開発者として、ECS システムが CompositionMode に基づいてクエリフィルタリングで分岐し、各パイプラインのシステムが適切なエンティティのみを処理するようにしたい。

#### Acceptance Criteria

1. The wintf crate shall ULW 固有の ECS システム群（`compositor_init_system`, `composite_render_system`, `ulw_present_system`）が `CompositionMode::ULW` のウィンドウおよびその配下のエンティティのみを処理する
2. The wintf crate shall DComp 固有の ECS システム群（`init_window_graphics`, `visual_resource_management_system`, `visual_hierarchy_sync_system`, `visual_property_sync_system`, `render_surface`, `deferred_surface_creation_system`, `commit_composition`）が `CompositionMode::DComp` のウィンドウおよびその配下のエンティティのみを処理する
3. The wintf crate shall パイプライン共通の ECS システム群（描画コマンド生成: `draw_rectangles`, `draw_labels`, `draw_bitmap_sources` 等）を両モードで共有し、変更なく再利用する
4. While 同一アプリ内に ULW ウィンドウと DComp ウィンドウが共存している時, the wintf crate shall 各ウィンドウの `CompositionMode` に応じた正しいパイプラインで独立して描画する

### Requirement 3: DComp パイプラインのスケジュール再登録

**Objective:** 開発者として、Phase 2 で無効化された DComp パイプラインのシステム群をスケジュールに再登録し、DComp モードのウィンドウが描画可能な状態にしたい。

#### Acceptance Criteria

1. The wintf crate shall 以下の DComp システム群を ECS スケジュールの適切なステージに再登録する：
   - `GraphicsSetup`: `init_window_graphics`（IDCompositionTarget + DeviceContext 作成）
   - `PreRenderSurface` または `GraphicsSetup`: `visual_resource_management_system`（IDCompositionVisual3 作成）
   - `RenderSurface`: `render_surface`（各エンティティの Surface 描画）
   - `Composition`: `visual_hierarchy_sync_system`, `visual_property_sync_system`
   - `CommitComposition`: `commit_composition`（IDCompositionDevice3::Commit）
2. The wintf crate shall DComp システムの再登録時に、既存の ULW システムのスケジュール順序・依存関係を破壊しない
3. When DComp モードのウィンドウが存在しない時, the wintf crate shall DComp システム群のクエリが空結果となり、実質的にスキップされる（明示的な条件分岐を不要とする）
4. The wintf crate shall `deferred_surface_creation_system` を DComp モードウィンドウの子エンティティに対してのみ動作するよう再登録する

### Requirement 4: GraphicsCore の条件付き DComp デバイス管理

**Objective:** 開発者として、GraphicsCore が DComp デバイスを条件付きで初期化・保持し、ULW のみ使用時にも DComp デバイス作成コストが発生しない構成にしたい。

#### Acceptance Criteria

1. The GraphicsCore shall DComp デバイスフィールド（`desktop: IDCompositionDesktopDevice`, `dcomp: IDCompositionDevice3`）を `Option` 型で保持し、DComp モードのウィンドウが必要な場合にのみ初期化する（遅延初期化）
2. When `CompositionMode::DComp` を持つウィンドウエンティティが初めて生成された時, the GraphicsCore shall DComp デバイスを初期化し、以降は共有する
3. While DComp モードのウィンドウが存在しない時, the GraphicsCore shall DComp デバイスの初期化・保持を行わず、ULW パイプラインのみで動作する
4. The GraphicsCore shall DComp デバイスへのアクセサメソッド（`dcomp()`, `desktop()`）を `Option` 返却として維持し、DComp デバイス未初期化時に `None` を返す
5. If デバイスロストが発生した場合, the GraphicsCore shall 既存の invalidate() → 再初期化フローを維持し、DComp デバイスが初期化済みの場合はそれも含めて再初期化する

### Requirement 5: ウィンドウスタイルの CompositionMode 連動

**Objective:** 開発者として、ウィンドウ生成時に CompositionMode に応じた適切な拡張ウィンドウスタイルが自動適用されるようにしたい。

#### Acceptance Criteria

1. When `CompositionMode::ULW` が設定されたウィンドウを生成する時, the wintf crate shall `WS_EX_LAYERED` 拡張スタイルを適用する
2. When `CompositionMode::DComp` が設定されたウィンドウを生成する時, the wintf crate shall `WS_EX_NOREDIRECTIONBITMAP` 拡張スタイルを適用する
3. The wintf crate shall ウィンドウ生成システム（`create_windows`）が `CompositionMode` を参照してスタイルを自動決定し、`WindowStyle` コンポーネントとの整合性を保つ
4. The wintf crate shall ウィンドウメッセージハンドラ（WM_PAINT, WM_ERASEBKGND, WM_WINDOWPOSCHANGED）が `CompositionMode` に応じた適切な処理を行う

### Requirement 6: 二方式併存時の実行パフォーマンス最適化

**Objective:** 開発者として、ULW/DComp 二方式の併存に伴う実行負荷の上昇を最低限にとどめ、単一パイプライン使用時と同等のパフォーマンスを維持したい。

#### Acceptance Criteria

1. While アプリ内の全ウィンドウが同一の CompositionMode を使用している時, the wintf crate shall 他方のパイプラインに関連するシステム群が空クエリで即座にスキップされ、計測可能な性能劣化を発生させない
2. The wintf crate shall パイプライン固有システムのクエリフィルタリングに ECS のネイティブクエリ機構（`With<T>`/ `Without<T>` フィルタ、またはコンポーネント値マッチ）を使用し、ランタイムの条件分岐オーバーヘッドを最小化する
3. The wintf crate shall 描画コマンド生成（GraphicsCommandList）を両パイプラインで共有し、パイプライン分岐に起因する描画処理の重複を排除する
4. The wintf crate shall DComp デバイスの遅延初期化により、ULW のみ使用するアプリが DComp COM オブジェクトの初期化・メモリコストを負担しない
5. The wintf crate shall パイプライン共通リソース（D3D11Device, D2D1Factory, D2D1Device, DirectWriteFactory）の初期化を1回のみとし、両パイプラインで共有する
6. The wintf crate shall スケジュール内のシステム実行順序を最適化し、パイプライン共通ステージ（Layout, Draw）と固有ステージ（Composition, CommitComposition）の間で不要な同期ポイントを設けない

### Requirement 7: DComp パイプラインの動作検証

**Objective:** 開発者として、再登録された DComp パイプラインが Phase 2 無効化前と同等に動作することを検証したい。

#### Acceptance Criteria

1. When `CompositionMode::DComp` のウィンドウを生成した時, the wintf crate shall IDCompositionTarget、IDCompositionVisual3、IDCompositionSurface が正しく作成され、DComp Visual 階層が構築される
2. When DComp モードのウィンドウ内でウィジェット（Rectangle, Label, BitmapSource）が描画された時, the wintf crate shall GraphicsCommandList を DComp Surface に正しく描画する
3. The wintf crate shall `dcomp_demo.rs` を DComp バックエンド検証用リファレンスとして維持し、DComp パイプラインの基本動作確認に活用する
4. The wintf crate shall `taffy_flex_demo` 相当の描画が DComp モードで正しく動作することを確認可能とする
5. The wintf crate shall DComp パイプライン再登録後の動作中に COM 操作エラーやリソース作成失敗が発生した場合、構造化ログ（tracing）でエラー/警告を出力し、原因調査を支援する

### Requirement 8: ULW/DComp ウィンドウの共存動作

**Objective:** 開発者として、同一アプリ内で ULW ウィンドウと DComp ウィンドウが同時に存在し、それぞれが正しく描画・操作されることを保証したい。

#### Acceptance Criteria

1. The wintf crate shall 1つの ECS World 内に `CompositionMode::ULW` と `CompositionMode::DComp` のウィンドウエンティティが混在可能とする
2. When ULW ウィンドウと DComp ウィンドウが同時に表示されている時, the wintf crate shall 各ウィンドウが独立して正しい描画パイプラインで描画される
3. While ULW ウィンドウが透過クリックスルーを実行している時, the wintf crate shall DComp ウィンドウの描画・インタラクションに影響を与えない
4. The wintf crate shall 既存のヒットテスト・ポインタイベント・ドラッグシステムが両 CompositionMode のウィンドウで正しく動作する

### Requirement 9: テスト・検証戦略

**Objective:** 開発者として、Phase 4 完了時に切り替え式バックエンドが正しく動作することを包括的に検証したい。

#### Acceptance Criteria

1. The wintf crate shall `cargo test` 全テストがパスすること
2. The wintf crate shall ULW モード単独での既存 example（`taffy_flex_demo`, `typewriter_demo` 等）が正しく動作すること（後方互換性）
3. The wintf crate shall DComp モード単独での描画検証が可能な example またはテストを提供すること
4. The wintf crate shall ULW + DComp 混在ウィンドウの同時動作を検証可能な example またはテストを提供すること
5. When `CompositionMode::ULW` のウィンドウが表示されている時, the wintf crate shall alpha=0 ピクセルのクリックスルーが正しく動作すること
6. When `CompositionMode::DComp` のウィンドウが表示されている時, the wintf crate shall 通常のマウスインタラクション（クリック、ホバー、ドラッグ）が正しく動作すること
