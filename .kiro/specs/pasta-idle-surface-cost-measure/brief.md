# Brief: pasta-idle-surface-cost-measure

> 2026-11-14 `wintf-visual-damage-redraw` の完了の棚卸で `/kiro-discovery` で起票。同 spec の要件の討議（議題 3）で、件 1 を同 spec から外し、測ることが本体の spec として別に起こすと決めた（`.kiro/specs/completed/wintf-visual-damage-redraw/requirements.md` の 14 行目と Out of scope）。同 spec の brief の「件 1」の節は、下の「出どころ」へそのまま移した。

## Problem

- ぱすた（`pasta-in-windows`）が面 1002（動く WebP の玉）を出したまま何もしていないあいだ、areka の release 版の CPU は SSP 2.9.26 の約 3 倍、WorkingSet は約 2 倍だった（2026-10-31 の測り・下の「出どころ」）。
- そのうち表示の適用（emo-present の `ShowSurface`）で説明できるのは 1 コアの約 3% だけで、残りの約 12 ポイントがどこで使われているかは測っていない。どこを直せば SSP の水準に近づくかが決められない。

## Current State（2026-11-14・main で `wintf-visual-damage-redraw` の着地の直前）

- 2026-10-31 の数（CPU 中央値 15.4%・WorkingSet 176 MB）は、描画面ごとの合成の入れ物（`crates/wintf/src/ecs/graphics/composite.rs`・`wintf-d2d-composite-rules`＝11-13 着地）が入る前のもの。入れ物は描画面ごとにテクスチャを 1 枚持つので、WorkingSet も CPU も変わっている見込み＝直す前の数は今の main で取り直す。
- `wintf-visual-damage-redraw` が合成の回ごとに target `wintf::damage` の debug の 1 行を出す（`crates/wintf/src/ecs/graphics/systems/render.rs`・欄は 12＝`entity`・`mode`・`reason`・`items_redrawn`・`items`・`area`・`surface_area`・`shift_area`・`exposed_area`・`present_area`・`compose_us`・`present_us`。書式は `crates/wintf/src/ecs/graphics/visual/draw/README.md`「差分の描き直し」）。入れ物への合成の所要（`compose_us`）と WUC の描画面へ写す所要（`present_us`）が描画面ごとに分かる＝内訳の材料に使える。
- 面 1002 はコマごとに新しい根の命令の列が入るので、`wintf-visual-damage-redraw` の後も全部の合成になる見込み（同 spec の要件の討議）＝差分の描き直しでは縮まない。
- 表示の適用の所要は emo-present の perf の行（`crates/areka-emo-present/src/presenter/timing.rs`・`show.rs` の `timing.emit(`）が測る。適用の外（DComp の commit・seriko のコマ送りと刻み・記録を書く処理・そのほか）には時間の印が無い。
- 測りの道具は `tools/perf/`（`invoke-cpu-sample.ps1`・`invoke-perf-run.ps1`・`perf-compare.py` ほか・交互の手順）。ぱすたの実機の起こし方は `doc/pasta-real-run.md`。

## Desired Outcome

- 面 1002 で何もしていないあいだの areka の CPU の内訳（emo-present の適用・wintf の入れ物への合成・WUC の描画面への写し・DComp の commit・seriko と刻み・記録・そのほか）が、今の main の数字で分かる。
- WorkingSet の内訳（描画面と入れ物のテクスチャ・画像の覚え・そのほか）が数字で分かり、SSP との差（2026-10-31 は 176 MB と 94 MB）のどこが何かが分かる。
- 内訳を記録して開発者に仰ぐ。直すのは数字が出てから（直しが小さければ本 spec で、大きければ別に起票）。

## Approach

1. **直す前の数を取り直す**: 今の main の release 版で、面 1002 の何もしていない走りの CPU と WorkingSet を `tools/perf/` の交互の手順で測る。SSP の同じ面も同じ手順で取り直してよい（2026-10-31 の 5.6%・94 MB と比べる）。
2. **内訳を測る**: `wintf::damage` の行（`compose_us`・`present_us`）と emo-present の perf の行を並べ、足りない所（DComp の commit・seriko のコマ送りと刻み・記録）に時間の印を足す。WorkingSet は描画面と入れ物の数と大きさ・形式から見積もった値と、測った値を並べる。
3. **記録して仰ぐ**: 内訳を `verification.md` などに残し、どこを直すかを開発者に仰ぐ。

