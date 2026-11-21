# ギャップ分析: audit-areka-input-menu（ponytail-audit・2026-11-21）

## 1. 要約

- **やり方**: `/ponytail:ponytail-audit`（5.3.3）の作法から「最大 20 件」と「読まなかった所」を外してかけた（要件 1.2・1.3）。範囲は `crates/areka-input/` だけ。本番 19 ファイルと `Cargo.toml`・`cursor/README.md` はここで全行を読み、テスト 30 ファイルは 3 つの下請けに分けて全行を読ませ、その報告を本番のコードと突き合わせて番号を振り直した（重い走行は回していない。`cargo doc -p areka-input --no-deps` だけ 1 回回した）。
- **結果**: 指摘は 77 件（必ず直す 0・直したほうがよい 52・できれば 25）。本番の振る舞いを壊すバグ・危ない所・負荷で壊れる所は見つからなかった。指摘の芯は ⑴ 落ちる力の無いテスト（守っているつもりの檻が本番を通らない・表明が常に真）⑵ 本番の写し（選択の送り出し 2 か所・強調の当て方 3 か所・送出の警告 4 か所）⑶ 古い説明とコメントの決まり（本体の中にあった頃の言い方・設計の札・変わった経緯の語）。
- **SSP 互換・ukadoc の正典に関わる直し（要件 2.2 の 3 種類目）は 0 件**。SHIORI へ送るイベントの名・参照・送る条件はどの直しも変えない。
- **預かった件 1〜11 は全部まだ在る**。行番号は件 5（170〜181 → 167〜173）のほか動いていない。件 7 の警告は 20 件でなく **19 件**（私有へのリンク 18・解けないリンク 1）。件 8 の「狭める」は **0 件**（開けた項目は全部、本体の本番か本体のテストに使い手がある）。
- **議題の芯**: 落ちの報告の分担（`test-crash-report-merge` の brief の誤り）、範囲の外の crate（`areka-runtime`・`areka-placement`）の持ち物を試すテスト 3 群の扱い（C24 の約束 ⑶ で動かせない）、ログの記録の名と出所（モジュールの道）を写しの寄せで変えないこと。

## 2. 数え直し（要件 1.6）と行番号の当て直し（要件 1.7）

着手のときの HEAD は `0ea5b477`（親 `f6405e09`＝`main`）。`crates/areka-input/src/` を `wc -l` で数えた。

| 区分 | ファイル | 行 |
|---|---|---|
| 本番（`_tests.rs`・`_test_support.rs` 以外） | 19 | 4,851 |
| テスト（`_tests.rs`・`_test_support.rs`） | 30 | 10,312 |
| 合計 | 49 | 15,163 |
| ほか | `Cargo.toml` 46 行・`cursor/README.md` 75 行・絵 4 つ（`hand.cur`・`grip.cur` 各 236,790 バイト・`hand.svg` 5,956・`grip.svg` 1,367） | |

要件の表と一致する。注意 1 つ: 本番に数えた `choice_drain.rs`（545 行）の 176〜545 行（370 行）は `#[cfg(test)] mod tests` で、中身はテスト。1,000 行に近いのは `balloon_pointer_handler_tests.rs` 874・`user_break_tests.rs` 853・`shell_box_handler_tests.rs` 719・`input_events_tests.rs` 699（越えるものは 0）。

行番号の当て直し（今の HEAD）:

| 件 | brief／要件の行 | 今の行 |
|---|---|---|
| 1 | `lib.rs` 508 | 508（動いていない） |
| 3 | テスト 3 本の説明 | `input_events_tests.rs` 364〜366・416〜417（表明 377〜383・408〜413・434）、`input_events_menu_tests.rs` 161〜162（表明 198） |
| 4 | `input_events_tests.rs` 362 | 362 |
| 5 | `spawn_assembly_tests.rs` 170〜181 | **167〜173**（「入力の側」の字面は PR#488 で消え、今は 167「input_events→placement」・171「input_events の檻」と、同じ crate の中を `areka_input::…` と外の道で書く 168・170） |
| 6 | `collision-probe.rs` 63・`setup.rs` 75〜76 | 63・75〜76 |
| 7 | 警告 20 件 | **19 件**（下の 5 節） |
| 9 | `user_break.rs` 201 | 201 |
| 10 | `choice_drain.rs` 253・`balloon_test_support.rs` 185 | 253・185 |
| 11 | `drag_test_support.rs` 283 | 283 |

## 3. 読んだ範囲（要件 1.3）

- 本番 19 ファイル: 全行（この報告の書き手）。`Cargo.toml`・`cursor/README.md`: 全行。
- テスト 30 ファイル: 全行（下請け 3 本・各ファイル「全行読んだ」の申告つき。判断に要った本番の箇所はこの書き手が読み直して確かめた）。
- `cursor/` の絵 4 つ: 中身は読まない（仮定 3）。置き場・名前・使われ方（`cursor_answer.rs:28-29` の `include_bytes!`・`tools/cursor-art.ps1:48` の `crates/areka-input/cursor`）と `README.md` の記述を突き合わせた（指摘 50）。
- 範囲の外は読むだけ: 使い手の数え（本体・`areka-runtime`・`areka-placement`・`real-run-kit`）、`areka_runtime::spine::join_bounded`、`crash_report_test_support`、`test-crash-report-merge` の brief、C24 の行。

## 4. audit の結果

**What this repo does**: `areka-input` は、キャラクターの窓とバルーンの窓に届いたポインタ・ホイール・クリック・ドラッグを、当たり判定の名と SHIORI へ渡す座標に解いて kanade へ送る層。あわせてカーソルの形、選択肢・アンカーの確定、利用者の中断（左ダブルクリック）、箱（シェルの上の文字の箱）の強調と送りを受け持つ。**想定した負荷**: 利用者 1 人のデスクトップ・UI スレッド 1 本・毎秒約 60 回の巡回・キャラクターの窓は 2 枚（スコープ 0・1）。この負荷で壊れる所は無かった。

各指摘は「場所｜何が問題か（具体例）｜直し」。末尾の［ ］は重なる預かりの件・議題の番号。

### 必ず直す（0 件）

バグ・危ない所・データの喪失・想定の負荷で壊れる所は無かった。下請けが「必ず直す」と付けた 2 件（指摘 1・2）は、本番の振る舞いは正しく、テストが落ちる力を持たないだけなので「直したほうがよい」へ下げた。

### 直したほうがよい（52 件）

**A. 落ちる力の無いテスト・弱い表明**

