# Implementation Plan

> 正本は `requirements.md`（要件）と `design.md`（設計）。コードは「何の定義か（関数名・型名）＋ファイル」で指す。検体は `sample_ghost_kit::SampleRoot::acquire` 経由でのみ受け、`vendors/sample_ghost/<検体名>/` の直パスを書かない（要件 7.11）。新しいテストはすべて新しいファイルに置き、1 ファイル 1,000 行以内に収める。行数の検査（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の例外表には触らない（要件 7.12）。ログを判定する檻は共有機構 `log_capture_kit::capture` を使い、捕捉窓を各所で書き写さない。

- [x] 1. 基盤: 検体の受け口

- [x] 1.1 `areka-seriko` と `areka` の 2 クレートに検体 2 体の受け口を足す
  - `crates/areka-seriko/src/sample_test_support.rs` と `crates/areka/src/placement/placement_shared_test_support.rs` に、`konnoyayame` と `R_POST_and_KOMAINU` の受け口を、既存の `emo2` と同じ `LazyLock<SampleRoot>` の形で足す
  - 受け口はシェルのフォルダ（`shell/master/`）の絶対パスを返す。`vendors/sample_ghost/` の直パスは書かない
  - `areka-emo-present` の受け口は、既存の `balloon_test_support.rs` が `pub(super)` で外へ出ないため別に要る。作るのは最初の利用者であるタスク 4.3 で、本タスクの範囲外である
  - 観測可能な完了: 両クレートで、受け口の返すフォルダに `surfaces.txt` が実在することを確かめるテストが緑になる
  - _Requirements: 7.11, 7.12_

- [x] 2. 互いに独立な 3 本と、それに追随する `emo2` の期待値

- [x] 2.1 (P) 抜き色の腕を実装する
  - `crates/areka-emo-atlas/src/normalize.rs` に `(UseSelfAlpha::On, AlphaSource::KeyColor)` の腕と `Normalizer::key_color` を足す。`normalize` のシグネチャは変更 0
  - 抜き色は受け取った 32bit 乗算済み BGRA バッファの座標 (0,0) の 4 バイト。完全一致（許容幅 0）で比べ、一致した画素に `0,0,0,0` を書き（色を残さない）、一致しない画素は 1 バイトも変えない。行の詰め物を読まないよう `stride` と `width` で行ごとに歩く
  - 幅か高さが 0 の絵はそのまま渡す。`.pna`・`full`・`Off` の腕は変更 0
  - `crates/areka-emo-atlas/src/lib.rs` の `bake` に抜き色の `debug!`（`set`・`rel_path`・`b`・`g`・`r`・`a`）を 1 本足す。`AlphaSource::KeyColor` の説明の「未実装」を事実に直す
  - `crates/areka-emo-atlas/src/normalize_key_color_tests.rs` を新設し、離れた同色も透明・1 成分だけ 1 違う色は不透明のまま 1 バイトも不変・透明にした画素が `0,0,0,0`・全画素同色の絵が `Ok` で全画素透明・α 付きは 1 バイトも不変・`stride > width*4` で詰め物を読まない・`tRNS` 相当（左上が透明・他に半透明）で半透明が残る・`key_color` が `On`＋α なしのときだけ `Some` を返す、を確かめる
  - 本体内テスト `on_no_alpha_no_pna_selects_keycolor_seam` を消さずに、抜かれた結果を確かめる形へ書き換える。`off_no_pna_selects_keycolor_seam` は据え置く
  - 観測可能な完了: 新しいテストと書き換えたテストが緑で、許容幅を 1 にすると「1 成分だけ 1 違う色」が赤になる
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10, 5.4, 6.3, 7.3, 7.12, 10.1, 10.2_
  - _Boundary: areka-emo-atlas Normalizer_

- [x] 2.2 (P) 間隔の語 `sometimes`・`rarely` を読み替える
  - `crates/areka-seriko/src/table.rs` の `AnimationTable::from_world` で、`Interval::Other(語)` の腕の中で `sometimes` を `LoopTrigger::Random { k: 2 }`・`rarely` を `Random { k: 4 }` として既存の手順（`k == 0` の検査・コマの整列・空の検査）へ流す。比べ方は小文字の完全一致。`LoopTrigger` の枝の追加 0・再生の仕組みは変更 0
  - 読み替えたときに `debug!` を 1 本出す（欄: `surface_id`・`animation_id`・`vocab`＝元の語・`k`）
  - 冒頭の「採録規則」の説明と `Interval::Other` の腕の注記を「`sometimes`・`rarely` は採る／他の語は元の語つきの `debug!` を出して採らない」へ直す
  - 本体内テスト `only_random_and_bindrandom_are_recorded_others_debug_logged` を**消さずに**書き換える: 「採らない語」の代表を `Other("sometimes")` から `Other("always")` へ差し替え、面 30 の表が空であることと `vocab="always"` が残ることを引き続き確かめる
  - `crates/areka-seriko/src/table_interval_words_tests.rs` を新設し、`sometimes` の表の項目が `Random{k:2}` と・`rarely` が `Random{k:4}` と等しいこと、`always`・`runonce`・大文字 `Sometimes` は採られず元の語が `debug!` に残ること、読み替えの `debug!` に `vocab=sometimes` が残ることを確かめる
  - 検体 `konnoyayame` の `surfaces.txt` から組んだ表で、面 0 のアニメ 0 が採られている（今日は 0 件）ことを確かめる
  - 観測可能な完了: `cargo test -p areka-seriko` が緑で、読み替えを経路から外すと `sometimes`＝`Random{k:2}` と `konnoyayame` のアニメ 0 の 2 本が赤になる
  - _Requirements: 7.10, 7.12, 10.7, 11.1, 11.2, 11.3, 11.4, 11.6, 11.7_
  - _Depends: 1.1_
  - _Boundary: areka-seriko AnimationTable_

- [x] 2.3 (P) ファイル名から面の番号を得る判定を実装する
  - `crates/areka-emo-present/src/balloon.rs` の `face_id_of` の 3 段判定（接頭辞を大小無視で外す → `.png` を外す → 残りが空でなく全部 ASCII 数字）を `pub(crate) fn face_digits_of(prefix, name) -> Option<String>` へ切り出し、`face_id_of` はそれを呼んで `parse::<u32>().ok()` するだけにする。戻り値の変更 0
  - `crates/areka-emo-present/src/shell_target.rs` を新設し、`select_surface_images`（fs に触らない純粋な関数）と `SurfaceImageSelection`（`images`・`duplicates`・`overflow`）を置く。番号は 10 進として読み、先頭の 0 を無視する。同じ番号に複数あれば辞書順で最小を採り `duplicates` に積む。`u32` に収まらない数字列は `overflow` へ入れる。この段では記録を出さない（出すのは 4.2）
  - `crates/areka-emo-present/src/lib.rs` に `pub mod shell_target;` と再輸出を足す
  - `crates/areka-emo-present/src/shell_target_names_tests.rs` を新設し、4 表記がすべて面 0・`surface0010.png` が面 10・要件 1.3 の各形（`menu_background.png`・`surfaces.txt`・`surfacetable.txt`・`surface+0.png`・`surface-1.png`・`surface.png`・`surface0.pna`・`surface0.jpg`）が 0 件・`SURFACE0.PNG` が面 0・重複で `surface0.png` を採り `duplicates` 1 件・`surface99999999999.png` が `overflow` 1 件・入力の順を入れ替えても結果が同じ、を確かめる（純粋な関数なので fixture も検体も要らない）
  - 観測可能な完了: `cargo test -p areka-emo-present` が緑で、既存の `balloon_series_tests.rs` が書き換え 0 件で緑のまま。数字列を数として読まなくすると 4 表記のテストが赤になる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.8, 2.6, 7.1, 7.12_
  - _Boundary: areka-emo-present shell_target, balloon_

- [x] 2.4 `emo2` の焼き結果を留めている既存テストと期待値を書き換える
  - 2.1 で `null.png` が焼かれるようになった結果に合わせ、`crates/areka-emo-atlas/src/emo2_e2e.rs` の `emo2_shell_all_elements_baked`・`emo2_balloon_same_bake_path_as_shell` と定数 `SHELL_NORMALIZE_SEAM_KEY` の説明、`emo2_golden.rs` の `emo2_shell_bake_is_deterministic`・`emo2_shell_matches_golden` を、**消さずに**「失敗 0 件・`null.png` は全画素が透明な絵として索引表に載る」を確かめる形へ書き換える
  - `record_golden` で `crates/areka-emo-atlas/src/testdata/emo2_shell_golden.txt` を作り直す
  - 2.1 の着地から本タスクの完了までの間は `cargo test -p areka-emo-atlas` が赤である（2.1 と同じ作業の流れで続けて行う）
  - 観測可能な完了: 期待値の差分が `0<TAB>purple/a/null.png<TAB>EMPTY orig=382x547` の **1 行の追加・削除 0 行**（54 → 55 行）であり、他の 54 行が 1 文字も変わらず、`cargo test -p areka-emo-atlas` が緑に戻る
  - _Requirements: 5.5, 5.6_
  - _Depends: 2.1_
  - _Boundary: areka-emo-atlas emo2 tests_

