# ギャップ分析: areka-P0-balloon-text-bugs

- 実施: 2026-10-16（`/kiro-validate-gap`・ブランチ `claude/areka-p0-balloon-text-bugs-bd1052`・基点 `398b76e9`＝main `1e6590d8` の上に spec の初期化 1 コミット）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C7 と「文字とバルーン」の列）
- 方針: 決めはしない。今のコードの読みから、直す場所の候補・それぞれの得失・設計で確かめることを並べる。行数は 2026-10-16 の実測。

## 1. 分析の要約

- **件 1 の「選択待ちが生きているか」の正本は kanade の選択帳簿（`State.choice`・3 段＝`Waiting`／`Cascading`／`TimeoutInFlight`）で、UI からは見えない。** UI の側で今いちばん近い合図は「トークの終わり」（`TalkEnded`）。sakura は選択待ちのバリアの間は再生を終えない（`drive.rs` の `settle_after_tick`）、kanade は中断・置き換え・終了で帳簿を必ず消す（`clear_choice_ledger`）＝**「選択が生きている ⇒ トークが生きている」が今のコードで成り立つ**。時間切れの計測はトークの終わりが届いてから立つ（`balloon_visibility_wait.rs` の `decide_timeout`）ので、**「選択肢の行が残っている」抑止が効くのは、選択が済んだ後だけ**＝今の抑止は件 1 の不具合の場面でしか働いていない。
- **済んだ選択肢の行の「光らない・押せない」（要件 1.10〜1.12）は、入力の 4 か所（`balloon_moved.rs`・`balloon_pressed.rs`・`balloon_exit.rs`・`shell_box_handler.rs`）がそろって読む `choice_hit_rows`（文字の層の当たり行の写し）で 1 か所に集約できる。** 写しには種類（`kind`）が入っているので、選択肢だけを外しアンカーを残せる。ただし文字の層は「済んだ」を知らない＝**合図をどの線で文字の層へ届けるか**が件 1 の設計の芯。線をまたぐと届く順が崩れ、次のトークの選択肢を誤って「済み」にしうる（§3.1.4）。
- **件 2 は `actor_present.rs` の `present_actor` の「初めての装着」の枝が、普通のバルーンと箱の両方が通る 1 か所。** ここで 0 寸を見分けて装着を呼ばなければ、ERROR 2 行は消える。ただし WARN は `region.rs` の `TextRegion::resolve` が毎フレーム出している（再追従の検査が毎フレーム解き直すため）＝**記録を「状態の変わり目の 1 回」にするには WARN の置き場所も動かす要がある**。先例は同じ `resolve` から登録口へ移した「折返し基準が描画範囲の外」の警告（`actor_attach.rs` の `warn_coarse_wrap_threshold`）。
- **件 3 は下線を D2D の `DrawTextLayout` が `SetUnderline` の真偽だけで描いており、線の位置を areka が持たない。** 3 案（面を広げる／下線を自分で描く／入る行の数え方）はどれも成り立つが、**面を広げる案だけが「行の箱からはみ出すインク」（下線と、和文フォントの descent）を同時に救う**見込みで、自分で描く案は完了 `text-decoration-canon` の要件 5.7 を改め、数え方の案は入る行の数（実機の 4 行→3 行）を変える。どの案もはみ出しの量を font metrics から得る調べが要る。
- 規模の見立て: 件 1＝M（合図の線の選びで 3〜7）・件 2＝S（4〜6）・件 3＝M（4〜7・案で幅）＝合計 11〜20 のまま。危険度は件 1＝中（届く順）・件 2＝低・件 3＝中（案の選びと縦書き・拡大率）。

## 2. 今のコードの地図（要件 → 資産）

### 2.1 件 1（要件 1・4.1）

| 要件 | 今の資産 | 状態 |
|---|---|---|
| 1.1 選択待ちの間は隠さない | 計測はトークの終わり（`TalkLifecycleSignal::TalkEnded`）の後にしか立たない（`balloon_visibility_wait.rs` `decide_timeout`・`effective_display_end` が `TalkEnd::NotYet` で `None`）。sakura は `WaitingForChoice` の間 `is_completed()` が偽＝受け口が落ちない＝`TalkEnded` が出ない（`areka-sakura/src/drive.rs` `settle_after_tick`）。`Cascading`（選んだ応答待ち）と `TimeoutInFlight`（`OnChoiceTimeout` の応答待ち）の間も旧トークはバリアで止まったまま（`ResolveChoice`／`CancelChoice` は応答の後にだけ出る・`schedule/steady.rs`） | **既に満たす**（抑止の数え方と無関係に）。ただし固定する檻は無い＝**Missing（検査）** |
| 1.2〜1.4 済んだら時間切れで隠し `OnBalloonTimeout` | `observe_suppression` が `observed.choice_active == Some(true)` で抑止を立てる（`balloon_visibility_wait.rs:284`）。その源は `TextLayerRuntime::choice_active`＝「スコープのどこかに種類が選択肢の範囲の記録があるか」（`areka-emo-text/src/actor.rs:479`）。範囲の記録は `Clear`／`ClearAll` でしか消えない | **Missing**（不具合の本体） |
| 1.5 行が残っていても理由に数えない | 同上 | **Missing** |
| 1.6 ドラッグ・滞在は今までどおり | `SuppressionKinds { dragging, hover, choice }`（`balloon_visibility.rs:163`） | 既存のまま使える |
| 1.7 新しい台詞は今まで通り | `TalkStarted` で計測を捨て状態を戻す（`balloon_visibility_decision.rs:120`） | 既存 |
| 1.8 待ちの長さ・起点は今のまま | `effective_display_end`（占有終端と止まった時刻の早い方）・抑止が解けたら今を起点に測り直す（`released` の枝） | 既存。**ただし起点の選び方で振る舞いが分かれる**（§3.1.5） |
| 1.9 選択肢の時間切れでは `OnBalloonTimeout` を送らない | kanade の `on_tick` → `OnChoiceTimeout`（`TimeoutInFlight`）。バルーンの側は関与しない | 既存 |
| 1.10 済んだ行を光らせない | 光るのは入力が `inject_choice_hover` で入れた番号を `present_actor` が `decorate_canvas` に焼くとき。入力は `choice_hit_rows`（`present_actor` が提示の成功時に作る写し `choice_snapshot`）で当たりを判定 | **Missing**（「済んだ」の印が無い） |
| 1.11 押しても選択の知らせを送らず警告も出さない | 押下は `choice_hit_rows` の当たり → `ChoiceSelectionInbox` → kanade `on_choice` → 帳簿が無ければ `choice_rejected_no_wait`（warn・`steady.rs:245`）。選んだ応答待ちの二度押しは `choice_rejected_busy`（warn） | **Missing**。当たり行から外せば kanade まで届かず警告も出ない |
| 1.12 見た目は今のまま | 選択肢の字・色・印は範囲の記録（`ActorTextState::choices`）と `annotate_lines` が決める | 範囲の記録を消さずに「済み」を足す形なら保てる |
| 1.13 箱でも同じ | `choice_active` はスコープの全場所（箱を含む）を見る。箱の入力は `shell_box_handler.rs` が `choice_hit_rows_at` を読む | 同じ 1 か所の直しで箱にも効く |
| 4.1 決定論のテスト（普通と箱） | 時間切れの表検査（`balloon_visibility_timeout_suppression_cases.rs` 687 行・`_tests.rs` 490 行）・箱の観測（`balloon_visibility_phase_box_tests.rs` 413 行）・入力の檻（`balloon_pointer_handler_tests.rs`・`balloon_hover_flag_tests.rs`・`shell_box_handler_tests.rs`）・文字の層（`actor_choice_contract_tests.rs` **991 行**・`actor_clear_atomicity_tests.rs` 478 行・`actor_route_tests.rs`） | 足す場所はある。`actor_choice_contract_tests.rs` は満杯＝新しい兄弟ファイルが要る見込み |

