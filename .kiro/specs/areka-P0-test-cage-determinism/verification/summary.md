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


## cal82-kit — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:24:26 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー clean） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 79 |
| 1 回の上限 | 120 秒（自動＝単独実測 2.4 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 2.5 秒・テスト実行体 5 本（刻印 logs/cal82-kit-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.2 走行前の期待件数の採り直し |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:23:39 | 47 | 0 | 79 | 0 | 2 | 0 | 6 | 緑 | `cal82-kit-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 47 / 最小 47 / 最大 47）


## cal82-seriko — areka-seriko の lib テスト（要件 3.7 の存在主張を含む） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:24:37 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p areka-seriko --lib` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 200 |
| 1 回の上限 | 120 秒（自動＝単独実測 0.4 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 7.6 秒・テスト実行体 1 本（刻印 logs/cal82-seriko-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.2 走行前の期待件数の採り直し |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:24:36 | 1.3 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `cal82-seriko-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 1.3 / 最小 1.3 / 最大 1.3）


## cal82-wait — 有界化した待機 2 テスト（要件 4） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:24:47 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p areka --bins -- spine_e2e_sakura_blink_default_off_emits_nothing spine_s4_balloon_free_onboot_completes_without_balloon_face_switch` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 2 |
| 1 回の上限 | 120 秒（自動＝単独実測 1.7 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 3.5 秒・テスト実行体 1 本（刻印 logs/cal82-wait-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.2 走行前の期待件数の採り直し |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:24:42 | 4.9 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `cal82-wait-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 4.9 / 最小 4.9 / 最大 4.9）


## cal82-wintf — wintf の lib テスト（錠を退役させた crate・要件 7.2） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:25:02 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p wintf --lib` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 842 |
| 1 回の上限 | 120 秒（自動＝単独実測 1.6 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 3.7 秒・テスト実行体 1 本（刻印 logs/cal82-wintf-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.2 走行前の期待件数の採り直し |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:24:54 | 8.3 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `cal82-wintf-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 8.3 / 最小 8.3 / 最大 8.3）


## cal82-workspace — ワークスペース全体 ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:28:35 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test --workspace` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 5865 |
| 1 回の上限 | 368 秒（自動＝単独実測 36.8 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 31.3 秒・テスト実行体 72 本（刻印 logs/cal82-workspace-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.2 走行前の期待件数の採り直しと基準線 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:25:53 | 162.4 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `cal82-workspace-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 162.4 / 最小 162.4 / 最大 162.4）


## pilot82-wintf — wintf の lib テスト（錠を退役させた crate・要件 7.2） ×4（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:29:39 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p wintf --lib` |
| 回数 / 同時プロセス | 4 / 4 |
| 期待 passed | 842 |
| 1 回の上限 | 120 秒（自動＝単独実測 1.6 秒 × 同時 4 × 10（下限 120 秒）） |
| 事前ビルド | 0.5 秒・テスト実行体 1 本（刻印 logs/pilot82-wintf-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 8.2 本走行前の上限の見積り（本日の負荷下 1 回あたりの所要を採る） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:29:27 | 11.2 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `pilot82-wintf-r001.out.log` |
| 2 | 22:29:27 | 11.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `pilot82-wintf-r002.out.log` |
| 3 | 22:29:27 | 11.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `pilot82-wintf-r003.out.log` |
| 4 | 22:29:27 | 11.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `pilot82-wintf-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 11.4 / 最小 11.2 / 最大 11.4）


## r92-seriko — areka-seriko の lib テスト（要件 3.7 の存在主張を含む） ×30（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:30:02 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p areka-seriko --lib` |
| 回数 / 同時プロセス | 30 / 4 |
| 期待 passed | 200 |
| 1 回の上限 | 120 秒（自動＝単独実測 0.4 秒 × 同時 4 × 10（下限 120 秒）） |
| 事前ビルド | 0.3 秒・テスト実行体 1 本（刻印 logs/r92-seriko-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 9.2（要件 3.7 の存在主張テスト群を含む） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:29:52 | 0.8 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r001.out.log` |
| 2 | 22:29:52 | 0.9 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r002.out.log` |
| 3 | 22:29:52 | 0.9 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r003.out.log` |
| 4 | 22:29:52 | 0.9 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r004.out.log` |
| 5 | 22:29:53 | 0.8 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r005.out.log` |
| 6 | 22:29:53 | 0.8 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r006.out.log` |
| 7 | 22:29:53 | 0.9 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r007.out.log` |
| 8 | 22:29:53 | 0.9 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r008.out.log` |
| 9 | 22:29:54 | 0.7 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r009.out.log` |
| 10 | 22:29:54 | 0.9 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r010.out.log` |
| 11 | 22:29:54 | 1 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r011.out.log` |
| 12 | 22:29:54 | 1 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r012.out.log` |
| 13 | 22:29:55 | 1.1 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r013.out.log` |
| 14 | 22:29:55 | 1.1 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r014.out.log` |
| 15 | 22:29:55 | 1.2 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r015.out.log` |
| 16 | 22:29:55 | 1.3 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r016.out.log` |
| 17 | 22:29:56 | 1.5 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r017.out.log` |
| 18 | 22:29:56 | 1.5 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r018.out.log` |
| 19 | 22:29:56 | 1.5 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r019.out.log` |
| 20 | 22:29:56 | 1.4 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r020.out.log` |
| 21 | 22:29:58 | 1.2 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r021.out.log` |
| 22 | 22:29:58 | 1.3 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r022.out.log` |
| 23 | 22:29:58 | 1.3 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r023.out.log` |
| 24 | 22:29:58 | 1.4 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r024.out.log` |
| 25 | 22:29:59 | 1.1 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r025.out.log` |
| 26 | 22:29:59 | 1.3 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r026.out.log` |
| 27 | 22:29:59 | 1.4 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r027.out.log` |
| 28 | 22:29:59 | 1.4 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r028.out.log` |
| 29 | 22:30:01 | 0.8 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r029.out.log` |
| 30 | 22:30:01 | 0.7 | 0 | 200 | 0 | 0 | 0 | 1 | 緑 | `r92-seriko-r030.out.log` |

**30 回走らせて 緑 30・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 1.1 / 最小 0.7 / 最大 1.5）


## r93-wait — 有界化した待機 2 テスト（要件 4） ×30（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:30:33 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p areka --bins -- spine_e2e_sakura_blink_default_off_emits_nothing spine_s4_balloon_free_onboot_completes_without_balloon_face_switch` |
| 回数 / 同時プロセス | 30 / 4 |
| 期待 passed | 2 |
| 1 回の上限 | 120 秒（自動＝単独実測 1.7 秒 × 同時 4 × 10（下限 120 秒）） |
| 事前ビルド | 0.4 秒・テスト実行体 1 本（刻印 logs/r93-wait-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 9.3（有界化した待機 2 テスト） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:30:08 | 3.2 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r001.out.log` |
| 2 | 22:30:08 | 3.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r002.out.log` |
| 3 | 22:30:08 | 3.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r003.out.log` |
| 4 | 22:30:08 | 3.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r004.out.log` |
| 5 | 22:30:12 | 3.1 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r005.out.log` |
| 6 | 22:30:12 | 3.1 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r006.out.log` |
| 7 | 22:30:12 | 3.1 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r007.out.log` |
| 8 | 22:30:12 | 3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r008.out.log` |
| 9 | 22:30:15 | 2.5 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r009.out.log` |
| 10 | 22:30:15 | 2.8 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r010.out.log` |
| 11 | 22:30:15 | 2.9 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r011.out.log` |
| 12 | 22:30:15 | 3.2 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r012.out.log` |
| 13 | 22:30:18 | 2.5 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r013.out.log` |
| 14 | 22:30:18 | 2.8 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r014.out.log` |
| 15 | 22:30:18 | 2.8 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r015.out.log` |
| 16 | 22:30:18 | 3.1 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r016.out.log` |
| 17 | 22:30:21 | 3.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r017.out.log` |
| 18 | 22:30:21 | 3.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r018.out.log` |
| 19 | 22:30:21 | 3.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r019.out.log` |
| 20 | 22:30:21 | 3.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r020.out.log` |
| 21 | 22:30:25 | 2.9 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r021.out.log` |
| 22 | 22:30:25 | 3.1 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r022.out.log` |
| 23 | 22:30:25 | 3.1 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r023.out.log` |
| 24 | 22:30:25 | 3.2 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r024.out.log` |
| 25 | 22:30:28 | 2.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r025.out.log` |
| 26 | 22:30:28 | 2.3 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r026.out.log` |
| 27 | 22:30:28 | 2.6 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r027.out.log` |
| 28 | 22:30:28 | 2.6 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r028.out.log` |
| 29 | 22:30:31 | 2.1 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r029.out.log` |
| 30 | 22:30:31 | 2.2 | 0 | 2 | 0 | 0 | 1237 | 1 | 緑 | `r93-wait-r030.out.log` |

**30 回走らせて 緑 30・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 3.1 / 最小 2.1 / 最大 3.3）


## r72-wintf — wintf の lib テスト（錠を退役させた crate・要件 7.2） ×30（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:31:44 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p wintf --lib` |
| 回数 / 同時プロセス | 30 / 4 |
| 期待 passed | 842 |
| 1 回の上限 | 120 秒（自動＝単独実測 1.6 秒 × 同時 4 × 10（下限 120 秒）） |
| 事前ビルド | 0.4 秒・テスト実行体 1 本（刻印 logs/r72-wintf-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 7.2・9 の条件（タスク 7.2 の反復証跡を本ハーネスで採り直し） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:30:40 | 8.7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r001.out.log` |
| 2 | 22:30:40 | 8.7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r002.out.log` |
| 3 | 22:30:40 | 8.7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r003.out.log` |
| 4 | 22:30:40 | 8.8 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r004.out.log` |
| 5 | 22:30:49 | 8.9 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r005.out.log` |
| 6 | 22:30:49 | 8.9 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r006.out.log` |
| 7 | 22:30:49 | 8.9 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r007.out.log` |
| 8 | 22:30:49 | 8.9 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r008.out.log` |
| 9 | 22:30:58 | 8.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r009.out.log` |
| 10 | 22:30:58 | 8.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r010.out.log` |
| 11 | 22:30:58 | 8.5 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r011.out.log` |
| 12 | 22:30:58 | 8.5 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r012.out.log` |
| 13 | 22:31:06 | 8.6 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r013.out.log` |
| 14 | 22:31:06 | 8.7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r014.out.log` |
| 15 | 22:31:06 | 8.7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r015.out.log` |
| 16 | 22:31:06 | 8.7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r016.out.log` |
| 17 | 22:31:15 | 6.9 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r017.out.log` |
| 18 | 22:31:15 | 6.9 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r018.out.log` |
| 19 | 22:31:15 | 6.9 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r019.out.log` |
| 20 | 22:31:15 | 7 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r020.out.log` |
| 21 | 22:31:22 | 8.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r021.out.log` |
| 22 | 22:31:22 | 8.4 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r022.out.log` |
| 23 | 22:31:22 | 8.5 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r023.out.log` |
| 24 | 22:31:22 | 8.5 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r024.out.log` |
| 25 | 22:31:30 | 8.6 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r025.out.log` |
| 26 | 22:31:31 | 8.6 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r026.out.log` |
| 27 | 22:31:31 | 8.6 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r027.out.log` |
| 28 | 22:31:31 | 8.6 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r028.out.log` |
| 29 | 22:31:39 | 5.1 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r029.out.log` |
| 30 | 22:31:39 | 5.2 | 0 | 842 | 0 | 0 | 0 | 1 | 緑 | `r72-wintf-r030.out.log` |

**30 回走らせて 緑 30・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 8.6 / 最小 5.1 / 最大 8.9）


## r91-workspace — ワークスペース全体 ×10（同時 2 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:39:58 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test --workspace` |
| 回数 / 同時プロセス | 10 / 2 |
| 期待 passed | 5865 |
| 1 回の上限 | 736 秒（自動＝単独実測 36.8 秒 × 同時 2 × 10（下限 120 秒）） |
| 事前ビルド | 2.6 秒・テスト実行体 72 本（刻印 logs/r91-workspace-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 9.1 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:32:03 | 166.3 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r001.out.log` |
| 2 | 22:32:03 | 168.3 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r002.out.log` |
| 3 | 22:34:53 | 96.5 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r003.out.log` |
| 4 | 22:34:53 | 97 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r004.out.log` |
| 5 | 22:36:30 | 71.6 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r005.out.log` |
| 6 | 22:36:30 | 71.9 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r006.out.log` |
| 7 | 22:37:42 | 71.9 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r007.out.log` |
| 8 | 22:37:42 | 72.2 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r008.out.log` |
| 9 | 22:38:55 | 62.5 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r009.out.log` |
| 10 | 22:38:55 | 62.8 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r91-workspace-r010.out.log` |

**10 回走らせて 緑 10・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 72.1 / 最小 62.5 / 最大 168.3）


## cal82-areka — custom ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:40:47 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p areka` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 16.4 秒・テスト実行体 3 本（刻印 logs/cal82-areka-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 9.5: 正体不明の 553/1 赤と同じコマンドの現在値を採る |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:40:35 | 11.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `cal82-areka-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 11.4 / 最小 11.4 / 最大 11.4）


## r95-areka — custom ×30（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:43:45 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p areka` |
| 回数 / 同時プロセス | 30 / 4 |
| 期待 passed | 1241 |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 0.4 秒・テスト実行体 3 本（刻印 logs/r95-areka-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 9.5: 正体不明の 553/1 赤と同じコマンドを負荷下で反復し再現の有無を採る |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:40:59 | 23.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r001.out.log` |
| 2 | 22:40:59 | 23.5 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r002.out.log` |
| 3 | 22:40:59 | 23.6 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r003.out.log` |
| 4 | 22:40:59 | 24.5 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r004.out.log` |
| 5 | 22:41:24 | 22 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r005.out.log` |
| 6 | 22:41:24 | 22.1 | 101 | 1237 | 1 | 1 | 0 | 1 | 赤 | `r95-areka-r006.out.log` |
| 7 | 22:41:24 | 22.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r007.out.log` |
| 8 | 22:41:24 | 22.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r008.out.log` |
| 9 | 22:41:46 | 19.5 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r009.out.log` |
| 10 | 22:41:46 | 19.7 | 101 | 1236 | 2 | 1 | 0 | 1 | 赤 | `r95-areka-r010.out.log` |
| 11 | 22:41:46 | 20.5 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r011.out.log` |
| 12 | 22:41:46 | 21.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r012.out.log` |
| 13 | 22:42:07 | 20.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r013.out.log` |
| 14 | 22:42:07 | 20.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r014.out.log` |
| 15 | 22:42:07 | 20.5 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r015.out.log` |
| 16 | 22:42:07 | 21.9 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r016.out.log` |
| 17 | 22:42:29 | 17.5 | 101 | 1237 | 1 | 1 | 0 | 1 | 赤 | `r95-areka-r017.out.log` |
| 18 | 22:42:29 | 21.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r018.out.log` |
| 19 | 22:42:29 | 21.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r019.out.log` |
| 20 | 22:42:30 | 21.5 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r020.out.log` |
| 21 | 22:42:51 | 21.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r021.out.log` |
| 22 | 22:42:51 | 21.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r022.out.log` |
| 23 | 22:42:51 | 21.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r023.out.log` |
| 24 | 22:42:51 | 21.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r024.out.log` |
| 25 | 22:43:13 | 21.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r025.out.log` |
| 26 | 22:43:13 | 21.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r026.out.log` |
| 27 | 22:43:13 | 21.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r027.out.log` |
| 28 | 22:43:13 | 21.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r028.out.log` |
| 29 | 22:43:34 | 10.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r029.out.log` |
| 30 | 22:43:34 | 11.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-r030.out.log` |

**30 回走らせて 緑 27・赤 3・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 21.3 / 最小 10.1 / 最大 24.5）

### 緑でなかった回の内訳

- **回 6・判定 赤**（終了コード 101・passed 1237・failed 1・filtered out 0・ログ `r95-areka-r006.out.log`）
  - 失敗したテスト（1 件）:
    - `placement::transition_judge::transition_signoff_tests::a_log_without_any_observation_line_is_an_error_not_an_empty_pass`

  失敗内容 `placement::transition_judge::transition_signoff_tests::a_log_without_any_observation_line_is_an_error_not_an_empty_pass`:

  ```
  
  thread 'placement::transition_judge::transition_signoff_tests::a_log_without_any_observation_line_is_an_error_not_an_empty_pass' (6304) panicked at crates\areka\src\placement\transition_signoff_tests.rs:108:28:
  一時ファイルを消せるはず: Os { code: 2, kind: NotFound, message: "指定されたファイルが見つかりません。" }
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::result::unwrap_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\result.rs:1870
     3: enum2$<core::result::Result<tuple$<>,core::io::error::Error> >::expect<tuple$<>,core::io::error::Error>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\result.rs:1183
     4: areka::placement::transition_judge::transition_signoff_tests::a_log_without_any_observation_line_is_an_error_not_an_empty_pass
               at .\src\placement\transition_signoff_tests.rs:108
     5: areka::placement::transition_judge::transition_signoff_tests::a_log_without_any_observation_line_is_an_error_not_an_empty_pass::closure$0
               at .\src\placement\transition_signoff_tests.rs:99
     6: core::ops::function::FnOnce::call_once<areka::placement::transition_judge::transition_signoff_tests::a_log_without_any_observation_line_is_an_error_not_an_empty_pass::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     7: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

