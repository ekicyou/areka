# ギャップ分析: audit-placement-follow（配置の追従と移り変わりの判定の見直し）

- 測った時点: 枝 `claude/kiro-start-audit-placement-follow-7ee3b1`（`d4243e0f`・`main` `30a343eb` の上に spec の初期化だけ）・2026-11-14。範囲の 56 ファイルは棚卸㊲の再測定（`df6f4e03`）から変わっていない（56 ファイル・コード 5,778 行・テスト 24,099 行・計 29,877 行を `wc -l` で数え直した）。
- 物差し: `/ponytail:ponytail-audit`（ponytail 5.3.3 の `SKILL.md` を読み、その作法で行った）。作法のうち 2 つを外した（要件 1.2・1.3）。
  - 「最大 20 件」を外した。指摘は全件を下に並べ、省いた件は無い。
  - 「読まなかった所（Not checked）」を残さない。範囲の 56 ファイルを全部読んだ（2 節）。
- 番号は audit の番号（組をまたいで通し番号）。組は audit の 3 つ（必ず直す・直したほうがよい・できれば）のまま。
- 申し送り ⑵ の確かめのため、`#[allow(dead_code)]`・`#[allow(unused_imports)]` の 36 個を一時的に消して `cargo check -p areka-placement --lib --tests --examples` を 1 回回し、確かめた直後に `git checkout` で元へ戻した（作業ツリーは clean）。そのほかにコードは変えていない。決めごとは要件の討議に回す。

## 1. 要約

- **何をする所か**: キャラ窓とバルーン窓の追従（ドラッグ・`\![move]`・面の `move` のずれ・組の連れ動き・作業領域への寄せ・見える範囲の守り・DPI が変わったときのオフセットの追随）と、移り変わりの記録（`[transition]` の行）を出す口（`transition_diag.rs`）。本体の `placement_tests/transition_*` は、その記録を読んで「1 つの遷移が 1 フレームで揃ったか」を決める、テスト専用の判定器とその檻。
- **想定した負荷**: 1 人の利用者の 1 台の机。窓は数枚、イベントはフレームごと・ドラッグごと。座標は物理 px。複数モニタと DPI の違うモニタは普通にある前提で見た。
- **指摘の数**: 必ず直す 4 件・直したほうがよい 14 件・できれば 10 件＝**28 件**（テストの指摘は種類ごとに 1 件へまとめ、場所は全部を挙げた）。
- **見つかった型**: ⑴ 同じ決まりを 2 か所で別に当てている（随伴の式の写しで守りが抜ける＝2・未確定の印の扱い＝20・ずれの扱い＝21）。⑵ 整合待ちの札と記録の順の穴（1）。⑶ 記録の語が事実と違う（4・6）。⑷ ドラッグの終わりの穴（3・7）。⑸ 判定器が壊れた行や測っていない窓を合格にする（12・13・27）。⑹ 壊れても落ちないテスト・同じテストの写し・部品の写し（11・14〜19）。⑺ crate を分けた後に古くなった `#[allow]` と説明、旧い名前 `emo2_boot`（9・22）。
- **申し送り**: ⑵ の 36 個の属性のうち 33 個は要らない（`cargo check` で確かめた）・3 個は要るが理由を書き直す。⑶ の公開の項目は、要らない公開を狭める直しが C20 の約束（要件 7.1）に当たる。⑴ の `MonitorSnapshot` は理由が今も事実で残す案を推す。旧い名前は 9 ファイル 21 か所（`Emo2Wiring` は 0 か所）。
- **大きさ**: 見込み 24〜27 タスク。20 を越えるので、組で 2 本に切る案を 9 節に出す。

## 2. 読んだ範囲（要件 1.3）

範囲の全 56 ファイルを全行読んだ。読むのは 9 人の下請け（読むだけ・cargo を回さない）に分け、進行役が指摘の行を開いて確かめた（確かめられなかった候補は落とした）。

| 受け持ち | ファイル（行） |
|---|---|
| 本番 甲 | `follow.rs`（283・進行役も全行）・`follow/anchor.rs`（195）・`keyword_base.rs`（185）・`move_shift.rs`（235）・`offset_space.rs`（269）・`sticky_follow.rs`（400）・`visibility.rs`（365）・`work_area.rs`（323）・`work_area_clamp.rs`（511） |
| 本番 乙 | `follow/drag_follow.rs`（978）・`follow/window_move.rs`（1,231）・`transition_diag.rs`（803） |
| テスト 丙 | `follow_anchor_tests.rs`（361）・`follow_balloon_drag_tests.rs`（366）・`follow_balloon_limit_tests.rs`（743）・`follow_default_pos_track_tests.rs`（345）・`follow_drag_end_gate_tests.rs`（451）・`follow_drag_end_limit_tests.rs`（673）・`follow_drag_end_persist_tests.rs`（712）・`follow_drag_tests.rs`（897） |
| テスト 丁 | `follow_keyword_base_tests.rs`（810）・`follow_move_shift_tests.rs`（396）・`follow_offset_component_tests.rs`（186）・`follow_offset_residual_tests.rs`（138）・`follow_offset_space_tests.rs`（882）・`follow_resize_tests.rs`（950）・`follow_sticky_follow_drag_tests.rs`（399）・`follow_sticky_follow_tests.rs`（143） |
| テスト 戊 | `follow_visibility_balloon_wiring_tests.rs`（962）・`follow_visibility_char_wiring_tests.rs`（899）・`follow_visibility_guard_tests.rs`（362）・`follow_transition_diag_tests.rs`（439）・`transition_diag_tests.rs`（794） |
| テスト 己 | `follow_test_support.rs`（316）・`follow_window_move_diag_tests.rs`（552）・`follow_window_move_hold_watch_tests.rs`（262）・`follow_window_move_tests.rs`（322）・`follow_work_area_clamp_chain_tests.rs`（419）・`follow_work_area_clamp_grow_tests.rs`（285）・`follow_work_area_clamp_restore_tests.rs`（228）・`follow_work_area_tests.rs`（448） |
| 判定器 庚 | `placement_tests/transition_judge.rs`（999）・`transition_judge_offset.rs`（765）・`transition_judge_verdict.rs`（871）・`transition_judge_test_support.rs`（346） |
| 判定器のテスト 辛 | `transition_judge_coalesced_follow_tests.rs`（584）・`transition_judge_frame_tests.rs`（321）・`transition_judge_negative_tests.rs`（273）・`transition_judge_offset_signoff_tests.rs`（508）・`transition_judge_offset_tests.rs`（836）・`transition_judge_origin_tests.rs`（253）・`transition_judge_reobservation_tests.rs`（496） |
| 判定器のテスト 壬 | `transition_judge_tests.rs`（1,039）・`transition_judge_verdict_tests.rs`（1,037）・`transition_signoff_procedure_tests.rs`（886）・`transition_signoff_tests.rs`（145） |

