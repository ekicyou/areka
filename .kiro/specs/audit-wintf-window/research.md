# ギャップ分析（＝見直しの audit）: audit-wintf-window

- 実施日: 2026-11-14
- 測った時点: 作業枝 `claude/kiro-start-audit-wintf-window-695c43` の `eb932eb3`（main `30a343eb` の上に spec の初期化を 1 つ足しただけ。範囲のファイルは main と同じ。範囲を最後に変えたのは `48bc0138`＝`audit-wintf-layout-widget`）
- 物差し: **ponytail 5.3.3** の `/ponytail:ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.3/skills/ponytail-audit/SKILL.md`。利用者全体に入っている版は `installed_plugins.json` で 5.3.3。キャッシュに残る 5.3.2 の同じフォルダと `diff -r` で中身が同じことを確かめた。起票のときの 5.3.2 と物差しは変わらない）。作法のうち「最大 20 件」と「読まなかった所（Not checked）」は外した（要件 1.4）。
- 読み方: 範囲の 69 ファイルを 9 つの組（G1〜G9）に分け、組ごとに 1 人の読み手が頭から終わりまで全部読んだ（読まなかったファイルは 0）。読み手は静的に読んだだけで、`cargo build`／`cargo test` は回していない。必ず直すの 6 件は、取りまとめの側でもコードの該当行を読み直して確かめた。
- この文書は分析と案だけを書く。直す・直さないの決めは要件の討議で行う。

---

## 1. 範囲の確定（要件 1.1・1.2）

`crates/wintf/src/ecs/window*` の下の git の管理下のファイル＝**69 本**（`window/` 51・`window_proc/` 18）。全部 Rust。本番 27・テスト 42（`_tests.rs`・`tests/` の下・`_test_support.rs`・`window/window_pos/tests.rs` をテストに数えた。要件の数え方と同じ）。着手のとき（`30a343eb`）から増減は無い（要件 1.2 の該当なし）。

| 区分 | ファイル | 行（空行を含む） |
|---|---:|---:|
| 本番 | 27 | 11,564 |
| テスト | 42 | 22,361 |
| 合計 | 69 | 33,925 |

（要件の表の 10,599／20,281 行は空行を除いた数。ここは空行を含む `wc -l`。）

読んだファイルの一覧（パスは `crates/wintf/src/ecs/` から。組は読んだ読み手）。

| ファイル | 区分 | 行 | 組 |
|---|---|---:|---|
| `window/command.rs` | 本番 | 941 | G1 |
| `window/command_batch_tests.rs` | テスト | 718 | G1 |
| `window/command_coalesce_tests.rs` | テスト | 809 | G1 |
| `window/command_threadlocal_tests.rs` | テスト | 184 | G1 |
| `window/command_transition_tests.rs` | テスト | 629 | G1 |
| `window/components.rs` | 本番 | 386 | G1 |
| `window/mod.rs` | 本番 | 40 | G1 |
| `window/window_system.rs` | 本番 | 91 | G1 |
| `window/window_handle.rs` | 本番 | 299 | G1 |
| `window/dpi.rs` | 本番 | 247 | G2 |
| `window/monitor.rs` | 本番 | 424 | G2 |
| `window/window_pos/mod.rs` | 本番 | 454 | G2 |
| `window/window_pos/tests.rs` | テスト | 261 | G2 |
| `window/window_band.rs` | 本番 | 292 | G2 |
| `window/window_band_pipeline_real_tests.rs` | テスト | 242 | G2 |
| `window/window_band_real_tests.rs` | テスト | 335 | G2 |
| `window/window_band_tests.rs` | テスト | 564 | G2 |
| `window/transition_diag.rs` | 本番 | 860 | G2 |
| `window/transition_diag_tests.rs` | テスト | 908 | G2 |
| `window/zorder_chain.rs` | 本番 | 631 | G3 |
| `window/zorder_chain_apply.rs` | 本番 | 930 | G3 |
| `window/zorder_chain_apply_nudge_tests.rs` | テスト | 672 | G3 |
| `window/zorder_chain_apply_release_tests.rs` | テスト | 720 | G3 |
| `window/zorder_chain_apply_test_support.rs` | テスト | 150 | G3 |
| `window/zorder_chain_apply_tests.rs` | テスト | 814 | G3 |
| `window/zorder_chain_diag.rs` | 本番 | 461 | G4 |
| `window/zorder_chain_diag_tests.rs` | テスト | 684 | G4 |
| `window/zorder_chain_order_inversion_tests.rs` | テスト | 276 | G4 |
| `window/zorder_chain_order_lifecycle_tests.rs` | テスト | 661 | G4 |
| `window/zorder_chain_order_outsider_tests.rs` | テスト | 427 | G4 |
| `window/zorder_chain_order_tests.rs` | テスト | 977 | G4 |
| `window/zorder_chain_release_tests.rs` | テスト | 461 | G4 |
| `window/zorder_chain_tests.rs` | テスト | 787 | G4 |
| `window/zorder_pair.rs` | 本番 | 929 | G5 |
| `window/zorder_pair_tests.rs` | テスト | 934 | G5 |
| `window/zorder_pair_deferred_vocabulary_tests.rs` | テスト | 375 | G5 |
| `window/zorder_pair_diag.rs` | 本番 | 237 | G5 |
| `window/zorder_pair_establish.rs` | 本番 | 224 | G5 |
| `window/zorder_pair_establish_release_tests.rs` | テスト | 160 | G5 |
| `window/zorder_pair_establish_tests.rs` | テスト | 524 | G5 |
| `window/zorder_pair_sink.rs` | 本番 | 214 | G6 |
| `window/zorder_pair_sink_tests.rs` | テスト | 816 | G6 |
| `window/zorder_pair_record_tests.rs` | テスト | 652 | G6 |
| `window/zorder_pair_measure_tests.rs` | テスト | 477 | G6 |
| `window/zorder_pair_maintain.rs` | 本番 | 563 | G7 |
| `window/zorder_pair_maintain_always_on_top_tests.rs` | テスト | 920 | G7 |
| `window/zorder_pair_maintain_detach_tests.rs` | テスト | 643 | G7 |
| `window/zorder_pair_maintain_multipair_tests.rs` | テスト | 472 | G7 |
| `window/zorder_pair_maintain_release_tests.rs` | テスト | 312 | G7 |
| `window/zorder_pair_maintain_survivor_tests.rs` | テスト | 466 | G7 |
| `window/zorder_pair_maintain_tests.rs` | テスト | 978 | G7 |
| `window_proc/window_pos.rs` | 本番 | 542 | G8 |
| `window_proc/window_pos_tests.rs` | テスト | 724 | G8 |
| `window_proc/window_pos_transition_tests.rs` | テスト | 763 | G8 |
| `window_proc/dpi_helpers.rs` | 本番 | 149 | G8 |
| `window_proc/dpi_helpers_tests.rs` | テスト | 586 | G8 |
| `window_proc/mod.rs` | 本番 | 382 | G8 |
| `window_proc/lifecycle.rs` | 本番 | 471 | G8 |
| `window_proc/lifecycle_tests.rs` | テスト | 217 | G8 |
| `window_proc/drop_files.rs` | 本番 | 160 | G9 |
| `window_proc/drop_files_tests.rs` | テスト | 244 | G9 |
| `window_proc/keyboard.rs` | 本番 | 233 | G9 |
| `window_proc/keyboard_tests.rs` | テスト | 392 | G9 |
| `window_proc/mouse_click.rs` | 本番 | 508 | G9 |
| `window_proc/mouse_click_tests.rs` | テスト | 149 | G9 |
| `window_proc/mouse_dblclick_wheel.rs` | 本番 | 264 | G9 |
| `window_proc/mouse_dblclick_wheel_tests.rs` | テスト | 224 | G9 |
| `window_proc/mouse_move.rs` | 本番 | 632 | G9 |
| `window_proc/mouse_move_drag_origin_tests.rs` | テスト | 54 | G9 |

1,000 行を越えるファイルは 0。いちばん長いのは `window/zorder_pair_maintain_tests.rs`（978 行・上限まで 22 行）と `window/zorder_chain_order_tests.rs`（977 行）。直しでこの 2 本に行を足すときは要件 8.1（新しいファイルへ分ける）に当たりうる。

---

## 2. このコードがしていることと、想定した負荷

wintf の「窓」と「窓の手続き」である。`window/` は、窓の部品（`Window`・`WindowStyle`・`WindowHandle`・`WindowPos`・`DPI`・`Monitor`）、窓の書き込み（`SetWindowPos`）をスレッドごとのキューに積んで畳み、tick の後に `DeferWindowPos` の 1 バッチで当てる仕組み（`command.rs`）、窓の重なり順を守る 2 つの仕組み（バルーン窓をキャラ窓のすぐ手前に置く「組」＝`zorder_pair*`、スコープの窓を所有の鎖でつなぐ「鎖」＝`zorder_chain*`）、常に手前の帯（`window_band.rs`）、DPI の移り変わりの記録（`transition_diag.rs`）を持つ。`window_proc/` は、OS からの窓のメッセージ（位置・DPI・閉じる・表示の構成・マウス・キー・ファイルの投げ込み）を受けて ECS へ写す。

想定した負荷: デスクトップマスコット 1 プロセス、UI スレッド 1 本、窓は数〜十数枚、画面更新 60〜120 Hz、マウスの移動ごとに窓の手続きが走る。wintf は crates.io に出すライブラリなので、areka の外の使い手が公開の型を普通に使う場合も「起こりうる」と数えた（areka で起きない件はその旨を各件に書いた）。

前提として確かめた作り: 窓の手続きの入口（`runtime/wndproc_bridge.rs:91`）は、World が書き込みのために借りられている間はハンドラを呼ばない。そのため範囲の中の「World を借りられない」枝に来るのは、読み取りの借用中だけである。

---

## 3. 指摘の一覧（要件 1.3〜1.5）

### 件数

| 組 | 件数 | 番号 | うち範囲の外へ直しが及ぶもの | うち「直さずに残してよい 3 種類」の候補 |
|---|---:|---|---|---|
| 必ず直す | **6** | 1〜6 | 2（1・6） | 1（3＝⑴ に近い既知の限界） |
| 直したほうがよい | **47** | 7〜51・62・63 | 5（11・15・28 を消す場合・62 の ⒜・32 の `api.rs` は任意） | 2（28・30 の一部＝⑴） |
| できれば | **10** | 52〜61 | 1（52） | 0 |
| 合計 | **63** | | | |

- 読み手 9 人の素の件数は 63。同じ根の 2 組を 1 件にまとめた: G6-2 と G7-5（組の実窓テストの下ごしらえの写し）→ 32 番、G1-8（③ を指す古い行番号）→ ③ を消す 41 番に含めた。これで 61 件。
- 62・63 番は、同じウェーブで走る `audit-wintf-input`（範囲＝wintf の `ecs/drag/`・`ecs/pointer/`・`ecs/clickthrough/`）のセッションが、こちらの範囲の中で見つけて「あちらでは直さない」と渡してきた 2 件を、取りまとめの側でコードを読んで確かめ、同じ物差しで指摘にしたもの（番号は末尾に足した。組は「直したほうがよい」）。
- 番号は audit の番号（要件 1.5）。以下「範囲の外」は木全体を `git grep` で探した結果。「残してよい種類」は ⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印 ⑶ SSP との互換や ukadoc の正典に関わる振る舞いの変更。
- 印: 範囲の中に `shortcut:`・`ponytail:`・`TODO`・`FIXME`・`#[deprecated]` は 0。`#[allow(dead_code)]` は 1 つ（`window/zorder_pair.rs:202`・30 番で扱う。理由の説明が無いので ⑵ には当たらない）。印の全部は 5 節。

### 必ず直す（6 件）

**1. UI スレッド以外から積んだ窓の書き込みが黙って消える**
- 場所: `window/command.rs:223-226`（`WINDOW_POS_COMMANDS` の `thread_local!`）・`:656-680`（`SetWindowPosCommand::enqueue`）
- 何が悪いか: キューはスレッドごとで、流す側（`runtime/tick_bridge.rs:252`・`world/vsync.rs:54`・`window_proc/window_pos.rs`）は UI スレッドだけ。公開の `enqueue` の説明にこの制約が無く、確かめる所も記録も無い。
- こうなると困る: 使い手が普通のシステムを `Update`（単一スレッドに固定してあるのは `world/mod.rs:41` の 6 つだけで、`Update` は多スレッドの実行器）に足して `enqueue` を呼ぶと、計算用のスレッドで走った tick は窓が動かず、Z を伴う指令はそのスレッドのキューに溜まり続ける。areka の呼び手（`zorder_pair_maintain.rs:530`・`graphics/systems/window_pos.rs:110`・`areka-placement` の `follow/window_move.rs:728`・`areka/examples/mock-shell.rs:403`）はどれも UI スレッドなので areka では起きない。
- 直し方の案: `enqueue` の説明に「UI スレッドから呼ぶ」を書き、`WinApp::run` の頭で UI スレッドの id を `OnceLock<ThreadId>` に控え、控えがあって今のスレッドと違うときは `error!` を残して捨てる。控えの無いヘッドレスのテストは今どおり。キューをプロセスで 1 本にする案は、テストがスレッドごとの隔たりに頼っているので取らない。
- 範囲の外: `runtime/mod.rs`（`WinApp::run` に 1 行・完了した `audit-wintf-runtime` の持ち物）。
- 残してよい種類: 当たらない。印: 公開の口の振る舞いが変わる（積み方の制約が増える。今の使い手は全部 UI スレッドで影響なし）。

**2. 位置か大きさの片方だけを与えた枠つきの窓が、生成の直後に枠の厚みだけずれて出て、次の刻みで跳ぶ（申し送り ⑴）**
- 場所: `window/window_pos/mod.rs:377-380`（片方が `CW_USEDEFAULT` なら換算を丸ごと飛ばす素通し）。固定しているテストは `window/window_pos/tests.rs:233-247`。
- 何が悪いか: クライアント領域の座標（中身の左上）が、窓の外枠の座標としてそのまま `CreateWindowEx` へ渡る。
- こうなると困る: `crates/wintf/examples/clip_demo.rs:266-276`（`WS_OVERLAPPEDWINDOW`・位置 (500,50) だけ）と `examples/visual_tree_demo.rs:139-149` で起きる。外枠の左上が (500,50) に出るので、中身は 96 DPI でおよそ (508,81) に出て、次の刻みで `apply_window_pos_changes` が正しく換算し直して (500,50) へ跳ぶ。大きさだけのときは中身が枠の分（およそ 16×39 px）小さく出てから広がる。`runtime/window_factory.rs:14-16` の「出た後の刻みで跳ばない」が枠つきの窓では成り立たない。areka の本番の窓は枠なしの `WS_POPUP` で、位置と大きさを必ず両方与える（`areka-placement/src/spawn.rs`）ので差は 0。
- 直し方の案: `AdjustWindowRectExForDpi` を空の RECT で 1 回呼んで枠の 4 辺を得て（`bMenu=false` なら位置にも大きさにもよらない）、与えられた側にだけ足す。両方とも既定なら今のまま返す。`:396-397` の古い説明（「dpi=96により」）も直す。`tests.rs:233-247` を「位置だけ」「大きさだけ」の 2 本（`WS_OVERLAPPEDWINDOW`）に改め、期待値は両方を与えたときの API の値から出す。
- 範囲の外: なし（呼び手は `runtime/window_factory.rs:290` だけ。`initial_pos_flags` は今のまま正しい）。
- 残してよい種類: 当たらない（`audit-wintf-runtime` の設計の討議の裁定 3 は「その spec では変えない」で、判断をこちらへ渡している）。印: 外の使い手の枠つきの窓で、生成の直後の位置が正しい側へ変わる（0.x の変更の知らせに 1 行）。

**3. 1 本でも書けた巡は、窓ハンドル待ちの残りの付与があっても持ち越しの印を落とし、残りは計画が変わるまで張られない**
- 場所: `window/zorder_chain_apply.rs:182-183`（巡の頭で `plan.dirty = false`）・`:265-299`（持ち越すのは `!outcome.acted && outcome.handle_missing` のときだけ）
- 何が悪いか: 同じ巡で 1 本が書け、別の 1 本が窓ハンドル待ちで見送られると、`acted` が真なので印が戻らない。areka の公開（`crates/areka/src/runtime/frame/zorder_drain.rs:313-317`）は計画の中身が変わったときしか印を立てず、ハンドルが付いても中身は変わらない。
- こうなると困る: 鎖 `[a, b, c]` を公開した巡に a・b はハンドルがあり c はまだ無いと、`a←b` だけ書いて印を落とす。次の巡に c のハンドルが付いても `b←c` は張られず、c は鎖の外に残る。記録は待ち始めの `skipped reason=HandleMissing` 1 行だけ。既存テスト `an_edge_without_handles_is_skipped_with_a_reason_and_the_rest_are_linked`（`zorder_chain_apply_tests.rs:424`）はこの配置だが印を確かめていない。
- 直し方の案: 持ち越しの判定を `acted` の外へ出す（`handle_missing` なら `plan.dirty = true; *carry_announced = true;`、そうでなければ `false`）。書けた分は次の巡には帳簿に載っているので、待つ間は OS を呼ばず記録も増えない。上のテストに「印が残る」の 1 行と、「ハンドルが付いた次の巡で `b←c` を張って印が落ちる」の 1 本を足す。
- 範囲の外: なし（印を立てるのは `zorder_drain.rs:317` と本ファイルだけ）。
- 残してよい種類: ⑴ に近い。完了した `areka-P0-zorder-scope-inversion` の design.md:68 は、これを「窓を 1 枚ずつ作る経路を足すとき」に見直す既知の限界として書いている（「今は UISetup の 1 回の走査でまとめて作るので起きない」）。裁定ではなく設計の注記なので、要件の討議で確かめる（議題 1）。

**4. ハンドルの付いた窓に後から組の宣言（`KeepDirectlyAbove`）を付けると、owner が張られず記録も出ない**
- 場所: `window/zorder_pair_establish.rs:113-139`
- 何が悪いか: 確立系が動くのは「この巡に `WindowHandle` が付いた」とき（`handle_added`）だけで、宣言が新しく付いたことは見ていない。
- こうなると困る: 使い手が既にある窓 2 枚に後から `KeepDirectlyAbove { peer }` を付けると、owner は張られず `ReassertZOrder` も入らず、見送りの記録も出ない。維持系も鎖の `restore_pair`（`PairReleased` のときだけ）も拾わず、バルーンは黙って守られない（記録の無い見送り）。areka は窓を作る前に宣言を付けるので今は当たらない（`spawn.rs:593`）。
- 直し方の案: クエリを `Ref<KeepDirectlyAbove>` にして `handle_added` に `declaration.is_added() ||` を足す（1〜2 行）。「ハンドルの付いた窓で 1 巡回してから宣言を付けると、次の巡で張られる」テストを 1 本足す。
- 範囲の外: なし。残してよい種類: 当たらない。

**5. 前面窓が不可視の窓だと、沈降観測が「前面窓より背面ではない」と誤って記録する**
- 場所: `window/zorder_pair_sink.rs:201-203`。誤った前提の説明は `window/zorder_pair_sink_tests.rs:767-768`・`window/zorder_pair.rs:659`（`FrontScan` の「判定に使う前面窓は必ず可視」）。
- 何が悪いか: 手前の列を集める `measure_windows_in_front` は可視の窓しか入れないので、不可視の前面窓は列に現れず、走査が最前面まで届いて `is_behind_foreground` が `Some(false)` を返す。
- こうなると困る: 通知領域のアイコンを右クリックしたときの決まった作法（隠れた自分の窓を `SetForegroundWindow` してからメニューを出す）を取るアプリで、バルーンが非活性になると `behind_foreground=false` が出る。実機の確かめの手順（完了 spec `areka-P0-ghost-window-zorder` の signoff-procedure.md:177-181）は「他アプリの窓なのに false なら FAIL」と読むので、偽の失敗になる。窓の振る舞いは変わらず、誤るのは記録だけ。
- 直し方の案: `sink.rs:203` で不可視の前面窓は「判定できない」に倒す（`foreground.filter(|&h| is_window_visible(h))`）。記録は `behind_foreground=-` になり、`foreground=` 欄は残る。純関数に「前面窓が可視か」の引数を 1 つ足して表へ 1 行足せば決定論で固定できる。`sink_tests.rs:768`・`zorder_pair.rs:659` の説明も直す。
- 範囲の外: なし（`measure_windows_in_front` の使い手は `zorder_chain_apply.rs:881` と本ファイルだけで、走査の側は変えない）。残してよい種類: 当たらない。

**6. マウスの離脱（`WM_MOUSELEAVE`）を一度取りこぼすと、その窓では二度と離脱が来ない**
- 場所: `window_proc/mouse_move.rs:157-189`（`needs_tracking`）・`:523-528`
- 何が悪いか: `TrackMouseEvent(TME_LEAVE)` を呼ぶのは `WindowPointerTracking` の旗が偽のときだけで、旗を偽へ戻すのは `WM_MOUSELEAVE` の受け手だけ。`TrackMouseEvent` の結果も捨てたまま旗だけ真にする（`:169`）。
- こうなると困る: 利用側のシステム（World を借りている間）が `MessageBoxW` などの入れ子のメッセージループを回すと、その間の `WM_MOUSELEAVE` は入口で黙って捨てられる。旗は真のまま残り、離脱の見張りは二度と掛からず、その窓の部品の `PointerState` は外れず `OnPointerExited` も出ない。areka は `CursorQuery` を差していて `cursor/answer.rs:217-227` が `WM_SETCURSOR` ごとに掛け直すので症状は隠れている。wintf だけの使い手では起きる。
- 直し方の案: `WM_MOUSEMOVE` で毎回 `TrackMouseEvent(TME_LEAVE)` を呼び、失敗は `cursor/answer.rs:225` と同じ形で `debug!` に残す（システムコール 1 回。answer.rs は既に毎回呼んでいる）。旗の判定と書き込みは消せる。
- 範囲の外: 型の定義 `ecs/pointer/types/mod.rs:206`・テスト `ecs/pointer/types/tests.rs:122-125`（同じウェーブで走っている `audit-wintf-input` の持ち物。型を消すなら要件 6.2 の申し合わせが要る）。使い手は wintf の中だけ（`crates/areka*`・examples・`crates/*/tests` は 0）。
- 残してよい種類: 当たらない。印: 型を消すなら公開の口が変わる（`wintf::ecs::WindowPointerTracking`・`ecs/mod.rs:35` から再公開）。型を残すなら、書くだけで誰も読まない欄になる（議題 4）。

### 直したほうがよい（47 件）

#### 窓の書き込みと窓の部品（`window/command*`・`components.rs`・`window_handle.rs`・`window_system.rs`）

**7. 同じ窓が 2 度入ったバッチは、1 本ずつ当てたときと Z の結果が違いうる**
- 場所: `window/command.rs:385-446`（`apply_as_batch`）・説明 `:254-259`・`:686-691`、対テスト `command_batch_tests.rs:659-718`
- 何が悪いか: `DeferWindowPos` は同じ窓の 2 本目以降を既存の 1 項目へ畳む（`hwndInsertAfter` は後着が勝ち、位置は前の項目の位置で当たる。Wine と ReactOS の実装の読み。実際の Windows は未確認）。一方、合流（`find_merge_target`）は Z の指令を仕切りにして同じ窓の指令を 2 本残す。「適用の並びも引数もキューのまま」の説明が成り立たない列がある。対テストは別々の窓 3 枚しか見ていない。
- こうなると困る: 1 回の流しが `[A を B の後へ][C を A の後へ][A を D の後へ]` のとき、1 本ずつなら C は A の直後に残らないが、バッチでは C が A の直後に付く。`WindowPos` の Z 指定を 2 枚に付けた使い手と組の維持が同じ tick に重なると出る。areka は Z 指定を本番で使っていないので出ない。
- 直し方の案: まず 3 本の列を実窓でバッチと 1 本ずつで比べるテストを 1 本足して、Windows の振る舞いを確かめる。違えば、同じ窓が 2 度ある列は `apply_sequentially` へ回し（`debug!` 1 行）、説明を直す（議題 6）。
- 範囲の外: なし。残してよい種類: 当たらない。

