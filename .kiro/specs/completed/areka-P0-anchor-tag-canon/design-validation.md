# 設計レビュー: areka-P0-anchor-tag-canon

> 2026-10-10・`kiro-validate-design`（非対話・サブエージェント）。入力は `design.md`・`requirements.md`・`research.md`・`brief.md`・steering（product／tech／structure／logging・roadmap の C5 の行）。設計の主張は現在のワークツリーのコードを Grep／Read で確かめた（引用は型名・関数名で指す）。

## Design Review Summary

設計は「選択肢 `\q` の道に種類 `SpanKind` を 1 つ足す」形で一貫しており、新しく作るのは範囲の開閉（`state_anchor.rs`）と帳簿の無い kanade の受理（`schedule/anchor.rs`）の 2 点だけ。設計を左右する事実 5 件（網羅の match の所在・SHIORI 往復の同期・中断は左ダブルクリックだけ・空回しの前方一致の比較対象・`choice_active` の消費者）はすべてコードで裏が取れた。要件討議の裁定 3 件も設計に写っていて蒸し返しは無い。残る問題は、台帳と連動する文書・檻の列挙漏れ（タスク生成で拾わないと赤になる）と、不変条件の文言の矛盾の 2 つで、どちらも構造を変えない。

## Critical Issues

### 🔴 Critical Issue 1: 台帳と連動する文書・檻の列挙漏れ（直さないと全体テストが赤になる）

