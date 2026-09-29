# Brief: areka-P0-mcp-tool-entrances

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **2 段目（空のダミー関数を置いて入り口だけ全部整備）**。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)、ツール定義の逐語は [doc/ssp-mcp/tools-list-ssp-2.9.05.json](../../../doc/ssp-mcp/tools-list-ssp-2.9.05.json)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

3 段目のツール 7 spec を並走させたい。並走できるかは「各 spec が自分のファイルだけを触る」形が先にできているかで決まる。ツール表・引数検査・`ghost_name` の解決・アプリ本体への橋を各ツールの spec がそれぞれ足すと、同じファイル（振り分けの `match`・inbox の enum）を全員が触って毎回 rebase になる。

## Current State

- `mcp-server-core` が着地した時点で、`tools/list` は空・`tools/call` の登録口だけがある。
- アプリ本体へ問い合わせて返事を待つ定石は 2 つ（`areka-actor` の慣行）: ⑴ UI スレッドの World へ届ける＝mpsc の受け口を NonSend 資源に置き、登録したシステムが毎フレーム `try_iter` で汲む（`crates/areka/src/emo2_boot/ghost_switch.rs` の `ChangeRx`＋`drain_change_requests`・`install/desk.rs` の `InstallDesk::drain`）、⑵ 返事は `areka_actor::ReplySender<T>`（落とすと要求側が `Err(Dropped)`）。ゴーストの切替で kanade の送り口が差し替わるので、**送り口を直接握らず World 経由で引く**のが切替に強い。
- ゴーストは 1 体だけ（`crates/areka/src/ghost_session.rs` の `GhostSlot(Option<GhostSession>)`）。名前は `GhostSession::names()`（descript の `name` など）。

## Desired Outcome

- `tools/list` が SSP 2.9.05 と**同じ 10 本**（名前・title・description・inputSchema が逐語で一致。一致は保存した JSON との比較テストで判定する）を返す。
- `tools/call` で、未知のツール名・必須引数の欠落・型違いは `-32602`、`ghost_name` の解決に失敗すれば `NG:Cannot find active ghost from specified name`（`isError: true`）、名前の要るツールで省略すれば `NG:Specified ghost is not active`（survey §3）。
- 解決できたら各ツールの実装関数（ダミー）へ届き、ダミーは `NG:not implemented yet`（文言は要件で決める）を `isError: true` で返す。
- **`get_active_ghost_list` だけは本物**（名前の解決に同じ一覧を使うため・survey §3 の形）。
- 3 段目の各 spec は「自分のツールのファイル」と「自分の触るエンジン」だけを触ればよい形になっている（ファイル配置を design で固定し、roadmap の干渉台帳へ書く）。

## Approach

- `areka-mcp` にツールごとの 1 ファイル（定義＋引数の型＋実装関数）を置き、表は全 10 本をこの段で埋める。
- アプリ本体側（`crates/areka/src/mcp/` 仮）に要求の enum（全 10 本の変種をこの段で揃える）と、UI スレッドで汲むシステム、ツールごとの処理ファイル（ダミー）を置く。サーバのスレッドは要求を送って `ReplyReceiver` で待つ（待ちの上限は要件で決める）。
- 結果の組み立て（`OK:`／`NG:`／素の値・画像の content）を共通の小関数にする。

## Scope

- **In**: 10 本の定義の逐語・引数の検査・`ghost_name`（名前またはルートフォルダのフルパス）の解決・アプリ本体への橋（要求の enum・汲むシステム・返事の待ち）・結果の共通形・`get_active_ghost_list` の実装・残り 9 本のダミー・保存した JSON との一致テスト。
- **Out**: 残り 9 本の中身（3 段目）。

## Boundary Candidates

- `areka-mcp`（プロトコル側のツール定義）と `crates/areka`（World へ届ける橋）の境。
- 橋の enum と汲むシステムはこの段で完成させ、3 段目は変種の処理関数の中身だけを書く。

## Out of Boundary

- kanade・sylphya・emo への新しい問い合わせ口（3 段目の各 spec）。
- 複数ゴーストの同時起動（予約「多重ゴースト」）。一覧は 1 体。

## Upstream / Downstream

- **Upstream**: `mcp-server-core`。
- **Downstream**: `mcp-get-property`・`mcp-kanade-tools`・`mcp-expression-table`・`mcp-log-history`・`mcp-reload`・`mcp-dump-images`（並走）→ `mcp-strict-errors`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `ghost_switch.rs`・`ghost_session.rs`（切替の経路を触らずに読むだけ。α の `shell-balloon-switch` が同じファイルを触るので、着手時に引き直す）。

## Constraints

- 10 本の定義は SSP の逐語を正とする（description の英文も変えない）。変えるなら要件で理由を書く。
- 待ちの上限を超えたら `NG:` で返し、ログを 1 行出す（サーバのスレッドが止まらない）。
