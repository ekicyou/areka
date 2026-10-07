# ギャップ分析: areka-P0-self-alpha-declaration

- 作成: 2026-10-05（`/kiro-validate-gap`）
- 対象: `requirements.md`（要件 1〜8）と、今のコードとの差
- 調べ方: ソースを読んだだけ（ビルド・実行はしていない）。検体は `vendors/sample_ghost/*.nar` の中の `descript.txt` を取り出して宣言の有無だけ確かめた
- 本書は選択肢と材料を並べるもので、決定ではない

## 1. まとめ

- **型と動作表は在るが、描けるのは `1` の 2 通りだけ**。`areka-emo-atlas` の `normalize.rs` の `Normalizer::select_source` は 3 通りの宣言の表を既に持つが、`Normalizer::normalize` が絵を返すのは `(On, AlphaChannel)` と `(On, KeyColor)` だけで、残りは `NormalizeError::Unsupported` を返す。
- **宣言を読む経路は 0 本**。本番で値を渡すのは 2 か所（`areka-emo-present` の `shell_target::build_shell_target_with_boxes` と `balloon::build_balloon_target_from_faces`）で、どちらも `UseSelfAlpha::On` を決め打ちしている。brief が挙げた残り 3 か所（`atlas_bind.rs`・`manifest.rs`・`lib.rs`）は、どれもテスト用のモジュールの中だった（本番の決め打ちではない）。
- **いちばん難しいのは「`0` × α を持つ絵」**。絵は読み手の段で乗算済みになって届くので、完全に透明な画素の色は既に失われている。要件 5 の 2・3（絵に書かれた色のまま不透明にしてから抜く）を文字どおり満たすには、読み手から「乗算していない色」を受け取る口が要る。
- **画像だけのシェルは入口 1 か所の変更で届く見込み**。`shell_target::load_shell_target` が `surfaces.txt` の読取失敗と面 0 個を失敗にしているだけで、その先の面の表（`EmoWorld::build_with_images` → `base_image::apply_base_images`）は、宣言の無い番号を画像から新設する枝を既に持つ。
- **要件と今の実装がぶつかる点が 2 つ**ある。⑴ 要件 1 の 5「シェルの descript.txt が無いとき」は、今は別の場所で起動の失敗になる。⑵ 要件 2 の 7（面ごとの設定ファイルで上書きしない）は、バルーンの今の読み手（2 層を重ねる）にそのまま足すと破れる。どちらも設計で扱いを決める必要がある。

## 2. 今の姿

### 2.1 透過の正規化（`crates/areka-emo-atlas/src/normalize.rs`）

| もの | 今の姿 |
|---|---|
| `UseSelfAlpha`（`On`・`Full`・`Off`） | 3 通りとも在る。`Clone, Copy, Debug` だけ（`PartialEq`・`Default` は無い） |
| `AlphaParams` | `use_self_alpha` 1 欄。`SurfaceSet`（`manifest.rs`）が出どころ単位で運ぶ |
| `Normalizer::select_source` | 3 通りの表を実装済み。`Full` × α 無しは `.pna` の有無に依らず `Opaque`、`Off` は α の有無に依らず「`.pna` 在り → `Pna`／無し → `KeyColor`」 |
| `Normalizer::normalize` | 絵を返すのは `(On, AlphaChannel)`（そのまま渡す）と `(On, KeyColor)`（左上と 4 バイトとも同じ画素を `0,0,0,0` に）だけ。ほかは `Unsupported(選ばれた腕)` |
| `Normalizer::key_color` | `(On, KeyColor)` のときだけ左上の 4 バイトを返す。`bake` がこれで抜いた色を `debug!` に出す |
| `clear_key_color` | 4 バイト完全一致で消す。動く絵の 2 枚目以降（`animated.rs` の `PendingFrames::new`）と共用 |

- 選ぶ表（`select_source`）は要件 3〜5 と既に合っている。`Off` × `.pna` 在り → `Pna` → `Unsupported` は、要件 5 の 7（表示せず記録）の形に今のままでなる。`Full` × α 無し × `.pna` 在り → `Opaque` も要件 4 の 3 と合う。
- 足りないのは `normalize` の 3 つの腕: `(Full, AlphaChannel)`・`(Full, Opaque)`・`(Off, KeyColor)`。

### 2.2 焼く流れ（`crates/areka-emo-atlas/src/lib.rs` の `bake_with_limits`）

- 絵ごとに「読む（`animated::load`）→ `.pna` の有無（`probe_pna`）→ `key_color` → `normalize` → 切り詰め」。`normalize` が失敗した絵は `BakeError::Normalize` に積んで飛ばす。
- 動く絵は 0 番のコマだけが `normalize` を通り、残りのコマは `PendingFrames::new` が「0 番の抜き色（`Option<[u8; 4]>`）を消すだけ」で通す。**`Off` × α を持つ動く絵では、残りのコマにも「不透明にしてから抜く」が要る**ので、ここは抜き色 1 つを渡す今の形では足りない。`Full` × α 無しは、読み手が α 無しの絵を全画素 α=255 で返すなら何もしなくてよい。

