# ギャップ分析: areka-P0-shell-balloon

- 作成: 2026-10-03（ブランチ `claude/areka-p0-shell-balloon-4ff8c9`・HEAD `0f0712b3`＝main `d4f9e93d` の直後）
- 入力: `requirements.md`（確定）・`brief.md`（2026-10-01 起票・10-02 再測定）・`.kiro/steering/`（`structure.md`・`tech.md`）
- 方針: brief の記述より HEAD のコードを正とした。コードについての主張はすべて、ファイルと「何の定義か」で示す（行番号は使わない）。「無い」と書いた箇所は、同じ検索が既知の実例に当たることを先に確かめている。
- 本書は選択肢と材料を示すもので、決定はしない（決定は要件ディスカッションと設計）。

---

## 1. 要約

- **一番下の層は用意済み、上の層は 1 スコープ 1 か所の前提で組まれている**。シェルの窓にも文字の層の差し込み口は作られる（`areka-emo-present/src/mount.rs` の `VisualMount::attach`）が、差し込み口は 1 つの窓に 1 つで、文字の層（`areka-emo-text`）の表はすべてスコープ（`ActorKey`）だけを鍵にしている。箱を複数持つには「鍵を広げる」「差し込み口を箱の数だけ持つ」の 2 つが要る。
- **surfaces.txt の読み手は `balloon.*`ブレスも描画メソッド `balloon` も読まない**（どちらも黙って捨てる）。一方、ブレスの中身をバルーンの descript.txt と同じ意味で読む部品（`areka_parsers::balloon::parse`）は KV の表を受け取る形なので、そのまま再利用できる。
- **一番の設計上の難所は「今のサーフェスにどの箱があるか」を文字の層がいつ・誰から知るか**。サーフェスの切替は seriko → 表示層、文字は emo-text と別の道を通る。ただし emo-text は台本の全指令（`\s`・`\b` を含む）を同じ順番で受け取っており、サーフェスの解決は純粋で決定的（`SurfaceResolver::resolve`）なので、0 フレームで解く道はある。その代わり「`\s` と `\b` は seriko だけが働く」という今の区分けの約束を見直すことになる。
- **併用しない規則・選択肢のクリック・状態の届け・時間切れは、どれも「バルーンの窓」を相手に書かれている**。箱はシェルの窓の中にあるので、何も足さなければ「箱の文字でバルーンの窓が出てしまう」「箱の選択肢は押せない」「箱の文字は時間切れで消えない」「`Status` の `balloon(…)` には載らない」になる。
- 規模は L（1〜2 週間）、リスクは中〜高（文字の層の鍵の広げ方と、シェルの窓でのポインタの扱いが未検証）。brief の「`actor.rs`・`region.rs` の分割が先に要る」は、`emo-text-file-split`（PR#217）の完了で解消済み。

---

## 2. 現状の調査（HEAD）

### 2.1 surfaces.txt の読み手（`crates/areka-parsers/src/shell/`）

| 事実 | 根拠 |
|---|---|
| ブレスの振り分けは `descript` → `kero.surface.alias` → `surface.append*` → `surface*` の順。どれにも当たらないブレスは何も積まず、ログも出さずに終わる | `decode.rs` の `dispatch_block` |
| `balloon` という語は読み手の本番ファイル（`decode.rs`・`model.rs`・`mod.rs`・`lexer.rs`・`parse.rs`）に 0 件（同じ検索で `surface.append` は `decode.rs` に 11 件当たる） | grep |
| element定義は描画メソッドが `overlay` のものだけを拾い、それ以外は黙って読み飛ばす。`surface*`ブレスと `surface.append*`ブレスは同じ関数を使う | `decode.rs` の `decode_elements`・`decode_surface_body` |
| element の型は `layer`・`path`・`x`・`y` の 4 欄で、描画メソッドの欄は無い | `model.rs` の `struct Element` |
| X・Y は「読めなければ 0」で読む（`field_i64`）。要件 2.6「整数として読めなければ読み捨てる」とは違う読み方 | `decode.rs` の `decode_elements` |
| `Element` の `elements` 欄を触るファイルは 24（うちテスト以外は 7: emo-compose の `fold.rs`・`plan.rs`・`normalized.rs`・`atlas_bind.rs`・`base_image.rs`・`world.rs` と emo-atlas の `manifest.rs`。残り 17 はテスト） | grep `\.elements\b`（120 件・24 ファイル） |
| 読み手は「忠実な転記」が役目で、検証や警告は下流が行う。emo の 3 段では「純粋な核は事実を戻り値（`…Report`）に載せ、ログは fs を触る入口 1 か所が出す」 | `structure.md`（Parser Crate の Pattern・emo Render Engine Crates） |
| 同じ番号の `surface*`ブレスを 2 度書くと、後のもので丸ごと置き換え、警告を 1 件出す。`surface.append*` は既にある番号へ足し合わせる | `areka-emo-compose/src/fold.rs` の `upsert_surface`・`fold_append` |

### 2.2 バルーンの descript.txt の読み手（`crates/areka-parsers/src/balloon/`）

- `parse.rs` の `parse(descript: &BTreeMap<String,String>, image: Option<&…>) -> BalloonModel` は KV の表を受け取る。ブレスの本体行（先頭の欄＝キー・残り＝値）を表にすれば、**要件 1.2「同じ名前のキーを同じ意味・同じ既定値で読む」は読み手を足さずに満たせる**。
- `windowposition.*` は数値と生の文字列の両方で `BalloonModel` に載る（`map_merged`）。`use_self_alpha`・`use_input_alpha` は `map_merged` が引かないキー（完全一致で引くので自然に無視）。要件 1.5 の「ログに 1 行残して読み捨てる」は、表に写す段で自前で見る必要がある。
- `BalloonModel` はバルーンの名前（descript.txt の `name`）を持たない。`map_merged` が引くキーに `name` は無い（`font.name` はフォント名）。

### 2.3 文字の層（`crates/areka-emo-text/src/`）

`emo-text-file-split`（PR#217）後のファイル割り: `actor.rs` 471 行・`actor_attach.rs` 237・`actor_present.rs` 303・`actor_decoration.rs` 188・`state.rs` 615・`state_decoration.rs` 483・`surface.rs` 792・`region.rs` 499・`choice.rs` 673。**1,000 行の上限に対して余白があり、brief が挙げた「先に分割が要る」は不要になった**。`actor_attach.rs` と `balloon_visibility_decision.rs` の先頭コメントには「足す予定の spec: shell-balloon」と書かれている。

| 事実 | 根拠 |
|---|---|
| 実行時の表はすべて `ActorKey`（スコープ）だけが鍵: 装着先 `routing`・配置の入力 `layout_input`・描画資源 `surfaces`・選択肢の強調 `choice_hover`・選択肢の当たり行 `choice_snapshot`・背景色 `balloon_background`・未解決の警告済み `unresolved_warned` | `actor.rs` の `struct TextLayerRuntime` |
| 文字の状態も `ActorKey` だけが鍵（`actors`・`clears`） | `state.rs` の `struct TextLayerState` |
| `ActorKey` は dola の型で、emo-text は `areka_sakura::contract` 経由で使う | `actor.rs` の `use` |
| `\c`（`Clear`）はそのスコープの中身だけ、台詞の頭（`ClearAll`）は全スコープの中身を消し、装飾も戻す（`reset_for_new_talk`） | `state.rs` の `TextLayerState::apply_cue`、`actor.rs` の `TextLayerRuntime::apply_cue` |
| `\b`（`BalloonSurface`）と `\s`（`Emote`）は文字の状態機械へ**届いているが何もしない**（明示の腕で無視） | `state.rs` の `apply_cue` の末尾の腕 |
| 台本の指令は全員に配られ、各自が「自分の担当か」を 1 つの分類表で決める。`Emote`・`BalloonSurface` は「シェル側（seriko）」、文字まわりは「バルーン側（emo-text）」。「`BalloonSurface` を文字の状態機械へ流すのは誤配線」と明記 | `dola/src/cue/sink.rs` の `cue_target_of`、`areka-emo-text/src/sink.rs` の `impl CueSink for EmoTextSink` |
| 領域の解決は「画像の大きさ」と `BalloonModel` だけから決まる。`validrect` が無ければ画像いっぱい、`origin` が無ければ書き出しの角（横書き＝左上・`VerticalRl`＝右上）、負の値は反対側の端から | `region.rs` の `TextRegion::resolve` |
| 文字の面は「`validrect` の大きさ × k」で作り、窓の中の位置は「`validrect` の左上 × k」。**領域は画像の左上を (0,0) とする前提で、箱の X,Y を足す口は無い** | `actor_present.rs` の `present_actor`（`physical_size`・`physical_offset`）、`surface.rs` の `TextSurface::attach` |
| 選択肢の当たり行の矩形も同じ前提（窓の物理 px・領域の左上から）で作る | `actor_present.rs` の `present_actor`（`to_window_physical`）、`choice.rs` の `to_window_physical` |
| 折り返しの警告と `origin` の警告のバルーン名の欄は、固定の文字列 `"(名前なし)"` | `region.rs` の `BALLOON_NAME_PLACEHOLDER`、使用は `actor_attach.rs` の `warn_coarse_wrap_threshold` と `actor_decoration.rs`（`warn_ignored_origin` の中） |
| 拡大率が変わったときの組み直しは、登録と同じ道を通り、描画資源だけを捨てて文字の進み具合は保つ | `actor_attach.rs` の `TextLayerRuntime::refresh_actor_binding` |
| フォントファイルを読み込む処理は無い。`font.name` の候補がフォントファイル名（`.ttf`／`.otf`／`.ttc`）のときは、読み込まずに警告を 1 度出して次の候補へ進む | `draw_catalog.rs` の先頭コメントと `FONT_FILE_EXTENSIONS`・`WarnKey::FontFile` |

