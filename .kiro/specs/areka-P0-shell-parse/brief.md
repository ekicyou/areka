# Brief: areka-P0-shell-parse（本坑 / main・M1 M-boot / parser トラック）

> **種別**: 本坑（main）。M1 `areka-P0-emo2-boot` の **parser トラック**（roadmap「parsers＝並行・単体テスト可・host 不要」）。**依存は無し＝即着手可・安全並走**（`shell-parse ∥ balloon-parse ∥ package-mount` の一員）。
> **規律**: emo2 が実際に使う機能のみ実装（過剰・予測実装は禁止）。拡張は型の `#[non_exhaustive]` シームのみ残す。

## Problem

emo2 の `surfaces.txt`（SERIKO/2.0）を**シェルサーフェスモデル**へ解析する parser が存在しない。下流の `shell-anim-engine`（SERIKO ループ）と `surface-engine`（統一 surface 合成）が消費するモデルの生成源が要る。

## Current State（調査済み・接ぎ木先）

- **`areka-parsers` クレート**（`crates/areka-parsers/`・`areka-P0-sakura-parse` が作成）: `src/sakura/` に確立したパターン ── `pub fn parse(&str) -> Vec<Model>`（**`Result` 無し・寛容パス**・未知は `Raw` 変種へ吸収）、値型は **NewType＋opaque inner＋read-only accessor**、enum は `#[non_exhaustive]`、依存は `tracing` のみ、in-source `#[cfg(test)]` テスト。本 spec は同クレートへ **`shell` モジュール**を追加。
- **emo2 fixture**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt`（surface0／surface1000＋MAYUNA bind アニメ 1100–1801・overlay/bind/random パターン）。
- **出力モデル型は未存在**: surface モデル（SERIKO 状態＋element 合成＋collision 階層）は codebase に無い＝**本 spec で定義**（`wintf` の `SurfaceGraphics`/`VisualGraphics` は GPU リソースであってパーサ契約ではない）。

## Desired Outcome

emo2 `surfaces.txt` を surface モデル（surface 定義＋element overlay リスト＋SERIKO interval＋collision rect＋surface alias）へ解析でき、emo2 fixture で pass。純粋関数・単体テストのみで観測可能（host 不要）。

## Approach

`areka-parsers` に `shell` モジュールを追加し、`sakura` パターンを踏襲。**emo2 最小 feature set**（`doc/emo2-conformance-scope.md` 由来）:

- SERIKO/2.0（`seriko.use_self_alpha,1`・`alignmenttodesktop,bottom`）。
- **element は `overlay` メソッドのみ**（`overlayfast`/`base`/`replace`/`interpolate`/`move`/`add`/`reduce` は emo2 未使用＝実装しない）。負 ID `overlay,-1` はレイヤクリア。
- **interval は `bind` / `random,N` / `bind+random,N` の 3 種のみ**（talk/periodic/never/runonce 等は不要）。
- **collision は矩形のみ**（Head/Bust・`collisionex` 楕円/多角形は不要）。Z 順＝アニメ ID 昇順（painter's）。
- 全 offset `0,0`（per-element 座標変換不要）。
- **surface alias**（`kero.surface.alias`・`\s[静観]` 等）: alias 名は**不透明に扱い parse しない**（`\s[]` 中身は opaque passthrough）。
- 未知トークンは寛容に `Raw` 変種へ。モデル型は `#[non_exhaustive]` で将来 element/interval 拡張のシームのみ残す。

## Scope

- **In**: `areka_parsers::shell` モジュール。surface モデル型定義。`surfaces.txt` パース（surface 定義／element overlay／interval bind・random／collision rect／alias 透過）。emo2 fixture ベースの in-source テスト。
- **Out**: レンダリング・surface 合成（`surface-engine`）。アニメ実行・SERIKO ループ・MAYUNA 実行時合成（`shell-anim-engine`）。collision→region/actor 写像（`collision-geometry` 増分）。emo2 未使用の SERIKO method/interval/collisionex。

## Boundary Candidates

- surface 定義パース（ID→element 群）
- element overlay リスト（＋負 ID クリア）
- SERIKO interval（bind / random,N / bind+random,N）
- collision 矩形リスト
- surface alias 透過（opaque）

## Out of Boundary

- 描画・アニメ実行（下流エンジン）。
- 他 parser（balloon/package）の領分。

## Upstream / Downstream

- **Upstream**: `areka-parsers`（`sakura` パターン・`areka-P0-sakura-parse` 完了）／emo2 fixture。
- **Downstream**: `areka-P0-shell-anim-engine`（SERIKO ループ）／`areka-P0-surface-engine`（統一 surface 合成）／`areka-P0-collision-geometry`（増分・collision 消費）。

## Existing Spec Touchpoints

- **Extends**: `areka-parsers`（`areka-P0-sakura-parse` が作成した基盤クレート）。
- **Adjacent**: `areka-P0-balloon-parse` / `areka-P0-package-mount`（同クレート別モジュール・**非衝突・並走安全**）。

## Constraints

- Rust 2024・std 中心・依存は `tracing` のみ（`sakura` に倣う）。**`Result` 無しの寛容パース**・値型は NewType＋opaque＋accessor・enum は `#[non_exhaustive]`。
- **emo2 実物 fixture で検証**・**過剰実装禁止**（emo2 使用分のみ・2 例目の実物が要求するまで抽象を足さない）。
- 不確実な SERIKO/surfaces.txt 仕様は推測せず `doc/emo2-conformance-scope.md`／fixture を正とし、無ければ質問。
