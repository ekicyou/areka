---
name: kiro-complete
description: 'Kiro仕様駆動開発のSpec完了ワークフローを実行する。DoDゲート検証→コミット→completedフォルダ移動→spec.json更新→参照パス修正（spec文書＋ソース全域）→ROADMAP更新→スキルドキュメント同期→最終コミット→移動後テスト再実行→PR作成→squashマージまでを中断なく完遂する。mainへの統合はこのPRが唯一の経路（直push禁止）。Use when: 実装完了を承認する, 承認してください, 完了を承認, spec承認, approve implementation, kiro承認完了。DO NOT USE when: 実装が完了したのみ（承認の明示がない場合）、タスクが終わっただけ'
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion
argument-hint: <feature-name>
---

# Kiro Spec 完了ワークフロー

## 発動条件（必須）

> **⚠️ このスキルは開発者の明示的「承認」がある場合にのみ発動する。**

### ✅ 発動する（承認の明示がある）
- 「実装完了を**承認**します」「**承認**してください」
- 「このspecを**承認**する」「**approve**」「kiro **承認**完了」

### ❌ 発動しない（承認の明示がない）
- 「実装が完了した」「タスクが全部終わった」
- 「spec完了」「アーカイブしてほしい」などの曖昧な表現のみ
- AIが自律的に「完了したと判断」した場合

### 承認が不明瞭なとき
発動を迷う場合は **発動しない**。必要なら開発者に確認する:
> 「実装完了の承認をいただけますか？承認いただいた場合、完了ワークフローを実行します。」

---

## いつ使うか
- 開発者が上記「発動する」に該当する承認を**明示的に**宣言したとき
- tasks.md の全タスクが `[x]` 完了している状態で使用
- 設計文書リフレッシュが完了した後

## 完了基準の権威（ベースライン内蔵 + 任意の workflow.md）

> **このスキルはベースラインの完了基準（DoD）・コミット規約を内蔵し、単体で機能する。**
> 完了基準（DoD）、コミット規約、ドキュメント更新判定の既定値はこのスキル内に定義する（下記）。
> **`.kiro/steering/workflow.md` が存在する場合のみ**、それを追加の権威として優先し、定義されたゲートをベースラインに上乗せする。存在しなければベースラインのみで完了できる。

## 哲学
- **中断せず一連で完遂する** — 全ステップを止めずに実行
- **VSCodeの変更ファイル確定挙動を回避** — spec.json編集は移動後に行う
- **ベースライン完了基準を内蔵** — DoD・コミット規約はこのスキルが既定値を持つ。`.kiro/steering/workflow.md` が存在すればそれを優先（任意）
- **mainへの統合はPRが唯一の経路** — フィーチャーブランチはハーネスのワークツリーが供給。1 feature = 1 branch = 1 PR。`{default-branch}` への直接 push は一切行わない
- **繰り返し仕様は移動しない** — 繰り返し実行型の仕様は常に `.kiro/specs/` 直下に留まる
- **移動はコードを壊し得る** — ソースが spec 文書を実ファイル読みしていればアーカイブ移動で壊れる。参照検索はソース全域まで及ぼし（ステップ5-2）、**移動をコミットした後にテストを再実行する**（ステップ7-2）

## 前提条件
- `.kiro/specs/{feature}/tasks.md` の全タスクが完了
- 設計文書と最終実装の整合確認済み

## 例外: 繰り返し仕様

リリース手順やレビュー・ループのような繰り返し実行型仕様は `completed/` に**移動しない**。

判定基準:
- spec.json や requirements.md に「繰り返し」「repeatable」「定期実行」「ループ」等の記述がある
- `/kiro-impl` のたびにタスクがリセットされる設計

繰り返し仕様の場合:
1. ステップ1（DoD検証）とステップ2（コミット）のみ実行
2. ステップ3〜6（移動・パス更新・ROADMAP）をスキップ（`completed/` へは移動しない）
3. ステップ8（リモート同期＝PR ベース）を実行
4. tasks.md のチェックボックスをリセット（全 `[x]` → `[ ]`）

---

## 手順

### ステップ0: 決定的解決（portable context）

リモート操作で用いる `{remote}` と `{default-branch}` を、**固定優先順序で1回だけ決定的に解決**する。各値はちょうど1つの結果（または明示的なスキップ）に収束させ、推測しない。解決した値は以降のステップ（特にステップ8）で `origin`/`main` のハードコードの代わりに再利用する。この優先順序は `kiro-start` の「Step 0: Resolve portable context」と整合している。

