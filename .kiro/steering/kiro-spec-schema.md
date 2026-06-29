---
inclusion: fileMatch
fileMatchPattern: '.kiro/specs/**'
updated_at: 2026-03-07
---

# Kiro spec.json スキーマ定義

本プロジェクトにおける `.kiro/specs/{feature}/spec.json` の拡張スキーマを定義する。

この文書は**厳格なバリデーター仕様**ではなく、現在のリポジトリで実際に使われているフィールドと、今後新規または更新対象の仕様で優先する値をまとめた運用ガイドである。`completed/`配下には旧フェーズ値を持つ履歴が残るため、履歴の完全正規化は前提にしない。

---

## 基本フィールド（全仕様共通）

```json
{
  "feature_name": "string",     // 必須: 仕様の一意識別子（ディレクトリ名と一致）
  "created_at": "ISO8601",      // 必須: 仕様作成日時
  "updated_at": "ISO8601",      // 必須: 最終更新日時
  "language": "ja|en",          // 必須: ドキュメント言語
  "phase": "string",            // 必須: 現在のフェーズ
  "approvals": { ... },         // 必須: 承認状態オブジェクト
  "ready_for_implementation": "boolean"  // 必須: 実装可能フラグ
}
```

---

## phase（フェーズ）の値

### 現在推奨する値

| 値 | 説明 |
| ---- | ---- |
| `init` | 初期化のみ |
| `requirements-draft` | 要件定義作成中 |
| `requirements-generated` | 要件定義生成済み・未承認 |
| `requirements-approved` | 要件定義承認済み |
| `design-generated` | 設計生成済み・未承認 |
| `design-approved` | 設計承認済み |
| `tasks-generated` | タスク生成済み・未承認 |
| `tasks-approved` | タスク承認済み（実装可能） |
| `in-progress` | 実装中 |
| `implementation` | 実装フェーズ中 |
| `implementation-complete` | 実装完了・完了処理対象 |
| `completed` | 完了済み履歴 |
| `survey-completed` | 調査完了（調査系仕様専用） |
| `rejected` | 却下 |

### 履歴上の旧値

以下は既存履歴で観測される値であり、参照時には受け入れるが、新規作成では原則使わない。

- `approved`
- `closed`
- `complete`
- `implemented`
- `implementation-completed`
- `not-started`
- `validated`

---

## approvals（承認状態）

```json
"approvals": {
  "requirements": {
    "generated": true,        // requirements.md 生成済みか
    "approved": false,        // 人間が承認したか
    "generated_at": "ISO8601",// 任意: 生成日時
    "approved_at": "ISO8601", // 任意: 承認日時
    "note": "string"          // 任意: メモ
  },
  "design": { /* 同構造 */ },
  "tasks": { /* 同構造 */ },
  "implementation": {
    "completed": false,
    "completed_at": "ISO8601",
    "validated": false,
    "validated_at": "ISO8601",
    "note": "string"
  }
}
```

- `implementation` オブジェクトは主に親仕様や完了済み履歴で使われる任意フィールド
- `generated_at` や `approved_at` は存在してもしなくてもよい

---

## 子仕様専用フィールド

親仕様を持つ仕様に追加するフィールド。

```json
{
  "parent_spec": "string",           // 親仕様の feature_name
  "parent_requirements": ["1.1", "1.2"],  // 対応する親要件ID
  "tier": 1,                         // 依存階層（1=基盤, 2=基盤依存, ...）
  "priority": "P0|P1|P2|P3",         // 優先度
  "dependencies": ["other-spec"]     // 依存する他仕様のリスト
}
```

### priority の定義

| 値 | 意味 | 説明 |
| ---- | ---- | ---- |
| `P0` | MVP必須 | 最小実行可能製品に必要 |
| `P1` | 体験向上 | UX向上のために重要 |
| `P2` | 高度機能 | 差別化機能 |
| `P3` | 将来展望 | 将来的な拡張 |

### tier の定義

