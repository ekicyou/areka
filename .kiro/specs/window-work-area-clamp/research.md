# ギャップ分析: window-work-area-clamp

- 分析日: 2026-10-29（main `75e42d55` の上・SSP とアプリは動かしていない＝静的な読みだけ）
- 入力: `requirements.md`（確定）・`brief.md`（後の節が前の節を上書き＝件 4・件 5・2026-10-24 の裁定・棚卸㉚の再測定を正本として読んだ）・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md` の C12 の行）
- 書き方: 情報と案を並べる。決めは要件の話し合いに残す。

---

## 1. 分析のまとめ

- **寄せの核はもう在る**: キャラ窓とバルーンで同じ「左辺・上辺を優先して収める」式（要件 7.2）が `placement/balloon_limit.rs` の `clamp_rect_to_work_area`（`pub(crate)`）にあり、窓の属するモニタの決め方（要件 7.1）は `placement/follow/work_area.rs` の `work_area_for_window` にある。同じ 1 軸の式 `clamp_axis` は `resolver.rs`・`balloon_limit.rs`・`persist.rs` に 3 回写されている。新しく式を書く必要は無く、足りないのは「どこで・いつ・どの窓に当てるか」の配線と判定である。
- **件 1・件 2 の足（起動の並べ直し・拡大率の後の詰め直し）は 1 か所で掛けられる**: `finalize_chain_once_with`（`emo2_boot/frame/drain_resnap.rs`）と `realign_chain_once_with`（`placement/chain_realign.rs`）は、どちらも `resolve_chain` が返す `Vec<ChainMove>`（x だけ）を受けて `move_window_to`／`move_window_with_route` で書く。寄せはこの「解いた後・書く前」に挟める。ただし `ChainSlot` は x と幅しか持たないので、寄せの判定には y・高さ・作業領域が別に要る。
- **件 2 は前提が動いた**: `descript-window-keys`（10-24 着地）の実機（`.kiro/specs/completed/descript-window-keys/tasks.md` 8.2・run1／run2）で、ぱすたの窓は最初から面 1000 の大きさ（200% で 1108×1368・下端 1704＝作業領域の下端）で置かれ、スコープ 0 のバルーンの `Clamp` は 0 件だった。ぱすたで見えた件 2 と件 4 は、もう再現しない見込みが高い。一方で「`free` の窓が起動の台本で既定の面より大きい面へ替わる」一般の場合は今も左上を保ってはみ出す（`project_anchor` の `Free` は恒等）。件 2 を直す場所（`resize_window_to`）は 1,221 行の `window_move.rs` の中にある。
- **件 3 は記憶に材料が足りない**: 記憶にある組の手がかりは隣り合いの印（`left`／`right`／`none`）だけで、`none` は「離れていた・重なっていた・一緒に動く組」を区別しない。一緒に動く組（`StickyGroupLedger`）は記憶に残らず、台本で作った組は戻す時点（窓を作る前の純関数 `apply_restored_placements`）にはまだ無い。要件 4.1 の 3 種の組を見分ける方法を決める必要がある。
- **件 5 は areka の側の見込みが濃い（確かめていない）**: 台本の位置の指令（`\![move]` など）は GPU の装着を待たず `GhostWindows` が在れば適用され、そのとき拡大率 k は `applied_ratio(..).unwrap_or(ScaleRatio::ONE)`＝初回の表示より前なら 1 に落ちる。また `frame.rs` は「面の指令と `\![move]` は別の線で届くので、別のフレームに割れた場合の隙間はここでは扱わない」と既知の穴を書いている。どちらも kanade ではなく `emo2_boot` の持ち物。測り ⑸ で決める。

---

## 2. 今のコードの地図（要件ごと）

### 2.1 共通の部品

| 部品 | 場所 | 中身 | この spec での使い道 |
| ---- | ---- | ---- | ---- |
| 作業領域の集合 | `placement/follow/work_area.rs` `MonitorSnapshot`（323 行） | 全モニタの作業領域（物理 px）。毎フレーム先頭の `work_area_sync` が作り直す | 寄せの入力 |
| 窓の属するモニタ | 同 `work_area_for_window`／`_with_origin` | 窓の中心の半開の帰属・どこにもいなければ一番近いモニタ・空なら `None` | 要件 7.1・7.4（`None`＝寄せずに記録） |
| 矩形の寄せ | `placement/balloon_limit.rs` `clamp_rect_to_work_area`・`limit_correction`（231 行） | 左辺・上辺を優先して 4 辺を収める。`limit_correction` は動かないとき `None` | 要件 7.2 と同じ式。キャラ窓にそのまま使える |
| 寄せの記録の型 | 同 `BALLOON_LIMIT_CLAMP_TAG`・`..._UNRESOLVED_TAG`・`context` の 3 語 | `[balloon-limit] Clamp` を 1 語で grep できる形 | 要件 9 の記録の先例（新しい接頭辞を立てる） |
| 起動の初期配置 | `placement/resolver.rs` `resolve_placement` P4（491 行） | キャラ窓を窓ごとに作業領域へ寄せる（重なりうる） | 件 1 の議題 1 の**今の先例**（下の 4.1） |
| 記憶からの戻し | `placement/persist.rs` `project_restore`・`apply_restored_placements`（815 行） | 窓ごとに: 作業領域とまったく交わらないときだけ寄せる。寄せた事実はログ（`areka::persist::project`・`clamped=`）だけで、後へ運ばない | 件 3・議題 4 |
| 単一の窓書込口 | `placement/follow/window_move.rs` `enqueue_window_set_pos`（1,221 行・行数の番人の例外表に載る） | 書く・バルーンの runtime 関門・既定の位置の追随（システム由来の経路だけ） | 全件の書き込み先 |

### 2.2 要件と資産の対応（Missing＝無い・Unknown＝測りや判断が要る・Constraint＝既存の縛り）

| 要件 | 今ある所 | 足りないもの |
| ---- | ---- | ---- |
| 1.1 起動の並べ直しを作業領域へ | `finalize_chain_once_with`（`drain_resnap.rs` 420 行目）→ `boot_pair_decision`（`chain_finalize.rs` 157）→ `resolve_chain`（375）→ `move_window_to`（518） | **Missing**: 解いた x を作業領域で寄せる段。`ChainSlot` に y・高さが無い（走査 `collect_chain_states` は `targets` に今の位置を持つので y は取れる・高さは `WindowPos.size` から） |
| 1.2 並びの順 | — | **Unknown**: 測り ⑴・議題 1 |
| 1.3 収まっていれば動かさない | `limit_correction` の `None` | なし（式で満たせる） |
| 1.4 台本で置いた窓は動かさない | `boot_pair_decision` の `Keep(ExplicitlyPlaced)`（`ScriptPlaced` の印） | なし（寄せは `moves` にだけ当てれば自然に外れる） |
| 2.1 拡大率の後の詰め直し | `realign_chain_once_with`（`chain_realign.rs` 214〜360）が `move_window_with_route(.., ChainRealign)` で書く | **Missing**: 1.1 と同じ段 |
| 2.2 記憶からの隣り合わせ | 1.1 と同じ道（`default_x == None` の組に `remembered_decision` の `JoinLeft`／`JoinRight`） | **Missing**: 1.1 と同じ段 |
| 2.3 動かさなかった窓 | `KeepReason` の各語 | なし |
| 2.4 寄せた位置を書かない | 詰め直しは `ChainRealign`（既定の位置は書込口が運ぶ・記憶には書かない）。記憶から隣り合わせた窓は既定の位置を付けない（`drain_resnap.rs` 536〜543） | **Constraint**: 寄せた量が既定の位置へ運ばれる経路を保つこと |
| 3.1 起動の表示の間の `free` の拡大 | `resnap_from_sizes`（`drain_resnap.rs` 221）→ `resize_window_to`（`window_move.rs` 180）→ `project_anchor` の `Free`＝恒等（`anchor.rs` 110） | **Missing**: ①「起動の表示の間」を表す状態 ② 拡大の後に寄せる段 |
| 3.2 はみ出した向きにだけ | `clamp_rect_to_work_area` が満たす（向きごとの min/max） | なし |
| 3.3 台本が収めた後の位置を基準に | `frame.rs` の相順: `resnap_shell_targets` → `run_move_drain_phase`（resnap の後・move-canon D4） | **Constraint**: 寄せは resnap の中（同じフレーム・移動の前）で済ませる必要がある。後で寄せると同じフレームの `\![move,--base=0,…]` がはみ出した矩形を読む（10-24 の記録の `base_bottom=2232` はこれ） |
| 3.4 並べ直しをやり直さない | 並べ直しは一度きり（`ChainFinalized`） | **Unknown**: 寄せが確定の**前**に起きると、確定は寄せた後の x を基準に相方を付ける（やり直しではないが結果は変わる・4.3） |
| 3.5 起動の後の扱い | 今は左上を保つ | **Unknown**: 議題 2 |
| 4.1 記憶の組を離さない | `project_restore` は窓ごと（`merge_scope` が 1 スコープずつ） | **Missing**: 組を見分ける材料（5.3）と組ごとに寄せる段 |
| 4.2 組の寄せ方の細部 | — | **Unknown**: 測り ⑶・議題 3 |
| 4.3・4.4・4.5 | `project_restore` の可視の判定・窓ごとの寄せ・書き戻さない構造（純関数で World を持たない） | なし（保つだけ） |
| 4.6 古い記憶の印の書き足し | `persist_unremembered_char_positions` → `push_adjacency_marks`（表示の矩形から決める） | **Missing**: 「この起動で寄せたか」を戻しから保存へ運ぶ値（今は `project_restore` のログにしか残らない） |
| 5.1〜5.4 バルーンだけの寄せ | 起動時関門 `apply_balloon_limit`（`main.rs` 785 行目の 1 点）・runtime 関門 `apply_balloon_limit_gate`（`window_move.rs` 985） | **Unknown**: 測り直し（5.4 の見込み） |
| 6.1〜6.3 窓あわせ | `run_move_drain_phase`（`drain_resnap.rs` 144）・`apply_move_directive`（`emo2_boot/move_cue_place.rs` 108）・kanade `schedule/boot.rs` | **Unknown**: 原因（測り ⑸・下の 6 節） |
| 7.3 1 フレーム遅らせない | 確定・詰め直しは同じ呼び出しの中で解いて書く | **Constraint**: 件 2 の寄せを `resize_window_to` の外（別の書き込み）で行うと、同じフレームに 2 回の `SetWindowPos` が出る（下の 4.2 案 2-b） |
| 7.4 作業領域が無い | `BALLOON_LIMIT_UNRESOLVED_TAG` と同じ型 | なし（型を写す） |
| 7.5 寄せた後の位置を記憶へ | 確定は `moved_default_pos(old_default, new_x)` で既定の位置の **x だけ**を運ぶ（`drain_resnap.rs` 545）。保存は「今の位置＝既定の位置」のときだけ書く | **Constraint**: 寄せで y も動くと既定と食い違い、「並べ終える前に動かされた」と読まれて書かれない。件 2 の寄せは `Resnap`（システム由来）なので既定の位置は書込口が運ぶ |
| 7.6 ドラッグの窓 | 詰め直しは `DraggingState` を `Keep(Dragging)`、確定は既定の位置との食い違いで外す | **Unknown**: 要件 2.2（覚えた印で隣り合わせた窓＝利用者のドラッグの位置から戻した窓）を寄せることと、7.6 の除外の文言（「記憶から戻すときの今の判定と要件 4 を除く」）が噛み合っていない（下の議題 11） |
| 7.7 バルーンが付いて動く | `move_window_with_route`・`resize_window_to` が `BalloonFollow` で随伴 | なし |
| 8 SSP の測り・§8 | `doc/COMPAT_ARCHITECTURE.md` §8（124 行目〜） | 測り（開発者の机）と行の追記 |
| 9 記録 | 確定・詰め直しは組ごとの行を既に出す（`chain_finalize:`・`chain_realign:`） | **Missing**: 寄せの行（場面・窓・作業領域・from→to）と、寄せなかった理由の行 |
| 10 テスト | 純関数の檻（`chain_finalize_*_tests.rs`）・フレームの檻（`emo2_boot/frame_chain_finalize_*_tests.rs`・偽の `PhysicalSizeSource`）・`persist_restore_tests.rs`・`follow_resize_tests.rs` | 新しい形 ⑴〜⑻。既存の檻のうち `MonitorSnapshot` を入れていて、解いた x が作業領域の外へ出るものは期待が変わりうる（設計で洗う） |

---

## 3. 規約と縛り（設計で守るもの）

- **行数**: `placement/mod.rs` 969 行（上限まで 31）・`main.rs` 981 行（同 19）・`placement/follow/window_move.rs` 1,221 行（例外表に載る・C12 の約束「足さない＝兄弟のファイルへ」）。`follow.rs`（238）・`drain_resnap.rs`（685）・`chain_finalize.rs`（613）・`chain_realign.rs`（423）・`persist.rs`（815）・`emo2_boot/frame.rs`（615）は余裕がある。
- **新しいファイルの置き場**: C12 の約束は「寄せる純関数の新しいファイルは `placement/follow/` の下」。`follow/` に置くと `mod` の行は `follow.rs`（238 行）に入り、`placement/mod.rs` には入らない。
- **テストの置き場**: 本番ファイルに本体を書かず、同じディレクトリの兄弟ファイル＋`#[path]` の接続宣言（`structure.md` 151 行目）。
- **単一の窓書込口**: 位置は `enqueue_window_set_pos` を通す。システム由来の経路（`Resnap`・`ChainRealign` ほか）だけ既定の位置を運ぶ（`track_default_char_pos`）。`MoveCue`（確定の `move_window_to`）は運ばないので、確定は自前で既定の位置を直す。
- **経路の語**（`PlacementRoute`）を増やすと、`window_move.rs` の網羅 `match`（`deferral_covers_route`）に腕を足すことになる＝`window_move.rs` に触る。
- **記録**: `logging.md` の水準。寄せた＝`info!`、作業領域が取れない＝`warn!`、寄せなかった理由（要件 9.2）＝実機で開ける水準（`debug!` か `trace!`）。接頭辞 1 語で grep できる形（`[balloon-limit] ` の先例）。
- **C12 の兄弟との重なり**（`roadmap.md` 198 行目）: この spec の持ち分は `placement/{chain_finalize,chain_realign,persist}.rs`・`emo2_boot/frame/drain_resnap.rs`・`placement/follow/` の新しいファイル・`balloon_limit.rs`・§8。約束は `placement/mod.rs` に `mod` を足さない・`window_move.rs` に足さない・`placement/spawn.rs` に触らない。兄弟 7 本の持ち分と重なるファイルは今の見立てでは 0。ただし:
  - ① `mcp-strict-errors` は kanade の `msg.rs`・`schedule/{sstp,translate}.rs` と `emo2_boot/consumer_ledger.rs`（行を足す長い鎖）を触る。件 5 の直しが kanade の `schedule/boot.rs` に及んでもファイルは重ならないが、`consumer_ledger.rs` に行を足す形は取れない（鎖に割り込む）。
  - ⑤ `startup-install-option` は `ghost_session.rs` に触らない約束。この spec も触らない形が取れる。
  - roadmap の ⑦ の行は件 4 の所を `placement/follow/balloon_limit.rs` と書いているが、実物は `placement/balloon_limit.rs`（次の棚卸で直す誤記）。

