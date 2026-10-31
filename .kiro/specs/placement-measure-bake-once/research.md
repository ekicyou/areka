# ギャップ分析: placement-measure-bake-once

> 2026-10-31・ブランチ `claude/kiro-start-placement-measure-bake-once-80df62`（main `da020c10` の上・コミット `6c915ae0`）で読み直した。行番号はこの時点のもの。設計の着手時に引き直すこと。
>
> 結論は出さない（案 A と案 B のどちらを取るかは要件討議の議題 1）。ここに置くのは、今のコードの道のり・足りない所・案ごとの触る所と数・調べ残しである。

## 1. 要約

- **今の道のりは brief のとおり**: 起動の準備で、採寸 `placement/measure.rs` と資産の組み立て `emo2_boot/assets.rs` が、それぞれ `load_shell_target` を 1 回ずつ呼んでシェルを全部焼く（動く絵は 2 回とも全コマ）。バルーンは採寸が scope ごとに面 0 だけ、資産の組み立てが scope ごとに全部の面を焼く。起動・ゴーストの切り替え・読み直しの 3 つとも `ghost_session::prepare_ghost_windows` → `boot_ghost`／`boot_ghost_strict` → `wire_emo2_boot` の同じ道を通る。拡大率の替わりは採寸を呼ばない（窓の寸法は持っている値を引き直すだけ）。
- **案 B は brief の見立てより小さくできる**: `load_shell_target` も `build_balloon_target_from_faces` も読み手（`ElementDecoder`）を引数で受けるので、採寸の側で「動く絵を 1 枚目だけで読む読み手」で包めば、`shell_target.rs`・`balloon.rs`・`areka-emo-atlas` を変えずに済む。上限（`AnimationLimits { max_frames: 1 }`）を渡す形は、動く絵 1 つごとに「上限を超えたので 1 枚だけ読みます」の `warn!` を起動のたびに出してしまうので、筋が悪い。ただし案 B だけでは要件 3（警告を 1 行ずつ）は満たせない。
- **案 A の運び方には先例がある**: 準備が決めた値を起動の結線へ運ぶ置き場 `BootShellChoice`（`ghost_session.rs:120`・準備の 219 行目で置き、129 行目の `take_boot_shell` で取る）と同じ形で、焼いた `ShellTarget` を運べる。一方で、採寸の「アセットは持たない・採寸後に捨てる」（`measure.rs:20-22`・42-50）と `PreparedPlacement` の Send 契約（`placement/mod.rs:262-264`・`placement_prepare_tests.rs:228`）は今の設計が明示した取り決めで、案 A はこれを書き換える。バルーンまで 1 回にするには、採寸の scope の集まり（descript から決める）と資産の scope の集まり（`derive_scopes()` が常に `[0, 1]`・`emo2_boot/mod.rs:307`）の食い違いを扱う必要がある。
- **バルーンの読み込みも起動のたびに同じ記録を重ねて出している**（要件の作者は確かめていなかった点）: 面が本体側の系列へ縮退した `warn!` は scope 1 以上で採寸・`windowposition` の読み・資産の組み立ての 3 か所から最大 3 行、透過の宣言の `info!`・`.pna` の `warn!` は 2 行ずつ出る。要件 3 が挙げる 3 種（脱落・箱・入れ子）はどれもシェルの側だけなので、要件 3 の範囲にバルーンや焼きの段（`areka-emo-atlas`）の警告を含めるかは決めが要る。
- **数を数える口は半分しか無い**: シェルを読んだ回数は、読み込みの権威が 1 回ごとに出す `info!`（欄 `recognized`・`shell_dir`）で数えられ、`boot_shell_tests.rs` が本番と同じ順で「2 回読む」をすでに固定している（165・209 行目の `vec![x.clone(), x.clone()]`）。コマを読んだ数・バルーンを焼いた回数を数える口は無い。採寸も資産の組み立ても読み手を中で作る（`measure.rs:160`・`assets.rs:251`）ので、数える読み手を差し込む入口が要る。

## 2. 今のコードの道のり

### 2.1 起動の準備の順

