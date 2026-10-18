# 確かめの記録: debug 版の `dump_balloon`（件 2）と要件 2.7

## 結論

**件 2 は再現しない。** 今の main から作った debug 版で、claudia と emo2 の測る回の 10 回（各 5 回）がどれも ⑴ 絵で返った。測る回の絵の数は claudia 5/5、emo2 5/5。areka のコードは変えていない（要件 3.3）。タスク 5（再現したときの直し）には進まない。

要件 2.7 の `tools/call` `{"name":7}` は、2 体とも中継ごしに `-32602`（`invalid params for tools/call`）で答えた。

## 条件

| 項目 | 値 |
|---|---|
| 走らせた日 | 2026-10-18 |
| areka の版 | `0.0.3`（ワークスペースの `Cargo.toml`） |
| 元のコミット | `6926871a`（本ブランチ。件 1 の直しと台本を含む） |
| main との分かれ目 | `e2370df8`（`git merge-base HEAD origin/main`・棚卸㉗ PR#334） |
| 分かれ目からの差 | 下の `git diff --stat`。`crates/areka/src/mcp/` には触れていない |
| ビルド | debug 版（`target\debug\areka.exe`・`areka-mcp-bridge.exe`）と i686 debug 版の `shiori-host32-helper.exe`（`areka.exe` の隣に写し、PE の機械種別 `0x014c` を確かめた） |
| ゴーストとバルーン | claudia＋バルーン `claudia`・emo2＋バルーン `emo2-kakukaku`（`nar-sample-path` の置き場。`target\nar-samples\manual\` の下） |
| 呼び方 | 中継ごし（標準入出力）・`initialize` の版 `2025-06-18`・`dump_balloon` の `scope` 0 |
| 上限 | 受け口 10 秒・中継 30 秒・台本の読み 40 秒・最初の絵を待つのは 60 秒まで |
| 置き場 | すべてワークツリーの `target\` の下（作業の置き場 `target\mcp-receiver-bugs\<ゴースト>\`・`AREKA_PROFILE_DIR` もその下） |
| 環境 | `RUST_LOG=warn,areka::mcp=debug,areka_mcp=debug`・`NO_COLOR=1`・`AREKA_NO_ALERT=1`・`AREKA_APP_SMOKE_EXIT_MS=360000`・`AREKA_ROOT` と `AREKA_USER_ROOT` は消した |
| 起こし方 | PowerShell 7 から `pwsh -NoProfile -File <絶対パス>\debug-dump-balloon.ps1`（既定の引数: `-Ghost claudia,emo2 -Calls 5 -Tool dump_balloon`） |
| 終了 | 2 体とも areka は自動終了で終了コード 0・中継も 0・台本が止めた子は無し・台本の終了コード 0 |

分かれ目からの差（`git diff --stat e2370df8 HEAD` の要約。spec の文書は除く）:

```
 .kiro/specs/areka-P0-mcp-receiver-bugs/verification/debug-dump-balloon.ps1
 crates/areka-mcp-bridge/tests/real_receiver_test.rs   |   6 +-
 crates/areka-mcp/src/handler.rs                       |  42 +-
 crates/areka-mcp/src/handler_tests.rs                 |  39 ++
 crates/areka-mcp/src/server_protocol_tests.rs         | 115 ++++++
 doc/ssp-mcp/bridge-diff-areka.md                      |   2 +-
```

## 結果

分類: ⑴ 絵 ⑵ 断り ⑶ 上限の答え ⑷ 中継の時間切れ ⑸ 返らなかった。

### claudia

| 段 | 回 | 呼んだもの | 所要 | 分類 |
|---|---|---|---|---|
| 2.7 | 1 | `tools/call` `{"name":7}` | 53 ms | `-32602` `invalid params for tools/call` |
| 待ち | 1 | `dump_balloon` | 58 ms | ⑴ 絵（データ 7,156 字） |
| 測る | 1 | `dump_balloon` | 65 ms | ⑴ 絵（7,156 字） |
| 測る | 2 | `dump_balloon` | 62 ms | ⑴ 絵（7,156 字） |
| 測る | 3 | `dump_balloon` | 119 ms | ⑴ 絵（7,156 字） |
| 測る | 4 | `dump_balloon` | 79 ms | ⑴ 絵（7,156 字） |
| 測る | 5 | `dump_balloon` | 298 ms | ⑴ 絵（7,432 字） |

測る回の絵: 5/5。

### emo2

| 段 | 回 | 呼んだもの | 所要 | 分類 |
|---|---|---|---|---|
| 2.7 | 1 | `tools/call` `{"name":7}` | 17 ms | `-32602` `invalid params for tools/call` |
| 待ち | 1 | `dump_balloon` | 1,774 ms | ⑴ 絵（データ 9,460 字） |
| 測る | 1 | `dump_balloon` | 115 ms | ⑴ 絵（9,460 字） |
| 測る | 2 | `dump_balloon` | 160 ms | ⑴ 絵（9,460 字） |
| 測る | 3 | `dump_balloon` | 106 ms | ⑴ 絵（9,460 字） |
| 測る | 4 | `dump_balloon` | 170 ms | ⑴ 絵（9,460 字） |
| 測る | 5 | `dump_balloon` | 157 ms | ⑴ 絵（9,460 字） |

測る回の絵: 5/5。

## 走らせ直した経緯

1 回目の走行（同じ日）は 2 体とも台本の終了コード 2（条件を作れない）で終わり、数えていない。台本が `AREKA_ROOT`・`AREKA_USER_ROOT` を消すつもりで空の値を置いており（PowerShell は `string` の引数へ `$null` を `""` で渡す）、areka が「置き場が決まらない」で起動を断っていた。台本を直して（`[NullString]::Value` で消す・想定外の例外は終了コード 2）回し直したのが上の結果。

## 添えるもの

- 台本: [debug-dump-balloon.ps1](debug-dump-balloon.ps1)
- 各回の生の結果は走行のたびに `target\mcp-receiver-bugs\<ゴースト>\results.json` に書かれる（`target\` の下なので記録には含めない）。