---

## 4. 件ごとの実装の案

### 4.1 件 1・件 2 の足（起動の並べ直し・詰め直し・記憶からの隣り合わせ）

今の流れ: 走査（`collect_chain_states`）→ 組の決め → `mark_non_positive_widths` → `resolve_chain`（x の列）→ 書く。確定と詰め直しで同じ `resolve_chain` を使う。

- **案 1-A（解きの中で寄せる）**: `resolve_chain` に作業領域（または「スロットごとの作業領域」）を渡し、組ごとに解いた x を寄せてから次の組の基準にする。
  - ✅ 3 体以上の鎖で「寄せた n の隣に n+1 を付ける」が自然に成り立つ（基準＝前の窓の解いた**後**の x・`dpi-realign-remembered-chain` 1.5 と同じ形）。
  - ❌ `resolve_chain` は純関数で今は作業領域を知らない。署名が変わると既存の檻（`chain_finalize_resolve_tests.rs` ほか）と `finalize_chain`（試験の口）が全部追随する。
- **案 1-B（解いた後に寄せる段を足す）**: `resolve_chain` は変えず、返った `moves` に対して「動かす窓の矩形（新しい x・今の y・幅・高さ）が基準の窓の作業領域からはみ出すなら寄せる」純関数を `follow/` の新しいファイルに置き、確定と詰め直しの 2 か所から呼ぶ。
  - ✅ 既存の解きと檻に触らない。差分が小さい。
  - ❌ 3 体以上の鎖で n を寄せても、n+1 は寄せる前の n を基準に解かれている（鎖が割れる）。2 体なら差は出ない。要件は「キャラクターの数を 2 体に決め打ちしない」と書くので、3 体の形をどう扱うかを決める必要がある。
