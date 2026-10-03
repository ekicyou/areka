# 設計レビュー: areka-P0-choice-timeout-directive

> 2026-10-03・ブランチ `claude/areka-p0-choice-timeout-1b7ecf`（設計 `eeab6d83`）。設計の主張はすべて実物のコード・文書を `grep`／`Read` で突き合わせた。コードは「何の定義か」で指す。

## レビューの要約

設計は要件 1〜10 を漏れなく部品へ落としており、鍵になる 2 つの変更（compile の `GenericCommand` の腕での読み取り・dola `TimedSchedule::tick` の `timeout_dur` の `WaitForChoice` の腕）は実物のコードで裏が取れた。テストの置き場・補助関数・台帳の連鎖も、名指しされたものはすべて実在し、`Cargo.toml`／`Cargo.lock` に触れない。実装へ進める（GO）。残るのは設計討議で確認したい 3 点で、いずれも設計の骨格を変えない。

## 検証した事実（設計の主張 → 実物）

| 設計の主張 | 実物での確認 |
|---|---|
| dola の `timeout_dur` の腕を `WaitForChoice { .. } => None` にすれば「着いたときに飛ばす」「止まっている間に自分で解く」の両方が選択の区切りに効かなくなる（要件 5） | `crates/dola/src/cue/schedule.rs` の `TimedSchedule::tick`: `timeout_dur` は `Entry::Barrier` の腕でだけ作られ、`barrier_timeout_offset` を書くのも同じ腕だけ。`None` なら `barrier_timeout_offset` は `None` のまま止まり、解けるのは `notify_barrier_resolved` だけ。`WaitForInput`・`Timeout` の腕は無改変で、既存テスト `input_barrier_with_timeout_skipped_when_jumped_past`・`barrier_timeout_auto_releases`・`timeout_barrier_skipped_when_already_past`（`tests/cue/schedule_test.rs`）が守る |
| `WaitForChoice` に `Some` を入れて tick する既存テストは無い（書き換え 0） | ワークスペース全体で `WaitForChoice { timeout: Some` は `crates/dola/src/cue/command_tests.rs` の serde 往復 1 か所だけ。dola のテスト 6 ファイルはすべて `timeout: None` |
| `TimedSchedule` を直接使う本番コードは `CuePlayer`（`runtime.rs`）と `to_talk_schedule`（`sheet.rs`）だけ | `facade.rs`・`cue/mod.rs`・`areka-sakura/src/lib.rs` の言及は doc 注記と `pub use` のみ |
| 字句の出力の形（省略 → `["choicetimeout"]`・空欄 → `["choicetimeout",""]`・空白は残る） | `scan_bracket_args`（`lexer.rs`）: `,` で push・`]` で「何か読んだ／既に積んだ／引用符を消費した」なら最後の 1 個を push・空白の切り落としなし。`decode_passthrough_bang`（`decode.rs`）が先頭を `name`・残りを `raw_args` に。`set` は特別扱いされず（`move` だけ `Instruction::Move`）、`End`／`Quit` で `compile` が `break` する |
| 下流（通知・中継・kanade）は秒のまま無改変（要件 8） | `TalkDriver::notify_choice_waiting_if_newly_waiting`（`drive.rs`）が `*timeout` を素通し → `DispatcherState::on_choice_waiting`（`dispatcher.rs`）が `timeout_directive_secs` を素通し → `choice_deadline`（`schedule/choice.rs`）が `None`＝既定・`<= 0`＝無期限・`> 0`＝`round(v×1000)`。`fire_choice_timeout_if_due` は `>=` で到達判定 |
| テストの補助関数が実在する | areka-sakura: `compile_test_support::{compile（1 引数）, command_of, cue_eq}`・`barrier_of` は `compile_arm_tests.rs` の私的関数（移す対象）・`drive_test_support::{recv_done, NEG_WINDOW, TalkNotice（From<TalkDone>／From<ChoiceWaiting>）}`。areka-ghost: `test_log_capture::{capture, assert_logged_event(events, Level, target, event_name)}`（`lib.rs` で `#[cfg(test)] mod`）・`dispatcher_test_support::{RecordingSink, test_system_vars, run_bounded}`・`spawn_dispatcher` は `dispatcher.rs` の `pub fn`。kanade: `tests/kanade/choice_test.rs` が `#[cfg(test)] #[path]` で 7 本を束ね、`choice_test_test_support::establish_choice_wait` と `super::{spawn_harness_gated, Fixture, …}` が引ける |
| `Cargo.toml`／`Cargo.lock` に触れない | areka-ghost は `[dependencies]` に `areka-sakura`・`areka-parsers`、`[dev-dependencies]` に `log-capture-kit` を既に持つ。areka-sakura に開発時依存は足さない。`areka_sakura::compile` を ghost のテストスレッドで呼ぶ前例は `sink.rs` の `diagnostic_default_wiring_logs_each_cue_exactly_once_through_broadcast` |
| 既存の `timeout: None` の表明 7 か所は無改変で緑（要件 9.6） | `compile_arm_tests.rs` 6 か所・`compile_sheet_tests.rs` 1 か所・`drive_choice_tests.rs` の `choice_waiting_notifies_exactly_once_with_ids_horizon_and_timeout`。指定の無い台本なので値は `None` のまま |
| 台帳の連鎖（要件 10） | `ledger/sakura-script.toml` の当該項目は `status = "vocabulary-only"`・`owner = "areka-P0-sakura-time-directives"`。`roadmap-draft.md` は `[briefs].count = 39`・`[[spec]]` 39 行・`sakura-time-directives` の `owner_count = 11`・「会話」の表に `sakura-time-directives（W16・5 件）`。前例「2026-09-29 の 2 行目の追加」あり。生成器の副手続きは `report`・`report-summary`（`cli_tests.rs`）。証拠の走査は `_tests.rs`・`tests/`・`#[cfg(test)]` を除外し、トークンは `ukadoc:`（前例 `sysvar.rs` の `%username`）。`COMPAT_ARCHITECTURE.md` §8 の allowlist の行・`choice-cascade-compat.md` 行 5b／5d・`briefing-sakura-script.md` の行はすべて実在 |
| 触らない場所 | `crates/areka-kanade/src/schedule/`・`crates/areka-emo-text/`・`crates/areka/src/emo2_boot/`・`crates/areka/src/input_events/`・`crates/areka-parsers/` は 0 ファイルで成立する。`drive.rs`・`dispatcher.rs`・`tests/kanade/choice_test.rs` への接続宣言 1 行は要件の「0 と明記」の一覧に無く、`structure.md` の兄弟テストの接続規約（親ファイルに `#[cfg(test)] #[path]` を 1 行）に従った最小の形 |

