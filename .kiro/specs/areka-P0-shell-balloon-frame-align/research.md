# ギャップ分析: areka-P0-shell-balloon-frame-align

> 2026-10-04 `kiro-validate-gap`。基準は worktree の HEAD `34a615a7`（main `e2a373b5` の上に spec の初期化だけ）。引用はすべて本文を読んで確かめた `file:line`。判断は設計へ渡す（本書は選択肢と材料だけ）。

## 1. 分析の要約

- **根は 1 つ**: 箱の置き場所と「箱が出ているか」の写しが、文字の層が受け取った `\s`（`TextLayerState::current_surface`）を基準にしている一方、絵は seriko → `PresentCommand` → `run_drain_phase` で別の時機に替わる。基準が 2 つあるので、どちらが先に届くかで 1 フレームずれる。brief の候補 1（`\s` の受け取りの時点で付け替える）は、絵が先に替わる並び（実機で 16 回中 2 回）を直せず、`apply_cue` は `World` も持たない。
- **0 フレームで解ける状態の持ち方がある**: 表示層は絵の番号を既に公開している（`EmoPresenter::current_surface_id(shell_target)`）。毎フレームの箱の同期（drain の後・提示の前）が「受け取った `\s`」でなく「いま表示している絵の番号」から置き場所を導けば、届く順に関係なく絵と置き場所は同じフレームで替わる。emo-present には触らない（公開済みの照会を読むだけ）。
- **`Status` は相の並びの問題でもある**: `report_balloons` は可視性の相（`balloon_visibility_phase.rs`）で呼ばれ、それは箱の同期（`run_text_scale_phase`）と提示（`run_text_phase`）より前に走る。写しの基準を絵へ移すだけでは、置き場所が替わるフレームの判定が 1 フレーム古いまま残る。届けをフレームの終わり（提示の後）へ移す案と、可視性の相の前に判定をそろえる案がある。
- **箱だけのときの番号**: 普通のバルーンを隠すと `apply(Hide)` が `current_surface_id` を `None` に落とす（`hub.rs:141`）ので、番号が取れず `warn!` になる。emo-present に触らずに直すなら、届けの台帳（`BalloonStatusLedger`）が最後に取れた番号を覚える形が最小。`balloon-canon-residue`（面の偶数・奇数）が番号の出どころを変えても、表示層の値を写すだけなら自動で揃う。
- **規模と危険**: M（5〜8 タスク）・中。触る相の並び（`frame.rs`）と、既存の箱の結線テストの組み方（`\s` の cue だけを当てて絵を替えていない）を作り直す必要がある。

## 2. 今の姿（コードの事実）

### 2.1 `\s` の 2 つの道

| 道 | どこで受け取るか | どこで効くか | 根拠 |
|---|---|---|---|
| 文字の層 | UI スレッドのポンプが起こす非同期の drain（フレームの外） | `apply_cue` → `route_surface` で行き先とサーフェス番号を即時に更新 | `areka-actor/src/ui.rs:82-125`（`spawn_ui`・`async_channel` の `recv().await`）・`areka-emo-text/src/actor.rs:344-345`・`state_route.rs:72-93` |
| 絵（seriko） | seriko の別スレッドが解決して `PresentCommand` を送る | フレームの中の `run_drain_phase` で `apply_show`／`apply(Hide)` | `areka-seriko/src/actor.rs:614-631`・`areka/src/emo2_boot/frame.rs:325` |

2 つの道は互いを待たない。どちらが先にフレームへ届くかは決まらない（設計にも明記・`completed/areka-P0-shell-balloon/design.md:101`）。

### 2.2 箱の置き場所の決め方

- 毎フレームの箱の同期 `sync_boxes`（`actor_box.rs:281-291`）→ `sync_box_bindings`（`:295-325`）→ `desired_box`（`:329-363`）。置き場所のサーフェス番号は **`self.state.current_surface(&key.actor)`**（`:344`）＝文字の層が受け取った `\s` の番号。
- 一方、同じ `desired_box` の装着先・拡大率・画像の大きさは **シェルの窓の `TextSlotView`**（表示層の今の姿）から取る（`:340-343`・`:361`）。基準が混ざっている。
- 唯一の呼び手は `run_text_scale_phase` の末尾（`frame/scale_text.rs:109`・`:133`）。`TextSlotView` にはサーフェス番号が無い（`areka-emo-present/src/presenter/read.rs:16-30`）。
- 置き場所が変われば `unregister_box`（面を despawn・`actor_box.rs:425-440`）→ `register_box`（`:370-422`）。面は同じフレームの `present_frame` が作り直す（`actor_present.rs:62`・`:128-215`）。

### 2.3 「箱が出ているか」の写し（`shown_boxes`）

