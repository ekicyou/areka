# ギャップ分析: areka-P0-anchor-tag-canon

> 2026-10-10・`kiro-validate-gap`。入力は `requirements.md`（確定済み）・`brief.md`・steering（product／tech／structure）。コードは現在の worktree（main `414d43eb` の直後）を Grep／Read で読んだ。**決めごとは書かない**（案と長短だけ）。正典の引用は `requirements.md` の冒頭（2026-10-10 に ukadoc MCP で引いたもの）を使い、ここで新しく引き直していない。

## 1. 要約

- `\q`（選択肢）の道は読み手 → compile → dola の `CueCommand::Choice` → 文字の層の `ChoiceSpan` → 行への写し（`annotate_lines`）→ 当たりの行（`derive_hit_rows` → `ChoiceHitRow`）→ ホバー（`hover_action`／`inject_choice_hover`）→ 押下（`click_selection` → `ChoiceSelection` → mpsc → `choice_drain` → `KanadeMsg::Choice`）→ kanade の帳簿照合とカスケード（`plan_cascade`／`on_choice`／`on_cascade_reply`）まで**全段がそろっていて、箱（シェル内バルーン）も同じ純関数を使う**。アンカーはこの道の大半をそのまま踏める。
- 選択肢と**構造が違う点は 2 つ**。⑴ 選択肢は「ラベルが cue の中に入った 1 つの cue」だが、アンカーは**開きと閉じのあいだに別の cue（文字・改行・装飾）が流れる範囲**。⑵ 選択肢は kanade に「選択待ちの帳簿」があって初めて受理されるが、アンカーには柵も帳簿も無い。この 2 点が、新しく作るもの（範囲の記録・kanade の受理の道）を決める。
- 分かれ目は 3 つの層で独立に選べる: ① 読み手の命令の形（専用の `Instruction` か `GenericCommand` か）／② dola の cue の形（専用の種類 2 つか・`Choice` の相乗りか・汎用キャリア `Custom` か）／③ 範囲の持ち方（`ChoiceSpan` を種類付きに一般化するか・別の `Vec<AnchorSpan>` か）。②は網羅の match（dola・ghost・emo-text の 4 か所）と `consumer_ledger`／`check_script` の診断に影響し、③は後の 3 spec（`range-choice-tag`・`link-context-copy`・`balloon-link-hover`）の使い回しに影響する。
- 要件の多くは既存の部品の**条件を 1 つ足す**ことで満たせる（中断しない＝`selected_now=true`・時間切れを広げない＝`choice_active` を選択肢だけに保つ・箱の結論＝`judge_box_click` の戻りの種類）。新規に書くのは「範囲の記録と行への写し」「kanade の受理（新しいファイル）」「崩れた形の判定（1 か所・compile と `check_script` で共有）」の 3 つが芯。
- 規模 M（13〜16 タスク・brief の見立てどおり）・リスク 中。危ないのは「遅れて届いた選択の棄却」（要件 4.12）の照合の鍵と、kanade の肥大ファイル（`schedule/mod.rs` 955 行・`steady.rs` 950 行）に腕を足す余地。

## 2. 現状の調査（`\q` の道を 1 段ずつ）

### 2.1 読み手（`crates/areka-parsers/src/sakura/`）

| 段 | 今 | アンカーに要ること |
|---|---|---|
| 字句 `lexer.rs` | `\_a[…]` は `Token::Tag{word:"_a", args}`・角括弧の無い `\_a` は `Token::Bare("_a")`（`sakura-bare-tag-lexer` で直し済み・固定長 2〜3 文字） | **変更なし**。 |
| 意味 `decode.rs` | `decode_tag` の `"q"` 腕 → `decode_choice`（第 1＝disp・第 2＝target・以降 references・引数 2 未満で `ArgumentDefaulted`）。`"_a"` の腕は無く `decode_passthrough_tag` → `Raw`＋`ReadNote::UnknownTag`。`decode_bare` も `"_a"` 腕なし → `decode_passthrough_bare` → `Raw`＋`UnknownTag`。`"j"` は `GenericCommand{name: JUMP_TAG_CARRIER="\\j", raw_args}`（汎用キャリアの先例）。 | `"_a"` の腕 2 つ（タグ形・bare 形）。 |
| 型 `model.rs` | `Instruction`（`#[non_exhaustive]`）に `Choice(Choice{disp,target,references})`・`GenericCommand{name,raw_args}`・`Raw`。`ReadNote`（`#[non_exhaustive]`）= `UnknownTag`／`Unclosed`／`ArgumentDefaulted`／`MarkerIgnored`。 | 命令の形（§4.1）。崩れた形を読み手で印にするなら `ReadNote` の追加。 |
| 公開 `mod.rs` | `pub use model::{Choice, Instruction, JUMP_TAG_CARRIER, …}` | 新しい型を出すなら 1 行。 |

`\_a` を素通しと固定している検査（要件 8.2 の対象・**この 3 ファイルだけ**。`decode_tests.rs`・`lexer*_tests.rs`・sakura crate には `\_a` の見本は無い）:

