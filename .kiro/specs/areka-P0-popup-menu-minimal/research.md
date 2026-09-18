# Gap Analysis: areka-P0-popup-menu-minimal

> 2026-09-18・本ブランチ（`claude/areka-p0-popup-menu-f3c3c3`・main `ca3c4fdc` 直後）での実測。file:line と行数は着手時に引き直すこと。引用は「何の定義か」で指し、行番号は補助に留める。

## 0. 要約

- **既存資産はほぼ全部「半分だけ」ある。** 右ボタンは押下・解放とも wintf が受けているが解放が areka に配られない。SHIORI リソースの GET は kanade に経路も結果語彙（`ResourceOutcome`＝Value／NoContent／Failed）もあるが、許可名が `username` 1 つで、しかも起動時 1 回の prefetch 専用。終了の握手は完成しているが `OnClose` に Ref1/Ref2 を載せる箱が無い。`\!` の汎用キャリアと消費者台帳は揃っていて `("open","readme")` を 1 行足せる形になっている。`readme` キーは descript の転記層（`package::resolve`）が読んでいない。
- **Win32 のメニューと既定アプリ起動は依存追加 0 で呼べる**（`windows` 0.62.2 の `Win32_UI_WindowsAndMessaging`／`Win32_UI_Shell`／`Win32_Graphics_Gdi` はワークスペースで有効・実在を registry で確認）。`TrackPopupMenu`／`HMENU`／`ShellExecuteW`／`WM_CONTEXTMENU` は `crates/` に 0 件（要件 Introduction の記述どおり）。
- **最大の構造的な分かれ目は 3 つ**: ⑴ 解放イベントの出し方（wintf のポインタ配送に「離した」を足す形）、⑵ メニューを出すたびに SHIORI へ 8 件前後の GET を行う経路（kanade のアクター殻で受けるか・状態機械を通すか）と UI スレッドの待ち方、⑶ モーダルな `TrackPopupMenuEx` を tick の借用の内側で呼ぶか外側（`spawn_ui_local`）で呼ぶか。⑶ は「表示中に描画が止まるか」「窓が消えたときの挙動」を決める。
- **行数の壁が 4 ファイルで近い**: `main.rs` 948・`kanade/schedule/steady.rs` 935・`areka-parsers/package/resolve.rs` 963（本文内に旧形式のテストを抱えている）・`placement/spawn.rs` 809。終了の scope・readme キー・結線を素直に足すとこれらに触るので、分割か迂回を設計で決める必要がある。
- **台帳の結合は実在する**: `cargo test -p ukadoc-survey` の腕 c（`owner_count` ＝ 台帳の数え直し）・腕 f（非空の担当は `[[spec]]` か `[[owner_completed]]` の名前）・腕 a（`[briefs].count` ＝ `[[spec]]` 行数）が `crates/ukadoc-survey/tests/consistency/spec_checks.rs` にあり、担当 15 件の登記は `roadmap-draft.md` の行追加（owner_count＝15・count 27→28）と同時でなければ赤になる。

## 1. 現状調査（既存資産の地図）

### 1.1 入力経路（右クリックが今どこまで届くか）

| 段 | 実体 | 状態 |
|---|---|---|
| Win32 メッセージ | `crates/wintf/src/ecs/window_proc/mouse_click.rs` の `WM_RBUTTONDOWN`／`WM_RBUTTONUP` → `handle_button_message` → `record_button_down`／`record_button_up` | **押下・解放とも記録される**（`ButtonBuffer{down_received, up_received}`・`pointer/types/mod.rs`） |
| バッファ → `PointerState` | `pointer/buffers.rs` の `transfer_buffers_to_world` | `down_received` なら `right_down=true`、**そうでなく** `up_received` なら `right_down=false`（`if … else if …`）。「離した」という一過性の旗は `PointerState` に**無い**（`double_click`／`wheel` にはある） |
| ハンドラ配送 | `pointer/dispatch/mod.rs` の `dispatch_pointer_events` | `OnPointerMoved` は常時、`OnPointerPressed` は `left_down‖right_down‖middle_down` のとき。**`OnPointerReleased` は型（同ファイル・`wintf::ecs` から再輸出済み）だけあって一度も配送されない**。配送後に全 `*_down` と `double_click` を消す |
| areka 側 | `crates/areka/src/input_events/mod.rs` の `on_char_pointer_pressed` | Ctrl＋左ダブルクリック＝終了、左右ダブルクリック＝`OnMouseDoubleClick`、単発は送らない（doc 7.3）、owner-draw メニュー非実装（doc 7.4）。`attach_char_pointer_handlers` が `CharWindowMarker` 窓へ Moved／Pressed を装着 |

**見落としやすい点**: 押下と解放が**同じ tick に**入ると（速いタップ）、`transfer_buffers_to_world` の `else if` で解放が捨てられ、その後 `dispatch_pointer_events` が `right_down` を消すので、「離した」を `up_received` から**独立に**立てないと速いクリックでメニューが出ない。

ドラッグ中判定（要件 1.9）は `wintf::ecs::drag::snapshot_drag_state()`（`pub`・`DragStateSnapshot::Dragging{hwnd,…}`）で読める。左ドラッグはマウスをキャプチャするので右の解放もキャプチャ窓に届く＝ハンドラ側で snapshot を見て抑止できる。

スコープは `CharWindowMarker.scope`（`placement/spawn.rs`・`usize`）を `char_scope` が `u32` にする。`debug_assert!(scope <= 1)` があり、**α で n≧2 は到達しない**（要件 3.6 の `char{n}` は語彙としてだけ持つことになる）。

HWND は `wintf::ecs::window::WindowHandle{hwnd}`（SparseSet・窓 entity）、窓の物理位置は `WindowPos.position`（`mouse_click.rs` がスクリーン座標を `client + position` で作っている前例）。`ClientToScreen` も `Win32_Graphics_Gdi` で使える。

