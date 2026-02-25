# Requirements Document

## Introduction

ウィンドウドラッグでスクリーン間を移動したときに挙動が不安定になる問題を解決する。主要な原因は以下の2つ：

1. **マウスキャプチャ未実装**: DPI変更でウィンドウが縮小した際、マウスカーソルがウィンドウ外に出ると `WM_MOUSEMOVE` が届かなくなりドラッグが途切れる（例: 200% DPI → 125% DPI 移動時）
2. **ステール offset の巻き戻し**: ドラッグ中に間接的なレイアウト再計算が走ると、古い `Arrangement.offset` が ECS パイプラインで復活し、ウィンドウ位置がジャンプ/振動する

本仕様は、スクリーン境界をまたぐドラッグ操作における安定性を確保し、ユーザーが意図した位置にスムーズにウィンドウを移動できるようにすることを目的とする。

## Project Description (Input)
ウィンドウドラッグでスクリーン間を移動したときに挙動が不安定になる事が多い。DPIが変わらない２スクリーン環境でも発生する。スクリーン境界におけるドラッグの安定性を実現せよ。

## Requirements

### Requirement 1: マウスキャプチャによるドラッグ継続性の保証

**Objective:** 開発者として、ドラッグ中にマウスカーソルがウィンドウ領域外に出てもマウスイベントを確実に受信したい。これにより、スクリーン境界付近でのドラッグ操作が途切れない。

#### Acceptance Criteria
1. While ドラッグ中, the wintf shall マウスキャプチャを取得してウィンドウ外でも `WM_MOUSEMOVE` / `WM_LBUTTONUP` イベントを受信する
2. When ドラッグが終了した, the wintf shall マウスキャプチャを解放する
3. If マウスキャプチャの取得が失敗した, the wintf shall エラーをログに記録し、キャプチャなしでドラッグ処理を続行する
4. If ドラッグ中に外部要因でキャプチャを失った(`WM_CAPTURECHANGED`), the wintf shall ドラッグ操作を安全に終了し `DragEndEvent` を発行する
5. If ドラッグ中にアプリケーションがパニックした, the wintf shall マウスキャプチャが確実に解放される（RAII ガード）

### Requirement 2: スクリーン境界をまたぐドラッグ時のウィンドウ位置安定性

**Objective:** ユーザーとして、ウィンドウをドラッグしてスクリーン境界をまたぐとき、ウィンドウがジャンプしたり振動したりせず滑らかに追従してほしい。

#### Acceptance Criteria
1. While ドラッグ中, when ウィンドウがスクリーン境界を横断した, the wintf shall ウィンドウ位置がマウスカーソルの移動量に一致して連続的に変化する
2. While ドラッグ中, the wintf shall `WM_WINDOWPOSCHANGED` のエコーバイパス処理がドラッグ操作によるSetWindowPosを正しくself-initiated判定する
3. While ドラッグ中, the wintf shall `SetWindowPos` が1フレームあたり高々1回実行される（複数回呼び出しによるちらつきを防止する）
4. While ドラッグ中, the wintf shall ECSレイアウトパイプラインによる位置更新がドラッグ操作を妨げない（`WindowDragging` マーカーで排他制御）
5. While ドラッグ中, when `WM_DPICHANGED` でウィンドウが縮小しマウスがウィンドウ外に出た, the wintf shall マウスキャプチャにより `WM_MOUSEMOVE` を受信し続けドラッグを継続する（Note: R1で保証）

### Requirement 3: 同一DPI環境におけるドラッグ安定性

**Objective:** 開発者として、DPI差のないマルチモニター環境でもドラッグが不安定になる原因を特定・解消したい。DPI起因ではない不安定要素を排除する。

#### Acceptance Criteria
1. While ドラッグ中 and 移動先スクリーンの DPI が同一, the wintf shall `WM_DPICHANGED` を受信しないこと、またはDPIが実質未変更であれば位置補正処理をスキップする
2. While ドラッグ中, the wintf shall `SetWindowPosGuard` による `SELF_INITIATED_DEPTH` カウンタがドラッグ由来のSetWindowPosで正しくインクリメント・デクリメントされる
3. While ドラッグ中, the wintf shall `Changed<WindowPos>` がドラッグ操作のエコーで発火しないようバイパスする
4. While ドラッグ中, the wintf shall VSYNCタイミングでtickが発生してもドラッグ位置が巻き戻らない

### Requirement 4: ドラッグ中のグラフィックスリソース安定性

**Objective:** ユーザーとして、スクリーン間ドラッグ中にウィンドウ描画が乱れたり消えたりしないでほしい。

#### Acceptance Criteria
1. While ドラッグ中, when スクリーン境界を横断した, the wintf shall DirectComposition のコミット処理がドラッグ移動と適切に同期する
2. While ドラッグ中, the wintf shall `WindowD3D11Compositor` のdirtyフラグが不要な再描画を引き起こさない
3. If スクリーン間移動に伴いグラフィックスリソースのリサイズが必要になった, the wintf shall ドラッグ終了後にリサイズを遅延実行する（ドラッグ中の描画途切れを防ぐ）
4. While ドラッグ中, the wintf shall `HasGraphicsResources` の `set_changed()` がドラッグ操作単体では発火しない

### Requirement 5: ドラッグ終了時の状態整合性

**Objective:** 開発者として、ドラッグ操作完了後にECSの各コンポーネント（WindowPos、DPI、Arrangement）が整合した状態であることを保証したい。

#### Acceptance Criteria
1. When ドラッグが終了した, the wintf shall `WindowPos` コンポーネントがウィンドウの実際の位置と一致する
2. When ドラッグが終了した, the wintf shall `DPI` コンポーネントがウィンドウの現在のモニターの DPI を反映する
3. When ドラッグが終了した, the wintf shall ECSレイアウトパイプラインが次フレームで正常に実行され、Arrangement が正しく計算される
4. When ドラッグが終了した and マウスキャプチャが取得されていた, the wintf shall `ReleaseCapture` が確実に呼び出される
5. If ドラッグ中にアプリケーションがクラッシュまたはパニックした, the wintf shall マウスキャプチャが解放される（drop 保証）
