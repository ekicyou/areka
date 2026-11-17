# ギャップ分析: audit-placement-follow-residue

> 2026-11-17 `kiro-validate-gap`。測ったツリーは枝の頭 `476793b8d`（`main` `5749a4c72` の上に spec の初めのコミット 1 つ）。`main` は brief の再測定（`7f41b53e`）の後に roadmap の文書のコミット（#473）が 1 つ進んだだけで、コードの行は動いていない。この文書は分析と選択肢までで、決めは要件の討議と設計に渡す。

## 0. 要約

- 件はどれも今のツリーに在る。brief の「2026-11-17 棚卸㊴の再測定」の行は、`follow.rs` の `rect_at` の再束縛（141 → 166 行）のほかは今も合う。名前の無い番号の数も 48 ファイル・218 行と 18 ファイル・39 行で同じ。
- 鎖の寄せは「組の相手を鎖の前の窓にする」だけでは済まない。今の `keep_restored_groups` は番号の続く窓を**並びの切れ目まで**まとめる形で、鎖の前の窓との組を入れると、組が並びの中で飛び飛びになる（辺のスコープを挟む）。組の作り方そのもの（つながりの束ね方）を変える必要がある。また、記憶の矩形が隣り合う・重なる組（`window-work-area-clamp` 要件 4.1）を辺のスコープにも当てるかで、要件 1.4 と `audit-placement-core` 18.10 の表（T5・T7 の F の列は「合う」）が食い違う。
- 164 は追随の窓だけの話ではない。吸着した窓と組に入った窓は wintf が動かさず areka が書く窓なので、位置が分からないと**ドラッグした窓そのもの**が (0,0) から差の分の所へ動き、離したときにその位置が記憶に書かれる。wintf の側は位置が取れないとき既に `warn!` を 1 行出している。
- 4 回目の `AnchorChange` はコードを読む限り鳴る: シェルの切り替えの後始末（`finish_shell`）は札（`DpiSyncHold`）を見ずに `reanchor_char_windows` を呼び、その書き込みは「見送りが覆う経路」に分けられているので、札のある窓の位置が変わると `warn!` と `debug_assert!(false)`（テストのビルドでは panic）に届く。届く場面が実際に作れるかはテストで確かめる。
- 規模は 14〜17 タスクの見込みで 20 の下に入る。ただし要件 7.2 の「要件の番号」を受け持つ範囲の全部と読むと、名前の無い要件の番号は約 540 行あり、20 に近づく。

## 1. 行の引き直し（要件 10.1）

| 件 | brief の行 | 今の行 | 中身 |
|---|---|---|---|
| 鎖の寄せ | `work_area_clamp.rs` 412・489・299 | `keep_restored_groups` 412・`joined_with_prev` 489-496・`opposite` 299-305・`RestoredWindow::mark` 392・doc 404-411 | 変わらない |
| 164 | `dispatch.rs` 103／129・`mod.rs` 77・`drag_follow.rs` 291／293 | 同じ。`dispatch.rs` 93-99 に位置が取れないときの `warn!` が既にある | 変わらない |
| 164 の組む所 | 11 か所 | 11 か所（`git grep -n "DraggingState {"` から型の定義 `mod.rs:73`・関数の頭 `follow_test_support.rs:90`・`frame_dpi_drag_grab_tests.rs:68` を除いたもの） | 同じ（下の 3.1） |
| 83 | `follow_test_support.rs:418` | 418（共有は `placement_shared_test_support.rs:100`） | 変わらない |
| 91 の残り | `follow/move_shift.rs:18` | 18（根の再公開は `lib.rs:83`） | 変わらない |
| 2 回目 | `point:192`・`contained:365`・`Store` 528〜572・`resize_under_filter:280`・`window_move_output_under_filter:483`・`k:35`・`events_of:79`・`scg` | 同じ。`scg` は `follow/sticky_follow.rs:379` の 1 か所（「scg 7.3」） | 変わらない |
| 358・426 | `position_of:37`・`drag_event:45`・`drag_end_event_at:108`・`window_pos_sized:118`・`point_of:298` | 同じ。写しは `spawn_follow_pipeline_tests.rs:37`（`drag_event`）・`balloon_limit_flip_tests.rs:336`（`point_of`）・`balloon_lock_drag_tests.rs:48`（`window_pos`） | 変わらない |
| 3 回目 | `follow_drag_tests.rs:143-144`・`move_shift.rs:96` | 144・96。`move_shift.rs:217` にも名前の無い「監査の指摘 5」がある | 1 行足す |
| 4 回目 | `window_move.rs` 526〜530・130・`keyword_base.rs:47`・`follow.rs:122` | 同じ（526-530 の「本番では未結線」・130 の上流の一覧・122 の `apply_restored_placements` と「task 1.4」「task 2.4」） | 変わらない |
| 431 | `visibility.rs:138` | 138（`clamp_x_into` の式は `x.min(hi).max(lo)` で `resolver::clamp_axis`（`resolver.rs:440`）と同じ） | 変わらない |
| 432 | `work_area.rs:45`・doc 42 | 45-56・doc 42 | 変わらない |
| 491 | `balloon_limit.rs` 202・291、`reseed.rs` 347、`persist.rs` 215／245 | 同じ | 変わらない |
| 491 の再束縛 | `follow.rs:141` | **166 行**（`use self::visibility::{…, rect_at, …}`＝私有の `use`。`visibility.rs:111` の `rect_at` は `pub(super)`） | 動いた |
| 511 | `code_only:465`・`SharedFakeIo:495` | 同じ。共有は `test_support.rs:46`（`SharedFakeIo`）・`:281`／`:424`（`code_only`） | 変わらない |
| 7 回目 | `sticky_follow.rs:311` | 311 | 変わらない |
| 1,000 行 | `drag_follow.rs` 1,000 | 1,000 | 変わらない |
| `audit-emo2-boot-spine` の申し送り ⑵ | `transition_judge.rs` 45・47-50 | 45（`runtime/frame_transition_*`）・47-50（「bin crate ゆえ…`main.rs` が宣言する」）。今の宣言は `crates/areka-runtime/src/lib.rs:111` | 変わらない |

名前の無い番号（受け持つ範囲＝`crates/areka-placement/src/follow*`（`follow/` を含む）・`src/transition_*`・`crates/areka-runtime/src/placement_tests/transition_*`、60 ファイル）:

