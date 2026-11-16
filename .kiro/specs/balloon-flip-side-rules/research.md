# Research & Gap Analysis: balloon-flip-side-rules

> 2026-11-16・`/kiro-validate-gap`（要件の段のギャップ分析）。基準は main `5b3a361c` の上のこのワークツリー（`7a9cb4cc`）。SSP と areka の MCP は打っていない（測りは開発者の許しが要る・要件 1）。この文書は分析と案だけを書き、決めは要件の話し合いに渡す。

## 1. 要約

- **件 1 の材料は 2 つの関門の両方にもうある**。起動の関門 `apply_balloon_limit` は全スコープの `ScopePlacement` の列を受け取っており、書き込みの関門 `apply_balloon_limit_gate` は `GhostWindows::scopes()`＋`char_window(s)`＋`window_rect_of` でほかのスコープのキャラ窓の矩形を読める。判定は `place_balloon_in_work_area` に段を 1 つ足すだけで、要件 2.4（自分の窓と重なる部分は数えない）も「重なりの矩形が自分のキャラ窓に収まり切るか」の 1 式で書ける。
- **ただし、4.3 ⑴ の形では件 1 は起きない（縦に重ならない）**。本体のバルーン 800×448 は y=352〜800、相方の窓は y=904〜1704。映した先（x=232〜1032）も押し込み（x=0〜800）も相方の窓と 1 ピクセルも重ならない。brief と要件 Open Questions 1 の「重なる」「36 ピクセル」は横の範囲だけで数えている。要件 5.3 の 1 件は「並べ替える（重ならない）」側の例になる。
- **起動の順番の穴**: 起動の並べ直し（相方を反対の側へ返す）は窓を作った後のフレームで `crates/areka/src/runtime/frame/drain_resnap.rs` が `move_window_to` で 1 スコープずつ動かす。本体のバルーンの判定は、相方が返される**前**の相方の矩形で下る。要件 2.7（ほかの窓が後から動いても置き直さない）のままだと、件 1 が本当に起きる形でも、並べ直しの後に重なりが残りうる。
- **件 2 の最大の落とし穴**: P5 が足す `ox` は descript の `balloon.offsetx` と、バルーンの `windowposition.x` の数値（SSP で「左右で反転しない素の画面の向き」と測った量）を**合流済みの 1 つの値**（`ScopeConfig.balloon_offset`）である。`ox` の符号をそのまま反転すると、emo2 の相方（`kero.balloon.alignment,right`・`windowposition.x,-190`）のバルーンが k=5/4 で 474px 動き、檻 `prepare_windowposition_adjust_is_identical_for_both_balloon_sides` が赤になる。descript の分だけを反転するために、合流の前の値を P5 まで運ぶ欄が要る。
- **ukadoc は要件 3 の読みを二重に裏づける**。`sakura.balloon.offsetx` は「右に出す時は左方向が＋」と書き、`windowposition.x` の項は「シェルのballoon.offsetxと違い、右に出す時も符号が反転しない」と対比している。`windowposition.x` の側は SSP の実測（反転しない）と合っている。`balloon.offsetx` の SSP の実測はまだ無い（研究の項目 R1）。

## 2. 今のコード（Current State）

### 2.1 並べ替えの判定と 2 つの関門

| 役 | 場所 | 今受け取るもの |
|---|---|---|
| 判定の純関数 | `crates/areka-placement/src/balloon_limit.rs` 157 行目 `place_balloon_in_work_area(pos, size, char_rect, area) -> BalloonPlace` | バルーンの位置と大きさ・自分のキャラ窓の矩形・作業領域だけ |
| 起動の関門 | 同 234 行目 `apply_balloon_limit(placements, snapshot)` → 245 行目 `apply_balloon_limit_one(p, snapshot)`（273 行目で判定を呼ぶ） | 1 スコープずつ `map` で写す。列の残りは見ない |
| 書き込みの関門 | `crates/areka-placement/src/follow/window_move.rs` 983 行目 `apply_balloon_limit_gate(world, window, proposed, size, route)`（1077 行目で判定を呼ぶ・呼び出しは 705 行目 `enqueue_window_set_pos`） | `World` を持つ。自分のスコープのキャラ窓だけを `GhostWindows::char_window(scope)` で引く |
| 記録 | 両関門の `info!`（`BALLOON_LIMIT_CLAMP_TAG`＝`[balloon-limit] Clamp`・`how = place.how()`） | 欄は scope・context・how・from・to・balloon_size・work_area（書き込みの関門は entity・route も） |

- `BalloonPlace` は `Fits`／`Flipped(PointPx)`／`Pushed(PointPx)` の 3 つ。`how()` の語は `fits`／`flipped`／`pushed`。
- 映しの式は 176〜182 行目の 1 文（`x' = char.left + char.right − (pos.x + size.w)`）。映した先が横に入らない・映しても x が変わらないときは押し込み（185 行目）。

### 2.1a ほかのスコープのキャラ窓の矩形は、各関門でもう手に入る（調査項目 (a)）

- **起動の関門**: `apply_balloon_limit` に来る `Vec<ScopePlacement>` が全スコープの `char_pos`・`char_size` を持つ。`map` の前に矩形の列を 1 度組めば、`apply_balloon_limit_one` へ「ほかのスコープの矩形」を渡せる。公開の関数 `apply_balloon_limit` の引数は変えなくてよい（呼び手は `crates/areka/src/main.rs` 791 行目と、`balloon_offset_persist_roundtrip_tests.rs`・`balloon_limit_gate_tests.rs`・`balloon_limit_flip_tests.rs`）。
  - 注意: この時点の矩形は**起動の並べ直しの前**の値（resolver の出力か記憶から戻した位置・採寸した窓寸）。相方を反対の側へ返すのはこの後のフレーム（2.3 節）。
