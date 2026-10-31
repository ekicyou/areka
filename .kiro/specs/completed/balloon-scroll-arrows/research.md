# 調査の記録: balloon-scroll-arrows

## ギャップ分析（2026-10-30・ブランチ `claude/balloon-scroll-arrows-37e29d`・main `484263230` の上）

### 分析の要約

- **名前の選び方の土台はもうある**: `crates/areka-emo-present/src/balloon.rs` の `SeriesFamily`（表の形の族）・`prefix_chain`・`select_faces`・`face_digits_of` が、族の名前を変えるだけで要件 2.1 の順（本体 `arrowp0def`→`arrows`→`arrow`、相方 `arrowp1def`→`arrowk`→`arrowp0def`→`arrows`→`arrow`、三人目以降 `arrowp{n}def`→`arrowk`→`arrowp0def`→`arrows`→`arrow`）をそのまま作る。`scope0_legacy: ["arrows", "arrow"]` と積めばよいことは、完了 `areka-P0-kero-balloon` が檻 `series_family_table_expresses_deeper_legacy_names` と §8 の行で用意済み。クリック待ち（`clickwait*`）も同じ表の 1 行で済む。
- **見える範囲は 1 か所で毎コマ計算している**: `LayoutEngine::visible_window`（`crates/areka-emo-text/src/layout.rs` 433 行）が行の列と文字の範囲だけから自動の位置を出し、`present_actor`（`actor_present.rs` 275 行）が毎コマ呼ぶ。送りの描き直し（`ScrollPlanner`・blit・`ScrollState.committed`）と選択肢の当たり（`to_window_physical` が `committed` を読む）は、渡された `VisibleWindow` に従うだけ。手動の位置は「この 1 か所で別の `VisibleWindow` を渡す」形で入り、`choice.rs` に触らずに当たりも合う（要件 8.5）。
- **無いもの**: 矢印の鍵の読み取り（`BalloonModel` に受け口が無い）・`--数値` の読み方（今のどの座標の鍵も読めない）・矢印の絵の解決と読み込み・矢印を出す面（今の文字の面は文字を描いてよい範囲の大きさしか無い）・普通のバルーンの窓のホイールの受け口・箱の上のホイールの横取り・矢印の当たり・手動の位置の状態とその捨て時・`dump_balloon` への写し。
- **描く層は 3 案**（下の「やり方の案」）。推す見込みは「文字の層に矢印だけの面を別に足す（案 B）」。今の文字の面へ描く案は、送りの blit で矢印まで動いてしまうのと、areka-balloon の `arrow1` が文字の範囲の外にあるので取りにくい。
- **調べが要る点**: SSP の測り（要件 10 の 6 点）・`\b[ID]` で面ごとの設定ファイルを読み直していない今の作り（要件 1.3・8.6 に効く）・箱の当たりが字の矩形だけである点・絵の透けた所に矢印があると押せない点。

### 1. 今の姿

#### 1.1 バルーンの読み手（areka-parsers）

- `crates/areka-parsers/src/balloon/parse.rs`（252 行）: 2 層（descript.txt＋面ごとの設定ファイル）を後勝ちで 1 つの表にまとめ、鍵を完全一致で引く。座標は `get_scalar::<i32>` で読み、**`--5` は数として読めず `None` になる**（`--数値` を読む所はリポジトリのどこにも無い）。マイナスを右端・下端からに直すのは下流の `crates/areka-emo-text/src/region.rs` の `resolve_coord`（`v >= 0` は素通し・負は `extent + v`）。
- `model.rs` は 774 行で上限に近い。新しい鍵は兄弟の新しいファイルに型を置き、`BalloonModel` には `with_…` の口と読み口を数行だけ足すのが今の流儀（`with_font_decoration_raw` などと同じ）。
- `validation_tests.rs` 190 行のコメントが「arrow… はモデルに accessor が無い」と書く（テストの期待値は distractor の検査だけで、鍵を足しても赤にならない。コメントは直す）。`parse_tests.rs` は 922 行で、テストは兄弟の新しいファイルへ。

#### 1.2 名前の選び方（areka-emo-present `balloon.rs`・701 行）

- `BALLOON_FAMILY` だけが実装された族。`select_faces` は **面の番号ごと**に連鎖を先頭から辿る（`balloonk0` があり `balloonk1` が無ければ面 1 は `balloons1`）。矢印に使うと「`0` は `arrows0`、`1` は `arrow1`」のように番号ごとに別の名前から採ることになる。要件 2.1 は「最初に見つかった名前の `0` と `1` を使う」（名前ごと）と書き、要件 2.5 は片方だけのときを決めている。**番号ごとか名前ごとか**は設計で決める点。
- 走査は `enumerate_file_names` で 1 回・判定は大小を無視（要件 2.6 と合う）。拡張子は `.png` 固定（`FRAME_SUFFIX`・要件の範囲外の `.webp` は `balloon-webp-names`）。
- 絵の読み込み（WIC の bake）は `build_balloon_target_from_faces` が emo-atlas を通して行う。矢印の絵も同じ口で読める見込み。
- 1 スコープの定義は `load_scope_balloon_model(dir, scope, face0)` が **面 0 の設定ファイルだけ**を重ねて作り（`crates/areka/src/emo2_boot/assets.rs` 475 行・`placement/mod.rs` 824 行）、`\b[ID]` で面が替わっても読み直さない（文字の層の `refresh_actor_scale` は同じ model で拡大率と面の大きさだけを追う）。

#### 1.3 見える範囲と送り（areka-emo-text）

- `visible_window` は「最新の行が収まる最小の飛ばし数」を毎コマ決め、`first_visible_line` と `block_offset` を返す。縦書き（`VerticalRl`）は `block_dir = -1`（中身が右へ動く）。純粋な関数で、手動の位置（行の番号）からも同じ式で `block_offset` が出せる。ただし今の式は「あふれが無いときは 0」と分けているので、行 0 を見せる手動の位置の扱い（冒頭の保留改行の空き）は設計で 1 行確かめる。
- 行は画像の px で配置され、拡大率は `ScaleContract` で掛ける。手動の位置を **行の番号**で持てば、拡大率が変わっても同じ行が古い側に残る（要件 8.7 は形の上で満たせる）。拡大率の再追従は描画資源だけを作り直し、純粋状態は残す（`actor_attach.rs` の `refresh_actor_scale`）。
- 場所ごとの実行時の表（`choice_hover`・`choice_snapshot` など）は `TextLayerRuntime` が `PlaceKey`（普通のバルーンと箱）ごとに持ち、`apply_cue` の `Clear`／`ClearAll` と箱の束の受け取り（`set_box_layout`）でまとめて捨てる。手動の位置も同じ置き方・同じ捨て時に乗る（要件 8.2・8.8）。
- `present_actor` は描き直しが要ったコマ（`changed`）だけ提示する。手動の位置が変われば `ScrollPlanner` が blit とダーティ描画を出すので、送りの描き直しの仕組みはそのまま使える。
- 文字の面は **文字を描いてよい範囲の大きさ**（`region.right - region.left` × `region.bottom - region.top` に拡大率）で、範囲の左上に置かれる（`actor_present.rs` 150〜158 行）。
- `\![set,autoscroll,disable]` は実装が無い（範囲の外のまま）。

#### 1.4 箱（シェル内バルーン）

- 箱の定義は `crates/areka-emo-compose/src/boxes.rs` の `BoxDef.model` が **普通のバルーンと同じ `balloon::parse`** で読む。読み捨てるのは `windowposition.*`・`use_self_alpha`・`use_input_alpha` だけなので、`BalloonModel` に矢印の鍵を足せば箱にも自動で届く（要件 7.1）。原点は箱の左上、大きさは `size`。
- 箱に絵は無い（地の色だけ）。箱の束 `ShellBoxAssets`（`crates/areka/src/emo2_boot/shell_box_assets.rs`）が持つ探す場所は書体の `font_dirs`（シェル→ゴースト）だけで、バルーンのフォルダも矢印の絵も持たない。シェルのフォルダを読む権威は `areka-emo-present/src/shell_target.rs`（`surface*.png` の判定は `face_digits_of` を接頭辞違いで共有）。
- シェル内バルーンは ukadoc に無い（ukadoc MCP で「シェル内バルーン」0 件）。areka の `areka-P0-shell-balloon` が決めた独自の仕組みなので、**箱の絵の置き場（議題 1）と箱の上のホイール（議題 6）は SSP では測れない**。
- 箱の窓の面は **表示している字の矩形だけ**でポインタを受ける（`present_actor` の `set_hit_cells(glyph_cells(…))`・shell-balloon 要件 9.4）。矢印の上の押下を箱が受けるには、矢印の矩形も当たりに足す必要がある。押下の判断は `input_events/shell_box.rs` の `box_under_point`（`shown_boxes` の四角＝箱の大きさ全体）と `judge_box_press`。
- 箱の上にポインタがある間の時間切れの止めは既にある（`balloon_visibility_phase.rs` の `box_hovered`）。

#### 1.5 入力（crates/areka/src/input_events）

- キャラ窓のホイールは `char_watch.rs`（238 行）が毎 tick `PointerState.wheel.vertical` を読み、窓ごとの溜め（`wheel.rs` の `WheelAccum`・100 ミリ秒の間引き）を経て `OnMouseWheel` を送る。横は読まない。バルーンの窓のホイールは **どこも読まない**（`balloon_pressed.rs` 23 行のコメント）。brief の再測定どおり、wintf は窓の中のポインタの持ち主へホイールを渡しているので、受け口は `char_watch.rs` を前例に `BalloonWindowMarker` の窓へ絞った新しい系で作れる。
- 要件 6.3 の「120 ごとに 1 行・端数は持ち越し」は `OnMouseWheel` の間引き（100 ミリ秒・和を送る）とは別の規則。`wheel.rs` の型は流用せず、端数だけを持つ小さな溜めで足りる見込み。
- 箱の上のホイール（議題 6）は `char_watch` の 5 番の段（回転を `rotations` に積む前）で、カーソルが「隠れた行のある箱」の上かを `box_under_point` で見て横取りする形になる。
- バルーンの押下は `balloon_pressed.rs`（198 行）の `on_balloon_pointer_pressed` が選択肢の確定 → 利用者の中断（`user_break.rs`）の順に流す。矢印の当たりはその頭に置けば要件 5.5 の「クリックとして扱わない」が 1 か所で済む。座標は窓の物理 px のまま（選択肢の行と同じ・÷k しない）。
- バルーンの窓の当たりは絵の α ≥ 0.5 の画素だけ（記憶「当たりは α ≥ 0.5」）。矢印が絵の透けた所にあると押下が窓に届かない。今回の 2 つのバルーンは矢印の下が不透明だった（下の 1.7）。