**既存の檻で、直しによって意味が変わるもの**（陳腐化か更新かの判断が要る）:
- `balloon_visibility_timeout_suppression_cases.rs` の ⑻ の行（`choice: true` の抑止を表で固定）。合成の観測で「トークの終わりの後に選択肢が表示中」を作っている＝直し方 A（§3.1.2）では意味を失う。
- `actor_choice_contract_tests.rs::choice_active_reflects_span_set_presence`・`actor_route_tests.rs::choice_active_sees_every_place_of_the_scope`・`input_events/*_tests.rs` の「前提: choice_active=true」系＝直し方 B（文字の層で問いの意味を変える）では前提の書き方が変わる。
- 完了 `balloon-visibility` の要件 5.4（選択肢の表示中を抑止に数える・「外部の照会を消費」）は、直し方 A なら事実上外れ、B なら「表示中」の意味が「選択が生きている」に改まる＝**完了 spec の決めの改め**として記録が要る（steering の「裁定で要件を改訂したら design・境界節まで追随」）。

### 2.2 件 2（要件 2・4.2）

| 要件 | 今の資産 | 状態 |
|---|---|---|
| 2.1 潰れたら供給面を作らず描かない | `present_actor` の初回装着の枝（`actor_present.rs:118〜`）が `physical_extent`（`(v × k).ceil() as u32`＝負は 0 に飽和）で寸を出し、0 のまま `TextSurface::attach`／`attach_window_child` → `create` → `create_composition_swap_chain`（`surface.rs:361`）→ `DXGI_ERROR_INVALID_CALL` | **Missing**（0 寸の分岐が無い） |
| 2.2 毎フレーム試みず ERROR 0 行 | 失敗で `surfaces` に入らない → 次フレームも同じ枝 → `present_frame` の `Err` → `scale_text.rs:302` の `error!`「present_frame が失敗（…次フレーム再試行…）」 | **Missing** |
| 2.3 入ったときに警告 1 回 | `region.rs:270` の `warn!`「解決後の validrect が退化している」は `TextRegion::resolve` の中＝**毎フレーム**（再追従 `refresh_actor_binding` が churn の判定のために毎フレーム解き直す・同じ関数の doc に実測の経緯） | **Constraint**：置き場所を変えないと 1 回にならない |
| 2.4 戻ったら 1 回記録し次から描く | 戻り＝バルーン・面の切り替えで `register_actor` が新しい領域を入れる → `surfaces` が無い → 次の `present_actor` で普通に装着 | 描く側は既存で足りる。「戻った」の記録が **Missing** |
| 2.5 隠す・出すで記録を増やさない | 隠しても `present_frame` は文字の残る場所を毎フレーム回る（`!items().is_empty() || surfaces.contains_key`） | ガードを装着の枝に置けば満たす |
| 2.6 ほかのバルーンを妨げない | `present_frame` は場所ごとに続ける（最初の失敗だけ返す） | 既存 |
| 2.7 箱でも同じ | 箱も同じ `present_actor` の枝を通る（`TextPlace::Box` は `attach_window_child`）。箱の登録は `actor_box.rs`（`register_box`・`layout_input.insert` 427 行目） | 枝で止めれば同じ直しが効く |
| 2.8 装着の記録の文を変えない | `info!`「テキスト供給面を予約スロットへ装着した（ActorRender 不在時のみ・以降は Present のみ）」（`actor_present.rs:224`） | 触らない |
| 4.2 決定論のテスト | `actor_region_warn_tests.rs` **894 行**（足しにくい）・`actor_tests.rs` 178 行・`actor_box_present_tests.rs` 345 行・`region_tests.rs` 479 行 | 置き場所はある |

**見つけた穴（brief に無い）**:
- **一度装着した後に潰れた場合の古い絵**: 再追従（`refresh_actor_binding`）は `self.surfaces.remove(&place)` で描画資源を捨てるが、`TextSurface::attach` が差し込み口の entity に入れた `VisualGraphics`・`Arrangement` は外さない（`surface.rs:330`）。今は次フレームの装着がそれを上書きしている。**0 寸で装着を止めると、上書きされずに古い文字が差し込み口に残る見込み**＝要件 2.1「文字を描かない」に触れる。箱は登録を外すとき `despawn_window_child` で片付けるが、再追従の経路では片付けない。**Research Needed**（差し込み口の中身の扱い）。
- `region.rs` の doc は「退化した validrect の `warn!` はそのまま残す（記録の無い縮退経路を作らない）」と過去の決めを書いている＝本 spec が改める決めとして doc ごと直す要がある。

### 2.3 件 3（要件 3・4.3〜4.5）

| 要件 | 今の資産 | 状態 |
|---|---|---|
| 3.1〜3.3 最下行の下線（フォント・大きさ・拡大率によらず） | 下線は `viewbox_draw_decoration.rs:187` の `layout.SetUnderline(look.underline, run.range)` だけ。線は D2D の `DrawTextLayout`（行ごと・`viewbox_draw.rs`／`draw.rs:533`）が DirectWrite の既定位置に描く。文字の面は validrect ちょうど（`actor_present.rs:123` の `physical_extent`）。描き替えの範囲は面の大きさで切る（`viewbox_diff.rs:405` `expand_guard_clamp`） | **Missing** |
| 3.4 縦書きの端の列 | 縦書き（`VerticalRl`）の行送りの遠い側は**左**＝面を広げるなら原点（`physical_offset`）がずれる | **Unknown**（端の列で同じ形が起きるか未確認） |
| 3.5 箱でも同じ | 箱の面も同じ枝で作る（`attach_window_child`）。箱は字の矩形でポインタを受ける（`glyph_cells` → `set_hit_cells`） | 面を広げる案では当たりの写しとの整合を確かめる |
| 3.6 上の行の見た目を変えない | 途中の行の下線は次の行の帯に落ちて見えている | 案ごとに守り方が違う（§3.3） |
| 3.7 行の位置・入る行の数の変化は最小に・記録 | あふれは行の矩形（文字の大きさの箱）の遠端で数える（`layout.rs:403` `visible_window`・境界ちょうどは収まる） | 数え方の案だけが触る |
| 3.8 打ち消し線は今のまま | `SetStrikethrough`（行の中に出る） | どの案でも当たらない見込み |
| 3.9 押せる範囲は今のまま | 当たりは描画と別に作る（`derive_hit_rows`・`to_window_physical`） | 面を広げても領域から作るので変わらない見込み |
| 4.3 最下行の下線が面の中に描かれる檻 | 下線の画素を見る檻は 0 本（`viewbox_draw_decoration_tests.rs` 559 行は区間の真偽の檻） | **Missing**。GPU の読み戻しの檻（`viewbox_draw_png_dump_tests.rs` 型）で実 GPU |
| 4.5 互換の記録 | `doc/COMPAT_ARCHITECTURE.md` §8（122 行目〜）・完了 `text-decoration-canon` 要件 5.7（線を描き分けない） | 案次第で 5.7 の改め |

