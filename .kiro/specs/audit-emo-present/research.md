# ギャップ分析: audit-emo-present

> 2026-11-05・作業場の先頭 `2c65abb3`（main `88eb0a43` ＋ spec 初期化のコミット）。物差しは ponytail 5.3.2 の `/ponytail:ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.2/skills/ponytail-audit/SKILL.md`）。範囲は `crates/areka-emo-present/` だけ。要件 1.2・1.3 に従い、audit の作法のうち「最大 20 件」と「読まなかった所（Not checked）」の 2 つを外した。
>
> この文書は分析と選択肢だけを書く。直す対象・直さずに残す候補・直し方の決めは、要件の討議と設計で行う。

## 1. 読んだ範囲と読み方

- **読んだファイル**: git が持つ 91 ファイル（Rust 90・`Cargo.toml` 1）を全部、1 行も飛ばさずに読んだ。計 32,436 行。
  - 本番 26 ファイル（7,300 行）と `Cargo.toml` は、このギャップ分析の担当が自分で全部読んだ。
  - テスト 64 ファイル（25,092 行）は、ファイルの組ごとに 5 つの下請けへ分けて全部読ませた（各組は自分の組の全ファイルを全行読んだと、行数つきで返した）。結果は担当が突き合わせ、同じ指摘をまとめ、番号を振り直した。
    - 組 1: `cache_tests.rs`・`balloon_*_tests.rs`・`balloon_test_support.rs`・`self_alpha_tests.rs`・`command_device_err_tests.rs`
    - 組 2: `shell_target_*`（テスト 12・支え 1）・`display_*_tests.rs`・`mount_*`・`scale_tests.rs`
    - 組 3: `presenter/*_tests.rs`（5）・`presenter_test_support.rs`・`presenter_{nesting_hit,scale_change,stage,keycolor_clickthrough,partial_recompose_gpu,blink_stop,cache_capacity,stage_generation,failure_note,resize_report,hide_contract}_tests.rs`
    - 組 4: `presenter_{frame_log,partial_recompose,replace,read_accessor,compose_input,budget_steady_state,budget_equivalence,display_failure,fractional_scale}_tests.rs`
    - 組 5: `presenter_{film,visibility,refresh_and_log,display,dpi_scale,perf_log}_tests.rs`
- **確かめたこと**: 「使われていない」と書く前に、`crates/` 全体（テスト・実行例・文字列での参照を含む）を grep した。範囲の外の呼び出し元の数もここで数えた。
- **回していないもの**: テストは回していない（このフェーズは読むだけ）。指摘はどれも読みから組んだ具体的な場面に基づく。赤で確かめるのは実装の段である（要件 4.1）。
- **想定した負荷**: 1 人の利用者のデスクトップ。ゴースト 1 体あたり提示の対象（シェル・バルーン）が 2〜4 つ、動く面ではコマ送りの `ShowSurface` が対象ごとに毎秒数十回届く。読み込み（シェル・バルーン）は起動と切り替えのときだけ。

## 2. このクレートがすること（audit の冒頭の 2〜3 文）

emo の提示段（三段直列の 3 段目）。上流が合成した原寸の絵を、wintf の WUC の表示面に載せる命令として記録し、当たり判定のマスクとともに UI スレッドで差し替える（`presenter/show.rs` の `apply_show` が単一の漏斗）。あわせて、シェルのフォルダとバルーンのフォルダを読む権威（`shell_target.rs`・`balloon.rs`・`balloon_definitions.rs`・`balloon_marker.rs`）と、拡大率 k の政策（`scale.rs`）を持つ。

## 3. 指摘の一覧（audit の番号のまま・全 43 件・省いた件なし）

書き方: 番号・組・場所（`crates/areka-emo-present/src/` からの相対パス:行）・何が問題か・壊れる場面・いちばん小さな完全な直しの案（選択肢・決めではない）・範囲の外に及ぶか。**例外の札**は、要件 3 の 3 種類のどれかに当たる**見込み**があるものにだけ付けた（⑴ 開発者の裁定に反する直し／⑵ わざと置いた先送りの印／⑶ SSP との互換・ukadoc の正典に関わる振る舞いの変更）。札は候補の印で、決めは開発者が行う。⑶ に当たる見込みの指摘は今回 0 件だった（どの直しも外から見た表示・当たり・正典の解釈を変えない）。

### 3.1 必ず直す（Must fix）— 3 件

**1. バルーンのテストの一時フォルダが `%TEMP%` に作られる**（`balloon_test_support.rs:9-51`）
- 問題: 共有の `TempDir` が `std::env::temp_dir()` の下（`%TEMP%\areka-emo-present-balloon-<pid>-<n>`）にフォルダを作る。実機の根・一時フォルダはワークツリーの `target\` の下だけ、という絶対の決まりに反する。使うのは `balloon_alpha_tests`・`balloon_quiet_tests`・`balloon_model_tests`・`balloon_target_tests`・`balloon_series_tests` の 5 つ。
- 場面: テストが強制終了されると `Drop` が走らず、`%TEMP%` にフォルダが残る。`cargo clean` でもワークツリーを消しても消えない。
- 直しの案: `TempDir` の中身を `temp_path_kit::TempPath::new("emo-present-balloon")` に替え、`touch`／`write` は `child()` 経由にする（`temp-path-kit` は dev 依存に既にある＝依存は増えない）。
- 範囲の外: **及ぶ**。`crates/log-capture-kit/tests/temp_path_guard_test.rs:156` の例外表にこのファイルが「プロセス識別子で一意化済み」として載っている。直すと表の項目が実在しなくなり、`every_exception_still_has_a_real_hit` が赤になるので、表の 1 項目を消す必要がある（`log-capture-kit` は `audit-test-kits` の範囲・要件 7.2）。
- 例外の札: ⑴ の候補（弱）。表は完了した spec（`areka-P0-test-cage-determinism` 要件 12.2）が「移行の対象外」と決めた結果だが、それは「その spec では触らない」という範囲の決めで、`%TEMP%` に置き続けよという裁定ではない。

**2. 「未装着」でない失敗が `TargetNotAttached` として返る**（`presenter/visibility.rs:86-92, 130-138, 156-163`・`presenter/read.rs:249-266`・`presenter/hub.rs:232-240`）
- 問題: `PresentError` には `Compose`・`TargetNotAttached`・`Device` の 3 つしか無い。そのため「一度も表示していない（`last_show` が無い）」「表示中の絵が合成メモから追い出された」「確立済みなのに装着が無い」「応答が失われた」がどれも `TargetNotAttached(target)` になる。
- 場面: バルーンを一度も表示していないうちに `show_target` が呼ばれると、本番の `crates/areka/src/emo2_boot/balloon_visibility_phase.rs:620-625` が `error = %error` で「target TargetId(1) is not attached」と記録する。装着は済んでいるので、記録が原因を取り違える。
- 直しの案: (a) `PresentError` に変位を足す（例 `NotShown(TargetId)`・`EntryEvicted{target, surface_id}`）。`#[non_exhaustive]` なので外の `match` は壊れない。(b) 応答の失われる 2 か所は 4 番の直しで消える。
- 範囲の外: 型を足すだけなら及ばない（外で `TargetNotAttached` を作るのは `crates/areka/src/emo2_boot/frame/switch_finish_tests.rs:335` だけ・意味は変わらない）。

