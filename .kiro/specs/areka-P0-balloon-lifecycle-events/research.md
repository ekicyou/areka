# ギャップ分析: areka-P0-balloon-lifecycle-events

> 実施: 2026-10-05（要件の生成の直後・ワークツリー `claude/areka-p0-balloon-lifecycle-d24784`・main `44fc0a61` の上）。
> 入力: `requirements.md`（確定）・`brief.md`（2026-10-05 の棚卸㉒まで）・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md` の C4 の行）。
> 引用は「何の定義行か」と行番号の組で書く（行番号はこの時点のもの）。

---

## 1. 要約

- **`OnBalloonBreak` と `OnBalloonClose` は、`msg.rs`・`actor.rs`・`steady.rs` に触らずに kanade の中だけで送れる。** 中断の合図 `Input::UserBreak{scope}` は「再生中のトークが無い」ときも kanade まで届いており（`schedule/user_break.rs` の `on_user_break` の「再生中のトークが無い」腕）、トークの完了は `schedule/mod.rs` の `on_talk_done` が一手に受ける。足りないのは ⑴ 中断の scope を控える欄 ⑵ 「直前に終えたトークの台本」を控える欄 ⑶ 定常へ戻った直後に GET を 1 本積む腕、の 3 つ。
- **`OnBalloonTimeout` は表示の側（UI スレッド）からしか起点を作れない。** 時間切れの判断は `balloon_visibility_wait.rs` の `decide_timeout` にあり、送り手はまだ無い。ただし kanade への送出端は `frame/status_report.rs` が既に `GhostSlot` から取り出して使っており、**同じ取り出し方なら `frame/wiring.rs` と `emo2_boot/mod.rs` を触らずに済む**（brief㉒の「表示の側から kanade へ送る口が無い＝wiring.rs と mod.rs を触る」は、別の既存の道で解ける）。運ぶ器は `msg.rs` に触れない以上、既存の `KanadeMsg::RaiseEvent` しか無い。UI は台本を持たないので、Reference0 は kanade 側で埋めることになる。
- **要件 5（中断で終わったトークの時間切れの起点）の前提に事実の食い違いがある。** 今の計測の起点は「台本を最後まで流したら終わるはずの時刻」ではなく「**それまでに配られた cue の終わりの最大値**」である（dola の cue は発火の時点で 1 件ずつ配られ、中断で残りは捨てられる）。余分に残るのは「止めた時点で進行中だった cue の残りの長さ」だけで、今ある中断の種類（利用者の中断＝即座に隠す・選択肢の時間切れの解除＝選択肢の表示中の抑止が効く）では、**目に見える差がほぼ出ない**。正確に扱うなら、talk スレッドで受け口が落ちる時刻を talk 相対秒で送るのが最短（既存の `NoUserBreakCueSink` の `Drop` と同じ型）。
- **中断位置（Reference2）の源はどこにも無い。** 字句の走査の本体は区間（バイト位置）を作っているが、`lex` が捨てている。`Instruction`・コンパイル・cue・完了の知らせ `TalkDone`（27 ファイル・60 か所で組まれている）のどれにも位置が無い。要件の既定（空で送り、別 spec に切る）は規模の見立てと一致する。
- **規模 M（10〜13 タスク）・リスク 中。** `schedule/mod.rs` は 938 行で、本 spec の足し分は 20 行前後の見込み＝1,000 行を越えず、60 行の閾値も越えない。手間の山は「台本を控える欄」を足すと `State` を `..` なしで組むテストの 14 か所を機械的に直すことと、既存の檻「中断で SHIORI へ 1 件も送らない」（`user_break_tests.rs`）の書き直し。

---

## 2. 今のコード（関係する資産）

### 2.1 会話の進行の側（kanade・`crates/areka-kanade/src/schedule/`）

| 資産 | 場所（何の定義行か） | 本 spec との関係 |
|---|---|---|
| 運行の入力 `Input::UserBreak{scope}` | `schedule/mod.rs:101-103`（`Input` の `UserBreak` の変種） | 中断の合図。scope はここまで届いている |
| 振り分け | `schedule/mod.rs:544`（`route` の `UserBreak` の腕）・`:550-554`（`RaiseEvent` の腕＝`change::on_raise_event` へ） | 中断・汎用の入口の入り口 |
| 中断の受理 | `schedule/user_break.rs:18-56`（`on_user_break`）。`:19-29` が「再生中のトークが無い」腕（記録だけで何も返さない）、`:31-39` が「同じトークへの 2 回目」腕、`:50` で `user_break_talk = Some(talk_id)` | `OnBalloonClose` の起点に使える腕と、`OnBalloonBreak` の scope を控える場所 |
| 中断の帳簿の後始末 | `schedule/user_break.rs:67-70`（`take_user_break_quit`＝帳簿を `take` して「終了の予約つきの中断か」だけを返す） | 「中断で終わったか」を返さない＝本 spec で形を変える |
| 中断の帳簿の欄 | `schedule/mod.rs:236-237`（`State.user_break_talk: Option<TalkId>`） | scope を控えていない |
| 再生中のトーク | `schedule/mod.rs:171-179`（`ActiveTalk`。`script` は `OnChoiceTimeout` の Reference0 の源） | Reference0 の源。ただし完了で捨てられる |
| トークの完了の横断の腕 | `schedule/mod.rs:693-764`（`on_talk_done`）。`:706-708` が `break_quit` の計算、`:715-720` 終了の予約、`:722-726` 保留の切替、`:727-736` 中断（非 quit）、`:737-740` 最後まで | `OnBalloonBreak` を積む場所の最有力 |
| 定常の完了 | `steady.rs:861-879`（`on_talk_done`。保留の終了があれば `begin_close`、無ければ `Steady{talk: None}`＝ここで `ActiveTalk` ごと台本が消える） | 触らない約束のファイル。外（`mod.rs`）から前後を見れば足りる |
| 起動の挨拶の完了 | `boot.rs:312-316`（`on_talk_done`＝`BootVersion{talk: None}` に戻すだけ） | 定常ではない＝送らない側 |
| 切替の送り出しの中断 | `change.rs:264-278`（`on_talk_wait`。`ChangeTalkWait` と `ChangeCloseTalkWait` のどちらの `Interrupted` も `cancel_by_user_break` へ）・`:314-323`（`cancel_by_user_break`＝帳簿を消して `Steady{talk: None}` と「切替の中止」の知らせ） | 討議 E の対象 |
| 切替の台本の控え | `change.rs:219-221`（`OnGhostChanging` の台本だけ `change.script` に控える。切替の `OnClose` の別れの台詞（`closing`）は控えない） | 切替の別れの台詞を中断したときの Reference0 の源が無い |
| 汎用の入口 | `change.rs:84-100`（`on_raise_event`。許可表と「`Phase::Steady{..}`」だけを見る＝**再生中（`Steady{talk: Some}`）でも送る**） | `OnBalloonTimeout` をそのまま流すと「次のトークが始まっていたら送らない」（要件 2.6）を満たさない |
| 応答で今のトークを置き換えるか | `events.rs:241-243`（`value_replaces_active_talk`＝`OnSecondChange` 以外は置き換える） | 3 語の応答は再生中なら置き換えになる |
| 送ってよいイベントの表 | `events.rs:108-216`（`ALLOWED_EVENT_IDS`・各行に ukadoc の URL の注記）・`events_change_tests.rs:39`（`len() == 48`） | 3 行を足して 51。URL の注記が台帳の「実装済み」の証拠になる |
| 組み立ての型 | `events.rs:719-725`（`on_choice_timeout`＝GET・Reference 1 個・`ExecutionStatus::derive`） | 3 語の組み立ての手本 |
| 印の台詞の中断 | `talk_gap.rs:121-129`（`is_marked_break`＝`OnShellChanging` の台詞の利用者の中断）・`:193-227`（`decide`＝定常で再生中でなければ「切れ目に達した」） | シェルの切替の送り出しの中断（討議 E）・切れ目の見張りとの絡み |
| 殻の一括の実行 | `actor.rs:10-17`（「一括の中の SHIORI の往復は高々 1 本」の約束）・`:433-` の `execute_batch` | 完了の腕から GET を 1 本積むのは前例あり（`begin_close` が `OnClose` を積む） |

### 2.2 表示の側（UI スレッド・`crates/areka/src/`）

| 資産 | 場所 | 本 spec との関係 |
|---|---|---|
| 押下の判定 | `input_events/user_break.rs:112-130`（`judge_press`）。バルーンの窓は**再生中かどうかを見ない**＝出ているバルーンの左ダブルクリックは常に `Break` | 再生中でないダブルクリックも kanade へ `UserBreak` として届く |
| 中断の送り出し | `input_events/user_break.rs:320-`（`send_break`＝表示の線へ `UserBreak`、kanade へ `KanadeMsg::UserBreak{scope}`） | 触らなくてよい |
| 箱（シェルの中のバルーン）の判定 | `input_events/shell_box.rs:47-67`（`judge_box_press`＝**話していなければ `ShellOp`**＝シェルのダブルクリック `OnMouseDoubleClick` へ流れる） | 箱では「閉じる」が起きない（討議 B） |
| 表示の線の信号 | `emo2_boot/talk_lifecycle.rs:53-`（`TalkLifecycleSignal`＝`TalkStarted`・`DisplayEndAt(秒)`・`UserBreak`） | 起点の精密化に信号を 1 つ足す候補 |
| 占有終端を送る受け口 | `emo2_boot/talk_lifecycle.rs`（`BalloonLifecycleSink`。`emit` のたびに `at + duration` の最大値を更新して送る。複製＝会話の境界） | 配られた cue だけを数えている |
| 予約の型 | `emo2_boot/talk_lifecycle.rs:195-196`（`#[allow(dead_code)]` と `BalloonLifecycleNotice`）。変種 `Closed{script}`・`TimedOut{script, remaining_ms}`・`Broken{script, scope, break_position}` | 「表示の側が台本を持っている」前提の形。実際には UI は台本を持たない |
| 判断中核 | `balloon_visibility_decision.rs:22-`（`decide`＝信号の畳み込み→中断→内容→時間切れ）・`:104-`（`apply_lifecycle_signals`）・`:171-`（`decide_user_break`） | 変えない |
| 時間切れの判断 | `balloon_visibility_wait.rs:76-221`（`decide_timeout`）。満了で隠す scope を `:209-220` で返す（抑止の間は返さない） | `OnBalloonTimeout` の起点。返り値が空でない巡に 1 回送れば要件 2.1・2.4 を満たす |
| 可視性の相 | `balloon_visibility_phase.rs:68-132`（`run_balloon_visibility_phase`。`world` を持つ） | 送り出しを足す場所の候補（751 行） |
| kanade への送出端の取り出し | `frame/status_report.rs:159-163`（`GhostSlot` → `session.kanade()`） | 同じ取り出しで送れる＝結線の束を変えずに済む |
| 時刻源 | `emo2_boot/talk_clock.rs:63-76`（`TalkClock::talk_time`）・`emo2_boot/mod.rs:450-452`（`TalkClock::new`） | 中断の時刻を talk 相対秒で取るのに使える |
| 落ちるときに送る受け口の前例 | `emo2_boot/user_break_cue.rs:120-126`（`NoUserBreakCueSink` の `Drop`＝合図を 1 つでも送ったトークの複製が落ちるときに `TalkEnded`） | 同じ型で「止まった時刻」を送れる |

