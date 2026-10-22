# 実機の確かめの記録（areka-P0-emo2-real-run-stability）

## 1. 件 1 の再現

直す前の形の試験（`crates/areka/tests/emo2_real_run.rs`・保険の自動終了 3 秒で子が自分で終わるのを待つ形）を回した記録。

- コミット: `0e0fb64e`（コードは変えずに回した）
- 作り方: debug（`cargo test -p areka --test emo2_real_run -- --nocapture`・`AREKA_EMO2_REAL_RUN=1`）
- 画面の拡大率: 200%（回す前に `GetDpiForSystem` で 192 DPI を読んだ）
- 回した日: 2026-10-22（排他テストの机を借りて回した）
- 32bit の仲介: `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` で建てた物（`target\i686-pc-windows-msvc\debug`）
- 回す前に示した回数と所要時間の見込み: 3 回・1 回あたり約 1〜2 分（締切 120 秒が上限）、合わせて最大約 6 分

| 回 | 結果 | 試験の所要（cargo の表示） |
|---|---|---|
| 1 | 緑（`test result: ok. 1 passed`・終了コード 0） | 7.49 秒 |
| 2 | 緑（`test result: ok. 1 passed`・終了コード 0） | 4.91 秒 |
| 3 | 緑（`test result: ok. 1 passed`・終了コード 0） | 4.52 秒 |

赤の文: 3 回とも出なかった（赤は 0 回）。

まとめ: 直す前の形の試験は、この条件（200%・debug）では 3 回続けて緑で、件 1 の赤は再現しなかった。所要が 4〜8 秒あるので、飛ばし（`return`）ではなく子を起こして回っている。

## 2. 件 1 の測りと切り分け

新しい形の試験（印を見てから利用者と同じ閉じ方で閉じる形）を回し、時間の 1 行を取った記録。

- コミット: `c6a70f14`（コードは変えずに回した）
- 作り方: debug（`AREKA_EMO2_REAL_RUN=1 cargo test -p areka --test emo2_real_run -- --nocapture`）
- 画面の拡大率: 200%（3 回とも時間の 1 行が `primary_dpi=192`）
- 回した日: 2026-10-23（排他テストの机を借りて回した）
- 32bit の仲介: `target\debug\shiori-host32-helper.exe`（タスク 4.3 で置いた新しい物）
- 回す前に示した回数と所要時間の見込み: 3 回・1 回あたり約 1 分（各待ちの締切 120 秒が上限）、合わせて約 3〜6 分
- 機械の混み具合: 負荷は測っていない。1 回目は試験の実行ファイルを建てた直後の最初の起動で、wire 成立まで 4 秒かかった（2・3 回目は 0.8 秒）。冷えた起動で遅かったとみるが、混んでいたかは分からない。3 回とも混んだ時間を狙って回したわけではない

| 回 | 結果 | 時間の 1 行 |
|---|---|---|
| 1 | 緑（`test result: ok`・終了コード 0・33.25 秒） | `emo2_real_run: wire_ms=4057 attach_ms=5838 wrap_ms=8512 close_ms=21508 talk_to_wrap_ms=240 attach_to_talk_ms=2458 primary_dpi=192` |
| 2 | 緑（`test result: ok`・終了コード 0・25.33 秒） | `emo2_real_run: wire_ms=751 attach_ms=956 wrap_ms=2292 close_ms=22860 talk_to_wrap_ms=55 attach_to_talk_ms=1281 primary_dpi=192` |
| 3 | 緑（`test result: ok`・終了コード 0・25.33 秒） | `emo2_real_run: wire_ms=762 attach_ms=1057 wrap_ms=2300 close_ms=22827 talk_to_wrap_ms=61 attach_to_talk_ms=1205 primary_dpi=192` |

値が `none` の回: 0 回（3 回とも全部の値が取れたので、3 回とも切り分けに使った）。

切り分け（設計の確かめの手順 4 の規則）:

| 回 | `attach_to_talk_ms` の符号 | 後に来た方 | areka 自身の時間 |
|---|---|---|---|
| 1 | 正（+2458） | 最初の台詞 | `talk_to_wrap_ms` = 240 ms |
| 2 | 正（+1281） | 最初の台詞 | `talk_to_wrap_ms` = 55 ms |
| 3 | 正（+1205） | 最初の台詞 | `talk_to_wrap_ms` = 61 ms |

- areka 自身の時間の最大: 240 ms（1 回目）。旧い保険の自動終了 3 秒を超えた回は 0 回
- `wrap_ms` の最大: 8512 ms（1 回目）。そのうち装着の後、最初の台詞まで約 2.5 秒が SHIORI の読み込みの待ち
- `close_ms` の最大: 22860 ms（2 回目）。閉じる知らせから終わるまでの時間で、節 3 の値の確かめに使う

