# ギャップ分析: areka-P0-shell-implicit-surface

> 作成 2026-09-19。入力は確定済みの `requirements.md`（以下「要件」）と本ブランチの実コード。`brief.md` は 2 か所で正典と食い違うため、本書は要件（とくに要件 10 と「正典の引き直し」C1〜C14）を正とした。
> コードは「何の定義か＋ファイル」で指す。行数は 2026-09-19 の実測（`wc -l`）。検体の画像の事実は、`cargo run -p sample-ghost-kit --bin nar-sample-path -- <検体名>` が配った木を Windows の WIC（areka の復号器と同じ部品）で開いて確かめた。
> 本書は選択肢と根拠を並べるもので、決定はしない。

## 1. 分析サマリ

- **面の番号を引く入口は、実行時は 1 か所に集まっている。** 起動時の面 0／10・`\s[N]`・コマの相手・採寸・当たり判定・`surface.append` の対象判定は、すべて `areka-emo-compose` の `EmoWorld`（`SurfaceIndex`／`EmoWorld::surface`）を引く。ここに画像だけの面を載せれば、これらは手を入れずに同じ結果になる。**例外は 1 つ**＝焼く画像の一覧を作る `ManifestDeriver::derive`（`areka-emo-atlas`）で、これは `EmoWorld` ではなく解析直後の `Shell.surfaces` を読む。したがって規則を足す場所は最少で 2 か所（面の表と、焼く一覧）、または両者の手前の 1 か所になる。
- **シェルのフォルダを列挙するコードは 0 本。** 「読む → 解析 → 焼く → 面の表を組む」の並びは、本番 2 か所（`build_boot_assets`・`build_shell_assets`）と `examples` 3 か所の計 5 か所に同じ形で複製されている。バルーンは同じ問題を「権威 1 本（`resolve_balloon_faces`＋`build_balloon_target_from_faces`）を両方が呼ぶ」形で解いており、名前の判定（`face_id_of`）・重複時の採り方（`select_faces`）・列挙（`enumerate_file_names`）の形がそのまま手本になる。
- **抜き色の腕は小さく閉じている。** `Normalizer::normalize` が受け取るのは WIC が 32bit の乗算済み BGRA へ変換し終えたバッファで、α の無い絵は全画素 α=255 になっている。両テンプレートの絵は復号前の形式が `Bgr24`（里々）／`Indexed8`（YAYA）で、どちらも `pixel_format_has_alpha` の表に無い＝`has_alpha=false`＝抜き色が選ばれることを実物で確かめた。左上の色は里々が白（255,255,255）・YAYA が緑（0,255,0）。依存する他のコードは `emo2_e2e.rs` のテストだけである。
- **要件が名指ししていない波及が 3 件見つかった。** ⑴ `konnoyayame` のまばたきは `animation0.interval,sometimes` で、`sometimes` を areka は**駆動しない**（`AnimationTable::from_world` が記録だけ残して採らない）。面の解決と抜き色を直しても、実機でまばたきは起きない。⑵ `emo2` の焼き結果を逐語で留めている `emo2_shell_matches_golden`（と `testdata/emo2_shell_golden.txt`）が、`null.png` の行が 1 行増えるので赤になる。要件 5.6 の書き換え対象の列挙に入っていない。⑶ `null.png` が「全画素が透明な絵」として焼かれると、`bake` が既存の `warn!`「element が全透明（α=0）でトリム後 0 寸です」を新たに出す。要件 5.5 は脱落の `warn!` が 0 回になることしか述べていない。
- **「`element0` が在る」を今のモデルは完全には表せない。** `decode_elements` は `overlay` 以外の `element` 行を値にせず捨てるので、要件 2.7（`overlay` 以外で書かれた `element0` でも画像を使わない）を満たすには、転記層が「`element0` の行が在った」事実を運ぶ必要がある。`Surface` に欄を足すと構造体リテラルが 29 ファイルで壊れる。3 検体にこの形は 0 件。

## 2. 現状調査

### 2.1 面の番号を引く入口と、その権威

| 入口 | 引いているもの（定義＋ファイル） | 面の有無を決めるもの |
|---|---|---|
| 起動時の面 0／10（表示） | `build_boot_assets`（`crates/areka/src/emo2_boot/assets.rs`）が `ScopeAssets.initial_surface_id` に 0／`KERO_INITIAL_SURFACE_ID`=10 を固定で入れ、表示は `apply_show` へ | `EmoWorld` |
| 起動時の面 0／10（採寸） | `measure_native_scope_sizes` → `compose_size` → `Composer::compose`（`crates/areka/src/placement/measure.rs`） | `EmoWorld` |
| `\s[N]` | `SurfaceResolver::resolve`（`crates/areka-seriko/src/resolve.rs`）は数値をそのまま `Show(N)` にし、有無は見ない。`apply_show`（`crates/areka-emo-present/src/presenter/show.rs`）→ `build_plan`（`crates/areka-emo-compose/src/plan.rs`）が `world.surface(id)` で判定し、無ければ `SurfaceNotFound`、層が無ければ `EmptyComposition` | `EmoWorld` |
| コマ／着せ替えの相手（描く） | `flatten_surface` → `push_static_element_ops` → `surface_and_binding`（`plan.rs`）。相手が無ければ記録 0 件で `return` | `EmoWorld` |
| コマ／着せ替えの相手（外形） | `compute_extent` → `flatten_extent`（`plan.rs`）。母集合は全 `element` ＋ **bind 種**のアニメの `pattern0` だけ（`sometimes` などのコマは外形に数えない） | `EmoWorld` |
| コマの表 | `AnimationTable::from_world`（`crates/areka-seriko/src/table.rs`）。相手の面の有無は見ない | `EmoWorld`（`surface_ids`） |
| 当たり判定 | `crates/areka-emo-present/src/presenter/hit.rs` が `emo_world.surface(id)` | `EmoWorld` |
| `surface.append` の対象判定 | `fold_append`（`crates/areka-emo-compose/src/fold.rs`）が `SurfaceIndex` を引き、無ければ既存の `warn!`「surface.append 対象 id が未存在: 新設せずスキップ」 | `EmoWorld`（畳み込みの途中の状態） |
| **焼く画像の一覧** | `ManifestDeriver::derive`（`crates/areka-emo-atlas/src/manifest.rs`）。`SurfaceSet.surfaces`＝解析直後の `Shell.surfaces` を読む | **`EmoWorld` ではない** |

