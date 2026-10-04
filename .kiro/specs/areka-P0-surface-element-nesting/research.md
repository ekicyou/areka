# ギャップ分析: areka-P0-surface-element-nesting

- 実施日: 2026-10-04
- 測った位置: ブランチ `claude/areka-p0-surface-nesting-d7ca3c`（`bb163c41`・main `e2a373b5` の上）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`structure.md`・`tech.md`）
- 方法: 既存コードを Grep／Read で実測。外部の依存の調査は不要だった（新しいクレートを要さない）。
- 本書は選択肢と材料を並べる。決めるのは要件ディスカッションと設計。

## 1. 要約

- **再帰の骨組みは在るが、element定義からは入っていない**。`areka-emo-compose` の `plan.rs` の `flatten_surface` は pattern定義が指すサーフェスへ再帰し、位置を足し合わせ、先祖の積み上げで循環を止める。element定義の側（同ファイルの `push_static_element_ops`）は画像を 1 枚ずつ命令にするだけで、再帰しない。
- **`shell-balloon` は画像の側の型に触っていない**。箱は 2 つ目の転記（`areka-parsers` の `parse_boxes`）と `areka-emo-compose` の `fold_boxes` だけで完結しており、描画メソッド `balloon` の行は画像の読み手（`decode_elements`）に届かない。要件 1.4（`balloon` の行は数字でも箱の名前）は**今の構造のまま満たされる**。足りないのは要件 6.1 の警告 1 種だけ。
- **数字だけの判定を置ける場所は、依存の向きで絞られる**。`areka-emo-atlas` は `areka-emo-compose` の上流なので、「判定は compose の側」という同じウェーブの約束を守ると atlas の `manifest.rs` からは判定を呼べない。代わりに、焼く前の入口（`areka-emo-present` の `build_shell_target_with_boxes`）で数字だけの element を外せば `manifest.rs` に触らずに済む（brief の「`manifest.rs` には必ず触る」は必須ではない）。
- **一番重いのは子の時計**。`areka-seriko` の `LoopRuntime` は「表示中の (スコープ, 面の種類)」だけを回し、面の切り替えで再生を全部捨て、切り替えの `Show` は空の `PatternState` を載せる。子の時計を持ち越すには、時計の置き場所・切り替えの `Show` への載せ方・見えない間の扱い・乱数の消費順の 4 点を設計で決める必要がある。
- **規模 M〜L・リスク中**。静的な入れ子（合成・外形・当たり判定・警告）は既存の型の延長で済む。子の時計は新しい状態を足すので、入れ子の無いシェルの乱数の消費順と合成の回数を変えないこと（要件 7）の檻が要る。

## 2. 今の実装（要件に関わる所だけ）

### 2.1 転記（`crates/areka-parsers/src/shell/`）

| 何 | 今の振る舞い |
|---|---|
| `model.rs` の `Element`（`layer`・`path: ElementPath`・`x`・`y`） | 描画メソッドの欄を持たない。`ElementPath` は文字列を包むだけの型（`new`・`as_str`）。 |
| `decode.rs` の `decode_elements` | 欄 1 が `overlay` の行だけを `Element` にする。欄 2 は無加工で `ElementPath` へ。`surface*`ブレスと `surface.append*`ブレスの両方から呼ばれる（要件 1.1・1.2 の入口は同じ 1 関数）。 |
| `boxes.rs` の `parse_boxes`／`surface_lines` | 欄 1 が `balloon` の行だけを `BoxElementLine` にする。画像の読み手とは別の転記。 |

→ 欄 2 が数字だけの `overlay` 行は、今は「`100` という名前の画像」として `Element` に入る。**転記は今のままで情報が足りている**（数字かどうかは `ElementPath::as_str()` から下流で読める）。

### 2.2 焼く（`crates/areka-emo-atlas/src/manifest.rs`・`crates/areka-emo-present/src/shell_target.rs`）