- **回 10・判定 赤**（終了コード 101・passed 1236・failed 2・filtered out 0・ログ `r95-areka-r010.out.log`）
  - 失敗したテスト（2 件）:
    - `restore_seam_tests::restore_seam_clamps_balloon_display_position_but_keeps_offset_raw`
    - `restore_seam_tests::restore_seam_prefers_saved_position_over_default`

  失敗内容 `restore_seam_tests::restore_seam_prefers_saved_position_over_default`:

  ```
  
  thread 'restore_seam_tests::restore_seam_prefers_saved_position_over_default' (1916) panicked at crates\areka\src\main_restore_seam_tests.rs:91:5:
  assertion `left == right` failed: 保存位置を採用した scope が復元済みとして報告される（scg 7.3）
    left: []
   right: [0]
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::assert_failed_inner
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:434
     3: core::panicking::assert_failed<alloc::vec::Vec<usize,alloc::alloc::Global>,alloc::vec::Vec<usize,alloc::alloc::Global> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\panicking.rs:394
     4: areka::restore_seam_tests::restore_seam_prefers_saved_position_over_default
               at .\src\main_restore_seam_tests.rs:91
     5: areka::restore_seam_tests::restore_seam_prefers_saved_position_over_default::closure$0
               at .\src\main_restore_seam_tests.rs:60
     6: core::ops::function::FnOnce::call_once<areka::restore_seam_tests::restore_seam_prefers_saved_position_over_default::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     7: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `restore_seam_tests::restore_seam_clamps_balloon_display_position_but_keeps_offset_raw`:

  ```
  
  thread 'restore_seam_tests::restore_seam_clamps_balloon_display_position_but_keeps_offset_raw' (30900) panicked at crates\areka\src\main_restore_seam_tests.rs:183:5:
  assertion `left == right` failed: 保存位置を採用した scope として報告される（合流規則は関門の設置で変わらない）
    left: []
   right: [0]
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::assert_failed_inner
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:434
     3: core::panicking::assert_failed<alloc::vec::Vec<usize,alloc::alloc::Global>,alloc::vec::Vec<usize,alloc::alloc::Global> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\panicking.rs:394
     4: areka::restore_seam_tests::restore_seam_clamps_balloon_display_position_but_keeps_offset_raw
               at .\src\main_restore_seam_tests.rs:183
     5: areka::restore_seam_tests::restore_seam_clamps_balloon_display_position_but_keeps_offset_raw::closure$0
               at .\src\main_restore_seam_tests.rs:154
     6: core::ops::function::FnOnce::call_once<areka::restore_seam_tests::restore_seam_clamps_balloon_display_position_but_keeps_offset_raw::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     7: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

- **回 17・判定 赤**（終了コード 101・passed 1237・failed 1・filtered out 0・ログ `r95-areka-r017.out.log`）
  - 失敗したテスト（1 件）:
    - `restore_seam_tests::restore_seam_prefers_saved_position_over_default`

  失敗内容 `restore_seam_tests::restore_seam_prefers_saved_position_over_default`:

  ```
  
  thread 'restore_seam_tests::restore_seam_prefers_saved_position_over_default' (16860) panicked at crates\areka\src\main_restore_seam_tests.rs:31:6:
  write ghost descript: Os { code: 3, kind: NotFound, message: "指定されたパスが見つかりません。" }
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::result::unwrap_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\result.rs:1870
     3: enum2$<core::result::Result<tuple$<>,core::io::error::Error> >::expect<tuple$<>,core::io::error::Error>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\result.rs:1183
     4: areka::restore_seam_tests::plant_minimal_ghost
               at .\src\main_restore_seam_tests.rs:31
     5: areka::restore_seam_tests::restore_seam_prefers_saved_position_over_default
               at .\src\main_restore_seam_tests.rs:62
     6: areka::restore_seam_tests::restore_seam_prefers_saved_position_over_default::closure$0
               at .\src\main_restore_seam_tests.rs:60
     7: core::ops::function::FnOnce::call_once<areka::restore_seam_tests::restore_seam_prefers_saved_position_over_default::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     8: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```


## r95-areka-serial — custom ×10（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:47:41 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（7 件）） |
| 実行コマンド | `cargo test -p areka` |
| 回数 / 同時プロセス | 10 / 1 |
| 期待 passed | 1241 |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 4.3 秒・テスト実行体 3 本（刻印 logs/r95-areka-serial-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 9.5 の切り分け: 同じコマンドを同時 1 プロセスで 10 回（プロセス間の一時パス衝突が原因かを分ける） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:45:20 | 24.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r001.out.log` |
| 2 | 22:45:45 | 17.8 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r002.out.log` |
| 3 | 22:46:03 | 16.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r003.out.log` |
| 4 | 22:46:19 | 16.9 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r004.out.log` |
| 5 | 22:46:36 | 14.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r005.out.log` |
| 6 | 22:46:51 | 11.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r006.out.log` |
| 7 | 22:47:03 | 9.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r007.out.log` |
| 8 | 22:47:12 | 9.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r008.out.log` |
| 9 | 22:47:21 | 8.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r009.out.log` |
| 10 | 22:47:30 | 11.6 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r95-areka-serial-r010.out.log` |

**10 回走らせて 緑 10・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 13 / 最小 8.2 / 最大 24.3）


## r3-before-warmup — ワークスペース全体 ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:54:42 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\areka-cage-before` |
| HEAD | `327e7fd3`（作業ツリー clean） |
| 実行コマンド | `cargo test --workspace` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 1200 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.7 秒・テスト実行体 67 本（刻印 logs/r3-before-warmup-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | R-3: 移行前ツリーの暖機（冷えた初回は所要が伸びるので比較から外す） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:54:02 | 39.5 | 101 | 5256 | 2 | 7 | 0 | 58 | 赤 | `r3-before-warmup-r001.out.log` |

**1 回走らせて 緑 0・赤 1・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.5 / 最小 39.5 / 最大 39.5）

### 緑でなかった回の内訳

- **回 1・判定 赤**（終了コード 101・passed 5256・failed 2・filtered out 0・ログ `r3-before-warmup-r001.out.log`）
  - 失敗したテスト（2 件）:
    - `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`
    - `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant' (29876) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant
               at .\src\ecs\world\tick_diag_tests.rs:166
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:164
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order' (21416) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order
               at .\src\ecs\world\tick_diag_tests.rs:140
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:138
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```


## r3-before-warm2 — custom ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 22:57:28 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\areka-cage-before` |
| HEAD | `327e7fd3`（作業ツリー clean） |
| 実行コマンド | `cargo test --workspace --no-fail-fast` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 0.5 秒・テスト実行体 67 本（刻印 logs/r3-before-warm2-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | R-3: 移行前ツリーの暖機（--no-fail-fast＝最後まで走らせる形） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:55:57 | 89.4 | 101 | 5754 | 2 | 34 | 0 | 86 | 赤 | `r3-before-warm2-r001.out.log` |

**1 回走らせて 緑 0・赤 1・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 89.4 / 最小 89.4 / 最大 89.4）

### 緑でなかった回の内訳

- **回 1・判定 赤**（終了コード 101・passed 5754・failed 2・filtered out 0・ログ `r3-before-warm2-r001.out.log`）
  - 失敗したテスト（2 件）:
    - `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`
    - `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant' (37436) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant
               at .\src\ecs\world\tick_diag_tests.rs:166
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:164
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order' (22820) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order
               at .\src\ecs\world\tick_diag_tests.rs:140
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:138
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```


## r3-before — custom ×5（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 23:01:45 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\areka-cage-before` |
| HEAD | `327e7fd3`（作業ツリー clean） |
| 実行コマンド | `cargo test --workspace --no-fail-fast` |
| 回数 / 同時プロセス | 5 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 9 秒・テスト実行体 67 本（刻印 logs/r3-before-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | R-3: 移行前（327e7fd3＝本ブランチの分岐点）の所要時間 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 22:57:45 | 77.8 | 101 | 5754 | 2 | 34 | 0 | 86 | 赤 | `r3-before-r001.out.log` |
| 2 | 22:59:03 | 48.3 | 101 | 5754 | 2 | 34 | 0 | 86 | 赤 | `r3-before-r002.out.log` |
| 3 | 22:59:52 | 39.6 | 101 | 5754 | 2 | 34 | 0 | 86 | 赤 | `r3-before-r003.out.log` |
| 4 | 23:00:32 | 38.3 | 101 | 5754 | 2 | 34 | 0 | 86 | 赤 | `r3-before-r004.out.log` |
| 5 | 23:01:10 | 34.6 | 101 | 5754 | 2 | 34 | 0 | 86 | 赤 | `r3-before-r005.out.log` |

**5 回走らせて 緑 0・赤 5・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.6 / 最小 34.6 / 最大 77.8）

### 緑でなかった回の内訳

- **回 1・判定 赤**（終了コード 101・passed 5754・failed 2・filtered out 0・ログ `r3-before-r001.out.log`）
  - 失敗したテスト（2 件）:
    - `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`
    - `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant' (15340) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant
               at .\src\ecs\world\tick_diag_tests.rs:166
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:164
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order' (10328) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order
               at .\src\ecs\world\tick_diag_tests.rs:140
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:138
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

- **回 2・判定 赤**（終了コード 101・passed 5754・failed 2・filtered out 0・ログ `r3-before-r002.out.log`）
  - 失敗したテスト（2 件）:
    - `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`
    - `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant' (20516) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant
               at .\src\ecs\world\tick_diag_tests.rs:166
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:164
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order' (15272) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order
               at .\src\ecs\world\tick_diag_tests.rs:140
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:138
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

- **回 3・判定 赤**（終了コード 101・passed 5754・failed 2・filtered out 0・ログ `r3-before-r003.out.log`）
  - 失敗したテスト（2 件）:
    - `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`
    - `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant' (33548) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant
               at .\src\ecs\world\tick_diag_tests.rs:166
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:164
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order' (24376) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order
               at .\src\ecs\world\tick_diag_tests.rs:140
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:138
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

- **回 4・判定 赤**（終了コード 101・passed 5754・failed 2・filtered out 0・ログ `r3-before-r004.out.log`）
  - 失敗したテスト（2 件）:
    - `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`
    - `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order' (3460) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order
               at .\src\ecs\world\tick_diag_tests.rs:140
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:138
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant' (33364) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant
               at .\src\ecs\world\tick_diag_tests.rs:166
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:164
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

- **回 5・判定 赤**（終了コード 101・passed 5754・failed 2・filtered out 0・ログ `r3-before-r005.out.log`）
  - 失敗したテスト（2 件）:
    - `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`
    - `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant' (18312) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant
               at .\src\ecs\world\tick_diag_tests.rs:166
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:164
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`:

  ```
  
  thread 'ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order' (5420) panicked at crates\wintf\src\ecs\world\tick_diag_tests.rs:34:10:
  try_tick_world の終端（インデント 4 の閉じ括弧）が見つからない
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: core::panicking::panic_display
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:259
     3: core::option::expect_failed
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\option.rs:2260
     4: enum2$<core::option::Option<usize> >::expect<usize>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\option.rs:969
     5: wintf::ecs::world::tick_diag::tests::try_tick_world_body
               at .\src\ecs\world\tick_diag_tests.rs:34
     6: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order
               at .\src\ecs\world\tick_diag_tests.rs:140
     7: wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure$0
               at .\src\ecs\world\tick_diag_tests.rs:138
     8: core::ops::function::FnOnce::call_once<wintf::ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     9: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```


## r3-after-warm — custom ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 23:04:03 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（21 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 28.3 秒・テスト実行体 72 本（刻印 logs/r3-after-warm-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | R-3: 移行後ツリーの暖機（--no-fail-fast・移行前と同じ形） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 23:02:36 | 86.6 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r3-after-warm-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 86.6 / 最小 86.6 / 最大 86.6）


## r3-after — custom ×5（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-25 23:07:36 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `aa698693`（作業ツリー dirty（21 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast` |
| 回数 / 同時プロセス | 5 / 1 |
| 期待 passed | 5865 |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 0.8 秒・テスト実行体 72 本（刻印 logs/r3-after-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | R-3: 移行後（HEAD aa698693）の所要時間 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 23:04:05 | 44.4 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r3-after-r001.out.log` |
| 2 | 23:04:50 | 41.7 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r3-after-r002.out.log` |
| 3 | 23:05:31 | 40.2 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r3-after-r003.out.log` |
| 4 | 23:06:12 | 43.2 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r3-after-r004.out.log` |
| 5 | 23:06:55 | 40.4 | 0 | 5865 | 0 | 36 | 0 | 92 | 緑 | `r3-after-r005.out.log` |

**5 回走らせて 緑 5・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 41.7 / 最小 40.2 / 最大 44.4）


---

# タスク 8.2 の結論（要件 3.7・7.2・9.1・9.2・9.3・9.5・9.6）

実施日 2026-08-25・HEAD `aa698693`・作業ツリーは要約と赤の複写のみ変更。
以下の数値はすべて本ファイルの各節（走行 1 回につき 1 節）と `red/` の生ログから採れる。

## 1. 走行前の期待件数の採り直し（タスク 8.1 からの申し送り ⑴）

100 回規模を回す前に、対象表の期待件数が現在も正しいかを 1 回ずつ走らせて確かめた。
`-- --list` の行数は使っていない（ignored を数えてしまうため）。実測の `passed` 列から採った。

| 対象 | 対象表の値（8.1・2026-08-24） | 8.2 の実測（2026-08-25） | 採った節 |
|---|---:|---:|---|
| `workspace` | 5865 | **5865** | `cal82-workspace` |
| `wintf` | 842 | **842** | `cal82-wintf` |
| `seriko` | 200 | **200** | `cal82-seriko` |
| `wait` | 2 | **2** | `cal82-wait` |
| `kit` | 79 | **79** | `cal82-kit` |

5 つとも増減なし。対象表の更新は不要で、本走行の「件数不一致」判定はそのまま意味を持つ。

## 2. 規定回数の反復（要件 9.1・9.2・9.3・7.2）

| 節 | 実行コマンド | 回数 | 同時プロセス | 判定 | 所要秒 中央値 |
|---|---|---:|---:|---|---:|
| `r91-workspace` | `cargo test --workspace` | 10 | 2 | **緑 10**・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0 | 72.1 |
| `r92-seriko` | `cargo test -p areka-seriko --lib` | 30 | 4 | **緑 30**・以下すべて 0 | 1.1 |
| `r93-wait` | `cargo test -p areka --bins -- （待機 2 テスト）` | 30 | 4 | **緑 30**・以下すべて 0 | 3.1 |
| `r72-wintf` | `cargo test -p wintf --lib` | 30 | 4 | **緑 30**・以下すべて 0 | 8.6 |
| **合計** | | **100** | | **緑 100・緑でない回 0** | |

**完了条件の判定: 規定回数の反復で、ログ捕捉・待機・退役した錠に起因する失敗は 0 件。**
各回の `passed` は表の列に残っており、`workspace` は 10 回とも 5865、`seriko` は 30 回とも 200、
`wait` は 30 回とも 2（`filtered out` 1237）、`wintf` は 30 回とも 842 だった。

補足 3 点:

- **タスク 7.2 の反復証跡（申し送り ⑵）はここで合流した。** 7.2 は「実装 36 回＋レビュー独立 156 回が
  すべて緑」と申告していたがリポジトリに成果物が無く、完了条件の第 3 項が申告の上に立っていた。
  `r72-wintf` の 30 回は各回の `passed` 件数（842）と所要秒まで表に残るので、以後は記録で裏が取れる。
- **実行体の刻印は 8.1 の走行と同じで、それが正しい。** `logs/r72-wintf-binaries.txt` と 8.1 の
  `logs/trial-load-binaries.txt` はサイズ 21007872・更新時刻 `2026-08-24T12:05:32` が一致する。
  手順書 §5-a はこの一致を「前のコードを測っている」疑いの印とするが、本件は
  `git diff --name-only 170edd6a aa698693 -- crates` が **0 ファイル**（8.1 以降 `crates/` を 1 行も触っていない）
  なので、同じ実行体を測るのが正しい。
- **冷えた初回だけ所要が伸びる。** `r91-workspace` は回 1・2 が 166.3／168.3 秒、回 5 以降は 62.5〜72.2 秒。
  判定には影響しない（10 回とも 5865 passed）。1 回の上限は 736 秒で、最長の回でもその 23% にとどまった。

なお本走行の前に、本日の負荷下の 1 回あたりの所要を `pilot82-wintf`（4 回・同時 4）で採った（11.2〜11.4 秒）。
自動算出の上限 120 秒に対して 10 倍以上の余裕があったので、上限は既定のまま回した。

## 3. 待機の 2 テスト（要件 9.3・4）

`r93-wait` が 30 回とも緑にした 2 本は
`spine_e2e_sakura_blink_default_off_emits_nothing`（`crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs:361`）と
`spine_s4_balloon_free_onboot_completes_without_balloon_face_switch`（`crates/areka/src/emo2_boot/spine_display_tests.rs:371`）。
有界化の本体は `crates/areka/src/emo2_boot/spine.rs:414` の `settle_bounded`
（最小持続と連続静穏回数の両立で返り、`SPIN_WAIT`＝`spine.rs:331` の 30 秒を超えたら必ず返る）。
30 回とも `2 passed` で、上限で打ち切られた回は 1 回も無い。

## 4. 正体不明だった 1 件の赤（要件 9.5）

**結論: 別系統として残る。ただし「正体不明」ではなくなった——名前・場所・原因が採れた。**

元の記録は `brief.md:177`「`cargo test -p areka` が 1 回だけ 553/1 で赤（13 秒・ログ未保存でテスト名不明）」。
要件 9.5 は「①硬化後に再現しなくなったか、別系統として残るか」の記録を求めている。

### 4-a. 同じコマンドを負荷下で 30 回回した

反復の 4 対象（§2）には `cargo test -p areka` 単体が入っていないので、9.5 のために追加で回した。

| 節 | 回数 | 同時プロセス | 判定 |
|---|---:|---:|---|
| `cal82-areka` | 1 | 1 | 緑 1（現在値の採取。**1241 passed** / 1 ignored / 実行体 3 本） |
| `r95-areka` | 30 | 4 | 緑 27・**赤 3**・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0 |
| `r95-areka-serial` | 10 | 1 | 緑 10・赤 0（切り分け用） |

