# サブエージェントへ渡す文（そのまま埋めて使う）

`Agent` ツールの `subagent_type` にエージェント名を入れる。自分のモデルがシステムプロンプトの
「You are powered by the model named …」で Fable 系、またはその行が見つからないときは
`model: "opus"` も一緒に渡す（自分が既に Opus 以下なら `model` を省く＝継承・要件 1.13）。

返ってきた**最初の 1 行**が `[agent-model] <名前>` で、`<名前>` に `opus` が含まれない
（大小無視）ときは、その周の記録の `reason` へ `agent-model-warning:<エージェント名>=<名前>`
を足して**続行する**（止めない・設計 C3）。

エージェントの返答は、下に挙げた見出しと鍵の行だけを読む。それ以外の文は台帳へ写さない。

---

## perf-measure（RANK 相・DECIDE 相）

```
goal: <goal>
mode: rank            # DECIDE 相では compare
run_dir: <順位付けの走行ディレクトリ>     # rank のとき
iteration: <n>                            # compare のとき
`## Measure` ブロックだけを返してください。順位表の全文は貼らないでください。
```

読む鍵: `RESULT` / `STAGE1_PROCESS` / `STAGE2_THREAD` / `STAGE3_FUNCTION` / `STAGE4_PHASE` /
`RANK_TOP`（rank）、`RESULT` / `COMPARE` / `SECONDARY`（compare）。

## perf-analyze（SELECT 相）

```
goal: <goal>
iteration: <n>
rank_txt: <rank.txt の絶対パス>
ledger: <loop-ledger.md の絶対パス>
design: .kiro/specs/<spec>/design.md   # 候補の中身は C16〜C20
順位表の最上位から順に候補カタログへ引き当て、除外の規則（Out of scope／担当 spec が
稼働中／既に試して差なし・悪化／信号が弱い）を当てて 1 つだけ選んでください。
選ばなかったものは全部 SKIPPED に理由つきで挙げてください。
`## Analysis` ブロックだけを返してください。
```

読む鍵: `HYPOTHESIS` / `CANDIDATE` / `FILES` / `PLAN` / `TESTS` / `SIZE` / `RISK` /
`SPEC_CHECK` / `HANDOFF` / `SKIPPED` / `NEXT_IF_REJECTED`（候補なしは `CANDIDATE: none`＋`PLATEAU: yes`）。

## perf-implement（IMPLEMENT 相・TOOLFIX 相）

```
goal: <goal>
iteration: <n>
<perf-analyze の ## Analysis ブロックをそのまま貼る>
1 周 1 変更で実装し、判断分岐に決定論テストを兄弟ファイルへ足し、対象 crate の
テストまで緑にしてください。Cargo.toml に触らないでください。git の破壊的操作を
しないでください（コミットも差し戻しも呼び出し側が行います）。
`## Implementation` ブロックだけを返してください。
```

TOOLFIX 相のときは `## Analysis` の代わりに次を渡す。

```
道具が壊れています。直してください（実行体のコードは触らないでください）。
failed_command: pwsh -NoProfile -File tools/perf/perf-loop.ps1 <sub> -Goal <goal> …
exit_code: <n>
stdout_tail: <末尾 20 行>
直したら `pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest -Goal <goal>` が
緑になることを確かめてください。
```

読む鍵: `STATUS` / `FILES_CHANGED` / `TESTS_ADDED` / `TESTS_RUN` / `CARGO_TOML` / `GIT` /
`NOTES` / `BLOCKER`。

## perf-review（TEST 相）

```
goal: <goal>
iteration: <n>
この作業ツリーの未コミットの差分を検査してください。
`## Review Verdict` ブロックだけを返してください。
```

読む鍵: `VERDICT` / `CHECKS` / `FILES_REVIEWED` / `FINDINGS` / `REMEDIATION`。