1. **÷k の檻が本番を通らない** — `balloon_pure_core_tests.rs:106-260`・`balloon.rs:208-211` の doc｜247〜253 行は自分で x/2 を計算して矩形の外と確かめるだけで、本番の関数を通らない。`balloon_moved.rs:73-74`・`balloon_pressed.rs:88-89` に `/k` を足しても全部緑。ハンドラを通るテストは k=1.0 だけ（`balloon_choice_settle_tests.rs:186`）。186〜212 の持ち上げの檻は `areka-emo-text` の `choice_tests.rs:381-476`（k=2）と同じ所を見ている｜(b) と持ち上げの檻を消し、`choice_settle` の GPU のテストを拡大率 2.0 で 1 本足してハンドラ経由で当たることを見る（または doc の「檻が固定する」を消す）。
2. **窓の件数の表明が落ちない・説明が誤り** — `input_events_tests.rs:364-366,377-383,408-413,416-417,434,684,697`・`input_events_menu_tests.rs:161-162,198`｜このクレートのテストの World には `InputHooks` が無く、強制退避の枝へ入っても `hooks::escape`（`hooks.rs:12-24`）は `error!` と `AppExit.request_exit()` だけで窓は消えない。`ghost_count` の表明はどの場合も真。落とす力は `try_recv` の表明だけ｜件数の表明を `!AppExit.is_requested()` に替え（`pointer_test_support::world_with_wiring` は `AppExit` を挿している）、説明を「枝へ入れば終わりの指示が立ち、`try_recv` も空になる」に直す。［件 3］
3. **`try_borrow_mut().is_ok()` は常に真** — `balloon_pointer_handler_tests.rs:171-174,642-645,755-758`・`balloon_leave_tests.rs:69-72`｜借用はハンドラの中で終わるので必ず成功する。注入が runtime に届いたかは誰も見ていない（`balloon_moved.rs:144`・`balloon_exit.rs:154` の `inject_choice_hover` を消しても緑）｜4 か所の表明を消し、注入の到達は指摘 5 のテストで強調の状態として見る。
4. **別々の構造体の独立を確かめるテスト** — `balloon_hover_flag_tests.rs:125-152`｜`BalloonWiring` と `BalloonHover` は別クレートの別の型で、漏れる道が無い｜消す。
5. **index と ordinal が食い違う行をハンドラ経由で見るテストが無い** — `balloon_moved.rs:120`｜テストの行はどれも ordinal==index。`balloon_pass_through_tests.rs:37-43` の `apply_last` は本番の写しなので `.map(|i| i)` に壊しても緑。例: 選択肢 0 が 2 行に折り返すと行は [0,0,1] で、3 行目の上で実在しない ordinal 2 を注入する｜選択肢を折り返させた GPU の組み立てで、移動ハンドラ経由で `hover(0)==row.ordinal` を見るテストを 1 本足す。
6. **`wire_balloon_choice` が入れ直す 2 つの持ち物を確かめていない** — `balloon_wiring_tests.rs:262-290`／`balloon.rs:387,390`｜`BalloonHover` を入れ忘れると `balloon_moved.rs:108`・`balloon_exit.rs:87` は `if let` で黙って飛ばす｜`BalloonHover`・`ShellBoxHover` の在の表明を 2 行足す。
7. **送った量を固定しない表明** — `balloon_wheel_tests.rs:84-96,122-129`・`balloon_pressed_tests.rs:182-184`｜「Moved」は 1 行以上送ったことしか示さない。持ち越しが壊れて 2 行送りになっても通る｜各テストの最後に `matches!(newer_by_one(..), ScrollChange::Stopped(_))` を足す。
8. **実機のログ grep の的（INFO `choice_selected`）をバルーンのハンドラ経由で見るテストが無い** — `balloon_pressed.rs:158,171-178`｜`balloon_choice_settle_tests.rs:206-211` の 1 回目の押下はログを捕まえていない（`anchor_selected` は `shell_box_handler_tests.rs:648` だけ）｜1 回目の押下を `capture_logs` で包み、INFO が 1 行出ることを見る。
9. **ホイールの溜めの起こし直しの枝にテストが無い** — `char_watch.rs:250-254`｜この行を消しても全部緑。そうなると、カーソルを止めた後の溜めは次の入力まで吐き出されない｜`pending_wheel_is_flushed_on_a_later_run_after_the_clock_advances` の Wait の後で `tick_wake::take` を呼び、期限が入っていることを見る。
10. **トークの番号の結線にテストが無い** — `user_break.rs:259,315,323-327,372,396`｜どのテストも `talk_id: None` を期待し、`user_break_talk_id_tests.rs` は `send_break` を単体で呼ぶだけ。入口が常に `None` を渡すように壊れても緑で、kanade は今のトーク（別のトーク）を止めてしまう｜見せる番号を置ける組み立てで、`press_with_visibility` と `on_box_press` が `Some(n)` を運ぶことを 1 本で見る（`RuntimeWiring::shown_talk_id` を置く口が `areka-runtime` の test-support に在るかは設計で確かめる＝要件 7.4 の範囲）。
11. **箱のテストの組み立てが壁の時計を使い、表明を弱めてある** — `shell_box_handler_tests.rs:131-138,383`｜`MouseWiring::new` は `Instant` を読み（`lib.rs:105-106` は「ユニット檻からは使わない」）、そのため 383 は `moves >= 1`。2 打目が届くかは機械の速さしだい｜`pointer_test_support::world_with_wiring`（注入の時計）に替え、383 を `== 1` にする。
12. **同梱の手の形の表明が「指差しでない」しか見ない** — `cursor_answer_tests.rs:107-108`｜`BUNDLED` の並びが入れ替わり Bust の Up が grip を出しても緑｜`bundled(0, BUNDLED[0].1, cursor_size_for_dpi(96))` と `assert_eq` で比べる。
13. **離脱の処理を通らない表明** — `shell_box_tests.rs:504-508`（487 の後半の文）｜`hover.set(1, None)` の直後に `get` が None を見るだけ。離脱は `shell_box_handler_tests.rs:415` が見ている｜507〜508 と 487 の後半を消す。

**B. 本番の写し・混ざった仕事**

14. **選択の送り出しの写しと二重の記録** — `balloon_pressed.rs:155-191`・`shell_box_handler.rs:318-359`・`balloon.rs:152-164`｜送る・済みにする・INFO を残す・失敗を残す、が 2 か所に写してある。しかも失敗のときは `send_selection` が WARN `choice_selection_send_failed` を出し、呼び手が同じ名の ERROR をもう 1 行出す（1 件の失敗で名の同じ記録が 2 つの水準で 2 行）。この失敗の道のテストは 0 本｜送り出しを `balloon.rs` の 1 つの関数に寄せ（箱は `box` の欄だけ足す）、`send_selection` は `bool` を返すだけにして記録は 1 行にする。受け口を落として記録が 1 行だけ出るテストを 1 本足す。［議題 5・6］
15. **強調の決定の当て方が 3 か所にある** — `balloon_moved.rs:133-166`・`balloon_exit.rs:143-176`・`shell_box_handler.rs:190-224`｜`HoverAction` を受けて注入・自前の記録・`debug!` を行う同じ腕が 3 つ。片方だけ直すと食い違う｜バルーンの 2 つを 1 つの関数にまとめる（箱は場所の鍵と記録の名が違うので、場所を引数に取る形で 3 つ目も寄せられるかを設計で決める）。記録の名（`choice_hover_inject`・`box_choice_hover_inject`）は変えない。［議題 5］
16. **送出と警告の写しが 4 か所にある** — `lib.rs:234-242,294-301,308-316`・`drag.rs:165-172`｜どれも「送る・失敗なら `mouse_send_failed` を warn」。`send_mouse` が既に在る｜残り 3 か所を `send_mouse` 経由にする（記録の名と欄は同じ）。
17. **結線漏れの error が 1 巡に 2 回出る** — `balloon_exit.rs:85-98` と `116-127`｜`BalloonWiring` が無いと、93 行で 1 回、さらにスコープごとに 121 行で `balloon_wiring_missing` が出る｜頭で 1 回だけ見て、無ければ記録して戻る。

