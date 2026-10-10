# Brief: areka-P0-zorder-chain-residue

> **起票 2026-09-02（棚卸⑫・Path C・台帳 spec）**: 完了 spec `scope-zorder-pinning`（zsp・PR#126・2026-09-02）が research §13.8／§13.9 に「裁定待ちの残件 9 件」として登記したうち、**引受先の実在検証で所有者ゼロと確定した 8 件**の受け皿。`balloon-canon-residue`／`status-execution-states` と同じ**台帳 spec**（列挙なき所有者なしを防ぐための登記先であり、着手は開発者裁定）。9 件中の残り 1 件（§13.9 #7＝COMPAT §8 の 5 行が roadmap の空行を指す）は棚卸⑫で **doc 側を直接是正して消化済み**（`doc/COMPAT_ARCHITECTURE.md:160-165` の `roadmap.md:132` 引用を台帳 spec／residue 項目番号への引用に差替）。

## 2026-09-20 棚卸⑮の再測定

**A 群は性質が変わった（main `fe157df1` での実測）。**

- **A-1（実窓の重なり順の間欠赤）は隔離済み。** `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs` の該当 2 本は `#[ignore]`＋環境変数 `AREKA_WINTF_REAL_WINDOW_ZORDER` で明示実行する形になっており、既定の `cargo test --workspace` をもう汚さない。本文の「e2e DoD への影響: あり（間欠）」は既定の走行では成り立たない。測定条件の問題か欠陥かの決着は未了のまま。
- **A-2 の vblank の側も隔離済み**（`crates/wintf/src/runtime/tick_bridge.rs`・`AREKA_WINTF_VBLANK_DEADLINE`）。本文が引く行は別物（役割名待ちの 5 秒の期限）。
- **A-2 の spine の族だけが生きている。** `SPIN_WAIT = 30 秒`（`crates/areka/src/emo2_boot/spine.rs`）を `spine_seriko_loop_tests.rs`・`spine_display_tests.rs`・`spine_talk_close_tests.rs`・`spine_text_scale_tests.rs`・`spine_boot_smoke_tests.rs` の 5 ファイルが使い、`#[ignore]` は 0 件。
- **2026-09-11 以降の main で、この族が実際に赤を出した記録は 0 件**（`.kiro/specs/completed/*/verification/` を `flaky|再実行で緑|間欠赤` で引き、PR#157〜#167 の 6 spec を個別に確かめた。同じ検索は completed 全体で 20 ファイル以上に当たる）。roadmap の例外条項（α の spec のテストを実際に赤にしたら A 群だけを挟む）の発動条件は満たされていない＝**先回りしない**。
- **B-6 は消化済みの見込み。** `crates/wintf/src/ecs/window/zorder_pair_maintain.rs` の冒頭の説明は既に 3 つの仕組みを述べている。着手時に 1 行で確かめて閉じる。

## Problem

zsp は「所有の鎖」でスコープ窓の重なりを構造保証して着地したが、完成検証（2026-08-31）と実機サインオフが掘り当てた**テスト側・文書側の穴 8 件**は、6 候補（`zorder-property`・`tick-gate-adoption`・`balloon-canon-residue`・`status-execution-states`・完了 spec `ghost-window-zorder`／`test-cage-determinism`）のいずれも引き受けを明示的に拒否している（各 brief の Out 節が根拠）。所有者ゼロのまま放置すると、**次に `zorder_pair_maintain*.rs`／`zorder_chain*.rs`／`tick_bridge.rs` を触る spec が申し送りを読めない**（記憶 deferral-requires-verified-owner）。うち 2 件（§13.8 ①②）はワークスペース全体テストの**間欠赤**であり、`emo2-conformance-e2e` の DoD（`cargo test --workspace` exit 0）を直接脅かす。

## Current State（2026-09-02 実測・zsp research §13.8／§13.9 転記）

**§13.8（測定条件か欠陥かの裁定・所有者ゼロ 2 件・いずれも間欠赤）**

