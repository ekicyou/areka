# ギャップ分析: areka-P0-app-lifetime-separation

> 2026-09-23・本ブランチ（main `637799d7` 相当）で実測。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 本文書は分析と選択肢を示すもので、最終決定は要件ディスカッションと設計フェーズで行う。

## 1. 分析サマリー

- **終了の決め手は wintf に 1 か所だけ在り、そこへ「選択」と「明示の指示」を足す形が最短。** `crates/wintf/src/runtime/mod.rs` の `WinApp::new` が `fn wire_shutdown_hook` で登録表（`WindowRegistry`）の空遷移フックに `Event::notify` を仕込み、`WinApp::run` は `ShutdownPolicy::shutdown_future` の完了を `MessageLoopDriver::block_on` で待つだけである。フックを仕込むか否かを構築時に選べれば要件 2.1〜2.4 は成立し、`Event` を外から鳴らす口を足せば要件 2.5〜2.6 が成立する。
- **areka 側の終了 4 か所はどれも `&mut World` しか持たない。** `WinApp` の参照は `main` と `wire_emo2_boot` にしか無い。ゆえに「明示の終了の指示」は World から届く形（NonSend リソース）でなければ、4 か所へ配線を通す手間が増える。wintf には既に `ClickThroughRegistryHandle`（`fn wire_click_through` が World へ挿す NonSend）という同型の先例がある。
- **要件 1.4／1.5（二重指示・ループ開始前の指示）は今の `Event` だけでは満たせない。** `event-listener` 5.4.2 の仕様は「リスナが居ない時の通知は失われる」（`lib.rs` のクレート doc: "If there are no active listeners at the time a notification is sent, it simply gets lost."）。`message_loop.rs` の `notify_shutdown` の doc も同じ制約を書いている。ゆえに「指示済み」を覚える 1 ビットが要る。
- **「終了のための全窓破棄」と「終了の指示」を 1 つの操作にまとめる置き場は、新しい小さなモジュールが自然。** 既存の全窓破棄は 2 関数に分かれている（`crates/areka/src/placement/spawn.rs` の `despawn_ghost_windows`＝ゴースト窓のみ・`crates/areka/src/main.rs` の `despawn_smoke_targets`＝ダミー窓＋ゴースト窓）。`placement` は example の `#[path]` include のため `crate::` パスを書けず（`open_startup_window` 内のコメントに明記）、`main.rs` は 958 行で目安の 1,000 行に近い。
- **並走 spec `areka-P0-baseware-root-layout` との共有は `main.rs` と `tests/smoke_boot_loop_exit.rs` の 2 ファイルで、関数は別。** 本仕様が終了関連コードを `main.rs` から新モジュールへ動かせば、相手が後で触る面積も `main.rs` の行数も減る。ただし相手の brief は「引数なし起動の smoke テストを陳腐化として除くか更新するか」を要件で決めるとしており、本仕様の要件 5.4 が同じテストを常設の見張りとして残す前提と**すれ違う可能性がある**（§6 の議題 9）。

規模 **S**（brief の見立て 5〜7 タスクと一致）・リスク **低〜中**（未知は「終了時に残った窓を誰が壊すか」と「メッセージループを回さない `run()` の決定論テストが可能か」の 2 点）。

## 2. 現状調査（実測）

### 2.1 wintf: 寿命を決めている場所

| 定義 | ファイル | 役割（実測） |
|---|---|---|
| `struct WinApp { world, shutdown: Rc<event_listener::Event> }` | `crates/wintf/src/runtime/mod.rs` | 終了シグナルの唯一の所有者。引数を取らない `new()` のみ。構築時の設定値の器は無い |
| `fn wire_shutdown_hook` | 同上 | `ProdWindowRegistry`（NonSend）が無ければ挿し、`set_shutdown_hook(move \|\| signal.notify(usize::MAX))` を仕込む。**ここが「窓 0 で終了」の唯一の起点** |
| `fn run` | 同上 | `wire_new_path`（`reconcile_window_registry` を `FrameFinalize` へ登録）→ VSync 橋・tick・クリック透過の起動 → `MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(shutdown))` → 復帰後に `ShutdownPolicy::notify_shutdown`（念押し）→ `Ok(())` |
| `fn reconcile_window_registry` | `crates/wintf/src/runtime/window_registry.rs` | `RemovedComponents<Window>` を消化し、**1 件以上除去して結果が空のときだけ**フックを呼ぶ。フックが `None` なら何もしない |
| `WindowRegistry::set_shutdown_hook` / `fire_shutdown_hook`（`cfg(test)`） | 同上 | フックは差し替え可能な `Option<Box<dyn Fn()>>` |
| `ShutdownPolicy::shutdown_future` | `crates/wintf/src/runtime/message_loop.rs` | `event.listen()` を先に立ててから `await`。**通知の記憶は無い**（`Event` の仕様） |
| `MessageLoopDriver::block_on` | 同上 | ライブラリ `wintf-winmsg-executor` 0.0.5 の `pub fn block_on` へ委譲。future 完了で `msg_loop.quit()`。**既に完了している future を渡しても正常に戻る**（同ファイルの `block_on_ready_future_returns_value` が実証） |
| `struct App { window_count, … }` | `crates/wintf/src/ecs/app.rs` | 旧経路の窓数カウンタ。`on_window_destroyed` が 0 で `info!("[App] Last window closed.")` を出すだけで、終了は駆動しない（doc に「終了は `WindowRegistry` の空遷移が駆動する」と明記） |

窓が消える経路（実測）: `world.despawn(窓)` → `on_window_handle_remove`（`crates/wintf/src/ecs/window/window_handle.rs`）が `PostMessageW(WM_CLOSE)` → 同 tick の `FrameFinalize` で `reconcile_window_registry` が `Window<WndState>` を drop → `DestroyWindow`。ウィンドウ手続き（`crates/wintf/src/runtime/wndproc_bridge.rs`）は World を `try_borrow` で借り、借用中なら安全に読み飛ばす。ゆえに **World を借りたまま登録表を空にしても再入で壊れない**（reconcile が tick の中でまさにそうしている）。

