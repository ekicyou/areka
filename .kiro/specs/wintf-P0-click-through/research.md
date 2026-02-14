# Research & Design Decisions

## Summary
- **Feature**: `wintf-P0-click-through`
- **Discovery Scope**: Extension（既存 WM_NCHITTEST ハンドラの修正）
- **Key Findings**:
  - 核心変更は `cached_nchittest()` 内の3行分岐のみ。既存インフラ（WM_MOUSELEAVE、TrackMouseEvent、キャッシュ）は変更不要
  - ドラッグ中に HTTRANSPARENT を返すと WM_MOUSEMOVE が途絶しドラッグが中断する。DragState ガードが必要
  - SetCapture は未実装だが将来的な解決策。現時点では DragState 参照による軽量ガードを採用

## Research Log

### Windows WM_NCHITTEST → HTTRANSPARENT メッセージフロー

- **Context**: HTTRANSPARENT 返却時の Windows 標準メッセージシーケンスを確認
- **Sources Consulted**: MSDN WM_NCHITTEST ドキュメント、既存コード分析
- **Findings**:
  1. `WM_NCHITTEST` で `HTTRANSPARENT (-1)` を返すと、Windows はそのウィンドウを「透明」とみなし、下位ウィンドウに `WM_NCHITTEST` を再送する
  2. `TrackMouseEvent(TME_LEAVE)` が設定済みの場合、HTTRANSPARENT 返却後に `WM_MOUSELEAVE` が発行される
  3. HTTRANSPARENT 領域では `WM_MOUSEMOVE` は発行されない（Windows がメッセージを下位ウィンドウに転送するため）
  4. HTCLIENT 領域に再進入すると、通常どおり `WM_MOUSEMOVE` が発行され、`TrackMouseEvent` の再設定も正常に動作する
- **Implications**: 既存の `WM_MOUSELEAVE` ハンドラ（`handlers.rs` L820-876）が PointerState クリーンアップを自動的に処理する。追加実装不要。

### DragState と HTTRANSPARENT の相互作用

- **Context**: ドラッグ中に透明領域を通過した場合の動作を分析
- **Sources Consulted**: `drag/state.rs`, `handlers.rs` WM_MOUSEMOVE ドラッグ処理（L580-700）
- **Findings**:
  1. 現在のドラッグ実装は `SetCapture` を使用していない。WM_MOUSEMOVE の継続受信に依存
  2. DragState は `thread_local!` で管理（`drag/state.rs` L76）。`cached_nchittest` と同一スレッドで実行
  3. ドラッグ中（`Preparing` / `JustStarted` / `Dragging`）に HTTRANSPARENT を返すと WM_MOUSEMOVE が途絶 → ドラッグが中断
  4. `DragState::Idle` / `DragState::JustEnded` のみ HTTRANSPARENT を許可すれば安全
- **Implications**: `cached_nchittest` 内で `read_drag_state()` を呼び、非Idle状態なら HTCLIENT を強制返却する必要がある

### SetCapture vs DragState ガード

- **Context**: ドラッグ中のHTTRANSPARENT問題に対する2つのアプローチを比較
- **Sources Consulted**: MSDN SetCapture ドキュメント、`drag/state.rs`
- **Findings**:

  | アプローチ | 説明 | 利点 | 欠点 |
  |-----------|------|------|------|
  | DragState ガード | `cached_nchittest` で DragState を参照し、非Idle時に HTCLIENT 強制返却 | 変更が局所的、既存コード変更なし | `cached_nchittest` と `drag` モジュールの結合度増加 |
  | SetCapture | ドラッグ開始時に `SetCapture(hwnd)` を呼び、全マウスメッセージをキャプチャ | Win32 標準パターン、HTTRANSPARENT を無視 | 未実装のため変更量大、ReleaseCapture のタイミング管理が必要 |

- **Implications**: 本仕様では DragState ガードを採用（変更の局所性を優先）。SetCapture 対応は将来の仕様として延期。

### WM_MOUSEMOVE の None 分岐

- **Context**: `handlers.rs` WM_MOUSEMOVE ハンドラで `hit_entity = None` の場合の処理を分析
- **Sources Consulted**: `handlers.rs` L700-790
- **Findings**:
  1. 現在: `hit_entity` が None の場合、Window エンティティ自体に PointerState を挿入
  2. HTTRANSPARENT 有効化後: 透明領域では WM_MOUSEMOVE が発行されないため、このパスは到達困難になる
  3. ただし、完全に到達不能ではない: `cached_nchittest` がドラッグ中に HTCLIENT を返す場合、エンティティ無し領域で WM_MOUSEMOVE が到着する可能性あり
- **Implications**: 防衛的コードとして残す（到達可能なエッジケースが存在）。削除は将来の整理タスクとして扱う。

### taffy_flex_demo の構成分析