**8. `WindowHandle` が外れたときに投げる `WM_CLOSE` は旧い経路の名残で、窓を壊す仕事をしていない**
- 場所: `window/window_handle.rs:257-283`（`:278-281` の `let _ = PostMessageW(..WM_CLOSE..)`・`:273` の「必要ならメッセージ送信」）
- 何が悪いか: 今の窓の破棄は登録簿の掃除（`runtime/window_registry.rs:116` の `reconcile_window_registry`）が `RemovedComponents<Window>` を見て行う。投げた `WM_CLOSE` は捨てられるか、`lifecycle.rs:114-119` の破棄済みの打ち切り（`[despawn-skip]`）になるだけ。投函の失敗は記録なしで捨て、`App::on_window_destroyed` の戻り値も捨てている。
- こうなると困る: 実体を残して `WindowHandle` だけを外すと、次のメッセージで `WM_CLOSE` が届き、`OnCloseRequest` の無い実体は丸ごと despawn される（`lifecycle.rs:128`）。外した側の思いと違う。areka の本番にこの外し方は無い（テストだけ）。
- 直し方の案: `PostMessageW` の 3 行と「ウィンドウクローズを非同期で要求」の説明を消し、`:270` の debug の文と `:273` の説明を今の形に直す。`forget_last_shown` と `on_window_destroyed` は残す。
- 範囲の外: なし（範囲の中の `window_proc/lifecycle.rs:80-87` の説明の追従が要る）。
- 残してよい種類: 当たらない。印: C20 の約束 ⑶ の行（`lifecycle.rs:34`・`:38` の `DESPAWNED_SKIP_TAG` とその説明）には触れない。直すのは別の段落（`:80-87`）。

**9. 枠のある窓の座標の逆変換（窓の外枠→クライアント）にテストが無い**
- 場所: `window/window_handle.rs:111-134`（`window_to_client_rect`）・`:176-200`
- 何が悪いか: 逆変換は「原点の 100×100 で正変換して差を引く」形で、`WM_WINDOWPOSCHANGED` ごとに呼ばれる。既定の `WS_POPUP` では差が 0 なので、符号を取り違えても今のテストは 1 本も赤にならない。
- こうなると困る: `:129-132` の引き算の符号を誤って直すと、`WS_OVERLAPPEDWINDOW` の窓はドラッグのたびに縮むか膨らむ（`components.rs:202-205` が書いている過去のバグと同じ種類）。
- 直し方の案: 実窓（`WS_OVERLAPPEDWINDOW`）を 1 枚作り、`client_to_window_coords` から `window_to_client_coords` へ戻すと元の値に戻ることを確かめるテストを 1 本足す。
- 範囲の外: なし。残してよい種類: 当たらない。

**10. `WindowStyle::from_hwnd` は誰も呼ばず、`WindowHandle::get_style` と同じ仕事をしている**
- 場所: `window/components.rs:216-223`
- 何が悪いか: `git grep -w from_hwnd` は定義の 1 か所だけ（テスト・examples・`crates/*/tests` を含めて）。中身は `get_style`（`window_handle.rs:45-67`）と同じ 2 回の `GetWindowLongPtrW`。説明の「新しい WindowStyle を作成」も中身と合わない。
- こうなると困る: 同じ読み取りが 2 つあり、片方だけ失敗の扱いを変えると食い違う。
- 直し方の案: `impl WindowStyle { from_hwnd }` を消す（約 8 行）。
- 範囲の外: なし。残してよい種類: 当たらない。印: 公開の口が消える（使い手 0）。

**11. `WindowHandle.instance` は書くだけで誰も読まない**
- 場所: `window/window_handle.rs:22-25`
- 何が悪いか: `.instance` の読み取りも分解の束縛も木全体で 0。値を入れるのは `runtime/window_factory.rs:235-237` の `get_instance_handle()` だけで、ほかはテストの偽の窓が `HINSTANCE::default()` を詰めている。
- こうなると困る: 偽の窓を作るたびに意味の無い欄を書かせ、`unsafe impl Send/Sync` の理由（`:27-34`）にも余計な型が入る。
- 直し方の案: 欄を消し、factory の `get_instance_handle()` も消す。
- 範囲の外: `WindowHandle {` を組むファイルが **65 本**（wintf 31・`crates/areka` 20・`crates/areka-placement` 14。`runtime/window_factory.rs` を含む）。`areka-placement` は同じウェーブで走る `audit-placement-core`・`audit-placement-follow` の持ち物なので、要件 6.2 の申し合わせが要る（議題 3）。
- 残してよい種類: 当たらない。印: 公開の口が変わる（公開の型の欄が消える）。

**12. `flush_window_pos_commands` は `SetWindowPosCommand::flush` を呼ぶだけの包み**
- 場所: `window/command.rs:810-813`
- 何が悪いか: 同じ仕事の口が 2 つある。本番の呼び手は自由関数だけで、メソッドを呼ぶのはテストだけ。
- こうなると困る: 読み手が 2 つの口の違いを探す（違いは無い）。
- 直し方の案: `flush` の本体を `flush_window_pos_commands` へ移しメソッドを消す。テストの 4 か所（`command.rs:936`、`command_transition_tests.rs:313・411・429`）・構造の検査の `code.contains("pub fn flush()")`（`command_transition_tests.rs:516`）・説明の参照（`command.rs:823` ほか）を直す。41 番（③ を消す）・G8 の `window_pos.rs` の呼び出しと一緒に直す。
- 範囲の外: なし（`SetWindowPosCommand::flush` は command 系の中だけ）。残してよい種類: 当たらない。印: 公開の口が消える（使い手 0）。

**13. 窓の書き込みのテストの道具が 3 つのファイルに写しで並んでいる**
- 場所: `fake_hwnd`＝`command_batch_tests.rs:86-95`・`command_coalesce_tests.rs:54-64`・`command_transition_tests.rs:85-101`。`clean_slate`・`transition_lines`・`lines_of_kind`・`emit_control`・`SIGNOFF_DIRECTIVES`・`CONTROL_LINE`＝`command_batch_tests.rs:52-83` と `command_transition_tests.rs:60-123`（`emit_control`・`SIGNOFF_DIRECTIVES` は coalesce にも）
- 何が悪いか: 一緒に変えなければならない写し。
- こうなると困る: `tag | 1` で 0x20 と 0x21 が同じ値に潰れる不具合（`command_transition_tests.rs:88-91`）は、写しの全部で別々に直す必要があった。次に 1 つだけ直すと、別の窓のつもりの呼び分けがまた同じ窓を指す。
- 直し方の案: 3 本が共有するテスト用のモジュール（`#[cfg(test)]`）へ 1 つずつ移し、写しを消す。
- 範囲の外: なし。残してよい種類: 当たらない。

#### DPI・モニター・窓の位置・帯・DPI の記録（`window/dpi.rs`・`monitor.rs`・`window_pos/`・`window_band*`・`transition_diag*`）

**14. 使われていない公開の `WindowPos::set_window_pos` が、本番の道と違うやり方で窓を書く**
- 場所: `window/window_pos/mod.rs:304-322`（あわせて `build_flags` と `build_flags_for_system` の素通し `:232-289`）
- 何が悪いか: クライアント領域の座標を換算せずに `SetWindowPos` へ渡し、`guarded_set_window_pos` を通らないので自分で書いた印（`is_self_initiated`）が立たない。
- こうなると困る: 外の使い手が呼ぶと、枠つきの窓は枠の分ずれ、戻る `WM_WINDOWPOSCHANGED` が外からの変化として扱われて `WindowPos` の書き戻しと `Changed` が余計に起きる。
- 直し方の案: `set_window_pos` を消す。残る `build_flags` は呼び手が 1 つになるので、本体を `build_flags_for_system` へ移して私有の方を消す。
- 範囲の外: なし（`.set_window_pos(` は examples・tests・pilot を含めて 0）。残してよい種類: 当たらない。印: 公開の口が消える（使い手 0）。

**15. モニターの列挙の失敗が「0 台」と区別できず、失敗の中身も捨てている**
- 場所: `window/monitor.rs:241-256`
- 何が悪いか: `EnumDisplayMonitors` が失敗すると空の `Vec` を返す。1 台ぶんの失敗は `warn!` だけで、`MonitorError` の値（どの API が失敗したか）を捨てる。
- こうなると困る: 表示の構成が変わった刻みに列挙が失敗すると、`layout/systems/monitor_systems.rs:341-347`（`apply_monitor_snapshot`）が全部の `Monitor` 実体を消し、次の構成の変わり目までモニターの表が空のままになる。
- 直し方の案: monitor.rs の 2 か所を `error!`（`error = %e`）にする。呼び手の `detect_display_change_system` では空の列挙を失敗とみなし、表を変えずに戻る（旗も下ろさず次で読み直す）。`areka-placement` の `lib.rs:883` は既に 0 台を失敗として扱っている。
- 範囲の外: `crates/wintf/src/ecs/layout/systems/monitor_systems.rs:202-222`（完了した `audit-wintf-layout-widget` の持ち物・走っているセッションは無い）。公開の形は変えないので、ほかの呼び手（`areka-placement`・examples・tests・`areka` の `ghost_session.rs`・collision-probe）はそのまま。
- 残してよい種類: 当たらない。印: C20 に触れない（`Monitor` 型は変えず、自由関数の中だけ）。

**16. `DpiSuggestedRectPolicy::ApplyPosition` の説明が最後の着地点を言い当てていない（申し送り ⑶ の残り）**
- 場所: `window/dpi.rs:138-150`
- 何が悪いか: 「OS 提案位置を `SWP_NOSIZE` で適用する（Per-Monitor v2 の標準応答）」とあるが、直後の `WM_WINDOWPOSCHANGED` で中心を保つ補正（`window_proc/dpi_helpers.rs` の `correct_position_for_dpi_center_preserve`）が掛かり、左上は提案から「大きさの差の半分」ずれる。
- こうなると困る: `taffy_flex_demo` で Window 1 を (-2304,355) へ送ると (-2104,530) に着き、説明を読んだ使い手は提案どおりの左上を期待して迷う。
- 直し方の案: 説明に 1〜2 行足す（「採った後に中心を保つ補正＝`wintf-dpi-window-center-preserve` が掛かる。左上を保ちたい窓は `ExternalAuthority` を付けて自分で置く」）。振る舞いは変えない。
- 範囲の外: なし。残してよい種類: 当たらない（説明だけ）。

**17. `SetWindowParentToLayoutRoot` の説明が、ありもしない後の手当てを約束している**
- 場所: `window/window_pos/mod.rs:448-449`
- 何が悪いか: 「後のフレームで `ensure_window_parent_system` が処理する」とあるが、その系は木のどこにも無い（`git grep` で 0）。
- こうなると困る: `LayoutRoot` の無い World で `Window` を作った使い手は、親が付かず置き場の決まらない窓を記録の無いまま抱え、説明を信じて原因を探し損ねる。本番では `EcsWorld::new`（`world/mod.rs:130`）が先に `LayoutRoot` を作るので起きない。
- 直し方の案: 説明を「`LayoutRoot` が無ければ親を付けない（`EcsWorld::new` が先に作る前提）」に直し、`debug!` を 1 行残す。
- 範囲の外: なし。残してよい種類: 当たらない。

**18. monitor.rs の冒頭の使用例が無いパスを使っている**
- 場所: `window/monitor.rs:15-24`
- 何が悪いか: `use wintf::ecs::monitor::{...}` とあるが、実際は `wintf::ecs::window::monitor`（または `wintf::ecs::Monitor`）。`rust,ignore` なのでコンパイルで見つからない。
- こうなると困る: crates.io の使い手が例を写すとすぐにコンパイルの誤りになる。
- 直し方の案: パスを直す（`ignore` を外して doctest にできればなおよい）。
- 範囲の外: なし。残してよい種類: 当たらない。

**19. 帯の実窓テストの門と窓を作る手伝いが、ほぼ同じ写しで並んでいる**
- 場所: `window/window_band_real_tests.rs:27-73,121-126` と `window/window_band_pipeline_real_tests.rs:34-81,105-117`
- 何が悪いか: 門の定数（`AREKA_WINTF_REAL_WINDOW_ZORDER`）・`require_explicit_real_window_run`・`create_ghost_shaped`・`destroy`・`in_band`・`relative_order` が 2 つのファイルに写してある（3 つ目の写しが `zorder_pair_maintain_always_on_top_tests.rs:77-169`）。
- こうなると困る: 本番のゴーストの窓の拡張スタイルや門の環境変数の名を片方だけ直すと、もう片方は古い形の窓を確かめ続け、緑のまま気付けない。
- 直し方の案: どちらも `window_band` の子のモジュールなので、`window_band_real_tests.rs` の手伝いを `pub(super)` にして pipeline 側は `use` する。
- 範囲の外: なし。残してよい種類: 当たらない。印: A-1 と重なる（同じ門の系統。3 つ目の写しは A-1 のファイル＝議題 9）。

**20. transition_diag の語の数の説明が古い**
- 場所: `window/transition_diag.rs:81-84`・`:98-99`、`window/transition_diag_tests.rs:5`
- 何が悪いか: 「wintf の 5 語（…＝`KIND_ALL`）」とあるが今の `KIND_ALL` は `windpi` を入れて 6 語。98 行の上位の語の列に `offset` が抜けている。テストの冒頭は「5 種のレコード」だが 6 種。
- こうなると困る: 判定の側へ語を足す人が説明の数を信じて `windpi` や `offset` を漏らす。
- 直し方の案: 数と列を今の定数に合わせる（3 か所・説明だけ）。
- 範囲の外: なし。残してよい種類: 当たらない。印: C20 に触れない（`MonitorRecord` は変えない）。

**21. DPI のテストの説明が、Rust の `.round()` を偶数丸めだと誤って伝える**
- 場所: `window/dpi.rs:237`
- 何が悪いか: 「（`.round()` の偶数丸めではない）」とあるが、`f32::round` は 0 から遠い側への丸めで、偶数丸めは `round_ties_even`。コードは正しい。
- こうなると困る: 読んだ人が `to_physical_x` の `.round()` を別の丸めへ「直して」しまいかねない。
- 直し方の案: 「1.5 は 0 から遠い側へ（`f32::round`。偶数丸めの `round_ties_even` ではない）」に直す。
- 範囲の外: なし。残してよい種類: 当たらない。

#### 重なり順の鎖（`window/zorder_chain*`）

**22. OS への窓口の数の説明が「5」と「6」で食い違い、6 本目を数え落としている**
- 場所: `window/zorder_chain_apply.rs:66`・`:803`・`:888`、`window/zorder_chain_apply_tests.rs:5`（正しく 6 と書くのは `zorder_chain_apply.rs:68-69`・`:906`）
- 何が悪いか: 窓口は `os_read_owner`・`os_clear_owner`・`os_set_owner`・`os_raw_next`・`os_nudge`・`os_measure_front` の 6 本。「5」の説明は `os_raw_next` を数え落としている。
- こうなると困る: 替え玉を直す人が「5」を信じて `os_raw_next` を替え玉から外すと、決定論テストが実 Win32 を呼ぶ形になる。
- 直し方の案: 4 か所の「5」を「6」にし、apply_tests.rs:5 の列に「生の 1 つ奥の読み取り」を足す。
- 範囲の外: なし。残してよい種類: 当たらない。

**23. 鎖の説明の中の場所と、持ち越しの印の意味が古い**
- 場所: `window/zorder_chain.rs:547`（呼び手を「areka の `emo2_boot/frame/zorder_drain.rs`」と書く）・`window/zorder_chain_apply.rs:40`（前例を `window_proc/window_pos.rs:475-493` と書く）・`window/zorder_chain.rs:115`（`dirty` を「適用系が読んだら false へ戻す」と書く）
- 何が悪いか: `emo2_boot` は木に無い（本当は `crates/areka/src/runtime/frame/zorder_drain.rs`）。`window_pos.rs` の前例は今 488〜515 行。`dirty` は、ハンドル待ちの巡では適用系が true へ戻す（`zorder_chain_apply.rs:284`）。
- こうなると困る: 読み手が辿れない場所を探す。公開の型 `ZOrderChainPlan` の使い手が「適用系が読めば必ず false」と思い込み、持ち越しを見落とす。
- 直し方の案: パスを直し、行番号は関数名に替え、`dirty` の説明に持ち越しの条件を足す（3 番を直すならその条件に合わせる）。
- 範囲の外: なし。残してよい種類: 当たらない。印: 公開の口の説明だけ（形は変わらない）。

**24. `settled` の記録の行の説明が、後押しで動かす窓を逆に書いている**
- 場所: `window/zorder_chain_diag.rs:272-273`
- 何が悪いか: `nudged_hwnd` を「鎖の先頭」、`insert_after` を「鎖の 2 番目」と書くが、実際に動かすのは鎖の根（`members` の末尾）で、挿入位置は根の 1 つ手前（錨）か先頭（`zorder_chain.rs:451-482` の `nudge_command`・`zorder_chain_apply.rs:758-764`）。説明の形は、研究ノート §13.2 で撤回された初版の形そのもの。
- こうなると困る: 実機のログ `[zorder-chain] settled nudged_hwnd=… insert_after=…` を読む人が根の窓を先頭と取り違え、「後押しが撤回済みの形に戻っている」と誤って診断する。書式の固定テスト（`zorder_chain_diag_tests.rs:440-452`）の例の値も先頭（0xA0）と 2 番目（0xB0）の形のまま。
- 直し方の案: 2 行を「後押しで差し直した窓（鎖の根）」「その挿入位置（錨＝根の 1 つ手前。錨の直後が既に根なら先頭）」に直し、テストの例の値を `nudged_hwnd=0xC0 insert_after=0xB0` に揃える（字面 1 行）。
- 範囲の外: なし。残してよい種類: 当たらない。

**25. 鎖の実窓テスト 4 本の足場が、ほぼ同じ写しで 2 重に置かれている**
- 場所: `window/zorder_chain_order_outsider_tests.rs:81-119`（`chain_schedule`・`spawn_window`・`publish_full_chain`）＝`zorder_chain_order_tests.rs:547-585` と字面まで同じ。`zorder_chain_order_inversion_tests.rs:136-149`（`place_after`）＝`zorder_chain_order_tests.rs:373-386`。`zorder_chain_order_inversion_tests.rs:53-58／62-87／152-161`（`schedule`・`spawn_scopes`・`balloon_offsets`）＝`zorder_chain_order_lifecycle_tests.rs:182-187／99-125／198-209` を少し変えた写し
- 何が悪いか: 同じ足場が 2 か所にあり一緒に直さないと食い違う。outsider の説明は「足場は兄弟から借りる」と書くのに写している。
- こうなると困る: schedule に本番の並び（`establish_owner_links → apply_window_band → apply_zorder_pair_maintenance → apply_zorder_chain`・`areka-placement/src/spawn.rs:722-727`）を写す直しを片方だけに入れると、もう片方の檻は古い配置のまま緑になり続ける（初版が落ちた原因と同じ形）。
- 直し方の案: `zorder_chain_order_tests.rs` の 4 つと lifecycle の 3 つを `pub(super)` にし、outsider と inversion は `use` で借りる（約 90 行減る）。
- 範囲の外: なし。残してよい種類: 当たらない。

**26. 欄の値が無いときの印（番兵）と、真偽の欄の書き方が、組の記録と 2 重にある**
- 場所: `window/zorder_chain_diag.rs:50`（`UNKNOWN`）・`:174-179`（`tristate_field`）＝`window/zorder_pair_diag.rs:27`・`:57-62`
- 何が悪いか: 説明（`zorder_chain_diag.rs:28`・`:49`）自身が「既存ペア機構と同じ字面」と書き、一緒に変わる前提なのに定義が 2 つある（`hwnd_field` だけは借りている）。
- こうなると困る: 組の番兵を `-` から変えると、鎖の行の中で `owned_hwnd=`（借りている）は新しい字、`segment=`（自前）は古い字になり、番兵を数える手順が欄ごとに外れる。
- 直し方の案: `zorder_pair_diag.rs` の 2 つを `pub(crate)` にし、鎖の側は `hwnd_field` と同じ行で `use` して自前の 2 つを消す。
- 範囲の外: なし。残してよい種類: 当たらない。

**27. タグの名簿の説明が、テストの外にも読み手がいると誤って書いている**
- 場所: `window/zorder_chain_diag.rs:94-95`
- 何が悪いか: 「`chain_record_tags` は隣接の spec と実機の確かめの道具が読む名簿なので変えない」とあるが、`#[cfg(test)]` で使い手は同じクレートのテスト 3 本だけ（`zorder_chain_diag_tests.rs`・`zorder_chain_tests.rs`・`zorder_chain_release_tests.rs`）。実機の手順（`signoff-scan.ps1`）はタグの字面を自前で持つ。
- こうなると困る: 名簿を変えると実機の道具が壊れると思い込み、逆に ps1 の字面を直し忘れても名簿のテストが守ると誤解する。
- 直し方の案: 1 行を「既存の 7 語の名簿は、隣接の spec が字面を固定したテストが読む（実機の手順は字面を自前で持つ）」に直す。
- 範囲の外: なし。残してよい種類: 当たらない。

#### 重なり順の組（`window/zorder_pair*`）

**28. 案 B（`ExplicitMaintenance`）と `raise_assist: true` は、説明どおりに動く仕組みが無い**
- 場所: `window/zorder_pair.rs:163-174`（説明 170・172 行）・`window/zorder_pair_establish.rs:145`・`window/zorder_pair.rs:344`
- 何が悪いか: 説明は案 B を「B2 検知＋B3 同乗の明示維持」、`raise_assist` を「G7 FAIL 時に有効化」と書くが、重なりの変化を知らせる仕組みは 1 つも無い（`ReassertZOrder` の説明 118〜126 行に「この供給者は存在しない」）。案 B だと確立系は owner を張らず、初めの `ReassertZOrder` も入れない。
- こうなると困る: 説明を信じて `ZOrderPairStrategy::ExplicitMaintenance` を入れた使い手の組は一度も直されず、キャラを活性化してもバルーンは他アプリの窓の奥に残る。`raise_assist: true` は `false` と同じ動き。
- 直し方の案: ⒜ いちばん小さいのは説明を直す（「案 B は owner を張らず、`ReassertZOrder` が挿されたときだけ直す。z の変化を知らせる仕組みは無く、`raise_assist` は判断の表にだけ効く」・344 行と establish の説明も合わせる）。⒝ 消すなら `ExplicitMaintenance`・`raise_assist`・`RaisedAbove`／`RaisedBelow` をまとめて消す。
- 範囲の外: ⒝ のときだけ及ぶ＝`crates/areka-placement/src/diag.rs:517-520`・`spawn.rs:713`・`diag_tests.rs:325-343,487`・`spawn_zorder_pair_export_tests.rs:104-114`・`spawn_zorder_pair_wiring_tests.rs:148,249`（同じウェーブの `audit-placement-*` の持ち物）。範囲の中でも `window/window_band_pipeline_real_tests.rs:155`（owner を張らない形として案 B を使う）・`zorder_pair_establish_tests.rs:412`。
- 残してよい種類: ⒝ は **⑴**（完了 spec `areka-P0-ghost-window-zorder` の要件 5.6 と design が実行時の案の切り替えを残している）。⒜ は ⑴ に触れない（議題 2）。印: ⒝ なら公開の口が変わる（`ZOrderPairStrategy` は `wintf::ecs` から pub use）。

