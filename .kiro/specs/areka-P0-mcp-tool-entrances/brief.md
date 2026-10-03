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
- サーバは rmcp（`mcp-server-core`・2026-09-29 開発者判断）。**ツールの定義は rmcp の `#[tool]` マクロ（schemars の自動生成）でなく、保存した JSON の逐語を `Tool` に詰めて手で返す**——自動生成の inputSchema は SSP の逐語（`"type": "integer"` の欄・description の英文・`required` の並び）と一致しないため。手で詰めるか、マクロで一致させられるかは design で確かめる。
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


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **C4 の候補**（10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」）で段は「その他」・`mcp-server-core`＝C3 の後）**・Fable 推奨**。規模 M（13〜17 タスク）。
- brief の記述は実物と一致（`ChangeRx`／`drain_change_requests`＝`emo2_boot/ghost_switch.rs`、`InstallDesk::drain`＝`install/desk.rs`、`GhostSlot`＝`ghost_session.rs`、汲む系の登録は `ghost_session.rs::register_systems`）。
- **足りなかった事実**: 3 つ目の定石がある＝`areka_actor::spawn_ui`／`UiSender`（async-channel で UI スレッドへ即時に届ける。emo-text と placement の follow が使っている）。毎フレームの `try_iter` より返事が速い。⚠ `ReplyReceiver` は std の mpsc で、待つとスレッドを塞ぐ＝tokio の current_thread の中で `recv_timeout` を呼ぶとほかの要求まで止まる。`spawn_blocking` で包むか async の返事にする。
- **触るファイル**: `crates/areka-mcp/src/tools/*.rs`（10 本）と定義の一致テスト・新規 `crates/areka/src/mcp/**`・`crates/areka/src/ghost_session.rs`（866 行・`register_systems` に 1 行）・`crates/areka/src/main.rs`（送り口を渡す）。
- **議題**: 返事を待つ上限／ダミーの文言／定義を JSON の逐語から手で詰めるかマクロで一致させられるか／汲み方（毎フレームか `spawn_ui` か）。
