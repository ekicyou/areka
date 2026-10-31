# Brief: seriko-pattern-calls

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
  - 重ね方の合成メソッド（`overlayfast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*` など＝`draw-methods-canon`・`draw-methods-blend`）。
  - 台本のタグ `\i[*]`・`\![anim,…]`（`seriko-script-triggers` とその後続）。
  - `auto`（element定義の `--source` が要る＝`element-clipping-option` の後の別件）。

## Boundary Candidates

- 語の見分け（解析・対応づけで「重ね方」と「働き」を分ける所）。
- SERIKO の再生での働き（別のアニメーションの開始・停止・差し込み・位置）。

## Out of Boundary

- 合成メソッドの描画そのもの（D2D の重ね方）。
- アニメーションの起こし方（`interval` の抽選＝`seriko-interval-combinations`・`seriko-trigger-lottery-gaps`）。

## Upstream / Downstream

- **Upstream**: なし（今ある SERIKO の再生の上に足す）。
- **Downstream**: `never` のアニメーション（7 語か `\i` でだけ動く）を持つシェル全般。`seriko-script-triggers` の `\i` と同じ「別のアニメーションを動かす」口を分け合いうる。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `draw-methods-canon`（brief が「`move`（位置の語）を範囲に入れるか」を議題に持っていた＝この spec へ移した）・`draw-methods-blend`（`move` を範囲の外とする）・`seriko-script-triggers`（7 語を範囲の外とする）・`audit-seriko`・`audit-emo-compose`（触るファイルが重なる）。

## Constraints

- 正典は ukadoc（SERIKO の pattern定義）。読めない点は SSP の MCP で測る。
- 決定論のテスト必達。
- 検体 `dot_sakura` は `vendors/sample_ghost/dot_sakura.nar`（`sample-ghost-kit` の `SAMPLES` に登記済み）。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: `descript-window-keys`（#374）が seriko に、シェルの差し替えの戻り先を足した（新しい `crates/areka-seriko/src/shell_fallback.rs`・`actor.rs` 768 → 799・`state.rs` 777・`lib.rs` に mod の 1 行）。7 語と `move` の扱いは変わらない＝表を作るとき `ComposeMethod::from_name`（`crates/areka-seriko/src/table.rs` 311 行目）が 8 語とも WARN を出して `Unknown` にし、描かない。`crates/areka-emo-compose/src/method.rs` の `is_move_name`（189 行目）の説明は、正典の言い方（土台の層の表示位置をずらす）に直った。
- **触るファイル**（行数）: 語の見分け＝`method.rs`（420）・`world.rs`（412・`targets_animation_id` は 311 行目）。読み手＝`crates/areka-parsers/src/shell/{lexer.rs 222, decode.rs 598, model.rs 301}`。再生＝seriko の `table.rs`（901）・`looper.rs`（924）・`parts.rs`（893）・`actor.rs`（799）か新しいファイル（4 つとも上限に近い＝足す分は兄弟の新しいファイルへ）。`move` は合成の命令列を作る `crates/areka-emo-compose/src/plan.rs`（706・`derive_ops` が `PatternState` を受けて層を積む所）で土台の層をずらす＝`plan.rs` に触るのはほぼ確か（位置を運ぶなら `pattern.rs` も）。外形の `plan_extent.rs`（198）。兄弟のテスト・網羅台帳・`doc/COMPAT_ARCHITECTURE.md` §8。
- **規模**: 12〜18（新しい書式だけ）のまま。
- **先に要るもの**: 働きの上で無し。同じウェーブに置けない相手（v0.1.0）: `animated-surface-log-flood`（向こうが件 25 の直しで emo-compose の `plan.rs`／`lib.rs`／`pattern.rs` に触るとき＝向こうが直しを emo-present の中〔`display.rs`・`cache.rs`〕に留めれば 0）・`shell-mouse-cursor`（本 spec が `areka-parsers/src/shell/` に新しいファイルを足すときだけ＝`shell/mod.rs`・足さなければ 0）・`mcp-strict-errors` の「再生まで印を運ぶ」案（seriko の `actor.rs`）。版の外と分け合うファイル: `extent-element-offset`（`plan_extent.rs`）・`animated-image-import`・`seriko-trigger-lottery-gaps`（seriko の `table.rs`・`looper.rs`・`parts.rs`・`actor.rs`）・`seriko-missing-surface-default`（いつか・seriko の面の切り替え）。`alternativestart` の候補の選び方の乱数の出どころは要件の段で決める（抽選のテストの期待をずらさない）。1 本だけの席・依存の追加は無い。どっとさくらの実機の 1 回と、読めない点の SSP の測りが要る。
- **優先度の区分**: A のまま（開発者の条件「次のリリース v0.1.0 までに治ること」）。中身はバグに近い。
- **要件定義のモデル**: Opus のまま（今の `interval` の時計で複数のアニメーションを回す作りと合成の上に足すだけ・新しい境目や並行の約束は無い）。
- **分割の案**: 棚卸㉙のまま。本文の「新旧の書式の両方」を残すなら 2 本＝`seriko-pattern-calls`（新しい書式 `animationN.patternM` の 7 語と `move`）／旧 SERIKO 1.x の書式の読み取り（仮の名 `seriko-legacy-pattern-format`・読み手の `lexer.rs`・`decode.rs`・`model.rs` だけ・全部の語が対象）。2 本は読み手のファイルで直列。どっとさくらに要るのは新しい書式だけ。
- **見つけた穴・古くなった記述**（棚卸㉙のうち今も正しいもの）: ⑴ 行を区切る `split_csv`（`lexer.rs` 195 行目）は角括弧の中のカンマしか守らない＝`alternativestart,(0,1)` の候補の並びは `(0` と `1)` の 2 つの欄に割れ、番号の欄は数として読めずに 0 になる（`decode.rs` の `field_i64`）。⑵ 旧書式は今どの語も読んでいない（`decode_animations` は `animation` で始まらない行を飛ばす）。⑶ 本文 Approach の「`move` は面の位置を動かす語」は、正典の言い方では「ベースレイヤの表示位置をずらす」（コードの説明は直った）。
