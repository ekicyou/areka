# ギャップ分析: areka-P0-keycolor-clickthrough-coverage

> 2026-09-23 実測。対象は本ブランチ HEAD `2ed0a146`（main `92f5f448` の直上・製品コードは main と同一）。
> 引用は「そのファイルの何を定義している箇所か」で指し、行番号は HEAD 時点の参考値として添える。
> 検体の画素の実測は `vendors/sample_ghost/*.nar` を作業用フォルダへ展開して GDI+ で読んだもので、値の桁を掴むための参考である（製品は WIC で復号する）。

## 1. 要約

- **要件が前提にしている「継ぎ目 0・製品コードの変更 0」で端から端まで届く**ことを実測で確認した。`load_shell_target` → `ShellTarget::build_world` → `EmoPresenter::attach_target` → `apply(ShowSurface)` → 窓の面 entity の `AlphaMaskResource::mask()` は、すべて既存の公開 API と既存テストが既に踏んでいる読み口だけで到達でき、GPU からの読み戻しは 0 回、実窓は 0 個で済む。
- **テストが読むマスクは wintf の当たり判定が読むものと同一の実体**である（`Arc` の参照の受け渡し・複製なし）。「並行する写し」ではない。
- **唯一の細工は可視性 1 点**: 検体の受け口（`shell_target_test_support.rs`）が `pub(super)` で `shell_target` モジュールの外から見えない。一方、GPU 付き World・窓 entity・`ShowSurface` 適用・面 entity の取り出しの補助は `presenter` 側の `presenter_test_support.rs` に揃っており、こちらも `pub(super)`。**どちらかの側の補助を `pub(crate)` へ広げれば新しい補助を 1 つも書かずに済む**（推奨は受け口側を広げてテストを `presenter` の兄弟に置く形）。
- **要件の主張で言い直しが要るのは 3 点**（§4）: ⑴「製品コードの変更 0 行」は、兄弟テストの接続宣言（`#[cfg(test)] #[path] mod …;` の 3 行）を製品ファイルへ足さない限り新しいテストを繋げないので、数え方の定義が要る。⑵ 合成段の差し替え候補「全画素を不透明で書く」は、転写がトリム後の矩形の内側しか書かないので実際に不透明になるのは矩形の内側だけ——それでも赤になる（内側に抜かれた画素が 4 検体面すべてで数千〜数万個ある）が、文言は正確でない。⑶ 私有 `build_shell_target` の改名は、要件 3.4 が触ってよいと定めた 2 ファイルの外（`collision-probe.rs` の doc リンク・`window-placement.rs` の説明文）にも同名の言及が残る。
- 規模は **S**（1〜2 日・タスク 3〜4 本）、リスクは **低**。設計で決めるべき項目は §6 に 10 件。

## 2. 現状の実測（要件 → 既存資産の対応表）

### 2.1 製品が実際に通る順と、テストから届くか

| 段 | 製品の経路（定義箇所） | テストから届くか | 備考 |
|---|---|---|---|
| 読む・焼く | `crates/areka-emo-present/src/shell_target.rs` の `load_shell_target`（fs を触る唯一の入口・L230）→ 同 `build_shell_target`（`UseSelfAlpha::On` 固定で `SurfaceSet` を組み `bake` を呼ぶ・L317〜347）→ `crates/areka-emo-atlas/src/lib.rs` の `bake`（`Normalizer::key_color` を正規化の前に呼ぶ・L89／`Normalizer.normalize`・L92）→ `crates/areka-emo-atlas/src/normalize.rs` の `Normalizer::normalize` の `(On, KeyColor)` の腕（左上と 4 バイト一致の画素を `[0,0,0,0]` にする・L193〜227） | **届く**。`shell_target_template_tests.rs` の `load` が同じ入口で 2 検体を実物ごと読んでいる | 起動側 `crates/areka/src/emo2_boot/assets.rs`（`load_shell_target` を 1 回・L291）と採寸側 `crates/areka/src/placement/measure.rs`（L336）も同じ入口 |
| 合成する | `crates/areka-emo-present/src/presenter/show.rs` の `apply_show`（`target.budget.native_scratch(|scratch| composer.compose_into(...))`・L102〜116）→ `crates/areka-emo-compose/src/lib.rs` の `Composer::compose_into`（`build_plan` → `blit::execute`・L119〜147）→ `crates/areka-emo-compose/src/blit.rs` の `execute`（乗算済み SourceOver・出力先を全透明にクリアしてから転写・L69〜163） | **届く**。`presenter_budget_equivalence_tests.rs` が `show_ok`（`presenter_test_support.rs`・実 `apply` を `ShowSurface` で駆動）で通している | 外形は `crates/areka-emo-compose/src/plan.rs` が各層の「配置＋原寸（`AtlasEntry::original`）」から出す（L626〜629）ので、トリムで縮まない＝面 0 の外形は PNG の実寸のまま |
| マスクを作る | 同 `show.rs`（交代後の表示バッファの `bytes()` を `target.budget.regenerate_mask` へ・L185〜191）→ `crates/areka-emo-present/src/presenter/budget.rs` の `MaskRotation::regenerate`（`AlphaMask::from_pbgra32`／`regenerate_from_pbgra32`・L315〜349）→ `cache.insert`（L194）→ `world.get_mut::<AlphaMaskResource>(mount.surface_entity())` に `set_shared(entry.mask.clone())`（L322〜327） | **届く**。同テストの ⑶ が `presenter.targets[..].mount.surface_entity()` を引き `world.get::<AlphaMaskResource>(..).mask()` を読んでいる（L353〜366） | 面 entity へ `AlphaMaskResource::new()` を最初から載せるのは `crates/areka-emo-present/src/mount.rs` の `VisualMount::attach`（L189〜199） |
| 窓側（本仕様の外） | `crates/wintf/src/ecs/layout/hit_test/mod.rs` の `alpha_mask_hit`（`world.get::<AlphaMaskResource>(entity).and_then(|r| r.mask())` を最優先で読む・L224〜231）→ `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs` の `AlphaMask::is_hit`（α ≥ 128 で「内」・L121〜147） | 読まない（観測点はその手前） | — |

