# X1-S: 横断プロジェクト設定 × シンプル化と整理（設定の簡素化・古い設定の是正）

- status: completed
- commit: refactor(X1): 陳腐化した launch.json（sample_dcomp.exe 参照・tasks キー誤配置）の是正と publish=true/profile を P71/P72 へ記録

本セルは X1 領域（横断プロジェクト設定）の S セル。S6（karpathy）基準で境界内設定ファイル（ルート/各クレート `Cargo.toml`・`.gitignore`・`.gitmodules`・`.vscode/`）の簡素化候補を検証し、**挙動非破壊で確実に改善する整理のみ適用**、**ビルド成果物の観測可能挙動を変える設定（profile/feature/依存/publish）は `proposals.md`（P71・P72）へ記録**した。タスク本文が明示する `.vscode/launch.json` の古い `sample_dcomp.exe` 参照修正＋`tasks` キー誤配置の是正を実施（いずれもエディタ設定でビルド成果物挙動に無関与＝挙動非破壊）。

## 点検対象設定ファイル一覧（境界 = 設定ファイルのみ）

| ファイル | 内容 | S6 判定 |
|----------|------|---------|
| ルート `Cargo.toml` | `[workspace]` members=`crates/*`・`[workspace.package]`（`publish = false`）・`[workspace.dependencies]`・`[patch.crates-io]`・`[profile.release]` | publish/profile は成果物挙動に関与 → **proposals**（P71・P72）。それ以外は健全・churn 回避で不適用 |
| `crates/areka/Cargo.toml` | `publish = true`（上書き・行10）・`[[bin]]`（`areka`）・deps | publish 上書き → P71。他は健全 |
| `crates/dola/Cargo.toml` | `publish = true`（上書き・行10）・`[features]`（`default=["json"]`・json/toml/yaml）・optional deps | publish 上書き → P71。feature 定義は健全（X1-T 検証済み） |
| `crates/wintf/Cargo.toml` | `publish = true`（上書き・行10）・`[features]`（serde）・`[dev-dependencies]` 5件 | publish 上書き → P71。dev-deps は全件実消費（X1-T 検証済み） |
| `.gitignore` | `target`/`Cargo.lock`/`tmpclaude*`/`.vs/`/`*_test.txt`/`*_dump.txt` | 簡潔・陳腐化なし・重複なし。**整理候補なし**（churn 回避で不適用） |
| `.gitmodules` | `vendors/pasta` 1件 | 単一・正当。**整理候補なし** |
| `.vscode/settings.json` | `formatOnPaste`/`formatOnSave` のみ | 最小・正当。**整理候補なし** |
| `.vscode/tasks.json` | `cargo build`・`cargo test`（`group.test.isDefault=true`・`$rustc` matcher） | 正当（cargo を正しく呼ぶ）。launch.json の誤配置 tasks の正しい所掌先。**整理候補なし** |
| `.vscode/launch.json` | CodeLLDB デバッグ構成2件＋**誤配置 `tasks` キー**・**陳腐化 `sample_dcomp.exe` 参照** | **是正適用**（下記） |

## 適用した整理（挙動非破壊・1ファイル）

### `.vscode/launch.json` の是正（タスク本文の明示対象）

2件の陳腐化を是正。**いずれもエディタ/デバッガ起動構成であり、cargo のビルド・テスト・成果物に一切関与しない**ため挙動非破壊。編集後 JSONC として妥当（行コメント strip 後に `JSON.parse` 成功を確認）。

1. **陳腐化バイナリ参照の修正（旧 `sample_dcomp.exe` → 新 `areka.exe`）**
   - 旧: `"program": "${workspaceFolder}/target/debug/sample_dcomp.exe"`（行22）
   - 新: `"program": "${workspaceFolder}/target/debug/areka.exe"`
   - 根拠: `sample_dcomp` はワークスペース全域で**実在しない**（grep ヒットは launch.json・本仕様の specs/research・tasks のみ。ソース/Cargo.toml に `[[bin]]`/`[[example]]` 定義ゼロ）。一方、ワークスペース唯一の `[[bin]]` は `areka`（`crates/areka/Cargo.toml:14-16`）で、`cargo build` 後の実成果物は `target/debug/areka.exe`（実ファイル存在確認: `target/debug/*.exe` = `areka.exe` のみ）。よって「ビルド済みバイナリを直接起動するデバッグ構成」が指すべき正しい実ターゲットへ修正。design.md X1 行・research.md 行32（「古いバイナリパス `sample_dcomp.exe` 残存」）が明示する是正対象。**挙動非破壊根拠**: launch.json の `program` は VS Code+CodeLLDB のデバッグ起動時のみ参照され、cargo ビルド/テスト/リリース成果物の生成・内容に無影響。
   - 別構成「Rust: Debug (cargo build)」（行5-16、`cargo: { args:["build"], filter:{kind:"bin"} }` で単一 bin を自動選択）は**正当かつ現役**のため温存（こちらは bin 名をハードコードせず自動解決するので陳腐化しない）。

2. **誤配置 `tasks` キーの削除**
   - 削除: launch.json 直下の `"tasks": [ {cargo build}, {cargo test} ]`（旧行28-43）。
   - 根拠: `tasks` は `tasks.json`（schema `2.0.0`）の所掌キーであり、**launch.json（schema `0.2.0`）には `tasks` プロパティが存在せず VS Code に無視される死設定**。同等の `cargo build`/`cargo test` タスク定義は `.vscode/tasks.json` に正しく存在する（しかも tasks.json 側は `group.test.isDefault`・`presentation` 等のより完全な定義を持つ）。**挙動非破壊根拠**: 当該キーは配置先で一切解釈されない（no-op）ため、削除しても VS Code の挙動は不変（タスク実行は元から tasks.json が供給）。かつ cargo に無関与。