### 2.3 読み手（`crates/areka-emo-atlas/src/decode.rs`・`decode/wic_arm.rs`・`decode/image_arm.rs`）

- `ElementDecoder` の口は `decode`・`probe_pna`・`probe_animation`・`decode_frames`・`decode_first_frame`。どれも透過の宣言を受け取らない。
- 静止画は `WicDecoderArm::decode_inner` が WIC で `32bppPBGRA`（乗算済み）へ変換して返す。`has_alpha` は変換前の画素形式から決める（`pixel_format_has_alpha` の一覧に載る形式だけ `true`）。
- 動く絵は `image_arm::to_bgra` が、乗算していない RGBA を受けて自前で乗算する（`premultiply`。コメントは「WIC と全 65,536 通りで一致する式」と書いている）。
- つまり **`normalize` に届く時点で色は乗算済み**。α=0 の画素は `0,0,0,0`、半透明の画素は色の精度が落ちている。

### 2.4 シェルの読み込み（`crates/areka-emo-present/src/shell_target.rs`）

- `load_shell_target(shell_dir, decoder)` が fs を触る唯一の入口。順は「一覧 → `surfaces.txt` 読取 → 解析 → 面の表 → 焼く」。記録はすべてここが出す（モジュール冒頭「記録を出す場所」）。
- `surfaces.txt` が読めないと理由を問わず `ShellLoadError::Read`、`shell.surfaces` が空だと `ShellLoadError::Empty`（面の画像が在っても）。
- 呼び手は本番 2 か所: 起動・切り替えの `areka` の `emo2_boot::assets::build_shell_assets`、採寸の `placement::measure::build_shell_assets`。ほかは examples 3 本。
- `ShellLoadError` は `emo2_boot/mod.rs` で `BootWiringError::ShellRead`／`ShellEmpty` へ写される。どちらの文言も「surfaces.txt が…」と書いている。
- 透過の値は `build_shell_target_with_boxes` の中の `SurfaceSet { alpha_params: On }` の 1 か所。

### 2.5 バルーンの読み込み（`crates/areka-emo-present/src/balloon.rs`）

- 絵を焼くのは `build_balloon_target_from_faces(balloon_dir, decoder, faces)`。**スコープごとに 1 回**呼ばれる（起動の `emo2_boot::assets::build_balloon_assets` のループと、採寸の `placement::measure::measure_balloon_surface0`）。焼く段で 1 枚でも落ちるとバルーン全体の構築を失敗にする。
- 設定は `load_scope_balloon_model` が「`descript.txt`（基層）＋面ごとの上書きファイル（`{接頭辞}{ID}s.txt`）」の 2 層を `areka_parsers::balloon::parse_str` に渡して重ねる（上書きが勝つ）。`BalloonModel` に透過の欄は無い。
- `descript.txt` の読取失敗は `warn!` を出して空として続ける（`read_descript_layer`）。

### 2.6 シェルの descript.txt を読んでいる場所

| 場所 | 読取失敗のとき |
|---|---|
| `areka_parsers::package::resolve_shell`（着せ替えの既定） | 続行（コメント「shell descript は存在確定していない」） |
| `areka` の `placement::source::load_descript_source_for_shell`（`shell_kv`） | **失敗**（`PlacementError::DescriptRead`） |
| `areka` の `emo2_boot::assets::build_shell_assets`（着せ替えの既定） | **失敗**（`BootWiringError::ShellRead`） |

- 近い前例は「作者基準 DPI」: `placement::source` の `DescriptSource::shell_author_dpi`／`load_balloon_author_dpi` が生の KV から読み、`parse_author_dpi` が「宣言なし → `debug!`＋既定／読めない値 → `warn!`＋既定」で縮退し、値を引数で下流へ渡している。

### 2.7 当たり判定

- 表示バッファと対で持つ `AlphaMask`（`areka-emo-present` の `cache.rs` の `CacheEntry::mask`・作るのは `AlphaMask::from_pbgra32`）が真実源。合成後の α から作るので、**要件 4 の 5（全面不透明はどこでも受ける）と要件 5 の 8（抜いた所は通す）は、焼いた画素が正しければ追加の実装なしで成り立つ見込み**（マスクの作り方の中身までは読んでいない）。

### 2.8 文書

| 文書 | 今の記述 |
|---|---|
| `dist/README.txt` の「◆ 既知の制限: 半透明を前提に作られたバルーンだけが正しく表示されます」 | 節が在る。`.pna` の段落も同じ節の中 |
| `doc/ukadoc-coverage/ledger/assets.toml` の `ukadoc:descript_balloon:use_self_alpha_2c_5024:1` | 「常に `use_self_alpha,1` 相当」「縮退」 |
| 同 `ukadoc:descript_shell:seriko.use_self_alpha_2c_5024:1` | 「宣言は今も読まず常に `1` 相当」 |
| `doc/COMPAT_ARCHITECTURE.md` | 該当の行が 2 つ（長い行） |
| `.kiro/steering/tech.md` | 「areka は宣言に依らず常に `use_self_alpha,1` 相当」 |