1. **デフォルトリモート（`{remote}`）**: `git remote` を実行し、以下の固定ルールを適用する。
   - `origin` が存在する → `{remote}` = `origin`。
   - そうでなく、リモートがちょうど1つだけ存在する → `{remote}` = そのリモート。
   - それ以外（リモートなし、または `origin` を含まない複数リモート）→ `{remote}` = none。リモート操作はすべてスキップ扱いとする（一度だけ警告する）。

   ```powershell
   $remotes = git remote
   if ($remotes -contains "origin") { $remote = "origin" }
   elseif (@($remotes).Count -eq 1) { $remote = $remotes }
   else { $remote = $null }  # none: リモート操作はスキップ
   ```

2. **デフォルトブランチ（`{default-branch}`）**: 以下の固定優先順序で決定的に解決する。
   - `{remote}` が解決済みなら、`git symbolic-ref --quiet --short refs/remotes/{remote}/HEAD` を読み、先頭の `"{remote}/"` プレフィックスを除去した名前。
   - それが空で、ローカルに `main` ブランチが存在する → `{default-branch}` = `main`。
   - そうでなく、ローカルに `master` ブランチが存在する → `{default-branch}` = `master`。
   - それ以外 → `{default-branch}` = 現在のブランチ。

   ```powershell
   $defaultBranch = $null
   if ($remote) {
     $defaultBranch = git symbolic-ref --quiet --short "refs/remotes/$remote/HEAD"
     if ($defaultBranch) { $defaultBranch = $defaultBranch -replace "^$remote/", "" }
   }
   if (-not $defaultBranch) {
     if (git show-ref --verify --quiet "refs/heads/main") { $defaultBranch = "main" }
     elseif (git show-ref --verify --quiet "refs/heads/master") { $defaultBranch = "master" }
     else { $defaultBranch = git branch --show-current }
   }
   ```

   `{default-branch}` は1つの具体的なブランチ名として確定し、以降で再評価しない。

> **以降のステップは、`origin`/`main` のハードコードではなく、ここで解決した `{remote}` / `{default-branch}` を用いる前提とする。** `{remote}` が none の場合、リモート同期（ステップ8）は安全にスキップし警告する。

### ステップ1: DoD（完了基準）ゲート検証

ベースラインの完了基準を順に検証する。判定ルールはこのスキルが内蔵する（下記）。**`.kiro/steering/workflow.md` が存在する場合のみ**読み込み、そこに定義された追加ゲートをベースラインに上乗せする（存在しなければベースラインのみで完了可）。

1. **任意**: `.kiro/steering/workflow.md` が存在すれば読み込み、追加の DoD ゲートを取り込む。存在しなければスキップ。
2. **ベースラインゲート**（最低限・常時検証）:
   - **Spec Gate**: 当該 spec の `tasks.md` が全タスク `[x]` 完了であること。
   - **Test Gate**: テストスイートが全通過していること（下記3）。
   - **License Gate**: 依存グラフのライセンス健全性を検証し、配布用の第三者謝辞を最新化すること（下記4。設定ファイルを持つリポジトリのみ）。
   - workflow.md がある場合は、そこで定義された追加ゲート（例: Doc / Steering 等）も順に検証する。
3. **Test Gate**: ワークスペース全体のテストを実行し、全通過を確認する。
   ```powershell
   cargo test --workspace 2>&1 | Select-String "test result:|FAILED|error\["
   ```
   - **スキップ可**: 直近のターンで `cargo test --workspace` が実行され `test result: ok` を確認済みで、その後にテスト対象コードの変更が無い場合は再実行を省略してよい。スキップ時は完了チェックリストに「(直近の実行結果により省略)」と注記する。
   - 判断に迷う場合は実行する。`kiro-verify-completion` スキルがある場合は、その fresh-evidence ゲートに委ねてもよい。
