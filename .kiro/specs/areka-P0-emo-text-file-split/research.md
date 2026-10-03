# ギャップ分析: areka-P0-emo-text-file-split

- 実施日: 2026-10-03
- 基準: ワークツリー HEAD `a11cecfb`（main `1ce4c74e` ＋ spec の初期化のみ）。対象 7 本の行数は要件定義の表と一致（975／977／914／871／930／923／977）。
- 方法: Grep／Read による現物の読み取り。外部の依存の調べは不要（依存は 1 行も変えない＝要件 2.6）。
- 本書は**情報と選択肢**を並べる。最終の切れ目・名前・可視性の付け方は設計で決める。

---

## 1. 要約

- **既にある流儀でほぼ足りる**: 6 本のうち 5 本は、すでに `#[path = "<親>_<役割>.rs"] mod <役割>;` で子モジュールを 1〜2 本抱えている（`actor_decoration.rs`・`layout_line_ops.rs`／`layout_styled.rs`・`viewbox_draw_plan.rs`／`viewbox_draw_decoration.rs`・`balloon_visibility_phase.rs`）。同じ形の子を足し、元のファイルが `use`／`pub use` で名前を束ね直せば、呼び出し側（他のモジュール・他の crate・examples・`tests/`）は 0 行で済む。
- **「定義は元に残し、処理を子へ」が最小の差分**: Rust の私有の項目は、定義したモジュールとその子孫から見える。構造体・列挙型の定義を元のファイルに残し、`impl` の塊や関数を子へ出すと、私有の欄（例 `TextLayerRuntime` の `routing`・`layout_input`、`CommittedLine` の `choice_marker`）をテストがそのまま読める。子へ出した私有の関数を親・兄弟・テストが呼ぶ場合だけ `pub(super)` が要る。
- **字面を読む見張りは 4 つ＋条件付きで 1 つ**: 要件の 4 つ（`lib.rs` の層規律・`layout_cursor_overflow_tests.rs`・`layout_styled_tests.rs`・`frame_attach_tests.rs`）に加え、`draw_format_metrics_tests.rs` の「`draw` で始まる本番ファイルの一覧」が、新しいファイル名を `draw` で始めた場合だけ反応する。ワークスペース全体を歩く見張り（1,000 行・同期送信・期限・起床の旗・ログ捕捉）は、対象 7 本が該当の綴りを持たないので影響 0。
- **`layout.rs` の大きい関数は割らずに目安へ届く**: 本体 `layout_inner` を（行を閉じる 4 関数と一緒に）丸ごと子へ移せば `layout.rs` は約 490 行になる。割る必要は行数の上からは無い。割るなら 10 個の可変の局所変数を持つ 1 本の走査を作り替えることになり、振る舞いを変える危険が大きい。
- **brief の役割の名指しに現物と合わない所がある**: `actor.rs` の `dispatch_block` は存在しない（相当は `TextLayerRuntime::apply_cue`）。`input_events/balloon.rs` に「中断のダブルクリック」と「ドラッグ」の本体は無い（中断は `input_events/user_break.rs`、ドラッグは wintf と `placement`）。`balloon_visibility.rs` の「窓への反映」は既に `balloon_visibility_phase.rs`（触れてはいけない側）にある。切れ目は現物で決め直す必要がある。

---

## 2. 現状の調べ

### 2.1 既にある流儀

| 形 | 先例 | 特徴 |
|---|---|---|
| 平らな兄弟ファイル＋`#[path]` の子モジュール | `actor.rs` の `mod decoration;`（`actor_decoration.rs`）、`layout.rs` の `mod line_ops;`／`mod styled;`、`viewbox_draw.rs` の `mod decoration;`／`mod plan;`、`balloon_visibility.rs` の `mod phase;` | 子は `use super::{…}` で親の名前を引き、親へ見せる項目は `pub(super)`。親は素の `use` で束ね直す。emo-text の流儀（structure.md「主要ファイルと接続」） |
| 元のファイルを入口に残し、下位ディレクトリへ移す形 | 完了 `areka-P0-file-slimming` の `placement/follow.rs`→`follow/*.rs`、`emo2_boot/frame.rs`→`frame/*.rs` | `pub use` で再輸出・呼び出し側 0 変更。子は私有 `mod` |
| テストだけを兄弟ファイルへ | 全 6 本が既に実施済み（各 7〜13 本の `#[cfg(test)] #[path] mod …_tests;`） | `region.rs` だけが本体と同居の `mod tests { … }` を持つ |

**emo-text では下位ディレクトリ形は不向き**: `lib.rs` の `every_source_file_is_either_scanned_or_explicitly_excluded` は `src/` を **1 段だけ** `read_dir` する。`src/layout/xxx.rs` に置くと、純粋層の見張りが黙ってそのファイルを読まなくなる（要件 5.5 の趣旨に反する）。emo-text は平らな兄弟が実質の一択。`crates/areka` の 2 本はどちらの形も取れるが、隣の既存の子（`balloon_visibility_phase.rs`）が平らな形なので揃えるのが自然。

**可視性の性質（設計の前提）**:
- 私有の項目・私有の `use` は、そのモジュールと**子孫**から見える。兄弟のテストモジュール（`actor::tests` など）は元のモジュールの子なので、元のファイルに残った私有の定義はそのまま読める。
- 子の `impl` に置いた私有のメソッドは、その子の中でしか見えない。親や兄弟（テストを含む）が呼ぶなら `pub(super)` が要る。
- `pub(crate) use` の再輸出は、非テストのビルドで誰も使わないと `unused_imports` の警告を出す（structure.md のファサード分割の注意）。

