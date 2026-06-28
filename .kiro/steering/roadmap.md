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
> **二坑種別**（[two-tunnel.md](two-tunnel.md) 要件 1.3／6.5）: 各本坑に【直行】＝方向・実現可能性・手順が十分確実ゆえ先進坑を経ず本坑着手可、【pilot要】＝怪しい seam を持ち先進坑の go を前提とする（`_Depends(confirmed):` で名指し）、【統合】＝収束点の統合 spec（先進坑非対象）。先進坑コードは `crates/pilot/examples/<spec-name>/` に隔離（現状 `_template` のみ・実先進坑ゼロ）。

- [x] areka-P0-shiori-com -- 内部唯一 ABI `IShiori`(COM)＋ネイティブ in-proc。Dependencies: none
- [x] areka-P0-shiori-protocol -- 正準 content プロトコル json-rpc 2.0 定義（D5 着地）。Dependencies: areka-P0-shiori-com
- [x] areka-P0-shiori-protocol-split -- 単一 TOML 正本をフラグメント群へ非破壊分割し論理 SSOT 化。Dependencies: areka-P0-shiori-protocol
- [x] areka-P0-shiori-reference -- 簡易リファレンス COM-SHIORI（「正解見本」DLL 契約・content 不透明）。Dependencies: areka-P0-shiori-com 〔completed/・本坑 deliverable（pilot ではない）・非テスト native 脳＋areka 実走デモ＋shiori_create 生成入口〕
- [ ] areka-P0-shiori-host-32 【pilot要】-- 32bit Rust 過去互換ホスト＋SAORI 同居。Dependencies: areka-P0-shiori-com, areka-P0-shiori-reference ／ _Depends(confirmed): pilot/shiori-host-32 〔brief・**次の着手候補**。怪しい seam＝クロス bitness IPC／自前メッセージループ／SAORI 同居（要件 7.2 被覆）。先進坑で go 確認後に本坑着手（**開発者合意済 2026-06-28**）。HGLOBAL/charset マーシャリングは ukadoc 正典で確実ゆえ先進坑の検証範囲外〕
- [ ] wintf-P0-surface-hierarchy 【直行】-- 汎用の階層アニメーション・サーフェス合成（wintf）。Dependencies: wintf-P0-animation-system 〔brief・透過/合成基盤は解決済み（ULW/DComp）の増分ゆえ直行〕
- [ ] areka-P0-seriko-runtime 【直行】-- SERIKO/MAYUNA を ukadoc 完全マップで解釈（areka）。Dependencies: wintf-P0-surface-hierarchy 〔brief・ukadoc 正典で仕様確定ゆえ直行〕
- [ ] areka-P0-shell-loader 【直行】-- 伺かシェルパッケージ読込→surface モデル（areka）。Dependencies: areka-P0-seriko-runtime 〔brief・文書化済フォーマット読込ゆえ直行〕
- [ ] areka-P0-sakura-script 【直行】-- さくらスクリプト runner（優先度順, areka）。Dependencies: areka-P0-seriko-runtime, wintf-P0-balloon-system 〔brief・ukadoc 優先度順マップで確定ゆえ直行〕
- [ ] areka-P0-balloon-loader 【直行】-- 伺かバルーンパッケージ読込（areka）。Dependencies: wintf-P0-balloon-system 〔brief・文書化済フォーマット読込ゆえ直行〕
- [ ] areka-P0-compat-ghost-integration 【統合】-- 実在里々ゴースト1体を E2E 起動（M1 北極星）。Dependencies: areka-P0-shell-loader, areka-P0-seriko-runtime, areka-P0-sakura-script, areka-P0-balloon-loader, areka-P0-shiori-host-32 〔brief・収束点の統合 spec（先進坑非対象）。統合サプライズのリスクは各上流の go で前倒し吸収〕

