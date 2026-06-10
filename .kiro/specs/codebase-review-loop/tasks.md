# Implementation Plan

本計画は design.md「レビューマトリクス定義」のインスタンス化である。**19レビュー領域 × 3レビュー観点 = 57セル**（うち W7a / W7b / W8 の T セルは事前分割により各2サブセル化、計60セルタスク）を、フェーズ0（環境準備・ベースライン確立）と最終フェーズ（起動テスト・レポート集約）で挟んで実行する。

## 実行プロトコル（全セル共通）

- **厳密直列実行**: 全タスクを記載順に1つずつ実行する。`(P)` マーカーは一切使用しない（全セルが同一ワークツリーを共有するため、並列実行は検証独立性と巻き戻し安全性を破壊する）
- **検証コマンド（S2）**: `cargo build --workspace` → `cargo test --workspace`。各セルで改善前（事前状態確認）と改善後（非破壊確認）に実行する
- **静的解析（S3）**: `cargo clippy --workspace`。警告は記録のみとし、ブロッカーとしない
- **セル実行ゲート**: 調査 → 改善 → 自己レビュー（kiro-review、差し戻し最大2回）→ 検証（S2）→ コミット。検証が成功していない変更はコミットしない
- **フレーキー判定**: S2 失敗時、失敗したテストスイートを隔離して最大2回再実行する。「隔離実行で安定して合格し、失敗が再現せず、かつ失敗テストが当該セルの `_Boundary:_` パス外」の場合のみフレーキーとして記録し通過する。境界内の失敗は再現性によらず回帰として kiro-debug へ渡す
- **巻き戻し**: kiro-debug が BLOCK_TASK または2ラウンド失敗を返した場合、`git restore --staged . && git restore . && git clean -fd {領域パス}` で直近正常コミットへ復元し、S2 再実行でベースライン復帰を確認後、巻き戻しの事実・セルID・理由をセル断片に記録して次セルへ進む（全体実行は中断しない）
- **実行時分割（NEEDS_SPLIT）**: サブエージェントが単独完遂不能と判定した場合は分割案（単独完遂可能なファイル部分集合）を返し、オーケストレーターがサブセル化して tasks.md を更新し直列キューに挿入して再委譲する。分割は1セルにつき1回まで（サブセルの再分割は不可）。それでも完遂不能な場合は部分完遂を受け入れ、未達範囲をセル断片に記録して次へ進む
- **記録**: 各セルは `report/cells/{cell-id}.md` 断片を必ず作成する（変更なし no-change・巻き戻し rolled-back の場合も docs コミットとして記録）。保留改善（挙動変更を要する対策・ロジック変更を要する簡素化・削除実証不能な非推奨コード等）は `report/proposals.md` へ提案様式で追記する
- **コミット規約（S10）**: `{type}({area-id}): {summary}` + 本文に `Task: {cell-id} in Spec: codebase-review-loop`
- **共通変更制約**: 外部観測可能な挙動を変更しない / 新機能追加・意図的挙動変更・大規模アーキテクチャ再設計をしない / `vendors/` 配下・`target/`・外部依存を変更しない / `.kiro/specs/` 配下の機能spec文書を変更しない
- **境界の注**: `.kiro/specs/codebase-review-loop/report/` 配下への記録、本書チェックボックスの更新、`report/proposals.md` への追記は全セルの境界内とみなす。T セルのテスト追加先（対象ソース内 `#[cfg(test)]` および対応クレートの `tests/` 該当ドメイン、S9 命名規約準拠）も境界内とする
- **セル共通完了条件**: 改善前後の S2 がグリーン（既存テスト失敗ゼロ、新規テスト追加分の増加のみ許容）であり、セル断片 `report/cells/{cell-id}.md` が作成され、変更が S10 形式のコミットとして記録されている（変更なしの場合は no-change 断片の docs コミットのみ）

## 実行記録（フェーズ0で確定し追記する）

- ベースラインコミット: （タスク 1.5 で記録）
- S7 初期化完了ログ文字列・タイムアウト: （タスク 1.3 で記録。タイムアウト既定60秒）
- 既知フレーキースイート一覧: （タスク 1.2 で記録。参考: research.md 実測では wintf `tests/ecs` がワークスペース並列実行時のみ稀に失敗。`tracker_timeout` が有力容疑）

## Tasks

- [ ] 1. フェーズ0: 環境準備とベースライン確立
- [ ] 1.1 環境ゲート（S8）と検証コマンドのグリーン確認
  - `git submodule update --init --recursive` を実行し、`vendors/pasta` サブモジュールが初期化済みであることを確認する（未初期化だとワークスペース全体がビルド不能）
  - S2（`cargo build --workspace` → `cargo test --workspace`）を全量実行し、失敗ゼロで完走することを確認する
  - S8 を満たせない場合はループ全体を停止する（本計画で唯一の全体停止条件）
  - 完了条件: S2 が失敗ゼロで完走した実行ログが確認できる
  - _Requirements: 2.5_

