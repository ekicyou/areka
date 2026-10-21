# Brief: areka-P0-resident-ghost-rotation

> **v0.4.0 の段**（2026-10-21 `/kiro-discovery` で起票）。開発者「裏でSHIORIだけ常駐維持しておいて、さくらスクリプト投げてきたゴーストを取っ替え引っ替え切り替えるモード。裏に隠れているSHIORIには時刻関係以外のアクティブイベントは投げない。複数SHIORIが同時にさくらスクリプトを要求してきた場合、現在表示中のゴーストの再生が終わるまで次の再生を遅延させる。」・時期は「これは、土台ができた後でよい。夢仕様とまではしないが、少なくともdola、wintf関係がリリースされた後の次のリリース候補」。下の「推す形」と「議題」は、要件の段で覆してよい。

## Problem

- 利用者は何体ものゴーストを持っていても、画面に出せるのは 1 体だけ。ほかのゴーストは降ろしてあるので、時報や雑談の時刻が来ても何も言わない。
- 複数のゴーストを同時に並べる形（`multi-ghost-runtime`）は、体数だけ窓が増えて画面を取る。1 つの場所に 1 体だけ出し、話したいゴーストが順番に出てくる使い方が無い。
- SSP にも、この使い方は無い（areka 独自のモード）。

## Current State（main `213a3747`・起票時の調べ）

### 時刻のイベント

- areka が SHIORI へ送る時刻のイベントは `OnSecondChange` だけ。`OnMinuteChange` は作らず、`OnHour`・`OnTalk` はゴースト（emo2・pasta）が `OnSecondChange` の中で自分で作るので送らない（`crates/areka-kanade/src/schedule/events.rs:104`）。
- 時計はゴーストごと。起こすたびに `boot_with_origin` が時計の actor を作り（`crates/areka-ghost/src/runtime.rs:741-747`・`ticker.rs:165-242`）、1 秒ごとに `KanadeMsg::Tick` を送る。`TickerMode::Disabled` で止められる。
- kanade の `on_tick`（`schedule/steady.rs:698-738`）: トークが無いと `OnSecondChange` を GET（Reference3＝1）、トークの最中は NOTIFY（Reference3＝0）で送り、返事は捨てる。ukadoc の `OnSecondChange`「トーク再生不能な時は、Reference3が0になった上でNOTIFYでイベント通知される」のとおり。
- 返事の台本は `on_reply`（`steady.rs:753-862`）で `Action::StartTalk` になり、dispatcher → トークごとの再生 → sinks の並びへ流れる。トークの最中に届いた台本は「溜めない・割り込まない」（`steady.rs:844-852`）＝kanade は台本を溜めない作り。

### 窓なしで動く部分と、窓の要る部分

- `areka_ghost::boot_with_origin`（`runtime.rs:568-777`）は窓を作らない: 置き場の解決・sylphya・SHIORI（ゴーストごとに 32bit の中継プロセス 1 つ）・kanade・dispatcher・時計。出口は `sinks: Vec<Box<dyn BootCueSink>>` だけ。
- 窓と UI スレッドの要る部分は `wire_emo2_boot`（`crates/areka/src/emo2_boot/mod.rs:402-779`）が足す: シェルとバルーンの絵・EmoPresenter・文字の actor・seriko・UI への 11 本の sinks・World の 1 つきりの置き場（`Emo2Wiring` ほか）。
- 窓なしの前例が在る: 結線に失敗したときの `[LogSink, DiscardSink]`（`crates/areka/src/boot_config.rs:537-554`・`ghost_session.rs:683-717`）。ただし LogSink は台本を「再生したこと」にしてすぐ終える＝裏のゴーストには、台本を受け取って持っておく出口が要る。

### 切り替えと表示

- 今の切り替え（`emo2_boot/ghost_switch.rs:663-731`）は、前のゴーストを `GhostSession::shutdown`（`OnClose` → SHIORI を降ろす）で止め、窓を閉じ、次のゴーストを起こし直す。SHIORI を残したまま表示だけを別のゴーストへ渡す道は無い。
- 近い前例は同じゴーストのシェルとバルーンの着替え（`shell_balloon_switch.rs`・`switch_assets.rs:127-245`）。SHIORI と kanade を残し、トークの切れ目を `KanadeMsg::AwaitTalkGap` で待ってから seriko の中身を替える。
- ゴーストの箱 `GhostSlot(pub Option<GhostSession>)`（`ghost_session.rs:401`）はプロセスに 1 つ。複数を持つ形は `multi-ghost-runtime` が作る（その brief の Current State に、1 つきりの置き場と止まり方の一覧が在る）。