#### 1.6 MCP `dump_balloon`

- `crates/areka/src/mcp/dump_balloon.rs`（163 行）は、presenter の `last_shown` の絵（原寸）に文字の面の写し（物理寸）を `dump_balloon_overlay.rs` の `overlay_text` で重ねる。箱は読まない（要件 3.6 が普通のバルーンに限るのと合う）。
- 矢印をバルーンの絵に焼くなら写しは自動で入る。別の面に出すなら、その面の写しを読むか、出している矢印の絵（原寸の BGRA）を原寸の座標で CPU で重ねる。後者は GPU の写しを足さずに済み、原寸の座標そのままなので位置がずれない。

#### 1.7 実物のバルーン（ワークツリーの `target\nar-samples\manual\` の展開形で測った）

| バルーン | 画像 | validrect | 矢印の鍵 | 矢印の絵 | 文字の範囲との関係 | 矢印の下の絵の α |
|---|---|---|---|---|---|---|
| ぱすた `pasta_tategaki`（`vertical,1`） | `balloons0`・`balloonk0` 436×450 | 0,0〜436,450（**画像全体**） | `arrow0` (-12,4)＝x 424・y 4、`arrow1` (-12,-12)＝x 424・y 438 | `arrow0.png`・`arrow1.png` 8×8 | 両方とも範囲の**中** | 255（不透明） |
| 既定 `areka-balloon` | `balloons0` 326×169 ほか | 0,0〜326,`-20`（`balloons0` で y 149 まで） | `arrow0` (303,10)、`arrow1` (303,-20)＝`balloons0` で y 149 | `arrow0.png`・`arrow1.png` 11×10 | `arrow0` は中、**`arrow1` は範囲のすぐ下の外** | 255（不透明・`balloons0`・`balloons2`・`balloonk0` とも） |

- 要件の議題 3 の理由「ぱすたの矢印は文字を描いてよい範囲の外にある」は、今の書庫の descript では成り立たない（ぱすたの validrect は画像全体）。結論（文字の範囲の中にしか写らない形は取れない）は、areka-balloon の `arrow1` が範囲の外にあることで同じく成り立つ。
- どちらのバルーンも `arrow*` だけで `arrows*`・`arrowk*`・`arrowp*def*` は持たない（旧名 `arrow` が要る）。

#### 1.8 台帳と文書

- `doc/ukadoc-coverage/ledger/assets.toml`: `arrow0.x`（717 行）・`arrow0.y`・`arrow1.x`・`arrow1.y` の 4 行は持ち主がもう `balloon-scroll-arrows`。`arrow.filename`（701 行）だけが `balloon-canon-residue` のまま。状態はどれも `absent`。`clickwaitmarker.*` の行は `balloon-canon-residue` のままで、本 spec は持たない（要件 2.7 は土台だけ）。
- `doc/ukadoc-coverage/roadmap-draft.md`: `balloon-scroll-arrows` の `owner_count = 4`（1078 行）・`balloon-canon-residue` の `owner_count = 17`（689 行）・束 13「バルーンの付属画像」の表の行（1124 行「`balloon-canon-residue`（W14・5 件）／`balloon-scroll-arrows`（v0.1.0・4 件）」）。`arrow.filename` を移せば 5 と 16 と表の件数を合わせる。数の検査は `crates/ukadoc-survey/tests/consistency/spec_checks.rs`。
- `doc/COMPAT_ARCHITECTURE.md` §8: 「装飾族の接尾辞なしの旧名」（161 行・未実装の語彙記録）と「`arrow0`／`arrow1` の縦書き再解釈」（205 行・未実装・疑義 SC10＝間接に推すだけ）の 2 行を実装の後の姿に直す。今の ukadoc の `vertical` の項は「arrow0（スクロール上）・arrow1（スクロール下）は、意味としてはそれぞれ右・左へのスクロールになる。画像と位置はバルーン側で用意すること」と明文で書く（ukadoc MCP `ukadoc:descript_balloon:vertical,0/1` で確かめた）ので、205 行の「正典側は未規定」は古い。

