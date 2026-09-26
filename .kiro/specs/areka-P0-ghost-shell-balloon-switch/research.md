# ギャップ分析: areka-P0-ghost-shell-balloon-switch

> 2026-09-26・要件確定後の実装ギャップ分析（`/kiro-validate-gap`）。対象のコードはブランチ `claude/areka-p0-ghost-shell-balloon-871462`（main `13b72893` と同じソース。その後の main は文書のコミットだけ）で読んだ。引用は「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。本文書は選択肢と根拠を並べるもので、最終決定は要件ディスカッションと設計で行う。

## 1. 要約

- **降ろして起こし直す部品は揃っており、本仕様が足すのは「判断」である。** `ghost_session::register_systems`（1 回）・`boot_ghost`（n 回）・`GhostSession::shutdown`・`app_exit::close_windows_for_restart` → `ghost_session::reopen_ghost_windows` は完了 `ghost-restart-unit` のテスト `boots_twice_in_one_process_without_double_registration`（`crates/areka/src/ghost_session_restart_tests.rs`）で同じ World での 2 周が実証済み。本仕様は最初の本番の呼び手になる（`#[cfg_attr(not(test), allow(dead_code))]` 2 か所を外す）。
- **停止通知の一本道が最大の障害で、分岐の置き方は 1 つの設計判断に集約される。** `run_ghost_quit_phase`（`crates/areka/src/emo2_boot/frame.rs`）は受け口 `KanadeStopRx` から停止通知を全件取り出し、原因を問わず `quit_app` を呼ぶ。通知 `KanadeStopped { cause }`（`crates/areka-kanade/src/msg.rs`）は原因 5 値だけを運び、どのゴーストの通知か・切替か終了かは分からない。**「切替の目印」をどこに持つか**（停止原因に値を足す／通知に相乗りの欄を足す／UI 側に予約を持つ）で、kanade 側の改変量・中止の伝え方・切替先の失敗の見分け方がすべて決まる（§4.1）。
- **kanade に「定常へ戻る」経路を 1 本増やすには、専用の相が要る。** `close.rs` の握手（`ClosePending` → `CloseTalkWait`）は 3 つの終わり方すべてで `Unloading` へ進み、`schedule/mod.rs` の横断の腕 `on_talk_done` は `TalkEndReason::Quit` と「中断＋終了の予約」を相を見る前に `Unloading{Quit}` へ送る。切替の相（新ファイル `schedule/change.rs`）は `close.rs` を流用できず、かつ横断の腕に「切替の相なら別扱い」の判定を差し込む必要がある（§2.3）。
- **`GhostSession` を World へ置く以外に現実的な形は無い。** `fn main`（`crates/areka/src/main.rs`）のローカル変数で `finish_after_run` の閉包が消費する。フレームの系から降ろすには NonSend 資源に置き、`run()` の後で `main` が取り出す形になる（§2.2・§4.2）。
- **汎用の通知の入口は独立した小さな増分で、切替の相とは別に作るのが自然である。** `KanadeMsg` の腕 → `Input` の腕 → `step` の 1 腕（定常だけ）→ `events.rs` の組み立て関数 1 つ → 許可表の照合、で済む。切替の握手は「返事を待って次へ進む相」を持つので、この入口では表せない（§2.4）。
- **要件の文言と実装の間に、要件ディスカッションで決めるべき境界が 5 つある**（§7 の議題 1〜5）。特に「切替の目印が下りる時点」（要件 3.5）と「切替先の SHIORI の失敗は非同期に届く」（要件 6.1・6.6）の境界、および「`boot_ghost` は失敗を返さず LogSink の起動へ倒れる」という今日の契約は、要件の言葉どおりには実装できない。

## 2. 現状調査（要件が触る既存資産）

### 2.1 停止通知の一本道（要件 3.3〜3.5・8.4）

| 定義 | 場所 | 今日の振る舞い |
|---|---|---|
| `KanadeStopRx(Receiver<KanadeStopped>)` | `crates/areka/src/emo2_boot/frame.rs` | World の NonSend 資源。プロセスに 1 つ。`emo2_boot::wire_kanade_stop` が据え、呼び手は `ghost_session::register_systems`（1 回） |
| `run_ghost_quit_phase(world) -> bool` | 同上 | 受け口が無ければ何もしない。届いていれば `try_recv` で全件取り出し、最初の 1 件の原因で `quit_app(world, ExitOrigin::KanadeStopped(cause))` を 1 度呼ぶ。2 件目以降は `debug!(event="ghost_quit_extra")` |
| `ghost_quit_system` | 同上 | 上を呼ぶだけの排他 system。`Update` に `emo2_frame_system` より前で登録 |
| `KanadeStopped { cause: KanadeStopCause }` | `crates/areka-kanade/src/msg.rs` | 原因だけ。`Debug, Clone, PartialEq, Eq` |
| `KanadeStopCause` | 同上 | `Quit`・`Forced`・`CloseSilent`・`DeadlineExceeded`・`Fault(ShioriFault)` の 5 値 |
| `TermCause`（`pub(crate)`） | `crates/areka-kanade/src/schedule/mod.rs` | 同じ 5 値。`Phase::Unloading { cause: TermCause }` が持つ |
| `stop_cause_of(&State) -> Option<KanadeStopCause>` | `crates/areka-kanade/src/actor.rs` | `TermCause` → `KanadeStopCause` の網羅 match（wildcard 無し） |
| `notify_stop(sink, cause)` | 同上 | `Action::StopSelf` の実行点で 1 回だけ送る。原因が控えられなければ `Fault(unknown)` として送る |
| `fault_of(&ExitOrigin) -> Option<&ShioriFault>` | `crates/areka/src/app_exit.rs` | `KanadeStopped(Fault(f))` だけ `Some`。他は網羅列挙で `None` |
| `quit_app(world, origin) -> usize` | 同上 | 全窓を閉じ、`FirstExit` を最初の 1 回だけ挿し、`AppExit::request_exit()` |

確認できた事実:

- **降ろした側の通知は、どの降ろし方でも 1 件だけ来る。** `GhostRuntime::shutdown`（`crates/areka-ghost/src/runtime.rs`）は `KanadeMsg::ForceQuit` を送るが、kanade が既に止まっていれば送出は失敗して `debug!` で流れる。正典どおりの握手を通して kanade が自分で `StopSelf` に至った場合、その 1 件が届いたあとに `GhostSession::shutdown` を呼べば `ForceQuit` は空振りし、2 件目は出ない。**2 件になるのは「通知を待たずに `shutdown` を先に呼ぶ」形だけ**（完了 `ghost-restart-unit` の 2 周テストはこの形で、判定の中で通知を自分で読み捨てている）。→ 切替の経路を「通知を受けてから降ろす」順に組めば、要件 3.3 の「どちらも同じ規則で捌く」は自然に満たされ、残り物の問題も出ない。
- **通知に「どのゴーストか」を載せる器は無い**（要件 3.4）。`spawn_kanade_with_stop_sink` は `Sender<KanadeStopped>` の写しをそのまま持つだけで、世代も名前も知らない。
- `frame_ghost_quit_tests.rs`（`crates/areka/src/emo2_boot/`）は `World::new()` ＋ `AppExit::new()` ＋ `KanadeStopRx` ＋ `GhostWindowMarker` だけで `run_ghost_quit_phase` を回す（実窓なし）。要件 10.7 の兄弟テストはこの形をそのまま使える。

### 2.2 `GhostSession` の置き場（要件 3.6）

- `GhostSession`（`crates/areka/src/ghost_session.rs`）は `ghost: Option<GhostRuntime>`・`seriko: Option<ActorHandle>`・`loop_ticker: Option<Sender<TickerMsg>>` を私有の欄で持ち、`shutdown(self, CloseReason)` が self を消費する。`ghost_name()` は告知の場面のために `mount().names.name` を返す。
- `fn main` はこれをローカル変数 `session` に受け、`app.run()` の後で `session.ghost_name()`（告知の場面）→ `finish_after_run(run, fault, move || { session.shutdown(...) ... })` の閉包へ move する。
- `GhostRuntime` の欄は `Sender`・`ActorHandle`・`SylphyaPublisher`・`SylphyaReader`・`MountModel`。`Send` かどうかは確認していない（§6 の Research）。World の NonSend 資源に置くなら `Send` は要らない。
- **`GhostSession::shutdown` は UI スレッドで同期に待つ。** 中の `GhostRuntime::shutdown` は kanade → dispatcher → shiori（本番は 32bit helper プロセスの unload を待つ）→ relay 2 本 → ticker → sylphya の順で join し、その後 seriko を join する。要件 3.8 の 1 秒目標はこの合計時間で決まる。未実測。

### 2.3 kanade の握手（要件 2・5・4.1〜4.5）

`schedule/mod.rs` の `step` は「横断の腕を先に判定 → 該当しなければ `dispatch_phase` で相ごとの `step`」の順である。本仕様に関わる横断の腕:

| 入力 | 今日の横断の判定 | 切替の相での必要な振る舞い |
|---|---|---|
| `Input::TalkDone` で `reason == Quit`（台本が `\-` に達した） | 相を見ずに `to_unloading_quit` → `Unloading{Quit}` ＋ `ShioriUnload` | 切替の送り出しの台詞が `\-` を含めば、降ろすことで終わればよい（結果は同じ `Unloading`）。ただし目印の持ち方によっては原因を切替の値にしたい（§4.1 の案 ①） |
| `Input::TalkDone` で `reason == Interrupted` かつ `take_user_break_quit` が真 | 同上（`talk_done_break_quit`） | **切替の相では中止して定常へ戻す**（要件 5.1・5.4）。横断の腕より先に「切替の相か」を見る判定が要る |
| `Input::TalkDone` で `reason == Interrupted`・`Ended` | 相ごとの `step` へ | 切替の相の `step` が受ける（`Ended` → 降ろす／`Interrupted` → 中止して定常へ） |
| `Input::ForceQuit` | 常に `Unloading{Forced}` | 変えない（`GhostSession::shutdown` の空振りは §2.1） |
| `Input::ShioriDown`・`ShioriReply{Failed}` | `Unloading{Fault}` | 変えない（要件 2.8 は UI 側で「目印が立っていれば続ける」） |
| `Input::UserBreak` | `user_break::on_user_break` が再生中なら `Action::CancelChoice` を返し `user_break_talk` を控える | 変えない。中止の判断は完了通知（`TalkDone{Interrupted}`）で行う |
| `Input::CloseRequest` | `dispatch_phase` | 切替の相に届いたら？（要件は沈黙。§7 議題 8） |

- `close.rs` の `on_close_pending` / `on_close_talk_wait` は `Value` → `StartTalk`＋`CloseTalkWait`、`NoContent` → `Unloading{CloseSilent}`、`TalkDone{Ended|Interrupted}` → `Unloading{Quit}`、期限 → `Unloading{DeadlineExceeded}`。**定常へ戻る腕は 0 本**で、切替の「204 のあとの `OnClose`」に流用すると中断で降ろしてしまう（要件 5.1 に反する）。→ 切替の相は `OnClose` の段も自分で持つ（`ClosePending`／`CloseTalkWait` の写しに「中断 → 定常」の腕を足した形）。`deadline_from`（30 秒）はそのまま使える。
- `Phase`（`pub(crate)` enum）に値を足すと、`phase_label`・`dispatch_phase`・`current_talk_id`・`snapshot_of` の match に腕が要る（`dispatch_phase` と `phase_label` は wildcard 無し・他は `_ =>` あり）。`State` には `pending_close: Option<CloseReason>` の先例があり、「再生中に受けた指示を完了まで保留する」形はそのまま写せる（`raise-event` 無しの切替＝要件 2.6・5.5）。
- `boot.rs` の `on_prefetch_reply` が起動系列の分岐点: `config.first_boot` が真なら `OnFirstBoot` GET → `BootType`、偽なら `OnBoot` GET → `BootMain`。**`OnGhostChanged` の差し込み点はここ**（`first_boot` の判定より先に「切替で起きたか」を見る）。`to_baseware_version` は `first_boot` が真なら起動記録を書く指令（`first_boot_epilogue`）を起動の台詞の末尾に添え、台詞が無くても指令だけの台本を起こす——**`OnFirstBoot` を送らずに `first_boot` を真のまま残せば、起動記録は今日の規則で書かれる**（要件 4.4 の「起動記録は同じ規則で書く」は既存の仕組みで満たせる）。
- `KanadeConfig`（`msg.rs`）は `new(shell_name, baseware_version)` で組み、`shell_name` が `OnBoot` の Ref0（＝要件 4.1 の Ref7「切替先のシェルのフォルダ名」と同じ値）。構造体リテラル `KanadeConfig {` は 17 か所（ほぼテスト）・`KanadeConfig::new(` は 65 か所。直前のゴーストの情報を kanade B へ渡す器は無い（§4.3）。

### 2.4 `KanadeMsg` と汎用の通知の入口（要件 7）