4. **License Gate**: MIT 配布を守るライセンス健全性ゲート。ルートに設定ファイルが存在する場合のみ実行し、無ければスキップする（ポータビリティ確保）。**この2コマンドを回す**:

   ```powershell
   # (a) 汚染ゲート: 強コピーレフト(GPL/LGPL/AGPL/MPL 等)や許可外ライセンスの混入を検出
   #     deny.toml が存在する場合のみ実行。許可外検出＝失敗ならワークフロー中断・開発者へ報告。
   cargo deny check

   # (b) 第三者謝辞の再生成: 配布バイナリ同梱用の attribution を最新化
   #     about.toml が存在する場合のみ実行。生成差分はステップ2/7のコミットに含める。
   cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md
   ```

   - **ツール未導入時**: `cargo install cargo-deny --locked` / `cargo install cargo-about --features cli --locked` で導入してから実行する。
   - **設定ファイル不在**: `deny.toml`・`about.toml` が無いリポジトリでは License Gate 全体をスキップし、チェックリストに「(設定不在により省略)」と注記する。
   - **謝辞に差分が出た場合**: 依存が変化した証跡。差分は後続のコミット（ステップ2/7）に自然に取り込まれるため、ここで個別コミットはしない。
5. **いずれかのゲートが失敗した場合**: ワークフローを中断し、開発者に報告。

### ステップ2: 未コミットファイルのコミット

実装中の変更をすべてコミットする。コミットメッセージ形式は下記のベースライン規約（`<type>({feature-name}): <要約>`）に従う。`.kiro/steering/workflow.md` が存在すればその規約を優先する。

```powershell
git add -A
git commit -m "<type>({feature-name}): 実装完了

- 変更の要約（箇条書き）"
```

### ステップ3: completedフォルダへの移動

specディレクトリをcompleted配下へ移動する。

```powershell
New-Item -ItemType Directory -Path ".kiro/specs/completed" -Force | Out-Null
Move-Item ".kiro/specs/{feature-name}" ".kiro/specs/completed/"
```

**重要**: この時点ではspec.jsonを**編集しない**。VSCodeが編集中のファイルを追跡しており、移動前に編集すると移動操作と競合してファイルが元の場所に復活する。

### ステップ4: spec.jsonのステータス更新

**移動完了後に** spec.json を更新する。areka の `spec.json` スキーマ（`.kiro/steering/kiro-spec-schema.md`）に従い、以下を変更:

```json
{
  "phase": "completed",
  "updated_at": "YYYY-MM-DDTHH:MM:SSZ"
}
```

- スキーマ規約: **`phase` 変更時は必ず `updated_at` も更新する**。
- 任意で `approvals.implementation` を更新してよい（`completed: true`, `completed_at: "YYYY-MM-DDTHH:MM:SSZ"`）。
- `"status"` フィールドは使用しない。`"phase": "completed"` で完了を表す。

### ステップ5: 参照パスの更新

このspecを参照している箇所のパスを更新する。**参照元は spec 文書だけではない。** ソースコードが spec 文書を実ファイルとして読んでいる場合、アーカイブ移動でその読み込みが壊れ、テストが赤くなる。

#### 5-1. spec 文書からの参照を検索

```powershell
Get-ChildItem ".kiro/specs" -Filter "*.md" -Recurse |
  Where-Object { $_.FullName -notlike "*completed*" } |
  Select-String -Pattern "{feature-name}" |
  Select-Object -ExpandProperty Path | Sort-Object -Unique
```

#### 5-2. ソースコードからの参照を検索（必須）

**5-1 だけではビルドを壊す参照を取りこぼす。** ソース全域（areka では `crates/**/*.rs`）を走査する。

```powershell
# (a) 本命: 移動する feature-name をソース全域から検索する
git grep -n "{feature-name}" -- crates

# (b) 衛生チェック: spec パス全般の参照も一度見ておく（他 spec の移動漏れが混ざっていることがある）
git grep -n "\.kiro/specs" -- crates
```

- **(a) を必ず実行する。** これが今回の移動で壊れ得る参照そのもの。
- (b) は補助。`.kiro/specs` という汎用語はスキル文書などが大量にヒットするため、**`crates`（ソース配下）に絞って**実行する。リポジトリ全域へ広げると kiro スキル自身のヒットに埋もれて使い物にならない。
- `git grep` はサブモジュールへ降りない。`vendors/` 配下は別リポジトリの spec を指しているので対象外。
- **ソースが `crates/` 以外にあるリポジトリ**では、そのソースルート（`src`・`lib`・`app` 等）へ読み替える。`.kiro/` と `doc/` を除いた追跡ファイル全域を走査してもよい。
- 検出がゼロでも「検索を実行した」ことをチェックリストに記録する。

#### 5-3. 検出した参照の仕分け（必須）

検出行を **2 種類に仕分ける**。この判定を省略しない。