- `ManifestDeriver::derive` は、渡された全サーフェスの element の道を `collect_elements` で鍵に入れる。子のサーフェスの画像は、子がシェルに在る限りもともと集まる（たどり直す必要は無い）。
- 数字だけの element は今、`100` を画像の鍵に入れ、焼く段が読み込みの失敗を記録する（要件 3.5 に反する今の振る舞い・brief「見つけた穴」と一致）。
- 焼く入口は `shell_target.rs` の `build_shell_target_with_boxes`。ここで `shell.surfaces` を複製し、土台に使う面の画像を足して `SurfaceSet` を作る。**atlas へ渡す前に手を入れられる唯一の場所**。
- `SurfaceSet` を本番で作る所は、ほかにバルーン（`areka-emo-present` の `balloon.rs`）だけ。バルーンの面は descript から組むので数字だけの element を持たない。

### 2.3 畳み込みと束縛（`crates/areka-emo-compose/src/`）

- `fold.rs` の `normalize_element` が `Element` を `NormalizedElement`（`layer`・`path`・`transform`・`method`）へ写す。`NormalizedElement` を構造体リテラルで作るのは `fold.rs`・`base_image.rs`・`normalized.rs` のテストの 3 ファイルだけ。
- `atlas_bind.rs` の `bind_atlas` は全 element を `AtlasTable::resolve` で引き、引けなければ `warn!`（「atlas 未束縛 element」）を出す。**面の表を組むたび（スコープの数だけ）出る**。数字だけの element はここでも毎回警告になる（要件 3.5）。
- `base_image.rs` の `apply_base_images` は「層 0 の element が在れば面の画像を使わない」。数字だけの `element0` が `SurfaceMaster.elements` に残っていれば、**要件 2.4（`surface*.png` を破棄）は今の判定のまま成り立つ**。
- 同じく `apply_base_images` は、ブレスの無い番号でも面の画像があれば面を新設する。子を `EmoWorld::surface(番号)` で引く限り、**要件 1.8（ファイル名の慣習だけで建つサーフェスを子に置く）は追加の仕組み無しで成り立つ**。
- `world.rs` の `EmoWorld::dangling_pattern_targets` が「相手の面が無いコマ」を面の表から 1 度だけ集め、記録は `load_shell_target` が読み込み 1 回につき 1 度出す。**無い番号・循環の警告（要件 3.1・3.2）の前例**。

### 2.4 合成の計画と外形（`plan.rs`）

- `push_static_element_ops`: element を層の昇順に並べ、束縛済みの画像だけを `BlitOp` にする。サーフェスを指す element を知らない。
- `flatten_surface`: 層 (i) で上の関数を呼び、層 (ii) で着せ替えとコマを animation の番号順に積む。コマと着せ替えの pattern0 が指すサーフェスへは再帰する（`is_top_level=false`）。再帰の先では `PatternState` を見ない＝止まった絵。**着せ替えの集合（`binds`）は全段で同じものを渡している**（要件 5.12 の「子へ持ち込まない」とは逆の今の振る舞い。pattern定義の側は今のまま、element定義の側だけ空にする必要がある）。
- 循環の `warn!` は `flatten_surface` と `flatten_extent` の中にあり、**合成のたびに出る**（外形と命令で 2 回）。
- `compute_extent`／`flatten_extent`: 全 element の「位置＋原寸」と、全着せ替えの pattern0 の先を足した静的な外形。`PatternState` に依らない。子の範囲を足す再帰をここにも入れれば、要件 2.5・2.6 は同じ規則で満たせる。

### 2.5 当たり判定（`hit.rs`・`crates/areka-emo-present/src/presenter/hit.rs`）

- `hit_region` は `SurfaceMaster.collisions` を逆順に走査する（後が手前）。受け取るのは一番上のサーフェスの `SurfaceMaster` 1 つだけ。
- 本番の入口は `EmoPresenter::hit_region_client` → `hit_region_scaled`。拡大率は呼ぶ前に割るので、子の領域を一番上のサーフェスの座標へ直しておけば要件 2.8 は追加の仕事無しで付いてくる。
- `SurfaceMaster.collisions` を読むのは `hit.rs` と `fold.rs` だけ。`SurfaceMaster` を構造体リテラルで作るのは `fold.rs`・`base_image.rs`・`hit.rs` と `normalized.rs` のテストの 4 ファイル。
- pattern定義が指すサーフェスの領域は今も持ち込まれていない（要件 4.7 は今のまま）。

