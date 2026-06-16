# Workflow - 開発ワークフロー

updated_at: 2026-06-16

Kiro仕様駆動開発における作業フロー・ブランチ戦略・完了時アクション。

---

## ブランチ＆マージ戦略（PRベース）

> **mainへの統合はPull Requestが唯一の経路。`main`（`{default-branch}`）への直接 push は一切行わない。**

- **ブランチ供給元**: フィーチャーブランチは Claude Code（ハーネス）のワークツリーが供給する。skillは自前でブランチ／ワークツリーを作成・削除しない。
- **1 feature = 1 branch = 1 PR**: 1つの仕様の全フェーズ（requirements → design → tasks → implementation）を**同一のワークツリーブランチ**で進め、完了時に**1回だけ** PR を作成して squash マージする。
- **入口**: `/kiro-start {feature}` — 要件ディスカバリ後の単一spec開始。ブランチは作らず、現在のワークツリーブランチで spec を初期化（push しない）。デフォルトブランチ上では STOP し、ワークツリーでの再実行を促す。
- **出口**: `/kiro-complete {feature}` — DoDゲート検証 → アーカイブ → PR作成 → squash マージ。**mainへの統合はここだけ。**
- **直push禁止**: 各タスク・各フェーズの途中で `main` へ fast-forward / 直 push しない。途中コミットはワークツリーブランチ上に積み、統合は完了時のPRに集約する。

---

## 実装完了時のアクション

仕様の実装が完了し、**開発者が明示的に「承認」した**際は、`/kiro-complete {feature}` を実行する（このスキルが以下を中断なく完遂する）。手順実体は `.claude/skills/kiro-complete/SKILL.md` を権威とし、本節はその要約。

### Step 0. portable context 解決

`{remote}` / `{default-branch}` を固定優先順序で決定的に解決する（`origin`/`main` をハードコードしない）。

### Step 1. DoDゲート検証

- **Spec Gate**: 当該 spec の `tasks.md` が全 `[x]` 完了。
- **Test Gate**: `cargo test --workspace` 全通過（直近の実行結果が `test result: ok` で以降コード変更がなければ省略可）。
- いずれか失敗時はワークフローを中断し開発者へ報告。

### Step 2. 実装コミット

ソースコード変更をコミットする（mainへ直 push はしない）。

```bash
git add -A
git commit -m "<type>(<scope>): <summary>

<body>

Spec: <spec-name>"
```

**コミットタイプ**: `feat` / `fix` / `refactor` / `docs` / `test`

### Step 3. 仕様フォルダーを `completed/` に移動

**移動を先に行い、移動後に `spec.json` を更新する。**
（VS Codeの不具合により、移動前にファイルを更新すると、エディターの確定操作で移動元に復活する場合がある）

```bash
mv .kiro/specs/<spec-name> .kiro/specs/completed/
```

> **繰り返し仕様の例外**: リリース手順・レビューループ等の繰り返し実行型仕様は `completed/` に**移動しない**（`.kiro/specs/` 直下に留め、tasks.md のチェックボックスをリセット）。

### Step 4. `spec.json` の `phase` を更新

**移動後のパスで** `spec.json` を編集する（`.kiro/steering/kiro-spec-schema.md` に準拠）。

- `phase` → `"completed"`
- `updated_at` → 現在日時（**phase 変更時は必ず updated_at も更新**）
- 任意: `approvals.implementation.completed = true` / `completed_at` を設定

### Step 5. 参照パスの更新

他specや親仕様がこのspecを参照している場合、`.kiro/specs/<spec-name>/` → `.kiro/specs/completed/<spec-name>/` に一括置換する（`Select-String` で網羅検索）。親仕様の design.md に完了マーク（✅）を反映。

### Step 6. ROADMAP更新（該当する場合）

仕様が `doc/ROADMAP.md` に記載されている場合、以下を更新する（参照タイミングは `.kiro/steering/focus.md`）。

#### 6-1. 仕様テーブルの状態列を更新

該当行のパスを `completed/<spec-name>` に、状態列を更新：
- ⚪ 未着手 / 🔵 進行中 → ✅ 完了

```markdown
例:
| ├ マルチウィンドウイベント | `completed/multiwindow-event-validation` | ✅ 完了 | |
```

