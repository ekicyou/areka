# W6b-T: wintf ドラッグ × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W6b-T（領域 W6b「wintf ドラッグ」 × 観点 T「テスト網羅性」）
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。**W6b 領域の最初のセル**（先行 W6b 断片なし）。`ecs/drag/` のモジュール×テスト対応表をゼロから作成した。
- requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9、レビュー観点列 T、CellExecutor 観点別規則（T）、セル断片様式、提案記録様式。領域定義「W6b: wintf ドラッグ / `crates/wintf/src/ecs/drag/` / 1,410 LOC / テスト薄め」
- 参考: `report/cells/W6a-T.md`（直前のポインター入力 T セル）・`W4b-T.md`（T セルの様式・モジュール×テスト対応表）

## 対象ファイル一覧（W6b = `crates/wintf/src/ecs/drag/`）

- `mod.rs`（モジュール re-export + コンポーネント定義: `DragConfig`(Default)、`DraggingState`、`DragConstraint`(apply クランプ)、`WindowDragging` マーカー、115→約200 LOC）
- `state.rs`（**ドラッグ状態機械**: `DragState` enum 5 状態 / `DragStateSnapshot` / `DragState::snapshot` / thread_local `DRAG_STATE` / `update_drag_state` / `read_drag_state` / `snapshot_drag_state` / **`start_preparing`** / **`start_dragging`** / **`update_dragging`** / **`end_dragging`** / **`cancel_dragging`** / `reset_to_idle` / **`check_threshold`**、510→約 900 LOC）
- `accumulator.rs`（wndproc→ECS 転送: `DragAccumulator`(accumulate_delta/update_position/set_transition/flush/Default)、`FlushResult`、`DragAccumulatorResource`(Arc<Mutex>)、`DragTransition` enum、166→約 320 LOC）
- `context.rs`（ECS→wndproc 転送: `WindowDragContext`(Default/clear)、`WindowDragContextResource`(new/set/get/clear)、96→約 200 LOC）
- `dispatch.rs`（**`dispatch_drag_events`**: Started/Ended 遷移処理・DraggingState 挿入/削除・WindowDragging マーカー・Arrangement.offset 同期・DragStart/Drag/DragEnd メッセージ配信、`DragStartEvent`/`DragEvent`/`DragEndEvent`、`OnDragStart`/`OnDrag`/`OnDragEnd` ハンドラ、393 LOC）
- `capture_guard.rs`（マウスキャプチャ RAII: `CaptureGuard`(acquire/mark_released/is_released/Drop/Debug)、本体 + in-source tests、105 LOC）
- `systems.rs`（`cleanup_drag_state`: DragEndEvent → DraggingState 削除、28 LOC）