- **書き込みの関門**: `GhostWindows::scopes()`（`crates/areka-placement/src/spawn.rs` 333 行目・昇順）・`char_window(s)`・同じファイルの `window_rect_of(world, e)`（1104 行目・位置か寸が未確定なら `None`＝要件 2.6 の「決まらない窓は数えない」にそのまま使える）。新しい資源や Component は要らない。
  - `window_rect_of` は可視かどうかを見ない（`WindowPos` だけを読む）。要件 2.8 の下の仮置き（隠れている窓も数える）とそのまま合う。数えないにするなら可視の読み方を足す必要がある（議題 3）。
  - 表示の前の仮の矩形（例: 4.3 の通しの `2012,330–2880,1704`＝868×1374）も「決まっている」として数えられる。

### 2.2 右に出すバルーンの式（P5・調査項目 (b)）

- `crates/areka-placement/src/resolver.rs` 362〜392 行目。`keyword_balloon_pos` が `None`（数値指定・未指定＝`Side`）の腕で、`on_right` なら `x.saturating_add(w)`（383 行目）、左なら `x − balloon_w`（385 行目）。どちらも 388 行目で `.saturating_add(ox)`。`ox` は `sc.balloon_offset.unwrap_or((0,0)).0`（354 行目）。
- `balloon_offset` の中身は 2 つの供給の和（`crates/areka-placement/src/lib.rs` 556 行目 `scale_then_merge`）:
  1. descript の `balloon.offsetx`（`config.rs` 262 行目の `cascade2`＝ゴーストとシェルの descript・`sakura.`／`kero.`／無印）を `apply_author_balloon_offset_scale`（`lib.rs` 507 行目）がシェル軸で換算した値
  2. バルーンの `windowposition.x` の数値を `windowposition::to_screen_adjust` がバルーン軸で換算し `apply_windowposition`（`windowposition.rs` 204 行目）で加えた値。**SSP の実測で「左右で反転しない素の画面の向き」と確定済み**（`windowposition.rs` 18〜24 行目・140〜175 行目の表: emo2 の相方 Right で wp.x=−190 → 画面の左へ 237）
- 合流した後は区別がつかない。P5 で `ox` の符号を側で変えると、2 の分まで反転する。
- 右に決まるのは `BalloonSide::Right` と、`BalloonSide::Auto`（`none`）でキャラ窓の中心が作業領域の中央より左のとき（380 行目）。`none` の側は resolver の中でしか決まらないので、descript の分の符号は**resolver の中で**決めるしかない（config の段で反転する案は `none` に効かない）。
- 同じ式を使う呼び手: 起動の準備（`lib.rs` 283 行目）・シェルの差し替え（`reseed.rs` 243 行目）・シェル位置初期化（`reset_pos.rs` 105 行目）。どれも `resolve_placement` を通るので、P5 を直せば 3 つとも揃う。ほかに右の側の式を書き写した場所は無い（`BalloonSide` を読むのは `config.rs`・`resolver.rs`・`lib.rs` 642 行目の記録だけ）。

#### 今の符号を固定している檻

| ファイル | テスト | 今の主張 | 要件 3 の後 |
|---|---|---|---|
| `crates/areka-placement/src/reseed_tests.rs` 64・178・201・240 行目 | `t_rs01_new_offset_and_alignment_reach_the_char_window_parts` ほか同じ `new_shell` を使う 3 本 | sakura（`balloon.alignment,right`・`offsetx,30`）の追従のずらし＝`CHAR0.w + 30` | `CHAR0.w − 30` へ変わる（期待値の書き換え 4 か所） |
| `crates/areka-placement/src/placement_windowposition_tests.rs` 200 行目 | `prepare_windowposition_adjust_is_identical_for_both_balloon_sides` | Right 側の `windowposition.x` の寄与＝+266（反転しない） | **変わってはいけない**（descript だけを反転する作りなら緑のまま。合流値ごと反転すると赤） |
| `crates/areka-placement/src/windowposition.rs` の `to_screen_adjust_is_side_independent_across_missing_axis_and_scale_matrix` | 調整量が置き側に依らない | 変わらない |
| `crates/areka-placement/src/balloon_offset_supply_tests.rs` 261 行目付近 | scope 1 は `windowposition` 由来だけ | 変わらない（descript の分が 0） |
| `crates/areka-placement/src/resolver_resolve_balloon_tests.rs` 65 行目 `t_r8_balloon_right_places_right_of_char_without_clamp` | 右・ずらし無し | 変わらない |
| 同 91 行目 `t_r8_balloon_offsetx_offsety_added` | **左**だけでずらしの加算を固定 | 変わらない。右のずらしを固定する檻は resolver の側に**無い**（要件 5.1 の 3.1 は新しく足す） |
| `crates/areka-placement/src/spawn_follow_pipeline_tests.rs` | scope1 右・ずらし無し | 変わらない |

- `crates/areka/src/` の下（本体・`runtime/` を含む）に、右に出すバルーンと 0 でない descript の横のずらしを組み合わせた檻は見つからなかった（`balloon.offsetx` を書くテストは `areka-placement` の中だけ）。`runtime-crate-split` との約束（`crates/areka/src/runtime/` に触らない）は件 2 では守れる見込み。

### 2.3 起動の並べ直しとバルーンの判定の順番