### 2.2 areka: 終了操作 6 種の合流点 4 か所

| 場所 | 持っている参照 | 今日の振る舞い |
|---|---|---|
| `fn run_ghost_quit_phase`（`crates/areka/src/emo2_boot/frame.rs`）。`emo2_frame_system`（`Update` 段）の**先頭**で呼ばれる | `&mut Emo2Wiring, &mut World` | `KanadeStopped` を全件取り出し、`info!(event="ghost_quit", cause)` の上で `despawn_ghost_windows` を 1 度呼ぶ。窓が 0 なら `debug!(event="ghost_quit_no_windows")`。メニューの「終了」・別れの台詞のあと・中断のあと、の 3 操作がここへ集まる |
| `fn on_char_pointer_pressed` の強制退避の腕（`crates/areka/src/input_events/mod.rs`） | `&mut World` | `info!(event="mouse_escape_close")` → `despawn_ghost_windows` → `true` |
| `fn on_dummy_pressed`（`crates/areka/src/main.rs`） | `&mut World` | `DummyWindowMarker` を持つ entity を全部 `despawn` |
| `fn open_startup_window` 内の smoke クロージャ（`crates/areka/src/main.rs`） | `Weak<RefCell<EcsWorld>>`（`spawn_local` タスク） | `async_io::Timer` の後、`despawn_smoke_targets`（`Or<(With<DummyWindowMarker>, With<GhostWindowMarker>)>`）を呼ぶ |

- `despawn_ghost_windows`（`crates/areka/src/placement/spawn.rs`）は `pub(crate)`・呼び手 2 つ（上の 1・2 番目）。`placement` モジュールは `crate::` パスを書けない（`open_startup_window` 内のコメント「placement は `crate::` パスを持てない（example の `#[path]` include で成立させるため）」）。
- `despawn_smoke_targets`（`main.rs`）はダミー窓も狙う。`DummyWindowMarker` は `main.rs` に定義された `pub struct`。
- `WinApp` を持つのは `fn main` と `wire_emo2_boot(app: &WinApp, …)`（`crates/areka/src/emo2_boot/mod.rs`）だけ。4 か所へ `&WinApp` を配るには `Emo2Wiring`・ポインタハンドラ・smoke クロージャの 3 経路を別々に通す必要があり、World 経由の 1 経路より配線が多い。

### 2.3 `fn main` の後始末（`crates/areka/src/main.rs`）

`app.run()?` の後: ① `loop_ticker.send(TickerMsg::Close)` → ② `runtime.shutdown(CloseReason::User { scope: 0 })`（失敗は `error!`＋`E_FAIL` を返す）→ ③ `seriko.join()`（失敗は同様）→ ④ `perf_report.stop_and_report_final()` → `Ok(())`＝終了コード 0。`app`（`WinApp`）は関数末で drop され、`world` → `WindowRegistry` → `Window<WndState>` の drop で残った窓は `DestroyWindow` される。**この順序は本仕様で変えない**（要件 1.2・3.8）。

### 2.4 wintf 単体の利用者（既定を変えない根拠）

`crates/wintf/examples/` の 7 本（`clip_demo`・`dcomp_demo`・`dcomp_taffy_demo`・`graphics_reinit_test`・`postmessage_click_test`・`taffy_flex_demo/main`・`typewriter_demo`）はいずれも `WinApp::new()?` → `mgr.run()` の形で、終了は `world.despawn(window)` の結果に任せている。終了の指示に当たる呼び出しは 0 件。加えて wintf 外にも同じ形の利用者が 7 本ある（`crates/areka/examples/` の 5 本・`crates/areka-emo-text/examples/` の 2 本）。`WinApp::new()` の意味を変えない限り、この 14 本は 1 文字も触らずに済む。

### 2.5 既存の決定論テスト（落とさない対象・要件 4.4）

| 置き場 | 何を固定しているか | 本仕様への影響 |
|---|---|---|
| `crates/wintf/src/runtime/mod.rs` の `tests`（7 本） | `new()` がフックを Event へ結ぶこと・`wire_new_path`・実 HWND での close→reconcile→通知の貫通・構築直後は未通知（`new_owns_unfired_shutdown_signal`） | 既定（窓 0 で終了）を保てば全て緑のまま。`new_owns_unfired_shutdown_signal` は「指示済み」ビットを足しても意味が変わらない |
| `crates/wintf/src/runtime/window_registry.rs` の `tests`（3 本） | 空遷移ちょうどでのみフックが鳴る | 触らない |
| `crates/wintf/src/runtime/message_loop.rs` の `tests`（4 本） | `block_on` が完了済み future で戻る・通知済み Event で future が完了する | 触らない。`block_on_ready_future_returns_value` は要件 1.5 の土台 |
| `crates/wintf/tests/win_app.rs`（2 本） | `new()` の headless 構築 | 触らない |
| `crates/areka/src/emo2_boot/frame_ghost_quit_tests.rs`（4 本） | 通知 1 件で全ゴースト窓が閉じ記録 1 行・2 件目は打ち切り・無通知は無操作・**窓 0 でも `ERROR` を出さない** | 素の `World` で相を直に叩く。終了の指示の受け口が World に無い状態でも走るため、受け口不在時の記録レベルの決め方に効く（§6 議題 6） |
| `crates/areka/src/input_events/input_events_tests.rs` の強制退避 3 本 | Ctrl+Shift+左で全ゴースト窓が消え kanade へ送らない・結線無しでも退避できる・退避以外は 1 枚も消さない | 素の `World`。同上 |
| `crates/areka/src/main_startup_window_tests.rs`（`double_click_left_despawns_all_dummy_windows` ほか） | ダミー窓のダブルクリック despawn | 素の `World`。同上 |
| `crates/areka/src/main_seam_tests.rs` の `despawn_smoke_targets_*`（3 本） | 標的の種類・空 World の no-op・連鎖破棄済み標的の打ち切り | 関数を動かすなら追随（テスト本文は不変で置き場だけ変わる） |
| `crates/areka/tests/smoke_boot_loop_exit.rs`（2 本） | 実プロセスで `AREKA_APP_SMOKE_EXIT_MS=500`・60 秒の見張り・終了コード 0・経路マーカー | **本仕様の実装が正しければ 1 文字も変えずに緑のまま**（マーカー文言を変えない限り） |