- **案 1-C（混ぜる）**: 解き（`resolve_chain`）はそのまま、寄せが入ったら寄せた x から残りを解き直す小さなループを新しいファイルに置く。
  - ✅ 檻を壊さず鎖も割らない。❌ 解きが 2 か所に見える。

共通の注意:
- **どの作業領域か**: 要件 1.1 は「本体の窓が属するモニタ」、2.1・2.2 は「組の動かさない側の窓が属するモニタ」と書く。2 体なら同じ。3 体以上で n−1 が別のモニタにいると食い違う（議題 2 として下に挙げた）。
- **今の先例**: 起動の初期配置（`resolver.rs` P4）は、隣へ付けた窓を**窓ごとに**作業領域へ寄せる（重なりうる）。ぱすたは本体が作業領域の左端（x=0）にいて、初期配置で相方も x=0 へ寄せられ（重なり）、起動の確定が `JoinLeft` で x=−1108 へ付け直した——件 1 の実測はこの道筋と合う。議題 1 の候補「窓ごとに作業領域の端へ寄せる（重なりうる）」は、初期配置と同じ決まりになる。
- **「組をまとめて内側へずらす」候補**は本体（先頭のスコープ）を動かす。確定の決まり「先頭のスコープは動かさない（接地点は不変・scg 7.2）」と当たる。
- **y**: 解きは x だけを扱う（7.2）。並べ直しで y がはみ出すことは無い見込み（既定の位置の y は初期配置で寄せ済み）だが、要件 7.2 は両軸を言う。y も寄せるなら、確定が既定の位置へ運ぶのは x だけなので、要件 7.5 の保存が「動かされた」と読んで書かない（3 節）。

