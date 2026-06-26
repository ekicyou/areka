# Brief: areka-P0-seriko-runtime

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §2,§4。互換契約=ukadoc正典。

## Problem
既存伺かシェルを動かすには、surfaces.txt が定義する SERIKO/MAYUNA のアニメ意味（pattern/interval/method、着せ替え）を ukadoc 通りに解釈・実行する必要がある。

## Current State
wintf に階層サーフェス合成（`wintf-P0-surface-hierarchy`）と dola タイミングが用意される見込み。だが SERIKO の語彙（interval=sometimes/random/periodic/runonce/talk/yen-e/bind…、method=overlay/base/interpolate/replace…）を解釈する層が無い。

## Desired Outcome
**SERIKO/MAYUNA を ukadoc 完全マップ**で解釈し、wintf surface-hierarchy ＋ dola タイミングへ差配する SERIKO ランタイムが areka に入る。MAYUNA(bind/着せ替え)は階層モデルの特殊形として実現。

## Approach
SERIKO ランタイムをオーケストレータとして実装：pattern発火タイミング→dola、サーフェス合成→wintf surface-hierarchy、talk/mouse/bind/collisionトリガ→wintfイベント（hit-test/pointer）＋トーク数状態。SERIKOは階層エンジンの「平坦サブセット」として写像する。

## Scope
- **In**: SERIKO/MAYUNA の全 interval/pattern/method、トリガのwintfイベント結線、exclusiveグループ、collision定義の hit-test 連携、対応表（ukadoc条項→挙動→検証）
- **Out**: ファイル読込（→ `areka-P0-shell-loader`）、階層参照の汎用機構（→ surface-hierarchy）、さくらスクリプトの `\s[]`（→ sakura-script が本ランタイムを駆動）

## Boundary Candidates
- interval トリガ機構（時間系 vs イベント系）
- pattern method 適用（合成オペレータ）
- MAYUNA(bind) の階層写像

## Out of Boundary
- surfaces.txt パース、SHIORI、バルーン

## Upstream / Downstream
- **Upstream**: `wintf-P0-surface-hierarchy`, `wintf-P0-animation-system`
- **Downstream**: `areka-P0-shell-loader`（生成した surface モデルを実行）、`areka-P0-sakura-script`（`\s[n]`でサーフェス切替）

## Existing Spec Touchpoints
- **Adjacent**: `event-hit-test-alpha-mask`/`-named-regions`（collision連携・完了）
- **Extends**: なし（新規 areka 層）

## Constraints
- 典拠は ukadoc（SSP実挙動の模倣ではない）。沈黙時は areka 裁量＋対応表記録。wintf を伺か知識で汚さない（SERIKO知識は areka 側に閉じる）。