| 段 | 場所 | 何をするか |
| --- | --- | --- |
| 1 | `main.rs:301` `open_ghost_windows` → `ghost_session.rs:197` `prepare_ghost_windows` | 起動のシェルを決め（`decide_boot_shell`）、配置の準備を呼ぶ |
| 2 | `ghost_session.rs:214` → `placement/mod.rs:854` `prepare_ghost_windows_for_shell` → 435 行目 `prepare_stages_for_shell` | descript を読み、k₀ を決め、`measure::measure_scope_sizes`（460 行目）を呼ぶ |
| 3 | `measure.rs:153` `measure_native_scope_sizes` | 読み手 `WicDecoderArm::new()`（160）→ `build_shell_assets`（169 → 338・343 行目で `load_shell_target`）→ scope ごとに `compose_size`、バルーンは scope ごとに `measure_balloon_surface0`（377・面 0 だけを `build_balloon_target_from_faces` で焼く）。焼いた結果は関数の終わりで捨てる（234 行目） |
| 4 | `placement/mod.rs:787` `scope_windowposition` | scope ごとに `resolve_balloon_faces` をもう 1 回呼び、`load_scope_balloon_model` で定義を読む（焼かない） |
| 5 | `ghost_session.rs:219` | `BootShellChoice` を World に置く。窓を作る閉包を組む（窓は投函の後の `Input` 段で生える） |
| 6 | `main.rs:514` `boot_ghost` → `ghost_session.rs:739` `take_boot_shell` → 860 行目 `wire_emo2_boot` | `emo2_boot/mod.rs:452` `build_boot_assets_for` → `assets.rs:241` `build_boot_assets_with_shell`（読み手を 251 行目で作り直す）→ `build_shell_assets`（343 行目で `load_shell_target` 2 回目）→ `build_balloon_assets`（scope ごとに全部の面を焼く・473 行目） |

- ゴーストの切り替え・読み直し: `emo2_boot/ghost_switch.rs:857` `reopen_ghost_windows`（`ghost_session.rs:373`・中身は `prepare_ghost_windows`）→ 862 行目 `boot_ghost_strict` → `wire_emo2_boot`。起動と同じ道なので、どちらの案でも同じ直しが届く（要件 1.3・3.2）。
- シェルの着替え・バルーンの着替え（`switch_assets.rs`）は `build_shell_assets`／`build_balloon_assets` を直接呼び、採寸を通らない（範囲外・変更 0）。案 A で `build_shell_assets` の形を変えるなら、着替えの呼び手（`switch_assets.rs:32`・251）を変えないように兄弟の関数で受ける必要がある。
- 拡大率の替わり: `measure_scope_sizes` の呼び手は `placement/mod.rs:460` の 1 か所だけ。`dpi_sync.rs`・`emo2_boot/frame/dpi.rs` は焼き直しも採寸もしない。
- テストと手本の呼び手: `emo2_boot/spine.rs:653`（`build_boot_assets` を直接）・`ghost_switch_test_support.rs:440`（準備を通らず `boot_ghost`）・`boot_shell_tests.rs`（本番と同じ順で準備 → 起動）・`mcp/dump_surface_gpu_test_support.rs:272`・326・`offline_dump/mod.rs:192`・examples 3 本（`load_shell_target` を直接）。

### 2.2 焼きの段と上限の口（`areka-emo-atlas`）

- `bake`（`lib.rs:62`）は `bake_with_limits(…, AnimationLimits::from_env())` を呼ぶだけ。`load_shell_target`（`shell_target.rs:547`）とバルーン（`balloon.rs` の `build_balloon_target_from_faces`）はどちらも `bake` を呼ぶので、外から上限を渡す道は今は無い。
- 動く絵の読み方（`animated.rs:35` `load`）: 読み手の `probe_animation` が見出しを返せば上限を判定（`limits.rs` の `judge`）→ 収まれば `decode_frames`（全コマ）→ 超えたら `warn!`「動く絵が上限を超えたので 1 枚だけ読みます」の上で `decode_first_frame` → それも駄目なら `decode`（WIC の 1 枚目）。見出しが無ければ `decode` だけ。
- `AnimationLimits` の欄は `pub` なので `max_frames: 1` を組むことはできる（環境変数の読みは 1 以上だけを通す）。ただし上の `warn!` を動く絵 1 つごとに出す。
- `ElementDecoder`（`decode.rs:104`）は `&self` の trait で、`decode`・`probe_pna`・`probe_animation`・`decode_frames`・`decode_first_frame` の 5 つ。`decode_first_frame` は「`decode_frames` の 0 番のコマと同じ画素・同じ `has_alpha`」が契約。`decode_frames` は見出しどおりの枚数・寸法でなければ失敗として扱われる（`animated.rs:60-68`）ので、動く絵の全コマは 1 枚目と同じ寸法である。
- 完了 `areka-P0-animated-image-decode` の research.md 9.14 節は「前」を作るのに、`WicDecoderArm` を包んで `probe_animation` を既定（`None`）にした読み手を使っている。読み手で包む形には先例がある。同じ節の ⑷ で `decode_first_frame` は上限いっぱいの APNG で release 4.6 ms・WebP 6.2 ms、山 11 MiB（全コマを焼く 1,373 ms／1,615 ms・523 MiB と比べて）。

### 2.3 シェルの読み込みが出す記録（`shell_target.rs:318-471`）