- [ ] 1.2 既知フレーキースイート一覧の記録
  - ワークスペース全体のテストを複数回（最低3回）実行し、不安定に失敗するテストスイートを特定する
  - 特定結果（ゼロ件の場合もその旨）を本書「実行記録」節へ追記する（以降のフレーキー判定の参照情報とする）
  - 完了条件: 「実行記録」節に既知フレーキースイート一覧が記録されている
  - _Requirements: 2.5_

- [ ] 1.3 起動テスト（S7）の初期化完了ログ文字列の確認
  - `RUST_LOG=info` を設定して areka アプリケーションを起動し、初期化完了を示すログ行を特定する
  - 特定した文字列と判定タイムアウト（既定60秒）を本書「実行記録」節へ追記する
  - プロセスは確認後に終了させる（パニック・error レベルログ・異常終了コードがないことを確認）
  - 完了条件: 「実行記録」節に S7 の合否判定基準（初期化完了ログ文字列・タイムアウト）が記録されている
  - _Requirements: 4.6_

- [ ] 1.4 マトリクス網羅性と実行プロトコルの確認記録
  - 本書のセルタスクを design.md の領域表（19領域）×観点（T/S/V）と突き合わせ、全57セルがタスク化され漏れがないことを「マトリクス網羅性記録」節の表で確認する
  - 除外領域（`vendors/` 配下・`target/`・外部依存）がいずれの領域にも含まれないこと、横断設定が独立領域 X1 として存在することを確認する
  - 拡張観点（R2.6）は設計判断により列追加なし（clippy は S3 として検証ステップへ、依存監査は X1-V へ内包）であることを確認・記録する
  - 各セルが粒度上限（S4: 約2,600行）以下でサブエージェント単独完遂可能であること、テスト空白の大領域（W7a/W7b/W8）の T セルが事前分割済みであることを確認する
  - 厳密直列の委譲・オーケストレーター専任（セル作業詳細をメインコンテキストへ展開しない）・巻き戻し・記録・継続実行のプロトコル（本書冒頭）が design.md と一致することを確認する
  - プロファイルスロット S1〜S10 が design.md の定義どおり解決可能であり、普遍手順がスロット名のみを参照している（特定言語・ビルドシステムへの直接参照がない）ことを確認する
  - 完了条件: 「マトリクス網羅性記録」節の表と本タスクの確認結果がセル断片 `report/cells/phase0-matrix.md` に記録されている
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.6, 3.1, 3.2, 3.3, 4.2, 4.3, 4.5, 5.4, 7.1, 7.2, 7.3, 7.4_

- [ ] 1.5 レポート骨格の作成とベースラインコミットの確定
  - `report/cells/` ディレクトリと `report/proposals.md`（提案様式のヘッダのみ）を作成する
  - ワークツリーがクリーンであることを確認し、フェーズ0の全記録（実行記録節・網羅性記録・レポート骨格）をコミットする
  - コミット後、そのハッシュを「実行記録」節へベースラインコミットとして追記し、追記を docs コミットとして確定する（以降の巻き戻し基準は常に「直近の正常コミット」）
  - 完了条件: 「実行記録」節にベースラインコミットハッシュが記録され、`git log` で確認できる
  - _Requirements: 4.4_

- [ ] 2. A1: areka エントリポイント
- [ ] 2.1 A1-T: テスト網羅性の調査と改善
  - モジュール×テスト対応を調査する（現状テストゼロ・399 LOC）。GUI 非依存に検証可能な純粋ロジック（設定組み立て・初期化順序の前提条件等）を特定しテストを追加する
  - GUI/COM 依存でテスト化できない箇所は深掘り解析のうえ所見を断片に記録し、必要に応じて `report/proposals.md` へ提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `A1-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/areka/src/_

- [ ] 2.2 A1-S: シンプル化の検証と適用
  - S6（karpathy-guidelines）基準でエントリポイントの簡素化候補を検証し、挙動を変えない簡素化を適用する
  - テスト保護のない箇所のロジック変更を要する簡素化は適用せず `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `A1-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/areka/src/_

- [ ] 2.3 A1-V: 脆弱性レビューと非破壊対策
  - panic! 経路（実測1箇所）による DoS 可能性、外部入力（起動引数・環境変数・ファイルパス）の検証欠如、リソースリークを点検する
  - 挙動を変えない対策（内部チェック追加・debug_assert 等）のみ投入し、挙動変更を要する対策は `report/proposals.md` へ根拠付きで記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `A1-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/areka/src/_

- [ ] 3. D1a: dola ランタイム中核
- [ ] 3.1 D1a-T: テスト網羅性の調査と改善
  - ランタイムファサード・ループ制御・タイムライン管理・購読管理・再生状態のモジュール×テスト対応表を作成し、テスト空白を特定して S9 準拠でテストを追加する
  - 不要テスト（重複・死テスト）は根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D1a-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/dola/src/runtime/{facade,loop_controller}.rs, crates/dola/src/runtime/{timeline_manager,subscription_manager}/, crates/dola/src/playback.rs, crates/dola/tests/（該当ドメイン）_