- [x] 3. 面の表の構築（土台の絵の決定）

- [x] 3.1 `apply_base_images` と `EmoWorld::build_with_images` を実装する
  - `crates/areka-emo-compose/src/base_image.rs` を新設し、`apply_base_images` と `BaseImageReport`（`used`・`shadowed`）を置く。`crates/areka-emo-compose/src/lib.rs` に `pub mod base_image;` と再輸出を足す
  - `crates/areka-emo-compose/src/world.rs` に `EmoWorld::build_with_images(shell, &BTreeMap<u32, String>)` と `EmoWorld::base_images()` を足し、`EmoWorld::build` を「画像 0 件で `build_with_images` を呼ぶ」に置き換える（既存の呼び手の変更 0 件）
  - `SurfaceImages(BTreeMap<u32, String>)` と `BaseImageReport` を `Resource` として面の表に置く
  - 判定は**畳み込み（`fold_shell`）の後・展開後の番号ごと**に下す: 面が無ければ画像 1 枚を層 0 に持つ面を新設（ア）／面が在って層 0 の `NormalizedElement` が無ければ画像を層 0・位置 (0,0)・`Overlay` で足して層の昇順に並べ直す（ア・イ）／層 0 が在れば何もせず `shadowed` に数える（ウ）／画像の無い番号には触らない（エ）。宣言も画像も無い番号は「存在しない面」のままで、`\s[N]` が指したときの既存の `error!`（前の絵を残す）も変更 0
  - 画素は持たない（`EmoWorld` の既存の不変条件のまま）。新しい層の型・新しい欄は 0 個
  - `crates/areka-emo-compose/src/base_image_tests.rs` を新設し、複数番号の見出し（`surface0,1`）で面 0 と面 1 がそれぞれ自分の画像を受け取ること、画像 0 件で組むと適用前と同じ面の表になること、`used` と `shadowed` のキーが重ならず和が渡した画像の全体に等しいことを確かめる
  - 観測可能な完了: `cargo test -p areka-emo-compose` の既存テストが書き換え 0 件で緑のまま、新しいテストも緑になる
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.7, 3.3, 3.9, 5.1, 5.3, 7.12, 10.3, 10.5_
  - _Boundary: areka-emo-compose base_image, world_

- [x] 3.2 `surface.append` が画像だけの面にも効くようにする
  - `crates/areka-emo-compose/src/fold.rs` の `fold_append` で、対象の番号が `SurfaceIndex` に無く `SurfaceImages` に在るとき、空の `SurfaceMaster`（`elements`・`collisions`・`animations` が空）をその番号で作ってから今と同じ追記を行う
  - どちらにも無ければ今の `warn!`（「surface.append 対象 id が未存在: 新設せずスキップ」）のまま（変更 0）
  - 画像だけの面への追記の後に同じ番号の波括弧が来た場合は、既存の全置換の規則と `warn!` のまま（偽の重複警告は 0 件）
  - `base_image_tests.rs` に、`surface.append` が後から `element0` を足した面が `shadowed` になること（追記で足した層 0 も「在る」に数える）を足す
  - 観測可能な完了: 画像だけの面への追記が反映された面が `build_with_images` の結果から引け、どちらにも無い番号では面ができず既存の `warn!` が出る
  - _Requirements: 3.7, 3.8, 10.4_
  - _Depends: 3.1_
  - _Boundary: areka-emo-compose fold_

- [x] 3.3 相手の面が無いコマを数える照会を足す
  - `crates/areka-emo-compose/src/world.rs` に `EmoWorld::dangling_pattern_targets() -> BTreeSet<(u32, u32)>` を足す
  - 母集合は全部の面の全部の `animation` の全部の `pattern` のうち、`surface_id >= 0` で、かつメソッドが `start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert` の**どれでもない**もの（この 7 語は欄 2 が面の番号でなくアニメーションの番号）
  - 語の比べ方は `ComposeMethod::from_name` と同じ（前後の空白を落とし・小文字にし・`-` と `_` を除く）が、`from_name` は未知の語で `warn!` を出すのでこの照会からは**呼ばない**
  - 重複を除く鍵は `(u32, u32)` の組で、文字列へ連結しない。画像だけで存在する面は「在る」に数える。表を 1 度なめるだけで、毎フレームの経路ではない
  - `base_image_tests.rs` に、負の番号を含めないこと・画像だけの面を「在る」に数えること・`start,5` で面 5 が無くても 0 組であることを足す
  - 観測可能な完了: 画像の対応を渡した `konnoyayame` で 0 組、対応を外すと 3 組（面 0 → 1031・1032・1033）が返る
  - _Requirements: 3.5_
  - _Depends: 3.1_
  - _Boundary: areka-emo-compose world_

- [x] 4. シェルの読み込みの権威

- [x] 4.1 `load_shell_target`・`build_shell_target`・`ShellTarget` の型と核を実装する
  - `shell_target.rs` に、fs を触る入口 `load_shell_target(shell_dir, decoder)` と、fs を触らない核 `build_shell_target(shell, selection, shell_dir, decoder)`、値 `ShellTarget`（`atlas()`・`bake_errors()`・`build_world()`）、`thiserror` の `ShellLoadError`（`List`・`Read`・`Empty`）を置く
  - 一覧はフォルダ**直下**の**ファイルだけ**（`file_type().is_file()`）。フォルダとサブフォルダの中身は 0 件。バルーンの `enumerate_file_names` は失敗の型がバルーン専用でフォルダを除かないので流用しない
  - 順序は「一覧 → `surfaces.txt` の読取と解析（`charset::decode(&bytes, DefaultEncoding::Ansi)` は今と同じ）→ 面の表（`build_with_images`）→ 使う画像を聞く → 焼く」。`ShellTarget::build_world` は `build_with_images` → `bind_atlas(SetId(0))` を毎回新しく行う
  - 焼く絵の一覧は「`shell.surfaces` の複製＋使う画像 1 枚につき `element0 = ファイル名, 0, 0` だけを持つ `Surface` 1 個」。`SurfaceSet`・`ManifestDeriver::derive`・`bake` の変更は 0 件。パスの綴りは層 0 の `ElementPath` と完全一致させる（`AtlasTable::resolve` は文字列の完全一致で引く）
  - 観測可能な完了: `emo2` のシェルで `load_shell_target` が `Ok` を返し、`bake_errors()` が 0 件・`base_images()` が `used` 0 件／`shadowed` 2 件になる
  - _Requirements: 1.7, 3.6_
  - _Depends: 2.3, 3.1, 3.3_
  - _Boundary: areka-emo-present shell_target_

- [x] 4.2 権威の記録一式を実装する
  - すべて `load_shell_target` の中で、**読み込み 1 回につき 1 度だけ**出す。`build_world` は新しい記録を 0 本
  - 一覧の結果を `info!`（`shell_dir`・`recognized`・`used`・`shadowed`）／`element0` が在って使わなかった画像を面ごとに `debug!`（`surface_id`・`file`）／同じ番号の重複を番号ごとに `warn!`（`surface_id`・`adopted`・`dropped`）／大きすぎる番号を名前ごとに `debug!`（`file`）／相手の無いコマを組ごとに `warn!`（`surface_id`・`target`）／焼く段で落ちた絵を絵ごとに `warn!`（文言に実機確認が数える語「shell bake で脱落した element」を含める）
  - 失敗は `error!`＋`Err`: フォルダの一覧が取れない（`shell_dir`・`error` 付き）・`surfaces.txt` が読めない・面が 0 個。**一覧の中の 1 件が取れない場合は `warn!` を出して その 1 件を飛ばして続行する**（バルーンと同じ扱い）。記録の無い失敗経路を持たない
  - 文言はスコープの接頭辞（`shell:`）で始め、値は構造化フィールドで渡す（steering `logging.md`）。target は既定（`areka_emo_present::shell_target`）
  - 観測可能な完了: `emo2` のシェルで `recognized=2 used=0 shadowed=2` の `info!` が 1 行、使わなかった画像の `debug!` が 2 行、重複・相手の無いコマ・脱落の `warn!` が 0 行になる
  - _Requirements: 1.5, 1.6, 3.5, 6.1, 6.2, 6.4, 6.5_
  - _Depends: 4.1_
  - _Boundary: areka-emo-present shell_target_

- [x] 4.3 表ア〜エと `surface.append` の結合テストを書く
  - `crates/areka-emo-present/src/shell_target_test_support.rs` を新設し、一時フォルダ（`balloon_test_support.rs` の `TempDir` と同型）・検体の受け口（`LazyLock<SampleRoot>`）・COM 初期化・ログの捕捉窓（`log_capture_kit::capture` へ委譲するだけ）を置く。捕捉窓の中身は書き写さない
  - `crates/areka-emo-present/src/shell_target_base_image_tests.rs` を新設し、メモリ上の復号器で `build_shell_target` を通す
  - 表の 4 通りを、画像の実寸の違いが外形に現れる入力で確かめる: ア＝外形が画像の実寸／イ＝画像と `element1` を合わせた外形で画像が奥／ウ＝`element0`（小）と画像（大・別の絵）で外形が `element0` の実寸／エ＝`EmptyComposition`
  - `surface.append` が画像だけの面に効くこと、どちらも無い番号では既存の `warn!` が出て面ができないことを確かめる
  - 観測可能な完了: 4 通りの外形がそれぞれ期待どおりで、層 0 の判定を外すとウが赤になる
  - _Requirements: 7.2, 7.7, 7.11, 7.12_
  - _Depends: 4.1, 3.2_
  - _Boundary: areka-emo-present shell_target tests_

