# W7a-T1: wintf ウィンドウ管理（ecs/window/） × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W7a-T1（領域 W7a「wintf ウィンドウ・メッセージ」の**事前分割サブセル1/2** × 観点 T「テスト網羅性」）。担当は **`ecs/window/` のみ**。`ecs/window_proc/` は 17.2 W7a-T2 の担当ゆえ一切触れていない。
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。**W7a-T1 の先行断片なし**。`ecs/window/` のモジュール×テスト対応表をゼロから作成した。
- requirements: 1.3（大領域の細分化 = T セル事前分割の根拠）, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9（テスト配置規約 = structure.md 命名規約）、レビュー観点列 T、CellExecutor 観点別規則（T）、W7a 領域定義（`crates/wintf/src/ecs/window/`, `ecs/window_proc/` / 2,630 LOC / window/ 未テスト・unsafe あり）と T セル事前分割、セル断片様式、提案記録様式
- 参考: `report/cells/W6b-T.md`（直前のドラッグ T セル。in-source `mod tests` をデバイス非依存ソースへ追加するパターン）・`W2-T.md`/`W5a-T.md`（Win32/COM/GUI 依存域 T セルの所見・提案の書き方）

## 対象ファイル一覧（W7a-T1 = `crates/wintf/src/ecs/window/`）

- `mod.rs`（re-export のみ、14 LOC）
- `components.rs`（`DpiChangeContext`(new/set/take・thread_local)、`CompositionMode`(enum/Default)、`Window`(Default/composition_mode・on_window_add フック)、`WindowStyle`(Default/from_hwnd)、230 LOC）
- `dpi.rs`（**`DPI` コンポーネント**: Default(96)/from_dpi/`from_WM_DPICHANGED`(WPARAM ビット解析)/scale_x・scale_y/to_logical_*/to_physical_*、135→約 230 LOC）
- `window_pos.rs`（`ZOrder`(enum/Default)、**`WindowPos`**(Default/new/builder×多数/`build_flags`(bool→SWP フラグ自動判定)/`get_hwnd_insert_after`(ZOrder 写像)/set_window_pos/to_window_coords/`to_window_coords_for_creation`(CW_USEDEFAULT 素通し))、`SetWindowParentToLayoutRoot`(Command)、440→約 690 LOC）
- `command.rs`（`is_self_initiated`/`SetWindowPosGuard`(RAII)/`guarded_set_window_pos`、**`SetWindowPosCommand`**(new/enqueue/flush・thread_local キュー)、`flush_window_pos_commands`、**`find_owner_window`**(World ベース祖先探索)、245 LOC）
- `window_handle.rs`（`WindowHandle`(get_dpi/get_style/client_to_window_rect/window_to_client_rect/client_to_window_coords/window_to_client_coords)、on_window_handle_add/remove フック、275 LOC）
- `window_system.rs`（`create_windows` 排他システム: CreateWindowExW、176 LOC）
- `monitor.rs`（`Monitor`(from_hmonitor/physical_size/top_left・Debug/PartialEq・on_monitor_add フック)、`MonitorError`(Display)、`enumerate_monitors`、202 LOC）