- 起動の関門は `restore_merged_placements`（`crates/areka/src/main.rs` 771〜799 行目）の中で、窓を作る**前**に 1 度だけ走る。
- 相方を反対の側へ返す鎖の寄せ（`ClampScene::BootRepack`・`ChainClampOutcome::Flipped`）は、最初の面が出た後のフレームで `crates/areka/src/runtime/frame/drain_resnap.rs` の `finalize_chain_once_with`（450 行目〜）が `place_chain_in_work_area` の結果を `move_window_to` で 1 スコープずつ書く（555 行目）。`move_window_to` はキャラ窓を書いてから自分のバルーンを随伴させる（`window_move.rs` 65〜85 行目）。ほかのスコープのバルーンは書き直さない。
- したがって本体のバルーンの判定（起動の関門、本体が最初の面で大きさを変えたときの随伴）は、相方が返される前の相方の矩形で下る。4.3 ⑴ の記録でも相方は `from={x:1340,y:904}` から `to={x:764,y:904}` へ返されている（`completed/window-work-area-clamp/tasks.md` 194 行目）。
- 後で本体のバルーンが書かれる（本体の面の切り替えに伴う `resize_window_to` の随伴・本体のドラッグ・`\![move]`）と、その時の相方の矩形で判定し直される（判定は毎回、生の相対の位置から出し直す＝状態を持たない）。いつ次の書き込みが来るかはゴーストの台本しだい（研究の項目 R3）。
- 「並べ直しの後に、動かされた窓のほかのスコープのバルーンを判定し直す」ことは、`drain_resnap.rs`（`runtime/` の下）に手を入れるか、`areka-placement` の側の `move_window_with_route`／`follow_balloon` で「ほかのスコープのバルーンも書き直す」道を足すかのどちらかになる。前者は同じウェーブの `runtime-crate-split` との約束に触れる。後者は要件 2.7 と食い違う（議題 A）。

### 2.4 4.3 ⑴ の形の幾何（計算だけ）

| 窓 | 矩形（x 範囲 × y 範囲） |
|---|---|
| 本体のキャラ窓 | 0〜764 × 610〜1704 |
| 相方のキャラ窓（返した後） | 764〜1436 × 904〜1704（672×800） |
| 本体のバルーン（元の位置） | −268〜532 × 352〜800 |
| 映した先 | 232〜1032 × 352〜800 |
| 押し込み | 0〜800 × 352〜800 |

- 映した先と相方の窓: 横は 764〜1032 で重なるが、縦はバルーンの下端 800 が相方の上端 904 より上＝**重ならない**。押し込みも同じく重ならない。
- 矩形の重なりで判定する限り、この形は要件 2.2（そのまま並べ替える）側に入る。要件 5.3 の 1 件は「直す前から緑」の例になり、要件 5.2（直す前に赤）の役は別の形（たとえば相方の窓の上端がバルーンの下端より上にある形）で立てる必要がある。
- 窓の矩形は絵の透明な部分を含む。絵の見た目で重なるかは矩形からは分からない（矩形の判定で足りるとするかは議題 C）。

### 2.5 要件 2.4（自分の窓と重なる相方の窓を数えない・調査項目 (d)）

- 半開区間の整数の矩形では、「映した先 B とほかの窓 O の重なりの矩形 I＝B∩O が空でなく、かつ I が自分のキャラ窓 C に収まり切らない」とき、C の外に必ず 1 ピクセル以上の重なりがある。逆も成り立つ。要件 2.4 は**矩形の交わり 1 回と内包の判定 1 回**で書ける（面積の引き算や多角形は要らない）。
- 配布版のぱすた（4.4 の形）: 本体と相方の窓が `0,336 1108×1368` で同じ位置。I は常に C に収まるので数えない＝今の並べ替え（`how="flipped"`）はそのまま。
- 交わりの判定は `crates/areka-placement/src/follow/visibility.rs` 120 行目に私有の `rects_intersect`（半開・接触は交差としない）と、111 行目に `rect_at`（`pub(super)`）がある。`pub(crate)` に開いて使えば新しい交わりの式は要らない。I を組む式（max/min の 4 つ）と内包の判定は新しく要る（数行）。

### 2.6 同梱の検体の数え（調査項目 (c)・要件 3.7 の下準備）

`vendors/sample_ghost/*.nar` を zip のまま読んだ（展開していない）。areka は surfaces.txt の surfaceブレスの `balloon.offsetx` と方向の 1 文字付きの鍵（`offsetxc` など）を読まないので、それらは数えない。

| 検体 | `balloon.alignment` | descript の横のずらし | 要件 3.1 で変わるか |
|---|---|---|---|
| emo2 | sakura left・kero **right** | 無し（バルーン `emo2-kakukaku` の `windowposition.x` が s0=266・k0=−190） | **descript の分だけを反転するなら 0**。合流値ごと反転すると kero が k=5/4 で 474px 動く |
| emily4 | none・none | sakura 7・kero 10（`char2.` は 40 だがスコープ 2 は作らない） | キャラ窓の中心が作業領域の中央より左で右に決まったときだけ、`2×ずらし×k` 動く |
| claudia | none・none（両方 free） | sakura 50・kero 70 | 同上（右に決まったとき 100／140 ×k） |
| konnoyayame | none・none（両方 free） | sakura 0・kero 10 | kero が右に決まったときだけ 20×k |
| dot_sakura（3 シェル） | 書かない（areka の既定は left） | あり | 変わらない（左） |
| R_POST_and_KOMAINU | 書かない（left） | あり | 変わらない（左） |
| areka-balloon・StayseeBalloon・emo2-kakukaku-offsetdpi／wplimit | バルーンだけ | — | `windowposition.x` は無いか、キーワード（`center`・`bottom`）でこの件の外 |
| 配布版のぱすた（リポジトリの外・brief と完了 spec の 4.4 の記録から） | sakura left（`offsetx,476`）・kero は 4.4 の記録で左に置かれ、ずらし 0 | — | 変わらない |