**C. テストの写し・要らないテスト**

18. **`balloon_pass_through_tests.rs` 全体（167 行）が要らない** — 冒頭 8〜16 行の「実ハンドラ経由は構造的に不可能」は `balloon_choice_settle_tests.rs:163-242` が実 GPU で通しているので誤り。1 本目は本番の写しで回すだけ、2 本目は `pure_core` の 3 本と同じ。一度だけ送ることは `balloon_wiring_tests.rs:74-112` が見る｜ファイルと `balloon.rs:415-417` を消す。
19. **キャッシュを入力に持たない関数の「stale」2 本** — `balloon_pure_core_tests.rs:434-447,449-472`｜`click_non_hit_returns_none`（412）・`click_hit_builds_selection_from_current_row`（382）と同じ入力の型｜2 本を消す。
20. **シナリオ表と重なる 6 本** — `balloon_pointer_handler_tests.rs:43-55,59-80,85-107,289-302,344-365,370-391`｜547〜652（移動）と 660〜765（押下）の表がより強く見ている｜6 本（約 130 行）を消す。
21. **derive を確かめるだけの 3 本** — `balloon_wiring_tests.rs:20-64`｜消す。
22. **系の数の重なり** — `balloon_wiring_tests.rs:292-310` と `312-329`（286〜289 で確かめ済み）｜295〜310 を消す。
23. **配置のテストの支えの写し** — `balloon_wiring_tests.rs:157-191`｜`areka_placement::spawn::test_support::two_scope_placements`／`titles`（`spawn_test_support.rs:17-49`）の写し。dev-dependency で test-support が開いている（`Cargo.toml:46`）｜写しを消して import する。
24. **`capture_logs` の写し** — `choice_drain.rs:240-256` と `balloon_test_support.rs:169-188`｜本体の 1 行（`capture_lines(LineFormat::LevelFields, f)`）も説明の塊も同じ｜`choice_drain` が `balloon::test_support::capture_logs` を使う（`pub(super)` → `pub(crate)`）。説明の塊は `log_capture_kit` の doc へ委ねて両方消す。［件 10］
25. **バルーンのテストの組み立ての写し** — (a) `BalloonWiring::new(tx)`＋`BalloonHover::default()` の挿入: `balloon_hover_flag_tests.rs:40-45`・`balloon_leave_tests.rs:31-39`・`balloon_pointer_handler_tests.rs:146-150,194-198,231-233,282-285,315,431,479,569-573,680-684,782,832-835`・`balloon_choice_settle_tests.rs:201-203`・`balloon_pressed_tests.rs:96-98`・`balloon_wiring_tests.rs:78-80`。(b) 空の runtime: `hover_flag:48-52`・`leave:108-110`・`pointer_handler:89,185,226,373,422,777`・`choice_settle:38`・`balloon_test_support.rs:123,160`。(c) `bubble_move`（`hover_flag:32`・`pointer_handler:34`）、`left_press`（`choice_settle:142`）と `bubble_left_press`（`pointer_handler:273`）｜`balloon_test_support.rs` に `empty_runtime`・`bubble_move`・`left_press`・`insert_wiring` を 1 つずつ置く。
26. **`row_with_refs`** — `balloon_pure_core_tests.rs:355-377`｜`row` と違うのは references だけ｜`ChoiceHitRow { references, ..row(..) }` にする。
27. **同じ「バルーンでない窓の離脱」の組み立て 2 本** — `balloon_hover_flag_tests.rs:257-280`・`balloon_leave_tests.rs:131-155`｜1 本にまとめ、両方の持ち物を表明する。
28. **メニューのテストの支えの写し** — `input_events_menu_tests.rs:23-32,35-49,58-63`｜`pointer_test_support.rs:51-62,26-40,43-48` と同じ名で引数の形が違う｜`use super::pointer_test_support::*` にして 3 つを消す。
29. **結線済みの Ctrl+左のテストの重なり** — `input_events_tests.rs:368-414` と `input_events_menu_tests.rs:168-203`｜違うのはスコープ 0 と 1 だけ｜面の座標まで見るメニュー側の 1 本に残す。［件 3］
30. **Ctrl なしのテストが新しいことを見ていない** — `input_events_tests.rs:416-442`｜Ctrl なしなので退避は条件の時点で起きず、送出は 278 の `handler_double_click_left_and_right_send` と同じ｜消す。ただし本体の `app_exit_escape_tests.rs:73` がこのテストを名で指す（本体は触れない）。［件 3・議題 4］
31. **範囲の外の持ち物を試すテスト（1）** — `balloon_hover_flag_tests.rs:62-68,306-324`｜`areka-runtime` の `BalloonHover` だけを見て、入力のコードを通らない（`areka-runtime` 側に `BalloonHover` のテストは 0 本）｜`crates/areka-runtime/src/balloon_hover.rs` の隣へ移す。C24 の約束 ⑶（要件 7.4）で今は動かせない。［議題 3］
32. **範囲の外の持ち物を試すテスト（2）** — `spawn_assembly_tests.rs` の 14 本のうち 13 本（30-163・222-586）｜`areka_placement::spawn_ghost_windows` だけを見る。入力の crate に要るのは 175 行の 1 本（ハンドラの装着）だけ｜13 本を `areka-placement/src/` へ移す。C24 の約束で今は動かせない。［議題 3］
33. **範囲の外の持ち物を試すテスト（3）** — `shell_box_tests.rs:471-509`・`shell_box.rs:18-19`｜`settle_box_hover`・`ShellBoxHover` は `areka-runtime` の持ち物で、`#[cfg(test)] pub(crate) use` の再輸出はこのテストのためだけ｜`areka-runtime/src/shell_box_hover.rs` の隣へ移し再輸出の 2 行を消す。C24 の約束で今は動かせない。［議題 3］

**D. 待ちの上限と後始末（テストの支え）**

34. **書き手のスレッドの join が無期限で panic を捨てる** — `drag_test_support.rs:281-284`（`close()`）・`258-262`（`saved()` の `barrier()`）｜`let _ = self.parts.handle.join()` は `ActorError::Panicked` を捨てる（アクターが panic してもテストは緑）。`barrier()` にも期限が無く、書き手が詰まれば永久に待つ｜`areka_runtime::spine::join_bounded("sylphya", self.parts.handle).expect(..)` に替え、`barrier` は期限つきの形にする（下の 6 節 (7)）。［件 11］

