# Brief: areka-P0-emo-compose

> **種別**: 本坑（main）。⑥ emo トラック直列チェーン **2/3**（emo-atlas → **emo-compose** → emo-present。旧 `areka-P0-emo-surface` の 2026-07-03 粒度分割）。
> **方針正本**: 合成は emo 自前・アトラス転写・1枚物（記憶 areka-emo-own-compositor-atlas／roadmap emo 節）。

## Problem

型付き Shell モデルと焼付済みアトラス（emo-atlas）はあっても、**surface を合成する頭脳——実サーフェスツリーの構築と、アトラス転写による1枚物ビットマップ合成——が存在しない**。DComp/WUC の visual ブレンドは SERIKO 合成メソッドをピクセル正確に写像できないため、この合成コアは自前で持つ（開発者決定）。

## Current State

- **emo-atlas（直列1・前提）**: element path → (頁, UV, trim_offset, original_size) と premultiplied BGRA 頁バッファを供給。
- **parser は転記層**: `areka_parsers::shell::Shell` は疎 id・`surface.append` ターゲット記述子（範囲非展開 `Vec<AppendTarget>`）・alias を**転記のまま**保持（記憶 areka-parser-transcribes-tree-downstream）。**展開・実ツリー構築は本ユニットの責務**。
- 合成メソッドの正典: ukadoc（overlay／overlayfast／replace／base／reduce=α乗算／asis／interpolate／blend-* 群）。emo2 は overlay 系 subset のみ使用（M1 スコープ表）。

## Desired Outcome

Shell モデル＋アトラスを入力に、**指定 surface id の合成済み1枚ビットマップ（premultiplied BGRA）を純粋に生成**できる。

**✔ 観測（単一 pass/fail・表示不要・純粋層）**: **オフスクリーン pixel 単体テスト**——emo2 fixture の surface0（base＋element）合成結果が期待ピクセル（golden または要点サンプリング）と一致。トリム済み element が「配置座標＋trim_offset」で見た目等価に転写されることを含む。

## Approach

1. **実サーフェスツリー構築**: 疎 id の解決・`append` の範囲展開と適用・alias 解決→「合成可能な Surface 定義」への正規化（純粋データ変換・単体テスト独立）。
2. **合成プラン**: Surface 定義→転写命令列（layer 順・**変換行列**（x,y は単位行列の特例）・合成メソッド・アトラス参照/入れ子参照）。
3. **合成実行**: アトラス転写で合成先バッファ（再利用・アロケーション無し）へ描画。**入れ子 surface 参照は再帰合成**（訪問集合で**循環検出・非パニック打ち切り**）。
4. **合成メソッド**: **写像表は全量作成**（D2D 標準ブレンドで可＝overlay=SourceOver・asis=α無視コピー・replace=矩形クリア＋コピー／生ピクセル要＝reduce=α乗算・interpolate・overlayfast 条件付き・blend-* 群）——**実装は emo2 使用分のみ**・他は型シーム。
5. **バックエンド選定（design 判断）**: CPU ピクセル演算 vs D2D オフスクリーン。判断材料: ①ピクセル忠実性（生ピクセル演算の要否——reduce 等は CPU が素直）②毎フレーム再合成コスト（M-life seriko-loop 前提・O(elements)・転写ベース）③スレッド制約（D2D device context 単一スレッド・WUC upload は UI スレッド）④単体テスト容易性（headless・golden 安定性）。**合成コア API はバックエンド非依存に切る**（差替え可能）。CPU 開始→必要時 D2D 化が有力だが design で確定。

## 設計指示・注意点

- **premultiplied 一貫性**: 合成演算は premultiplied 前提で組む（SourceOver: `dst = src + dst*(1-src_a)`）。straight α の式を混ぜない。
- **トリム契約の遵守**: 転写先座標＝element 配置＋trim_offset。空エントリ（全透明）は転写スキップ。**トリムが見た目を変えない**ことをテスト固定。
- **決定性**: 同一入力→バイト同一の出力（golden テスト前提）。浮動小数の丸め差が出る演算は整数/固定小数で。
- **再合成予算**: 合成先バッファ再利用・アトラス転写 O(elements)・途中アロケーション無し（M-life の毎フレーム再合成に耐える構造を最初から）。
- **DPI 非持込**: 合成はピクセル等倍（surface 原寸）。拡縮は表示側（wintf）の責務。
- **キャッシュはここでは持たない**: surface id→合成結果のキャッシュ・無効化は emo-present の領分（本ユニットは純粋関数に徹する）。
- 既知ドリフト解消: `balloon/model.rs:6` doc コメントの旧名 `text-layer`/`surface-engine` 参照を本チェーン着手時に追随修正（記憶 areka-engine-names）。

## Scope

- **In**: 実サーフェスツリー構築（疎 id・append 範囲展開・alias）／合成プラン（layer・行列・メソッド）／アトラス転写合成（1枚物生成）／入れ子再帰＋循環検出／合成メソッド写像表（全量）＋emo2 使用分実装／バックエンド選定（API 非依存化）／オフスクリーン pixel テスト。
- **Out**: アトラス焼付（**emo-atlas**）／表示・AlphaMask・キャッシュ・指令 API（**emo-present**）／SERIKO 再生・MAYUNA（seriko）／テキスト（emo-text-layer）／emo2 未使用メソッドの実装。

## Boundary Candidates

- ツリー構築（Shell→正規化 Surface 定義・純粋）／プラン生成（定義→命令列・純粋）／転写実行（命令列＋アトラス→画素）の三層。

## Out of Boundary

- wintf・WUC・窓・visual——本ユニットは**一切 wintf を知らない**（純粋層）。

## Upstream / Downstream

- **Upstream**: `areka-P0-emo-atlas`（直列1・アトラス表＋頁バッファ）／`areka-P0-shell-parse` ✅・`-balloon-parse` ✅。
- **Downstream**: `areka-P0-emo-present`（合成結果の表示・キャッシュ・指令 API）／`areka-P0-seriko-engine`（将来の毎フレーム再合成の駆動者）。

## Existing Spec Touchpoints

- **Extends**: なし（新設層）。**Adjacent**: `areka-P0-emo-atlas`（供給契約＝AtlasEntry/頁バッファの形）。

## Constraints

- Rust 2024・tokio 禁止・**新規依存なし**（アトラスのクレートは emo-atlas 側・合成コアは std のみが理想）。
- 最小実装＋薄い拡張シーム（emo2 使用分のみ・写像表/行列/入れ子/循環検出の**構造**は最初から）。
- 正典は ukadoc・emo2 fixture は最小適合サンプル。
