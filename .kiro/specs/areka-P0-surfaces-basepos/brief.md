# Brief: areka-P0-surfaces-basepos

> **種別**: 追跡 spec（正典先送りの 4 点セット＝完全語彙＋縮退シーム＋追跡 spec＋roadmap 明記）。②parsers（surfaces 転記層）＋⓪ghost（move 解決）帰属の小型ユニット。
> **源**: `areka-P0-sakura-dialogue-tags` 要件ディスカッション議題4（2026-07-18 A-1 決裁・R5.2 が正本）。
> **着手ゲート**: M1 外。`point.basepos` を宣言する実シェル／fixture の適合が必要になった時（M2 シェル互換拡充）に解禁。

> **📌 2026-08-13 追記(63)（`areka-P0-scope-chain-gap` からの申し送り・前提の更新）**: **`\![move]` の座標算出式が変わった。** 本 brief が差し替え対象とする `BaseposResolver` 型シームは**無傷**（宣言 basepos の実導出は従来どおり trait の別実装を差すだけで済む）が、式と署名が変わっているため着手時に再突合すること。
> - **変更点**: `resolve_move_target_position`（`crates/areka/src/emo2_boot/move_cue.rs`）が `k: ScaleRatio` 引数を取り、**台本由来の `dx`／`dy` を `placement::scale_signed` で k 倍**するようになった。`apply_move_directive` も同じく `k` を取る。k の真実源は表示層の `applied_ratio`（実際に絵へ掛かった k）。
> - **是正後の式**: `x' = base_pos.x + basepos(base窓).x + k·dx − basepos(対象窓).x`（Y も同型）。fixture 検算は `x' = pos0.x + w0/2 + k·(−353) − w1/2`。k=1 なら従来式と一致する。
> - **`AxisSpec::Px` の契約が是正された**: doc は「物理 px」と書いていたが実際には台本の作者基準値が素通しで入っていた。**「作者基準 px・k 倍は解決側の責務」**へ改めた（転記層 `parse_move_directive` はスケールしない＝parser は転記層の正典を維持）。
> - **理由（実測）**: 拡大率 200% で emo2 の二体が **365px 重なる**欠陥。過剰分 353px はスケールし損ねた `dx` そのものだった。是正後は 12px（＝100% の 6px のちょうど 2 倍）。
> - **⚠️ 互換上の裁定が未確定**: 参照実装 SSP は `\![move]` オフセットを**無スケール**で適用する（`.kiro/specs/completed/areka-P0-scope-chain-gap/ssp-oracle-notes.md` の SSP 自己不整合 #2）。ゆえに本是正は**意図的な SSP 非互換**である。同種の値 `windowposition.x/y` が実測で `wp.x × k` と確定していること（`placement/windowposition.rs` の SSP 実測表）との内部整合を優先した判断だが、**互換対応表（COMPAT §8）への記録が要る**。scg 側で記録するが、本 spec が `\![move]` 意味論を扱う際は**この裁定を前提として引き継ぐこと**（旧挙動へ戻す設計をしないこと）。

## Problem

`\![move]` の基準位置語彙 `base` は正典で「**base は surfaces.txt 内の point.basepos 指定に従う**」（一次 SSP HTML `list_sakura_script.html` `\![move]` 項）。しかし宣言 `point.basepos.x/y` の実導出（surfaces.txt からの転記→move 解決での宣言値優先）は areka に存在しない（`grep basepos` → 全 codebase 0 件・emo2 fixture の surfaces.txt を含む）。

M1（sakura-dialogue-tags）は**正典既定のみを実導出**する: `point.basepos.x` 既定＝サーフェス幅÷2／`point.basepos.y` 既定＝下端（`ukadoc:descript_shell_surfaces:point.basepos.x/y`）。emo2 は `point.basepos` を宣言しないため**正典既定がそのまま適用される正規経路**であり、fixture は canon 通りに動く（Y=fix ゆえ実効は basepos.x のみ）。宣言形の実導出だけが先送りされる。

## Current State

- emo2 `shell/master/surfaces.txt` は `point.basepos` を宣言しない（2026-07-17 実測）。
- `completed/areka-P0-sakura-dialogue-tags` R5.2 が既定 basepos 算出＋**差し替え可能な型シーム**を要件化・着地済み（2026-07-18）: `BaseposResolver` trait（`fn basepos(&self, window_size: SizeI) -> PointPx`）＋既定実装 `CanonDefaultBasepos`（x=幅÷2・y=下端）＝`crates/areka/src/emo2_boot/move_cue.rs`。座標算出は `resolve_move_target_position`（同ファイル）が `BaseposResolver` を注入で受け取る形＝本 spec（宣言 `point.basepos` の実導出）はこの trait の別実装を差し替えるだけで済む型シームが確保済み。
- 裸 `base`（ドット無し形・正典形式は `X基準.Y基準`）は `base.base` と等価に解する areka 裁量を対応表へ記録済み（同 R5.2）。

## Desired Outcome

surfaces.txt の `point.basepos.x/y` 宣言が parse で転記され、move 解決が**宣言値を正典既定に優先**して用いる。宣言ありシェルで `\![move,...,base,base]` が宣言基準点どおりに動く（決定論檻＋宣言ありシェルでの実機確認）。

## Approach

1. ②parsers（surfaces 転記層）へ `point.basepos.x/y` の転記を追加（転記層の規律＝解釈せず記述保持・[[areka-parser-transcribes-tree-downstream]]）。
2. ⓪ghost の move 解決型シーム（M1 予約済み）へ宣言値を差す（宣言なし＝既定 fallback 維持）。
3. 檻: 宣言あり／なし両経路の決定論 unit＋宣言ありシェルの実機サインオフ。

## Scope

- **In**: `point.basepos.x/y` の parse 転記／move 解決の宣言値優先＋既定 fallback／決定論檻。
- **Out**: basepos を消費する他機能（バルーン初期配置等）／SERIKO 側の basepos 消費／`\![move]` の他の意味論（dialogue-tags で確定済み）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-sakura-dialogue-tags`（既定 basepos＋型シームの正本・R5.2）／completed shell-parse 系（surfaces 転記層）。
- **Downstream**: emo2 以外の実シェル適合（`emo2-conformance-e2e` の後継たる実ゴースト互換検証）。

## Constraints

- 正典は ukadoc（emo2 は最小適合 fixture）。転記層は解釈しない。決定論檻必達。
- 宣言なしシェルの挙動（既定経路）は本 spec で**一切変えない**（非退行）。
