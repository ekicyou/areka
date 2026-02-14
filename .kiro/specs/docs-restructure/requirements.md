# Requirements Document

| 項目 | 内容 |
|------|------|
| **Document Title** | 憲法系ドキュメント及びREADME.md再構成 要件定義書 |
| **Version** | 1.0 |
| **Date** | 2026-02-14 |
| **Author** | GitHub Copilot (Claude Opus 4.6) + えちょ |

---

## Introduction

本仕様書は、areka プロジェクトの「憲法系ドキュメント」およびREADME.mdの再構成に関する要件を定義する。

プロジェクトは2025年11月のECS移行以降、wintfフレームワーク（ECS + DirectComposition + Direct2D）の基盤レイヤーを堅牢に構築し、57件のスペックを完了してきた。しかし、ドキュメント群は開発過程で有機的に成長した結果、以下の問題を抱えている：

- **README.md**はwintfクレート視点の開発者モチベーション維持用フェーズリストであり、プロジェクト全体の顔になっていない
- **ロードマップ**が ukagaka-desktop-mascot メタ仕様の ROADMAP.md に埋もれており、「何をすればアルファリリースできるのか」が一目でわからない
- **設計理念・憲法**が steering/ と doc/spec/ と requirements.md に分散しており、体系化されていない
- **プロジェクトの二層構造**（wintf = UIフレームワーク / areka = デスクトップマスコットアプリ）が外部から認識しにくい

### アルファリリースターゲット

**ぱすたさん** — areka プロジェクト初のデスクトップマスコット。

| 属性 | 内容 |
|------|------|
| 名前 | ぱすたさん |
| 種別 | ゴースト（pasta DSL 解釈・実行） |
| シェル | 1体キャラクター表示（透過ウィンドウ、アイドルアニメーション） |
| バルーン | 縦書きタイプライター付き吹き出し |
| スクリプト | pasta DSL（里々インスパイアのカスタムDSL） |

### 現状サマリー（2026-02-14時点）

**実装済み（基盤レイヤー ~70%）:**
- ECSフレームワーク統合（bevy_ecs + bevy_app）
- ウィンドウ管理（Win32、マルチウィンドウ、WndProc→ECS接続）
- グラフィックスパイプライン（D3D11 → DXGI → DirectComposition → D2D）
- レイアウトシステム（Taffy Flexbox統合）
- ビジュアルツリー同期（ECS → DirectComposition自動同期）
- 画像ウィジェット（BitmapSource: WIC読込み、非同期タスクプール、アルファマスク）
- テキスト / Typewriter（横書き/縦書き、文字送り、pause/resume/skip）
- ポインターイベント（Tunnel/Bubble 2フェーズ、ヒットテスト、ダブルクリック）
- ドラッグ移動（ウィンドウドラッグ、制約付きドラッグ）
- マルチモニタ/DPI対応
- dola（宣言的アニメーション定義フォーマット）

**未実装（アプリケーション層）:**
- アニメーションシステム（dola → wintf 統合）
- バルーンシステム（吹き出しUI）
- ウィンドウ配置（デスクトップ端固定等）
- リファレンスゴースト/シェル/バルーン
- システムトレイ / 永続化 / パッケージマネージャ / MCPサーバー

---

## Project Description (Input)

憲法系ドキュメント及びREADME.mdの改訂。現在取っ散らかってる感じがしてあと何を実現すればとりあえずデスクトップマスコットを仮公開できるのか読めない。現在の実装状況と仕様をサルベージしたうえで、ロードマップやREADME.mdなどを再構成せよ。また、本システムが目指す憲法（ゴール・設計理念・基本実装やクレートなど）を整理する。README.mdは現状、開発者のモチベーション維持のファイルになっているが、本プロジェクトの顔として整理しなおし、現在のREADME.mdは別名にし、現在の達成状況を反映、更にフェーズを再構成して欲しい。とりあえずのゴールは「1体のデスクトップマスコット、ぱすたさんをアルファリリース」するところまで。ぱすたさんはシェルおよび縦書きのタイプライターをもち、pasta DSLを解釈・実行するゴーストである。

---

## Requirements

### Requirement 1: README.md の再構成

**Objective:** プロジェクト貢献者・閲覧者として、README.mdからプロジェクトの全体像・現在地・ゴールを即座に把握したい。それによりプロジェクトへの参加判断や開発方針の理解が容易になる。

