# Brief: areka-P0-mcp-log-history

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントは台本を流した後、「エラーが出たか」「何が再生されたか」をログで確かめる（SSP の `get_log`・`since_id` で差分だけ読む）。areka のログは標準エラーへ流れて消えるだけで、動いているアプリへ問い合わせて読めない。

## Current State

- ログは `tracing_subscriber::fmt()` だけ（`crates/areka/src/main.rs` 143〜148 行目付近）。独自の Layer・メモリ上の履歴・通し番号は無い。
- `log-capture-kit`（テスト専用・`publish = false`）の捕捉 Subscriber は形の手本になるが、アプリから依存してはならない。
- SSP の形（survey §3）: 種別 5 つ（error 既定／script／network／update／status）、1 行 `#<id> <yyyy/mm/dd hh:mm> [<種別>] <名> : <本文>`、`\r\n` 区切り、古い順、継続行はタブ始まり、0 件は `(no log entries)`、**id は全種別で通し**、`since_id`・`max_count`（新しい方から）・`ghost_name` で絞る。**SSP は script の本文の逆斜線を JSON でエスケープし損ねる**（survey §4-2）＝areka は正しく返す。

## Desired Outcome

- アプリの `tracing` の出来事のうち、5 種別に当たるものが通し番号付きでメモリに残り（上限あり・古いものから捨てる）、`get_log` で SSP と同じ書式で読める。
- **種別への振り分けの規則**（error＝warn 以上＋strict の記録、script＝再生した台本、network／update＝`areka-update`・インストールの出来事、status＝起動・読み込みの節目）を要件で決め、tracing の **target 名の取り決め**として文書にする（出す側の spec はこの名前で出す）。

## Approach

`tracing_subscriber` の Layer を 1 つ足し、取り決めた target と level で種別を決めて環状の履歴へ積む。履歴は `mcp-strict-errors` が error 種別へ書き込む口も持つ（直接 `tracing` で出してもよい＝design で決める）。

## Scope

- **In**: 履歴の Layer と上限・種別の規則と target 名の取り決め・`get_log` の中身・network／update／status の出す側（`areka-update`・インストール・起動の既存の行に target を付ける）・決定論テスト。
- **Out**: script 種別を出す 1 行（kanade 側＝`mcp-kanade-tools`）・strict の記録（`mcp-strict-errors`）・ログのファイル保存・ログ窓の UI。

## Boundary Candidates

- 「何を残すか」（target と level の規則）と「どう読むか」（書式・絞り込み）の境。
- 出す側（各エンジン）と履歴の間は tracing の target 名だけで結ぶ（コードの依存を作らない）。

## Out of Boundary

- 既存の `fmt` 出力の書式を変えること。
- SSP の開発者ツールのログ窓のような表示。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`。
- **Downstream**: `mcp-strict-errors`（error 種別へ記録）。`mcp-kanade-tools` は取り決めた target で script の行を出す（並走・コードの依存なし）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `network-update`（α・`areka-update` の行を触る。α の着地後に本 spec が target を足す）。

## Constraints

- ログ檻の盲点に注意: bevy_ecs は `log::` で出すので tracing の捕捉に届かない（記憶 areka-log-cage-harness-blindspots）。
- 1 行の長さ・履歴の上限でメモリを食い潰さない（上限の数は要件で決める）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。