合計 約 1,413 LOC（design.md W6b 概算 1,410 と整合）。境界 = `ecs/drag/` のみ。`ecs/pointer/`（W6a）・`window_proc/`（W7a）には一切触れていない。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `mod.rs` | `DragConfig::default`（threshold=5・左ボタンのみ）、**`DragConstraint::apply`**（min/max クランプ・軸独立）、`DraggingState`/`WindowDragging`（マーカー） | **なし（純粋データ/計算）** | **なし（0件）**（`DragConfig::default` は boxstyle_coordinate_separation_test で incidental に component として使用されるのみ・apply は未検証） | **6件** | 空白: `DragConfig` 既定値、`DragConstraint::apply` の全 None 素通し・範囲内不変・min クランプ・max クランプ・軸独立（X のみ制約 Y 素通し）。`DraggingState`/`WindowDragging` は dispatch / window_dragging_filter_test 側で挙動検証されるため mod.rs では型定義のみ |
| `state.rs` | **状態機械全体**: `start_preparing`（Idle/JustEnded→Preparing、active 中は無視）、`start_dragging`（Preparing→JustStarted）、`update_dragging`（JustStarted→Dragging（context 読取）/ Dragging→Dragging（prev 更新））、`end_dragging`（→JustEnded・cancelled 伝播）、`cancel_dragging`（→JustEnded cancelled=true・position=start_pos）、`reset_to_idle`（JustEnded のみ→Idle）、`check_threshold`（distance_sq>=threshold_sq）、`DragState::snapshot`、`read_drag_state` | **状態機械はデバイス非依存**（thread_local + null HWND）。唯一の Win32 接点 `CaptureGuard::acquire/drop`（SetCapture/ReleaseCapture）は UI スレッド外で null HWND に対し実質 no-op（capture_guard_panic_safety_test が実証済み） | **なし（0件・最大の空白）** | **21件** | **最大の空白（0テスト）**: 5 状態遷移マトリクスを全面特性化。各テストは thread_local 汚染回避のため `force_idle()` で開始（CaptureGuard を borrow 解放後にドロップ）。`update_dragging` の context 取込（hwnd/initial_window_pos/move_window/constraint）と context=None フォールバック（HWND::default/(0,0)/false/None）、Dragging→Dragging の prev_pos 保持、`check_threshold` の (3,4)=距離5=閾値ちょうど（>=で true）/(3,3)<5（false）/非 Preparing で false、`snapshot` 5 バリアント写像を固定。Entity は `Entity::from_raw_u32` で World 無しに生成 |
| `accumulator.rs` | `DragAccumulator`（accumulate_delta 加算・update_position 上書き・set_transition（Started→entity Some / Ended→entity None）・flush（delta リセット・transition take・position/entity 保持）・Default）、`DragAccumulatorResource`（Arc<Mutex> 共有・各メソッド） | **なし（純粋 + Arc<Mutex>、デバイス非依存）** | **なし（0件）** | **8件** | **空白（0テスト）**: new/Default 空状態、accumulate の累積加算、flush の delta リセット（position/entity 保持・transition 消費）、set_transition Started/Ended の entity 設定/クリア、update_position 上書き、Resource の clone 越し状態共有（accumulate/set_transition の往復観測） |
| `context.rs` | `WindowDragContext`（Default 未設定・clear 全フィールドリセット）、`WindowDragContextResource`（set/get 往復・new で Default・clear・Arc<Mutex> 共有） | **なし（純粋 + Arc<Mutex>、デバイス非依存）** | **なし（0件）** | **6件** | **空白（0テスト）**: Default が全 None/false、clear が設定済みを Default へ、Resource の set→get 往復（initial_window_pos/move_window）、new の Default 読出、clear 後の未設定復帰、clone 越し共有 |
| `dispatch.rs` | **`dispatch_drag_events`**: Started（Window 祖先探索・DraggingState 挿入・WindowDragging 付与・WindowDragContext 書込・DragStartEvent 配信）、Ended（DragEndEvent 配信・DraggingState/WindowDragging 除去・Arrangement.offset 同期・context クリア）、DragEvent（デルタ非ゼロで配信・prev_frame_pos 更新）。`DragStartEvent`/`DragEvent`/`DragEndEvent`、`OnDrag*` ハンドラ | **bevy World 操作はデバイス非依存**（WindowHandle なしフォールバック経路を選べば実 HWND 不要）。実 HWND + `client_to_window_coords`（GDI）経路のみ Win32 依存（所見1） | **なし（0件・統合テストにコメント言及のみ）** | **9件**（新規 `tests/drag/dispatch_test.rs`） | **空白（0テスト）**: Started の DraggingState 挿入＋WindowDragging マーカー＋DragStartEvent 1件＋initial_inset=WindowPos.position（WindowHandle なしフォールバック）、DragConfig.move_window/DragConstraint の context 転送、Ended の状態/マーカー除去＋Arrangement.offset 同期＋DragEndEvent、cancelled フラグ伝播＋context クリア、デルタ非ゼロで DragEvent＋start_position=DraggingState.drag_start_pos＋prev_frame_pos 更新、デルタゼロで DragEvent 不発、遷移/対象なしで no-op（パニックなし）。`cleanup_drag_state`（systems.rs）の DragEndEvent→DraggingState 削除/イベントなし noop も本ファイルで固定 |
| `capture_guard.rs` | `CaptureGuard`（acquire/mark_released/is_released/Drop の ReleaseCapture スキップ/Debug） | `acquire`/`Drop` は SetCapture/ReleaseCapture（Win32）だが UI スレッド外・null HWND で実質 no-op。`mark_released`/`is_released`/`Debug` は純粋 | in-source `mod tests` 3件（null_hwnd 生成/mark_released フラグ/Debug 形式）+ 統合 `capture_guard_panic_safety_test.rs` 3件（panic 時 Drop / catch_unwind / 正常スコープ） | 0件 | 既存 6 件で acquire・mark_released・is_released・Drop（panic/catch_unwind/正常）・Debug を網羅済み。過不足整理: 不足なし（`is_released` は本番読み取りゼロのテスト専用アクセサ → 所見2/P61 同時検討候補） |
| `systems.rs` | `cleanup_drag_state`（DragEndEvent → DraggingState remove） | **なし（bevy World、デバイス非依存）** | **なし（0件）** | （上記 dispatch_test に 2件） | 空白を `tests/drag/dispatch_test.rs` で固定（DragEndEvent 受信で削除・イベントなしで残置）。dispatch.rs と同じテストファイルに集約（同一ドメイン・同一リソース構成のため） |

