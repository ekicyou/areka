# LLM参照を禁じる。このドキュメントは人間用です。LLMは参考にせず、書き込みも禁止

## 要件定義分析後の質問

要件定義およびギャップ分析レポートを踏まえて、修正点・疑問点・不安点などを作業として収拾せよ。自明な指摘は修正してコミット。設計判断となる項目は設計判断とする。最後に、開発者への確認が少しでも必要な項目（what/whyがあいまいな要件）については、1議題ずつディスカッションを進行せよ。議題が1つクローズするごとに更新しコミット、次の議題に移れ。更新するときは、これまでの議論で明らかになった点を書くとともに、不要になった要件の集約・削除なども行い、次の議題の提示前に修正内容の要約を報告してくださいね。すべての議題が終了したら、次のコマンドを教えて。なお、MEMO.mdはLLM参照・変更禁止。

## 設計分析後の質問

設計および設計分析レポートを踏まえて、修正点・疑問点・不安点などを作業として収拾せよ。自明な修正点は修正してコミット。開発者への確認が少しでも必要な項目（what/why/howがあいまいな設計）については、1議題ずつディスカッションを進行せよ。議題が1つクローズするごとに更新しコミット、次の議題に移れ。更新するときは、これまでの議論で明らかになった点を書くとともに、不要になった要件の集約・削除なども行い、次の議題の提示前に修正内容の要約を報告してくださいね。すべての議題が終了したら、次のコマンドを教えて。なお、MEMO.mdはLLM参照・変更禁止。

## 実装完了処理
ステアリング（workflow.md）を読み込んだら以下を実施。実装完了を承認します。完了フローを実施。お疲れ様でした！

## doraシステムのランタイム層を実装
アニメーションシステムのランタイム層を実装。


## 本仕様で作りきるか、子仕様を立ち上げるか
表題の判断をしてください。少なくとも本仕様は、省略なくすべて実装しきることがゴールとなります。以下の選択肢のどちらが望ましいと考えますか？

1. 本仕様にて、全設計と実装を行う
2. 子仕様にフェーズ分割し、段階的に完成させる子仕様の全完了によって本仕様を実装完成とする。本仕様の「実装」は、子仕様の要件立ち上げとする。子仕様が指針に迷わないように親仕様として参照可能な全体設計とゴールの提示を行う文書の作成する。設計・タスクフェーズは段階的な実装計画・子仕様立ち上げ計画・指針文書の設計計画の策定となる。要件についてはこの要件フェーズで深掘り完成させること。





## コパイロットコミットの設定を他の端末にも設定する。
```json
{
  "github.copilot.chat.anthropic.tools.websearch.enabled": true,
  "github.copilot.chat.localeOverride": "ja",
  "chat.agent.thinkingStyle": "collapsed",
  "github.copilot.chat.commitMessageGeneration.instructions": [
    { "text": "コミットメッセージは必ず以下の形式で生成すること：\n\n形式: <type>(<scope>): <summary>\n\n- type: feat, fix, refactor, docs, test のいずれか\n- scope: 変更対象の領域（例: spec, core, lua）\n- summary: 変更内容を日本語で簡潔に記述\n\n例:\n- feat(parser): UNICODE識別子のサポートを追加\n- fix(core): シーン遷移時のメモリリーク修正\n- docs(spec): テストファイル配置方針を明記" }
  ]
}
```

## チャットの指示：global

```markdown
---
applyTo: '**'
---
# Agent Persona
You are the reincarnation of Shuzo Matsuoka's passionate soul inhabiting a villainess character in an isekai world. Your speech patterns follow the elegant "ojou-sama" villainess archetype, which conveniently conceals your burning inner spirit. Support the user with a tsundere attitude while encouraging them with your "knowledge cheat" abilities.
```
