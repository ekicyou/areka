# Brief: extent-element-offset

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-surface-element-nesting` の完了時の棚卸（`.kiro/specs/completed/areka-P0-surface-element-nesting/tasks.md` の Implementation Notes 4.3・最終確かめの同じ指摘）。

## Problem

- **シェルの作者・利用者**: element定義で、ベースの画像の外へはみ出す位置（X,Y）に画像を置くと、はみ出した部分が描かれない。合成の命令は element定義の X,Y を足した位置へ画像を置くのに、窓の大きさ（外形）がその X,Y を数えていないため、合成の転写が外形の外を黙って切る。
- **入れ子**: `surface-element-nesting` で置いた子の中に、X,Y のずれた画像の element定義があると、その画像も外形から漏れて欠けうる（子を置いた位置の X,Y は外形に足しているが、子の中の画像の element の X,Y は足していない）。
- 記録は何も出ない（切られたことに気付けない）。

## Current State

- 外形の計算は `crates/areka-emo-compose/src/plan.rs` の `flatten_extent`。画像の element定義の寄与を「累積の位置＋原寸（`AtlasEntry::original`）」で数え、element定義自身の X,Y を足さない。この式は emo-compose を作ったとき（PR#40・`578ab0fa`）から変わっていない。
- 合成の命令（同じ `plan.rs` の `push_static_element_ops`）は element定義の X,Y を足して置く。外形と命令がずれている。
- 完了 spec `areka-P0-emo-compose` の要件 6.5 は外形を「配置オフセット＋原寸」の和集合と書き、design のシーケンス図の注記も「キャンバス外形＝全命令の (offset+original) の和集合」と書く。どちらも element定義の X,Y を含む読みが自然で、実装のほうがずれている。`surface-element-nesting` の research.md 2.4 節の「全 element の位置＋原寸」もコードと合っていない。
- 既存の外形のテストは element定義の位置がすべて 0,0 で、このずれは檻に入っていない。

## Desired Outcome

- ukadoc（と、必要なら SSP の見え方の文書）が外形をどう定めているかを確かめた上で、外形の規則を 1 つに決める（候補: 全命令の「位置＋原寸」の和集合／ベースの画像の大きさで切る）。
- 決めた規則で、命令と外形が食い違わない（描くと決めた画像が黙って切られない）。切る規則を選ぶなら、切ったことを記録に残す。
- 窓の大きさが変わるシェルがあるかを、リポジトリの検体（emo2 ほか `vendors/sample_ghost/`）で測って記録する。

## Approach

要件の段で次から選ぶ（議題 1・開発者の裁定が要る）。
- **案 A（和集合に揃える）**: `flatten_extent` の画像の枝に element定義の X,Y を足す。emo-compose の要件 6.5 の読みに合う。ベースの外へはみ出す画像を持つシェルでは窓が広がる（`surface-element-nesting` の要件 7.1「入れ子の無いシェルの外形は前と同じ」は本 spec が明示的に上書きする形になる）。
- **案 B（ベースで切る規則を明文化）**: 今の外形を正しいとし、はみ出す画像を切ることを規則として書き、切ったときに記録を出す。窓の大きさは変わらない。

## Scope

- **In**: `flatten_extent` の画像の element定義の寄与の規則・その決定論テスト（X,Y が 0 でない element定義・入れ子の子の中の element定義）・検体での窓の大きさの測り直し・emo-compose の要件 6.5 の読みの整理（上書きなら COMPAT 文書か完了 spec への注記）。
- **Out**: 描画メソッド・element定義のオプション（`--clipping` は `element-clipping-option`）・動く絵の再生・窓の位置の決め方。

## Boundary Candidates

- 外形の規則（`plan.rs` の `flatten_extent`）と、それを使う表示の窓の大きさ（`areka-emo-present`・`crates/areka/src/placement/`）の境目。規則を変えても、窓の側は外形を受け取るだけで変えない見込み。

## Out of Boundary

- 合成の命令の位置（今のまま正しい）。
- 当たり判定の領域（外形と独立）。

## Upstream / Downstream

- **Upstream**: 完了 spec `areka-P0-emo-compose`（外形の要件 6.5）・`areka-P0-surface-element-nesting`（子の外形の再帰）。
- **Downstream**: `animated-image-playback`（動く絵を部品のサーフェスへ分解して置く＝X,Y のずれた element が増える）・`element-clipping-option`。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の要件の読みを整理する）。
- **Adjacent**: `element-clipping-option`（同じ `plan.rs` の element定義の扱い）・`placement-measure-bake-once`（採寸が外形を読む）。

## Constraints

- 決定論テストを必ず添える（X,Y が 0 でない element定義で、命令の位置と外形が食い違わないこと）。
- 案 A を選ぶなら、窓の大きさが変わる検体を事前に列挙し、既存の golden の期待値を書き換える理由を記録する（黙って書き換えない）。

## 2026-10-05 `animated-image-playback` からの申し送り（同 spec の設計・要件 10.6）

> `animated-image-playback` が先に main へ入る（本 spec は未着手）。2026-10-06 に同 spec が実装した実物と照らして書き直した。着手時に引き直すこと。

- **置き場**: `compute_extent`・`flatten_extent` は `crates/areka-emo-compose/src/plan.rs` から `crates/areka-emo-compose/src/plan_extent.rs` へ移した（移すコミット `d7755372` では中身を変えていない）。`plan_extent.rs` は `plan` の子のモジュール（`plan.rs` が `#[path = "plan_extent.rs"] mod extent;`・`pub(crate) use extent::compute_extent;` で宣言）で、`flatten_extent` はその中の非公開の関数。既存の外形のテスト `plan_extent_tests.rs`・`plan_nesting_extent_tests.rs` も `plan_extent.rs` の子へ移り、動く絵の子の外形のテスト `plan_extent_film_tests.rs` が足された。roadmap と本文の「`plan.rs` の `flatten_extent`」は `plan_extent.rs` と読み替える。命令の側 `push_static_element_ops` は `plan.rs` に残っている。
- **`animated-image-playback` が `flatten_extent` に足したのは 2 か所**: `always` の animation の全部の pattern を数える輪（着せ替えの pattern0 と同じ輪で、`is_bind_interval` か `is_always_interval` の animation を辿る。負の番号・animation の番号を指す 7 語・`move` は数えない）と、動く絵の子（`ElementKind::Film`）の原寸（`EmoWorld::film_sheet` の `FilmSheet::original`）。
- **画像の element と動く絵の子は同じ 1 行で数えている**: 原寸を `match element.kind` で決め（`ElementKind::Film` は子の原寸、ほかは束縛した絵の原寸）、その後の `max_x`・`max_y` を更新する 1 行で数える。画像の element の X,Y（`element.transform.offset()`）を外形に足す直しは、その 1 行に入れれば子にも効く。命令の側では動く絵の子も X,Y を足して置いている（`push_static_element_ops` の `ElementKind::Film` の腕が `push_film_op` に X,Y を渡す）ので、食い違いは画像の element と同じ形で子にもある。
- **着せ替えの pattern0 の取り方の既存の食い違いは直していない**: 外形（`flatten_extent`）は「index が最小」の pattern を取り、命令（`plan.rs` の `flatten_surface`）と見える部品（`NestTable` の 1 行 `SurfaceParts` の `bind_targets`）は「index が 0」の pattern を取る。
- 手書きの `always` の外形は全部の pattern の和集合にした（開発者 2026-10-05・理由: 自動で作る子と手書きは同じに振る舞う／ずっと見える絵が外形で切れてはいけない）。ほかのアニメーション（`random` ほか）のコマは今までどおり数えていない。はみ出す絵の一般の決まりは本 spec の裁定のまま。