読み取れること:

1. 実行時の入口 8 本はすべて `EmoWorld` を引く。`EmoWorld` は `EmoWorld::build` → `fold_shell`（`fold.rs`）の 1 本道でしか作られない。ここへ画像だけの面を載せれば、要件 3.1〜3.4・3.6（面の側）・3.7〜3.9 は入口ごとの修正 0 件で満たせる。
2. 焼く一覧だけが別の入力を読む。ただし `ManifestDeriver::derive` は `set.surfaces` の**全部**の `element` を無条件に集めており（`collect_elements` を全面に対して呼ぶ）、`resolve_indirect` が辿る相手も同じ集合の中にしか居ない。つまり `resolve_indirect` が相手不在で `continue` しても一覧から漏れる画像は 0 枚で、要件の導入部が挙げた「`resolve_indirect`＝`continue`」は、使う画像を一覧へ直接足せば直す必要が無い。なお `derive` の `by_id` は代表の番号（`Surface.id`）しか索引しない（`surface0,1` のような複数番号の見出しは先頭だけ）が、上の理由で害は出ていない。
3. `fold_plain_surface` は `surface0,1` のような複数番号の見出しを番号ごとに展開する（`expand_targets`・`normalize_surface`）。面ごとの画像は番号ごとに違うので、画像を足す処理は**展開後**（番号ごと）でなければならない。解析直後の `Surface`（本体を番号間で共有する）へ足すと、面 0 の画像が面 1 にも付く。
4. `surface.append` が後から `element0` を足す場合がある（C4「同時にelement0が定義されている場合」）。「`element0` が在るか」の判定は畳み込みが**終わった後**の状態で下すのが素直で、解析直後のモデルで下すには `expand_targets`（`fold.rs` の私有関数）と同じ展開をもう 1 度書くことになる。

### 2.2 シェルのフォルダの列挙と、画像のパスの解決

- シェルのフォルダを `read_dir` するコードは `crates/` に **0 本**。シェルのフォルダは `areka_parsers::package::resolve` の `model.shell.dir`（`build_boot_assets`）と、呼び手から渡される `shell_dir`（`build_shell_assets`）で、どちらも `surfaces.txt` と `descript.txt` を名指しで開くだけである。
- 画像の実パスは `bake`（`crates/areka-emo-atlas/src/lib.rs`）の中の `set.base_dir.join(&key.rel_path)` の 1 か所でしか作られない。一覧に相対パス（ファイル名）を足せば、焼く側は手を入れずに読める。`SurfaceSet` の欄は `surfaces`・`base_dir`・`alpha_params` の 3 つで、`element` に由来しない画像を渡す口は **0 個**。
- 「読む → `decode` → `areka_parsers::shell::parse` → `shell.surfaces.is_empty()` の検査 → `SurfaceSet` → `bake` → `EmoWorld::build` → `bind_atlas`」の並びは次の 5 か所に同じ形で在る: `build_boot_assets`（`assets.rs`）・`build_shell_assets`（`measure.rs`）・`crates/areka/examples/emo-present/setup.rs`・`crates/areka/examples/collision-probe/setup.rs`・`crates/areka/examples/window-placement.rs`。要件 3.6（採寸と表示が同じ対応を使う）は、この複製のままでは規約で守るしかない。
- 手本（バルーン・`crates/areka-emo-present/src/balloon.rs`）: `enumerate_file_names`（`read_dir` 1 回・UTF-8 でない名前は落とす・失敗は `error!`＋`Err`）／`face_id_of`（接頭辞を大小無視で外す → `.png` を外す → 残りが空でなく全部数字 → `parse::<u32>()`。先頭の 0 は `parse` が吸収する。符号付きは数字検査で落ちる）／`select_faces`（`BTreeMap` で決定化・同じ番号に複数あれば辞書順で最小）。要件 1.1〜1.6・1.8 はこの形でそのまま満たせる。`u32` に収まらない数字列は `parse` が `Err` を返すので、要件 1.6 の `debug!` はそこで出せる。
- 置き場の制約: `areka-parsers` は純粋層で、ファイルを読むのは `package` モジュールだけ（steering `structure.md`）。依存の向きは `areka-parsers` ← `areka-emo-atlas` ← `areka-emo-compose` ← `areka-emo-present` ← `areka`（`areka-seriko` は `areka-emo-compose` に依存）。`examples` は `#[path]` で取り込む作りなので、5 か所を 1 本にまとめるなら置き場は `areka` の bin ではなくライブラリのクレートになる。
- 現状の 2 つの硬い失敗: ⑴ `surfaces.txt` が読めなければ `BootWiringError::ShellRead`／`PlacementError::Measure` ⑵ `shell.surfaces.is_empty()` なら `ShellEmpty`。C1 は「画像を用意する**か**、surfaces.txt で合成する」と書いており、`surfaces.txt` が無い・波括弧が 0 個のシェルも正典上は成り立つ。3 検体にこの形は **0 件**で、要件にも記述は **0 件**。

