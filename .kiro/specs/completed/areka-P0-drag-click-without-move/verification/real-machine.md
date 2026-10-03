# 実機の記録: areka-P0-drag-click-without-move（タスク 3.1・3.2）

## 1. 走行の条件

| 項目 | 値 |
|---|---|
| 配布物 | `target\alpha\areka-alpha-x64-20261003-7873fb3.zip`（`tools/package-alpha.ps1` で 2026-10-03 にコミット `7873fb3b` から組んだもの・修正 2.1 を含む・判定 1〜8 すべて合） |
| 根 | `<ワークツリー>\target\drag-click-signoff\root`（R1 の前に zip から新しく展開） |
| 記憶 | `<ワークツリー>\target\drag-click-signoff\profile`（R1 の前は空） |
| ゴースト | 既定ゴースト（emo2） |
| `RUST_LOG` | `info,areka=debug,wintf::ecs::drag=debug`（design C5） |
| 安全弁 | `AREKA_APP_SMOKE_EXIT_MS=900000`（15 分・どの走行でも発火していない） |
| 起動 | `target\drag-click-signoff\run.ps1 -Run R1｜R2｜R3`（`AREKA_*`・`WINTF_*` を消してから `AREKA_ROOT`・`AREKA_PROFILE_DIR` を設定） |
| 操作 | 開発者 |

`runs.txt`（逐語）:

```
zip=areka-alpha-x64-20261003-7873fb3.zip
R1 pid=33612 start=2026-10-03T12:30:16.5617038+09:00
R1 exit=0 end=2026-10-03T12:31:11.1423272+09:00
R2 pid=24820 start=2026-10-03T12:31:19.9186163+09:00
R2 exit=0 end=2026-10-03T12:31:33.1608783+09:00
R3 pid=17636 start=2026-10-03T12:31:39.2906976+09:00
R3 exit=0 end=2026-10-03T12:31:52.9965671+09:00
```

## 2. 操作と目視

- R1: 空の記憶からの起動（初回）。相方の絵の上を動かさずに左クリック 1 回 → メニューの「終了」。
- R2: 起動して窓を掴まずに「終了」。
- R3: 相方を 1 回ドラッグ → 「終了」。
- 開発者の申告（逐語）: 「「初回の台詞が相方をずらすのを待って」これ、起動直後に実施されるので、画面上で目視できない。それいがいはR3まで終わりました。」
  - 初回のずらしは起動の直後に済むので、目では見えない。R1 のクリックは、ずらしが終わった後の相方に対して行われた。R1 で捨てた記録の対象 `entity=27v0` は、R3 でドラッグして `scope=1` に保存された窓と同じ entity である。

## 3. 件数（記録から数えたもの）

| 数えた行 | R1 | R2 | R3 |
|---|---|---|---|
| `char DragEnd 保存`（`areka::persist::save`） | **0** | 0 | **1** |
| `[DragEndEvent] Dispatching` | **0** | 0 | 1 |
| `[DragStartEvent] Dispatching` | 0 | 0 | 1 |
| `[DragAccumulator] Ended without Started dropped` | **1** | 0 | 0 |
| `写像スキップ` | **0** | 0 | 0 |
| ERROR | 0 | 0 | 0 |
| WARN | 3 | 3 | 3 |

WARN の 3 件は 3 回の走行とも同じ種類で、α の受入記録で外したものと同じ（`purple/a/null.png` の全透明 2 件と、折返し基準 1 件）。本修正には関係ない。

## 4. 判定

### R1（要件 4.1・5.7）: 合格

- 保存 0 件・終了の知らせ 0 件・捨てた記録 1 件（1 件以上）・写像スキップ 0 件。
- 捨てた記録（`run-R1.log` 33345 行目・逐語）:
  - `2026-10-03T03:31:00.109455Z DEBUG actor{actor=emo-text}: wintf::ecs::drag::accumulator: [DragAccumulator] Ended without Started dropped entity=27v0 cancelled=false`
- 修正前の α の受入記録（A1r）では、同じ操作で開始 0・終了 1・保存 1・写像スキップ 1 だった（research.md §7.1 の表）。

### R2（要件 4.1）: 合格

- 相方（scope 1）の起動時の位置が既定と同じ（`run-R2.log`・逐語）:
  - `2026-10-03T03:31:20.160765Z  INFO areka::persist::restore: merge_scope restore scope=1 anchor=Bottom saved_win_x=None saved_win_y=None default_char_x=1340 default_char_y=904 char_x=1340 char_y=904 char_w=672 char_h=800 saved_off_x=None saved_off_y=None balloon_off_x=292 balloon_off_y=-150 balloon_x=1632 balloon_y=754`
- `saved_win_x=None saved_win_y=None` で、R1 は位置を 1 件も保存していない。本体（scope 0）も `saved_win_x=None` で既定の位置（`char_x=2012 char_y=330`）に立った。修正前の A1r では本体の位置が保存されていた件も、ここでは起きていない。

### R3（本当のドラッグ）: 合格

- 保存 1 件、開始 → 終了の順（`run-R3.log`・逐語）:
  - 2458 行目: `2026-10-03T03:31:43.238509Z  INFO actor{actor=emo-text}: wintf::ecs::drag::dispatch: [DragStartEvent] Dispatching entity=27v0 x=1670 y=1325`
  - 3710 行目: `2026-10-03T03:31:44.711885Z  INFO actor{actor=emo-text}: wintf::ecs::drag::dispatch: [DragEndEvent] Dispatching entity=27v0 x=1513 y=1356 cancelled=false`
  - 3712 行目: `2026-10-03T03:31:44.712393Z  INFO actor{actor=emo-text}: areka::persist::save: char DragEnd 保存 scope=1 char_x=1235 char_y=904 saved_x=1571 saved_y=904 char_w=Some(672) anchor=Bottom`
- R3 は R2 の後に走らせたので、R2 の判定には影響しない。

## 5. 全体テスト（タスク 3.2・要件 5.9）: 合格

`pwsh -NoProfile -File tools/test-all.ps1`（コミット `7873fb3b`・開始時の未コミットの変更 0 件）の一覧（逐語）:

```
OK    i686 ターゲット導入（0 秒・終了コード 0）
OK    i686 成果物ビルド（123 秒・終了コード 0）
OK    fmt --check（8 秒・終了コード 0）
OK    x64 ワークスペース全テスト（1290 秒・終了コード 0）
OK    i686 テスト（host-32 系）（64 秒・終了コード 0）

全段 緑
```

- `test result:` の行を足し合わせると、通過 9,343 本・失敗 0 本・無視 44 本（118 のテストの実行ファイル）。次のテストも、この 0 件の失敗の中に含めて通っている。
  - 1,000 行の検査（`crates/log-capture-kit/tests/file_length_guard_test.rs`）
  - ドラッグの状態の移り変わりのテスト
  - 右クリックメニューの引き金のテスト（`trigger_flow_tests`）
  - ゴーストへのマウスのイベントのテスト
- `git diff main -- doc/COMPAT_ARCHITECTURE.md` は空（0 行）。§8 は書き換えていない（要件 5.8）。
