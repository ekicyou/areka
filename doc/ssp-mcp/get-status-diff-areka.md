# SSP と areka の `get_status` の差（2026-10-06 時点）

ゴーストの今の実行の状態を返す `get_status` について、SSP と areka で答えが違う場合、または SSP の答えをまだ測っていない場合を行ごとに並べる。書式（ukadoc `Status [SSP拡張]` の語をカンマでつなぐ・何も無ければ空の本文・`isError: false`）と、宛先が見つからないときの文言（`NG:Cannot find active ghost from specified name`）のように SSP と同じものは載せない。輸送の差は [transport-diff-areka.md](transport-diff-areka.md) にある。

- **SSP の印**: 「実測」は SSP で測ったもの（2.9.05 の実測は [survey.md](survey.md)）。「未実測」は SSP で測っていないもの。「対応物なし」は SSP にその仕組みが無いもの。SSP の旗の一覧は、ツールの説明文（[tools-list-ssp-2.9.05.json](tools-list-ssp-2.9.05.json) の `get_status`）による
- **areka の根拠**: 答えを返す処理は `crates/areka/src/mcp/get_status.rs` の `handle`。語を決めるのは `crates/areka-kanade/src/status.rs` の `ExecutionStatus::derive`（SHIORI へ送る要求の `Status` の値と同じ作り方）

| 項目 | areka | SSP | SSP の印 |
|---|---|---|---|
| 旗 `changing` | 出さない。ukadoc `Status [SSP拡張]` の語の一覧に無いため | ツールの説明文の旗の一覧に名前がある（ゴーストの切替中と見られる）。いつ出るかは測っていない | 未実測 |
| `minimizing`・`induction`・`passive`・`timecritical`・`opening(…)` | 出さない。areka にこれらの状態の出どころがまだ無く、状態を作り出さない | ツールの説明文の旗の一覧に名前がある。いつ出るかは測っていない | 未実測 |
| ゴーストの切替の途中に届いた呼び出し | 状態の語でなく、宛先が居ないときと同じ `NG:`（`ghost_name` を渡していれば `NG:Cannot find active ghost from specified name`、省略なら `NG:Specified ghost is not active`）・`isError: true` で答える。切替の途中はゴーストの置き場が空になるため | 測っていない（`changing` の旗を返すのかもしれない） | 未実測 |
| 各旗の出る条件 | kanade の状態の決め方のとおり。普段の会話・起動の挨拶の再生中は `talking`、選択肢を待つ間は `choosing`、中断の無効化モードで再生中は `nouserbreak`、ネットワーク通信中は `online`、見えているバルーンがあれば `balloon(…)` | 測れた答えは `balloon(0=0/1=0/2=0)` と `talking,balloon(0=0/1=0)` の 2 例だけ。ほかの旗がどの場面で出るかは測っていない | 未実測 |
| ゴーストの SHIORI が考えている間に届いた呼び出し | SHIORI の答えが返ってから答える（kanade は SHIORI との往復の間、ほかの知らせを処理しない）。10 秒を超えれば待ちの上限の `NG:areka did not respond within 10 seconds` | 測っていない | 未実測 |
| 終了の挨拶・切り替えのお別れの台詞の再生中 | `talking` が出ない（kanade の今の決め方が、普段の会話と起動の挨拶の再生中だけを「再生中」と数えるため）。直すのは `farewell-talk-status` | 測っていない | 未実測 |

## 補足

- 宛先は見つかったのに、そのゴーストの kanade への問い合わせ先が置き場に無いときの `NG:Status is not available`（`isError: true`・`warn!` 1 件）は、本番では起きないはずの areka の内部の食い違いの知らせで、SSP に対応物は無い。
