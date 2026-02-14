# Workflow - 開発ワークフロー

Kiro仕様駆動開発における作業フローと完了時アクション。

---

## 実装完了時のアクション

仕様の実装が完了した際、以下の手順を **この順序で** 実行すること。

### Step 1. 実装コミット

ソースコード変更をコミットする（プッシュはまだ行わない）。

```bash
git add -A
git commit -m "<type>(<scope>): <summary>

<body>

Spec: <spec-name>"
```

**コミットタイプ**: `feat` / `fix` / `refactor` / `docs` / `test`

### Step 2. 仕様フォルダを `completed/` に移動

**移動を先に行い、移動後に `spec.json` を更新する。**
（VS Code の不具合により、移動前にファイルを更新すると、エディタの確定操作で移動元に復活する場合がある）

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

Phase 内の仕様完了状況に応じて進捗バーとパーセンテージを更新。

📍 **参照**: `.kiro/steering/focus.md` のROADMAP更新タイミング

### Step 5. 完了コミット & プッシュ

仕様移動とメタデータ更新をコミットし、**まとめてプッシュ**する。
（CIの無駄な多重実行を避けるため、プッシュは最後の1回のみ）

```bash
git add -A
git commit -m "chore(specs): <spec-name> を完了フォルダに移動"
git push origin <branch>
```

### 完了チェックリスト

すべての Step を実行した後、以下を確認する：

- [ ] 全テストがパス（`cargo test`）
- [ ] スペックフォルダが `.kiro/specs/completed/<spec-name>/` に存在
- [ ] `spec.json` の `phase` が `"implementation-complete"`
- [ ] 移動元（`.kiro/specs/<spec-name>/`）にファイルが残っていない
- [ ] ROADMAP更新済み（該当する場合）
  - 仕様の状態列が ✅ 完了 になっている
  - 完了済み仕様数がインクリメントされている

## 仕様フェーズフロー

```
requirements → design → tasks → implementation → implementation-complete
```

各フェーズ移行時に進捗を確認し、完了時は上記アクションを実行。

---
_Document patterns, not every workflow variation_