### 2.6 箱（`boxes.rs`・`shell_target.rs`・`crates/areka-emo-text/src/actor_box.rs`）

- `BoxLayout` はサーフェス番号ごとの置き場所の表。読むのは `actor_box.rs`（今の一番上のサーフェスの番号で `placements` を引く）と `shell_target.rs` だけ。**子のサーフェスの箱は今も親の中に置かれない**（要件 6.1 の後半・6.3・6.4 は構造のまま成り立つ）。
- 足りないのは要件 6.1 の警告。`fold_boxes` は面の表（`EmoWorld`）を引数に持つので、「親 P の element が子 C を指し、C に置き場所がある」を数えられる。報告は `BoxIssue` に 1 種足し、記録は `shell_target.rs` の `log_box_issue` に 1 本足す（`BoxIssue` の `match` は網羅なので、足し忘れはコンパイルで止まる）。
- `boxes.rs` の `place` は「画像の element の最大の番号より小さい番号の箱」を報告する。その「画像の element」は `SurfaceMaster.elements` 全体なので、サーフェスを指す element も数に入る（絵として描かれるので筋は通る。設計で一言確かめる）。

### 2.7 seriko（`crates/areka-seriko/src/`）

| 何 | 今の振る舞い | 要件との差 |
|---|---|---|
| `table.rs` の `AnimationTable::from_world` | サーフェス番号 → 動かす animation の列。`SurfaceMaster.animations` だけを読む。採るのは `random`・`bind+random`・`sometimes`・`rarely`。 | 「このサーフェスを表示すると、どの子が見えるか」の表が無い（要件 5.1・5.2）。seriko は実行中に面の表を見ないので、構築時に写す必要がある。 |
| `looper.rs` の `LoopRuntime::on_tick` | `ScopeStates::shown_slots` の (スコープ, 面の種類, 番号) だけを回す。再生は `(スコープ, 面の種類)` → animation の番号 → 開始時刻。 | 子の再生を持つ場所が無い。鍵は「スコープ × 子の番号」（要件 5.3）。 |
| `looper.rs` の `LoopRuntime::on_surface_changed`・`actor.rs` のシェル面切替の分岐 | 面が替わると、その (スコープ, シェル) の再生を全部捨てる。 | 一番上の分は今のまま（要件 5.9）。子の分は捨てない（要件 5.6・5.7）。 |
| `state.rs` の `ScopeStates::apply` | 切り替えの `DisplayCommand::Show` に空の `PatternState` を載せ、保持していた `PatternState` を消す。 | 子のコマを載せないと、切り替えた 1 コマだけ子が土台の絵へ戻って見える（要件 5.6）。 |
| `state.rs` の `ScopeStates::commit_pattern` | 前と同じ `PatternState` なら何も出さない。 | 見えない子の進みを `PatternState` に入れなければ、要件 5.11（描き直さない）はこの番人で満たせる。 |
| `looper.rs` の `LoopRuntime::replace_shell_table`・`state.rs` の `ScopeStates::rebase_shell` | シェルの差し替えで、シェル側の再生とコマを全スコープ分捨てる。 | 子の時計を捨てる所（要件 5.8）。ゴーストが降りるときはアクターごと消える。 |
| 乱数の消費順（`looper.rs` 冒頭の「抽選の固定消費順」） | スコープの昇順 → シェル → バルーン → animation の番号の昇順。 | 子の分をどこへ挟むかが未定。入れ子の無いシェルでは今の順のまま（要件 7.3）。 |

### 2.8 合成のキャッシュ（`crates/areka-emo-present/src/cache.rs`）

- 鍵 `ComposeKey` は `surface_id`・`binds`・`pattern: PatternState`。**子のコマを `PatternState` の中へ入れれば `cache.rs` は変えずに済む**（等しさは派生）。
- 容量は 3（開発者裁定・emo2 のまばたきが 3 コマだから）。親のアニメと子のアニメが同時に動くシェルでは鍵の組み合わせが増え、命中率が下がりうる（入れ子の無いシェルには影響しない）。

