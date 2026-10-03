# ギャップ分析: areka-P0-translate-pipeline

- 実施日: 2026-10-03（ブランチ `claude/areka-p0-translate-pipeline-2cf68b`・main `d4f9e93d` の直後）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md`）
- 方法: コードの実物を読んで、要件ごとに「今あるもの／足りないもの／制約」を突き合わせた。決定はせず、選択肢と論点を並べる。

---

## 1. 分析の要約

- **台詞の出口は今も 1 つ**。SHIORI の台詞から再生が始まるのは kanade の運行表が返す `Action::StartTalk` だけで、殻（`crates/areka-kanade/src/actor.rs` の `execute_actions`→`send_talk_command`）がそれを再生側へ送る。kanade の外で `StartTalk` を作る所は 0 か所（`areka-ghost` の `dispatcher.rs` は受けて渡すだけ）。SHIORI の台詞で `StartTalk` を作る腕は **8 か所**（boot 1・close 1・change 2・steady 4）、areka が自分で作る台詞は **1 か所**（boot の「起動の記録だけの台詞」）。
- **「一度に 1 往復」は構造で守られている**。殻の `drive` は 1 通のメッセージの処理の中で「行動の一括を全部実行 → 最後の SHIORI の応答だけを入れ直す」を行動が尽きるまで繰り返す。その間、殻は受け箱を読まない。よって `OnTranslate` の往復を同じ `drive` の中に置けば、翻訳待ちの間に届くマウス・毎秒・終了・切替・外からの依頼は**受け箱に積まれたまま**で、翻訳の後の再生が始まってから今日の規則で処理される（要件 5.3・5.4 は新しい仕組みなしで成り立つ）。
- **足りないもの**: ⑴ 翻訳の待ちを表す状態と、その応答を受ける腕 ⑵ 元のイベントの ID と Reference を応答の後まで覚えておく所（今は送った時点で捨てている・選択肢の任意名は記録上 `"OnChoiceEvent"` に丸められる）⑶ 文字列のままで行う展開（今は字句解析の後の compile でしか展開しない）と、その値の源（sylphya の写し）を kanade へ届ける道 ⑷ `OnTranslate` の組み立てと許可表への追加 ⑸ Reference1 の「欠番」を線上で表す方法（今の線の形は 0 から詰めて並べる形しか持たない）。
- **一番大きい危険はテストの側**。`crates/areka/src/emo2_boot/spine.rs` の `ScriptedShioriBackend::get` は台本に無い GET で panic する。台詞を返す台本を持つテストはすべて `OnTranslate` で落ちる。このファイルは要件の「`emo2_boot/` に触れない」の内側にある（論点 12）。記録した呼び出しの列を丸ごと比べるテストも多い（論点 13）。
- **候補の形**: 翻訳の待ちを「相（`Phase`）を 1 つ足す」形（brief の案 ⒜）と「相は今日のまま進め、`State` に翻訳の帳簿を足して `StartTalk` と後続の行動を後ろへ回す」形（本分析で見つけた案）と「殻の中で同期に済ませる」形（brief の案 ⒝）の 3 つ。台詞の切れ目の見張り（`talk_gap.rs`）と Status の導き方への影響が分かれ目（§4）。

---

## 2. 今あるもの（コードの実物）

### 2.1 SHIORI との往復と台詞の出口

| 何か | 場所 | 中身 |
|---|---|---|
| 駆動の繰り返し | `crates/areka-kanade/src/actor.rs` の `drive` | `step` の行動を全部実行し、最後の往復の応答だけを `Input::ShioriReply { outcome, origin }` で入れ直す。往復が無い一括で処理を終える。 |
| SHIORI へ出る唯一の点 | 同 `round_trip_request` | 許可表で ID を確かめ、`trace!(shiori_request)` を残して送る。エラー応答（`ShioriFailure::Shiori`）は GET なら `NoContent`・NOTIFY なら `Notified` へ写し `warn!(shiori_error_response)` を 1 件残す（shiori-fault-notice の決まり）。他の失敗（`Handshake`・`Timeout`・`Ipc`・`Internal`）は `Failed` のまま。 |
| 再生への唯一の送り口 | 同 `send_talk_command` | `TalkCommand::Start` 等を 1 本の通路へ送る。 |
| 応答の出所 | 同 `execute_actions` | `origin: &'static str` は送った ID の写し。選択肢の任意名（`EventId::Choice(String)`）は `"OnChoiceEvent"` に丸める。**Reference は保持しない**。 |
| 例外の 1 件 | 同モジュール冒頭の doc（「唯一の例外は `ForceQuit`」） | `[OnClose NOTIFY, Unload]` の一括だけが 1 つの一括に 2 往復を持つ。 |
| 外からの依頼の返事 | 同 `drive` の戻り値 `first_reply`・`raise_outcome_of` | **最初の一括**の往復の結果を返事にする。翻訳の往復は 2 つ目以降の一括になるので、返事は元のイベントの結果のまま（今日と同じ）。 |
| リソース照会 | `crates/areka-kanade/src/actor_resources.rs` の `answer` | `step` を通らず殻で往復する。メッセージの間でしか動かないので翻訳と同時にはならない。 |

### 2.2 台詞から `StartTalk` を作る腕（9 か所）

| 経路（要件の 5 種類） | 腕 | 場所 | 一括の中身 |
|---|---|---|---|
| 1 起動の挨拶（`OnFirstBoot`・`OnBoot`・`OnGhostChanged`・更新の起動の知らせ・起動種別の応答） | `to_baseware_version` の `Some(script)` | `crates/areka-kanade/src/schedule/boot.rs` | `[StartTalk{script, epilogue}, basewareversion NOTIFY]`。相は `BootVersion{talk: Some}`。NOTIFY の Status は相を変えた**後**に撮る（`talking`）。 |
| （areka が作る台詞） | `to_baseware_version` の epilogue だけの腕 | 同上 | `script` が空・`epilogue` だけの `StartTalk`。翻訳しない（要件 1.3）。 |
| 2 終了の別れ `OnClose` | `on_close_pending` の `Value` | `crates/areka-kanade/src/schedule/close.rs` | `[StartTalk]`。相は `CloseTalkWait{talk_id, deadline}`（期限は応答の時点の `last_now` から）。 |
| 3 切替の送り出し（`OnGhostChanging`・続く `OnClose`） | `on_reply_wait` の `Value` | `crates/areka-kanade/src/schedule/change.rs` | `[StartTalk]`。`OnGhostChanging` のときだけ `state.change.script` に台詞を控える（→ 次のゴーストの `OnGhostChanged` の Ref1）。 |
| 3 取りやめた切替の `OnGhostChanging` | `on_yielded_reply` の `Value` | 同上 | `[StartTalk]`。`ActiveTalk.script` に台詞を控える。 |
| 4 定常（毎秒・マウス・外からの依頼）・空き | `on_reply` の `Steady{talk: None}` の `Value` | `crates/areka-kanade/src/schedule/steady.rs` | `[StartTalk]`。`ActiveTalk.script` に控える。 |
| 4 定常・再生中の置き換え | `on_reply` の `Steady{talk: Some}` の置き換えの腕（`events::value_replaces_active_talk`） | 同上 | `[StartTalk]`。`OnSecondChange` の Value は捨てる（翻訳の対象にならない）。 |
| 5 選択の連鎖 | `on_cascade_reply` の `Value` | 同上 | `[ResolveChoice{old}, StartTalk(new)]` を**この順**で 1 つの一括に置く。 |
| 5 `OnChoiceTimeout` | `on_timeout_reply` の `Value` | 同上 | `[StartTalk]`。 |

`StartTalk` は `crates/areka-talk/src/lib.rs` の `{ talk_id, script, epilogue }`。起動の記録（epilogue）は**文字列ではなく別の欄**なので、`script` だけを翻訳すれば要件 1.4 は形の上で成り立つ。

### 2.3 控える台詞