- [x] 4.4 `emo2` の不変・摂動・記録のテストを書く
  - `crates/areka-emo-present/src/shell_target_emo2_tests.rs` を新設する（4.3 の `shell_target_test_support.rs` を使う）
  - `load_shell_target` が返した 1 つの `ShellTarget` に対して、A＝`ShellTarget::build_world()`（権威経由）と、B＝同じ `ShellTarget::atlas()` を装着した `EmoWorld::build(shell)`（画像 0 件）の 2 つの面の表を組み、`surface_ids()` の**全部の面**で外形と全画素が一致することを確かめる。索引表を権威経由にするのは、`flatten_extent` が索引表で引けない層を記録なしで飛ばすため（別に焼くと壊れていても緑になる）
  - `base_images()` が `used` 0 件・`shadowed` 2 件（面 0・面 10）であることを確かめる
  - 4.2 の記録を檻に入れる: 捕捉窓の中で `load_shell_target` を 1 回呼び、`info!` が 1 行で `recognized=2`・`used=0`・`shadowed=2` を持つこと、使わなかった画像の `debug!` が 2 行（面 0・面 10）であること、重複・相手の無いコマ・脱落の `warn!` が 0 行であることを判定する（印字するだけにしない）
  - 観測可能な完了: `apply_base_images` の層 0 の判定を外すと、A の面 10 が 336×400 → 427×463 になり、A／B の一致と記録の `shadowed=2` の両方が赤になる（面 0 の形には頼らない）
  - _Requirements: 5.1, 5.2, 5.3, 5.8, 6.1, 6.2, 6.4, 7.8, 7.12_
  - _Depends: 4.2, 4.3_
  - _Boundary: areka-emo-present shell_target tests_

- [x] 5. 呼び手 5 か所を権威へ寄せる（統合）

- [x] 5.1 本番 2 か所を `load_shell_target` の呼び出しに置き換える
  - `crates/areka/src/emo2_boot/assets.rs` の `build_boot_assets` を `load_shell_target` 1 回＋scope の数だけ `build_world()` に置き換える。`atlas` は `target.atlas().clone()`。`descript.txt` の読取とバルーンの組み立ては変更 0
  - `crates/areka/src/placement/measure.rs` の `build_shell_assets` を `load_shell_target` → `build_world()` 1 回 → `(world, atlas.clone())` に置き換える
  - `crates/areka/src/emo2_boot/mod.rs` に `impl From<ShellLoadError> for BootWiringError` を置く（`List`・`Read` → 既存の `ShellRead`、`Empty` → 既存の `ShellEmpty`。枝の追加 0）。`PlacementError` へは 3 つとも既存の `Measure { scope: 0, reason }`
  - 「既知の α 無し `null.png` が落ちる」の注記を消す
  - 観測可能な完了: `measure_tests.rs` の `SCOPE0_W`／`SCOPE0_H`／`SCOPE1_W`／`SCOPE1_H`（434／687／336／400）が**書き換え 0 件**で緑のまま、`assets_tests.rs` も書き換え 0 件で緑
  - _Requirements: 3.1, 3.6, 5.7_
  - _Depends: 4.2, 2.4_
  - _Boundary: areka emo2_boot, areka placement_

- [x] 5.2 `examples` 3 本を同じ置き換えにする
  - `crates/areka/examples/emo-present/setup.rs`・`crates/areka/examples/collision-probe/setup.rs`・`crates/areka/examples/window-placement.rs` の「読む → 解析 → 焼く → 組む」を `load_shell_target` の呼び出しに置き換える
  - `read_to_string`（UTF-8 だけ）が本番と同じ文字コードの扱いになる。`emo2` の `surfaces.txt` は `charset,UTF-8` なので結果は同じ
  - 3 か所の `null.png` の注記を消す
  - 観測可能な完了: `cargo build --examples -p areka` が通り、3 本の本文に `shell::parse(` の呼び出しが 0 件になる
  - _Requirements: 5.7_
  - _Depends: 4.1_
  - _Boundary: areka examples_

- [x] 5.3 採寸と表示が同じ結果になることを確かめる
  - `crates/areka/src/placement/measure_template_tests.rs` を新設する（`measure_tests.rs` は 981 行で追記の余地が無い）
  - 検体 2 体について、`build_shell_assets` → `compose_size` の外形と、`load_shell_target` → `build_world` → `Composer::compose` の外形が一致することを確かめる
  - 併せて、`assets.rs` と `measure.rs` の本文に `shell::parse(` の呼び出しが 0 件であること（複製が戻ったら赤になる）を確かめる
  - 観測可能な完了: `R_POST_and_KOMAINU` の 236×462／140×160 と `konnoyayame` の 260×390／200×200 が、2 つの経路で一致する
  - _Requirements: 3.6, 7.6, 7.12_
  - _Depends: 5.1, 1.1_
  - _Boundary: areka placement tests_

- [x] 6. 検体 2 体の決定論テストと摂動

- [x] 6.1 検体 2 体を実物の絵で焼くテストを書く
  - `crates/areka-emo-present/src/shell_target_template_tests.rs` を新設する（4.3 の `shell_target_test_support.rs` を使う）
  - `R_POST_and_KOMAINU`: 面 0＝236×462・面 10＝140×160・宣言の無い面 10 が `build_world().surface(10)` で引ける・面 0 の合成結果の左上の画素の α が 0・`bake_errors()` が 0 件
  - `konnoyayame`: 面 0＝260×390・面 10＝200×200・`bake_errors()` が 0 件
  - `konnoyayame` の面 0 のまばたきを、`PatternState` に「アニメ 0 → 面 1031・位置 93,103」を入れた合成と入れない合成の比較で確かめる: **違う画素が 1 つ以上在り、違いが矩形 (93,103)〜(165,133) の中だけ**に在る（素通りなら違いが 0 になる）
  - 観測可能な完了: 両検体で `bake_errors()` が 0 件になり、`konnoyayame` の比較で差分の画素が 0 でない
  - _Requirements: 3.2, 3.4, 7.4, 7.5, 7.11, 7.12, 11.5_
  - _Depends: 4.3, 2.1_
  - _Boundary: areka-emo-present shell_target tests_

- [x] 6.2 摂動を実行して記録する
  - 4 つの判断について「その判断を経路から外す」形（値をずらす形ではない）で摂動し、赤になるテストを確かめる: 先頭の 0 を無視しない → 名前の判定の 4 表記／層 0 の判定を外す → 表ウ・`emo2` の A／B・`SCOPE1_W`／`SCOPE1_H`／画像だけの面を作らない → `konnoyayame` のコマの差分・`dangling_pattern_targets`／許容幅を 1 にする → 「1 成分だけ 1 違う色」
  - 間隔の語の読み替えを外す → `sometimes`＝`Random{k:2}`・`konnoyayame` のアニメ 0 が赤になることも確かめる
  - 観測可能な完了: 5 つの摂動それぞれについて、赤になったテスト名と出力を tasks の完了記録に残し、摂動を戻して全体が緑に復すること
  - _Requirements: 7.8, 7.9, 7.10_
  - _Depends: 6.1, 5.3, 4.4, 2.2_

- [x] 7. 実機確認

- [x] 7.1 `R_POST_and_KOMAINU` を実機で確かめる
  - 32bit 補助プロセスを先にビルドし、`areka.exe <ゴーストの絶対パス> <StayseeBalloon の絶対パス>` を `AREKA_APP_SMOKE_EXIT_MS` の有界の自動終了で走らせ、`RUST_LOG=info,areka_emo_present::shell_target=debug,areka_emo_atlas=debug,areka_seriko::table=debug` でログを採る。絶対パスは `cargo run -p sample-ghost-kit --bin nar-sample-path -- <検体名>` が教える
  - ⑴ 本体側 236×462・相方側 140×160 の絵がキャラクターの形に抜かれて出る ⑵ 起動挨拶がバルーンに出る ⑶ 絵の外のクリックが背後の窓へ抜ける ⑷ 右クリックメニューの 1 項目目が辞書 `dic06_String.txt` の 3 候補のどれかで `(&R)` に下線が付く
  - 「0 件」を根拠に書くときは、同じ走行に `debug` の行が実在することを併せて示す。プロセスを pid の決めつけで止めない
  - 観測可能な完了: 4 項目の結果と、`EmptyComposition` の `ERROR` が 0 行であることを、`debug` の行の実在とともに完了記録に残す
  - _Requirements: 8.1, 8.2, 8.6_
  - _Depends: 6.1, 5.1_

