# Design Document

## Overview
本仕様は、wintf、dola、arekaの3つのクレートをcrates.ioに初回公開（バージョン0.0.1）し、名称を確保するための技術設計を定義する。公開は最小限のメタデータとドキュメントで行い、本格的な機能実装は今後のバージョンで提供する。

**目的**: crates.io上でクレート名を確保し、将来的な本格公開（0.1.0以降）の基盤を整備する。

**対象ユーザー**: Rustエコシステムの開発者（将来的な利用者）

**影響**: プロジェクトの公開可視性が向上し、外部からの名前衝突リスクを排除する。

### Goals
- wintf、dola、arekaの3クレートをcrates.ioに公開し名称を確保
- 最小限のメタデータとドキュメントでcrates.io公開要件を満たす
- ワークスペースレベルでの共通メタデータ管理による保守性向上
- cargo publish 実行前の検証により公開エラーを回避

### Non-Goals
- 0.0.1での実用的な機能提供（名前予約のみ）
- crates.io badges やドキュメント生成の最適化
- CI/CD パイプラインでの自動公開設定
- Cargo.lock の管理方針決定（将来バージョンで対応）

## Architecture

### Existing Architecture Analysis
現在のワークスペース構成：
- モノレポ構造（`crates/*`）
- ワークスペースレベルで依存関係を管理（`workspace.dependencies`）
- 各クレートは独自の `Cargo.toml` を持つが、共通設定の継承は最小限

**現在の依存関係**:
- dola: 独立
- wintf: 独立
- areka: wintf に依存

### Architecture Pattern & Boundary Map
本仕様はアーキテクチャパターンの導入ではなく、既存ワークスペース構成への**メタデータ追加**である。

**変更箇所**:
```
ワークスペースルート (Cargo.toml)
├── workspace.package (追加・更新)
│   ├── version = "0.0.1"
│   ├── authors, license, repository (追加)
│
各クレート (crates/{wintf,dola,areka}/Cargo.toml)
├── metadata フィールド追加
│   ├── description, keywords, categories
│   ├── workspace 継承 (version, authors, license, repository)
│   ├── publish = true
│
各クレート (crates/{wintf,dola,areka}/README.md)
├── 新規作成（既存がある場合は検証のみ）
│
プロジェクトルート (LICENSE-MIT)
├── 新規作成（存在しない場合のみ）
```

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| Build System | Cargo (Rust 2024 Edition) | クレート公開ツール | workspace inheritance 使用（Cargo 1.64+） |
| Registry | crates.io | Rustクレート公開プラットフォーム | 標準レジストリ |
| VCS | Git + GitHub | リポジトリホスティング | タグ v0.0.1 作成 |

## System Flows

公開プロセスは単純な順次実行のため、フロー図は省略。

