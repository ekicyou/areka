# 実機の記録: areka-P0-drag-cancel-borrow-miss（タスク 4.2）

## 1. 走行の条件

| 項目 | 値 |
|---|---|
| 配布物 | `target\package\areka-0.0.1-x64.zip`（`tools/package.ps1` で 2026-10-05 にコミット `1bd66f0d` から組んだもの・未コミットの変更 0 件・本 spec の修正 2.1〜3 をすべて含む） |
| 根 | `<ワークツリー>\target\drag-cancel-signoff\root`（R1 の前に zip から新しく展開） |
| 記憶 | `<ワークツリー>\target\drag-cancel-signoff\profile`（R1 の前は空）。窓の位置は根の下のゴーストの記憶 `root\ghost\emo2\ghost\master\profile\areka\sylphya.toml` に入る（R1 の前は無く、R1 の `merge_scope restore` は `saved_win_x=None`）。R2・R3 は前の走行の記憶をそのまま使う |
| 一時フォルダ | `<ワークツリー>\target\drag-cancel-signoff\temp` |
| ゴースト | 既定ゴースト（emo2） |
| `RUST_LOG` | `info,areka=debug,wintf::ecs::drag=debug`（位置の保存・`[DragEndEvent] Dispatching`・ドラッグの扱いのデバッグまで開ける） |
| 安全弁 | `AREKA_APP_SMOKE_EXIT_MS=900000`（15 分・どの走行でも発火していない） |
| 起動 | `target\drag-cancel-signoff\run.ps1 -Run R1｜R2｜R3`（`AREKA_*`・`WINTF_*` を消してから `AREKA_ROOT`・`AREKA_PROFILE_DIR` を設定） |
| 数え方 | `target\drag-cancel-signoff\count.ps1 -Run Rn`（目印はソースの文言そのまま） |
| 操作 | 開発者（1 回の起動で 1 項目・開発者の「GO」の後に起動） |

`runs.txt`（逐語）:

```
zip=areka-0.0.1-x64.zip commit=1bd66f0d sha256=17E1026B7C1BF39E8721F29A5B363F6D6396BFE233C9A48130D2AE82565AB542
R1 pid=21352 start=2026-10-05T08:49:08.4530725+09:00
R1 exit=0 end=2026-10-05T08:49:56.3702221+09:00
R2 pid=31636 start=2026-10-05T08:51:21.3139027+09:00
R2 exit=0 end=2026-10-05T08:51:51.8561845+09:00
R3 pid=4120 start=2026-10-05T08:52:38.1115911+09:00
R3 exit=0 end=2026-10-05T08:53:13.0927617+09:00
```

## 2. 操作

- R1: 閾値を越えるドラッグを 3 回（相方 1 回・本体 2 回）→ メニューの「終了」。
- R2: 本体と相方を、動かさずに左クリック 1 回ずつ → 「終了」。
- R3: 閾値を越えてドラッグし、左ボタンを押したまま ESC を押してから離す、を 2 回（本体・相方）→ 「終了」。

## 3. 件数（記録から数えたもの）

| 数えた行 | R1 | R2 | R3 |
|---|---|---|---|
| `char DragEnd 保存`（`areka::persist::save`） | **3** | **0** | **2** |
| `balloon DragEnd 保存` | 0 | 0 | 0 |
| `[DragStartEvent] Dispatching` | 3 | 0 | 2 |
| `[DragEndEvent] Dispatching`（全） | 3 | 0 | 2 |
| うち `cancelled=false` | 3 | 0 | 0 |
| うち `cancelled=true` | 0 | 0 | **2** |
| `drag_reentry_handled`（全） | 4 | 4 | 0 |
| うち `action="ended"` | **0** | **0** | **0** |
| うち `action="cancelled"` | **0** | **0** | **0** |
| うち `action="none"`（debug） | 4 | 4 | 0 |
| `drag_reentry_pos_unreadable` | 0 | 0 | 0 |
| `[DragAccumulator] Ended without Started dropped` | 0 | 0 | 0 |
| ERROR | 0 | 0 | 0 |
| WARN | 3 | 3 | 3 |

WARN の 3 件は 3 回の走行とも同じ種類で、前の spec の実機の記録で外したものと同じ（`purple/a/null.png` の全透明 2 件と、折返し基準 1 件）。本修正には関係ない。