## 重要な指摘（3 件・設計討議へ）

### 指摘 1: 要件 9.4「台本から期限まで通し」を 2 本のテストの合成で満たす点の明文化

- **懸念**: 設計は ghost のテスト（台本 → `KanadeMsg::ChoiceWaiting { display_end: 1_350, timeout_directive_secs: Some(1.234) }`）と kanade の外側のテスト（`Some(1.234)` を受けて `display_end + 1_234` ちょうどで `OnChoiceTimeout`）を、同じ値 `N = 1234` で結んで「通し」とみなす。`schedule/` が 0 ファイル・`choice_deadline` が `pub(crate)` という制約の下では妥当だが、2 本が別クレートにあるので境の値を 1 つの定数で共有できず、片方だけ値を変えても両方緑のまま通る。
- **影響**: 要件 9.4 の読みが「1 本で貫く」なら満たしていない、と後から言われうる。
- **提案**: 設計の Testing Strategy にある合成の説明を、両テストの doc 注記に互いの名前付きで写し（「対になるテストは ○○」）、要件 9.4 を「2 本の合成で満たす」と討議で確定しておく。コードは変えない。
- **要件**: 9.4 ／ **設計の箇所**: Testing Strategy「Integration Tests（ghost／kanade の外側）」の末尾の段落。

