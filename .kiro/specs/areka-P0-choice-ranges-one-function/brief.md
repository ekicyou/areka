# Brief: areka-P0-choice-ranges-one-function

> 2026-10-06 起票（`/kiro-discovery`）。出どころは spec `areka-P0-budoux-reveal-reflow` の完了時の棚卸（`completed/areka-P0-budoux-reveal-reflow/tasks.md` の Implementation Notes「5.3」と最終確認の点検）。調停役 areka棚卸 の指示で起票。バグではなく、検査が本番の並びを見張れない穴。

## Problem

- **開発者**: 選択肢の強調とクリックを受ける範囲は「表示されている字と同じ行の割り当てから導く」（`budoux-reveal-reflow` 要件 4.4）。ところが、それを導く本番の手順が関数 1 つにまとまっていないので、検査は同じ手順を検査の側で並べ直して呼んでいる。本番の並びや渡す行の列が変わっても、検査は気付けない。

## Current State

- `crates/areka-emo-text/src/actor_present.rs` の `present_actor` が、`arrange_lines` で得た行の列 `lines` を次の順に渡している:
  - `annotate_lines(&lines, spans)`（選択肢の範囲を行ごとに割り当てる）
  - `line_bands(&lines, mode, &metrics)`（行の帯）
  - 間に `decorate_canvas`（強調の描画）と `render.executor.render_styled` が挟まる
  - `changed` のときだけ `derive_hit_rows(&lines, &segments, mode, &region, &bands)`（クリックを受ける範囲）
- 3 つの関数は `crates/areka-emo-text/src/choice.rs` の公開の純粋な関数。
- 検査 `crates/areka-emo-text/src/actor_lookahead_shapes_tests.rs` の `choice_highlight_and_click_ranges_come_from_the_judged_rows` は、`present_frame` が DWrite の実測の字幅を使い「決まった字幅」（要件 5.5）と食い違うため、`arrange_for_test` の行の列へ上の 3 つを同じ順で呼び直している。本番が同じ `lines` を 3 か所へ渡すことは、今は構造（束縛が 1 つ）で確かめているだけ。

## Desired Outcome

- 「行の列 → 選択肢の範囲・帯・クリックを受ける範囲」を本番の関数 1 つ（GPU の資源を要らない形）にまとめ、`present_actor` も検査もそれを呼ぶ。
- 検査は本番の関数を呼ぶだけで、並びを組み直さない。本番の並びを崩す変異で検査が赤になる。
- 見た目・クリックの結果は変えない（既存の選択肢の検査が書き換えなしで緑）。

## Approach

- `annotate_lines`・`line_bands`・`derive_hit_rows` をまとめる関数を `choice.rs`（純粋な層）か `actor_present.rs` に置き、`present_actor` から呼ぶ。`decorate_canvas` と描画は今の位置のまま、まとめた関数の結果を使う。`changed` のときだけクリックの範囲を作る今の省き方は保つ（まとめた関数を 2 段に分けるか、遅延にするかは設計で決める）。
- 検査用の読み口（`arrange_for_test` と同じ形の `#[cfg(test)]`）から、決まった字幅でその関数を呼べるようにする。

## Scope

- **In**: 本番の手順を関数 1 つにまとめる・`present_actor` の呼び替え・検査 8 の選択肢の判定をその関数の呼び出しへ替える・本番の並びを崩す変異で赤になることの確認。
- **Out**: 選択肢の見た目・強調の色・クリックの判定の規則の変更。`decorate_canvas` と描画の段の作り替え。

## Boundary Candidates

- 純粋な層（行の列 → 範囲）と、描画の段（GPU・面の反映）の境目。

## Out of Boundary

- 選択肢のイベント（`OnChoiceSelect` など）・ホバーの扱い。
- `present_frame` を決まった字幅で回せるようにすること。

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-budoux-reveal-reflow`（`arrange_lines`・`arrange_for_test`・検査 8）。
- **Downstream**: 選択肢やリンクの範囲を足す spec（`range-choice-tag`・`balloon-link-hover`・`anchor-tag-canon` など）は、まとめた関数へ足せばよくなる。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**: `range-choice-tag`・`balloon-link-hover`・`anchor-tag-canon`（同じ `choice.rs`・`actor_present.rs` を触る）。

## Constraints

- 規模は S 見込み（数タスク）。`crates/areka-emo-text/src/lib.rs` の純粋な一覧・1 ファイル 1,000 行の番人を守る。ビルドとテストは `-j 2`。
