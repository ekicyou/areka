# X1-T: 横断プロジェクト設定 × テスト構成の点検（設定がテスト実行へ与える構成）

- status: no-change
- commit: （設定ファイルの変更なし。docs コミット — 断片 X1-T.md ＋ proposals.md への P70 追記のみ）

本セルは X1 領域（横断プロジェクト設定）の最初のセルであり、コードではなく**ワークスペース設定がテスト実行へ与える構成**を点検する。点検の結論は **no-change**（境界内の設定ファイルに是正すべき設定起因テスト漏れは存在せず）。唯一の構成ギャップ（S2 が非既定 feature テストを実行しない）は S2／プロファイル領分かつ Revalidation Trigger に該当するため、設定変更ではなく **P70** として記録した（後述）。

## 点検対象設定ファイル一覧（境界 = 設定ファイルのみ）

| ファイル | 内容 | テスト構成への関与 |
|----------|------|-------------------|
| ルート `Cargo.toml` | `[workspace]` members=`crates/*`・`[workspace.package]`（`publish = false`）・`[workspace.dependencies]`・`[patch.crates-io]`（pasta_core→vendors）・`[profile.release]` | members 解決・共有 dep バージョン・全 feature 定義の供給元 |
| `crates/areka/Cargo.toml` | `publish = true`（上書き）・`[[bin]]`・deps。dev-deps なし・features なし | bin 1 + in-source `#[cfg(test)]`（main.rs）。tests/ ディレクトリなし |
| `crates/dola/Cargo.toml` | `publish = true`（上書き）・`[features]`（`default=["json"]`・`json`/`toml`/`yaml`）・optional deps（serde_json/toml/serde_yaml）。**dev-deps なし** | feature ゲート付きテストの可否を決定する核心 |
| `crates/wintf/Cargo.toml` | `publish = true`（上書き）・`[features]`（`serde`）・`[dev-dependencies]`（human-panic/rand/async-io/tracing-subscriber/image） | 統合テスト・examples・in-source test の dev-dep 供給元 |
| `.gitignore` | `target`/`Cargo.lock`/`tmpclaude*`/`.vs/`/`*_test.txt`/`*_dump.txt` | テスト生成物・スクラッチの除外（テスト実行に支障なし） |
| `.gitmodules` | `vendors/pasta`（pasta_core の patch 元） | S8 ゲート（submodule 初期化）の対象。初期化済みを確認 |
| `.vscode/settings.json` | formatOnSave/formatOnPaste のみ | テスト実行に無関与 |
| `.vscode/tasks.json` | `cargo build`・`cargo test`（`group.test.isDefault=true`・`$rustc` matcher） | エディタからのテスト実行タスク。**正当**（cargo test を呼ぶ） |
| `.vscode/launch.json` | CodeLLDB デバッグ構成2件＋（誤配置の）`tasks` キー | デバッグ用。テスト実行には無関与だが所見あり（後述） |

## テスト構成の点検結果

### (1) テストエントリポイントの束ね規約 — 漏れなし（核心の点検）

全クレートは **`[[test]]` 明示宣言を一切持たず純粋に auto-discovery**（`autotests=false` も不在。`areka/Cargo.toml` の `[[bin]]` のみが明示ターゲット）。したがって `tests/*.rs` 直下の各ファイルが独立した統合テストバイナリとして自動検出され、各バイナリ内へ `#[path=...] mod` で実テストファイルを束ねる規約になっている。**束ね漏れ（どのエントリにも mod 宣言されない孤児テスト）= cargo test で実行されない死テスト**が唯一の設定起因漏れ経路であり、これを機械的に突き合わせた。

- **エントリポイント数 = 15**（dola 6: compile/cue/general/runtime/trigger/validation、wintf 9: com/drag/ecs/graphics/layout/thread_mgr/visual/widget/window）。うち **14 が `mod` 集約**、`wintf/tests/thread_mgr.rs` のみ `#[test]` を直書きする単一ファイル統合テスト（companion ディレクトリなし＝正当）。
- **`tests/` 配下の全 `*.rs` = 124 ファイル** = 15 エントリ + 109 サブファイル。
- **突き合わせ（find で実在ファイル列挙 ↔ 各エントリの `#[path]` 宣言を comm で差分）結果: 孤児ファイル 0 / 宣言済みだが実在しない（dangling mod）0**。14 ディレクトリすべてで実在ファイル集合と宣言集合が**完全一致**。`common/mod.rs`（共有ヘルパ）パターンも全該当エントリ（compile/trigger/validation/com/visual）で正しく宣言済み。
- ディレクトリ直下サブディレクトリはすべて同名 `<name>.rs` エントリと対応。例外は **`wintf/tests/assets/`**（画像・バイナリ fixture 13 ファイルのみ、`*.rs` ゼロ）でエントリ不要＝正当。
- in-source 別ファイルテストモジュール（`*/tests.rs` 9 箇所: dola 4・wintf 5）も親 `mod.rs` の `#[cfg(test)] mod tests;` で全件宣言済み。