- `parse_bare_tag_tests.rs`: `CANONICAL_BRACKETLESS_SPELLINGS`（:34）に `"_a"` が入り `each_canonical_bracketless_tag_yields_exactly_one_raw` が `Raw` 1 個を固定／`anchor_pair_shows_only_the_body`（:55〜）が `\_a[Hint]アンカー\_aをクリックする。` を `raw,text,raw,text` と固定／`input_of_bracketless_tags_only_yields_no_text`（:127）の列に `\_a`。
- `parse_word_boundary_tests.rs:145`: `parse(r"\_a[ID]") == [raw(r"\_a[ID]")]`。
- `parse_noted_tests.rs:153`: `unknown_tag_is_noted_on_spellings_without_an_arm` に `(r"\_a本文", r"\_a")`／`raw_iff_unknown_tag_or_unclosed`（:263）の見本に `\_a`（こちらは不変条件「Raw ⇔ 印あり」なので、腕を足せばそのまま通る）。

「素通しを前提にした見本を `\_a` 以外の知らないタグに置き換える」は、`\_n`・`\_s`・`\__v` など同じ bare 群の無所有タグ（`COMPAT_ARCHITECTURE.md` §8 の行に 9 タグ列挙）で足りる。

### 2.2 compile（`crates/areka-sakura/src/compile.rs`・411 行）

- `Instruction::Choice` → `emit(scope, offset, 0.0, CueCommand::Choice{id: target, text: disp, references})`。走査後に `has_choice`（`CueCommand::Choice` の有無）で `BarrierKind::WaitForChoice{timeout}` を 1 個 append。`\q` の無い台本は柵を出さない（R2.5）。
- `Instruction::Font` → `CueCommand::command_carrier(FONT_TAG_CARRIER, args)`（汎用キャリアの先例）。`GenericCommand` も同じキャリアへ。
- catch-all `other => debug!("M-boot 外タグを無視")` は `Raw` だけが落ちる（`compile_arm_tests.rs::catch_all_ignored_set_is_raw_only` が固定）。
- **要件 2.5（柵を作らない）は、アンカーを `CueCommand::Choice` に乗せない限り自動で満たす**（`has_choice` は `Choice` だけを数える）。
- compile は台本全体を 1 回だけ走査するので、「閉じの無い開き → 表示の終わりまで」（要件 1.8）は**compile が末尾に閉じを補って警告 1 件**とすれば、空回し（§2.4）と二重にならない（要件 2.10）。

### 2.3 dola（`crates/dola/src/cue/`）

- `CueCommand`（`command.rs:132`・`Serialize/Deserialize`・**`#[non_exhaustive]` ではない**）= `Text`／`Clear`／`Emote`／`Choice{id,text,references}`／`EntityRef`／`Custom{command,params}`／`NewLine`／`BalloonSurface`／`Cursor`／`Wait`／`ClearAll`（10 種）。`command_carrier(name, tokens)`／`as_command_carrier` がキャリアの正準形。
- `CuePlayer`（`runtime.rs:226`）は配送時に `CueCommand::Choice` を `pending_choices` へ積む（`resolve_choice` の照合用）。**`Choice` に相乗りすると、ここと compile の `has_choice` に「アンカーなら除く」の条件が要る**。
- 網羅の match（catch-all 無し・種類を足すとコンパイルが止まって直させる所）: `dola/src/cue/sink.rs::cue_target_of`（`Choice`→`Balloon`）・`areka-ghost/src/sink.rs::command_kind`・`areka-emo-text/src/actor.rs:357`（`apply_cue` の「状態へ渡すだけ」の腕の列）・`areka-emo-text/src/state.rs:592`（`Wait` などの無視の腕の列）。`matches!`／`if let` だけの所（`lookahead.rs`・`areka-seriko/src/actor.rs:489`・`runtime.rs:226`）と `emo2_boot/*_cue.rs`（`Custom` だけを見る）は触らない。dola の網羅檻 `crates/dola/tests/cue/sink_test.rs::cue_target_of_classifies_every_variant` と ghost の `command_kind` の檻も直す。
- **制約**: `dola` は `publish = true`（crates.io へ出している・`tools/crates-io.ps1`）。`CueCommand` に種類を足すのは公開 API の追加（次の版の minor 上げ）。`Custom` に乗せれば dola は無改変。

### 2.4 文字の層（`crates/areka-emo-text/src/`）