**実行順序**:
1. Cargo.toml メタデータ更新（ワークスペース + 各クレート）
2. README.md 作成・確認（各クレート）
3. LICENSE-MIT 確認・作成（プロジェクトルート）
4. `cargo test` で全テストパス確認
5. `cargo publish --dry-run` で検証（dola, wintf, areka 各々）
6. `cargo publish` で公開（dola → wintf → areka の順）
7. crates.io ページ確認
8. Git タグ `v0.0.1` 作成・プッシュ

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1 | Workspace metadata 設定 | Cargo.toml (workspace) | N/A | Step 1 |
| 1.2 | wintf metadata 設定 | Cargo.toml (wintf) | N/A | Step 1 |
| 1.3 | dola metadata 設定 | Cargo.toml (dola) | N/A | Step 1 |
| 1.4 | areka metadata 設定 | Cargo.toml (areka) | N/A | Step 1 |
| 1.5, 1.6 | Workspace inheritance 使用 | All Cargo.toml | N/A | Step 1 |
| 2.1, 2.2, 2.3, 2.4, 2.5 | README.md 準備 | README.md (各クレート) | N/A | Step 2 |
| 3.1, 3.2, 3.3 | LICENSE-MIT 確認 | LICENSE-MIT | N/A | Step 3 |
| 4.1, 4.2, 4.3, 4.4, 4.5 | 公開前検証 | cargo commands | CLI | Step 4-5 |
| 5.1, 5.2, 5.3, 5.4, 5.5 | crates.io 公開実行 | cargo publish | CLI | Step 6 |
| 6.1, 6.2, 6.3, 6.4, 6.5 | 公開後確認 | crates.io, git | Web/CLI | Step 7-8 |

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies (P0/P1) | Contracts |
|-----------|--------------|--------|--------------|--------------------------|-----------|
| Workspace Cargo.toml | Build Config | 共通メタデータ管理 | 1.1 | Cargo (P0) | Config File |
| wintf Cargo.toml | Build Config | wintf クレートメタデータ | 1.2, 1.5 | Workspace Cargo.toml (P0) | Config File |
| dola Cargo.toml | Build Config | dola クレートメタデータ | 1.3, 1.5 | Workspace Cargo.toml (P0) | Config File |
| areka Cargo.toml | Build Config | areka クレートメタデータ | 1.4, 1.5 | Workspace Cargo.toml (P0), wintf (P0) | Config File |
| README.md (wintf) | Documentation | crates.io ページ表示 | 2.1, 2.4, 2.5 | N/A | Markdown |
| README.md (dola) | Documentation | crates.io ページ表示 | 2.2, 2.4, 2.5 | N/A | Markdown |
| README.md (areka) | Documentation | crates.io ページ表示 | 2.3, 2.4, 2.5 | N/A | Markdown |
| LICENSE-MIT | Legal | MIT ライセンス条項 | 3.1, 3.2, 3.3 | N/A | Plain Text |
| Cargo Publish Process | Build Pipeline | 公開実行・検証 | 4.x, 5.x, 6.x | Cargo (P0), crates.io (P0) | CLI |

### Build Configuration

#### Workspace Cargo.toml

| Field | Detail |
|-------|--------|
| Intent | 3クレート共通のメタデータを一元管理 |
| Requirements | 1.1, 1.5 |

**Responsibilities & Constraints**
- `workspace.package` セクションで version, authors, license, repository を定義
- 各クレートが継承可能な形式で提供
- `publish = false` を維持（ワークスペースルート自体は非公開）

**Dependencies**
- Outbound: Cargo (P0) - ビルドシステム

**Contracts**: Config File [x]

##### Config File Structure
```toml
[workspace.package]
version = "0.0.1"
authors = ["ekicyou <dot.station@gmail.com>"]
license = "MIT"
repository = "https://github.com/ekicyou/areka"
```

**Implementation Notes**
- 既存の `version = "0.0.0"`, `publish = false` を更新
- `authors` を "Dot-Station Master" から "ekicyou" に変更
- `license` を "MIT OR Apache-2.0" から "MIT" に変更

#### wintf Cargo.toml

| Field | Detail |
|-------|--------|
| Intent | wintf クレートの公開メタデータ設定 |
| Requirements | 1.2, 1.5, 1.6 |

**Responsibilities & Constraints**
- クレート固有の description, keywords, categories を定義
- ワークスペースから version, authors, license, repository を継承
- `publish = true` に設定

**Dependencies**
- Inbound: Workspace Cargo.toml (P0) - メタデータ継承元

**Contracts**: Config File [x]

##### Config File Structure
```toml
[package]
name = "wintf"
description = "Windows Tategaki Framework - Rust UI library with Japanese vertical text support"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
publish = true
keywords = ["windows", "ui", "directcomposition", "japanese", "vertical-text"]
categories = ["gui", "graphics", "rendering", "os::windows-apis"]
```

**Implementation Notes**
- 既存の `publish = { workspace = true }` を `publish = true` に明示的変更
- `authors`, `license`, `repository` フィールドを追加（workspace 継承）
- `description`, `keywords`, `categories` を追加

#### dola Cargo.toml