### 2. 要件ごとの対応（Missing＝無い・Unknown＝調べが要る・Constraint＝制約）

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1.1・1.4 鍵を読む | 2 層の後勝ちと完全一致引き（`parse.rs`） | Missing: `arrow0.x/y`・`arrow1.x/y`・`arrow.filename` の型と読み口（新しいファイル） |
| 1.2 座標の読み方 | 負の値を端からに直す `resolve_coord`（emo-text） | Missing: `--数値`。今の座標の鍵はどれも読めない（`get_scalar` が `None`）。Constraint: 要件 1.2 の「文字を描いてよい範囲などの座標と同じ読み方」は `--` の点で今の実装と違う。矢印だけ新しい座標の型で読むか、他の鍵にも広げるか（範囲の外）は決めが要る。また validrect の右・下の 0 は端と読む特例（`resolve_far_edge`）があり、矢印の 0 は左上からと読むのが要件どおり |
| 1.3 面ごとの上書き | 2 層マージ | Constraint: スコープの定義は面 0 の設定ファイルだけを重ねる。`\b[ID]` で替えた面の設定ファイル（`balloons2s.txt` など）は今は読まれない |
| 1.5 読めない値の警告 | 読み手は記録を出さない（寛容な写像） | Missing: 警告の出し場所。emo の型（記録は fs を触る入口 1 か所・事実は戻り値に載せる）に合わせるなら、読み手は「読めなかった鍵と値」を戻り値に載せ、emo-present の `load_scope_balloon_model` か文字の層の入口が `warn!` を出す |
| 1.6 既存の振る舞いを保つ | 既存のテスト群 | 鍵を持たないバルーンで何も変わらないことは、追加の型が `None` のままなら構造で保てる |
| 2.1〜2.3・2.6 名前の選び方 | `SeriesFamily`・`prefix_chain`・`select_faces` | Missing: 矢印の族の定数（`base: "arrow"`・`scope0_legacy: ["arrows","arrow"]`・`scope1_legacy: ["arrowk"]`）と、`arrow.filename` を先頭にして無ければ既定の連鎖へ戻す口。Unknown: 番号ごとか名前ごとか（1.2 節） |
| 2.4・2.5 絵が無い・片方だけ | 面 0 が無いと `Err` にする `resolve_balloon_faces` | Missing: 矢印は無くても失敗にしない別の入口（面 0 必在の契約を矢印へ持ち込まない） |
| 2.7 クリック待ちの名前 | 同上 | Missing: 族の定数 1 つと、要件 2.1〜2.3 と同じ順で解けることの決定論のテスト |
| 3.1〜3.3・3.8 出る条件 | `visible_window` の `first_visible_line` | Missing: 「古い側に押し出された行があるか」「手動の位置で新しい側に隠れた行があるか」の判断（行の列と見える範囲から純粋に出せる） |
| 3.4・3.5 置き場と手前 | 文字の面は validrect の大きさ | Missing: 矢印を出す面（案 A〜C）。Constraint: areka-balloon の `arrow1` は範囲の外 |
| 3.6 `dump_balloon` | 背景＋文字の面の写し | Missing: 矢印の写し（CPU で重ねるのが軽い） |
| 3.7 座標が無いとき | — | Unknown: SSP の置き場（要件 10 ⑷） |
| 4.1・4.2 縦書き | `VerticalRl` の送りの符号・座標は左上原点のまま | Unknown: areka 独自の `writing_mode` の `vertical_lr`（古い側は左）で `arrow0` を何と読むか。要件は `vertical,1` だけを書く |
| 4.3・4.4 ぱすたで SSP と同じ所 | — | Unknown: SSP の測り（要件 10 ⑴⑸） |
| 5.1〜5.4 クリックで送る | `ScrollPlanner` の blit | Missing: 手動の位置の状態（場所ごと・行の番号）と限り・戻り |
| 5.5・5.6 クリックとして扱わない | `on_balloon_pointer_pressed` の頭 | Missing: 矢印の当たりの判断（選択肢・中断より先）。Constraint: 矢印が絵の α < 0.5 の所にあると窓に届かない |
| 5.7 拡大率に合う当たり | 選択肢の行は `× k` 済みの物理 px | 同じ規約で矢印の矩形を出せばよい |
| 6.1〜6.7 ホイール | `char_watch.rs` の前例・wintf が値を届ける | Missing: バルーンの窓のホイールの系（新しいファイル）と 120 ごとの端数の持ち越し |
| 7.1 箱の鍵 | `BoxDef.model` が同じ `parse` | 読み手に足せば自動で届く |
| 7.3 箱の絵のフォルダ | 書体の探す場所だけ（シェル→ゴースト） | Missing: 箱の束が矢印の絵（シェルのフォルダとバルーンのフォルダの両方から解いたもの）を持つ口 |
| 7.4 箱の矢印の押下 | `judge_box_press` | Missing: 箱の当たりに矢印の矩形を足す（今は字の矩形だけ） |
| 7.5・7.6 箱の上のホイール | `char_watch` が `OnMouseWheel` を送る | Missing: 隠れた行のある箱の上だけ横取りする分岐 |
| 8.1 新しい文字で戻る | `apply_cue` の `Text` 等・表示の時刻で字が増える | Unknown: 「新しい文字が出た」を合図の着いた時に見るか、見える字が増えた時に見るか（下の 4 節） |
| 8.2 消去で捨てる | `Clear`／`ClearAll` が `choice_hover` を捨てる所 | 同じ所に足すだけ |
| 8.3 消えたら捨てる | 時間切れは窓を隠すだけで字を残す（`forget_scope` は字を残す） | Missing: 可視の相（areka 本体）から文字の層へ「このスコープの手動の位置を捨てよ」と伝える口 |
| 8.4 時間切れの数え方 | `is_balloon_hovered`・`box_hovered` | 変えない（何も足さない） |
| 8.5 選択肢の当たり | `to_window_physical` が `committed` を読む | 手動の `VisibleWindow` を渡せば自動で合う（`choice.rs` に触らない） |
| 8.6 面の替わり | `BalloonSurface` の合図は文字の層では行き先の切替（名前の形）だけ | Missing: 整数の `\b[ID]` で手動の位置を捨てる所。Constraint: 新しい面の鍵は面 0 の設定ファイルのもの（1.3 の制約） |
| 8.7 拡大率 | 行の番号で持てば保てる | 再追従で描画資源を作り直しても純粋状態は残る |
| 8.8 場所ごと | `PlaceKey` で表を持つ流儀 | そのまま |
| 9.1〜9.4 記録 | emo の「入口 1 か所で 1 回」の型・`logging.md` | Missing: 解決の debug 1 回・手動の始まりと終わりの debug・壊れた絵の warn |
| 9.5・9.6 §8 と台帳 | 4 行は持ち主済み | Missing: `arrow.filename` の持ち主・5 行の状態と注記・`roadmap-draft.md` の数（5 と 16・束 13 の表）・§8 の 2 行の直しと新しい行 |
| 10 SSP の測り | — | Unknown: 6 点すべて（開発者の許し・排他の机） |
| 11.3〜11.5 実機 | emo2 の実機テスト・ぱすたの手順書 `doc/pasta-real-run.md` | Unknown: 箱を持つ実機の検体が見当たらない（箱の固定のテストは `crates/areka-emo-text/tests/fixtures/shell-balloon/` にだけある）。実機の確かめは `target\` の下に箱を持つシェルを組む必要があるかもしれない |

### 3. やり方の案

#### 案 A: 今の文字の面に矢印も描く

- 文字の面を「validrect と矢印の矩形を合わせた大きさ」か「バルーンの絵の全体」へ広げ、`ViewboxExecutor` の描画の最後に矢印の絵を描く。
- ✅ 面を増やさない。`dump_balloon` は今の文字の面の写しで矢印も写る。
- ❌ 送りは面の中身全体を blit するので、止まっているべき矢印も動く。毎回の送りで矢印の矩形を消して描き直すダーティの規則が要り、`viewbox_diff.rs`・`viewbox_draw_render.rs`（661 行）に手が入る。
- ❌ 面の原点と大きさが変わると、選択肢の当たり（`to_window_physical` の原点）・箱の字の矩形・`dump_balloon` の重ね位置・既存の多数の檻の前提が一緒に動く。

#### 案 B: 文字の層に「矢印の面」を場所ごとに足す（推す見込み）

- 普通のバルーンと箱の場所ごとに、矢印だけを描く小さな面（矢印 1 枚ごと、またはバルーンの絵の大きさ 1 枚）を文字の面の手前に置く。矢印の出る・出ないは `present_actor` が行の列と見える範囲から決め、変わったコマだけ描き直す。
- 名前の選び方は emo-present の `balloon.rs` に族を足し、絵の読み込みは今の bake の口を使う。手動の位置と矢印の判断は emo-text の新しい純粋なファイル（`lib.rs` の `PURE_SOURCES` に載せる・brief の「1 本だけの席」）。
- ✅ 送りの blit と独立（矢印の面は送らない）。文字の面の形も当たりの規約も変えない。
- ✅ 後に続く `balloon-markers`（状態の印）・`talk-fast-forward`（クリック待ちの印）が同じ面と置き方に乗れる。
- ❌ 面を窓の子として手前に挿す口が要る。箱は `attach_window_child` の前例（差し込み口の直後に element番号の順）があるが、普通のバルーンは今は予約スロット 1 つへの装着だけで、矢印の面の並び（文字より手前）を決める口を emo-present の装着か文字の層に足す。
- ❌ `dump_balloon` に写しの口を 1 つ足す（矢印の絵の原寸の BGRA を原寸の座標で重ねるなら GPU の写しは要らない）。

#### 案 C: 矢印をバルーンの絵の側（emo-present）で重ねる

- 矢印の出る・出ないの状態をバルーンの提示に渡し、バルーンの絵に矢印を重ねて提示し直す（4 通りの見た目をキャッシュに持つ形も取れる）。
- ✅ `dump_balloon` は `last_shown` で自動で写る。当たりの α の型に矢印の画素も入るので、透けた所の矢印も押せる。
- ❌ 文字の層の状態（押し出された行）を提示の層へ毎回渡す逆向きの流れが要る（今の依存は emo-present → emo-text の順だけで、emo-present は emo-text を知らない）。
- ❌ 箱はシェルの絵の中なので、箱の矢印のためにシェルの提示まで動かす。文字の更新で再合成しない今の決め（R9.3）とぶつかる。

#### 組み合わせ（案 B を軸に）

- 読み手（areka-parsers の新しいファイル）＋族の定数（emo-present `balloon.rs`）＋判断と手動の位置（emo-text の新しい純粋なファイル）＋矢印の面（emo-text・案 B）＋入力（areka の新しいホイールの系と押下の頭の分岐・`char_watch` の箱の横取り）＋写し（`dump_balloon` の CPU の重ね）＋台帳と §8。

### 4. 要件の議題 6 点について、コードが言うこと

1. **箱の矢印の絵のフォルダ**: 箱の束は書体の探す場所（シェル→ゴースト）しか持たず、バルーンのフォルダも絵も持たない（`shell_box_assets.rs`）。シェルのフォルダの読み込みの権威は `shell_target.rs` で、名前の判定 `face_digits_of` は接頭辞違いで共有できる。シェル内バルーンは ukadoc に無い areka 独自の仕組みなので、SSP では測れず areka の決めになる。バルーンのフォルダは 1 体のゴーストに 1 つ（スコープで違うのは系列だけ）なので、「そのスコープが今使っているバルーンのフォルダ」の側は普通のバルーンと同じ連鎖を同じフォルダで引けばよい。
2. **手動の最中の新しい文字・時間切れ**: 見える範囲は毎コマ行の列から計算し直す作りで、手動の位置は場所ごとの表に置ける（`choice_hover` と同じ）。消去は `apply_cue` の `Clear`／`ClearAll` が今も場所ごとの表を捨てる所があり、そこに足せる。時間切れは窓を隠すだけで字を残す（`forget_scope` は字を残す）ので、「消えたら捨てる」には可視の相から文字の層への口が要る。時間切れの止め（バルーン・箱の上）は既にあり、変えずに済む。「新しい文字が出た」は、字が台本のタグの切れ目ごとに時刻つきで届き、そのうえ表示の時刻で 1 字ずつ見える数が増える作りなので、合図の着いた時に捨てるか、見える字（または行）が増えた時に捨てるかで振る舞いが違う（バルーンの可視の相は「見える字の数が増えた」を `Content` として既に見ている）。
3. **描く層**: 文字の面は validrect の大きさで、送りは面の中身を blit する。ぱすたのバルーンは validrect が画像全体（矢印は範囲の中）だが、areka-balloon の `arrow1` は範囲のすぐ下の外。`dump_balloon` は「バルーンの絵＋文字の面の写し」を重ねる作りで、別の面の矢印は原寸の絵を CPU で重ねれば写せる。議題の理由の書き方（ぱすたの矢印は範囲の外）は今の書庫に合わないが、結論は areka-balloon で同じく成り立つ。
4. **縦書きの向きと出る条件**: ukadoc の `vertical` の項の明文を MCP で確かめた（右・左）。`visible_window` は `VerticalRl` で中身を右へ動かす符号を既に持ち、古い側は右。areka には独自の `writing_mode` の `vertical_lr`（古い側は左）もあり、`vertical,1` だけを書く要件の外になる。§8 の 205 行は「正典は未規定」のままで古い。出る条件・送る量・座標が無いときの置き場はコードからは決まらない（SSP の測り）。
5. **`arrow.filename`**: 2 層の後勝ちで面ごとの設定ファイルの値が descript を上書きする所は今もある。ただしスコープの定義は面 0 の設定ファイルだけを重ねるので、「`balloonk0s.txt` に書けば相方だけ名前を変えられる」は面 0 については成り立ち、`\b[ID]` で替えた面の設定ファイルは今は読まれない。連鎖は `prefix_chain` の先頭に書かれた名前を 1 つ足し、無ければ今の連鎖へ戻る形で作れる。
6. **箱の上のホイールと `OnMouseWheel`**: キャラ窓のホイールは `char_watch` が窓ごとに溜めて 100 ミリ秒ごとに `OnMouseWheel` を送る。箱の四角（`shown_boxes`・箱の大きさ全体）と `box_under_point` は既にあるので、回転を溜めに積む前に「隠れた行のある箱の上か」を見て横取りできる。ただし箱の窓の面は字の矩形だけでポインタを受けるので、字の無い所の回転はシェルの絵の当たりとして届く（座標で箱を引く今の判断なら箱の上と分かる）。シェル内バルーンは areka 独自なので、ここも SSP では測れない。

### 5. 規模と危うさ

- **規模: L（1〜2 週）**。読み手・族・判断と手動の位置・矢印の面・普通のバルーンと箱の入力・写し・台帳と §8・SSP の測り・2 つの実機の確かめと、層が 5 つにまたがる。brief の 13〜16 タスクの見立てと合う。
- **危うさ: 中**。どの部品も前例（族の表・送りの計画・`char_watch`・箱の子の面の装着・`dump_balloon` の重ね）に乗るが、矢印の面を窓の子として手前に挿す口が新しく、SSP の測りで出る条件や送る量が推奨と違えば要件が動く。

### 6. 設計の段への申し送り

- 推す見込み: 案 B（文字の層に矢印の面）を軸にした組み合わせ。
- 要件の話し合い（2026-10-30・全議題を推奨の案で確定）で決まった点: 下の 1（番号ごと＝要件の議題 7）・5（見える字が増えた時＝議題 2）・11（本 spec で面ごとの設定ファイルを読み直す＝議題 8・要件 1.7）。残りの 2・3・4・6・7・8・9・10 は設計で決める。
- 決める点:
  1. 矢印の連鎖を番号ごと（`select_faces` と同じ）に採るか、名前ごと（要件 2.1 の字面）に採るか。
  2. `--数値` を矢印の座標だけの新しい型で読むか（他の鍵は範囲の外）。座標の型をクリック待ちなど後の印と共有する置き場。
  3. 読めない値の警告を出す所（読み手は記録を出さない・入口 1 か所の型）。
  4. 矢印の面の形（矢印ごとに 1 枚か、バルーンの絵の大きさで 1 枚か）と、普通のバルーンの窓で文字の面より手前に挿す口（emo-present の装着か文字の層か）。
  5. 「新しい文字が出た」を合図の着いた時に見るか、見える字が増えた時に見るか。
  6. 時間切れ・トークの終わり・ゴーストの切り替えで手動の位置を捨てる口（areka 本体の可視の相 → 文字の層）。
  7. 箱の当たり（字の矩形だけ）へ矢印の矩形を足す形と、箱の上のホイールの横取りを `char_watch` のどこに置くか。
  8. 絵の透けた所（α < 0.5）に矢印があるときに押せるようにするか（今回の 2 つのバルーンは不透明で起きない）。
  9. `vertical_lr`（areka 独自）での `arrow0` の読み（古い側＝左か、画面の右か）。
  10. 行 0 を見せる手動の位置の `block_offset`（冒頭の保留改行の空きを見せるか）。
  11. `\b[ID]` で面ごとの設定ファイルを読み直していない今の作りを、本 spec で直すか（要件 1.3・8.6 の「新しい面の矢印の鍵」に効く）、面 0 の値のままで §8 に残すか。
- 調べが要る点（Research Needed）:
  - SSP の測り（要件 10 の ⑴〜⑹・開発者の許しと排他の机）。
  - 普通のバルーンの窓へ文字の面より手前の子の面を足せるか（wintf の子の並びと emo-present の装着の口）。
  - 箱を持つ実機の検体の有無（無ければ `target\` の下に組む）。
  - 矢印の絵を拡大率で描くときの補間（バルーンの絵と同じ見え方になるか）。

## 設計の段の記録（2026-10-30・`/kiro-spec-design balloon-scroll-arrows -y`）

### 要約

- **発見の種類**: 拡張（light）。既存の族の表・見える範囲の 1 か所・窓の子の面・`char_watch` の前例・`dump_balloon` の重ねに乗る。外の crate は足さない（Web の調べは不要）。
- **使った決まりと資料**: スキル `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、テンプレート `.kiro/settings/templates/specs/design.md`・`research.md`、steering の `product.md`・`tech.md`・`structure.md`・`focus.md`・`roadmap.md`（ウェーブ C13 の兄弟）、記憶の開発者の方針（本質の設計・1 フレーム遅らせない・変わる時点が分かるならプッシュ・当たりは α ≥ 0.5・外の crate はそのまま・実機の根はワークツリーの `target\`・メッセージボックスを出さない）。ukadoc MCP で `arrow0.x` と `windowposition.x`／`.y`／`.limit` の項を引き直した。
- **鍵になった発見**:
  1. 面ごとの定義は `BalloonScopeAssets.model`（面 0 の設定ファイルだけ）→ `Emo2Wiring.balloon_models` → 毎フレームの再追従で使われる。面の番号の一次の情報源は presenter の `current_surface_id`、面が替わる時点は drain の相（`drain_resnap.rs`）で分かる。文字の層の再追従は既にモデルの値で比べている（`ResolveInputs::is`）ので、「今の定義」を差し替えれば同じフレームで作り直る。
  2. 正典は `windowposition.x`・`.y`・`.limit` を「SSP に限り、descript.txt にも書ける」とする＝本来は面ごとの設定ファイルの鍵。要件 1.7 の「すべての鍵」は配置の鍵も含む。今の作りは配置へ面 0 の値を起動とシェルの切り替えの時だけ入れ、バルーンの切り替え（`finish_balloon`）でも入れ直していない。
  3. `dump_balloon` の `overlay_text` は「物理寸の面を原寸へ縮めて重ねる」だけの純粋な関数なので、印の面を画像全体の大きさで作れば、ずらし 0 でそのまま使える（新しい重ねの関数が要らない）。

### 要件 1.7 の重なりの確かめ（2026-10-30・`main` `484263230`）

- 調べたもの: `.kiro/specs/` 直下の spec（`completed/` と本 spec を除く）で design.md／tasks.md を持つもの＝このワークツリーでは 0 本。並走のワークツリー（`git worktree list`）のブランチ: `claude/blissful-wing-97d9e5`（`stay-on-top-directives`・要件まで）・`claude/seriko-pattern-calls-175601`（`seriko-pattern-calls`・要件まで）・`claude/animated-surface-log-flood-54fed6`（`animated-surface-log-flood`・要件まで）・`claude/kiro-next-725a46`（差分なし）。どれも roadmap の C13 の兄弟で design.md・tasks.md は無い。各ブランチの brief・requirements・research を `git show` で読み、ファイル名を拾った。
- 結果:
  - 要件 1.7 の対象（`crates/areka/src/emo2_boot/assets.rs`・`crates/areka/src/placement/mod.rs`・`crates/areka-emo-present/src/balloon.rs`・`crates/areka-emo-text/src/actor*.rs`・`viewbox*.rs`）を直すと書いた並走の spec は **0 本**。
  - `animated-surface-log-flood`: `crates/areka-emo-present/src/balloon.rs` は調べた範囲に挙げただけ。`crates/areka-emo-text/src/actor_scroll_retain_tests.rs` は「水準の主張があるものだけ直す」候補のテスト（本 spec は触らない）。
  - `stay-on-top-directives`: `crates/areka/src/emo2_boot/frame/switch.rs`（`finish_shell` の重なりの基底・491 行あたり）・`frame/wiring.rs`・`frame.rs` を今の姿として引く。本 spec は同じ `switch.rs` の `finish_balloon`、`wiring.rs` の欄、`frame.rs` の `mod` の行を触る＝同じファイルの別の所（許す重なりの候補・向こうの設計の後に着地の順で合わせる）。
  - `seriko-pattern-calls`: `crates/areka/src/emo2_boot/mod.rs` と seriko・emo-compose で、重なり 0。
  - brief だけの後のウェーブで同じファイルを名指すもの: `placement-measure-bake-once`（`assets.rs`・`placement/mod.rs`・案 B なら `balloon.rs`＝C14・本 spec の後と決めてある）・`extra-character-windows`・`balloon-font-file`・`ghost-alias-file`・`multi-ghost-runtime`・`placement-crate-split`・`shell-scale-linkage`・`shell-balloon-scale-setting`・`balloon-canon-residue`・`balloon-webp-names`・`balloon-markers`・`talk-fast-forward`。本 spec は `balloon.rs` を触らず（新しい `balloon_definitions.rs`・`balloon_marker.rs` へ）、`placement/mod.rs` は `mod` の 1 行だけ。

### 設計の決め（research の 6 節の 2・3・4・6・7・8・9・10。1・5・11 は要件の討議で決まった）

| 番号 | 決め | 理由 |
|---|---|---|
| 1（議題 7） | 番号ごと（`select_faces` と同じ）。明示の名前は連鎖の先頭 | 要件 2.1 の改訂どおり |
| 2 | `--数値` は読み手の新しい型 `MarkerCoord` で矢印の鍵だけ読む。型は後の印（クリック待ち・状態の印）と共有 | 他の鍵へ広げるのは範囲の外（要件 1.2） |
| 3 | 読み手は事実（`KeyIssue`）を返すだけ。警告は fs の入口 `load_scope_balloon_definitions` がスコープにつき重複なしで 1 回 | emo の「記録は入口 1 か所」。descript の値は全部の面に重なるので面ごとに鳴らさない |
| 4 | 印の面は場所ごとに 1 枚・画像（箱）全体の大きさ・文字の面の直後の窓の子。描画範囲の外の矢印も出て、送りの blit と独立、`dump_balloon` はずらし 0 で `overlay_text` を流用 | 案 A は blit で矢印が動き面の原点が動く・案 C は emo-present が emo-text を知る逆向きの流れが要る |
| 5（議題 2） | 見える字の数が増えた時（`observe_visible`） | 要件 8.1 |
| 6 | 隠したときは可視の相（`balloon_visibility_phase.rs`）から `forget_manual_at(…, Hidden)`。消去は `apply_cue` の腕、箱の登録を外すのは `actor_box.rs`、面の切り替えは `select_balloon_face`、ゴーストの切り替えは結線ごと作り直る | どれも変わる時点が分かる所から届ける（プッシュ） |
| 7 | 箱の矢印の押下は箱の押下の判断の頭で `marker_at_point`。箱の上のホイールは `char_watch` の 5 番の段で、回転を積む前（透けた所の判断より前）に `box_under_point` と `has_hidden_lines_at` を見て横取り | 箱の当たり（字の矩形）に手を入れず、印の面のマスクで矢印の画素を受ける |
| 8 | 印の面の当たりのマスクを矢印の絵の α ≥ 0.5 の画素にする＝バルーンの絵が透けた所の矢印も押せる | 当たりの原則（α ≥ 0.5）に合い、SSP を写さない |
| 9 | `arrow0`＝古い側へ送る矢印（`vertical_rl` は右・areka 独自の `vertical_lr` は左）。§8 に登記 | 書字方向の正準表に揃える |
| 10 | 手動の位置は行 0 でも自動と同じ式（一番古い行が描画範囲の始まりに来る・冒頭の保留改行の空きは見せない） | 要件 5.3 の字面。自動の送りが先頭行 0 であふれを解いたときと同じ見え方 |
| 11（議題 8） | 定義は面ごとに読み込みの時に作り（`ScopeBalloonDefinitions`）、今の定義の書き手は `select_balloon_face` の 1 つ。drain の後に presenter の面の番号を見て差し替え、文字の層・手動の位置・配置の `windowposition` へ同じ点から届ける | 1 か所で直す（要件 1.7）。文字の層の作り直しは今の唯一の権威に任せ、第 2 の道を作らない。1 フレーム遅れなし |

### 統合の 3 つの見方（design-synthesis）

- **一般化**: 印の族（`ARROW_FAMILY`・`CLICKWAIT_FAMILY`）・明示の名前つきの連鎖・`MarkerCoord`・`MarkerPair`・印の面は、`balloon-markers`・`talk-fast-forward` がそのまま乗る口にする。中身（描く・出す）は矢印の分だけ作る。
- **作るか採るか**: wintf の `BitmapSource`（パスから非同期に読む）は採らない＝UI スレッドで読み込みが起き、`dump_balloon` に画素が要り、拡大率と当たりのマスクを文字の面と同じ規約で扱えないため。絵の読みは emo-atlas の `ElementDecoder`＋`Normalizer`、面は文字の層の `TextSurface` と `attach_window_child` の前例を採る。重ねは `overlay_text` を採る。
- **削ったもの**: 矢印 1 枚ごとの面（場所ごとに 1 枚で足りる）・emo-present の `balloon.rs` の改造（新しいファイルで足りる）・`BalloonModel` の座標の他の鍵への `--数値`・ホイールの間引き（端数の持ち越しだけ）・箱の矢印の「バルーンのフォルダの名前を箱の名前で探し直す」読み込み（スコープの普通のバルーンの絵で埋める＝要件の表の理由どおり）。

### 危うさと手当て

- 面ごとの `windowposition` でバルーンが動く振る舞いは SSP で測っていない → 設計の討議の議題 1。測りに 1 点足すことを推す。
- 普通のバルーンの窓へ文字の面より手前の子を挿すのは初めて → 装着の檻（子の並び）と実 GPU のテストで固定。
- 測りの答えが推奨と違う → 差し込み点は `marker_visibility`・`PlaceScroll::step`／`wheel`・`observe_visible`・`default_marker_position`・`balloon_pressed.rs` の分岐の 1 か所ずつ。

## SSP での測り（2026-10-30・要件 10・設計の段で行う＝設計の議題 2）

### やり方

- SSP 2.9.26（`baseware.version` = `SSP/2.9.26 (20261029-1; Windows NT 10.0.26300)`）を `C:\wintools\ssp\ssp.exe --option readonly,dontloadmodule --ghost pasta-in-windows` で起こした（SHIORI を読まない＝ゴーストは自分から話さない・pasta の 9276 も握らない）。排他テストの机を取り、ウェーブの兄弟（seriko-pattern-calls・stay-on-top-directives・animated-surface-log-flood）に知らせてから測った。
- バルーン: ぱすたの縦書き `窓際のぱすたさん縦書き`（`pasta_tategaki`）と、`ボトル（でふぉ改）`（`bottle`。`arrow0.x,303`・`arrow0.y,10`・`arrow1.x,303`・`arrow1.y,-20`・`validrect.bottom,-20` で、既定の `areka-balloon` と同じ座標）。台本は SSP の MCP の `sakurascript` で送った（`\![change,balloon,ボトル（でふぉ改）]`・`\![set,balloontimeout,600000]`・`\_q` で 15 行など）。
- 見え方は SSP のバルーンの窓（`Tmessageform`・画面の拡大率 150%）を窓の矩形で撮った。SSP の MCP の `dump_balloon` は「before scaling and transparency」の絵で、**矢印を写さない**（押し出された行があっても右上の矢印は出ない）。
- 入力: `SendInput`（`SetCursorPos` で窓の上を確かめた後も）はバルーンの窓に効かなかった（右クリックのメニューも出ない）。窓へ `PostMessage` で `WM_MOUSEWHEEL`・`WM_LBUTTONDOWN`／`UP`（client 座標）を送ると効いた。OS のホイールの行数（`SPI_GETWHEELSCROLLLINES`）は 3。

### 答え（測れた点）

| 点 | SSP の答え | 推奨（要件の本文）との比べ |
|---|---|---|
| ⑴ 横書きの出る条件 | 自動の位置で古い行が押し出されていれば `arrow0` だけが出る。戻している間は `arrow1` も出る。一番上まで戻すと `arrow0` が消える。自動の位置へ戻ると `arrow1` が消える | 同じ（要件 3.1〜3.3） |
| ⑴ 縦書きの出る条件 | ぱすたの `arrow0.png`・`arrow1.png` は 8×8 で**全画素 α=0（透明）**なので、SSP でも何も見えない。ぱすたのバルーンの絵の矢印だけを `bottle` の見える矢印に替えた測り用のバルーン（`vertical,1`・同じ座標）で測った: 自動の位置では右上の `arrow0` だけ、戻している間は右下の `arrow1` も出る。一番右（古い側の端）まで戻すと `arrow0` が消える | 同じ（要件 3.1〜3.3・4.1） |
| ⑵ 縦書きの送る量と向き | `arrow0` のクリックで右（古い側）へ **2 列**、`arrow1` で左へ 2 列。ホイールを上（向こう側）へ 1 目盛りで右（古い側）へ **3 列** | 向きは同じ（要件 4.1・6.1）。量は違う |
| ⑵ 送る量 | 矢印のクリック 1 回で **2 行**（上へも下へも。端で止まる）。ホイール 1 目盛り（120）で **3 行**（OS の行数の設定と同じ数。設定を変えての確かめはしていない） | 違う（要件 5.1・5.2・6.3 は 1 行） |
| ⑶ 戻している最中の新しい字 | `\_w[8000]` の後の新しい行が出たとき、最新の行の位置へ戻った（手動の位置を捨てる） | 同じ（要件 8.1） |
| ⑹ 矢印の押下と早送り・閉じる | **話している最中**に `arrow0` をクリックすると、`\_w[8000]` の待ちが飛ばされて残りの字が出た＝バルーンのクリックとしての早送りが起きた。**話し終えた後**の矢印のクリックは送りだけで、バルーンは閉じなかった。ホイールは話している最中でも早送りにならない | 一部違う（要件 5.5 は話している最中も送りだけ） |
| ⑹ 矢印が消えている所の押下 | 一番上（縦書きは一番右）まで戻して `arrow0` が消えた後、その所をクリックすると、話し終えた後のバルーンのクリックとしてバルーンが閉じた | 同じ（要件 5.6） |
| ⑸ ぱすたの縦書きのバルーン | 矢印が透明なので SSP でも何も見えない。**透明な `arrow0` の所（424,4 の 8×8）をクリックすると、SSP は右へ 2 列送った**（`arrow1` の所で左へ 2 列）＝SSP の矢印の当たりは絵の α を見ない（矢印の矩形） | 違う（areka の当たりは矢印の絵の α ≥ 0.5 の画素だけ＝ぱすたの矢印は押せない） |
| ⑷ 座標が無いときの置き場 | `bottle` から矢印の座標の鍵だけを抜いた測り用のバルーン（326×169・`validrect` 0,0,0,-20・`origin` 14,11・`wordwrappoint.x` -20）で、`arrow0` の左上は (306,15)、`arrow1` の左上は (306,154)（画面の 150% の撮影から ±1 px）。＝右端から 20 px（`326−20`）、上から 15 px、下から 15 px（`169−15`）。1 つの大きさでしか測っていないので、「右端・上下から固定の px」か「`wordwrappoint` などに連動」かは分けられていない | 違う（設計の暫定は描画範囲の右端に右寄せ・上端と下端） |
| ⑺ 面ごとの `windowposition` | `bottle` の写しの `balloons0s.txt` に `windowposition.x,0`・`.y,0`、`balloons2s.txt` に `200`・`-150` を書いた測り用のバルーンで、`\b[2]` でバルーンの窓が x へ +300 物理 px（＝200×1.5）、y は面の高さの差と合わせて動き、`\b[0]` で元の位置へ戻った（ゴーストは `balloon.dontmove,true`・ドラッグは無し） | 同じ（設計の FaceWindowPosition を残す） |
| （ついで）自動の送り | SSP の自動の送りは画素単位で、一番上の行が半分切れて見える | 自動の送りの規則は範囲の外（変えない） |

- 中断と再開: 途中で負荷テストの停止の要請（window-work-area-clamp）が来たので SSP を閉じて机を返し、再開の後に机を取り直して残りを測った。測り用のバルーン 2 つ（`areka-measure-arrows`・`areka-measure-arrows-v`）は開発者の許し（「置いてよい」）を得て `C:\wintools\ssp\balloon\` に置き、測り終えて SSP を閉じた後に消した。

### ついでに見つけたこと

- SSP の `claudia` バルーンは `balloonk0s.txt` などに `arrow.filename,arrowb` を書き、相方だけ別の矢印の絵（`arrowb0.png`）を使う＝面ごとの設定ファイルでスコープごとに名前を変える実例（要件の議題 5 の推奨どおりの使い方）。


## areka での実機の走り（2026-10-31・タスク 8.2・要件 11.3）

### 経緯

- run1（2026-10-30 18:09 UTC）: スクリーンセーバーが動いていて入力を送れず、長い台詞で `arrow0` が出るところまでで止めた。
- run2（22:54 UTC）: 送りは効いたが、崩れを 2 つ見つけた。(1) 古い側へ戻すと 1 行だけ字の上の一部しか描かれない（`03`・`04` の 3 行目、`06` の 2 行目）。(2) 手動の送りの記録が 1 行も残らない。記録の出所は `areka_emo_text::actor::scroll`（`actor.rs` の中の `#[path = "actor_scroll.rs"] mod scroll;`）なのに、design.md の Monitoring と `RUST_LOG` は `areka_emo_text::actor_scroll` と書いていた。
- どちらも `4e0e4a1d6` で直した（古い側への送りは全域を描き直す・Monitoring の名前を `areka_emo_text::actor::scroll=trace` に）。下は直した後の run3 の記録。

