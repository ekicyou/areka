# ギャップ分析: window-resetwindowpos-hold

> 2026-11-21・`/kiro-validate-gap`（要件の段の下書き `requirements.md` に対して・main `f6405e09` の上のワークツリー）。決めは書かない。案と材料と調べの残りだけを書く。行番号は今のワークツリーのもの。

## 1. 分析のまとめ

- **今ある形がほぼそのまま使える**: 預かりの判定 `release_ready`（`crates/areka-runtime/src/frame/move_hold.rs` 188 行）は、窓の在庫（`GhostWindows`）と表示の読み口（`ShownRatioSource`）をすでに受け取っている。並べ直しを「窓のある全部のスコープが最初の面を表示するまで」預かる判定は、`unshown_scope`（135 行）と同じ材料で書ける。公開の入口 `run_move_drain_phase`・`run_move_drain_with` の形は変えずに済む。
- **まず確かめること（重い）**: 起動の確定 `finalize_chain_once_with`（`drain_resnap.rs` 419 行）は、全部のスコープが表示した巡に、既定の位置のままの窓の鎖を実表示寸で解き直す。並べ直しは窓の台本の印 `ScriptPlaced` を外し、既定の位置を並べた位置に書き換える（`reset_pos.rs` 114・117 行）。このため、起動の確定より前に当たった並べ直しの窓は、確定の段で「既定のまま」とみなされて解き直される見込みが高い。窓どうしの横の隙間（要件 1.5）は、今のコードでも同じ巡の後の相で消える可能性がある。残りそうなのは、先頭の窓が作業領域の右端にそろわない（幅の差の半分だけ離れる）点で、§8 の並べ直しの行（143 行）が「起動」の名残として書いている形と同じである。要件 7.1 の赤をどの形で作るかは、この確かめで変わる。
- **「待っても表示しないと分かった窓」の確かな合図は今のコードに無い**。あるのは「まだ表示していない」だけである（§3 (a)）。起動の確定もこの待ちを「ゴースト待ち」として数えず、際限なく待つ（`drain_resnap.rs` 401〜409 行・檻 `never_shown_scope_is_a_ghost_wait_and_is_not_counted`）。
- **並べ直しを預かると、今の檻 2 本が赤になる**。areka-runtime の `a_reset_after_a_move_to_a_never_shown_window_supersedes_it`（`frame_move_hold_tests.rs` 233 行）と、**本体の** `a_script_reset_sends_nothing_to_kanade`（`crates/areka/src/runtime_tests/frame_move_order_tests.rs` 21 行）。後の 1 本は同じウェーブの `session-module-gather` がパスを直すファイルにある（§3 (b)）。
- 案は 4 つ（§5）: 案 0＝預からず、起動の確定に任せる／案 A＝スコープごとの表示で預かる／案 B＝起動の確定の印が立つまでだけ預かる／案 C＝A か B に「表示しない窓」の判定を足す。規模は S〜M、危うさは中（§6）。

## 2. 今のコードの地図

### 2.1 位置の指令の線

| 部品 | 場所 | 今の働き |
|---|---|---|
| `run_move_drain_phase` | `crates/areka-runtime/src/frame/drain_resnap.rs` 149 | 本番の入口。`run_move_drain_with(wiring, &wiring.presenter, world)` を呼ぶだけ |
| `run_move_drain_with<S: ShownRatioSource>` | 同 154 | `GhostWindows` が無ければ戻る（チャネルが預かる）。`release_ready` に預かりの列・届いた指令・窓・読み口・組の帳簿を渡し、返った順に当てる。並べ直しの腕は 175 行（`reset_window_positions(world)`・拡大率は読まない） |
| `ShownRatioSource` | `move_hold.rs` 31 | `shown_ratio(scope) -> Option<ScaleRatio>`。本番は `EmoPresenter::applied_ratio(shell_target(scope))`、檻は閉包（`all_shown`・`shown_only`） |
| `touched_scopes` | 同 84 | 指令が関わる窓。`ResetWindowPos` は `None`＝全部の窓（127 行） |
| `unshown_scope` | 同 135 | 移動の対象と基準のうち、窓があってまだ表示していないもの。窓の無いスコープは待たない |
| `Blocked` | 同 152 | 預かった指令が関わる窓の集合（`all`＋`BTreeSet<u32>`）。`all` が立つと後の全部の指令が追い越さずに預かられる |
| `release_ready` | 同 188 | 預かり（先）と届いた指令（後）を順に判定。並べ直しが来て預かりに移動があれば、移動を捨て（`move_hold_superseded`・219 行）、残りを並べ直しの前へ戻して判定し直す（207〜233 行）。表示待ちの判定は `PositionCue::Move` だけ（236〜239 行）。記録は届いた巡に 1 度（`just_arrived`） |
| フレームの順 | `crates/areka-runtime/src/frame.rs` 199〜238 | 表示の取り出し → 窓寸の照合 → 再スナップ 220 → **位置の指令の相 226** → 面の `move` のずれ 230 → **起動の確定 233** → 拡大率の後の詰め直し 238 |

### 2.2 並べ直しの本体（読むだけ）

- `crates/areka-placement/src/reset_pos.rs` 50 `reset_window_positions`。窓の大きさは `WindowPos.size`（76 行）から読むので、表示の前は仮の大きさになる。材料が無ければ `reset_window_pos_skipped`（53・88 行）。並べた窓ごとに `move_window_to`→`rebase_shift`→`set_default_char_pos`（114）→`ScriptPlaced` を外す（117）→`note_script_moved`（119・詰め直しの武装中だけ効く）→`reset_window_pos` の info。最後に `rejudge_all_balloons`（132）と `persist_unremembered_char_positions`（133）。
- 戻り値 `bool` は捨てられている（今の相の決まり）。失敗の道は `warn!`＋`false` で、`Err` を返す道は無い（要件 5.6 の「失敗として返す」をどこで満たすかは §7 R5）。

