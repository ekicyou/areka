# Brief: areka-P0-surfaces-basepos

> **種別**: 追跡 spec（正典先送りの 4 点セット＝完全語彙＋縮退シーム＋追跡 spec＋roadmap 明記）。②parsers（surfaces 転記層）＋⓪ghost（move 解決）帰属の小型ユニット。
> **源**: `areka-P0-sakura-dialogue-tags` 要件ディスカッション議題4（2026-07-18 A-1 決裁・R5.2 が正本）。
> **着手ゲート**: M1 外。`point.basepos` を宣言する実シェル／fixture の適合が必要になった時（M2 シェル互換拡充）に解禁。

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