- `state.rs`: `ActorTextState{items, reveal, choices: Vec<ChoiceSpan>, glyph_styles, styles, decor}`。`ChoiceSpan{ordinal, id, label, references, glyph_range}`（`glyph_range` は `Glyph` だけを数える序数・互いに素・追記順に単調）。`CueCommand::Choice` の腕（:505）は `text` のクラスタを items へ足し、`push_current_style` で装飾番号を付け、`choices.push`。`Clear`／`ClearAll` で items と同時に初期化。空回し（`quiet`）では `warn!` を出さない。
- `lookahead.rs`（空回し）: `advance_state` は本番と同じ適用。`begins_with(full, arrived)` は字と装飾だけを比べる（brief 2026-10-10 の確認どおり）→ **範囲を状態に足しても崩れない**。
- `choice.rs`（純粋層・744 行）: `annotate_lines(lines, &[ChoiceSpan]) -> Vec<LineChoiceSegment{line_index, ordinal, inline_range}>`（折返しは行ごとに分割・部分リビールは配置済みグリフ数で自然に打ち切り＝要件 2.2／2.3 の芯）／`line_bands`／`derive_hit_rows(lines, segments, mode, region, bands) -> Vec<CanvasHitRow{ordinal, rect}>`／`to_window_physical`／`decorate_canvas(canvas, segments, hover: Option<usize>, ResolvedChoiceStyle, …)`（ホバー行へ塗り＋文字色）／`glyph_cells`（箱の字の矩形）。**どれも `ordinal` と `glyph_range` しか見ない**——`ChoiceSpan` 固有の `id`／`label`／`references` は `actor_present.rs` が `ChoiceHitRow` を組むときに引くだけ。
- `actor.rs`（647 行）: `ChoiceHitRow{ordinal,id,label,references,rect}`・`TextLayerRuntime{choice_hover: HashMap<PlaceKey, Option<usize>>, choice_snapshot: HashMap<PlaceKey, Vec<ChoiceHitRow>>}`・`inject_choice_hover(_at)`（現存 ordinal でなければ縮退）・`choice_hit_rows(_at)`・`choice_active`（＝`choices` が非空）。`Clear`／`ClearAll` で hover と snapshot を原子的に消す（要件 2.7 の「消えたら押せない」はこの仕組みがそのまま効く）。
- `actor_present.rs`（410 行）: 1 コマ＝`spans = actor_state.choices()` → `annotate_lines` → `line_bands` → `decorate_canvas` → `render_styled` → 変化があれば `derive_hit_rows` → `ChoiceHitRow` の snapshot を更新（普通のバルーンも箱 `TextPlace::Box` も同じ関数）。
- `canvas.rs`: `ResidentContent::Choice(ChoiceLineContent{run, segments: Vec<ChoiceRowSegment>, hovered, highlight: Option<HighlightPaint>, band_extent, band_offset})`。非ホバーは `GlyphRun` と同一の素描画。
- `viewbox_draw_render.rs`（571 行）: `ResidentContent::Choice` の行で hover の矩形塗り＋文字色（`highlight_rect`）。先頭注記に「足す予定の spec: anchor-tag-canon（強調の矩形・行の描画）」。
- 下線の描画基盤（`text-decoration-canon`）: `viewbox_draw_decoration.rs::apply_font_ranges` が `glyph_styles`＋`StyleTable` の区間ごとに `SetUnderline(look.underline, range)`。`TextLook.underline: bool`。`look.rs` は `anchor*` キーを `is_unowned`（`starts_with("anchor")`）で所有外として `unowned_vocab()` に保持・`\f[color,default.anchor*]` は `Note::AnchorColorAsDefault`（**この腕は `anchor-style-canon` の仕事**・本 spec は触らない＝要件 5.4）。
- `lib.rs` に「`src/*.rs` の実ファイル集合と mod の一覧を突き合わせる」構造檻（:471）→ 新しいファイルは一覧に登記が要る。

### 2.5 入力（`crates/areka/src/input_events/`）

- `balloon.rs`（433 行）: `ChoiceSelection{id,label,scope,references}`・`BalloonWiring{selection_tx, hover, balloon_hover}`・`hit_choice_row(rows,x,y)`（**逆順走査＝後定義が手前**・半開区間）・`hover_action(active, hit, last) -> HoverAction`・`click_selection(active, rows, x, y, scope)`。
- `balloon_moved.rs`／`balloon_pressed.rs`（先頭注記に「足す予定の spec: anchor-tag-canon」）: 借用規律 ①〜⑤ の薄い結線。押下は左だけ（`left_down`）・右・中は素通し（要件 3.8 は今の形のまま満たす）。押下の末尾で `user_break::on_left_press(world, scope, double_click, selected_now)`——**`selected_now=true` なら中断にしない**＝要件 3.3／3.4 はここへ「アンカーで使った」を真として渡せば足りる。
- `balloon_exit.rs`: 窓から出たときの hover 解除は `choice_active` だけを見る → アンカーの hover も消すには「範囲があるか」へ広げる（要件 3.2）。
- `choice_drain.rs`（422 行）: `ChoiceSelectionInbox` を毎フレーム drain → `to_choice_input` → `KanadeMsg::Choice`。判断なし・全件素通し。
- 箱: `shell_box.rs`（202 行）`judge_box_move -> BoxMove{Outside, OverChoice{name, ordinal}, OverBody}`・`judge_box_click -> Option<ChoiceSelection>`・`judge_box_press(double_click, selected_now, prev_press_selected, talking, no_user_break) -> BoxPressVerdict{ShellOp, ConsumedBySelection, Disabled, Break}`（⑴ この押下が選択 ⑵ 左ダブルクリックでない→シェル ⑶ 直前の押下が選択 ⑷ 話していない→シェル ⑸ 禁止区間 ⑹ 中断）。`shell_box_handler.rs::press_with_point` は `judge_box_click` → `send_selection` → `on_box_press(selected_now)`。**アンカーで使った押下を `selected_now=true` に畳めば `judge_box_press` は無改変で「シェルの操作にしない」になる**（要件 3.7）。結論の名前（`ConsumedBySelection`）の読み替えだけが論点。
- `user_break.rs::on_box_press`／`on_left_press`: 直前の押下の記憶（`prev_press_selected`）を普通の窓と箱で共有。

### 2.6 kanade（`crates/areka-kanade/src/`）

