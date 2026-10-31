# Brief: emo-bake-log-overlap

> 2026-10-31 `placement-measure-bake-once` の完了時の棚卸で `/kiro-discovery` で起票（対話なし・完了した spec からの引き渡し）。起票の条件: 同 spec の要件討議（議題 2）と設計討議で、開発者が「残る重なりは完了の棚卸で起票する」と裁定済み。あわせて、直し方が複数あって裁定が要り、別のクレート（`areka-emo-atlas`・`areka-emo-compose`）に新しい口を足す規模である。区分 **C**。

## Problem

- **起動の記録を読む人・記録を数える道具**: 起動の準備で、窓の大きさを測る段（採寸）と、起動の資産の組み立ての両方が、シェルを 1 回ずつ焼き、バルーンを scope ごとに 1 回ずつ焼く。そのため、焼く段と面の表の畳み込みの記録が、同じ絵・同じ面について 2 行ずつ出る。
- 出るのは壊れた絵・特別な絵のときだけ。0 寸の絵・全透明の絵の `warn!`（`areka-emo-atlas`）と、畳み込みの `warn!`（`areka-emo-compose`）である。
- `placement-measure-bake-once` は、シェルとバルーンの読み込みの記録を 1 件 1 行にした。焼く段と畳み込みの記録は範囲外として残した（同 spec の要件 3.7）。

## Current State

- 採寸は `crates/areka/src/placement/measure.rs` の `measure_baked` で、シェルとバルーンの面 0 を焼き、面の表を組む。読み込みの記録は `_quiet` の入口で黙らせてある。
- 資産の組み立ては `crates/areka/src/emo2_boot/assets.rs` の `build_shell_assets`・`build_balloon_assets` で、同じ絵をもう 1 回焼く。
- 例外 2 件（`placement-measure-bake-once` の設計討議で受け入れたもの）:
  1. 1 枚目が全透明の動く絵では、採寸の道で焼く段の「全透明」の `warn!` が 1 行増える。採寸の読み手 `FirstFrameDecoder` が動く絵を静止画として見せるためで、同 spec の要件 3.7 の例外・設計 D4 にあたる。檻は `placement/measure_first_frame_tests.rs` の `fade_in_animation_warns_all_transparent_only_when_wrapped`。
  2. バルーンの 2 層マージ `load_scope_balloon_model`（`areka-emo-present/src/balloon.rs`）の中で基層の `descript.txt` が読めないと、その `warn!` は `windowposition` の読みと資産の面ごとの定義の読みから重ねて出る。同 spec の要件 3.2 の例外・設計 D3 にあたる。
- 実機（ぱすた・emo2）では、ぱすたの動く WebP の 1 枚目は全透明でなく、例外 1 には当たらない（同 spec の research.md 8.2）。

## Desired Outcome

- 起動 1 回（と読み込み直し 1 回）で、焼く段・畳み込み・2 層マージの記録が、同じ絵・同じ面について 1 行ずつ出る。
- 起動を止める失敗の記録は、どの道でも必ず出す（`logging.md`）。

## Approach

要件の段で次から選ぶ（裁定が要る）。

- **案 1（記録を出さない口を焼く段と畳み込みに足す）**: `placement-measure-bake-once` の `LoadRecords` と同じ型の口を、`areka-emo-atlas` の `bake` と `areka-emo-compose` の畳み込み、`load_scope_balloon_model` に足し、採寸の道だけ黙らせる。例外 1 は、採寸の道の焼きを黙らせれば消える。
- **案 2（焼くのを 1 回にする）**: `placement-measure-bake-once` のギャップ分析の案 A。採寸が焼いた物を資産の組み立てへ運ぶ。重なりそのものが消えるが、配置の層の「素の値だけを運び、採寸に使った資産は捨てる」取り決めと `ghost_session.rs` に触れる。
- 推奨の見込み: 案 1（範囲が記録の口に閉じ、取り決めを保てる）。

## Scope

- **In**: 焼く段（0 寸・全透明の `warn!`）・面の表の畳み込みの `warn!`・2 層マージの基層の `descript.txt` の `warn!` の重なりの解消。決定論の檻（記録の行の数）。
- **Out**: 動く絵の読み方・焼く速さ・窓の配置。シェルとバルーンの読み込みの記録（`placement-measure-bake-once` で済み）。

## Boundary Candidates

- `areka-emo-atlas` の焼く段の記録の口
- `areka-emo-compose` の畳み込みの記録の口
- `areka-emo-present` の 2 層マージの記録の口
- 採寸の道（`placement/measure.rs`）と `windowposition` の読み（`placement/mod.rs`）がそれを使う所

## Out of Boundary

- `emo2_boot/switch_assets.rs`（着替えの道は今どおり記録を出す）
- 動く絵の読み込みと再生（`areka-emo-atlas` の読み方・上限）

## Upstream / Downstream

- **Upstream**: ✅ `placement-measure-bake-once`（`LoadRecords`・`_quiet` の入口・`FirstFrameDecoder`・数える檻）
- **Downstream**: 起動の記録を数える自動の試験

## Existing Spec Touchpoints

- **Extends**: なし（完了した `placement-measure-bake-once` の続き）
- **Adjacent**: `placement-crate-split`（`placement/` を動かす）・`shell-scale-linkage`（`placement/mod.rs`）

## Constraints

- 記録の水準・文言・欄は変えない。依存を足さない。1 ファイル 1,000 行以下。
- `placement/mod.rs` は 970 行なので、足すなら別のファイルへ。
- `placement` は examples から `#[path]` で読まれるので、本番のファイルで `crate::` を使わない。
- **優先度の区分**: C（ログの重なり・壊れた絵や特別な絵のときだけ出る）。