**3. バルーンの読み込みの失敗が全部「全透明の合成」として返る**（`balloon.rs:329-336, 415-424, 732-738, 756-765`）
- 問題: フォルダが読めない・面 0 が無い・面が空・焼く段で絵が落ちた、の 4 つがどれも `PresentError::Compose(ComposeError::EmptyComposition(0))` になる。`EmptyComposition` は本来「合成の結果が全部透明」で、`apply_show` では Hide に縮退する正常な退化の意味を持つ。
- 場面: バルーンのフォルダが読めないと、起動の資産組み（`crates/areka/src/emo2_boot/assets.rs`）が `BootWiringError::Balloon` に包んで「compose failed: …empty…」と伝える。真の原因は直前の `error!` に出ているが、戻り値だけを見る呼び手はフォルダの読みの失敗と面の無いバルーンを区別できない。
- 直しの案: (a) バルーンの読み込み専用の失敗型を作る（シェルの `ShellLoadError` と同じ形・`List`／`NoFace0`／`Empty`／`Bake`）。(b) `PresentError` に読み込みの変位を足す。どちらも外の呼び手は `?` で包み直すだけ。
- 範囲の外: **及ぶ**。`crates/areka/src/emo2_boot/assets.rs:470-486`（`EmptyComposition(0)` を自分で作って返す 1 か所）と、戻り値の型を名指すテスト・実行例（`build_balloon_target` を使う外のファイル 6）。
- 例外の札: ⑴ の候補。モジュール冒頭（`balloon.rs:24-27`）が「EmptyComposition は下流で Hide 縮退として許容されるため、バルーン構築失敗はゴーストごと殺さず穏当に縮退する」と書いている（完了した spec の設計の討議 #1 を引く）。ただし、その討議の対象は `apply_show` の全透明の扱いで、読み込みの失敗をそれに畳めという決めかどうかは読みからは分からない（**要確認**）。

### 3.2 直したほうがよい（Should fix）— 29 件

**4. `apply_show` が結果をクレートの中の応答の口で返し、呼び手が口を作って読み直している**（`presenter/hub.rs:221-246`・`presenter/visibility.rs:107-139`・`presenter/refresh.rs:121-146`・`presenter/replace.rs:122-134`）
- 問題: `apply_show` は戻り値を持たず `Option<ReplySender>` へ送る。クレートの中の呼び手 3 つ（`show_or_hold`・`show_target`・差し替え）はそのためにチャンネルを作って受け直し、届かなかったときの枝（到達しない）を持つ。`refresh_scale` は口を使わず、状態（`applied`）の後追いで成否を推し量る。同じファイルの `show_target` の doc 自身が「状態の後追いで成否を組み立てると誤読する」と書いている。
- 場面: 今は誤読しない（k が変わった時だけ再表示するため）。だが成否の読み方が 2 通りあり、片方を変えるともう片方が黙ってずれる。届かない枝は 2 番の取り違えた失敗を生んでいる。
- 直しの案: `apply_show` が `PresentOutcome` を返し、`apply` の入口だけが `reply` へ送る。`show_or_hold`・`show_target`・`refresh_scale` は戻り値を読む。チャンネルと届かない枝が消える。
- 範囲の外: 及ばない（`apply_show` は `pub(super)`）。

**5. 毎コマの表示で可視と当たりの部品が「変わった」印を立て、wintf が毎コマ COM を呼ぶ**（`mount.rs:133-155`・`presenter/show.rs:403-406`）
- 問題: シェル（命令で見える対象）の `apply_show` は成功のたびに `mount.set_visible(world, true)` を呼ぶ。`apply_visibility` は今の値と比べずに `Visual` を書き換え（`set_visible` は代入だけ）、`*ht = mode` で `HitTest` も書く。bevy はどちらも「変わった」と数える。同じファイルの `set_layout`・`set_display` は「同値なら書かない」を守っているのに、ここだけ守っていない。
- 場面: 動く面のコマ送りのたびに、面と文字層の 2 つの entity で `Changed<Visual>` が立つ。wintf の `visual_property_sync_system` が `SetOffset`・`SetOpacity` を、`clip_sync_system` がクリップの組み直しを、毎コマ 2 回ずつ COM へ投げる。
- 直しの案: 書く前に比べる（`Visual` は `is_visible` を比べてから `set_visible`、`HitTest` は `set_if_neq`）。直しを守るテストは「見えている対象へ同じ面を 2 回出しても 2 回目は `Visual`／`HitTest` の変更の印が立たない」。
- 範囲の外: 及ばない。

**6. `attach_target` が使わない引数を受け、失敗しないのに `Result` を返す**（`presenter/hub.rs:59-101`・`presenter/replace.rs:95-106`）
- 問題: `_world: &mut World` は読まれず、戻り値は常に `Ok(())`。そのため `apply_replace` の「登録し直せなかった」枝（`error!` と `Err` の返信）は到達しない。
- 場面: 到達しない失敗の枝が「置き換えは失敗し得る」と読み手に思わせる。テストで覆えない。
- 直しの案: (a) 引数と `Result` を外す（呼び手は範囲の外に 17 ファイル）。(b) 署名は残し、`replace.rs` の届かない枝だけを消す（範囲の外に及ばない）。
- 範囲の外: (a) なら**及ぶ**——`crates/areka/src/emo2_boot/`（6）・`crates/areka/src/emo2_boot/frame/attach.rs:299,372`・`crates/areka/examples/`（3）・`crates/areka-emo-text/tests/`（3）・`crates/areka-emo-text/examples/`（2）・`crates/pilot/examples/pilot-balloon-asset-swap/`（1）。emo-text は既存ファイルを直すだけ（ファイルは足さない・要件 7.4）。

**7. 外に出している口が、外で使われていない**（`lib.rs:76, 79-83`・`presenter.rs:92`・`presenter/replace.rs:18, 40`・`balloon_marker.rs:33-115, 252-309`・`balloon.rs:74, 280`）
- 問題: 範囲の外に使い手が 0 の公開の項目がある——`build_shell_target`・`build_shell_target_with_boxes`・`select_surface_images`・`SurfaceImageSelection`・`CacheEntry`・`ComposeCache`・`DetachedState`・`detach_target`・`marker_chain`・`MarkerChain`・`MarkerNames`・`MarkerName`・`select_marker_names`・`select_single_marker_name`・`resolve_single_marker`・`SingleMarker`・`select_faces`・`BALLOON_FAMILY`（`crates/` 全体を grep・使い手はこのクレートの中だけ）。
- 場面: 公開の面が広いほど、変えるときに「外で使われているかも」を毎回確かめる手間が要り、6 本の v0.2.0 の spec がこの面を前提に設計しかねない。
- 直しの案: `pub(crate)` へ狭め、`lib.rs` の再輸出を消す。
- 範囲の外: 型の上では及ばない。`crates/areka-emo-compose/src/nesting_fixture_tests.rs:4` のコメントが `build_shell_target_with_boxes` を名指す（`audit-emo-compose` の範囲・C17 で並んで走る）。直すならそのセッションと申し合わせる（要件 7.2）か、名指しをそのまま残す。