- `git grep -nE "(task|タスク) ?[0-9]+"`: 48 ファイル・218 行（spec の名前の付いた行も入った粗い数。`areka-P0-`・`audit-` などの名前が同じ行にあるのは約 14 行）。多いのは `follow/window_move.rs` 25・`follow.rs` 22・`follow/drag_follow.rs` 14・`transition_judge_verdict_tests.rs` 13・`transition_signoff_procedure_tests.rs` 11・`transition_judge.rs` 10。
- `git grep -nE "指摘 ?[0-9]+"`: 18 ファイル・39 行（`audit-placement-follow・指摘 16` の形の名前付きが 3 行、`audit の指摘 12` の形が 3 行ほか）。
- 999 行の 2 ファイルの当たり: `transition_judge.rs` 10 行・`transition_judge_tests.rs` 3 行（task と指摘の合計）。

## 2. 件ごとの今あるものと直し方

### 2.1 鎖の寄せ（要件 1）

今の形:

- 呼び手は `persist::restore_and_judge`（`persist.rs:385`）だけ。`restored_windows`（`:416`）が記憶から位置を戻したスコープだけを `RestoredWindow` に組み（スコープ昇順）、`keep_restored_groups(&windows, snapshot)` を呼ぶ。本体の呼び手は `main.rs:787`（起動）、ほかは `apply_restored_placements` 経由のテストと `reseed.rs:70`（1 スコープだけ渡すので組は無い）。
- `keep_restored_groups` は並びを前から見て、隣の窓と `joined_with_prev` が真のあいだ伸ばした**ひと続き**を 1 つの組にする。`joined_with_prev` は「番号が 1 つ違う」かつ「n の印が `left`／`right`、または記憶の矩形の `judge_adjacency` が離れていない」。
- `restore_and_judge` に入る `ScopePlacement` は resolver の出力で、`anchor` は descript の揃え方（起動の前なので台本の吸着は無い）。つまり `resolver::joins_chain(p.anchor)` は `ChainView::Saved` の見方と同じ答えになる。
- 鎖の前の窓を引く共有の口は `ChainRule::prev(ChainView::Saved, n)`（`chain_rule.rs:192`）。ただし `ChainRule` を組む口は World から（`of`・`of_ghost_windows`）と、テストだけの `from_terms`（`#[cfg(test)]`・`:154`）しか無い。`restore_and_judge` は World を持たない純関数。

食い違いの中身（18.10 の表の F の列）:

- T6 ⑴: 辺のスコープを挟んだ組 (p, n) は番号が続かないので、n の印が `left`／`right` でも窓ごとに寄せる。
- T6 ⑵: n−1 が辺のスコープで n の印が `left`／`right`（本当は p との組の印）だと、辺のスコープと n を印で 1 つの組にする。
- T8: 中のスコープが無いゴースト（0 と 2 だけ）は番号が続かないので窓ごと。
- 今のテスト `follow_work_area_clamp_restore_tests.rs` の `apart_or_non_consecutive_pairs_are_not_grouped`（125 行）は、0 と 2 で印 `right` の対を「組にしない」と固めている。これは T8 の今の振る舞いそのもので、直すと期待が変わる（スコープ 1 が在るのか・辺なのかを入力に持たないと、このテストは意味が決まらない）。

直し方の骨:

1. `restore_and_judge` から、全スコープ（記憶から戻したものだけでなく `placements` の全部）の鎖に入るかを `keep_restored_groups` へ渡す。
2. 印での組は（鎖の前の窓 p, n）。p が記憶から戻した窓でなければ（既定の位置のまま）、n は印では誰とも組にならない（起動の確定が既定の窓の隣へ付け直す）。
3. 組は「ひと続き」でなく「つながったものの束」になる（p と n のあいだに辺のスコープ k が挟まり、k が記憶の矩形で p や n とつながることがある）。今の `while` の伸ばし方をやめ、組の印を辿って束ねる形（スコープは数個なので、番号の小さい側へ付ける素朴な束ね方で足りる）に替える。和の矩形・全員が寄せられたかの判定・記録は今のまま使える。
4. `RestoredWindow::mark` の doc を「鎖の前の窓との組の覚えた印」に、`keep_restored_groups` の doc を新しい組の作り方に直す（要件 1.7）。
5. `joined_with_prev` を消す（要件 1.8 の grep で当たりが消える）。

鎖に入るかの渡し方（設計の決め (b)・下の 3.2）。

記憶の矩形での組（重なり・隣り合い）をどの対に当てるか（設計の決め (b2)・下の 3.2）。

### 2.2 C14 の `opposite`（要件 1.5）

- `opposite` を呼ぶのは `resolve_chain_with`（`work_area_clamp.rs:158`）だけで、その呼び手は `place_chain_in_work_area`＝起動の確定（本体 `drain_resnap.rs:511`）と詰め直し（`chain_realign.rs:395`）。**記憶からの組の寄せ（`keep_restored_groups`）は `opposite` を通らない**（和の矩形を `clamp_rect_to_work_area` で収めるだけで、反対の側へ回すことはしない）。要件 1.5 は「記憶から戻した窓を寄せる」要件の中にあるが、`opposite` は別の道の話（下の 4 節）。
- 520 の直し（18.10 の ⑷）で足した辺のスコープとの組は詰め直しの道だけで作られる: `chain_realign.rs:361-378` が辺のスコープを `Keep(DesktopEdge)` のスロットとして列に入れ、付けた窓に `JoinLeft`／`JoinRight` を付ける。この組は鎖の寄せの直し（2.1）と関係なく、直した後も今のまま `opposite` に届く（起動の確定は辺のスコープを列に入れないので届かない）。
- 届いたときの振る舞い: 付けた窓が基準の窓（辺のスコープ）のモニタの作業領域に横に入らないと、反対の側（辺のスコープの向こう＝画面の端の外）を試す。辺のスコープは作業領域の端に揃っているので、反対の側はほぼ必ず作業領域の外で入らず、結果は「端へ寄せる」（`Edge`・辺のスコープと重なってよい）になる。反対の側が入るのは、辺のスコープが端に揃っていない（作業領域が変わった直後など）ときだけ。
- `JoinLeftOffset` を回さない（C14・`defaultx` の隙間の組は端へ寄せる）は 18.10 の C14 の採る側どおり。
- 直し方: 振る舞いは変えず、`opposite` の doc に「`JoinLeftOffset` は回さない理由（C14）」と「辺のスコープとの組（⑷）は回す側に入るが、反対の側はほぼ画面の外なので端へ寄せる」を書く。壊れたら落ちるテストを置くなら、辺のスコープの隣の窓が入らない場合に `Edge` になることを `place_chain_in_work_area` の檻（`follow_work_area_clamp_chain_tests.rs`・456 行）で固める。

