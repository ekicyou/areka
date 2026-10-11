# Brief: areka-P0-seriko-lottery-ended-play-skip

> 起票: 2026-10-10（`areka-P0-seriko-trigger-intervals` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes の 4.1（査読の指摘）と 4.1-fix（範囲外の観察）。コードの読みで見つけた件で、実機で絵の誤りを見たものではない。区分 B（バグ）。

## Problem

抽選で始まる animation（interval `random,数値`・`bind+random,数値`・`sometimes`・`rarely`）は、1 秒ごとの境界で「今再生中でなければ」くじを引く。ところが「再生中か」を、境界の時刻に再生が続いているかではなく、「再生の記録がまだ片付けられていないか」で測っている。境界より前に終わった再生でも、片付けがその境界の刻みまで残っていると、その境界のくじを 1 回飛ばし、乱数も引かない。

- **ゴーストの作者**: まばたきなどの抽選の animation が、書いた確率より少し起きにくくなる場面がある（再生の終わりから次の境界までの間に刻みが 1 回も入らなかったとき）。
- **開発者**: 同じ形の件を、`seriko-trigger-intervals` は 3 語（`runonce`・`periodic`・`talk`）の側でだけ直した。抽選の側は、乱数の消費の並びが変わるので触らなかった（同 spec の要件 8.2「`random`・`bind+random`・`sometimes`・`rarely`・`always`・`bind` の決定論のテストの結果と乱数の消費の並びを変えない」）。

## Current State

2026-10-10 時点（`seriko-trigger-intervals` のブランチ・コミット `84ca70e6`）。境界は 1,000 ms の絶対の格子（`crates/areka-seriko/src/timeline.rs` の `LotteryBoundary`）。

- **一番上の面**（`crates/areka-seriko/src/looper.rs` の `LoopRuntime::on_tick`）: 手順の (2)「抽選」が (3)「進行」より前にある。(2) は、再生の表 `playback` にその animation が載っていれば `continue` する（「(a) 非再生中のみ対象」）。終わった再生を表から外すのは (3) の `put_top_play`（末尾に着いた・負の番号で止まった）なので、境界をまたいだ刻みで初めて終わりに気付く再生は、(2) の時点ではまだ表に載っている。その境界ではくじを引かない。
- **部品**（`crates/areka-seriko/src/parts.rs` の `PartClocks::advance` の中の 1 段目の評価）: 同じ形。`playing = matches!(clocks.get(&key), Some(PartAnim::Playing { .. }))` を見てから `should_fire` を呼び、その後で `progress` が終わった時計を片付ける（末尾のコマを保つ時計 `PartAnim::Residual` へ移すか、消す）。
- **起きる条件**: 再生が終わる時刻と次の境界の間に、刻みが 1 回も入らないこと。刻みは約 16 ms ごとなので、ふだんは境界の直前の 1 刻み分の窓だけ。刻みが遅れたときは、その分だけ窓が広がる。抽選の再生は境界をまたいだ刻みの時刻で始まるので、長さが 1,000 ms の倍数に近い animation は終わりが次の境界の刻みの近くに来て、当たりやすい見込み（どのくらいの頻度で起きるかは測っていない＝要件の段で、偽の時計のテストで数える）。
- **3 語の側の直し**（比べる相手）: `crates/areka-seriko/src/trigger.rs` の `Armed::poll` は、「再生中か」を判定の時刻でなく、返すことになる開始の時刻で呼び手に尋ねる（引数 `playing_at`）。呼び手（`looper_trigger.rs` の `fire_top_triggers`・`parts_trigger.rs` の `fire_part_triggers`）は「その時刻のコマがまだ終わりでないこと」で答え、終えていて片付けていない再生は、その時刻まで進めて片付けてから入れ替える。この直しの前は、長さが周期ちょうどの `periodic` が 2 周に 1 回しか鳴らなかった（同 spec の Implementation Notes の 4.1 の査読の指摘）。
- 見た目の誤りを実機で見たことは無い。同梱の検体の抽選の animation（emo2 の `bind+random` 3 本・`random` 4 本、claudia と konnoyayame の `sometimes` 各 1 本）で頻度を測ってもいない。

## Desired Outcome

- 抽選の「再生中か」が、境界の時刻に再生が続いているかで決まる。境界より前に終わった再生は、その境界のくじに入る。
- または、今の振る舞いを「areka の決め」として記録して閉じる（正典は境界と再生の終わりが重なる場面を書いていない）。
- どちらを選んでも、選んだ方が決定論のテスト（乱数を固定した抽選の並び）で固定されている。

## Approach

要件で選ぶ。

- **⑴ 境界の時刻で測る**: 抽選の前に、境界の時刻までに終わった再生を片付ける（3 語の側と同じ「その時刻のコマがまだ終わりでないこと」の述語を使う）。乱数を引く回数と並びが変わるので、抽選の決定論のテスト（一番上・部品・着せ替えつき）の期待値を引き直す。引き直す本数を要件の段で数える。
- **⑵ 記録して閉じる**: `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳の備考に「境界の刻みで終わりに気付いた再生は、その境界のくじを引かない」と書く。コードは変えない。

⑴ を選ぶなら、`seriko-rebuild-hidden-lottery`（見えていない部品のくじ）も同じテストの期待値を引き直すので、2 本を続けて取ると引き直しが 1 度で済む。

## Scope

- **In**: 一番上の面と部品の抽選の「再生中か」の測り方。抽選の決定論のテストの引き直し（⑴ のとき）。決めの記録。
- **Out**: 抽選の確率と境界の間隔。3 語の引き金（`seriko-trigger-intervals` で確定）。見えていない部品のくじ（`seriko-rebuild-hidden-lottery`）。`always`（抽選しない）。

## Boundary Candidates

- `looper.rs` の `on_tick` の (2) と (3) の順、または (2) の前の片付け。
- `parts.rs` の `advance` の 1 段目の、抽選の腕の `playing` の測り方。

## Out of Boundary

- 乱数の出どころ（`LoopRng`）と種。
- 刻みの周期。

## Upstream / Downstream

- **Upstream**: `areka-P0-seriko-trigger-intervals`（✅ 2026-10-10・開始の時刻で「再生中か」を測る形の出どころ。働きの上の依存は無いが、同じ述語と同じファイルを使う）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（⑴ を選ぶと、完了した SERIKO の抽選の決定論のテストの期待値を改める）。
- **Adjacent**（seriko の `looper.rs`・`parts.rs` を触るので直列）: `seriko-rebuild-hidden-lottery`（同じ抽選のテスト）・`seriko-trigger-teardown-gaps`・`seriko-talk-clock-fidelity`・`seriko-interval-combinations`・`seriko-script-triggers`・`animated-image-import`（`parts.rs`）。

## Constraints

- 段は**バグ**・区分 B。
- `looper.rs` は 924 行・`parts.rs` は 893 行（1 ファイル 1,000 行以下）。足すなら兄弟の本番ファイルへ。
- 時刻は正確に扱う。1 フレーム遅らせる解は取らない。
- 規模の見込み: XS〜S（記録だけ 1〜2・直すなら 4〜6 タスク。引き直すテストの本数で上下する）。
- 要件定義のモデル: Opus。