ゴースト窓のスタイル（`placement/spawn.rs` の共通 `WindowStyle`）は `WS_POPUP|WS_VISIBLE` ＋ `WS_EX_LAYERED|WS_EX_TOOLWINDOW`。`WS_EX_TOPMOST` も `WS_EX_NOACTIVATE` も無い＝`SetForegroundWindow` の前提は素直。

バルーン窓（要件 1.6）: `input_events/balloon.rs` に右ボタンの扱いは無く、右クリックで何も起きない現状は自然に保たれる。

### 1.2 UI スレッド・tick・モーダルループ（メニューをどこで出すか）

- `WinApp`（`crates/wintf/src/runtime/mod.rs`）は `CoInitializeEx(COINIT_MULTITHREADED)` で COM を初期化する。`app.world()` が `Rc<RefCell<EcsWorld>>` を返し、`spawn_ui_local` で UI スレッドの async タスクを投入できる。
- tick は `runtime/tick_bridge.rs` の `tick_one_frame_with` が **`try_borrow_mut` を tick の間ずっと保持**し、再入ガード `IS_TICK_FLUSH_IN_PROGRESS` も張る。ポインタハンドラは tick 内の排他 system `dispatch_pointer_events(&mut World)` から呼ばれる。
- ウィンドウ手続き（`window_proc`）の各ハンドラは `world.try_borrow_mut()` に失敗すると**黙って何もしない**（`mouse_click.rs` のフォールバックが典型）。
- 実行器（`wintf-winmsg-executor` 0.0.5・registry で確認）の起床は `PostMessageW(executor_hwnd, WM_USER)`＝**隠し窓宛のメッセージ**。`TrackPopupMenuEx` の内部モーダルループも `GetMessage`／`DispatchMessage` で回るので、**メニュー表示中も起床メッセージは実行器の窓手続きへ届き、tick タスクは poll される**。
  - tick の**内側**（ハンドラの中）で `TrackPopupMenuEx` を呼ぶと、表示中の tick は `try_borrow_mut` に失敗して素通り（＝描画・アニメ停止）、窓手続きも借用に失敗して入力を落とす。戻った後は普通に続く。要件 7.2 は「表示中に描画が止まることは許容」と書いてあるのでこれでも要件は満たす。
  - tick の**外側**（`spawn_ui_local` した future や、借用を解いた後）で呼ぶと、表示中も tick が回り続ける（アニメが止まらない）。ただし表示中に `run_ghost_quit_phase`（`emo2_boot/frame.rs`・`emo2_frame_system` の先頭）が走って窓を despawn しうる＝owner 窓が消えた `TrackPopupMenuEx` がどう戻るかを要件 7.4 の検証で確かめる必要がある。
- `KanadeStopped`（kanade の停止通知）は `Emo2Wiring.kanade_stop`（`frame/wiring.rs`）を `run_ghost_quit_phase` が tick ごとに `try_recv` する。停止の検出そのものは kanade／SHIORI アクターのスレッド側。
- SHIORI の死活監視（要件 7.1）は `crates/areka-kanade/src/shiori/real.rs` の受信ループ（`recv_timeout(IDLE_INTERVAL)` → `backend.on_idle()`＝host32 だけ `pump_pending_messages` → `status()` 確認）で、**SHIORI アクターのスレッド上**にある。UI スレッドがモーダルループに入っても止まらない（構造上の帰結。テストは実 `ShioriConnection` ではなく「UI が止まっても shiori スレッドの `on_idle` が呼ばれ続ける」を既存の `real_idle_tests.rs` の形で言える）。
- kanade への `Tick` は ticker アクター（別スレッド）から来る。

### 1.3 kanade の SHIORI リソース照会（項目名と表示可否）

- 送出の唯一の出口は `crates/areka-kanade/src/actor.rs` の `round_trip_request`。ID の受理は `EventId::Static(&'static str)` なら `is_allowed_event_id ∨ is_allowed_resource_id`、`EventId::Choice(String)` なら `On` 接頭。
- `crates/areka-kanade/src/schedule/resources.rs`: `ALLOWED_RESOURCE_IDS = &["username"]`（**確認済み**）・`resource_username`・結果語彙 `ResourceOutcome{Value, NoContent, Failed}`・注入シンク `ResourceSink`。凍結テスト `allowed_resource_ids_are_exactly_username` がある＝名前を足すとこのテストは**意図して**書き換える。
- 照会は `schedule/boot.rs` の prefetch 段（`OnInitialize` の後・`OnFirstBoot` の前に 1 回）だけ。`on_prefetch_reply` が 200／204／失敗を `ResourceOutcome` へ写し `Action::ResourceOutcome{id, outcome}` でシンクへ渡す。**要件 3.2〜3.4 の 3 分岐（値／空・204／失敗）はこの語彙と 1 対 1**。
- 状態機械（`schedule/mod.rs` の `step`）は `Action::ShioriRequest` を出し、アクター殻 `drive` が往復し、**バッチ中の最後の応答だけ**を `Input::ShioriReply{outcome, origin}` として再投入する（`BatchResult.last_reply`）。**1 バッチに GET を 8 件並べると 7 件の応答が捨てられる**＝メニュー用の複数 GET を既存の Action だけで表現することはできない。
- アクター殻には状態機械を経ずに処理する前例がある（`KanadeMsg::Close` を `step` の前で捕まえて即 Break）。殻は `state`（`Phase`）と `config` を閉包で持つので、`snapshot_of(&state.phase)` から `ExecutionStatus` を導ける。
- 返信の道具は `areka_actor::reply_channel()`（`ReplySender`／`ReplyReceiver::recv_timeout`）。
- kanade は sylphya に依存しない（steering・`ResourceSink` の疎結合）。名前の表を sylphya の語彙台帳（`areka-sylphya/src/vocab/shiori_resource.rs` に 13 名とも実在）から引くことはできず、kanade 側に自分の許可表を持つ。
- `EventId::Static` は `&'static str` なので `char{n}.popupmenu.visible`（n≧2）を作れない。α では到達しない（1.1）が、語彙を持つなら `EventId` に文字列を運ぶ第 3 の出所（例: `Resource(String)`＋接尾辞規則）を足すか、n≧2 を「語彙だけ・送らない」と要件側で明示するかの分かれ目。