- [ ] 3.2 D1a-S: シンプル化の検証と適用
  - S6 基準で簡素化候補を検証し、挙動を変えない簡素化を適用する（unwrap 多数域のため変更は T セルで整備した回帰検知器の保護下で行う）
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D1a-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/dola/src/runtime/{facade,loop_controller}.rs, crates/dola/src/runtime/{timeline_manager,subscription_manager}/, crates/dola/src/playback.rs_

- [ ] 3.3 D1a-V: 脆弱性レビューと非破壊対策
  - unwrap/expect 多数域の panic 経路、整数変換の切り捨て・オーバーフロー、時刻計算の境界条件を点検する
  - 挙動を変えない対策のみ投入し、エラー応答や API シグネチャを変える対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D1a-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/dola/src/runtime/{facade,loop_controller}.rs, crates/dola/src/runtime/{timeline_manager,subscription_manager}/, crates/dola/src/playback.rs_

- [ ] 4. D1b: dola 補間・状態
- [ ] 4.1 D1b-T: テスト網羅性の調査と改善
  - 補間器・競合解決・ドキュメントストア・インスタンス管理・ストーリーボード/トランジション/イージング/値/変数のモジュール×テスト対応表を作成し、空白に S9 準拠でテストを追加する
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D1b-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/dola/src/runtime/{conflict_resolver,document_store,types,clock,instance_state}.rs, crates/dola/src/runtime/{interpolator,instance_manager}/, crates/dola/src/{storyboard,transition,easing,value,variable}.rs, crates/dola/tests/（該当ドメイン）_

- [ ] 4.2 D1b-S: シンプル化の検証と適用
  - S6 基準で簡素化候補を検証し、挙動を変えない簡素化を適用する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D1b-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/dola/src/runtime/{conflict_resolver,document_store,types,clock,instance_state}.rs, crates/dola/src/runtime/{interpolator,instance_manager}/, crates/dola/src/{storyboard,transition,easing,value,variable}.rs_

- [ ] 4.3 D1b-V: 脆弱性レビューと非破壊対策
  - unwrap 多数域の panic 経路、補間計算の数値境界（NaN・無限大・ゼロ除算）、状態遷移の不変条件を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D1b-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/dola/src/runtime/{conflict_resolver,document_store,types,clock,instance_state}.rs, crates/dola/src/runtime/{interpolator,instance_manager}/, crates/dola/src/{storyboard,transition,easing,value,variable}.rs_

- [ ] 5. D2: dola コンパイル・DSL
- [ ] 5.1 D2-T: テスト網羅性の調査と改善
  - in-source テストなしの compile/ と Builder API・エラー型について、統合テストとの過不足を整理しモジュール×テスト対応表を作成、空白に S9 準拠でテストを追加する
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D2-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/dola/src/compile/, crates/dola/src/{builder,error}.rs, crates/dola/tests/（compile ドメイン）_

- [ ] 5.2 D2-S: シンプル化の検証と適用
  - S6 基準で解決・型変換ロジックと Builder API の簡素化候補を検証し、挙動を変えない簡素化を適用する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D2-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/dola/src/compile/, crates/dola/src/{builder,error}.rs_

- [ ] 5.3 D2-V: 脆弱性レビューと非破壊対策
  - 外部入力（JSON/TOML/YAML ドキュメント）のデシリアライズ境界・検証欠如・panic 経路・再帰深度を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D2-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/dola/src/compile/, crates/dola/src/{builder,error}.rs_

- [ ] 6. D3: dola 検証・Cue
- [ ] 6.1 D3-T: テスト網羅性の調査と改善
  - 未テストの validate/ を最優先に、cue/・ドキュメント定義のモジュール×テスト対応表を作成し、空白に S9 準拠でテストを追加する
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D3-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/dola/src/validate/, crates/dola/src/cue/, crates/dola/src/{document,lib}.rs, crates/dola/tests/（該当ドメイン）_

- [ ] 6.2 D3-S: シンプル化の検証と適用
  - S6 基準でバリデーションロジックと Cue モデルの簡素化候補を検証し、挙動を変えない簡素化を適用する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D3-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/dola/src/validate/, crates/dola/src/cue/, crates/dola/src/{document,lib}.rs_

