# W7a-T2: wintf メッセージブリッジ（ecs/window_proc/） × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W7a-T2（領域 W7a「wintf ウィンドウ・メッセージ」の**事前分割サブセル2/2**（メッセージ種別ごとの変換・ディスパッチロジック） × 観点 T「テスト網羅性」）。担当は **`ecs/window_proc/` のみ**。`ecs/window/` は 17.1 W7a-T1 の担当ゆえ一切触れていない。
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。**W7a-T2 の先行断片なし**。`ecs/window_proc/` のモジュール×テスト対応表をゼロから作成した（`dpi_helpers.rs` のみ既存 in-source 9件）。
- requirements: 1.3（大領域の細分化 = T セル事前分割の根拠）, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9（テスト配置規約 = structure.md 命名規約、in-source `mod tests` を許容）、レビュー観点列 T、CellExecutor 観点別規則（T）、W7a 領域定義と T セル事前分割（>1,500行・テスト薄め域の2サブセル化）、セル断片様式、提案記録様式
- 参考: `report/cells/W7a-T1.md`（直前の W7a 姉妹セル = `ecs/window/`。Win32 依存所見・in-source `mod tests` パターン・WPARAM 解析テスト）・`W6a-T.md`（メッセージ→イベント変換 T セル。WPARAM/LPARAM 解析の参考）

## 対象ファイル一覧（W7a-T2 = `crates/wintf/src/ecs/window_proc/`）

プロダクション LOC（本セルのテスト追加前、git stash 実測）合計 **2,264 LOC**。`dpi_helpers.rs` の 259 は既存 in-source 9件を含む。

