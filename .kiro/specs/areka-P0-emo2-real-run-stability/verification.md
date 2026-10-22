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

後のタスクで埋める。

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

後のタスクで埋める。

## 6. くり返しの結果

後のタスクで埋める。

## 7. 閉じ方と根拠

後のタスクで埋める。
