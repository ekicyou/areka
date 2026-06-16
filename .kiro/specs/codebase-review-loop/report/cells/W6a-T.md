# W6a-T: wintf ポインター入力 × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W6a-T（領域 W6a「wintf ポインター入力」 × 観点 T「テスト網羅性」）
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。**W6a 領域の最初のセル**（先行 W6a 断片なし）。`ecs/pointer/` のモジュール×テスト対応表をゼロから作成した。
- requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9、レビュー観点列 T、CellExecutor 観点別規則（T）、セル断片様式、提案記録様式。領域定義「W6a: wintf ポインター入力 / `crates/wintf/src/ecs/pointer/` / 1,830 LOC / テスト薄め」
- 参考: `report/cells/W4b-T.md`・`W5b-T.md`（T セルの様式・モジュール×テスト対応表）

## 対象ファイル一覧（W6a = `crates/wintf/src/ecs/pointer/`）

- `mod.rs`（re-export のみ、41 LOC）
- `types.rs`（基本型: PhysicalPoint/DoubleClick/WheelDelta/CursorVelocity/PointerButton/PointerState/PointerLeave/WindowPointerTracking、バッファ型: PositionSample/PointerBuffer/ButtonBuffer/WheelBuffer、hit_test プレースホルダ2関数。本体 + in-source tests、548 LOC）
- `buffers.rs`（thread_local バッファ POINTER/BUTTON/WHEEL/DOUBLE_CLICK/MODIFIER + ヘルパ push_pointer_sample / record_button_down/up / add_wheel_* / set_modifier_state / **transfer_buffers_to_world**、232→440 LOC）
- `systems.rs`（**process_pointer_buffers** / clear_transient_pointer_state / debug_pointer_state_changes / debug_pointer_leave + 後方互換エイリアス、279→485 LOC）
- `nchittest_cache.rs`（WM_NCHITTEST キャッシュ: lookup/insert/clear/cached_nchittest、本体 + in-source tests、309 LOC）
- `dispatch/mod.rs`（Phase<T> / EventHandler 型 / OnPointer* ハンドラコンポーネント5種 / **build_bubble_path** / **dispatch_event_for_handler** / **dispatch_pointer_events**、260 LOC）
- `dispatch/tests.rs`（分離 in-source テスト、162→355 LOC）

