# Brief: areka-P0-collisionex-regions

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-mouse-drag-events` の完了時の棚卸（`.kiro/specs/completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes・`verification/real-machine.md` 4 章 ⑵）。開発者の方針（2026-10-05）「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」。

## Problem

- **利用者・ゴーストの作者**: 当たり判定を `collisionex`（矩形・円・楕円・多角形）で書いたシェルでは、areka のマウスのイベントの Reference4（当たり判定の名前）がいつも空になる。クローディアの当たり判定は全部 `collisionex` なので、撫で（`OnMouseMove`）・ダブルクリック・ドラッグのどれでも「どこを触ったか」がゴーストに届かない。
- `mouse-drag-events` の実機では、R1・R2 の 4 回のドラッグで Reference4 が全部空だった（クローディアのドラッグの台詞は当たり判定を見ないので確認には響かなかった）。

## Current State

- 読み手 `crates/areka-parsers/src/shell/decode.rs` の `decode_collisions` は、番号が数字だけの `collisionN`（矩形）だけを値にし、`collisionex` の行は記録なしに読み飛ばす。
- 当たり判定の解決は `crates/areka-emo-compose/src/hit.rs`（矩形と名前）。areka の `input_events/mod.rs` の doc にも「`collisionex` は実装しない（7.4）」とある。
- 台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `collisionex` の行は `absent`・担当なし（優先度 A12・価値「触れ合い」）。

## Desired Outcome

- `collisionex*,名前,rect|ellipse|circle|polygon,…` を読み、当たり判定の解決が形に応じて内外を判定する。`collision` と `collisionex` の重なりは今の画家の順（後に書いたものが手前）に揃える。
- 読めない行（形の名前が未知・座標の数が足りない）は記録を 1 件残す。
- 決定論のテストで 4 つの形の内外と重なりを固定し、実機でクローディアの撫でとドラッグの Reference4 に名前（`Head`・`Bust` など）が載ることを確かめる。

## Approach

ukadoc の `collisionex` の書式（`descript_shell_surfaces`）を引き直し、読み手に形の種類を足して、`hit.rs` の判定を形ごとに広げる。入れ子のサーフェス（`surface-element-nesting` で持ち込む子の当たり判定）にも同じ型で乗せる。

## Scope

- **In**: `collisionex` の読み・形ごとの内外判定・重なりの順・記録・台帳の行・`doc/COMPAT_ARCHITECTURE.md`・決定論のテスト・実機の確認。
- **Out**: `animation*.collision*` の当たり判定（アニメーション中の当たり判定）・カーソルの形（`The Hand`）。

## Boundary Candidates

- 読み手（`shell::{model,decode}`）
- 当たり判定の解決（`areka-emo-compose` の `hit.rs`）

## Out of Boundary

- マウスのイベントの送り方そのもの（kanade・`input_events`）は今のまま。名前が載るだけ。

## Upstream / Downstream

- **Upstream**: 完了 `shell-parse`・`emo-compose`・`surface-element-nesting`（子の当たり判定の持ち込み）。
- **Downstream**: 撫で・クリック系のイベント全般（Reference4 を使うゴースト）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `element-base-method`・`element-clipping-option`（同じ `decode.rs`）——同時に走らせない。

## Constraints

- 段は**優先**（シェルの element の列・既存のゴーストの当たり判定がまるごと効かない）。