- 判定器が読む行の形は、今の 4 つの発行元（`wintf` の `ecs/window/transition_diag.rs`・`areka-placement` の `transition_diag.rs`・`areka-emo-present` の `presenter/transition_record.rs` と `show.rs`）と合っていた（12 の `kind` 語・各 `*_FIELDS` と判定器が引く表が一致・判定器は欄の名のリテラルを書かない）。
- 範囲の中に `shortcut:`・`ponytail:` の注釈は 0 件（grep）。`#[allow(dead_code)]` に実在の理由を付けた語彙の口に当たりうるのは `anchor_changed_system`（指摘 8）だけ。
- 範囲の中に毎回同じ結果にならないテストは見つからなかった（`Instant::now()` の値は比べていない・`SetWindowPosCommand` の列はスレッドごと・`HashMap` の並びに依らない・テストの一時フォルダは `temp-path-kit` で `<workspace>\target\test-roots\` の下）。

## 3. 指摘の一覧

各件の欄: **場所**／**何か**／**困る場面**／**いちばん小さい直し**／**範囲の外への波及**／**分類**（普通に直す・残す 3 種類のどれか＝要件 4.1 の 1 裁定に反する・2 わざと置いた先送りの印・3 SSP 互換・ukadoc 正典・C20 の約束で止まる）。

### 必ず直す

#### 1. `\![move]` の絶対の軸の当て直しが、整合待ちの札を見ずに書き、書けなかったときは記録だけ進む（`follow/move_shift.rs:87-102`・呼び手 `crates/areka/src/runtime/move_cue_place.rs:225`）

- **何か**: `rebase_shift_axes` は、`\![move]` が絶対の位置へ書いた軸の「当てた分」を 0 にして記録（`MoveShiftApplied`）を入れ直し、すぐ `apply_char_shift` で今のずれを当て直す。当て直しは `PlacementRoute::PatternMove` の書込で、これは整合待ちの札（`DpiSyncHold`）が覆うべき経路（`window_move.rs:544`）。毎フレームの相（`runtime/frame/move_shift.rs:70-71`）は `dpi_sync::defers_window_write` で札を見てから書くが、この口は見ない。
- **困る場面**: ⑴ 面の `move` のずれを持つ窓に DPI の移り変わりで札が付いている間に、台本が絶対の X の `\![move]` を出す。当て直しが `enqueue_window_set_pos`（`window_move.rs:684-699`）へ届き、`warn!` と `debug_assert!(false)` が鳴る＝試験と開発の作りでは panic、出荷の作りでは札の間に書く（札が止めるはずの書込）。⑵ 記録を書込の**前に**入れるので、`apply_char_shift` が `no_work_area`・`write_refused` で書けなかったとき、記録は「当てた分 0・同じずれ・同じ倍率」になり、`shift_is_current` が真を返す。毎フレームの相は以後そのずれを当て直さない（`apply_char_shift` の説明「記録は変えないので次のフレームで当て直す」の約束が破れる）。
- **いちばん小さい直し**: `rebase_shift_axes` の中で書く前に `defers_window_write(world, window, HoldSite::PatternMove)` を見る。札があるとき・書けなかったときは、記録を「まだ当てていない」と分かる形で残す（例: 記録に「当て直し待ち」の印を 1 つ足し、`shift_is_current` がそれを偽と読む）。毎フレームの相が札の明けた後に当てる。テスト＝札のある窓へ絶対の軸の `\![move]` 後の当て直しを呼んでも書かず、札を外した次のフレームで当たる／書けなかった後に `shift_is_current` が偽（直す前は panic・真）。
- **範囲の外への波及**: なし（`rebase_shift_axes` の署名は変えない・`defers_window_write` は核の公開の関数を呼ぶだけ・呼び手 `move_cue_place.rs` は変えない）。
- **分類**: 普通に直す。

#### 2. `move_window_with_route` が随伴の式を写して持ち、システム由来の経路でバルーンの見える範囲の守りが走らない（`follow/window_move.rs:64-97`）

- **何か**: キャラ窓を書いた後、随伴バルーンを `offset + ずれ` の自前の式で書く。同じ仕事をする `follow_balloon`（`drag_follow.rs`）を呼ばないので、`follow_balloon` の中の守り（`guard_balloon_position`・S3′）が走らない。呼び手 3 つのうち `chain_realign.rs:351`（`ChainRealign`）と `move_shift.rs:176`（`PatternMove`）はシステム由来の経路で（`diag.rs` の `is_system_reanchor` が真）、守りが要る側。同じ経路でも `resize_window_to` は `follow_balloon(.., Placement(route))` を通るので守りが走る＝同じ決まりを 2 か所で別に当てている。
- **困る場面**: DPI の移り変わりの後の連鎖の解き直しが、キャラ窓を左端のモニタの x=0 へ置く。バルーンの offset が −400・幅 350 なら、バルーンは画面の外に丸ごと出て、キャラ窓だけが見える（`BalloonLimit(true)` でないバルーン）。同じ幾何を `Resnap` で通すと引き戻される。このバルーンの守りを `move_window_with_route` で確かめるテストは 0（`follow_visibility_balloon_wiring_tests.rs` は `follow_balloon` を直に呼ぶ）。
- **いちばん小さい直し**: 77〜94 行を `follow_balloon(world, window, Point{x, y}, BalloonFollowTrigger::Placement(route))` の 1 呼び出しにする。`MoveCue` はシステム由来でないので、今と同じ書込になる（ずれの足し方は `follow_balloon` が同じ式で持つ）。テスト＝`ChainRealign`・`PatternMove` でバルーンが画面の外へ出る形が引き戻される（直す前は外へ出る）。あわせて `move_window_to` の説明（33〜41 行「唯一の位置ライター」「呼び手はどれも `MoveCue` を名乗るため route は引数で受けない」）は事実と違うので直す（指摘 22）。
- **範囲の外への波及**: 署名は変わらない。`chain_realign.rs`（`audit-placement-core` の持ち物）の呼び出しの結果（バルーンの位置）が、画面の外へ出る形でだけ変わる＝相手のセッションへ知らせる（10 節）。既にある核のテスト（`dpi_sync_tests.rs:332-340`）が赤にならないかは設計で確かめる（11 節）。
- **分類**: 普通に直す（SSP の互換に関わらない・areka の守りの決まりの揃え）。

#### 3. バルーンを単独でドラッグして離した瞬間の最後の動きが、記憶には入るが、今の `BalloonFollow` には入らない（`follow/drag_follow.rs:666-784`・`:571-598`）

- **何か**: 離したとき `on_balloon_drag_end` は最終の `WindowPos` から `offset = balloon − char` を出し直して保存する。今の `BalloonFollow.offset` は説明どおり変えない（「連続ドラッグ側 `on_balloon_drag` が所有」）。ところが wintf は離したフレームの最後の動きを `DragEvent` として配らない（`wintf/src/ecs/drag/accumulator.rs:84` で `current_dragging_entity` を先に消す・`on_char_drag_end` の説明 113〜117 行が同じ穴を書いている）。窓そのものは wndproc が動かし終えている。
- **困る場面**: バルーンを素早く払い、最後の動きと同じフレームで離す（最後の 30 px が配られない）。保存の値は正しく、今の `BalloonFollow.offset` は 30 px 古い。次にキャラ窓をドラッグする・`\![move]`・再スナップすると、バルーンが 30 px 戻る。起動し直すと保存の値で正しい所へ出る＝今の位置と次の起動の位置が食い違う。
- **いちばん小さい直し**: `on_balloon_drag` の持ち主のキャラ窓の `BalloonFollow` を出し直す塊（571〜598 行）を 1 つの関数に切り、`on_balloon_drag_end` で保存の直後（limit の補正より前＝DD6 の順を守る）にも呼ぶ。説明の「変異させない」を直す。テスト＝最後の `DragEvent` と違う最終の `WindowPos` で `DragEnd` を送り、`BalloonFollow.offset()` が保存した値と同じ（直す前は古い値）。
- **範囲の外への波及**: なし。`drag_follow.rs` は 978 行で上限に近い（関数に切るので行は増えても数行・8 節）。
- **分類**: 普通に直す。確かさは中（wintf のコードを読んでの見立て・実機では再現していない＝11 節）。

#### 4. バルーンの守りが、毎回 `OffscreenPull`（画面の外の窓を引き寄せた）を誤って記録する（`follow/visibility.rs:320-331`・呼び手 `drag_follow.rs:494-503`）

- **何か**: `evaluate_visibility_guard` は、射影の**入力**（`raw`）がどの作業領域とも交わらず最近傍で解決したとき `OffscreenPull` を出す。バルーンには射影の段が無いので、呼び手は `raw = proposed` を渡す。すると `OffscreenPull` と `NearestFallback`（決めた位置の最近傍）が、同じ矩形について必ず一緒に出る。
- **困る場面**: 見えていたバルーンが、提案位置で画面の外へ出る（`balloon_visibility_guard_clamps_x_on_non_drag_placement_triggers` の形・(2000,240)・500×300）。`OffscreenPull`・`NearestFallback`・`ClampX` の 3 行が出て、`OffscreenPull` は「画面の外に居た窓を引き寄せた」と書くが、実際は見えていた窓を引き戻しただけ。利用者が自分で画面の外へ置いたバルーン（`Keep`）でも `OffscreenPull` が出て、動いていないのに「引き寄せた」と書く。キャラ窓の檻（`follow_visibility_char_wiring_tests.rs:401-405`）は「1 つの出来事を 2 語で数えない」を固定しており、`move_shift.rs:171` は同じ穴を自前で避けている。実機の記録（`.kiro/specs/completed/areka-P0-balloon-text-bugs/real-machine-signoff.md:189`）にバルーンの `OffscreenPull／NearestFallback／ClampX` が 565 行並ぶ。バルーンのテストは `OFFSCREEN_PULL_TAG` を 1 度も見ない。
- **いちばん小さい直し**: 321 行の条件に「`raw != proposed`」を足す（動かしていなければ引き寄せていない）。キャラ窓の道（`resize_window_to` は射影が動かしたときだけ `raw != proposed`・`move_shift` は `raw == proposed`）でも正しい。テスト＝バルーンの寄せと留置の 2 本で `OffscreenPull` が 0 行（直す前は 1 行）。
- **範囲の外への波及**: なし（記録の語の出方だけ・本体の手順書 `diagnosis-procedure.md` の数え方は、出る行が減る向きにしか変わらない）。
- **分類**: 普通に直す。

### 直したほうがよい

#### 5. 面の `move` のずれを当てると、利用者が画面の端に置いた窓が中へ寄せられ、ずれが 0 に戻っても元へ戻らない（`follow/move_shift.rs:160-163`）

- **何か**: `apply_char_shift` は「元の位置＋ずれ」の矩形を、作業領域の 4 辺へ丸ごと寄せる（`clamp_rect_to_work_area`）。元の位置そのものが作業領域からはみ出していても寄せる。
- **困る場面**: 作業領域の右端 1920・幅 200 の Free の窓を、利用者が x=1820 に置いた。ずれ (5,0) で x=1720 へ寄り、当てた分は −100。ずれが (0,0) に戻ると元の位置 1820 をまた寄せて 1720。窓は 1820 へ戻らない。保存の側（`origin_position`）は 1820 と言うので、今の位置と次の起動の位置が食い違う。作業領域への寄せ（`window-work-area-clamp`）は既定の位置の窓だけで、利用者が置いた窓は寄せない決まり（`clamp_grown_free_default`・守りの「明示の留置を尊重」）と食い違う。
- **いちばん小さい直し**: 案 甲＝寄せる先を「作業領域と元の矩形を合わせた範囲」にする（ずれで外へは出さないが、元の位置は尊重）。案 乙＝ずれが 0 のときは寄せない（最小・端の 1 例だけ救う）。案 丙＝今のまま（裁定どおり）。テスト＝はみ出した元の位置で (5,0)→(0,0) が元へ戻る。
- **範囲の外への波及**: なし。
- **分類**: **種類 1 の議題**（議題 2）。完了した `seriko-pattern-calls` の要件 7.8 が「`move` でずらした後のキャラクターの窓の位置にも、今ある置き方の決まり（…作業領域の中に置く決まり）をそのまま当てる」と定め、2026-10-30 の議題 3 で開発者が決めている（例「`move,0,10,0,9999` も作業領域の中へ戻される」）。「今ある決まり」が利用者の置いた窓をどう扱うかまでは書いていない。SSP の見え方に合わせた決めなので種類 3 にも近い（ukadoc の `move` の項に端の扱いの記述は無い見込み＝11 節で引く）。

#### 6. `ClampX` が X だけを直し、縦に見えないままでも「引き戻した」と記録する（`follow/visibility.rs:101-104,352-363`）

- **何か**: 守りは見える→見えないの移り変わりで X だけを作業領域へ寄せ、Y は変えない（設計 D6「Y は射影の所有」）。事後条件「寄せた後は作業領域と交わる」（66〜67 行）は、射影が Y を決めた正常系でだけ成り立つ。バルーンには射影が無い。
- **困る場面**: 下に揃えたキャラ窓が y=640、バルーンの offset が (100,−1000)、提案 (1600,−360)・500×300。どの作業領域とも交わらず、旧矩形は見えていたので `ClampX` で x=1420 になるが y=−360 のまま＝バルーンは丸ごと見えないのに `[visibility-guard] ClampX … X を引き戻した` が出る。
- **いちばん小さい直し**: 案 甲＝寄せた後も交わらないときは別の語（例 `ClampXStillHidden`）で記録する（位置は今のまま）。案 乙＝バルーンの呼び手に限り Y も寄せる。案 丙＝今のまま、`guard_balloon_position` の説明に限界を書く。
- **範囲の外への波及**: 案 甲は本体の手順書の語の表が 1 語増える（`diagnosis-procedure.md`・完了した spec の文書）。案 乙は外から見える位置が変わる。
- **分類**: **種類 1 の議題**（議題 3）。D6 は完了した `areka-P0-dpi-transition-atomicity` の設計の決め。

#### 7. 保存を `debug!` だけで黙って捨てる道がある（`follow/drag_follow.rs:210-259`・`:697-737`）

- **何か**: `on_balloon_drag_end` は `BalloonWindowMarker` が無い・追従元のキャラ窓が見つからないとき、`on_char_drag_end` は `CharWindowMarker` が無いとき、`debug!` だけで保存を飛ばす。追従元を探す問い合わせは記録にしか使わない `&Anchored` まで要るので、`Anchored` の無いキャラ窓だとバルーンの保存が `debug!` 水準で消える。
- **困る場面**: 部品の付け方が変わって `Anchored` か印が欠けた窓をドラッグすると、利用者が置いた位置が保存されず、既定の水準の記録には何も残らない（今は spawn が全部付けるので届かない）。記録の無い失敗の道を作らない決まり（要件 8.4）の、既にある穴。
- **いちばん小さい直し**: 3 か所を `warn!` にし、問い合わせから `&Anchored` を外す（記録は `Option` で出す）。ハンドラは `bool` を返す形で `Err` を返せないので、記録だけ上げる（ドラッグのハンドラの約束＝常に伝播を続ける）。テスト＝印の無いバルーンを離すと警告が 1 行。
- **範囲の外への波及**: なし。
- **分類**: 普通に直す。`Err` を返せない形をどう読むかは設計で決める（議題にはしない）。

#### 8. `anchor_changed_system` は本番に呼び手が無い（`follow/window_move.rs:449-480`・再公開 `follow.rs:138-142`・説明 `anchor.rs:173-175`）

- **何か**: `Changed<Anchored>` に反応して窓を置き直す system。schedule への登録はツリーのどこにも無く（grep・本体は説明の中で名を挙げるだけ）、テスト（`follow_resize_tests.rs:603-722` の 3 本と `follow_window_move_diag_tests.rs:294`）だけが呼ぶ。本番の `AnchorChange` の書込は `reseed.rs:263` から `resize_window_to` を通る。中に `.expect(..)` がある。`deferral_covers_route` の分類（`window_move.rs:548-552`）は「本番では未結線・結線したら決めさせるため鳴る側に置く」と書く。
- **困る場面**: 4 本のテストは利用者が通らない道を守っている。`reseed` の側の置き直しが壊れても、これらは緑のまま。読む人は「アンカーの変化は system が拾う」と読み違える（`anchor.rs:173-175`「反応 system は後続 task」）。
- **いちばん小さい直し**: 案 甲＝system・4 本のテスト・再公開・`anchor.rs` の説明を消す（`AnchorChange` の経路語は `reseed` が使うので残す）。案 乙＝語彙の口として残し、属性（要らない＝指摘 9）を外して、説明に「本番は `reseed` から書く・結線の予定は無い」を書く。
- **範囲の外への波及**: 消すのは公開の項目（`follow::anchor_changed_system`）を減らす＝要件 7.1 の字面に当たる（本体の使い手は説明の文だけ・コードの使い手は 0）。
- **分類**: **種類 2 の議題**（議題 4）。実在の理由を付けた `#[allow(dead_code)]`（「schedule へ登録する呼び手は本番に無い・テストだけが使う」）なので、わざと置いた先送りの印に当たりうる。消す案は C20 の約束（7.1）にも当たる。

