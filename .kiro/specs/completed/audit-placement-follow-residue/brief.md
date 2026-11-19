# Brief: audit-placement-follow-residue

> 2026-11-15 `/kiro-discovery` で起票（`audit-placement-core` のタスク 8.2 の完了の申し送りから）。開発者の承認は同じ日（「推奨どおりで」）。`audit-placement-core` は、かけ直しの 1〜9 回目と鎖の規則の総括（同 spec の `research.md` 18.1〜18.10）で、追随の持ち物（`audit-placement-follow` の範囲）の指摘を直さずに申し送りとして残した。`audit-placement-follow` は 2026-11-14 に完了していて受け手が無い（同 `research.md` 19.2）。この spec がその受け手になる。方針は見直しの 33 本と同じ＝指摘は全部直す（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。行の番号は起票のとき（枝 `claude/kiro-start-audit-placement-core-61b071` の `8f1a8394f`＝`audit-placement-core` の着地の前）の実測＝着手のときに `main` で測り直す。

## Problem

`audit-placement-core` の audit のかけ直しは、追随の持ち物の中にも指摘を見つけた。けれども追随の持ち物は `audit-placement-follow` の範囲なので、`audit-placement-core` は 1 行も触らずに申し送りへ回した。`audit-placement-follow` はもう完了しているので、このままでは直す spec が無い。

中身は 4 つに分かれる。

- **鎖の決まりとのずれ**: 記憶から戻した窓を作業領域へ寄せるとき、追随の `keep_restored_groups` は組を「番号が続く対 (n−1, n)」で作る。鎖の決まり（`crates/areka-placement/src/chain_rule.rs`・`audit-placement-core` の `research.md` 18.10 で採った仕様）では、組は「descript の鎖の前の窓との組」。辺のスコープを挟む組や、中のスコープが無いゴーストで食い違う。最後の位置は起動の確定が決まりどおりに直すので、ずれるのは寄せの途中の組だけ。
- **ドラッグの始めの位置が分からないとき（164）**: wintf がドラッグを始めた窓の位置を取れないと、`DraggingState.initial_window_pos` に (0,0) が入る。追随の `policy_mapped_position` はそれを基準に窓を動かしうる。
- **写しと古い文**: 共有の部品があるのに追随の側で手で写したもの、消えた名前を指す説明、名前の無い番号。
- **1,000 行**: `follow/drag_follow.rs` がちょうど 1,000 行で、1 行も足せない（164 の直しはここに触る）。

## Current State

受け持つ件。どれも `audit-placement-core` の `research.md` の件の番号をそのまま使う。

### 鎖の決まりとのずれ（18.10 の表の F の列・T6・T8・C14・C16）

- `crates/areka-placement/src/follow/work_area_clamp.rs:488-496` の `joined_with_prev` が、組を `prev.scope + 1 == cur.scope` と、印 `left`／`right` か記憶の矩形の `judge_adjacency` で判じる。鎖の決まり（`chain_rule.rs` の `ChainRule`）を使わない。食い違う所は 2 つ: ⑴ 辺のスコープを挟んだ組 (p, n) と、中のスコープが無い組は番号が続かないので、印が `left`／`right` でも窓ごとに寄せる ⑵ n−1 が辺のスコープで n の印が `left`／`right`（本当は本体との組の印）だと、辺のスコープと n を 1 つの組にする。直し方の案（18.10）: 記憶の矩形と一緒に「鎖に入るか」（`ScopePlacement::anchor` と `resolver::joins_chain`）を渡し、組の相手を鎖の前の窓にする。呼ぶ共有の関数（`judge_adjacency`・`resolve_chain`）は直す所が無い。
- 同じファイルの doc: `RestoredWindow::mark`（392 行「組 (scope−1, scope) の覚えた印。」）と `keep_restored_groups`（404-411 行「組は隣り合う番号の対 (n−1, n) が…」）。
- 同じファイルの `opposite`（298-305 行）: `JoinLeftOffset` を反対の側へ回さない（`audit-placement-core` の 7 回目の決めのまま＝C14）。`audit-placement-core` の 520 の直しで足した辺のスコープとの組は `JoinLeft`／`JoinRight` なので、今の `opposite` のまま反対の側へ回る。この振る舞いでよいかを鎖の決まりに照らして確かめ、doc に書く。

