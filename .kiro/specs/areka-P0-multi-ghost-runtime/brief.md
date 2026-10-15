# Brief: areka-P0-multi-ghost-runtime

> **0.1.0 の段**（2026-10-15 `/kiro-discovery`「版の段」）。開発者「ゴースト切り替えができること。これはできてる。複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」。roadmap の「予約」に名前だけ在った「多重ゴースト」を、土台の本 spec と、呼ぶ働きの `areka-P0-ghost-call` の 2 本に分けた（1 本では 20 タスクを確実に超えるため）。下の「推す形」と「議題」は、要件の段で覆してよい。

## Problem

- areka は、同時に 1 体のゴーストしか動かせない。SSP の利用者は複数のゴーストを並べて使うことが多く、0.1.0 の目標（よく使われるゴーストが SSP と同じように動く）に届かない。
- ゴーストの部品（kanade・SHIORI・プロパティ・seriko）は、もうゴーストごとに作れる。止めているのは areka 本体の側の「1 つだけ」の置き場と、「止まったらアプリが終わる」流れ。

## Current State（main `46123b00`・起票時の調べ）

### ゴーストごとに作れる物（そのまま使える）

- `areka_ghost::boot*`（`crates/areka-ghost/src/runtime.rs`）が、起こすたびに kanade の actor・dispatcher・sylphya の actor（`spawn_ghost_sylphya`）・SHIORI の actor を作る。SHIORI は `ShioriWiring::Helper` → `shiori_host32_host::spawn` で、ゴーストごとに 32bit の中継プロセスが 1 つ。
- `wire_emo2_boot`（`crates/areka/src/emo2_boot/mod.rs`）が、起こすたびに seriko の actor と刻みを作る。
- `GhostBootInputsSource` は、どの `ConfigInputs` からでも起動の入力を作れる。

### 「1 つだけ」の置き場（本 spec が直す）

- **ゴーストの箱**: `ghost_session::GhostSlot(pub Option<GhostSession>)`（`crates/areka/src/ghost_session.rs`・World の NonSend・プロセスに 1 つ）。`boot_first_ghost` が入れ、`ghost_switch::boot_into`／`take_down` が入れ替え、`main::after_run`・`session_end::end_session_from` が取り出して止める。
- **起こすたびに差し替わる結線**（World の NonSend／Resource・ゴーストの区別なし）: `Emo2Wiring`（`emo2_boot/frame/wiring.rs`）・`MouseWiring`（`input_events/mod.rs`）・`BalloonWiring`／`ChoiceSelectionInbox`（`input_events/balloon.rs`）・`MenuWiring`（`menu/mod.rs`）・`ChangeRx`・`SwitchInFlight`（`ghost_switch.rs`）・`SwitchRx`／`SkinSwitchInFlight`（`shell_balloon_switch.rs`）・`PersistWiring`（`placement/persist.rs`）・readme と user_break の結線。
- **窓**: `GhostWindows`（`placement/spawn.rs`）は `BTreeMap<スコープ, ScopeWindows>` の Resource 1 つ。`CharWindowMarker{scope}`・`BalloonWindowMarker{scope}`・`GhostWindowMarker` はゴーストを持たない。重なりの順の `ZOrderChainPlan` ほか（`app_exit::close_windows_for_restart` の一覧）も 1 つずつ。
- **知らせ**: `KanadeNoticeRx` はプロセスに 1 本の mpsc。`KanadeNotice`／`KanadeStopped`（`crates/areka-kanade/src/change.rs`・`msg.rs`）は送り主のゴーストを持たない。`run_ghost_quit_phase`（`emo2_boot/frame.rs`）は FIFO の順とゴーストが 1 体であることに頼る。
- **起動の文脈**: `BootContext.current`（`boot_config.rs`）は 1 体。メニューのチェック（`menu/ghost_frame.rs`）もこれを見る。
- **記憶**: `PersistKey::LastGhost`（`areka.last.ghost`）と `PersistKey::LastRunning`（`areka.last.running`＝起動中の印）は App の範囲の文字列 1 つ（`crates/areka-sylphya/src/persist/mod.rs`）。`boot_resolve.rs` のコメントは「App の範囲を書くのは動いているランタイムが 0 のときだけ（読んで直して書くため）」と書く＝N 体が同時に書くと、そのコメント自身の競合の心配が当たる。
- **MCP**: `mcp/resolve.rs::active(world) -> Option<ActiveGhost>` が `GhostSlot` を読む＝`get_active_ghost_list` は 0 行か 1 行。