| Field | Detail |
|-------|--------|
| Intent | dola クレートの公開メタデータ設定 |
| Requirements | 1.3, 1.5, 1.6 |

**Responsibilities & Constraints**
- クレート固有の description, keywords, categories を定義
- ワークスペースから version, authors, license, repository を継承
- `publish = true` に設定

**Dependencies**
- Inbound: Workspace Cargo.toml (P0) - メタデータ継承元

**Contracts**: Config File [x]

##### Config File Structure
```toml
[package]
name = "dola"
description = "Declarative Orchestration for Live Animation"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
publish = true
keywords = ["animation", "declarative", "easing", "interpolation", "timeline"]
categories = ["graphics", "game-development"]
```

**Implementation Notes**
- 既存の `description` を維持（既に適切）
- 既存の `publish.workspace = true` を `publish = true` に明示的変更
- `repository.workspace = true` を追加
- `keywords`, `categories` を追加

#### areka Cargo.toml

| Field | Detail |
|-------|--------|
| Intent | areka クレートの公開メタデータ設定 |
| Requirements | 1.4, 1.5, 1.6 |

**Responsibilities & Constraints**
- クレート固有の description, keywords, categories を定義
- ワークスペースから version, authors, license, repository を継承
- `publish = true` を維持
- wintf への依存関係を保持

**Dependencies**
- Inbound: Workspace Cargo.toml (P0) - メタデータ継承元
- Outbound: wintf (P0) - アプリケーション依存

**Contracts**: Config File [x]

##### Config File Structure
```toml
[package]
name = "areka"
description = "Desktop mascot platform for Windows"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
publish = true
keywords = ["ukagaka", "desktop-mascot", "windows", "character", "interactive"]
categories = ["gui", "games", "multimedia"]
```

**Implementation Notes**
- 既存の `version = "0.0.1"` を `version.workspace = true` に変更
- 既存の `repository = "https://github.com/ekicyou/areka"` を `repository.workspace = true` に変更
- `description` を "Desktop mascot platform inspired by Ukagaka" から "Desktop mascot platform for Windows" に変更
- `keywords`, `categories` を追加

### Documentation

#### README.md (wintf, dola, areka)

| Field | Detail |
|-------|--------|
| Intent | crates.io クレートページに表示される説明ドキュメント |
| Requirements | 2.1, 2.2, 2.3, 2.4, 2.5 |

**Responsibilities & Constraints**
- プロジェクト名、説明（英語）、開発状態、基本的な使用方法の注記を含む
- バージョン 0.0.1 が名前予約目的であることの警告を含む
- crates.io のレンダリング要件を満たす Markdown 形式

**Dependencies**
- なし（静的ドキュメント）

**Contracts**: Markdown Document [x]

##### Document Structure (Template)
```markdown
# [Crate Name]

[Brief description matching Cargo.toml description field]

## Status

⚠️ **Early Development - Version 0.0.1**

This crate is published for name reservation purposes. The API is not stable and may change significantly in future versions.

## About

[1-2 paragraphs about the crate's purpose and goals]

## Usage

Not recommended for production use at this stage. Please check back for future releases.

## License

MIT
```

**Implementation Notes**
- 各クレートで固有の description と About セクションを記述
- 既存の README.md がある場合は、Title, Description, Status セクションの存在を確認し、不足があれば追加

### Legal

#### LICENSE-MIT

| Field | Detail |
|-------|--------|
| Intent | MIT ライセンス条項の提供（crates.io 公開要件） |
| Requirements | 3.1, 3.2, 3.3 |

**Responsibilities & Constraints**
- 標準的な MIT ライセンステキストを含む
- Copyright holder: "ekicyou"
- Copyright year: 2026
- Cargo.toml の `license = "MIT"` フィールドと一致

**Dependencies**
- なし（静的ファイル）

**Contracts**: Plain Text File [x]

##### File Structure
```
MIT License

Copyright (c) 2026 ekicyou

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

[... standard MIT license text ...]
```