**マスクが同一の実体であること**: `show.rs` は `entry.mask.clone()`（`Arc<AlphaMask>` の参照カウント増）を `AlphaMaskResource::set_shared` へ渡し（`hit_test/mod.rs` の `set_shared`・L189〜191）、`alpha_mask_hit` は同じ `AlphaMaskResource` から `mask()` で借りる。テストが `mask()` で読むものと wintf が読むものは同じ `Arc` の中身であり、写しは存在しない。キャッシュのスロット（`CacheEntry::mask`・`cache.rs` L102）も同じ `Arc` を握っているが、要件 1.2 が「スロットだけを読んで済ませない」と定めるとおり、読むのは面 entity 側でよい。

**GPU の前提**: `presenter_test_support.rs` の `make_world_with_gpu`（`GraphicsCore::new` ＋ `WucGraphicsResource::new`・COM は MTA 初期化）と `spawn_window_with_dpi`（`DPI` component だけを持つ素の entity）が既存の presenter 系テスト 15 本の前提そのもの。`show.rs` は合成した原寸バイトから D2D bitmap を作ってコマンドリストに記録する（`record_display`・CPU → GPU の**転送**）が、GPU → CPU の読み戻しは経路のどこにも無い。`EmoPresenter::read_back`（`presenter/read.rs` L230〜）も「合成メモの原寸バイトをそのまま返す」だけで表示面を読まない。本テストはそれすら呼ぶ必要がない。

### 2.2 可視性（要件が「唯一の細工」と呼んだ点）の実測

| 補助 | 場所 | 可視性 | 見える範囲 |
|---|---|---|---|
| 検体の受け口 `emo2_shell_dir`／`r_post_and_komainu_shell_dir`／`konnoyayame_shell_dir`・COM 初期化 `with_com_initialized`・ログ捕捉 `capture_events` | `crates/areka-emo-present/src/shell_target_test_support.rs`（`shell_target.rs` が `#[cfg(test)] #[path] mod test_support;` で繋ぐ・L434〜436） | `pub(super)` | `shell_target` モジュールの配下（同ファイルが繋ぐ 5 つのテストモジュール）だけ |
| GPU 付き World `make_world_with_gpu`・窓 entity `spawn_window_with_dpi`・`ShowSurface` の適用 `show_ok`・装着済み target の面 entity と文字層スロット `mount_entities`（`presenter.targets` を読む） | `crates/areka-emo-present/src/presenter_test_support.rs`（`presenter.rs` が繋ぐ・L134〜136） | `pub(super)` | `presenter` モジュールの配下（同ファイルが繋ぐ 15 のテストモジュール）だけ |
| `presenter.targets`（`PresentTarget` の `mount` を含む） | `presenter/hub.rs` の `EmoPresenter`（L21）・`presenter/target.rs` の `PresentTarget`（全欄 `pub(super)`） | `pub(super)` | `presenter` 配下 |

つまり、新しい通しテストは **どちらか一方の補助には届くが両方には届かない**。届かない側を `pub(crate)` へ広げる（テスト専用ファイルの `pub(super)` → `pub(crate)` の書き換え・製品コードではない）か、届かない側を自前で持つ（受け口の複製は要件 1.7 が禁じる）かの二択で、前者が要件どおり。共有の形の選択肢は §5。

なお受け口側のファイル冒頭の説明文は「`balloon_test_support.rs` を流用できないのは、あちらの受け口が `pub(super)` で外から引けないため」と、同じ壁を既に一度踏んだことを記している。

### 2.3 検体の実測（要件 Introduction の表の裏取り）

`vendors/sample_ghost/{R_POST_and_KOMAINU,konnoyayame}.nar` を展開して確認した。

| 検体・面 | PNG（IHDR） | チャンク | 左上の画素（BGRA） | 抜かれる画素 ／ 全画素 | 抜かれない画素の外接矩形 | 矩形の**内側**にある抜かれる画素 |
|---|---|---|---|---|---|---|
| `R_POST_and_KOMAINU` 面 0 `surface0000.png` | 236×462・色種別 2（truecolor・α 無し） | IHDR, tpNg, IDAT, IEND | 白 `[255,255,255,255]` | 59,831 ／ 109,032（54.9%） | (20,50)〜(199,461) | **24,959** |
| 同 面 10 `surface0010.png` | 140×160・色種別 2 | 同上 | 白 | 11,816 ／ 22,400（52.7%） | (0,16)〜(133,159) | **8,712** |
| `konnoyayame` 面 0 `surface0000.png` | 260×390・色種別 3（パレット・`tRNS` 無し） | IHDR, PLTE, IDAT, IEND | 緑 `[0,255,0,255]` | 70,574 ／ 101,400（69.6%） | (7,47)〜(251,384) | **51,984** |
| 同 面 10 `surface0010.png` | 200×200・色種別 3 | 同上 | 緑 | 28,747 ／ 40,000（71.9%） | (33,65)〜(155,195) | **4,860** |