### 2.3 再生の側（dola・sakura）

- `dola/src/cue/runtime.rs:200-226`（`CuePlayer::tick` の配送）——cue は**時刻が来た巡に 1 件ずつ**全受け口へ配られる（`:224` の `sink.emit`）。
- `areka-sakura/src/drive.rs:416-430`（`on_close`）——中断で `player.stop()` して残りを捨て、受け口を落としてから中断の知らせを送る。
- `areka-talk/src/lib.rs:112-118`（`TalkDone`＝`talk_id`・`reason`・`quit_reserved` の 3 欄。位置も時刻も無い）。
- `areka-parsers/src/sakura/lexer.rs:119`（`scan`＝字句ごとに区間 `Range<usize>`（バイト）を渡す走査の本体。`lex` は区間を捨てる）。

---

## 3. 要件と資産の対応

凡例: **既存**＝そのまま使える／**不足**＝作る／**未知**＝設計で調べる／**制約**＝決まりや約束による縛り。

| 要件 | 必要なもの | 状態 |
|---|---|---|
| 1.1 中断の後に定常へ戻ったら送る | 完了の横断の腕（`mod.rs` の `on_talk_done`）の後で「中断で終わった・終了へ進まない・結果が `Steady{talk: None}`」を見て GET を積む | **不足**（腕は既存・判断を足す） |
| 1.1 起動の挨拶 | 本番では `OnBoot` の応答→`basewareversion` の往復が同じ一括で済み、挨拶は `Steady{talk: Some(origin=OnBoot)}` で中断される＝定常の道に乗る。`BootVersion{talk: Some}` で完了が届く道（`boot.rs:312-316`）は守りの腕 | **既存**（守りの腕では送らない＝要件 4.4 と一致） |
| 1.1 切替の送り出し | `change.rs:314-323` が `Steady{talk: None}` へ戻す | **不足**（台本の控えは `OnGhostChanging` だけ・討議 E） |
| 1.2 Reference0＝止めたトークの台本 | `ActiveTalk.script`（翻訳の後の台本に書き換わる＝`translate.rs:233-242` の `rewrite_notes`） | **既存**（完了の前に取り出す必要あり） |
| 1.3 Reference1＝scope | `on_user_break(state, scope)` の引数 | **不足**（帳簿に控えていない） |
| 1.4 Reference2＝空 | — | **既存**（空文字を入れるだけ） |
| 1.5 止まり終えてから送る | `TalkDone` は talk スレッドが受け口を落とした後に届く（`drive.rs:426-428`） | **既存** |
| 1.6 定常へ戻らないとき送らない | 終了の予約（`mod.rs:715-720`）・保留の終了（`steady.rs` の `begin_close`→`ClosePending`）・保留の切替（`mod.rs:722-726`→`begin_change`）・別れの台詞（`close.rs`→`Unloading`）はどれも結果の相が `Steady{talk: None}` にならない | **既存**（結果の相で判定すれば自然に外れる） |
| 1.7 受理されない中断 | `on_user_break` の 2 つの腕は帳簿を立てない・UI の「禁じる区間」は kanade へ送らない | **既存** |
| 1.8 1 回だけ | 帳簿は現行トークの完了で必ず空になる（`user_break.rs:67-70`） | **既存** |
| 2.1 時間切れで 1 回送る | `decide_timeout` の返り値（空でない巡）→ kanade へ送る口 | **不足**（送り手） |
| 2.2 Reference0＝直前に終えたトークの台本 | kanade が「直前に終えたトークの台本」を持つ欄 | **不足**（`steady.rs` の完了で捨てられる） |
| 2.3 Reference1＝`0` | 定数 | **既存** |
| 2.4 抑止の間は送らない | `decide_timeout` は抑止の間は対象を返さない | **既存** |
| 2.5 他の理由では送らない | 隠す契機の種類 `VisibilityTrigger::Timeout` だけが起点 | **既存** |
| 2.6 次のトークが始まっていた・定常でない | `on_raise_event` は定常かだけを見て、再生中でも送る（`change.rs:94-99`） | **不足**（専用の判定が要る） |
| 2.7 選択肢の時間切れでは送らない | `OnChoiceTimeout` は kanade の別の道（`steady.rs` の `fire_choice_timeout_if_due`） | **既存** |
| 3.1 再生中でない定常のダブルクリックで送る | `on_user_break` の「再生中のトークが無い」腕 | **不足**（腕は既存・送出を足す） |
| 3.1 シェルの中のバルーン | `judge_box_press` は話していなければ `ShellOp`＝`UserBreak` は飛ばない | **制約**（討議 B） |
| 3.4 定常でない間は送らない | `on_user_break` は相を問わない腕なので、相の判定を足す。終了の保留中はマウスの GET を出さない決まり（`steady.rs:67-79` の `on_mouse` の先頭）と揃えるか | **不足**／**未知**（保留の終了の扱い） |
| 4.1〜4.3 GET・翻訳・204 | `Action::ShioriRequest` を積めば、`translate::before/after`（`mod.rs:440-449`）と `steady.rs` の `on_reply` が既存のまま扱う | **既存** |
| 4.4 定常の間だけ・積まない | kanade の判定で決まる | **不足**（判定を足す） |
| 4.6 失敗の扱い | 殻の `round_trip_request`（エラー応答は GET なら 204 扱い） | **既存** |
| 5.1〜5.6 中断の起点 | 中断の時刻を talk 相対秒で UI へ届ける信号・判断中核で min を取る | **不足**＋**未知**（§4 の「要件 5 の前提」） |
| 6.1〜6.7 記録の更新 | COMPAT §8 の行（`doc/COMPAT_ARCHITECTURE.md:161`・`:162`・`:165`・`:166`）・台帳（`shiori.toml:1297-` ほか 3 行・`sakura-script.toml:3019-`）・生成物 | **不足**（文書）。台帳の状態の語彙に `degraded`（縮退）がある＝`OnBalloonBreak` に使える |
| 6.6 予約の印を外す | `talk_lifecycle.rs:195` | **未知**（型そのものを残すか・§5） |
| 7 記録 | `tracing` の `target: "kanade"`・`event = ...` の流儀（`user_break.rs` と同じ） | **既存**（流儀） |
| 8.1 決定論のテスト | `schedule` は純関数で `step` を直接駆動できる（`user_break_tests.rs`・`change_cancel_tests.rs` が手本）。UI の判断中核も純関数 | **既存**（手本あり）。既存の檻「中断で SHIORI へ 1 件も送らない」（`user_break_tests.rs:101-110` の補助と、それを使う各テスト）は書き直しになる |
| 8.3 実機 | `AREKA_BALLOON_TIMEOUT_MS` で待ち時間を短縮できる（`balloon_visibility.rs` の `TIMEOUT_ENV_KEY`） | **既存** |

