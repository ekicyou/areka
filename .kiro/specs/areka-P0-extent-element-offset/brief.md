# Brief: areka-P0-extent-element-offset

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

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（3〜6）のまま。案 A なら実装は `flatten_extent` の画像の枝の 1 か所（子の枝と同じく `element.transform.offset()` を足す）＋檻＋検体の測り直しで 3〜4、案 B なら記録と規則の文書で 3〜5。切らない。
- 前提の状態: 上流（`surface-element-nesting`）は着地済み・今すぐ着手できる。起票（`fa12dba7`）の後に `crates/areka-emo-compose/`・`areka-parsers/src/shell/`・`areka-emo-atlas/` を触ったコミットは 0。
- 崩れた前提／古くなった位置: 無し。`plan.rs` の `flatten_extent` の画像の枝は今も「累積の位置＋原寸」で element定義の X,Y を足さない。同じ関数の子の枝（`ElementKind::Surface`）は X,Y を足しており、命令の側 `push_static_element_ops` も足す＝食い違いは本文どおり。
- ukadoc の引き直し（議題 1 の材料）: element定義の項は外形（窓の大きさ）を定めていない。「element0 があると surface*.png の内容は捨てられ element0 で置き換わる」「合成結果を一枚の画像として扱う」だけ。描画メソッド `base` の項は「ベースサーフェスを新規レイヤで完全に置き換える」「element では element0 にしか使えず、それ以外は overlay に読み替える」。`overlay` は「ベースレイヤに新規レイヤを単に重ねる」。＝「土台（element0）の大きさが外形」と読む余地があり、案 B の根拠になりうる。どちらに倒すかは開発者の裁定（SSP の見え方を測る手は記憶「SSP 実測主義は取らない」により使わない）。
- 触るファイル: `crates/areka-emo-compose/src/plan.rs`（`flatten_extent`）・`plan_extent_tests.rs`・`plan_nesting_extent_tests.rs`・`golden_tests_frame_extent_tests.rs`（期待値が変わるなら）・案 B なら記録の文言の檻（`log_firing_tests.rs`）・`doc/COMPAT_ARCHITECTURE.md` §8・必要なら完了 `emo-compose` の要件 6.5 への注記。
- 議題:
  1. 案 A（和集合）か案 B（土台で切る・記録を出す）か（本文どおり・上の ukadoc の読みを添えて出す）。
  2. `element-base-method` との順: 両方が `plan.rs` を触り、`base` の意味（土台を置き換える）が外形の規則と直に絡む。**`element-base-method` を先に着地させ、その上で本 spec の裁定をする**のが自然（同時に走らせない）。
- 同時に走らせない: `element-base-method`・`animated-image-playback`（C4・`plan.rs`）・`element-clipping-option`。


## 2026-10-05 `animated-image-playback` からの申し送り（同 spec の設計・要件 10.6）

> `animated-image-playback` が先に main へ入る（本 spec は未着手）。2026-10-06 に同 spec が実装した実物と照らして書き直した。着手時に引き直すこと。

