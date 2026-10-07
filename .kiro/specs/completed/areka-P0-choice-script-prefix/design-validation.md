# 設計の検証レポート: areka-P0-choice-script-prefix

> 2026-10-05 `kiro-validate-design`（対話なしで実施）。入力: `requirements.md`・`design.md`・`research.md`・`brief.md`・steering。
> 設計書が既存のコードについて述べていることは、設計書を信じずにソースを読んで確かめた（重いビルドは回していない）。

## 判定

**GO（タスク生成へ進んでよい）**

設計書が頼っている既存の動きはすべてソースで裏が取れ、要件の 27 項目はもれなく部品・テスト・文書に割り当てられている。下の「話し合いたい点」は 2 件あるが、どちらも設計の作り直しを求めるものではない。

## まとめ

`script:` の選択肢の結末を、すでにある 2 つの形（選択肢のイベントが台本を返したとき／何も返さなかったとき）にそのまま重ねる設計で、新しい状態・新しい入力・新しい再生の仕組みを作らない。変える範囲は小さく、分かれ道はすべて実機なしのテストで固定できる。

## ソースで確かめたこと

| 設計書の主張 | 確かめた場所 | 結果 |
|---|---|---|
| `script:` の ID は今 `CascadePlan::Unsupported` になり、`on_choice` は警告を出して解決 1 つだけを返す | `schedule/choice.rs` の `plan_cascade`、`schedule/steady.rs` の `on_choice` の最初の腕 | 正しい |
| `on_choice` は帳簿を `take()` で取り出しており、戻さなければ期限ごと消える（タイムアウトの計測が終わる） | `steady.rs` の `on_choice` の冒頭と `Unsupported` の腕 | 正しい。今の腕もすでに戻していない |
| 「選択肢のイベントが台本を返したとき」の腕が書くのは `phase`・`next_talk_id`・`choice_prev_talk` の 3 つだけで、返す一括は「解決 → 再生開始」の順 | `steady.rs` の `on_cascade_reply` の `Value` の腕 | 正しい。終了の保留・切替の保留・中断の控え・切れ目の見張りには触れていない |
| 元のトークの遅れた完了の知らせは、1 世代の控えで情報の記録に落ちる | `schedule/mod.rs` の `on_talk_done` の末尾の腕 | 正しい。入れ子で 2 段続けて選んでも、控えは新しい方で上書きされるだけ（選択肢のイベントが続けて台本を返すときと同じ） |
| `Input::Choice` から出す再生開始は翻訳に捕まらない | `schedule/translate.rs` の `before`（材料の台本は入力が SHIORI の応答のときだけ）と `capture`（材料が無ければ素通し） | 正しい。`after` が控える「元のイベント」も、往復の無い一括では変わらない |
| 受け取る側は元のトークを閉じてから新しいトークを始める | `areka-kanade/src/actor.rs` の一括の実行（解決と再生開始を同じ通り道へ順に送る）、`areka-ghost/src/dispatcher.rs` | 正しい |
| 利用者の中断と重なっても食い違わない | `schedule/user_break.rs`（中断を受けた時点で帳簿を消す） | 正しい。中断の後の選択は「待ちが無い」で捨てられ、`begin` まで来ない |
| `"…"` の括りの中の `,` と `]` は区切りにならない／括らずに書くと `script:\![open` と余分な引数 2 個になる | `areka-parsers/src/sakura/lexer.rs` の `scan_bracket_args`、`decode.rs` の `decode_choice` | 正しい |
| 再生時の `%…` の展開は応答の台本と同じ規則 | `areka-sakura/src/sysvar.rs`（翻訳の前の展開は再生時と同じ `resolve_system_var` を使う） | 正しい（要件 1.5） |
| `pub(in crate::schedule)` と、`choice.rs` に置く `#[path = "choice_script.rs"] pub(super) mod script;` は通る | モジュールの並び（`schedule/mod.rs` が `pub(crate) mod choice;`・`pub(crate) mod steady;`） | 通る。`crate::schedule` は新しいモジュール `schedule::choice::script` の先祖なので範囲の指定は有効。`#[path]` は宣言を書いたファイルのあるフォルダから数えるので、`choice.rs` からも、その子の `choice_script.rs` からのテストの宣言も `schedule/` の中の兄弟のファイルを指す。`steady.rs` からは `super::choice::script::begin` で呼べる |
| `on_choice` の腕で `state` を書き替えられる | `State::snapshot_with_choice` は値を返す | 借用は残らない。`begin(&mut state, talk_id, &input)` の後に `input.id` を解決へ渡す順も通る |
| 新しいテストは最上位の `step` から通せる | `schedule::step`・`State` の欄・`ChoiceState`・`log_capture` はどれもクレートの中から見える。`steady` のテストの補助は `steady` の中からしか見えない | 正しい。自前の小さな補助を持つという設計の判断は妥当 |
| 古い前提のテストは 3 本 | `choice.rs` の 1 本、`steady_choice_tests.rs` の 1 本、`schedule_log_firing_tests.rs` の 1 本。ほかのクレートに `script:` の選択肢や `Unsupported` を前提にしたテストは無い | 正しい |
| 台帳の行は `degraded`・持ち主は空、根拠の行は kanade のソースにも置ける | `doc/ukadoc-coverage/ledger/sakura-script.toml`、`schedule/events.rs` の既存の根拠の行、`doc/ukadoc-coverage/README.md` の 3 章 | 正しい。根拠の行は「分岐の腕」に 1 行だけ・URL の後ろに語を続けない、という決まりにも合う |

## 要件の割り当て