- `mod.rs`（92 LOC）— `ecs_wndproc`（メッセージ種別→ハンドラの dispatch table = 31 アームの `match`）、`get_entity_from_hwnd`（`GetWindowLongPtrW(GWLP_USERDATA)` → `Entity::try_from_bits`）、`set_ecs_world`/`try_get_ecs_world`（`OnceLock<SendWeak>` 経由の World 弱参照管理）
- `lifecycle.rs`（152 LOC）— `WM_NCCREATE`（CREATESTRUCTW→GWLP_USERDATA 保存）/`WM_NCDESTROY`（despawn + USERDATA クリア）/`WM_ERASEBKGND`（常に `LRESULT(1)`）/`WM_PAINT`（CompositionMode 判定→DComp は委譲・ULW は BeginPaint/EndPaint）/`WM_CLOSE`（DestroyWindow）/`WM_DISPLAYCHANGE`（App::mark_display_change）
- `window_pos.rs`（368 LOC）— `WM_WINDOWPOSCHANGED`（3ステッププロトコル: echo 判定→WindowPos/BoxStyle 更新→try_tick_on_vsync→flush_window_pos_commands）/`WM_DPICHANGED`（DPI 直接更新→DpiChangeContext::set→guarded_set_window_pos）。WPARAM 解析は `DPI::from_WM_DPICHANGED`（**W7a-T1 境界の `window/dpi.rs`** に委譲・そこで特性化済み）
- `mouse_move.rs`（480 LOC）— `WM_NCHITTEST`（ScreenToClient + GetClientRect 判定→cached_nchittest）/`WM_MOUSEMOVE`（TrackMouseEvent + hit_test + ドラッグ閾値/累積 + leave 収集 + deferred SetWindowPos）/`WM_MOUSELEAVE`（ウィンドウスコープ PointerState 除去）/**`collect_entities_to_leave`**（`pub(super)`・World ベース leave 対象収集）
- `mouse_click.rs`（453 LOC）— `handle_button_message`（共通: LPARAM 座標 + WPARAM 修飾キー抽出 + hit_test + PointerState 確保 + ドラッグ準備/終了）/WM_[LRM]BUTTON[DOWN\|UP] 6種/WM_XBUTTON[DOWN\|UP]（XBUTTON 抽出）/**`find_ancestor_with_drag_config`**（private・World ベース ChildOf 走査）
- `mouse_dblclick_wheel.rs`（221 LOC）— `handle_double_click_message`（DoubleClick→PointerButton マッピング + LPARAM/WPARAM 抽出 + hit_test）/WM_[LRMX]BUTTONDBLCLK 4種/WM_MOUSEWHEEL・WM_MOUSEHWHEEL（HIWORD 符号付き delta 抽出）
- `keyboard.rs`（239 LOC）— `WM_KEYDOWN`（VK_ESCAPE→ドラッグキャンセル）/`WM_CANCELMODE`（ドラッグキャンセル）/`WM_ACTIVATE`（LOWORD activation_state→WA_INACTIVE 時ドラッグキャンセル）/`WM_CAPTURECHANGED`（capture_guard.mark_released→キャンセル）
- `dpi_helpers.rs`（259 LOC、既存テスト 9件込み）— `calculate_physical_size_from_box_style`（private・BoxStyle.size×DPI scale→ceil 物理サイズ）/`calculate_center_correction`（private・(old-new)/2）/**`correct_position_for_dpi_center_preserve`**（`pub(super)`・中心保持補正エントリポイント）

境界 = `ecs/window_proc/` のみ。`ecs/window/`（W7a-T1）には一切触れていない。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `mod.rs` | `ecs_wndproc`（31 アーム dispatch table）、`get_entity_from_hwnd`、`set_ecs_world`/`try_get_ecs_world` | **全面 Win32/OnceLock 依存**。dispatch table の各アームは実 HWND/WPARAM/LPARAM を要する `pub(super)` ハンドラを呼ぶ。`get_entity_from_hwnd` は `GetWindowLongPtrW`、World 弱参照は `OnceLock`（プロセス単一・set 後不変） | なし（実 WndProc 経路・実起動 S7） | 0件 | 純粋ロジックの抽出可能箇所なし。dispatch の網羅性（メッセージ種別→ハンドラ写像）は `match` 文の静的構造であり、各アームが Win32 依存ハンドラに直結するためユニット到達不能（所見1） |
| `lifecycle.rs` | WM_NCCREATE/NCDESTROY/ERASEBKGND/PAINT/CLOSE/DISPLAYCHANGE | **全面 Win32/World 依存**。CREATESTRUCTW/WINDOWPOS 生ポインタ・SetWindowLongPtrW・despawn・DestroyWindow・BeginPaint/EndPaint・App リソース通知 | なし | 0件 | 純粋判定の単独抽出箇所なし。`WM_ERASEBKGND` は無条件 `LRESULT(1)`（定数）で特性化価値が低い。`WM_PAINT` の CompositionMode 分岐は実 World + Window コンポーネント取得が前提（所見2） |
| `window_pos.rs` | WM_WINDOWPOSCHANGED（3ステップ）、WM_DPICHANGED | **全面 Win32/World 依存**。WINDOWPOS/RECT 生ポインタ・window_to_client_coords（実 HWND の AdjustWindowRectExForDpi）・bypass_change_detection・try_tick_on_vsync・guarded_set_window_pos（実 SetWindowPos）。WPARAM 解析は **`DPI::from_WM_DPICHANGED`（W7a-T1 で特性化済み）** に委譲 | なし（DPI WPARAM 解析は window/dpi.rs に存在） | 0件 | 中心保持補正の純粋部は `dpi_helpers.rs` に既に分離されている（下記参照）。本ハンドラ本体は echo 判定・WindowHandle 座標変換・DerefMut/bypass 選択・vsync tick・flush の手続きで、実 HWND/World に密結合（所見3） |
| `mouse_move.rs` | WM_NCHITTEST/MOUSEMOVE/MOUSELEAVE、**`collect_entities_to_leave`** | WM_* ハンドラは **全面 Win32/World 依存**（ScreenToClient・GetClientRect・TrackMouseEvent・hit_test_in_window・cached_nchittest・guarded_set_window_pos・ドラッグ thread_local）。**`collect_entities_to_leave` は World ベースで完全にデバイス非依存** | **なし（0件）** | **4件** | 空白: **`collect_entities_to_leave`**（PointerState 保持者のうち exclude 以外かつ当該ウィンドウ所属を `find_owner_window` で収集）が完全未テストだった。当該ウィンドウ所属収集・exclude 除外・**他ウィンドウ PointerState 保護**・PointerState 不在スキップを特性化。ハンドラ本体の LPARAM/WPARAM 抽出はインライン（所見4・P64） |
| `mouse_click.rs` | `handle_button_message`、WM_[LRMX]BUTTON[DOWN\|UP] 10種、**`find_ancestor_with_drag_config`** | `handle_button_message` と各ハンドラは **全面 Win32/World/drag 依存**（hit_test・PointerState 挿入・start_preparing/end_dragging・DragAccumulatorResource・snapshot_drag_state）。**`find_ancestor_with_drag_config` は World ベースで完全にデバイス非依存** | **なし（0件）** | **5件** | 空白: **`find_ancestor_with_drag_config`**（ChildOf 走査で start 自身または祖先の DragConfig を返す）が完全未テストだった。start 自身・祖先・不在・孤立エンティティ・**最近傍優先（start と祖先双方保持時 start を返す）**を特性化。XBUTTON 抽出（HIWORD==1→XB1/else→XB2）・LPARAM 座標・MK_SHIFT/MK_CONTROL 抽出はハンドラ内インライン（所見5・P64） |
| `mouse_dblclick_wheel.rs` | `handle_double_click_message`（DoubleClick→PointerButton マッピング含む）、WM_*DBLCLK 4種、WM_MOUSEWHEEL/MOUSEHWHEEL | **全面 Win32/World 依存**。マッピング・XBUTTON 抽出・HIWORD 符号付き delta 抽出はすべて `pub(super)` ハンドラ内のインライン式で、実 HWND/World/hit_test と同一スコープに埋め込まれ単独到達不能 | なし | 0件 | DoubleClick→PointerButton の 6 アーム match と wheel delta の `((wparam.0 >> 16) & 0xFFFF) as i16` 符号付き抽出はデバイス非依存だが、いずれもハンドラ本体に埋め込まれ抽出関数化されていないため現状ユニット不能（所見6・P64） |
| `keyboard.rs` | WM_KEYDOWN/CANCELMODE/ACTIVATE/CAPTURECHANGED | **全面 drag thread_local/World 依存**。VK_ESCAPE 比較・WM_ACTIVATE の LOWORD activation_state 抽出（`(wparam.0 & 0xFFFF) as u32`）・WA_INACTIVE 判定はデバイス非依存だが、snapshot_drag_state/cancel_dragging/update_drag_state（drag thread_local）と同一ハンドラ内に埋め込まれ単独到達不能 | なし | 0件 | activation_state の LOWORD 抽出 + 早期 return（非ゼロ→None）はデバイス非依存判定だが、後続が drag thread_local 操作と不可分でハンドラ単位の到達には実 drag 状態が必要（所見7・P64） |
| `dpi_helpers.rs` | `calculate_physical_size_from_box_style`、`calculate_center_correction`、**`correct_position_for_dpi_center_preserve`** | **全面デバイス非依存（純粋）** | in-source `mod tests` 9件（center_correction 4件: 減少/増加/同一/中心保持、physical_size 5件: 125%/200%/None/ceiling/非Px） | **5件** | 空白: **`correct_position_for_dpi_center_preserve`**（純粋エントリポイント）が**ゼロテストだった**（2ヘルパは 9件で網羅済みだがエントリの分岐は未固定）。dpi_context=None 素通し・box_style=None フォールバック・非Px フォールバック・補正量0 素通し・**実補正適用（pos+correction の writethrough + 中心保持）**を特性化 |

追加テスト合計 **14件**（dpi_helpers 5・mouse_click 5・mouse_move 4、すべて **in-source `mod tests`**）。**プロダクションコードの変更なし**（R5.1 充足。git diff: 322 insertions / 0 deletions、新規 `#[test]` = 14・削除 0、すべて `#[cfg(test)]` 内のテスト・ヘルパ）。新規テストファイルなし（dpi_helpers.rs は既存 `mod tests` へ追記、mouse_click.rs/mouse_move.rs は `mod tests` を新規作成）。統合テスト側（`tests/`）への追加・変更なし。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/window_proc/dpi_helpers.rs`（既存 `mod tests` へ追記, 5件）**
- `test_correct_position_returns_input_when_dpi_context_none` — dpi_context=None（DPI 変更なし）で client_pos 素通し
- `test_correct_position_returns_input_when_box_style_none` — box_style=None フォールバックで素通し（warn 分岐）
- `test_correct_position_returns_input_when_size_not_px` — BoxStyle.size 非Px（物理サイズ計算不可）でフォールバック素通し
- `test_correct_position_returns_input_when_correction_is_zero` — 新旧サイズ一致（補正量0）で素通し（BoxStyle 400×300 @192dpi=800×600 と client_size 800×600 一致）
- `test_correct_position_applies_center_preserving_correction` — 実補正: client_size 800×600 → 新 500×375（BoxStyle 400×300 @120dpi）で correction=(150,112)・corrected=(250,312)・X 中心保持を検証

**`crates/wintf/src/ecs/window_proc/mouse_click.rs`（in-source `mod tests`・新規, 5件）**
- `test_find_drag_config_on_start_entity_itself` — start 自身が DragConfig 保持→(start, clone) を返す
- `test_find_drag_config_on_ancestor` — start に無し祖先（親の親）が保持→祖先 entity を返す
- `test_find_drag_config_returns_none_when_absent` — チェーン上に DragConfig なし→None
- `test_find_drag_config_returns_none_for_isolated_entity` — ChildOf なし孤立エンティティ→None（ループ終端）
- `test_find_drag_config_prefers_nearest_ancestor` — start と祖先双方が保持→最近傍（start, threshold=3）を優先（祖先 99 ではない）

**`crates/wintf/src/ecs/window_proc/mouse_move.rs`（in-source `mod tests`・新規, 4件）**
- `test_collect_excludes_target_and_collects_siblings` — 同一ウィンドウ配下の PointerState 保持者から exclude 以外を収集
- `test_collect_does_not_include_entities_without_pointer_state` — PointerState 不在エンティティは非収集
- `test_collect_protects_other_windows_pointer_state` — 別ウィンドウ B 配下の PointerState を保護（非収集）・A 配下のみ収集（ウィンドウスコーピング）
- `test_collect_returns_empty_when_only_excluded_holder_exists` — 保持者が exclude 1件のみ→空

## 除外したテスト

なし。`ecs/window_proc/` 配下の既存 in-source テストは `dpi_helpers.rs` の 9件のみで、いずれも `calculate_center_correction`/`calculate_physical_size_from_box_style` の異なる観点（サイズ増減・同一・ceiling・None・非Px）を固定しており重複・死テストは検出されなかった（本セルでは触れず、未固定の `correct_position_for_dpi_center_preserve` エントリポイントのみ補完した）。統合テスト側で window_proc ハンドラを直接テストするものは存在しない（`ecs_wndproc` は `mod.rs` の `match` から `pub(super)` ハンドラを呼ぶのみで、ハンドラは crate 外非公開・実 WndProc 経路でのみ到達）。過不足整理の結論: **不足のみ存在（14件で充足）、過剰なし**。

## Win32 依存で未テストの箇所・深掘り所見（R2.8）

1. **`mod.rs` の dispatch table（`ecs_wndproc`）は実 WndProc 経路でのみ到達** — `ecs_wndproc`（mod.rs:42-83）は 31 種のメッセージ種別を `match` で各 `pub(super)` ハンドラへ写像し、未対応種別は `DefWindowProcW` にフォールバックする。dispatch の網羅性（メッセージ→ハンドラ写像）は `match` 文の静的構造で検証する性質のものであり、各アームが実 HWND/WPARAM/LPARAM を要するハンドラに直結するためユニットで駆動できない（呼べば実 SetWindowLongPtrW/DestroyWindow/SetWindowPos 等が走る）。`get_entity_from_hwnd` は `GetWindowLongPtrW(GWLP_USERDATA)` を実 HWND に対して呼び、`set_ecs_world`/`try_get_ecs_world` は `OnceLock`（プロセス単一・初回 set 後不変）で World 弱参照を管理する。いずれも実起動 S7 と実メッセージポンプが回帰検知器。環境制約のため提案化しない。

2. **`lifecycle.rs` 全ハンドラが実 HWND/World 依存** — `WM_NCCREATE`（CREATESTRUCTW 生ポインタ→SetWindowLongPtrW）/`WM_NCDESTROY`（get_entity_from_hwnd→despawn→USERDATA クリア）/`WM_PAINT`（実 World から Window コンポーネント取得で CompositionMode 判定→DComp 委譲 or BeginPaint/EndPaint）/`WM_CLOSE`（DestroyWindow）/`WM_DISPLAYCHANGE`（App::mark_display_change）はいずれも実ウィンドウ生成/破棄/描画と実 World を要する。`WM_ERASEBKGND` は無条件 `LRESULT(1)`（背景消去スキップ）の定数で、特性化しても定数の再記述にしかならず観測価値が低い。環境制約のため提案化しない。

3. **`window_pos.rs` のハンドラ本体は実 HWND/World に密結合（純粋部は dpi_helpers.rs に既に分離済み）** — `WM_WINDOWPOSCHANGED`（window_pos.rs:30-266）は (a) `is_self_initiated()` echo 判定（drag thread_local）、(b) WINDOWPOS 生ポインタ→`WindowHandle::window_to_client_coords`（実 HWND の AdjustWindowRectExForDpi）、(c) `bypass_change_detection`/DerefMut の選択、(d) `try_tick_on_vsync`、(e) `flush_window_pos_commands`（実 SetWindowPos）の3ステッププロトコルで、抽出可能な純粋計算は**既に `dpi_helpers.rs` の `correct_position_for_dpi_center_preserve` に分離されている**（本セルで5件追加）。`WM_DPICHANGED`（:279-368）の WPARAM 解析は `DPI::from_WM_DPICHANGED` に委譲され、これは **W7a-T1 境界の `window/dpi.rs`** で既に特性化済み（W7a-T1 の dpi 9件、WPARAM ビット解析含む）であり本セル境界外。残る本体（DPI 直接更新・DpiChangeContext::set・guarded_set_window_pos）は実 HWND/World 依存。環境制約のため提案化しない。

4. **`mouse_move.rs` の WM_* ハンドラは実 Win32 依存、純粋部は `collect_entities_to_leave` のみ抽出済み** — `WM_NCHITTEST`（ScreenToClient + GetClientRect + cached_nchittest）/`WM_MOUSEMOVE`（TrackMouseEvent + hit_test_in_window + ドラッグ閾値/累積 + deferred guarded_set_window_pos）/`WM_MOUSELEAVE`（ウィンドウスコープ除去）はいずれも実 HWND・実 hit_test・drag thread_local を要する。デバイス非依存な `collect_entities_to_leave`（mouse_move.rs:67-83・`pub(super)`）は本セルで4件特性化した。ハンドラ本体の LPARAM 座標抽出（`(lparam.0 & 0xFFFF) as i16 as i32` / `((lparam.0 >> 16) & 0xFFFF) as i16 as i32`）・MK_SHIFT(0x04)/MK_CONTROL(0x08) 抽出は **3ファイル（mouse_move/mouse_click/mouse_dblclick_wheel）に同一式が重複**するインラインコードで、抽出関数化されていないため現状ユニット不能（→ P64）。

5. **`mouse_click.rs` のハンドラは実 Win32/drag 依存、純粋部は `find_ancestor_with_drag_config` のみ抽出済み** — `handle_button_message`（mouse_click.rs:18-297）は hit_test_in_window・PointerState 挿入・`start_preparing`/`end_dragging`・DragAccumulatorResource・snapshot_drag_state（drag thread_local）と密結合し、WM_[LRMX]BUTTON ハンドラはこれを呼ぶ薄いラッパ。デバイス非依存な `find_ancestor_with_drag_config`（:438-453・private）は本セルで5件特性化した。`WM_XBUTTON*` の XBUTTON 抽出（`((wparam.0 >> 16) & 0xFFFF) as u16` → 1 なら XButton1 / else XButton2）はデバイス非依存判定だが、`handle_button_message` を即座に呼ぶハンドラ内インライン（抽出関数なし）のため単独到達不能（→ P64）。

6. **`mouse_dblclick_wheel.rs` の DoubleClick→PointerButton マッピングと wheel delta 抽出はハンドラ内インラインで未抽出** — `handle_double_click_message`（mouse_dblclick_wheel.rs:26-124）内の `match double_click { Left=>..., None=>return }`（6 アーム）と、`WM_MOUSEWHEEL`/`WM_MOUSEHWHEEL`（:188-221）の `((wparam.0 >> 16) & 0xFFFF) as i16` 符号付き delta 抽出はいずれもデバイス非依存の純粋写像だが、(a) マッピングは hit_test を含む `pub(super)` ハンドラ本体に埋め込まれ、(b) wheel 抽出は `get_entity_from_hwnd`（実 HWND）→`add_wheel_*`（thread_local）と同一スコープにあり、どちらも抽出関数化されていない。現状ユニット到達不能（→ P64）。

7. **`keyboard.rs` の activation_state 抽出と drag キャンセルロジックは不可分** — `WM_ACTIVATE`（keyboard.rs:115-166）の `(wparam.0 & 0xFFFF) as u32` LOWORD 抽出 + WA_INACTIVE(0) 判定 + 早期 return（非ゼロ→None）はデバイス非依存判定だが、後続が `snapshot_drag_state`/`cancel_dragging`（drag thread_local）と不可分。`WM_KEYDOWN` の VK_ESCAPE 比較・`WM_CAPTURECHANGED` の `update_drag_state`+`capture_guard.mark_released` も同様に drag thread_local 状態に密結合し、ハンドラ単位の到達には実 drag 状態の構築が必要（drag ドメイン = 境界外）。環境制約 + 境界制約のため本セルでは抽出せず、P64（共通抽出提案）に集約。

## proposals へ回した候補

- **P64**: `window_proc` のメッセージパラメータ抽出ロジック（LPARAM 座標の符号付き lo/hi ワード抽出・WPARAM 修飾キー/XBUTTON/wheel delta 抽出・WM_ACTIVATE activation_state 抽出・DoubleClick→PointerButton マッピング）が各 `pub(super)` ハンドラ本体にインライン埋め込みされており、純粋関数として抽出されていないため単体到達不能。LPARAM 座標抽出式は3ファイルに同一複製。挙動非破壊な純粋ヘルパ抽出（例: `extract_client_point(lparam) -> (i32, i32)`、`extract_modifier_keys(wparam) -> (bool, bool)`、`double_click_to_button(DoubleClick) -> Option<PointerButton>`）でデバイス非依存テストと DRY を両立できるが、4ファイルにまたがるプロダクション構造変更（R2.9/R2.10 の「判断に迷う構造変更」）のため本 T セルでは実装せず記録した。

既存提案との関連: W6a-T の **P58**（`transfer_buffers_to_world` のボタン down/up 転送 match 重複）と同系統の「メッセージ→ECS 変換ロジックの DRY/抽出整理」候補。P64 は window_proc 側のメッセージパラメータ抽出に特化し、抽出後に W6a-T が特性化済みの buffers 経路（`record_button_down`/`set_modifier_state` 等）との接続点をユニットで固定可能になる。W7a-T1 の所見では `find_owner_window`（window/command.rs）が World ベースで特性化済みであり、本セルの `find_ancestor_with_drag_config`/`collect_entities_to_leave` は同じ「World ベース走査ロジックはデバイス非依存」原則で追加した（提案不要、本セルで充足）。

## verification (S2)

- BEFORE: 親のベースライン（**1611 passed / 0 failed**・クリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。触れたバイナリ（wintf lib のみ）の BEFORE 内訳は、改善前に `cargo test -p wintf --lib window_proc::` を実測して **9件**（`dpi_helpers.rs` の既存 in-source 9件のみ・他7ファイルは 0件）であることを確認済み。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1625 passed / 0 failed**（全テストバイナリで failed=0、`test result:` 行を awk で合算して実測。`error[`/`panicked`/`FAILED`/`test result: FAILED` 行ゼロ）。
  - グローバル合計は 1611 → 1625（**+14**）。追加分はすべて wintf lib in-source（`--lib`）: **397 → 411（+14）**。他バイナリの件数変動なし。
  - 触れたファイルの新規 `#[test]` 件数内訳（git diff の実数と完全一致。`git diff --unified=0 -- crates/wintf/src/ecs/window_proc | grep -c "^+.*#\[test\]"` = 14、削除 0）:
    - `dpi_helpers.rs`: **9 → 14（+5）**
    - `mouse_click.rs`: **0 → 5（+5）**
    - `mouse_move.rs`: **0 → 4（+4）**
    - 合計 **+14**（5+5+4）
  - 反復検証: `cargo test -p wintf --lib window_proc::` で window_proc モジュール in-source **23 passed / 0 failed**（既存9 + 追加14）。
  - 全14件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。深掘りを要する初回失敗なし（バグ・前提誤りの検出なし）。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W7a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `... ok` と合格（隔離再実行不要・`test cue_performance_test::bench_pop_ready_empty_queue ... ok` をログで確認）。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` は既存警告（collapsible if 68 / 複雑型 30 / let-else 8 / derivable impl 5 等）+ 既存 error 20件を出力。
  - **error 20件はすべて `com/d2d/command_sink.rs`**（`clippy::not_unsafe_ptr_arg_deref`= COM vtable コールバックの生ポインタ引数）であり、`ecs/window_proc/` とは無関係・本セル以前から存在（W7a-T1 所見と一致）。S3 規定により記録のみ・非ブロッカー。
  - `ecs/window_proc/` 配下の clippy 診断が参照する行はすべて**プロダクションコードの既存行**（keyboard.rs:37-220 / lifecycle.rs:43-142 / mouse_click.rs:28-261 / mouse_dblclick_wheel.rs:35-213 / mouse_move.rs:50-471 / window_pos.rs:42-314）。本セルで追加した `mod tests`（dpi_helpers.rs:259 以降 = clippy 参照ゼロ / mouse_click.rs:436 以降 / mouse_move.rs:481 以降）の行を指す診断は**ゼロ**（最大参照行 mouse_click.rs:261・mouse_move.rs:471 はいずれも追加テスト開始行より手前）。
  - 本セルはテスト追加のみでプロダクションコード未変更のため、**新規 clippy 警告/error の導入はゼロ**。S3 規定によりブロッカーとせず記録に留める（簡素化・抽出は S 観点 W7a-S / P64 の担当）。

## RED フェーズ代替の検証

追加14件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した:
- **dpi_helpers**: `correct_position_for_dpi_center_preserve`（dpi_helpers.rs:62-113）の4つの早期 return（dpi_context=None / box_style=None / 物理サイズ計算不可 / 補正量(0,0)）→ client_pos 素通し、および最終の `corrected = {x: client_pos.x + dx, y: client_pos.y + dy}`（writethrough）をソースから導出。実補正ケースの correction=(150,112) は `calculate_center_correction((800,600),(500,375))=((800-500)/2,(600-375)/2)`、新物理サイズ 500×375 は `calculate_physical_size_from_box_style(400×300, dpi=120) = (400*1.25).ceil(), (300*1.25).ceil()` を手計算で導出。補正量0ケースの 800×600 は `400*2.0, 300*2.0`（dpi=192）。
- **mouse_click**: `find_ancestor_with_drag_config`（mouse_click.rs:438-453）の loop（current が DragConfig 保持→`(current, clone)` 返却 / ChildOf あり→parent へ / なし→None）をソースから導出。最近傍優先は loop が start から評価するため start 自身の DragConfig が祖先より先にヒットする構造から導出（祖先 threshold=99 ではなく start threshold=3 を期待）。
- **mouse_move**: `collect_entities_to_leave`（mouse_move.rs:67-83）の `query::<(Entity, &PointerState)>` 走査 + `e != exclude && find_owner_window(world, e) == Some(window_entity)` フィルタをソースから導出。他ウィンドウ保護は `find_owner_window`（W7a-T1 で特性化済み・window/command.rs:227 の ChildOf 走査）が別ウィンドウ配下のエンティティに対し異なる Window を返す性質に依拠。

初回実行で14件全件が導出どおり一致し、バグ・前提誤りは検出されなかった（深掘りを要する初回失敗なし）。