### 2.3 164（要件 2）

今の形:

- `dispatch.rs:83-103`: 持ち主の窓から `drag_origin`（`:343`）で位置を引く。取れないとき（`WindowPos` が無い・`position` が無い・`client_to_window_coords` が失敗）は `warn!`（「ドラッグを始めた窓の位置が取れない: 窓は動かさない」・`entity` と `window` 付き）を出し、`WindowDragContext.initial_window_pos` は `None`、`DraggingState.initial_window_pos` は `(0.0, 0.0)` を入れる。
- wintf の中の控え `DragState::Dragging.initial_window_pos: Point`（`state/mod.rs:84`）は `ctx.initial_window_pos.unwrap_or((0,0))` だが、`move_window` を `ctx.move_window && ctx.initial_window_pos.is_some()`（`:260`）にして、位置が無いときは wndproc が窓を動かさない。wndproc は `DraggingState` が在ればそちらの値を正本として読む（`mouse_move.rs:290-295`・`dragging_window_origin` `:398`）が、`move_window` が偽なので使われない。
- areka の側: 吸着した窓と組に入った窓は `move_window: false`（areka が唯一の書き手）。`on_char_drag`（`drag_follow.rs:62`）は `policy_mapped_position`（`:276`）で `initial_window_pos + (カーソル − 押した位置)` を出し、その位置へ**ドラッグした窓そのもの**を書き、バルーンと組の相手を付いて動かす。`on_char_drag_end`（`:147`）も同じ式で最終の位置を決め、記憶へ書く。
- したがって位置が分からないとき、吸着・組の窓は (0,0)＋差の分の所（を吸着の辺へ当てた所）へ飛び、その位置が記憶に残る。組に無い `Free` の窓は wintf が書く窓で、wintf が動かさず、areka は `WindowPos.position` を読むだけなので (0,0) は効かない。
- 起きる条件: 本番では窓に `WindowPos`・`position` が無いか、HWND の座標変換が失敗したとき。**テストでは偽の HWND で座標変換が失敗するので、この道が毎回通る**（`follow_drag_end_gate_tests.rs:200-209` の `destination()` は「(0,0)＋差」を期待値にしていて 8 か所で使う。本体の `input_events/drag_test_support.rs:242-250` の `set_initial_window_pos` は開始の後に `(0,0)` を窓の位置で上書きする。`input_events/drag_tests.rs:9`・`drag_sticky_tests.rs:55` の説明も同じ前提）。

読み手と書き手（`DraggingState.initial_window_pos`）:

- 組む所 11 か所: wintf `ecs/drag/dispatch.rs:129`・`ecs/drag/systems.rs:41`・`ecs/window_proc/mouse_move_drag_origin_tests.rs:12`・`tests/drag/dispatch_test.rs:442`・`:484`、配置 `follow_test_support.rs:91`、`areka-runtime` `frame_chain_realign_exclusion_tests.rs:248`・`frame_dpi_drag_grab_tests.rs:60`・`:185`、本体 `runtime_tests/frame_chain_finalize_persist_restart_tests.rs:66`・`runtime_tests/frame_chain_rule_tests.rs:414`。
- 読む・書き換える所: 配置 `follow/drag_follow.rs:291`・`:293`、`areka-runtime` `frame/dpi.rs:186-240`（`rescale_drag_grab`・`realign_drag_grab`＝拡大率が変わったとき掴んだ点を置き直す）、wintf `ecs/window_proc/mouse_move.rs:404-405`、本体 `input_events/drag_test_support.rs:245-249`、`areka-runtime` `frame_dpi_drag_grab_tests.rs:34-35`・`:78`・`:137-138`、wintf `tests/drag/dispatch_test.rs:117`。
- `DraggingState` の有る無しだけを見る所（形を変えても触らないが、「位置が分からないときは入れない」案では振る舞いが変わる）: `chain_realign.rs:278`（ドラッグ中の窓は詰め直さない）・`follow/work_area_clamp.rs:350`（大きくなった窓を寄せない）・`follow/move_shift.rs:139`・`:146`（面の move のずれを当てない）・wintf `ecs/drag/systems.rs:17`（ドラッグ中の次の画面の更新の予約）・`dispatch.rs:301-304`（`DragEvent.start_position`）・`frame/dpi.rs:226`。
- 公開の面: `DraggingState` は `wintf::ecs` から再公開（`ecs/mod.rs:26`）され、欄はどれも `pub`。wintf は `publish = true`、版はワークスペースの版（今は `0.1.0`・v0.2.0 は未公開）。前例は同じ `ecs/drag` の `WindowDragContext.initial_window_pos: Option<Point>`（`context.rs:19`）。

持たせ方の比べ（設計の決め (a)・下の 3.1）。

### 2.4 `follow/drag_follow.rs` の分け方（要件 3）

- 中身は 3 つの塊に分かれ、塊のあいだの呼び合いは一方向: ⑴ キャラ窓のドラッグ（1-312 行: `on_char_drag`・`on_char_drag_end`・`policy_mapped_position`） ⑵ バルーンの追随（314-502 行: `BalloonFollowTrigger`・`follow_balloon`・`guard_balloon_position`。⑴ が呼ぶ） ⑶ バルーン単独のドラッグ（503-1000 行: `on_balloon_drag`・`known_position`・`reestablish_balloon_follow`・`on_balloon_drag_end`・`retire_keyword_base_on_save`・`apply_release_limit_correction`。26-33 行の `BALLOON_DRAG_BASE_UNPINNED_TAG` を使うのは ⑶ だけ）。⑴⑵ は ⑶ の関数を 1 つも呼ばない。
- 分け方の案: ⑶ と記録の札（26-33 行）を新しい `follow/balloon_drag.rs`（約 510 行）へ移す。`drag_follow.rs` は ⑴⑵ で約 500 行。`follow.rs:34`・`:78`・`:80`・`:82` の `mod`／`use` を足し替え、外への名前（`areka_placement::follow::on_balloon_drag` ほか）は今のまま再公開する（呼び手のパスは変わらない）。
- 移すと古くなる文（名前で `drag_follow::…` を指す所）: `follow.rs:52`・`:64`・`resolver.rs:103`・`:150`・`resolver_resolve_balloon_tests.rs:126`・`spawn.rs:200`・`:229`・`windowposition.rs:16`・`diag.rs:213`・`follow/window_move.rs:168`、テストの `follow_balloon_drag_tests.rs:7`（`use super::drag_follow::BALLOON_DRAG_BASE_UNPINNED_TAG`）。`tools/`・`.ps1`・`.py` に `drag_follow` の字面は無い。
- 移すだけの段なので今のテストがそのまま通る（要件 3.2）。名前の無い番号（「指摘 3」「指摘 7」ほか）の直しは移した後に別の段で行う。