---

## 4. 討議で確定する点に関わるコードの事実

### A. 中断位置（Reference2）を今作るか

- 位置の源は無い: `TalkDone` に欄が無い（`areka-talk/src/lib.rs:112-118`）。cue にも位置が無く、字句の段は区間を作るが捨てている（`lexer.rs:119` の `scan` は区間を渡す・`lex` は使わない）。
- 作るときの手間は brief㉑の見立てのまま: `TalkDone {` の書き方は 27 ファイル・60 か所（`crates/` 全体で再数えて一致）。さらに字句→`Instruction`→コンパイル→dola の cue→再生→完了の知らせ、の 5 段を通す。
- 作るときに決めねばならない未知が 3 つ増える: ⑴ 正典は「文字数」だが走査はバイト位置 ⑵ どの台本の中の位置か（`OnTranslate` で書き換えた後か・`%` の環境変数の置き換え（`lexer.rs` の `substitute_system_vars`）の前か後か） ⑶ 待機の途中で止めたときの位置（その待機タグの前か後か）。
- → 要件の既定（空で送る・縮退を登記・別 spec）を変える材料は見つからなかった。

### B. `OnBalloonClose` が起きる場面

- 「再生中でないときのダブルクリック」は**既に kanade まで届いている**: バルーンの窓の押下は再生中かどうかを見ずに `KanadeMsg::UserBreak` を送り（`input_events/user_break.rs:112-130` の `judge_press` に再生中の判定が無い・`:320-` の `send_break`）、kanade は `user_break.rs:19-29` の腕で「何も止めない」と記録して終わる。表示の側は同じ合図でバルーンを隠す（`balloon_visibility_decision.rs:104-` の `UserBreak` の畳み込み）。＝既定の案は **UI を 1 行も変えずに** kanade の 1 つの腕で実現できる。
- 「再生中か」の判断は kanade が持つ: 台詞の文字が出し終わっていても、末尾の待機（`\w` など）が残っていれば kanade では再生中＝`OnBalloonBreak` になる（トークの完了は占有区間の終端まで来ない）。利用者の目には「読み終えたバルーンを閉じた」に見える場面で `OnBalloonBreak` が飛ぶ幅がある。
- **シェルの中のバルーン（箱）では起きない**: `judge_box_press`（`shell_box.rs:47-67`）は話していなければ `ShellOp` を返し、ダブルクリックはシェルへの操作（`OnMouseDoubleClick`）になる。箱の決まりは完了 spec `areka-P0-shell-balloon` の要件 9.1・9.7 の持ち物。既定の案のままなら「箱では `OnBalloonClose` は起きない」と書くか、箱の決まりを変えるかの判断が要る。
- 終了の保留中（`Steady{talk: None}` かつ `pending_close` あり＝次の刻みで終了の握手へ進む一瞬）にダブルクリックが来たとき、マウスの GET は出さない決まり（`steady.rs:67-79`）がある。揃えるかどうかは要件 3.4 の「終了の握手の待ち」の読み方次第。
- Reference0 の源は「直前に終えたトークの台本」を kanade に控える欄（新設）。控えが無いとき（起動直後にトークが 1 本も無かったのにバルーンが出ていた、などの守りの場面）に空で送るか送らないかは設計で決める。