| 何に使うか | 控える所 | 読む所 |
|---|---|---|
| `OnChoiceTimeout` の Reference0 | `ActiveTalk.script`（`crates/areka-kanade/src/schedule/mod.rs`）。上の各腕が `script.clone()` で入れる | `steady.rs` の `fire_choice_timeout_if_due` → `events::on_choice_timeout` |
| 次のゴーストの `OnGhostChanged` の Reference1 | `ChangeState.script`（`change.rs` の `on_reply_wait`） | `actor.rs` の `handoff_of` → 停止通知 `KanadeStopped.handoff` → `crates/areka/src/emo2_boot/ghost_switch.rs` の `switch_to` が `BootOrigin::ChangedFrom{script}` へ → `events::on_ghost_changed` |
| 取りやめた切替を定常のトークとして流すとき | `change.rs` の `yield_to_close`（`change.script.unwrap_or_default()` を `ActiveTalk.script` へ） | 同上の `fire_choice_timeout_if_due` |

どれも kanade の中で台詞を入れる点が決まっているので、**控える値を翻訳の後に替えるのは kanade の中だけで済む**（`emo2_boot/` には触れずに済む）。

### 2.4 環境変数の展開

| 何か | 場所 | 中身 |
|---|---|---|
| 字句の規則 | `crates/areka-parsers/src/sakura/lexer.rs` の `lex`・`scan_sysvar`・`scan_tag`・`scan_bracket_args` | `%` の後ろの英数字と `_` を**貪欲に**読む（`%usernameさん` は `username`＋「さん」、`%usernameabc` は名前 `usernameabc`）。`\%` は文字の `%`、`\\` は文字の `\`。タグの引数（`[...]` の中）の `%` は展開の対象にならない。`Token` は位置を持たず、`lex` も `Token` も `pub(crate)`。 |
| 展開 | `crates/areka-sakura/src/compile.rs` の `compile` の `Instruction::SystemVar` の腕 → `crates/areka-sakura/src/sysvar.rs` の `resolve_system_var` | 写しに値があればその値（**名前は `username` に限らない**）、無くて `username` なら `DEFAULT_USERNAME`（「ユーザーさん」）、それ以外は `%名前` のまま。展開した値は文字として扱われ、字句解析をもう一度通らない。 |
| 値の源 | `crates/areka-ghost/src/dispatcher.rs` の `on_start`（`(self.system_vars)()` をトークの開始ごとに 1 回）・`crates/areka-ghost/src/sylphya_wiring.rs` の `from_sylphya_provider`（`SylphyaReader::talk_snapshot`） | 写しには `username`（起動の照会の結果）に加え、descript から `selfname`・`selfname2`・`keroname` も載る（同ファイルの `derive_flat_statics`）。 |
| kanade の依存 | `crates/areka-kanade/Cargo.toml` | `areka-actor`・`areka-talk`・`shiori-host32-host` だけ。**`areka-sakura`・`areka-parsers`・`areka-sylphya` には依存しない**。`areka-ghost` が kanade と sakura の両方に依存する。 |
| kanade へ外から関数を渡す前例 | `actor.rs` の `spawn_kanade` の `resource_sink`（`ResourceSink`＝注入の関数）・`KanadeConfig.first_boot_epilogue`（中身を解釈しない運び） | 「kanade は sylphya へ依存しない疎結合の継ぎ目」。steering `structure.md` も kanade を「純粋な運行表＋殻＋境界の 3 層・sylphya へ依存しない」、`areka-ghost` を「結線の層」、`areka-talk`（`StartTalk` の正本）を「依存ゼロの契約」と定めている。 |

### 2.5 許可表・失敗の分け方・状態の表し方

| 何か | 場所 | 中身 |
|---|---|---|
| 許可表 | `crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS` | 45 語（各行に ukadoc の URL の注記）。数は `schedule/events_change_tests.rs` が `assert_eq!(ALLOWED_EVENT_IDS.len(), 45)` で固定。`OnTranslate` は無い。 |
| `OnTranslate` の正典の定義 | `doc/shiori/fragments/events/28.other.toml` の `[entry."OnTranslate"]` | Ref0〜3 は登記済み・Rust 側の消費者は 0。 |
| 網羅の台帳 | `doc/ukadoc-coverage/ledger/shiori.toml` の `ukadoc:list_shiori_event:OnTranslate:1` | `status = "absent"`。`crates/ukadoc-survey` の検査は「実装済みなのにソースに URL が無い」（`ImplementedWithoutEvidence`）を見るので、台帳を `absent` のままにする限り検査は赤にならない。 |
| 失敗の分け方 | `actor.rs` の `round_trip_request`（エラー応答は 204 へ写す）・`schedule/mod.rs` の `on_shiori_reply`（応答待ちの相で `Failed` → `to_unloading_fault`） | 選択の往復（`Cascading`・`TimeoutInFlight`）だけは先の腕で `steady.rs` へ渡し、失敗を 204 と同じに扱う（`choice_shiori_failed_as_204`）。 |
| 応答を待つ相 | `schedule/mod.rs` の `awaits_reply` | `BootInit`・`BootType`・`BootMain`・`BootVersion`・`Steady`・`ClosePending`・`ChangePending`・`ChangeClosePending`。`CloseTalkWait`・`ChangeTalkWait`・`ChangeCloseTalkWait` は応答を待たない（届けば `unexpected_reply` で捨てる）。 |
| Status の導き方 | `schedule/mod.rs` の `State::snapshot`・`talk_active_of`・`crates/areka-kanade/src/status.rs` の `ExecutionStatus::derive` | `talking` は相が `Steady{Some}` か `BootVersion{Some}` のとき。`SecurityLevel: local` は線の側（`crates/shiori-host32-host/src/shiori3.rs` の `build_request`）が全要求に付ける。 |
| 状態の帳簿の前例 | `schedule/mod.rs` の `State`（`pending_close`・`choice`・`pending_change`・`talk_gap`） | 相の外に置く帳簿の型がある。 |
| 台詞の切れ目の見張り | `crates/areka-kanade/src/schedule/talk_gap.rs` の `marked_reply`・`observe`・`decide` | 印のイベント（`OnShellChanging`）の応答の `step` の**直後**に、相から今のトークを読んで「印の台詞が始まったか」を決める。相が `Steady` でなければ即座に「終了中」と決める。 |

### 2.6 線の形（Reference の並び）

- `ShioriCall` の `references: Vec<String>`（`crates/areka-kanade/src/msg.rs`）→ `ShioriBackend::get(id, references: &[String], status)`（実装は 15 余り）→ `crates/shiori-host32-host/src/shiori3.rs` の `build_request` が `Reference0`・`Reference1`… を**0 から詰めて**書く。途中の番号を抜く形は無い。
- 前例: `events::on_boot` の「前回落ちた」では Ref1〜5 を**空文字**で埋めて番号を保つ（「番号を詰めない」）。

### 2.7 テストの資産と影響

- kanade の運行表は純粋な関数（`step`）で、`schedule/*_tests.rs` が状態と行動を直接確かめる。殻のテストは `actor_*_tests.rs`、結合のテストは `crates/areka-kanade/tests/kanade/`（偽の SHIORI は `common/common_mock_shiori.rs`・応答の表は `common/common_fixture.rs` の `respond`・止まる偽の SHIORI もある）。DLL なしの決定論テストの道具は揃っている。
- **偽の SHIORI が台本に無い GET で panic する**: `crates/areka/src/emo2_boot/spine.rs` の `ScriptedShioriBackend::get`（`username`・`homeurl` だけ既定の 204 を補う）、`crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の同名の型も同じ。前者を使うテストファイルは 27 本（`crates/areka/src/` の `ghost_session_switch_*_tests.rs`・`shell_balloon_switch_session_*_tests.rs`・`session_end_deadline_tests.rs` など、`emo2_boot/` の外にもある）。
- 記録した呼び出しの列を比べるテスト（`common_recording.rs`・`spine_conformance_*`）は、台詞を返すたびに `OnTranslate` が 1 件混ざるので期待の列がずれる。
- `crates/areka-kanade/tests/kanade/steady_test.rs` は「送った ID がすべて許可表にある」を確かめる（`OnTranslate` を許可表に足せば通る）。

### 2.8 ファイルの長さ（1,000 行未満の決まり）

| ファイル | 行数 | 見立て |
|---|---|---|
| `crates/areka-kanade/src/schedule/steady.rs` | 929 | 腕を書き換えるなら先に分ける（brief の申し送りどおり）。 |
| `crates/areka-kanade/src/schedule/mod.rs` | 908 | 相か帳簿を足すと 930〜960 の見込み。相を足す形では `phase_label`・`awaits_reply`・`current_talk_id`・`dispatch_phase` の腕が増えるので上限に近づく。 |
| `crates/areka-kanade/src/msg.rs` | 902 | 注入の関数を `KanadeConfig` に足すと増える。 |
| `crates/areka-kanade/src/actor.rs` | 725 | 展開の実行と `OnTranslate` の記録を足しても余裕あり。 |
| `crates/areka-kanade/src/schedule/events.rs` | 638 | `on_translate` の組み立てを足しても余裕あり。 |
| `crates/areka-kanade/src/schedule/close.rs`・`change.rs`・`boot.rs` | 647・425・345 | 余裕あり。 |
| `crates/areka-parsers/src/sakura/lexer.rs` | 362 | 展開位置を返す関数を足しても余裕あり。 |

### 2.9 COMPAT §8

- `doc/COMPAT_ARCHITECTURE.md` の「## 8. 沈黙ルール対応表（正典沈黙箇所の areka 裁量記録）」（表の形・列は「項目｜裁量｜根拠｜出典 spec」）。
- **既にある行と食い違う**: 「**`OnGhostChanged` の Ref1 に何を載せるか**」の行は「`OnGhostChanging` が返した台本を**そのまま**載せる」と書く（出典 areka-P0-ghost-shell-balloon-switch）。要件 6.2 は翻訳の後の台詞にするので、新しい 5 行を足すだけでなく、この行の書き換えが要る。`%username` の既定値の行（出典 areka-P0-sakura-dialogue-tags）も、展開が翻訳の前でも行われることを書き足す候補。

---

## 3. 要件ごとの対応表

記号: **有**＝今あるものが使える／**欠**＝足りない／**未**＝調べが要る（Research Needed）／**制**＝制約。

| 要件 | 今あるもの | 判定と中身 |
|---|---|---|
| 1.1・1.2 翻訳を 1 か所で | 出口 1 つ・`StartTalk` の腕 8 か所 | **欠**: 翻訳の待ちと腕。8 か所から 1 つの関数へ寄せる形か、`step` の出口でまとめて捕まえる形か（論点 2）。 |
| 1.3・1.4 areka の台詞は翻訳しない | epilogue は別の欄・epilogue だけの腕は `boot.rs` の 1 か所 | **有**（`script` だけを渡せば足りる）。ただし `step` の出口でまとめて捕まえる形では「SHIORI が空を返した＋記録あり」と「areka が作った」を見分ける印が要る（論点 2）。 |
| 1.5 翻訳を外すと赤 | 運行表の純粋なテスト・偽の SHIORI | **有**（道具）・**欠**（各経路のテスト）。 |
| 2.1〜2.3 展開してから翻訳 | `resolve_system_var`・`DEFAULT_USERNAME`・sylphya の写し | **欠**: 文字列のままの展開・kanade への値の道（論点 4・5・16）。**制**: kanade は sakura・parsers に依存しない。 |
| 2.4 `username` 以外は残す | 写しに `selfname` 等が載り、今日は compile が展開している | **未・食い違い**: 要件の文と今日の実態がずれる（論点 7）。 |
| 2.5 204 のとき今日と 1 文字も違わない | 字句の規則（貪欲・タグの引数・`\%`・`\\`） | **欠**: 字句解析と同じ規則で展開位置を決める関数（論点 5）。値に `\`・`%` があるときの扱い（論点 6）。 |
| 2.6 残った綴りは再生時に展開 | `compile` の `SystemVar` の腕 | **有**（触らずに残す）。 |
| 3.1・3.4 GET を 1 回・許可表 | `round_trip_request`・`ALLOWED_EVENT_IDS` | **欠**: `events.rs` に `on_translate`、表に 1 語（数のテスト 45→46）。 |
| 3.2 Ref0・Ref2・Ref3 | 送った後は ID も Reference も捨てる | **欠**: 元のイベントの ID（選択肢の任意名を含む）と Reference を応答の後まで持つ所。Ref3 はバイト値 1（`\u{1}`）で連ねる。 |
| 3.3 Ref1 欠番 | 0 から詰める線の形のみ | **欠・制**: 欠番を線で表す方法（論点 8）。 |
| 3.5 切替の間の送り先 | kanade と SHIORI はゴーストごとに 1 組（`crates/areka-ghost/src/runtime.rs` の結線） | **有**: 送り出しの台詞は前のゴーストの kanade の中で、起動の挨拶は新しいゴーストの kanade の中で翻訳される＝形の上で正しい送り先になる。 |
| 3.6 同じ見出し | `ExecutionStatus::derive`・線の `SecurityLevel: local` | **有**（`SecurityLevel` は自動）。Status の中身は**未**（論点 9）。 |
| 4.1〜4.3 200・空・204 | `ShioriOutcome::Value`・`NoContent` | **欠**: 応答の腕。空の Value をそのまま採る（`raise_outcome_of` は空白だけの Value を「返事なし」に数えるが、これは外からの依頼の返事の話で再生には影響しない）。 |
| 4.4 エラー応答は元の台詞＋警告 | `round_trip_request` が 204 へ写し `warn!(shiori_error_response)` を残す（ID 付き） | **有**（既存の警告に元のイベントの ID を足すかは論点に含める）。 |
| 4.5 輸送路の失敗は故障 | `on_shiori_reply` の `Failed`→`to_unloading_fault` | **有**。ただし相を足す形では `awaits_reply` に入れないと応答が捨てられる。選択の連鎖の後は既存の例外と食い違う（論点 10）。 |
| 4.6 再び送らない | — | **欠**（翻訳の応答の腕は `StartTalk` だけを返す形にすれば構造で成り立つ）。 |
| 4.7 記録 | `tracing` の語彙の慣行 | **欠**: `event = "translate_…"` の語彙。 |
| 5.1〜5.4 一度に 1 往復・待ちの間の入力 | `drive` の繰り返し・受け箱 | **有**（構造）・**欠**（状態の表し方＝論点 1）。止まる偽の SHIORI で「待ちの間に届いた入力は再生の後に処理される」を確かめられる。 |
| 5.5 再生の開始の時点の状態 | 各腕が相を決めてから `StartTalk` を返す | 形によって変わる（§4）。帳簿の形なら相は今日と同じ値のまま。 |
| 5.6 往復の数 | — | 空の台詞をどう数えるかが**未**（論点 11）。 |
| 6.1・6.2 控える台詞は翻訳の後 | `ActiveTalk.script`・`ChangeState.script` の入れ所が kanade の中 | **欠**: 翻訳の応答で控えを書き換える。`emo2_boot/` には触れずに済む。 |
| 6.3 §8 の登記 | §8 の表 | **欠**: 5 行＋既存の `OnGhostChanged` Ref1 の行の書き換え（§2.9）。 |
| 7.1〜7.4 MAKOTO の口 | — | **欠**: 「台詞と元のイベントの ID → 台詞」の関数 1 つ（素通しだけ）。kanade の中に置けば DLL の知識は入らない。 |
| 8.1 決定論テスト ⑴〜⑨ | 運行表のテスト・偽の SHIORI・ログの捕まえ方（`schedule/log_capture.rs`・`log-capture-kit`） | **有**（道具）。 |
| 8.2 emo2（pasta）で今日と同じ | 実機の台詞の記録 | **未**: pasta が `OnTranslate` に 204 を返すことの確認（`vendors/pasta` はこのワークツリーでは中身が無い）。**制**: `emo2_boot/` の偽の SHIORI が panic する（論点 12）。 |
| 8.3 全体のテスト | `tools/test-all.ps1` | **制**: 記録の列を比べる既存テストの書き換え（論点 13）。 |
| 8.4 記録の無い経路 0 本 | log-first の慣行 | **欠**（新しい腕すべてに記録）。 |

---

## 4. 実装の形の候補

### 案 A: 相（`Phase`）を 1 つ足す（brief の案 ⒜）

- 形: `Phase::Translating { resume }` を足し、8 か所の腕を「翻訳を始める（`OnTranslate` を送る）→ 応答で `resume` に従って今日の遷移をする」に書き換える。`resume` は経路ごとの続き（`BootVersion` へ・`CloseTalkWait` へ・`ChangeTalkWait` へ・`Steady{Some}` へ・選択の解決つき…）。
- 触る所: `schedule/mod.rs`（相・`phase_label`・`awaits_reply`・`current_talk_id`・`talk_active_of`・`dispatch_phase`）・`boot.rs`・`close.rs`・`change.rs`・`steady.rs`（先に分割）・`talk_gap.rs`・`events.rs`・`actor.rs`。
- 良い点: 「翻訳を待っている」が相の名前として見える。brief と roadmap の想定どおり。
- 悪い点:
  - 台詞の切れ目の見張り（`talk_gap.rs` の `observe`・`decide`）が壊れる。印のイベントの応答の直後に相が `Translating` になるので「印の台詞は無かった」「定常でない＝終了中」と誤って決まる。見張りに翻訳の相を教える改修が要る（`areka-P0-shell-balloon-switch` の振る舞い）。
  - `OnTranslate` の Status は相を `Translating` にした後に撮ると `talking` が落ち、撮り方を経路ごとに決める必要がある。
  - 相の追加は `match` の網羅（`phase_label` など、意図して既定の腕を置いていない所）すべてに判断を求める。`mod.rs` が 1,000 行に近づく。
  - 8 か所すべての書き換え＝既存テストの差分が大きい。

### 案 B: 相は今日のまま進め、帳簿で翻訳を待つ（本分析の案）

- 形: 各腕は今日どおり相・採番・期限・控えを決める。ただし `StartTalk`（と、同じ一括でその後ろに並ぶ行動）をすぐには出さず、`State` に `translate: Option<TranslateWait>`（`talk_id`・元のイベントの ID と Reference・展開済みの台詞・後ろへ回した行動）を置き、行動は `OnTranslate` の GET 1 本にする。応答は `on_shiori_reply` の**先頭の腕**（`Unloading` の判定より前・`awaits_reply` に依らない）で受け、最終の台詞を MAKOTO の口へ通し、`ActiveTalk.script`・`ChangeState.script` を書き換え、後ろへ回した行動を返す。
- 捕まえ方は 2 通り（論点 2）:
  - B1: 8 か所の腕が `StartTalk` を作る代わりに共通の関数（例 `translate::defer(state, start, rest)`）を呼ぶ。
  - B2: `step` の出口（今 `talk_gap::observe` が居る所）で、返る行動の中の SHIORI 由来の `StartTalk` を見つけて差し替える。元のイベントは `step` が送る `ShioriRequest` を毎回帳簿に写して覚える。
- 触る所: `schedule/mod.rs`（帳簿・先頭の腕）・新しい `schedule/translate.rs`（兄弟テスト）・`events.rs`（`on_translate`・許可表）・`actor.rs`（展開の実行点・記録）。B1 なら 8 か所の腕も（1 行ずつ）。
- 良い点:
  - 相・期限・`talk_gap` の見張り・Status の導き方が今日と同じ値のまま（要件 5.5 が形で成り立つ）。`OnTranslate` の Status も今日の再生の開始時点の状態から撮れる。
  - 帳簿の型は `pending_close`・`choice` と同じ前例がある。`steady.rs` の分割が要らない見込み（B2 なら腕を書き換えない）。
  - 選択の連鎖では `choice` の帳簿が既に消えているので、翻訳の失敗が選択の例外に紛れない。
- 悪い点:
  - 「翻訳を待っている」が相の名前に出ない（要件 5.2 の「運行の状態の 1 つとして表す」を帳簿で満たすと読むかは論点 1）。
  - 待ちの間の相は「再生中のつもり」の値になる。ただし殻が受け箱を読まないので、外からこの状態を見る入力は来ない。
  - B2 は暗黙で、areka が作る台詞の見分けに印が要る。

### 案 C: 殻の中で同期に済ませる（brief の案 ⒝）

- 形: 運行表は変えず、殻の `execute_actions` が `Action::StartTalk` を実行する直前に展開と `OnTranslate` の往復を済ませ、結果の台詞で送る。
- 良い点: 運行表の差分が最小。
- 悪い点:
  - 「一括の最後の往復の応答だけを入れ直す」の決まりに 2 つ目の例外を作る（boot の一括 `[StartTalk, basewareversion NOTIFY]` では NOTIFY の応答を入れ直す必要があり、翻訳の応答は入れ直さない特別扱いになる）。
  - 控える台詞（要件 6）を翻訳の後にするには、運行表へ結果を知らせる入力がもう 1 つ要る。
  - 輸送路の失敗（要件 4.5）を運行表の故障へつなぐ道が別に要る。
  - 元のイベントの Reference を殻が覚える必要がある。
  - 要件 5.2（例外を作らない）と真っ向から当たる。

### 展開の置き場所の候補（どの案とも組み合わせる）

| 候補 | 形 | 良い点 | 悪い点 |
|---|---|---|---|
| X1 | kanade が `areka-sakura`（と `areka-parsers`）に依存して展開する | 展開の関数をそのまま呼べる | `crates/areka-kanade/Cargo.toml` を触る（同じウェーブの `crates-io-publish` が各 `Cargo.toml` を触る）。値の源（sylphya の写し）は結局外から渡す必要がある。kanade（運行）が再生側の部品に依存する向きが新しく生まれる |
| X2 | 展開の関数を外から注入する（`ResourceSink` と同じ形・`KanadeConfig` の欄か `spawn_kanade` の派生）。中身は `areka-ghost` が sylphya の写し＋`resolve_system_var`＋字句解析と同じ規則の走査で組む | kanade の依存は増えない。前例どおり。テストは注入を替えるだけで「太郎さん」を作れる | `crates/areka-ghost/src/runtime.rs`（結線）を触る（brief の「非接触」とずれるが要件の禁止範囲の外）。注入の関数は値を読む＝純粋でないので、運行表（`step`）ではなく殻で呼ぶ形にする必要がある |
| X3 | kanade が起動の照会で得た `username` を自分で持つ | 純粋 | `selfname` 等や起動の後の値の変化を拾えず、要件 2.3（今日と同じ値）を満たさない |

---

## 5. 規模と危険

- **規模: M〜L**（16〜20 タスクの見込み）。運行の本体は案 B なら M。テストの書き換え（偽の SHIORI の既定・記録の列）と、展開の関数（字句解析と同じ規則）と、線の Reference1 の扱いで膨らむ。案 A は `steady.rs` の分割と `talk_gap` の改修が加わり L に寄る。
- **危険: 中〜高**。運行表の作り（相の網羅・見張り）は知っている形だが、⑴ `emo2_boot/` の偽の SHIORI が panic する件が要件の境界とぶつかる ⑵ 展開を前へ出すと字句の規則の細部（貪欲な名前・タグの引数・エスケープ・値の中の `\`）で表示が 1 文字ずれる危険がある ⑶ 記録の列を比べる既存テストが広く動く。

---

## 6. 設計で決める論点（要件の討議へ）

1. **翻訳の待ちの表し方**: 相を足す（案 A）か、帳簿で表す（案 B）か。要件 5.2 の「運行の状態の 1 つ」を帳簿で満たすと読んでよいか。相を足すなら台詞の切れ目の見張り（`talk_gap.rs`）と Status の撮り方の改修が要る。
2. **翻訳を通す点の捕まえ方**: 8 か所の腕が共通の関数を呼ぶ（明示）か、`step` の出口でまとめて捕まえる（暗黙）か。後者なら「areka が作る台詞」の印をどう持つか（今は `script` が空で `epilogue` だけ、という見た目でしか見分けられない）。
3. **後ろへ回す行動の範囲**: `StartTalk` だけか、同じ一括でその後ろに並ぶ行動（boot の `basewareversion` NOTIFY）も翻訳の後へ回すか。選択の連鎖の `ResolveChoice{old}` は翻訳の前に出すか後に出すか（今日は同じ一括で `ResolveChoice` → `StartTalk` の順）。
4. **展開の置き場所と実行点**: X1（kanade の依存を増やす・`Cargo.toml` を触る）か X2（注入・`areka-ghost` の結線を触る）か。値を読む関数は殻で呼び、結果を運行表へ入力として渡す形でよいか。
5. **展開の字句の規則**: 今日の字句解析と同じ規則（貪欲な名前・タグの引数は展開しない・`\%`・`\\`）で位置を決めるため、`crates/areka-parsers/src/sakura/lexer.rs` に「展開する位置を返す」公開の関数を足してよいか。brief の「lexer に触れない・既知の名前の最長一致の文字列走査」は `%usernameabc` で今日と表示が変わる（今日は `%usernameabc` のまま・最長一致だと「太郎abc」）ので要件 2.5 と両立しない。
6. **展開した値に `\` や `%` があるとき**: そのまま埋めると、再生時の字句解析がタグや環境変数として読み直し、今日と表示が変わる（今日は値を文字として扱う）。エスケープして埋める（`\` → `\\`・`%` → `\%`）と表示は今日と同じだが、`OnTranslate` の Reference0 にエスケープの綴りが見える。どちらにするか。
7. **要件 2.4 の「`username` 以外」の読み**: 今日の写しには `selfname`・`selfname2`・`keroname` なども載り、compile は値があれば展開している。翻訳の前に展開するのは `username` だけか、写しにある名前すべてか（どちらでも再生時の展開が残りを拾うので表示は変わらない。変わるのは `OnTranslate` が見る文字）。要件の文の手直しが要るかもしれない。
8. **Reference1 の欠番の運び方**: 線で行ごと抜く（`ShioriCall` の Reference の型・`ShioriBackend` の 15 余りの実装・`crates/shiori-host32-host/src/shiori3.rs` の `build_request`・in-proc の経路まで変わる）か、空文字で送る（`OnBoot` の「前回落ちた」の前例＝番号を詰めない・里々／YAYA は空と欠けを区別しない見込み）か。
9. **`OnTranslate` の Status の中身**: 翻訳の後の再生が始まる時点の状態（定常の台詞なら `talking`）で撮るか、元のイベントを送った時点と同じにするか。正典は沈黙。
10. **選択の連鎖・時間切れの後の `OnTranslate` の輸送路の失敗**: 要件 4.5 どおり故障（終了系列）にするか、選択の往復の既存の例外（失敗を 204 と同じに扱う・`choice_shiori_failed_as_204`）に揃えるか。
11. **空の台詞（200 で空・空白だけ）を返したイベント**: 今日も `StartTalk` は出る。これに `OnTranslate` を送るか（要件 5.6 の「台詞が返った」の線引き）。
12. **`emo2_boot/` の偽の SHIORI との衝突**: `crates/areka/src/emo2_boot/spine.rs` の `ScriptedShioriBackend::get` は台本に無い GET で panic する（`username`・`homeurl` だけ既定あり）。`OnTranslate` に既定の 204 を補う 1 行を足すには `emo2_boot/` に触れる必要があり、要件の「`emo2_boot/` に触れない」とぶつかる。境界を広げる（同じウェーブの `shell-balloon` と順番を決める）か、別の方法（例: 翻訳を構成で切れるようにして既存テストでは切る＝本番と違う構成になる）を取るか。`crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の同名の型も同じ。
13. **記録の列を比べる既存テストの扱い**: 期待の列に `OnTranslate` を書き足すか、比べる道具に「`OnTranslate` を除いて比べる」を足すか（陳腐化か破損かの方針に沿って分ける）。
14. **COMPAT §8 の既存の行の書き換え**: 「`OnGhostChanged` の Ref1 に何を載せるか」の行（「そのまま載せる」）を翻訳の後へ改める。新しい 5 行と同時に行う。
15. **許可表の注記と台帳**: 他の 45 語は ukadoc の URL の注記つきだが、URL の注記は `areka-P0-ukadoc-survey-shiori` の仕事（要件の範囲外）。`OnTranslate` の行を注記なしで足し、台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）も `absent` のままにするか。
16. **展開の写しを撮る時点**: 翻訳の直前（kanade）と再生の開始（dispatcher の `on_start`）で 2 回撮ることになる。間は数ミリ秒だが、その間に値が変わると翻訳に渡した名前と残りの展開の名前が食い違いうる。翻訳の前に撮った写しを再生へ渡す（`StartTalk` に写しを載せる＝依存ゼロの契約 `areka-talk` の型を変え、`areka-ghost` の `dispatcher.rs` の `on_start` も変わる）かどうか。

### 6.1 要件の討議での仕分け（2026-10-03）

- **要件の側で片付けたもの**:
  - 論点 7 → 要件 2.3・2.4 を「今日の再生時の展開と同じ規則（写しに値がある名前はその値・`username` は既定値あり・それ以外は綴りのまま）」に改めた。
  - 論点 11 → 中身が 0 文字の 200 には `OnTranslate` を送らない（要件 3.1・5.6・§8 の ⑹）。空白だけの台詞は「中身がある」として送る。
  - 論点 14 → 要件 6.4（§8 の既存の行を改める）。
  - 論点 15 → `areka-P0-ukadoc-survey-shiori` は完了済みのため、URL の注記と台帳の `implemented` 化は本 spec が行う（要件 3.7・前例 `areka-P0-file-drop`・`areka-P0-network-update`）。
  - 論点 1 の一部 → 要件 5.2 を「運行表の状態として表す（相か帳簿かは設計）・殻の中で往復を足す形（案 C）は取らない」に改めた。
  - 要件の生成の段で brief から変えた 2 点はそのまま採る: 輸送路の失敗は他のイベントと同じく故障（09-24 の `shiori-fault-notice` の裁定が brief の後に出た）・翻訳の待ちの間の終了の要求は打ち切らず受け箱で待たせる（今日のどの GET の往復とも同じ）。
- **設計（`/kiro-spec-design`）で決めるもの**: 論点 1（相か帳簿か）・2・3・4・5・6・8・9・10・13・16。
  - 論点 8 の注意: 要件 3.3 は正典どおり「欠番」（Reference1 の行そのものが無い）を求める。空文字で送る形を選ぶなら要件へ戻して改める必要がある。
  - 論点 10 の注意: 要件 4.5 は例外を作らない。選択の往復の既存の例外（`choice_shiori_failed_as_204`）は選択の往復そのものに限り、その後の `OnTranslate` には及ばない形を前提にする。
- **開発者に問うもの**: 論点 12（`emo2_boot/` のテスト用の偽の SHIORI と境界）→ **2026-10-03 裁定: 案ア**。本番のコードは触らず、テスト用の偽の SHIORI に「`OnTranslate` は既定で 204」を足す（記録には残す）こととテストの期待の列の書き足しだけを境界の内とする。`shell-balloon` 等と同じテストのファイルが重なったら後着の側が rebase で直す（要件の「開発上の制約」）。注意: `spine.rs` は 998 行なので、足すのは既存の `homeurl` の既定と同じ式へ混ぜる形にし、1,000 行を超えない。

---

## 7. 調べが要ること（Research Needed）

- **pasta（emo2）の `OnTranslate` への応答**: 204 を返すと見込むが未確認（`vendors/pasta` はこのワークツリーで中身が無い）。設計の前に emo2 で 1 回動かして記録を見る。
- **実際に `OnTranslate` を持つゴースト**: `vendors/sample_ghost/R_POST_and_KOMAINU.nar` の中に `OnTranslate` の綴りがある（中身は未確認）。実機の確かめの候補。
- **里々・YAYA の `OnTranslate` が Reference1〜3 をどう読むか**（空と欠けの違いを見るか・Reference2 で出所を分けるか）。ukadoc MCP の YAYA docs・里々 Wiki で引く。
- **正典が `OnTranslate` を空の台詞にも送るか**（ukadoc の「スクリプトが返却された場合に」の読み）。

---

## 8. 設計への申し送り

- 推す形は決めない。比べる軸は「要件 5.2 の読み（相か帳簿か）」「`talk_gap` と Status への波及」「触るファイルの数（`steady.rs` の分割の要否）」の 3 つ。
- 展開は「今日の字句解析と同じ規則で位置を決め、今日と同じ値で、今日と同じ表示になる」ことを先に単独の部品として固める（brief の「展開の前倒しは単独でも着地できる」）。その後に翻訳の往復を足す順が、差分の確かめ方として素直。
- 論点 12（`emo2_boot/` の偽の SHIORI）は要件の境界に関わるので、設計より前に答えが要る。

---

## 9. 設計フェーズの記録（2026-10-03・`/kiro-spec-design`）

### 9.1 Summary
- **Feature**: `areka-P0-translate-pipeline`
- **Discovery Scope**: Extension（既存の運行表への差し込み。調べ方は light＝既存コードの差し込み点・依存・前例の確認。外部の新しい依存は 0）
- **Key Findings**:
  - 台詞から再生を始める腕 9 か所（SHIORI の台詞 8・areka が作る台詞 1）は、どれも入力が `Input::ShioriReply` の `step` の中でだけ `Action::StartTalk` を作る。よって `step` の出口 1 か所で全部を捕まえられ、腕を 1 行も書き換えずに済む（`steady.rs` の分割も要らない）。
  - 線の Reference の組み立ては `crates/shiori-host32-host/src/shiori3.rs` の `build_request` の 1 か所で、補助プロセス経由も in-proc（`crates/areka-ghost/src/shiori_inproc.rs` の `build_input`）もここを通る。欠番はここ 1 か所で表せる。
  - 文字列のままの展開は、値の先頭が直前のタグの読みを変える並びがある（`\w%username` で値が数字始まり・`\n%username` で値が `[` 始まり・`\_%username`）。エスケープだけでは足りず、読みの同値の照合が要る。

### 9.2 Research Log

#### 再生を始める腕の再確認
- **Sources**: `crates/areka-kanade/src/schedule/boot.rs` の `to_baseware_version`・`close.rs` の `on_close_pending`・`change.rs` の `on_reply_wait`／`on_yielded_reply`・`steady.rs` の `on_reply`／`on_cascade_reply`／`on_timeout_reply`
- **Findings**: §2.2 の表のとおり 9 か所（うち `on_reply_wait` は `OnGhostChanging` と切替の `OnClose` の 2 役）。`StartTalk` を `ShioriReply` 以外の入力から作る所は 0 か所。areka が作る台詞は 204 の応答の `step` で `script` が空のまま生まれる。
- **Implications**: 「台詞を返した応答の `step` が作った `StartTalk`」という規則で、SHIORI の台詞と areka の台詞を見た目に頼らず分けられる。

#### 許可表と外からの依頼
- **Sources**: `schedule/events.rs` の `ALLOWED_EVENT_IDS`（45 語）・`allowed_static`・`schedule/events_change_tests.rs` の数の検査
- **Findings**: 許可表は「送ってよい」と「外から頼める」を兼ねる。`OnTranslate` を足すと汎用の通知の入口からも頼めるようになる。既にある `OnClose`・`OnFirstBoot` も同じ。
- **Implications**: 表を 2 つに分ける仕組みは作らない（design の OnTranslateCall の Risks に記載）。

#### `ShioriBackend` の実装の数と境界
- **Sources**: `crates/areka-kanade/src/shiori/real.rs` の `ShioriBackend`・実装 17 個（本番は `ShioriConnection`・`InProcBackend`、残りはテストの偽物）
- **Findings**: Reference の型を変えると 17 個の実装の関数の形が変わり、`crates/areka/src/emo2_boot/spine.rs`（998 行）の偽の SHIORI も変わる。要件の「開発上の制約」が許すのは「`OnTranslate` は既定で 204」を足すことだけ。
- **Implications**: 欠番は関数の形を変えずに運ぶ（欠番の印）。

#### 字句の規則
- **Sources**: `crates/areka-parsers/src/sakura/lexer.rs` の `lex`・`scan_tag`・`bare_tag_len`・`scan_bracket_args`・`scan_sysvar`
- **Findings**: `%` の後ろは英数字と `_` を貪欲に読む。`\\`・`\%` は文字。角括弧の中の `%` は引数の一部。未閉じの `[` は末尾まで 1 かたまり。角括弧なしのタグの長さは直後の文字に左右される（短縮形 `\w`・`\b`・`\p` ＋数字、`_` 始まりの 2〜3 文字、直後の `[`、`\q*[`）。
- **Implications**: 置き換えの位置は `lex` と走査を共有して決める。値を埋めた後の読みは `parse` の結果で照合する。

#### pasta（emo2）の `OnTranslate` への応答（§7 の調べ事の 1 つ目）
- **Sources**: 主リポジトリの `vendors/pasta`（`48c42fc3`・2026-09-20）の `crates/pasta_lua/pasta_scripts/pasta/shiori/event/init.lua` の `EVENT.fire`・`EVENT.no_entry`、`pasta/scene.lua` の `SCENE.co_exec`
- **Findings**: 登録の無いイベントは同名のシーンを探し、無ければ `nil` → 204 を返す。続きを待っているシーン（`STORE.co_scene`）には触れない。
- **Implications**: emo2 では `OnTranslate` は 204 で、会話の続きにも影響しない見込み。実機での確認は実装の最初に行う（emo2 の DLL がこの版と同じとは限らない）。

#### 里々・YAYA の `OnTranslate`（§7 の調べ事の 3 つ目）
- **Sources**: ukadoc MCP の YAYA docs「OnTranslateイベント」「OnTranslateの使い方」
- **Findings**: 例はどれも `reference0` だけを読んで置換し、その文字列を返す（敬称の重なり・句読点の後の `\w5`）。Reference1〜3 を読む例は無い。
- **Implications**: 欠番と空文字の違いが標準の辞書の動きを変える見込みは薄いが、要件 3.3 は正典どおりの欠番を求めるので欠番で送る。応答にはタグが足されうる（`\w5`）ので、翻訳の結果は再生側で普通に字句解析される前提で足りる。

### 9.3 Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 相を足す（案 A） | `Phase::Translating` と経路ごとの続き | 待ちが相の名前に出る | `talk_gap` の見張りと Status が変わる・相の網羅の `match` 全部・`steady.rs` の分割 | 不採用 |
| 帳簿＋腕が共通の関数を呼ぶ（案 B1） | 8 か所の腕を 1 行ずつ書き換える | 明示 | 呼び忘れが素通りになる・8 か所の差分 | 不採用 |
| 帳簿＋出口で捕まえる（案 B2） | `step` の出口で一括を預ける | 腕の差分 0・将来の腕も自動で通る・相が今日のまま | 暗黙になる（規則を 1 か所に書いて補う） | **採用** |
| 殻で同期（案 C） | `StartTalk` の実行の直前に往復 | 運行表の差分が最小 | 要件 5.2 に反する | 不採用 |

### 9.4 Design Decisions

#### Decision: 翻訳の待ちは相の外の帳簿、捕まえるのは `step` の出口
- **Context**: 論点 1・2・3。要件 5.2・5.5・1.2・1.5。
- **Alternatives Considered**: 9.3 の表。
- **Selected Approach**: `State::translate`（預けた一括）と `State::reply_source`（元のイベント）を足す。`step` は `route` の前後で `translate::before`／`translate::after` を呼ぶ。条件（台詞を返した応答の `step`・台詞が 1 文字以上・一括に `StartTalk` がある）を満たせば、一括を丸ごと預けて `[Action::Translate]` に替える。結果は専用の入力 `Input::TranslateDone` で戻す。
- **Rationale**: 再生を始める時点の相・期限・見張り・Status が今日と同じ値になることが形で決まる。`Input::ShioriReply` の形を変えないので、それを組み立てている多数の既存テストが変わらない。翻訳の結果が専用の入力なので、応答待ちの相の判定（`awaits_reply`）にも選択の往復の例外にも当たらない。
- **Trade-offs**: 待ちの間、相は「再生中のつもり」の値になる。ただし 1 つの `drive` の中で完結するので、その状態を見る外からの入力は無い。
- **Follow-up**: 最上位の `step` を通る既存テストのうち「`Value` の直後に `StartTalk`」を期待するものの数を、実装の最初に数える。

#### Decision: 一括を丸ごと後ろへ回す
- **Context**: 論点 3。
- **Alternatives Considered**: `StartTalk` だけ回す／一括を丸ごと回す。
- **Selected Approach**: 丸ごと・順序そのまま。
- **Rationale**: 起動の一括（`StartTalk`＋`basewareversion` NOTIFY）を割ると 1 つの一括に往復が 2 つ並ぶ。選択の連鎖の `ResolveChoice` を先に出すと、古い台詞の選択待ちが解けてから新しい台詞が始まるまでに隙間ができる。
- **Trade-offs**: `basewareversion` は `OnTranslate` の後に送られる（今日は挨拶の `StartTalk` の直後）。挨拶との前後は今日と同じ。

#### Decision: 展開は注入の関数を殻が呼ぶ。実体は sakura、位置は parsers
- **Context**: 論点 4・5・6・16。要件 2.1〜2.5。
- **Alternatives Considered**: kanade が sakura に依存する（X1）／注入（X2）／kanade が `username` を持つ（X3）／最長一致の文字列走査（brief）。
- **Selected Approach**: `areka_parsers::sakura::substitute_system_vars`（`lex` と走査を共有・値はエスケープ）→ `areka_sakura::expand_system_vars`（`resolve_system_var` で値を決め、`parse` の結果で同値を照合し、違えば元の文字列を返す）→ `areka-ghost` の `translate_wiring` が写しの源と組んで `ScriptExpander` にする → 殻が `Action::Translate` の実行の中で呼ぶ。
- **Rationale**: kanade の依存は増えない。規則の写しを作らない。同値の照合 1 つで、タグが値を飲み込む並びをすべて拾える（並びを列挙しない）。
- **Trade-offs**: 読みが変わる並びの台詞は翻訳の前に展開されず、`OnTranslate` が `%名前` を見る（表示は今日と同じ）。値の中の `\`・`%` は Reference0 にエスケープの綴りで見える。写しは台詞 1 つにつき 2 回読む。
- **Follow-up**: 値の境界で書記素がつながる並び（design の SysVarExpander の Risks）は検査を足さない。実在のゴーストで見つかったら同値の照合に書記素の数の比較を足す。

#### Decision: Reference1 の欠番は線の層の印で運ぶ
- **Context**: 論点 8。要件 3.3。
- **Alternatives Considered**: Reference の型を変える／空文字で送る／印。
- **Selected Approach**: `shiori3::ABSENT_REFERENCE`（NUL 1 文字）。`build_request` がその位置の行を出さず番号を保つ。
- **Rationale**: 関数の形を変えないので 17 個の実装と境界の外のファイルに触れない。線の組み立ては 1 か所なので、補助プロセス経由も in-proc も同じ結果になる。
- **Trade-offs**: 値の中に印を混ぜる形（型で表していない）。印は線に載せられない文字で、実際の値と重ならない。偽の SHIORI は印をそのまま受け取る。
- **Follow-up**: 将来 Reference の欠番が他のイベントでも要るようになったら、型で表す形へ移す（そのときは `ShioriBackend` の形を変える spec を別に立てる）。

#### Decision: 応答の読みは純粋な関数、MAKOTO の口は殻で 1 回
- **Context**: 要件 4・7。
- **Alternatives Considered**: 応答の読みと MAKOTO の口をそれぞれ行動と入力の組にして運行表を 2 段にする／殻の 1 回の実行にまとめる。
- **Selected Approach**: 殻の `run_translate` が「展開 → 往復 → `translate::read_reply`（純粋）→ MAKOTO の口」を 1 回の実行で行い、最終の台詞か輸送路の失敗を `Input::TranslateDone` で戻す。
- **Rationale**: MAKOTO の口は後半の spec で DLL を呼ぶので運行表（純粋）の中には置けない。2 段にすると帳簿の段と入れ直しの種類が増えるが、本 spec の口は素通しなので得るものが無い。口を関数 1 つにしておけば、後半の spec は結線で差し替えるだけで済む。
- **Trade-offs**: 応答の読みを殻が呼ぶ（判断そのものは純粋な関数に置き、殻は呼ぶだけ）。

#### Decision: `OnTranslate` の Status は再生を始める時点の状態
- **Context**: 論点 9。正典は沈黙。
- **Selected Approach**: 捕まえた時点（腕が相を決めた後）の `State::snapshot`。
- **Rationale**: 既存の決まり「Status は送る時点の運行の状態から導く」のまま。起動の挨拶では直後の `basewareversion` と同じ値になる。
- **Follow-up**: §8 に登記する（要件 6.3 の 6 点とは別に、設計で決めた裁量として）。

#### Decision: 既存テストは期待の列に書き足す
- **Context**: 論点 13。
- **Selected Approach**: `OnTranslate` を期待の列に足す。偽の SHIORI は既定で 204 を返し、記録に残す。除いて比べる道具は作らない。
- **Rationale**: 往復の数（要件 5.6）が既存テストからも見える。

### 9.5 Synthesis（まとめ直しの結果）
- **一般化**: 5 種類の経路は「SHIORI の GET が台詞を返し、その `step` が再生を始める」という 1 つの形の変種。経路ごとに継ぎ目を作らず、出口の規則 1 つにした。
- **作るか使うか**: 展開は既存の `resolve_system_var`・`lex`・`parse` を使う。失敗の分類は `areka-P0-shiori-fault-notice` の決まりを使う。注入は `ResourceSink` の前例、帳簿は `pending_close`・`choice` の前例に倣う。新しい外部の依存は 0。
- **削ったもの**: 翻訳の相・経路ごとの続きの型・MAKOTO の待ちの段・Reference の新しい型・`StartTalk` に写しを載せる欄・「`OnTranslate` を除いて比べる」道具・許可表を 2 つに分ける仕組み。

### 9.6 Risks & Mitigations
- 最上位の `step` を通る既存テストの差分が広い — `translate_test_support.rs` に「翻訳を 204 で通す」補助を置き、期待の列への書き足しを機械的にする。
- `crates/areka/src/emo2_boot/spine.rs` が 998 行 — 既定の 204 は既存の `homeurl` の既定の式へ混ぜ、行を増やさない。
- 同じウェーブの spec と同じテストのファイルが重なる — 後から着地する側が rebase で直す（要件の「開発上の制約」）。
- pasta の実物が 204 以外を返す — 実装の最初に emo2 を 1 回動かして `translate_reply` の `kind` を見る。エラー応答なら警告が台詞ごとに出るだけで表示は変わらない。
- 展開の関数・MAKOTO の口が返らないと運行が止まる — 本 spec の実装（写しの読み取り・素通し）はすぐ返る。後半の spec の口は期限つきで返す責任を持つ（design の TranslateRunner の Risks）。

### 9.7 References
- [トランスレータ](https://ssp.shillest.net/ukadoc/manual/manual_translator.html) — 翻訳の順序とタイミング
- [OnTranslate](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnTranslate:1) — Reference0〜3・「OnTranslate 自身では再度発生しない」
- [YAYA docs: OnTranslateの使い方](https://yaya-shiori.github.io/yaya-docs/tips/on-translate-usage/) — 標準の辞書が Reference0 だけを読む例

### 9.8 実装の最初の確かめ（2026-10-03・タスク 1.1）

結論: **emo2 の pasta は `OnTranslate` に 204 を返す**。続きを待っている雑談の途中でも 204 で、その雑談の続きは `OnTranslate` を挟まない場合と同じように出る。実装はこのまま進めてよい。

#### 使ったもの（すべてワークツリーの `target\` の下・絶対パス）
- 補助プロセス: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe`（32bit・`cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`）
- pasta: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\target\nar-samples\manual\emo2\ghost\emo2\ghost\master\pasta.dll`（`cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` で展開した emo2）
- 読み込みの手順は `crates/areka-kanade/tests/kanade/real_helper_test.rs` と同じ（親の窓 → 補助プロセスの起動 → HELLO → LOAD）。要求は手で組んだ SHIORI/3.0 の文字列をそのまま送り、Reference1 は**行ごと出さない**形（正典どおりの欠番）で送れた。確かめのための `#[ignore]` テストは `real_helper_test.rs` に一時的に足し、走らせた後に `git checkout` で消した（リポジトリに残っていない）。

#### 送った要求と応答（1 回の読み込みの中で上から順に）

| # | ID | Reference | 応答 |
|---|---|---|---|
| 1 | `OnBoot` | 0=`master` | **200**・起動の台詞（`\p[0]…こんばんわー！…夜の部、開幕やー！…\p[1]…夜更かしはお肌に悪いよ。…\e`）＝ pasta の Lua が読めている対照 |
| 2 | `OnTranslate` | 0=#1 の台詞そのもの・1=欠番・2=`OnBoot`・3=`master` | **204 No Content** |
| 3 | `OnTranslate` | 0=`\0テスト\e`・1=空文字・2=`OnSecondChange`・3=`0`〜`0` を 0x01 で連結 | **204 No Content**（今の線の層の「空文字で送る」形でも同じ） |
| 4 | `OnSecondChange` | 0=`0`・1=`0`・2=`0`・3=`1`・4=`0` | 204（時計の初期化） |
| 5 | `季節07月`（emo2 の辞書でチェイントークを持つシーンを名前で直に呼ぶ） | なし | **200**・前半（`…七夕やー！…短冊に願い事書かな！…何をお願いするの？…ひ・み・つ♪…ふーん？…\e`）。シーンは続きを待つ状態になる |
| 6 | `OnTranslate` | 0=#5 の台詞そのもの・1=欠番・2=`OnSecondChange`・3=#3 と同じ | **204 No Content**（雑談の途中） |
| 7 | `OnSecondChange` を 1 秒ごと | #4 と同じ | 25 回は 204、26 秒後に **200**・後半（`…七夕は置いといて、…天神祭の奉納花火のほうが見たいな。…わかってるやん！…あれめっちゃ最高やねん！…\e`） |

- 比べる相手として、#6 だけを抜いた同じ手順をもう 1 回、別の読み込みで走らせた。26 秒後に同じ後半（同じ文・同じ待ち）が出た。違ったのは表情の 1 か所（`\s[静観]` と `\s[通常]`）だけで、これは pasta が表情を毎回くじで選ぶためで、#1 の起動の台詞でも同じ揺れが出ている。
- よって、雑談の途中に `OnTranslate` を挟んでも続きは捨てられず、同じように進む（research §9.2 の見込みのとおり）。後半が出るまでの秒数は pasta の雑談の間隔（くじ）で決まり、どちらも 26 秒だったのはたまたま。
- どちらの回も最後は正規の終了（unload）で `Clean`。

記録: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\target\translate-baseline\probe-with-translate.txt`（#6 あり）・`…\target\translate-baseline\probe-control-no-translate.txt`（#6 なし）

#### 本 spec の前の版で採った比べる記録（タスク 6.2 で比べる相手）
- 版: この確かめの時点の HEAD（`11cb22ca`・翻訳の仕組みはまだ無い）の `cargo build -p areka`。`target\debug\shiori-host32-helper.exe` は 32bit の版で上書きしてから起動した（64bit のままだと pasta の読み込みが 0x800700C1 で落ちる）。
- 起動: `RUST_LOG=info,kanade=trace,areka_kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS=420000`（自動の終了より先に手で終了した）で、`target\debug\areka.exe <emo2 のゴーストの絶対パス> <emo2-kakukaku の絶対パス>`。終了コード 0。
- 1 周の中身（いずれも実機で手で操作）:
  1. 起動: `OnFirstBoot`（Reference0=`0`）が台詞を返し、起動の台詞を再生（`boot_talk` talk_id=1）。画面の台詞は「OK？ まあ、これから、よろしゅうに！」「ちがうよう。よろしくね。」
  2. 雑談: `OnSecondChange` 由来の `steady_talk` が talk_id=2〜8 の 7 回。
  3. 選択肢: キャラをダブルクリック → `OnMouseDoubleClick` の台詞（emo2 のメニュー・選択肢 3 つ・雑談を置き換えて talk_id=9）→「おしゃべり頻度」（`Onおしゃべり頻度メニュー`・talk_id=10・選択肢 4 つ）→「ほどよく」（`Onほどよくおしゃべり`・talk_id=11）→「もどる」（`Onメインメニュー`・talk_id=12）→「閉じる」（`Onメニュー閉じる`・talk_id=13）。続けて雑談 1 回（talk_id=14）、もう一度ダブルクリック（talk_id=15）→「閉じる」（talk_id=16）。
  4. 終了: 右クリックのメニュー →「終了」→ `OnClose`（Reference0=`user`）→ 終了の台詞（`close_talk_start` talk_id=17・画面は「またね～。」）→ `talk_done_quit` → `unload_clean` → `app_exit`。
- 置き場所:
  - 全体の記録: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\target\translate-baseline\baseline-run.log`（1,857 行）
  - 台詞の記録（再生の開始・選択・終了の行と、毎秒の `OnSecondChange` を除いた `shiori_request` の行を順に抜いたもの）: `…\target\translate-baseline\talk-record.txt`（100 行）
- 6.2 で比べるときの注意:
  - **今の版は台詞の本文を記録に残さない**（`steady_talk` などの行は talk_id と元のイベントだけ）。比べられるのは「どのイベントが台詞を返し、どの順で再生が始まったか」の並びと、画面の目視（起動・メニュー・終了の台詞）である。
  - 雑談の回数と中身はくじで毎回変わる。比べてよいのは、起動（ただし 2 回目以降の起動は `OnFirstBoot` でなく `OnBoot` になりうる）・ダブルクリックのメニュー → 選択肢の連なり（台詞を返すイベントの名前と選択肢の数）・終了の握手の並び。
  - マウスを動かした分の `OnMouseMove` の行は操作で変わるので差に数えない。

### 9.9 実機の確かめ（2026-10-04・タスク 6.2）

結論: **翻訳を挟んでも、emo2 で表示される台詞の並びは本 spec の前と同じ**。再生を始めた 13 回すべてで `OnTranslate` を 1 回ずつ送り、pasta はどれにも 204 を返した。翻訳を飛ばす枝・失敗の記録は 1 件も出ていない。

#### 走らせ方
- 版: `9ed0dd88`（タスク 6.1 の後）の `cargo build -p areka`。`target\debug\shiori-host32-helper.exe` は 32bit の版（machine=0x14c）で上書きしてから起動した。
- 起動: `RUST_LOG=info,kanade=trace,areka_kanade=trace`・`AREKA_APP_SMOKE_EXIT_MS=600000`（自動の終了より先に手で終了した）で、`target\debug\areka.exe <emo2 のゴーストの絶対パス> <emo2-kakukaku の絶対パス>`（どちらも `target\nar-samples\manual\emo2\` の下）。終了コード 0。
- 1 周の中身（画面操作で実施）: 起動（`OnFirstBoot`）→ 雑談 5 回 → キャラをダブルクリックしてメニュー →「おしゃべり頻度」→「ほどよく」→「もどる」→「閉じる」→ 雑談 1 回 → 右クリックのメニューの「終了」→ `OnClose` の別れ → `talk_done_quit` → `unload_clean` → `app_exit`。

#### 確かめた 3 点
1. **`translate_reply` が台詞ごとに `kind=no_content` で 1 件**: 再生の開始（`boot_talk` 1・`steady_talk` 11・`close_talk_start` 1）が 13 回で、`translate_begin`・`translate_reply`・`translate_resume` もそれぞれ 13 件。`translate_reply` の元のイベントは `OnFirstBoot` 1・`OnSecondChange` 6・`OnMouseDoubleClick` 1・選択肢の任意名 4（`Onおしゃべり頻度メニュー`・`Onほどよくおしゃべり`・`Onメインメニュー`・`Onメニュー閉じる`）・`OnClose` 1 で、すべて `kind="no_content"`。選択肢の任意名は Reference2 にも逐語で載っている。
2. **台詞の記録が 1.1 の記録と同じ**: 毎秒の `OnSecondChange`・`OnMouseMove`・`OnTranslate` を除いた `shiori_request` と、再生の開始・選択・終了の行を順に並べて比べた。並びは同じ形（起動の照会 → 起動の挨拶 → `basewareversion` → 雑談 → メニューの選択の連なり → 終了の握手）。違いは次の 2 つだけで、どちらも操作とくじによる: 雑談の回数（前 7 回・後 5 回＋1 回）と、前の回だけ 2 回目のダブルクリックをしていること。前の回の最初のダブルクリックは雑談の最中で「置き換え」の腕を通り、後の回は雑談の合間で「空き」の腕を通った（どちらも翻訳を通る腕）。画面の台詞（起動の挨拶・メニューの文言と選択肢・選んだ後の返事）も前の回と同じだった。`OnTranslate` の Reference0 には pasta が返した台詞がそのまま載っている（emo2 の台詞には `%username` などが無いので、展開しても変わらない）。
3. **記録の無い分岐が無い**: `translate_skipped_empty`・`translate_skipped_self`・`translate_source_missing`・`translate_failed`・`sysvar_expand_fallback`・`reference_absent_marker_replaced` はどれも 0 件。13 回の翻訳はすべて「捕まえる → 204 → 預けた一括を返す」の 1 本の道を通った。

#### 置き場所
- 全体の記録: `C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\target\translate-after\after-run.log`
- 比べた結果: 同じフォルダの `compare-output.txt`（比べる道具は同じフォルダの `compare.py`）
- 前の記録: `…\target\translate-baseline\`（9.8）
- 注意: 前の回の記録は文字コードが違うため、比べた結果の中で選択肢の名前が文字化けして見える。並びの比べには影響しない。