`load_shell_target` が読み込み 1 回につき 1 度ずつ出す。採寸と資産の組み立てで 2 回呼ぶので、起動 1 回で全部 2 行ずつになる。

- 要件 3 の 3 種: 焼く段で落ちた絵の `error!`（424）・箱の報告の `warn!`（432-434）と箱の数の `info!`（443）・入れ子の `warn!`（426-428）。
- 3 種の外で同じく 2 行ずつ出るもの: 同じ番号の重複の `warn!`（327）・descript が読めない `warn!`（382）・透過の宣言の `info!`／`warn!`（`self_alpha.rs`）・`.pna` の `warn!`（402）・相手の無いコマの `warn!`（417）・形の誤りの `warn!`（430）・描けない描画メソッドの `warn!`（436）・画像だけで組んだ `info!`（455）・一覧の `info!`（462）。
- 焼きの段の中の `warn!`（`areka-emo-atlas` の `lib.rs`・`animated.rs`）: 元画像 0 寸・全透明・動く絵の上限超え・動く絵として読めない。焼く回数だけ出る。
- 面の表を組むたびに出る畳み込みの `warn!`（`areka-emo-compose` の `fold.rs`・`ShellTarget::build_world` の説明 272-274）: 今は採寸の読み込みの中で 1・採寸の `build_world` で 1・資産の読み込みの中で 1・資産の `build_world` で scope の数（2）＝起動 1 回で 5 回組む。

完了 `areka-P0-surface-element-nesting` の `real-machine-check.md` 87-88 行目は「検体のシェルの読み込み N=2・警告がそれぞれ 2 行」を実機で記録している（brief の記述の出どころ）。

### 2.4 バルーンの読み込みが起動で重ねて出す記録（要件の作者が確かめていなかった点）

| 記録 | 出どころ | 起動 1 回の行数 |
| --- | --- | --- |
| 面が本体側の系列へ縮退した `warn!`（scope 1 以上・縮退した面ごと） | `resolve_balloon_faces`（`balloon.rs:399`） | 採寸（`measure.rs:403`）・`windowposition` の読み（`placement/mod.rs` の `scope_windowposition`）・資産（`assets.rs:471`）で **最大 3 行** |
| 系列の解決が完了した `info!` | 同 414 | scope ごとに 3 行 |
| 透過の宣言の `info!`／読めない値の `warn!` | `load_balloon_use_self_alpha`（`balloon.rs:604`）→ `self_alpha.rs` | 採寸（`measure.rs:202`）・資産（`assets.rs:469`）で 2 行 |
| `.pna` を使わない `warn!` | 同 620 | 2 行 |
| バルーンの descript が読めない `warn!` | `read_descript_layer` | 透過の読みで 2・`load_scope_balloon_model` で scope ごとにさらに |
| 2 層マージが確定した `info!` | `load_scope_balloon_model`（`balloon.rs:531`） | 「placement／boot の 2 呼出点から scope あたり 2 行出るが、値が一致すること自体が権威一元化の生き証人」と説明に書かれた、意図して重ねている記録 |
| 焼きの段の 0 寸・全透明の `warn!`（面 0） | `areka-emo-atlas` | 採寸（面 0 だけ）と資産（全部の面）で 2 行 |
| 面画像の焼きに失敗した `error!` | `balloon.rs:671` | 面 0 なら採寸が先に起動を止めるので 1 行。面 0 以外は資産だけなので 1 行 |

- emo2 の正常な検体では、上のうち `warn!` が実際に出るのは縮退の行だけの見込み（検体の縮退の有無は未確認＝調べ残し R4）。
- どちらの案でも、`windowposition` の読みのための 3 回目の系列の解決は残る（焼かないので本 spec の数には入らない）。

### 2.5 数を数える口