### 2.3 起動の確定（並べ直しと重なる働き）

- `finalize_chain_once_with`（`drain_resnap.rs` 419）は `ChainFinalized` が無い間、全部のスコープの実表示寸が引けて窓寸と一致した巡に 1 度だけ走る。`NotShownYet` は「ゴースト待ち」で数えない（432〜440 行）。
- 組の決め `boot_pair_decision`（`crates/areka-placement/src/chain_finalize.rs` 126）は `default_x == current_x` かつ `ScriptPlaced` 無しなら解き直す。並べ直しは既定の位置を並べた位置へ書き、印を外すので、この条件に入る。表示の後の再アンカーは既定の位置を同じ量だけ運ぶ（`follow/window_move.rs` 774 `track_default_char_pos`）ので、一致は保たれる。
- 確定の後に `persist_unremembered_char_positions` をもう 1 度呼ぶ（564 行）ので、並べ直しが仮の位置で書いた記憶は、確定の位置で書き直される見込み。
- `window-work-area-clamp` の測り ⑸ で 52px の隙間が残ったのは、台本の `\![move]` が置いた窓に `ScriptPlaced` が付き、確定の段で「explicitly-placed」として外されたためである（同 spec の `research.md` 12 節）。並べ直しはこの印を外すので、同じ形にはならない見込み。

### 2.4 表示の状態（`areka-emo-present`）

| 読み口 | 場所 | `None`／偽になるとき |
|---|---|---|
| `applied_ratio(target)` | `presenter/read.rs` 166 | 未登録・一度も表示が成り立っていない。更新は表示の成り立つ点だけ（`presenter/show.rs` 421）。`Hide` では消えない。置き換え（`replace.rs` 98）では引き継ぐ |
| `target_physical_size` | `read.rs` 124 | 同じ（起動の確定と再スナップが読む） |
| `last_shown` | `presenter/snapshot.rs` 47 | 一度も表示していない。`Hide` では消えない |
| 今の面（`current_surface_id`） | `hub.rs` 254・`show.rs` 403 | 一度も表示していない・`Hide` の後 |
| `failure_note`（非公開） | `presenter/target.rs` 199 | 描く装置の失敗を覚えるだけ。合成の失敗では `None` に戻す（`show.rs` 278・289） |

## 3. 開いた問いへの答え（コードの事実）

### (a) スコープごとの「表示した」の合図と、表示しない窓の見え方

- 合図は `EmoPresenter::applied_ratio(shell_target(scope))` だけで、`ShownRatioSource` がそれを包む。`Some` は「一度は表示が成り立った」、`None` は「未登録か、まだ一度も成り立っていない」。
- `\s[-1]` は呼び手（seriko）が `PresentCommand::Hide` に写す（`command.rs` 9 行）。`apply_hide`（`hub.rs` 232）は可視を落とし、今の面を `None` にするだけで、`applied` には触らない。**最初の面より前に隠された窓は、`applied` が `None` のまま**で、「まだ表示していない窓」と見分けがつかない。一度表示した後に隠した窓は `Some` のまま（表示済み扱い）。
- 面を出せなかった窓（合成の失敗・装置の失敗・未装着）は、`show.rs` の早期の戻り（76・190・290 行）で `applied` を書かないので、同じく `None` のまま。失敗は `error!`／`warn!` と呼び手への `Err` で残るが、スコープの状態としては残らない（`failure_note` は装置の失敗だけ・非公開・合成の失敗では消える）。
- **「待っても表示しない」と言える合図は無い**。どの状態からも、後の `\s[n]` で表示が成り立ちうる。今の設計の前例（起動の確定）は、この待ちをゴーストの都合として際限なく待ち、上限を置かない（巡や秒で区切るとどこかのゴーストの正常な待ちで鳴るため・`drain_resnap.rs` 404〜409 行）。

### (b) 期待を改める今のテスト

| テスト | 場所 | 預かると何が変わるか |
|---|---|---|
| `a_reset_after_a_move_to_a_never_shown_window_supersedes_it` | `crates/areka-runtime/src/frame_move_hold_tests.rs` 233〜265 | `shown_only(&[0])`（相方が未表示）で並べ直しを送る。案 A では並べ直しが預かられ、`Blocked.all` が立つ。252〜255 行の「`reset_window_pos` の記録がある」と 256 行の「後の本体の指令が当たった（x=30）」が赤になる。247〜251 行の `move_hold_superseded` と 258〜264 行の「捨てた移動が後で当たらない」は残せる。`steady_harness` は既定の位置の材料を置かないので、並べ直しは `reset_window_pos_skipped` の記録だけを出す（253 行の `contains("reset_window_pos")` はこの行にも当たる） |
| `a_script_reset_sends_nothing_to_kanade` | `crates/areka/src/runtime_tests/frame_move_order_tests.rs` 21〜51（**本体**） | `wiring()` は素の `EmoPresenter::new()`（全部のスコープが未表示）で `run_move_drain_phase` を呼ぶ。案 A では並べ直しが預かられ、44 行の「`reset_window_pos_skipped` が 1 行」が 0 行になって赤。同じウェーブの `session-module-gather` がこのファイルのパスを直す（brief の C24 の約束）。直すなら `run_move_drain_with(&wiring, &all_shown, …)` へ替える 1 行だが、本体のテストに触る |
| 兄弟の 2 本（`a_sticky_group_unrelated_to_a_never_shown_window_goes_first` 269・`a_balloon_reset_after_a_move_to_a_never_shown_window_goes_first` 317） | `frame_move_hold_tests.rs` | 並べ直しを送らないので変わらない |
| `run_move_drain_phase_routes_reset_and_continues_after_its_failure` | `crates/areka-runtime/src/frame_drain_text_tests.rs` 257 | `all_shown` で回すので変わらない |
| `spine_*`（本体） | `crates/areka/src/runtime_tests/spine*.rs` | `all_shown` で回し、並べ直しを流す台本も無い（`resetwindowpos` を含むテストは上の 1 本だけ）。変わらない見込み |
| `Blocked` の檻 | `crates/areka-runtime/src/frame/move_hold_tests.rs` | 変わらない |