合計 約 1,717 LOC（design.md W7a 概算 2,630 のうち window/ 分。残差 ≈ 913 LOC が window_proc/ = W7a-T2 担当）。境界 = `ecs/window/` のみ。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `mod.rs` | re-export のみ | なし | — | 0件 | テスト対象なし |
| `components.rs` | `Window::default`（title/parent/mode）、`WindowStyle::default`（WS_POPUP\|WS_VISIBLE / WS_EX_LAYERED）・PartialEq、`CompositionMode`、`DpiChangeContext`(new/set/take) | **`Window::default`/`WindowStyle::default`/`DpiChangeContext` は純粋（thread_local 含むがデバイス非依存）**。`WindowStyle::from_hwnd`（GetWindowLongPtrW）・`on_window_add` フック（GetDpiForSystem・Visual/WindowPos 自動挿入）は Win32/ECS 依存 | `tests/window/composition_mode_test.rs` 5件（**`CompositionMode` のみ**: default=ULW・Window.composition_mode()・DComp setter・Clone/Eq・Debug） | **8件** | 空白: `Window::default` の title="Window"/parent=None（mode 以外）、`WindowStyle::default` の全ビット（WS_POPUP\|WS_VISIBLE/WS_EX_LAYERED・非 OVERLAPPEDWINDOW・非 WS_CAPTION）・PartialEq、`DpiChangeContext` の new/set→take 消費/未設定 None が**完全に未テストだった**。`on_window_add` フックは実 GetDpiForSystem + DeferredWorld を要し対象外（所見1） |
| `dpi.rs` | `Default`(96=100%)、`from_dpi`(軸独立)、**`from_WM_DPICHANGED`**(LOWORD=X/HIWORD=Y のビット解析)、`scale_x/scale_y`、`to_logical_{x,y,size,point}`、`to_physical_{x,y,size,point}`(round) | **なし（純粋算術）** | doctest 3件（`from_dpi(120).scale_x()==1.25`・`to_logical_x(200)@192dpi==100`・`to_physical_x(100)@192dpi==200` の最小スモーク） | **9件** | 空白: `Default`=96/scale=1.0、`from_dpi` 軸独立(120/144)、**`from_WM_DPICHANGED` の WPARAM ビット解析**（LOWORD/HIWORD 分離・上位ビットマスク）、scale@192=2x、to_logical の y/size/point、to_physical の y/size/point、**`.round()` の half-away-from-zero**（1.5→2 / 4.5→5 が偶数丸めでないこと）、100% 恒等。doctest はスモーク3件のみで y 軸・WPARAM 解析・丸め境界は未固定だった |
| `window_pos.rs` | `ZOrder`(Default=NoChange)、`WindowPos`(Default=CW_USEDEFAULT・new・builder 全 setter)、**`build_flags`**(position/size None→NOMOVE/NOSIZE・NoChange→NOZORDER・10 bool→各 SWP)、**`get_hwnd_insert_after`**(ZOrder 6 バリアント写像)、`to_window_coords_for_creation`(CW_USEDEFAULT 素通し・無フレームスタイル恒等) | **`build_flags`/`get_hwnd_insert_after`/Default/builder/ZOrder は純粋**。`to_window_coords_for_creation` の CW_USEDEFAULT 分岐は純粋（API 非呼出）。`set_window_pos`/`to_window_coords`（実 SetWindowPos/AdjustWindowRectExForDpi on 実 HWND）は Win32 依存（所見2） | **なし（0件）** | **15件** | **最大の空白（0テスト）**: `build_flags`（自動判定 NOMOVE/NOSIZE/NOZORDER + 10 bool→SWP 全写像）と `get_hwnd_insert_after`（6 バリアント→HWND_TOPMOST/NOTOPMOST/TOP/BOTTOM/InsertAfter/None）は本番 `apply_window_pos_changes` が依存する中核ロジックだが完全未テストだった。`to_window_coords_for_creation` は CW_USEDEFAULT 素通し（API 非呼出）と WS_POPUP/ex=0 で AdjustWindowRectExForDpi の調整量0＝座標恒等を特性化 |
| `command.rs` | `SetWindowPosCommand`(new 全フィールド格納・None insert_after)、`is_self_initiated`(ネストカウンタ 0→false)、`flush`(空キュー no-op)、`find_owner_window`(World 祖先探索) | **`SetWindowPosCommand::new`/`is_self_initiated`(rest)/空 flush は純粋/デバイス非依存**。`find_owner_window` は **World ベースで完全にデバイス非依存**。`guarded_set_window_pos`/非空 flush（実 SetWindowPos）は Win32 依存（所見3） | `tests/window/multiwindow_event_test.rs` で **`find_owner_window` 3件**（basic 多重ウィンドウ祖先探索・no_window・isolated）＋ build_bubble_path/mouseleave/drag-guard で間接利用 | **4件** | 空白: `SetWindowPosCommand::new` のフィールド格納（hwnd/x/y/w/h/flags/insert_after）・None 許容、`is_self_initiated` のスコープ外 false、空キュー flush の no-op（early-return）が未固定だった。`find_owner_window` は既存3件で網羅済み → 追加なし。**enqueue 内容を観測する API がない**ため非空 flush 経路はキュー内容アサーション不能（所見3・P1 と同根） |
| `window_handle.rs` | `WindowHandle`(get_dpi/get_style/client_to_window_rect/window_to_client_rect/client_to_window_coords/window_to_client_coords)、on_window_handle_add/remove | **全面 Win32（実 HWND 必須）** | なし（実起動の S7・統合経路のみ） | 0件 | 純粋ロジックの抽出可能箇所なし。全メソッドが GetDpiForWindow/GetWindowLongPtrW/AdjustWindowRectExForDpi を実 HWND に対して呼ぶ。所見4 |
| `window_system.rs` | `create_windows`（排他システム: CreateWindowExW・ShowWindow） | **全面 Win32（実ウィンドウ作成）** | なし | 0件 | 純粋ロジックなし。SystemState クエリ→CreateWindowExW→WindowHandle 挿入の手続きで、座標導出は `to_window_coords_for_creation`（window_pos.rs で特性化済み）に委譲。所見5 |
| `monitor.rs` | `Monitor`(physical_size/top_left・**PartialEq=handle のみ**・Debug 整形)、`MonitorError`(Display)、from_hmonitor/enumerate_monitors | **`physical_size`/`top_left`/`PartialEq`/`Debug`/`MonitorError::Display` は純粋（合成 Monitor で検証可）**。`from_hmonitor`(GetMonitorInfoW/GetDpiForMonitor)・`enumerate_monitors`(EnumDisplayMonitors) は Win32 依存（所見6） | `tests/window/monitor_hierarchy_test.rs` で合成 Monitor（`make_test_monitor`）を使い physical_size/top_left を update_monitor_layout_system 経由で間接検証 + Monitor 階層（W4b-T 追加分含む） | **7件** | 空白: `physical_size`/`top_left` の**直接**検証（負原点セカンダリモニタ含む）、**`PartialEq` が handle のみ比較**（bounds/dpi/is_primary 相違でも等価）の特性化、`Debug` の bounds カスタム整形 "(l,t,r,b)"、`MonitorError::Display` の2文言が未固定だった。`from_hmonitor`/`enumerate_monitors` は実 HMONITOR 必須で対象外（所見6） |

