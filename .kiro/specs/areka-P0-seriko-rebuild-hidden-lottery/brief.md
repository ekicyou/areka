# Brief: areka-P0-seriko-rebuild-hidden-lottery

> 2026-10-07 `animated-image-playback` の完了時の棚卸で起票（`completed/areka-P0-animated-image-playback/tasks.md` の Implementation Notes の 3.x の範囲外の項）。

## Problem

入れ子の面（element定義の子の面）を持つシェルで、外側の面が別のコマに居る刻みでも、見えていない部品の `random` の抽選が回ることがある。見た目の誤りは確認されていないが、乱数の消費が見えない部品に回るので、見える部品の抽選の並びが「見えている部品だけを引いたとき」と変わる。

## Current State

- `crates/areka-seriko/src/parts.rs` の `rebuild` は、`pattern.clear_parts()` の直後の 1 回目の `nest.visible_parts(…)` で、経過 0 の辺・着せ替えの辺の先の部品も見える部品として評価する（`evaluate(PartKey::Surface(part), pattern)`）。外側が別のコマに居ても、この 1 回目では評価が回る。
- 評価が終わった後、見えない部品のコマは捨てる（`evaluated.len() > visible.len()` の枝）。表示には出ない。
- `surface-element-nesting` からの性質。`animated-image-playback` の前後で引く乱数の数は同じ。

## Desired Outcome

- 見えていない部品の抽選を回さない（見える部品だけが乱数を引く）か、今の振る舞いが正典上も問題ないと確かめて記録して閉じる。

## Approach

要件で決める。⑴ 1 回目から外側の今のコマで見える部品だけを評価する。⑵ 今のままにし、乱数の消費の並びを決まりとして記録する。決定論のテスト（乱数を固定した抽選の並び）で選んだ方を固定する。

## Scope
- **In**: `rebuild` の評価の順と範囲・決定論のテスト。
- **Out**: 抽選の確率・間隔の決まり（`seriko-trigger-intervals`）。

## Boundary Candidates
- `crates/areka-seriko/src/parts.rs`（`rebuild`）・`nest_table` の `visible_parts`

## Out of Boundary
- 動く絵の子の再生（`animated-image-playback` で確定）

## Upstream / Downstream
- **Upstream**: `surface-element-nesting`（✅）・`animated-image-playback`（✅ 10-07）
- **Downstream**: なし

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `seriko-trigger-intervals`（同じ `parts.rs`・`looper.rs` を触る＝同時に走らせない）

## Constraints
- 規模の見立て: XS〜S（記録だけ〜4〜6）。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 起票（10-07）から `crates/areka-seriko/src/parts.rs` は変更 0。Current State は今の main と一致（`rebuild` の 1 回目が見える部品を求めては評価し、終わってから見えない部品のコマを外す）。
- 触るファイル: `crates/areka-seriko/src/parts.rs`（`rebuild`）と兄弟のテスト。案 ⑴ でも、見える部品を求める `visible_parts`（`crates/areka-emo-compose/src/nesting.rs`）は呼ぶだけで変えない見込み。
- 規模: 記録だけで閉じるなら 1〜2、直すなら 4〜6 タスク。切らない。
- 先に要るもの: 働きの上では無し。`seriko-trigger-intervals` と `parts.rs`・`looper.rs` を分け合うので、その後。
- 優先度の区分: C（見た目の誤りが確認されていない持ち越し）。
- 要件定義のモデル: Opus。
- 分割の案: 無し。
- 見つけた穴・古くなった記述: `seriko-trigger-intervals` が `runonce`・`periodic`・`talk` を部品にも足すと、この性質は「乱数の並びが変わるだけ」でなく「見えていない部品で再生が始まる」に変わりうる。あちらの設計で部品の門を見るときに一緒に決めてしまえば、本 spec は記録だけで閉じられる。
