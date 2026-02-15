# Requirements Document

## Project Description (Input)
SetWindowRgn ベースのクリックスルー（クロスプロセス対応）実装 - **実験的仕様**

wintf フレームワークにおいて、HitTestMode::None エンティティのクリックスルーをクロスプロセスで実現する。
従来の WS_EX_TRANSPARENT + WM_NCHITTEST (HTTRANSPARENT) アプローチは同一スレッド内のウィンドウ間でしか機能しないことが判明したため、SetWindowRgn を使用してウィンドウリージョンからクリックスルー領域を除外する方式に切り替える。

### 背景
- WS_EX_TRANSPARENT: "siblings beneath the window (that were created by the same thread)" のみ対象
- HTTRANSPARENT: DWM Step 2 で同一スレッド内の兄弟ウィンドウのみ転送
- SetWindowRgn: DWM Step 1 でリージョン外をスキップ → クロスプロセスで貫通可能

### 技術要件
- DirectComposition (WS_EX_NOREDIRECTIONBITMAP) との互換性検証が必要（互換性が確認できない場合は DirectComposition 利用を破棄する判断もあり得る）
- **矩形ベースのリージョン構築**により、実装コストとヒットテスト負荷を最小化（実験的アプローチとしてシンプルさを優先）
- エンティティの bounds（`GlobalArrangement.bounds`）を直接 HRGN に合成する方式
- レイアウト変更時にリージョンを動的に更新
- ドラッグ中のリージョン一時拡張（ドラッグ操作の継続性保証）
- **継続可能性**: 将来的に HitTestMode::AlphaMask のピクセル単位クリックスルーが必要になった場合は、ビットマップ中間表現方式への拡張を許容
- **本仕様は実験的性質を持ち、パフォーマンス検証結果によってアプローチの根本的変更もあり得る**

## Requirements

### Requirement 1: リージョン定期更新メカニズム
**Objective:** 開発者として、クリックスルー領域がウィンドウ操作に正しく反映されるよう、SetWindowRgn によるリージョン更新を定期的に実施したい。これにより、クロスプロセスでのクリックスルーが常に最新のレイアウト状態を反映する。

#### Acceptance Criteria
1. The wintf shall update the window region every 0.25 seconds
2. When region update timer elapses, the wintf shall invoke the region construction process
3. The wintf shall execute region updates independently from the main rendering loop
4. The wintf shall maintain region update timing accuracy within ±50ms

### Requirement 2: 矩形ベースのリージョン構築
**Objective:** 開発者として、エンティティの矩形 bounds を直接 HRGN に合成してウィンドウリージョンを構築したい。これにより、実装をシンプルに保ちながらクリックスルー機能を実現できる。

#### Acceptance Criteria
1. When window region update is triggered, the wintf shall query all entities with GlobalArrangement and HitTest components (note: entities without HitTest component default to HitTestMode::Bounds)
2. When an entity has HitTestMode other than None (i.e., Bounds, AlphaMask, or NamedRegions), the wintf shall collect the entity's physical bounds (GlobalArrangement.bounds)
3. When an entity has HitTestMode::None, the wintf shall skip the entity (leaving the region click-through)
4. For each collected bounds, the wintf shall snap the rectangle to the configured grid size and create a rectangular region using CreateRectRgn
5. The wintf shall combine all rectangular regions into a single HRGN using CombineRgn(RGN_OR)
6. The wintf shall apply the combined HRGN to the target window using SetWindowRgn
7. The wintf shall delete temporary HRGN objects after combination to prevent resource leaks

### Requirement 3: グリッドスナップの構成可能性
**Objective:** 開発者として、リージョン構築時のグリッドサイズを調整可能な定数として宣言したい。これにより、HRGN の複雑度とクリック精度のトレードオフを調整できる。

#### Acceptance Criteria
1. The wintf shall declare the grid snap size as a named constant (default: 4x4 physical pixels)
2. The wintf shall snap entity bounds to the grid before creating rectangular regions
3. The wintf shall allow grid size modification via a single constant definition

### Requirement 4: レイアウト変更検知と動的更新
**Objective:** 開発者として、ECS レイアウトシステムの変更を検知してリージョンを即座に更新したい。これにより、エンティティの移動・サイズ変更時にクリックスルー領域が正確に反映される。