**29. 「ハンドルが変われば確立をやり直す」を見張るテストが無い**
- 場所: `window/zorder_pair_establish.rs:183-190`（約束は 38〜41 行）
- 何が悪いか: 説明は「窓が作り直されて別のハンドルが付いたら確立をもう一度試みる」と約束するが、`records_failure_once_and_does_not_retry_on_the_same_handles` が試すのは同じハンドルの組と「印を外した場合」だけ。
- こうなると困る: 比べる行を `failed.is_some()` に書き換えても全テストが緑のままで、一度失敗した組は窓を作り直しても二度と張られない。areka の本番は `WindowHandle` を付け直さないので、当たるのは外の使い手。
- 直し方の案: null のハンドルで失敗させたあと実窓のハンドルに付け替えて 1 巡回し、`owner-established` が 1 本出ることを確かめるテストを 1 本足す。
- 範囲の外: なし。残してよい種類: 当たらない。

**30. `PairTrigger::Establish` はテストでしか作られず、`#[allow(dead_code)]` の説明は残す理由を書いていない**
- 場所: `window/zorder_pair.rs:202-212`・`:392`
- 何が悪いか: 確立系は `decide_pair_fix` を呼ばず、初めの隣接は `ReassertZOrder` 経由の `Reassert` で直す。`Establish` を作る本番コードは 0（テストの `ALL_TRIGGERS` だけ）。`#[allow(dead_code)]` の説明は使い手が無い事実を書くだけで、先送りの理由も追う spec も書いていない（⑵ の「実在の理由の説明」に当たらない）。
- こうなると困る: 読み手は確立系がこのトリガで判断を通ると思い込み、表の 1 行とテストの組み合わせの 1/4 が無い道を固定している。
- 直し方の案: `Establish` を消す（`pub(crate)` なので外へは及ばない）。392 行を `Reassert` だけにし、テストの 2 か所を直す。`RaisedAbove`／`RaisedBelow` は 28 番の決めに従う。
- 範囲の外: なし。残してよい種類: `Establish` は当たらない（`RaisedAbove`／`RaisedBelow` は 28 番と同じく ⑴）。

**31. 組の説明の古くなった行が読み手を誤らせる**
- 場所: `window/zorder_pair.rs:347-348`（ツリーに無い関数 `compute_pair_z_intent`）・`:418-419`（結線を「後続タスクの担当」・結線済み）・`:823`（「見送り 5 種」・今は `ReleasedByChain` を足して 6 種）・`:90-93`／`:102-108`（`ReassertZOrder` の挿入元に ②balloon-visibility・③ z 変化検知を並べ、実機確認を「`areka-P0-balloon-visibility` の着地後」と書く。本番で挿すのは確立系だけで、③ は G8 PASS でやらないと決まり、② を挿す側も無い）・`window/zorder_pair_tests.rs:11`（「適用系は後続タスク」）・`window/zorder_pair_deferred_vocabulary_tests.rs:321`（「ペア・グループ」・グループの仕組みは退役して今は鎖）
- こうなると困る: 読み手が無い関数や済んだ作業を探して迷い、要件 2.6 の口に呼び手が無いことも説明から読めない。
- 直し方の案: 該当する行を今の事実に合わせて消すか書き直す（コードは変えない）。
- 範囲の外: なし。残してよい種類: 当たらない。

**32. 組の実窓テストの下ごしらえが 10 本以上のファイルに丸ごと写されている**
- 場所: `zorder_pair_maintain_tests.rs:56-276`・`_multipair_tests.rs:52-188`・`_detach_tests.rs:53-184`・`_survivor_tests.rs:58-230`・`_always_on_top_tests.rs:61-274`・`_release_tests.rs:32-185`・`zorder_pair_sink_tests.rs:77-233`・`zorder_pair_measure_tests.rs:67-161,268-280,349-378`、ほかに `zorder_pair_establish_tests.rs:163`・`zorder_pair_establish_release_tests.rs`・`zorder_chain_apply_tests.rs:146`
- 何が悪いか: `create_window`・`create_test_parent`・`create_test_child`・`create_hidden_test_child`・`insert_after`・`destroy_test_window`・`fake_hwnd`・`spawn_window`・`declare_pair`・`request_reassert`・`maintain_schedule`・`pair_schedule`・`issue_control_command`・`lines_with`・`assert_line_count`・`CAPTURE_DIRECTIVES`・タグの文字列（`"[guarded_set_window_pos] Calling SetWindowPos"` は `command.rs:178` の `WINDOW_WRITE_LOG_MESSAGE` の手写し）が、ほぼ同じ中身で最大 6〜10 回ある。
- こうなると困る: 写しは既に食い違っている（キューの残りを流し切る直しは survivor だけに入り detach には無い＝39 番）。v0.2.0 の windows-rs の移行で `CreateWindowExW`／`SetWindowPos` の呼び方が変わると、写し全部を同じに直す必要があり、漏れた 1 本だけ古い作法で残る。
- 直し方の案: `#[cfg(test)]` の共有の下ごしらえを `window/` に 1 ファイル置き（`window/mod.rs` に 1 行）、各テストから `use` する。`measure_tests` の可視の前提の assert は共有側に残す。テストの中身と名前は変えない（`crates/areka/src/placement_tests/zorder_inversion_branch_coverage_tests.rs:107-112` が `zorder_pair_maintain_release_tests.rs` のテスト関数の名前を文字列で読む）。千行前後が消える見込み。
- 範囲の外: なし（`crates/wintf/src/ecs/api.rs` のテストにも写しがあるが、範囲の外なので残してよい）。
- 残してよい種類: 当たらない。印: A-1 と重なる（`zorder_pair_maintain_always_on_top_tests.rs` に触れる。`#[ignore]` と環境変数の隔離は変えない＝議題 9）。

**33. 沈降観測のテストの説明が「非表示の子窓」と書くが、実際は可視**
- 場所: `window/zorder_pair_sink_tests.rs:496-500`
- 何が悪いか: `Pair::create` は可視の親の下に `WS_VISIBLE` 付きの子窓を作る（同じファイル 105-136 行）。前面窓になり得ない本当の理由は「子窓だから」。
- こうなると困る: 読み手が「不可視だから走査に入らない」と誤解し、隣接や走査のテストの前提を取り違える。
- 直し方の案: 「子窓（トップレベル窓ではないので前面窓にならない）」に直す（1 行）。
- 範囲の外: なし。残してよい種類: 当たらない。

**34. 「記録のマクロを他のファイルへ移すと grep の対象が分裂する」という理由が誤りで、それを守るはずの検査も赤にならない**
- 場所: `window/zorder_pair_sink.rs:31-33`・`window/zorder_pair_record_tests.rs:43-45,381-384`（同じ理由が `window/zorder_pair.rs:21-22,434-437` にも）
- 何が悪いか: `EnvFilter` の絞り込みは出力先の名前の前方一致（tracing-subscriber 0.3.23 `filter/directive.rs:250` の `starts_with`）なので、`wintf::ecs::window::zorder_pair=debug` は `..._sink`・`..._diag` にも効く。実機の grep も判定語（タグ）で行う。テストの `out.contains(LOG_TARGET)` も `zorder_pair_sink` から出た行を部分一致で通す。
- こうなると困る: 誤った理由のせいで、中身を渡すだけの `log_sink_observed`／`log_orphan_sink_mark_dropped` を残す判断が続き、テストは「全部が既定の出力先へ出る」（record_tests.rs:9）を守っているように見えて守れていない。
- 直し方の案: 理由の文を事実に合わせて直すか消す。出力先を固定したいなら検査を区切りまで含めた完全一致にし、要らないなら検査を消す。
- 範囲の外: なし。残してよい種類: 当たらない（完了 spec の「出力先はモジュールの名前が既定」とは食い違わない）。

**35. 見送りの理由のテストの説明が「5 行」のまま（実際は 6 種）**
- 場所: `window/zorder_pair_record_tests.rs:263`
- 何が悪いか・困ること: `ReleasedByChain` が加わって 6 種になったのに「5 行がすべて相異なる」のまま。次に増やすときの数え合わせを誤る。
- 直し方の案: 「6 行」に直すか、数を書かない文にする。
- 範囲の外: なし。残してよい種類: 当たらない。

#### 組の維持（`window/zorder_pair_maintain*`）

**36. 本番の結線を引いた説明が古い（2 本の chain と書くが本番は 4 本）**
- 場所: `window/zorder_pair_maintain.rs:383-386`
- 何が悪いか: 説明は `areka-placement/src/spawn.rs:721-728` を指して `(establish_owner_links, apply_zorder_pair_maintenance).chain()` と引くが、本番（`spawn.rs:720-729` の `wire_zorder_pair`）は `(establish_owner_links, apply_window_band, apply_zorder_pair_maintenance, apply_zorder_chain).chain()`。結論（確立系が本 system より前に走る）は今も正しく、古いのは引いた字面だけ。
- こうなると困る: 要求を出す側を足そうとする人が、帯の系と鎖の適用系が同じフレームに居ることを知らないまま順番を決める。
- 直し方の案: 3 行を本番の 4 本の字面に直すか、字面を引かずに「`wire_zorder_pair` の chain では確立系が本 system より前」と書く（説明だけ）。
- 範囲の外: なし（本番の字面は `crates/areka-placement/src/spawn_zorder_chain_wiring_tests.rs:245-247` が固定）。
- 残してよい種類: 当たらない。印: **B-6 と重なり、B-6 はまだ生きている**。台帳 `zorder-chain-residue` の brief は「冒頭の説明は既に 3 つの仕組みを述べているので消化済みの見込み」と書くが、冒頭の 3 つ（切り離し・沈み込みの観測・維持）は最初のコミット `b06615c5f` から在り、B-6 を書いた時点（`9ed1e9ed2`・#126）も同じだった。B-6 が指していたのは関数の中のこの説明で、当時の本番は確立・維持・鎖の 3 本＝「2 系統を述べたまま（本番は 3 系統）」と合う。今は #399 で `apply_window_band` が入って 4 本になり、ずれは広がった（議題 9）。

**37. 「後続のタスクが受け持つ」「G8 で判定する」と書くが、G8 は PASS で決着し、そのタスクはやらないと決まっている**
- 場所: `window/zorder_pair_maintain.rs:66-72`・`:253-257`
- 何が悪いか: 「本モジュールが担わないもの（後続タスクの担当）」に、切り離しを早める強化と重なりが動いたことの検知（案 B）が挙がっているが、完了した `areka-P0-ghost-window-zorder` の tasks.md:371 は「G1〜G8 全 PASS、5.2〜5.7 は実施しない」と決めている。
- こうなると困る: 終了時に `owner-detached` が出ないのを調べる人が、受け持ちの居る未着手の仕事と読み、無い引受先を探す。
- 直し方の案: 見出しを「G8 PASS により足さないと決めたもの」にし、2 か所を「G8 で PASS と判定済み（tasks.md 5.2 は不実施）」に直す（説明だけ・裁定に沿わせる）。
- 範囲の外: なし。残してよい種類: 当たらない。

**38. 隔離の説明が「2 本」と書くが、隔離されているのは 3 本**
- 場所: `window/zorder_pair_maintain_always_on_top_tests.rs:70`・`:73`・`:83`（隔離は `:410`・`:788`・`:857` の 3 本）
- 何が悪いか: 説明は「実窓の重なり順を直に測る 2 本のみ」と書くが、`#[ignore]` と `require_explicit_real_window_run()` が付くのは `pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares`（:411）・`the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`（:789）・`a_pair_inside_the_band_is_fixed_directly_above_the_character_within_the_band`（:858・#399 で足された）の 3 本。:411 が見るのは帯に入っているかの印だけで重なり順は見ておらず、台帳の症状も「`HWND_NOTOPMOST` が Ok を返したのに帯から出ない」なのに、`#[ignore]` の理由の文は 3 本とも「重なり順に他の窓が割り込む」。
- こうなると困る: A-1 に着手する人が説明を信じて 2 本だけ直し、3 本目を隔離のまま残す（台帳の brief も「2 本」と書き、指す行 `:767` は今 `:789`）。
- 直し方の案: 「3 本」に直してテストの名前を並べ、:410 の理由の文を「帯から外す指令が効かない回がある」に直す（説明と属性の文だけ）。
- 範囲の外: なし。残してよい種類: 当たらない。印: **A-1 と重なる**（議題 9）。

**39. 切り離しのテストの下ごしらえが「要求を外すので指令は飛ばない」と書くが、指令はもう積まれている**
- 場所: `window/zorder_pair_maintain_detach_tests.rs:137-166`（`establish_pair`）
- 何が悪いか: `pair_schedule` は確立系と維持系を chain でつなぐので、確立系が入れた要求を同じフレームの維持系がもう使い、非表示のトップレベル窓どうしは隣と測れないので直す指令が積まれ `IssuedPairFix` も付く。その後で `ReassertZOrder` だけを外すので、キューに指令が 1 本残り `IssuedPairFix` もぶら下がる。兄弟の `survivor_tests.rs:412-417` は同じ形を範囲の外で流し切り「流し忘れると 2 本になる赤を確認済み」と書いている。
- こうなると困る: このファイルの捕捉の中に `flush` と指令の本数の主張を足すと、前のフレームの残り 1 本が混じって赤になり、説明を信じた人は原因を見失う。
- 直し方の案: survivor と同じく外した直後に `SetWindowPosCommand::flush();`（12 番の後は自由関数）を 1 行足し、説明を直す。`remove::<(ReassertZOrder, IssuedPairFix)>()` にすればぶら下がりも消える。
- 範囲の外: なし。残してよい種類: 当たらない。

**40. 対照の説明が挿入の向きを逆に書いている（「すぐ手前」と書くが実際は「すぐ背後」）**
- 場所: `window/zorder_pair_maintain_always_on_top_tests.rs:399-400`・`:459`・`:466`・`:614`・`:622`
- 何が悪いか: `pair_fix_command(x, InsertSpec::After(band_window))` は `hWndInsertAfter` の決まりで x を帯の窓の**すぐ背後**に置く（兄弟の `insert_after` の説明にも「すぐ背後（1 つ背面側）」）。
- こうなると困る: 「帯の窓の後ろに入れても帯に引き込まれる」という、この対照が証拠にする OS の性質が逆向きに読まれ、新しい対照を作る人が向きを取り違える。
- 直し方の案: 5 か所の「すぐ手前」を「すぐ背後」に直す（説明だけ）。
- 範囲の外: なし。残してよい種類: 当たらない。印: A-1 のファイル（隔離は変えない）。

#### 窓の手続きの位置・DPI・生き死に（`window_proc/window_pos*`・`dpi_helpers*`・`mod.rs`・`lifecycle*`）

**41. 錠を取らない 2 回目の流し（③）は、本番ではいつも空のキューを流すだけで、説明が読み手を誤らせる（申し送り ⑷）**
- 場所: `window_proc/window_pos.rs:295-299`（③）・`:41-44`（「3 ステップ」の説明）。同じ ③ を指す古い行番号＝`window/command.rs:22`（`:290`）・`window/command_transition_tests.rs:445`（`:268`）。③ を流しの入れ子の理由に挙げる説明＝`window/command.rs:709-714`・`window/transition_diag.rs:806-811`・`window/command_transition_tests.rs:444-446`・`window_proc/window_pos_transition_tests.rs:594-596`
- 何が悪いか: 入口（`runtime/wndproc_bridge.rs:91`）は World の書き込みの借用中はハンドラを呼ばないので、③ が働けるのは ② が錠に阻まれた場合（`tick_one_frame` の最後の流しの内側か、別の ② の内側）だけで、どちらも流しの頭でキューを全部取り出し済み。後から積むのは画面更新中に走るシステムだけ（`::enqueue(` の呼び手 3 か所）。だから ③ はいつも空のキューを見る。錠が空いていれば ② がもう流している。
- こうなると困る: 説明の「SetWindowPos 実行」「World 借用解放後なので安全」を信じた読み手は ③ に仕事があると思い込む。入口の確認が将来外れる・ハンドラが別の経路（テストの `test_wndproc`＝`window_pos_transition_tests.rs:511` には入口の借用確認が無い）から呼ばれると、③ は画面更新の途中（World を借りたまま）でキューを流し、モニターをまたぐ移動の内側で届く `WM_DPICHANGED` が World を読めず DPI が更新されず、方針を読めずに「書く」へ倒れて、外部が位置を決める窓にも OS の勧める位置が書かれる。
- 直し方の案: 299 行の呼び出しと 295-298 行の説明を消し、41-44 行を「① 借用 → ② 錠を取って流す」の 2 手順に直す。行番号の参照は「`WM_WINDOWPOSCHANGED` ハンドラの手順②」と名前で指す形にし、入れ子の理由の説明（上の 4 か所）を「入れ子は錠を取らない直の flush の内側の ② で起きる」に直す（入れ子を戻す `FlushEpoch` は公開の直の flush のために残す）。崩れを防ぐテストを 1 本足す（錠を `engage_tick_flush_guard()` で取ったまま 1 本積み、`WM_WINDOWPOSCHANGED` を配送しても `drain_window_pos_commands()` が 1 本返す。今のコードでは赤）。
- 範囲の外: なし（直す説明は全部範囲の中）。
- 残してよい種類: 当たらない（完了した `audit-wintf-graphics-world` がこの spec へ申し送った件）。印: `since_flush_us` の観測は外側の起点が残るので失われない。

**42. `WM_WINDOWPOSCHANGED` の判断に決定論テストが 1 本も無い**
- 場所: `window_proc/window_pos.rs:143`（`use_bypass`＝echo を変更の印なしで書く）・`:177-180`（値が同じなら書かない）・`:228`（`skip_box_style`＝DPI の直後は `BoxStyle` を書かない）・`:237-238`（外からの大きさの変更だけ `BoxStyle.size` を論理 px で書く）
- 何が悪いか: このハンドラを配送するテストは記録の行と戻り値しか見ず、`WindowHandle` が無いので座標の変換で抜けて判断の本体へ届かない（`WindowPos` の変更の印や `BoxStyle` を検めるテストは `git grep` で 0）。
- こうなると困る: 143 行を `is_echo` だけに変えても全テストが緑のまま＝134-142 行の説明にある無限ループ（DPI の変更が旧モニターへ戻され、また `WM_DPICHANGED`、で固まる）の形そのもの。228 行から `dpi_context.is_some()` を落としても赤にならず、DPI の変更のたびに古い物理寸を新しい倍率で割った値が `BoxStyle.size` に入り寸が縮む・膨らむ。
- 直し方の案: 兄弟ファイル 1 本にテストを 3 つ（既存の 0x0 の不可視窓と配送の型＝`window_pos_transition_tests.rs:500-585` を使う）。⑴ 素の `SetWindowPos` で `Changed<WindowPos>` が立ち `BoxStyle.size` が物理 px ÷ 倍率になる ⑵ `guarded_set_window_pos` の echo で値は変わるが印は立たない ⑶ `DpiChangeContext::set` の後で印は立つが `BoxStyle` は変わらない。
- 範囲の外: なし。残してよい種類: 当たらない。

**43. テストの説明が S1 の証跡の置き場を取り違えている**
- 場所: `window_proc/dpi_helpers_tests.rs:424-425`
- 何が悪いか・困ること: 「S1 の赤→緑の正証跡は `window_proc/mod.rs` の dispatch 檻が担う」とあるが、`s1_red_*`・`s1_*` は `window_pos_tests.rs:396-724` に在る（`git grep "fn s1_"`）。読み手が `mod.rs` を開いて見つけられず「S1 には純関数の檻しか無い」と誤って結論しうる。
- 直し方の案: `window_proc/mod.rs` を `window_proc/window_pos_tests.rs` に 1 語直す。
- 範囲の外: なし。残してよい種類: 当たらない。

#### 窓の手続きの入力（`window_proc/drop_files*`・`keyboard*`・`mouse_*`）

**44. ドラッグで窓を動かす `SetWindowPos` の失敗が記録に残らない**
- 場所: `window_proc/mouse_move.rs:445-457`
- 何が悪いか: `guarded_set_window_pos` の `Err` を `let _ =` で捨てている。
- こうなると困る: ドラッグを始めた時点で窓に `WindowHandle` が無く `move_window` が真だと、`drag/dispatch.rs:102`・`drag/state/mod.rs:383` は取っ手を `HWND::default()` にし、マウスが動くたびに空の取っ手への `SetWindowPos` が失敗して窓は動かず、ログにも何も出ない（記録の無い失敗の道＝要件 8.2）。
- 直し方の案: `if let Err(e) = … { warn!(event = "drag_move_window_failed", hwnd = ?drag_hwnd, error = %e, …) }` の 1 か所。
- 範囲の外: なし。残してよい種類: 当たらない。

**45. 「離脱を付けて `PointerState` を移す」処理が 3 か所に写してあり、もう食い違っている**
- 場所: `window_proc/mouse_move.rs:326-384`（当たった枝）・`385-438`（当たらない枝）・`496-521`（`WM_MOUSELEAVE`）
- 何が悪いか: 当たった枝と当たらない枝は相手が `hit_entity` か `window_entity` かの違いしかない写しで、`WM_MOUSELEAVE` は `collect_entities_to_leave` の中身を自前でもう一度書いている。
- こうなると困る: 既に食い違っている（当たった枝は `get_entity_mut` の失敗を `warn!` に残す＝376 行が、当たらない枝は黙る＝412 行）。片方だけ直すと空白の上か部品の上かで振る舞いが変わる。
- 直し方の案: `let target = hit_entity.unwrap_or(window_entity);` の 1 本にまとめ、離脱の付け外しは `collect_entities_to_leave` を `exclude: Option<Entity>` にして 3 か所から呼ぶ（約 60 行減る）。
- 範囲の外: なし。残してよい種類: 当たらない。

**46. ダブルクリックの受け手がふつうの押下の受け手のほぼ写しで、当たらなかったときの扱いだけ抜けている**
- 場所: `window_proc/mouse_dblclick_wheel.rs:33-149`（写しの元は `mouse_click.rs:24-216` の押下の枝）
- 何が悪いか: `PointerState` の付与・修飾キー・`record_button_down`・ドラッグ準備の決まり（`enabled && left_button`）・画面座標の式を別々に書き、ふつうの押下が当たらないときや World を借りられないときに窓の entity へ記録する所（195-197 行）が、ダブルクリックの方には無い。
- こうなると困る: `WM_LBUTTONDBLCLK` が当たらない形で届くと押下は記録されず、続く `WM_LBUTTONUP` だけが予備の枝で窓の entity に記録され「押していないのに離した」並びになる。ドラッグの決まりを変えるときも 2 か所を一緒に直さないとダブルクリックだけ古いまま残る。
- 直し方の案: `handle_button_message` に `double_click: DoubleClick` を足し（ふつうの押下は `DoubleClick::None`）、4 つの `WM_*BUTTONDBLCLK` はそれを呼ぶだけにして `handle_double_click_message` を消す（約 90 行減る）。
- 範囲の外: なし。残してよい種類: 当たらない。

**47. クライアント座標を画面座標に直す式が 3 か所にある**
- 場所: `window_proc/mouse_click.rs:54-69`・`mouse_move.rs:198-210`・`mouse_dblclick_wheel.rs:133-136`
- 何が悪いか・困ること: ドラッグの押した位置（click・dblclick）と今の位置（move）を同じ規則で別々に計算していて、1 か所だけ変える（`GetMessagePos` に替えるなど）と、差の分だけドラッグを始めた瞬間に窓が跳ぶ。
- 直し方の案: dblclick の 1 行の形を小さな関数 1 つにして 3 か所から呼ぶ（46 番を直せば写しは 2 か所）。
- 範囲の外: なし。残してよい種類: 当たらない。

