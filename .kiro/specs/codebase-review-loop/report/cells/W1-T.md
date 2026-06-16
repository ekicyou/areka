# W1-T: wintf レガシー・プロセス × テスト網羅性

- status: completed
- commit: test(W1): wintf レガシー4モジュール（win_state/win_style/api/process_singleton）のテスト空白に34件のギャップテストを追加

## findings

### モジュール×テスト対応表（非・非推奨4モジュール、改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `win_style.rs` (393 LOC) | コンストラクタ（default / WS_OVERLAPPEDWINDOW / WS_TILEDWINDOW / WS_POPUPWINDOW / WS_OVERLAPPED / WS_POPUP）のビット構成・CW_USEDEFAULT 初期座標 | なし | 6件 | `default()` の座標は 0、`with_style` 系は CW_USEDEFAULT という差異を特性化 |
| 〃 | builder（position / size / parent）のフィールド独立性 | なし | 1件 | スタイルビット非干渉も同時に固定 |
| 〃 | `set_style` 経路の ON/OFF ラウンドトリップ・対象ビットのみのクリア・複合ビット（WS_CAPTION = BORDER\|DLGFRAME）の一括クリア・エイリアス同値（SIZEBOX=THICKFRAME / ICONIC=MINIMIZE）・style と ex_style の独立性 | なし | 7件 | `ws_tiled_is_noop_because_bit_value_is_zero` で WS_TILED（定数値 0）の ON/OFF がともに no-op である現行挙動を特性化（所見3） |
| 〃 | `set_ex` 経路の ON/OFF・複合 ex フラグ（WS_EX_OVERLAPPEDWINDOW / WS_EX_PALETTEWINDOW / WS_EX_WINDOWEDGE）のビット構成 | なし | 4件 | `ws_ex_windowedge_sets_only_windowedge_bit` で doc コメント（CLIENTEDGE との組み合わせと記載）と実ビット（0x100 単独）の乖離を特性化（所見4） |
| 〃 | ON/OFF 対（RIGHTSCROLLBAR↔LEFTSCROLLBAR / LEFT↔RIGHT / LTRREADING↔RTLREADING / LAYOUTLTR↔LAYOUTRTL）の既定値復帰 | なし | 4件 | |
| 〃 | `new(hwnd)` / `commit(hwnd)` の null HWND エラー伝播 | なし | 2件 | ウィンドウ生成不要の決定的エラー経路のみ（成功経路は実 HWND 必須のため対象外、所見1） |
| `win_state.rs` (110 LOC) | `SimpleWinState` の Default 値（null HWND / tracking=false / dpi=96.0）・セッター往復 | なし | 2件 | |
| 〃 | `WinState` トレイトのデフォルト実装（`mouse_tracking()`=true 固定 / `set_mouse_tracking` no-op） | なし | 1件 | 最小実装 `MinimalState` を定義して検証。`SimpleWinState` のオーバーライド（可変）との差異を固定 |
| 〃 | `set_dpi_change_message` の wparam 下位 16bit（X 軸 DPI）抽出・上位 16bit / lparam の無視 | なし | 2件 | WM_DPICHANGED 伝達仕様の特性化（所見5） |
| 〃 | `effective_window_size` の null HWND エラー伝播 | なし | 1件 | DPI スケール計算を含む成功経路は実 HWND（AdjustWindowRectExForDpi）必須のため解析のみ（所見1） |
| `api.rs` (62 LOC) | `get_window_long_ptr` / `set_window_long_ptr` の null HWND → ERROR_INVALID_WINDOW_HANDLE 変換 | なし | 2件 | SetLastError 事前クリア → `Error::from_thread` のエラー変換ロジックを固定。成功経路（res==0 かつ無エラー時の Ok(0) を含む）は実 HWND 必須のためコード解析で確認（所見2） |
| `process_singleton.rs` (112 LOC) | `get_or_init` の OnceLock 同一インスタンス性・クラス名（wintf_window_class / wintf_ecs_window_class）・instance 非 null・hidden_window=None | なし | 2件 | RegisterClassExW はクラス登録のみでウィンドウ生成・メッセージループを伴わないためヘッドレス可と判断。wndproc 結線の動作検証は GUI 必須のため対象外（所見6） |

追加テスト合計 34 件（すべて in-source `#[cfg(test)] mod tests`、S9 Unit Tests Inline 方式）。`win_style.rs` 24 / `win_state.rs` 6 / `api.rs` 2 / `process_singleton.rs` 2。`api.rs` / `process_singleton.rs` は `pub(crate)` 項目のため in-source が唯一の配置選択肢であり、`win_style.rs` も `WinStyle` フィールドが `pub(crate)` のためビット検証は in-source が必須。統合テストの新規ファイル・テスト入口ファイルの変更なし。

### 除外テスト

0 件（当該4モジュールに既存テストが存在しないため除外対象なし）。

