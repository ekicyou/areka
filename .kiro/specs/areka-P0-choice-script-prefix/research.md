# ギャップ分析: areka-P0-choice-script-prefix

> 2026-10-05 `kiro-validate-gap`。入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md` の kanade の列）。
> 本書は分析と選択肢を並べるもので、最終の決定は設計の段で行う。

## 1. 要約

- **埋める穴は 1 か所**: `plan_cascade`（`crates/areka-kanade/src/schedule/choice.rs`）が `script:` を `CascadePlan::Unsupported` にし、`on_choice`（`schedule/steady.rs` の受理の後の `match plan` の最初の腕）が `choice_unsupported_category`（warn）を出して `ResolveChoice`（理由 `"unsupported"`）を返すだけになっている。ここを「台本を新しいトークとして始める」に替える。
- **使い回せる型がすでにある**: 選択肢の SHIORI イベントが台本を返したときの道（`on_cascade_reply` の `Value` の腕）が、まさに要件 1.3／1.4 の形＝「新しい talk_id を採番 → 枠を差し替え → 旧 talk_id を 1 世代控える（`choice_prev_talk`）→ `[ResolveChoice{旧}, StartTalk(新)]` を同じ一括で返す」を持っている。新しい再生の仕組みは要らない。読み込み（`"…"` の括り・`""`・`\]`）も要件の段の確認どおり正典どおりで、手を入れる所は無い。
- **一番大きな分かれ目は翻訳（`OnTranslate`・MAKOTO）**: SHIORI の応答の台本は `schedule/translate.rs` の出口の規則で `OnTranslate` に通るが、その規則は「入力が SHIORI の応答で結果が台本」のときだけ働く。選択の入力（`Input::Choice`）から出す `StartTalk` は翻訳に通らない。要件 1.5「SHIORI の応答の台本と同じ読み方・同じ再生の結果」とどう合わせるかを決める必要がある（下の議題 1）。
- **置き場の約束は守れる**: 新しいファイルを `choice.rs` の子（`#[path]` の宣言を `choice.rs` に置く）として作れば、`schedule/mod.rs`・`lib.rs`・`msg.rs`・`actor.rs` に触らずに済む。`steady.rs`（947 行）は腕を呼び出しに付け替えるだけで行数は減る見込み。
- **規模 S・危険度 低**（既存の型の写し・純粋な判定と状態機械のテストで固定できる）。注意は翻訳の扱い・解決の発行点の一本化・後の `mcp-kanade-tools` との口の形の 3 点。

## 2. 現状の調べ

### 2.1 関係するファイルと役目

| 場所 | 今の役目 | 本 spec との関係 |
|---|---|---|
| `crates/areka-parsers/src/sakura/lexer.rs`（角括弧の引数の走査） | `"` から次の `"` までを 1 引数（間の `,`・`]` は区切らない）・`""`→`"`・角括弧の中の `\]`→`]` | 正典どおり。変更なし（要件の段の確認と一致） |
| `crates/areka-parsers/src/sakura/decode.rs` の `decode_choice` | 第 1 引数＝`disp`・第 2＝`target`・残り＝`references` を加工せず転記 | 変更なし。`script:` の ID もそのまま届く |
| `crates/areka/src/input_events/choice_drain.rs` の `to_choice_input` | 選択の通知を `ChoiceInput{id,label,scope,references}` へ加工せず写す | 変更なし。第 3 引数以降は `references` に入る（要件 3.3 の数の源） |
| `crates/areka-kanade/src/schedule/choice.rs`（346 行） | 純関数 `plan_cascade`（3 分岐）・`choice_deadline` とそのテスト | `Unsupported` の枝の意味を替える。新しい子のファイルの宣言を置く |
| `crates/areka-kanade/src/schedule/steady.rs`（947 行） | `on_choice`（受領の検証 → 受理の記録 `choice_accepted` → `plan` で分岐）・`on_cascade_reply`・`resolve_choice`（`choice_resolved` を出す唯一の発行点・非公開） | `Unsupported` の腕（313〜326 行）を新しいファイルの呼び出しに付け替える |
| `crates/areka-kanade/src/schedule/mod.rs` | `State`（`phase`・`next_talk_id`・`choice`・`choice_prev_talk` などの欄は `pub`）・`ActiveTalk{talk_id, origin: &'static str, script}`・`Action::{StartTalk, ResolveChoice, CancelChoice}`・`clear_choice_ledger`・`step`（`translate::before/after` と `talk_gap::observe` で `route` を挟む） | 触らない（C4 の約束）。欄が `pub` なので子のファイルから状態を書ける |
| `crates/areka-kanade/src/schedule/translate.rs` | SHIORI の応答の台本の再生開始を預けて `OnTranslate` へ回す出口の規則（`capture`） | 選択の入力では働かない＝`script:` の台本は今のままだと翻訳に通らない |
| `crates/areka-ghost/src/dispatcher.rs` | `ResolveChoice`・`StartTalk` を受け、枠の差し替え時は旧トークを閉じてから新トークを始める（Close-then-spawn）・古い `TalkDone` は捨てる | 変更なし。カスケードの `Value` と同じ一括の形なら同じ動き |
| `crates/areka-sakura/src/drive.rs` | 台本の読み込み → 選択肢の待ち（`ChoiceWaiting` の通知・候補 ID の列） | 変更なし。新しいトークの中の `\q` もいつもどおり待ちに入る |