- `R_POST_and_KOMAINU` の面の絵は `surface0000`〜`0006`・`0010`・`0100`・`0200` の 10 枚すべて色種別 2、`konnoyayame` は `surface0000`〜`0007`・`0010`・`0011`・`1030`〜`1033`・`1040`〜`1043` の 18 枚すべて色種別 3。要件の表のとおり。
- WIC の腕（`crates/areka-emo-atlas/src/decode/wic_arm.rs` の `decode_inner`）は変換前のピクセル形式で `has_alpha` を決める（`pixel_format_has_alpha`・L137〜153）。24bpp BGR も 8bpp Indexed も α 付きの集合に無いので `has_alpha=false`、`.pna` も無いので抜き色の腕（`select_source` → `KeyColor`）へ落ちる。2 体とも同じ腕を通る。
- **`R_POST_and_KOMAINU` の抜き色は白**である。要件 4.1 のとおり「つながりに依らず全画素・完全一致」なので、キャラクターの内側の純白の画素（目の白目やハイライトがあれば）も「外」になる。テストはこの規則をそのまま主張すればよく、「絵の内側は全部『内』」とは主張しない（報告でもそう書かない）。
- 面 0 の合成外形が PNG の実寸に一致する（トリムで縮まない）ことは `shell_target_template_tests.rs` が 236×462／260×390 で既に固定している。土台の絵は層 0・位置 (0,0)（`crates/areka-emo-compose/src/base_image.rs` の `base_element`・L111〜118）なので、PNG の座標 (x,y) と合成後・マスクの座標 (x,y) は同じ。
- `surfaces.txt` の面 0 は 2 体とも `element` 行 0・`collision` 行だけ（`konnoyayame` は `animation0`＝まばたきも持つが、`PatternState::default()` で適用すればコマは重ならない。`shell_target_template_tests.rs` の `konnoyayame_blink_frame_changes_pixels_only_inside_the_frame_rect` がその前提で通っている）。`R_POST_and_KOMAINU` の面 10 は `surfaces.txt` に波括弧を持たない（同テストの `is_declared` の較正で固定済み）。

### 2.4 「正解」の決め方（要件 1.3）に使える復号器

| 候補 | 実在 | 長所 | 短所 |
|---|---|---|---|
| 製品と同じ `WicDecoderArm`（`areka_emo_atlas::WicDecoderArm`・`ElementDecoder::decode` が `DecodedImage { width, height, stride, bgra, has_alpha }` を **公開欄**で返す・`decode.rs` L22〜30） | ✅ 既存 | 正規化より手前の出力＝要件 1.3 の「復号した生の画素」そのもの。パレット展開・チャンネル順・色管理の差が原理的に出ない。新しい依存 0 | 復号器が製品と共通（ただし復号は要件が「導かない」と定めた 4 段——正規化・焼き・合成・マスク——の外。復号器が壊れて全画素が同色になれば要件 1.5 の較正が止める） |
| `image`（`png` 機能）を `areka-emo-present` の dev 依存へ足す | ❌ `crates/wintf/Cargo.toml` の dev 依存にしか無い（L44） | 製品と独立 | 依存の追加は開発者の承認事項（`tech.md` の `encoding_rs`・`miniz_oxide` の登記の前例）。RGB(A)→BGRA の並べ替えと α=255 の補完をテストが自前で書く。パレット PNG は色へ展開してから比べる必要がある（添字の一致と色の一致は別） |

### 2.5 差し替え（要件 2）が到達する経路に在るか・自身の主張で赤になるか・道連れで赤になる既存テスト