**8. テストだけが読む項目を `#[allow(dead_code)]` で本番に残している**（`presenter/budget.rs:150-151, 172-178, 206-212, 534-537`・`presenter/timing.rs:55-56`）
- 問題: `AllocSite::ALL`・`BudgetDelta::count`・`BudgetCounters::count`・`FrameBudget::cumulative`・`PERF_LINE_MESSAGE` は読み手がテストだけで、本番では死んでいる。抑止の指示で警告を消している。
- 場面: 本番の読み手が消えても、新しく生まれても、警告で気づけない。
- 直しの案: `#[cfg(test)]` に替える（`presenter/budget_tests.rs`・`presenter_budget_steady_state_tests.rs`・`presenter/timing_tests.rs` はどれも子か同じクレートのテストなので届く）。
- 範囲の外: 及ばない。
- 例外の札: ⑵ の候補。どれも「読み手はテストだけ・死蔵ではない」という理由の doc が付いている。ただし語彙の口（配線ゼロの追跡）ではなくテストの観測口なので、⑵ の「語彙の口」に当たるかは開発者の読み次第。

**9. 合成メモのテストのうち 4 本が、自分の数えた回数を確かめているだけ**（`cache_tests.rs:214-252, 460-526, 528-577, 611-649`）
- 問題: `compose_calls` はテスト自身の `if cache.get(..).is_none()` で増える。本番の合成の道は 1 本も通らない。554-568 では `assert!(hit.is_some())` の直後の `if hit.is_none()` が死んでおり、続く `compose_calls == 1` は落ちようがない。
- 場面: `presenter/show.rs` の「外れたら合成し直す」を壊しても、この 4 本は緑のまま。
- 直しの案: 4 本を消す（同じ主張は `presenter_cache_capacity_tests.rs` ほかの `apply_show` を通るテストが本物で持つ）。
- 範囲の外: 及ばない。

**10. `cache_tests.rs` が 1,000 行の番人を越えている（1,301 行）**（`cache_tests.rs` 全体）
- 問題: 番人の例外表に載っているので赤にはならないが、上限を越えたままである。9 番の 4 本と、ほかのテストの部分集合の 3 本（432-458 は 743-771 の部分集合・579-609 は 1027-1038 と 1045-1062 で覆われる・1069-1103 は 950-1017 と同じ手順）を消すと約 330 行減り、1,000 行を下回る見込み。
- 場面: 例外表に載ったままだと、後でテストを足す人が上限を気にせず足し続ける。
- 直しの案: (a) 消して 1,000 行を切り、例外表から外す。(b) 消すだけで表は触らない（表の項目が実在のまま残るので番人は緑）。(c) 足りなければ「容量 3・LRU」の節（1019-1301）を兄弟ファイルへ分ける。要件 6.2 の「行を足さない」は消すだけなので守れる。
- 範囲の外: (a) なら**及ぶ**——`crates/log-capture-kit/tests/file_length_guard_test.rs:66-69`（項目）・`:101`（件数 9 → 8）・`:104`（自己検査が外して試す 1 件 `CALIBRATION_DROPPED_ENTRY` を別の項目へ替える）。`audit-test-kits` の範囲で、要件 7.5 の申し合わせが要る（`audit-test-kits` は C17 に居ない＝相手のセッションは今は無い）。

**11. 合成メモの容量 3 がテストの 3 か所に手で写してある**（`presenter_budget_equivalence_tests.rs:72, 533-536`・`presenter_budget_steady_state_tests.rs:131`・`presenter_cache_capacity_tests.rs:49`）
- 問題: `cache.rs:134` の `CAPACITY` は私有で、テストは `CACHE_CAPACITY = 3` を写して持つ。`ROTATING_FACES > CACHE_CAPACITY` の確かめは同じ写しどうしの比較なので常に真。
- 場面: 容量が 4 になって写しが 3 のままだと、「回す面が容量を越えていない」を言うはずの確かめは黙り、定常状態の暖機と等価の確かめが別の、紛らわしい文言で赤になる。
- 直しの案: `CAPACITY` を `pub(crate)` にして 3 つのファイルから参照し、533-536 の常に真の確かめを消す。
- 範囲の外: 及ばない。

**12. 落ちようがない確かめ**（下の場所の全部）
- 問題と場面（どれも「その行を消しても、あるいは本番を壊しても、その確かめは緑」）:
  - `presenter_display_failure_tests.rs:642`・`display_fault_tests.rs:375-381`・`presenter/budget_tests.rs:64-71`: 配列の型 `[_; 4]`／`[_; 2]` が長さを決めているので `len() == 4`／`== 2` はコンパイルの時点で真。新しい変位を足して配列に入れ忘れても緑。
  - `presenter/snapshot_tests.rs:348-392`（⑸）と `:456-494`（⑺ の状態の前後比較）: 照会は `&self` で、presenter に内部可変（`Cell`・`RefCell`・`Mutex`）は無い。「状態を変えない」は型が保証しており、確かめは落ちない。
  - `shell_target_base_image_tests.rs:324-339`: `used == [0,1,5]` と `shadowed == [2]` が通った後の重なり・和の確かめは定数。
  - `presenter_fractional_scale_tests.rs:138-141, 176-180, 282-286, 343-346, 652-655`: 直前の確かめが通れば必ず真。
  - `balloon_series_tests.rs:15-19`: `.all(..)` は結果が空でも真。
  - `command.rs:200-276` の 3 本（`variants_construct_with_owned_data`・`target_id_is_copy_and_hashable_key`・`present_error_maps_compose_error_via_from`）: 型の上で成り立つことを実行時に確かめているだけ（`present_command_is_send_static` はコンパイル時の檻として意味がある）。
- 直しの案: 消す。網羅の `match` で守れているものは `match` だけ残す。長さの番が要るなら、列挙の正本を列挙型の隣に置いて網羅の `match` から作る。
- 範囲の外: 及ばない。

**13. まばたきを止めるテストが、うたう「いちからの合成」を通らない**（`presenter_blink_stop_tests.rs:125-143`）
- 問題: ループの前の `show(.., PatternState::default())` が鍵 (1000, {1400}, 空) を合成メモに入れる。容量 3 の LRU なので、1 周目の「まばたきを外す」段も 2 周目の全段も引き当てで済み、まばたきの後の絵は一度も合成されない。モジュール doc（9 行目）は `compose_into` のいちからの道を守ると書いている。
- 場面: 空のコマでの `compose_into` が 1410／1411 のコマの重ねや席の画素を引きずっても、`read_back` は前に入れた絵を返し、テストは緑。
- 直しの案: 125 行目の事前の表示を消す（1 周目は 3 段とも外れ、2 周目が当たる＝105 行目の doc どおりになる）。
- 範囲の外: 及ばない。

**14. 合成メモの容量のテストの「負の対照」が対照になっていない**（`presenter_cache_capacity_tests.rs:231-237`）
- 問題: 鍵 99 が表示前に無いことしか見ていない。外れたときにマスクの番地が動くことを示さないので、224 行目の番地の一致が「落ち得る」ことを示せていない。
- 場面: マスクの輪番が毎回同じ `Arc` を渡す誤りでは、224 行目の一致が外れでも成り立ち、対照も通る。
- 直しの案: `show(.., 99)` の後に `mask_of(&presenter, 99) != before` を足す。
- 範囲の外: 及ばない。

**15. 隠す契約のテストの見張りが、書き換えを見ていない**（`presenter_hide_contract_tests.rs:271-300`・doc `:15-16`）
- 問題: 見張りは `Insert` だけを数える。`VisualMount::set_visible` は `Visual` をその場で書き換えるので `Insert` を出さない。doc は「同じフレームで一度見えてから消える往復を除く」と書くが、それは確かめられていない。
- 場面: `show.rs` が不可視で作ってから `set_visible(true)`→`set_visible(false)` を同じ呼び出しでしても緑（`areka-P0-balloon-visibility` 要件 1.2 を壊す形）。
- 直しの案: `show_ok` の前後で `Visual` の変更の印（change tick）を読む。できなければ doc を「挿入の値だけ」へ書き直す。
- 範囲の外: 及ばない。