### 2.5 写し（要件 4）

| 件 | 今あるもの | 直し方と注意 |
|---|---|---|
| 83 | `follow_test_support.rs:418` と共有の `placement_shared_test_support.rs:100` は同じ判定（失敗の文だけ違う） | 追随の側を消し、使い手を共有の側へ |
| 91 の残り | `move_shift.rs:18` が `windowposition::scale_signed` を直に引く | `crate::scale_signed`（`lib.rs:83`）へ。`lib.rs:24-26` の「根を引く子」に `follow/move_shift` を足す |
| 2 回目 `point` | `follow_test_support.rs:192` と `test_support.rs:98` が同じ | 共有へ |
| 2 回目 `contained` | `:365` は素の足し算、`test_support.rs:106` は飽和の足し算（`test_support` の doc が素の足し算を戒める） | 共有へ。期待が変わる檻が無いかを使い手で確かめる |
| 2 回目 `Store`・511 `SharedFakeIo` | `:495` の `SharedFakeIo(Arc<FakePersistIo>)` と `SylphyaRig`（504-572 行）。共有の `test_support::SharedFakeIo`（`:46`・`new(store)`・`commit_count`）がある | brief の「`Store` の写し」と 511 の `SharedFakeIo` は同じ所を指す（同じ直しで 2 件が片付く） |
| 511 `code_only` | `:465` と `test_support::zorder_pair_deferred::code_only`（`:281`・`:424` の 2 か所） | 共有へ |
| 220 `k` | `follow_move_shift_tests.rs:35` | `placement_shared_test_support` の `k` へ |
| 219 `events_of` | `follow_sticky_follow_drag_tests.rs:79` | `shared_test_support::events_with` へ |
| 358・426 | 5 つの部品は `pub(super)`。`follow::test_support` は既に `pub(crate) mod`（`follow.rs:243-244`）なので、部品を `pub(crate)` にすれば crate のどこからも引ける | 写しは `spawn_follow_pipeline_tests.rs:37`・`balloon_limit_flip_tests.rs:336`・`balloon_lock_drag_tests.rs:48` |
| 記録の捕まえの手書き | `follow_transition_diag_tests.rs:280`・`follow_window_move_diag_tests.rs:483` | 共有の `capture_under_filter` へ寄せず、`diag_tests.rs` と同じ捕まえ損ねの番兵を足す（380） |
| 218 `scg` | `sticky_follow.rs:379` の「scg 7.3」 | `areka-P0-scope-chain-gap` 要件 7.3 と書く |
| 431 | `visibility.rs:138` | `resolver::clamp_axis(x, wa.left, wa.right.saturating_sub(w))`（式は同じ）。`follow_visibility_*_wiring_tests.rs` の説明（384・666 行）が `clamp_x_into` を名指すので直す |
| 432 | `work_area.rs:45-56` | `crate::work_area_px` を使う。42 行の doc の `primary_work_area` を直す。`lib.rs` の「根を引く子」に 1 行足す |
| 491 | 手組みの矩形 5 か所（`balloon_limit.rs:202`・`:291`、`reseed.rs:347`、`persist.rs:215`・`:245`） | `rect_at` を crate の中へ開くには、`follow.rs:166` の私有の `use` を `pub(crate) use` にし、**`visibility.rs:111` の `rect_at` 自体も `pub(crate)` にする**（`pub(super)` の項目は広く出し直せない）。`reseed.rs` は `wintf::ecs::Point` を `PointPx` へ写して渡す |
| 19.2 の残り（要件に無い） | `transition_diag.rs:685-700` の `live_work_area_bottom` が `Monitor` から作業領域の矩形を手で組む（`work_area_px` の写し） | `audit-placement-core` 19.2 が「Implementation Notes 4.5」として挙げているが、brief と要件に入っていない（下の 4 節） |

### 2.6 7 回目（要件 5）

- `on_char_drag_end`（`drag_follow.rs:199`）が `finish_char_drag`（`sticky_follow.rs:292`）を呼び、その中で相手ごとに `persist_entries` を 1 回ずつ呼ぶ（`:311`）。その後で `on_char_drag_end` がドラッグした窓の位置を `persist_entries` で書く（`:254`）。1 回のドラッグの終わりで N+1 回。
- 直し方の案: `finish_char_drag` が相手の記憶の束（`Vec<(PersistKey, String)>`）を返し、`on_char_drag_end` がドラッグした窓の束と合わせて 1 回で書く。ドラッグした窓に `CharWindowMarker` が無いとき（今は `warn!` で窓の保存だけ飛ばす）も、相手の束は書く。
- 束の中身は今と同じ（相手は `origin_position` で面の move のずれを引いた元の位置・形は `saved_form_anchor`）。`persist_entries` が足す隣り合いの印（T37・`dpi-realign` 6.2）は、どの呼び出しも同じ画面の並びを読むので、1 回にまとめても書く印は同じになる見込み（設計で `persist_entries` の印の決め方を読んで確かめる）。
- 檻: 書き込みの回数を数える口は共有の `test_support::SharedFakeIo::commit_count`（`:58`）がある。

### 2.7 4 回目の `AnchorChange`（要件 6）

コードを読んだ結果（設計の決め (d)・下の 3.4）:

- 本番で `PlacementRoute::AnchorChange` を書くのは `reseed::reanchor_char_windows`（`reseed.rs:278`）だけ（`diag.rs:170-172` は 407 の直しで既にそう書く）。呼び手は `areka-runtime` の `frame/switch.rs:573`（`finish_shell`）。`finish_shell` にも、その手前にも `DpiSyncHold` を見る所は無い（`areka-runtime/src` で札を読むのは `frame/dpi.rs` と `work_area_sync.rs` だけ）。
- `reanchor_char_windows` は全スコープのキャラ窓に `resize_window_to(…, PlacementRoute::AnchorChange)` を呼ぶ。`resize_window_to`（`window_move.rs:169`）は位置と寸が今と同じなら書かない（べき等の読み飛ばし）が、新しいシェルで揃え方が変わり位置が変われば `enqueue_window_set_pos` まで行く。
- `enqueue_window_set_pos`（`:631`）は `deferral_covers_route(Some(AnchorChange))` が真（`:530`）で札のある窓なら `warn!`（「整合待ちの札がある窓へ窓書込が到達した」）と `debug_assert!(false, "DpiSyncHold …")` を出す。テストのビルドでは panic、配布のビルドでは `warn!` が残って書き込みは通る。
- 札は窓の拡大率とモニタの表が合わないあいだ最大 30 フレーム付く（`dpi_sync.rs:57`・`:221-251`）。その間にシェルの切り替えが終わり、揃え方が変わる窓があれば鳴る、とコードからは読める。本当にその並びが起きるか（切り替えの段と拡大率の段の順・札が付いている間に切り替えが終わりうるか）はテストで確かめる（要件 6.1）。最小の再現は配置の crate の中で作れる: キャラ窓に `DpiSyncHold` と `WindowHandle` を付け、`Anchored` を `Top` にして `reanchor_char_windows` を呼ぶ（`follow_window_move_hold_watch_tests.rs` の `held_world`・`#[should_panic(expected = "DpiSyncHold")]` の形がそのまま使える）。
- 文の直し: `window_move.rs:526-530`（「本番では未結線・現に到達しない」）と `:130` の上流の一覧に `reanchor_char_windows` を足す。`follow/keyword_base.rs:47` の文（「`AnchorChange` だけは現在の表示寸をそのまま渡す」）は今の事実と食い違わない（書き手の名を足すかどうかだけ）。`follow_window_move_hold_watch_tests.rs:77` の二重記帳の表 `EXPECTED` も、分け方を変えるなら直す。

### 2.8 古い文と名前の無い番号（要件 7）

- `follow.rs:122` の `persist::apply_restored_placements`（本番は `restore_and_judge`）と「task 1.4」「task 2.4」。
- `transition_judge.rs:45`（`runtime/frame_transition_*` → `crates/areka-runtime/src/frame_transition_*`）・`:47-50`（今は `areka-runtime` の `lib.rs:111` が `#[cfg(test)] mod placement_tests;` で宣言する。`areka-runtime` は lib の crate なので「bin crate ゆえ `pub` でも dead_code 免除されず」の理由の文も書き直しが要る）。999 行なので行を増やさない。
- 記憶の帳面の名は `follow_drag_tests.rs:144` の 1 つ。ほかに `[[4.6 の教訓]]`・`[[2.2 の教訓]]`・`[[5.2 の教訓＝空虚性 6 例目]]`・`[[6.1 → 6.2 の申し送り]]` の形の二重角括弧が 16 行ある（`follow_visibility_balloon_wiring_tests.rs`・`follow_visibility_char_wiring_tests.rs`・`follow/drag_follow.rs:435`・`follow/visibility.rs:248`）。これは帳面の名ではなく、名前の無い古い spec の task 番号を括弧で包んだもので、要件 7.4 の 2 つの grep（`task ?[0-9]`・`指摘 ?[0-9]`）に当たらない（下の 4 節）。
- 名前の無い要件の番号: 受け持つ範囲で `(要件|Req) ?[0-9]+\.[0-9]+` は 577 行、spec らしい名前が同じ行に無いものが約 540 行（粗い数）。ほかに「（4.2/4.3/4.7）」のような素の番号も多い。要件 7.2 の「要件の番号」をこの全部と読むか、挙げた所（`drag_follow.rs:43`・`:133` ほか）だけと読むかで規模が大きく変わる（下の 3.6）。

## 3. 設計の決め（選択肢）

### 3.1 (a) 164 の「位置が分からない」の持たせ方

| 案 | 中身 | 良い所 | 悪い所 |
|---|---|---|---|
| A1 `initial_window_pos: Option<(f32, f32)>` | dispatch は位置が取れないとき `None` を入れる。`policy_mapped_position` は `None` なら位置を出さず（`on_char_drag` は書かずに戻る＝窓・バルーン・組の相手は今の所のまま）、記録を残す。`realign_drag_grab` は `None` なら置き直さない。wndproc は `None` なら控えの値（`move_window` は偽）へ | 前例 `WindowDragContext.initial_window_pos: Option<Point>` と同じ形。`DraggingState` の「在る＝ドラッグ中」の意味を保つので、在る無しを見る 7 か所は変わらない。型が `(0,0)` の取り違えを許さない | 公開の欄の型が変わる（壊れる変更）。11 か所の組む所と読む所を全部直す。偽の HWND のテストが通る道が変わるので、`follow_drag_end_gate_tests.rs` の `destination()`（8 か所）の期待を作り直す（開始の後に位置を入れる手伝いを足すか、「動かない」を期待にする） |
| A2 位置が分からないときは `DraggingState` を入れない | 型は変えない | 公開の型は変わらない | 「在る＝ドラッグ中」の意味が崩れる: 詰め直しがドラッグ中の窓を動かす・大きくなった窓を寄せる・面の move のずれを当てる・次の画面の更新の予約が立たない・`DragEvent.start_position` が今のカーソルになる。本体の `drag_test_support.rs:248` の `expect`・`drag_tests.rs:155` の `assert!(… is_some())` が落ちる。`on_char_drag_end` の「`DraggingState` が無いときは `WindowPos.position` で保存する」道（多窓の穴埋め）と見分けがつかない |
| A3 最初の移動の知らせで位置を取り直す | dispatch が `DragEvent` のたびに `drag_origin` を引き直し、取れたら始めの位置と押した位置を置き直す | 位置が後から取れる場合は追随が始まる | 取れるまでの間を表す形がやはり要る（A1 か A2 の上に乗る）。取れない理由（`WindowPos` が無い・座標変換の失敗）は続くことが多い。押した位置の置き直しを誤るとカーソルと窓がずれる。wintf の `DragAccumulator` の流れに手が入る |

- 推す案: **A1**。理由は前例と同じ形・意味を崩さない・直しが型で漏れなく見つかる。離したとき（`on_char_drag_end`）に位置を出せないときは、今の「`DraggingState` が無いとき」と同じく `WindowPos.position`（動いていない今の位置）で保存するか、保存を飛ばすかを設計で決める（同じ位置を書き直すだけなので、どちらでも記憶は壊れない）。
- 公開の段への影響（要件 2.6）: `wintf::ecs::DraggingState` の公開の欄の型が変わる。wintf は 0.x で、ワークスペースの版は `0.1.0`、まだ出していない v0.2.0 で出る（0.x の版上げは壊れる変更を許す）。`tools/crates-io.ps1` の包む確かめ（全体テストの段）は型の変更に関係しない。設計に「v0.2.0 の wintf で `DraggingState.initial_window_pos` が `Option` になる」を書き、公開の記録（`doc/crates-io-publish.md` や版の告知）に載せるかを決める。
- 記録（要件 2.3）: wintf は既に `warn!`（`entity`・`window`・理由）を出す。追随の側は `policy_mapped_position` の `None` で `debug!`（どのスコープの窓か）を足せば、1 回のドラッグで `warn!` 1 行と追随の `debug!` が残る。`DragEvent` のたびに出ると多いので、出す回数（始めだけか毎回か）を設計で決める。
- 追随の側でテストが要るのは `on_char_drag`（位置が分からないとき、窓・バルーン・組の相手が動かず記録が残る）と `on_char_drag_end`（記憶に (0,0) 由来の位置を書かない）。wintf の側は `tests/drag/dispatch_test.rs` に「位置が取れないとき `None` が入る」。