#### Acceptance Criteria

1. **The** docs-restructure プロセス **shall** 現在の `README.md` を `doc/DEVLOG_ORIGINAL_README.md` にリネームして保存する
2. **The** 新 `README.md` **shall** 以下のセクションを含む：プロジェクト概要、スクリーンショット/デモ（プレースホルダー可）、技術スタック、クレート構成（wintf / dola / areka）、ビルド手順、アルファリリース目標（ぱすたさん）、現在の到達点、開発ロードマップ概要、ライセンス
3. **The** 新 `README.md` **shall** プロジェクトの二層構造（wintf = Windows UIフレームワーク / areka = デスクトップマスコットアプリ）を明確に説明する
4. **The** 新 `README.md` **shall** 「ぱすたさんアルファリリース」をマイルストーンとして提示し、残タスクの概要をロードマップセクションで示す
5. **The** 新 `README.md` **shall** 日本語で記述し、プロジェクトの「顔」として洗練された表現を用いる
6. **The** 新 `README.md` **shall** 現在の実装済み機能リストを客観的に反映する（達成済みチェックマーク付き）

---

### Requirement 2: 憲法ドキュメント（CONSTITUTION.md）の策定

**Objective:** 開発者として、プロジェクトの設計理念・ゴール・基本原則を一箇所で参照したい。それにより設計判断の一貫性を保ち、新規参加者の理解を促進できる。

#### Acceptance Criteria

1. **The** docs-restructure プロセス **shall** `doc/CONSTITUTION.md`（憲法ドキュメント）を新規作成する
2. **The** 憲法ドキュメント **shall** 以下の章を含む：ミッション宣言、設計理念（5原則以内）、責務境界（プラットフォーム憲法）、クレート構成と責務、技術的意思決定記録の参照先
3. **The** 憲法ドキュメント **shall** プラットフォーム / ゴースト / シェル / バルーンの責務分離表を ukagaka-desktop-mascot requirements.md から引用・再構成する
4. **The** 憲法ドキュメント **shall** スコープ外事項（アクセシビリティ強制・コンテンツ国際化等）を明示する
5. **The** 憲法ドキュメント **shall** MCP採用方針と ECS+DirectComposition アーキテクチャ選定の根拠を記載する
6. **If** 既存ドキュメント（steering/, research.md, design.md）と記載内容が重複する場合, **the** 憲法ドキュメント **shall** 参照リンクで原典を指し示し、二重管理を避ける

---

### Requirement 3: ロードマップの再構成

**Objective:** 開発者として、「ぱすたさんアルファリリース」までに何をすべきか、その依存関係と順序を即座に把握したい。それにより計画的に開発を進められる。

#### Acceptance Criteria

1. **The** docs-restructure プロセス **shall** `doc/ROADMAP.md` を新規作成し、アルファリリースに焦点を絞ったロードマップをまとめる
2. **The** ロードマップ **shall** 以下のフェーズ構成を採用する：
   - **Phase A（基盤完成）**: イベントシステム完成（drag-system）、アニメーションシステム（dola→wintf統合）
   - **Phase B（表示層）**: バルーンシステム、ウィンドウ配置
   - **Phase C（コンテンツ）**: リファレンスシェル/バルーン/ゴースト（ぱすたさん定義）
   - **Phase D（アプリ統合）**: システムトレイ、永続化、パッケージ構造
   - **Phase E（アルファ出荷）**: 統合テスト、README最終化、リリースビルド
3. **The** ロードマップ **shall** 各フェーズに対応する子仕様名（`.kiro/specs/` のディレクトリ名）を明記する
4. **The** ロードマップ **shall** 現在の完了状態（57件完了スペック、5件実装済み基盤要素）を反映したプログレスバー/サマリーを提供する
5. **The** ロードマップ **shall** Mermaid記法の依存関係図を含み、クリティカルパスを視覚化する
6. **When** フェーズの完了状況が変化した場合, **the** ロードマップ **shall** 更新されるべき箇所が明確に識別できる構成とする
7. **The** ロードマップ **shall** ukagaka-desktop-mascot ROADMAP.md を置き換える位置づけとし、旧ROADMAPは `doc/archive/` に移動する

---

### Requirement 4: アーキテクチャ概要ドキュメントの整備

