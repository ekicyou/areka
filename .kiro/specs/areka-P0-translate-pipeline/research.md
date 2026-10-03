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