## 3. 要件 → 既存資産の対応

凡例: **在**＝今の実装のまま満たす／**延**＝既存の関数を延ばす／**無**＝新しく作る／**未**＝設計で決める

| 要件 | 対応する資産 | 状態 |
|---|---|---|
| 1.1・1.2 数字だけ→番号 | `decode_elements`（転記は足りている）・`fold.rs` の `normalize_element` | 延（読み分けを 1 か所に足す） |
| 1.3 それ以外は画像 | 同上 | 在 |
| 1.4 `balloon` は名前 | `parse_boxes`／`decode_elements` が行を分けている | 在 |
| 1.5 数字だけの名前の画像を使わない | 焼く前に外す（`build_shell_target_with_boxes`）か `manifest.rs` | 延 |
| 1.6・1.7 上限なし・使い回し | `flatten_surface` の先祖の積み上げ（枝を出るときに外す） | 在（再帰に入れば） |
| 1.8 ファイル名の慣習だけの子 | `apply_base_images` が面を新設 | 在 |
| 2.1〜2.3 位置・重ね順・多段 | `push_static_element_ops`・`flatten_surface` | 延（層の順の途中で再帰） |
| 2.4 `element0` が子 | `apply_base_images` の層 0 の判定 | 在 |
| 2.5・2.6 外形 | `flatten_extent` | 延 |
| 2.7 pattern定義の先の子は止まった絵 | `flatten_surface`（`is_top_level=false`） | 延 |
| 2.8 拡大率 | `hit_region_scaled`・wintf の変換 | 在 |
| 3.1 無い番号の警告 | 前例 `dangling_pattern_targets` | 無（同じ型で作る） |
| 3.2〜3.4 循環 | `flatten_surface` の `visited` | 延（警告の出し方は未） |
| 3.5 失敗を記録しない | `collect_elements`・`bind_atlas` | 延（2 か所で外す） |
| 4.1〜4.6 領域の持ち込み | `hit_region`・`SurfaceMaster.collisions` | 無（持ち方は未） |
| 4.7 pattern定義の先 | 今も持ち込まない | 在 |
| 5.1・5.2 子のアニメ | `AnimationTable`・`LoopRuntime::on_tick`・`flatten_surface` | 無 |
| 5.3〜5.8 子の時計 | `LoopRuntime` の再生の表・`ScopeStates` | 無（置き場所は未） |
| 5.9・5.10 一番上は今までどおり | `on_surface_changed` | 在（別の表に持てば） |
| 5.11 見えない子で描き直さない | `commit_pattern` の同値の番人 | 在（見える分だけ載せれば） |
| 5.12 着せ替えを持ち込まない | `flatten_surface` が全段へ同じ `binds` を渡す | 延（element の再帰だけ空に） |
| 5.13 pattern定義の先は動かさない | `is_top_level=false` | 在 |
| 6.1 子の箱の警告 | `fold_boxes`・`BoxIssue`・`log_box_issue` | 無（1 種足す） |
| 6.2〜6.4 | `BoxLayout` は番号ごと・`actor_box.rs` は一番上の番号で引く | 在 |
| 7.1〜7.4 入れ子なしは不変 | 既存の決定論テスト・golden（`golden_tests*.rs`・`emo2_golden.rs`）・seriko の乱数順のテスト | 在（檻は既存。新しい分岐が空のとき素通しになることを足す） |
| 8.1 黙った読み飛ばしなし | 上の 3.1・3.2・6.1 | 無 |
| 8.2 検体 | 前例 `crates/areka-emo-text/tests/fixtures/shell-balloon/surfaces.txt` | 無 |
| 8.3 決定論テスト | `MemoryDecoder`・`MockSurfaceOutput`・注入乱数（`LoopRng`）・`send_tick(now_ms)` | 在（道具は揃っている） |
| 8.4 登記 | `doc/COMPAT_ARCHITECTURE.md`「8. 沈黙ルール対応表」 | 無（行を足す） |

## 4. 実装の進め方の選択肢

### 4.1 数字だけの読み分けをどこに置くか

