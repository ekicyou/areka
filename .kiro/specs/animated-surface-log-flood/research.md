# ギャップ分析: animated-surface-log-flood

> 2026-10-30・ブランチ `claude/animated-surface-log-flood-54fed6`（main `484263230` の上）で、確定した `requirements.md` と今のコードを突き合わせた。決めは書かず、選べる案と材料だけを書く。行番号はこの時点のもの。

## 1. まとめ

- **記録の出し分け（要件 1・2）は今の材料だけで書ける**。表示の行は `crates/areka-emo-present/src/presenter/show.rs:386-403` の `tracing::info!` 1 か所で、「コマ送りだけの適用」の見分けに要る値（前の面と着せ替え＝`last_show`・前の拡大率＝`applied`・窓の大きさの替わり＝`size_changed`・初めて＝`last_show` が空）はどれも同じ関数の中にもう在る。性能の道具はどれも `RUST_LOG` に `areka_emo_present=debug` を入れているので、行を `debug!` へ下げても道具・見本・判定は 1 行も変わらない。ただし `trace!` へ下げると道具から行が消える（§3.1）。
- **コマ替えの重ね直し（要件 3）は、emo-present の中だけでは要件 3.1 を満たせない**。合成は `Composer::compose_into`（`crates/areka-emo-compose/src/lib.rs:137`）の 1 回呼びで、どの層が動くかは合成の内側（`plan::build_plan`・`blit::execute`、どちらも `pub(crate)`）にしか見えない。emo-present からは仕上がった 1 枚の絵しか受け取れない（§3.3）。
- **ぱすたの面 1002 の実物を測った**（ワークツリーの `target\nar-samples\manual\pasta-in-windows`）。キャンバスは 554×684、動くボールは 140×140 の 3 枚（コマ数 40・48・40、どれも 1 コマ 70 ms＝毎秒およそ 14.3 回）で、面の約 5% にすぎない。3 枚の組み合わせは 240 通り（40 と 48 の最小公倍数）あるので、今の「容量 3 の控え」を広げて当てる形は 240 枚×1.5 MB＝約 364 MB になり使えない（§3.4）。
- **重ね直しの案は 3 つある**。⒜ 控えの数や GPU の絵の持ち方だけを変える（emo-present の中だけ・要件 3.1 を満たさない）、⒝ 動く層の下までを焼いた土台を控えて、その上だけ重ねる（emo-compose に口が要る）、⒞ 前の絵を写して、変わった矩形だけを重ね直す（emo-compose に口が要る・画素が同じになることを示しやすい）。整数の重ね方の丸めのため、動く層より上の層を「先にまとめて焼いておく」形は画素単位で同じにならない（§4.2）。
- **調べが要ること**: ⑴ 1 回の適用の重さの内訳（合成・当たりのマスク・GPU への上げ・wintf の描き直し・測っていない区間）を、ぱすたの面 1002 で測っていない。release 版の 20% が合成をやめるだけで SSP 並みに下がるかは分からない。⑵ D2D のコマンドリストが絵を参照で持つか（持つなら、絵を使い回して中身だけ書き替えられる）。⑶ `seriko-pattern-calls` の `move` が土台の層を動かすので、「動く層」の見分け方に効く（§5・議題 4）。

## 2. 調べた範囲

- 仕様: `spec.json`（言語 `ja`）・`requirements.md`・`brief.md`。
- steering: `product.md`・`tech.md`・`structure.md`（1 ファイル 1,000 行の目安・兄弟のテストの置き方）・`logging.md`（水準の選び方・`log-capture-kit`）。
- コード: `crates/areka-emo-present/src/presenter/{show.rs, hub.rs, target.rs, timing.rs, budget.rs, refresh.rs, replace.rs, visibility.rs}`・`cache.rs`・`display.rs`・`mount.rs`・`balloon.rs`、`crates/areka-emo-compose/src/{lib.rs, plan.rs, blit.rs, pattern.rs, film.rs}`、`crates/wintf/src/ecs/graphics/systems/{render.rs, surface.rs}`・`crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`、`crates/areka/src/log_history.rs`・`crates/areka/src/emo2_boot/{adapter.rs, frame/attach.rs}`・`crates/areka-seriko/src/state.rs`。
- 道具と手順書: `tools/perf/{judge-perf.py, judge-followup.py, perf-rank.py, invoke-perf-run.ps1, invoke-followup-checks.ps1}`・`tools/perf/goals/draw-load-parity.toml`・`doc/pasta-real-run.md`。
- 検体: ぱすたのシェル `shell/master/surfaces.txt` と `window/ball/*.webp`（RIFF の塊を読んでコマ数と待ち時間を数えた）。
- 実機の走行・SSP・areka のプロセスには触れていない。