- 右に**固定**（`right`）で 0 でない descript の横のずらしを持つ同梱の検体は 0 本。変わるのは `none` の 3 本（emily4・claudia・konnoyayame）の、右に決まった場面だけ。
- 記憶にバルーンの相対の位置が残るのは、利用者がバルーンをドラッグして離したときだけ（`persist::balloon_offset_entries` の呼び手は `follow/drag_follow.rs` 789 行目の 1 か所）。要件 3.6 の「記憶の値をそのまま使う」は今の `persist::merge_scope`（529 行目〜・保存値は基準変換なしで採る）のままで満たされ、新しいコードは要らない。

### 2.7 ファイルの大きさ（調査項目 (e)）

| ファイル | 行数 | 余裕 |
|---|---|---|
| `crates/areka-placement/src/balloon_limit.rs` | 301 | 十分 |
| `crates/areka-placement/src/balloon_limit_flip_tests.rs` | 313 | 十分（件 1 の檻の置き場） |
| `crates/areka-placement/src/balloon_limit_gate_tests.rs` | 472 | 十分 |
| `crates/areka-placement/src/resolver.rs` | 559 | 十分 |
| `crates/areka-placement/src/resolver_resolve_balloon_tests.rs` | 223 | 十分（件 2 の檻の置き場） |
| `crates/areka-placement/src/reseed_tests.rs` | 527 | 期待値の書き換えだけ |
| `crates/areka-placement/src/follow/window_move.rs` | **1,228**（上限の例外表に載ったまま） | 足すのは呼び出しの引数と記録の欄の数行に抑え、矩形を集める本体は外へ置く |
| `crates/areka-placement/src/follow/visibility.rs` | 387 | `rects_intersect` を開くだけ |

## 3. 要件ごとの対応表（Requirement-to-Asset Map）

| 要件 | 今ある資産 | ギャップ |
|---|---|---|
| 1.1〜1.5 測り | 完了 spec の `research.md` 13 節の取り方（MCP `get_property`） | **Missing**（未実施・許しが要る）。areka 側の値は MCP の `areka` の `get_property` で同じ形を読める見込み（今回は打っていない） |
| 2.1 重なれば押し込み | `place_balloon_in_work_area` の並べ替えの腕・`limit_correction` | **Missing**: ほかの窓の矩形の引数・重なりの判定 1 段 |
| 2.2 重ならなければ今のまま | 同上 | 無し（今の腕） |
| 2.3 両関門で同じ規則 | 判定の純関数が 1 本（両関門が同じ関数を呼ぶ） | **Missing**: 起動の関門で列の矩形を集める・書き込みの関門で `GhostWindows` から集める。**Constraint**: 起動の並べ直しとの順番（2.3 節） |
| 2.4 自分の窓と重なる部分は数えない | `RectPx` | **Missing**: 交わりと内包の数行（2.5 節） |
| 2.5 押し込みの結果の重なりは許す | `limit_correction` | 無し |
| 2.6 決まらない窓は数えない | `window_rect_of`（`None`） | 起動の関門の列は常に決まっている。書き込みの関門は `None` を飛ばすだけ |
| 2.7 置く・書き込む時点だけ | 関門は状態を持たない | **Constraint**: 4.3 ⑴ の類の形では並べ直しの後に重なりが残りうる（議題 A） |
| 2.8 スコープ数を決め打ちしない | `GhostWindows::scopes()`・列 | 無し（全スコープを回せば満たす） |
| 3.1 右は「右端 − ずらし」 | P5 の 1 式 | **Missing**: descript の分と `windowposition.x` の分を P5 で分けて持つ欄（2.2 節） |
| 3.2 左は今のまま | P5 | 無し |
| 3.3 初めから右＝並べ替えて右 | 映しの式 | descript の分だけなら式の上で一致する（4 節）。**Unknown**: `windowposition.x` が 0 でないときは一致しない（議題 B） |
| 3.4 3 つの鍵に同じに当てる | `cascade2` | 無し（合流の前の値で反転すれば 3 つとも効く） |
| 3.5 縦・`windowposition.y`・キーワードを変えない | `keyword_balloon_pos` は別の腕 | 無し。ただし要件 3.5 は `windowposition.x` の**数値**に触れていない（議題 B） |
| 3.6 記憶はそのまま | `persist::merge_scope` | 無し（新しいコード不要） |
| 3.7 見た目の変わる場面を数える | 2.6 節 | 配布版のぱすたの kero の descript の値はリポジトリの外（**Unknown**・研究の項目 R4） |
| 4.1 押し込みに倒した理由の欄 | 両関門の `info!` | **Missing**: 重なったスコープを運ぶ値（`BalloonPlace::Pushed` に理由を持たせる等）と欄 1 つ |
| 4.2・4.3・4.4 | 今の語と `[balloon-limit] Unresolved` | 無し |
| 5.1〜5.3 決定論の檻 | `balloon_limit_flip_tests.rs`・`resolver_resolve_balloon_tests.rs` | **Missing**: 新しい檻。5.3 の形は直す前から緑（2.4 節） |
| 5.4 §8 の行 | `doc/COMPAT_ARCHITECTURE.md` 431 行目（【上書き】バルーンだけが作業領域からはみ出すとき） | 行の「測っていない」の置き換えは測りの後 |
| 6.1〜6.4 | — | 6.4 は「descript の分だけ反転」なら emo2・ぱすたとも 1 ピクセルも変わらない |

## 4. 式の確かめ（件 2・計算だけ）

記号: キャラ窓 L〜R、バルーンの幅 w、descript の横のずらし d、`windowposition.x` の数値 p（どちらも換算済みの物理 px）。

| 場面 | 今 | 要件 3.1 の後（d だけ反転） |
|---|---|---|
| 初めから左 | L − w + d + p | 同じ |
| 初めから右 | R + d + p | R − d + p |
| 左から映して右へ | L + R − (L − w + d + p + w) ＝ R − d − p | 同じ |
| 右から映して左へ（今の初めの右から） | L − w − d − p | L − w + d − p |