| 種別 | 見分け方 | 対応 |
|---|---|---|
| **コメント内の参照**（無害） | `//!` `///` `//` `#` などのコメント行・docコメント・文章中の出典表記 | **放置可**。正確さのために更新してもよいが、しなくてもビルドは壊れない |
| **実ファイル読み**（ビルドを壊す） | パス文字列が `std::fs::read_to_string` / `include_str!` / `File::open` / `fs::read` などへ渡る | **必ず更新**する |

- **(a) は数十行ヒットするのが普通**（実装コードは spec を出典としてコメントで名指しするため）。ヒット数の多さに怯まず、下の絞り込みで実ファイル読みだけを取り出す。

**絞り込みは「コメント行を落とす」で行う。** 実ファイル読みのパスは必ずコード行（文字列リテラル）に現れるので、コメント行を除けば候補だけが残る。

```powershell
# ヒット行からコメント行を落とす。残った行が実ファイル読みの候補
git grep -n "{feature-name}" -- crates |
  Select-String -NotMatch -Pattern ':\d+: *(//|#|\*)'
```

- この絞り込みは areka の実例で検証済み: PR #114 の spec は (a) が 32 行ヒットするが、コメント行を落とすと**実ファイル読みの 1 行だけ**が残る。別の完了 spec では残りゼロ（＝全てコメント参照）になる。
- **読み込み API 名（`read_to_string` 等）での絞り込みは当てにならない。** パスが定数へ切り出されていると API 呼び出しと別行になり、空振りする（PR #114 がまさにこの形）。**空振りは「無い」の証明にならない。**
- 残った行が定数定義（例: `const PROCEDURE_RELATIVE_PATH: &str = "…";`）だった場合は、**定数名で再 grep して使用箇所まで追う**。

```powershell
git grep -n "PROCEDURE_RELATIVE_PATH" -- crates
```

- **コメント参照は放置してもテストは緑のまま。実ファイル読みだけがビルド／テストを赤にする。**
- 相対パス（`env!("CARGO_MANIFEST_DIR")` 起点の `../../.kiro/specs/…` など）も同様に対象。

#### 5-4. パスの一括置換

`.kiro/specs/{feature-name}/` → `.kiro/specs/completed/{feature-name}/`

対象は 5-1 の spec 文書と、5-3 で「実ファイル読み」に仕分けたソース行。

#### 5-5. 親仕様への完了マーク

親仕様のdesign.mdに完了ステータス（✅）を反映する（該当する場合）。

### ステップ6: 追加更新チェック

以下を実施する。`.kiro/steering/workflow.md` が存在すれば、その「実装完了時アクション」「ドキュメント保守」セクションの指示も併せて適用する。

#### 6-1. ROADMAP更新

`doc/ROADMAP.md` を確認し、完了したSpecが記載されているか判定する（参照タイミングは `.kiro/steering/focus.md`）。

**スコープ判定（優先順位）**:
1. `requirements.md` に明示的なROADMAP項目との紐付け記述がある場合
2. 開発者が直接指示した場合
3. `doc/ROADMAP.md` の仕様テーブルにこの feature-name が含まれる場合
4. 判断に迷う場合は開発者に確認

**スコープ内の場合**（`doc/ROADMAP.md` の areka 形式に従う）:
- **6-1-1. 仕様テーブルの状態列を更新**: 該当行のパスを `completed/{feature-name}` に、状態列を ⚪ 未着手 / 🔵 進行中 → ✅ 完了 に更新。
- **6-1-2. プログレスサマリーを更新**: 「完了済み仕様」を +1、該当すれば「アクティブ仕様(P0)」を −1。
- **6-1-3. フェーズ進捗率を更新**: Phase内の完了状況に応じて進捗バーとパーセンテージを更新（必要な場合）。

**スコープ外の場合**: スキップ。

#### 6-2. スキルドキュメント更新

変更領域に関連するスキルドキュメント（`.claude/skills/` 配下で当該機能を解説するもの）が存在すれば、整合性を確認・更新する。該当がなければスキップ。`.kiro/steering/workflow.md` に「スキルドキュメント更新検討」の指示があればそれに従う。

#### 6-3. ステアリング・ドキュメント更新

当該変更で陳腐化する steering（`.kiro/steering/*.md`）やドキュメントがあれば更新する。`.kiro/steering/workflow.md` に「ドキュメント保守 > 更新チェックリスト」があればそれに従う。