| 値 | 説明 |
| ---- | ---- |
| 1 | 基盤層（他に依存しない） |
| 2 | 基盤依存層 |
| 3 | 上位層 |
| 4+ | さらなる上位層 |

---

## メタ仕様専用フィールド

子仕様を持つ親仕様（メタ仕様）に追加するフィールド。

```json
{
  "child_specifications": {
    "total": 32,                      // 子仕様の総数
    "note": "string",                 // 説明
    "categories": {                   // カテゴリ別分類
      "category_name": ["spec-1", "spec-2"]
    },
    "created": ["spec-1", "spec-2"],  // 作成済み子仕様
    "pending": 0                      // 未作成数
  }
}
```

---

## 特殊用途フィールド

### 調査系メタデータ

```json
{
  "note": "string",          // 仕様の目的説明
  "survey_summary": {        // 調査結果サマリー
    "archived_specs": 26,
    "active_specs": 6,
    "completed_since_last_survey": ["spec-1"],
    "high_priority_next": "spec-2"
  }
}
```

### プロセス支援仕様

```json
{
  "metadata": {
    "priority": "P0",
    "layer": "kiro",           // wintf|areka|kiro
    "category": "process-support",
    "description": "string"
  }
}
```

---

## 仕様タイプ別テンプレート

### 通常仕様（子仕様）

```json
{
  "feature_name": "wintf-P0-example",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z",
  "language": "ja",
  "phase": "requirements-draft",
  "parent_spec": "ukagaka-desktop-mascot",
  "approvals": {
    "requirements": { "generated": true, "approved": false },
    "design": { "generated": false, "approved": false },
    "tasks": { "generated": false, "approved": false }
  },
  "ready_for_implementation": false,
  "parent_requirements": ["1.1"],
  "tier": 1,
  "priority": "P0",
  "dependencies": []
}
```

### メタ仕様（親仕様）

```json
{
  "feature_name": "meta-example",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z",
  "language": "ja",
  "phase": "completed",
  "approvals": {
    "requirements": { "generated": true, "approved": true },
    "design": { "generated": true, "approved": true },
    "tasks": { "generated": true, "approved": true }
  },
  "ready_for_implementation": true,
  "child_specifications": {
    "total": 10,
    "categories": { "core": ["spec-1", "spec-2"] },
    "created": ["spec-1", "spec-2"],
    "pending": 0
  }
}
```

### スタンドアロン仕様

```json
{
  "feature_name": "standalone-example",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z",
  "language": "ja",
  "phase": "requirements-draft",
  "approvals": {
    "requirements": { "generated": true, "approved": false },
    "design": { "generated": false, "approved": false },
    "tasks": { "generated": false, "approved": false }
  },
  "ready_for_implementation": false
}
```

### 調査系仕様テンプレート

```json
{
  "feature_name": "future-requirements-survey",
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z",
  "language": "ja",
  "phase": "survey-completed",
  "approvals": {
    "requirements": { "generated": true, "approved": true },
    "design": { "generated": false, "approved": false },
    "tasks": { "generated": false, "approved": false }
  },
  "ready_for_implementation": false
}
```

---

## 命名規則

### feature_name パターン

```text
{layer}-{priority}-{feature-name}
```

| layer | 説明 |
| ------- | ------ |
| `wintf` | UIフレームワーク層 |
| `areka` | アプリケーション層 |
| `kiro` | 開発プロセス支援 |

例: `wintf-P0-animation-system`, `areka-P1-devtools`

---

## 更新ルール

1. **phase 変更時**: 必ず `updated_at` も更新
2. **承認時**: `approvals.{phase}.approved = true` + `approved_at` 設定
3. **子仕様作成時**: 親の `child_specifications.created` に追加、`pending` を減算
4. **新規または更新対象の仕様**: 可能な限り「現在推奨する値」に寄せる
5. **完了済み履歴**: 古い `phase` 値だけを理由に機械的な正規化はしない。必要な作業がある仕様だけ更新する