- p＝0 なら、要件 3.1 の後は「初めから右」と「左から映して右」が R − d で一致する（要件 3.3 を満たす）。今は 2d ずれる。
- p≠0 だと、要件 3.1 の後も 2p ずれる。映しの式は位置だけを見るので、p まで「シェルへの間合い」として映してしまう。`windowposition.x` は ukadoc でも SSP の実測でも「左右で反転しない」量なので、並べ替えた後の正しい位置は「初めからその側に出したとき」と同じ L − w + d + p（右から左）／ R − d + p（左から右）と読める。映しの式に p を戻すには、両関門が p を知る必要がある（`ScopePlacement` にも `BalloonFollow` にも今は無い）。
- emo2 の相方（Right・d=0・p=−237）が左へ並べ替わる場面では、今の映しは L − w + 237（シェルに 237 重ねる）、`windowposition.x` を素の向きで読むと L − w − 237（シェルから 237 離す）。この差は件 2 とは別の、映しの式の問題で、測っていない（研究の項目 R2）。

## 5. 案（Implementation Approach Options）

### 件 1

**案 A（今の部品を広げる）**
- `place_balloon_in_work_area` に引数 `others: &[RectPx]`（ほかのスコープのキャラ窓の矩形）を足し、並べ替えの腕（185 行目）の条件に「映した先がどれとも数える重なりを持たない」を足す。重なったら `Pushed`。
- `BalloonPlace::Pushed` に「重なったスコープ」（`Option<usize>`）を持たせ、`how()` は変えない（要件 4.2）。記録の欄は両関門で 1 行ずつ。
- 起動の関門: `apply_balloon_limit` で `map` の前にスコープと矩形の列を組み、`apply_balloon_limit_one` へ渡す（`balloon_limit.rs` の中で閉じる）。
- 書き込みの関門: `window_move.rs` で `GhostWindows::scopes()` を回して矩形を集める（数行）。
- ✅ 新しいファイル無し・規則は純関数 1 本のまま。❌ `window_move.rs`（1,228 行）が数行増える。`others` の組み立てが 2 か所に分かれる。

**案 B（矩形を集める道を新しく置く）**
- 案 A の判定に加え、書き込みの関門のための「`World` からほかのスコープのキャラ窓の矩形を集める」関数を `follow/` の新しい小さなファイル（または `follow/visibility.rs` の隣）へ置き、`window_move.rs` は呼ぶ 1 文と記録の欄 1 つだけにする。`window_rect_of` は同じ所へ移すか `pub(super)` に開く。
- `balloon_limit.rs` に置かない理由: このファイルは `bevy_ecs` を import しない約束（同 26〜31 行目の依存規約）。
- ✅ `window_move.rs` の増分が最小。❌ ファイルが 1 つ増える。`window_rect_of` の置き場を動かすなら `audit-placement-follow-residue`（`window_move.rs` の説明の直し）と触る所が近い。

**案 C（並べ直しの後の判定し直しも入れる・議題 A で要件 2.7 を改めるとき）**
- 案 A か B に加え、鎖の寄せでキャラ窓を動かしたとき、ほかのスコープのバルーンも判定し直す。置き場は `areka-placement` の側（`move_window_with_route` の後か `follow_balloon` の後）に限れば `runtime/` に触れずに済むが、「キャラ窓 1 つの書き込みで、ほかのバルーンまで書く」新しい道になり、書き込みの数とフレームの中の順番の檻が要る。
- ✅ 4.3 ⑴ の類の起動の形でも重なりが残らない。❌ 規模が S から M 寄りへ。要件 2.7 を改める。

### 件 2

**案 D（合流の前の descript の分を `ScopeConfig` に別に持つ）**
- `ScopeConfig` に descript の横のずらし（換算済み）を別の欄で持ち（`apply_author_balloon_offset_scale` が書く）、P5 の右の腕だけ `− 2×d`（または「基本位置 ± d ＋ p」へ組み直し）。`balloon_offset` の合流と、キーワードの腕（`keyword_balloon_pos`・頭上と足元は ukadoc でも右方向が＋）は今のまま。
- ✅ emo2 の `windowposition.x` の檻が緑のまま・`none` にも効く・シェルの差し替えとシェル位置初期化も揃う。❌ `ScopeConfig` の欄が 1 つ増え、`config.rs`・`lib.rs`・`resolver.rs` の 3 か所に触る。

**案 E（`windowposition.x` の分を別に持つ）**
- 逆に `windowposition.x` の調整量を `balloon_offset` へ合流させず別の欄で運び、P5 で「基本位置 ±（descript の分）＋ p」を組む。
- ✅ 映しの式に p を戻す（議題 B）ときも同じ欄を関門まで運べる。❌ `apply_windowposition` の「合流する」設計（`windowposition.rs` 186〜203 行目・design D1'）と、それを固定する檻（`apply_windowposition_*`・`balloon_offset_supply_tests.rs`）の意味が変わる。触る所が案 D より多い。

**案 F（測りで SSP が ukadoc と違うと分かったとき）**
- P5 は今のまま（画面の右向き）とし、映しの式（`window-work-area-clamp` 要件 5.4）の側を「descript の分を反転しない映し」に改める。映しは位置しか見ないので、こちらも d を関門まで運ぶ必要がある（案 E と同じ問題）。

## 6. 規模と危うさ

- **規模: S〜M**。件 1 は判定に段を 1 つ（案 A・B なら S）。件 2 は欄 1 つと P5 の 1 式（案 D なら S）。議題 A で並べ直しの後の判定し直し（案 C）か、議題 B で映しの式に p を戻すことになれば M。
- **危うさ: 中**。式そのものは単純だが、(1) 起動の並べ直しとの順番で件 1 の直しが効かない形がある（2.3 節）、(2) 合流済みの `balloon_offset` をそのまま反転すると SSP で測った `windowposition.x` の向きを壊す（2.2 節）、(3) SSP の測りがまだで、件 2 の向きが決まっていない。