### やり方（run3）

- 根: `<w>\target\bsa-verify\root`（`<w>` はワークツリー `C:\home\maz\git\areka\.claude\worktrees\emo2-specimen-freshness-check-b831f3`）。走りの前に作り直した。`nar-sample-path` で開いた `emo2` を `ghost\emo2` に、`areka-balloon` を `balloon\areka-balloon` に写した。記憶（`profile`）は空。
- 起動: `4e0e4a1d6` の上で建て直した debug 版 `target\debug\areka.exe` に、ゴーストとバルーンのフォルダを絶対パスの位置の引数で渡した。`RUST_LOG=info,areka_emo_present::balloon_marker=debug,areka_emo_text::actor::scroll=trace,areka::emo2_boot::frame::balloon_face=debug,areka::boot_config=debug`、`AREKA_APP_SMOKE_EXIT_MS=900000`。起動の前に 9276・9801・9821 の待受が無いこと、スクリーンセーバーと鍵の画面のプロセスが無いことを確かめた。pid 17696、MCP は 9801。
- 記録: `<w>\target\bsa-verify\logs\run3.stdout.log`。
- 台本は MCP の `sakurascript` で送った。`\_q` と `\C` は areka が知らないタグ（`check_script` が `unknown_tag`）なので使わず、`\![set,balloontimeout,600000]` の後に 15 行を `\n` で並べた。
- emo2 は約 20 秒ごとに自分から話す（`OnSecondChange`）ので、ゴーストの話が終わった直後に、台本 → 矢印のクリック → ホイール → 新しい台詞までを 1 つの PowerShell で続けて回した（00:09:41〜00:09:55 UTC）。
- バルーンの窓の矩形は `get_property currentghost.balloon.scope(0).rect` で `1412,610,2064,948`（物理の画素・326×169 の 2 倍）。矢印の中心はここから求めた: `arrow0`＝(303,10) の 11×10 の中心 → 画面 (2029,640)、`arrow1`＝(303,169−20) の中心 → (2029,918)、ホイールはバルーンの中 (1732,770)。
- 入力は `tools/mouse-input.ps1`（クリック）と、同じ SendInput の形でホイールを送る一時の道具。押す前に毎回、カーソルをその点へ動かしてから、点の上の窓のプロセスが起こした areka（pid 17696）であることを確かめた（6 回とも 17696）。カーソルを動かす前に確かめると explorer が返る（areka の窓は、カーソルが上に無いときはクリックが透ける作りのため）。

