# ギャップ分析: hit-id-pass

> 2026-11-20・`/kiro-validate-gap`（`/kiro-start` の中の下請け）。対象は `requirements.md`（下書き・未承認）と今の main（`9bf6e32e`）のコード。
> 調べは静的な読みだけ（SSP の測り・areka の起動・ビルド・テストの実行はしていない）。この文書は材料と候補を並べるもので、決めは要件の討議が持つ。

## 1. 分析の要約

- **当たり判定の解決の入口は `&EmoPresenter` だけを受ける**（`presenter/hit.rs` の `hit_region_client(&self, ..)` → `areka-runtime/src/hit_region.rs` の `resolve_hit_region(presenter, ..)` → 本体 `input_events/mod.rs` 184 行）。World も装置も渡らない。GPU の ID の描き先を wintf の部品（描画面の持ち主の実体に付く形）にすると、読むために口の形を変えることになり、本体のファイルに触る（要件 8.6 とウェーブ C23 の約束 ⑶ に当たる）。GPU でも、presenter が自分で持つ単体の入れ物（World を介さず `&self` で読める形）なら本体に触らずに済む。
- **色の合成は今も CPU**（`draw-methods-canon` は未着地）。表示中の絵の α は、原寸の `AlphaMask`（閾値 128）として presenter のキャッシュの中に在り、作者の座標へ戻した点でそのまま引ける。要件 4.1（色の絵の α が 0.5 未満なら当たり無し）は、GPU の読み戻しを使わなくても今の資産で満たせる。
- **brief の「ほかの形（CPU）」の費用の見立ては今のコードと合わない**。「変形ごとに逆変換を書く」と書いてあるが、今ある変形は窓全体の拡大率 k だけで、その逆変換（`ScaleRatio::unscale_coord`）は SHIORI へ届けるローカル座標のためにすでに毎回走っている。CPU 側で ID を持つ形でも、新しく書く逆変換は 0 本である。
- **wintf の `ColorMapData`／`HitRegionMap` は `region` の意味論と形が合わない**（1 枚の画像で色 → 名前が 1 対 1・255 色まで・先に書いた方が勝つ・多角形は偶奇・画像を実体の大きさへ伸ばす・乗算済みで読む）。使い回せるのは WIC で読む 30 行ほどだけで、areka は自前の読み手の口（`areka-emo-atlas` の `ElementDecoder`）を持つ。どの候補でも要件 5.17（使い回すなら 16 番を直す）の前提が成り立たない見込み。
- **「まだ働かない形」の記録を止めるには読み手のファイルに触る**（`ShellIssue::ShapesInactive` は `areka-parsers` の `shell/{model,decode,parse,mod}.rs` に在り、文言を 2 つのテストが字面で固定している）。brief の「読み手に触らない」とは両立しない。
- 候補は 3 つ: **A** GPU の ID の描き先を wintf に作る（要件の下書きの形）／**B** ID の描き先を CPU のメモリに持ち、areka（emo-compose）で塗る／**C** B の仕組みを wintf の `layout/hit_region/` に置く。規模と危険は A＝L・高／B＝M・低〜中／C＝M〜L・中（5 節）。

## 2. 今の状態（調べた資産）

### 2.1 当たり判定の解決の今の道

| 層 | ファイル | 今の中身 |
| --- | --- | --- |
| 純粋な判定 | `crates/areka-emo-compose/src/hit.rs`（737 行・本体は 172 行まで） | `hit_region_in(collisions, x, y, priority) -> Option<&str>`: 並びを後ろから見て最初に当たった名前を返す。形の述語は私的な `hits`（99〜111 行）で、`Rect` だけ閉区間の比べ、ほかの 4 つの形は `false`。`hit_region_scaled_in` は点を `ScaleRatio::unscale_coord`（画素の中心の逆写像）で戻してから同じ関数へ渡し、戻した点 `surface_point` を返す。wintf にも α にも依らない（冒頭の文書 5 行） |
| 並び | `crates/areka-emo-compose/src/hit_import.rs`（315 行）・`world.rs` 166 行 | `EmoWorld::hit_regions(id) -> Option<&[Collision]>`。子から持ち込んだ分が在れば［持ち込み］→［直接］の列。形ごとのずらしは私的な `shifted`（285 行）。持ち込む数の予算は `IMPORT_BUDGET = 1 << 20` |
| presenter | `crates/areka-emo-present/src/presenter/hit.rs`（150 行） | `hit_region_client(&self, target, x, y) -> ClientHit<'_>`。k は `PresentTarget::applied` の直読。`debug!` 1 行（k・窓の点・作者の座標の点・名前）を残す。文書は「`&self` のみを取り World・GPU に依存しない」と書く（85〜87 行） |
| 結線 | `crates/areka-runtime/src/hit_region.rs`（138 行） | `resolve_hit_region(presenter: &EmoPresenter, scope, x, y) -> HitRegion`。名前を所有の `String` へ写すだけ |
| 本体 | `crates/areka/src/input_events/mod.rs` 174〜200 行・446〜455 行 | `resolve_hit_owned(world: &World, ..)` が `RuntimeWiring::presenter()` を借りて `resolve_hit_region(p, scope, x, y)` を呼ぶ。手元に在るのは `&World` |
| 型 | `crates/areka-parsers/src/shell/model.rs` 199〜248 行 | `Collision { index, shape, name }`・`CollisionShape`（`Rect`・`Ellipse`・`Circle`・`Polygon`・`Region`）・`CollisionRegion { file, red, green, blue, invert, x, y }`。`file` は書いたままで開いていない |

- presenter の装着の口 `attach_target(target, window, emo_world, atlas, author_dpi)` の本番の呼び手は `crates/areka-runtime/src/frame/attach.rs` 303・372 行（ウェーブ C23 の約束 ⑶ で触らない）。テストの呼び手は 60 か所を越える。
- シェルの読み込みは `crates/areka-emo-present/src/shell_target.rs` の `load_shell_target_with` → `build_core`（シェルのフォルダと読み手 `&impl ElementDecoder` を持つ）。面の表 `EmoWorld` は `ShellTarget::build_world`／`take_world` が組んで渡す。**`region` の画像を `build_core` で読み、`EmoWorld` の中へ載せて渡せば、`attach_target` の形・`frame*`・本体に触らずに presenter まで届く**（持ち込みの列 `HitRegions` を `EmoWorld` の部品として載せているのと同じ道）。
- 採寸の読み `load_shell_target_quiet`（areka-placement）も同じ核を通る。`region` の画像を核で読むと、採寸の読みでも読むことになる。

### 2.2 wintf の `layout/hit_region/mod.rs`（508 行・完了 `event-hit-test-named-regions`）

実際に在るもの:

- `Shape`（`Rect { x, y, width, height }`・`Polygon { vertices }`・DIP の f32）と `ShapeRegion { name, shape }`。楕円と円は無い。
- `ColorMapData { index_map: Vec<u8>, region_names, width, height }`。`from_image(path, mapping: &HashMap<(u8,u8,u8), String>)` は WIC で読み、**乗算済みの `GUID_WICPixelFormat32bppPBGRA`**（181 行）へ変換してから、α を見ずに BGR で対応を引く（209〜221 行「アルファは無視」）。対応は 255 色まで（146 行・越えると `E_INVALIDARG`）。
- `HitRegionMap`（ECS の部品）。`Shapes` と `ColorMap` は排他（`RegionKind`）。`hit_test_region(rel_x, rel_y, entity_size)` は正規化した f32 の座標を受け、`Shapes` は**先に書いた方が勝つ**（351〜352 行）、`ColorMap` は `rel × 画像の幅` で画素を引く（375〜376 行＝画像を実体の大きさへ伸ばす）。
- 多角形の内外は `point_in_polygon`（472〜501 行）の Ray Casting（**偶奇**）。
- ビルダーは幅か高さが 0 以下の矩形を `InvalidRectSize` で断る（435 行）。
- 使い手は wintf の例（`examples/taffy_flex_demo`）・`layout/hit_test/mod.rs`・wintf 自身のテストだけ。**areka のクレートからの参照は 0**（`HitRegionMap`・`ColorMapData`・`from_color_map` を全クレートで引いた結果）。

`region`（要件 2）と比べた食い違い:

| 観点 | `ColorMapData`／`HitRegionMap` | 要件 2・3 が求める形 |
| --- | --- | --- |
| 画像と名前 | 1 枚の画像・1 色 → 1 つの名前（`HashMap`） | 当たり判定 1 件ごとに（ファイル・色・反転）→ 名前。同じ色を 2 件が指せる。反転は「指定色でない全部」なので 1 画素が何件にも属する |
| 重なり | 先に書いた方が勝つ | 後に書いた方が手前（5 つの形と `collision*` をまたぐ 1 本の並び） |
| 数 | 255 色（`u8`） | 255 を越える数を区別（3.7） |
| 置き方 | 実体の大きさへ伸ばす | 1 画素を 1 画素に・伸ばさない（2.5） |
| 透明 | 乗算済みで読む＝透明は黒 (0,0,0) に当たり、半透明は色がずれる（申し送り 16 番） | 透明は当たらない・半透明は掛ける前の色で比べる（2.8・2.9） |
| 多角形 | 偶奇 | 巻き数（1.4） |

→ `region` に使い回せるのは WIC の読みの手順だけ。**申し送り 16 番は今も開いたまま**（181 行は PBGRA のまま・`tests/color_map.rs` 293 行は「16 番の透明の扱いは hit-id-pass で決める・ここで固定しない」と書いて透明の答えを固定していない）。

### 2.3 クリックの透過と色の絵の α

- `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`（438 行）: `AlphaMask`（1 画素 1 ビット・閾値 `ALPHA_THRESHOLD = 128`・`is_hit(x, y)` は範囲外で `false`）。`from_pbgra32`／`regenerate_from_pbgra32`。
- `crates/areka-emo-present/src/presenter/budget.rs`（566 行）: `FrameBudget`。新しい確保を数える器と、合成先の席・マスクの輪番（`regenerate_mask`）。α の判定そのものは持たない。
- `crates/areka-emo-present/src/cache.rs`（332 行）: `ComposeCache`（容量 3・LRU）。1 件 `CacheEntry { composed, mask: Arc<AlphaMask>, display }`。`composed` は**原寸**の乗算済み BGRA、`mask` はその原寸のバイトから作る（`presenter/show.rs` 248〜254 行）。
- 合成の外形は原点 (0,0) 固定（`plan_extent.rs` 27〜28 行）。当たり判定の座標と原寸の絵の画素は同じ空間に在る。
- つまり**作者の座標へ戻した点 `(sx, sy)` で `entry.mask.is_hit(sx, sy)` を引けば、要件 4.1 の答えが出る**。`cache.get`（順を動かさない読み）は `&self` で引ける。表示中の入力は `PresentTarget::last_show` が持つ。
- 注意 1: 表示中の 1 件が `InvalidateCache` で消えた後、次の表示までのあいだは引けない（そのときの答えの決めが要る）。
- 注意 2: wintf のクリックの透過は `alpha_mask_hit`（物理寸の境界に対する比例の写像）で原寸のマスクを引く。`unscale_coord`（画素の中心の逆写像）とは k が整数でないとき縁の 1 画素で食い違いうる（今も、クリックの透過と名前の解決は別の写像で動いている）。
- `draw-methods-canon` が合成を D2D へ移すと、この CPU の画素とマスクの出どころは無くなる（そのときの付け替えはあちらの議題・要件の Out of scope）。

### 2.4 wintf の入れ物・1 ピクセルの読み戻し・WARP の足場（完了 `wintf-d2d-composite-rules`・`wintf-visual-tree`）