### 2.6 並走 spec との共有（`.kiro/steering/roadmap.md`「干渉台帳（A1）」）

- ⑤（本仕様）の登記: `crates/wintf/src/runtime/mod.rs`・`crates/areka/src/emo2_boot/frame.rs`・`input_events/mod.rs`・`main.rs`（`on_dummy_pressed`・smoke のクロージャ・`app.run()` の後ろ）・`tests/smoke_boot_loop_exit.rs`。
- 本分析が追加で触る見込みのファイル: `crates/wintf/src/runtime/message_loop.rs`（`ShutdownPolicy` に「指示済み」ビットを持たせる場合）・`crates/wintf/src/runtime/window_registry.rs`（触らずに済む見込み）・`crates/areka/src/placement/spawn.rs`（`despawn_ghost_windows` を動かす場合）・新モジュール 1 本。A1 の他 spec ①〜④⑥⑦の登記ファイル（`shiori-host32-helper`・`wintf/src/ecs/drag`・`ecs/clickthrough/controller.rs`・`ecs/window_proc/{mouse_click,keyboard}.rs`・`areka/src/menu/*`・`readme.rs`・`areka-nar`・`sample-ghost-kit` ほか）と**重なりは 0 のまま**。完了時に干渉台帳へ追記する。
- ⑧ `baseware-root-layout` は `brief.md` のみ存在（要件・設計は未着手）。共有は `main.rs`（相手は `main()` の根の解決と `resolve_config_inputs` の `warn!` 継続経路）と `tests/smoke_boot_loop_exit.rs`。相手の brief は「引数なし smoke（フォールバック方向）を陳腐化として除くか更新するか」を要件で決めるとしている。

## 3. 要件→資産マップ

| 要件 | 既存資産 | 状態 | 備考 |
|---|---|---|---|
| 1.1 窓 0 でも生きる | `wire_shutdown_hook`（常にフックを仕込む） | **Missing** | 構築時の選択が無い |
| 1.2 指示で戻り後始末は同順 | `fn main` の①〜④ | 既存で足りる | `run()` の戻り方が変わるだけ |
| 1.3 残った窓も閉じる | `WinApp` drop → 登録表 drop → `DestroyWindow` | **Constraint** | 今は「プロセス終了の直前」に壊れる。ゴースト終了統括・seriko 合流の間は窓が残る。壊す責任の置き場が未決（§6 議題 4） |
| 1.4 二重指示で失敗しない | `Event::notify` は冪等（`notify_shutdown_wakes_armed_listener_and_is_idempotent`） | 部分的 | 通知自体は冪等だが「2 回目を記録で区別する」には 1 ビット要る |
| 1.5 ループ開始前の指示 | `shutdown_future` は `listen` してから待つ | **Missing** | `Event` はリスナ不在の通知を失う（`event-listener` 5.4.2 の doc）。「指示済み」ビットが必須 |
| 2.1〜2.2 構築時の選択・既定は従来 | `WinApp::new()` 引数なし | **Missing** | 2 つ目の構築口（または設定値）が要る。`new()` は不変 |
| 2.3 既定で従来どおり | `reconcile_window_registry`＋フック | 既存 | 既存テスト 7＋3 本が保証 |
| 2.4 選べば窓 0 でも続く・後で窓を開ける | フックを仕込まない／鳴らさない | **Missing** | `reconcile` はフック `None` で何もしない。登録表は空でも `insert` を受けるので「後で開ける」は自然に成立 |
| 2.5〜2.6 明示の指示 1 本・選択にかかわらず終わる | `Rc<Event>` は `WinApp` 私有 | **Missing** | World から届く受け口（NonSend）か `WinApp` の公開メソッド |
| 2.7 受けたことを info で記録 | — | Missing（小） | `info!` 1 行。`logging.md` の「ライフサイクル事象＝info」に一致 |
| 2.8 実行中に切り替える口を持たない | — | 設計で守る | フィールドを不変にする・setter を作らない |
| 3.1〜3.5 4 か所から指示 | 4 か所とも `&mut World` を持つ | **Constraint** | World 経由の受け口があれば各 1 行 |
| 3.6 全窓破棄と指示を 1 操作に | `despawn_ghost_windows`・`despawn_smoke_targets` の 2 関数 | **Missing** | 統合先の置き場と可視性の設計（§6 議題 5） |
| 3.7 出所を info で記録 | `ghost_quit`／`mouse_escape_close` は既に info | 部分的 | ダミー窓・smoke は文言のみ。出所を 1 つの語彙に揃える |
| 3.8 後始末の順序・終了コード不変 | `fn main` | 既存 | — |
| 4.1 example 7 本を触らない | `WinApp::new()` 不変 | 成立見込み | 2.4 節の 14 本 |
| 4.2〜4.3 kanade の握手・`fn shutdown` 不変 | — | 成立見込み | 本仕様は `KanadeStopped` を受けた**後**だけ変える |
| 4.4 既存テストを落とさない | §2.5 | **Constraint** | 素の World で走る areka テスト群が「受け口不在」を踏む |
| 4.5 `main.rs` 1,000 行以内 | 958 行 | **Constraint** | 終了関連を新モジュールへ動かせば減る |
| 5.1 「窓 0 で終了しない」の決定論テスト | `close_to_reconcile_to_shutdown_chain_wakes_listener`・`new_owns_unfired_shutdown_signal` が型 | 新設 | 同じ構造で「鳴らない」を `wait_timeout` で主張 |
| 5.2 「指示で終わる」の決定論テスト | `shutdown_future_completes_when_event_notified` が型 | 新設 | 「指示 → future 完了」。ループを回さない `run()` が headless で戻るかは要調査（§5） |
| 5.3 配線を再テストしない | — | 方針 | 足すのは判断分岐 2 つ＋1.4／1.5 の縁 |
| 5.4 smoke テスト緑・見張りの役を保つ | `smoke_boot_loop_exit.rs` | 既存 | マーカー文言を変えない |
| 5.5〜5.6 実機確認 | `AREKA_APP_SMOKE_EXIT_MS`・`RUST_LOG` | 既存 | 自動終了では別れの台詞は流れない（記憶: 終了挨拶はメニューの終了で確かめる）。プロセス残存の確認は `Start-Process -PassThru` の PID だけを見る |
| 6.1〜6.2 仮の裁定 | — | 設計で具体化 | §4・§6 |