### 2.9 検体の宣言（`.nar` の中の `descript.txt` を取り出して確かめた）

| 検体 | シェル | バルーン |
|---|---|---|
| `emo2.nar` | `seriko.use_self_alpha,1` | `emo2-kakukaku`: `use_self_alpha,1` |
| `claudia.nar` | `seriko.use_self_alpha,1` | `claudia`・`claudia_vertical`: `use_self_alpha,1`（`use_input_alpha,1` も） |
| `StayseeBalloon.nar` | − | `use_self_alpha,1`（`use_input_alpha,1` も） |
| `emo2-kakukaku-offsetdpi.nar`・`-wplimit.nar` | − | `use_self_alpha,1` |
| `R_POST_and_KOMAINU.nar`・`konnoyayame.nar` | 宣言なし | − |

- `.pna` はどの検体にも 0 件。全検体が `surfaces.txt` を持つ（**画像だけのシェルの検体は手持ちに無い**）。
- 要件 8 の前提「宣言の無い 2 体の PNG は α を持たない」は、本分析では確かめ直していない（要件の申し送り 3 の調べをそのまま信じている）。

## 3. 要件ごとの差

| 要件 | 使える既存のもの | 差 | 種別 |
|---|---|---|---|
| 1（シェルの宣言を読む） | 生の KV の読み手 `areka_parsers::kv::parse_kv`／DPI の前例 | 値を読む関数・運ぶ経路が無い。5 の「descript.txt が無い」は今は別の場所で失敗（2.6） | Missing／Constraint |
| 1 の 7（切り替え） | `emo2_boot::switch_assets` は `build_shell_assets`／`build_balloon_assets` を呼び直す | 値をこの 2 つの中か手前で導けば追加なしで付いてくる | − |
| 2（バルーンの宣言を読む） | `read_descript_layer`（基層だけを読む） | 値を読む関数・運ぶ経路が無い。7 は 2 層を重ねる読み手に足すと破れる | Missing／Constraint |
| 3（`1` は変えない） | 今の 2 つの腕 | 変更なし。読み手の契約を変える案を採るときだけ画素一致の確かめが要る | Constraint |
| 4（`full`） | `select_source` の表 | `normalize` の `(Full, AlphaChannel)`・`(Full, Opaque)`。α 無しの絵が全画素 α=255 で届くことの確かめ | Missing／Unknown |
| 5（`0`） | `select_source` の表・`clear_key_color` | `(Off, KeyColor)`。α を持つ絵の元の色を得る口。動く絵の残りのコマ。`key_color` と記録の広げ | Missing／Constraint |
| 5 の 7（`.pna`） | `Off` × `.pna` → `Unsupported(Pna)` → `BakeError::Normalize` | 形は今のまま届く。記録の文言が「`.pna` 非対応のため」と分かるかの確かめ。バルーンは 1 枚落ちると全体が失敗になる（2.5） | Constraint |
| 6（画像だけのシェル） | `select_surface_images`・`apply_base_images` | `load_shell_target` の 2 つの失敗の条件。`ShellLoadError`／`BootWiringError` の文言 | Missing |
| 7（記録） | `load_shell_target` の「1 回につき 1 度」の型 | 透過の扱いの `info!`、読めない値の `warn!`、画像だけで組んだときの `info!` | Missing |
| 8（見た目・文書） | 検体の窓口（`sample-ghost-kit`）・既存の golden | 文書 4 つの書き直し。台帳の検査との整合 | Missing／Unknown |

## 4. 進め方の選択肢

### 4.1 `0` × α を持つ絵の色をどこで得るか（brief 議題 1）

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| ア: 正規化の段で逆算する | 乗算済みの画素を α で割り戻してから不透明にして抜く。読み手は触らない | 変更が `normalize.rs` と `animated.rs` に収まる | α=0 の画素は色が失われていて黒になる。半透明の画素は色がずれる。**要件 5 の 2・3 を文字どおりには満たせない**（元が透明な画素は「絵に書かれた色」でなく黒で出る。左上が透明な絵では黒が抜き色になる） |
| イ: 読み手に「乗算していない色で返す」口を足す | `ElementDecoder` に引数か兄弟のメソッドを足し、`Off` のときだけ WIC は `32bppBGRA`（乗算なし）へ変換、`image_arm::to_bgra` は乗算を飛ばす。正規化が α を 255 にして抜く | 元の色がそのまま得られる。`1`・`full` の経路は 1 バイトも変わらない | 読み手の口（静止画・動く絵の全コマ・動きの 1 枚目）とテスト用の `MemoryDecoder` に手が入る。`bake` が読む前に宣言を見る形になる |
| ウ: 読み手は常に乗算なしで返し、正規化の末尾で乗算する | `normalize.rs` 冒頭のコメント「シーム腕実装時は本段末尾で premultiply する」の向き | 口が 1 つで済み、3 通りが同じ形で書ける | `1` の経路の画素が「WIC の乗算」から「自前の乗算」に替わる（要件 3 の 3 の画素一致は式の一致に頼る）。乗算済みのデータを `MemoryDecoder` に入れている既存のテスト（数十ファイル）と golden に波及する |