### 2.3 抜き色の腕が受け取るもの

- `WicDecoderArm::decode_inner`（`crates/areka-emo-atlas/src/decode/wic_arm.rs`）は、変換**前**のフレームの形式で `has_alpha` を決め（`pixel_format_has_alpha`・α を持つ形式 12 種の表）、その後 `GUID_WICPixelFormat32bppPBGRA`（乗算済み BGRA）へ変換して `DecodedImage.bgra` に入れる。α の無い絵は全画素 α=255 で埋まる。
- 実物の確認（WIC 経由・2026-09-19）:

  | 絵 | 変換前の形式 | `has_alpha` | 左上の画素（B,G,R,A） | 左上と同じ色の画素 | α≠255 の画素 |
  |---|---|---|---|---|---|
  | 里々 `surface0000.png`（236×462） | `Bgr24` | false | 255,255,255,255（白） | 59,831／109,032 | 0 |
  | 里々 `surface0010.png`（140×160） | `Bgr24` | false | 255,255,255,255 | 11,816／22,400 | 0 |
  | YAYA `surface0000.png`（260×390） | `Indexed8`（α 付きのパレット色 0 個） | false | 0,255,0,255（緑） | 70,574／101,400 | 0 |
  | YAYA `surface0010.png`（200×200） | `Indexed8` | false | 0,255,0,255 | 28,747／40,000 | 0 |
  | YAYA `surface1031.png`（72×30） | `Indexed8` | false | 0,255,0,255 | 9／2,160 | 0 |
  | emo2 `purple/a/null.png`（382×547） | `Indexed8`（パレット 2 色・うち α 付き 1 色） | false | 0,0,0,0 | 208,954／208,954 | 208,954 |
  | emo2 `surface0.png`／`surface10.png`／`CityPop\surface0010.png` | `Bgra32` | true | — | — | — |

- 読み取れること:
  1. 両テンプレートの絵は `has_alpha=false` で、今日は `Normalizer::select_source` が `AlphaSource::KeyColor` を選び、`Normalizer::normalize` が `NormalizeError::Unsupported(KeyColor)` を返し、`bake` がその絵を落とす。要件の「原因 2」は実物で再現どおり。
  2. α の無い絵は α=255 で届くので、乗算済みと非乗算の値が一致する。抜き色は「左上の B・G・R と一致する画素を 0,0,0,0 にし、他はそのまま」で、要件 4.1〜4.3・4.6 を満たし、出力は乗算済みのまま（`NormalizedImage` の契約どおり）。
  3. **パレット＋`tRNS` の絵は、腕に届く時点で `tRNS` の透明度が既に掛かっている**（`null.png` は全画素 0,0,0,0）。完全に透明な画素の元の色は `DecodedImage` からは取り戻せない。要件 4.8 の「`tRNS` を生かすかどうか」のうち、「生かさない（元の色で抜き色を判定し直す）」を選ぶには復号器の側（α の無い形式のときの変換先）を変える必要がある。「届いた 4 バイトのまま比べる」なら腕の中だけで済む。`null.png` はどちらでも全画素が透明になる（要件 4.8 の見立てどおり）。
  4. `UseSelfAlpha::Off` でも `select_source` は `KeyColor` を返し、このとき `has_alpha=true` の絵も抜き色へ来る（テスト `off_no_pna_selects_keycolor_seam` が `has_alpha` の真偽両方を回している）。α を持つ絵は乗算済みで届くので、「自分の α を無視して左上の色で抜く」は `DecodedImage` からは正しく作れない。areka が `Off` を渡す経路は 0 本（要件 Boundary）。
- 全画素が左上と同じ色の絵: `Trimmer.trim` が配置なし（`placement: None`）・原寸ありの項目にする。`push_static_element_ops` は配置なしの層を命令にせず、`flatten_extent` は原寸を外形に数える。要件 4.7 の既存の扱いはコード上そのとおり。ただし `bake` はこのとき `warn!`「bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性）」を出す。
- 当たりのマスク（要件 4.10）は合成後の α から作られるので、腕の出力が α=0 なら追加の仕組みは 0 個で足りる。
- バルーン: `build_balloon_target_from_faces` は焼きの失敗が 1 件でもあればバルーン全体を失敗にする。抜き色が入ると、α の無いバルーンの絵は失敗ではなく抜かれて出る。そのとき `balloon_background::face_origin_color` は左上の画素が透明なので白へ落ちる（既存の縮退経路・`debug!`）。リポジトリ内のバルーンでこの形は 0 枚（要件 Adjacent のとおり）。

### 2.4 書き換える既存テストの実在確認