### C. 項目 7（`balloontimeout` の表示の側）の扱い

- 台本のコンパイルの側の持ち主 `areka-P0-sakura-time-directives` の brief は、今「`balloontimeout`（受ける側は `balloon-lifecycle-events` の項目 7）」と書いている（`.kiro/specs/areka-P0-sakura-time-directives/brief.md:58`）。切り離すなら、この行の書き換えが相互登記の作業になる（要件 6.5）。
- 台帳 `sakura-script.toml:3019-3034` の `balloontimeout` の行の `owner` は分割元 `areka-P0-balloon-canon-residue` のまま。COMPAT §8 の 2 行（`:129` のコンパイルの側・`:165` の寿命の側）の追跡先も分割元のまま。
- 表示の側の作り（ここに書くのは参考）: 判断中核は待ち時間を引数で受けるだけ（`decide(state, obs, now, timeout_secs)`）なので、トークごとの指定を受ける形にするのは軽い。重いのは指定の値を台本から UI まで運ぶ道（汎用の `\!` の運び手の受け口を 1 本足す型＝`consumer_ledger.rs` の流儀）と、コンパイルの側の「位置に依らず効く」の扱い。項目 7 を切り離しても本 spec の他の作業は 1 つも変わらない。

### D. `OnBalloonTimeout` の Reference1（残り時間）