| 段 | 差し替えの定義（候補） | 到達する経路に在るか | 本テストの赤の形（要件 1.4 の主張） | 道連れで赤になる既存テスト（見込み） |
|---|---|---|---|---|
| 焼く | `normalize.rs` の `(On, KeyColor)` の腕で `if let Some(key) = key { … }` の中の書き換えを行わない（または `Normalizer::key_color` が `None` を返す） | 在る（§2.1 の 1 行目） | 抜かれるはずの全画素が α=255 のまま合成され「内」になる → 食い違い 59,831 件（`R_POST` 面 0） | `normalize.rs` 内 `on_no_alpha_no_pna_selects_keycolor_seam`・`normalize_key_color_tests.rs` の各件・`shell_target_template_tests.rs` の `r_post_and_komainu_shows_both_scopes_from_file_names_alone`（左上の α=0 の主張）。`key_color` を `None` にする形なら `bake` の抜き色の `debug!` 行も消える |
| 合成する | `blit.rs` の `execute` で転写先の α を `source_over_channel` で求めずに 255 を書く（または `compose_into` の末尾で全画素の α を 255 に上書き） | 在る（`show.rs` → `compose_into` → `execute`。`Composer::compose` も同じ `compose_into` を通る） | 転写はトリム後の矩形の内側しか書かないので、不透明になるのは矩形の内側だけ。**内側の抜かれた画素**（`R_POST` 面 0 で 24,959・面 10 で 8,712・`konnoyayame` 面 0 で 51,984・面 10 で 4,860）が「内」になり赤 | `blit.rs` 内の `known_pixel_pair_source_over_byte_exact`・`transparent_src_leaves_dst`・`trim_offset_shifts_destination` など／`areka-emo-compose` の合成 golden 群／`presenter_budget_equivalence_tests.rs`（便宜経路も同じ `execute` を通るので両者は一致するが、`assert_expected_is_not_empty` の「マスクに hit と非 hit の両方が在る」で止まる） |
| マスク | `show.rs` で `regenerate_mask` へ `display.bytes()` の代わりに同じ長さの全 255 のバッファを渡す | 在る（§2.1 の 3 行目・ミス経路） | 全画素「内」→ 食い違い＝抜かれた画素の数 | `presenter_budget_equivalence_tests.rs`（スロットのマスク・供給されたマスクの両方が独立再現と食い違う）。`budget_tests.rs` は `MaskRotation` を自前の画素で直に叩くので影響しない（`MaskRotation::regenerate` 側を差し替える形にすれば逆に `budget_tests.rs` が赤になる） |

いずれも「透明を運ぶ段の入力や出力を別の物に置き換える」形で、座標や値をずらす形ではない。走らせ方は `cargo test -p areka-emo-atlas`／`-p areka-emo-compose`／`-p areka-emo-present` の 3 本で、道連れの赤はこの 3 本の出力にすべて現れる（`crates/areka` 側の `measure_template_tests.rs` は外形しか見ないので焼く段の差し替えでも緑のまま）。

### 2.6 相乗り 3 件の実測

| 候補 | 実測（HEAD） | 判定 |
|---|---|---|
| ① `crates/areka-emo-compose/src/sample_test_support.rs` の較正 | 受け口は `emo2_root`（`folder()` そのもの）と `konnoyayame_shell_root`（`folder().join("shell/master")`）の 2 口・`pub(crate)`。同ファイルに `#[test]` は 0 本。使い手は `base_image_tests.rs`（L527）・`fold_tests.rs`（L724）・`golden_tests_test_support.rs`（L15）・`world.rs`（L314）。`lib.rs` が `#[cfg(test)] mod sample_test_support;` で繋いでいる（L172〜173） | 要件どおり 1 本足せる。置き場は同ファイルの中（`shell_target_test_support.rs` の `every_sample_receptor_points_at_a_real_shell_folder` と同じ形）が最小 |
| ② `crates/areka/src/placement/measure_template_tests.rs` の本文走査 | 走査対象は `include_str!("measure.rs")`（`the_measure_side_does_not_parse_the_shell_itself`）と `include_str!("../emo2_boot/assets.rs")`（`the_boot_side_…`）の 2 本。`load_shell_target` を**呼ぶ** example は `examples/emo-present/setup.rs`（L92）・`examples/collision-probe/setup.rs`（L120）・`examples/window-placement.rs`（L365）の 3 本（`emo-present.rs`・`collision-probe.rs` は `use` で名前を持ち込むだけ）。3 本の本文の `shell::parse` は 0 件、走査器が拒む生文字列（`r"`・`r#`）と二重引用符の文字リテラル（`'"'`）も 0 件（正規表現で実測） | 要件どおり。`include_str!("../../examples/…")` は同ファイルからの相対パスで読める |
| ③ 私有 `fn build_shell_target` | `examples/emo-present/setup.rs`（定義 L90・呼び出し L36・説明文 L73）と `examples/collision-probe/setup.rs`（定義 L119・呼び出し L93・説明文 L112）の 2 本に実在。公開の `areka_emo_present::build_shell_target`（`lib.rs` L70〜73 で再輸出）と同名で別物。**加えて** `examples/collision-probe.rs` の冒頭説明文（L101 の ``[`build_shell_target`]`` リンク）と `examples/window-placement.rs` の説明文（L354「donor `build_shell_target`」）が旧名を指す | 改名は 2 本で足りるが、説明文の残りをどうするかは要件 3.4（触るファイルを 2 本に限る）との兼ね合いで設計が決める（§4 ⑶） |

roadmap の干渉台帳（A1・2026-09-20）で ④ の触るファイルは「新規のテストファイル（相乗りを採れば `areka-emo-compose` のテスト・本文走査のテスト・`crates/areka/examples/emo-present/setup.rs`）」。要件が足した `examples/collision-probe/setup.rs` は ①〜⑦ のどの行にも無く、重なりは 0 のまま。

## 3. 要件ごとの資産対応（Requirement-to-Asset Map）