- `crates/wintf/src/ecs/graphics/composite.rs`（476 行）: `SurfaceComposite`（描画面 1 枚の入れ物・`ID2D1Bitmap1`・`B8G8R8A8_UNORM`・乗算済み・96 DPI）。`SurfaceGraphics.composite`（`components.rs` 195 行）に 1 つ。作るのは係 `render_surface`（`systems/render.rs` 265〜288 行・`Changed<SurfaceGraphicsDirty>` の回）。`read_pixel(x, y)`（413〜440 行）は 1×1 の `CPU_READ` の写し先へ `CopyFromBitmap` → `Map` → `Unmap`。写し先と控え `(x, y, 世代, 値)` を 1 つの `Mutex` に入れ、同じ座標・同じ世代なら GPU から読まない。
- `crates/wintf/src/ecs/graphics/readback.rs`（141 行）: 公開の口は `SystemParam` の `SurfaceReadback`（`read_pixel(surface_owner, x, y) -> PixelRead`・`read_all`）。答えは `Pixel([u8; 4])`・`Outside`・`Unreadable`。**使い手は wintf 自身のテストだけ**（`visual/draw/README.md` 83 行）。areka はまだ使っていない。
- 1 ピクセルの読み戻しの待ち（`wintf-d2d-composite-rules` の design.md 689 行・実 GPU・開発機の内蔵 GPU・512×512）: 合成の直後＝中央値 516 µs・95% 点 747 µs・最大 1.26 ms／GPU が空いているとき＝中央値 344 µs・95% 点 556 µs。UI スレッドで待つ。
- WARP の足場: `crates/wintf/src/ecs/graphics/visual/draw/emit_test_support.rs`（`warp_device()`・`warp_dc()`・`draw_to_pixels` ほか）。**`pub(crate)` で、wintf の中のテストからしか使えない**。areka の側の presenter のテストは実 GPU の `GraphicsCore::new()`（`presenter_test_support.rs` 43 行）で回っている。
- steering `tech.md` 72 行: WARP を使ってよいのは「合成の答えの画素（描画の木・ブレンド・不透明度の層・入れ物・読み戻し）」だけで、許容差は ±1。ID の描き先（要件 10.7＝完全一致）はこの列に無い。
- 形を描く資産: `com/d2d/mod.rs` の `FillEllipse`・`FillRectangle` の包み、`visual/draw/clip_geometry.rs` の `CreatePathGeometry`。**縁のぼかしなし（`D2D1_ANTIALIAS_MODE_ALIASED`）で形を塗る道・`D2D1_FILL_MODE_WINDING` の指定・マスクの画像で塗る道・α を無視する的は、どれも今の wintf に無い**（`ALIASED` の使用は `composite.rs` 392 行の軸に沿う切り抜き 1 か所だけ）。
- presenter が表示の成立点で共有の DC を借りて同期に D2D を呼ぶ前例は在る（`presenter/show.rs` 182〜222 行の `record_display`）。装置の作り直しへの追従の口も在る（`presenter/refresh.rs` の `refresh_after_device_change`・毎フレーム呼ばれる）。

### 2.5 「まだ働かない形」の記録（`ShellIssue::ShapesInactive`）

| 所 | 行 | 中身 |
| --- | --- | --- |
| `crates/areka-parsers/src/shell/model.rs` | 357〜363・378〜385・398・446〜453 | 腕 `ShapesInactive { heading, text, counts }`・`ShapeCounts`・重さ（warn）・文言「…まだ当たり判定として働かない（ID の描き先に塗るのは hit-id-pass）…」 |
| `crates/areka-parsers/src/shell/decode.rs` | 39〜47・84〜110・310 | `Reading` の `inactive`・`first_inactive`・`finish` |
| `crates/areka-parsers/src/shell/parse.rs` | 69〜70 | `reading.finish()` |
| `crates/areka-parsers/src/shell/mod.rs` | 78 | `ShapeCounts` の公開 |
| `crates/areka-emo-present/src/shell_target.rs` | 469〜476 | 記録の出口（`issue.message()` を `warn!`） |
| テスト `crates/areka-parsers/src/shell/decode_collisionex_tests.rs` | 74〜（7 本ほど） | 腕・数・並びの最後に 1 件・文言に `hit-id-pass` を含むこと |
| テスト `crates/areka-emo-present/src/shell_target_element_base_tests.rs` | 386〜444 | クローディアで warn 以上が「まだ働かない形の記録 1 行（楕円 32・円 0・多角形 28・region 0）」と surface28 の 3 行だけ。**文言を 1 行まるごと固定** |
| 文書 | `doc/ukadoc-coverage/ledger/assets.toml` 7039 行・`doc/COMPAT_ARCHITECTURE.md` 283 行・`doc/ukadoc-coverage/roadmap-draft.md` 589 行 | `ShapesInactive` と「まだ当たらない」の説明 |

- 記録を**出なくする**だけなら出口（`shell_target.rs` 470 行の繰り返し）で捨てられるが、読み手の腕・文言・テスト 7 本ほどが「まだ働かない」のまま残り、要件 9.4（文言を書き直す）と 10.13（テストを書き直す）を満たせない。
- 腕ごと消すなら、触るのは parsers の 4 ファイルとテスト 1 本、emo-present のテスト 1 本。完了 `collisionex-regions` の設計は「`ShellIssue` の腕を足し引きしたとき」を見直しの引き金に挙げている（design.md 54 行）。
- `ukadoc-survey` は `ShapesInactive`・`ShellIssue` を字面で読んでいない（クレートの中を引いて 0 件）。

「当たらない」を固定している既存のテスト（要件 10.13 で書き直す的）:

- `crates/areka-emo-compose/src/hit.rs` 691〜732 行 `non_rect_shapes_never_hit_and_fall_through`
- `crates/areka-emo-compose/src/hit_order_tests.rs` 41〜70 行 `shapes_not_yet_working_never_hit_and_never_cover_a_rect_below`（冒頭の説明 4 行も）
- コードの説明: `hit.rs` 99 行・`model.rs` 214 行（「今当たるのはこの形だけ」）・`decode.rs` 22〜24 行

### 2.6 完了 `collision-geometry` の「名前の解決は α を参照しない」

- 要件 6.1（`completed/areka-P0-collision-geometry/requirements.md` 89 行）: 「点が透明画素上にありかつ当たり判定矩形の内側にある → 画素の α に関わらず領域名を解決する」。設計の正典確定表 C8（design.md 39 行）は、正典が沈黙する点を areka の決めとして「別層＝純関数は α を一切参照しない」と置いた。
- **実装は確かにそうなっている**: `hit_region_in`／`hit_region_scaled_in` は α の入力を持たない（引数は並び・点・k・優先の規則だけ）。presenter も名前の解決でマスクを引かない。
- **固定しているのは形（引数に α が無いこと）で、透明な画素の上で名前が返ることを名指しで確かめるテストは無い**（設計の対応表 302 行は 6.1 の手段を「α を参照しない純関数」とし、tasks.md は 6.1 を純粋な核のタスクに並べるだけ。`α に関わらず`・`透明画素` を emo-compose・emo-present・runtime・本体のテストで引いて 0 件）。
- 同じ設計の C-3（design.md 656 行）: 透明な所ではクリックの透過でマウスのイベントが窓へ届かないので、ふだんの道では 6.1 の違いは見えない。
- → 要件 4.1（仮定 8）で改めても、赤になる既存のテストは無い。改めるのは決め（§8 の表と `hit.rs` の冒頭の文書）と、新しく足すテスト。

### 2.7 画像の読み手

- areka の読み手の口は `crates/areka-emo-atlas/src/decode.rs` の `ElementDecoder::decode(path) -> DecodedImage { width, height, stride, bgra, has_alpha }`。既定の腕 `decode/wic_arm.rs` は **乗算済みの PBGRA へ変換**して返す（88〜98 行）。テストの腕 `MemoryDecoder` が在る。
- ファイルの探し方は `base_dir.join(rel_path)`（`areka-emo-atlas/src/lib.rs` 150 行）。区切りと大小文字は OS に任せている。要件 2.1 の「サーフェスの絵と同じ決まり」はこの 1 行。
- **乗算済みで受け取ると要件 2.9（半透明は掛ける前の色で比べる）を厳密には満たせない**（α が小さいと元の色へ戻せない）。透明（α = 0）は当たらない決め（2.8）なので色が失われても困らない。厳密に満たすなら、乗算しない読み（`GUID_WICPixelFormat32bppBGRA`）の口を読み手に足す＝`areka-emo-atlas` に触る（brief の「触るファイル」に無い）。

### 2.8 文書・台帳

- 台帳の `collisionex*` は 1 行（`assets.toml` 7031〜7041 行・`degraded`・担当 `collisionex-regions`）。`region` だけの行は無い（brief の「`region` の行」はこの 1 行のこと）。`collision*` の行（7026 行）の縮退の理由は「重なりの順が正典と逆」。
- `doc/COMPAT_ARCHITECTURE.md` §8 の 283 行が「`ellipse`・`circle`・`polygon`・`region` がまだ当たらないこと」。

## 3. 要件と資産の対応（Missing＝無い／Unknown＝調べが要る／Constraint＝制約）

| 要件 | 今の資産 | ギャップ |
| --- | --- | --- |
| 1 楕円・円・多角形の当たり | `hit.rs` の `hits` が `false` を返す腕 3 つ。形の欄は型に在る | **Missing**: 塗り方（どの候補でも新しく書く）。**Unknown**: SSP の縁・大きさ 0 の振る舞い（未測定）。**Constraint**: 1.10「サーフェスの中だけ」は、今の `rect`（外形の外でも当たる）と食い違う（6 節の 12） |
| 2 `region` の当たり | 型 `CollisionRegion`（開いていない）。読み手の口 `ElementDecoder`（乗算済み） | **Missing**: 画像の読み・色の一致・反転・置き方。**Constraint**: 2.9 は乗算しない読みが要る。**Unknown**: SSP の置き場所・透明の扱い（未測定） |
| 3 `rect` と重なりの順 | `hit_region_in` の後ろからの走査・`hit_import.rs`・既存のテスト（k=2・k=5/4 の縁の期待値つき） | **Constraint**: 3.1 を ID の描き先でも保つこと（A では D2D の塗りの規則が `unscale_coord` と同じ答えを出すことの確かめが要る・B と C は点を戻してから原寸で引くので構造で同じ）。3.8 の上限越えは 24 ビットでは実データで届かない（持ち込みの予算 2^20 より上限が大きい）＝テストの作り方が要る |
| 4 透過・拡大率・時 | 原寸の `AlphaMask`（2.3）・`unscale_coord`・`applied` の直読 | 4.1 は資産あり（引く 1 行と、引けないときの決め）。4.3〜4.5 は今のまま。**Constraint**: 4.4 は ID の描き先の作り直しと名前の表の入れ替えを同じ時点で行うこと |
| 5 wintf の ID の描き先 | `SurfaceComposite`・`SurfaceReadback` がひな形。WARP の足場 | **Missing**: 全部（A のときだけ要る）。**Constraint**: 入口が `&EmoPresenter`（1 節）・WARP の足場は wintf の中だけ・`tech.md` の WARP の例外の列。B・C では 5.1〜5.16 は要らなくなる |
| 5.17 色分け画像の読み込みの直し | `ColorMapData::from_image`（16 番が開いたまま） | **Constraint**: どの候補でも `region` に使い回さない見込み（2.2）＝この項の前提が成り立たない |
| 6 負荷 | 点 1 つは今は並びの件数に比例 | **Missing**: ID の描き先・画像の 1 度読み。6.5（実 GPU の待ちの測り）は A のときだけ意味を持つ。**Unknown**: 持ち込みで並びが大きいとき（上限 2^20 件）の塗る仕事の予算 |
| 7 記録 | `ShapesInactive`（2.5）・`hit_region_client` の `debug!`（`presenter_refresh_and_log_tests.rs` が固定） | 7.2 は読み手に触る（2.5）。7.3・7.4 は A のときだけ |
| 8 下流 | `HitRegion`・Reference4・カーソルの形の行は名前の文字列を受けるだけ | ギャップなし（口の形を変えなければ）。8.6 は A の部品の形しだいで破れる |
| 9 文書と台帳 | 台帳 1 行・§8 283 行 | 書き直しと追記。9.5 は A のときだけ |
| 10 確かめ | emo-compose の純粋なテスト・emo-present の実 GPU のテスト・wintf の WARP の足場・クローディアの検体（楕円 32・多角形 28） | 10.7・10.8 は A のときだけ。10.11（`circle`・`region` の検体が無い）は作り物の検体が要る |

