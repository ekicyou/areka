# Brief: window-resetwindowpos-hold

> 2026-10-30 `window-work-area-clamp` の完了時の棚卸で `/kiro-discovery` で起票（対話なし・完了した spec からの引き渡し）。出どころは同 spec の最終検証の「棚卸へ渡す範囲外の項目」の ①（`completed/window-work-area-clamp/tasks.md` の Implementation Notes「範囲外」と「3.3 の預かりの決まり」・3.3 のレビューの観察）。起票の条件: 直し方が複数あり（並べ直しも全部の窓の最初の表示を待たせる・並べ直しが預かった移動を捨てる今の決まりとの合わせ方）、実機の確かめが要る。区分 **B**。

## Problem

- **ゴーストの作者・利用者**: 起動の直後のトーク（`OnBoot`・`OnFirstBoot` の台本の先頭など）で `\![execute,resetwindowpos]`（並べ直し）を流すと、キャラ窓がまだ最初の面を表示していないうちに、表示の前の仮の大きさで並べてしまう。最初の面が出て窓の大きさが変わった後、窓どうしの間に隙間が残りうる（`window-work-area-clamp` の 3.2 で `\![move]` について測った 52px の隙間と同じ形）。
- SSP は初回起動の台本をシェルを出し終えた後に流す（`research.md` 12 節）。SSP では起きない見込み。

## Current State

- `window-work-area-clamp`（2026-10-30 完了）は、位置の指令を、対象の窓と基準の窓が最初の面を表示するまで預かるようにした（要件 6.2・`crates/areka/src/emo2_boot/frame/move_hold.rs` 187 行目の `release_ready`・270 行）。預かるのは `\![move]` だけで、表示を待つ判定は `unshown_scope`（134 行目）。
- 並べ直しは表示を待たない: `touched_scopes` は `ResetWindowPos` に「全部の窓」（`None`）を返すだけ（126 行目）で、`unshown` の判定は `PositionCue::Move` にしか当たらない（235〜238 行目）。前に預かった指令が無ければ、並べ直しはその巡でそのまま当たる。
- 並べ直しは前に預かった移動を当てずに捨てる（206〜233 行目・`move_hold_superseded`）。並べ直しが全部の窓を既定の位置へ置くので、順のまま当てたのと結果が同じ、という決まり（同 spec の「3.3 の預かりの決まり」）。捨てないと、最初の面を表示しない窓が 1 つで後の全部の指令が預かられ続ける。
- 並べ直しの本体は `crates/areka/src/placement/reset_pos.rs` 47 行目の `reset_window_positions`。窓の大きさは `WindowPos.size`（80 行目）から読むので、表示の前は仮の大きさになる。材料が無いスコープは `reset_window_pos_skipped` の `warn!` で外す（91 行目）。呼ぶのは `emo2_boot/frame/drain_resnap.rs` 176 行目の腕（148 行目の `run_move_drain_phase`）。
- 実機では確かめていない。4.3 ⑷ の検体（`target\wwac32\root` の写し・トークの先頭に `\![move]`）は並べ直しを流していない。要件 6.2 は移動・前後・組の指令だけが範囲で、並べ直しは範囲の外だった。

## Desired Outcome

- 起動の直後に流した並べ直しが、どのキャラ窓も最初の面を表示した後の大きさで並べる。並べた後に隙間が残らない。
- 並べ直しより前に預かった移動と、並べ直しの後に来た指令の順の決まりが、表示を待つ形でも崩れない（取りこぼさない・追い越さない・表示されない窓が 1 つで列全体が詰まらない）。
- 待ったこと・捨てたことは記録に 1 件ずつ残る。

## Approach