### 非推奨3モジュールの調査所見（7.2 削除判定向け・テスト追加なし）

**`#[deprecated]` 注記の実態（重要）**: 3モジュールのうち実際に `#![deprecated]` 注記を持つのは `win_message_handler.rs` のみ。`win_thread_mgr.rs`（先頭 `#![allow(deprecated)]` のみ）と `winproc.rs`（同）は非推奨注記を持たず、steering（structure.md:110-112）の「3モジュールとも ⚠️ `#[deprecated]`」という記載は実態と乖離している。R2.9（削除条件: 非推奨指定かつ利用ゼロの実証)の前提となるため 7.2 で要考慮。

| モジュール | `#[deprecated]` | ワークスペース内利用（grep: crates / examples / tests） | 削除可否の所見 |
|------------|----------------|--------------------------------------------------------|----------------|
| `win_message_handler.rs` (1,378 行) | **あり**（`#![deprecated(since="0.1.0")]`、lib.rs で `#[allow(deprecated)] pub use` 再エクスポート） | `winproc.rs`（トレイトオブジェクト dispatch）、`win_thread_mgr.rs:142`（`create_window` の `handler: Arc<dyn BaseWinMessageHandler>` 引数）、`examples/dcomp_demo.rs:94`（`impl WinMessageHandler for DemoWindow`） | 利用ゼロではないため単独削除不可（R2.10）。削除セット = {win_message_handler 全体 + winproc のハンドラ dispatch 経路 + `WinThreadMgrInner::create_window` + dcomp_demo example} を一括で扱う必要がある |
| `winproc.rs` (90 行) | **なし** | `process_singleton.rs:57`（レガシークラス `wintf_window_class` の lpfnWndProc として登録）、`win_thread_mgr.rs`（メッセージ専用隠しウィンドウが同クラスで生成される） | **一部ロジックは現役**: `wndproc` の `WM_LAST_WINDOW_DESTROYED` アーム（78-83行）は areka 終了経路の一部。`ecs/app.rs:81` がメッセージウィンドウへ post し、通常は `win_thread_mgr::run()` の PeekMessage 側アーム（252行）が先に消費するが、モーダルループ（ウィンドウドラッグ等）中は OS のメッセージループが DispatchMessage するため wndproc 側アームが唯一の処理経路となる。完全削除にはメッセージウィンドウのクラス/プロシージャ移設が必要 |
| `win_thread_mgr.rs` (294 行) | **なし**（lib.rs の `pub use win_thread_mgr::*;` にも `#[allow(deprecated)]` なし） | `crates/areka/src/main.rs:87`（`WinThreadMgr::new()` — 本体アプリのエントリポイント）、examples 12 ファイル、`ecs/app.rs:81`（`WM_LAST_WINDOW_DESTROYED` 定数）、`ecs/world/mod.rs:494`（`VSYNC_TICK_COUNT` / `LAST_VSYNC_TICK`）、`ecs/world/vsync.rs:78`（`DEBUG_WNDPROC_TICK_COUNT`） | **現役インフラであり削除不可**。メッセージループ（`run()`）・VSync 監視スレッド・ECS world 保有・終了制御を担い、ecs モジュール側が同モジュールの static に依存。削除には常駐基盤の全面移設（新規仕様規模）が必要。非推奨化の経緯（steering 記載）と実装実態の不一致として 7.2 で要整理 |

**winproc.rs の健全性問題（7.2 判定の補強材料）**: `get_boxed_ptr`（winproc.rs:25-36）は (a) `into_boxed_ptr` が `Box<Arc<dyn BaseWinMessageHandler>>` として格納したポインタを `*mut Arc<dyn WinMessageHandler>` （別トレイトのファットポインタ）として読み出す型混同、(b) `#[allow(mutable_transmutes)]` による `&dyn` → `&mut dyn` の transmute（共有参照からの可変参照生成 = 未定義動作領域）の 2 点の健全性違反を含む。areka 本体ではこの経路は実行されない（メッセージウィンドウは lpParam なしで生成され GWLP_USERDATA が null のまま → `get_boxed_ptr` は常に None → DefWindowProcW へフォールバック）が、レガシー `create_window` 利用者（dcomp_demo）では実行される。7.2 でハンドラ dispatch 経路ごと削除できれば解消、削除しない場合は別途健全性修正の仕様提案が必要。

### テスト不能箇所・深掘り所見（非・非推奨4モジュール）