案 B（起動の確定の印 `ChainFinalized` が立つまでだけ預かる）では、上の 2 本はどちらも `steady_harness`（`ChainFinalized` を差す・`frame_shared_test_support.rs` 65〜70 行）なので**緑のまま**になる。その代わり、新しい赤の檻は `ChainFinalized` の無い足場（`FrameHarness::new()`）で組む。

### (c) `doc/COMPAT_ARCHITECTURE.md` §8 の件 5 の行

- 452 行（§8「沈黙ルール対応表」・124 行から）。第 1 列は「起動のトーク（初回を含む）の台本の位置の指令（`\![move]`・重なりの順の指令・窓を組にする指令）を当てる時機（ukadoc は起動のイベントの台本をシェルの表示のどの時点で流すかと、位置の指令が基準の窓の表示より先に来たときに沈黙）」。
- 第 2 列（areka の決め）は、移動を対象と基準の窓の最初の表示まで預かること、同じ窓は追い越さず関わりの無い窓は先に当てること、**並べ直し（`resetwindowpos`）は前に預かった移動を捨て `move_hold_superseded` を残すこと**、初回起動や特定のゴーストを名指ししないことを書く。並べ直しそのものを預かるかは書いていない。
- 第 3 列は SSP 2.9.26 の測り ⑸（初回の台本は `Materialized.` の後）と、areka の 52px の隙間、「起動のイベントを流す時機は SSP に合わせず、位置の指令の側で表示を待って差を消す」こと、定義点 `move_hold.rs` の `release_ready` を書く。第 4 列は `window-work-area-clamp（要件 6.1〜6.4・8.1）`。
- 見えない窓への `\![move]` を SSP がどう扱うかの測りは、この行にも §8 のほかの行にも無い（「`move`」と「隠す・非表示・表示されない」を同じ行に持つ行が無い）。brief の申し送りの「§8 の該当の行に SSP 2.9.26 の記録がある」は、この行の測り ⑸（初回の台本の時機）を指すと読める。見えない窓への移動の扱いそのものは測っていない（§7 R3）。
- 隣の 143 行（`\![execute,resetwindowpos]` の並べ直し）は「今表示している窓の大きさで解く」と「起動（仮の面で測った名残で、先頭のスコープが幅の差の半分だけ離れる）と違い、今の大きさで右端にそろう」と書く。この spec の決めがこの行にも触れるかは議題になりうる（brief の約束は「§8 は件 5 の行だけ」）。

### (d) 入口の形を変える必要があるか

- **変えずに済む**。`run_move_drain_with` は `GhostWindows`・読み口・`world` をすでに持ち、`release_ready` へ窓と読み口を渡している。並べ直しの待ちは `release_ready`（`pub(super)`）の中で `windows.scopes()` を回し、`char_window` のあるスコープに `shown.shown_ratio(s).is_none()` が 1 つでもあれば預かる、と書ける。
- 案 B で `ChainFinalized` を見るなら、`run_move_drain_with` が `world.get_resource::<ChainFinalized>()` を読んで `release_ready` へ渡す（`release_ready` の引数が 1 つ増えるが `pub(super)` なので約束の外）。
- 案 C で「表示しない窓」の合図を表示層から引くなら、公開のトレイト `ShownRatioSource` にメソッドを足すことになる。既定の実装つきで足せば、閉包の実装（`impl<F: Fn(u32) -> Option<ScaleRatio>>`）と本体の呼び手（`all_shown` を渡す `spine*`）はそのまま通る。入口の関数の形は変わらない。表示層（`areka-emo-present`）に新しい状態を足すかは別に決める（§5 案 C）。

## 4. 要件と資産の対応