| 要件 5.6 が名指しするもの | 実在 | 置き場 |
|---|---|---|
| `emo2_shell_all_elements_baked` | あり | `crates/areka-emo-atlas/src/emo2_e2e.rs` |
| 定数 `SHELL_NORMALIZE_SEAM_KEY` とその説明 | あり | 同上 |
| `emo2_balloon_same_bake_path_as_shell` | あり | 同上 |
| `emo2_shell_bake_is_deterministic`（失敗集合の比較） | あり | `crates/areka-emo-atlas/src/emo2_golden.rs` |
| `on_no_alpha_no_pna_selects_keycolor_seam` | あり | `crates/areka-emo-atlas/src/normalize.rs` の `mod tests` |
| `off_no_pna_selects_keycolor_seam` | あり | 同上 |
| `SCOPE0_W`／`SCOPE0_H`／`SCOPE1_W`／`SCOPE1_H`（434／687／336／400） | あり | `crates/areka/src/placement/measure_tests.rs` |
| 注記「α 無し `null.png` が落ちる」5 か所 | あり（5／5） | `assets.rs`・`measure.rs`・`examples` 3 本 |

**要件 5.6 の列挙に無いが、同じ理由で赤になるもの 1 件**: `emo2_shell_matches_golden`（`emo2_golden.rs`）と、その比較相手 `crates/areka-emo-atlas/src/testdata/emo2_shell_golden.txt`。このファイルは索引表の全項目を 1 行ずつ持ち（現在 54 行）、`null.png` の行は **0 行**。適用後は `0<TAB>purple/a/null.png<TAB>EMPTY orig=382x547` が 1 行増える。作り直しの手順は同ファイル冒頭の説明にある（`record_golden` を `--ignored` で回す）。他の 54 行が変わらないことは、作り直した差分が「1 行の追加だけ」であることで確かめられる。

`null.png` を名前で使う他のテスト（`crates/areka-emo-compose/src/golden_tests_blink_static_tests.rs`）は、メモリ上の復号器に不透明な単色を入れており、実物の絵も抜き色も通らない（影響 0 件）。`Unsupported(KeyColor)` に依存するコードは `emo2_e2e.rs` の外に **0 件**。

### 2.5 1 ファイル 1,000 行の目安（触りうるファイル）

| ファイル | 行数 | 余地 |
|---|---|---|
| `crates/areka/src/placement/measure_tests.rs` | 981 | なし（要件 7.11 のとおり） |
| `crates/areka/src/emo2_boot/assets_tests.rs` | 976 | なし（同上） |
| `crates/areka-emo-compose/src/fold_tests.rs` | 968 | **なし**（畳み込みのテストを足すなら新しいファイル。`Surface`／`SurfaceAppend` に欄を足す案では、リテラル 6 か所の追記だけで 974 行前後になる） |
| `crates/areka-emo-compose/src/plan_ops_tests.rs` | 1,374 | 既に超過（例外表に載っている。例外表は名前だけを持ち行数は持たないので、追記しても検査は変わらないが、足さないのが目安の趣旨） |
| `crates/areka-emo-present/src/cache_tests.rs` | 1,301 | 同上（`Surface` のリテラル 2 か所） |
| `crates/areka-emo-compose/src/composer_tests.rs` | 895 | 小 |
| `crates/areka-emo-compose/src/plan.rs` | 730 | あり |
| `crates/areka-emo-present/src/balloon.rs` | 644 | あり（シェルの権威を同居させると 1,000 行に近づく。別ファイルが無難） |
| `crates/areka-emo-atlas/src/lib.rs` | 596 | あり |
| `crates/areka-parsers/src/shell/decode.rs` | 565 | あり |
| `crates/areka-seriko/src/table.rs` | 529 | あり |
| `crates/areka/src/placement/measure.rs` | 485 | あり |
| `crates/areka/src/emo2_boot/assets.rs` | 436 | あり |
| `crates/areka-emo-atlas/src/manifest.rs` | 352 | あり |
| `crates/areka-emo-atlas/src/emo2_e2e.rs` | 351 | あり |
| `crates/areka-emo-atlas/src/normalize.rs` | 309 | あり（テストは本体と同居の古い形。新しいテストは兄弟ファイル `normalize_<主題>_tests.rs` へ置くのが現行の規約） |
| `crates/areka-emo-compose/src/fold.rs` | 272 | あり |
| `crates/areka-emo-compose/src/world.rs` | 256 | あり |

この変更で 1,000 行を新たに超えるファイルは、新しいテストを新しいファイルへ置く限り **0 本**。本仕様が触りうるクレート 6 つ（`areka-parsers`・`areka-emo-atlas`・`areka-emo-compose`・`areka-emo-present`・`areka-seriko`・`areka`）は、どれも既に `sample-ghost-kit` を `Cargo.toml` に持っており、要件 7.10 のための依存の追加は 0 件。`konnoyayame`／`R_POST_and_KOMAINU` を `SampleRoot::acquire` で引いているテストは今日 **0 本**（`emo2` だけ）。

## 3. 要件と既存資産の対応

凡例: **無**＝該当する実装が無い／**不明**＝設計で調べる／**制約**＝既存の作りが選択肢を狭める。