## Scope

- **In**: 直す前の数の取り直し（areka と SSP）・CPU と WorkingSet の内訳の測り・足りない所への時間の印・内訳の記録。数字の後に決めた小さな直し。
- **Out**: 差分の描き直しそのもの（`wintf-visual-damage-redraw`）・適用の中の 16 ms を超える尾（`present-emit-tail-latency`）・動く絵の再生の決まり（完了 `animated-image-playback`）・コマ送りの回数を減らすこと。

## Boundary Candidates

- 測りの手順と記録（`tools/perf/` の使い方・`verification.md`）。
- 時間の印（wintf の描画の系・DComp の commit・seriko と刻み）。
- 数字の後の直し（どこになるかは測ってから）。

## Out of Boundary

- `wintf::damage` の行と emo-present の perf の行の形を変えること（`tools/perf/` の判定の台本が読む）。印は新しい行か、既存の行の外に足す。

## Upstream / Downstream

- **Upstream**: `wintf-visual-damage-redraw`（`wintf::damage` の行・完了の手続き中）・✅ `wintf-d2d-composite-rules`（入れ物）・✅ `animated-surface-log-flood`（2026-10-31 の測り）。
- **Downstream**: 内訳から起こす直しの spec・`tick-gate-adoption`（描かないフレームを作る話）・`compose-f16-precision`（入れ物の形式を変えると WorkingSet が変わる）。

## Existing Spec Touchpoints

- **Extends**: なし（`wintf-visual-damage-redraw` の brief の件 1 を引き取った）。
- **Adjacent**: `present-emit-tail-latency`（適用の中の尾・同じ perf の行を使う・同じ静かな机）・`wintf-surface-auto-promotion`（描画面の数と大きさが変わる＝WorkingSet の内訳に効く）・`draw-methods-canon`（emo-present の表示を触る）・`balloon-text-visual-tree`（文字の層の自前のスワップチェーンを無くす＝WorkingSet の内訳に効く）。

## Constraints

