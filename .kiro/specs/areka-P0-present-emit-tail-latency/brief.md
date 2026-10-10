# Brief: areka-P0-present-emit-tail-latency

> 2026-10-07 `animated-image-playback` の完了時の棚卸で起票（`completed/areka-P0-animated-image-playback/tasks.md` の Implementation Notes の 1.1・6.2、`research.md`「後の数字」）。

## Problem

シェルの絵を出す適用（emo-present の `ShowSurface`）のうち、約 1.2% が 16 ms を超える（最大 315〜360 ms）。その適用の間はその窓の絵が止まって見える。動く絵（APNG・動く WebP）は適用の回数を約 13 倍にするので、16 ms を超える適用の本数も同じ割合で増え、カクつきが目に見えやすくなった。

## Current State

- 適用の時間は `crates/areka-emo-present/src/presenter/show.rs` の `timing.mark(Stage::…)` で段ごとに測り、関数の終わりの `timing.emit(` が perf の行を 1 行出す。
- 16 ms を超えた適用は**全部**、最後に記録した段（合成の覚えに当たらなかったときは `Stage::MaskGen`、当たったときは `CacheLookup`）から `timing.emit(` までの、どの段にも入らない区間で時間を使っている。この区間に何があるか（`info!` の 1 行・窓寸の照合・`frame_of(world)`・`take_delta` など）は測っていない。
- `animated-image-playback` の前の実行体でも同じ割合で出る（6.2 の A/B 交互の計測）。本 spec が持ち込んだものではない。
- 中央値は速い（A/B 交互で前 p50 0.9〜1.5 ms・後 1.5 ms 前後）。問題は尾だけ。

## Desired Outcome

- 測っていない区間に段を足し、16 ms を超える適用の時間がどこで使われているかが perf の行で分かる。
- 原因を根本で直し、16 ms を超える適用が出なくなる（直せない外因なら、その理由と数字を記録して閉じる）。

## Approach

まず計測（区間に段を足す）→ 原因の特定 → 根本の直し、の順。原因の見立て（記録の書き出し・ロックの待ち・確保・OS の呼び出し）は計測の後に決める。時刻は正確に扱う（設計の大原則）ので、待ちを丸めたり間引いたりして見かけの数字を下げる案は取らない。

## Scope
- **In**: `show.rs` の適用の測っていない区間の計測と、その原因の直し。perf の行の段の追加。
- **Out**: 合成（emo-compose）そのものの速さ・動く絵の再生の決まり・文字の層。

## Boundary Candidates
- 表示の段（`crates/areka-emo-present/src/presenter/show.rs`・`presenter/` の計時）
- 原因が記録や OS の呼び出しなら、その呼び出し元

## Out of Boundary
- 動く絵の適用の回数を減らすこと（再生の決まりは `animated-image-playback` で確定）

## Upstream / Downstream
- **Upstream**: `animated-image-playback`（✅ 10-07・適用の回数が増えた）・draw-load-parity（perf の行と段の計時）
- **Downstream**: 動く絵を使うすべてのゴースト・`seriko-trigger-intervals`

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `tick-gate-adoption`（`apply_show` を対象外と確定済み）・`mcp-strict-errors`（`show.rs` に触る）

## Constraints
- 1 フレーム遅らせる解は取らない。状態の持ち方を変えて解く。
- 計測は `tools/perf/` の A/B 交互の手順で前後を比べる（機械の負荷で桁が動くので、単発の数字で判定しない）。
- 規模の見立て: S〜M（5〜10）。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 起票（10-07）から `crates/areka-emo-present/src/presenter/show.rs`・`timing.rs` は変更 0。段は今も 4 つ（照会・合成・当たりのマスク・表示の記録）で、Current State は今の main と一致。
- 触るファイル: `crates/areka-emo-present/src/presenter/{show.rs, timing.rs}` と兄弟のテスト `timing_tests.rs`・`presenter_perf_log_tests.rs`（883 行）。perf の行に段が増えるので、`tools/perf/` の読み手が段の名前を見ていればそこも。原因しだいで、その呼び出し元。
- 規模: 5〜10 タスクのまま。**計測が約半分**（3〜4＝段を足す・計時のテスト・perf の行の読み手・前後の測り比べと記録）。直しは 2〜6 で、計測の後でないと決まらない。
- 先に要るもの: 無し（今すぐ着手できる）。`extent-element-offset`・`collisionex-regions`・`seriko-trigger-intervals`・`animated-image-import`・`placement-measure-bake-once` とは重なり 0。`mcp-strict-errors`（`show.rs`）・`balloon-element-order`（`presenter/`）とは同時に走らせない。
- 優先度の区分: B（バグ＝絵が止まって見える）。
- 要件定義のモデル: Opus（まず測る仕事。原因がロックやスレッドの待ちと分かったら、設計は Fable）。
- 分割の案: 無し。計測の結果、直しが emo-present の外で大きいと分かったら、計測と記録で閉じて直しを別に起票する。
- 見つけた穴・古くなった記述: 本文は測っていない区間の中身を「`info!` の 1 行・窓寸の照合・`frame_of(world)`・`take_delta`」と書くが、実際にはその前に、窓の中身へ絵を渡す処理が全部入っている＝初回の表示の部品の生成・`set_display`・`set_layout`・当たりのマスクの受け渡し・`set_visible`・遷移の記録の 2 行（どれも `show.rs` の「当たりのマスク」の印の後）。wintf の World への書き込みが主な候補なので、段は「絵を渡す塊」と「記録の塊」に分けて足す。
