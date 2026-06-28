---
inclusion: manual
updated_at: 2026-06-28
---

# Roadmap — areka ukadoc互換ベースウェア

> **配置方針**: 本ファイルが**ロードマップの正本**。kiro ツールチェーン（`/kiro-discovery` 再入・`/kiro-spec-batch`）が標準パス `.kiro/steering/roadmap.md` で参照する。`inclusion: manual` ゆえ毎セッションの自動ロードはされず（コンテキスト最小化）、`focus.md`（`inclusion: always`・参照/更新タイミングと配置ルールの lean ポインタ）から辿る。`doc/ROADMAP.md` はポインタ stub（旧リンク互換のため残置）。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。

## Overview

ゴール: ① ukadoc 準拠の**互換ベースウェア**（SSP 代替・既存伺かゴーストが実際に動く）を確立 → ② **ぱすたさん**（native 旗艦ゴースト）を同じ土台の上に建てる。互換ベースウェアを先行し、ぱすたさんは後続。最難関の互換部を前倒しで潰してリスクを早期に溶かす戦略（v2.0 戦略転換・2026-06-26）。

北極星（縦スライス）:
- **M1（互換ベースウェア）**: 実在の里々ベースゴースト1体が SAORI 込みで実際に表示・会話する。
- **M2（ぱすたさん）**: 同じ土台で native 脳 pasta＋階層サーフェスの本領が動く。

## Approach Decision

- **Chosen**: 二枚看板・**互換先行**。①互換ベースウェア（既存伺か資産を動かす）→ ②ぱすたさん（native 旗艦）。互換契約は ukadoc 正典（SERIKO/MAYUNA 完全マップ／さくらスクリプト優先度順／沈黙時は areka 裁量＋対応表記録）。
- **Why**: 既存ゴーストが実際に動く達成感がモチベーションを最大化し、最難関の互換部を前倒しで解消できる。
- **Rejected alternatives**: v1.x ボトムアップ表示層計画（→本トラック体系へ再マップ・破棄ゼロ）。`_rejected/`: `wintf-P0-click-through-rgn`（`SetWindowRgn` は DComp 描画をクリップし両立不可）, `wintf-P1-clickthrough`（完了済みクリック透過に超越）, `areka-P1-legacy-converter`（互換ベースウェアで里々をネイティブ実行する方針により役割消失）。

## Scope

- **In**: 階層サーフェス/アニメーションエンジン（SERIKO 内包）、さくらスクリプト互換＋バルーン、SHIORI ホスト（IShiori COM＋32bit 過去互換）、シェル/バルーンパッケージローダ、互換ゴースト E2E 統合。
- **Out**: M2 ぱすたさん native 旗艦（互換土台の後続）、アプリ統合・出荷層（トレイ/永続化/パッケージマネージャ/MCP）は M1 クリティカルパス外。

## Constraints

- Rust 2024・マルチクレート（wintf/dola/areka＋最小依存 `shiori-abi`）。32bit 可搬性を崩さない（恒久コード依存を最小化）。
- 透過は ULW/DComp 切替式（実装済み・ULW 既定）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト（flat-C/HGLOBAL/charset/SAORI 同居/自前 IPC）。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。

## Boundary Strategy

- **Why this split**: M1 を 4 トラック（T1 階層サーフェス Engine／T2 さくらスクリプト＋balloon／T3 SHIORI ホスト／T4 統合）に分け、各トラックを独立に前進可能にする。T1/T2/T3 は概ね並行、T4 が収束点。
- **Shared seams to watch**: SHIORI 契約（com→protocol→protocol-split で確定済み）と host-32/reference の境界。balloon-system 再スコープ（さくらスクリプト駆動＋balloon ローダ前提）と sakura-script/balloon-loader の依存。dola→wintf バインディング（animation-system）と階層サーフェス（surface-hierarchy）の seam。

## 解決済み基盤（伺か土台の最難関は完了済み）

- 透過/ULW/click-through（DComp→ULW 移行完遂・別プロセス自動透過）、event/hit-test/alpha-mask、dola 演出ランタイム（コア〜ループ/nested）。
- **T3 SHIORI 契約チェーン完了**: `areka-P0-shiori-com`（内部唯一 ABI）→ `areka-P0-shiori-protocol`（json-rpc 2.0 正準 content・446 entry/802 field 正本）→ `areka-P0-shiori-protocol-split`（単一 TOML をフラグメント群へ非破壊分割・論理 SSOT 化）。3 仕様すべて completed/・PR マージ済み。