結論: **遅さの在りかは「待ち方」**。3 回とも最初の台詞が装着より後に来ていて、`wrap` を遅らせたのは SHIORI の読み込みの待ちと冷えた起動だった。最初の台詞から `wrap` までの areka 自身の時間は最大でも 240 ms だった。旧い形で赤になりうるのは、保険の自動終了 3 秒が起動と SHIORI の読み込みの待ちを含めた全体（1 回目は `wrap_ms` 8.5 秒）に足りないためで、装着そのものの遅さではない。タスク 6.2（装着の遅さの直し）は掛からない。

直しの有無（2.3）: 装着そのものではないので直しは無い。areka 自身の時間が 3 秒を超えた回は 0 回で、切り分けの規則では「装着そのもの」に当たらない。設計への追補も、直した後の測り直しも無い。保険の自動終了を延ばすだけの直しも採っていない。

## 3. 保険の自動終了の値の確かめ

件 1 の試験の回（節 2 の 3 回・節 4 の 6 回）と、くり返しの成功の回（節 6 の 20 回）の時間の 1 行から出した（机は使っていない）。

| 出どころ | `wrap_ms` の最大 | `close_ms` の最大 |
|---|---|---|
| 節 2（件 1 の試験・3 回） | 8512 ms（1 回目） | 22860 ms（2 回目） |
| 節 4（件 1 の試験・6 回） | 2936 ms（100% 相当の 3 回目） | 22915 ms（200% の 2 回目） |
| 節 6（くり返しの成功・20 回） | 8734 ms（1 回目） | 24579 ms（5 回目） |
| 全部の最大 | 8734 ms | 24579 ms |

- 和: 8734 ＋ 24579 ＝ 33313 ms（約 33.3 秒）
- 線の 35 秒と比べると: 35 秒以下（残りは 1687 ms）
- 45 秒と比べると: 45 秒より 11687 ms 短い
- 値を変えたか: 変えていない。保険の自動終了は 45 秒のまま（`crates/areka/tests/emo2_real_run/common.rs:10` の `SAFETY_EXIT`）
- 書き添え: 35 秒の線までの残りは 1687 ms と小さい。和を押し上げているのは `close_ms`（最大 24579 ms）と、つなぐまでが長かった回の `wrap_ms`（最大 8734 ms）

## 4. 200% と 100% 相当の確かめ

新しい形の試験を、200% のまま 3 回、100% 相当（互換モード）で 3 回回した記録。

- コミット: `4efab284`（コードは変えずに回した）
- 作り方: debug（`AREKA_EMO2_REAL_RUN=1 cargo test -p areka --test emo2_real_run -- --nocapture`）
- 画面の拡大率: 200%（OS の表示の設定は替えていない）
- 100% 相当の作り方: `AREKA_EMO2_COMPAT_LAYER=DPIUNAWARE` を置き、`common::spawn_emo2` が子の `__COMPAT_LAYER` にだけ渡した。初めの値で 3 回とも時間の 1 行が `primary_dpi=96` になったので、予備の値（`GDIDPISCALING DPIUNAWARE`）は回していない
- 回した日: 2026-10-23（排他テストの机を借りて回した）
- 32bit の仲介: `target\debug\shiori-host32-helper.exe`

| 画面 | 回 | 結果 | `wrap_ms` | `primary_dpi` | 時間の 1 行 |
|---|---|---|---|---|---|
| 200% | 1 | 緑（`test result: ok`・終了コード 0・22.03 秒） | 2294 | 192 | `emo2_real_run: wire_ms=764 attach_ms=959 wrap_ms=2294 close_ms=19521 talk_to_wrap_ms=72 attach_to_talk_ms=1277 primary_dpi=192` |
| 200% | 2 | 緑（`test result: ok`・終了コード 0・25.47 秒） | 2309 | 192 | `emo2_real_run: wire_ms=921 attach_ms=1116 wrap_ms=2309 close_ms=22915 talk_to_wrap_ms=62 attach_to_talk_ms=1145 primary_dpi=192` |
| 200% | 3 | 緑（`test result: ok`・終了コード 0・22.05 秒） | 2355 | 192 | `emo2_real_run: wire_ms=870 attach_ms=1146 wrap_ms=2355 close_ms=19478 talk_to_wrap_ms=78 attach_to_talk_ms=1190 primary_dpi=192` |
| 100% 相当（`DPIUNAWARE`） | 1 | 緑（`test result: ok`・終了コード 0・24.35 秒） | 2375 | 96 | `emo2_real_run: wire_ms=924 attach_ms=1175 wrap_ms=2375 close_ms=21755 talk_to_wrap_ms=80 attach_to_talk_ms=1129 primary_dpi=96` |
| 100% 相当（`DPIUNAWARE`） | 2 | 緑（`test result: ok`・終了コード 0・25.80 秒） | 2588 | 96 | `emo2_real_run: wire_ms=1068 attach_ms=1396 wrap_ms=2588 close_ms=22910 talk_to_wrap_ms=44 attach_to_talk_ms=1128 primary_dpi=96` |
| 100% 相当（`DPIUNAWARE`） | 3 | 緑（`test result: ok`・終了コード 0・24.91 秒） | 2936 | 96 | `emo2_real_run: wire_ms=1411 attach_ms=1661 wrap_ms=2936 close_ms=21700 talk_to_wrap_ms=148 attach_to_talk_ms=1128 primary_dpi=96` |