### 1.4 終了の握手（OnClose の Ref1/Ref2）

- 入口: `input_events/mod.rs` の `MouseWiring::send_close_request(CloseReason)` → `KanadeMsg::CloseRequest{reason}`（`msg.rs`）。構築点は input_events と `emo2_boot/spine_conformance_support.rs` の `Injection::CloseRequest` の 2 か所＋テスト。
- 経路: `actor.rs` で `Input::CloseRequest{reason}` → `steady.rs` の `on_close_request`（talk なし＝即 `begin_close`・あり＝`pending_close: Option<CloseReason>` に保留）→ `begin_close` が `Phase::ClosePending{reason}` へ遷移し `events::on_close(reason, INACTIVE)` を発行。`boot.rs` の `record_pending_close` も同じ型で保留する。
- `events.rs` の `on_close` は `references: vec![reason.as_ref_str()]`（doc に「Ref1/2 は M1 では省略」）。`on_close_notify`（ForceQuit の best-effort NOTIFY）も同形。
- `CloseReason{User, System}` は `Copy`・`as_ref_str` を持つ。**scope を運ぶ箱がどこにも無い**。`close.rs` の応答処理（204＝無言終了・quit で解放・非 quit で `Steady` へ復帰＝要件 5.4 の既存経路）は `reason` しか見ない。
- 正典（ukadoc `OnClose`）: Ref1＝「終了操作したメニューが属するキャラクターのスコープ番号・`popupmenu.type` に影響される」、Ref2＝「終了操作したウインドウのスコープ番号・影響されない」。要件 5.2 の Ref1＝Ref2 はこの読みと整合する。

### 1.5 `\![open,readme]` の運搬と消費

- 構文: `areka-parsers/src/sakura/model.rs` の `Instruction::GenericCommand{name, raw_args}` → `areka-sakura/src/compile.rs` が `CueCommand::command_carrier(name, tokens)`（`dola/src/cue/command.rs`・`Custom{command, params}`）へ。消費は `as_command_carrier()` で `(name, tokens)` を取り出して**名前で自己選別**する（typed variant 新設禁止＝記憶 areka-bang-commands-generic-carrier）。
- 台帳: `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical()`（`move`／`bind`／`("set","zorder")`／`("reset","zorder")`／`\f`）と `CommandConsumer` enum。選別子つき登記 `("open", Some("readme"))` は台帳の排他規則（同名で選別子の有無を混ぜない）に触れない（`open` は未登記）。
- 消費者の型: `emo2_boot/move_cue.rs` の `MoveCueSink`（talk スレッド・`dola::cue::CueSink`）が `mpsc` で `MoveDirective` を UI へ送り、`Emo2Wiring.move_rx` を `run_move_drain_phase`（`frame.rs`）が tick ごとに drain する。**`\![open,readme]` も同型（talk スレッドで選別 → UI へ送る → frame 相で「説明書を開く」関数を呼ぶ）にすると、メニューの「説明書」と同じ関数へ到達し（要件 4.5）、実行スレッドも 1 つに揃う**。`GhostBootOptions.sinks` は `emo2_boot/mod.rs` の `wire_emo2_boot` が組む（現在 5 本）。
- 台帳の `sakura-script.toml`: `\![open,readme]` は `status="absent"`・`priority="E6"`・担当 `""`（備考に「キャリアを開ける受け口が名前で自己選別し、担当外として debug! を残す」とある＝現状の落とし方）。
- 引数付き `\![open,readme,種類,名前]`（要件 4.6）: 同じ消費者が `tokens.len() > 1` で warn＋no-op。

### 1.6 `readme` キーとゴーストの根

- `areka-parsers/src/package/resolve.rs` が descript から読むキーは `name`／`sakura.name`／`sakura.name2`／`kero.name`／`shiori`／`shiori.encoding`／`shiori.forceencoding`／`seriko.defaultsurfacedirectoryname` のみ。`MountModel`（`model.rs`・`#[non_exhaustive]`）に `readme` は無い。台帳 `assets.toml` の備考も同じ事実を書いている。
- 根（`ghost/`・`shell/` を含む階層）は `crates/areka/src/boot_config.rs` の `cfg.ghost_root`（argv 第 1 引数）で、`wire_emo2_boot(&app, &cfg.ghost_root, …)` へ渡る。`GhostRuntime`（`areka-ghost/src/runtime.rs`）は `mount: MountModel` を私有で持ち**公開アクセサが無い**（`kanade()`／`dispatcher()`／`sylphya_publisher()` はある）。
- `readme` のファイル名は非 ASCII になりうるので、descript を読み直すなら `package::resolve` と同じ文字コード処理（`kv::parse_kv`＋`charset` プリスキャン＋`default_encoding`）を通す必要がある。
- 検体: emo2 も `R_POST_and_KOMAINU` も根の直下に `readme.txt`・`descript.txt` に `readme` キー無し（要件 Introduction のとおり）＝正典既定 `readme.txt` の経路が実機で踏まれる。
- `ShellExecuteW` は `Win32_UI_Shell` で使える。MS の文書は STA 初期化スレッドからの呼び出しを推奨するが、UI スレッドは `COINIT_MULTITHREADED`（1.2）。単純な `open` 動詞は MTA でも動くのが通例だが、**実機で確かめる項目**（要件 9.9 ⑵）。

