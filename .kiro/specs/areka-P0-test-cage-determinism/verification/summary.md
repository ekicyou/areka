# 反復実行の記録（要件 9.1-9.4）

repeat-tests.ps1 が 1 回の走行につき 1 節を追記する。読み方と負荷の定義は repeat-tests.md。
生ログは logs/（非追跡・再生成できる）、赤の回の生ログだけ red/（追跡）へ複写される。

注記（2026-08-24・タスク 8.1）: `cal-red` 節より前の 11 節は、判定が 5 値だった版で採った記録である
（合計行に「打ち切り」の欄が無い）。有界待機と「打ち切り」判定はレビュー所見 1 を受けて後から入れた。
数値そのものは採り直していない（対象・回数・passed 件数の意味は変わっていない）。


## kit — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×3（同時 2 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:33:22 |
| HEAD | `170edd6a`（作業ツリー dirty（2 件）） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 3 / 2 |
| 期待 passed | 81 |
| 事前ビルド | 0.2 秒・テスト実行体 5 本（刻印 logs/kit-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.1 の試走（緑の並び） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:33:17 | 2.6 | 0 | 79 | 0 | 2 | 0 | 6 | 件数不一致 | `kit-r001.out.log` |
| 2 | 21:33:17 | 2.7 | 0 | 79 | 0 | 2 | 0 | 6 | 件数不一致 | `kit-r002.out.log` |
| 3 | 21:33:19 | 2.2 | 0 | 79 | 0 | 2 | 0 | 6 | 件数不一致 | `kit-r003.out.log` |

**3 回走らせて 緑 0・赤 0・空振り 0・件数不一致 3・ビルド失敗 0**（所要秒 中央値 2.6 / 最小 2.2 / 最大 2.7）

### 緑でなかった回の内訳

- **回 1・判定 件数不一致**（終了コード 0・passed 79・failed 0・filtered out 0・ログ `kit-r001.out.log`）

- **回 2・判定 件数不一致**（終了コード 0・passed 79・failed 0・filtered out 0・ログ `kit-r002.out.log`）

- **回 3・判定 件数不一致**（終了コード 0・passed 79・failed 0・filtered out 0・ログ `kit-r003.out.log`）


## cal-seriko — areka-seriko の lib テスト（要件 3.7 の存在主張を含む） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:33:48 |
| HEAD | `170edd6a`（作業ツリー dirty（3 件）） |
| 実行コマンド | `cargo test -p areka-seriko --lib` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 200 |
| 事前ビルド | 0.3 秒・テスト実行体 1 本（刻印 logs/cal-seriko-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 対象表 Expect 値の較正（8.1） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:33:48 | 0.4 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `cal-seriko-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 0.4 / 最小 0.4 / 最大 0.4）


## cal-wait — 有界化した待機 2 テスト（要件 4） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:33:51 |
| HEAD | `170edd6a`（作業ツリー dirty（3 件）） |
| 実行コマンド | `cargo test -p areka --bins -- spine_e2e_sakura_blink_default_off_emits_nothing spine_s4_balloon_free_onboot_completes_without_balloon_face_switch` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 2 |
| 事前ビルド | 0.4 秒・テスト実行体 1 本（刻印 logs/cal-wait-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 対象表 Expect 値の較正（8.1） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:33:50 | 1.7 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `cal-wait-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 1.7 / 最小 1.7 / 最大 1.7）


## cal-wintf — wintf の lib テスト（錠を退役させた crate・要件 7.2） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:33:54 |
| HEAD | `170edd6a`（作業ツリー dirty（3 件）） |
| 実行コマンド | `cargo test -p wintf --lib` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 842 |
| 事前ビルド | 0.4 秒・テスト実行体 1 本（刻印 logs/cal-wintf-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 対象表 Expect 値の較正（8.1） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:33:53 | 1.6 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `cal-wintf-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 1.6 / 最小 1.6 / 最大 1.6）


## cal-workspace — ワークスペース全体 ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:34:48 |
| HEAD | `170edd6a`（作業ツリー dirty（3 件）） |
| 実行コマンド | `cargo test --workspace` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 5865 |
| 事前ビルド | 0.5 秒・テスト実行体 72 本（刻印 logs/cal-workspace-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 対象表 Expect 値の較正と基準線の確認（8.1） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:34:11 | 36.8 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `cal-workspace-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 36.8 / 最小 36.8 / 最大 36.8）