#### Acceptance Criteria
1. When layout system completes entity arrangement updates, the wintf shall mark the window region as dirty
2. When region is marked as dirty and region update timer elapses, the wintf shall trigger immediate region reconstruction
3. If layout changes occur within the 0.25-second update interval, the wintf shall defer region update to the next scheduled update cycle

### Requirement 5: ドラッグ操作中のリージョン一時拡張
**Objective:** 開発者として、ウィンドウドラッグ操作中はリージョンをウィンドウ全体に拡張したい。これにより、ドラッグ開始エンティティから意図せずマウスが外れた場合でも、ドラッグ操作が継続できる。

#### Acceptance Criteria
1. When window drag operation starts, the wintf shall expand the window region to cover the entire window bounds
2. While window drag operation is in progress, the wintf shall maintain the expanded region regardless of timer-based updates
3. When window drag operation completes, the wintf shall restore region to the hit-test-based construction on the next update cycle
4. When drag operation is cancelled, the wintf shall restore region to the hit-test-based construction on the next update cycle

### Requirement 6: DirectComposition互換性検証
**Objective:** 開発者として、SetWindowRgn がウィンドウスタイル WS_EX_NOREDIRECTIONBITMAP と併用可能であることを検証したい。これにより、DirectComposition による描画とクリックスルー機能が同時に動作するかを確認し、不可能な場合は DirectComposition 利用破棄の判断材料とする。

#### Acceptance Criteria
1. The wintf shall attempt SetWindowRgn on windows with WS_EX_NOREDIRECTIONBITMAP style
2. When SetWindowRgn is applied, the wintf shall verify that DirectComposition visual rendering continues to function correctly
3. If SetWindowRgn fails with WS_EX_NOREDIRECTIONBITMAP, the wintf shall log an error with HRESULT code and detailed compatibility diagnostics
4. If visual rendering is broken after SetWindowRgn, the wintf shall log a critical incompatibility warning

### Requirement 7: クロスプロセスクリックスルー
**Objective:** エンドユーザーとして、HitTestMode::None エンティティの領域をクリックした際、他のプロセスのウィンドウにクリックイベントが貫通することを期待する。これにより、デスクトップマスコットの透過領域を通してデスクトップアイコンや他のアプリケーションを操作できる。

#### Acceptance Criteria
1. When user clicks on a HitTestMode::None entity area, the wintf shall allow the click event to pass through to windows from other processes
2. When user clicks on a non-HitTestMode::None entity area, the wintf shall capture the click event and prevent pass-through
3. The wintf shall achieve cross-process click-through without requiring WS_EX_TRANSPARENT or HTTRANSPARENT

### Requirement 8: パフォーマンス測定と最適化指針
**Objective:** 開発者として、リージョン更新処理のパフォーマンスを測定し、実用性を判断したい。これにより、本アプローチの継続可否を決定できる。

#### Acceptance Criteria
1. The wintf shall measure and log the time required for each region update cycle (entity query, bounds snapping, HRGN creation and combination)
2. When region update time exceeds 16ms (60 FPS threshold), the wintf shall log a performance warning
3. The wintf shall provide configuration to disable region updates for performance comparison
4. If sustained region update overhead degrades application responsiveness, the wintf shall document the incompatibility for architectural decision

### Requirement 9: モジュール化とリジェクション容易性
**Objective:** 開発者として、実験的実装が失敗した場合に容易に機能を削除できるようにしたい。これにより、パフォーマンスや互換性の問題が判明した際、既存コードへの影響を最小限に抑えながら機能を無効化できる。

#### Acceptance Criteria
1. The wintf shall provide a standalone function that takes ECS World and returns HRGN (e.g., `build_window_region_from_world(world: &World) -> Result<HRGN>`)
2. The wintf shall invoke the region construction function only from the 0.25-second timer callback
3. The wintf shall allow disabling region updates by removing the timer callback invocation without modifying the region construction function
4. When region updates are disabled, the wintf shall not interfere with existing window behavior
5. The wintf shall encapsulate all SetWindowRgn-related logic within a clearly defined module boundary