**E. 古い説明（事実と違う）**

35. **`lib.rs:508`「The Hand／collisionex は実装しない」** — `collisionex` は `collisionex-regions`（完了）で当たり判定の解決に入った。「The Hand」はツリーのどこにも無い｜行を「右クリックメニューは本ハンドラでは扱わない（引き金は右ボタンの解放）」だけにする。［件 1］
36. **本体の中にあった頃の説明** — `lib.rs:1-4`（「UI→kanade のマウス入力配信配線」・`[`throttle`]` への私有リンク）・`lib.rs:41-49,360-372,404-411`（`input_events→placement`・「本モジュール」＝本体の中のモジュールを指す書き方）・`shell_box.rs:4`・`shell_box_handler.rs:3`（`mod.rs` のハンドラ＝今は `lib.rs`）・`balloon.rs:351-353,357`（`donor …（crates/areka-input/src/lib.rs）`・`main.rs`・`spawn.rs 不改変`・`上流 input-events 成果`）｜入力の crate の今の事実（crate の根の受け手・依存は `areka_input → areka_placement`）で書き直す。［件 2］
37. **下位の crate が本体の呼び手を道つきで名指す** — `lib.rs:42,44,346,376,410,494,499`・`balloon.rs:10-13,186-187,379-382`・`balloon_exit.rs:52`・`balloon_wheel.rs:29`・`char_watch.rs:40`・`choice_drain.rs:38,151-153,168`・`shell_box_handler.rs:421`・`user_break.rs:9,204-205,224`・`hooks.rs:3`｜`ghost_session::register_systems`・`ghost_session::boot_ghost`・`ghost_session::prepare_ghost_windows`・`ghost_session_runtime::rewire_runtime_consumers`・`app_exit::quit_app`・`app_exit::install_input_hooks`・`menu::trigger`・`menu::request_close` は、同じウェーブの `session-module-gather` が本体のファイルを `session/` へ移すと古びる｜「本体の登録の口から 1 回」「ゴーストの起動ごと」のように役目で書く（名で指すなら関数名だけ・モジュールの道を書かない）。［件 9 と同じ扱い・仮定 6］
38. **`user_break.rs:201`・`220` のパス** — `crates/areka/src/readme.rs`・`crates/wintf/src/ecs/world/mod.rs`｜パスを書かずに関数の名（`fn wire_readme`・`EcsWorld::new`）で指す。［件 9］
39. **テストの説明が指す先が無い** — `input_events_tests.rs:362`「強制退避は下の 2 本（結線前・Ctrl+Shift）が持つ。」｜その 2 本は本体の `app_exit_escape_tests.rs`（`handler_ctrl_shift_left_double_click_…`・`escape_works_without_mouse_wiring` ほか 4 本）へ移った｜「強制退避は本体の強制退避のテストが見る」とパスを書かずに指す（仮定 6）。［件 4］
40. **`spawn_assembly_tests.rs:167,168,170,171`** — `input_events→placement`・`input_events の檻`、同じ crate の中を `areka_input::attach_char_pointer_handlers` と外の道で書く｜`areka_input`（この crate）・`crate::` に直す。［件 5］
41. **`collision-probe` の理由の文が誤り** — `crates/areka-runtime/examples/collision-probe.rs:63`・`collision-probe/setup.rs:75-76`｜「`pub(crate)` で内部が `crate::` パスを使うため」は誤り。`attach_char_pointer_handlers` は `pub`。使えない本当の理由は依存の向き（`areka-input` が `areka-runtime` に依存するので、`areka-runtime` の example は `areka-input` を使えない。`areka-runtime/Cargo.toml` に `areka-input` は無い）｜理由を依存の向きに直す。［件 6・範囲の外だが説明の文に限って触ってよい］
42. **`crash_report_tests.rs:7` の言い過ぎ** — 「（とプロセスの寿命の MTA）」｜`installed()` は vectored の受け手の据え付けしか映さない（MTA の成否は映らない）｜括弧を消す。
43. **バルーンのテストの古い説明** — `balloon_pointer_handler_tests.rs:9-14,263-267`（「本層では実演できず」は指摘 18 と同じ理由で誤り）・`304-305`（「中断の入口にも届かない」を確かめていない）・`509-526`（重複でない理由の長い説明）・`balloon_pass_through_tests.rs:8-16`（`actor.rs:389-393` などの他 crate の行番号）・`balloon_hover_flag_tests.rs:11`（「下 2 本」はファイルの最後の 2 本でない）｜消す。
44. **`cursor/README.md:69` のパス** — 「道具は自分の在るリポジトリの `crates\areka\cursor\` だけを読み書きします」｜道具（`tools/cursor-art.ps1:48`）が読み書きするのは `crates/areka-input/cursor`｜`crates\areka-input\cursor\` に直す。
45. **`cargo doc -p areka-input --no-deps` の警告 19 件** — 私有へのリンク 18（`lib.rs:4,335,381,382,449`・`balloon.rs:58`×2・`123,129,130,131`・`choice_drain.rs:3,10,156`・`shell_box_handler.rs:420`・`user_break.rs:6`×2・`235`）と解けないリンク 1（`lib.rs:109` の `MouseWiring::with_clock`＝`cfg(test)` の私有の関数）｜リンクを名だけ（バッククォート）にするか doc ごと短くする（指摘 36・47 と同じ所が多い）。［件 7］

**F. コメントの決まり（CLAUDE.md「## Comments」）**

46. **変わった経緯の語** — 本番: `lib.rs:204`（従前の配信値）・`545`（今までどおり）・`balloon.rs:64-66`（選択肢だけだった頃のまま）・`80`（今までの名前のまま）・`balloon_pressed.rs:157`・`char_watch.rs:258,270`（今までどおり）・`drag.rs:50`（今までどおり）・`choice_drain.rs:517`。テスト: `balloon_test_support.rs:172-173`（移行前と 1 バイト変わらない）・`balloon_pressed_tests.rs:204,221`・`balloon_pure_core_tests.rs:516,532,551`・`drag_tests.rs:109,346`・`drag_test_support.rs:7,138`・`click_tests.rs:141`（とテスト名 `keeps_its_return_value`）・`pointer_test_support.rs:24-25`・`input_events_tests.rs:24,621-622,644,657`・`input_events_menu_tests.rs:122-125`（とテスト名 `…_old_immediate_path`）・`shell_box_handler_tests.rs:306,557,673,689`・`shell_box_tests.rs:386`・`spawn_assembly_tests.rs:382`・`user_break_talk_id_tests.rs:6`｜今の振る舞いだけを書く。ログの文（`char_watch.rs:270` のメッセージ）は記録の欄でなく文なので直してよい。
47. **設計の札・要件の番号・作業の語** — 本番: `drag.rs:12,57`（設計 D1・D2）・`cursor_pick.rs:81`（13 行の表）・`hover_region.rs:47`（7 行の表）・`lib.rs:421`（M1）・`lib.rs:506,556`（「α 後」＝版の段に使わなくなった語。`OnMouseDoubleClickEx` を送らない事実だけを書く）・「donor」「檻」「②」の語（`lib.rs:91,95,106,124`・`balloon.rs:211,396`・`balloon_exit.rs:40,149,166`・`balloon_moved.rs:27,36,39,95,139,156`・`balloon_pressed.rs:33,165`・`choice_drain.rs:10,33,87,124,127,148`）。テスト: `choice_drain.rs:258,267,281,285,305,310,352,379,384,443,453,458`（Req1.x・design ログ語彙表）・`balloon_hover_flag_tests.rs:91,117,215,234,253`・`balloon_leave_tests.rs:62,67,123,178`・`balloon_pointer_handler_tests.rs:41,57,163,168,287,342,444,492,595,621,627,640,707,734,740,753,809`・`balloon_pure_core_tests.rs:132,227,252,282,294,299,326,331,342,417,430,445`・`balloon_wiring_tests.rs:96,106,110,114,139,141,157,168,181,216,228,235,239,249,256,284,288,293,308`・`balloon_test_support.rs:30,61,66,71,106`・`balloon_choice_settle_tests.rs:27,121-122`・`drag_tests.rs:2,8,55,83,108,146,166,209,226,260,296,307,346,389,435,455,473,492,516`（A1〜A9）・`drag_sticky_tests.rs:4`（2.1）・`cursor_pick_tests.rs:56,100,157,209`（表の 13 行・要件 4.4 ほか）と `row01_`〜`row13_`・`hover_region_tests.rs:33,47,55,64,73,82,97,112,121`（表 N 行目）・`input_events_tests.rs:187,228,231,400,411,416`（配線挿入檻・暫定退避檻・要件 5.3・メモへのリンク）・`spawn_assembly_tests.rs:141,381,419,454,458,473,477,481,485,491,583` と `t_i1_`〜`t_i3_`・`user_break_tests.rs:87,109,361,387`（判断分岐 ⑷⑸＝`judge_press` の番号と数え違い）｜札と番号を消して文だけにする。テスト名の札（`row01_`・`t_i1_`）は外せれば外す。
48. **本体の別のモジュールを手元のように書く（テスト）** — `input_events_tests.rs:361`（`menu::request_close`）・`input_events_menu_tests.rs:165-166`（`menu::mod_wiring_tests`・`menu::trigger_wired_tests`＝正しくは `menu::trigger::trigger_wired_tests`）・`user_break_tests.rs:713`（`ghost_session::register_systems`）・`shell_box_handler_tests.rs:6,172,184`（`mod.rs`）・`balloon_wiring_tests.rs:288,293`（`ghost_session::register_systems`）｜役目で書く（指摘 37 と同じ扱い）。
49. **コードを言い直すだけの説明** — 本番の長い doc（`balloon_moved.rs:16-49`・`balloon_exit.rs:18-53`・`balloon_pressed.rs:20-59`・`balloon.rs:190-245,247-298,300-346`・`throttle.rs:26-47`・`choice_drain.rs:42-51,76-87` ほか）は手順を言い直している。`user_break_tests.rs:5-16` は下のテストの一覧の言い直し（18〜21 の GPU の理由は残す）｜`/comment-cleanup` の作法で、理由（÷k をしない理由・借用の順の理由・ログの非対称の理由）だけを残して削る。`pub` の項目の 1 行目と `//!` の 1 行目は残す。
50. **`cursor/README.md:50` の要件の番号** — 「（要件 6.2）」｜文書の中の仕様の番号は古びる（コードのコメントの決まりの外だが同じ理由）｜消すか、仕様の名だけにする（114 行は既に名で指している）。

