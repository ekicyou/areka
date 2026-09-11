# Brief: areka-P0-balloon-font-descript-keys

> 起票: 2026-09-11（棚卸⑬・`areka-P0-text-decoration-canon` の分割 ⑶＝descript `font.*` 基底 13 キーの解析拡張を独立 spec に切り出し）。親 brief 棚卸⑫追記が「完全独立スライス（`areka-parsers/balloon` に閉じ・共有ファイル 0）」と認定した片。

## Problem

バルーン descript の `font.*` 基底 13 キーのうち、`crates/areka-parsers/src/balloon/parse.rs` が引くのは `font.color.r/g/b`・`font.name`・`font.height` の 5 本だけ。`font.underline`／`font.bold`／`font.italic`／`font.strike`／`font.shadowcolor.*`／`font.shadowstyle` 等は解析されず無言で捨てられる（未知キーは `kv/parse.rs` が保持するが `BalloonModel` に写像が無い）。`\f[...]` の既定層（親 spec）がこれらを読むには、まず転記層に写像が要る。

## Current State

2026-09-11 実測: `parse.rs` の `font.*` 引きは 5 本（親 brief 再測定・OK）。`BalloonModel` の `impl Font` に `name`／`height`／`color` の取得口。`disable.font.*` は draw 側の予約シーム（`FontDisableSeam`）のみ。ukadoc の descript_balloon ページの `font.*` 掃引は `doc/ukadoc-coverage/ledger/assets.toml`（完了 spec `ukadoc-survey-assets`）に 1 項目 1 行で載っている＝キー集合の照合元。

## Desired Outcome

基底 13 キーが `BalloonModel` に写像され（値形式・既定値・未指定の表現が決定論テストで固定）、各定義行に `/// ukadoc: <url>` が 1 行ある。読み手（親 spec の既定層）が着地した時点でそのまま消費できる。

## Approach

転記のみ（`areka-parser-transcribes-tree-downstream`）。`map_merged` にキー 8 本を足し、`Font` に取得口を足す。消費側は親 spec＝**後着の方が配線する**（本 spec が先なら親 spec が読む・親 spec が先なら本 spec が最後のタスクで既定層へ繋ぐ）。相互登記済み。

## Scope

- **In**: 基底 13 キーの解析＋モデル＋テスト＋URL コメント／`ledger/assets.toml` の `owner` 記入。
- **Out**: 接頭辞付き font 族 9 系統（`sstpmessage.font.*`／`number.font.*`／`communicatebox.font.*` 等＝各機能 spec の解禁時）／`\f` 側の意味論（親 spec）／`disable.font.*` の実体化（親 spec・draw 側）。

## Boundary Candidates

- 単一スライス（S）。

## Out of Boundary

- バルーン名 `name,` の写像（`emo-text-canon-residue` 項目 14・同じ `map_merged` を触るため**同居不可**・本 spec を先に着地させる）。

## Upstream / Downstream

- **Upstream**: なし（即着手可）。
- **Downstream**: `text-decoration-canon`（既定層で読む）・`text-align-shadow-canon`（影の既定値）・`emo-text-canon-residue`（`parse.rs` 後着）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-text-decoration-canon`（分割元・⑶ を引き継ぐ）。
- **Adjacent**: `anchor-tag-canon`（`anchor.font.*` を同じ `parse.rs` で引く＝後着 rebase）。

## Constraints

- 編集集合: `crates/areka-parsers/src/balloon/{parse,model}.rs`＋兄弟テスト（`parse_tests.rs` の distractor 行を壊さない）・`doc/ukadoc-coverage/ledger/assets.toml`。W13 の他 spec と共有ファイル 0（実測）。
- 規模 S・要件定義は Opus で足りる。