**16. 原寸の追従のテストの 3 段目が、doc の言う誤りを捕まえられない**（`presenter_dpi_scale_tests.rs:840-842, 856`）
- 問題: doc は 3 段目を「外れたときだけ `native_size` を書く実装」を捕まえる引き当てだと書くが、3 段目は 2 段目と同じ面 3000 なので `native_size` は既に (4,3)。
- 場面: 外れたときだけ書く実装でも 3 段目は緑（捕まえるのは 773 行目の別のテストだけ）。
- 直しの案: 3 つ目の組と 840-842 の doc の段落を消す。
- 範囲の外: 及ばない。

**17. 表示の記録のテストが「行の幅は本物の stride」とうたうが確かめられない**（`display_tests.rs:3, 30-34`）
- 問題: `ComposedSurface::new` は常に `stride = w*4`（`crates/areka-emo-compose/src/composed.rs:30`）なので、`display.rs:57` を `pitch: w * 4` に変えても緑。
- 直しの案: doc と確かめの文言からそのうたいを外す（詰め物つきの面を作る口が無いので本物の確かめは組めない）。
- 範囲の外: 及ばない。

**18. 面の entity に当たりのマスクが無いときの `warn!` にテストが無い**（`presenter/show.rs:391-402`）
- 場面: この `warn!` を消すと、当たり判定が黙って前のマスクか矩形のままになり、どのテストも赤にならない（記録の無い失敗の道を禁じる決まり）。
- 直しの案: 1 回目の表示の後で面の entity から `AlphaMaskResource` を外し、もう一度出して `warn!` を 1 件捕まえるテストを 1 本。
- 範囲の外: 及ばない。

**19. 装着の `warn!` 4 つにテストが無い**（`mount.rs:143, 149-153, 223-226, 243-246, 291-293`）
- 場面: `apply_visibility`（`Visual`／`HitTest` が無い）・`set_layout`（`Arrangement` が無い）・`set_display`（entity が無い）・`despawn`（既に居ない）の `warn!` のどれを消しても緑。
- 直しの案: 面の entity を先に消してから `set_visible`／`set_layout`／`set_display`／`despawn` を呼び、それぞれ `warn!` 1 件を確かめるテストを 1 本。
- 範囲の外: 及ばない。

**20. バルーンの面 0 が無い・面が空のときの `error!` と代わりの値にテストが無い**（`balloon_definitions.rs:158-173`・`balloon.rs:732-738`）
- 場面: どちらかの `error!` を消すか、代わりの値を panic に変えても、クレート全体が緑（`crates/` を grep しても、この 2 つの文言を起こすテストが無い）。
- 直しの案: `load_scope_balloon_definitions(dir, 0, &[], ..)` と `build_balloon_target_from_faces(dir, &dec, &[], ..)` を呼び、`error!` 1 件と、代わりの値または `Err` を確かめるテストを 2 本。3 番の直しと同じタスクで扱える。
- 範囲の外: 及ばない。

**21. 印の絵のフォルダが読めないときの `warn!` にテストが無い**（`balloon_marker.rs:328-335`）
- 場面: この `warn!` を消すと、印が黙って出なくなり、どのテストも赤にならない。
- 直しの案: 無いフォルダを `resolve_marker_pair` に渡し、`warn!` 1 件と `MarkerPair::default()` を確かめるテストを 1 本。
- 範囲の外: 及ばない。

**22. `last_binds` を読むテストが無い**（`presenter/snapshot.rs:59-66`）
- 問題: 本番の読み手は `crates/areka/src/emo2_boot/shell_carry.rs:242`（ゴーストの読み直しで着せ替えを持ち越す）だけで、そのテストは presenter を通らない。
- 場面: `last_binds` が `current_surface_id` を見るように変わると、`\s[-1]` で隠したシェルを読み直したときに着せ替えが消え、何も赤にならない。
- 直しの案: `snapshot_tests` の ⑵ の `hide` の後に `last_binds(SHOWN) == Some(&bound())` と、`NEVER_SHOWN`・`UNREGISTERED` で `None` を足す。
- 範囲の外: 及ばない。

**23. 合成メモを捨てた後の `last_shown` の `Some((id, None))` にテストが無い**（`presenter/snapshot.rs:44-57`）
- 場面: 絵がメモに無いときに `None` を返すよう変わると、MCP の `dump_surface` が面の番号を失い、何も赤にならない。
- 直しの案: 表示 → `InvalidateCache` → `matches!(last_shown(SHOWN), Some((BASE, None)))` のテストを 1 本。
- 範囲の外: 及ばない。

**24. `take_world` の 1 回目のアトラスの装着をクレートの中で確かめていない**（`shell_target.rs:287-297`）
- 問題: クレートの中のテスト（`shell_target_shape_issues_tests.rs:54-67`）は面の番号しか比べない。
- 場面: 1 回目の `bind_atlas` を消しても、このクレートは緑（範囲の外の `crates/areka/src/offline_dump/offline_dump_tests.rs` が画素で捕まえる見込みだが、**未確認**）。
- 直しの案: `shape_issues_tests` で 1 回目の `take_world()` から面を 1 つ合成して空でないことを確かめる。
- 範囲の外: 及ばない。

**25. 下敷きの合成で、着せ替えか面を途中で変えたときの絵をテストしていない**（`presenter/show.rs:132-140`）
- 問題: `presenter_partial_recompose_tests.rs` はセルの x だけを変える。`partial_base` が真のまま着せ替え・面を変えて絵を比べるテストが無い。
- 場面: 矩形の差分がセルの帯の外に出る着せ替えの層を覆い損ねると、古い画素が残ったまま全テストが緑（正しさは emo-compose の命令の差分に頼っている）。
- 直しの案: `frame(0)`→`frame(2)`→同じコマで `BindSet::from_ids([n])`→`assert_picture` の 1 件を `partial_recompose_tests` に足す（`build_assets` に着せ替えのアニメが要る）。
- 範囲の外: 及ばない。**要調査**: emo-compose の `blit_rect` の範囲がどこまで覆っているか（読んでいない・`audit-emo-compose` の範囲）。

**26. テストが `C:\areka-test\...` という固定の外のパスを「無いはず」として使う**（`shell_target_load_tests.rs:195, 428`・`shell_target_quiet_tests.rs:220`）
- 場面: その機械にたまたまそのフォルダが在ると、一覧の失敗が起きず 3 本とも赤。ワークツリーの `target` の外を根として使っている。
- 直しの案: `TempPath::new("x").child("missing")` に替える（fs を触らない鍵としての `C:\areka-test` は無害なので残す）。
- 範囲の外: 及ばない。

**27. 矢印の族の写しをテストが自前で持つ**（`balloon_series_tests.rs:155-183`）
- 問題: `balloon_marker::ARROW_FAMILY` とバイト単位で同じ `ARROW_FAMILY` をテストが定義する。doc の「本仕様は吹き出し族のみ実装する」も古い（矢印の族は実装済み）。
- 場面: 本物の `ARROW_FAMILY` が変わっても、テストは写しを確かめて緑のまま。
- 直しの案: `crate::balloon_marker::ARROW_FAMILY` を使い、古い文を消す。
- 範囲の外: 及ばない。

