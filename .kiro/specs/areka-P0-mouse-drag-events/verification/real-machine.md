# 実機の記録: areka-P0-mouse-drag-events（タスク 4.2・要件 3.4・8.1・9.3）

## 1. 走行の条件

| 項目 | 値 |
|---|---|
| 版 | コミット `9602f640`（4.1 まで・決定 D7 の改修 2.5 を含む）から組んだ debug 版 `areka.exe`（x64）と `shiori-host32-helper.exe`（i686） |
| 根 | `<ワークツリー>\target\mde-signoff\root`（`real-machine-run.ps1 -Prepare` で `nar-sample-path claudia` の展開から写したもの） |
| 検体 | クローディア（`sample-ghost-kit` の `claudia`・YAYA・32bit） |
| 根の写しへの手当て | `ghost\claudia\shell\master\surfaces.txt` の `element0,base,` 5 か所を `element0,overlay,` に書き替えた（R2 から）。areka が描画メソッド `base` を描けず、surface6・11・26 が上乗せの部品の大きさだけで描かれてキャラクターが消えるため（下の 4 章 ⑴）。製品のコードとリポジトリの検体は無改変 |
| 記憶 | R7b 以外は起動の前に `profile` とゴーストの `profile\areka` を消した。R7b は R7a の記憶を残した |
| `RUST_LOG` | `info,areka=debug,kanade=trace,wintf::ecs::drag=debug` |
| 安全弁 | `AREKA_APP_SMOKE_EXIT_MS=900000`（どの走行でも発火していない） |
| 起動 | `.kiro/specs/areka-P0-mouse-drag-events/real-machine-run.ps1 -Run <項目>`。記録は `target\mde-signoff\run-<項目>.log` |
| 操作 | 開発者（1 項目ずつ GO を待って起動） |

`runs.txt`（逐語・R1 は開発者が閉じる前にこちらで止めたので exit=-1。R4 は 1 回目で手順 3 がダブルクリックにならなかったのでやり直した）:

```
R1 pid=15260 start=2026-10-05T12:57:04.0328592+09:00 exit_ms=900000
R1 exit=-1 end=2026-10-05T13:03:30.0914617+09:00
R2 pid=16600 start=2026-10-05T13:03:49.5375434+09:00 exit_ms=900000
R2 exit=0 end=2026-10-05T13:04:16.1760495+09:00
R4 pid=23836 start=2026-10-05T13:04:52.9686028+09:00 exit_ms=900000
R4 exit=0 end=2026-10-05T13:06:11.3016149+09:00
R4 pid=7172 start=2026-10-05T13:06:42.5375879+09:00 exit_ms=900000
R4 exit=0 end=2026-10-05T13:07:07.1538862+09:00
R5 pid=32724 start=2026-10-05T13:11:14.4378332+09:00 exit_ms=900000
R5 exit=0 end=2026-10-05T13:11:40.7079447+09:00
R6 pid=4264 start=2026-10-05T13:12:08.7763879+09:00 exit_ms=900000
R6 exit=0 end=2026-10-05T13:12:25.9639545+09:00
R7a pid=21384 start=2026-10-05T13:12:55.8300386+09:00 exit_ms=900000
R7a exit=0 end=2026-10-05T13:13:09.1569740+09:00
R7b pid=31904 start=2026-10-05T13:13:26.5267984+09:00 exit_ms=900000
R7b exit=0 end=2026-10-05T13:17:15.4569785+09:00
```

## 2. 件数（記録から数えたもの）

| 記録 | `OnMouseDragStart` | `OnMouseDragEnd` | `OnMouseDoubleClick` | `mouse_drag_dropped` | ERROR | WARN |
|---|---|---|---|---|---|---|
| R1 | 2 | 2 | 0 | 0 | 0 | 0 |
| R2 | 2 | 2 | 0 | 0 | 0 | 0 |
| R4（1 回目） | 1 | 1 | 1 | 0 | 0 | 0 |
| R4（やり直し） | 1 | 1 | 1 | 0 | 0 | 0 |
| R5 | 1 | 1 | 0 | 0 | 0 | 0 |
| R6 | 1 | 1 | 0 | 0 | 0 | 0 |
| R7a | 1 | 1 | 0 | 0 | 0 | 0 |
| R7b | 0 | 0 | 0 | 0 | 0 | 0 |

どの記録でも、`OnMouseDragStart` の直後に同じ窓の `OnMouseDragEnd` が来て、開始 → 終了の順が崩れたものは無い。

## 3. 判定

### R1（本体のドラッグ・要件 9.3・8.1）: 合格

- 本体: `OnMouseDragStart`（`["171", "358", "0", "0", "", "0", "mouse"]`）→ 絵が `\s[29]`（333×500）→ 離すと `OnMouseDragEnd` → `DragEnd0` の台詞。
- 相方も 1 回ドラッグされ、Reference3 が `1`・絵が `\s[19]`。
- 開発者の申告「ドラッグ後、クローディアが表示されない」: 離した後の台詞の `\s[26]` が `element0,base,…` の面で、上乗せの部品（100×56）だけで描かれたため（4 章 ⑴）。2 つのイベントの送り方とは関係ない。起動直後の `\s[26]` でも同じ現れ方をする。

### R2（相方のドラッグ）: 合格