追加テスト合計 **43件**（dpi 9・window_pos 15・command 4・components 8・monitor 7、すべて **in-source `mod tests`**）。**プロダクションコードの変更なし**（R5.1 充足。git diff: 追加 `#[test]` = 43・削除 0、すべて `#[cfg(test)]` 内）。新規テストファイルなし（5ソースファイルへ `mod tests` を新規作成）。統合テスト側（`tests/window/`）への追加・変更なし。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/window/dpi.rs`（in-source `mod tests`・新規, 9件）**
- `test_default_is_96_dpi` — Default=96/96・scale=1.0/1.0
- `test_from_dpi_stores_axes_independently` — from_dpi(120,144) で X/Y 独立保持・scale 個別
- `test_from_wm_dpichanged_parses_loword_and_hiword` — WPARAM 0x00C00078 → X=120/Y=192（LOWORD=X/HIWORD=Y）
- `test_from_wm_dpichanged_masks_upper_bits` — 上位32bit を含む値で下位16bitのみ採用（マスク）
- `test_scale_at_192_dpi_is_2x` — 192 DPI=200%
- `test_to_logical_y_and_size_and_point` — y/size/point の論理変換（192 DPI）
- `test_to_physical_y_and_size_and_point` — y/size/point の物理変換（192 DPI）
- `test_to_physical_rounds_half_away_from_zero` — 1.5→2・4.5→5（`.round()` が偶数丸めでないこと）
- `test_logical_physical_at_100_percent_is_identity` — 96 DPI で恒等

**`crates/wintf/src/ecs/window/window_pos.rs`（in-source `mod tests`・新規, 15件）**
- `test_zorder_default_is_no_change` — ZOrder::default=NoChange
- `test_default_uses_cw_usedefault_position_and_size` — Default の position/size=CW_USEDEFAULT・全 bool false
- `test_new_equals_default` — new()==default()
- `test_builder_with_position_and_size` — with_position/with_size
- `test_builder_zorder_helpers` — zorder_* 6種 + with_zorder
- `test_builder_bool_flag_setters` — 10 bool setter の反映
- `test_build_flags_default_sets_nozorder_only_when_pos_size_present` — 既定（pos/size Some・NoChange）で NOZORDER のみ
- `test_build_flags_position_none_sets_nomove` — position None→SWP_NOMOVE
- `test_build_flags_size_none_sets_nosize` — size None→SWP_NOSIZE
- `test_build_flags_nonzero_zorder_clears_nozorder` — NoChange 以外で NOZORDER 不立
- `test_build_flags_maps_each_bool_to_swp_flag` — 10 bool→各 SWP フラグ全写像
- `test_get_hwnd_insert_after_maps_each_zorder` — ZOrder 6 バリアント→HWND_*/None
- `test_to_window_coords_for_creation_passes_through_cw_usedefault` — 既定 CW_USEDEFAULT で API 非呼出素通し
- `test_to_window_coords_for_creation_passes_through_when_position_x_is_cw_usedefault` — position.x のみ CW_USEDEFAULT でも素通し
- `test_to_window_coords_for_creation_no_frame_style_is_identity` — WS_POPUP/ex=0 で AdjustWindowRectExForDpi 調整量0＝座標恒等

**`crates/wintf/src/ecs/window/command.rs`（in-source `mod tests`・新規, 4件）**
- `test_set_window_pos_command_new_stores_all_fields` — new の全フィールド格納
- `test_set_window_pos_command_new_allows_none_insert_after` — insert_after=None 許容
- `test_is_self_initiated_false_at_rest` — guarded スコープ外で false
- `test_flush_empty_queue_is_noop` — 空キュー flush の early-return no-op（SetWindowPos 非呼出・パニックなし）

**`crates/wintf/src/ecs/window/components.rs`（in-source `mod tests`・新規, 8件）**
- `test_window_default_fields` — Window::default の title="Window"/parent=None/mode=ULW
- `test_window_style_default_is_popup_visible_layered` — style=WS_POPUP\|WS_VISIBLE / ex=WS_EX_LAYERED
- `test_window_style_default_does_not_use_overlappedwindow` — 非 WS_OVERLAPPEDWINDOW・非 WS_CAPTION（ドラッグ縮小バグ回避根拠）
- `test_window_style_partial_eq` — WindowStyle の PartialEq
- `test_composition_mode_eq_distinguishes_variants` — ULW≠DComp（既存5件の補完）
- `test_dpi_change_context_new_stores_fields` — new の new_dpi/suggested_rect 格納
- `test_dpi_change_context_take_returns_none_when_unset` — 未設定 take=None
- `test_dpi_change_context_set_then_take_consumes` — set→take 消費・2回目 None（thread_local 消費的取得）

**`crates/wintf/src/ecs/window/monitor.rs`（in-source `mod tests`・新規, 7件）**
ヘルパ `make_monitor(handle, l, t, r, b)` を追加（実 HMONITOR 非依存の合成 Monitor。`monitor_hierarchy_test.rs::make_test_monitor` と同方式）。
- `test_physical_size_from_bounds` — (right-left, bottom-top)
- `test_physical_size_with_negative_origin` — 負原点セカンダリでも幅/高さ正
- `test_top_left_from_bounds` — (left, top)
- `test_top_left_with_negative_origin` — 負原点の top_left
- `test_partial_eq_compares_handle_only` — **handle 同一なら bounds/dpi/is_primary 相違でも等価／handle 相違で非等価**
- `test_debug_format_contains_fields` — Debug の bounds カスタム整形 "(0,0,800,600)"
- `test_monitor_error_display` — GetMonitorInfoFailed/GetDpiFailed の Display 文言

## 除外したテスト

なし。`ecs/window/` 配下に既存 in-source テストは存在しなかった（除外対象自体なし）。統合テスト側（`tests/window/` 4ファイル30件）には重複・死テストは検出されず（`composition_mode_test`=CompositionMode 5件・`find_owner_composition_mode_test`=ChildOf 走査ロジックの再現6件・`monitor_hierarchy_test`=階層/レイアウト/Monitor 系・`multiwindow_event_test`=find_owner_window/bubble/mouseleave/drag-guard、いずれも異なる観点を固定）、本セルでは触れていない。

**重複の意図的回避**: `find_owner_window`（command.rs）は `multiwindow_event_test.rs` の3件で既に直接特性化されている（World ベース・デバイス非依存）ため、本セルでは command.rs に追加せず重複を避けた。`CompositionMode::default`/`Window::default().composition_mode()`/Debug/Clone/Eq は `composition_mode_test.rs` で固定済みのため、components.rs では未固定の `Window::default` の title/parent と ULW≠DComp の補完のみを追加した。`physical_size`/`top_left` は `monitor_hierarchy_test.rs` が update_monitor_layout_system 経由で**間接**検証していたが、関数を**直接**呼ぶ単体テスト（負原点・PartialEq の handle-only 性質・Debug 整形）は空白だったため追加した。過不足整理の結論: **不足のみ存在（43件で充足）、過剰なし**。

## Win32 依存で未テストの箇所・深掘り所見（R2.8）

1. **`components.rs` の `WindowStyle::from_hwnd` と `on_window_add` フックは Win32/DeferredWorld 依存** — `from_hwnd` は実 HWND に対し `GetWindowLongPtrW(GWL_STYLE/GWL_EXSTYLE)` を呼ぶ。`on_window_add`（Window コンポーネント on_add フック）は `GetDpiForSystem()` を呼びつつ `DeferredWorld::commands()` で Visual/WindowPos/DPI を自動挿入する。前者は実ウィンドウ、後者は DeferredWorld 内フック実行（コマンド適用後の World 状態観測）を要し、いずれもユニット単体では決定的に再現できない。Visual/WindowPos の自動挿入効果は `tests/visual/widget_visual_auto_insert_test.rs`（W5a-T で確認）と同型の統合経路・実起動 S7 が回帰検知器。環境制約のため提案化しない。

2. **`window_pos.rs` の実 HWND 経路（`set_window_pos`/`to_window_coords`）と `to_window_coords_for_creation` のフレーム調整経路はデバイス依存** — `set_window_pos` は実 `SetWindowPos`、`to_window_coords` は `WindowHandle::client_to_window_coords`（実 HWND の AdjustWindowRectExForDpi）に委譲する。`to_window_coords_for_creation` は CW_USEDEFAULT 素通し分岐のみ API 非呼出で、具体座標 + 実フレームスタイル（WS_OVERLAPPEDWINDOW 等）の場合は `AdjustWindowRectExForDpi` がスタイル/DPI 依存のフレーム幅を加算する。本セルでは **WS_POPUP/ex=0（フレームなし）の調整量0＝座標恒等** を特性化し（実 API を呼ぶが結果は決定的・無フレームのため環境非依存）、純粋判定部（CW_USEDEFAULT 素通し・bool→SWP 写像・ZOrder→HWND 写像）を全面固定した。フレーム付きスタイルの座標変換は実ウィンドウスタイル依存でユニット不能。環境制約のため提案化しない。

3. **`SetWindowPosCommand` の enqueue 内容を観測する API が存在しない（→ P63）** — `SetWindowPosCommand::enqueue` は thread_local `WINDOW_POS_COMMANDS` に push し、`flush` が `guarded_set_window_pos`（実 SetWindowPos）で消費しながら drain する。キュー内容を非破壊で読み取る検査 API がないため、「enqueue されたコマンドの座標/フラグが正しいか」をユニットで観測できない（flush すると実 SetWindowPos が呼ばれ、かつキューが空になる）。本セルでは `new`（フィールド格納）と空キュー flush の no-op までを特性化した。これは A1-T の P1（areka 側で `on_shell_drag` の enqueue 内容検証が同 API 欠如で不能）と**同根の wintf 側ギャップ**であり、テスト用キュー検査 API（`#[cfg(any(test, feature="test-util"))] pub fn take_queued()` 等）の追加候補として P63 に記録。`guarded_set_window_pos`/非空 flush の実 SetWindowPos 副作用（実ウィンドウ移動・WM_WINDOWPOSCHANGED 同期発火・`is_self_initiated()` の echo 判定）は実環境統合経路でのみ検証可能で、ユニット不能は環境制約。