赤の回: 0 回（6 回とも緑）。

まとめ: 200% と 100% 相当のどちらでも 3 回続けて緑だった。`wrap_ms` の最大は 200% で 2355 ms、100% 相当で 2936 ms。`close_ms` の最大は 22915 ms（200% の 2 回目）。

## 5. 仲介で拾える手がかり

コードを読んで調べた記録（机は使っていない・要件 5.4）。

**今の記録は `LoadReturnedFalse` だけだった**

- 元の失敗の回で仲介が標準エラーへ出したのは `[helper] SHIORI 初期化の入口: loadu` と `[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse` の 2 行だけ。失敗の行は `crates/shiori-host32-helper/src/main.rs:339` の `eprintln!("[helper] LOAD 失敗（観測・ack[0]）: {e:?}")` で、直す前の `ProxyError::LoadReturnedFalse` は中身を持たなかったので、行には型の名前しか出なかった
- SHIORI の初期化の入口（`loadu`／`load`）は真偽しか返さない。仲介が受け取れるのは戻り値が 0 だったことだけ

**`HRESULT(0x80004005)` は仲介の手がかりではない**

- 元の回の標準エラーの `Error: Error { code: HRESULT(0x80004005), message: "エラーを特定できません" }` は、仲介ではなく areka.exe の行
- areka.exe の入口は `fn main() -> Result<()>`（`crates/areka/src/main.rs:154`・`windows::core::Result`）。失敗で終わるときは決まった値 `E_FAIL`（0x80004005）を返す（`crates/areka/src/main.rs:198` 起動前の解決の失敗・`:290` 開始の窓の失敗・`:413` `finish_after_run` の失敗の印）。Rust は `main` が `Err` を返すと `Error: {:?}` を標準エラーへ出し、終了コード 1 で終える
- よってこの行は「areka.exe が終了コード 1 で終わった」ことを言うだけで、なぜ LOAD が断られたかの手がかりは何も持たない

**足した手がかり: OS の最後のエラー**

- コミット `3c8d5cc6`: 失敗の型を `LoadReturnedFalse { last_error: u32 }` にした（`crates/shiori-host32-helper/src/shiori_proxy.rs:121`）。初期化の入口が 0 を返した直後、ほかの関数（`eprintln!`・`FreeLibrary`）を呼ぶ前に `GetLastError` を読んで載せる（`crates/shiori-host32-helper/src/shiori_proxy.rs:285`〜`286`）。記録の行（`main.rs:339`）の綴りは変えておらず、`{e:?}` によって `LoadReturnedFalse { last_error: 5 }` の形で出る。ack は `last_error` の値に依らず 0（`crates/shiori-host32-helper/src/main_load_ack_tests.rs:19`）
- コミット `86faa363`: 試験用の DLL の失敗の注入（`HOST32_TESTDLL_LOADU_FAIL=1`）で、0 を返す直前に最後のエラーへアクセス拒否（5）を置くようにした（`crates/shiori-host32-testdll-loadu/src/lib.rs:76`）。i686 の試験 `testdll_loadu_false_is_load_returned_false_without_falling_back` が `last_error == 5` を確かめる（`crates/shiori-host32-helper/src/shiori_proxy_loadu_tests.rs:281`〜`282`）

**限り（約束のない手がかり）**

- SHIORI の約束に「失敗したら最後のエラーを置く」は無い。SHIORI の DLL（pasta.dll など）には置く義務が無い
- 読めるのは、DLL の中で最後に呼ばれた OS の関数の結果がたまたま残っていたときだけ。DLL が失敗の後に記録のファイルへ書くなどほかの関数を呼べば上書きされ、0 や失敗の原因と無関係の値になりうる
- そのため `last_error` は「名前替えの失敗（5＝アクセス拒否・32＝共有違反）などがそのまま残っていれば強い手がかり」として読み、0 や無関係に見える値を「OS の失敗ではない」証拠とは読まない。原因の確かめは、残した写しの中の SHIORI 自身の記録（要件 5.1）を合わせて読む
- 型の説明にもこの限りを書いてある（`crates/shiori-host32-helper/src/shiori_proxy.rs:117`〜`120`）