| # | 事項 | 実測アンカー | e2e DoD への影響 |
|---|---|---|---|
| A-1 | 既存ペア機構の実窓の檻が **3 プロセス同時 regime で稀に赤**——他プロセスの可視窓が owner 一組の間に割り込む／`SetWindowPos(HWND_NOTOPMOST)` が Ok を返しつつ帯から出さない。隔離測定では両ツリーとも 0 本＝**測定条件か欠陥か未決着** | `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs:767`・同 `:411` | あり（間欠） |
| A-2 | **壁時計期限の飢餓**——vblank 500ms 期限切れ／boot 応答 5 本が有界内に発火しない（後者は zsp 分岐点 `35387f00` でも同じ場所・同じ文言＝zsp 由来ではない） | `crates/wintf/src/runtime/tick_bridge.rs:355`・`crates/areka/src/emo2_boot/spine_boot_smoke_tests.rs:46`・`spine_talk_close_tests.rs:306`・**2026-09-11 e2e からの追記＝同じ族（壁時計期限 `SPIN_WAIT` 30 秒）**: `crates/areka/src/emo2_boot/spine_text_scale_tests.rs`・`spine_seriko_loop_tests.rs`・`spine_display_tests.rs`（e2e `verification/isolation-decision.md` §4.5.1・負荷走行で赤を観測・既定では緑） | あり（間欠） |

**§13.9（完成検証が掘った未担当・所有者ゼロ 6 件・いずれも挙動不変）**

| # | 事項 | 実測アンカー |
|---|---|---|
| B-1 | 分岐一覧が要件文と機械で結ばれていない（12 個目の偽分岐を足しても緑） | `crates/areka/src/placement/zorder_group_branch_coverage_tests.rs` |
| B-2 | 分岐 11 の wintf 側（`plan_chain_ops`）を名簿が守らない（crate 跨ぎ `include_str!` 不可） | `crates/wintf/src/ecs/window/zorder_chain_tests.rs` |
| B-3 | 生産者名簿の穴 2 件（照合が file 単位／doc 写し検査が wintf の木しか歩かない） | `crates/wintf/src/ecs/world/tick_gate_tests.rs`・`crates/areka/src/tick_gate_config_producers_tests.rs` |
| B-4 | 要件 12.1〜12.4・13.3・13.4 が COMPAT §8 の行だけに乗り檻が無い（**13.3/13.4 は `zorder-property` の語彙記録行そのもの**＝同 spec が実質の受け皿・本台帳は 12.1〜12.4 のみ持つ） | `doc/COMPAT_ARCHITECTURE.md:192-207`（09-02 合流で 176-191 から 16 行ずれ） |
| B-5 | 要件 11.5（タスクバーへの出方・クリック透過）と 8.2（入力受付を損なわない）に実行テストが無い（構造で論証のみ・実機目視でしか測れない性質・§8 に裁量登記済み） | — |
| B-6 | ペア機構の doc が 2 系統の処理列を述べたまま（本番は 3 系統） | `crates/wintf/src/ecs/window/zorder_pair_maintain.rs` |

所有者ゼロの根拠（zsp research §13.8 の引受先実在検証）: `zorder-property` は「Out: 窓の是正機構そのもの」、`tick-gate-adoption` は「In は門の裁定・`tick_bridge` の檻は範囲外」で明示的に拒否。完了 spec は申し送りを消化できない（記憶 deferral-requires-verified-owner）。

## Desired Outcome

1. 8 件それぞれに「消化」「隔離（除外＋理由）」「裁定で閉じる（測定条件と確定）」のいずれかの終端が付く。
2. A-1／A-2 は、**e2e が先に踏んだ場合は e2e が隔離裁定（除外 or 更新・記憶 obsolete-vs-broken-test-policy）を行い根治は本 spec**という分担を守る——e2e の DoD を間欠赤で汚さない。
3. B-1〜B-3 は「檻が浸食に対して片側しか守っていない」欠陥類型（zsp が 5 度掘り当てた）の是正として、**両側を守る形**へ（記憶 mutate-by-replacing-not-translating）。