**Objective:** 開発者として、クレート構成・モジュール構成・依存関係を俯瞰したい。それにより「この機能はどこにあるか」「この変更はどこに影響するか」を素早く判断できる。

#### Acceptance Criteria

1. **The** docs-restructure プロセス **shall** `doc/ARCHITECTURE.md` を新規作成する
2. **The** アーキテクチャドキュメント **shall** 以下を含む：クレート依存関係図、wintf の3層構造（COM → ECS → Message Handling）、dola の責務、areka アプリケーション層の概要
3. **The** アーキテクチャドキュメント **shall** wintf の ECS モジュール一覧（window, graphics, layout, widget, pointer, drag 等）を記載し、各モジュールの責務を1-2行で説明する
4. **The** アーキテクチャドキュメント **shall** steering/structure.md を原典として参照し、差分のみを記載する（二重管理回避）
5. **The** アーキテクチャドキュメント **shall** 新規開発者が「どのファイルを読めば理解が進むか」の推奨読書順序を提供する

---

### Requirement 5: ステアリングドキュメントの整合性更新

**Objective:** AIエージェントとして、steering/ のプロジェクト記述が実装の現状と一致していてほしい。それにより、スペック生成やコード生成の品質が向上する。

#### Acceptance Criteria

1. **The** docs-restructure プロセス **shall** `.kiro/steering/product.md` を更新し、areka アプリケーション層の記述とアルファリリースターゲット（ぱすたさん）を追加する
2. **The** docs-restructure プロセス **shall** `.kiro/steering/structure.md` を更新し、dola クレートの記述を追加する
3. **The** docs-restructure プロセス **shall** `.kiro/steering/tech.md` を更新し、dola クレート・bevy_ecs 0.18・windows-rs 0.62 等のバージョン情報を現状に合わせる
4. **The** docs-restructure プロセス **shall** `.kiro/steering/focus.md` を更新し、新ロードマップ（`doc/ROADMAP.md`）への参照に切り替える
5. **While** steering ファイルを更新する間, **the** docs-restructure プロセス **shall** 既存の構造・形式・詳細度のスタイルを維持する

---

### Requirement 6: 旧ドキュメントのアーカイブ

**Objective:** 開発者として、旧ドキュメントを失わずにアーカイブし、新ドキュメントとの混乱を防ぎたい。それにより履歴の追跡可能性を維持しつつ、最新情報への到達を妨げない。

#### Acceptance Criteria

1. **The** docs-restructure プロセス **shall** `doc/archive/` ディレクトリを作成し、置換されるドキュメントを移動する
2. **The** docs-restructure プロセス **shall** 旧 `README.md` を `doc/DEVLOG_ORIGINAL_README.md` として保存する
3. **The** docs-restructure プロセス **shall** `.kiro/specs/ukagaka-desktop-mascot/ROADMAP.md` を `doc/archive/ROADMAP_ukagaka_meta.md` に移動する（旧ロードマップ）
4. **If** `doc/spec_backup_20251101_082532/` が存在する場合, **the** docs-restructure プロセス **shall** `doc/archive/` 配下に移動する
5. **The** docs-restructure プロセス **shall** アーカイブ移動の git commit を実装変更とは別に行い、履歴を追跡可能にする

---

### Requirement 7: ぱすたさんプロファイルの定義

**Objective:** 開発者・コンテンツ作者として、アルファリリース対象の「ぱすたさん」の基本プロファイルを明文化したい。それにより、リファレンス実装の要件が明確になる。

#### Acceptance Criteria

1. **The** docs-restructure プロセス **shall** `doc/PASTA_PROFILE.md` を新規作成し、ぱすたさんの基本プロファイルを定義する
2. **The** プロファイル **shall** 以下を含む：キャラクター名、キャラクター設定（1段落）、シェル構成（サーフェス一覧）、バルーン種別（縦書きタイプライター）、ゴースト種別（pasta DSL）、最小限の会話サンプル
3. **The** プロファイル **shall** areka-P0-reference-ghost / shell / balloon の要件定義のインプットとして機能する位置づけとする
4. **The** プロファイル **shall** pasta DSL の概要（里々インスパイアの会話記述DSL）を1セクションで説明する
5. **Where** areka-P0-script-engine の設計が完了している場合, **the** プロファイル **shall** script-engine の対応機能への参照リンクを含む
