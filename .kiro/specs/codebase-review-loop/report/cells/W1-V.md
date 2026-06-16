# W1-V: wintf レガシー・プロセス × 脆弱性レビューと非破壊対策

- status: completed
- commit: fix(W1): 脆弱性点検に基づく SAFETY/NOTE 注記・非発火 debug_assert・スレッドライフサイクル特性化テスト10件を追加

## findings

### 単一実行制御（process_singleton.rs）

1. **「ミューテックス」は存在せず、プロセス内 OnceLock シングルトン** — OS レベルの単一実行制御（名前付きミューテックス等によるプロセス間排他）は実装されておらず、`WIN_PROCESS_SINGLETON: OnceLock` によるプロセス内1回限りのウィンドウクラス登録（`wintf_window_class` / `wintf_ecs_window_class`）＋ DPI awareness 設定のみ。プロセス間排他は本フレームワークの責務外と判断（所見のみ・対策不要）。
2. **ウィンドウクラスの未登録解除はリークではない** — `RegisterClassExW` した2クラスは `UnregisterClassW` されないが、プロセス生存期間と一致するシングルトンのためプロセス終了時に OS が回収する（健全。所見のみ）。
3. **初期化クロージャの非冪等性（panic 回復不能 + 原因偽装）** — 1つ目のクラス登録成功後に2つ目が失敗して panic すると、`OnceLock::get_or_init` の仕様（panic 時未初期化のまま）によりクロージャが再実行され、1つ目の `RegisterClassExW` が ERROR_CLASS_ALREADY_EXISTS で 0 を返して誤誘導メッセージ（"Failed to register window class"）で再 panic する。panic メッセージは GetLastError を含まず診断不能。冪等化・診断改善は panic 挙動の変更を伴うため **P31** として記録し、NOTE を付記。
4. **`unsafe impl Send/Sync` は健全** — HINSTANCE はプロセス生存中不変のモジュールハンドル、HSTRING はアトミック参照カウントの不変文字列、構造体は初期化後読み取り専用。根拠を SAFETY コメントとして付記（従来は無根拠の unsafe impl だった）。
5. **`GetModuleHandleW(None).unwrap()` / `LoadCursorW.unwrap()`** — 自プロセスのモジュールハンドル取得と システム共有カーソルのロードは実用上不可謬（panic 経路として許容。診断メッセージ統一は P31 へ包含）。`SetProcessDpiAwarenessContext` の戻り値無視はマニフェスト設定済み環境での再設定失敗を許容する意図的なものと判断。

### Win32 API ラッパー境界（api.rs）

6. **SetLastError クリアパターンは正しい** — `get_window_long_ptr` は呼び出し前に ERROR_SUCCESS を設定してから判定するため、(a) GetWindowLongPtrW が正当に 0 を返すケースと失敗の曖昧性を解消し、(b) スレッドに残留した無関係なエラーコードによる成功の誤 Err 化を防ぐ（MS 推奨パターン準拠）。`set_window_long_ptr` は「戻り値 0 かつエラー設定あり」のみ Err とする非対称だが、これも前回値が正当に 0 のケースを扱う正しい形。**従来「成功経路は GUI 非依存では検証不能」とされていたが、`GetDesktopWindow()`（ウィンドウ生成不要・常に実在）で検証可能であることを確認し、成功経路テスト + 残留エラークリアの特性化テスト2件を追加**（api.rs:60-83）。
7. **整数変換は健全** — `isize` をそのまま受け渡し、スタイル値の `as u32` / `as _` 変換は呼び出し側（win_style/win_state、W1-T で特性化済み）の責務で、ラッパー自体に切り詰めなし。

### panic 経路・unsafe（winproc.rs — P28 影響範囲の深掘りを含む）

8. **P28（get_boxed_ptr 健全性違反）の影響範囲分析** — 実行時に本関数へ到達する経路は2系統であることをコールパス追跡で確定した。(1) **areka 本体・全 ECS 系 examples**: メッセージ専用ウィンドウ（`win_thread_mgr.rs::new`）は lpParam なしで生成され GWLP_USERDATA が null のまま → 常に None 返却で **UB コードは一切実行されない**。(2) **レガシー `create_window` 利用者 = examples/dcomp_demo.rs のみ**: 全メッセージ dispatch で型混同 + mutable transmute が実行される。現状「動作して見える」理由: ファットポインタ（データ + vtable）のビット列が格納時のまま保存され、誤型の中間参照 `&dyn WinMessageHandler` ではメソッドを呼ばず、最後の transmute で格納時の型へ戻ってから dispatch するため格納時の vtable が使われる。**ただしハンドラが同一ウィンドウへ同期送信（SendMessageW 等）すると wndproc が再入し、同一ハンドラへの `&mut` が2つ同時生存（エイリアスされた可変参照 = 即 UB）**。分析結果を NOTE として winproc.rs へ追記（修正自体は P27/P28 のとおり保留を維持）。
9. **wndproc の null ガードは網羅的** — WM_NCCREATE の CREATESTRUCT null チェック、get_boxed_ptr/from_boxed_ptr の null 早期 return により、ハンドラ未登録経路に panic・不正 deref なし。WM_NCDESTROY は GWLP_USERDATA を 0 クリアしてから解放するため二重解放なし（メッセージ処理は同一スレッドで直列）。健全なペア（into_boxed_ptr/from_boxed_ptr の格納・解放・参照カウント保存）と null 経路・DefWindowProc フォールバックを unit テスト6件で固定（UB 既知の get_boxed_ptr 非 null 経路はテストから意図的に実行しない）。
10. **WM_NCCREATE がハンドラの戻り値を無視して常に LRESULT(1) を返す** — ハンドラの WM_NCCREATE 拒否（None 以外）が生成中止に反映されない仕様だが、レガシー経路専用で利用者は dcomp_demo のみのため所見に留める（P27 削除セットで経路ごと消滅）。