- [ ] 6.3 D3-V: 脆弱性レビューと非破壊対策
  - バリデーション網羅性の欠落（不正ドキュメントの素通り）、スケジュール時刻の整数境界、panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策（検証の厳格化を含む）は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `D3-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/dola/src/validate/, crates/dola/src/cue/, crates/dola/src/{document,lib}.rs_

- [ ] 7. W1: wintf レガシー・プロセス
- [ ] 7.1 W1-T: テスト網羅性の調査と改善
  - 非・非推奨部分（win_state / win_style / process_singleton / api）を優先してモジュール×テスト対応表を作成し、空白に S9 準拠でテストを追加する
  - 非推奨3モジュール（win_message_handler / win_thread_mgr / winproc、計約1,838 LOC）は削除候補（7.2 で判定）のため新規テスト追加の対象外とし、調査所見のみ断片に記録する
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W1-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/{win_message_handler,win_thread_mgr,winproc,win_state,win_style,process_singleton,api}.rs, crates/wintf/tests/（該当ドメイン）_

- [ ] 7.2 W1-S: シンプル化と非推奨コードの実証付き削除
  - 非推奨指定モジュール（win_message_handler / win_thread_mgr / winproc）について、ワークスペース内（crates / examples / tests）での利用箇所を grep とビルド確認で実証調査する
  - 利用箇所ゼロを実証できたモジュールは削除し、S2 で挙動非破壊を確認する（最終起動テスト 21 でも再確認される）。実証できない場合は削除せず `report/proposals.md` へ削除候補として記録する
  - 残存コードに S6 基準の簡素化を適用する（本ワークスペースは `publish = false` のため後方互換性の考慮は不要）
  - 完了条件: 利用実証調査の結果（grep 範囲・結果・判定）が断片 `W1-S.md` に記録され、セル共通完了条件を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 2.9, 2.10, 4.1, 5.1, 5.3_
  - _Boundary: crates/wintf/src/{win_message_handler,win_thread_mgr,winproc,win_state,win_style,process_singleton,api}.rs_

- [ ] 7.3 W1-V: 脆弱性レビューと非破壊対策
  - プロセス単一実行制御（ミューテックス・ハンドルリーク）、Win32 API ラッパーの境界条件、panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W1-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/{win_message_handler,win_thread_mgr,winproc,win_state,win_style,process_singleton,api}.rs_

- [ ] 8. W2: wintf COM層
- [ ] 8.1 W2-T: テスト網羅性の調査と改善
  - COM ラッパー層のうちデバイス非依存に検証可能な純粋ロジック（パラメータ変換・構造体構築・定数マッピング等）を特定し、S9 準拠でテストを追加する
  - デバイス依存でテスト化できない箇所（unsafe 最密集域）は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W2-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/com/, crates/wintf/tests/（該当ドメイン）_

- [ ] 8.2 W2-S: シンプル化の検証と適用（unsafe 保守則適用）
  - S6 基準で簡素化候補を検証する。テストで保護されない unsafe/COM 部分は、命名・コメント・自明な重複除去等の構造的整理に限定し、ロジック変更を伴う簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W2-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5_
  - _Boundary: crates/wintf/src/com/_

- [ ] 8.3 W2-V: 脆弱性レビューと非破壊対策
  - unsafe ブロックの境界条件（ポインタ有効性・ライフタイム・Send/Sync 妥当性）、COM ハンドルのリーク・二重解放、整数変換の切り捨てを点検する
  - 挙動を変えない対策（debug_assert・安全性コメントの根拠明記・内部チェック追加）のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W2-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/com/_

- [ ] 9. W3a: wintf コンポジタ・描画
- [ ] 9.1 W3a-T: テスト網羅性の調査と改善
  - compositor 系・render/surface/init/clip_sync systems・components のうちデバイス非依存に検証可能なロジックを特定し、S9 準拠でテストを追加する
  - GPU 依存でテスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W3a-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/graphics/（compositor 系・render/surface/init/clip_sync systems・components）, crates/wintf/tests/（該当ドメイン）_

- [ ] 9.2 W3a-S: シンプル化の検証と適用（unsafe 保守則適用）
  - S6 基準で簡素化候補を検証する。テストで保護されない unsafe/GUI 部分は構造的整理に限定し、ロジック変更を伴う簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W3a-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5_
  - _Boundary: crates/wintf/src/ecs/graphics/（compositor 系・render/surface/init/clip_sync systems・components）_

- [ ] 9.3 W3a-V: 脆弱性レビューと非破壊対策
  - unsafe 境界・expect/unwrap の panic 経路・デバイスロスト時のリソースリーク・整数変換を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W3a-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/graphics/（compositor 系・render/surface/init/clip_sync systems・components）_

- [ ] 10. W3b: wintf グラフィックス資源
- [ ] 10.1 W3b-T: テスト網羅性の調査と改善
  - visual/visual_manager/clip/core/dcomp_resource/command_list・残り systems のうちデバイス非依存に検証可能なロジック（Visual 階層管理・世代管理等）を特定し、S9 準拠でテストを追加する
  - GPU 依存でテスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W3b-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/graphics/（visual/visual_manager/clip/core/dcomp_resource/command_list・残り systems）, crates/wintf/tests/（該当ドメイン）_

- [ ] 10.2 W3b-S: シンプル化の検証と適用（unsafe 保守則適用）
  - S6 基準で簡素化候補を検証する。テストで保護されない unsafe/GUI 部分は構造的整理に限定し、ロジック変更を伴う簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W3b-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5_
  - _Boundary: crates/wintf/src/ecs/graphics/（visual/visual_manager/clip/core/dcomp_resource/command_list・残り systems）_

- [ ] 10.3 W3b-V: 脆弱性レビューと非破壊対策
  - unsafe 境界・リソースの生成/破棄対称性・デバイスロスト再初期化経路・panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W3b-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/graphics/（visual/visual_manager/clip/core/dcomp_resource/command_list・残り systems）_

- [ ] 11. W4a: wintf taffy・配置
- [ ] 11.1 W4a-T: テスト網羅性の調査と改善
  - taffy 統合・arrangement・box_style・dimension 系のモジュール×テスト対応表を作成し、空白に S9 準拠でテストを追加する
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W4a-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/layout/（taffy/arrangement/box_style/dimension 系）, crates/wintf/tests/（該当ドメイン）_

- [ ] 11.2 W4a-S: シンプル化の検証と適用
  - S6 基準で配置計算・スタイル変換の簡素化候補を検証し、挙動を変えない簡素化を適用する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W4a-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/wintf/src/ecs/layout/（taffy/arrangement/box_style/dimension 系）_

- [ ] 11.3 W4a-V: 脆弱性レビューと非破壊対策
  - unwrap の panic 経路、レイアウト計算の数値境界（負値・NaN・オーバーフロー）を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W4a-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/layout/（taffy/arrangement/box_style/dimension 系）_

- [ ] 12. W4b: wintf ヒットテスト・計測
- [ ] 12.1 W4b-T: テスト網羅性の調査と改善
  - hit_test/hit_region/metrics/rect/monitor・window_pos systems のモジュール×テスト対応表を作成する（テスト比較的厚い領域のため過不足整理を重視）
  - 空白に S9 準拠でテストを追加し、重複・死テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W4b-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/layout/（hit_test/hit_region/metrics/rect/monitor・window_pos systems）, crates/wintf/tests/（該当ドメイン）_

- [ ] 12.2 W4b-S: シンプル化の検証と適用
  - S6 基準でヒットテスト・計測ロジックの簡素化候補を検証し、挙動を変えない簡素化を適用する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W4b-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/wintf/src/ecs/layout/（hit_test/hit_region/metrics/rect/monitor・window_pos systems）_

- [ ] 12.3 W4b-V: 脆弱性レビューと非破壊対策
  - 座標変換の整数境界・モニタ構成変更時の境界条件・panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W4b-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/layout/（hit_test/hit_region/metrics/rect/monitor・window_pos systems）_

- [ ] 13. W5a: wintf テキスト描画
- [ ] 13.1 W5a-T: テスト網羅性の調査と改善
  - テキストウィジェットのうちデバイス非依存に検証可能なロジック（レイアウトパラメータ・スタイル解決等）を特定し、S9 準拠でテストを追加する
  - DirectWrite 依存でテスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W5a-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/widget/text/, crates/wintf/tests/（該当ドメイン）_

- [ ] 13.2 W5a-S: シンプル化の検証と適用（unsafe 保守則適用）
  - S6 基準で簡素化候補を検証する。テストで保護されない unsafe/GUI 部分は構造的整理に限定し、ロジック変更を伴う簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W5a-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5_
  - _Boundary: crates/wintf/src/ecs/widget/text/_

- [ ] 13.3 W5a-V: 脆弱性レビューと非破壊対策
  - unsafe 境界・テキストリソースのリーク・外部入力（フォント名・テキスト内容）の検証・panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W5a-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/widget/text/_

- [ ] 14. W5b: wintf 図形・画像・ブラシ
- [ ] 14.1 W5b-T: テスト網羅性の調査と改善
  - 図形・画像ソース・ブラシのうちデバイス非依存に検証可能なロジックを特定し、S9 準拠でテストを追加する（bitmap_source/ の分離テストパターンを参考にする）
  - GPU/WIC 依存でテスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W5b-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/widget/{shapes,bitmap_source}/, crates/wintf/src/ecs/widget/brushes.rs, crates/wintf/tests/（該当ドメイン）_

- [ ] 14.2 W5b-S: シンプル化の検証と適用（unsafe 保守則適用）
  - S6 基準で簡素化候補を検証する。テストで保護されない unsafe/GUI 部分は構造的整理に限定し、ロジック変更を伴う簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W5b-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5_
  - _Boundary: crates/wintf/src/ecs/widget/{shapes,bitmap_source}/, crates/wintf/src/ecs/widget/brushes.rs_

- [ ] 14.3 W5b-V: 脆弱性レビューと非破壊対策
  - 外部入力（画像ファイル・パス）の検証欠如、unsafe 境界、リソースリーク、整数変換（画像寸法）を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W5b-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/widget/{shapes,bitmap_source}/, crates/wintf/src/ecs/widget/brushes.rs_

- [ ] 15. W6a: wintf ポインター入力
- [ ] 15.1 W6a-T: テスト網羅性の調査と改善
  - ポインターバッファリング・配信のモジュール×テスト対応表を作成し（テスト薄め領域）、空白に S9 準拠でテストを追加する
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W6a-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/pointer/, crates/wintf/tests/（該当ドメイン）_

- [ ] 15.2 W6a-S: シンプル化の検証と適用
  - S6 基準でイベント収集・配信ロジックの簡素化候補を検証し、挙動を変えない簡素化を適用する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W6a-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/wintf/src/ecs/pointer/_

- [ ] 15.3 W6a-V: 脆弱性レビューと非破壊対策
  - 入力イベントの境界条件（座標範囲・ボタン状態の不整合）、バッファ枯渇、panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W6a-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/pointer/_

- [ ] 16. W6b: wintf ドラッグ
- [ ] 16.1 W6b-T: テスト網羅性の調査と改善
  - ドラッグ状態遷移・ディスパッチのモジュール×テスト対応表を作成し（テスト薄め領域）、空白に S9 準拠でテストを追加する
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W6b-T.md`、S10 コミット）を満たす
  - _Requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/drag/, crates/wintf/tests/（該当ドメイン）_