### 1.7 Win32 メニュー API（依存追加 0 の確認）

- ワークスペース `Cargo.toml` の `windows` 機能: `Win32_UI_WindowsAndMessaging`・`Win32_UI_Shell`・`Win32_Graphics_Gdi`・`Win32_UI_Input_KeyboardAndMouse` が有効。`crates/areka/Cargo.toml` は workspace の `windows` を使う。
- registry（`windows-0.62.2`）で実在を確認: `CreatePopupMenu`／`AppendMenuW`／`InsertMenuItemW`／`CheckMenuItem`／`EnableMenuItem`／`TrackPopupMenuEx`／`DestroyMenu`／`SetForegroundWindow`／`PostMessageW`（WindowsAndMessaging）、`ShellExecuteW`（Shell）、`ClientToScreen`（Gdi）。
- `TPM_RETURNCMD` で選ばれた ID が戻り値（0＝未選択＝要件 8.3 の debug 記録）。既知の作法（MS KB Q135788）: 表示前に `SetForegroundWindow(owner)`、戻った後に `PostMessageW(owner, WM_NULL)`。これを怠るとメニュー外クリックで閉じない・2 度目に閉じない症状が出る＝要件 1.3 の実機確認項目。
- メニュー外クリックはメニューが消費し、下の窓には届かない（通常の `TrackPopupMenu` の振る舞い）＝要件 1.3 は API の既定で満たされる見込み。実機で確認。
- 既定の IME 窓（記憶 windows-default-ime-window-sits-above-owner）: スレッド最初の窓に所有され既に居座っている。メニュー窓（クラス `#32768`）は別物で、閉じれば消える。要件 7.5 は「増えない・残らない」を実機で見る。
- `WM_CANCELMODE`: `window_proc/keyboard.rs` のハンドラがドラッグを終える。メニュー表示時に OS が owner へ `WM_CANCELMODE` を送るかは環境依存（キャプチャ中の窓に送られる）。要件 1.9 でドラッグ中は出さないので実害は薄いが、設計の注記に値する。

### 1.8 網羅台帳と検査

- 15 項目の現状: `ledger/shiori.toml` の 13 件（7 つの `*button.caption`・3 つの `popupmenu.visible`・3 つの `popupmenu.type`）はすべて `status="vocabulary-only"`・`owner=""`・`priority="A11"`。`sakura-script.toml` の `\![open,readme]` は `absent`／`E6`。`assets.toml` の `descript_ghost readme,ファイル名` は `absent`／`B7`。`quitbutton.caption` も `vocabulary-only`（担当にしない・要件 10.1）。
- 検査（`crates/ukadoc-survey/tests/consistency/spec_checks.rs`・6 つの腕）: **a** `[briefs].count` ＝ `[[spec]]` 行数（現在 27＝27）・**b** 各名前の実在・**c** `owner_count` ＝ 台帳の数え直し・**d** `bundle` が `linkage.md` に在る（`[bundle."メニュー"]` は実在）・**e** `[[owner_completed]]` の整合・**f** 非空の担当が `[[spec]]` か `[[owner_completed]]` の名前。**要件 10.2 の「同じコミットで行を足す」は腕 f と腕 c が要求している**（確認済み）。
- 報告: `report/<ドメイン>.md` 4 本は常時検査が純粋層の出力と突き合わせ（`tests/consistency/documents.rs` の冒頭 doc）、`summary.md` も同様。`cargo run -p ukadoc-survey -- report` と `-- report-summary` で作り直す。
- 実装済みの証拠: `status="implemented"` には**定義箇所**に `/// ukadoc: <URL>` 1 行（許可表の要素・分岐の腕・語彙表の 1 行のどれか・`README.md` §3）。本仕様なら `ALLOWED_RESOURCE_IDS` の要素 13 行・消費者台帳の `("open","readme")` 行・`readme` を持つ定義（`MountModel` の欄など）が置き場。
- `roadmap-draft.md` の手書きの数: `[briefs].count = 27`、本文「いま置き場にあるが表に無いもの 2 本」（09-18 の起票 6 本で既に古い・腕 b は総数を見ないので緑のまま）。要件 10.2/10.5 の「実数えで直す」対象。A0 の並走 2 本（`nar-install`・`default-balloon-bundle`）は現時点で `[[spec]]` 行を持たない（後着側が数え直す）。

### 1.9 テスト資産

- 偽 SHIORI: `crates/areka/src/emo2_boot/spine.rs` の `ScriptedShioriBackend`（`get_scripts: HashMap<id, VecDeque<Result<Option<String>, RequestError>>>`・台本が無い GET は **panic**・`username` は既定で `Ok(None)` を補う）と `areka-ghost` 側の `FakeShioriBackend`／`MapShioriBackend`。要件 9.2 の 4 通り（値／空／204／失敗）は `Ok(Some(""))`／`Ok(Some("x"))`／`Ok(None)`／`Err(RequestError)` で台本化できる。**既存の spine テストはメニューを出さないので新たな GET が走らず壊れない**が、メニューの照会を spine 経路で通すテストは 8 件前後の台本を毎回書く（builder に既定を足す前例が `username` にある）。
- 終了の観測（要件 9.6）: `input_events_tests.rs` の `world_with_wiring` が `Receiver<KanadeMsg>` を返し `rx.try_recv()` で件数を数える。`handler_ctrl_left_double_click_sends_one_close_request_and_keeps_the_windows` がそのまま型になる。
- wintf 配送: `pointer/dispatch/tests.rs` に `test_dispatch_pressed_gating_requires_main_button`／`test_dispatch_clears_button_state_after_dispatch` があり、`OnPointerReleased` の配送を足すときの型。
- ログの表明: `log-capture-kit`（対照イベント必須・恒真防止）。
- 1,000 行の番人: `crates/log-capture-kit/tests/file_length_guard_test.rs`（`LINE_LIMIT`＝1000）。