**48. `WM_MOUSELEAVE` の受け手にテストが無い**
- 場所: `window_proc/mouse_move.rs:486-533`
- 何が悪いか: 窓ごとに範囲を絞る分かれ道（自分の窓の部品だけ離脱にし、ほかの窓の `PointerState` は残す）と旗の戻しはこの受け手にしか無く、壊れても落ちるテストが無い（`collect_entities_to_leave` のテストはこの受け手の写しを通らない）。
- こうなると困る: 絞り込みを外すと、バルーンの窓から出ただけでキャラの窓のホバーも消えるが、テストは緑のまま。
- 直し方の案: 窓 A・B に `PointerState` の子を 1 つずつ置き、`dispatch_window_message` で A へ `WM_MOUSELEAVE` を送り、A の子は `PointerLeave` 付きで `PointerState` が無く、B の子はそのままであることを確かめる決定論テストを 1 本（6 番・45 番を直した後の形で書く）。
- 範囲の外: なし。残してよい種類: 当たらない。

**49. OS から投げ込みの一覧を読む部分（`unsafe`）にテストが無い**
- 場所: `window_proc/drop_files.rs:87-109`（`read_dropped_paths`）・`40-47`（`FinishOnDrop`）。テスト側の説明 `drop_files_tests.rs:219-221`
- 何が悪いか: テストの説明は「Query の腕は実物の HDROP でしか踏めない」とあるが、HDROP は「`GlobalAlloc` した `DROPFILES` の頭＋2 つの NUL で終わるワイド文字のパスの並び」なので、窓を作らずにテストの中で作れる。
- こうなると困る: バッファの `+ 1` を落とすなど長さを 1 つ間違えると、どのパスも最後の 1 文字が欠ける（`.nar` が `.na`）が、今のテストは全部緑。
- 直し方の案: テストに `#[link(name = "kernel32")] extern "system" { fn GlobalAlloc(…); }` を置き（wintf に `Win32_System_Memory` の機能が無いので Cargo を触らない形）、日本語を含むパス 2 本の HDROP を作って `WM_DROPFILES` を呼び、同じ 2 本が届くことを確かめる（`DragFinish` が片付ける）。
- 範囲の外: なし。残してよい種類: 当たらない（テストを足すだけで、SSP の `OnFileDrop` 系との互換には触れない。SSP のイベントは areka の側で組む）。

**50. ファイル頭の説明がファイルの中身と合っていない**
- 場所: `window_proc/keyboard.rs:1-4`（`WM_CAPTURECHANGED` が抜け）・`mouse_move.rs:1-3`（`WM_SETCURSOR` が抜け）・`mouse_dblclick_wheel.rs:19-21`（説明の 1 行目が `#[inline]` の上に分かれ、同じ意味の行が 2 回）
- こうなると困る: 捕捉を失ったときやカーソルの受け手を探す人が、ファイル頭の一覧を見てそのファイルを飛ばす。
- 直し方の案: 一覧に足す。dblclick は 19 行目を消して `#[inline]` を説明の後ろへ移す。
- 範囲の外: なし。残してよい種類: 当たらない。

**51. ダブルクリックのテストが、ドラッグの累積器を本番と別のものに入れ替えている**
- 場所: `window_proc/mouse_dblclick_wheel_tests.rs:56`
- 何が悪いか: `w.insert_resource(DragAccumulatorResource::new())` で、`EcsWorld::new` が資源と窓の手続き側の控えへ置いた同じ実体を別物に替えている（`keyboard_tests.rs:38-40`・`mouse_click_tests.rs:24-28` はこれをしてはいけないと書く）。
- こうなると困る: このファイルでは終了・取り消しの知らせ（控えへ積まれる）が配られず、「ダブルクリック → 閾値を越える → 離すと終了が 1 件」のテストを足すと本番が正しくても 0 件で赤になる。142 行の「終了 0 件」も何も確かめていない。
- 直し方の案: 56 行を消す。
- 範囲の外: なし。残してよい種類: 当たらない。

#### `audit-wintf-input` から渡された 2 件（取りまとめの側で確かめた）

**62. ドラッグの閾値の判定は本番では窓の手続きの中の写しで行い、テストは使われない `check_threshold` だけを試している**
- 場所: `window_proc/mouse_move.rs:228-235`（`distance_sq >= threshold_sq` の手書き）。写しの元は `ecs/drag/state/mod.rs:623` の `check_threshold`。
- 何が悪いか: `check_threshold` の使い手は `ecs/drag/mod.rs:26` の再公開と `ecs/drag/state/tests.rs:380-440` のテストだけ（`git grep` で確かめた・本番の呼び手 0）。境の値・負の向き・i16 の端であふれないことのテストは、使われない写しを試していて、本番の判定（`mouse_move.rs`）を試すテストは無い。`audit-wintf-input` は使われない `check_threshold` を消す予定で、消すと本番の判定を守るテストが 1 本も無くなる。
- こうなると困る: 閾値 5 で (3,4) へ動かすと距離はちょうど 5 でドラッグが始まるはずだが、`mouse_move.rs:235` の `>=` を `>` に書き換えても全テストが緑のまま、ちょうど閾値の動きでドラッグが始まらなくなる。
- 直し方の案: ⒜ `mouse_move.rs` の 5 行を消して `check_threshold` を呼ぶ（既にあるテストが本番を守る形になる。`check_threshold` はドラッグの状態を読み直して `Preparing` でなければ warn を出すが、この腕は `Preparing` の写しに当たった後なので結果は同じ）。⒝ あちらの予定どおり `check_threshold` を消し、こちらで本番の判定に境の値のテストを置く（`window_proc` の配送のテストで、ちょうど閾値・1 手前の 2 本）。いちばん小さく完全なのは ⒜ だが、あちらの「消す」予定と逆なので申し合わせが要る（議題 11）。
- 範囲の外: ⒜ は `ecs/drag/state/mod.rs`（あちらの持ち物）の `check_threshold` を残してもらう。⒝ はなし。
- 残してよい種類: 当たらない。

**63. クリックのたびに `info!` を出し、そのための文字列を毎回組んでいる**
- 場所: `window_proc/mouse_click.rs:80-106`（`entity_name` と `bounds_info` を `format!` で組んでから `info!`「hit_test result」・押すたびと離すたび）・`:154-160`（左ボタンでドラッグの準備を始めるたびの `info!`）・`window_proc/mouse_dblclick_wheel.rs:71-77`（ダブルクリックのたびの `info!`）・`window_proc/keyboard.rs:174-177`（ドラッグ中に窓が非活性になって取り消すときの `info!`）
- 何が悪いか: 決まり（`.kiro/steering/logging.md:27-28`）は `info!` を「ライフサイクルイベント（初期化完了・終了・ディスプレイ構成変更）」に、状態の変化や操作の詳細を `debug!` に置く。既定の絞り込みは `info`（同 `:97`）なので、これらは本番のログに出る。
- こうなると困る: ゴーストを 1 回クリックするたびに本番のログへ 2〜3 行（押す・離すの当たり判定と、ドラッグの準備）が足され、よく触られるゴーストほどログが膨らんで、終了や構成の変化など本当に読みたい `info` の行が埋もれる。`entity_name`・`bounds_info` は水準に関係なく毎回 `format!` で確保される。
- 直し方の案: 4 か所を `debug!` に下げ、`entity_name`・`bounds_info` はマクロの中で組む（`debug` が開いているときだけ確保される形）。この 4 つの文言を読むテストは木の中に 0（`git grep` で確かめた）。`drop_files.rs:140` の `files_dropped`（投げ込み 1 回に 1 行・`drop_files_tests.rs:66` が水準を固定）と `lifecycle.rs` の 2 か所（閉じる・終了＝ライフサイクル）は決まりどおりなので変えない。
- 範囲の外: なし（同じ種類の `info!` を `audit-wintf-input` が `ecs/drag/dispatch.rs`・`ecs/pointer/buffers.rs` の側で `debug!` に下げる予定＝あちらの持ち物）。
- 残してよい種類: 当たらない。

### できれば（10 件）

**52. `create_windows` が毎 tick、問い合わせの状態を作り直している**
- 場所: `window/window_system.rs:41`
- 何が悪いか・困ること: `UISetup` で毎 tick 走るのに、そのたびに `SystemState::new` で QueryState を組み直す（確保と全 archetype との照合）。窓の生成が無い定常の tick にも確保が起きる小さな損。
- 直し方の案: 排他システムの引数に `pending: &mut QueryState<Entity, Pending>` を取り、直に `create_windows(&mut world)` を呼ぶテスト（`window_system.rs:76`・`runtime/window_factory_creation_tests.rs:104・111`）は `run_system_once` に替える。
- 範囲の外: `runtime/window_factory_creation_tests.rs`（完了した `audit-wintf-runtime` の持ち物）。残してよい種類: 当たらない。

**53. 鎖の適用の `outcome` の `missed` 欄は組み立てるだけで読まれない**
- 場所: `window/zorder_chain_apply.rs:260-264`
- 直し方の案: 構造体を組まずにローカル変数 `acted`・`handle_missing` の 2 つにする（-3 行ほど。3 番と一緒に直す）。範囲の外: なし。残してよい種類: 当たらない。

**54. 後押しのフラグを確かめる行が、すぐ上の行と同じことを生の数値で言い直している**
- 場所: `window/zorder_chain_tests.rs:597-602`（上の `:586-591` がフラグの全一致を既に主張）
- 直し方の案: 6 行を消し（理由の 1 行コメントは上へ移す）、`SET_WINDOW_POS_FLAGS` の import も消す。範囲の外: なし。残してよい種類: 当たらない。

**55. 走査の失敗の記録が、どの窓を測っていたかを載せない**
- 場所: `window/zorder_pair.rs:557-565`・`:696-702`
- 何が悪いか・困ること: 失敗の warn の `hwnd=` は辿っている途中の窓（`cursor`）で、上限に達したときの warn（569〜573・710〜713 行）は走査を始めた窓。同じ欄の名が 2 つの意味で使われ、途中の不可視窓（IME 窓など）が壊れると、組が複数あるときにどの組の測定が不在に倒れたのか分からない。
- 直し方の案: 失敗の行に始めた窓（`hwnd`）と失敗した窓（`at=cursor`）の 2 つを載せる（`[zorder-pair]` のタグは無いので実機の grep の語は変わらない）。範囲の外: なし。残してよい種類: 当たらない。

**56. 見送りの理由の一覧が同じテストファイルに 2 回書かれている**
- 場所: `window/zorder_pair_record_tests.rs:239-246,468-475`
- 直し方の案: 先頭に `const ALL_REASONS: [(SkipReason, &str); 6]` を 1 つ置き両方から使う（7 種目を足したとき片方だけ直して漏れる穴を塞ぐ）。範囲の外: なし。残してよい種類: 当たらない。

**57. `WA_INACTIVE`／`WA_ACTIVE` を自前の定数で書いている**
- 場所: `window/zorder_pair_sink_tests.rs:68-71`
- 直し方の案: `windows::Win32::UI::WindowsAndMessaging::{WA_ACTIVE, WA_INACTIVE}` を使う（`drag/reentry.rs:18` と同じ）。範囲の外: なし。残してよい種類: 当たらない。

**58. 組の外の関数を弱い主張で 2 度目に試しているテスト**
- 場所: `window/zorder_pair_maintain_tests.rs:865-879`（`measure_window_below_maps_failure_to_absence_and_records_it`・主張は「ログが空でない」だけ）
- 何が悪いか: 同じことを `zorder_pair_measure_tests.rs:325-336` が `FAILURE_MARK` の本数で厳しく押さえている。このファイルは 1,000 行の番人まであと 22 行。
- 直し方の案: このテストを消す（15 行減る）。範囲の外: なし。残してよい種類: 当たらない。

**59. 中身の無い入れ子のブロックが 2 段ずつ残っている**
- 場所: `window_proc/window_pos.rs:83-84` と `:300-301`、`:387-388` と `:430-431`
- 直し方の案: 2 つのハンドラから意味の無い 2 段のブロックを外す（振る舞いは変わらない。41 番と一緒に直す）。範囲の外: なし。残してよい種類: 当たらない。

**60. X ボタンの番号をボタンの種類に直す式が 3 か所にある**
- 場所: `window_proc/mouse_click.rs:364-369`・`382-387`・`mouse_dblclick_wheel.rs:217-222`
- 直し方の案: 小さな関数 1 つにまとめる（46 番の後は 2 か所）。範囲の外: なし。残してよい種類: 当たらない。

**61. `PointerState` を付けるときのボタンの初めの値は書かなくてよい**
- 場所: `window_proc/mouse_click.rs:120-125`・`mouse_dblclick_wheel.rs:91-95`
- 何が悪いか: 直後の `record_button_down`／`record_button_up` が溜めに入り、次の tick の `transfer_buffers_to_world`（`pointer/buffers.rs:203-237`）がその値で上書きする。
- 直し方の案: `..Default::default()` に任せて消す。範囲の外: なし。残してよい種類: 当たらない。

### 判定と減らせる量

- Verdict: 健全ではない。先に直すのは必ず直すの 6 件（3・4・6 は「記録の無いまま黙って守られない／戻らない」形、1・2 は外の使い手の場面、5 は実機の確かめの記録の誤り）。
- Lean: 写しのまとめ（13・19・25・32・45・46・47・60・62）と死んだ口（10・11・12・14・30・41・53・54・58）で、およそ -1,300 行（大半は 32 番のテストの下ごしらえ）。依存の増減は 0。

---

## 4. 完了した見直しからの申し送り 4 件の判断（要件 2）

| # | 事柄 | 判断 | 番号・組 | 理由 |
|---|---|---|---|---|
| ⑴ | 位置だけを指定した窓の生成のときの座標の換算（`window/window_pos/mod.rs` の `to_window_coords_for_creation` の素通し） | **指摘にする** | 2・必ず直す | 片方だけ与えた枠つきの窓は、生成の直後に枠の厚み（`WS_OVERLAPPEDWINDOW`・96 DPI でおよそ (8,31) px、大きさだけなら 16×39 px）ずれて出て次の刻みで跳ぶ。ツリーの中の `clip_demo`・`visual_tree_demo` で起きる。areka の本番の窓（枠なしの `WS_POPUP`・位置と大きさを両方与える）では差は 0。`audit-wintf-runtime` の裁定 3 は「その spec では変えない」で、判断をこちらへ渡している |
| ⑵ | 生成に失敗した窓の印 `WindowCreationFailed` を外して立ち直らせる側をどこに置くか | **指摘にしない**（wintf の側で直すことは無い） | — | 印の定義は `runtime/window_factory.rs:82-85`（`pub`・`wintf::WindowCreationFailed`）、付けるのは `:105-109` の `mark_creation_failed`（`LibWindow::new_ex` が `Err` のときだけ）。外す口は開いている（公開の Component なので使い手が `remove` すれば次の tick に作り直す＝`window_factory_creation_tests.rs:90-113` が固定）。失敗の理由は外部クレート `wintf-winmsg-executor-0.0.5`（`src/util/window.rs:86-94`）によると `WM_NCCREATE`／`WM_CREATE` の拒否・窓ハンドルの上限・デスクトップのヒープの枯渇や外部のフックで、前 2 つは wintf の中からは起きず（生成中は World を借りているので wndproc は既定へ流れる）、上限は一時的でない。一時的になりうるのはヒープの枯渇と外部のフックだけで稀。外部クレートが原因を捨てる（`map_err(|_| …)`）ので、wintf は原因と結び付く「やり直してよい合図」を持たず、自動で外しても同じ失敗を繰り返すだけ。案は 3 つ: **A 何もしない（今の「使い手が外せば作り直す」のまま）**・B 表示の構成や DPI の変化で印を全部外す（原因とつながらない）・C 有界の回数・間隔の作り直し（状態と時刻が増え、`instant-to-interrupt-ticks` とも重なる。外部クレートが失敗のたびに箱を解放するか未確認）。利用者へ失敗を伝える・ゴーストの切り替えで作り直すのは areka の持ち物（直すなら `/kiro-discovery` で起票する候補）。議題 7 |
| ⑶ | DPI の変わり目で窓の中心を保つ動き（`taffy_flex_demo` で副モニターの原点へ送った窓が大きさの差の半分ずれて着く）が正しいか | **指摘にしない**（説明だけ 16 番で直す） | 16（説明）・直したほうがよい | 道筋: `WM_DPICHANGED`（`window_proc/window_pos.rs:434-484`）が推奨矩形の左上を `SWP_NOSIZE` で書く → 同じ呼び出しの中の `WM_WINDOWPOSCHANGED` が `DpiChangeContext` を見て中心を保つ補正（`dpi_helpers.rs:94-149`・量は（古い物理の大きさ − `BoxStyle`×新しい倍率）/2）を掛ける。(-2304,355) → (-2104,530) は差の半分 (200,175) と合う（推奨矩形の値そのものは測っておらず、数字から逆に推した）。Windows の既定（推奨矩形をそのまま当てる）からはわざと外れているが、それは完了 spec `wintf-dpi-window-center-preserve` の要件 1.1・1.2（DPI で大きさが変わったら中心を保つ・高 DPI から低 DPI へドラッグしたとき元のモニターへ引き戻されるのを防ぐ）どおりで、振る舞いを変えれば ⑴ に当たる。areka のキャラ窓とバルーン窓は両方 `ExternalAuthority`（`spawn.rs:17`・`:643`）なので補正は一度も掛からず、「DPI 追従が基本設計」「キャラ窓の原点は下端中央」は areka-placement が自分で守る＝衝突しない。残る問題は `ApplyPosition` の説明が着地点を言い当てていないことだけ。議題 8 |
| ⑷ | `window_proc/window_pos.rs` の 2 つ目の、錠を取らない flush（③）を直す対象に入れるか | **指摘にする**（消す） | 41・直したほうがよい | 入口が書き込みの借用中にハンドラを呼ばないので、③ が走るのは ② が錠に阻まれた場合だけで、そのときキューは取り出し済み＝本番ではいつも空を流す。害は「入口の確認が外れた／別の経路から呼ばれた」ときに画面更新の途中でキューを流す形に限られるが、説明が読み手を誤らせ、同じ範囲の 4 か所の説明が ③ を流しの入れ子の理由に挙げている。消しても `since_flush_us` の観測は失われない |

---

## 5. 印の一覧（要件 4.1 ⑵ の確かめ）

| 場所 | 印 | 決めごとか指摘か |
|---|---|---|
| `window/zorder_pair.rs:202` | `#[allow(dead_code)]`（`PairTrigger`） | 指摘（30・28）。理由の説明が無い＝⑵ に当たらない |
| `window/zorder_chain_apply.rs:574` | `#[allow(clippy::too_many_arguments)]`（`execute_ops`・引数 8） | 決めごと（束ねても型が 1 つ増えるだけで失敗の場面が無い） |
| `window/dpi.rs:54` | `#[allow(non_snake_case)]`（`from_WM_DPICHANGED`） | 決めごと（Win32 のメッセージ名） |
| `window_proc/window_pos.rs:6`・`lifecycle.rs:9`・`drop_files.rs:8`・`keyboard.rs:6`・`mouse_click.rs:5`・`mouse_dblclick_wheel.rs:5`・`mouse_move.rs:5` | `#![allow(non_snake_case)]` | 決めごと（ハンドラ名を `WM_*` にそろえる） |
| `window/window_band_real_tests.rs:147`・`:282`・`window/window_band_pipeline_real_tests.rs:132` | `#[ignore = …]`（実窓の門 `AREKA_WINTF_REAL_WINDOW_ZORDER`） | 決めごと（A-1 と同じ系統の隔離） |
| `window/zorder_pair_maintain_always_on_top_tests.rs:410`・`:788`・`:857` | `#[ignore = …]` | 決めごと（A-1 の隔離）。説明の古さは 38 |
| `window/transition_diag.rs:852` | `#[doc(hidden)]`（`reset_for_test`） | 決めごと（areka のテストも使うテストの口） |
| `window/command.rs:829` | `#[doc(hidden)]`（`drain_window_pos_commands`） | 決めごと（areka-placement の別クレートのテストの口・説明に理由） |
| `window/command.rs:323-356` | `#[cfg(test)]` の差し口（`FORCE_BATCH_BEGIN_FAILURE`） | 決めごと（縮退の道を決定論で踏む） |
| `window/zorder_chain_diag.rs:79・96・102・459` | `#[cfg(test)]` の名簿 | 決めごと（`:94-95` の説明は 27） |
| `window/zorder_pair_deferred_vocabulary_tests.rs` | 先送りした正典の語（最小化・タスクバー＝追う spec は未着手の `tasktray-minimize`）と帯の語が重なり順のコードに無いことを見張るテスト | 決めごと（記憶「正典機能の先送りは完全語彙＋縮退シーム＋追跡 spec」の見張り役。消す・弱める直しは ⑴／⑵。名簿の 8 本は実在の `zorder_` の本番ファイルと一致） |
| `window_proc/mouse_move.rs:300` | `NOTE:` | 決めごと（`SetWindowPos` を借用の外へ遅らせる理由・今のコードと合う） |

`shortcut:`・`ponytail:`・`TODO`・`FIXME`・`#[deprecated]` は範囲の中に 0。

---

## 6. 問題にしなかった所（疑って確かめ、指摘にならなかったもの）

- `DpiChangeContext`（`window/components.rs:33-90`）が窓ごとに分かれていない: set と take は同じ窓への 1 回の同期の `SetWindowPos` の内側で対になり、別の窓に漏れる場面を作れない。欄 `new_dpi`・`suggested_rect` は `trace!` でだけ読まれる（使い手 0 ではない）。
- `window_proc/window_pos.rs:104-113` の `window_to_client_coords(..).ok()` が失敗を捨てる: 自分の窓の手続きの中で自分の有効な窓への換算が失敗する具体の場面を作れない。
- `window_proc/window_pos.rs:357-362` の `WM_DPICHANGED` の lParam が null のとき `RECT::default()` に倒れる: OS の約束の上では起きない。
- `WM_CLOSE`（`lifecycle.rs:113`）・`WM_DISPLAYCHANGE`（`:221`）・`WM_ACTIVATE`（`keyboard.rs:197-199`）・投げ込み・キーが World を借りられないと飛ばす／`warn!`: 入口の確認があるので来るのは読み取りの借用中だけで、その間に同期で届くのは `SWP_FRAMECHANGED` の `WM_WINDOWPOSCHANGED` だけ。場面が無い。
- `WM_DPICHANGED` で方針を読めないときに「書く」へ倒れる作り: 完了 spec（dpi-window-vanish の要件 1.5）の決めごとで、`window_pos_tests.rs:595-665` が固定。
- `guarded_set_window_pos` が失敗するとコンテキストが残る（`window_pos.rs:522`）: 自分の有効な窓への `SetWindowPos` が失敗する現実の場面が無い。
- 合流の探索の O(n²)・`plan_chain_ops` の二重ループ・去る窓の切り離しの毎フレームの全走査・`collect_entities_to_leave` の全走査: n は十数で軽い。
- `EndDeferWindowPos` の失敗の全件の書き直し・`DeferWindowPos` が親の同じ窓を求めること: 最終の状態は変わらず、縮退の道（`warn!`＋1 本ずつ）へ落ちるだけ。
- `SIBLING_SCAN_LIMIT = 512`（`zorder_pair.rs:533`）: この机のトップレベル窓は 430 枚（可視 36）で上限の 84% だが、測る関数は最初の可視の窓や鎖の要素で止まり、越えても warn 1 行と番兵だけ。
- `measure_window_is_always_on_top` と `measure_below_is_always_on_top` の似た形: 読めないときに倒す向きがわざと逆で、一緒に変わる複製ではない。
- `ExpectedOrder`・`OwnerLink`・`OwnerEstablishFailed`・`IssuedPairFix` の `unsafe impl Send/Sync`: HWND の値の写しだけで、Win32 を呼ぶ system は `NonSendMarker` で UI スレッドに固定。
- 2 窓が同じフレームで消える場合とアプリの終了経路で切り離しが走らない: 完了 spec が G8 PASS と裁定済み（⑴）。
- `ZORDER` の早起きの旗の振る舞いのテストが無い: 旗はプロセス全体で 1 つの原子変数で並んで走るテストと取り合い、外れても遅れは心拍（30 フレーム）まで。
- 実窓の鎖のテスト（`zorder_chain_order*`）・確立の実窓テストに `#[ignore]` が無い: 走査のやり直しと持ち上げの出し直しを有界回数に収め、他プロセスの窓に割り込まれない形で、決まった赤の場面が無い（A-1 は別ファイル）。
- `Monitor` の `PartialEq` が handle だけを比べる: 意図した決めごと（`differs_in_value` と分けてありテストで固定）。
- `MSG_ALL`（`transition_diag.rs:137`）の使い手 0: 今は 3 語を全部含み、間違う場面が無い。
- `DPI::to_logical_point/size` 等の使い手がテストだけ: 公開の型の対称な換算の口で、外の使い手が普通に使う形。
- 閾値を越えた直後（`JustStarted`）の `WM_MOUSEMOVE` で窓が次の tick まで動かない: ドラッグのモジュールの設計（`clickthrough/controller.rs:43` が当てにする）。動いた量は失われない。
- ファイルの投げ込みが SSP の互換（⑶）に当たるか: wintf はパスの一覧を `OnFilesDropped` へ渡すだけで、`OnFileDrop2` などの SSP のイベントは areka の側で組む。範囲の指摘に投げ込みの外から見た振る舞いを変えるものは無い（49 はテストを足すだけ）。
- `Instant` の使い方（`command.rs:35・397・460・738`・`transition_diag` の `TickStart` ほか・`window_pos.rs:489`・`mouse_move.rs:134/383/436`）: 観測の計時と参考値の µs だけで誤りの場面は無い。作り直しは `instant-to-interrupt-ticks`（v0.2.0）と重なる。
- 各ファイルの行数: 1,000 行を越えるものは無い。