## 3. 今の形

### 3.1 表示の行と、それを読む側

- 行は `show.rs:386-403` の 1 か所。target は `areka_emo_present::presenter::show`。文の頭の注記（`show.rs:381-385`）が「実機サインオフは `RUST_LOG` を grep するので `info!` が契約」と書いている＝要件 1 で変えるなら、この注記も書き直す。
- 失敗の道（未装着・GraphicsCore 不在・表示の記録の失敗・合成の失敗・全透明の縮退）はどれもこの行より前で `error!`／`warn!` を出して早く戻る。行の水準を変えても失敗の記録は動かない（要件 1.5 は今のまま満たす）。
- すぐ下の perf の行（`timing.rs:195-213`）は前から `debug!`。要件 2.3 の「対で隣り合う」は、表示の行を `debug!` にすれば、詳しく出す水準で今と同じに保てる。
- 読む側（`.kiro/specs/completed` と `target` を除いて文言を grep した）:

| 読む側 | 何を読むか | `debug!` に下げたときの影響 |
|---|---|---|
| `tools/perf/invoke-perf-run.ps1:161` | `RUST_LOG='info,areka_emo_present=debug'` で回す | 変わらない（行はすべて出る） |
| `tools/perf/invoke-followup-checks.ps1:108,137,497` | 同じ水準で回し、表示が済んだ印として最初の表示の行を待つ | 変わらない（最初の表示はどの案でも INFO） |
| `tools/perf/judge-perf.py:507`・`judge-followup.py:115`・`perf-rank.py:118` | 文言で数える・突き合わせる | 変わらない。ただし `judge-perf.py:506` の注記は `show.rs:359`（もう古い。今は 402）・「info!」と書いている。`judge-followup.py:107` は `show.rs:402` |
| `tools/perf/fixtures/**/run.log`・`fixtures-loop/**/run.log` | 記録の見本 | 変わらない（見本は詳しく出す水準の記録） |
| `doc/pasta-real-run.md:127,141` | `RUST_LOG='info,areka::boot_resolve=debug,…'`（`areka_emo_present` は入っていない） | コマ送りだけの行が消える（これが狙い） |
| `doc/pasta-real-run.md:314` | 「着地するまで 9 割が `apply(ShowSurface)`……外して読む」の注記 | 書き直す（要件 8.6） |
| `crates/areka-emo-present/src/presenter_refresh_and_log_tests.rs:65-76` | 最初の表示の行が `Level::INFO` であることを確かめる | 変わらない（最初の表示は INFO のまま） |
| `presenter_refresh_and_log_tests.rs:247-257` | 拡大率だけが替わった適用の行の `cache_hit=true` | 変わらない（拡大率の替わりは INFO・議題 2 の推奨どおりなら） |
| `presenter_film_tests.rs:26,228-238` | 文言で数えて「成立した回数・合成した回数」を出す | 捕捉の水準しだい。`log-capture-kit` は TRACE まで拾う（`capture_tests.rs:153`）ので数は変わらない見込み。設計で確かめる |
| `presenter_perf_log_tests.rs`（883 行）・`hub.rs`・`timing.rs`・`crates/areka-emo-text/src/actor_scroll_retain_tests.rs` | 文言と対の関係 | 文言は変えないので、水準の主張があるものだけ直す |
| `crates/areka/examples/{collision-probe.rs, emo-present.rs, emo-present/state.rs, emo-present/systems.rs, window-placement.rs}` | ほとんどは注記。最初の表示の行を読む | 最初の表示は INFO のままなので、注記の書き直しが主 |

- **`get_log`（MCP）の履歴には効かない**: `crates/areka/src/log_history.rs:115-131` の振り分けの表に `areka_emo_present` は無く、この行はもともと履歴に残っていない。要件 1 は標準出力の記録だけの話になる。
- **`trace!` は選べない**: 性能の道具の水準は `areka_emo_present=debug` なので、`trace!` に下げると道具から行が消え、要件 2.1・2.4 を壊す。⒜ の「`debug!`／`trace!`」は `debug!` に絞られる。
- **`tracing` の水準は字面で決める**: `tracing::event!` の水準は定数でなければならず、「同じ欄で水準だけ替える」は 2 つの呼び出しに分けることになる。欄の並びが 2 か所でずれないよう、局所の `macro_rules!` か、欄の並びを 1 か所にまとめる工夫が要る（要件 2.2 の「文言と欄を変えない」を、2 か所が同じであることのテストで縛るか、構造で縛るか）。