- 時刻は正確に扱う。コマ送りの回数を間引いて数字を下げる案は取らない。
- 単発の数字で判定しない。`tools/perf/` の交互の手順で測る。
- 静かな机で 1 本ずつ（ウェーブに並べない）。負荷の走行は赤が出たものだけに絞る。
- SSP でぱすたを起こしているあいだは 127.0.0.1:9276 を握るので、areka のぱすたと同時に起こさない（交互に測る）。
- 実機の根・一時フォルダはワークツリーの `target\` の下だけ。
- 測りのための印で perf の行の形を変えない。

## 2026-11-14 起票時の見立て

- 触るファイル: 印を足すなら `crates/wintf/src/ecs/graphics/systems/render.rs`（239 行・DComp の commit の近く）・`crates/areka-seriko/`（刻み）。記録は spec のフォルダの中。直しは測ってから決まる。
- 規模: S〜M（5〜10・測りが大半＝取り直し 1・印 2〜3・交互の測りと記録 2）。直しは数字の後。
- 先に要るもの: `wintf-visual-damage-redraw` の着地（`wintf::damage` の行が main に入ってから）。
- 優先度の区分: B。件 1 の節は区分を書いていない。何もしていないあいだに SSP の約 3 倍の CPU を使うのは負荷のバグに当たり、測ることが本体の同じ形の `present-emit-tail-latency` も B なので、それに合わせた。
- 席: ウェーブの外（測ることが本体・静かな机で 1 本ずつ）。
- 要件定義のモデル: Opus（測ることが本体）。
- 分割の案: 無し。直しが大きいと分かったら、測りと記録で閉じて直しを別に起票する（`present-emit-tail-latency` の「分割の案」と同じ形）。

## 出どころ（`wintf-visual-damage-redraw` の brief から移した節・そのまま）

## 件 1（2026-10-31 `animated-surface-log-flood` の申し送り）: ぱすたの面 1002 で、何もしていないあいだの CPU が SSP の約 3 倍

> 出どころ: `animated-surface-log-flood` の測り（`.kiro/specs/completed/animated-surface-log-flood/verification.md` のタスク 1 の「作る前の測り」の節〔「適用 1 回の所要を areka の CPU と比べる」〕と、タスク 5.2 の節〔「release の交互の測り直し」〕と、タスク 5.3 の「SSP の基準」）。同 spec の設計の判断 D4 は、wintf の全面の描き直しを縮めないと決めていた＝範囲の外として申し送る。場所と数は 2026-10-31 のもの＝着手時に引き直す。

### Problem

- ぱすた（`pasta-in-windows`）が面 1002（動く WebP の玉）を出したまま何もしていないあいだ、areka の release 版は 1 コアの約 15% を使う。同じ面の SSP 2.9.26 は約 5.6%。WorkingSet も areka 176 MB・SSP 94 MB と 2 倍近い。
- `animated-surface-log-flood` が直したのは記録の量と合成の範囲で、CPU の大半はそこに無かった。残りの大部分は、表示の適用（emo-present の `ShowSurface`）の perf の行が測る段の外で使われている。

### Current State（2026-10-31・`animated-surface-log-flood` の測り）

- areka の release 版・面 1002・何もしていないあいだの CPU（1 コアに対して・直す前と直した後を交互に 2 回ずつ）: 直す前 中央値 15.9%（最大 34.0%）・直した後 中央値 15.4%（最大 30.0%）。WorkingSet はどれも 176 MB のまま。
- SSP 2.9.26・面 1002（面の番号を SSTP で確かめた走り）: 中央値 5.6%・最大 7.2%（トークの行を除く 66 行）・WorkingSet 94 MB。
- 適用 1 回の所要（release 版・面 1002）: 全体 2,944 → 1,987 µs（合成 1,028 → 815 µs・当たりのマスク 203 → 178 µs）。コマ送りは毎秒約 14 回＝適用の全部で 1 コアの約 3%。areka の 15% のうち約 12 ポイントは適用の外にある。
- 適用の外で使われている所は測っていない。見込みは wintf の描画面の全面の描き直しと出す処理（present・DComp の commit）だが、動く絵のコマを進める処理（seriko・刻み）や記録を書く処理の分もありうる＝内訳は分からない。

### Desired Outcome

- 面 1002 で、何もしていないあいだの areka の CPU の内訳（emo-present の適用・wintf の描き直し・present・DComp の commit・seriko と刻み・そのほか）が数字で分かる。測りは `tools/perf/` の交互の手順で、単発の数字で判定しない。
- 内訳のうち描き直しと出す処理の分を本 spec の差分の描き直しで縮め、SSP の水準（中央値 5〜6%）を目標にする。届かなければ、残りの内訳と理由を数字で記録して開発者に仰ぐ。
- WorkingSet の差（176 MB と 94 MB）も内訳を測り、本 spec で縮められる分（描画面と入れ物の大きさ・形式）と、そうでない分を分けて記録する。

### Approach

1. **まず測る**: 面 1002 の何もしていない走りで、wintf の描画の系（`ecs/graphics/systems/render.rs` ほか）・present・DComp の commit・emo-present の適用・seriko の刻みに時間の印を足し、1 コマあたりの所要と CPU の割合を並べる。上の議題 2（効きを測ってから入れるか）の答えの材料にもなる。
2. 内訳で描き直しと出す処理が大きければ、本 spec の本体（変わった範囲だけを描き直す）でそこを縮め、前後を交互に測る。
3. 描き直しの外（刻み・seriko・記録）が大きいと分かったら、本 spec は測りと記録で閉じ、直しは別に起票する（`present-emit-tail-latency` の「分割の案」と同じ形）。

### Constraints

- 時刻は正確に扱う。コマ送りの回数を間引いて数字を下げる案は取らない（動く絵の再生の決まりは完了 `animated-image-playback` で確定）。
- 比べる相手は SSP の同じ面（面 1002）・同じ版（2.9.26 以上）。SSP の測りは `animated-surface-log-flood` の 5.3 の手順（面の番号を SSTP の `GetProperty` で毎行確かめる）で取り直してよい。
- 規模の見立て: 本文の 8〜12 に、測り 2〜3 タスク（印を足す・交互の測りと記録）を足す。
- 版: 本 spec は v0.2.0（`wintf-visual-tree`・`wintf-d2d-composite-rules` の後）。測りだけを先に取りたいときは、着手の前に開発者が決める（下の議題 3）。

### 議題（本文の議題に足す）

3. **測り（Approach の 1）を v0.2.0 の前に別に取るか**: 内訳が分かれば、描き直しが主因かどうか（＝本 spec で縮むか）が先に決まる。取らなければ、本 spec の着手のときに測る。
