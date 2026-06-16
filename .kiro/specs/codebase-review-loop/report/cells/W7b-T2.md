# W7b-T2: ECS基盤・World（ecs/world/ ＋ ecs/app.rs） × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W7b-T2（領域 W7b「ECS基盤・World」の**事前分割サブセル2/2** × 観点 T「テスト網羅性」）。担当は **`ecs/world/` ＋ `ecs/app.rs` のみ**。`ecs/common/` は 18.1 W7b-T1 の担当ゆえ一切触れていない。
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。`ecs/world/`（schedule labels・vsync・フレーム進行）と `ecs/app.rs`（ウィンドウカウント・ディスプレイ構成変更）のモジュール×テスト対応表をゼロから作成した。
- requirements: 1.3（大領域の細分化 = T セル事前分割の根拠）, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9（テスト命名・配置規約 = structure.md 命名規約）、レビュー観点列 T、CellExecutor 観点別規則（T）、W7b 領域定義（ECS基盤・World）と T セル事前分割、セル断片様式、提案記録様式
- 参考: `report/cells/W7b-T1.md`（W7b 姉妹セル・直前完了）・`W6b-T.md`（in-source `mod tests` パターン）・`W7a-T1.md`（事前分割 T セル様式）・`taffy_flex_layout_pure_test.rs`（既存の `EcsWorld::new()`+`try_tick_world()` 駆動の統合テスト）

## 対象ファイル一覧（W7b-T2 = `crates/wintf/src/ecs/world/` ＋ `ecs/app.rs`）

- `world/mod.rs`（**`EcsWorld`**: world/has_systems/message_window/frame_count/last_log_time を保持。`new`（リソース初期化 + 13 スケジュール登録 + デフォルトシステム登録）/`set_message_window`/`message_window`/`schedules_mut`/`add_systems`/`world`/`world_mut`/`spawn`/`measure_and_log_framerate`（private）/`try_tick_world`（FrameCount++ ・FrameTime 更新・13 スケジュール順次実行）/`try_tick_on_vsync`（VSYNC カウンター比較）/`Default`/`Debug`、526 LOC）
- `world/schedule_labels.rs`（`FrameCount`（Resource・Default）、13 個の `ScheduleLabel` マーカー構造体: `Input`/`Update`/`PreLayout`/`Layout`/`PostLayout`/`UISetup`/`GraphicsSetup`/`Draw`/`PreRenderSurface`/`RenderSurface`/`Composition`/`CommitComposition`/`FrameFinalize`、110 LOC）
- `world/vsync.rs`（`VsyncTick` トレイト（`Rc<RefCell<EcsWorld>>` 実装）、`IS_TICK_FLUSH_IN_PROGRESS`（thread_local 再入防止）、`TickFlushGuard`（RAII）、102 LOC）
- `app.rs`（**`App`**（Resource）: `window_count`/`message_window`/`display_configuration_changed` を private 保持。`new`/`Default`/`set_message_window`/`mark_display_change`/`reset_display_change`/`display_configuration_changed`/`on_window_created`（カウント++）/`on_window_destroyed`（saturating_sub・最後のウィンドウで PostMessageW + true）/`window_count`、98 LOC）

