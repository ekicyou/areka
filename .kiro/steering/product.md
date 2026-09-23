---
inclusion: always
updated_at: 2026-09-20
---

# Product Overview

このワークスペースは、Windows向け縦書きUIフレームワークwintf、宣言的アニメーション基盤dola、それらを統合するデスクトップマスコット試作アプリarekaの3層で構成されています。目標は、「伺か」のような常駐キャラクターアプリをRustとWindowsネイティブAPIで実現できる基盤を整えることです。

## Core Capabilities

- **ウィンドウ管理**: Win32 APIを使用したウィンドウの生成、管理、メッセージ処理
- **2D描画**: Windows.UI.Composition（WUC・2026-07-02 DComp から移行完了）、Direct2D、DirectWriteを活用した高品質な2D描画
- **縦書きテキスト**: DirectWriteによる日本語縦書き・横書き両対応のテキストレンダリング
- **透過ウィンドウ**: GPU 合成（WUC）＋`WS_EX_TRANSPARENT` 動的トグルによる透過処理とヒットテスト（ULW は 2026-07-05 撤去済み＝GPU 合成単独）
- **画像表示**: WIC（Windows Imaging Component）を使用した透過画像の読み込みと描画
- **演出オーケストレーション**: dolaによるアニメーション定義、CueSheetによる離散演出制御
- **ゴースト試作統合**: arekaによるシェルとバルーンの2ウィンドウ構成、会話・表情・入力導線の統合検証

## Target Use Cases

- 「伺か」のようなデスクトップマスコットアプリケーション
- 日本語縦書きテキストを必要とするWindows向けGUIアプリケーション
- 透過ウィンドウと高度なインタラクションを持つデスクトップツール
- WUC/Direct2Dを使用した高パフォーマンスな2D描画アプリケーション

## Value Proposition

Rust言語による型安全性とメモリ安全性を保ちながら、Windows固有の低レベルAPIを使った透過ウィンドウ、縦書きテキスト、演出制御を一貫した設計で扱えます。既存UIフレームワークでは扱いづらい、日本語縦書きとデスクトップマスコット表現を中核ユースケースとしている点がこのプロジェクトの価値です。

## areka アプリケーション層

**areka**はwintfを基盤としたデスクトップマスコット・プラットフォームです。「伺か」にインスピレーションを得た、ゴースト（キャラクター）常駐型のデスクトップアプリケーションを実現します。

### 構成

- **areka バイナリクレート**: wintf/dolaを統合し、ゴースト実行環境の試作を提供
- **dola ライブラリクレート**: 宣言的アニメーション定義フォーマット（JSON/TOML/YAML）
- **pasta** *(ベンダリング済みサブモジュール)*: 里々インスパイアの会話記述DSLスクリプトエンジン。`vendors/pasta/` に**調査資料として**同梱する（SHIORI flat-C ABI の一次源・`virtual_dispatcher.lua` の talk 抑制ゲートの出典）。ビルド依存は持たない＝`cargo` はこのサブモジュールが未取得でも通る

### 製品ポジショニング: ukadoc互換ベースウェア（2026-06-26 転換）

areka は「ぱすたさん専用の試作」から、**ukadoc準拠の互換ベースウェア（SSP代替）**へ狙いを定め直した。新規ユーザーを得るには既存の伺かエコシステム資産（既存ゴースト/シェル/バルーン）が動くことが要件となるため。

- **二枚看板**: ①互換ベースウェア（既存伺か資産を動かす）／②ぱすたさん（native旗艦ゴースト）
- **順序は互換ベースウェアを先行**（達成感によるモチベーション最大化と、最難関の互換部の前倒し）
- **互換契約**: ukadoc が正典。SERIKO/MAYUNA完全マップ、さくらスクリプトは優先度順、沈黙時はareka裁量＋対応表記録
- 確定した戦略・アーキテクチャ判断の正本は `doc/COMPAT_ARCHITECTURE.md`

### 現在の進め方

