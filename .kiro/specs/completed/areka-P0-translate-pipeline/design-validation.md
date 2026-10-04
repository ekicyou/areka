# 設計レビュー報告: areka-P0-translate-pipeline

- 対象: `.kiro/specs/areka-P0-translate-pipeline/design.md`（要件は `requirements.md`・調査は `research.md`）
- 実施日: 2026-10-03
- 進め方: 非対話。設計の主張を実際のコードで確かめた（読んだ範囲は末尾「確かめたこと」）。
- 判定: **GO**（重要な指摘 3 件は設計討議で決めれば足り、設計の骨組みは変わらない）

## 要約

設計の中心（`step` の出口 1 か所で捕まえる・相は今日のまま進めて相の外の帳簿で待ちを表す・一括を丸ごと預ける）は、コードと突き合わせて成り立つことを確かめた。要件 1.1〜8.4 の全 ID が部品に結び付いており、依存の向きも増えない。残る懸念は、外から頼まれた `OnTranslate` が再び翻訳される穴、欠番の印が `OnTranslate` 以外の要求にも効く点、要件 2.2 の字面と「読みが変わる台詞は展開しない」裁量の食い違いの 3 つで、どれも小さな追記で閉じる。

## 重要な指摘（3 件）

### 指摘 1: 外から頼まれた `OnTranslate` の応答が、もう一度翻訳される

- **懸念**: 設計は `OnTranslate` を許可表 `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`）に足す。汎用の通知の入口 `on_raise_event`（`crates/areka-kanade/src/schedule/change.rs`）と台詞の切れ目の口 `begin`（`crates/areka-kanade/src/schedule/talk_gap.rs`）は、どちらも `events::allowed_static` に通った名前をそのまま GET で送る。その応答が台詞なら、定常の応答の腕 `on_reply`（`schedule/steady.rs`）が `StartTalk` を作り、出口の規則の 3 条件（入力が SHIORI の応答で `Value`・1 文字以上・一括に `StartTalk`）をすべて満たす。結果、元のイベントが `OnTranslate` の `OnTranslate` が送られる。
- **影響**: 正典の「OnTranslate 自身では再度発生しない」と要件 4.6 に反する経路が 1 本残る。設計の Risks は「害は台詞が 1 つ流れるだけ」と書くが、再翻訳には触れていない。
- **提案**: 出口の規則に 4 つ目の条件「元のイベントの ID が `OnTranslate` でない」を足す（1 行）。`translate_tests.rs` に「外から頼んだ `OnTranslate` の応答は `[StartTalk]` のまま出る」を 1 本足す。許可表を「送ってよい」と「外から頼める」に分ける案は要らない。
- **要件**: 4.6・3.4
- **設計の該当箇所**: 「TranslateLedger」の Responsibilities & Constraints（捕まえる条件）・「OnTranslateCall」の Implementation Notes（Risks）

### 指摘 2: 欠番の印がすべての要求に効く（`OnTranslate` に限られていない）

- **懸念**: 欠番の印（NUL 1 文字）は `build_request`（`crates/shiori-host32-host/src/shiori3.rs`）で「値が印と一致する位置の行を出さない」と決めるので、どのイベントのどの Reference でも効く。Reference は外から入る（汎用の通知の入口の `references`・選択肢の Reference）。値がちょうど NUL 1 文字なら、そのイベントの Reference 行が黙って消える。`OnTranslate` の Reference3 も、元のイベントの Reference が NUL 1 文字の 1 個だけなら同じ値になり、行が消える。
- **影響**: 実害は小さいが、記録の無いまま線の形が変わる経路になる（要件 8.4）。要件 3.3 そのもの（Reference1 の行が無い）は、補助プロセス経由（`crates/shiori-host32-host/src/client.rs` の要求の送り出し）も in-proc（`crates/areka-ghost/src/shiori_inproc.rs` の `build_input`）も `build_request` を通るので満たされる。
- **提案**: kanade の送出の 1 か所（`actor.rs` の `round_trip_request` から分ける「検査して送る」関数）で、「`on_translate` が作った添字 1 以外に印がある」要求を見つけたら `warn!` を 1 件残して空文字に置き換える。`on_translate` も Reference3 が印と等しくなる場合を同じ扱いにする。`events_tests.rs` に 1 本ずつ足す。Reference の型を変える案（17 個の実装に波及）は採らなくてよい。
- **要件**: 3.3・8.4
- **設計の該当箇所**: 「AbsentReference」・「設計で決めたこと」論点 8