**Implementation Notes**
- プロジェクトルートに配置
- 既存の LICENSE-APACHE は削除不要（残しても問題ないが、Cargo.toml の `license` フィールドに記載しないため無視される）

### Build Pipeline

#### Cargo Publish Process

| Field | Detail |
|-------|--------|
| Intent | crates.io への公開実行と検証 |
| Requirements | 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5, 6.1, 6.2, 6.3, 6.4, 6.5 |

**Responsibilities & Constraints**
- 公開前検証（dry-run, tests）
- 依存順序での公開実行（dola → wintf → areka）
- 公開後の確認（crates.io ページ、git タグ）

**Dependencies**
- Outbound: Cargo CLI (P0) - ビルド・公開ツール
- Outbound: crates.io (P0) - レジストリ
- Outbound: Git (P0) - バージョンタグ作成

**Contracts**: CLI Workflow [x]

##### CLI Command Sequence
```bash
# Step 1: テスト実行
cargo test

# Step 2: Dry-run 検証
cargo publish --dry-run -p dola
cargo publish --dry-run -p wintf
cargo publish --dry-run -p areka

# Step 3: 公開実行（依存順序）
cargo publish -p dola
cargo publish -p wintf
cargo publish -p areka

# Step 4: crates.io 確認（手動）
# - https://crates.io/crates/dola
# - https://crates.io/crates/wintf
# - https://crates.io/crates/areka

# Step 5: Git タグ作成
git tag v0.0.1
git push origin v0.0.1
```

**Implementation Notes**
- 各 `cargo publish` コマンドの成功を確認してから次に進む
- エラー発生時は修正後に再度 dry-run から実施
- crates.io ページの表示確認では、description と README の正しいレンダリングを目視確認

## Data Models
本仕様ではデータモデルの変更はなし（設定ファイルとドキュメントのみ）。

## Migration and Deployment

### Migration Strategy
既存リポジトリへの追加作業のため、マイグレーションは不要。

**変更内容**:
- Cargo.toml ファイルの更新（破壊的変更なし）
- README.md の新規作成または検証
- LICENSE-MIT の新規作成（存在しない場合）

### Deployment Steps
1. ローカルブランチで Cargo.toml, README.md, LICENSE-MIT を更新
2. `cargo test` で全テストパス確認
3. `cargo publish --dry-run` で検証
4. Git コミット・プッシュ
5. `cargo publish` でcrates.ioに公開
6. Git タグ `v0.0.1` 作成・プッシュ

### Rollback Plan
crates.io からのクレート削除は不可能（[yank](https://doc.rust-lang.org/cargo/commands/cargo-yank.html) のみ可能）。

**0.0.1 公開後の修正方針**:
- メタデータの誤りが発見された場合は 0.0.2 で修正版を公開
- 致命的な問題がある場合は `cargo yank` で 0.0.1 を非推奨化し、0.0.2 を公開

## Performance Considerations
本仕様はビルド時・公開時の一度限りの操作のため、パフォーマンス要件なし。

## Security Considerations
- crates.io API トークンの管理: `cargo login` で取得したトークンは `~/.cargo/credentials` に保存される
- 公開後のクレートは誰でもダウンロード可能（パブリックリポジトリ）
- 0.0.1 は名前予約のみで実用コードを含まないため、セキュリティリスクは最小限

## Testing Strategy

### Validation Tests
- `cargo test` - 既存の全テストがパスすることを確認
- `cargo publish --dry-run` - 各クレートが公開可能であることを確認

### Manual Verification
- crates.io クレートページの表示確認（description, README, keywords, categories）
- Git タグの正確性確認（`git tag -l` で v0.0.1 存在確認）

### Test Coverage
新規コードなし（メタデータのみ）のため、コードカバレッジ要件なし。

## Monitoring and Observability
本仕様は一度限りの公開操作のため、継続的なモニタリングは不要。

**公開後の確認項目**:
- crates.io download 数（0.0.1 は名前予約のため、ダウンロード数は少ないことを想定）
- GitHub リポジトリへのリンクが正しく機能すること
