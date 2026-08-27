# Brief: areka-P0-zorder-property

> **種別**: 追跡 spec（正典先送り 4 点セット＝完全語彙＋縮退シーム＋**追跡 spec＋roadmap 明記**）。sylphya（統一プロパティシステム）＋ zsp グループ台帳の帰属。
> **源**: `areka-P0-scope-zorder-pinning`（zsp）要件 13。2026-08-27 の zsp 要件ディスカッション議題 3 で「追跡先の実在検証＝拾い手ゼロ」と判定され、開発者裁定「今作っておくべき」により即日起票。
> **着手ゲート**: **M1 外（M2 解禁ゲート棚）**。`currentghost` 配下プロパティの実導出を解禁する時、または実ゴーストが本プロパティを使うことが実測された時に解禁。

## Problem

ukadoc プロパティ **`currentghost.seriko.zorder`**（SSP 2.8.78・[SET有効]）は、現在の zorder 設定状態のスクリプトからの読み書きを定める。zsp（M1）はタグ `\![set,zorder,...]`／`\![reset,zorder]`／descript `seriko.zorder` の 3 入口を実装するが、**プロパティの読み書きは実装しない**（zsp 要件 13.1）。放置＝手抜きにせず、後続が実装できる完全な仕様をここに台帳化する。

## 完全語彙（ukadoc 2.8.78・zsp brief:18 から転記）

- **読み**: 現在の設定状態を返す。グループ内は**カンマ区切り**・グループ間は**セミコロン区切り**・要素は**手前から順**。
- 明示モード `s0,b0,s1;s2,b2` と数値モード `0,1;2,3` は**排他**（混在不可）。
- **書き込み**: 現在の設定の**完全置換**（タグ `\![set,zorder]` がグループを追加式に足すのと異なる）。
- **空文字列**の書き込みで全解除。
- 要素 **2 個未満**のグループは無視。

## Current State（2026-08-27）

- 縮退シーム: sylphya の `currentghost` 配下は M1 では NOT_FOUND 応答＝**何も実装しないことで現行どおりの応答が成立**（zsp 要件 13.2）。
- sylphya の SET 有効プロパティ一覧（`crates/areka-sylphya/src/vocab/dotted.rs`・21 項）に `seriko.zorder` は**入れない**（zsp 議題 3 裁定＝動かない名前の先行登録はしない。本 brief が語彙の正本）。
- zsp が着地させる**グループ台帳**（areka 側・scope／窓種別のまま保持）が、本プロパティの読み書きの唯一の情報源になる想定。

## Desired Outcome

（解禁時）読み＝グループ台帳の現在状態を上記の正典書式へ直列化して返す。書き＝完全置換としてグループ台帳へ反映し、タグと同じ検証規則（モード混在拒否・重複拒否・2 個未満無視など＝zsp の解釈純関数）を通す。

## Approach

- zsp のトークン解釈（parse）と対になる直列化（serialize）を足し、往復（parse→serialize→parse が恒等）を決定論テストで固定する見込み。
- 「名前で引ける値は 1 機構」（sylphya）に従い、バッキングは zsp グループ台帳＝可視性状態の二重帳簿を作らない先例（balloon-visibility R7.5）と同型。

## Scope

- **In**: プロパティ読み書きの実導出・SET 有効一覧への追加・書式検証規則の zsp との共有・タグ入口との整合（プロパティ書込後の是正発火）。
- **Out**: タグ／descript 入口（zsp で完成・不変）・窓の是正機構そのもの・他の `currentghost.*` プロパティ。

## Upstream / Downstream

- **Upstream**: `areka-P0-scope-zorder-pinning`（グループ台帳・解釈純関数・COMPAT §8 登記＝完成待ち）。
- **Downstream**: M2 互換面拡大の各ゴースト適合。

## Constraints

- M1 では着手しない（`surfaces-basepos`・`balloon-canon-residue` と同じ M2 解禁ゲート棚）。
- zsp の COMPAT §8 登記と本 brief は相互参照で重複記載しない（語彙の正本は本 brief）。