### 4.2 件 2（起動の表示の間の `free` の窓の拡大）

まず「起動の表示の間」を表す状態が無い:
- kanade の `KanadeNotice::Steady` は `basewareversion` の応答で出る（`schedule/boot.rs` 120〜133）。起動のトークは撃ちっぱなしで、`Steady` はふつうトークの終わりより**前**に届く。
- 起動のトークの終わり（`boot_talk_done`）は kanade の中のログだけで、UI の側へは来ない。
- UI の側に来るのは、全トークに共通の `TalkLifecycleSignal::TalkStarted`／`TalkEnded`（`emo2_boot/talk_lifecycle.rs`・番号つき）。起動のトークは窓の一式で最初のトーク。
- `ChainFinalized`（確定）は全スコープの初回表示が揃った巡で立つ＝トークの途中で早く立つ。

「状態」の案:
- **案 2-① 起動のトークの終わりまで**: 窓の一式を作ったときに印を置き、その一式で最初の `TalkEnded`（または起動のトークの番号の `TalkEnded`）で外す。要件 3.1 の字面に一番近い。起動のトークが無い（204）回は、確定の時点で外すなどの決めが要る。
- **案 2-② 既定の位置にいる間**: 期間を持たず、「`free`・既定の位置のまま（`GhostWindows::default_char_pos` と一致）・台本の印なし・ドラッグ中でない」窓は、面が大きくなったらいつでも寄せる。状態が要らず、要件 7.6 とも合う。代わりに議題 2（起動の後も寄せるか）を「既定の位置のままなら寄せる」と決めることになり、要件 3.5 の「決まるまでは左上を保つ」と食い違う。
- **案 2-③ 確定まで**: 既存の `ChainFinalized` で足りるが、確定は早く立つので、トークの途中の `\s[...]` を取りこぼす。要件 3.1 を満たさない。