- [x] 7.2 `konnoyayame` を実機で確かめる
  - 同じ定石で起動する
  - ⑴ 本体側 260×390・相方側 200×200 がキャラクターの形に抜かれて出る ⑵ 本体側がまばたきし、目の周りに四角い地色が出ない ⑶ 起動挨拶の文字が文字化けせずにバルーンに出る ⑷ `sakura.balloon.alignment,none`／`kero.balloon.alignment,none` が自動調整として効く（窓が画面の右半分ならバルーンは左隣、左半分なら右隣）
  - 観測可能な完了: 4 項目の結果と、間隔の語の読み替えの `debug!`（`vocab=sometimes`）がログに実在することを完了記録に残す
  - _Requirements: 8.1, 8.3, 8.6, 11.5_
  - _Depends: 6.1, 5.1, 2.2_

- [x] 7.3 `emo2` を実機で確かめる
  - 同じ定石で起動し、立ち絵・バルーン・撫で・メニュー・終了が適用前と同じに見えることを確かめる
  - 起動時の `warn!`「shell bake で脱落した element」が 0 回であること、`null.png` の「全透明」の `warn!` が出ていることを確かめる
  - 観測可能な完了: 3 項目の結果を、`debug` の行の実在とともに完了記録に残す
  - _Requirements: 5.9, 8.1, 8.5, 8.6_
  - _Depends: 5.1, 2.4_

- [x] 7.4 実機で見つかった妨げを本仕様の中で直す
  - 7.1〜7.3 で、検体 2 体の動作（起動・絵・起動挨拶・まばたき・クリック透過・メニュー・バルーンの位置・終了）を妨げる欠陥が見つかったら、別件へ送らず要件と設計を改訂して本仕様の中で直す
  - 直すのに独立した仕様 1 本ぶんの規模が要ると分かったときだけ、黙って先送りせず開発者に諮る。検体 2 体が使っていない機能の欠陥は、引受先を確かめて起票する
  - 観測可能な完了: 見つかった妨げが 0 件ならその旨を、1 件以上なら直した内容と改訂した要件・設計の箇所を完了記録に残す
  - _Requirements: 8.4, 10.6_
  - _Depends: 7.1, 7.2, 7.3_

- [x] 8. 文書と台帳（着地時）

- [x] 8.1 網羅台帳を実測に合わせて直す
  - `doc/ukadoc-coverage/ledger/assets.toml` の `ukadoc:descript_shell_surfaces:sometimes:1` と `…:rarely:1` の 2 件に、担当 `areka-P0-shell-implicit-surface` と実測の状態を登記する。新しい行の追加は 0 件
  - 備考 2 項目を要件の文面どおりに直す（状態と担当は変えない）: ⑴ `descript_shell_surfaces` の `element*` ⑵ `descript_shell` の `seriko.use_self_alpha,値`
  - 報告を作り直す
  - 観測可能な完了: `cargo test -p ukadoc-survey` が緑になる
  - _Requirements: 9.1, 9.3_
  - _Depends: 7.4_

- [x] 8.2 沈黙ルール対応表・roadmap・申し送りを書く
  - `doc/COMPAT_ARCHITECTURE.md` §8 に 6 行を足す: 大小無視／重複は辞書順で最小／抜き色の許容幅 0／`tRNS` は生かす／色の比較は 32bit へ変換した後の値で行う／画像だけの面への追記の後に来た波括弧は既存どおり全置換する
  - `.kiro/steering/roadmap.md` に「引き受け手の居ない残り」7 件を登記する（⑶ に「`UseSelfAlpha::Off` の下の抜き色は未実装のまま」を含める）。実在しない spec の名前は書かない
  - `roadmap.md` の本仕様の行（#10）を、裁定 2 件が正典の逐語で決まったことと範囲に抜き色が加わったことを反映して直す
  - `.kiro/specs/areka-P0-coverage-roadmap-refresh/brief.md` へ、「`dev_shell`・`manual_shell` の当該の文を項目へ割り、担当を本仕様にする」依頼を申し送る
  - 観測可能な完了: 4 つの文書それぞれの差分が存在し、`roadmap.md` に書いた引受先の spec がすべて `.kiro/specs/` に実在して `completed/` の下に無い
  - _Requirements: 9.2, 9.4, 9.5, 9.6, 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7_
  - _Depends: 8.1_

## Implementation Notes

