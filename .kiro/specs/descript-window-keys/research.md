# ギャップ分析: descript-window-keys

> 2026-10-23・main `94e08e0a`（ワークツリー `claude/kiro-start-descript-window-keys-451069`）で測った。対象は確定済みの `requirements.md`。決めは置かず、案と問いだけを置く。

## 1. 要旨

- **既定の面の決め打ちは 3 か所**（`emo2_boot/assets.rs` 36・344 行目、`placement/measure.rs` 43・173〜205 行目、手本 `examples/window-placement.rs` 123・296〜308 行目）。起動・ゴーストの切り替え・ゴーストの読み直しはどれも `placement::prepare_ghost_windows_for_shell` → `measure_scope_sizes` を通るので、測りの直しは 1 経路で済む。一方 **シェルの切り替えは測り直さない**（窓の大きさは置き換えた面の表示の報告で決まる）。
- **シェルの切り替えには戻り先が無い**ことを確かめた。seriko の `rebased()` が今の面をそのまま載せ、`apply_replace` → `apply_show` が新しいシェルに無い面で `error!` を残して、窓は表示なしのまま次の `\s` まで残る。ゴーストの読み直しの戻り先（`shell_carry::decide`）は 1 段だけで、要件 3.3 の 2 段（descript → 組み込み → 出さない）は無い。
- **`\![lock,balloonmove]`／`\![unlock,balloonmove]` は今、何もしない**。実行時は受け手ごとの名前の選別で `debug!`（`MoveCueSink: 担当外コマンド名を良性スキップ` ほか）が出るだけで、既定の記録の段では何も残らない。MCP の `check_script` は `unknown_command`（"no part of areka handles this \! command; playback ignores it"）を返す。受け手の表 `consumer_ledger.rs` に行は無い。
- **バルーンのドラッグは wintf が wndproc で窓を動かす**（`DragConfig::default()`＝`move_window: true`・`placement/spawn.rs` 455 行目）。`on_balloon_drag` は動いた後に呼ばれるので、ハンドラの中で止めることはできない。止めるにはドラッグが始まる前にバルーンの窓の `DragConfig` を書き換える必要がある。wintf は `enabled` を押した時と閾値を越えた時に、`move_window` をドラッグの始まり（`drag/dispatch.rs` 136 行目）に読んで控えるので、「次のドラッグから」（要件 5.6）は wintf の今の作りのまま成り立つ。
- **同じウェーブとの重なりが 1 つ出た**。台本の lock を UI スレッドへ運ぶ自然な道（`MoveCueSink` の `PositionCue` → `frame/drain_resnap.rs` の `run_move_drain_phase`）も、新しい相を足す道（`emo2_boot/frame.rs` の `emo2_frame_system`）も、C11 の約束で ⑥ `dpi-realign-remembered-chain` の持ち物になっている。設計の前に向こうのセッションと相談する（下の議題 4）。

## 2. 今の作り（調べた所）

### 2.1 最初の面の決め打ち

| 所 | 今の値 | 使われ方 |
|---|---|---|
| `crates/areka/src/emo2_boot/assets.rs` 36 行目 `KERO_INITIAL_SURFACE_ID`・340〜345 行目 | スコープ 0 は 0・それ以外は 10 | `ScopeAssets.initial_surface_id` → `frame/attach.rs` 115 行目 `PlannedAttach.initial_surface_id` → 285 行目 `shell_carry::decide` の戻り先（ゴーストの読み直しだけ）。起動では窓に出さない（defect #5 の注記・`frame/attach.rs` 302〜311 行目） |
| `crates/areka/src/placement/measure.rs` 43 行目・173〜205 行目 | 面 0（スコープ 0・失敗は `PlacementError::Measure`）／面 10（スコープ 1 以降・失敗はスコープ 0 の大きさで代える） | 起動の位置の窓の大きさ。ぱすたの件 4 の原因 |
| `crates/areka/examples/window-placement.rs` 123 行目・296〜308 行目 | 同じ | 手本。emo2 の検体だけを使う |
| `crates/areka/src/placement/measure_template_tests.rs` 35 行目 | 注記で定数を名指し | テスト |
| `crates/areka/src/emo2_boot/assets_tests.rs` 48・53 行目、`frame_attach_tests.rs` 39・50 行目 | 0・10 を固定 | 決め打ちの檻（descript を読まない検体なので値は変わらない見込み） |