### 指摘 3: 要件 2.2 の字面と「読みが変わる台詞は展開しない」の食い違い、および検査の無い要件

- **懸念**: 設計は、展開すると台本の読みが変わる台詞（`\w%username` で値が数字始まり・`\n%username` で値が `[` 始まり・`\_%username`）を展開せずに `OnTranslate` へ渡す。字句解析 `scan_tag`（`crates/areka-parsers/src/sakura/lexer.rs`）は語の読み取りを `%` で止めるので、この判断は正しく、要件 2.5 を守るにはこれしかない。ただし、そのとき Reference0 に `%username` が残り、要件 2.2（「展開した後の台詞を入れる」）に例外ができる。要件には例外の記述が無い。あわせて、要件 2.6（翻訳の結果に残った `%username` を再生時に展開）・3.5（切替の間の送り先）・3.6（`OnTranslate` の `Status`）には、名指しの決定論テストが無い（追跡表では「変更なし」「構造」とだけ書かれている）。
- **影響**: 例外を開発者が承知しないまま実装に入ると、後で要件との不一致として差し戻される。2.6 は「再生側を変えない」ことに頼っており、展開の位置を動かした後の後退を拾う検査が無い。
- **提案**: 設計討議で「読みが変わる台詞は展開せずに送る（§8 の 9 番）」を要件 2.2 の例外として開発者に確認する（要件は確定済みなので、§8 の登記で足りるかを決める）。テストを 3 本足す: ⑴ `OnTranslate` が `%username` を含む台詞を返したら再生側で展開される（`areka-ghost` の結線テスト・2.6）⑵ 起動の挨拶の `OnTranslate` の `Status` が `talking`、終了の別れでは行なし（`translate_path_tests.rs`・3.6）⑶ 切替の送り出しの `OnTranslate` が切替の前の SHIORI の記録に載る（既存の切替の結合テストの期待の列・3.5）。
- **要件**: 2.2・2.5・2.6・3.5・3.6・8.1
- **設計の該当箇所**: 「設計で決めたこと」論点 6・「SysVarExpander」の同値の照合・「Requirements Traceability」の 2.6／3.5／3.6 の行・「Supporting References」の 9 番

## 設計の強み

1. **出口 1 か所で捕まえる形は、コードの事実と合っている**。`Action::StartTalk` を作る所は 9 か所（`schedule/boot.rs` の `to_baseware_version` に 2・`close.rs` の `on_close_pending` に 1・`change.rs` の `on_reply_wait` と `on_yielded_reply` に 1 ずつ・`steady.rs` の `on_cascade_reply`・`on_timeout_reply`・`on_reply` の 2 腕）で、設計の表と一致する。9 か所とも入力が SHIORI の応答のときだけ通り、areka が作る台詞（起動の記録だけ）は 204 の腕で `script` が空のまま生まれるので、条件に当たらない。相を足さないので、`talk_gap::observe`・`State::snapshot`・`awaits_reply`・`current_talk_id` の判断は今日と同じ値を読む。
2. **「往復は一度に 1 つまで」に例外を足さない**。`drive`（`crates/areka-kanade/src/actor.rs`）は 1 メッセージの処理の間は受け箱を読まないので、翻訳の待ちの間の入力（マウス・毎秒の時刻・終了・切替・外からの依頼）は積まれたまま残り、再生の開始の後に今日の規則で処理される。起動の `[StartTalk, basewareversion]` と選択の連鎖の `[ResolveChoice, StartTalk]` を丸ごと預ける判断も、1 つの一括に往復が 2 つ並ぶことと、古い台詞が先に進む隙間の両方を避けている。