→ **束ね漏れによる死テストはゼロ。設定起因のテスト漏れ（束ね規約由来）なし。**

### (2) feature 組合せ — ギャップ1件（既定 S2 が非既定 feature テストを実行しない → P70）

- `wintf` の features は `serde`（optional serde 派生）のみ。**wintf テストに `#[cfg(feature=...)]` ゲートは 0 件**（feature 差でビルド/実行されないテストなし）。
- `dola` の features は `default=["json"]`・`json`/`toml`/`yaml`。`crates/dola/tests/general/integration_test.rs` に **feature ゲート付きテスト 3 件**:
  - `#[cfg(feature = "toml")]`（行193-212）: `complete_document_toml_roundtrip`・`btreemap_key_order_deterministic_toml`
  - `#[cfg(feature = "yaml")]`（行218-229）: `complete_document_yaml_roundtrip`
- これらは `toml`/`yaml` が既定外のため**正準 S2（`cargo test --workspace`＝既定 feature）では1件もビルド・実行されない**。実測で裏取り:
  - `cargo test --workspace` = **1713 passed / 0 failed / 32 ignored**（親ベースライン一致）
  - `cargo test --workspace --all-features` = **1716 passed / 0 failed / 32 ignored**（ビルド成功・失敗ゼロ）
  - 差分 **+3** はテスト名 diff で上記3テストと**厳密一致**（`toml_integration_tests::*` ×2 ＋ `yaml_integration_tests::complete_document_yaml_roundtrip`）。
- feature ゲート**自体は健全**（`toml::`/`serde_yaml::` 参照は cfg モジュール内に限定済みで、feature オフ時に不在 crate を参照しない）。ギャップは「ループ標準の回帰検知（S2）が非既定 feature のシリアライズ往復経路を保護しない」点。S2 の定義変更は design.md「Revalidation Triggers」（S2 変更＝全セル非破壊確認の意味が変わる）に該当しプロファイル（X1-S/X1-V）の領分のため、本セルでは設定変更せず **P70** に記録（CI 欠落と同様、構成是正判断は上位へ）。

### (3) dev-dependencies の整合 — 漏れ・余剰なし

- **wintf `[dev-dependencies]`（5件）はすべて実消費**: `image`→統合テスト `tests/com/wic_test.rs` ＋ in-source test `src/ecs/widget/bitmap_source/tests.rs`（`#[cfg(test)] mod tests;` で gate 済み）＋ examples 3件。`human-panic`/`rand`/`async-io`/`tracing-subscriber`→ `examples/`（`cargo build --examples --workspace` 成功で結線確認）。`image` の src 非テスト参照は **`ID2D1Image` のローカル変数名 `image`** であり `image` crate ではない（`image::`/`use image` の実 crate 参照は非テスト src に 0 件と確認）。dev-dep として正しいスコープ。
- **dola は `[dev-dependencies]` 不在**。テストが使う `serde_json`/`toml`/`serde_yaml` は通常 optional deps を feature 経由でテストへ供給する設計（serde_json は `default=["json"]` で常時利用可、toml/yaml は (2) のゲートと対応）。dev-dep 宣言漏れではなく feature 駆動の意図的構成。
- **areka は dev-deps 不要**（in-source `#[cfg(test)]` は std のみ・tests/ なし・examples なし）。

## 特定・是正した設定起因テスト漏れ

**なし**（境界内設定ファイルの変更ゼロ）。束ね規約は孤児ゼロで健全、dev-deps は整合、feature ゲートは正当。唯一の構成ギャップ（(2)）は S2／プロファイル領分のため P70 として記録に留め、X1-T の境界（設定ファイルのみ・挙動/検証契約を変える変更はしない）を遵守した。

## 自動化できない所見 / 申し送り