**見つけた穴（brief に無い）**:
- 面が validrect ちょうどなら、**最下行の和文・欧文の descent のインク**も同じ形で切れている見込み。`choice.rs` の `line_bands` の doc に「em ボックス丈で切ると和文フォントの descent インクが帯の外へ出る（実機不具合「選択肢の文字の下が切れる」の真因）」とあり、行の箱（DirectWrite の行の丈＝`GlyphMetrics::line_box_height`）は文字の大きさの箱より背が高い。下線の位置（行の上端から 33 画素）がその内側にあるなら、`g`・`y`・`ぁ` 等の下も面の端で欠けうる。**Research Needed**（撮り直しのときに一緒に見る）。

## 3. 直し方の候補

### 3.1 件 1

#### 3.1.1 合図の候補（UI が「選択が済んだ」を知る手段）

| 合図 | どこから | 正確さ | 届く順 |
|---|---|---|---|
| S1 トークの終わり（`TalkEnded`・可視性の線） | `BalloonLifecycleSink` の `Drop`（`emo2_boot/talk_lifecycle.rs:240`） | 済んだ後に必ず来る。選んだ直後ではなく、`ResolveChoice` → 残りの合図の再生 → 終わりの後 | 可視性の線の中では「前のトークの終わり → 次のトークの始まり」が保たれる。**文字の線（cue）とは別の線** |
| S2 トークの終わり（`NoUserBreakSignal::TalkEnded`・入力の線） | `NoUserBreakCueSink` の `Drop`（`user_break_cue.rs:120`）。入力の側は既に `fold_talking` で「話している最中か」を畳んでいる（`shell_box.rs:147`・選択待ちの間も最中） | S1 と同じ | 入力の線の中では保たれる。文字の線とは別 |
| S3 文字の線にトークの終わりを流す（新設） | `ClockedTextSink<EmoTextSink>`（`talk_clock.rs:91`）か `EmoTextSink`（`sink.rs`）を、上の 2 本と同じ「複製ごとに `started`・`Drop` で 1 回」の形にして `TextMsg` に 1 つ足す | S1 と同じ | **文字の合図と同じ線の FIFO**＝前のトークの終わりは次のトークの `ClearAll`／`Choice` より必ず前 |
| S4 押した瞬間（UI 自身） | 押下が `ChoiceSelectionInbox` へ選択を積んだ時点（`balloon_pressed.rs`・`shell_box_handler.rs`） | 「選んだ」だけ。時間切れ・中断は分からない | UI 内で即時 |
| S5 kanade の知らせ（新設） | 帳簿を消す 1 か所（`clear_choice_ledger` と `on_choice`／`on_timeout_reply` の消す点）から `KanadeNotice`（kanade → UI の線・`change.rs:65`）に 1 つ足す | **正本そのもの**（`Cascading` の終わりも分かる） | 文字の線とは別。kanade の公開語彙が増える |

#### 3.1.2 案 A — 時間切れの数え方だけを直す（可視性の側）

- 中身: `observe_suppression`（`balloon_visibility_wait.rs:271〜289`）から選択肢の項を外す、または「トークの終わりが未着の間だけ数える」に絞る（§1 の不変から、どちらも同じ振る舞い）。
- 触る: `balloon_visibility_wait.rs`（289 行）・`balloon_visibility.rs`（`SuppressionKinds.choice` を残すか消すか）・`balloon_visibility_phase.rs` の観測（832 行・消すなら 1 行）・表検査の ⑻。
- 満たす: 1.1〜1.9・1.13（箱は同じ観測）。**満たさない: 1.10〜1.12**（済んだ行は光り、押すと kanade が warn で捨てる）。
- 利点: 差分が最小・emo-text に触れない（C7 の約束と `range-choice-tag` との重なり 0）。
- 欠点: 要件 1.10〜1.12 のために別の直しが要る（→ 案 C）。完了 `balloon-visibility` 要件 5.4 を外す記録が要る。

#### 3.1.3 案 B — 文字の層に「済み」の印を持たせる（根の 1 か所）

- 中身: 範囲の記録に「済み」を足す（範囲ごとか場所ごと）。済みの合図で、①選択肢の範囲を済みにし、②当たり行の写し（`choice_snapshot`）から済みの選択肢を外し、③その行を指す強調（`choice_hover`）を外す——`apply_cue(Clear)` が今している「範囲・写し・強調を同時に無効にする」（`actor.rs:323〜337`・要件 5.2 の原子性）と同じ形。`choice_active` は「済みでない選択肢があるか」で答える。`present_actor` の写しの作り直しも済みの選択肢を外す。
- 触る: `actor.rs`（663 行）・`state.rs`（713 行・範囲の記録の持ち主なら）・`actor_present.rs`（411 行）・合図の線（S3 なら `sink.rs` 536 行と `talk_clock.rs`、ほかの線なら `emo2_boot` の結線）。入力の 4 か所と可視性は**直さずに**直る（全部が `choice_active`／`choice_hit_rows` を通る）。
- 満たす: 1.1〜1.13 を 1 か所で。見た目（1.12）は範囲の記録を消さないので保てる。
- 欠点: 文字の層の公開の問いの意味が変わる（`choice_active` を読む `hover_inject.rs` も含め、前提を書いた檻の書き直し）。`range-choice-tag`（`actor.rs`・`state.rs`・`input_events/balloon_pressed.rs`・`shell_box_handler.rs`）と重なる＝本 spec が先（開発者の裁定どおり）。

#### 3.1.4 届く順の罠（案 B と案 C の芯）

文字の層の更新は UI のメッセージの汲み出し（`spawn_ui` の handler）で、可視性の線（S1）と入力の線（S2）は毎フレームの段で汲む。**別の線の合図で文字の層に「済み」を付けると、同じフレームの中で次のトークの `ClearAll`＋`Choice` が先に文字の層へ入っていた場合に、新しい選択肢を誤って済みにする**（選択への応答が新しい台詞を始める場合＝`[ResolveChoice, StartTalk]` は間を置かずに続く）。逃げ方は 3 つ:
- S3（文字の線に終わりを流す）＝同じ FIFO なので構造で起きない。先例の形（`BalloonLifecycleSink`・`NoUserBreakCueSink` の `Drop`）がそのまま使える。`EmoTextSink` は今 `#[derive(Clone)]` で一括の `BootCueSink` 実装に乗っている＝複製の境界を型で持たせるなら手書きの `Clone` が要る（emo-text は `areka-ghost` に依らないので、`BootCueSink` の手書きは areka 側の包み `ClockedTextSink` が担う形が素直）。
- 範囲に「どのトークの範囲か」を持たせ、トークの番号つきで済ませる（文字の層はトークの番号を今知らない＝新しい運び）。
- 消去の回数（`clear_count`）を世代として使う（内容の無いトークは `ClearAll` を出さない＝`areka-sakura/src/compile.rs:330`。世代が進まないトークがある）。

#### 3.1.5 起点の選びで振る舞いが分かれる（要件 1.8 の読み）

- 選択肢の時間切れ（既定 30 秒・`msg.rs:503`）は表示の終わりから測る。バルーンの時間切れ（既定 30 秒）も表示の終わり（占有終端・止まった時刻の早い方）から測る。選択の待ちの間、占有終端はほとんど進まない（受け口はバリアを受け取らない・`talk_lifecycle.rs` の doc）。
- そのため、**長く待ってから済んだ選択では、トークの終わりが届いたフレームで満了予定がもう過ぎている**＝ほぼ即座に隠れて `OnBalloonTimeout` が出る（例: 選択肢の時間切れ → `OnChoiceTimeout` が 204 → 数十 ms で隠れる）。
- マウスで選んだときはポインタがバルーンの上に居る＝滞在の抑止が効き、離れたときに「今から測り直し」（要件 5.3 の解除の枝）になる＝自然。
- 案 B で「選択が生きている」を抑止のまま残すと、**済みの印とトークの終わりのどちらが先に可視性へ届くか**で「すぐ隠れる」と「今から 30 秒」に分かれる（解除の枝は計測が成り立たない間に立つと何もしない・`balloon_visibility_wait.rs` の `released`）。案 A（項を外す）なら常に「すぐ隠れる」側で決まる。
- どちらが「選択肢の無い台詞と同じ決まり」か、要件の字面だけでは決まらない＝議題（§5 の 3）。