### 2.2 使い回せる既存の型（3 か所に同じ形がある）

`TalkId(state.next_talk_id)` の採番 → `state.next_talk_id += 1` → `state.phase = Steady{talk: Some(ActiveTalk{..})}` → `Action::StartTalk(StartTalk::new(id, script))` の 4 手は、`steady.rs` の次の 3 か所に**複写で**ある。

1. `on_cascade_reply` の `Value`（選択の SHIORI イベントが台本を返した）＝`[ResolveChoice{旧}, StartTalk(新)]`・`choice_prev_talk = Some(旧)`・記録 `steady_talk`（info）。**要件 1.4 が名指しする道**。
2. `on_timeout_reply` の `Value`＝`[StartTalk(新)]` のみ（解決は出さず、旧トークは dispatcher が閉じる）・`choice_prev_talk = Some(旧)`。
3. `on_reply` の `talk: None`／置換の腕（pump・マウス・汎用の入口）＝置換のときは `clear_choice_ledger` と `choice_prev_talk = None`。

`script:` の実行は 1 の形をそのまま写せば要件 1.3（待ちを閉じるのと新トークの開始が同じ一括）・1.4（置き換え）・3.2（空なら `ResolveChoice` だけ＝カスケード終端の 204 と同じ）を満たす。

### 2.3 規約・制約

- **テストの置き場**: 新しい本番ファイルのテストは兄弟の `<stem>_tests.rs` に置き、本番ファイルには `#[cfg(test)] #[path = "…"] mod tests;` だけを残す（`structure.md`）。`steady_choice_tests.rs`（732 行）・`schedule_log_firing_tests.rs`（723 行）は 1,000 行の目安の内だが、足す分は新しいファイル側の兄弟に置くのが素直。
- **記録の段**: 受理は `choice_accepted`（info）・解決は `choice_resolved`（info）。要件 5.1「受け付けた記録と同じ段」＝info。空の `script:` は warn（要件 3.1）。
- **網羅台帳**: 状態を `implemented` にするときは `owner` に spec 名を書き、根拠として定義行に `// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1` の 1 行を置く（`doc/ukadoc-coverage/README.md`「1 項目 1 行」）。台帳を直したら `cargo run -p ukadoc-survey -- report`（と `report-summary`）で報告を作り直し、`cargo test -p ukadoc-survey` を通す（報告は手で直さない）。
- **`Cargo.*` に触らない**: 初回リリースの条件（`Cargo.*` を触る開いた PR 0 本）に掛かるため、テストのために依存を足す案は取らない。`areka-kanade` は `areka-parsers` に依存していない（`Cargo.toml` の `[dependencies]` は `areka-actor`・`areka-talk` など）。