合計 約1,831 LOC（design.md W6a 概算 1,830 と整合）。境界 = `ecs/pointer/` のみ。`ecs/drag/`（W6b）には一切触れていない。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `mod.rs` | re-export のみ | なし | — | 0件 | テスト対象なし |
| `types.rs` | `PointerBuffer::{push,latest,clear,calculate_velocity}`（MAX_SAMPLES=5 退避・速度計算・dt ガード）、`ButtonBuffer`、`WheelBuffer`（saturating_add）、`CursorVelocity::new`（magnitude）、`PointerState::default`、各 enum/marker、**`hit_test_placeholder`/`hit_test_with_local_coords`** | **なし（純粋データ/計算）** | in-source `mod tests` 13件（push/max_samples/velocity 1サンプル/button/wheel/cursor_velocity/state_default/marker/tracking/point/double_click/wheel_delta/button_enum） | **9件** | 空白: **2サンプル間の非ゼロ速度**・**dt<0.0001 ガード**・**最新2サンプルのみ採用**（既存は1サンプルの0ケースのみ）、`clear` 後の空状態、MAX_SAMPLES 退避で最新保持、**WheelBuffer の i16 飽和境界**、CursorVelocity(0,0) の magnitude、**`hit_test_placeholder`/`hit_test_with_local_coords` がゼロテストだった**（Phase 1 プレースホルダだが公開関数） |
| `buffers.rs` | `push_pointer_sample`/`record_button_down`/`record_button_up`/`add_wheel_vertical`/`add_wheel_horizontal`/`set_modifier_state`（thread_local 操作）、**`transfer_buffers_to_world`**（thread_local → World 転送・位置/速度/ボタンエッジ検出/修飾キー） | **なし（thread_local + bevy World、デバイス非依存）** | **なし（0件）** | **9件** | **最大の空白（0テスト）**: ヘルパ各種（サンプル累積・down/up 独立フラグ・ホイール独立累積・修飾キー上書き）と `transfer_buffers_to_world`（位置/速度反映・f32→i32 切り捨て・**ボタンエッジ検出**（down→true / up→false / イベントなし→維持）・転送後 ButtonBuffer reset・全5ボタン写像・修飾キー転送・PointerState 不在エンティティのスキップ）を特性化 |
| `systems.rs` | **`process_pointer_buffers`**（DOWN 優先ボタンルール・位置/ホイール/ダブルクリック/修飾キー取り込み）、`clear_transient_pointer_state`（double_click/wheel リセット + PointerLeave 除去）、`debug_pointer_state_changes`/`debug_pointer_leave`（tracing のみ） | `process_*`/`clear_*` は**デバイス非依存**だが `process_*` は thread_local 直読の未登録デッド関数（所見1/P57）。debug 系は tracing 出力のみ | **なし（0件）** | **5件** | **最大の空白（0テスト）**: `clear_transient_pointer_state`（transient リセット + PointerLeave 除去 + ボタン保持 + 空 World noop）3件。`process_pointer_buffers` の **DOWN 優先ボタンルール**（down/up/維持/同時 DOWN 優先）と位置/ホイール/ダブルクリック/修飾キー反映 + 消費（wheel reset / double_click remove）2件。後者2件は未使用関数の thread_local 直読をバッファ投入スレッドと同一スレッドで決定論的に駆動するため `ExecutorKind::SingleThreaded` を使用（本番スレッドモデルの再現ではない＝本番実行経路なし。所見1/P57） |
| `nchittest_cache.rs` | `lookup`/`insert`/`clear_nchittest_cache`（純粋キャッシュ）、`cached_nchittest`（Win32 ScreenToClient + hit_test_in_window + DragState） | `lookup`/`insert`/`clear` は**純粋**。`cached_nchittest` は**全面 Win32/COM/World 依存** | in-source `mod tests` 6件（lookup_insert/multiple_hwnds/clear/update/httransparent_storage/htclient_httransparent_coexist） | 0件 | 純粋なキャッシュ操作（座標一致判定・HWND 独立・上書き・HTTRANSPARENT 格納・共存）は既存6件で網羅済み。`cached_nchittest` は実 HWND + ScreenToClient + hit_test_in_window（layout）+ read_drag_state を要しユニット到達不能（所見2）。過不足整理: 不足なし |
| `dispatch/mod.rs` | `Phase<T>::{value,is_tunnel,is_bubble}`、`build_bubble_path`（ChildOf 走査・Window 停止）、**`dispatch_event_for_handler`**（Tunnel/Bubble 伝播・存在チェック）、**`dispatch_pointer_events`**（収集・Pressed ゲート・post-dispatch クリア） | **なし（bevy World、デバイス非依存）** | `dispatch/tests.rs` 9件（phase tunnel/bubble/clone・handler_size・bubble_path single/hierarchy・dispatch no_handlers/with_handler/stop_propagation）+ 統合 `tests/window/multiwindow_event_test.rs` で build_bubble_path の Window 境界停止3件 | **5件** | 空白: `Phase::value` の **Bubble 側**（既存は Tunnel のみ）、**Tunnel 順序が root→sender**（記録順で固定）、**OnPointerPressed ゲート**（left/right/middle のみ発火・**XButton では不発火**）、**post-dispatch ボタン/double_click クリア**（修飾キーは保持）、`dispatch_event_for_handler` の**削除済みエンティティ存在チェック**（静かに終了・パニックなし） |