## Approach

台帳 spec。着手時は zsp の `verification/`・research §13 を正本として各件を再測定し、A 群（間欠赤）を先に決着させる（隔離測定 n≥… の走行時間は**開発者方針「長時間試行禁止」**に従い、始める前に決着可能な設計を組む——zsp の 4,440 走行の教訓）。B 群は檻の改修のみで挙動不変。

## Scope

- **In**: 上の 8 件。A 群の隔離裁定の手順書化。
- **Out**: 所有の鎖そのもの（zsp 着地物）の変更・`currentghost.seriko.zorder`（`zorder-property`）・門の本採用（`tick-gate-adoption`）・§13.9 #7（棚卸⑫で消化済み）。

## Boundary Candidates

- A 群（間欠赤・test-determinism 類）と B 群（檻の片側性・doc）は独立——着手時に A 群だけ先に切ってもよい。

## Out of Boundary

- ペア機構の N 窓グループ化（zsp が「所有の鎖」で置き換え済み・退役済み）。
- `ReassertZOrder` 未消費（W7 申し送り⑴）は **e2e の着手時義務**（roadmap W12 行）であり本 spec は持たない。

## Upstream / Downstream

- **Upstream**: `scope-zorder-pinning`（完了）・`test-cage-determinism`（完了・A 群の道具＝`log-capture-kit`／`temp-path-kit`）。
- **Downstream**: `emo2-conformance-e2e`（DoD の間欠赤の分担先）・`zorder-property`（B-4 の 13.3/13.4）・`tick-gate-adoption`（B-3 の名簿を読む）。

## Existing Spec Touchpoints

- **Extends**: なし（台帳）。
- **Adjacent**: `zorder-property`（同じ zsp 残件の別軸）・`tick-gate-adoption`（`tick_bridge.rs` は同ファイルだが本 spec は檻のみ・製品行に触れない＝design で不変条件化）。

## Constraints

- `file_length_guard_test.rs` の例外表には触れない（新規ファイルは 1,000 行未満）。
- 檻は `log-capture-kit`／`temp-path-kit` の共通窓口経由（cage 規律）。
- M2 解禁ゲート（M1 では着手しない・A 群のみ e2e DoD の都合で前倒し裁定可）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 低・据え置き。09-11 以降 main で赤を出した記録は 0 件のまま（発動の条件は満たされていない）。
- 古くなった点: B-6 は解決済み・B-4 の指す場所がずれた・`SPIN_WAIT` を使うファイルは 5 → 約 10。B-4 は `zorder-property` へ移してよい。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 据え置きのままでよい。10-02 の後に C1・C2 で着地した 12 本の完了記録に、spine の族や重なり順・vblank のテストが赤を出した記録は無い（発動の条件は満たされていない）。`crates/wintf/src/ecs/window/zorder_pair_maintain.rs`・`crates/wintf/src/runtime/tick_bridge.rs` を触ったコミットは 0。
- A-2 の族の数え直し: 30 秒の期限 `SPIN_WAIT`（`crates/areka/src/emo2_boot/spine.rs`）を名指しするか、それを使う `spin_wait_until` を呼ぶファイルは `crates/areka/src/` に 18 本（名指しは 10 本）。`#[ignore]` は 0。brief の表の「5 ファイル」「約 10」は古い＝着手時に A-2 の対象を引き直す。
- 規模・切り方は 10-02 のまま。議題・穴: なし。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M のまま（A 群を外せば B 群だけで S〜M）。切り方は下の議題しだい。
- 前提の状態: **A-2 の発動の条件に触れる記録が出た**。roadmap の覚え書き（10-05）: `drag-cancel-borrow-miss` の完了時の全体テストで `emo2_boot::spine` の 3 本（`spine_blink_smoke_send_tick_drives_loop_pattern_command`・`spine_move_cue_drives_window_move_end_to_end`・`close_request_that_lands_during_boot_is_honored_without_any_second_change`）が `run_bounded` の 10 秒の期限（「spine ghost shutdown did not complete」）で赤。機械が重い回（x64 の段が約 1.3 倍）で、回し直し 5 回はすべて緑。main 上の赤ではないが、完了の手順の全体テストを止めた。
- 崩れた前提／古くなった位置:
  - A-2 の部品（`crates/areka/src/emo2_boot/spine.rs` の `spin_wait_until`・`run_bounded`）は、`ghost-session-test-load-flake` が直そうとしている待ちの部品と同じ。`spin_wait_until`／`SPIN_WAIT` を使うファイルは `crates/areka/src/` に 20 本（㉑の 18 本から増えた）・`run_bounded`／`join_bounded` は 7 本。`#[ignore]` は 0 のまま。
  - **`spine.rs` はちょうど 1,000 行**（番人は 1,000 を超えると赤）＝A-2 を直す spec は最初に部品を別ファイルへ出す必要がある。
  - A-1・B 群の場所は動いていない（C3 で `crates/wintf/src/ecs/window/zorder_pair.rs` の注記 2 行が直っただけ＝`drag-cancel-borrow-miss`・振る舞い不変）。`zorder_pair_maintain.rs`・`tick_bridge.rs`・B-1〜B-3 の檻のファイルを触ったコミットは 0。