**G. テストのファイルの形**

51. **1 本だけのテストのファイル** — `user_break_talk_id_tests.rs`（37 行）・`user_break.rs:406-408` の `#[path]`｜`user_break_tests.rs` の末尾へ移しても約 880 行で 1,000 行に届かない（指摘 10 を足すなら行数を確かめる）｜移してファイルと `#[path]` を消す。
52. **テストのファイル名が本体の中のモジュール名のまま** — `input_events_tests.rs`・`input_events_menu_tests.rs`・`input_events_char_clamp_tests.rs`（`lib.rs:592,599,603` の `#[path]`）｜`input_events` は本体の中にあった頃のモジュール名。名を変えると `collision-probe.rs:66`（「`input_events_tests.rs` が確かめる」）が古びる｜変えるなら同じ変更で `collision-probe.rs:66` も直す（要件 5.6 と同じ doc の塊）。［議題 9］

### できれば（25 件）

53. `click_tests.rs:246-252` の `scaled` と `drag_test_support.rs:52-58` の `fake_hit` は同じ写像｜`pointer_test_support` へ 1 つ置く。
54. `drag_tests.rs:48` の `offset` と `drag_sticky_tests.rs:24,31` の `add`・`sub`、`drag_tests.rs:215` の `begin_drag` と `drag_sticky_tests.rs:55` の `begin`｜`drag_test_support` へ寄せる。
55. `KanadeMsg::Mouse` だけを取り出す drain が 3 つ（`char_watch_tests.rs:106-114`・`click_tests.rs:297-304`・`drag_test_support.rs:271-279`）｜`pointer_test_support` に 1 つ置く。
56. `pointer_test_support.rs:8-20` の `bubble_pointer` は `bubble_pointer_mods(.., false)` と同じ｜1 行で呼ぶ（-6 行。本体のテストが使う名と形は変えない）。
57. `drag_sticky_tests.rs:285,370-377` の `1200` は `drag_test_support.rs:40` の `WORK_AREA` の手写し｜`pub(super)` にして使う。
58. `click_tests.rs:128-139` `note_press_before_wiring_is_a_no_op` は落ちようが無い（挿す者が居ない）｜消す（474〜477 が解放側の結線前を見る）。
59. `cursor_pick_tests.rs:344-351` の `pick` は `pick_shape` を呼ぶだけの包み｜直に呼ぶ。
60. `click.rs:145-152`（窓の大きさが無いときは中とみなす）にテストが無い。`false` に壊しても緑で、そうなると大きさの決まる前の解放が全部捨てられる｜大きさ無しの窓で押して放し、ButtonUp が 1 件届くテストを足す。
61. `cursor_answer.rs:168-172` の 2 段目（ブレスのファイルが作れないとき descript の差し替え）を区別するテストが無い（`cursor_answer_tests.rs:244-256` は 3 段目と同じ答え）｜作れるファイルを `assets.ghost.finger` に置き、そのカーソルが出ることを見る。
62. `balloon_wheel.rs:108-112`（`WHEEL_PAGESCROLL` → `Page`）にテストが無い。壊れると OS を「1 画面ずつ」にした利用者で端まで飛ぶ｜写しを `fn wheel_lines_from(u32) -> WheelLines` に分け 1 本で確かめる。
63. 結線漏れ・送出失敗の記録の枝にテストが無い: `shell_box_handler.rs:135-144`（`shell_box_hover_missing`）・`327-333`（`balloon_wiring_missing`）・`user_break.rs:373-379`（`box_break_no_wiring`）・`lib.rs:234-242,294-301`（`mouse_send_failed`）・借用に失敗する枝（`balloon_wheel.rs:72-78`・`balloon_pressed.rs:220-227`・`balloon_exit.rs:131-140`）・離脱のスコープの重複除け（`balloon_exit.rs:66-75`）｜単一スレッドで実害は低い。指摘 16 で送出を 1 つに寄せたら、その 1 つに 1 本足す。
64. `click_tests.rs:30,258`・`input_events_char_clamp_tests.rs:25` が `MouseWiring::new`（本物の時計）を使う。今は時計を読む道を通らないので結果は変わらない｜`world_with_wiring` に替える。
65. `user_break_tests.rs:268-273` の `named` と `shell_box_handler_tests.rs:564-572` の `lines_of` は同じ中身｜1 つにする。
66. `shell_box_tests.rs:35-45,72-81,316-321`・`shell_box_handler_tests.rs:59-69`・`balloon_test_support.rs:31,48` の `name`・`row`・`anchor_row` が近い写し｜`balloon::test_support::row` に `..` で references を足して使う。
67. `input_events_tests.rs:154-185`（送出の手助けの右版）は 278 のハンドラ経由と `input_events_menu_tests.rs:127` の預かり経由で 2 度見られている｜消す。
68. `lib.rs:97-98` の `#[allow(dead_code)]`（`RegionSource::Mock`）は効いていない（`pub` の列挙の `pub` の腕は dead_code にならない。本体のテストも使う）。先送りの印でもない（理由は「テスト専用」）｜2 行を消す。［議題 10］
69. `drag.rs:98-107` の `kind_name` の `match` は 6 腕のうち 2 腕しか来ない（`Move`・`DoubleClick`・`other` は死んだ腕）｜`notify_drag` に記録の名（`&'static str`）を渡す。
70. `throttle.rs:56-96` の 3 つの分岐が同じ構造体を組む｜`MouseMoveThrottle { last_pos: Some(pos), ..state.clone() }` の形で短くする（-15 行）。
71. `user_break.rs:270` の早い戻りで `judge_press` の `NotDoubleClick` の腕（286）は呼び手からは来ない｜早い戻りを残すなら腕は `unreachable` でなく今のまま（`judge_press` は単体で試されている）。直すなら戻りを外して `balloon_break_detected` の trace を判定の後に置く（記録の順が変わる）。
72. `choice_drain.rs:88` の `forward_all` の戻り値（件数）は本番で使われない（「檻の観測点」）｜テストは受け手の `try_recv` で数えられるので戻り値を外せる（-数行）。
73. `char_watch.rs:157-214` の上に居る窓の巡回で、窓ごとに `get_non_send::<MouseWiring>()` を引き直している（158〜164）｜巡回の前に `hover` を写しておけば 1 回で済む（窓は 2 枚なので速さの差は無い・読みやすさだけ）。
74. `cursor_pick.rs:176` が当たり判定の名を引くたびに `String` を複製する（`WM_SETCURSOR` ごと）｜鍵の形が `(u32, String, CursorScene)` なので今のままでよいが、気になるなら `rows` の側を借用の鍵で引ける形にする（`areka-runtime` の型なので今回は触れない）。
75. `balloon_wheel.rs:22-23` の `WHEEL_PAGESCROLL` の自前の定義（「windows クレートに無い」）｜`windows` 0.62 に定数が在るかは確かめていない（Research Needed）。在れば使う。
76. `hover_region.rs:39-45` の `send` は構造体を組むだけの包み（呼び手 6 か所）｜`HoverSend { edge, surface, region }` を直に書く（-7 行）。なお `Cargo.toml` の依存は全部使われている（本番 12・開発 5 と test-support つきの 2 つ）ので、依存の指摘は 0。
77. `balloon_pointer_handler_tests.rs`（874 行）は 1,000 行の番人に近い｜指摘 20 で約 130 行減る。足すテストは新しいファイルへ（要件 6.1）。