| 数えたいもの | 今ある口 | 足りないもの |
| --- | --- | --- |
| シェルを読んだ回数 | 読み込みの `info!`（欄 `recognized`・`shell_dir`）。`boot_shell_tests.rs:90` `loaded_shell_dirs` が `log_capture_kit::capture` で拾い、本番と同じ順（`prepare_ghost_windows` → `rig.boot`）で **2 回** を固定している（165・209 行目） | 案 A ならこの 2 本の期待値が 1 回に変わる。案 B で採寸の記録を出さない形にすると、この行は焼いた回数を数えなくなる |
| 警告の行の数 | 同じ捕まえ口で数えられる（`count_event` の形）。入れ子・箱の検体は `shell_target_nesting_tests.rs`・`shell_target_boxes_tests.rs` | 起動の準備 1 回を通した行の数の檻は無い |
| 動く絵のコマを読んだ数 | 無い。`MemoryDecoder`（`decode.rs`・`pub`）は答えるだけで数えない | 呼ばれた回数を数える読み手（`ElementDecoder` を包み、5 つの口ごとに `Cell` で数える）と、採寸・資産の組み立てに読み手を差し込む入口（今はどちらも `WicDecoderArm::new()` を中で呼ぶ） |
| バルーンを焼いた回数 | 無い（焼くたびに出る記録が無い） | 同じ数える読み手で、面のファイルごとの `decode` の回数から数えられる |
| 動く絵の検体 | `areka-emo-compose/tests/fixtures/animated-playback/`（シェルとバルーン・`rgb.apng`・`rgb.webp`・`alpha.webp`）。`shell_target_animated_fixture_tests.rs`・`film_playback_e2e_tests.rs` が本物の読み手で読んでいる | ゴーストの形（`ghost/master`・descript・SHIORI の偽物）で起こす検体は無い。`boot_shell_tests.rs` の土台（ゴースト A）に検体の絵を写せば作れる（写しはワークツリーの `target\` の下・要件 4.5） |
| COM | `with_com_initialized`（`measure_tests.rs:17` 等） | なし |

- 1,000 行の上限: `measure_tests.rs` はちょうど 1,000 行で足せない。新しいテストは兄弟のファイルへ置く（`measure_template_tests.rs` と同じ形）。
- 本文を読む構造テスト（`measure_template_tests.rs:308-336`）は `measure.rs` と `assets.rs` の本文にシェルの解析が戻っていないことを見ている。どちらの案でも新しいファイルは解析をしないので赤にはならない見込みだが、新しいファイルを走査の一覧に足すかは設計で決める（steering `structure.md:186`）。

## 3. 要件と今の資産の対応

| 要件 | 今ある資産 | ギャップ |
| --- | --- | --- |
| 1.1 採寸で 2 枚目以降を読まない | `decode_first_frame`（全読み手）・`ElementDecoder` を引数で受ける読み込みの入口 | **Missing**: 採寸の道は全コマを読む。案 A＝採寸と資産で 1 回だけ焼く運び方、案 B＝採寸用の読み手 |
| 1.2 全コマを読むのは 1 度だけ | 同上 | **Missing**（同上） |
| 1.3 切り替え・読み直しも同じ | 3 つの道が `prepare_ghost_windows` → `wire_emo2_boot` を共有 | なし（共有の道を直せば届く）。案 A は運んだ物を取り損ねたときの扱いが要る（5 章 A-4） |
| 1.4 静止画だけのシェルは同じ絵 | 案 B の包む読み手は見出しの無い絵をそのまま通す | 案 A は焼く入力が同じなので同じ |
| 1.5 再生へ渡す動く絵の中身は同じ | 資産の組み立ての `bake` は変えない | どちらの案も資産の側の読み方を変えない |
| 2.1〜2.3 窓の大きさ・位置・倍率が同じ | `measure_tests.rs`（emo2 の 434×687・400×224／288×203・k 適用）・`measure_template_tests.rs`（採寸と表示の外形の一致） | **Constraint**: 動く絵の検体での「全コマで測った値＝変えた後の値」の檻は無い（足す）。案 B は寸法が 1 枚目と同じになることが読み手の契約から言える（2.2 節） |
| 2.4 合成失敗は scope 0 で代わり＋同じ `warn!` | `measure.rs:211-220` | 案 A で採寸の中身を新しいファイルへ移すなら文言を写す（記録の語彙を変えない） |
| 2.5 読めないときは窓を作る前に失敗・記録あり | `PlacementError::Measure`（`measure.rs:342-346`）・`ShellLoadError` | 案 A で資産の側が運ばれた物を使うとき、資産の側の失敗の写し替え（`BootWiringError::ShellRead`／`ShellEmpty`）が通らなくなる道ができる（採寸が先に止めるので実害は無い見込み）。`assets.rs` の descript を読めないときの失敗（406-412）は残す |
| 2.6 窓の大きさと位置の記録は同じ | `placement/mod.rs:465-474` の `info!` | 変えない |
| 3.1〜3.4 警告を 1 行ずつ | 記録はすべて `load_shell_target` の 1 か所 | **Missing**: 案 A は自然に 1 行。案 B は採寸の道で記録を出さない入口が要る（`shell_target.rs` に触る）。**Unknown**: 3 種の外の記録（2.3 節・2.4 節）を含めるか |
| 4.1〜4.6 回数の檻 | 2.5 節 | **Missing**: 数える読み手・差し込む入口・動く絵のゴーストの検体 |
| 5.1〜5.5 測り直し・本物で確かめる | 9.14 節の測り方（捨てる測定プログラム） | 9.14 節の測り方は `load_shell_target` を通さず焼く入力を作り直して焼いている。「直した後」の採寸の分を測るには、採寸の関数そのものを測る形が要る（**Research Needed** R3） |

## 4. 実装の案

### 案 A: 焼くのを 1 回に（採寸が焼いた物を資産の組み立てへ運ぶ）

**運び方**（先例 `BootShellChoice` に合わせる場合）:

1. 採寸: `measure_native_scope_sizes` が焼いた `ShellTarget`（と、バルーンも共有するなら scope ごとの焼いた結果）を捨てずに返す口を足す。今の `measure_scope_sizes` と `MeasuredSizes`（素の数値だけ・`Clone`／`PartialEq`）は形を保つ。
2. 配置の準備: `PreparedPlacement` は `Clone, PartialEq, Eq` を導出しており（`placement/mod.rs:265`）、`ShellTarget` は `Clone` も `PartialEq` も持たない（`EmoWorld` が複製できない）。欄として足すと導出が壊れるので、`(PreparedPlacement, ShellTarget)` を返す兄弟の入口を新しいファイルに置く形になる。`placement` は examples から `#[path]` で読み込まれ `crate::` を使えない（`examples/window-placement.rs:105-109`）ので、運ぶ型は `areka-emo-present` の `ShellTarget` そのままか `placement` の中の型にする。新しいファイルは `placement` の子のモジュールにすれば、`mod.rs` の私有の `prepare_stages_for_shell` を呼べる（`mod.rs` に足すのは `mod` の 1 行と呼ぶ所の付け替え）。
3. `ghost_session::prepare_ghost_windows`: `BootShellChoice` と並べて、焼いた物の置き場（ゴーストの根とシェルのフォルダで照合する）を World に置く（数行・940 行の上限の内）。
4. 起動の結線: `wire_emo2_boot`（`world` を持つ）か `take_boot_shell` の隣で置き場を取り出し、`build_shell_assets` の兄弟（運ばれた `ShellTarget` から組む）へ渡す。運ばれていなければ今どおり自分で読む（準備を通らないテストの道・`spine.rs`・`ghost_switch_test_support.rs`）。着替えの道（`switch_assets.rs`）は今の `build_shell_assets` を呼び続けるので変更 0 にできる。
5. 面の表: `ShellTarget::take_world` が読み込みで組んだ表を 1 度だけ渡す（`shell_target.rs:283`）。採寸が `build_world` で組み、資産が scope ごとに組むと、畳み込みの `warn!` は今の 5 回から 3 回になる。採寸が組んだ表を資産の 1 つ目の scope へ回せば 2 回まで減る（設計の決め）。