着手時の 554 件（553 passed ＋ 1 failed）に対し、現在の同コマンドは **1241 passed** で 2 倍以上に増えている。

### 4-b. 赤 3 回の中身（テスト名不明の赤は残さない・要件 9.4）

| 回 | 落ちたテスト | 場所と本文 |
|---:|---|---|
| 6 | `placement::transition_judge::transition_signoff_tests::a_log_without_any_observation_line_is_an_error_not_an_empty_pass` | `crates/areka/src/placement/transition_signoff_tests.rs:108` 「一時ファイルを消せるはず: Os { code: 2, kind: NotFound }」 |
| 10 | `restore_seam_tests::restore_seam_prefers_saved_position_over_default` | `crates/areka/src/main_restore_seam_tests.rs:91` `left: []` / `right: [0]` |
| 10 | `restore_seam_tests::restore_seam_clamps_balloon_display_position_but_keeps_offset_raw` | `crates/areka/src/main_restore_seam_tests.rs:183` `left: []` / `right: [0]` |
| 17 | `restore_seam_tests::restore_seam_prefers_saved_position_over_default` | `crates/areka/src/main_restore_seam_tests.rs:31` 「write ghost descript: Os { code: 3, kind: NotFound }」 |

生ログは `red/r95-areka-r006.out.log`・`red/r95-areka-r010.out.log`・`red/r95-areka-r017.out.log`。

### 4-c. 原因（2 系統とも同じ形）

どちらも**テストが使う一時パスがプロセスをまたいで固定されている**。

- `crates/areka/src/placement/transition_signoff_tests.rs:102`
  `std::env::temp_dir().join("areka-transition-signoff-empty.log")` を書き、`:108` で消す。
- `crates/areka/src/main_restore_seam_tests.rs:16-20` の `unique_temp_dir` は
  `<temp>/areka_main_restore_seam_tests_<tag>` を組み、`:24` の `plant_minimal_ghost` が
  `remove_dir_all` してから作り直す。

名前は**プロセスの中では**一意だが、同じテスト実行体が 2 つ以上同時に走ると同じ絶対パスを取り合う。
片方の削除がもう片方の「書いてから読む」の間に割り込むと、消す側は `NotFound` で落ち、
消された側は空の結果を読んで `left: []` になる。

### 4-d. 切り分けの実測

同じコマンド・同じ実行体を**同時 1 プロセス**で 10 回回すと **緑 10・赤 0**（`r95-areka-serial`）。
赤が出るのは同時 4 プロセスのときだけである。したがってこの赤は

- ログ捕捉に由来しない（捕捉を共有機構へ寄せた後も、寄せていない経路でも同じように出る形ではない）
- 待機に由来しない（落ちているのは待機を直した 2 テストではない）
- 退役した錠に由来しない（`crates/wintf` ではなく `crates/areka` のテストで、錠は元から掛かっていない）

**別系統**である。要件 9.1-9.3 が定める 4 対象の 100 回は 1 回も赤にならなかった（§2）。

### 4-e. 2026-08-05 の 553/1 と同一かは決められない

当時はテスト名もログも残っていないので、同定はできない。言えるのはここまで:

- `cargo test -p areka` の赤は硬化後も出る。ただし**複数プロセスが同時に走るときだけ**出る。
- 出る赤は 2 系統・3 テストで、いずれも一時パスの共有が原因であり、本仕様が直した 3 つの機序
  （ログ捕捉・待機・錠）のいずれでもない。
- 単独走行では 10 回とも緑で、当時の走行が単独だったならこの経路では説明できない。

**是正は本仕様の範囲外**（`crates/` の変更はタスク 8.2 の範囲外）。8.3 の台帳へ引受先つきで登記する候補として残す。

## 5. 較正を並べる（要件 9.6・3.4・8.4・10.3）

「緑が並んだ」は道具が壊れていても出る。100 回の緑がそうでないことを、
**取りこぼしを作る側**と**赤を作る側**の両方から並べる。
⑴〜⑷ は本走行と同じ日・同じ実行体で実際に走ったテスト、⑸〜⑻ は反復の道具そのものの較正である。

| # | 何を疑うか | 較正の中身 | 実測（どこで確かめたか） |
|---|---|---|---|
| ⑴ | 硬化しなければ本当に取りこぼすのか | `crates/log-capture-kit/tests/capture_calibration_test.rs:89` `bare_capture_drops_what_hardened_capture_keeps`。同じ場面・同じ発行点を**別プロセス**で 2 通り走らせ、素の捕捉が **0 件**（`:153` `child_bare_capture_drops_the_event`）・共有機構の捕捉が **1 件**（`:170` `child_hardened_capture_keeps_the_event`）であることを対比する。子が 1 件も実行せずに終了コード 0 で終わる抜け道は `:80-83` で塞いである | `logs/cal82-kit-r001.out.log` に `test bare_capture_drops_what_hardened_capture_keeps ... ok`。子 2 本は親が起こすので `ignored` として並ぶ |
| ⑵ | 迂回した呼出の見張りは既知の陽性で赤になるか | `crates/log-capture-kit/tests/with_default_guard_test.rs:679` `unlisted_returns_the_hit_whose_exception_was_dropped`（例外表から 1 件落とすと、その当たりが返ってくる）・`:698` `stale_entries_names_the_exception_that_no_longer_has_a_hit`・`:718` `scanning_finds_the_token_only_when_it_is_real_code`。例外表は 4 件（`:98-121`・件数は `:122` に逐語で持つ） | 同ログに 3 本とも `ok` |
| ⑶ | 走査そのものが空を返していないか | `crates/log-capture-kit/tests/workspace_scan_test.rs:59` `scan_tokens_reports_the_known_positive_direct_call_once`・`:288` `over_limit_returns_the_file_that_was_dropped_from_the_allow_list`・`:341` `walk_does_not_vacuously_contain_everything`・`:251` `the_limit_is_the_one_thousand_lines_the_rule_names`（閾値そのものを逐語で縛る） | 同ログに 4 本とも `ok` |
| ⑷ | 1,000 行の見張りは既知の陽性で赤になるか | `crates/log-capture-kit/tests/file_length_guard_test.rs:227` `dropping_a_known_exception_turns_the_guard_red`・`:252` `the_measurement_is_not_vacuous_and_matches_the_allow_table`。例外表は 11 件（`:61`・件数は `:109`） | 同ログに 2 本とも `ok` |
| ⑸ | 反復の道具は赤を赤と読むか | `cal-red` 節（8.1・2026-08-24）。わざと 1 本落ちる使い捨てクレートで **2 回とも判定 赤・終了コード 101**、失敗したテスト名と本文が要約へ載った | 本ファイルの `cal-red` 節・`red/cal-red-r001.out.log`／`red/cal-red-r002.out.log` |
| ⑹ | 終了コード 0 の空振りを緑にしないか | `cal-empty` 節（8.1）。フィルタの綴りを 1 文字誤らせると `0 passed; 26 filtered out`・**終了コードは 0** になるが、判定は **空振り** | 本ファイルの `cal-empty` 節 |
| ⑺ | 1 回の上限は効くか | `cal-hang` 節（8.1）。決して終わらないテストで **15.0 秒・判定 打ち切り・`test result:` 行 0 本**、走行後の残存プロセス 0 | 本ファイルの `cal-hang` 節・`red/cal-hang-r001.out.log` |
| ⑻ | 件数が黙って減っていないか | 全走行で期待 `passed` と実測を毎回突き合わせる。8.1 の最初の `kit` 節が実際に「件数不一致」を 3 回出した実例（`--list` の 81 と実測 79 の食い違い） | 本ファイルの最初の `kit` 節 |

**⑸ について、8.2 は「作っていない赤」でも同じことを実演した。**
8.1 の `cal-red` は意図的に作った赤だったが、8.2 の `r95-areka`（§4）と `r3-before`（§6）は
**こちらが作っていない赤**を道具が拾い、テスト名・panic の位置・失敗本文を要約へ載せ、
生ログを `red/` へ複写した。較正が実運用でも成立していることの直接の証拠になる。

**まとめ**: 100 回の緑は、⑴〜⑷ が「硬化が無ければ取りこぼす／見張りは既知の陽性で赤になる」ことを
**同じ走行の中で**示し、⑸〜⑻ が「この道具は赤・空振り・打ち切り・件数不一致を緑にしない」ことを
示した上で出ている。緑が道具の故障で出ている経路は、上の 8 つのいずれかが赤にならない限り成立しない。

## 6. 移行前後の所要時間（R-3・design.md `:605`「Performance & Scalability」）

常時の有効判定（テストバイナリ限定）が全体の所要時間に効くかを、同じ道具・同じ引数で 2 本のツリーを測って比べた。

### 6-a. 比較の相手は `origin/main` ではなく分岐点

手順書 §7 は `git worktree add ../areka-before origin/main` と書いているが、`origin/main` は
`7c1ca58c` で本ブランチの分岐点より **3 コミット先行**している（`12afa8e6`／`a4556290`／`7c1ca58c`＝
いずれも本仕様と無関係）。混ぜると差の出所が分からなくなるので、**分岐点 `327e7fd3`** を移行前とした。
比較用ツリーは測定後に `git worktree remove --force` で片付け済み（`git worktree list` に残っていない）。
下位モジュール `vendors/pasta` は `Cargo.toml:39` の `pasta_core` が実体を要求するので、
比較用ツリーでは `git submodule update --init --recursive` が要った（素の `worktree add` では populate されない）。

**再実行する人への注意**: 冷えたツリーをハーネスへそのまま渡してはいけない。
事前ビルドの待機は**1 回の上限と同じ値**で有界化されている（`repeat-tests.ps1:202-205`）ので、
`workspace` を同時 1 で回すと上限は 368 秒になり、一からのビルドはそれを超えて
「事前ビルドが上限に達したので打ち切った」で止まる。比較用ツリーでは先に
`cargo test --workspace --no-run` を素で通してからハーネスを呼んだ（本走行の事前ビルドは 9 秒で済んだ）。
`-TimeoutSec` を大きくして回避してもよいが、その場合は 1 回の上限も一緒に伸びる。

### 6-b. 移行前のツリーは `cargo test --workspace` では最後まで走らない

最初の走行（`r3-before-warmup`）は **39.5 秒・5256 passed・2 failed・実行体 58 本**で終わった。
移行後の 92 本に対して 58 本しか走っていない——**赤が出た時点で cargo が打ち切っている**。
この 39.5 秒は最後まで走った値ではないので、比較には使えない。
両側とも `--no-fail-fast` を付けて採り直した（`r3-before` / `r3-after`）。

移行前ツリーの 2 件の赤は**本ブランチが既に直した欠陥**である:

- 落ちるのは `ecs::world::tick_diag::tests::try_tick_world_evaluates_the_guard_before_taking_any_instant` と
  `ecs::world::tick_diag::tests::try_tick_world_runs_the_thirteen_labels_in_the_declared_order`。
  どちらも `crates/wintf/src/ecs/world/tick_diag_tests.rs:34` で「終端（インデント 4 の閉じ括弧）が見つからない」。
- 原因は本文走査が `"\n    }\n"` という **LF 固定**の文字列を探していたこと。Windows の CRLF チェックアウトでは
  永久に一致しない。両ツリーの `crates/wintf/src/ecs/world/mod.rs` を生バイトで数えると
  どちらも **CR=941 LF=941 CRLF=941 bareCR=0 bareLF=0**（＝CRLF）で、チェックアウトの差ではない。
- 本ブランチの `40ee8460`（`fix(areka-P0-test-cage-determinism): tick_diag の逐語検査を改行コード非依存にする`）が
  行末の `\r` を落としてから照合する形へ直しており、移行後ツリーは 5865 passed / 0 failed。
- 生ログは代表 1 件だけ `red/r3-before-r001.out.log`（と `.err.log`）を残した。5 回とも同じ 2 件が同じ理由で落ちる
  **決定論的な赤**で、内容が同一の複写が 7 組（約 3.6MB）になったため、残り 6 組は削除した。
  「赤は再生成できない」という `red/` の趣旨（手順書 §4）は、再現しない赤に向けたものである。
  この赤は `327e7fd3` を Windows でチェックアウトすれば毎回再現する。

### 6-c. 実測

いずれも**同時 1 プロセス**・暖機の走行を別に 1 回挟んでから 5 回。
**事前ビルドの時間は所要秒に含まれない**（表頭に別途出る）ので、比べているのは
ビルド時間ではなく**テストの実行時間**である。

| 側 | 節 | HEAD | passed / failed | 実行体 | 中央値 | 最小 | 最大 |
|---|---|---|---|---:|---:|---:|---:|
| 移行前 | `r3-before` | `327e7fd3` | 5754 / 2 | 86 | **39.6 秒** | 34.6 | 77.8 |
| 移行後 | `r3-after` | `aa698693` | 5865 / 0 | 92 | **41.7 秒** | 40.2 | 44.4 |

差は中央値で **+2.1 秒（+5.3%）**。

### 6-d. この差を「常時の有効判定の費用」と読んではいけない

2 つの理由で、この比較は +2.1 秒の出所を分離できない。

1. **測っているテスト集合が違う。** 移行後は通るテストが **111 件**多く、テスト実行体が **6 本**多い
   （本仕様が共有 crate とその検査群を新設したため）。テスト 1 件あたりに直すと
   6.88 ms → 7.11 ms（**+3.3%**）で、差はさらに縮む。
2. **移行前側の揺れが差より 20 倍大きい。** 5 回の最小〜最大が 34.6〜77.8 秒＝幅 43.2 秒あり、
   回を追うごとに 77.8 → 48.3 → 39.6 → 38.3 → 34.6 と縮み続けている（暖機 1 回では足りていない）。
   移行後側は 40.2〜44.4 秒＝幅 4.2 秒に収まっている。

**記録として言えること**: 移行後の所要時間は移行前より**大きく伸びてはいない**（中央値で +2.1 秒・+5.3%、
テスト 1 件あたりでは +3.3%）。それ以上の分解——追加テストの寄与と常時の有効判定の寄与を分ける——は、
**同一のテスト集合で硬化の有無だけを切り替えて測る**必要があり、本仕様の範囲外である。
分離が要るなら別途起票すること（8.3 の台帳へ回す候補）。

## 7. 記録の正本（タスク 8.1 からの申し送り ⑶）

`logs/` は非追跡なので、squash-merge 後に残るのは本 `summary.md` と `red/` だけになる。
8.2 の完了条件が言う「保存されたログで裏付けられている」の実体は、次の 3 つで、いずれも追跡される:

1. **各回の `test result:` の要約値**が回ごとに 1 行ある（表の `passed` / `failed` / `ignored` / `filtered` / `実行体` の 5 列）。
   縮約版を別ファイルとして持つ必要は無い——表がその縮約版そのものである。
2. **緑でなかった回**は、テスト名・panic の位置・失敗本文が本文へ転記される（§4-b・§6-b がその実例）。
3. その回の**生ログが `red/` へ複写**される（現在 7 組・約 919KB）。

したがって別途の縮約ファイルは作らない。**本 `summary.md` を記録の正本とする。**
（正本であることの台帳への登記そのものは 8.3 が行う。）

## 8. 8.3 へ渡す残り

| 項目 | 中身 | 状態 |
|---|---|---|
| 一時パスの共有による赤 | §4-c の 2 系統・3 テスト（`transition_signoff_tests.rs:102`・`main_restore_seam_tests.rs:16-20`）。同時 4 プロセスで 30 回中 3 回。是正は `crates/` の変更なので 8.2 の範囲外 | **引受先未定**（8.3 で起票先を決めること） |
| 所要時間の分離 | §6-d。同一のテスト集合で硬化の有無だけを切り替える測り方が要る | **引受先未定**（要なら起票） |
| 記録の正本の宣言 | §7 の結論（`summary.md` を正本とし、縮約ファイルは作らない） | 8.3 が台帳へ登記 |
| 錠の退役の安全性の主張（`tasks.md:331` の申し送り） | 錠が付随的に守り得た状態の列挙から、`command_batch_tests.rs` が `with_forced_batch_begin_failure`（`command.rs:366`・呼出 5 箇所）で実際に動かす `FORCE_BATCH_BEGIN_FAILURE`（`command.rs:361`）が漏れていた。**本タスクで数え直した正しい形**: `command.rs` と `transition_diag.rs` の static は **6 個**で、うち **5 個が `thread_local!` の中**（`command.rs:70` `SELF_INITIATED_DEPTH`・`:256` `WINDOW_POS_COMMANDS`・`:361` `FORCE_BATCH_BEGIN_FAILURE`／`transition_diag.rs:655` `TICK_MIRROR`・`:728` `FLUSH_START`）、**例外は退役した錠の内側の `LOCK`（`command.rs:100`）1 個だけ**——これは `#[cfg(test)]` 関数 `lock_self_initiated_for_test`（`:99`）の本体にあるプロセス共有の static である。結論（スレッド局所なので錠が無くても並列実行で干渉しない）は変わらず、`r72-wintf` の 30 回全緑（§2）がその実測にあたる | 8.3 が台帳へ登記 |

