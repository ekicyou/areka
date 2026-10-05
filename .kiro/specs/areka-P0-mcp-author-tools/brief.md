# Brief: areka-P0-mcp-author-tools

> 起票: 2026-10-05 `/kiro-discovery`（開発者「エージェント目線でこんなツールがあったらいいな」「すそ野を広げる」）。開発者の方針（同日）: 「さくらスクリプトを知らないエージェントは無い・スキルでもカバーできる。SSP で喋らせる機能がさくらスクリプトに限定されているのは、さくらスクリプトでできることが強力だから。さくらスクリプトでもできるツールは優先度を下げる」。

## Problem

伺かの作者が AI エージェントと一緒にゴーストを作ろうとしても、エージェントが自分の書いたものを確かめる道が無い。

- **台本を確かめられない**: 再生しないと、知らないタグや存在しない surface に気付けない。再生すると、ゴーストが実際に喋ってしまう。
- **使えるタグが分からない**: areka が対応しているタグと `\!` が分からないので、エージェントは ukadoc の全タグを前提に書いてしまう。
- **設定ファイルを検査できない**: descript.txt・surfaces.txt・balloon の descript は、読み込ませて壊れるまで誤りが見えない。

どれもさくらスクリプトでは届かない（台本は実行することしかできない）。SSP の MCP にも無い。AI と作る作者が増えれば、ゴーストの数が増え、すそ野が広がる。YAYA のテンプレート「紺野ややめ」がエージェント向けの開発キットを同梱しているのも同じ流れ。

## Current State

- `areka-mcp` の SSP 互換の 10 本は、SSP との逐語一致をテストで固定している（`crates/areka-mcp/src/tools/tools_tests.rs` の `TABLE.len()==10` と `doc/ssp-mcp/tools-list-ssp-2.9.05.json` の照合）。**新しいツールはこの表に足せない**。
- 本体への橋（`ToolCall` の enum・`crates/areka/src/mcp/mod.rs` の `dispatch`）も 10 変種に固定されている。登録口 `ToolRegistry::register` は別の表を受け付ける（`main.rs` で受け取る registry）。
- 部品はそろっている。
  - 台本の解釈器（`areka-sakura`・`areka-talk`）。
  - 純粋な設定の読み取り（`areka-parsers` の shell・balloon・package）。
  - `\!` の対応表（`emo2_boot/consumer_ledger.rs`）。
  - 網羅台帳（`doc/ukadoc-coverage/ledger/*.toml`）。

## Desired Outcome

- **areka 独自のツールの登録口**: SSP 互換の 10 本を変えずに、別の表と本体への橋を持つ。後続（`mcp-user-response` など）もここへ足す。
- **`check_script`**（仮名）: 台本を再生せずに解釈し、次のものを返す。
  - 知らないタグ・不正な引数。
  - 今のシェルに無い surface・アニメーション、無いバルーン。
  - （`script-impact-tiers` が着地していれば）影響の段。
  - 返す形は、`strict` のエラーログの文言と揃える。
- **`list_capabilities`**（仮名）: areka が対応するタグ・`\!` コマンド・SHIORI イベントと、それぞれの状態（実装・縮退・未実装）を返す。網羅台帳と対応表から作る。
- **`validate_ghost`**（仮名）: 指定したゴースト（またはフォルダ）の descript・surfaces・balloon を検査して、診断を返す。読むだけ。

## Approach

- 独自の表は `areka-mcp` に別の定義の表として置く。SSP の表と、それを固定するテストには触らない。
- 本体への橋を独自のツール向けに広げる（汎用の要求の型か、別の受け口かは設計で決める）。
- ツール名の接頭辞（SSP が将来同じ名前を足したときの衝突を避ける `areka_` など）は要件の議題。起票時の推しは「付ける」。
- 3 本とも読むだけで、kanade の再生を要さない。`mcp-kanade-tools` を待たずに着手できる。

## Scope

- **In**:
  - 独自ツールの登録口と橋。
  - 上の 3 本。
  - サーバーの指示文（`INSTRUCTIONS`）への独自ツールの案内。
  - help の更新。
- **Out**:
  - 台本を再生するツール（`sakurascript` で足りる）。
  - 喋らせる・表情を付ける・ゴーストを切り替える・インストールするツール（さくらスクリプトでできる＝優先度を下げた）。
  - 画像を出すツール（`mcp-dump-images`）。

## Boundary Candidates

- 登録口と橋（`areka-mcp` の共有ファイル・`crates/areka/src/mcp/mod.rs`）。
- 3 本それぞれの中身（解釈器・設定の読み取り・台帳）。

## Out of Boundary

- SSP 互換の 10 本の定義と振る舞い。
- 台帳そのものの正しさ（`coverage-roadmap-refresh`）。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`（着地済み）。
- **Downstream**: `mcp-user-response`（登録口を使う）・エージェント向けのスキル・文書。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `mcp-strict-errors`（`check_script` の診断の文言を揃える）。
  - `mcp-ghost-name-match`（`resolve.rs`）。
  - MCP の共有ファイル（`handler.rs`・`registry.rs`・`tools/mod.rs`・`tools/bridge.rs`・`crates/areka/src/mcp/mod.rs`）を触る。共有ファイルを触る MCP の spec とは同じウェーブに置かない。

## Constraints

- SSP 互換の 10 本は逐語一致を保つ（`doc/ssp-mcp/` が正本）。
- MCP は rmcp で、無状態・JSON 単発。返事の待ちは最長 10 秒。
- 規模の見立て: M〜L（14〜20 タスク・超えたら `validate_ghost` を切る）。
</content>
</invoke>
