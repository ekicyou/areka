# Brief: areka-P0-mcp-reload

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントは辞書やシェルを書き換えたら、ゴーストを再読み込みして結果を見る。SSP の `reload` は ghost／shiori／shell／balloon／makoto／descript の 6 対象で、同じことは台本の `\![reload,…]` でもできる（ukadoc・MCP の `sakurascript` からも使える）。areka には再読み込みという操作そのものが無い。

## Current State

- 再読み込みは 1 つも無い（`makoto-dll-host` の brief も「reload という概念自体が未実装」と書く）。
- いちばん近いのはゴーストの再起動＝切替の経路（`crates/areka/src/emo2_boot/ghost_switch.rs` の `switch_to`・`boot_into`、完了 `ghost-restart-unit`）。正典の `\![reload,ghost]` は「自分自身へのゴースト切り替えと、SHIORI イベントが発生しないことを除けばほぼ同じ」。
- シェル・バルーンの載せ替えは α の `shell-balloon-switch`（`\![reload,shell|balloon]` を範囲外にしている）が作る。

## Desired Outcome

- 6 対象を 1 つのエンジンの操作として作り、**MCP の `reload` と台本の `\![reload,ghost／shiori／shell／balloon／descript]`（`\![reload,descript,パラメータ]`・旧 `\![reloadsurface]` を含む）の両方**から呼べる。
- 読み込みの失敗はエラーログ（`mcp-log-history` の error 種別）へ出る（SSP: 「Errors while loading are recorded to the error log」）。
- **makoto**: `makoto-dll-host`（α 後）が着地するまでは `NG:` で返してログを出す縮退の口を置き、同 spec の brief へ「MCP の reload makoto はこの口を埋める」と申し送る（記憶 defer-canon-with-full-vocabulary-and-tracking-spec）。

## Approach

`shell-balloon-switch` と切替の経路が作った載せ替えの部品を、「同じものへ載せ替え・SHIORI イベントを出さない」形で呼ぶ。shiori だけの載せ替え（シェルは残す）と descript の読み直しは要件で ukadoc を引いて意味を確定する。

## Scope

- **In**: 5 対象（ghost／shiori／shell／balloon／descript）の再読み込み・`\![reload,…]` の消費者（`\!` の汎用キャリアの台帳へ登記）・MCP `reload` の中身・makoto の縮退の口・ukadoc 網羅台帳の `owner`・決定論テスト。
- **Out**: makoto の本体（`makoto-dll-host`）・`\![reload,aigraph]`（AI グラフは無い）。

## Boundary Candidates

- 載せ替えの部品（既存・`shell-balloon-switch`）と、再読み込みの操作（新）と、2 つの入口（台本・MCP）の境。

## Out of Boundary

- 別のゴースト・シェル・バルーンへの切替（既存）。
- ファイルの変更を見張って自動で読み直すこと。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`・`shell-balloon-switch`（α・載せ替えの部品）。
- **Downstream**: `makoto-dll-host`（makoto の口を埋める）。

## Existing Spec Touchpoints

- **Extends**: `makoto-dll-host`（brief へ申し送りを追記済み・2026-09-29）。
- **Adjacent**: `shell-balloon-switch`・完了 `ghost-restart-unit`・`ghost_session.rs`。

## Constraints

- 既存の終了経路と切替の決定論テストを 1 本も落とさない（roadmap の保存義務）。
- 規模 M〜L。kanade を触る必要が出たら同じウェーブの `mcp-kanade-tools` と重なる＝design で触らない形を先に探し、触るなら `mcp-kanade-tools` の後へ回す（干渉台帳）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。

## 2026-10-03 C4 の候補（10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。C3 に入れなかった理由: `mcp-kanade-tools` と kanade を分け合いうる。