### 1.10 行数（本仕様が触りうるファイル・実測）

| ファイル | 行 | 備考 |
|---|---|---|
| `crates/areka/src/main.rs` | 948 | 結線を足す場所。**残 52 行** |
| `crates/areka-kanade/src/schedule/steady.rs` | 935 | `on_close_request` に scope を通すと触る。**残 65 行** |
| `crates/areka-parsers/src/package/resolve.rs` | 963 | 本文内に旧形式テスト（`#[cfg(test)]` 塊）。**残 37 行** |
| `crates/areka/src/placement/spawn.rs` | 809 | ハンドラ装着は input_events 側なので触らずに済む |
| `crates/areka-kanade/src/msg.rs` | 744 | `KanadeMsg` 増分 |
| `crates/areka-ghost/src/runtime.rs` | 678 | `mount` アクセサを足すなら |
| `crates/areka/src/emo2_boot/mod.rs` | 654 | sinks に 1 本足す |
| `crates/areka/src/emo2_boot/consumer_ledger.rs` | 627 | 台帳 1 行＋enum 1 variant |
| `crates/areka-kanade/src/actor.rs` | 500 | 殻での照会 |
| `crates/areka/src/input_events/mod.rs` | 475 | 解放ハンドラ 1 本 |
| `crates/areka-kanade/src/schedule/events.rs` | 424 | `on_close` の参照列 |
| `crates/wintf/src/ecs/pointer/dispatch/mod.rs` | 260 | Released 配送 |
| `crates/areka-kanade/src/schedule/resources.rs` | 224 | 許可表 |

## 2. 要件 → 資産の対応（欠落・不明・制約）

| 要件 | 既存資産 | 状態 |
|---|---|---|
| 1.1 右ボタン解放で表示 | 解放は wintf が記録。配送なし | **Missing**: 一過性の「離した」旗＋`OnPointerReleased` 配送（wintf）＋areka の解放ハンドラ |
| 1.2〜1.4 OS 標準・閉じ方・選択後 1 回 | `TrackPopupMenuEx(TPM_RETURNCMD)` | **Missing**（新規 `menu` モジュール）。**Unknown**: フォアグラウンド作法（KB Q135788）と MTA スレッドの実機挙動 |
| 1.5 `OnMouseClick` 送らない | 送る経路が存在しない | 満たしている（現状維持） |
| 1.6 バルーン右クリック不変 | `balloon.rs` に右の扱い無し | 満たしている |
| 1.7 表示失敗の `error!` | ― | **Missing**（`GetLastError` 相当を `hresult` 形で） |
| 1.8 起動前の抑止 | `MouseWiring` 不在の self-gating | 既存パターンで足りる |
| 1.9 ドラッグ中は出さない | `snapshot_drag_state()` | 既存 API で足りる |
| 2.x 枠 7 種・並び・未登記は出さない・サブメニュー・チェック・無効・一意 ID | 無し | **Missing**: 純粋構造（登記 → 計画 → ID）。OS 非依存でテスト可 |
| 3.1〜3.4 caption の GET と 3 分岐 | `ResourceOutcome` 3 語彙・`round_trip_request`・許可表 `username` のみ | **Missing**: 許可名 13・オンデマンド照会の経路・UI 側の待ち。**Constraint**: 1 バッチ最後の応答しか再投入されない（1.3） |
| 3.5 `&` 素通し | `AppendMenuW` の既定 | 何もしない（`&&` エスケープも行わない） |
| 3.6 `char{n}` n≧2 | `EventId::Static` は `&'static` | **Constraint**: α は scope {0,1}。動的名の運び方は設計判断 |
| 3.7 `visible=0` で出さない | ― | **Missing**（照会結果の判定 1 つ） |
| 3.8 `type` は差を付けない | ― | 照会するだけ（値を捨てる）か照会しないか＝設計判断（台帳は「縮退」） |
| 3.10 起動前は既定名 | kanade の `Phase` は殻が知っている | 殻／状態機械が Boot 系で `NoContent` を即返しすれば UI は区別不要 |
| 4.1 `readme` キー | `package::resolve` は読まない・根は `cfg.ghost_root` | **Missing**: キーの転記か再読。**Constraint**: `resolve.rs` 963 行 |
| 4.2 既定アプリで開く | `ShellExecuteW` 実在 | **Missing**。**Unknown**: MTA からの `open` |
| 4.3 無ければ灰色 | ― | 計画時に `Path::exists` |
| 4.5 `\![open,readme]` 同一関数 | キャリア・台帳・`MoveCueSink` 型の前例 | **Missing**: sink 1 本＋台帳 1 行＋UI の受け口 |
| 4.6 引数付きは warn | 同上 | 同じ sink の 1 分岐 |
| 5.1 `CloseRequest{User}` 経路 | `send_close_request` | 既存関数を呼ぶだけ |
| 5.2〜5.3 Ref1/Ref2 | `on_close` は Ref0 のみ・scope の箱無し | **Missing**: `CloseRequest` から `on_close` まで scope を運ぶ（`steady.rs` 935 行に触る） |
| 5.4 拒否経路 | `close.rs` 既存 | 不変 |
| 6.x `MenuRegistry` | 無し | **Missing**（新規・純粋） |
| 7.1 死活監視 | shiori スレッド側 | 構造上満たす。表明の仕方は設計 |
| 7.2〜7.4 表示中の並行 | 1.2 の tick 構造 | **Unknown**: 表示場所（tick 内／外）で挙動が変わる |
| 7.5 IME 窓 | 記憶あり | 実機確認 |
| 8.x 記録 | `logging.md` 規約 | 新規コードの規律 |
| 9.x テスト | 1.9 の資産 | 既存の型で書ける |
| 10.x 台帳 | 1.8 | 手順どおり。並走 2 本との数の衝突は R10.5 |