- **Context**: 手動テスト環境として taffy_flex_demo にクリックスルー領域を追加するための調査
- **Sources Consulted**: `examples/taffy_flex_demo.rs` L150-500
- **Findings**:
  1. 2段構成: 上段（イベントシステムデモ）+ 下段（リージョンテスト）
  2. 下段の `region_container` 内に各種リージョンデモが HitTest::named_regions() で配置
  3. `HitTestMode::None` を使ったエンティティは現在存在しない
  4. 既存の構造から、下段に新たなクリックスルーデモ要素を追加するのが自然
- **Implications**: `region_container` と同階層に、`HitTest::none()` を持つ薄い半透明矩形を追加。隣に通常の `HitTest::bounds()` 矩形を並べ、クリック貫通の対比テストを可能にする。

## Architecture Pattern Evaluation

| Option | 説明 | 利点 | リスク・制約 | 備考 |
|--------|------|------|------------|------|
| Option A: 最小変更 | `cached_nchittest` の3行分岐変更のみ | 極小リスク、既存パターン適合 | ドラッグ断裂リスク | ギャップ分析で推奨 |
| Option A+B: DragState ガード付き | Option A + DragState 参照による HTCLIENT 強制 | ドラッグ安全、変更局所的 | drag モジュールとの結合度増加 | **本仕様で採用** |
| Option C: SetCapture 対応 | ドラッグ時に SetCapture を使用 | Win32 標準 | 実装量大、本仕様のスコープ外 | 将来仕様として延期 |

## Design Decisions

### Decision: DragState ガード付き HTTRANSPARENT 実装

- **Context**: HTTRANSPARENT を返す際にドラッグ操作が中断するリスクへの対処
- **Alternatives Considered**:
  1. Option A 単体 — 3行変更のみ、ドラッグ問題は放置
  2. Option A+B — DragState 参照ガードを追加
  3. Option C — SetCapture を実装
- **Selected Approach**: Option A+B（DragState ガード付き最小変更）
- **Rationale**: 
  - ドラッグ断裂は確実に発生する問題であり、放置不可
  - DragState は同一 thread_local で管理されており、参照コストは無視できる
  - SetCapture は未実装であり、本仕様のスコープ（3行変更レベル）を大幅に超える
- **Trade-offs**: `cached_nchittest` が `drag::read_drag_state` に依存する結合度増加。ただし両者とも wndproc 層のヘルパーであり、レイヤー違反ではない。
- **Follow-up**: SetCapture 対応を将来仕様として検討。実装後は DragState ガードを削除可能。

### Decision: WM_MOUSEMOVE None 分岐の保持

- **Context**: HTTRANSPARENT 有効化後、`hit_entity = None` パスが到達困難になる
- **Alternatives Considered**:
  1. 削除 — デッドコード整理
  2. 保持 — 防衛的コード
- **Selected Approach**: 保持（防衛的コード）
- **Rationale**: DragState ガードにより HTCLIENT 強制返却時にこのパスが到達可能。到達不能の証明が困難なため防衛的に残す。
- **Trade-offs**: わずかなコード冗長性。ただしロジック変更なし。
- **Follow-up**: SetCapture 実装時に再評価。

### Decision: 既存コメント問題の解決策

- **Context**: 「HTTRANSPARENT を返すとマウスイベントがブロックされてしまう」の原因特定
- **Selected Approach**: コメントは過去（WM_MOUSELEAVE 未実装期）の問題を記録したもの。現在は解決済み。
- **Rationale**:
  1. 当時 WM_MOUSELEAVE ハンドラが未実装 → PointerState が残留 → 「ブロック」と認識された
  2. 現在は TrackMouseEvent(TME_LEAVE) + WM_MOUSELEAVE ハンドラが実装済み
  3. HTTRANSPARENT 返却後の正常なメッセージフロー: HTTRANSPARENT → WM_MOUSELEAVE → PointerState 全クリア
- **Follow-up**: コメントを更新し、設計ドキュメントに原因分析を記載（要件 3.1）。

## Risks & Mitigations

- **ドラッグ中の HTTRANSPARENT** — DragState ガードにより HTCLIENT を強制返却して回避
- **高速マウス移動時のメッセージシーケンス** — TrackMouseEvent の再設定は WM_MOUSEMOVE で行われるため、HTCLIENT→HTTRANSPARENT→HTCLIENT の素早い遷移でも TrackMouseEvent が再設定される。実機検証で確認。
- **マルチウィンドウでの PointerState 混入** — 既存の `find_owner_window` スコーピングにより保護済み（変更不要）

## References

- [MSDN: WM_NCHITTEST message](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest) — HTTRANSPARENT (-1) の定義と動作
- [MSDN: TrackMouseEvent function](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-trackmouseevent) — TME_LEAVE によるマウス離脱検出
- [MSDN: SetCapture function](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setcapture) — 将来のドラッグ改善の参考
- ギャップ分析: `.kiro/specs/wintf-P0-click-through/gap-analysis.md`
