# Brief: areka-P0-sakura-script

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §2,§3。互換契約=ukadoc正典。

## Problem
SHIORI（脳）の出力であるさくらスクリプトを解釈し、サーフェス切替・バルーン表示・ウェイト・選択肢を駆動できねば、会話が成立しない。さくらスクリプトは互換土台とnative脳pastaの双方の出力先となる結節点。

## Current State
balloon-system（設計承認済）＋ balloon01-06、typewriter（完了）、SERIKO ランタイム（予定）が揃いつつあるが、これらを束ねるさくらスクリプト実行器が無い。

## Desired Outcome
ukadoc 記載タグを**優先度順**に解釈するさくらスクリプト runner が areka に入る。`\s[n]`→サーフェス（SERIKO ランタイム）、テキスト/`\n`/`\w`/`\_w`/`\e`→バルーン、`\0\1\p[]` スコープ、選択肢 `\q` 系を駆動。

## Approach
パーサ＋実行器。テキスト/制御を逐次解釈し、サーフェス指示は seriko-runtime へ、表示・タイプライター速度・ウェイトは balloon＋dola タイミングへ、選択肢は入力導線へ。優先タグから着手し対応表で網羅状況を可視化。

## Scope
- **In**: 字句/構文パーサ、優先タグ集合（スコープ・サーフェス・改行・ウェイト・終端・選択肢）の実行、balloon/SERIKO/入力への差配、対応表
- **Out**: `\![...]` コマンドの全集合（段階対応）、SHIORI プロトコル本体（→ shiori 群）

## Boundary Candidates
- パーサ（タグ語彙）
- 実行器（balloon/surface/input へのディスパッチ）
- 優先度・対応表運用

## Out of Boundary
- SHIORI 通信、サーフェス合成の実体、バルーン描画の実体

## Upstream / Downstream
- **Upstream**: `wintf-P0-balloon-system`, `areka-P0-seriko-runtime`, `wintf-P0-typewriter`（完了）
- **Downstream**: `areka-P0-compat-ghost-integration`、ぱすたさんnative（pastaがscriptを吐く）

## Existing Spec Touchpoints
- **Extends**: balloon-system（駆動側）
- **Adjacent**: balloon04-choice/05-link（選択肢・リンク）

## Constraints
- ukadoc 準拠・優先度順。沈黙時は areka 裁量＋対応表記録。