## 3. 要件と既存の資産の対応

| 要件 | 既存の資産 | ギャップ |
|---|---|---|
| 1.1 `script:` の後ろを新トークで再生 | カスケードの `Value` の道（§2.2-1） | **Missing**: `Unsupported` の腕を「採番・差し替え・`StartTalk`」に替える処理 |
| 1.2 SHIORI のイベントを起こさない | `Unsupported` の腕はすでにイベントを出さない | なし（形を保つ） |
| 1.3 待ちを同じときに閉じる | `on_choice` が帳簿を `take()` したまま戻さない・`ResolveChoice` を同じ一括に載せる型 | なし（型を写す） |
| 1.4 元トークを置き換える | dispatcher の Close-then-spawn・`choice_prev_talk` の 1 世代の控え | なし（型を写す）。控えを忘れると旧トークの遅れた `TalkDone` が `unknown_talk_done`（error）になる＝**Constraint** |
| 1.5 応答の台本と同じ読み方・再生の結果 | 読み込み・再生は `StartTalk` の台本文字列から同じ道 | **Unknown／Constraint**: `OnTranslate`・MAKOTO を通すか（議題 1）。展開（`%…`）は再生時にも掛かるので差は翻訳と MAKOTO だけの見込み（要確認・§7） |
| 1.6 小文字の `script:` だけ | `plan_cascade` の `starts_with("script:")`・境目のテスト（`script`・`Script:x`・`xscript:y`）が既にある | なし |
| 2.1 `\e` で閉じる | `\e` は新トークの再生で効く | なし（テストだけ） |
| 2.2 入れ子の括りの読み | lexer の `"` の扱い | **Missing（テストのみ）**: 正典の記述例そのものの読みのテストが無い（`decode_tests.rs` に `その１` の例は無い）。置き場は議題 6 |
| 2.3・2.4 その１ → その２ → 表示 | kanade は台本文字列を渡すだけ・待ちは sakura が作る | **Missing（テストのみ）**: kanade の状態機械で 2 段の選択を通すテスト |
| 2.5 開く系のタグ | 汎用の `\!` の運び手 | なし（`open-external-tags` の持ち分。本 spec は流すだけ） |
| 3.1 空の `script:` | `resolve_choice` の型 | **Missing**: 空の判定・warn の記録 |
| 3.2 空なら元トークを続ける | カスケード終端 204＝`[ResolveChoice]` のみ | なし（型を写す） |
| 3.3 余分な引数は使わず数を記録 | `ChoiceInput.references` に入っている | **Missing**: 数の記録（実行の記録の欄に足すか、別の記録か） |
| 4.1・4.2 出どころは元と同じ | 出どころの印はまだ無い（`script-security-level` の持ち分）・`ActiveTalk.origin` は書くだけで読まれない記録用のラベル（`lib.rs` の `#[allow(dead_code)]` の注記） | **Constraint**: 今は「他のトークと同じ 1 つの入口から始める」ことだけが要る。`origin` に何を書くかは議題 4 |
| 5.1 実行の記録 | `choice_accepted`・`steady_talk`・`choice_resolved` | **Missing**: 選択肢の ID・新 talk_id・旧 talk_id を 1 件に持つ info の記録（新しい event 名か `steady_talk` の流用か） |
| 5.2 未対応の警告を出さない | `choice_unsupported_category` | 撤去（語彙の後始末は議題 3） |
| 5.3 黙って終わる道を持たない | 2 つの結末とも記録がある形にする | 設計で全分岐を表にする |
| 6.1〜6.3 台帳・互換の記録 | `sakura-script.toml` の該当行（`degraded`・owner 空・A1）・`doc/choice-cascade-compat.md` 7a-i／7a-ii・`doc/COMPAT_ARCHITECTURE.md` §選択肢の行 | **Missing**: 3 文書の書き替えと報告の作り直し・`// ukadoc:` の根拠の行 |
| 7.1〜7.3 決定論のテスト | `steady_choice_tests.rs` の `steady_with_ledger`・`choice_input_of`、`log_capture` | **Missing**: 新しい兄弟のテストファイル |
| 7.4 古い前提のテストを残さない | `choice.rs` の `plan_cascade_unsupported_for_script_prefix`・`steady_choice_tests.rs` の `unsupported_choice_resolves_without_emitting_any_event`・`schedule_log_firing_tests.rs` の `warn_choice_unsupported_category_logs` | 3 本を書き替えるか消す（議題 3 と連動） |

