# Research & Design Decisions

## Summary
- **Feature**: `crate-name-reservation`
- **Discovery Scope**: Simple Addition
- **Key Findings**:
  - Cargo.toml workspace inheritance は Cargo 1.64+ で安定版サポート
  - crates.io は最大5個の keywords と categories を許可
  - 公開順序は依存関係に従う必要がある（依存先を先に公開）

## Research Log

### Cargo.toml Workspace Inheritance
- **Context**: 3つのクレートで共通メタデータをワークスペースレベルで管理
- **Sources Consulted**: 
  - [The Cargo Book - Workspace Inheritance](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-package-table)
  - 既存の `Cargo.toml` 設定
- **Findings**:
  - `workspace.package` セクションで `version`, `authors`, `license`, `repository` を定義可能
  - 各クレートで `field.workspace = true` または `{ workspace = true }` で継承
  - `description`, `keywords`, `categories` はクレート固有のため、各 `Cargo.toml` で定義
- **Implications**: ワークスペースレベルで共通メタデータを一元管理し、各クレートで継承する設計を採用

### crates.io Publication Requirements
- **Context**: 初回公開（0.0.1）に必要な最小限のメタデータと検証手順
- **Sources Consulted**: 
  - [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
  - [crates.io Manifest Format](https://doc.rust-lang.org/cargo/reference/manifest.html)
- **Findings**:
  - 必須フィールド: `name`, `version`, `authors`, `license` or `license-file`
  - 推奨フィールド: `description`, `repository`, `keywords` (最大5個), `categories` (最大5個)
  - README.md は自動的にパッケージに含まれ、crates.io ページに表示
  - `cargo publish --dry-run` でローカル検証可能
- **Implications**: 要件で定義したメタデータで crates.io 公開要件を満たす

### Publication Order
- **Context**: 依存関係を持つクレートの公開順序
- **Sources Consulted**: 既存の `Cargo.toml` 依存関係
- **Findings**:
  - dola: 独立（他のワークスペースクレートへの依存なし）
  - wintf: 独立（他のワークスペースクレートへの依存なし）
  - areka: wintf に依存
- **Implications**: 公開順序は dola → wintf → areka（arekaはwintf公開後に実行）

## Design Decisions

### Decision: Workspace Inheritance for Metadata
- **Context**: 3つのクレートで共通メタデータ（version, authors, license, repository）を管理
- **Alternatives Considered**:
  1. 各クレートで個別に定義 - 重複が多く保守性が低い
  2. Workspace inheritance - 一元管理で保守性向上
- **Selected Approach**: `workspace.package` で共通メタデータを定義し、各クレートで継承
- **Rationale**: DRY原則に従い、バージョン更新時の変更箇所を最小化
- **Trade-offs**: Cargo 1.64+ が必要だが、現在のプロジェクトは Rust 2024 Edition を使用しており問題なし

### Decision: MIT License Only
- **Context**: ライセンスを MIT のみに変更（元の MIT OR Apache-2.0 から）
- **Selected Approach**: MIT ライセンスのみを採用
- **Rationale**: シンプルさと管理コストの削減
- **Trade-offs**: Apache-2.0 の特許条項の保護は失うが、0.0.1 名前予約段階では問題なし

## Risks & Mitigations
- **Risk**: areka 公開時に wintf 0.0.1 が crates.io に未公開の場合、依存解決エラー
  - **Mitigation**: 公開順序を dola → wintf → areka で厳守し、各公開成功を確認してから次に進む
- **Risk**: Cargo.toml メタデータの記述ミスによる `cargo publish` エラー
  - **Mitigation**: `cargo publish --dry-run` で事前検証を実施
- **Risk**: README.md の不足によるcrates.ioページの品質低下
  - **Mitigation**: 最小限の構成（Title, Description, Status, Warning）を要件で定義

## References
- [The Cargo Book - Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [The Cargo Book - Workspace Inheritance](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-package-table)
- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