| 要件 | 既存資産 | 状態 |
|---|---|---|
| 1.1 同じ入口で読み・面の表を組み・装着し・`ShowSurface` を適用 | `load_shell_target`／`ShellTarget::build_world`／`EmoPresenter::attach_target`／`apply`（すべて `pub`） | 揃っている |
| 1.2 wintf が読むのと同じ場所からマスクを読む | `AlphaMaskResource::mask()`（`pub`）・面 entity は `mount_entities`（presenter 配下）か World の走査で引ける | 揃っている（引き方は設計・§6 の 3） |
| 1.3 正解は同じ PNG を復号した生の画素から | `WicDecoderArm::decode` → `DecodedImage.bgra`（公開欄） | 揃っている（復号器の選択は §6 の 2） |
| 1.4 全画素の判定と食い違いの数 0 | `AlphaMask::is_hit`／`width`／`height` | 揃っている |
| 1.5 較正（0 < 抜かれた画素 < 全画素） | — | テスト側で書く（実測値は §2.3） |
| 1.6 面 10 と `konnoyayame` 面 0 | 同じ経路。面 10 は宣言無し（既存の較正あり） | 揃っている（1 target で面を切り替えるか 3 target にするかは §6 の 6） |
| 1.7 検体は受け口経由のみ | `shell_target_test_support.rs` の 3 口（`pub(super)`） | **可視性の壁**（§2.2・§5） |
| 1.8 常時テスト・GPU 前提は既存と同一・読み戻し 0 | `make_world_with_gpu`／`spawn_window_with_dpi`（`pub(super)`） | **可視性の壁**（同上） |
| 1.9 名前と説明文 | — | テスト側で書く |
| 1.10 製品コード 0 行 | — | **数え方の定義が要る**（§4 ⑴） |
| 2.1〜2.4 差し替え 3 段 | §2.5 | 定義は候補どおり成立（合成段の文言だけ要修正） |
| 3.1〜3.4 相乗り | §2.6 | 成立（③ の説明文の扱いだけ要判断） |
| 4.x 変えないもの | — | 本分析の範囲で変える理由は見つからない |
| 5.x 報告 | — | 「GPU 転送は 1 回起きる（読み戻しではない）」を報告文で混同しない |

## 4. 要件の主張のうち、そのままでは正しくない・言い直しが要るもの

1. **要件 1.10／5.2「製品コード（非テストファイル）を 0 行変えない」「変更行数 0」**。`.kiro/steering/structure.md` の規約（兄弟ファイル＋接続宣言）では、新しいテストモジュールは製品ファイル（`presenter.rs` か `shell_target.rs`）へ `#[cfg(test)] #[path = "…"] mod …;` の 3 行を足さないと繋がらない。既存の 15＋5 本もすべてこの形で繋がっている（`presenter.rs` L101〜147・`shell_target.rs` L434〜456）。要件 1.10 の括弧書き「`#[cfg(test)]` の中」はこれを許す意図と読めるが、5.2 の「変更行数 0」を数として書くには「`#[cfg(test)]` の付いた項目の外の行＝0」のように数え方を定めるべきである。定めないと、報告の 0 が読み手の数え方次第で 3 になる。
2. **要件 2.1 の合成段の候補「転写が元の α を捨てて全画素を不透明で書く」**。`blit.rs` の `execute` は出力先を全透明にクリアしてからトリム後の矩形（`placement.uv_rect`）の内側だけを転写するので、α を捨てる差し替えで不透明になるのは矩形の内側に限られ、矩形の外は透明のまま「外」である。それでも矩形の内側に抜かれた画素が 4 面すべてで在る（§2.3 の最終列）ため本テストは赤になる。文言を「転写した画素の α を捨てて不透明で書く」に改めるか、差し替えの定義に「矩形の内側の抜かれた画素で赤になる」旨を添えるのが正確。
3. **要件 3.3／3.4 の改名**。改名の対象は 2 本の私有関数で正しいが、旧名への言及は `examples/collision-probe.rs` の冒頭説明文（``[`build_shell_target`]`` の doc リンク）と `examples/window-placement.rs` の説明文にも在る。3.4 は触るファイルを「上の examples 2 本」に限っているので、⑴ 説明文の言及は残す（リンク先が無くなるだけ・examples の rustdoc は生成しない）、⑵ 3.4 の列挙へ 2 ファイルの説明文だけを足す、のどちらかを設計で決める。
4. **要件 Introduction「読み戻し 0 回」は正しいが、GPU への転送は起きる**。`show.rs` の `record_display` は合成バイトから D2D bitmap を作る（CPU → GPU）。報告で「GPU に触れない」と書くと誤りになる。「GPU からの読み戻しは 0 回・GPU 資源の前提は既存の presenter 系テストと同一」の範囲に留めること（要件 5.2 の文言はその範囲に収まっている）。
5. **`R_POST_and_KOMAINU` の抜き色は白**（§2.3）。要件は色に触れていないので誤りではないが、「キャラクターの外＝抜かれる／内＝残る」という説明は近似であり、白い画素はどこに在っても抜かれる。テストの説明文と報告はこの点を混同しない書き方にする。

> **要件ディスカッション（2026-09-23）での反映**: 上の 1〜5 はすべて requirements.md へ反映済み。⑴ 要件 1.10 に数え方（`#[cfg(test)]` の付いた項目の外で変わった行＝0・接続宣言と可視性の書き換えは別に件数を報告）・要件 5.2 がそれを参照。⑵ 要件 2.1 の合成段の候補を「転写した画素の α を捨てて不透明で書く」へ。⑶ 要件 3.3／3.4 に `collision-probe.rs`・`window-placement.rs` の説明文を含めた（roadmap の干渉台帳で、この 2 ファイルに触る A1 の他 spec は 0 件）。⑷ 要件 5.2 に「CPU → GPU の転送は起きる」。⑸ 要件 1.3 と 5.3 に「抜き色は絵の内側の画素にも当たる」。