## cal106-areka — custom ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 11:36:43 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `f8d6fb86`（作業ツリー clean） |
| 実行コマンド | `cargo test -p areka` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 31.9 秒・テスト実行体 3 本（刻印 logs/cal106-areka-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 12.6: 10.6 の反復に先立ち cargo test -p areka の期待件数を採り直す（--list の行数は使わない） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 11:36:30 | 12.9 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `cal106-areka-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 12.9 / 最小 12.9 / 最大 12.9）


## r106-areka — custom ×30（同時 4 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 11:39:48 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `f8d6fb86`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test -p areka` |
| 回数 / 同時プロセス | 30 / 4 |
| 期待 passed | 1241 |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 0.4 秒・テスト実行体 3 本（刻印 logs/r106-areka-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 12.6: 一時パスの窓口移行（10.2/10.3/10.4/10.7）後に、r95-areka と同じ条件（cargo test -p areka・30 回・同時 4 プロセス・期待 1241）で赤が消えたことを示す |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 11:36:58 | 20.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r001.out.log` |
| 2 | 11:36:58 | 20.8 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r002.out.log` |
| 3 | 11:36:58 | 21 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r003.out.log` |
| 4 | 11:36:58 | 21.6 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r004.out.log` |
| 5 | 11:37:19 | 21.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r005.out.log` |
| 6 | 11:37:19 | 22.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r006.out.log` |
| 7 | 11:37:19 | 22.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r007.out.log` |
| 8 | 11:37:20 | 22.6 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r008.out.log` |
| 9 | 11:37:42 | 20.6 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r009.out.log` |
| 10 | 11:37:42 | 20.8 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r010.out.log` |
| 11 | 11:37:42 | 21 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r011.out.log` |
| 12 | 11:37:42 | 21.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r012.out.log` |
| 13 | 11:38:04 | 22 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r013.out.log` |
| 14 | 11:38:04 | 22.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r014.out.log` |
| 15 | 11:38:04 | 22.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r015.out.log` |
| 16 | 11:38:04 | 22.6 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r016.out.log` |
| 17 | 11:38:27 | 20.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r017.out.log` |
| 18 | 11:38:27 | 21.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r018.out.log` |
| 19 | 11:38:27 | 21.4 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r019.out.log` |
| 20 | 11:38:27 | 22.5 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r020.out.log` |
| 21 | 11:38:49 | 20 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r021.out.log` |
| 22 | 11:38:49 | 20.2 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r022.out.log` |
| 23 | 11:38:49 | 22.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r023.out.log` |
| 24 | 11:38:50 | 22.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r024.out.log` |
| 25 | 11:39:12 | 20.3 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r025.out.log` |
| 26 | 11:39:12 | 21 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r026.out.log` |
| 27 | 11:39:12 | 22.6 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r027.out.log` |
| 28 | 11:39:12 | 22.7 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r028.out.log` |
| 29 | 11:39:35 | 11.7 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r029.out.log` |
| 30 | 11:39:35 | 13.1 | 0 | 1241 | 0 | 1 | 0 | 3 | 緑 | `r106-areka-r030.out.log` |

**30 回走らせて 緑 30・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 21.2 / 最小 11.7 / 最大 22.7）


## cal106-red — custom ×2（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 11:40:15 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `f8d6fb86`（作業ツリー dirty（1 件）） |
| 実行コマンド | `cargo test --manifest-path <temp>\areka-cage-calibration\redcal\Cargo.toml --lib` |
| 回数 / 同時プロセス | 2 / 1 |
| 期待 passed | 1 |
| 1 回の上限 | 1800 秒（custom の既定 1800 秒（単独実測が無いため）） |
| 事前ビルド | 0.2 秒・テスト実行体 1 本（刻印 logs/cal106-red-binaries.txt） |
| i686 成果物の検査 | 実施 |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 要件 12.6 の較正: 10.6 の全緑が空虚でないことを示すため、同じ道具・同じ日に意図的な赤を出して判定が赤になることを確かめる |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 11:40:14 | 0.3 | 101 | 1 | 1 | 0 | 0 | 1 | 赤 | `cal106-red-r001.out.log` |
| 2 | 11:40:15 | 0.1 | 101 | 1 | 1 | 0 | 0 | 1 | 赤 | `cal106-red-r002.out.log` |

**2 回走らせて 緑 0・赤 2・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 0.2 / 最小 0.1 / 最大 0.3）

### 緑でなかった回の内訳

- **回 1・判定 赤**（終了コード 101・passed 1・failed 1・filtered out 0・ログ `cal106-red-r001.out.log`）
  - 失敗したテスト（1 件）:
    - `tests::redcal_this_one_fails_on_purpose`

  失敗内容 `tests::redcal_this_one_fails_on_purpose`:

  ```
  
  thread 'tests::redcal_this_one_fails_on_purpose' (30604) panicked at src\lib.rs:11:9:
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

- **回 2・判定 赤**（終了コード 101・passed 1・failed 1・filtered out 0・ログ `cal106-red-r002.out.log`）
  - 失敗したテスト（1 件）:
    - `tests::redcal_this_one_fails_on_purpose`

  失敗内容 `tests::redcal_this_one_fails_on_purpose`:

  ```
  
  thread 'tests::redcal_this_one_fails_on_purpose' (31640) panicked at src\lib.rs:11:9:
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

## タスク 10.6 — 一時パスの窓口移行の前後（要件 12.6）

**結論: 同じコマンド・同じ回数・同じ同時プロセス数で、緑 27・赤 3 → 緑 30・赤 0。**

### 10.6-a. 対比

| 項目 | 移行前 `r95-areka`（タスク 8.2・2026-08-25） | 移行後 `r106-areka`（タスク 10.6・2026-08-27） |
|---|---|---|
| HEAD | `aa698693` | `f8d6fb86` |
| 実行コマンド | `cargo test -p areka` | 同左 |
| 回数 / 同時プロセス | 30 / 4 | 同左 |
| 期待 passed | 1241 | 1241（走行前に採り直した＝10.6-b） |
| 1 回の上限 | 1800 秒（custom の既定） | 同左 |
| 判定 | 緑 27・**赤 3**・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0 | **緑 30**・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0 |
| 所要秒（中央値 / 最小 / 最大） | 21.3 / 10.1 / 24.5 | 21.2 / 11.7 / 22.7 |
| 道具の終了コード | 1 | **0** |

各回の件数は上の `r106-areka` 節の表から読める——**30 回すべて passed 1241 / failed 0 / ignored 1 / filtered 0 / 実行体 3 本・終了コード 0**。
生ログは `logs/r106-areka-r001.out.log` 〜 `-r030.out.log` の 30 本（同数の `.err.log` つき）。
`red/` へ複写された回は 1 つも無い（赤・ビルド失敗・打ち切りが 0 だったため）。

### 10.6-b. 走行前に期待件数を採り直した（タスク 8.2 の申し送り）

**`--list` の行数は使っていない。** 反復の道具で 1 回だけ走らせ、要約の passed 列を読む形を採った（`cal106-areka` 節・`repeat-tests.md` §2）。

- `cargo test -p areka` は実行体 3 本で、`test result:` 行は **1238 + 1 + 2 = 1241 passed**
  （`logs/cal106-areka-r001.out.log` の `:1243`・`:1249`・`:1256`）。
- タスク 8.2 の `cal82-areka`（1241）と同値。移行はテスト本数を変えていない（要件 12.3 の主張がここでも成り立つ）。
- 事前ビルドの刻印 `logs/cal106-areka-binaries.txt` は 3 本とも更新時刻 `2026-08-27T02:36`（UTC）で、
  **移行後の実行体を測っている**（前周の古い実体を黙って測る事故の否定＝`repeat-tests.md` §5-a）。

### 10.6-c. 消えた赤の出所

`r95-areka` の赤 3 回は 2 系統・3 テストで、いずれも**テストが使う一時パスがプロセス間で共有されていた**ことが原因だった（§4-b・§4-c）。

| 移行前に落ちていた場所 | 是正 |
|---|---|
| `crates/areka/src/placement/transition_signoff_tests.rs:102` — `env::temp_dir()` の下に固定名のファイルを書き、`:108` で消す | タスク 10.2 で一時パスの窓口 crate（`crates/temp-path-kit`）へ寄せた |
| `crates/areka/src/main_restore_seam_tests.rs:16-20` — `unique_temp_dir` は名前に反しプロセス間では一意でなく、`plant_minimal_ghost`（`:24`）が入口で `remove_dir_all` して隣のプロセスの前提を消す | 同上 |

窓口は名前に `std::process::id()` と単調増加の連番を含め、破棄時に再帰削除する（要件 12.1・タスク 10.1）。
窓口を迂回する新設はタスク 10.5 の見張りが検知する。

### 10.6-d. 全緑が空虚でないことの較正

「30 回とも緑」は道具が壊れていても出る。**同じ道具・同じ日に意図的な赤を出して**、判定が赤になることを確かめた（`cal106-red` 節）。

- `repeat-tests.md` §6-a の使い捨てクレート（`<temp>/areka-cage-calibration/redcal`）を 2 回走らせ、
  **2 回とも 判定 赤・終了コード 101・`1 passed; 1 failed`**。
- 失敗したテスト名 `tests::redcal_this_one_fails_on_purpose` と失敗本文が要約へ載り、
  生ログが `red/cal106-red-r001.out.log`・`red/cal106-red-r002.out.log` へ複写された。
- 道具そのものの終了コードは **1**。

したがって `r106-areka` の終了コード 0（緑 30）は、赤を読めない道具が出した緑ではない。

### 10.6-e. 残す注記

- **`r95-areka-serial`（同時 1 プロセス × 10 回）に対応する移行後の走行は採っていない。** 移行前はそれが
  「赤が出るのは同時 4 プロセスのときだけ」（§4-d）を示す切り分けとして要ったが、移行後は同時 4 プロセスでも
  赤が 0 なので、単独走行を足しても主張は強くならない。
- 本走行が測るのは `cargo test -p areka` の範囲＝要件 12.6 が名指しする対象そのものである。
  `areka-ghost`（10.3）・`areka-parsers`（10.4）・`areka-sylphya`（10.7）の移行前後の A/B は
  各タスクの記録（`tasks.md` の `## Implementation Notes`）にあり、本節は重複させない。
- 走行時に `crates/` は 1 行も触っていない（`git status --porcelain -- crates/` が 0 件）。**上の節の HEAD 欄が
  「作業ツリー dirty（1 件）」と記すのはこのファイル自身**——道具が走行中に要約を追記していくためである
  （レビューの指摘で「clean」から言い換えた。節どうしで字面が食い違って読めた）。本タスクが増やしたのは
  本ファイルの追記と `logs/`（非追跡）・`red/cal106-red-*`（較正の赤）だけである。

## ab-on-1 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:03:20 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（3 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4545 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.5 秒・テスト実行体 74 本（刻印 logs/ab-on-1-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=on` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | A/B 交互 第1区（常駐あり）・除外集合は exclusion/exclusion-skip-args.txt（183 値） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:00:45 | 34.4 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-1-r001.out.log` |
| 2 | 13:01:20 | 40.8 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-1-r002.out.log` |
| 3 | 13:02:01 | 39.2 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-1-r003.out.log` |
| 4 | 13:02:40 | 39.5 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-1-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.4 / 最小 34.4 / 最大 40.8）


## ab-off-1 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:06:14 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4545 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/ab-off-1-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=off` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | A/B 交互 第2区（常駐なし）・除外集合は同上 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:03:34 | 39.3 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-1-r001.out.log` |
| 2 | 13:04:13 | 40.1 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-1-r002.out.log` |
| 3 | 13:04:54 | 40.1 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-1-r003.out.log` |
| 4 | 13:05:34 | 40.2 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-1-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 40.1 / 最小 39.3 / 最大 40.2）


## ab-on-2 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:09:06 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4545 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.5 秒・テスト実行体 74 本（刻印 logs/ab-on-2-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=on` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | A/B 交互 第3区（常駐あり）・除外集合は同上 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:06:27 | 37.9 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-2-r001.out.log` |
| 2 | 13:07:05 | 40.2 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-2-r002.out.log` |
| 3 | 13:07:46 | 40.3 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-2-r003.out.log` |
| 4 | 13:08:26 | 39.6 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-2-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.9 / 最小 37.9 / 最大 40.3）


## ab-off-2 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:11:55 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4545 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/ab-off-2-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=off` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | A/B 交互 第4区（常駐なし）・除外集合は同上 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:09:18 | 38.3 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-2-r001.out.log` |
| 2 | 13:09:57 | 39.7 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-2-r002.out.log` |
| 3 | 13:10:37 | 38.6 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-2-r003.out.log` |
| 4 | 13:11:15 | 39.8 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-2-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.2 / 最小 38.3 / 最大 39.8）


## ab-on-3 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:14:47 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4545 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/ab-on-3-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=on` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | A/B 交互 第5区（常駐あり）・除外集合は同上 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:12:07 | 38.9 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-3-r001.out.log` |
| 2 | 13:12:46 | 40.4 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-3-r002.out.log` |
| 3 | 13:13:27 | 39.7 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-3-r003.out.log` |
| 4 | 13:14:07 | 39.8 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-on-3-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.8 / 最小 38.9 / 最大 40.4）


## ab-off-3 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:17:38 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4545 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.5 秒・テスト実行体 74 本（刻印 logs/ab-off-3-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=off` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | A/B 交互 第6区（常駐なし）・除外集合は同上 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:14:59 | 38.4 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-3-r001.out.log` |
| 2 | 13:15:37 | 40.3 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-3-r002.out.log` |
| 3 | 13:16:18 | 40.2 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-3-r003.out.log` |
| 4 | 13:16:58 | 40 | 0 | 4545 | 0 | 31 | 1354 | 95 | 緑 | `ab-off-3-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 40.1 / 最小 38.4 / 最大 40.3）


## ab-base — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:21:57 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（4 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/ab-base-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | （無し） |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | A/B の基準線（除外なし・環境変数なし＝既定）。除外した分の所要と、既定の挙動が変わっていないことの対照 |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:18:59 | 37.5 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab-base-r001.out.log` |
| 2 | 13:19:37 | 46.8 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab-base-r002.out.log` |
| 3 | 13:20:24 | 45.8 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab-base-r003.out.log` |
| 4 | 13:21:10 | 46.5 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab-base-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 46.2 / 最小 37.5 / 最大 46.8）


## 11.2 常時化の費用の A/B — 同一実行体・同一テスト集合での比較（2026-08-27）

> **⚠ この節（§1〜§8）は 2026-08-27 の差し戻し 1 巡目で撤回・差し替えた。数字を引くな。**
> 除外集合が**共有捕捉窓を使うテスト 96 件を構造的に取りこぼしており**（走査器の供給条件が
> 「自己テストを持たないファイル」に限られていたため、`crates/areka-emo-atlas/src/log_capture.rs`
> と `crates/areka-emo-compose/src/log_capture.rs` の包みが供給者から外れていた）、
> **本節の 125 / 183 / 1,351 / 1,354 / 4,545 とそれに基づく所要秒はすべて別の集合の数字**である。
> §4 の「上の 24 回全緑がその証拠にあたる」も撤回する（96 件が走り続けても全緑だった）。
> §1 の 2 番目の箇条書き（判定を読む檻が届いたことの確認になる、という主張）も過大である。
> §5 の「12 組中 4 組は符号が逆」は数え違いで、当時の実測は 5 組だった。
> **正本は下の「11.2（採り直し）」の節。** 本節は撤回の記録として原文のまま残す
> （生の走行記録 `ab-base` ／ `ab-on-1` … `ab-off-3` の 7 節も、実際に走った証跡なので残す）。

> 上の 7 節（`ab-base` ／ `ab-on-1` … `ab-off-3`）は反復の仕組みが自動で追記した生の記録で、
> 本節はその読み方と結論である（要件 13.3・13.4・13.5／設計 `#### C9`）。
> **要件 13.5 が求める申し送り台帳への登記はタスク 12.1 が行う。本節がその原文である。**

### 1. 何を切り替えて測ったか

常駐 probe の確立（`crates/log-capture-kit/src/probe.rs` の `ensure_interest_probes`）だけを
環境変数 `AREKA_LOG_CAPTURE_PROBES` で切り替えた。値は逐語の `on`（常駐する＝未設定と同じ）と
`off`（常駐しない）で、それ以外は panic する（タスク 11.1 で実装）。

**同一のテスト実行体を測ったことの確認**（要件 13.1）。反復の仕組みは各区で
`cargo test … --no-run --message-format=json` が解決したテスト実行体のパス・サイズ・更新時刻を
`logs/<札>-binaries.txt` へ刻む。7 区すべての刻印が**バイト単位で同一**（本文の md5 先頭 16 桁が
7 区とも `94a622d11af3c58a`）で、事前ビルドはいずれも 0.5〜0.6 秒＝**やることが無かった**。
環境変数は実行時に `std::env::var_os` で読むだけなので cargo の再ビルドを起こさない、という
前提が実測で裏を取れている（起こしていたらその時間が測定を汚していた）。

**環境変数が測定対象のプロセスまで届いたことの確認**（2 通りで確かめた）。

- 綴りを誤った値で走らせると panic する性質を使った較正。`AREKA_LOG_CAPTURE_PROBES=typo` で
  `-Target kit` を 1 回走らせると **27 件が赤**になり、本文は逐語で
  「`AREKA_LOG_CAPTURE_PROBES` の値が不正: "typo"」だった。届いていなければ既定側が静かに緑で
  通る（＝立てたつもりで立っていない事故）ので、これは**赤を作れることの確認**でもある。
- 両側で走る檻 `probe::tests::the_decision_agrees_with_what_the_environment_actually_says` が
  `interest_probes_enabled()` を読み、指定と判定の食い違いを赤にする。この檻は共有 crate の
  `src/` にあり**除外集合に入っていない**（`exclusion/exclusion-tests.txt` に不在）ので、
  24 回すべてで実際に走って緑だった。

### 2. 除外したテストの集合とその根拠

**特定は観測ではなく静的な列挙で行った。** 起草時の設計 C9 は「常駐なし側で赤になるテストを
観測して除外する」だったが、タスク 11.1 の実測で**赤の集合は非決定**だと分かっている
（8 回の走行で不変に赤なのは較正テスト 1 本だけ・他は 1/8〜5/8 で出入り・5 本除外すると 6 本目が
出る）。無効側の赤は毒化の競合が起こすもので、それは本仕様が消そうとしている病そのものだから、
並列の巡り合わせに依存するのが当然だった。

列挙の道具は `verification/capture-window-tests.py`（本タスクで新設）。種は共有 crate
`log-capture-kit` の**窓を開く公開 API 5 本**（`capture` / `capture_lines` / `count_levels` /
`capture_under_filter` / `install_global_capture_all`）と**共有機構を迂回する素の呼出 3 語**
（`with_default` / `set_default` / `set_global_default`）。各 crate の薄い包み
（`test_log_capture.rs` の `capture`、`placement/test_support.rs` の `capture_logs` など）を辿る
ため、`log_capture_kit` を参照し `#[test]` を 1 本も持たないファイルで定義された非テスト関数の
名前を呼出語へ足す不動点を回し、足した語は**定義元の crate の中だけ**で当てる。