## 4. 実装アプローチ

### 案 A: 既存ファイルの中だけで拡張

- wintf: `runtime/mod.rs` に `pub enum ExitPolicy { OnLastWindowClose, Explicit }` と `WinApp::with_exit_policy(policy)` を足し、`new()` は `with_exit_policy(OnLastWindowClose)` へ委譲。`wire_shutdown_hook` は `Explicit` のときフックを仕込まない。`WinApp::request_exit(&self)` を足す。
- areka: 4 か所それぞれの despawn の直後に `request_exit` 相当を 1 行足す。ただし `&WinApp` が無いので、`Emo2Wiring`・ポインタハンドラ・smoke クロージャの 3 経路へ `Rc<Event>` かハンドルを配る。

| ✅ | ❌ |
|---|---|
| 新ファイル 0 | 3 経路の配線が増え、`main.rs` の行数も増える |
| 変更点が既存テストの隣に置ける | **要件 3.6 に反する**（全窓破棄と指示が別々の呼び出しのまま） |
| | 要件 1.5 の「指示済み」ビットが `WinApp` 私有になり、areka 側から見えない |

### 案 B: 新しい部品を作る

- wintf: `runtime/exit.rs`（仮）に `pub struct AppExit { requested: Cell<bool>, signal: Rc<Event> }`（NonSend）を置き、`WinApp::new`／`with_exit_policy` が World へ挿す。`request_exit(&self)` は `requested` を立てて `notify`、2 回目は `debug!` で流す。`ShutdownPolicy::shutdown_future` は `listen` の後に `requested` を見て、立っていれば即完了（arm と check の順で競合を塞ぐ）。既定ポリシーのフックも `AppExit::request_exit` を呼ぶ形に寄せれば、**終了の完了機構が 1 本**になる。
- areka: `crates/areka/src/app_exit.rs`（仮）に `pub(crate) enum ExitOrigin { KanadeStopped(KanadeStopCause), Escape, DummyWindow, Smoke }` と `pub(crate) fn quit_app(world: &mut World, origin: ExitOrigin) -> usize` を置く。中で「ダミー窓＋ゴースト窓を全部 despawn」→「`AppExit` を World から取り出して `request_exit`」→ `info!(event="app_exit", origin, closed)`。`despawn_ghost_windows`（`placement/spawn.rs`）と `despawn_smoke_targets`（`main.rs`）はこの関数の私有部品へ吸収する（外から呼べる全窓破棄を残さない＝要件 3.6 を構造で守る）。4 か所は `quit_app(world, 出所)` の 1 行になる。

| ✅ | ❌ |
|---|---|
| 要件 3.6 を「片方だけ呼べない」形で満たす | 新ファイル 2 本 |
| `main.rs` から `despawn_smoke_targets`・`on_dummy_pressed` の本体が出て行き、行数が減る（並走 spec への恩恵） | `main_seam_tests.rs` の 3 本と `frame_ghost_quit_tests.rs`・`input_events_tests.rs` の素の World が「`AppExit` 不在」を踏む（§6 議題 6） |
| wintf の受け口が `ClickThroughRegistryHandle` と同型で、利用側の学習コストが無い | `placement/spawn.rs` が干渉台帳の登記外（ただし他 spec との重なりは 0） |

### 案 C: 混成（推奨）

wintf 側は**既存ファイルの中**で済ませる（`ExitPolicy`・`AppExit`・`with_exit_policy`・`request_exit` を `runtime/mod.rs` に、「指示済み」ビットの参照を `message_loop.rs` の `ShutdownPolicy` に。`runtime/mod.rs` は 627 行で余裕がある）。areka 側は**新モジュール 1 本**（`app_exit.rs`）に統合操作を置き、4 か所を 1 行ずつに書き換える。

- 推す理由: wintf の変更は 3 つの定義の追加で収まり、既存テストの隣に置ける。areka の変更は「終了とは何か」を 1 か所に集めるものなので、`main.rs` に足すより新モジュールの方が読み手に優しく、1,000 行の目安と並走 spec の両方に効く。
- 段取り: ⑴ wintf（ポリシー・受け口・ビット・決定論テスト 2＋2 本）→ ⑵ areka 新モジュール＋4 か所の書き換え＋既存テストの追随 → ⑶ `main` の `WinApp::with_exit_policy(Explicit)` への切替（**この 1 行を最後に入れる**と、それまでの段で smoke テストが従来どおり緑であることを確かめられる）→ ⑷ smoke テスト緑の確認と実機確認。

## 5. 規模・リスク・要調査