追加テスト合計 **28件**（types 9・buffers 9・systems 5・dispatch 5）。**プロダクションコードの変更なし**（R5.1 充足。git diff: 762 insertions / 0 deletions、すべて `#[cfg(test)]` 内）。新規テストファイルなし（既存 in-source `mod tests` 2ファイル + 分離 `dispatch/tests.rs` への追記 + types/systems への `mod tests` 追記。うち buffers.rs / systems.rs は `mod tests` を新規作成）。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/pointer/types.rs`（in-source `mod tests`, 9件）**
- `test_velocity_calculation_two_samples_nonzero` — 2サンプル間で dx/dt・dy/dt の非ゼロ速度
- `test_velocity_calculation_tiny_dt_guards_to_zero` — 同一タイムスタンプ（dt<0.0001）でゼロ返却（発散ガード）
- `test_velocity_uses_latest_two_samples_only` — 3サンプル以上でも最新2サンプル間のみ（古い大ジャンプを無視）
- `test_pointer_buffer_clear_resets_to_empty` — clear 後 is_empty/len=0/velocity=0
- `test_pointer_buffer_eviction_keeps_newest` — MAX_SAMPLES 超過で最古退避・最新保持
- `test_wheel_buffer_saturates_at_i16_bounds` — saturating_add の i16::MAX/MIN 飽和（ラップ/panic なし）
- `test_cursor_velocity_zero_is_zero_magnitude` — (0,0) の magnitude=0・Default 全成分0
- `test_hit_test_placeholder_returns_window_entity` — Phase 1 プレースホルダが常に window_entity
- `test_hit_test_with_local_coords_passes_through_screen_coords` — ローカル=スクリーン座標の素通し

**`crates/wintf/src/ecs/pointer/buffers.rs`（in-source `mod tests`・新規 mod, 9件）**
- `test_push_pointer_sample_accumulates_in_thread_local` — サンプルが thread_local POINTER_BUFFERS へ累積・latest 反映
- `test_record_button_down_up_sets_independent_flags` — down/up が (Entity,Button) ごとに独立フラグ
- `test_add_wheel_helpers_accumulate_separately` — 垂直/水平が独立累積
- `test_set_modifier_state_overwrites_latest` — 修飾キーは最新値で上書き（累積でない）
- `test_transfer_buffers_to_world_updates_position_and_velocity` — 位置（f32→i32）・速度反映 + 転送後クリア
- `test_transfer_buffers_to_world_button_edge_detection_and_reset` — down→true / イベントなし→維持 / up→false のエッジ検出（ButtonBuffer reset 込み）
- `test_transfer_buffers_to_world_maps_all_buttons` — Left/Right/Middle/XButton1/XButton2 個別写像
- `test_transfer_buffers_to_world_applies_modifier_state` — Shift/Ctrl 転送
- `test_transfer_buffers_to_world_skips_entity_without_pointer_state` — PointerState 不在で生成せずスキップ（パニックなし）

**`crates/wintf/src/ecs/pointer/systems.rs`（in-source `mod tests`・新規 mod, 5件）**
- `test_clear_transient_resets_double_click_and_wheel` — double_click→None / wheel→default / ボタンは保持
- `test_clear_transient_removes_pointer_leave_marker` — PointerLeave 除去・PointerState 側存続
- `test_clear_transient_no_targets_is_noop` — 空 World で noop（パニックなし）
- `test_process_pointer_buffers_button_down_priority` — DOWN 優先ルール（down→true / up→false / 維持 / 同時 DOWN 優先）。`ExecutorKind::SingleThreaded` で thread_local 直読を投入スレッドと同一スレッドで決定論的に駆動（本番実行経路なしの未使用関数の characterization）
- `test_process_pointer_buffers_applies_position_wheel_doubleclick` — 位置/ホイール/ダブルクリック/修飾キー反映 + wheel reset / double_click remove の消費。同上 SingleThreaded

**`crates/wintf/src/ecs/pointer/dispatch/tests.rs`（分離 in-source, 5件）**
- `test_phase_value_on_bubble` — Phase::value が Bubble 側でもデータ返却
- `test_dispatch_tunnel_order_is_root_to_sender` — Tunnel が root→sender 順（記録順で固定）
- `test_dispatch_pressed_gating_requires_main_button` — OnPointerPressed は Left で発火・XButton1 のみでは不発火
- `test_dispatch_clears_button_state_after_dispatch` — dispatch 後に全ボタン/double_click クリア・修飾キー保持
- `test_dispatch_event_for_handler_guards_deleted_entity` — path 内削除済みエンティティで静かに終了（パニックなし・後続ハンドラ未到達）

## 除外したテスト

なし。対象モジュールの既存テスト（types in-source 13件・nchittest_cache in-source 6件・dispatch in-source 9件・統合 multiwindow_event_test の pointer 関連 build_bubble_path 3件）に重複・死テスト（到達不能・常に真・対象消失）は検出されなかった。`buffers.rs`/`systems.rs` は 0 テストだったため除外対象自体が存在しない。過不足整理の結論: **不足のみ存在（28件で充足）、過剰なし**。

## テスト不能箇所・深掘り所見（R2.8）

1. **`process_pointer_buffers` はワークスペース全域で未登録・未使用のデッド/レガシー pub 関数（→ P57）** — `process_pointer_buffers`（systems.rs:24-157）はシステム本体から thread_local バッファ（POINTER/BUTTON/WHEEL/DOUBLE_CLICK/MODIFIER）を直接読むが、**どのスケジュールにも `add_systems` で登録されておらず、本番呼び出しがゼロ**である。`world/mod.rs:114-116` に「注: process_pointer_buffersは廃止／WndProc スレッドの thread_local バッファは try_tick_world() 内の transfer_buffers_to_world() で直接 World に転送される」と明記されており、`Input` スケジュール（`world/mod.rs:74` で `Schedule::new(Input)` を挿入）には `dispatch_pointer_events`・`dispatch_drag_events` 等が登録される一方、`process_pointer_buffers` は登録されない。ワークスペース全域の grep でも `add_systems(... process_pointer_buffers ...)` は**本セル新規テスト2件（systems.rs:407, 463）のみ**で本番経路に存在しない。本番で thread_local を消費するのは `transfer_buffers_to_world`（buffers.rs:134、`try_tick_world` 冒頭の mod.rs:458 から WndProc スレッド上・`try_run_schedule(Input)` の**前**に同期呼び出し）であり、buffers.rs:129-133 のコメントがこの意図的設計（Input スケジュール実行前に WndProc スレッドで thread_local→World 転送を完了させ、マルチスレッドシステムは World 経由でアクセス）を明記している。すなわち `process_pointer_buffers` は `transfer_buffers_to_world` への移行で残ったデッドコード（および `#[deprecated]` エイリアス `process_mouse_buffers`、systems.rs:160-162）である。本セルでは現行挙動を特性化する2件を追加したが、これは **thread_local を読む当該関数をバッファ投入スレッドと同一スレッドで決定論的に駆動するための手段**として `ExecutorKind::SingleThreaded` を用いただけであり（bare `Schedule::default()` の既定 MultiThreaded エグゼキュータでは run 呼び出しスレッドと別スレッドへ退避され得て thread_local を取りこぼす）、本番スレッドモデルの忠実再現ではない（当該関数には本番実行経路が存在しない）。削除・整理は `pub` API 表面の変更を伴うため R2.9/R2.10 に従い実装せず **P57** に記録した。なお bevy 並列実行に必要な `ComputeTaskPool` は本番で初期化済み（`common/tree_system.rs:140` が transform 階層の並列伝播のため `ComputeTaskPool::get_or_init(TaskPool::default)` を呼ぶ）であり、「ComputeTaskPool 不在」前提の懸念は成り立たない。本セルで追加した buffers 9件（本番経路 `transfer_buffers_to_world` 側）・systems 5件（うち process 側 2件は未使用関数の現行挙動の characterization）が回帰検知器となる。