- 第一の案は、並べ直しも「窓のある全部のスコープが最初の面を表示する」まで預かる（`unshown_scope` を並べ直しにも当てる）。そのとき今の「前に預かった移動を捨てる」決まりと、表示されない窓が 1 つあるときに列が詰まらない決まりをどう合わせるかを、要件の段で決める（例: 捨てるのは今どおりで並べ直しだけを預かる／表示を待つ上限を窓の有無で決める）。
- 直す前に赤になる決定論のテストを置く（最初の面の前に流した並べ直しが仮の大きさで並べる）。檻は `frame_move_hold_tests.rs` の兄弟。
- 実機は `window-work-area-clamp` の 4.3 ⑷ と同じ形の検体（emo2 の写し・初回のトークの先頭に `\![execute,resetwindowpos]`）で、並べ直しの後に隙間が無いことを確かめる。SSP の振る舞いは `research.md` 12 節（台本はシェルを出し終えた後）で足りる見込みで、新しく測らない。

## Scope

- **In**: 並べ直しの預かり（`move_hold.rs`）・並べ直しと預かった移動の順の決まり・記録・決定論のテスト・実機の確かめ・§8 の件 5 の行の書き足し。
- **Out**: 並べ直しの並べ方そのもの（`reset_pos.rs` の既定の位置の式）・メニューの「シェル位置初期化」の道（起動の後に利用者が選ぶので表示の後）・バルーン位置初期化（`\![execute,resetballoonpos]`）の預かり。

## Boundary Candidates

- 位置の指令の預かり（`emo2_boot/frame/move_hold.rs` の `release_ready`・`touched_scopes`・`unshown_scope`）
- 位置の指令を流す相（`emo2_boot/frame/drain_resnap.rs` の `run_move_drain_with`）

## Out of Boundary

- `\![move]` の預かりの決まり（`window-work-area-clamp` で決まった・変えない）
- kanade の起動のイベントの時機

## Upstream / Downstream

- **Upstream**: 完了 spec `window-work-area-clamp`（`.kiro/specs/completed/window-work-area-clamp/`・要件 6.2〜6.4・設計の MoveHold・`research.md` 12 節）。完了の `areka-P0-window-link-directives`（並べ直しの指令）・`areka-P0-move-canon`。
- **Downstream**: `extra-character-windows`（3 人目以降の窓も並べ直す）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の範囲外の項目の引き渡し）。
- **Adjacent**: `audit-emo2-boot-frame`（`emo2_boot/frame/` の見直し）・`runtime-module-untangle`／`runtime-crate-split`（`emo2_boot` を `runtime` に改名して `areka-runtime` へ移す＝同じウェーブに置かない・パスは roadmap の「本体の crate 分け」の対応表で読み替える）・`placement-crate-split`（`placement/reset_pos.rs` を移す）・`extra-character-windows`。

## Constraints

- 1 フレーム遅らせる形にしない（指令は入力がそろった巡の、その相で当たる・`window-work-area-clamp` 要件 7.3）。
- ゴーストの名前・初回起動・特定のイベントを見る分岐を入れない（同 要件 6.4・7.8）。
- 失敗の道は必ず記録（`error!`＋`Err`）。テストは兄弟のファイル `*_tests.rs`。
- 実機の根・検体はワークツリーの `target\` の下だけ。人の手を介さない検体の起動は `AREKA_NO_ALERT=1`、ほかのぱすたがデバッグの口を持つときは `PASTA_DEBUG=0`（その起動の中だけ）。

- **優先度の区分**: B（バグ。起動の直後の並べ直しで窓の間に隙間が残りうる）。
- **版**: v0.2.0（致命的でない新しいバグ＝次の版。v0.1.0 の検体のぱすたの起動の台本は並べ直しを流さない＝4.4 の通しで当たっていない）。
- **要件定義のモデル**: Opus。今ある預かりの判定を並べ直しにも当てる形で、新しい境界・並行の設計には当たらない。
- **規模**: S（3〜6 タスクの見込み・見積もりは要件の段）。
- **分割の案**: なし。
- **要件の段の議題**（答えで作業が変わるもの）:
  1. 並べ直しを預かるとき、前に預かった移動を捨てる今の決まりと、表示されない窓が 1 つで列が詰まらない決まりをどう合わせるか。