- [ ] 16.2 W6b-S: シンプル化の検証と適用
  - S6 基準で状態遷移・キャプチャガードの簡素化候補を検証し、挙動を変えない簡素化を適用する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W6b-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/wintf/src/ecs/drag/_

- [ ] 16.3 W6b-V: 脆弱性レビューと非破壊対策
  - ドラッグ状態の不整合（キャプチャ解放漏れ）、座標計算の整数境界、panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W6b-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/drag/_

- [ ] 17. W7a: wintf ウィンドウ・メッセージ
- [ ] 17.1 W7a-T1: テスト網羅性の調査と改善（ウィンドウ管理）
  - 未テストの ecs/window/ についてモジュール×テスト対応表を作成し、HWND/Entity マッピング・状態同期等のテスト可能ロジックに S9 準拠でテストを追加する（テスト空白の大領域のため事前分割サブセル1/2）
  - Win32 依存でテスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7a-T1.md`、S10 コミット）を満たす
  - _Requirements: 1.3, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/window/, crates/wintf/tests/（該当ドメイン）_

- [ ] 17.2 W7a-T2: テスト網羅性の調査と改善（メッセージブリッジ）
  - ecs/window_proc/ についてモジュール×テスト対応表を作成し、メッセージ種別ごとの変換・ディスパッチロジックに S9 準拠でテストを追加する（事前分割サブセル2/2）
  - Win32 依存でテスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7a-T2.md`、S10 コミット）を満たす
  - _Requirements: 1.3, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/window_proc/, crates/wintf/tests/（該当ドメイン）_