- 正典は「残り時間。」としか書かない（ukadoc `list_shiori_event.html#OnBalloonTimeout` を MCP で再確認）。
- areka の時間切れは「満了予定に達した巡」で決まる（`balloon_visibility_wait.rs` の `expired = now >= deadline`）。満了の時点の残りは常に 0 以下で、`0` 以外の値を作る材料はコードに無い。単位（ミリ秒か秒か）を決めずに済むのも `0` だけ。
- SSP の実際の値は未確認（**調べ物**: SSP で実測すれば決まるが、開発者の方針は「SSP の実測主義は取らない」）。

### E. 切替の送り出しの台詞を中断したとき

- ゴーストの切替の送り出しは 2 段ある: `OnGhostChanging` の台詞（`ChangeTalkWait`）と、204 のときに続けて送る切替の `OnClose` の別れの台詞（`ChangeCloseTalkWait`）。**どちらの中断も** `cancel_by_user_break`（`change.rs:264-278`・`:314-323`）で切替を中止して `Steady{talk: None}` へ戻る。
- 要件 1.6 は「別れの台詞を止めた」を送らない側に挙げている。終了の握手の別れの台詞（`CloseTalkWait`）は確かに終了へ進む（`close.rs`）が、**切替の `OnClose` の別れの台詞は中断すると定常へ戻る**。規則「定常へ戻ったら送る」をそのまま当てると、こちらは送る側になる。要件 1.6 の言い回しを「終了の握手の別れの台詞」に絞るかの確認が要る。
- Reference0 の源: `OnGhostChanging` の台本は `change.script` に控えてある（`change.rs:219-221`）が、切替の `OnClose` の台本は控えていない（同じ行の `!closing` の条件）。送る側にするなら控えを足す（`change.rs` は 425 行で余裕あり）か、「直前に始めたトークの台本」を一般に控える欄で兼ねる。
- シェルの切替の送り出し（`OnShellChanging`）は切替の相を通らず定常のトークとして再生される。中断は `talk_gap.rs:121-129` が「印の台詞の中断」と読み、`mod.rs:706-708` で終了の予約を無視して定常へ戻る＝定常の道に乗る。UI は同じ巡の `talk_gap::observe` で `CancelledByUser` を受けてシェルの切替を取りやめる。
- 中断の後に `OnBalloonBreak` の応答で新しいトークが始まると、それは「切替を取りやめた後の最初のトーク」になる。切替の取りやめの知らせ（`Action::Notice`）と GET は同じ一括に入り、知らせが先に UI へ流れる（`actor.rs` の `execute_batch` は列の順に実行する）。

