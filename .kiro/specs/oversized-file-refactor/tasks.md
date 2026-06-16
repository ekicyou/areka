# Implementation Plan

> **共通完了条件（全分割タスクに適用）**: 各タスク完了時、(1) 対象モジュールが責務境界で分割され各≤~600行（凝集を保てば最大~650行）、(2) 当該クレートの `cargo test` がグリーン（Windows/DirectComposition）、(3) テスト名インベントリのゴールデン差分が空（追加・消失・リネーム・重複ゼロ）、(4) 公開APIのパス・可視性が不変（呼び出し側コード無改変）。red が出た場合はグリーンへ回復するまで是正、解消不能なら当該変更を revert。
>
> **共有 common 所有権ルール（テスト分割タスクに適用）**: 共有 `common/mod.rs` を新設・追記する場合、その common ファイルは**単一タスクが所有・書き込み**し、他タスクは読み取りのみ（並行書き込み禁止）。新設が必要なら先行タスク化し後続を `_Depends_` で直列化する。

- [x] 1. Foundation: 検証ベースラインとガードレール
- [x] 1.1 挙動非破壊ゴールデンの取得とクリーンビルド確認
  - 全3クレート（wintf, dola, areka）のテスト名インベントリを「同一性ゴールデン」として取得・保存する
  - Windows（DirectComposition対応）環境で `cargo build` 全体グリーン、`cargo test` 全クレートグリーンを確認する
  - 死体削除フェーズで消えるテストが無いこと（R1の削除対象はすべて非テスト項目）を前提として明記する
  - 観測: 3クレート分のテスト名一覧が基準ファイルとして存在し、ワークスペース全体がビルド成功・全テストグリーン
  - _Requirements: 3.5, 4.3, 4.5_

- [x] 2. Phase 1: wintf 死体コード削除
- [x] 2.1 確定リストの死体コード削除
  - mouse_* deprecated エイリアス群と mouse モジュール、opacity deprecated static を削除する
  - 参照ゼロの死体 example（旧実装）を削除する
  - 各削除の直前に grep で実利用ゼロを再検証し、参照が判明した項目は削除せず参照状況を報告対象として残す
  - 保護対象（deprecated 3ファイルおよび facade の後方互換メソッド）は削除対象に含めない
  - 観測: 削除対象が `#[cfg(test)]` テストを一切含まないことを確認し、削除後のテスト名インベントリがゴールデンと完全一致（差分が空）、かつ `cargo test -p wintf` グリーン
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.10_
- [x] 2.2 確定リスト外の死体コード掃き出し
  - 作業中に発見した確定リスト外の死体を grep で実参照ゼロと検証できたもののみ削除する
  - 保護対象（deprecated 3ファイル・facade の後方互換メソッド）は除外する
  - 観測: 追加削除した項目とスキップした項目の一覧が報告として記録され、`cargo test -p wintf` グリーン、インベントリ差分が空
  - _Requirements: 1.9, 1.10_

- [x] 3. wintf wave: 製品ソース分割
- [x] 3.1 (P) drag 状態モジュールの分割
  - in-source テスト抽出を第一手段とし、本体が目安を超える場合のみ責務 seam で追加分割する
  - 観測: 共通完了条件を満たす（状態モジュールが分割され `cargo test -p wintf` グリーン・インベントリ差分が空・公開API不変）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / drag state_
- [x] 3.2 (P) compositor render systems モジュールの分割（最高リスク・Pattern B）
  - 唯一のテスト無しファイルで Pattern B（本体の責務分割）を適用する。RAIIガードと再帰走査の凝集を保持し、必要なら3分割を許容する
  - private 項目を sub-module 跨ぎで共有する箇所は `pub(super)` / `pub(crate)` へ最小昇格する
  - 観測: 昇格した項目を列挙して可視性マッピングを明示し、外部可視性・パスが不変であること、`cargo test -p wintf` グリーン、各モジュール≤~600行
  - _Requirements: 2.1, 2.2, 2.4, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / compositor render systems_