合計 約 836 LOC。境界 = `ecs/world/` ＋ `ecs/app.rs` のみ。**`ecs/common/` は未参照**（W7b-T1 担当）。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス/実時間依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `app.rs` → `display_configuration_changed` 系 | `mark_display_change`/`reset_display_change`/`display_configuration_changed`（フラグ set/reset/get） | **なし（純粋 bool フラグ）** | **`tests/window/monitor_hierarchy_test.rs::test_display_configuration_changed_flag` 1件（mark→true→reset→false の往復を直接検証）** + `test_monitor_update_on_change` が `mark_display_change`/`display_configuration_changed` を system 経由で間接駆動 | 0件 | **既存で網羅済み**。ディスプレイ構成変更フラグの set/reset/get 契約は monitor_hierarchy_test（W4b/W7a ドメインで配置済み）が直接特性化しており、重複回避のため app.rs 側に追加しない |
| `app.rs` → ウィンドウカウント系 | `Default`/`new`（count=0）、`on_window_created`（+1）、`on_window_destroyed`（saturating_sub・count==0 で true・PostMessageW）、`window_count`（getter） | **`message_window=None`（デフォルト）の経路は純粋・デバイス非依存**。`message_window=Some` の `PostMessageW`（最後のウィンドウ破棄時）のみ Win32 依存（所見1） | **なし（0件・最大の空白）** | **6件**（in-source `mod tests`） | **空白（0テスト）**: `Default`/`new` の初期 count=0、`on_window_created` の単調増加、`on_window_destroyed` の残あり false / 最後 true、**saturating_sub による count=0 アンダーフロー防止**（0 から破棄で 0 に留まり true）、作成・破棄混在シーケンスの累積。`message_window=None` のため最後破棄時の PostMessageW 分岐は不発（テスト中に Win32 副作用なし） |
| `world/schedule_labels.rs` → `FrameCount` | `Default`（0）、タプル素通し格納 | **なし（純粋 Resource）** | **なし（0件・直接特性化なし）**。多数の統合テストが `FrameCount(0)`/`FrameCount::default()` を**前提リソースとして挿入**するが値そのものは検証しない | **2件** | **空白（直接 0件）**: `Default`=0（フレーム未経過初期値）、内部 u32 の素通し格納（42/u32::MAX） |
| `world/schedule_labels.rs` → 13 ScheduleLabel | `Input`〜`FrameFinalize`（`#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]`） | **なし（純粋ユニット構造体）** | **なし（0件）**。`pub use` で `wintf::ecs::{Input,...}` として露出し本番 `Schedules` のキーとして使われるが derive 契約は未固定 | **5件** | **空白（0テスト）**: 各バリアントの自己等価（PartialEq/Eq）、Clone 等価保存、Eq↔Hash 整合（HashMap キー要件）、Debug の型名整形（13 ラベル全列挙）、`Box<dyn ScheduleLabel>` での型区別（Input≠Update・同一バリアント等価）。これらは `Schedules` がラベルを intern/同定する根拠 |
| `world/mod.rs` → `EcsWorld` | `new`（13 スケジュール登録 + FrameCount=0）、`try_tick_world`（FrameCount++・13 スケジュール順次実行）、`set_message_window`/`message_window`、`Default`、`Debug`、UISetup の SingleThreaded 設定 | **`new`/`try_tick_world`（Window 非保持時）/`set_message_window`/`Default`/`Debug` はデバイス非依存**（既存 `taffy_flex_layout_pure_test` が同様に `try_tick_world` を駆動。Window/WindowHandle 不在で graphics 系は no-op）。`try_tick_world` の実 graphics 経路（実 Window/HWND/DComp）は device 依存（所見2） | **なし（0件・直接特性化なし）**。`taffy_flex_layout_pure_test`/`taffy_layout_integration_test` が `EcsWorld::new()`+`try_tick_world()` を**レイアウト検証の土台として**駆動するが、FrameCount 進行・スケジュール登録・message_window・Default 等価は未検証 | **8件**（新規統合 `tests/ecs/world_lifecycle_test.rs`） | **空白（直接 0件）**: 新規 World の FrameCount=0、`try_tick_world` 毎回 +1 の累積（フレーム進行カウント）、13 スケジュールラベル全登録、UISetup のみ SingleThreaded（他は非 SingleThreaded）、初期 message_window=None、`set_message_window` の World/App 双方反映、`Default`==`new` 初期状態、Debug 非網羅整形 |
| `world/mod.rs` → `try_tick_on_vsync` | VSYNC_TICK_COUNT/LAST_VSYNC_TICK 比較・LAST 更新・`try_tick_world` 委譲 | **実時間/vsync 依存**（`win_thread_mgr` の `pub(crate)` プロセスグローバル atomic を読む。実 ~16ms vsync スレッドが駆動）（所見3） | なし | 0件 | テスト不能（所見3）。atomic が `pub(crate)` ゆえ統合テストから操作不能・プロセスグローバルゆえ並列テストで非決定 |
| `world/vsync.rs` → `VsyncTick`/再入ガード | `try_tick_on_vsync`（`IS_TICK_FLUSH_IN_PROGRESS` 再入ブロック・`try_borrow_mut` 失敗スキップ・`flush_window_pos_commands` 呼出）、`TickFlushGuard`（Drop） | **実 vsync + 実 Win32（SetWindowPos/WM_WINDOWPOSCHANGED 同期発火）依存**（所見3） | なし | 0件 | テスト不能（所見3）。`Rc<RefCell<EcsWorld>>` 実装で WndProc モーダルループ経路を要し、再入は実 `flush_window_pos_commands`→実 SetWindowPos→同期 WM_WINDOWPOSCHANGED 経由でのみ発生 |