- 作り直すのは提示の最後だけ（`actor_present.rs:96` → `actor_box.rs:176-218`）。
- `apply_cue` のたびに `prune_shown_boxes`（`actor.rs:364` → `actor_box.rs:221-233`）。判定 `box_still_shown`（`:239-260`）は **`state.current_surface`**（`:251-258`）の置き場所と登録済みの置き場所を比べる。よって `\s` を受け取った瞬間に、まだ絵が替わっていなくても写しから外れる。
- `hide_boxes`（`:125-129`）・`unregister_box`（`:431-435`）もその場で外す。
- 読み手: `Status` の届け（`frame/status_report.rs:79-83`）・可視性の相の時間切れ／中断の観測（`balloon_visibility_phase.rs:394`）・箱のポインタの前段（`input_events/shell_box_handler.rs:82`）。

### 2.4 相の並び（`emo2_frame_system`）

`frame.rs:281-386`: attach → dpi → **drain（`:325`）** → switch（`:329`）→ **可視性（`:334`・ここで `report_balloons`）** → 窓寸 → move → zorder → resnap → finalize → realign → **text-scale（`:383`・ここで `sync_boxes`）** → **text（`:384`・ここで `present_frame` と `refresh_shown_boxes`）**。

- `report_balloons` の呼び出しは `balloon_visibility_phase.rs:133-136`（`issue_actions` の後）。
- 可視性の相は普通のバルーンの窓を出すかを `balloon_shown_glyphs`（`balloon_visibility_phase.rs:309` → `actor_box.rs:140-150`）で決め、これも **`state.current_surface`** 基準。
- 時間切れの「箱を隠す印」は可視性の相で立ち（`balloon_visibility_phase.rs:571-586`）、同じフレームの後の `sync_boxes` が登録を外す（`actor_box.rs:337-339`）ので、隠したフレームに画素も消える。**可視性 → 同期 → 提示の並びはこの性質の支えになっている**（同期を可視性より前へ動かす案の注意点・4 節）。

### 2.5 `Status` の番号

- `collect_bindings`（`status_report.rs:42-60`）は `visible == Some(true) || box_showing` のスコープを `balloon_id = surface_id.unwrap_or(0)` で載せ、`None` を警告の対象に返す。警告は `:98-109`（`event="balloon_status_surface_unknown"`・スコープごとに 1 度）。
- `surface_id` は `presenter.current_surface_id(balloon_target(scope))`（`:80`）。可視性の相は窓を隠すとき `PresentCommand::Hide` を通す（`balloon_visibility_phase.rs:549-557`）→ `apply(Hide)` が `current_surface_id = None`（`areka-emo-present/src/presenter/hub.rs:141`）。全透明退化も `None`（`show.rs:220`）。確立すると `Some`（`show.rs:354`）。
- 装着の相はバルーンを面 0 で「不可視のまま確立」する（`frame.rs:6-10` の説明）ので、一度も隠していなければ `Some(0)` が取れる＝警告は「出して隠した後」にだけ出る（実機の記録 `real-machine-check.md:482` と一致）。
- 表示層は最後に確立した面を `last_show` に私有で持つ（`presenter/visibility.rs` の `show_target` が読む）が、公開の照会は無い。

### 2.6 テストの足場

- 本番の相順を GPU 付きで回す檻 `frame_shell_box_integration_tests.rs`（`Cage`）がある。文字の cue は `apply_cue` を直接当て（`:142-149`）、絵は `present_tx` へ `PresentCommand::ShowSurface` を積めば次のフレームの drain が適用する（`:99-109`）。**両方の届く順を決定論で組める**。
- ただし既存の 3 本は `\s` の cue だけを当てて絵を替えていない（`:199`・`:221`・`:245`・`:259`・`:283-285`）。置き場所の基準を絵へ移すと、これらは絵の差し替え（`ShowSurface`）も送る形に直す必要がある（箱のテストなので要件 4.5 の「普通のバルーンだけを使う既存のテスト」には当たらない）。
- 普通のバルーンの `Status` の檻 `frame_visibility_integration_tests.rs:482-528` は `emo2_frame_system` を丸ごと回して 1 フレームごとの届けを数える。届けの位置を同じフレームの中で後ろへ動かしても数は変わらない見込み（要件 4.5）。
- 純関数の檻 `status_report_tests.rs:105-119` は「箱だけ・番号なし → 0 で警告の対象」を固定しており、要件 3 で期待が変わる（箱の檻）。普通のバルーンの警告の檻 `:214-233` は変えない。
- 箱の同期の檻（`actor_box_sync_tests.rs`・`actor_box_present_tests.rs`・`tests/box_attach_test.rs:289`）は `sync_box_bindings`／`sync_boxes` の引数の形に依存する。

### 2.7 制約

- `emo2_boot/spine.rs` はちょうど 1,000 行（`wc -l`）。触らない。他の対象ファイルは `frame.rs` 542・`balloon_visibility_phase.rs` 754・`actor.rs` 575・`actor_box.rs` 460・`scale_text.rs` 319・`status_report.rs` 146 行で余裕がある。
- 同じウェーブの約束（brief「ウェーブ C3-③」）: `input_events/`・`frame/wiring.rs`・emo-present・`frame/{attach,switch}.rs` に触らない。`BalloonStatusLedger` は `status_report.rs` に定義があり（`:27-30`）、欄を足すだけなら `wiring.rs` は触らずに済む（`Default` で作られる）。
- 開発者方針: 1 フレーム遅らせて揃える解は取らない（要件 1.7）。