#### 6-2. プログレスサマリーを更新

- **完了済み仕様**: カウントをインクリメント（+1）
- **アクティブ仕様(P0)**: 該当する場合デクリメント（-1）

#### 6-3. 必要に応じてフェーズ進捗率を更新

Phase内の仕様完了状況に応じて進捗バーとパーセンテージを更新。

📍 **参照**: `.kiro/steering/focus.md` のROADMAP更新タイミング

### Step 7. 完了コミット

仕様移動・メタデータ更新・参照パス修正・ROADMAP更新をコミットする。

```bash
git add -A
git commit -m "chore(specs): <spec-name> を完了フォルダに移動"
```

### Step 8. リモート同期（PRベース）

> **mainへの統合はこのPRが唯一の経路。`{default-branch}` への直接 push は行わない。**

**PR 可否判定**（すべて満たすとき PR 可）:
1. 現在ブランチが `{default-branch}` 以外
2. `{remote}` が none でない
3. `gh` が認証済み

**PR 可**: 現在ブランチを push して PR を作成し、squash マージする。

```bash
gh pr create --base <default-branch> --head <current-branch> --title "<subject>" --body "<body>"
gh pr merge --squash --delete-branch --subject "<subject>" --body "<body>"
```

- squash メッセージは `merge-base..HEAD` のコミット履歴を要約して生成する（固定文言にしない）。
- マージ成否は**マージ API の結果のみ**で判定。`--delete-branch` のローカル削除警告は**非致命**（リモートブランチは API 削除済み、ローカルの後始末はハーネス teardown へ委譲）。

**PR 不可**（デフォルトブランチ上 / リモートなし / `gh` 未認証）: 警告し PR・push をスキップ。ローカルコミットは保持（**mainへ直 push しない**）。

**中断条件**: PR の作成・マージ（API）が失敗した場合は**ブランチを削除せず**中断し開発者へ報告。

### 完了チェックリスト

- [ ] DoDゲート通過（Spec / Test）
- [ ] 全テストがパス（`cargo test --workspace`、または直近の実行結果により省略）
- [ ] スペックフォルダーが `.kiro/specs/completed/<spec-name>/` に存在（繰り返し仕様を除く）
- [ ] `spec.json` の `phase` が `"completed"` + `updated_at` 更新済み
- [ ] 移動元（`.kiro/specs/<spec-name>/`）にファイルが残っていない
- [ ] 参照パス更新済み
- [ ] `doc/ROADMAP.md` 更新済み（該当する場合: 状態列✅ + 完了数インクリメント）
- [ ] 完了コミット済み
- [ ] PRベースでmainへ統合済み（PR可の場合）／ローカル保持（PR不可の場合）

---

## タスク完了時のアクション

個別タスク（実装の一部）が完了した際、以下を実行する。**コミットはワークツリーブランチ上に積み、`main` へ直 push しない**（統合は完了時の単一PRに集約）。

### Step 1. タスク関連ファイルをコミット

完了したタスクに関連するファイルのみをコミットする。

```bash
git add <関連ファイル>
git commit -m "<type>(<scope>): <task-summary>

<詳細説明>

Task: <task-id> in Spec: <spec-name>"
```

**コミットタイプ**: `feat` / `fix` / `refactor` / `docs` / `test`

### Step 2. （任意）フィーチャーブランチを push

バックアップ／CI 目的でワークツリーブランチをリモートの**同名ブランチ**へ push してよい。**`{default-branch}` へは push しない。**

```bash
git push <remote> HEAD
```

### タスク完了チェックリスト

- [ ] 関連する単体テストがパス
- [ ] コミットメッセージにTask IDとSpec名を記載
- [ ] `main` へ直 push していない（統合は完了時のPRのみ）

---

## 仕様フェーズフロー

```text
discovery → start(requirements) → design → tasks → implementation → complete(PRマージ)
```

```text
requirements → design → tasks → implementation → completed
```

各フェーズ移行時に進捗を確認し、完了時は `/kiro-complete` を実行。フェーズはすべて**同一のワークツリーブランチ**上で進行し、`main` への統合は完了時のPRに一本化する。

---
Document patterns, not every workflow variation.
