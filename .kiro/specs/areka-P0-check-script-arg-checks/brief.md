# Brief: areka-P0-check-script-arg-checks

> 2026-10-10 棚卸㉓で、roadmap の覚え書き「`check_script` が診ない引数の誤り」（2026-10-05 `mcp-author-tools` の要件ディスカッションで登記）から起票した。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

MCP のツール `check_script` は、台本を再生せずに「areka で効くかどうか」を確かめる道具。けれど `\!` については「その名前と第 1 引数を誰かが拾うか」までしか診ない。拾った後の引数の誤り（`\![move]` の読めない座標・`\![change,ghost]` の名前なし・`\![execute,install,…]` の相対パス・`\f[bold,abc]` の値の誤りなど）は、再生すると警告を 1 行残して何も起きないのに、`check_script` は診断を 1 件も返さない。作者は「診断が出ない＝効く」と受け取りやすく、再生してはじめて動かないことに気付く。

## Current State

- 判定の本体は `diagnose`（`crates/areka/src/mcp/check_script_judge.rs`・196 行）。判定を自分では持たず、再生が実際に使う関数の答えを診断の種類へ写すだけの作り。`\!` は対応表 `ConsumerLedger::consumer_of` を名前と第 1 引数で引き、誰も拾わなければ `unknown_command`。拾う者がいれば、`set,choicetimeout` を除いて引数は見ない。`\![move]` も表を引くだけ。
- `\f` は同じファイルの `judge_font` が `apply_font_tag`（`crates/areka-emo-text/src/look.rs`）を使い捨ての見た目に当てる。値の誤り（`FontTagFailure::BadValue`）は答えとして返ってきているのに、`judge_font` が捨てている（「値の誤りは診ない」と決めた腕）。
- 診ていないタグの一覧は `doc/ssp-mcp/areka-tools.md` の「⑷ 診ていないもの」（`mcp-author-tools` が書いた）。
- **覚え書きの「判定が受け口のファイルに散らばりログ出力と一体」は、半分だけ当たっている**。受け口ごとに今の姿を確かめた:

| タグ | 引数を読む所 | 今の形 |
|---|---|---|
| `\![move,…]` | `parse_move_directive`（`crates/areka/src/emo2_boot/move_cue.rs`） | 純粋な関数が既にある（読めなければ `MoveDegradation`・縮退して受ける分は `m1_degradations`）。受け口 `MoveCueSink` は答えを `warn!` にするだけ |
| `\![execute,install,…]` | `script_request`（`crates/areka/src/install/judge.rs`） | 純粋な関数が既にある（断りは `ScriptRefusal` の 5 通り）。受け口 `InstallCueSink` が腕ごとに `warn!` |
| 更新の 3 つ | `parse_update_command`（`crates/areka/src/emo2_boot/update_cue.rs`） | 純粋な関数が既にある（断り `Refusal` と、読み飛ばした指定の列を返す） |
| `\![set,zorder,…]` | `parse_zorder_tokens`（`crates/areka/src/placement/zorder_group_ledger.rs`） | 純粋な関数が既にある（断り `ZOrderReject`）。受け口 `ZOrderCueSink` は中身を読まずに運ぶだけ。「その窓が今あるか」は画面の状態が要る |
| `\![bind,…]` | `parse_bind_directive`（`crates/areka-seriko/src/bind.rs`） | 純粋な関数が既にある（形の誤りは `BindDirective::Malformed`）。名前が今のシェルに在るかは事実が要る |
| `\f[…]` の値 | `apply_font_tag`（`crates/areka-emo-text/src/look.rs`） | 純粋。`check_script` の側が答えを捨てている |
| `\_l[x,y]` | 軸ごとの読み `CursorCoord`（`crates/areka-emo-text/src/state.rs`）と `resolve_cursor_axis`（同 `cursor_tag.rs`） | 数として読めない・寄せの語の軸違いは値で返る（`CursorDegrade`）。字面だけで呼べる形かは着手時に確かめる |
| `\![change,ghost,…]` | `ChangeCueSink` の `emit`（`crates/areka/src/emo2_boot/change_cue.rs`） | 判定（名前なし・知らない option）が受け口の中にあり、`warn!` と一体 |
| `\![change,shell\|balloon,…]` | `SwitchCueSink` の `emit`（同 `switch_cue.rs`） | 同じく受け口の中 |
| `\![open,…]`・`\j[…]` | `ReadmeCueSink` の `emit`（同 `readme_cue.rs`） | 同じく受け口の中 |

＝取り出しが要るのは下の 3 つだけで、上の 7 つは「既にある関数を `diagnose` から呼ぶ」だけで足せる見込み。

- 診断の種類は 6 つ（`crates/areka-mcp/src/tools/check_script.rs` の `Kind`）。引数の誤りに使えるのは `unreadable_argument`（今は「閉じていない括弧」「既定の値へ落とした引数」の 2 つの文面）と `ignored`。
- 表と受け口が合っていることを固定するテストは `crates/areka/src/emo2_boot/consumer_ledger_agreement_tests.rs`（受け口を本物で組んで回す形）。

## Desired Outcome

- 上の表のタグについて、字面だけで決まる引数の誤りを `check_script` が返す。
- 再生と検査が同じ関数を通る（検査だけの別の判定を作らない）。
- 受け口が再生のときに出す記録（文・水準・欄）は変わらない。
- `doc/ssp-mcp/areka-tools.md` の ⑶（種類と文面）と ⑷（診ていないもの）が実物と合う。

## Approach