## 3. 要件と資産の対応

| 要件 | 今の資産 | ギャップ |
|---|---|---|
| 1.1 絵が先・`\s` が先のどちらでも同じフレーム | `desired_box` が `state.current_surface` 基準（`actor_box.rs:344`） | **Missing**: 置き場所を絵の番号から導く口が無い。`sync_boxes` に絵の番号が渡っていない（`scale_text.rs:109`） |
| 1.2 同じ名前の箱が無い面へ → 表示をやめる | 同上・`unregister_box` | **Missing**（1.1 と同じ根）。保持は `route_surface` のまま（要件 6 不変） |
| 1.3 `\s[-1]` で絵が消えるフレームに消す | 文字の層の `Hide` で `current_surface=None` | **Missing**（同上）。絵の側は `apply(Hide)` で `current_surface_id=None`（`hub.rs:141`）なので、絵の番号基準なら同じフレームで外れる |
| 1.4 保持した文字を同じ名前の箱で再表示 | `desired_box` がスコープの文字を名前で引く | **Missing**（同上） |
| 1.5 同じ置き場所なら途切れない | `sync_box_bindings` は同じなら何もしない（`:306-316`） | 既に成立。基準を変えても保つ必要あり（**Constraint**） |
| 1.6 シェルに無い番号は今のまま | `resolve_for_text` が `Unresolved`（`shell_box_assets.rs:58-68`）・seriko は合成失敗で表示不変 | 絵の番号基準でも成立する見込み（**Unknown**: seriko が解決できて合成で落ちる番号の扱いを設計で確かめる） |
| 1.7 遅らせない | — | **Constraint** |
| 2.1 置き場所の替わるあいだも欠けない | `prune_shown_boxes` が `\s` 受け取りで外す（`actor.rs:364`）・届けが提示より前（`frame.rs:334` < `:384`） | **Missing**: 写しの基準と、届けの時機 |
| 2.2 窓 ↔ 箱の受け渡し | 窓は `balloon_shown_glyphs`（`state.current_surface` 基準）、箱は同期（同じ基準） | **Unknown**: 箱だけ絵の基準へ移すと、窓と箱の切り替わりのフレームが分かれうる（4 節・決定事項 3） |
| 2.3 遅れて外す・遅れて載せることをしない | 同上 | **Missing**（届けの時機） |
| 3.1 箱だけのときの番号 | `surface_id.unwrap_or(0)`（`status_report.rs:56`） | **Missing**: 隠した後の番号を持つ所が無い |
| 3.2 警告の段を出さない | `warn!`（`:104-107`） | **Missing** |
| 4.1〜4.4 直す前に赤になる決定論テスト | `Cage`（GPU の檻）・純関数の檻 | 足場はある。届く順の 2 通りと、隠した後の箱だけの並びを組む必要 |
| 4.5 普通のバルーンだけのテストを変えない | `frame_visibility_integration_tests.rs` ほか | **Constraint**: 届けの位置を動かすならここを通す |

## 4. 実装の選択肢

### 置き場所（要件 1）

**案 A1 — 絵の番号を基準にする（推奨候補）**

- `run_text_scale_phase` がシェルの窓ごとに `presenter.current_surface_id(shell_target(scope))` を集め、`TextSlotView` と一緒に `sync_boxes` へ渡す（`scale_text.rs:109`）。文字の層は「表示しているサーフェス番号」をスコープごとに持ち、`desired_box`・`box_still_shown`（必要なら `balloon_shown_glyphs`）がそれを使う。行き先の決定（`route_surface`）は今のまま `\s` の受け取りで動く（要件 6 不変）。
- ✅ 届く順に依らず、drain の後・提示の前で 1 回決まる＝0 フレーム。表示層の照会を読むだけで emo-present は変えない。表示層を真実源にする既存の規律（`read.rs` の `target_visible` の説明「第 2 の帳簿を作らせない」）とも合う。
- ✅ `\s` の受け取りで写しが外れなくなる（絵はまだ替わっていない）＝要件 2.1 の半分が自然に解ける。
- ❌ 文字の層に「行き先の基準（受け取った `\s`）」と「表示の基準（絵）」の 2 つの番号が並ぶ。どちらを何に使うかの線引きを設計で明記する必要がある。
- ❌ 既存の箱のテストは `\s` の cue だけで置き場所を動かしているので、絵の差し替えも送る形に直す（2.6）。

**案 A2 — drain で絵の差し替えを捕まえて文字の層へ知らせる**

- `run_drain_phase` が `ShowSurface`／`Hide` のシェル宛てを適用した直後に、文字の層へ番号を渡す。
- ✅ 番号の変化が起きた瞬間に分かる。
- ❌ drain（`frame.rs` の本体）に文字の層への分岐が入り、照会で足りるものを 2 本目の経路で運ぶ。適用の失敗（合成失敗）を自前で見分ける必要があり、表示層の結果と食い違う余地がある。A1 の方が表面が小さい。