**28. 他のテストの部分集合か写しのテスト**（消す候補・計 15 か所）
- `presenter_display_tests.rs:67-131`（`:196-323` の部分集合・`:194` の doc が自認）
- `presenter_dpi_scale_tests.rs:240-315`（`:317-400` とほぼ同じ）・`:432-455`（`:402-431` と同じ前提・同じ等式・`presenter_visibility_tests.rs:182-199` と `presenter_display_tests.rs:670-682` も同じ等式）
- `presenter_read_accessor_tests.rs:44-78`（`:111-116` と `:154-158` が同じ `Some(1000)` を見る）
- `presenter_refresh_and_log_tests.rs:850-855`（`:807-812` と同じ確かめ。`:840-841` のコメントは「陽性 1 件」だが数は 2）
- `presenter_budget_equivalence_tests.rs:268-292`（前提の確かめは `:532-563` がより強く持つ）
- `presenter_budget_steady_state_tests.rs:520-534`（引き当て 1 回のために GPU の World を 2 つ作り 12 回の外れを回し直す・遅い）
- `shell_target_load_tests.rs:116-147`（`shell_target_emo2_tests.rs:310-314, 373-387` と同じ・emo2 の全画像を実 WIC でもう 1 回読む・遅い）・`:192-233`（3 本とも `:427-440`・`:444-456`・`shell_target_image_only_tests.rs:273-301` で覆われる）
- `mount_tests.rs:78-127`（`mount_visibility_tests.rs:44-110` の部分集合）
- `display_gpu_tests.rs:268-285`（`:228-266` と `display_fault_tests.rs:292-303` で覆われる）
- `balloon_series_tests.rs:13-53`（`:226-264, 318-343, 348-374` の繰り返し）
- `balloon_target_tests.rs:62-122`（`:420-475` で全部覆われる）
- 場面: 契約が変わると 2〜3 か所を同じ形で直す羽目になる。2 本は毎回の走行を遅くする。
- 直しの案: 消す（`shell_target_load_tests` の一覧の失敗のパスの確かめだけは `:433` の `matches!` へ移す）。
- 範囲の外: 及ばない。

**29. presenter のテストの手伝いが多数のファイルに写されている**（重複）
- 表示して応答を確かめる手伝い（`presenter_test_support::show_ok` が着せ替えとコマを受けないため）: `presenter/transition_record_tests.rs:494-508`・`presenter_failure_note_tests.rs:44-58`・`presenter_cache_capacity_tests.rs:77-93`・`presenter_blink_stop_tests.rs:87-103`・`presenter_partial_recompose_gpu_tests.rs:109-125`・`presenter_scale_change_tests.rs:133-147`・`presenter_resize_report_tests.rs:207-224, 248-267`・`presenter_frame_log_tests.rs:48`・`presenter_partial_recompose_tests.rs:109-130, 283-299`（`show_ok` の完全な写し）・`presenter_budget_steady_state_tests.rs:166`・`presenter_compose_input_tests.rs:148, 213-227, 250-265, 427`・`presenter_read_accessor_tests.rs:57-71, 93, 139-153, 199-213`・`presenter_display_tests.rs` の 9 か所（`:40-58` ほか・同じファイルが `show_ok` を `:846` で使っている）
- 隠す手伝い: `presenter_visibility_tests.rs:118` と `presenter_display_failure_tests.rs:413`（同じ `hide_ok`）・`presenter_frame_log_tests.rs:72`・`presenter/snapshot_tests.rs:181-194`・`presenter/transition_record_tests.rs:511-523`・`presenter_hide_contract_tests.rs:100-113`・`presenter_read_accessor_tests.rs:160-171`・`presenter_display_tests.rs:496-507`・`presenter_refresh_and_log_tests.rs:479-490`
- 捨てる手伝い: `presenter_partial_recompose_tests.rs:132` と `presenter_read_accessor_tests.rs:215-226`
- 装置の失敗の注入の守り: `presenter_partial_recompose_tests.rs:354-381`・`presenter_display_failure_tests.rs:272-314`（doc は「唯一の入口」とうたう）・`presenter_failure_note_tests.rs:233-258`（同じファイルの `:61-81` の写し）
- 絵の生成＋焼き＋組み: `presenter_test_support.rs:184-212, 247-279, 373-387`・`presenter_compose_input_tests.rs:25-128` と `:316-397`（ほぼ写し）・`presenter_display_failure_tests.rs:123-162`・`presenter_fractional_scale_tests.rs:72-107, 479-532`・`presenter_budget_equivalence_tests.rs:111-169`・`presenter_partial_recompose_tests.rs:46-88`・`presenter/snapshot_tests.rs:96-119`・`presenter_partial_recompose_gpu_tests.rs:95-106`・`presenter_blink_stop_tests.rs:69-80`・`presenter_display_tests.rs:155-167`・`presenter_dpi_scale_tests.rs:703-714`・`presenter_perf_log_tests.rs:276-324`（`presenter_display_tests.rs:141` の完全な写し）
- そのほか: `arrangement_of`（`presenter_dpi_scale_tests.rs:31`・`presenter_fractional_scale_tests.rs:54`・`presenter_resize_report_tests.rs:21`・変形が `presenter_refresh_and_log_tests.rs:20` と `presenter_visibility_tests.rs:32/39`）・`surface_entity_of`（`presenter_fractional_scale_tests.rs:34`＝`mount_entities(..).0`・`presenter_compose_input_tests.rs:237-242` も）・`mask_bit_counts`（`presenter_fractional_scale_tests.rs:241-251` と `presenter_budget_equivalence_tests.rs:210-223`）・文言の定数 `RANGE_LINE_MESSAGE`（3 か所）と `INFO_LINE_MESSAGE`（2 か所・`presenter_refresh_and_log_tests.rs:68, 250` は部分文字列で照合）・`presenter/transition_record_tests.rs:642-646, 684-688`（同じファイルの `attach` の写し）・`cache_tests.rs:66-94`（`presenter_test_support.rs:141-170` の写し）
- 場面: 応答の契約・空の面の作り方・記録の文言が変わると、写しの 1 つだけが古びる。
- 直しの案: `presenter_test_support.rs` に `show_with(.., binds, pattern) -> PresentOutcome`・`hide_ok`・`invalidate_ok`・`show_with_armed_fault`・`gradient(w,h,salt)`・`bake_world(..)`・`arrangement_of`・`mask_bit_counts`・文言の定数を 1 つずつ置き、写しを消す（約 200〜400 行減る見込み）。`cache_tests.rs` と共有するにはクレートの中のテストの支えの置き場が要る（新しいファイルになる）。
- 範囲の外: 及ばない。

**30. シェルの読み込みのテストの手伝いが写されている**（重複）
- `describe`（`shell_target_shape_issues_tests.rs:42-50`・`shell_target_nesting_tests.rs:73-81`・`shell_target_element_base_tests.rs:359-366`）・`decoder_with`（`nesting:64`・`boxes:229`・`quiet:293`・近い写しが `load:32` と `shape_issues:35-37`）・`SHELL_TARGET` 定数（`nesting:24`・`boxes:226`・`image_only:16`・`load:238`）・`layer0_path`（`base_image:167`・`load:37`）・`pixel`（`template:77`・`base_image:159`）・`insert_solid`（`base_image:106`・`element_base:63`）・検体のパスと 8 つの絵の名前の一覧（`nesting:27-61` が `quiet:387-405` に・`boxes:246` が `quiet:412-417` に）
- 場面: `surface-nesting/` に絵を 1 枚足して `nesting` の一覧だけ直すと、`quiet` の写しが黙ってずれる。
- 直しの案: `shell_target_test_support.rs` へ寄せて写しを消す。`shell_target_test_support.rs:3-6` の「8 つの檻が使う」という古い数と一覧（今は 12）も消す。
- 範囲の外: 及ばない。