## 5. 実装の選択肢

### 案 A: テストを `presenter` の兄弟に置き、検体の受け口を `pub(crate)` へ広げる（推奨）

- 新規: `crates/areka-emo-present/src/presenter_keycolor_clickthrough_tests.rs`（名前は仮。stem `presenter` ＋テーマ。同じディレクトリに `presenter_keycolor…` から導出しうる別の製品ファイルは無いので命名規約の衝突なし）。接続宣言 3 行を `presenter.rs` の既存の並びへ足す。
- 変更: `shell_target_test_support.rs` の `r_post_and_komainu_shell_dir`・`konnoyayame_shell_dir`（必要なら `emo2_shell_dir`・`with_com_initialized`）を `pub(super)` → `pub(crate)`。`LazyLock` の実体はそのまま（複製 0）。
- そのまま使える: `make_world_with_gpu`・`spawn_window_with_dpi`・`show_ok`・`mount_entities`（面 entity）。`use super::*;` で `EmoPresenter`・`TargetId`・`AlphaMaskResource`・`World` などが揃う。
- 長所: 新しい補助 0・変更は可視性キーワード 2〜4 か所。equivalence テストと同じ読み口で面 entity を引ける。
- 短所: 検体を焼く読み込みのテストが `presenter` 側に置かれる（役割で見ると「提示段の通しテスト」なので不自然ではない）。

### 案 B: テストを `shell_target` の兄弟に置き、presenter 側の補助を `pub(crate)` へ広げる

- 新規: `shell_target_clickthrough_tests.rs`（仮）。接続宣言は `shell_target.rs`。
- 変更: `presenter_test_support.rs` の `make_world_with_gpu`・`spawn_window_with_dpi`・`show_ok` を `pub(crate)` へ。面 entity は `presenter.targets` が `pub(super)` で届かないため、World を走査して `AlphaMaskResource` を持つ entity（1 個であることを較正）を引くか、`mount_entities` も `pub(crate)` へ広げる。
- 長所: 検体を読むテストが既存の 5 本と同じ場所に並ぶ。
- 短所: 広げる補助が 3〜4 個で案 A より多い。`presenter_test_support.rs` は `use super::*;` で presenter の束縛に依存しており、外から呼ぶには型の import を足す必要が出る場合がある。

### 案 C: クレート直下（`lib.rs`）に通しテストのモジュールを置く

- 両側を `pub(crate)` へ広げる必要があり、案 A・B の短所を足した形。採る理由が無い。

### 相乗り 3 件（案の差は無い）

- ①: `sample_test_support.rs` に `#[test]` を 1 本（`konnoyayame_shell_root().join("surfaces.txt").is_file()`・`emo2_root().join("shell/master").is_dir()`）。
- ②: `measure_template_tests.rs` に 3 本（または「走査対象の一覧」を配列にして 1 本で回す形）。`include_str!("../../examples/emo-present/setup.rs")` などで読み、既存の `assert_no_blind_spot_literals` → `code_only` → `!contains(SHELL_PARSE)` をそのまま使う。
- ③: 2 本の私有関数を例えば `load_shell_assets`（examples 内で公開名と取り違えない名前）へ。呼び出し 2 か所・説明文の扱いは §4 ⑶。`cargo build -p areka --examples` で確認。

## 6. 設計で決める項目（要件ディスカッションへ送る）

1. **テストの置き場と可視性の広げ方**: 案 A（`presenter` 兄弟＋受け口を `pub(crate)`）か案 B（`shell_target` 兄弟＋presenter 補助を `pub(crate)`）か。
2. **正解の復号器**: 製品と同じ `WicDecoderArm`（新しい依存 0・チャンネル順とパレット展開が同じ）か、`image` を dev 依存に足す（承認事項・並べ替えとパレット展開をテストが担う）か。
3. **面 entity の引き方**: `mount_entities`（`presenter.targets` を読む・equivalence テストと同じ）か、World の走査（`AlphaMaskResource` を持ち `ChildOf(window)` の entity が 1 個であることを較正して引く・私有状態に触れない）か。
4. ~~**「製品コード 0 行」の数え方**~~ → 要件ディスカッションで解決（要件 1.10）。
5. **差し替え 3 段の確定**（§2.5 の候補のどれを採るか・合成段の文言の修正・道連れの赤の一覧を記録に含める形式・走らせ方＝3 クレートの `cargo test`）。
6. **3 つの面の通し方**: 1 target に面 0 → 面 10 を続けて適用する（外形変化＋マスクの輪番を踏む・実機と同じ）か、検体ごと・面ごとに World と target を作り直す（判定の独立性が高い）か。`konnoyayame` は別の World が要る（面の表は装着で消費される）。
7. **食い違いの報告形式**: 件数と先頭 N 件の座標（`shell_target_template_tests.rs` の「先頭 5 件」の形が前例）。
8. **本文走査の対象**: `load_shell_target` を呼ぶ 3 本だけか、`use` で名前を持ち込む `emo-present.rs`・`collision-probe.rs` も含めた 5 本か（含めても今日は 0 件・費用は 2 行）。
9. **改名後の名前**（説明文の扱いは要件ディスカッションで解決＝旧名を指す説明文 4 か所とも改める・要件 3.3）。
10. **較正の置き場（相乗り①）**: `sample_test_support.rs` の中（present 側と同じ形・触るファイル 1 本）か、別ファイルか。