- [ ] 17.3 W7a-S: シンプル化の検証と適用（unsafe 保守則適用）
  - S6 基準で領域全体（window/ + window_proc/）の簡素化候補を検証する。テストで保護されない unsafe/GUI 部分は構造的整理に限定し、ロジック変更を伴う簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7a-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5_
  - _Boundary: crates/wintf/src/ecs/window/, crates/wintf/src/ecs/window_proc/_

- [ ] 17.4 W7a-V: 脆弱性レビューと非破壊対策
  - unsafe 境界、HWND ライフサイクル（解放後使用・リーク）、メッセージパラメータの整数変換、マルチスレッド境界を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7a-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/window/, crates/wintf/src/ecs/window_proc/_

- [ ] 18. W7b: wintf ECS基盤・World
- [ ] 18.1 W7b-T1: テスト網羅性の調査と改善（共通インフラ）
  - ecs/common/ の階層伝播システム（ジェネリック伝播ロジック）についてモジュール×テスト対応表を作成し、空白に S9 準拠でテストを追加する（テスト空白の大領域のため事前分割サブセル1/2）
  - 不要テストは根拠を断片に記録したうえで慎重に除外する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7b-T1.md`、S10 コミット）を満たす
  - _Requirements: 1.3, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/common/, crates/wintf/tests/（該当ドメイン）_

- [ ] 18.2 W7b-T2: テスト網羅性の調査と改善（World・アプリ状態）
  - 未テストの ecs/world/（schedule labels・vsync・フレーム進行）と ecs/app.rs（ウィンドウカウント・ディスプレイ構成変更）についてモジュール×テスト対応表を作成し、テスト可能ロジックに S9 準拠でテストを追加する（事前分割サブセル2/2）
  - vsync 等の実時間依存でテスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7b-T2.md`、S10 コミット）を満たす
  - _Requirements: 1.3, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/world/, crates/wintf/src/ecs/app.rs, crates/wintf/tests/（該当ドメイン）_

- [ ] 18.3 W7b-S: シンプル化の検証と適用
  - S6 基準で領域全体（common/ + world/ + app.rs）の簡素化候補を検証し、挙動を変えない簡素化を適用する。テスト保護のない箇所はロジック変更を避け構造的整理を優先する
  - ロジック変更を要する簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7b-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: crates/wintf/src/ecs/{common,world}/, crates/wintf/src/ecs/app.rs_

