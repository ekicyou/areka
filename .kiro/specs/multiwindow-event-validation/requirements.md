# Requirements Document

## Introduction

既存の `taffy_flex_demo` をマルチウィンドウ版に改修し、複数ウィンドウにおけるクリックイベント・ドラッグ（ウィンドウ移動）・ポインタ状態管理が正しく動作するかを検証する。問題が発見された場合、本仕様のスコープ内で修正を行う。

### 背景

現行の `multi_window_test.rs` はウィンドウ生成とグラフィックス初期化のみを検証しており、イベント処理（クリック、ドラッグ、ホバー/Leave）のマルチウィンドウ対応は未検証。コード調査により以下の潜在的問題が判明:

- `WM_MOUSELEAVE` 処理が全ウィンドウの全エンティティを対象にしており、ウィンドウスコープがない
- ドラッグ状態 (`DragState`) が `thread_local!` シングルトンで単一ドラッグのみ対応
- `SetCapture`/`ReleaseCapture` が未実装（ウィンドウ外へのドラッグ追従不可）
- イベントディスパッチがウィンドウ境界を明示的にスコーピングしていない

### スコープ

- **対象**: wintf クレートのイベント処理層（`ecs/window_proc/`, `ecs/pointer/`, `ecs/drag/`）
- **対象外**: OS ファイル DnD（`WM_DROPFILES` / OLE `IDropTarget`）は本仕様スコープ外
- **対象外**: ホイールイベントのhit_test非経由問題（G9）は既存のシングルウィンドウでも同様の制約であり、本仕様スコープ外

## Requirements

### Requirement 1: マルチウィンドウデモの作成

**Objective:** 開発者として、既存の `taffy_flex_demo` をマルチウィンドウ版に改修し、複数ウィンドウでのイベント動作を手動かつ視覚的に検証可能にしたい。

#### Acceptance Criteria

1. The `taffy_flex_demo` shall 複数の `Window` エンティティを生成し、各ウィンドウに独立したウィジェットツリーを構築する
2. When デモを起動した場合, the `taffy_flex_demo` shall 少なくとも2つの独立したウィンドウを表示する
3. The `taffy_flex_demo` shall 各ウィンドウに既存のウィジェット構成（RedBox色トグル、BlueBoxサイズトグル、GreenBoxダブルクリック、FlexContainerドラッグ移動、SeikatuImageαマスクヒットテスト）を完全に再現する
4. When 各ウィンドウの要素をクリックした場合, the `taffy_flex_demo` shall 各ウィンドウ独立でイベントハンドラが動作し、他ウィンドウに影響しないことを `tracing` ログで確認可能にする

### Requirement 2: WM_MOUSELEAVE のウィンドウスコープ修正

**Objective:** 開発者として、ウィンドウAからマウスが離脱したときに、ウィンドウBのポインタ状態が破壊されないようにしたい。これにより、マルチウィンドウでのホバー状態が正しく維持される。

#### Acceptance Criteria

1. When `WM_MOUSELEAVE` メッセージを受信した場合, the wintf shall 当該ウィンドウに属するエンティティの `PointerState` のみを削除し、他ウィンドウのエンティティの `PointerState` を保持する
2. When `WM_MOUSELEAVE` メッセージを受信した場合, the wintf shall 当該ウィンドウに属するエンティティにのみ `PointerLeave` マーカーを付与する
3. While ウィンドウAでホバー中にウィンドウBからマウスが離脱した場合, the wintf shall ウィンドウAのホバー状態を維持したままウィンドウBの `PointerState` のみをクリアする

### Requirement 3: ドラッグ状態のマルチウィンドウ安全性

**Objective:** 開発者として、複数ウィンドウが存在する環境でドラッグ操作が安全に動作することを保証したい。

#### Acceptance Criteria

1. While ウィンドウAでドラッグ操作中の場合, when ウィンドウBでクリックが発生した場合, the wintf shall ウィンドウAのドラッグ状態を破壊せず適切に処理する
2. When ウィンドウAでドラッグを開始した場合, the wintf shall ドラッグ操作がそのウィンドウのHWNDに対してのみ `SetWindowPos` を発行する
3. When ドラッグ中にマウスポインタが開始ウィンドウのクライアント領域外に出た場合, the wintf shall ドラッグ追従が継続するよう `SetCapture` を使用してマウスキャプチャを取得する
4. When ドラッグ操作が終了した場合, the wintf shall `ReleaseCapture` によりマウスキャプチャを解放する

### Requirement 4: ポインタイベントのウィンドウスコープ

**Objective:** 開発者として、イベントディスパッチがウィンドウ境界を越えて伝播しないことを保証したい。

#### Acceptance Criteria

1. When ポインタイベントが発生した場合, the wintf shall Tunnel/Bubbleイベント伝播パスが当該ウィンドウのエンティティツリー内に閉じていることを保証する
2. When ウィンドウAのエンティティでクリックが発生した場合, the wintf shall ウィンドウBのエンティティにイベントが配信されないことを保証する
3. The `dispatch_pointer_events` system shall PointerState を持つエンティティのイベント伝播パスがウィンドウルートエンティティを超えないことを保証する

### Requirement 5: マルチウィンドウ統合テスト

**Objective:** 開発者として、マルチウィンドウのイベント処理が回帰なく動作することを自動テストで検証したい。

#### 前提条件

- 全テストは `cargo test` で実行可能であること

#### Acceptance Criteria

1. The test suite shall 2つ以上のウィンドウを生成し、各ウィンドウで独立にヒットテストが正しく動作することを検証するテストを含む
2. The test suite shall `WM_MOUSELEAVE` が特定ウィンドウのエンティティのみに影響することを検証するテストを含む
3. The test suite shall マルチウィンドウ環境でのドラッグ状態の整合性を検証するテストを含む