- `KanadeMsg`（`msg.rs`）は 12 変種で、名前と Reference 列で SHIORI イベントを頼む変種は無い。`actor.rs` の `spawn_kanade_with_stop_sink` は `KanadeMsg` → `Input` を 1 対 1 で写す（`Close` と `ResourceQuery` だけは `step` を経ずその場で処理）。
- `ALLOWED_EVENT_IDS`（`schedule/events.rs`）は 11 語（`OnInitialize`・`OnFirstBoot`・`OnBoot`・`basewareversion`・`OnSecondChange`・`OnClose`・`OnMouseMove`・`OnMouseDoubleClick`・`OnChoiceSelectEx`・`OnChoiceSelect`・`OnChoiceTimeout`）。`round_trip_request`（`actor.rs`）が送出直前に `is_allowed_event_id` で照合し、通らなければ `error!(event="event_id_not_allowed")`＋`Failed(Internal)`（→ 横断の腕で `Unloading{Fault}`＝**今日は許可表に無いイベントを頼むと終了系列へ進む**。要件 7.2 の「送らず `warn!`」は入口側で先に弾く必要がある）。
- `EventId` は `Static(&'static str)` と `Choice(String)` の 2 値。`ShioriReply { origin: &'static str }` の出所は `&'static str`。汎用の入口が受ける名前は `String` だが、**許可表の照合で一致した `&'static str` を取り出せば `EventId::Static` と `origin` の両方に使える**（許可表が `&[&'static str]` なので新しい型は要らない）。
- `steady.rs` の `on_reply` は「再生中（`talk: Some`）に `Value` が返ったとき」を出所で分ける（マウス系は置き換え・それ以外は `warn!`＋捨てる）。この match は意図して wildcard を置かない（第 3 の出所が来たらレビューで政策判断を要求する設計）。汎用の入口の応答が再生中に返ったときの扱いは決まっていない（§7 議題 4）。`on_mouse` は `pending_close` があれば GET を出さない。
- 定常以外で頼まれたら捨てる（要件 7.4）は `Input::Choice` の非定常の腕（`warn!`＋状態不変）と同じ形で書ける。

### 2.5 台本の受け口（要件 1.2・1.3・1.6・1.10）

- `ConsumerLedger::canonical()`（`crates/areka/src/emo2_boot/consumer_ledger.rs`）の登記は 8 組で `(change, …)` は無い。`try_register(name, Some(selector), CommandConsumer::…)` の形。`CommandConsumer` に新しい値（例 `ChangeSink`）を足す。
- 前例 `ReadmeCueSink`（`emo2_boot/readme_cue.rs`）: `CueSink::emit` で `cue.command.as_command_carrier()` を開封し、`(name, selector) == ("open", "readme")` を自己選別、`Sender<ReadmeRequest>` で UI へ送る。受信端は `readme::wire_readme` が World の NonSend 資源に据え、`Input` の段の `drain_readme_requests`（登録は `register_readme_drain`・1 回）が取り出す。`\![change,ghost,名,--option=raise-event]` は `command_carrier("change", ["ghost","名","--option=raise-event"])` として届く（`dola::cue::CueCommand::command_carrier` はトークンを無変形で保持する）。
- 受け口は boot より前に組まれ（`wire_emo2_boot` の手順 4）、kanade の `Sender` は boot の後にしか得られない。→ 受け口は UI へ送る（`user_break` と同型）。**名前の突き合わせ（要件 1.5〜1.7）は talk スレッドの受け口ではなく UI 側の取り出しで行う**（目録の読み取りは fs I/O。`ghost-change-name-resolution` が置き換える「解決できない名前の腕」の所在に影響＝§7 議題 9）。
- `wire_emo2_boot` は 767 行。線 1 本＋受け口 1 つ＋`sinks` の 8 本目を足して約 +30 行。

### 2.6 メニュー（要件 1.4・1.11・1.12・10.9）

- `menu::Frame::Ghost`・文言 `ghostrootbutton.caption`（`menu/captions.rs` の `FRAME_CAPTIONS`）・`ItemBody::Submenu(Vec<MenuItem>)`・`MenuRegistry::unregister`・`menu::register(world, frame, supplier)` が在り、後の 3 つに `#[allow(dead_code)]`。**`Submenu` は `plan.rs` の `PlanEntry::Submenu` と `win32.rs` で表示まで結線済み**（作る側が居ないだけ）。
- `Supplier` は `Rc<dyn Fn(&World, &MenuContext) -> MenuItem>` でメニューを出すたびに呼ばれる（目録をそのとき読める）。`MenuAction` は `Rc<dyn Fn(&mut World, &MenuContext)>` で、`request_close` が `MouseWiring::send_close_request` へ送る形の先例がある（切替要求も同じ口＝`MouseWiring` に送る関数を 1 つ足すか、専用の小さな資源）。
- `MenuWiring` は `boot_ghost` → `menu::wire_menu` で起こすたびに新品になり、組込 2 項目（説明書・終了）だけを登記する。「ゴースト」枠の登記は `boot_ghost` の中（`wire_menu` の直後）が自然な置き場。
- **目録を引くのに `BasewareRoot` が要るが、World にもどの資源にも無い**（`resolve_boot_from` のローカル値。`ConfigInputs` は `ghost_root`・`balloon_root` の 2 パスだけ）。→ 根とアプリのプロファイルの置き場を切替の文脈として World に置く必要がある（Missing）。

### 2.7 目録・記憶・起動解決・バルーン（要件 1.5・4.6・4.7・8.7）

- `catalog::list_ghosts(root) -> Vec<GhostEntry>`（`crates/areka-ghost/src/catalog.rs`）はフォルダ名のバイト順。`Identity` は `folder`・`name`・`craftman`・`craftmanw`・`id`・`readme`・`has_thumbnail` の 7 項目で `sakura.name` は無い。`identity()` の `name` は `parse_kv` の値そのまま（鍵は小文字化・値は無変形）。
- `companion_balloon(ghost_dir)` は `install.txt` を `lowercased` で読む単独の読み手。**`ghost/master/descript.txt` の `sakura.name` を読む同型の関数を 1 つ足す**（Ref0 用・要件 8.7）。`MountModel.names`（`GhostNames { name, sakura_name, sakura_name2, kero_name }`）は起動済みのゴースト（直前のゴースト）なら `GhostRuntime::mount()` から読める。
- `boot_resolve::resolve_balloon`・`read_last_balloon(ghost_dir)`・`companion_balloon`・`list_balloons` は揃っているが、それを繋ぐのは `boot_config::resolve_boot_from` の中で argv・根の解決と一体（`pub(crate)`）。→ 「切替先のフォルダに対するバルーンの解決」の小さな包みが要る（要件 4.7・Missing・十数行）。
- `LastUsed::record`（`boot_resolve.rs`）は `ghost.route == GhostRoute::Argv` なら書かない。`GhostRoute` は `Argv`・`Memory`・`Only`・`Default`・`Random`。切替後は `Argv` 以外を渡す（新しい値 `Switched` を足すか既存値を流用するかは設計）。書き込みは `main::on_boot_ok` が `boot_ghost` の両腕から呼ぶ（そのまま使える）。
- 元へ戻すとき（要件 6.2）`LastGhost` を「元のゴーストのまま」にする＝元を起こし直す `boot_ghost` にも `on_boot_ok` が走るので、元のフォルダ名で上書きされる（結果は同じ値・要件は満たす）。