## 7. 設計の段へ渡すもの

### 推す方向（決めではない）

- 件 1 は案 A か案 B（`window_move.rs` の増分で選ぶ）。交わりの判定は `visibility.rs` の `rects_intersect` を開いて使う。
- 件 2 は案 D（descript の分だけを P5 で反転し、`windowposition.x` は素の向きのまま）。

### 研究の項目（Research Needed）

- **R1**: SSP は descript の `balloon.offsetx` を右に出すとき反転するか（測り ⑵）。ukadoc は反転すると書き、`windowposition.x` の項でもそれと対比している。SSP の実測は `windowposition.x` の側（反転しない）だけ。
- **R2**: SSP は `windowposition.x` が 0 でないバルーンを反対の側へ並べ替えたとき、`windowposition.x` を素の向きで当てるか、シェルへの間合いとして映すか（emo2 の相方の形で測れる）。
- **R3**: 起動の並べ直しで相方が返された後、本体のバルーンが次に書かれる（判定し直される）のはいつか。面の切り替えの `resize_window_to` の随伴がどの条件で走るかを読む（大きさが変わらない面の切り替えでも随伴するか）。
- **R4**: 配布版のぱすた（`pasta-in-windows`・リポジトリの外）の `kero.balloon.alignment` と `kero.balloon.offsetx` の値。要件 3.7 の数えに要る（読むだけ・写すなら `target\` の下）。
- **R5**: 件 1 が実際に起きる形（縦にも重なる形）が、同梱の検体と配布版のぱすたの起動の形にあるか。無ければ件 1 は合成の形の檻だけで固定することになる。

### 付記（この spec の外の気づき）

- `crates/areka-placement/src/windowposition.rs` 155〜157 行目の注記は、ukadoc の `windowposition.x` を「シェル側が+、シェルから離れる側が-」と書くとして SSP との乖離を記録しているが、今の ukadoc（https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.x）は「バルーンを左右どちらに出していても常に右方向が+」と書き、SSP の実測と合っている。注記と §8 の該当の記述は古い可能性がある（要件 5.4 で §8 を書くときに合わせて確かめる）。

## 8. 要件の話し合いへの議題（ギャップ分析から出たもの）

- **議題 A（要件 2.7 と起動の順番）**: 起動の並べ直しは本体のバルーンを判定した後に相方を返す。要件 2.7 のままでは、件 1 が起きる形でも並べ直しの後に重なりが残りうる。⑴ 2.7 のまま（次にバルーンが書かれたときに直る） ⑵ 鎖の寄せでキャラ窓を動かしたときだけ、ほかのスコープのバルーンも判定し直す（案 C・`areka-placement` の側に置けば `runtime/` に触れない）。
- **議題 B（`windowposition.x` の数値）**: 要件 3.1 の「横のずらし」は descript の `balloon.offsetx` だけを指し、`windowposition.x` の数値は素の向きのまま、と要件に書き足すか。あわせて要件 3.3（初めから右＝並べ替えて右）を `windowposition.x` が 0 でないときにも求めるか（求めるなら映しの式に p を戻す＝M 寄り）。
- **議題 C（重なりの数え方）**: 判定はキャラ窓の矩形（透明な部分を含む）で足りるか。4.3 ⑴ の形は矩形では縦に重ならない。
- **議題 D（要件 5.3 の形）**: 4.3 ⑴ の形は直す前から「並べ替える」で緑になる。要件 5.2（直す前に赤）を満たす件 1 の檻は、縦にも重なる合成の形で立てる。5.3 の形は「重ならないので並べ替える」の例として残すか。
- **議題 E（Open Questions 1 の数字）**: 「押し込みも相方の窓に 36 ピクセル重なる」は横だけの数え。縦も入れると 4.3 ⑴ の押し込みは重ならない。⑵（押し込みへ倒す）を選ぶ理由づけを、縦に重なる形の数字で言い直すか。

## 9. SSP の測り（2026-11-16・要件 1・議題 8 の許し）

SSP 2.9.34（20261113-4）を `C:\wintools\ssp\ssp.exe` で起こし、MCP の `\![move,…,--base=me]` と `get_property` だけで測った。reload・インストール・設定の変更はしていない。測った後で窓を元の位置へ戻した。同じウェーブの 4 セッションへ測る前に知らせた。座標は SSP の `get_property` の値（左,上,右,下）。主の画面は `0,0,2880,1800`（作業領域の下端 1704）、左の画面は `-3413,260,0,2393`（どちらも 192 dpi）。

### 9.1 ぱすたさん（窓際のぱすたさんDEBUG・バルーン `pasta_tategaki`）では測れなかった

- 本体と相方の窓が同じ矩形 `-2568,819,-1737,1845` で、`\1\![move]` で相方だけを動かしても 2 つが一緒に動いた。相方を反対の側へ返した形（測り ⑴）は作れない。開発者「ぱすたさんは一般的なシェル構成ではない」で打ち切り、開発者が えも2DEBUG（バルーン `kakukaku for emo-gs`・`balloons0s.txt` の `windowposition.x,266`・`balloonk0s.txt` の `windowposition.x,-190`）へ切り替えた。
- 相方のバルーンは窓の右隣 `-1737,819,-1083,1494`（左端＝窓の右端）。窓の左に 845px の余地がある位置でも右に出ている＝これは並べ替えではなく相方の既定の側。2026-10-30 の `window-work-area-clamp` の測り（同 spec の `research.md` 13 節）が「SSP は反対の側へ並べ替える」と読んだ根拠も同じ形（窓が画面の左端・相方のバルーンが右隣）で、既定の側と並べ替えを見分けられていない疑いがある。

### 9.2 えも2DEBUG: `\![move]` で窓を端へ寄せても、SSP はバルーンを並べ替えも押し込みもしない

| 手順 | キャラ窓 | バルーン | はみ出し |
|---|---|---|---|
| 初めの位置 | 本体 `-2234,1248,-1852,1795`・相方 `-2570,1395,-2234,1795` | 本体 `-2337,1091,-1937,1315`・相方 `-2439,1279,-2151,1482` | なし |
| 本体を左の画面の左端へ（`\0\![move]` の後に本体が話す） | 本体 `-3413,1248,-3031,1795` | 本体 `-3516,1091,-3116,1315` | 左へ 103px（左の画面の左端 −3413） |
| 相方を主の画面の右端へ（`\1\![move]` の後に相方が話す） | 相方 `2524,1395,2860,1795` | 相方 `2655,1279,2943,1482` | 右へ 63px（主の画面の右端 2880） |

- 主の画面の右端では、画面を撮って（`CopyFromScreen`・スクラッチパッドの `ssp-right-edge.png`）、相方のバルーンが画面の端で切れて出ているのを目で確かめた。左端へ寄せた本体は、撮った画像に写らなかった（左の画面は倍率の違う画面との座標の対応を読み切れていない）。
- バルーンの既定の `windowposition.limit` は 1（ukadoc「バルーンを強制的に画面内に維持するかどうか。1で維持」・https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.limit）。`kakukaku for emo-gs` の descript.txt に `windowposition.limit` は無い。それでも SSP 2.9.34 は `\![move]` の後のバルーンを画面の中へ戻さなかった。
- ドラッグ（開発者の許し・排他テストの机を取り、兄弟のセッションへ知らせ、`SendInput` 相当の `mouse_event` で相方の窓を 1 回ドラッグ。押す前にマウスの下が SSP の `Tkeroform` であることを確かめた）: 相方を主の画面の中ほど `2000,1395` から右へ 524px ドラッグ → 窓 `2524,1304,2860,1704`（SSP が窓の下端を作業領域の下端 1704 へ寄せた）。その後に相方が話す → バルーン `2655,1188,2943,1391`＝**右へ 63px はみ出したまま**。画面を撮って（`ssp-drag-right.png`）端で切れているのを目で確かめた。ドラッグでも、SSP はバルーンを並べ替えも押し込みもしない。
- 戻し: 本体は `-2234,1248` へ戻した。相方は x を `-2570` へ戻したが、y は `\![move]` の縦の量が効かず `1304,…,1704`（初めは `1395,…,1795`）のまま＝相方の縦の位置が 91px 上がっている。

### 9.3 この測りで分かったこと・分からないこと

- 測り ⑴（並べ替えた先が相方の窓に重なるときの SSP の扱い）と測り R2（`windowposition.x` のあるバルーンを並べ替えたときの位置）は、**`\![move]` でもドラッグでも SSP が並べ替えをしないので測れなかった**。
- SSP 2.9.34 は、キャラ窓が作業領域に収まりバルーンだけが横にはみ出す形（はみ出しは 63px と 103px）で、バルーンを並べ替えも押し込みもしなかった。はみ出しの大きい形（バルーンの大半が外へ出る形）と、起動の直後・面の切り替えの場面は測っていない。9.1 のとおり、10-30 の「SSP は並べ替える」という読みは、既定の側と見分けられていない疑いがある。

## 10. 設計の段の調べ（2026-11-16・`/kiro-spec-design -y`）

> 発見の型は **light**（既存の配置の拡張）。新しい外部の crate・API は無いので外の調べはしていない（ukadoc の 2 項目は要件と 1 節で引用済み）。読んだ規則は kiro-spec-design の `design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`・`design-principles.md` と、steering の `structure.md`（兄弟のテスト・1,000 行・ファサード）・`logging.md`。下請けは出さず、コードは本文の中で読んだ。

### 10.1 調べて分かったこと

- **起動の並べ直しの書き込みの経路**: `drain_resnap.rs` の `finalize_chain_once_with` は鎖の寄せの結果を `move_window_to` で書く。`move_window_to` は `move_window_with_route(.., PlacementRoute::MoveCue)` で、台本の `\![move]` と同じ経路の語を名乗る。経路では並べ直しと台本を見分けられない。`place_chain_in_work_area` は `&World` を受けるだけで、動いた窓を World に残さない。
- **並べ直しの終わりの印**: `ChainFinalized`（`crates/areka-placement/src/chain_finalize.rs` 66 行目・資源）は、並べ直しの書き込みの**後**に `drain_resnap.rs` 593 行目で入り、終了のときに `crates/areka/src/app_exit.rs` 203 行目で外れる。`resource_added::<ChainFinalized>` を引き金にすれば、`runtime/` に触れずに「並べ直しの後に 1 度」を取れる。
- **同じ巡で流れるか**: `frame_system` は `Update`、`EcsWorld::try_tick_world` は `Update` → … → `FrameFinalize` の順に回し、積んだ窓の書き込みは巡の後に `tick_bridge` の `flush_window_pos_commands` がまとめて流す。`enqueue_window_set_pos` は `WindowPos` を即座に写す。`FrameFinalize` に置いた系は並べ直しと同じ巡で、動いた後の矩形を読んで書ける（1 フレーム遅らせない）。
- **系の登録の先例**: `areka_placement::balloon_syncscale::register(world)` を `crates/areka/src/ghost_session.rs` の `register_systems`（`runtime/` の外）が呼ぶ形がある。同じ形で 1 行足せる。
- **キャラ窓の「表示している」**: `areka-placement` にはキャラ窓の表示の有無の Component が無い。`\s[-1]` は `areka-emo-present` の `VisualMount::set_visible(false)` が、窓の子（surface entity と文字の層の枠）の `wintf` の `Visual.is_visible` を倒す。窓の `WS_VISIBLE` は変えない。キャラ窓の子の `Visual` を読めば、`areka-placement` から表示の有無が分かる。
- **`ScopeConfig` を全欄で組む所**: `config.rs` の `resolve_scope`（欄が増えたらコンパイルで赤になるように `..` を使わない・意図された形）と `resolver_balloon_keyword_tests.rs` の 2 か所。ほかは `..ScopeConfig::default()`。
- **行数の番人**: `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表は「超えている」事実だけを持ち、行数の上限を持たない。`window_move.rs` に数行足しても番人は赤にならない。`follow/drag_follow.rs` はちょうど 1,000 行なので触らない。
- **`follow/` の兄弟の相互参照**: ファサード `follow.rs` を経由する（`use self::window_move::…` の私有の再束縛）。`window_rect_of` は `window_move.rs` の私有なので、`pub(super)` に開いてファサードで再束縛する。