| 実測（2026-08-27・HEAD `87a640de`） | 値 |
|---|---|
| 走査したソース | 1,035 ファイル（`crates/**/*.rs`・コメント除去後） |
| 包みの語を提供したファイル | 14 件 |
| 走査語（不動点後・適用範囲つき） | 18 語 |
| 当たったファイル | 125 件 |
| 除外の粒度 | ファイル単位（当たったファイルの全テストを除外） |
| `--skip` に渡した値 | 183 個（`exclusion/exclusion-skip-args.txt`） |
| 除外したテスト（完全な名前） | 1,351 個（`exclusion/exclusion-tests.txt`） |

除外は**両側から同じフィルタ**で行った。走らせた命令は 7 区すべてで

    cargo test --workspace --no-fail-fast -- --skip <183 個の値>

で、`--no-fail-fast` は必須である（既定の fail-fast だと常駐なし側は最初の 1 本で打ち切られ、
所要時間が比較値にならない）。

**除外の粒度をファイル単位にした理由。** テスト単位まで絞ると除外は 1,324 個へ減るが、
`--skip` の値が 319 個・約 28.8KB になり Windows のコマンドラインの上限 32,767 文字に迫る
（ファイル単位は 183 個・約 9.9KB）。過剰除外の側に倒しても測定の意味は保たれる——常駐の代償は
捕捉テストの中だけでなくワークスペース全体に及ぶ（probe が 2 個居ると `has_just_one` が偽になり、
あらゆる発火点が毎回の判定を通る）ので、差が出るなら残り 4,545 件のほうにも出る。

### 3. 除外が本当に効いたことの較正

**終了コード 0 は空振りでも返る。** `--skip` の名前を綴り誤ると、除外したつもりのテストが走り
続けたまま緑で通る。そこで**当たる件数を先に予言して突合**した。`--list` の全 5,930 行に対して
183 個の値を部分一致させると **1,354 行**が当たる、と走査側が予言し、実走の `filtered out` が
**24 回すべて 1,354**（`passed` 4,545・`failed` 0・`ignored` 31・実行体 95 本も全回一致）。
除外なしの基準線 `ab-base` は `passed` 5,894・`ignored` 36・`filtered out` 0 で、
**5,894 + 36 = 5,930** と **4,545 + 31 + 1,354 = 5,930** が一致する。
5,894 − 4,545 = 1,349 と 36 − 31 = 5 の和も 1,354 で、算の閉じ方が 3 通りとも合う。

走査の道具そのものも既知の答えで較正した（`--calibrate`）。純粋な部分（コメント除去・語の左端の
アンカー・関数の範囲・テスト属性の判定）を当たる／当たらない両側の見本で縛り、実データ側では
実在する見張り `crates/log-capture-kit/tests/with_default_guard_test.rs` の例外表 4 件・別表 2 件を
**別の実装で逐語再現**する（0 件なら緑、にならないよう陽性を要求する）。

### 4. 較正では止まらず、実走が掘り当てた穴（記録）

**別名の取り込みを追わない走査は、当たりが 0 件でも緑で通る。**
`crates/areka/src/emo2_boot/frame_diag_route_tests.rs:24` ほか 3 ファイルが
`capture_logs as capture_diag_logs` の形で窓口を取り込んでおり、初版の走査はこの 4 ファイルを
1 件も拾わなかった。較正 4 種は全部緑のままで、**常駐なし側を 3 回走らせた 3 回目に**
`emo2_boot::frame::diag_route_tests::boot_without_any_dpi_change_emits_no_dpi_reproject_record` が
赤になって初めて露見した（1 回目・2 回目は緑。非決定なので 1 回の走行では出ない）。
別名を追う処理と、その 4 ファイルを既知の答えとする較正を足して塞いだ。
**教訓は「除外集合の較正は静的な突合だけでは閉じない・常駐なし側を複数回走らせるまでは
除外集合が足りている証拠が無い」で、上の 24 回全緑がその証拠にあたる。**

### 5. 数字

いずれも 1 回の走行の所要秒（`repeat-tests.ps1` が測る壁時計。事前ビルドは含まない）。
機械の状態が時間とともに動くので **4 回ずつの区を交互に 6 区**（on → off → on → off → on → off）
並べ、各側 12 回を採った。

**回数と並べ方の根拠。** タスク 8.2 では移行前側**単独**の散らばりが差の 20 倍あり、少ない回数では
何も言えないと分かっていた。そこで片側の四分位が採れる下限（8 回）を上回り、かつ全体の壁時計が
20 分に収まる上限として**片側 12 回**を採った。片側をまとめて走らせると差が機械の状態の変化を
拾うので、区を交互に並べて線形な変動を打ち消している。結果として散らばりは差の 26 倍で、
**回数を増やしても結論の向きは変わらない見込み**である（この差を散らばりから分離するには
桁で多い回数が要り、そこまでして得られるのは「0.3 秒より小さい」の桁を 1 つ詰めることでしかない）。

| 側 | n | 中央値 | 平均 | 最小 | 最大 | 四分位（下・上） | 標準偏差 |
|---|---:|---:|---:|---:|---:|---|---:|
| A 常駐あり（`on`） | 12 | **39.65** | 39.23 | 34.4 | 40.8 | 38.97 / 40.27 | 1.63 |
| B 常駐なし（`off`） | 12 | **39.90** | 39.58 | 38.3 | 40.3 | 38.78 / 40.18 | 0.72 |
| 参考: 除外なし・環境変数なし | 4 | 46.15 | 44.15 | 37.5 | 46.8 | — | — |

- **中央値の差（B − A）= +0.25 秒（+0.63%）。符号は「常駐なしのほうが遅い」向き**である。
- 各区の先頭回はどちらの側でもその区の最速（事前ビルド直後の暖まり）なので、先頭回を落として
  9 回ずつで採り直しても **A 39.8 / B 40.1・差 +0.30 秒**で、向きも大きさも変わらない。
- 位置を対応させた 12 組の差の中央値は **+0.20 秒**で、12 組中 4 組は符号が逆（A のほうが遅い）。

### 6. 結論（要件 13.3）

**差は散らばりに埋没している。そのままを結論とする。**

A 側単独の実測値の幅は **6.4 秒**（34.4〜40.8）で、これは中央値の差 0.25 秒の **26 倍**である。
B 側単独でも幅は 2.0 秒＝差の 8 倍。四分位の区間（A 38.97〜40.27 / B 38.78〜40.18）は
ほぼ完全に重なる。そのうえ**差の符号は硬化の代償という仮説と逆**——常駐を切ったほうが 0.25 秒
遅い——なので、この 0.25 秒を「硬化の費用」と読むことはできない。

**したがって「常駐の仕掛けが `cargo test` の所要時間に与える影響は、この測り方の分解能では
検出できない」が結論である。** 「速くなった」でも「遅くなった」でもなく、**上限として
「あったとしても片側 12 回の中央値で 0.3 秒（約 0.7%）より小さい」**としか言えない。
タスク 8.2 の +2.1 秒 / +5.3%（移行前ツリーとの比較）は、集合が違い散らばりが差の 20 倍だった
ので分離不能に終わったが、**今回は集合を揃えても差そのものが出なかった**——8.2 の +2.1 秒は
テストが 111 件・実行体が 6 本多いことの側に帰属する、と読むのが実測に合う。

### 7. この数字が測っていないもの（要件 13.5）

「測った、問題なかった」では済まない。**測っていない範囲を具体に書く。**

1. **除外した 1,351 個のテストの所要は測っていない。** 除外なしの基準線が中央値 46.15 秒、
   除外後が 39.65／39.90 秒なので、除外した分はおよそ 6.3 秒＝全体の 13〜14% にあたる。
   そこに常駐の代償が乗っているかどうかは、本測定は**何も言っていない**（その集合は常駐なし側で
   赤になるので、そもそも所要時間を比べられない）。
2. **本番プロセスは 1 秒も測っていない。** 測ったのはテスト実行体だけである。常駐 probe は
   dev-dependency 経由でテストビルドにしか入らない設計なので本番には乗らない、という
   **構造の主張**は別に立っているが（`with_default_guard_test.rs` の製品側依存の見張り）、
   本測定はその主張の裏を取っていない。
3. **1 台の機械の 1 日の状態しか測っていない。** 2026-08-27 の 13 時台・同一ワークツリー・
   同一 HEAD（`87a640de`）・cargo 1 版・Defender の再スキャンなど他プロセスの負荷は制御していない。
   4 回ずつの交互で線形な変動は打ち消しているが、周期的な負荷や別の機械での再現は見ていない。
4. **同時に複数プロセスで走らせたときの差は測っていない。** `-Parallel 1`（負荷なし）だけである。
   常駐の代償が効くとすれば interest の判定回数が増える経路なので、機械が飽和した状態での
   振る舞いは別の測定になる。
5. **`--test-threads` を変えたときの差も測っていない。** 既定の並列度だけである。
6. **どこに時間が行っているかは測っていない。** 測ったのは `cargo test` 1 回の壁時計の合計で、
   `Interest::sometimes` 常態化によって増えるはずの判定 1 回あたりの費用は見ていない。
   「合計に出ない」ことは「1 回あたりが 0」を意味しない。
7. **除外集合が十分だったことは 24 回の全緑までしか言えない。** 別名の穴（第 4 節）は 3 回目で
   初めて出た類のもので、走査に別の形の穴が残っている可能性を 24 回の全緑は排除しない。

### 8. 再現の手順

    cargo test --workspace -- --list > list.txt
    python .kiro/specs/areka-P0-test-cage-determinism/verification/capture-window-tests.py \
        --root . --list list.txt --out-dir <出力先> --calibrate
    # 出力の exclusion-skip-args.txt の各行の前に --skip を挟んで -TestArgs へ渡す
    verification/repeat-tests.ps1 -Target custom \
        -CargoArgs test,--workspace,--no-fail-fast -TestArgs <上記> \
        -Times 4 -ExpectPassed 4545 -Tag <札> -TimeoutSec 900 \
        -EnvVars AREKA_LOG_CAPTURE_PROBES=on   # もう一方は =off

`-EnvVars` は本タスクで反復の仕組みへ足した引数である（既定は何も足さない＝既存の呼出は不変）。
読み方は `repeat-tests.md` §10。