| 要件 | 既存の資産 | 差分 |
|---|---|---|
| 1.1〜1.6・1.8 名前の判定 | バルーンの `face_id_of`・`select_faces`（同じ形） | **無**（シェル用は 0 本）。形は移せる |
| 1.7 一覧が取れないとき | `enumerate_file_names` の失敗経路が手本。受け側は `BootWiringError`（`ShellRead` は `path`＋`io::Error` を持つ）／`PlacementError::Measure` | **無**。失敗の型に新しい枝を足すか既存の枝を使うかは設計 |
| 2.1 ア・イ（画像を土台に） | `NormalizedElement`（`layer`・`path`・`transform`・`method`）と層の昇順ソート。`element0` が無いときだけ画像を使うので、**層 0 は必ず空いている** | **無**。「`element0` より下」を表す新しい層の型は要らない（brief の難所は要件の読みで消えている） |
| 2.1 ウ・2.7（`element0` が在れば使わない） | `overlay` の `element0` は `Surface.elements` に載る | **制約**: `overlay` 以外の `element0` は `decode_elements` が捨てるので、在ったことが下流から見えない。転記層の変更が要る（3 検体に 0 件） |
| 2.2 画像だけの面 | `upsert_surface`（`fold.rs`） | **無**。**制約**: 既にある番号へ `fold_plain_surface` が来ると「surface id 重複: 既存定義を全置換する」の `warn!` が出る。画像由来の仮の面を先に置く作りでは、これが偽の警告にならない工夫が要る |
| 2.3 波括弧の他の定義を生かす | `normalize_surface` が当たり判定・アニメをそのまま運ぶ | なし（絵を足すだけ） |
| 2.4・3.3 どちらも無い番号 | `build_plan` の `SurfaceNotFound`／`EmptyComposition`・`apply_show` の `error!` | なし（変更 0） |
| 3.1〜3.4・3.9 | 2.1 節のとおり全入口が `EmoWorld` | 面の表に載れば修正 0 件 |
| 3.4 の後半・7.5・8.3 ⑵（まばたき） | `AnimationTable::from_world` は `Random`／`BindRandom` だけを採る | **制約（範囲外の欠落）**: `sometimes` は非駆動（4 節の論点 1） |
| 3.5 相手の無いコマの `warn!` を 1 回 | 今は `push_static_element_ops` が記録 0 件で `return`。`atlas_bind.rs` の `bind_atlas` は未解決の `element` に `warn!` | **無**。**制約**: `EmoWorld::build` は 1 回の起動で 3 回走る（表示 2 スコープ＋採寸 1 回）ので、組むたびに検査すると同じ組で 3 回出る |
| 3.6 採寸と表示が同じ | バルーンは権威 1 本。シェルは 5 か所に複製 | **無**（構造で守る仕組みが無い） |
| 3.7・3.8 `surface.append` | `fold_append` の存在条件と既存の `warn!` | 画像だけの面が畳み込みの**前**に面の表に居れば、`fold_append` の修正 0 件で満たせる |
| 4.1〜4.3・4.6・4.7 抜き色 | `Normalizer::normalize` の継ぎ目・`Trimmer`・`flatten_extent` | **無**（腕の中身）。入力は α=255 なので単純な比較で足りる |
| 4.4 α 付きは不変 | `(UseSelfAlpha::On, AlphaChannel)` の素通し | なし（変更 0） |
| 4.8 `tRNS` | 復号器が `tRNS` を掛け終えて渡す | **不明／制約**（2.3 節の 3） |
| 4.9 `.pna`・`full` は据え置き | `Unsupported(Pna)`・`Unsupported(Opaque)` | なし（変更 0） |
| 4.10 当たり | `MaskRotation::regenerate` → `AlphaMask::from_pbgra32` | なし（追加 0） |
| 5.1〜5.4 `emo2` 不変 | `measure_tests.rs` の定数・合成結果のテスト群 | なし。ウの判定が正しければ緑のまま |
| 5.5・5.6 | 2.4 節 | 書き換え。**列挙漏れ 1 件**（`emo2_shell_matches_golden`）。**新しい `warn!` 1 種**（全透明の警告） |
| 5.8 面 10 の形で赤を示す | `emo2` の面 10（`element0`＝336×400・画像 427×463） | **無**（新しいテスト）。外形が 336×400 → 427×463 へ動くので `SCOPE1_W`／`SCOPE1_H` でも赤になる |
| 6.1〜6.3 記録 | steering `logging.md` | **無** |
| 7.x | `sample-ghost-kit`・メモリ上の復号器 `MemoryDecoder`（`crates/areka-emo-atlas/src/decode.rs`） | **無**（新しいファイルへ） |
| 8.x 実機 | 定石は既存 | 論点 1 の影響を受ける |
| 9.x 文書 | 台帳・`COMPAT_ARCHITECTURE.md` §8・`roadmap.md` | 着地時の作業 |

## 4. 実装アプローチの選択肢

どの案でも共通: ⑴ 名前の判定は文字列の列を入力にした純粋な関数（バルーンと同型）⑵ 抜き色は `Normalizer::normalize` の腕 1 本 ⑶ 新しいテストは新しいファイル。

### 案 A: 解析直後のモデル（`Shell`）を書き換えてから、焼く側と面の表の両方へ渡す

「番号 → ファイル名」の対応から、画像だけの番号には `element0,overlay,<ファイル名>,0,0` を 1 行持つ `Surface` を作り、波括弧が在って `element0` が無い番号にはその 1 行を足す。バルーンの `synthetic_surfaces_txt` と同じ発想。