4. **`window_handle.rs` 全体が実 HWND 依存** — `WindowHandle` の全メソッド（`get_dpi`=GetDpiForWindow、`get_style`=GetWindowLongPtrW、`client_to_window_rect`/`window_to_client_rect`/`client_to_window_coords`/`window_to_client_coords`=AdjustWindowRectExForDpi）は実ウィンドウハンドルを要する。`window_to_client_rect` の「原点差分による逆変換」アルゴリズム自体は決定的だが、入力の `client_to_window_rect` が実 HWND の DPI/スタイルに依存するため切り離せない。`on_window_handle_add`（GetDpiForWindow + App リソース通知 + DPI 更新）/`on_window_handle_remove`（App 通知 + PostMessageW(WM_CLOSE)）も実 HWND と App リソースを要する。実起動 S7 と graphics 統合テスト群が回帰検知器。環境制約のため提案化しない。

5. **`window_system.rs::create_windows` は実ウィンドウ作成（CreateWindowExW）** — SystemState クエリ→CompositionMode による ex_style 調整→`to_window_coords_for_creation`→`CreateWindowExW`→`ShowWindow`→WindowHandle/HasGraphicsResources 挿入の排他システム。座標導出は window_pos.rs（本セルで特性化済み）に委譲し、CompositionMode→ex_style 分岐（ULW=WS_EX_LAYERED / DComp=WS_EX_NOREDIRECTIONBITMAP かつ WS_EX_LAYERED 除去）は純粋だが、システム全体が `WinProcessSingleton`（プロセスシングルトン・ウィンドウクラス登録）と実 CreateWindowExW に密結合のため抽出単体不能。実起動 S7 が最終回帰検知器。環境制約のため提案化しない。なお ex_style 分岐ロジックの単体抽出（純粋関数化）は将来の簡素化候補だが、判断に迷う構造変更のため本 T セルでは見送り（S 観点 W7a-S の検討事項として申し送り）。