## 4. 実装の選択肢

### 案 A: 既存のファイルに足す（`steady.rs` の `on_choice` の腕の中で実行）

- 中身: `Unsupported` を `Script` に改名し、`on_choice` の腕に採番・差し替え・記録を直接書く。
- ✅ 新しいファイル 0・カスケードの `Value` と並んで読める。
- ❌ `steady.rs` が 947 行で上限 1,000 の手前。約 30〜40 行増えると上限を超える見込み。
- ❌ C4 の約束「`on_choice` は呼び出しの付け替えだけ」に反する。
- ❌ 後の `mcp-kanade-tools` の `sakurascript` が使い回す口にならない。
- **評価**: 約束に反するので実質不可。比較のために挙げる。

### 案 B: `choice.rs` の子の新しいファイルに実行をまるごと置く

- 中身: `schedule/choice_script.rs`（名前は仮）を作り、`choice.rs` に `#[path = "choice_script.rs"] pub(super) mod script;`（または同等）を置く。新しいファイルは
  1. 純関数: ID から `script:` の後ろを取り出す・空かどうか（`plan_cascade` と同じく副作用なし）。
  2. 実行: `State` を受けて採番・差し替え・`choice_prev_talk` の控え・記録を行い、`[ResolveChoice, StartTalk]` か `[ResolveChoice]` を返す。
  を持つ。`steady.rs` の腕は `return choice::script::run(state, talk_id, input);` の 1 行に替える。
- ✅ 約束どおり（触るのは `choice.rs`・`steady.rs` の腕・新しい子だけ）。`steady.rs` は約 10 行減る。
- ✅ テストを兄弟の `choice_script_tests.rs` に閉じられる。
- ❌ `ResolveChoice` の発行点が 2 か所になる（`steady.rs` の `resolve_choice` は非公開なので、子のファイルが自前で `Action::ResolveChoice` と `choice_resolved` の記録を組むことになる）。`Action::ResolveChoice` の説明の「発行点は単一化されている（1 選択＝高々 1 解決・Req5.4）」と食い違う＝議題 2。

### 案 C: 混成（判定と台本の始め方は子のファイル・解決の発行は `steady.rs` に残す）

- 中身: 子のファイルは「台本を新しいトークとして始める」芯（採番・差し替え・`StartTalk` を返す）と、`script:` の判定（本体を取り出す・空・余分な引数の数）を持つ。`steady.rs` の腕は子の結果を受け取り、既存の `resolve_choice` で解決を組んで一括に並べる（例: `vec![resolve_choice(..), start]`）。
- ✅ 解決の発行点が `resolve_choice` 1 か所のまま。
- ✅ 「台本を新しいトークとして始める」芯を `pub(in crate::schedule)` で出せば、後の `mcp-kanade-tools` がそのまま呼べる（議題 5）。
- ❌ `steady.rs` の腕が案 B より数行長い（「付け替えだけ」の範囲かの解釈が要る）。
- ❌ 芯の置き場が「選択肢」の下になる。MCP から使うと名前と中身がずれる（後で `mcp-kanade-tools` が `schedule/mod.rs` を触るときに移す手もある）。

### 共通して要る後始末（どの案でも同じ）

- `CascadePlan::Unsupported` の名前と説明（「M1 未対応カテゴリ」「裁定 7」）・`plan_cascade` の判定規則の説明・`steady.rs` の `on_choice` の説明の `Unsupported` の行・`Action::ResolveChoice` の説明の「未対応カテゴリの即時解決」を新しい意味に合わせる。
- 古い前提のテスト 3 本（§3 の 7.4 の行）を書き替える。
- 台帳・`choice-cascade-compat.md`・`COMPAT_ARCHITECTURE.md` の更新と報告の作り直し。