- 2.2: レビュー差し戻しの原因は 2 往復とも「コードと食い違う古い注記の取り残し」だった。振る舞いを変えたら、**モジュール冒頭の説明・変更した関数の公開 rustdoc・分岐の直前の注記・既存テストの doc コメント**の 4 か所を必ず走査して事実へ直すこと。特に公開 rustdoc（`cargo doc` に出る）は見落としやすい。
- 2.1 → 2.4: 抜き色の腕の着地で `emo2` の `null.png` が焼かれるようになり、`emo2_e2e`・`emo2_golden` の期待値が 1 行増えた（54 → 55 行）。`areka-emo-atlas` の焼き結果に触る変更は golden の作り直し（`record_golden`）が要る。
- 4.1: 一時フォルダは自作せず共有窓口 `temp_path_kit::TempPath` を使う（`crates/log-capture-kit/tests/temp_path_guard_test.rs` が自作を拒む）。`areka-emo-present` の dev-dep に `temp-path-kit` を追加済み。
- 4.1: `shell_target.rs` と `shell_target_load_tests.rs` に「記録はタスク 4.2 が足す」「受け口はタスク 4.3 が `shell_target_test_support.rs` へ移す」の前方参照が 10 か所ある。4.2／4.3 の着地時に事実へ直すこと。
- 4.2: `emo2` の記録の檻（`recognized=2 used=0 shadowed=2`・使わなかった画像の `debug!` 2 行・`warn!` 0 行）は `shell_target_load_tests.rs` に既済。タスク 4.4 は A／B の全画素の一致が残りで、記録の檻を二重に置かないこと。
- 4.3: `surface.append` にしか現れない絵の綴りは焼かれない（`ManifestDeriver::derive` が `shell.surfaces` だけを読む）。本仕様の着地で生じた欠陥ではなく既存の性質で、焼く一覧の導出は 1 行も変えていない。検体 2 体の `surface.append` は 0 行なので影響は無いが、7.1／7.2 の実機確認で露見したらタスク 7.4 の担当。
- 4.3: tasks.md が言う `TempDir` は新設せず共有窓口 `temp_path_kit::TempPath` を使った（同型の新設は `temp_path_guard_test.rs` の例外表の編集を強いるため）。レビューで妥当と裁定済み。
- 4.4: 要件 5.8 の括弧内「誤って二重に重ねても結果のバイトが変わらず検査にならない」は実測と食い違う。摂動を当てると面 0 も画素は変わる（先頭の差は 827 バイト目）。変わらないのは**外形**だけで、縁が半透明なので重ねると色が変わる。檻は面 10 の外形で較正しているので判定は正しいが、**要件 5.8 の文面と `shell_target_emo2_tests.rs` の `SURFACE10_EXTENT` の doc を「変わらないのは外形だけ」へ直すこと**（タスク 6.2 または要件 9 の文書作業の担当）。→ **タスク 6.2 で実施済み**。要件 5.8 と `SURFACE10_EXTENT` の doc を「変わらないのは外形だけ・画素は面 0 でも変わる（先頭の差は 827 バイト目）」へ是正した。
- 4.4: `shell_target_test_support.rs` の冒頭が挙げる「檻は 4 本」のうち `shell_target_template_tests.rs` はタスク 6.1 の持ち物。6.1 の着地でファイル名が変わったらこの 1 行も追随させること。
- 5.3: 5.1 の着地で採寸側も表示側も `load_shell_target` を通るため、「2 経路の外形が一致する」だけの主張は**今日は構造的にほぼ恒真**である。要件 7.6 を実際に支えているのは ⑴ 要件 3.1 の実数（236×462／140×160／260×390／200×200）を両経路で判定していること ⑵ `assets.rs`・`measure.rs` の本文に `shell::parse(` が 0 件であることの走査、の 2 点。**報告で「2 経路の一致」だけを要件 3.6 の根拠に挙げないこと。**
- 5.3: 本文走査の自作の走査器は `'"'` の文字リテラルと生文字列を扱えず、本物の呼び出しを静かに見逃す。前提条件を檻で主張して塞いである（持ち込むと赤になる）。走査器を強くせずにこの形式を持ち込まないこと。
- 8.1: **`doc/ukadoc-coverage/briefing-assets.md` には機械の見張りが 1 つも無い。** このファイルは台帳から機械で組み直した表を貼ったものだと自分で宣言しており（同ファイル「SERIKO/MAYUNA 世代別対応表」の冒頭・「台帳の側を直したら、表も作り直して貼り直すことになる」）、状態の欄も「台帳の状態をそのまま写したもの」と書いてある。ところが `crates/ukadoc-survey/` はこのファイルを**1 行も読まない**（`briefing-assets` を `crates/` 全域に当てて 0 件）。同クレートが名前で読む文書は `linkage.md`・`values.md`・`briefing.md`・`roadmap-draft.md`・`README.md` の 5 本だけで、ドメイン別のブリーフィング 4 本（`briefing-assets.md`・`briefing-property.md`・`briefing-sakura-script.md`・`briefing-shiori.md`）はどれも入っていない。したがって **`cargo test -p ukadoc-survey` が緑でも、`cargo run -p ukadoc-survey -- check` が 0 件でも、台帳と表の食い違いには気付けない**。`briefing.md` のほうは段階と順位・件数・優先度を台帳と突き合わせる腕（`crates/ukadoc-survey/tests/consistency/briefing_checks.rs`・同 `briefing_arms.rs`・同 `spec_checks.rs`）が在るので、同じ種類の取り残しは赤になる。**この非対称がタスク 8.2 の扱う欠落である**（台帳の `status` と表の第 4 欄を id で突き合わせる腕を、ドメイン別ブリーフィング 4 本にも置くかどうか）。
- 8.1: 上の見張りが無いため、この差し戻しの是正では**手で突き合わせて 0 件を確かめた**。手順は ⑴ `ledger/assets.toml` から `[entry."<id>"]` とその直下の `status` を拾って `ukadoc:descript_shell_surfaces:` で始まる 137 件を取る、⑵ `briefing-assets.md` の表から同じ接頭辞の行を拾って第 4 欄を取る（137 行・重複 0）、⑶ id で突き合わせる、の 3 つ。結果は**食い違い 0 行**、内訳は両側とも 実装済み 7・語彙のみ 55・縮退 4・別名 4・未対応 67。是正したのは表の 3 行（`charset,文字コード` 未対応→実装済み・`rarely` 語彙のみ→実装済み・`sometimes` 語彙のみ→実装済み）と、内訳を書いた 2 か所（表の読み方の段落と ⑶ surfaces.txt の「台帳の内訳」）。実装済み 7 件がソース側に正典 URL の 1 行を持つことは `crates/` を grep して 1 件ずつ確かめた（`areka-emo-compose/src/method.rs:148`・`:149`／`areka-parsers/src/shell/decode.rs:388`・`:502`／`areka-parsers/src/charset/prescan.rs:57`／`areka-seriko/src/table.rs:131`・`:133`。この 7 件以外に同ページの正典 URL は `crates/` に無い）。
- 8.1: 表の 3 行を直すと、同じファイルの散文 2 か所が**自分の表と矛盾する**ので併せて是正した。⑴ 注記 ⑴「間隔の語は 2 語だけ」——`sometimes`／`rarely` を `random,2`／`random,4` へ読み替える腕が `crates/areka-seriko/src/table.rs:130`〜`:136` に在るので駆動するのは 4 語。⑵ ⑶ surfaces.txt の「このファイルの文字コードの宣言は読まれない」——実際は `crates/areka-emo-present/src/shell_target.rs:254`〜`:255` が `areka-parsers` の `charset::decode`（中で `prescan_charset` を呼ぶ）を通しており、宣言は読まれる。後者は本仕様より前からの誤りで、`main` の `crates/areka/src/emo2_boot/assets.rs` も同じ復号を通していた（`git show main:` で確認）。
- 8.2: **「バイトが変わらない」の同じ誤りが 2 つの文書に残るが、是正せず据え置く**（タスク 6.2 からの申し送りへの回答）。⑴ `brief.md` の「今日の壊れ方」の面 0 の箇条（`golden_tests_surface0_base_tests.rs` の byte 等価 golden について「premultiplied SourceOver で自分の上に重ねてもバイトが変わらない」と書く行）⑵ `design-validation.md` の要件 5.8 の懸念の段（「面 0 は…同じ絵の二重重ねなので要件 5.8 が言うとおり検査にならない」と書く行）。**どちらも発掘時・検証時のスナップショット文書**で、その時点の読みを記録することが役目だから、後から事実へ書き換えると「いつ何が分かっていたか」が読めなくなる。正本（`requirements.md` の 5.8 と `shell_target_emo2_tests.rs` の `SURFACE10_EXTENT` の doc）はタスク 6.2 で「変わらないのは外形だけ・画素は面 0 でも変わる（先頭の差は 827 バイト目）」へ是正済みなので、食い違いは正本の側に無い。**後から「正本と食い違う」と再発掘しないために、据え置きの理由をここに書く。**
- 8.2: **タスク 8.1 が見つけた見張りの非対称は、`.kiro/steering/roadmap.md` へ登記しない**と判断した。理由は引受先が既に在るからである——`doc/ukadoc-coverage/` の「検査の外にある手書きの数」の棚卸は `areka-P0-coverage-roadmap-refresh`（`.kiro/specs/` に実在・`completed/` の下に無い）の Scope の In そのもので、同 spec の Approach が「実装側だけを見る検査を足す」と書いている。ゆえに本タスクは同 spec の `brief.md` の申し送りの節へ事実だけを足し、roadmap の「引き受け手の居ない残り」には入れなかった（あの節は**引受先が 0 本**のものだけを並べる）。登記した事実は「`crates/ukadoc-survey/` が名前で読む文書は `linkage.md`・`values.md`・`briefing.md`・`roadmap-draft.md`・`README.md` の 5 本だけで、ドメイン別のブリーフィング 4 本はどれも入っていない（2026-09-20 に `crates/ukadoc-survey/` 全域を当てて `briefing-assets` は 0 件）」である。
- 8.2: **タスク 4.3 の申し送り（`surface.append` にしか現れない絵の綴りは焼かれない）は、「引き受け手の居ない残り」7 件の中ではなく、同じ節の**別の段**として登記した**。7 件は要件 9.5 が数を明示して列挙したものなので、そこへ 8 件目を混ぜると要件の文面と数が食い違う。一方でこの性質にも引受先は無いので、節の末尾に「上の 7 件の外だが、同じく引受先の無い既存の性質」として、`ManifestDeriver::derive` が波括弧の側だけを読むこと・本仕様が焼く一覧の導出を 1 行も変えていないこと・`R_POST_and_KOMAINU` と `konnoyayame` の `surface.append` が 0 件・`emo2` は 5 定義あるがどれも `element` 行を持たないので追記の側にしか現れる絵は 3 体とも 0 件であることを書いた。
- 8.2: **タスク 7.1〜7.4 の「自動走行では確かめられなかった 5 項目」は、新しい spec を起票せず `roadmap.md` の同じ節の末尾へ登記した**。5 項目はどれも本仕様が新しく作った経路ではなく（クリック透過・右クリックメニュー・字形の描画・バルーンの左右反転は既存の機能）、独立した仕様 1 本ぶんの規模も無いので、要件 8.4 の「別件へ送らず本仕様の中で直す」にも「独立した仕様が要るときだけ開発者に諮る」にも当たらない。開発者の目が通る場所として `areka-P0-alpha-release-signoff`（`.kiro/specs/` に実在・`completed/` の下に無い・第三者の手順 11 項目の実機サインオフ）の実機一周を挙げたが、**同 spec の brief は本タスクの範囲外なので触っていない**。同 spec が要件段階でこの 5 項目を拾えるよう、再現の手順の在りか（本ファイルの「実機確認の記録」の「共通の条件」）を roadmap 側に書いてある。

## 摂動の記録（タスク 6.2・2026-09-20）

5 つの摂動を 1 つずつ当てて赤を確かめ、そのつど元へ戻した。摂動はすべて「その判断を経路から外す」形で、値をずらしてはいない（唯一の例外は ⑷ で、要件 7.9 が名指しする摂動そのものが「許容幅を 1 にする」である）。摂動を当てる前の緑は `areka-emo-atlas` 83 件・`areka-emo-compose` 219 件・`areka-emo-present` 254 件・`areka-seriko` 207 件（＋結合 27 件）・`areka` 1,718 件（＋結合 3 件）。

### ⑴ 先頭の 0 を無視しない

- 摂動: `crates/areka-emo-present/src/shell_target.rs` の `select_surface_images` で、数字列を 10 進数として読む段を経路から外し、字面の一致（`digits == id.to_string()`）でしか番号を採らないようにした。先頭に 0 の付く綴りは面の画像と認められなくなる。
- 走らせたコマンド: `cargo test -p areka-emo-present`
- 赤になったテスト（9 件）: `shell_target::names_tests::leading_zeros_are_ignored_and_digits_read_as_decimal`・`shell_target::names_tests::duplicate_ids_adopt_lexicographic_minimum_and_report_the_dropped`・`shell_target::names_tests::selection_is_independent_of_input_order`・`shell_target::load_tests::used_image_is_baked_under_the_same_spelling_as_layer_zero`・`shell_target::load_tests::directories_are_not_taken_as_surface_images`・`shell_target::load_tests::every_record_is_emitted_once_per_load`・`shell_target::template_tests::r_post_and_komainu_shows_both_scopes_from_file_names_alone`・`shell_target::template_tests::konnoyayame_shows_both_scopes_from_file_names_alone`・`shell_target::template_tests::konnoyayame_blink_frame_changes_pixels_only_inside_the_frame_rect`
- 出力（抜粋）: `panicked at shell_target_names_tests.rs:23:9: assertion 'left == right' failed: surface00.png は面 0 の画像である` ／ `test result: FAILED. 245 passed; 9 failed`
- 戻した後: `cargo test -p areka-emo-present` が 254 件緑。

### ⑵ 層 0 の判定を外す