6. **`monitor.rs` の `from_hmonitor`/`enumerate_monitors` は実 HMONITOR/EnumDisplayMonitors 依存** — `from_hmonitor` は `GetMonitorInfoW`/`GetDpiForMonitor` を実 HMONITOR に対して呼ぶ。`enumerate_monitors` は `EnumDisplayMonitors` のコールバック（生ポインタ経由で Vec へ push）でシステム全モニタを列挙する。いずれも実ディスプレイ構成を要しユニット不能。デバイス非依存な導出関数（physical_size/top_left）と等価/整形（PartialEq/Debug/MonitorError::Display）は合成 Monitor で全面固定した（本セル7件）。列挙の実挙動は `monitor_hierarchy_test.rs::test_monitor_enumeration`（実環境で `monitor_count >= 1` を確認）と実起動 S7 が回帰検知器。環境制約のため提案化しない。

7. **`find_owner_composition_mode_test.rs` は本番関数ではなくロジック再現をテストしている（所見・本セルでは非対応）** — `tests/window/find_owner_composition_mode_test.rs` の `query_composition_mode` ヘルパは「`find_owner_window_composition_mode` は DeferredWorld が必要だが、同じロジックを World で再現する」とコメントし、本番関数（`ecs/window_proc/` 配下にあると推測される DeferredWorld ベースの関数）ではなく **ChildOf 走査ロジックの複製**を検証している。本番関数の直接テストは DeferredWorld 依存かつ **W7a-T2（window_proc/）境界**のため本 T1 セルでは対象外。再現ロジックと本番ロジックの乖離リスク（複製の同期ずれ）は所見として記録するに留める（境界外）。