### 再生の終わりと、台本を流し込む口

- 終わりの知らせ: トークの再生 → dispatcher の `on_done` → `KanadeMsg::TalkDone`（`crates/areka-ghost/src/dispatcher.rs:363-377`）→ kanade が `Steady{talk:None}` へ。外からは `KanadeMsg::AwaitTalkGap`（`crates/areka-kanade/src/msg.rs:246`・`schedule/talk_gap.rs`・答えは `TalkGap::Reached`）が「トークも閉じる・替える予定も無い」ときに 1 度だけ答える＝「表示中のゴーストが話し終えた」の知らせにそのまま使える。
- 台本を流し込む口: MCP の `sakurascript` が使う `KanadeMsg::SstpSend`（`crates/areka/src/mcp/sakurascript.rs:33-65`・kanade 側は `schedule/sstp.rs:28-36`）。受けたら今のトークを置き換える（待たない）。
- イベントを止める旗は無い。マウス・選択肢・アンカーは窓から来るので、窓の無いゴーストにはもともと届かない。`GhostSlot` から宛先を引く送り手（`install/desk.rs:455`・`update/desk.rs:239,376`・`update/worker.rs:120`・`input_events/file_drop.rs:141`・`menu/reset_pos_frame.rs:79`・`emo2_boot/frame/switch.rs:541`）は、表示中のゴーストだけを宛先にし続ける必要がある。

## Desired Outcome

1. このモードでは、選んだ何体ものゴーストの SHIORI が裏で動き続け、画面には 1 体だけが出る。
2. 裏のゴーストへ送るイベントは時刻のもの（今は `OnSecondChange`）だけ。マウス・メニュー・投げ込み・インストールと更新・MCP の宛先は、表示中のゴーストだけ。
3. 裏のゴーストが台本を返すと、表示がそのゴースト（そのシェルとバルーン）に替わって再生する。SHIORI は降ろさず、`OnClose` も `OnBoot` も起こさない。
4. 複数のゴーストが台本を返したら、表示中のゴーストの再生が終わるまで次を待たせ、届いた順に出す。表示中のゴースト自身のトークも同じ順番に並ぶ。
5. 話し終えた後は、次に話すゴーストが来るまで、最後に話したゴーストが出たまま。
6. このモードを使わないときは、今と同じに動く（決定論テストを 1 本も落とさない）。

## Approach

**推す形**（要件の段で覆してよい）:

- `multi-ghost-runtime` が作るゴーストの席の集まりに乗る。席を「表示の席（窓あり）1 つ」と「裏の席（窓なし）」に分け、裏の席の出口を「台本を受け取って持っておく出口」にする。
- 待ち行列は UI スレッドに 1 つ置き、（ゴースト・台本）を届いた順に並べる。kanade は溜めない作りのまま（行列は kanade の外）。
- 表示中のゴーストの `AwaitTalkGap` が `Reached` を答えたら先頭を取り出し、表示をそのゴーストへ渡して再生する。表示の受け渡しは、同じゴーストの着替え（`shell_balloon_switch.rs`）の「裏で絵を組み、切れ目で替える」形を、別のゴーストの席へ広げる。
- 裏の席へは時計の `Tick` だけを通す。台本を持っている間は、そのゴーストを「再生中」として扱う＝`OnSecondChange` は Reference3＝0 の NOTIFY（ukadoc の「トーク再生不能な時」）になり、1 体が溜めるのは 1 本まで（議題 1）。

**採らない方向**:

- 切り替えのたびに SHIORI を降ろして載せ直す（今の切り替えの流用）。降ろしている間のゴーストは時刻を知らず、話しかけてこられない＝このモードの目的を外す。
- ゴーストごとに窓を開いたまま隠す。体数だけ窓と絵の置き場が残り、隠した窓の重なりの順と DPI の追従を全部の体で持つことになる。
- `multi-ghost-runtime` を待たずに、横へ窓なしの溜まり場を足す。表示の受け渡しで、`multi-ghost-runtime` と同じ結線の直しをもう一度やることになる（開発者「土台ができた後でよい」）。