- `measure.rs` の 196 行目の `warn!`「scope n≥2 の初期 surface は正典既定なし（id10 暫定で採寸）」は古い。ukadoc は `char*.seriko.defaultsurface` の既定を 10 と書く（https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#char*.seriko.defaultsurface%2C%E6%95%B0%E5%80%A4 「\p[*]のゴーストのデフォルトのサーフェス番号。」・既定 10）。ただし今は `detect_scopes` がスコープ 2 以降を立てないので、この行は実際には出ない。
- 測りは着せ替えを付けずに合成する（`measure.rs` 410〜426 行目 `compose_size` の `BindSet::default()`）。表示は既定の着せ替えを載せる。ぱすたの面 1000 は着せ替えの面なので、着せ替えの部品が素の面の外へはみ出すと、測った大きさと出た大きさが食い違う（下の「調べること」R2）。

### 2.2 descript の読み取り

- `crates/areka/src/placement/config.rs`（689 行・本体は 1〜305 行・残りは同じファイルの中のテスト）の `build_placement_config(ghost_kv, shell_kv)` が配置の値を組む。スコープの綴りは `scope_prefixes`（0＝`sakura.`＋別名 `char0.`・1＝`kero.`＋別名 `char1.`・n≥2＝`char{n}.`）、1 層から引くのは `scoped_value`（正典＞別名）、ゴースト＜シェルの 2 層は `cascade2`。
  - 要件 1.3（別名を同じ優先で）は `scoped_value(ghost_kv, scope, &["seriko.defaultsurface"])` をそのまま使える。要件 1.6（ゴーストだけから読む）は `cascade2` を使わないことで守れる。
  - `PlacementConfig.scopes` は `detect_scopes` が立てたスコープ（0 と、相方が居れば 1）だけを持つ。要件 1.4（スコープ 2 以降も読むだけ）は、`scopes` の外でゴーストの descript の `char<n>.seriko.defaultsurface`（n≥2）の鍵を拾う必要がある。
- 読みの源 `DescriptSource`（`placement/source.rs` 97 行目）は `ghost_kv` と `shell_kv` を持つ。起動・切り替え・読み直しのどれでもこの値が手元に在る:
  - 起動・ゴーストの切り替え・ゴーストの読み直し: `ghost_session::prepare_ghost_windows`（`ghost_session.rs` 205 行目）→ `placement::prepare_ghost_windows_for_shell` → `prepare_stages_for_shell`（`placement/mod.rs` 419〜440 行目・`build_placement_config` の直後に `measure_scope_sizes`）。
  - シェルの切り替え・シェルの読み直し: `switch_assets.rs` 207〜241 行目の `build_shell` が `load_descript_source_for_shell` を読んでから `build_shell_assets` を呼ぶ。後始末 `frame/switch.rs` 433 行目の `finish_shell` も `source.ghost_kv`・`source.shell_kv` で `build_placement_config` を呼ぶ。
- 起動の資産 `build_boot_assets_with_shell`（`emo2_boot/mod.rs` 441 行目から `build_boot_assets_for` 経由）は descript を受け取らない。`build_shell_assets` は `resolve_with_shell` の `MountModel` を読むが、ゴーストの descript の生の値は持たない。準備が「1 度だけ読んだ値」を起動の結線へ運ぶ器は `ghost_session::StartupDescriptValues`（`ghost_session.rs` 99 行目・`author_dpi` と `zorder_raw`）で、組む所は本番 1 か所とテストの手書き 8 か所（`ghost_switch_test_support.rs`・`ghost_session_reload_tests.rs`×2・`ghost_session_restart_tests.rs`×2・`ghost_session_strict_tests.rs`・`main_session_mark_tests.rs`・`main_session_mark_write_tests.rs`）。

### 2.3 シェルの切り替えでの窓の大きさと面

1. 受理 → 背景のスレッドで `build_shell`（`switch_assets.rs`）が新しいシェルの `ShellAssets` を組む（`initial_surface_id` は 0/10 の決め打ちで入る）。
2. `frame/switch.rs` の `split`（609〜640 行目）が `ScopeAssets` を `(scope, emo_world, atlas, author_dpi)` に崩す。**`initial_surface_id` はここで捨てられる**。
3. seriko が定義を差し替え、`actor.rs` 364〜392 行目の `rebased()` が「各スコープの今の面（無ければ `None`）」を `RebasedShow` に載せる。
4. `emo2_boot/adapter.rs` 180〜224 行目の `send_rebased` が荷物と突き合わせ、`PresentCommand::ReplaceTarget { show }` を送る。ここでは新しいシェルの `emo_world` が手元に在る（`emo_world.surface(id)` で在るかを引ける）。
5. `areka-emo-present/src/presenter/replace.rs` の `apply_replace` が古い装着を消して新しく登録し、`show` を `apply_show` で出す。面が無ければ `apply_show` が `error!` を残し、**その窓は表示なしのまま**（同ファイル 72〜75 行目の注記）。
6. `finish_shell` が `reconcile_reported_sizes`（表示が積んだ窓寸の報告を窓へ反映）→ `apply_shell_descript`・`reanchor_char_windows` → `replace_sticky_descript` ほかを行う。**測りは呼ばない**。窓の大きさは出した面の大きさで決まる。

