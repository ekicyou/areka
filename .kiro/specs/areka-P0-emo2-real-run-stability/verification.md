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

後のタスクで埋める。

## 3. 保険の自動終了の値の確かめ

後のタスクで埋める。

## 4. 200% と 100% 相当の確かめ

後のタスクで埋める。

## 5. 仲介で拾える手がかり

後のタスクで埋める。

## 6. くり返しの結果

後のタスクで埋める。

## 7. 閉じ方と根拠

後のタスクで埋める。