## 7. 規模とリスク

- **規模: S**（1〜2 日・タスク 3〜4 本）。通しテスト 1 本（既存の補助で組める）・差し替えの実証 3 段（各 1 行の一時変更と記録）・相乗り 3 件（いずれも数行）。
- **リスク: 低**。到達経路・読み口・GPU 前提はすべて既存テストが日常的に踏んでいる。未知の技術は無い。残る不確かさは §8 の 2 件で、どちらも設計で閉じる。

## 8. 設計へ持ち越す調査項目（Research Needed）

1. **`with_com_initialized` と `make_world_with_gpu` の併用**: 前者は末尾で `CoUninitialize` を呼び、後者は初期化だけで解放しない。案 A で `WicDecoderArm::new` を `make_world_with_gpu` の後に置けば前者は不要になる見込みだが、テストの並列実行（各テスト専用スレッド）で COM の参照回数が崩れないことを 1 度確かめる。
2. **実行時間**: 実物の PNG を 2 検体ぶん焼き（既存の template テストと同量）、GPU 付き World を 1〜3 個作る（既存の equivalence テストは DPI 3 水準で 3 個）。既存の枠内に収まる見込みだが、面ごとに World を作り直す案（§6 の 6）を採るなら計測しておく。

## 9. 設計フェーズの記録（2026-09-23・`/kiro-spec-design`）

> 対象は本ブランチ HEAD `0c63a0f6`（製品コードは main `92f5f448` と同一）。§6 の 10 項目と §8 の 2 項目をここで閉じた。引用した定義はすべて実物を Grep／Read で再確認した。

### 9.1 発見の範囲

- **Discovery Scope**: Extension（既存の presenter 系テストと同じ読み口で組む・新しい技術 0・外部調査 0）。`design-discovery-light.md` の手順で、統合点（3 段の入口と読み口）・既存パターン（兄弟ファイル＋接続宣言・受け口の共有）・互換性（可視性の壁）だけを見た。
- **Key Findings**:
  1. §2.2 の可視性の壁は関数 2 本だけでは越えられない。`shell_target.rs` が繋ぐ `mod test_support;` は**私有モジュール**なので、中の関数を `pub(crate)` にしても `presenter` 配下からは届かない（Rust の可視性はモジュールの経路ごとに検査される）。`mod` 宣言そのものを `pub(crate) mod test_support;` にする 1 行が追加で要る。この 1 行は `#[cfg(test)]` の付いた項目なので要件 1.10 の製品コードの変更行数には含めず、可視性の書き換えとして数える（合計 3 行＝`mod` 1・関数 2）。
  2. `ShellTarget::build_world` は「スコープの数だけ呼んでよい」（同関数の doc）、`atlas()` は `&AtlasTable` で `AtlasTable` は `#[derive(Clone)]`（`Arc` 共有・安価）。ゆえに `konnoyayame` に別の wintf World は要らず、GPU 付き World 1 個・presenter 1 個に target 3 本を装着すれば足りる（§6 の 6 の「別の World が要る」は面の表＝`EmoWorld` の話であり、`build_world` を target ごとに呼べば済む）。
  3. `make_world_with_gpu` は `CoInitializeEx(None, COINIT_MULTITHREADED)` を呼び解放しない。`WicDecoderArm::new` は「COM は呼び出しスレッドで初期化済みでなければならない」（同関数の doc・`CoCreateInstance`）。ゆえに `make_world_with_gpu` の**後**に `WicDecoderArm::new` を呼べば `with_com_initialized` は要らない。

### 9.2 設計で決めた項目（§6）の裁定