## proposals へ回した候補

- **P63**: `SetWindowPosCommand` キューのテスト用検査 API 追加（enqueue 内容の非破壊観測手段欠如）。A1-T の P1 と同根の wintf 側ギャップ。R2.8 適用域（テスト保護外領域の解析所見）。

既存提案との関連: A1-T の **P1**（areka `on_shell_drag` の enqueue 内容検証が同 API 欠如で不能）が wintf への API 追加を「A1-T 境界外」として保留していた所見であり、本セルで wintf `ecs/window/command.rs` 側からも同一ギャップ（enqueue 観測不能）を確認した。P63 は P1 と統合実装可能（wintf に検査 API を追加し、wintf 側 command.rs と areka 側 on_shell_drag の双方で座標アサーションを可能にする）。

## verification (S2)

- BEFORE: 親のベースライン（**1568 passed / 0 failed**・クリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。触れたバイナリ（wintf lib のみ）の BEFORE 内訳は、改善前に `cargo test -p wintf --lib window::` を実測して **0件**（`ecs/window/` に in-source テストなし・wintf lib 全体 354件）であることを確認済み。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1611 passed / 0 failed**（全テストバイナリで failed=0、`test result:` 行を awk で合算して実測。`error[`/`panicked`/`FAILED` 行ゼロ）。
  - グローバル合計は 1568 → 1611（**+43**）。追加分はすべて wintf lib in-source（`--lib`）: **354 → 397（+43）**。他バイナリの件数変動なし。
  - 触れたファイルの新規 `#[test]` 件数内訳（git diff の実数と完全一致。`git diff --unified=0 -- crates/wintf/src/ecs/window | grep -c "^+.*#\[test\]"` = 43、削除 0）:
    - `dpi.rs`: **0 → 9（+9）**
    - `window_pos.rs`: **0 → 15（+15）**
    - `command.rs`: **0 → 4（+4）**
    - `components.rs`: **0 → 8（+8）**
    - `monitor.rs`: **0 → 7（+7）**
    - 合計 **+43**（9+15+4+8+7）
  - 反復検証: `cargo test -p wintf --lib window::` で window モジュール in-source **43 passed / 0 failed**（既存0 + 追加43）。`cargo test -p wintf --test window` で統合 **30 passed / 0 failed**（既存維持・本セル変更なし）。
  - 全43件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。深掘りを要する初回失敗なし（バグ・前提誤りの検出なし）。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W7a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリは **79 passed / 0 failed** と合格（隔離再実行不要）。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` は既存警告122件 + 既存 error 20件を出力。
  - **error 20件はすべて `com/d2d/command_sink.rs`**（`clippy::not_unsafe_ptr_arg_deref`= COM vtable コールバックの生ポインタ引数）であり、`ecs/window/` とは無関係・本セル以前から存在（W2-T 所見1で既知）。S3 規定により記録のみ・非ブロッカー。
  - `ecs/window/` 配下の clippy 警告（`window_pos.rs:40` ZOrder の derivable Default / `:337` useless conversion Point/SizeI / `:425` collapsible if）はすべて**プロダクションコード**の既存警告。本セルで追加した `mod tests`（dpi.rs/window_pos.rs:442 以降/command.rs/components.rs/monitor.rs の各 `#[cfg(test)]` ブロック）を指す診断は**ゼロ**。
  - 本セルはテスト追加のみでプロダクションコード未変更のため、**新規 clippy 警告/error の導入はゼロ**。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点 W7a-S の担当。ZOrder の derivable Default 等は W7a-S 検討候補として申し送り）。