#### 3.1.6 案 C — 組み合わせ

- C1: 案 A（可視性）＋ 入力の側で済んだ選択肢を当たりから外す（S2 の `fold_talking` を普通のバルーンの入力にも持たせ、「話していない間は種類が選択肢の当たり行を数えない」）。emo-text に触れない。ただし入力の線と文字の線が別＝新しいトークの最初の数フレーム、`TalkStarted` より先に選択肢の行が当たりに出ると、その間は押せない（拒む側に倒れる）。強調の消し忘れ（済んだ瞬間にポインタが行の上に居る）は入力の側から `inject_choice_hover(None)` を打つ要がある。
- C2: 案 B ＋ S4（押した瞬間にその場所の選択肢を済みにする）。選んだ応答待ちの二度押しの `choice_rejected_busy` も消える。ただし要件 1.1 は「選んだ応答を待つ間」を選択待ちに数えている＝押した瞬間に済みにするのは要件 1.10/1.11 の「済んだ後」の読みの選び（§5 の 2）。

#### 3.1.7 件 1 の比べ

| | 1.1〜1.9 | 1.10〜1.12 | 触るファイル | 届く順 | 完了 spec の改め |
|---|---|---|---|---|---|
| A | ○ | × | 可視性 2〜3 本 | 問題なし | `balloon-visibility` 5.4 を外す |
| B＋S3 | ○ | ○ | emo-text 3〜4 本＋`talk_clock.rs` | 構造で安全 | 5.4 の「表示中」の意味を改める |
| B＋S1/S2 | ○ | ○ | emo-text＋結線 | **罠あり**（§3.1.4） | 同上 |
| B＋S5 | ○ | ○（応答待ちも正確） | emo-text＋kanade＋結線 | **罠あり**＋kanade の語彙 | 同上 |
| C1 | ○ | ○（入力の線の遅れの縁あり） | 可視性＋`input_events` 4 本 | 拒む側の縁 | 5.4 を外す |

### 3.2 件 2

| 案 | 中身 | 得 | 失 |
|---|---|---|---|
| 2-a 装着の枝で止める | `present_actor` の初回装着の手前で領域の幅か高さが 0 以下（または `physical_size` が 0）なら装着せず `Ok(())`。場所ごとの印（既存の `unresolved_warned`・`box_overflow_warned` と同じ `BTreeSet<PlaceKey>`）で入りに `warn!` 1 回・戻りに 1 回 | 普通と箱が通る 1 か所＝根。`surface.rs`（995 行）に触れない。装着の記録の文も残る | `actor_present.rs` が `anchor-tag-canon` 後の列の頭の席（C7 の約束の内） |
| 2-b 登録口で止める | `register_actor`（`actor_attach.rs`）と `register_box`（`actor_box.rs`）で潰れを見分けて印を付け、提示はその印で飛ばす | 記録が「値が新しく決まったとき」の層に乗る（折返しの警告の先例と同じ層） | 2 か所に分かれる。提示の側にも見る点が要る |
| 2-c 供給面の生成で止める | `TextSurface::create` で 0 寸を断る | 下の層で確実 | `surface.rs` 995 行＝6 行以上なら先に検査を兄弟へ出す（`lib.rs` の一覧に触る）。失敗の形のままなので毎フレームの試行は止まらない＝根でない |
| 2-d 再試行の側で止める | `scale_text.rs` で同じ失敗の記録を抑える | 重なり 0 | 症状止め（毎フレームの生成の試みは残る）・`balloon-canon-residue` と重なる |

- どの案でも **`region.rs:270` の毎フレームの WARN を外すか、状態の変わり目の層へ移す**要がある（2.3・2.5）。
- どの案でも、**装着後に潰れたときの差し込み口の古い絵**（§2.2 の穴）の扱いを決める要がある。

### 3.3 件 3

| 案 | 中身 | 3.6 上の行 | 3.7 行と入る数 | 縦書き 3.4 | 箱 3.5 | 改める決め |
|---|---|---|---|---|---|---|
| 3-a 面を広げる | 面を行送りの遠い側へ「行の箱が文字の大きさの箱からはみ出す量」だけ大きく作る。原点（左上）は変えない（横書き） | 変わらない（描く座標は同じ） | 変わらない | 縦書き（右から左）は遠い側が**左**＝原点をずらす要があり、描く座標・当たり・スクロールの写しへ波及 | 箱の面は窓の子＝はみ出しは箱の外へ数画素描く。字の矩形の当たりは領域から作るので不変の見込み | 「validrect の外には描かない」の読み（行送りの軸に数画素はみ出す）＝§8 に記録 |
| 3-b 下線を自分で描く | `SetUnderline` をやめ、区間の矩形（`HitTestTextRange`）と font metrics の下線の位置・太さから矩形を塗る。位置を行の箱の中へ収める | 自分で描いた線が D2D の線と 1 画素も違わない保証が要る（全行を自前にすると見た目が変わりうる・最下行だけ自前は 2 流儀） | 変わらない | 縦書きの線の側（列の右）を自前で決める＝`anchor-style-canon` の領分に食い込む | 同じ式で効く | 完了 `text-decoration-canon` 要件 5.7（線を描き分けない）を改める |
| 3-c 入る行の数え方を変える | `visible_window` の遠端を下線（またはインク）まで含めて数える | 変わらない | **最下行が収まらなくなる**＝実機の検体で 4 行 → 3 行。下線の無い台詞まで減らすか、下線のある行だけで数えるか（後者は下線を足すとスクロールが起きる） | 同じ式で効く | 同じ | 完了 `emo-text-line-height-canon` の「行の矩形で数える」 |

- 3-a は descent のインクの欠け（§2.3 の穴）も同時に救う見込み。3-b と 3-c は下線だけ。
- 3-a のはみ出しの量の出どころ: `GlyphMetrics::line_box_height(em)`（`draw_metrics.rs:350`）と行送り（`line_pitch`）は既にある。下線の位置と太さ（`DWRITE_FONT_METRICS` の `underlinePosition`／`underlineThickness`）は今の metrics に無い。面は**装着のとき 1 回**作り、以後の内容（`\f[height,40]` の大きな行）で大きさは変わらない＝最大の字の大きさが装着の時点で分からない。**Research Needed**（量の決め方・面を作り直すか、上限を決めるか）。
- 描き替えの範囲（`expand_guard_clamp`）・スクロールの写し（描画実行部の blit・檻は `viewbox_draw_scroll_retain_tests.rs`）・提示（`present`）は面の大きさを使う＝3-a で面が大きくなると一緒に動くが、はみ出しの帯に前の行の線が残らないかを確かめる要がある。

## 4. 規模と危険度

| 件 | 規模 | 危険度 | 一行の理由 |
|---|---|---|---|
| 件 1 | M（3〜7） | 中 | 直しそのものは小さいが、合図の線の選びと届く順（§3.1.4）・起点の振る舞い（§3.1.5）が設計の判断。檻の書き直しが多い |
| 件 2 | S（4〜6） | 低 | 1 か所のガード＋記録の置き場所。差し込み口の古い絵だけ未確認 |
| 件 3 | M（4〜7） | 中 | 案の選び・縦書きと拡大率・はみ出しの量の出どころが未知。実 GPU の読み戻しの檻が要る |
| 合計 | 11〜20 | — | brief の見立てのまま。件 1 を B＋S3、件 3 を 3-a の縦書き込みで組むと上限に寄る |