追加テスト合計 **21件**（app **6件** + schedule_labels **7件** の **in-source 13件** + EcsWorld 統合 **8件**）。**プロダクションコードの変更なし**（R5.1 充足。git diff: in-source 追加 `#[test]`=13・削除0、すべて `#[cfg(test)]` 内。新規統合ファイル 1件=8。`tests/ecs.rs` への mod 宣言1行追記）。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/app.rs`（in-source `mod tests`・新規, 6件）**
- `test_default_and_new_start_with_zero_windows_and_no_display_change` — `Default`/`new` の window_count=0・display フラグ false
- `test_on_window_created_increments_window_count` — 呼び出しごとに +1（1→2→3）
- `test_on_window_destroyed_decrements_and_returns_false_while_windows_remain` — 残ありで -1・false（最後でない）
- `test_on_window_destroyed_returns_true_when_last_window_closed` — 最後の1つ破棄で count=0・true
- `test_on_window_destroyed_saturates_at_zero_and_returns_true` — count=0 からの破棄で saturating_sub によりアンダーフローせず 0 維持・true
- `test_window_count_tracks_mixed_create_destroy_sequence` — 作成3→破棄→再作成→破棄連続→最後の累積増減と 0 到達 true

**`crates/wintf/src/ecs/world/schedule_labels.rs`（in-source `mod tests`・新規, 7件）**
- `test_frame_count_default_is_zero` — `FrameCount::default()`=0
- `test_frame_count_stores_inner_value` — 内部 u32 素通し（42・u32::MAX）
- `test_schedule_labels_equal_themselves` — 13 ラベル各自の PartialEq 自己等価
- `test_schedule_labels_clone_preserves_equality` — Clone が等価値を返す
- `test_schedule_labels_hash_is_consistent_with_eq` — 同一バリアントの Hash 一致（Eq↔Hash 整合）
- `test_schedule_labels_debug_contains_type_name` — 13 ラベルの Debug 整形（型名）
- `test_distinct_schedule_labels_are_not_equal_as_dyn` — `Box<dyn ScheduleLabel>` で Input≠Update・同一バリアント等価（Schedules の型区別根拠）

**`crates/wintf/tests/ecs/world_lifecycle_test.rs`（新規, 8件）**
- `new_ecs_world_starts_frame_count_at_zero` — 新規 World の FrameCount=0
- `try_tick_world_increments_frame_count_each_call` — 1 tick→1・3 tick→3（毎 tick +1 のフレーム進行カウント）
- `new_ecs_world_registers_all_schedule_labels` — `Schedules::contains` で 13 ラベル全登録
- `uisetup_schedule_uses_single_threaded_executor` — UISetup のみ `ExecutorKind::SingleThreaded`（Update は非 SingleThreaded）
- `new_ecs_world_has_no_message_window` — 初期 message_window=None
- `set_message_window_stores_hwnd_in_world_and_app` — ダミー HWND を World/App 双方へ反映（Win32 非呼出の格納のみ）
- `default_matches_new_initial_state` — `Default`==`new`（FrameCount=0・スケジュール登録・message_window None）
- `ecs_world_debug_is_non_exhaustive` — Debug 出力が型名 "EcsWorld" を含む

## 除外したテスト
なし。`ecs/world/` 配下に既存 in-source テストは存在しなかった（除外対象自体なし）。`app.rs` の display フラグ系は `monitor_hierarchy_test.rs` で既に直接特性化されており（重複回避のため追加せず）、これは死テスト・到達不能テストではない（mark→reset の往復を異なるドメイン文脈で固定する有効なテスト）。統合側の `taffy_flex_layout_pure_test`/`taffy_layout_integration_test` は `EcsWorld` をレイアウト検証の土台として駆動する別観点で、本セルの EcsWorld 契約特性化（FrameCount 進行・スケジュール登録・message_window・Default）とは非重複（触れていない）。過不足整理の結論: **不足のみ存在（21件で充足）、過剰なし**。

**重複の意図的回避**: (1) `app.rs` の `mark_display_change`/`reset_display_change`/`display_configuration_changed` は `monitor_hierarchy_test.rs::test_display_configuration_changed_flag`（W4b/W7a で配置済み）が mark→true→reset→false を直接アサートしており、本セルでは重複を避けて未テストだった**ウィンドウカウント系**（on_window_created/on_window_destroyed/saturating_sub/window_count）のみを追加した。(2) `FrameCount` は多数の統合テストが前提リソースとして挿入するが**値を検証しない**ため、`Default`=0 と内部格納の直接単体テストは空白であり追加した。(3) `EcsWorld::new()`+`try_tick_world()` は既存統合テストが駆動するが**FrameCount 進行・スケジュール登録の直接検証**は空白だったため追加した。

## vsync/実時間依存で未テストの箇所・深掘り所見（R2.8）

1. **`app.rs::on_window_destroyed` の最後のウィンドウ破棄時 `PostMessageW` は Win32 依存だが `message_window=None` で回避済み** — `on_window_destroyed` は `window_count` が 0 に達したとき `message_window`（`Option<isize>`）が `Some` なら `PostMessageW(hwnd, WM_LAST_WINDOW_DESTROYED, ...)`（app.rs:77-86）を実 HWND へ送信する。デフォルト（`set_message_window` 未呼出）では `message_window=None` のため `if let Some(hwnd_raw)` 分岐は不発で、カウント減算と戻り値（count==0 で true）のみが純粋に観測できる。本セルの 6 件はこの None 経路でカウント増減・saturating_sub・戻り値を全面特性化した。`message_window=Some` での実 PostMessageW 副作用（実メッセージウィンドウへの WM_LAST_WINDOW_DESTROYED 配信 → `winproc.rs:91` の PostQuitMessage 経由のアプリ終了）は実メッセージループ + 実 HWND を要し、`win_thread_mgr` の実起動経路（S7）が回帰検知器。`set_message_window` は private 化された `message_window: Option<isize>` を設定するのみ（Win32 非呼出）で、その格納は EcsWorld 統合テスト（`set_message_window_stores_hwnd_in_world_and_app`）が App リソース経路で固定した。環境制約のため提案化しない。

2. **`world/mod.rs::try_tick_world` の実 graphics 経路（実 Window/HWND/DComp/D2D1）は device 依存だが Window 非保持で回避済み** — `try_tick_world`（mod.rs:436-479）は FrameCount++・FrameTime 更新・`transfer_buffers_to_world`・13 スケジュールの `try_run_schedule` 順次実行・NCHITTEST キャッシュクリアを行う。13 スケジュールには `init_graphics_core`/`init_window_graphics`/`compositor_init_system`/`create_windows`/各種 draw/composite システムが含まれるが、これらは `Window`/`WindowHandle`/`HasGraphicsResources` を持つエンティティに対してのみ実 device 呼出を行うクエリ駆動システムであり、それらを spawn しない World では空クエリで no-op となる（既存 `taffy_flex_layout_pure_test` が Window なしで `try_tick_world` を 2 回駆動して成立する事実が実証）。本セルは Window 非保持 World で FrameCount 進行（フレーム進行カウント = `frame_count.0 += 1`、mod.rs:446-448）とスケジュール順次実行の成立を特性化した。実 Window を spawn した場合の `create_windows`（CreateWindowExW）・graphics 初期化・DComp commit の実 device 副作用は実起動 S7 が回帰検知器。`measure_and_log_framerate`（private・mod.rs:409-432）は `Instant::now()` ベースの 10 秒ごと FPS ログで、実時間 10 秒経過を要するため単体検証不能だが、tracing ログ出力のみで観測可能挙動に影響せず（frame_count フィールドのリセットは内部状態）提案化しない。環境制約のため提案化しない。

3. **`world/mod.rs::try_tick_on_vsync` と `world/vsync.rs` の `VsyncTick` 全体は実 vsync/実時間/実 Win32 依存でテスト不能** — `try_tick_on_vsync`（mod.rs:493-512）は `win_thread_mgr::{VSYNC_TICK_COUNT, LAST_VSYNC_TICK}`（`pub(crate)` の `AtomicU64`・win_thread_mgr.rs:38/42）を比較し、変化があれば `LAST_VSYNC_TICK` を更新して `try_tick_world` を委譲する。この atomic は (a) `pub(crate)` ゆえ crate 外の統合テストから読み書き不能、(b) プロセスグローバルかつ実 vsync スレッド（win_thread_mgr.rs:359 で ~16ms ごとに `fetch_add`）が駆動するため、複数テスト並列実行で非決定的に進行する。`world/vsync.rs` の `VsyncTick for Rc<RefCell<EcsWorld>>` 実装（vsync.rs:51-101）はさらに `IS_TICK_FLUSH_IN_PROGRESS`（thread_local 再入ガード）・`try_borrow_mut` の借用失敗スキップ・`flush_window_pos_commands()`（実 SetWindowPos 経由で同期 WM_WINDOWPOSCHANGED を発火しうる）を含み、再入経路（tick→flush→SetWindowPos→WM_WINDOWPOSCHANGED→再 tick）の発生には実 Win32 メッセージループ + 実モーダルドラッグが必要で、ユニットでは決定的に再現できない。これは VSYNC 優先レンダリング + モーダルループ中描画継続という実時間 GUI 機能の中核であり、実起動 S7（実 vsync 駆動・実ウィンドウドラッグ）が唯一の回帰検知器。実時間/vsync/実 Win32 依存の三重制約のため提案化しない（テスト用に atomic を pub 化したり vsync を注入する構造変更は観測可能挙動への影響リスクがあり、判断に迷う構造変更として見送り）。

## proposals へ回した候補
なし（新規採番なし）。`ecs/world/` ＋ `ecs/app.rs` のテスト可能ロジック（ウィンドウカウント増減・saturating_sub アンダーフロー防止・スケジュールラベル derive 契約・FrameCount 進行・スケジュール登録・message_window 反映・Default 等価）は **挙動変更を要する欠陥・脆弱性・削除候補を検出せず**、デバイス非依存に高くテスト可能であった（21件すべて初回 GREEN・バグ検出ゼロ）。所見1〜3 はいずれも環境（実 Win32/実 vsync/実時間）制約または既存テストでの回避で、新規仕様化を要しない。proposals.md 末尾は **P65**（変更なし。次セルの新規採番は P66 から）。

なお clippy（後述）で `app.rs:18` の `impl Default for App` に既存の `derivable_impls` 警告（`#[derive(Default)]` で代替可能）があるが、これは**プロダクションコードの既存診断**であり本セルのテスト追加とは無関係。挙動非破壊な簡素化候補だが S 観点（W7b-S）の担当ゆえ本 T セルでは実施せず、W7b-S への申し送りとする（提案化は不要 = clippy が機械検出する標準診断）。