---

## 7. ほかの spec と重なる所（要件 1.7・6・7）

### `zorder-chain-residue`（未着手の台帳 spec）の A-1・B-6

| 台帳の件 | この spec の番号 | 今の姿 | どちらで直すか（議題 9） |
|---|---|---|---|
| A-1（実窓の重なり順の間欠の赤） | 38（隔離の説明の数と理由の文）・32（A-1 のファイルの下ごしらえを共有へ）・19（同じ門の写し）・40（A-1 のファイルの説明の向き） | 隔離は 3 本（`:410`・`:788`・`:857`）とも `#[ignore]`＋本文 1 行目の `require_explicit_real_window_run()`（環境変数 `AREKA_WINTF_REAL_WINDOW_ZORDER` が無いまま `--ignored` で呼ばれると赤で落ちる）。スクリプトや CI に `--include-ignored` で呼ぶ所は無い。「測定条件か欠陥か」の決着は未了。台帳の brief は「2 本」・行 `:767` と書くが今は 3 本・`:789` | この spec は説明・属性の文・下ごしらえの置き場だけを直し、隔離そのものと根治（測定条件か欠陥か）は台帳に残す案が自然。台帳の brief の「2 本」の訂正をどちらで書くかも決める |
| B-6（ペア機構の doc が 2 系統のまま・本番は 3 系統） | 36 | **まだ生きている**（台帳の brief の「消化済みの見込み」と結論が逆）。冒頭の説明ではなく関数の中の結線の説明（`zorder_pair_maintain.rs:383-386`）が 2 本の chain を引き、本番は 4 本 | 1 行の説明の直しなので、この spec で直して台帳の B-6 を閉じる案が自然 |

### C20 の約束（要件 7）

- 要件 7.1〜7.3 の型と綴り（`MonitorRecord`・`Monitor`・`DESPAWNED_SKIP_TAG`＝`[despawn-skip]`・`areka_placement::diag`）を変えずには直せない指摘は **0 件**。15（`monitor.rs` の自由関数の中だけ）・20（`transition_diag` の説明だけ）・8（`lifecycle.rs:80-87` の別の段落）は触れない。
- 依存の追加・`Cargo.lock` の変更が要る指摘は **0 件**（49 は `extern "system"` で Cargo を触らない形）。
- 範囲の外へ直しが及び、走っているセッションと申し合わせが要る候補（要件 6.2）: 6 番の型を消す場合（`ecs/pointer/types/`＝`audit-wintf-input`）・11 番（`crates/areka-placement` 14 ファイル＝`audit-placement-core`・`audit-placement-follow`、ほかに `crates/areka` 20 ファイル）・28 番の ⒝（`crates/areka-placement`）・62 番の ⒜（`ecs/drag/state/mod.rs` の `check_threshold` を残してもらう＝`audit-wintf-input`）。逆向きに、`audit-wintf-input` の直しがこちらの `window_proc/mouse_*` に及ぶ分は下の節。
- 範囲の外で、走っているセッションの無い持ち物: 1（`runtime/mod.rs`）・15（`ecs/layout/systems/monitor_systems.rs`）・52（`runtime/window_factory_creation_tests.rs`）。

### `audit-wintf-input`（同じ C20 で走っている・範囲＝`ecs/drag/`・`ecs/pointer/`・`ecs/clickthrough/`）

あちらのセッションから、あちらの直しのうちこちらのファイルに及ぶ予定のもの（実装の前に申し合わせ直す）が届いた。こちらの指摘で**同じ行・同じ塊を変えるもの**を突き合わせた。

| あちらの直し | こちらのファイルの場所 | 同じ行・塊を変えるこちらの指摘 | 重なり方 |
|---|---|---|---|
| `DragConfig.right_button`／`middle_button` を消す | `window_proc/mouse_dblclick_wheel_tests.rs:213`（1 行） | 46（ダブルクリックの受け手を消してふつうの押下へ寄せる＝このテストファイルの試し方が変わりうる）・51（同じファイルの `:56`） | 同じファイル。46 の形次第で `:213` の行そのものが動く |
| `PointerState.local_point` に本当のローカル座標を入れる | 構築の式 `mouse_click.rs:119`・`mouse_dblclick_wheel.rs:90`・`mouse_move.rs:364`・`:422` | 61（`mouse_click.rs:120-125`・`mouse_dblclick_wheel.rs:91-95` の同じ構築の式の欄を消す）・46（`mouse_dblclick_wheel.rs:90` を含む受け手を丸ごと消す）・45（`mouse_move.rs:326-438` の当たった枝と当たらない枝を 1 本にまとめる＝`:364` と `:422` が 1 か所になる）・63（`mouse_click.rs:80-106` の直前の塊） | **4 か所とも同じ式を変える**。45・46 を先に当てると、あちらの直しの場所は 4 から 2（`mouse_click.rs` の 1 と `mouse_move.rs` の 1）に減る |
| `DraggingState.initial_inset` を `initial_window_pos` へ名を替える | `mouse_move.rs:464-479` | 44（`mouse_move.rs:445-457` の `guarded_set_window_pos` の失敗を記録する＝同じドラッグの移動の塊の直前） | 隣り合う同じ塊。行は重ならない |
| `DragStateSnapshot` の写しを消す | `mouse_move.rs` ほかの読み手（`mouse_move.rs:215` の `match state_snapshot`） | 62（`mouse_move.rs:228-235`＝この `match` の `Preparing` の腕の中の閾値の判定）・47（`mouse_move.rs:198-210` の画面座標の式＝直前） | **62 は同じ腕の中を変える** |
| `push_pointer_sample` の引数を `i32` に（見込み） | `mouse_move.rs:383`・`:436` | 45（`:383` と `:436` は 45 でまとめる 2 つの枝のそれぞれの末尾＝1 か所になる） | 同じ行。45 を先に当てると場所は 1 か所 |

- このほか、こちらの 6 番は型 `WindowPointerTracking`（`ecs/pointer/types/mod.rs:206`＝あちらの持ち物）に触れうる（議題 4）。44 番の空の取っ手の出どころ（`ecs/drag/dispatch.rs:102`・`ecs/drag/state/mod.rs:383`）はあちらのファイルで、こちらは記録を足すだけ。
- 誰がどの行を直すかは議題 11。

### `instant-to-interrupt-ticks`（v0.2.0・この spec の着地を待つ）

`Instant` に関わる指摘は 0。`Instant` の使い方は 6 節のとおりで、作り直しはあちらの持ち物。⑵ の案 C は時刻を使うので重なる。

### `clippy-199-lints`

clippy の指摘を機械的に直す指摘は 0。

---

## 8. 公開の口への響き（要件 3.5 の見込み）

| 番号 | 公開の口 | 変わり方 | 木の中の使い手 |
|---|---|---|---|
| 1 | `SetWindowPosCommand::enqueue` | UI スレッドの外で積むと `error!` で捨てる（制約が増える） | 全部 UI スレッド（影響なし） |
| 6 | `wintf::ecs::WindowPointerTracking` | 型を消す場合だけ消える | wintf の中だけ |
| 10 | `WindowStyle::from_hwnd` | 消える | 0 |
| 11 | `WindowHandle.instance` | 欄が消える | 65 ファイル（構築の式） |
| 12 | `SetWindowPosCommand::flush` | 消える（`flush_window_pos_commands` へ寄せる） | テストだけ |
| 14 | `WindowPos::set_window_pos` | 消える | 0 |
| 28 | `ZOrderPairStrategy::ExplicitMaintenance`・`raise_assist` | ⒝ のときだけ消える | `crates/areka-placement` 5 ファイル・範囲の中 2 ファイル |
| 2 | `to_window_coords_for_creation`（`pub` だが呼び手は `runtime` の 1 か所） | 形は変わらず、片方だけのときの値が正しい側へ変わる | 1 |

---

## 9. 要件と今の姿のつき合わせ（ギャップ分析）

| 要件 | 今の姿 | ギャップ |
|---|---|---|
| 1.1・1.2 範囲の数え上げ | 69 本を数え、一覧を 1 節に残した。増減なし | なし |
| 1.3・1.4 audit を全件・全部読む | 9 組で全 69 本を最後まで読んだ。件数の上限なし | なし |
| 1.5 番号と組と件数 | 3 節（6・47・10・計 63） | なし |
| 1.6 確定した一覧を要件へ | 未（要件の討議で書く） | 討議の後に requirements.md へ書き加える |
| 1.7 A-1・B-6 との重なり | 7 節（36＝B-6・38／32／19／40＝A-1） | どちらで直すかは討議（議題 9） |
| 1.8 20 タスクを越えそうなら 2 本目 | 見立ては 9 節の下（14〜20・11 番と 32 番を分ければ 20 を越えうる） | 越えうる（議題 10） |
| 2 申し送り 4 件 | 4 節（⑴＝2・⑵＝しない・⑶＝しない＋16・⑷＝41） | 扱いの確定は討議（議題 7・8） |
| 3 全部直す | 未着手 | 63 件 |
| 3.5 公開の口の一覧 | 見込みは 8 節 | 設計で確定 |
| 4 残す指摘の裁定 | 候補は 3（⑴ に近い）・28 ⒝／30 の一部（⑴） | 討議（議題 1・2） |
| 5 かけ直し・テスト・全体テスト | 未着手 | 必ず直すの 6 件と、危ない所の 7・9・29・41・42・44・48・49 にテストが要る |
| 6・7 範囲の外・C20 | 7 節。C20 の型・綴りに触れる指摘 0、依存の追加 0 | 申し合わせの候補 4 つ（6・11・28 ⒝・62 ⒜）と、`audit-wintf-input` の直しが `window_proc/mouse_*` に及ぶ 5 件の行の分け方 |
| 8 守る決まり | 978・977 行の 2 本に足す直しは要件 8.1 に当たりうる（58 は 15 行減らす側） | 設計で行の見積もり |

### 直し方の選び方

- **案 A: この spec 1 本で全部直す**（範囲の外は 1・15・52 と、申し合わせた上で 6・11）。良い点: 1 回で終わる。同じファイルに触る直し（3＋23＋53、41＋12＋59、45＋46＋47＋48＋60＋6）をまとめて当てられる。悪い点: タスクの数が 20 前後でぶれる。11 番は 65 ファイルに触り、走っている `audit-placement-*` と申し合わせが要る。
- **案 B: 組で 2 本に分ける（要件 1.8）**。1 本目＝必ず直すの 6 件と、それと同じファイルに触る直したほうがよい（7・23・28〜31・39・41・42・44〜48）。2 本目＝残りの説明の直しと写しのまとめ・できれば。良い点: どちらも 20 以下。悪い点: 同じ範囲で 2 本走らせる手間と、「同じ範囲で新しい指摘 0」をどちらで見るか（要件 5）。
- **案 C: 場所で 2 本に分ける（`window/` と `window_proc/`）**。`window_proc/` は 41〜51・59〜61 と 6 の 15 件ほど。良い点: 触るファイルがきれいに割れる。悪い点: 41 番（③）は `window/` の 4 か所の説明に及び、12 番とも絡むので割り切れない。

### 手間と危うさ

- 手間: **M〜L**。直しの大半は説明の直し（約 20 件）とテストの写しのまとめ・足し（約 12 件）で、既にある作りの形のまま直せる。新しい仕組みは 1 番の UI スレッドの控えだけ。11 番が 65 ファイルに及ぶ分で L に寄る。
- 危うさ: **中**。振る舞いが変わるのは 1・2・3・4・5・6・7（直す場合）・8・15・41（消す）・44・45・46 で、どれも正しい側へ。areka の本番の振る舞いが変わる見込みのあるのは 6（離脱の見張りを毎回掛ける＝ areka では answer.rs が既に毎回掛けていて同じ）・8（areka は `WindowHandle` だけを外さない）・15（列挙の失敗の稀な場面）・41（空のキューを流さなくなるだけ）・44・45 で、実機の差はほぼ 0 の見込み。実窓テストに触る直し（19・25・32・38〜40）は A-1 の隔離の形を崩さないことが条件。

### タスクの数の見立て（要件 1.8）

| まとまり | 番号 | タスク |
|---|---|---|
| 書き込みのキューと窓の部品 | 1・7・8・10・11・12・13・52 | 3〜4（11 番の 65 ファイルで 1 本） |
| 生成の座標・DPI・モニター・帯・記録 | 2・14〜21 | 2〜3 |
| 鎖 | 3・22〜27・53・54 | 2 |
| 組・沈降・維持 | 4・5・28〜40・55〜58 | 3〜4（32 番の写しのまとめで 1 本） |
| 窓の手続きの位置・DPI | 41〜43・59 | 1〜2 |
| 窓の手続きの入力 | 6・44〜51・60〜63 | 2〜3 |
| かけ直しの audit・全体テスト | — | 1〜2 |
| 合計 | 63 | **14〜20**（11 番と 32 番を分ければ 20 を越えうる） |

### 設計へ持ち越す調べ（Research Needed）

- 7 番: 実際の Windows の `DeferWindowPos` が同じ窓の 2 本目を 1 項目へ畳むか（Wine・ReactOS の読みだけ）。足すテストで先に確かめてから直すかを決める。
- 1 番: UI スレッドの id を控える置き場（`WinApp::run` の頭か、`EcsWorld::new` か）と、ヘッドレスのテスト・`areka-placement` の別クレートのテストで控えが無いときの扱い。
- 2 番: 枠の厚みの実際の px（`AdjustWindowRectExForDpi`）を足すテストで測る。実機では測っていない。
- 5 番: 前面窓が不可視になる場面（通知領域の作法）を決定論で固定する純関数の形。
- 6 番: `WindowPointerTracking` を残すか消すか（`audit-wintf-input` との申し合わせ）。
- 11 番: 65 ファイルの直しの手順（`areka-placement` の 14 ファイルを `audit-placement-*` の着地の前にするか後にするか）。
- 41 番: 入口の確認（`runtime/wndproc_bridge.rs:91`）が今の形で全部の経路を覆うかを、③ を消す前に確かめる。
- 42・49 番: 実窓の配送と HDROP を決定論で作る形（既存の 0x0 の不可視窓の型・`GlobalAlloc` の宣言）。

---

## 10. 要件の討議へ渡す議題（答えで作業が変わるもの）

1. **3 番（鎖の持ち越しの印）**: 完了 spec `areka-P0-zorder-scope-inversion` の design.md:68 は「窓を 1 枚ずつ作る経路を足すときに見直す既知の限界」として書いている。推奨: 直す（裁定ではなく注記で、直しは数行・待つ間は OS を呼ばない規律を保つ）。代案: ⑴ として残す。
2. **28 番（案 B と `raise_assist`）**: 推奨: ⒜ 説明だけ直す（完了 spec の要件 5.6 の案の切り替えを残す＝⑴ に触れない）。代案: ⒝ 消す（⑴ に反するので裁定が要り、`areka-placement` へも及ぶ）。30 番の `RaisedAbove`／`RaisedBelow` もこれに従う。
3. **11 番（`WindowHandle.instance` を消す）**: 65 ファイル（うち `areka-placement` 14）に及ぶ。推奨: 直す。`audit-placement-core`・`audit-placement-follow` のセッションと申し合わせ、構築の式の 1 行を消すだけの機械的な直しとして入れる。代案: 申し合わせが取れなければ wintf と areka の 51 ファイルを先に直し、`areka-placement` の 14 ファイルはあちらの着地の後に（または次のウェーブへ・要件 7.4 に準ずる）。
4. **6 番（離脱の見張り）**: 推奨: 毎回 `TrackMouseEvent` を呼び、`WindowPointerTracking` の型は残して書き込みだけ消すか、`audit-wintf-input` と申し合わせて型ごと消す。どちらにするか。
5. **1 番（UI スレッドの外の enqueue）**: 推奨: 説明＋UI スレッドの id の控えで `error!` を残して捨てる。代案: 説明だけ（振る舞いを変えない）。
6. **7 番（同じ窓が 2 度入ったバッチ）**: 推奨: 実窓のテストで Windows の振る舞いを先に確かめ、違えば 1 本ずつへ回す。同じなら説明だけ直す。
7. **申し送り ⑵（`WindowCreationFailed`）**: 推奨: 指摘にしない（案 A＝今のまま）。代案: B・C（4 節）。
8. **申し送り ⑶（DPI の中心の保ち）**: 推奨: 指摘にしない。説明だけ 16 番で直す。
9. **`zorder-chain-residue` との重なり（要件 1.7）**: 推奨: 36 番（B-6）はこの spec で直して台帳の B-6 を閉じる。38・40・32・19（A-1 のファイルと門）は説明・属性の文・下ごしらえの置き場だけをこの spec で直し、隔離と根治（測定条件か欠陥か）は台帳に残す。台帳の brief の「B-6 は消化済み」「2 本」の訂正をどちらで書くか。
10. **分け方（要件 1.8）**: 見立ては 14〜20 タスク。推奨: 案 A（1 本）で進め、設計でタスクが 20 を越えると分かったら、組で 2 本目を切る（案 B）。
11. **`audit-wintf-input` と同じ行を直す分の分け方（7 節の表）**: こちらの 45・46・61・62・63（と 44・47・51）と、あちらの `local_point`・`initial_inset` の名替え・`DragStateSnapshot` を消す・`push_pointer_sample` の引数・`DragConfig` の欄を消す直しが、`window_proc/mouse_click.rs`・`mouse_dblclick_wheel.rs`・`mouse_move.rs`・`mouse_dblclick_wheel_tests.rs` の同じ式と塊を変える。推奨: `window_proc/` の中の作りの直し（45・46・61・62・63）はこちらが持ち、あちらがこちらのファイルに入れるのは型の形が変わったことに伴う機械的な追従（欄の名・引数の型・構築の式の 1 欄）だけにする。着地の順は先に着いた方がそのまま入れ、後の方が rebase して追従する（こちらが先なら 45・46 で `local_point` の構築の場所は 4 から 2 に減る）。あわせて 62 番の ⒜（`check_threshold` を残してもらい、こちらがそれを呼ぶ）か ⒝（あちらが消し、こちらがテストを足す）かを両方のセッションで決める。代案: どちらか一方のセッションが 4 ファイルの直しを全部持つ。

---

## 11. まとめ

- 範囲は 69 本（本番 27・テスト 42・33,925 行）。ponytail 5.3.3 の audit を全部読んでかけ、**63 件**（必ず直す 6・直したほうがよい 47・できれば 10）。うち 62・63 は `audit-wintf-input` が渡してきた 2 件を確かめて指摘にしたもの。
- 必ず直すの 6 件のうち 4 件（1・2・4・6）は「areka の今の使い方では起きないが、公開の型を普通に使う外の使い手で起きる」形。3 は areka の窓の作り方が変わると起きる既知の限界、5 は実機の確かめの記録の誤り。
- 申し送り 4 件: ⑴＝2 番（必ず直す）・⑵＝指摘にしない・⑶＝指摘にしない（説明は 16 番）・⑷＝41 番（消す）。
- C20 の型と綴りに触れる指摘 0、依存の追加 0。範囲の外で申し合わせが要る候補は 6・11・28 ⒝・62 ⒜。`audit-wintf-input` の直しとは `window_proc/mouse_*` の同じ式と塊で重なる（議題 11）。
- `zorder-chain-residue` の B-6 は生きていて 36 番に当たる。A-1 は 38・40・32・19 が説明と下ごしらえで重なる。

---

## 12. 設計の段の調べ（2026-11-14・`/kiro-spec-design -y`）

- 種類: 既にある仕組みの直し（Extension）＝軽い調べ（`rules/design-discovery-light.md`）。新しい依存・新しい外の API は 0 なので、外の文書の調べ（WebSearch）は行わなかった。測った時点は作業枝 `b915fd3b`（範囲のファイルは main `30a343eb` と同じ・`origin/main` も同じ）。
- 使ったスキルと手引き: `kiro-spec-design` の 5 つの規則（設計の原則・軽い調べ・まとめの 3 つの見方・設計の門）・ponytail（いちばん小さな完全な変更・消せるものは消す・新しい判断の分岐には小さなテスト）・steering の `structure.md`（1,000 行・共有のテストの道具は `<stem>_test_support.rs`）・`logging.md`（`info!` はライフサイクルだけ・`warn!` は立ち直れる失敗）・`tech.md`（全体テスト・`log-capture-kit` は開発用）。完了した `audit-wintf-runtime` の design.md の形に合わせた。

### 12.1 9 節の「設計へ持ち越す調べ」の答え