### 10.2 合成（design-synthesis）

- **一般化**: 2.4 の規則（重なりの矩形が自分のキャラ窓に収まり切るなら数えない）は自分の窓との重なりを必ず外すので、判定に渡す列から自分のスコープを外す必要が無い。両関門とも「全スコープのキャラ窓の矩形」を渡すだけになり、2.8（スコープ数を決め打ちしない）もそのまま満たす。
- **作るか採るか**: 交わりの判定は `follow/visibility.rs` の `rects_intersect`（真偽だけ）では足りず、交わりの矩形そのものが要る。`balloon_limit.rs` に `max`／`min` の数行で置く（`visibility.rs` は開かない）。系の引き金は bevy の `resource_added` を採る（自前の印を作らない）。
- **削ったもの**: 新しい経路の語（`PlacementRoute` の変種）・新しい記録の行・動いた窓を覚える資源・`rects_intersect` の公開・`place_chain_in_work_area` の署名の変更は、どれも作らない。判定し直しは「全スコープのバルーンを今の位置で随伴させ直す」1 本にし、動いた窓だけに絞る仕組みを持たない（判定は状態を持たないので結果は同じ）。

### 10.3 決めたこと（design.md の正本）

| 件 | 決め | 理由 |
|---|---|---|
| 件 1 の核 | `place_balloon_in_work_area` に `chars: &[(usize, RectPx)]`・`BalloonPlace::PushedOverlap { to, scope }`（語は `pushed`） | 既存の分岐を 1 ビットも変えず段を 1 つ足す（案 A）。既存の檻の `Pushed(..)` の主張が変わらない |
| 書き込みの関門の列 | 新設の `follow/other_chars.rs` の `shown_char_rects` | `window_move.rs`（例外表）の増分を数行に抑える（案 B） |
| 2.10 | `FrameFinalize` の系・`run_if(resource_added::<ChainFinalized>)`・全スコープを `follow_balloon(.., Placement(MoveCue))` | `runtime/` に触れない・経路の語を増やさない・同じ巡で流れる |
| 件 2 | `ScopeConfig.balloon_offset_descript_x`・P5 の右の腕だけ `ox − 2d` | 合流済みの値から descript の分だけを分ける（案 D）。`windowposition.x` の檻が緑のまま |
| 記録 | `[balloon-limit] Clamp` に欄 `overlap_scope`（`Option<usize>`） | 語を増やさない（4.2） |

