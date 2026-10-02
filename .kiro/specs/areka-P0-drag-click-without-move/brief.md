# Brief: areka-P0-drag-click-without-move

> 2026-10-02 `/kiro-discovery` で起票（`alpha-release-signoff` の完了の手順の中・開発者指示「あ、起票はあとでやってくれますよね。「実装完了を承認」スキルは最後に実施しますし。その前提で、今は実装に戻ってください。」）。roadmap「alpha-release-signoff の持ち越し」節。出どころは `alpha-release-signoff` の完成判定 `verification/alpha-completion.md` §6 と受入記録 `verification/acceptance-record.md` §8.5。**種別はバグ**（設計の意図「掴んで離したときだけ保存する」に反する）。本文のソースの指し先は起票時（`c430480d`）の実測＝着手時に引き直すこと。

## Problem

- **利用者**: キャラクターの絵の上を、動かさずに左クリックしただけで、その窓の位置が記憶に書かれる。次の起動からは「既定の配置」でなく「保存した位置」として読み戻される。
- 実害の筋: 初回の起動で台詞が相方をずらした（初回だけの位置合わせ）後に、利用者が相方を 1 回クリックすると、**ずらした位置が保存される**。「初回だけの位置合わせは保存しない」（`alpha-release-signoff` の項目 12・要件討議の議題 6 で 2026-10-01 に開発者が確定）が、クリック 1 回で崩れる。

## Current State

