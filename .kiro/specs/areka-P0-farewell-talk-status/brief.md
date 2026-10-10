# Brief: areka-P0-farewell-talk-status

> 2026-10-05 `mcp-get-status` の要件ディスカッション（議題 1）で見つかり、開発者の裁定「1 でよい・必要な起票をせよ」で起票した。`mcp-get-status` は要件 2.1 を「普段の会話と起動の挨拶の再生中」に絞り、お別れの台詞の間に `talking` が出ないことを SSP との差の一覧に書いて先へ進む。直すのは本 spec。

## Problem

終了の挨拶と、ゴーストの切り替えのお別れの台詞を再生している間、kanade は「台本を再生中」と見なさない。そのため実行の状態（ukadoc `Status [SSP拡張]`）に `talking` が載らない。

- MCP の `get_status` は、ゴーストが話しているのに空か `balloon(…)` だけを返す（`mcp-get-status` の着地の後）。
- 切り替えのお別れの台詞の中から `\![raise,…]` で SHIORI へイベントを送ると、その要求の `Status` にも `talking` が載らない（`schedule/change.rs` の raise の組み立てが `State::snapshot()` を使い、その中の判定が偽になる）。

## Current State

- 「再生中か」の判定は `crates/areka-kanade/src/schedule/mod.rs` の `talk_active_of`。真を返すのは `Phase::Steady { talk: Some(_) }` と `Phase::BootVersion { talk: Some(_) }` の 2 つだけ。
- `Phase::CloseTalkWait`・`Phase::ChangeTalkWait`・`Phase::ChangeCloseTalkWait` は再生中のトークの番号を持つ（同じファイルの、相から再生中のトークの番号を引く関数がこの 3 相で `Some` を返す）。それでも `talk_active_of` は偽になる。
- 起動・切替・終了の握手の要求（`OnInitialize`・`OnBoot`・`OnGhostChanging`・`OnClose` など）は `State::snapshot_without_talk()` で組み立てられる。この握手の要求には `talking` を載せない作りであり、本 spec の対象ではない。

## Desired Outcome

ゴーストが終了の挨拶・切り替えのお別れの台詞を再生している間は、実行の状態に `talking` が載る。`get_status` の答えにも、その間に SHIORI へ送る要求の `Status` にも載る。

## Approach

`talk_active_of` に 3 つの相（`CloseTalkWait`・`ChangeTalkWait`・`ChangeCloseTalkWait`）を足す。この判定を使うすべての所への波及（`State::snapshot` の利用者・選択待ちや `nouserbreak` の導出）を要件の段で洗い出す。そのうえで、握手の要求が `snapshot_without_talk()` のまま変わらないことをテストで固定する。

## Scope

- **In**: お別れの台詞の再生中の `talking`（`get_status` と SHIORI への要求の `Status` の両方）。決定論テスト。`mcp-get-status` が書いた SSP との差の一覧（`doc/ssp-mcp/get-status-diff-areka.md`）の該当行を消す。
- **Out**: SSP の旗 `changing`（足さない＝`mcp-get-status` の裁定）。出どころの無い 5 語（`minimizing`・`induction`・`passive`・`timecritical`・`opening(…)`）。握手の要求の `Status` の作り方。

## Boundary Candidates

- kanade の運行の判定（`schedule/mod.rs` の `talk_active_of` とその兄弟テスト）

## Out of Boundary

- MCP のツールの側（`crates/areka/src/mcp/get_status.rs`）。kanade が正しい値を返せば手を入れずに直る。

## Upstream / Downstream

- **Upstream**: `mcp-get-status`（差の一覧の行を消すため・着地を待つ）。`balloon-lifecycle-events`（C4-⑩・`schedule/` の持ち主＝同じウェーブに置かない）。
- **Downstream**: `currentghost-property-others` の `currentghost.status`（同じ値を読む）。

## Existing Spec Touchpoints

- **Extends**: 完了 `status-execution-states` の導出表（再生中の定義）。
- **Adjacent**: kanade の進行の列（`schedule/`）。

## Constraints

- `schedule/mod.rs` は 938 行（上限 1,000）。足すのは 3 行程度だが、テストは兄弟のテストファイルへ置く。
- 規模 S（2〜4 タスク）。Opus で足りる（−）。段＝バグ。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - 上流の 2 本（`mcp-get-status`・`balloon-lifecycle-events`）は着地した＝待つものは無い。
  - 「再生中か」の判定（`crates/areka-kanade/src/schedule/mod.rs` の `talk_active_of`）は今も普段の会話と起動の挨拶の 2 つの相だけを真にする。SSP との差の一覧 `doc/ssp-mcp/get-status-diff-areka.md` に該当の行（終了の挨拶・切り替えのお別れの台詞の再生中）がある。
  - `schedule/mod.rs` は 938 行から 955 行になった（`balloon-lifecycle-events`・`choice-script-prefix`）。足すのは判定の 3 行と、新しいテストファイルをつなぐ宣言の 3 行ほど。
  - `balloon-lifecycle-events` が、実行の状態を写して SHIORI へ送る所を 1 つ増やした（`schedule/balloon_events.rs` の、バルーンの 3 つのイベントの要求を組む所）。お別れの台詞の間にバルーンのイベントが送られるなら、その要求の `Status` にも `talking` が載るようになる＝波及の洗い出しの対象に足す。
  - `get_status` に答える所（kanade の殻 `actor.rs` の、状態の問い合わせに答える関数）も同じ写しを使う＝判定を直せば、MCP の側は手を入れずに答えが変わる。
- 触るファイル: `crates/areka-kanade/src/schedule/mod.rs`（判定と宣言）・新しい兄弟のテストファイル（既存の `schedule_tests.rs` は 903 行なので足さない）・`doc/ssp-mcp/get-status-diff-areka.md`（1 行を消す）。問い合わせの通しのテストを足すなら `crates/areka-kanade/tests/kanade/status_query_test.rs`。
- 規模: S（2〜4 タスク）のまま。
- 先に要るもの: 働きの前提なし＝今すぐ始められる。
  - 同じウェーブに置けない相手は `schedule/mod.rs` を触る spec: `mcp-kanade-tools`・`script-security-level`・`anchor-tag-canon`・`sakura-time-critical`・`property-query-channels`。小さい直しなので、同じ時期に進めたいなら本 spec を先に着地させ、相手が取り込む。
  - 重なり 0: `mcp-stdio-bridge`・`mcp-shiori-query`・`mcp-reload`・`dump-balloon-debug-timeout`。
- 優先度の区分: B（バグ・話しているのに `talking` が出ない）。
- 要件定義のモデル: Opus（判定 1 か所・波及の洗い出しは読むだけで済む）。
- 分割の案: なし。
- 見つけた穴・古くなった記述: Constraints の「`schedule/mod.rs` は 938 行」は古い（955 行）。