| 要件 | 今ある資産 | ギャップ |
|---|---|---|
| 1.1〜1.3 並べ直しを最初の表示まで預かる・表示の巡で当てる・表示済みなら今どおり | `release_ready`・`unshown_scope`・フレームの順（再スナップの後に位置の相） | **Missing**: `ResetWindowPos` に表示待ちの判定が無い（236〜239 行は `Move` だけ）。表示の巡で当てる順は今のフレームの順で満たせる |
| 1.4 窓の無いスコープは待たない | `unshown_scope` の `char_window(s).is_some()` | 同じ書き方で足りる |
| 1.5 並べた後に仮の大きさ由来の隙間を残さない | 起動の確定が既定のままの窓の鎖を解き直す（§2.3） | **Unknown**: 今のコードで横の隙間がすでに消えるかを確かめていない（§7 R1）。消えるなら残る差は先頭の窓の右端のそろいと記録の順 |
| 1.6 並べ方と記憶への書き方を変えない | `reset_pos.rs` | 触らない |
| 2.1 前に預かった移動を捨てる | 207〜233 行 | 預かった並べ直しが次の巡に判定し直されても、その前に預かった移動は 1 巡目で捨て済み。後の移動は並べ直しの後ろに並ぶので捨てられない（今の形のまま成り立つ見込み・檻で確かめる） |
| 2.2 移動でない預かりは並べ直しの前で判定し直す | 214〜231 行 | 今のまま |
| 2.3〜2.5 追い越さない・取りこぼさない | `Blocked.all`・`held` の列 | 並べ直しを `held` に積めば `blocked.add(None)` で `all` が立つ（今の形のまま） |
| 2.6 捨てた移動を後で当てない | 檻 258〜264 行 | 今のまま |
| 3.1〜3.2・3.4 表示しない窓で詰まらない・今の大きさで入れて記録 | 無い | **Missing／Unknown**: 「表示しないと分かった」合図が無い（§3 (a)）。`Blocked.all` が立つと表示済みの窓への後の指令も全部預かられる |
| 3.3 関わりの無い窓への前の指令は先に当てる | 今の判定（並べ直しより前に届いた指令は並べ直しより先に判定される） | 今のまま |
| 4.1〜4.2 表示しない窓への移動を捨てる・見込みのある間は預かる | 無い | **Missing**: 3 と同じ合図が要る。**Constraint**: 列の伸びは `held` の `VecDeque` の長さで、上限は無い |
| 4.3 SSP の測り | 測り ⑸ の手順（`window-work-area-clamp` の `research.md` 12 節） | **Unknown**: 見えない窓への `\![move]` の SSP の扱いは未測 |
| 5.1 待った記録を届いた巡に 1 度 | `just_arrived` の型（`move_ratio_unset`） | **Missing**: 並べ直しの待ちの語が無い（新しい語を作るか・`move_ratio_unset` を広げるか） |
| 5.2〜5.3・5.5 並べ直し・捨てた記録・材料不足の警告 | `reset_window_pos`・`move_hold_superseded`・`reset_window_pos_skipped` | 今のまま |
| 5.4 待たなかった・捨てた記録 | 無い | 3・4 の合図しだい |
| 5.6 判定の途中の失敗を返す | `release_ready` は失敗の道を持たない（`Vec` を返す） | **Constraint**: 今の判定には失敗しうる箇所が無い。`reset_window_positions` は `bool` で、相が捨てている（§7 R5） |
| 6.1 ゴースト・初回・イベントを見ない | 今の判定は窓と表示と帳簿だけを見る | 案 B の `ChainFinalized`（窓の一式ごとの起動の確定の印）がこの決まりに触れるかは議題 |
| 6.2 1 フレーム遅らせない | フレームの順（位置の相は再スナップの後・確定の前） | 表示の巡の同じ相で当たる |
| 6.4 メニューの道・バルーン位置初期化を変えない | メニューは `crates/areka/src/menu/reset_pos_frame.rs` 100・137 行で `reset_window_positions` を直に呼ぶ（位置の指令の線を通らない） | 触れない |
| 6.5 定常は待ちを足さない | 全部が表示済みなら判定はその巡で通る | 案 A・B とも満たす |
| 7.1 直す前に赤 | 足場 `FrameHarness`・`PerTargetSizes`・`run_resnap`・`run_chain_finalize`（`frame_test_support.rs`・945 行） | **Unknown**: 赤を「位置の相だけ」で作るか「確定まで通した形」で作るか（R1）。足場を足すなら新しいファイル（C24 の約束） |
| 7.3 今の檻の期待を改める | §3 (b) | 本体の 1 本が `session-module-gather` と重なる |
| 7.4〜7.5 実機 | 測り ⑸ の検体の作り方 | 検体に `\![execute,resetwindowpos]` を足す。並べ直しの前後の窓の矩形はログの `reset_window_pos` と確定の行で読める |
| 7.6 §8 件 5 の行 | 452 行 | 既にある行の書き換え |

## 5. 実装の案

### 案 0: 預からない（起動の確定に任せる）＋申し送りの件だけ

- 中身: 並べ直しの判定は変えない。決定論の檻で「表示の前の並べ直し → 表示 → 再スナップ → 位置の相 → 起動の確定」を通し、窓どうしの隙間が残らないことを固定する。§8 の件 5 の行に、並べ直しは預からず起動の確定が直すことを書く。要件 4（表示しない窓への移動）は別に扱う。
- 良い点: 触る所が最小。今の檻は全部緑のまま。本体のテストに触らない。
- 悪い点: 要件 1.1 の「預かる」を書きどおりには満たさない（要件の書き換えが要る）。先頭の窓の右端のそろい（幅の差の半分）は直らない。記憶へ一度仮の位置を書いてから書き直す。確かめで隙間が残ると分かれば成り立たない。

### 案 A: スコープごとの表示で預かる（brief の第一の案）

- 中身: `release_ready` で `ResetWindowPos` にも表示待ちの判定を当てる。待つのは `GhostWindows::scopes()` のうち `char_window` のあるスコープで、`shown_ratio` が `None` のもの。届いた巡に 1 度だけ待ちの記録を出す。預かれば `Blocked.all` で後の指令が並ぶ。
- 触る所: `move_hold.rs`（判定と記録・数十行）・`frame_move_hold_tests.rs`（233 行の檻の改め）・新しい檻のファイル（兄弟・`frame.rs` の `mod` の行）・本体の `frame_move_order_tests.rs`（1 行・`session-module-gather` と重なる）・§8 件 5 の行。
- 良い点: brief と要件の書きどおり。定常（全部が表示済み）は今と同じ巡で当たる。判定の材料が今のまま。
- 悪い点: 表示しない窓が 1 つあると `Blocked.all` で列全体が止まる。要件 3 の合図が要る（案 C と組む）。本体のテストに触る。

### 案 B: 起動の確定の印が立つまでだけ、未表示の窓を待つ