- `msg.rs`（926 行）: `ChoiceInput{id,label,scope,references}`・`KanadeMsg::Choice`・`EventId{Static(&'static str), Choice(String)}`（`Choice` は「選択起源の任意名」＝`On` 始まりを事前登録なしで逐語で送るカテゴリ・構成点は `events::on_choice_named` の 1 点）。
- `actor.rs`（900 行）: `KanadeMsg::Choice(c) => Input::Choice(c)` の写し 1 行・送ってよいか（`is_allowed_event_id || is_allowed_resource_id` ／ `EventId::Choice` は `is_allowed_choice_event`＝`starts_with("On")`）。
- `schedule/mod.rs`（955 行）: `Input::Choice` は `Phase::Steady` のときだけ `steady::on_choice`、他は warn で棄却。`State{choice: Option<ChoiceState{talk_id, candidates, deadline, phase: Waiting|Cascading{choice_id,next}|TimeoutInFlight}>, choice_prev_talk}`。
- `schedule/choice.rs`（376 行・純関数）: `plan_cascade(id) -> Script|Named|Canonical`・`script_body`・`choice_deadline`。
- `schedule/steady.rs`（950 行）: `on_choice`（帳簿なし／talk_id 不一致／段の進行中／候補外を棄却 → 受理 → `Script`／`Named`（`on_choice_named`）／`Canonical`（`on_choice_select_ex` → 残段 `CascadeNext::Select`））・`on_cascade_reply`（`Value` → 新 talk_id で枠を差し替え `[ResolveChoice, StartTalk]`・`NoContent`／`Failed` → 次段または解決のみ）・`on_reply` の**choice 先行アーム**（帳簿が `Cascading` なら origin を見ずに捌く）・origin 別の `Value` 政策（`value_replaces_active_talk(origin)`＝`OnSecondChange` 以外は再生中でも置き換える＝要件 4.8 の「単一の再生枠の規律」）。
- `schedule/events.rs`（793 行）: `ALLOWED_EVENT_IDS`（固定の表・`OnChoiceSelectEx`／`OnChoiceSelect`／`OnChoiceTimeout` を含む）・`on_choice_select_ex(label,id,refs,snapshot)`（Ref0=label・Ref1=id・Ref2..=refs・空なら位置を作らない）・`on_choice_select(id)`・`on_choice_named(id, refs)`（Ref0..=refs・label と id は載せない）。**アンカーの Reference の割付（要件 4.1〜4.6）は、この 3 関数と同じ形で名前だけ違う**（`OnAnchorSelectEx`：Ref0=表示文字・Ref1=ID・Ref2..、`OnAnchorSelect`：Ref0=ID、`On` 始まりは `on_choice_named` を**そのまま使える**）。
- `schedule/balloon_events.rs`: `State.shown: Option<ShownTalk{talk_id, …, timeout_pending}>`＝最後に再生を始めたトークの番号。**バルーンに出ているトークを kanade が知る唯一の手がかり**（要件 4.12 の照合の候補）。
- `mod.rs` の横断判定: `Failed` は `Unloading{Fault}` へ（choice の in-flight だけ免除・:807）。アンカーの GET の `Failed` を「204 と同じ」（要件 4.10）にするには、**同じ免除をアンカーの in-flight にも通す**必要がある。

### 2.7 道具・台帳・文書

- `check_script`（`crates/areka/src/mcp/check_script_judge.rs`・196 行）: `parse_noted` の印 → 診断（`UnknownTag`→`unknown_tag`・`Unclosed`／`ArgumentDefaulted`→`unreadable_argument`・`MarkerIgnored`→`ignored`・未知の印は `_ => continue`）。`GenericCommand`／`Move` は `ConsumerLedger::consumer_of(name, first)` に無ければ `unknown_command`。`Kind`（`areka-mcp/src/tools/check_script.rs:59`）＝`UnknownTag`／`UnknownCommand`／`MissingSurface`／`MissingBalloon`／`UnreadableArgument`／`Ignored` の 6 つ。**要件 6.3（崩れた形の警告）に合う種類は無い**→ 既存の種類へ畳むか新しい種類を足すか（§6）。
- `consumer_ledger.rs`（943 行・1,000 行の目安に近い）: 正準台帳は **24 組**（コードの doc 注記）。`doc/ssp-mcp/areka-tools.md:96` と brief は「23 組」と書く（**文書のずれ**・触るなら合わせる）。`\f`／`\j` は選別子なしで `TextLayer`／`ReadmeSink`。
- 網羅台帳: `sakura-script.toml` は `owner = "areka-P0-anchor-tag-canon"` が **18 行**（根 2 件 `\_a[ID,r2,r3...]`（A1）・`\_a[OnID,r0,r1...]`（A10）＋装飾 16 行）。別名 `\_a[ID]` は `status = "alias"`・`owner = ""`（別名の行は owner を持たない慣例）。`assets.toml` は **43 行**（descript の `anchor.*` 族）。`shiori.toml` の `OnAnchorSelect`／`OnAnchorSelectEx` は `status = "absent"`・`owner = ""`。状態の語彙は `absent`／`alias`／`degraded`／`implemented`／`vocabulary-only`（「ある」＝`implemented`）。
- `COMPAT_ARCHITECTURE.md` §8（横断表・1 行に畳めない spec は詳細台帳へポインタ・選択肢は `doc/choice-cascade-compat.md` に provenance 3 値 `ukadoc`／`ssp_secondary`／`areka_discretion` で記録）。要件 7.1／7.2／7.5 はこの型をそのまま踏める。
- 時間切れの抑止: `emo2_boot/balloon_visibility_wait.rs::observe_suppression` は `choice_active == Some(true)` で抑止（要件 2.6 は**この観測にアンカーを混ぜないこと**を求める）。