追加テスト合計 **50件**（mod 6・state 21・accumulator 8・context 6 の **in-source 41件** + dispatch/cleanup の **統合 9件**）。**プロダクションコードの変更なし**（R5.1 充足。git diff: in-source 追加 `#[test]` = 41・削除 0、すべて `#[cfg(test)]` 内。新規統合ファイル 1件 = 9）。新規テストファイル: `tests/drag/dispatch_test.rs`（9件）+ 束ね役 `tests/drag.rs` への mod 1行追記。in-source は `state.rs`/`accumulator.rs`/`context.rs`/`mod.rs` へ `mod tests` を新規作成（4ファイル）。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/drag/mod.rs`（in-source `mod tests`・新規, 6件）**
- `test_drag_config_default_values` — DragConfig 既定（threshold=5・enabled・move_window・左のみ有効）
- `test_drag_constraint_apply_no_bounds_passthrough` — 全 None で座標素通し
- `test_drag_constraint_apply_within_bounds_unchanged` — 範囲内は不変
- `test_drag_constraint_apply_clamps_to_min` — 下限未満は min クランプ（x/y 個別）
- `test_drag_constraint_apply_clamps_to_max` — 上限超過は max クランプ（x/y 個別）
- `test_drag_constraint_apply_axis_independent` — X のみ制約・Y 素通しの軸独立

**`crates/wintf/src/ecs/drag/state.rs`（in-source `mod tests`・新規, 21件）**
- `test_start_preparing_from_idle_enters_preparing` — Idle→Preparing・entity/start_pos 保持
- `test_start_preparing_ignored_when_already_active` — Preparing 中の再 press 無視（最初の press 維持）
- `test_start_preparing_allowed_from_just_ended` — JustEnded からの新規 preparing 許可
- `test_start_dragging_preparing_to_just_started` — Preparing→JustStarted・current_pos 反映
- `test_start_dragging_noop_when_not_preparing` — 非 Preparing で no-op
- `test_update_dragging_just_started_to_dragging_without_context` — context=None で HWND/位置/move_window/constraint デフォルト・prev_pos=current_pos
- `test_update_dragging_reads_window_drag_context` — context の hwnd/initial_window_pos/move_window/constraint 取込
- `test_update_dragging_dragging_updates_prev_pos` — Dragging→Dragging で prev_pos=直前 current_pos
- `test_update_dragging_noop_when_idle` — Idle で no-op
- `test_end_dragging_from_preparing` — Preparing→JustEnded(cancelled=false)・position/entity
- `test_end_dragging_from_dragging_preserves_cancelled_flag` — Dragging→JustEnded(cancelled=true 伝播)
- `test_end_dragging_noop_when_idle` — Idle で no-op
- `test_cancel_dragging_uses_start_pos_and_sets_cancelled` — JustEnded(cancelled=true・position=start_pos)
- `test_cancel_dragging_noop_when_idle` — Idle で no-op
- `test_reset_to_idle_only_from_just_ended` — JustEnded→Idle
- `test_reset_to_idle_noop_when_preparing` — Preparing で変化なし
- `test_check_threshold_true_at_or_beyond_distance` — (3,4)=距離5=閾値（>=で true）・(10,0)>5 で true
- `test_check_threshold_false_below_distance` — (3,3)≈4.24<5 で false
- `test_check_threshold_false_when_not_preparing` — 非 Preparing で常に false（warn のみ）
- `test_snapshot_maps_each_variant` — 5 状態の snapshot バリアント写像
- `test_read_drag_state_observes_current_state` — read_drag_state が現在状態参照を渡す

**`crates/wintf/src/ecs/drag/accumulator.rs`（in-source `mod tests`・新規, 8件）**
- `test_new_is_empty` — new/Default は delta/position 0・entity/transition None
- `test_accumulate_delta_sums` — accumulate の累積加算
- `test_flush_resets_delta_but_keeps_position_and_entity` — flush で delta リセット・position/entity 保持・transition 消費
- `test_set_transition_started_sets_entity_and_is_taken_on_flush` — Started で entity Some・flush で take
- `test_set_transition_ended_clears_entity` — Ended で entity None
- `test_update_position_overwrites` — position 上書き（累積でない）
- `test_resource_shares_inner_state_across_clone` — Resource clone 越し共有（accumulate/position）
- `test_resource_set_transition_reflected_in_flush` — Resource set_transition の反映

**`crates/wintf/src/ecs/drag/context.rs`（in-source `mod tests`・新規, 6件）**
- `test_default_is_unset` — Default 全 None/false
- `test_clear_resets_all_fields` — clear で Default 相当へ
- `test_resource_set_then_get_roundtrip` — set→get 往復
- `test_resource_new_returns_default` — new の Default 読出
- `test_resource_clear_resets` — clear 後の未設定復帰
- `test_resource_shares_state_across_clone` — clone 越し共有

**`crates/wintf/tests/drag/dispatch_test.rs`（新規, 9件）**
- `dispatch_started_inserts_dragging_state_and_marker_and_event` — Started: DraggingState 挿入＋WindowDragging マーカー＋DragStartEvent 1件＋initial_inset（WindowHandle なしフォールバック）
- `dispatch_started_writes_drag_context_resource` — Started: DragConfig.move_window/DragConstraint の context 転送
- `dispatch_ended_removes_state_marker_and_syncs_offset` — Ended: DraggingState 削除＋WindowDragging 除去＋Arrangement.offset 同期＋DragEndEvent
- `dispatch_ended_cancelled_propagates_flag` — Ended: cancelled=true 伝播＋context クリア
- `dispatch_emits_drag_event_when_delta_nonzero` — デルタ非ゼロで DragEvent＋start_position＝drag_start_pos＋prev_frame_pos 更新
- `dispatch_no_drag_event_when_delta_zero` — デルタゼロで DragEvent 不発
- `dispatch_noop_when_no_transition_and_no_entity` — 遷移/対象なしで全メッセージ不発（パニックなし）
- `cleanup_removes_dragging_state_on_end_event` — cleanup_drag_state: DragEndEvent で DraggingState 削除
- `cleanup_noop_without_end_event` — cleanup_drag_state: イベントなしで残置

## 除外したテスト

なし。`capture_guard.rs`（in-source 3件 + 統合 capture_guard_panic_safety 3件 = 6件）に重複・死テスト（到達不能・常に真・対象消失）は検出されなかった（acquire/mark_released/is_released/Drop の panic・catch_unwind・正常スコープ/Debug を異なる観点で固定しており冗長でない）。`window_dragging_filter_test.rs`（7件）は W4b-T で window_pos systems の Without<WindowDragging> フィルタとして既に整理済みで、本セルでは drag ドメインのマーカー消費契約の再検証として残置（除外せず）。`mod.rs`/`state.rs`/`accumulator.rs`/`context.rs`/`systems.rs`/`dispatch.rs` は 0 テストだったため除外対象自体が存在しない。過不足整理の結論: **不足のみ存在（50件で充足）、過剰なし**。

## テスト不能箇所・深掘り所見（R2.8）

1. **`dispatch_drag_events` の実 HWND 経路（`client_to_window_coords` / GDI）はデバイス依存** — Started 処理は Window 祖先に `WindowHandle` がある場合、`wh.client_to_window_coords(pos, size)`（dispatch.rs:110-114、GDI `AdjustWindowRectEx` 系を内部で使用）でクライアント領域座標→ウィンドウ枠込みスクリーン座標へ変換し `initial_window_pos` を導出する。この変換は実ウィンドウスタイル/DPI に依存し、ユニットで決定的に再現できない。本セルでは **WindowHandle なしフォールバック経路**（dispatch.rs:117-124、`WindowPos.position` を直接 initial_window_pos に使う）を選んで Started の World 効果（DraggingState/マーカー/メッセージ/context 転送）を特性化した。実 HWND 経路は実起動 + 実ウィンドウが最終的な回帰検知器（既存の S7 起動テスト・実環境）であり、テスト不能は環境制約のため提案化しない。

2. **`CaptureGuard::acquire`/`Drop` の実 SetCapture/ReleaseCapture はデバイス依存だが既存テストで回避済み** — `SetCapture`/`ReleaseCapture`（KeyboardAndMouse）は UI スレッドのマウスキャプチャ API であり、本来は実 HWND と実 UI メッセージループを要する。ただし `capture_guard_panic_safety_test.rs` の NOTE どおり、テストスレッド・null HWND では実質 no-op として安全に呼べる（副作用なし）。この性質を利用し、state.rs の状態機械テスト 21件は `CaptureGuard` を内包したまま null HWND で全遷移を駆動できた（`force_idle()` で borrow 解放後に Drop）。実キャプチャ取得/解放の OS 副作用（実際のマウスキャプチャ移動・`WM_CAPTURECHANGED` 配信）は実環境統合経路でのみ検証可能で、ユニット不能は環境制約のため提案化しない。なお `CaptureGuard::is_released`（capture_guard.rs:47-50）は `#[allow(dead_code)]` 付きで**本番読み取りゼロ・テスト 2 件のみが参照**するアクセサであり、デッドコード整理の副次候補として P61 に併記した（優先度は低）。