**バルーンも 1 回にするか**（要件の表の「バルーン 1」）:

- 採寸は面 0 だけ・資産は全部の面を焼く。共有するには採寸の側で全部の面を焼くことになる（採寸の時間が少し延び、資産の側が丸ごと減る）。
- scope の集まりが食い違う: 採寸は descript から（`cfg.scopes`・本体側は常に・相方側は `kero.*` が在るとき・`char*` も）、資産は `derive_scopes()` の固定 `[0, 1]`。相方側を持たないゴーストでは資産だけが scope 1 を焼き、`char2` 以上を持つゴーストでは採寸だけが焼く。両方に在る scope だけを運び、残りは今どおり各自で焼く形になる。
- 合成は `Composer::compose(&EmoWorld, &AtlasTable, …)` で表を借りるだけなので、採寸に使った表をそのまま資産へ回せる。
- emo2 のバルーンは静止画だけで 1 回 11〜12 ms（brief）。動く絵のバルーンは今の検体には無い。

**触るファイル**: `placement/measure.rs`（465）・新しい兄弟（`placement/` の下に 1 本）・`placement/mod.rs`（970・`mod` の 1 行と付け替え）・`ghost_session.rs`（940・置き場を置く数行）・`emo2_boot/mod.rs`（828・取り出し）・`emo2_boot/assets.rs`（540・兄弟の関数）または新しい兄弟・`boot_shell_tests.rs`（期待値の 2→1）・テストの兄弟。バルーンを共有するなら `build_balloon_assets` の兄弟も。

**明示された取り決めとの関係**:

- `measure.rs:20-22`「採寸に使ったアセットは採寸後に破棄・二重ロードは M1 受容トレードオフ」・`measure.rs:42-45`「採寸後破棄の契約を型で担保する」・`placement/mod.rs:262-264`「Send な素の値のみを運ぶ・COM/WIC 等は持たない」。案 A はこの取り決めを本 spec の要件で書き換えることになる（`ShellTarget` は COM を持たない値なので Send の取り決め自体は守れる見込み＝R1）。
- `emo2_boot/mod.rs:297-306`（DD-12）「placement の実結果は async クロージャへ move 済みで同期参照不能」。実際には準備の結果は `ghost_session.rs` の同期の段で受け取れており、`BootShellChoice` がその隙間を使って運んでいる。