#### 9. 理由の古い `#[allow]` 33 個と、古い理由の説明（申し送り ⑵・`follow.rs`・`follow/work_area.rs`・`follow/work_area_clamp.rs`・`follow/window_move.rs`・`transition_diag.rs`）

- **何か**: 36 個の属性のうち 33 個は、消しても警告が出ない（`cargo check -p areka-placement --lib --tests --examples` で確かめた・4 節の表）。`pub mod` の下の `pub` の項目と `pub use` は lib の外から届くので、`dead_code`・`unused_imports` は鳴らない。理由の文は「areka は bin crate」「examples が `#[path]` で取り込む」を前提にしていて、どちらも事実でない（`examples/window-placement.rs` は公開の API だけを使う・`collision-probe` は本体の example で配置のソースを取り込まない）。残る 3 個（`follow.rs:112`・`follow.rs:123`・`work_area_clamp.rs:118`）は要るが、理由は「テストだけが名で呼ぶ」に書き直す。
- **困る場面**: 要らない属性は、本当に使われなくなった項目の警告を隠す（`transition_diag.rs` の説明 44 行がまさにそれを避けたいと書いている）。古い理由の文は、読む人に crate の形を読み違えさせる。
- **いちばん小さい直し**: 33 個を消す。3 個は理由を書き直すか、項目そのものを `#[cfg(test)]` にする（`resolve_chain_in_work_area` と `follow.rs:112-116` の再束縛はテストだけが使う＝指摘 23 と同じ直し）。説明（`follow.rs:77-82,98-102,110-111,126-129,134-137,147-149,153-156,165-166,169-170`・`work_area.rs:275-277`・`work_area_clamp.rs:18-20` と各行の注・`transition_diag.rs:38-46` と 14 行の注・`offset_space.rs:49-58`）を今の事実に合わせる。
- **範囲の外への波及**: なし（属性と説明だけ・公開の形は変わらない）。
- **分類**: 普通に直す。`follow.rs:108` と `window_move.rs:449` は brief が「古くなっていない」とした分だが、属性そのものは要らない＝要件 2.3 と 2.5 の読み方を議題 5 にする。

#### 10. `OffsetRescale::Unchanged` の説明が、呼び手の守る約束と逆のことを言う（`follow/offset_space.rs:182-183,213-215`）

- **何か**: `Unchanged` は「値も基準も変えない」「恒等——値も基準も動かさない」と書く。ところが唯一の呼び手（`crates/areka/src/runtime/frame/balloon_offset_follow.rs:46-54`）は、`Unchanged` でも `base.offset` を書き戻さねばならない（144→192→144 で高 DPI の値が取り残されるのを防ぐ・2026-08-28 の是正）。
- **困る場面**: 新しい呼び手がこの説明を信じて `Unchanged` で何もしないと、DPI の往復で値がずれる欠陥をもう一度作る。
- **いちばん小さい直し**: 説明を「比が恒等。呼び手は `apply_rescaled` で `base.offset` を書き戻す」に直す（2 か所）。
- **範囲の外への波及**: なし。
- **分類**: 普通に直す。

#### 11. 本番の分岐で、壊れても落ちるテストが無いもの

- **場所と何か**（どれも開いて確かめた）:
  - ⑴ `enqueue_window_set_pos` の寸の有無で `SWP_NOSIZE` を付け外す腕（`window_move.rs:718-723`）。テストは ECS の写し（`WindowPos`）しか見ない。2 つの腕を入れ替えると、全部の移動と随伴が窓を 0×0 に縮めるが全部緑。説明（`follow_window_move_tests.rs:260-262`・`follow_resize_tests.rs:19-21,729-736`）は「書込の列は私有で中を覗けない」と書くが、`wintf::ecs::window::drain_window_pos_commands()` は公開で、`follow_work_area_clamp_grow_tests.rs:263`・`follow_move_shift_tests.rs:349` が既に使っている。
  - ⑵ ドラッグの終わりの保存の形を descript の向きで選ぶ道（`drag_follow.rs:225` の `saved_form_anchor(..).unwrap_or(anchor)`・`sticky_follow.rs:292-301` の組の相手）。どのテストも `AnchorSource` を付けない。descript が Bottom で台本が Free に変えた幅 200 の窓を (100,300) で離すと、正しくは x=200 を保存する。`anchor` を使う形に壊すと x=100 を保存し、次の起動で 100 px 左に出るが全部緑。相手に `Anchored` が無いときの `warn!` の腕も試されていない。
  - ⑶ `on_balloon_drag_end` の `Phase::Tunnel` と他の窓宛ての守り（`drag_follow.rs:672-678`）。`drag_handlers_ignore_events_targeting_other_entities`（`follow_drag_tests.rs:762-802`）は他の 3 つのハンドラしか呼ばない。守りを消すと 1 回の離しで保存・補正・素材の退役が 2 度走るが全部緑。
  - ⑷ `apply_char_shift` の `write_refused`（`move_shift.rs:176-178`・書けなかったら記録を書かない＝次のフレームで当て直す約束）と、`apply_balloon_shift` の持ち主のキャラ窓が無い腕（`:220-222`）。記録の挿入を書込の前へ動かしても全部緑。
  - ⑸ 記憶から戻した組の「印だけで繋がる」腕（`work_area_clamp.rs:506` の `Some(Left|Right)`）。テストの `Right` の組はどれも記憶の矩形が接している（隙間 0）ので、`judge_adjacency` だけで繋がる。`Left` は 1 度も使わない。腕を消すと、寸が変わって離れた「右」の組が 1 つずつ寄せられて重なるが全部緑。
  - ⑹ 面の拡大の寄せが「提案の矩形の作業領域」を使う決まり（`work_area_clamp.rs:375`・要件 7.1）と、World の口の `no-work-area` の理由（`:261`・写しはあるが空）。テストはモニタ 1 台・提案と今の位置が同じなので、今の位置を使う形に壊しても緑。
  - ⑺ `keyword_base.rs:87-93`（`BalloonFollow` が無い）・`:99-105`（`CharWindowMarker` が無い）の `warn!` の腕。素材を残す約束を壊しても緑。
- **いちばん小さい直し**: ⑴ 2 本のテストで書込の列を引き、`flags` と寸を見る（説明も直す）。⑵ `AnchorSource{descript: Bottom, by_script: true}`＋`Anchored(Free)` の窓で 1 本・組の相手で 1 本。⑶ 今のテストに 2 つの呼び出しを足す。⑷ `missing_inputs_do_not_apply` に 2 つの形を足す。⑸ 離れた矩形で `Right`・`Left` の 2 本。⑹ モニタ 2 台で提案が別のモニタに載る 1 本と、空の写しの 1 本。⑺ 部品を 1 つずつ外す表のテスト 1 本。
- **範囲の外への波及**: なし（テストだけ）。
- **分類**: 普通に直す。

#### 12. 移り変わりの判定器が、壊れた行を黙って捨てる（`placement_tests/transition_judge_offset.rs:416-449,707-714`・`transition_judge_offset_signoff_tests.rs:402-408`）

- **何か**: ⑴ 追随の行（`kind=offset`）の判定は、遷移の区切りの中の行しか見ない。最初の起点より前の行は、`keyword_pending_scopes` が形の壊れた行を落としてから読む。オフセットのサインオフの走り手は `judge_offset_log` だけを呼ぶので、`transition_judge_verdict.rs` の `unassigned_malformed_records` に当たるものが無い。⑵ 走り手は、追随の行はあるが数えられないとき（起点が無い・全部の行が壊れている）、`report` を出す前に `Err("…追随レコードが 1 行も無い… 観測 target と水準が有効になっていない")` を返す。
- **困る場面**: ⑴ 起点より前の scope 1 の `keyword-pending` の行が欄の重複で壊れると、scope 1 は ⑷ の確かめから黙って外れ、scope 1 の 10 px の残差が合格になる。⑵ 発行元が欄の名を変えた（全部の行が `MalformedRecord`）とき、走り手は「`RUST_LOG` を見直せ」と言い、本当の理由（壊れた行の一覧）は出ない。
- **いちばん小さい直し**: ⑴ `judge_offset_log` で、どの区切りにも入らない壊れた `kind=offset` の行ごとに違反を足す（`MalformedRecord` の `transition` を `Option` にするか、新しい腕を 1 つ）。⑵ `Err` の文に `{report}` を足す（1 行）。テスト＝起点の前の壊れた行で赤・全部壊れた行で `Err` に違反の一覧が載る。
- **範囲の外への波及**: なし（判定器は本体の `placement_tests` の中・`runtime/` の檻は判定器の署名を変えなければ触らない）。
- **分類**: 普通に直す。

#### 13. 判定器の数え方が緩く、測っていない scope や往復を合格に数える（`transition_judge_offset.rs:720-757,586-603,661-666`）

- **何か**: ⑴ キーワードの scope の揃えの確かめ（⑷）は、全部の scope をまとめた `measured` の数が 0 のときだけ `NoKeywordAlignmentMeasured` を出す。往復の確かめ（⑴・579〜609 行）も、どれか 1 つの scope が往復すれば通る。⑵ 1 つの遷移の中に同じ scope の同じ行が 2 つあると、往復 1 回と数える（発行元の約束は「遷移ごと scope ごとに 1 行まで」）。⑶ 元の DPI へ戻る `rescaled` の行（`base_dpi == new_dpi`・残差は作りから 0）を、測った 1 回に数える。
- **困る場面**: ⑴ scope 0 と 1 がどちらも `keyword-pending` を出し、後で `rescaled` が来たのが scope 0 だけでも合格＝scope 1 は確かめられていない（`transition_judge_verdict.rs` が窓ごとに `judged_windows` で塞いでいる「黙っていれば合格」の形）。⑵ 二重に出す後退が入ると、192 へ戻っていないのに往復が観測されたことになる。⑶ 拡大の側が `saturated`（測らない）で戻りの側だけ `rescaled` の scope が、一度も拡大した値を確かめずに合格する。
- **いちばん小さい直し**: ⑴ `measured` を scope ごとに数え、0 の scope ごとに違反を出す（scope の欄つき）。⑵ 往復は「最初の遷移と違う遷移の行」だけを数える。⑶ `residual_measurable` で `base_dpi == new_dpi` も `None` にする。テスト 3 本。
- **範囲の外への波及**: なし。ただし ⑴ を直すと、実機のサインオフの合否が厳しくなる（今までの実機ログで赤になるかは 11 節）。
- **分類**: 普通に直す。⑴ が scope ごとの意図か全体の意図かは要件（完了した `areka-P0-balloon-offset-dpi` の要件 8.5）を読んで設計で決める（議題にはしない・11 節）。