### 2.8 告知（要件 6.3・6.4）

- `AlertScene`（`crates/areka/src/alert.rs`）は 5 場面。`alert_text` は網羅 match で題名を場面ごとに決める（`TITLE`・`SHIORI_FAULT_TITLE`）。場面を 1 つ足せば題名・本文の 3 行を足すだけ。`raise` はボタンを返さない（本仕様に不要）。
- 告知を出す位置は `fn main` の `run()` の後: `FirstExit`（`app_exit.rs`）→ `fault_of` → `Some(ShioriFault)` のときだけ `AlertScene::ShioriFault` を組み `finish_after_run(run, fault=true, …)` で終了コード 1。**二重失敗を同じ後始末で告知するには `ExitOrigin` に値（切替の二重失敗＝切替先と元の名前と失敗の種類）を足し、`fault_of` の網羅 match と `main` の場面の組み立てを広げる**（Missing・小）。二重失敗のときは `session.ghost_name()` が `None` になりうるので、名前は切替の記録から取る。

### 2.9 切替先の起動失敗の観測（要件 6.1・6.6）

- **`boot_ghost` は失敗を返さない。** `wire_emo2_boot` が `Err`（資産の組立の失敗・`boot_with_kanade_stop` の `Err`＝マウントの失敗など）なら `wired=false` で戻り、`boot_ghost` は `LogSink`×2 の fallback の起動へ倒れる（それも失敗なら `ghost: None` の `GhostSession`）。今日の契約は「起動の失敗は非致命・骨格起動は止めない」。→ 切替先の「起動解決の失敗・マウントの失敗」を要件 6.1 の言うとおり検出するには、`boot_ghost`（か切替専用の派生）が「実 sink の結線が成立したか／ゴースト実行系が居るか」を返す必要がある（Constraint→要件討議の議題 2）。
- **SHIORI の失敗（`Fault`）は非同期に届く。** `spawn_shiori_actor` の connect 閉包はアクタースレッドで動き、失敗は `ShioriDown{ConnectFailed}` → kanade `Unloading{Fault}` → `StopSelf` → 停止通知、という順で**数フレーム後**に `KanadeStopRx` に届く。`boot_with_kanade_stop` は `Ok` を返している。→ 「切替先の起動が失敗した」と「起動した切替先があとで失敗した」を UI が見分ける手段は今日無い（§7 議題 1）。
- `derive_scopes()`（`emo2_boot/mod.rs`）は `[0, 1]` 固定・`placement::config::detect_scopes` は `kero.*` の有無で 1 を足す。1 人のゴーストでは窓 1 組＋資産 2 スコープになり、`plan_attachments` が余りを分類して落とすので落ちない（要件 4.9 は据え置きで通る見込み・実機で確認）。

### 2.10 テストの土台（要件 10）

- 偽の SHIORI: `ScriptedShioriBackendBuilder`（`crates/areka-ghost/tests/ghost/spine_e2e_test.rs`・`get(id, Ok(Some(script)))`／`notify`／`unload` を FIFO で積む）と `ShioriWiring::Custom`。`SpineHarness::standard_backend`（`emo2_boot/spine.rs`）が起動系列の標準台本を組む。**2 体目の台本は別の `ScriptedShioriBackend` を用意して切替先の `Emo2BootInputs.shiori` へ渡す**だけで作れる。切替先の失敗は `Custom(|| Err(..))`（接続に失敗する偽 SHIORI）。
- 同じ World での 2 周: `ghost_session_restart_tests.rs` の手順（`World::new()`・`Schedules`・`AppExit::new()`・`register_systems` → `boot_round` → `shutdown` → `close_windows_for_restart` → `boot_round`）。**実窓は作れない**（`open_ghost_windows` は `WintfTaskPool`（`EcsWorld::new` が挿す）と実モニタを要し、テストは `WindowsClosed` を読んで捨てている）。要件 10.1 ⑷「窓ごとの状態が B のもの」は配線状態（`ReadmeWiring`・`MenuWiring` など）で判定する形になる（§7 議題 10）。
- 停止通知の相: `frame_ghost_quit_tests.rs`（`world_with_ghost_windows`・`exit_requested`）。フレームを回す: `SpineHarness`（`emo2_boot/spine.rs`・GPU World・`run_attach_phase` など）。
- kanade の純粋な `step` のテスト: `schedule/close.rs` の `mod tests`（`close_pending(..)`・`close_talk_wait(..)` の状態を直に組む）が切替の相のテストの写し元。`steady_test_support.rs`／`boot_test_support.rs` が状態の組み立てを持つ。
- 常設 smoke: `crates/areka/tests/smoke_boot_loop_exit.rs` は `#[test]` 4 本（argv・根・空の根・Fault）。本仕様は触らない。
- 行数（上限 1,000 の目安）: kanade `steady.rs` 935・`actor_tests.rs` 970・`schedule_tests.rs` 962・`steady_flow_tests.rs` 931・areka-ghost `runtime_tests.rs` 986・`spine.rs` 971・`spine_conformance_lap_tests.rs` 990 は足さない。`msg.rs` 852・`emo2_boot/mod.rs` 767・`schedule/mod.rs` 758・`consumer_ledger.rs` 726・`close.rs` 636・`main.rs` 556・`frame.rs` 474・`ghost_session.rs` 454 は今回の増分（各 +10〜+60）で収まる。

## 3. 要件 → 資産の対応表

