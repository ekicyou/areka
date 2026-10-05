# Brief: areka-P0-shell-tooltip

> 2026-10-05 `/kiro-discovery` で起票（「バルーンのリンクと OS の連携」の議題 6・開発者裁定＝案 A＝起票して `wintf-tooltip` の後に作る）。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。

## Problem

- **利用者**: キャラクターの当たり判定に説明を添えたいシェル作者・ゴースト作者と、触れる場所を知りたい利用者。
- 正典（ukadoc）には 3 つの道がある。
  - surfaces.txt の **tooltipブレス**（`sakura.tooltips`・`kero.tooltips`・`char*.tooltips`）。中の行は「当たり判定名,表示内容」＝当たり判定の上に来たときに出す文字。例: `Head,頭をなでる`。
  - SHIORI リソース `tooltip`＝シェルにおけるツールチップ内容の取得（Reference0・1＝マウスの座標・Reference3＝本体 0／相方 1／それ以降・Reference4＝当たり判定の識別子）。「シェル側設定がある場合はそちらが優先される」。
  - プロパティ `currentghost.seriko.tooltip.scope(ID).textlist(当たり判定名).text` ほか（[SET有効]・空文字の SET で定義を消す）。
- areka ではどれも未実装で、tooltipブレスは**黙って吸収**される（記録なし）。

## Current State

- tooltipブレス: `crates/areka-parsers/src/shell/` の `dispatch_block` が名前で振り分ける塊に入っておらず、塊ごと吸収される。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の「当たり判定名,表示内容」の行は「無い」・引受先なし。
- SHIORI の `tooltip`: `crates/areka-sylphya/src/vocab/shiori_resource.rs` の語の表だけ。台帳 `shiori.toml` で「語の表だけ」・引受先なし。
- プロパティ: `crates/areka-sylphya/src/vocab/dotted.rs` に `seriko.tooltip.text`・`seriko.tooltip.name`（`RuntimeCommand`）の語がある。台帳 `property.toml` の `currentghost.seriko.tooltip.*` の 5 行の状態は要件の段で確かめる。
- 当たり判定はキャラクター窓の当たりの仕組み（`collision`・`surface-element-nesting` の `HitRegions`）。`collisionex` の形は `collisionex-regions` が作る。

## Desired Outcome

- キャラクターの当たり判定にマウスが止まったら、次の順で決めた文字をツールチップに出す。
  1. プロパティで差し替えた文字。
  2. tooltipブレスの文字。
  3. ゴーストへ `tooltip` を尋ねて返った文字。
  4. どれも無ければ出さない。
- 1 と 2 の優先の順は要件の段で ukadoc を引き直して確定する。
- tooltipブレスを読めない行は記録を出す（黙って吸収しない）。

## Approach

- `wintf-tooltip` の口を使う。静的な登録（tooltipブレス・プロパティ）と動的な表示（SHIORI の `tooltip` の答え待ち）の両方。
- tooltipブレスの読み込みは `areka-parsers` の shell の塊の振り分けに 1 つ足す。

## Scope

- **In**: tooltipブレスの読み込み・SHIORI の `tooltip`・プロパティの SET と GET・キャラクター窓の配線・決定論テスト・台帳の行。
- **Out**:
  - バルーンのツールチップ（`balloon-link-hover`）。
  - マウスカーソルの差し替え（`mousehover*` などのカーソルの項目）。

## Boundary Candidates

- tooltipブレスの読み込み（parsers・純粋）。
- 文字の決め方と配線（キャラクター窓・SHIORI・プロパティ）。

## Out of Boundary

- 当たり判定の形（`collisionex-regions`）。

## Upstream / Downstream

- **Upstream**: `wintf-tooltip`。`collisionex-regions` が先に入れば `collisionex` の形にも出せる。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `collisionex-regions`（`shell/decode.rs` を触る）・`currentghost-property-tree`（プロパティの動く値の口）。

## Constraints

- シェルの element の列（`shell/{model,decode}.rs`）とプロパティの動く値の列に掛かる見込み。着手の前に照合する。
- 段: その他（バルーンのリンクとは別の窓・急ぐ理由が無い）。規模の見込み M（8〜12）。