### 3.2 (b) 鎖に入るかの渡し方と、記憶の矩形の組

渡し方:

| 案 | 中身 | 良い所 | 悪い所 |
|---|---|---|---|
| B1 `ChainRule` を `restore_and_judge` で組んで渡す | `chain_rule.rs` に揃え方の列から組む口（`from_terms` をテストの外へ開く、か `from_anchors(&[(usize, Anchor)])` を足す）を作り、`keep_restored_groups(&windows, &rule, snapshot)` が `rule.prev(ChainView::Saved, n)` を引く | 鎖の前の窓を引く式が 1 か所（`ChainRule::prev`）のまま。18.10「これで終わる理由」2 の形（鎖の見方を組む所の列）に `persist.rs` が 1 つ増えるだけ | `chain_rule.rs`（`audit-placement-core` の持ち物・規則の外の口を足すだけ）に触る。brief と roadmap の C22 の触るファイルに `chain_rule.rs` が無い（同じウェーブに触る相手はいない）。`ChainTerms` は `default_x` を持つが、記憶の戻しでは使わないので `None` を入れることになる |
| B2 `RestoredWindow` に「鎖の前の窓の番号」を持たせる | `restored_windows` が全スコープの `ScopePlacement::anchor` から鎖の前の窓を引いて入れる | `keep_restored_groups` の入力が小さい | 引く所で B1 と同じ `ChainRule` が要る。自前で「下の番号で `joins_chain` の一番近いもの」を探すと、鎖の式の写しを追随の側に新しく持つ（brief の Constraints と要件 1.8 に反する） |
| B3 `RestoredWindow` に `joins_chain: bool` を持たせ、記憶から戻していないスコープも並びに入れる | 並びは全スコープ、記憶の無いものは寄せの対象外の印つき | 型は小さい | 鎖の前の窓の探し方を `keep_restored_groups` の中に書く＝B2 と同じく写しになる |

- 推す案: **B1**（`ChainRule` に揃え方の列から組む口を 1 つ足す）。写しを持たずに済む形はこれだけ。

記憶の矩形での組（`window-work-area-clamp` 要件 4.1「記憶の位置と今の窓の大きさから隣り合っていた、または重なっていたと分かる組」）をどの対に当てるか:

| 案 | 中身 | 18.10 の表との関係 |
|---|---|---|
| R1 印は鎖の前の窓との組、矩形は今と同じ番号の続く対 (n−1, n) | 印の読み違い（T6 ⑴⑵・T8）だけを直す | T5・T7（辺のスコープも番号が続けば記憶の矩形で組）・T29（重なりは組）の F の列「合う」をそのまま保つ。T6 ⑵ で辺のスコープと n の記憶の矩形が本当に隣り合っていれば、それは幾何どおりの組として残る |
| R2 印も矩形も鎖の前の窓との組だけ・辺のスコープは誰の相手にもしない | 要件 1.4 の字面どおり | T5・T7 の F の列が「合う」から変わる（辺のスコープと鎖の窓が重なって外へ出ていた組が、窓ごとに寄せられる）。完了した `window-work-area-clamp` 要件 4.1 の組の読みを狭める＝開発者の裁定に当たる（要件 8.2） |
| R3 印は鎖の前の窓との組、矩形は記憶から戻した窓の並びで隣り合う対（番号の差を問わない） | T8 で 0 と 2 の矩形が重なっていれば組になる | R1 より 4.1 の字面に近い。スコープ 1 が在って記憶が無い（既定の位置）とき、0 と 2 が幾何で組になる |

- 推す案: **R1**。要件 1 の直しの目的（印の読みを鎖の決まりに合わせる）を満たし、18.10 が「合う」とした行を動かさない。R2 を採るなら要件 1.4 は今のままで、`window-work-area-clamp` 4.1 の読みを変える裁定が要る。要件 1.4 は R1 では「辺のスコープを**印での**組の相手にしない」と書き直すことになる（下の 4 節）。
- 組の束ね方は、どの案でも「ひと続き」から「つながったものの束」へ変える（2.1 の 3）。

### 3.3 (c) C14 の `opposite`

- 2.2 のとおり、鎖の寄せの直しの後も、辺のスコープとの組（520）は詰め直しの道で `opposite` に届く。届いても反対の側はほぼ画面の外なので、端へ寄せる結果になる。
- 案: C1 振る舞いを変えず doc を書き、辺のスコープとの組が入らないときに端へ寄せることを檻で固める（推す）。C2 辺のスコープとの組は反対の側へ回さない（`Keep(DesktopEdge)` が前の窓のときは回さない枝を足す）＝結果はほぼ同じで枝が 1 つ増えるだけ。
- 要件 1.5 は「記憶から戻した窓を寄せる」要件の中にあるが、`opposite` を通るのは起動の確定と詰め直しの道（下の 4 節）。

### 3.4 (d) 4 回目の `AnchorChange`

コードを読む限り鳴る（2.7）。届く並びが本番で作れるかはテストで確かめる。鳴ると決まったときの直し方:

| 案 | 中身 | 良い所 | 悪い所 |
|---|---|---|---|
| D1 `AnchorChange` を「見送らないことが正しい経路」へ移す | `deferral_covers_route` の腕を動かし、doc と `EXPECTED` の表を直す | 1 語の分け方を変えるだけ。シェルの切り替えは利用者か台本の操作で、`MoveCue` と同じ扱いになる | 結線していない見本の `anchor_changed_system` を将来つないだとき、見送りの漏れを見張れない（その時に分け直すと doc に書く） |
| D2 `reanchor_char_windows` に専用の経路語を足す | 例 `ShellReanchor`（見送らない側）。`AnchorChange` は見本の語として鳴る側に残す | 語ごとの意味が 1 つになる | 経路語は網羅の `match` が 4 か所（`deferral_covers_route`・`is_system_reanchor`・`as_str`・`diag.rs` の表）と檻の表 2 つに及び、移り変わりの判定器（`placement_tests/transition_*`）が語を読むなら判定にも及ぶ。規模が大きい |
| D3 札のある窓は置き直しを見送る | `reanchor_char_windows` が札を見て飛ばし、`debug!` を残す | 札の「札のある窓への窓書込は 0」の不変条件をそのまま守る | 見送った窓へ新しい揃え方が後で当たる保証が要る（札が外れるフレームの拡大率の段の書き込みが今の `Anchored` で射影するか、寸が変わらないと書かないか）。調べが要る |