- [ ] 18.4 W7b-V: 脆弱性レビューと非破壊対策
  - フレーム時間計算の数値境界、スケジュール順序の不変条件、ディスプレイ構成変更時の境界条件、panic 経路を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W7b-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/{common,world}/, crates/wintf/src/ecs/app.rs_

- [ ] 19. W8: wintf Cue・Dola統合
- [ ] 19.1 W8-T1: テスト網羅性の調査と改善（Cue 統合）
  - in-source テストゼロの ecs/cue/（CueQueue・CueSheetTracker・EntityRef ラウンドトリップ）についてモジュール×テスト対応表を作成し、S9 準拠でテストを追加する（テスト空白領域のため事前分割サブセル1/2）
  - 既知フレーキーテスト所在域（wintf `tests/ecs`、`tracker_timeout` が有力容疑）の安定化（タイミング依存の除去）を本セルの改善対象に含める
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W8-T1.md`、S10 コミット）を満たし、フレーキー安定化の実施結果（または見送り根拠）が断片に記録されている
  - _Requirements: 1.3, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/cue/, crates/wintf/tests/（ecs ドメイン）_

- [ ] 19.2 W8-T2: テスト網羅性の調査と改善（Dola 統合）
  - in-source テストゼロの ecs/dola/（DolaAnimator・tick システム・UpdateResult 消費）についてモジュール×テスト対応表を作成し、S9 準拠でテストを追加する（事前分割サブセル2/2）
  - テスト化できない箇所は深掘り解析のうえ所見と提案を記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W8-T2.md`、S10 コミット）を満たす
  - _Requirements: 1.3, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: crates/wintf/src/ecs/dola/, crates/wintf/tests/（該当ドメイン）_

- [ ] 19.3 W8-S: シンプル化の検証と適用（unsafe 保守則適用）
  - S6 基準で領域全体（cue/ + dola/）の簡素化候補を検証する。テストで保護されない unsafe 部分（`unsafe impl Send + Sync` を含む）は構造的整理に限定し、ロジック変更を伴う簡素化は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W8-S.md`、S10 コミット）を満たす
  - _Requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5_
  - _Boundary: crates/wintf/src/ecs/{cue,dola}/_

- [ ] 19.4 W8-V: 脆弱性レビューと非破壊対策
  - `unsafe impl Send + Sync` の妥当性（排他アクセス保証の根拠）、Entity ビット変換のラウンドトリップ安全性、スケジュール時刻の整数境界を点検する
  - 挙動を変えない対策のみ投入し、挙動変更を要する対策は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `W8-V.md`、S10 コミット）を満たす
  - _Requirements: 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: crates/wintf/src/ecs/{cue,dola}/_

- [ ] 20. X1: 横断プロジェクト設定
- [ ] 20.1 X1-T: テスト構成の点検と改善
  - ワークスペース設定がテスト実行へ与える構成（テストエントリポイントの束ね規約・feature 組合せのビルド/テスト可否・dev-dependencies の整合）を点検し、設定起因のテスト漏れを特定・是正する
  - 自動化できない確認事項は所見として断片に記録し、必要に応じて `report/proposals.md` へ提案を記録する（CI 欠落の事実は所見として記録。CI 新設は本ループの対象外）
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `X1-T.md`、S10 コミット。変更なし時は no-change 断片）を満たす
  - _Requirements: 1.4, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1_
  - _Boundary: ルート Cargo.toml, crates/*/Cargo.toml, .gitignore, .gitmodules, .vscode/_

- [ ] 20.2 X1-S: 設定の簡素化と整理
  - S6 基準でワークスペース設定・エディタ設定の簡素化候補を検証し、不要・古い設定を整理する（古いバイナリパス `sample_dcomp.exe` が残存する launch.json の修正を含む）
  - ビルド成果物の挙動（リリース最適化・LTO 等）を変える設定変更は適用せず `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `X1-S.md`、S10 コミット）を満たす
  - _Requirements: 1.4, 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3_
  - _Boundary: ルート Cargo.toml, crates/*/Cargo.toml, .gitignore, .gitmodules, .vscode/_

- [ ] 20.3 X1-V: 依存監査と設定の脆弱性点検
  - 依存クレートの既知脆弱性を調査する（`cargo audit` 相当の調査。ツール未導入の場合は依存一覧と公開アドバイザリの突き合わせで代替し、手順を断片に記録）
  - `.gitignore` / `.gitmodules` / ワークスペース依存固定の安全性を点検する。依存更新は挙動影響を評価のうえ慎重に適用し、挙動変更を伴う更新は `report/proposals.md` へ記録する
  - 完了条件: セル共通完了条件（前後 S2 グリーン、断片 `X1-V.md`、S10 コミット）を満たし、依存監査の調査結果が断片に記録されている
  - _Requirements: 1.4, 2.3, 2.4, 2.5, 2.7, 2.8, 4.1, 5.1, 5.2_
  - _Boundary: ルート Cargo.toml, crates/*/Cargo.toml, .gitignore, .gitmodules, .vscode/_

