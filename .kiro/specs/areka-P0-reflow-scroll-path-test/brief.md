# Brief: areka-P0-reflow-scroll-path-test

> 2026-10-06 起票（`/kiro-discovery`）。出どころは spec `areka-P0-budoux-reveal-reflow` の完了時の棚卸（最終確認の点検「要件 4.2・4.3 は構造上の保証だけ」）。調停役 areka棚卸 の指示で起票。バグではなく、検査の網羅の穴。

## Problem

- **開発者**: `budoux_newline,1` のバルーンで、区間の全文で配置する枝（`TalkLookahead::basis` が全文を返すとき）と、あふれのスクロールを組み合わせて踏む検査が無い。字が欄からあふれてスクロールが起きたとき、表示済みの字の行の割り当てが変わらない（`budoux-reveal-reflow` 要件 4.2）、スクロールの仕組みそのものを変えない（要件 4.3）ことが、今は構造（`visible_window` が行の列だけの関数であること）でしか保証されていない。

## Current State

- 本番の経路: `crates/areka-emo-text/src/actor_present.rs` の `present_actor` が `arrange_lines` の行の列を `LayoutEngine::visible_window` へ渡してあふれの窓を決める。
- 既存のあふれの検査 `crates/areka-emo-text/src/actor_scroll_retain_tests.rs`（`present_actor_scroll_keeps_first_visible_line_ink_k1`・`_k2`）は先渡し無しで流すので、届いた字で配置する枝しか通らない（全文の枝を踏まない）。
- 経路の検査の支え（`crates/areka-emo-text/src/actor_lookahead_tests.rs` の `Rig`・`Talk`・`reveal_violations`・`budoux_model`）は、先渡しありで本番の経路を歩き、決まった字幅で行の列を取れる。今の検査 1〜11 はどれも欄にあふれない台本。

## Desired Outcome

- 先渡しありの台本を `budoux_newline,1` の欄からあふれさせ、合図が届くたびに次を判定する経路の検査がある:
  - 表示済みの字の行の割り当て（同じ行に並ぶ字の組と順）が変わらない（行がまとめて上へ送られるのは「変わる」に含めない）。
  - あふれの窓（見える行の範囲）が、先渡しなしで同じ台本を流したときの出終わりと同じ。
- 全文の枝を外す変異（`basis` が常に届いた字で、を返す）で赤になる。

## Approach

- `actor_lookahead_tests.rs` の支えを使い、あふれる長さの台本（区間の中で 3 行以上になり、欄に 2 行しか入らない高さ）を流す。判定は行の列と `visible_window` の結果の両方。
- 検査ファイルが 1,000 行を超えるなら兄弟のファイルへ分け、`lib.rs` の `SOURCES_OUTSIDE_THE_PURE_SCAN` に足す。

## Scope

- **In**: 経路の検査の追加と、変異で赤になることの確認。
- **Out**: あふれのスクロールの仕組みの変更。フェード（`balloon-scroll-fade` の担当）。

## Boundary Candidates

- 検査だけ（本番のコードは変えない）。

## Out of Boundary

- 描画の画素（インクの位置）での確かめ（既存の `actor_scroll_retain_tests.rs` が受け持つ）。

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-budoux-reveal-reflow`（経路の検査の支え・全文の枝）。
- **Downstream**: `balloon-scroll-fade`・`text-reveal-fade`（表示の途中で字の行が変わらないことを当てにする）。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**: `balloon-scroll-fade`（あふれのスクロールに手を入れる）。

## Constraints

- 規模は S 見込み（2〜3 タスク）。決まった字幅・GPU なし。ビルドとテストは `-j 2`。
