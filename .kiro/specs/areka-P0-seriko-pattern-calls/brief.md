# Brief: areka-P0-seriko-pattern-calls

> 起票: 2026-10-23（`areka-P0-dot-sakura-specimen` のタスク 5.1・開発者と `/kiro-discovery`）。根拠は `.kiro/specs/completed/areka-P0-dot-sakura-specimen/verification.md` の 7 節・8 節の件 4 と件 5。
> **開発者の条件（2026-10-23）: 次のリリース v0.1.0 までに治ること。ただし roadmap.md はこの起票では触らない**（版の段と並びは次の棚卸で決める）。

## Problem

シェルの `surfaces.txt` の pattern定義で、アニメーションを呼ぶ語と `move` が読み捨てられ、そのアニメーションが動かない。開発者のゴースト「．さくら」のシェル `master` では、`animation0.pattern1,move,0,10,0,6` などの `move` が 22 行、`animation2.pattern0,alternativestart,(0,1)` などの `alternativestart` が 5 行、`未知の合成メソッド名: Unknown シームへ吸収` の WARN になった。

## Current State

- 解析: `crates/areka-parsers/src/shell/decode.rs` の `decode_animations` は pattern定義の 2 番目の欄をそのまま `DrawMethod` に写す（捨てない・解かない）。`model.rs` の注記は旧 SERIKO 1.x の並びと語（`overlay`・`base`・`move`・`scaling`・`start`・`stop`・`alternativestart` …）を挙げる。
- 対応づけ: `crates/areka-emo-compose/src/method.rs` の `ComposeMethod::from_name` → `known_method`。アニメーションを呼ぶ 7 語（`start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert`）と `move` は表に無く、WARN を出して `ComposeMethod::Unknown` になる。呼び手は `crates/areka-seriko/src/table.rs`（再生の表を作るとき）。
- 7 語は `crates/areka-emo-compose/src/world.rs` の `targets_animation_id` で「アニメーションの番号を指す語」と見分けられ、ぶら下がりの検査と外形の計算（`plan_extent.rs`・`move` も飛ばす）では飛ばされる。再生（`crates/areka-seriko/src`）にはどの語の扱いも無い＝`Unknown` のまま描かれない。

## Desired Outcome

pattern定義のアニメーションを呼ぶ 7 語と `move` が正典どおりに再生され、「．さくら」のシェル `master` のアニメーションが SSP と同じに動く。WARN は出ない。

## Approach

7 語と `move` を合成メソッドの表（重ね方）から切り離し、pattern の「働き」として SERIKO の再生（`areka-seriko`）が受ける。7 語は別のアニメーションの開始・停止・差し込みを指し、`move` は面の位置を動かす語（重ね方ではない）。それぞれの意味（`alternativestart` の候補の選び方・`parallelstart` の並び・`insert` の差し込み・`move` の座標の意味と重なりの順）は要件の段で ukadoc から引き、読めない点は SSP の MCP で測る。

## Scope

- **In**:
  - 件 1: アニメーションを呼ぶ 7 語（`start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert`）の再生（どっとさくらの件 5）。どっとさくらで見えたのは `alternativestart` だけだが、直す場所が同じなので族ごと 1 本にまとめる（記憶「小さいバグの起票は触る場所が同じものを 1 本にまとめる」）。
  - 件 2: `move` で面の位置を動かす（どっとさくらの件 4）。
  - 新旧の書式（SERIKO/2.0 の `animationN.patternM,…` と旧 1.x の並び）の両方。
  - 決定論のテストと、どっとさくらの実機の 1 回（WARN が 0 になり、アニメーションが動く）。
- **Out**:
  - 重ね方の合成メソッド（`overlayfast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*` など＝`areka-P0-draw-methods-canon`・`areka-P0-draw-methods-blend`）。
  - 台本のタグ `\i[*]`・`\![anim,…]`（`areka-P0-seriko-script-triggers` とその後続）。
  - `auto`（element定義の `--source` が要る＝`element-clipping-option` の後の別件）。

## Boundary Candidates

- 語の見分け（解析・対応づけで「重ね方」と「働き」を分ける所）。
- SERIKO の再生での働き（別のアニメーションの開始・停止・差し込み・位置）。

## Out of Boundary

- 合成メソッドの描画そのもの（D2D の重ね方）。
- アニメーションの起こし方（`interval` の抽選＝`areka-P0-seriko-interval-combinations`・`areka-P0-seriko-trigger-lottery-gaps`）。

## Upstream / Downstream

- **Upstream**: なし（今ある SERIKO の再生の上に足す）。
- **Downstream**: `never` のアニメーション（7 語か `\i` でだけ動く）を持つシェル全般。`areka-P0-seriko-script-triggers` の `\i` と同じ「別のアニメーションを動かす」口を分け合いうる。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-draw-methods-canon`（brief が「`move`（位置の語）を範囲に入れるか」を議題に持っていた＝この spec へ移した）・`areka-P0-draw-methods-blend`（`move` を範囲の外とする）・`areka-P0-seriko-script-triggers`（7 語を範囲の外とする）・`areka-P0-audit-seriko`・`areka-P0-audit-emo-compose`（触るファイルが重なる）。

## Constraints

- 正典は ukadoc（SERIKO の pattern定義）。読めない点は SSP の MCP で測る。
- 決定論のテスト必達。
- 検体 `dot_sakura` は `vendors/sample_ghost/dot_sakura.nar`（`sample-ghost-kit` の `SAMPLES` に登記済み）。
