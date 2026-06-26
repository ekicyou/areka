# Brief: wintf-P0-surface-hierarchy

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §4。本briefはT1の汎用能力部分。

## Problem
SERIKOの平坦なエレメント合成では、入れ子のアニメ部品（例: 独立して瞬き・口パクする「顔」を胴体へ合成）を表現できない。areka-native旗艦と互換の双方が、階層的なサーフェス合成を必要とする。

## Current State
wintfは visual-tree（実装・同期・clip 完了）、ECS親子伝播（GlobalArrangement）、dola nested-storyboard、DolaAnimator を保有。だが「アニメするサーフェス・ノードを入れ子に組む」汎用モデルは未整備。

## Desired Outcome
wintfに、**汎用の階層アニメーション・サーフェス合成能力**が入る。1ノード＝（VisualGraphics＋子）で、子ノードはそれ自身がアニメし、親へ合成される。伺か非依存の汎用機能として提供。

## Approach
「サーフェス・ノード＝ECSサブツリー」「エレメント＝子エンティティ」「別サーフェス参照＝サブツリー埋め込み」。合成は visual-tree、時間は dola nested-storyboard＋DolaAnimator。SERIKO意味は持ち込まず、純粋な合成/タイミングのプリミティブに留める。

## Scope
- **In**: 入れ子サーフェス・ノードのデータモデル、合成順序、ノード毎タイミング結線、循環検出、多重インスタンス同一性
- **Out**: SERIKO/MAYUNA意味解釈（→ `areka-P0-seriko-runtime`）、ファイル形式の読込（→ loader群）

## Boundary Candidates
- 合成モデル（ツリー構造・Zオーダ・クリップ）
- タイミング結線（DolaAnimator/nested-storyboard との接続）
- 循環・インスタンス管理

## Out of Boundary
- surfaces.txt 等のパース、SERIKOの pattern/interval 語彙
- バルーン・スクリプト・SHIORI

## Upstream / Downstream
- **Upstream**: `wintf-P0-animation-system`（dola→wintf バインディング）、visual-tree（完了）、dola nested-storyboard（完了）
- **Downstream**: `areka-P0-seriko-runtime`（この汎用能力の上にSERIKO意味を載せる）

## Existing Spec Touchpoints
- **Extends**: `wintf-P0-animation-system`（再生制御プリミティブを共有）
- **Adjacent**: visual-clip, visual-tree-synchronization

## Constraints
- wintfは伺か非依存を維持（汎用UIフレームワークの責務分離）。COMライフタイム規約、ECSスケジュール順序に従う。
