# 確認の記録（areka-P0-frame-phases-after-exit）

## 変異の確認（タスク 2.2・要件 3.3）

- 日付: 2026-09-28
- 対象のコミット: `91a7a6d8`（`91a7a6d86e19370764ffbc9a759c0856583dc995`・タスク 2.1 のコミット）
- 回したもの: `cargo test -p areka -j 4 exit_gate`（`frame_exit_gate_tests.rs` の 2 本だけ）
- 変異の前の状態: 2 本とも緑（2 passed・0 failed）
- 手順: `frame.rs` を作業用の控えに複写してから書き換え、確認の後に控えから戻した。git の取り消しの操作は使っていない。

### 変異 1: 判定を外す

- 変えたこと: `emo2_frame_system` の最初の文である判定（終了の受け口が指示済みなら読み飛ばしの記録を 1 行残して戻る `if` の塊）と、その直前の注釈 2 行を消した。
- 結果: 指示済みのテスト `frame_phases_are_skipped_once_exit_is_requested` が **赤**。未指示のテストは緑のまま（1 passed・1 failed）。
- 最初に落ちた確認: ERROR の件数（`frame_exit_gate_tests.rs` の ERROR 0 件の `assert_eq!`）

  ```
  assertion `left == right` failed: 指示済みの巡では指令を適用しない＝ERROR 0 件
    left: 2
   right: 0
  ```

- そのとき捕捉した行（3 行）:
  - `level=WARN` `work_area_sync`: モニタ表が空（列挙異常）→ 作業領域源を差し替えず現状維持
  - `level=ERROR` `areka_emo_present::presenter::hub`: `apply(Hide)`: 未装着ターゲット target_id=TargetId(0)
  - `level=ERROR` `balloon_visibility::phase`: 表示ライフサイクル信号の受信端が切断された
- 読み: 判定が無いと指令が取り出されて適用され（`apply(Hide)` の ERROR）、作業領域の同期も走る（WARN）。読み飛ばしの記録も出ない。ERROR の確認で先に止まるので、WARN 1 件・記録 0 件・受信端 0 件の確認までは進まないが、捕捉した行から WARN 1 件と記録 0 件も読み取れる。

### 変異 2: 判定を作業領域の同期の後ろへ動かす

- 変えたこと: 判定の塊（注釈 2 行を含む）を、結線を取り出す文と作業領域の同期（`work_area_sync::sync_monitor_snapshot(world)`）の後ろへ動かし、`return;` の前に `world.insert_non_send(wiring);` を足して結線を World へ戻すようにした。
- 結果: 指示済みのテスト `frame_phases_are_skipped_once_exit_is_requested` が **赤**。未指示のテストは緑のまま（1 passed・1 failed）。
- 落ちた確認: WARN の件数（`frame_exit_gate_tests.rs` の WARN 0 件の `assert_eq!`）。ERROR 0 件の確認は通った。

  ```
  assertion `left == right` failed: 指示済みの巡では作業領域の同期も走らない＝WARN 0 件
    left: 1
   right: 0
  ```

- そのとき捕捉した行（2 行）:
  - `level=WARN` `work_area_sync`: モニタ表が空（列挙異常）→ 作業領域源を差し替えず現状維持
  - `level=DEBUG` `emo2_boot::frame`: 終了が指示済み——毎フレームの処理を読み飛ばす `event="frame_phases_skipped_after_exit"`
- 読み: 赤の理由は「モニタ表が空」の WARN **1 件だけ**である。ERROR は 0 件で、読み飛ばしの記録は 1 行出ている。判定の位置が作業領域の同期より後ろへずれる退行を、WARN 0 件の確認が捕まえることを確かめた。

### 戻した後

- `frame.rs` を控えから戻し、控えと `cmp` で同一、かつ `git show HEAD:crates/areka/src/emo2_boot/frame.rs` と sha256 が一致（`f34e73a5…e59cd9370`）することを確かめた。
- `frame.rs` の更新時刻を `touch` で新しくしてから回し直した（`Compiling areka` が出て作り直されたことを確認）。
- 結果: 2 本とも **緑**（2 passed・0 failed・1974 filtered out）。
- `crates/**` の差分は 0 件（`git diff --stat -- crates` が空）。