要件 1.1〜1.7・2.1〜2.5・3.1〜3.3・4.1〜4.2・5.1〜5.3・6.1〜6.3・7.1〜7.4 の 27 項目すべてが、設計書の対応表で部品・テスト・文書のいずれかに割り当てられている。抜けは無い。

## 話し合いたい点（最大 3 件・どちらも進行を止めない）

### 1. 境目の ID の「記録」を、最上位の `step` から確かめるテストが無い（要件 7.3）

- **気になる所**: 要件 7.3 は、`script` だけ・`Script:`・`xscript:` の「扱いと、それぞれの記録」を確かめるよう求めている。設計のテスト 5 は判定の関数（`plan_cascade`）の戻り値だけを見ており、境目の ID を実際に選んだときに `script:` 用の記録が出ないこと・通常の選択肢のイベントが出ることは「完了済みのテストが持つ」として確かめ直さない。
- **なぜ効くか**: 本仕様の後、`script:` の綴りの判定は `body` の 1 か所に寄り、`plan_cascade` と `begin` の両方がそれを読む。判定の関数のテストだけでは、「境目の ID が `begin` に入らない」ことは間接にしか固定されない。
- **提案**: `choice_script_tests.rs` に 1 本足す。たとえば `Script:x` を選ぶと、返る一括が `OnChoiceSelectEx` の依頼 1 つで、`choice_script_started`・`choice_script_empty` が記録に無いこと。数行で済む。足さないなら、設計書に「境目の記録は受理の記録の `plan` の欄で足りる」と理由を 1 行書く。
- **該当**: 要件 7.3・設計書「Testing Strategy」のテスト 5。

### 2. 並走の約束の外で触る 4 つのファイルの了解

- **気になる所**: 設計書は、約束の外として `schedule/mod.rs`（説明のコメント 1 句）・`steady_choice_tests.rs` と `schedule_log_firing_tests.rs`（テストを各 1 本消す）・`areka-parsers` の `decode_tests.rs`（テストを 1 本足す）を挙げ、「開発者の調整が要る」としている。中身は妥当で、どれも数行だが、了解がまだ記録されていない。
- **なぜ効くか**: 要件 7.4（古い前提のテストを残さない）と要件 2.2・7.2（読みの確認）は、この 4 つに触らないと満たせない。`schedule/mod.rs` は 938 行で上限に近いが、句の書き替えだけなので行数は増えない。
- **提案**: 設計の話し合いで了解を取り、設計書の表にそのまま残す。断られた場合の代わりは、`mod.rs` のコメントだけ（後に入る側が直す）で、テストの 3 つは代わりが無い。
- **該当**: 設計書「Modified Files」の約束の外の表。

## 話し合い無しで直せる小さな点

- `choice.rs` の先頭の説明は「判断の分かれ道だけを置く・記録も出さない」層だと述べている。その子に状態を書き替えて記録を出すファイルをぶら下げるので、先頭の説明に一言足す（子の `script` は結末を持つ）。あわせて `plan_cascade` の説明の「規則 1 → `Unsupported`」「未対応カテゴリが最優先」「裁定 7」と、テスト `plan_cascade_canonical_for_near_miss_script_prefix` の「未対応ではない」という言い回しも今の動きに合わせる。設計書は結論の名前の説明の書き替えしか挙げていない。
- 受理の記録 `choice_accepted` の文言「カスケードを開始」は、`script:` のときは当たらない。文言だけ「選択確定を受理」に短くするか、そのままにするかを実装の段で決めればよい（event 名と欄は変えない）。
- `crates/areka/src/emo2_boot/spine_conformance_script.rs` の説明の中に `choice.rs:58-66` と行番号で指している所が 2 つあり、`choice.rs` に行を足すとずれる。説明の中だけの話でテストは壊れない。直すなら行番号でなく「`plan_cascade` の `On` 始まりの腕」と名前で指す（本仕様の範囲の外なので、触らずに申し送りでもよい）。
- `on_choice` の腕では、`begin` を先に呼んでから解決を作る（記録の順を設計書の表「実行の記録 → 解決の記録」どおりにするため）。設計書の事後条件から読み取れるが、タスクに一言あると取り違えない。
- 新しい警告 2 つ（`choice_script_empty`・`choice_script_unused_args`）の発火の確認は `choice_script_tests.rs` が持ち、`schedule_log_firing_tests.rs` には置かない。置き場が 2 つに分かれることをタスクに書いておく。
- `doc/choice-cascade-compat.md` には 7a-i を指す根拠の表の行（下の方の表）もある。7a の 2 行を書き替えるときに、その行が今の内容と食い違わないかを見る。

## 良い点

- **既存の形をそのまま使い、新しいものを作らない**。台本ありは「イベントが台本を返したとき」、空は「何も返さなかったとき」と同じ一括・同じ書き込みになるので、終了の保留・切替の保留・中断・切れ目の見張りとの重なりに新しい取り決めが要らない。ソースを読んでも、その腕が触る欄は設計書の言うとおり 3 つだけだった。
- **解決を作る場所を 1 か所のままにした**。新しいファイルは再生開始だけを返し、解決は今までどおり `steady.rs` の 1 つの関数が作る。「1 回の選択につき解決は高々 1 回」が、作る場所が 1 つであることで保たれ続ける。翻訳に通さないことも、何も足さずに既存の規則だけで成り立つ。

## 次の一歩

- 上の 2 件を設計の話し合い（`/kiro-design-discussion areka-P0-choice-script-prefix`）で決める。
- その後 `/kiro-spec-tasks areka-P0-choice-script-prefix` でタスクを作る。小さな点はタスクに織り込めば足りる。
