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


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 先に要った `wintf-tooltip`（10-08）が着地した。口は `crates/wintf/src/ecs/tooltip/` の `register`・`update`・`unregister`・`supply_text`・`dismiss` と、窓に付ける知らせ `OnTooltip`。範囲は「窓の全体」か矩形だけ。areka の側でこの口を使うコードはまだ 0（最初の使い手は本 spec か `balloon-link-hover`）。tooltipブレスは今も記録なしに吸収される（読み手の塊の振り分けが知らない見出しを捨てる）。
- 触るファイル: `crates/areka-parsers/src/shell/` の新しい転記のファイル（`boxes.rs`・`undrawn.rs` と同じ型）と `shell/mod.rs` の数行・キャラクター窓の配線（`crates/areka/src/input_events/` の隣）・SHIORI のリソースを尋ねる道 `crates/areka-kanade/src/schedule/resources.rs`・文字の表を運ぶなら `crates/areka-emo-present/src/shell_target.rs`・台帳 `assets.toml`・`shiori.toml` の行。
- 規模: 8〜12 タスク（プロパティを外した場合）。プロパティの SET・GET まで持つと 13〜16。
- 先に要るもの: 働きの上では、プロパティの部分だけ `currentghost-property-tree`（未完了）。ファイルの上では `balloon-link-hover`（同じ wintf の口・`resources.rs`）と同時に走らせない。`collisionex-regions` が先なら、形の当たり判定にも出せる。
- 優先度の区分: A（開発者 10-05 の決めごと「キャラクター窓のツールチップも起票する」）。段は開発者が「その他」と決めているので急がない。
- 要件定義のモデル: Opus。
- 分割の案: 無し。プロパティの 5 行は下のとおり持ち主を 1 つにする。
- 見つけた穴・古くなった記述:
  - 台帳 `property.toml` の `currentghost.seriko.tooltip.*` の 5 行は、持ち主が `currentghost-property-tree` になっている（状態は「語の表だけ」）＝本 spec の In と二重。推しは「本 spec が文字の表（スコープ × 当たり判定名）と差し替えの口を作り、プロパティとして見せるのは `currentghost-property-tree`」。
  - Constraints の「シェルの element の列に掛かる見込み」は、2 つ目の転記にすれば掛からない（`shell/{model,decode}.rs` に触らない）。
  - 優先の順は正典が「シェル側設定がある場合はそちらが優先される」と書く＝tooltipブレスが SHIORI の `tooltip` より先。プロパティで差し替えた文字との順だけが議題に残る。

## 2026-10-10 棚卸㉓の申し送り

- **実機の確かめに 1 項目足す: 主画面より上に置いた画面でツールチップを出す**。完了 `wintf-tooltip` は、ツールチップの位置を OS へ渡すときに 2 つの座標を 16 ビットずつに詰める（`crates/wintf/src/ecs/tooltip/os.rs` の `make_lparam`・負の座標は 2 の補数のまま）。OS が符号付きで読むかは実機でしか決まらない。同 spec の `tasks.md` の Implementation Notes は、3.1 で「負の座標の画面で出す確かめを入れる」と書き、6.1 で「y が負の画面が無いので負の y の詰め方は未確認」と残した。
- areka からこの口を最初に使う spec（本 spec か `balloon-link-hover`）の実機の確かめで、主画面の上に画面を置き、そこでツールチップの位置が合うかを見る。ずれたら wintf の側の不具合として起票する（本 spec の中では直さない）。