### ドラッグの始めの位置（164・`audit-wintf-input` の申し送り件 1・`audit-placement-core` の裁定で「C20 の後に起票」）

- `crates/wintf/src/ecs/drag/dispatch.rs:103` が `origin.unwrap_or(Point { x: 0, y: 0 })` で、位置が取れないときに (0,0) を入れ、`:129` で `DraggingState` を組む。型は `crates/wintf/src/ecs/drag/mod.rs:73`（`initial_window_pos: (f32, f32)`）。追随の `follow/drag_follow.rs:276` の `policy_mapped_position` が `:291`・`:293` でそれを基準に使う。
- `DraggingState` を組む所は今 11 か所（`git grep -n "DraggingState {" -- crates` から型の定義と関数の頭を除いたもの・裁定のときは 9 か所）: wintf の `ecs/drag/dispatch.rs:129`・`ecs/drag/systems.rs:41`・`ecs/window_proc/mouse_move_drag_origin_tests.rs:12`・`tests/drag/dispatch_test.rs:442`・`:484`、配置の `follow_test_support.rs:91`、本体の `runtime/frame_chain_finalize_persist_restart_tests.rs:66`・`runtime/frame_chain_realign_exclusion_tests.rs:246`・`runtime/frame_chain_rule_tests.rs:413`・`runtime/frame_dpi_drag_grab_tests.rs:60`・`:185`。
- 今の振る舞いは `audit-wintf-input` の前と同じで、窓に `WindowPos`・配置が無いときだけ起きる。型の形（「位置が分からない」を持たせる）と、組む所の直しと、`drag_follow.rs` の分け方をあわせて決める。

### 写し・古い文・名前の無い番号（`audit-placement-core` の 1〜9 回目の申し送り）