## verification (S2)

- BEFORE: 親のベースライン（**1646 passed / 0 failed**・W7b-T1 完了時点のクリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。触れる対象の BEFORE は改善前 grep で確認: `ecs/world/`（in-source テスト 0件）・`ecs/app.rs`（in-source テスト 0件）・`tests/ecs` バイナリ（既存 93件、うち world 関連 0件）。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1667 passed / 0 failed**（32 ignored）。全テストバイナリで failed=0、全 `test result:` 行を awk で合算した実測（`passed=1667 failed=0 ignored=32`）。`test result: FAILED` 行ゼロ・`error[`/`panicked` 行ゼロ。
  - グローバル合計は 1646 → 1667（**+21**）。
  - 触れたファイルの新規 `#[test]` 件数内訳（git diff 実数と完全一致。`git diff --unified=0 -- crates/wintf/src/ecs/app.rs | grep -c "^+.*#\[test\]"`=6・`... schedule_labels.rs ...`=7・削除0。新規 `tests/ecs/world_lifecycle_test.rs` の `#[test]`=8）:
    - `app.rs`（in-source）: **0 → 6（+6）**
    - `schedule_labels.rs`（in-source）: **0 → 7（+7）**
    - `world/mod.rs`・`world/vsync.rs`: **0 → 0（変更なし。EcsWorld は統合テストで特性化・vsync はテスト不能）**
    - `tests/ecs/world_lifecycle_test.rs`（新規統合）: **+8**
    - 合計 **+21**（6+7+8）
  - 反復検証: `cargo test -p wintf --lib "ecs::app::tests"` で **6 passed / 0 failed**。`cargo test -p wintf --lib "world::schedule_labels::tests"` で **7 passed / 0 failed**。`cargo test -p wintf --test ecs world_lifecycle_test` で **8 passed / 0 failed**。`tests/ecs` バイナリ全体は **101 passed / 0 failed**（既存93 + 追加8）。wintf lib in-source 全体は **431 passed**（既存418 + 追加13）。
  - 全21件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。深掘りを要する初回失敗なし（バグ・前提誤りの検出なし）。