- 中身: 案 A の判定に「`ChainFinalized` が無い」を掛ける。印が立った後（窓の一式が全部表示して確定した後）は、未表示の窓があっても待たない。`run_move_drain_with` が印を読んで `release_ready` へ渡す。
- 良い点: 今の 2 本の檻は `steady_harness`（印あり）なので緑のまま＝本体のテストに触らない。定常で待ちが足されないことが構造で決まる（要件 6.5）。
- 悪い点: 起動の確定が「ゴースト待ち」で際限なく待つのと同じく、表示しない窓が 1 つあると、印が立たず並べ直しも後の指令も止まる（要件 3 は案 A と同じく残る）。印は窓の一式ごとの状態でイベントではないが、「起動の間だけ」の扱いに見えるので要件 6.1 に触れないかを決める要がある。起動の確定と並べ直しの預かりが同じ印に結びつく。

### 案 C: A か B に「表示しないと分かった窓」の判定を足す

判定の材料の候補（どれも今は無い・§3 (a)）:

1. **最初の面の前に `Hide` を受けた窓**を待たない: 表示層に「表示の前に隠された」を覚えさせ、`ShownRatioSource` に既定の実装つきのメソッドを足して読む。ゴーストが後で `\s[n]` を出せば表示しうるので「分かった」とは言い切れない。
2. **面の指令を受けた窓だけを待つ**（要件の未決の問い 2 の代わり）: 表示層に `ShowSurface` を受けたか（成り立ったかでなく）を覚えさせる。面の指令が来ていない窓は待たない。表示の失敗で `applied` が書かれない窓は、受けた印はあるのに表示しないので、失敗を別に覚える要がある。
3. **並べ直しを流したトークの表示が終わった時点**で、まだ表示していない窓を待たない: `TalkLifecycleSignal::DisplayEndAt`（`wiring.rs` 53 行）が材料になりうる。今の受け手は可視性の相 1 つで、位置の相が読むには受け渡しが要る。トークの終わりはイベントの名前ではないが、時機の決まりとして要件 6.1 に触れないかを決める要がある。

- 良い点: 要件 3・4 を満たす道がある。
- 悪い点: どの材料も表示層か結線に新しい状態が要り、触る所が `areka-emo-present` や `wiring.rs` へ広がる（規模が M へ寄る）。1・2 は「分かった」でなく「見込みが低い」の判定である。

### 組み合わせの見立て

- 要件 4（表示しない窓への移動の列の伸び）は、並べ直しの預かりの案とは別に、案 C の材料と同じ判定で扱える。材料を作らずに要件 4 を満たす形は無い（上限で落とす形は要件の仮置きが取らない）。
- 案 0 は R1 の確かめの結果しだいで成り立つかが決まる。A と B の分かれ目は「本体のテストに触るか」と「印を判定に使うか」。

## 6. 規模と危うさ

- **規模: S〜M**。判定と記録は `move_hold.rs` の中で数十行（S）。要件 3・4 のために表示層や結線に状態を足すと M（`areka-emo-present` の読み口・その檻・`ShownRatioSource` の拡張）。檻は 3〜6 本を新しい兄弟のファイルへ。
- **危うさ: 中**。判定そのものは今の形の延長で低いが、(1) 今のコードがすでに隙間を直している見込みがあり、何を赤にするかが決まっていない、(2) 「表示しない窓」の判定に確かな材料が無い、(3) 本体のテスト 1 本が同じウェーブの相手と重なる、の 3 つが残る。

## 7. 設計へ持ち越す調べ（Research Needed）

- **R1（先に・重い）**: 起動の確定より前に当たった並べ直しの後、相方が最初の面を表示した巡に、起動の確定が鎖を実表示寸で解き直して窓どうしの横の隙間が消えるかを、決定論の檻（`FrameHarness::new()`・`run_resnap`→`run_move_drain_with`→`run_chain_finalize` の順）で確かめる。結果で要件 1.5・7.1 の赤の形と、案 0 が成り立つかが決まる。先頭の窓の右端のそろい・縦の位置（上端に付く揃え方）・記憶に残る位置も合わせて見る。
- **R2**: 表示層の状態のうち、「最初の面の前に隠された」「面の指令を受けたが表示が成り立たない」をスコープごとに覚える最小の形（`areka-emo-present` の `PresentTarget` に足すか、位置の相の側で `PresentCommand` を見て覚えるか）。
- **R3**: SSP が見えない窓（最初の面を出していない・`\s[-1]` の窓）への `\![move]` をどう扱うか（ukadoc は沈黙・未測）。測るなら開発者の許しと排他テストの机が要る（要件 4.3）。
- **R4**: 拡大率の後の詰め直しの武装中に並べ直しが当たると、`note_script_moved` で窓が詰め直しから外れる（`chain_realign.rs` 213 行）。新しい拡大率の大きさが着地する前の並べ直しで同じ形の隙間が残りうるか（この spec の範囲外の見込みだが、同じ「仮の大きさ」の形）。
- **R5**: 要件 5.6 の「判定の途中の失敗」に当たる道がどこにあるか。`release_ready` は失敗しうる箇所を持たず、`reset_window_positions` の失敗は `warn!`＋`false` で相が捨てる。要件の書き方を今の形に合わせるかを決める材料。
- **R6**: 実機の検体（emo2 の写し・初回のトークの先頭に `\![execute,resetwindowpos]`）で、並べ直しが最初の面の前に届くか（同じ巡に面と位置の指令が届けば今でも表示の後に当たる・測り ⑸ の 2 回目は同じ巡だった）。

## 8. 要件の話し合いへの材料（議題の候補）