| 要件 | 既存資産 | ギャップ | 種別 |
|---|---|---|---|
| 1.1〜1.3・1.10 台本の入口 | `ConsumerLedger::canonical`・`ReadmeCueSink` の型・`command_carrier` | `(change, ghost)` の登記・`change_cue.rs`・UI の受信端と取り出し | Missing |
| 1.4・1.11・1.12 メニューの「ゴースト」枠 | `Frame::Ghost`・`Submenu`（表示まで結線済み）・`menu::register`・`MenuWiring` の n 回契約 | 供給関数・動作・`BasewareRoot` を World へ置く器 | Missing |
| 1.5〜1.7 名前の突き合わせ | `catalog::list_ghosts`・`Identity.name/folder` | 突き合わせの純関数（`name` → フォルダ名・大文字小文字区別）・降ろす前の判定位置 | Missing（小） |
| 1.8 自分自身への切替 | 2 周テストで同じ検体を 2 度起こす実証あり | 例外を作らないだけ | なし |
| 1.9 切替中の二重要求 | `pending_close` の先例 | 目印の置き場で判定場所が決まる | Missing（設計） |
| 2.1〜2.5 送り出しの握手 | `close.rs` の形・`deadline_from`・`events::on_close` | `OnGhostChanging` の組み立て関数・切替の相（新ファイル）・横断の腕の例外 | Missing |
| 2.6 `raise-event` 無しの待ち | `State.pending_close` | 「保留中の切替」の欄と `TalkDone` での消化 | Missing |
| 2.7 SHIORI の解放順 | kanade `Unloading` → `ShioriUnload` → `StopSelf` → `ShioriMsg::Close`・`GhostRuntime::shutdown` の join | 「通知を受けてから降ろす」順に組めば構造で満たす | なし（順序の制約） |
| 2.8 握手中の `Fault` | `Unloading{Fault}` → 通知 | UI が目印で「続ける」と判断 | 目印に依存 |
| 3.1・3.2・3.7・3.9 降ろして起こし直す | `GhostSession::shutdown`・`close_windows_for_restart`・`reopen_ghost_windows`・`boot_ghost`・`windows_closed_for_restart` の語彙 | `allow(dead_code)` を外す・呼び手 | なし（呼び手だけ） |
| 3.3〜3.5・8.4 停止通知の分岐 | `run_ghost_quit_phase` | 切替の分岐・目印・どのゴーストの通知かの区別 | Missing（設計の中核） |
| 3.6 `GhostSession` の置き場 | `main` のローカル・`finish_after_run` | World の NonSend 資源＋`main` の取り出し | Missing（構造） |
| 3.8 1 秒以内 | `GhostSession::shutdown` は同期 join | 未実測（helper の unload を含む） | Unknown |
| 4.1〜4.5 迎え入れの握手 | `boot.rs` の `on_prefetch_reply`・`to_baseware_version`（起動記録の指令） | `OnGhostChanged` の組み立て関数・起動系列の腕・直前のゴースト情報を kanade B へ渡す器 | Missing |
| 4.6 記憶 | `LastUsed::record`・`on_boot_ok` | `GhostRoute` を `Argv` 以外で渡す | なし（値の選び方） |
| 4.7 切替先のバルーン | `resolve_balloon`・`read_last_balloon`・`companion_balloon`・`list_balloons` | argv 無しで 1 ゴースト分を解く包み | Missing（小） |
| 4.8・4.10 窓ごとの状態と窓の手順 | 2 周テストで実証・`reopen_ghost_windows` | なし | なし |
| 4.9 1 スコープのゴースト | `derive_scopes` 固定・`plan_attachments` の縮退 | 実機で確認 | Unknown（低） |
| 5.1〜5.7 中止して定常へ | `on_user_break`・`take_user_break_quit`・`CancelChoice` | 切替の相の `Interrupted` の腕・横断の腕の例外・`info!` | Missing |
| 6.1・6.2・6.5 元へ戻す | `boot_ghost`（n 回） | **起動失敗の観測口**（今日は fallback に倒れる）・非同期 `Fault` の見分け・1 回だけの試みの状態 | Constraint→議題 |
| 6.3 告知なし | — | なし | なし |
| 6.4 二重失敗の告知 | `AlertScene`・`alert_text`・`FirstExit`／`fault_of`・`finish_after_run` | 場面 1 つ・`ExitOrigin` の値 1 つ・`main` の場面の組み立て | Missing（小） |
| 6.6 切替先の `Fault` を戻す経路へ | `run_ghost_quit_phase` | 目印の持ち方に依存 | 目印に依存 |
| 6.7 エラー応答は致命でない | `round_trip_request` の写し | なし | なし |
| 7.1〜7.6 汎用の通知の入口 | `KanadeMsg`→`Input` の写し・`ALLOWED_EVENT_IDS`・`steady::on_reply` | 変種 1 つ・腕 1 つ・組み立て関数 1 つ・入口側の許可表の照合・再生中の応答の政策 | Missing（独立・小） |
| 8.6 `GhostBootOptions` に欄を足さない | 構造体リテラル 27 か所（17 ファイル） | 派生関数で渡す | Constraint |
| 8.7 `sakura.name` の単独の読み手 | `companion_balloon` の形 | 同型 1 関数 | Missing（小） |
| 9.1 台帳 | `shiori.toml` 2 行・`sakura-script.toml` 1 行（`absent`・owner 空）・生成器 `cargo run -p ukadoc-survey -- report` | 更新と再生成 | なし（手順） |
| 9.2 §8 | `doc/COMPAT_ARCHITECTURE.md` §8 の表（項目・裁量・根拠・出典 spec の 4 列） | 7 行を足す | なし（手順） |
| 10.x 決定論テスト | §2.10 | 切替の 1 周・相の分岐・入口の分岐・メニューの登記 | Missing（テスト） |

## 4. 実装の分かれ目と選択肢

### 4.1 「切替の目印」をどこに持つか（要件 3.3〜3.5・3.4・5.2・6.6・1.9 を一度に決める）