- 83: `follow_test_support.rs:418` の `assert_no_event` が、共有の `placement_shared_test_support.rs:100` の写し。追随の側を消して共有の側を使う。
- 91 の残り: `follow/move_shift.rs:18` が根の `scale_signed` でなく `windowposition::scale_signed` を直に引く（根のものを使えば `lib.rs` の規約の文と合う）。
- 2 回目: `follow_test_support.rs:192` の `point`・`:365` の `contained`（飽和加算でなく、`test_support.rs` が戒める形）と、`Store` の写し（528〜572 行あたり）。`follow_transition_diag_tests.rs:280`（`resize_under_filter`）・`follow_window_move_diag_tests.rs:483`（`window_move_output_under_filter`）の記録の捕まえの手書きの写しは、`capture_under_filter` に寄せるのでなく（`env-filter` は wintf だけ・380）、`diag_tests.rs` と同じく番兵を足す（3 回目の直し）。`follow_move_shift_tests.rs:35` の `k` の写し（220）・`follow_sticky_follow_drag_tests.rs:79` の `events_of` の写し（219）・`follow/sticky_follow.rs` の略称 `scg`（218・379 行ほか）。
- 358・426: `follow_test_support.rs` の `position_of`（37 行）・`drag_event`（45 行）・`drag_end_event_at`（108 行）・`window_pos_sized`（118 行）・`point_of`（298 行）を `pub(crate)` に開き、`spawn_follow_pipeline_tests.rs` の写し（`drag_event` 37 行ほか）と `balloon_limit_flip_tests.rs`（`point_of` 192 行ほか）・`balloon_lock_drag_tests.rs` の写しを消す。
- 3 回目: `follow_drag_tests.rs:143-144` の開発者の記憶の帳面の名（`[[test-only-decision-branches-not-proven-wiring]]`・357 と同じ形）・`follow/move_shift.rs:96` の名前の無い「監査の指摘 1」。
- 4 回目: `follow/window_move.rs:525-528` が `PlacementRoute::AnchorChange` を「本番では未結線・現に到達しない」と書くが、`reseed::reanchor_char_windows`（シェルの切り替えのたび）が書く。`DpiSyncHold` を持つ窓のあいだにシェルを切り替えると、正しい書き込みで見張りが鳴りうる＝文だけでなく振る舞いも確かめる。あわせて `window_move.rs:128-131`（`resize_window_to` の上流の一覧に `reanchor_char_windows` が無い）・`follow/keyword_base.rs:47`（`AnchorChange` の扱いの文）。`follow.rs:105` が `persist::apply_restored_placements` を名指す（本番は `restore_and_judge`）・同じ行の名前の無い「task 1.4」「task 2.4」。
- 431: `follow/visibility.rs:138` の `clamp_x_into` が resolver の `clamp_axis` の写し。消して `resolver::clamp_axis(x, wa.left, wa.right.saturating_sub(w))` を呼ぶ。
- 432: `follow/work_area.rs:45` の `MonitorSnapshot::from_monitors` が根の `work_area_px` の写しで、doc（42 行）が消えた `mod.rs` の `primary_work_area` を名指す。`crate::work_area_px` を使い、doc を直し、`lib.rs` の「根を引く子」の文に 1 行足す。
- 491: 窓の矩形を位置と大きさから手で組む式が、`balloon_limit.rs`・`reseed.rs`・`persist.rs` の 3 か所にある（`follow/visibility.rs` の `rect_at` の写し）。`follow.rs:141` の `rect_at` の再公開を `pub(crate)` にし、3 か所を `rect_at(pos, size)` に替える（3 つのファイルは `audit-placement-core` の持ち物＝その着地の後に触る）。
- 511: `follow_test_support.rs:465` の `code_only` が共有の `test_support::zorder_pair_deferred::code_only`（`test_support.rs:281`／`:424`）の写し、`:495` の `SharedFakeIo` が共有の `test_support::SharedFakeIo`（`test_support.rs:46`）の小さな写し。共有のものを使う。
- 7 回目: `follow/sticky_follow.rs:311` の組のドラッグの終わりは、相手ごとに `persist_entries` を 1 回ずつ呼ぶ（1 回のドラッグの終わりで記憶へ N+1 回書く）。1 つの束にまとめて 1 回で書く（`audit-placement-core` の 50 と同じ形）。
- 名前の無い番号: 追随の持ち物（`crates/areka-placement/src/follow*`・`src/transition_*`・本体の `crates/areka/src/placement_tests/transition_*`）に、名前の無い古い spec の task 番号が `git grep -nE "(task|タスク) ?[0-9]+"` で 47 ファイル・215 行（spec の名前の付いた行も入った粗い数）、名前の無い指摘番号が `git grep -nE "指摘 ?[0-9]+"` で 18 ファイル・39 行。`audit-placement-core` の 165・166〜207 と同じ形で、番号を消すか spec の名前を足す。`follow/drag_follow.rs:43`・`:133` の名前の無い要件の番号「2.4」「4.5」（`window-link-directives` の番号）もこれに入る。

### 1,000 行

- `follow/drag_follow.rs` は 1,000 行ちょうど（番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の上限）。164 の直しも名前の無い番号の直しもここに触るので、先に分ける。`follow/window_move.rs` は 1,228 行で例外表に載っている（表の持ち物は `audit-test-kits`）。

## Desired Outcome

