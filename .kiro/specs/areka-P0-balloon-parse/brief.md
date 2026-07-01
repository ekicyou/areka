# Brief: areka-P0-balloon-parse（本坑 / main・M1 M-boot / parser トラック）

> **種別**: 本坑（main）。M1 `areka-P0-emo2-boot` の **parser トラック**（並行・単体テスト可・host 不要）。**依存は無し＝即着手可・安全並走**（`shell-parse ∥ balloon-parse ∥ package-mount`）。
> **規律**: emo2 が実際に使う balloon フィールドのみ。過剰・予測実装は禁止。

## Problem

emo2 のバルーン定義（`descript.txt` ＋ サーフェス別上書き `balloons0s.txt`/`balloonk0s.txt`）を**バルーンモデル**へ解析する parser が無い。統一グラフィック方針（バルーン＝シェル surface 上の文字層）ゆえ、`text-layer`／`surface-engine` がバルーン枠・文字領域・座標を消費するモデルの生成源が要る。

## Current State（調査済み・接ぎ木先）

- **`areka-parsers` クレート**（`crates/areka-parsers/`）: `sakura` の確立パターン（`Result` 無しの寛容パース・NewType＋opaque＋accessor・`#[non_exhaustive]`・`tracing` のみ・in-source テスト）を踏襲。本 spec は **`balloon` モジュール**を追加。
- **emo2 fixture**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/{descript.txt, balloons0s.txt}`（`descript.txt` 84 行／`balloons0s.txt` 27 行・`type,balloon`・`use_self_alpha,1`）。`k0s` は未 vendored。
- **出力モデル型は未存在**＝本 spec で定義（descript フィールド＋マージ済み s0s/k0s 状態）。

## Desired Outcome

emo2 balloon 定義を、**base descript → サーフェス別 s0s/k0s 上書き**をマージしたバルーンモデルへ解析でき、emo2 fixture で pass。純粋関数・単体テストのみで観測可能。

## Approach

`areka-parsers` に `balloon` モジュールを追加し `sakura` パターン踏襲。**emo2 最小フィールド**（`doc/emo2-conformance-scope.md`）:

- `type,balloon`・`use_self_alpha,1`・画像（`balloons0.png` 400×224／`balloonk0.png` 288×203）。
- `origin.x,y`・**`windowposition.x,y`**（descript base → s0s/k0s で上書き・kero 例 x=266/y=-129）。
- **`validrect.top,bottom,left,right`**（**負値＝逆端基準**の解釈に注意）。
- **`wordwrappoint.x,y`**（**負値＝右端基準**・`x,-34` 等）。
- `font.name`（Yu Gothic UI）・`font.height`（28）・`font.color` RGB・`anchor.font.color` RGB（リンク文字色）。
- `arrow0.x,y` / `arrow1.x,y`（スクロール矢印）。
- **マージ規則**: base descript に s0s/k0s をサーフェス別 overlay。
- **M1 省略**（emo2-kakukaku 未使用）: `communicatebox`／`onlinemarker`／`sstpmarker`／`marker.png`／`number.*`／cursor style。未知は寛容に `Raw` へ。

## Scope

- **In**: `areka_parsers::balloon` モジュール。バルーンモデル型定義。`descript.txt` パース＋`s0s`/`k0s` マージ（windowposition/validrect/wordwrappoint の**負値基準**・font・arrow・origin）。emo2 fixture テスト。
- **Out**: バルーン描画・文字レイアウト（`text-layer`）。surface 合成（`surface-engine`）。emo2 未使用の marker/communicatebox/number/cursor。`k0s`（未 vendored ゆえ構造対応のみ・実データ検証は s0s）。

## Boundary Candidates

- descript フィールドパース（font/origin/arrow/画像）
- 座標フィールドの負値基準解釈（validrect / wordwrappoint / windowposition）
- base → s0s/k0s サーフェス別マージ

## Out of Boundary

- 文字描画・折返し実行（`text-layer`）・surface 合成。
- 他 parser（shell/package）の領分。

## Upstream / Downstream

- **Upstream**: `areka-parsers`（`sakura` パターン）／emo2 fixture。
- **Downstream**: `areka-P0-text-layer`（バルーン文字層・折返し点/font 消費）／`areka-P0-surface-engine`（統一ゆえバルーン枠も surface）／`areka-P0-choice-render`（増分・選択肢表示）。

## Existing Spec Touchpoints

- **Extends**: `areka-parsers`。
- **Adjacent**: `areka-P0-shell-parse` / `areka-P0-package-mount`（同クレート別モジュール・非衝突）。統一グラフィック方針ゆえ shell モデルと最終的に同一 surface 系へ合流（合流は engine 側）。

## Constraints

- Rust 2024・std 中心・`tracing` のみ・`Result` 無しの寛容パース・`#[non_exhaustive]`。
- **座標の負値基準を取り違えない**（validrect/wordwrappoint/windowposition）。emo2 実物 fixture で検証。
- **過剰実装禁止**（emo2 使用フィールドのみ）。不確実は `doc/emo2-conformance-scope.md`／fixture を正とし、無ければ質問。
