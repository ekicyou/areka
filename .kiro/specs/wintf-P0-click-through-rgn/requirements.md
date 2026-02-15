# Requirements Document

## Project Description (Input)
SetWindowRgn ベースのクリックスルー（クロスプロセス対応）実装 - **実験的仕様**

wintf フレームワークにおいて、HitTest::None エンティティのクリックスルーをクロスプロセスで実現する。
従来の WS_EX_TRANSPARENT + WM_NCHITTEST (HTTRANSPARENT) アプローチは同一スレッド内のウィンドウ間でしか機能しないことが判明したため、SetWindowRgn を使用してウィンドウリージョンからクリックスルー領域を除外する方式に切り替える。

### 背景
- WS_EX_TRANSPARENT: "siblings beneath the window (that were created by the same thread)" のみ対象
- HTTRANSPARENT: DWM Step 2 で同一スレッド内の兄弟ウィンドウのみ転送
- SetWindowRgn: DWM Step 1 でリージョン外をスキップ → クロスプロセスで貫通可能

### 技術要件
- DirectComposition (WS_EX_NOREDIRECTIONBITMAP) との互換性検証が必要（互換性が確認できない場合は DirectComposition 利用を破棄する判断もあり得る）
- ビットマップベースのリージョン構築により、ヒットテスト負荷を最小化
- エンティティが自身の不透明領域をビットマップにプッシュする方式
- レイアウト変更時にリージョンを動的に更新
- ドラッグ中のリージョン一時拡張（ドラッグ操作の継続性保証）
- **本仕様は実験的性質を持ち、パフォーマンス検証結果によってアプローチの根本的変更もあり得る**

## Requirements

### Requirement 1: リージョン定期更新メカニズム
**Objective:** 開発者として、クリックスルー領域がウィンドウ操作に正しく反映されるよう、SetWindowRgn によるリージョン更新を定期的に実施したい。これにより、クロスプロセスでのクリックスルーが常に最新のレイアウト状態を反映する。

#### Acceptance Criteria
1. The wintf shall update the window region every 0.25 seconds
2. When region update timer elapses, the wintf shall invoke the region construction process
3. The wintf shall execute region updates independently from the main rendering loop
4. The wintf shall maintain region update timing accuracy within ±50ms

### Requirement 2: ビットマップベースのリージョン構築
**Objective:** 開発者として、ウィンドウサイズの1bitビットマップを基にウィンドウリージョンを構築したい。これにより、高頻度なヒットテストクエリを回避し、更新負荷を最小化できる。

#### Acceptance Criteria
1. When window region update is triggered, the wintf shall allocate a 1-bit bitmap matching the physical pixel dimensions of the window
2. The wintf shall initialize the bitmap with all pixels set to transparent (0: click-through)
3. When bitmap is ready, the wintf shall invoke entity rendering pass to populate opaque regions
4. When bitmap population is complete, the wintf shall convert the bitmap to HRGN representation
5. The wintf shall apply the constructed HRGN to the target window using SetWindowRgn
6. The wintf shall release bitmap resources after HRGN construction

### Requirement 3: エンティティによる不透明領域書き込み
**Objective:** 開発者として、各エンティティが自身の不透明領域を1bitビットマップに書き込めるようにしたい。これにより、HitTest::None 以外のエンティティのみがクリック可能領域として登録される。

#### Acceptance Criteria
1. When entity rendering pass is invoked, the wintf shall iterate all entities with HitTest component
2. When an entity has HitTest::Opaque or HitTest::Client, the wintf shall write the entity's physical bounds as opaque (1) to the bitmap
3. When an entity has HitTest::None, the wintf shall skip writing to the bitmap (leaving the region transparent)
4. The wintf shall clip entity bounds to the window dimensions before writing to bitmap
5. The wintf shall write entity regions to bitmap in parent-to-child hierarchy order to respect z-ordering

### Requirement 4: ビットマップ解像度の構成可能性
**Objective:** 開発者として、リージョン構築時のビットマップ解像度単位を調整可能な定数として宣言したい。これにより、メモリ使用量とクリック精度のトレードオフを調整できる。

#### Acceptance Criteria
1. The wintf shall declare the bitmap resolution scale factor as a named constant (default: 4x4 physical pixels per bitmap pixel)
2. The wintf shall use the resolution scale factor consistently throughout bitmap allocation and entity bounds conversion
3. The wintf shall allow resolution scale modification via a single constant definition

