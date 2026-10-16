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

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 起票（10-06）の後、関係のファイルは変わっていない。`crates/areka-emo-text/src/actor_present.rs` の `present_actor` の並び（`annotate_lines` → `line_bands` → 強調の描画 → 描き替えがあったときだけ `derive_hit_rows`）も、検査 8 が同じ 3 つを呼び直している所（`actor_lookahead_shapes_tests.rs`）もそのまま。
- **触るファイル**:
  - `crates/areka-emo-text/src/choice.rs`（744 行）か `actor_present.rs`（410 行）のどちらかにまとめた関数を置く。
  - `crates/areka-emo-text/src/actor.rs`（検査用の読み口）・`actor_lookahead_shapes_tests.rs`（421 行）。
  - 新しいファイルは足さない＝`lib.rs` に触らない。
- **規模**: S（3〜4 タスク）。
- **先に要るもの**: 無い（`budoux-reveal-reflow` は着地済み）。
- **優先度の区分**: C（バグではない。検査が本番の並びを見張れない穴＝`budoux-reveal-reflow` の持ち越し）。
- **要件定義のモデル**: Opus。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**:
  - `anchor-tag-canon`・`range-choice-tag`・`choice-marker-styling` と `choice.rs`・`actor_present.rs`・`actor.rs` が重なる＝同じウェーブに置けない。
  - **`anchor-tag-canon` の前に置くのが素直**（後にすると、アンカーの範囲を足した後の並びをもう一度まとめ直す）。前に置く席が無ければ、`anchor-tag-canon` の最初のタスクに入れる手もある（合わせて 16〜20 タスク）。
  - `reflow-scroll-path-test` とは、あちらが `actor_lookahead_tests.rs` に足す形なら重ならない。`balloon-text-bugs` の件 2 とは、あちらが `actor_present.rs` で止める設計なら重なる。

## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）

- **前提の変化**: `anchor-tag-canon` が着地した（10-10）。上の棚卸㉓の「起票の後、関係のファイルは変わっていない」と「`anchor-tag-canon` の前に置くのが素直」は古い。
  - `present_actor` の並び（`annotate_lines` → `line_bands` → 強調の描画 → 描き替えがあったときだけ `derive_hit_rows`）は同じ。3 つの関数は、選択肢とアンカーを種類つきの同じ列で扱うようになった。
  - **同じ 3 つを検査の側で呼び直す所が 2 か所に増えた**: 前からの `actor_lookahead_shapes_tests.rs` に加えて、`anchor-tag-canon` の新しい検査 `choice_anchor_tests.rs` の補助の関数も、同じ順で 3 つを呼んでいる。まとめた関数へ替えるのは 2 か所。
- **触るファイル**: `crates/areka-emo-text/src/actor_present.rs`（411 行・呼び替えは必ず要る）・`choice.rs`（748 行・まとめた関数をここに置くなら）・`actor.rs`（663 行・検査用の読み口）・`actor_lookahead_shapes_tests.rs`（421 行）・`choice_anchor_tests.rs`（273 行）。新しいファイルは足さない＝`lib.rs` の席は使わない。
- **規模**: S（3〜5 タスク）。
- **先に要るもの**: 無い＝今すぐ着手できる。
- **優先度の区分**: C のまま。**要件定義のモデル**: Opus のまま。
- **見つけた穴・古くなった記述**:
  - 同じウェーブに置けない相手: `range-choice-tag`・`balloon-font-file`（`actor.rs`）・`balloon-text-bugs`（`actor_present.rs`）。`choice-marker-styling` とは、まとめた関数を `actor_present.rs` に置けば重ならない（`choice.rs` に置くと重なる）。
  - 並べられる相手: `font-size-keywords`・`real-run-log-noise`（重なり 0）。
  - `range-choice-tag` より前に入れると、あちらが範囲の選択肢の検査を書くときに、まとめた関数を最初から呼べる。後になっても直す所は増えない見込み（範囲の選択肢は、種類つきの同じ列に乗るだけ）。

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）

- **前提の変化**: `balloon-text-bugs` が着地し（10-16）、先に要るものは無くなった。`crates/areka-emo-text/src/actor_present.rs` の `present_actor` の並び（行の割り当て → 行の帯 → 強調の描画 → 描き替えがあったときだけ当たりの行）は同じ。変わったのは次の 3 つ:
  - 描画範囲が潰れた場所は、並びに入る前に描かずに抜ける入口が足された。
  - 済んだ選択肢を強調から外す絞り（ホバーの印）と、当たりの写しから外す絞りが足された。どちらも 3 つの関数の外（呼び手の側）にある。まとめた関数にこの絞りを入れるかは設計で決める（入れなくても見た目と当たりは今と同じ）。
  - 検査の側で 3 つを並べ直す所は 2 か所のまま（`actor_lookahead_shapes_tests.rs` と `choice_anchor_tests.rs` の補助の関数）。`layout_cluster_tests.rs` も 3 つを同じ順で呼ぶが、関数ごとの検査で本番の並びの見張りではない。`layout_styled_tests.rs` に足された検査は当たりの行の関数だけを呼ぶ。
- **触るファイル**（行数）: `crates/areka-emo-text/src/actor_present.rs`（435）・`choice.rs`（748・まとめた関数をここに置くなら）・検査 `actor_lookahead_shapes_tests.rs`（421）・`choice_anchor_tests.rs`（273）。`actor.rs`（768）は、検査用の読み口 `arrange_for_test` をそのまま使い、決まった字幅を検査の側からまとめた関数へ渡せば触らずに済む。
- **新しいファイルと依存**: 足さない＝emo-text の `lib.rs` の一覧の席を使わない。依存の追加も無い。
- **規模**: 3〜5 タスク。
- **人の手・机**: 実機の確かめは要らない（見た目と当たりを変えない直しで、決定論の検査だけで確かめられる）。静かな机・前後の全体テストの比べも要らない。
- **先に要るもの**: 無い。v0.0.3 の 2 本とは触るファイルが重ならない。同じウェーブに置けない相手: `actor.rs` を触る形にしたときの `range-choice-tag`・`balloon-font-file`、`choice.rs` に置いたときの `choice-marker-styling`、`actor_present.rs` を触る `bottom-row-text-clip`。本 spec が `actor.rs` に触らず、`range-choice-tag` が `actor_present.rs`・`choice.rs` に触らなければ、`range-choice-tag` と同じウェーブに置ける。`real-run-log-noise` とは重なり 0。
- **優先度の区分**: C のまま（バグではない。検査が本番の並びを見張れない穴）。
- **要件定義のモデル**: Opus のまま（新しい境目も並行の決めごとも無い）。
- **分割の案**: 切らない。
- **見つけた穴・古くなった記述**: 上の棚卸㉔の行数（`actor_present.rs` 411・`actor.rs` 663）は古い。`actor.rs` を「検査用の読み口」として触る前提は要らない見込み。