**案 A3 — `\s` の受け取りで付け替える（brief の候補 1）**

- ❌ `apply_cue` は `World` を持たない（`actor.rs:201`・`:306`）。仮に付け替えても、絵が先に替わる並び（実機で起きた方）は直らず、`\s` が先の並びでは「前の絵の上に新しい置き場所」を増やす。要件 1.1 の両方の並びを満たせない。**不採用の見込み**（記録のため残す）。

### `Status` の時機（要件 2）

**案 S1 — 届けをフレームの終わり（提示の後）へ移す**

- `report_balloons` の呼び出しを可視性の相から外し、`emo2_frame_system` の `run_text_phase` の後で毎フレーム呼ぶ（`frame.rs:384` の後）。窓の可視（表示層）と箱の写し（このフレームの提示）がどちらもフレームの最終の姿になる。
- ✅ 判定を複製しない。普通のバルーンの届けもフレームの中で後ろへ動くだけで、1 フレームごとの数は変わらない見込み（要件 4.5）。
- ❌ `run_text_phase` が時刻未確立で提示を飛ばすフレーム（`scale_text.rs:288-291`）は写しが作り直されない。ただし箱の文字は台詞の後にしか無いので実害は無い見込み（設計で確かめる）。
- ❌ `report_balloons` は文字の層を借りる（`borrow_runtime`）。可視性の相の外へ出すと、借用失敗の記録の置き場所を決め直す必要がある。

**案 S2 — 可視性の相の時点で「このフレームに出る箱」を判定する**

- 可視性の相の前に箱の同期を済ませる、または写しの代わりに「登録あり・見えている字が 1 以上・隠す印なし」を純粋に判定して届ける。
- ❌ 同期を可視性より前へ動かすと、時間切れで立てた「隠す印」が同じフレームの同期に間に合わず、隠したフレームに画素だけ残る（2.4 の支えを崩す）。純粋な判定にすると、提示の失敗（`present_frame` の `Err`）と食い違う余地が残る。

**案 S3 — 写しを「最後に提示した姿」のまま保ち、外すのを提示まで待つ**

- ❌ 要件 2.3（遅れて外さない）に反する。遅らせる手当てに当たる。**不採用の見込み**。

### 箱だけのときの番号（要件 3）

**案 N1 — 届けの台帳が最後に取れた番号を覚える（推奨候補）**

- `BalloonStatusLedger` にスコープごとの「最後に取れた `current_surface_id`」を足し、窓が見えず箱だけのときはそれを使う（取れていれば警告しない）。
- ✅ `status_report.rs` だけで閉じる。`\b[数字]` を隠した後に打っても、seriko の `ShowSurface` が確立して `Some` が返る（`show.rs:354`）ので台帳が追う。`balloon-canon-residue` が面の決め方を変えても、表示層の値を写すだけなので自動で揃う。
- ❌ 表示層の外に番号の写しが 1 つ増える（ただし台帳は既に「最後に送った組」を持つ帳簿）。ゴーストごとに新品（要件 4.8・`frame_visibility_integration_tests.rs:535-578`）なので、切替の直後は装着の相の確立（面 0）を最初の値として拾う必要がある。

**案 N2 — 表示層に「最後に確立した面」の照会を足す**

- `last_show` の番号を返す additive な照会を emo-present に足す。
- ✅ 真実源が 1 つのまま。
- ❌ 同じウェーブの約束で emo-present は触らない（brief）。ウェーブの外へ回すか約束を変える必要がある。

**案 N3 — 箱だけのときは警告の水準を下げ、番号は 0 のまま**

- ❌ 普通のバルーンを `\b[2]` などに切り替えてから隠した場合、要件 3.1（5.5 の規則＝今の面の番号）に対して 0 が誤りになりうる。要件 3.2 だけを満たす。

## 5. 組み合わせと規模・危険

- 筋の良い組: **A1 ＋ S1 ＋ N1**。触るファイルは `areka-emo-text/src/{actor_box.rs, actor.rs（欄の追加）, state_route.rs（必要なら）}`・`areka/src/emo2_boot/{frame.rs, frame/scale_text.rs, frame/status_report.rs, balloon_visibility_phase.rs}`＋各テスト。`frame/{attach,switch,wiring}.rs`・emo-present・`input_events/` には触らない（brief の約束の範囲内）。
- **規模: M**（5〜8 タスク）。置き場所の基準の付け替え・`Status` の時機の付け替え・番号の台帳・赤のテスト 3 群・既存の箱のテストの組み直し。
- **危険: 中**。相の並びを動かす（`frame.rs`）ので、可視性・時間切れ・箱のポインタの前段など `shown_boxes` の他の読み手の意味が変わる。新しい技術は無い。

## 6. 設計で決めること（要件討議へ渡す）