- [x] 3.3 (P) cue queue モジュールの分割
  - 観測: 共通完了条件を満たす
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / cue queue_
- [x] 3.4 (P) window position モジュールの分割
  - 観測: 共通完了条件を満たす
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / window position_
- [x] 3.5 (P) pointer types モジュールの分割
  - 死体削除（2.1）で deprecated エイリアスが除去された後の状態を分割する
  - 観測: 共通完了条件を満たす
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / pointer types_
  - _Depends: 2.1_
- [x] 3.6 (P) typewriter ウィジェットモジュールの分割
  - 観測: 共通完了条件を満たす
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / typewriter widget_
- [x] 3.7 (P) hit-region in-source テストモジュールの分割
  - structure.md のディレクトリモジュール化パターンに従い、テスト群を複数 sub-file へ分離する
  - 観測: 同一テストが実行され（インベントリ差分が空）、`cargo test -p wintf` グリーン、各≤~600行
  - _Requirements: 2.1, 2.2, 2.4, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / hit-region tests_
- [x] 3.8 (P) hit-test 拡張 in-source テストモジュールの分割
  - 観測: 同一テストが実行され（インベントリ差分が空）、`cargo test -p wintf` グリーン、各≤~600行
  - _Requirements: 2.1, 2.2, 2.4, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / hit-test tests_

- [ ] 4. wintf wave: 統合テスト分割
- [x] 4.1 (P) taffy advanced レイアウトテストの分割
  - テスト群別に sub-file へ分割し、ドメイン入口に `#[path] mod` 宣言を追加・旧宣言を削除する。共有 setup が生じる場合は common 所有権ルールに従う
  - 観測: 既存テストケースの内容・アサーション不変、同一テストが実行（インベントリ差分が空）、`cargo test -p wintf` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / wintf layout tests_