## cal112-typo — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:49:52 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（9 件）） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 指定なし |
| 1 回の上限 | 120 秒（自動＝単独実測 2.4 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 0.2 秒・テスト実行体 6 本（刻印 logs/cal112-typo-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=typo` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し: 環境変数が測定対象のプロセスまで届いたことの較正（不正値で赤を作る・証跡をディスクへ残す） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:49:51 | 0.2 | 101 | 3 | 27 | 0 | 0 | 1 | 赤 | `cal112-typo-r001.out.log` |

**1 回走らせて 緑 0・赤 1・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 0.2 / 最小 0.2 / 最大 0.2）

### 緑でなかった回の内訳

- **回 1・判定 赤**（終了コード 101・passed 3・failed 27・filtered out 0・ログ `cal112-typo-r001.out.log`）
  - 失敗したテスト（27 件）:
    - `capture::tests::captures_event_whose_callsite_another_thread_registered_before_the_window`
    - `capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window`
    - `capture::tests::captures_trace_level_events`
    - `capture::tests::declares_failure_when_the_sentinel_is_not_captured - should panic`
    - `capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads`
    - `capture::tests::ensure_interest_probes_is_idempotent_across_threads`
    - `capture::tests::extracts_events_even_while_the_shared_sink_is_still_held`
    - `capture::tests::sentinel_is_removed_before_returning`
    - `event::tests::capture_lines_does_not_leak_the_sentinel`
    - `event::tests::capture_lines_returns_formatted_lines_and_the_closure_result`
    - `event::tests::count_levels_counts_each_level`
    - `event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live`
    - `event::tests::field_names_sorted_is_ascending_and_includes_message`
    - `event::tests::field_str_is_none_for_values_that_did_not_come_through_record_str`
    - `event::tests::field_str_returns_the_raw_value_and_field_returns_the_debug_representation`
    - `event::tests::fields_map_is_name_to_debug_representation`
    - `event::tests::format_line_is_byte_identical_to_current_formatting_code`
    - `event::tests::level_fields_matches_verbatim_fixture`
    - `event::tests::level_is_rendered_with_display_not_debug`
    - `event::tests::level_target_fields_matches_verbatim_fixture`
    - `event::tests::message_is_the_body_and_is_empty_when_absent`
    - `event::tests::missing_field_is_none`
    - `global::tests::fails_explicitly_when_a_different_global_is_already_installed - should panic`
    - `global::tests::installs_once_and_the_second_call_returns_the_same_buffer`
    - `probe::tests::establishing_the_probes_is_idempotent`
    - `probe::tests::the_decision_agrees_with_what_the_environment_actually_says`
    - `probe::tests::the_environment_is_read_exactly_once_for_the_whole_process`

  失敗内容 `capture::tests::captures_trace_level_events`:

  ```
  
  thread 'capture::tests::captures_trace_level_events' (30004) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<tuple$<>,void (*)()>
               at .\src\capture.rs:102
    16: log_capture_kit::capture::tests::captures_trace_level_events
               at .\src\capture_tests.rs:147
    17: log_capture_kit::capture::tests::captures_trace_level_events::closure$0
               at .\src\capture_tests.rs:146
    18: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::captures_trace_level_events::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::missing_field_is_none`:

  ```
  
  thread 'event::tests::missing_field_is_none' (1328) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::missing_field_is_none
               at .\src\event_tests.rs:375
    17: log_capture_kit::event::tests::missing_field_is_none::closure$0
               at .\src\event_tests.rs:374
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::missing_field_is_none::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `capture::tests::ensure_interest_probes_is_idempotent_across_threads`:

  ```
  
  thread 'capture::tests::ensure_interest_probes_is_idempotent_across_threads' (25980) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::tests::ensure_interest_probes_is_idempotent_across_threads
               at .\src\capture_tests.rs:234
    15: log_capture_kit::capture::tests::ensure_interest_probes_is_idempotent_across_threads::closure$0
               at .\src\capture_tests.rs:231
    16: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::ensure_interest_probes_is_idempotent_across_threads::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    17: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::capture_lines_does_not_leak_the_sentinel`:

  ```
  
  thread 'event::tests::capture_lines_does_not_leak_the_sentinel' (35420) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,tuple$<>,log_capture_kit::event::tests::capture_lines_does_not_leak_the_sentinel::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<tuple$<>,log_capture_kit::event::tests::capture_lines_does_not_leak_the_sentinel::closure_env$0>
               at .\src\capture.rs:102
    16: log_capture_kit::event::capture_lines<tuple$<>,log_capture_kit::event::tests::capture_lines_does_not_leak_the_sentinel::closure_env$0>
               at .\src\event.rs:173
    17: log_capture_kit::event::tests::capture_lines_does_not_leak_the_sentinel
               at .\src\event_tests.rs:276
    18: log_capture_kit::event::tests::capture_lines_does_not_leak_the_sentinel::closure$0
               at .\src\event_tests.rs:275
    19: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::capture_lines_does_not_leak_the_sentinel::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    20: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::capture_lines_returns_formatted_lines_and_the_closure_result`:

  ```
  
  thread 'event::tests::capture_lines_returns_formatted_lines_and_the_closure_result' (17896) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,i32,log_capture_kit::event::tests::capture_lines_returns_formatted_lines_and_the_closure_result::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<i32,log_capture_kit::event::tests::capture_lines_returns_formatted_lines_and_the_closure_result::closure_env$0>
               at .\src\capture.rs:102
    16: log_capture_kit::event::capture_lines<i32,log_capture_kit::event::tests::capture_lines_returns_formatted_lines_and_the_closure_result::closure_env$0>
               at .\src\event.rs:173
    17: log_capture_kit::event::tests::capture_lines_returns_formatted_lines_and_the_closure_result
               at .\src\event_tests.rs:260
    18: log_capture_kit::event::tests::capture_lines_returns_formatted_lines_and_the_closure_result::closure$0
               at .\src\event_tests.rs:259
    19: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::capture_lines_returns_formatted_lines_and_the_closure_result::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    20: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::count_levels_counts_each_level`:

  ```
  
  thread 'event::tests::count_levels_counts_each_level' (13240) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,ref$<str$>,log_capture_kit::event::tests::count_levels_counts_each_level::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<ref$<str$>,log_capture_kit::event::tests::count_levels_counts_each_level::closure_env$0>
               at .\src\capture.rs:102
    16: log_capture_kit::event::count_levels<ref$<str$>,log_capture_kit::event::tests::count_levels_counts_each_level::closure_env$0>
               at .\src\event.rs:201
    17: log_capture_kit::event::tests::count_levels_counts_each_level
               at .\src\event_tests.rs:291
    18: log_capture_kit::event::tests::count_levels_counts_each_level::closure$0
               at .\src\event_tests.rs:290
    19: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::count_levels_counts_each_level::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    20: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::level_is_rendered_with_display_not_debug`:

  ```
  
  thread 'event::tests::level_is_rendered_with_display_not_debug' (8928) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::level_is_rendered_with_display_not_debug
               at .\src\event_tests.rs:232
    17: log_capture_kit::event::tests::level_is_rendered_with_display_not_debug::closure$0
               at .\src\event_tests.rs:231
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::level_is_rendered_with_display_not_debug::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `capture::tests::declares_failure_when_the_sentinel_is_not_captured`:

  ```
  
  thread 'capture::tests::declares_failure_when_the_sentinel_is_not_captured' (34312) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::tests::SentinelDroppingSubscriber,tuple$<>,log_capture_kit::capture::tests::declares_failure_when_the_sentinel_is_not_captured::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::tests::declares_failure_when_the_sentinel_is_not_captured
               at .\src\capture_tests.rs:137
    16: log_capture_kit::capture::tests::declares_failure_when_the_sentinel_is_not_captured::closure$0
               at .\src\capture_tests.rs:134
    17: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::declares_failure_when_the_sentinel_is_not_captured::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    18: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  note: panic did not contain expected string
        panic message: "AREKA_LOG_CAPTURE_PROBES の値が不正: \"typo\"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること"
   expected substring: "対照イベント"
  ```

  失敗内容 `event::tests::field_str_returns_the_raw_value_and_field_returns_the_debug_representation`:

  ```
  
  thread 'event::tests::field_str_returns_the_raw_value_and_field_returns_the_debug_representation' (35768) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::field_str_returns_the_raw_value_and_field_returns_the_debug_representation
               at .\src\event_tests.rs:329
    17: log_capture_kit::event::tests::field_str_returns_the_raw_value_and_field_returns_the_debug_representation::closure$0
               at .\src\event_tests.rs:328
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::field_str_returns_the_raw_value_and_field_returns_the_debug_representation::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live`:

  ```
  
  thread 'event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live' (7620) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,tuple$<>,log_capture_kit::event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<tuple$<>,log_capture_kit::event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live::closure_env$0>
               at .\src\capture.rs:102
    16: log_capture_kit::event::count_levels<tuple$<>,log_capture_kit::event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live::closure_env$0>
               at .\src\event.rs:201
    17: log_capture_kit::event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live
               at .\src\event_tests.rs:318
    18: log_capture_kit::event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live::closure$0
               at .\src\event_tests.rs:316
    19: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::count_levels_counts_zero_when_nothing_is_emitted_but_the_window_is_live::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    20: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `global::tests::installs_once_and_the_second_call_returns_the_same_buffer`:

  ```
  
  thread 'global::tests::installs_once_and_the_second_call_returns_the_same_buffer' (23636) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::global::install_into::closure$0<tracing_core::dispatcher::SetGlobalDefaultError,log_capture_kit::global::install_global_capture_all::closure_env$0>
               at .\src\global.rs:81
    15: std::sync::once_lock::impl$0::get_or_init::closure$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>,log_capture_kit::global::install_into::closure_env$0<t
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    16: std::sync::once_lock::impl$0::initialize::closure$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>,std::sync::once_lock::impl$0::get_or_init::closure_env$
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
    17: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
    18: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
    19: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>,std::sync::o
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
    20: std::sync::once_lock::OnceLock<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global> >::initialize<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::V
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    21: std::sync::once_lock::OnceLock<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global> >::get_or_try_init<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::v
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    22: std::sync::once_lock::OnceLock<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global> >::get_or_init<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    23: log_capture_kit::global::install_into<tracing_core::dispatcher::SetGlobalDefaultError,log_capture_kit::global::install_global_capture_all::closure_env$0>
               at .\src\global.rs:79
    24: log_capture_kit::global::install_global_capture_all
               at .\src\global.rs:65
    25: log_capture_kit::global::tests::installs_once_and_the_second_call_returns_the_same_buffer
               at .\src\global_tests.rs:36
    26: log_capture_kit::global::tests::installs_once_and_the_second_call_returns_the_same_buffer::closure$0
               at .\src\global_tests.rs:35
    27: core::ops::function::FnOnce::call_once<log_capture_kit::global::tests::installs_once_and_the_second_call_returns_the_same_buffer::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
  …（60 行で切った。全文は生ログ）
  ```

  失敗内容 `event::tests::field_names_sorted_is_ascending_and_includes_message`:

  ```
  
  thread 'event::tests::field_names_sorted_is_ascending_and_includes_message' (11484) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::field_names_sorted_is_ascending_and_includes_message
               at .\src\event_tests.rs:385
    17: log_capture_kit::event::tests::field_names_sorted_is_ascending_and_includes_message::closure$0
               at .\src\event_tests.rs:384
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::field_names_sorted_is_ascending_and_includes_message::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::message_is_the_body_and_is_empty_when_absent`:

  ```
  
  thread 'event::tests::message_is_the_body_and_is_empty_when_absent' (7612) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::message_is_the_body_and_is_empty_when_absent
               at .\src\event_tests.rs:358
    17: log_capture_kit::event::tests::message_is_the_body_and_is_empty_when_absent::closure$0
               at .\src\event_tests.rs:357
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::message_is_the_body_and_is_empty_when_absent::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `capture::tests::extracts_events_even_while_the_shared_sink_is_still_held`:

  ```
  
  thread 'capture::tests::extracts_events_even_while_the_shared_sink_is_still_held' (16944) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::tests::extracts_events_even_while_the_shared_sink_is_still_held
               at .\src\capture_tests.rs:217
    16: log_capture_kit::capture::tests::extracts_events_even_while_the_shared_sink_is_still_held::closure$0
               at .\src\capture_tests.rs:212
    17: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::extracts_events_even_while_the_shared_sink_is_still_held::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    18: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `probe::tests::establishing_the_probes_is_idempotent`:

  ```
  
  thread 'probe::tests::establishing_the_probes_is_idempotent' (27820) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::probe::tests::establishing_the_probes_is_idempotent
               at .\src\probe_tests.rs:56
    15: log_capture_kit::probe::tests::establishing_the_probes_is_idempotent::closure$0
               at .\src\probe_tests.rs:55
    16: core::ops::function::FnOnce::call_once<log_capture_kit::probe::tests::establishing_the_probes_is_idempotent::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    17: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::field_str_is_none_for_values_that_did_not_come_through_record_str`:

  ```
  
  thread 'event::tests::field_str_is_none_for_values_that_did_not_come_through_record_str' (3240) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::field_str_is_none_for_values_that_did_not_come_through_record_str
               at .\src\event_tests.rs:345
    17: log_capture_kit::event::tests::field_str_is_none_for_values_that_did_not_come_through_record_str::closure$0
               at .\src\event_tests.rs:344
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::field_str_is_none_for_values_that_did_not_come_through_record_str::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::level_fields_matches_verbatim_fixture`:

  ```
  
  thread 'event::tests::level_fields_matches_verbatim_fixture' (34492) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::level_fields_matches_verbatim_fixture
               at .\src\event_tests.rs:183
    17: log_capture_kit::event::tests::level_fields_matches_verbatim_fixture::closure$0
               at .\src\event_tests.rs:182
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::level_fields_matches_verbatim_fixture::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `capture::tests::sentinel_is_removed_before_returning`:

  ```
  
  thread 'capture::tests::sentinel_is_removed_before_returning' (26880) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,u32,log_capture_kit::capture::tests::sentinel_is_removed_before_returning::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<u32,log_capture_kit::capture::tests::sentinel_is_removed_before_returning::closure_env$0>
               at .\src\capture.rs:102
    16: log_capture_kit::capture::tests::sentinel_is_removed_before_returning
               at .\src\capture_tests.rs:193
    17: log_capture_kit::capture::tests::sentinel_is_removed_before_returning::closure$0
               at .\src\capture_tests.rs:192
    18: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::sentinel_is_removed_before_returning::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::level_target_fields_matches_verbatim_fixture`:

  ```
  
  thread 'event::tests::level_target_fields_matches_verbatim_fixture' (22388) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::level_target_fields_matches_verbatim_fixture
               at .\src\event_tests.rs:162
    17: log_capture_kit::event::tests::level_target_fields_matches_verbatim_fixture::closure$0
               at .\src\event_tests.rs:161
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::level_target_fields_matches_verbatim_fixture::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `probe::tests::the_decision_agrees_with_what_the_environment_actually_says`:

  ```
  
  thread 'probe::tests::the_decision_agrees_with_what_the_environment_actually_says' (21548) panicked at crates\log-capture-kit\src\probe_tests.rs:44:24:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::tests::the_decision_agrees_with_what_the_environment_actually_says
               at .\src\probe_tests.rs:44
     3: log_capture_kit::probe::tests::the_decision_agrees_with_what_the_environment_actually_says::closure$0
               at .\src\probe_tests.rs:28
     4: core::ops::function::FnOnce::call_once<log_capture_kit::probe::tests::the_decision_agrees_with_what_the_environment_actually_says::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
     5: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::format_line_is_byte_identical_to_current_formatting_code`:

  ```
  
  thread 'event::tests::format_line_is_byte_identical_to_current_formatting_code' (18172) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::format_line_is_byte_identical_to_current_formatting_code
               at .\src\event_tests.rs:206
    17: log_capture_kit::event::tests::format_line_is_byte_identical_to_current_formatting_code::closure$0
               at .\src\event_tests.rs:205
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::format_line_is_byte_identical_to_current_formatting_code::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `global::tests::fails_explicitly_when_a_different_global_is_already_installed`:

  ```
  
  thread 'global::tests::fails_explicitly_when_a_different_global_is_already_installed' (6932) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::global::install_into::closure$0<log_capture_kit::global::tests::ForeignGlobalAlreadyInstalled,log_capture_kit::global::tests::fails_explicitly_when_a_different_global_is_already_installed::closure_env$0>
               at .\src\global.rs:81
    15: std::sync::once_lock::impl$0::get_or_init::closure$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>,log_capture_kit::global::install_into::closure_env$0<l
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    16: std::sync::once_lock::impl$0::initialize::closure$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>,std::sync::once_lock::impl$0::get_or_init::closure_env$
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
    17: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
    18: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
    19: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global>,std::sync::o
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
    20: std::sync::once_lock::OnceLock<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global> >::initialize<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::V
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    21: std::sync::once_lock::OnceLock<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global> >::get_or_try_init<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::v
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    22: std::sync::once_lock::OnceLock<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::Vec<log_capture_kit::event::CapturedEvent,alloc::alloc::Global> >,alloc::alloc::Global> >::get_or_init<alloc::sync::Arc<std::sync::poison::mutex::Mutex<alloc::vec::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    23: log_capture_kit::global::install_into<log_capture_kit::global::tests::ForeignGlobalAlreadyInstalled,log_capture_kit::global::tests::fails_explicitly_when_a_different_global_is_already_installed::closure_env$0>
               at .\src\global.rs:79
    24: log_capture_kit::global::tests::fails_explicitly_when_a_different_global_is_already_installed
               at .\src\global_tests.rs:74
    25: log_capture_kit::global::tests::fails_explicitly_when_a_different_global_is_already_installed::closure$0
               at .\src\global_tests.rs:71
    26: core::ops::function::FnOnce::call_once<log_capture_kit::global::tests::fails_explicitly_when_a_different_global_is_already_installed::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    27: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  …（60 行で切った。全文は生ログ）
  ```

  失敗内容 `capture::tests::captures_event_whose_callsite_another_thread_registered_before_the_window`:

  ```
  
  thread 'capture::tests::captures_event_whose_callsite_another_thread_registered_before_the_window' (6360) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<tuple$<>,void (*)()>
               at .\src\capture.rs:102
    16: log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registered_before_the_window
               at .\src\capture_tests.rs:62
    17: log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registered_before_the_window::closure$0
               at .\src\capture_tests.rs:57
    18: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registered_before_the_window::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `event::tests::fields_map_is_name_to_debug_representation`:

  ```
  
  thread 'event::tests::fields_map_is_name_to_debug_representation' (30608) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::event::tests::LegacyLineFormatting,tuple$<>,void (*)()>
               at .\src\capture.rs:64
    15: log_capture_kit::event::tests::run_fixture
               at .\src\event_tests.rs:151
    16: log_capture_kit::event::tests::fields_map_is_name_to_debug_representation
               at .\src\event_tests.rs:404
    17: log_capture_kit::event::tests::fields_map_is_name_to_debug_representation::closure$0
               at .\src\event_tests.rs:403
    18: core::ops::function::FnOnce::call_once<log_capture_kit::event::tests::fields_map_is_name_to_debug_representation::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `probe::tests::the_environment_is_read_exactly_once_for_the_whole_process`:

  ```
  
  thread 'probe::tests::the_environment_is_read_exactly_once_for_the_whole_process' (26676) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::probe::tests::the_environment_is_read_exactly_once_for_the_whole_process
               at .\src\probe_tests.rs:74
    15: log_capture_kit::probe::tests::the_environment_is_read_exactly_once_for_the_whole_process::closure$0
               at .\src\probe_tests.rs:73
    16: core::ops::function::FnOnce::call_once<log_capture_kit::probe::tests::the_environment_is_read_exactly_once_for_the_whole_process::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    17: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window`:

  ```
  
  thread 'capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window' (35552) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,tuple$<>,log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<tuple$<>,log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window::closure_env$0>
               at .\src\capture.rs:102
    16: log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window
               at .\src\capture_tests.rs:77
    17: log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window::closure$0
               at .\src\capture_tests.rs:76
    18: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::captures_event_whose_callsite_another_thread_registers_inside_the_window::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```

  失敗内容 `capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads`:

  ```
  
  thread 'capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads' (28700) panicked at crates\log-capture-kit\src\probe.rs:133:18:
  AREKA_LOG_CAPTURE_PROBES の値が不正: "typo"。`on`（常駐する＝未設定と同じ）か `off`（常駐しない＝測定の被験側）のいずれかを指定すること
  stack backtrace:
     0: std::panicking::panic_handler
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\panicking.rs:679
     1: core::panicking::panic_fmt
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\panicking.rs:80
     2: log_capture_kit::probe::probes_requested
               at .\src\probe.rs:133
     3: log_capture_kit::probe::probes::closure$0
               at .\src\probe.rs:162
     4: std::sync::once_lock::impl$0::get_or_init::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,log_capture_kit::probe::probes::closure_env$0>
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
     5: std::sync::once_lock::impl$0::initialize::closure$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_env$0<enum2$<core::option::Option<tuple$<tr
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:543
     6: std::sync::once::impl$2::call_once_force::closure$0<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_i
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     7: std::sys::sync::once::futex::Once::call
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\std\src\sys\sync\once\futex.rs:183
     8: std::sync::once::Once::call_once_force<std::sync::once_lock::impl$0::initialize::closure_env$0<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > >,std::sync::once_lock::impl$0::get_or_init::closure_
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once.rs:226
     9: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::initialize<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispa
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:542
    10: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_try_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:410
    11: std::sync::once_lock::OnceLock<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Dispatch> > > >::get_or_init<enum2$<core::option::Option<tuple$<tracing_core::dispatcher::Dispatch,tracing_core::dispatcher::Disp
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\std\src\sync\once_lock.rs:321
    12: log_capture_kit::probe::probes
               at .\src\probe.rs:161
    13: log_capture_kit::probe::ensure_interest_probes
               at .\src\probe.rs:186
    14: log_capture_kit::capture::run_with_subscriber<log_capture_kit::capture::CaptureSubscriber,tuple$<>,log_capture_kit::capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads::closure_env$0>
               at .\src\capture.rs:64
    15: log_capture_kit::capture::capture<tuple$<>,log_capture_kit::capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads::closure_env$0>
               at .\src\capture.rs:102
    16: log_capture_kit::capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads
               at .\src\capture_tests.rs:169
    17: log_capture_kit::capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads::closure$0
               at .\src\capture_tests.rs:166
    18: core::ops::function::FnOnce::call_once<log_capture_kit::capture::tests::does_not_capture_events_from_outside_the_window_or_other_threads::closure_env$0,tuple$<> >
               at C:\rust\up\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\src\rust\library\core\src\ops\function.rs:250
    19: core::ops::function::FnOnce::call_once
               at /rustc/88d9e12ae178fab0fb5cc050a94da85685d449ea/library\core\src\ops\function.rs:250
  note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
  ```


## ab2-on-1 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:52:57 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip bake_entry_tests:: --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip log_firing_tests:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip plan::ops_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::ratio_tests:: --skip scale::resample_tests:: --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4449 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/ab2-on-1-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=on` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し 1 巡目の採り直し: 除外集合を 1,447 個へ改めた後の A/B・区 1（常駐あり） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:50:25 | 31.6 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-1-r001.out.log` |
| 2 | 13:50:57 | 39.6 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-1-r002.out.log` |
| 3 | 13:51:37 | 39.7 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-1-r003.out.log` |
| 4 | 13:52:17 | 39.9 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-1-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.7 / 最小 31.6 / 最大 39.9）


## ab2-off-1 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:55:53 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip bake_entry_tests:: --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip log_firing_tests:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip plan::ops_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::ratio_tests:: --skip scale::resample_tests:: --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4449 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.5 秒・テスト実行体 74 本（刻印 logs/ab2-off-1-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=off` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し 1 巡目の採り直し: 区 2（常駐なし） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:53:15 | 36.9 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-1-r001.out.log` |
| 2 | 13:53:52 | 40.4 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-1-r002.out.log` |
| 3 | 13:54:32 | 40.8 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-1-r003.out.log` |
| 4 | 13:55:13 | 39.3 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-1-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.8 / 最小 36.9 / 最大 40.8）


## ab2-on-2 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 13:58:40 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip bake_entry_tests:: --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip log_firing_tests:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip plan::ops_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::ratio_tests:: --skip scale::resample_tests:: --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4449 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.5 秒・テスト実行体 74 本（刻印 logs/ab2-on-2-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=on` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し 1 巡目の採り直し: 区 3（常駐あり） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:56:02 | 39.7 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-2-r001.out.log` |
| 2 | 13:56:42 | 39.4 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-2-r002.out.log` |
| 3 | 13:57:21 | 39.1 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-2-r003.out.log` |
| 4 | 13:58:00 | 39.6 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-2-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.5 / 最小 39.1 / 最大 39.7）


## ab2-off-2 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 14:01:27 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip bake_entry_tests:: --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip log_firing_tests:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip plan::ops_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::ratio_tests:: --skip scale::resample_tests:: --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4449 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/ab2-off-2-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=off` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し 1 巡目の採り直し: 区 4（常駐なし） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 13:58:49 | 38.9 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-2-r001.out.log` |
| 2 | 13:59:28 | 39.4 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-2-r002.out.log` |
| 3 | 14:00:08 | 39.9 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-2-r003.out.log` |
| 4 | 14:00:48 | 38.6 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-2-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39.2 / 最小 38.6 / 最大 39.9）


## ab2-on-3 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 14:04:09 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip bake_entry_tests:: --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip log_firing_tests:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip plan::ops_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::ratio_tests:: --skip scale::resample_tests:: --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4449 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.5 秒・テスト実行体 74 本（刻印 logs/ab2-on-3-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=on` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し 1 巡目の採り直し: 区 5（常駐あり） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 14:01:36 | 36.2 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-3-r001.out.log` |
| 2 | 14:02:13 | 39.8 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-3-r002.out.log` |
| 3 | 14:02:53 | 37.8 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-3-r003.out.log` |
| 4 | 14:03:31 | 38.6 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-on-3-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 38.2 / 最小 36.2 / 最大 39.8）


## ab2-off-3 — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 14:06:55 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast -- --skip actor::actor_criteria_cage:: --skip actor::bind_loop_tests:: --skip actor::dispatch_tests:: --skip actor::runtime_frame_tests:: --skip actor::tests::all_senders_dropped_terminates_normally --skip actor::tests::allowed_event_id_is_sent_and_wire_trace_logged --skip actor::tests::allowed_event_id_unaffected_by_resource_or_extension --skip actor::tests::allowed_resource_id_passes_guard_and_is_sent --skip actor::tests::apply_barrier_emits_barrier --skip actor::tests::apply_close_emits_stop --skip actor::tests::apply_is_deterministic_across_variants --skip actor::tests::apply_persist_put_deterministic_same_ref --skip actor::tests::apply_persist_put_projects_to_dotted_global_and_saves --skip actor::tests::apply_publish_shiori_absent_emits_actor_debug_log --skip actor::tests::apply_publish_shiori_none_records_absent_no_default --skip actor::tests::apply_publish_shiori_some_sets_flat_per_asker --skip actor::tests::apply_publish_static_flat_per_asker_and_dotted_global --skip actor::tests::apply_set_effective_emits_runtime_command_reserved --skip actor::tests::apply_set_free_emits_host_store_write --skip actor::tests::apply_set_not_settable_emits_no_write --skip actor::tests::apply_store_write_emits_actor_debug_log --skip actor::tests::boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink --skip actor::tests::choice_actions_map_to_talk_commands_and_preserve_order --skip actor::tests::choice_fixed_ids_pass_the_static_guard_and_are_sent --skip actor::tests::choice_origin_bare_on_is_accepted_and_sent --skip actor::tests::choice_origin_scheduler_forbidden_ids_are_sent_verbatim --skip actor::tests::choice_origin_without_on_prefix_is_not_sent_and_logs_error --skip actor::tests::classify_set_canonical_non_effective_is_not_settable --skip actor::tests::classify_set_effective_key_is_runtime_command --skip actor::tests::classify_set_free_dotted_key_is_store_write --skip actor::tests::classify_set_three_branches_all_reachable --skip actor::tests::classify_set_unparseable_key_is_store_write --skip actor::tests::close_message_terminates_and_join_succeeds --skip actor::tests::disallowed_id_still_rejected_as_internal_after_resource_extension --skip actor::tests::forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error --skip actor::tests::force_quit_emits_onclose_notify_then_unload_then_close_in_order --skip actor::tests::mouse_msg_maps_to_input_and_is_ignored_in_idle_phase --skip actor::tests::runtime_command_sink_trait_is_reserved --skip actor::tests::sakura_disconnected_start_talk_failure_continues_run --skip actor::tests::same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin --skip actor::tests::shiori_disconnected_send_failure_terminates_into_fault --skip actor::tests::shiori_reply_dropped_maps_to_ipc_and_logs --skip actor::tests::shiori_send_failure_maps_to_ipc_and_logs --skip actor::tests::talk_command_send_failure_does_not_abort_the_action_batch --skip actor::tests::talk_command_send_failure_logs_error_and_continues --skip bake_entry_tests:: --skip balloon::model_tests::load_scope_balloon_model_debug_logs_missing_override_and_continues --skip balloon::model_tests::load_scope_balloon_model_info_logs_scope_and_resolved_values --skip balloon::model_tests::load_scope_balloon_model_inherits_unspecified_keys_from_descript --skip balloon::model_tests::load_scope_balloon_model_merges_per_scope_on_emo2_fixture --skip balloon::model_tests::load_scope_balloon_model_warns_on_missing_descript --skip balloon::model_tests::load_scope_balloon_model_warns_on_non_notfound_override_error --skip balloon::series_tests:: --skip bare_capture_drops_what_hardened_capture_keeps --skip bindrandom_off_consumes_no_rng_full_path --skip bindrandom_on_fires_full_path --skip child_bare_capture_drops_the_event --skip child_hardened_capture_keeps_the_event --skip choice_test::timeout_tests::choice_timeout_value_replaces_talk_via_existing_start_path --skip dispatcher::choice_tests:: --skip draw:: --skip ecs::layout::systems:: --skip ecs::window::command::command_batch_tests:: --skip ecs::window::command::command_coalesce_tests:: --skip ecs::window::command::command_transition_tests:: --skip ecs::window::transition_diag:: --skip ecs::window::zorder_pair::measure_tests:: --skip ecs::window::zorder_pair::record_tests:: --skip ecs::window::zorder_pair_establish:: --skip ecs::window::zorder_pair_maintain:: --skip ecs::window::zorder_pair_sink:: --skip ecs::window_proc::dpi_helpers:: --skip ecs::window_proc::lifecycle:: --skip ecs::window_proc::window_pos:: --skip emo2_boot::adapter:: --skip emo2_boot::balloon_visibility::lifecycle_e2e_tests:: --skip emo2_boot::balloon_visibility::phase::tests:: --skip emo2_boot::balloon_visibility::timeout_config_tests:: --skip emo2_boot::frame::chain_finalize_tests:: --skip emo2_boot::frame::chain_realign_tests:: --skip emo2_boot::frame::diag_route_tests:: --skip emo2_boot::frame::dpi_reproject_none_tests:: --skip emo2_boot::frame::dpi_reproject_tests:: --skip emo2_boot::frame::dpi_sync_hold_tests:: --skip emo2_boot::frame::drain_text_tests:: --skip emo2_boot::frame::harness_tests:: --skip emo2_boot::frame::text_scale_tests:: --skip emo2_boot::frame::transition_atomicity_tests:: --skip emo2_boot::frame::transition_branch_tests:: --skip emo2_boot::frame::visibility_integration_tests:: --skip emo2_boot::frame::work_area_resnap_hold_tests:: --skip emo2_boot::frame::work_area_sync_tests:: --skip emo2_boot::move_cue::move_severity_log_tests:: --skip emo2_boot::spine::boot_smoke_tests:: --skip emo2_boot::spine::display_tests:: --skip emo2_boot::spine::seriko_loop_tests:: --skip emo2_boot::spine::talk_close_tests:: --skip emo2_boot::spine::text_scale_tests:: --skip emo2_boot::talk_lifecycle:: --skip from_view_keeps_image_size_exact_at_sub_unity_scale --skip from_view_reads_physical_size_so_image_space_stays_k_invariant --skip input_events::balloon::hover_flag_tests:: --skip input_events::balloon::leave_tests:: --skip input_events::balloon::pointer_handler_tests:: --skip input_events::choice_drain:: --skip kero_negative_tail_restores_base_full_path --skip layout::cursor_tests:: --skip log_capture:: --skip log_firing_tests:: --skip looper:: --skip monitor_snapshot_seam_tests:: --skip other_negative_surface_warns_once_and_spares_others_full_path --skip persist::format:: --skip persist::tests:: --skip placement::balloon_limit::gate_tests:: --skip placement::diag:: --skip placement::follow::balloon_limit_wiring_tests:: --skip placement::follow::drag_end_limit_tests:: --skip placement::follow::keyword_base_tests:: --skip placement::follow::transition_diag_tests:: --skip placement::follow::visibility_balloon_wiring_tests:: --skip placement::follow::visibility_char_wiring_tests:: --skip placement::follow::window_move_diag_tests:: --skip placement::follow::window_move_hold_watch_tests:: --skip placement::measure:: --skip placement::monitor_tests:: --skip placement::source:: --skip placement::spawn::assembly_tests:: --skip placement::spawn::cleanup_tests:: --skip placement::spawn::zorder_pair_wiring_tests:: --skip placement::transition_diag:: --skip placement::windowposition_tests:: --skip placement::windowposition_vocab_tests:: --skip plan::ops_tests:: --skip playing_anim_not_relotteried_across_boundary_full_path --skip presenter::perf_log_tests:: --skip presenter::refresh_and_log_tests:: --skip presenter::timing:: --skip presenter::transition_record:: --skip region:: --skip residual_immediately_cleared_on_refire_full_path --skip restore_seam_tests:: --skip runtime::tests::boot_happy_path_wires_all_components_and_kicks_off_boot_sequence --skip runtime::tests::boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring --skip runtime::tests::boot_then_shutdown_joins_everything_and_returns_ok --skip runtime::tests::exit_flush_reflects_barrierless_clone_puts_after_shutdown_sequence --skip runtime::tests::gate_absent_boot_record_marks_first_boot_and_injects_set_epilogue --skip runtime::tests::gate_boot_record_is_existence_not_value --skip runtime::tests::gate_present_boot_record_marks_returning_and_no_epilogue --skip runtime::tests::gate_vanish_count_absent_defaults_zero --skip runtime::tests::gate_vanish_count_non_numeric_degrades_zero --skip runtime::tests::gate_vanish_count_present_numeric_is_parsed --skip runtime::tests::inproc_wiring_boots_drives_and_shuts_down_through_real_test_dll --skip runtime::tests::into_parts_exposes_live_senders_and_all_handles_for_manual_teardown --skip runtime::tests::mount_variant_constructs_and_displays --skip runtime::tests::mount_variant_is_a_std_error --skip runtime::tests::shutdown_confirms_persist_flush_via_barrier_before_close --skip runtime::tests::sylphya_publisher_accessor_yields_live_publisher_that_accepts_persist_put --skip sakura_residual_tail_keeps_frame_full_path --skip scale::ratio_tests:: --skip scale::resample_tests:: --skip scale::tests:: --skip scale_refresh_logs_k_transition_and_reattach_physical_size --skip schedule::boot:: --skip schedule::log_firing_tests:: --skip schedule::steady::choice_tests:: --skip schedule::steady::choice_timeout_tests:: --skip schedule::tests:: --skip seam_tests:: --skip shiori_demo:: --skip single_actor_attaches_only_to_its_own_target --skip sink:: --skip spine_e2e_test::s7_second_boot_record_present:: --skip state::bind_pattern_tests:: --skip state::cue_apply_tests:: --skip surface_switch_clears_playback_and_frame_full_path --skip sylphya_wiring:: --skip table::tests::empty_table_api --skip table::tests::method_is_resolved_and_frames_sorted_by_pattern_index --skip table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged --skip table::tests::recorded_anims_satisfy_postconditions --skip table::tests::table_is_send --skip table::tests::zero_k_and_empty_frames_are_not_recorded_with_warn --skip two_actors_are_routed_to_their_own_targets_and_draw_independently --skip unchanged_tick_emits_nothing_full_path --skip unregistered_actor_accumulates_without_disturbing_registered_actor --skip wrap:: --skip writing::` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 4449 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/ab2-off-3-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | `AREKA_LOG_CAPTURE_PROBES=off` |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し 1 巡目の採り直し: 区 6（常駐なし） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 14:04:18 | 38.6 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-3-r001.out.log` |
| 2 | 14:04:57 | 39.3 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-3-r002.out.log` |
| 3 | 14:05:37 | 39.1 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-3-r003.out.log` |
| 4 | 14:06:16 | 38.8 | 0 | 4449 | 0 | 31 | 1450 | 95 | 緑 | `ab2-off-3-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 39 / 最小 38.6 / 最大 39.3）