1. **置き場所の基準**: 箱の置き場所と「出ているか」の判定を、文字の層が受け取った `\s` から「いま表示している絵の番号（`current_surface_id(shell_target)`）」へ移すか（A1）。移すなら、文字の層の中で 2 つの番号（行き先用・表示用）の使い分けをどこで線引きするか。
2. **`Status` の届けの時機**: 届けをフレームの終わり（提示の後）へ移すか（S1）、可視性の相の時点で判定をそろえるか（S2）。S1 なら借用失敗の記録と、提示を飛ばすフレームの扱い。
3. **普通のバルーンの窓の判断の基準**（要件 2.2 の周辺）: `balloon_shown_glyphs`（窓を出すか）も絵の番号基準へそろえるか。箱だけ移すと、窓が消えるフレーム（`\s` 基準）と箱が出るフレーム（絵基準）が分かれ、両方とも出ていない 1 フレームが生じうる（`Status` は要件 2.3 どおりそのフレームで外すので要件違反ではないが、画面の受け渡しは揃わない）。そろえると、`\s` の後・絵の差し替えの前に書いた文字の扱い（行き先は新しい面の箱・まだ表示できない）を明記する必要がある。
   - **2026-10-04 要件討議で決着（案 a）**: 窓の判断も絵の番号基準へそろえる。箱を持つシェルで窓と箱のあいだを文字が移るときは、窓と箱を絵と同じフレームで入れ替える（要件 1.8）。`\s` の後・絵の差し替えの前に書いた文字は要件 6 の行き先へ書き、絵が替わるフレームで表示する（要件 1.9）。箱を持たないシェルでの窓の振る舞いは変えない（要件 4.5）。設計で決めるのは、2 つの番号（行き先用・表示用）の線引きと `balloon_shown_glyphs` の基準の付け替え方。
4. **箱だけのときの番号の持ち方**: 台帳が最後に取れた番号を覚える（N1）か、表示層に照会を足す（N2・ウェーブの約束に触れる）か。N1 なら「最後に取れた番号」の意味を `balloon-canon-residue` の面の番号の規則（偶数・奇数）とどう揃えるかを文書に残す（表示層の値の写しなら自動で揃う）。
5. **`shown_boxes` の他の読み手への波及**: 箱のポインタの前段（`input_events/shell_box_handler.rs:82`・触らない約束）と、可視性の相の時間切れ／中断の観測（`balloon_visibility_phase.rs:394`）は、写しの意味の変化（`\s` の受け取りで外れなくなる）をそのまま受ける。意図どおりか（当たり判定は本 spec の範囲外だが、画素と同じ時機にそろうことを確かめるか）。
6. **既存の箱のテストの組み直し**: `frame_shell_box_integration_tests.rs` の 3 本と emo-text の箱の同期の檻を、絵の差し替え（`ShowSurface`）も送る形・絵の番号を渡す形に直す。`status_report_tests.rs:105-119` の「箱だけ・番号なし → 警告の対象」の期待を要件 3 に合わせて改める。どれも箱のテストで、要件 4.5 の対象外であることを設計に書く。

## 7. 調べが要ること（Research Needed）

- **新しい窓の子が同じフレームに出るか**: 置き場所が替わると箱の面は despawn → 同じフレームの提示で `attach_window_child` し直す（`actor_box.rs:425-440`・`actor_present.rs:199`）。新しい entity の見た目が絵の差し替えと同じ合成の確定に乗るか（実機では絵が先の回でも「付け替えたフレーム」に揃っていたので乗っている見込み・`real-machine-check.md:468`）。乗らないなら、面を作り直さず位置だけ動かす形を検討する。
- **シェルに在る番号でも合成で落ちる場合**（要件 1.6 の周辺）: 文字の層の解決は面の表で `Unresolved` にする（`shell_box_assets.rs:64-66`）一方、seriko は解決できれば `ShowSurface` を送り、合成失敗なら表示は前のまま（`current_surface_id` 不変）。絵の番号基準ならこの場合も「今の置き場所のまま」になる見込みだが、檻で確かめる。
- **SERIKO のアニメーションで土台の面が替わる場合**: `current_surface_id` は `ShowSurface` の番号であり、アニメーションの差し替えで動くかを確かめる（動かないなら箱の置き場所も動かない＝今と同じ）。
- **`Cage` の絵の番号**: 檻の絵は emo2 の検体のシェル、箱の表は檻の文面（`frame_shell_box_integration_tests.rs:37-48`）。面 0 と面 10 の `ShowSurface` が検体のシェルで合成できるか（できなければ檻の面の番号を選び直す）。

## 8. 次の段

- 要件討議（`/kiro-requirements-discussion areka-P0-shell-balloon-frame-align`）で 6 節の 1〜6 を扱い、その後 `/kiro-design areka-P0-shell-balloon-frame-align`。

---

# 設計フェーズの記録（2026-10-04 `kiro-spec-design`）

## Summary