### 2.4 表示層（`crates/areka-emo-present/src/`）

- `mount.rs` の `VisualMount::attach` は、**シェルの窓にもバルーンの窓にも**同じ 2 つの子を作る: 絵の entity（`emo-surface`）と文字の層の差し込み口（`emo-text-layer-slot`）。差し込み口は絵より手前（子の並びの先頭＝最前面）。**差し込み口は 1 つの窓に 1 つ**（`VisualMount` の `text_slot` 欄は 1 つ）。
- `surface.rs`（emo-text）の `TextSurface::attach` は、差し込み口の entity そのものへ文字の面（`VisualGraphics`＋`Arrangement`）を挿す。**1 つの差し込み口に載る文字の面は 1 枚**。
- `presenter/read.rs` の `EmoPresenter::text_slot_view(target)` は相手（シェル／バルーン）を問わず、差し込み口・窓・原寸・物理寸・実際の拡大率 k を返す。シェルの窓の k はそのまま得られる（要件 3.5 の材料）。
- `VisualMount::set_visible` は絵と差し込み口を一緒に切り替える。差し込み口は見えているとき矩形で当たり判定を持ち（`slot_hit_test`＝`HitTest::bounds()`）、隠すと当たらない。
- `presenter/read.rs` の `current_surface_id(target)`・`target_visible(target)` で「今のサーフェス番号」「見えているか」を引ける。

### 2.5 結線（`crates/areka/src/emo2_boot/`）

| 事実 | 根拠 |
|---|---|
| 文字の層へつなぐのはバルーンの窓だけ。シェルの対象は装着するが、`text_slot_view` を取って登録する処理は無い | `frame/attach.rs` の `run_attach_phase`・`connect_balloon_text` |
| 拡大率の追い直しはバルーンの対象だけを回る（`balloon_models` の鍵＝バルーンを装着したスコープ） | `frame/scale_text.rs` の `run_text_scale_phase` |
| スコープ → 表示対象の対応はシェル＝`2*scope`・バルーン＝`2*scope+1`。スコープ → 文字の層の鍵は `ActorKey::from(scope.to_string())` で、装着・追い直し・可視性・入力の 6 か所以上が同じ式を書いている | `target_map.rs` の `shell_target`・`balloon_target`、grep `ActorKey::from(scope.to_string())` |
| バルーンの窓を出す・消すの判断は「そのスコープの見えている文字の数」の増減だけで決まる（増えたら出す・0 に落ちたら消す） | `balloon_visibility_decision.rs` の `decide_content`、観測は `balloon_visibility_phase.rs`（`visible_glyphs(&actor, t)`・`clear_count(&actor)`） |
| 時間切れで消す相手は「見えているバルーンの窓」だけ | `balloon_visibility_wait.rs` の `decide_timeout` |
| `Status` の `balloon(…)` は、バルーンの対象の `target_visible` と `current_surface_id` から作る | `frame/status_report.rs` の `report_balloons`・`collect_bindings` |
| シェル・バルーンの切替は `frame/switch.rs`・`shell_balloon_switch.rs`・`switch_assets.rs`、ゴーストの切替は `ghost_switch.rs` が持つ（要件 6.8 の当事者） | ファイルの先頭コメント |

### 2.6 `\b` の今の道筋

- `areka-parsers/src/sakura/decode.rs` が `\b[…]` の第 1 引数を文字列のまま `Instruction::BalloonSurface` にし、`areka-sakura/src/compile.rs` の `Instruction::BalloonSurface` の腕が `CueCommand::BalloonSurface { key }` として流す（数値化しない）。
- 受けるのは seriko: `areka-seriko/src/resolve.rs` の `resolve_balloon_key` が、数値として読めれば `Show(id)`／`-1` は `Hide`／負や桁あふれは `Invalid`、**読めなければ `NameForm`**。`areka-seriko/src/actor.rs` の `BalloonResolve::NameForm` の腕が `warn!`（「名前解決できず読み飛ばす」）を出して何もしない。**要件 4.9（名前の形の警告を出さない）はこの腕に手を入れないと満たせない**。
- seriko のバルーンの状態（`areka-seriko/src/state.rs` の `apply_balloon`）は台詞の頭で戻らない（seriko の本番コードに `ClearAll` の語は 0 件。同じ検索は emo-text・ghost・`talk_lifecycle.rs` に当たる）。つまり `\b[ID番号]` の指定は今、次の台詞へ持ち越される。

### 2.7 ポインタ操作（`crates/areka/src/input_events/`）

| 事実 | 根拠 |
|---|---|
| シェルの窓のハンドラは窓の entity に付き、座標から当たり判定の名前を引く（どの子に当たったかは見ない）。送るのは移動（`OnMouseMove`）と左ダブルクリック（`OnMouseDoubleClick`）、右ダブルクリックはメニューへ預ける。**単発のクリックは何も送らない** | `mod.rs` の `on_char_pointer_moved`・`on_char_pointer_pressed`、`emo2_boot/hit_region.rs` の `resolve_hit_region` |
| 選択肢の当たり判定・強調・クリックの確定は**バルーンの窓のハンドラだけ**が行う。スコープは `BalloonWindowMarker` から取り、当たり行は `choice_hit_rows(&actor)` を窓の物理 px のまま突き合わせる | `balloon_pressed.rs` の `on_balloon_pointer_pressed`、`balloon.rs` の `click_selection`・`hit_choice_row`・`hover_action` |
| 確定した選択は `ChoiceSelection` として送り口へ渡り、`choice_drain.rs` が kanade へ運ぶ。運ぶ側は窓の種類を知らない（要件 8.3・8.4 はこの口をそのまま使えば満たせる） | `balloon.rs` の `BalloonWiring::send_selection`、`choice_drain.rs` の `drain_choice_selections` |
| 台詞の中断は「バルーンの窓の左ダブルクリック」だけが入口 | `balloon_pressed.rs` の末尾（`user_break::on_left_press` の呼び出し） |
| 矩形の当たり判定は「見えているか」を見ず、不透明度と前景色の α だけで決まる | `wintf/src/ecs/layout/hit_test/mod.rs` の `hit_test_entity`（`HitTestMode::Bounds` の腕） |

### 2.8 登記先と検体