### ステップ7: 完了最終コミット

#### 7-1. コミット

移動・ステータス更新・参照パス修正・追加更新をコミットする。

```powershell
git add -A
git commit -m "chore({feature-name}): spec完了・アーカイブ"
```

#### 7-2. 移動後テストゲート（必須）

**アーカイブ移動をコミットした「後」に、テストスイートを再実行する。**

```powershell
cargo test --workspace 2>&1 | Select-String "test result:|FAILED|error\["
```

- **移動前の緑は、移動後の緑を保証しない。** ステップ1の Test Gate はアーカイブ移動より**前**に走るため、「全緑で完了」と「移動でテストが赤化」が同じ手順の中で両立してしまう。この構造的な穴は、移動後の再実行でしか塞げない。
- **このゲートはスキップしない。** ステップ1で「直近の実行結果により省略」を使った場合でも、ここは必ず実行する（省略可なのはステップ1側だけ）。移動というテスト対象への変更が、その間に挟まっている。
- **赤になった場合**: 原因はほぼ確実にステップ5の参照パス更新漏れ（ソースからの実ファイル読み）。ステップ5-2 / 5-3 に戻って仕分けをやり直し、修正を追加コミットしてからこのゲートを回し直す。**赤のままステップ8（PR 作成・マージ）へ進まない。**
- **テスト構成が無いリポジトリ**: `cargo test` に相当する構成が無ければスキップし、チェックリストに「(テスト構成不在により省略)」と注記する。

### ステップ8: リモート同期（PR ベース）

> **手順実体**: リモート同期は **PR（Pull Request）ベース**であり、本セクションがその手順実体である。`.kiro/steering/workflow.md` に同等のブランチ戦略が定義されていればそれを優先する。

> **前提**: 本ステップは `origin`/`main` をハードコードせず、ステップ0で解決した `{remote}` / `{default-branch}` を用いる。フィーチャーブランチ／ワークツリーは Claude Code（ハーネス）が供給しており、このスキルは自前でブランチ／ワークツリーを作成・削除しない。1つの feature = 1つのブランチ = 1つの PR とし、完了時に1回だけ PR を作成して squash マージする。**`{default-branch}` への直接 push は一切行わない。**

確認不要。現在のブランチを判定し、以下を中断なく実行する。

```powershell
$branch = git rev-parse --abbrev-ref HEAD   # 現在の作業ブランチ（ハーネス供給）
```

#### PR 可否判定

以下を**すべて満たす**ときのみ PR を作成・マージする（PR 可）:

1. 現在ブランチが `{default-branch}` 以外（非デフォルトブランチ）。
2. ステップ0で解決した `{remote}` が none でない（リモートあり）。
3. `gh` が認証済み（`gh auth status` が成功）。

いずれかが欠ける場合は **PR 不可** とし、下記「フォールバック（PR 不可時）」へ進む。

#### PR 可: PR 作成 → squash マージ → リモートブランチ削除

```powershell
# 1. 現在ブランチを push して PR を作成（base = {default-branch}, head = 現在ブランチ）
gh pr create --base {default-branch} --head $branch --title "<subject>" --body "<body>"

# 2. squash マージ（--squash 固定、--delete-branch でリモートブランチを API 削除）
#    --subject / --body は下記「squash メッセージ生成」に従って供給する
gh pr merge --squash --delete-branch --subject "<subject>" --body "<body>"
```

- **マージ成否はマージ API の結果のみで判定する。** `gh pr merge` の成否がマージ成否であり、それ以外の警告でマージ成功を覆さない。
- **リモートブランチ削除**: `gh pr merge --delete-branch` が **PR マージ成功後に** API でリモート feature ブランチを削除する。
- **ローカル後始末警告は非致命**: `--delete-branch` のローカル削除試行は、カレントワークツリーでブランチがチェックアウト中のため**ブロックされ警告を出す**ことがある。これは**非致命**でありマージ成功（API 結果）を覆さない。リモートブランチは API により削除済みである。
- **ローカルブランチ／ワークツリーの後始末はハーネスへ委譲**: このスキルは自分のワークツリー／カレントブランチを削除しない（構造的に不可）。ローカルブランチ・ワークツリーの teardown はハーネスがセッション/タスク境界で実施する。