- 良い点: 下流（`ManifestDeriver`・`fold_shell`・`plan.rs`・`areka-seriko`）の修正が 0 件。焼く一覧にも自動で載る。
- 悪い点: ⑴ 複数番号の見出し（`surface0,1`）は本体を共有するので、番号ごとに `Surface` を割る処理が要る ⑵ `surface.append` が後から `element0` を足す場合を正しく扱うには、`fold.rs` の `expand_targets` と存在条件をモデルの側でもう 1 度書くことになる（同じ規則の二重実装）⑶ 画像だけの `Surface` を定義の並び（`Shell.definitions`）のどこへ挿すかで `surface.append` の効き方が変わる（先頭へ挿す必要がある）⑷ 転記層の「書いてあるとおり」という性質が、下流へ渡る時点で崩れる。

### 案 B: 面の表（`EmoWorld`）の構築に「番号 → ファイル名」を渡す

`EmoWorld` の構築で、畳み込みの前に画像だけの番号を「在る面」として置き、畳み込みの後に「画像が在り、層 0 が空で、`element0` の行も無かった」面へ画像を層 0 として足す。焼く一覧には、面の表が「土台に使った画像」を答え、それを `SurfaceSet`（新しい欄）経由で `ManifestDeriver::derive` が足す。

- 良い点: 判定が番号ごと・畳み込み後なので、複数番号の見出しも `surface.append` の `element0` も 1 つの規則で正しく扱える。`fold_append` の修正 0 件で要件 3.7・3.8 が成り立つ。土台に使った／使わなかったの数（要件 6.1・6.2）をその場で数えられる。
- 悪い点: ⑴ 今の並びは「焼く → 面の表を組む」なので、「面の表を組む → 使う画像を聞く → 焼く → `bind_atlas`」へ入れ替える必要がある（`EmoWorld::build` は焼いた結果を必要としないので入れ替えは可能）。5 か所の複製すべてに及ぶ ⑵ `SurfaceSet` に欄を足すと、`SurfaceSet { … }` のリテラルを書いている全箇所に追記が要る ⑶ 仮に置いた面へ `fold_plain_surface` が来たときの「重複」の `warn!` を抑える区別が要る。
- 変種 B′（認めた画像を全部焼く）: 並びの入れ替えが要らず単純だが、`emo2` で `surface10.png`（使わない画像）も焼かれ、索引表が 2 枚増える。要件 5.5 の「変わるものは 1 件だけ・索引表は 1 枚増える」と両立しない。

### 案 C: 権威を 1 本にまとめ、その中で案 B を使う

バルーンの `resolve_balloon_faces`＋`build_balloon_target_from_faces` と対になる形で、「シェルのフォルダ → 列挙 → 解析 → 面の表 → 使う画像 → 焼く」をライブラリ側の 1 本にし、`build_boot_assets`・`build_shell_assets`・`examples` 3 本がそれを呼ぶ。スコープごとに面の表が要る `build_boot_assets` のために、「焼いた結果＋面の表を必要な数だけ組める値」を返す形になる。

- 良い点: 要件 3.6 が構造で守られる（`measure_balloon_surface0` の説明が警告している「列挙の規則が 2 つの実装に分かれる」事態が起きえない）。並びの入れ替え（案 B の悪い点 ⑴）が 1 か所で済む。要件 5.7 の注記 5 か所も 1 か所へ寄る。要件 3.5 の「同じ組で 1 回」を、権威が最初に組んだ面の表に対して 1 度だけ検査する形で素直に満たせる。
- 悪い点: 触るファイルが増える（`examples` 3 本を含む）。置き場を決める必要がある（`areka-emo-present` に `balloon.rs` の兄弟として置くのが依存の向きに合う。`areka-emo-compose` では `examples` から見えるが「焼く」と「組む」の両方を知る層としては `areka-emo-present` の前例に揃わない）。`build_boot_assets` は同じ読取結果から `descript.txt` も読むので、切り出す境界の設計が要る。
- 段階を踏むなら: 第 1 段で名前の判定・抜き色・面の表（案 B の核）を純粋な単位として入れ、第 2 段で権威 1 本へ寄せて 5 か所を差し替える。第 1 段だけでは要件 3.6 を構造で守れないので、途中で止めない前提になる。

### 「`element0` の行が在った」を運ぶ方法（案 B・C に共通）

| 方法 | 影響 |
|---|---|
| `Surface`／`SurfaceAppend` に欄を足す | `Surface { … }` のリテラルが 29 ファイル（約 56 か所）、`SurfaceAppend { … }` が 8 ファイルで壊れる。ほぼ全部テスト。`fold_tests.rs`（968 行）は追記だけで上限に迫る |
| `Element` に描画メソッドの欄を足し、`overlay` 以外も載せる | `Element { … }` のリテラルが 24 ファイルで壊れる。下流すべてに「`overlay` 以外は描かない」の関門が要る（網羅台帳の縮退＝担当 `areka-P0-shell-parse` の領分に踏み込む） |
| `Shell` に「値にしなかった `element` 行」の控えを足す（番号の記述子＋層の番号） | `Shell { … }` のリテラルが 25 ファイルで壊れる |
| 転記層は変えず、要件 2.7 を「`overlay` の `element0` だけを見る」に狭める | コードの影響 0。ただし要件 2.7 の改訂が要る（要件は確定済みなので、採るなら要件ディスカッションで） |

`SurfaceMaster`（リテラル 3 ファイル）／`NormalizedElement`（2 ファイル）は狭いので、下流側の型に欄を足す分には影響が小さい。

## 5. 規模とリスク