| 案 | 形 | 利点 | 難点 |
|---|---|---|---|
| ① 停止原因に切替の値を足す | `TermCause::Changed(手渡しの中身)`・`KanadeStopCause::Changed(..)`。切替要求は kanade へ送り、kanade が握手の末に原因＝切替で止まる。UI は原因で分岐 | 目印が 1 か所（kanade の相）。中止は kanade の中で完結し UI へ何も伝えなくてよい。二重要求の無視も kanade で `warn!`。`OnGhostChanged` の Ref1（`OnGhostChanging` の台本）も同じ通知に載る。`stop_cause_of`・`fault_of` の網羅 match が漏れを止める | **握手中の `Fault`（要件 2.8）で切替の情報が落ちる**（`TermCause::Fault` に相乗りの欄を足すか、`Fault` は今日どおり終了へ流すかの判断）。**切替先の起動失敗（要件 6.1・6.6）は kanade B の通知に切替の情報が無い**ので、UI 側に「直前に切替で起こした・元は誰か」の状態が別に要る |
| ② UI 側に予約を持つ | UI の NonSend 資源 `SwitchInFlight { 元, 切替先, 段 }`。kanade へは切替要求を送り、停止通知の原因は今日の 5 値のまま。中止は kanade から UI へ返事（`ResourceQuery` の `reply` の形か、`menu/trigger.rs` の `PendingQuery` のように毎 tick 覗く受信端） | kanade の停止の語彙が不変。`Fault` も「予約があれば続ける」で一様に捌ける。切替先の失敗も同じ資源の「段」で見分ける | 中止を伝える返事の線が 1 本増える（`ReplySender` は 1 回きり・「受理」と「中止」の 2 回は返せないので、終端の 1 回＝「中止した／降ろす（台本つき）」だけを返す形）。目印が UI と kanade の 2 か所に分かれる |
| ③ 通知に相乗りの欄を足す | `KanadeStopped { cause, change: Option<手渡しの中身> }`。kanade の `State` に「受理した切替」を控え、`notify_stop` が原因と一緒に載せる。`stop_cause_of` は不変 | ①の利点＋`Fault` でも切替の情報が残る（原因と欄が独立）。中止は kanade の中で完結 | 切替先の失敗（6.1・6.6）は依然 UI の状態が要る。`KanadeStopped` の構築点（テスト含む）に欄が増える |

いずれの案でも **要件 6.1・6.6（切替先の失敗で元へ戻す）には UI 側の「直前に切替で起こした」状態が要る**。したがって現実的な組み合わせは「①か③（送り出しの側）＋ UI の小さな状態（迎え入れの側）」か「②（両側を UI に集める）」の 2 通り。brief の推し（①）は送り出しの側に限れば最少だが、`Fault` の扱いで③へ寄る。

### 4.2 切替の実行の場所と順序（要件 3.1〜3.7・2.7）

推奨の順（案 A・拡張）: `run_ghost_quit_phase` の中で「目印が立っている停止通知」を受けたら、その場で ⑴ `GhostSession`（World の資源から取り出す）`.shutdown(reason)` → ⑵ `close_windows_for_restart` → ⑶ 切替先の起動解決（バルーン）→ ⑷ `reopen_ghost_windows(cfg_B)` → ⑸ `boot_ghost(B, 直前の情報)` → ⑹ 資源へ戻す。`ForceQuit` は空振り（§2.1）し通知は 1 件で済む。⑴ が UI スレッドを塞ぐ（要件 3.8）。

別案（案 B・新規）: ⑴ を別スレッドで行い、完了を次のフレームで受けて ⑵〜⑹ を行う（`GhostSession` が `Send` である必要・フレームをまたぐ状態が 1 つ増える・「降ろし中に届く入力」の扱いが増える）。要件 3.8 の実測が 1 秒を超えたときの逃げ道として設計に残す。

`run_ghost_quit_phase` は `frame.rs`（474 行）にあり、切替の分岐の本体は新ファイル（例 `emo2_boot/frame/ghost_switch.rs`）へ置き、`run_ghost_quit_phase` からは「目印が立っていれば委譲」の数行だけにするのが 1,000 行の目安と要件 8.4（分岐を限る）に合う。

### 4.3 直前のゴーストの情報を kanade B へ渡す口（要件 4.1・8.6）

| 案 | 形 | 難点 |
|---|---|---|
| ⓐ `KanadeConfig` に欄 `changed_from: Option<…>` | `resolve_kanade_config` の後で `boot_with_kanade_stop` の派生関数が詰める。`boot.rs` はこの欄で分岐 | `KanadeConfig {` の構造体リテラル 17 か所（ほぼテスト）に欄が増える。`KanadeConfig::new` 経由（65 か所）は不変 |
| ⓑ `spawn_kanade_with_stop_sink` の引数に `Option<…>` を足し `State::initial` に持たせる | config を触らない | `State` の構築点と `Phase` の初期値に影響。テスト支援 `boot_test_support.rs` の写し |
| ⓒ `KanadeMsg::Boot` に載せる（`Boot { changed_from }`） | 器が最小 | `KanadeMsg::Boot` の送出点（`areka-ghost` の boot 系列）とテストの `Boot` 全部が変わる |

いずれも「派生関数で渡す」（要件 8.6・前例 `boot_with_kanade_stop`）の形は保てる。

### 4.4 切替の相の置き方（要件 2・5）

- 新ファイル `crates/areka-kanade/src/schedule/change.rs`（前例 `user_break.rs`・`close.rs`）に、`Phase` の新しい値（例: `ChangePending{req}`・`ChangeTalkWait{req, talk_id, deadline}`・`ChangeClosePending{req}`・`ChangeCloseTalkWait{req, talk_id, deadline}`、または `Changing { req, stage }` の 1 値）と `step` を置く。`raise-event` 無しの切替は `State` に `pending_change: Option<req>` を持たせ、`Steady` の `TalkDone` で消化する（`pending_close` と同型）。
- `schedule/mod.rs` の横断の腕 `on_talk_done` に「切替の相なら中断は中止（定常へ）・終了の予約は効かせない」の判定を足す（要件 5.4・衝突表 2）。`dispatch_phase`・`phase_label`・`current_talk_id`・`snapshot_of` へ腕を足す。
- `events.rs` に `on_ghost_changing(req, snapshot)`・`on_ghost_changed(from, snapshot)` を足し、許可表に 2 語を足す。

### 4.5 汎用の通知の入口（要件 7）

- `KanadeMsg::RaiseEvent { id: String, references: Vec<String>, method: Get|Notify }` → `Input::RaiseEvent{..}` → `step`: 定常なら `events::raise(..)`（許可表で一致した `&'static str` を `EventId::Static` に）を `Action::ShioriRequest` に、定常以外は `warn!`＋捨てる。許可表に無ければ入口で `warn!`＋捨てる（`round_trip_request` の `error!`＋`Fault` へ流さない）。
- 応答は `steady::on_reply` へ流れる。`talk: None` なら再生、`talk: Some` の政策は要決定（§7 議題 4）。
- `OnGhostChanging`／`OnGhostChanged` はこの入口では表せない（応答の後に相が進む）ので、切替の相の専用の腕で送る（要件 7.5 への答えの候補）。

### 4.6 全体の案の比較