#### 14. 判定器の分岐で、負のテスト（壊れたら赤になるテスト）が無いもの

- **場所と何か**:
  - ⑴ 直の `BalloonFollow` の書込とまとめられた書込が同じ窓に混ざる形（`transition_judge.rs:928-940` の `note_follow_landing`・725・838 行から呼ぶ）。後ろにある着地を残す守り `*current_index >= write_index` を消すと、遅れた追随が合格になるが全部緑。
  - ⑵ 往復の鍵（`RoundTripKey`）に `base_offset`・`base_dpi` を入れて、基準の付け替え（ドラッグ）を新しい往復の区切りにする所（`transition_judge_offset.rs:569-609`＝この判定器の核）。2 つの欄を鍵から外しても全部緑。
  - ⑶ 揃えの残差の境（`:680` の `>`）。テストの説明（`transition_judge_offset_tests.rs:646-665`）は「許容量ちょうど（3px）は合格側」と書くが、試すのは 2.5 px だけ。`>=` に変えても緑。
  - ⑷ `STABLE_VERDICTS`（`:82-87`）は `unresolved` だけが試される。`anchored`・`unchanged`・`keyword-pending` を表から落としても緑。`expected_verdicts`（`:518`）の非恒等の腕と、`base_offset == old_offset` の恒等の腕は試されない。`point_field` の誤り（`:476-491`・`new_offset=abc`）と、`residual_measurable` の `base_dpi != 0` の守り（`:665`・無いと 0 割りで panic）も試されない。
- **いちばん小さい直し**: それぞれ 1 本ずつ（⑴ 混ざる形で `!balloon_same_frame`・⑵ 付け替えた scope が緑のまま・⑶ ちょうど 3.0 px が緑・⑷ `STABLE_VERDICTS` を回すテスト・2 つの腕・壊れた点の欄・`base_dpi=0` の行）。
- **範囲の外への波及**: なし。足す先は `transition_judge_offset_tests.rs`（836 行）と `transition_judge_coalesced_follow_tests.rs`（584 行）。1,000 行を越えるなら新しいファイルへ（要件 8.1）。
- **分類**: 普通に直す。

#### 15. 失敗しえないテスト・名前と違うことを確かめるテスト（偽の安心）

- **場所と何か**（どれも開いて確かめた）:
  - `transition_signoff_procedure_tests.rs:350-372`（`the_offset_procedure_names_its_runner_vocabulary_verbatim`）: 手順書として `transition_judge_offset_signoff_tests.rs` のファイル全体を読む。手順書は `//!` の説明だけなのに、コードの行も探す。`judges_a_real_machine_offset_log` は 415 行の関数名、`transition_judge_offset_signoff` は 393 行の文字列、`AREKA_TRANSITION_LOG` は 414 行の `#[ignore]` の理由にあるので、説明から語を全部消しても緑。374〜433 行の句の確かめも同じ。
  - `follow_offset_space_tests.rs:710-726`（`transition_verdict_is_axis_independent`）: 最初の `assert_ne!` はテストの中の定数どうしの比べ。`rescale_follow_offset` は軸を入力に持たないので、軸に依るかはこのテストで分からない。
  - `follow_offset_component_tests.rs:133-151,153-174`: 値を手で渡すので、往復は一度も歩かれない。`:116` のテストが落ちるときしか落ちない。
  - `follow_offset_space_tests.rs:154-164`（`assert_value_untouched`）と `:193`: 1 行目は自分で「自明」と書く比べ。呼ぶ 7 か所（282・297・741・762・781・817・843）は先に判定の腕を確かめ済み。
  - `transition_judge_reobservation_tests.rs:264-284`: 手書きの検体の `t_us` の最大と最小を比べるだけで、判定器を呼ばない。
  - `transition_diag_tests.rs:389-464`（`the_declared_field_lists_match_the_lines_that_are_actually_built`）: 名は「過不足なく一致」だが、表⊆行しか見ず、`chain` は先頭の欄だけ、`snapshot`・`chain` の行は組まない。`CHAIN_FIELDS` から `FIELD_REASON` を消しても緑（判定器は `reason` を求めなくなる）。
  - `transition_judge_offset_tests.rs:831-835`: `format!("let x = {probe};").contains(&probe)` は常に真＝較正の段が働いていない。
  - `follow_move_shift_tests.rs:293-319`（`extreme_balloon_shift_does_not_overflow`）: 説明は「飽和する」と約束するが、`true` が返ることしか見ない。
  - `follow_anchor_tests.rs:330-361`: bevy の部品の付け外しを試すだけ（コンパイルの事実）。
  - `follow_drag_tests.rs:94-105`・`follow_balloon_drag_tests.rs:286-291`: ハンドラが常に `false` を返す・その道は窓を書かない、ので「変わらない」は常に成り立つ。
  - `follow_window_move_hold_watch_tests.rs:116`: 13 個の枠で全部の経路を 94 行で見つけた後なので、重複は先に 94 行で落ちる。
  - `transition_judge_tests.rs:867-893`: 最後の `assert_ne!(KIND_MSG, KIND_MONITOR)` は定数どうし。まとめられていない `enqueue` が追随に数えられないこと（`balloon_follow_windows` が空）を見ていない。
  - `transition_judge_verdict_tests.rs:124-138`: 名は「起点で始まらない区切り」だが、空の列しか試さない。
  - `follow_visibility_char_wiring_tests.rs:413-437`: 戻り値を捨てるので、`resize_window_to` が早く戻っても「出ない」が通る。組んだ World の `WindowPos` を上書きし `let _ = old_pos;` が残る。
  - `transition_judge_origin_tests.rs:228-234`: 215 行の列の一致の後では数の比べは落ちない。
  - `transition_diag_tests.rs:347-385`（壊れた語 2 本）・`:658-661`（68〜77 行の配列の一致に含まれる）・`follow_transition_diag_tests.rs:418-439`（ソースの字面を固定・名前を変えるだけで赤・振る舞いは見ない）。
  - `follow_work_area_tests.rs:191,302`: 1 行の包み（`work_area_for_window`・`MonitorSources::from_monitors`）が中身と同じかを比べる。包みを書き換えない限り落ちない（消すなら `work_area.rs:76-77` の名指しも直す）。
  - `follow_resize_tests.rs:216-221,233-234,242-245,312-313,509-510`: 直前の確かめの算数の言い直し。
- **いちばん小さい直し**: `read_procedure` で `.rs` のときは `//!` の行だけを残す（1 行）。ほかは消すか、落ちうる確かめに替える（往復は `rescale_follow_offset` で値を出す・`assert_ne!` を `balloon_follow_windows.is_empty()` に・較正は判定器のソースに `OFFSET_VERDICT_RESCALED` の名が在ること・表は行の欄の名から接頭の 3 欄を除いた集合と完全一致・戻り値を `assert!`）。
- **範囲の外への波及**: なし。
- **分類**: 普通に直す。

#### 16. テストの組み立て部品が多くのファイルに写してある

- **場所**（範囲の中）:
  - limit の 2 ファイル（`follow_balloon_limit_tests.rs:39-158` と `follow_drag_end_limit_tests.rs:36-183`）: `wa`・`single_monitor`・`balloon_size`・`max_pos`・`contained`・`base_placement`・`world_with`・`base_world`・`assert_no_event` と 4 つの定数。`wa`・`single_monitor` は `follow_test_support::odd_edge_snapshot` と同じ矩形、`left_wa`（`follow_balloon_limit_tests.rs:56-58`）は `follow_test_support::left_wa` の写し。
  - `assert_no_event` が 3 つ（`follow_keyword_base_tests.rs:178`・`follow_balloon_limit_tests.rs:148`・`follow_drag_end_limit_tests.rs:173`）。`dpi(v)` が 3 つ（`follow_offset_component_tests.rs:27`・`follow_offset_residual_tests.rs:25`・`follow_offset_space_tests.rs:25`）。`k_axis` が 2 つ（`follow_offset_space_tests.rs:657`・`follow_offset_residual_tests.rs:30`）。`point`・`pt`（`follow_sticky_follow_drag_tests.rs:80`・`follow_offset_component_tests.rs:31`）は `follow_test_support::point` の写し。`tagged` が 3 つ（`follow_work_area_clamp_chain_tests.rs:294`・`_grow_tests.rs:65`・`_restore_tests.rs:52`）で、`follow_test_support::guard_events` と同じ。
  - `SharedFakeIo` と sylphya の組み立て（`ScopeRoots`・`spawn_sylphya`・`PersistWiring`・閉じと join）が範囲の中に 6 つ（`follow_drag_end_gate_tests.rs:60-69`・`_limit_tests.rs:296-316`・`_persist_tests.rs:48-68`・`follow_drag_tests.rs:408-428,521-541,642-662`）と `follow_sticky_follow_drag_tests.rs:149` の中に 2 つ。`follow_drag_end_persist_tests.rs:210-246` と `:476-499` の本物のファイルの組み立ても同じ。
  - `gap_bound_char_world`（`follow_visibility_char_wiring_tests.rs:49-65`）と `gap_bound_char_world_with_balloon`（`follow_visibility_balloon_wiring_tests.rs:696-724`）。`char_with_far_balloon_world`（balloon 72-97）は `char_with_balloon_window_pos(..)`（376-395）で言える。
  - `code_only`（`follow_transition_diag_tests.rs:379`）と `code_only_lines`（`transition_diag_tests.rs:788`）。同じ `Monitor` の値が 3 つ（`transition_diag_tests.rs:531,569`・`follow_transition_diag_tests.rs:54`）。
  - 判定器のテスト: `deterministic_verdict`（`transition_judge_reobservation_tests.rs:434-439`）と `deterministic_violations`（`transition_judge_negative_tests.rs:40-42`）は `test_support::summarize_lines` の写し（支えの規約「ヘルパは 1 件でも本ファイルへ集約する」・`transition_judge_test_support.rs:228`）。`log_of` が 2 つ（`negative_tests.rs:35`・`offset_tests.rs:228`）。`monitor_with_raw_frame`（`frame_tests.rs:22-28`）は欄 `entity=2v0` をリテラルで書く。`coalesced_follow_tests.rs:563-569` は `replace_once` を手で書く。
  - `transition_signoff_tests.rs:112,144`・`transition_judge_offset_signoff_tests.rs:489,507` の `fs::remove_file` は、`TempPath` が落ちるときにフォルダごと消すので要らない。
