# Brief: multi-ghost-runtime

> **0.1.0 の段**（2026-10-15 `/kiro-discovery`「版の段」）。開発者「ゴースト切り替えができること。これはできてる。複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」。roadmap の「予約」に名前だけ在った「多重ゴースト」を、土台の本 spec と、呼ぶ働きの `ghost-call` の 2 本に分けた（1 本では 20 タスクを確実に超えるため）。下の「推す形」と「議題」は、要件の段で覆してよい。

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
- **Adjacent**: `mcp-kanade-tools`・`mcp-reload`（kanade の `msg.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`）・`write-failure-notice`（`boot_resolve.rs` の終了の印）・`dpi-realign-remembered-chain`（`placement/`）・`extra-character-windows`（`switch_assets.rs`・スコープの数）。`main.rs` を触る spec（`baseware-root-list`・`user-data-root`・`write-failure-notice`）とは直列。本体の crate 分けの 6 本（2026-10-28 `/kiro-discovery`・`placement-crate-split` → … → `session-crate-split`）とは直列で、特に `session-module-gather`・`session-crate-split` とはファイルがほぼ全部重なる（`GhostSlot`・止まる流れ・印）。v0.1.0 の余力の枠で着地しなければ 6 本の後に取り、パスは roadmap の「本体の crate 分け」の新旧の対応表で読み替えて測り直す。

## Constraints

- 1 ファイル 1,000 行（`file_length_guard_test.rs` の例外表に触れない）。
- 決定論テスト網羅・ログ無し失敗経路の禁止・終了経路は正規実装（メッセージボックスを出さない）。
- 1 フレーム遅らせる解を取らない（状態の持ち方を変えて解く）。

## 議題（要件の段で決める）

1. 同じゴーストを 2 体同時に起こしてよいか（SSP は同じゴーストの二重起動をしない。推しは「しない」）。
2. 2 体目以降を起こす最小の入口を本 spec に置くか（推しは「前回の並びの復元」と、検査用の入口だけ。利用者向けの「呼ぶ」は `ghost-call`）。
3. メニューの「終了」と「すべて終了」の出し分け（今のメニューは Win32 標準。1 体のときは「終了」だけで `OnCloseAll` を起こす＝ukadoc どおり）。
4. 窓の重なりの順を、ゴーストをまたいでどう決めるか（ゴーストごとの鎖を独立に持つ、が推し）。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）
- 前提の変化: 10-15 の起票の後の初めての測定。版の段で **v0.1.0**（複数ゴーストの同時起動）。ソースは起票の時から変わっていない。
- 本文の主張は全部コードに在った。足りない分:
  - ゴーストの箱 `GhostSlot`（`crates/areka/src/ghost_session.rs`）を読む本番のファイルは 22。本文の挙げた外に `mcp/` の 5 つ（`resolve`・`get_status`・`get_property`・`get_expression_table`・`dump_surface` の支え）・`install/{desk,names}.rs`・`update/{desk,mod}.rs`・`readme/opener.rs`・`input_events/file_drop.rs`・`menu/shell_frame.rs`・`emo2_boot/{balloon_visibility_phase,frame/status_report,frame/switch}.rs`。
  - 窓の印と `GhostWindows` に触る本番のファイルは 43。`quit_app` の呼び手は 8 ファイル。止める理由 `CloseReason` は kanade の `msg.rs` に在る（`change.rs` ではない）。