- **置き場**: `compute_extent`・`flatten_extent` は `crates/areka-emo-compose/src/plan.rs` から `crates/areka-emo-compose/src/plan_extent.rs` へ移した（移すコミット `d7755372` では中身を変えていない）。`plan_extent.rs` は `plan` の子のモジュール（`plan.rs` が `#[path = "plan_extent.rs"] mod extent;`・`pub(crate) use extent::compute_extent;` で宣言）で、`flatten_extent` はその中の非公開の関数。既存の外形のテスト `plan_extent_tests.rs`・`plan_nesting_extent_tests.rs` も `plan_extent.rs` の子へ移り、動く絵の子の外形のテスト `plan_extent_film_tests.rs` が足された。roadmap と本文の「`plan.rs` の `flatten_extent`」は `plan_extent.rs` と読み替える。命令の側 `push_static_element_ops` は `plan.rs` に残っている。
- **`animated-image-playback` が `flatten_extent` に足したのは 2 か所**: `always` の animation の全部の pattern を数える輪（着せ替えの pattern0 と同じ輪で、`is_bind_interval` か `is_always_interval` の animation を辿る。負の番号・animation の番号を指す 7 語・`move` は数えない）と、動く絵の子（`ElementKind::Film`）の原寸（`EmoWorld::film_sheet` の `FilmSheet::original`）。
- **画像の element と動く絵の子は同じ 1 行で数えている**: 原寸を `match element.kind` で決め（`ElementKind::Film` は子の原寸、ほかは束縛した絵の原寸）、その後の `max_x`・`max_y` を更新する 1 行で数える。画像の element の X,Y（`element.transform.offset()`）を外形に足す直しは、その 1 行に入れれば子にも効く。命令の側では動く絵の子も X,Y を足して置いている（`push_static_element_ops` の `ElementKind::Film` の腕が `push_film_op` に X,Y を渡す）ので、食い違いは画像の element と同じ形で子にもある。
- **着せ替えの pattern0 の取り方の既存の食い違いは直していない**: 外形（`flatten_extent`）は「index が最小」の pattern を取り、命令（`plan.rs` の `flatten_surface`）と見える部品（`NestTable` の 1 行 `SurfaceParts` の `bind_targets`）は「index が 0」の pattern を取る。
- 手書きの `always` の外形は全部の pattern の和集合にした（開発者 2026-10-05・理由: 自動で作る子と手書きは同じに振る舞う／ずっと見える絵が外形で切れてはいけない）。ほかのアニメーション（`random` ほか）のコマは今までどおり数えていない。はみ出す絵の一般の決まりは本 spec の裁定のまま。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 先に要った `element-base-method`（10-05）と `animated-image-playback`（10-07）がどちらも着地し、待つものが無くなった。外形の計算は `crates/areka-emo-compose/src/plan_extent.rs`（`plan.rs` の子）に在り、動く絵の子と `always` の全部の pattern を数える（上の申し送りは今の main と一致）。画像の element定義の X,Y を足さない食い違いは今もそのまま。
- 触るファイル: `crates/areka-emo-compose/src/plan_extent.rs`（外形を数える 1 行）と兄弟のテスト `plan_extent_tests.rs`・`plan_nesting_extent_tests.rs`・`plan_extent_film_tests.rs`、期待値が動くなら `golden_tests_frame_extent_tests.rs`。案 B（切って記録）なら記録を出す場所と `log_firing_tests.rs`。`doc/COMPAT_ARCHITECTURE.md` §8。`plan.rs` は触らない見込み。
- 規模: 3〜6 タスクのまま。切らない。
- 先に要るもの: 無し（今すぐ着手できる）。同じ `plan_extent.rs` を触る見込みの `animated-image-import`・`element-clipping-option` とは同時に走らせない。`seriko-trigger-intervals` は `always` を含む組み合わせを持つ間だけ重なる（同 brief の分割の案を採れば重ならない）。`collisionex-regions`・`placement-measure-bake-once`・`present-emit-tail-latency` とは重なり 0。
- 優先度の区分: B（バグ＝描くと決めた絵が記録なしに切られる）。
- 要件定義のモデル: Fable（正典が外形を定めていない・案 A か案 B かは開発者の裁定）。
- 分割の案: 無し。
- 見つけた穴・古くなった記述:
  - 着せ替えの最初の pattern の取り方が、外形（番号が最小のもの）と命令（番号が 0 のもの）で違う件は同じ関数の中なので、本 spec で一緒に揃えるのが安い（議題に足す）。
  - `crates/areka-emo-compose/src/log_firing_tests.rs` の `pattern0_less_bind_skip_fires_debug_and_no_warn_error` の説明に、移す前の `plan.rs` の行番号（`:529`）が残っている。
  - Current State・Scope の「`plan.rs` の `flatten_extent`」は `plan_extent.rs` と読む。
  - 外形が変わると採寸の結果も変わる。`placement-measure-bake-once` より先に着地させると、あちらの「採寸の結果が同じ」の固定が書きやすい。
