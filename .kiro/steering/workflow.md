---
inclusion: always
updated_at: 2026-06-16
---

# Workflow - 開発ワークフロー

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

仕様が `.kiro/steering/roadmap.md`（ロードマップ正本）に記載されている場合、以下を更新する（参照タイミングは `.kiro/steering/focus.md`）。`doc/ROADMAP.md` はポインタ stub のため更新対象ではない。

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
- [ ] `.kiro/steering/roadmap.md`（正本）更新済み（該当する場合: Specs 状態 `[x]` + 完了数インクリメント）
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

## 二坑統合（先進坑・本坑）

> **二坑規律の詳細正本は `.kiro/steering/two-tunnel.md`（`inclusion: manual`）。本節は要約＋参照に留め、常駐コストを抑える。**

二坑モデル（先進坑＝pilot・使い捨て検証 / 本坑＝main・完成品）を既存の spec 駆動ワークフローへ上乗せする。最適化対象は手応えの速さでなく**誤った方向の可逆性**である。各規律の詳細・判断基準・記法は `two-tunnel.md` を権威とし、ここでは既存フローとの接点のみ示す。

### 先進坑フェーズと既存フローの関係

- 方向・実現可能性・手順が怪しい所だけ、本坑着手の**前**に先進坑（`crates/pilot/examples/<spec>/`）で確認する。よく分かっている所は先進坑を経ず直に本坑へ（掘りすぎ防止）。
- 本坑 spec は既存フロー（requirements → design → tasks → implementation → complete）を従来どおり辿る。先進坑はこのフローの**前段**に位置し、本坑 design は先進坑 README の検証結果を参照する（二重化しない）。
- 詳細: `two-tunnel.md`「先進坑/本坑の定義」「先進坑の一次記録（README 3 幕）」。

### go ハードゲート（本坑着手の前提条件）

- 各本坑 spec は、方向を確定する先進坑の **go 判定**を前提依存に持つ。go に至るまで当該本坑 spec は**着手不能（BLOCKED）**として扱う。
- go 判定は開発者が出力を見て下す**人間判断**であり、自動判定にはしない。spec 上の記法（`_Depends(confirmed): <pilot-spec>`）は `roadmap.md` の凡例と `two-tunnel.md` を参照。
- 詳細: `two-tunnel.md`「ハードゲート」。

### 依存マップ重点検証ルール

- spec 分解時（discovery / `/kiro-spec-batch`）に、先進坑⟷本坑の依存関係を**手動チェックリスト**で目視検証する（被覆・孤児なし・DAG・各エッジの合否基準明示）。いずれか満たさなければ当該本坑 spec を ready にしない。
- 自動チェックツールは設けない（対象グラフは小規模・合否は人間判断）。詳細: `two-tunnel.md`「依存マップ重点検証（手動チェックリスト）」。

### 削除/隔離規律

- **命綱（葉ノード隔離）**: 出荷グラフ上のいかなるクレートも先進坑コードに依存しない。
- **隔離保全**: 命綱が満たされている限り、先進坑コードは物理削除せず知見クレート `crates/pilot` へ隔離保全してよい（検疫所効果で production を常時クリーンに保つ）。
- **掘り直し禁止（コピペ donor 禁止）**: 本坑は先進坑の知見を「見てクリーンに掘り直す」。先進坑コードをコピペ流用しない。
- 詳細: `two-tunnel.md`「削除/隔離規律」。

### 先進坑の多重並列運用

- 先進坑は細粒度・独立（1 仕様 = 1 フォルダ）ゆえ多重並列で掘れる。並列実行には**既存の Agent / Workflow 機構**を運用で用い、新規の並列実行基盤は開発しない。

### 既存規約との整合（上乗せ・非置換）

二坑規律は上記の既存規約を**変更しない上乗せ**である：

- 「ブランチ＆マージ戦略（PR ベース・`main` 直 push 禁止）」は不変。先進坑・本坑とも同一ワークツリーブランチ上で進み、統合は完了時の単一 PR に集約する。
- 「実装完了時のアクション」「タスク完了時のアクション」「仕様フェーズフロー」は不変。二坑ゲート・隔離規律はこれらと整合し、置き換えない。
