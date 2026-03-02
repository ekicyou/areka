# 継続情報: dola-cue-pasta-dsl-extension 要件レビュー

> 作成日: 2026-03-02
> 前提: requirements.md v2, gap-analysis.md v2 を基に要件レビューを実施中

---

## 完了済みアクション

### 自明な修正（コミット済み）

| 内容 | コミット |
|------|----------|
| BarrierKind 名称を実コードに整合（All/Any/Explicit → WaitForInput/WaitForChoice/Timeout） | `d04a934` |

### ディスカッション済み議題

| # | 議題 | 結論 | コミット |
|---|------|------|----------|
| Q1 | 暗黙キーフレームの「所要時間」 | **外部注入アプローチ**: Duration Resolver トレイトを定義し CueSheet ビルダーに注入。パーサーは行の出現順序と構造のみ出力。dola 内で所要時間は確定しない。Req 2 AC 5-6 追加、gap-analysis R-1 解決済み | Q1コミット |
| Q2 | 未定義 `@command` の Emote フォールバック根拠 | **最頻出用途が Emote だから**。Req 4 設計注記に根拠追記 | `5446200` |

### 設計判断（design.md で詳細化する事項 — 変更不要）

| ID | 項目 |
|----|------|
| D-1 | `!` コマンド行の具体的 PEG 文法 (gap-analysis R-4) |
| D-2 | `@alias = Command(args)` の PEG 文法 (gap-analysis R-2) |
| D-3 | 実装アプローチ選択 A/B/C (gap-analysis R-5) |
| D-4 | CueCommand 記法の EN/JA 対応表 |
| D-5 | MVP フェーズ分割計画 |

---

## 未完了ディスカッション議題（ここから再開）

### Q3: 継続行（`:content`）の CueCommand::Text 挙動（Req 5.4, gap-analysis R-10）

**問題**: Req 5.4 は「直前のアクション行の `CueCommand::Text` に追加する」とあるが、「追加」の具体的意味が未確定。

**選択肢**:
- **A: 同一 `Cue` の `Text` に文字列結合** — `"前の行\n継続行"` のように1つの Text に連結。Cue 数は増えない。
- **B: 別の `Cue` として同一 `start_time` で生成** — 新しい Cue エントリを追加。

**補足**: 現行 pasta DSL では `ContinueAction` は「直前のアクション行への追記」として扱われており、A が自然に見える。

**開発者への質問**: A（文字列結合）と B（別 Cue）のどちらにするか？

---

### Q4: `%` 行不在時のデフォルトスロット（Req 6.6, gap-analysis R-7）

**問題**: `%` 行が存在しない場合、ActorKey のスロット番号はどうするか？

**選択肢**:
- **A: 出現順に 0, 1, 2... 自動割り当て**
- **B: `%` 行必須、なければエラー**
- **C: デフォルトスロット 0 割り当て**

**開発者への質問**: どの方針にするか？

---

### Q5: `CueCommand::Clear` 生成ポリシー（Req 5.5, gap-analysis R-9）

**問題**: Clear はいつ生成するか？

**選択肢**:
- **A: `!clear` 明示コマンドのみ** — スクリプト作者が明示的に書いた場合のみ
- **B: シーン遷移時に自動生成** — シーン終了時に全アクターに対して Clear を自動挿入
- **C: 両方** — 明示 `!clear` + シーン遷移時自動

**開発者への質問**: どの方針にするか？

---

### Q6: `RouteRemove` 発行条件（gap-analysis R-8）

**問題**: `RoutingCommand::RouteRemove` はいつ発行するか？

**選択肢**:
- **A: シーン終了時に自動** — シーン内で RouteAdd したものを自動 Remove
- **B: 明示 `!` コマンドのみ** — スクリプト作者が `!route_remove` 等で指定
- **C: 両方** — 明示 + シーン終了時自動クリーンアップ

**開発者への質問**: どの方針にするか？

---

### Q7: 1行内の複数 `@command` 処理（Req 5.6, gap-analysis R-6）

**問題**: `さくら：こんにちは@happy@sad` のように1行に複数 `@command` がある場合の処理。

**選択肢**:
- **A: 全て適用** — 出現順に CueCommand を生成（同一 start_time で複数 Cue）
- **B: 最後のみ適用** — 最後の `@command` のみ有効
- **C: エラー** — 複数 `@command` はパースエラー

**開発者への質問**: どの方針にするか？

---

## 全議題終了後の次ステップ

```
/kiro-spec-design dola-cue-pasta-dsl-extension
```

design.md リビルドでは以下を含める:
- `!` コマンド行の PEG/EBNF 文法
- エイリアス定義行の PEG/EBNF 文法
- Duration Resolver トレイト設計
- CueCommand 記法対応表（EN/JA）
- 実装フェーズ計画（MVP → Full）
- `cue.pasta` の v2 全面書き換え