- 上の件が全部直っている（開発者「とにかく指摘事項は全部直す方針」）。
- 追随の寄せの組が、鎖の決まり（`chain_rule.rs`）と同じ「鎖の前の窓との組」で作られている。追随の持ち物に、鎖の式の写しが残らない（`audit-placement-core` の `research.md` 18.10「これで終わる理由」2 の grep で、追随の `joined_with_prev` の 1 か所が消える）。
- ドラッグの始めの位置が分からないとき、(0,0) を基準に窓を動かさない。分からないことが記録に残る（`warn!` か `debug!`・記録の無い失敗の道を作らない）。
- `follow/drag_follow.rs` が上限の下に入っている。
- 直した振る舞いには、それが壊れたら落ちるテストが 1 本ずつある（鎖の寄せは 18.10 の表の T6・T8 の形の World で）。
- 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑。

## Approach

- 件はもう出揃っているので、audit を新しくかけることは本体にしない（ギャップ分析は、上の件の行を `main` で引き直すことと、件ごとの直し方の下ごしらえ）。直した後に受け持つ範囲へ audit をかけ直すかは要件の討議で決める（`audit-placement-core` の 18.11 は開発者の決めで打ち切った）。
- 鎖の寄せは、`chain_rule.rs` の `ChainRule`（`ChainView::Saved`＝descript の揃え方で見る・印は次の起動が読むので）から組の相手を引く形を第一に確かめる。`keep_restored_groups` は記憶の矩形と印だけを受けているので、呼び手（記憶の戻しの `restore_and_judge` の側）から「鎖に入るか」を渡す形が要る。
- 164 は、wintf の型の形（`initial_window_pos` を `Option` にするか、位置の分からないときに `DraggingState` を挿さないか）を設計で比べる。組む 11 か所と、wintf の公開の形（crates.io へ出すクレート）への影響を挙げる。

## Scope

- **In**: 上の件の直しと、そのためのテストの追加と直し。`follow/drag_follow.rs` の分け方。164 のための wintf の `ecs/drag` の型と組む所の直し。491 のための `balloon_limit.rs`・`reseed.rs`・`persist.rs` の 3 か所の式の差し替え。
- **Out**: 鎖の決まりそのものの変更（`chain_rule.rs` の規則は `audit-placement-core` が決めた仕様のまま使う）・`defaultx` の読み（`defaultx-canon` の持ち物）・新しい機能・`vendors/`・clippy の指摘の機械的な直し（`clippy-199-lints`）・上限の例外表の変更（`audit-test-kits`）。

## Boundary Candidates

- 鎖の寄せ（`work_area_clamp.rs` とその呼び手）・164（wintf の型と `drag_follow.rs` の分け方）・写しと古い文（テストの支えが多い）の 3 つの塊でタスクを切る。`drag_follow.rs` の分け方は 164 より先。

## Out of Boundary

- 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件・`audit-placement-core` の `research.md` 18.10「この仕様で行く」と「残す限り」の 8 つ）を覆す直し。直さずに議題にする。

## Upstream / Downstream

- **Upstream**: `audit-placement-core`（鎖の決まり `chain_rule.rs` と、491 の 3 つのファイルの持ち主・着地を待つ）。
- **Downstream**: `defaultx-canon`（`chain_rule.rs` の隙間の読み ⑵ を開け直す＝この spec の鎖の寄せと同じ所を通る。この spec の着地の後に取る＝同じウェーブに並べない）。`runtime-crate-split`（本体の `placement_tests/transition_*` を `areka-runtime` へ移す＝名前の無い番号の直しと重なるので、どちらを先にするかはウェーブを組む棚卸で決める）。

## Existing Spec Touchpoints

- **Extends**: 完了した `audit-placement-follow`（同じ範囲の 2 本目）。
- **Adjacent**: `defaultx-canon`（`chain_rule.rs`・`resolver.rs` と鎖の寄せ）・`clippy-199-lints`（同じクレート）・`pilot-windows-rs-next` と windows-rs の移行の本体（wintf の `ecs/drag` に触る＝v0.2.0 の作り直しの前に済ませる）・`runtime-crate-split`。

## Constraints