その他: launch.json の既存 `//` コメント（CodeLLDB 案内・「run→build に変更」「対象bin自動選択」「exe 名を変更」）は説明価値があり実態と整合的なため**温存**（churn 回避＝karpathy 原則「自分の変更が生んだ孤児のみ除去、既存の無関係箇所は触れない」）。ファイル末尾の no-newline（元から）も維持。

## proposals へ回した候補（P71・P72・新規採番）

タスク本文の指示「ビルド成果物の挙動（リリース最適化・LTO 等）を変える設定変更は適用せず proposals へ記録」および公開可否＝挙動相当の判断に従い、以下は**適用せず記録のみ**。

- **P71**（新規・source X1-S）: 各クレートの `publish = true` 上書き（areka/dola/wintf の各 `[package]` 行10）と要件前提（requirements.md 行25「`publish = false`・外部利用者なし」）の矛盾。ルート `[workspace.package]` は `publish = false`（行13）だが3クレートが個別に `true` へ上書きしており、本ループの削除判断（R2.9/R5.3 の「後方互換性考慮不要」前提）の土台と食い違う。**公開可否という挙動相当の設定**であり design.md「Revalidation Triggers」（行52「公開ポリシー変更＝ R2.9 前提が崩れる」）が明示トリガー対象とするため、明白なミスと断定せず意図確認へ回した（W1-S → X1-T → 本セルの申し送り。X1-V の依存監査と併せ最終判断）。
- **P72**（新規・source X1-S）: `[profile.release]`（行82-92）の最適化設定見直し。`opt-level = 'z'`（サイズ最優先）はデスクトップマスコットの実行時性能とのトレードオフで `'s'`/`3` が適切な可能性、`lto`/`codegen-units`/`strip`、および末尾の陳腐化コメント「(変更)」（行89・91）が候補だが、**いずれもリリース成果物のサイズ・性能・パニック戦略・シンボル有無という観測可能な成果物特性を変える**ためタスク本文の指示どおり適用せず記録（ベンチ計測を伴う独立タスク化を提案）。

## 適用しなかった候補と理由

- **publish=true・profile**: 上記 P71・P72（成果物挙動/公開可否に関与＝本セル不適用）。
- **`[profile.release]` 内のコメント整理単独**: 成果物特性を決定するブロック内への編集は判断を要し churn。P72 に内包し見送り。
- **依存バージョン固定・feature 既定の変更**: 成果物/挙動を変えるため対象外（X1-T が feature 構成の健全性を検証済み。本セルでは触れない）。
- **`.gitignore`/`.gitmodules`/`settings.json`/`tasks.json`**: いずれも簡潔・正当で陳腐化/重複/dead 設定なし。**改善で挙動が不変かつ明確に良くなる整理が存在しない**ため karpathy 原則（surgical・churn 回避）で不適用（X1-T が同ファイル群をテスト構成観点でも健全と確認済み）。
- **clippy 指摘（156件規模）**: 全て**ソースコード**の lint（境界外）。境界内設定ファイルに `clippy.toml`/`[lints]`/deny 指定は**不在**（X1-T 確認・本セルでも再確認）ため、X1-S で是正可能な lint 構成設定はなし。

## verification (S2)

- **BEFORE**: 親検証済みベースライン（1713 passed / 0 failed / 32 ignored）を信頼し省略（本セルは設定整理タスク・境界内コード変更なし）。
- **AFTER（全量・必須）**:
  - `cargo build --workspace` 成功。
  - `cargo build --examples --workspace` 成功（launch.json が参照する実バイナリ群／example 専用 dev-dep の結線確認。`areka.exe` 実在も確認）。
  - `cargo test --workspace --no-fail-fast` = **1713 passed / 0 failed / 32 ignored**（ベースライン完全一致・増減ゼロ）。
- **増減内訳**: ±0。変更は `.vscode/launch.json`（cargo 無関与のエディタ設定）と `proposals.md`（仕様ドキュメント）のみで、cargo のビルド/テスト/成果物に影響しないため当然。1713/0/32 維持。
- **JSON 妥当性**: `.vscode/launch.json` を行コメント strip 後に `JSON.parse` 成功（JSONC として妥当）。構文非破壊。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy --workspace` は exit 非ゼロ（wintf lib が 20 errors + 112 warnings で停止）。上位カテゴリ: `collapsible_if`×68・`type_complexity`×30・**`not_unsafe_ptr_arg_deref`×20（error 級）**・`question_mark`(let-else)×8・`too_many_arguments`×6（8/7・11/7・15/7 含む）・`derivable_impls`×3 等。X1-T 記録と同傾向。
- `not_unsafe_ptr_arg_deref` の `error:` 表示は**clippy 既定の `#[deny(...)]`**（プロジェクト設定ではない。境界内設定ファイルに lint 設定不在を再確認）。raw ポインタ deref する pub 関数が `unsafe` 未マークという**ソースコード**条件で、COM/winproc 系（既レビュー領分）。
- 全件**ソースコード**の指摘で X1-S 境界（設定ファイルのみ）外。S2（build/test）は緑であり S3 はブロッカーとしない規約に従い記録のみ。境界内設定に是正すべき lint 構成はなし。

## flaky

- 既知フレーキー `wintf tests/ecs cue_performance_test::bench_pop_ready_empty_queue`（壁時計ベンチ負荷依存）は本セルの全量実行で初回から pass。**flaky 発生なし。** 再実行不要。