再入の扱いで終えた・取り消した記録（`action` が `"ended"`・`"cancelled"`）は、3 回の走行とも **0 件**。再入は狙って起こしにくいので、この件数は合否には使わない（design「実機（8.6）」）。

`action="none"` の 8 件の内訳は次のとおり。どれもドラッグがすでに休んだ後に届いたもので、何もしていない。

- `WM_CAPTURECHANGED` 5 件（R1 で 3 件・R2 で 2 件）。離しで捕捉の守りを落とすとき、OS が送り返すもの。
- `WM_ACTIVATE(WA_INACTIVE)` 3 件（R1 で 1 件・R2 で 2 件）。終了のときの非活性化。

## 4. 判定

要件 5 は「取り消しや離しが再入で届いたとき」の要件である。再入は狙って起こしにくいので、design「実機（8.6）」に従い、実機ではふつうの条件で対応する操作を確かめた。再入の形は入口を通るテスト（tasks.md の 1.1・1.2・3 の表）で確かめている。

### R1（要件 5.2）: 合格

閾値を越えたドラッグ 1 回につき、開始 → 終了 → 保存がちょうど 1 件ずつ出ている（`run-R1.log`・行頭の時刻を除いて逐語）。

- 1 回目（相方）:
  - 11041 行目: `INFO actor{actor=emo-text}: wintf::ecs::drag::dispatch: [DragStartEvent] Dispatching entity=27v0 x=1691 y=1293`
  - 11791 行目: `INFO actor{actor=emo-text}: wintf::ecs::drag::dispatch: [DragEndEvent] Dispatching entity=27v0 x=1425 y=1325 cancelled=false`
  - 11793 行目: `INFO actor{actor=emo-text}: areka::persist::save: char DragEnd 保存 scope=1 char_x=1138 char_y=904 saved_x=1474 saved_y=904 char_w=Some(672) anchor=Bottom`
- 2 回目（本体）: 12813 行目の開始 → 12839 行目の終了 → 12841 行目の保存（`scope=0 saved_x=2378`）
- 3 回目（本体）: 12912 行目の開始 → 13177 行目の終了 → 13179 行目の保存（`scope=0 saved_x=2222`）

### R2（要件 5.3）: 合格

- 動かさないクリック 2 回で、保存 0 件・開始 0 件・終了 0 件。
- 押しで準備に入ったことは記録にある（`[start_preparing] DragState -> Preparing (with capture)` が 2 件）。
- 開始の無い終了を捨てた記録も 0 件。前の spec の実機の R1 では同じ操作で 1 件出ていた。本修正の後は、準備中から休ませても終了の種を積まないためで、design「drag state」の事後条件のとおり。

### R3（要件 5.1）: 合格

閾値を越えた後の ESC で、取り消しの印つきの終了が 1 件だけ出て、今どおり押した位置で 1 件保存される。ESC の後の離しは、2 件目の終了を出していない（`run-R3.log`・行頭の時刻を除いて逐語）。

- 1 回目（本体）:
  - 975 行目: `INFO actor{actor=emo-text}: wintf::ecs::drag::dispatch: [DragStartEvent] Dispatching entity=25v0 x=2097 y=1373`
  - 1161 行目: `INFO actor{actor=emo-text}: wintf::ecs::drag::dispatch: [DragEndEvent] Dispatching entity=25v0 x=2097 y=1373 cancelled=true`
  - 1163 行目: `INFO actor{actor=emo-text}: areka::persist::save: char DragEnd 保存 scope=0 char_x=1840 char_y=610 saved_x=2222 saved_y=610 char_w=Some(764) anchor=Bottom`
- 2 回目（相方）: 1353 行目の開始 → 1452 行目の終了（`cancelled=true`）→ 1455 行目の保存（`scope=1 saved_x=1474`）

保存した位置は、ドラッグを始める前の位置（R1 の終わりに保存された `scope=0 saved_x=2222`・`scope=1 saved_x=1474`）と同じ。取り消しで窓が元の位置へ戻り、その位置を保存している。これは areka の受け手のテスト `cancel_after_threshold_persists_as_today` の「今どおり」と同じ振る舞い。