## 5. 設計へ持ち越すこと

### 5.1 議題（答えで作業が変わるもの）

1. **件 1 をどの層で直すか**: 案 A（可視性だけ＋1.10〜1.12 は別）／案 B（文字の層の「済み」の印・根の 1 か所）／案 C1（可視性＋入力）。B なら合図の線（S1〜S5）も選ぶ。届く順を構造で守れるのは S3（文字の線にトークの終わりを流す）だけ。
2. **「選択が済んだ」の時点**: トークの終わり（済んだ選択の全部に共通）か、押した瞬間（選んだときだけ早い・二度押しの `choice_rejected_busy` も消える）か。要件 1.1 は「選んだ応答を待つ間」を選択待ちに数えているので、時間切れはトークの終わりの側。光り・押下（1.10・1.11）だけ押した瞬間にするかが分かれ目。
3. **長く待った後に済んだ選択の時間切れの起点**: 今の決まりのまま（表示の終わりから＝トークの終わりが届いてすぐ隠れることがある）か、「選択待ちを抑止の 1 つとして数え、解けた時から測り直す」（要件 5.3 の形・今から 30 秒）か。案 B で抑止を残すと、どちらになるかが合図の届く順で揺れる＝どちらかへ決める要がある。
4. **完了 `balloon-visibility` 要件 5.4 の扱い**: 外す（案 A・C1）か、「選択肢の表示中」を「選択が生きている」へ改める（案 B）か。どちらも §8 か完了 spec への申し送りの記録が要る。
5. **件 2 の止める層**: 装着の枝（2-a）か登録口（2-b）か。あわせて **`region.rs` の毎フレームの WARN をどこへ移すか**（2-a の印へ寄せるか、登録口の「値が新しく決まったとき」へ寄せるか）。
6. **件 2 の装着後に潰れたとき**: 差し込み口に残る古い絵を外す（`VisualGraphics` を外す・空の絵にする）か。箱は `despawn_window_child` の既存の片付けを使えるか。
7. **件 3 の案**: 3-a（面を広げる・descent のインクも救う見込み・validrect の外へ数画素）／3-b（自分で描く・`text-decoration-canon` 5.7 を改める・`anchor-style-canon` が上に乗る）／3-c（数え方・入る行が減る）。
8. **件 3 で 3-a のとき、はみ出しの量をどう決めるか**: 既定の字の大きさの行の箱から 1 回だけ決めるか、大きな字の行が来たら面を作り直すか、上限の比を決めるか。縦書きで原点をずらすか。

> **2026-10-16 要件の討議の結果**: 議題 1＝押した瞬間に済み（時間切れ・中断はトークの終わり）。議題 2＝選択が生きている間を抑止に数え、解けた時から測り直す。議題 3＝3-a（面を広げる）と 3-c（数え方）は採らず、**横書きの下線を areka が自分で描く（3-b の変形）**：位置はどの行もフォントのベースラインの直下、最下行で面に収まらないときだけ面の内側へ寄せる（字にかぶってよい）。縦書きは線の側と位置を今のまま、端の列で収まらないときだけ寄せる。設計で決めること: ベースラインと下線の太さの出どころ（`DWRITE_LINE_METRICS.baseline`・`DWRITE_FONT_METRICS.underlineThickness`）・`SetUnderline` をやめる範囲（横書きだけか）・打ち消し線を DirectWrite のまま残す並び・拡大率での丸め。

### 5.2 Research Needed（設計の調べ）

- R1: 差し込み口の entity に入れた `VisualGraphics`／`Arrangement` が、`ActorRender` を捨てた後にどう見えるか（再追従の経路・`surface.rs:330`）。外す口が wintf に在るか。
- R2: Yu Gothic UI 28 画素と他のフォントで、DirectWrite の行の上端から下線・descent のインクまでの量（`DWRITE_FONT_METRICS`・`DWRITE_LINE_METRICS`）。最下行の descent も欠けているか（撮り直しの検体に `g`・`y`・`ぁ` を入れる）。
- R3: 縦書き（`VerticalRl`）の端の列で下線が欠けるか、欠けるならどちらの辺か（列の右＝`balloon-vertical-canon` の裁定）。
- R4: 拡大率 125％・150％で、`physical_extent` の切り上げと下線の位置の丸めで欠け方が変わるか。
- R5: 文字の線の汲み出しと、可視性・入力の段の汲み出しが同じフレームの中でどの順に走るか（§3.1.4 の罠の実際の幅）。S3 を選ぶなら、`EmoTextSink` の複製の境界（`Clone` を手書きにするか、包みの `ClockedTextSink` に持たせるか）と、登録の原本が落ちたとき何も送らない形。
- R6: 選択の応答の後、旧トークがバリアの後に合図を持つ台本（`\q` の後に字が続く）で、占有終端と済みの合図の順がどうなるか（ukadoc の `\q` の後続の字の扱いと sakura の compile の並び）。

### 5.3 ウェーブ C7 との境界

- C7 の ② の触るファイルの一覧は「`region.rs`・`actor_present.rs`・`surface.rs`（件 1 を案 B で直すなら `actor.rs`・`state.rs`・`input_events/{balloon_pressed,shell_box_handler}.rs`）・emo-text の新しいファイルと `lib.rs`」。**件 1 を S3 で組むと `emo2_boot/talk_clock.rs`（と `sink.rs`）、可視性を直すと `emo2_boot/balloon_visibility_*.rs`、件 3 で `viewbox_diff.rs`・`viewbox_draw*.rs`・`layout.rs`・`draw_metrics.rs` が一覧の外**。同じウェーブの ①（`emo2_boot/{ghost_switch,shell_balloon_resolve,shell_balloon_switch}.rs` ほか）・④（`actor_lookahead_tests.rs`・`actor.rs` と `lib.rs` に触らない約束）とは重ならないが、設計で一覧を引き直して roadmap の約束の欄へ返す要がある。`clippy-199-lints` は `balloon_visibility_*` に触る（列の外・同じウェーブではない）。
- `actor_choice_contract_tests.rs`（991 行）・`actor_region_warn_tests.rs`（894 行）・`surface.rs`（995 行）は足す余白が無い＝新しい兄弟ファイルは `lib.rs` の一覧（`PURE_SOURCES`／`SOURCES_OUTSIDE_THE_PURE_SCAN`）に載せる。

## 6. 次の段

- 要件の討議（`/kiro-requirements-discussion areka-P0-balloon-text-bugs`）で §5.1 の議題を決め、そのあと `/kiro-design areka-P0-balloon-text-bugs`。

---

# 設計の調べ（2026-10-16・`/kiro-spec-design`）

## 要約