### 2.2 対象ごとの目録（行番号は doc と属性を含む塊の範囲・基準 `a11cecfb`）

#### `crates/areka-emo-text/src/actor.rs`（975 行・結線層）

| 範囲 | 中身 |
|---|---|
| 1〜42 | モジュール doc・`use` |
| 44〜158 | `TextSlotBinding`（＋`new`・`from_view`）・`ResolvedBalloonText`（＋`resolve`） |
| 160〜200 | `warn_coarse_wrap_threshold`（私有・粗いバルーン定義の警告） |
| 202〜236 | `pub use crate::choice::HitRectPx`・`ChoiceHitRow`・`ActorRender`（私有） |
| 238〜282 | `TextLayerRuntime` の定義（欄はすべて私有） |
| 284〜615 | `impl TextLayerRuntime`: `new`／登録と再追従（`register_actor`・`register_actor_view`・私有 `register_actor_binding`・`refresh_actor_scale`・私有 `refresh_actor_binding`＝304〜479・176 行）／`apply_cue`（481〜536・指令の振り分け）／読み口 9 本（538〜614） |
| 617〜649 | `spawn_emo_text` |
| 651〜931 | `present_frame`・私有 `present_actor`（1 コマの描画の流れ・計 281 行） |
| 933〜975 | テストの接続 10 本と `mod decoration;` |

- テストが私有の項目に触れている: `actor_decoration_tests.rs`・`actor_region_warn_tests.rs` が私有メソッド `register_actor_binding`／`refresh_actor_binding` を呼び、`actor_scale_refresh_tests.rs` ほかが私有の欄 `rt.layout_input`／`rt.routing` を読む。
- `present_actor` は `decoration::build_actor_render`／`decoration::glyph_styles_of`（`pub(super)`）と、`TextLayerRuntime` の私有の欄 8 つを使う。

#### `crates/areka-emo-text/src/layout.rs`（977 行・純粋層）

| 範囲 | 中身 |
|---|---|
| 1〜87 | モジュール doc（87 行・軸の読み替え表ほか） |
| 88〜99 | `use`・`mod line_ops;`・`mod styled;` |
| 100〜253 | 公開の型: `GlyphMetrics`・`FixedMetrics`・`FIXED_LINE_BOX_RATIO`・`LineRect`・`PositionedGlyph`・`PositionedLine`・`VisibleWindow`・`WrapPlan`・`LayoutEngine` |
| 255〜366 | `impl LayoutEngine` の公開の入口 `layout`（256〜324）・`layout_with_cursor_warn`（325〜366） |
| 367〜744 | 私有の本体 `layout_inner`（doc 367〜377・関数 378〜744＝367 行） |
| 746〜820 | `visible_window`（見える範囲の計算・`tracing::debug!` 1 つ） |
| 822〜933 | 私有の仕上げ 4 本: `finish_pending_line`・`apply_pending_newline`・`apply_pending_cursor`・`finish_line` |
| 935〜977 | テストの接続 14 本 |

- 是正（要件の注の補足）: 要件の注は `layout_inner` を「378〜765 行目・約 388 行」と書くが、関数そのものは **378〜744 行目（367 行）** で、745〜765 行目は次の `visible_window` の doc である。結論（入口 35 行＋私有の本体 1 本）は変わらない。
- `layout_inner` は `for item in items` の 1 本の走査に、走査をまたいで生きる可変の局所変数を 10 個持つ（`heights`・`lines`・`current`・`inline_pos`・`block_pos`・`placed`・`pending`・`pending_cursor`・`seg_remaining` ほか）。
- `layout_styled.rs` の `impl LayoutEngine { pub fn layout_styled }` は子の中から親の私有メソッド `Self::layout_inner` を呼んでいる（子孫からは見える）。

#### `crates/areka-emo-text/src/viewbox.rs`（871 行・純粋層）

| 範囲 | 中身 |
|---|---|
| 1〜48 | doc・`use` |
| 49〜326 | 計画: `ScrollState`・`FramePlan`・`DirtyRect`（＋テスト専用の `PartialEq<PhysicalRect>`）・`block_axis_vector`・`ScrollPlanner`（欄は私有）と 1 つ目の `impl`（`plan`・`plan_with_overhangs`・`commit`・`request_clear` ほか） |
| 328〜436 | ダーティ導出の型: `DIRTY_GUARD_IMG_PX`・`LineOverhang`・`PhysicalRect`・`CommittedLine`（`pub(crate)`・欄は私有） |
| 438〜647 | 2 つ目の `impl ScrollPlanner`（`committed_lines`・私有 `is_backward_shrink`・`derive_dirty`・`derive_dirty_with_overhangs`） |
| 649〜852 | 私有の補助 6 本（`glyph_run_indices`・`line_fingerprint`・`resident_rect`・`block_axis_overhang`・`exposure_band`・`expand_guard_clamp`） |
| 854〜871 | テストの接続 6 本 |

- 既に 2 つの `impl ScrollPlanner` に分かれている＝**塊の境目がそのまま切れ目**になる。
- テストは私有の `line_fingerprint` を `super::` で引き、`CommittedLine` の私有の欄（`choice_marker`・`text`・`block_pos_bits`・`extent_bits`）を読む。1 つ目の `impl` の `plan_with_overhangs` が 2 つ目の私有 `is_backward_shrink` を呼ぶ。

#### `crates/areka-emo-text/src/viewbox_draw.rs`（914 行・COM 層）