| 番号 | 問い | 答え（根拠） | 設計への響き |
|---|---|---|---|
| 7 | `DeferWindowPos` は同じ窓の 2 本目を 1 項目へ畳むか | 静的には決まらない（Wine・ReactOS の読みだけ）。実窓の確かめを直しの最初の手順に置き、その結果で 2 つの枝のどちらかを取る | Q4 の手順 1 が門。どちらの枝でも残るテストは「重なった列のバッチの結果＝1 本ずつの結果」の 1 本 |
| 1 | UI スレッドの id の控えの置き場 | `WinApp::run` の頭。木の中で `WinApp::run` を同じプロセスのテストから呼ぶ所は 0（`crates/wintf/tests/win_app.rs`・`crates/areka` の `input_events_tests.rs` ほか 4 本は `WinApp` を組むが `run` は呼ばない。`crates/areka/tests/smoke_boot_loop_exit.rs` は別のプロセス）。よってどのテストの実行ファイルでも控えは立たず、ヘッドレスのテストと `areka-placement` の別クレートのテストの振る舞いは変わらない | 判断は純関数 `admits_enqueue` に出して試す（控えのプロセス全体の値をテストで立てない＝同じ実行ファイルの他のテストを壊さない） |
| 2 | 枠の厚みの px | 測らなくてよい。期待値を「両方を与えたときの API の値」から出せば、DPI と OS の版に依らず決まる。空の矩形を `AdjustWindowRectExForDpi` に渡すと枠の 4 辺が出る（`bMenu=false` なら位置と大きさに依らない）ので、片方だけのときも同じ 1 本の式で済む | P1 の 2 番は「空の矩形で枠を得て、与えられた側にだけ足す」1 本の道に置き換える（今の矩形の道は消える） |
| 5 | 不可視の前面窓を決定論で固定する形 | `is_behind_foreground` に引数 `foreground_visible: bool` を 1 つ足す。呼び手は `get_foreground_window()` の値に `is_window_visible` をかけて渡す。走査の側（`measure_windows_in_front`）は変えない | Z5。表へ 1 行（不可視なら `None`） |
| 11 | 65 ファイルの直しの手順 | 直しの最後のタスク（検証の前）。その前に `audit-placement-core`・`audit-placement-follow` と申し合わせる。`areka-placement` の 14 ファイルのうち `follow_test_support.rs:25` の 1 行が `audit-placement-follow` の持ち物、残りが `audit-placement-core`。この spec が足すテストのファイルも `WindowHandle {` を組む（`window_pos_decision_tests.rs` ほか）ので、タスクの頭で `git grep -l "WindowHandle {"` を数え直す | H1。`crates/wintf/tests/graphics/` の 4 本は `wintf-visual-damage-redraw` も 1 行ずつ触りうる＝後に着いた方が rebase |
| 41 | 入口の確認は全部の経路を覆うか | 覆う。本番で `dispatch_window_message` を呼ぶのは `runtime/wndproc_bridge.rs` の `make_wndproc` だけで、そこは World の借用を試してから呼ぶ。ほかの呼び手はテストだけ（`ecs/tooltip/system_tests.rs:680`・`window/zorder_pair_sink_tests.rs:275`・`window_proc/` のテスト・`window_proc/window_pos_transition_tests.rs` の `test_wndproc`） | Q2 で ③ を消してよい。崩れを防ぐテストを 1 本足す |
| 42・49 | 決定論の形 | 42: `window_pos_transition_tests.rs:499-583` の 0x0 の不可視の道具窓と配送の wndproc を `pub(super)` にして新しい兄弟ファイルから使い、entity に `WindowHandle` を付けて判断の本体へ届かせる。49: `#[link(name = "kernel32")] extern "system"` で `GlobalAlloc` を宣言し、`DROPFILES` の頭（20 バイト・`pFiles=20`・`fWide=1`）と NUL で区切った UTF-16 のパス 2 本と終わりの NUL を書く。`DragFinish` が片付ける | Q3 |

### 12.2 申し合わせの更新（2026-11-14・取りまとめの側から）

- **C20 は 2 段**: この spec は 1 段目（`wintf-visual-damage-redraw`・`wintf-winmsg-executor-import`・`audit-placement-follow` と同じ段）。`audit-wintf-input`・`audit-placement-core` は 2 段目（1 段目の着地の後に実装を始める）なので、実際にはこの spec が両方より先に着地する。
- **`audit-wintf-input`**: ⒜ 62 番＝あちらは `check_threshold` とそのテスト 5 本（`ecs/drag/state/tests.rs`）を残し、説明の古さだけ直す。こちらは `mouse_move.rs` の写しを `check_threshold` の呼び出しに替える。⒝ 6 番＝こちらが型 `WindowPointerTracking`（`ecs/pointer/types/mod.rs:206`・`tests.rs:122-125`・`ecs/mod.rs:35` の再公開）を丸ごと消す。あちらは触らない。実装の直前にもう一度確かめる。⒞ あちらがこちらのファイルに入れるのは機械的な追従だけ＝欄の名 `initial_inset`→`initial_window_pos`・`DragStateSnapshot` を消したことに伴う読み手の型の名・`DragConfig.right_button` を消したことに伴うテストの 1 行・`push_pointer_sample` の引数が `i32` になれば 2 行。`PointerState` の `local_point` の構築の場所は変えない（あちらは配送の前に自分の範囲で計算する）ので、45・46 番は影響を受けない。⒟ 着地は先に着いた方がそのまま入れ、後の方が rebase する。
- **`audit-placement-follow`**: `Monitor` はこの spec で変えない（15 番は `monitor.rs` の自由関数の中だけ）とあちらへ伝えた。11 番で触るあちらの持ち物は `follow_test_support.rs:25` の 1 行だけ。
- **`wintf-visual-damage-redraw`**: `crates/wintf/tests/graphics/` の 4 本（11 番が 1 行ずつ直す）に触りうる。後に着いた方が rebase する。
- **`wintf-winmsg-executor-import`**: ソースの重なり 0（取りまとめの側の測り）。
- 全体テストは排他テストの机を通す。
- **門 GI の確かめ（2026-11-14・相手 `audit-wintf-input` のセッション・タスク 13.1）**: ⒜ はい（`WindowPointerTracking` は型・既定のテスト・`ecs/mod.rs` の再公開までこの spec が丸ごと消す。あちらの tasks 6.1 はそのテストを消す対象から外してある）。⒝ はい（あちらは `check_threshold`・`state/tests.rs` の 5 本・再公開を残し、説明だけ直す。`mouse_move.rs` の閾値の写しを呼び出しに替えるのはこの spec）。⒞ はい（`PointerState` を作る 4 か所の `local_point` は変えない）。足し: あちらの機械的な追従はテストのファイル（`keyboard_tests.rs`・`mouse_click_tests.rs`・`mouse_dblclick_wheel_tests.rs`・`mouse_move_drag_origin_tests.rs`）にもあり、どれも `DragStateSnapshot`→`DragState` か欄名の置き換えだけ。あちらの設計・タスク（`45b8cd22`）は main に未着地で、窓の手続きのコードにはまだ手が入っていない。
- **門 GP の確かめ（2026-11-14・タスク 15.1）**: 相手 `audit-placement-core` のセッション＝⑴ はい（あちらの設計は `WindowHandle.instance` の欄が消えることを前提にし、main を取り込んだ後に名指す行を grep し直す・持ち物の構築の式は 13 本）⑵ はい（こちらが先に着地し、あちらは先の段の 4 本の着地の後に main を取り込んで始める）。こちらが触るのは `instance` の 1 欄とそれだけが使う import だけ、と返した。相手 `audit-placement-follow` のセッション＝⑴ はい（持ち物で欄を持つのは `follow_test_support.rs:25` の 1 行だけで、あちらのブランチでも手を付けていない）⑵ はい（こちらが先に着地し、あちらが後で取り込む）。取り込み: この時点で `wintf-winmsg-executor-import` は main に着地済みで、こちらのブランチを `origin/main` へ rebase した。窓の工場のインスタンスの取り方（`crate::executor::util::get_instance_handle`）は変わっていない。`wintf-visual-damage-redraw` は未着地で、`WindowHandle {` を持つファイルとの重なりは 0。

### 12.3 まとめの 3 つの見方（`rules/design-synthesis.md`）

- **一般化**: 写しの指摘（13・19・25・32・45・46・47・60）は「同じ下ごしらえ・同じ式が複数の所にある」という 1 つの形。どれも新しい層を作らず、既にあるファイルの 1 つを正本にして `pub(super)` で借りるか、共有の `<stem>_test_support.rs`（`structure.md` の決まり）を 2 本（`command_test_support.rs`・`zorder_pair_test_support.rs`）だけ足す。
- **作るか借りるか**: UI スレッドの控えは `std::sync::OnceLock<ThreadId>`。枠の厚みは `AdjustWindowRectExForDpi`（既に使っている）。`WA_ACTIVE`／`WA_INACTIVE` は `windows` の定数（`drag/reentry.rs` と同じ）。閾値の判定は既にある `check_threshold`。HDROP を作るための `GlobalAlloc` だけは、`windows` の機能 `Win32_System_Memory` を足すと `Cargo.toml` が変わる（C20 の約束 ⑸）ので、テストの中の `extern "system"` の宣言で借りる。ログの確かめは既にある `log-capture-kit`。
- **削り**: 新しい本番の部品は UI スレッドの控えの 2 つの関数だけ。ほかは消す（`WindowStyle::from_hwnd`・`WindowHandle.instance`・`SetWindowPosCommand::flush`・`WindowPos::set_window_pos`・`build_flags`・③・`WM_CLOSE` の投函・`PairTrigger::Establish`・`WindowPointerTracking`・`handle_double_click_message`・`ExecOutcome` の組み立て・弱いテスト 1 本・重なった主張 6 行）か、説明を直すだけ。

### 12.4 設計で決めたこと（3 節の案から変えた所・決めた所）

- 6 番の `TrackMouseEvent` の失敗は `debug!`（`cursor/answer.rs:225` と同じ・次の `WM_MOUSEMOVE` でまた呼ぶので害が無い）。成功は今の `trace!` の行を 1 呼び出しに 1 行出す（テストが呼んだ回数を数える手がかり）。
- 15 番は呼び手の `detect_display_change_system` で「列挙が 0 台なら表を変えず、旗も下ろさずに戻る」。Windows はセッションがある間は 1 台以上を返すので、続くのは一時的な失敗の間だけ。
- 44 番の記録は `error!`（要件 8.2）。窓の手続きの中なので返す相手は無い。
- 46 番は `handle_button_message` に `double_click: bool` を足し、`DoubleClick` はボタンから決める。これで 60 番の X ボタンの式は 1 つの関数を 3 か所（押す・離す・ダブルクリック）から呼ぶ形になる。
- 34 番は理由の文を事実（`EnvFilter` は前方一致・実機の grep はタグで行う）に直し、出力先を 1 つ（`zorder_pair`）に揃える決まりは残して、テストの検査を区切り（`:`）まで含めた一致にする（記録の関数は全部 `zorder_pair.rs` にあるので緑のまま意味を持つ）。
- 8 番の後も `lifecycle.rs` の `[despawn-skip]` の枝は要る（despawn から登録簿の掃除までの間に、利用者や OS が投げた `WM_CLOSE` が届きうる）。説明の段落（`:80-87`）だけをその事実に直す。
- 7 番は門の手順（実窓の確かめ）の結果で枝を取る。どちらの枝でも、確かめのテストは「重なった列を流した結果＝1 本ずつ当てた結果」を主張する形で残す。
- 11 番は「直しの最後のタスク」と読み、その後に記録と検証（かけ直しの audit・全体テスト）を置く（かけ直しが 11 番の直しも読むため）。

### 12.5 危うさと手当て

| 危うさ | 手当て |
|---|---|
| 7 番の実窓の確かめが他のアプリの窓に割り込まれて揺れる | 自分の 0x0 の不可視の道具窓 4 枚だけで、相対の順（`GetWindow(GW_HWNDNEXT)`）だけを見る。2 回回して同じ答えが出てから枝を決める |
| 32 番の写しのまとめで A-1 の隔離が崩れる | `#[ignore]` の属性と本文 1 行目の `require_explicit_real_window_run()` は動かさない（関数の置き場だけが共有のファイルへ移る）。`--ignored` を付けずに回して 3 本が走らないことを確かめる |
| 1,000 行の番人（978 行・977 行の 2 本） | 978 行の `zorder_pair_maintain_tests.rs` は Z3・Z6 で減る側（下ごしらえ約 220 行・58 番の 15 行）。977 行の `zorder_chain_order_tests.rs` は Z2 で関数に `pub(super)` を付けるだけ。足すテストは全部 1,000 行から遠いファイルか新しい兄弟のファイルへ |
| `audit-wintf-input` と同じ塊（`mouse_move.rs` の `Preparing` の腕・`dragging_window_origin`） | あちらは 2 段目＝こちらが先に着地する見込み。I1 の頭で申し合わせを確かめ直す |
| 11 番の 65 ファイルと並走の spec | 最後に回し、頭で数え直しと申し合わせ。`areka-placement` の 14 ファイルと `crates/wintf/tests/graphics/` の 4 本は 1 欄の行を消すだけ |

## 13. 設計の討議の決め（2026-11-14・開発者「承認」＝2 議題とも推奨案）

1. **6 番の `TrackMouseEvent` の失敗の記録は `debug!`**。要件 8.2（`error!`＋`Err`）と `logging.md`（立ち直れる失敗は `warn!`）の字面からは外れるが、窓の手続きの中で `Err` を返す相手が無い・次の `WM_MOUSEMOVE` でまた呼ぶので立ち直る・失敗が続く場面で `warn!` だと移動のたびに溢れる・`cursor/answer.rs:225` が同じ失敗を `debug!` で残す、の 4 つを理由とし、本番の説明にも 1 行書く（設計の検証の重大な点 1）。
2. **15 番の 0 台の判定は `apply_monitor_snapshot` の頭へ**。戻り値を `Option<usize>` にし、0 台なら `None`（表を変えない）。`detect_display_change_system` は `None` なら旗を下ろさず次の刻みで読み直し、0 台が続く間は `Local<bool>` で覚えて `warn!` と頭の `info!` を続きの最初の 1 回だけ出す。空の一覧を渡すテストを 1 本足す（設計の検証の重大な点 3）。選ばなかった案＝0 台でも旗を下ろして次の `WM_DISPLAYCHANGE` を待つ（一時的な失敗の後に知らせが来なければ古い表が残る）。

## 14. 実装の記録（2026-11-14・タスク 1.1〜15.2）

- 測った時点は作業枝の `f88c9516b`（15.2 の後）。タスク 1 つにコミット 1 つ（`git log --oneline origin/main..HEAD` の `feat(audit-wintf-window): <番号> …` の 34 本＝小タスク 1.1〜15.2）。門 GI（13.1）と門 GP（15.1）の確かめは 12.2 節の終わりの 2 項にあり、ここでは繰り返さない。
- 実装の途中で書いた控えはこの節へ移し、控えのファイルは消した。

### 14.1 確定した 63 件と直した小タスク（要件 1.6・1.8・3.1）

63 件は、どれもちょうど 1 つの小タスクで直した（tasks.md の小タスクの見出しの番号を数え、1〜63 が 1 回ずつ現れ、抜けも重なりも 0 であることを確かめた）。大タスクは 17（design.md の組 Q1〜Q4・C1・P1・Z1〜Z6・I1・I2・H1・V1・V2）で 20 に収まり、2 本目の spec は切らなかった（要件 1.8）。

| 番号 | 小タスク | 番号 | 小タスク | 番号 | 小タスク | 番号 | 小タスク |
|---|---|---|---|---|---|---|---|
| 1 | 4.1 | 17 | 6.2 | 33 | 11.2 | 49 | 3.2 |
| 2 | 6.1 | 18 | 6.3 | 34 | 11.2 | 50 | 14.3 |
| 3 | 7.1 | 19 | 9.2 | 35 | 11.2 | 51 | 14.3 |
| 4 | 10.1 | 20 | 2.3 | 36 | 12.1 | 52 | 5.3 |
| 5 | 11.1 | 21 | 6.2 | 37 | 12.1 | 53 | 7.1 |
| 6 | 13.2 | 22 | 7.2 | 38 | 12.1 | 54 | 7.2 |
| 7 | 4.2 | 23 | 7.2 | 39 | 12.2 | 55 | 11.2 |
| 8 | 5.1 | 24 | 7.2 | 40 | 12.1 | 56 | 11.2 |
| 9 | 5.2 | 25 | 8.1 | 41 | 2.2 | 57 | 11.2 |
| 10 | 5.2 | 26 | 7.2 | 42 | 3.1 | 58 | 12.2 |
| 11 | 15.2 | 27 | 7.2 | 43 | 3.2 | 59 | 2.2 |
| 12 | 2.1 | 28 | 10.2 | 44 | 13.4 | 60 | 14.2 |
| 13 | 1.1 | 29 | 10.1 | 45 | 13.3 | 61 | 14.1 |
| 14 | 6.2 | 30 | 10.2 | 46 | 14.1 | 62 | 13.4 |
| 15 | 6.3 | 31 | 10.2 | 47 | 14.2 | 63 | 14.3 |
| 16 | 6.2 | 32 | 9.1 | 48 | 13.3 | | |

### 14.2 7 番の門の結果と、申し送り 4 件の扱い（要件 2.1・2.3）

- **7 番の門（タスク 4.2）＝「同じ」の枝**。`command_batch_tests.rs` の `a_queue_with_the_same_window_twice_lands_in_the_same_order_as_one_write_per_command` で、自分の不可視の窓 4 枚（A〜D）を毎回 `0,1,2,3` に並べ直し、Z だけの列 `[A を B の後へ][C を A の後へ][A を D の後へ]` を ⑴ 1 本ずつ `guarded_set_window_pos` で当てる ⑵ キューに積んで `flush_window_pos_commands` で流す、の 2 通りで比べた。2 回とも ⑴＝`[1, 2, 3, 0]`（B,C,D,A）・⑵＝`[1, 2, 3, 0]`。⑵ がバッチの道を通ったこと（書込レコード 3 本とも `in_batch=true`）も同じテストで見る。Wine・ReactOS の読み（2 本目を畳む）なら `[1, 3, 0, 2]` になるはずで、**この列の実際の Windows の結果には当たらない**。
- 「同じ」の枝なので本番のコードは変えず（1 本ずつの道へ回す判定も `debug!` も足さない）、説明だけを直した。`apply_as_batch` の説明は、確かめたのは上の 1 本の列だけ・ほかの列は確かめていない、と範囲を絞って書く（レビューの差し戻しで一般の決まりの書き方から絞った）。`flush_window_pos_commands` の説明は確かめた範囲を `apply_as_batch` の説明へ指すだけ。門のテストは確かめとして残した。
- 直した後の `window/command.rs` は **984 行**（1,000 行以下。`command_ui_thread.rs` は作らなかった）。4.2 の直後は 983 行で、説明を絞った差し戻しの後に 984 行になった。
- **申し送り 4 件**（4 節）: ⑴＝2 番で直した（6.1）・⑷＝41 番で直した（2.2）・⑵ と ⑶＝指摘にしない（討議の決め 6）。⑶ の説明（`DpiSuggestedRectPolicy::ApplyPosition` の着地点）は 16 番で直した（6.2）。

### 14.3 公開の口の変化（要件 3.5）と、コードの実物との突き合わせ

design.md「公開の口の変化」の表の行を 1 つずつ、作業枝の木で `git grep` して確かめた。0.x の壊す変更として次の版でまとめて出す（`#[deprecated]` は挟まない）。

| 番号 | 口 | 表の変化 | コードの実物（`git grep`） | 突き合わせ |
|---|---|---|---|---|
| 1 | `wintf::ecs::SetWindowPosCommand::enqueue` | 形は同じ・UI スレッドの外から積むと `error!` を残して捨てる | `command.rs:697` の頭で `admits_enqueue(cmd.hwnd, UI_THREAD.get().copied())`。控えは `runtime/mod.rs:372` の `record_ui_thread()`（`pub(crate)`）。判定の `admits_enqueue` は私有 | 一致 |
| 2 | `wintf::ecs::WindowPos::to_window_coords_for_creation` | 形は同じ・片方だけ与えたとき与えた側に枠を足す | `window_pos/mod.rs:335` の定義。呼び手は `runtime/window_factory.rs:287` の 1 か所 | 一致 |
| 3 | `wintf::ecs::apply_zorder_chain`・`ZOrderChainPlan.dirty` | 形は同じ・ハンドル待ちが残る巡は印を戻す | `zorder_chain_apply.rs:273-274` が `plan.dirty = handle_missing;`・`*carry_announced = handle_missing;`（書けた本数に依らない） | 一致 |
| 4 | `wintf::ecs::establish_owner_links` | 形は同じ・後から付けた宣言でも owner を張る | `zorder_pair_establish.rs:143` が `declaration.is_added()` を条件に含む | 一致 |
| 6 | `wintf::ecs::WindowPointerTracking` | 消える | `git grep -c "WindowPointerTracking" -- crates` が 0 件（型・テスト・`ecs/mod.rs` の再公開とも） | 一致 |
| 7 | `wintf::ecs::flush_window_pos_commands` | 「違う」の枝のときだけ、重なった列を 1 本ずつ当てる | 門は「同じ」の枝。`has_repeated_window` は 0 件で、流しの本体（`command.rs:769`）は振る舞いを変えず説明だけが変わった | **変化なし**（表の条件の枝を取らなかった） |
| 10 | `wintf::ecs::WindowStyle::from_hwnd` | 消える | `git grep "from_hwnd" -- crates` が 0 件 | 一致 |
| 11 | `wintf::ecs::WindowHandle` の欄 `instance` | 消える | `pub instance` は 0 件。`git grep -n "instance:" -- crates` で `WindowHandle` の構築の欄は 0 件（残りは別物の `StoryboardInstance` の引数と、窓クラスの登録に使うローカルの `let hinstance` だけ） | 一致 |
| 12 | `wintf::ecs::SetWindowPosCommand::flush` | 消える | `git grep "SetWindowPosCommand::flush" -- crates` が 0 件。自由関数 `flush_window_pos_commands` は `ecs/mod.rs:55` で再公開 | 一致 |
| 14 | `wintf::ecs::WindowPos::set_window_pos` | 消える | `fn set_window_pos` と私有の `fn build_flags(` が 0 件（`raise_reset_window_pos` は別物）。`build_flags_for_system` は残り、呼び手 3 つは変わらない | 一致 |
| 15 | `wintf::ecs::window::monitor::enumerate_monitors` | 形は同じ・失敗を `error!` で残す | `monitor.rs:244`（`hmonitor`・`error` の欄）と `:253` が `error!`。返す値（失敗で空の `Vec`）は同じ | 一致 |

表に無かった公開の口の変化（実装で分かったもの）:

| 口 | 変化 | 使い手への響き |
|---|---|---|
| `wintf::ecs::layout::detect_display_change_system`（`pub`・`ecs::layout` の再公開） | 6.3（15 番の範囲の外の追従）で system の引数に `Local<bool>`（0 台の続きの控え）が 1 つ増えた | system として登録する形（`ecs/world/mod.rs:231`・`crates/wintf/tests/window/monitor_hierarchy_test.rs:260` の `add_systems`）は直さずに通る。関数として直に呼ぶ使い手は木の中に 0 |

クレートの外から見えない口の形の変化（表に入れない）: `create_windows`（`pub(crate) mod window_system`・問い合わせの状態を引数で受ける）・`apply_monitor_snapshot`（`pub(crate)`・戻り値が `Option<usize>`）・`PairTrigger::Establish`（消えた）・`is_behind_foreground`（引数 `foreground_visible`）・`collect_entities_to_leave`（`exclude` が `Option<Entity>`）・`handle_button_message`（`pub(super)`・`double_click` の引数）・`WINDOW_WRITE_LOG_MESSAGE`（`pub(super)`）・`zorder_pair_diag` の `UNKNOWN`／`tristate_field`（`pub(crate)`）。

**0.x の変更の知らせに載せる 1 行**: 枠つきの窓に位置だけ・大きさだけを与えたとき、生成の直後の位置が正しくなった（次の刻みで枠の厚みぶん跳ばない。タスク 6.1・2 番）。

### 14.4 消した項目と、消す前の `git grep`（要件 6.3）

どれもツリー全体（テスト・fixture・設定・文字列を含む）で引いた。`.kiro/` の中（この spec の文書と `completed/` の過去の spec の記録）に残る名は、指摘の説明・歴史の記録なので直していない。