## ab2-base — custom ×4（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 14:10:07 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace --no-fail-fast` |
| 回数 / 同時プロセス | 4 / 1 |
| 期待 passed | 5894 |
| 1 回の上限 | 900 秒（-TimeoutSec で明示指定） |
| 事前ビルド | 0.5 秒・テスト実行体 74 本（刻印 logs/ab2-base-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | （無し） |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し 1 巡目の採り直し: 参考（除外なし・環境変数なし） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 14:07:04 | 45 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab2-base-r001.out.log` |
| 2 | 14:07:49 | 47 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab2-base-r002.out.log` |
| 3 | 14:08:36 | 45 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab2-base-r003.out.log` |
| 4 | 14:09:21 | 45.8 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `ab2-base-r004.out.log` |

**4 回走らせて 緑 4・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 45.4 / 最小 45 / 最大 47）


## cal112-kit-expect — 共有 crate log-capture-kit の全テスト（試走用の小さい対象） ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 14:13:33 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test -p log-capture-kit` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 99 |
| 1 回の上限 | 120 秒（自動＝単独実測 2.4 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 0.2 秒・テスト実行体 6 本（刻印 logs/cal112-kit-expect-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | （無し） |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し: 対象表の期待件数を採り直した後の既存呼出の健全性（-Target kit が件数不一致を出さないこと） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 14:13:30 | 3.1 | 0 | 99 | 0 | 2 | 0 | 7 | 緑 | `cal112-kit-expect-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 3.1 / 最小 3.1 / 最大 3.1）

## 11.2（採り直し）常時化の費用の A/B — 除外集合を是正して採り直した（2026-08-27・差し戻し 1 巡目）

> 上の 7 節（`ab2-on-1` ／ `ab2-off-1` … `ab2-base`）は反復の仕組みが自動で追記した生の記録で、
> 本節はその読み方と結論である（要件 13.3・13.4・13.5／設計 `#### C9`）。
> **本節が 11.2 の正本**で、先行する「11.2 常時化の費用の A/B」節（`ab-*` にもとづくもの）は
> 撤回済みである。要件 13.5 が求める申し送り台帳への登記はタスク 12.1 が行い、本節がその原文。

### 0. 何を差し戻され、何を採り直したか

**除外集合が共有捕捉窓を使うテストを 96 件取りこぼしていた。** 走査器
`verification/capture-window-tests.py` の**供給条件**が「`log_capture_kit` を参照し、かつ
`#[test]` を 1 本も持たないファイル」に限られていたのが原因である。ところがこの repo では
**包みファイルが自己テストを持つのが定石**で、

| 包み | 中身 | 自己テスト | 呼び手 |
|---|---|---:|---:|
| `crates/areka-emo-atlas/src/log_capture.rs:34` | `capture_logs` ＝ `log_capture_kit::capture_lines` の包み | 3 本 | 1 ファイル |
| `crates/areka-emo-compose/src/log_capture.rs:34` | 同上 | 3 本 | 4 ファイル |

の 2 件がどちらも供給者から外れ、その呼び手 5 ファイルが除外集合に **1 件も**入っていなかった。

| 取りこぼしていたファイル | 呼出の先頭 | `#[test]` |
|---|---|---:|
| `crates/areka-emo-atlas/src/lib.rs` | `:478`（他 `:523` `:548`） | 8 |
| `crates/areka-emo-compose/src/log_firing_tests.rs` | `:154` | 14 |
| `crates/areka-emo-compose/src/plan_ops_tests.rs` | `:963` | 31 |
| `crates/areka-emo-compose/src/scale_ratio_tests.rs` | `:139` | 27 |
| `crates/areka-emo-compose/src/scale_resample_tests.rs` | `:295` | 16 |
| **計** | | **96** |

**この 96 件は 24 回の走行すべてで両側を走り続けていた。** 撤回した節の §4 は「24 回全緑が
除外集合の十分性の証拠にあたる」と書いていたが、その全緑のさなかにこの取りこぼしがあった。

これは撤回した節の §4 が記録した**「別名の穴」と同じ家族の 2 件目**である。別名の穴が 1 ファイル
分の取りこぼしだったのに対し、こちらは**規則そのものが構造的に外していた**——同じ形は今後も
再発する。したがって登記で済ませず、走査器・除外集合・A/B のすべてを採り直した。

### 1. 走査器の是正（供給条件を関数単位にした）

ファイル単位の条件を落とし、**「テスト関数の外で定義され、本体が窓を開く語を呼ぶ非テスト関数」**
で判定する形へ改めた（`capture-window-tests.py` の `donor_fn_spans` と `fixpoint_tokens`）。
包みが自己テストを持っていても供給者になれる。テスト関数の**内側**で定義された入れ子の関数は
供給者にしない（その名前はそのテストのローカルな道具であって、他ファイルから呼ばれる窓口では
ないため）。

**較正へ既知の答えとして足した**（`calibrate_self_tested_wrappers`・別名の穴と同じ扱い）。
**陽性を要求する**形で書いてある——「拾われないこと」ではなく、

- 上表の包み 2 件が実際に `capture_logs(` を自分の crate へ**供給していること**
- その呼び手 5 件が**除外集合に当たっていること**
- そもそも包みが自己テストを持ち続けていること（この較正が守っている形が消えたら赤）

を要求し、1 件でも欠けたら非 0 で止まる。**赤を作れることを実測した**——供給条件を旧版へ
戻した変異体を走らせると較正 5 が 4 つの問題を挙げて**終了コード 2**（証跡
`red/cal112-donor-red-r001.err.log`）。是正後は**終了コード 0**（`red/cal112-donor-green-r001.out.log`）。

### 2. 採り直した除外集合

| 実測（2026-08-27・HEAD `87a640de`） | 是正後 | 撤回した回 |
|---|---:|---:|
| 走査したソース | 1,035 ファイル | 1,035 |
| 包みの語を提供しうるファイル | **39 件** | 14 |
| 走査語（不動点後・適用範囲つき） | **28 語** | 18 |
| 当たったファイル | **130 件**（helper-only 12 / per-test 84 / whole-file 34） | 125 |
| 除外の粒度 | ファイル単位（当たったファイルの全テスト） | 同じ |
| `--skip` に渡した値 | **188 個**（`exclusion/exclusion-skip-args.txt`） | 183 |
| 除外したテスト（完全な名前・重複除去） | **1,447 個**（`exclusion/exclusion-tests.txt`） | 1,351 |

