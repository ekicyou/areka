# Implementation Plan

## Overview
本実装計画は、wintf、dola、arekaの3つのクレートをcrates.ioに初回公開（バージョン0.0.1）し、名称を確保するための具体的な作業手順を定義する。

## Tasks

### 1. Cargo.toml メタデータ設定
- [ ] 1.1 (P) ワークスペース Cargo.toml 更新
  - `workspace.package` セクションに `version = "0.0.1"` を設定
  - `authors = ["ekicyou <dot.station@gmail.com>"]` に更新
  - `license = "MIT"` に変更（"MIT OR Apache-2.0" から）
  - `repository = "https://github.com/ekicyou/areka"` を追加
  - _Requirements: 1.1, 1.5_

- [ ] 1.2 (P) wintf Cargo.toml 更新
  - `description = "Windows Tategaki Framework - Rust UI library with Japanese vertical text support"` を追加
  - `publish = true` に明示的変更
  - `keywords = ["windows", "ui", "directcomposition", "japanese", "vertical-text"]` を追加
  - `categories = ["gui", "graphics", "rendering", "os::windows-apis"]` を追加
  - `authors.workspace = true`, `license.workspace = true`, `repository.workspace = true` を追加
  - _Requirements: 1.2, 1.5, 1.6_

- [ ] 1.3 (P) dola Cargo.toml 更新
  - `publish = true` に明示的変更
  - `keywords = ["animation", "declarative", "easing", "interpolation", "timeline"]` を追加
  - `categories = ["graphics", "game-development"]` を追加
  - `repository.workspace = true` を追加
  - _Requirements: 1.3, 1.5, 1.6_

- [ ] 1.4 (P) areka Cargo.toml 更新
  - `version.workspace = true` に変更（既存の `version = "0.0.1"` から）
  - `repository.workspace = true` に変更（既存の直接指定から）
  - `description = "Desktop mascot platform for Windows"` に変更
  - `keywords = ["ukagaka", "desktop-mascot", "windows", "character", "interactive"]` を追加
  - `categories = ["gui", "games", "multimedia"]` を追加
  - _Requirements: 1.4, 1.5, 1.6_

### 2. README.md ドキュメント作成
- [ ] 2.1 (P) wintf README.md 作成
  - プロジェクト名 "wintf" とタイトルを記載
  - 簡潔な説明（英語）を記載（Cargo.toml の description と一致）
  - "Early Development - Version 0.0.1" ステータスセクションを追加
  - 名前予約目的である旨の警告を記載
  - About セクションでプロジェクトの目的と将来の方向性を説明
  - Usage セクションで本番環境での使用を推奨しない旨を記載
  - License セクションで "MIT" を明記
  - _Requirements: 2.1, 2.4, 2.5_

- [ ] 2.2 (P) dola README.md 作成
  - プロジェクト名 "dola" とタイトルを記載
  - 簡潔な説明（英語）を記載（Cargo.toml の description と一致）
  - "Early Development - Version 0.0.1" ステータスセクションを追加
  - 名前予約目的である旨の警告を記載
  - About セクションでプロジェクトの目的と将来の方向性を説明
  - Usage セクションで本番環境での使用を推奨しない旨を記載
  - License セクションで "MIT" を明記
  - _Requirements: 2.2, 2.4, 2.5_

- [ ] 2.3 (P) areka README.md 作成または検証
  - プロジェクト名 "areka" とタイトルを確認
  - 簡潔な説明（英語）を確認・更新（Cargo.toml の新 description と一致）
  - "Early Development - Version 0.0.1" ステータスセクションを追加または確認
  - 名前予約目的である旨の警告を記載
  - About セクションでプロジェクトの目的と将来の方向性を確認
  - Usage セクションで本番環境での使用を推奨しない旨を確認
  - License セクションで "MIT" を明記
  - _Requirements: 2.3, 2.4, 2.5_

### 3. LICENSE-MIT ファイル確認・作成
- [ ] 3. (P) LICENSE-MIT ファイル確認・作成
  - プロジェクトルートに LICENSE-MIT ファイルが存在するか確認
  - 存在しない場合、標準 MIT ライセンステキストを作成
  - Copyright holder を "ekicyou" に設定
  - Copyright year を 2026 に設定
  - Cargo.toml の `license = "MIT"` フィールドと整合性を確認
  - _Requirements: 3.1, 3.2, 3.3_

### 4. 公開前検証
- [ ] 4.1 テスト実行確認
  - `cargo test` を実行し、全テストがパスすることを確認
  - テスト失敗がある場合は修正してから次の手順に進む
  - _Requirements: 4.5_

- [ ] 4.2 Dry-run 検証実行
  - `cargo publish --dry-run -p dola` を実行し、エラーがないことを確認
  - `cargo publish --dry-run -p wintf` を実行し、エラーがないことを確認
  - `cargo publish --dry-run -p areka` を実行し、エラーがないことを確認
  - エラーが発生した場合は Cargo.toml または README.md を修正
  - 全ての dry-run が成功するまで繰り返す
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

### 5. crates.io 公開実行
- [ ] 5.1 dola クレート公開
  - `cargo publish -p dola` を実行
  - 公開成功メッセージを確認
  - crates.io で dola ページが作成されたことを確認（https://crates.io/crates/dola）
  - _Requirements: 5.1, 5.4, 5.5_

- [ ] 5.2 wintf クレート公開
  - 5.1 の公開成功を確認後、`cargo publish -p wintf` を実行
  - 公開成功メッセージを確認
  - crates.io で wintf ページが作成されたことを確認（https://crates.io/crates/wintf）
  - _Requirements: 5.2, 5.4, 5.5_

- [ ] 5.3 areka クレート公開
  - 5.2 の公開成功を確認後、`cargo publish -p areka` を実行
  - wintf 0.0.1 への依存が正しく解決されることを確認
  - 公開成功メッセージを確認
  - crates.io で areka ページが作成されたことを確認（https://crates.io/crates/areka）
  - _Requirements: 5.3, 5.4, 5.5_

### 6. 公開後確認とタグ作成
- [ ] 6. 公開確認と Git タグ作成
  - crates.io で wintf ページを開き、description と README が正しく表示されることを確認
  - crates.io で dola ページを開き、description と README が正しく表示されることを確認
  - crates.io で areka ページを開き、description と README が正しく表示されることを確認
  - 各ページで keywords と categories が正しく表示されることを確認
  - GitHub リポジトリへのリンクが正しく機能することを確認
  - Git タグ `v0.0.1` を作成: `git tag v0.0.1`
  - タグをリモートにプッシュ: `git push origin v0.0.1`
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

## Implementation Notes

### 実行順序
タスク 1-3 は並列実行可能。タスク 4 以降は順次実行が必要。

### 依存関係
- タスク 4: タスク 1-3 完了後
- タスク 5: タスク 4 完了後
- タスク 6: タスク 5 完了後

### エラーハンドリング
公開前検証（タスク 4）でエラーが発生した場合、タスク 1-3 に戻って修正。公開実行（タスク 5）で失敗した場合、エラーメッセージを確認し、必要に応じてメタデータを修正後、該当クレートの公開を再試行。

### crates.io API トークン
初回公開時は `cargo login` で crates.io API トークンを取得する必要がある。トークンは `~/.cargo/credentials` に保存される。