## 3. 要件 → 資産の対応表

| 要件 | 既存の資産 | 欠け／未知／制約 |
|---|---|---|
| 1.1〜1.4 読み取り 4 形 | lexer は済み。`decode_tag`／`decode_bare` に腕を足す場所と、`"q"`・`"j"` の先例 | **Missing**: `"_a"` の腕 2 つと命令の形（§4.1） |
| 1.5 区切り・引用の規則 | lexer が全タグ共通 | なし |
| 1.6 知らないタグの印を付けない | 腕を足せば `UnknownTag` は付かない | 検査 3 本の書き換え（要件 8.2） |
| 1.7 あいだの文字・改行・装飾 | 開き／閉じを 0 秒の cue にすれば、あいだの cue は今の道のまま | **Missing**: 範囲の始点・終点を状態で結ぶ（§4.3） |
| 1.8〜1.10 崩れた形 | なし | **Missing**: 開き／閉じの対応を判定する 1 か所（compile か読み手か・`check_script` と共有＝§4.4） |
| 1.11 空の ID | `decode_choice` は引数不足を `ArgumentDefaulted` にする | **Unknown**: `\_a[]` に印を付けるか（付けると `check_script` が `unreadable_argument` を出す） |
| 2.1〜2.3 範囲の当たり・部分リビール・折返し | `annotate_lines`／`derive_hit_rows` がそのまま | `ChoiceSpan` 依存の型をどう広げるか（§4.3） |
| 2.4 話している最中も押せる | hover／click は `talking` を見ない | なし |
| 2.5 柵・時間切れなし | `has_choice` と `pending_choices` は `Choice` だけ | `Choice` に相乗りしなければ自動（§4.2 B の弱点） |
| 2.6 バルーンの時間切れを遅らせない | `observe_suppression` は `choice_active` | **Constraint**: `choice_active` は選択肢だけに保ち、ホバー／押下の活性は別の述語にする |
| 2.7 消えたら押せない | `Clear`／`ClearAll` で spans・hover・snapshot を原子的に消す | なし（範囲も `choices` と同じライフサイクルに置く） |
| 2.8 複数アンカー | ordinal 主キー | なし |
| 2.9 箱も同じ | `present_actor` は箱も同じ関数 | なし |
| 2.10 空回しで二重に出さない | `quiet`・compile は 1 回 | 警告を compile に置けば自動 |
| 3.1／3.2／5.2 ホバーの強調 | `decorate_canvas`＋`ResolvedChoiceStyle`・`inject_choice_hover` | `balloon_exit.rs` の活性判定を広げる |
| 3.3／3.4 押下を中断に重ねない | `on_left_press(…, selected_now)` | `selected_now` にアンカーも含める |
| 3.5 選択肢 → アンカーの順 | `hit_choice_row` は逆順走査（後定義が手前） | **Missing**: 種類を見た順（並べ方で解くか・走査で解くか） |
| 3.6 ほかの押下は不変 | `None` なら今の道 | なし |
| 3.7 箱 | `judge_box_click`／`judge_box_press` | 戻りの種類（`ChoiceSelection` に種類を足すか・別の型か） |
| 3.8 右・中は無視 | `left_down` だけ | なし |
| 4.1〜4.6 Reference・カスケード | `on_choice_select_ex`／`on_choice_select`／`on_choice_named`／`plan_cascade` の形 | **Missing**: アンカー版の構築 2 関数と段の進行の帳簿（`script:` の扱いは Unknown） |
| 4.5 `On` 始まりの受理 | `EventId::Choice`＋`is_allowed_choice_event` | 名前が「選択起源」。読み替えるか新しい variant か |
| 4.7 共通ヘッダ | `ExecutionStatus::derive(snapshot)` | なし |
| 4.8 単一の再生枠 | `on_reply` の `value_replaces_active_talk` | in-flight の帳簿を `on_reply` の先行アームへ足す |
| 4.9 204 で続ける | `NoContent` の腕 | なし |
| 4.10 失敗は 204 と同じ | choice だけ `Failed→Fault` を免除 | **Constraint**: 免除をアンカーの in-flight にも通す（`mod.rs:807` 付近） |
| 4.11 高々 1 回 | 押下は 1 dispatch 1 send。kanade は帳簿で二重を棄却 | in-flight 中の 2 回目をどうするか（Unknown） |
| 4.12 遅れた知らせの棄却 | UI 側は現行 rows だけ読む（消えた後は発行できない）。kanade は `shown.talk_id` を持つ | **Unknown**: 選択の知らせに載せる照合の鍵（§6 研究） |
| 5.1／5.3 既定の下線 | `apply_font_ranges` の `SetUnderline`・`glyph_styles` | **Missing**: 下線を付ける経路（装飾番号に焼くか・描画で区間に掛けるか＝§4.5） |
| 5.4 作者指定は保持だけ | `is_unowned`／`unowned_vocab` | なし（触らない） |
| 6.1／6.2 `check_script` | `parse_noted` の印・`consumer_of` | 命令の形で自動に決まる（§4.1） |
| 6.3 崩れた形の警告 | `Kind` 6 種に該当なし | **Missing**: 種類（既存へ畳む／足す） |
| 7.1〜7.5 互換記録・台帳 | `choice-cascade-compat.md`・§8 の型・台帳の語彙 | 詳細台帳を新設か既存へ節追加か |
| 8.x 決定論テスト | 兄弟 `_tests.rs`・1,000 行の見張り・`lib.rs` 構造檻 | 置き場の空き（§5） |