- [ ] 21. 最終起動テスト（S7）の実行と完全解消
  - 全セル完了後、S7 を実行する: `RUST_LOG=info` で areka を起動し、タイムアウト（実行記録節に記録した値、既定60秒）内に初期化完了ログ（実行記録節に記録した文字列）を確認してプロセスを終了する
  - パニック・error レベルログ・異常終了コードがないことを合格条件とする
  - 失敗した場合は kiro-debug により根本原因を解消してから再実行する（直近セル群のコミットを bisect 的に疑う）。合格するまで完了としない
  - 完了条件: S7 合格のエビデンス（起動ログ・終了コード）が `report/cells/final-launch.md` に記録され、解消のための修正があればコミットされている
  - _Requirements: 4.6, 4.7, 5.1_

- [ ] 22. 改善内容レポートの集約と新規仕様提案の一括整理
  - `report/cells/` の全断片を本書のセル一覧（マトリクス網羅性記録）と突き合わせ、欠落セルは no-change として補完記録のうえレポートに明記する
  - レビュー領域×レビュー観点ごとの実施結果（追加・除外したテスト、簡素化の内容、脆弱性の所見と対応）、巻き戻しが発生したセルとその理由、フレーキー判定記録を `report.md` に集約する（再実行時は全置換。断片が真実源）
  - `report/proposals.md` の提案候補を重複統合し、優先度付きの新規仕様提案として `report.md` の提案セクションに一括整理する
  - 完了条件: `.kiro/specs/codebase-review-loop/report.md` が全57セル分の実施結果を欠落なく含み、巻き戻し記録と新規仕様提案セクションを備えている
  - _Requirements: 4.3, 4.5, 6.1, 6.2, 6.3, 6.4_

## マトリクス網羅性記録（R1.6）

全19レビュー領域 × 3レビュー観点 = 57セルのタスク対応表。全セルが定義済みタスクに対応しており、漏れはない。

| 領域 | T（テスト網羅性） | S（シンプル化） | V（脆弱性） |
|------|------------------|----------------|------------|
| A1: areka エントリポイント | 2.1 | 2.2 | 2.3 |
| D1a: dola ランタイム中核 | 3.1 | 3.2 | 3.3 |
| D1b: dola 補間・状態 | 4.1 | 4.2 | 4.3 |
| D2: dola コンパイル・DSL | 5.1 | 5.2 | 5.3 |
| D3: dola 検証・Cue | 6.1 | 6.2 | 6.3 |
| W1: wintf レガシー・プロセス | 7.1 | 7.2 | 7.3 |
| W2: wintf COM層 | 8.1 | 8.2 | 8.3 |
| W3a: wintf コンポジタ・描画 | 9.1 | 9.2 | 9.3 |
| W3b: wintf グラフィックス資源 | 10.1 | 10.2 | 10.3 |
| W4a: wintf taffy・配置 | 11.1 | 11.2 | 11.3 |
| W4b: wintf ヒットテスト・計測 | 12.1 | 12.2 | 12.3 |
| W5a: wintf テキスト描画 | 13.1 | 13.2 | 13.3 |
| W5b: wintf 図形・画像・ブラシ | 14.1 | 14.2 | 14.3 |
| W6a: wintf ポインター入力 | 15.1 | 15.2 | 15.3 |
| W6b: wintf ドラッグ | 16.1 | 16.2 | 16.3 |
| W7a: wintf ウィンドウ・メッセージ | 17.1, 17.2（事前分割） | 17.3 | 17.4 |
| W7b: wintf ECS基盤・World | 18.1, 18.2（事前分割） | 18.3 | 18.4 |
| W8: wintf Cue・Dola統合 | 19.1, 19.2（事前分割） | 19.3 | 19.4 |
| X1: 横断プロジェクト設定 | 20.1 | 20.2 | 20.3 |

**網羅性の注記**:
- 観点の実行順は各領域内で T → S → V に固定（R2.7）。改善前後の検証（R2.5）と自己レビュー（R4.1）は全セル共通のセル内ゲートとして実行する
- `vendors/` 配下・`target/`・外部依存はスロット S5 によりいずれの領域にも含まれない（R1.5）
- 拡張観点（R2.6）は設計判断により列追加なし: clippy は S3 として検証ステップに統合、依存監査は X1-V（20.3）に内包
- 全セルを厳密直列で実行するため `(P)` マーカーは付与しない（design.md「タスク構造への写像」）
- R7.1〜7.4（言語非依存・移植性）は design.md の2層構造（普遍手順層＋プロジェクト・プロファイル層）として実現済みであり、本計画ではタスク 1.4 でスロット解決可能性を確認する