- 上限の近く（触る見込み）: `main.rs` 956・`emo2_boot/consumer_ledger.rs` 943・kanade の `msg.rs` 930 と `schedule/mod.rs` 969・`emo2_boot/mod.rs` 911・`ghost_switch.rs` 902・`placement/mod.rs` 902・`ghost_session.rs` 873。
- 規模: 24〜30 タスク（割る仕事 3〜4・集まりと識別子 2・窓と重なりの順 4・結線 9 種の配り先 4〜5・知らせの送り主 2・止まり方と `OnCloseAll` 4・席での切替 2・印と記憶の並び 3・2 体目の入口と MCP 2・説明と実機 2）。
- 分割の案: 一度切り出した spec なので、20 を少し超えるだけなら切らない。要件で 25 を超えたら、前＝「席と区別」（集まり・識別子・窓・結線・知らせ・検査用の 2 体目の入口・`get_active_ghost_list`）、後＝「止まり方と記憶」（1 体の終了・`OnCloseAll`・席での切替・印と記憶の並び）に切る。`ghost-call` は後の方を待つ。
- 先に要るもの: 働きの上では無し。ファイルの上で次の着地の後: `baseware-root-list`・`user-data-root`・`write-failure-notice`（`main.rs`・`boot_resolve.rs`・`boot_config.rs`・`ghost_switch.rs`）、`mcp-kanade-tools`（kanade の `msg.rs`・`schedule/mod.rs` を先に割る・`emo2_boot/frame/switch.rs`）、`mcp-reload`（kanade の `change.rs`・`emo2_boot/mod.rs`・`ghost_switch.rs`・`shell_balloon_switch.rs`）。
- 同じウェーブに置けない相手（24 本の中）: 上の 5 本・`script-security-level` と `mcp-strict-errors`（kanade の `msg.rs`）・`ghost-call`・`dpi-realign-remembered-chain`（`placement/spawn.rs`・`persist.rs`）・`placement-measure-bake-once`（案 A なら `ghost_session.rs`・`placement/mod.rs`・`emo2_boot/mod.rs`）・`property-name-case-fold`（App の記憶の書き方を `persist/mod.rs` で変えるとき）。MCP の宛先を引く関数（`mcp/resolve.rs` の `active`）の形を変えると、`mcp-kanade-tools` が作る `sakurascript`・`raise_event` にも波及する。
  - 重なり 0: `app-icon`・`dot-sakura-specimen`・`balloon-webp-names`・`extent-element-offset`・`seriko-trigger-lottery-gaps`・`mcp-receiver-bugs`。＝v0.1.0 の中では、ほぼ単独で走る後ろの方の席。
- 優先度の区分: A（開発者「複数ゴーストの同時起動→…まだ出来ていない」）。
- 要件定義のモデル: **Fable**。当たる基準＝並行と順の保証を新しく設計する。プロセスに 1 本の知らせの受け口を、止まりの相が「届く順」と「ゴーストが 1 体であること」に頼って読んでいる（`emo2_boot/frame.rs` の `KanadeNoticeRx`・`run_ghost_quit_phase`）。App の記憶は「読んで重ねて書く」ので、N 体の書き手が同時に書くと片方が消えうる（`boot_resolve.rs` の印の関数の上の説明）。
- 見つけた穴・古くなった記述: 本文の `GhostSlot` の読み手の一覧が足りない（上の 22）。「知らせ」の項の `CloseReason` の置き場（`msg.rs`）。

## 申し送り（2026-10-17・`areka-P0-move-canon` の要件ディスカッションから）

- `move-canon` の `\![move]` は、基準に他のゴーストの名前（ukadoc: sakura 名か \0 名とスコープ ID）が書かれると、今はスコープ 0 に置き換えて動かし、置き換えたことを記録する（SSP 2.9.18 の実測で、知らない名前はスコープ 0 扱い）。複数のゴーストが同時に動くようになったら、名前を走っているゴーストの窓へ解決することを本 spec の要件の段で引き取る。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 版の段で「v0.1.0 の余力の枠」（v0.1.0 の残りが人や外の待ちで止まったときだけ取る）。10-16 の後に `user-data-root`・`write-failure-notice`・`move-canon`・`mcp-kanade-tools`・`startup-options-core`・`mcp-reload` が、この spec の触るファイルを先に触って着地した。
  - `crates/areka/src/main.rs` は 1,000 行ちょうど → **965 行**（`startup-options-core` が起動オプションを `cli/` へ出した）。最初のタスクで中身を外へ出す必要は弱まったが、余りは 35 行。
  - kanade の `msg.rs` は 930 → **539**、`schedule/mod.rs` は 969 → **664**、`emo2_boot/consumer_ledger.rs` は 943 → **472**（`mcp-kanade-tools`・`mcp-reload` が割った）。
  - いま上限に近いのは `emo2_boot/ghost_switch.rs` 921・`placement/mod.rs` 915・`ghost_session.rs` 910 行。ほかに `emo2_boot/mod.rs` 787・`placement/spawn.rs` 776・`boot_resolve.rs` 714。
