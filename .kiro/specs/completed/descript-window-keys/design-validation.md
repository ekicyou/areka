# 設計の検証: descript-window-keys

> 2026-10-23・ワークツリー `claude/kiro-start-descript-window-keys-451069`（`24a7eb96`）。`requirements.md`・`design.md`・`research.md`・steering を読み、設計が名指しするコードを実物で確かめた。SSP・areka の MCP・画面の操作は使っていない。

## まとめ

設計は、既存の 2 つの型（「準備が 1 度読んだ値を配る」と「帳簿＋書き手 1 か所＋ドラッグの控え」）をそのまま延ばしており、新しい仕組みを作らない。wintf の口・seriko の差し替え・読み直しの戻り先・受け手の表について、設計の書く行番号と働きは実物と合っていた。直す点は 3 つあるが、どれも設計の討議で決められる大きさで、根本の形を変えるものではない。

## 実物で確かめたこと

| 設計の主張 | 実物 | 結果 |
|---|---|---|
| wintf はドラッグの始まりに `DragConfig.move_window` を読み、`WindowDragContextResource` に控える | `wintf/src/ecs/drag/dispatch.rs` 135〜152 行目で読んで `set`、その後 191 行目で `OnDragStart` を配る | 合う。`on_balloon_drag_start` が呼ばれるとき控えは書かれている（I2 は順の上で成り立つ） |
| バルーンの窓は `DragConfig::default()`（`move_window: true`）で、`OnDragStart` は付いていない | `placement/spawn.rs` 455 行目。付くのは `OnDrag`・`OnDragEnd` だけ | 合う |
| キャラクターの窓に先例がある | `placement/follow/sticky_follow.rs` の `sync_drag_writer`（61 行目）・`CharDragSession`・`drag_wintf_moves`（203 行目）。始まりの控えは `input_events/drag.rs` の `on_char_drag_start` が作る | 合う |
| `input_events/drag_tests.rs` 161 行目の檻 | `balloon_drag_sends_nothing` が `OnDragStart` の無いことを確かめる。組み立て（`drag_test_support.rs`）は `spawn_ghost_windows`＋`attach_char_pointer_handlers` で、`seed_balloon_lock` を通らない | 檻は通る。ただし下の問題 3 |
| seriko の差し替えは `rebase_shell` の後に `rebased()` で今の面を載せる | `areka-seriko/src/actor.rs` 314〜326・364〜392 行目 | 合う。`replace_shell_table` は Shell の時計を捨てるだけなので、戻り先へ替えた面も次の刻みで新しい表から始まる |
| 読み直しの戻り先は `shell_carry::decide(carried, initial, exists)` の 1 段 | `emo2_boot/shell_carry.rs` 103 行目・`frame/attach.rs` 285 行目 | 合う |
| `close_windows_for_restart` が窓の組の帳簿と控えを外す | `app_exit.rs` 201〜202 行目。呼ぶのは `ghost_switch.rs` の 2 か所だけ（SHIORI だけの読み直しは通らない） | 合う |
| `run_move_drain_phase` の照合は網羅で、窓が無い間は線が控えを兼ねる | `frame/drain_resnap.rs` 140〜192 行目 | 合う。読み直しでは結線（`Emo2Wiring`）ごと取り替わるので、前のゴーストの lock が次へ漏れない |
| 受け手の表の 5 組・手書きの数 36・見本 | `consumer_ledger.rs` 475〜485 行目・`consumer_ledger_tests.rs` 294〜342 行目・`consumer_ledger_agreement_tests.rs` 126〜130 行目 | 合う。`lock`・`unlock` の既存の登記は無く、重ならない |
| 1,000 行に近いファイル | `placement/mod.rs` 939・`drag_follow.rs` 958・`ghost_session.rs` 917・`measure_tests.rs` 993・`emo2_boot/spine.rs` 940・`frame_attach_tests.rs` 905 | `measure_tests.rs` だけが越える（問題 1） |
| 決め打ちの 3 か所 | `emo2_boot/assets.rs` 36 行目・`placement/measure.rs` 43 行目・`examples/window-placement.rs` 123 行目 | 合う |
| 並走ウェーブとの約束 | ⑥: `PositionCue` の腕 1・`MoveCueSink` の 1 組・`run_move_drain_phase` の 1 腕で、`spawn.rs`・`frame.rs`・wintf は触らない。⑦: 表の 2 行・数 36→38・見本 2 件・`consumer_of` は触らない | 設計の書き方は約束どおり。ただし `consumer_ledger_tests.rs` に新しい檻を 1 つ足す点は、⑦ との約束の字面（数と見本だけ）より少し多い。⑦ と同じ所に手が入るので、着地の順で合わせる |