---

## 5. 実装の案

### 共通（どの案でも同じ）

- `events.rs`: 許可表に 3 行（ukadoc の URL の注記つき）と組み立て関数 3 本。`events_change_tests.rs:39` の数を 48→51。`lib.rs` の `pub mod events` の `pub use` に 3 本。
- kanade の `State` に「直前に終えたトークの台本」を控える欄、中断の帳簿に scope。
- 文書: COMPAT §8 の 4 行・台帳 4 行・生成物・関係する brief の相互登記。

### 案 A: kanade に寄せる（既存の部品を広げる）

- `OnBalloonBreak`: `mod.rs` の `on_talk_done` で、振り分けの前に「中断の帳簿」「止めたトークの台本」を取り出し、振り分けの後の相が `Steady{talk: None}` なら GET を積む（切替の中止・シェルの切替の中止・定常の完了をこの 1 か所で拾える＝`steady.rs` に触れない）。`take_user_break_quit`（`user_break.rs`）は「中断で終わったか」と「終了の予約か」を両方返す形に変える。
- `OnBalloonClose`: `on_user_break` の「再生中のトークが無い」腕で、定常（と保留の終了の扱い）を見て GET を積む。
- `OnBalloonTimeout`: UI の可視性の相が、`decide_timeout` が対象を返した巡に `KanadeMsg::RaiseEvent{id: "OnBalloonTimeout", ..}` を送る（送出端は `status_report.rs` と同じ `GhostSlot` からの取り出し）。kanade は `route` の `RaiseEvent` の腕（または `change::on_raise_event` の先頭）でこの名前だけを専用の判断へ回し、「再生中なら送らない」と Reference0 の補いを行う。
- 起点の精密化（要件 5）: `BalloonLifecycleSink` に `TalkClock` の複製を持たせ、`Drop`（トークの複製が落ちるとき）で「止まった talk 相対秒」を新しい信号で送る。判断中核の `apply_lifecycle_signals` で `display_end = min(display_end, 止まった時刻)`。最後まで流れたトークでは落ちる時刻が占有終端より後になるので、同じ式で今と同じ値になる（要件 5.3 が式の帰結になる）。
- ✅ `msg.rs`・`actor.rs`・`steady.rs`・`spine.rs`・`frame/wiring.rs` に触らない。`emo2_boot/mod.rs` は受け口の組み立て 1 行（`TalkClock` を渡す）だけ。
- ❌ `OnBalloonTimeout` だけ「汎用の入口」なのに中身を kanade が補う特別扱いになる。汎用の入口は将来 MCP の `raise_event`（`mcp-kanade-tools`）からも使われるので、外から `OnBalloonTimeout` を頼まれたときの扱い（補うのか、渡された値を使うのか）を決めておく必要がある。

### 案 B: 新しいファイルに分ける

- kanade に `schedule/balloon_events.rs`（仮）を新設し、3 語の判断（送るか・Reference の組み立て・送らなかった理由の記録）を全部置く。`mod.rs` は呼ぶだけ（数行）、`user_break.rs` は帳簿の形の変更だけ。
- UI 側は可視性の相の子に送り出しの小さなファイル（例 `balloon_visibility_notify.rs`）を置く。
- 予約の型 `BalloonLifecycleNotice` は「UI→kanade で台本を運ぶ」前提の形で、案 A・B のどちらでも UI は台本を持たないため、**そのままの形では使い道が無い**。外すか（kanade 側に同じ語彙の判断があるので重複しない）、`TimedOut` だけを UI の送り出しの内部の型として残すか。
- ✅ `mod.rs` の行数が増えない・テストを兄弟ファイルにまとめやすい・後の spec（`talk-fast-forward`・`anchor-tag-canon`）との文字の衝突が減る。
- ❌ ファイルが 2 つ増える。