- **Feature**: `areka-P0-shell-balloon-frame-align`
- **Discovery Scope**: Extension（既存の文字の層と結線の直し。軽い discovery＝統合点と既存の型の確認だけ。外の依存は足さない）
- **Key Findings**:
  - 絵の番号を文字の層に覚えさせる必要は無い。使う所（箱の同期・窓に出す文字の数）はどちらもフレームの中で表示層を持つ結線から呼ばれるので、引数で渡せば足りる。文字の層に「表示用の番号」の欄を作らない＝番号の写しが増えない。
  - `box_still_shown` の「受け取った `\s` の番号」の条件は、足し替えでなく**外す**だけでよい。登録（`box_sites`）は直近の同期で絵の番号から導いた置き場所で、絵はフレームの外では替わらないから、登録があること自体が絵と合っていることを表す。
  - `Status` の届けを外しても、可視性の相の檻（`balloon_visibility_phase_tests.rs` ほか）は届けを見ていない。届けを数える檻は `emo2_frame_system` を丸ごと回す `frame_visibility_integration_tests.rs`（とその子）と、純関数の `status_report_tests.rs` だけである（要件 4.5 を保てる）。

## Research Log

### 7 節の「調べが要ること」の答え

- **SERIKO のアニメーションで絵の番号が動くか**: 動かない。seriko の `DisplayCommand::Show` はスコープの面の番号（`surface_id`）とアニメーションの進み具合（`pattern`）を別の欄で運び（`areka-seriko/src/output.rs`）、表示層は `ShowSurface` の `surface_id` を `current_surface_id` に書く（`presenter/show.rs` の表示成立点）。コマの差し替えでは番号は同じ。
- **シェルに在る番号でも合成で落ちる場合**: 表示層は解決できない番号・合成の失敗を `error!` で記録して表示成立点より前で戻るので、`current_surface_id` は前のまま（`presenter/show.rs` の `Err(e)` の腕）。全透明への退化だけは `Hide` と同じ扱いで `None` になる（同ファイルの `EmptyComposition` の腕）。絵の番号を基準にすれば、前者は「今の置き場所のまま」、後者は「絵が消えたので箱も消す」になる。
- **新しい窓の子が同じフレームに出るか**: 置き場所が替わるときの「面を片付けて登録し直し、同じフレームの提示で付け直す」道は完了 spec のままで、本 spec は呼ばれる時機だけを変える。実機の記録（`real-machine-check.md` の項目 4）では付け替えたフレームに揃っていた。任意の実機の確かめで見る。
- **檻の絵の番号**: `Cage::boot` は面 0 の `ShowSurface` を既に送って確立させている。面 10 は emo2 の検体で `\1` の面として使われる番号。合成できなければ檻の中の番号だけを選び直す（実装の最初のタスクで確かめる）。

### 可視性の判断は入力の数え方だけで足りるか

- **Sources**: `balloon_visibility_decision.rs` の `decide_content`。
- **Findings**: 窓 → 箱は「文字の数が 0 へ落ち、現に可視」の腕（契機 `Clear`・`hide_reaches_boxes` が偽なので箱へ届かない）、箱 → 窓は「0 からの増加・現に不可視」の腕がそのまま当たる。既存の檻（`box_surface_withholds_the_balloon_window_and_a_plain_surface_shows_it`）が、サーフェスの切替だけで数が 0 ↔ n に動く形を既に通している。
- **Implications**: `decide` は触らない。`balloon_shown_glyphs` の基準を絵の番号に替えるだけで、窓の表示・非表示が絵と同じフレームに乗る（要件 1.8）。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A1 引数で渡す（採用） | 結線が毎フレーム絵の番号を `sync_boxes`・`balloon_shown_glyphs` へ渡す | 番号の写しが増えない・届く順に依らない・emo-present 無変更 | 既存の箱の檻を絵も送る形に直す | ギャップ分析の A1 を「覚えさせず渡す」形に絞った |
| A1' 文字の層に欄を持つ | 文字の層がスコープごとの「表示している番号」を覚える | 呼び出しの引数が増えない | 表示層の値の写しが 1 つ増える・書く時機を別に決める必要 | 不採用（写しを増やさない） |
| A2 drain で知らせる | `run_drain_phase` が適用のたびに文字の層へ知らせる | 変化の瞬間が分かる | 照会で足りるものを 2 本目の経路で運ぶ・合成失敗の見分けを複製 | 不採用 |
| S1 届けをフレームの終わりへ（採用） | `run_text_phase` の後に届けの相 | 判定を複製しない・窓と箱の最終の姿を読む | 借用の失敗の記録を置き直す | |
| S2 可視性の相で先読み | 同期を前へ動かす／純粋に判定する | 相を足さない | 「隠す印 → 同期 → 提示」の並びを崩す・提示の失敗と食い違う | 不採用 |
| N1 台帳が番号を覚える（採用） | `BalloonStatusLedger` に最後に取れた番号 | `status_report.rs` で閉じる | 同じフレームの「切り替えてすぐ隠す」は追えない | |
| N2 表示層に照会を足す | `last_show` の番号を公開 | 真実源が 1 つ | 同じウェーブの約束（emo-present に触らない）に触れる | 不採用（N1 の限界が問題になったときの上げ先） |

## Design Decisions

### Decision: 絵の番号は文字の層に覚えさせず、引数で渡す