- **機能**: `areka-P0-balloon-text-bugs`
- **調べの種類**: 既存の仕組みの拡張（軽い調べ）。外のライブラリの新しい採用は無い（使うのは DirectWrite に既にある口と、ワークスペースに既にある `windows-core`）。調べはコードの読みで行い、手分けの調べ役は起こしていない（3 件とも触る場所が少数のファイルに絞れたため）。
- **使った手引き**: `kiro-spec-design` の規則（設計の原則・軽い調べ・統合・査読の門）だけ。デザイン・アクセシビリティ・UX の手引きは当たらない（見た目を変えない直し）。
- **設計を決めた発見**:
  1. 選択待ちの区切りは**台本の最後**に置かれる（`crates/areka-sakura/src/compile.rs` 311〜327 行・選択肢が 1 つでもあれば最後に 1 個）。だから「選択の待ちが解ける」＝「そのトークが終わる」で、トークの終わりの時刻（受け口が落ちた瞬間・丸めない）がそのまま「待ちが解けた時刻」になる。表示の終わり（占有終端）は区切りの待ちを数えないので、選択肢のトークでは待ち始めの時刻に止まったまま——これが件 1 の「長く待ってから済むとすぐ隠れる」の根。
  2. 合図の線は 2 本とも「トークごとの複製が落ちるときに終わりを送る」形が使える（`BalloonLifecycleSink`・`NoUserBreakCueSink` の先例）。次のトークは前のトークの受け口がすべて落ちてから始まる（`talk_lifecycle.rs` の Event Contract）ので、**文字の線に終わりを流せば、前のトークの終わりは次のトークの合図より必ず前に届く**。
  3. 下線は今、D2D の `DrawTextLayout` が DirectWrite の幾何で引いている。DirectWrite は `IDWriteTextLayout::Draw` に受け手（`IDWriteTextRenderer`）を渡すと、区間ごとの下線の幾何（`DWRITE_UNDERLINE`＝幅・太さ・ずれ・読みの向き・行送りの向き）と、その区間の色（描画効果）を教えてくれる。これを採れば、横書きの縦位置だけを置き直し、縦書きは今の位置のまま、どちらも面の内側へ寄せられる。

## 調べの記録

### 件 1: 選択待ちとトークの終わりの関係（R5・R6 の答え）

- **きっかけ**: 議題 2（解けた時から測り直す）を、合図の届く順に揺られずに実装できるか。
- **見たもの**: `compile.rs` の区切りの出し方・`crates/dola/src/cue/runtime.rs` の `resolve_choice`／`skip_barrier`（解いた後に占有の終わりに達していれば即座に完了）・`talk_lifecycle.rs` の `Drop`（落ちた瞬間の talk 相対秒を送る）・`balloon_visibility_wait.rs` の `effective_display_end` と抑止が解けたときの枝・`crates/areka-kanade/src/schedule/steady.rs` の帳簿の確立点（`on_choice_waiting`）。
- **分かったこと**:
  - 区切りの後ろに合図は無い（R6 の答え）。解かれたトークはその場で完了し、受け口が落ちる。
  - 今の「抑止が解けたら今から測り直す」の枝は、計測が成り立たない間（トークの終わりの前）に解けると何もしない。ゆえに抑止として残すと、解ける時刻と終わりの時刻のどちらが先かで振る舞いが割れる（ギャップ分析 §3.1.5 のとおり）。
  - 選択待ちの間はトークが終わらない＝「トークの終わりが届くまで計測しない」という既存の規則が、待ちの間の抑止をそのまま担っている。足りないのは**起点**だけ。
- **設計への効き目**: 可視性の側は「選択肢つきのトークの起点＝止まった時刻」とする。選択肢の行が表示中かを毎フレーム問う観測は要らなくなる（外す）。

### 件 1: 済んだ選択肢の行（光らない・押せない）の置き場所

- **見たもの**: `actor.rs` の `apply_cue(Clear)`（範囲・当たり行の写し・強調を同時に無効にする原子性）・`actor_present.rs` の写しの作り方・入力の 4 か所が `choice_hit_rows`／`hit_active` しか読まないこと・`state_decoration.rs` の `clear_content`（範囲の列と一緒に消える欄の置き場）。
- **分かったこと**: 「済み」を範囲の列と同じ寿命の欄（場所ごとの「ここまでの範囲は済み」の境目）に持たせれば、`\c`・台詞の頭で境目も構造的に消える。境目にしておけば、将来 `\C`（前の台詞へ続ける）を入れたときも、後から足された選択肢を済みにしない。
- **R5（汲み出しの順）の答え**: 文字の層の「済み」は、文字の線の上のトークの終わり（新設）と、UI 自身の押下からしか付けない。可視性の線・入力の線の合図で文字の層を書き換えないので、線をまたぐ順の罠（§3.1.4）は構造上起きない。

### 件 2: 潰れた描画範囲（R1 の答え）

- **見たもの**: `region.rs` の毎フレームの `warn!`・`actor_attach.rs` の `refresh_actor_binding`（描画資源を捨てるが装着物は外さない）・`surface.rs` の `attach`（差し込み口へ `VisualGraphics`＋`Arrangement` を入れる）・`crates/wintf/src/ecs/graphics/components.rs` の `VisualGraphics` の `on_remove`（親の Visual から子を外す）・`crates/wintf/src/ecs/graphics/systems/visual_sync.rs`（未同期の子を見つけると親の子を全部外して `Children` の順に入れ直す）・`crates/areka-emo-present/src/mount.rs`（差し込み口は `Visual`＋`HitTest` だけで生まれ、`Arrangement` を持たない）。
- **分かったこと**:
  - 差し込み口から `VisualGraphics` と `Arrangement` を外せば、`on_remove` が文字の絵を親の Visual から外す＝古い絵は残らない。差し込み口は内容と配置の無い姿に戻る。
  - 戻ったときの再装着は今の装着と同じ（新しい `VisualGraphics` は未同期なので、同期の段が親の子を `Children` の順に入れ直す＝重なりの順は保たれる）。
  - 再追従は World を持たないので、潰れたときだけ描画資源を捨てずに残し、World を持つ提示の段が外す（同じフレームの下流・`run_text_scale_phase` は `run_text_phase` の上流）。
- **記録の置き場所**: 「状態が変わったときだけ」の意味を持つのは登録口（値が新しく決まる所）。ただし箱は隠す・出すで登録を外し・付け直すので、「前の値」を場所ごとの記憶（潰れている場所の集合）で持つ。記憶は登録を外しても消さない（隠す・出すで記録を増やさない＝要件 2.5）。

### 件 3: 下線の幾何の出どころ（R2〜R4 の答え）

- **見たもの**: `viewbox_draw_decoration.rs` の `apply_font_ranges`（`SetUnderline` の真偽だけ）・`viewbox_draw_render.rs` の Phase 1（可謬の準備）と Phase 2（描画・不可謬）・`draw_line_store.rs`（行の TextLayout の生成と保持・はみ出しの実測）・`windows` 0.62 の `IDWriteTextRenderer_Impl`（`DrawUnderline` は基準線の原点・`DWRITE_UNDERLINE`・描画効果を受け取る）・`windows-core` 0.62 の `implement`（`crates/areka`・`crates/wintf` が既に使う依存）。
- **分かったこと**:
  - `DrawUnderline` が渡す基準線の原点は、その行のベースラインそのもの。横書きは「上端＝ベースライン」に置き直すだけで要件 3.10 になる。太さ・幅は DirectWrite が区間の書体と大きさから決めた値をそのまま使える（要件 3.2・フォントに依らない）。
  - 縦書きは DirectWrite の幾何をそのまま使い（要件 3.4「側と位置は今のまま」）、面に収まらないときだけ行送りの軸で寄せる。端の列で本当に欠けるか（R3）はこの形なら答えに依らず満たせる（収まっていれば寄せは何もしない）。実機の撮り直しで確かめる。
  - 色は D2D の既定の描き方と同じく、その区間の描画効果（装飾の色・強調の文字色）か、無ければ既定の文字色のブラシ。描画効果は毎フレーム焼き直されるので、幾何と色は Phase 1（効果を焼いた後）に集め、Phase 2 で塗る。
  - 拡大率（R4）: 寄せは画像の画素で、面の物理寸 ÷ k を境目にして行う（面は `ceil(validrect × k)` なので境目は validrect 以上）。線は k の変換の下で塗るので、拡大率に依らず面の内側に入る。
  - `windows-core` を emo-text の依存に足す（新しい外部のクレートは 0。`#[implement]` が生む式が `::windows_core` を名指しするため、直接の依存が要る。`Cargo.lock` の emo-text の依存の列に 1 行増える）。ウェーブ C7 の「依存を足す席」は空き（roadmap 142 行）。