### 止まり方はどれもアプリの終了へ流れる

- `app_exit::quit_app` が全部の `GhostWindowMarker` の窓を消して `AppExit::request_exit()`。
- `emo2_boot/frame.rs` の `run_ghost_quit_phase` → `quit_as_today`、`ghost_switch.rs` の `on_ghost_stopped`（切り替えでない枝はどれも `quit_app`）・`fatal`、`app_exit::on_ghost_os_close`（Alt+F4）、`input_events/mod.rs` の Esc、`main.rs` の煙の確かめの時計・`after_run`、`session_end.rs`。
- kanade に `OnCloseAll` は無く、`CloseReason` に「このゴーストだけ」は無い。
- 完了 `app-lifetime-separation` の要件は「複数ゴーストの同時起動…『1 体だけ終了』『すべて終了』（OnCloseAll）」を範囲外にした。完了 `baseware-root-layout` の要件は「同時起動は 1 体」。

### 上限の近いファイル（触る見込み・1,000 行が上限）

`crates/areka/src/main.rs` 956・`emo2_boot/consumer_ledger.rs` 943（`ghost-call` が行を足す）・`crates/areka-kanade/src/msg.rs` 930・`emo2_boot/mod.rs` 911・`emo2_boot/ghost_switch.rs` 902・`placement/mod.rs` 902・`ghost_session.rs` 873。ほかに `placement/spawn.rs` 775・`placement/persist.rs` 642・`boot_resolve.rs` 609・`emo2_boot/frame.rs` 568・`boot_config.rs` 503・`app_exit.rs` 291・`session_end.rs` 185・`crates/areka-kanade/src/change.rs` 175・`mcp/resolve.rs` 105。

### ukadoc（正典）

- `OnCloseAll`: 「すべて終了」の指示のほか、**唯一起動中のゴーストで「終了」が指示されたとき**と、OS のシャットダウンで終わるときにも起きる。204 なら続けて `OnClose`。Reference0＝`user`／`system`、Reference1・2＝スコープ番号（SSP のみ）。
- `OnClose`: 終了の指示で起きる（`OnGhostChanging`・`OnCloseAll` が 204 のときも続けて起きる）。
- `OnOtherGhostClosed`: 複数起動中に他のゴーストが終了したとき（Reference0・1・2・7）＝周りへの知らせは `ghost-call` の持ち物。

## Desired Outcome

1. 2 体以上のゴーストが同時に動き、それぞれが自分の窓・バルーン・メニュー・マウスの知らせ・台本の再生を持つ。片方の操作がもう片方へ漏れない。
2. 1 体の「終了」はそのゴーストだけを止め（`OnClose`）、ほかは動き続ける。最後の 1 体の「終了」と「すべて終了」と OS のシャットダウンは、ukadoc どおり `OnCloseAll` → `OnClose` で全員を止めてアプリを終える。
3. 1 体の切り替え（今ある `\![change,ghost]`・メニュー）は、そのゴーストの席だけを入れ替える。
4. 起動中の印と「最後に使ったもの」の記憶が、動いていたゴーストの並びを持つ。きれいに終わらなかったときの知らせ（`OnBoot` の Reference6＝`halt`・Reference7）は、落ちたゴーストごとに正しく出る。
5. 1 体しか動かさない今の使い方は、今と同じに動く（今の決定論テストを 1 本も落とさない。特に終了の握手 → `ghost_quit` の経路）。
6. MCP の `get_active_ghost_list` が、動いている全員を返す。

## Approach

**推す形**（要件の段で覆してよい）:

- `GhostSlot` を「ゴーストの識別子 → `GhostSession`」の集まりにする。識別子は起動ごとに振る番号（同じゴーストを 2 度起こさないかは議題 1）。
- 窓の印（`CharWindowMarker` ほか）・`GhostWindows`・結線の NonSend・`KanadeNotice` に識別子を持たせる。結線は「ゴーストごとの部品」として持ち、World の単体の置き場をやめる（ECS の部品の形にするか、識別子つきの表にするかは設計の段）。
- 止まる流れを「このゴーストが止まった」と「アプリを終える」に分け、`quit_app` を呼ぶのは「最後の 1 体が止まった」「すべて終了」「OS の終了」「煙の確かめ」だけにする。
- App の範囲の記憶を書く所を 1 か所（UI スレッド）に集め、各ゴーストの sylphya は自分の範囲だけを書く。
- **最初のタスク**: 上限の近いファイル（上の 7 つのうち触るもの）を、中のテストを兄弟のファイルへ出す・役目で割るなどして、足せる形にする。