## 4. 実装の案

### 4.1 読み手の命令の形（`Instruction`）

| 案 | 中身 | 長所 | 短所 |
|---|---|---|---|
| ①-a 専用 variant | `Instruction::Anchor(Anchor{id, references})`（開き）＋ `Instruction::AnchorClose`（閉じ）。`Choice` と対。 | `check_script` は `_ => {}` で素通り＝要件 6.1／6.2 が自動。型で開き／閉じが読める。 | `#[non_exhaustive]` なので下流の catch-all は壊れないが、compile に腕 2 つ。 |
| ①-b `GenericCommand` | `\j` と同じく `GenericCommand{name: "\\_a", raw_args}`。閉じは `raw_args: []`。 | 読み手の変更が最小。 | **`\_a[]`（空 ID の開き）と閉じが同じ `[]` になり区別できない**（要件 1.11 と衝突）。`check_script` は `consumer_of("\\_a", …)` を引くので台帳に行が要る。 |

### 4.2 dola の cue の形（brief の議題 1）

| 案 | 中身 | 長所 | 短所 |
|---|---|---|---|
| ②-A 専用の種類 2 つ | `CueCommand::AnchorBegin{id, references}`・`CueCommand::AnchorEnd`（0 秒）。`Choice`／`Cursor` と同じ「第一級の典型」。 | 文字の層の腕が明快。`consumer_ledger`・`check_script` の台帳に触らない（brief 2026-10-10 の指摘どおり）。`has_choice`・`pending_choices` に条件は要らない。 | 網羅の match 4 か所＋檻 2 本を直す。`dola` は crates.io 公開＝API の追加。`balloon-canon-residue` と `areka-seriko/src/actor.rs` で重なる可能性（brief の並走判定——ただし seriko は `matches!` なので実際には触らない見込み）。 |
| ②-B `Choice` に相乗り | `CueCommand::Choice` に `kind`（選択肢／アンカー）を足す。 | 種類が増えない。 | **形が合わない**: `Choice` は「ラベルを cue の中に持ち、その cue が文字を足す」。アンカーは開き〜閉じのあいだに別の `Text`／`NewLine`／`\f` の cue が流れる（要件 1.7）。相乗りするには compile が範囲の文字を 1 つに畳む必要があり、1 字ずつのリビール・改行・装飾を失う。さらに `has_choice`・`pending_choices`・`choice_active`・`observe_suppression` の全部に「アンカーなら除く」が要る。serde のワイヤ形も変わる。**推奨しない**（長短の記録として残す）。 |
| ②-C 汎用キャリア | `CueCommand::command_carrier("\\_a", tokens)`（`\f`／`\j` の先例）。開きは `[id, r2, …]`、閉じは空。 | dola 無改変。文字の層は名前で自己選別（`\f` と同じ腕）。 | ①-b と同じ**空 ID と閉じの衝突**（避けるには先頭に `open`／`close` の印を入れる等、正準形から外れた約束が要る）。`consumer_ledger` に 1 行（943 行のファイル・24→25 組・`areka-tools.md` の数も直す）。ghost／seriko にも配送されて名前で捨てられる（`\f` と同じで害は無い）。steering の「`\!` は汎用キャリア 1 本」は `\!` の話で、`\q`→`Choice`・`\_l`→`Cursor` の先例もある＝どちらも規律の内。 |

①と②は独立に選べる（①-a × ②-C も可。その場合 `check_script` は台帳を引かない）。

### 4.3 範囲の持ち方（後の 3 spec の使い回し）

| 案 | 中身 | 長所 | 短所 |
|---|---|---|---|
| ③-i `ChoiceSpan` を一般化 | `RangeSpan{kind: Choice|Anchor, ordinal, id, label, references, glyph_range}` の 1 列（`ordinal` は共通の通し番号）。`annotate_lines`／`derive_hit_rows`／`decorate_canvas`／`ChoiceHitRow` に `kind` が乗る。 | **`range-choice-tag`（範囲の選択肢）・`link-context-copy`・`balloon-link-hover` がそのまま乗れる**（brief 2026-10-05 の「範囲の当たりは選択肢にも使い回せる形」）。hover／hit の配線は無改変で両方を扱う。 | `choice_active`（柵・抑止）は `kind == Choice` で絞る必要があり、述語が 2 つに割れる（§3 要件 2.6）。`ChoiceSpan` を使う既存テストの名前替え。`label` は閉じまで決まらない（開きで空・閉じで埋める）。 |
| ③-ii 別の `Vec<AnchorSpan>` | `anchors: Vec<AnchorSpan{ordinal, id, references, glyph_range(開き〜閉じ), label(確定時)}>` を `choices` の隣に置き、純関数は「`(ordinal, glyph_range)` を返す小さな trait／引数」で両方を受ける。 | 選択肢の型に触らない（`choice_active` も不変）。 | 行への写し・hit rows・hover の注入を 2 回呼ぶか、呼び手で結合する。後の spec が「2 列を結合する」約束を引き継ぐ。ordinal の空間が 2 つになり、hover の `Option<usize>` が種類を持つ必要（`inject_choice_hover` の鍵を広げる）。 |