- wintfでWindowsネイティブUI基盤を先に固める（透過/ULW・当たり判定は完了済み。**2026-07 方針転換: 表示合成は DirectComposition → Windows.UI.Composition へ✅移行完了（07-02）・別プロセス透過は `WS_EX_TRANSPARENT` 動的トグルを wintf 本体へ✅実装完了（07-02・`wintf-clickthrough-alpha-toggle`）・ULW 除去も✅完了（07-05・`wintf-ulw-removal`）**＝詳細は tech.md／roadmap.md）
- **M1 のエンジン固有名（2026-07-02 確定）**: 7トラック ⓪〜⑥＝**`ghost` / `shiori` / `parsers` / `kanade` / `sakura` / `seriko` / `emo`**。コード/spec/会話の参照はこの名で統一（正本は roadmap.md「エンジン固有名」節。`emo` エンジンと適合ゴースト `emo2` は別概念）
- dolaを「タイミングに特化した下位層」と位置づけ、SERIKO/さくらスクリプトランナーをその上に建てる
- 互換ベースウェアとして実在ゴースト1体を動かす縦スライスを先に通し、その後ぱすたさんをnative旗艦として実装する

### アルファリリースターゲット（2026-09-18 棚卸⑭で改訂）: α＝第三者がデスクトップマスコットを管理できる

| 段 | 内容 | 状態 |
| ------ | ------ | ------ |
| M1 互換ベースウェア | 適合ゴースト emo2（32bit SHIORI）を起動→会話→撫で→メニュー→終了まで E2E 実走 | ✅ 2026-09-11 完成宣言 |
| **M2 α 版** | zip を展開して起動 → `.nar` を窓へ落とす（またはメニュー）→ 起動 → 右クリックメニューでゴースト／シェル／バルーンを替える → ネットワーク更新 → 終了 → 再起動で前回の状態へ。表現力は M1 の水準で据え置き・オーナードローは持たない | 進行中（`roadmap.md` A0〜A5） |
| M3 以降 | 表現力（ukadoc 網羅の段階 A〜E）・ぱすたさん（native 旗艦：pasta DSL・階層サーフェス・縦書きタイプライターバルーン） | α 完了後に組み直し |

### 適合の検体＝SHIORI 3 系統の標準テンプレート（2026-09-20 時点）

互換の物差しは「emo2 1 体」から **3 系統の検体**へ広がった。いずれも `vendors/sample_ghost/*.nar`（配布形のまま保管）で、窓口は `sample-ghost-kit`。

| 検体 | SHIORI | 位置づけ |
| ------ | ------ | ------ |
| `emo2` | pasta（32bit） | M1 の適合ゴースト。`surfaces.txt` に `element` を書く流儀 |
| `R_POST_and_KOMAINU` | 里々 | 標準テンプレート。**`surfaces.txt` に `element` 行が無く、`surface<数字>.png` のファイル名の慣習だけで面が建つ**流儀 |
| `konnoyayame`（紺野ややめ） | YAYA | 標準テンプレート。同上。**シェルは CC BY-NC-ND＝開発用検体に限り、areka の配布物へ同梱しない・畳み直さない** |

- テンプレート 2 体は `areka-P0-shell-implicit-surface`（2026-09-20）で**実機で動く**ようになった（ファイル名の慣習による面＋α の無い絵の抜き色透過＝左上 1 画素と完全一致の色を抜く＋`sometimes`／`rarely`）。α の検証に使う検体はこの 3 体＋既定バルーン（`roadmap.md` の裁定「nar-install を α の先頭に置く」の項）。YAYA の検体は「正しく展開される」から「実機で動く」へ進んだ段階で、出た不具合は個別に起票する。
- 第三者のゴーストの多くはバルーンを同梱しないので、**既定バルーンは CC0 の `StayseeBalloon` に確定**した（`areka-P0-default-balloon-bundle` 2026-09-19・`areka-P0-default-balloon-nar-fold` で `vendors/sample_ghost/StayseeBalloon.nar` へ無改変のまま畳んで保管し、登記表 `SAMPLES` 経由で引く・表示は決定論テストと実機目視で確認済み）。今は `areka.exe <ゴーストの根> <バルーンの根>` の第 2 引数で渡す形で、**バルーン無指定時の自動の既定採用は下流 spec の仕事**（id `StayseeBalloon` を申し送り済み）。
- α（M2）の着地済み: `.nar` インストールのエンジン（`areka-nar`）・既定バルーン・右クリックメニュー第 1 スライス（説明書／終了・OS ネイティブメニュー）・ファイル名の慣習による面。残りは `roadmap.md` A0〜A5。

詳細: `doc/COMPAT_ARCHITECTURE.md`, `doc/PASTA_PROFILE.md`, `.kiro/steering/roadmap.md`（ロードマップ正本）

---
Focus on patterns and purpose, not exhaustive feature lists.