### Requirement 5: レイアウト変更検知と動的更新
**Objective:** 開発者として、ECS レイアウトシステムの変更を検知してリージョンを即座に更新したい。これにより、エンティティの移動・サイズ変更時にクリックスルー領域が正確に反映される。

#### Acceptance Criteria
1. When layout system completes entity arrangement updates, the wintf shall mark the window region as dirty
2. When region is marked as dirty and region update timer elapses, the wintf shall trigger immediate region reconstruction
3. If layout changes occur within the 0.25-second update interval, the wintf shall defer region update to the next scheduled update cycle

### Requirement 6: ドラッグ操作中のリージョン一時拡張
**Objective:** 開発者として、ウィンドウドラッグ操作中はリージョンをウィンドウ全体に拡張したい。これにより、ドラッグ開始エンティティから意図せずマウスが外れた場合でも、ドラッグ操作が継続できる。

#### Acceptance Criteria
1. When window drag operation starts, the wintf shall expand the window region to cover the entire window bounds
2. While window drag operation is in progress, the wintf shall maintain the expanded region regardless of timer-based updates
3. When window drag operation completes, the wintf shall restore region to the hit-test-based construction on the next update cycle
4. When drag operation is cancelled, the wintf shall restore region to the hit-test-based construction on the next update cycle

### Requirement 7: DirectComposition互換性検証
**Objective:** 開発者として、SetWindowRgn がウィンドウスタイル WS_EX_NOREDIRECTIONBITMAP と併用可能であることを検証したい。これにより、DirectComposition による描画とクリックスルー機能が同時に動作するかを確認し、不可能な場合は DirectComposition 利用破棄の判断材料とする。

#### Acceptance Criteria
1. The wintf shall attempt SetWindowRgn on windows with WS_EX_NOREDIRECTIONBITMAP style
2. When SetWindowRgn is applied, the wintf shall verify that DirectComposition visual rendering continues to function correctly
3. If SetWindowRgn fails with WS_EX_NOREDIRECTIONBITMAP, the wintf shall log an error with HRESULT code and detailed compatibility diagnostics
4. If visual rendering is broken after SetWindowRgn, the wintf shall log a critical incompatibility warning

### Requirement 8: クロスプロセスクリックスルー
**Objective:** エンドユーザーとして、HitTest::None エンティティの領域をクリックした際、他のプロセスのウィンドウにクリックイベントが貫通することを期待する。これにより、デスクトップマスコットの透過領域を通してデスクトップアイコンや他のアプリケーションを操作できる。

#### Acceptance Criteria
1. When user clicks on a HitTest::None entity area, the wintf shall allow the click event to pass through to windows from other processes
2. When user clicks on a non-HitTest::None entity area, the wintf shall capture the click event and prevent pass-through
3. The wintf shall achieve cross-process click-through without requiring WS_EX_TRANSPARENT or HTTRANSPARENT

### Requirement 9: パフォーマンス測定と最適化指針
**Objective:** 開発者として、リージョン更新処理のパフォーマンスを測定し、実用性を判断したい。これにより、本アプローチの継続可否を決定できる。

#### Acceptance Criteria
1. The wintf shall measure and log the time required for each region update cycle (bitmap allocation, entity rendering, HRGN conversion)
2. When region update time exceeds 16ms (60 FPS threshold), the wintf shall log a performance warning
3. The wintf shall provide configuration to disable region updates for performance comparison
4. If sustained region update overhead degrades application responsiveness, the wintf shall document the incompatibility for architectural decision

### Requirement 10: モジュール化とリジェクション容易性
**Objective:** 開発者として、実験的実装が失敗した場合に容易に機能を削除できるようにしたい。これにより、パフォーマンスや互換性の問題が判明した際、既存コードへの影響を最小限に抑えながら機能を無効化できる。

#### Acceptance Criteria
1. The wintf shall provide a standalone function that takes ECS World and returns HRGN (e.g., `build_window_region_from_world(world: &World) -> Result<HRGN>`)
2. The wintf shall invoke the region construction function only from the 0.25-second timer callback
3. The wintf shall allow disabling region updates by removing the timer callback invocation without modifying the region construction function
4. When region updates are disabled, the wintf shall not interfere with existing window behavior
5. The wintf shall encapsulate all SetWindowRgn-related logic within a clearly defined module boundary