## 4. 実装の候補

### 共通（どの候補でも同じ）

- `region` の画像は `shell_target.rs` の `build_core` で読み、`ShellTarget` に持ち、`build_world`／`take_world` で `EmoWorld` へ載せる（スコープの数だけ組むので共有で持つ）。記録（読めないファイル 1 件ずつ）は `load_shell_target_with` の出口で出す。`attach_target`・`frame*`・本体には触らない。
- 色の絵の α（要件 4.1）は presenter が原寸の `AlphaMask` を作者の座標の点で引く。
- `hit_region_client(&self, ..) -> ClientHit` と `resolve_hit_region` の形は変えない。
- `ShapesInactive` を腕ごと消す（6 節の 10 の決めしだい）。台帳・§8・既存のテストの書き直し。
- 名前の表は「ID → 並びの中の位置」。ID は並びの位置＋1 でよい（並びが決まれば表は要らず、`collisions[id - 1].name` で引ける）。

### 候補 A: GPU の ID の描き先を wintf に作る（要件の下書きの形）

wintf に ID の描き先（`B8G8R8A8`・α 無視・`ALIASED`・上書き）と 1 点の読み戻しを足し、areka は形の列（矩形・楕円・多角形・マスクの画像）を拡大つきで渡す。

- **部品の置き方が 2 通り**:
  - **A-1 描画面の持ち主の実体に付く部品＋係＋`SystemParam` の読み口**（`SurfaceGraphics`／`SurfaceReadback` と同じ形）。要件 5.1・5.2・5.13 の文のとおり wintf が寿命を持てる。ただし読むには World が要り、`resolve_hit_region` の形を変えて本体 `input_events/mod.rs` を書き換えることになる（要件 8.6・ウェーブの約束 ⑶ に当たる）。
  - **A-2 presenter が持つ単体の入れ物**（World を介さない型。描くのは表示の成立点で共有の DC を借りて同期に・読むのは中の `Mutex` を通して `&self` で）。本体にも `frame*` にも触らない。作り直し（大きさの変化・装置の失い）は使い手（presenter の `refresh_after_device_change`）が起こす形になり、要件 5.1・5.13 の文（wintf が持たせる・wintf が作り直す）を書き直すことになる。
- **触るクレート**: wintf（新しいファイル 2〜3 本＝入れ物・形の塗り・読み口とそのテスト、`ecs/graphics/mod.rs` の公開、描画の文書）・areka-emo-present（形の組み立て・ID の描き先の持ち主・`hit.rs`・`show.rs`・`refresh.rs`・`scale_change.rs`・テスト）・areka-emo-compose（`hit.rs` の述語と文書・テスト）・areka-parsers（`ShapesInactive`）・areka-emo-atlas（乗算しない読みを足すなら）・steering `tech.md`（WARP の例外の列）。
- **wintf に無くて要るもの**: 縁のぼかしなしの形の塗り・巻き数の指定・マスクの画像を補間なしで ID の色で置く道・α を無視する的・ID の読み分けの型。
- **利点**:
  - 開発者の 2026-10-15 の案（「ヒットテストIDだけを持ったサーフェス」「仕組みは wintf か areka か」）にそのまま応える。
  - 表示の拡大率が大きいとき、楕円と多角形の縁が物理ピクセルの細かさで決まる。
  - 将来、形ごとに別の変形が要るようになったとき（`animation*.collisionex*` や合成の D2D 化の後）、色の絵と同じ変形で描ける。
- **欠点・危険**:
  - マウスが動くたびに UI スレッドで 0.3〜0.8 ms の同期の待ち（2.4 の測り。同じ座標の再読は控えで 0）。色の α も GPU で読むなら 2 回になる（CPU のマスクで引けば 1 回）。
  - 要件 3.1（`rect` の答えを拡大率によらず今と同じに）が、D2D の塗りの規則と `unscale_coord` の一致に頼る。算術の上では同じ（画素の中心が `[left·k, (right+1)·k)` に入るか）だが、f32 の変換と GPU の小数の刻みの下で縁の画素が同じになるかは未確認。
  - 楕円は D2D が折れ線に割ってから塗るので、縁の画素は「数式の楕円」でなく「D2D の割り方」で決まる。WARP と実 GPU で縁が同じになるかは見込みだけ（brief「機種で揺れない見込み」）。
  - マスクの画像を拡大して置くとき、補間なしで ID を混ぜずに置ける D2D の手段が未確認（`FillOpacityMask` の補間・ID の色を焼いた絵を `NEAREST_NEIGHBOR` で描く形だと `region` 1 件ごとに絵が要る）。
  - WARP の足場が wintf の中だけなので、areka の側の形の答え（10.1〜10.6）は実 GPU のテストになるか、wintf の語彙で wintf の中に置くことになる。`tech.md` の WARP の例外の列に ID の描き先を足す直しも要る。
  - 失敗の道が増える（装置の失い・まだ描いていない・読み戻しの失敗＝要件 7.3・7.4）。
  - 持ち込みで並びが大きいシェル（上限 2^20 件）では描く命令の数の予算が要る。

### 候補 B: ID の描き先を CPU のメモリに持ち、areka（emo-compose）で塗る

面の当たり判定の並びを、原寸の「画素ごとの ID の配列」へ後ろほど上書きで塗る純粋な関数を emo-compose に足す。点は今と同じく `unscale_coord` で戻し、その 1 画素の ID を読む。wintf には触らない。

- **触るクレート**: areka-emo-compose（新しいファイル 1 本＝塗り＋テスト、`hit.rs` の口と文書）・areka-emo-present（`shell_target.rs` の `region` の読み・`presenter/hit.rs`・ID の配列を持つ所と作り直す時点・テスト）・areka-parsers（`ShapesInactive`）・areka-emo-atlas（乗算しない読みを足すなら）。wintf・`tech.md`・本体・`frame*` は 0。
- **利点**:
  - 開発者の 2026-11-17 の裁定「形は ID で塗る」を満たす（塗る先が CPU のメモリ）。点 1 つの判定は 1 画素の読みと名前の引き当て（要件 6.4）。
  - 要件 3.1 が構造で成り立つ（点を戻す式も原寸の座標も今と同じ）。`rect` の既存の期待値（k=2・k=5/4 の縁）をそのまま使える。
  - 決定論のテストが GPU なしで全部書ける（10.1〜10.6・10.9）。UI スレッドの GPU の待ちが無い。装置の失いの道が無い（7.3・7.4 が要らない）。
  - 縁の画素の規則（画素の中心が形の内側か）を数式で決められ、機種に依らない。
  - `region` は画像の画素をなめて ID を置くだけ。ID の幅は `u32` で足り、幅の議題（仮定 3）が軽くなる。
  - 新しい公開の口が wintf に増えない。
- **欠点・危険**:
  - 開発者の 2026-10-15 の案の「wintf の層の仕組み」にはならない（areka の中に閉じる）。
  - メモリ: 原寸 × 4 バイト（クローディア 333×500 で 約 670 KB）。持つのは表示中の面だけにするか、面ごとに覚えるかの決めが要る。
  - 面を切り替えた回に塗る仕事が走る（形の数 × 囲みの面積。`region` が 100 件なら 100 × 画像の面積）。持ち込みで並びが大きいシェルでは予算が要る（A と同じ問い）。
  - 楕円と円の内外の式を自前で書く（完了 `collisionex-regions` の設計の初版が同じ式を書きかけ、決め 7 で外した。そのときは点ごとの判定で整数の極値の溢れを気にしたが、塗る形では先に面の外形で切るので座標は外形の中に収まる）。
  - 拡大率が大きいとき、縁は原寸の 1 画素の刻みになる（今の `rect` と同じ粗さ）。
  - 将来、形ごとに別の変形が要るようになったら、塗りの関数に変形を足すことになる（窓全体の変形なら点の側の戻しだけで済む）。
- **要件への響き**: Requirement 5 が丸ごと要らなくなる。6.5・7.3・7.4・9.5・10.7・10.8 が消えるか変わる（仮定 1 に書いてあるとおり）。4.2 は「点を拡大率で戻してから引く」形で満たす。

### 候補 C: B の仕組みを wintf の `layout/hit_region/` に置く（仕組みは wintf・意味は areka）

B と同じ「CPU のメモリの ID の配列」を、wintf の公開の型（areka の語を持たない・形の列と ID を受けて塗り、1 点の ID を返す）として `layout/hit_region/` に足す。areka は並びから形の列を組んで渡し、名前を引く。あわせて `ColorMapData::from_image` の 16 番を直す。

- **触るクレート**: wintf（`layout/hit_region/` に新しいファイル・公開・文書・16 番の直し）・areka-emo-present・areka-emo-compose（述語と文書）・areka-parsers・areka-emo-atlas（同上）。
- **利点**: brief の層の分け方（仕組みは wintf・意味は areka）を GPU なしで保つ。B の利点（決定論・待ちなし・3.1 が構造で成り立つ）はそのまま。16 番を wintf の中で閉じられる。
- **欠点・危険**: wintf の使い手は今のところ areka だけで、areka の外の使い手が居ない仕組みを公開の口として作ることになる。既存の `HitRegionMap`（先に書いた方が勝つ・偶奇・正規化の座標）とは規則が逆で、同じフォルダに 2 つの流儀が並ぶ。wintf の座標は f32 の DIP が基本で、整数の作者の座標の型をどちらで持つかの決めが要る。塗りの関数を emo-compose（wintf に依らない層）から使えないので、形の答えのテストは wintf の語彙で wintf に置く。

### 候補にしないもの

- **形ごとに CPU の式で点を判定する**（ID の配列を持たない）: 開発者の裁定（2026-11-17「形は ID で塗る」）で外れた。点 1 つの仕事が並びの件数に比例したままで、要件 6.4 も満たさない。
- **形は GPU・`region` は CPU**（2 つの仕組み）: 重なりの順を 2 つの仕組みにまたがって解くことになる（仮定 2 が書く欠点と同じ）。
- **`ColorMapData` を `region` にそのまま使う**: 2.2 の表のとおり意味論が合わない。

## 5. 規模と危険

| 候補 | 規模 | 危険 | 理由 |
| --- | --- | --- | --- |
| A（GPU・wintf） | L（1〜2 週間・14〜18 タスクの見込み） | 高 | wintf に前例の無い公開の口（ID の描き先・縁のぼかしなしの塗り・マスクの置き方・読み分け）。`rect` の答えを保てるか・機種で縁が揺れないか・マスクを補間なしで置けるかが未確認。入口が `&EmoPresenter` であることと A-1 が両立しない。UI スレッドの同期の待ち |
| B（CPU・areka） | M（3〜7 日・9〜12 タスクの見込み） | 低〜中 | 純粋な関数とそのテストが中心。前例（`hit.rs`・`hit_import.rs`・`AlphaMask`）に沿う。危険は塗る仕事の予算と、楕円・円の式の縁の決め |
| C（CPU・wintf） | M〜L（5〜9 日・11〜14 タスクの見込み） | 中 | B に wintf の公開の口の設計と 16 番の直しが乗る。同じフォルダの既存の流儀との並び方の決めが要る |

- どの候補でも共通の仕事: `region` の画像の読みと記録・α の引き当て・`ShapesInactive` の後始末・台帳と §8・クローディアの実機の確かめ・`circle` と `region` の作り物の検体。
- brief の見立て（12〜16 タスク）は A の形に近い。

## 6. 設計の決めが要る項目（要件の討議へ・答えで作業が変わるもの）