**Verdict**: 本番は健全。まず指摘 1・2・5・9・10（落ちる力の無いテストと、壊れても気づかない本番の枝）と 14（記録の二重）を直す。
**Lean**: 消せる行の見込み 約 −700 行（`balloon_pass_through_tests.rs` 167・重なるテスト約 300・写しの寄せ約 150・長い説明の削り約 100）・依存の減り 0。
**Not checked**: `cursor/` の絵の中身（仮定 3）。テストは走らせていない（読んだ内容からの判断）。`windows` 0.62 の `WHEEL_PAGESCROLL` の有無（指摘 75）。

## 5. 預かった件 1〜11 の照合（要件 5・要件 1.5）

| 件 | 今も在るか・場所 | 重なる指摘 |
|---|---|---|
| 1 | 在る。`lib.rs:508` | 35 |
| 2 | 在る。`lib.rs:1-4,41-49,360-372,404-411`・`shell_box.rs:4`・`shell_box_handler.rs:3`・`balloon.rs:351-357` | 36（37 も同じ種類） |
| 3 | 在る。3 本とも説明が事実でない（`input_events_tests.rs:364-366,416-417`・`input_events_menu_tests.rs:161-162`）。表明 `ghost_count` は落ちない（`377-383,408-413,434,697`・`198`） | 2・29・30 |
| 4 | 在る。`input_events_tests.rs:362` | 39 |
| 5 | 在る。行は 167〜173 へ動いた | 40 |
| 6 | 在る。`collision-probe.rs:63`・`setup.rs:75-76`。直す先の理由は「依存の向き」 | 41 |
| 7 | 在る。**19 件**（brief の 20 件から 1 件減った。私有へのリンク 18・解けないリンク 1） | 45 |
| 8 | 狭められる項目は **0**（6 節 (4)）。記録に「全部に使い手あり」と書いて閉じる | — |
| 9 | 在る。`user_break.rs:201`（同じ形の `220` も） | 37・38 |
| 10 | 在る。`choice_drain.rs:253`（説明の塊 240〜251 も）と `balloon_test_support.rs:185` は 1 字も違わない | 24 |
| 11 | 在る。`drag_test_support.rs:283`。型は合う（6 節 (7)） | 34 |

## 6. OPEN QUESTION の事実