2. **`cached_nchittest` は全面 Win32/COM/World 依存でユニット到達不能** — `cached_nchittest`（nchittest_cache.rs:98-171）は (a) 実 HWND と `ScreenToClient`（GDI）でスクリーン→クライアント変換、(b) `ecs_world.try_borrow()` + `hit_test_in_window`（layout ドメイン、実 GlobalArrangement/HitRegion を要する）、(c) `read_drag_state`（drag ドメインの thread_local 状態）の3つの実環境依存を結合する手続きで、抽出可能な純粋計算ブロックは存在しない（キャッシュ判定の純粋部分 lookup/insert/clear は既に6件で網羅済み、DragState ガードと hit_result の OR による HTCLIENT/HTTRANSPARENT 決定は実 hit_test 結果が前提）。ユニット到達不能は環境制約でありコード改善余地ではないため提案化しない（既存の実環境統合経路 = tests/window・実起動が最終的な回帰検知器）。

3. **`debug_pointer_state_changes`/`debug_pointer_leave` は tracing 出力のみのデバッグシステム** — いずれも `Added`/`Changed` クエリを走査して `debug!`/`tracing` ログを出すのみで World を変更しない。特性化しても「パニックしない」程度のアサーションにしかならず観測価値が低い。なお `debug_pointer_state_changes` には「bevy_ecs では Added は Changed のサブセットのため移動・ボタン変化のみのログ出力には別途フラグ管理が必要だが、デバッグ用なので許容」というコメント付きの既知の限界（systems.rs:226-229）があるが、デバッグ専用かつ挙動非依存のため本 T セルでは現行コメントの事実確認に留め、提案化しない。

