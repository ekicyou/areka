---
inclusion: manual
updated_at: 2026-06-28
---

# Roadmap — areka ukadoc互換ベースウェア

> **配置方針**: 本ファイルが**ロードマップの正本**。kiro ツールチェーン（`/kiro-discovery` 再入・`/kiro-spec-batch`）が標準パス `.kiro/steering/roadmap.md` で参照する。`inclusion: manual` ゆえ毎セッションの自動ロードはされず（コンテキスト最小化）、`focus.md`（`inclusion: always`・参照/更新タイミングと配置ルールの lean ポインタ）から辿る。`doc/ROADMAP.md` はポインタ stub（旧リンク互換のため残置）。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。

## Overview

ゴール: ① ukadoc 準拠の**互換ベースウェア**（SSP 代替・既存伺かゴーストが実際に動く）を確立 → ② **ぱすたさん**（native 旗艦ゴースト）を同じ土台の上に建てる。互換ベースウェアを先行し、ぱすたさんは後続。最難関の互換部を前倒しで潰してリスクを早期に溶かす戦略（v2.0 戦略転換・2026-06-26）。

北極星（縦スライス・実物ゴール）:
- **M1（最小 SSP 互換ベースウェア）**: 適合対象ゴースト **emo2**（作者自作・脳=pasta.dll・**32bit SHIORI**）が areka（x64）上で「**そのまま**」起動→会話→撫で→メニュー→終了まで E2E 実走する。emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台（互換＝普及の入口）。実物スコープは [emo2-conformance-scope.md](../specs/areka-P0-compat-ghost-integration/emo2-conformance-scope.md)。
- **M2（ぱすたさん native 旗艦）**: 同じ土台で pasta を **native x64・`IShiori` in-proc** に建て直し、縦書き・ベクトル描画・AI へ膨らませる（差別化の出口）。

## Approach Decision

- **Chosen**: 二枚看板・**互換先行**。①最小 SSP 互換ベースウェア（自作 emo2 を実物適合基準に「そのまま」動かす）→ ②ぱすたさん（native x64 旗艦）。互換契約は ukadoc 正典に従うが、**M1 スコープは emo2 が実際に使う機能で実物定義**（完全網羅は生態系拡張へ後ろ倒し）。
- **Why**: 自作 emo2 を適合基準にすると(a)スコープが推測でなく実物で確定し(b)ドッグフード可能で(c)同じ汎用ブリッジが里々/YAYA へ波及する。「伺かっぽいマスコット」でなく「伺か互換系」であることが普及の引力＝長期ロードマップの起点。
- **Rejected alternatives**: v1.x ボトムアップ表示層計画（→本トラック体系へ再マップ・破棄ゼロ）。`_rejected/`: `wintf-P0-click-through-rgn`（`SetWindowRgn` は DComp 描画をクリップし両立不可）, `wintf-P1-clickthrough`（完了済みクリック透過に超越）, `areka-P1-legacy-converter`（互換ベースウェアで里々をネイティブ実行する方針により役割消失）。

## Scope

- **In（M1）**: emo2 を動かす最小実装 — 32bit SHIORI ホスト（host-32・**SAORI 同居は除外**＝emo2 未使用）、SERIKO/2.0＋MAYUNA bind サブセット（overlay z-order・interval 3種）、さくらスクリプト約12タグ＋`\![move]`、balloon ローダ（必須フィールド）、shell/package ローダ、emo2 E2E 適合。
- **Out（M1）**: 汎用シーングラフ/演出エンジン・縦書き・ベクトル・AI（→M2）、SAORI・Shift_JIS・里々/YAYA 網羅・SERIKO 追加機能・NAR インストール（→生態系拡張）、アプリ統合・出荷層（トレイ/永続化/パッケージマネージャ/MCP）。

## Constraints

- Rust 2024・マルチクレート（wintf/dola/areka＋最小依存 `shiori-abi`）。32bit 可搬性を崩さない（恒久コード依存を最小化）。
- 透過は ULW/DComp 切替式（実装済み・ULW 既定）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト（flat-C/HGLOBAL/charset/SAORI 同居/自前 IPC）。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。

## Boundary Strategy

- **Why this split**: M1 を **emo2 の見える増分による垂直スライス**（pilot→S0 骨格→S1 二人→S2 着せ替え→S3 生命感→S4 対話→S5 E2E）に切る。耐力壁 host-32 を先進坑で先に貫き、以降は動く土台への加算。big-bang 統合と完全忠実度の盛りすぎを同時に回避（水平レイヤ分解からの転換・2026-06-28）。
- **Shared seams to watch**: balloon の左右配置は shell descript（`*.balloon.alignment`）依存。`\s[]` は不透明文字列（日本語エイリアス可）。OnSecondChange が OnTalk/OnHour を**内部生成**（areka から送らない・二重発火注意）。host-32 charset は emo2=UTF-8、汎用化で Shift_JIS。座標の負値=反対端基準（balloon/SERIKO 共通）。