- 要件 5 の 2・3・6 を縛りどおりに満たせるのはイとウ。変更の広がりはイ＜ウ。
- どの案でも `bake` の「0 番のコマだけ正規化 → 残りは抜き色を消すだけ」（`PendingFrames::new`）は、`Off` × α を持つ動く絵のために「残りのコマにも同じ扱いを当てる」形へ広げる必要がある。

### 4.2 宣言の文字列を値にする関数をどこに置くか

- `UseSelfAlpha` は `areka-emo-atlas` に在り、`areka-emo-atlas` が `areka-parsers` に依存している（逆向きには依存できない）。
- 案 A: `areka-parsers` に写しの型（3 通り＋読めなかった生の値）を置き、`areka-emo-present` が `UseSelfAlpha` へ写す。brief の「読む口は `areka-parsers`」に沿う。型が 2 つになる。
- 案 B: 関数を `areka-emo-present`（または `areka-emo-atlas`）に置き、`Option<&str>` → `UseSelfAlpha`＋出どころ（宣言／既定／読めない値）を返す。型は 1 つ。DPI の前例（`parse_author_dpi` は `areka` に在る）に近い。
- どちらでも、シェルとバルーンは**キー名だけが違う同じ関数**で足りる（`seriko.use_self_alpha`／`use_self_alpha`）。要件 2 の 8（混ぜない）はキー名を引数にすれば構造で成り立つ。

### 4.3 シェルの値をどう運ぶか

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| S1: `load_shell_target` が自分で `descript.txt` を読む | 署名は変えない。読取失敗は「宣言なし」として続ける | 呼び手 5 か所（本番 2・examples 3）が無変更。起動と採寸が食い違う余地が無い。記録の置き場が今の「記録を出す場所」と同じ。要件 1 の 5 をこの入口の中では満たせる | 同じファイルの読み手が 3 つ目になる |
| S2: 呼び手が値を渡す（DPI と同じ運び方） | `placement::source` が `shell_kv` から読み、`measure` と `build_shell_assets` へ引数で渡す | 二重の読取が無い | `load_shell_target`・`build_shell_target*`・呼び手・テストの署名が変わる。起動と採寸が別々に値を作ると食い違いうる |

- fs を触らない核（`build_shell_target_with_boxes`）は、どちらの案でも値を引数で受ける形になる（メモリ上の復号器でテストするため）。

### 4.4 バルーンの値をどう運ぶか

- 要件 2 の 7 のため、**値は `descript.txt` の基層だけから読む**（`BalloonModel` の 2 層の重ね合わせには入れない）。
- B1: `build_balloon_target_from_faces` の中で読む。署名は変わらないが、スコープごと × （起動＋採寸）で読取と記録が繰り返され、要件 7 の 1「読み込み 1 回につき 1 度」と合わない。
- B2: 「バルーンのフォルダ → 透過の扱い」の関数を `balloon.rs` に足し、`build_balloon_assets`（起動・切り替え）と採寸がスコープのループの外で 1 回呼んで、`build_balloon_target_from_faces` へ引数で渡す。署名が 1 つ変わり、呼び手（本番 2・ラッパ `build_balloon_target`・examples）が追随する。

### 4.5 画像だけのシェル（要件 6）

- 入口 `load_shell_target` で次のように変えれば届く見込み:
  - `surfaces.txt` の読取失敗のうち「無い」（`ErrorKind::NotFound`）だけを空の定義として続け、ほかの失敗は今までどおり `ShellLoadError::Read`（要件 6 の 6）。
  - `shell.surfaces` が空でも、認めた面の画像が 1 つ以上在れば続ける。0 個なら `ShellLoadError::Empty`（要件 6 の 5）。
- `ShellLoadError::Empty` と `BootWiringError::ShellEmpty` の文言・`path` の意味（今は「`surfaces.txt` が…」）を「面が 1 つも無い」に合わせる必要がある。枝を増やすか文言だけ直すかは設計で決める。
- 新しい層は要らない（`EmoWorld::build_with_images` は空の `Shell` と画像の対応から面を新設できる）。

### 4.6 全体の組み方