1. **ID の描き先を GPU（A）で持つか、CPU のメモリ（B か C）で持つか**（仮定 1・未決 1）。材料: 逆変換は今すでに 1 本在り新しく書く分は 0（1 節）／色の合成は今 CPU で α は手元に在る／入口は `&EmoPresenter`／A は UI スレッドで 0.3〜0.8 ms の待ち・未確認が 3 つ（7 節）。A を採るなら A-1 か A-2 か（A-1 は本体に触る）。
2. **仕組みを wintf に置くか（C）、areka に閉じるか（B）**（開発者の 2026-10-15 の問い「wintf層か、areka層か」）。CPU で持つときの分かれ目。
3. **`rect` も同じ ID の描き先へ寄せるか**（仮定 2・未決 2）。B・C なら寄せても `rect` の答えは構造で変わらない。A なら 3.1 の確かめ（7 節の R1）が前提になる。
4. **ID の幅**（仮定 3・未決 3）。CPU なら `u32` で決まり、議題が消える。A なら 24 ビット（α を無視する的が D2D で使えることの確かめが要る）。
5. **色の絵の α をどこで引くか**（要件 4.1・仮定 8）。原寸の `AlphaMask` を作者の座標の点で引く形（今の資産）か、brief の Desired Outcome が書く GPU の読み戻し（`SurfaceReadback`）か。あわせて、表示中の 1 件がキャッシュに無いとき（`InvalidateCache` の直後）の答え。
6. **完了 `collision-geometry` の「α を参照しない」を改めてよいか**（未決 8）。改めても赤になる既存のテストは無い（2.6）。
7. **`region` の画像の半透明の画素**（要件 2.9・仮定 7）。掛ける前の色で厳密に比べる（`areka-emo-atlas` に乗算しない読みの口を足す）か、乗算済みから戻した色で比べるか、半透明は当てないか。
8. **申し送り 16 番の行方**（要件 5.17・未決 7）。どの候補でも `ColorMapData` を `region` に使い回さない見込みなので、「使い回すなら SSP で測ってから決める」という条件が外れる。⑴ wintf 自身の決めとしてこの仕様で直す（乗算しない読みへ替え、α = 0 を対応外にする・テスト 1 本）⑵ 直さずに残す（持ち主を決め直す）。C を採るなら ⑴ が自然。
9. **正典が沈黙する所を SSP で測るか**（仮定 4〜7・未決 4〜7）。`region` の画像の置き場所と大きさ・画像の外・透明な画素・楕円と円の縁・大きさ 0 の形。測るなら開発者の許しと排他テストの机が要る（完了 `collisionex-regions` の research.md 9 節・12 節と同じ手順）。測らないなら areka の決めとして §8 に書く。
10. **`ShapesInactive` を腕ごと消してよいか**（仮定 10・未決 10）。消すなら parsers の 4 ファイルとテスト 2 本に触る（2.5）。出口で捨てるだけにすると古い文言とテストが残る。形の数の知らせ（楕円 32・多角形 28 など）を別の重さ（`info!` や `debug!`）で残すかもここで決める。
11. **採寸の読み（`load_shell_target_quiet`）で `region` の画像を読むか**。同じ核を通るので、何もしなければ採寸でも読む（記録は黙る）。採寸は当たり判定を使わない。
12. **サーフェスの外形の外の点**（要件 1.10 と 3.1 の食い違い）。今の `rect` は外形を知らず、外形の外の点でも囲みの中なら名前を返す（マウスを掴んだまま窓の外へ動かしたときに起こる）。ID の描き先を外形の大きさで持つと、外は当たり無しになる。要件 4.1（外は α が 0）とは揃うが、8.4 の「`rect` の当たり方を変えない」の例外に足すかの決めが要る。あわせて、ID の描き先の大きさを何で決めるか（表示中の絵の原寸・形の囲み）。
13. **区別できる数の上限越え**（要件 3.8）。24 ビットでも `u32` でも、持ち込みの予算（2^20 件）より上限が大きく、実データでは届かない。上限を小さく注入できる形にしてテストするか、要件から外すか。
14. **塗る仕事の予算**（要件 6・負荷）。並びが大きい面（持ち込みで最大 2^20 件）と `region` 100 件以上の面で、面を切り替えた回の塗りに上限を置くか・越えたときの答えと記録。A でも B でも要る。
15. **台帳の `collisionex*` の行の状態**（仮定 11・未決 11）と、**点 1 つの判定の待ちの上限**（未決 12。B・C なら待ちは無いので問いが消える）。

## 7. 調べが要る項目（Research Needed・設計の段へ）

A を採るときだけ要るもの:

- **R1 `rect` の答えの一致**: 縁のぼかしなしで拡大つきの矩形を塗った ID の描き先が、`hit.rs` の既存の期待値（k=2・k=5/4 の縁・`scaled_five_branches_at_k2`／`_k125`）と全部同じになるか。WARP と実 GPU の両方で。
- **R2 楕円・多角形の縁の機種差**: `ALIASED` の楕円と巻き数の多角形の画素が WARP と実 GPU で完全に一致するか（`wintf-d2d-composite-rules` の research.md 10 節と同じやり方の試し）。
- **R3 マスクの画像の置き方**: 拡大つきで、補間なし・ID を混ぜずに置ける D2D の手段（`FillOpacityMask` の補間の有無・ID の色を焼いた絵の `NEAREST_NEIGHBOR`・`region` 100 件のときの GPU のメモリ）。
- **R4 α を無視する的**: `B8G8R8A8_UNORM`＋`D2D1_ALPHA_MODE_IGNORE` を描き先にできるか、ID の色が 8 ビットで 1 つも化けずに往復するか（brief の見立てで、未確認）。
- **R5 待ちの長さ**: ID と色の α を続けて読むときの UI スレッドの待ち（要件 6.5）。

候補に依らないもの:

- **R6 SSP の測り**（6 節の 9・開発者の許しが要る）。
- **R7 WIC の読みが色を変えないか**: PNG の gAMA・ICC などで R・G・B が書いた値から動かないか（指定色の完全一致の前提。抜き色の一致も同じ読み手に乗っているので、まず既存の抜き色の決めとテストを確かめる）。
- **R8 塗る仕事の重さの見積もり**: クローディア（楕円 32・多角形 28・面ごとの件数）と、作り物の `region` 100 件での、面を切り替えた回の所要。
- **R9 同じウェーブとの重なり**: `ShapesInactive` を消すと `crates/areka-parsers/src/shell/{model,decode,parse,mod}.rs` に触る。ウェーブ C23 の兄弟（`balloon-flip-ssp-review`・`audit-emo2-boot-frame`・`input-crate-split`）がこの 4 ファイルに触らないことを設計の前に確かめる。`areka-emo-atlas` に触る場合も同じ。

## 8. 設計の段への申し送り

- **分析の上での見立て**（決めではない）: 今のコードの形（入口が `&EmoPresenter`・色の合成が CPU・変形は窓全体の k だけ・逆変換は在る）では、候補 B がいちばん少ない新しいコードで要件 1〜4・6〜10 を満たし、`rect` の答えの保証と決定論のテストを構造で得られる。候補 A の利点（形ごとの変形・拡大したときの縁の細かさ）が効くのは、合成が D2D へ移った後か、形ごとの変形が要るようになった後である。開発者の案（「ヒットテストIDだけを持ったサーフェス」）が GPU の描き先を指すのか、ID を持つ絵であれば CPU でもよいのかは、要件の討議で確かめる。
- A を採る場合、要件 5 の文は部品の置き方（A-1・A-2）に合わせて書き直す所が出る（5.1・5.2・5.13）。B・C を採る場合、要件 5・6.5・7.3・7.4・9.5・10.7・10.8 を外すか書き直す。
- どの候補でも先に決める順: 6 節の 1 →（CPU なら）2 → 10 → 12 → 9（測るかどうか）。残りは推奨案つきでまとめて出せる。
- 引き継ぐ事実: `region` の画像は `build_core` で読み `EmoWorld` に載せて渡す道が `frame*` と本体に触らない／α は原寸の `AlphaMask` を作者の座標の点で引ける／`collision-geometry` の「α を参照しない」を固定するテストは無い／WARP の足場は wintf の中だけ／`ColorMapData` は `region` に合わない。

## 9. 要件の討議の前の仕分け（2026-11-20・6 節の 15 項目の行き先）

