# Brief: areka-P0-draw-methods-canon

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-element-base-method` の完了時の棚卸。同 spec の要件の討議（2026-10-05）で開発者が「残りの描画メソッドは起票」を選び、あわせて方針「合成の方法は Direct2D が支える全部に対応できるよう、wintf も含めて広げる」を示した（`.kiro/specs/completed/areka-P0-element-base-method/requirements.md` の範囲外の節）。

## Problem

- **ゴーストの作者・利用者**: `overlay`・`base` 以外の描画メソッドで書いた element定義・pattern定義が描かれない。`overlayfast`／`overlay-fast`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*`（加算・乗算・スクリーン・覆い焼きなど）・pattern定義の `base` を使うシェルでは、その部品や着せ替えが消えるか、土台に別の絵が残る。
- element定義では areka が描かない行を読み込み 1 回につき `warn!` で知らせる（`element-base-method`）。pattern定義では合成の段が描かずに飛ばす。どちらも見た目は正典と違う。
- 1 つずつ場当たりに足すと、合成の段の形（今は「SourceOver だけを整数で転写する」）がそのたびに歪む。

## Current State

- 読み手 `crates/areka-parsers/src/shell/decode.rs` の `is_image_element_method` が `overlay` と `base` だけを画像の element定義の値にする。ほかの語の行は `crates/areka-parsers/src/shell/undrawn.rs` の `parse_undrawn_elements` が拾い、`crates/areka-emo-present/src/shell_target.rs` の `load_shell_target` が 1 行 1 件 `warn!` する。`Element` には描画メソッドの欄が無い。
- 合成の段 `crates/areka-emo-compose`:
  - `method.rs` の `ComposeMethod` は全部の語を型として持つ（`Overlay`・`OverlayFast`・`Interpolate`・`Replace`・`Asis`・`Base`・`Reduce`・`Auto`・`Blend(BlendMode)`・`Unknown`）。`is_implemented` は `Overlay` だけ真。
  - `blit.rs` は premultiplied SourceOver を CPU の整数演算で転写する（浮動小数を持ち込まない・決定論）。`is_implemented` が偽の命令は飛ばす。
  - `fold.rs` の `normalize_element` は element定義を描画メソッドに関係なく `Overlay` で置く。
- pattern定義の語は `areka-seriko` から `ComposeMethod::from_name` を通る。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml`: element定義の行は `degraded`、`ukadoc:descript_shell_surfaces:base:1` は `degraded`（element定義では描ける・pattern定義では未対応）。`blend-*` などの語の行もある。
- 描画は emo の自前の合成（CPU）で、Direct2D は表示の段（wintf）にある。

## Desired Outcome

- ukadoc の `descript_shell_surfaces` が element定義・pattern定義で使えると書く描画メソッドを、areka が描く。範囲は「Direct2D の合成・ブレンドが支える全部」へ広げる向きで、要件の段で ukadoc と D2D の対応表を作って決める。
- element定義では、読み手が描画メソッドの欄を `Element` に運び、合成の段がその語で描く。`is_image_element_method` を広げた分だけ `parse_undrawn_elements` の警告の対象が自動で減る（「3 つの転記が element定義の行を漏れなく重なりなく分ける」テストを確かめ直す）。
- pattern定義の `base`（下の絵を使わず、指した絵から合成を始める）を描く。
- 描けない語が残るなら、記録を出す今の作りを保つ。
- 決定論のテストで語ごとの画素を固定する。合成の答えを CPU と D2D のどちらで出すかで、決定論の取り方が変わる（下の Approach）。

## Approach

要件・設計の段で次の分かれ目を決める（ここでは決めない）。

- **合成をどこで行うか**: 今の CPU の整数の合成を語ごとに広げるか、合成を Direct2D（wintf の D2D の口・ブレンドの効果）へ移すか、あるいは両方を持つか。開発者の方針は「D2D が支える全部に対応できるよう wintf も含めて広げる」で、wintf を触ることは範囲外ではない。
- **決定論**: 今の檻は CPU の整数の転写で画素を固定している。D2D へ移すなら、画素の固定の仕方（読み戻し・許容差・GPU とソフトウェアの描き手の違い）を決める。
- **語の対応表**: ukadoc の語（`blend-*` の新書式・旧書式の別名・`-fast` 付き）と D2D のブレンドの種類の対応。D2D に無い語の扱い。

## Scope

- **In**: element定義・pattern定義の残りの描画メソッド全部（pattern定義の `base` を含む）・読み手の `Element` の描画メソッドの欄・合成の段・必要なら wintf の D2D の口・台帳の該当の行・`doc/COMPAT_ARCHITECTURE.md`・決定論のテスト・実機の確認。
- **Out**: element定義のオプション（クリップなど・`element-clipping-option`）・外形の求め方（`extent-element-offset`）・動く画像の再生（`animated-image-playback`）・α の宣言（`self-alpha-declaration`）。

## Boundary Candidates

- 読み手（`shell::{model,decode}`・`undrawn.rs` の連動）
- 合成の段（`areka-emo-compose` の `method.rs`・`plan.rs`・`blit.rs`・`fold.rs`）
- D2D の合成・ブレンドの口（wintf）——合成を移す案を取るときだけ
- pattern定義の経路（`areka-seriko` から合成の段への受け渡し）

## Out of Boundary

- 描画メソッドの語の綴りの正規化（`canonical_method_name`）と登録簿（`known_method`）の写しは、広げる語の分だけ触る。登録簿の作り直しはしない。
- 表示の段（wintf の窓・スワップチェーン）の作り。

## Upstream / Downstream

- **Upstream**: 完了 `element-base-method`（語の集合の 1 関数・描けない行の警告）・`emo-compose`・`seriko-runtime`・`surface-element-nesting`。
- **Downstream**: 描画メソッドを使うシェル全般。合成を D2D へ移すなら、合成の段の決定論のテスト（golden）全部。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `element-base-method` の継ぎ目 2 つ＝`is_image_element_method` と `parse_undrawn_elements` を使う）。
- **Adjacent**: `element-clipping-option`・`extent-element-offset`・`collisionex-regions`（同じ `shell/model.rs`・`shell/decode.rs`）・`animated-image-playback`（同じ `plan.rs`）——同時に走らせない。

## Constraints

- 段は**優先**（シェルの element の列・アニメーション）。
- 規模は大きい見込み（語の数と、合成を D2D へ移すかどうかで変わる）。要件の段で測り、20 タスクを大きく超えるなら分け方を議題にする。
- 1 フレーム遅らせる解・時刻を丸める解は取らない（設計の大原則）。