- **実機の記録**（`alpha-release-signoff` の一周・生の記録 `C:\home\maz\lap-records\alpha-signoff-20261001\`）。保存の行はどれも窓を動かしていない（値は既定の配置と同じ）:
  - A1r `2026-10-02T13:08:51.371241Z` `char DragEnd 保存 scope=0`（項目 3 の透明な場所のクリックが本体の絵に当たった）
  - A2r `2026-10-02T13:20:46.217387Z` `char DragEnd 保存 scope=1`（項目 5 の `claudia` の相方）
  - A2r `2026-10-02T13:23:12.861575Z` `char DragEnd 保存 scope=0`（左クリックの後の右クリックの手順の左クリック）
  - 3 件とも直前に `DraggingState 不在のため生ドラッグ座標を復元できない（写像スキップ）`（debug）があり、走行の中のドラッグ開始の知らせ（`[DragStartEvent] Dispatching`）は A1r・A2r とも **0 件**。本当に掴んで動かした A3r の 1 件は、開始の知らせ 1 件・写像スキップ 0 件（2026-10-02 に生の記録を数えた）。
- **静的に分かったこと**:
  - wintf は左ボタンを離したとき、ドラッグの閾値（`DragConfig` の既定 5 px）に届かず `Preparing` のままでも、終了の遷移 `DragTransition::Ended` を記録する（`crates/wintf/src/ecs/window_proc/mouse_click.rs` の左ボタンを離したときの分岐・2 か所〔当たり判定が取れたとき／取れなかったときの予備〕）。そのため開始の知らせも `DraggingState` も無いまま `DragEndEvent` が配られる（`crates/wintf/src/ecs/drag/dispatch.rs` の `Ended` の腕）。
  - areka の `on_char_drag_end`（`crates/areka/src/placement/follow/drag_follow.rs`）は、`DraggingState` が無いと今の窓の位置へ縮退して**必ず保存する**。この縮退は「多窓のとき `DraggingState` が `DragEnd` より先に落ちても、ドラッグした窓の位置を落とさない」ために入れたもの（同関数のコメント）で、外すとその穴が戻る。
  - 同じ形の疑い（未観測）: バルーンの `on_balloon_drag_end` も `DragEndEvent` だけを見て offset を保存し、キーワードで決めた位置の素材を退役させる（`retire_keyword_base_on_save`）。動かさないクリックでバルーンの位置の記憶が書かれるかは着手時に確かめる。
  - 保存の書き手は `char_pos_entries` → `persist_entries`（`crates/areka/src/placement/persist.rs` の周り）。記録の target 名は `areka::persist::save`。

## Desired Outcome

- 窓の位置（キャラクター・バルーン）を記憶へ書くのは、**掴んで動かして離したときだけ**。動かさないクリック・ダブルクリック・右クリックの前の左クリックでは書かない（0 件）。
- 多窓のときに `DraggingState` が先に落ちても、本当にドラッグした窓の位置は今どおり保存される（縮退が守っていた穴を戻さない）。
- 初回だけの位置合わせでずらした相方をクリックしても、次の起動は既定の配置に立つ。
- 直す前に赤になる決定論のテストがある（動かさないクリックで保存 0 件・閾値を越えたドラッグで保存 1 件・多窓の縮退の場面で保存 1 件）。

## Approach

- 根本を 1 か所で直す。候補は 2 つで、要件の段で呼び手を全部洗ってから決める:
  - (a) wintf: 開始の知らせを出していない（`Preparing` のまま離した）ときは `DragEndEvent` を配らない＝「開始の無い終了」を作らない。`OnDragEnd` の受け手（起票時は `crates/areka/src/placement/spawn.rs` で結線するキャラクターとバルーンの 2 つ）が同じ口を通るので、直す場所は 1 つで済む。メニューの引き金（`crates/areka/src/menu/trigger.rs`）は知らせでなくドラッグの状態を読むので、その状態の遷移（`Preparing` → `JustEnded`）は変えない。ドラッグの状態が `JustEnded` で休む決まり（完了 `wintf-drag-state-rest-contract`）とクリックの判定に響かないかを確かめる。
  - (b) areka: 保存の腕を「このドラッグで開始を見たか」で分ける。wintf に触らないが、バルーンの側にも同じ判断が要る。
- 1 フレーム遅らせる解は取らない。既存の `crates/areka/src/placement/follow_drag_end_persist_tests.rs` の組み立てで赤を先に立てる。

## Scope

- **In**: 動かさないクリックで位置を保存しない修正（キャラクター・バルーン）・決定論のテスト・実機での確かめ（相方を初回の位置合わせの後にクリック → 次の起動で既定の配置）・完了 spec の要件を上書きするなら `doc/COMPAT_ARCHITECTURE.md` §8 への記録。
- **Out**: 閾値の値（5 px）の変更・ドラッグ中の追従の作り・初回だけの位置合わせそのもの・位置の記憶の形式。

## Boundary Candidates

- ドラッグの終了の知らせを出す条件（wintf の `mouse_click.rs`・`dispatch.rs`・状態機械）
- 位置を記憶へ書く条件（areka の `drag_follow.rs` のキャラクターとバルーンの終了の受け手）

## Out of Boundary

- 右クリックメニューの出し方・ドラッグの状態の休み方（完了 `wintf-drag-state-rest-contract` の決め）は変えない。
- 拡大率の切替での位置の扱い（`dpi-transition-two-tick-bounce`）。

## Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。完了 `wintf-drag-state-rest-contract`・`popup-menu-minimal`・`position-persist` 系の上に建つ。
- **Downstream**: 初回だけの位置合わせの規則（項目 12）が実際の利用で守られる。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 完了 `wintf-drag-state-rest-contract`（同じ wintf のドラッグの状態・`crates/areka/src/menu/trigger.rs` がその状態を読む）・`popup-menu-residue`（`menu/` に触る残件）・`dpi-transition-two-tick-bounce`（位置の別の話）。

## Constraints

- バグの修正は根本・0 フレーム・決定論のテスト（直す前に赤）。ログの無い失敗の経路を作らない。
- 実機の確かめは判定の分岐の記録の level（`areka::persist::save` と `drag_follow` の debug）まで `RUST_LOG` を開ける。実機の根はワークツリーの `target\` の下だけ。
- 1 ファイル 1,000 行（`drag_follow.rs` の行数は着手時に確かめ、足すなら兄弟のファイルで）。