- [ ] 4.2 (P) boxstyle 座標分離テストの分割
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p wintf` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / wintf layout tests_
- [ ] 4.3 (P) taffy layout 統合テストの分割
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p wintf` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / wintf layout tests_
- [ ] 4.4 (P) arrangement bounds テストの分割
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p wintf` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / wintf layout tests_

- [ ] 5. dola wave: 製品ソース分割
- [ ] 5.1 loop controller モジュールの分割
  - in-source テスト抽出を第一手段とする
  - 観測: 共通完了条件を満たす（`cargo test -p dola` グリーン・インベントリ差分が空・公開API不変）
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / loop controller_

- [ ] 6. dola wave: 統合テスト分割
- [ ] 6.1 (P) conflict resolution テストの分割
  - 共有フィクスチャを runtime 用 common へ新設する場合は**本タスクが所有・書き込み**する
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola runtime tests_
- [ ] 6.2 (P) time resolution テストの分割
  - 既存 compile 用 common は読み取りのみ
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola compile tests_
- [ ] 6.3 (P) facade テストの分割
  - runtime 用 common を 6.1 と共有するため、common 新設は 6.1 に委ねる
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola runtime tests_
  - _Depends: 6.1_
- [ ] 6.4 (P) loop offset テストの分割
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola runtime tests_
- [ ] 6.5 (P) general integration テストの分割
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola general tests_
- [ ] 6.6 (P) transition validation テストの分割
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola validation tests_
- [ ] 6.7 (P) core types テストの分割
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola general tests_
- [ ] 6.8 (P) compile integration テストの分割
  - compile 用 common へトリガーヘルパー等を追記する場合は**本タスクが単独所有**する
  - 観測: 同一テストが実行（インベントリ差分が空）、`cargo test -p dola` グリーン、各≤~600行
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_
  - _Boundary: Integration Test Splitter / dola compile tests_

- [ ] 7. areka wave: 製品ソース分割
- [ ] 7.1 アプリケーション エントリ（main）モジュールの分割
  - in-source テストを抽出し、エントリ本体とテストを分離する（バイナリクレートの可視性は子モジュールが親の private を参照可能なため昇格不要）
  - 観測: 本体とテストが分離して各≤~600行、`cargo test -p areka` グリーン、インベントリ差分が空、バイナリ挙動不変
  - _Requirements: 2.1, 2.5, 2.6, 2.7, 4.1, 4.2, 4.4_
  - _Boundary: Module Splitter / application entry_

- [ ] 8. 最終検証とスコープガードレール確認
- [ ] 8.1 ワークスペース全体検証
  - `cargo build` 全体（examples 含む）がグリーンであることを確認し、deprecated 3ファイルの現役参照の健全性と公開APIの後方互換を実証する
  - examples と deprecated 3ファイルが本仕様を通じて無改変であることを git diff で確認する
  - 全クレートの `cargo test` グリーン、および全クレートのテスト名インベントリがゴールデンと完全一致（差分が空）であることを確認する
  - 観測: ワークスペース全体ビルド・全テストグリーン、スコープ外ファイルの無改変が diff で確認され、全インベントリ差分が空
  - _Requirements: 4.1, 4.3, 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 6.4_
  - _Depends: 2.1, 2.2, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 4.1, 4.2, 4.3, 4.4, 5.1, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 7.1_

## Implementation Notes

- **環境セットアップ**: worktreeでは git submodule `vendors/pasta` が未初期化。`git submodule update --init --recursive` を実行済み（`pasta_core` 依存解決に必須）。
- **ゴールデン検証プロトコル（全分割タスク共通）**: ゴールデンは `.kiro/specs/oversized-file-refactor/golden/` に保存。
  - **権威ある不変量 = リーフ名多重集合** (`{crate}.leaf.txt`): 各テスト名から最終 `::` 以降の関数名のみを抽出・sort した多重集合。分割でモジュールパスのネストが深くなっても不変ゆえ、全パターンで差分ゼロを要求できる。検証コマンド: `cargo test -p {crate} --all-targets -- --list 2>/dev/null | grep ': test$' | sed -E 's/^.*:://; s/: test$//' | sort` を golden と `diff` し、差分が空であること。
  - **補助的厳格チェック = フルパス版** (`{crate}.txt`): Pattern A（in-source `mod tests` のファイル化、モジュール名 `tests` 維持）と Pattern B（テスト無し）ではフルパスも不変ゆえ、これらのタスクでは `cargo test -p {crate} --all-targets -- --list ... | grep ': test$' | sort` の差分も空であるべき。3.7/3.8・Pattern C（統合テスト分割）はネスト段が増えるためフルパスは変化し得る（リーフ多重集合のみ権威）。
  - in-source 単体テストの形: `module::path::tests::<fn>`（`::tests::` を含む行が456件/wintf）。Pattern A は親モジュール直下の `tests` を `{module}/tests.rs` へ移すのみ → フルパス完全保存。
- **ベースライン**: 全3クレート `cargo test` グリーン（wintf 1102 / dola 580 / areka 22 テスト、DirectComposition環境、失敗ゼロ）。
- **既知のフレーキーテスト**: `world_lifecycle_test::try_tick_world_increments_frame_count_each_call`（DirectComposition/GPU実行時依存）はフルスイートのGPU競合下で稀に失敗するが、単独実行・ベースラインでは成功する環境起因の揺らぎ。これ**単独**の失敗は回帰ではない。判定法: `cargo test -p wintf --test world_lifecycle_test try_tick_world_increments_frame_count_each_call` を単独再実行し成功すればフレーキー扱い。他のテスト失敗や単独でも再現する失敗は真の回帰として扱う。
- **コミット注意**: 分割タスクは新規サブファイル（tests.rs 等）が生じる。`git add <module-dir>/` でディレクトリごとステージし、コミット後 `git status --porcelain` 清浄を必ず確認（rename のみ捕捉して新規ファイルを取りこぼす事故を防ぐ。task 3.4 で一度発生→amendで是正）。
- **2.1 死体削除の知見**: research.md の確定リストのうち `Opacity` deprecated static (metrics.rs:65-92) は**真の死体ではなかった**。R1.7 grep-gate が `tests/layout/metrics_test.rs`・`tests/visual/component_test.rs` の実参照を検出し、削除せず除外（削除すればテスト消失でゴールデン違反）。実際に削除したのは types.rs の5エイリアス・systems.rs の3関数・mouse モジュール/再export・`taffy_flex_demo_old.rs` の4項目。metrics.rs は無改変。→ deprecated 表記でも grep で実参照を確認するまで削除しない原則を厳守。