- **規模 S**: wintf 3 定義＋テスト 4 本、areka モジュール 1 本＋4 か所各 1 行＋テスト追随、`main` 1 行。brief の 5〜7 タスクと整合。
- **リスク 低〜中**:
  - 低: 既定ポリシーの経路は 1 文字も変わらないので example・既存テストは影響を受けない。
  - 中: 要件 1.3 の「残った窓をいつ壊すか」。今は `WinApp` drop（`main` の末尾）まで残る。smoke の経路では despawn と指示が同じ `borrow_mut` の中で起き、tick を待たずにループが終わる可能性が高く、窓は後始末①〜③の間だけ画面に残る（実測は要る）。`run()` の復帰直前に登録表を空にする（`Window<WndState>` を drop して `DestroyWindow`）なら、ウィンドウ手続きが `try_borrow` で読み飛ばすため再入でも壊れない（§2.1）。
- **Research Needed**:
  1. **ループを回さない `run()` の決定論テストが headless で通るか。** `run()` は `VsyncEventBridge`（`DwmFlush` のスレッド）とクリック透過ワーカを起こす。指示済みの状態で呼べば `block_on` は即戻るはず（`block_on_ready_future_returns_value` と同じ理屈）だが、既存テストは「full `run()` の E2E は手動」としており実績が無い。通れば要件 1.5 を `run()` そのもので固定できる。通らなければ `shutdown_future`＋ビットの単位で固定し、`run()` の貫通は smoke テストに任せる。
  2. **指示のあとに tick が 1 巡するか。** 上の 1.3 の実測。`RUST_LOG=wintf=debug` で `DestroyWindow` のタイミングを見る。
  3. **`ecs/app.rs` の `App::on_window_destroyed` が出す `"[App] Last window closed."`（info）。** 「窓 0 で終了しない」を選んだ後もこの行は出るが、終了は起きない。誤読を招くなら文言の調整を検討（本仕様の範囲に含めるかは議題）。

## 6. 設計判断項目（要件ディスカッションへ）

1. **構築時の選択口の形。** ⑴ `pub enum ExitPolicy { OnLastWindowClose, Explicit }` を引数に取る 2 つ目の構築関数 `WinApp::with_exit_policy(policy)`（`new()` は既定で委譲）／⑵ 設定値の構造体 `WinAppOptions { exit_on_last_window_close: bool }`／⑶ ビルダー。推奨は ⑴——選択肢が 2 つしか無く、将来の設定値が現れたとき ⑵ へ移るのは容易。いずれも `new()` の署名は不変。
2. **明示の終了の指示を areka の 4 か所へどう届けるか。** ⑴ wintf が構築時に World へ挿す NonSend リソース（例 `AppExit`）を areka が `world.get_non_send::<AppExit>()` で取る（`ClickThroughRegistryHandle` と同型・配線 0）／⑵ `WinApp::exit_handle()` が返すハンドルを areka が自前の NonSend（`Emo2Wiring` など）に持ち回る。推奨は ⑴。
3. **「指示済み」ビットの持ち方と、既定ポリシーの合流。** `Cell<bool>`＋`Rc<Event>` を 1 つの型にまとめ、`shutdown_future` は `listen` を立てた後にビットを見る（arm → check の順で通知の取りこぼしを塞ぐ）。既定ポリシーの空遷移フックも同じ `request_exit` を呼ぶ形にすれば、終了の完了機構が 1 本になる（フックが `Event` を直接鳴らす今の形を残すか、ビット経由へ寄せるか）。2 回目以降の指示は `debug!` で流す（要件 1.4）。
4. **明示の終了のとき残っている窓を誰が壊すか（要件 1.3）。** ⑴ wintf の `run()` が `block_on` 復帰後・戻る前に登録表を空にして `DestroyWindow` する（決定論・ウィンドウ手続きは `try_borrow` で安全）／⑵ `WinApp` drop に任せる（今日と同じ・後始末①〜③の間は窓が残る）／⑶ areka が despawn のあと tick を待ってから指示する（**採らない**——1 フレーム遅らせる解）。推奨は ⑴。
5. **areka の統合操作の置き場と形。** 新モジュール `crates/areka/src/app_exit.rs`（仮）に `quit_app(world, origin) -> usize` を置き、`despawn_ghost_windows`（`placement/spawn.rs`）と `despawn_smoke_targets`（`main.rs`）をその私有部品へ吸収して外から呼べなくする。`DummyWindowMarker`（`main.rs`）と `GhostWindowMarker`（`placement/spawn.rs`）の両方を狙う 1 本の despawn に統合する。既存テスト（`main_seam_tests.rs` の 3 本）は本文不変で置き場だけ追随。**後続のゴースト切替が要る「全窓を閉じるが終了しない」操作は本仕様では作らない**（要件の Out of scope）ので、閉じる部品を私有にしても後続を縛らない（後続が自分の操作を足す）。
6. **統合操作が World に受け口を見つけられなかったときの扱い。** 本番では `WinApp` が必ず挿すので不在は配線の誤り＝`error!`＋指示なし（記録無しの失敗経路を作らない）。ただし素の `World` で走る既存テスト（`frame_ghost_quit_tests.rs` の 1 本は `ERROR` 0 行を主張）がこれを踏む。選択肢: ⑴ テスト側の補助関数が headless の `AppExit`（`Event::new()`＋`Cell` だけで作れる）を World へ挿す（推奨・本番の意味論を曲げない）／⑵ 不在を `warn!` に留める／⑶ 受け口を引数で渡す署名にする（4 か所の配線が増える）。
7. **出所の記録（要件 2.7・3.7）。** ⑴ areka が `info!(event="app_exit", origin=?ExitOrigin)` を 1 行、wintf が受領の `info!` を 1 行（層ごとに 1 行・wintf は areka の語彙を知らない）／⑵ wintf の `request_exit(reason: &'static str)` に理由を渡して 1 行にまとめる。推奨は ⑴。`ExitOrigin` は既存の `KanadeStopCause` を包む変種を持ち、`ghost_quit` の `cause` と同じ語で出す。
8. **`ecs/app.rs` の `"[App] Last window closed."`（info）を本仕様で触るか。** 「窓 0 で終了しない」を選んだ後は事実と違う含意（終了）を持つ行になる。⑴ 触らない（本仕様の範囲外・別 spec）／⑵ 文言を「窓が 0 になった」に改める（1 行）。
9. **並走 spec `baseware-root-layout` との調整。** 相手の brief は引数なし smoke（フォールバック方向）を陳腐化候補としているが、本仕様の要件 5.4 はこのテストを「窓が無いのにプロセスが残る」壊れ方を赤にする常設の見張りとして残す。本仕様が先に着地するので、相手の要件段階で「フォールバック方向のマーカーは変えても、60 秒の見張りと終了コード 0 の判定は残す」旨を申し送る必要がある。また本仕様が終了関連を `main.rs` から出すことで、相手の `main.rs` の作業面積と行数の圧力が下がる。
10. **要件 1.5 の決定論テストの単位。** ⑴ `WinApp::with_exit_policy(Explicit)` → `request_exit()` → `run()` が戻る（`run()` そのもの・headless で通るかは §5 の要調査 1）／⑵ `shutdown_future`＋ビットの単位（`listen` の前に指示済みでも完了する）。⑴ が通れば ⑴、通らなければ ⑵＋smoke テストで貫通。
11. **実機確認の手順。** 自動終了（smoke 経路）と、メニューの「終了」（`KanadeStopped` 経路・別れの台詞を伴う）の 2 走行。`RUST_LOG=info` で `app_exit`（areka）と wintf の受領行を grep し、`Start-Process -PassThru` で得た自分の PID だけを `HasExited` で確かめる（要件 5.6）。強制退避・ダミー窓は決定論テストで足りるか、実機でも 1 度ずつ踏むかは開発者の判断。