| 案 | 中身 | 向く条件 | 主な難点 |
|---|---|---|---|
| A 拡張（既存の型と相に腕を足す） | 目印①か③・実行は `run_ghost_quit_phase` の中・相は kanade の新ファイル・`GhostSession` は World の資源 | 完了 `ghost-restart-unit` の形をそのまま使う・行数の余裕がある | 横断の腕への例外・`Fault` 時の切替情報・UI 側の「迎え入れ」状態 |
| B 新規（切替の統括を新しい単位に） | UI に `GhostSwitcher` 資源（段・元・先・返事待ち）を置き、kanade へは要求と返事の 2 線。降ろすのは別スレッド | 1 秒目標を超える見込みが強い・後続 3 spec が同じ統括を使う | フレームをまたぐ状態が増える・`Send` の確認・入力の扱い |
| C 混成 | A を基本に、UI の「迎え入れ」状態だけを小さな資源に切り出し、降ろすのは同期（実測で超えたら B へ） | 本仕様の規模 L に収める | 目印が 2 か所（送り出し＝kanade・迎え入れ＝UI） |

## 5. 規模とリスク

- **規模: L**（brief の 14〜16 タスクと整合）。kanade の相 1 ファイル＋横断の腕の例外＋起動系列の腕（3〜4 タスク）、UI の受け口・取り出し・名前の突き合わせ・メニュー（3 タスク）、停止通知の分岐と `GhostSession` の置き場・起こし直し・戻す経路・告知（4 タスク）、汎用の入口（1〜2 タスク）、台帳と §8・実機（2 タスク）。
- **リスク: 中〜高**。理由: ⑴ kanade の状態機械に「定常へ戻る」新経路と横断の腕の例外を足す（既存の決定論テストが多く、字面で固定しているものは追随が要る）、⑵ UI スレッドの停止時間が未実測（1 秒目標）、⑵′ 切替先の失敗が非同期に届き要件 3.5 と 6.6 の境界が未定、⑷ `boot_ghost` の「失敗しない」契約を切替では変える必要がある。

## 6. 設計フェーズへの推奨と要調査

**推奨**: 案 C（A を基本・迎え入れの状態だけ UI の小さな資源）。送り出しの目印は ③（通知に相乗りの欄）を第一候補にする——①の利点（中止が kanade の中で完結・二重要求も kanade で弾ける・Ref1 の台本を同じ通知で運べる）を保ちつつ、`Fault` でも切替の情報が落ちない。降ろすのは同期（`run_ghost_quit_phase` の中）で始め、実機の計測で 1 秒を超えたら B の別スレッド化へ。

**Research Needed（設計で引き直す）**:
1. `GhostSession::shutdown` の実機の所要時間（emo2・R_POST_and_KOMAINU・helper 経由）。要件 3.8。
2. `GhostRuntime`／`GhostSession` が `Send` か（B 案の可否）。
3. 切替先の `Fault` が届くまでのフレーム数と、kanade B が定常に達したことを UI が知る手段（今日は無い。候補: 通知の型を「停止」以外にも広げる／`ResourceQuery` に運行状態を足す／時間で区切る）。
4. `steady::on_reply` の再生中の政策に汎用の入口の出所を足すときの扱い（置き換え／捨てる／待つ）。
5. `KanadeStopped` の構築点（テストを含む）の数——③の欄を足す影響の範囲。
6. 1 スコープのゴーストへの切替で `plan_attachments` の縮退が実機で問題なく通るか（要件 4.9）。
7. 台帳の生成器（`cargo run -p ukadoc-survey -- report`／`report-summary`）の実行手順と `check` の通り方（並走 spec の `.kiro/specs` 直下の増減で偶発の赤が出る件）。

## 7. 要件ディスカッションへの議題（答えで作業が変わるもの）

1. **要件 3.5 と 6.1・6.6 の境界＝「切替の目印が下りる時点」。** 切替先の SHIORI の失敗（`Fault`）は `boot_with_kanade_stop` が `Ok` を返した数フレーム後に非同期で届く。3.5 の「切替先の起動系列が始まったあとは今日どおり終了へ」を字句どおりに取ると 6.6 の「切替先の `Fault` は元へ戻す」が成り立たない。候補: ⒜ 目印は「切替先の kanade が定常に達する」まで立て続ける（UI がそれを知る手段を設計で作る）、⒝ 切替先の失敗を「起動から N 秒以内の停止」で近似する、⒞ 6.6 を「同期で分かる失敗（解決・マウント・窓）だけ戻す。非同期の `Fault` は今日どおり告知して終了」に縮める（開発者裁定「1 体の失敗はアプリの失敗ではない」と離れる）。
2. **切替先の起動失敗の判定点。** `boot_ghost` は失敗を返さず LogSink の起動へ倒れる（今日の契約「起動の失敗は非致命」）。切替では ⒜ fallback を使わず失敗として戻す（`boot_ghost` に切替用の派生を作るか、戻り値で結線の成否を返す）か、⒝ fallback を許す（切替先が居ないまま窓だけ出る＝要件 6.1 に反する）か。要件 6.1 に「fallback へは倒れない」を明記するか。
3. **握手中の `Fault`（要件 2.8）と切替の情報。** 目印①（停止原因に値）を採ると `Fault` の通知に切替の情報が無い。⒜ 通知に相乗りの欄（③）で残す、⒝ 握手中の `Fault` は今日どおり告知して終了（2.8 を改める）、のどちらか。
4. **汎用の入口で頼んだ GET の応答が、再生中のトークに重なったとき**（要件 7.3 は「定常にある」だけ）。⒜ マウスと同じく置き換える、⒝ `warn!` して捨てる（今日の第 3 の出所の既定）、⒞ 再生が終わるまで待つ（待ち行列は要件 7.4 で「積まない」と決めており矛盾）。`steady::on_reply` の出所別の match は wildcard を置かない設計なので、どれかを明示する必要がある。
5. **要件 3.8 の 1 秒目標が実測で超えたときの扱い。** 同期で降ろす（案 A/C）か、別スレッドで降ろして次フレームで起こす（案 B）かは設計判断だが、後者は「降ろしている間に届く入力（メニュー・OS の閉鎖要求）」の扱いを要件に足す必要がある。今の段階で「超えたら B へ」と決めておくか。
6. **切替の相に `CloseRequest`（メニューの「終了」・OS の閉鎖要求）が届いたとき**（要件は沈黙）。候補: ⒜ 切替を中止して終了へ（利用者の終了の意思が勝つ）、⒝ 保留して切替後に終了、⒞ 無視して `warn!`。
7. **`OnGhostChanging` の Ref0（切替先の `sakura.name`）を目録の素性に足さない（要件 8.7）ので、単独の読み手が切替のたびに `descript.txt` を読む。** メニューの供給関数（メニューを出すたびに呼ばれる）でも読むか、`OnGhostChanging` を送る直前だけ読むか（性能は問題にならないが、読む回数の規則を要件に置くか）。
8. **要件 1.9 の二重要求の判定場所。** 目印の置き方で kanade（相で `warn!`）か UI（資源で `warn!`）かが決まる。どちらでも要件は満たすが、`ghost-install`・`network-update` が同じ入口を使うので、どこで弾くかを要件で明示しておくと後続の設計が揺れない。
9. **「解決できない名前」の腕の所在**（`ghost-change-name-resolution` が置き換える）。名前の突き合わせは目録（fs I/O）を読むので talk スレッドの受け口 `change_cue.rs` ではなく UI 側の取り出しで行うのが自然。並走 spec の brief は `change_cue.rs` の腕を置き換えると書いている——所在を「UI 側の取り出し（例 `frame/ghost_switch.rs` か `input_events/change.rs`）」と要件に書き直し、並走側へ申し送るか。
10. **要件 10.1 ⑷「窓ごとの状態が B のもので置き換わっている」の判定対象。** 決定論テストでは実窓を作れない（`WintfTaskPool`・実モニタ）。完了 `ghost-restart-unit` と同じく配線状態（説明書の経路・メニューの登記・中断の旗・停止通知の受け口）で判定する形を要件に明記するか。
11. **元へ戻す試みの起動経路（要件 6.2）。** 元のゴーストを起こし直すのも `boot_ghost` なので `on_boot_ok` が `LastGhost` を元のフォルダ名で書き直す（値は同じ）。「記憶を書かない」と読める文言を「同じ値で書き直す」に直すか、書かない経路を作るか（`GhostRoute::Argv` を流用すると `info!(last_used_skipped_argv)` が出て記録の語彙が嘘になる）。