- **決まった（開発者の答えを待たない・`requirements.md` の「決め」）**: 6（色の絵の α で当たり無し＝2026-10-22 の裁定・決め 8）・8（申し送り 16 番はこの仕様で wintf 自身の決めとして直す・決め 12）・10（`ShapesInactive` は腕ごと消す・数の知らせは残さない・決め 10）・12（サーフェスの外の点は当たり無し＝決め 8 に含む）・15 の前半（台帳は `degraded` のまま・決め 11）。同じウェーブ C23 の 3 本は parsers の 4 ファイルにも `areka-emo-atlas` にも触らない（roadmap の C23 の行）＝7 節の R9 は済み。
- **議案（開発者に諮る）**: 1 と 2（ID の絵を GPU の描き先で持つか CPU のメモリで持つか・どの層に置くか）・9（正典が沈黙する所を SSP で測るか＝測るには開発者の許しが要る）。3（`rect` も寄せる）・4（ID の幅）・13（上限越え）・15 の後半（待ちの上限）は 1 の答えで決まる。7（`region` の半透明の画素）は 9 の測りの答えで決まる。
- **議案の結果（2026-11-20）**: 1 と 2 は**候補 A（GPU の ID の描き先を wintf に作る）**に決まった。討議では分析の見立てどおり候補 B（CPU・areka）を推したが、開発者が退けた（「D2D描画は捨てられず、CPU描画はすべての前提をひっくり返す。そうなるとSSPとの差別化が出来なくなる。CPUは許容できない。議論の余地なし。」）。8 節の「分析の上での見立て」（候補 B）は採らない。候補 B・C を前提にした記述（4 節・5 節・下の「設計の段」の 1〜2 のうち CPU の配列の話）は材料として残すだけで、設計は候補 A で組む。これに伴い 3（`rect` も寄せる）・4（24 ビットの 1 つ）・仮定 9（読めないときは当たり無し・CPU の判定へ落とさない）も決まった（`requirements.md` の決め 1〜3・9）。9（SSP の測り）は承認された（決め 13・結果は下の節に足す）。
- **候補 A で設計の段が先に決めること**: 部品の置き方（4 節の A-1＝描画面の持ち主の実体に付く部品と `SystemParam` の読み口／A-2＝presenter が持つ単体の入れ物）。本質的な形を選び、ウェーブ C23 の約束 ⑶（本体と `frame*` に触らない）を理由に形を曲げない。A-1 で入口（`resolve_hit_region`・本体の `input_events/`）の形が変わるなら、同じウェーブの `input-crate-split`（`input_events/` を新しい crate へ移す）のセッションと触り方を突き合わせる。色の絵の α の出どころ（GPU の読み戻し `SurfaceReadback` か、今の CPU の `AlphaMask` か）もここで決める（CPU のマスクは `draw-methods-canon` の着地で出どころが無くなる）。7 節の R1〜R5 の試しは設計の段で行う。`tech.md` の WARP の例外の列に ID の描き先を足す直しも要る。
- **D3D11 を使う形（開発者 2026-11-20「目的はヒットテストです。D3D11も利用してヒットテストを実装すべきかも。確か、１ピクセルだけCPUに書き戻す方法があるよね？」）**: 候補 A の中の分かれ目として設計の段で比べる。⑴ D2D で `B8G8R8A8`（α なし）へ縁のぼかしなしで塗り、今ある `SurfaceComposite::read_pixel` と同じ形（1×1 の `CPU_READ` へ `CopyFromBitmap` → `Map`）で読む。⑵ D3D11 の整数の描き先（`R32_UINT` など）へ ID を描き、`CopySubresourceRegion`（1×1 の `D3D11_BOX`）で `D3D11_USAGE_STAGING` のテクスチャへ写して `Map` で読む。⑵ は ID が本物の整数になり、7 節の R4（α を無視する的と色の往復）が要らなくなる。代わりに D2D は整数の的へ描けないので、形の塗り（楕円・巻き数の多角形・マスクの画像）を D3D11 の側でどう描くか（D2D の形を三角形に割る・自前のシェーダー）と、wintf に D3D11 で描く道が今あるかの調べが要る。⑴ と ⑵ の折衷（D2D で描いた的を D3D11 の口で読む）も在る。待ちの長さ（R5）は ⑴ ⑵ とも実 GPU で測る。
- **SSP の測り（10 節）の後の決め（2026-11-20・議題 3〜5 は開発者が推奨どおり承認）**: `region` の置き方・色の完全一致・領域反転と画像の外・読めない画像は下書きの仮定のまま確定。透明な画素（α が 0 は黒として比べる・α が 1 以上は掛ける前の色）・形の端（左と上を含み右と下を含まない・`rect` も改める）・逆向きの座標と負の半径（並べ直す・絶対値）は SSP に合わせる。SSP と食い違わせるのは、`polygon` の左と上の辺の上の 1 画素（1 つの規則で通す）・全部が指定色の画像（当てる）・重なりの順（画家の順の裁定のまま）の 3 つ。`requirements.md` の決め 4〜7・14 が正本。
- **形の端の規則と GPU の塗り**: 決め 5 の規則（形の座標をそのまま置き、画素の中心が中なら当たる）は、縁のぼかしなしの塗りの規則と同じ向きなので、7 節の R1 は「今の閉区間の答えと合うか」ではなく「SSP の測りの S8・S9 の点（`rect` 100〜199・`ellipse` 100〜199・`circle` 110〜189・`polygon` 20〜79）と合うか」を確かめる。拡大率が 1 でないときは、サーフェスの 1 画素に当たる画面の画素がどれも同じ答えになること（SSP は画像の画素ごとに決まる＝10 節の読み 3）を確かめる。物理ピクセルの細かさで形を塗ると、拡大率が整数でないときに端の画面の画素が隣のサーフェスの画素の答えを持ちうるので、ID の絵をサーフェスの原寸で塗って点の側を戻すか、物理の大きさで塗るかをここで決める。
- **`input-crate-split` との取り決め（2026-11-20・セッション間の相談）**: 向こうは本体の `crates/areka/src/input_events/mod.rs`（`resolve_hit_region` の唯一の呼び手）を中身を変えずに `crates/areka-input/src/lib.rs` へ移す。こちらはこのウェーブのあいだ本体の `input_events/mod.rs` に触らない。`resolve_hit_region` の署名を変えると設計で決まったら、向こうへ知らせ、呼び手の追従は向こうの着地の後に `crates/areka-input/src/lib.rs` で行う（C23 の約束 ⑶ の「本体」に `areka-input` は入らない、という読みで両者が合意・設計の討議で開発者に報告する）。`crates/areka-runtime/src/hit_region.rs` は、向こうが冒頭の説明の 8・18・20 行、こちらが 14・36 行あたり。
- **`areka-emo-atlas` に触る**: `region` の画像を、色に α を掛けない形で読む口を足す（決め 7・2.7 節）。同じウェーブ C23 の 3 本はこの crate に触らない。
- **設計の段（`/kiro-design`）で決める**:
  1. ID の絵を持つ所と作り直す時点（表示中の面だけ持つか・面ごとに覚えるか）。
  2. 色の絵の α を引けないとき（表示中の 1 件が `InvalidateCache` で消えた直後）の答え（6 節の 5 の後半）。
  3. 採寸の読み（`load_shell_target_quiet`）で `region` の画像を読むか（6 節の 11）。
  4. 塗る仕事の予算（持ち込みで並びが大きい面・`region` 100 件以上の面）と、越えたときの答えと記録（6 節の 14・7 節の R8）。
  5. WIC の読みが R・G・B を変えないことの確かめ（7 節の R7）。
  6. `circle` と `region` の作り物の検体の置き方（クローディアはどちらも持たない）。

## 10. SSP の測り（2026-11-20・要件の討議の議題 2）