## 3. 実装方針の選択肢

### 3.1 全体の形

**A. 既存に足す（最小差分）**
- wintf: `PointerState` に一過性の解放旗を足し `dispatch_pointer_events` で `OnPointerReleased` を配送。
- areka: `input_events/mod.rs` に `on_char_pointer_released` を足し、そこから新規 `menu` モジュールを呼ぶ。kanade は殻で照会を受ける。`readme` は areka が descript を再読。
- 利点: 触る箱が少ない。欠点: `input_events/mod.rs` にメニューの都合（HWND・座標・ドラッグ判定）が混ざる。`main.rs` の残 52 行を結線で使う。

**B. 新規モジュールに寄せる**
- `crates/areka/src/menu/`（`mod.rs`＝`MenuRegistry`・枠と計画の純粋関数／`win32.rs`＝HMENU 組立と `TrackPopupMenuEx`／`readme.rs`＝ファイル決定と `ShellExecuteW`／`captions.rs`＝リソース照会の写像／`*_tests.rs`）。`input_events` の解放ハンドラは「scope と座標を集めて `menu::request(world, …)` を呼ぶ」1 関数だけ。
- 結線は `menu::wire_menu(world, kanade_sender, ghost_root, app.world())` の 1 呼出に畳み、`main.rs` の増分を数行に抑える。
- 利点: 後続 4 本の spec が `MenuRegistry` だけ見れば済む。1,000 行の管理が容易。欠点: ファイル数。

**C. 混成（推奨の候補）**
- wintf の解放配送（A）＋ areka の `menu/` 新設（B）＋ kanade は殻で照会（3.3 R-A）＋ readme は parsers に転記（3.5 案①）。段階を「表示 → 説明書・終了 → 照会 → 登記 → 台帳」の順で積む。

### 3.2 右ボタン解放をどう捕まえるか

| 案 | 中身 | 評価 |
|---|---|---|
| ⑴ wintf 配送に Released を足す | `PointerState` に `released: {left,right,middle,x1,x2}`（`double_click` と同じ一過性）を足し、`transfer_buffers_to_world` で `up_received` から**独立に**立て、`dispatch_pointer_events` で `OnPointerReleased` を配送し末尾で消す | 型は既にある。既存テストの型で檻に入る。速いクリック（同 tick 押下＋解放）も取れる。**推奨** |
| ⑵ `WM_CONTEXTMENU` を wintf に足す | `WM_RBUTTONUP` を `DefWindowProc` へ流すか `WM_CONTEXTMENU` を自前で合成し、新しいハンドラ部品で areka へ | ECS のポインタ経路の外に別経路ができる。Shift+F10 も拾える利点はあるが α には過剰 |
| ⑶ 解放を areka 側で推定 | `right_down` の立ち下がりを areka が前フレームと比べる | 配送後に wintf が旗を消すので前フレームの値が残らない。不可 |

### 3.3 項目名の照会（メニューを出すたびの GET）

| 案 | 中身 | 評価 |
|---|---|---|
| R-A アクター殻で受ける | `KanadeMsg::MenuResources{scope, ids, reply}` を `actor.rs` の閉包が `step` の前で捕まえ（`Close` と同じ位置）、`Phase` が Steady 系なら `round_trip_request` を 1 件ずつ回して `Vec<(id, ResourceOutcome)>` を返す。Boot 系なら全件 `NoContent` を即返し（要件 3.10） | 状態機械の Action 列を変えない。既存の出口と許可表を通る。UI は `reply_rx.recv_timeout` で待つ。**推奨候補** |
| R-B 状態機械を通す | `Input::MenuResources` → `Action::ResourceQuery{ids, reply}` を 1 個出し、殻の `execute_actions` がそれを N 回の往復に展開して返信 | フェーズ判定が状態機械の腕に残る（純粋テストしやすい）。`Action` に variant 1 つ。R-A より数十行多い |
| R-C UI が `Sender<ShioriMsg>` を直接持つ | kanade を経ずに shiori アクターへ GET | 出口の受理規則（`round_trip_request`）を迂回する。採らない |

共通の論点:
- **許可表**: `ALLOWED_RESOURCE_IDS` に 10 名（caption 7＋`sakura|kero.popupmenu.visible`＋…）を足し、凍結テスト `allowed_resource_ids_are_exactly_username` を新しい集合へ書き換える。`popupmenu.type` 3 名を照会するか（要件 3.8 は「値で中身を変えない」＝照会しない選択も要件に反しない。台帳では「縮退」）。`char{n}` は `EventId` の第 3 出所を足すか α では持たないか。
- **待ち時間**: UI スレッドは往復の間止まる。8 件× host32 IPC。上限（例 300〜1000 ms）を超えたら既定名＋`warn!` 1 回（要件 3.4）。kanade が長い往復（終了挨拶の GET など）の最中だと待ちが積む＝上限で守る。
- **Status ヘッダ**: 殻で `snapshot_of(&state.phase)` から `ExecutionStatus::derive` を作れる（会話中なら `talking`）。

### 3.4 メニューを出す場所（モーダルループと tick）

| 案 | 中身 | 評価 |
|---|---|---|
| M-1 ハンドラ（tick 内）で `TrackPopupMenuEx` | 借用を持ったまま表示。表示中は tick も窓手続きも素通り | 最小。要件 7.2 の「描画停止は許容」で通る。窓が消える経路（7.4）は表示中に走らない（tick が回らない）ので単純。**`RefCell` 借用を握ったモーダルループ**という形は今後の負債 |
| M-2 `spawn_ui_local` の future で表示 | ハンドラは `MenuRequest{scope, hwnd, screen_pos}` を NonSend へ置くだけ。future が次の poll で借用→照会→計画→**借用を解いて**表示→再借用して動作 | 表示中もアニメが続く。表示中に `run_ghost_quit_phase` が窓を消しうる＝owner 消失時の `TrackPopupMenuEx` の戻りと、戻った後の entity 不在を耐える必要。`Rc<RefCell<EcsWorld>>` を NonSend に持つ（`app.world()`） |
| M-3 frame 相で表示 | `emo2_frame_system` の一相として `MenuRequest` を消化 | M-1 と同じく tick 内。順序の置き場が増えるだけで利点薄い |