### 案 C: 段に分ける

- 1 段目＝`OnBalloonBreak` と `OnBalloonClose`（kanade だけで閉じる）。2 段目＝`OnBalloonTimeout`（UI の送り出し＋kanade の受け）。3 段目＝起点の精密化（要件 5）。
- ✅ 各段が単独で緑にできる。要件 5 を討議の結果で小さくしても前の段に響かない。
- ❌ 計画が少し増える。全体は 1 spec のまま（規模は変わらない）。

---

## 6. 行数と同じウェーブの約束

| ファイル | 今 | 見込み | 判断 |
|---|---|---|---|
| `areka-kanade/src/schedule/mod.rs` | 938 | 案 A で +15〜25（欄 2・初期値 2・完了の腕の前後の取り出しと判定・`RaiseEvent` の振り分け）／案 B で +5〜10 | 1,000 を越えない・60 行の閾値も越えない＝分割は不要の見込み |
| `schedule/steady.rs` | 947 | 0 | 触らずに済む（完了の前後は `mod.rs` から見る・Reference0 は `mod.rs` で振り分けの前に取り出す） |
| `schedule/user_break.rs` | 75 | +20〜40 | 余裕あり |
| `schedule/change.rs` | 425 | 0〜+10（切替の `OnClose` の台本を控えるなら） | 余裕あり |
| `schedule/events.rs` | 733 | +40〜50（3 行の表＋組み立て 3 本） | 余裕あり |
| `msg.rs`（909）・`actor.rs`（876） | — | 0 | **触らない**（`mcp-get-status` の持ち物）。新しい `KanadeMsg` は作れない＝`UserBreak` と `RaiseEvent` の既存の 2 つで足りる |
| `areka/src/emo2_boot/balloon_visibility_phase.rs` | 751 | +20〜30 | 余裕あり（新しい子ファイルに出せばさらに少ない） |
| `balloon_visibility_decision.rs` | 281 | +5〜10（起点の min） | 余裕あり |
| `emo2_boot/talk_lifecycle.rs` | 212 | ±（信号 1 つ・`Drop`・予約の型の扱い） | 余裕あり |
| `emo2_boot/mod.rs` | 883 | +1〜3（受け口に時刻源を渡すなら） | 余裕あり |
| `emo2_boot/spine.rs` | — | 0 | **触らない**（`ghost-session-test-load-flake` の持ち物） |
| `frame/wiring.rs` | 368 | 0 の見込み | `GhostSlot` から取り出す形なら触らない |

- `State` を `..` なしで組む所: `user_break_talk:` で数えて 15 か所（初期値 `mod.rs:267` の 1 か所＋テスト 14 か所＝`boot_reply_branch_tests.rs` 4・`boot_sequence_tests.rs` 6・`close.rs` の中のテスト 2・`schedule_log_firing_tests.rs` 2。brief㉒の「16」は数え直すと 15）。新しい欄を足すとこの全部に 1 行ずつ要る。中断の帳簿の型を `Option<TalkId>` から `Option<中断の控え>` に変えるだけなら `None` の 14 か所は直さずに済み、比較を書いている所（`user_break.rs:31`・`:69`・`talk_gap.rs:127`・`user_break_tests.rs` の 4〜5 か所・`translate_path_tests.rs:225`）だけ直す。
- 同じウェーブとの文字の衝突: `choice-script-prefix` は `steady.rs` の `on_choice` を触る＝本 spec が `steady.rs` に触らなければ衝突 0。`mcp-get-status` は `msg.rs`・`actor.rs` だけ＝衝突 0。`char-position-save-on-exit` と `open-external-tags` は `emo2_boot/mod.rs` に触らない約束＝衝突 0。

---

## 7. 規模とリスク

- **規模: M（3〜7 日・10〜13 タスク）**——kanade の判断 3 つ・UI の送り出し 1 つ・起点の精密化 1 つ・文書と台帳。いずれも既存の型（`OnChoiceTimeout` の組み立て・`status_report.rs` の送り出し・`NoUserBreakCueSink` の `Drop`）の写しで書ける。Reference2 を作るなら L（18〜24）で、brief の推しどおり別 spec。
- **リスク: 中**——新しい技術は無いが、⑴ UI と kanade の 2 本の線の到着順に依る判断（時間切れの最中に次のトークが始まる・ダブルクリックと完了の行き違い）⑵ 切れ目の見張り（`talk_gap`）と、中断の後の GET の応答で始まるトークの重なり ⑶ 既存の檻（中断で SHIORI へ送らない）を約束の改めとして書き直すこと、の 3 つで取りこぼしが出やすい。

---

## 8. 設計へ持ち越す調べ物（Research Needed）