- ゴーストの箱 `GhostSlot` を読む本番のファイルは 22 → **28**。増えたのは `emo2_boot/reload.rs`・`emo2_boot/shell_carry.rs`（`mcp-reload`＝読み直しでシェルの状態を持ち越す道も、席ごとに分ける対象になった）・`mcp/sakurascript.rs`（`mcp-kanade-tools`）・`menu/reset_pos_frame.rs`（`move-canon`）・`roots/edit.rs`・`ghost_session_runtime.rs`。`quit_app` を呼ぶファイルは 8 → **10**（`smoke_exit.rs`・`write_failure.rs` ほか）。窓の印と `GhostWindows` に触る本番のファイルは 41。止まりの知らせの受け口は今も 1 本（`emo2_boot/frame.rs` 173 行の `KanadeNoticeRx`）。
- 触るファイル: 本文と棚卸㉕のとおり＋上の新しい読み手。
- 規模: 24〜30 タスク（読み手が 6 増えた分を含めて上限のまま）。
- 先に要るもの: 働きの上では無し。ファイルの上では `dpi-realign-remembered-chain`（`placement/`）を先に着地させる（roadmap の決めのまま）。
- v0.1.0 との重なり（ほぼ全部）: `boot_resolve.rs`・`boot_config.rs`（`readonly-mode`・`ghost-inner-balloon`・`startup-install-option`・`default-balloon-selfmade`）・`ghost_session.rs`（`placement-measure-bake-once`・`startup-install-option`・`dpi-realign-remembered-chain`）・kanade の `msg.rs`（`mcp-strict-errors`・`script-security-level`・`mouse-click-wheel-events`）・`input_events/mod.rs`（`mouse-click-wheel-events`・`shell-mouse-cursor`）・`menu/mod.rs`（`popup-menu-shell-look`）・`main.rs`（`offline-surface-dump`・`readonly-mode`・`startup-install-option`）・sylphya の `persist/` と `mcp/resolve.rs`（`property-name-case-fold`・`readonly-mode`）。＝余力の枠で取るときも、ほぼ単独で走る。
- 1 本だけの席: 取らない見込み（依存の追加なし・emo-text に触らない）。人の待ち: 2 体を並べた実機の確かめ（AI が回せる）。静かな机・前後の全体テストの比べは要らない（終わり方を作り直すので、赤が出たテストに絞って回し直す）。
- 優先度の区分: A（開発者「複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」）。
- 要件定義のモデル: **Fable**（厳しく見直しても当たる）。当たる基準＝並行と順の保証を新しく設計する。止まりの知らせは 1 本の受け口に届く順と「ゴーストが 1 体」に頼って読まれ（`emo2_boot/frame.rs` の `KanadeNoticeRx`・`run_ghost_quit_phase`）、アプリの範囲の記憶は「読んで重ねて書く」ので複数の書き手が同時に書くと片方が消えうる（`boot_resolve.rs` の印の関数の説明）。今の作りに乗るだけでは済まず、届く順と書き手を 1 つにする決まりを新しく作る。
- 分割の案: 棚卸㉕のまま（要件で 25 を超えたら、前＝「席と区別」、後＝「止まり方と記憶」。`ghost-call` は後を待つ）。読み直しの道（`reload.rs`・`shell_carry.rs`）を席ごとにするのは前に入れる。
- 見つけた穴・古くなった記述:
  - 本文の「上限の近いファイル」の行数（`main.rs` 956・`consumer_ledger.rs` 943・`msg.rs` 930 ほか）は古い（上の今の数）。`consumer_ledger.rs` はこの spec の重さに効かなくなった。
  - roadmap の「`main.rs` は 1,000 行ちょうど」の記述（直列の列の 2 か所）も古い。
  - この brief の最後の申し送りの節の 4 行だけ、行末の形がほかの行と違う（その場で直せる）。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: C10 の 8 本のうち、この spec の触るファイルを先に触って着地したもの＝`offline-surface-dump`（`main.rs` 965→978 行・余りは 22 行）・`window-link-directives`（`placement/mod.rs` 915→939 行・`emo2_boot/consumer_ledger.rs` 472→496 行）・`script-security-level`（kanade の `msg.rs` 539→544 行・`schedule/` の多くのファイル）・`ghost-inner-balloon`（`boot_config.rs` 622・`install/desk.rs` 560・`update/desk.rs` 706）。上限に近いのは `main.rs` 978・`placement/mod.rs` 939・`emo2_boot/ghost_switch.rs` 921・`ghost_session.rs` 917。ゴーストの箱 `GhostSlot` を読む本番のファイルは 28 のまま、`quit_app` を呼ぶファイルは 10 のまま、止まりの知らせの受け口は今も 1 本（`emo2_boot/frame.rs` 173 行目の `KanadeNoticeRx`）。kanade は今も `OnCloseAll` を送らない（`crates/areka-kanade/src/schedule/close.rs` の注記）。