- 摂動: `crates/areka-emo-compose/src/base_image.rs` の `apply_base_images` で、`master.elements.iter().any(|e| e.layer == 0)` の判定を経路から外し（条件を `false` に潰し）、層 0 が在る面にも画像を敷くようにした。
- 走らせたコマンド: `cargo test -p areka-emo-compose` ／ `cargo test -p areka-emo-present` ／ `cargo test -p areka`
- 赤になったテスト（`areka-emo-compose` 3 件）: `base_image::tests::used_and_shadowed_partition_the_images`・`base_image::tests::append_reaches_image_only_faces`・`base_image::tests::brace_after_append_replaces_the_image_only_face`
- 赤になったテスト（`areka-emo-present` 6 件）: `shell_target::base_image_tests::case_c_layer_zero_present_keeps_the_declared_extent`・`shell_target::base_image_tests::every_image_is_either_used_or_shadowed_exactly_once`・`shell_target::load_tests::every_record_is_emitted_once_per_load`・`shell_target::load_tests::emo2_shell_records_two_shadowed_images_and_no_warnings`・`shell_target::load_tests::emo2_shell_loads_with_zero_bake_errors_and_two_shadowed_images`・`shell_target::emo2_tests::emo2_every_surface_is_identical_with_and_without_base_images`
- 赤になったテスト（`areka` 6 件）: `placement::measure::tests::measure_emo2_fixture_yields_exact_nonzero_sizes`・`placement::measure::tests::measure_emo2_fixture_applies_k_end_to_end`・`placement::prepare_tests::prepare_emo2_returns_two_scope_placements`・`placement::prepare_tests::prepare_emo2_at_dpi_120_places_scopes_adjacent`・`placement::prepare_tests::prepare_emo2_scales_window_sizes_by_k0`・`placement::windowposition_tests::prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120`
- 出力（抜粋）: `面 0: 画素が違う（先頭の差は 827 バイト目 A=Some(4) B=Some(2)・長さ A=1192632 B=1192632）` ／ `面 10: 外形が違う A=427x463(stride 1708) B=336x400(stride 1344)` ／ `measure_tests.rs:101:9: assertion 'left == right' failed / left: 427 / right: 336`（`SCOPE1_W`）
- 併せて確かめた事実: 面 0 は**外形が変わらないまま画素だけが変わる**（先頭の差は 827 バイト目）。要件 5.8 と `shell_target_emo2_tests.rs` の `SURFACE10_EXTENT` の doc が「結果のバイトが変わらない」と書いていたのは誤りで、本タスクで「変わらないのは外形だけ」へ直した。
- 戻した後: `areka-emo-compose` 219 件・`areka-emo-present` 254 件・`areka` 1,718 件が緑。

### ⑶ 画像だけの面を作らない

- 摂動: 同じ `apply_base_images` の「宣言の無い番号を新設する」腕（表ア）を経路から外し、面を作らずに読み飛ばすようにした。
- 走らせたコマンド: `cargo test -p areka-emo-compose` ／ `cargo test -p areka-emo-present`
- 赤になったテスト（`areka-emo-compose` 4 件）: `base_image::tests::konnoyayame_has_no_dangling_pattern_targets_with_images`・`base_image::tests::dangling_pattern_targets_counts_image_only_faces_as_existing`・`base_image::tests::used_and_shadowed_partition_the_images`・`base_image::tests::append_to_unknown_id_still_warns_and_creates_nothing`
- 赤になったテスト（`areka-emo-present` 4 件）: `shell_target::template_tests::konnoyayame_blink_frame_changes_pixels_only_inside_the_frame_rect`・`shell_target::template_tests::r_post_and_komainu_shows_both_scopes_from_file_names_alone`・`shell_target::base_image_tests::case_a_image_only_surface_takes_the_image_extent`・`shell_target::base_image_tests::every_image_is_either_used_or_shadowed_exactly_once`
- 出力（抜粋）: `base_image_tests.rs:550:5: 画像の対応を渡すと相手の無いコマは 0 組: {(0, 1031), (0, 1032), (0, 1033)}` ／ `shell_target_template_tests.rs:198:9: コマの相手の面 1031 が面の表に居ない（要件 3.4 の解決が効いていない）`
- 戻した後: `areka-emo-compose` 219 件・`areka-emo-present` 254 件が緑。

### ⑷ 抜き色の許容幅を 1 にする

- 摂動: `crates/areka-emo-atlas/src/normalize.rs` の抜き色の腕で、完全一致（`*px == key`）を成分ごとの差 1 以内の一致へ置き換えた（要件 7.9 が名指しする摂動）。
- 走らせたコマンド: `cargo test -p areka-emo-atlas`
- 赤になったテスト（1 件）: `normalize::normalize_key_color_tests::color_off_by_one_in_a_single_component_stays_untouched`
- 出力（抜粋）: `normalize_key_color_tests.rs:69:5: assertion 'left == right' failed: B が 1 違う色は不透明のまま / left: [0, 0, 0, 0] / right: [11, 20, 30, 255]`
- 戻した後: `cargo test -p areka-emo-atlas` が 83 件緑。

### ⑸ 間隔の語の読み替えを外す

- 摂動: `crates/areka-seriko/src/table.rs` の `AnimationTable::from_world` で、`Interval::Other(語)` の腕の読み替え（`sometimes` → 2・`rarely` → 4）を経路から外し、どの語も非採録へ落ちるようにした。
- 走らせたコマンド: `cargo test -p areka-seriko`
- 赤になったテスト（4 件）: `table::interval_words_tests::sometimes_records_the_same_entry_as_random_2`・`table::interval_words_tests::rarely_records_the_same_entry_as_random_4`・`table::interval_words_tests::rewrite_is_debug_logged_with_the_original_vocab`・`table::interval_words_tests::konnoyayame_surface0_animation0_is_recorded`
- 出力（抜粋）: `table_interval_words_tests.rs:46:5: assertion 'left == right' failed: sometimes は random,2 と同じ引き金・同じコマ列で採録される / left: [] / right: [LoopAnimation { id: 0, trigger: Random { k: 2 }, frames: [...] }]` ／ `table_interval_words_tests.rs:112:5: konnoyayame の面 0 はまばたき 1 本が採録される … left: 0 / right: 1`
- 戻した後: `cargo test -p areka-seriko` が 207 件緑（＋結合 27 件）。

### 戻した後の全体

- `git status --porcelain` が空（摂動の残りは 1 行も無い）。
- `areka-emo-atlas` 83・`areka-emo-compose` 219・`areka-emo-present` 254・`areka-seriko` 207（＋結合 27）・`areka` 1,718（＋結合 3）がすべて緑。`cargo fmt -- --check` は 5 クレートとも差分 0。
- design.md の「emo2 の不変の示し方」には「バイトが変わらない」に当たる記述が無く（面 0 については「同じ絵には頼らない」とだけ書いている）、直す箇所は 0 件だった。
- 6.2: 「バイトが変わらない」の同じ誤りが `brief.md`（`golden_tests_surface0_base_tests.rs` の byte 等価 golden について）と `design-validation.md`（「同じ絵の二重重ねなので要件 5.8 が言うとおり検査にならない」）にも在る。どちらも発掘時・検証時のスナップショット文書なので是正はしないが、**タスク 8.2 でその旨を明記して据え置く**こと（後から「正本と食い違う」と再発掘されないため）。`design.md` に同じ誤りは 0 件。

## 実機確認の記録（タスク 7.1〜7.3・2026-09-20）

### 共通の条件

- 事前ビルド（PowerShell）: `cargo build -p areka` → `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` → `Copy-Item target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe target\debug\ -Force`
- 起動は**絶対パス**。ゴーストの絶対パスは `cargo run -p sample-ghost-kit --bin nar-sample-path -- <検体名>` の `folder=` 行から採った。バルーンは検体 2 体が同梱を持たないので `vendors/sample_ghost/StayseeBalloon` を渡した。
- 有界の自動終了 `AREKA_APP_SMOKE_EXIT_MS`。プロセスは pid の決めつけで止めていない（すべて自動終了で exit 0）。
- 「0 件」を主張するときは、同じ走行に `debug` の行が実在することを併せて示す。

### 7.1 `R_POST_and_KOMAINU`

走行: `AREKA_APP_SMOKE_EXIT_MS=45000`・`RUST_LOG=info,areka_emo_present::shell_target=debug,areka_emo_atlas=debug,areka_seriko::table=debug`・**exit 0**・ログ 97 行。