- **規模: M（上の端）〜L の下の端。** 名前の判定と抜き色はそれぞれ小さい。重さは、5 か所の複製の扱い（案 C）、転記層が「`element0` の行が在った」を運ぶ変更の広がり、検体 2 体を使う新しいテスト群、実機 3 体にある。要件 10.1 の見立て（M の上の端）と合う。論点 1 を範囲に入れると `areka-seriko` が加わる。
- **リスク: 中。** 技術は既知で、正典の読みも要件で確定している。中とする理由は ⑴ `emo2` の不変が合否そのもので、`EmoWorld` の構築・焼く一覧という中核 2 つに手が入る ⑵ 並びの入れ替えが 5 か所に及ぶ ⑶ 構造体リテラルの広い追随で見落としが出やすい、の 3 点。抜き色単体のリスクは低（入力が α=255 で、依存するテストが 1 ファイル）。

## 6. 設計へ持ち越す調査項目

1. `tRNS` 付きパレットの扱い（要件 4.8）。腕に届く時点で `tRNS` が掛かっているという制約（2.3 節の 3）の下で、どちらに定めるか。「生かさない」を選ぶ場合に復号器をどう変えるか、α 付きの 57 枚に影響が及ばないことをどう示すか。
2. WIC が `Indexed8` 以外（1／2／4bit のパレット・16bit のグレーなど）の α なし PNG を 32bit へ変換したときも α=255 で届くか。表 `pixel_format_has_alpha` に無い形式で α を持つものが無いか（グレー＋α の PNG を WIC がどの形式で報告するか）。
3. 権威の置き場と返す値の形（案 C）。`EmoWorld` は複製できないので、スコープの数だけ組む必要がある。
4. 仮に置いた面と「surface id 重複」の `warn!` の区別のしかた。波括弧が `surface.append` より**後ろ**に在り、画像も在る番号の扱い（画像で「既にある」ので `surface.append` は効くが、後から来る波括弧が既存の規則どおり全置換すると追記が消える。正典は沈黙）。
5. 要件 3.5 の `warn!` を出す場所と、1 回の起動で同じ組が 1 回になる仕組み。検査の母集合（全アニメの全コマか、駆動されるものだけか）。
6. 要件 1.7 の失敗を、`BootWiringError`／`PlacementError` の既存の枝で運ぶか新しい枝を足すか。
7. `UseSelfAlpha::Off` で抜き色が選ばれたとき（とくに `has_alpha=true`）の扱い。要件 5.6 は `off_no_pna_selects_keycolor_seam` も「抜かれた結果を確かめる形」へ書き換えるとしている。
8. 記録の target 名と構造化フィールド（steering `logging.md`）。`areka-emo-atlas`・`areka-emo-compose` は `target:` を明示する流儀、`areka` と `balloon.rs` は既定の target。
9. `emo2` の合成結果が 1 画素も変わらないことの示し方（既存の合成結果のテストが面 0・面 10・面 1000 をどこまで覆っているかの棚卸し）。

## 7. 要件ディスカッションへ上げる論点

1. **`konnoyayame` のまばたきは、本仕様の範囲だけでは実機で起きない。** 面 0 の `animation0.interval,sometimes` を areka は駆動しない。`decode.rs` の interval の正規化は `bind`・`random`・`bind+random` 以外を `Interval::Other` にし、`AnimationTable::from_world` は `Other` を `debug!` だけ残して採らない（網羅台帳 `ukadoc:descript_shell_surfaces:sometimes:1` は「語彙のみ」・担当は空欄）。正典は `sometimes` を「そのサーフェスである間毎秒2分の1の確率で再生。」（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#sometimes:1 ）、`random,数値` を「そのサーフェスである間毎秒数値分の1の確率で再生。」（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#random_2c_6570_5024:1 ）と定めており、字面の上では `sometimes` は `random,2` と同じ頻度である。影響を受ける要件: 3.4 の後半・7.5・8.3 ⑵。選択肢: ⑴ `sometimes` の駆動を本仕様へ入れる（`areka-seriko` が加わる。`rarely`・`always`・`runonce` など同類の語をどこまで入れるかも決める）⑵ 範囲外のままにし、要件 7.5 は「手で与えたコマの状態に対して、面 1031〜1033 の画像を位置 93,103 に描く命令になる」（合成の段で確かめられる）に留め、要件 8.3 ⑵ を外すか「別件として起票」に改める。
2. **要件 5.6 の書き換え対象に `emo2_shell_matches_golden` と `testdata/emo2_shell_golden.txt` を加えるか。** 加えなければ、要件どおりに実装した時点でこのテストが赤になる（2.4 節）。
3. **`emo2` で新しく出る `warn!` をどう扱うか。** `null.png` が焼かれると、`bake` の既存の `warn!`「element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性）」が、表示と採寸の各 1 回ずつ出る。要件 5.5・8.5 は脱落の `warn!` が 0 回になることだけを合否にしている。選択肢: そのまま受け入れて要件に明記する／抜き色で全透明になった絵はこの警告の対象から外す（C14 は単色の `surface10.png` を正しい作り方として勧めているので、制作者のミスとは限らない）。
4. **`overlay` 以外で書かれた `element0`（要件 2.7）のために転記層をどこまで変えるか。** 3 検体に 0 件で、満たすには 25〜29 ファイルのリテラルの追随が要る（4 節の表）。要件 2.7 を保つか、「`overlay` の `element0` だけを見る」へ狭めて残りを `areka-P0-shell-parse` の縮退に委ねるか。
5. **5 か所の複製を 1 本へ寄せること（案 C）を本仕様の範囲に含めるか。** 要件 3.6 を構造で守るにはこれが要る。含めないなら、要件 3.6 はテスト（要件 7.6）だけで守ることになる。
6. **`surfaces.txt` が無い／波括弧が 0 個で、画像だけが在るシェル。** C1 は成り立つ形として書いているが、今は `ShellRead`／`ShellEmpty` で起動に失敗する。3 検体に 0 件・要件に記述 0 件。範囲外と明記するか、受けるか。
7. **`tRNS` の扱い（要件 4.8）の選択肢には技術上の非対称がある。** 「届いたまま比べる」は腕の中で済み、「`tRNS` を無視する」は復号器の変更を伴う（2.3 節の 3）。3 検体での結果はどちらでも同じ。
8. **`UseSelfAlpha::Off` の抜き色をどこまで実装するか。** 要件 10.2 は「抜き色の腕だけ」、要件 5.6 は `off_no_pna_selects_keycolor_seam` の書き換えを求める。`Off` かつ α 付きの絵は正しく抜けない（2.3 節の 4）。`Off` を渡す経路は 0 本なので、`On` のときだけ実装して `Off` は未実装のまま残し、当該テストは書き換えない、という選択肢もある。
9. **要件 3.5 の `warn!` の「1 回」の単位。** 面の表は 1 回の起動で 3 回組まれる。「1 回の起動で 1 回」か「面の表 1 つにつき 1 回」かで、置き場（権威 1 本か、面の表の構築か）が変わる。

