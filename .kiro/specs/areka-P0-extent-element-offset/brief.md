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