**Concern**: 「変更するファイル」の表に無いが、本設計の変更で必ず赤になる／古びるものがある。
- `doc/ukadoc-coverage/briefing.md` の `[[barrier]] page = "list_shiori_event"`（今 `implemented = 50`・`absent = 236`）。`OnAnchorSelect`／`OnAnchorSelectEx` を `implemented` へ移すと `ukadoc-survey` の判定 ⑷ 腕 f（`briefing_arms.rs::distribution_findings`）が数え直しとの食い違いで赤になる。
- `doc/ukadoc-coverage/report/{sakura-script,shiori,assets}.md` と `report/summary.md`。台帳を動かすと `DomainReportStale`（`check/finding.rs`）と `documents_checks.rs::summary_findings`（全文一致）が赤になる。先例（`choice-script-prefix` の tasks）どおり `cargo run -p ukadoc-survey -- report` と `report-summary` で作り直す。
- `crates/areka-kanade/src/schedule/events_change_tests.rs:40` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 51)`——2 名足すので 53 へ。
- `crates/areka/src/input_events/shell_box_handler.rs` の窓の離脱の道（`on_box_pointer_leave` 相当・`rt.choice_active(&actor)` を `hover_action` に渡す箇所）。表は `read_point` の 1 行だけを挙げているが、ここも `hit_active` にしないと箱のアンカーの強調が離脱で戻らない（要件 3.2／3.7）。
- 古びるだけで赤にはならないもの: `crates/dola/src/cue/command_tests.rs::cue_command_ten_variants`・`crates/dola/tests/cue/sheet_test.rs` の「presentation コマンドは 10 種」（手書きの列なので赤にはならないが「全 variant」の主張が嘘になる）。

**Impact**: 実装は正しくても `/kiro-validate-impl` の全体テストで `ukadoc-survey` と kanade の檻が赤になり、タスクに無い作業が終盤に湧く。箱の離脱は利用者に見える不具合。
**Suggestion**: 「変更するファイル」の表に `briefing.md`（barrier の 2 数）・報告 4 本の作り直し（手で直さない）・`events_change_tests.rs`（51→53）・`shell_box_handler.rs` の離脱の道を足し、dola の「10 種」の 2 檻は 12 種へ揃える。タスク生成では台帳・briefing・roadmap-draft・報告の作り直しを 1 タスクにまとめる。
**Traceability**: 要件 7.3・7.4（台帳）・3.2・3.7（箱の離脱）・4.5（許可表）・8 全般。
**Evidence**: design.md「File Structure Plan › 変更するファイル」「Testing Strategy › 網羅の檻」。

### 🔴 Critical Issue 2: 不変条件「`glyph_range` は互いに素」と「Choice の追記で範囲を伸ばす」が矛盾している

**Concern**: 「Data Models › 不変条件」は `glyph_range` を**互いに素**と書き、`state.rs` の `ChoiceSpan` の doc（「互いに素かつ追記順に単調」）もそう言う。一方「System Flows › 文字の層の範囲」は「Text や Choice の追記で end を伸ばし」、要件 3.5 の 2 段走査は `\_a[x]…\q[題,ID]…\_a` の重なりを前提にしている。アンカーの範囲が選択肢の範囲を包むなら互いに素ではない。
**Impact**: 実装者がどちらを守るかで `state_anchor.rs` の `extend_open_anchor`（Choice の腕で呼ぶか）と Reference0 の文字（選択肢の文字を含むか）が変わる。純粋層は重なりに耐えることを確かめた（`annotate_lines` はスパンごとに独立に行と交差・`decorate_canvas` は行と ordinal で絞る・`derive_hit_rows` は矩形を 1 つずつ出す・`hit_choice_row` は種類の 2 段）ので、壊れるのは文言と檻だけである。
**Suggestion**: 不変条件を「**同じ種類の**範囲は互いに素・アンカーの範囲は選択肢の範囲を包んでよい（押下は選択肢が勝つ＝3.5）」に改め、`state.rs` の doc と `choice_anchor_tests.rs` の「混在」の檻をこの文言に合わせる。Reference0 には選択肢の文字も含める（開きから閉じまでに追記された書記素クラスタ、の定義どおり）。もし包まない方を選ぶなら、Choice の腕で `extend_open_anchor` を呼ばないことを設計に明記する。
**Traceability**: 要件 1.7・2.8・3.5・4.1。
**Evidence**: design.md「Data Models › 不変条件」「System Flows › 文字の層の範囲」「Existing Architecture Analysis」（`hit_choice_row` の段落）。

## Design Strengths

1. **既存の道の再利用が構造で裏付けられている**。純粋層（`annotate_lines`／`derive_hit_rows`／`decorate_canvas`）が `ordinal` と `glyph_range` しか読まないことをコードで確かめた上で `kind` を 1 欄足す形にしており、ホバー・当たり・箱の結線が無改変で両方に効く。後の 3 spec（`range-choice-tag`・`link-context-copy`・`balloon-link-hover`）が同じ列に乗れる。
2. **kanade の帳簿を持たない判断が同期の事実に立っている**。`actor.rs` の DD-2（`execute_batch` → `round_trip_request` → `reply_rx.recv()` → `Input::ShioriReply` の再投入）を根拠に `State.anchor` を 1 回の `step` の連鎖の中だけで生かし、`Failed`→Fault の免除も `on_shiori_reply` の先行アーム（`on_reply` が段を `take` する**前**）に足すので要件 4.10 が成立する。要件討議の裁定 3 件（既定の見た目・話している最中は消費して中断しない・照合の鍵を作らない）はそのまま守られている。

## 設計の主張の検証（依頼 (a)〜(g)）

| 項 | 結果 | 根拠 |
|---|---|---|
| (a) `CueCommand` の追加で壊れる網羅の match は設計の 4 か所だけか | **確認**（追記あり） | catch-all の無い match は `dola/src/cue/sink.rs::cue_target_of`・`areka-ghost/src/sink.rs::command_kind`・`areka-emo-text/src/actor.rs` の `apply_cue`（`Emote … Wait => {}` の列）・`areka-emo-text/src/state.rs` の `apply_cue`（`Emote | EntityRef | BalloonSurface | Wait` の列）の 4 つ。`areka-ghost/src/prop_sink.rs`・`areka-seriko/src/actor.rs`・`emo2_boot/*_cue.rs`・`lookahead.rs` は `Custom`／`Emote`／`Clear` 系だけを見て既定の腕を持つ（今 2〜3 腕でコンパイルが通っている＝catch-all がある）。`areka-sakura/src/contract.rs` は doc の言及のみ。檻は `dola/tests/cue/sink_test.rs`・ghost の `command_kind` の檻に加え、Issue 1 の「10 種」2 檻が古びる |
| (b) SHIORI の往復は同期で、GET 中に 2 回目の押下が割り込まない | **確認** | `areka-kanade/src/actor.rs` の冒頭 doc「DD-2 同期往復ループ」・`execute_batch` が `round_trip_request(shiori, call)` を呼び、`round_trip_raw` が `reply_rx.recv()` で待つ・結果は `drive` のループで `Input::ShioriReply` として `step` に再投入され、尽きるまで次の `KanadeMsg` は読まない |
| (c) 普通のバルーンの中断は左ダブルクリックだけ・`selected_now=true` で 3.3／3.4 が満たせる | **確認** | `input_events/user_break.rs::judge_press`——`double_click != Left` なら `NotDoubleClick`、`selected_now || prev_press_selected` なら `ConsumedBySelection`。`press_with_visibility` は単押しでも `prev_press_selected = selected_now` を書く。`balloon_pressed.rs` は `selected_now = sent`（種類を見ない）。箱は `shell_box.rs::judge_box_press` の第 1 分岐 `selected_now → ConsumedBySelection` |
| (d) 空回しの `begins_with` は `items`・`glyph_styles`・`styles` だけを比べる | **確認** | `areka-emo-text/src/lookahead.rs::begins_with` は `items().starts_with` ∧ `glyph_styles().starts_with` ∧ `styles().starts_with` の 3 項。`push_current_style` は `styles.intern(&decor.current, &layers.default)` の番号を `glyph_styles` に繰り返すだけなので、下線を立てた写しを同じ順で intern すれば空回しと本番で番号が揃う |
| (e) `choice_active` を `kind == Choice` に絞っても時間切れの抑止と `WaitForChoice` の柵は変わらない | **確認** | `TextLayerRuntime::choice_active`（`actor.rs`）は「場所の `choices` が非空」。消費者は `balloon_moved`／`balloon_pressed`／`balloon_exit`／`shell_box_handler`（`read_point` と離脱の道の 2 か所）／`balloon_visibility_phase` → `balloon_visibility_wait::observe_suppression`（`choice_active == Some(true)` で抑止）／`emo2_boot/hover_inject.rs`（env 付きの開発用）。柵は `compile.rs` の `has_choice`（`matches!(… CueCommand::Choice …)`）と dola `runtime.rs` の `pending_choices`（`if let CueCommand::Choice`）が源で `choice_active` を見ない。kanade の `ExecutionSnapshot.choice_active` は帳簿から導く別物 |
| (f) 1,000 行の見張りの余地と emo-text の `PURE_SOURCES` | **確認** | 番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`（`LINE_LIMIT` 1,000・例外表 10 件に本 spec の触るファイルは無い）。実測（物理行）: `schedule/mod.rs` 955（+約 12→967）・`steady.rs` 950（+5〜6）・`msg.rs` 926（+2〜3）・kanade `actor.rs` 900（+1）・`events.rs` 793（+約 45）・`compile_arm_tests.rs` 917（+数行）・`user_break_tests.rs` 812・emo-text `state.rs` 676・`state_decoration.rs` 635・`actor.rs` 647・`lib.rs` 524・`balloon.rs` 433・`choice_drain.rs` 422・`check_script_judge.rs` 182・`decode.rs` 477。`mod.rs`／`steady.rs` の余地は 30〜45 行で、設計の「本体は `anchor.rs` へ」の手当てどおりに腕だけを足せば収まる。`lib.rs` は `assert_eq!(PURE_SOURCES.len(), 73)` と `src/*.rs` の実ファイル集合との突き合わせの 2 檻＝3 ファイル足して 76。C5 の「emo-text に新しいソースを足す席」は本 spec が持つ（roadmap.md の C5 の行） |
| (g) 要件討議の裁定 3 件が守られ蒸し返されていない | **確認** | ①既定の見た目＝下線（⑤-a・`push_current_style` に焼く）＋ホバーは選択肢と同じ（`decorate_canvas` 無改変）。②話している最中の押下＝`selected_now=true` で消費し中断しない（新しい中断の道を作らない）。③照合の鍵なし＝`State.anchor` は段の記憶だけ（talk_id・候補・期限を持たない）・`on_anchor` は「届いたものをそのまま送る」・research §8-5 の busy 棄却も落としている |

### そのほかの小さな注記（議題にしない）

- `check_script_judge::diagnose` は「`\e`・`\-` で打ち切らない」が、`pair_anchors` は `End`／`Quit` で止まる。`\e` の後ろの `\_a` は再生でも読まれないので「本番と同じ内容」のままだが、設計に 1 行書いておくと実装で迷わない。
- アンカーの開きと閉じの間で行き先の場所が替わる（`\![set,…]` で箱へ）と、開きは前の場所に残り閉じは新しい場所で迷子になる。compile は対応済みと見るので文字の層の防御（`debug!`）で済み、害は無い。

## Final Assessment

**Decision: GO**

**Rationale**: 既存の境界・依存の向き・ログ規律に沿い、設計を左右する事実はすべてコードで裏付けられた。Critical Issue の 2 件は構造を変えず、設計討議で文言と表を直せばそのままタスク生成に進める。

**Next Steps**:
1. 設計討議で Issue 1 の追記（`briefing.md`・報告の作り直し・`events_change_tests.rs`・`shell_box_handler.rs` の離脱の道・dola の「10 種」2 檻）と Issue 2 の不変条件の文言を `design.md` に反映する。
2. `/kiro-spec-tasks areka-P0-anchor-tag-canon` でタスクを生成する（台帳・briefing・roadmap-draft・報告の作り直しは 1 タスクにまとめる）。