- **鎖の振る舞いは `chain_rule.rs` に従う**（`audit-placement-core` の `research.md` 18.10 で採った仕様）。追随の側に鎖の式を新しく持たない。
- **範囲の外へ直しが及ぶとき**: 関数の形を変えたら、ツリー全体で呼び出し元を探して全部直す。相手の範囲の spec が走っていれば、そのセッションと申し合わせてから触る。wintf の `DraggingState` は公開の型なので、形を変えるなら `wintf` の公開の段（crates.io）への影響を設計に書く。
- **直さずに残してよいのは次の 3 つだけ**（見直しの 33 本と同じ）。どれも要件の討議で開発者の裁定を仰ぐ。
  1. 開発者の裁定に反する直し。
  2. わざと置いた先送りの印（`#[allow(dead_code)]` に実在の理由の doc を付けた語彙の口・限界が今の負荷を越えていない `shortcut:`・`ponytail:` の注釈）。
  3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。ukadoc で確かめてから決める。
- **守る決まり**: 1 ファイル 1,000 行の番人を越えない（足すなら新しいファイルへ）・決定論のテストで網羅する・記録の無い失敗の道を作らない（`error!`＋`Err`）・実機の根と一時フォルダはワークツリーの `target` の下・文字化けをコンソールの文字コードで逃げない・依存を足すなら `tech.md` へ登記して開発者の承認を得る。
- 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
- 1 spec 20 タスクを越えそうなら、要件の討議で 164 を同じ範囲の 2 本目として切ってよい。
- **規模の見立て**: M（12〜18 タスク）。164 と `drag_follow.rs` の分け方が半分を占める。
- **要件定義のモデル**: Opus（今ある crate の中の直しで、鎖の決まりはもう決まっている）。
- **優先度の区分**: A（見直しの方針の続き）。