1. **R1 の結果で範囲を決め直すか**: 起動の確定がすでに窓どうしの隙間を直すなら、預かりを入れる理由は「先頭の窓の右端のそろい」「記憶へ仮の位置を書かない」「記録の順」に絞られる。案 0（預からない）を残すか。
2. **待ちの範囲**: スコープごとの表示で待つ（案 A）か、起動の確定の印が立つまでだけ待つ（案 B）か。B は印を判定に使うことが要件 6.1 に触れないかの判断が要る。
3. **「表示しないと分かった窓」の材料**（未決の問い 2）: 今のコードには「まだ表示していない」しかない。案 C の 1〜3 のどれを取るか、または「分かった」を作らずに別の形で詰まりを避けるか。
4. **本体のテスト `a_script_reset_sends_nothing_to_kanade` の扱い**: 案 A では 1 行の改めが要り、同じウェーブの `session-module-gather` がパスを直すファイルに触る。相手と順を合わせるか、案 B で触らずに済ませるか。
5. **表示しない窓への `\![move]`（申し送り・未決の問い 3）**: 範囲に入れるか。入れるなら議題 3 と同じ材料で捨てるか。SSP を測るか（R3）。
6. **待ちの記録の語**: 新しい語を作るか、`move_ratio_unset` を並べ直しへ広げるか（後の読み手のログの grep に効く）。
7. **§8 の書き換えの範囲**: 件 5 の行（452）だけか、並べ直しの行（143）の「起動と違い右端にそろう」の記述にも及ぶか（brief の約束は件 5 の行だけ）。
8. **要件 5.6 の書き方**（R5）: 今の判定に失敗の道が無いことに合わせて改めるか。→ 要件の話し合いの自明な直しで「失敗しうる道を足すときは `error!`＋`Err`」に改めた。

### 8.1 要件の話し合いで設計の段へ回した事項（2026-11-21）

- 待ちの範囲の実装: スコープごとの表示（案 A）か、起動の確定の印 `ChainFinalized` までか（案 B）。B は要件 6.1 に触れないかも合わせて決める。
- 本体の檻 `a_script_reset_sends_nothing_to_kanade` の扱い: 案 A なら 1 行の改めが要る。同じウェーブの `session-module-gather` がパスを直すファイルなので、触るなら設計の段で向こうのセッションと順を合わせる。
- 待ちの記録の語（新しい語か `move_ratio_unset` を広げるか）。
- §8 の書き換えの範囲（件 5 の行 452 だけか、並べ直しの行 143 の「起動と違い右端にそろう」にも及ぶか）。
- 赤の檻の形（R1 の確かめで、位置の相だけで作るか、起動の確定まで通すか）と、表示層に足す状態の最小の形（R2）。

## 9. 設計の段の調べ（2026-11-21・`/kiro-spec-design -y`）

> 発見の形は light（今ある預かりの判定の延長・新しい外部の依存なし）。使った手引き: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、テンプレート `design.md`・`research.md`。ukadoc は MCP（`\![move]` の注記の `--base` の値）を引いた。コードはワークツリーの `74be6ccb0` で読んだ。

### 9.1 R1 の答え（コードの読みで、実行で未確認）

- 読んだ所: `crates/areka-runtime/src/frame.rs` 199〜238（取り出し → 窓寸の照合 → 再スナップ → 位置の指令の相 → 面の `move` のずれ → 起動の確定）、`crates/areka-placement/src/reset_pos.rs` 50〜135、`crates/areka-runtime/src/frame/drain_resnap.rs` 419〜567（`finalize_chain_once_with`）、`crates/areka-placement/src/chain_finalize.rs`（「先頭スコープは動かさない」・`boot_pair_decision`）、`crates/areka-placement/src/persist_save.rs` 301（記憶は下端の中央の x）、`follow/window_move.rs` 774（`track_default_char_pos`）。
- 本体と相方が最初の面を出す前の巡 t0 に並べ直しが当たると、仮の幅 `w0p`・`w1p` で `x0 = R − w0p`・`x1 = x0 − w1p` に置き、既定の位置をその値にして `ScriptPlaced` を外し、記憶へ送る。
- 両方が最初の面を出す巡 t1 に、再スナップが揃え方を保って大きさを変え（下端なら下端の中央）、既定の位置も同じ量だけ運ぶ。同じ巡の起動の確定は先頭を動かさず、相方を `JoinLeft` で `x0' − w1` に解き直す。**窓どうしの隙間は今のコードでも t1 に消える見込み**。
- **残るのは先頭の窓の位置**: 右端は `R − (w0p − w0)/2`。本当の幅が狭いとき（測り ⑸ の 868→764 なら 52px）右端から離れ、鎖全体がその分だけ左へずれる。広いときは右へはみ出し、`place_chain_in_work_area` が寄せ戻す見込み。
- **記憶**: 先頭の下端の中央の x は再スナップで変わらないので、離れた位置が記憶に書かれ、次の起動もそこに出る（起動の確定の後の 2 度目の保存も同じ値）。
- **2 度動く**: 並べ直しと起動の確定の 2 回で置かれる（要件 1.7 に反する）。
- 相方だけが未表示の巡に並べ直しが当たる形では、先頭は本当の幅で置かれ、相方の隙間は起動の確定が埋めるので残りは無い見込み。
- よって赤の檻は「再スナップ → 位置の指令の相 → 起動の確定」まで通し、先頭の右端のそろいと「起動の確定が動かさない」を見る（設計の Testing Strategy の 1）。案 0（預からない）は先頭の位置と記憶が直らないので成り立たない。

### 9.2 R2 の答え（表示しないと分かった窓の覚え方）