## Scope

- **In**: モードの入口（議題 3）／裏の席（窓なし・台本を持っておく出口）／裏の席へのイベントの絞り（時刻だけ）／UI スレッドの待ち行列と順番／再生の終わりを待って表示を別のゴーストの席へ渡す流れ／宛先を引く送り手を「表示中のゴースト」に揃える／モード中の終了（全員の `OnClose` とアプリの終わり）／`dist/README.txt` と `doc/COMPAT_ARCHITECTURE.md` の行。
- **Out**: 複数のゴーストを同時に並べて出すこと・席の集まり・1 体の終了と `OnCloseAll`（`multi-ghost-runtime`）／他のゴーストを呼ぶ働き（`ghost-call`）／ゴースト同士の掛け合い（`OnOtherGhostTalk` ほか・v0.3.0 の覚え書き）／入れ替わりの見た目の効果（フェードなど）／設定画面（windows-reactor 待ち）。

## Boundary Candidates

- 裏の席: 窓なしの起動と、台本を受け取って持っておく出口（`areka-ghost` の sinks・`boot_config.rs`）。
- イベントの絞り: 裏の席へ時刻だけを通す所（kanade の `schedule/`・時計）。
- 待ち行列と順番: UI スレッドの行列と、`AwaitTalkGap` で先頭を出す所（`emo2_boot/`）。
- 表示の受け渡し: 表示の席を別のゴーストへ渡す流れ（`shell_balloon_switch.rs`・`switch_assets.rs`・`placement/`）。
- 宛先: 表示中のゴーストを引く 1 か所と、その送り手（`mcp/resolve.rs` ほか）。

## Out of Boundary

- 席の集まりと識別子・窓と結線のゴーストの区別・止まり方の分け方（`multi-ghost-runtime` の持ち物）。
- 呼ぶ・呼ばれる・周りへのイベント（`ghost-call`）。

## Upstream / Downstream

- **Upstream**: `multi-ghost-runtime`（席の集まり・知らせの送り主・`get_active_ghost_list`）。v0.2.0 の wintf・dola の基幹の整備（開発者「dola、wintf関係がリリースされた後」）。
- **Downstream**: 入れ替わりの見た目の効果・設定画面からのモードの切り替え（どちらも起票はまだ）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `multi-ghost-runtime`・`ghost-call`（席の集まりと止まり方）・`readonly-mode`・`startup-options-core` の後続（起動オプションの読み方の前例）・`shell-hover-animation`（同じ版・`input_events/`）。

## Constraints

- 1 ファイル 1,000 行（`file_length_guard_test.rs` の例外表に触れない）。
- 決定論テスト網羅・ログ無し失敗経路の禁止・終了経路は正規実装。
- 1 フレーム遅らせる解を取らない（状態の持ち方を変えて解く）。
- 時刻は正確に扱う（裏の席の時計も表示の席と同じ刻みで動かす）。
- 裏の体数だけ 32bit の中継プロセスが立つ（ゴーストごとに 1 つ）。

## 議題（要件の段で決める）

1. 台本を持っている間の、そのゴーストへの `OnSecondChange`（推し: 「再生中」として Reference3＝0 の NOTIFY・1 体 1 本まで。代わり: いつも GET で返事を全部溜める＝行列が伸び、古い台本が遅れて出る）。起票の問いには答えが無かった。
2. 裏に置くゴースト（推し: 入っているゴースト全員。代わり: 利用者が選んだ一覧＝選ぶ所の分だけ大きくなる）。起票の問いには答えが無かった。
3. モードの入口（推し: 起動オプションとメニューの切り替え。名前は `startup-options-core` の決まりに合わせる）。
4. 起動のイベント（推し: 読み込みと `OnBoot` は全員へ送り、`OnBoot` の台本も同じ順番に並ぶ）と、終了（推し: モードの終了は全員へ `OnClose`）。
5. 表示中のゴーストの扱い（推し: 普通のモードと同じに全部のイベントを受け、自分のトークも行列に並ぶ）。
6. ゴーストが `\![change,ghost]` を出したとき、このモードでは何をするか（推し: 表示の席をそのゴーストへ渡すだけで、SHIORI は降ろさない）。
