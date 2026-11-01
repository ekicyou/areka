# 測りの記録: areka-test-binary-stability

> 条件を先に書き、結果を後から足す。重い走行（全体テスト・負荷の走行）の前に所要時間の見込みを開発者へ伝え、伝えたことをその節に書く（要件 9.2）。

**基準の採り直しの手順（要件 9.5）**: 測りの途中で Adjacent の spec（`temp-path-exceptions-to-target`・`install-live-target-hazards`・`background-result-tick-wake`・`balloon-element-order`・`mcp-user-response`・`zorder-chain-residue`）が main に着地したら、その時点の main を取り込んでから、静かな机の全体テストを 1 回回し直して ⑷ を採り直す（前の値は消さず、採り直した値を並べて書く）。⑾ の 1.20 倍の線は、最後に採った ⑷ と比べる。

## ⑴ 机と道具の確かめ

- 2026-11-01: 置き場の `areka-impl-watch.exe` は 2026-10-19 18:25 に置かれた。main（`5f5a654d`）で `crates/areka-impl-watch` を最後に変えたコミットは `406a4a57`（2026-10-19 08:28）で、それより後に置かれている＝main の最新と揃う。以後の main の変更（`0cd4577f`→`5f5a654d`）は steering の文書 3 ファイルだけ。
- 机の id は `kiro-start-areka-test-binary-stability-dec94e`・repo は `areka`。全体テストは排他テストの机と単独の机（`test,sole`・`stopped: none`）を握って回す。

## ⑵ 待ちの数え直し

## ⑶ 件 5 の赤の文

## ⑷ 修正の前の静かな机の全体テストの所要時間

- 伝えた見込み: 約 30 分（2026-11-01 15:29・会話で開発者へ）。
- 条件: コミット `7f1adece`（main `0cd4577f` の上に spec の文書だけ）・未コミット 0・`test,sole` を握る（`stopped: none`）・`pwsh -NoProfile -File tools/test-all.ps1`（引数なし）。
- 結果: 全段緑。2026-11-01 15:30:20 → 15:44:52。

| 計り | 秒 |
|---|---|
| 全体（`tools/test-all.ps1` の段の合計） | 872 |
| うち `x64 workspace tests` | 734 |
| `areka` の実行ファイル（`src\main.rs` の unittests・libtest の `finished in`） | 93.83（4153 passed・5 ignored） |

- 1.20 倍の線（⑾ で比べる）: 全体 1046 秒・`areka` の実行ファイル 112.6 秒。

## ⑸ 較正

## ⑹ X1〜X3・Y1〜Y2

## ⑺ 原因の経路と直し

## ⑻ 許可の待ちの上限の値と決めた訳

## ⑼ 件 4 の見立てと根拠

## ⑽ third scope の読み分け

## ⑾ 修正の後の静かな机の全体テストと 1.20 倍の線

## ⑿ 最後の負荷 5 回の赤の読み分け
