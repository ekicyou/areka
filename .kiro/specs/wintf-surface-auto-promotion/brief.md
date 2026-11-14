# Brief: wintf-surface-auto-promotion

> 2026-11-06 `wintf-visual-tree` の完了の棚卸で起票（`/kiro-discovery`）。下の「推す形」と「議題」は、着手のときの要件の段で覆してよい。
>
> 出どころ: `wintf-visual-tree` の要件 Introduction の到達点（2026-11-06 開発者）＝「既定は描画面なし・更新が頻繁になった Visual から描画面を作る（更新の頻度で自動で昇格・降格する）」。同 spec の設計の Out of Boundary「節の下の描画面の持ち主を支えること（配置の伝播と WUC の木の同期が節をまたぐこと）＝自動の昇格の spec」。起票の訳＝規模が spec 1 本に相当する（kiro-complete 冒頭ステップ手順 3）。

## Problem

- `wintf-visual-tree` で、描画面の有無を Visual ごとに選べるようになった（規則は `wants_own_surface` の 1 か所・新しい描き方の部品を持つ Visual は描画面なしの「節」・目印 `OwnSurface` で手動で持ち主へ上げる）。ただし上げ下げは使い手の手作業で、更新の頻度を見て自動で決める仕組みは無い。
- 節の下に描画面の持ち主を置けない（要件 1.11 で `error!` を残して切る）。そのため、木の途中の Visual だけを持ち主へ上げることができず、自動の昇格の前提が欠けている。
- 持ち主へ上げた Visual の `VisualTransform` は WUC の側で無視して `warn!` を残すだけ（`wintf-visual-tree` 最終検証 I-1 の裁定）。上げた Visual の変形が消える。

## Current State（起票時・`wintf-visual-tree` の着地の時点）

- 規則: `crates/wintf/src/ecs/graphics/visual/mod.rs` の `wants_own_surface(has_command_list, has_own_surface, is_node)`。節は `WorldVisual` を持つ Visual（`VisualDrawContent`／`VisualTransform` が `#[require]` で連れてくる）。
- 出入りの係: `visual/systems.rs` の `sync_surface_owner_parts`（3 部品の出し入れ・降格で `DrawTree` も外す）と `update_draw_trees`（昇格・降格の回は `TreeEdits` で全部の根を組み直す）。
- 節の下の持ち主: `visual/world_visual.rs` の `fill_world_visuals` が切って `error!`（1 度だけ・続くあいだは繰り返さない）。
- 持ち主の自分の中身は `DrawTree` の先頭の 1 件として恒等の行列で描く。持ち主の `VisualTransform` は無視して変化の回に `warn!`。

## Desired Outcome

- 更新の頻度（見た目の版 `VisualVersion` の上がり方など）を見て、頻繁に変わる Visual を自動で持ち主へ上げ、落ち着いたら節へ戻す。上げ下げで画素の答えは変わらない（WARP で固定）。
- 節の下に描画面の持ち主を置ける（配置の伝播と WUC の木の同期が節をまたぐ）。持ち主の `VisualTransform` が WUC の側でも効く。
- 既定の「描画面なし」へ寄せる道筋（今の描き方の Visual の既定を変えるかどうかを含む）を要件の段で決める。

## Approach

**推す形**（要件の段で覆してよい）:

1. 判断の材料は、既に在る見た目の版と変化の時刻から取る（新しい計測の仕組みを足さない）。上げ下げの閾値と履歴（行ったり来たりを防ぐ）を 1 か所の純粋な関数にして、真理値表・時系列の決定論のテストで固定する。
2. 節の下の持ち主は、WUC の Visual を親の持ち主の描画面の Visual の子として繋ぎ、節の世界の行列を WUC の変形へ写す。
3. 画素の答えは `wintf-visual-tree` の WARP の足場（`draw/emit_test_support.rs`）で、上げた形・下げた形・全部を節にした形が一致することを確かめる。

## Scope

- **In**: 自動の昇格・降格の判断と係、節の下の持ち主（配置の伝播・WUC の木の同期）、持ち主の `VisualTransform` を WUC で効かせること、既定の寄せ方の決め。
- **Out**: 差分の描き直し（`wintf-visual-damage-redraw`）、合成の規則（`wintf-d2d-composite-rules`）、areka の側の使い方。