### 3.2 「コマ送りだけの適用」を見分ける材料

- `apply_show` の中に、要件 1 の定義に要るものが全部ある: 前に成立した面と着せ替えとコマ＝`target.last_show`（`target.rs:140`・成立点 `show.rs:379` でだけ書く）、前の拡大率＝`target.applied`（`show.rs:369`）、窓の大きさの替わり＝`size_changed`（`show.rs:291`）、窓に付けてから初めて＝`last_show` が `None`。
- 付け直し（`replace.rs`）は `attach_target` で新しい target を作るので `last_show` は空から始まる（`hub.rs:84`）＝要件 1.4 の「付け直した後の最初」はそのまま INFO になる。`applied`・`native_size` は引き継ぐので、そのときの `size_changed` は false になりうるが、「初めて」の判定で INFO に残る。
- **同じ形の述語がもうある**: `hub.rs:183-188`（`show_or_hold`）が「面の番号と着せ替えが `last_show` と同じで、コマだけ違う」を、隠れているバルーンのコマを預かる判定に使っている。要件 1 の判定はこれに拡大率と窓の大きさを足したものになる。
- **`Hide` は `last_show` を消さない**（`target.rs:136-138`・`hub.rs:217-240`）。隠した後に同じ面・同じコマで出し直す適用（`visibility.rs:108` の `show_target`・シェルの `\s[-1]` の後の `\s[同じ面]`）は、要件 1 の定義どおりだと「コマ送りだけ」になり、既定の水準では出ない。見えなかった窓が見えるようになった適用を INFO に残すかは、要件の文に無い（判断項目 2）。

### 3.3 合成から表示までの 1 回の適用

`apply_show` が引き当てを外したとき（ぱすたの面 1002 では毎回）、次を面の全体で行う。

1. **合成**（`show.rs:102-116` → `Composer::compose_into`）: `plan::build_plan` が層の命令列（`BlitOp` の列・下から上の順）を作り、`blit::execute` が合成先を `resize_and_clear` で消してから全部の命令を整数の重ね方で写す。動く絵のコマは `PatternState` の `films`（`pattern.rs:39`）で選ばれ、`plan_always::push_film_op` がその element の位置に命令を 1 つ積む。合成器は結果を持たない（`lib.rs:77-110`）。
2. **表示の記録**（`show.rs:129-160` → `display.rs:142`）: 毎回、原寸の D2D の絵を `CreateBitmap` で作り直し（全面を GPU へ上げる）、コマンドリストを新しく作って閉じる。
3. **当たりのマスク**（`show.rs:185-191` → `budget.rs:504`）: 全面の α から作り直す（入れ物は輪番で使い回す）。wintf の `AlphaMask` には全面の作り直し（`regenerate_from_pbgra32`）しかなく、矩形だけ直す口は無い。
4. **控えへ入れる**（`cache.rs`）: 合成入力（面・着せ替え・コマ）を鍵にした容量 3 の LRU。鍵が 1 か所でも違えば外れる。
5. **wintf の描き直し**（`render.rs:25`）: 表示の記録が替わると `mark_dirty_surfaces`（`surface.rs:30-43`・`Changed<GraphicsCommandList>` など）が印を付け、`render_surface` が窓の面を全面で描き直す。**WUC の `BeginDraw` は全面でしか呼べない**（`render.rs:63-64` の注記・部分矩形は `E_INVALIDARG`）＝ここは本 spec では縮められない。

要するに、ボールが 1 コマ動くたびに、面の 95% を占める変わらない部分まで、消す・重ねる・α を読む・GPU へ上げる・描き直すを全部やっている。emo-present から見えるのは 1 の仕上がりの 1 枚だけで、どの層が動いたかは分からない。

### 3.4 ぱすたの面 1002 の実物

`surfaces.txt:290-302` と画像のヘッダーを読んだ。

| 順 | element | 絵 | 大きさ | 動くか |
|---|---|---|---|---|
| 0 | element0 | `window/canvas.png` (0,0) | 554×684 | 動かない |
| 1 | element9 | `pasta/null.png` (234,264) | 320×420 | 動かない（透明） |
| 2 | element1 | `window/ball/glow.png` (22,314) | 180×180 | 動かない |
| 3 | element2 | `window/ball/eback.webp` (42,334) | 140×140 | 40 コマ・70 ms・繰り返し |
| 4 | element3 | `window/ball/base_rings.webp` (42,334) | 140×140 | 48 コマ・70 ms・繰り返し |
| 5 | element4 | `window/ball/efront.webp` (42,334) | 140×140 | 40 コマ・70 ms・繰り返し |
| 上 | animation1500（bind） | 面 1510 の overlay | — | 着せ替え（赤い ×）。着ているときだけ上に乗る |