## cal-empty — custom ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:35:48 |
| HEAD | `170edd6a`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test -p log-capture-kit --lib -- capture_evnt_from_inside_the_window` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 1 |
| 事前ビルド | 0.2 秒・テスト実行体 1 本（刻印 logs/cal-empty-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 較正: フィルタの綴りを 1 文字誤らせた走行（evnt）。終了コード 0・0 passed を緑と数えないことの実証 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:35:48 | 0.2 | 0 | 0 | 0 | 0 | 26 | 1 | 空振り | `cal-empty-r001.out.log` |

**1 回走らせて 緑 0・赤 0・空振り 1・件数不一致 0・ビルド失敗 0**（所要秒 中央値 0.2 / 最小 0.2 / 最大 0.2）

### 緑でなかった回の内訳

- **回 1・判定 空振り**（終了コード 0・passed 0・failed 0・filtered out 26・ログ `cal-empty-r001.out.log`）


## trial-kit — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×3（同時 2 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:36:07 |
| HEAD | `170edd6a`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 3 / 2 |
| 期待 passed | 79 |
| 事前ビルド | 0.2 秒・テスト実行体 5 本（刻印 logs/trial-kit-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.1 の試走（対象表の期待値を実測 79 へ是正した後の再走） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:36:02 | 2.6 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-kit-r001.out.log` |
| 2 | 21:36:02 | 2.6 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-kit-r002.out.log` |
| 3 | 21:36:05 | 2.4 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-kit-r003.out.log` |

**3 回走らせて 緑 3・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 2.6 / 最小 2.4 / 最大 2.6）


## trial-load — wintf の lib テスト（錠を退役させた crate・要件 7.2） ×8（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:36:19 |
| HEAD | `170edd6a`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test -p wintf --lib` |
| 回数 / 同時プロセス | 8 / 4 |
| 期待 passed | 842 |
| 事前ビルド | 0.4 秒・テスト実行体 1 本（刻印 logs/trial-load-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.1 の試走（負荷の定義＝同時 4 プロセス。8.2 の 30 回反復と同じ形の縮小版） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:36:09 | 4.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r001.out.log` |
| 2 | 21:36:09 | 4.5 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r002.out.log` |
| 3 | 21:36:09 | 4.5 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r003.out.log` |
| 4 | 21:36:09 | 4.5 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r004.out.log` |
| 5 | 21:36:13 | 5.7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r005.out.log` |
| 6 | 21:36:13 | 5.8 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r006.out.log` |
| 7 | 21:36:13 | 5.8 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r007.out.log` |
| 8 | 21:36:13 | 5.8 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `trial-load-r008.out.log` |

**8 回走らせて 緑 8・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 5.1 / 最小 4.4 / 最大 5.8）


## trial-root — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:37:06 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `170edd6a`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 79 |
| 事前ビルド | 0.2 秒・テスト実行体 5 本（刻印 logs/trial-root-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.1 の試走（-Root と 走行ルート 列の確認） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:37:04 | 2.1 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-root-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 2.1 / 最小 2.1 / 最大 2.1）


## trial-root-expect — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:40:06 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `170edd6a`（作業ツリー dirty（5 件）） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 指定なし |
| 事前ビルド | 0.2 秒・テスト実行体 5 本（刻印 logs/trial-root-expect-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.1 の試走（-Root の明示指定と -ExpectPassed -1 で期待値を無効化できることの確認） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:40:04 | 2.2 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-root-expect-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 2.2 / 最小 2.2 / 最大 2.2）


