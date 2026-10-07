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