- 候補 1: 表示層（`PresentTarget`）に「最初の面を表示しないまま決まった理由」の欄を 1 つ足す。書く点は `apply` の `ShowSurface`・`Hide`・`ReplaceTarget` の 3 腕で、`applied` が `None` のまま `Hide`／`Ok` の縮退なら `Hidden`、`Err` なら `Failed`、表示が成り立てば `None`。
- 候補 2: 位置の相の側で `PresentCommand` を写し見て覚える。表示層の結果（成り立ったか・失敗したか）は指令だけでは分からず、第 2 の帳簿になる。
- **候補 1 を取る**。表示層が一次情報源（`target_visible` の照会と同じ考え）・欄 1 つと照会 1 本で済む・装着の前は指令がチャネルに残る（`drain_resnap.rs` 58 行の `attached` のゲート）ので、未装着の target への適用は起きない。
- 読み口は `ShownRatioSource` に既定の実装つきの `never_shown(scope) -> Option<NeverShown>` を足す。閉包・`all_shown` はそのまま通り、公開の入口の形は変わらない。

### 9.3 R3・R5・R6

- R3: SSP の測りの手順は §10。設計は枝 A（捨てる）・枝 B（位置だけ置く）を【SSP の測りの後に決める】で並べ、仮の既定を枝 B にした。
- R5: 新しい失敗の道は足さない（照会は `None` を返すだけ・判定は失敗しうる箇所を持たない）。
- R6: 実機の検体は並べ直しをトークの先頭に置く。`reset_window_pos_held` が出たかで「最初の面の前に届いた」回かを見分ける（出なければ同じ巡に届いた回）。

### 9.4 設計の決め

| 事項 | 決め | 理由 |
|---|---|---|
| 待ちの範囲（§8.1） | スコープごとの表示の決まり（案 A＋案 C）。`ChainFinalized` は使わない | 議題 2 の決め・下流 `extra-character-windows` の確定の後の窓も同じ決まりで待つ・6.1 |
| 本体の檻 | `run_move_drain_with(&wiring, &all_shown, …)` へ改める（呼び出し 1 行と `use` 1 行） | `steady_harness` は確定の後の足場なので「全部が表示した」読み口がその足場に合う。`session-module-gather` と順を合わせる |
| 記録の語（§8.1） | 新しい語 `reset_window_pos_held`・`reset_window_pos_unshown`・`move_hold_unshown` | `move_ratio_unset` は移動の対象と基準を欄に持つ測りの語で形が違う。`reset_window_pos_` で始めて grep で並べ直しの行が全部引ける |
| §8 の範囲（§8.1） | 件 5 の行（452）だけ | 143 行の「今表示している窓の大きさで解く」はこの直しの後も正しい |
| 赤の檻の形（§8.1） | 起動の確定まで通す | 9.1 のとおり窓どうしの隙間は今も消えるので、先頭の右端と「確定が動かさない」が赤の材料 |
| 新しいテストの足場 | `frame_reset_hold_test_support.rs`（偽の読み口 `SettledSource`） | C24 の約束（`frame_test_support.rs` に足さない） |

### 9.5 総合（design-synthesis）

- 一般化: 「窓の表示が決まるまで待つ」は移動と並べ直しで同じ問題で、スコープの決まり `Settle`（`Shown`／`NeverShown`／`Pending`）1 つで両方を判定する。移動の待ち（対象と基準）と並べ直しの待ち（窓のある全部のスコープ）は待つスコープの集め方だけが違う。
- 作るか借りるか: 外の仕組みは要らない。表示の事実は表示層の今の照会（`applied_ratio`）の隣に置く。
- 簡素化: 枝 A／枝 B を切り替える設定や口は作らない（測りの後に片方だけを実装）。預かった並べ直しの待ちのスコープは覚えず毎巡読み直す。起動の確定・`reset_pos.rs`・相の順は触らない。

## 10. SSP の測りの手順（要件 4.3・設計の段で行う・進行役が回す）

> 目的: SSP が「表示していない窓」への `\![move]` を、当てずに捨てるか、位置だけ動かすかを測る。結果で設計の枝 A／枝 B を決める。開発者の許し 2026-11-21（要件の話し合いの議題 3）。SSP は最初の面の前の窓を作らない（シェルを出し終えてから台本を流す）ので、(1) 一度出した窓を `\s[-1]` で隠した形と、(3) まだ一度も出していない追加のスコープ `\p[2]` の形の 2 つで近づける。

### 10.1 前提と後始末

- 排他テストの机を持ってから始め、終わったらすぐ手放す（`doc/impl-watch.md`）。ほかのウェーブのセッションに、SSP を使う旨を先に知らせる。
- 使う道具は SSP の MCP（`mcp__ssp__get_active_ghost_list`・`get_property`・`sakurascript`・`get_log`）だけ。`reload`・設定の変更・profile の書き換えはしない。
- 版: `get_property` の `baseware.version`（と `baseware.name`）を控える。ゴースト名は `get_active_ghost_list` の先頭を使う（以下 `<G>`）。
- 測る前の位置と面を控える: `currentghost.scope(0).rect`・`currentghost.scope(1).rect`・`currentghost.scope(0).surface.num`・`currentghost.scope(1).surface.num`（以下 `R0_0`・`R0_1`・`S0`・`S1`）。`currentghost.scope(2).rect` も読み、空でなければ (3) は行わない（そのゴーストはすでに `\p[2]` を出している）。
- 後始末（毎回・測りの最後）: `\0\s[<S0>]\1\s[<S1>]\e` で面を戻し、`\0\![move,--X=<R0_0.left>,--Y=<R0_0.top>,--base=global,--option=ignore-sticky-window]\1\![move,--X=<R0_1.left>,--Y=<R0_1.top>,--base=global,--option=ignore-sticky-window]\e` で位置を戻す。`--base=global` は ukadoc の `\![move]` の注記の「システム座標系」で、`--base-offset`・`--move-offset` の既定は `left.top`。戻した後に rect を読み、`R0_0`・`R0_1` と同じことを確かめる。`\![execute,resetwindowpos]` は記憶を書くので使わない。