3. **`DraggingState.prev_frame_pos` は毎ドラッグフレーム書き込み・本番読み取りゼロのデッドストア（→ P61）** — mod.rs:76-78 のコメント「デルタ計算用、現在は未使用」どおり、`dispatch_drag_events` は Started 初期化（dispatch.rs:160）と DragEvent 発火時（dispatch.rs:382-388）に `prev_frame_pos` を書き込むが、ワークスペース全域で**読み取り箇所が存在しない**（grep 実証: 定義1・書込2・読取0。デルタは `current_position - drag_start_pos` で都度算出）。ドラッグ移動という高頻度経路の無駄な書き込みであり、削除候補。`pub` フィールド除去（型シグネチャ変更）かつ dispatch.rs の get_mut ブロック整理を伴うため R2.9/R2.10 に従い本 T セルでは実装せず P61 に記録。本セル追加の `dispatch_emits_drag_event_when_delta_nonzero` が現行更新挙動を特性化しており削除時の回帰検知器となる。

4. **wndproc 層（`window_proc/mouse_*`・`keyboard.rs`）からの状態機械呼び出しは W7a 境界** — `start_preparing`/`start_dragging`/`update_dragging`/`end_dragging`/`cancel_dragging` の本番呼び出し元は `crates/wintf/src/ecs/window_proc/`（mouse_move.rs / mouse_click.rs / keyboard.rs）であり、これは W6b 境界外（W7a）。本セルでは状態機械そのものの遷移契約を in-source で特性化するに留め、wndproc メッセージ→状態遷移の結線（実 WPARAM/LPARAM 解釈・実 HWND・祖先ウィンドウ探索 `find_owner_window`）は W7a セルの担当とした。これらは実 Win32 メッセージを要し本セルからはテスト不能（境界外かつデバイス依存）。