2 段で進める。⑴ 既に純粋な関数がある 7 つを `diagnose` から呼び、答えを診断へ写す。⑵ 受け口の中に判定が埋まっている 3 つ（ゴーストの切替・シェルとバルーンの切替・開く系）は、判定を「理由を値で返す関数」へ取り出し、受け口はその値を今と同じ文で記録する。字面で決まらないこと（窓が在るか・名前がシェルに在るか・ファイルが在るか）は診ないまま、⑷ に残す。

## Scope

- **In**: 上の表の 10 のタグの引数の診断・3 つの受け口からの判定の取り出し・`areka-tools.md` の ⑶ ⑷ の更新・決定論テスト（誤りごとの赤と緑、再生と同じ関数を通ることの固定）。
- **Out**: 再生中の誤りの記録（`mcp-strict-errors`）・影響の段を答えに足すこと（`script-impact-tiers`）・`%` の変数・切替の後ろの `\s`／`\b`・`\![raise]` の先の台本・設定ファイルの検査（`config-parse-diagnostics`）。

## Boundary Candidates

- 診断へ写す所（`crates/areka/src/mcp/check_script_judge.rs` と兄弟のテスト）。
- 判定の取り出し（`crates/areka/src/emo2_boot/{change_cue,switch_cue,readme_cue}.rs` と各兄弟のテスト）。
- 文書（`doc/ssp-mcp/areka-tools.md`）。

## Out of Boundary

- 対応表 `consumer_ledger.rs`（行は足さない）。
- 既に純粋な関数の中身（`parse_move_directive` ほか。呼ぶだけで変えない）。
- kanade・台本を読む段（`areka-parsers` の `sakura/`）。

## Upstream / Downstream

- **Upstream**: なし（`mcp-author-tools` は完了済み）。
- **Downstream**: `mcp-strict-errors`（再生中の記録を同じ種類と文面に揃える。種類や文面を足すなら正本が変わる）・`script-impact-tiers`（`check_script` の答えに段を足す）。

## Existing Spec Touchpoints

- **Extends**: なし（`mcp-author-tools` は完了済みで消化できない＝新しい spec）。
- **Adjacent**: `mcp-strict-errors`・`script-impact-tiers`（どちらも `check_script_judge.rs` と `areka-tools.md` に触る）・`mcp-reload`（`change_cue.rs` を挙げている）・`popup-menu-residue`（`readme_cue.rs` を挙げている）。

## Constraints

- 検査だけの判定を持たない（`mcp-author-tools` の要件「再生と一致」を守る）。
- 受け口の記録を変えない。記録を固定している既存のテスト（`move_cue_move_severity_log_tests.rs` など）は書き換えずに緑のまま。
- 誤って「無い」と答えない。字面で決まらないものは診ない。
- `check_script` は 10 秒の上限の中で答える（今の作りのまま・台本の長さに比例する計算だけを足す）。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**: `crates/areka/src/mcp/check_script_judge.rs`（196）・`check_script_judge_tests.rs`（313）・`check_script_tests.rs`（281）・`crates/areka/src/emo2_boot/change_cue.rs`（119）・`switch_cue.rs`（123）・`readme_cue.rs`（140）と各兄弟のテスト・`doc/ssp-mcp/areka-tools.md`（160）。種類を足すと決めたときだけ `crates/areka-mcp/src/tools/check_script.rs`。`move_cue.rs`（683）・`update_cue.rs`・`install/judge.rs`・`zorder_group_ledger.rs`・`areka-seriko`・`areka-emo-text` は呼ぶだけで触らない見込み（`\_l` の読みを字面だけで呼べないと分かったときは `areka-emo-text` の `cursor_tag.rs` に触る）。
- **規模**: M（8〜12 タスク）。既にある関数を呼ぶだけの 7 つは軽く、取り出しの 3 つと文書と一致のテストが残り。
- **先に要るもの**: なし（今すぐ着手できる）。同じウェーブに置けない相手: `mcp-strict-errors`・`script-impact-tiers`・`anchor-tag-canon`・`anchor-style-canon`・`text-align-shadow-canon`・`sakura-time-directives`・`talk-fast-forward`（どれも brief が `check_script_judge.rs` を挙げている）・`mcp-reload`（`change_cue.rs`）・`popup-menu-residue`（`readme_cue.rs`）。
- **優先度の区分**: C（持ち越し。動かないものを「動く」と誤って答えるのではなく、診断が出ないだけで、文書が「診ない」と明記している＝バグではない）。
- **要件定義のモデル**: Opus（判断の分かれ目は下の議題 2 つで、手本は `mcp-author-tools` にある）。
- **議題**:
  1. どの受け口から足すか。推しは「既に関数がある 7 つを先に、取り出しの 3 つを同じ spec の後半に」。3 つを別の spec に切る案もある（`mcp-reload`・`popup-menu-residue` と重なるのは後半だけ）。
  2. 受け口の記録を同じに保つ方法。推しは「取り出した関数が理由を値で返し、受け口は今の文のまま記録する」。記録の文を固定するテストが無い受け口（`change_cue`・`switch_cue`・`readme_cue`）は、取り出す前に今の記録を固定するテストを足すかどうか。
  3. 縮退して受けるもの（`\![move]` の時間つき・`install` の読まない後ろの引数・更新の読み飛ばした指定）を `unreadable_argument` と `ignored` のどちらで返すか、文面を足すか。足すと `mcp-strict-errors` が揃える正本（`areka-tools.md` の ⑶）が変わる。
- **覚え書きとの違い**: 覚え書きは「判定を純粋な関数へ取り出せば足せる」と書くが、10 のうち 7 つは既に純粋な関数になっている。ukadoc の照合は不要（areka の道具の話で、正典の主張を含まない）。