**採らない方向**:

- ゴーストごとにプロセスを分ける（SSP の作りと違い、今の 1 プロセスの作り・WUC の窓の扱いを捨てることになる）。
- ゴーストごとに wintf の World を分ける（窓・重なりの順・DPI の追従は 1 つの World で揃えている）。

## Scope

- **In**: 複数の `GhostSession` の持ち方／窓・結線・知らせのゴーストの区別／1 体の終了とすべて終了（`OnCloseAll`）／1 体の席での切り替え／起動中の印と記憶の並び／2 体目以降を起こす最小の入口（議題 2）／`get_active_ghost_list` の全員／`dist/README.txt` と `doc/COMPAT_ARCHITECTURE.md` §8 の行。
- **Out**: メニューの「他のゴーストを呼ぶ」・`\![call,ghost]`・呼ぶときと周りへの知らせのイベント（`ghost-call`）／ゴースト同士の掛け合い（`OnOtherGhostTalk`・`\![set,otherghosttalk]`・`\![raiseother]`＝0.3.0 の覚え書き）／消滅（`OnOtherGhostVanished`＝予約）／3 人目からのキャラクター窓（`extra-character-windows`）／タスクトレイ（windows-reactor 待ち）。

## Boundary Candidates

- ゴーストの集まりと識別子（`ghost_session.rs`）。
- 窓と重なりの順のゴーストの区別（`placement/`）。
- 入力・メニュー・バルーンの結線の配り先（`input_events/`・`menu/`）。
- 止まる流れとアプリの寿命（`app_exit.rs`・`session_end.rs`・`emo2_boot/frame.rs`・`ghost_switch.rs`・kanade の `change.rs`／`msg.rs`）。
- 記憶と起動中の印（`boot_resolve.rs`・sylphya の `persist/`）。

## Out of Boundary

- 呼ぶ働きと、呼ぶ・呼ばれる・周りへのイベント（`ghost-call`）。
- 他のゴーストへ話しかける・聞く働き全般。
- 設定画面（windows-reactor 待ち）。

## Upstream / Downstream

- **Upstream**: 完了 `app-lifetime-separation`（アプリの寿命を窓の数から切り離した）・完了 `ghost-shell-balloon-switch`（プロセス内の切り替え）・`baseware-root-list`・`user-data-root`（0.0.3・`main.rs`・`boot_resolve.rs`・`boot_config.rs` を先に触る＝着地を取り込んでから）。
- **Downstream**: `ghost-call`（本 spec の集まりと止まる流れの上に乗る）・`property-catalog-lists`（`activeghostlist`）・`window-snap` の積み重なり（夢）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `mcp-kanade-tools`・`mcp-reload`（kanade の `msg.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`）・`write-failure-notice`（`boot_resolve.rs` の終了の印）・`dpi-realign-remembered-chain`（`placement/`）・`extra-character-windows`（`switch_assets.rs`・スコープの数）。`main.rs` を触る spec（`baseware-root-list`・`user-data-root`・`write-failure-notice`）とは直列。

## Constraints

- 1 ファイル 1,000 行（`file_length_guard_test.rs` の例外表に触れない）。
- 決定論テスト網羅・ログ無し失敗経路の禁止・終了経路は正規実装（メッセージボックスを出さない）。
- 1 フレーム遅らせる解を取らない（状態の持ち方を変えて解く）。

## 議題（要件の段で決める）

1. 同じゴーストを 2 体同時に起こしてよいか（SSP は同じゴーストの二重起動をしない。推しは「しない」）。
2. 2 体目以降を起こす最小の入口を本 spec に置くか（推しは「前回の並びの復元」と、検査用の入口だけ。利用者向けの「呼ぶ」は `ghost-call`）。
3. メニューの「終了」と「すべて終了」の出し分け（今のメニューは Win32 標準。1 体のときは「終了」だけで `OnCloseAll` を起こす＝ukadoc どおり）。
4. 窓の重なりの順を、ゴーストをまたいでどう決めるか（ゴーストごとの鎖を独立に持つ、が推し）。