| 範囲 | 中身 |
|---|---|
| 1〜85 | doc（44 行）・`use`・`mod decoration;`／`mod plan;`・テスト用の束ね直し `#[cfg(test)] use plan::plan_inconsistency;` |
| 87〜148 | `FormatKey`・`DrawStats`・`ViewboxExecutor`（欄はすべて私有） |
| 150〜314 | `impl ViewboxExecutor` の前半（`new`・`new_shared`・描画の設定・テスト専用の失敗注入・`stats`・`scroll_state`・`request_clear`・`render`） |
| 316〜653 | `render_styled`（doc 11 行＋本体 327 行・1 本） |
| 655〜727 | 私有 `line_layout_for`・`ensure_format` |
| 729〜886 | 私有の補助（`none_err`・`device_err`・`LineDraw`・`ChoiceDraw`・`ChoiceHover`・`color_f`・`highlight_rect`・`expand_overhang_for_band`・`segment_text_range`） |
| 888〜914 | テストの接続 9 本 |

- brief の「計画（どこを描き直すか）」は、描画側では既に `viewbox_draw_plan.rs` に出ている。残る大きな塊は `render_styled`（327 行）と補助（158 行）。
- テストが引く名前は `ViewboxExecutor`・`DrawStats`・`plan_inconsistency`・`decoration`・`test_support` だけで、補助の私有関数には触れない。

#### `crates/areka/src/input_events/balloon.rs`（930 行）

| 範囲 | 中身 |
|---|---|
| 1〜41 | doc・`use` |
| 43〜176 | 受け渡しの型と資源: `ChoiceSelection`・`BalloonWiring`（＋メソッド 7 本）・`ChoiceSelectionInbox` |
| 178〜328 | 判定の純関数: `hit_choice_row`・`HoverAction`・`hover_action`・`click_selection` |
| 329〜490 | `on_balloon_pointer_moved`（ホバーの追従） |
| 491〜662 | `on_balloon_pointer_pressed`（選択肢のクリック。末尾で `super::user_break::on_left_press` へ渡すだけ） |
| 663〜838 | `clear_balloon_hover_on_leave`（窓の外へ出たときのホバー解除・排他システム） |
| 839〜908 | 結線: `attach_balloon_pointer_handlers`・`wire_balloon_choice`・`register_balloon_leave_system` |
| 910〜930 | テストの接続 7 本 |

- 項目はすべて `pub(crate)`。外からの道筋は `input_events::balloon::{BalloonWiring, ChoiceSelection, attach_balloon_pointer_handlers, register_balloon_leave_system, wire_balloon_choice}`（`ghost_session.rs`・`choice_drain.rs`・`balloon_visibility_phase.rs` ほか）。
- テスト 7 本はすべて `use super::*;`（＋`use super::test_support::{…}`）で名前を引く。純関数・ハンドラ・`ChoiceSelectionInbox` を名指しで使う。
- **brief との食い違い**: 「中断のダブルクリック」は `user_break.rs`（305 行・別ファイル済み）、「ドラッグ」は本ファイルに無い。実在する役割は「受け渡しの型」「判定の純関数」「ホバーの追従」「クリック」「離脱」「結線」の 6 つ。

#### `crates/areka/src/emo2_boot/balloon_visibility.rs`（923 行）

| 範囲 | 中身 |
|---|---|
| 1〜70 | doc（64 行）・`use`（私有 `use super::talk_lifecycle::TalkLifecycleSignal`） |
| 72〜160 | 待ち時間の設定: `DEFAULT_BALLOON_TIMEOUT_SECS`・私有 `TIMEOUT_ENV_KEY`・`TimeoutSource`（私有メソッド `as_str`）・`parse_timeout_ms`・私有 `resolve_timeout_secs`・`configured_timeout_secs` |
| 162〜453 | 観測と状態の型 12 個（`VisibilityTrigger`・`SuppressionKinds`・…・`BalloonVisibilityState`） |
| 455〜712 | 見える・隠すの判断: `decide`・`ContentDecisions`・`apply_lifecycle_signals`・`decide_user_break`・`decide_content` |
| 714〜875 | 時間切れ: `decide_timeout`（141 行）・`observe_suppression` |
| 876〜923 | `mod phase;`＋`pub(super) use phase::run_balloon_visibility_phase;`・テストの接続 8 本 |

- **子 `balloon_visibility_phase.rs`（変更 0 が要件）が `super::` から引く名前は 10 個**: `BalloonVisibilityState`・`GlyphObservation`・`ScopeObservation`・`TalkLifecycleSignal`・`VisibilityAction`・`VisibilityLogEvent`・`VisibilityObservations`・`VisibilityTrigger`・`configured_timeout_secs`・`decide`。加えて孫のテスト `balloon_visibility_phase_tests.rs` が `super::super::{BalloonVisibilityState, MeasurementDiscardReason, ScopeVisibility, SuppressionKinds}` と `super::super::configured_timeout_secs()` を引く。どれも分割後に `balloon_visibility` の直下で見えていればよい（定義が残るか、`use` で束ね直されるか）。
- `balloon_visibility_timeout_config_tests.rs` は私有の `resolve_timeout_secs`・`TIMEOUT_ENV_KEY`・`TimeoutSource::as_str` を `use super::*;` 経由で使う。
- テスト 7 本が `use super::test_support::*; use super::*;` の 2 つの glob を並べている。
- **brief との食い違い**: 「窓への反映」は既に `balloon_visibility_phase.rs`（触れない）にある。本ファイルに残る役割は「設定」「型」「見える・隠すの判断」「時間切れ」の 4 つ。

#### `crates/areka-emo-text/src/region.rs`（977 行・純粋層・任意）

