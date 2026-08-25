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