- 触るファイル: A-2＝`crates/areka/src/emo2_boot/spine.rs` と新しい部品のファイル・`spine_*_tests.rs` の族。A-1＝`crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs`。B 群＝`crates/areka/src/placement/zorder_group_branch_coverage_tests.rs`・`crates/wintf/src/ecs/window/zorder_chain_tests.rs`・`crates/wintf/src/ecs/world/tick_gate_tests.rs`・`crates/areka/src/tick_gate_config_producers_tests.rs`。
- 議題: **A-2 を `ghost-session-test-load-flake` へ移すか**（同じ仕組み・同じファイルの部品を直すので、別々に走らせると取り合う）。移せば本 spec は A-1 と B 群だけになり、据え置きのまま規模は S〜M へ下がる。

### 棚卸㉒の裁定（2026-10-05）

- A-2（`emo2_boot/spine.rs` の締切の待ち）は `ghost-session-test-load-flake` へ移した。10-05 に `spine` の 3 本が `run_bounded` の 10 秒の締切で赤になり、直す部品が同じ `spine.rs` のため。本 spec は A-1 と B 群だけ＝S〜M。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: A-2 の spine の族は `ghost-session-test-load-flake`（✅ 10-10・PR#278）が引き取って直した。待ちの部品は `crates/areka/src/emo2_boot/spine_wait.rs` へ出て「進みの目印を見て打ち切る」形になり（`spine.rs` は 1,000 → 937 行）、負荷の下で許す赤の決まりが `.kiro/steering/tech.md` の「テストの待ちと、負荷の下で許す赤」に入った。
- **A 群で本 spec に残るもの**: 2 つだけ＝A-1（実際の窓の重なり順の 2 本・`crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs`）と、A-2 のうち vblank の期限の 1 本（`crates/wintf/src/runtime/tick_bridge.rs`）。どちらも `#[ignore]` と環境変数で明示して走らせる形に隔離済みで、ふだんの全体テストには入らない。残る仕事は「測り方の問題か欠陥か」の決着だけ。
- **10-05 の後に main で赤が出たか**: C4 の 16 本の完了の記録を引いた限り、重なり順・vblank・B 群の檻のテストが赤を出した記録は 0（`ghost-session-test-load-flake` の負荷の再現の記録にも出ていない）。発動の条件は満たされていない＝先回りしない（roadmap「生きている決まり」5）。
- **場所の動き**: `zorder_pair_maintain.rs`・`zorder_pair_maintain_always_on_top_tests.rs`・`tick_bridge.rs`・`crates/areka/src/placement/zorder_group_branch_coverage_tests.rs`・`crates/wintf/src/ecs/window/zorder_chain_tests.rs`・`crates/wintf/src/ecs/world/tick_gate_tests.rs`・`crates/areka/src/tick_gate_config_producers_tests.rs` を `f26aa1c1` の後に触ったコミットは 0。`wintf-tooltip`（✅ 10-08）は wintf に新しいファイルを足したが、これらには触れていない（同 spec の設計に「名簿の検査は行頭が `//` の行しか読み飛ばさない」とあり、B-3 と同じ檻の弱い所がもう 1 つ記録された）。
- **触るファイル**: 前回の一覧から A-2 の spine の分（`emo2_boot/spine.rs` と `spine_*_tests.rs`）を除く。**規模**: S〜M のまま。**分割の案**: 決着だけの A 群の残りと、檻を直す B 群に分けられるが、据え置きのあいだは分けない。
- **先に要るもの**: なし。**ファイルの重なり**: 保留の `tick-gate-adoption`（`tick_bridge.rs`・`tick_gate_tests.rs`）・`zorder-property`（B-4 の `doc/COMPAT_ARCHITECTURE.md` の行）。
- **優先度の区分**: D（据え置き）。**要件定義のモデル**: Opus。
- **見つけた穴・古くなった記述**: 冒頭の 09-20 の節の「A-2 の spine の族だけが生きている」「`SPIN_WAIT = 30 秒`（`spine.rs`）」は古い（定数と待ちの部品は `spine_wait.rs` へ移り、族は直った）。roadmap の台帳の行の「A-2 は 10-05 に移した」は合っている。