## 8. 要件討議（2026-09-26）での §7 の振り分け

§7 の 11 件は要件討議で次のように振り分けた。要件側の文言は requirements.md に反映済み。

| §7 | 振り分け | 結果 |
|---|---|---|
| 1 目印が下りる時点 | 要件で確定（A） | 要件 3.5＝切替先の kanade が起動系列を終えて定常に入った時点。**UI がそれを知る手段は設計**（下の B-2） |
| 2 切替先の起動失敗の判定点 | 要件で確定（A） | 要件 6.1＝切替では `boot_ghost` の LogSink fallback へ倒れず失敗として扱う。派生関数か戻り値かは設計（B-5） |
| 3 握手中の `Fault` と切替の情報 | 設計（B-1） | 要件 2.8 は据え置き。目印の形は「`Fault` でも切替の情報が落ちない」ことを満たす案（③ か ②）から選ぶ |
| 4 汎用の入口の応答が再生中に重なる | 要件で確定（A） | 要件 7.3＝マウス系と同じく置き換える |
| 5 1 秒目標を超えたとき | 設計（B-3） | 同期で始め、実機の計測で超えたら別スレッド化（案 B）を設計討議へ |
| 6 切替の相の終了要求 | 要件で確定（A） | 要件 2.9＝切替を取りやめ、台詞の終わりで今日の終了経路へ |
| 7 `sakura.name` の読み手の回数 | 要件で確定（A） | 要件 8.7＝`OnGhostChanging` の Reference を組むときだけ。メニューは読まない |
| 8 二重要求の判定場所 | 設計（B-1 に従属） | 目印の置き場で決まる |
| 9 「解決できない名前」の腕の所在 | 要件で確定（A） | 要件 1.7＝UI 側の取り出し。`ghost-change-name-resolution` の brief へ申し送り済み |
| 10 要件 10.1 ⑷ の判定対象 | 要件で確定（A） | 配線の資源で判定 |
| 11 要件 6.2 の記憶の文言 | 要件で確定（A） | 「同じ値で書き直され結果として元のまま」 |

### 要件討議の裁定で変わった点（設計への注記）

- **議題 1（2026-09-26 開発者裁定「初回起動が最優先」）**: §2.3 の「`OnGhostChanged` の差し込み点は `first_boot` の判定より先」は逆になる。差し込みは **`first_boot` が偽のときの `OnBoot` の代わり**（`first_boot` 真なら今日どおり `OnFirstBoot`・`OnGhostChanged` 0 件）。設計は ukadoc の「204 なら続けて」の木（requirements.md の「起動と終了の根の木」）を「根の表＋共通の葉 `OnBoot`」の 1 か所で持つ形を検討する（範囲外の `OnGhostCalled`・`OnVanished` は行を足すだけで入る）。

- **議題 2（2026-09-26 開発者裁定「既定ゴースト固定で戻す・メッセージボックスは出さない」）**: §2.7・§2.9・§7-11 の「元のゴーストを起こし直す」は「既定ゴースト（`GhostRoute::Default`）を起こす」に読み替える。UI 側の迎え入れの状態は「直前に切替で起こした・失敗したら既定へ」だけで足り、呼び出し元の情報は `OnGhostChanged` の Reference 用にだけ残す。既定ゴーストの `OnBoot` に Ref6＝`halt`・Ref7＝切替先の名前を載せる口（`KanadeConfig` か派生関数）が §4.3 の器に加わる。

### 設計フェーズへ送る判断（B）

1. **B-1 切替の目印の形**（§4.1 の ①②③）。制約: `Fault` でも切替の情報が落ちない（要件 2.8）・二重要求を弾く場所が決まる（要件 1.9）・中止を UI に伝える線の要否（要件 5.2）。推奨は ③＋UI の小さな迎え入れの状態（§6）。
2. **B-2 切替先の kanade が定常に入ったことを UI が知る手段**（要件 3.5）。候補: 停止通知の型を「運行状態の通知」に広げる／`ResourceQuery` に運行状態を足す。時間で区切る近似は採らない（状態の持ち方で解く）。
3. **B-3 降ろす処理を同期にするか別スレッドにするか**（§4.2・要件 3.8）。同期で始め、実機で 1 秒を超えたら設計討議へ。
4. **B-4 直前のゴーストの情報を kanade B へ渡す器**（§4.3 の ⓐⓑⓒ）。`GhostBootOptions` に欄を足さない（要件 8.6）。
5. **B-5 `boot_ghost` の切替用の派生**（fallback へ倒れず成否を返す形）と、`GhostSession` の置き場（§2.2・要件 3.6＝World の NonSend 資源が第一候補）。
6. **B-6 `BasewareRoot` とアプリのプロファイルを World に置く器**（§2.6。メニューの目録・切替先のバルーンの解決に要る）。
7. **B-7 `LastUsed::record` に渡す `GhostRoute` の値**（§2.7。`Argv` 以外なら記憶が書かれる。値を足すか流用するか）。
8. **B-8 切替の相の置き方**（§4.4）と **汎用の入口の形**（§4.5）。`OnGhostChanging`／`OnGhostChanged` は専用の腕で送る（要件 7.5 への答えの候補）。