| タスク | 消したもの | 消す前の `git grep` | 消した後 |
|---|---|---|---|
| 1.1 | 書き込みのテストの 3 本のファイルの道具 7 項目の写し（`fake_hwnd`・`clean_slate`・`transition_lines`・`lines_of_kind`・`emit_control`・`SIGNOFF_DIRECTIVES`・`CONTROL_LINE`）・字面 `[coalesce-probe] alive` | `command_(batch\|coalesce\|transition)_tests::<7 項目>` は 0 件（どれも私的な項目）。`coalesce-probe` は合流のテストが定数を通して読むだけ | `coalesce-probe` 0 件 |
| 2.1 | メソッド `SetWindowPosCommand::flush`（と中身の無い旧 `flush_window_pos_commands` の包み） | コード 46 件（`command.rs` 4・`command_transition_tests.rs` 3・組と帯の実窓テスト 8 本 43）＋ spec 文書。areka・areka-placement は 0 | `.kiro` の外で 0 件 |
| 2.2 | `WM_WINDOWPOSCHANGED` の手順③の呼び出しと、行番号で ③ を指す説明 | `手順③`・`3ステップ`・`window_pos.rs:2xx` が `window_pos.rs:41・44・296`・`command.rs:22・712`・`command_transition_tests.rs:390`・`transition_diag.rs:808`・`window_pos_transition_tests.rs:595` | 0 件（`WM_DPICHANGED` の別の ②③ は残る） |
| 5.1 | ハンドルの外れで投げる `PostMessageW(hwnd, WM_CLOSE)` | ハンドルの外れから投げる行は `window_handle.rs:280` の 1 件だけ（ほかの `WM_CLOSE` の投函は `tooltip/os_tests.rs:446`・`shiori-host32-ipc/src/send_timeout_tests.rs:101` の自前の窓のテスト）。`on_window_handle_remove` の使い手はフックの登録だけ。`"sending WM_CLOSE"` を読むテスト・道具は 0 | `crates/wintf/src/ecs/window` の `PostMessageW` が 0 件。ハンドルの外れから投げると書く説明も 0 件 |
| 5.2 | `WindowStyle::from_hwnd`（と、それだけが使っていた `components.rs` の `use crate::api::*;`） | コードでは `components.rs:218` の定義だけ（`get_entity_from_hwnd`・`WindowCoordinateConverter::from_hwnd` は別物で `.kiro/specs/completed/` にだけ出る） | コードで 0 件 |
| 5.3 | `create_windows` の中の型の別名と `SystemState` の import（公開の口ではない） | 直に呼ぶ所は `window_system.rs` 1・`runtime/window_factory_creation_tests.rs` 2・`runtime/window_factory.rs` 2 | `create_windows(&mut` が crates で 0 件 |
| 6.1 | テスト「position.x だけ CW_USEDEFAULT でも素通し」1 本と古い説明 | `to_window_coords_for_creation` の使い手は `runtime/window_factory.rs` とこのファイルのテストだけ | ― |
| 6.2 | `WindowPos::set_window_pos`・私有の `build_flags` | `(^\|[^_])set_window_pos\(\|build_flags\(` で crates の使い手は定義の `window_pos/mod.rs` の 2 か所だけ（ほかは別物の `raise_reset_window_pos(` と completed の記録）。examples・tests・fixture・設定は 0 | crates の定義・呼び出し 0 件 |
| 7.1 | 呼び出し元の `ExecOutcome { … }` の組み立て（型は残す） | 関数・型・口は消していない | ― |
| 7.2 | 鎖の側の `UNKNOWN`・`tristate_field`・`zorder_chain_tests.rs` の重なった主張 6 行と `SET_WINDOW_POS_FLAGS` の import | 鎖の側の 2 つは `zorder_chain_diag.rs` と子のテストの中だけ（`zorder_chain_diag::UNKNOWN` を名指す所は 0）。`SET_WINDOW_POS_FLAGS` は消した 2 行だけ | `fn tristate_field\|const UNKNOWN` は `zorder_pair_diag.rs` の 2 行だけ |
| 8.1 | 鎖の順のテストの足場 7 つ（`chain_schedule`・`spawn_window`・`publish_full_chain`・`schedule`・`spawn_scopes`・`balloon_offsets`・`place_after` の写し） | どれも私的な関数。同じ名の関数がほかのテストのファイルにあるが別のモジュールの別物 | outsider・inversion に自前の定義 0 件 |
| 9.1 | 組の実窓テスト 11 本のファイルの下ごしらえの写し | どれも私的な `fn`／`const`。当たるのは別のモジュールの同じ名の定義だけ。文字列で読むのは `areka` の `zorder_inversion_branch_coverage_tests.rs` のテスト名だけで、テスト名は変えていない | 消したものへの参照 0 件 |
| 9.2 | 実窓の門の写し 3 つと、帯の窓の手伝い 4 つの写し | 消したのは私的な `fn`／`const`。同じ名の別物（`command_batch_tests.rs` の `destroy(windows: &[HWND])` ほか）は触っていない | 門の定義は `zorder_pair_test_support.rs` の 1 組だけ |
| 10.2 | `PairTrigger::Establish` | `PairTrigger::Establish\|Establish\b` で `zorder_pair.rs` の定義・表の行・腕と `zorder_pair_tests.rs` の 3 か所だけ（ほかは `.claude/skills/kiro-impl/SKILL.md` の英語で別物） | crates で 0 件 |
| 11.2 | `zorder_pair_sink_tests.rs` の自前の `WA_INACTIVE`／`WA_ACTIVE` | 同じファイルの定義と 3 か所の使い手だけ | 活性の定数の自前の定義 0 件（`WA_BOTTOM` ほかは別の意味） |
| 12.2 | テスト `measure_window_below_maps_failure_to_absence_and_records_it` とその節の見出し・`measure_window_below` の use | 定義 1 か所と、research.md・completed の記録だけ。コード・fixture・設定は 0 | ― |
| 13.2 | 型 `WindowPointerTracking`・そのテスト・`ecs/mod.rs` の再公開 | crates では `ecs/mod.rs:35`・`pointer/types/mod.rs:198,206`・`pointer/types/tests.rs:122,125`・`window_proc/mouse_move.rs`（8 か所）だけ。areka・areka-placement・examples・`crates/*/tests`・fixture・設定は 0 | crates で 0 件 |
| 13.3 | 字面「PointerState inserted on Window (no hit)」「PointerState moved to Window, Leave marker inserted」「PointerLeave marker inserted (scoped to window)」 | `mouse_move.rs` の本番の行だけ（テスト・fixture・設定・areka からの参照は 0） | ― |
| 13.4 | `Preparing` の腕の手書きの閾値の判定（ローカルの束縛だけ） | 外から引く名は無い | ― |
| 14.1 | `handle_double_click_message`（約 120 行） | crates では定義と 4 つの呼び出しだけ。tools・テスト・fixture・設定は 0。`doc/COMPAT_ARCHITECTURE.md:368` の 1 件は直した。ログの字面 `Double-click detected` を読む所は 0 | ― |
| 14.3 | `entity_name`・`bounds_info` の束縛・テストの `DragAccumulatorResource` の挿入 1 行 | 下げた `info!` の字面（`hit_test result`・`Calling start_preparing`・`deactivated during drag, cancelling`）を読むテスト・tools・doc は 0 | ― |
| 15.2 | `WindowHandle` の欄 `instance`・それだけが使う `HINSTANCE` の import（47 ファイル）・`get_instance_handle` の import 1 つ・`runtime/spine.rs` の `HINSTANCE` | `git grep -l "WindowHandle {"` は着手のとき 62 本（crates の中は wintf・areka・areka-placement）。`.instance` の読み取り・分解の束縛は木全体で 0 | 構築の欄 74 行（52 ファイル）と分解の束縛 4 か所を消した。`instance:` の構築の欄 0 件 |

### 14.5 design.md からの小さなずれ

| タスク | ずれ | 扱い |
|---|---|---|
| 14.3 | 50 番（ファイル頭の一覧）を I2 の 14.3 にまとめ、`mouse_move.rs` の頭の 1 行（`WM_SETCURSOR`）もそこで直した。`keyboard.rs` には `WM_CAPTURECHANGED` を足した。`mouse_dblclick_wheel.rs` の分かれた説明は 14.1 の作り直しで既に無く、頭に 1 行足した | tasks.md の頭の決めどおり |
| 9.1 | `window/command.rs` の `WINDOW_WRITE_LOG_MESSAGE` を `pub(super)` にした（修飾子だけ・行数 984 のまま）。共有の道具の `SET_WINDOW_POS_TAG` が手写しでなくこの定数そのものを読む | design.md の Z3 の表に無い `command.rs` への手 |
| 5.3 | tasks.md に挙げた 3 か所のほかに、`runtime/window_factory.rs` の `create_windows_new_path_spawns_registers_and_handles_without_class_error` の直の呼び出し 2 か所も `run_system_once` へ替えた（引数が増えてビルドが通らなくなるため・範囲の外） | design.md に無い範囲の外の追従 |
| 4.1 | `admits_enqueue` に引数 `hwnd` を足した（`admits_enqueue(hwnd, ui_thread)`）。`error!` の行に窓を載せるため。3 項目は `command.rs` に置いた（直した後 976 行で 1,000 行に届かないので `command_ui_thread.rs` は作らない） | design.md の形の引数が 1 つ増えた |
| 13.2 | 道具窓を借りるため、`window_pos.rs` の `mod window_pos_transition_tests` を `pub(super)` に、`DISPATCH_TARGET`・`create_test_window` を `pub(in crate::ecs::window_proc)` に広げた（2.2 で `pub(super)` にした道具窓の 5 項目のうちの 2 つ） | テストの時だけの可視性で本番は変わらない |
| 5.1 | `WM_CLOSE` の投函を消して偽になった範囲の外の説明を直した（コメントだけ・行は増やさない）: `ecs/clickthrough/controller.rs:91`・`areka/src/menu/trigger_flow_tests.rs:805-807`・`areka/examples/mock-shell.rs:423,437`・`areka/examples/emo-present/input.rs:18`（範囲の中では `zorder_pair_maintain_detach_tests.rs:298-299`・`lifecycle.rs` の「投函契約と」の語・テスト `wm_close_skips_stale_entity_as_normal_teardown` の説明と腕の注） | 要件 6.1 の最小の追従 |
| 6.3 | `detect_display_change_system` の引数が 8 個になり `#[allow(clippy::too_many_arguments)]` を付けた（`clip_sync.rs`・`zorder_chain_apply.rs` の前例）。公開の口の引数が増えた（14.3 の表の外の行） | 討議の決め 2 の形のまま |
| 1.1 | 合流のテストの `CONTROL_LINE` の値を `"[coalesce-probe] alive"` から共有の `"[transition-probe] alive"` にそろえた（定数を通してしか読まないので振る舞いは同じ） | ― |
| 7.2 | `zorder_chain_apply.rs` の前例の場所を行番号から `window_proc/window_pos.rs` の `WM_DPICHANGED` の名に替えた | ― |
| 8.1 | inversion の少し変えた写し 3 つは引数で表せたので、lifecycle 版を引数つきで呼ぶ形にした（lifecycle は fmt で 661→665 行） | ― |
| 9.1 | 測りのテストの `assert!(is_window_visible(parent), …)` を共有の `create_banded_test_parent` に移した（`create_test_parent` を通る全ファイルにかかる）。名の違うものは `use … as …` で受け、テストの本文 76 本の差分は 0 | ― |
| 11.2 | 34 番の誤った理由の文を、指摘の 5 か所に加えて `zorder_pair_diag.rs`・`zorder_pair_maintain.rs` の頭でも直した | 説明だけ |
| 12.1 | 40 番と同じ向きの誤り（常に手前のテストの腕①のコメント「すぐ手前」＝実は `InsertSpec::After` ですぐ背後）も直した（784 行） | 説明だけ |
| 13.4 | `ecs/drag/state/mod.rs:618-621` の `check_threshold` の説明（「本番呼び出しがゼロ」「`mouse_move.rs` 内にインライン複製」）は 62 番の直しで事実に合わなくなったが、12.2 の門 GI ⒝ の申し合わせで `audit-wintf-input` が説明を直すので触っていない | あちらの持ち物 |
| 14.1 | `doc/COMPAT_ARCHITECTURE.md:368` の定義点を `handle_button_message` に直した（範囲の外）。振る舞いの差: 消した受け手は既にある `PointerState` の `shift_down`／`ctrl_down` を直に書いていたが、新しい道はふつうの押下と同じく `set_modifier_state` の溜め（次の tick）に任せる。ログの `Double-click detected` の行は無くなり、`hit_test result` の行が出る | 字面を読む所は 0 |
| 14.3 | ダブルクリックの印が `debug` の行にしか出なくなるので、`hit_test result` の行に `double_click` の欄を足した | ― |
| 10.1 | 宣言を窓の生成の前に付ける areka の経路（`spawn.rs:593`）では、宣言の付いた巡にまだハンドルが無ければ `reason=HandleMissing` の見送りの記録が組ごとに 1 本増えうる（実機では確かめていない・窓の振る舞いは変わらない） | 記録だけ |
| 15.2 | 道具窓の `create_test_window`（`window_pos_transition_tests.rs`）は `(HWND, HINSTANCE)` でなく `HWND` を返す形にした（呼び手 4 か所）。`crates/areka/examples/mock-shell.rs` の `mod tests` の `HINSTANCE` の import も消した（`--examples` のビルドでは組まれず、`cargo test … --examples --no-run` で確かめた） | ― |

### 14.6 タスクごとの較正（壊し方と赤になったテスト）

足したテストと確かめを直したテストは、本番に壊し方を 1 度入れて赤を見てから明示の編集で戻し、差分が意図した直しだけに戻ったことを確かめた。

| タスク | 番号 | 壊し方 | 赤になったテスト |
|---|---|---|---|
| 2.1 | 12 | `pub fn flush_window_pos_commands() {` に空白を 1 つ入れる | `command_transition_tests::flush_keeps_the_front_guard_so_the_default_run_pays_no_observation_cost` |
| 2.2 | 41 | ② の直後に `flush_window_pos_commands()` の呼び出しを戻す | `windowposchanged_does_not_flush_the_queue_while_the_lock_is_held`（left: 0, right: 1） |
| 3.1 | 42 | `use_bypass` を `is_echo` だけにする／`skip_box_style` から `dpi_context.is_some()` を落とす | どちらも `after_a_dpi_change_window_pos_is_marked_and_box_style_is_left_alone` |
| 3.2 | 49 | `drop_files.rs` の `vec![0u16; len as usize + 1]` の `+ 1` を落とす | `dispatch_real_hdrop_delivers_the_same_two_paths`（`.nar` が `.na`） |
| 4.1 | 1 | 比べを外す（`Some(ui)` だけにする）／`error!` を消す | `enqueue_is_admitted_on_the_recorded_ui_thread`／`enqueue_off_the_recorded_ui_thread_is_dropped_with_one_error` |
| 4.2 | 7 | `apply_as_batch` の積み上げを `.rev()` にする | `a_queue_with_the_same_window_twice_lands_in_the_same_order_as_one_write_per_command`（キュー側 `[1, 0, 3, 2]`） |
| 5.2 | 9 | `window_to_client_rect` の `- left_diff` を `+ left_diff` にする | `framed_window_client_to_window_round_trips`（`(84, 200)`） |
| 6.1 | 2 | 直す前の素通し／早戻りを `!position_given \|\| !size_given` にする | `test_to_window_coords_for_creation_position_only_adds_frame_to_position`・`test_to_window_coords_for_creation_size_only_adds_frame_to_size` |
| 6.3 | 15 | 0 台の判定を `if false && …` にする | `empty_snapshot_keeps_monitor_table_and_returns_none`（`Some(0)` ≠ `None`） |
| 7.1 | 3 | 持ち越しを「書けなかったとき」に戻す（`handle_missing && !acted`） | `an_edge_without_handles_is_skipped_with_a_reason_and_the_rest_are_linked`・`the_remaining_link_is_made_once_the_late_handle_attaches_and_the_mark_drops` |
| 7.2 | 24 | `settled_line` の `nudged` と `after` を入れ替える | `the_settled_line_carries_the_declaration_and_the_measurement_together`・`the_four_signoff_fields_of_the_settled_line_stay_adjacent_in_this_order` |
| 9.1 | 32 | （確かめ）共有の `issue_control_command` から流しを外す | 組のテスト 13 本（`maintain::tests` 6・`survivor_tests` 2・`sink::tests` 2・`release_tests` 3） |
| 10.1 | 4・29 | `declaration.is_added() \|\|` を外す／比べを `failed.is_some()` にする | `establishes_when_the_declaration_is_added_after_the_handles`／`retries_and_establishes_once_the_handles_are_replaced` |
| 10.2 | 30 | `(RaisedBelow, ExplicitMaintenance)` の腕を `Skip(EchoOrIrrelevant)` にする | `decide_pair_fix_follows_the_band_table_for_the_place_above_arm`・`decide_pair_fix_handles_raised_below_per_strategy` |
| 11.1 | 5 | 引数 `foreground_visible` を判定に使わない | `is_behind_foreground_reports_absence_rather_than_a_false_negative`（`Some(false)` ≠ `None`） |
| 11.2 | 34 | `log_sink_observed` の `debug!` に兄弟の `target:` を足す | `every_record_is_visible_under_the_signoff_directive`（前の検査はこの壊し方で緑のままだった） |
| 12.2 | 39 | `establish_pair` の流しの 1 行を外す | `character_first_despawn_detaches_the_owner_and_spares_the_balloon`（left: 2, right: 1） |
| 13.2 | 6 | `track_mouse_leave` に「1 度だけ」の門を入れる | `every_mouse_move_arms_the_leave_watch`（left: 1, right: 2） |
| 13.3 | 48 | `collect_entities_to_leave` の窓の絞り込みを外す | `mouse_leave_leaves_only_the_window_it_reached`・`test_collect_protects_other_windows_pointer_state` |
| 13.4 | 44 | 直す前の `let _ =` のまま | `failed_drag_window_move_is_logged_as_error`（0 本） |
| 14.1 | 46 | 予備の枝の `record_button_down(window_entity, button)` を外す | `w3_left_dblclick_without_hit_records_press_on_window` |
| 14.2 | 47・60 | `screen_point` の `x + pos.x` を `x` に／`xbutton_of` の `== 1` を `== 2` に | `test_screen_point_adds_window_position_or_keeps_client_point`・`w1_left_dblclick_prepares_and_crossing_threshold_starts`／`test_xbutton_of_reads_hiword_of_wparam` |
| 14.3 | 51 | （一時のテスト・残していない）資源の挿入を消す前に「ダブルクリック → 閾値越え → 離す → 終了 1 件」 | 一時のテストが消す前に赤（left: 0, right: 1）・消した後に緑 |

較正の当たらないもの（足したテスト・直したテストが無い・説明だけ・振る舞いを変えない直し）: 1.1・2.3・5.1・5.3（既にあるテストが緑のまま）・6.2・8.1・9.2（門の内側は `#[ignore]` で既定の走行に入らない）・12.1・15.2。

### 14.7 テストの数（範囲を絞った走行・`--ignored` なし）

| タスク | 走行 | 前 → 後 |
|---|---|---|
| 1.1 | `--lib command` | 63 通過・1 無視 → 同じ（名と結果の一覧の差分 0） |
| 2.2 | `--lib window_proc::window_pos` | 30 → 31 |
| 3.1 | `--lib window_proc::window_pos` | 31 → 34 |
| 3.2 | `--lib window_proc::drop_files` | 6 → 7 |
| 4.1 | `--lib command_threadlocal_tests` | 3 → 6 |
| 4.2 | `--lib command_batch_tests` | 8 → 9 |
| 5.2 | `--lib window_handle` | 10 → 11 |
| 6.1 | `--lib window_pos` | 57 → 58 |
| 6.3 | `--lib monitor`／`monitor_systems` | 38 → 39／21 → 22 |
| 7.1 | `--lib zorder_chain_apply` | 53 → 54 |
| 8.1 | `--lib zorder_chain_order` | 11 → 11 |
| 9.1 | `--lib zorder_pair -- --list` | 97 → 97（一覧の差分 0・走行は 94 緑・3 無視） |
| 9.2 | `--lib window_band`／`always_on_top` | 16 緑・3 無視／6 緑・3 無視 → 同じ |
| 10.1 | `--lib zorder_pair_establish` | 9 → 11 |
| 10.2 | `--lib zorder_pair` | 96 緑・3 無視（改名 1・増減 0） |
| 12.2 | `--lib zorder_pair_maintain` | 33 → 32（弱いテスト 1 本を消した） |
| 13.2 | `window_proc::mouse_move`／`pointer::types` | 8 → 9／22 → 21 |
| 13.3 | `--lib window_proc::mouse_move` | 9 → 10 |
| 13.4 | `--lib window_proc::mouse_move` | 10 → 11 |
| 14.1 | `--lib window_proc` | 116 → 117 |
| 14.2 | `--lib window_proc` | 117 → 119 |
| 14.3 | `--lib window_proc` | 119 → 119 |

A-1 の 3 本（`#[ignore]`）は既定の走行に入らないまま（`-- --list --ignored` で 3 本だけ・12.1 で確かめた）。番人の 2 本は増えなかった: `zorder_chain_order_tests.rs` は 977 → 972 行（15.2 で `instance` の欄の行だけ減った）・`zorder_pair_maintain_tests.rs` は 978 → 784 行。

## 15. かけ直し（タスク 17.1・V2・2026-11-14）

- 物差し: 1 回目のギャップ分析（この文書の頭・1〜3 節）と同じ **ponytail 5.3.3** の `/ponytail:ponytail-audit`。作法のうち「最大 20 件」と「読まなかった所（Not checked）」は外し、件数の上限なしで全部読む（要件 1.4・5.4）。
- やり方: 1 回目は範囲の全部を 4 つの束に分け、束ごとに 1 人の読み手が頭から終わりまで読んだ（束 1＝`window/` のうち下の 2 つを除く 20 本・束 2＝`window/zorder_chain*` 14 本・束 3＝`window/zorder_pair*` 19 本・束 4＝`window_proc/` 20 本）。2 回目からは、前の回の直しで変わったファイルだけを読み直した（`completed/audit-wintf-runtime` の 15 節と同じ形）。返った指摘は根拠の行を読み直してから直し、確かめを直したもの・振る舞いを変えたものは較正で赤を見た。
- 基点: `HEAD` `995ab6518`（16.2 の後）。直しのコミットは 1 回目の 2 本（`cda6f1c9d`＝束 1・束 4 の 11 件・`7b11f67b2`＝束 2・束 3 の 15 件）・2 回目の `0c99f0b36`（6 件と追いの 2 点）・3 回目の `4624ad7a4`（2 件）。4 回目は直しが無いのでコミットも無い。

### 15.1 範囲の数え直し（要件 1.1・1.2）

範囲は 1 節と同じ（`git ls-files crates/wintf/src/ecs/window crates/wintf/src/ecs/window_proc`）。

| 時点 | 本番 | テスト | 計 | 行（空行を含む） |
|---|---:|---:|---:|---:|
| 着手（main `30a343eb`・1 節） | 27 | 42 | 69 | 33,925 |
| かけ直しの始め（`995ab6518`） | 27 | 46 | **73** | 33,306 |
| かけ直しの直しの後（`4624ad7a4`） | 27 | 46 | **73** | 33,373 |

- 着手との差: 消えた 0・増えた 4（`git diff --name-status --diff-filter=ADR 30a343eb..995ab6518` で確かめた。改名も 0）。増えた 4 本はどれもこの spec が作ったテストのファイルで、tasks.md 17.1 の「新しい 4 本」に当たる。`command_ui_thread.rs` は作らなかった（14.2）。

| 増えたファイル | 作った小タスク |
|---|---|
| `window/command_test_support.rs` | 1.1（`56eaff60c`） |
| `window/zorder_pair_test_support.rs` | 9.1（`205427fd4`） |
| `window_proc/window_pos_decision_tests.rs` | 2.2（`ead58d7dd`） |
| `window_proc/mouse_move_dispatch_tests.rs` | 13.2（`396240a64`） |

- **読んだ数（1 回目）: 73 / 73**（20＋14＋19＋20。範囲の数と一致）。新しい 4 本も読んだ。
- かけ直しの直しはファイルを足しも消しもしていない（`git diff --name-status --diff-filter=ADR 995ab6518..HEAD` は 0 件）。範囲の数は直しの後も 73。

### 15.2 回ごとの件数（要件 5.4）

