# Requirements Document

## Project Description (Input)
SetWindowRgn ベースのクリックスルー（クロスプロセス対応）実装。

wintf フレームワークにおいて、HitTest::None エンティティのクリックスルーをクロスプロセスで実現する。
従来の WS_EX_TRANSPARENT + WM_NCHITTEST (HTTRANSPARENT) アプローチは同一スレッド内のウィンドウ間でしか機能しないことが判明したため、SetWindowRgn を使用してウィンドウリージョンからクリックスルー領域を除外する方式に切り替える。

### 背景
- WS_EX_TRANSPARENT: "siblings beneath the window (that were created by the same thread)" のみ対象
- HTTRANSPARENT: DWM Step 2 で同一スレッド内の兄弟ウィンドウのみ転送
- SetWindowRgn: DWM Step 1 でリージョン外をスキップ → クロスプロセスで貫通可能

### 技術要件
- DirectComposition (WS_EX_NOREDIRECTIONBITMAP) との互換性が必要
- HitTest::None エンティティの bounds をリージョンから除外
- レイアウト変更時にリージョンを動的に更新
- ドラッグ中のリージョン一時拡張（ドラッグ操作の継続性保証）

## Requirements

### Requirement 1: リージョン定期更新メカニズム
**Objective:** 開発者として、クリックスルー領域がウィンドウ操作に正しく反映されるよう、SetWindowRgn によるリージョン更新を定期的に実施したい。これにより、クロスプロセスでのクリックスルーが常に最新のレイアウト状態を反映する。

#### Acceptance Criteria
1. The wintf shall update the window region every 0.25 seconds
2. When region update timer elapses, the wintf shall invoke the region construction process
3. The wintf shall execute region updates independently from the main rendering loop
4. The wintf shall maintain region update timing accuracy within ±50ms

### Requirement 2: ヒットテスト統合リージョン構築
**Objective:** 開発者として、ECS ワールドのヒットテスト結果を基にウィンドウリージョンを構築したい。これにより、HitTest::None エンティティの bounds がクリックスルー領域として正しく除外される。

#### Acceptance Criteria
1. When constructing window region, the wintf shall perform hit-test queries against the ECS world in 4x4 pixel grid units (physical pixels)
2. When a 4x4 pixel region contains at least one entity with HitTest::None, the wintf shall exclude that region from the window region
3. When a 4x4 pixel region contains no HitTest::None entities, the wintf shall include that region in the window region
4. The wintf shall construct the final HRGN as a union of 4x4 pixel rectangles
5. The wintf shall apply the constructed HRGN to the target window using SetWindowRgn

### Requirement 3: リージョン解像度設定の構成可能性
**Objective:** 開発者として、リージョン構築時のグリッド解像度（4x4ピクセル）を調整可能な定数として宣言したい。これにより、将来的なパフォーマンス調整やクリック精度の最適化が容易になる。

#### Acceptance Criteria
1. The wintf shall declare the region grid size (4x4 pixels) as a named constant
2. The wintf shall use the grid size constant consistently throughout all region construction logic
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

### Requirement 6: DirectComposition互換性
**Objective:** 開発者として、SetWindowRgn がウィンドウスタイル WS_EX_NOREDIRECTIONBITMAP と併用可能であることを確認したい。これにより、DirectComposition による描画とクリックスルー機能が同時に動作する。

#### Acceptance Criteria
1. The wintf shall support SetWindowRgn on windows with WS_EX_NOREDIRECTIONBITMAP style
2. When SetWindowRgn is applied, the wintf shall not interfere with DirectComposition visual rendering
3. If SetWindowRgn fails with WS_EX_NOREDIRECTIONBITMAP, the wintf shall log an error with HRESULT code

### Requirement 7: クロスプロセスクリックスルー
**Objective:** エンドユーザーとして、HitTest::None エンティティの領域をクリックした際、他のプロセスのウィンドウにクリックイベントが貫通することを期待する。これにより、デスクトップマスコットの透過領域を通してデスクトップアイコンや他のアプリケーションを操作できる。

#### Acceptance Criteria
1. When user clicks on a HitTest::None entity area, the wintf shall allow the click event to pass through to windows from other processes
2. When user clicks on a non-HitTest::None entity area, the wintf shall capture the click event and prevent pass-through
3. The wintf shall achieve cross-process click-through without requiring WS_EX_TRANSPARENT or HTTRANSPARENT