### 指摘 2: 読めない値の警告の出し方（要件 3.4 との読み合わせ）と警告文の綴り

- **懸念**: 設計は「選択肢の有無で出し分けない」「読めない指定 1 回につき 1 行」を選んだ。要件 3.4 は選択肢の無い台本について「誤りとしても扱わない」と書くので、「警告は誤り扱いではない」という設計の読みを開発者が追認する必要がある。また、設計に書かれた警告文 `"[compile] \![set,choicetimeout,…] の時間が…"` は Rust の文字列では `\!` が不正なエスケープになる（`\\!` か raw 文字列が要る）。
- **影響**: 読みが追認されないと要件 3.4 の解釈で手戻りが出る。綴りはコンパイルで止まるだけだが、設計をそのまま写すと赤になる。
- **提案**: 討議で「3.4 は失敗にしない・振る舞いを変えない、の意味」を確定し、設計の警告文は `\\![set,choicetimeout,…]` の形に直す（または `\!` を書かない文にする）。
- **要件**: 3.4・6.1・6.2 ／ **設計の箇所**: Components「compile の `GenericCommand` の腕」の Responsibilities 3 点目・Error Handling「Monitoring」。

### 指摘 3: 設計の文面と実物の小さなずれ（タスク生成で迷わないために）

- **懸念**: ⑴ `spawn_dispatcher` は `dispatcher_test_support` でなく `dispatcher.rs` の `pub fn`（兄弟テストからは `super::spawn_dispatcher`）。⑵ kanade の `establish_choice_wait` は `timeout_directive_secs: None` を固定で送るので、新しいテストは注入列を自前で書く（設計の「同じ型で指令だけ `Some`」はそういう意味だと読める）。⑶ `briefing-sakura-script.md` には設計が挙げた ⑵ 消費側の表・⑶ 担当の突合表に加え、「語彙の登記」の表にも `set,choicetimeout` の行（§8 の allowlist の行を指す）があり、§8 の行を書き足すなら整合を見る。⑷ 同じ C1 の `status-execution-states` は `ledger/sakura-script.toml` に 3 項目を持つので、先に着地すると `report/sakura-script.md`・`report/summary.md` を本 spec 側で作り直す（設計 9.4 の緩和策どおり）。
- **影響**: いずれも実装の方針は変えないが、タスクの記述をそのまま写すと探す場所を間違える。
- **提案**: タスク生成でこの 4 点を該当タスクの注記に転記する。設計本文の改訂は不要。
- **要件**: 9.2・9.4・10.5・10.6 ／ **設計の箇所**: Allowed Dependencies・File Structure Plan（文書・台帳）・research §9.4。

## 設計の強み

1. **要件 5 を 1 腕で構造として満たし、値の正本を 1 か所に保つ**: 案 A は実物の `tick` の作りと一致し、既存テストの書き換えが本当に 0 であることを全文 grep で確かめられた。区切りの値が正本（DD-8）のまま流れるので、`areka_talk::ChoiceWaiting` の注記も `choice-cascade-compat.md` 行 5d も嘘にならない。
2. **テストの置き場が制約を全部満たす**: 4 群すべてが兄弟ファイルか既存のテストファイルへの追記で、`Cargo.toml`／`Cargo.lock`・kanade の `schedule/`・parsers に触れない。名指しした補助関数は署名まで一致する。

## 判定

**GO**。既存の層の境界に乗った小さな変更で、要件への対応表・テスト・台帳の連鎖がすべて実物で裏付けられる。上の 3 点は設計討議で確定・転記すれば足り、設計の骨格を変えない。

次の段: 設計討議（指摘 1・2 の確定）→ `/kiro-spec-tasks areka-P0-choice-timeout-directive`。