## 解決済み基盤（伺か土台の最難関は完了済み）

- 透過/ULW/click-through（DComp→ULW 移行完遂・別プロセス自動透過）、event/hit-test/alpha-mask、dola 演出ランタイム（コア〜ループ/nested）。
- **T3 SHIORI 契約チェーン完了**: `areka-P0-shiori-com`（内部唯一 ABI）→ `areka-P0-shiori-protocol`（json-rpc 2.0 正準 content・446 entry/802 field 正本）→ `areka-P0-shiori-protocol-split`（単一 TOML をフラグメント群へ非破壊分割・論理 SSOT 化）。3 仕様すべて completed/・PR マージ済み。

## Specs (dependency order) — M1 垂直スライス

> M1 は **emo2（適合対象・脳=pasta.dll・32bit SHIORI）が「そのまま」動く**ことを実物ゴールとし、各スライスを「**動く emo2 の見える増分**」で切る（層ごと完成→最後に統合する big-bang を回避）。スコープの実輪郭は [emo2-conformance-scope.md](../specs/areka-P0-compat-ghost-integration/emo2-conformance-scope.md)（実測正本）。
> 記法: `[x]`=完了, `[ ]`=未着手。**二坑種別**（[two-tunnel.md](two-tunnel.md)）: 【先進坑】=使い捨て探索, 【pilot要】=先進坑 go 前提（`_Depends(confirmed):`）, 【直行】, 【統合】=収束点。
> **旧水平 brief**（host-32/seriko-runtime/shell-loader/sakura-script/balloon-loader/surface-hierarchy）は**スライスへ再配分される素材**として disk 残置（spec 化前）。スライス spec 化時に該当 brief を畳み込み、旧 brief dir は archive する。

### 解決済み（SHIORI 契約チェーン・completed/）
- [x] areka-P0-shiori-com / -shiori-protocol / -shiori-protocol-split / -shiori-reference -- 「解決済み基盤」節を参照（内部唯一 ABI `IShiori`＋正準 content＋reference DLL）。

### M1 スライス（pilot → S0 … S5）
- [ ] **pilot/shiori-host-32** 【先進坑】-- x64 areka が emo2 の 32bit `pasta.dll` を 1 往復（load→OnBoot→`Value` 受領→unload）できるか検証。耐力壁の go 判定。`crates/pilot/examples/shiori-host-32/`。**最初に掘る・開発者合意済 2026-06-28**。
- [ ] **S0 骨格** 【pilot要】-- emo2 が起動挨拶を喋る（むらさき静止 surface0 ＋最小バルーン）。host-32 本実装＋package mount＋shell/sakura/balloon 各最小（`\p \s \n \w \e`＋テキスト）。_Depends(confirmed): pilot/shiori-host-32
- [ ] **S1 二人＋表情** 【直行】-- むらさき(side0)＆エモ(side1)両立・`\s[]` 表情切替（kero 丸ごと差替＋surface alias 不透明解決）。Dependencies: S0
- [ ] **S2 着せ替え** 【直行】-- むらさきの MAYUNA bind 多層合成（overlay z-order・8 bindgroup・mustselect・sakura.menu auto）。Dependencies: S1
- [ ] **S3 生命感** 【直行】-- まばたき（random / bind+random）＋矩形 collision(Head/Bust)＋OnMouseMove 撫で（areka が region/actor 解決）＋OnSecondChange 自発会話。Dependencies: S2
- [ ] **S4 対話** 【直行】-- ダブルクリックメニュー・`\q` 選択肢＋`\![*]`・`\_l`・`\![move]`＋OnChoiceSelectEx・OnClose。Dependencies: S3
- [ ] **S5 北極星 E2E** 【統合】-- emo2 を vendoring（submodule）し boot→talk→touch→menu→close 一周を適合テスト化。Dependencies: S4

### 二坑モデル依存マップ検証（要件 7）
- **被覆(7.2)**: 不確実な耐力壁 host-32 は `pilot/shiori-host-32` でゲート済。S0 が pilot go 前提、S1-S5 は S0 の動く土台への増分ゆえ【直行】。
- **孤児なし(7.3)／DAG(7.4)**: `pilot/shiori-host-32 → S0 → S1 → S2 → S3 → S4 → S5` の単純連鎖（循環・分岐なし）。
- **合否基準(7.5)**: pilot go 基準＝x64 から emo2 の 32bit `pasta.dll` を 1 往復成功＋窓持ち SHIORI のメッセージループ生存。**SAORI は emo2 未使用ゆえ go 基準外**。go／違う／直す を開発者が判定。