4. **`types.rs` の hit_test プレースホルダは Phase 1 の一時実装** — `hit_test_placeholder`/`hit_test_with_local_coords`（types.rs:354-376）は「event-hit-test 完了後に実際の実装に差し替え」とコメントされた Phase 1 スタブで、常にウィンドウエンティティ + スクリーン座標素通しを返す。公開関数だがゼロテストだったため現行（スタブ）挙動を2件で特性化した（差し替え時はこのテストが期待値更新の対象になる）。実利用箇所の有無は本 T セルの調査範囲では `pointer/` 内に見当たらず（実 hit_test は layout/hit_test/ が担当）、削除/差し替え判断は S/V セルおよび event-hit-test 仕様の進捗に委ねる（本セルからの提案化はしない）。

## proposals へ回した候補

- **P57**: `process_pointer_buffers` / `process_mouse_buffers` がワークスペース全域で未登録・未使用のデッド/レガシー `pub` 関数（`world/mod.rs:114-116` で廃止明記。本番 thread_local 消費は `transfer_buffers_to_world` に一本化済み。`ComputeTaskPool` は `tree_system.rs:140` で初期化済み）。削除または `#[deprecated]` 明示による整理候補（R2.9/R2.10）。削除時は本セル追加の process 側特性化テスト2件も対象消失で除去、transfer 側9件は本番経路の検知器として残存。あわせて transfer/process の位置・ボタン・修飾キー二重実装も削除で解消

既存提案との関連: なし（W6a は本セルが最初）。

## verification (S2)

