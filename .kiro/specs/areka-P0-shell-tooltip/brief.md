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

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）

- **前提の変化**: `baseware-root-list` が着地した（10-16）。あちらが触ったのは置き場の並び・メニュー・`crates/areka/src/emo2_boot/` のゴーストの切り替えまわりで、キャラクター窓の当たり判定とマウスの道（`crates/areka/src/input_events/`・`emo2_boot/hit_region.rs`）には触っていない＝「窓の結線の場所が `baseware-root-list` と重ならないか」の心配は消えた。そのほかは変わらない:
  - wintf のツールチップの口（`crates/wintf/src/ecs/tooltip/mod.rs` の `register`・`supply_text`・窓に付ける `OnTooltip`）を areka で使うコードは今も 0。
  - tooltipブレスは今も記録なしに捨てられる（`crates/areka-parsers/src/shell/decode.rs` の `dispatch_block` が知らない見出しを捨てる）。箱の転記（`shell/boxes.rs`）・描かない element の転記（`shell/undrawn.rs`）と同じく 2 つ目の転記にすれば `decode.rs` に触らない。結果は `crates/areka-emo-present/src/shell_target.rs` の `parse_boxes` を呼ぶ所の隣で拾える。
  - プロパティの部分の持ち主 `currentghost-property-tree` は未着手のまま＝プロパティの SET・GET は外すしかない。
- **触るファイル**（プロパティを外した形・行数）: `crates/areka-parsers/src/shell/` の新しい転記のファイルと検査・`shell/mod.rs`（72）／`crates/areka-emo-present/src/shell_target.rs`（757）／キャラクター窓の配線＝`crates/areka/src/input_events/` か `emo2_boot/` の新しいファイルとその `mod.rs`（`emo2_boot/mod.rs` は 913 行で 1,000 行に近い・`input_events/mod.rs` は 571 行）・当たり判定名を引く `emo2_boot/hit_region.rs`（167）／SHIORI の `tooltip` を尋ねる道 `crates/areka-kanade/src/schedule/resources.rs`（359）と、メニューからの照会と同じ口 `crates/areka-kanade/src/actor_resources.rs`／台帳 `assets.toml`・`shiori.toml` の行。
- **新しいファイルと依存**: 依存の追加は無い。emo-text には触らない＝emo-text の席は使わない。
- **規模**: 8〜12 タスク（プロパティを外した形）。プロパティまで持つと 13〜16。
- **人の手・机**: **人の手の実機の確かめが要る**。上の申し送りの「主画面より上に置いた画面でツールチップの位置が合うか」は画面の並べ替えが要り、AI だけでは回せない＝席の空きに挟む候補にはならない（この項目を `balloon-link-hover` へ回しても、マウスを当たり判定に止めて出るかを見る実機の確かめは残る）。静かな机・前後の全体テストの比べは要らない。
- **先に要るもの**: 働きの上ではプロパティの部分だけ `currentghost-property-tree`。ファイルの上では `balloon-link-hover`（同じ wintf の口・`resources.rs`）と同時に走らせない。v0.0.3 の 2 本とは、窓の配線を `emo2_boot/` に置き、あちらも `emo2_boot/mod.rs` にファイルを足すときだけ重なる（`input_events/` に置けば重ならない）。`emo2_boot/ghost_switch.rs` には触らない見込み（文字の表はシェルの読み込みに乗るので、切り替えで作り直される）。
- **優先度の区分**: A のまま（開発者 10-05 の決めごと「キャラクター窓のツールチップも起票する」）。段は「その他」。
- **要件定義のモデル**: Opus のまま（wintf の口は先に作ってあり、SHIORI のリソースを尋ねる道もメニューの文言で先例がある）。
- **分割の案**: 無し（プロパティは外して `currentghost-property-tree` 側へ）。
- **見つけた穴・古くなった記述**:
  - SHIORI の `tooltip` は Reference0〜4（座標・スコープ・当たり判定の名前）を付けて尋ねるが、今の照会の組み立て（`resources.rs` の `resource_get`）は Reference を付けない形だけ＝付ける形を 1 つ足す。
  - Constraints の「シェルの element の列（`shell/{model,decode}.rs`）に掛かる見込み」は、2 つ目の転記にすれば掛からない（棚卸㉓で指摘済み・本文はまだ古いまま）。