### 要件ディスカッションでの確定（2026-09-23）

- **議題 4 は要件側で縛った。** 要件 1.3 を「残っている窓をすべて閉じ終えてからメッセージループから戻る」へ改めた（後始末①〜④の間に窓を残さない＝今日と同じ見え方）。⑵（`WinApp` drop に任せる）は要件に反するので**採らない**。⑴ の実現手段（`run()` の復帰直前に登録表を空にする等）は設計で決める。
- **議題 9 は要件側へ転記済み。** 要件の Adjacent expectations に `baseware-root-layout` への申し送り（マーカーは変えてよい・60 秒の見張りと終了コード 0 の判定は残す）を書いた。相手の brief への追記は本仕様の完了時（roadmap の干渉台帳の更新と同時）に行う。
- **要件 4.1 の example の数を実測で改めた。** `WinApp::new()`＋`run()` の形は wintf 7 本に加え `crates/areka/examples/` 5 本・`crates/areka-emo-text/examples/` 2 本（§2.4）。要件は数ではなく「その形の example すべて」で縛る。
- 議題 1・2・3・5・6・7・8・10・11 は設計フェーズ（`/kiro-spec-design`）で解決する（→ 下の §8 で解決済み）。

---

## 7. 設計フェーズの調査（2026-09-23・拡張型の軽い調査）

> `design.md` の生成に先立って実ファイルを読み直したもの。分類は **Extension（既存系の拡張）**＝統合点に絞った調査。外部依存の追加は 0 なので Web 調査は行っていない。サブエージェントは使わず、主文脈で全ファイルを読んだ。

### 7.1 窓が消える経路の全段（要件 1.3 の実現手段を決めるため）

- **Context**: 明示の終了の指示のあと、まだ画面に残っている窓を「誰が・いつ」壊すか。
- **Sources**: `crates/wintf/src/ecs/window/window_handle.rs` の `fn on_window_handle_remove`・`crates/wintf/src/ecs/window_proc/lifecycle.rs` の `fn WM_CLOSE`・`crates/wintf/src/runtime/wndproc_bridge.rs` の `fn make_wndproc`・`crates/wintf/src/runtime/window_registry.rs` の `fn reconcile_window_registry`・`crates/wintf/src/runtime/mod.rs` の `fn run`。
- **Findings**:
  - `world.despawn(窓)` → `on_window_handle_remove`（`WindowHandle` の除去フック）が `App::on_window_destroyed` を呼び、`PostMessageW(WM_CLOSE)` を投函する。実際の `DestroyWindow` は **同 tick の `FrameFinalize`** で `reconcile_window_registry` が `Window<WndState>` を drop したときに起きる。
  - `WM_CLOSE` の受け手（`lifecycle.rs`）は `try_borrow_mut` が取れれば entity を despawn し、既に無ければ `DESPAWNED_SKIP_TAG` の `debug!` で打ち切る。借用できなければ何もしない。
  - ウィンドウ手続き（`make_wndproc`）は World を `try_borrow` で試し、失敗なら既定手続きへ委譲する。`reconcile_window_registry` は tick の中（World 借用中）で `DestroyWindow` を呼んでいるので、**World を借りたまま窓を壊す**のは今日の経路が毎回踏んでいる条件である。
  - `run()` は `block_on` の復帰後に `ShutdownPolicy::notify_shutdown` を撃って `Ok(())` を返すだけで、登録表を見ない。
- **Implications**: `run()` が戻る直前に `remove_non_send::<ProdWindowRegistry>()` で登録表ごと取り出して drop すれば、残った窓は `DestroyWindow` される。新しい API（`WindowRegistry::clear` 等）は要らない。`run()` を 2 度呼ぶ運用は今も想定外（`wire_new_path` の doc）なので登録表を取り去って差し支えない。`DestroyWindow` はその窓宛ての未処理メッセージを捨てるので、投函済みの `WM_CLOSE` が後から届く経路も無い。

### 7.2 指示が出るタイミングと tick の関係（Flow 1／Flow 2 の根拠）