寄せる場所の案（`resize_window_to` は `window_move.rs` の中）:
- **案 2-a（呼ぶ 1 行だけ足す）**: `resize_window_to` の可視性の関門（手順 3c）の後・べき等の判定の前に、`follow/` の新しいファイルの関数を 1 行で呼ぶ。位置と大きさは今どおり 1 回で書かれ（要件 7.3）、同じフレームの `\![move]` は寄せた後の矩形を読む（要件 3.3）。経路が `Resnap`（システム由来）なので既定の位置も書込口が運ぶ（要件 7.5）。❌ `window_move.rs` に行が増える（約束の読み方の確認が要る＝議題 5）。
- **案 2-b（resnap の後で書き直す）**: `resnap_from_sizes`（`drain_resnap.rs`）が `resize_window_to` の後に寄せた位置を `move_window_with_route` で書き直す。`window_move.rs` に触らない。❌ 同じフレームに `SetWindowPos` が 2 回出る（見えるかは確かめていない＝調べること R3）。経路を `Resnap` で書けば既定の位置は運ばれる。
- **案 2-c（経路の語を足す）**: 起動の間だけ `PlacementRoute` の新しい語で `resize_window_to` を呼び、可視性の関門（`visibility.rs`）にその語の腕で寄せさせる。❌ 経路の網羅 `match` が `window_move.rs` にあり、結局そこに触る。記録の語彙も動く。

前提の変化（重要）:
- `descript-window-keys` の着地で、ぱすたは最初から面 1000 の大きさで置かれ、下端は 1704 に収まった（同 spec の `tasks.md` 8.2 の run1・run2）。10-24 の件 2 の記録（`run4`）はその前の版。**今の main でぱすたを回しても、件 2 の寄せは走らない見込み**。
- それでも「descript の既定の面より大きい面へ、起動の台本が切り替える `free` のゴースト」では起きる。決定論のテスト（要件 10.1 ⑶）は合成の値で固定できる。実機の確かめ（要件 10.4 ⑴）は、ぱすたでは寄せの道を通らない（崩れないことの確かめになる）。

### 4.3 件 3（記憶からの組の戻し）

今の流れ: `main.rs` `restore_merged_placements` → `persist::apply_restored_placements`（スコープごとに `merge_scope` → `project_restore`）→ `apply_balloon_limit` → 窓を作る → 初回表示が揃ったら確定（覚えた印 `left`／`right` の組は今の幅で隣り合わせ直す）。