1. **実 HWND 依存経路はヘッドレス検証不能** — `WinStyle::new/commit`、`WinState::effective_window_size`、api ラッパーの成功経路は実在ウィンドウを要するため、本セルでは null HWND の決定的エラー経路のみ固定し、成功経路はコード解析で確認した（R2.8）。`effective_window_size` の DPI スケール計算（dpi/96.0 倍 → ceil → AdjustWindowRectExForDpi）は AdjustWindowRectExForDpi 呼び出しと不可分なため純粋ロジック部分の単独テスト不能。分離（スケール計算の純粋関数化）はテスト容易性リファクタだが必須ではないため見送り。
2. **api.rs のエラー判定規約** — `get_window_long_ptr` は戻り値によらず thread error で判定（GetWindowLongPtrW は正常戻り値 0 があり得るため SetLastError 事前クリアが必須の Win32 イディオム）、`set_window_long_ptr` は戻り値 0 のときのみ error 照会（直前値 0 と失敗の弁別）。両者の差は意図的で正しい実装と確認。
3. **`WS_TILED` / `WS_OVERLAPPED` フラグメソッドは no-op** — 定数値が 0 のため `WS_TILED(true)` / `WS_TILED(false)` ともスタイル不変（OFF にしてもオーバーラップ属性は外せない）。Win32 定数の性質によるもので呼び出し側に実害はないが、誤解を招く API。特性化テストで固定。メソッド削除は公開 API の変更となるため実施せず（利用箇所ゼロのため W1-S での dead code 整理候補）。
4. **`WS_EX_WINDOWEDGE()` の doc コメント乖離** — コメントは「WS_EX_CLIENTEDGE (0x200) と WS_EX_WINDOWEDGE (0x100) の組み合わせ」と述べるが、実際に設定されるのは 0x100 単独（組み合わせは `WS_EX_OVERLAPPEDWINDOW` の説明の誤転記とみられる）。コメント修正は挙動非破壊のため W1-S へ申し送り。
5. **`set_dpi_change_message` は X 軸 DPI のみ採用** — WM_DPICHANGED の wparam 下位 16bit のみ使用し、lparam の推奨 RECT を無視する。Win32 推奨実装（推奨 RECT への SetWindowPos）とは異なるが、本トレイトの利用箇所は非推奨 `win_message_handler` 経由のみであり、ECS 経路は `ecs/window_proc/` 側で別実装のため実害なし。
6. **`process_singleton.rs` の失敗時 panic** — `RegisterClassExW` 失敗・`LoadCursorW` / `GetModuleHandleW` の unwrap で panic する設計。プロセス起動時の回復不能条件であり panic は許容範囲と判断（Result 化は公開挙動の変更を伴い、便益も限定的なため提案化せず）。`hidden_window` フィールドは常に None の dead code（`#[allow(dead_code)]` 付き）— W1-S での整理候補。
7. **`win_style.rs` の `set_ex2` は未使用の private 関数** — 呼び出し箇所ゼロ（ON/OFF 対メソッドはすべて `set_ex` を使用）。dead code のためテスト追加せず、W1-S での削除候補として申し送り。
8. **RED フェーズ代替の検証** — 追加テストは既存挙動の特性化のため RED は N/A。ビット演算の期待値は windows-rs 定数定義と `set_style`/`set_ex` の実装読解から導出してから記述し、実行で全件一致を確認した（D3-T と同パターン）。

### 検証（S2）

- BEFORE: HEAD b8aed86 で `cargo build --workspace` 成功 / `cargo test --workspace` 1176 passed / 0 failed（親指示のベースラインと一致。既知フレーキーも本実行では合格）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 1210 passed / 0 failed（+34 はすべて追加分。既存テストの変更・削除なし）
- 変更は 4 ファイルへの `#[cfg(test)] mod tests` 追記のみ（359 行追加 / 削除 0 行）。プロダクションコードの変更なし＝外部観測可能な挙動の変更なし（R5.1 充足）

## flaky

- AFTER 初回実行で既知フレーキー `wintf tests/ecs cue_performance_test::bench_pop_ready_empty_queue` が fail（10,000 回空 pop_ready 1.27ms > 閾値 1ms）。隔離再実行 2 回も fail したが、並行セルの cargo ビルドプロセス 2 件が稼働中（CPU 負荷 45%）であることを確認。ビルドプロセス終了を待機後の隔離再実行で 79 passed / 0 failed、続く全体再実行でも 1210 passed / 0 failed の安定合格 → パススルー判定。本テストは壁時計ベンチマーク（ハード閾値 1ms）のため並行ビルド負荷に敏感で、マシン負荷依存のフレーキー性が再確認された（境界外のため対処は行わず記録のみ）。

## proposals

- 新規提案なし（P27 以降の追記なし）。本セルで発見した改善点は、(a) 挙動非破壊で対応可能なもの（doc コメント乖離・dead code 整理 → W1-S へ申し送り、所見4/6/7）、(b) 非推奨モジュールの調査所見（winproc の健全性違反・削除セットの依存関係・steering 記載乖離 → 7.2 の削除判定へ申し送り）のいずれかであり、独立した新規仕様提案には該当しない。7.2 で winproc のハンドラ dispatch 経路を削除しないと判定した場合のみ、健全性修正（trait object 型混同 transmute の解消）の提案化を推奨する。
