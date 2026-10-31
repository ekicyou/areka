# Brief: choice-ranges-one-function

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

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: ✅ `vertical-choice-hover-band`（10-24・`choice.rs` の帯の関数 `line_bands` の縦書きの枝）・✅ `real-run-log-noise`・✅ `balloon-scroll-arrows`（10-31・`present_actor` に読み返しの窓と矢印の印を足した）が着地し、`crates/areka-emo-text/src/actor_present.rs` は 435→555 行、`choice.rs` は 748→790 行、`choice_anchor_tests.rs` は 273→340 行になった。本番の並び（`actor_present.rs` の 316 行目の範囲 → 325 行目の帯 → 336 行目の強調の描画 → 347 行目の描画 → 描き替えがあったときだけ 367 行目の当たり）は同じで、読み返しの窓は並びの前（301 行目）で決まる。検査の側で同じ 3 つを並べ直す 2 か所は `actor_lookahead_shapes_tests.rs` 183〜185 行目と `choice_anchor_tests.rs` 131〜133 行目（ずれた）。
- **触るファイル**（行数）: `actor_present.rs`（555）・検査 `actor_lookahead_shapes_tests.rs`（421）・`choice_anchor_tests.rs`（340）。まとめた関数を `choice.rs`（790）に置くならそれも。`actor.rs` は検査用の読み口 `arrange_for_test` をそのまま使えば触らない。新しいファイルは足さない＝emo-text の `lib.rs` の一覧の席を使わない・依存の追加も無い。
- **規模**: 3〜5 タスク。上限 20 の内。
- **先に要るもの**: 無い（先頭）。v0.1.0 との重なり: まとめた関数を `actor_present.rs` に置けば 0。`choice.rs` に置くと、`script-small-tags` が議題（`\![*]` の印を描くか）で emo-text の選択肢の飾り `choice.rs` に触れるときに重なる＝隙間に入れるなら `actor_present.rs` に置く形に限る。済んだ選択肢を外す絞りとホバーの絞りは今も 3 つの関数の外（呼び手の側）＝まとめた関数に入れるかは設計で決める。
- **人の待ち・机**: 要らない（見た目と当たりを変えない直しで、決まった字幅の検査だけで確かめる）。静かな机・前後の全体テストの比べも要らない。
- **優先度の区分**: C（バグではない。検査が本番の並びを見張れない穴＝`budoux-reveal-reflow` の持ち越し）。
- **要件定義のモデル**: Opus（新しい境目も並行の決めごとも無い）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**: 同じウェーブに置かない相手は roadmap の文字とバルーンの列（`range-choice-tag`・`choice-marker-styling`・`balloon-font-file`・`text-typesetting`・`anchor-style-canon`）。v0.1.0 の後の見直し `audit-emo-text-state-actor` が `actor*`・`choice*` を持つ＝この spec を先に取れば先に済む。後に回すなら同じウェーブに置かない。
