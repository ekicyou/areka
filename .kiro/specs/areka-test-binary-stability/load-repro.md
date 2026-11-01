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

- 伝えた見込み: 3 回で約 5 分（2026-11-01 16:19・会話で開発者へ）。排他テストの机（`test`）を握り、負荷なし（`-Burners 0`）・各 1 回・`-Ignored`（`--ignored --exact`）。据え付けは `.CRT$XCU` の初期化子のまま（足場の入口への替えは要らなかった）。
- 位置は較正のテストの中の `RaiseException(0xC0000005)` なので、`module=` は `KERNELBASE.dll`（`RaiseException` の中）を指す。

**areka**（`target\load-flake\calib-areka-20261101-162013`・終了コード `-1073741819`＝`0xC0000005`・1.3 秒・報告のあった回 1）:

```
AREKA-CRASH code=0xC0000005 addr=0x7fff0289569a tid=33756
AREKA-CRASH module=C:\WINDOWS\System32\KERNELBASE.dll+0xC569A thread=""
AREKA-CRASH-HOLDER tid=35192 name="emo2_boot::crash_report_test_support::crash_report_names_the_running_test"
AREKA-CRASH-THREAD tid=39244 name="main"
AREKA-CRASH-THREAD tid=35192 name="emo2_boot::crash_report_test_support::crash_report_names_the_running_test"
AREKA-CRASH-THREAD tid=33756 name=""
（ほか名前の無いスレッド 6 本）
AREKA-CRASH-STACK    4: areka::emo2_boot::crash_report_test_support::report
AREKA-CRASH-STACK   11: RaiseException
AREKA-CRASH-STACK   12: areka::emo2_boot::crash_report_test_support::crash_report_names_the_running_test::closure$0
```

- 落ちたスレッド（`tid=33756`）は名前なし、持ち手の行と生きているスレッドの一覧に較正のテストの名前が出た。`GraphicsCore` を作った後（GPU の DLL を読み込んだ後）も受け手が生きている。

**wintf の lib**（`target\load-flake\calib-wintf-20261101-162023`・終了コード `0xC0000005`・9.1 秒・報告のあった回 1）:

```
AREKA-CRASH code=0xC0000005 addr=0x7fff0289569a tid=2896
AREKA-CRASH module=C:\WINDOWS\System32\KERNELBASE.dll+0xC569A thread=""
AREKA-CRASH-THREAD tid=31504 name="main"
AREKA-CRASH-THREAD tid=25036 name="test_crash_report::crash_report_names_the_running_test"
AREKA-CRASH-THREAD tid=2896 name=""
（ほか名前の無いスレッド 3 本）
AREKA-CRASH-STACK    4: wintf::test_crash_report::report
AREKA-CRASH-STACK   12: wintf::test_crash_report::crash_report_names_the_running_test::closure$0
```

**areka・許可の数 1**（`-SetEnv AREKA_TEST_GPU_SLOTS=1`・`target\load-flake\calib-areka-slots1-20261101-162336`）: `conditions.txt` に `set_env: AREKA_TEST_GPU_SLOTS=1`、持ち手の表は 1 行（`AREKA-CRASH-HOLDER tid=37876 name="...crash_report_names_the_running_test"`）。注意: 較正のテストは許可を 1 つしか取らないので、表が 1 行なのは許可の数 4 の走行と同じ。子に値が届くことの直の証は `conditions.txt` と、子の環境だけに置く `load-flake.ps1` の作り（2.3 の査読）で、値そのものの働きは 4.2 の X2 で見る。

## ⑹ X1〜X3・Y1〜Y2

## ⑺ 原因の経路と直し

## ⑻ 許可の待ちの上限の値と決めた訳

## ⑼ 件 4 の見立てと根拠

## ⑽ third scope の読み分け

## ⑾ 修正の後の静かな机の全体テストと 1.20 倍の線

## ⑿ 最後の負荷 5 回の赤の読み分け