どちらでも、開きの時点で `glyph_range.start` を今のグリフ数で決め、閉じで `end` を確定する（閉じが無いときは compile が補う＝§2.2）。開いている間の表示（要件 2.1／2.2）は `end` を「今のグリフ数」で読めば部分リビールと同じ仕組みで満たせる。

### 4.4 崩れた形（1.8〜1.10）の判定の置き場

| 案 | 中身 | 長所 | 短所 |
|---|---|---|---|
| ④-a 読み手の印 | `ReadNote` に `AnchorUnclosed`／`AnchorReopened`／`AnchorStrayClose` を足し、`decode_noted` が開き／閉じの対応を数える。 | `check_script` は印の `match` に腕を足すだけ。 | 読み手は「転記層・木の組み立ては下流」（memory `areka-parser-transcribes-tree-downstream`）。開き／閉じの対応は木の組み立てに近い。 |
| ④-b 共有の純関数 | `areka_sakura` に `pair_anchors(&[Instruction]) -> (命令の列に補う閉じ, Vec<AnchorIssue{span, kind}>)` のような純関数を置き、compile（`warn!` 1 件ずつ）と `check_script_judge`（診断）が同じ関数を呼ぶ。 | 判定が 1 か所（「本番と同じ内容の警告」＝要件 6.3 の構造保証）。読み手は転記のまま。 | `check_script_judge` が `Read` の列から `Instruction` の列を作り直す 1 手間。`Kind` に種類を足すか既存へ畳むかは残る。 |

### 4.5 既定の下線（5.1／5.3）

| 案 | 中身 | 長所 | 短所 |
|---|---|---|---|
| ⑤-a 装飾番号に焼く | 開き〜閉じのあいだ `push_current_style` が `current` に下線を強制した見た目を `intern` する。 | 描画は無改変（`apply_font_ranges` が `SetUnderline`）。縦書きの位置も既存どおり（要件 5.3）。 | 作者の装飾状態（`Decoration.current`）に一時の上書きを混ぜる。`\f[underline,false]` を範囲の中で書いたときの読みが要る。`anchor-style-canon` が 3 状態の色・形へ差し替えるときに、この焼き込みを剥がす。 |
| ⑤-b 描画で区間に掛ける | `ChoiceLineContent` に相当する住人へ「下線を引く区間」を持たせ、`viewbox_draw_render.rs` が hover の文字色と同じ所で `SetUnderline(true, range)` を掛ける。 | 作者の装飾状態に触らない。`anchor-style-canon` が形（square／underline／none）を差し替える場所がここに揃う。 | 描画の腕が増える（571 行に余地はある）。ダーティ帯の広げ方の確認。 |

### 4.6 kanade の受理の道

- 新しい入力: `KanadeMsg::Anchor(AnchorInput{id, text, scope, references})`（`msg.rs` 926 行→**新しいファイル**に型を置き `pub use`）・`actor.rs` に写し 1 行・`schedule/mod.rs` に腕 1 本（Steady だけ）・**`schedule/anchor.rs`（新・`choice.rs` に倣う）** に `plan_anchor_cascade`（`Named`／`Canonical`。`script:` は ukadoc の `\_a` に無い→ Unknown）・`events.rs` に `on_anchor_select_ex`／`on_anchor_select` の 2 関数（`ALLOWED_EVENT_IDS` に 2 名）。`On` 始まりは `on_choice_named`＋`EventId::Choice` をそのまま使う（名前の読み替え）か、`EventId::Anchor` を足すか。
- in-flight の帳簿: `State.anchor: Option<AnchorCascade{id, next}>` を `choice` の隣に置き、`on_reply` の先行アームと `mod.rs` の `Failed` 免除に同じ照合を足す。`Value` は `on_reply` の置き換えアーム（`value_replaces_active_talk`）へ流せば要件 4.8 が自動。
- UI 側: `ChoiceSelection` に種類を足して同じ mpsc・`choice_drain` を通す（drain は `KanadeMsg::Choice`／`Anchor` へ振り分ける 1 分岐）か、別の channel。前者が最小。

## 5. 規模とリスク