**squash メッセージ生成**（`gh pr merge --squash` の `--subject` / `--body`）:
- 固定文言にせず、**分岐点以降のコミット履歴を要約**して作成する。
- 手順:
  1. `git log --no-merges --pretty=format:"%h %s%n%b" {default-branch}..HEAD`（= `merge-base..HEAD`）で分岐点以降の全コミットを取得
  2. 対象 spec の `requirements.md` / `design.md` のタイトル・概要も参照し意図を補強
  3. 以下の形へ再構成:
     - **subject**（`--subject`）: `<type>({feature-name}): <機能全体を1文で表す要約>`
     - **body**（`--body`）: 主な開発仕様・変更内容を箇条書き（3〜7項目目安）。関連コミットは統合し、`fixup`/typo/WIP 等の些末な履歴は集約・省略。個々のコミット羅列ではなく「何を・なぜ作ったか」の開発単位で再構成する。

#### フォールバック（PR 不可時）

現在ブランチが `{default-branch}` である / `{remote}` が none（リモートなし・オフライン）/ `gh` 未認証 のいずれかの場合:

- **警告を出力**し、PR 作成・push を**スキップ**する。
- ローカルコミットは**そのまま保持**して継続する。
- **`{default-branch}` への直接 push は一切行わない。**

#### 中断条件

PR の**作成またはマージ（API）が失敗**した場合（コンフリクト / mergeable でない / 権限不足等）は、**ブランチを削除せず**処理を中断し開発者へ報告する（復旧可能性を確保するため）。中断するのは**マージ API が失敗したとき**のみであり、`--delete-branch` のローカル削除警告（非致命）とは区別する。

---

## 完了チェックリスト

```
- [ ] DoD ベースラインゲート通過（Spec / Test / License。workflow.md が存在すれば追加ゲートも）
- [ ] cargo test --workspace 成功（または直近の実行結果により省略）
- [ ] License Gate: `cargo deny check` 成功 + `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md` で謝辞再生成（deny.toml/about.toml があるリポジトリ。無ければ「設定不在により省略」）
- [ ] 未コミットファイルをコミット済み（ステップ2）
- [ ] completedフォルダへ移動済み（ステップ3）※繰り返し仕様はスキップ
- [ ] spec.json の phase を "completed" に更新済み + updated_at 更新（ステップ4）※繰り返し仕様はスキップ
- [ ] 参照パス更新済み（ステップ5）※繰り返し仕様はスキップ
      - [ ] spec 文書の参照を検索（5-1）
      - [ ] ソース全域（`crates/**/*.rs` 等）を `git grep "{feature-name}"` で検索（5-2(a)。検出ゼロでも実施を記録）
      - [ ] 検出行を「コメント参照（放置可）／実ファイル読み（要更新）」に仕分け済み（5-3）
- [ ] doc/ROADMAP.md 更新済み（スコープ内の場合: 状態列✅ + 完了数インクリメント）
- [ ] スキルドキュメント同期済み（該当する場合）
- [ ] 完了コミット済み（ステップ7-1）
- [ ] **移動後テストゲート通過**（ステップ7-2。アーカイブ移動をコミットした後に `cargo test --workspace` を再実行。移動前の緑では代替不可・スキップ不可）
- [ ] ステップ0で `{remote}` / `{default-branch}` を決定的解決済み
- [ ] リモート同期完了（ステップ8、PR ベース。解決した `{remote}`/`{default-branch}` を使用）
      - PR 可（非デフォルトブランチ かつ `{remote}` あり かつ `gh` 認証あり）: `gh pr create --base {default-branch} --head <current>` → `gh pr merge --squash --delete-branch --subject … --body …`（メッセージは `merge-base..HEAD` 履歴を要約）。マージ成否は API 結果のみで判定し、`--delete-branch` のローカル削除警告は非致命として継続。リモートブランチは API 削除、ローカルブランチ／ワークツリーはハーネス teardown へ委譲
      - PR 不可（`{default-branch}` 上 / `{remote}` none / `gh` 未認証）: 警告して PR・push スキップ、ローカルコミット保持（`{default-branch}` への直接 push なし）
      - PR 作成／マージ（API）失敗: ブランチを残し中断・報告
```

---

## エラー回避

### VSCode変更確定問題
- **症状**: 移動したファイルが元の場所に復活する
- **対策**: spec.jsonは必ずステップ4（移動後）で編集。移動前に編集しない

### 参照パス更新漏れ
- **症状**: 後続specが旧パスで参照しファイルが見つからない
- **対策**: ステップ5-1 で `Select-String` による網羅的検索を実施