## 8. 要件ディスカッションでの振り分け（2026-09-19）

7 節の論点 9 件を次のとおり振り分けた。

| 論点 | 振り分け | 結果 |
|---|---|---|
| 2（`emo2_shell_matches_golden` の漏れ） | 自明な修正 | 要件 5.6 に追記済み。期待値の置き場は `crates/areka-emo-atlas/src/testdata/emo2_shell_golden.txt`（`src/` の下） |
| 3（`emo2` で新しく出る「全透明」の `warn!`） | 自明な修正 | 受け入れる。特例は足さない。要件 5.5・8.5 に明記済み |
| 9（要件 3.5 の「1 回」の単位） | 自明な修正＋設計 | 要件 3.5 は「毎フレーム出さない・面の表を組む回数以下」までを定め、単位は設計で定める |
| 5（複製 5 か所を 1 本へ寄せるか＝案 C） | 設計で決める | 要件 3.6 は結果（採寸と表示の一致）だけを定めている。構造で守るかテストで守るかは設計の判断 |
| 7（`tRNS` の 2 案の費用の非対称） | 設計で決める | 要件 4.8 のまま。設計は 2.3 節の制約を踏まえて定める |
| 8（`UseSelfAlpha::Off` の下の抜き色） | 設計で決める | 要件 5.6 を「設計の定めに合わせて書き換えるか据え置く」へ改めた |
| 1（`sometimes` の駆動） | 開発者と議論 | 下の 9 節 |
| 4（`overlay` 以外の `element0`＝要件 2.7） | 開発者と議論 | 下の 9 節 |
| 6（`surfaces.txt` が無いシェル） | 開発者と議論 | 下の 9 節 |

## 9. 開発者裁定（2026-09-19）と 3 議題の決着

開発者の逐語:「この際、テンプレートゴーストを動かすために必要な実装を、本specで巻き込んで対応する方向でスコープ拡大してもらえるか？その前提で他の議題も調整せよ。」

物差しは「検体 2 体のどちらかが、起動から終了までの道筋で実際に使っているか」（要件 10.7）。

| 論点 | 検体での使用 | 決着 | 反映先 |
|---|---|---|---|
| 1 `sometimes` の再生 | `konnoyayame` の面 0 のまばたきが使う（1 件） | **範囲内**。`rarely` は検体に 0 件だが同じ読み替えの 1 語ぶんなので併せて入れる。他の語は範囲外 | 要件 11・7.10・8.3 ⑵・9.1 |
| 4 `overlay` 以外の `element0` | 0 件 | **範囲外**。解析の結果の型は変えない。既知のずれとして記す | 要件 2.1 の定義・2.7・9.3 ⑴・9.5 ⑹ |
| 6 `surfaces.txt` が無いシェル | 0 件 | **範囲外**。今の起動の失敗を変えない | Boundary Context・要件 9.5 ⑸ |

併せて、実機確認で見つかった妨げは別件へ送らず本仕様の中で直す、に改めた（要件 8.4）。

**ほかの妨げの事前確認**（両検体を `RUST_LOG=debug` で 25 秒ずつ実走）: 台本の文字化け 0 件・未対応の台本の命令 0 件・`ERROR` は面 0 の合成失敗に由来する 3 行だけ。`emo2` の `interval,sometimes`／`rarely` は 0 件（`interval` の行は 37 件＝`bind` 30・`bind+random` 3・`random` 4。同じ数え方で `konnoyayame` は 1 件）なので、要件 11 は `emo2` の再生されるアニメーションの集合を変えない。

設計への追加の持ち越し: 読み替えを行う段（`areka-parsers` の間隔の語の読み取りか、`areka-seriko` の `AnimationTable::from_world` か）。どちらでも元の語が記録から読み取れること（要件 11.7）。