- **Context**: 指示のあとに reconcile が走るかどうかは、指示が tick の中で出るか外で出るかで変わる。
- **Sources**: `crates/areka/src/emo2_boot/frame.rs` の `fn emo2_frame_system`（`Update` 段・排他 system）・`crates/areka/src/main.rs` の `fn open_startup_window` 内 smoke クロージャ（`wintf::executor::spawn_local`）・`crates/wintf/src/runtime/message_loop.rs` の `MessageLoopDriver::block_on` の doc。
- **Findings**: `run_ghost_quit_phase`・強制退避・ダミー窓は tick の中（ポインタ配送も tick の `Input` 段）で動くので、同じ tick の `FrameFinalize` が窓を壊してから `block_on` が future を見る。smoke クロージャは tick の外の async タスクなので、`block_on` が先に戻り得る。
- **Implications**: 要件 1.3 が実際に効くのは smoke 経路（Flow 2）。実機確認ではこの走行で「windows remained open」の行が出るかを見る（出なくても reconcile が先に走っただけで正常）。

### 7.3 `block_on` と `spawn_local` の振る舞い（決定論テストの単位を決めるため）

- **Sources**: `message_loop.rs` の `block_on_ready_future_returns_value`・`MessageLoopDriver::block_on` の doc（「`spawn_local` で投入済みの UI タスクも並行に駆動される」）・`crates/wintf/src/runtime/mod.rs` の `wire_click_through_starts_and_inserts_registry_handle`（`spawn_local` をループ非実行下で投入して安全）。
- **Findings**: 完了済み future を渡した `block_on` は正常に戻る。投入済みタスクは `block_on` の中で駆動される。`run()` 自体を headless で回した実績は無い（VSync スレッド・クリック透過ワーカを起こす）。
- **Implications**: 要件 1.5／5.2／1.4 のテストは `MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(exit))` の単位で組める（§6 議題 10 の ⑵）。`run()` 直接のテストは作らない。

### 7.4 素の `World` で終了経路を叩く既存テストの全数（受け口不在の扱いを決めるため）

- **Sources**: `crates/areka/src/emo2_boot/frame_ghost_quit_tests.rs`（4 本・`world_with_ghost_windows` ヘルパ・テスト 4 は `level=ERROR` 0 行を主張）、`crates/areka/src/input_events/input_events_tests.rs`（`world_with_wiring` ヘルパ経由 1 本＋素の `World` 2 本が強制退避を踏む）、`crates/areka/src/main_startup_window_tests.rs`（`double_click_left_despawns_all_dummy_windows`）、`crates/areka/src/main_seam_tests.rs`（`despawn_smoke_targets_*` 3 本・対照アーム付き）。
- **Findings**: 終了経路を踏むのは上の 4 ファイル。いずれも `WinApp` を建てない素の `World`。`log-capture-kit` は `[dev-dependencies]`（`crates/areka/Cargo.toml`）。
- **Implications**: 受け口不在を `error!` にすると `frame_ghost_quit_tests` のテスト 4 が赤になる。ヘルパへ `wintf::AppExit::new()` を挿す（本番の意味論は曲げない）。`despawn_smoke_targets_*` は関数が私有部品へ移るので置き場を `app_exit_tests.rs` へ移す（本文不変）。

### 7.5 呼び手と参照の全数（削除の安全確認）

- **Sources**: `git grep` で `despawn_ghost_windows`／`despawn_smoke_targets` を `crates/` 全域。
- **Findings**: 本体 2 か所と呼び手 3 か所（`frame.rs`・`input_events/mod.rs`・`main.rs` smoke クロージャ）、テスト 3 本（`main_seam_tests.rs`）、doc コメント 1 行（`placement/spawn_cleanup_tests.rs`）のほかに参照は無い。example の `#[path]` include は `placement/spawn.rs` を含むが、関数を削除しても参照が無いので壊れない。`spawn.rs` の `debug!`／`DESPAWNED_SKIP_TAG` は他の場所でも使われており、削除後に未使用 import は出ない。

### 7.6 `ClickThroughRegistryHandle` の形（受け口の先例）

- **Sources**: `crates/wintf/src/ecs/clickthrough/controller.rs` の `pub struct ClickThroughRegistryHandle { registry: Rc<RefCell<…>> }`・`pub(crate) fn new`・`pub fn register/remove/len`。`crates/wintf/src/runtime/mod.rs` の `fn wire_click_through` が `insert_non_send` する。
- **Implications**: `AppExit` も「`Rc` を中に持つ `pub struct`・`WinApp` が World へ挿す・利用側は `get_non_send` で取る」の同型で組める。`new()` だけは areka のテストが headless で建てるため `pub` にする（先例は `pub(crate)`）。

## 8. 設計判断（`design.md` に転記済み・ここは根拠の控え）

### Decision: 口の形（§6 項目 1）
- **Alternatives**: ⑴ `ExitPolicy` enum＋`with_exit_policy` ／ ⑵ `WinAppOptions` 構造体 ／ ⑶ ビルダー。
- **Selected**: ⑴。`new()` は `with_exit_policy(OnLastWindowClose)` へ委譲。
- **Rationale**: 選択肢が 2 つしか無い。将来 2 つ目の設定値が現れたら ⑵ へ移るのは容易で、今は器を作らない。
- **Trade-offs**: なし（`new()` の署名不変・example 14 本を触らない）。

### Decision: 指示の届け方（§6 項目 2）
- **Selected**: ⑴ `WinApp` が World へ挿す NonSend `AppExit`。
- **Rationale**: areka の 4 か所はどれも `&mut World` しか持たない。`ClickThroughRegistryHandle` と同型で配線 0。⑵（ハンドルを `Emo2Wiring` 等へ持ち回る）は 3 経路の配線が増える。