| 案 | 中身 | 利点 | 欠点 |
|---|---|---|---|
| **A: 畳み込みで読む** | `fold.rs` の `normalize_element` が `NormalizedElement` に「画像か、サーフェスの番号か」を持たせる。判定の関数は compose が公開し、`build_shell_target_with_boxes` が焼く前の複製から数字だけの element を外す。 | `model.rs`・`decode.rs`・`manifest.rs` に触らない（同じウェーブの `mcp-expression-table`・`animated-image-decode` と当たらない）。`NormalizedElement` のリテラルは 3 ファイル。「parser は転記・解決は下流」に合う。 | 焼く前に外す所と畳み込みの 2 か所が同じ判定を呼ぶ（関数は 1 つ）。atlas を直に呼ぶ他の利用者が現れたら同じ外し方が要る。 |
| B: 転記の型で読む | `ElementPath` に `surface_ref()` を足す（`model.rs`）。atlas の `collect_elements` と compose の両方がそれを呼ぶ。 | 判定が最上流の 1 か所。atlas 単体でも正しい。 | `model.rs` と `manifest.rs` に触る（ウェーブの約束「判定は compose の側」「`manifest.rs` は見張る継ぎ目」に反する）。 |
| C: `Element` を列挙にする | 画像・サーフェス・箱を 1 つの列挙へ。 | 型で読み分けが見える。 | `ElementPath::new` を書く 29 ファイルへ波及。`shell-balloon` の「箱の無いシェルは前と同じ」の構造の保証を崩す。brief の棚卸㉑が既に退けている。 |

### 4.2 当たり判定の持ち方

| 案 | 中身 | 利点 | 欠点 |
|---|---|---|---|
| **A: 構築時に並べた列を別に持つ** | 面の表を組んだ後、サーフェスごとに「子の領域（element の番号の昇順・位置を足した写し）→ 自分の領域」の順の列を作り、`SurfaceMaster` とは別の欄か別のコンポーネントに持つ。`hit_region` は今の逆順の走査のまま。 | 要件 4.3〜4.5 の手前奥が「並べる順」だけで決まる。判定のたびの再帰が無い。`SurfaceMaster.collisions` は転記のまま残る。 | 列を作る段が 1 つ増える。循環・無い番号の枝を除く判定（要件 4.6）を合成と揃える必要がある。 |
| B: `SurfaceMaster.collisions` へ直接足す | 依存の順に、子の列を親の列の前へ差し込む。 | `hit_region`・presenter に変更なし。 | 「転記のまま」の不変条件を崩す。足した後のサーフェスがさらに別の親の子になる順序に気を遣う。 |
| C: 判定のたびに再帰する | `hit_region` に面の表を渡し、element をたどる。 | 持つものが増えない。 | ポインタが動くたびに再帰。`hit_region` の「`SurfaceMaster` 1 つの純関数」の契約と、その上の多数のテストが変わる。 |

### 4.3 無い番号・循環の警告の出し方

| 案 | 中身 | 利点 | 欠点 |
|---|---|---|---|
| **A: 読み込みのときに 1 度** | `dangling_pattern_targets` と同じ型で、面の表から「無い番号」「循環」を集めて報告にし、`load_shell_target` が 1 度だけ `warn!`。合成の中の読み飛ばしは `trace!`／`debug!`。 | emo 三段の「記録は fs を触る入口 1 か所」に合う。毎コマの警告にならない。 | 静的な解析（element の辺だけの循環の検出）を 1 つ書く。合成の中の実際の読み飛ばしと同じ結論になることをテストで留める。 |
| B: 合成の中で出す | 今の `flatten_surface` の循環の `warn!` をそのまま element にも使う。 | 新しい解析が要らない。 | 合成のたび（しかも外形と命令で 2 回）に出る。子が動くと毎コマ出る。 |

### 4.4 子の時計の置き場所と見えない間の扱い

置き場所:

| 案 | 中身 | 利点 | 欠点 |
|---|---|---|---|
| **A: `LoopRuntime` に再生・`ScopeStates` にコマ** | 再生（開始時刻）は `LoopRuntime` に「(スコープ, 子の番号) → animation の番号」の表を足す。子のコマは `ScopeStates` が「スコープ × 子の番号」で持ち、`apply` と `commit_pattern` が見える子の分だけを `PatternState` へ写す。 | 切り替えの `Show` に子のコマを載せられる（要件 5.6 の 1 コマの戻りが出ない）。今の「再生は looper・コマは state」の分担のまま。 | `ScopeStates` が「どの子が見えるか」の表（`AnimationTable` の写し）を知る必要がある。 |
| B: 全部 `LoopRuntime` に持つ | `apply` は空の `PatternState` のまま。切り替えの直後にアクターが子のコマを足した `Show` を出し直す。 | `state.rs` の変更が小さい。 | 切り替えで `Show` が 2 件出るか、アクターが `apply` の結果を書き換える。順序の檻が要る。 |

`PatternState` への載せ方は brief のとおり**別の欄を足す**（animation の番号 → コマ、の今の欄は変えない）。`PatternState` を名指しするファイルは 82 あるが、欄が私有なので波及しない。`ComposeKey` は `PatternState` を丸ごと含むので、キャッシュの鍵は自動で子のコマを含む。

見えない間の扱い（要件 5.7・5.11）:

| 案 | 中身 | 利点 | 欠点 |
|---|---|---|---|
| A: 見えない子も毎秒抽選する | 動き出した子は見えなくても抽選と進行を続け、コマは見える分だけ載せる。 | 「止まらずに進んでいた」に字義どおり。 | 見えない子が乱数を消費する。動き出した子が増えるほど毎秒の仕事が増える。 |
| **B: 再生中の分だけ進める** | 見えない子は抽選しない。再生中だったものは開始時刻を持ったまま、戻ったときに経過から今のコマを引く（多くは終わっている）。 | 見えない間の仕事が 0。乱数は見えている子だけが消費する。今動く間隔の語（`random` 系）では A と見分けがつかない。 | 将来、周期で動く間隔の語（`always`・`periodic`）や動く画像が載ると「見えない間も周期が進んでいた」を開始時刻から計算する形に延ばす必要がある。 |

### 4.5 全体の組み立て

- **案 X（既存を延ばす）**: 4.1-A・4.2-A・4.3-A・4.4-A を、`plan.rs`・`fold.rs`・`boxes.rs`・`table.rs`・`looper.rs`・`state.rs` の中へ足す。新しいファイルは検体とテストだけ。行数は `plan.rs` 694・`looper.rs` 410・`state.rs` 600・`table.rs` 517（いずれもテストを除く）で、1,000 行には届かない見込み。
- **案 Y（新しい部品に切る）**: 静的な入れ子の解析（参照の表・無い番号・循環・領域の列）を compose の新しいモジュール 1 つに、子の時計を seriko の新しいモジュール 1 つに置く。
- **案 Z（混ぜる）**: 合成と外形の再帰は `plan.rs` の中で延ばし（同じ再帰に入れるのが自然）、静的な解析と領域の列は compose の新しいモジュール、子の再生の表は seriko の新しいモジュールへ。`looper.rs`・`state.rs` は呼ぶだけにする。brief が示した継ぎ目（「静的な入れ子」と「子の時計」）とも重なる。

## 5. 規模とリスク

- **規模: M〜L**。静的な入れ子（読み分け・合成・外形・警告・領域・箱の警告・検体）は既存の型の延長。子の時計は seriko の 3 ファイルと compose の `PatternState`・`plan.rs` にまたがる。brief の見立て（16〜21 タスク）と合う。4.1-A を採ると `manifest.rs`・`model.rs`・`cache.rs` の作業が消える。
- **リスク: 中**。
  - 入れ子の無いシェルの不変（要件 7）: 乱数の消費順と `PatternState` の等しさが、子の欄が空のとき今と同じであることを檻で留める必要がある。既存の golden と seriko の決定論テストがそのまま番人になる。
  - 切り替えの 1 コマ（要件 5.6）: `apply` が空の `PatternState` を載せる今の形のままだと、子が閉じ目の途中でも 1 コマだけ開いた目に戻る。実機でしか見えにくいので、`MockSurfaceOutput` の発行列で留める。
  - 合成のキャッシュ: 容量 3 のまま、親と子が同時に動くシェルで命中率が下がりうる。容量の変更は別の裁定の門（`cache.rs` の `CAPACITY` の注記）なので、本 spec では測るだけに留めるかを決める。