## Specs (dependency order)

> `[x]`=完了（`completed/`）, `[ ]`=未完了。「brief」= `brief.md` 済・`spec.json` 未生成（`/kiro-start` または `/kiro-spec-init` で着手）。「拡張」= 既存 spec のスコープ拡張（新規 brief なし）。

- [x] areka-P0-shiori-com -- 内部唯一 ABI `IShiori`(COM)＋ネイティブ in-proc。Dependencies: none
- [x] areka-P0-shiori-protocol -- 正準 content プロトコル json-rpc 2.0 定義（D5 着地）。Dependencies: areka-P0-shiori-com
- [x] areka-P0-shiori-protocol-split -- 単一 TOML 正本をフラグメント群へ非破壊分割し論理 SSOT 化。Dependencies: areka-P0-shiori-protocol
- [ ] areka-P0-shiori-reference -- 簡易リファレンス COM-SHIORI（「正解見本」DLL 契約・content 不透明）。Dependencies: areka-P0-shiori-com 〔brief・**次の着手候補（依存充足済み）**〕
- [ ] areka-P0-shiori-host-32 -- 32bit Rust 過去互換ホスト＋SAORI 同居。Dependencies: areka-P0-shiori-com, areka-P0-shiori-reference 〔brief・reference 待ち〕
- [ ] wintf-P0-surface-hierarchy -- 汎用の階層アニメーション・サーフェス合成（wintf）。Dependencies: wintf-P0-animation-system 〔brief〕
- [ ] areka-P0-seriko-runtime -- SERIKO/MAYUNA を ukadoc 完全マップで解釈（areka）。Dependencies: wintf-P0-surface-hierarchy 〔brief〕
- [ ] areka-P0-shell-loader -- 伺かシェルパッケージ読込→surface モデル（areka）。Dependencies: areka-P0-seriko-runtime 〔brief〕
- [ ] areka-P0-sakura-script -- さくらスクリプト runner（優先度順, areka）。Dependencies: areka-P0-seriko-runtime, wintf-P0-balloon-system 〔brief〕
- [ ] areka-P0-balloon-loader -- 伺かバルーンパッケージ読込（areka）。Dependencies: wintf-P0-balloon-system 〔brief〕
- [ ] areka-P0-compat-ghost-integration -- 実在里々ゴースト1体を E2E 起動（M1 北極星）。Dependencies: areka-P0-shell-loader, areka-P0-seriko-runtime, areka-P0-sakura-script, areka-P0-balloon-loader, areka-P0-shiori-host-32 〔brief〕

### 既存仕様のスコープ拡張（新規 brief なし）
- wintf-P0-animation-system -- dola→wintf バインディングに「階層サーフェス＋SERIKO 再生プリミティブ」を追加（T1 の心臓・要件生成済）。
- wintf-P0-balloon-system ＋ balloon01〜06 -- 「さくらスクリプト駆動＋balloon パッケージ読込」前提へ再スコープ（balloon-system は設計承認済・タスク 8/9）。

## M2 — ぱすたさん（native 旗艦・互換後続）
- areka-P0-reference-shell / areka-P0-reference-balloon / areka-P0-reference-ghost（active・要件ドラフト）。pasta 脳が `IShiori` を native 実装。pasta エンジンは `completed/areka-P0-script-engine`（vendored `vendors/pasta/`）。

## アプリ統合・出荷（M1 クリティカルパス外・P0 active）
- areka-P0-system-tray / -persistence / -package-manager / -mcp-server / -window-placement（要件ドラフト）。

## クリティカルパス（M1）
animation-system＋surface-hierarchy → seriko-runtime＋shell-loader → sakura-script＋balloon（balloon-system/loader） → shiori-host-32（reference 経由） → compat-ghost-integration

## ポートフォリオ実数（2026-06-28・配置フォルダ基準）
| 配置 | 件数 |
|------|:----:|
| `completed/` | 97 |
| `.kiro/specs/` 直下（active P0・spec.json 保持） | 17 |
| `.kiro/specs/` 直下（brief のみ・構想/未 init） | 9 |
| `backlog/`（待機 P1-P3） | 21 |
| `_rejected/` | 3 |

> 件数は **配置フォルダ基準**で数える（`phase` 値は履歴上ズレる）。集計・更新タイミング・配置ルールの運用正本は [focus.md](focus.md)。