いずれでも: 選ばれた動作は `TrackPopupMenuEx` が**戻った後**に 1 回だけ実行する（要件 1.4／6.5）。

### 3.5 `readme` キーの読み方

| 案 | 中身 | 評価 |
|---|---|---|
| ① parsers に転記 | `MountModel` に `readme: Option<String>`（`#[non_exhaustive]` ゆえ additive）、`resolve.rs` で `map.get("readme")`、`GhostRuntime::mount()` を公開して UI へ | 転記層の原則どおり・正典 URL の置き場も自然。**`resolve.rs` 963 行**＝本文内の旧形式テストを兄弟ファイルへ出す分割が同時に要る（`resolve_tests.rs` は既にあるので別テーマ名） |
| ② areka が descript を再読 | `menu/readme.rs` が `kv::parse_kv`＋charset 処理で `readme` だけ読む | parsers 無改変。同じファイルを 2 か所で読む（文字コードの扱いが二重化） |
| ③ 起動時に 1 回読んで `PathBuf` を保持 | ②を `wire_emo2_boot` 時に 1 回 | 実行時コスト 0。②と同じ二重化 |

いずれも既定名 `readme.txt`・根の直下・`Path::exists` で有効／無効（要件 4.1／4.3）。

### 3.6 `OnClose` の scope

| 案 | 中身 | 評価 |
|---|---|---|
| S-1 `CloseReason::User{scope}` | `System` は無引数のまま。`as_ref_str` は変えず、`on_close` が `User{scope}` のとき Ref1/Ref2 を積む | 箱を増やさず `pending_close`／`Phase::ClosePending` を通る。`Copy` 維持。構築点 2 か所＋テストの書き換え |
| S-2 `CloseRequest{reason, scope}` | `Input`／`pending_close`／`Phase::ClosePending`／`begin_close`／`record_pending_close` に scope を並走させる | 触る箇所が多く `steady.rs`（935 行）が伸びる |

`ForceQuit` の `on_close_notify` は要件外（Ref0 のみ据え置き）。

### 3.7 `\![open,readme]`

- `ReadmeCueSink`（talk スレッド・`CueSink`・名前 `open`＋第 1 引数 `readme` で自己選別・引数付きは `warn!`）→ `mpsc` → UI の受け口（`Emo2Wiring` に `Receiver` を足すか `menu` の NonSend に持つか）→ frame 相か `menu` の drain で `open_readme()` を呼ぶ。台帳 `("open", Some("readme"))` → `CommandConsumer::ReadmeSink`（新 variant）。
- 別案: sink の中で直接 `ShellExecuteW`（talk スレッド）。「同じ関数」は満たすが実行スレッドが 2 つになり、MTA/STA の実機確認が 2 倍になる。採らない方が素直。

### 3.8 `MenuRegistry` の口

- 登記単位（要件 6.1）: `MenuEntry{frame: Frame, label: String, caption_resource: Option<&'static str>, enabled: bool, checked: Option<bool>, body: Either<children: Vec<MenuEntry>, action: MenuAction>}`。要件 6.2 の「そのときの内容」は登記を `Box<dyn Fn() -> MenuEntry>`（供給関数）にするか、`MenuRegistry::snapshot()` 前に各登記者へ `refresh` を求めるかの 2 案。供給関数の方が後続 spec（列挙・現在の選択）に素直。
- 動作（要件 6.6）: `MenuAction` は `enum { OpenReadme, Close{scope}, Custom(Box<dyn FnMut(&mut World)>) }` の形か、全部 `Box<dyn FnMut(&mut World)>` にして本仕様の 2 項目もその形で登記するか。後者が「抜け道を作らない」に近い。
- 計画（要件 6.7／9.1）: `plan(registry_snapshot, captions) -> MenuPlan{items: Vec<Planned{id: u32, label, enabled, checked, children}>, actions: HashMap<u32, …>}` の純粋関数。ID は表示ごとに 1 から払い出す。`win32.rs` は `MenuPlan` を `HMENU` に写すだけ。

## 4. 工数とリスク

- **工数: M〜L**。M＝3.1 の A＋R-A＋M-1＋readme ②（既存に足すだけ・分割なし）。L＝C＋R-B＋M-2＋readme ①（`resolve.rs`／`steady.rs` の分割を伴う）。実機確認 6 項目（要件 9.9）と台帳の作り直しはどちらでも要る。
- **リスク: Medium**。既知の技術（Win32 メニュー）だが、⑴ 実行器の起床が隠し窓メッセージで**モーダルループ中も tick が poll される**という相互作用、⑵ MTA スレッドからの `ShellExecuteW`、⑶ フォアグラウンド作法、⑷ owner 消失時の `TrackPopupMenuEx` の戻り、は実機でしか確かめられない。⑸ 行数上限に近いファイル 3 本。

## 5. 設計フェーズへの推奨

- 推奨の骨格: **C（混成）**＝wintf に Released 配送（3.2 ⑴）／areka `menu/` 新設（3.1 B）／kanade は殻で照会（3.3 R-A・上限付き待ち）／表示は **M-1 を第 1 候補、M-2 を第 2 候補**（要件 7.2 が描画停止を許容している間は M-1 が最小。M-2 は「アニメが止まらない」を買う代わりに 7.4 の耐性を設計する）／readme は **①**（転記層の原則）だが `resolve.rs` の分割が嫌なら ③／`OnClose` は **S-1**／`\![open,readme]` は sink → UI（3.7）。
- 先に決める順: 3.4（表示場所）→ 3.3（照会経路と待ち上限）→ 3.6 → 3.5 → 3.8 の口の形。