## 直す点（3 つ）

### 問題 1: `measure_tests.rs` が 1,000 行を越える

- **何が問題か**: `measure_tests.rs` は今 993 行で、`measure_scope_sizes` の呼び出しは 14 か所あり、どれも 1 引数 1 行の形で書かれている。設計どおり `&DefaultSurfaces::builtin(),` を足すと 1 か所 1 行ずつ増え、およそ 1,007 行になる。設計は「呼び出し 16 か所に足す（期待値は変わらない）」と書くだけで、行の上限に触れていない。数も実物と違う（`measure_tests.rs` は 14 か所、`measure_template_tests.rs` は `measure_scope_sizes` を呼ばず、直すのは 35 行目の注記だけ）。
- **なぜ効くか**: 1 ファイル 1,000 行以下は steering の決まり。実装の段で初めて気付くと、テストの置き場をその場で決め直すことになる。
- **直し方**: 設計に置き場を書く。案は 2 つ。⑴ `measure_tests.rs` に、今の 4 つの引数を受けて `DefaultSurfaces::builtin()` を足すだけのテスト用の小さな関数を 1 つ置き、14 か所はその関数を呼ぶ（行は増えない）。⑵ 既存のテストのうち 1〜2 本を新しい兄弟 `measure_default_surface_tests.rs` へ移す。⑴ の方が差分が小さい。あわせて「16 か所」を実物の数に直す。
- **要件**: 7.2・2.6（今の期待値を変えない）
- **設計の場所**: 「File Structure Plan > Modified Files」の `measure_tests.rs` の行

### 問題 2: シェルの切り替えで戻り先の面へ替えても、文字の層は前の面の番号のまま

- **何が問題か**: 設計は seriko の「今の面」と出ている面をそろえる（Goals「表示と seriko の今の面を食い違わせない」）。しかし「今の面」を持つ所はもう 1 つある。文字の層（`areka-emo-text`）は、面の中の文字の箱へ文字を流すためにスコープごとの面の番号を持つ（`state_route.rs` の `current_surface`）。シェルの切り替えの `ShellBoxAssets::hand_to` → `set_box_layout` は「番号は保ち、行き先を既定へ引き直す」働きで、合図 `Rebased` は文字の層へ届かない（`adapter.rs` の `send_rebased` は表示の層へだけ送る）。そのため、面 1000 が新しいシェルに無く seriko が面 0 へ戻しても、文字の層は面 1000 のままになる。面 0 に文字の箱があっても、次の `\s` まで文字はそこへ入らず、普通のバルーンへ流れる。
- **なぜ効くか**: 今は窓が空のままなので目立たない。本設計で戻り先の面が出るようになると、「出ている面」と「文字の行き先が見ている面」が食い違う状態を新しく作ることになる。文字の箱を持つシェルでだけ起きる。
- **直し方**: 設計で次のどちらかを決める。⑴ seriko の戻り先の結果（`ShellFallbackOutcome`）を、`finish_shell` で文字の層へも渡す。例えば箱の束を渡した直後に、戻したスコープだけ `route_surface(scope, Show(to))`（隠したスコープは `Hide`）を当てる。決定論のテストを 1 本足す。⑵ 本 spec の範囲外とするなら、Out of Boundary と Revalidation Triggers に書き、完了のときに起票する。⑴ は触るファイルが `frame/switch.rs`（666 行）だけで済む見込みで、要件 3.2 の「その面を出す」をそろって満たせる。
- **要件**: 3.2・3.5
- **設計の場所**: 「System Flows > 既定の面: 測りと戻り先」のシェルの切り替えの項・「SwitchSplit・SerikoShellFallback」