**31. バルーンのテストの一時フォルダと手伝いが写されている**（重複）
- `balloon_marker_load_tests.rs:15-57` と `balloon_definitions_tests.rs:20-50` が `TempDir`（連番・パス・`Drop`）を自前で持つ（`target/test-tmp` の下で、共有の `TempPath` の `target\test-roots` の外）。`balloon_test_support.rs` と合わせて 3 つ目の写し。
- `size()` と `ALPHA`（`balloon_marker_load_tests.rs:59-65`・`balloon_definitions_tests.rs:52-63`）・`balloon_definitions_tests.rs:157-162` のクロージャ `count` は同じファイルの `:238-243` の関数と同じ・引用符の外し（`balloon_marker_load_tests.rs:74-76`・`balloon_definitions_tests.rs:204, 310`）が `balloon_test_support.rs:92-101` の `FieldUnquoted` の写し・`balloon_model_tests.rs:159-162, 222-225, 257-260` の同じ 3 行。
- 場面: 名前の付け方や後始末の直しが 1 つの写しにだけ入る。強制終了の残りが共有の置き場の外に散る。
- 直しの案: 2 つのファイルを `TempPath` ＋ `std::fs::write(dir.child(..))` に替え、ほかの写しを 1 つにする（`balloon::test_support` は `balloon_marker`・`balloon_definitions` の子から届かないので、置き場は 1 番の直しと合わせて決める）。
- 範囲の外: 及ばない（1 番と違い、この 2 つは番人の例外表に載っていない）。

**32. emo2 の検体を 1 つのテストの実行ファイルで 2 回写している**（`shell_target_test_support.rs:73`・`balloon_test_support.rs:115`）
- 問題: どちらも自分の `static EMO2 = SampleRoot::acquire("emo2")` を持ち、検体のフォルダを丸ごと 2 回写す。支えのファイル自身の doc（7-8 行目）が「避ける」と書いているその代金である。
- 直しの案: `shell_target_test_support` の 1 つを `pub(crate)` にし、`emo2_balloon_root` はそれを使う。
- 範囲の外: 及ばない。

### 3.3 できれば（Nice to have）— 11 件

**33. 呼び出しを渡すだけの手伝い**（`shell_target_test_support.rs:53-55`・`balloon_test_support.rs:107`・`scale_tests.rs:315-317` の `capture_events`＝`log_capture_kit::capture` そのもの・`scale_tests.rs:299-309` の `ExpectField`・`scale_tests.rs:259-290` の `log_capture_kit` の crate doc の 30 行の写し・`presenter_compose_input_tests.rs:401` の `pattern_overlay`＝`pattern_overlay_at(.., 0, 0)`・`cache_tests.rs:16-18` の `transparent_surface`・`balloon_target_tests.rs:8-10` の `opaque_1x1`＝`opaque(1,1)`）
- 直しの案: 消して呼び出し元へ直に書く。範囲の外: 及ばない。

**34. 静かな読み込みのテストが同じ入力を何度も読む**（`shell_target_quiet_tests.rs:379-381, 448-449, 458-459`）
- 問題: `assert_quiet_matches_emit` は 1 回で 2 回読み込む。同じ入力に 3 回・2 回・2 回呼ぶので、約 8 回の読み込みが余分。
- 直しの案: 探す文言の列を受ける形にする。範囲の外: 及ばない。

**35. 本番のコメントが古い**
- `presenter.rs:61`「テストファイル 8 本」（今は 20 を越える）・`command.rs:152` が撤去済みの `chain` モジュールを名指す・`command.rs:165, 167` の文言「D3D/DXGI 呼び出しが失敗」（包んでいるのは D2D の呼び出し）・`command.rs:283` のテストの文脈 `"CreateSwapChain"`（撤去済みの供給面の名残）・`presenter/budget.rs:374`「2 スロット＋各バッファの到達済み詰めバイト長」（今は空き 1 枚で高水位を持たない）・`balloon.rs:73`「本仕様で唯一実装する族」（矢印・印の族も実装済み）・`shell_target.rs:150-154`「写し替えはタスク 5.1」・`balloon.rs:15` の `marker`（`\![*]` マーカー）の説明と今の `MARKER_FAMILY` の役（行の中の印）の食い違い。
- 直しの案: 直すか、数・タスク番号を消す。範囲の外: 及ばない。