- **descent のインク**: 最下行の字の下の部分が面の端で欠けているかは、要件どおり撮り直しで確かめ、欠けていれば範囲の外として起票する（本 spec の設計には入れない）。

## 案の比べ（設計の段）

| 件 | 案 | 中身 | 採否と理由 |
|---|---|---|---|
| 1（時間切れ） | 抑止のまま残す | 「選択が生きている」を抑止の 1 つに数え、解けた縁で今から測る | 不採用。解ける縁がトークの終わりより前に来ると縁が空振りし、起点が占有終端（待ち始め）へ戻る＝議題 2 に反する場面が合図の順で生まれる |
| 1（時間切れ） | **起点を待ちが解けた時刻にする** | 選択肢を出したトークは、止まった時刻を起点にする（時刻が無ければ終わりを畳んだフレームの現在時刻） | **採用**。待ちの間は既存の「終わりが届くまで計測しない」が担う。合図は可視性の線の中だけで完結する |
| 1（行） | 入力の側で外す | 入力の 4 か所で「話していない間は選択肢を当たりに数えない」 | 不採用。4 か所に分かれる・強調の消し忘れを入力から打つ要がある・線の遅れの縁がある |
| 1（行） | **文字の層の「済み」の境目** | 範囲の列と同じ寿命の境目。押した瞬間（その場所）と文字の線のトークの終わり（全部の場所）で付け、当たり行・強調・`choice_active` が同時に従う | **採用**。`Clear` と同じ原子的な無効化の形・入力と可視性は直さずに従う |
| 2 | 生成で断る（`surface.rs`） | 0 寸の生成を断る | 不採用。失敗の形のまま＝毎フレームの試みが残る・995 行の上限 |
| 2 | **装着の枝で見分ける＋登録口で記録** | 提示は潰れた場所を描かず、登録口が状態の変わり目に 1 回ずつ記録する | **採用**。普通のバルーンと箱が通る 1 か所・記録は値が決まる層 |
| 3 | 下線の矩形を自分で計算する | 行の metrics・`HitTestTextRange`・書体の metrics から矩形を作る | 不採用。縦書きの位置を DirectWrite と同じに再現する計算を自前で持つことになる・区間の色の分け方も自前 |
| 3 | **DirectWrite に下線の幾何を聞いて自分で塗る** | `IDWriteTextLayout::Draw` に記録だけする受け手を渡す | **採用**。太さ・幅・縦書きの位置・色は DirectWrite と D2D の決め方のまま、横書きの縦位置と寄せだけを areka が決める |

## 統合の 3 つの見方

- **まとめられるもの**: 件 1 の「済み」の無効化は、`Clear` の「範囲・当たり行の写し・強調を同時に」と同じ形＝1 つの内部の手順（場所を指して写しと強調から済みの選択肢を外す）を、押下とトークの終わりの 2 つの入口が呼ぶ。件 2 の「状態の変わり目に 1 回」は既存の `box_overflow_warned` と同じ「場所の集合を持つ」形。
- **作るか採るか**: 下線の幾何は DirectWrite の `IDWriteTextRenderer` を採る（作らない）。差し込み口の片付けは wintf の `VisualGraphics` の `on_remove` を採る。トークの終わりは既存の「複製が落ちるときに 1 回送る」型を採る。
- **削ったもの**: 可視性の観測 `ScopeObservation::choice_active` と抑止の種類 `SuppressionKinds::choice` を外す（起点の規則が代わりを担う）。`region.rs` の毎フレームの `warn!` を外す。新しい公開型は増やさない（下線の受け手は emo-text の中だけ）。

## 決めたこと

### 決め: 選択肢つきのトークの時間切れの起点は「止まった時刻」

- **背景**: 要件 1.2〜1.4・1.8、議題 2。
- **比べた案**: 抑止のまま残す／起点を変える。
- **選んだ形**: `BalloonLifecycleSink` が選択肢の合図を初めて見たときに `ChoiceShown` を 1 回送る。可視性はそのトークを「選択肢つき」と覚え、終わりが届いたら止まった時刻（時刻が無ければそのフレームの現在時刻）を起点に、そのトークの待ち時間で測る。
- **理由**: 区切りが台本の最後にあるので、止まった時刻＝待ちが解けた時刻。合図は同じ線の FIFO なので順に揺られない。
- **失うもの**: 完了 `balloon-visibility` の要件 5.4（選択肢の表示中を抑止に数える）を外す＝COMPAT §8 に記録する。
- **後で確かめること**: 実機で 3 つの済み方それぞれ、済んでから既定の長さで隠れること。

### 決め: 「済み」は範囲の列の境目で持ち、押した瞬間とトークの終わりで付ける

- **背景**: 要件 1.10〜1.13、議題 1・開発者の裁定（範囲に入れる）。
- **選んだ形**: 場所ごとの境目（それより前の選択肢は済み）。押下が選択を送れたら、その場所の境目を今の範囲の数へ進める。文字の線にトークの終わり（`TextMsg::TalkEnded`）を流し、届いたら全部の場所の境目を進める。どちらも同時に当たり行の写しから済みの選択肢を外し、済みの選択肢を指す強調を外す。
- **理由**: 範囲の記録を消さない＝見た目（字・色・印）は変わらない。入力と可視性は当たり行と `choice_active` しか読まないので直さずに従う。
- **後で確かめること**: 区切りに達する前に押された選択（運行が「待っていない」で捨てる）の後は、その場所の選択肢は押せないまま選択肢の時間切れを待つ（リスクの節）。

### 決め: 潰れた描画範囲は描かず、記録は登録口で状態の変わり目に 1 回

- **背景**: 要件 2.1〜2.9。
- **選んだ形**: `TextRegion::is_collapsed`（幅か高さが正でない・非数を含む）。提示は潰れた場所を装着せず描かず、装着済みの描画資源があれば差し込み口から外して捨てる。登録口（普通のバルーン・箱）が場所ごとの記憶と突き合わせ、入ったときに `warn!` 1 回・戻ったときに `info!` 1 回。`region.rs` の毎フレームの `warn!` は外す。
- **理由**: 普通のバルーンと箱が通る 1 か所で止まる・記録は値が決まる層・隠す出すで記録が増えない。

### 決め: 下線は DirectWrite の幾何で areka が塗る（横書きはベースラインの直下）

- **背景**: 要件 3.1〜3.11、議題 3。
- **選んだ形**: 行の TextLayout には下線を焼かない（D2D は下線を描かない）。下線を引く区間（今 `SetUnderline(true)` を焼いている区間と同じ選び方）を行ごとに持ち、行を描く前の準備で、区間に下線を一時的に立てて記録の受け手で幾何と色を集めて下線を戻す。描画では行の字を描いた後に線を塗る。横書きは上端をベースラインに置き直し、縦書きは DirectWrite の位置のまま。どちらも面に収まらなければ行送りの軸で面の内側へ寄せる。行のはみ出しに線の分を足して、描き替えの範囲から漏れないようにする。
- **理由**: 字の描き方（`DrawTextLayout`）は変えない。太さ・幅・色・縦書きの位置は DirectWrite と D2D の決め方のまま。
- **失うもの**: 完了 `text-decoration-canon` の要件 5.7（線を描き分けない）を横書きの下線について改める＝COMPAT §8 に記録する。`windows-core` を emo-text の依存に足す（`Cargo.lock` の席）。