### panic 経路・スレッドライフサイクル（win_thread_mgr.rs）

11. **VSync スレッドの HWND 越境と停止順序は健全** — HWND を isize として Send する手法は、`Drop` の「stop_flag 設定 → join → DestroyWindow」順序により join 完了までウィンドウが破棄されないため、スレッドからの PostMessageW の送信先は常に有効（破棄後送信は構造的に発生しない）。根拠を SAFETY コメントとして明文化し、`spawn_vsync_thread` 冒頭へ非発火 debug_assert（有効 HWND のみ渡る不変条件）、Drop へ join 不変条件の debug_assert を追加。**新規統合テスト `tests/thread_mgr.rs` 2件で new+drop の非ハング（join 完了）と多重生成の非 panic を特性化**。DwmFlush 失敗時は 15ms スリープでリトライする有界ループで panic なし。チャネル・unwrap は本モジュールに存在しない（スレッドクロージャ内も unwrap ゼロ）。
12. **CoInitializeEx に対応する CoUninitialize が Drop にない** — COM 初期化カウントがインスタンスごとに残置（単一インスタンス常駐運用では実害なし）。**P30** として記録、NOTE 付記。
13. **create_window 失敗時のハンドラ Box リーク** — 回収経路が WM_NCCREATE → WM_NCDESTROY のみのため、CreateWindowExW が WM_NCCREATE 送出前に失敗すると `into_boxed_ptr` の確保がリーク。エラー経路限定で実害は限定的。**P30** に包含、NOTE 付記。
14. **ECS_WORLD 束縛の初回固定（多重生成時の整合性侵害）** — `set_ecs_world` は OnceLock の `let _ = set(...)` で2回目以降を黙殺するため、2個目の WinThreadMgr の ECS ウィンドウは初代 world へ配信され、初代 drop 後は黙って DefWindowProc へフォールバック。複数スレッド多重生成では非アトミック Rc/Weak の跨スレッド upgrade（UB）に至る。areka 本体は単一インスタンス運用で現行実害なし。**P32** として記録、NOTE 付記、多重生成の非 panic を特性化テストで固定。
15. **静的カウンター群（VSYNC_TICK_COUNT 等）の Relaxed 操作は健全** — 単調増加カウンターの「変化したか」比較のみに使用され、データ依存を運ばないため Relaxed で十分（デバッグ統計も同様）。

### 非推奨モジュール（win_message_handler.rs）

16. **panic 経路なし** — 1,400 行を点検し unwrap/expect/transmute/生ポインタ deref ゼロ。unsafe は FFI 呼び出し（PostQuitMessage/PostMessageW/SendMessageW/TrackMouseEvent/DwmDefWindowProc/DefWindowProcW）のみで戻り値処理も妥当。モジュール全体が削除候補（P27）のため W1-S 方針を踏襲し一切手を入れず（所見のみ）。

### 適用した対策（全て挙動非破壊・追加のみ、R5.1 準拠）

| 種別 | 内容 |
|------|------|
| SAFETY コメント | process_singleton（Send/Sync 根拠）、win_thread_mgr（HWND 越境の生存期間保証） |
| NOTE コメント | process_singleton（P31: 非冪等初期化）、winproc（P28 影響範囲分析の追記）、win_thread_mgr ×3（P30: CoUninitialize 欠如・Box リーク、P32: ECS_WORLD 束縛固定） |
| 非発火 debug_assert | win_thread_mgr ×2（spawn_vsync_thread の有効 HWND、Drop の join ハンドル存在）— いずれも唯一の呼び出し経路から非発火を証明可能 |
| 特性化テスト +10 | api.rs +2（GetDesktopWindow 成功経路・残留エラークリア）、winproc.rs +6（null ガード・Box/Arc ラウンドトリップ・wndproc 3経路）、tests/thread_mgr.rs +2（new+drop 非ハング・多重生成非 panic） |

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace --no-fail-fast` **1209 passed / 0 failed**（親指示ベースラインと一致、既知フレーキー含め全合格）
- AFTER: `cargo build --workspace` 成功（警告 0）/ `cargo test --workspace --no-fail-fast` **1219 passed / 0 failed**（+10 は本セル追加テストのみ。既存テストの変更・削除ゼロ）
- プロダクションコードへの変更はコメント・debug_assert（リリースビルドでコード生成なし・非発火証明済み）のみで、実行経路のロジック変更ゼロ＝外部観測可能な挙動の変更なし（R5.1）

## flaky

- 既知フレーキー `wintf tests/ecs cue_performance_test::bench_pop_ready_empty_queue` は BEFORE / AFTER の両全体実行で合格（隔離再実行不要）。記録のみ。

## proposals

- **P30**（新規）: WinThreadMgr のリソース解放整備（CoUninitialize 欠如・create_window 失敗時のハンドラ Box リーク。P27 実施が先行すれば後者は不要）
- **P31**（新規）: WinProcessSingleton 初期化の部分失敗非冪等性と panic 診断の改善（ERROR_CLASS_ALREADY_EXISTS 許容 + GetLastError 付きメッセージ）
- **P32**（新規）: WinThreadMgr 多重生成時の ECS_WORLD 束縛固定の解消（単一インスタンス契約の明示 Err 化を推奨）
- 既存参照: **P28** の影響範囲分析を深掘りし NOTE へ追記（修正は引き続き保留・P27 優先の推奨を維持）、**P27** / **P29** は変更なし