## trial-final — areka-seriko の lib テスト（要件 3.7 の存在主張を含む） ×4（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:42:15 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `170edd6a`（作業ツリー dirty（6 件）） |
| 実行コマンド | `cargo test -p areka-seriko --lib` |
| 回数 / 同時プロセス | 4 / 4 |
| 期待 passed | 200 |
| 事前ビルド | 0.3 秒・テスト実行体 1 本（刻印 logs/trial-final-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.1 の最終試走（負荷 4 同時・完成した仕組みでの通し確認） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:42:14 | 0.7 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `trial-final-r001.out.log` |
| 2 | 21:42:14 | 0.8 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `trial-final-r002.out.log` |
| 3 | 21:42:14 | 0.8 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `trial-final-r003.out.log` |
| 4 | 21:42:14 | 0.8 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `trial-final-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0**（所要秒 中央値 0.8 / 最小 0.7 / 最大 0.8）



## cal-red — custom ×2（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:59:22 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `170edd6a`（作業ツリー dirty（5 件）） |
| 実行コマンド | `cargo test --manifest-path <temp>\areka-cage-calibration\redcal\Cargo.toml --lib` |
| 回数 / 同時プロセス | 2 / 1 |
| 期待 passed | 1 |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 0.1 秒・テスト実行体 1 本（刻印 logs/cal-red-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 較正: わざと 1 本落ちる使い捨てクレート（手順書 §6-a の作り方・リポジトリ外）。赤の回にテスト名と失敗内容が要約へ載ることの実証 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:59:22 | 0.1 | 101 | 1 | 1 | 0 | 0 | 1 | 赤 | `cal-red-r001.out.log` |
| 2 | 21:59:22 | 0.1 | 101 | 1 | 1 | 0 | 0 | 1 | 赤 | `cal-red-r002.out.log` |

**2 回走らせて 緑 0・赤 2・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 0.1 / 最小 0.1 / 最大 0.1）

### 緑でなかった回の内訳

- **回 1・判定 赤**（終了コード 101・passed 1・failed 1・filtered out 0・ログ `cal-red-r001.out.log`）
  - 失敗したテスト（1 件）:
    - `tests::redcal_this_one_fails_on_purpose`

  失敗内容 `tests::redcal_this_one_fails_on_purpose`:

  ```
  
  thread 'tests::redcal_this_one_fails_on_purpose' (27544) panicked at src\lib.rs:11:9:
  assertion `left == right` failed: わざと落とす較正用のテスト（実測 41）
    left: 41
   right: 42
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::assert_failed_inner
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:434
     3: core::panicking::assert_failed<i32,i32>
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:394
     4: redcal::tests::redcal_this_one_fails_on_purpose
               at .\src\lib.rs:11
     5: redcal::tests::redcal_this_one_fails_on_purpose::closure$0
               at .\src\lib.rs:9
     6: core::ops::function::FnOnce::call_once<redcal::tests::redcal_this_one_fails_on_purpose::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     7: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

- **回 2・判定 赤**（終了コード 101・passed 1・failed 1・filtered out 0・ログ `cal-red-r002.out.log`）
  - 失敗したテスト（1 件）:
    - `tests::redcal_this_one_fails_on_purpose`

  失敗内容 `tests::redcal_this_one_fails_on_purpose`:

  ```
  
  thread 'tests::redcal_this_one_fails_on_purpose' (30976) panicked at src\lib.rs:11:9:
  assertion `left == right` failed: わざと落とす較正用のテスト（実測 41）
    left: 41
   right: 42
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::assert_failed_inner
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:434
     3: core::panicking::assert_failed<i32,i32>
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:394
     4: redcal::tests::redcal_this_one_fails_on_purpose
               at .\src\lib.rs:11
     5: redcal::tests::redcal_this_one_fails_on_purpose::closure$0
               at .\src\lib.rs:9
     6: core::ops::function::FnOnce::call_once<redcal::tests::redcal_this_one_fails_on_purpose::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     7: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```


## cal-hang — custom ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 21:59:38 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `170edd6a`（作業ツリー dirty（6 件）） |
| 実行コマンド | `cargo test --manifest-path <temp>\areka-cage-calibration\hangcal\Cargo.toml --lib` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 1 |
| 1 回の上限 | 15 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.1 秒・テスト実行体 1 本（刻印 logs/cal-hang-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 較正: 決して終わらないテストを 1 本だけ持つ使い捨てクレート。1 回の上限が効いてプロセス木が止まり、要約から打ち切りが読めることの実証 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 21:59:23 | 15 | -1 | 0 | 0 | 0 | 0 | 0 | 打ち切り | `cal-hang-r001.out.log` |

**1 回走らせて 緑 0・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 1**（所要秒 中央値 15 / 最小 15 / 最大 15）

### 緑でなかった回の内訳

- **回 1・判定 打ち切り**（終了コード -1・passed 0・failed 0・filtered out 0・ログ `cal-hang-r001.out.log`）
  - **上限 15 秒に達したので打ち切った**（-TimeoutSec で明示指定）。上限に達したのでプロセス木を停止した。
  - 打ち切りの回の出力は途中までしか無い。上限が短すぎたのか本当にハングしたのかは生ログの最終行で判断すること（理由の分からない打ち切りを残さない）。


## trial-bounded — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×3（同時 2 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-24 22:02:05 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `170edd6a`（作業ツリー dirty（6 件）） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 3 / 2 |
| 期待 passed | 79 |
| 1 回の上限 | 120 秒（自動＝単独実測 2.4 秒 × 同時 2 × 10（下限 120 秒）） |
| 事前ビルド | 0.2 秒・テスト実行体 5 本（刻印 logs/trial-bounded-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 上限つき（有界待機）の仕組みで実対象を 1 回通す確認 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:02:01 | 2.3 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-bounded-r001.out.log` |
| 2 | 22:02:01 | 2.3 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-bounded-r002.out.log` |
| 3 | 22:02:03 | 2 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `trial-bounded-r003.out.log` |

**3 回走らせて 緑 3・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 2.3 / 最小 2 / 最大 2.3）