- 3 枚のコマは全部 70 ms なので、適用は毎秒およそ 14.3 回（件 25 の「毎秒およそ 14 回」と合う）。
- **3 枚は隣り合っていて、あいだに変わらない層を挟まない**。要件 3.3 の括弧書き「そのあいだに変わらない層を挟むとき（ぱすたの面 1002 のボールの 3 枚）」は、面 1002 の形とは合わない。挟む形を確かめるには作った見本が要る（判断項目 7）。
- 面 1000（ふだんのぱすた）にも同じ 3 枚のボールがある（`surfaces.txt:139-141`）。ほかに SERIKO のアニメーション（口・まばたき・赤い ×）があるが、注記ではボールと「空間的に非重複」。
- 組み合わせは 240 通り（40 と 48 の最小公倍数）。今の控えを広げて当てる形は 240×1.5 MB（原寸 554×684×4）＝約 364 MB に、マスクと GPU の絵が加わる。要件 3.6（記憶に上限）と合わない。

### 3.5 バルーンの動く絵も同じ道を通るか（要件の作者が残した点）

- **通る**。バルーンの面は `DisplayCommand::ShowBalloon` → `PresentCommand::ShowSurface`（`crates/areka/src/emo2_boot/adapter.rs:66-77`）で、シェルと同じ `apply_show` に入る。コマの進みも、seriko の `commit_pattern` がシェルとバルーンを同じ道で扱い（`crates/areka-seriko/src/state.rs:615-664`・`animated-image-playback` 要件 6.1）、`ShowBalloon` に新しいコマを載せて出す。
- バルーンの面画像は `{接頭辞}{ID}.png`（`balloon.rs:44`）で、1 面に overlay の element が 1 つ。APNG なら動く絵として分解されうる。
- 違いは 1 つ: バルーンは外から所有される（`VisibilityOwnership::External`）ので、隠れているあいだのコマだけの指令は `show_or_hold`（`hub.rs:162-205`）が預かり、合成も記録も 0 回。見えているあいだは、シェルと同じに毎コマ `apply_show` を通る。
- よって、**直しを `apply_show` の中に置けばバルーンにも同じに効く**。バルーンだけの別の道は無い。テストは、外から所有される target（`presenter_film_tests.rs` の `established` が使う形）でも 1 本持つとよい。

### 3.6 何もしていないあいだの CPU の手がかり