**トレードオフ**:

- ✅ 焼く回数が 1 回になり、時間と読む量が半分になる。警告も自然に 1 行（要件 3 を別の工夫なしに満たす）。焼きの段の `warn!`（0 寸・全透明・上限超え）も 1 行になる。
- ✅ バルーンも共有すれば 4 回 → 2 回。
- ❌ 起動の段取り（採寸 → 窓の配置 → 資産の組み立て）の間で、焼いた物の持ち主が変わる。置き場の取り損ね（準備の後に起動が失敗した・別のゴーストの残り）で大きな絵を World に残さない手当てが要る。
- ❌ `shell-scale-linkage`（C15 がこの着地を待つ）・`balloon-communicatebox`（`emo2_boot/mod.rs`）・余力の枠の `multi-ghost-runtime` と重なる。
- ❌ 窓が生えるまでの時間は変わらない（採寸は今も全部焼いている）。縮むのは窓が生えた後の資産の組み立ての分。

### 案 B: 採寸は全コマ無しで

**B-1（読み手で包む）**: 採寸の側だけで、`ElementDecoder` を包む小さな読み手を使う。

- `probe_animation` は `None` を返す（焼きの段は動く絵の道に入らず、上限の判定も `warn!` も出ない）。
- `decode` は、包んだ読み手の `probe_animation` が見出しを返せば `decode_first_frame`、それが失敗すれば今までの `decode`。見出しが無ければ `decode`。`probe_pna` はそのまま渡す。
- この順は、資産の組み立ての道の縮め方（`decode_frames` が駄目なら `decode_first_frame` → `decode` → 失敗）と同じ段を同じ順で通るので、採寸で残る絵と寸法は、資産で残る 0 番のコマと同じになる（全コマは見出しの寸法と同じ・2.2 節）。
- 置き場所は `placement/` の下の新しいファイル（`areka_emo_atlas` の公開の型だけを使うので examples の `#[path]` の制約に触れない）。`measure.rs` は読み手を作る 1 行を変える（バルーンの面 0 の焼きにも同じ読み手が渡る）。
- `shell_target.rs`・`balloon.rs`・`areka-emo-atlas` は変えない。brief が挙げた `mcp/dump_surface_gpu_test_support.rs` も、`prepare_ghost_windows_with_work_area` の形が変わらないので触らずに済む見込み。

**B-2（上限を渡す入口）**: brief の書き方。`load_shell_target` の兄弟として上限を受ける入口を `shell_target.rs` に、バルーンにも同じ入口を足し、採寸が `AnimationLimits { max_frames: 1, .. }` を渡す。

- ❌ 動く絵 1 つごとに「上限を超えたので 1 枚だけ読みます」の `warn!` が起動のたびに出る（ぱすたなら 3 行）。上限を超えていないのに超えたと書く誤った記録で、要件 3 の向きとも逆になる。
- ❌ `shell_target.rs`（874）・`balloon.rs`（701・`balloon-webp-names` と重なる）に入口を足す。
- B-1 より触る所が大きく、得る物は同じ。

**要件 3 を案 B で満たす形**（B-1・B-2 共通）:

- 採寸の道が読み込みの記録を出さない入口が要る。`load_shell_target` の中の記録は、戻り値に載らない途中の値（重複・桁溢れ・描かない element定義の行・`image_only`）も使うので、「記録を出すかどうか」を受ける 1 本の中身を、記録する入口と記録しない入口が共有する形になる（読み込みの権威を 2 つに分けない）。`shell_target.rs` に 20 行ほど。
- それでも焼きの段の `warn!`（0 寸・全透明）は採寸と資産で 2 行のまま（`areka-emo-atlas` は変えないため）。採寸の側の読み手が記録を抑える手は無い。
- 「採寸の道では記録を出さない」と、いま起動の失敗を採寸が先に記録している道（読めない・面が無い）の `error!` は採寸の側で出し続ける必要がある（要件 2.5・記録の無い失敗の道を作らない）。

**トレードオフ**:

- ✅ 触る所が小さく、v0.1.0 の残りと重なり 0（B-1 で要件 3 を外すなら `shell_target.rs` にも触らない）。
- ✅ 窓が生えるまでの時間が縮む（採寸の分＝動く絵 1 つあたり release で約 1.4〜1.6 秒が、1 枚目を読む約 5〜6 ms になる）。
- ❌ 焼く回数は 2 回のまま（静止画の焼き・面の表の組み立て・バルーンの 4 回は残る）。
- ❌ 要件 3 は追加の入口なしでは満たせない。入口を足しても焼きの段の `warn!` は 2 行に残る。