### 結果（run3）

| 段 | 操作（UTC） | `dump_balloon`（scope 0） | 結果 |
|---|---|---|---|
| 1 | 15 行の台詞（00:09:41.33・話し終わり 00:09:44.52） | `evidence/01-long-arrow0.png` | 1〜4 行目が押し出され、5 行目の上半分が切れて見え、右上に `arrow0` だけが出た（要件 3.1）。合う |
| 2 | `arrow0` をクリック（00:09:45.99） | `evidence/02-after-arrow0-click.png` | 4〜13 行目が見え、`arrow0` と右下の `arrow1` の両方が出た（要件 3.2・5.1）。合う |
| 3 | ホイールを上へ 1 目盛り（00:09:46.81） | `evidence/03-after-wheel-up.png` | 一番上（1 行目）まで戻り、`arrow0` が消えて `arrow1` だけが出た（要件 3.3・6.1）。1〜10 行目がすべて欠けずに描かれた（run2 の崩れは無い）。合う |
| 4 | `arrow1` をクリック（00:09:47.44） | `evidence/04-after-arrow1-click.png` | 新しい側へ送られ、`arrow0` と `arrow1` の両方が出た（要件 5.2）。3〜12 行目は欠けずに描かれ、2 行目は面の上の端で切れて見える（字の欠けではなく、見える範囲の端）。合う |
| 5 | ホイールを下へ 1 目盛り（00:09:48.12） | `evidence/05-after-wheel-down.png` | 段 1 と同じ自動の位置（最新の行）へ戻り、`arrow1` が消えた（要件 3.3・8.2）。合う |
| 6 | ホイールを上へ 2 目盛り（00:09:48.93） | `evidence/06-scrolled-back-before-new-talk.png` | 一番上まで戻り、`arrow1` だけが出た。1〜10 行目がすべて欠けずに描かれた。合う |
| 7 | 戻した最中に 12 行の新しい台詞（00:09:49.35・話し終わり 00:09:54.92） | `evidence/07-new-talk-latest.png`（**run2 の絵**・下の注） | 記録で、新しい台詞で手動の位置を捨てて自動へ戻ったこと（`reason=Cleared`）を確かめた（要件 8.1）。最新の行が見える絵は run2 のもので受ける（開発者の側の判断・2026-10-31）。合う |

