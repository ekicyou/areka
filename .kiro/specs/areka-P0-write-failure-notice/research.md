# ギャップ分析: areka-P0-write-failure-notice

> 2026-10-16・本ブランチ（main `0bb51d42` の上に spec の初期化 `c4a0799a`）のコードを読んだ結果。コードは関数名・型名とファイルで指し、行番号は目安として添える。
> この文書は「選べる道と、それぞれに何がかかるか」をまとめる。決めるのは要件の討議（下の「討議へ渡す項目」）。

## 1. 要約

- **知らせの道（議題 1）で、今の areka に「ベースウェアが組んだ文を、動いているゴーストのバルーンに出す」口は 0 本。** ただし部品は揃っている。kanade の中に「台本 1 つを、SHIORI を通さずに新しいトークとして始める」関数 `start_talk`（`crates/areka-kanade/src/schedule/steady_choice_script.rs`・`schedule` の中だけで使える）と、「台詞の切れ目まで待つ」口 `KanadeMsg::AwaitTalkGap`（`schedule/talk_gap.rs`）が在る。外から台本を流す口（`KanadeMsg` の変種）は、未着手の `mcp-kanade-tools`（v0.1.0）が作る予定のもの。候補 (a) は、この口をこの spec が先に作ることになる。
- **書き込みの失敗を 1 か所で拾える場所が在る。** ゴーストが動いている間の書き込み（最後に使ったもの・印の投函・置き場の並び・窓の位置・プロパティ・入れたバルーン）は、どれもそのゴーストの記憶の書き手（`areka-sylphya` のアクター `run_actor`）を通る。アクターは保存の結果 `PersistOutcome` をその場で知っている。ここに「失敗を外へ渡す口」を 1 つ置けば、呼び手 9 か所を直さずに要件 1.3（投函だけの書き込みも漏らさない）を満たせる。
- **`PersistOutcome::Degraded` は中身を持たない**（`Copy` の 2 値）。パスと OS の理由は `save_scope` の `error!` にだけ在る。要件 1.4 には、`Degraded` に中身を持たせるか、別の返し方を足す必要がある。
- **インストールの語（議題 2）は小さい。** `NarError::Io` は段 `IoPhase`（`Read`・`Stage`・`Commit`・`Rollback`）を持つので、`areka-nar` に触らずに「`.nar` を読めない＝`Read`」と「書けない＝`Stage`・`Commit`・`Rollback`」を分けられる。根が無いとき（`install/destination.rs` の `install_into`）は `Stage` で作っているので、同じ語へ入る。検体のゴースト 5 体の辞書は、知らない語でも壊れない（下の §5）。
- **調べが要る所**: ⑴ (a) を採るとき、ベースウェアの台本を `OnTranslate` に通すか・再生中のトークを止めるか待つか、⑵ パスをさくらスクリプトに入れるときの `\` と `%` の扱い、⑶ `user-data-root` とファイルが重ならない結線の場所。

## 2. 今のコードの地図（要件ごと）

| 要件 | 今在るもの（定義点） | 足りないもの | 印 |
|---|---|---|---|
| 1.1 アプリの記憶（UI スレッドから直接書く 3 つ） | `crates/areka/src/boot_resolve.rs` の `save_app` → `write_session_mark`・`write_switch_drop`・`clear_session_mark`。結果は見て `warn!` するが戻り値は `()` | 結果を呼び手へ返す・知らせの道へ渡す | 不足 |
| 1.1 アプリの記憶（書き手へ投函する 3 つ） | `LastUsed::record`（`LastGhost`）・`emo2_boot/ghost_switch.rs` の `record_steady_memory`（`LastRunning`）・`roots/edit.rs` の `apply`（ゴーストが動いている間の `BasewareRoots`） | 結果が返らない | 不足 |
| 1.1 置き場の並び（ゴーストが居ないとき） | `roots/edit.rs` の `apply` が `save_scope` を直接呼び、`error!(roots_memory_write_failed)` | 知らせの道へ渡すだけ | 不足（小） |
| 1.2 ゴーストごとの記憶 | `LastUsed::record`（`LastBalloon`・`LastShell`）・`record_last_shell`・`record_last_balloon`・`install/desk.rs` の `remember_balloon`・`placement/persist.rs`（窓の位置）・`crates/areka-ghost/src/prop_sink.rs`（プロパティ・talk のスレッドから） | どれも投函だけ。結果は返らない | 不足 |
| 1.3 投函の失敗を拾う | `crates/areka-sylphya/src/actor.rs` の `run_actor` が `Effect::PersistSave` で `save_scope` を呼び、`persist_outcome` を持つ。返すのは `reply: Some` のときだけ（今の呼び手は全部 `None`） | 失敗を外へ渡す口 | 不足（拾う場所は在る） |
| 1.4 パスと OS の理由 | `persist/mod.rs` の `save_scope` の `error!`（`path`・`error`） | `Degraded` に中身が無い | 不足 |
| 1.5 働きを止めない | 今日どおり（縮退して続く） | — | 在る |
| 2.1〜2.5 知らせ | 告知 `alert.rs` の `AlertScene`（メッセージボックス・足さない）。ゴーストへ送れるイベントは許可表 `crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS` のみ | ベースウェアの文をバルーンに出す口・まとめ方（同じ置き場・同じ理由は 1 回） | 不足（新しく作る） |
| 2.6 §8 の登記 | `doc/COMPAT_ARCHITECTURE.md` §8 の表 | 行を 1 つ足す | 不足（小） |
| 3.1〜3.4 印 | `main.rs` の `boot_first_ghost`（起こす前に書く）・`settle_session_mark`・`session_mark_verdict`。前回の印は `halted` として起動時に読んでいる | 「残る印が在るか」を知る手（下の §4.3） | 不足 |
| 4.1 「書きました」の 4 行 | `last_used_recorded`・`last_shell_recorded`・`last_balloon_recorded`・`install_balloon_remembered`。どれも投函の直後に出る | 文の直し（または結果を見てから出す） | 不足 |
| 4.2・4.3 印の 2 つの文 | `clear_session_mark` の `session_mark_clear_degraded`・`main.rs` の `session_mark_kept` | 残る印の有無で文を分ける | 不足 |
| 4.4 記録の読み手 | 下の §4.4 の 14 ファイル | 要件 4.4 の一覧に無い読み手が在る | 制約 |
| 5.1〜5.3 インストールの語 | `install/judge.rs` の `failure_word`・`FailureWord`（3 値）。`NarError::Io` は全部 `Extraction` | 4 値目と、`IoPhase` での振り分け | 不足（小） |
| 5.2 根が無い | `install/destination.rs` の `install_into` が `IoPhase::Stage` の `NarError::Io` を作る | 5.1 の振り分けで自動で入る | 在る（形は） |
| 5.7 `install_failed` の欄 | `install/procedure.rs` の `fail_nar` が `kind`・`phase`・`word` を出す | `word` が新しい語になるだけ | 在る |
| 5.8 入れたバルーンの記憶 | `remember_balloon`（投函） | 1.3 の口で拾える | 不足（1.3 と同じ） |
| 6.4 行数 | `main.rs` 971 行（上限 1,000）・`ghost_switch.rs` 917 行・`boot_resolve.rs` 615 行・`areka-sylphya/src/actor.rs` 754 行 | 結線を別のファイルへ | 制約 |
| 7.1〜7.6 テスト | 偽の保存の口 `SharedFakeIo`（`PersistIo` の実装）が 10 本以上のテストに在る。書けない置き場は「App の置き場を普通のファイルにする」前例（`boot_resolve_tests.rs` の `session_mark_writers_warn_once_when_they_cannot_write`） | 書けない口の偽物（`commit` が `Err` を返す）・知らせの判定 | 不足（型は在る） |
| 7.7 実機 | `AREKA_PROFILE_DIR` で App の置き場を `target\` の下の「普通のファイル」へ向ければ、権限を触らずに書けない置き場が作れる | 手順 | 不足（手は在る） |
| 8 申し送り | `vendors/sample_ghost/emo2.nar` の `ghost/master/dic/install.pasta` は「理由を問わず共通トーク」3 場面 | — | 在る |

## 3. 議題 1（知らせの道）: 候補ごとの費用

### 3.1 今在る「バルーンに文を出す」口は 0 本

調べた口と、ベースウェアの文を出すのに使えるか:

| 口 | 定義点 | 使えるか |
|---|---|---|
| 告知（メッセージボックス） | `crates/areka/src/alert.rs` | 使えない（要件 2.2・09-26 の裁定） |
| `OnBoot` の Reference6・7 | `crates/areka-kanade/src/schedule/boot.rs`・`BootOrigin::Halted` | Ref6 の正典の値は `halt` だけ（要件 2.3 に触れる）。emo2 は `ref[6] == "halt"` かつ Ref7 が空でないときだけ反応し、ほかの Reference は読まない（`ghost/master/scripts/pasta/shiori/event/boot.lua` の `REG.OnBoot`） |
| `RaiseEvent`（汎用の通知の入口） | `crates/areka-kanade/src/msg.rs`・許可表 `ALLOWED_EVENT_IDS` | SHIORI のイベントを送るだけ。正典に「書けない」の器が無い |
| `script:` の選択肢 | `schedule/choice.rs`・`schedule/steady_choice_script.rs` の `begin`・`start_talk` | 利用者が選択肢を選んだときだけ動く。`start_talk`（台本 1 つを新しいトークとして始める・番号の採番と枠の差し替えを持つ）は `schedule` の中だけで使える |
| 起動の締めの添え物 | `KanadeConfig` の `first_boot_epilogue`（`schedule/boot.rs` の `to_baseware_version`）・`areka-talk` の `StartTalk.epilogue` | 起動の挨拶の末尾に付けるのは「運行のコマンド」（`EpilogueCommand`）で、文ではない。ただ「ベースウェアが起動の時点で決めた物を、起動のトークに付ける」前例にはなる |
| MCP の `sakurascript` | `crates/areka/src/mcp/sakurascript.rs` | 中身が無い（`NG:not implemented yet`）。作るのは `mcp-kanade-tools`（v0.1.0・brief だけ） |
| 台詞の切れ目の口 | `KanadeMsg::AwaitTalkGap`（`raise: Option<GapRaise>`）・`schedule/talk_gap.rs` | 切れ目まで待ち、切れ目で印のイベントを送れる。台本は流せない。呼び手は `emo2_boot/frame/switch.rs`・`emo2_boot/shell_balloon_switch.rs` |
| sakura へ直接 `TalkCommand::Start` | `areka-sakura` | kanade の番号と枠を飛ばすので、トークの完了の照合が壊れる。採れない |

### 3.2 候補 (a) ベースウェアの文を今のゴーストのバルーンに出す

作り方は 3 通りに分かれる:

- **(a-1) 「切れ目で台本を流す」を kanade に足す**: `AwaitTalkGap` の `raise` と並べて「切れ目でこの台本を流す」を持てるようにし（`crates/areka-kanade/src/change.rs` に型 1 つ・`talk_gap.rs` の `begin` で `start_talk` を呼ぶ）、areka の側は「知らせの机」（新しいファイル）から送って結果を待つ。切れ目で流すので、ゴーストのトークを途中で切らない。待っている間に次のトークが割り込む競りも、kanade の中で 1 回で決まるので起きない。
  - 触る所: kanade の `msg.rs`（930 行）・`change.rs`（175 行）・`schedule/talk_gap.rs`（261 行）・`schedule/steady_choice_script.rs`（`start_talk` の公開の範囲）・`schedule/mod.rs`（969 行・振り分けを触るなら）。areka の側は新しいファイル 1 つと、ゴーストを起こした所からの呼び出し。
  - 規模: 3〜5 タスク増し。kanade のファイルは上限の近く（`schedule/mod.rs` 969・`steady.rs` 957・`msg.rs` 930・`actor.rs` 901）＝新しいファイルへ置く前提。
  - ロードマップ: kanade の直列の列（`mcp-kanade-tools` が先頭）にこの spec が割り込む形になる。C8 のほかの 6 本は kanade に触らないので、ウェーブの重なりは 0。`mcp-kanade-tools` の brief は「外からの台本」の変種を作る予定なので、こちらが先に作った口を向こうが使い回す（または向こうが作り直す）申し送りが要る。
- **(a-2) `mcp-kanade-tools` を待つ**: v0.0.3 の spec が v0.1.0 の spec を待つ逆向きになる（ロードマップの「版の段」が禁じる）。採りにくい。
- **(a-3) 起動の時点の失敗だけを、起動のトークに付ける**: 起動の前に分かる失敗（印を書けない・前回の印が残っている）は、`first_boot_epilogue` と同じ形で `KanadeConfig` に載せて起動の挨拶の後に流せる。ただ、起動の後の失敗（最後に使ったもの・置き場の並び・窓の位置など）は拾えない。単独では要件 1・2 を満たさない。

(a) に共通する注意:

- **線の内か外か（討議で必ず確かめる）**: 完了 `ghost-install` 要件 7「ゴーストが応えなかったとき、代わりの画面や台詞を出さない」と、09-26 の裁定「失敗は既定ゴースト（確実に応えるゴースト）の台詞で、正典の器を通して伝える」。(a) は「ゴーストの口でベースウェアの文を話させる」なので、どちらの文言にも正面からは当たらないが、どちらの精神（利用者との接点はゴーストの台詞・ベースウェアは代わりに喋らない）とも緊張がある。
- **`OnTranslate` に通すか**: SHIORI の台詞は再生の前に `OnTranslate` を通る（`schedule/translate.rs`）。ベースウェアの文を通すと、ゴーストの辞書が文を書き換えうる。通さないと、ゴーストの口調の変換が効かない。`mcp-kanade-tools` も同じ問いを持っている（同 brief の議題）。
- **パスの書き方**: さくらスクリプトでは `\` がタグの始まりなので、パス `C:\Program Files\areka\profile\areka` はそのままでは壊れる（`\P` などがタグに読まれる）。`\\` へ直す必要がある。`%` も `areka-sakura` の `sysvar.rs`（`%username` などの展開）に読まれうる。
- **どの声で・どのスコープで**: `\0`（さくら側）に出すのが素直だが、ゴーストの名前を名乗らせるか、ベースウェアの文だと分かる形（例: 文頭に「areka より:」）にするかは文面の問題。
- **何回出すか**: 要件 2.4 のまとめ方は、areka の側の机で「置き場のフォルダ＋OS の理由の種類」を鍵に 1 回にできる（kanade は知らなくてよい）。
- **このとき要件 2.7（ゴーストが応えない）は起きない**（ゴーストの返事を要らない）。

### 3.3 候補 (b) 既定のゴーストの `OnBoot` の Reference に載せる

- 費用は小さい（`BootOrigin` に値を足す・kanade の `boot.rs`）。
- ただし Ref6 の正典の値は `halt` だけ。「書けない」を載せると要件 2.3 に触れる。Ref8 以降は正典に無い。
- 今の emo2 は載せても喋らない（§3.1）。辞書の側の直し（議題 4）が来るまで、利用者に届く物は 0。
- 起動の後の失敗は、次の起動まで届かない（次の起動も書けなければ、そのたびに届くが、話すのは次の起動）。

### 3.4 候補 (c) 知らせない（記録と MCP の `get_log` だけ）

- 費用はほぼ 0（記録の文と欄の直しだけ）。brief の目指す姿 1 に反する。

### 3.5 要件の外だが比べのために挙げる物

- **メニューに 1 行出す**（右クリックのメニューに「記憶を書けません: <フォルダ>」を出す）: ゴーストの台詞ではないが、メッセージボックスでもない。自分から伝えない（利用者がメニューを開くまで見えない）。(a)・(b) の補いにはなる。
- **OS の通知（トースト）**: 要件 2.2（ゴーストの台詞の道）と製品の考え方に合わない。
- **areka 独自のイベント**: 要件 2.3（新しいイベント 0）で採れない。

## 4. 書き込みの失敗を拾う口（要件 1）

### 4.1 拾う場所の候補

| 案 | やり方 | 触る所 | 良い所 | 気になる所 |
|---|---|---|---|---|
| ① アクターに「失敗の受け口」 | `areka-sylphya` の `run_actor` が `Degraded` を見たら、登録された受け口（`RuntimeCommandSink` と同じ形のトレイト、または `mpsc` の送り手）へ渡す。UI の側は受け取り口を毎フレーム覗く（`mcp/mod.rs` の `later`／`poll_later` と同じ型） | `actor.rs`（754 行）・受け口の登録の口 | 呼び手 9 か所を直さない。今後足される書き込みも自動で拾う | 登録の口の置き場（下の 4.2） |
| ② 呼び手が返事つきで投函 | `SylphyaMsg::PersistPut` の `reply: Some` を使う（中身は在る）。各呼び手が返事を覗く | 呼び手 9 か所（`prop_sink.rs` は talk のスレッドで World を持たない） | アクターを触らない | 呼び手ごとの直し・`prop_sink` は覗く場所が無い・新しい呼び手が漏れうる |
| ③ UI スレッドの直接の書き込み | `write_session_mark`・`write_switch_drop`・`clear_session_mark`・`roots/edit.rs`（ゴーストが居ないとき）が結果を返す | `boot_resolve.rs`・`roots/edit.rs` | 小さい | ①か②と組み合わせが要る |

①＋③ の組で、要件 1.1・1.2・1.3・5.8 の書き込みが全部 1 つの道へ集まる。

### 4.2 受け口の登録の場所と、ウェーブの約束

- 本番でゴーストの記憶の書き手を起こすのは `crates/areka-ghost/src/sylphya_wiring.rs` の `spawn_ghost_sylphya` 1 か所。`SylphyaInit` に欄を足すと、`SylphyaInit {` を組む 32 か所（多くはテスト）が全部直る。
- `user-data-root` は議題 7 を選ぶと `sylphya_wiring.rs`（`profile_areka_root`）・`crates/areka-ghost/src/runtime.rs`・`placement/persist.rs` に触る。C8 の約束は「触るソースファイルの重なり 0」なので、`sylphya_wiring.rs` は避けるのが安全。
- 避け方: `SylphyaInit` を変えず、`SylphyaPublisher` に「失敗の受け口を登録する」投函（新しい `SylphyaMsg` の変種）を足し、areka の側がゴーストを起こした直後に登録する。登録はゴーストの実行系を起こす所（`main.rs` の `boot_first_ghost` と `ghost_switch.rs` の `boot_into` の近く）から、新しいファイルの関数 1 つを呼ぶ形にできる。
- App スコープだけの書き手（`crates/areka/src/shiori_host.rs` の `spawn_app_sylphya_sink`）は記憶の投函を持たない（`persist_put` の呼び手に無い）ので、受け口は要らない見込み。

### 4.3 パスと理由を運ぶ形（要件 1.4）

- 案 A: `PersistOutcome::Degraded` に中身（置き場のパス・OS の理由の文・理由の種類）を持たせる。今 `Copy`・`Eq` で、`== PersistOutcome::Degraded` と比べる所が 15 か所（`roots/edit.rs`・`boot_resolve.rs`・テスト）。`Clone` に落とし、比べる所を `matches!` へ直す。
- 案 B: `PersistOutcome` はそのままにし、失敗の中身だけを別の型（例「保存の失敗」）で、受け口と `save_scope` の詳しい版から返す。既存の比べには触らない。
- 「保存先が無い」（root 不在）の `Degraded` は、パスを持たない。App の置き場はいつも在るので、起きるのはゴーストの置き場が組めないときだけ。

### 4.4 印の文（要件 3.2・3.3・4.2・4.3）の材料

- 「記憶に残る印が在るか」は、消せなかった直後に `read_session_mark`（`boot_resolve.rs`）で読み直せば分かる（読めなければ「分からない」＝予告しない）。読み直しは「次の起動で本当に起きること」と一致する。
- もう 1 つの手は「この回に印を書けたか」を覚えておくこと。ただ、前回の印が消せずに残っている場合（要件 3.4）や、定常での印の投函だけが落ちた場合に、覚えた旗と実際の記憶がずれる。
- 要件 3.4（前回の印が残り、書き替えも消すこともできない）は、起動の時点で分かる（`halted` が在り、かつ `write_session_mark` が失敗した）。
- `session_mark_kept` の文は `main.rs` に在る。分ける枝を `main.rs` に書くと行が増えるので、記録を出す関数を `boot_resolve.rs` か新しいファイルへ移し、`main.rs` は呼び出しだけにできる。

### 4.5 記録の読み手（要件 4.4 の一覧との違い）

6 つの記録の名前（`last_used_recorded`・`last_shell_recorded`・`last_balloon_recorded`・`install_balloon_remembered`・`session_mark_clear_degraded`・`session_mark_kept`）を読むのは 14 ファイル（`.kiro` と `vendors` を除く）:

- 本体: `crates/areka/src/boot_resolve.rs`・`main.rs`・`install/desk.rs`
- テスト: `boot_resolve_tests.rs`・`main_session_mark_tests.rs`・`main_session_mark_fallback_tests.rs`・`install/desk_record_tests.rs`・`emo2_boot/frame/switch_finish_tests.rs`・`shell_balloon_switch_session_lap_tests.rs`・`shell_balloon_switch_session_balloon_tests.rs`・`shell_balloon_switch_session_abort_tests.rs`・`session_end_tests.rs`・`session_end_deadline_tests.rs`
- 文書: `doc/roadmap-overview-2026-10-16.html`（生成物）

要件 4.4 の一覧（`roots/edit_tests.rs`・`install/procedure_branch_tests.rs`・`install/desk_overwrite_tests.rs` ほか）には、`shell_balloon_switch_session_*_tests.rs`・`session_end*_tests.rs`・`install/desk_record_tests.rs`・`emo2_boot/frame/switch_finish_tests.rs` が無い。**これらのテストは「書きました」の行を「投函が起きた印」として数えている**（例: `switch_finish_tests.rs` は `last_shell_recorded` の数を、`shell_balloon_switch_session_lap_tests.rs` はその並びを見る）。名前を変えるか、出す時点を「書けた後」に動かすと、これらの判定の意味が変わる。

### 4.6 `main.rs` の行数

- `main.rs` は 971 行。足すもの: 起動の前の印の結果を知らせの机へ渡す（1〜3 行）・受け口の登録の呼び出し（1〜2 行）・新しいファイルの `mod` の 1 行・`session_mark_kept` の文の分け（関数を外へ移せば ±0）。合わせて 10 行前後で、上限の内に収まる見込み。
- 余裕を作るなら、`session_mark_verdict`・`settle_session_mark`（約 80 行）を別のファイルへ移せる（判定の中身は変えない＝要件 3.6 に触れない）。ただし `main_session_mark_tests.rs`（941 行）が `super::session_mark_verdict` などを直接使っているので、テストの `use` も動く。

## 5. 議題 2（インストールの語）の確かめ

### 5.1 振り分けは `areka-nar` に触らずにできる

- `NarError::Io` は `phase: IoPhase` を持つ（`crates/areka-nar/src/error.rs`）。`Read` は `.nar` を丸ごと読む所（`lib.rs` の `read_archive_bytes`）だけ。`Stage` は作業フォルダを作る・組み上げる（`WorkArea::create`・`install.rs` の `stage_placement`）、`Commit`・`Rollback` は確定と巻き戻し。
- 要件 5.1・5.2・5.3 の振り分けは「`Read` → `extraction`、それ以外の段 → 新しい語」で書ける。根が無いとき（`install_into`）は `IoPhase::Stage` で作られているので、自動で新しい語へ入る。
- 注意 1 つ: `Stage` の中には、**今在る宛先の中身を読んで作業フォルダへ写す**所（`copy_existing` の `read_dir`）がある。宛先のファイルを読めない失敗も新しい語になる。要件 5.1 の 3 つ（作業フォルダを作れない・入れる先へ書けない・確定や巻き戻しで書けない）のどれとも少しずれるが、「入れる先が触れない」という意味では近い。
- 直るテスト: `install/judge_tests.rs` の `every_io_phase_is_extraction`・`failure_words_are_the_three_canonical_spellings`、`install/procedure_branch_tests.rs` の `install_failure_sends_extraction_and_logs_each_survivor_with_seven_days`。§8 の「インストールの失敗理由の語の写し」の行。

### 5.2 正典の語の出どころ

- `fileio` は ukadoc に 3 か所出る: 「ネットワーク失敗理由」の `fileio`（「容量不足による更新対象への書き込み失敗。」）と、`OnExecuteHTTPComplete` の Reference4「取得結果コード。(timeout|fileio|200|204|403|404)等。」（https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnExecuteHTTPComplete ）。インストールの失敗理由の一覧には無い。
- areka のネットワーク更新は、作業フォルダ・確定の書き込み・巻き戻しの失敗をすでに `fileio` で送っている（`crates/areka/src/update/refs.rs` の `failure_word` の `WorkArea`・`CommitWrite`・`RollbackFailed` など）。

### 5.3 検体のゴーストの辞書は、知らない語でどう動くか

`vendors/sample_ghost/` の 5 体の `OnInstallFailure` を読んだ:

| ゴースト | 辞書の形 | Reference0 の使い方 | 知らない語（例 `fileio`）を受けたとき |
|---|---|---|---|
| emo2（pasta） | `ghost/master/dic/install.pasta` | 見ない（3 場面から 1 つ） | 今日と同じ（3 回に 1 回「ファイルが壊れてるのかもね。」） |
| claudia（YAYA） | `dic/normal/yaya_etc.dic` | 語をそのまま「理由：…」と出す | 「理由：fileio」と出る |
| konnoyayame（YAYA） | `dic/normal/yaya_etc.dic` | 語をそのまま出す（2 通り） | 「理由：fileio。あーあ。」など |
| emily4（YAYA） | `aya_etc.dic` | `artificial`・`unsupported`・`extraction` で分け、ほかは語をそのまま出す | 「駄目みたいだね。fileioだってさ。」（今日の `extraction` は「ファイルが壊れてると思うから、もう一度取ってきてみてね。」＝誤った案内） |
| R_POST_and_KOMAINU（里々） | `dic02_Event.txt` | 語をそのまま「原因は「…」だとさ。」 | 「原因は「fileio」だとさ。」 |

**どの辞書も知らない語で壊れない**（台詞が出なくなる辞書は 0）。emo2 は今日と同じで悪くならず、emily4 は誤った「壊れている」の案内が消える。YAYA・里々の 3 体は語をそのまま見せるので、areka の新しい語（候補 (c)）を使うと、その綴りが利用者の目に入る。

## 6. 実装の進め方の候補（全体）

### 案 A: 今在る物を広げる

- `boot_resolve.rs`（UI スレッドの書き込みが結果を返す）・`areka-sylphya` の `actor.rs`（失敗を外へ渡す）・`install/judge.rs`（4 値目）・記録の文の直し。知らせの道は (b) か (c)。
- ✅ 小さい（7〜9 タスク）。kanade に触らない。
- ❌ (b) は正典の意味から外れ、今の emo2 は喋らない。(c) は brief の目指す姿 1 に届かない。

### 案 B: 知らせの机を新しく作る

- 新しいファイル（例 `crates/areka/src/write_notice.rs`）に、失敗の受け取り・まとめ（鍵＝置き場のフォルダ＋理由の種類）・文面の組み立て（パスの `\` を直す）・送る時点の判断を置く。kanade には「切れ目で台本を流す」口を足す（(a-1)）。
- ✅ 要件 2 を正典の意味を曲げずに満たす。どのゴーストでも同じに伝わる。`mcp-kanade-tools` が使い回せる口が先にできる。
- ❌ kanade の上限の近いファイルを触る。`OnTranslate`・割り込みの扱いなど決めることが増える。kanade の直列の列へ割り込む。

### 案 C: 組み合わせ（段に分ける）

- 1 段目: 拾う口（§4）・記録の文・インストールの語（議題 2）・印の扱い（議題 3）を先に作る（案 A の部分）。
- 2 段目: 知らせの机と kanade の口（案 B の部分）を同じ spec の後半のタスクで作る。
- ✅ 討議で (a) の線が「外」と決まっても、1 段目はそのまま使える。タスクの順で危うい所を後ろへ置ける。
- ❌ タスクの数が上限の近く（下の §7）。

## 7. 規模と危うさ

- **規模**: 案 A＝S〜M（7〜9 タスク）／案 B・C＝M〜L（11〜15 タスク）。brief の見立て（7〜11）は「伝える道を今在るものから選ぶ場合」で、(a) を採ると kanade の口の分（3〜5 タスク）が増える。上限 20 の内。
- **危うさ**: 案 A＝低（今の形を広げるだけ・前例が在る）／案 B・C＝中〜高（kanade に新しい境界〔ベースウェアの台本の入口〕を引く・`OnTranslate` とトークの割り込みの決まりが要る・上限の近いファイル・`mcp-kanade-tools` と設計がかぶる）。
- (a) を採るなら、要件定義の後の設計は「先例の無い境界を新しく引く」に当たりうる（Fable を勧める基準の 1 つ目）。決めるのは開発者。

## 8. 討議へ渡す項目（答えで作業が変わるものだけ）

1. **議題 1 の (a) は線の内か外か**（`ghost-install` 要件 7 と 09-26 の裁定）。外なら (b)・(c)・メニューの 1 行のどれにするか。
2. **(a) を採るなら作り方**: (a-1) この spec が kanade に「切れ目で台本を流す」口を作る（kanade の列への割り込み・`mcp-kanade-tools` への申し送り）／(a-3) 起動の時点の失敗だけ起動のトークに付け、起動の後の失敗は次の起動まで持ち越す、のどちらか。(a-2)（`mcp-kanade-tools` を待つ）は版の向きが逆になる。
3. **(a) の台本を `OnTranslate` に通すか**（`mcp-kanade-tools` の同じ問いと揃えるか）。
4. **(a) の文面と見せ方**: ベースウェアの文だと分かる形にするか・どのスコープで出すか・パスを全部出すか（長いパス・`\` と `%` の直し）。
5. **失敗を拾う口の形**: アクターの受け口（§4.1 ①）か、呼び手ごとの返事（②）か。①なら登録を `SylphyaInit` の欄でなく投函で行い、`sylphya_wiring.rs` に触らない形にするか（`user-data-root` との約束）。
6. **`Degraded` に中身を持たせるか、別の型で返すか**（§4.3 の案 A・B）。
7. **「書きました」の 4 行の直し方**: 名前を残して文を「書くように頼みました」に改め、結果は受け口の側の 1 行（成功は出さない／失敗だけ出す）で出すか、名前ごと変えるか。後者なら要件 4.4 の一覧に無い 7 本のテスト（§4.5）も直り、それらが「投函の印」として数えている意味を置き換える必要がある。
8. **印の文の材料**: 消せなかった直後に記憶を読み直して「残る印が在るか」を決めるか、この回に書けたかを旗で覚えるか（§4.4）。
9. **インストールの語の振り分けの線**: 「`IoPhase::Read` だけ `extraction`・ほかは新しい語」でよいか（`Stage` の中の「今の宛先を読めない」も新しい語になる・§5.1）。
10. **ロードマップの更新**: (a-1) を採るなら kanade の直列の列の並びを改める（この spec を `mcp-kanade-tools` の前へ）。

## 9. 設計へ持ち越す調べ

- さくらスクリプトでパスを安全に出す書き方（`\` と `%`・`areka-sakura` の `sysvar.rs` の展開がどこまで効くか）。
- (a-1) で、切れ目を待っている間にゴーストが切り替わった・終わったときの扱い（`TalkGap::Left` の理由と同じでよいか）。
- 受け口に渡すのはアクターのスレッドから（`Send` が要る）。UI の側の受け取りを毎フレーム覗くか、既存の覗く口（`mcp::poll_later` のような）に相乗りするか。
- 窓の位置の記憶は動かすたびに書くので、失敗が続く。まとめの鍵と、記録（`error!`）の数をどう抑えるか（記録は今日どおり毎回出るが、知らせは 1 回）。
- 実機の確かめ（要件 7.7）: `AREKA_PROFILE_DIR` を `target\` の下の「普通のファイル」に向ける手（前例 `boot_resolve_tests.rs`）が、OS の理由の文としては「アクセスが拒否されました」でなく別の文になる。理由の文を見せる確かめなら、それで足りるかを決める。