### Research Needed（設計で調べる・実機で確かめる）

1. `TrackPopupMenuEx` を `COINIT_MULTITHREADED` の UI スレッド・`WS_EX_TOOLWINDOW|WS_EX_LAYERED` の popup owner で出したときの閉じ方（外クリック・Esc）と `SetForegroundWindow`＋`WM_NULL` の要否。
2. 表示中に tick が poll される（実行器の `WM_USER` 起床）ことの実測。M-1 なら「借用失敗で素通り」のログ（`trace!` "Re-entry blocked"）が出ることの確認。M-2 なら owner 窓 despawn 時の戻り値。
3. `ShellExecuteW("open", readme.txt)` を MTA スレッドから呼んで既定アプリが開くこと（要件 9.9 ⑵）。
4. host32 経由の GET 8 件の往復時間（待ち上限の根拠）。`R_POST_and_KOMAINU` の里々が `updatebutton.caption` 等を返すこと（要件 9.9 ⑷・`dic06_String.txt`）。
5. 既定 IME 窓と `#32768` メニュー窓の Z 順（要件 7.5・`GetClassNameW` で確認）。
6. `WM_CANCELMODE` が owner に届く条件（ドラッグ状態との相互作用）。

## 6. 設計判断項目（要件ディスカッションへ）

1. **表示をどこで呼ぶか**（3.4）: M-1（tick 内・描画停止を受容）か M-2（`spawn_ui_local`・アニメ継続・窓消失の耐性を設計）か。要件 7.2 の「設計に委ねる」の実体。
2. **照会の経路**（3.3）: R-A（殻）か R-B（状態機械の Action）か。R-C は採らない。
3. **待ち上限と失敗の扱い**: UI が kanade の返信を待つ上限（ms）と、上限超過を要件 3.4 の「失敗」（`warn!` 1 回・既定名・メニューは出す）に含めるか。
4. ~~**`popupmenu.type` を照会するか**~~ → **要件で決着（2026-09-18 要件ディスカッション）**: 照会しない（要件 3.8）。台帳は語彙のみのまま担当だけ登記（要件 10.3）。
5. ~~**`char{n}`（n≧2）の運び方**~~ → **要件で決着（同上）**: α は scope {0,1} 固定・`Static` 2 名だけ持つ（要件 3.6）。`EventId` の第 3 出所は足さない。`char*.popupmenu.visible` は語彙のみ。
   - 補足: α の第 1 スライスでメニュー 1 回あたりの往復は **3 件**（`readmebutton.caption`・`closebutton.caption`・`popupmenu.visible`）。§3.3 の「8 件前後」は 7 枠すべてが登記された後の数。
6. **`readme` キーの読み場所**（3.5）: parsers 転記（`resolve.rs` の分割込み）か areka 再読か。正典 URL コメントの置き場が変わる。
7. **`OnClose` の scope の箱**（3.6）: `CloseReason::User{scope}` か `CloseRequest{…, scope}` か。`System` の Ref1/Ref2 は積まない（現状維持）でよいか。
8. **`MenuAction` の形**（3.8）: 閉包一本化か enum＋閉包か。要件 6.6「台本の操作と同じ経路」を型で縛るか規約で守るか。
9. **登記の「そのときの内容」の取り方**（3.8）: 供給関数（`Fn() -> MenuEntry`）か、登記者への refresh 要求か。
10. **`\![open,readme]` の実行スレッド**（3.7）: UI へ送って開く（推奨）か talk スレッドで直接開くか。
11. **`main.rs` の残 52 行**: 結線を `menu::wire_menu` 1 呼出に畳むか、`main.rs` のファサード分割を本仕様で行うか（棚卸で別途か）。
12. **`ALLOWED_RESOURCE_IDS` の凍結テスト**の改訂方針: 集合の逐語一致を維持（13 名を書き足す）か、「username を含む」＋「メニュー名を含む」の 2 表明に分けるか。
13. **A0 並走との台帳衝突**（要件 10.5）: `[briefs].count` と本文の手書き数を後着側が数え直す運用で足りるか、`roadmap-draft.md` の行追加を本仕様の最終コミットまで遅らせるか。
14. **登記された文言の `&`**（要件 3.5 の裏側）: 要件 3.5 は SHIORI から返った文言を素通しにする。一方、後続 spec が登記する子項目名（ゴースト名・シェル名・バルーン名＝フォルダ名や `name`）に `&` が含まれると OS はアクセラレータと解釈して 1 文字消す。登記の口が既定名・子項目名を `&&` に写すか、登記側の責務とするかを設計で決める（前者が後続 4 本にとって安全）。
15. **要件 7.1（表示中の死活監視）の表明方法**: 構造上 SHIORI アクターのスレッド側にあり UI のモーダルループと独立（§1.2）。決定論で言うなら「UI が止まっても shiori スレッドの `on_idle` が呼ばれ続ける」を `real_idle_tests.rs` の形で 1 本、実機なら要件 9.9 ⑸ の一部として観察する。どちらで満たすかを設計で決める。
16. **右ダブルクリックの抑止の置き場**（要件 1.10・2026-09-18 追加）: `OnMouseDoubleClick(Right)` を「メニューが有効なら送らない」にするには、押下ハンドラが `popupmenu.visible` の結果を知る必要がある。解放時の照会結果を待ってから双方を決める（押下側の送出を解放側へ遅らせる）か、直前の照会結果を短く覚えるか。要件は「同時に起きない」だけを定める。
