# Gap Analysis: multiwindow-event-validation

## 概要

`taffy_flex_demo` のマルチウィンドウ化と、wintf クレートにおけるマルチウィンドウイベント処理の検証・修正を対象とする。既存コードベースを詳細に調査した結果、ウィンドウ生成・グラフィックス・ヒットテストはマルチウィンドウ対応済みだが、**ポインタ状態管理・イベントディスパッチ・ドラッグシステム**に複数の未対応箇所が確認された。

---

## 1. Current State Investigation

### 関連ファイル・モジュール

| モジュール | パス | 責務 |
|---|---|---|
| Window Management | `ecs/window.rs`, `ecs/window_system.rs` | ウィンドウ生成、HWND-Entity マッピング |
| Window Procedure | `ecs/window_proc/mod.rs`, `handlers.rs` | Win32メッセージ → ECS変換 |
| Pointer System | `ecs/pointer/mod.rs`, `dispatch.rs` | PointerState管理、Tunnel/Bubbleイベント配信 |
| Drag System | `ecs/drag/` (6ファイル) | ドラッグ状態機械、ウィンドウ移動 |
| Hit Test | `ecs/layout/hit_test.rs` | ヒットテスト（Bounds/AlphaMask） |
| Demo | `examples/taffy_flex_demo.rs` | シングルウィンドウFlexboxデモ |
| Multi Window Test | `examples/multi_window_test.rs` | 3ウィンドウ生成・グラフィックス初期化のみ |

### マルチウィンドウ対応状況

| 機能 | 状態 | 備考 |
|---|---|---|
| ウィンドウ生成 | ✅ 対応済み | `GWLP_USERDATA` による Entity-HWND 双方向マッピング |
| グラフィックス初期化 | ✅ 対応済み | `multi_window_test` で検証済み |
| ヒットテスト | ✅ 対応済み | `hit_test_in_window` がウィンドウスコープで走査 |
| WM_MOUSELEAVE | ❌ 未対応 | 全エンティティのPointerStateをクリア |
| WM_MOUSEMOVE Leave | ❌ 未対応 | entities_to_leave がグローバルクエリ |
| ドラッグ状態 | ⚠️ 制約あり | thread_local! シングルトン |
| SetCapture | ❌ 未実装 | TODOコメントのみ |
| イベント伝播境界 | ⚠️ 未明示 | LayoutRootまで伝播 |
| テスト | ❌ 皆無 | イベント処理のマルチウィンドウテストなし |

### アーキテクチャパターン

- **階層構造**: `LayoutRoot → Window(s) → ChildOf → ウィジェットツリー`
- **イベント配信**: WndProc → hit_test → PointerState/ButtonBuffer → ECSフレームで dispatch
- **thread_local! バッファ**: `POINTER_BUFFERS`, `BUTTON_BUFFERS`, `WHEEL_BUFFERS`, `DRAG_STATE` — WndProc↔ECS間のブリッジ
- **SetWindowPos 遅延パターン**: World借用競合回避のため `deferred_set_window_pos` を使用

---

## 2. Requirement-to-Asset Map

| Req | 要件 | 関連資産 | ギャップ |
|-----|------|----------|----------|
| Req1 | マルチウィンドウデモ | `taffy_flex_demo.rs`, `multi_window_test.rs` | **Missing**: マルチウィンドウ版デモが存在しない |
| Req2 | WM_MOUSELEAVE スコープ修正 | `handlers.rs` L815-860 (WM_MOUSELEAVE), L673-742 (WM_MOUSEMOVE Leave) | **Missing**: ウィンドウスコーピングロジック |
| Req2 | エンティティ-ウィンドウ所有権クエリ | `drag/dispatch.rs` L96-118 (ad-hoc実装のみ) | **Missing**: 共通 `find_owner_window()` ユーティリティ |
| Req3 | ドラッグ状態安全性 | `drag/state.rs` (DRAG_STATE), `drag/context.rs` | **Constraint**: シングルトン設計だがWin32のSetCapture制約と整合 |
| Req3 | SetCapture/ReleaseCapture | `handlers.rs` L1014-1015 (TODOコメント) | **Missing**: 完全に未実装 |
| Req4 | イベント伝播境界 | `pointer/dispatch.rs` (build_bubble_path) | **Missing**: ウィンドウ境界での停止条件 |
| Req5 | 統合テスト | `tests/` (31ファイル) | **Missing**: マルチウィンドウイベントテスト皆無 |

---

## 3. 詳細ギャップ分析

### G1: WM_MOUSELEAVE のグローバルクリア【Critical】

**場所**: [handlers.rs](crates/wintf/src/ecs/window_proc/handlers.rs) L815-860

**現状コード**:
```rust
pub(super) fn WM_MOUSELEAVE(hwnd, ...) -> HandlerResult {
    let mut query = world_mut.query::<(Entity, &PointerState)>();
    for (e, _) in query.iter(world_mut) {
        entities_with_pointer_state.push(e);  // ← 全エンティティ対象
    }
}
```