- 本体 1〜487、内蔵テスト 489〜970（`#[cfg(test)] mod tests { … }`・482 行）、兄弟テストの接続 972〜977。
- 内蔵テストは `windows` を 0 件しか含まない（純粋層）。`log_capture_kit::count_levels` を使う。モジュール名 `tests` のまま `region_tests.rs` へ出せば、テストの完全な名前（`areka_emo_text::region::tests::…`）は変わらない。既存の兄弟は `region_inline_limit_tests.rs`・`region_vertical_canon_tests.rs` で、`region_tests.rs` と名前はぶつからない。

### 2.3 新しいファイルの名前の制約（structure.md「最長 stem」と「前向きの衝突禁止」）

同じディレクトリに `foo.rs` と `foo_bar.rs` があると、`foo_bar_baz_tests.rs` は長い方の `foo_bar` の子と読まれる決まりである。したがって**新しい本番ファイルの名前が、既存の兄弟テストの名前の頭と重なってはいけない**（重なると、既存のテストの親の読み取りが狂う）。機械の検査は無いが、steering の決まりである。

| ディレクトリ | 使えない頭（既存のテスト・本番ファイル由来） | 使える例 |
|---|---|---|
| `areka-emo-text/src/`（actor） | `actor_choice`・`actor_clear`・`actor_decoration`・`actor_region`・`actor_runtime`・`actor_scale`・`actor_scroll`・`actor_test` | `actor_present.rs`・`actor_attach.rs`・`actor_cue.rs` |
| 同（layout） | `layout_cluster`・`layout_cursor`・`layout_hard`・`layout_segmented`・`layout_visible`・`layout_wrap`・`layout_styled`・`layout_line_ops`・`layout_test` | `layout_place.rs`・`layout_window.rs`・`layout_entry.rs` |
| 同（viewbox） | `viewbox_axis`・`viewbox_choice`・`viewbox_dirty`・`viewbox_plan`・`viewbox_style`・`viewbox_test`・`viewbox_draw` | `viewbox_damage.rs`・`viewbox_diff.rs` |
| 同（viewbox_draw） | `viewbox_draw_choice`・`_color`・`_decoration`・`_frame`・`_live`・`_oracle`・`_png`・`_scroll`・`_test`・`_plan` | `viewbox_draw_render.rs`・`viewbox_draw_highlight.rs` |
| 同（全体） | `draw` で始まる名前（`draw_format_metrics_tests.rs` の見張りが反応する） | — |
| `areka/src/input_events/` | `balloon_hover`・`balloon_leave`・`balloon_pass`・`balloon_pointer`・`balloon_pure`・`balloon_wiring`・`balloon_test` | `balloon_move.rs`・`balloon_press.rs`・`balloon_exit.rs` |
| `areka/src/emo2_boot/` | `balloon_visibility_content`・`_forget`・`_lifecycle`・`_phase`・`_test`・`_timeout`・`_user_break` | `balloon_visibility_decide.rs`・`balloon_visibility_expiry.rs` |

「自然な名前」（`viewbox_dirty.rs`・`layout_visible_window.rs`・`balloon_visibility_timeout.rs`・`balloon_leave.rs`）の多くが既存のテストの頭とぶつかる。別案として、新しい子を既存のテストの**新しい親**にする（例 `balloon_visibility_timeout.rs` の下へ `timeout_config_tests` を付け替える）手もあるが、テストの完全な名前・`super::` の意味が変わり、前後のテスト一覧の突き合わせに対応表が要る（要件 4.1／4.3 の負担が増える）。

---

