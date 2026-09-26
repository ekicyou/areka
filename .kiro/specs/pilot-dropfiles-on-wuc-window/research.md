# ギャップ分析: pilot-dropfiles-on-wuc-window

> 2026-09-26・`/kiro-validate-gap`。対象は確定済みの `requirements.md`（要件 1〜7）と、ブランチ `claude/pilot-dropfiles-wuc-window-f3f940`（main `13b72893` 相当）の実物のコード。
> §1〜§9 は「情報と選択肢」を出すギャップ分析で、決定は書かない。決めるべき点は §7「設計で決める事項」に番号を付けて並べ、設計段の決定は §10 の決定ログに書いた。
> 引用は「どのファイルの、何の定義か」で指す（行番号は使わない）。

## 1. 要約

- **落とし物の語彙はコードに 0 件**。`WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`IDropTarget`／`WS_EX_ACCEPTFILES`／`SetWindowSubclass`／`GWLP_WNDPROC` を `crates/` 全域で検索して一致なし（brief の 09-26 時点の記述を再確認）。実装はすべて新規だが、必要な Win32 API は `windows` 0.62.2 に揃っており、ワークスペースの機能（feature）も既に有効。**`crates/pilot/Cargo.toml` を変えずに書ける見込み**。
- **wintf の窓手続きには落とし物の腕が無く、外から差し込む口も無い**（`crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message` は表に無いメッセージを `None`＝既定手続きへ流す・`crates/wintf/src/runtime/message_loop.rs` の filter は常に転送で `pub(crate)`）。したがって example は **HWND を得た後に自分で窓手続きを重ねる**（`SetWindowSubclass` か `GWLP_WNDPROC` の差し替え）か、**スレッドのメッセージ取得フック**（`WH_GETMESSAGE`）で拾う。どちらも wintf を変えずに済む（§4 で 3 案を比較）。
- **`WS_EX_ACCEPTFILES` は `WindowStyle.ex_style` に足すだけで生成時に効く**。`crates/wintf/src/runtime/window_factory.rs` の `compute_ex_style` は `WS_EX_LAYERED` を落として `WS_EX_NOREDIRECTIONBITMAP` を足すだけで他のビットを通し、その後の `apply_initial_state` は `GWL_STYLE` しか触らない。クリック透過の付け外し（`crates/wintf/src/win_style.rs` の `apply_click_through`／`apply_layered_companion`）も自分のビット 1 つしか書き換えないので、**受け入れの宣言が付け外しで消える経路は静的には無い**（要件 4.4 の読み戻しは「無いことの確認」になる）。
- **UI スレッドは MTA**（`crates/wintf/src/runtime/mod.rs` の `WinApp::with_exit_policy` が `CoInitializeEx(COINIT_MULTITHREADED)`・WUC は `DQTAT_COM_NONE` で同じスレッドに乗る＝`crates/wintf/src/ecs/graphics/wuc_resource.rs`）。`WM_DROPFILES` の経路は COM のアパートメントに依らないので本坑には無関係だが、「違う」となったときの `IDropTarget` は `RegisterDragDrop` が `OleInitialize`（STA）を要求し、MTA のスレッドでは `RPC_E_CHANGED_MODE` になる。見立ての材料を §6 に置いた。
- 手本は 2 つ揃っている。**窓の建て方・透過機構への登録・上限時間つきの終了**は `crates/pilot/examples/pilot-balloon-asset-swap/main.rs`、**「透明な余白＋不透明な部分」を持つ最小の絵と当たり判定**は `crates/areka/examples/clickthrough_two_rects.rs`（矩形＝`HitTest` 既定の矩形判定・PNG＝`HitTest::alpha_mask()`）。規模は **S・リスクは低〜中**（未知は Win32 側の振る舞いだけで、それを測るのが本坑の目的）。

## 2. 現状の調査

### 2.1 落とし物に関係する既存資産

| 資産 | 場所 | 本坑での使い方 |
|---|---|---|
| 窓の宣言的生成 | `crates/wintf/src/ecs/window/components.rs` の `Window`／`WindowStyle`／`WindowPos`・`crates/wintf/src/runtime/window_factory.rs` の `EcsWindowFactory::create_window` | `WindowStyle { style: WS_POPUP\|WS_VISIBLE, ex_style: WS_EX_LAYERED\|WS_EX_TOOLWINDOW\|WS_EX_ACCEPTFILES }` を spawn する。`compute_ex_style` が LAYERED を落とし NOREDIRECTIONBITMAP を足す（要件 2.2 の「wintf に任せる分」）。 |
| 拡張スタイルの読み戻し | `crates/wintf/src/ecs/window/window_handle.rs` の `WindowHandle::get_style`（`GWL_STYLE`・`GWL_EXSTYLE` を返す公開メソッド） | 要件 2.6・4.4 の読み戻しにそのまま使える。 |
| クリック透過の機構 | `crates/wintf/src/ecs/clickthrough/`（`ClickThroughRegistryHandle::register`・`controller.rs` の `evaluate_targets`・`monitor.rs` のカーソル監視 12ms 周期）・`crates/wintf/src/win_style.rs` の `apply_click_through`／`apply_layered_companion` | `Added<WindowHandle>` の system で `register(entity, hwnd)`（要件 2.5）。切り替えのログは `debug!(?window, ?desired, "clickthrough: ex-style トグル適用")`＝**`RUST_LOG=…,wintf::ecs::clickthrough=debug` で見える**（要件 2.7）。`clickthrough_two_rects.rs` の既定フィルタ `"info,wintf::ecs::clickthrough=debug"` が先例。 |
| 当たり判定の問い合わせ | `crates/wintf/src/ecs/layout/hit_test/mod.rs` の `hit_test_in_window(world, window, client_point: PhysicalPoint) -> Option<Entity>`（公開） | 落とした位置が絵の不透明な所か透明な所かを、クリック透過と同じ判定器で判定できる（要件 3.2）。引数は**物理 px のクライアント座標**。 |
| 絵の部品 | `crates/wintf/src/ecs/widget/shapes` の `Rectangle`＋`brushes::Brushes`（矩形判定）・`crates/wintf/src/ecs/widget/bitmap_source` の `BitmapSource`＋`HitTest::alpha_mask()`（画素の α 判定） | 手本 `crates/areka/examples/clickthrough_two_rects.rs` の `spawn_rect`／`Image-alpha`。窓 entity は必ず `HitTest::none()`（さもないと窓全面が当たり＝永遠に不透過）。 |
| 上限時間つきの終了 | `crates/pilot/examples/pilot-balloon-asset-swap/main.rs` の `EXIT_ENV`（`AREKA_APP_SMOKE_EXIT_MS`）・`exit_ms_from`・`Run` リソース・`deadline_system`（窓を despawn → `WinApp` 既定の「最後の窓が閉じたら終了」で `run()` が戻る） | 要件 6 をほぼ写しで満たす。既定 90 秒はこの手本の値。 |
| 窓手続きへの配送 | `crates/wintf/src/runtime/wndproc_bridge.rs` の `make_wndproc` → `crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message`（表に無い種は `None` → ライブラリが `DefWindowProcW`） | **落とし物の腕は無く、差し込み口も無い**。example 側で受け取る方法は §4。 |
| メッセージループ | `crates/wintf/src/runtime/message_loop.rs` の `MessageLoopDriver::block_on`（ライブラリ `wintf-winmsg-executor` の `block_on`）。`MessageLoop::run(filter)` の filter はあるが wintf は `block_on` 経路を使い、filter は `pub(crate)` で常に転送 | filter 経由の横取りは wintf を変えないと届かない＝**不採用**。 |
| ライブラリ側の窓手続き | `wintf-winmsg-executor` 0.0.5 `src/util/window.rs`: `WM_NCCREATE` で `GWLP_WNDPROC` を型付きの手続きへ差し替え、`GWLP_USERDATA` に状態を置く。`WM_NCDESTROY` で状態を解放 | example が生成後に `GWLP_WNDPROC` を差し替えても（あるいは `SetWindowSubclass` で重ねても）ライブラリの差し替えは既に済んでいるので衝突しない。`GWLP_USERDATA` は**使ってはいけない**（ライブラリ占有）。 |
| Win32 API の所在（`windows` 0.62.2） | `Win32::UI::Shell`: `HDROP`・`DragQueryFileW(hdrop, i, Option<&mut [u16]>) -> u32`・`DragQueryPoint(hdrop, *mut POINT) -> BOOL`・`DragFinish`・`DragAcceptFiles`・`SetWindowSubclass`／`DefSubclassProc`／`RemoveWindowSubclass`・`SUBCLASSPROC`・`IsUserAnAdmin`。`Win32::UI::WindowsAndMessaging`: `WM_DROPFILES`（563）・`WS_EX_ACCEPTFILES`（0x10）・`ChangeWindowMessageFilterEx`・`MSGFLT_ALLOW`・`WindowFromPoint`・`SetWindowsHookExW`／`WH_GETMESSAGE` | **`Win32_UI_Shell`・`Win32_UI_WindowsAndMessaging` はワークスペース既定で有効**（ルート `Cargo.toml` の `windows` の features）。`WM_COPYGLOBALDATA`（0x0049）は定数が無いので数で書く。`RegisterDragDrop`／`OleInitialize` は `Win32::System::Ole`＝**未有効**（本坑は `IDropTarget` を作らないので不要）。 |
| 管理者かどうかの記録 | `IsUserAnAdmin`（shell32・`Win32_UI_Shell`） | 要件 5.2 を手書きでなくログで自動記録できる。 |

### 2.2 pilot crate の規約と依存

- `crates/pilot/Cargo.toml`: `[target.'cfg(not(target_arch = "x86"))'.dev-dependencies]` に `wintf`・`bevy_ecs`・`tracing`・`tracing-subscriber` が既にある。`windows`／`windows-core` は `[dependencies]` に既にあり、features は加算のみ。**本坑の実装に新しい依存は要らない見込み**（Rectangle・Brushes・BitmapSource はいずれも wintf の中）。
- `_template/main.rs` は `println!` 1 行の雛形。実質の手本は `pilot-balloon-asset-swap/main.rs`（`tracing_subscriber` の `EnvFilter`・`ExitReason`/`exit_code`・`boot_and_run`）。
- example のフォルダは spec 名と一致（`examples/pilot-dropfiles-on-wuc-window/`）。`main.rs` があれば Cargo が自動で example として拾う（`[[example]]` 宣言は不要・`shiori-host-32-helper` のようなサブファイルの別 entry を持たなければ）。
- 触るファイルは `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` の下だけ。並走 `ghost-shell-balloon-switch` との共有は 0（要件 1・Adjacent expectations と一致）。

### 2.3 スレッドと COM の実態

- UI スレッド＝`main` スレッド。`WinApp::with_exit_policy` が `CoInitializeEx(None, COINIT_MULTITHREADED)`（既に別モデルで初期化済みなら成功扱い）。WUC の `DispatcherQueue` は `DQTAT_COM_NONE`（MTA では ASTA が `RPC_E_CHANGED_MODE`）。`bitmap_source` の WIC 復号は MTA 前提で背景スレッドへ渡している（`crates/wintf/src/ecs/widget/bitmap_source/systems.rs` の注記）。
- `WM_DROPFILES` はエクスプローラ側の OLE が **窓のスレッドのキューへ投函する通常の窓メッセージ**で、受け手のアパートメントを問わない。したがって「MTA だから届かない」は起きない（本坑の未知は §3 の 3 点）。
- カーソル監視（`monitor.rs`）は別スレッドで `GetCursorPos` を 12ms 周期に読む。エクスプローラのドラッグ中もカーソル座標は取れるので、**ドラッグ中も透過の付け外しは動き続ける**はず（これ自体が観測項目ⓑ・ⓒの前提）。

## 3. 要件と資産の対応（ギャップ表）

凡例: **既存**＝写せば足りる／**新規**＝本坑で書く（既知の API）／**未知**＝Win32 の振る舞いで実験が答えを出す／**制約**＝守るべき既存の枷

| 要件 | 必要な物 | 資産 | 区分 |
|---|---|---|---|
| 1.1〜1.6 隔離 | フォルダ配置・依存を pilot の下に限る | `two-tunnel.md`・`crates/pilot/Cargo.toml` | 既存（制約） |
| 2.1〜2.2 本番と同じ窓 | `WindowStyle`（本番は `crates/areka/src/placement/spawn.rs` の `window_style()`＝`WS_POPUP\|WS_VISIBLE`／`WS_EX_LAYERED\|WS_EX_TOOLWINDOW`）に `WS_EX_ACCEPTFILES` を足す | `window_factory.rs` の `compute_ex_style` が他ビットを通す | 既存 |
| 2.3 `DragAcceptFiles` を呼ばない | 拡張スタイルだけで宣言 | — | 新規（1 ビット） |
| 2.4 透明＋不透明の絵 | 窓 `HitTest::none()`＋子の `Rectangle`（矩形判定）または `BitmapSource`＋`HitTest::alpha_mask()` | `clickthrough_two_rects.rs` | 既存 |
| 2.5 透過機構への登録 | `Added<WindowHandle>` で `ClickThroughRegistryHandle::register` | `pilot-balloon-asset-swap/main.rs` の `register_click_through` | 既存 |
| 2.6 拡張スタイルの読み戻しログ | `WindowHandle::get_style` → 各ビットの有無 | 公開メソッドあり | 既存 |
| 2.7 切り替えのログ水準 | `wintf::ecs::clickthrough=debug` を既定フィルタに含める | `clickthrough_two_rects.rs` の先例 | 既存 |
| 2.8 200% のまま検証 | 物理 px と論理 px の混在を避ける（`hit_test_in_window` は物理 px・`BoxStyle` の寸法は論理 px） | `controller.rs` の注記（`ScreenToClient` 委譲） | 制約 |
| 3.1 wintf を変えず受け取る | HWND への窓手続きの重ね掛け or スレッドのメッセージフック | **差し込み口なし** | 新規（§4 の 3 案） |
| 3.2 到着時刻・位置・不透明か・透過か | `DragQueryPoint`（クライアント座標）・`hit_test_in_window`・`GetWindowLongPtrW(GWL_EXSTYLE)` | 公開 API あり | 新規（組み合わせ） |
| 3.3 数とパス | `DragQueryFileW(hdrop, 0xFFFFFFFF, None)` で数・`DragQueryFileW(hdrop, i, None)` で長さ・バッファを渡して取得 | `windows` 0.62.2 | 新規 |
| 3.4 後片付け | `DragFinish(hdrop)` を必ず呼ぶ（失敗経路でも） | — | 新規 |
| 3.5 失敗を止めずにログ | `error!`＋続行（`logging.md`・ログ無し失敗経路の禁止） | steering | 新規 |
| 3.6 機械で選び出せる目印 | ログ本文に固定の語（例: `[dropfiles]`）を付ける。`logging.md` の「スコーププレフィックス」の作法 | steering | 新規 |
| 4.1〜4.3 絵の外・付け外し前後 | 手順の文書化（README）＋ⓑで「出ない行」を証拠にする | — | 新規（文書）＋未知 |
| 4.4 付け外し後の受け入れの宣言 | 到着時に `GWL_EXSTYLE` を読み戻す | `apply_click_through` は TRANSPARENT のみ・`apply_layered_companion` は LAYERED のみ書き換える | 既存（静的には消えない）＋確認 |
| 5.1〜5.3 手順と go の規則 | README | — | 新規（文書） |
| 5.2 管理者として起動したか | `IsUserAnAdmin` をログに出す | `Win32_UI_Shell` | 新規（1 行） |
| 5.4 手当て 3 種 | ①生成後に `SetWindowLongPtrW(GWL_EXSTYLE)` で付け直し／`DragAcceptFiles(hwnd, true)`・②透過の時機 | API あり。③`ChangeWindowMessageFilterEx` は要件討議（議題 2）で外した＝管理者起動は扱わない。切替は環境変数か起動引数で | 新規（分岐）＋未知 |
| 5.6 `IDropTarget` の見立て | 文書のみ（試作しない） | §6 | 新規（文書） |
| 6.1〜6.4 有界な終了 | `AREKA_APP_SMOKE_EXIT_MS`・既定値・終了理由と回数のログ | `pilot-balloon-asset-swap/main.rs` | 既存（既定値は要検討） |
| 7.1〜7.6 README 3 幕 | 動機・概要・検証結果 | `_template/README.md`・`pilot-balloon-asset-swap/README.md` | 既存（型） |

**未知（実験が答えを出す点）は 3 つに絞れる**: (a) `WS_EX_NOREDIRECTIONBITMAP`＋LAYERED（フラグのみ）の窓で、エクスプローラ側の当たり判定が `WM_DROPFILES` を投函するか、(b) `WS_EX_TRANSPARENT` が付いている瞬間は背後へ抜けるか・ドラッグ中に付け外しが追随してドロップ時点で正しい側に居るか（監視 12ms＋tick 1 回の遅れが効くか）。(c) 権限差（管理者起動）は要件討議（議題 2）で対象外とした（ふつうの権限で走らせ、管理者の走行は判定に使わない）。

## 4. 実装の選択肢（落とし物の受け取り方＝要件 3.1）

wintf を変えずに `WM_DROPFILES` を example の側で受け取る方法は 3 つある。いずれも「HWND が付いた後（`Added<WindowHandle>`）に UI スレッドで仕掛ける」点は同じ。

### 案 A: `SetWindowSubclass`（comctl32 の重ね掛け）

- `Added<WindowHandle>` の system で `SetWindowSubclass(hwnd, Some(proc), id, refdata)`。`proc` は `WM_DROPFILES` だけ処理し（`DragQueryPoint`→`DragQueryFileW`→ログ→`DragFinish`→`LRESULT(0)`）、他は `DefSubclassProc` へ流す＝ライブラリの型付き窓手続き（→ wintf の `dispatch_window_message`）に届く。
- 利点: Windows が連鎖を管理する公式の重ね掛け。`GWLP_USERDATA` に触らない（ライブラリ占有を侵さない）。`WM_NCDESTROY` でも comctl32 が片付ける。
- 欠点: `refdata`（`usize`）で状態を渡すか `thread_local!` で持つ。World へは直接触れないので、「不透明な所か」の判定は (i) 絵の幾何を定数で持って手続きの中で計算する、(ii) 受け取った記録を `thread_local` の待ち行列に積み、ECS の system が次の tick で `hit_test_in_window` を当ててログを出す、のどちらか（→ 設計事項 3）。
- **これが「窓手続きまで届いた」の直接の証拠になる**（要件 7.5 の学びに最も近い）。

### 案 B: `SetWindowLongPtrW(GWLP_WNDPROC)` の差し替え＋`CallWindowProcW`

- 旧来の重ね掛け。ライブラリは `WM_NCCREATE` で差し替えを終えているので、生成後の差し替えは安全。
- 利点: comctl32 に依らない・依存の追加なし（案 A も追加なし）。
- 欠点: 手で連鎖を持つ（旧手続きのポインタを保持）。`WM_NCDESTROY` でライブラリが状態を解放したあと自分の手続きへ戻す作法が要る。案 A に対して利点が薄い。

### 案 C: スレッドのメッセージ取得フック `SetWindowsHookExW(WH_GETMESSAGE, …, GetCurrentThreadId())`

- 投函されたメッセージを `GetMessageW` の直後・`DispatchMessageW` の前に覗く。`WM_DROPFILES` を見たらログ（`HDROP` は `lParam` に入っている）。
- 利点: 窓手続きに一切触れない。wintf のライブラリが `WH_MSGFILTER` フックを既に使っており（`wintf-winmsg-executor` の `msg_filter_hook.rs`）、フックの共存は問題ない。
- 欠点: **「窓のスレッドの待ち行列に届いた」ことは示すが「窓手続きに届いた」ことは直接示さない**（配送はその後）。`DragFinish` をどこで呼ぶかが曖昧（フックで呼ぶと配送先の `DefWindowProcW` は何もしないので二重解放はしないが、責務が二重になる）。`WM_DROPFILES` が投函（post）でなく送信（send）で届く経路があれば見えない。

**組み合わせ**: 案 A を本線にし、案 C を「届かない」ときの切り分けの補助（待ち行列に来ているのに窓手続きへ届かないのか、そもそも来ていないのか）に足す価値がある。切り分けの手当て（要件 5.4）と同様、環境変数で足し外しできる形が安い。

### 受け取りの後（要件 3.2〜3.6 の組み方）

- `DragQueryPoint(hdrop, &mut pt)`＝**クライアント座標（本プロセスは PMv2 なので物理 px）**と「クライアント領域の内か」の真偽。`hit_test_in_window(world, window, PhysicalPoint{pt})` へそのまま渡せる（透過機構と同じ判定器・同じ座標系）。
- `DragQueryFileW`: `iFile = 0xFFFF_FFFF` で本数、各 `i` で `None` を渡して必要長を得てから `Vec<u16>`（長さ＋1）で取得。0 が返ったら失敗＝`error!` して続行・最後に必ず `DragFinish`。
- 透過の状態は到着時の `GetWindowLongPtrW(GWL_EXSTYLE)`（`WindowHandle::get_style` でもよい）で `WS_EX_TRANSPARENT`／`WS_EX_ACCEPTFILES`／`WS_EX_LAYERED`／`WS_EX_NOREDIRECTIONBITMAP` の 4 ビットを 1 行に出す（要件 2.6・3.2・4.4 を同じ関数で満たせる）。
- 目印: `logging.md` のスコープ接頭辞の作法に合わせ、落とし物の行だけ固定の語（例 `[dropfiles]`）を本文の先頭に置き、構造化フィールドで `n`・`path`・`x`・`y`・`opaque`・`transparent`・`accept_files` を出す。

## 5. 実装の構え（extend／new／hybrid）

- **既存を写す（extend）**: 窓・登録・終了は `pilot-balloon-asset-swap/main.rs` の写し、絵は `clickthrough_two_rects.rs` の写し。ここに新規の判断は無い。
- **新規（new）**: 落とし物の受け口（§4）・手当ての切替（環境変数）・ログの目印・README。1 ファイル（`main.rs`）か、受け口だけ `dropfiles.rs` に分けるか（`pilot-balloon-asset-swap` は役割ごとに分けた。使い捨てなので 1〜2 ファイルで足りる）。
- **hybrid**: 実質これ。規模は **S（要件どおり 1〜3 タスク）**。

**リスク: 低〜中**。技術は既知（Win32 の古い API）で、未知は §3 末尾の 3 点だけ。それを測るのが本坑なので「分からない」は失敗ではない。落とし穴として起こり得るのは、(1) 200% で論理 px と物理 px を混ぜて「不透明な所」の判定が表示とずれる（`controller.rs` の注記どおり `ScreenToClient`／`DragQueryPoint` の物理 px を使えば避けられる）、(2) 窓 entity に `HitTest::none()` を付け忘れて窓全面が当たり＝透過が一度も付かず、ⓑが測れない、(3) `DragFinish` を失敗経路で忘れる、の 3 つ。

## 6. `IDropTarget` へ倒す場合の見立て（要件 5.6・試作はしない）

- `RegisterDragDrop(hwnd, target)` は呼び出しスレッドが `OleInitialize` 済み（＝STA）であることを要求する。wintf の UI スレッドは `COINIT_MULTITHREADED` で初期化済みなので `OleInitialize` は `RPC_E_CHANGED_MODE` で失敗する。ここが brief の言う衝突。
- 置き場所の候補（設計段で比較する材料。本坑では文書化のみ）:
  1. **UI スレッドを STA にする**: `WinApp` の初期化を `COINIT_APARTMENTTHREADED` へ。WUC は STA でも動く（`DQTAT_COM_ASTA`／`DQTAT_COM_STA` の道がある・`crates/wintf/src/com/wuc.rs` の注記）が、WIC の背景復号（`bitmap_source` が MTA 前提）と `crates/areka-emo-*` の `CoInitializeEx(COINIT_MULTITHREADED)` 呼び出し群の見直しが要る＝広い変更。
  2. **STA の別スレッドに受け口の窓を置く**: ゴースト窓と同じ位置・寸法に透明な受け口の窓を重ね、`RegisterDragDrop` はそのスレッドで行う。重なり順・クリック透過の付け外しとの二重管理・DPI 追従が新たに要る＝複雑。
  3. **`WM_DROPFILES` で行けるところまで行く**: `IDropTarget` が要るのはテキスト・URL の投げ込み（α 後）なので、α の `.nar` は `WM_DROPFILES` で足りる、という切り分け。本坑が go なら本坑 `areka-P0-ghost-install` はこの線。
- 「違う」の判定になったとき README に書くのは 1〜3 の比較と、どれが `areka-P0-ghost-install` の規模に収まるかの見立てまで。

## 7. 設計で決める事項（要件ディスカッションへ）

1. **受け取りの仕掛け**: 案 A（`SetWindowSubclass`）を本線にするか。案 C（`WH_GETMESSAGE`）を切り分け用に同梱するか（環境変数で足し外し）。
2. **絵の作り**: 子の `Rectangle`（矩形判定・ファイル不要・最短）か、`BitmapSource`＋`HitTest::alpha_mask()`（本番と同じ画素の α 判定・PNG が要る＝`sample-ghost-kit` の検体から引くか、`crates/areka/shell/base.png` を読むか）。要件 2.4 はどちらでも満たす。本番との一致を重く見るなら後者、切り分けの単純さを重く見るなら前者。
3. **「不透明な所か」の判定の場所**: 窓手続きの中で幾何を定数で計算するか、記録を待ち行列に積んで ECS の system が `hit_test_in_window` で判定してログを出すか（透過機構と同じ判定器を使うのは後者）。
4. **手当て（要件 5.4）の切替の形**: 環境変数（例 `PILOT_DROPFILES_FIX=reapply|dragaccept`）か起動引数か。手当てなしの素の走行が既定であること。
5. **既定の上限時間**: 手本の 90 秒か、手でⓐ〜ⓒを 1 走行で試す余裕（透過を何度か付け外し→落とす、を 3 回）を見て 120〜180 秒か。あるいはⓐ〜ⓒを別々の走行にして 90 秒のままか。
6. **ログの目印の語**: `[dropfiles]` 等の 1 語と、構造化フィールドの名前（`n`・`path`・`x`・`y`・`opaque`・`transparent`・`accept_files`・`admin`）。README の grep 例に同じ語を書く。
7. **走行の分け方**: ⓐ〜ⓒと手当て 3 種を 1 走行で追うか、走行ごとに 1 項か（ログの読み取りの容易さと上限時間に関わる）。
8. **管理者判定の記録**: `IsUserAnAdmin` を起動時に 1 行出す（要件 5.2＝管理者の走行を判定から外すための目印）。

## 8. 研究が要る点（設計段で確かめる・Research Needed）

- **R1**: `WM_DROPFILES` の届き方は投函（`PostMessage`）で確定か。案 C（`WH_GETMESSAGE`）の有効性はこれに依る（送信なら `WH_CALLWNDPROC` になる）。案 A なら無関係。
- **R2**: エクスプローラ側の落とし先の探索（OLE の `WindowFromPoint` 相当）が `WS_EX_TRANSPARENT` の窓を飛ばすことは、クリック透過が別プロセスで効いている事実（`pilot-clickthrough-alpha-toggle/REPORT.md`）から見込めるが、**ドラッグ中に付け外しが起きたときの再標的化（DragLeave→DragEnter）の追随**は実測しかない。監視 12ms＋tick 1 回の遅れが「絵の縁で落としたときの取りこぼし」になるかを観測項目に含めるか。
- **R3**: `WS_EX_ACCEPTFILES` の窓に落としたとき、OLE の既定の受け口が `WM_DROPFILES` に変換して投函するのは Windows 11 でも同じか（`DragAcceptFiles` を呼ぶ手当てとの差は無いはず＝`DragAcceptFiles` はビットの付け外しだけ）。
- ~~**R4**: 管理者起動のときの遮断（UIPI）を通す `ChangeWindowMessageFilterEx` の組~~ → 要件討議（議題 2）で対象外（areka を管理者で動かすことは普通は無い）。
- **R5**: `SetWindowSubclass` で重ねた手続きと、ライブラリの `WM_NCDESTROY` の状態解放（`Box::from_raw`）の順序。comctl32 は `WM_NCDESTROY` で自動的に重ね掛けを外すので問題は無い見込みだが、終了時の警告ログが出るかは実走で見る。

## 9. 次の段へ

- 本書の §7 を要件ディスカッションの議題にする（答えで作業が変わるものだけ＝1・2・3・5 が主。4・6・7・8 は how なので設計で決めて結果だけ報告でよい）。
- 設計は `pilot-balloon-asset-swap` の design の型（Runner・終了の理由・環境変数）を写し、受け口だけを新しく描けば足りる。
- 本坑 `areka-P0-ghost-install` の design が参照するのは README の検証結果だけ。ここで書いた API の所在（§2.1 の表）は設計へ写してよいが、検証結果は README に一本化する（`two-tunnel.md` の「二重化しない」）。

## 10. 設計段の調査と決定ログ（2026-09-26・`/kiro-spec-design`）

> §1〜§9 はギャップ分析（決定を書かない文書）。本節から先は設計段の記録で、§7 の各項に対する**決定**とその理由を書く。結論は `design.md` の Key Decisions に転記済み（design.md は単体で読める）。

### 10.1 調査の範囲と要点（Discovery: light＝既存の先進坑の型の拡張）

- **Feature の分類**: 既存の先進坑 `pilot-balloon-asset-swap` の器（窓・透過機構への登録・上限時間）の写しに受け口を 1 つ足す拡張。外部の新しい依存は無く、Web 調査は要らない。未知は Win32 の振る舞い（§3 末尾の (a)(b)）で、それを測るのが本坑。
- **引用した定義の実在を再確認**（2026-09-26・ブランチ `claude/pilot-dropfiles-wuc-window-f3f940`）:
  - `crates/pilot/examples/pilot-balloon-asset-swap/main.rs`: `EXIT_ENV`／`DEFAULT_EXIT_MS`／`exit_ms_from`／`ExitReason`／`exit_code`／`Run`／`register_click_through`／`deadline_system`／`boot_and_run`。
  - `crates/areka/examples/clickthrough_two_rects.rs`: 窓の `HitTest::none()`・`spawn_rect`（`Rectangle::new()`＋`Brushes::with_foreground`＋絶対配置の `BoxStyle`）・既定フィルタ `"info,wintf::ecs::clickthrough=debug"`・`register_click_through_windows`。
  - wintf: `hit_test_in_window(world, window, client_point: PhysicalPoint)`（`crates/wintf/src/ecs/layout/hit_test/mod.rs`・`PhysicalPoint` は同ファイルで `PointF` の別名・`PointF::new(f32, f32)` は `crates/wintf/src/ecs/types.rs`）／`WindowHandle::get_style`（`crates/wintf/src/ecs/window/window_handle.rs`）／`ClickThroughRegistryHandle::register(window, hwnd)`（`crates/wintf/src/ecs/clickthrough/controller.rs`）／`compute_ex_style`・`apply_initial_state`（`crates/wintf/src/runtime/window_factory.rs`）／`apply_click_through`・`apply_layered_companion`（`crates/wintf/src/win_style.rs`）／`dispatch_window_message`（`pub(crate)`・`crates/wintf/src/ecs/window_proc/mod.rs`）／`WinApp::new`・`with_exit_policy`（`COINIT_MULTITHREADED`・`crates/wintf/src/runtime/mod.rs`）。
  - areka 本番の窓の様式: `crates/areka/src/placement/spawn.rs` の `window_style`＝`WS_POPUP | WS_VISIBLE`／`WS_EX_LAYERED | WS_EX_TOOLWINDOW`（`WS_EX_TOPMOST` なし）。
  - `windows` 0.62.2（`c:\rust\cargo\registry` の実物）: `Win32::UI::Shell` に `SetWindowSubclass`／`DefSubclassProc`／`RemoveWindowSubclass`／`SUBCLASSPROC`／`DragAcceptFiles`／`DragFinish`／`DragQueryFileW(hdrop, u32, Option<&mut [u16]>) -> u32`／`DragQueryPoint(hdrop, *mut POINT) -> BOOL`／`IsUserAnAdmin`。`Win32::UI::WindowsAndMessaging` に `WM_DROPFILES`（563）／`WS_EX_ACCEPTFILES`（0x10）／`GetWindowLongPtrW`／`SetWindowLongPtrW`／`SetWindowsHookExW`／`WH_GETMESSAGE`。両モジュールはルート `Cargo.toml` の `[workspace.dependencies.windows]` の features `Win32_UI_Shell`・`Win32_UI_WindowsAndMessaging` で有効＝**`crates/pilot/Cargo.toml` の変更は不要**（§2.2 の見込みを確定）。
  - `wintf-winmsg-executor` 0.0.5 `src/util/window.rs`: `WM_NCCREATE` で `GWLP_WNDPROC` を差し替え `GWLP_USERDATA` に状態を置く・`WM_NCDESTROY` で `Box::from_raw` で解放（R5 の前提を確認）。
- **R1（`WM_DROPFILES` は投函か）**: Windows の落とし物の受け口（OLE がシェルの `CF_HDROP` を `WM_DROPFILES` に直して届ける経路）は窓のスレッドの待ち行列へ投函する経路で、設計は投函前提（案 A なら投函でも送信でも受ける）。同期に送られる経路があれば `try_borrow` の失敗＝`opaque=unknown` の行として実走で見える（設計はそれを R1 の観測に兼ねる）。
- **R3（`DragAcceptFiles` との差）**: `DragAcceptFiles` は `WS_EX_ACCEPTFILES` の付け外しだけ。差は無いはずで、手当て `dragaccept` はそれを確かめる走行になる。
- **R2（ドラッグ中の付け外しの追随）**: 実測のみ。ⓑ・ⓒの観測項目に含めた。「絵の縁で落としたときの取りこぼし」は今回の手順に含めない（矩形の縁から離れた所に落とす）。

### 10.2 統合（synthesis）

- **一般化**: しない。受け口は `WM_DROPFILES` 1 種・窓 1 枚。本坑が要る「窓手続きの表への 1 分岐」の形は学びとして README に書くだけ。
- **作るか採るか**: 重ね掛けは comctl32 の `SetWindowSubclass`（OS 標準）を採る。自前の連鎖（`GWLP_WNDPROC` の差し替え）は作らない。パスの取り出しは shell32 の `DragQueryFileW` そのまま。
- **簡略化**: 観測用の待ち行列・ECS の system を介した判定・スレッドのメッセージフック（案 C）を外した。ファイルは `main.rs`＋`dropfiles.rs` の 2 つ。終了コードは 0／2 の 2 値（この example には「完了」が無く、上限時間で終わるのが正常）。

### 10.3 決定ログ（§7 の 8 項）

#### 決定 1: 受け口は `SetWindowSubclass`（案 A）1 本・案 C は同梱しない
- **選択肢**: A `SetWindowSubclass`／B `GWLP_WNDPROC` 差し替え＋`CallWindowProcW`／C `WH_GETMESSAGE` フック／A＋C 同梱。
- **選択**: A のみ。
- **理由**: A は連鎖を OS が管理し `GWLP_USERDATA` に触らず、「窓手続きまで届いた」の直接の証拠になる（要件 7.5）。C は「待ち行列に来たか」しか示さず `DragFinish` の責務が二重になる。ⓐで届かないと分かった時だけ足せば足りる（先に作るのは使わない可能性のある部品）。B は A に対して利点が無い。
- **確認事項**: 終了時（`WM_NCDESTROY`）の連鎖の順で警告が出ないか（R5）。

#### 決定 2: 絵は窓 `HitTest::none()`＋不透明な `Rectangle` 1 つ
- **選択肢**: `Rectangle`（矩形判定・ファイル不要）／`BitmapSource`＋`HitTest::alpha_mask()`（本番と同じ α 判定・PNG が要る）。
- **選択**: `Rectangle`。窓 320×320・矩形 (100,100) の 120×120（論理 px）。
- **理由**: 落とし先を決めるのは OS で、見るのは `WS_EX_TRANSPARENT` のビットだけ。矩形か α かは wintf の中の判定方式で、実験の答えに効かない（α 判定そのものは `clickthrough_two_rects.rs` で既に別に確かめられている）。矩形は境目がはっきりし、200% でも余白が各辺 200 物理 px 取れる。
- **代償**: 「本番と同じ α の絵で測った」とは言えない。README の学びに「判定方式は結果に効かない理由」を 1 行書く。

#### 決定 3: 「絵の上か外か」は到着の場で `hit_test_in_window` に聞く（`try_borrow`）
- **選択肢**: (i) 窓手続きの中で幾何を定数で計算／(ii) 記録を待ち行列に積み ECS の system が次の tick で判定／(iii) 受け口が World の取っ手（`Rc<RefCell<EcsWorld>>`）を文脈として持ち、到着の場で `try_borrow` して判定。
- **選択**: (iii)。
- **理由**: 透過機構と同じ判定器・同じ座標系（物理 px のクライアント座標＝`DragQueryPoint` の値をそのまま渡せる）で、要件 4 の「クリックの当たり判定と一致するか」を直接測れる。(i) は 200% で論理 px と物理 px を混ぜる罠（§5 の落とし穴 (1)）。(ii) は行が 1 tick 遅れ、待ち行列と system が 1 つずつ増える。窓手続きが `Rc<RefCell<EcsWorld>>` を借りるのは wintf 自身の `dispatch_window_message` と同じ作法。
- **代償**: 借りられない瞬間（tick の途中の同期配送）は `opaque=unknown`。それ自体が R1 の観測になる。

#### 決定 4: 手当ての切替は環境変数 `PILOT_DROPFILES_FIX`（未設定＝手当てなし）
- **選択肢**: 環境変数／起動引数。
- **選択**: 環境変数（`reapply`／`dragaccept`・未知の値は `warn!` して手当てなし）。
- **理由**: 上限時間（`AREKA_APP_SMOKE_EXIT_MS`）と同じ形で揃い、引数の解釈を書かずに済む。「透過の付け外しの時機」は手順（ⓐ／ⓒ）で観測する項目なので切替にしない。

#### 決定 5: 既定の上限時間は 180 秒
- **選択肢**: 90 秒（手本のまま・ⓐ〜ⓒを別走行に）／120〜180 秒（1 走行でⓐ〜ⓒ）。
- **選択**: 180 秒・1 走行でⓐ〜ⓒ。
- **理由**: 手で 3 回落とし、ⓒの前にカーソルを出し入れする余裕。短くしたいときは環境変数で上書きできる。

#### 決定 6: ログの目印は `[dropfiles]`
- **選択**: 落とし物に関する行はすべて本文の先頭に `[dropfiles]`。フィールド名は `seq`・`x`・`y`・`in_client`・`opaque`（`true`／`false`／`unknown`）・`transparent`・`accept_files`・`layered`・`noredirect`・`n`・`i`・`path`・`admin`・`fix`・`limit_ms`・`drops`・`when`・`raw`。
- **理由**: `logging.md` のスコープ接頭辞の作法。README の grep 例に同じ語を書く。

#### 決定 7: ⓐ〜ⓒは 1 走行・手当ては走行ごとに 1 種
- **選択肢**: 全部 1 走行／項ごとに 1 走行／ⓐ〜ⓒ 1 走行＋手当ては別走行。
- **選択**: ⓐ〜ⓒ 1 走行（既定の手当てなし）＋手当ては環境変数を変えて別走行。
- **理由**: 到着の行は `seq`・位置・`opaque` で自己記述的なので 1 走行のログで読み分けられる。手当ては窓の生成時に効くものなので走行を分けないと比べられない。

#### 決定 8: 管理者判定は起動時に 1 行（`IsUserAnAdmin`）
- **選択**: `info!(admin, fix, limit_ms, "[dropfiles] 起動")`。
- **理由**: 要件 5.2（管理者の走行を判定から外す目印）。要件討議（議題 2）で管理者起動の手当ては対象外。

#### 付随の決定
- **終了コードは 0（上限時間で終了＝正常）／2（初期化の失敗）**。手本の 1（打ち切り）・3（較正不合格）はこの example に該当が無い。
- **到着の時刻は tracing の行の時刻**で足りる（行は受け口の中で同期に出る）。
- **透過の状態は到着時の `GWL_EXSTYLE`**。落とした瞬間から配送までの数 ms に監視（12ms 周期）が付け外す可能性は README に明記する。
- **文脈の `Box` はプロセスの終わりまで生かす**（`WM_NCDESTROY` で外さない・使い捨て）。
- **`crates/pilot/Cargo.toml` は変えない**（10.1 で確定）。

### 10.4 リスクと備え

- `try_borrow` の失敗が頻発する → 投函でなく同期配送の経路がある証拠。行に `opaque=unknown` が残るので見落とさない。必要なら (ii) の待ち行列へ切り替える。
- 窓が非最前面でエクスプローラに隠れる → 手順に「絵をクリックして前に出す」を書く。`WS_EX_TOPMOST` は本番に無いので付けない（要件 2.2）。
- 窓 entity の `HitTest::none()` を忘れる → 窓全面が当たりで透過が一度も付かず、ⓑが測れない。生成直後の `ex-style` の行と `clickthrough` の debug 行が出ないことで気付ける。
- `DragFinish` を失敗経路で忘れる → `handle_drop` の最後に無条件で呼ぶ形にする（設計で固定）。