### 旧水平 brief → スライス再配分マップ
- `areka-P0-shiori-host-32` → **S0**（耐力壁本実装・**SAORI 同居は M1 除外**）
- `areka-P0-shell-loader` → **S0/S1**（package mount＋surfaces.txt パーサ）
- `areka-P0-seriko-runtime` → **S2/S3**（bind 合成＋まばたき。完全マップでなく SERIKO/2.0 サブセット）
- `areka-P0-sakura-script` → **S0→S4**（約12タグを段階実装）
- `areka-P0-balloon-loader` → **S0**（必須フィールドのみ・shell descript の alignment 参照）
- `wintf-P0-surface-hierarchy` / `wintf-P0-animation-system` → **S2/S3 に最小分のみ**（overlay z-order＋まばたきタイマー）。汎用シーングラフ/演出エンジンは **M2 へ後ろ倒し**
- `wintf-P0-balloon-system`(＋balloon01-06) → **S0/S4**（バルーン描画基盤・タスク 8/9 既進行）

### 生態系拡張（emo2 適合の後・互換面拡大）
- Shift_JIS charset / SAORI 同居 / 里々・YAYA 網羅 / SERIKO 追加 interval・method / collisionex / NAR インストール。emo2 マイルストーン達成後に順次着手。

## M2 — ぱすたさん（native 旗艦・互換後続）
- pasta を **native x64・`IShiori` in-proc** に建て直す（M1 の 32bit `pasta.dll`/host-32 経路の上位互換）。**同じ emo2 が脳だけ差し替えて動く**＝M1 の適合が M2 の土台を保証。pasta エンジンは `completed/areka-P0-script-engine`（vendored `vendors/pasta/`）。
- areka-P0-reference-shell / -reference-balloon / -reference-ghost（active・要件ドラフト）。
- ここで**汎用シーングラフ/演出エンジン・縦書き・ベクトル描画・AI** 等「やりたい方向」を展開（M1 で後ろ倒しした wintf 汎用合成エンジンが活きる出口）。

## アプリ統合・出荷（M1 クリティカルパス外・P0 active）
- areka-P0-system-tray / -persistence / -package-manager / -mcp-server / -window-placement（要件ドラフト）。

## クリティカルパス（M1）
pilot/shiori-host-32（go）→ S0 骨格 → S1 二人＋表情 → S2 着せ替え → S3 生命感 → S4 対話 → S5 北極星 E2E（emo2 そのまま実走）

## ポートフォリオ実数（2026-06-28・配置フォルダ基準）
| 配置 | 件数 |
|------|:----:|
| `completed/` | 99 |
| `.kiro/specs/` 直下（active P0・spec.json 保持） | 17 |
| `.kiro/specs/` 直下（brief のみ・構想/未 init） | 7 |
| `backlog/`（待機 P1-P3） | 21 |
| `_rejected/` | 3 |

> 件数は **配置フォルダ基準**で数える（`phase` 値は履歴上ズレる）。集計・更新タイミング・配置ルールの運用正本は [focus.md](focus.md)。
> **注（2026-06-28 再 carving）**: M1 を垂直スライス（S0-S5）へ再分解した。旧水平 brief 7 本は disk 残置（再配分素材）で件数は不変（99/17/7）。スライス spec はまだ disk 未作成（`/kiro-start` で順次 init し、対応する旧 brief を畳み込み・archive する）。

## 凡例: 依存記法（`Dependencies:` と `_Depends(confirmed):`）

本ファイルの Specs 一覧は spec 間の依存を**自由テキスト**で表記する。二種の記法を使い分ける：

- **`Dependencies: <spec>, <spec>`**（通常の依存メモ）— 「この spec はこれらの spec の上に建つ」という**順序上の依存**を示す非ゲート注記。着手の絶対ブロックを意味しない（依存先の進行度に応じて並行着手の判断余地がある）。本一覧の既存表記はこれ。
- **`_Depends(confirmed): <pilot-spec>`**（確定前提依存・ハードゲート）— **先進坑 go 必須の確定前提依存**。当該本坑 spec は名指しした先進坑（pilot・先進坑）の **go 判定が下るまで着手不能（BLOCKED）**。go は開発者が先進坑の出力を見て下す人間判断であり、自動判定ではない。`Dependencies:` の通常注記とは異なり、これは**確定済みのハードゲート前提**を明示的にマークする記法。

**運用上の注意（二重管理回避）**:
- `_Depends(confirmed):` は **roadmap.md の自由テキストにのみ**置く。spec.json の `dependencies` 配列には**記載しない**（同一事実を二箇所で保守すると齟齬の温床になるため・二重管理回避）。
- ゲート規律の全容（先進坑/本坑の役割・go 判定・BLOCKED 扱い・直行許容など）は本凡例の対象外。記法の意味は本節で自己完結するが、ゲート運用の正本は [two-tunnel.md](two-tunnel.md) の「ハードゲート」節を参照。