- **困る場面**: 検体（バルーンの寸 223×158・既定の配置）や記録の文言を 1 つ変えると、6〜8 ファイルを同じに直す。直し忘れたファイルだけが古い前提を確かめ続ける。
- **いちばん小さい直し**: `follow_test_support.rs`（316 行）と `transition_judge_test_support.rs`（346 行）へ上げて写しを消す。
- **範囲の外への波及**: 範囲の外にも同じ写しがある（`SharedFakeIo` は `persist_*`・`reset_pos`・`balloon_lock_drag_tests` に 5 つ、`assert_no_event` は `balloon_limit_gate_tests.rs:96`・`placement_windowposition_vocab_tests.rs:62`、`fake_handle`・`position_of`・`px` は `reseed_test_support.rs`・`resolver_test_support.rs` ほか）。どれも `audit-placement-core` の持ち物＝この spec では範囲の中だけをまとめる（要件 6.4）。
- **分類**: 普通に直す。

#### 17. ログを捕まえる手段の写しが、捕まえ損ねを見張らない（`follow_window_move_diag_tests.rs:491-552`・`follow_transition_diag_tests.rs:303`）

- **何か**: `window_move_output_under_filter` は、自前の `VecWriter` で `log_capture_kit::capture_under_filter`（`log-capture-kit/src/filter.rs:82`）を写したもので、窓の中の見張り（sentinel）と `rebuild_interest_cache` が無い。同じ写しが `follow_transition_diag_tests.rs:303` にもある（範囲の外 `diag_tests.rs:452` にも）。
- **困る場面**: `window_move_records_are_silent_under_default_info_filter` は「出ない」しか見ず、「出る」の対の確かめは別のテスト・別の購読者で走る。捕まえる手段が死ぬと、「出ない」のテストは黙って緑のまま。
- **いちばん小さい直し**: 案 甲＝`crates/areka-placement/Cargo.toml` の `log-capture-kit` の dev-dependency に `features = ["env-filter"]` を足し、2 つの写しを `capture_under_filter` に替える。`Cargo.lock` の `log-capture-kit` の項は既に `tracing-subscriber` を持つ（wintf が有効にしている）ので、`Cargo.lock` は変わらない見込み（11 節で確かめる）。案 乙＝写しのまま、捕まえた行が 1 行以上あることの見張りを写しに足す（範囲の中だけ）。
- **範囲の外への波及**: 案 甲は `crates/areka-placement/Cargo.toml`（範囲の外・`audit-placement-core` の持ち物）に 1 語足す。依存は足さない（同じ crate の feature を開けるだけ）。
- **分類**: 普通に直す。直し方は議題 6。

#### 18. システム由来の 7 経路の表を、3 つのテストファイルが別々に固定している

- **場所**: `follow_default_pos_track_tests.rs:314-328`（列）・`:337-345`（`route_applies_visibility_guard == is_system_reanchor` を全経路で）・`follow_visibility_char_wiring_tests.rs:78-103`（同じ 7 経路の `matches!` と `fired == 7`）・`follow_visibility_balloon_wiring_tests.rs:119-123`（`fired == 7`）。
- **困る場面**: `is_system_reanchor` に 1 経路足すと、3 つのテストファイルを同じに直す。
- **いちばん小さい直し**: キャラ窓の表のテストを消し（列と区分の一致は `default_pos_track` が固定済み）、バルーンの表のテストから `fired` の数を外す（`Drag` の腕と経路ごとの一致は残す）。
- **範囲の外への波及**: なし。
- **分類**: 普通に直す。

### できれば

#### 19. 同じことを 2 度確かめているテスト

- `transition_judge_verdict_tests.rs:1011-1025` は `:394-408` と同じ入力・同じ合否（違うのは 2 つの数の確かめだけ＝前のテストへ移す）。
- `transition_judge_tests.rs:628-668` は `:806-836` と同じ 2 つの欄を見る（-41 行）。
- `transition_judge_frame_tests.rs:120-142` は `transition_judge_negative_tests.rs:261-273` に丸ごと含まれる。
- `transition_judge_offset_tests.rs` の `:321`・`:512`・`:598`・`:741` の 4 本は、`:243`（`the_known_passing_log_is_green`）に検体の中身の確かめを足しただけ。
- `follow_offset_space_tests.rs:41-129` の初めの 4 本は、後の網羅の節（`:199-211`・`:738-744`・`identity_dpi_never_moves_any_value`）と同じ入力・同じ期待。`:862-882` は各腕の確かめが落ちるときしか落ちない。
- `follow_resize_tests.rs:545-587`（Top の写し・自分で「代表」と書く）・`:938` は範囲の外の `diag_tests.rs:108` と同じ形（範囲の外の側が多く見る＝範囲の中の側を消す）。`:144` と `diag_tests.rs:70`、`:833` と `diag_tests.rs:130` は同じ組み立て。
- `follow_window_move_tests.rs:23` と `:234`、`:56` と `:194` は同じ移動先。`follow_window_move_diag_tests.rs:241` は `:215` の一部。
- `follow_drag_end_limit_tests.rs:648-673` は `:284-438` の一部（足すのは常に真の「`GhostWindows` がある」だけ）。
- `follow_anchor_tests.rs` の Bottom: `project_anchor(Bottom, ..)` は `BottomSnapPolicy.resolve` をそのまま返す（`anchor.rs:117-119`）のに、`:18-81` と `project_anchor_*` の Bottom の部分（93-114・191-222・257-273・287-291・315）が同じ場合を 2 度確かめる。
- `follow_visibility_guard_tests.rs:214-289` のバルーンの 3 本は、窓の種類を持たない `guard_visibility` をキャラ窓の 3 本と寸だけ変えて回す（`:304-362` が両方の寸で全部の腕を回す）。
- **直し**: 後ろの側（または弱い側）を消すか 1 本へまとめる。`transition_judge_tests.rs` は 41 行減って 998 行＝番人の例外表から外せる（要件 8.3・9 節）。**波及**: なし（`diag_tests.rs` は残す）。**分類**: 普通に直す。

#### 20. 未確定の印（`CW_USEDEFAULT`・`i32::MIN`）の扱いが場所で違う

- `window_move.rs:1191-1207`（`resize_window_keep_position`）は位置の印を見ずに書く。`resize_window_to`（`:258`）と `window_rect_of`（`:1118`）は見る。位置が印のままのバルーンに当たると x=`i32::MIN` へ `SetWindowPos` を出す。
- `drag_follow.rs:216-239`（`on_char_drag_end` の保存）は寸の `> 0` を見ない。`guard_balloon_position`・`apply_release_limit_correction` は見る。寸が印のままの Bottom の窓を離すと `x = 左 + i32::MIN/2`（約 −10 億）を `sylphya.toml` に保存する。寸が無いときは左上の x を保存し、戻すときは中心として読むので w/2 ずれる。
- `work_area_clamp.rs:209-217,231-233`（`place_chain_in_work_area`）は高さの `> 0` を見ない。印の高さを「分かっている」と数え、矩形が裏返って縦に並んだモニタの一番上を選ぶ。
- **困る場面**: どれも今は spawn が実の位置と寸を入れる（`spawn.rs:648`）ので届かない。同じ決まりを場所で別に当てていて、部品の付け方が変わると届く。
- **直し**: 3 か所に同じ `.filter(..)`（各 1 行）とテスト 1 本ずつ。`window_move.rs` は例外表の 1,231 行＝行を足さない（指摘 2・8 で減る分の中で入れる・8 節）。**波及**: なし。**分類**: 普通に直す。

#### 21. 組のドラッグの終わりで、動かした窓と相手で、面の `move` のずれの扱いが違う（`drag_follow.rs:193-194`・`sticky_follow.rs:299-302`）

- **何か**: 動かした窓は当てた分を 0 にする（置いた所が新しい元の位置）。組の相手は当てた分を残し、元の位置を「置いた所−当てた分」とする（`finish_persists_partner_position_without_its_pattern_shift` が固定）。
- **困る場面**: 両方の窓が面のずれを持ったまま組でドラッグし、ずれが終わると、相手だけがずれの分戻り、動かした窓は戻らない。組の並びが動かした窓の古いずれの分ずれる。
- **直し**: どちらか 1 つの決まりに揃える（相手も 0 にする・動かした窓も引く）。**波及**: なし。**分類**: **種類 1 の議題**になりうる（`seriko-pattern-calls` の task 6.2「ドラッグで作った相対の位置へ二重に足さない」と、組の相手の固定のテストの意図を読む）＝議題 7。

#### 22. 古くなった説明（旧い名前 `emo2_boot` を含む）

- 旧い名前 `emo2_boot`（今は `runtime`）: `follow.rs:80,98,136,147,154,156,169`・`follow/move_shift.rs:7`・`follow/offset_space.rs:57`・`follow/window_move.rs:33,968,1161`・`follow/work_area.rs:28,249`・`follow_balloon_limit_tests.rs:455`・`follow_default_pos_track_tests.rs:14`・`follow_transition_diag_tests.rs:16,23`・`follow_visibility_balloon_wiring_tests.rs:854,858`（9 ファイル 21 か所・4 節）。
- crate の形: `follow.rs:77-82`（「crate 内に留める」だが `pub use`）・`:147-149`（「`pub(crate)` へ格上げ」だが `pub`・「bin crate」）・`offset_space.rs:49-58`・`transition_diag.rs:38-46`・`work_area.rs:275-277`・`work_area_clamp.rs:18-20`・`placement_tests/transition_judge.rs:49`（「`areka_placement::test_support` と同じ形（`#[cfg(test)]`）」だが今は `#[cfg(any(test, feature = "test-support"))]`）。`transition_judge.rs:47` の「areka は lib ターゲットを持たない bin crate」は今も事実（本体は `[[bin]]` だけ）。
- 数: `transition_diag.rs:5-6`・`transition_diag_tests.rs:1-7`（「4 種」→ 5 種）・`transition_judge.rs:3-4,99`（「3 crate」→ 4）・`visibility.rs:215`・`follow_default_pos_track_tests.rs:143,155`（「6 経路」→ 7）・`follow_visibility_char_wiring_tests.rs:67-70`（「13 種…4 種…残り 8 種」・算数も合わない・`WorkAreaResnap` は今は `resize_window_to` を通る）・`:73-76,105`・`follow_visibility_balloon_wiring_tests.rs:126`・`window_move.rs:149-158`（「4 経路」→ 7）・`follow_test_support.rs:246-257`（「3 語に共通の接頭辞」が `OFFSCREEN_PULL_TAG` の説明の 1 行目に紛れている・今は 4 語）・`follow_work_area_tests.rs:160`（幅 60 → 40）・`follow_window_move_hold_watch_tests.rs:63`（「プロセス共有のキュー」→ スレッドごと＝`drop_queued_writes` は分離に効かない）。
- もう済んだ話: `transition_diag.rs:30-36`（`log_offset_rescale` は語彙だけ → 本番 `balloon_offset_follow.rs:160,180,251` が呼ぶ）・`drag_follow.rs:517-518`（「記憶はセッション内のみ・ghost.dat は M-life」→ `on_balloon_drag_end` が保存する）・`follow_drag_end_persist_tests.rs:444-446`（`DraggingState` が無ければ保存しない → 今は `WindowPos` で補う）・`follow_drag_end_gate_tests.rs:3-13`（直しの経緯）・`anchor.rs:173-175`（指摘 8）・`transition_judge_reobservation_tests.rs:8-11`（検体の `t_us` の順が忠実でない＝今の検体には 13,800 が無く、`Bounds::signoff` で判定されている）・`:36-41`（「task 4.2 で必ず再検証」）・`:69-71`（使い手の名）・`:251`（「整合待ちは未実装」）・`follow_transition_diag_tests.rs:109`（「現行の欠陥を固定する」・名の `current_48px_float` はモジュールの説明と逆）。
- 行の番号・名の参照: `window_move.rs:777`（「:753 で `Some` を渡す」）・`transition_diag.rs:713`（`follow_balloon_limit_tests.rs:246`）・`transition_judge.rs:15`・`transition_judge_offset.rs:19`（``[`super::transition_diag`]`` は今の `super` に無い）・`transition_judge.rs:22,403`（`judge-perf.py:562,588-596` → 789〜797・763）・`transition_judge.rs:45-50`（使い手の一覧に `runtime/frame_transition_*` が無い）・`transition_signoff_procedure_tests.rs:747-750`（`verdict.rs:551-565`・`:857-863` → 550〜573・864〜871）・`transition_judge_tests.rs:1-8`（姉妹のモジュールの一覧が 2 つだけ）・`:602-605`（`last_follow_frame_per_window` → `follow_landing_per_window`）・`transition_judge_offset_signoff_tests.rs:108`（`persist.rs:388` → 560〜570）・`transition_judge_origin_tests.rs:154-158`（「逐語のログ」だが合成の値）・`transition_judge_test_support.rs:8-10`（`builders_produce_well_formed_records` が全部の部品を見ると書くが `windpi`・`enqueue_merged_into` は見ない）・`:68`（「上と同じ理由」の上に理由が無い・理由は 138〜139・161〜162 行）・`follow_test_support.rs:144-145`（`resolver.rs` の `px()` → `resolver_test_support.rs:11`）。
- **直し**: 事実に合わせるか消す（行の番号は名に替える）。**波及**: 範囲の外の `runtime/frame_transition_atomicity_tests.rs:57,150`（`transition_judge.rs:609-613` → 711〜716）と `diag_tests.rs:158,278,492` にも古い参照があるが、範囲の外なので残す（要件 6.4・持ち主へ申し送る）。**分類**: 普通に直す。旧い名前を直したら `runtime-crate-split` の brief へ申し送る（要件 2.9）。

