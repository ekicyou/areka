# Research & Design Decisions

## Summary
- **Feature**: `multiwindow-event-validation`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - PointerState のグローバルクエリが3箇所に存在し、マルチウィンドウで状態破壊を引き起こす
  - `find_owner_window` 相当のロジックが `drag/dispatch.rs` にアドホック実装されており、共通化が必要
  - `build_bubble_path` は LayoutRoot まで無制限に伝播するが、現時点では実害は限定的
  - DragState シングルトンは OS 制約と整合するが、HWND 検証の欠如により誤終了リスクがある

## Research Log

### ChildOf 逆走査によるウィンドウ所有権特定

- **Context**: G1, G2, G5, G7 の修正に共通して「エンティティの所属ウィンドウ」を特定する機能が必要
- **Sources Consulted**: `drag/dispatch.rs` L86-126 のアドホック実装、`window.rs` の `on_window_add` フック
- **Findings**:
  - 既存パターン: `ChildOf` を辿り `Window` コンポーネントを持つ祖先で停止
  - `SetWindowParentToLayoutRoot` により Window エンティティは必ず LayoutRoot の直下
  - ウィジェットは `ChildOf` チェーンで Window の子孫。最大深度は一般的に 3-5 階層
  - Entity ID はフレーム内で安定しており、キャッシュ不要（WndProc コールバック内で都度逆引きで十分）
- **Implications**: `ChildOf` 逆走査が最もシンプルかつ既存パターンと整合。キャッシュコンポーネント (`OwnerWindow(Entity)`) は不要

### WM_MOUSELEAVE / WM_MOUSEMOVE のスコーピング修正パターン

- **Context**: G1, G2 — グローバルクエリを Window スコープに制限する必要がある
- **Sources Consulted**: `handlers.rs` L671-686, L732-740, L829-849
- **Findings**:
  - 3箇所とも同一パターン: `world.query::<(Entity, &PointerState)>()` → 全エンティティ走査
  - `hwnd` は関数引数として既に利用可能、`get_entity_from_hwnd(hwnd)` で window_entity を取得済み
  - フィルタ方法: 各エンティティに対して `find_owner_window(world, e)` を呼び、`window_entity` と一致するものだけを対象にする
  - パフォーマンス: PointerState を持つエンティティ数は通常 0-2 個（マウス直下のみ）。逆走査コストは無視可能
- **Implications**: `find_owner_window` フィルタの追加で修正完了。クエリ構造の根本的変更は不要

### build_bubble_path のウィンドウ境界停止条件

- **Context**: G5 — イベント伝播が LayoutRoot まで到達する問題
- **Sources Consulted**: `dispatch.rs` L113-121, `window.rs` L1065-1092
- **Findings**:
  - 現状: `ChildOf` を持たないエンティティ（LayoutRoot）で停止
  - Window → LayoutRoot の `ChildOf` リンクが存在するため、パスに LayoutRoot が含まれる
  - 実害: LayoutRoot にイベントハンドラを登録しない限り問題なし（現在のデモでは登録なし）
  - Window コンポーネント検出で停止させることで、Tunnel は Window → target、Bubble は target → Window となる
  - Window エンティティ自身はパスに**含める**（Window レベルのハンドラが動作するため）
- **Implications**: `Window` コンポーネント検出で停止。明示的フラグの導入は不要

### DragState の HWND 検証方式

- **Context**: G3 — マルチウィンドウでの誤ドラッグ終了防止
- **Sources Consulted**: `state.rs` L252-270, `handlers.rs` L1030-1060
- **Findings**:
  - `end_dragging()` は状態遷移のみで HWND を検証しない
  - 呼び出し元 `handle_button_message` が HWND を保持しているが、DragState との照合なし
  - `DragState::Dragging` には既に `hwnd: HWND` フィールドが存在
  - 修正方法: `handle_button_message` 内で DragState を読み取り、`Dragging.hwnd` が現在の `hwnd` と一致する場合のみ `end_dragging()` を呼ぶ
  - DragState 構造体自体の変更は不要（既に hwnd フィールドあり）
- **Implications**: 呼び出し側のガード条件追加のみ。DragState の HashMap 化は不要

### デモ改修パターン

- **Context**: Req1 — 既存 `taffy_flex_demo.rs` のマルチウィンドウ化
- **Sources Consulted**: `taffy_flex_demo.rs` L112-141, L168-388
- **Findings**:
  - `create_flexbox_window(world)` がウィンドウ＋ウィジェットツリーを一括構築
  - パラメータ化（ウィンドウタイトル、位置）で複数回呼び出し可能
  - イベントハンドラ関数（`on_red_box_pressed` 等）は `sender` / `entity` 引数で動作し、グローバル状態への依存なし → 複数ウィンドウで安全に共有可能
  - マーカーコンポーネント（`RedBox`, `BlueBox` 等）は複数エンティティに付与可能