**(1) 落ちの報告の分担**
- `crash_report_tests.rs` が使うのは 2 つだけ: 5 行 `areka_runtime::crash_report_test_support::install`（`pub extern "C" fn`・`.CRT$XCU` の static に置く）と 10 行 `installed()`。門は `areka-runtime/src/lib.rs:69-70` の `#[cfg(any(test, feature = "test-support"))]`。`areka-input` は dev-dependency で `features = ["test-support"]` を開けている（`Cargo.toml:45`）。
- `test-crash-report-merge` の brief の計画: 写し 3 本（`wintf/src/test_crash_report.rs`・`areka-placement/src/test_crash_report.rs`・`areka-runtime/src/crash_report_test_support.rs`）を 1 本にまとめ、置き場は `log-capture-kit` の新しいファイル（任意の feature の後ろに `windows`・推す案）か新しい leaf crate。許可の持ち手の表は `areka-runtime` にだけ残す（差し込む口か薄く残すかは設計で決める）。使い手の一覧（13・72 行）に `areka-input` が無く、76 行は「`audit-areka-input-menu` とは 0（`areka-input` は落ちの報告を使わない）」と書く＝**誤り**。
- 食い違いの帰結: C24 の約束 ⑷ で `test-crash-report-merge` は `crates/areka-input/` に触らない。向こうが `areka_runtime::crash_report_test_support::{install, installed}` の名を消すと、向こうの全体テストで `areka-input` のテストが組み上がらず赤になる。**向こうが 2 つの名を `areka-runtime` に薄く残せば、どちらの spec も相手のファイルに触らずに済む**（推す案）。残せないなら、後に着地する側が `crash_report_tests.rs` の 2 行を付け替える（`log-capture-kit` に移るなら、こちらは dev-dependency の `log-capture-kit` に feature を足すことになる＝`Cargo.toml` の 1 行・`Cargo.lock` は変わらない見込み）。
- 向こうの brief の 13・72・76 行の訂正は範囲の外（要件 7.11 の申し送りの的）。

**(2) メニュー（参考）**
- 入力の crate の中でメニューの側から呼ばれる口は `MouseWiring::take_pending_right_double_click`・`send_pending_right_double_click`・`send_close_request`・`char_scope`・`click::on_char_pointer_released`・`attach_char_pointer_handlers`・`PendingDoubleClick` の欄。見直しの指摘はこれらの名と形を変えない（変えるのは説明の文だけ）。

**(4) 公開の項目の使い手（狭められるか）**
- 開けた項目は全部、本体（`crates/areka/`）の本番かテストに使い手がある。`real-run-kit` と examples の使い手は 0（`areka-placement`・`areka-runtime` の 6 ファイルの `areka_input::…` は doc の中の字面だけ）。
- **本体の本番が使う**: `MouseWiring`（`app_exit.rs`・`menu/mod.rs`・`menu/trigger.rs`）・`PendingDoubleClick`・`char_scope`・`take_pending_right_double_click`・`send_pending_right_double_click`・`send_close_request`・`wire_mouse_input`・`attach_char_pointer_handlers`・`hooks::InputHooks`・`balloon::{attach_balloon_pointer_handlers, wire_balloon_choice, register_balloon_leave_system}`・`balloon_wheel::register_balloon_wheel`・`char_watch::register_char_watch`・`choice_drain::{wire_choice_drain, register_choice_drain}`・`click::on_char_pointer_released`・`shell_box_handler::register_box_leave_system`・`user_break::{wire_user_break, register_user_break_drain}`。
- **本体のテストだけが使う**（仮定 4 で使い手に数える）: `MouseWiring::new`（7 ファイル）・`RegionSource`（`Mock`）・`defer_right_double_click`・`on_char_pointer_moved`（`app_exit_escape_tests.rs`）・`on_char_pointer_pressed`（3 ファイル）・`balloon::{BalloonWiring, BalloonWiring::new, ChoiceSelection}`（`runtime_tests/` 5 ファイル）・`shell_box::ShellBoxHover`（2 ファイル）・`char_watch::char_watch_with`・`char_watch::cursor_answer::register_char_cursor`（`switch_cursor_tests.rs`）・`user_break::{UserBreakWiring, UserBreakWiring::new, drain_no_user_break_signals, on_left_press}`・`flag_rx`（2 ファイル）・`flag_source_connected`（3 ファイル）・`pointer_test_support`（`app_exit_escape_tests.rs` が `*` で読む）。
- `MouseWiring::with_clock` は `pub` でない（`cfg(any(test, feature = "test-support"))` の私有の関数）。外からは `pointer_test_support::world_with_wiring` 経由でだけ届く。
- ログの出所（モジュールの道）にも読み手がある: `areka_input::user_break`（`emo2_mcp_real_run.rs:114` の `RUST_LOG`・`log_history_tests.rs:150`）・`areka_input::char_watch::cursor_answer`（`log_history_tests.rs:144`）・`areka_input::throttle`・`areka_input::hover_region::cursor_pick`（`doc/ukadoc-coverage/ledger/assets.toml` の説明。`cursor_answer` もここに再び出る）。記録の名の読み手: `balloon_wiring_missing`（本体 `ghost_session.rs`）・`anchor_selected`（`doc/anchor-compat.md`・台帳）・`mouse_click_not_sent`（kanade と台帳）。`choice_selection_send_failed`・`anchor_selection_send_failed`・`mouse_send_failed`・`choice_hover_inject`・`box_choice_hover_inject` の範囲の外の読み手は 0。

**(7) `drag_test_support.rs:283` と `join_bounded`**
- `self.parts.handle` の型は `areka_actor::ActorHandle`（中は `JoinHandle<()>` と名前）。作るのは `spawn_sylphya`（`drag_test_support.rs:149`）。
- `areka_runtime::spine::join_bounded(what: &str, handle: ActorHandle) -> Result<(), ActorError>`（`spine_wait.rs:428`・`pub use self::wait::join_bounded`）。`spine` は `areka-runtime/src/lib.rs:64-65` の `#[cfg(any(test, feature = "test-support"))]` の中で、`areka-input` の dev-dependency（test-support つき）から届く。30 秒（`SPIN_WAIT`）待って終わらなければ panic し、その後 `handle.join()` を返す。
- **型は合う**。そのまま渡せる（`areka-input` は `areka_actor` を名指さなくてよい・依存は増えない）。`barrier()` の期限つきの形（下請けの報告では `barrier_within(SPIN_WAIT)`）が `areka-sylphya` の公開の面に在るかは設計で確かめる。

## 7. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今のコードの資産 | ギャップ |
|---|---|---|
| 1 洗い出し | この報告 | 無し（Not checked は絵の中身と `WHEEL_PAGESCROLL` の有無だけ） |
| 2 確定 | 4 節の 77 件・9 節の議題 | Constraint: 議題 3・4（C24 の約束で範囲の外へ動かせない直し） |
| 3 直し切る | 指摘ごとの直しの案 | Missing: 写しの寄せ先（指摘 14・15・25・28・53〜57）の関数の名と置き場は設計で決める |
| 4 テストで縛る | 決定論の支え（注入の時計・偽の窓・`log-capture-kit`・`GpuPermit`） | Missing: 指摘 1・5・8・9・10・14・60〜63 の新しいテスト。Unknown: 指摘 10 の番号を置く口（`areka-runtime` の test-support に在るか）・指摘 1・5 の GPU の組み立てで拡大率 2.0／折り返しを作れるか |
| 5 預かった件 | 5 節 | 件 8 は 0 件で閉じる。件 6 は範囲の外の説明の文だけ |
| 6 決まり | 1,000 行の番人（`balloon_pointer_handler_tests.rs` 874） | Constraint: 足すテストは新しいファイルへ。依存は足さない（指摘 34 も依存は増えない） |
| 7 範囲の外 | 6 節 (1)(4) | Constraint: 本体のファイル・`areka-runtime`・`areka-placement`・`Cargo.lock`・`log-capture-kit` は触らない。ログの記録の名と出所を変えない（議題 5） |
| 8 副作用 | 実機の確かめは要らない（どの直しも決定論のテストで確かめられる。GPU のテストは既存の `GpuPermit` の組み立て） | 無し |
| 9 仕上げ | — | かけ直しの作法は要件どおり |