## 未指示のテストで捕捉した行の読み（design の Testing Strategy・合否には使わない）

`frame_phases_run_as_today_when_exit_is_not_requested_or_absent` に、捕捉した行を 1 行ずつ印字する作業用の 1 行を一時的に足し、`-- --nocapture` で回して全行を読んだ。読んだ後に作業用の 1 行は控えから戻して取り除き、`HEAD` と sha256 が一致することを確かめた。

### 未指示（受け口あり・指示なし）: 4 行

| 水準 | 出所 | 内容 | 想定の内か |
|---|---|---|---|
| WARN | `areka::emo2_boot::frame::work_area_sync` | モニタ表が空（列挙異常）→ 作業領域源を差し替えず現状維持 | 想定の内（素の World はモニタ 0 台） |
| ERROR | `areka_emo_present::presenter::hub` | `apply(Hide)`: 未装着ターゲット target_id=TargetId(0) | 想定の内（テストが確かめる適用の観測） |
| INFO | `areka::emo2_boot::balloon_visibility` | バルーン非表示までの待ち時間を確定 env="AREKA_BALLOON_TIMEOUT_MS" timeout_secs=30.0 source="default" | **想定の外** |
| ERROR | `areka::emo2_boot::balloon_visibility::phase` | 表示ライフサイクル信号の受信端が切断された（以後の会話の占有終端を観測できない＝タイムアウト計測は始まらず表示を保持する） | **想定の外** |

### 受け口なし: 3 行

| 水準 | 出所 | 内容 | 想定の内か |
|---|---|---|---|
| WARN | `areka::emo2_boot::frame::work_area_sync` | モニタ表が空（列挙異常） | 想定の内 |
| ERROR | `areka_emo_present::presenter::hub` | `apply(Hide)`: 未装着ターゲット target_id=TargetId(0) | 想定の内 |
| ERROR | `areka::emo2_boot::balloon_visibility::phase` | 表示ライフサイクル信号の受信端が切断された | **想定の外** |

### 想定の外の 2 種類について

- **表示ライフサイクル信号の受信端が切断された（ERROR）**: テストの補助 `headless_wiring_with`（`frame_test_support.rs`）は、表示ライフサイクル信号の受信端を `mpsc::channel::<TalkLifecycleSignal>().1` で作り、送り手をその場で捨てている。バルーン可視性の相が受信端を読むと切断として観測し、1 回だけ ERROR を出す（`balloon_visibility_phase.rs` の受信端の読み取り）。テストの組み立てに由来するもので、本番の結線では送り手が生きている。本件の判定の働きとは関係しない。
- **バルーン非表示までの待ち時間を確定（INFO）**: `balloon_visibility.rs` の待ち時間の解決は `OnceLock` で 1 回だけ行い、そのときに INFO を 1 行出す。同じテストの実行体の中で最初に全相を回した組み立て（今回は「未指示」）にだけ出る。並びによって出る組み立てが変わり得る。
- どちらもテスト 2 の確認（`apply(Hide)` を含む ERROR 1 件・読み飛ばしの記録 0 件・受信端が空・結線が残る）には入らない。design D6 のとおり ERROR の総数と WARN の件数を確かめないので、合否は変わらない。指示済みのテスト（判定あり）では相が 1 つも走らないので、これらの行は出ない（ERROR 0 件・WARN 0 件はそのテストが確かめている）。

## crate 単位のテストと行数の上限の検査（タスク 2.3）

- 日付: 2026-09-28・対象のコミット: `4194bfb8`
- `cargo test -p areka -j 4`: 終了コード 0。単体 1974 passed・0 failed・2 ignored（新しいテスト 2 本を含む）、結合 1 passed・0 failed、4 passed・0 failed
- `cargo test -p log-capture-kit`: 終了コード 0。失敗 0 件。単体 30 passed、`capture_calibration_test` 1 passed（2 ignored）、`file_length_guard_test`（1,000 行の上限の検査）6 passed、`sample_path_guard_test` 16 passed、`temp_path_guard_test` 16 passed、`with_default_guard_test` 24 passed、`workspace_scan_test` 20 passed、doc 4 passed
- 新しいファイル `frame_exit_gate_tests.rs` は 141 行（上限 1,000 行未満）

## 実機の確認（タスク 3.1）

（タスク 3.1 で記入）