## 2026-10-10 `impl-watch` の完了時の全体テストからの申し送り

- **隔離していない 3 本目が赤になった**: `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs` の `a_fix_whose_insert_position_is_always_on_top_keeps_the_balloon_out_of_the_band`（`#[ignore]` でない・ふだんの全体テストに入る）が、`tools/test-all.ps1` の x64 の段でこの 1 本だけ赤（コミット `09f696c0`・ほかの段はすべて緑）。赤になったのは**対照の判定**「帯の中の窓を挿入位置にすると常時最前面になる（この危険が実在する）」＝帯の外の窓を、自プロセスの常時最前面の窓のすぐ後ろへ `SetWindowPos` で入れても `WS_EX_TOPMOST` の印が付かなかった。主張の側（バルーンとキャラに印が付かない）は通っている。
- **頻度**: 直後にこのテストだけを回し直すと、3 回のうち 2 回赤 → 続けて単独 12 回が 12 回とも赤・モジュール（29 本）6 回が 6 回とも赤。同じ日の朝の全体テスト（コミット `90f6ae90`）と、同じ日のほかの spec の完了の全体テストでは緑だった。
- **机の状態**: ロック画面でない・スクリーンセーバーなし・`cargo`／`rustc`／テストの実行ファイル／`areka`／`ssp` のプロセスは 0（静かな机）。OS は Windows 11 の 10.0.26300.9550。
- **このブランチの変更ではない**: `impl-watch` のブランチは `crates/wintf` を 1 ファイルも触っておらず、`Cargo.lock` の差分は新しいクレートの 1 塊だけ（＝main でも同じ実行ファイルになる。main での実走はしていない）。
- **読み**: 机の上のトップレベル窓の並び（常時最前面の帯にほかの窓が居るか・どこに居るか）で、「常時最前面の窓の後ろへ入れた窓に印が付くか」が変わる見込み。A-1 と同じ「測り方の問題か欠陥か」の仲間で、決着はここに残す。対照が机の状態に左右されるなら、対照の作り方（代役の後ろにもう 1 枚の常時最前面の窓を置いてから入れる、など）を見直すか、A-1 の 2 本と同じく明示の実行へ隔離する。
