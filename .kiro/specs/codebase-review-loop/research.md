# Gap Analysis: codebase-review-loop

analyzed_at: 2026-06-10

## 1. 現状調査（Current State Investigation）

### 1.1 コードベース全体像

| クレート | 総LOC | ファイル数 | テストLOC | in-sourceテスト | unsafe | unwrap/expect/panic | テスト空白地帯 |
|---------|-------|-----------|----------|----------------|--------|---------------------|----------------|
| areka | 399 | 1 | 0 | 0 | 0 | 1 (panic!) | クレート全体 |
| dola | 6,849 | 30 | 9,236 | 9 | 1 | 66 | compile/, validate/ |
| wintf | 27,973 | 94 | 12,740 | 23 | 313 (block+impl) | 110 | ecs/cue/, ecs/window/, ecs/world/ |
| **合計** | **35,221** | **125** | **21,976** | **32** | **314** | **177** | 5モジュール |

主要な観察:
- **wintf がコードベースの約80%、unsafe の99%を占める。** COM相互運用層（com/d2d: 37 unsafe、dcomp.rs: 33 unsafe）と graphics（57 unsafe + 14 unsafe impl + 27 expect()）が高リスク集中域。
- **dola/runtime/** は最大密度モジュール（3,832 LOC）。テストは比較的厚いが unwrap/panic 53箇所のエラーハンドリング・ホットスポット。
- **テスト空白地帯**: areka 全体（399 LOC）、dola/compile（839 LOC）、dola/validate（583 LOC）、wintf の ecs/cue + ecs/window + ecs/world（計 3,841 LOC、in-sourceテスト 0）。
- **最大単一ファイル**: wintf/src/win_message_handler.rs（1,399 LOC、`#[deprecated]` 指定済みレガシー）。非推奨3モジュール（win_message_handler / win_thread_mgr / winproc）が計 1,838 LOC 残存。
- **`#[allow(...)]` 抑制**: dola 4箇所、wintf 15箇所（レビュー時に正当性確認の対象）。
- **examples/**: wintf に19ファイル（6,307 LOC）。手動検証資産でありテストの代替ではない（steering 規約どおり）。

### 1.2 横断的プロジェクト設定の現状

| 項目 | 状態 | 備考 |
|------|------|------|
| CI（.github/workflows/） | **欠落** | 自動検証なし。レビューループは ローカル検証コマンドに依存する |
| clippy.toml / rustfmt.toml | 欠落 | デフォルト設定で運用 |
| .editorconfig / deny.toml / .cargo/config.toml | 欠落 | サプライチェーン監査基盤なし |
| ルート Cargo.toml | 存在 | workspace deps 一元管理、release はサイズ最適化 + LTO |
| .vscode/ | 存在 | tasks.json（build/test）、launch.json（**古いバイナリパス `sample_dcomp.exe` 残存**） |
| .gitignore | 存在 | 最小限 |
| vendors/pasta (submodule) | 存在 | **当初未初期化でワークスペース全体がビルド不能だった**（本分析中に初期化済み） |
| doc/ | 存在 | 10ファイル・2,835行。CONSTITUTION.md 等。正式な検証手順書はなし |

### 1.3 検証コマンドベースライン

| コマンド | 状態 |
|---------|------|
| cargo 1.96.0 | ✅ 利用可能 |
| cargo clippy 0.1.96 | ✅ 利用可能 |
| cargo fmt | ✅ 利用可能（rust標準） |
| cargo test --workspace | 実行中（ベースライン計測を本分析末尾の追記で確定） |

**重大発見**: `vendors/pasta` submodule が未初期化の場合、`pasta_core` のパッチ解決に失敗し **workspace 全体のビルド・テストが不能**。レビューループの検証ゲート（R2.5, R4）の前提条件として「submodule 初期化確認」を環境準備ステップに必須化する必要がある。

### 1.4 利用可能なオーケストレーション基盤（Upstream 資産）

- `kiro-review`（タスク局所の敵対的レビュー・プロトコル）→ R4.1 自己レビューに直結
- `kiro-debug`（根本原因優先デバッグ）→ R4.2 のデバッグ試行に直結
- `kiro-verify-completion`（新鮮な証拠による完了検証）→ R4.4 コミット前ゲートに直結
- `/karpathy-guidelines`（シンプル化判定基準）→ R2.2 に直結
- `/kiro-impl`（タスク単位サブエージェント + 独立レビュー + 最終検証の自律モード）→ R3 の実行基盤
- steering `structure.md` が wintf の責務マップ（COM層 + ECS 9サブシステム）を提供済み → 領域分解の根拠資料

## 2. 要件実現可能性分析（Requirements Feasibility）

### Requirement-to-Asset Map

| 要件 | 既存資産 | ギャップ | タグ |
|------|---------|---------|------|
| R1 マトリクス構築・領域分解 | structure.md 責務マップ、本分析の実測LOC | 領域定義の形式（タスクへの落とし込み方式）が未定義 | Missing |
| R2.1 テスト網羅性 | tests/ 21,976 LOC、命名規約あり | カバレッジ計測手段なし（cargo-llvm-cov 等未導入）。空白地帯は実測済み | Missing/Unknown |
| R2.2 シンプル化 | karpathy-guidelines スキル | 適用は新規作業。非推奨モジュール 1,838 LOC の扱い判断 | Constraint |
| R2.3 脆弱性レビュー | unsafe 314箇所の実測マップ | cargo-audit / deny.toml なし。unsafe 監査は手動 | Missing |
| R2.5 検証コマンド | cargo build/test/clippy 利用可能 | **GUI クレートのため一部挙動はテスト不能**（examples は手動検証）。ベースライン確立要 | Constraint |
| R3 サブエージェント委譲 | kiro-impl の自律モード実績 | セル粒度の上限規定（コンテキスト制約）の定式化 | Missing |
| R4 サイクル安全機構 | git、kiro-verify-completion | 巻き戻しプロトコル（git reset/restore の安全手順）の規定 | Missing |
| R5 挙動非破壊 | 既存テスト群が回帰検知器 | テスト空白地帯では回帰検知不能 → **テスト追加を他観点より先行させる順序制約** | Constraint |
| R6 レポート | なし | レポート様式・集約方式が未定義 | Missing |
| R7 言語非依存 | なし | 抽象スロット定義（領域発見・検証コマンド・粒度判定）の設計が本丸 | Missing |

### 複雑性シグナル
- 本仕様は「コード変更」ではなく「**ワークフロー定義 + 大規模オーケストレーション実行**」。複雑性はアルゴリズムではなくプロセス設計と完走保証にある。
- GUI/COM 層（wintf graphics, com/）は自動テストで挙動保証しづらい。**「検証コマンド成功 = 挙動非破壊」が成立しない領域がある**ことを設計で明示すべき（unsafe 変更は especially 保守的に）。

## 3. レビュー領域分解案（マトリクスの行）

実測LOCに基づく分解。目標: 各領域のレビュー対象ソース ≤ 約2,500 LOC（サブエージェント単独完遂可能な粒度、R1.3/R3.3）。

| # | 領域 | 対象 | 対象LOC | 特記事項 |
|---|------|------|--------|---------|
| A1 | areka 全体 | crates/areka | 399 | テストゼロ。panic! 1 |
| D1 | dola ランタイム | runtime/, storyboard, transition, playback, value, variable, easing | ~3,950 | unwrap 53。テスト最厚。**やや大きい→設計で2分割検討**（runtime/前半・後半） |
| D2 | dola コンパイル&DSL | compile/, builder, error | ~1,262 | in-sourceテストなし |
| D3 | dola 検証&Cue | validate/, cue/, document, lib | ~1,358 | validate/ 583 LOC 未テスト |
| W1 | wintf レガシー&プロセス | win_message_handler, win_thread_mgr, winproc, win_state, win_style, process_singleton, api | ~2,475 | 非推奨 1,838 LOC 含む。削除可否は挙動非破壊原則と要相談 |
| W2 | wintf COM層 | com/（d2d, dcomp, dwrite, animation, wic, ulw, d3d11, dxgi） | ~2,356 | unsafe 最密集（~130）。挙動保証は型レベル+ビルドのみ |
| W3 | wintf graphics | ecs/graphics/ | ~4,206 | unsafe 57 + expect 27。**大きい→設計で2分割検討**（compositor系 / resource系） |
| W4 | wintf layout | ecs/layout/ | ~4,765 | unwrap 20。**大きい→設計で2分割検討**（taffy/arrangement系 / hit_test系） |
| W5 | wintf widget | ecs/widget/ | ~3,353 | unsafe 34。**分割検討**（テキスト系 / 図形・画像系 等） |
| W6 | wintf 入力 | ecs/pointer/, ecs/drag/ | ~3,237 | テストあり（薄め） |
| W7 | wintf ウィンドウ&ECS基盤 | ecs/window/, ecs/window_proc/, ecs/common/, ecs/world/, ecs/app | ~5,403 | window/world 未テスト。**分割検討**（window+window_proc / common+world+app） |
| W8 | wintf cue&dola統合 | ecs/cue/, ecs/dola/ | ~1,518 | in-sourceテストゼロ |
| X1 | 横断設定 | ルートCargo.toml, 各crate Cargo.toml, .gitignore, .gitmodules, .vscode/, CI欠落の扱い | n/a | submodule 初期化ガード、古い launch.json 等 |

- 計 13 領域（分割検討が全部採用されれば最大 17 領域）× 観点列（テスト網羅性 / シンプル化 / 脆弱性 / 非破壊確認は各セル内ゲート）。
- 行の最終確定・大領域の分割判断は設計フェーズで行う（要件 R1.3 の粒度判定基準を適用）。

## 4. 実装アプローチ選択肢

### Option A: 既存 kiro-impl 流用（仕様 = tasks.md のみで駆動）
通常の機能specと同様に tasks.md にマトリクスのセルを列挙し、既存の `/kiro-impl` 自律モード（タスク毎サブエージェント + レビュー + 検証）に乗せる。
- ✅ 既存インフラをそのまま活用、追加設計最小
- ✅ レビュー・デバッグ・完了検証プロトコルが既に統合済み
- ❌ R7（言語非依存・移植性）が tasks.md に埋没し、別プロジェクトへコピーする「普遍手順」が成果物として残らない
- ❌ 巻き戻しプロトコル・レポート集約など本仕様固有の安全機構は kiro-impl 標準にない

### Option B: 専用ワークフロー文書の新設（普遍手順書 + 差し込み設定の2層成果物）
design.md に「普遍レビュー・ループ手順」（言語非依存）と「プロジェクト固有設定スロット」（Rust/cargo差し込み）を明確に分離して定義し、tasks.md はそのインスタンス化として生成する。
- ✅ R7 を直接満たす（手順書をコピーし設定だけ差し替えれば他プロジェクトで再現可能）
- ✅ 巻き戻し・サイクルコミット・レポート様式を一級の設計対象にできる
- ❌ 設計文書が重くなる
- ❌ 実行系（kiro-impl）との整合を設計で明示する手間

### Option C: ハイブリッド（B の2層設計 + A の実行基盤）★推奨
design.md で「普遍手順 + 抽象スロット + 本リポジトリ差し込み設定」を定義し、実行は kiro-impl 互換のタスク構造（マトリクスのセル = タスク）に落とす。サイクル安全機構（コミット/巻き戻し/レポート断片）は各タスクの完了条件・実装指示テンプレートとして tasks.md に織り込む。
- ✅ R1〜R7 全要件を満たしつつ既存実行基盤を再利用
- ✅ 普遍手順書が独立成果物として残り、移植は設定差し替えのみ
- ❌ 設計でテンプレート（セル実行プロトコル）の精密な定義が必要

## 5. 工数・リスク評価

| 項目 | 評価 | 根拠 |
|------|------|------|
| 工数 | **XL (2+ weeks 相当)** | 13〜17領域 × 3実行観点 ≈ 40〜50セル。1セル = 調査+改善+検証+コミットの完結サイクル |
| リスク | **Medium** | 技術は既知（Rust/cargo/git）。リスク源は (1) GUI/unsafe 領域の挙動保証不能性、(2) テスト空白地帯での回帰検知不能、(3) 長時間実行の完走管理。いずれも設計の順序制約と保守性ルールで緩和可能 |

リスク緩和の鍵:
1. **順序制約**: 各領域で「テスト網羅性（回帰検知器の整備）→ シンプル化 → 脆弱性」の列順実行を必須化（R5 成立の前提）
2. **unsafe 保守則**: COM/graphics の unsafe 変更は「テストで保護されない限り簡素化対象から除外」等の保守的ルールを設計に明記
3. **ベースライン先行**: 全セル開始前に検証コマンドのグリーン状態（ベースライン）を確立・コミット

## 6. 設計フェーズへの申し送り（Research Needed）

1. **テストカバレッジ計測手段**: cargo-llvm-cov / tarpaulin の導入可否（導入しない場合は静的な「モジュール×テスト対応表」方式で代替）
2. **脆弱性スキャナ**: cargo-audit / cargo-deny の導入可否と、オフライン環境での代替手順
3. **GUI領域の非破壊確認**: examples による手動確認をループにどう位置づけるか（自動ゲート外の記録事項とするか）
4. **非推奨モジュール（1,838 LOC）の扱い**: 削除は挙動変更か否かの判定基準（公開APIだが deprecated。利用箇所ゼロ確認なら削除可能か → 慎重判断ルール要）
5. **大領域（D1/W3/W4/W5/W7）の最終分割**: 設計時に各モジュール内部構造を確認して確定
6. **巻き戻しプロトコル詳細**: `git reset --hard` vs `git restore` の使い分け、ワークツリー汚染検知手順
7. **検証コマンドの実測時間**: テストベースライン（本文書末尾の追記）を踏まえたセル当たり検証コストの見積もり
8. **抽象スロットの形式**: プロジェクト固有設定（領域発見/検証コマンド/粒度判定）を design.md 内の表とするか、独立した設定セクションとするか

## 7. 推奨

**Option C（ハイブリッド）を推奨。** 普遍手順 + 抽象スロットの2層設計が R7 の本丸であり、実行は実績ある kiro-impl 互換タスク構造に乗せるのが完走保証（R4.5）への最短経路。設計フェーズでは (a) セル実行プロトコル・テンプレート、(b) 領域分解の最終確定、(c) サイクル安全機構の git 手順、(d) レポート集約様式の4点を中心に詰める。

---

## 追記: 検証コマンドベースライン実測（2026-06-10）

`vendors/pasta` submodule 初期化後、`cargo test --workspace` を実行した結果:

| スイート | 結果 |
|---------|------|
| areka（unit） | 0 tests（テストなし） |
| dola（unit + 統合6スイート） | 計 ~360 passed / 0 failed |
| wintf（統合スイート群） | 計 ~283 passed / **初回1 failed**（tests/ecs） |

**フレーキーテストの発見**:
- 初回のワークスペース全体実行で `wintf --test ecs` が 78 passed / 1 failed。
- 同スイート単独再実行は **6回連続で 79 passed / 0 failed**（うち5回は連続実行で確認）。
- 結論: ワークスペース並列実行時の負荷でのみ稀に失敗する**タイミング依存のフレーキーテスト**が tests/ecs に存在する（タイムアウト系テスト、例: `tracker_timeout` が有力容疑）。

**設計への影響（Research Needed 追加）**:
9. **検証ゲートの再試行ポリシー**: フレーキーな失敗と真の回帰を区別する手順（例: 失敗時は当該スイートを単独で最大N回再実行し、安定再現する場合のみ回帰と判定）を普遍手順に組み込む。あわせて、フレーキーテスト自体の安定化を W8（cue&dola統合）またはテスト網羅性観点の改善対象として扱う。
10. **環境準備ゲート**: submodule 初期化確認（`git submodule update --init --recursive`）を全セル実行前の環境準備ステップとして必須化する（未初期化だと全ビルドが失敗するため）。

**セル当たり検証コスト見積もり**: フルビルド後のテスト実行自体は数秒以内（各スイート 0.0〜0.5s）。支配的コストは初回コンパイル（数分規模）であり、以降のインクリメンタル検証は軽量。検証コマンドをセルごとに全量実行しても完走可能と判断できる。

---

## 追記: 要件ディスカッションで確定した設計判断項目（2026-06-10）

`kiro-requirements-discussion` による要件精査の結果、以下を**設計フェーズ（`/kiro-spec-design`）で解決する設計判断（カテゴリB）**として確定。§6 の Research Needed（1〜8）および本文書の検証ベースライン追記（9〜10）に加え、以下2項目を補完する。

11. **改善内容レポートの様式・集約方式**: R6 が要求するレポートの構造（レビュー領域 × レビュー観点のセル単位での実施結果・巻き戻し記録・例外根拠・保留提案）を、どの粒度・どのファイル形式で集約するか。セルごとに断片を出力してメインが集約する方式か、最終一括生成か。
12. **セル実行プロトコル雛形**: マトリクスの1セルをサブエージェントが完遂するための標準実行手順テンプレート（調査→改善→自己レビュー→検証→コミット／巻き戻し）の定義。kiro-review・kiro-debug・kiro-verify-completion の各スキルをどう組み込むか。R3（委譲）・R4（サイクル安全機構）の実装中核。

なお、要件ディスカッションのカテゴリC（開発者判断）で確定した方針は requirements.md 本体へ反映する（本文書ではなく要件側が正）。

---

## 設計フェーズ: ディスカバリーと設計判断（2026-06-10）

### Discovery Scope
New Feature（プロセス仕様・フルディスカバリー）。Web調査は不要（対象は内部オーケストレーション設計）。並列サブエージェント2系統で実施: (1) 大領域5クラスタの最終分割調査、(2) 既存実行基盤スキル群の実行モデル調査。

### Research Log

#### 大領域の最終分割（Research Needed #5 の解決）
- **Context**: D1/W3/W4/W5/W7 が粒度上限（約2,600 LOC）超過
- **Findings**: 5クラスタとも結合の弱いモジュール境界で2分割可能。実測でプロダクションコード 1,290〜2,700 LOC の10領域に分割（テストファイル除外後の再計測で当初推計より小さい領域もあり）
- **Implications**: 最終マトリクスは **19領域 × 3観点 = 57セル + フェーズタスク**。W6（pointer 1,831 + drag 1,406 = 3,237）も上限超過のため W6a/W6b に分割

#### kiro-impl 実行モデルとの統合点（Research Needed #12 の解決）
- **Findings**: kiro-impl は (a) タスク毎サブエージェント、(b) kiro-review ゲート（差し戻し最大2回）、(c) kiro-debug エスカレーション（最大2回）、(d) タスク毎選択コミット、(e) 失敗時 `_Blocked:_` 記録して次タスク継続——を既に提供
- **Implications**: R3/R4 の大半は kiro-impl 再利用で充足。**差分は4点のみ**: 巻き戻しプロトコル、フレーキー判定付き検証、レポート断片集約、観点順序強制。design.md はこの差分だけを新規定義した

### Architecture Pattern Evaluation

| Option | 説明 | 強み | リスク | 採否 |
|--------|------|------|--------|------|
| kiro-impl 拡張（オーケストレーター・ワーカー） | 既存自律モードにセルブリーフ・巻き戻し・断片集約を上乗せ | 実績ある実行基盤、R4.5 の継続実行が既製 | kiro-impl 仕様変更に追随が必要（Revalidation Trigger に登録） | **採用** |
| 専用ループランナー新設 | レビュー専用のオーケストレーションを別スキルとして実装 | 自由度最大 | 二重保守、レビューゲート・デバッグ統合の再発明 | 却下 |
| Workflow（スクリプト駆動）一括実行 | 全セルをワークフロースクリプトで並列発射 | 並列性最大 | 巻き戻しの直列性と衝突、git 状態の競合管理が複雑化 | 却下（領域間並列は kiro-impl の (P) 範囲で限定的に許容） |

### Design Decisions

#### Decision: 観点列を増やさない（R2.6 の判断）
- **Alternatives**: 静的解析列の追加 / 依存監査列の追加 / 現状3列維持
- **Selected**: 3列維持。clippy はスロット S3 として検証ステップへ、依存監査は X1 領域の V 観点へ内包
- **Rationale**: 列追加はセル数を19件単位で増やし完走保証（R4.5）を圧迫する。既存セルへの内包で同じ効果を得られる
- **Trade-offs**: lint 専任セルがないため clippy 所見の網羅性は劣るが、警告は記録されレポートに残る

#### Decision: カバレッジ計測ツールを導入しない（Research Needed #1 の解決）
- **Alternatives**: cargo-llvm-cov 導入 / tarpaulin 導入 / 静的なモジュール×テスト対応分析
- **Selected**: 静的分析（T 観点セル内で「モジュール×テスト対応表」を作成し空白を特定）
- **Rationale**: ツール導入は環境変更を伴い、GUI/COM コードでは計測自体が不安定。ギャップ分析で空白地帯は既に実測済みであり、セル内の対応表作成で十分
- **Trade-offs**: 行カバレッジの数値は得られないが、本仕様の目的（空白の発見と充足）には影響しない

#### Decision: 巻き戻しは git restore + clean、reset --hard は使わない（Research Needed #6 の解決）
- **Selected**: `git restore --staged . && git restore . && git clean -fd {領域パス}`。clean の範囲を領域パスに限定し、他セルの未コミット成果物への波及を防ぐ
- **Rationale**: reset --hard はブランチ参照を動かしうる。restore + 範囲限定 clean はワークツリー復元のみを行い安全
- **Follow-up**: 巻き戻し後に S2 再実行でベースライン復帰を確認する手順を設計に明記済み

#### Decision: フレーキー判定プロトコル（Research Needed #9 の解決）
- **Selected**: 検証失敗時、失敗スイートを隔離して最大2回再実行。安定合格かつ非再現ならフレーキーとして記録し通過。再現すれば真の回帰として kiro-debug へ
- **Rationale**: ベースライン計測で実証された負荷依存フレーキー（wintf tests/ecs）による偽の巻き戻しを防ぐ

### Synthesis Outcomes
- **Generalization**: 3観点のセル実行を単一プロトコル（調査→改善→自己レビュー→検証→コミット）に統一し、観点差は「観点別規則」のみに局所化。拡張観点（R2.6）が将来追加されても同プロトコルに載る
- **Build vs Adopt**: 実行基盤・レビュー・デバッグ・完了検証・シンプル化基準の5機能すべて既存スキルを採用。新規定義は差分4点のみ
- **Simplification**: 専用ランナー・カバレッジツール・追加観点列・レポートDB等をすべて不採用。成果物はマークダウン断片＋git ログに限定

### Risks & Mitigations（設計確定版）
- 並列セル実行中の巻き戻しが他セル作業を破壊 — 巻き戻し時は実行中セルの完了を待つ（design.md Orchestrator 制約）
- GUI/COM 領域の挙動退行をユニットテストで検知不能 — R5.5 保守則 + 最終起動テスト（S7）の二段構え
- テスト「除外」の誤判定 — 除外根拠のセル断片必須記録 + CellReviewer の敵対的検証
- kiro-impl の将来変更との乖離 — Revalidation Triggers に登録済み
