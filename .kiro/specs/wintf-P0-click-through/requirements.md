# Requirements Document

## Project Description (Input)
wintfのWM_NCHITTESTハンドラにおけるクリックスルー（HTTRANSPARENT）の有効化。現在`nchittest_cache.rs`では`hit_test_in_window()`の結果を取得しているが、常に`HTCLIENT`を返しており`HTTRANSPARENT`は`#[allow(dead_code)]`で封印されている。ヒットテスト結果がNone（透明領域）の場合に`HTTRANSPARENT`を返すことで、BitmapSourceのαマスク等に基づくWindowsレベルのクリックスルーを実現する。「HTTRANSPARENT を返すとマウスイベントがブロックされてしまう」という既存コメントの問題を調査・解決し、ECSポインターイベントとの共存を確保する。

## Introduction

本仕様は、wintfウィンドウのWM\_NCHITTESTメッセージハンドラにおいて、ECSヒットテスト結果に基づくクリックスルー（HTTRANSPARENT）を有効化するための要件を定義する。透過ウィンドウの透明領域をマウスイベントが貫通し、背後のウィンドウやデスクトップに到達できるようにすることで、デスクトップマスコットとしての自然な操作感を実現する。

## Requirements

### 要件 1: ヒットテスト結果に基づくHTTRANSPARENT返却

**目的:** 開発者として、ヒットテスト結果がNone（透明領域）の場合にWM\_NCHITTESTからHTTRANSPARENTが返却される仕組みが欲しい。これにより、αマスクやHitTestMode::Noneによる透明領域でマウスイベントがウィンドウを貫通するようになる。

#### 受入基準

1. When `hit_test_in_window()` が `None` を返した場合, the wintf shall WM\_NCHITTESTの戻り値として `HTTRANSPARENT` (`-1`) を返す
   - Note: `hit_test_in_window()` は既に HitTestMode（None / Bounds / AlphaMask / NamedRegions）を考慮した結果を返すため、各モードの個別対応は不要
2. When `hit_test_in_window()` が `Some(entity)` を返した場合, the wintf shall WM\_NCHITTESTの戻り値として `HTCLIENT` (`1`) を返す
3. The wintf shall `nchittest_cache.rs` 内の `#[allow(dead_code)]` アノテーションを `HTTRANSPARENT` 定数から除去し、実際に使用する
4. When クライアント領域外の座標に対するWM\_NCHITTESTを受信した場合, the wintf shall 従来どおり `DefWindowProcW` に処理を委譲する（既存動作を維持）
5. The wintf shall HTTRANSPARENT / HTCLIENT いずれの結果もWM\_NCHITTESTキャッシュに同一方式で格納し、同一座標への再問い合わせ時にキャッシュから正しく返す

### 要件 2: ECSポインターイベントとの共存

**目的:** 開発者として、HTTRANSPARENT返却時にもECS内部のポインター状態管理が正しく動作することを保証したい。これにより、ホバー中のエンティティの `PointerState` が適切にクリーンアップされ、不正な状態が残らないようにする。

#### 受入基準

1. When HTTRANSPARENT領域にマウスカーソルが移動した場合, the wintf shall 直前にホバーしていたエンティティの `PointerState` コンポーネントを適切に除去する（PointerLeave 相当の処理）
2. When HTTRANSPARENT領域からHTCLIENT領域にマウスカーソルが再進入した場合, the wintf shall ヒットしたエンティティに対して `PointerState` を正しく付与する（PointerEnter 相当の処理）
3. If HTTRANSPARENT返却により `WM_MOUSELEAVE` がWindowsから発行された場合, the wintf shall 既存の `WM_MOUSELEAVE` ハンドラで全PointerStateを正常にクリーンアップする

### 要件 3: 既存コメント問題の解決

**目的:** 開発者として、「HTTRANSPARENT を返すとマウスイベントがブロックされてしまう」という既存コメントに記載された問題の原因を特定し、解決策を明確化したい。

#### 受入基準

1. When HTTRANSPARENT を有効化した場合, the wintf shall 既存コメント「HTTRANSPARENT を返すとマウスイベントがブロックされてしまう」の原因と解決策を設計ドキュメントに記載する
2. When HTTRANSPARENT領域を含むウィンドウにおいてマウス操作を行った場合, the wintf shall HTCLIENT領域のマウスイベント（WM\_MOUSEMOVE、WM\_LBUTTONDOWN 等）を正常に受信し続ける
3. If ECSポインターイベントシステムとHTTRANSPARENTの間に非互換性がある場合, the wintf shall 設計ドキュメントにて回避策と制約を明示する

### 要件 4: テスト検証可能性

**目的:** 開発者として、クリックスルー機能の動作を自動テストで検証できるようにしたい。また、手動テストで実際のクリックスルー挙動を確認できる環境が欲しい。

#### 受入基準

1. The wintf shall `cached_nchittest` 関数のHTTRANSPARENT返却パスについてユニットテストを持つ
2. The wintf shall HTCLIENT・HTTRANSPARENT両方の結果がキャッシュに正しく格納されることを検証するテストを持つ
3. The wintf shall `hit_test_in_window()` が `None` を返す条件（透明領域、HitTestMode::None、エンティティ不在）ごとにテストケースを持つ
4. The wintf shall exampleアプリケーション（`taffy_flex_demo`）において、`HitTestMode::None` を持つクリックスルー領域と通常のHTCLIENT領域を並べて配置し、開発者が手動でクリックスルー挙動を確認できるテストシーンを提供する