- 段 7 の絵: 撮る直前（00:09:55.11）に emo2 が自分の話（`OnSecondChange`）を始め、00:09:55.33 の `dump_balloon` はその話の書き始め（「今日」）を写した（`evidence/07-run3-overwritten-by-emo2-talk.png`）。新しい台詞の最新の行が見える絵は run2 のもの（`evidence/07-new-talk-latest.png`・12 行の新しい台詞の 2〜12 行目と `arrow0` だけ）だけである。run2 と run3 の間の直し（`4e0e4a1d6`）は古い側への送りの描き直しだけで、自動の位置へ戻る道は変えていない。
- 終わり方: MCP の `sakurascript` で `\0\-\e` を送り、`app_exit origin=KanadeStopped(Quit) closed=4`・終了コード 0。記録の ERROR・panic は 0 件。

### 記録の行（run3）

矢印の絵の解決（要件 9.1）と、文字の層が矢印の絵を受け取った行:

```text
2026-10-31T00:08:55.856981Z DEBUG areka_emo_text::actor::scroll: 箱の矢印の絵を受け取った boxes=2
2026-10-31T00:08:55.863750Z DEBUG areka_emo_text::actor::scroll: 普通のバルーンの矢印の絵を受け取った actor=0 up=true down=true
2026-10-31T00:08:55.867627Z DEBUG areka_emo_text::actor::scroll: 普通のバルーンの矢印の絵を受け取った actor=1 up=true down=true
2026-10-31T00:08:55.692124Z DEBUG areka_emo_present::balloon_marker: balloon_marker: 印の絵を解いた scope=0 base="arrow" chain=arrowp0def>arrows>arrow up=<w>\target\bsa-verify\root\balloon\areka-balloon\arrow0.png 11x10 down=<w>\target\bsa-verify\root\balloon\areka-balloon\arrow1.png 11x10
2026-10-31T00:08:55.709564Z DEBUG areka_emo_present::balloon_marker: balloon_marker: 印の絵を解いた scope=1 base="arrow" chain=arrowp1def>arrowk>arrowp0def>arrows>arrow up=<w>\target\bsa-verify\root\balloon\areka-balloon\arrow0.png 11x10 down=<w>\target\bsa-verify\root\balloon\areka-balloon\arrow1.png 11x10
2026-10-31T00:08:55.864095Z DEBUG areka::emo2_boot::frame::balloon_face: balloon_face: 今の定義を差し替えた scope=0 previous=None face=0 cause=Attach
```

手動の送りの始まり・1 段ごとの送り・自動へ戻った行（要件 9.2・9.3）:

```text
2026-10-31T00:09:45.905414Z DEBUG areka_emo_text::actor::scroll: 手動の送りを始めた actor=0 place=Balloon via="矢印"
2026-10-31T00:09:46.688858Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:09:47.364188Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="矢印"
2026-10-31T00:09:48.005783Z DEBUG areka_emo_text::actor::scroll: 手動の送りを終えて自動へ戻した actor=0 place=Balloon reason=ReachedAuto via="ホイール"
2026-10-31T00:09:48.680420Z DEBUG areka_emo_text::actor::scroll: 手動の送りを始めた actor=0 place=Balloon via="ホイール"
2026-10-31T00:09:48.800887Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:09:49.362205Z DEBUG areka_emo_text::actor::scroll: 手動の送りを終えて自動へ戻した actor=0 place=Balloon reason=Cleared via="捨てる口"
```

- 段 2 の矢印で始まり（`via="矢印"`）、段 3 のホイールと段 4 の矢印は 1 段ごとの送り（`trace`）、段 5 のホイールで自動の位置に追い付いて終わり（`reason=ReachedAuto`）、段 6 のホイールで再び始まり、段 7 の新しい台詞で捨てられた（`reason=Cleared via="捨てる口"`）。

## areka での実機の走り（2026-10-31・タスク 8.3・ぱすたの縦書きのバルーン・要件 4.3・11.4）

### やり方（r1）