- emo2（まばたきだけ）の何もしていないあいだの release の CPU は、完了 `areka-P0-draw-load-parity` で 1 コアの 3% 未満が合格の線になっている（`tools/perf/goals/draw-load-parity.toml` の `idle_cpu_release_max_pct = 3.0`）。SSP の emo2 の参考値は 3.05%（`perf-rank.py:102-104`・2026-08-15）。
- ぱすたの release の 20% のうち、おおよそ 17 ポイントが毎秒 14 回の適用の道（＋wintf の描き直し）と見られる。1 回あたり 12 ms ほどになる。
- ただし、その内訳は測っていない。perf の行（`t_compose_us`・`t_mask_us`・`t_upload_us`・`t_total_us`）は `debug!` なので件 25 の記録（INFO）には無く、記録そのものも消えている（`<w>\target\pasta-verify-quiet\` は今は無い）。`present-emit-tail-latency` の brief は、`t_total_us` の中に測っていない区間（`set_display`・`set_layout`・マスクの受け渡し・`set_visible`・遷移の記録）があり、wintf の World への書き込みが主な候補だと書いている。wintf の描き直し（`render_surface`）と DComp の確定は perf の行の外にある。
- 粗い見積もり（測っていない）: 合成で重ねる画素は 1 回あたりおよそ 60 万（キャンバス 38 万・null 13 万・glow 3 万・ボール 3×2 万）。release で数 ms、debug でその数十倍。debug の 69% は合成の重さで説明しやすいが、release の 12 ms は合成だけでは説明しきれない見込み。**合成をやめるだけで release が SSP 並みに下がるとは限らない**（議題 3・判断項目 8）。

## 4. 要件と今の資産の対応

| 要件 | 今ある物 | 足りない物 | 種類 |
|---|---|---|---|
| 1.1〜1.4 出し分け | 判定の材料（§3.2）・`show_or_hold` の同じ形の述語 | 述語 1 つと、水準の違う 2 つの呼び出し（欄の並びをそろえる工夫） | Missing（小さい） |
| 1.5 失敗の記録 | 失敗はどれも行より前で記録して戻る | なし | — |
| 1.6 回数・時刻を変えない | 回数は seriko の進みが決める・`apply_show` は運ぶだけ | なし（変えない） | Constraint |
| 2.1〜2.3 詳しく出す水準で同じ事実 | perf の行も `debug!`・同じ関数の終わりで対 | なし（`debug!` を選べば済む） | — |
| 2.4・2.5 道具と見本 | 道具は `areka_emo_present=debug` | なし（`trace!` を選ぶと壊れる） | Constraint |
| 2.6 注記と行の位置 | `judge-perf.py:506`（もう古い）・`judge-followup.py:107`・`show.rs:381-385`・`doc/pasta-real-run.md:314`・例の注記 | 書き直し | Missing（小さい） |
| 3.1・3.3 変わらない層を作り直さない | 合成の命令列は emo-compose の中にある（`pub(crate)`） | emo-compose の外へ出す口（命令列か、途中から重ねる口か、矩形で切って重ねる口） | Missing（本体） |
| 3.2 画素が同じ | 整数の重ね方（`blit.rs:14-32`）・合成の見本の檻（`golden_tests*`） | 動く層より上の層を先にまとめると丸めがずれる（§5 案 ⒝'）。「前の絵を写して変わった所だけ」か「下から途中までの控え」なら同じになる | Constraint |
| 3.4 動かない面は変えない | 容量 3 の LRU・定常の確保 0 の檻（`presenter_budget_steady_state_tests.rs`・`presenter_cache_capacity_tests.rs`） | 新しい道が動かない面で走らない形 | Constraint |
| 3.5 合成か使い回しかを記録で読む | `cache_hit`（真偽だけ） | 3 つ目の状態（一部だけ重ね直した）の表し方。perf の行は変更 0 の約束なので、足すなら表示の行か別の行 | Unknown（判断項目 3） |
| 3.6 記憶の上限 | 控えの容量 3・予算の席（`budget.rs`） | 案しだいで 0〜1 枚（1.5 MB）の追加 | Unknown |
| 3.7 当たり判定・クリックの透過・窓 | マスクは原寸の絵から作る | マスクを矩形だけ直すなら wintf の `AlphaMask` に口が要る（今は全面だけ） | Unknown（判断項目 5） |
| 4.1・4.2 失敗で前の表示を保つ | 「失敗し得る GPU 呼び出しは控えの回収より前」の順（`show.rs:124-128`） | 使い回しの道でも同じ順を守る | Constraint |
| 4.3 同じ原因の失敗を積まない | emo-present に「続くあいだは 1 回」の仕組みは無い（grep で 0 件） | target ごとの小さな状態 | Missing |
| 5・6・7 実機の測り | 手順書 4 節の起動の塊・`tools/perf/check-quiet.ps1`・`invoke-cpu-sample.ps1`・impl-watch の机 | 面 1002 が出ていたことの確かめ方（`dump_surface` か記録）・SSP の面の確かめ | Unknown（実機） |
| 8.4 実 GPU で画素が同じ | `display_gpu_tests.rs`（実 GPU の檻） | 「一部だけ重ね直した絵＝いちから合成した絵」の比べ | Missing |

## 5. 実装の案

### 5.1 記録の出し分け（要件 1・2）

- **案 L-a（推奨の案 ⒜ と同じ）**: `apply_show` の終わりで「コマ送りだけか」を求め、そうなら `debug!`、違えば `info!` で、同じ文言・同じ欄を出す。触るのは `show.rs` だけ（＋兄弟のテスト・注記）。道具・見本・判定は変わらない。
  - 良い所: 差分が小さい・道具が無傷・失敗の道に触れない。
  - 気を付ける所: 2 つの呼び出しの欄の並びをそろえ続けること（§3.1）。`show.rs` の頭の注記（`info!` が契約）を書き直す。
- **案 L-b（まとめて 1 行）**: 一定の間のコマ送りを数えて 1 行に。`judge-perf.py` などの「1 適用 1 行」を壊し、まとめる間の長さという新しい決めごとが増える。時刻の扱い（時刻は正確に・メモリの「時刻は正確に扱う」）にも触れる。採る理由は見当たらない。
- **案 L-c（状態が変わったときだけ INFO）**: 結果は L-a と同じ。違いは書き方だけ。

### 5.2 コマ替えの重ね直し（要件 3）

**案 ⒜ emo-present の中だけ（控えと GPU の絵の持ち方）**
- 控えの容量を広げる: 240 通りなので記憶が合わない（§3.4）。
- GPU の絵を作り直さず `CopyFromMemory` で中身だけ書き替える: GPU の確保は減るが、合成は全部の層を毎回重ねるまま。**要件 3.1 を満たさない**。
- 触るファイル: `display.rs`・`cache.rs`・`show.rs`。`seriko-pattern-calls` との重なり 0。
- 評: 棚卸㉚の「emo-present の中に収める約束」は守れるが、件 25 の本体（変わらない層の作り直し）が残る。

**案 ⒝ 下から途中までを焼いた土台を控える（emo-compose に口）**
- 命令列のうち、最初に動く命令より下（ぱすたなら canvas・null・glow）を焼いた土台を target ごとに 1 枚控え、コマ替えでは土台を写してから残りの命令だけ重ねる。下から順に同じ命令を同じ順で重ねるので、画素は同じになる。
- 動く層より上の層（ぱすたなら着ているときの赤い ×）は毎回重ね直すことになる。上の層を先にまとめて焼いておく形（案 ⒝'）は、整数の丸め（`div255(v)=(v+127)/255`）のため `(A over B) over C` と `A over (B over C)` が 1 ずれうるので、**要件 3.2 の画素単位の一致と両立しない**。
- 記憶: target ごとに原寸 1 枚（ぱすたで約 1.5 MB）。上限は「target ごとに 1 枚」で決まる。
- 触るファイル: emo-compose の `lib.rs`（口）と `blit.rs`（途中から重ねる）、場合によって `plan.rs`（動く命令の見分け）。emo-present の `show.rs`・`budget.rs`・`cache.rs`。
- 気を付ける所: 「動く命令」をどう見分けるか。コマに由来する命令（動く絵・SERIKO のコマ）だけを動くとみなすと、`seriko-pattern-calls` の `move`（土台の層の位置をコマでずらす）が入った後に土台そのものが動く。

**案 ⒞ 前の絵を写して、変わった矩形だけ重ね直す（emo-compose に口）**
- 前に成立した適用の命令列と今の命令列を比べ、違う命令（前の位置と今の位置の両方）の外接矩形を求める。前の絵（いま表示に使っている控えの中身）を合成先へ写し、その矩形だけを消して、全部の命令を矩形で切って重ね直す。画素ごとに独立した計算なので、矩形の中はいちから合成したのと同じ・外は前と同じになる。
- ぱすたなら矩形は 140×140（面の約 5%）。上の層も矩形の中だけ重ね直すので、上下どちらの層があっても画素は同じ。
- 命令列の比べで見分けるので、「動く命令」の分類が要らない。`move` で土台がずれても、ずれた命令が違う命令として矩形に入る＝`seriko-pattern-calls` の後もそのまま正しい。
- 記憶: 合成先の席はもう在る（`FrameBudget`）。前の命令列の写し（数十件の `BlitOp`）を持つだけで、絵の追加は 0。
- 触るファイル: emo-compose の `lib.rs`（命令列を返す口か、矩形で切って重ねる口）と `blit.rs`（782 行・切る版を足すと 1,000 行に近づく＝兄弟の新しいファイルへ）。`plan.rs` の中身は変えずに済む見込み（`build_plan` の出力を使うだけ）。emo-present の `show.rs`・`budget.rs`・`cache.rs`。
- 気を付ける所: 要件 3.1 の「作り直さない」を、「変わった矩形の中で変わらない層も重ね直す」ことまで禁じると読むなら、この案は字面に合わない（判断項目 4）。控えの LRU との関係（emo2 のまばたきは 56% が当たり＝当たりを先に見て、外れたときだけこの道へ）。

**マスクと GPU への上げ（案 ⒝・⒞ のどちらにも付く）**
- 当たりのマスク: 今は全面の作り直しだけ。矩形だけ直すなら wintf の `AlphaMask` に口を足す（wintf に触る）。全面のままでも正しさは同じで、重さが残るだけ。
- GPU への上げ: 絵を作り直さず `ID2D1Bitmap1::CopyFromMemory` に矩形を渡せば、上げる量も矩形だけになる。ただし同じコマンドリストのままだと `set_display`（`mount.rs:235-248`）が「同じ」と見て差し込まず、wintf が描き直さない。`GraphicsCommandList` を `get_mut` で触って `Changed` を立てれば `mark_dirty_surfaces` が拾う（wintf を直さずに済む）。前提は「D2D のコマンドリストが絵を参照で持つ（中身を写さない）」こと（Research Needed ⑵）。
- 窓の面の描き直し（WUC の `BeginDraw`）は全面のまま（§3.3 の 5）。

**案 ⒟ 組み合わせ（段を分ける）**
- 1 段目: 記録の出し分け（L-a）＋案 ⒞ の合成だけ（マスクと GPU は全面のまま）。2 段目: 実機で内訳を測り、残りが大きければマスクの矩形・GPU の矩形を足す。
- 良い所: 測る前に wintf や D2D の振る舞いに賭けない。悪い所: 1 段目で CPU が SSP 並みに下がらなければ、2 段目の作業と測り直しがもう 1 回要る。

## 6. 議題ごとの材料

1. **記録の出し方（議題 1）**: §3.1 のとおり、`debug!` なら道具・見本・判定は変わらない（`invoke-perf-run.ps1:161`・`invoke-followup-checks.ps1:108` が `areka_emo_present=debug`）。`trace!` は道具から行を消すので選べない。⒝ は `judge-perf.py` などの 1 適用 1 行を壊す。`get_log` の履歴には元から載っていない（`log_history.rs:115-131`）。推奨の ⒜ を支える材料がそろっている。
2. **「コマ送りだけ」の範囲（議題 2）**: 材料は `apply_show` の中にそろう（§3.2）。SERIKO のコマ（emo2 のまばたき）も同じ `PatternState` の違いとして入ってくる（`pattern.rs:30-40`）ので、動く絵だけに限るほうがかえって見分けが要る（`films` だけ違うかを見る）。拡大率の替わりは `refresh_scale` の道（`refresh.rs:121`）で来て、今のテスト（`presenter_refresh_and_log_tests.rs:247-257`）がその行を読むので、INFO に残すと直しが少ない。残る穴は「隠した後の出し直し」（§3.2・判断項目 2）。
3. **CPU の合格の線（議題 3）**: emo2 の areka の release は 3% 未満が合格の線で、SSP の emo2 は 3.05%（§3.6）。ぱすたの差（およそ 17 ポイント）は適用の道とその後ろの描き直しにある。ただし内訳は測っておらず、wintf の全面の描き直しは本 spec では縮められない（§3.3 の 5）。設計の前か設計の中で、`areka_emo_present=debug` を足した走行で面 1002 の perf の行を取り、合成・マスク・上げ・測っていない区間の割合を見ると、「直す前より下がる」と「SSP の中央値を上回ったら仰ぐ」のどちらの線が現実的かが読める。
4. **emo-present の中に収めるか（議題 4）**: 合成は `compose_into` 1 回呼びで、命令列も途中から重ねる口も `pub(crate)`（§3.3）。emo-present だけでは要件 3.1 を満たせない（§5 案 ⒜）。案 ⒞ なら emo-compose で触るのは `lib.rs`（口）と `blit.rs`（矩形で切る版・新しいファイルへ）で、`plan.rs`・`pattern.rs` の中身は変えずに済む見込み。`seriko-pattern-calls`（今は要件の段・ブランチ `claude/seriko-pattern-calls-175601`）は `move` で `plan.rs` の `derive_ops` に触る見込み（同 spec の brief の 64 行目）。`move` は土台の層をコマでずらす＝「動く層」を種類で見分ける案 ⒝ は `move` の後に見分けを直す必要があり、命令列を比べる案 ⒞ は影響を受けない。`lib.rs` は 218 行で、口を 1 つ足す程度なら重なりは小さいが 0 ではないので、設計で触る行を挙げて向こうのセッションと合わせる。
5. **同じ面を出した SSP の基準（議題 5）**: ぱすたの面 1000（SSP の起動で出る面・`doc/pasta-real-run.md:183` の `\0\s[1000]`）にも同じ 3 枚のボールがある（`surfaces.txt:139-141`）。件 25 の SSP の 10% は、SSP がふつうに面 1000 で立っていたなら、同じボールが動いていた見込みが高い。ただし確かめていない。areka は空の記憶で `OnFirstBoot`＝面 1002 で測っており、面 1000 と面 1002 は変わらない層の中身が違う（1000 は影と立ち絵・1002 は glow と透明の null）。「同じ面」をそろえるなら、SSP を 1002 にするか、areka を 1000 にするかのどちらか。SSP の操作は毎回開発者の許しが要る（要件 7.3）。

## 7. 工数とリスク

- **記録の出し分けだけ**: S（1〜2 日）。リスク 低（1 か所・道具は無傷・失敗の道に触れない）。
- **重ね直しまで（案 ⒞ の合成だけ）**: M（3〜7 日）。リスク 中。emo-compose に口を足す・画素が同じことを実 GPU と決定論の檻で示す・控えの LRU と定常の確保 0 の檻を崩さない。
- **マスクと GPU の矩形まで**: L（1〜2 週）。リスク 中〜高。wintf の `AlphaMask` の口・D2D の絵の参照の振る舞い・`Changed` の立て方に頼る。
- **実機の測り（要件 5〜7）**: 静かな机と、SSP の基準なら SSP の机と許しが要る。debug・release の 2 版 × 6 分 × 前後。
- brief の見立て「10〜16」と合う。どこまでで止めるかは測りの結果しだい。

## 8. 設計へ持ち越す調べ物（Research Needed）

1. ぱすたの面 1002 で、1 回の適用の重さの内訳（`perf(apply_show)` の各段・`t_total_us` と段の和の差・wintf の描き直しのスレッドの CPU）。`invoke-perf-run.ps1` は emo2 を前提にしているので、ぱすたで回せるか（ゴーストの指定）を確かめる。
2. D2D のコマンドリストが絵を参照で持ち、閉じた後に `CopyFromMemory` で書き替えた中身が次の描き直しに出るか（Microsoft の `ID2D1CommandList` の説明と、実 GPU の檻で）。
3. `log-capture-kit` の捕捉で、`debug!` に下げた行が今と同じに数えられるか（`presenter_film_tests.rs` の `applies` など）。
4. 命令列の比べで、動く絵のコマの切り詰め（`trim.rs`・コマごとに置き場の矩形が違う）を矩形にどう入れるか。前の位置と今の位置の両方を入れる。
5. 面 1002 が出ていたことを実機の記録から確かめる方法（`dump_surface` か、詳しく出す水準の表示の行の `surface_id=1002`）。

## 9. 設計の判断項目（要件討議へ）

1. **水準は `debug!` に決め打ちするか**（`trace!` は道具を壊すので外す）。議題 1 の推奨 ⒜ の中身を「`debug!`」に絞る。
2. **隠した後の出し直しを INFO に残すか**。`Hide` は `last_show` を消さないので、要件 1 の定義のままだと「見えなかった窓が見えるようになった」適用は既定の水準で出ない。残すなら、見分けに `visible` の前の値を足す。
3. **「いちから合成したか・使い回したか」の記録の形（要件 3.5）**。perf の行は変更 0 の約束、表示の行は文言と欄の意味を変えない約束（要件 2.2）。3 つ目の状態（一部だけ重ね直した）を、表示の行の末尾に欄を足して出すか、別の `debug!` の行にするか。そのとき `cache_hit` の意味（一部だけのとき false か）をどうするか。
4. **要件 3.1 の「作り直さない」の読み**。案 ⒞（変わった矩形の中だけ、変わらない層も含めて重ね直す）を満たすとみなすか。みなさないなら案 ⒝ になり、上の層は毎回重ね直す（先にまとめると画素がずれる）。
5. **マスクと GPU への上げも矩形にするか**。するなら wintf の `AlphaMask` に口を足す（wintf に触る）・D2D の絵の使い回しに頼る。段を分けて、測ってから決める形（案 ⒟）もある。
6. **emo-compose に触る範囲（議題 4）**。案 ⒞ なら `lib.rs`（口）と `blit.rs`（新しいファイルへ）で、`plan.rs`・`pattern.rs` の中身は変えない見込み。`seriko-pattern-calls` のセッションと触る行を先に合わせる。
7. **要件 3.3 の見本**。面 1002 の 3 枚は隣り合っていて、あいだに変わらない層は無い。挟む形は作った見本で確かめ、要件の括弧書きを直すか。
8. **CPU の合格の線（議題 3）と、届かなかったときの扱い**。wintf の全面の描き直しは本 spec では縮められないので、合成を減らしても SSP の中央値に届かない見込みがある。そのときに範囲を広げる（wintf）のか、差と理由を書いて仰ぐのか。
9. **SSP の基準をそろえる面（議題 5）**。SSP を面 1002 にするか、areka を面 1000 で測るか。面 1000 にも同じボールがあり、件 25 の SSP の 10% は面 1000 だった見込みが高い（未確認）。
10. **emo2 のまばたきの道を崩さない順**。控えの当たりを先に見て、外れたときだけ新しい道へ入る。定常の確保 0（`presenter_budget_steady_state_tests.rs`）と容量 3 の LRU（`presenter_cache_capacity_tests.rs`）の檻をそのまま通す。
11. **同じ原因の失敗を積まない仕組み（要件 4.3）**。emo-present に前例が無いので、target ごとに「最後に出した失敗の原因」を持つ小さな状態を新しく足す。