- 要件 2.1 の「測るすべての時機」は、今の作りでは起動・ゴーストの切り替え・ゴーストの読み直しの 3 つ（どれも `prepare_stages_for_shell`）と手本だけ。`\![execute,resetwindowpos]` の並べ直し（`placement/reset_pos.rs` 78〜101 行目）は今の窓の大きさを読むので測らない。
- 要件 3.2・3.3 の戻り先は、手順 4 の `send_rebased` か手順 3 の `rebased()` のどちらかに入れる場所がある（議題 1）。

### 2.4 ゴーストの読み直しの戻り先

- `emo2_boot/shell_carry.rs` 103〜115 行目 `decide(carried, initial, exists)`: 在れば持ち越し、無ければ `initial`、それも無ければ出さない。`initial` は `PlannedAttach.initial_surface_id`（＝`assets.rs` の決め打ち）。
- 要件 3.1 は `initial_surface_id` を descript の値にすれば満たせる。要件 3.3 の 2 段目（組み込みの既定の面）は `decide` に引数を 1 つ足す形になる（`CarriedShow` に「2 段目へ落ちた」の腕を足すかは設計で決める）。
- 控えた面は `replay` で seriko へ `\s` と同じ合図で送り直すので、seriko の「今の面」と出ている面は食い違わない。

### 2.5 バルーンのドラッグの道

- 窓を作る所: `placement/spawn.rs` 434〜466 行目。バルーンの窓は `DragConfig::default()`・`OnDrag(on_balloon_drag)`・`OnDragEnd(on_balloon_drag_end)` を持ち、`OnDragStart` は持たない（`input_events/mod.rs` 363 行目「バルーン窓には何も付けない」）。
- wintf（`crates/wintf/src/ecs/drag/`）:
  - `DragConfig` は `enabled`・`move_window`・`threshold`・ボタンを持つ（`drag/mod.rs` 39〜67 行目）。
  - `enabled` は押した時（`window_proc/mouse_click.rs` 153 行目）と閾値の判定（`mouse_move.rs` 226 行目）で読まれ、ドラッグ中（`Dragging` の腕・258〜312 行目）は読まれない。
  - `move_window` はドラッグの始まりの配信で読まれ（`drag/dispatch.rs` 136 行目）、`WindowDragContextResource` に控えられる。ドラッグ中の窓の移動（`mouse_move.rs` 301 行目）はこの控えを使う。`WindowDragContextResource::get()` は公開されている（`drag/context.rs` 68 行目・`drag/mod.rs` 19 行目で再公開）。
- areka のハンドラ（`placement/follow/drag_follow.rs`・958 行）:
  - `on_balloon_drag`（511 行目）: 動いた窓の位置から追従のずれ `BalloonFollow.offset` を引き直し、`reestablish` で基準の表示 DPI を焼き直す。
  - `on_balloon_drag_end`（652 行目）: 最終位置からずれを引き直し、**記憶へ書く**（750〜751 行目 `balloon_offset_entries` → `persist_entries`）→ キーワードの素材を退役（755 行目）→ 解放の時の画面内への補正（760 行目）。
- キャラクターの窓には先例がある: `placement/follow/sticky_follow.rs` が「`DragConfig.move_window` を書くのは 1 か所」の決まりで付け直し（58〜72 行目）、ドラッグの始めの値を `CharDragSession` に控えて（`drag_wintf_moves`・203〜210 行目）ハンドラがそれを読む。

### 2.6 台本の lock・unlock の今

- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` 1739・3713 行目の注記どおり、キャリアに載って最後まで運ばれ、受け口ごとに `debug!` で担当外として捨てられる（例: `emo2_boot/move_cue.rs` 289〜294 行目）。`compile` の catch-all は通らない。
- `check_script`: `mcp/check_script_judge.rs` 91〜94 行目が `ConsumerLedger::consumer_of("lock", Some("balloonmove"))` を引き、`None` なので `unknown_command`。**受け手の表に 2 行を足せば、`check_script_judge.rs` を触らずに要件 5.7 を満たす**（判定は表の答えを写すだけ）。表の檻は `consumer_ledger_tests.rs` 294〜342 行目の手書きの数 36 と、`consumer_ledger_agreement_tests.rs` の見本。
- 選別子つき（`("lock", Some("balloonmove"))`・`("unlock", Some("balloonmove"))`）で登記すれば、`\![lock,repaint]` などほかの第 1 引数は担当なしのまま空く（`repaint` も今は誰も受けていない）。

### 2.7 寿命の先例（窓の組の帳簿）

- `placement/sticky_group_ledger.rs`: 起動で `seed_sticky_descript`（`ghost_session.rs` 280 行目・窓を作る閉包の中）、シェルの切り替えで `replace_sticky_descript`（`frame/switch.rs` 468 行目・台本の組は保ち descript の分だけ取り替える）、窓を閉じて起こし直すとき `app_exit.rs` 201 行目 `close_windows_for_restart` が資源ごと外す（ゴーストの終了・切り替え・読み直し）。SHIORI だけの読み直しはここを通らず残る（`app_exit.rs` 181〜182 行目）。
- 「バルーンの動かせない指定」（descript の段＋台本の lock）は、この帳簿と同じ 3 つの時機でそのまま持てる。

### 2.8 網羅台帳と対応表

- `doc/ukadoc-coverage/ledger/assets.toml`: ゴーストの `sakura.seriko.defaultsurface`（3425）・`balloon.dontmove`（2550）は `owner = "descript-window-keys"`。`kero.seriko.defaultsurface`（3043）・`char*.seriko.defaultsurface`（2697）・シェルの `sakura.`（5486）・`kero.`（4639）・`char*.balloon.dontmove`（4205）は持ち主が空。どれも `status = "absent"`。
- ゴーストの `balloon.dontmove` の行の注記は「areka のバルーンは常に本体の窓に付いて動く……切ると書いても付いて動く」と、追従を止める読みで書いてある。lock・unlock の行も「バルーンの追随を止められず」と書く（要件 8.3 で直す相手）。
- `doc/COMPAT_ARCHITECTURE.md` §8（122 行目から）には、この spec のキーの行はまだ無い（162 行目はバルーンの `*.balloon.defaultsurface` の行＝範囲外）。

## 3. 要件と今の作りの対応

| 要件 | 使える物 | 抜け | 印 |
|---|---|---|---|
| 1.1〜1.3 既定の面の読み | `config.rs` の `scoped_value`・`scope_prefixes` | 読む関数と値の型（スコープ → 面の番号＋読めなかった記録） | Missing |
| 1.4 スコープ 2 以降 | `scope_prefixes` の `char{n}.` | `detect_scopes` の外で鍵を拾う所 | Missing |
| 1.5 読めない値 | `parse_i32` の `warn!` の型 | 0 以上の整数の読み（`u32`） | Missing |
| 1.6・1.7 ゴーストだけ・切り替えで不変 | `DescriptSource.ghost_kv` | 切り替えでも同じ値を引く道（`build_shell` は `source` を持つ） | Constraint |
| 2.1・2.5・2.6 測り | `measure_scope_sizes`・`compose_size` | 面の番号を引数で受ける形（今は定数） | Missing |
| 2.2 測りと戻り先で同じ値 | なし（今は 3 か所に同じ定数） | 1 か所で決めて両方へ配る道 | Missing |
| 2.3・2.4 無いときの段 | `measure.rs` の代替（スコープ 0 は失敗・それ以外はスコープ 0 の大きさ） | descript → 組み込みの 1 段と記録 | Missing |
| 3.1 読み直しの戻り先 | `shell_carry::decide`・`PlannedAttach.initial_surface_id` | 値を descript にする | Missing |
| 3.2 切り替えの戻り先 | `send_rebased` の手元の `emo_world` | 戻り先そのもの（今は表示なしで残る） | Missing |
| 3.3 2 段の戻り先 | `decide` の 1 段 | 2 段目と記録 | Missing |
| 3.5 最初から出す | `ReplaceTarget.show`・`PlannedAttach` | 置き換えの命令の中で決める（後から差し替えない） | Constraint |
| 4.1〜4.5 descript の段 | `config.rs` の `scoped_value`・`sticky_group_ledger` の段の持ち方 | 値の読み・段の優先・帳簿 | Missing |
| 5.1〜5.3 lock・unlock | `MoveCueSink` の選別の形・受け手の表 | 受け手・UI への運び（議題 4） | Missing／Constraint |
| 5.4・5.5 寿命 | `sticky_group_ledger` の 3 つの時機 | 帳簿 | Missing |
| 5.6 ドラッグの途中 | wintf の `move_window` の控え・`enabled` の読み方 | ハンドラの側の「このドラッグは止めたか」の控え | Constraint |
| 5.7 `check_script` | `ConsumerLedger::consumer_of` | 表の 2 行 | Missing |
| 5.8・6.6 記録 | `tracing` の `event =` の型 | 語彙 | Missing |
| 6.1 位置と記憶を変えない | `on_balloon_drag(_end)` | 入口の見張り（`move_window: false` の案なら） | Missing |
| 6.2〜6.5 ほかの操作は今どおり | 追従・`\![move]`・バルーン位置初期化は `DragConfig` を見ない | 確かめの檻 | Unknown（R4） |
| 7.x 決定論のテスト | `config.rs` の内側のテスト・`shell_carry` の純関数・`follow_test_support.rs` | 新しい檻 | Missing |
| 8.x 台帳と §8 | 2.8 の行 | 書き換え | Missing |
| 9.x ぱすた | `doc/pasta-real-run.md` の段 1 | 着地の後の実走 | Unknown（R1・R2） |

## 4. 案

### 4.1 既定の面

- **案 A（今の部品を延ばす）**: `config.rs` に読みを足し、`PlacementConfig` に「スコープ → descript の既定の面」を持たせる。`measure_scope_sizes` は面の番号を引数で受ける。起動の資産へは `StartupDescriptValues` に欄を足して `wire_emo2_boot` → `build_boot_assets_for` → `build_shell_assets` へ運ぶ。切り替えは `build_shell` が手元の `source.ghost_kv` から同じ関数で引く。
  - ✅ 「1 度だけ読んだ値を配る」今の型に合う。❌ `config.rs` の本体が太る（ファイル全体はテスト込みで 689 行）・`StartupDescriptValues` を組むテスト 8 か所・`wire_emo2_boot` の引数・`placement/mod.rs`（939 行・`measure_scope_sizes` の呼び 1 行）・`ghost_session.rs`（917 行）・`emo2_boot/mod.rs`（789 行）に手が入る。どれも brief の「触るファイル」に無い。
- **案 B（新しいファイル）**: 読みと戻り先の決め方を新しい `placement/default_surface.rs`（名前は仮）に置く。中身は純関数 2 つ＝「ゴーストの KV → スコープごとの既定の面（記録つき）」と「descript の面 → 組み込みの面 → 無し、を在るかの問い合わせで決める」。`measure.rs`・`assets.rs`・`shell_carry.rs`・切り替えの戻り先・手本がこれを呼ぶ。値の運びは案 A と同じ（`PlacementConfig` か `StartupDescriptValues`）。
  - ✅ 3 か所の定数を 1 か所にまとめられる（brief の棚卸㉙の「見つけた穴」）。`config.rs` を太らせない。決定論のテストを 1 ファイルに集められる。❌ ファイルが 1 つ増える。
- **案 C（資産の側で読み直す）**: `build_shell_assets` が自分でゴーストの descript を読む。
  - ✅ 運ぶ欄が要らない。❌ 同じ descript を 2 回読む（`StartupDescriptValues` の注記が戒める「別々に読み直す余地」）。測りと資産が違う読みを見る日が来うる。

### 4.2 シェルの切り替えの戻り先（要件 3.2・3.3・3.5）

- **案 1（`send_rebased` で決める）**: 荷物 `SwapPayload.targets` にスコープの descript の既定の面を足し、`send_rebased` が手元の `emo_world` で在るかを引いて `show` を決める（`shell_carry::decide` と同じ関数）。
  - ✅ 置き換えの命令の中で決まる＝1 フレーム遅れない。❌ seriko の「今の面」は無かった面のままになり、出ている面と食い違う（次の `Rebased`・面を引く問い合わせ・`\s` の別名の解決に効くかを設計で確かめる）。食い違いを消すには seriko へ「今の面はこれ」を返す合図が別に要る。
- **案 2（seriko の `rebased()` で決める）**: `SerikoReplace::Shell` に新しいシェルの面の番号の集合とスコープの既定の面を載せ、seriko が自分の状態を直してから `RebasedShow` を出す。
  - ✅ seriko と表示が食い違わない。❌ `areka-seriko`（`actor.rs` 768 行）の公開の型が変わる。面の番号の集合は `EmoWorld::surface_ids()` から背景のスレッドで作れる（`assets.rs` 339 行目で同じことをしている）。
- **案 3（読み直しと同じく置き換えの後に送り直す）**: `shell_carry::replay` と同じく、置き換えの後で seriko へ `\s` を送る。
  - ❌ 置き換えで表示なし → 次のフレーム以降に出る、となり要件 3.5 に反する。

### 4.3 ドラッグを止める仕組み（要件 4・5・6）

- **仕組み X（`enabled: false`）**: 動かせないスコープのバルーンの窓の `DragConfig.enabled` を落とす。
  - ✅ ハンドラに手が入らない。記憶も追従のずれも構造的に変わらない（6.1）。❌ ドラッグの合図そのものが来ないので、要件 6.6（1 回のドラッグにつき 1 件の記録）を出す所が無い。押して動かした後に放すと、ドラッグと見なされずにクリックとして届く場合がある（選択肢の上で起きると今と振る舞いが変わる・R4）。
- **仕組み Y（`move_window: false`＋入口の見張り）**: `move_window` を落とし、`on_balloon_drag`・`on_balloon_drag_end` の入口で「このドラッグは wintf が窓を動かしていない」を見て、ずれの引き直し・記憶の書き込み・素材の退役・解放の補正を飛ばし、`on_balloon_drag_end` で 1 件記録する。「このドラッグ」の判定は wintf の控え `WindowDragContextResource::get().move_window`（ドラッグの始まりの値）か、`OnDragStart` で areka が控える値で読む。
  - ✅ 6.6 の記録の時機がある。ドラッグの扱いは今と同じなのでクリックの振る舞いが変わらない。5.6（途中の lock は次のドラッグから）は、控えた値を読めば成り立つ。❌ `drag_follow.rs`（958 行）の入口に数行ずつ。見張りを忘れると、窓が動いていないのに `reestablish` が基準の DPI を焼き直す・記憶を同じ値で書く。
- **書き手**: どちらの仕組みでも、`sticky_follow.rs` の先例どおり「バルーンの窓の `DragConfig` を書くのは 1 か所」の関数を新しいファイルに置き、descript の段が決まった時（起動の窓を作る閉包・シェルの切り替えの後始末）と台本の lock・unlock を受けた時に呼ぶ。

### 4.4 台本の lock・unlock の運び（議題 4 と対）

- **道 P（`MoveCueSink` に足す）**: `PositionCue` に 2 つの腕を足し、`frame/drain_resnap.rs` の `run_move_drain_phase` に 2 つの腕を足す。台本の並び（`\![move]` と同じ線）が保たれる。❌ `drain_resnap.rs` は C11 の約束で ⑥ の持ち物。
- **道 Q（新しい受け手と線）**: `ZOrderCueSink` と同じ形の受け手と線を足し、新しい相を `emo2_frame_system` に 1 行足す。❌ `emo2_boot/frame.rs` も ⑥ の持ち物。
- **道 R（受け手が共有の印を書き、UI が既にある相で読む）**: 受け手が `Arc` の印を書き、UI スレッドの今ある相（⑥ の持ち物でない所）で `DragConfig` を付け直す。❌ 置き場が意味の上で無理になる（たとえばシェルの切り替えの相の中）。設計の筋が悪い。

## 5. 同じウェーブ（C11）との重なり

`roadmap.md`「ウェーブ編成（2026-10-23 棚卸㉙）」の C11 の行の約束: ① この spec は `placement/{spawn,persist,chain_realign,chain_finalize}.rs`・`emo2_boot/frame.rs`・`emo2_boot/frame/{dpi,drain_resnap}.rs` に触らない（⑥ の持ち物）・`placement/follow/window_move.rs` に触らない。⑥ は `placement/{config,measure}.rs`・`placement/follow/drag_follow.rs` に触らない。

| この spec が触りそうな所 | C11 の持ち主 | 判定 |
|---|---|---|
| `placement/config.rs`・`measure.rs`・`follow/drag_follow.rs` | ①（この spec） | 重なり 0 |
| `emo2_boot/assets.rs`・`switch_assets.rs` | ① | 重なり 0（④ の新しい `emo2_boot` のファイルとは別） |
| `emo2_boot/consumer_ledger.rs`＋兄弟のテスト | ①（⑦ は案 1 なら触らない約束） | 重なり 0 |
| `examples/window-placement.rs` | ① | 重なり 0 |
| **`emo2_boot/frame/drain_resnap.rs`（道 P）・`emo2_boot/frame.rs`（道 Q）** | **⑥ `dpi-realign-remembered-chain`** | **約束に反する**＝議題 4 |
| `placement/spawn.rs`（バルーンの `DragConfig` の初期値を窓を作る所で決めるなら） | ⑥ | 約束に反する＝作った後に書き手の関数で書けば避けられる |
| wintf `drag/dispatch.rs`・`window_proc/mouse_move.rs` | ⑥（件 9） | 4.3 の仕組み X・Y はどちらも wintf を触らない＝重なり 0 |
| `ghost_session.rs`・`emo2_boot/mod.rs`・`placement/mod.rs`（値の運び・案 A/B） | C11 の表に持ち主なし（④ は `emo2_boot/mod.rs` に触らない約束） | brief の「触るファイル」に無い＝設計で挙げる |
| `emo2_boot/frame/switch.rs`（`split`・`finish_shell`）・`emo2_boot/adapter.rs`（案 1）・`emo2_boot/shell_carry.rs` | C11 の表に持ち主なし | brief の「触るファイル」に無い＝設計で挙げる |
| `app_exit.rs`（帳簿を外す 1 行） | C11 の表に持ち主なし | 同上 |
| `areka-seriko/src/actor.rs`・`output.rs`（案 2） | C11 の表に持ち主なし | 同上（公開の型が変わる） |
| 網羅台帳 `assets.toml`・`sakura-script.toml` の行・§8 の追記 | 許す重なり | 重なり 0 |

- 後の版・同じ v0.1.0 の後続（同じウェーブに居ない）: `placement-measure-bake-once`（`measure.rs`・案 A なら `assets.rs`）・`window-work-area-clamp`（件 2 はこの spec の件 4 と同じ記録・持ち分の線は brief のとおり）・`extra-character-windows`（`switch_assets.rs`・スコープ 2 以降の既定の面を受け取る）・`balloon-canon-residue`（`config.rs`）・`window-snap`（`drag_follow.rs`）・`shell-companion-balloon`（`frame/switch.rs`）。

## 6. 規模と危うさ

- **規模: M**（brief の見立て 6〜9 タスクのまま）。既定の面＝読み・測り・起動の資産と読み直しの戻り先・切り替えの戻り先で 3〜4、動かさせない＝帳簿と書き手・ハンドラの入口・lock の受け手と運びで 3〜4、台帳と記録で 1。値を運ぶ道（`StartupDescriptValues` とテスト 8 か所）と切り替えの戻り先（案 1/2）で上へ振れうる。
- **危うさ: 中**。どれも今ある型（帳簿・書き手 1 か所・受け手の表・純関数の決め方）の延長だが、⑴ 台本の lock の運びが同じウェーブの ⑥ の持ち物にかかる ⑵ 切り替えの戻り先で seriko と表示の状態をそろえる所 ⑶ `drag_follow.rs` が 958 行で上限に近い、の 3 つが残る。

## 7. 調べること（設計の段へ）

- **R1（SSP で測る・開発者の機械）**: 要件の段の裁定 1・2・4（`brief.md` の議題 1・2）。⑴ ぱすたのバルーンだけを利用者がドラッグして動かないか ⑵ descript の既定の面がシェルに無いとき SSP は何を出すか ⑶ `\0` で lock した後に `\1` のバルーンが動くか。測る前に毎回開発者に許しを取る。
- **R2**: ぱすたの面 1000 を着せ替えなしで合成した大きさが、表示の 554×684（SSP の rect の 1/1.5 倍）と一致するか。違えば測りに既定の着せ替えを載せるかが議題になる（`compose_size` は `BindSet::default()`）。
- **R3**: 案 1 を採るとき、seriko の「今の面」が無かった面のまま残ることが、どこへ効くか（次の `Rebased`・プロパティ・`\s` の相対の解決）。
- **R4**: 仕組み X で、押して閾値を越えて動かした後に放したとき、バルーンの選択肢・リンクのクリックとして届くか（今はドラッグとして扱われ、クリックにならない）。仕組み Y で `WindowDragging` の印がバルーンの窓に付いたままになることが、ほかの相（重なり順・追従）に効かないか。
- **R5**: 測りで「在る」とみなす基準（合成が成り立ち 0 寸でない）と、戻り先で「在る」とみなす基準（`EmoWorld::surface(id).is_some()`）が違う。共通の決め方を 1 つにするとき、どちらに合わせるか。
- **R6**: ukadoc の `char*.balloon.dontmove` は「2人目以降の吹き出しのユーザーによる移動操作を禁止する。」（https://ssp.shillest.net/ukadoc/manual/descript_shell.html#char*.balloon.dontmove_ ）と書く。`char*.seriko.defaultsurface` は「\p[*]の」と書き、`config.rs` は `char0.`・`char1.` を本体と相方の別名に読む。兄弟の項目の型で読むと「`char<n>` は `\p[n]`」で、`char1.balloon.dontmove` は相方の別名になる。字面の「2人目以降」は相方を含む読みにもなる。

## 8. 設計の段への推奨（決めではない）

- 既定の面は案 B（新しいファイルに純関数 2 つ・値は準備が 1 度読んで配る）が、3 か所の定数をまとめる brief の指摘と、`config.rs` を太らせない約束の両方に合う。
- ドラッグは仕組み Y（`move_window: false`＋入口の見張り・wintf の控えを読む）が要件 5.6・6.1・6.6 をそろって満たす。書き手は `sticky_follow.rs` の型で 1 か所。帳簿は `sticky_group_ledger` と同じ 3 つの時機で持つ。
- 台本の lock の運びは、設計の前に ⑥ のセッションと相談する（開発者の方針「他のセッションの持ち場の事実で詰まったら、開発者に案を出す前にそのセッションへ相談」）。

## 9. 要件の討議へ渡す議題

1. **シェルの切り替えの戻り先をどこで決めるか**: 案 1（`send_rebased`・seriko と表示が食い違う）／案 2（seriko の `rebased()`・`areka-seriko` の公開の型が変わる）。案 3 は要件 3.5 に反する。
2. **descript の既定の面を起動の資産へどう運ぶか**: 準備が読んだ値を `StartupDescriptValues` で運ぶ（テスト 8 か所・`wire_emo2_boot` の引数・`ghost_session.rs`・`emo2_boot/mod.rs`・`placement/mod.rs` に手が入る）か、資産の側で読み直すか。brief の「触るファイル」に無いファイルが増える。
3. **ドラッグを止める仕組み**: 仕組み X（`enabled: false`・要件 6.6 の記録の時機が無い・クリックの振る舞いが変わりうる）／仕組み Y（`move_window: false`＋`drag_follow.rs` の入口の見張り）。
4. **台本の lock・unlock を UI へ運ぶ道が C11 の約束（⑥ の持ち物）にかかる**: 道 P（`drain_resnap.rs` に 2 つの腕）・道 Q（`frame.rs` に 1 行）のどちらも ⑥ `dpi-realign-remembered-chain` の持ち物。⑥ のセッションへ相談して、触ってよい範囲を決めるか、⑥ の着地の後に回すか。
5. **シェルの `*.balloon.dontmove` の別名**: `char0.`・`char1.` を本体と相方の別名として読むか（要件 1.3 は既定の面にだけ別名を書く）。ukadoc の「2人目以降」の読み（R6）。
6. **SHIORI だけの読み直し（`\![reload,shiori]`）で台本の lock を保つか**: 窓の組の先例（完了 `areka-P0-window-link-directives` 要件 6.3）は保つ。要件 5.5 の「読み直される」がゴーストの読み直しだけを指すかを書き足す。
7. **測りに既定の着せ替えを載せるか**: R2 の結果しだい。載せないままだと、着せ替えの部品が素の面からはみ出すゴーストで、窓の大きさが表示とずれる。
8. **`measure.rs` の「scope n≥2 は正典既定なし」の `warn!`**: ukadoc は既定 10 と書くので、この記録の文言を直すか外す（今は出ない行）。
9. **手本 `examples/window-placement.rs` の定数**: 新しい決め方の関数を呼ぶ形にそろえるか、emo2 の検体だけの手本として決め打ちのまま残すか。
10. **測りと戻り先の「在る」の基準をそろえるか**（R5）。

## 10. 要件の討議での追記（2026-10-23）

### 10.1 SSP での測り（R1 の一部・SSP 2.9・ぱすた・200% の画面・MCP だけ）

- 起動した SSP はぱすたで上がった。`currentghost.scope(0).seriko.defaultsurface`＝`1000`、`scope(1)`＝`10`、`currentghost.scope(0).surface.num`＝`1000`、`currentghost.scope(0).rect`＝`1710,376,2818,1744`（1108×1368＝面 1000 の 554×684 の 2 倍）、`currentghost.balloon.scope(0).rect`＝`1790,492,2662,1392`。
- `\0\s[9998]\e`（strict）: 誤りの記録に `surface not found (9998)` が 1 件。`surface.num` は `-1` になり、既定の面 1000 は出なかった。rect は変わらない。ukadoc の「存在しない番号を\sタグで指定した時などに代わりに表示する」の字面と違う（裁定 11・起票の brief へ書く）。測った後に `\0\s[1000]\e` で戻した。
- `\0\![lock,balloonmove]\![unlock,balloonmove]\![lock,balloonmove,foo]\e`（strict）: 誤りの記録は 0 件＝SSP は 2 つの指令を知っている（3 つ目の余分な引数も黙って受ける）。測った後に `\![unlock,balloonmove]` で解いた。
- まだ測っていない: 裁定 1（ぱすたのバルーンだけを利用者がドラッグして動くか）・裁定 4（`\0` で lock した後に `\1` のバルーンが動くか）は画面の操作が要る。裁定 2（descript の既定の面がシェルに無いとき）は、そういう descript のゴーストを SSP に置く（環境を変える）か読み直しが要る。

### 10.2 設計の段へ回す事項（要件の討議のカテゴリ B）

9 節の議題 1・2・3・4・7・8・9・10 と 7 節の R2〜R5 は how の判断なので、設計（`/kiro-design`）で決める。とくに議題 4（台本の lock・unlock を UI へ運ぶ道が C11 の ⑥ `dpi-realign-remembered-chain` の持ち物にかかる）は、設計の前に ⑥ のセッションへ相談する。9 節の議題 5（シェルの `*.balloon.dontmove` の別名）と 6（SHIORI だけの読み直しで lock を保つ）は、要件 4.2・5.5 に先例どおり書き足して閉じた。