- 根: `<w>\target\bsa-verify-83\root`（走りの前に作り直した）。`tools\sample-fetch.ps1` で取り寄せを確かめ（最新・`pasta-in-windows.nar` の sha256 `4d2b3efa9f3cd561b0dc3ec3e140e4fdd7b73551fd88f8ef9ba986ca1c206eb9`）、`nar-sample-path pasta-in-windows` で開いたゴーストを `ghost\pasta-in-windows` に、縦書きのバルーンを `balloon\pasta_tategaki` に、`areka-balloon` を `balloon\areka-balloon` に写した。記憶（`profile`）は空。この書庫は `install.txt` に `balloon.directory,pasta_tategaki` を持ち、バルーンをゴーストのフォルダの中の `balloon\pasta_tategaki` に置く。areka はこちらを解いた（記録の `印の絵を解いた` の `up=` の路）。中身は `balloon\` に写したものと同じ。
- 測り用のバルーン: ぱすたの矢印は 8×8 の全画素 α=0 なので、そのままでは置き場が絵に写らない。SSP の測り（上の表の ⑴ 縦書き）と同じ考えで、`pasta_tategaki` の写し `balloon\pasta_tategaki_visarrow`（`name` だけ `窓際のぱすたさん縦書き矢印見える測り用` に替え、`arrow0.png` を不透明のマゼンタ・`arrow1.png` を不透明のシアンの 8×8 に替えた。座標の鍵 `arrow0.x,-12`・`arrow0.y,4`・`arrow1.x,-12`・`arrow1.y,-12` と面の絵は同じ）を置き、`\![change,balloon,…]` で切り替えて置き場を読んだ。その後、本物の `窓際のぱすたさん縦書き` へ戻して同じ操作を回した。
- 起動: HEAD `6b2fd262d` の debug 版 `target\debug\areka.exe`（建て直して最新を確かめた・i686 の補助も `sync-i686-helper.ps1` で確かめた）に `--ghost <w>\target\bsa-verify-83\root\ghost\pasta-in-windows`（絶対パス）。`PASTA_DEBUG`・`PASTA_DEBUG_PORT` はその PowerShell の中だけ消した。`RUST_LOG=info,areka_emo_present::balloon_marker=debug,areka_emo_text::actor::scroll=trace,areka::emo2_boot::frame::balloon_face=debug,areka::boot_config=debug,areka::boot_resolve=debug,areka::input_events=debug`、`AREKA_APP_SMOKE_EXIT_MS=900000`。起動の前に 9276・9801・9821 の待受が無いこと、スクリーンセーバーと鍵の画面のプロセスが無いことを確かめた。pid 3208、MCP は 9801、走行中に 9276 は開かれなかった。
- 記録: `<w>\target\bsa-verify-83\logs\r1.stdout.log`。
- 台本は MCP の `sakurascript` で `\0\![set,balloontimeout,600000]` の後に `1列目`〜`30列目` を `\n` で 30 列並べた。バルーンの切り替えにはぱすたが `OnBalloonChange` で話すので、その話が終わってから台本を送った（最初の 1 回は台本がぱすたの話に上書きされたので回し直した）。
- バルーンの窓の矩形は `get_property currentghost.balloon.scope(0).rect` で `80,452,952,1352`（物理の画素・436×450 の 2 倍＝画面の拡大率 200%）。ホイールはバルーンの中 (218,225) ＝画面 (516,902) で、`SendInput` で送った（1 目盛り 120・OS の行数の設定 3）。毎回カーソルをその点へ動かしてから、点の上の窓のプロセスが起こした areka（pid 3208）であることを確かめた（11 回とも 3208）。
- 置き場は `dump_balloon`（scope 0・バルーンの画像の画素＝436×450）の絵の中で、マゼンタとシアンの画素の外接の矩形を数えて読んだ。

### 結果（r1）

測り用のバルーン（見える矢印・00:17:11〜00:17:24 UTC）:

| 段 | 操作 | `dump_balloon` | `arrow0` の矩形（左上・大きさ） | `arrow1` の矩形 | 結果 |
|---|---|---|---|---|---|
| 1 | 30 列の台詞（自動の位置） | `evidence/t83-vis-01-pushed-out.png` | (424,4) 8×8 | 無し | 1〜11 列目が右へ押し出され、右上に `arrow0` だけが出た（要件 3.1・4.1） |
| 2 | ホイールを上へ 1 目盛り | `evidence/t83-vis-02-after-wheel-up.png` | (424,4) 8×8 | (424,438) 8×8 | 右（古い側）へ 3 列戻った（右端の列が 12 列目→9 列目）。`arrow0` と右下の `arrow1` の両方が出た（要件 3.2・4.1・6.1・6.3） |
| 3 | ホイールを上へ 10 目盛り | `evidence/t83-vis-03-oldest-end.png` | 無し | (424,438) 8×8 | 一番右（1 列目）まで戻って止まり、`arrow0` が消えた（要件 3.3・5.3） |
| 4 | ホイールを下へ 1 目盛り | `evidence/t83-vis-04-after-wheel-down.png` | (424,4) 8×8 | (424,438) 8×8 | 左（新しい側）へ 3 列送った（要件 6.2） |
| 5 | ホイールを下へ 10 目盛り | `evidence/t83-vis-05-back-to-auto.png` | (424,4) 8×8 | 無し | 自動の位置へ戻り、`arrow1` が消えた（要件 3.3・5.4） |

本物の `pasta_tategaki`（透明な矢印・00:17:53〜00:18:13 UTC）:

| 段 | 操作 | `dump_balloon` | 結果 |
|---|---|---|---|
| 1 | 30 列の台詞（自動の位置） | `evidence/t83-real-01-pushed-out.png` | 測り用と同じ列の並び（右端が 12 列目）。矢印は透明で何も見えない |
| 2 | ホイールを上へ 1 目盛り | `evidence/t83-real-02-after-wheel-up.png` | 右（古い側）へ 3 列戻った（右端が 9 列目）＝ホイールで右へ読み返せた |
| 3 | ホイールを上へ 10 目盛り | `evidence/t83-real-03-oldest-end.png` | 一番右（1 列目）まで戻って止まった |
| 4 | ホイールを下へ 1 目盛り | `evidence/t83-real-04-after-wheel-down.png` | 左へ 3 列送った |
| 5 | ホイールを下へ 10 目盛り | `evidence/t83-real-05-back-to-auto.png` | 自動の位置へ戻った |
| 6 | 話し終えた後、自動の位置で透明な `arrow0` の中心 (428,8) ＝画面 (936,468) を左クリック | `evidence/t83-real-06-after-transparent-arrow0-click.png` | 送りは起きなかった（段 5 と同じ絵・手動の送りの記録は無い）。記録はバルーンのクリックとしての `choice_click_rejected reason="inactive"` だけ（要件 4.3・5.6・5.7 のとおり＝透明な矢印は押せない） |

- 終わり方: MCP の `sakurascript` で `\0\-\e` を送り、`app_exit origin=KanadeStopped(Quit) closed=4`・終了コード 0。記録の ERROR・panic は 0 件。

### SSP との比べ

| 点 | SSP 2.9.26（上の「SSP での測り」） | areka（r1） | 比べ |
|---|---|---|---|
| `arrow0` の置き場 | (424,4) の 8×8（`436−12`, `4`） | (424,4) の 8×8 | 同じ |
| `arrow1` の置き場 | 右下（`arrow1.x,-12`・`arrow1.y,-12` ＝ (424,438)） | (424,438) の 8×8 | 同じ |
| 出る条件（縦書き） | 自動の位置では右上の `arrow0` だけ、戻している間は右下の `arrow1` も、一番右まで戻すと `arrow0` が消える | 同じ（測り用の段 1〜5） | 同じ |
| ホイールの向きと量 | 上へ 1 目盛りで右（古い側）へ 3 列 | 上へ 1 目盛りで右へ 3 列・下へ 1 目盛りで左へ 3 列 | 同じ |
| 透明な矢印の絵 | 何も見えない | 何も見えない | 同じ |
| 透明な `arrow0` の所のクリック | 右へ 2 列送った（矢印の矩形で当たる） | 送らない（α ≥ 0.5 の画素だけが当たり） | 違う。ただし要件 4.3・5.7 で決めて §8 に残した既知の違い（上の表の ⑸）で、新しい違いではない |

- 要件 4.4 で開発者に諮る新しい違いは無い。

### 記録の行（r1）

矢印の絵の解決（測り用へ切り替えた所と、本物へ戻した所）:

```text
2026-10-31T00:16:25.813724Z DEBUG areka_emo_present::balloon_marker: balloon_marker: 印の絵を解いた scope=0 base="arrow" chain=arrowp0def>arrows>arrow up=<w>\target\bsa-verify-83\root\balloon\pasta_tategaki_visarrow\arrow0.png 8x8 down=<w>\target\bsa-verify-83\root\balloon\pasta_tategaki_visarrow\arrow1.png 8x8
2026-10-31T00:16:25.868176Z DEBUG areka::emo2_boot::frame::balloon_face: balloon_face: 今の定義を差し替えた scope=0 previous=Some(0) face=0 cause=BalloonSwitch
2026-10-31T00:17:53.449815Z DEBUG areka_emo_present::balloon_marker: balloon_marker: 印の絵を解いた scope=0 base="arrow" chain=arrowp0def>arrows>arrow up=<w>\target\bsa-verify-83\root\ghost\pasta-in-windows\balloon\pasta_tategaki\arrow0.png 8x8 down=<w>\target\bsa-verify-83\root\ghost\pasta-in-windows\balloon\pasta_tategaki\arrow1.png 8x8
2026-10-31T00:17:53.487236Z DEBUG areka_emo_text::actor::scroll: 普通のバルーンの矢印の絵を受け取った actor=0 up=true down=true
```

本物の `pasta_tategaki` での手動の送り（ホイールだけ。1 目盛りに 1 行）:

```text
2026-10-31T00:18:06.460136Z DEBUG areka_emo_text::actor::scroll: 手動の送りを始めた actor=0 place=Balloon via="ホイール"
2026-10-31T00:18:07.401656Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:18:07.518421Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:18:07.643362Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:18:09.401693Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:18:10.277494Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:18:10.419024Z TRACE areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Balloon via="ホイール"
2026-10-31T00:18:10.534912Z DEBUG areka_emo_text::actor::scroll: 手動の送りを終えて自動へ戻した actor=0 place=Balloon reason=ReachedAuto via="ホイール"
```

- 段 2 の 1 目盛りで始まり（3 列）、段 3 の 10 目盛りは 3 目盛り（9 列）で一番右に着いて止まった（残りの目盛りは限りで送らない＝要件 5.3・6.4）、段 4・5 の下への目盛りで自動の位置に追い付いて終わった（`reason=ReachedAuto`）。段 6 の透明な矢印のクリックでは手動の送りの行は出ていない。

## areka での実機の走り（2026-10-31・タスク 8.4・箱の矢印・要件 11.5）

### やり方（r2）

- 根: `<w>\target\bsa-verify-84\root`（走りの前に作り直した）。8.2 の根の `emo2` を `ghost\emo2` に、`areka-balloon` を `balloon\areka-balloon` に写した。記憶（`profile`）は空。
- 箱を持つシェル: 写した `ghost\emo2\shell\master\surfaces.txt` に箱 `talk` を足した（`balloon.talk`ブレス＝`size,105,80`・`font.height,12`・`arrow0.x,-12`・`arrow0.y,2`・`arrow1.x,-12`・`arrow1.y,-12`、`surface0` の element定義に `element1,balloon,talk,250,470`）。置き場は `surface0.png` の α が全部 255 の所（服の上）を選んだ。矢印の置き場が絵に写るよう、シェルのフォルダに不透明な 10×10 の `arrow0.png`（マゼンタ）と `arrow1.png`（シアン）を置いた（要件 7.3 のとおりシェルのフォルダを先に探す）。
- 起動: HEAD `479c1b043` の debug 版 `target\debug\areka.exe`（`cargo build -p areka` で最新を確かめた・i686 の補助は `sync-i686-helper.ps1` で i686 を確かめた）に、ゴーストとバルーンのフォルダを絶対パスの位置の引数で渡した。`RUST_LOG=info,areka_emo_present::balloon_marker=debug,areka_emo_text::actor::scroll=trace,areka::emo2_boot::frame::balloon_face=debug,areka::boot_config=debug,areka::input_events=debug,areka::input_events::char_watch=trace,areka::input_events::shell_box_handler=trace,kanade=debug`、`AREKA_APP_SMOKE_EXIT_MS=900000`。起動の前に 9276・9801・9821 の待受が無いこと、スクリーンセーバーと鍵の画面のプロセスが無いことを確かめた。pid 21468、MCP は 9801、走行中に 9276 は開かれなかった。
- r1（pid 15940）でも同じ操作を回し、箱の矢印・押下・ホイールは r2 と同じ答えだった。ただし `OnMouseWheel` を送った行（`mouse_event_sent`）は kanade の debug の水準なので r1 の `RUST_LOG` では残らず、対照の確かめができなかった。`kanade=debug` を足して r2 を回し直した。下は r2 の記録。
- 記録: `<w>\target\bsa-verify-84\logs\r2.stdout.log`。
- 台本は MCP の `sakurascript` で `\0\s[0]\![set,balloontimeout,600000]` の後に `1行目`〜`20行目` を `\n` で並べた。emo2 は約 20 秒ごとに自分から話すので、その話が終わった直後に段 A（00:27:41〜00:27:50 UTC）と段 B（00:28:00〜00:28:09 UTC）をそれぞれ 1 つの PowerShell で続けて回した。
- シェルの窓の矩形は `get_property currentghost.scope(0).rect` で `2064,610,2828,1704`（物理の画素）。絵は `tools/mouse-input.ps1 -Action shot` でこの矩形を撮り、マゼンタとシアンの画素の外接の矩形を数えて矢印の置き場を読んだ（`arrow0` は窓の中の (634,664) 20×20＝拡大率 2）。`evidence/t84-*` はそこから箱のまわり（窓の中の (420,620) から 280×240）を切り出したもの。切り出した絵の中で `arrow0` は (214,44)、`arrow1` は (214,176)。
- 押下とホイールは `SendInput`（クリックは `tools/mouse-input.ps1`・ホイールは同じ形の一時の道具・1 目盛り 120・OS の行数の設定 3）。毎回カーソルをその点へ動かしてから、点の上の窓のプロセスが起こした areka（pid 21468）であることを確かめた（6 回とも 21468）。矢印の中心は撮った絵の矩形の中心、箱の中の点は面の (290,510) ＝画面 (2592,1350)、箱の外のシェルの点は面の (300,320)（顔）＝画面 (2612,970)。

### 結果（r2）

段 A（話し終えた後の矢印の押下と、隠れた行のある箱の上のホイール）:

| 段 | 操作（UTC） | 絵 | `arrow0` | `arrow1` | 結果 |
|---|---|---|---|---|---|
| 1 | 20 行の台詞（話し終わり 00:27:44.69） | `evidence/t84-A1-overflow.png` | 出た | 無し | 16〜20 行目が見え、箱の右上に `arrow0` だけが出た（要件 7.1・7.2・3.1）。合う |
| 2 | `arrow0` を左クリック（00:27:45.72） | `evidence/t84-A2-after-arrow0-click.png` | 出た | 出た | 古い側へ 2 行送られ 14〜19 行目が見え、右下の `arrow1` も出た（要件 7.4・5.1）。解放はクリックとして送られなかった（`mouse_click_not_sent reason="box_choice"`）。合う |
| 3 | `arrow1` を左クリック（00:27:46.65） | `evidence/t84-A3-after-arrow1-click.png` | 出た | 無し | 新しい側へ 2 行送られ自動の位置へ戻り、`arrow1` が消えた（要件 7.4・5.2）。解放はクリックにならなかった。合う |
| 4 | 箱の中でホイールを上へ 2 目盛り（00:27:47.67） | `evidence/t84-A4-after-wheel-up-over-box.png` | 出た | 出た | 1 目盛りに 3 行、古い側へ 6 行送られ 10〜15 行目が見えた（要件 7.5・6.1）。`OnMouseWheel` は送られなかった。合う |
| 5 | 箱の中でホイールを下へ 10 目盛り（00:27:48.72） | `evidence/t84-A5-after-wheel-down-back-to-auto.png` | 出た | 無し | 2 目盛りで自動の位置に追い付き、`arrow1` が消えた。残りの 8 目盛りも、押し出された行が残る箱の上なので箱が受け取り（`change=Unchanged`）、`OnMouseWheel` は送られなかった（要件 7.5）。合う |

段 B（対照）:

| 段 | 操作（UTC） | 絵 | 結果 |
|---|---|---|---|
| 1 | 20 行の台詞（話し終わり 00:28:04.58） | `evidence/t84-B1-overflow-again.png` | 段 A の 1 と同じく `arrow0` だけが出た |
| 2 | 箱の外のシェル（顔）でホイールを上へ 1 目盛り（00:28:05.82） | `evidence/t84-B2-after-wheel-outside-box.png` | 箱は動かず、`OnMouseWheel`（delta=120）が送られた（00:28:05.70）。箱に隠れた行があっても箱の外の回転は今までどおり（要件 7.5 の「その箱の上」の外）。合う |
| 3 | 1 行の短い台詞（話し終わり 00:28:07.33） | `evidence/t84-B3-short-no-hidden-lines.png` | 隠れた行が無く、矢印はどちらも出なかった（要件 3.3） |
| 4 | 箱の中でホイールを上へ 1 目盛り（00:28:08.10） | — | `OnMouseWheel`（delta=120）が送られた（00:28:07.97）。隠れた行の無い箱の上は今までどおり（要件 7.6）。合う |

- 走り全体で `OnMouseWheel` を送った行はこの 2 つだけ（段 A の箱の上の 12 目盛りは 1 つも送っていない）。
- 終わり方: MCP の `sakurascript` で `\0\-\e` を送り、`app_exit origin=KanadeStopped(Quit) closed=4`・終了コード 0。記録の ERROR・panic は 0 件（r1 も同じ）。
- 要件のとおりでなかった点は無い。

### 記録の行（r2）

箱の矢印の絵の解決（シェルのフォルダから・要件 7.3・9.1）と、文字の層が受け取った行（普通のバルーンの行は比べのため）:

```text
2026-10-31T00:27:08.222433Z  INFO areka_emo_present::shell_target: shell: 箱を読んだ（書かれた balloon.*ブレスの数・箱の置き場所を持つサーフェスの数） braces=1 surfaces=1
2026-10-31T00:27:08.420818Z DEBUG areka_emo_present::balloon_marker: balloon_marker: 印の絵を解いた scope=0 base="arrow" chain=arrowp0def>arrows>arrow up=<w>\target\bsa-verify-84\root\ghost\emo2\shell\master\arrow0.png 10x10 down=<w>\target\bsa-verify-84\root\ghost\emo2\shell\master\arrow1.png 10x10
2026-10-31T00:27:08.421426Z DEBUG areka_emo_present::balloon_marker: balloon_marker: 印の絵を解いた scope=1 base="arrow" chain=arrowp1def>arrowk>arrowp0def>arrows>arrow up=<w>\target\bsa-verify-84\root\ghost\emo2\shell\master\arrow0.png 10x10 down=<w>\target\bsa-verify-84\root\ghost\emo2\shell\master\arrow1.png 10x10
2026-10-31T00:27:08.436605Z DEBUG areka_emo_present::balloon_marker: balloon_marker: 印の絵を解いた scope=0 base="arrow" chain=arrowp0def>arrows>arrow up=<w>\target\bsa-verify-84\root\balloon\areka-balloon\arrow0.png 11x10 down=<w>\target\bsa-verify-84\root\balloon\areka-balloon\arrow1.png 11x10
2026-10-31T00:27:08.567126Z DEBUG actor{actor=emo-text}: areka_emo_text::actor::scroll: 箱の矢印の絵を受け取った boxes=2
```

- `boxes=2` はスコープの数（スコープ 0 と 1 がそれぞれ箱 `talk` の絵を持つ）。

段 A の手動の送り（要件 7.4・7.5・9.2・9.3）:

```text
2026-10-31T00:27:45.725396Z DEBUG actor{actor=emo-text}: areka_emo_text::actor::scroll: 手動の送りを始めた actor=0 place=Box(BoxName("talk")) via="矢印"
2026-10-31T00:27:45.725624Z TRACE actor{actor=emo-text}: areka::input_events::shell_box_handler: 箱の矢印の押下: 送りだけをしてクリックにしない event="box_arrow_pressed" scope=0 box="talk" kind=Up change=Started
2026-10-31T00:27:45.797802Z  INFO actor{actor=emo-text}: areka::input_events::click: 解放をクリックにしない event="mouse_click_not_sent" reason="box_choice" scope=0 button="0"
2026-10-31T00:27:46.658200Z DEBUG actor{actor=emo-text}: areka_emo_text::actor::scroll: 手動の送りを終えて自動へ戻した actor=0 place=Box(BoxName("talk")) reason=ReachedAuto via="矢印"
2026-10-31T00:27:46.658432Z TRACE actor{actor=emo-text}: areka::input_events::shell_box_handler: 箱の矢印の押下: 送りだけをしてクリックにしない event="box_arrow_pressed" scope=0 box="talk" kind=Down change=Stopped(ReachedAuto)
2026-10-31T00:27:46.741531Z  INFO actor{actor=emo-text}: areka::input_events::click: 解放をクリックにしない event="mouse_click_not_sent" reason="box_choice" scope=0 button="0"
2026-10-31T00:27:47.674708Z DEBUG actor{actor=emo-text}: areka_emo_text::actor::scroll: 手動の送りを始めた actor=0 place=Box(BoxName("talk")) via="ホイール"
2026-10-31T00:27:47.677373Z TRACE actor{actor=emo-text}: areka::input_events::char_watch: 隠れた行のある箱の上のホイール: 箱を送り OnMouseWheel にしない event="box_wheel_scrolled" scope=0 box="talk" delta=120 change=Started
2026-10-31T00:27:47.791350Z TRACE actor{actor=emo-text}: areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Box(BoxName("talk")) via="ホイール"
2026-10-31T00:27:47.794218Z TRACE actor{actor=emo-text}: areka::input_events::char_watch: 隠れた行のある箱の上のホイール: 箱を送り OnMouseWheel にしない event="box_wheel_scrolled" scope=0 box="talk" delta=120 change=Moved
2026-10-31T00:27:48.724954Z TRACE actor{actor=emo-text}: areka_emo_text::actor::scroll: 手動の位置を送った actor=0 place=Box(BoxName("talk")) via="ホイール"
2026-10-31T00:27:48.727221Z TRACE actor{actor=emo-text}: areka::input_events::char_watch: 隠れた行のある箱の上のホイール: 箱を送り OnMouseWheel にしない event="box_wheel_scrolled" scope=0 box="talk" delta=-120 change=Moved
2026-10-31T00:27:48.858231Z DEBUG actor{actor=emo-text}: areka_emo_text::actor::scroll: 手動の送りを終えて自動へ戻した actor=0 place=Box(BoxName("talk")) reason=ReachedAuto via="ホイール"
2026-10-31T00:27:48.861257Z TRACE actor{actor=emo-text}: areka::input_events::char_watch: 隠れた行のある箱の上のホイール: 箱を送り OnMouseWheel にしない event="box_wheel_scrolled" scope=0 box="talk" delta=-120 change=Stopped(ReachedAuto)
2026-10-31T00:27:48.974992Z TRACE actor{actor=emo-text}: areka::input_events::char_watch: 隠れた行のある箱の上のホイール: 箱を送り OnMouseWheel にしない event="box_wheel_scrolled" scope=0 box="talk" delta=-120 change=Unchanged
```

- 00:27:48.97 の後に同じ `change=Unchanged` の行が 00:27:49.85 まで 7 行続く（段 5 の残りの目盛り）。00:27:45.51 の `OnMouseEnterAll` から 00:27:50.63 の `OnMouseLeaveAll` の間に `OnMouseWheel` の行は無い。

段 B の対照（kanade が `OnMouseWheel` を送った行・要件 7.6）:

```text
2026-10-31T00:28:05.700117Z DEBUG actor{actor=kanade}: kanade: マウスのホイール・出入りを送る event="mouse_event_sent" id="OnMouseWheel" scope=0 delta=120
2026-10-31T00:28:07.974991Z DEBUG actor{actor=kanade}: kanade: マウスのホイール・出入りを送る event="mouse_event_sent" id="OnMouseWheel" scope=0 delta=120
```

- 1 行目は箱の外のシェル（顔）の回転、2 行目は隠れた行の無い箱の上の回転。どちらにも `box_wheel_scrolled` の行は出ていない。
