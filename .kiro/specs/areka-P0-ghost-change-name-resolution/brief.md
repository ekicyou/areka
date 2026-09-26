# Brief: areka-P0-ghost-change-name-resolution

> 2026-09-26 `/kiro-discovery` 再入（棚卸⑰）で起票（台帳 #61）。`areka-P0-ghost-shell-balloon-switch`（台帳 #13）の再測定で想定タスクが 18〜21 本に張り付いたため、**切替先の名前解決**を S の別仕様として切り出した。切る場所は「名前解決」——#13 は名指しの `\![change,ghost,名]` だけを受け、解決できない名前は「該当なし → 無視＋`warn!`」で受ける。本仕様がその腕を `random`／`sequential`／`lastinstalled` の解決に置き換え、`\+`／`\_+` を同じ経路へ繋ぐ。
> 本文の file:line は**起票時の実測値**（2026-09-26・main `13b72893`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: `.nar` を入れた直後に「入れたゴーストに交代します」と言うゴーストの利用者。里々 wiki の定石（`satori:ゴースト切り替え`「インストールしたゴーストに即チェンジまたは呼び出し（ＳＳＰ専用）」）は `OnInstallComplete` から `\![change,ghost,lastinstalled]` を出す。#13 だけでは `lastinstalled` は「該当なし」で黙って（`warn!` だけで）何も起きず、**入れて替える α の一周がゴーストの台本からは完結しない**。

同じ経路の仲間として、`\+`（ランダムに他のゴーストへ切替）・`\_+`（次のゴーストへ切替）・`\![change,ghost,random|sequential]` も今日は何も起きない。`\+`／`\_+` は字句解析で `Bare("+")`／`Bare("_+")` に切れ、`crates/areka-parsers/src/sakura/decode.rs` の `decode_bare` の既定の腕で `Raw` になり、`crates/areka-sakura/src/compile.rs` の catch-all が捨てる（#13 の棚卸⑰の再測定 項目 9）。

## Current State

- 列挙は完了 `areka-P0-baseware-root-layout` の `areka_ghost::catalog`（`BasewareRoot`・`list_ghosts`・`GhostEntry`）が持つ。`random`／`sequential` はその上の純関数 1 本で足りる。
- 「最後にインストールしたゴースト」の記録はどこにも無い（`crates/` で `lastinstalled` は 0 件の見込み・着手時に確かめる）。正典は「SSP を一度終了した場合は無効」＝**プロセスの中だけの記録**でよい（永続化しない）。書く側は #15 `areka-P0-ghost-install`。
- 切替の経路そのもの（`SwitchRequest`・`\![change,ghost]` の受け口 `emo2_boot/change_cue.rs`・kanade の切替の相）は #13 が作る。

## Desired Outcome

1. `\![change,ghost,random]` と `\+` で、**今のゴースト以外**の中から 1 体を選んで切り替わる（正典 `\+`「ランダムに他のゴーストに切り替わる。このスクリプトによる切り替えではSHIORIイベントOnGhostChangingは通知されない」）。候補が 0 体なら無視＋`warn!`。
2. `\![change,ghost,sequential]` と `\_+` で、目録の並びで「次」のゴーストへ切り替わる（末尾なら先頭へ）。`\_+` も `OnGhostChanging` を送らない。
3. `\![change,ghost,lastinstalled]` で、同じプロセスの中で最後に入れたゴーストへ切り替わる。入れていなければ無視＋`warn!`（正典「SSPを一度終了した場合は無効」）。**受け皿（記録の置き場所と書く口）を本仕様が用意し、#15 はそこへ書くだけ**。
4. 切替の経路は #13 と同じ 1 本（本仕様は名前を名指しへ解決して #13 の経路へ渡すだけ・kanade に触らない）。

## Approach