### Decision: 指示済みビットと完了機構の一本化（§6 項目 3）
- **Selected**: `AppExit { requested: Rc<Cell<bool>>, signal: Rc<Event> }` を `message_loop.rs` に置き、`shutdown_future` は arm → 確認 → await。既定ポリシーの空遷移フックも `request_exit` を呼ぶ。`WinApp` の `shutdown: Rc<Event>` は `exit: AppExit` へ置き換える（既存テストの `app.shutdown.listen()` 3 か所は `app.exit.signal().listen()` へ・意味不変）。2 回目以降の指示は `debug!`。
- **Rationale**: `event-listener` 5.4.2 はリスナ不在の通知を失うので記憶が要る（要件 1.4・1.5）。フックを `request_exit` に寄せると終了の完了機構が 1 本になり、既定と明示で振る舞いが分かれない。
- **Trade-offs**: example の終了時に `info` が 1 行増える。`WinApp` の欄を 1 つ置き換えるので wintf 内テスト 3 行が追随する。
- **Follow-up**: 実装で `Rc` を 2 本にするか `Rc<ExitState>` 1 本にするかは自由（公開面は同じ）。

### Decision: 残存窓の破棄（要件 1.3・§6 項目 4 の実現手段）
- **Selected**: `run()` が `block_on` 復帰直後に `remove_non_send::<ProdWindowRegistry>()` で登録表ごと drop する。World を借りたまま行う（`reconcile_window_registry` と同条件）。空でなければ `info` 1 行。
- **Rationale**: 新 API 0・`window_registry.rs` 変更 0 行。1 フレーム待つ解を採らない。

### Decision: 統合操作の置き場（§6 項目 5）
- **Selected**: 新モジュール `crates/areka/src/app_exit.rs` に `quit_app(world, origin) -> usize`。`despawn_ghost_windows`／`despawn_smoke_targets` は削除し私有部品 `despawn_app_windows` へ吸収（両マーカーを 1 本の query で狙う）。
- **Rationale**: `placement` は `crate::` パスを持てず、`main.rs` は 958 行。「終了とは何か」を 1 か所へ集めると読み手に優しく、並走 spec の `main.rs` 圧力も下がる。外から呼べる全窓破棄を残さないことが裁定 2 の構造的な守りになる。

### Decision: 受け口不在の扱い（§6 項目 6）
- **Selected**: ⑴ テスト側が `AppExit::new()` を挿す。本番の不在は `error!(event="app_exit_unwired")`＋指示なし。
- **Rationale**: 本番では `WinApp` が必ず挿すので不在は配線の誤り。記録無しの失敗経路を作らない。テストのために本番の記録レベルを下げない。

### Decision: 出所の記録（§6 項目 7）
- **Selected**: ⑴ 層ごとに 1 行。areka `info!(event="app_exit", origin=?ExitOrigin, closed)`・wintf `info!("[AppExit] exit requested")`。
- **Rationale**: wintf は areka の語彙を知らない。`ExitOrigin::KanadeStopped(cause)` の `Debug` は `ghost_quit` の `cause` と同じ語を出す。

### Decision: `"[App] Last window closed."`（§6 項目 8）
- **Selected**: ⑴ 触らない。
- **Rationale**: 文は事実（最後の窓が閉じた）を述べているだけで終了を含意しない。`App` は旧経路のカウンタで本仕様の範囲外。

### Decision: 要件 1.5 のテストの単位（§6 項目 10）
- **Selected**: ⑵ `block_on(shutdown_future(exit))` の単位。`run()` 直接のテストは作らない。
- **Rationale**: 判断分岐（arm → 確認 → await）はこの単位で固定できる。`run()` は VSync スレッドとワーカを起こし headless の実績が無い。`run()` の貫通は smoke テストが実プロセスで踏む。要件 5.3（足すテストを判断分岐に限る）にも合う。

### Decision: 実機確認の手順（§6 項目 11）
- **Selected**: smoke の自動終了（Flow 2）とメニューの「終了」（Flow 1）の 2 走行。強制退避は任意。ダミー窓は引数なし smoke テストが踏む。`RUST_LOG=info,wintf::runtime=debug`・`Start-Process -PassThru` の PID だけを `HasExited` で見る。
- **Rationale**: 2 走行で「tick の中の指示」「tick の外の指示」の両方を踏む。判定の分岐（2 回目の指示の `debug!`）まで記録が出る level にする。

## 9. 総合（設計合成の 3 つの視点）

- **一般化**: 終了操作 6 種は「全窓を閉じて終了を指示する」1 つの操作の変種で、違いは出所だけ。`quit_app(world, origin)` の 1 関数に集め、出所は値（`ExitOrigin`）で渡す。wintf 側の既定（窓 0 で終了）と明示の指示も、`request_exit` 1 本を通る同じ完了機構の 2 つの入口として一般化した。
- **作るか採るか**: 新しい依存は 0。`event-listener`（既存）・`Cell<bool>`（std）・`remove_non_send`（bevy 既存）・`block_on`（既存）だけで組む。
- **単純化**: 設定値の構造体・ビルダー・トレイト・`WindowRegistry` の新 API・`run()` 直接の headless テスト・時間の安全網・「閉じるが終了しない」操作は、いずれも今の要件が求めないので作らない。

## 10. リスクと備え（設計後）

- `run()` の残存窓破棄が World 借用中に `DestroyWindow` を呼ぶ — 今日の `reconcile_window_registry` と同じ条件で新しい再入は無い。実機の smoke 走行で確認。
- `spawn_local` から `request_exit` するテスト（5.2）が `block_on` の中で駆動されない — `MessageLoopDriver::block_on` の doc と `wire_click_through` の先例が「投入済みタスクは駆動される」と述べている。赤になれば `block_on(async { exit.request_exit(); shutdown_future(exit).await })` の形へ退避しても判断分岐の固定は保てる。
- 既定ポリシーの終了時に `info` が 1 行増える — example と wintf 単体の利用者の終了結果（exit 0）は変わらない。
- `main.rs` の行数 — `despawn_smoke_targets`（doc 込み約 50 行）と `on_dummy_pressed` の本体が出て、`mod app_exit;` と 1 行化が入るので 958 から減る。実装後に `wc -l` で確かめる。
