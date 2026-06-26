# Brief: areka-P0-balloon-loader

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §2。互換契約=ukadoc正典。

## Problem
既存ゴーストのバルーンを使うには、伺かバルーンパッケージ（balloon descript.txt、画像、位置/余白/フォント定義）を読み込み、balloon-system が描画できる形へ変換する必要がある。

## Current State
balloon-system（設計承認済）＋ balloon01-06 が描画側を担う見込みだが、既存バルーン形式を読むローダが無い。

## Desired Outcome
ディスク上の実在バルーンディレクトリを読み込み、balloon-system が消費する内部バルーン定義へ変換するローダが areka に入る。テキスト領域・余白・画像・位置決め・charset に対応。

## Approach
balloon descript.txt をパースし、画像（WIC）と各種メトリクス（テキスト矩形・余白・行間・座標）を内部モデルへ。balloon-system の描画パラメータへ写像。

## Scope
- **In**: balloon descript.txt パーサ、画像/メトリクス解決、位置決め定義、charset
- **Out**: バルーン描画の実体（→ balloon-system）、さくらスクリプト解釈（→ sakura-script）

## Boundary Candidates
- パーサ（balloon descript 語彙）
- メトリクス/位置決め写像
- リソース解決

## Out of Boundary
- 描画、スクリプト、SHIORI

## Upstream / Downstream
- **Upstream**: `wintf-P0-balloon-system`, WIC（完了）
- **Downstream**: `areka-P0-compat-ghost-integration`

## Existing Spec Touchpoints
- **Extends**: balloon-system（供給側）

## Constraints
- ukadoc 準拠。charset 規約に従う。沈黙時は areka 裁量＋対応表記録。