### 10.2 測り (1): 隠した窓を動かす

1. `sakurascript`: `\1\s[-1]\e`（相方を隠す）。
2. `get_property` `currentghost.scope(1).rect` → `H1`（隠した窓の rect の答え方を控える）。
3. `sakurascript`: `\1\![move,--X=100,--Y=100,--base=primaryscreen,--base-offset=left.top,--move-offset=left.top,--option=ignore-sticky-window]\e`。
4. `get_property` `currentghost.scope(1).rect` → `H2`。
5. `sakurascript`: `\1\s[<S1>]\e`（同じ面で出し直す）。
6. `get_property` `currentghost.scope(1).rect` → `H3`。`get_log` で 1〜5 の間の警告・エラーの行を控える。
- 読み方: `H3` の左上が主モニタの `(100,100)` なら **位置だけ動かす**（枝 B）。`H3` が `R0_1` のままなら **捨てる**（枝 A）。`H2` が `H1` と違うかも控える（隠している間に位置が更新されるか）。

### 10.3 測り (2): 隠した窓を基準に動かす

1. `sakurascript`: `\1\s[-1]\e`。
2. `sakurascript`: `\0\![move,--X=0,--Y=0,--base=1,--base-offset=left.top,--move-offset=right.top,--option=ignore-sticky-window]\e`（本体の右上を、隠した相方の左上へ）。
3. `get_property` `currentghost.scope(0).rect`・`currentghost.scope(1).rect` → `B0`・`B1`。
4. 後始末へ（10.1）。
- 読み方: `B0` の右上が `B1` の左上なら、隠した窓も基準として使う（枝 B と合う）。`B0` が `R0_0` のままなら基準の窓が隠れていると移動を捨てる（枝 A と合う）。

### 10.4 測り (3): まだ出していない窓（`\p[2]`）を面より先に動かす

1. `sakurascript`: `\p[2]\![move,--X=200,--Y=200,--base=primaryscreen,--base-offset=left.top,--move-offset=left.top,--option=ignore-sticky-window]\e`。
2. `get_property` `currentghost.scope(2).rect` → `P1`。
3. `sakurascript`: `\p[2]\s[<S1>]\e`（相方の面の番号で `\p[2]` を出す）。
4. `get_property` `currentghost.scope(2).rect` → `P2`。`get_log` を控える。
5. `sakurascript`: `\p[2]\s[-1]\e` で隠す。
- 読み方: `P2` の左上が `(200,200)` なら、まだ出していない窓への移動も位置を覚えて当てる（areka の「表示しないと分かった窓」より強い形で枝 B と合う）。`P2` が SSP の既定の位置なら、出していない窓への移動は捨てる。`\p[2]` の位置が SSP の記憶に残るかもしれないので、残ったら記録に書く（消す操作はしない）。

### 10.5 記録の形

- この節の下に「10.6 結果」を足し、SSP の版・ゴースト名・日時・各手の rect の値・`get_log` の該当行・後始末の確かめを書く。結果から枝 A／枝 B を決め、設計の「MoveHold の判定の規則 2」と OPEN QUESTION 2 を改め、`doc/COMPAT_ARCHITECTURE.md` §8 の件 5 の行の第 3 列に測りの版と結果を書く。

### 10.6 結果（2026-11-21 20:07〜20:09・進行役・途中でやめた）

- 版: `SSP/2.9.34 (20261113-4; Windows NT 10.0.26300)`。ゴースト: 起動していたのは `窓際のぱすたさんDEBUG` だけ（`get_active_ghost_list`）。排他テストの机を持って測り、終わってすぐ返した。
- 測る前: `scope(0).rect`＝`scope(1).rect`＝`-2560,819,-1729,1845`（相方は本体の上にぴったり重なる）・面は `1000`／`10`・`scope(2).rect`＝`0,0,0,0`。
- **測りが混ざる理由**: このゴーストは毎秒の `OnSecondChange` で `\0\s[1000]\1\s[10]\![move,--X=0,--Y=0,--time=0,--base=0,…,--option=ignore-sticky-window]` を流し、相方を本体の上へ置き直す（script のログ #2155・#2159・#2164）。加えて、ほかの誰かが SSTP で `\![reload,shiori]` を打っていた（#2156・#2161。開発中のゴーストと読める）。
- 測り (1) の取れた値:
  - 相方を `\1\s[-1]` で隠した後の `scope(1).rect`＝`-1500,900,-669,1926`（直前に見える状態で置いた位置のまま）・`surface.num`＝`-1`。
  - 隠したまま `\1\![move,--X=100,--Y=100,--base=primaryscreen,…]` → `scope(1).rect`＝`100,100,931,1126`。**SSP は隠した窓への `\![move]` を捨てず、窓の位置を動かす**（strict のエラーの記録は 0 件）。
  - 出し直した後の値（H3）は、毎秒の置き直しが先に当たるので読めなかった（1 本の台本で「隠す・動かす・出す」をまとめても、読む前に #2164 が当たった）。
- 測り (2)（隠した窓を基準にする）と (3)（まだ出していない `\p[2]`）は行っていない。
- 後始末: 2 つの窓を `--base=global` で `-2560,819` へ戻し、両方の rect が測る前と同じであることを読んで確かめた。面は `1000`／`10` のまま。記憶を書く `\![execute,resetwindowpos]` は使っていない。
- 読み: 隠した窓の位置は SSP でも動く（枝 B「位置だけ置く」の側の材料）。出し直した後もその位置が残るかと、基準として使えるかは未測。測り直すなら、毎秒の位置の指令を流さないゴースト（例: emo2）を SSP に立て、ほかのセッションが SSTP を打たない時に行う。