**36. テストのコメントが古い・文言が壊れている**
- `presenter_visibility_tests.rs:602`（`refresh.rs:74-80` を可視のゲートとして引くが、今そこは k 不変のゲート・可視のゲートは 94-110）・`presenter_display_tests.rs:612, 701-703`（`scale()` を「恒常値 1.0」と書く・`cache.get(.., k)` の k は今は引数でない）・`presenter_perf_log_tests.rs:16, 280, 583`（「12 フィールド」・「早期復帰 8 経路」は今の数と合わない）・`presenter_display_failure_tests.rs:4`（撤去済みの `presenter_upload_failure_tests.rs`）・`presenter_read_accessor_tests.rs:373-377`・`presenter_budget_steady_state_tests.rs:36, 488`（「既存 184 件」）・`presenter_hide_contract_tests.rs:14, 134`（`crates/areka-emo-text/src/surface.rs:249-262` を引くが今は 310-334）・`presenter_test_support.rs:23, 312`・`presenter/transition_record_tests.rs:724`・`balloon_test_support.rs:53-79, 111-112`（今は別のファイルに在るテストの「檻 7・8」の見出しと経緯）・`shell_target_emo2_tests.rs:18`・`shell_target_base_image_tests.rs:25`（`plan.rs` → 今は `plan_extent.rs`）・`display_fault_tests.rs:6-20, 71`・`display_gpu_tests.rs:8-9`（撤去済みの `chain*.rs`・`UploadFault`）
- `presenter_display_tests.rs:785`: `expect` の文言に 9 つ続きの空白が入っている（`\` の行継ぎが失われた跡・バイトで確認済みと組 5 が報告）。
- 直しの案: 行番号を消すか今に合わせる。文言の行継ぎを戻す。範囲の外: 及ばない（`crates/areka/src/emo2_boot/frame_visibility_integration_tests.rs:390` も同じ古い行番号を引くが範囲の外・触らない）。

**37. `Cargo.toml` の dev 依存の `windows` が重複で、理由のコメントが誤り**（`Cargo.toml:30-35`）
- 問題: `windows` は `[dependencies]` に同じ `{ workspace = true }` で既に在り、通常の依存はテストからそのまま使える。コメントの「dev-deps は workspace 既定の暗黙継承がなく明示が要る」は事実と違う。
- 直しの案: dev 依存の 1 行とコメントを消す（同じパッケージ・同じ機能なので `Cargo.lock` は変わらない見込み＝**実装の段で `Cargo.lock` の差分 0 を確かめる**・要件 6.5）。範囲の外: 及ばない。

**38. 合成の鍵のハッシュが知らない描画メソッドの名前を混ぜない**（`presenter/timing.rs:311-325`）
- 問題: `ComposeMethod::Unknown(Box<str>)` は判別子だけを混ぜ、中の名前を混ぜない。名前の違う知らないメソッドが同じ `key_hash` になり、perf の行の「異なる鍵の数」を少なく数える（`Blend` で同じ穴を塞いだ理由と同じ）。
- 直しの案: `Unknown(name)` のとき `name.hash(..)` を足す。測る行だけの話で表示は変わらない。範囲の外: 及ばない。

**39. 刻印と perf の行のフレーム番号の出どころが資源の片方だけのときに食い違う**（`presenter/transition_record.rs:213-231`）
- 問題: `stamp_of` は `FrameCount` と `TickStart` が両方在るときだけ番号を出し、`frame_of` は `FrameCount` だけで出す。doc は「同じ資源から読む」とうたう。
- 場面: `FrameCount` だけが在る World（テストの World など）では、perf の行は frame=N、surface の行は frame=0 になり、突き合わせが合わない。本番の `EcsWorld::new` は両方を入れる見込み（**未確認**）。
- 直しの案: `frame_of` を `stamp_of(world).frame` にする。範囲の外: 及ばない。

**40. 読み込みで同じフォルダ・同じファイルを何度も読む**（`balloon.rs:408, 655, 663-669`・`balloon_marker.rs:150, 295`・`balloon.rs:554-555`・`shell_target.rs:390-392, 440`）
- 問題: バルーン 1 つの読み込みで、フォルダの一覧を系列の解決・透過の宣言の `.pna` 数え・矢印の族・印の族ごとに取り直し、`descript.txt` を面ごとに読み直して文字コードを解き直す（`load_scope_balloon_model` が面ごとに呼ばれる）。シェルは `surfaces.txt` の本文を 4 回解析する（`parse`・`parse_boxes`・`parse_undrawn_elements`・`parse_cursor_braces`）。
- 場面: 起動と切り替えのときだけなので体感の遅さは小さい。面の多いバルーンで「面ごとの 2 層の重ねが確定」の `info!` と、`descript.txt` が読めないときの `warn!` が面の数だけ並ぶ。
- 直しの案: 一覧と `descript.txt` の本文を 1 回取って渡す。シェルの 4 回の解析は parsers 側の口の形に縛られるので、このクレートだけでは減らせない（parsers は `audit-parsers-sakura` の範囲）。
- 範囲の外: 一覧と本文の使い回しは及ばない。解析の回数を減らすなら `crates/areka-parsers/`（C17 で並んで走る `audit-parsers-sakura` の範囲・申し合わせが要る）。
- **申し合わせ（2026-11-05・`audit-parsers-sakura` と確定）**: 解析の回数は `audit-parsers-sakura` が直す（あちらの指摘 35＝1 回の字句分けから Shell・boxes・undrawn・cursor の 4 つを返す口を parsers に置き、`shell_target.rs:390-392`・`:440` の呼び出しをそれへ替える・あちらの実装の最後の段・あちらの要件 7.10 に記録）。この spec が直すのはバルーンの一覧と `descript.txt` の使い回しだけ。あわせて `build_shell_target_with_boxes` は、署名（`ShellBoxes` → `Vec<BoxDefinition>`・あちらの指摘 76）があちら、可視性（指摘 7）がこちら。後に着地する方が合わせる。

**41. 使われない戻り値と使われない `&mut self`**（`cache.rs:247-278`・`presenter/budget.rs:461-476`）
- 問題: `ComposeCache::insert` は挿入したエントリの参照を返すが、本番の呼び手（`presenter/show.rs:261-268`）は捨てる。`FrameBudget::display_buffer` は doc が「状態を持たない」と書くのに `&mut self` を取る。
- 直しの案: 戻り値を `()` に、`display_buffer` を関連関数か `&self` に。テストが戻り値を読んでいれば `get` に替える。範囲の外: 及ばない。

**42. 作者の DPI の 0 の正規化が 2 か所にある**（`scale.rs:68-91, 124-130, 190-196`）
- 問題: `ScalePolicy::new` と `derive_scale`（`classify`）が同じ正規化と同じ `warn!` を持つ。欄が公開なので構造体の字面で迂回されても守る「多層防御」と doc は説明する。
- 場面: 今の本番の呼び手は `ScalePolicy::new` だけなので、`derive_scale` 側の正規化は通らない。同じ規則が 2 か所にあり、片方だけ変えられる。
- 直しの案: (a) 欄を私有にして正規化を `new` の 1 か所にする（範囲の外で `ScalePolicy` を名指すのは 2 ファイル・字面で組んでいれば直す）。(b) そのまま残す。
- 例外の札: ⑴ の候補。doc が完了した spec の設計（「design の Service Interface どおり公開のまま」）を根拠に挙げている。

**43. テストだけが使う語彙の公開の定数**（`balloon_marker.rs:24-29` の `CLICKWAIT_FAMILY`・`presenter/transition_record.rs:78-89` の `SURFACE_REASON_ALL`）
- 問題: `CLICKWAIT_FAMILY` は doc に「本 spec では表と決定論のテストだけ・絵は読まない」とあり、使い手はこのクレートのテストだけ。`SURFACE_REASON_ALL` は doc が判定側の除外の意味の単一の定義元とうたうが、範囲の外の使い手は 0（判定側の `crates/areka/src/placement/transition_judge` は語を直に名指している見込み・**未確認**）。
- 直しの案: (a) 残す（語彙の口として）。(b) `CLICKWAIT_FAMILY` は `#[cfg(test)]` にし、`SURFACE_REASON_ALL` は判定側から参照させるか消す。
- 範囲の外: (b) で判定側から参照させるなら `crates/areka/src/placement/`（`placement-crate-split` の範囲で C17 に居ない）。
- 例外の札: ⑵ の候補（クリック待ちの族は配線ゼロの語彙として置いた先送りの印と読める）。

### 3.4 audit の締め

- **Verdict**: 本番のコードは筋がよい（単一の漏斗・失敗の記録・決定論のテストが揃う）。先に直すのは 1 番（一時フォルダの決まり違反）、次に 2・3 番（失敗の型の取り違え）と 5 番（毎コマの無駄な COM 呼び出し）。テストは落ちようがない確かめ（9・12〜17 番）と写し（28〜31 番）が目立つ。
- **Lean**: 消せる見込み 約 −1,200〜−1,600 行（テストの部分集合・写し・落ちない確かめ・届かない枝・渡すだけの手伝い）。依存 −0（`Cargo.toml` の dev 依存の重複 1 行は消せるが、同じパッケージなので依存の数は変わらない）。
- **Not checked**: なし（要件 1.3）。ただし上の「未確認」「要確認」「要調査」の 5 点は、範囲の外のファイルの中身に関わる推測で、設計の段で確かめる。

## 4. 要件と既存の資産の対応（Requirement-to-Asset Map）