### 問題 3: 調べること I4 を設計のうちに決める（`OnDragStart` の檻と付け方）

- **何が問題か**: 実物を読むと、`input_events/drag_tests.rs` 161 行目の檻の組み立ては `seed_balloon_lock` を通らない。そのため檻は今のまま通る。しかし本番ではバルーンの窓に `OnDragStart` が付くので、この檻の「付けるのはキャラクター窓だけ」という主張は本番について正しくなくなる。正しく動くことと、檻が実物を守っていることは別。さらに、受け手を付ける働きが帳簿を据える関数（`seed_balloon_lock`）の中に隠れている。`spawn_ghost_windows` だけで窓を組む既存のテストの組み立て（`drag_test_support.rs`・`balloon_test_support.rs` など）では、受け手の付いていないバルーンになる。
- **なぜ効くか**: 檻が本番と違う組み立てを守り続けると、後で誰かが本番のバルーンの受け手を外したり替えたりしても、どの檻も赤にならない。
- **直し方**: I4 を「調べること」から外し、設計で決める。⑴ 受け手を付ける働きを `seed_balloon_lock` から分け、`balloon_lock::attach_balloon_drag_handlers(world)` として、窓を作る閉包の `input_events::attach_char_pointer_handlers` の隣で呼ぶ（付け方の先例と同じ置き方）。⑵ `drag_tests.rs` の該当の行は、「ゴーストへ知らせる受け手（`on_char_drag_start`）がバルーンの窓に付いていない」ことと `drain()` が空であることを確かめる形へ直す。本番の付け方を通した組み立てで、バルーンの窓に `OnDragStart` が付くことを `balloon_lock_drag_tests.rs` で 1 本確かめる。
- **要件**: 6.1・6.6・7.4
- **設計の場所**: 「BalloonLock > Implementation Notes」の最後の項・「調べること」I4

## 良い点

- **1 フレーム遅らせる解を、どの道でも取っていない。** 測りは窓を作る前に 1 回だけ行う。読み直しの戻り先は装着の前の World で決める。シェルの切り替えの戻り先は、seriko が `Rebased` を出す前に今の面を書き換える。そのため `ReplaceTarget { show }` は最初から戻り先の面を載せる。バルーンの `DragConfig` も、窓を作るのと同じ閉包の中で付け直す。開発者の方針（状態の持ち方を変えて 0 フレームで解く）にそのまま沿っている。
- **ドラッグの途中の lock（要件 5.6）を、wintf が始まりに控える値に合わせて解いている。** wintf の中を触らずに、areka の控え（`BalloonDragSession.held`）も同じ始まりの値だけを見る。マウスと窓が離れる中途半端な状態が、形の上で起きない。先例（`CharDragSession`・`drag_wintf_moves`）と同じ形で、黙った失敗も無い（帳簿が無いときは `warn!` を残して据える）。

## 判定

**GO**

- **理由**: 既存の作りとの食い違いは無く、要件 1〜9 のすべてに道筋がある。3 つの問題は、行の上限の置き場・文字の層への 1 手・檻の言い分の直しで、どれも設計の文を直せば片づき、根本の形を変えない。
- **次の手順**: 設計の討議（`/kiro-design-discussion descript-window-keys`）で問題 1〜3 の直し方を決めて `design.md` へ反映し、その後に `/kiro-spec-tasks descript-window-keys` へ進む。