- **Implications**: `create_flexbox_window` をパラメータ化して複数回呼び出す設計が最小変更

## Design Decisions

### Decision: `find_owner_window` 実装方式

- **Context**: 複数のギャップ修正（G1, G2, G5, G7, G8）で共通して必要な基盤ユーティリティ
- **Alternatives Considered**:
  1. ChildOf 逆走査（都度計算） — 既存 `drag/dispatch.rs` のパターンと同一
  2. キャッシュコンポーネント `OwnerWindow(Entity)` — 各エンティティに所属ウィンドウを事前計算
- **Selected Approach**: Option 1 — ChildOf 逆走査
- **Rationale**: PointerState を持つエンティティ数は通常 0-2 個、ツリー深度は 3-5 階層でコスト無視可能。キャッシュは階層変更時の同期コストが追加される
- **Trade-offs**: ✅ シンプル、既存パターン踏襲、同期コストなし / ❌ 大規模ツリーでは非効率（現実的に問題にならない）
- **Follow-up**: パフォーマンス問題が顕在化した場合にキャッシュ方式に移行可能

### Decision: `build_bubble_path` 停止条件

- **Context**: G5 — イベント伝播パスが Window 境界を越えて LayoutRoot まで到達する
- **Alternatives Considered**:
  1. `Window` コンポーネント検出で停止
  2. 明示的な `EventBoundary` マーカーコンポーネント導入
- **Selected Approach**: Option 1 — Window コンポーネント検出
- **Rationale**: Window が自然なイベント境界。専用マーカーは概念の重複
- **Trade-offs**: ✅ 追加コンポーネント不要 / ❌ Window 以外の境界が将来必要になる可能性（その時点で拡張可能）

### Decision: PointerState のウィンドウ情報

- **Context**: G6 — PointerState がどのウィンドウ由来かを保持していない
- **Alternatives Considered**:
  1. `PointerState` に `source_window: Entity` フィールド追加
  2. `find_owner_window` で都度逆引き
- **Selected Approach**: Option 2 — find_owner_window で都度逆引き
- **Rationale**: PointerState はフレーム単位で更新される一時的な状態。フィールド追加は構造体を肥大化させ、全生成箇所の修正が必要。逆引きコストは実質ゼロ
- **Trade-offs**: ✅ 構造体変更なし、既存コード影響ゼロ / ❌ 毎回逆引きのオーバーヘッド（無視可能）

### Decision: DragState HWND 検証方式

- **Context**: G3 — 異なるウィンドウのボタンUpがドラッグを誤終了させる可能性
- **Alternatives Considered**:
  1. `end_dragging()` 内部で HWND 引数を受け取り検証
  2. 呼び出し側（`handle_button_message`）でガード条件を追加
  3. DragState を `HashMap<HWND, DragState>` に変更
- **Selected Approach**: Option 2 — 呼び出し側ガード
- **Rationale**: `DragState::Dragging` に既に `hwnd` フィールドが存在。呼び出し側で `hwnd` 一致を確認するだけで十分。DragState の API 変更を伴わず最小影響
- **Trade-offs**: ✅ API 変更なし、最小差分 / ❌ 呼び出し側の責任が増える（1箇所のみ）

## Risks & Mitigations

- **R1: 既存シングルウィンドウ動作の回帰** — 全修正後に既存 `taffy_flex_demo` の動作確認を実施。マルチウィンドウ化しても1ウィンドウのみのケースは既存と等価であることをテストで保証
- **R2: WndProc 内の unsafe コード影響** — `find_owner_window` は safe コードのみ（ECS クエリ）。unsafe 領域への変更なし
- **R3: thread_local! バッファの整合性** — Entity キーのグローバルユニーク性により、バッファ構造自体の変更は不要。`find_owner_window` による選択的クリアで対応

## References

- `drag/dispatch.rs` L86-126: 既存の ChildOf 逆走査パターン（find_owner_window の参考実装）
- `handlers.rs` L829-849: WM_MOUSELEAVE の現在の実装（修正対象）
- `handlers.rs` L671-686, L732-740: WM_MOUSEMOVE の leave 処理（修正対象）
- `dispatch.rs` L113-121: build_bubble_path の現在の実装（修正対象）
- `state.rs` L252-270: end_dragging の現在の実装（呼び出し側を修正）