## flaky
- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（`tests/ecs` バイナリ内・W7b-T2 の追加対象外）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリは **101 passed / 0 failed**（`bench_pop_ready_empty_queue` 含め全 `... ok`、隔離再実行不要）。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）
- `cargo clippy --workspace` は既存警告群（合計 157 warning 系）を出力。
  - **本セルで追加/変更したファイルの `mod tests`（`app.rs` の `#[cfg(test)]` ブロック・`schedule_labels.rs` の `#[cfg(test)]` ブロック）および新規 `tests/ecs/world_lifecycle_test.rs` を指す診断はゼロ**。
  - 唯一 `app.rs:18` を指す診断は **`impl Default for App`（プロダクションコード・本セル以前から存在）の `clippy::derivable_impls`**（`#[derive(Default)]` で代替可能）であり、本セルで追加したテストコード（app.rs:99 以降の `mod tests`）由来ではない。S3 規定により記録のみ・非ブロッカー。挙動非破壊な簡素化候補だが S 観点 W7b-S の担当として申し送り。
  - 本セルはテスト追加のみでプロダクションコード未変更のため、**新規 clippy 警告/error の導入はゼロ**。

## RED フェーズ代替の検証
追加21件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した:
- **app.rs**: `on_window_created` の `self.window_count += 1`（app.rs:56-63）、`on_window_destroyed` の `self.window_count = self.window_count.saturating_sub(1)`（:67-91）と `if self.window_count == 0 { ... true } else { false }`、`Default` の全フィールド初期値（:18-26、count=0）から、カウント増減・saturating_sub アンダーフロー防止・最後のウィンドウ true 判定を導出。`message_window=None` で PostMessageW 分岐が不発であることはソース（:77 の `if let Some(hwnd_raw)`）から導出。
- **schedule_labels.rs**: `FrameCount(pub u32)` の `Default`（derive、=0）と内部格納（:7-8）、各 ScheduleLabel の `#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]`（:17-109）から、derive された等価/Clone/Hash/Debug 契約と `Box<dyn ScheduleLabel>` の dyn 比較（bevy_ecs の intern_label 用）を導出。Debug 整形（ユニット構造体の型名）は標準 derive 仕様から導出。
- **world/mod.rs (EcsWorld)**: `new` の `world.insert_resource(FrameCount::default())`（:39）と 13 スケジュールの `schedules.insert(Schedule::new(...))`（:74-93）、UISetup のみ `sc.set_executor_kind(ExecutorKind::SingleThreaded)`（:81-85）、`try_tick_world` の `frame_count.0 += 1`（:446-448）と 13 `try_run_schedule`（:461-473）、`set_message_window` の `self.message_window = Some(hwnd)` + `app.set_message_window(hwnd)`（:333-339）、`Default` の `Self::new()`（:515-519）、`Debug` の `finish_non_exhaustive`（:521-525）から、FrameCount 初期値/進行・スケジュール登録・SingleThreaded 設定・message_window 反映・Default 等価・Debug 整形を導出。`Schedules::contains`/`get`/`get_executor_kind` は bevy_ecs 0.18.0 の public API。Window 非保持時の `try_tick_world` 無副作用は既存 `taffy_flex_layout_pure_test` の成立事実から導出。

初回実行で21件全件が導出どおり一致し、バグ・前提誤りは検出されなかった（深掘りを要する初回失敗なし）。