- 推す案: D1（ただしテストで鳴ることを確かめた後）。D3 は「見送った窓に後で新しい揃え方が当たるか」を確かめられれば候補になる。鳴らないと分かれば文の直しだけ（要件 6 の仮定どおり）。

### 3.5 (e) `drag_follow.rs` の分け方

- 2.4 の案（バルーン単独のドラッグの塊を `follow/balloon_drag.rs` へ）。ほかの切り方（キャラ窓のドラッグを外へ出す）は、`follow_balloon` の呼び手が残る側と出す側の両方にまたがり、外への名前の付け替えも増えるので推さない。
- 守ること: 移すだけの段（関数の中身・記録の文・順を変えない）、外への名前は `follow.rs` から今のまま、`file_length_guard_test.rs` の例外表に足さない。`follow/window_move.rs`（1,234 行・例外表に載る）は割らない（同じウェーブの約束 ⑵）。

### 3.6 (f) タスクの数

見立て（どれも 1 タスク・テストを含む）:

1. `drag_follow.rs` を分ける（移すだけ・名指しの文を直す）
2. 164 wintf の側（型・dispatch・wndproc・`systems.rs`・wintf のテスト）
3. 164 読み手の側（`drag_follow.rs`・`frame/dpi.rs`・組む所のテスト・`drag_test_support.rs`）と追随の檻・`follow_drag_end_gate_tests.rs` の期待の作り直し
4. 鎖の寄せ（`ChainRule` の口・`restore_and_judge`・`keep_restored_groups` の束ね方・doc・T6／T8 の檻・今の檻の直し）
5. C14 の doc と檻
6. 7 回目（記憶へ 1 回で書く・回数の檻）
7. 4 回目（鳴るかの檻→直し→文）
8. 本番の式の差し替え（431・432・491・`lib.rs` の文）
9. テストの支えの写し（83・2 回目・511・220・219・358・426）
10. 記録の捕まえの番兵（2 か所）
11. 古い文（`scg`・帳面の名・`follow.rs:122`・`transition_judge.rs:45-50`・`window_move.rs`・`keyword_base.rs`）
12〜13. 名前の無い task 番号と指摘番号（配置の crate と `areka-runtime` の 2 つ）
14. 仕分けの grep・行の数・全体テスト

- 14 前後で、ばらつきを入れて 14〜17。20 の下に入る（未決 2 の「164 を切るか」は切らずに収まる見込み）。
- 20 に近づく条件: 要件 7.2 を名前の無い要件の番号の全部（約 540 行・60 ファイル）と読む（+2〜4）、`[[…]]` の 16 行を入れる（+0〜1）、要件に無い `transition_diag.rs` の写しを入れる（+0・8 に入る）、4 回目で D2 を採る（+1〜2）。

## 4. 要件とコードの食い違い

1. **要件 1.4 と 18.10 の表**: 要件 1.4「左右の辺のスコープを寄せの組の相手にしない」は、18.10 の表の T5・T7 の F の列（「番号が続けば記憶の矩形で組にする（重なりも組・`work-area-clamp` 4.1）」・扱い「合う」）と食い違う。今のコードも T5・T7 は表のとおりで、食い違いは印の読み（T6・T8）だけ。要件 1.4 を字面どおりにすると、完了した `window-work-area-clamp` 要件 4.1 の組の読みを狭める（3.2 の R2）。
2. **要件 1.5 の置き場**: `opposite` は記憶からの組の寄せ（`keep_restored_groups`）を通らない。通るのは起動の確定と詰め直しの `place_chain_in_work_area`。要件 1.5 の「寄せた組が作業領域に入らない」は、要件 1 の場面（記憶から戻した窓）では起きない形。要件の文を「起動の確定と詰め直しで、組が作業領域に入らない」に移すのが合う。
3. **要件 2.2 の仮定の前提**: 「ドラッグした窓自身の扱いは wintf の今の守り（位置が無いときは窓を動かさない）のまま」は、組に無い `Free` の窓にしか当たらない。吸着した窓と組に入った窓は areka が書く窓で、今はドラッグした窓そのものが (0,0) を基準に動き、離したときにその位置が記憶に書かれる（2.3）。要件 2 は追随の窓だけでなく、ドラッグした窓（areka が書く窓）と記憶への書き込みも対象に含める必要がある。
4. **要件 2.3 の今ある分**: wintf は位置が取れないとき、もう `warn!`（どの entity・どの窓・理由）を出している（`dispatch.rs:93-99`）。要件 2.3 の「記録に残す」は一部が今ある。
5. **brief の「起きる条件」**: 「窓に `WindowPos`・配置が無いときだけ起きる」とあるが、HWND の座標変換（`client_to_window_coords`）が失敗したときも起きる。テストの偽の HWND はこの道を毎回通っていて、`follow_drag_end_gate_tests.rs` の期待値と本体の `drag_test_support.rs` はそれに寄りかかっている。
6. **要件 4 に無い写し**: `audit-placement-core` 19.2 が追随の持ち物として挙げた「Implementation Notes 4.5: `transition_diag.rs` の `live_work_area_bottom` の作業領域の写し」（今は `transition_diag.rs:685-700`）が、brief にも要件 4.1 にも無い。432 と同じ形（`work_area_px` か `MonitorSnapshot::from_monitors` を使う）。
7. **要件 7.4 の grep が拾わない番号**: `[[4.6 の教訓]]` の形の二重角括弧 16 行（名前の無い古い spec の task 番号）は、要件 7.4 の 2 つの grep に当たらない。要件 7.3 は帳面の名 1 つ（`follow_drag_tests.rs:144`）だけを挙げる。
8. **要件 7.2 の「要件の番号」の幅**: 名前の無い要件の番号は受け持つ範囲で約 540 行（粗い数）。7.4 の確かめの grep は task と指摘だけなので、7.2 の要件の番号は直したかを確かめる手段が無い。どこまでを受け持つかを決める必要がある。
9. **491 の開け方**: brief の「`follow.rs:141` の `rect_at` の再公開を `pub(crate)` にし」は、今は 166 行の私有の `use` で、`visibility.rs:111` の `rect_at` 自体も `pub(crate)` にしないと開けない。
10. **2 回目の `Store` と 511**: brief は 2 件に分けて書くが、`follow_test_support.rs:495` の `SharedFakeIo` と `SylphyaRig` の同じ所を指す。
11. **要件 9.1 と例外表のファイル**: 要件 9.1「どのファイルも上限を越えさせない」に対し、`follow/window_move.rs`（1,234 行）は既に越えていて例外表に載る。番人は例外表のファイルの行を数えないので、4 回目の文の直しで行が増えても赤にはならない。行を増やさない直しにとどめるかを決めておく。