- **環境**: SSP（`C:\wintools\ssp\ssp.exe`・pid 29816）。開発者の許し（2026-11-20）を得て、排他テストの机を取って測った。測りに使ったのは `．さくら`（`dot_sakura`・「SHIORI呼出ログ」が有効）で、`えも2DEBUG` の横へ `\![call,ghost,．さくら]` で呼び、試しのシェル `shell\hit-id-probe\`（300×300 の不透明な一色の画像 `surface0.png`・`seriko.use_self_alpha,1`）を `\![change,shell,hit-id-probe]` で着せた。試しの組は面 100〜110 に 1 組ずつ置き、組ごとに `\0\s[N]\1\s[-1]\e`（kero は隠す）を送ってから測った。点は `SendInput`（絶対座標・仮想デスクトップ・移動だけ）で、まず窓の中の別の点 (5,295) へ動かしてから目当ての点へ動かし、`ghost\master\ssp_shiori_log.txt` に増えた分の最後の `OnMouseMove` の Reference0・1・4 を読んだ。動かす前に毎回 `WindowFromPoint` で点の下が pid 29816 の `．さくら` の本体の窓（`Tmainform`）であることを確かめた。SSP の版は `baseware.version` で `SSP/2.9.34 (20261113-4; Windows NT 10.0.26300)`。
- **拡大率**: `currentghost.scope(0).rect` が `2352,1104,2952,1704`（幅 600）で、`EnumWindows` で見た本体の窓の矩形と同じ。画像の幅 300 で割って 2。画像の点 (x,y) は画面の (2352+2x+d, 1104+2y+d) に当たる。Reference0・1 はどの点でも画像の座標（画面の点ではない）だった。端の点は d=0 と d=1（画像の 1 点に当たる画面の 2 点の最初と最後）の両方を測り、どの点でも 2 つの答えは同じだった（表では「d=0/1」と書く）。
- **画面の外**: 窓の右端は画面の外にはみ出ていた（画面の右端が x=2880）。画像の x が 264 以上の点は測れなかったので、頼まれた点のうち (299,299)・(290,40) は (263,290)・(263,40) に替えた。
- **測りの途中の崩れ**: `．さくら` は `OnSurfaceRestore` に `\u\s[10]\w8\w8\h\s[0]\e` を返す。そのため面を変えて十数秒たつと面が 0 へ戻り、最初の 3 回の読みが崩れた（`get_property` で `scope(0).surface.num` が 0）。その後は、点を読むたびにログに `OnSurfaceRestore` が来ていないかを見て、来たら読みを捨てて組を測り直した。表に書いたのは崩れなかった読みだけ。
- **追加の組**: 一色だけの画像（`map_small.png`）が読み込みで「Color not found」になったので、その理由を分ける 2 組と、重なったときの順と画像が無いときの扱いを分ける 3 組を足した。シェルの一覧は呼んだ後に足したフォルダを拾わなかった（`\![change,shell,hit-id-probe-b]` は何も起きなかった）。そのため同じ `hit-id-probe` のフォルダの `surfaces.txt` と画像を入れ替え、`master` へ着替えてから `hit-id-probe` へ着替え直して読ませた。足した組の本文は `evidence/ssp-probe-b-surfaces.txt`。
- **後片付け**: `．さくら` を `master` のシェルへ戻して（`get_property` で `master` を確かめた）`\-` で閉じ、起動中のゴーストが `えも2DEBUG` だけに戻ったことと pid 29816 が生きていることを確かめた。試しのシェルのフォルダ（SSP が中に作った `profile` ごと）と、使わずに終わった `hit-id-probe-b` を消し、カーソルを始めの位置 (1001,578) へ戻し、机を返した。`えも2DEBUG` には呼び出しの 1 回のほか何も送っていない。
- 試しのシェルの本文は `evidence/ssp-probe-surfaces.txt` と `evidence/ssp-probe-b-surfaces.txt`、画像は `evidence/ssp-probe-images.py` で作り直せる（8bit RGBA・IHDR/IDAT/IEND だけ・書いた後で読み直して画素を確かめる）。

### S0 置き方と色の幅（面 100・`collisionex0,RegRed,region,map_full.png,255,0,0`・画像は 300×300 の白地に赤 (255,0,0) x20〜79,y20〜79・赤より 1 だけ暗い (254,0,0) x100〜159・緑 x180〜239）

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (50,50) 赤 | 赤の塊 | `RegRed` |
| (130,50) (254,0,0) | 赤より 1 だけ暗い塊 | 空 |
| (210,50) 緑 | 緑の塊 | 空 |
| (250,250) 白 | 地 | 空 |
| (19,50) d=0/1 | 赤の左の外 | 空 |
| (20,50) d=0/1 | 赤の左端 | `RegRed` |
| (79,50) d=0/1 | 赤の右端 | `RegRed` |
| (80,50) d=0/1 | 赤の右の外 | 空 |
| (50,19) d=0/1 | 赤の上の外 | 空 |
| (50,20) d=0/1 | 赤の上端 | `RegRed` |
| (50,79) d=0/1 | 赤の下端 | `RegRed` |
| (50,80) d=0/1 | 赤の下の外 | 空 |

### S1 面より小さい画像（面 101・`collisionex0,RegSmall,region,map_small.png,0,0,255`・画像は 100×100 で全部が青）

読み込みで「Color not found」の誤り（エラーログ #136）。

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (0,0)・(10,10)・(50,50) | 画像の中（青） | 空 |
| (99,99) d=0/1 | 画像の右下の端（青） | 空 |
| (100,100) d=0/1 | 画像の外 | 空 |
| (150,150)・(250,250)・(50,250)・(250,50)・(263,290) | 画像の外 | 空 |

### S1b 一色でない小さい画像（追加・`collisionex0,RegSmallTl,region,map_small_tl.png,0,0,255`・100×100 の青で (0,0) の 1 点だけ白）

読み込みの誤りは出なかった。

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (50,50)・(1,0)・(99,99) | 画像の中（青） | `RegSmallTl` |
| (0,0) | 白の 1 点 | 空 |
| (100,100)・(150,150) | 画像の外 | 空 |

### S1c 左上の 1 点が指定の色の画像（追加・`collisionex0,RegRedTl,region,map_red_tl.png,255,0,0`・300×300 の白地に赤の塊 x20〜79,y20〜79 と (0,0) の赤 1 点）

読み込みの誤りは出なかった。

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (0,0) | 左上の赤 1 点 | `RegRedTl` |
| (1,0) | 白 | 空 |
| (50,50) | 赤の塊 | `RegRedTl` |
| (80,50) | 白 | 空 |

### S2 面より大きい画像（面 102・`collisionex0,RegLarge,region,map_large.png,255,255,0`・400×400 の白地に黄 x20〜79,y20〜79 と x320〜379,y20〜79）

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (17,17)・(19,19) d=1 | 黄の外 | 空 |
| (20,20) d=0 | 黄の左上の端 | `RegLarge` |
| (22,22)・(50,50)・(65,65)・(70,70)・(78,78) | 黄の中 | `RegLarge` |
| (79,79) d=1 | 黄の右下の端 | `RegLarge` |
| (80,80) d=0 | 黄の外 | 空 |
| (250,40)・(263,40) | 白（2 つ目の黄は x320〜で面の外） | 空 |
| (150,150) | 白 | 空 |

### S3 領域反転と小さい画像（面 103・`collisionex0,RegInv,region,map_small_red.png,255,0,0,true`・100×100 の白地に赤 x20〜59,y20〜59）

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (30,30) | 画像の中の赤 | 空 |
| (19,30) d=1 | 画像の中の白（赤の左の外） | `RegInv` |
| (20,30) d=0 | 赤の左端 | 空 |
| (59,30) d=1 | 赤の右端 | 空 |
| (60,30) d=0 | 画像の中の白（赤の右の外） | `RegInv` |
| (10,10)・(80,80) | 画像の中の白 | `RegInv` |
| (99,99) d=1・(99,50) d=1 | 画像の中の白（画像の右端） | `RegInv` |
| (100,100) d=0・(100,50) d=0 | 画像の外 | 空 |
| (150,150)・(250,250)・(50,150) | 画像の外 | 空 |

### S4〜S6 透明な画素（面 104〜106・画像 `map_alpha.png`＝300×300・地は RGBA (0,0,0,0)）

塊の RGBA: A (255,0,0,0)・B (255,0,0,128)・C (255,0,0,255)・D (255,0,0,1)・E (255,0,0,254)・F (255,255,255,0)。点は塊の中心。

| 点（画像の座標） | 画素の RGBA | S4 `region,…,255,0,0` | S5 `region,…,255,0,0,true` | S6 `region,…,0,0,0` |
| --- | --- | --- | --- | --- |
| (49,49) A | (255,0,0,0) | 空 | `RegAlphaInv` | `RegBlack` |
| (129,49) B | (255,0,0,128) | `RegAlpha` | 空 | 空 |
| (209,49) C | (255,0,0,255) | `RegAlpha` | 空 | 空 |
| (49,149) D | (255,0,0,1) | `RegAlpha` | 空 | 空 |
| (129,149) E | (255,0,0,254) | `RegAlpha` | 空 | 空 |
| (209,149) F | (255,255,255,0) | 空 | `RegAlphaInv` | `RegBlack` |
| (250,250) 地 | (0,0,0,0) | 空 | `RegAlphaInv` | `RegBlack` |

### S7 画像が無い（面 107・108 と追加の 3 組）

| 組 | 当たり判定 | 点（画像の座標） | Reference4 |
| --- | --- | --- | --- |
| 面 107 | `collisionex0,Under,rect,20,20,280,280`・`collisionex1,RegMissing,region,nothing.png,255,0,0` | (150,150)・(50,50) | `Under` |
| 面 108 | `collisionex0,Under,rect,…`・`collisionex1,RegMissingInv,region,nothing.png,255,0,0,true` | (150,150)・(50,50) | `Under` |
| 追加（番号の順を逆に） | `collisionex0,RegMissing,region,nothing.png,255,0,0`・`collisionex1,Under,rect,20,20,280,280` | (150,150)・(50,50) | `Under` |
| 追加（番号の順を逆に・領域反転） | `collisionex0,RegMissingInv,region,nothing.png,255,0,0,true`・`collisionex1,Under,rect,…` | (150,150)・(50,50) | `Under` |
| 同上の 2 組 | — | (10,10) | 空 |
| 追加（重なりの順） | `collisionex0,Under,rect,20,20,280,280`・`collisionex1,RegRed,region,map_full.png,255,0,0` | (50,50)（赤の上）・(150,150) | `Under` |

### S8 矩形・楕円・多角形の端（面 109・参考）

`collisionex0,Rc,rect,100,200,200,260`・`collisionex1,Ell,ellipse,100,100,200,160`・`collisionex2,Poly,polygon,20,200,80,200,80,260,20,260`

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (100,230) d=0/1 | rect の左端 | `Rc` |
| (99,230) | rect の左の外 | 空 |
| (199,230) | rect の右から 1 つ内 | `Rc` |
| (200,230) d=0/1・(201,230) | rect の右の値とその外 | 空 |
| (150,200) d=0/1 | rect の上端 | `Rc` |
| (150,199) | rect の上の外 | 空 |
| (150,259) | rect の下から 1 つ内 | `Rc` |
| (150,260) d=0/1・(150,261) | rect の下の値とその外 | 空 |
| (150,130) | ellipse の中心 | `Ell` |
| (100,130) d=0/1 | ellipse の左端 | `Ell` |
| (99,130) | ellipse の左の外 | 空 |
| (199,130) | ellipse の右から 1 つ内 | `Ell` |
| (200,130) d=0・(201,130) | ellipse の右の値とその外 | 空 |
| (150,100) d=0/1 | ellipse の上端 | `Ell` |
| (150,99) | ellipse の上の外 | 空 |
| (150,159) | ellipse の下から 1 つ内 | `Ell` |
| (150,160) d=0・(150,161) | ellipse の下の値とその外 | 空 |
| (103,103)・(115,108) | 外接矩形の中で楕円の外 | 空 |
| (50,230) | polygon の中 | `Poly` |
| (20,230) d=0/1・(19,230) | polygon の左の辺の上とその外 | 空 |
| (21,230) d=0 | polygon の左の辺から 1 つ内 | `Poly` |
| (79,230) d=1 | polygon の右の辺から 1 つ内 | `Poly` |
| (80,230) d=0/1・(81,230) | polygon の右の辺の上とその外 | 空 |
| (50,200) d=0/1・(50,199) | polygon の上の辺の上とその外 | 空 |
| (50,201) d=0 | polygon の上の辺から 1 つ内 | `Poly` |
| (50,259) d=1 | polygon の下の辺から 1 つ内 | `Poly` |
| (50,260) d=0/1・(50,261) | polygon の下の辺の上とその外 | 空 |

（polygon の辺から 1 つ内の点と rect・ellipse の「1 つ内」の点は、頼まれた点の答えに驚いたので足した。）

### S9 円と崩れた形（面 110・参考）

| 点（画像の座標） | 当たり判定 | Reference4 |
| --- | --- | --- |
| (150,150) | `Cir,circle,150,150,40` の中心 | `Cir` |
| (110,150) d=0/1・(150,110) d=0/1 | 円の左端・上端（中心−半径） | `Cir` |
| (109,150)・(150,109) | その外 | 空 |
| (189,150)・(150,189) | 中心＋半径から 1 つ内 | `Cir` |
| (190,150) d=0・(150,190) d=0 | 中心＋半径 | 空 |
| (191,150)・(150,191) | その外 | 空 |
| (178,178) | 斜め（中心からの距離 39.6） | `Cir` |
| (179,179) | 斜め（中心からの距離 41.0） | 空 |
| (50,50) d=0/1・(51,50) | `Cir0,circle,50,50,0` | 空 |
| (50,250)・(40,250)・(30,250)・(69,250)・(50,230) | `CirNeg,circle,50,250,-20` | `CirNeg` |
| (29,250)・(70,250)・(50,229)・(50,270) | 同上の外 | 空 |
| (250,50) d=0/1・(249,50)・(251,50) | `EllThin,ellipse,250,20,250,80`（幅 0） | 空 |
| (260,130)・(240,130)・(260,159) | `EllInv,ellipse,280,160,240,100`（左上と右下を逆に） | `EllInv` |
| (239,130)・(260,160)・(259,100)・(259,99) | 同上の外と上端の近く | 空 |
| (150,280)・(150,279)・(150,281) | `PolyLine,polygon,120,280,180,280,150,280`（面積 0） | 空 |
| (260,230)・(240,230)・(260,200)・(260,259) | `RcInv,rect,280,260,240,200`（左上と右下を逆に） | `RcInv` |
| (239,230)・(260,199)・(260,260) | 同上の外 | 空 |

### SSP のエラーログ

シェルを着せたときに出たのは次の行（ほかに `Surface=20 どこからも使われていません。` の Notice が着替えのたびに出た）。

- #136 [Error] `shell\hit-id-probe\surfaces.txt:Line=18:Collision=0 定義ミスです。 [Color not found]`（S1・一色だけの画像）
- #137・#138 [Error] `…Line=49:Collision=1 定義ミスです。 [File not found]`・`…Line=55:Collision=1 …[File not found]`（S7 の面 107・108）
- #139 [Warning] `…Line=68:Collision=1 定義ミスです。 [Empty rectangle]`（`Cir0`・半径 0）
- #140 [Warning] `…Line=70:Collision=3 定義ミスです。 [Empty rectangle]`（`EllThin`・幅 0 の楕円）
- #141 [Warning] `…Line=72:Collision=5 定義ミスです。 [Empty rectangle]`（`PolyLine`・面積 0 の多角形）
- #180・#181 [Error] `…Line=29:Collision=0 定義ミスです。 [File not found]`・`…Line=35:Collision=0 …[File not found]`（追加の 2 組）
- 半径が負の円・左上と右下が逆の楕円・左上と右下が逆の矩形・`map_small_tl.png`・`map_red_tl.png`・透明な画素を含む `map_alpha.png` には何も出なかった。

### 読み（SSP がすること）

1. `region` の画像は面の左上 (0,0) に合わせて 1:1 で置かれる。伸び縮みはしない。面より大きい画像は面の外の部分が使われず、面より小さい画像は画像の外が「指定色でない」とも数えられない。
2. 色は R・G・B がちょうど同じときだけ当たる（(254,0,0) は (255,0,0) に当たらない）。
3. 当たりは画像の画素ごとに決まる。拡大率 2 で、画像の 1 点に当たる画面の 2 点はいつも同じ答え。Reference0・1 は画像の座標。
4. 領域反転は「画像の中で指定色でない画素」だけが当たる。画像の外（面より小さい画像の残り）は当たらない。
5. α=0 の画素は、蓄えた R・G・B によらず (0,0,0) として数えられる（(255,0,0,0) も (255,255,255,0) も黒に当たり、赤に当たらない）。α が 1 以上なら α の値によらず蓄えた R・G・B で数えられる（α=1・128・254・255 はみな赤に当たる）。領域反転では α=0 の画素は「指定色でない」ので当たる。
6. 画像の全部が 1 色のとき、その色を指定しても「Color not found」の誤りになり、当たりは無い。1 点でも別の色があれば読まれる。左上の画素を抜き色とする扱いはない（左上の 1 点の赤も当たる）。
7. 画像が無いと「File not found」の誤りになり、その当たり判定は無いものとして扱われる（領域反転でも当たらない・番号が前でも後ろの rect の当たりを塞がない）。
8. 当たり判定が重なると番号の小さいほうの名前が返る（`collisionex0` の rect と `collisionex1` の region の重なりで rect の名前）。
9. rect と ellipse は左・上の値を含み、右・下の値を含まない（rect 100〜199・200〜259／ellipse 100〜199・100〜159）。左上と右下が逆の rect・ellipse は並べ直されて当たる。
10. polygon（軸に沿った正方形）は辺の上の点を 4 辺とも含まない（20〜80 の正方形で 21〜79 だけが当たる）。
11. circle は中心−半径を含み、中心＋半径を含まない（x 110〜189）。外接矩形 (cx−r, cy−r, cx+r, cy+r) の楕円と同じ当たり。負の半径は絶対値の半径として当たる（警告なし）。
12. 半径 0 の circle・幅 0 の ellipse・面積 0 の polygon は「Empty rectangle」の警告を出し、当たらない。

### 測れなかった所

- 画像の x が 264 以上の点（窓の右端が画面の外）。面より大きい画像のうち面の外の部分が当たらないことは、面の外へはマウスが行けないので測りようがなく、面の中の点だけで読んだ。
- `region` を大量に（100 件〜）置いたときの重さ。
- `region` の画像に PNG 以外（BMP など）や、パレット・16bit の PNG を使ったとき。色を `.pna` と組み合わせたとき。
- `region` の画像が面より小さいとき、画像を面の左上以外へずらす手段（ukadoc に書式が無く、試していない）。
- 楕円・円の周の上の点を斜めの向きで細かく（S9 の斜めは 2 点だけ）。

## 11. 設計の段の調べと試し（2026-11-20・`/kiro-design` の設計の生成）

- **調べの形**: 全部の調べ（新しい公開の口と骨組みの分かれ目が在るため）。外の資料は引かず、コードの読みと手元の試しで決めた。使った決まりは `kiro-spec-design` の `design-principles.md`・`design-discovery-full.md`・`design-synthesis.md`・`design-review-gate.md` と、`CLAUDE.md` の Comments の節（コードの説明に spec の名前・要件の番号・タスクの番号を書かない）。
- **試しの置き場**: wintf の中の使い捨てのテスト 1 本（`crates/wintf/src/ecs/graphics/` に置き、`cargo test -p wintf --lib hit_id_probe -- --nocapture --test-threads=1` で名指しして回した）。WARP は `emit_test_support` の装置、実 GPU は `GraphicsCore::new()` の装置（開発機の内蔵 GPU・デバッグビルド）。窓も画面も使っていない。数字を採った後で消した（木に残していない・コミットしていない）。
- **結論**: ID の描き先は、**D2D で塗る `B8G8R8A8` の描き先を、サーフェスの原寸で、presenter が持つ wintf の入れ物として持つ**。読み戻しは D2D の 1×1 の写し先。色の絵の α はクリックの透過と同じマスクで引く。`resolve_hit_region` の署名は変わらない。

### 11.1 R1 縁のぼかしなしの塗りと SSP の測り（拡大率 1・WARP と実 GPU で同じ答え）

塗り方: `D2D1_ANTIALIAS_MODE_ALIASED`・不透明の ID の色・書いた座標のまま（`FillRectangle`・`FillEllipse`・巻き数の path geometry の `FillGeometry`）。

| 形 | 塗られた画素 | SSP の測り |
| --- | --- | --- |
| `rect,100,200,200,260` | x 100〜199（100 画素）・y 200〜259 | 同じ |
| `ellipse,100,100,200,160` | y=130 の行で x 100〜199 | 同じ |
| `circle,150,150,40` | y=150 の行で x 110〜189 | 同じ |
| `polygon,20,200,80,200,80,260,20,260` | x 20〜79 | SSP は 21〜79（決め 5 のとおり左と上の辺の 1 画素だけ広い） |
| 半径 0 の円・幅 0 の楕円と矩形・一直線の多角形 | 0 画素 | 同じ（当たらない） |
| 五芒星（巻き数） | 中心は塗られ、凸包の中で星の外は塗られない | 同じ |

- 混ざった画素（ID でも透明でもない画素）は 0。
- **SSP と合わない点が 3 つ見つかった**（多角形の辺のほか）: (115,108)＝楕円 `100,100,200,160` で D2D は中・SSP は外／(178,178)＝円 `150,150,40` で D2D は外・SSP は中／(259,100)＝楕円 `240,100,280,160` で D2D は中・SSP は外。画素の中心で決める規則（要件 1.5）では、(115.5,108.5) は楕円の中（正規化した半径の 2 乗 0.9897）、(178.5,178.5) は円の外（中心から 40.3）、(259.5,100.5) は楕円の中（0.9676）で、D2D の答えは規則のとおり。SSP の答えは「画素の左上の角の点が形の中か（左と上の周の上は含む）」と読むと、10 節の S8・S9 の楕円と円の点が全部合う（SSP の当たる所は、規則の当たる所を右下へ半画素ずらした形）。→ 要件 10.1 の「多角形の辺の上の点だけ規則の答えにする」は 3 点ぶん足りない（設計の未決の議題 1）。
- D2D の楕円は式の楕円そのものではない: 画素の中心の規則（f64 の式）と比べて 1 つの楕円につき 8〜18 画素が違う。どれも周から 0.1 画素以内（D2D が曲線を折れ線へ割る誤差）。細かく割った多角形（`Simplify`・許容 0.01 と 0.001）で塗ると、実 GPU は式と 0 画素の違いになるが、WARP は 3〜8 画素違ったままで、装置の差は消えない。→ `FillEllipse` のまま使う。

### 11.2 拡大つきで物理ピクセルに塗る形（採らなかった形の測り）

物理ピクセルの画素 v の答えを、拡大率 1 の絵の `unscale_coord(v)` の答えと比べた（違う画素の数）。

| 拡大率 | 矩形・軸に沿った多角形 | 楕円 `100,100,200,160` | 円 `150,150,40` | 五芒星 |
| --- | --- | --- | --- | --- |
| 2 | 0 | 224〜226 | 196〜204 | 314〜320 |
| 5/4 | 0 | 88 | 94〜102 | 76〜81 |
| 3/2 | 0 | 131〜137 | 146 | 219〜222 |
| 11/10 | 0 | 52〜56 | 88〜96 | 127 |

- 矩形は試した座標（偶数）で全部一致。ただし画素の中心がちょうど辺に載る並び（例: 拡大率 11/10 で左端が 5・15・…）は試していない。拡大率が 2 のべき乗で割り切れない値では f32 の拡大率が正確でないので、その並びで端の画面の画素が 1 つずれうる。
- 曲線は物理ピクセルの細かさで決まるので、サーフェスの画素ごとの答え（SSP の決め方）とは一致しない。

### 11.3 R3 マスクの画像の置き方（100×100 の乱数のマスク・(20,30) に置く・下に別の ID の矩形）

| 置き方 | 拡大率 1 | 2 | 5/4 | 3/2 | 11/10 |
| --- | --- | --- | --- | --- | --- |
| `FillOpacityMask`（BGRA のマスク） | 誤り 0 | 混ざり 34,588 | 13,458 | 19,383 | 10,346 |
| `FillOpacityMask`（8 ビットの α のマスク） | 誤り 0 | 混ざり 34,588 | 13,458 | 19,383 | 10,346 |
| ID の色を焼いた絵を最近傍で `DrawBitmap` | 0 | 0 | 0 | 0 | WARP 1,087 画素が戻した点とずれ（実 GPU 0） |
| 最近傍のビットマップの筆を層の不透明度の筆に | 0 | 0 | WARP 1,529 | WARP 6,244 | WARP 571（実 GPU はどれも 0） |

- `FillOpacityMask` は拡大つきで補間する（ID が混ざる）。拡大率 1（変換が恒等）では、面の中・負の位置 (-10,250)・はみ出す位置 (250,-40) とも誤り 0（WARP・実 GPU）。
- 物理ピクセルで置ける形は「ID の色を焼いた絵を最近傍で描く」だけで、`region` 1 件ごとに絵が要る。
- 手間（300×300 のマスク 100 件・実 GPU）: 8 ビットの α の絵の作成 約 170〜180 ms＋`FillOpacityMask` の塗りと読み 約 8 ms（WARP 約 22 ms）。ID の色を焼いた絵で置く形は 約 750 ms。

### 11.4 R2 WARP と実 GPU の画素の一致（拡大率 1）

| 形 | 違う画素 | 中身 |
| --- | --- | --- |
| 矩形・軸に沿った多角形・マスク | 0 | — |
| 楕円 `100,100,200,160`・円 `150,150,40` | 0 | — |
| 円 `50,250,20` | 1 | (32,259)＝周から 0.088 画素 |
| 楕円 `240,100,280,160` | 2 | (240,123)・(240,136)＝周から 0.024 画素 |
| 頂点が整数の三角形 | 0 | 両方とも式の巻き数の規則と 0 画素の違い |
| 頂点が整数の五芒星（辺が交差） | 9 | WARP は式の規則と一致・実 GPU が 9 画素違う |

- 拡大つきでは 1 枚あたり 4〜26 画素が違った（11.2 の絵・曲線と五芒星）。
- → 完全には一致しない。違うのは、画素の中心が周から約 0.1 画素以内の画素と、辺が交差する多角形の周の画素。同じ装置では毎回同じ答え。決定論のテストの点は周から 0.25 画素より離して選ぶ。

### 11.5 R4 ID の色の往復と整数の的

- 2,819 個の ID（1〜2,048・各成分 1〜255 の単色と灰色・`0xFFFFFF`・`0xFFFFFE`・2^20 の前後）を 1×1 の矩形で塗って読み戻し、化けた ID は 0（WARP・実 GPU、α 掛け済みの的と `D2D1_ALPHA_MODE_IGNORE` の的の両方）。読んだ α は全部 255。
- D2D の的を整数の形式の DXGI の面から作る試し: `R32_UINT`・`R8G8B8A8_UINT`・`R16_UINT` とも `CreateBitmapFromDxgiSurface` が `0x88982F80`（ビットマップのピクセル形式を扱えない）。→ D3D11 の整数の描き先へ D2D では描けない。

### 11.6 R5 1 ピクセルの読み戻しの待ち（実 GPU・512×512・`B8G8R8A8`・各 300 回）

| 読み方 | GPU が空いているとき（中央値・95% 点・最大） | 別の絵を描いた直後 |
| --- | --- | --- |
| D2D 1 回（1×1 の `CPU_READ` へ `CopyFromBitmap` → `Map`） | 396 µs・605 µs・2.89 ms | 405 µs・586 µs・0.99 ms |
| D2D 2 回（ID と色を別々に） | 817 µs・1,231 µs・1.67 ms | 722 µs・1,068 µs・1.35 ms |
| D2D で 2 枚を 2×1 の写し先へ写して `Map` 1 回 | 360 µs・569 µs・0.94 ms | 446 µs・714 µs・1.16 ms |
| D3D11 1 回（`CopySubresourceRegion` 1×1 → `Map`・D2D の錠の中） | 304 µs・441 µs・0.59 ms | 393 µs・552 µs・1.45 ms |
| D3D11 で 2 枚を 2×1 へ写して `Map` 1 回 | 364 µs・516 µs・0.94 ms | 417 µs・590 µs・0.80 ms |

- 待ちは GPU との同期が占め、D2D と D3D11 の口の差は 0.1 ms ほど。2 枚を読むなら 1 度の `Map` にまとめると 1 回ぶんで済む。
- 完了 `wintf-d2d-composite-rules` の記録（合成の直後 516 µs・747 µs／空いているとき 344 µs・556 µs）と同じ帯。

### 11.7 塗る手間（375×375 の的・塗り＋`EndDraw`＋ 1 ピクセルの読み）

| 件数 | 矩形（実 GPU／WARP） | 楕円 | 5 角形の多角形（図形の作成を含む） |
| --- | --- | --- | --- |
| 100 | 5.3／1.4 ms（最初の回の暖機を含む） | 1.7／1.8 ms | 4.6／1.4 ms |
| 1,000 | 1.5／1.9 ms | 3.9／3.3 ms | 19.8／9.2 ms |
| 10,000 | 10.7／7.8 ms | 27.8／24.0 ms | 178.6／38.0 ms |
| 100,000 | 77.9／62.1 ms | 187.3／205.7 ms | 1,622.6／336.0 ms |
| 2^20 | 663.3／695.8 ms | 2,011.6／2,515.5 ms | — |

- 大きな形: 半径 2^16・2^20・2^24 の円（面の真ん中を周が通る）は 2〜5 ms で塗れ、失敗しない（半径 2^24 では中の画素が 46,044／理想 45,000 で、f32 の精度ぶんずれる）。頂点が ±2^24 の三角形と ±10^9 の矩形も 約 3 ms で、矩形は面の中の部分がちょうど塗られる。

### 11.8 まとめ直し（一般化・作るか使うか・削る）

- **一般化**: 5 つの形と `collision*` の矩形は「形を ID で上書きして塗る」1 つの口に載る。`region` は「マスクが立っている画素に ID を置く」形で同じ口に載る。wintf の口は形の列と ID だけを受け、areka の語を持たない。
- **作るか使うか**: 形の内外は D2D の塗り（矩形・楕円・巻き数の path geometry・`FillOpacityMask`）を使い、式を自分で書かない。読み戻しは `SurfaceComposite::read_pixel` と同じ形（1×1 の写し先・控え）。窓の点の戻しは `ScaleRatio::unscale_coord`。色の絵の α は `AlphaMask`。新しく作るのは ID の描き先の入れ物・`region` の色の規則・形の決めの 3 つ。wintf の `ColorMapData` は `region` に使わない（2.2 節のとおり作りが合わない）。
- **削る**: 拡大つきで塗る口・描き先を描画面の持ち主の実体に付ける部品と係・`SystemParam` の読み口・ID の表（位置の足し算で足りる）・GPU のマスクの絵の覚え・CPU の矩形の判定と `RegionPriority`・採寸の読みだけ画像を読まない分かれ道。

### 11.9 設計の決めと退けた形

| 決め | 採った形 | 退けた形と理由 |
| --- | --- | --- |
| 描く道具 | D2D・`B8G8R8A8`・α 掛け済み・不透明の ID の色・縁のぼかしなし | D3D11 の整数の描き先（D2D で描けない＝11.5。楕円・巻き数の多角形・マスクを描く 2 つ目の描き手が要る） |
| 読み戻し | D2D の 1×1 の写し先 | D3D11 の口で読む折衷（差は 0.1 ms ほど＝11.6。D2D と D3D11 の錠が要る） |
| 細かさ | サーフェスの原寸・点の側を `unscale_coord` で戻す | 物理ピクセル（マスクが補間で混ざる＝11.3・割り切れない拡大率で端がずれうる＝11.2・拡大率が変わるたびに描き直す） |
| 置き場 | presenter が持つ wintf の入れ物（候補 A-2） | 描画面の持ち主の実体に付く部品と係（候補 A-1）。原寸で持つ描き先は描画面の物理の大きさに依らないので、wintf が寿命を合わせる利点が無い。係に描かせると指令の適用から描画の段まで古い絵が残る。読むのに World が要り署名が変わる |
| 描く時点 | 表示の成立点（サーフェスが変わった回だけ） | 読みのときに描く（読みは `&self` で装置を借りられない）・毎フレーム比べる |
| 色の絵の α | クリックの透過と同じ α マスク（表示中の写しを対象が持つ） | GPU の読み戻し（読みが増える・World が要る・描画の段まで前の絵の α・クリックの透過と縁で食い違いうる・presenter のテストに本物の描画面が要る）。`draw-methods-canon` がマスクの出どころを替えるときに一緒に移る |
| ID の割り当て | 並びの位置＋1（上限越えのときは塗る先頭からの位置） | ID と名前の表を別に持つ |
| 塗る件数の上限 | 65,536 件・手前の側から残す | 上限なし（2^20 件で 0.7〜2 秒 UI スレッドが止まる）・別スレッドで描く（描き終えるまで読めない窓ができる） |
| `region` の読み | 読み手に乗算しない読みの口を足し、合成の層でマスクにする（GPU なしで色の規則を固定） | wintf の `ColorMapData`（作りが合わない）・乗算済みから色を戻す（α が小さいと戻せない） |
| 採寸の読み | `region` の画像を読む（読み方を分けない） | 採寸のときだけ読まない（その対象を表示に使うと黙って当たらなくなる） |
| 大きな座標 | f32 へ移すとき ±2^24 で頭を打つ | 式で切り取る（交差する多角形の切り取りは巻き数を保てない） |

### 11.10 試していないこと

- **WIC の乗算しない読み（`32bppBGRA`）が、ガンマや色の情報のチャンクを持つ PNG で R・G・B を変えないか**（7 節の R7）。読み手の crate のテストを別に組む手間がかかるので設計の段では試さなかった。実装の最初のタスクで決定論のテストとして確かめる。変わるなら `region` の読みだけ `image` の PNG の読みへ替える（設計の未決の議題 6）。
- **拡大率が割り切れない値で、画素の中心がちょうど辺に載る並びの矩形**（11.2）。採った形（原寸で塗る）では起きないので試さなかった。
- **最終の `IdTarget` を通した読みの待ち**（要件 6.5）。11.6 は同じ手順（1×1 の写し先）の試しの数字。実装で `tests/graphics/id_target_test.rs` が測り直す。
- **クローディアの面ごとの形の数と塗りの所要**（7 節の R8）。11.7 の件数あたりの所要から、100 件で 1〜5 ms と見積もった。

### 11.11 危険と手当て

- 周のすぐそばの画素が装置で分かれる（11.4）→ テストの点を周から離す・§8 に書く・steering `tech.md` の WARP の例外の列に「固定する点」を書く。
- 要件の文と合わない所（要件 10.1 の 3 点・要件 5.1・5.6・5.7・5.8・5.13 の文）→ 設計の未決の議題 1・2 として討議へ出し、決まったら要件の文を直す。
- `region` を多く使うシェルでサーフェスの切り替えが重い（11.3）→ 数字を設計の「Performance」に書き、GPU の絵を覚える形は後に回す。
- 同じウェーブの 3 本（`balloon-flip-ssp-review`・`audit-emo2-boot-frame`・`input-crate-split`）との重なり: 9 節のとおり読み手の 4 ファイルと `areka-emo-atlas` は重ならない。この設計が新しく触る `crates/areka-emo-present/src/presenter/{show,refresh,hit,target,hub,replace}.rs`・`shell_target.rs`・`crates/areka-emo-compose/src/{hit.rs,world.rs,lib.rs}` と wintf の `ecs/graphics/mod.rs`・`ecs/layout/hit_region/mod.rs` は、タスクの生成の前に roadmap の C23 の行の「触るファイル」と突き合わせる。本体の `input_events/` には触らず、`resolve_hit_region` の署名も変えないので、`input-crate-split` への知らせは「署名は変わらない」の 1 行で足りる。

## 12. 設計の討議の決め（2026-11-20）

設計の検証（`design-validation.md`・条件つき GO）の後、設計の議題 6 件を開発者に諮り、6 件とも推奨どおり承認された。正本は `requirements.md` の決め 16〜20 と `design.md` の「設計の討議で決まったこと」。

| 議題 | 決め | 根拠（11 節の試しと検証） | 採らなかった形 |
| --- | --- | --- | --- |
| 1 楕円・円の周のそばの画素 | 「画素の中心が形の中か」の 1 つの規則で通す（決め 17） | 周のそばの 3 点が SSP と半画素ずれる。軸の端は合う | 楕円と円だけ半画素ずらして塗る（軸の端が周の上に載り、答えが丸めに左右される） |
| 2 ID の絵の細かさ | サーフェスの原寸で持ち、点の側を戻す（決め 16） | 画面の画素の大きさで塗ると `region` が補間で混ざり（1〜3.5 万画素）、最近傍でも拡大率 11/10 で 1,087 画素がずれた。SSP は画像の画素ごとに決める | 画面の画素の大きさで塗る |
| 3 置き場 | presenter が持つ wintf の入れ物 `IdTarget`（決め 16） | 表示の成立と同じ呼び出しで塗れる・塗り直す時点を知るのは presenter・入口の署名が変わらない（検証がコードで確かめた） | 描画面の持ち主の実体に付く部品と係 |
| 4 描く道具 | D2D で塗り D2D の 1×1 で読む・24 ビット（決め 18） | D2D は整数の描き先へ描けない（`0x88982F80`）。D3D11 で読むだけの差は 304 µs 対 396 µs | D3D11 の整数の描き先と自前のシェーダー・D3D11 で読む折衷 |
| 5 色の絵の α | クリックの透過と同じ α マスク（決め 19） | マウスが窓に届くかを決める表と名前の表を 1 つにする。`draw-methods-canon` で一緒に移る | GPU の読み戻しで画面の画素の α を読む（brief の Desired Outcome の形） |
| 6 `region` の GPU の絵 | 表示対象の寿命で覚える控えを入れる・上限 65,536 件（決め 20） | 載せ直しは 1 件 約 1.7 ms・100 件 約 180 ms、塗りだけなら約 8 ms | サーフェスが変わるたびに載せ直す |

- 議題 5 は、検証が「裁定（GPU に持つ）への違反とは読まないが緊張は残る」と書いた点で、その旨を添えて諮ったうえでの承認である。`draw-methods-canon` の要件の段では、α マスクの出どころの付け替えがクリックの透過と名前の解決の両方に効くことを前提にする（向こうの brief への申し送りは、この仕様の実装の最後の段で書く）。
- 議題 1 に伴い、要件の討議の議題 4 の説明（「楕円と円は SSP と 1 画素まで同じになる」）は言い過ぎだったと開発者へ伝えた（軸の端は合うが、周の斜めのあたりは半画素ずれる）。
- 検証の指摘で設計に入れた直し: `crates/areka-runtime/src/hit_region.rs` は触らない（直す所が無い）・書き直すテストに `presenter_film_tests.rs` を足す・`region` の読みは新しいファイル `shell_target_regions.rs` へ置く（`shell_target.rs` は 1,000 行の番に近い）・`shown_mask` を足しても α マスクの輪番の確保が増えないことをテストで固定する。
- 同じウェーブとの取り決め: 入口の署名は変わらないので `input-crate-split` の追従は要らない（同日に知らせた）。`doc/COMPAT_ARCHITECTURE.md` §8 の既にある行の書き直しと steering `tech.md` の 1 文は、実装で触る前に相手のセッション（`balloon-flip-ssp-review`・`input-crate-split`）へ知らせる。
- 設計の前提で、コードで追っていないもの: 「マウスの受け手が走るのは、表示の指令の適用と描画の後だけ」。この設計は ID の絵を表示の成立点で塗り、α マスクも同じ所で入れ替えるので、受け手がどの段で走っても答えは変わらない（この前提に依らない）。

## 13. 実装の段の測り: 乗算しない読みの道（2026-11-21・タスク 1）

`region` の画像の乗算しない読みは WIC の道（1 枚目のコマを `GUID_WICPixelFormat32bppBGRA` へ変換）に決め、`image` の PNG の読みへは替えなかった。テスト（`crates/areka-emo-atlas/src/decode/wic_arm_unmultiplied_tests.rs`）でメモリに組んだ 4×1 の RGBA の PNG（RGBA (200,100,50,128)・(10,20,30,255)・(77,88,99,0)・(255,1,128,1)）に、gAMA（ガンマ 1.0＝100000）・cHRM（白色点 (0.3127,0.329)・原色を sRGB から外した値）・sRGB（意図 0）・iCCP（ガンマ 1.0 の灰色の形を装った 132 バイトの ICC）を 1 つずつ、また gAMA＋cHRM＋sRGB を一緒に差し込んで読んだところ、5 枚とも 4 画素の R・G・B・α が書いた値と 1 つも違わなかった（α＝0 の画素の色も残る）。差し込んだチャンクは WIC の metadata の名の列（`/gAMA`・`/cHRM`・`/sRGB`・`/iCCP`）に出ることを同じテストで確かめ、チャンクの無い絵には `/gAMA` が出ないことも確かめた（壊れたチャンクを WIC が黙って読み飛ばした結果ではない）。α の無い RGB の PNG（gAMA つき）は 4 画素とも α＝255・色はそのままで、`has_alpha` は偽。画像でないファイルは理由つきの `Decode`、無いファイルは `NotFound`、一辺 16,385 の絵は寸法と上限（16,384）を書いた `Decode` で断る。

## 14. 実装の段の測り: crate の外から使う 1 画素の読みの待ち（2026-11-21・タスク 2.5）

実 GPU（`GraphicsCore::new()`）・512×512 の `IdTarget` を矩形とマスクで塗り、塗り直さずに座標を変えながら 1 画素の `read` を 300 回（毎回 GPU を読む）測った（`crates/wintf/tests/graphics/id_target_test.rs`・1 回の走り・上限は置かない）。

| 記録 | 条件 | 中央値 | 95% 点 | 最大 |
| --- | --- | --- | --- | --- |
| 今回（`IdTarget::read`） | GPU が空いているとき・300 回 | 312 µs | 499 µs | 17.2 ms |
| 完了 `wintf-d2d-composite-rules`（§10.6） | 続けてもう 1 点読む（GPU が空いている）・200 回 | 344 µs | 556 µs | 0.91 ms |
| 同上 | 合成の直後に読む | 516 µs | 747 µs | 1.26 ms |

- 中央値と 95% 点は完了 spec の記録（および 本書 §11.6 の 396 µs・605 µs）と同じ帯。最大の 17.2 ms は 1 回だけの外れ値（初回の読みか OS の割り込み）で、数字の読みには使わない。