## RED フェーズ代替の検証

追加43件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した:
- **dpi**: `from_WM_DPICHANGED` の `wparam.0 & 0xFFFF`（X）/`(wparam.0 >> 16) & 0xFFFF`（Y）のビット演算（dpi.rs:54-58）、`scale_x = dpi_x/96.0`（:61-63）、`to_physical_x = (logical*scale).round()`（:114-116）の half-away-from-zero 丸め、Default=96（:30-37）をソースから転記。WPARAM ビット境界（0x00C00078→120/192）と丸め境界（1.5→2/4.5→5）は手計算で導出。
- **window_pos**: `build_flags` の自動判定（position None→SWP_NOMOVE / size None→SWP_NOSIZE / NoChange→SWP_NOZORDER）と 10 bool→各 SWP の `if` 連鎖（window_pos.rs:223-273）、`get_hwnd_insert_after` の 6 アーム match（:283-292）、`to_window_coords_for_creation` の CW_USEDEFAULT 早期 return（:367-370）、Default=CW_USEDEFAULT（:73-97）をソースから導出。WS_POPUP/ex=0 で AdjustWindowRectExForDpi 調整量0（座標恒等）は Win32 仕様（フレームなしウィンドウは枠サイズ0）から導出し実行で確認。
- **command**: `SetWindowPosCommand::new` のフィールド素通し格納（command.rs:131-149）、`is_self_initiated` の `SELF_INITIATED_DEPTH > 0`（:40-42、rest=0）、`flush` の `commands.is_empty()` early-return（:173-175）をソースから導出。
- **components**: `Window::default`（title="Window"/parent=None/mode=ULW、components.rs:133-141）、`WindowStyle::default`（WS_POPUP\|WS_VISIBLE / WS_EX_LAYERED、:157-170）、`DpiChangeContext::set`/`take` の thread_local 設定・take 消費（:58-89）をソースから転記。
- **monitor**: `physical_size`=(right-left, bottom-top)（monitor.rs:142-146）、`top_left`=(left, top)（:149-151）、`PartialEq` が `self.handle == other.handle` のみ（:103-107）、`Debug` の bounds "(l,t,r,b)" 整形（:80-86）、`MonitorError::Display` の2文言（:161-168）をソースから導出。

初回実行で43件全件が導出どおり一致し、バグ・前提誤りは検出されなかった（深掘りを要する初回失敗なし）。