| 項目 | 結果 | 証跡 |
|---|---|---|
| ⑴ 本体側 236×462・相方側 140×160 が抜かれて出る | **PASS** | `apply(ShowSurface) … TargetId(0) surface_id=0 native_w=236 native_h=462`／`TargetId(2) surface_id=10 native_w=140 native_h=160`（**面 10 は `surfaces.txt` に宣言が無い＝ファイル名だけで建った**）。抜き色の `debug!` が 20 行（`rel_path="surface0000.png" b=255 g=255 r=255 a=255` ほか） |
| ⑵ 起動挨拶がバルーンに出る | **PASS** | `kanade: 起動グリーティングを再生起動 event="boot_talk" talk_id=1` → `[balloon-visibility] バルーンの可視状態が遷移した scope=0 trigger="content" visible=true` |
| ⑶ 絵の外のクリックが背後の窓へ抜ける | **未確認（開発者の手が要る）** | 構造の証跡のみ: `apply(ShowSurface): 表示・マスクを更新` が面ごとに出ており α マスクは更新されている。クリックそのものは自動走行では起こせない |
| ⑷ 右クリックメニューの 1 項目目 | **未確認（開発者の目と手が要る）** | メニューは右クリックで初めて組まれるので自動走行のログには現れない。`(&R)` の下線は Win32 のアクセラレータ表示で OS 設定に依る |
| `EmptyComposition` の `ERROR` | **0 行** | 同じ走行に `ERROR` が 1 行も無く、`shell_target`・`areka_emo_atlas` の `debug` は実在する（抜き色 20 行・`shell:` の `info!` 2 行） |
| 焼く段で落ちた絵 | **0 行** | `shell bake で脱落した element` が 0 件（`debug` の実在は上と同じ） |

### 7.2 `konnoyayame`

走行 A: `AREKA_APP_SMOKE_EXIT_MS=60000`・`RUST_LOG=info,areka_emo_present::shell_target=debug,areka_emo_atlas=debug,areka_seriko=debug`・**exit 0**。
走行 B（まばたき観測用）: `AREKA_APP_SMOKE_EXIT_MS=300000`・`RUST_LOG=info,areka_seriko=debug`・**exit 0**。

| 項目 | 結果 | 証跡 |
|---|---|---|
| ⑴ 本体側 260×390・相方側 200×200 が抜かれて出る | **PASS** | `native_w=260 native_h=390`（面 0）／`TargetId(2) surface_id=10 native_w=200 native_h=200`。抜き色の `debug!` が 36 行（**緑抜き** `b=0 g=255 r=0 a=255`）。一覧は `recognized=18 used=18 shadowed=0` |
| ⑵ 本体側がまばたきする | **PASS** | 走行 B で `seriko: loop 抽選発火 … scope="0" slot=Shell animation_id=0 k=2` が **101 回**（走行 B 全体・`ghost shutdown sequence completed` で正常終了・exit 0）。`k=2` は `sometimes` → `Random{k:2}` の読み替えそのもの |
| ⑵ 目の周りに四角い地色が出ない | **PASS（開発者の目視・2026-09-20）** | 下の「開発者の目視サインオフ」の `konnoyayame` 節を見よ。画素の証跡もある: 抜き色が全 18 枚に効いており、タスク 6.1 の檻が「差分は矩形 (93,103)〜(165,133) の中だけ」を判定済み |
| ⑶ 起動挨拶が文字化けしない | **PASS（文字の経路＋開発者の目視・2026-09-20）** | バルーンへ渡る文字が正しい日本語で並ぶ: `command=Text("初めまして。")`・`Text("わたしはややめ。紺野ややめだよ。")`・`Text("わたしは、YAYAのサンプルゴースト。")`。`charset_initial charset="Shift_JIS" source="default"`。字形の描画は開発者の目 |
| ⑷ `balloon.alignment,none` が自動調整として効く | **PASS** | `placement: windowposition を初期既定位置の調整量へ変換した … scope=0 balloon_side=Auto`／`scope=1 balloon_side=Auto`。左右どちらへ寄るかは窓の実位置に依るので、画面の左右で切り替わる様子は開発者の目 |
| 間隔の語の読み替えの `debug!` | **実在** | `seriko table: 間隔の語を random,K と同じ引き金へ読み替えて採録（要件 11.1/11.2） surface_id=0 animation_id=0 vocab="sometimes" k=2` |
| `ERROR` | **0 行** | 走行 A・B とも 0。`debug` の実在は上のとおり |

**まばたきの観測で分かったこと（欠陥ではない）**: `konnoyayame` の `animation0` は**面 0 にしか無い**（`surfaces.txt` 実測）。起動挨拶の台本は面を何度も切り替え、走行によっては面 5 や面 6 で終わる。その走行ではまばたきは 1 度も発火しない（面 0 が表示されていないので正しい）。面 0 で終わった走行 B では 101 回発火した。**areka の欠陥ではなくゴーストの台本の性質**である。`emo2` の同じ走行で 19 回発火していることが、ループそのものが動いている対照になる。

### 7.3 `emo2`

走行: `AREKA_APP_SMOKE_EXIT_MS=120000`・`RUST_LOG=info,areka_emo_present::shell_target=debug,areka_emo_atlas=debug,areka_seriko=debug`・**exit 0**・ログ 131,812 バイト。

| 項目 | 結果 | 証跡 |
|---|---|---|
| 立ち絵・バルーン・メニュー・終了が適用前と同じに見える | **PASS（ログの範囲）／視覚は未確認** | `ERROR` 0 行・`loop 抽選発火` 19 回・バルーンの可視遷移・`ghost shutdown sequence completed` で exit 0。撫でとメニューの操作は自動走行では起こせない |
| 起動時の `warn!`「shell bake で脱落した element」が 0 回 | **PASS** | 0 件。同じ走行に `shell_target` の `debug!` が実在する（`element0 が在るため面の画像を土台に使わなかった（R6.2） surface_id=0 file="surface0.png"`／`surface_id=10 file="surface10.png"`） |
| `null.png` の「全透明」の `warn!` が出ている | **PASS** | `bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547`。直前に抜き色の `debug!`（`b=0 g=0 r=0 a=0`）も出ており、抜き色の腕を通ったことが分かる |
| 記録の実測 | **要件どおり** | `shell: シェルの面の画像の一覧が終わった（R6.1） … recognized=2 used=0 shadowed=2`・使わなかった画像の `debug!` が面 0・面 10 の 2 行・重複／相手の無いコマ／脱落の `warn!` が 0 行 |

### 自動走行では確かめられなかった項目（開発者の目と手が要る）

次の 5 項目は、走っている窓に対する**操作**か**目視**が要るため、有界の自動終了＋ログ grep では確かめられない。実装側の証跡は上の表に挙げたとおりで、**いずれも本仕様が新しく作った経路ではない**（クリック透過・右クリックメニュー・字形の描画・バルーンの左右反転は既存の機能で、本仕様はそこへ絵を届けられるようにしただけである）。

1. 7.1 ⑶ 絵の外のクリックが背後の窓へ抜ける — **2026-09-20 に開発者が目視サインオフ済み**
2. 7.1 ⑷ 右クリックメニューの 1 項目目が `dic06_String.txt` の 3 候補のどれかで `(&R)` に下線が付く — **2026-09-20 に開発者が目視サインオフ済み**
3. 7.2 ⑵ 目の周りに四角い地色が出ない（画素の証跡はタスク 6.1 の檻にある） — **2026-09-20 に開発者が目視サインオフ済み**
4. 7.2 ⑶ 起動挨拶の**字形**が文字化けしない（文字の経路は上の表のとおり正しい） — **2026-09-20 に開発者が目視サインオフ済み**
5. 7.3 撫で・メニュー・終了の操作が適用前と同じに見える

### 7.4 実機で見つかった妨げ

**妨げは 0 件**である。自動走行で確かめられた範囲で、検体 2 体と `emo2` の動作（起動・絵・起動挨拶・まばたき・バルーンの位置・終了）を妨げる欠陥は 1 つも見つからなかった。3 体とも `ERROR` 0 行・正常終了（exit 0）である。要件・設計の改訂は 0 件（要件 5.8 の文面の是正はタスク 6.2 で別に済ませた）。

調査したが欠陥でなかったもの:

- **`konnoyayame` のまばたきが発火しない走行がある**（走行 A の 60 秒・別の 180 秒走行）。原因はゴーストの台本で、`animation0` が**面 0 にしか無い**のに起動挨拶が面 5 や面 6 で終わるため。面 0 で終わった走行 B では 101 回発火した。`emo2` の同じ条件の走行で 19 回発火していることが、ループそのものが動いている対照になる。areka の欠陥ではない。
- **`surface.append` にしか現れない絵の綴りは焼かれない**（タスク 4.3 のレビューで確かめたとおり、`ManifestDeriver::derive` が `shell.surfaces` だけを読む既存の性質）。検体 2 体の `surface.append` は 0 行なので実機に影響しない。本仕様は焼く一覧の導出を 1 行も変えていない。

**開発者にお願いしたいこと**: 上の「自動走行では確かめられなかった項目」5 件は、走っている窓に対する操作か目視が要るため、この走行では確かめられていない。いずれも本仕様が新しく作った経路ではない（クリック透過・右クリックメニュー・字形の描画・バルーンの左右反転は既存の機能で、本仕様はそこへ絵を届けられるようにしただけである）が、**α 版の受け入れとしては開発者の目で 1 度確かめてほしい**。再現の手順は上の「共通の条件」のとおりで、`AREKA_APP_SMOKE_EXIT_MS` を長めに取って手で触れば足りる。
- 8.2: **要件 4.2 の括弧書きを「赤・緑・青の各成分が等しい」から「B・G・R・A の 4 バイトが等しい」へ是正した**（レビュー裁定・2026-09-20）。3 成分だけで比べると、左上が完全透明（乗算済み `0,0,0,0`）のときに黒寄りの不透明な画素まで一致して消え、要件 4.1（それ以外の画素は 1 バイトも変えない）と 4.8（`tRNS` の透明度を生かす）を同時に壊す。実装（`Normalizer::normalize` の `*px == key`）が正しく、要件の文面の側が誤っていた。`doc/COMPAT_ARCHITECTURE.md` §8 の「抜き色の色を比べる値の空間」の根拠欄も同時に追随させた。