1. **CI 欠落（所見・記録のみ）** — `.github/workflows`・`.gitlab-ci.yml`・`azure-pipelines.yml`・`.circleci`・`appveyor.yml`・`.travis.yml`・`Jenkinsfile` をすべて確認し**いずれも不在**。テスト構成（feature 全網羅・examples ビルド・全ターゲット）を自動で常時検証する基盤がない。CI 新設は本ループ対象外（design.md Non-Goals）のため事実の所見記録に留める。P70 の suggestion (b) で「CI 新設時は `--all-features` ジョブを含める」ことを申し送った。
2. **`publish = true` 上書き（X1 全体メモの再確認・意図判断は X1-S/X1-V へ申し送り）** — requirements.md「Boundary Context」Adjacent expectations 及び R2.9 前提は「本ワークスペースのクレートは未公開（`publish = false`）」だが、実態は `[workspace.package]` の `publish = false`（ルート Cargo.toml 行13）を **areka（行10）・dola（行10）・wintf（行10）の各 `[package]` が `publish = true` で明示上書き**している。これは W1-S 申し送り（本ループ発見）と一致。design.md「Revalidation Triggers」は「公開ポリシー変更（`publish = false` 解除）＝ R2.9 の非推奨コード削除前提が崩れる」とするため、この上書きの意図確認と整合性判断（要件前提の更新 or 設定の是正）は X1-S/X1-V の担当。X1-T では**テスト実行への影響なし**（publish 設定はテスト実行に無関与）と確認のうえ事実を再記録。
3. **`.vscode/launch.json` の陳腐化（テスト実行には無関与・X1-S 申し送り）** — (a) 「Rust: Debug (binary path)」が `target/debug/sample_dcomp.exe` を参照するが、`sample_dcomp` は bin/example として**存在しない**（grep 0 件）陳腐な参照。(b) `tasks` キーが launch.json 内に誤配置（tasks は tasks.json の所掌で launch.json では無効・無視される。tasks.json 側に同等定義が正しく存在）。これらは**デバッグ/起動構成**の不備でありテスト実行構成には影響しないため、X1-T（テスト構成点検）では是正せず X1-S（横断設定の構造的整理）へ申し送る。design.md X1 行の「古い launch.json」記載と一致。
4. **examples とテストの分離（構成事実の記録）** — 正準 S2（`cargo build --workspace` ＋ `cargo test --workspace`）は **examples をビルドしない**（cargo の既定）。example 専用 dev-dep（human-panic/async-io/tracing-subscriber/rand）の結線は `cargo build --examples` でのみ検証される。本セルで `cargo build --examples --workspace` 成功を確認済み。これは既知の構成（W1-S が `cargo build --examples -p wintf` を補助実行済み）であり新たな漏れではないが、S2 単独では example のビルド可否を保証しない点を構成事実として記録（P70 の (a) と独立。example は test ではないため P70 の範囲外）。

## proposals へ回した候補

- **P70**（新規・source X1-T）: 検証コマンド S2 が feature ゲート付きテスト（dola toml/yaml 往復3件）を既定で実行しない（回帰非保護）。S2 へ `--all-features` 一巡の追加を提案。実測根拠 1713→1716（+3 厳密一致）を proposals.md に記録。挙動変更は伴わず（実行範囲の拡大のみ）、既存 P14（NaN/inf を「feature ゲートで既定外」と言及）が未記録だった**検証スコープ側の事実**を補完。

## verification (S2)

- **BEFORE**: 親検証済みベースライン（1713 passed / 0 failed）を信頼し省略（X1-T は設定点検タスクで境界内コード変更なし）。
- **AFTER（全量・必須）**:
  - `cargo build --workspace` 成功。
  - `cargo test --workspace --no-fail-fast` = **1713 passed / 0 failed / 32 ignored**（ベースラインと一致・増減ゼロ。設定ファイル変更なしのため当然）。
  - 追加点検 `cargo build --workspace --all-features` 成功（toml/serde_yaml/wintf-serde を追加コンパイル）。
  - 追加点検 `cargo test --workspace --all-features --no-fail-fast` = **1716 passed / 0 failed / 32 ignored**（既定外 feature の +3 テストも合格）。
  - 追加点検 `cargo build --examples --workspace` 成功（example 専用 dev-dep の結線確認）。
- **増減内訳**: 既定 S2 は ±0（設定無変更）。`--all-features` は既定比 **+3**（feature ゲート付き toml/yaml 往復テストが追加実行され全件合格）。失敗ゼロ。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy --workspace` は約156件の指摘を出力し非ゼロ終了（exit 101）。内訳上位: `collapsible_if`×68・`type_complexity`×30・**`not_unsafe_ptr_arg_deref`×20（error 級）**・`question_mark`×8・`too_many_arguments`×6 等。
- `not_unsafe_ptr_arg_deref` が `error:` 表示なのは**clippy 既定の `#[deny(...)]`**（プロジェクト設定ではない。`clippy.toml`/`[lints]`/deny 指定は境界内設定ファイルに**不在**と確認）。raw ポインタ deref する pub 関数が `unsafe` 未マークという**ソースコード**条件で、COM/winproc 系（W2/W1）の既レビュー領分。
- これらはすべて**ソースコード**の指摘であり X1-T の境界（設定ファイルのみ）外。S2（build/test）は緑であり、S3 はブロッカーとしない規約に従い記録のみ。境界内設定ファイルに clippy 設定は存在せず X1-T で是正すべき lint 構成はなし。

## flaky

- 既知フレーキー `wintf tests/ecs cue_performance_test::bench_pop_ready_empty_queue`（壁時計ベンチ負荷依存・W8-T1 で解消済み）は本セルの全量実行（既定・--all-features とも）で初回から pass。再実行不要。**flaky 発生なし。**