### 案 C: 組み合わせ（参考）

- **C-1（シェルだけ案 A・バルーンはそのまま）**: シェルの共有だけを入れ、バルーンは今の 4 回を残す。scope の集まりの食い違いを扱わずに済む。要件の表の「バルーン 1」は 2 に変わる。
- **C-2（案 B-1 ＋ 記録しない入口）**: 2 章の数で言えば「シェル 2 回・採寸のコマ 1 枚・警告 1 行（3 種のみ）」。
- **C-3（段を分ける）**: 先に B-1 で採寸の全コマをなくし（窓が生えるまでが縮む）、焼く回数の 1 本化は後の spec に回す。`shell-scale-linkage` の着地を待たずに入れられる。

## 5. 規模と危うさ

| 案 | 規模 | 危うさ | 一行の理由 |
| --- | --- | --- | --- |
| A（シェル＋バルーン） | M（10〜14 タスク） | 中 | 起動の段取りの間で焼いた物の持ち主を変え、明示された「採寸後に捨てる」取り決めを書き換え、scope の集まりの食い違いを扱う |
| C-1（シェルだけ A） | M（8〜11 タスク） | 中 | 運び方は A と同じ。バルーンを外す分だけ小さい |
| B-1（読み手で包む・要件 3 は別に） | S（4〜6 タスク） | 低 | 公開の trait を包むだけで、資産の道と同じ縮め方を通る |
| B-1 ＋ 記録しない入口 | S〜M（5〜8 タスク） | 低〜中 | 読み込みの権威の中の記録の出し方に枝を足す。失敗の記録を落とさない檻が要る |
| B-2（上限を渡す入口） | S〜M（5〜8 タスク） | 中 | 誤った「上限を超えた」記録を起動のたびに出す |

## 6. 設計の段への申し送り

### 6.1 決めが要ること（要件討議へ）

1. **議題 1 そのもの**: 案 A／C-1／B-1／B-1＋記録しない入口／C-3 のどれを取るか。要件 4.1 で固定する数（シェルの焼く回数・バルーンの焼く回数・採寸のコマの数）はこの答えで決まる。C-1 を取るなら要件の表の「案 A のバルーン 1」を 2 に直す。
2. **案 B の形**: B-2（上限を渡す）を取らず B-1（読み手で包む）にしてよいか。B-2 は動く絵 1 つごとに「上限を超えた」の誤った `warn!` を起動のたびに出す。
3. **要件 3 の範囲**: 3 種（脱落・箱・入れ子）だけか、`load_shell_target` が出す他の記録（重複・相手の無いコマ・形の誤り・描かない element定義・透過の宣言・`.pna`）まで含めるか。焼きの段の `warn!`（0 寸・全透明・上限超え）と、面の表を組むたびに出る畳み込みの `warn!` を含めるか。案 B では焼きの段の `warn!` を 1 行にできない。
4. **バルーンの重なる記録**（2.4 節）を本 spec で扱うか: 縮退の `warn!` が最大 3 行・透過の宣言と `.pna` が 2 行ずつ。案 A でバルーンを共有すれば採寸と資産の 2 回分は 1 回になるが、`windowposition` の読みの 1 回は残る。範囲外にするなら、起票するか（開発者の決まり「範囲外の問題は `/kiro-discovery` で起票」）。2 層マージの `info!` 2 行は今の設計が意図して重ねているもので、変えない前提でよいか。
5. **案 A を取るなら**: 採寸の「アセットは持たない・採寸後に捨てる」と `PreparedPlacement` の Send・素の値の取り決め（`measure.rs:20-22`・42-45・`placement/mod.rs:262-264`）を、本 spec の要件で書き換えてよいか。運ぶ置き場を `BootShellChoice` と同じ World の置き場の形にするか、戻り値で運ぶか。
6. **案 A でバルーンも共有するなら**: 採寸の scope の集まり（descript から）と資産の scope の集まり（`derive_scopes()` の `[0, 1]`）の食い違いを、両方に在る scope だけ運ぶ形で扱ってよいか（`derive_scopes` を直すのは本 spec の外の判断）。
7. **檻の置き方**: 「焼いた回数・読んだコマの数」を、採寸と資産の組み立てに読み手を差し込む入口（関数の段の檻）で固定するか、`boot_shell_tests.rs` の形（準備 → 起動を本番の順で通し、記録の行を数える）でも固定するか。後者は動く絵を持つゴーストの検体（`animated-playback` の絵を土台のゴーストへ写す）が要る。`boot_shell_tests.rs` の今の期待値（シェルを 2 回読む）は案 A なら 1 回へ変わる（壊れたテストではなく、仕様が変わったための更新）。

