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
- **設計の段（`/kiro-design`）で決める**:
  1. ID の絵を持つ所と作り直す時点（表示中の面だけ持つか・面ごとに覚えるか）。
  2. 色の絵の α を引けないとき（表示中の 1 件が `InvalidateCache` で消えた直後）の答え（6 節の 5 の後半）。
  3. 採寸の読み（`load_shell_target_quiet`）で `region` の画像を読むか（6 節の 11）。
  4. 塗る仕事の予算（持ち込みで並びが大きい面・`region` 100 件以上の面）と、越えたときの答えと記録（6 節の 14・7 節の R8）。
  5. WIC の読みが R・G・B を変えないことの確かめ（7 節の R7）。
  6. `circle` と `region` の作り物の検体の置き方（クローディアはどちらも持たない）。