| 回 | 基点 | 読んだもの | 必ず直す | 直したほうがよい | できれば | 計 |
|---|---|---|---:|---:|---:|---:|
| 1 | `995ab6518` | 範囲の全部 73 / 73 | **0** | **11** | **15** | **26** |
| 2 | `7b11f67b2` | 1 回目に直したファイル 35 / 35（束 A＝`zorder_*` 20・束 B＝ほか 15）と、範囲の外の 2 本（`layout/systems/monitor_systems.rs`・`monitor_systems_tests.rs`）の変えた所 | **0** | **1** | **5** | **6** |
| 3 | `0c99f0b36` | 2 回目に直したファイル 13 / 13（範囲の 11・範囲の外の 2） | **0** | **0** | **2** | **2** |
| 4 | `4624ad7a4` | 3 回目に直したファイル 3 / 3 | **0** | **0** | **0** | **0** |

- **判定: 0 件で収束**（4 回目で 0 件。5 回目の打ち切りには当たらない）。
- 各回の読み手は、前の回の指摘が全部直っていることも確かめた（2 回目＝1 回目の 26 件・3 回目＝2 回目の 6 件・4 回目＝3 回目の 2 件。どれも「全部直っている」）。
- 番号は回ごとに振った。1 回目は「R<束>-<連番>」（束 1〜4）、2 回目は「R2B-」（束 B）・「R2A-」（束 A）、3 回目は「R3r-」。3 節の 1〜63 番とは別の番号。

### 15.3 指摘と直し

「較正」は 14 節と同じ作法（本番に 1 度壊し方を入れて赤を見て、手か控えのコピーで戻し `git diff`／`cmp` で戻ったのを確かめた）。「—」は説明だけ・写しの片付けだけで、赤にする道が無いもの。

**1 回目（26 件）**

| 番号 | 組 | 場所 | 中身 | 直し | 較正（壊し方 → 赤のテスト） |
|---|---|---|---|---|---|
| R1-1 | Should | `window/window_pos/tests.rs:233-272` | 生成のテストの期待値を、調べる関数そのもの（`overlapped_both_given`）で出していた。幅から `-frame.left` を抜いても緑 | 期待値を `AdjustWindowRectExForDpi`（`RECT{500,50,800,250}`・`WS_OVERLAPPEDWINDOW`・96）から直に出す `overlapped_expected` に替え、両方を与えたときのテストを足した | 幅から `- frame.left` を抜く → `..._both_given_adds_frame`・`..._size_only_adds_frame_to_size`。y を `position.y + frame.left` に → `..._both_given_adds_frame`・`..._position_only_adds_frame_to_position` |
| R1-2 | Should | `window_band_pipeline_real_tests.rs`・`window_band_real_tests.rs` | `insert_after`・`raise_to_top`・`CAPTURE_DIRECTIVES`・`FIX_TAG`・`VERIFY_FAILED_TAG` が `zorder_pair_test_support.rs` の写し | 写しを消して共有のものを使う（`#[ignore]`・門・テスト名は同じ） | —（消す前の grep はファイルの中だけ） |
| R1-3 | Should | `transition_diag.rs`・`window_band_pipeline_real_tests.rs` | 定義が areka-placement にあるもの（`PLACEMENT_KIND_ALL`・`PlacementRoute`・`wire_zorder_pair` ほか）を「areka」と書く | areka-placement に直した（判定器が本当に `areka` にある 1 か所は残した） | — |
| R1-4 | Nice | `window_pos/mod.rs:232-233,:286-287` | `build_flags_for_system`・`get_hwnd_insert_after` の説明が呼び手を 1 つだけ挙げ「（公開用）」と書く | 本番の呼び手 3 つ（`apply_window_pos_changes`・`nudge_command`・`pair_fix_command`）を挙げた | — |
| R1-5 | Nice | `monitor.rs:244-254`・範囲の外の `layout/systems/monitor_systems.rs` | 列挙が失敗し続けると `error!` が毎刻み出る | クレートの中だけの `try_enumerate_monitors`（ログを出さない）を足し、刻みの system は `monitors_or_log_failure` で続きの最初の回だけ `error!` を出す。公開の `enumerate_monitors` は形もログも同じ | `if !zero_streak` を `if true` に → `enumeration_failure_is_logged_once_per_streak` |
| R1-6 | Nice | `command.rs`・`command_batch_tests.rs`・`command_transition_tests.rs` | 消えた `flush` を名指す | `flush_window_pos_commands` に直し、頭の一覧に `flush_window_pos_commands`・`drain_window_pos_commands` を足した | — |
| R1-7 | Nice | `components.rs:3-6` | 頭の一覧が `window_handle.rs` の `WindowHandle` を挙げ、`OnCloseRequest` ほか 5 つが抜ける | 一覧を実物に合わせた | — |
| R1-8 | Nice | `window_band.rs:7-8` | 「`HWND_TOPMOST` を書くのはここだけ」が誤り | ほかに `tooltip/os.rs` と `WindowPos::get_hwnd_insert_after` があると書いた（`ZOrder::TopMost`／`NoTopMost` を積む本番の呼び手は grep で 0） | — |
| R2-1 | Should | `zorder_chain.rs:23-29,:546`・`zorder_chain_tests.rs:631,:678` | 記録の出口を集める理由が「grep の対象が割れる」（34 番と同じ偽の理由） | `zorder_pair.rs:21-22` と同じ言い方に直した（検査は同じ） | — |
| R2-2 | Should | `zorder_chain_diag.rs`・`zorder_chain_diag_tests.rs` | 区間の `Some`／`None` の説明が呼び手の事実と逆 | 繋いだ行・外した行は常に `Some`、張り失敗は `restore_pair` が `None` と書き、外し失敗に区間を載せない理由を直した | — |
| R2-3 | Should | `zorder_chain_apply.rs:33-36,:870` | 挿入の位置が取り下げた nudge の形のまま。「実窓の檻は別 task」が古い | 「錨（根の 1 つ手前）か先頭」・「下の `zorder_chain_order_*_tests`」に直した | — |
| R2-4 | Should | `zorder_chain_apply_tests.rs:14` | 盲点の指し先が `zorder_pair_establish_tests.rs:142-152`（9.1 で移った） | `zorder_pair_test_support.rs` のモジュール文書「記録の捕捉」を指す | — |
| R2-5 | Nice | `zorder_chain_diag_tests.rs:142` | 「8 つ目・3 つ目」 | 「名簿の外のタグ」に | — |
| R2-6 | Nice | `zorder_chain_order_lifecycle_tests.rs:26,:171-175` | 本番の順から `apply_window_band` が抜ける | `wire_zorder_pair` どおりの 4 つの順に書いた | — |
| R3-1 | Should | `zorder_pair_tests.rs:427,:655,:679` | 1 要素の `for`・古い「確立・再断行」 | `let trigger = PairTrigger::Reassert;`・「再断行」に（名・assert は同じ） | — |
| R3-2 | Should | `zorder_pair.rs:159`・`zorder_pair_deferred_vocabulary_tests.rs:29` | 「areka main が挿入」「areka の main.rs の結線 1 行」 | `areka_placement::spawn::wire_zorder_pair`（areka の `ghost_session.rs` から呼ぶ）に | — |
| R3-3 | Should | `zorder_pair.rs:629-642` | `measure_below_is_always_on_top` の失敗の戻り（偽）にテストが無い | 既定の走りのテストを `zorder_pair_maintain_always_on_top_tests.rs` に足した（帯の中→真・ゴースト形→偽・`HWND::default()`→偽と記録 1 本） | 失敗の戻りを `true` に → `measuring_the_character_band_answers_both_ways_and_falls_back_out_of_the_band_on_failure` |
| R3-4 | Should | `zorder_pair_test_support.rs:108-135` | `create_window` が `crate::api::create_test_window` の写し | その呼び出しにした（`CreateWindowExW` の import も外した） | — |
| R3-5 | Nice | `zorder_pair_*_tests.rs` の 6 本 | 共有の道具の写し（`SIGNOFF_DIRECTIVES`・`fake_hwnd`・`lines_with`・`released_skips`・`count`・生の `DestroyWindow`・`create_hidden_popup`・`Trio`） | 共有の 1 つにまとめた（形の違う `measure_tests.rs` の `Trio` は残した・テスト名は同じ） | —（消す前の grep はファイルの中だけ） |
| R3-6 | Nice | `measure_tests.rs:36-44`・`always_on_top_tests.rs:69` | 入れ替わった `as` の別名 | 共有の名をそのまま使う | — |
| R3-7 | Nice | `establish_tests.rs:499-508`・`sink_tests.rs:645-648` | `!out.is_empty()` だけの弱いテスト | 前者は消した（同じ枝は `measure_tests.rs` が 1 本で数える）。後者は「手前側の走査に失敗しました」が 1 本と数える | 失敗の枝の `warn!` を外す → `measure_windows_in_front_reports_failure_as_an_incomplete_chain`（1 本のはずが 0 本） |
| R3-8 | Nice | `zorder_pair_test_support.rs:76` | `REAL_WINDOW_ZORDER_ENV` が `pub(super)` だが中だけで使う | 私有に | — |
| R3-9 | Nice | `zorder_pair_maintain.rs:406-409` | 宣言の無い要求を落とすとき `ReassertZOrder` だけ消し、`IssuedPairFix` が残る（説明 :113 は両方と書く） | `remove::<(ReassertZOrder, IssuedPairFix)>()`。テスト `dropping_an_orphan_request_also_drops_its_issued_fix` を足した | 直す前に足したテストが赤（`IssuedPairFix` が残る）→ 直して緑 |
| R4-1 | Nice | `window_proc/mouse_click.rs:228-241` | `screen_point` が既定の `Some(CW_USEDEFAULT)` を足す（debug で桁あふれの panic になりうる） | 位置の x が `CW_USEDEFAULT` なら足さない（`filter`）。既にあるテストに assert を 1 つ足した | フィルタを抜く → `test_screen_point_adds_window_position_or_keeps_client_point` |
| R4-2 | Nice | `window_proc/window_pos.rs:3-4` | センター補正がここにあると書く | `dpi_helpers.rs` にあり、ここは呼ぶだけ、と直した | — |
| R4-3 | Nice | `window_proc/mouse_move.rs:343-352` | 破棄済みの窓で `get_entity_mut` が失敗しても `warn!` を出して位置を積む | `DESPAWNED_SKIP_TAG` の `debug!` を残して積まない（届かなくなる `warn!` の枝は消した）。テスト `mouse_move_to_a_despawned_window_skips_with_debug` を足した | 破棄済みの枝で積む側へ倒す → 同テスト（`Sample added` の assert。この assert を足す前は緑のままだったので足した） |

**2 回目（6 件と追いの 2 点）**

| 番号 | 組 | 場所 | 中身 | 直し | 較正 |
|---|---|---|---|---|---|
| R2B-1 | Should | `window_proc/window_pos.rs:512`・`window_pos_tests.rs:246,:672` | R1-7 で行がずれ、`window/components.rs:29-31` を指す 3 か所が外れた | 行番号でなく「`window/components.rs` の `DpiChangeContext` の doc」と名で指す | — |
| R2B-2 | Nice | `monitor.rs:250-254`（R1-5 の続き） | 1 台ずつの情報取得の失敗の `error!` がまだ毎刻み出る | `try_enumerate_monitors` はログを出さず、失敗の列を列挙の成否によらず返す。刻みの system は続きの最初の回だけ 1 本にまとめて出す。公開の `enumerate_monitors` のログは前と同じ順と形 | `!zero_streak &&` を外す → `monitor_info_failures_are_logged_once_per_streak`（1 本のはずが 3 本）。追い: 列挙の失敗で先に戻る形に → 同テスト（1 本のはずが 0 本） |
| R2A-1 | Nice | `zorder_pair_maintain.rs:517,:530`（R3-9 の続き） | 見送りの枝と計画の組めない枝が `ReassertZOrder` だけ消す | 両方で `remove::<(ReassertZOrder, IssuedPairFix)>()`。既にあるテスト `request_for_a_missing_peer_issues_no_command` に持ち越しを先に置いて確かめる。追い: 届かない枝は黙って落とさず `debug!` を 1 行出す | 見送りの枝を `remove::<ReassertZOrder>()` に戻す → 同テスト |
| R2A-2 | Nice | `zorder_pair_deferred_vocabulary_tests.rs`・`zorder_pair_maintain_always_on_top_tests.rs` | 「帯を書くのは `window_band.rs` だけ」が R1-8 の事実と食い違う | 「ゴーストの窓の帯を書くのは `window_band.rs`（ほかの書き手はその頭の doc）・`zorder_*` は帯を書かない」に（説明と assert の文だけ） | — |
| R2A-3 | Nice | `zorder_chain_diag.rs:205` | 宙に浮いた「この検索」 | 「適用系（`execute_ops`）が望む鎖から区間を引く検索」に | — |
| R2A-4 | Nice | `zorder_chain_tests.rs`・`zorder_chain_diag_tests.rs`・`zorder_chain_release_tests.rs` | ローカルの `fake_hwnd` 3 本（共有と同じ本体） | `zorder_pair_test_support::fake_hwnd` を使う | —（消す前の grep はファイルの中だけ） |

**3 回目（2 件）**

| 番号 | 組 | 場所 | 中身 | 直し | 較正 |
|---|---|---|---|---|---|
| R3r-1 | Nice | `zorder_chain_diag_tests.rs:242` | assert の文の理由が「grep 対象が分裂する」（34 番・R2-1 と同じ偽の理由） | 「出力先が割れて RUST_LOG の指定から外れうる・写しが食い違う」に（判定は同じ） | — |
| R3r-2 | Nice | `zorder_pair_maintain.rs:534-538` | R2A-1 の追いで足した `tracing::debug!` の直書きが「記録のマクロは `zorder_pair.rs` に置く」決まりを破る | `zorder_pair.rs` に `pub(crate) fn log_pair_fix_unplanned(entity, peer)` を足して呼ぶ（文言・水準・欄は同じ。maintain の直書きは grep で 0） | —（届かない枝・同列の記録の関数も出力先のテストを持たない） |

### 15.4 要件 4 で残すと裁定した指摘

**0 件**（無し）。4 回の 34 件は全部直した。

決めどおり残したもの（指摘に数えない。読み手も指摘にしなかった）:

- A-1 の 3 本の隔離（`#[ignore]` と環境変数の門・議題 9）。かけ直しの直しでも `#[ignore]` と門の呼び出しの差分は 0 行。
- 案 B（`ExplicitMaintenance`）の語（28 番の決め・`zorder_pair_deferred_vocabulary_tests.rs` が守る）。
- `ecs/drag/state/mod.rs` の `check_threshold` とそのテスト 5 本は `audit-wintf-input` の持ち物（門 GI ⒝）。説明の古さもあちらが直す（14 節の 13.4 の行）。

### 15.5 範囲の外で見つけたもの（数えない）と申し送り

どれも作業枝の木（`4624ad7a4`）で `git grep`／行の読み直しをして、まだ残っていることを確かめた。直したもの（R1-5・R2B-2 の `monitor_systems.rs`）は下の 15.6。

| 相手 | 場所 | 中身 |
|---|---|---|
| `audit-wintf-input` | `ecs/pointer/nchittest_cache.rs:25,:142` | WM_MOUSELEAVE の受け手を「handlers.rs L820-876」と指す古い参照 |
| `audit-wintf-input` | `ecs/pointer/buffers.rs:144-179` | `POINTER_BUFFERS` の窓ごとの項目が窓の破棄の後も消えない |
| `audit-wintf-input` | `ecs/drag/state/mod.rs:618-621` | `check_threshold` の説明が「本番呼び出しがゼロ」「`mouse_move.rs` 内にインライン複製」のまま（62 番の直しで事実に合わなくなった・門 GI ⒝ であちらが直す） |
| `audit-placement-follow` | `areka-placement` の `follow/drag_follow.rs:486`・`follow/window_move.rs:255`・`follow_visibility_char_wiring_tests.rs:660` | `CW_USEDEFAULT` の確かめとして `monitor_systems.rs:408` を指すが、その行は今 436 行目あたり（前から外れていた・R1-5 でさらにずれた） |
| `audit-emo2-boot-frame` | `crates/areka/src/runtime/frame_transition_atomicity_tests.rs:47,:527` | `window_pos.rs:464`・`:468-489` の行番号が古い（今の `guarded_set_window_pos` の呼び出しは 475 行目） |
| `audit-emo2-boot-frame` | `crates/areka/src/runtime/frame/zorder_drain.rs:52`・`zorder_descript.rs:33`・`zorder_drain_tests.rs:443` | 34 番・R2-1 と同じ種類の偽の理由（「grep 対象が 2 本に割れる」） |
| 完了した `audit-wintf-layout-widget`（完了の棚卸） | `ecs/layout/hit_test/mod.rs:509-513` の `hit_test_in_window_ex` | R4-1 と同じく、位置が `CW_USEDEFAULT` でも足す |
| 完了した `audit-wintf-runtime`（完了の棚卸） | `runtime/wndproc_bridge.rs:91-104` | World の借用中に届いた `WM_CLOSE` を既定の手続きへ渡す |
| 完了した `audit-wintf-runtime`（完了の棚卸） | `api.rs:302` の `create_test_window` | 説明は「見えない」だが、子の窓には `WS_VISIBLE` が付く |
| 完了した `audit-wintf-runtime`（完了の棚卸） | `api.rs:435` | テストの中だけの `insert_after` が `zorder_pair_test_support.rs` の同じ関数の写し |
| 完了した `audit-emo-present`・`audit-emo-text-*`（完了の棚卸） | `areka-emo-present`・`areka-emo-text` の 9 か所 | もう無い `emo2_boot` のパスを指す説明 |
| この spec（完了の棚卸） | `tasks.md:271`（16.1） | `_Requirements:` の 8.8 は requirements.md に無い（要件 8 は 6 まで） |
| この spec（完了の 0.x の知らせ） | — | 6.1（2 番）の 1 行は 14.3 の終わりにある。完了のときに版の知らせへ写す |

### 15.6 範囲の外のファイルへ及んだ直し

| ファイル | 番号 | 中身 |
|---|---|---|
| `crates/wintf/src/ecs/layout/systems/monitor_systems.rs` | R1-5・R2B-2 | `detect_display_change_system` が `try_enumerate_monitors` を呼び、`monitors_or_log_failure` で列挙の失敗と情報取得の失敗を続きの最初の回だけ記録する（6.3 で触ったファイル） |
| `crates/wintf/src/ecs/layout/systems/monitor_systems_tests.rs` | R1-5・R2B-2 | `enumeration_failure_is_logged_once_per_streak`・`monitor_info_failures_are_logged_once_per_streak` を足した |

- 公開の口の形は変えていない（`enumerate_monitors` の署名とログ・`detect_display_change_system` の引数は同じ。`try_enumerate_monitors`・`monitors_or_log_failure` はクレートの中だけ）。14.3 の表への追記は無い。

### 15.7 テストの数（範囲を絞った走行・`--ignored` なし）

直す前の数は測っていないので、直した後の数と、足した・消した本数だけを書く。

| 走行 | 直した後 | 足した・消した |
|---|---|---|
| `--lib window_pos` | 60 緑 | +1（R1-1。R4-1 は既にある本へ assert を足しただけ） |
| `--lib monitor` | 41 緑 | +2（R1-5・R2B-2 で 1 本ずつ） |
| `--lib window_proc` | 120 緑 | +1（R4-3） |
| `--lib zorder_pair` | 96 緑・3 無視 | +2・−1（R3-3・R3-9 で足し、R3-7 で消した） |
| `--lib zorder_chain` | 123 緑 | 0 |
| `-p areka zorder_inversion_branch_coverage` | 3 緑 | 0 |

- テストの関数名は、消した 1 本（`measure_window_above_maps_failure_to_absence_and_records_it`・消す前の grep は `crates/` で定義の 1 か所だけ）と足した 6 本のほかは変わっていない（変えたファイルの `#[test]` の名を `995ab6518` と突き合わせた）。A-1 の 3 本は無視のまま。
- 直しの各回で build（wintf・areka・areka-placement の tests と examples）は警告 0、`cargo fmt --all -- --check` は通った。

### 15.8 静的な約束とビルド・テスト（タスク 17.2）

差分の基準は `git merge-base HEAD origin/main`＝`73c4f87a0`。

| 確かめ | 調べ方 | 結果 |
|---|---|---|
| 1,000 行の番人 | `cargo test -p log-capture-kit --test file_length_guard_test` | 6 緑・赤 0。例外表 `OVER_LIMIT_ALLOWED`（同じファイルの `:61`）は `git diff 73c4f87a0 -- crates/log-capture-kit` が 0 行で、足したファイルは無い。変えた `.rs` の最長は `command.rs` の 986 行 |
| `Cargo.toml`（全部）・`Cargo.lock`・`vendors/` | `git diff 73c4f87a0 --stat -- '*Cargo.toml' Cargo.lock vendors/` | 0 行 |
| `MonitorRecord` の定義 | 基準と今の `pub struct MonitorRecord {`〜`}`（`transition_diag.rs:403`・14 行）を awk で切り出して突き合わせ | 同じ。このファイルの差分の塊は `:5`・`:82`・`:98`・`:143`・`:330`・`:334`・`:806` だけで、構造体にも impl にも掛からない |
| `Monitor` の定義 | 同じく `pub struct Monitor {`（`monitor.rs:68`・7 行）を突き合わせ | 同じ。差分の塊は `:16`・`:30` と `:232` から後の自由関数（`enumerate_monitors` と足した自由関数 `try_enumerate_monitors`）だけで、`impl Monitor`（`:76`〜`:109` から）は 0 行 |
| `DESPAWNED_SKIP_TAG` の行とその説明 | `git diff 73c4f87a0 -U0 -- crates/wintf/src/ecs/window_proc/lifecycle.rs` の塊の位置 | 塊は `:82`・`:95`・`:324`・`:383` だけで、説明（`:20` 台〜`:37`）と定義（`:38`）は 0 行。`crates/` の差分に出るこの名の `+` 行 6 本は `mouse_move.rs` とそのテストで定数を使う側だけ |
| `areka_placement::diag` を名指す綴り | `git diff 73c4f87a0 -- crates \| grep -cE '^[+-].*areka_placement::diag'` | 0 行 |
| ビルド | `cargo build -p wintf -p areka -p areka-placement --tests --examples` | 通った・警告 0 |
| `cargo test -p wintf` | `--ignored` なし | 1,743 緑・赤 0・無視 48（A-1 の 3 本は無視のまま） |
| `cargo test -p areka-placement` | 同じ | 1,008 緑・赤 0・無視 1 |
| `cargo test -p areka` | 同じ | 本体 3,171 緑・無視 6、`embedded_icon_test` 2 緑、`emo2_first_load_repeat` 19 緑、`offline_dump_exe_test` 13 緑（別に回した）。`emo2_mcp_real_run` の 1 本が赤（下）。cargo がそこで止まったので、`emo2_real_run`・`smoke_boot_loop_exit`・`startup_install_real_run`・`stay_on_top_real_run`・`window_link_real_run` は回っていない |

- `emo2_mcp_real_run::emo2_mcp_get_log_origin_and_strict_over_one_real_boot` の赤は、32bit の `shiori-host32-helper.exe` がこのワークツリーの `target\i686-pc-windows-msvc\` に建っていないため、起こす前に `real-run-kit/src/i686.rs:77` で止まったもの（0.08 秒・窓は出ていない）。この試験は env の門が無く既定で走り、emo2 を本当に起こす。この spec の変更とは関わらない。
- ここで確かめていない areka の試験のバイナリは 6 本＝`emo2_mcp_real_run`・`emo2_real_run`・`smoke_boot_loop_exit`・`startup_install_real_run`・`stay_on_top_real_run`・`window_link_real_run`。どれも areka を本当に起こす（`smoke_boot_loop_exit` は env を自分で立てて窓を開き、`emo2_mcp_real_run` は 32bit の helper も要る）ので、排他テストの机と i686 の helper が要り、机を持たないこのタスクでは回さなかった。2 度は回さず、helper を建ててから回すタスク 17.3 の `tools/test-all.ps1` の全体テスト 1 回で確かめる。