### 6.2 調べ残し（Research Needed）

- **R1**: `ShellTarget`（中の `EmoWorld`＝`bevy_ecs::World`・`BoxLayout`・`CursorBrace` など）が `Send + Sync` で、World の置き場（`Resource`）に置けるか。置けなければ `insert_non_send` で置く（UI スレッドの上なので足りる見込み）。
- **R2**: 案 B-1 で、APNG の既定の絵がアニメーションに含まれない形（`IDAT` が 1 枚目のコマでない）のとき、`decode_first_frame` と WIC の `decode` の寸法が同じか。B-1 は `decode_first_frame` を先に使うので資産の 0 番のコマと同じになる見込みだが、包む読み手のテストで確かめる。
- **R3**: 要件 5.1 の測り直しの形。9.14 節は `load_shell_target` を通さずに焼く入力を作り直していた。「採寸の分が消えた」を示すには `measure_scope_sizes` と `build_boot_assets_with_shell` を前後で測る捨てる測定プログラムが要る（`areka` は bin のクレートなので、examples の `#[path]` の形で `placement` を読むか、`#[ignore]` のテストで測るか）。静かな机は 1 回だけ（要件 5.4）。
- **R4**: emo2・ぱすたのバルーンで、本体側の系列へ縮退する面が実際にあるか（2.4 節の縮退の `warn!` が今の起動で何行出ているか）。実機の記録か、`resolve_balloon_faces` を検体に当てる小さなテストで確かめる。
- **R5**: 案 A で、採寸が組んだ面の表を資産の 1 つ目の scope へ回すには、`ShellTarget` の面の表を借りる口（`take_world` 以外）が要るか。要るなら `shell_target.rs` に触る（触らない形なら、採寸が `take_world` を使い資産が `build_world` を scope の数だけ呼ぶ）。
- **R6**: 新しいファイルを `measure_template_tests.rs` の本文の走査の一覧に足すか（steering `structure.md:186` は兄弟の `<stem>_*.rs` の列挙を求める）。

### 6.3 並走の約束の確かめ

- どの案も `boot_config.rs` と `crates/areka/src/install/` に触らない。依存を足さない（`areka_emo_atlas`・`areka_emo_present` の公開の型だけで足りる）。
- 上限に近いファイル: `placement/mod.rs`（970）には案 A で `mod` の 1 行と呼ぶ所の付け替えだけ、`ghost_session.rs`（940）には案 A で置き場を置く数行だけ。案 B-1 はどちらにも触らない。`measure_tests.rs`（1,000）には足さない。
- 同じ版の相手との重なりは brief の棚卸㉛の節のとおり（案 A＝`shell-scale-linkage`・`balloon-communicatebox`・`multi-ghost-runtime`／案 B-1＝重なり 0／B-1＋記録しない入口＝`shell_target.rs` だけで、`balloon-webp-names` とは重ならない／B-2＝`balloon.rs` で `balloon-webp-names` と重なる）。

### 6.4 要件討議の結論と設計の段への申し送り（2026-10-31）

- **決まったこと**（requirements.md 末尾）: 6.1 の 1・2 は「B-1（読み手で包む）＋記録しない入口」（4 章の C-2）。6.1 の 3・4 は「シェルとバルーンの読み込みの記録すべてを 1 行・焼く段と畳み込みの記録と 2 層マージの `info!` は今のまま」。6.1 の 5・6 は案 A を取らないので要らない。
- **設計の段で決めること**:
  1. 採寸用の読み手の形と置き場所（2 章 B-1・`placement/` の下の新しいファイル）。
  2. 記録を出さない道をどちらにするか（採寸と `windowposition` の読みを黙らせるか、起動の資産の組み立てを黙らせるか）。着替えの道（`switch_assets.rs`）は記録を出し続け、変更 0。起動を止める失敗の記録は黙らせる道の上でも出す（要件 3.6）。シェル（`shell_target.rs`）とバルーン（`balloon.rs`）の読み込みの権威を 2 つに分けず、1 本の中身を記録する入口と記録しない入口で共有する。
  3. 数える読み手と、採寸・資産の組み立てに読み手を差し込む入口。檻を関数の段に置くか、準備 → 起動の通し（`boot_shell_tests.rs` の形）にも置くか。動く絵を持つゴーストの検体は `target\` の下に作る。
  4. `boot_shell_tests.rs` の「シェルを 2 回読む」の期待値（読み込みの `info!` の行で数えている）。記録を出さない道を足すと行の数が焼いた回数を表さなくなるので、焼いた回数は数える読み手の檻へ移し、記録の行は要件 3 の数として固定し直す（仕様が変わったための更新）。
  5. 測り直しの道具（6.2 の R3）。
  6. 調べ残し R2・R4・R6。