### 二坑モデル依存マップ検証（要件 7・spec 分解時の手動チェックリスト）
- **被覆（7.2）**: 不確実な本坑は host-32 のみ→`_Depends(confirmed): pilot/shiori-host-32` で go ゲート付与済。他本坑は【直行】判定（要件 6.5・上記各行の理由）。
- **孤児なし（7.3）**: 先進坑は host-32 用 1 本のみ（未掘削）。対応本坑を持たない pilot・参照されない pilot なし。
- **循環なし／DAG（7.4）**: 上記 Dependencies は DAG（循環なし）。
- **合否基準明示（7.5）**: `pilot/shiori-host-32` の go 基準＝クロス bitness で実 32bit shiori.dll を 1 往復（load→request→unload）成功＋SAORI 同居 1 例＋窓持ち SHIORI のメッセージループ生存。go／違う／直す を開発者が判定。

### 既存仕様のスコープ拡張（新規 brief なし）
- wintf-P0-animation-system -- dola→wintf バインディングに「階層サーフェス＋SERIKO 再生プリミティブ」を追加（T1 の心臓・要件生成済）。
- wintf-P0-balloon-system ＋ balloon01〜06 -- 「さくらスクリプト駆動＋balloon パッケージ読込」前提へ再スコープ（balloon-system は設計承認済・タスク 8/9）。

## M2 — ぱすたさん（native 旗艦・互換後続）
- areka-P0-reference-shell / areka-P0-reference-balloon / areka-P0-reference-ghost（active・要件ドラフト）。pasta 脳が `IShiori` を native 実装。pasta エンジンは `completed/areka-P0-script-engine`（vendored `vendors/pasta/`）。

## アプリ統合・出荷（M1 クリティカルパス外・P0 active）
- areka-P0-system-tray / -persistence / -package-manager / -mcp-server / -window-placement（要件ドラフト）。

## クリティカルパス（M1）
animation-system＋surface-hierarchy → seriko-runtime＋shell-loader → sakura-script＋balloon（balloon-system/loader） → 〔pilot/shiori-host-32 go〕→ shiori-host-32（reference 経由） → compat-ghost-integration

## ポートフォリオ実数（2026-06-28・配置フォルダ基準）
| 配置 | 件数 |
|------|:----:|
| `completed/` | 99 |
| `.kiro/specs/` 直下（active P0・spec.json 保持） | 17 |
| `.kiro/specs/` 直下（brief のみ・構想/未 init） | 7 |
| `backlog/`（待機 P1-P3） | 21 |
| `_rejected/` | 3 |

> 件数は **配置フォルダ基準**で数える（`phase` 値は履歴上ズレる）。集計・更新タイミング・配置ルールの運用正本は [focus.md](focus.md)。

## 凡例: 依存記法（`Dependencies:` と `_Depends(confirmed):`）

本ファイルの Specs 一覧は spec 間の依存を**自由テキスト**で表記する。二種の記法を使い分ける：

- **`Dependencies: <spec>, <spec>`**（通常の依存メモ）— 「この spec はこれらの spec の上に建つ」という**順序上の依存**を示す非ゲート注記。着手の絶対ブロックを意味しない（依存先の進行度に応じて並行着手の判断余地がある）。本一覧の既存表記はこれ。
- **`_Depends(confirmed): <pilot-spec>`**（確定前提依存・ハードゲート）— **先進坑 go 必須の確定前提依存**。当該本坑 spec は名指しした先進坑（pilot・先進坑）の **go 判定が下るまで着手不能（BLOCKED）**。go は開発者が先進坑の出力を見て下す人間判断であり、自動判定ではない。`Dependencies:` の通常注記とは異なり、これは**確定済みのハードゲート前提**を明示的にマークする記法。

**運用上の注意（二重管理回避）**:
- `_Depends(confirmed):` は **roadmap.md の自由テキストにのみ**置く。spec.json の `dependencies` 配列には**記載しない**（同一事実を二箇所で保守すると齟齬の温床になるため・二重管理回避）。
- ゲート規律の全容（先進坑/本坑の役割・go 判定・BLOCKED 扱い・直行許容など）は本凡例の対象外。記法の意味は本節で自己完結するが、ゲート運用の正本は [two-tunnel.md](two-tunnel.md) の「ハードゲート」節を参照。