- **A（今ある部品を広げる）**: `normalize.rs` に 3 つの腕、`load_shell_target` に条件 2 つ、値を読む小さい関数 1 つ、運ぶ引数。新しいファイルはテストの兄弟ファイルだけ。本仕様の大きさに合う。
- **B（透過の宣言を扱う新しいモジュールを立てる）**: 宣言の読取・出どころ・記録をまとめた型を新設する。キー 2 つ・値 3 通りに対しては大きすぎる。
- **C（混ぜる）**: 正規化と入口は A、宣言の文字列 → 値だけ小さい純粋な関数として切り出す（4.2 の A か B）。実質は A と同じで、置き場所だけの違い。

## 5. 規模とリスク

- **規模: M（3〜7 日）**。腕 3 つと入口の条件は小さいが、`0` × α の色の口（4.1）・動く絵の残りのコマ・値を運ぶ署名の追随・文書 4 つ・検体での見た目の確かめが重なる。brief の見立て（S〜M・8〜12 タスク）の上の端。
- **リスク: 中**。
  - `0` × α の案の選び方で、読み手の契約か既存の画素一致のどちらかに触れる（4.1）。
  - 既定が `1` → `0` に変わるので、宣言の無い資産の見た目は実機で確かめる必要がある（手持ちで該当は 2 体）。
  - 「シームである」ことを固定している既存のテストが赤になる（下記）。直す前提で数えておく。

### 書き換えが要る既存のテスト（見つけた分）

- `normalize.rs` の `tests`: `full_with_alpha_selects_alphachannel_but_seam`・`full_no_alpha_selects_opaque_seam`・`off_no_pna_selects_keycolor_seam`（`Unsupported` を期待）。`off_with_pna_ignores_own_alpha_selects_pna_seam` は要件 5 の 7 と合うので残る。
- `normalize_key_color_tests.rs`: `Normalizer::key_color` が `Full`・`Off` で `None` を返すことを見ている箇所。
- `shell_target` のテスト: `surfaces.txt` の読取失敗・面 0 個の失敗を見ているもの（要件 6 で条件が変わる）。
- テストの中で `AlphaParams { use_self_alpha: UseSelfAlpha::On }` を直に組んでいるファイルは約 40 在るが、`AlphaParams` の形を変えなければ無変更で通る。

## 6. 設計へ持ち越す調べもの（Research Needed）

1. **パレットの PNG に `tRNS` が付いた絵を WIC がどう返すか**。`pixel_format_has_alpha` の一覧に索引つきの形式は無いので `has_alpha = false` になるはずだが、PBGRA へ変換した画素に α<255 が出るなら、`full`（全画素不透明）と `0`（半透明 0 個）の縛りが破れる。色の型が RGB で `tRNS`（1 色を透明に）を持つ絵も同じ。手持ちの検体には無い見込み（要件の申し送り 3）なので、合成した絵で確かめる。
2. **α を持たない絵は必ず全画素 α=255 で届くか**（`full` の腕を「そのまま渡す」で済ませられるかの前提）。WIC の変換のコメントは「α 無し画像も 100% 不透明として埋める」と書いているが、動く絵の読み手（`image`）の側も同じかを確かめる。
3. **WIC の `32bppBGRA`（乗算なし）は、α=0 の画素の色を元のまま返すか**（4.1 の案イ・ウの前提）。
4. **`AlphaMask::from_pbgra32` の判定**（α がいくつ以上を「当たり」とするか）。要件 4 の 5・5 の 8 が追加の実装なしで成り立つことの裏取り。
5. **台帳の検査**（`crates/ukadoc-survey`）が、`assets.toml` の行の状態・担当・根拠の綴りに何を求めるか。行を書き換えたときに赤くなる検査が無いか。
6. **面 0 だけの画像だけのシェル**で、相方（`KERO_INITIAL_SURFACE_ID`）の面が無いときの今の振る舞い。本仕様の外の既存の経路だが、要件 6 の検体を作るときに踏む。
7. **画像だけのシェルの検体**が手持ちに無い。テストは合成したフォルダで足りるが、要件 8 の「見た目の確かめ」に入れるかを決める。

## 7. 要件ディスカッションへ出す論点

