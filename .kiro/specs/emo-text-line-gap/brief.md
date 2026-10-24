# Brief: emo-text-line-gap

> 2026-10-24 `vertical-choice-hover-band` の完了の棚卸（`/kiro-complete` の冒頭ステップ）で `/kiro-discovery` で起票（対話なし・測った記録からの引き渡し）。起票の条件: 直し方が 2 通りあり開発者の裁定が要る（手順 3 の 1 つ目）・直したことを確かめるのに実物の書体の読み戻しを何度も回す必要がある（手順 3 の 4 つ目）。

## Problem

行間（hhea の lineGap）を持つ書体で、emo-text が見込む行の箱の高さが、DirectWrite が実際に組む行の箱より狭い。横書きの選択肢とアンカーの反転の帯は、この見込みの高さから厚みと寄せを出すので、行間を持つ書体（游ゴシックなど）を `font.name` に書いたバルーンでは、帯が字からずれて塗られる見込みがある（まだ測っていない）。

## Current State

- `crates/areka-emo-text/src/draw_metrics.rs` の `measure_line_box_ratio` は行の箱の比を `(ascent ＋ descent) ÷ upem` だけで出し、行間を数えない。書式を作る `draw.rs` の `create_text_format` は行の間隔（`SetLineSpacing`）を設定しないので、DirectWrite は書体の既定の行の高さ（行間を含む）で組む。
- 測った数（`vertical-choice-hover-band` のタスク 5.1・縦書きの列の箱）: 游ゴシック（hhea の lineGap 1024・upem 2048）の大きさ 20 で、比から出す行の箱は 25.74、DirectWrite が組む列の箱は 32.04（28 では 36.04 と 44.86）。行間 0 の Yu Gothic UI とＭＳ ゴシックでは両者が一致する。書体の表の値: 游ゴシック Regular と Medium はどちらも lineGap 1024（upem 2048）。
- 縦書きの帯は `vertical-choice-hover-band`（タスク 5.2）で DirectWrite が測った基線から直接置く形に改めたので、この食い違いの影響を受けない。
- 横書きの帯は `choice.rs` の `line_bands` の横書きの枝で、`highlight_band_extent`（厚み）と `highlight_band_offset`（寄せ）が今も `line_box_height`＝比 × 大きさを読む。厚みの上限は行送り（`line_pitch`＝設定から出す値・比は読まない）で抑えられる。行間を持つ書体で横書きの帯がどれだけずれるかは測っていない。
- 今の手元の検体では、ぱすたの唯一のバルーン `pasta_tategaki` は縦書き（`vertical,1`）、emo2 の `emo2-kakukaku` は Yu Gothic UI（行間 0）、既定はＭＳ ゴシック（行間 0）で、横書きでこのずれに当たる検体は無い。
- 証跡: `.kiro/specs/completed/vertical-choice-hover-band/research.md` の 16.4 節（見立てと書体の表）と 17.6 節（数での裏づけ）、17.2 節の表。

## Desired Outcome

- 行間を持つ書体の横書きで、選択肢とアンカーの反転の帯が字に対して正しい所（縦書きと同じく、字に対して中央）に塗られ、当たりの範囲も同じ所になる。
- 行の箱の比を読む所（帯の厚みと寄せ・縦書きの既定の実装の逃げ道）が、DirectWrite が実際に組む行の箱と食い違わない。または、比を読まない形に改めてある。
- 行間 0 の書体（Yu Gothic UI・ＭＳ ゴシック）の帯の画素は変わらない（今あるバイト一致の檻が緑のまま）。

## Approach

まず測る。`vertical-choice-hover-band` のタスク 5.1 と同じ形（無視の印つきの記録だけの検査）で、游ゴシックの横書きの行の箱・基線とインクの位置、今の帯の位置を読み戻し、ずれの量を確かめる。そのうえで直し方を要件の段で開発者に決めてもらう:

- 案 1: 行の箱の比に行間を数える（`measure_line_box_ratio` を DirectWrite の行の高さに合わせる）。
- 案 2: 横書きの帯も、縦書きと同じく DirectWrite が測った基線から置く（`GlyphMetrics` に横書きの置き場を足し、比を帯から外す）。

推奨は案 2（縦書きと同じ単一の源・比の意味を変えないので行間 0 の書体の画素が動かない）。ただし測った数しだいで決め直す。

## Scope

- **In**: 行間を持つ書体の横書きのずれの測り・直し方の裁定・横書きの帯の厚みと寄せの直し・行の箱の比の扱い・決定論のテストと実物の書体の読み戻し・`doc/COMPAT_ARCHITECTURE.md` §8 の行（扱いを変えたとき）。
- **Out**: 縦書きの帯（`vertical-choice-hover-band` で済み）・行送りそのもの（設定の `line_pitch`）・書式に行の間隔を設定して DirectWrite の組み方を変えること（要るなら要件の段で決める）・選択肢の飾り（`choice-marker-styling`）。

## Boundary Candidates

- 書体の寸法を測る所（`draw_metrics.rs` の `DWriteMetrics` と `measure_line_box_ratio`）
- 帯の式（`choice.rs` の `line_bands` の横書きの枝・`highlight_band_extent`・`highlight_band_offset`）

## Out of Boundary

- 字の描画そのもの（DirectWrite の組み方）を変えること。

## Upstream / Downstream

- **Upstream**: 完了 `vertical-choice-hover-band`（縦書きの帯を基線から置く `GlyphMetrics::vertical_em_box_left` と、その測りの形）。
- **Downstream**: `choice-ranges-one-function`（選択肢の範囲を 1 つの関数にまとめる・`choice.rs` を分け合う）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の残り物）。
- **Adjacent**: `choice-ranges-one-function`（`choice.rs`）・`audit-emo-text-layout-draw`（`draw_metrics.rs`・`choice.rs` を見直しで触る）・`balloon-font-file`（書体の引き当て）。

## Constraints

- 実物の書体の読み戻しは GPU を使う検査で、排他テストの机が要る。游ゴシックが入った機械を前提にする（無ければ名前の固定で赤になる形は `vertical-choice-hover-band` の 5.3 の読み戻しと同じ）。
- emo-text に新しいソースファイルを足すなら、`lib.rs` の `SOURCES_OUTSIDE_THE_PURE_SCAN`（または純粋な一覧）の行を足す＝ウェーブで 1 本だけの席を使う。
- 規模の見立て: S（4〜7。測り 1・裁定・直し 1〜2・檻と読み戻し 1〜2・記録 1）。
- 要件定義のモデル: Opus（先例＝`vertical-choice-hover-band` の基線から置く形）。

- **優先度の区分**: B（バグ）。
- **版**: 版に縛らない。ぱすたの唯一のバルーン `pasta_tategaki` は縦書き（`font.name` は游ゴシックに引けるが、縦書きの帯は基線から置く形で済んでいる）で、ぱすたの横書きのバルーンは無い＝v0.1.0 のゴール「ぱすたが SSP と同じに動く」には当たらない。
