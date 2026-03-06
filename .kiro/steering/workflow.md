# Workflow - 開発ワークフロー

updated_at: 2026-03-07

Kiro仕様駆動開発における作業フローと完了時アクション。

---

## 実装完了時のアクション

仕様の実装が完了した際、以下の手順を **この順序で** 実行すること。

### Step 0. リモートから最新を取得

コンフリクトを避けるため、作業開始前にリモートの最新状態を取得する。

```bash
git pull origin <branch>
```

### Step 1. 実装コミット

ソースコード変更をコミットする（プッシュはまだ行わない）。

```bash
git add -A
git commit -m "<type>(<scope>): <summary>

<body>

Spec: <spec-name>"
```

**コミットタイプ**: `feat` / `fix` / `refactor` / `docs` / `test`

### Step 2. 仕様フォルダーを `completed/` に移動

**移動を先に行い、移動後に `spec.json` を更新する。**
（VS Codeの不具合により、移動前にファイルを更新すると、エディターの確定操作で移動元に復活する場合がある）

```bash
mv .kiro/specs/<spec-name> .kiro/specs/completed/
```

### Step 3. `spec.json` の `phase` を更新

**移動後のパスで** `spec.json` を編集する。

- `phase` → `"implementation-complete"`
- `updated_at` → 現在日時

### Step 4. ROADMAP更新（該当する場合）

仕様が `doc/ROADMAP.md` に記載されている場合、以下を更新する：

#### 4-1. 仕様テーブルの状態列を更新

該当する仕様の行を見つけ、状態列を更新：
- ⚪ 未着手 / 🔵 進行中 → ✅ 完了

```markdown
例:
| ├ マルチウィンドウイベント | `completed/multiwindow-event-validation` | ✅ 完了 | |
```

#### 4-2. プログレスサマリーを更新

- **完了済み仕様**: カウントをインクリメント（+1）
- **アクティブ仕様(P0)**: 該当する場合デクリメント（-1）

```markdown
例:
**完了済み仕様**: 58件 / **アクティブ仕様(P0)**: 14件 / **バックログ(P1-P3)**: 18件
    ↓
**完了済み仕様**: 59件 / **アクティブ仕様(P0)**: 13件 / **バックログ(P1-P3)**: 18件
```

#### 4-3. 必要に応じてフェーズ進捗率を更新

Phase内の仕様完了状況に応じて進捗バーとパーセンテージを更新。

📍 **参照**: `.kiro/steering/focus.md` のROADMAP更新タイミング

### Step 5. 完了コミット & プッシュ

仕様移動とメタデータ更新をコミットし、**まとめてプッシュ**する。
（CIのムダな多重実行を避けるため、プッシュは最後の1回のみ）

```bash
git add -A
git commit -m "chore(specs): <spec-name> を完了フォルダに移動"
git push origin <branch>
```

### 完了チェックリスト

すべてのStepを実行した後、以下を確認する：

- [ ] 全テストがパス（`cargo test`）
- [ ] スペックフォルダーが `.kiro/specs/completed/<spec-name>/` に存在
- [ ] `spec.json` の `phase` が `"implementation-complete"`
- [ ] 移動元（`.kiro/specs/<spec-name>/`）にファイルが残っていない
- [ ] ROADMAP更新済み（該当する場合）
  - 仕様の状態列が✅完了になっている
  - 完了済み仕様数がインクリメントされている

---

## タスク完了時のアクション

個別タスク（実装の一部）が完了した際、以下の手順を実行すること。

### Step 1. リモートから最新を取得

他の変更とのコンフリクトを避けるため、コミット前にリモートの最新状態を取得する。

```bash
git pull origin <branch>
```

### Step 2. タスク関連ファイルをコミット

完了したタスクに関連するファイルのみをコミットする。

```bash
git add <関連ファイル>
git commit -m "<type>(<scope>): <task-summary>

<詳細説明>

Task: <task-id> in Spec: <spec-name>"
```

**コミットタイプ**: `feat` / `fix` / `refactor` / `docs` / `test`

### Step 3. リモートにプッシュ

変更をリモートリポジトリに反映する。

```bash
git push origin <branch>
```

### タスク完了チェックリスト

- [ ] 関連する単体テストがパス
- [ ] コミットメッセージにTask IDとSpec名を記載
- [ ] リモートにプッシュ完了

---

## 仕様フェーズフロー

```text
requirements → design → tasks → implementation → implementation-complete
```

各フェーズ移行時に進捗を確認し、完了時は上記アクションを実行。

---
Document patterns, not every workflow variation.