#### 23. 本番と判定器の要らない小さなコード

- `anchor.rs:12-69` の `DragPositionPolicy`（実装 1 つ・呼び手 1 つ・範囲の外の使い手 0）＝Bottom の腕の私有の関数にする（`follow.rs:130-131` の再公開と `follow_anchor_tests.rs` の重複が消える＝指摘 19）。公開の項目を減らすので C20 の約束に当たる（指摘 28 と同じ議題）。
- `work_area_clamp.rs:118-125` の `resolve_chain_in_work_area` はテストだけが呼ぶ（World の口は `resolve_chain_with` を呼ぶ）＝`#[cfg(test)]` にする（指摘 9 の属性も消える）。
- `work_area_clamp.rs:235-283` は記録の 1 行のためだけに `base` を 3 か所で同じに付け直す＝`ChainPlacement` に作業領域を持たせるか、付け直しを 1 か所にする。
- `transition_judge_offset.rs:422-430` は `TransitionOrigin::of` が済ませた起点の読みをもう一度する。`continue` の 2 つの腕は届かず、届けば遷移を黙って捨てる＝`TransitionOrigin::of` を使う。
- 「値が無いときは `-`」の整形が 3 つ（`transition_judge_verdict.rs:691-693,736-743`・`transition_judge_offset.rs:331-336`）。うち 2 つは `MISSING` でなくリテラル＝1 つの関数にする。
- `transition_judge.rs:961-999` の 10 個と `transition_judge_offset.rs:759,763` の 2 個の `#[cfg(test)]` は、親の `#[cfg(test)] mod placement_tests`（`main.rs:48`）の下なので働かない。
- `transition_judge_test_support.rs:327-338` の `replace_once` は「1 行だけ」を確かめた後、その行の中の全部を置き換える（説明と違う）＝`replacen(.., 1)` と数の確かめ。
- `visibility.rs:214-218` の `route_applies_visibility_guard` は 1 行の包みだが、説明に「区分の定義元を 1 本にする」理由がある＝決めごととして数えない（下の節）。
- **波及**: `DragPositionPolicy` だけ公開の形に当たる。**分類**: 普通に直す（`DragPositionPolicy` は C20 の約束で止まる）。

#### 24. 出荷の作りでだけ赤になるテスト（`follow_window_move_hold_watch_tests.rs:166-196`）

- `#[should_panic(expected = "DpiSyncHold")]` の 2 本は `window_move.rs:695` の `debug_assert!` に依る。`cargo test --release` では 2 本が赤になる（今は出荷の作りでテストを回す台本が無い）。
- **直し**: `#[cfg_attr(not(debug_assertions), ignore)]`。**波及**: なし。**分類**: 普通に直す。

#### 25. 取り消しのテストが、本番が送らない位置を使う（`follow_drag_end_gate_tests.rs:433-451`・`ended()` 191-197）

- 取り消した `Ended` に今のカーソルの位置を載せる。本番の wintf は押した位置を載せる（`drag_follow.rs:119-122`・本体 `input_events/drag_tests.rs:230` が固定）。期待の値が本番では起きない位置になる（保存が 1 回という数は正しい）。
- **直し**: `cancelled=true` のときは開始の位置を載せ、期待を開始の位置にする。**波及**: なし。**分類**: 普通に直す。

#### 26. 実機のサインオフのログの置き場が `target` の下に決まっていない（`transition_judge_offset_signoff_tests.rs:153`）

- 手順の説明は `$LOG = "<絶対パス>\signoff.log"` と書くだけで、ワークツリーの `target` の下と決めていない（要件 8.6）。
- **直し**: 例を `<ワークツリー>\target\…\signoff.log` にする。**波及**: なし。**分類**: 普通に直す。

#### 27. 判定器が、読めない `base_dpi`・`scope` を「無い」と読む（`transition_judge_offset.rs:465-466`）

- `int_field` は `base_dpi=19x` を `None`＝係留前と読み、`MalformedRecord` にしない（テストの部品の説明 `transition_judge_offset_tests.rs:762` も「読めない欄の誤りは `frame`・`t_us` だけ」と書く）。`verdict=anchored base_dpi=19x` が正しい係留の行として通る。
- **直し**: 生の欄を読み、`-` でなく数にもならなければ違反にする。**波及**: なし。**分類**: 普通に直す（発行元が壊れたときの穴）。

#### 28. 範囲の外に使い手の無い公開の項目（申し送り ⑶・4 節）

- **何か**: 範囲の外に使い手が 0 の公開の項目＝`BottomSnapPolicy`・`DragPositionPolicy`・`VisibilityVerdict`・`guard_visibility`・`MonitorDpiEntry`・`anchor_changed_system`（説明の文だけ）。範囲の外の使い手がテストだけ＝`on_balloon_drag`・`sync_drag_writer`・`WORK_AREA_CLAMP_TAG`・`AnchorSource`・`WorkAreaResolution`・`work_area_for_window_with_origin`・`move_window_with_route`・`MonitorSources::is_empty`、`transition_diag.rs` の語・欄・行の組み立ての大半（本体の判定器とテストが使う）。公開の関数の戻りの型なのに名で呼べない＝`ChainPlacement`・`ChainClampOutcome`（`place_chain_in_work_area` が返す・再公開は `pub(crate)` だけ）。
- **困る場面**: 中の段取りを変えるたびに公開の形が変わるかを確かめる手間が残る。テストだけの使い手のために本番の公開を開けている。
- **いちばん小さい直し**: 使い手 0 の項目は `pub(crate)` に下げる。テストだけの項目は `test-support` の feature の下へ移す（本体は既にこの feature を dev-dependency で付けている）。`ChainPlacement`・`ChainClampOutcome` は名で呼べるよう `pub use` にする（足すだけ）。
- **範囲の外への波及**: 狭める直しは公開の形を変える（要件 7.1）。`ChainPlacement`・`ChainClampOutcome` の再公開は足すだけなので約束に当たらない。
- **分類**: **C20 の約束で止まる**（議題 8・要件 7.3）。足すだけの再公開は C20 のうちに直せる。

### 決めごととして指摘に数えなかったもの

- `shortcut:`・`ponytail:` の注釈は範囲に 0 件。
- `route_applies_visibility_guard`（`visibility.rs:214-218`）は 1 行の包みだが、説明が「区分を 1 本にし、割る日が来たら理由を設計へ登記する」と理由を持つ。
- 解放時の limit の補正が寄せるだけで反対の側へ返さない（`drag_follow.rs` の解放の道）: `window-work-area-clamp` の要件（`requirements.md:145` の 6）の決めどおり。
- 副モニタを抜いたときの最近傍への引き寄せ（`visibility.rs:313-317`）: 開発者の裁定 2026-08-20（位置は変えない・記録だけ残す）。
- 判定器の読みの形と `tools/perf/judge-perf.py` の正規表現の一致: Python の `\s` が Unicode の空白も受けることを除いて同じ（行は ASCII で出るので差は出ない）。
- 範囲の中のテストの一時フォルダは全部 `TempPath`（`<workspace>\target\test-roots\`）。手順書を読む 2 つの道（`.kiro/specs/completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md`・`transition_judge_offset_signoff_tests.rs`）は実在し、無ければ panic する（黙って通らない）。完了した手順書の中の `emo2_boot` のパス（175〜176 行）はどの檻も読まない＝範囲の外。
- `transition_judge_verdict_tests.rs:82-85` の `HOLD_FRAME_ALLOWANCE == DPI_SYNC_HOLD_MAX_FRAMES` は今は自明に見えるが、定義を別のリテラルに替えたら落ちる＝働く。

## 4. 申し送り ⑴〜⑶ と古い名前の判断（要件 2）

### ⑴ wintf との重なり（`follow/work_area.rs` の `MonitorSnapshot`）

- **今の形**: `MonitorSnapshot` は wintf の `Monitor`（`crates/wintf/src/ecs/window/monitor.rs:68`・`handle`・`bounds`・`work_area`・`dpi`・`is_primary` の 5 欄の Component）を包むのではなく、`work_area` を `RectPx` へ写して持つ。`bounds` と `dpi` は別の資源 `MonitorDpiTable` へ写す。2 つは `MonitorSources::from_monitors(&[Monitor])` がまとめて作り、番号で並びを揃える（`same_monitors` が番号で比べる）。`Monitor` の entity も World に残る（`runtime/move_cue_place.rs:86` が主モニタを引く）。
- **判断**: **理由が今も事実なので残す**（推す）。写しの理由（頭の無いテストが合成の写しを組む・座標の型を配置の `RectPx` に揃える）は今も事実で、`MonitorSnapshot { .. }` を直に組むファイルは 50（うち本番 5）。wintf の型へ寄せるのは wintf の型に触る（要件 7.4＝`audit-wintf-window` の持ち物）か、50 ファイルを直す（核のファイルを含む）大きな直しで、audit の指摘にも上がらなかった。寄せるなら C20 の後に別の spec で。

### ⑵ 理由の文が古くなった `#[allow]` と説明（指摘 9）

`cargo check -p areka-placement --lib --tests --examples` で、属性を全部消したときに出た警告は 3 つだけだった（`follow.rs:110` の `pub(crate) use` の 5 項目・`follow.rs:119` の `GroupShift`・`work_area_clamp.rs:111` の `resolve_chain_in_work_area`＝行は消した後の番号）。