## proposals へ回した候補

- **P61**: `DraggingState.prev_frame_pos` のデッドストア整理（毎ドラッグフレーム書き込み・本番読み取りゼロのフィールド削除候補。`CaptureGuard::is_released` のテスト専用アクセサ整理も併記）。R2.9/R2.10 適用域。

既存提案との関連: W6a-T の P57〜P60（ポインター入力のデッド関数・二重実装・デッドストレージ・thread_local キー寿命）と同系統の「書き込まれるが消費されないストレージ／本番未使用 API」の所見であり、ドラッグ側にも prev_frame_pos のデッドストアが存在することを示した。

## verification (S2)

- BEFORE: 親のベースライン（1516 passed / 0 failed・クリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。触れたバイナリ（wintf lib・wintf drag 統合）の BEFORE 内訳は git diff（in-source 追加 41件・統合新規 9件・削除 0件）と AFTER 実測の差分から逆算して検証した。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1566 passed / 0 failed**（全テストバイナリで failed=0、awk による全 `test result` 行の合算で実測。`test result: FAILED`/`error`/`panicked` 行ゼロ）。
  - グローバル合計は 1516 → 1566（**+50**）。追加分の内訳は wintf lib in-source（`--lib`）+41、wintf drag 統合バイナリ +9。
  - 触れたファイルの新規 `#[test]` 件数内訳（git diff の実数と完全一致。`git diff --unified=0 -- crates/wintf/src/ecs/drag | grep -c "^+.*#\[test\]"` = 41、削除 0。新規ファイル `tests/drag/dispatch_test.rs` の `#[test]` = 9）:
    - `mod.rs`: **0 → 6（+6）**
    - `state.rs`: **0 → 21（+21）**
    - `accumulator.rs`: **0 → 8（+8）**
    - `context.rs`: **0 → 6（+6）**
    - `capture_guard.rs`: 3 → 3（+0）
    - in-source 小計 **+41**（6+21+8+6）
    - `tests/drag/dispatch_test.rs`: **新規 9件**
    - 合計 **+50**（41+9）
  - 反復検証: `cargo test -p wintf --lib drag::` で drag in-source **44 passed / 0 failed**（既存 capture_guard 3 + 追加 41）。`cargo test -p wintf --test drag` で drag 統合バイナリ **19 passed / 0 failed**（既存 capture_guard_panic_safety 3 + window_dragging_filter 7 + 追加 dispatch_test 9）。
  - 全50件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。深掘りを要する初回失敗なし（バグ・前提誤りの検出なし）。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W6b 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリは **155 passed / 0 failed** と合格（`bench_pop_ready_empty_queue` 含め全 `... ok`、隔離再実行不要）。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --tests` は既存警告（`collapsible_if`/`needless let-chains`/`Default` derive 推奨 等）を出力。**いずれも本セルの追加テスト由来ではない**。clippy 診断が参照する drag 配下ファイルの行番号を全列挙（`grep -oE "(src|tests)[\\/]...drag[\\/]....rs:N"`）した結果、参照先はすべて **プロダクションコード**（`dispatch.rs:107/111/119/121/173/341/383`、`state.rs:185/195/206`、`context.rs:32` の既存 `impl Default`）であり、本セルで追加した `mod tests`（state.rs:511 以降 / accumulator.rs:167 以降 / context.rs:97 以降 / mod.rs:115 以降）および新規 `tests/drag/dispatch_test.rs` を指す診断は**ゼロ**。本セルはテスト追加のみでプロダクションコード未変更のため、新規 clippy 警告/error の導入はゼロ。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点の担当）。

## RED フェーズ代替の検証

追加50件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した:
- **mod**: `DragConfig::default`（mod.rs:52-63）の各既定値、`DragConstraint::apply`（mod.rs:99-105）の `map_or` クランプ順（min→max・軸独立）をソースから転記。
- **state**: 5 状態遷移の各 match アーム — `start_preparing` の active 中 early-return（state.rs:227-235）、`start_dragging` の Preparing ガード（:262）、`update_dragging` の JustStarted/Dragging 分岐と context 読取（:305-385）、`end_dragging`/`cancel_dragging` の JustEnded 生成と CaptureGuard 抽出（:398-471）、`reset_to_idle` の JustEnded ガード（:477）、`check_threshold` の distance_sq>=threshold_sq（:485-508）、`snapshot` の全バリアント写像（:123-180）をソースから導出。
- **accumulator**: `flush` の「delta リセット・transition take・position/entity 保持」（accumulator.rs:80-95）、`set_transition` の Started→entity Some / Ended→entity None（:65-75）、`accumulate_delta` 加算（:54-57）、`update_position` 上書き（:60-62）、Resource の Arc<Mutex> 共有をソースから導出。
- **context**: `WindowDragContext::default`/`clear`（context.rs:32-51）、Resource の set/get/clear（:72-88）の Arc<Mutex> 往復をソースから転記。
- **dispatch**: Started の DraggingState 挿入（dispatch.rs:156-161）・WindowDragging 付与（:173-181）・DragStartEvent write（:198-200）・WindowHandle なしフォールバック initial_window_pos（:117-124）、Ended の DraggingState remove（:252-253）・WindowDragging remove（:267-268）・Arrangement.offset 同期（:288-318）・context clear（:331-335）、DragEvent のデルタ非ゼロゲート（:341-342）・start_position 取得（:344-347）・prev_frame_pos 更新（:382-388）、`cleanup_drag_state`（systems.rs:12-27）の DragEndEvent→remove をソースから導出。初回実行で50件全件が導出どおり一致し、バグ・前提誤りは検出されなかった。