- `\+`／`\_+` は **areka-sakura の compile で汎用キャリア `\![change,ghost,random]`／`\![change,ghost,sequential]` と同じ命令へ写す**（新しい typed 命令を作らない＝`\!` は汎用キャリア 1 本の原則。消費者台帳 `consumer_ledger.rs` に行を足さずに済む）。decode で `Raw` に落ちる腕を直すか compile で拾うかは設計で決める。
- 名前解決は `areka-ghost` の新ファイル（`catalog.rs` 303 行の隣）に純関数で置く（`resolve_change_target(entries, current, name, last_installed) -> Option<GhostEntry>` の形）。乱数は注入（決定論テスト）。
- #13 の `change_cue.rs` の「解決できない名前」の腕を、この純関数の呼び出しに置き換える。

## Scope

- **In**: `\+`／`\_+` の字句・compile の写し／`random`／`sequential`／`lastinstalled` のゴースト名の解決／`lastinstalled` の受け皿（プロセス内の記録と書く口）／網羅台帳 `sakura-script.toml` の `\+`・`\_+` と `\![change,ghost…]` の備考・生成物
- **Out**: シェル・バルーンの `random`／`lastinstalled`（#50 の議題 ⑷。正典 `\![change,shell]` の `lastinstalled` は原文が「最後にインストールしたゴースト」と書く＝#50 で読み方を裁定する）／`\![call,ghost,…]`（多重ゴースト・α 後）／`lastinstalled` を書く側（#15）／`sequential` の順を「ゴーストエクスプローラの下側」に合わせること（areka に無い）

## Boundary Candidates

- 字句と compile（`areka-parsers`・`areka-sakura`）
- 名前解決の純関数（`areka-ghost`）
- 受け口の腕の置き換え（`crates/areka/src/emo2_boot/change_cue.rs`＝#13 が作る）

## Out of Boundary

- 切替の握手・降ろして起こし直す仕組み（#13）
- シェル・バルーン切替（#50）
- インストール（#15）

## Upstream / Downstream

- **Upstream**: #13 `areka-P0-ghost-shell-balloon-switch`（`SwitchRequest`・`change_cue.rs`・切替の経路）／完了 `areka-P0-baseware-root-layout`（目録）
- **Downstream**: #15 `areka-P0-ghost-install`（`lastinstalled` の受け皿へ書く）・#17 `areka-P0-alpha-release-signoff`（入れて替える一周）

## Existing Spec Touchpoints

- **Extends**: #13 の受け口（解決できない名前の腕）
- **Adjacent**: #50 `areka-P0-shell-balloon-switch`（**同じウェーブで並走する**＝下の Constraints の接触制限を守る）

## Constraints

- **#50 と並走する条件（共有ソース 0）**: 本仕様は `consumer_ledger.rs`・`boot_resolve.rs`・`boot_config.rs`・`emo2_boot/mod.rs`・`ghost_session.rs`・kanade・`menu/mod.rs` に**触らない**。触るのは `areka-parsers/src/sakura/decode.rs`（391 行）・`areka-sakura/src/compile.rs`（346 行）・`areka-ghost` の新ファイル・`emo2_boot/change_cue.rs`（#13 が作り、#50 は触らない＝#50 は別ファイル `switch_cue.rs`）とそれぞれの兄弟テストだけ。網羅台帳 `sakura-script.toml` は #50 と同じファイルの**別の行**（`\+`・`\_+` 対 `\![change,shell|balloon…]`）、生成物（`report/*.md`）は手で直さず合流後に生成器で作り直す。着手時に #13 の実物（`change_cue.rs` の名前と形）で接触ファイルを再測定し、重なりが出たら #50 の後へ直列に戻す。
- 決定論テスト網羅（乱数は注入・`list_ghosts` の並びは偽の目録で固定）。ログ無し失敗経路の禁止（候補 0・記録なしは `warn!`）。
- 規模 **S（4〜5 タスク）**。要件は Opus で足りる（正典が語を決めており、裁量は `sequential` の順と「今のゴーストを候補から外す」の 2 点）。