## Boundary Candidates

- 昇格・降格の判断（純粋な関数）／出入りの係（既存の `sync_surface_owner_parts` の拡張）
- 節をまたぐ WUC の木の同期と配置の伝播

## Out of Boundary

- 前の列の保持・LIS・描き直す範囲（`wintf-visual-damage-redraw`）
- 層・不透明度・重ね方の規則（`wintf-d2d-composite-rules`）

## Upstream / Downstream

- **Upstream**: ✅ `wintf-visual-tree`（土台）、`wintf-visual-damage-redraw`（要件どおりその後＝差分の描き直しで描き直しの量が測れるようになってから判断の材料を決める）
- **Downstream**: `draw-methods-canon`・`shell-multi-compose` など、emo の部品を節として載せる spec（頻繁に変わる部品だけが描画面を持つ形の恩恵を受ける）

## Existing Spec Touchpoints

- **Extends**: なし（`wintf-visual-tree` は完了済み）
- **Adjacent**: `wintf-visual-damage-redraw`（`visual/systems.rs`・`render.rs` を分け合う＝その後に直列）

## Constraints

- wintf は areka に依存しない・公開の名前に areka の語を使わない。
- 依存を増やさない（`Cargo.toml` を触らない）見込み。
- 時刻は正確に扱う（頻度の判断で時刻を丸めない）。
- **優先度の区分**: A（開発者の到達点・2026-11-06）。版は v0.2.0（`wintf-visual-damage-redraw` の後）。

## 申し送り（wintf-visual-damage-redraw より・2026-11-14）

- **昇格・降格の判断の材料に使える記録の行**: target `wintf::damage`・水準 debug・文言 `[render_surface] damage`（`crates/wintf/src/ecs/graphics/systems/render.rs` の `log_damage`）。描画面 1 枚の 1 回の合成ごとに 1 行。欄は 12 個:
  - `entity`（描画面の実体名）・`mode`（`full`・`partial`・`present`＝入れ物を描かず描画面へ写すだけ・`skip`）・`reason`（`full` のときだけ。`first`・`recreated`・`root_list`・`scale`・`no_prev`・`undecidable`・`covers_surface`＝`crates/wintf/src/ecs/graphics/damage.rs` の `FullReason::as_str`）
  - `items_redrawn`（描き直した件の数）・`items`（描く列の件数）・`area`（描き直した面積）・`surface_area`（描画面の面積）・`shift_area`（入れ物の中で写した面積）・`exposed_area`（写しの露出した帯の面積）・`present_area`（描画面へ写した面積）
  - `compose_us`・`present_us`（合成と描画面へ写す所要。この target の debug が開いているときだけ測り、閉じていれば 0）
  - 欄の名と意味の変更は `.kiro/specs/completed/wintf-visual-damage-redraw/design.md` の Revalidation Triggers と DamageRecord に載る。履歴の層（info 以上）には入らないので、判断の係が使うならこの行の外で同じ量を持つ形も考える（未決）。
- **持ち主へ上げた後の動きの量**（`crates/wintf/src/ecs/graphics/visual/draw/README.md` の「写し」の後半）: `OwnSurface` の子を `Arrangement` の位置で動かすだけなら親の描画面に触らず、写し終えた後の自分の描画面の計画は `skip`（描き直しも写しも 0）。命令の列だけを持つ持ち主は拡大の変わらない位置の変化で印も立たない。描画の木を持つ持ち主は木を組み直して項の列を比べる手間は掛かる。節の下に持ち主を置けないこと・持ち主の `VisualTransform` が WUC で効かないことは、この brief の Problem のとおり（変わっていない）。
- **範囲だけの描き直しの粒は件を丸ごと**: 描く矩形を交わる件の範囲で閉じる（`.kiro/specs/completed/wintf-visual-damage-redraw/design.md` の Risks の R1）ので、大きな件に触れる変化は `covers_surface` で全部になる。昇格の判断で「描き直した面積」を見るなら、この粒を前提にする。
