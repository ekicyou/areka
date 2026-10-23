# Brief: ondestroy-on-exit

> 2026-10-22 棚卸㉘（`/kiro-next`）で起票。完了 `mcp-reload` の残り物（持ち主が居なかった）：同 spec は SHIORI の読み直しのときだけ `OnDestroy`（Reference0＝`reload`）を送る形を入れたが、終了のときに送る形は誰の持ち物でもなかった。網羅台帳の行も「無い」のまま古くなっている。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

ukadoc の `OnDestroy` は「終了が指示された際に発生。[NOTIFY]」で、Reference0 は「リロード時に reload、それ以外は無し（空文字列または Reference0 自体が来ない）」（https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnDestroy ）。areka はリロードのときだけ送り、ふつうの終了（右クリックメニューの「終了」→ `OnClose` の握手 → 降ろす）と、ゴーストの切り替えで降ろすときには送らない。終了の合図で覚えたことを書き出すゴーストは、その機会を失う。

## Current State

- 送る所は 1 か所だけ: SHIORI の読み直しの手順（`crates/areka/src/emo2_boot/reload.rs` の「切れ目の印」・Reference0＝`reload`・NOTIFY）。許可の名前の表（`crates/areka-kanade/src/schedule/events.rs`）には `OnDestroy` が載った（`mcp-reload`）。
- 終了の道: kanade の進行（`crates/areka-kanade/src/schedule/close.rs` の終了トークの待ち → `schedule/mod.rs` の「降ろす相へ移る」関数 `to_unloading_quit` → SHIORI を降ろす指示）。ここで `OnDestroy` を送る段が無い。
- 切り替えの道（`schedule/change.rs`）も同じく送らない。
- 網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `OnDestroy:1` の行は `status = "absent"`・持ち主なし・「組み立てる構築関数も無い」と書いたまま＝`mcp-reload` の後は古い（リロードの場面では送っている）。

## Desired Outcome

- ふつうの終了とゴーストの切り替えで、SHIORI を降ろす前に `OnDestroy`（NOTIFY・Reference0 なし）を 1 回送る。返事の台本は再生しない（NOTIFY）。
- 送る順は SSP で測って決める（`OnClose` の後・`OnGhostChanging` の前後・unload の直前のどこか）。
- 網羅台帳の行を、リロードの場面（`mcp-reload`）とこの spec の場面を合わせた状態に書き直す。

## Approach

kanade の降ろす相へ移る所で、NOTIFY を 1 本送ってから降ろす。リロードの手順が使う NOTIFY の送り方を手本にする。切り替えの道も同じ段を通す。SHIORI がエラーを返しても終了は止めない（SHIORI のエラー応答は致命ではない、という決まり）。

## Scope

- **In**: 終了と切り替えで降ろす前の `OnDestroy`（NOTIFY）・送る順の SSP での測り・網羅台帳の行の直し・決定論テスト
- **Out**: リロードの `OnDestroy`（`mcp-reload` が済ませた）・異常終了の道（降ろすのも最善の努力だけ）・OS のシャットダウンの合図の道の作り直し

## Boundary Candidates

- kanade の降ろす相への移り（`schedule/mod.rs`・`schedule/close.rs`・`schedule/change.rs`）
- 網羅台帳の 1 行

## Out of Boundary

- `OnClose` の握手そのもの・終了トークの扱い（決まりの「終了イベントのトークは必ず終了で終わる」を変えない）

## Upstream / Downstream

- Upstream: ✅ `mcp-reload`（NOTIFY の送り方の手本）
- Downstream: なし

## Existing Spec Touchpoints

- `script-security-level`（kanade の `schedule/` の多くのファイル・`close.rs` を含む）・`mouse-click-wheel-events`（`schedule/events.rs`）と同じ場所＝kanade の列で 1 本ずつ。
- `audit-kanade`（見直し・v0.1.0 の後）が kanade を持つ。

## Constraints

- 既存の終了経路の決定論テストを 1 本も落とさない（保存義務）。
- 1 フレーム遅らせる解は取らない。
- 送るバイトが変わる＝SSP で測ってから順を決める（意味論は ukadoc・読めない点は SSP の MCP で測る）。

## 2026-10-22 棚卸㉘の見立て（main 52b014de）

- 触るファイル: `crates/areka-kanade/src/schedule/{mod,close,change}.rs`（`steady.rs` 957 行には触らない）と兄弟のテスト・`doc/ukadoc-coverage/ledger/shiori.toml` の 1 行・`doc/COMPAT_ARCHITECTURE.md` §8（追記）
- 規模: 4〜7 タスク
- 先に要るもの: なし（働き）。ファイルは kanade の列＝`script-security-level`・`mouse-click-wheel-events` と同じウェーブに置かない。
- 優先度の区分: C（ukadoc の拾い残し・完了 spec の持ち越し）
- 版: いつか（台本と SHIORI）
- 要件定義のモデル: Opus（既存の NOTIFY の送り方と降ろす相に乗るだけ）
- 議題: ⑴ 送る順（SSP で測る）⑵ 切り替えでも送るか（ukadoc は「終了が指示された際」とだけ書く＝SSP で測る）