## 5. 要件と今あるものの突き合わせ

| 要件 | 今あるもの | 足りないもの・印 |
|---|---|---|
| 1.1〜1.3・1.9 | `ChainRule::prev`・`joins_chain`・`ScopePlacement::anchor`・`persist_restore_group_tests.rs`（191 行） | 記憶の戻しで `ChainRule` を組む口（Missing）・つながりで束ねる組の作り方（Missing）・T6／T8 の檻（Missing） |
| 1.4 | 今は番号の続く対で矩形の組を作る | 辺のスコープの扱い（Constraint: 18.10 T5・T7 と `work-area-clamp` 4.1） |
| 1.5 | `opposite` は詰め直し・起動の確定の道 | doc（Missing）・要件の置き場（Constraint） |
| 1.6〜1.8 | 起動の確定は `chain_rule::boot_chain` | doc の直し・`joined_with_prev` を消す |
| 2.1〜2.7 | dispatch の `warn!`・wintf の控えの守り・`WindowDragContext` の前例 | 型（Missing）・11 か所と読み手（Missing）・追随の記録（Missing）・偽の HWND に寄りかかった期待の作り直し（Unknown: 規模） |
| 3.1〜3.3 | 塊の境は一方向 | 新しいファイル 1 つ・名指しの文 10 か所ほど |
| 4.1〜4.6 | 共有の部品はどれも在る | `rect_at`・`follow::test_support` の部品の開け方・`transition_diag.rs` の写し（要件に無い） |
| 5.1〜5.3 | `SharedFakeIo::commit_count` | `finish_char_drag` の戻りの形（Missing） |
| 6.1〜6.4 | 札の見張り・`held_world` の檻の形 | 鳴るかの檻（Missing）・直し方（Unknown: テストしだい） |
| 7.1〜7.4 | — | 名前の無い番号の幅（Unknown）・`[[…]]` の扱い（Unknown） |
| 8・9・10 | 番人・例外表・`tools/test-all.ps1` | — |

## 6. 規模と危うさ

- 規模: **M**（14〜17 タスク）。今ある部品を使う直しが大半で、新しい形は鎖の寄せの束ね方と 164 の型だけ。
- 危うさ: **中**。⑴ 164 は crates.io へ出す wintf の公開の型を変え、組む所・読む所が 4 つの crate にまたがる。偽の HWND に寄りかかったテストの期待を作り直す量が読み切れていない。⑵ 鎖の寄せは完了した spec の要件（`window-work-area-clamp` 4.1）の読みに触れる案がある。⑶ 4 回目は直し方がテストの結果しだい。どれも決定論のテストで赤を先に作れる。

## 7. 調べが要ること（設計の段へ）

1. 7 回目: `persist_entries` が束から隣り合いの印を決める形を読み、N+1 回を 1 回にまとめても書く印が変わらないことを確かめる。
2. 4 回目: シェルの切り替えの後始末と拡大率の段の順で、札の付いた窓に `reanchor_char_windows` が届く並びをテストで作れるか。D3 を採るなら、見送った窓へ新しい揃え方が後で当たるか（札が外れるフレームの書き込みが今の `Anchored` で射影するか、寸が変わらないと書かないか）。
3. 164: `follow_drag_end_gate_tests.rs` と本体の `input_events/drag_*` のテストで、偽の HWND の道に寄りかかる期待がいくつあるか（`destination()` の 8 か所のほか）。開始の後に位置を入れる手伝いを配置の側にも置くか。
4. 鎖の寄せ: R1 を採るとき、T6 ⑵（辺のスコープと n の記憶の矩形が隣り合う）を幾何どおりの組として残してよいかを 18.10 の表で確かめる。
5. 移り変わりの判定器（`placement_tests/transition_*`）が経路語の綴り（`"AnchorChange"`）を読むか（4 回目で D2 を採るときだけ）。

## 7.1 要件の討議から設計の段へ回すこと（2026-11-17）

1. 164 の持たせ方（3.1 の A1〜A3・推すのは A1）と、離したときに位置が出せないときの保存（今の位置で書くか、飛ばすか）。追随の側の記録の水準と回数（始めだけか、毎回か）。
2. 鎖に入るかの渡し方（3.2 の B1〜B3・推すのは B1）。B1 は `chain_rule.rs` に揃え方の列から組む口を 1 つ足す＝規則は変えない。`chain_rule.rs` は brief と roadmap の C22 の触るファイルに無いが、同じウェーブに触る相手はいない。設計で触るファイルに足す。
3. 組の束ね方（「ひと続き」から「つながったものの束」へ・2.1 の 3）と、今の檻 `apart_or_non_consecutive_pairs_are_not_grouped` の期待の作り直し。
4. C14 の檻の置き場（3.3 の C1）。
5. 4 回目の直し方（3.4 の D1〜D3）。鳴るかの檻を先に作る。
6. `drag_follow.rs` の分け方（3.5）。
7. `rect_at` の開け方（`visibility.rs:111` も `pub(crate)` に・4 節 9）。
8. 7 回目の束の印（7 節 1）。

## 8. 次の段

- 要件の討議（`kiro-requirements-discussion`）で、下の議題を決める。決まったら `/kiro-design audit-placement-follow-residue`。
- 議題の材料: 3.1（164 の持たせ方・A1 を推す）・3.2（鎖の渡し方 B1 と矩形の組 R1 を推す・要件 1.4 の書き直し）・3.3（C14・C1 を推す・要件 1.5 の置き場）・3.4（4 回目・テストの後に D1 を推す）・3.6 と 4 節 8（要件 7.2 の幅）・4 節 6（`transition_diag.rs` の写しを入れるか）・4 節 7（`[[…]]` の 16 行を入れるか）・未決 1（audit のかけ直し）・未決 5（位置が分からないドラッグの見え方＝A1 なら「その回は動かない」）。