**問題**: `hwnd` からウィンドウエンティティを取得可能だが、PointerStateクリアが全エンティティ対象のグローバルクエリ。Window AのMOUSELEAVEがWindow Bのホバー状態も破壊する。

**修正方針**: ウィンドウエンティティの子孫のみをフィルタリングする。`ChildOf` チェーンを辿り、当該ウィンドウ配下のエンティティのみを対象とする。

### G2: WM_MOUSEMOVE の Leave 処理もグローバル【Critical】

**場所**: [handlers.rs](crates/wintf/src/ecs/window_proc/handlers.rs) L673-684, L731-742

**問題**: `WM_MOUSEMOVE` ハンドラ内の「新しいhitから外れたエンティティをleaveにする」処理も、`PointerState` の全クエリを使用。G1と同根の問題。

### G3: thread_local! ドラッグ状態シングルトン【Medium】

**場所**: [drag/state.rs](crates/wintf/src/ecs/drag/state.rs) L74-78

**現状**: `DRAG_STATE` がプロセス全体で1つ。  
**分析**: Win32の `SetCapture` 自体がプロセスで1つのHWNDのみキャプチャ可能なため、「同時に1つのドラッグのみ」は実質的にOS制約と整合する。ただし、ウィンドウAドラッグ中にウィンドウBのボタンUpがドラッグ終了をトリガーする可能性がある（state.rs の `try_end()` がHWND検証なし）。

**修正方針**: `DragState` にHWNDまたはウィンドウEntityを保持させ、終了処理時にHWNDを検証する。

### G4: SetCapture/ReleaseCapture 未実装【High】

**場所**: [handlers.rs](crates/wintf/src/ecs/window_proc/handlers.rs) L1014-1015

**現状**: TODO コメントとして5箇所に記載:
- `handle_button_message` のDown処理
- ドラッグ開始準備
- ドラッグ終了処理

**影響**: ドラッグ中にマウスがウィンドウのクライアント領域外に出ると `WM_MOUSEMOVE` が来なくなり、ドラッグが途切れる。マルチウィンドウ環境で顕著（ウィンドウ間をまたぐ移動時）。

**Research Needed**: `windows` クレートの現バージョン (0.62.2) で `SetCapture`/`ReleaseCapture` が利用可能か確認が必要。

### G5: build_bubble_path がウィンドウ境界で停止しない【Medium】

**場所**: [pointer/dispatch.rs](crates/wintf/src/ecs/pointer/dispatch.rs) L123-131

**現状コード**:
```rust
pub fn build_bubble_path(world: &World, start: Entity) -> Vec<Entity> {
    let mut path = vec![start];
    let mut current = start;
    while let Some(child_of) = world.get::<ChildOf>(current) {
        path.push(child_of.parent());
        current = child_of.parent();
    }
    path
}
```

**問題**: `ChildOf` をLayoutRootまで無条件に辿る。Windowエンティティは含まれるため Window にハンドラがあれば正しく配信されるが、LayoutRoot にもハンドラが登録されると全ウィンドウのイベントが集約される。

**修正方針**: `Window` コンポーネントを持つエンティティで停止するか、明示的にウィンドウ境界フラグを導入する。

### G6: PointerState にウィンドウ情報なし【Medium】

**場所**: [pointer/mod.rs](crates/wintf/src/ecs/pointer/mod.rs) L121-186

**問題**: `PointerState` がどのウィンドウのマウスイベント由来かを保持していない。WM_MOUSELEAVEのスコーピングやデバッグに不便。

**修正方針**: PointerState にソースウィンドウ Entity フィールドを追加する、または所有権ユーティリティ（G7）で代替。

### G7: find_owner_window ユーティリティの不在【Medium】

**既存の ad-hoc 実装**: `drag/dispatch.rs` L96-118

**問題**: 「エンティティが所属するウィンドウ」を逆引きする共通関数がない。WM_MOUSELEAVE修正、イベントスコーピング、テストなど複数箇所で必要。

**修正方針**: `ecs/window.rs` に `find_owner_window(world: &World, entity: Entity) -> Option<Entity>` を追加。

### G8: thread_local! バッファのウィンドウ非分離【Medium】

**場所**: [pointer/mod.rs](crates/wintf/src/ecs/pointer/mod.rs)

**現状**: `POINTER_BUFFERS`, `BUTTON_BUFFERS`, `WHEEL_BUFFERS` はEntity IDをキーとしたHashMap。Entity IDはグローバルユニークなので異なるウィンドウ間でキー衝突はしないが、WM_MOUSELEAVE等で「このウィンドウに属するバッファをまとめてクリア」する操作ができない。

**修正方針**: G7のユーティリティと組み合わせてスコープ付きクリアを実装。バッファ構造自体の変更は不要。

**深刻度補足**: Entity IDのグローバルユニーク性により衝突は発生しない。G7 (`find_owner_window`) で解決可能なため、単独の修正対象としては Medium。

### G9: ホイールイベントのhit_test非経由【Low】

**問題**: `add_wheel_vertical/horizontal` が `hwnd` → Entity直接変換で、hit_testを経由せず子エンティティに配信されない。本仕様スコープ外だが記録。