| 場所（今の行） | 項目 | 書かれた理由 | 属性は要るか | 判断 |
|---|---|---|---|---|
| `follow.rs:83` | `OffsetRescale`・`rescale_follow_offset` | crate 内・examples の `#[path]` | 要らない | 直す（消す・説明も直す） |
| `follow.rs:103` | `MonitorDpiTable`・`MonitorSources`・`same_monitors` | examples の `#[path]` | 要らない | 直す |
| `follow.rs:108` | `MonitorDpiEntry` | 名で呼ぶのはテストだけ（事実） | 要らない（`pub use`） | 議題 5（brief は「古くない」・要件 2.3 では消す） |
| `follow.rs:112` | `pub(crate) use` の 5 項目 | examples の `#[path]` | **要る** | 直す（理由を「テストだけが名で呼ぶ」に・または `#[cfg(test)]`） |
| `follow.rs:123` | `GroupShift` | 戻りの型を名で呼ぶのは檻だけ（事実） | **要る** | 残す（brief のとおり・数えない） |
| `follow.rs:130`・`:132`・`:138` | `BottomSnapPolicy` ほか・`VisibilityVerdict` ほか・`window_move` の組 | bin crate | 要らない | 直す |
| `follow.rs:157`・`:167`・`:171` | move_shift の組・`CharDragSession` ほか・`follow_sticky_move` ほか | `emo2_boot`・examples | 要らない | 直す |
| `follow.rs:147-149`（説明） | `BalloonFollowTrigger`・`follow_balloon` | `pub(crate)`・bin crate | — | 直す（今は `pub use`） |
| `work_area.rs:278` | `MonitorSources::is_empty` | bin crate | 要らない | 直す |
| `work_area_clamp.rs:38,41,44,48,63,79,95,202` | 鎖の核の型・語・World の口 | examples に呼び手が無い | 要らない | 直す |
| `work_area_clamp.rs:118` | `resolve_chain_in_work_area` | task 1.2 の World の口（事実でない） | **要る** | 直す（`#[cfg(test)]` にすると属性も消える＝指摘 23） |
| `transition_diag.rs` 14 個（94・115・139・160・192・244・248・261・271・275・378・416・499・770） | 語・欄の表・`OffsetRecord`・`point_field`・`offset_line`・`log_offset_rescale` | 語彙先着（bin crate ゆえ） | 要らない（全部が `pub` で届く・`point_field` は `offset_line` から使う・`log_offset_rescale` は本番が呼ぶ） | 直す（語彙の口は全部結線済み＝語彙を消す話ではない・要件 2.4） |
| `window_move.rs:449` | `anchor_changed_system` | 本番に呼び手が無い（事実） | 要らない（`pub`） | 属性は議題 5、項目そのものは議題 4（要件 2.4 で分けて判断） |
| `offset_space.rs:49-58`（説明） | 結線の履歴 | bin crate・`emo2_boot` | — | 直す |
| `transition_judge.rs:47`（説明） | 判定器を本番に置かない理由 | bin crate（**今も事実**） | — | 47 行は残す。49 行の「`#[cfg(test)]` と同じ形」は直す |

- 36 個のうち、消す 33 個（議題 5 の 2 個を含む）・理由を直して残す（または `#[cfg(test)]`）2 個・そのまま残す 1 個（`follow.rs:123`）。

### ⑶ 外から使うために開けた項目（指摘 28）

ツリー全体（`vendors/` を除く）を grep した結果（説明の文の行は数えない）。

| 項目 | 範囲の外の本番の使い手 | 判断 |
|---|---|---|
| `OffsetRescale`・`rescale_follow_offset` | `runtime/frame/balloon_offset_follow.rs` | 要る |
| `on_char_drag_end` | `input_events/drag.rs`・`mod.rs` | 要る |
| `ClampScene`・`place_chain_in_work_area` | `runtime/frame/drain_resnap.rs` | 要る |
| `apply_balloon_shift`・`apply_char_shift`・`shift_is_current` | `runtime/frame/move_shift.rs` | 要る |
| `rebase_shift_axes` | `runtime/move_cue_place.rs` | 要る |
| `BalloonFollowTrigger`・`follow_balloon` | `runtime/frame/balloon_offset_follow.rs`・`dpi.rs` | 要る |
| `CharDragSession`・`begin_char_drag` | `app_exit.rs`・`input_events/drag.rs` | 要る |
| `ChainPlacement`・`ChainClampOutcome` | 0（値は `drain_resnap.rs` が受けるが名で呼ばない・再公開は `pub(crate)`） | 要る（公開の関数の戻り）。名で呼べるように `pub use` を足すのは約束の内 |
| `WORK_AREA_CLAMP_TAG`・`on_balloon_drag`・`sync_drag_writer` | 0（本体のテストだけ） | 本番の公開としては要らない。狭めると本体のテストが壊れる＝`test-support` の feature へ移す案（C20 で止まる） |

- 判断: 要らない公開を狭める直しは、どれも要件 7.1 に当たる＝**C20 のうちには直せない**（議題 8）。推す＝C20 の後へ申し送る（`runtime-crate-split` が判定器を `areka-runtime` へ移すとき、テストだけの公開の扱いが変わるので、そのときにまとめる）。

### 旧い名前 `emo2_boot`・`Emo2Wiring`（要件 2.8・指摘 22）

- 受け持ちの 9 ファイルに `emo2_boot` が 21 か所（`follow.rs` 7・`follow/window_move.rs` 3・`follow/work_area.rs` 2・`follow/move_shift.rs` 1・`follow/offset_space.rs` 1・`follow_transition_diag_tests.rs` 2・`follow_visibility_balloon_wiring_tests.rs` 2・`follow_balloon_limit_tests.rs` 1・`follow_default_pos_track_tests.rs` 1）。`Emo2Wiring` は 0 か所（本体の型は今 `RuntimeWiring`＝`runtime/frame/wiring.rs:59`）。`emo2` の名のうち検体のゴースト・バルーンの名（`検体 emo2` ほか）は古い名ではないので数えない。
- 判断: **直す**（推す）。説明の中のパスだけで、`runtime::…` の今の場所が全部分かっている（下請けが 1 つずつ引いた）。直したら `runtime-crate-split` の brief へ「受け持ちの 9 ファイルの `emo2_boot` は直した」と申し送る（要件 2.9）。

## 5. 要件と手持ちの対応（Requirement-to-Asset Map）

| 要件 | 手持ち | ギャップ |
|---|---|---|
| 1 洗い出し | 本文書（28 件・全 56 ファイル読了） | なし（要件 1.5 の書き足しは討議の後） |
| 2 申し送りと古い名前 | 4 節（⑴〜⑶・36 個の属性を `cargo check` で確かめた・旧い名前 21 か所） | **Constraint**: ⑶ の狭める直しは C20 の約束（議題 8）・`follow.rs:108`／`window_move.rs:449` は要件 2.3 と 2.5 の読み方（議題 5） |
| 3 全部直す | 1〜4・7・9〜20・22〜27 は範囲の中で閉じる | **Constraint**: 5・6・21 は裁定に当たる・8 は先送りの印・28 は C20 の約束 |
| 4 残す裁定 | 議題の候補は 5・6・8・21・28 と ⑵ の 2 個 | **Unknown**: 5 は ukadoc の `move` の項を引いて種類 3 に当たるかを確かめる（11 節） |
| 5 直した後 | 全体テスト `tools/test-all.ps1`・`log-capture-kit`・`temp-path-kit`・`drain_window_pos_commands`・判定器の部品（決定論の土台は揃っている） | **Missing**: 1〜4・7・11・12・13・14 の「壊れたら落ちるテスト」は今 0 本 |
| 6 範囲の外 | 呼び手を grep で確かめ済み（各件の欄） | **Constraint**: 2 は `chain_realign.rs` の結果が変わる・17 の案 甲は `Cargo.toml` |
| 7 C20 の約束 | `areka-placement` の公開の署名を変える直しは 0 件（8・23・28 の「減らす」案だけが当たる）。wintf に触る直しは 0 件。依存の追加は 0 件・`Cargo.lock` は変わらない見込み | **Constraint**: 8・23（`DragPositionPolicy`）・28 は要件 7.1 |
| 8 守る決まり | `window_move.rs` 1,231（例外表）・`drag_follow.rs` 978・`transition_judge.rs` 999・`transition_judge_tests.rs` 1,039（例外表）・`transition_judge_verdict_tests.rs` 1,037（例外表） | 2・8 で `window_move.rs` は減る（20 の 3 行はその中）。3・7 で `drag_follow.rs` が増える＝足す分を兄弟のファイルへ出すか同じファイルの重複を減らす。19 で `transition_judge_tests.rs` は 998 行＝例外表から外す（要件 8.3） |
| 9 大きさ | — | **Constraint**: 見込み 24〜27 タスク（9 節） |

## 6. 直し方の選択肢

- **案 A（今のファイルの中で直す）**: 全件を今のファイルの中で直す。新しいファイルは、例外表のファイルや上限に近いファイルへ足せないテスト（11・14 の一部）の兄弟のファイルだけ。テストの部品は今の `follow_test_support.rs`・`transition_judge_test_support.rs` へ上げる。
  - ✅ 構造を変えない・公開の形は変わらない（`ChainPlacement` の再公開を足すだけ）。❌ `drag_follow.rs`（978）に 3・7 を入れると 1,000 行に当たる見込み＝同じファイルの中の重複（`on_balloon_drag` と終わりの出し直しの塊）を 1 つにして行を作る必要がある。
- **案 B（随伴の書き手を 1 つにまとめてから直す）**: 2・3・20 の「随伴とバルーンの書き方」を `follow_balloon` 1 本に集め（`move_window_with_route` の写し・終わりの出し直し・未確定の印の確かめ）、その上で 1・4 を直す。
  - ✅ 同じ決まりを 2 か所で当てる型（2・20）が根から消える。❌ 差分が大きい・`chain_realign.rs` の結果の変化を核のセッションと確かめる手間が先に来る。
- **案 C（混ぜる）**: 必ず直すの 1〜4 は案 A で先に入れる（2 は `follow_balloon` を呼ぶだけ＝案 B の芯）。テストの整理（15〜19）は部品を上げる（16）のを先にしてから。裁定待ち（5・6・8・21・28）は討議の後に案 A で。

## 7. 手間と危うさ

- **手間**: L（1〜2 週）。直しの 1 件ずつは小さいが、テストの整理（11・14〜19）の行数が多く、判定器の 4 ファイルとそのテスト 11 ファイルにも及ぶ。
- **危うさ**: Medium。1・3 は記録と書込の順に触る。2 は核の連鎖の解き直しの結果が画面の外の形で変わる。12・13 は判定器を厳しくするので、今までの実機のログ（完了した spec の検証の記録）で赤にならないかを確かめる要がある。テストの整理は決定論のテストだけで、本番の振る舞いは変えない。

## 8. 設計の段への申し送り

