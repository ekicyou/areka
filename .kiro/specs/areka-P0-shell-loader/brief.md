# Brief: areka-P0-shell-loader

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §2,§4。互換契約=ukadoc正典。

## Problem
既存シェルを動かすには、伺かシェルのパッケージ（descript.txt / surfaces.txt / surface*.png / collision・element 定義）を読み込み、SERIKO ランタイムが実行できる surface モデルへ変換する必要がある。

## Current State
SERIKO ランタイム（`areka-P0-seriko-runtime`）は surface モデルを実行できる見込みだが、ディスク上の伺かシェル形式を読むローダが無い。WIC 画像読込は基盤あり。

## Desired Outcome
ディスク上の実在シェルディレクトリを読み込み、SERIKO ランタイム／surface-hierarchy が消費する内部モデルへ変換するローダが areka に入る。文字コード（Shift_JIS/UTF-8）と SERIKO/2.0・1.4 差異に対応。

## Approach
descript.txt（seriko.version 等）→ surfaces.txt（surfaceN/elementN/animationN/collisionN）→ surface*.png を WIC で読み、内部 surface モデルへ。collision は wintf hit-test 領域へ写像。ukadoc の記述順・優先規則に準拠。

## Scope
- **In**: descript.txt/surfaces.txt パーサ、element 合成定義、collision 定義、charset 判定、SERIKO version 差異、画像解決
- **Out**: アニメ実行（→ seriko-runtime）、バルーン読込（→ balloon-loader）、nar 解凍/インストール（将来）

## Boundary Candidates
- パーサ（字句・構文・charset）
- モデル写像（ファイル定義→内部 surface モデル）
- リソース解決（画像/相対パス）

## Out of Boundary
- SHIORI、さくらスクリプト、パッケージ配布形式(nar/install.txt)の本格対応

## Upstream / Downstream
- **Upstream**: `areka-P0-seriko-runtime`（消費先モデル）、WIC画像（完了）
- **Downstream**: `areka-P0-compat-ghost-integration`

## Existing Spec Touchpoints
- **Adjacent**: `wintf-P0-image-widget`（完了・WIC）

## Constraints
- ukadoc 準拠。charset は Charset 規約に従い判定。沈黙時は areka 裁量＋対応表記録。