1. **`0` × α を持つ絵の「絵に書かれた色」をどこまで厳密に求めるか**（4.1）。要件 5 の 2・3 を文字どおり守るなら読み手に手が入る（案イ・ウ）。正規化の段の逆算（案ア）で良しとするなら、要件 5 の 2・3 の文面を「元が完全に透明な画素は黒として扱う」等に直す必要がある。
2. **要件 1 の 5「シェルの descript.txt が無いとき」**。今は `placement::source::load_descript_source_for_shell` と `emo2_boot::assets::build_shell_assets` が、シェルの `descript.txt` の読取失敗を起動の失敗にしている（2.6）。⑴ この文言を「行が無いとき」だけに狭める、⑵ 2 か所の失敗も緩めて `descript.txt` の無いシェルを起動できるようにする（画像だけのシェル＝要件 6 と相性が良いが、範囲が広がる）、のどちらか。
3. **要件 7 の 1「読み込み 1 回につき 1 度」の数え方**。シェルの入口 `load_shell_target` は起動（切り替え）と採寸の 2 か所から呼ばれるので、今の記録（面の画像の一覧の `info!`）も 1 回の起動で 2 行出ている。バルーンはさらにスコープごとに焼く。「入口の呼び出し 1 回につき 1 度」でよいか、「利用者から見た 1 回の読み込みで 1 行」まで求めるか。
4. **`0`（既定）× `.pna` 在りのバルーン**（要件 5 の 7）。バルーンは絵が 1 枚でも焼けないと全体の構築が失敗する（`build_balloon_target_from_faces`）。「その絵を表示せず」は、バルーンでは「そのバルーンを使えない」になる。今の `1` × α 無し × `.pna` 在りと同じ結果ではあるが、既定が `0` になると「α 付きの絵＋`.pna`＋宣言なし」のバルーンが新しくこれに当たる（要件の申し送り 4 の続き）。
5. **宣言の値の読み方の幅**。`1`・`true`・`full`・`0` の大文字小文字（`TRUE`・`Full`）、前後の空白、`false` を読めない値として既定に落とすか。正典に定めが無いので、要件 1 の 6・2 の 6 の「どれでもない」の線引きを決めたい。
6. **`ShellLoadError::Empty`／`BootWiringError::ShellEmpty` の意味の変更**（4.5）。失敗の種類は今のまま文言だけ直すか、「`surfaces.txt` が無く画像も無い」を別の枝にするか。利用者に出る文言（既定ゴーストの台詞の材料）に関わる。
7. **brief が挙げた「決め打ち 5 か所」は本番では 2 か所**だった（残り 3 つはテストの中）。台帳 `assets.toml` のバルーンの行は既に「この 1 か所だけ」と正しく書いている。要件の文面には影響しないが、設計・タスクの数え方の前提として共有する。

## 8. 要件ディスカッションの結果（2026-10-05）

- **要件へその場で入れたもの**: 宣言の値の比べ方（前後の空白を除く・英字の大小を区別しない・`false` は読めない値。論点 5）、「読み込み 1 回につき 1 度」の数え方（入口の呼び出し 1 回・バルーンはスコープごとに繰り返さない。論点 3。バルーンは 4.4 の B2 の向きになる）。
- **設計へ送るもの**（要件の文面は変えない）:
  1. `0` と明示 × α を持つ絵の色の得方（4.1）→ **裁定で軽くなった**。「読み込み時に α を切って、残った赤・緑・青で抜き色を決める。ゴースト作者の食い違いを繕わない」（開発者 2026-10-05）。透明だった画素の元の色の正確さは求めないので、読み手に乗算なしの口を足す案イ・ウは採らなくてよい。いちばん手間の少ないやり方（手元の画素の α を 255 にして抜く、など）を設計で選ぶ。6 章の調べもの 3 は不要。
  7. **議題 1 の裁定**: 宣言が無いときは正典の既定 `0` でなく、絵の中身を見て決める（要件 9。透明・半透明の画素を 1 つでも持てば α、全部不透明か α 無しなら抜き色。動く絵は 1 枚目のコマで決める）。`UseSelfAlpha` に 4 つ目の場合（宣言なし）が要る。明示の `0` は正典どおり。
  2. 宣言の文字列を値にする関数の置き場所（4.2）と、シェル・バルーンの値の運び方（4.3・4.4）。
  3. `ShellLoadError::Empty`／`BootWiringError::ShellEmpty` の枝と文言（4.5・論点 6）。
  4. バルーンの透過の値は基層の `descript.txt` だけから読む（論点 7）。
  5. 6 章の調べもの 7 件（`tRNS` の付いた絵の届き方を含む）。
  6. 本番の決め打ちは 2 か所という数え方（論点 8）。
  8. **議題 2 の裁定**: `.pna` は宣言が何であっても無いものとして扱い、絵は普通に描く（要件 5 の 7）。`select_source` の表から `.pna` の有無による枝（`Pna` → `Unsupported`）が無くなる。バルーンが `.pna` のせいで全体ごと使えなくなる経路（論点 4）も無くなる。`probe_pna` は「無視した枚数」の記録のためだけに残るか、設計で決める。既存のテスト `off_with_pna_ignores_own_alpha_selects_pna_seam` などは書き換えになる。
  9. **議題 3 の裁定**: シェルの `descript.txt` が無いシェルは扱わない（論点 2 の ⑴）。`placement::source::load_descript_source_for_shell` と `emo2_boot::assets::build_shell_assets` の失敗は今のまま。

## 9. 設計の調べと決定（2026-10-05・`/kiro-spec-design`）