- **触るファイル**: 本文と棚卸㉕・㉘のとおり。
- **規模**: 24〜30 タスクのまま（上限 20 を超える＝下の分割の案）。
- **先に要るもの**: 働きの上では無し。ファイルの上では `dpi-realign-remembered-chain`（`placement/`）を先に着地させる（決めのまま）。v0.1.0 の未完了のほぼ全部と重なる: `readonly-mode`・`startup-install-option`・`default-balloon-selfmade`（`boot_resolve.rs`・`boot_config.rs`・`main.rs`）・`mcp-strict-errors`（`mcp/sakurascript.rs`・案 ⑵ なら kanade の `msg.rs`）・`mouse-click-wheel-events`（`msg.rs`・`input_events/mod.rs`）・`window-work-area-clamp`（`placement/persist.rs`）・`balloon-communicatebox`・`popup-menu-shell-look`（`menu/mod.rs`）・`placement-measure-bake-once`・`ghost-call`＝v0.1.0 の残りが人や外の待ちで止まったときに、ほぼ単独で走る（変わらず）。1 本だけの席は取らない。人の待ち・静かな机・全体テストの比べは要らない。
- **優先度の区分**: A のまま（開発者「複数ゴーストの同時起動→アプリ設計として可能だが、まだ出来ていない」）。
- **要件定義のモデル**: 厳しく当て直して、**後半だけ Fable・前半は Opus**。当たる基準＝順の保証を新しく設計する。全員を止める流れ（`OnCloseAll` → 各ゴーストの `OnClose` → 全員の終わりの握手を待ってアプリを終える・2 体がほぼ同時に「終了」したときにどちらが最後かを決める・OS のシャットダウンの持ち時間の中で N 体を止める）は、今の 1 体の握手（`emo2_boot/frame.rs` の `run_ghost_quit_phase`）では決まらない。一方、知らせにゴーストの区別を付けて 1 本の受け口で振り分けるのは、送り手ごとの順を守る今の受け口の上に乗るだけで、アプリの範囲の記憶の書き手を UI のスレッド 1 つに集めるのも今の作りに乗る＝どちらも先例で足りる。
- **分割の案**: 棚卸㉕の切り方を保つ（一度切り出した spec だが、20 を少し超えるのでなく 24〜30 なので要件の段で切る）。前＝「席と区別」（集まりと番号・窓と重なりの順・結線 9 種の配り先・知らせの送り主・読み直しの道〔`reload.rs`・`shell_carry.rs`〕・検査用の 2 体目の入口・`get_active_ghost_list`・上限の近いファイルを割る仕事）＝12〜15・Opus。後＝「止まり方と記憶」（1 体の終了・`OnCloseAll`・席での切り替え・起動中の印と記憶の並び）＝10〜14・Fable。`ghost-call` は後を待つ（`OnOtherGhostClosed` が「1 体だけ止まり、ほかは動き続ける」に乗るため）。前だけが着地した間は、どのゴーストの「終了」もアプリを終える今の振る舞いのまま（2 体目は検査用の入口からだけ起きる）。
- **見つけた穴・古くなった記述**: 棚卸㉘の「申し送りの節の 4 行だけ行末の形が違う」は今は当たらない（全行が同じ形）。棚卸㉘の行数（`main.rs` 965・`placement/mod.rs` 915・`ghost_session.rs` 910）は上のとおり増えた。