組を見分ける材料:
- 隣り合っていた組（`left`／`right`）: 印で分かる。しかも確定で隣り合わせ直されるので、本体が `project_restore` で寄せられれば相方は寄せた本体の隣へ付き、相方が外へ出れば要件 2.2 の寄せが当たる＝**要件 4.1 の「隣り合ったまま」は 4.1 の段が入れば確定の側で満たせる見込み**（ただし本体が見えていて相方だけが見えない形では、相方だけが `project_restore` で寄せられ、その後で確定が本体の隣へ戻す＝結果は隣り合う）。
- 重なっていた組・一緒に動く組: 印は `none`（`AdjacencyMark::Separate`）で、離れていた組と区別できない。一緒に動く組の帳簿（`StickyGroupLedger`）は記憶に書かれず、台本の `\![set,sticky-window,…]` で作る組は戻す時点にはまだ無い（descript の `seriko.sticky-window` の組だけは読める）。

案:
- **案 3-A（記憶の位置どうしから組を推す）**: 記憶にある 2 つの位置と今の窓の大きさから矩形を組み、`judge_adjacency` で隣り合い・重なりを見て、組なら「寄せが要る窓の寄せ量と同じだけ組の全員を動かす」（相対の位置を保つ）。記憶の形は変えない。❌ 拡大率が変わった後の記憶では、今の大きさで組んだ矩形が覚えたときの並びとずれうる。一緒に動く組でも離れて置かれていたものは拾えない。
- **案 3-B（記憶の印の語を足す）**: 保存の時に `overlap`／`sticky` などの語を書き、戻す時に読む。❌ 記憶の書式が変わる（古い記憶との互換・`dpi-realign-remembered-chain` の印の決まりの改め＝§8 に残す件になる）。
- **案 3-C（寄せ量を組で共有するだけ）**: 組の見分けはせず、「記憶から戻したスコープのどれかが寄せられたら、記憶から戻した他のスコープも同じ量だけ動かす」。❌ 離れて置いた窓まで動く（要件 4.4 の「離れていた組は窓ごと」と当たる）。

置き場: `apply_restored_placements`（`persist.rs`・815 行）の中で全スコープを見た後に組の段を足すか、`follow/` の新しいファイルに置いて `persist.rs` から呼ぶ。`main.rs`（981 行）に足すのは避ける。

議題 4（古い記憶への印の書き足し）: 「寄せた起動では書き足さない」を採るなら、`project_restore` が寄せた事実を窓の一式まで運ぶ値が要る（今はログだけ）。候補は `GhostWindows` の覚えた印の表（`restored_scope_marks` が返す `BTreeMap<usize, Option<AdjacencyMark>>`）に並べて持たせるか、別の資源にするか。どちらも `main.rs` の窓を作る閉包の受け渡しに触れうる＝行数に注意。

### 4.4 件 4（バルーンだけの寄せ）

- `descript-window-keys` の 8.2（run1・run2）で、スコープ 0 のバルーンの `Clamp` は 0 件（起動の関門も runtime の関門も）。件 4 の起点（スコープ 0 のバルーンが上へ 88 論理画素寄せられた）は消えた見込み。
- 同じ走りで、スコープ 1 のバルーンに横向きの `Clamp` が出た（x −872→0・y はそのまま。起動の関門 1 件と、相方のドラッグの間の多数）。同 spec は「上へは寄せていない・関門の正しい働き」と判断した。これがぱすたの見た目でシェルとの組を崩すか（相方の側のバルーンが見える場面があるか）は、測り直し（要件 5.2）の対象に入れるのがよい。
- 要件 5.4 の場合（直しが要らない）なら、コードの差分は 0 で、記録と §8 の行だけになる。

### 4.5 件 5（初回起動のトークの窓あわせ）

静的に読んで立つ見込み（どれも確かめていない）:
- **見込み H1（areka・`emo2_boot`）: 装着より前に位置の指令が適用される**。`run_drain_phase` は GPU の装着（`wiring.attached`）まで面の指令を溜めるが、`run_move_drain_phase` は `GhostWindows` が在れば適用する（`drain_resnap.rs` 144〜150・「OnFirstBoot の位置調整を早期に取りこぼさない」ための作り）。装着の前は `applied_ratio(..)` が `None` で、k は `ScaleRatio::ONE` に落ちる（同 196〜199）。200% の画面では作者の px のずらしが半分になり、基準の窓の矩形も表示の前の大きさになる。初回の起動は記憶も表示の材料も冷えているので、装着がトークに遅れやすい見込み。
- **見込み H2（areka・`emo2_boot/frame.rs`）: 面の指令と `\![move]` が別のフレームに割れる**。`frame.rs` の注記（`run_move_drain_phase` の前）が「別の線で届くので、別のフレームに割れて届いた場合の隙間はここでは扱わない」と書く既知の穴。`\0\s[1000]\1\s[10]\![move,…]` の `\![move]` が面の差し替えより先のフレームで適用されると、古い大きさで位置を決め、後の再スナップが大きさだけを変える。
- **見込み H3（kanade）: 起動のイベントの時機**。kanade は窓の表示を待たず `OnInitialize` → 名前の照会 → `OnFirstBoot` を送る。SSP が `OnFirstBoot` をシェルの表示の後に送るなら、時機の差がずれの元になりうる。
- 「トークの後にずれる」ほうの元は、静的には決められない（ぱすたの辞書が後のイベントでもう一度窓あわせをする・拡大率の詰め直し・確定のどれか）。