- **Context**: 6 節の 1（2 つの番号の線引き）。
- **Alternatives Considered**: A1'（欄を持つ）・A2（drain で知らせる）。
- **Selected Approach**: 行き先用の番号は `state_route.rs` の中だけで読む。表示用の番号は結線が `EmoPresenter::current_surface_id(shell_target(scope))` を毎フレーム渡す。`actor_box.rs` は `state.current_surface` を読まなくなる。
- **Rationale**: 0 フレームで解くには、絵と同じ真実源から同じフレームの中で導くのが最短。覚えさせると「いつ書くか」という新しい時機の問題が生まれる。
- **Trade-offs**: `sync_boxes`・`balloon_shown_glyphs` の引数が増え、箱の檻を直す。
- **Follow-up**: 実装後に `actor_box.rs` で `current_surface` を grep して 0 件を確かめる。

### Decision: 写しの刈り込みから番号の条件を外す

- **Context**: 要件 2.1（`\s` の受け取りで写しが外れて `balloon(ID群)` が欠ける）。
- **Selected Approach**: `box_still_shown` を「登録がある・隠す印が無い・文字を持つ」にする。`prune_shown_boxes` の呼び出しは残す（`\c`・台詞の頭ではその場で外す）。
- **Rationale**: 足すのでなく削るだけで要件を満たす。写しへ足すのは提示だけ、という不変条件は変わらない。
- **Trade-offs**: 箱のポインタの前段（`input_events/shell_box_handler.rs`）が読む写しも `\s` の受け取りでは外れなくなる。画素と同じ時機になるので意図どおり（6 節の 5）。

### Decision: 届けは `run_text_phase` の後の独立した相

- **Context**: 6 節の 2。
- **Selected Approach**: `status_report.rs` に `run_status_report_phase` を足し、`emo2_frame_system` の最後で呼ぶ。可視性の相からは呼び出しを外す。借用の失敗は台帳の旗で 1 度だけ記録。
- **Rationale**: 提示の後なら窓と箱の両方が最終の姿。可視性 → 同期 → 提示の並びを動かさずに済む。
- **Follow-up**: `frame_visibility_integration_tests.rs` の普通のバルーンの 2 本が期待値を変えずに通ること。

### Decision: 箱だけのときの番号は台帳が覚える。窓が見えているときの規則は変えない

- **Context**: 6 節の 4・要件 3。
- **Selected Approach**: `report_observed` の頭で、番号の取れたスコープを `last_surface` へ書く。`collect_bindings` は箱だけ・今の番号なしのときに覚えた番号（無ければ 0）を使い、警告の対象にしない。
- **Rationale**: 「無ければ 0」は完了 spec の要件 5.5 の「切り替えていなければ 0」そのもの。普通のバルーンの警告の檻は変えずに済む（境界の外）。
- **Trade-offs**: 同じフレームの「切り替えてすぐ隠す」は追えない（design.md に限界として記載）。

### Decision: 要件 1.8 は「移る先にそのフレームで見えている文字がある」ときの入れ替わりと読む

- **Context**: 絵が先に届き、移る先にまだ文字が無い（保持した文字も、`\s` の後の文字も無い）場合、絵が替わるフレームで前の側の表示をやめる（要件 1.2）一方、移る先には出すものが無い。
- **Selected Approach**: 空の窓・空の箱は出さない（完了 spec の規則のまま）。文字が届いたフレームで移る先に出る。枠の檻は、両側に文字を持たせた形で両方の並びを確かめる。
- **Rationale**: 要件 1.8 の条件は「文字の表示が…移る場合」で、表示する文字が無ければ移るものが無い。要件 1.2 と、内容の無いバルーンを出さない既存の規則から一意に決まる。

## Risks & Mitigations

- 絵の差し替えが失敗し続けると、`\s` の後の文字が出ないまま溜まる — 失敗は表示層が `error!` で記録済み。次に絵が替われば出る。design.md の Error Handling に明記。
- 相を 1 つ足すことで、届けを前提にした他の相の順が崩れる — 届けは kanade への片道の送出だけで、同じフレームの他の相は結果を読まない。
- 既存の箱の檻の直し漏れ — 引数の形が変わるのでコンパイルで分かる。絵を送らない檻は期待が外れて赤になる。
- 並走の `balloon-canon-residue` が番号の出どころを変える — 台帳は表示層の値を写すだけ。決定論テストの期待値だけ見直す（Revalidation Triggers）。

## References

- `.kiro/specs/completed/areka-P0-shell-balloon/requirements.md` — 要件 5.5・5.6・6（番号の規則と行き先の規則）
- `.kiro/specs/completed/areka-P0-shell-balloon/real-machine-check.md` — 16 回中 2 回のずれと警告の実機の記録
- ukadoc `spec_shiori3` の `Status` — `balloon(ID群)` の定義

### 2026-10-04 設計討議 議題 1: 要件 1.8 の文面