## 3. 要件と既存資産の対応

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| 1.1 700 行以下 | 2.2 の目録・既存の子の流儀 | **Constraint**: 名前の制約（2.3）。5 章の案でどれも 700 行以下に収まる見込み |
| 1.2 超える場合の記録 | — | 案 L-b を取る場合のみ発生（約 755 行） |
| 1.3 役割で切る | brief の Approach | **Constraint**: brief の名指しに現物と合わない所が 3 つ（1 章）。切れ目を現物で決め直す |
| 1.4 子の doc に 1〜2 行 | 既存の子の doc の型（`actor_decoration.rs` の冒頭） | **Missing**: 各子がどの後続 spec の受け口かの対応（brief の表から引ける） |
| 1.5 子の一覧の doc | `lib.rs` 冒頭 22〜30 行目の段落（子の一覧を列挙） | **Missing**: 新しい子の追記。steering `structure.md` の「主要ファイルと接続」（emo-text 節）も同じ列挙を持つ＝**Unknown**: 本 spec で直すか完了時の文書同期に回すか |
| 2.1〜2.4 振る舞い・ログ | 純移動の先例と機械の照合（file-slimming の `RustParse.ps1`） | ログの発生元の名前は子の道筋へ変わる（例 `areka_emo_text::actor::present`）。前置きの絞り込みは同じ行を拾う |
| 2.3 大きい関数を割るか | — | 5 章 L 案。割らずに目安へ届く |
| 2.5 発生元の完全一致 | 対象 7 本の発生元を完全一致で判定する箇所: 0 件（再確認。`target=areka::emo2_boot::balloon_background` の完全一致はあるが別モジュール） | なし |
| 2.6 依存 | — | なし |
| 2.7 警告を増やさない | ファサード分割の注意（structure.md） | **Constraint**: テストだけが使う名前の束ね直し（`#[cfg(test)] use` か `#[allow(unused_imports)]`）と要件 3.1 の両立（8 章 論点 5） |
| 3.1〜3.3 道筋 | `pub use`／`pub(crate) use` の再輸出・私有 `mod` | 外の呼び出し側は 0 行で済む（examples の `#[path]` 取り込みは `placement` 等だけで対象 7 本は無い） |
| 3.4 `balloon_visibility_phase.rs` 0 行 | 2.2 の 10 個＋孫テストの 5 個の名前 | 元のファイルの直下で見えていればよい |
| 4.1〜4.2 本数と結果の一致 | file-slimming の `Compare-TestLists.ps1`（多重集合で突き合わせ）・`before_/after_*.txt` の採り方 | **Missing**: `tools/test-all.ps1` は個々のテストの名前と結果の一覧を出さない（引数は `-Format`／`-License` のみ）。前後の一覧の採り方を設計で決める |
| 4.3〜4.6 テストを書き換えない | テストの接続は元のファイルに残せる＝テストの完全な名前は不変 | テストの付け替え（2.3 の別案）を取らない限り 0 本 |
| 5.1〜5.7 字面の見張り | 4 章 | **Missing**: 一覧の追随 |
| 6.1〜6.4 番人・触らない場所 | `file_length_guard_test.rs`（`OVER_LIMIT_ALLOWED_COUNT = 10`） | 対象 7 本は例外の表に無い。新しいファイルはすべて 700 行以下の見込み |
| 7 `region.rs` | file-slimming の `Compare-RelocatedTests.ps1`（行頭の空白を無視した本文一致） | 移すなら `lib.rs` の純粋層の一覧に 1 本足す |
| 8 実機 | 既存の emo2 起動の手順（`target\` の下） | なし |

---

## 4. 字面を読む見張りへの影響

| 見張り | 読むもの | 分割が強いる変更 |
|---|---|---|
| `lib.rs` `pure_layer_modules_have_no_windows_imports`／`every_source_file_is_either_scanned_or_explicitly_excluded` | `PURE_SOURCES`（59 件・件数を `assert_eq!` で固定）と `SOURCES_OUTSIDE_THE_PURE_SCAN`、`src/*.rs` の実ファイル（1 段のみ） | 新しいファイルを 1 本ずつどちらかへ載せる。純粋層（`layout`・`viewbox`・`region` の子）は `PURE_SOURCES` へ・件数 59 を増やす。COM・結線層（`viewbox_draw`・`actor` の子）は除外の一覧へ。**先例の食い違い**: `viewbox_draw_plan.rs`（親は COM 層）は純粋層の一覧に載っている＝要件 5.5「親と同じ層」と合わない先例がある |
| `layout_cursor_overflow_tests.rs` `no_content_less_line_is_ever_emitted_and_line_closing_sites_are_pinned` | `include_str!("layout.rs")` の `finish_line(` 4 回・`finish_pending_line(` 3 回・`fn finish_pending_line(` を含むこと | 呼び出し 3＋2 は `layout_inner` の中と `finish_pending_line` の中にある。`layout_inner` か仕上げ 4 本のどちらかを動かすと数が崩れる。**両方を同じ子へ動かす**なら、読むファイルをその子へ（または `concat!(include_str!("layout.rs"), include_str!("<子>.rs"))` で両方へ）向ければ数は同じ・`finish_pending_line` は私有のまま呼び出し元も同じファイルに閉じる。ただし失敗時の文言は「`layout.rs` の私有関数」と言い続ける（文言は書き換え禁止）。要件 5.4 の「性質そのもの」にファイル名が含まれるかは設計の判断（8 章 論点 1） |
| `layout_styled_tests.rs` `the_line_pitch_formula_is_reached_through_a_single_call_site` | `layout.rs`・`layout_styled.rs`・`layout_line_ops.rs` の 3 本（件数 3 を固定）で `metrics.line_pitch(` が 1 回・`line_gap` が 0 回 | `layout.rs` から出たコードの行き先を一覧へ足し、件数 3 を増やす |
| `frame_attach_tests.rs` `every_production_scan_of_the_actor_map_is_registered` | ワークスペースの本番 `.rs` のうち空白を潰して `state.actors()` を含むもの＝`crates/areka-emo-text/src/actor.rs`（`present_frame` の中）ほか 2 本 | `present_frame` を子へ出すなら、登記の道筋を子のファイルへ差し替える（`crates/areka/src/emo2_boot/frame_attach_tests.rs` に 1 行）。出さないなら 0 行 |
| `draw_format_metrics_tests.rs` `draw_facade_sources_cover_every_draw_production_file` | `src/` の `draw` で始まる本番ファイル | 新しいファイル名を `draw` で始めなければ 0 行（`viewbox_draw_*` は `draw` で始まらない） |
| ワークスペース全体を歩く見張り（`file_length_guard_test`・`with_default_guard_test`・`session_end_sync_send_tests`・`session_end_deadline_tests`・`tick_gate_config_producers_tests`） | 綴り（`SendMessage`・`.arm(`・`tick_wake::`・`with_default(` ほか）の在りかを名簿と突き合わせ | 対象 7 本はどの綴りも 0 件（実測）＝新しいファイルへ移っても名簿は動かない |

要件 5.6 の数え直しの結果: 上の 5 つ以外に、対象 7 本の中身を字面や道筋で名指しする見張りは見つからなかった（`areka-sylphya` の `include_str!("../actor.rs")` と `areka-emo-text/tests/staysee_balloon_fixture/region.rs` は同名の別ファイル）。

---

## 5. 実装の選択肢

### 5.1 全体の型

| 型 | 中身 | 長所 | 短所 |
|---|---|---|---|
| **A: 元のファイルに定義を残し、処理を子へ** | 構造体・列挙型・定数は元に残し、`impl` の塊・関数を平らな兄弟の子へ。元は `use`／`pub use` で束ね直す | 私有の欄・私有の型に子とテストがそのまま届く＝可視性の付与が最小。既存の子（`actor_decoration.rs` ほか）と同じ形 | 1 つの型の `impl` が複数ファイルに分かれる（`impl X {` の頭の行が増える＝要件 2.2 の許す差分に入るかを設計で明記） |
| B: 型ごと子へ | 型と `impl` を一緒に子へ移し、元は再輸出だけ | 1 つの型が 1 ファイルにまとまる | 私有の欄・私有のメソッドをテストや兄弟が使う所で `pub(super)` が大量に要る（`TextLayerRuntime`・`CommittedLine`・`ViewboxExecutor`）。再輸出の未使用の警告も増える |
| C: 下位ディレクトリ（file-slimming の `follow/` 形） | `actor/present.rs` など | file-slimming の先例どおり | emo-text では層規律の見張りが下位ディレクトリを読まない（2.1）。採るなら見張りを広げる必要があり範囲が膨らむ |

### 5.2 ファイルごとの案（行数は概算・新しい子の doc と `use` に 15〜30 行を見込む）

**actor.rs**
- A-1: 1 コマの描画の流れ（`spawn_emo_text`・`present_frame`・`present_actor`）を `actor_present.rs` へ、登録と再追従（`register_*`・`refresh_*`・`warn_coarse_wrap_threshold`）を `actor_attach.rs` へ。→ `actor.rs` 約 490／`actor_present.rs` 約 340／`actor_attach.rs` 約 240。`TextLayerRuntime` の定義と `apply_cue`・読み口は元に残る（後続の多くが欄を足す場所）。`frame_attach_tests.rs` の登記を 1 行直す。テストが呼ぶ私有メソッド 2 本に `pub(super)`。
- A-1′: 描画の流れだけを出す。→ `actor.rs` 約 668（余白は約 30 行）。
- A-2: `present_frame` を元に残し（`frame_attach_tests.rs` 0 行）、登録と再追従＋`apply_cue` と、`TextSlotBinding`／`ResolvedBalloonText` を子へ。→ `actor.rs` 約 640。公開の型が子へ移るので `pub use` が増える。

**layout.rs**
- L-a: 本体 `layout_inner` と仕上げ 4 本を一緒に `layout_place.rs` へ（割らない）。→ `layout.rs` 約 490／子 約 515。`layout_inner` に `pub(super)`（親の入口 2 本と兄弟 `styled` が呼ぶ）。見張り 2 つ（4 章）の読む一覧を追随。
- L-a′: L-a に加えて `visible_window` を `layout_window.rs` へ。→ `layout.rs` 約 420。brief の 3 分割（本体／見える範囲／仕上げ）に近い。ただし仕上げ 4 本は本体と同じファイルに置く（見張りの性質を保つため）。
- L-b: 本体と仕上げは `layout.rs` に残し、公開の型（154 行）と `visible_window`（75 行）を出す。→ 約 755（要件 1.2 の記録が要る）。入口 2 本（111 行）も出せば約 645。`layout_cursor_overflow_tests.rs` は 0 行。
- L-c: `layout_inner` を複数の関数へ割る（要件 2.3 の例外）。10 個の可変の局所変数を状態の構造体か多数の `&mut` 引数へ移す本体の書き換えになる。行数の上では不要で、危険が最も大きい。

**viewbox.rs**
- V-a: 2 つ目の `impl ScrollPlanner`（ダーティ導出）と私有の補助 6 本を `viewbox_damage.rs`（仮名）へ。型 4 つ（`DIRTY_GUARD_IMG_PX`・`LineOverhang`・`PhysicalRect`・`CommittedLine`）は元に残す。→ `viewbox.rs` 約 465／子 約 440。`line_fingerprint`・`is_backward_shrink` に `pub(super)`、`line_fingerprint` はテスト用の束ね直し。
- V-b: 型も一緒に出す。→ `CommittedLine` の私有の欄をテストが読むため欄へ `pub(super)` が要る。

**viewbox_draw.rs**
- D-a: `render_styled` と描画の補助（`LineDraw` ほか）を `viewbox_draw_render.rs`（仮名）へ。→ 元 約 425／子 約 530。`render_styled` は公開メソッドのまま（`impl` の塊を分けるだけ）。
- D-a′: `render_styled` だけを出す（補助は元に残し子は `super::` で引く）。→ 元 約 585／子 約 360。
- D-b: 補助と `line_layout_for`／`ensure_format` だけを出す。→ 元 約 690（余白がほぼ無い）。

**input_events/balloon.rs**
- B-a: ホバーの追従（`on_balloon_pointer_moved`）・クリック（`on_balloon_pointer_pressed`）・離脱（`clear_balloon_hover_on_leave`）を 3 つの子へ。→ 元 約 440（型・純関数・結線）／子 各 170〜200。
- B-b: 2 つのハンドラを 1 つの子へ、離脱と結線は元に残す。→ 元 約 615／子 約 345。
- 共通: 外へ公開している 5 つの名前は元の定義のまま、または `pub(crate) use` で道筋を保つ。動かすハンドラはテストが `use super::*;` で引くので元の束ね直しが要る。

**emo2_boot/balloon_visibility.rs**
- BV-a: 判断の一群（`decide` から `observe_suppression` まで 421 行）を 1 つの子へ。→ 元 約 510／子 約 445。
- BV-b: 「見える・隠すの判断」（`decide`・`apply_lifecycle_signals`・`decide_user_break`・`decide_content`）と「時間切れ」（設定 6 項目＋`decide_timeout`・`observe_suppression`）を 2 つの子へ。→ 元 約 420／子 約 280 と約 270。
- 共通: 型 12 個は元に残す（`phase` と孫のテストが `super::`／`super::super::` で引く）。`configured_timeout_secs`・`decide` は `phase` が使うので束ね直しは未使用にならない。`resolve_timeout_secs`・`TIMEOUT_ENV_KEY`・`TimeoutSource::as_str` を動かすなら `pub(super)` とテスト用の束ね直しが要る（`TimeoutSource` の定義を元に残す手もある）。

**region.rs（任意）**
- R-a: 内蔵テストを `region_tests.rs` へ（`#[cfg(test)] #[path = "region_tests.rs"] mod tests;`）。→ `region.rs` 約 498。`lib.rs` の純粋層の一覧に 1 本足す。テストの完全な名前は不変。
- R-b: 移さない（`region.rs` 0 行）。直後の `shell-balloon` は本体を変えない見込み（brief）。

### 5.3 組み合わせの見立て

- **最小の字面の見張りの変更**: A-2＋L-b（入口も出す）＋V-a＋D-a′＋B-b＋BV-a＋R-b。`frame_attach_tests.rs`・`layout_cursor_overflow_tests.rs` は 0 行、`lib.rs` と `layout_styled_tests.rs` の一覧だけ。
- **後続の足す余地の最大**: A-1＋L-a′＋V-a＋D-a＋B-a＋BV-b＋R-a。全ファイル 540 行以下の見込み。見張り 4 つすべてに一覧の追随が入る。
- どちらでも依存・公開の道筋・テストの中身は不変で、差は「どこまで字面の見張りに手を入れるか」と「余白の大きさ」。

---

## 6. 触る場所・触らない場所の確認

- **触らずに済む（強いられない）ことを確かめた**: `emo2_boot/frame/`（`frame/wiring.rs` は `BalloonVisibilityState` を引くだけ・`frame.rs` は `run_balloon_visibility_phase` を引くだけ）・`placement/`（`BalloonWindowMarker` を引くだけ）・`install/`・`main.rs`・`areka-sakura`・`wintf`・`areka-kanade`・`user_break_cue.rs`・`balloon_visibility_phase.rs`（3.4 の名前が見えていれば 0 行）・`update/`。`emo2_boot/mod.rs`（`pub mod balloon_visibility;`）・`input_events/mod.rs`（`pub(crate) mod balloon;`）も、子を各ファイルの中の `#[path]` で宣言する限り 0 行。
- **brief の「触るファイル」の外で変わりうるもの**（要件 5.7 の一覧の候補）:
  - `crates/areka-emo-text/src/layout_cursor_overflow_tests.rs`（L-a 系のとき）
  - `crates/areka-emo-text/src/layout_styled_tests.rs`（`layout.rs` からコードを出すとき）
  - `crates/areka/src/emo2_boot/frame_attach_tests.rs`（`present_frame` を出すとき）。このファイルは `frame/` の外だが `frame.rs` のテストである。同じ C1 で `frame/` を触るのは `restart-chain-finalize-stall`（`frame/drain_resnap.rs`）で、本ファイルとは重ならない
  - 対象の兄弟テストの `use` の付け替え（束ね直しで足りれば 0 行）
- `crates/areka-emo-text/src/lib.rs` は brief の「モジュールの宣言」に入るが、実際に変わるのは doc の段落と、テストの中の 2 つの一覧と件数 59 である。

---

## 7. 規模とリスク

- **規模: S〜M**（目安 6〜9 タスク＝1 ファイル 1 タスク＋`region.rs`＋見張りと証跡の仕上げ）。既存の流儀の繰り返しで、新しい仕組みは無い。証跡（前後のテスト一覧の突き合わせ・純移動の機械照合）に file-slimming の道具を流用できるかで M 寄りにもなる。
- **リスク: 低〜中**。中身を変えない移動なのでコンパイルが大半の誤りを捕まえる。残る危険は (1) 束ね直しの漏れによる警告の増加、(2) glob の重なり（`use super::test_support::*; use super::*;`）での名前の曖昧さ、(3) 字面の見張りの読む範囲が黙って縮むこと、(4) `layout_inner` を割る案を取った場合の振る舞いの変化。

---

## 8. 設計へ持ち越す論点と調べもの

### 論点（答えで作業が変わるもの）

1. **`layout.rs` の大きい関数を割るか・どこへ置くか**: 割らずに丸ごと子へ（L-a／L-a′）、割らずに元に残す（L-b）、割る（L-c）のどれか。L-a のとき、`layout_cursor_overflow_tests.rs` の「`finish_pending_line` は `layout.rs` の私有関数」を、読むファイルの差し替え（または `concat!` で両方）で追随させてよいか、それとも要件 5.4 の「性質そのもの」に当たるとして仕上げ 4 本を `layout.rs` に残すか。
2. **新しいファイルの名前**: 既存の兄弟テストの頭とぶつからない名前を選ぶ（2.3 の表）か、新しい子を既存のテストの新しい親に付け替えるか。付け替えはテストの完全な名前が変わり、要件 4.1 の一致に対応表が要る。
3. **型の定義を元のファイルに残すか**（全体の型 A／B）。残せば可視性の付与が最小、移せば欄への `pub(super)` が増える。あわせて、1 つの型の `impl` を複数ファイルに分ける（`impl X {` の頭の行を足す）ことを要件 2.2 の許す差分に含めると設計に明記するか。
4. **`present_frame` を `actor.rs` から出すか**: 出すと「1 コマの描画の流れ」が独立し余白が大きいが、`crates/areka/src/emo2_boot/frame_attach_tests.rs` の登記を 1 行直す（別 crate のファイル）。出さないなら他の塊（登録・型）を出して目安へ届かせる。
5. **テストだけが使う名前の束ね直しの書き方**: 要件 3.1（`pub(crate)` の名前を同じ道筋で保つ）と要件 2.7（警告を増やさない）が、テストからしか使われない再輸出でぶつかる。`#[cfg(test)] use`（先例 `viewbox_draw.rs` の `plan_inconsistency`）か `#[allow(unused_imports)]`（先例 `placement/follow.rs`）か。前者は非テストのビルドで道筋が消えるので、要件 3.1 の「届く状態」の読みを設計で定める必要がある。対象: `input_events/balloon.rs` の純関数（`hit_choice_row` ほか・本番ではハンドラからしか呼ばれない）、`balloon_visibility.rs` の `TimeoutSource` など。
6. **brief の役割の名指しの読み替え**: `dispatch_block`（実在しない→`apply_cue`）、`balloon.rs` の「ダブルクリック」「ドラッグ」（本体は別ファイル→実在する「ホバー／クリック／離脱」で切る）、`balloon_visibility.rs` の「窓への反映」（既に `phase`）。要件 1.3 は「出発点は brief、最終は設計」なので、設計で現物の役割へ置き換えた理由を残す。
7. **余白の大きさ**: 各ファイルを 700 行ぎりぎり（例 A-1′ 約 668・D-b 約 690）に止めるか、後続の spec の数（`actor.rs` 5 本・`viewbox_draw.rs` 5 本・`balloon.rs` 5 本）を見て 500 行前後まで下げるか。
8. **`region.rs` の内蔵テストを移すか**（要件 7）。移すなら `lib.rs` の純粋層の一覧に `region_tests.rs` を足し、空白を無視した本文の一致を file-slimming の `Compare-RelocatedTests.ps1` で示せる。
9. **新しい子の層の一覧の載せ先**: 要件 5.5 は「親と同じ層」だが、先例 `viewbox_draw_plan.rs`（親は COM 層）は純粋層の一覧に載っている。`viewbox_draw` の子を足すとき、どちらに倣うか（`render_styled` を移すなら `windows` を使うので除外の一覧しか取れない）。
10. **steering `structure.md` の emo-text 節**（主要ファイルと接続の列挙）を本 spec の中で直すか、完了時の文書同期に回すか。

### 調べもの（設計の時点で確かめる）

- **前後のテスト一覧の採り方**: `tools/test-all.ps1` は個々のテストの名前と結果を一覧にしない。`cargo test … -- --list` と本走の出力（`test X ... ok`）の採取、`Compare-TestLists.ps1`（多重集合での突き合わせ）の流用可否を確かめる。i686 の段（host-32）も同じ一覧に含める。
- **glob の重なり**: `balloon_visibility_*_tests.rs` と `input_events/balloon_*_tests.rs` の `use super::test_support::*; use super::*;` に、束ね直しで増える名前が `test_support` の名前とぶつからないかをコンパイルで確かめる（E0659 の回避は明示の `use`）。
- **`concat!(include_str!(…), include_str!(…))`** で 1 つの定数に 2 本を読ませる書き方が、見張りの数え方（4／3）を変えずに通るかを確かめる（論点 1 の追随の手段）。
- **ログの発生元の名前の変化の確認**: 例 `choice_selected` は `areka::input_events::balloon` から子の道筋へ、粗いバルーン定義の警告は `areka_emo_text::actor` から子の道筋へ変わりうる。`doc/`・`tools/` に完全一致で探す手順は 0 件（確認済み）。完了 spec の記録にある過去の行は記録として残す。実機の確かめ（要件 8）で前置きの絞り込みが同じ行を拾うことを見る。
- **動かした doc の内部リンク**（`[`present_frame`]` など）は子の中で解決先が変わりうる。`cargo doc` はゲートではなく、structure.md は「直さない」を正としている（本文を不変に保つため）。
- **純移動の機械照合**: file-slimming の `RustParse.ps1`（項目単位の分解）で、分割前の `git show` と分割後の連結を 1 対 1 に突き合わせられるか。許す差分（`use`・可視性・`impl` の頭の行・doc の 1〜2 行）を吸収できるかを確かめる。

---

### 要件の討議での扱い（2026-10-03）

- 要件側で決着したもの: 論点 6（brief の名指しの読み替え→要件 1.3 に追記）・論点 9（`windows` を使わない子は純粋層へ→要件 5.5 を改訂）・論点 10（steering の列挙も本 spec で合わせる→要件 1.5）・論点 3 の後半（`impl X {` の行は要件 2.2 の許す差分）・論点 5 の要件側（テストからだけ引かれる名前はテストのビルドでだけ束ね直してよい→要件 3.1）。
- 設計へ持ち越すもの: 論点 1（`layout.rs` の置き場所。割るかどうかは討議の議題 1 の裁定に従う）・論点 2（名前）・論点 3 の前半（型の定義の置き場所）・論点 4（`present_frame`）・論点 5 の書き方・論点 7（余白。要件 1.1 の 700 行の目安で足りる）・論点 8（`region.rs`）と「調べもの」の全項目。

## 9. 次の段

- 本書の論点 1〜10 を要件の討議（`kiro-requirements-discussion`）で扱い、要件の改訂が要るもの（例: 要件 5.4 の読み、要件 3.1 と 2.7 の両立）を確定する。
- その後 `/kiro-design areka-P0-emo-text-file-split` で切れ目・名前・可視性・証跡の採り方を設計に落とす。