持ち場の見立て: H1・H2 なら `emo2_boot`（`drain_resnap.rs` と `frame.rs`・`move_cue_place.rs`）＝この spec の持ち分の中で直せる（C12 の兄弟と重ならない）。H3 なら kanade の `schedule/boot.rs`（① と同じクレートだが別のファイル）。どれかは測り ⑸ で決まる。

---

## 5. 全体の実装の形（A／B／C）

- **A 既存を広げる**: `chain_finalize.rs` の `resolve_chain` に作業領域を入れ（案 1-A）、`persist.rs` の `apply_restored_placements` に組の段を入れ、`window_move.rs` の `resize_window_to` に件 2 の寄せを書く。✅ 新しいファイルが無い。❌ `window_move.rs` が伸びる（約束に反する）・純関数の檻の署名が広く動く。
- **B 新しい部品を立てる**: `placement/follow/` に寄せの新しいファイル（例: 「キャラ窓の作業領域への寄せ」＋場面の語＋記録の接頭辞）を立て、確定・詰め直し・戻し・件 2 の 4 か所は呼ぶだけにする。式は `balloon_limit::clamp_rect_to_work_area` を呼ぶ（5 つ目の `clamp_axis` を写さない）。✅ 約束（`mod.rs`・`window_move.rs`・`spawn.rs`）を守りやすい・記録の語彙が 1 か所に集まる・檻を兄弟のファイルにまとめられる。❌ 呼び口が 4 つに散る。
- **C 混ぜる**: B を骨にして、件 1 の鎖（3 体以上）だけ `resolve_chain` の後に解き直す（案 1-C）、件 3 は `persist.rs` の中に組の段（新しいファイルの関数を呼ぶ）、件 2 は案 2-a の 1 行。✅ 差分が小さく、約束の例外は件 2 の 1 行だけに絞れる。❌ 件 2 の 1 行の扱いを決める必要がある。

---

## 6. 規模と危うさ

- **規模: M**（8〜12 タスクの見立てと合う）。式は既にあり、足すのは配線・判定・記録・檻。件 3 の組の見分けと件 5 の原因で増減する。件 4 は差分 0 の見込み。
- **危うさ: 中**。
  - 件 5 の原因が測るまで分からない（H1〜H3）。H3 なら kanade に触る。
  - 件 2 の「起動の表示の間」を表す状態が無く、新しく決める必要がある。
  - 件 3 は記憶の材料が足りず、案によっては記憶の書式が変わる。
  - 既存の檻のうち `MonitorSnapshot` を入れて作業領域の外へ解く期待を持つものは書き換えになる（要件 10.2）。

---

## 7. 設計へ持ち越す調べ（Research Needed）

- **R1（測り ⑴）**: SSP は 2 人並べると本体の隣が作業領域の外になるとき、どこへ置くか。areka の初期配置（窓ごとに寄せる・重なりうる）と同じかも合わせて見る。
- **R2（測り ⑶）**: 記憶の位置が作業領域の外の組を SSP はどう寄せるか。重なっていた組・一緒に動く組を SSP が記憶で区別しているか（SSP の記憶のファイルに何が残るか）。
- **R3**: 同じフレームに同じ窓へ `SetWindowPos` を 2 回出したとき、途中の位置が画面に出るか（案 2-b を採るときだけ）。
- **R4（測り ⑸）**: 初回起動で装着と最初の `\![move]` の前後（areka の記録で `applied_ratio` が `None` だったか・k=1 で動かしたか）と、SSP が `OnFirstBoot` を送る時機（`ssp_shiori_log.txt` の差分）。areka の記録は今 k を移動の行に出しているかを先に確かめる（出ていなければ測りの前に記録を足す必要がある）。
- **R5（測り直し・要件 5.2）**: 今の main でぱすたを回し、スコープ 0・1 のバルーンの `Clamp` の行と、相方の側のバルーンの見え方を撮る。
- **R6**: 3 体以上の鎖で、寄せた窓の先の窓をどう解くか（案 1-A／1-C の要否）。`extra-character-windows` の先で効く。
- 測りの段取り: SSP の測り（⑴⑶⑷⑸）は開発者の机で行う（reload を打つときは毎回許しを取る・ウェーブの兄弟へ知らせる）。ukadoc が黙る点は議題の前に SSP で測る決まりがあるので、⑴⑶ を要件の話し合いの前に回すかどうかも決めどころ。