**増えたのはちょうど上表の 5 ファイル・96 件だけ**である（当たりファイルの差分を取ると
追加 5・削除 0。1,447 − 1,351 ＝ 96 ＝ 8＋14＋31＋27＋16）。他の crate へ波及していない。

走らせた命令は 7 区すべてで

    cargo test --workspace --no-fail-fast -- --skip <188 個の値>

除外は**両側から同じフィルタ**で行った。`--no-fail-fast` は必須である（既定の fail-fast だと
常駐なし側は最初の 1 本で打ち切られ、所要時間が比較値にならない）。

粒度をファイル単位にした理由は撤回した節と同じ——テスト単位まで絞ると `--skip` の値が
Windows のコマンドラインの上限 32,767 文字に迫る。過剰除外の側に倒しても、常駐の代償は
ワークスペース全体に乗るので残りにも差は出るはず（設計 C9）。

### 3. 除外が本当に効いたことの較正（予言と実走の突合）

**終了コード 0 は空振りでも返る。** `--skip` の名前を綴り誤ると、除外したつもりのテストが
走り続けたまま緑で通る。そこで**当たる件数を先に予言して突合**した。

- 走査側の予言: `--list` の全 **5,930 行**に 188 個の値を部分一致させると **1,450 行**が当たる。
- 実走: **24 回すべてが `filtered out` 1,450**・`passed` 4,449・`failed` 0・`ignored` 31・
  実行体 95 本も全回一致。
- 除外なしの基準線 `ab2-base` は `passed` 5,894・`ignored` 36・`filtered out` 0。

算が 3 通りとも閉じる。

    4,449 + 31 + 1,450 = 5,930
    5,894 +      36    = 5,930
    (5,894 - 4,449) + (36 - 31) = 1,445 + 5 = 1,450

走査器の報告が言う「残るテスト名 4,478 個」と実走の残り 4,480 行の差 2 は**同名テストの重複**で
説明が付く（`--list` は 5,930 行 / 重複除去 5,925 個＝重複 5。除外側は 1,450 行 / 1,447 個＝
重複 3。残りの重複は 5 − 3 ＝ 2）。**件数がどこにも吸い込まれていない。**

**同一のテスト実行体を測ったことの確認**（要件 13.1）。反復の仕組みは各区で
`cargo test … --no-run --message-format=json` が解決したテスト実行体のパス・サイズ・更新時刻を
`logs/<札>-binaries.txt` へ刻む。7 区すべての刻印の**本文**（見出しの 3 行を除いた 74 行）が
**バイト単位で同一**（md5 先頭 16 桁が 7 区とも `94a622d11af3c58a`）。**撤回した回の `ab-*` 7 区と
まで同一**なので、本タスクの是正が `verification/` の外へ 1 バイトも及んでいないことも同時に
示している。事前ビルドはいずれも 0.5〜0.6 秒＝やることが無かった。
（刻印ファイル全体の md5 は区ごとに違う。見出しに札と採取時刻が入るためで、比較するのは本文。）

**環境変数が測定対象のプロセスまで届いたことの確認。** 綴りを誤った値で走らせると panic する
性質を使った較正のみが、これを示せる。`AREKA_LOG_CAPTURE_PROBES=typo` で `-Target kit` を
1 回走らせると **27 件が赤**になり（`passed` 3）、本文は逐語で
「`AREKA_LOG_CAPTURE_PROBES` の値が不正: "typo"」だった。
**証跡はディスクに残してある**——`red/cal112-typo-r001.out.log`（当該変数を含む行 29 本）。

**両側で走る檻 `probe::tests::the_decision_agrees_with_what_the_environment_actually_says` は
届いたことの証拠にならない。** 撤回した節はこれを 2 つ目の確認として挙げていたが、実装は
`crates/log-capture-kit/src/probe_tests.rs:29` の `std::env::var(PROBES_ENV).ok()` で、
届いていなければ `None` 分岐＝「未設定なら既定（常駐する）」に落ちて**緑になる**。
この檻が縛るのは「値が届いたとき、その値と判定が食い違わない」ことだけである
（それでも両側で走る対照としては有効で、24 回すべてで実際に走って緑だった。
共有 crate の `src/` にあり除外集合に入っていない＝`exclusion/exclusion-tests.txt` に不在）。

### 4. 除外集合が閉じていることを何で担保するか（撤回の代わりに置くもの）

**撤回する: 「24 回全緑が除外集合の十分性の証拠にあたる」（撤回した節 §4 の末尾）。**
今回の反例で崩れた——**96 件が両側で走り続けたまま 24 回とも全緑だった**。
常駐なし側で赤が出るかどうかは**並列の巡り合わせ次第**で（タスク 11.1 の実測: 8 回の走行で
不変に赤なのは較正テスト 1 本だけ・他は 1/8〜5/8 で出入りする）、走らせて緑だったことからは
何も言えない。**全緑は十分性の証拠にならない。**

代わりに置くのは**静的な走査の側の担保**で、次の 4 点である。

1. **較正は陽性を要求する形で書く。** 「当たりが 0 件でないこと」「既知のファイルが**拾われる**
   こと」を要求し、1 件でも欠けたら非 0 で止める。「拾われないこと」を確かめる形は、走査が
   丸ごと空振りしていても緑になる。
2. **既知の答えは別の実装から取る。** `with_default` 系 4 件と `install_global_capture_all` 2 件は
   実在する見張り `crates/log-capture-kit/tests/with_default_guard_test.rs` の例外表を逐語で
   写したもので、表が動けば較正が赤になる（走査器と見張りが互いを検算する）。
3. **穴が見つかったら、その実例を既知の答えとして較正へ足す。** 別名の穴（4 ファイル）と
   本件（包み 2 件＋呼び手 5 ファイル）はどちらもそうしてある。**同じ穴は二度開かない**が、
   これは既知の形についてしか言えない。
4. **当たる件数を先に予言して `filtered out` と突き合わせる**（§3）。走査と実走が別経路で
   同じ数に到達することを毎回確かめる。

**それでも「走査に別の形の穴が残っていない」ことは言えない。** 本件は 2 件目で、どちらも
「較正が全部緑のまま取りこぼす」形だった。3 件目があるとすれば、やはり較正は緑のままだろう。

### 5. 数字

いずれも 1 回の走行の所要秒（`repeat-tests.ps1` が測る壁時計。事前ビルドは含まない）。
機械の状態が時間とともに動くので **4 回ずつの区を交互に 6 区**（on → off → on → off → on → off）
並べ、各側 12 回を採った（撤回した回と同じ並べ方）。

**定義**（次に採り直す者が同じ数字を再現できるように明記する）。

- **標準偏差は母標準偏差**（偏差平方和を n で割る・`statistics.pstdev`）。標本標準偏差
  （n−1 で割る・`statistics.stdev`）も併記する。撤回した節は母標準偏差を定義なしで載せていた。
- **四分位は包含法**（`statistics.quantiles(sorted(v), n=4, method='inclusive')`＝両端を含む線形補間）。
- **中央値**は `statistics.median`（偶数個なら中央 2 値の平均）。

| 側 | n | 中央値 | 平均 | 最小 | 最大 | 四分位（下・上） | 母標準偏差 | 標本標準偏差 |
|---|---:|---:|---:|---:|---:|---|---:|---:|
| A 常駐あり（`on`） | 12 | **39.50** | 38.42 | 31.6 | 39.9 | 38.40 / 39.70 | 2.30 | 2.40 |
| B 常駐なし（`off`） | 12 | **39.20** | 39.17 | 36.9 | 40.8 | 38.75 / 39.52 | 0.95 | 0.99 |
| 参考: 除外なし・環境変数なし | 4 | 45.40 | 45.70 | 45.0 | 47.0 | 45.00 / 46.10 | 0.82 | 0.95 |

生の 24 回（区の順）:

    A on : 31.6 39.6 39.7 39.9 | 39.7 39.4 39.1 39.6 | 36.2 39.8 37.8 38.6
    B off: 36.9 40.4 40.8 39.3 | 38.9 39.4 39.9 38.6 | 38.6 39.3 39.1 38.8
    base : 45.0 47.0 45.0 45.8

- **中央値の差（B − A）＝ −0.30 秒（−0.76%）。符号は「常駐ありのほうが遅い」向き**である。
- **撤回した回は同じ手順で +0.25 秒（常駐なしのほうが遅い）だった。集合が違うので所要秒
  そのものは比べられないが、符号は採り直しで反転した。** これが本節でいちばん強い実測である
  ——同じ道具・同じ機械・同じ日に採った 2 度の A/B で、差の向きが入れ替わる。
- 各区の先頭回は 3 区中 2 区でその区の最速（事前ビルド直後の暖まり）。先頭回を落として
  9 回ずつで採り直すと **A 39.60 / B 39.30・差 −0.30 秒**で、向きも大きさも変わらない。
- 位置を対応させた 12 組の差（B − A）は
  `5.3 0.8 1.1 -0.6 -0.8 0.0 0.8 -1.0 2.4 -0.5 1.3 0.2`。
  内訳は**正 7 組・負 4 組・0 が 1 組**（正 ＝ 常駐なしのほうが遅い側）。
  **その中央値は +0.50 秒で、中央値どうしの差 −0.30 秒と符号が逆になる。**
  同じ 24 個の数字を 2 通りにまとめると向きが変わる、というのがこの集団の性質である。
  （撤回した節は同じ欄を「12 組中 4 組は符号が逆」と書いていたが、当時の実測は 5 組だった。
  本節は正・負・0 の 3 つを全部数えて載せる。）

### 6. 結論（要件 13.3）

**差は散らばりに埋没している。そのままを結論とする。**

A 側単独の実測値の幅は **8.3 秒**（31.6〜39.9）で、これは中央値の差 0.30 秒の **28 倍**である。
B 側単独でも幅は 3.9 秒＝差の 13 倍。四分位の区間（A 38.40〜39.70 / B 38.75〜39.52）は
B が A にほぼ含まれる形で重なる。そのうえ

- 中央値どうしの差は −0.30（A が遅い）、位置対応の差の中央値は +0.50（B が遅い）と**内部で
  符号が食い違い**、
- **撤回した回の同じ手順は +0.25（B が遅い）で、符号が反転している。**

**したがって「常駐の仕掛けが `cargo test` の所要時間に与える影響は、この測り方の分解能では
検出できない」が結論である。** 「速くなった」でも「遅くなった」でもなく、**上限として
「あったとしても片側 12 回の中央値で 0.5 秒（約 1.3%）より小さい」**としか言えない
（撤回した回の +0.25 と本回の −0.30 の両方を包む幅を上限に採った）。

**結論の向きは撤回した回から変わっていない。** 集合が 96 件増えても、差は散らばりに埋没する
という読みは同じである——ただし**根拠は強くなった**。前回は「符号が仮説と逆だから費用とは
読めない」だったが、今回は**符号そのものが再測で反転する**ことを実測したので、
「符号に意味が無い」を仮説ではなく観測として言える。

タスク 8.2 の +2.1 秒 / +5.3%（移行前ツリーとの比較）は、集合が違い散らばりが差の 20 倍だった
ので分離不能に終わったが、**今回は集合を揃えても差そのものが出なかった**——8.2 の +2.1 秒は
テストが 111 件・実行体が 6 本多いことの側に帰属する、と読むのが実測に合う。

### 7. この数字が測っていないもの（要件 13.5）

「測った、問題なかった」では済まない。**測っていない範囲を具体に書く。**

1. **除外した 1,447 個（1,450 行）のテストの所要は測っていない。** 除外なしの基準線が中央値
   45.40 秒、除外後が 39.50／39.20 秒なので、除外した分はおよそ **5.9〜6.2 秒＝全体の 13%**に
   あたる。そこに常駐の代償が乗っているかどうかは、本測定は**何も言っていない**（その集合は
   常駐なし側で赤になるので、そもそも所要時間を比べられない）。**そして今回 96 件増えたのは
   まさにこの測れない側**である。
2. **本番プロセスは 1 秒も測っていない。** 測ったのはテスト実行体だけである。常駐 probe は
   dev-dependency 経由でテストビルドにしか入らない設計なので本番には乗らない、という
   **構造の主張**は別に立っているが（`with_default_guard_test.rs` の製品側依存の見張り）、
   本測定はその主張の裏を取っていない。
3. **1 台の機械の 1 日の状態しか測っていない。** 2026-08-27 の 13〜14 時台・同一ワークツリー・
   同一 HEAD（`87a640de`）・cargo 1.98.0・Defender の再スキャンなど他プロセスの負荷は制御して
   いない。4 回ずつの交互で線形な変動は打ち消しているが、周期的な負荷や別の機械での再現は
   見ていない。**同じ日の 2 度の A/B で符号が反転したのは、この未制御の側の大きさの現れである。**
4. **同時に複数プロセスで走らせたときの差は測っていない。** `-Parallel 1`（負荷なし）だけである。
   常駐の代償が効くとすれば interest の判定回数が増える経路なので、機械が飽和した状態での
   振る舞いは別の測定になる。
5. **`--test-threads` を変えたときの差も測っていない。** 既定の並列度だけである。
6. **どこに時間が行っているかは測っていない。** 測ったのは `cargo test` 1 回の壁時計の合計で、
   `Interest::sometimes` 常態化によって増えるはずの判定 1 回あたりの費用は見ていない。
   「合計に出ない」ことは「1 回あたりが 0」を意味しない。
7. **除外集合が十分だったことは、静的な走査が閉じている範囲までしか言えない**（§4）。
   **全緑は証拠にならない**——本件の 96 件は 24 回全緑のさなかに走り続けていた。
   走査に 3 件目の穴が残っている可能性を、24 回の全緑も較正の全緑も排除しない。

### 8. 反復の仕組みの対象表を採り直した（申し送りではなく是正した）

`repeat-tests.ps1` の `$Targets` の期待件数を 5 件すべて実走で数え直した。

| 対象 | 表の値（旧） | 実測 | 扱い |
|---|---:|---:|---|
| `workspace` | 5865 | **5894** | 是正 |
| `kit` | 79 | **99** | 是正 |
| `seriko` | 200 | 200 | 据え置き |
| `wintf` | 842 | 842 | 据え置き |
| `wait` | 2 | 2 | 据え置き |

**申し送りにせず直した根拠**は手順書自身の流儀である——`repeat-tests.md` §2 と `$Targets` の
見出しコメントが「**期待件数は不変量ではない。テストが増減したら採り直して表を更新する**」と
述べており、陳腐化は欠陥ではなく更新の対象として設計されている。放置すると `-Target kit` の
ような**既存の呼び方が「件数不一致」＋終了コード 1 を返す**状態が常態化し、**本物の赤を隠す**。
是正後は既存の呼び方が両方とも緑——`-Target kit` は `cal112-kit-expect` 節（`passed` 99）、
`-Target workspace` は `cal112-ws-expect` 節（`passed` 5894）。
`Solo`（上限秒の自動算出にだけ使う単独実測）は触っていない——上限は性能の合否ではなく
ハングの止め木で、現行値でも実測の 8 倍以上の余裕がある。

### 9. 再現の手順

    cargo test --workspace -- --list > list.txt
    python .kiro/specs/areka-P0-test-cage-determinism/verification/capture-window-tests.py \
        --root . --list list.txt --out-dir <出力先> --calibrate
    # 出力の exclusion-skip-args.txt の各行の前に --skip を挟んで -TestArgs へ渡す
    verification/repeat-tests.ps1 -Target custom \
        -CargoArgs test,--workspace,--no-fail-fast -TestArgs <上記> \
        -Times 4 -ExpectPassed 4449 -Tag <札> -TimeoutSec 900 \
        -EnvVars AREKA_LOG_CAPTURE_PROBES=on   # もう一方は =off

`-EnvVars` は本タスクで反復の仕組みへ足した引数である（既定は何も足さない＝既存の呼出は不変）。
読み方は `repeat-tests.md` §10。**走る前に `--calibrate` の緑と、不正値較正の赤を 1 回ずつ
作ること**（前者は走査の穴、後者は「立てたつもりで立っていない」を塞ぐ。どちらも
終了コード 0 では区別が付かない）。

## cal112-ws-expect — ワークスペース全体 ×1（同時 1 プロセス）

| 項目 | 値 |
|---|---|
| 実行日時 | 2026-08-27 14:21:23 |
| 走行ルート | `C:\home\maz\git\areka\.claude\worktrees\ghost-window-zorder-0055fb` |
| HEAD | `87a640de`（作業ツリー dirty（11 件）） |
| 実行コマンド | `cargo test --workspace` |
| 回数 / 同時プロセス | 1 / 1 |
| 期待 passed | 5894 |
| 1 回の上限 | 368 秒（自動＝単独実測 36.8 秒 × 同時 1 × 10（下限 120 秒）） |
| 事前ビルド | 0.6 秒・テスト実行体 74 本（刻印 logs/cal112-ws-expect-binaries.txt） |
| i686 成果物の検査 | 実施 |
| 渡した環境変数 | （無し） |
| cargo | cargo 1.98.0 (797e8a9bc 2026-08-05) |
| 備考 | 11.2 差し戻し: 対象表の期待件数を採り直した後の既存呼出の健全性（-Target workspace） |

| 回 | 開始 | 所要秒 | 終了 | passed | failed | ignored | filtered | 実行体 | 判定 | ログ |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | 14:20:47 | 35.8 | 0 | 5894 | 0 | 36 | 0 | 95 | 緑 | `cal112-ws-expect-r001.out.log` |

**1 回走らせて 緑 1・赤 0・空振り 0・件数不一致 0・ビルド失敗 0・打ち切り 0**（所要秒 中央値 35.8 / 最小 35.8 / 最大 35.8）