#### ソースコードからの spec 文書読み込みが壊れる（静かに赤くなる）
- **症状**: アーカイブ移動の後、コードが `std::fs::read_to_string` / `include_str!` などで spec 文書を実ファイルとして読んでいるテストが「ファイルが見つからない」で失敗する。DoDゲートは全緑だったのに、マージ後の `{default-branch}` が赤いままになる
- **原因（構造的に見えない）**: ステップ1の Test Gate は**移動より前**に走るため、移動でパスが変わってもその場では誰も気づかない。さらに spec 文書側の検索（ステップ5-1）はソースを見ないので、この参照は検索網に一度も掛からない
- **実例**: `crates/areka/src/placement/transition_signoff_procedure_tests.rs` の定数 `PROCEDURE_RELATIVE_PATH` が `.kiro/specs/areka-P0-dpi-transition-atomicity/signoff-procedure.md` を読んでいた。PR #114 の完了ワークフローで当該 spec が `completed/` へ移動したがパスが追随せず、`main` 上で 5 件のテストが赤のまま放置された（別作業で偶然踏むまで誰も気づかなかった）
- **対策**: ステップ5-2 のソース全域 grep と 5-3 の仕分けを必ず実施し、**ステップ7-2 の移動後テストゲートで実際に緑を確認する**。仕分けの要点は「**コメント参照は放置可・実ファイル読みだけがビルドを壊す**」

### コミット漏れ
- **症状**: pushしたが変更が反映されていない
- **対策**: 各コミット前に `git status --short` で確認

### テスト失敗時
- **症状**: `cargo test --workspace` が失敗
- **対策**: ワークフローを中断し開発者に報告。テスト修正後に再実行

### License Gate 失敗時
- **症状**: `cargo deny check` が失敗（許可外ライセンス・強コピーレフト混入を検出）
- **対策**: ワークフローを中断し開発者に報告。混入 crate と経路を提示し、依存の差し替え／除外か allowlist 妥当性の再検討を仰ぐ。**安易に allow を広げて通さない**（MIT 配布の前提が崩れる）
- **症状**: `cargo deny` / `cargo about` が未導入（`no such command`）
- **対策**: `cargo install cargo-deny --locked` / `cargo install cargo-about --features cli --locked` で導入後に再実行
- **症状**: `cargo about generate` が `failed to satisfy license requirements` で失敗
- **対策**: 出荷対象外の dev-dependency が混入していないか確認（`about.toml` の `ignore-dev-dependencies = true`）。真に出荷される新規ライセンスなら permissive 性を確認のうえ `about.toml` の `accepted` と `deny.toml` の `allow` を揃えて追記

### リモート同期関連（ステップ8 PR ベース）

#### PR 作成失敗
- **症状**: `gh pr create` が失敗（既存 PR との衝突 / push 権限不足 / ネットワーク等）
- **対策**: **ブランチを削除せず**中断して開発者へ報告。既存 PR がある場合はその PR を確認して再マージを検討

#### マージ不可（mergeable でない / API 失敗）
- **症状**: `gh pr merge --squash` が失敗（コンフリクトで mergeable=false / 必須チェック未通過 / 権限不足等）
- **対策**: **ブランチを削除せず**中断して開発者へ報告。コンフリクトは GitHub 上または別途解決のうえ再実行。中断判定は**マージ API の結果のみ**で行い、`--delete-branch` のローカル削除警告とは混同しない

#### gh 未認証 / リモートなし（PR 不可）
- **症状**: `gh auth status` が失敗、または `{remote}` が none
- **対策**: 警告を出力し PR・push をスキップ。ローカルコミットは保持して継続する。**`{default-branch}` への直接 push は行わない**

#### `{default-branch}` 上で承認された
- **症状**: 現在ブランチが `{default-branch}`（PR の head に使えない）
- **対策**: 警告を出力し PR・push をスキップ、ローカルコミット保持。通常はハーネス供給の非デフォルトブランチ上で完了する想定

#### `--delete-branch` のローカル削除警告
- **症状**: `gh pr merge --delete-branch` がローカルブランチ削除を試みてブロックされ警告を出す（カレントワークツリーでチェックアウト中のため）
- **対策**: **非致命として無視し継続**。リモートブランチは API で削除済み。ローカルブランチ／ワークツリーの後始末はハーネスのワークツリー teardown に委ねる（このスキルは自分のワークツリーを削除しない／できない）