1. **要件 5 の実害の有無**: 中断の時点で「進行中の cue の残り」が占有終端を押し上げる場面が、今ある中断の種類で起きるか。選択肢の時間切れの解除は区切り（`WaitForChoice`）で止まっている最中なので、配られた cue の終わりは中断の時刻より前のはず（要確認: `ChoiceWaiting` の `display_end` と区切りの時刻の関係）。起きないなら、要件 5 は「今後の中断の種類（`\x` など）への備え」になる。
2. **選択肢の時間切れの解除の後のバルーン**: 選択肢の行は内容の消去（`state_decoration.rs` の `clear_content`）でしか消えないように読める。204 で解除したトークの選択肢が表示に残ると、選択肢の表示中の抑止（`balloon_visibility_wait.rs` の `observe_suppression`）が効き続け、時間切れも `OnBalloonTimeout` も次のトークまで起きない。実際にそうなるかを確かめる（そうなら本 spec の範囲外の起票の対象）。
3. **時間切れと次のトークの行き違い**: UI が時間切れを決めた後、kanade では次のトークが「始まって終わっていた」場合、Reference0 が新しいトークの台本になる。UI はトークの番号を知らない（受け口は番号を受け取らない）。起きる幅と、許すか（記録だけ残す）を設計で決める。
4. **切れ目の見張りとの絡み**: 中断で定常へ戻った同じ巡で `talk_gap::observe` は「切れ目に達した」と決める（`talk_gap.rs:193-227`）。その後に `OnBalloonBreak` の応答で新しいトークが始まると、切れ目を待っていた側（インストール・更新・シェルの切替の送り出しの後の知らせ）の汎用の入口の応答がそのトークを置き換える。今の pump（`OnSecondChange`）と同じ性質で問題が無いかを確かめる。
5. **起点の時刻の軸**: `TalkClock` の epoch は全トークで 1 つ。受け口が落ちる時点では次のトークの cue はまだ観測されていない（次のトークは前のトークの合流の後に起きる＝`user_break_cue.rs:108-117` の説明）ので、落ちた時点の `talk_time` は止めたトークの軸のはず。設計で決定論のテストに落とす。
6. **Reference2 を作る後続 spec の名前と範囲**（要件 6.2）: 本 spec の完了までに `/kiro-discovery` で起票。上の A の未知 3 つを brief に写す。

---

## 9. 要件に直しが要るかもしれない点（討議へ渡す）

1. **要件の「今どうなっているか」と用語「占有区間の終端」の説明が、コードの事実と食い違う。** 要件は「中断で終わった会話でも、時間切れまでの計測は『台本を最後まで流したら終わったはずの時刻』から始まる」と書くが、実際の起点は「中断までに**配られた** cue の `at + duration` の最大値」である——cue は発火の巡に 1 件ずつ配られ（`dola/src/cue/runtime.rs:200-226`）、中断で残りは捨てられ（`areka-sakura/src/drive.rs:424-427`）、受け口は配られた cue だけを集約する（`emo2_boot/talk_lifecycle.rs` の `BalloonLifecycleSink::emit`）。余分に残るのは進行中の cue の残りの長さだけ。完了 spec の注記（`talk_lifecycle.rs` 冒頭の「中断も起点は正常終了と同一値」・COMPAT `:161`）も同じ粗さで書かれている。要件 5 の目的・用語の書き直しと、要件 5 の規模（§8 の 1）を討議で確かめたい。
2. **要件 1.6 の「別れの台詞を止めた」**は、終了の握手の別れの台詞（終了へ進む）と、ゴーストの切替の `OnClose` の別れの台詞（中断すると切替が中止されて定常へ戻る・`change.rs:264-278`）を区別していない。討議 E の答えと一緒に言い回しを決める。
3. **要件 3.1 の「出ているバルーン」にシェルの中のバルーン（箱）が含まれるか。** 箱は話していないときのダブルクリックをシェルへの操作に回す（`shell_box.rs:47-67`）ので、既定の案のままでは箱で `OnBalloonClose` は起きない。要件 1.3 は箱を明記しているので、要件 3 でも明記したほうが読み違いが無い。
4. **要件 6.6（予約の型の印を外す）**: 予約の型 `BalloonLifecycleNotice` は「表示の側が台本・scope・位置を kanade へ渡す」前提の形だが、実際に台本と scope を持つのは kanade である。印を外して型を「使う」のではなく、型そのものを消すことになる見込みが高い（語彙と Reference の割り当ては kanade の組み立て関数と COMPAT §8 が持つ）。要件の「外す」に「型ごと消すことを含む」と読めるかを確かめたい。
5. **要件の「今どうなっているか」の「時間切れでバルーンを隠す判断は表示の側にあり、それを会話の進行の側へ伝える口が無い」**は正しいが、brief㉒の「`frame/wiring.rs`・`emo2_boot/mod.rs` を触る」は必須ではない（`frame/status_report.rs:159-163` と同じ取り出しで送れる）。要件の本文は直さなくてよいが、設計の触るファイルの一覧はこちらで組み直す。