| 要件 | 既存の資産・決まり | ギャップ（Missing／Unknown／Constraint） |
|---|---|---|
| 1 指摘の洗い出し | ponytail 5.3.2 の audit の作法・本文書 | 済み（43 件・Not checked なし） |
| 2 全部直す | 上の直しの案 | Unknown: 例外の札の付いた 5 件（1・3・8・42・43）の裁定 |
| 3 例外 3 種 | roadmap「生きている決まり」1〜10・記憶の決めごと・完了 spec の要件 | Constraint: ⑶ に当たる見込みの指摘は 0 件（ukadoc の確かめは不要の見込み） |
| 4 直しを守るテスト | `log-capture-kit`（記録の捕捉）・`temp-path-kit`（一時フォルダ）・`presenter_test_support`（GPU の World・実 GPU のみ・WARP なし） | Missing: 2・3・5・18〜25 番の新しいテスト。Constraint: GPU のテストは実 GPU（`GraphicsCore::new()` は HARDWARE のみで、無ければ `expect` で落ちる＝黙って飛ばさない・確認済み） |
| 5 かけ直しで 0 件 | 本文書と同じ読み方 | Constraint: かけ直しで新しい指摘が出ればタスクが増える |
| 6 守る決まり | 番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`・`shell_target.rs` は 951 行 | Constraint: `shell_target.rs` は足せる余地が 49 行。`cache_tests.rs` に行を足さない（10 番は消すだけ） |
| 7 範囲の外 | 呼び出し元 `crates/areka`・`areka-emo-text`・`pilot`・`log-capture-kit` | Constraint: 1・10 番は `log-capture-kit`（`audit-test-kits`・C17 に居ない）、7 番のコメント 1 行は `areka-emo-compose`（`audit-emo-compose`・C17 で並走）、40 番の解析は `areka-parsers`（`audit-parsers-sakura`・C17 で並走）、6 番 (a) は 17 ファイル、3 番は `crates/areka` の 1 か所＋型を名指すテスト |
| 8 20 タスク以下 | — | 下の 6 章（約 15〜17 タスクの見込み・20 に近い） |

## 5. 進め方の選択肢

### 案 A: 組ごとに切る（必ず直す → 直したほうがよい → できれば）
- 要件 8.3 の切り方そのまま。1 組の中でファイルが散るので、同じファイルを複数のタスクが触る（例: `presenter_test_support.rs` は 2・4・29 番が触る）。
- ✅ 優先度の順に着地できる ❌ 同じファイルの書き換えが重なり、レビューが重い

### 案 B: ファイルのまとまりごとに切る（presenter／mount と display／shell_target／balloon 系／cache）
- 1 タスクが 1 つのまとまりの指摘を組をまたいで全部直す。
- ✅ ファイルの重なりが少なく、テストの支えの寄せ（29〜31 番）と直しを同じタスクで済ませられる ❌ 「必ず直す」が後ろのタスクに散る

### 案 C: 混ぜる（推す形の候補）
- 先頭に「必ず直す」と、型を変える直し（2・3・4・6 番）を置く（呼び出し元に及ぶものを早く着地させ、後のテストの直しが新しい型の上で書けるようにする）。その後をファイルのまとまりで切る。最後を検証のタスク（かけ直し＋全体テスト）にする（要件 8.4）。
- ✅ 範囲の外に及ぶ直しを先に片づけられる ❌ 計画が少し複雑

## 6. タスクの数の見立てと 20 の上限（要件 8）

案 C で切った見立て（1 タスクは「直し＋守るテスト＋そのまとまりのテストの緑」）:

1. テストの一時フォルダを `target` の下へ（1・31 番＋ `log-capture-kit` の例外表 1 項目）
2. 失敗の型（2・4 番＝`apply_show` が結果を返す・`PresentError` の変位）
3. バルーンの読み込みの失敗の型と、その `error!` のテスト（3・20 番＋ `crates/areka/src/emo2_boot/assets.rs`）
4. 毎コマの可視・当たりの書き換え（5 番）
5. `attach_target` の署名と届かない枝（6 番・(a) なら範囲の外 17 ファイル）
6. 公開の面を狭める・`allow(dead_code)` を `cfg(test)` へ（7・8・41・43 番）
7. 合成メモのテスト（9・10・11 番＋ `log-capture-kit` の番人の表 3 か所）
8. 落ちようがない確かめと弱いテスト（12〜17 番）
9. presenter の抜けたテスト（18・22・23・25 番）
10. 装着・バルーン・印・シェルの抜けたテスト（19・21・24 番）
11. 写しのテストを消す（28・34 番）
12. presenter のテストの支えを寄せる（29 番）
13. シェルとバルーンのテストの支えを寄せる（26・27・30・32・33 番）
14. 古いコメントと小さな直し（35〜40・42 番）
15. 検証（同じ範囲へ audit をかけ直して 0 件・`pwsh -NoProfile -File tools/test-all.ps1` の緑）

- **見立て: 15 タスク（幅 15〜18）**。20 を越えない見込み。ただし幅の上の側は次の 3 つで増える: ⑴ 6 番を (a)（17 ファイルの署名の直し）で取る、⑵ 12 番の支えの寄せが 1 タスクに収まらない（写しの数が多い）、⑶ かけ直しで新しい指摘が出る（要件 5.2）。
- **20 を越えたとき**（要件 8.2）: 「できれば」の 11 件（33〜43 番）と、テストの支えの寄せ（29〜31 番）を同じ範囲の 2 本目の spec へ出すのが、切り口として最も素直（どちらも本番の振る舞いを変えない）。

## 7. 大きさと危うさ

- **Effort: M**（3〜7 日）— 本番の直しは小さい（どれも数十行以内）が、テストの寄せと消しが 64 ファイルに広く散る。
- **Risk: Low〜Medium** — 本番の振る舞いは変えない直しがほとんどで、既存の型と手伝いの延長で済む。Medium の側は、範囲の外に及ぶ 4 点（`log-capture-kit` の 2 つの表・`attach_target` の 17 ファイル・バルーンの失敗の型の呼び手）と、テストを消すことによる守りの取りこぼし（消す前に、残るテストが同じ主張を本当に持つかを 1 件ずつ確かめる・要件 2.4）。

## 8. 設計の段への申し送り

### 決めが要る点（要件の討議の議題の候補）
1. 例外の札の付いた 5 件（1・3・8・42・43 番）を直すか残すか。
2. 6 番を (a)（署名を直す・範囲の外 17 ファイル）と (b)（届かない枝だけ消す）のどちらで取るか。
3. 1・10 番で `log-capture-kit` の 2 つの例外表（`temp_path_guard_test.rs:156`・`file_length_guard_test.rs:66-69, 101, 104`）を触るか。触るなら `audit-test-kits`（C17 に居ない）の範囲を、この spec が申し合わせ無しで触ってよいか（要件 7.5）。
4. 7 番の `crates/areka-emo-compose/src/nesting_fixture_tests.rs:4` のコメント 1 行と、40 番の `areka-parsers` の解析の回数を、並走する `audit-emo-compose`・`audit-parsers-sakura` と申し合わせるか、この spec では触らないか。
5. タスクの切り方（5 章の案 A／B／C）と、20 を越えたときの 2 本目の切り口（6 章）。

**2026-11-05 要件の討議で全議題を推奨案で確定**（開発者「承認」）＝上の 1〜5 の決めは requirements.md の要件 9（直す対象）・7.7（例外表）・8.5（2 本目の切り口）が正本。

### 調べが要る点（Research Needed）
- 3 番: 読み込みの失敗を `EmptyComposition` に畳む形が、完了した spec の裁定（「設計ディスカッション #1」）の範囲に入るか（その spec の requirements／design を引く）。
- 24 番: `crates/areka/src/offline_dump/offline_dump_tests.rs` が 1 回目の `take_world` のアトラスの装着を画素で捕まえているか。
- 25 番: emo-compose の下敷きの合成の矩形が、セルの帯の外の着せ替えの層を覆うか（`audit-emo-compose` の範囲・読むだけ）。
- 39 番: 本番の `EcsWorld::new` が `FrameCount` と `TickStart` を必ず両方入れるか。
- 43 番: 判定側（`crates/areka/src/placement/transition_judge*`）が `SURFACE_REASON_*` の語をどう名指しているか。
- 37 番: dev 依存の 1 行を消したとき `Cargo.lock` が 1 バイトも変わらないか（実装の段で確かめる）。