## 5. 規模と危険度

- **規模: S（1〜3 日）** — 既存の型（カスケードの `Value`・終端の 204）を写すだけで、新しい再生の仕組み・新しい入力の型・新しい状態は要らない。テストと文書の書き替えが半分を占める。brief の見込み（5〜8 タスク）と合う。
- **危険度: 低** — 純粋な状態機械（`step`）のテストで分岐を全部固定でき、実機に依らない。残る不確かさは翻訳の扱い（議題 1）と、選ばれた直後の「解決 → 閉じる」の順（カスケードの `Value` と同じ一括なので既知の型の中）。

## 6. 設計の段で決めること（議題の候補）

1. **`script:` の台本を `OnTranslate`・MAKOTO に通すか**
   - 事実: 今の翻訳の出口の規則（`schedule/translate.rs` の `capture`）は「入力が SHIORI の応答で結果が台本」のときだけ働くので、`Input::Choice` から出す `StartTalk` は通らない。`script:` の中身は、選択肢を含んでいた元の台本の一部として**すでに 1 回** `OnTranslate` を通っている。ukadoc の `OnTranslate` は「主に、GET コマンドの SHIORI Event に対してベースウェアへスクリプトが返却された場合に、それに応じて発行される」と書き、`script:` の場合には触れていない（正典は沈黙）。Reference2（元のイベント ID）は「イベントが原因ではない場合欠番」。
   - 案 (a) 通さない（元の台本と一緒に翻訳済みとみなす）。新しいファイルだけで済む。正典の沈黙を areka の決めごととして `choice-cascade-compat.md` に記す。
   - 案 (b) 通す（Reference2・3 は欠番）。`translate.rs` の出口の規則は C4 の約束の外のファイルなので、子のファイルが `TranslateWait` を自分で組むか、約束を広げる必要がある。同じ問いを `mcp-kanade-tools` の brief が `sakurascript` について議題に挙げている（「MCP の台本を `OnTranslate` に通すか」）。
   - 要件 1.5 の「同じ再生の結果」の読み方（読み込みと再生の結果を指すのか、翻訳まで含むのか）もここで確定させる。
2. **解決（`ResolveChoice`）の発行点を 1 か所に保つか**（案 B と案 C の分かれ目）
   - `steady.rs` の `resolve_choice` は「1 選択＝高々 1 解決」を発行点の排他で保つ作り。子のファイルから使うには `pub(super)` へ広げる（`steady.rs` の 1 語の変更）か、腕で組み立てる（案 C）か、子のファイルに 2 つ目の発行点を作る（案 B）か。
3. **`CascadePlan::Unsupported` の扱い**
   - 改名（例 `Script`）して意味を替えるか、名前は残して説明だけ替えるか。`choice_unsupported_category` の記録の語彙と、`"unsupported"` の解決の理由の語を消すか（要件 5.2 は「出さない」だけを求める）。消すなら古いテスト 3 本は書き替えでなく置き換えになる。
4. **新しいトークの `origin`（記録用のラベル）と実行の記録の形**
   - `origin` に固定の語（例 `"choice_script"`）を入れるか、元のトークの `origin` を引き継ぐか。今は読まれない欄だが、`script-security-level` が出どころを運ぶときの手掛かりになる（要件 4.1 の「元と同じ」）。
   - 要件 5.1 の記録は新しい event 名（例 `choice_script_started`）を 1 件出すか、カスケードと同じ `steady_talk` に欄を足すか。要件 3.3 の余分な引数の数を同じ 1 件の欄に入れるか、別の記録にするか（別にすると 1 選択の記録が増える）。解決の記録 `choice_resolved` の理由の語（例 `"script"`・`"script_empty"`）。