- 決着: 要件 1.8 を「送り先の切り替え」で書き直した。絵が替わるフレームで送り先（窓か箱か）は必ず切り替わる。送り先は表示されていなくても存在するので、移る先が空のあいだ画面に何も出ないのはずれではない。「窓も箱も表示されていないフレームを作らない」は外した。
- 動きは設計の読みのまま。枠の檻に「移る先が空」の場面を足した。

### 2026-10-04 設計討議 議題 2: `frame/wiring.rs` の説明文

- 決着: 触らない約束の例外として、`Emo2Wiring` の説明文 2 か所だけ直す（コードは変えない）。届けの相が移ると説明文が事実と合わなくなるため。並んで走る spec が同じ行を直していたら、マージで手直しする。

# 直す前の赤の記録（2026-10-04 タスク 1.2）

- 走らせたコミット: `98e542df`（タスク 1.1 の足場まで・本番のコードは直す前のまま）に、枠の檻（`crates/areka/src/emo2_boot/frame_shell_box_align_tests.rs`）を足した作業木。
- コマンド: `cargo test -p areka --bin areka shell_box_integration_tests` → 4 passed・7 failed。通ったのは既存の 3 本と 1.1 の確かめの 1 本。落ちた 7 本はどれも前提（`speak_in_box`・`speak_in_both`・`Cage::boot` の前提の判定）を通った後の、ずれの場面の判定で落ちた（組み立ての失敗・コンパイルの失敗・前提での失敗は 0 件）。
- 判定の形: 毎フレームの終わりに `Look`（絵の番号・箱の四角の左上・窓の可視）と届いた組を比べる。左が実際、右が直した後に成り立つべき値。DPI 96 の檻では拡大率が 1 なので、箱の四角の左上は element定義の X,Y（面 0＝`10,10`・面 10＝`60,30`）そのもの。

| 檻 | テスト名（`align_tests::` の下） | 落ちた判定 | 左（実際） | 右（あるべき姿） |
|----|-------------------------------|------------|------------|------------------|
| 1 | `box_moves_with_the_picture_when_the_picture_arrives_first` | 絵だけ届いたフレームの `Look`（要件 1.1） | 絵 10・箱 `(10,10)`・窓 隠れ | 絵 10・箱 `(60,30)`・窓 隠れ |
| 2 | `box_stays_with_the_picture_when_the_script_surface_arrives_first` | `\s` だけ届いたフレームの `Look`（要件 1.1） | 絵 0・箱 `(60,30)`・窓 隠れ | 絵 0・箱 `(10,10)`・窓 隠れ |
| 3（絵が先） | `window_and_box_swap_with_the_picture_when_the_picture_arrives_first` | 箱 → 窓で絵が替わったフレームの `Look`（要件 1.2・1.8） | 絵 10・箱 `(10,10)`・窓 隠れ | 絵 10・箱なし・窓 出る |
| 3（`\s` が先） | `window_and_box_swap_with_the_picture_when_the_script_surface_arrives_first` | 箱 → 窓で `\s` だけ届いたフレームの `Look`（要件 1.8） | 絵 0・箱なし・窓 出る | 絵 0・箱 `(10,10)`・窓 隠れ |
| 3（移る先が空） | `moving_into_an_empty_window_drops_the_scope_until_the_first_glyph` | 絵が替わったフレームの `Look`（要件 1.2・1.8） | 絵 10・箱 `(10,10)`・窓 隠れ | 絵 10・箱なし・窓 隠れ |
| 4 | `text_written_before_the_picture_arrives_appears_with_the_picture` | 箱 → 窓で `\s` と文字だけ届いたフレームの `Look`（要件 1.9） | 絵 0・箱なし・窓 出る | 絵 0・箱 `(10,10)`・窓 隠れ |
| 5 | `box_only_after_hiding_the_window_reports_without_a_warning` | 箱にだけ文字が出た 2 フレームの、警告の段の `balloon_status_surface_unknown` の行の数（要件 3.2） | 1（`見えているバルーンの番号が取れないので 0 として届ける`・scope 0） | 0 |

読み:

- 1・3（絵が先）・3（移る先が空）は「新しい絵の上に前の置き場所の文字が残る」ずれ（箱の同期が受け取った `\s` の番号＝面 0 を見ている）。
- 2・3（`\s` が先）・4 は「前の絵の上に新しい置き場所の文字が出る／窓が先に出る」ずれ（`\s` を受け取った時点で箱の同期と窓に出す文字の数が面 10 へ動く）。
- 2 では、コードの読みでは同じフレームに空の組も届く（`\s` の受け取りで写しが刈られ、可視性の相の届けが空を見る）が、先に `Look` の判定で落ちるので組の判定までは走っていない（実測していない）。組の判定はそれぞれの `Look` の判定の直後に置いてあり、置き場所の判定が直った後も 1 フレームの空の組が残れば、そこで落ちる。
- 5 は普通のバルーンを一度出して隠した後（表示層の番号が `None`）に箱だけの組を届ける場面で、警告の段の行が 1 件出ることで落ちる。組の判定（`(0, 0)` が 1 度）は警告の判定の後にあるので、直す前のコードでは走っていない。