## 6. 設計へ持ち越す調べもの

1. `flatten_surface` の引数の形: 今の `is_top_level: bool` を「一番上／element の子（子のコマを見る・着せ替えは空）／pattern定義の先（コマを見ない・着せ替えは今のまま）」の 3 通りへ広げる形。pattern定義の先の、そのまた element の子は「コマを見ない」を引き継ぐ（要件 2.7・5.13）。
2. `bind+random` の animation を子に書いた場合: 着せ替えを持ち込まない（要件 5.12）ので、発火の条件（`LoopRuntime::on_tick` の着せ替えの番人）をどう読むか。
3. 数字が `u32` に収まらない欄（例 `99999999999`）: 画像として読むか、無い番号として警告するか。
4. 同じ子が親のアニメのコマ（pattern定義）にも element にも現れるとき、片方は止まった絵・片方は動く絵になる。そのままでよいかの確認。
5. `animated-image-playback` が要る間隔の語（周期で回る類）は今 `AnimationTable` が採らない。本 spec の時計の型が、後から「開始時刻からの周期」を載せられる形かだけ確かめる。
6. 検体の置き場所と形: フォルダのまま置く（`shell-balloon` の前例）か、`.nar` にして `sample-ghost-kit` 越しに引くか。

## 7. 要件ディスカッションへ出す論点

答えで作業が変わるものだけ。

> **要件ディスカッションでの仕分け（2026-10-04）**: 1・2・4・5・6・8 は実現の仕方の話なので設計（`/kiro-spec-design`）で決める。2 は、要件 5.7・5.11 の見え方（戻っても巻き戻らない・見えない子で描き直さない）を満たす限りどちらでもよい。6 は容量 3 のまま測るだけを既定とする。3 は要件 5.1 に「着せ替えのアニメーションは除く」を書き足して解消。7 は要件 1.9（無い番号と同じ扱い）として書き足して解消。5 には開発者裁定（議題 1）で条件が足された: 親に直接書いた領域が手前で、子・孫からは「親が持たない名前の領域」だけを段ごとに持ち込む（要件 4.8〜4.10）。列を作るときに名前で落とす段が要る。

1. **数字だけの読み分けの置き場所**（4.1）。A なら `manifest.rs`・`model.rs`・`decode.rs` に触らず、brief の「`manifest.rs` には必ず触る」は不要になる。B なら同じウェーブの約束を 2 つ外す。
2. **見えない子の扱い**（4.4 後半・要件 5.7・5.11）。「見えない間も毎秒抽選する」か「再生中の分だけ経過で進める」か。今動く間隔の語では見た目に差が出ないが、乱数の消費と将来の周期の語の載せ方が変わる。
3. **子の中の `bind+random`**（要件 5.1 と 5.12 の重なり）。5.1 は「一番上で動くものと同じ種類」、5.12 は「着せ替えを持ち込まない」。子の中の `bind+random` は動かない、でよいか。
4. **警告を出す時機**（4.3・要件 3.1・3.2・8.1）。読み込みのときに 1 度だけ出し、合成の中では黙って読み飛ばす、でよいか（今の pattern定義の循環の警告は合成のたびに出ている。こちらは本 spec では変えない）。
5. **当たり判定の手前奥を「並べる順」で実現する**（4.2-A）。子の領域 → 親の領域の順に並べた列を構築時に作る形でよいか。孫の領域は「その子を単独で表示したときと同じ」（要件 4.4）なので、子の列をそのまま位置だけずらして入れる。
6. **合成のキャッシュの容量**（2.8）。本 spec では 3 のまま測るだけにするか、入れ子のシェルの命中率を要件に入れるか。
7. **`u32` に収まらない数字**（6 の 3）。要件 1.1 の「十進のサーフェス番号として読む」の端。
8. **箱の「画像より下」の報告**（2.6）。サーフェスを指す element を「画像の element」に数える今の成り行きでよいか。