- BEFORE: 親のベースライン（1488 passed / 0 failed・クリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。触れたバイナリ（wintf lib）の BEFORE 内訳は git diff（追加 28 件・削除 0 件）と AFTER 実測の差分から逆算して検証した。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1516 passed / 0 failed**（全テストバイナリで failed=0、awk による全 `test result` 行の合算で実測）。
  - グローバル合計は 1488 → 1516（**+28**）。追加分はすべて wintf lib の in-source（`--lib`）テスト。
  - 触れたファイルの in-source 件数内訳（git diff の `#[test]` 実数と完全一致。`git diff --unified=0 ... | grep -c "^\+.*#\[test\]"` = 28、削除 0）:
    - `types.rs`: **13 → 22（+9）**
    - `buffers.rs`: **0 → 9（+9）**
    - `systems.rs`: **0 → 5（+5）**
    - `dispatch/tests.rs`: **9 → 14（+5）**
    - `nchittest_cache.rs`: 6 → 6（+0）
    - 合計 **+28**（9+9+5+5）
  - wintf lib バイナリ全体: AFTER **311 passed**（BEFORE 283 + W6a 追加 28）。
  - 反復検証: `cargo test -p wintf --lib pointer::` で pointer モジュール **56 passed / 0 failed**（既存28 + 追加28）。内訳 `pointer::types` 22・`pointer::buffers` 9・`pointer::systems` 5・`pointer::nchittest_cache` 6・`pointer::dispatch` 14。
  - 全28件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照。ただし systems の process_pointer_buffers 2件は初回 bare `Schedule::default()` で失敗 → 既定 MultiThreaded エグゼキュータが run 呼び出しスレッドと別スレッドへシステムを退避し thread_local を取りこぼすため → `ExecutorKind::SingleThreaded` 明示で GREEN 化。これは未使用関数の thread_local 直読を投入スレッドと同一スレッドで決定論的に駆動するためのテスト技法であり、深掘りの結果プロダクションバグではなく当該関数がデッド/レガシー pub であると確定し P57 へ記録）。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W6a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` は **79 passed / 0 failed** と合格（隔離再実行不要）。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` は既存警告130件 + deny レベル error 20件を出力。**いずれも本セルの追加テスト由来ではない**。clippy 診断が参照するファイルを全列挙（`grep -oE "src[\\/]...\.rs:N"` で集計）した結果、`ecs/pointer/` 配下のファイルは**一切出現しない**（参照先は winproc.rs / win_thread_mgr.rs / window_proc/mouse_move.rs / window/window_pos.rs / drag/dispatch.rs / validate/rules.rs 等、すべて本セル境界外の既存プロダクションコード）。deny レベル error（"public function might dereference a raw pointer but is not marked unsafe" 等）も同様にポインターモジュール外の既存条件。本セルはテスト追加のみでプロダクションコード未変更のため、新規 clippy 警告/error の導入はゼロ。S3 規定によりブロッカーとせず記録に留める（S 観点の担当）。

## RED フェーズ代替の検証

追加28件はうち26件が既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した:
- **types**: `calculate_velocity` の「最新2サンプル・(dx/dt, dy/dt)・dt<0.0001 で (0,0)」（types.rs:253-267）、`push` の MAX_SAMPLES 退避（pop_front）、`WheelBuffer::add_*` の saturating_add、`CursorVelocity::new` の magnitude=sqrt(x²+y²)、プレースホルダの固定戻り値をソースから転記。
- **buffers**: `transfer_buffers_to_world` の「latest→client/local（as i32）・速度・down_received→true/up_received→false/維持・転送後 reset・修飾キー転送・get_mut None スキップ」をソース（buffers.rs:134-232）から導出。
- **systems**: `clear_transient_pointer_state` の double_click/wheel リセット + PointerLeave 除去、`process_pointer_buffers` の DOWN 優先ルール（systems.rs:42-63）と消費（wheel reset / double_click remove）をソースから導出。
- **dispatch**: `dispatch_pointer_events` の Pressed ゲート（left||right||middle、systems mod.rs:231）・post-dispatch クリア（mod.rs:243-252）、`dispatch_event_for_handler` の Tunnel=rev 順 + 存在チェック return（mod.rs:163-192）をソースから転記。

残り2件（`test_process_pointer_buffers_*`）は初回 bare `Schedule::default()` で**失敗**した（位置/ボタンが反映されず）。これは深掘りの結果**プロダクションバグではなく**、bevy の既定 MultiThreaded エグゼキュータが thread_local を持たない別スレッドへシステムを退避したためと判明。`process_pointer_buffers` は thread_local を直読する関数であり、これを**バッファ投入スレッドと同一スレッドで決定論的に駆動する**ため `ExecutorKind::SingleThreaded` を明示して GREEN 化した（本番スレッドモデルの再現ではなく、当該関数はどのスケジュールにも未登録で本番実行経路が存在しない）。さらに深掘りの結果、`process_pointer_buffers`（および `#[deprecated]` エイリアス `process_mouse_buffers`）は `world/mod.rs:114-116` で廃止が明記されワークスペース全域で本番呼び出しがゼロのデッド/レガシー `pub` 関数と確定し、削除/非推奨明示の整理候補として P57 に記録した。挙動変更は一切行っていない（プロダクションコード無変更）。