まとめ: 仲介が今より多く拾えるのは OS の最後のエラー 1 つだけで、それを LOAD の失敗の記録の行に足した（要件 5.5）。ほかに仲介の側で拾える手がかりは無い。

## 6. くり返しの結果

くり返しの試験を基本の条件で 20 回回した記録。

- 見込み（回す前に示した値）: 20 回、1 回約 25 秒で全部で約 9〜12 分
- コミット: `141edc9a`（コードは変えずに回した）
- 作り方: debug（`$env:AREKA_EMO2_REPEAT="20"; cargo test -p areka --test emo2_first_load_repeat -- --nocapture`）
- 条件: 基本（`AREKA_EMO2_REPEAT_DROP` は置いていない）
- 画面の拡大率: 200%（20 回とも `primary_dpi=192`）
- 回した日: 2026-10-23 00:33:28〜00:42:03（排他テストの机を借りて回した。試験の所要は 510.43 秒、終了コード 0）

**結果**

まとめの 1 行: `emo2_first_load_repeat: condition=basic runs=20 ok=20 load_failed=0 other_failed=0`

| 結果 | 回数 |
|---|---|
| 成功（`ok`） | 20 |
| LOAD の失敗（`load_failed`） | 0 |
| ほかの失敗（`other_failed`） | 0 |

- 落ちた回: 0 回。残った置き場が無いので、手引きの順に読むものは無かった
- 成功の回の置き場が残っていないこと: 走り終えた後の `target\test-roots\` は空だった（20 回とも窓口が片付けた）

**各回の時間の 1 行**（7.3 で使う）

| 回 | `wire_ms` | `attach_ms` | `wrap_ms` | `close_ms` | `talk_to_wrap_ms` | `attach_to_talk_ms` |
|---|---|---|---|---|---|---|
| 1 | 8732 | 8733 | 8734 | 20277 | 814 | 2528 |
| 2 | 2914 | 2914 | 2915 | 22306 | 88 | 1087 |
| 3 | 2873 | 2873 | 2874 | 22759 | 79 | 1296 |
| 4 | 5769 | 5770 | 5773 | 22546 | 384 | 2516 |
| 5 | 6381 | 6382 | 6385 | 24579 | 196 | 2146 |
| 6 | 8476 | 8479 | 8481 | 21754 | 151 | 1970 |
| 7 | 2502 | 2503 | 2503 | 19378 | 85 | 1051 |
| 8 | 2031 | 2031 | 2031 | 19475 | 65 | 1076 |
| 9 | 2559 | 2560 | 2560 | 21309 | 70 | 1127 |
| 10 | 2870 | 2871 | 2871 | 18936 | 46 | 1148 |
| 11 | 2967 | 2968 | 2969 | 22459 | 88 | 1236 |
| 12 | 2913 | 2914 | 2915 | 21297 | 57 | 1237 |
| 13 | 2776 | 2777 | 2777 | 22542 | 81 | 1196 |
| 14 | 2840 | 2840 | 2841 | 21328 | 79 | 1203 |
| 15 | 3153 | 3154 | 3156 | 21329 | 133 | 1228 |
| 16 | 3379 | 3380 | 3381 | 22192 | 107 | 1132 |
| 17 | 3153 | 3153 | 3154 | 19094 | 56 | 1327 |
| 18 | 3427 | 3428 | 3429 | 22479 | 104 | 1243 |
| 19 | 2950 | 2950 | 2951 | 21123 | 60 | 1102 |
| 20 | 2826 | 2827 | 2827 | 19133 | 58 | 1266 |

- 成功の回の `wrap_ms` の最大: 8734 ms（1 回目）
- 成功の回の `close_ms` の最大: 24579 ms（5 回目）
- 気づいたこと: 1・4・5・6 回目は `wrap_ms` が 5.7〜8.7 秒で、節 4 の 6 回（最大 2936 ms）より長かった。ほかの 16 回は 2.0〜3.4 秒。長かった回も遅れは `wire_ms` の前（つなぐまで）に在り、失敗にはなっていない

**取り合いの相手を外した比べ**

失敗 0 回のため比べは回していない（`gap`・`copy-wait`・`no-mcp` の条件はどれも回していない）。

## 7. 閉じ方と根拠

後のタスクで埋める。