## 軽微な指摘（判定には影響しない）

- **切替の送り出しで翻訳の輸送路が失敗したとき**: `on_reply_wait` は翻訳の前に `ChangeState.script` へ元の台詞を控える。`on_done` の `Err` で故障へ進むと、表示されなかった台詞が停止通知の切替の中身（`actor.rs` の `handoff_of`）に残る。`Err` の腕で `ChangeState.script` を空にすると要件 6.2 と揃う。
- **`drive` の停止原因の控え**: `drive` は `step` の直前に `stop_cause_of` と `handoff_of` を控える。翻訳の結果を入れ直す枝でも同じ控えを取ること（故障で止まるときの原因が落ちない）。
- **1,000 行の上限**: 上限はテストのファイルにも効く（`crates/log-capture-kit/tests/file_length_guard_test.rs`）。期待の列を直すファイルのうち `crates/areka-kanade/src/actor_tests.rs`（970 行）・`schedule/steady_flow_tests.rs`（925 行）は余裕が小さい。期待の本体は `crates/areka/src/emo2_boot/spine_conformance_script.rs`（808 行）にあるので emo2 側は足りる。タスクで行数の見込みを先に数えること。`spine.rs` は既定の式に条件を 1 つ混ぜるだけで 998 行のまま収まる。
- **数の食い違い**: `ShioriBackend` の実装は設計の「17 個」に対し、`crates/` の検索では 16 個（判断には影響しない）。
- **文字のかたまり**: 展開した値は前後の文字と 1 つの `Text` になる（今日は `compile` の `SystemVar` の腕が別のかたまりを作る）。文字の並びと合計の再生時間は同じ（`text_playback_duration` は書記素の数に比例）。設計が書いた書記素の境界の件を除き、差は無い。

## 確かめたこと

| 設計の主張 | 結果 | 根拠 |
|---|---|---|
| `StartTalk` は SHIORI の応答の `step` でだけ作られる | 正しい | `schedule/` の本番の 9 か所を全数確認 |
| 一括を預けても相・期限・見張りは今日と同じ | 正しい | `step`・`talk_gap::marked_reply`／`observe`・`on_shiori_reply`（`schedule/mod.rs`・`talk_gap.rs`） |
| 待ちの間は他のイベントを送らない | 正しい | `drive` と `execute_actions`（`actor.rs`） |
| 補助プロセス経由も in-proc も `build_request` を通る | 正しい | `client.rs` の送り出し・`shiori_inproc.rs` の `build_input` |
| 欠番の印は実際の値と重ならない | 一部（指摘 2） | 外から入る Reference には制限が無い |
| `\\`・`\%` は文字として読まれる | 正しい | `lex` のエスケープの腕（`lexer.rs`） |
| 値の先頭が直前のタグに飲まれる並びがある | 正しい | `scan_tag` の短縮形・角括弧・`bare_tag_len` |
| 値の無い名前は綴りのまま・`username` は既定値 | 正しい | `resolve_system_var`（`crates/areka-sakura/src/sysvar.rs`） |
| 偽の SHIORI は ID ごとに答える | 正しい | `ScriptedShioriBackend::get`（`emo2_boot/spine.rs`・`tests/ghost/spine_e2e_test.rs`）・kanade の結合テストの `respond` |
| 未知の ID に 204 | 正しい | `shiori4-testdll` の `select_response` |
| 要件 1.1〜8.4 の全 ID が追跡表にある | 正しい | 38 行を全数確認（テストの名指しが無いのは指摘 3 の 3 件） |
| 境界の外（`ghost_session.rs`・`main.rs`・`emo2_boot/` の本番）に触れない | 正しい | Modified Files の一覧 |

## 最終判定

**GO**。構造の食い違い・要件の取りこぼし・過大な複雑さは無い。指摘 1〜3 を設計討議で決め（1 と 2 は条件 1 行ずつとテスト、3 は開発者の確認とテスト 3 本）、その結果を持って `/kiro-spec-tasks areka-P0-translate-pipeline` へ進める。