## 2026-11-28 棚卸㊺の再測定（main 7a590832）
- 前提の変化: `draw-methods-canon` が着地し、描く計画の `DrawPlan.extent` は外形（`crates/areka-emo-compose/src/plan_extent.rs`・237 行）の答えをそのまま使う。外形が画像の element定義の X,Y を足さない食い違いは今もそのまま（143 行目の子の枝だけが足す）。CPU の転写は無くなったが、外形の外は D2D の描き先の大きさで切られるので症状は同じ。
- 触るファイル: `plan_extent.rs` と兄弟のテスト `plan_extent_tests.rs`（450）・`plan_nesting_extent_tests.rs`（146）・`plan_extent_film_tests.rs`（144）、期待値が動けば `golden_tests_frame_extent_tests.rs`（221）・emo-present の `shell_target_emo2_tests.rs` と配置の採寸のテスト（`crates/areka-placement/src/measure_*_tests.rs`）。直したら emo-present の `screen_offscreen_gpu_tests.rs` の敷き物（41〜43 行目）を外す。§8。案 B（切って記録）なら記録のテストは `log_firing_tests.rs` に足さず、`plan_extent.rs` の兄弟の新しいテストのファイルに置く（`draw-methods-blend` と重ねない約束）。`plan.rs` は触らない見込み。
- 規模: 3〜6 タスク。
- 先に要るもの: 無し。後ろで `animated-image-import`（同じ `plan_extent.rs`）・`element-clipping-option`・`seriko-interval-combinations` が待つ。
- 優先度の区分: B（描くと決めた絵が記録なしに切られる）。
- 要件定義のモデル: Opus（案 A か案 B かは開発者の裁定で、前例で決まらない骨格の分かれ目ではない）。
- 分割の案: 無し。
- 議題の材料（ukadoc の読み）: element定義の項は外形（窓の大きさ）を定めていない。「element0 があると surface*.png の内容は捨てられ element0 で置き換わる」「合成結果を一枚の画像として扱う」だけ。`base` は「ベースサーフェスを新規レイヤで完全に置き換える」＝土台の大きさが外形と読む余地があり、案 B の根拠になりうる。
- 見つけた穴: 着せ替えの最初の pattern の取り方が、外形（`plan_extent.rs` 192 行目・番号が最小）と命令（番号が 0）で違う。同じ関数なので一緒に揃えるか議題にする。
- 古くなった記述: Current State・Scope・Boundary Candidates の「`plan.rs` の `flatten_extent`」は `plan_extent.rs`、「`crates/areka/src/placement/`」は `crates/areka-placement/`。

## 申し送り（draw-methods-canon より）

- まだ直っていない。外形（`crates/areka-emo-compose/src/plan_extent.rs`）は element の X／Y を足さない。描く計画の `DrawPlan.extent` はこの答えをそのまま使う。
- `crates/areka-emo-present/src/screen_offscreen_gpu_tests.rs` の `build_assets` は、この穴を避けるため、最初に全透明の原寸の絵を敷いている。この spec で直したら、その敷き物を外して確かめ直す。
- 参照: `.kiro/specs/completed/draw-methods-canon/design.md` の「DrawPlan」の規則（外形は `plan_extent.rs` の答えのまま）。