## 最終検証の是正（`/kiro-validate-impl`・2026-09-20）

feature 全体の検証で 3 件の申し送りが出たので、その場で直した（3 件とも本仕様の正しさは変えない）。

1. **語の正規化が 2 か所に書かれていた** — `EmoWorld::dangling_pattern_targets` の `targets_animation_id` と `ComposeMethod::from_name` が「前後の空白を落とし・小文字にし・`-` と `_` を除く」を別々に綴っており、逐語同一ではあったが**片方だけ変えても赤が 1 本も立たなかった**。檻を足すのではなく、正規化そのものを `crates/areka-emo-compose/src/method.rs` の `canonical_method_name` 1 本へ寄せて両者が引く形にした（定義が 1 つなのでずれようが無い）。較正: `canonical_method_name` から `-`／`_` の除去を落とすと `dangling_pattern_targets_skips_methods_that_take_animation_ids` と `from_name_maps_simple_methods`・`from_name_maps_blend_family` の **3 本が同時に赤**になる（戻して 219 件緑を再確認）。
2. **`BaseImageReport::shadowed` の doc が事実より狭かった** — 「層 0 の element が在ったため使わなかった画像」と書いていたが、本来生じない不整合（`SurfaceIndex` が指すのに `SurfaceMaster` が欠ける）で飛ばした画像もここへ入る（不変条件「和は渡した画像の全体」を保つため）。doc を事実へ直した。到達しない枝なので要件 6.2 の `debug!` の文言は据え置く。
3. **`design.md` の「Allowed Dependencies」が「`Cargo.toml` の変更 0 件」と言い切っていた** — 実際はタスク 4.1 で `areka-emo-present` の `[dev-dependencies]` へ `temp-path-kit` を 1 行足している（一時フォルダの自作は共有の見張りが拒むため）。design.md を事実へ直した。

**直さずに残したもの（判断と理由）**:

- **要件 4.10（抜き色で透明になった場所はクリックが背後へ抜ける）には檻も実機確認も無い。** 当たりマスクは合成後の画素の α から作られ（`crates/areka-emo-present/src/presenter/budget.rs` の `MaskRotation::regenerate`）その経路は変更 0 なので構造上は成り立つはずだが、**赤を立てるものが 1 本も無い**。「テストで言い切れている」と報告してはならない。開発者の目と手が要る 5 項目の ⑴ がまさにこれで、`areka-P0-alpha-release-signoff` の実機一周で潰す想定である。
- **本文走査（`shell::parse(` が 0 件）は本番 2 ファイルだけを見張り、examples 3 本を見ていない。** design の Testing Strategy が本番 2 ファイルしか求めていないため。examples に自前の解析が戻っても赤にならない（今日の 0 件は手で確かめた）。examples は手動検証の補助でテストの代替ではない（`.kiro/steering/tech.md`）ので据え置く。
- **`crates/areka/examples/emo-present/setup.rs` の私有 `fn build_shell_target` が、本仕様が新設した公開 `areka_emo_present::build_shell_target` と同名で別物。** 読み手が取り違え得るが、example の中に閉じた私有関数で振る舞いは正しい。改名は次に同ファイルを触るときでよい。
- **`areka-emo-compose` の `konnoyayame` の受け口だけ較正の檻が無い**（他 2 クレートには在る）。受け口が壊れれば `base_image_tests.rs` が落ちて露見する。

## 開発者の目視サインオフ（2026-09-20）

自動走行では確かめられなかった 5 項目を、開発者が実機を触って確かめた記録。

### `R_POST_and_KOMAINU`（要件 8.2 ⑶⑷・要件 4.10）— **問題なし**

走行: `AREKA_APP_SMOKE_EXIT_MS=1800000`（安全網・実際には使われず）・`RUST_LOG=info,areka_emo_present::shell_target=debug,areka::menu=debug,areka::input_events=debug`・約 3 分 28 秒・**exit 0**・`ERROR` 0 行。

**開発者の判定: 問題なし。** 併せてログが独立に裏付けたもの:

- **右クリックメニューが開いた**——`[menu] shown event="menu_shown" scope=0 items=2` が 2 回（23:28:03 と 23:31:03）。メニューの見出しは SHIORI へ問い合わせて解決しており、既定へ落ちたのは `closebutton.caption` の 1 件だけ（`event="menu_resource_empty" ids=["closebutton.caption"]`）。
- **1 項目目が実際に効いた**——`[menu] selected event="menu_selected" scope=0 frame=Readme id=1` → `[readme] opened the readme with the default application`。
- **終了はメニュー経由の正規の経路**——`kanade: reason=Quit——終了系列（Quit）へ event="talk_done_quit" talk_id=3` → `shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)）` → `ghost_quit cause=Quit`。**終了挨拶の talk を経ており、有界の自動終了（強制終了の経路）ではない。**
- 面の一覧は `recognized=10 used=10 shadowed=0`、本体側 236×462・相方側は**宣言の無い面 10** の 140×160。

これで要件 8.2 の ⑶（絵の外のクリックが背後へ抜ける＝**要件 4.10 の唯一の検証手段**）と ⑷（右クリックメニューの 1 項目目）、および ⑴⑵ が開発者の目で確かめられた。

### `konnoyayame`（要件 8.2 ⑵ 目の周りに四角い地色が出ない）— **問題なし**

走行: `AREKA_APP_SMOKE_EXIT_MS=1800000`・`RUST_LOG=info,areka_seriko=debug`・**exit 0**・`ERROR` 0 行。

**開発者の判定: 「多分問題ない。結構早いので目視は厳しいが、変な画像にはなっていないと思う。」**

まばたきは 6 コマ合計 240ms（`animation0.pattern0..5` の持続 0/50/20/70/30/70）で、目視が厳しいのは素材の仕様どおりである。そこで**画素でも裏を取った**。

- **発火の実在**: `seriko: loop 抽選発火（再生開始・先頭コマから・要件 2.1/2.2） scope="0" slot=Shell animation_id=0 k=2` と `seriko: loop 停止（負 surface でベース復帰・要件 4.3）` の対が **74 行＝37 回**。抽選は 1 秒グリッド（`LOTTERY_GRID_MS = 1000`）・`sometimes`→k=2 なので毎秒 1/2 の頻度と一致する。
- **まばたきの素材は本仕様が新設した経路そのもの**: `animation0.pattern*,overlay,1031/1032/1033,…,93,103` が指す `surface1031.png`・`surface1032.png`・`surface1033.png` は **`surfaces.txt` に宣言が 1 行も無い**。ファイル名の慣習だけで面が建っている。
- **四角の縁が浮かないことの画素証跡**: 素材 72×30 を基準絵 `surface0000.png` の (93,103) と全画素突き合わせた実測。

| 領域 | 画素数 | 差の中央値 | 目に見える差（Δ>24） | 最大差 |
|---|---|---|---|---|
| 外周 2px（四角の縁） | 384 | **0** | 17（4.4%） | 90 |
| 内側（目の周り） | 1767 | 5 | 663（37.5%） | 255 |

縁の画素は半数以上が基準絵と**バイト一致**で、目に見える差は 4.4%（睫毛や髪が枠を横切る部分）に留まる。変化は内側＝目もとに集中しており、**矩形の境目が地色で浮く形にはならない**。素材側の抜き色は基準絵と同じ緑 `(0,255,0)`・`tRNS` 無しで、2160 画素中 9 画素だけ（板はほぼ不透明に瞑り絵を描き直して被せる造り）。

**再観測の手順（この項目を次に確かめる人への申し送り）**: `konnoyayame` の `animation0` は面 0 にしか無いので、**起動挨拶が面 0 で終わった走行でしか観測できない**。台本を数えると面 0 で終わる挨拶は `OnBoot` の**「朝」（hour 4〜11）の 3 本中 1 本だけ**で、昼・夕方・夜・深夜の分岐には面 0 終わりが 1 本も無い。撫で反応は面 0 へ戻すが滞在は **2.6 秒**（実測）しかなく、1 秒 1/2 の抽選では 25% の確率で 1 度も出ない。**朝の時間帯に、駐車面が 0 になるまで起動し直す**のが確実な手順である。

**⑶ 起動挨拶の字形（要件 8.2 ⑶）も同日サインオフ済み。** 開発者の判定: 「バルーンで変だったことはない。」本日この検体を 4 度起動し、そのたびに起動挨拶がバルーンへ流れるのを開発者が見ている。文字の経路の証跡（`charset_initial charset="Shift_JIS" source="default"` と `command=Text("初めまして。")` ほか）は 7.2 の表のとおりで、**描かれた字形の側も目視で確かめられた**。