### G10: マルチウィンドウテスト皆無【High】

**問題**: `tests/` に31ファイルあるが、マルチウィンドウのイベント処理テストが一切存在しない。`multi_window_test.rs` はexample（手動確認用）でありグラフィックス初期化のみ検証。

---

## 4. Implementation Approach Options

### Option A: 既存コンポーネント拡張（Minimal Fix）

**対象**: G1, G2, G3, G4, G5 を既存ファイル内で修正

**変更対象ファイル**:
| ファイル | 変更内容 |
|---|---|
| `ecs/window.rs` | `find_owner_window()` ユーティリティ追加 |
| `ecs/window_proc/handlers.rs` | WM_MOUSELEAVE/WM_MOUSEMOVE のスコーピング修正、SetCapture実装 |
| `ecs/pointer/dispatch.rs` | `build_bubble_path` にウィンドウ境界停止条件追加 |
| `ecs/drag/state.rs` | `DragState` にHWND検証追加 |
| `examples/taffy_flex_demo.rs` | マルチウィンドウ版に改修（既存ファイル変更） |

**トレードオフ**:
- ✅ 変更ファイル最小、既存パターン踏襲
- ✅ 既存テスト・デモへの後方互換性を維持しやすい
- ❌ `taffy_flex_demo.rs` のシングルウィンドウ版が失われる
- ❌ handlers.rs が肥大化

### Option B: 新コンポーネント作成

**対象**: G6, G7 を新ファイルとして分離、デモは別ファイル

**新規作成ファイル**:
| ファイル | 内容 |
|---|---|
| `ecs/window_scope.rs` | `find_owner_window()`, `iter_window_entities()` 等の共通ユーティリティ |
| `examples/multiwindow_flex_demo.rs` | 新規マルチウィンドウデモ（`taffy_flex_demo.rs` を保存） |
| `tests/multiwindow_event_test.rs` | マルチウィンドウイベント統合テスト |

**トレードオフ**:
- ✅ 既存デモ (`taffy_flex_demo.rs`) が保存される
- ✅ ウィンドウスコーピングロジックが集約され再利用しやすい
- ❌ 新ファイル追加でナビゲーションコスト増
- ❌ `window_scope.rs` と `window.rs` の責務境界が曖昧になる可能性

### Option C: ハイブリッドアプローチ【推奨】

**組み合わせ戦略**:

| フェーズ | 作業 | アプローチ |
|---|---|---|
| Phase 1: 基盤 | `find_owner_window()` を `ecs/window.rs` に追加 | 既存拡張 |
| Phase 2: 修正 | WM_MOUSELEAVE, WM_MOUSEMOVE, build_bubble_path を修正 | 既存拡張 |
| Phase 3: ドラッグ | DragState HWND検証 + SetCapture 実装 | 既存拡張 |
| Phase 4: デモ | `multiwindow_flex_demo.rs` を新規作成 | **新規** |
| Phase 5: テスト | `multiwindow_event_test.rs` を新規作成 | **新規** |

**トレードオフ**:
- ✅ 既存ファイルの修正は最小限（バグ修正レベル）
- ✅ 新規デモ・テストは別ファイルで既存を安全に保存
- ✅ 段階的に実装可能（各フェーズ独立検証可能）
- ❌ フェーズ間の整合性管理が必要

---

## 5. Implementation Complexity & Risk

**Effort**: **M (3–7 days)**
- 既存パターンの修正が主体だが、WndProc内のウィンドウスコーピングは慎重なテストが必要。handlers.rs は800行超の複雑なファイル。SetCapture/ReleaseCapture の統合も調査が必要。

**Risk**: **Medium**
- WndProc は unsafe コードと thread_local! が絡む敏感な領域。修正が既存のシングルウィンドウ動作を壊さないことの検証が必須。
- `windows` クレート 0.62.2 で `SetCapture`/`ReleaseCapture` が利用可能かの確認が必要（Research Needed）。
- ドラッグのグローバルシングルトン変更は、既存 `taffy_flex_demo` の動作に影響を与える可能性。

---

## 6. Design Phase への推奨事項

### 推奨アプローチ
**Option C（ハイブリッド）** を推奨。既存コードの修正は最小限のバグ修正として行い、新規デモ・テストは独立ファイルで作成する。

### 設計フェーズで決定すべき事項

1. **`find_owner_window` の実装方式**: `ChildOf` 逆走査 vs キャッシュコンポーネント（`OwnerWindow(Entity)` を各エンティティに付与）
2. **`build_bubble_path` の停止条件**: `Window` コンポーネント検出で停止 vs 明示的フラグ
3. **`PointerState` のウィンドウ情報**: フィールド追加 vs `find_owner_window` で都度逆引き
4. **`DragState` のHWND検証**: 既存シングルトンにHWNDフィールド追加 vs HashMap化

### Research Needed（設計フェーズで調査）

- `windows` クレート 0.62.2 における `SetCapture` / `ReleaseCapture` の利用可否と呼び出しパターン
- `WM_CAPTURECHANGED` メッセージの処理要否（キャプチャが別ウィンドウに奪われた場合）