- 調べの種類: 既存の仕組みの拡張（軽い調べ）。ソースを読んだだけで、ビルド・実行はしていない。外部クレートの追加は無い。
- 1〜7 章は裁定の前に書いたもので一部が古い（`select_source` の表・4.1 の案の表など）。正は `requirements.md` と本章と `design.md`。

### 9.1 6 章の調べものの答え

| 番号 | 答え | 設計への効き方 |
|---|---|---|
| 1（パレットの PNG に `tRNS`） | 実物では確かめていない。`decode/wic_arm.rs` の `pixel_format_has_alpha` の一覧に索引つきの形式は無いので、`has_alpha` が偽で届き、α<255 の画素を含みうる、という読みのまま | 設計を答えに依らない形にした。`full` × α なしと `0` は全画素の α を 255 に書くので、要件 4 の 2・5 の 6 は読み手の返し方に依らず成り立つ。`1` と宣言なしでは今と同じ扱い（抜き色の枝）で、読み手は変えない |
| 2（α の無い絵は全画素 α=255 で届くか） | WIC の側はコメントの記述だけ。動く絵の側（`image`）は実物では確かめていない | 上と同じく依らない形にしたので、確かめは要らなくなった |
| 3（WIC の乗算なしの形式） | 不要（裁定で、読み手に乗算なしの口を足さないと決まった） | − |
| 4（`AlphaMask` の判定） | `wintf` の `alpha_mask.rs` の `pack_pbgra32_alpha` が、α が 128 以上の画素を当たりにする | 要件 4 の 5・5 の 8・9 の 5 は、焼いた画素の α（255 か 0）が正しければ追加の実装なしで成り立つ |
| 5（台帳の検査） | `implemented` の行は、ソースの定義箇所に正典の URL の 1 行コメントが要る（無いと `ImplementedWithoutEvidence`）。`degraded` は説明に違いを書けばよい。報告は台帳と一致していることを検査される（`check/freshness.rs`） | 2 行とも `degraded` にする。直した後は `cargo test -p ukadoc-survey` を通し、報告は `ukadoc-survey` の `report`・`report-summary` で作り直す |
| 6（面 0 だけのシェルと相方） | 採寸（`placement::measure::measure_native_scope_sizes`）は、番号 10 の面の合成に失敗するとスコープ 0 の寸法で代えて `warn!` を出す。表示の側は追っていない（既存の経路） | 本仕様は変えない。範囲外として設計に書いた |
| 7（画像だけのシェルの検体） | 手持ちに無い | テストは一時フォルダに合成する。実機の確かめは `target\` の下に手で作ったフォルダで行う |

### 9.2 新しく分かったこと

- `NormalizeError`・`AlphaSource`・`BakeError::Normalize` を使っているのは `areka-emo-atlas` の中だけ（ほかのクレートからの参照は grep で 0 件）。`.pna` を無いものとして扱うと描けない組み合わせが無くなるので、正規化は失敗を返さなくてよい。
- `ElementDecoder::probe_pna` を本番で呼ぶのは `bake_with_limits` の 1 か所。実装は `WicDecoderArm`（同じ名前の `.pna` が在るか）と `MemoryDecoder` の 2 つ。
- 動く絵の 2 枚目以降は `animated.rs` の `PendingFrames::new` が「抜き色 1 つ」だけを受けている。`0`（α を 255 にしてから抜く）と `full` × α なしのために、「1 枚目で決めた画素の扱い」を受ける形に替える必要がある。
- `animated_tests.rs` の `picture_dropped_by_normalize_is_not_counted_in_the_total` は、`.pna` を使って正規化を失敗させている。正規化が失敗しなくなるので、この枝とテストは無くなる。
- バルーンの `balloon.rs` は charset つきで読む `read_decoded` と、基層を読む `read_descript_layer`（失敗は `warn!`）を既に持つ。透過の宣言の読取はこれを使える。
- 切り替え（`emo2_boot::switch_assets`）は `build_shell_assets`／`build_balloon_assets` を呼び直す。入口かこの 2 つの中で値を導けば、切り替えにも付いてくる（確かめた）。
- 説明書の文言を固定しているテストは無い（`README.txt` を読むテストは `crates/` に 0 件）。

### 9.3 設計の決定

#### 決定: 透過の決定を「1 枚目で決める扱い（`AlphaRule`）」1 つにまとめる

- 背景: 4 通りの宣言 × 絵の場合を描き分け、動く絵の全部のコマに同じ扱いを当てる必要がある。
- 比べた案: ⑴ 今の `select_source`＋`normalize` の場合分けに腕を足し、動く絵の側へ別に「不透明にする」の引数を足す ⑵ 「α を 255 にするか」「どの 4 バイトを抜くか」の 2 欄の値を 1 か所で決め、静止画にも全部のコマにも同じ関数で当てる。
- 採った案: ⑵。決める場所が 1 つになり、今の `key_color` と `select_source` と失敗の型が要らなくなる（削除のほうが多い）。
- 引き受けること: `Normalizer::normalize` の署名が変わる（`has_pna` と `Result` が無くなる）。呼び手は `areka-emo-atlas` の中だけ。

#### 決定: 明示の `0` は「α を 255 に書いてから抜く」

- 裁定「読み込み時に α を切って、残った赤・緑・青で抜き色を決める。作者の食い違いを繕わない」に対する、いちばん手間の少ないやり方。届いた乗算済みの画素の 4 バイト目を 255 に書くだけで、読み手にも色にも触らない。元が透明だった所は乗算済みの色（多くは黒）で出る。

#### 決定: 宣言なしは「`has_alpha` が真で、α<255 の画素が 1 つ以上」なら α、ほかは抜き色

- 要件 9 の 1〜3 をそのまま写した。「α を持つ」の物差しは `1` のときと同じ（読み手の `has_alpha`）なので、宣言なし × α なしは `1` × α なしと必ず同じ結果になる（要件 8 の 2）。

#### 決定: 宣言を読む関数は `areka-emo-present` に 1 つ（`self_alpha.rs`）

- `UseSelfAlpha` は `areka-emo-atlas` に在り、`areka-emo-atlas` は `areka-parsers` に依存している。`areka-parsers` に写しの型を置く案（4.2 の A）は型が 2 つになるので採らない。
- シェルの入口とバルーンの入口の両方から使うので、どちらかのファイルに寄せず小さいファイル 1 つにした。型は足さない（戻り値は `UseSelfAlpha` だけ）。記録もこの関数が出すので、読み方と記録の 3 つの場合が 1 か所で固定できる。

#### 決定: シェルは入口が自分で読む（4.3 の S1）・バルーンはループの外で 1 回読んで渡す（4.4 の B2）

- シェル: `load_shell_target` の署名を変えない。呼び手（本番 2・examples）が無変更で、起動と採寸が食い違わない。
- バルーン: スコープごとに焼くので、焼く関数の中で読むと記録がスコープの数だけ出る（要件 7 の 1 に合わない）。`load_balloon_use_self_alpha` を新設し、`build_balloon_assets` と採寸がループの前で呼ぶ。1 スコープ用の包み `build_balloon_target` は中で呼ぶ（署名は変えない）。

#### 決定: `.pna` の数は、シェルは焼きの結果から・バルーンはフォルダの一覧から

- シェルは焼きが 1 回なので、`bake` が今も呼んでいる `probe_pna` の答えを数えて返せば正確（サブフォルダの絵も入る）。
- バルーンはスコープごとに焼くので、1 回だけ数えられる場所が「宣言を読む関数」になる。そこではフォルダ直下の `.pna` のファイルを数える（areka が読まない絵に添えた物も含む）。
- 比べた案: 両方ともフォルダの一覧で数えて `probe_pna` を消す。シェルの絵はサブフォルダにも置けるので、そこに添えた `.pna` が記録から漏れる。採らない。

#### 決定: `ShellLoadError` の枝は増やさない

- `Empty` の意味を「面が 1 つも無い」に広げ、文言を直す。`path` はシェルのフォルダにする。`BootWiringError::ShellEmpty` も文言だけ直す。「`surfaces.txt` が無く画像も無い」を別の枝にしても、呼び手のすることは変わらない。

### 9.4 まとめ直し（synthesis）

- **まとめられたもの**: `1` の抜き色・`0` の抜き色・宣言なしの抜き色・動く絵の 2 枚目以降は、どれも「（要れば α を 255 にして）決めた 4 バイトを抜く」の同じ処理。1 つの値（`AlphaRule`）で表した。シェルとバルーンの宣言の読みは、キー名だけが違う同じ関数。
- **作るか借りるか**: 値の読みは `areka_parsers::kv::parse_kv`、抜く処理は今の `clear_key_color`、面の新設は今の `apply_base_images`、当たり判定は今の `AlphaMask` を使う。新しく書くのは「α を 255 に書く」数行と「α<255 を探す」数行と、宣言の読み。
- **削ったもの**: 透過の宣言を扱う新しいモジュールや出どころの型（4.6 の B）、読み手の乗算なしの口（4.1 のイ・ウ）、`AlphaSource`・`NormalizeError`・`BakeError::Normalize`、`ShellLoadError` の新しい枝。

### 9.5 残るリスク

- パレットの PNG に `tRNS` が付いた絵の届き方は実物で確かめていない。設計は答えに依らないが、`full`／`0` でその絵の透明だった所が黒などで出る見た目は、実機の確かめで 1 度見ておくとよい（手持ちの検体には無い見込み）。
- 宣言なしの資産の見た目は、「α を持つが全部の画素が不透明な絵」でだけ変わる（左上の色が抜かれる）。手持ちの検体には無い見込みだが、実機の確かめで宣言なしの 2 体を見る。
- 既存のテストの書き換え（設計の「書き換え・削除する既存のテスト」）は、赤くなる前提で数えておく。