- 2 回とも `OnMouseDragStart` → `OnMouseDragEnd` が各 1 件・Reference3 が `1`・掴んでいる間の絵が `\s[19]`（333×500）・離すと位置の保存と相方の台詞。
- 根の写しの書き替え後、本体の `\s[26]` は 333×500 で出るようになった。

### R3（当たり判定の外・箱の上から）: 合格（走行なし・R1・R2 の記録で判定）

- クローディアの当たり判定は `collisionex` だけで書かれており、areka は読まない（4 章 ⑵）。R1・R2 の 4 回のドラッグはどれも「当たり判定の外」から始まった扱いで、4 回とも Reference4 が空のまま送られている（要件 1.3・4.5）。
- クローディアのシェルは箱を定義していない（`surfaces.txt` に `balloon` の element定義が無い）。design の「箱を出しているなら」に当たらないので、箱の上からのドラッグは該当なし。

### R4（クリック・ダブルクリック・2 回目の押下のまま動かす・要件 3.1・3.2・決定 D7）: 合格

- 1 回目の記録: クリックだけ → 準備に入り離すと「開始の無い終了」として捨てられ、ドラッグの知らせ 0 件。ダブルクリックだけ → 2 回目の押下（`WM_LBUTTONDBLCLK`）から準備に入り（`[handle_double_click_message] Double-click detected` の直後に `[start_preparing] DragState -> Preparing`）、`OnMouseDoubleClick` が 1 件、ドラッグの知らせ 0 件。手順 3 は 2 回目が普通の押下として届いたので、やり直した。
- やり直しの記録（`run-R4.log`）: 49.637 普通の押下（離すと捨てられる）→ 49.877 `WM_LBUTTONDBLCLK` で `OnMouseDoubleClick` 1 件 → 間に押下が無いまま 53.215 `OnMouseDragStart`（座標 `139,306` はダブルクリックと同じ）→ 54.898 `OnMouseDragEnd`・位置の保存。

### R5（右ボタン・バルーン窓・要件 3.3・3.4）: 合格

- バルーン窓の左ボタンのドラッグ: wintf はバルーンの窓として開始と終了を配り、バルーンの位置を保存したが、`OnMouseDrag*` は 0 件。
- 本体の右ボタン: 押下と離しが記録され、ドラッグの準備は始まらず、ドラッグの知らせ 0 件。離したところでメニューが開いた。押下と離しの位置は同じ（client 193,699）で、押したまま動かした証跡は無い。ただし wintf はドラッグの準備を左ボタンでしか始めない作り（`mouse_click.rs` の `button == PointerButton::Left` の分岐）なので、動かしても 0 件は変わらない。
- この回の最初に本体の左ボタンのドラッグが 1 回あり、開始 → 終了が各 1 件（表の 1 件ずつはこの分）。

### R6（ESC の取り消し・要件 2.2・4.2）: 合格

- `OnMouseDragStart`（窓の開始の位置 `2214,704`・Reference `145,347`）→ ESC で `[DragEndEvent] Dispatching … cancelled=true`（位置は押した位置）→ 保存 `char_x=2214 char_y=704`（開始の位置へ戻った）→ `OnMouseDragEnd` 1 件・Reference `145,347`（開始と同じ）。
- 開発者の申告（逐語）: 「元の一に戻った」。

### R7（再起動で動かした位置に立つ・要件 6.1）: 合格

- R7a: 本体のドラッグで `OnMouseDragStart` → `\s[29]` → `OnMouseDragEnd`、保存 `char_x=1169 char_y=272`。掴んでいる間にキャラクター窓の置き直し・大きさの変更は 0 件（`\s[29]` は `\s[0]` と同じ 333×500）。
- R7b: `merge_scope restore scope=0 … saved_win_x=Some(1169) saved_win_y=Some(272) … char_x=1169 char_y=272`。本体は動かした位置に立った。
- 開発者の申告: 相方も動いていた。相方は R7a で一度もドラッグしておらず記憶に位置が無いので、既定の位置から起動の最後の `chain_finalize` で本体の左隣へ並べ直された（`scope=1 from_x=1548 to_x=503`）。SHIORI の台詞に移動の指示は無い。本 spec の前からの振る舞いで、4 章 ⑶ に起票の対象として残す。

## 4. 範囲外・起票の対象（開発者 2026-10-05「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」）

- ⑴ element定義の描画メソッド `base` を描けない。2 か所ある: `areka-parsers` の `shell::decode_elements` が `overlay` 以外の element定義の行を読み捨てる（台帳の element定義の行は縮退で、担当は完了済みの `areka-P0-shell-parse`）。合成器 `areka-emo-compose` も `ComposeMethod::is_implemented` が `overlay` だけで `base` を描けない（台帳の `base` の行は担当なし）。両方を直さないと `base` は描けない。進行中の担当 spec は無い。`element0,base,<別の絵>` ＋ `overlay` の面が上乗せの部品の大きさだけで描かれ、キャラクターが消える。
- ⑵ `collisionex` を読まない（`areka-parsers` の `shell::decode`）。`collisionex` だけで当たり判定を書くゴーストでは Reference4 がいつも空。
- ⑶ キャラクター窓の位置を記憶に書くのはドラッグの終了だけで、終了するときや最初に並べ終えたときに書かない。一度もドラッグしていないキャラクターは、再起動で前回の並びに戻らない。
