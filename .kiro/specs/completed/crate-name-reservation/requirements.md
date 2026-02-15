# Requirements Document

## Project Description (Input)
wintf / dola / areka クレートを、ver 0.0.1で公開しておきたい（名前の予約程度の位置づけ）

## Introduction
本仕様は、wintf、dola、arekaの3つのクレートをcrates.ioに初回公開（バージョン0.0.1）し、名称を確保することを目的とする。公開は最小限のメタデータとドキュメントで行い、本格的な機能実装は今後のバージョンで提供する。

## Requirements

### Requirement 1: Cargo.toml メタデータ設定
**Objective:** 開発者として、3つのクレートを公開可能な状態にするため、Cargo.tomlに必要なメタデータを設定したい

#### Acceptance Criteria
1. The workspace Cargo.toml shall set workspace.package fields: version = "0.0.1", authors = ["ekicyou <dot.station@gmail.com>"], license = "MIT", repository = "https://github.com/ekicyou/areka"
2. The wintf Cargo.toml shall include: description = "Windows Tategaki Framework - Rust UI library with Japanese vertical text support", publish = true, keywords = ["windows", "ui", "directcomposition", "japanese", "vertical-text"], categories = ["gui", "graphics", "rendering", "os::windows-apis"], and inherit version/edition/authors/license/repository from workspace
3. The dola Cargo.toml shall include: description = "Declarative Orchestration for Live Animation", publish = true, keywords = ["animation", "declarative", "easing", "interpolation", "timeline"], categories = ["graphics", "game-development"], and inherit version/edition/authors/license/repository from workspace
4. The areka Cargo.toml shall include: description = "Desktop mascot platform for Windows", publish = true, keywords = ["ukagaka", "desktop-mascot", "windows", "character", "interactive"], categories = ["gui", "games", "multimedia"], and inherit version/edition/authors/license/repository from workspace
5. When Cargo.toml ファイルを更新する際、the 開発者 shall use workspace inheritance syntax (field.workspace = true) for shared metadata
6. The 全てのクレート shall have all required crates.io metadata fields populated before publication

### Requirement 2: README.md ドキュメント準備
**Objective:** 開発者として、crates.io上でクレートの目的が明確に伝わるよう、README.mdを準備したい

#### Acceptance Criteria
1. The wintf クレート shall have a README.md with project name, brief description (English), current status ("early development"), and basic usage note
2. The dola クレート shall have a README.md with project name, brief description (English), current status ("early development"), and basic usage note
3. The areka クレート shall have a README.md with project name, brief description (English), current status ("early development"), and basic usage note
4. When README.md が既に存在する場合、the クレート公開プロセス shall verify it contains minimum required sections (Title, Description, Status)
5. The README.md shall include a warning that this is version 0.0.1 for name reservation purposes

### Requirement 3: ライセンスファイル確認
**Objective:** 開発者として、crates.io公開要件を満たすため、適切なライセンスファイルが存在することを確認したい

#### Acceptance Criteria
1. The プロジェクトルート shall contain LICENSE-MIT file matching Cargo.toml license field ("MIT")
2. If LICENSE-MIT ファイルが存在しない場合、the クレート公開プロセス shall create standard MIT license file with copyright holder "ekicyou"
3. The LICENSE-MIT file shall include current year (2026) and copyright holder name "ekicyou"

### Requirement 4: 公開前検証
**Objective:** 開発者として、crates.ioへの公開前にローカルで検証を行い、エラーを事前に検出したい

#### Acceptance Criteria
1. When 公開準備が整った場合、the 開発者 shall run `cargo publish --dry-run -p wintf` and verify no errors occur
2. When 公開準備が整った場合、the 開発者 shall run `cargo publish --dry-run -p dola` and verify no errors occur
3. When 公開準備が整った場合、the 開発者 shall run `cargo publish --dry-run -p areka` and verify no errors occur
4. If dry-run でエラーが発生した場合、the 開発者 shall fix the issues before proceeding to actual publication
5. The 公開プロセス shall verify all tests pass (`cargo test`) before publication

### Requirement 5: crates.io 公開実行
**Objective:** 開発者として、検証済みのクレートをcrates.ioに公開し、名称を確保したい

#### Acceptance Criteriadola` to publish dola crate first (no dependencies)
2. When dola 公開が成功した場合、the 開発者 shall run `cargo publish -p wintf` to publish wintf crate (no dependencies)
3. When wintf 公開が成功した場合、the 開発者 shall run `cargo publish -p areka` to publish areka crate (depends on wintf)
4. The 公開プロセス shall publish crates in dependency order: independent crates (dola, wintf) first, then dependent crate (areka)
4. The 公開プロセス shall publish crates in dependency order (wintf → dola → areka) to avoid dependency resolution errors
5. When 公開が完了した場合、the 開発者 shall verify each crate appears on crates.io with version 0.0.1

### Requirement 6: 公開後確認
**Objective:** 開発者として、公開されたクレートがcrates.io上で正しく表示されることを確認したい

#### Acceptance Criteria
1. The 開発者 shall verify wintf crate page (https://crates.io/crates/wintf) displays correctly with description and README
2. The 開発者 shall verify dola crate page (https://crates.io/crates/dola) displays correctly with description and README
3. The 開発者 shall verify areka crate page (https://crates.io/crates/areka) displays correctly with description and README
4. When 全ての公開が完了した場合、the 開発者 shall document publication completion in specification status
5. The 開発者 shall create a git tag "v0.0.1" and push to repository after successful publication