## 8. 直し方の案

- **案 A（今あるファイルの中で直す）**: 本番の写しは親のモジュール（`balloon.rs`・`lib.rs`）へ寄せ、テストの写しは既にある支え（`pointer_test_support.rs`・`balloon_test_support.rs`・`drag_test_support.rs`）へ寄せる。新しいファイルはテストを足して 1,000 行に近づくときだけ。✅ いちばん小さい差分・構造を変えない ❌ 支えのファイルが少し太る。
- **案 B（共通のテストの支えを 1 つ新しく作る）**: crate 全体の `test_support.rs` を作り、`capture_logs`・`empty_runtime`・drain・`world_with_wiring` を集める。✅ 置き場が 1 つ ❌ `pointer_test_support` は本体のテストが `*` で読む公開の面なので、そこから動かすと本体の呼び手が変わる（要件 7.3 に触れる）。新しいモジュールの道が増える。
- **案 C（A を基本に、範囲の外の持ち物のテストだけ後へ）**: 案 A で直し、指摘 31〜33 は今回は動かさず裁定で残すか、後の spec へ回す。
- **見立て**: 規模は M（3〜7 日）。タスクの見込みは 14〜18（組: 本番の写しの寄せ 3・落ちるテストの立て直し 4・テストの写しと削除 4・古い説明とコメント 3・`cargo doc` と預かりの記録 1・かけ直しの検証 1〜2）。20 を越える見込みは低い（仮定 5 のまま 1 本）。危険は低〜中（本番の写しの寄せで記録の名と出所を変えない注意が要る・GPU のテストを 2 本足す）。
- **推す案**: 案 C（＝案 A＋範囲の外の持ち物は議題 3 で決める）。

## 9. 設計と要件の討議への持ち越し

### 議題（答えで作業が変わるもの）

1. **落ちの報告の分担（OPEN QUESTION 1）** — 推す答え: `test-crash-report-merge` が `areka_runtime::crash_report_test_support::{install, installed}` の 2 つの名を `areka-runtime` に薄く残す（両 spec とも相手のファイルに触らない）。要件の討議の前に向こうのセッションへ、brief の 13・72・76 行の誤りと一緒に知らせる。残せないと向こうが決めたら、後に着地する側が `crash_report_tests.rs` の 2 行を付け替える（仮定 1 のまま）。
2. **メニュー（OPEN QUESTION 2）** — 推す答え: 仮定 2 のまま（本体の `menu/` は見直さない・入力の crate の中のメニュー向けの口は範囲に入れ、名と形は変えない）。この audit でメニュー向けの口に出た指摘は説明の文（指摘 36・37・48）だけ。
3. **範囲の外の持ち物を試すテスト（指摘 31・32・33）** — C24 の約束 ⑶（要件 7.4）で `areka-runtime`・`areka-placement` に移せない。回す先の生きた spec も無い（`audit-placement-core`・`audit-placement-follow` ほかは完了）。推す答え: 今回は動かさず「残す」と裁定し（要件 2.5）、説明の文だけ直す。移すのは C24 の後に `/kiro-discovery` で小さな件として起票する。
4. **本体のコメントが名で指すテストを消す（指摘 30）** — 本体の `app_exit_escape_tests.rs:73` が `handler_left_double_click_without_ctrl_does_not_despawn_and_sends` を名で指す。推す答え: 消して、`app_exit_escape_tests.rs` の持ち主の `audit-areka-app` の brief へ「73 行の名指しを直す」を申し送る（要件 7.9〜7.11）。消さないなら説明だけ直す。
5. **ログの記録の名と出所を写しの寄せで変えない（指摘 14・15・16）** — 寄せると記録の出所（`areka_input::balloon::pressed` → `areka_input::balloon` など）が変わる。推す答え: 記録の名と欄は変えない。出所は、範囲の外に読み手のある 4 つ（`areka_input::user_break`・`areka_input::char_watch::cursor_answer`・`areka_input::hover_region::cursor_pick`・`areka_input::throttle`）を動かさない限り、外への約束に触れない（読み手の一覧は 6 節 (4)）。設計で寄せ先ごとに出所の変化を表にする。
6. **送り失敗の二重の記録のどちらを残すか（指摘 14）** — 推す答え: 呼び手の ERROR を 1 行残し、`send_selection` は `bool` を返すだけにする（範囲の外の読み手は 0）。ログの行が 1 行減るのは外への約束（ログの行の語）の変化に当たるので、要件 7.7 の裁定として記録する。
7. **件 8 の結果（狭める 0 件）** — 推す答え: 「開けた項目は全部、本体の本番かテストに使い手あり」を記録して閉じる（要件 5.8・仮定 4）。
8. **件 11 の直し方（指摘 34）** — 推す答え: `join_bounded` を直に呼ぶ（型は合う・依存は増えない）。`barrier` の期限つきの形が `areka-sylphya` の公開の面に無ければ、`join` だけ直して `barrier` は議題に戻す。
9. **テストのファイル名の `input_events_` を変えるか（指摘 52）** — 推す答え: 変える。同じ変更で `collision-probe.rs:66` の 1 行（件 6 と同じ doc の塊）も直す。範囲の外の文を件 6 より 1 行だけ広げることになるので、要件 7.4 の例外として裁定を記録する。広げたくなければ名は変えない。
10. **`RegionSource::Mock` の `#[allow(dead_code)]`（指摘 68）** — 要件 2.2 の 2 種類目（わざと置いた先送りの印）かの判断。推す答え: 当たらない（書かれた理由は「本番からは構築されないテスト専用の口」で、先送りでない・そもそも効いていない）ので消す。
11. **規模（仮定 5）** — 推す答え: 1 本のまま（見込み 14〜18 タスク）。

### Research Needed（設計で確かめる）

- 指摘 1・5: `choice_settle` の GPU の組み立てで拡大率 2.0 と選択肢の折り返しを作れるか。
- 指摘 10: `RuntimeWiring::shown_talk_id` に番号を置く口が `areka-runtime` の test-support に在るか（無ければ要件 7.4 に触れずに組めるか）。
- 指摘 34: `areka-sylphya` の publisher に期限つきの barrier が在るか。
- 指摘 75: `windows` 0.62 に `WHEEL_PAGESCROLL` が在るか。
- 指摘 15: 箱の強調の当て方（場所の鍵が違う）をバルーンの 2 つと同じ関数に寄せられるか。