- 推す並び: テストの部品を上げる（16）→ 必ず直す（1〜4）→ 直したほうがよい（7・9〜15・17・18）→ 裁定の後の 5・6・8・21 → できれば（19〜27）→ 28 の足すだけの再公開。部品を先に上げると、テストを足す・消す手間が 1 度で済む。
- 決めること: 1 の「当て直し待ち」の持ち方（記録の欄を足すか、記録を外すか）・3 の出し直しの関数の置き場（`drag_follow.rs` の行の上限）・7 の `Err` を返せないハンドラの記録の水準・13 ⑴ を scope ごとにするか・17 の直し方・22 の行の番号を名に替える方針。
- 1,000 行: `window_move.rs`（例外表）は 2 と 8（案 甲なら）で減る分の中で 20 を入れる。`drag_follow.rs` は 3・7 で増える分を、同じファイルの重複の削り（3 の関数への切り出しで `on_balloon_drag` の塊が 1 つになる）で埋める。`transition_judge.rs`（999）は 22 の説明の直しで行を増やさない。`transition_judge_tests.rs` は 19 で 998 行になったら例外表と件数の定数から外す。

## 9. タスクの数の見込みと切り方（要件 9.1）

| 組 | 指摘 | タスクの見込み |
|---|---|---|
| 必ず直す | 1〜4 | 4（1 件ずつ・テストつき） |
| 直したほうがよい | 5〜18 | 13〜15（5・6 は裁定しだいで 0〜1 ずつ・7・8・9・10 は 1 ずつ（10 は 22 に入れてもよい）・11 は 2・12＋13 は 2・14 は 1・15 は 2・16 は 2・17 は 1・18 は 15 に入れる） |
| できれば | 19〜28 | 5〜6（19 は 1・20 は 1・21 は裁定しだいで 0〜1・22 は 1・23 は 1・24〜27 で 1・28 は足すだけなら 23 に入れる） |
| 確かめ | — | 2（audit のかけ直し・全体テスト） |
| 計 | 28 件 | **24〜27** |

20 を越える見込みなので、要件 9.1 のとおり切り方を出す。

- **切り方 甲（推す）**: 本 spec＝必ず直す＋直したほうがよい（1〜18・確かめ 2 を入れて約 19〜21）。2 本目（同じ範囲・例 `audit-placement-follow-2`）＝できれば（19〜28・確かめ 2 を入れて約 7〜8）。2 本目は本 spec の着地を待つ（テストの部品を共有する）。ただし判定器の 4 ファイルとそのテストは C21 の `runtime-crate-split` が `areka-runtime` へ移す＝2 本目のうち判定器に触る分（19 の一部・22 の一部・23 の一部・27）は `runtime-crate-split` より前に着地するか、移した後のパスで直すかを決める。
- **切り方 乙**: 本 spec＝本番のコードの直し（1〜10・20・21・23・28 と 22 の本番の説明）。2 本目＝テストと判定器の整理（11〜19・22 のテストの説明・24〜27）。判定器の整理は `runtime-crate-split` の前にまとめて着地させやすい。テストの整理が本 spec の新しいテストの後になるので、部品の写しを一度増やしてから消す。
- **切らない**: 5・6・21 を「今のまま」、28 を C20 の後へ回す裁定なら、見込みは 22〜24。それでも 20 を越える。

## 10. 並走の兄弟（ウェーブ C20）に及ぶ所

- `audit-placement-core`（同じ crate の核）: 署名を変える直しは 0 件。2 の直しで `chain_realign.rs:351` の呼び出しの結果（バルーンが画面の外へ出る形だけ）が変わる＝相手のセッションへ知らせ、核のテスト（`dpi_sync_tests.rs:332-340`）が赤にならないかを確かめる。17 の案 甲は `crates/areka-placement/Cargo.toml` に 1 語足す＝申し合わせる。16・19・22 の範囲の外の写しと古い参照（`diag_tests.rs`・`persist_*`・`reset_pos`・`balloon_lock_drag_tests`・`reseed_test_support.rs`・`resolver_test_support.rs`）は相手の持ち物として残し、相手へ伝える。1 は核の公開の `defers_window_write` を呼ぶだけ。
- `audit-wintf-window`（wintf の型）: wintf に触る直しは 0 件。⑴ の `MonitorSnapshot` は寄せない（4 節）。11 ⑴ は wintf の公開の `drain_window_pos_commands` を読むだけ。
- `audit-wintf-input`（wintf のドラッグの型・進行役から 2026-11-14 に伝言）: wintf の側で 2 つの直しを予定していて、受け持ちのファイルに届く。実装の前に改めて申し合わせる約束。
  - ⑴ `DraggingState.initial_inset` を `initial_window_pos` へ改名する。受け持ちの読み手＝`follow/drag_follow.rs:271`（説明）・`:294-295`（`ds.initial_inset.0`・`.1`）と `follow_test_support.rs:90`（説明）・`:95`（組み立て）。
  - ⑵ ドラッグの通知の `is_primary` の欄（いつも真・誰も読まない）を消す。受け持ちの書き手＝`follow_test_support.rs:51,105,116` の `is_primary: true`（ほかの出現は `audit-placement-core` の持ち物）。
  - この文書の指摘は、どちらの欄にも別の直し（別の改名・欄の足し引き）を推さない。指摘 3 の出し直しの関数と指摘 16 の部品の上げは `drag_follow.rs`・`follow_test_support.rs` の同じ所に触りうるので、着手の順（どちらが先に着地するか）を申し合わせ、後の側が今の名で rebase する。
- `wintf-winmsg-executor-import`（`Cargo.lock` の席）: 依存を足す直しは 0 件。17 の案 甲も `Cargo.lock` は変わらない見込み（11 節で確かめる）。
- 本体 `crates/areka/`（`runtime/`・`input_events/`・`menu`）: 公開の形を変える直しは 0 件（8・23・28 の「減らす」案は議題）。1・2 は本体の呼び手を変えない。判定器の署名を変える直しは 0 件＝`runtime/frame_transition_atomicity_tests.rs`・`frame_transition_branch_tests.rs` は触らない（古い行の参照は残して申し送る）。
- `runtime-crate-split`（C21）: 旧い名前を直したら brief へ申し送る（要件 2.9）。判定器のファイルを移す前に本 spec が着地する（9 節の切り方 甲の注意）。

## 11. 調べが要る所（Research Needed）

1. 指摘 3: 離したフレームの最後の動きが本当に `DragEvent` として配られないか。決定論のテストで wintf の `DragAccumulator`・`dispatch_drag_events` を通して確かめる（できなければ実機で 1 回）。
2. 指摘 2: `follow_balloon` へ替えたとき、`dpi_sync_tests.rs:332-340`・`follow_window_move_hold_watch_tests.rs` ほか `move_window_with_route` を通すテストが、守りの記録や位置で赤にならないか。
3. 指摘 5: ukadoc の `\![move]`・面の `move` の項に、作業領域の端の扱いの記述があるか（種類 3 に当たるかの確かめ・要件 4.2）。無ければ種類 1 だけの議題。
4. 指摘 13 ⑴: 完了した `areka-P0-balloon-offset-dpi` の要件 8.5 が scope ごとの確かめを求めているか。あわせて、厳しくした判定器で完了した spec の実機のログ（検証の記録に残る分）が赤にならないか。
5. 指摘 17 の案 甲: `crates/areka-placement/Cargo.toml` の `log-capture-kit` に `env-filter` を開けても `Cargo.lock` が 1 バイトも変わらないか（`cargo metadata --locked` で確かめる）。
6. 指摘 21: `finish_persists_partner_position_without_its_pattern_shift` を置いた spec（`seriko-pattern-calls`）の要件・設計で、組の相手の扱いが開発者の決めか。

## 12. 議題（要件の討議へ）

1. **切り方（要件 9）**: 推す＝切り方 甲（本 spec＝必ず直す＋直したほうがよい・2 本目＝できれば）。判定器に触る 2 本目の分は `runtime-crate-split` より前に着地させる。
2. **面の `move` のずれで利用者の置いた窓が寄せられて戻らない（指摘 5・種類 1）**: 推す＝案 甲（寄せる先を「作業領域と元の矩形を合わせた範囲」にする）。理由＝`seriko-pattern-calls` 要件 7.8 の「今ある置き方の決まり」は、ほかの所では利用者が置いた窓を寄せない。ずれで外へ出さない決まりは守れる。
3. **`ClampX` の後も見えない窓（指摘 6・種類 1）**: 推す＝案 甲（位置は今のまま、別の語で記録する）。理由＝D6（Y は射影の所有）を崩さず、「引き戻した」の嘘だけを消せる。
4. **`anchor_changed_system`（指摘 8・種類 2・C20）**: 推す＝案 乙（C20 のうちは残し、属性を外して説明に「本番は `reseed` から書く」を書く・消すのは C20 の後）。理由＝公開の項目を減らす直しが C20 の約束に当たる。開発者が「結線の予定は無い」と決めれば C20 の後に消す。
5. **brief が「古くない」とした `follow.rs:108`・`window_move.rs:449` の属性（要件 2.3 と 2.5）**: 推す＝消す。理由＝理由の文は事実だが、属性そのものは lib の公開の項目なので働かない（`cargo check` で確かめた）。要件 2.5 は数えないと言い、2.3 は要らなければ消すと言う＝2.3 を優先する。
6. **ログを捕まえる手段の写し（指摘 17）**: 推す＝案 甲（`log-capture-kit` の `env-filter` を開けて写しを消す・`Cargo.lock` が変わらないことを確かめてから・`audit-placement-core` と申し合わせる）。`Cargo.lock` が変わるなら案 乙。
7. **組のドラッグの終わりのずれの扱い（指摘 21）**: 推す＝11 節の 6 を確かめてから決める。決めごとでなければ「相手も当てた分を 0 にする」に揃える（動かした窓と同じ）。
8. **要らない公開（指摘 28・申し送り ⑶・C20）**: 推す＝狭める直しは C20 の後へ申し送る（`runtime-crate-split` の後に、テストだけの公開を `test-support` の feature へ移す）。`ChainPlacement`・`ChainClampOutcome` の `pub use` は足すだけなので本 spec で入れる。
9. **申し送り ⑴（`MonitorSnapshot`）**: 推す＝理由が今も事実なので残す（指摘に上がらず・寄せるなら wintf の型と 50 ファイル）。
10. **旧い名前 `emo2_boot`（要件 2.8）**: 推す＝直して、`runtime-crate-split` の brief へ申し送る。

> 2026-11-14 要件の討議の結果は `requirements.md` の「確定した直す対象」が正本。上の推奨から変えたのは 4 つ＝議題 1 は切らない（`audit-dola` の前例）・議題 4 は system を残す（範囲の外の 5 か所の説明が見本として名指す）・議題 7 は残す（`seriko-pattern-calls` の設計の決めどおり）・議題 8 は使い手 0 の公開を C20 のうちに狭め、テストだけが使う公開を `runtime-crate-split` へ申し送る。

> 2026-11-14 並走の申し合わせ（設計の段へ）: `audit-placement-core` とは、指摘 2 で相手のテスト（`dpi_sync_tests.rs` ほか）が赤になったら後から着地する側が直す・`Cargo.toml` の `env-filter`（指摘 17）は触ってよい・相手は `dpi_sync` の `DPI_SYNC_HOLD_MAX_FRAMES` ほかテストだけが使う公開を `cfg(any(test, feature = "test-support"))` の内側へ移す案（判定器のテストは本体の `test-support` で届く）。`audit-tools-perf-judge` とは、`transition_judge.rs` の `judge-perf.py` の行番号の参照（22・403 行）をこちらが指摘 22 で「ファイル名と関数名」の形に直す。
