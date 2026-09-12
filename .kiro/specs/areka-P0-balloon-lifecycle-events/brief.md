# Brief: areka-P0-balloon-lifecycle-events

> 起票: 2026-09-11（棚卸⑬・`areka-P0-balloon-canon-residue` の 3 軸分割 ⑵＝表示寿命側の項目 7〜10 を独立 spec に切り出し）。項目本文の正本は分割元 brief の「`balloon-visibility` からの追加登記（2026-08-12）」節。ここでは所有と着地条件だけを書き、本文は重複させない。

## Problem

完了 spec `balloon-visibility` が語彙・Reference 割当・受け渡し口の型（`BalloonLifecycleNotice`・`crates/areka/src/emo2_boot/talk_lifecycle.rs`・`#[allow(dead_code)]` で本 spec 群を名指し）までを残し、実装を先送りした 4 項目が消費者ゼロのまま残っている:

7. `\![set,balloontimeout,時間]` の実導出（表示側）。
8. `OnBalloonClose`／`OnBalloonTimeout`／`OnBalloonBreak` の SHIORI 発火（UI→kanade の通知路＋kanade 送出側の受理）。
9. `\x`／`\x[noclear]`（クリック待ち＝会話の進行を止める機能・可視性側での近似実装は禁止）。
10. 中断で終わった会話のタイムアウト起点の精密化（8 と一体・単独で先行させない）。

## Current State

2026-09-11 実測: `BalloonLifecycleNotice` の `#[allow(dead_code)]` 注記「消費者ゼロ（意図的予約・Requirement 7.8）: 実発火は areka-P0-balloon-canon-residue が所有」が逐語で現存（**所有者名は本 spec へ読み替える**・分割元 brief の追記に登記）。`noclear`・`balloontimeout` は `crates/` で 0 件。emo2 は 3 イベントとも消費者ゼロ・`\x` も `balloontimeout` も辞書に無い（M1 実害なし）。

## Desired Outcome

4 項目が着地し、`#[allow(dead_code)]` の予約が外れる。`\x` はクリックで会話が進み、`\x` は scope を `\0` へ戻して `\f` 系の効果を解除、`\x[noclear]` は内容・scope・`\f` 系を保持する。3 イベントが正典の Reference で発火し、中断時のタイムアウト起点は中断時刻になる。

## Approach

kanade（会話進行）と UI（表示寿命）の間に通知路を 1 本敷く（8・10 の情報は同一）。9 は「会話を止める」機構＝sakura の再生に待ち相を足す（可視性の規則には触れない）。7 は表示側の既定 30 秒を台本の指定で上書きする読み口。

## Scope

- **In**: 項目 7（表示側）・8・9・10。
- **Out**: 項目 7 の compile 側（`\![set,balloontimeout]` の台本コンパイル時の干渉＝`sakura-time-directives` 所有・双方が揃って初めて 7 が成立）／「`\f` 状態の何がリセットされるか」の権威定義（`text-decoration-canon` が供給・本 spec は消費）／可視性の判断規則そのもの（`balloon-visibility` で完成・不変）。

## Boundary Candidates

- 通知路（8・10）／`\x` の待ち相（9）／`balloontimeout` の読み口（7）の 3 片。8・10 は一体。

## Out of Boundary

- 系列解決（分割元 ⑴）・emo-text 側の残件（`emo-text-canon-residue`）。

## Upstream / Downstream

- **Upstream**: `text-decoration-canon`（9 のリセット意味論・**先行必須**）・`sakura-time-directives`（7 の compile 側・**先行必須**）・完了 `balloon-visibility`（送り元）。
- **Downstream**: これらを使う実ゴーストの適合（M2）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-balloon-canon-residue`（分割元・7〜10 を引き継ぐ）。
- **Adjacent**: `translate-pipeline`（kanade `schedule/events.rs` を共有＝同居不可）・`sakura-time-directives`。

## Constraints

- 編集集合の見込み: `crates/areka/src/emo2_boot/talk_lifecycle.rs`・`crates/areka-kanade/src/schedule/{events,steady}.rs`・`crates/areka-sakura/`（`\x` の待ち相）・`doc/COMPAT_ARCHITECTURE.md` §8。
- 正典の曖昧点 1 件を要件で裁定: `balloontimeout` の「`0` または `-1`」（同一項で表現が割れ `-2` の扱いが曖昧）。
- 決定論テスト必達（3 イベントの発火・`\x` の 2 形・中断起点）。要件定義は Opus で足りる（裁定は上の 1 件と `\x` の scope リセット範囲の 2 件）。

> **📌 2026-09-13 相互登記（`areka-P0-text-decoration-canon` 着地）**——項目 9 の「`\f` 状態の何がリセットされるか」の権威定義は `crates/areka-emo-text/src/state_decoration.rs` の `TextLayerState::reset_decoration(scope)` で、`\x` はこれを `None`（全スコープを 1 回で戻す）で呼び、`\x[noclear]` は呼ばない——という配線を本 spec が足す（親 spec の着地時点では `\f[default]`／`\f[disable]`／台詞開始の `ClearAll` の 3 経路だけが同じ実体を通っており、クリック待ちからの呼び出し元は 0 件）。