## 2026-11-17 棚卸㊴の再測定（main 7f41b53e）
- **前提の変化**: `balloon-flip-side-rules`（11-17）が `balloon_limit.rs`・`follow.rs`・`follow/window_move.rs`（1,234 行）を書き換え、新しい `follow/other_chars.rs`（108 行・判定に入れるキャラ窓の列と、並べ直しの後の判定し直し `register_balloon_rejudge`）とそのテストを足した。`runtime-crate-split`（11-17）で本体の `runtime/` は `crates/areka-runtime/src/` へ、`placement_tests/transition_*` の 16 ファイルは `crates/areka-runtime/src/placement_tests/` へ移った。
- **行の引き直し**: `work_area_clamp.rs`（`opposite` 299・`keep_restored_groups` 412・`joined_with_prev` 489）・`drag_follow.rs`（1,000 行ちょうど・291／293）・`follow_test_support.rs`（614 行・`assert_no_event` 418）・wintf の `dispatch.rs` 103／129・`ecs/drag/mod.rs` 77 は変わらない。動いたのは `window_move.rs` の `AnchorChange` の文（526〜530 行・上流の一覧は 130 行）・`follow.rs` の `apply_restored_placements` と「task 1.4」「task 2.4」（122 行）・`balloon_limit_flip_tests.rs` の `point_of`（336 行）。
- **491 の足し**: `balloon_limit.rs` の手組みの矩形は 2 か所になった＝202 行（映した先の矩形・`balloon-flip-side-rules` が足した）と 291 行の `char_rect_of`（前の 253 行の式を 1 つにまとめたもの）。`reseed.rs` 347・`persist.rs` 215／245 は残っている。
- **164 の触る所（新しい置き場）**: `DraggingState` を組む所は 11 か所のまま＝wintf の `ecs/drag/dispatch.rs:129`・`ecs/drag/systems.rs:41`・`ecs/window_proc/mouse_move_drag_origin_tests.rs:12`・`tests/drag/dispatch_test.rs:442`・`:484`、配置の `follow_test_support.rs:91`、`crates/areka-runtime/src/frame_chain_realign_exclusion_tests.rs:248`・`frame_dpi_drag_grab_tests.rs:60`・`:185`、本体の `crates/areka/src/runtime_tests/frame_chain_finalize_persist_restart_tests.rs:66`・`frame_chain_rule_tests.rs:414`。読む・書き換える所＝`crates/areka-runtime/src/frame/dpi.rs`（179〜240 行）・wintf の `ecs/window_proc/mouse_move.rs`（404・405 行）・`crates/areka/src/input_events/drag_test_support.rs`（245〜249 行）・`crates/areka-runtime/src/frame_dpi_drag_grab_tests.rs`（34・78・137 行）・`tests/drag/dispatch_test.rs` 117 行。wintf の中の控え `DragState::Dragging.initial_window_pos`（`ecs/drag/state/mod.rs` 84・259 行・位置が無いときは `move_window` を偽にして守っている）と `ecs/clickthrough/controller_tests.rs` 26 行は、型をそろえるなら触る。
- **申し送りの抜け**: `runtime-crate-split` はこの brief に書いていない（同 spec の `verification.md` 9.5）。`audit-emo2-boot-spine` の brief の申し送り ⑵ が、この spec の持ち物として `crates/areka-runtime/src/placement_tests/transition_judge.rs` の古い説明を挙げている＝45 行の `runtime/frame_transition_*`（今は `crates/areka-runtime/src/frame_transition_*`）と、47〜50 行の「areka は lib ターゲットを持たない bin crate ゆえ…`main.rs` が `#[cfg(test)]` の下で宣言する」（今は `areka-runtime` の `lib.rs` が宣言する）。
- **触るファイル**: `crates/areka-placement/src/` の `follow*`（新しい `follow/other_chars.rs` を含む）・`transition_*`・`balloon_limit.rs`・`reseed.rs`・`persist.rs`・`balloon_limit_flip_tests.rs`・`balloon_lock_drag_tests.rs`・`spawn_follow_pipeline_tests.rs`・`lib.rs`、wintf の `ecs/drag/`・`ecs/window_proc/mouse_move*.rs`・`tests/drag/dispatch_test.rs`、`crates/areka-runtime/src/` の `frame/dpi.rs`・`frame_dpi_drag_grab_tests.rs`・`frame_chain_realign_exclusion_tests.rs`・`placement_tests/transition_*`、本体の `runtime_tests/frame_chain_{finalize_persist_restart,rule}_tests.rs`・`input_events/drag_test_support.rs`。名前の無い番号は 48 ファイル・218 行と 18 ファイル・39 行（ほぼ変わらない）。
- **規模**: M（14〜19 タスク）。164 の読み手と申し送りの 1 件を足して上の端へ寄った。越えたら 164 を同じ範囲の 2 本目に切る（Constraints のとおり）。
- **先に要るもの**: 無し（`balloon-flip-side-rules`・`runtime-crate-split` は着地した）。
- **同じウェーブに置けない相手**: `balloon-flip-ssp-review`（`balloon_limit.rs`・`balloon_limit_flip_tests.rs`・`follow/window_move.rs`、結論しだいで `follow/other_chars.rs`＝この spec が先）・`defaultx-canon`（この spec の後）・`audit-emo2-boot-frame`（`frame/dpi.rs` と `frame_*_tests.rs` の持ち主＝この spec の後）・`input-crate-split`・`audit-areka-input-menu`（`input_events/drag_test_support.rs`）。`balloon-text-visual-tree` とは 0（ただし箱の文字の面の名前を `other_chars.rs` が読む＝向こうが名前を変えるなら順を決める）。
- **優先度の区分**: A（見直しの方針の続き・開発者「推奨どおりで」）。
- **要件定義のモデル**: Opus（今ある crate の中の直しで、鎖の決まりはもう決まっている。wintf の公開の型の形は前例 `WindowDragContext.initial_window_pos: Option<Point>` がある）。
- **分割の案**: 無し（越えたときだけ 164 を切る）。