## リスクと手当て

- **区切りの前に押された選択**: 選択肢の文字が出た後、区切りに達する前（選択肢の後ろの字の表示中や `\w` の待ち）に押すと、運行は「待っていない」で捨て（今も同じ）、本 spec ではその場所の選択肢が済みになる＝選択肢の時間切れまで選べない。今は捨てられた後に押し直せる。範囲の狭い場面だが、設計の討議で扱いを確かめる（要件 1.10 の字面どおりに作る）。
- **トークの終わりの受け口を落とす検査**: 文字の受け口の複製で `emit` してから落とすと、トークの終わりが流れて選択肢が済みになる。既存の検査で、受け口を落とした後に選択肢の当たり行を読むものがあれば、受け口を生かしたまま読む形へ直す（実装の最初に洗う）。
- **下線の受け手の画素への寄せ**: 受け手が答える変換と DIP の比が D2D と食い違うと、縦書きの線が今の位置から画素未満ずれうる。D2D 自身が描いた下線との画素の比べ（実 GPU）で固定する。
- **毎フレームの下線の立て直し**: 下線のある行を描くフレームだけ、区間の下線の立て・戻しと記録の呼び出しが増える。下線の無い行と、変化の無いフレーム（`NoChange`）には何も足さない。
- **差し込み口から外した後**: 外した後に wintf が既定の空の Visual を付け直さないこと・再装着で重なりの順が保たれることを、ECS の檻（差し込み口の部品・親の子の順）で固定する。

## 参考

- [IDWriteTextRenderer::DrawUnderline](https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritetextrenderer-drawunderline) — 下線の幾何と描画効果を受け取る口。
- [DWRITE_UNDERLINE](https://learn.microsoft.com/windows/win32/api/dwrite/ns-dwrite-dwrite_underline) — 幅・太さ・ずれ・読みの向き・行送りの向き。
- `crates/areka-sakura/src/compile.rs` 311〜327 行 — 選択待ちの区切りは台本の最後に 1 個。
- `crates/wintf/src/ecs/graphics/components.rs` 84〜110 行・`crates/wintf/src/ecs/graphics/systems/visual_sync.rs` — `VisualGraphics` を外すと親の Visual から外れる・未同期の子で親の子を並べ直す。

### 設計の討議 議題 3 の決め（2026-10-16）

- 下線の受け手は `#[implement(IDWriteTextRenderer)]` で作り、emo-text に `windows-core = { workspace = true }` を足す（開発者「足すのは良い」）。
- 版に注意する（開発者「今出てる 0.100.0 は互換性が無い。windows のバージョンと合わせるように」）。ワークスペースのピン `windows-core = "0.62.2"` が `windows = "0.62"`（`Cargo.lock` で `0.62.2`）と揃っているので、emo-text の `Cargo.toml` には版を書かない。`windows-core` だけを上げない。
- C7 の「依存を足す席」⑴ を本 spec が使うと roadmap に書き戻した。

### 設計の討議 議題 1・2 の決め（2026-10-16・開発者「議題1と2は推奨案で承認」）

- **議題 1（区切りより前の押下）**: 押下で選択肢を済ませるのは、そのトークが選択待ちに入った後だけ。再生（`drive.rs` の `notify_choice_waiting_if_newly_waiting`）が `ChoiceWaiting` を done の口へ送った後に、`dola::cue::CueSink` の新しい既定つきメソッド `choice_wait_began` で文字の線へ `TextMsg::ChoiceWaitBegan` を積む（先渡しの `preview` と同じ型）。それより前の押下は今のまま kanade が捨て（`steady.rs` 245 行の `choice_rejected_no_wait`）、押し直せる。根拠: 押せる時を kanade が受け付ける時と同じ順で決められ、台本の最後の `\w` の待ちも正しく扱える。採らなかった案: 先渡しの `Upcoming` で文字の層だけで見分ける（最後の `\w` を見落とす）・範囲の外として起票（穴が残る）。残る確かめ: `ChoiceWaiting` は dispatcher を経て kanade へ着くので、押下の選択の着く順がそれより後と言えるかを実装の最初のタスクで経路を読んで確かめる。
- **議題 2（縦書きの下線）**: 縦も横も同じ塗り方の 1 本の道。D2D が描いた縦書きの下線との一致の合格の幅を先に決めた（覆う画素の範囲が同じ・各画素の各成分の差 4/255 以下）。採らなかった案: 縦書きは D2D に描かせ端の列だけ自前で塗る（分かれ道とフレームごとの切り替えが増える）。

### タスク 4.1 の確かめ: 押下の選択と `ChoiceWaiting` が kanade に着く順（2026-10-16・結論は「後と言い切れない」）

kanade の入口は 1 本の mpsc（`crates/areka-ghost/src/runtime.rs:724` の `kanade_tx.clone()` を dispatcher へ、`crates/areka/src/ghost_session.rs:820` の `runtime.kanade().clone()` を選択の取り次ぎへ渡す＝同じ受け口の複製）。2 つの伝言は別々のスレッドからこの口へ送られる。

- 経路 A（選択待ち）: 再生のスレッドが `crates/areka-sakura/src/drive.rs:398` で `ChoiceWaiting` を done の口（dispatcher の inbox）へ送る → **dispatcher のスレッド**が inbox を順に汲み、`crates/areka-ghost/src/dispatcher.rs:130` → `on_choice_waiting`（228 行）→ 261〜270 行で `KanadeMsg::ChoiceWaiting` を kanade へ送る。
- 経路 B（押下・設計どおりに作った後）: 再生のスレッドが 398 行の後に `announce_choice_wait` → 文字の線へ `ChoiceWaitBegan` → UI が受けて済ませてよい印を立てる → 押下 `crates/areka/src/input_events/balloon_pressed.rs:163`（`send_selection`・`crates/areka/src/input_events/balloon.rs:138`）→ **UI のスレッド**の `drain_choice_selections` が `crates/areka/src/input_events/choice_drain.rs:101` で `KanadeMsg::Choice` を kanade へ直接送る（dispatcher を通らない）。
- 再生のスレッドで「done の口へ送った」が「文字の線へ積んだ」より先なのは言えるが、それは **dispatcher の inbox に入った**ことまでしか保証しない。dispatcher が kanade へ送り出す（261 行）ことと、UI が kanade へ送る（101 行）ことの間には同期が無い＝happens-before が無い。dispatcher の inbox に先に積まれた伝言（`Tick`・`Done`、`Start`／`Close` なら 379 行の `join` で待つ）を汲み終えるまで、A の送り出しは遅れうる。
- B が先に着いた時の結果: kanade は帳簿が無いので `crates/areka-kanade/src/schedule/steady.rs:239` の `on_choice` が 248 行の `choice_rejected_no_wait`（WARN）で捨てる。UI 側ではその場所の選択肢は既に済み（押せない）ので、利用者は押し直せず選択が失われ、要件 1.11 の「警告を出さない」にも反する。
- 実際の窓は「UI の受け取り＋押下の処理」と「dispatcher のスレッドの起き上がり」の競り合いで狭いが、構造では閉じていない。タスク 4.1 の指示どおり、ここで止めて開発者へ報告する（4.2 以降へ進まない）。コードは変えていない。
