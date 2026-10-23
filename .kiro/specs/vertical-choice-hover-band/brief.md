# Brief: vertical-choice-hover-band

> 2026-10-23 `areka-P0-pasta-in-windows-verification` で `/kiro-discovery` により起票（本 spec からの申し送り（2026-10-23））。崩れた所は開発者の目視の判定（同日のチャット）「選択肢をマウスホバーした時の背景の塗りが崩れている。横書きの時と縦書きの時で、枠の補正方法がことなるのでは？」。起票の決めは開発者の言葉「0.1.0までに解決しない場合、独立して起票せよ」「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」と、この起票の提案への「承認」（同日）。版は **v0.1.0**（ゴールの 2 つ目「ぱすたが SSP と同じに動く」）。

## Problem

- **ゴーストの利用者とバルーンの作者（ぱすた）**: 縦書きのバルーン（ぱすたの `pasta_tategaki`・`vertical,1`）で選択肢の列にマウスを乗せると、反転の帯が列の左へずれ、列の幅のおよそ半分しか塗らない。横書きのバルーンでは帯は行に合っている。

## Current State

- **証拠**: `verification.md` の崩れた所の表の **23**（開発者の目視の判定・2026-10-23）。撮った物 `target\pasta-verify-alt\shots\alt2\2b-choice-hover.png`（「雨だよ」の列の反転）。回し直しの撮った物は `target\pasta-verify-rerun\shots\rerun1\` の下（選択肢の段は `2-choice-zorder.png`・`2b-after-choice-1.png`・`2b-after-choice-2.png`）。
- **原因の見込み**（file:line は要件の段で確かめる）: `crates/areka-emo-text/src/choice.rs`（748 行）の帯の作りが、横書きの行の箱の寸法（ascent＋descent・ブロックの近い辺からのずれ）で帯の厚みとずれを決める（`highlight_band_extent` 138 行目・`highlight_band_offset` 176 行目・216 行目の分岐）。縦書き（`VerticalRl`／`VerticalLr`）でも同じ厚みとずれを x の軸にそのまま当てている（330〜335 行目の `LineRect { left: line.rect.left + band_offset, … right: line.rect.left + band_offset + band_extent }`）。縦書きの列のブロックの軸の厚みは字の幅で決まるので、横書きの補正では帯がずれて細くなる。当たりの行（クリックの判定）も同じ値を使う（394・434・454 行目）＝塗りと当たりが一緒にずれている見込み。
- 今あるテスト `crates/areka-emo-text/src/viewbox_draw_choice_hover_tests.rs:397` の `vertical_band_offset_shifts_paint_on_block_axis` は `VerticalLr` だけを見ている。ぱすたは `VerticalRl`。
- **SSP の基準**: ぱすたの測り（`ssp-measurement.md` 2 節の段 2）では選択肢 4 つの並びと選んだ後を撮ったが、縦書きの選択肢に乗せたときの反転は撮っていない＝要件の段で SSP の MCP で測る（帯の幅・ずれ・列との合い方）。正典（ukadoc）は `cursor.style`・`cursor.blendmethod`・`cursor.brush.color.*`・`cursor.pen.color.*`（ぱすたの縦書きのバルーンが書いている鍵）で、縦書きの帯の寸法そのものは書いていない。

## Desired Outcome

- 縦書き（`VerticalRl`・`VerticalLr`）の選択肢の反転の帯が、列の幅いっぱいに列と合って塗られる。当たりの範囲も同じ所になる。
- 横書きの帯は今のまま（変えない）。
- ぱすたの縦書きのバルーンで、SSP と同じ形の反転になる。

## Approach

- 縦書きでは帯の厚みとずれを列の寸法（字の幅で決まるブロックの軸の厚み）から取る。横書きの補正の関数はそのまま横書きに使う。

## Scope

- **In**: 縦書きの帯の厚みとずれの求め方（塗りと当たり）・`VerticalRl` と `VerticalLr` の決定論のテスト・SSP の MCP での測り・ぱすたの実機の確かめ（`ghost-inner-balloon` の着地の後）。
- **Out**: 選択肢の前の `\![*]` の印（`script-small-tags`・崩れた所の表の 3）・横書きの帯・`cursor.*` の鍵の色や描き方の範囲を広げること・選択肢のマーカーの見た目（`choice-marker-styling`）。

## Boundary Candidates

- emo-text の選択肢の帯の寸法（`choice.rs`）

## Out of Boundary

- 縦書きの列の並べ方そのもの（完了 `balloon-vertical-canon`）

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。実機でぱすたの縦書きのバルーンを見るには `ghost-inner-balloon`（v0.1.0）の着地が要る（働きの前提ではない）。
- **Downstream**: `pasta-in-windows-verification` の回し直し。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec は直さない）。
- **Adjacent**: `choice-ranges-one-function`（選択肢の範囲の関数をまとめる・同じ `choice.rs`＝同じウェーブに置かない）・`range-choice-tag`・`balloon-link-hover`（同じ反転の帯を使う）・`balloon-scroll-arrows`（emo-text）。

## Constraints

- 1 ファイル 1,000 行（`choice.rs` 748 行）。
- 決定論のテストは必達。横書きの帯の既存のテストを 1 本も落とさない。

## 起票時の見立て（2026-10-23・ブランチ `claude/areka-p0-pasta-windows-d4ed7a`）

- 触るファイル: `crates/areka-emo-text/src/choice.rs` と兄弟のテスト（`viewbox_draw_choice_hover_tests.rs` ほか）。
- 規模: S（3〜5）。
- 優先度の区分: A（開発者の指示＝v0.1.0 のゴール・ぱすた）。段: 優先（バルーンの見た目＝段の決まりの「バルーン関係」）。
- 要件定義のモデル: Opus。
- 議題: SSP の縦書きの反転の帯の形（測る）。