- `doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」は「項目・裁量・根拠・出典 spec」の 4 列の表で、各 spec が着地時に行を足す（要件 10.3 の登記先として実在）。
- 検体は `vendors/sample_ghost/*.nar` を `sample-ghost-kit` 越しに引く形と、シェルをメモリ上で組む形（`areka-emo-present/src/shell_target.rs` の `build_shell_target`＝fs に触れない核）の 2 つがある。箱を持つ試験用シェル（要件 10.4）はどちらでも作れる。
- 1,000 行の番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 `OVER_LIMIT_ALLOWED`・件数 10）。
- `areka-P0-surface-element-nesting`・`areka-P0-balloon-element-order`・`areka-P0-balloon-font-file`・`areka-P0-talk-fast-forward` はいずれも `.kiro/specs/` に未着手で残っている（`completed/` ではない）。同じ `Element` の型を触る `surface-element-nesting` はまだ着地していないので、読み分けを揃える側は後から着地する方になる。

---

## 3. 要件と既存資産の対応

凡例: **流用**＝既存の部品でほぼ足りる／**不足**＝無いので作る／**制約**＝既存の約束と当たる／**未確認**＝設計で調べる

| 要件 | 既存資産 | 差 |
|---|---|---|
| 1.1 `balloon.*`ブレスを読む | `dispatch_block`（未知のブレスは捨てる） | **不足**: 振り分けの腕と、モデル上の置き場（`Shell` に表を足す） |
| 1.2 キーを descript.txt と同じ意味で読む | `balloon::parse`（KV の表 → `BalloonModel`） | **流用**: 本体行を表にして渡すだけ。読み手は転記にとどめ、`BalloonModel` への写しは下流（brief の境界どおり） |
| 1.3 `size` | なし | **不足**: 2 値のキー。`balloon::parse` の外で読む |
| 1.4・1.5・10.1 不正の記録 | 「核は事実を返し、入口が 1 度だけ記録する」型（`shell_target.rs`） | **不足**: 箱の定義の報告の型。**制約**: 読み手（`areka-parsers`）は検証しない約束なので、検証は下流に置く |
| 1.6 背景なし | 文字の面は透明で初期化（`surface.rs` の `create_transparent_source_tex`） | **流用**。無効表示の混色の相手（`set_balloon_background`）に何を渡すかは **未確認**（箱に背景色は無い） |
| 1.7・2.7・5.4・10.5 今までどおり | 既存テスト一式 | `Element` に欄を足す案では 24 ファイルに波及（4 章 論点 B） |
| 2.1〜2.4 element定義で置く | `decode_elements`（`overlay` だけ・両ブレス共用） | **不足**: `balloon` の読み取り。`surface.append*` と範囲・列挙の展開（`fold.rs` の `expand_targets`・`fold_append`）と同じ規則で番号ごとに配る処理 |
| 2.5・2.6 読み捨て | X・Y は「読めなければ 0」 | **不足**: 箱だけ厳しく読む（既存の `overlay` の読み方は変えない） |
| 3.1〜3.3 箱の中の座標 | `TextRegion::resolve` に箱の `size` を画像の大きさとして渡せばそのまま成り立つ | **流用**。ただし**箱の X,Y を窓の中の位置と選択肢の当たり行へ足す口が無い**（`present_actor`・`to_window_physical`）＝**不足** |
| 3.4 同じキーが同じ結果 | `ResolvedBalloonText::resolve_with_background`・`present_actor` の一連 | **流用**（同じ道を通す限り自動で満たせる） |
| 3.5 拡大率 | `text_slot_view(shell_target)` が k を返す。`refresh_actor_binding` | **不足**: 追い直しがバルーンの対象しか回らない（`run_text_scale_phase`） |
| 3.6 ドラッグ追従 | 箱はシェルの窓の子になる | **流用**（窓が 1 枚なので追従の処理は要らない見込み。実機で確かめる） |
| 3.7・3.8 画像より手前 | 差し込み口は絵より手前 | **流用**＋element番号の大小の警告（**不足**） |
| 3.9 箱どうしの重なり | 子の並びの先頭が最前面（`mount.rs` の冒頭コメント） | **不足**: 箱の数だけ文字の面を並べ、並び順を element番号で決める（4 章 論点 C） |
| 3.10 フォントの探す場所の順 | フォントファイルの読み込み自体が無い | **不足**: 順番を運ぶ口だけ（読み込みは `balloon-font-file`）。何を渡せば「口」になるかは **未確認** |
| 3.11・3.12 警告の名前の欄 | `BALLOON_NAME_PLACEHOLDER`（2 か所で使用） | **不足**: 箱＝ブレスの名前。普通のバルーンは `BalloonModel` に名前が無いので「特定できる名前」の出どころを決める（6 章 追加の論点 9） |
| 4.1〜4.7 行き先 | 文字の状態はスコープに 1 つ | **不足**: 「スコープと箱の名前の組」ごとの状態・スコープごとの今の行き先・今のサーフェスの箱の一覧（4 章 論点 A・D） |
| 4.8・4.9 `\b` の読み分け | seriko の `NameForm` の腕が警告して捨てる | **不足**＋**制約**（`cue_target_of` の区分け） |
| 5.1〜5.3 併用しない | `decide_content`（スコープの文字数で窓を出す） | **不足**: 観測を「普通のバルーンへ向かった文字」だけに絞る |
| 6.1〜6.6 サーフェスの切替 | なし | **不足**: 表示だけやめて文字は持ち続ける仕組み（今の `refresh_actor_binding` は描画資源を捨てて状態を保つ＝近い形がある） |
| 6.7 新しい台詞で全部消す | `ClearAll`（全スコープを空に） | **流用**＋箱の分へ広げる |
| 6.8 シェル・ゴーストの切替 | `frame/switch.rs`・`ghost_switch.rs` | **不足**: 箱の状態と定義を作り直す手続き |
| 7.1〜7.3 `\c` | `Clear`（スコープ単位） | **不足**: 今の行き先だけに絞る |
| 8.1・8.2 選択肢の表示と強調 | `present_actor` の選択肢の流れ・`ResolvedChoiceStyle::resolve` | **流用**（同じ道を通せば出る） |
| 8.3〜8.5 選択肢のクリック | バルーンの窓のハンドラだけ | **不足**: シェルの窓のハンドラで先に選択肢を見る |
| 9.1〜9.3 箱の上の操作 | シェルのハンドラは座標で判定 | 概ね **流用**。差し込み口の矩形の当たり判定が透明な画素の上で何を起こすかは **未確認**（5 章 R-2） |
| 10.3 登記 | `doc/COMPAT_ARCHITECTURE.md` §8 | **流用**（行を足す） |
| 10.4 検体とテスト | `build_shell_target`・`sample-ghost-kit` | **不足**: 箱を持つ検体 |

---

## 4. 実装の分かれ目と選択肢

### 論点 A: 「スコープと箱の名前の組」をどこで持つか

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| A-1 emo-text の中の表の鍵を広げる | `TextLayerRuntime` と `TextLayerState` の表の鍵を「スコープ＋行き先（普通のバルーン／箱の名前）」の型にする。`ActorKey` は変えない | dola・sakura・seriko・ghost に波及しない（brief 再測定の見立てどおり）。普通のバルーンと箱が同じ道を通るので要件 3.4 が自動で成り立つ | emo-text の公開の読み口（`choice_hit_rows`・`choice_active`・`visible_glyphs`・`clear_count`・`is_attached` ほか）の引数が変わり、`emo2_boot`・`input_events` の呼び出し側と多数のテストが追随する |
| A-2 鍵は変えず、スコープの状態の中に箱ごとの子の状態を持つ | `ActorTextState` を「行き先ごとの中身の表」にする | 外向きの読み口の変更を小さくできる（スコープで引いて、既定は普通のバルーン） | 実行時の表（`routing`・`surfaces` ほか 7 つ）は結局箱ごとに要るので、2 種類の持ち方が混ざる |
| A-3 `ActorKey` に箱の名前を連結した綴りを入れる（例 `0#台詞`） | 型は一切変えない | 変更が最小 | `target_map.rs` の `scope_of`（数値として読む）と当たる。名前に区切り文字が現れうる。連結した綴りを鍵にしない、というプロジェクトの規律に反する＝**採りにくい** |

### 論点 B: surfaces.txt のモデルの形

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| B-1 `Shell` と `Surface`／`SurfaceAppend` に別の表を足す | `Shell.balloons`（ブレスの名前と本体行）と、各サーフェスの「箱の置き場所」の表。`Element` は触らない | 画像の合成の道（emo-compose・emo-atlas のテスト以外の 7 ファイル）が 1 行も変わらない＝要件 2.7 が構造で成り立つ | `surface-element-nesting`・`balloon-element-order` が後で「1 つの列挙に揃える」ときにもう 1 度動かす |
| B-2 `Element` に描画メソッドの欄（列挙）を足す | 画像／箱（後にサーフェス）を 1 つの列挙で持つ | 後続 2 spec の最終形に近い。element番号の順序が 1 本の列で分かる（要件 3.8 の警告が素直に出せる） | 24 ファイルへ波及。画像でない element を合成側がすべての箇所で除ける必要があり、要件 2.7 の「前と同じ結果」を守る検査が増える |
| B-3 併用 | 読み手は B-1（転記）、下流の正規化で 1 つの並びにする | 読み手の変更が小さい | 正規化の型（`NormalizedElement`）に同じ問いが移るだけ |

どの案でも、`surface.append*` と範囲・列挙の展開は画像の element と同じ規則（`fold.rs` の `expand_targets`・「その時点で既にある番号だけに足す」）で行う必要がある。箱の置き場所をサーフェス番号ごとに引ける表（番号 → 箱の列）を誰が持つか（emo-compose の `EmoWorld` か、`emo2_boot` の資産か）も併せて決める。

### 論点 C: 1 つの窓に複数の文字の面をどう載せるか

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| C-1 emo-present が差し込み口を箱の数だけ作る | `VisualMount` が「名前 → 差し込み口」を持ち、`text_slot_view` に箱の指定を足す | 可視・非表示・片付け（`set_visible`・`despawn`）が今の 1 か所のまま箱にも効く | emo-present が箱を知る。サーフェスごとに箱の構成が違うので、切替のたびに増減が要る |
| C-2 emo-text が今の差し込み口の下に子を作る | 差し込み口は入れ物のまま、箱ごとの文字の面を子の entity として emo-text が作る・消す | emo-present を変えない。箱の増減が emo-text の中で閉じる。重なり順は子の並びで決められる | 「絵の無い親の下に子の面を並べる」形が wintf で正しく合成・当たり判定されるかは **未確認**（5 章 R-1）。`TextSurface::attach` が挿す相手が変わる |
| C-3 窓いっぱいの文字の面 1 枚に全部の箱を描く | 面は 1 枚、箱ごとに描く範囲を切る | 差し込み口の数の問題が消える | 描画の実行部（`ViewboxExecutor`）は 1 つの領域のスクロールと差分描画を前提にしており、箱ごとの独立したスクロール（要件 3.4）に合わない＝**採りにくい** |

### 論点 D: 「今のサーフェスにどの箱があるか」を文字の層へどう伝えるか

文字の行き先（要件 4.1・4.2・6.1〜6.6）は、そのスコープが今出しているサーフェスで決まる。サーフェスの切替は seriko → 表示層の道、文字は emo-text の道で、別々に届く。

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| D-1 emo-text が自分の指令の列から決める | emo-text は `\s`（`Emote`）と `\b`（`BalloonSurface`）を既に同じ順番で受け取っている。サーフェスの解決表（`SurfaceResolver`＝純粋・別名は先頭固定で決定的）の写しと「番号 → 箱の列」の表を渡し、状態機械の中で行き先を決める | 台本の順番どおりに決まる（`\s[1001]` の直後の文字が必ず新しいサーフェスの箱へ行く）。時計も窓も要らないので決定論のテストが書きやすい | 「`Emote`・`BalloonSurface` に働くのは seriko だけ」という区分け（`cue_target_of` と、区分けを確かめる既存テスト `areka-ghost/tests/ghost/spine_e2e_test_broadcast_relevance_partition.rs`）を見直す。seriko が表示に失敗したとき（無い番号など）に両者の認識がずれうる |
| D-2 結線が表示層の結果を文字の層へ知らせる | シェルの `ShowSurface` を適用した相（`run_drain_phase`）で、結線が「このスコープは今この番号」を文字の層へ伝える | 表示の真実源（`EmoPresenter`）と必ず一致する。区分けを変えない | 文字の指令と表示の指令は別の道で届くので、同じフレームに届いた `\s` と文字の前後が保証されない（文字が先に古い行き先へ入りうる）。行き先を「後から付け替える」仕組みが要り、フレームをまたぐ解になりやすい |
| D-3 文字は「行き先未定」のまま貯め、描く時点で割り当てる | 状態機械は `\s`・`\b[名前]` を区切りとして記録するだけにし、割り当ては描画の直前に結線が渡すサーフェス番号で行う | 表示の真実源と一致し、順番も保てる | 状態の形が大きく変わる（今の「スコープごとに 1 本の列」ではなくなる）。`\c`・選択肢・装飾の既存の約束すべてに影響 |

`\b[名前]` を誰が受けるかも同じ論点に乗る: seriko の `NameForm` の腕を黙らせるには、seriko に「今のサーフェスの箱の名前」を教えるか（seriko は今のサーフェス番号を自分で持つ）、警告の役目を emo-text 側へ移すかのどちらか。要件 4.4 の警告（名前とサーフェス番号つき）と 4.9（ある名前なら警告なし）を**同じ 1 か所**が出す形にしないと、片方が黙っても片方が鳴る。

### 論点 E: 普通のバルーンの窓を出さない判断

`decide_content` は「スコープの見えている文字の数」しか見ない。A-1／A-2 のどちらでも、観測（`balloon_visibility_phase.rs`）を「普通のバルーンへ向かった文字の数」に絞れば、判断の純関数は変えずに要件 5.1〜5.3 を満たせる見込み。要件 6.4（箱の無いサーフェスへ移ったら以後は普通のバルーン）も、行き先が普通のバルーンへ戻れば文字数の増加として自然に窓が出る。

### 論点 F: 箱の選択肢のクリックをどこで受けるか

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| F-1 シェルの窓のハンドラの先頭で選択肢を見る | `on_char_pointer_pressed`・`on_char_pointer_moved` が、箱の当たり行に当たれば `ChoiceSelection` を送って終わり、当たらなければ今までの道へ落とす | 要件 8.5・9.1 が 1 つの分岐で書ける。送り口（`BalloonWiring::send_selection`）と判定の純関数（`click_selection`・`hover_action`）をそのまま使える | `mod.rs` は 546 行。バルーン側のハンドラと同じ借用の手順を 2 つ目として持つ（共通の関数へ括り出せるかは設計で） |
| F-2 箱の文字の面の entity にハンドラを付ける | 箱ごとの entity が自分の選択肢を受ける | 窓のハンドラを変えない | 箱の entity が「選択肢以外」を親へ素通しする保証が要る。箱の増減のたびに付け外しが要る |

強調の解除（ポインタが離れたとき）はバルーン側では `balloon_exit.rs` が持つ。シェルの窓でも同じ離脱の扱いが要る。

---

## 5. 設計で調べること（Research Needed）

- **R-1 子の文字の面の合成と重なり順（wintf）**: 差し込み口の下に子の entity を複数置いたとき（論点 C-2）、合成・位置（`Arrangement` の `offset`）・並び順が期待どおりか。`mount.rs` の `logical_arrangement` のコメントは「`offset` が 0 でないと描画位置と当たり判定の境界が k 倍ずれる」と警告している（絵の entity についての話だが、箱は X,Y の分だけ必ず 0 でない位置に置く）。`TextSurface::attach` は今も 0 でない `offset` を物理 px で書いているので、同じ書き方が箱でも通るかを実測する。
- **R-2 箱の当たり判定と、透明な画素の上のクリックの素通し**: 差し込み口は見えているとき矩形で当たる（`slot_hit_test`）。シェルの絵が透明な画素の上に箱が重なると、今は下のアプリへ抜けるクリックが抜けなくなる可能性がある。要件 9.3（文字が 1 字も無い箱は箱が無いのと同じ）を満たすには、文字が無いあいだは当たらないようにする切替が要る。文字があるあいだの透明な画素の扱いは要件 9.1 からは一意に決まらない（6 章 追加の論点 8）。窓のドラッグ（`placement/follow/drag_follow.rs`）が箱の上から始められるかも併せて実測する。
- **R-3 シェルの窓からのはみ出し**: 文字の面が窓の外へ出たとき、窓の端で切れるのか・窓の大きさの計算に影響するのか（6 章 論点 7）。
- **R-4 表示だけやめて持ち続ける形**: 要件 6.2〜6.6 の「保持したまま表示をやめる」を、描画資源を捨てる（`refresh_actor_binding` と同じ）で実現するか、面を残して隠すだけにするか。戻ったとき（6.3）に全部描き直す前者は既存の道があり、後者は切替が速い。
- **R-5 箱の無効表示の色**: `\f[disable]` の見た目は背景色との混色で作る（`set_balloon_background`・バルーンは面 0 の原点の画素）。箱には背景の絵が無いので、何色を相手にするか。
- **R-6 台詞の途中で届く、台本によらないサーフェスの変化**: シェルの切替（`frame/switch.rs`）や SERIKO の動きで今のサーフェスが変わる場面を、D-1 の「台本の列から決める」がどこまで取りこぼすか。
- **R-7 フォントの探す場所の「口」**: `balloon-font-file` が未着手の今、順番（シェルのフォルダ → ゴーストのフォルダ）をどの型でどこに置けば、後続が受け取れるか。
- **R-8 起動時の装着の時機**: バルーンは起動時に面 0 を見えないまま確立して差し込み口を取る（`run_attach_phase`）。シェルは最初の `\s` まで表示を確立しないので、`text_slot_view(shell_target)` は最初の `\s` まで `None`。箱の登録は「サーフェスが出るたび」に行う形になる。

---

## 6. 要件ディスカッションへ持ち込む 7 点について、コードと正典が言っていること

### 1. 話している最中の、箱の中（選択肢の上を除く）のポインタ操作

- **コード**: シェルの窓は、話しているかどうかに関係なく、移動（`OnMouseMove`）と左ダブルクリック（`OnMouseDoubleClick`）を送る（`input_events/mod.rs` の `on_char_pointer_moved`・`on_char_pointer_pressed`）。単発のクリックは今どこへも送っていない。台詞の中断は「バルーンの窓の左ダブルクリック」だけが入口で（`balloon_pressed.rs` → `user_break.rs` の `on_left_press`）、中断を禁じる区間（`nouserbreak`）の判定もそこにある。
- **つまり**: 何も足さなければ「箱の上のダブルクリックは話している最中でもシェルのダブルクリックとして SHIORI へ届き、中断にはならない」。箱でも中断できるようにする場合、同じ 1 回のダブルクリックで「`OnMouseDoubleClick` を送る」と「中断する」のどちらを取るか（両方か）を決める必要がある。箱しか出していないゴーストでは、今の作りのままだと**利用者が台詞を中断する手段が無くなる**（バルーンの窓が出ないため）。
- **正典**: ukadoc に箱の概念は無い。バルーンのダブルクリックでの中断は SSP の振る舞いで、areka は `areka-P0-balloon-break` で入れている。

### 2. `\b[名前]` の行き先を次の台詞へ持ち越すか

- **コード**: 台詞の頭の `ClearAll` で、文字の状態は中身を消し装飾も既定へ戻す（`state.rs` の `apply_cue`・`reset_for_new_talk`＝「装飾は台詞をまたいで残らない」）。一方、seriko のバルーンの番号（`\b[ID番号]`）は台詞の頭で戻らない（2.6）。
- **つまり**: 「文字の装飾と同じく戻す」「`\b[ID番号]` と同じく持ち越す」のどちらにも、既存の前例がある。戻す場合の差し込み場所（`reset_for_new_talk`）は既にある。持ち越す場合は「次の台詞の最初のサーフェスにその名前が無いとき」の扱い（要件 6.2 と同じ＝既定へ）も併せて決まる。
- **正典**: `ukadoc:list_sakura_script` の `\b[ID番号]` は「現スコープ側のバルーンをID番号のバルーンに変更する」とだけ書き、台詞をまたぐかどうかは書いていない。

### 3. 箱を表示しているあいだの `\b[ID番号]` と `\b[-1]`

- **コード**: `\b[-1]` は seriko が `HideBalloon` を出し、表示層がバルーンの対象の絵と差し込み口を隠す（`mount.rs` の `set_visible`）。文字の状態は消えない（emo-text は `BalloonSurface` を無視する）。箱はシェルの対象の側にあるので、**何も足さなければ `\b[-1]` は箱の文字に効かない**。`\b[ID番号]` は箱を出しているあいだもバルーンの面の番号を切り替える（窓は文字数で出るので、箱へ書いているあいだは見えないまま番号だけ変わる）。
- **正典**: `\b[ID番号]` の項に「`\b[-1]`でバルーンを非表示」。箱を「現スコープ側のバルーン」の読み替えと見るなら箱も隠すのが素直、「普通のバルーンへの指定」（要件 4.8）と見るなら効かないのが素直で、正典からは決まらない。
- **補足**: `\b[ID1,--fallback=ID2]`（SSP 2.6.34）の形は、読み手が第 1 引数だけを取る（`sakura/decode.rs`）。名前の形で `--fallback` を書いたときの扱いは要件に無い。

### 4. `Status` ヘッダの `balloon(ID群)`

- **コード**: `frame/status_report.rs` の `report_balloons` は、バルーンの対象が見えているスコープだけを「スコープ＝面の番号」で並べる。箱しか出ていないときはバルーンの窓が見えていないので、**何も足さなければ `balloon(…)` には載らない**。届ける型（`areka_kanade::BalloonBinding`）は `character_id`・`balloon_id` とも数値。
- **正典**: `ukadoc:spec_shiori3` の Status「balloon(ID群) バルーンが表示状態。キャラクターID=バルーンID の形式で列挙される。…例：balloon(0=2/1=0)」。バルーンIDは数値の前提で、箱には番号が無い。載せるなら「何の番号で載せるか」（名前は入らない）を決めることになり、kanade の型にも波及する。ゴースト側の辞書がこの欄を「バルーンが出ているから話しかけない」の判断に使っている場合、載せないと箱の表示中に雑談が割り込む余地がある（`status-execution-states` の目的と関係する）。

### 5. 普通のバルーンが時間で閉じる場面での箱の文字

- **コード**: `balloon_visibility_wait.rs` の `decide_timeout` は「見えているバルーンの窓」を対象に、台詞が終わってからの待ち時間で隠す。選択肢の表示中・ポインタがバルーンの上・ドラッグ中は待ちを止める。隠すのは窓で、文字の状態は次の台詞の頭まで残る。箱はバルーンの対象ではないので、**何も足さなければ箱の文字は次の台詞が始まるまで出たまま**になる。
- **つまり**: 同じ時機に箱も消す場合、要件 6.2 の「表示だけやめる」と同じ仕組みで足りるが、時間切れの判断（純関数 `decide`）の観測に「箱に文字が出ているスコープ」を足し、待ちを止める条件（ポインタが箱の上・箱に選択肢）も箱へ広げることになる。
- **正典**: `ukadoc:list_shiori_event` の `OnBalloonTimeout`「選択肢以外でバルーンがタイムアウトした際に発生」。箱だけが出ているときにこのイベントを起こすかも、同じ裁定に乗る（areka がこのイベントを今送っているかは本分析では確かめていない＝`balloon-lifecycle-events` の範囲）。

### 6. 定義の重複

- **同じ名前の `balloon.*`ブレスを 2 度**: 近い前例は 2 つ。同じ番号の `surface*`ブレスは「後のもので丸ごと置き換え＋警告 1 件」（`fold.rs` の `upsert_surface`。コメントに「ukadoc 明文規則なし＝de-facto」）。KV の同じキーは「後勝ち・無言」（`kv/parse.rs`）。足し合わせの前例は `surface.append*` だけ。
- **同じサーフェスに同じ名前の箱を 2 つ**: 行き先は「スコープと名前の組」なので 1 つに決まらない（brief も `surface-element-nesting` の項で同じ理由から入れ子の内側の箱を無視すると決めている）。`surface*`ブレスと `surface.append*`ブレスの両方が同じ名前を置いた場合もここに入る。画像の element では同じ番号の扱いは「同じ番号は出現順のまま両方残す」（`decode_elements` の安定ソート）。
- **数字だけの名前**: `resolve_balloon_key` は数値として読めるものを必ず番号として扱う（`-1` は非表示）。数字だけの名前の箱は `\b[名前]` で指せない。読み込みの段で断る（要件 1.4 と同じ扱い）か、置けるが既定の行き先としてしか使えないと割り切るか。`ukadoc` の `\b[ID番号]` は番号の形しか定めていない。

### 7. 箱がサーフェスの画像からはみ出す場合

- **コード**: `TextRegion::resolve` は画像の外かどうかを見ない（箱の `size` を画像の大きさとして受けるだけ）。文字の面は窓の子として X,Y の位置に置かれる。シェルの窓の大きさは絵の物理寸に合わせている（`frame/scale_text.rs` の `reconcile_reported_sizes`）ので、窓の外へ出た分は窓の端で切れると見込まれるが、実測していない（R-3）。サーフェスの大きさは番号ごとに違い、`surface.append*` や着せ替えでも変わりうるので、「はみ出し」は読み込み時に 1 度だけ判定できるとは限らない。
- **正典**: 該当なし（element定義の画像がベースからはみ出す場合の合成の広がり方は ukadoc の element の項の範囲で、箱には当てはまらない）。

### 分析中に見つかった追加の論点

8. **文字の出ている箱が、シェルの絵の透明な画素の上にあるときのポインタ**（R-2）。要件 9.1 は「同じ位置のシェルの絵への操作として扱う」だが、絵が透明な位置は今は下のアプリへクリックが抜ける。箱の文字が出ているあいだ、その位置を「立ち絵の一部」として掴めるようにするか、今までどおり抜けるか。
9. **普通のバルーンの「特定できる名前」（要件 3.12）の出どころ**。`BalloonModel` は名前を持たない。候補はバルーンのフォルダ名、descript.txt の `name`（読み手に 1 本足す）、スコープと面の番号。
10. **`\s`・`\b` に働く演者の区分け**（論点 D）。`cue_target_of` の「`BalloonSurface` を文字の状態機械へ流すのは誤配線」という約束を、areka 独自の `\b[名前]` のために広げるかどうか。
11. **箱を持つサーフェスへ切り替えた瞬間に、普通のバルーンの窓に出ていた文字の扱い**。要件 6.4 は箱 → 普通のバルーンの向きだけを書いている。逆向き（普通のバルーンに書いている途中で箱のあるサーフェスへ）では、要件 5.1 により窓は出さないが、書きかけの文字を持ち続けるのか・箱へは続きだけが入るのかが要件から読み取れない。
12. **`\b[名前,--fallback=…]`** の扱い（3 の補足）。

---

## 7. 規模とリスク

- **規模: L（1〜2 週間）**。読み手・文字の層の鍵・結線・入力・検体の 5 か所にまたがり、文字の層では表 9 つと読み口の形が変わる。先行の分割が済んでいるぶん、brief の見立て（15〜19 タスク）の範囲に収まる見込み。
- **リスク: 中〜高**。
  - 高い側: 論点 D（行き先を決める時機）は既存の区分けの約束と当たり、選び方で状態の形が変わる。R-1・R-2（wintf の合成と当たり判定）は実測していない。
  - 低い側: ブレスの中身の読み取り・領域の解決・選択肢の送り口・登記は既存の部品がそのまま使える。
- **既存の振る舞いを守る材料**: 要件 1.7・2.7・5.4・10.5 は「箱が 1 つも無ければ前と同じ」。論点 B-1（`Element` を触らない）と論点 E（判断の純関数を変えず観測だけ絞る）を選ぶと、この約束は構造で守りやすい。

## 8. 設計フェーズへの申し送り

- 先に決めると後が決まる順: 論点 D（行き先の時機）→ 論点 A（鍵の形）→ 論点 C（文字の面の載せ方）→ 論点 B（モデルの形）→ 論点 F・E。
- 設計の前に実測しておくと選択肢が減るもの: R-1（子の面の合成）と R-2（箱の当たり判定と素通し）。
- 触るファイルの見込み（HEAD の割りで引き直し）:
  - `areka-parsers/src/shell/{decode,model}.rs`（＋ブレスの本体を KV の表にする口）
  - `areka-seriko/src/actor.rs`（`NameForm` の腕）・場合により `resolve.rs`
  - `areka-emo-text/src/{actor,actor_attach,actor_present,actor_decoration,state,state_decoration,surface,choice,region}.rs`（`region.rs` は固定の名前の文字列の撤去だけ）と `lib.rs` の走査一覧（新設ファイルを載せる）
  - 論点 C-1 なら `areka-emo-present/src/{mount.rs,presenter/read.rs}`
  - `areka/src/emo2_boot/{frame/attach.rs,frame/scale_text.rs,frame/switch.rs,assets.rs,balloon_visibility_phase.rs,ghost_switch.rs}`（論点 4・5 の裁定しだいで `frame/status_report.rs`・`balloon_visibility_wait.rs`・`balloon_visibility_decision.rs`）
  - `areka/src/input_events/{mod.rs,balloon.rs}`（＋離脱の扱い）
  - 論点 D-1 なら `dola/src/cue/sink.rs` のコメントと区分けのテスト
  - 検体と `doc/COMPAT_ARCHITECTURE.md` §8
- 同時に走らせない相手（brief どおり・HEAD でも変わらず）: `surface-element-nesting`（同じ element の型）と、文字まわりの後続 spec（`actor*`・`frame/attach.rs` を共有）。
- 要件ディスカッションから設計へ送ったもの（2026-10-03）:
  - 箱がサーフェスの画像からはみ出したときの見え方（R-3 を実測して決める。定義は読み捨てない）
  - 普通のバルーンの「特定できる名前」（要件 3.12）の出どころ（6 章 追加の論点 9）
  - `\s`・`\b` に働く演者の区分けの見直しと、`\b[名前]` の警告を 1 か所から出す形（論点 D・追加の論点 10）
  - 箱の無効表示（`\f[disable]`）の混色の相手の色（R-5）
  - `\b[名前,--fallback=…]` の扱い（追加の論点 12。読み手は今も第 1 引数だけを取る）
  - 要件 2.6 は箱の X・Y だけを厳しく読む（既存の画像の element の「読めなければ 0」は変えない）
- 要件ディスカッションで要件へ入れたもの: 定義の重複（要件 1.8・1.9・2.7）、普通のバルーン → 箱の向きの切替（要件 6.9）、透明な画素の上の箱（要件 9.4＝追加の論点 8・11）。

---

## 9. 設計フェーズの記録（2026-10-03）

1〜8 章はギャップ分析（選択肢と材料）で、決定は本章に記す。発見の進め方は「既存システムの拡張」向けの軽い手順（統合点・既存の型・波及の確認）で、外部の技術調査は行っていない（新しい外部クレートが 0 のため）。

### 9.1 設計で追加で確かめた事実（HEAD・ファイルと定義の名前）

- `Shell`・`Surface`・`SurfaceAppend` は構造体リテラルで組まれている箇所が多い（`Shell {`・`Surface {` のどちらも、テストを含めて約 30 ファイルに現れる）。欄を 1 つ足すとテストを含む全部に波及する。4 章 論点 B の B-1 は「画像の合成の道が変わらない」が、リテラルの波及は残る。
- 字句解析（`areka-parsers/src/shell/lexer.rs` の `lex`）は見出しの語を選ばず、任意のブレスを `BlockStart`／`Line`／`BlockEnd` に切る。`balloon.名前`ブレスも今のまま切り出せる。
- 文字の層は全指令を受け取っている（`areka-emo-text/src/sink.rs` の `impl CueSink for EmoTextSink` は選別せずに積む）。選別は状態機械の腕（`state.rs` の `TextLayerState::apply_cue`）が行う。`cue_target_of` の戻り値を変えなくても、文字の層は `Emote`・`BalloonSurface` を読める。
- 区分けのテスト（`areka-ghost/tests/ghost/spine_e2e_test_broadcast_relevance_partition.rs`）は `cue_target_of` の戻り値だけを確かめている。D-1 を採っても期待値は変わらない。
- 重なり順: 同期していない子（`parent_visual` が無い子）を持つ親は、全部の子を外してから `Children` の順に挿し直す（`wintf/src/ecs/graphics/systems/visual_sync.rs` の `visual_hierarchy_sync_system`）。後から足した子も、`Children` の中の位置どおりの重なり順になる。
- 文字の面は今、窓の直接の子（差し込み口）に、0 でない物理 px の位置で挿されて動いている（`areka-emo-text/src/surface.rs` の `TextSurface::attach`。`validrect` の左上が 0 でないバルーンで常用）。
- 矩形の当たり判定は不透明度だけを見る（`wintf/src/ecs/layout/hit_test/mod.rs` の `hit_test_entity`）。当たり判定を持つ文字の面をシェルの窓へ置くと、透明な画素の上でもクリックを取る。
- 台詞の始まりと終わりは UI 側の中断の持ち物へ届いている（`areka/src/emo2_boot/user_break_cue.rs` の `NoUserBreakSignal::TalkStarted`・`TalkEnded`、畳むのは `areka/src/input_events/user_break.rs` の `drain_no_user_break_signals`）。UI 側に「話している最中か」の旗そのものは今は無い（`areka/src` を `talking` で検索して、テストの補助以外に当たらないことを確認。同じ検索は `spine_conformance_script.rs` に当たる）。
- フレームの中の順番は「装着 → 拡大率 → 表示の指令の適用（`run_drain_phase`）→ 切替 → バルーンの表示の判断 → … → 文字の拡大率の追従（`run_text_scale_phase`）→ 文字の描画（`run_text_phase`）」（`areka/src/emo2_boot/frame.rs` の `emo2_frame_system`）。
- `areka_parsers::balloon::parse` は KV の表を受け取り、`BalloonModel` は `Clone`・`PartialEq`・`Eq`。

### 9.2 設計の分かれ目の決定（4 章 A〜F）

| 論点 | 決定 | 理由 | 捨てた案とその理由 |
|---|---|---|---|
| D | D-1（文字の層が自分の指令の列から決める）。サーフェス番号の解決は seriko と同じ `SurfaceResolver::resolve` を、結線が閉包にして渡す | 文字と `\s` は同じ列を同じ順番で届くので、指令を当てた瞬間に行き先が決まる（0 フレーム）。解決の関数が同じなので、seriko が解決できない `\s` では両者とも状態を変えない | D-2（結線が表示の結果を知らせる）は文字と表示が別の道で届くので、文字が先に古い行き先へ入りうる。後から付け替える仕組み＝フレームをまたぐ解になる。D-3（描く時点で割り当てる）は状態の形が大きく変わり、`\c`・選択肢・装飾の既存の約束すべてに響く |
| A | A-1（表の鍵を `PlaceKey` に広げる）。`ActorKey` を取る既存の読み口は「普通のバルーンの場所」の意味のまま残し、箱の読み口を足す | dola・sakura・seriko・ghost に波及しない。既存の呼び出しとテストを書き換えない。普通のバルーンと箱が同じ道を通る | A-2 は 2 種類の持ち方が混ざる。A-3 は連結した綴りを鍵にする（規律に反する） |
| C | C-2 の変形: 文字の層が、箱 1 つにつき 1 つの entity を**シェルの窓の直接の子**として作る。当たり判定は付けない（2026-10-04 改訂: 表示されている字の矩形の集まりで受ける。design.md R-2） | 今のバルーンの文字の面と同じ構成なので、入れ子の合成（R-1 の未測定の部分）に頼らない。`areka-emo-present` を変えない | C-1 は表示の層が箱を知ることになり、サーフェスごとの増減も要る。C-2 の原案（差し込み口の下の子）は入れ子の位置と拡大率の伝わり方が未測定。C-3 は箱ごとのスクロールに合わない |
| B | B-1 の変形: `Shell`・`Element` を触らず、同じ文面を読む 2 つ目の転記（`parse_boxes` → `ShellBoxes`）を足す | 画像の合成の道が 1 行も変わらず（要件 1.7・2.8 が構造で成り立つ）、構造体リテラルの波及も 0 | B-1 の原案（`Shell`・`Surface` に欄を足す）は 30 を超えるファイルのリテラルに波及する。B-2 は合成側の全箇所で箱を除ける検査が増える。3 通りの読み分けを 1 つの列挙に揃えるのは、後から着地する `surface-element-nesting` 側（brief どおり） |
| E | `decide_content` は変えず、観測する文字の数を「普通のバルーンに今出ている文字の数」（`balloon_shown_glyphs`）へ絞る | 箱のあるサーフェスへ移ると 0 へ落ちて窓が隠れ、戻ると 0 から増えて窓が出る。既存の規則だけで要件 5.1〜5.3・6.4・6.9 が成り立つ | — |
| F | F-1（シェルの窓のハンドラの先頭で座標を見る） | 届くかはシェルの絵と、箱に表示されている字 1 字ずつの矩形で決まる（要件 9.4・2026-10-04 開発者裁定で改訂。旧文は「シェルの絵だけで決まる」）。箱の entity に当たった操作も窓のハンドラへ上がるので、判定は窓のハンドラの座標の 1 か所で足りる | F-2（箱の entity のハンドラで判定する）は、字の矩形の外の箱の四角（要件 9.5）を受けられず、判定が 2 か所に分かれる |

### 9.3 調べる項目（5 章 R-1〜R-8）の結論

- **R-1**: 入れ子にせず、窓の直接の子にして避けた。重なり順は `Children` の並びで決まることをコードで確かめた（9.1）。測定用の検体は足していない。「箱を出し入れするときにシェルの絵がちらつかない」だけを実機の確かめに残す。
- **R-2**: 箱の entity に当たり判定を付けないので、素通しの振る舞いは箱の有無で変わらない。ドラッグもシェルの絵の entity が今までどおり受ける。
- **R-3**: はみ出す定義は採用する。窓の大きさは絵の大きさだけで決まり（`frame/scale_text.rs` の `reconcile_reported_sizes`）、窓の子の合成は窓の範囲で切れるので、はみ出した部分は見えない。最初に面にするときに 1 度警告する。実機の確かめの項目に入れる。
- **R-4**: 「表示だけやめて持ち続ける」は、面を片付けて文字の状態を保つ形にする（拡大率の追い直しと同じ既存の道＝`refresh_actor_binding`）。戻ったときは全部描き直す。
- **R-5**: 箱の `\f[disable]` の混色の相手は既定の白（`DEFAULT_BALLOON_BACKGROUND`）。箱に背景の絵が無く、下のシェルの絵の色は位置で違うため 1 色に決められない。§8 に登記する。
- **R-6**: 行き先は seriko が持つ「スコープの今のサーフェス番号」と同じもの（台本の `\s`）で決める。SERIKO のコマの動きはこの番号を変えないので、取りこぼさない。シェルの切替は箱の束の入れ替え（`set_box_layout`）が全スコープの行き先を引き直す。
- **R-7**: 探す場所の順は純関数 `box_font_search_dirs`（シェルのフォルダ → ゴーストのフォルダ）が決め、`set_box_layout` で文字の層へ渡し、`box_font_dirs` で読める。`balloon-font-file` はこれを読む。
- **R-8**: シェルの窓が確立するまで（最初の `\s` まで）箱は面にしない。文字は既存の「装着先が未解決」と同じく貯まり、確立したフレームで描かれる。箱の登録は毎フレームの `sync_boxes` が行う。

### 9.4 要件ディスカッションから送られた項目の結論

- 普通のバルーンの「特定できる名前」（要件 3.12）: 結線がバルーンのフォルダ名を `set_balloon_label` で入れる。未設定なら `スコープ{番号}のバルーン`。`BalloonModel` に欄は足さない。
- `\s`・`\b` に働く演者の区分け: `cue_target_of` の戻り値は変えない（表示を動かすのは seriko だけ）。文字の層は `Emote` と名前の形の `BalloonSurface` を「行き先を決めるために読む」。`dola/src/cue/sink.rs` と区分けのテストはコメントだけ直す。
- `\b[名前]` の警告を 1 か所から出す形: seriko の `NameForm` の腕は `debug!` へ下げ、文字の層の `route_select` だけが警告する。
- `\b[名前,--fallback=…]`: 読み手が第 1 引数だけを渡すので名前だけが届く。`--fallback` は働かない（§8 に登記）。
- 要件 2.6: 箱の X・Y は畳み込みが厳しく読む。転記は文字列のまま持つ。画像の element の「読めなければ 0」は触らない。

### 9.5 設計で新しく決めたこと

- **「話している最中」**（要件 9.1・9.6・9.7）: 台詞の始まりの合図から終わりの合図まで（選択肢を待っているあいだを含む）。中断を禁じる区間を数えるのと同じ合図の列から畳む。
- **箱を隠す印**（要件 6.10・9.6）: 時間切れと利用者の中断で立て、次の台詞の頭で文字の層が自分で下ろす。中断と次の台詞の始まりが同じフレームに届いたときは、箱は新しい台詞のものなので隠さない（純関数 `hide_reaches_boxes`）。
- **装飾（`\f`）の指定はスコープごと**（2026-10-03 の裁定で「場所ごと」から改めた。経緯は 9.7）: 台本の指定は行き先が替わっても保ち、指定していない項目は行き先の定義の既定値にする。
- **時間切れの観測**: 既存の欄の意味は変えず、`box_showing` を足す。`decide_content` は箱を知らない。
- **`choice_active(&ActorKey)`** だけは、スコープのどの場所かに選択肢があれば真を返すように広げる（待ちを止める条件の単位がスコープ）。

### 9.6 統合（設計の前に 3 つの見方で見直した結果）

- **まとめられるもの**: 要件 6.2・6.3・6.5・6.6・6.10・3.5・6.8 の「箱の文字が見えなくなる・また見える・位置が変わる」は、すべて「あるべき置き場所と登録済みの置き場所を比べ、違えば面を片付けて登録し直す」1 つの手続き（`sync_boxes`）にまとめた。要件 5.1・5.2・6.4・6.9 は観測の数の絞り込み 1 つにまとめた。
- **作るか使うか**: ブレスの中身の読み取りは `balloon::parse`、見出しの展開は `expand_targets`、領域の解決は `TextRegion::resolve`、配置と描画は `present_actor`、選択の確定は `click_selection` と `send_selection`、中断の送り出しは `user_break.rs` を使う。新しく作るのは転記・畳み込み・行き先の状態・箱の同期・箱の押下の判断だけ。
- **削ったもの**: 表示の層の差し込み口を増やす案、箱ごとのポインタのハンドラ、面を残して隠すだけの速い道、箱の背景色をシェルの絵から採る処理。どれも今の要件には要らない。

### 9.7 設計検証（`design-validation.md`）を受けた手直し（2026-10-03）

9.2 の D と 9.3 の R-3・R-6 の記述を、次のとおり改める（決定の向きは変えない）。

- **D の補い（シェルに無い番号）**: 「同じ解決の関数を使うので認識はずれない」は、番号として読めるがシェルに無いサーフェスでは成り立たない。seriko は状態を進めるが、表示の層は `ComposeError::SurfaceNotFound` で前の表示を保つ（`areka-emo-present/src/presenter/show.rs`）。文字の層は画面に合わせる: 結線の閉包（純関数 `resolve_for_text`）が、解決した番号が面の表に在るかも見て、無ければ「解決できない」と同じ結果を返す。
- **R-6 の補い（シェルの切替）**: `set_box_layout` は各スコープの今のサーフェス番号を**保ち**、行き先だけを新しい表での既定へ引き直し、箱の場所の文字を捨てる。seriko が同じ番号のまま出し直す（`areka-seriko/src/actor.rs` の `rebased`）のと揃える。
- **ゴーストの切替で `set_box_layout` を呼ぶ必要は無い**（コードで確認）: `areka/src/emo2_boot/ghost_switch.rs` の `boot_into` が窓を作り直し（`reopen_ghost_windows`）、`ghost_session.rs` が `wire_emo2_boot` を呼ぶ。`wire_emo2_boot`（`emo2_boot/mod.rs`）は `TextLayerRuntime::new` で文字の層を新しく作り、`Emo2Wiring` は `attached: false`（`frame/wiring.rs`）から始まるので、装着の相がもう 1 度走って箱の束を渡す。`TextLayerRuntime::new` の本番の呼び出しは `emo2_boot/mod.rs` の 1 か所だけ。
- **同じフレームの記述の訂正**: 行き先の決定は 0 フレームだが、seriko は別のスレッドなので、表示の指令が 1 フレーム後に届くことはありうる。実機の確かめの項目に入れた。
- **R-3 は未測定**: 「窓の端で切れる」は見込みで、実機の確かめの「はみ出す定義」の項目で確定する。
- **テストの抜けの補い**: 移動の側の判断を純関数（`judge_box_move`・`judge_box_click`・`next_box_hover`・`settle_box_hover`）にし、滞在の印が残らないことを決定論のテストで確かめる。`choice_active` のスコープ全体化と、箱の位置を足した当たり行にもテストを足した。
- **名前が空の `balloon.`ブレス**: `BoxIssue::BraceEmptyName` を足した（記録して採らない）。
- **`\f` の装飾の持ち方の裁定（2026-10-03・要件 3.13）**: スコープごと。根拠は開発者が SSP で目で確かめた 2 点——⑴ `\0\f[color,255,0,0]…\1…\0…` で `\0` は赤・`\1` は既定の黒・`\0` へ戻るとまた赤（`\f` はスコープごと）、⑵ `\0\f[color,255,0,0]…\n\b[2]…`（面 2 を持つバルーン・`Status` は `balloon(0=2)`）で `\b[2]` の後の文字も赤（同じスコープの中で文字の出る先を替えても戻らない）。
  - 持ち方: 今の `Decoration`（`state_decoration.rs`）はすでに「場所の既定（`layers`）」と「台本の指定（`base`・`applied`・`unowned`・`warned`）」に分かれ、`rebase` が前者の上に後者を当て直す。型は変えず、台本の指定の正本を「スコープの今の行き先の場所」に置き、行き先が替わる瞬間に新しい場所へ写して `rebase` する（`carry_script_decor`）。
  - 捨てた案: 台本の指定をスコープの表へ切り出す形。`Decoration` の型と読み口（`current_look`・`unowned_vocab`）が変わり、普通のバルーンだけの構成で「前と同じ」を示すのに既存のテストの書き換えが要る。写す形なら、箱の無いシェルでは写す処理が 1 度も走らない。
  - 箱の場所の 2 層は、文字が届く前に `set_box_layout` が箱の名前ごとに作って状態へ渡す（見た目の番号が場所の既定との差として作られるため）。
  - 規模への影響: `state_decoration.rs` に 50 行程度とテスト 1 ファイル。1,000 行の上限にもタスク数の見立て（15〜19）にも響かない。
- **`font.follow` の裁定（2026-10-03・要件 3.14〜3.19）**: `\f` がスコープに従うか箱に閉じるかを、`balloon.*`ブレスのキー `font.follow`（areka 独自）で箱ごとに選べる。`scope`＝既定（普通のバルーンは常にこれ）・`balloon`＝その箱で書いた `\f` はその箱だけ・不正な値は記録して `scope`・台詞の頭でどちらも戻す。
  - 上の「行き先が替わるたびに前の場所から写す」形は、スコープに従う箱 A → 箱に閉じる箱 X → スコープに従う箱 B で B が X の指定を受け取ってしまうので改めた。
  - 改めた形: スコープごとの行き先の状態（`ScopeRoute`）に `shared`（スコープの指定を今持っている場所。スコープに従う場所だけを指す・始めは普通のバルーン）を足す。行き先が替わるとき、新しい行き先がスコープに従う場所なら `shared` の場所から写して `shared` を新しい場所にし、箱に閉じる箱なら何も写さず `shared` も動かさない。前の場所の種類は見ない。X の中の `\f` は X の `Decoration` にだけ当たるので、X を出たときに写されるのは入る前のスコープの指定になり、X へ戻れば X の指定が残っている。
  - 捨てた案: 「前の場所が箱に閉じる箱なら写さない・新しい場所が箱に閉じる箱なら写さない」と両側で場合分けする形。X を出た先（B）へ何も写さないと、B は入る前の指定（A の赤）ではなく B 自身の古い指定で表示してしまう。
  - `font.follow` を読むのは畳み込み（`fold_boxes`。`size` と同じ段で KV の表から抜く）。不正な値は `BoxIssue::BraceBadFollow`。文字の層へは `BoxDef.follow` → `set_box_layout` → 状態の `set_box_traits`（箱の名前ごとの 2 層と種類）で届く。
  - 規模への影響: 追加は `BoxDef` の欄 1 つ・`BoxIssue` 1 種類（計 13）・`ScopeRoute` の欄 1 つ・`carry_script_decor` の分岐 1 つ・テストの追加。`state_decoration.rs` の増分は 70 行程度の見込み。1,000 行を超えるファイルは無く、タスク数の見立ては 15〜19 の範囲のまま（上寄り）。

### 9.8 残るリスク

- 箱の出し入れのたびに窓の全部の子が挿し直される。ちらつきは実機で確かめる。
- 存在の条件の追い方（`fold_boxes`）が画像の畳み込み（`fold.rs`）とずれること。同じ検体で突き合わせるテストで守る。
- 選択肢を待っているあいだにサーフェスが替わり、選択肢の出ている箱が今のサーフェスから外れると、選べなくなる（時間切れまで待つ）。台本の書き方の問題として扱い、本 spec では手当てしない。
- `surface-element-nesting` が着地するとき、`ShellBoxes` と `Element` の統合が要る（brief で決定済みの順）。