5. **後の `mcp-kanade-tools`（`sakurascript`）が使い回す口の形**
   - 選択肢の道は必ず `Steady{talk: Some}` から始まり `choice_prev_talk = Some(旧)` を控えるが、MCP の道は `talk: None` からも始まり、置き換えるときは `clear_choice_ledger` と `choice_prev_talk = None` が要る（§2.2 の 3）。共有できるのは「採番・枠への書き込み・`StartTalk` を返す」芯までで、前後は呼び手ごとに違う。
   - 芯を本 spec で切り出して公開の範囲を `crate::schedule` にするか、本 spec は選択肢専用に作って口の形だけ設計書に書き、切り出しは `mcp-kanade-tools` に任せるか。既存の 3 か所の複写（§2.2）を芯に寄せるのは `steady.rs` を触るので本 spec の範囲の外。
6. **正典の入れ子の記述例の「読み」のテスト（要件 2.2・7.2）の置き場**
   - `areka-kanade` は `areka-parsers` に依存しないので、kanade の中では「選んだ ID 文字列 → 台本文字列」までしか確かめられない。読み（括りの中の `,` と `]`）を確かめる置き場の候補: (a) `crates/areka-parsers/tests/` に新しい統合テストのファイル（ソースも `Cargo.*` も触らない。統合テストの入口の規約〔`tests/{domain}.rs` は `#[path]` の宣言だけ〕に合わせる要あり）、(b) `crates/areka-sakura` の既存の選択のテスト（`drive_choice_tests.rs` は `areka_parsers::sakura::parse` を既に使い、候補 ID の列の通知まで確かめられる。ただし他の spec が触るファイルとの接触を確認する要あり）、(c) `areka-parsers` の `decode_tests.rs` に足す（`decode.rs` は次に `anchor-tag-canon` が触る・C4 の約束の文言の外）。

## 7. 調べが要ること（Research Needed）

- **展開（`%…`）の差の有無**: 翻訳の前の展開 `expand_system_vars`（`crates/areka-sakura/src/sysvar.rs`）が角括弧の引数の中の `%…` を置き換えるか。置き換えないなら `script:` の中の `%…` は新しいトークの再生時の展開で埋まり、`script:` だけの差は `OnTranslate`・MAKOTO に絞られる（議題 1 の判断材料）。
- **閉じ忘れた括りの台本の見え方**: 括らずに `\q[メモ帳を開く,script:\![open,file,notepad.exe]]` と書くと台本は `\![open` になり、角括弧が閉じないので読み込みの寛容な吸収（`BracketScan::Unclosed`）で文字として出る見込み。要件 1.5 により応答の台本と同じ扱いでよいが、要件 3.3 の記録（余分な引数 2 個）で気付けることを設計で一言書くとよい。
- **台帳の検査の条件**: `implemented` にしたときに `cargo run -p ukadoc-survey -- check` が根拠の行（`// ukadoc:` の URL）を求めるか・`introduced` の欄をどう書くか（既存の `implemented` の行は `introduced = ""`・`owner` に spec 名）。
- **保留中の終了・切替・中断との重なり**: `pending_close`・`pending_change`・`user_break_talk`・`talk_gap` は、カスケードの `Value` で新トークを始めるときと同じに振る舞う見込み（どれも枠の差し替えの前後で特別扱いをしていない）。設計で「カスケードの `Value` と同じ」と言い切れるかを表で確かめる。

## 8. 設計の段への申し送り

- 推す方向: **案 C（または議題 2 の答え次第で案 B）**＝`choice.rs` の子の新しいファイルに判定と台本の始め方を置き、`steady.rs` は腕の付け替えだけ。一括の形はカスケードの `Value`（`[ResolveChoice{旧}, StartTalk(新)]`・`choice_prev_talk = Some(旧)`）、空はカスケード終端の 204（`[ResolveChoice]`）に合わせる。
- 先に答えを出すもの: 議題 1（翻訳）。答えで触るファイル（C4 の約束の内か外か）が変わる。
- 設計書に一言書くもの: 議題 5（`sakurascript` が使い回す口の形）＝brief の C4 の約束。