---

## 8. 要件の話し合いへの材料（議題の候補）

> **2026-10-29 の要件の話し合いの結果**（一括承認・正本は `requirements.md` の「要件の話し合いで決めたこと」）: 開発者の方針「汎用の対策を入れる・ぱすた独自の対応は不要」。下の 1 は反対の側へ並べ替え（入らなければ端へ寄せる・重なってよい）。4 は案 2-②（既定の位置にいる間はいつでも寄せる・SSP に無い安全網として §8 へ）。6 は emo2 を写した検体で件ごとの道を通すのが必達、ぱすたはゴールの通しの一例。7 は案 3-A（記憶の書式は変えない）。8 は「寄せた起動では書き足さない」。9 は一般の崩れとして扱い、シェルを収める直しで無くす（ぱすたの再現の有無で決めない）。10 はこの spec で直す（`consumer_ledger.rs` が要るときだけ切り出す）。11 は要件 7.6 の除外に 2.2 を足した。2 は「並べる基準の窓（2 体では本体）」へそろえた。3・5・12 と案の選び（1-A／1-B／1-C・2-a／2-b／2-c）は設計の段で決める。SSP の測りは ⑸ 必須・⑷ 条件つき・⑴⑶ 任意。

1. **件 1 の並びの順（議題 1）**: 初期配置の今の決まり（窓ごとに寄せる・重なりうる）が先例。「組をまとめて内側へずらす」は本体を動かし、確定の「先頭は動かさない」と当たる。測り ⑴ の前に決めるか、測ってから決めるか。
2. **寄せの基準の作業領域**: 要件 1.1 は「本体のモニタ」、2.1・2.2 は「組の動かさない側のモニタ」。3 体以上で食い違うので、どちらかへ揃えるか。
3. **件 1 で y も寄せるか**: 解きは x だけ。y も寄せるなら、確定が既定の位置へ運ぶ量に y を足さないと要件 7.5 の保存が書かれない。
4. **件 2 の「起動の表示の間」の定義**: 案 2-①（起動のトークの終わりまで・字面どおり・新しい状態が要る）・案 2-②（既定の位置にいる間ずっと・状態が要らない・議題 2 の答えも決まる）・案 2-③（確定まで・早すぎて要件を満たさない）。
5. **件 2 の寄せを `resize_window_to` に 1 行で差すか**: C12 の約束「`window_move.rs` に足さない（兄弟のファイルへ）」を「本体は兄弟のファイル・呼ぶ 1 行は可」と読んでよいか。だめなら案 2-b（同じフレームの 2 回書き・R3）か案 2-c（経路の語を足す＝結局 `window_move.rs` の `match` に触る）。
6. **件 2 の実機の確かめ**: 今の main のぱすたは件 2 の道を通らない見込み。要件 10.4 ⑴ は「崩れないこと」の確かめで足りるとするか、件 2 を通すゴースト（検体）を別に用意するか。
7. **件 3 の組の見分け**: 案 3-A（記憶の位置から推す）・案 3-B（記憶の印の語を足す＝書式の改め）・案 3-C（寄せ量を共有するだけ）。隣り合っていた組は確定の隣り合わせ直し＋要件 2.2 で満たせる見込みなので、要件 4 の実体は「重なっていた組・一緒に動く組」になる。
8. **議題 4 の値の運び方**: 「寄せた起動では書き足さない」を採るなら、戻しが寄せた事実を保存まで運ぶ値の置き場（`main.rs` は 981 行）。
9. **件 4 の扱い**: スコープ 0 の `Clamp` は `descript-window-keys` の後 0 件。測り直し（R5）の結果で要件 5.4 の腕に入るなら差分 0。スコープ 1 の横向きの `Clamp`（x −872→0）をこの件の対象に入れるか。
10. **件 5 の持ち場**: 静的な見込みは areka の `emo2_boot`（H1 装着の前の k=1・H2 フレームの割れ）。kanade（H3）なら、① `mcp-strict-errors` とファイルは重ならないが同じクレート。`consumer_ledger.rs` に行を足す形は ① の鎖に割り込むので取れない。測り ⑸ の後に切り出しを決める段取りでよいか。
11. **要件 2.2 と 7.6 の文言**: 覚えた印で隣り合わせた窓は利用者のドラッグの位置から戻した窓で、7.6 の「ドラッグで置いた位置にある窓は動かさない」の除外に要件 2.2 が入っていない。2.2 を除外に足すか、2.2 の寄せを外すか。
12. **既存の檻の書き換えの範囲**: 要件 10.2 の 3 つの保つテストのほかに、確定・詰め直しの檻のうち作業領域の外へ解く期待のものがあれば書き換える（設計で列挙）。