- **規模: M**（brief どおり 13〜16 タスク）。読み手 2 腕＋検査 3 本／compile 2 腕＋崩れた形の判定／dola（②-A なら 2 種類＋match 4 か所＋檻 2 本）／文字の層（範囲の記録・行への写し・hit rows・hover・下線・新ファイル 1〜2）／input_events（活性の述語・種類の振り分け・判定の順・箱・exit）／kanade（型・腕・新ファイル・events 2 関数・in-flight・Failed 免除）／`check_script`／台帳 3 本と互換記録／実機 1 周。
- **リスク: 中**。
  - 要件 4.12 の照合の鍵が未決（§6 研究 1）。決め方しだいで `ChoiceSelection`／`AnchorInput` の形と kanade の判定が変わる。
  - kanade の肥大ファイル（`schedule/mod.rs` 955・`steady.rs` 950・`msg.rs` 926・`actor.rs` 900・`consumer_ledger.rs` 943）は 1,000 行の見張り（`file_length_guard_test.rs`）にかかりやすい。腕 1 本・写し 1 行に留め、本体は新しいファイルへ。
  - `dola` の公開 API（②-A）。
  - 並走の重なり（brief 2026-10-10）: `choice-ranges-one-function`（`choice.rs`・`actor_present.rs`・`actor.rs`）は③の形を変える相手＝**前に済ませるのが望ましい**。

## 6. 設計への申し送り（研究項目）

1. **遅れた選択の照合の鍵（要件 4.12）**: UI 側は `Clear`／`ClearAll` で snapshot を消すので「消えた後に発行」は起きないが、発行から kanade の処理までの間に台詞が置き換わる窓は残る。選択肢は `ledger.talk_id` で照合する。アンカーには帳簿が無く、しかも `TalkDone` の後もバルーンが出ている間は押せる（Steady{talk: None} でも有効）。候補: ⒜ `balloon_events::ShownTalk.talk_id`（最後に再生を始めたトーク）と、文字の層が cue から知れる何か（`TalkCue` は talk_id を運ばない→ 空回しの `ClearCounts` の番号など）を突き合わせる／⒝ UI 側（drain）で「発行時の世代」を付け、同一フレームの drain で古ければ捨てる（kanade に届かない）／⒞ kanade では照合せず、`steady_talk_replace` の規律に任せる（要件 4.12 を UI 側の棄却で読む）。設計で 1 つに決める。
2. **in-flight 中の 2 回目の押下（要件 4.11）**: 選択肢は `choice_rejected_busy`。アンカーは「棄却」か「待たせる」か。ukadoc は沈黙（互換記録の対象候補）。
3. **`script:` 始まりの ID**: ukadoc の `\_a` には無い形。`plan_cascade` を共用すれば `Script` に落ちる。逐語で `OnAnchorSelectEx` に流す（Canonical 扱い）か、選択肢と同じく台本として走らせるか。
4. **`\_a[]` の印**（要件 1.11）: `ArgumentDefaulted` を付けると `check_script` が `unreadable_argument` を出す。付けないなら何も出ない。
5. **`check_script` の `Kind`**（要件 6.3）: 既存 6 種に畳む（`unreadable_argument` か `ignored`）か、新しい種類（例: `broken_range`）を `areka-mcp` に足すか。文言の正本は `doc/ssp-mcp/areka-tools.md`。
6. **判定の順（要件 3.5）**: `hit_choice_row` の逆順走査は「後定義が手前」。種類で順を付けるには、snapshot をアンカー→選択肢の順に並べる（逆順走査で選択肢が勝つ）か、走査を種類ごとに 2 段にするか。前者は「ordinal 昇順×行昇順」の不変条件の注記を書き換える。
7. **互換記録の置き場**（要件 7.1／7.2／7.5）: `doc/choice-cascade-compat.md` にアンカーの節を足す（同じ provenance 3 値）か、`doc/anchor-compat.md` を新設して §8 から引くか。
8. **文書のずれ**: `areka-tools.md:96`「今 23 組」と `consumer_ledger.rs` の doc「24 行」。②-C を採るなら必ず触る。採らなくても気付いた以上は直す候補（spec の外なら `/kiro-discovery` へ）。
9. **`dola` の版**: ②-A を採るなら `CueCommand` の追加が公開 API の変更になることを `release-cycle` の記録に残すかどうか。

## 7. 要件討議へ回す議題（答えで作業が変わるもの）

1. dola の cue の形: 専用の種類 2 つ（②-A・網羅の match を直す・`dola` の API 追加）か、汎用キャリア（②-C・`consumer_ledger` と `areka-tools.md` に 1 行・空 ID と閉じの区別に印が要る）か。`Choice` への相乗り（②-B）は形が合わないので除く案。
2. 範囲の持ち方: `ChoiceSpan` を種類付きに一般化（③-i・後の 3 spec が同じ列に乗る・`choice_active` を種類で絞る）か、別の列（③-ii・選択肢の型は不変・結合は呼び手）か。
3. 既定の下線の経路: 装飾番号に焼く（⑤-a・描画無改変）か、描画で区間に掛ける（⑤-b・`anchor-style-canon` の差し替え点が揃う）か。
4. 崩れた形の判定の置き場: 読み手の印（④-a）か、compile と `check_script` が共有する純関数（④-b）か。あわせて `check_script` の種類（研究 5）。
5. 遅れた知らせの照合の鍵（研究 1）と、in-flight 中の 2 回目の扱い（研究 2）。
6. `script:` 始まりの ID の読み（研究 3）と `\_a[]` の印（研究 4）。
7. `On` 始まりの受理に `EventId::Choice`（「選択起源」）をそのまま使うか、`EventId::Anchor` を足すか（`actor.rs` の `allowed` 判定・`"OnChoiceEvent"` のログ名に影響）。
