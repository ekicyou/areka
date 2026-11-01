# Brief: emo-balloon-merge-log-overlap

> 2026-11-01 `emo-bake-log-overlap` の完了時の棚卸で `/kiro-discovery` で起票した（対話なし・完了した spec からの引き渡し）。同 spec の要件 5.2 と設計の決定 5 で、例外 2 は完了の棚卸で起票して後続へ回すと決めてある。起票の条件は 2 つ。進行中の `shell-scale-linkage` が `crates/areka/src/placement/mod.rs` を触るので重なる。また、2 行出すのを良しとした前の取り決め（R6.3）を改めるので裁定が要る。区分 **C**。

## Problem

- **起動の記録を読む人・記録を数える道具**: バルーンの 2 層マージ `load_scope_balloon_model`（`crates/areka-emo-present/src/balloon.rs` 548 行あたり）は、起動の準備の中で scope ごとに 2 回呼ばれる。
  - 窓の位置の読み（`crates/areka/src/placement/mod.rs` 825 行あたり）
  - 資産の組み立て（`crates/areka-emo-present/src/balloon_definitions.rs` 98 行あたり、`build_balloon_target` の読み込み）
- そのため、次の 2 つが scope ごとに 2 行ずつ出る。
  - 基層の `descript.txt` や上書き層が読めないときの `warn!`
  - `windowposition`／`validrect` の確定の `info!`
- `emo-bake-log-overlap` は、焼く段と面の表の畳み込みの重なりだけを 1 行にした。この 2 層マージは同 spec の設計の決定 5 で範囲外にした。

## Current State

- `load_scope_balloon_model` は基層を `read_descript_layer(.., LoadRecords::Emit)` で読み、記録を出す・出さないを選ぶ口を持たない。
- 説明（`# 観測（R6.3）`）には、2 か所から呼ばれて scope あたり 2 行出ること、そして「値が一致すること自体が権威一元化の生き証人になる」ことが書いてある。2 行は意図して残した記録である。
- `emo-bake-log-overlap` で、記録を出さない入口の型ができた。各クレートが private な `enum Records { Emit, Quiet }` を持ち、分岐と結果は変えず、記録の行だけを止める。`areka-emo-present` には `LoadRecords` がすでにある。

## Desired Outcome

- 起動 1 回（と読み込み直し 1 回）で、2 層マージの読めない `warn!` と確定の記録が scope ごとに 1 行ずつ出る。
- 起動を止める失敗の記録は、どの道でも必ず出す（`logging.md`）。
- 説明の R6.3 の取り決め（2 行で一致を見せる）を、裁定に合わせて書き直す。

## Approach

要件の段で R6.3 の扱いを裁定する。推奨は案 1。

- **案 1（記録を出さない入口を足す）**: `load_scope_balloon_model` に `LoadRecords` を受ける private な核を作り、記録を出さない `load_scope_balloon_model_quiet` を足す。窓の位置の読み（`placement/mod.rs`）をそちらへ替える。資産の組み立ては今のまま記録を出す。一致は両方の道を比べるテストで確かめる（R6.3 の「生き証人」を記録からテストへ移す）。
- **案 2（読むのを 1 回にする）**: 窓の位置の読みで作った `BalloonModel` を資産の組み立てへ運ぶ。重なりそのものが消える。ただし、配置の層の「素の値だけを運ぶ」取り決めと、起動の配線（`ghost_session.rs`・`emo2_boot/assets.rs`）に触れる。

## Scope

- **In**: `load_scope_balloon_model` の記録を出さない入口、`placement/mod.rs` の呼び出しの付け替え、説明の R6.3 の書き直し、起動の準備 1 回の行の数の檻（`placement/measure_bake_log_tests.rs` の形）
- **Out**: 焼く段と畳み込みの記録（`emo-bake-log-overlap` で済み）、バルーンの読み込みの記録の言葉そのもの

## Boundary Candidates

- `areka-emo-present` の 2 層マージ（`balloon.rs`）
- 配置の層の窓の位置の読み（`placement/mod.rs`）

## Out of Boundary

- 資産の組み立て（`balloon_definitions.rs`・`emo2_boot/assets.rs`）の記録の出方（案 1 では替えない）
- `areka-emo-atlas`・`areka-emo-compose` の記録

## Upstream / Downstream

- **Upstream**: ✅ `emo-bake-log-overlap`（記録を出さない入口の型）、`shell-scale-linkage`（`placement/mod.rs` を触る。着地を待つ）
- **Downstream**: 起動の記録を数える道具・実機の確認

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）
- **Adjacent**: `shell-scale-linkage`（`placement/mod.rs`）、完了 `placement-measure-bake-once`（要件 3.2 の例外・設計 D3）

## Constraints

- **優先度の区分**: C
- 規模 S（案 1 で 3〜5 タスクの見込み）
- 着手は `shell-scale-linkage` の着地の後
- 1 ファイル 1,000 行の上限（`placement/mod.rs` は 970 行ほど。足すのは 1〜2 行に留める）

## 2026-11-01 棚卸㉝の再測定（main 13b80ab1）
- **前提の変化**: 待つ相手の ✅ `shell-scale-linkage` が 11-01 に着地した。同 spec が `crates/areka/src/placement/mod.rs` に足したのは mod の宣言 1 行（`balloon_syncscale`）だけで、今 971 行。2 層マージ `load_scope_balloon_model` は `crates/areka-emo-present/src/balloon.rs`（795 行）548 行目のままで、基層は 555 行目で `LoadRecords::Emit` に決め打ち。窓の位置の読みの呼び出しは `placement/mod.rs` 826 行目（98 行目の use で既に `resolve_balloon_faces_quiet` と並ぶ）、資産の組み立ては `balloon_definitions.rs`（184）98 行目。
- **触るファイル**: `crates/areka-emo-present/src/balloon.rs`（795・案 1 で十数行）・`crates/areka/src/placement/mod.rs`（971・use の名前と 826 行目の呼び出しを替えるだけで行は増えない）・両方の道を比べるテストと行の数の檻（`balloon_model_tests.rs` か `placement/measure_bake_log_tests.rs` 300 の隣。新しい兄弟のファイルにすると `placement/mod.rs` に 1 行＝972 で上限の内）。案 2 なら `ghost_session.rs`（943）・`emo2_boot/assets.rs`（540）にも入る。
- **規模**: 案 1 で 3〜5 のまま。
- **先に要るもの**: 働きの上では無い（先頭）。v0.1.0 の `balloon-communicatebox` とは重ならない（向こうのバルーンの読み手は `areka-parsers` の `balloon/` で、emo-present の `balloon.rs` も `placement/mod.rs` も触らない）。`balloon-dpi-switch-redraw` が直しを emo-present の `balloon.rs` に置くなら同じファイル。
- **机・待ち**: 静かな机・前後の全体テストの比べ・長い走行・人や外の待ち・依存の追加・emo-text の新しいファイルは要らない。R6.3 の扱いは要件の討議で決める（推しは案 1）。
- **優先度の区分**: C のまま。
- **要件定義のモデル**: Opus（記録を出さない入口の型は ✅ `emo-bake-log-overlap` の前例どおり）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 本文の `placement/mod.rs`「825 行あたり」は 826 行目。案 1 は記録を出す入口を残すので、記録の宛先を数える emo-text のテスト（`tests/staysee_balloon_fixture/faces.rs` ほか）と `mcp/dump_surface_gpu_test_support.rs` 312 行目の呼び出しは替えなくてよい見込み。