### 10.4 危うさ

- 判定し直しの系は、窓が動いていない巡でも全スコープのバルーンを書き直す（同じ位置の `SetWindowPos` が 1 巡に数本増える）。目に見える動きは無い見込み。
- 「表示している」を子の `Visual` で読むのは、`areka-emo-present` の装着の形（描画面を窓の子に置く）に寄りかかる。装着の形が変われば読み方も直す必要がある（`extra-character-windows` も同じ装着を使う見込み）。
- `crates/areka/src/ghost_session.rs` に 1 行足すので、`runtime-crate-split` が同じファイルを動かすなら衝突しうる（1 行なので解くのは易しい）。

### 10.5 要件 3.7 の数えの残り

- 2.6 節の表のとおり、右に固定で descript の横のずらしが 0 でない同梱の検体は 0 本。変わるのは `none` の emily4・claudia・konnoyayame が右に決まった場面だけで、emo2（descript の横のずらし無し）は 1 ピクセルも変わらない。
- 配布版のぱすたの kero の `balloon.alignment`・`balloon.offsetx`（R4・リポジトリの外）は、実装のタスクで読んで（写すなら `target\` の下）この節に書き足す。brief と完了 spec 4.4 の記録では kero は左に置かれ、ずらしは 0 なので、変わらない見込み。

## 11. 設計の話し合いの決めごとの根拠（2026-11-16）

- 開発者が設計の全議題（5 件）を推奨案で一括承認した。中身は `design.md` の「決めごと」D1〜D5。
- D1（判定し直しを 3 つの場面に広げる）の根拠は `design-validation.md` の重大な指摘 1。`reset_pos.rs` の `reset_window_positions` と `chain_realign.rs` の `realign_chain_once_with` は、起動の並べ直しと同じく `move_window_to` で窓を 1 つずつ書き、自分のバルーンだけを付いて来させる。
- D5（`windowposition.x` の注記）: ukadoc の文言は ukatech/ukadoc の f92e4ff9（2026-09-20）で「数値指定の場合シェル側が+、シェルから離れる側が-」から「バルーンを左右どちらに出していても常に右方向が+。シェルのballoon.offsetxと違い、右に出す時も符号が反転しない」に改まった（`gh api repos/ukatech/ukadoc/commits/f92e4ff9` の `manual/descript_balloon.html` の差分で確かめた）。2026-07-31 の SSP の実測（`windowposition.rs` の表・COMPAT §8 166 行目）と一致するので、この項目に ukadoc と SSP の食い違いは残っていない。