| § | 項目 | 裁定 | 理由 |
|---|---|---|---|
| 6-1 | 置き場と可視性 | **案 A**: `crates/areka-emo-present/src/presenter_keycolor_clickthrough_tests.rs`（`presenter.rs` に接続宣言 3 行）＋ `shell_target.rs` の `mod test_support` を `pub(crate) mod` へ（1 行）＋ `shell_target_test_support.rs` の `r_post_and_komainu_shell_dir`・`konnoyayame_shell_dir` を `pub(crate)` へ（2 行） | 広げる数が 3。案 B は presenter 側の補助 4 本＋`mod` 1 の 5 で多く、`presenter_test_support.rs` が `use super::*;` で presenter の束縛に依存している点でも不利。案 C は両方を広げる必要があり採る理由が無い |
| 6-2 | 正解の復号器 | **`WicDecoderArm::decode`**（`DecodedImage` の公開欄 `bgra`/`stride`/`width`/`height`/`has_alpha`） | 新しい依存 0・チャンネル順とパレット展開が製品と同じ・要件 1.3 の「復号した生の画素」そのもの。復号は要件が「導かない」と定めた 4 段（正規化・焼き・合成・マスク）の外。復号器が壊れて全画素が同色になれば要件 1.5 の較正が止める。`image` crate は承認事項かつ並べ替え・パレット展開をテストが担うことになるので不採用 |
| 6-3 | 面 entity の引き方 | **`mount_entities(&presenter, target).0`**（`presenter_test_support.rs`・`presenter.targets` を読む） | `presenter_budget_equivalence_tests.rs` の ⑶ と同じ読み口。案 A なら `pub(super)` のまま届く。World の走査は書く量が増えるだけで利点が無い |
| 6-4 | 数え方 | 要件 1.10 で解決済み | — |
| 6-5 | 差し替え 3 段 | 焼く＝`normalize.rs` の `(On, KeyColor)` の腕の `let key = Self::key_color(&img, params, has_pna);` → `let key: Option<[u8; 4]> = None;`／合成＝`blit.rs` の `execute` の `dst[di + 3] = source_over_channel(src_a, dst_a, inv_src_a);` → `dst[di + 3] = 255;`（転写した画素の α を捨てて不透明で書く）／マスク＝`show.rs` の `apply_show` の `regenerate_mask` 第 2 引数 `display.bytes()` → `&vec![255u8; display.bytes().len()]`。走らせ方は 3 クレートの `cargo test`。記録の形式は design.md の表 | いずれも「透明を運ぶ段の入力や出力を別の物に置き換える」形。焼く段を `key_color` 側でなく腕の入力で外すのは、`bake` の `debug!` の記録がそのまま出る＝記録だけでは退行に気付けないことを同時に示すため |
| 6-6 | 3 面の通し方 | **World 1 個・presenter 1 個・target 3 本**（`TargetId(0)`＝`R_POST` 面 0・`TargetId(1)`＝同 面 10・`TargetId(2)`＝`konnoyayame` 面 0・窓 entity は target ごとに DPI 96）。`R_POST` は `load_shell_target` 1 回・`build_world()` 2 回 | 起動側 `build_boot_assets` がスコープごとに面の表を組むのと同じ形（面 0 と面 10 は本番でも別の target）。外形変化＋輪番の経路は `presenter_resize_report_tests.rs` 等が既に持つので本テストで踏まない。判定の独立性は target 単位で保たれる |
| 6-7 | 食い違いの報告 | 件数＋先頭 5 件の `(x, y, 期待は内か)` | `shell_target_template_tests.rs` の前例に揃える |
| 6-8 | 本文走査の対象 | `load_shell_target` を**呼ぶ** 3 本だけ（`emo-present/setup.rs`・`collision-probe/setup.rs`・`window-placement.rs`） | 要件 3.2 の定めどおり。`use` だけの 2 本は含めない（含める理由が要件に無い） |
| 6-9 | 改名後の名前 | **`load_shell_assets`** | 公開の `build_shell_target`・`measure.rs` の `build_shell_assets`・`window-placement.rs` の `build_shell_material` のどれとも異なる。リポジトリ内の同名は 0 件（実測） |
| 6-10 | 較正の置き場 | `crates/areka-emo-compose/src/sample_test_support.rs` の中 | 触るファイル 1 本・present 側と同じ形 |

### 9.3 持ち越し（§8）の解決

1. **`with_com_initialized` と `make_world_with_gpu` の併用**: 併用しない。`make_world_with_gpu()` を最初に呼び（MTA 初期化・解放なし）、その後に同じスレッドで `WicDecoderArm::new()` を呼ぶ。`with_com_initialized` は末尾で `CoUninitialize` を呼ぶので、GPU 資源が生きている間に COM を解放する形を避ける。各テストは専用スレッドで走り、COM の参照回数はスレッドごとなので他のテストと交差しない（既存の presenter 系テスト 15 本と `display_gpu_tests.rs` の `gpu_dc` が同じ形）。
2. **実行時間**: 較正の実測（2026-09-23・本ブランチ・`cargo test -p areka-emo-present --lib -- template_tests budget_equivalence_tests`）＝実物の焼き 2 検体 × 3 テスト＋GPU 付き World 3 個 × 2 テストの 6 本が **0.86 秒**で完了。本テストは World 1 個・焼き 2 検体・`ShowSurface` 3 回・復号 3 枚・約 233,000 画素の比較で、その 1 テストぶん未満。追加の計測は要らない。

### 9.4 統合（synthesis）の結果

- **一般化**: しない。3 面の判定は同じ私有 fn 2 本（正解の決め方・判定）で回すが、テストファイルの外へ出す補助は 0（本テスト以外に使い手が無い）。
- **Build vs Adopt**: 既存の補助（`make_world_with_gpu`・`spawn_window_with_dpi`・`show_ok`・`mount_entities`・受け口 2 口）と既存の復号器（`WicDecoderArm`）を採る。新設 0・新しい依存 0。
- **簡素化**: `with_com_initialized`・`emo2_shell_dir`・`capture_events` は広げない（使わない）。実測の画素数（59,831 等）はテストに固定しない（検体の絵の差し替えで赤になり退行検出と混ざる）。面ごとの World 作り直しはしない。

### 9.5 リスクと対策

- `make_world_with_gpu` が HARDWARE デバイスを要る — 既存の 15 本と同じ前提であり本仕様で増える前提ではない。
- `konnoyayame` 面 0 の `animation0`（まばたき・`sometimes`）— `PatternState::default()` で適用すればコマは重ならない（`shell_target_template_tests.rs` の前提と同じ）。
- 検体の PNG が α 付きに差し替わる — `has_alpha == false` の前提の主張で止まり、原因が名指しされる。
- 差し替えを戻し忘れる — 手順に `git checkout -- <ファイル>` と `git diff --stat` が空であることの記録を含める。
