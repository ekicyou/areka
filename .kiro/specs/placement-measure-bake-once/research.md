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

## 7. 設計の段（2026-10-31・`/kiro-spec-design -y`）

> ブランチ `claude/kiro-start-placement-measure-bake-once-80df62`（`a6da785f`）で読み直した。決めた形の正本は `design.md`。ここに置くのは、調べたこと・決めの理由・選ばなかった案・危うさである。

### 7.1 要約

- **調べ方**: 軽い調べ（既存の道の拡張）。新しい依存は無いので外の文書は引いていない。読んだ所: `placement/measure.rs` 全体・`placement/mod.rs` の 420-510・770-870・`placement/config.rs` の `detect_scopes`・`emo2_boot/assets.rs` の 200-540・`emo2_boot/mod.rs` の 285-470・`ghost_session.rs` の 112-230・`shell_target.rs` の 1-60・295-565・755-815・`balloon.rs` の 315-701・`balloon_definitions.rs`・`self_alpha.rs`・`areka-emo-atlas` の `decode.rs`・`animated.rs`・`decode/wic_arm.rs`・`boot_shell_tests.rs`・`measure_template_tests.rs` の走査・検体 `animated-playback`。
- **使った手引き**: スキル `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`・設計の雛形。steering は `structure.md`（テストの兄弟のファイル・本文の走査の列挙・1,000 行）と `logging.md`（ログの無い失敗の道を作らない）。
- **いちばん効いた事実**: 配置の scope の集まりは scope 1 まで（`config.rs:142-148`・`char{n}.`（n≥2）からは足さない）なので、いつも資産の scope の集まり `derive_scopes()` の `[0, 1]` の内に収まる。だから採寸の側を黙らせても、採寸が測る scope の記録は資産の組み立てが必ず 1 行出す。

### 7.2 調べたこと

#### 黙らせる道をどちらにするか（6.4 の 2）

- **案 M（採寸と `windowposition` を黙らせ、資産の組み立てで出す）**: 触るのは `measure.rs` の呼ぶ所 3 つと `placement/mod.rs` の呼ぶ所 1 つ（`use` の行と 803 行目）だけ。資産の組み立て・着替えの道の本文は変えない。記録は全コマで焼いた表示の側の事実になる（焼く段で落ちた絵の `error!` が表示の絵と一致する）。準備を通らずに資産を組むテスト・手本（`spine.rs`・`ghost_switch_test_support.rs`）も今どおり記録を出す。
- **案 S（採寸で出し、資産の組み立てを黙らせる）**: `build_shell_assets`・`build_balloon_assets` は着替えの道（`switch_assets.rs:219`・251）と共有なので、起動の側だけを黙らせる兄弟か引数が `assets.rs` に要る。記録は 1 枚目で焼いた採寸の側の事実になり、表示の絵とずれ得る。窓の前に記録が出る利点はある。
- scope の集まりの食い違い（6.1 の 6）: 案 M で記録が消えるのは「配置にだけ在る scope」、案 S で消えるのは「資産にだけ在る scope」。前者は今のコードでは起きない（上の要約）。後者は `kero.*` の無いゴーストの scope 1 で起きる。→ **案 M を取った**（design D2）。
- 起動の結線の手順 1 が資産の組み立てなので、案 M で記録が出る時点は窓の後・SHIORI の起動の前。準備の後で資産の前に起動が止まると、その起動では読み込みの記録が出ないが、止まった理由の記録は別に出る（採寸の失敗は採寸の道の `error!`）。

#### 「descript が読めない」の範囲（要件 3.2）

- バルーンの基層 `descript.txt` を読む `read_descript_layer` は 2 か所から呼ばれる: 透過の宣言の入口 `load_balloon_use_self_alpha`（採寸と資産で 1 回ずつ）と 2 層マージ `load_scope_balloon_model`（`windowposition` の読みで scope ごと・資産の面ごとの定義 `load_scope_balloon_definitions` で「自分の設定ファイルを持つ面」ごと）。
- 後者は資産の組み立ての 1 回の中でも面の数だけ出るので、起動の重なりを外しても 1 行にはならない。2 層マージは要件 3.7 と Out of scope で「変えない」とした記録の持ち主である。
- → 要件 3.2 の「descript が読めない」は透過の宣言の入口の `warn!` と読む（要件の並び「透過の宣言・`.pna` を使わない・descript が読めない」がこの入口の 3 つの記録と同じ）。2 層マージの中の読みは変えない（design D3）。壊れたバルーンのときだけ出る記録である。

#### 焼く段の記録（要件 3.7）と採寸の読み手

- 採寸の読み手は `probe_animation` を `None` にする（議題 1 の決まり・ギャップ分析 4 章 B-1「焼きの段は動く絵の道に入らず、上限の判定も `warn!` も出ない」）。そのため `areka-emo-atlas` の動く絵の 3 つの `warn!`（上限を超えた・動く絵として読めない・1 枚目を読めない）は採寸の道では出なくなり、資産の組み立ての 1 回だけになる。
- 0 寸・全透明の `warn!` と畳み込みの `warn!` は、採寸が今どおり焼いて面の表を組むので今と同じ数。
- → 要件 3.7 の「変えない」は `areka-emo-atlas`・`areka-emo-compose` の中身と、静止画として焼く段の記録の数を指すと読み、動く絵の 3 つの `warn!` が採寸の道で出なくなるのは議題 1 の読み手の形がそのまま持つ働きとして design D4 に書いた（設計討議で確かめる候補）。

#### 読み手の契約と寸法（gap R2）

- `ElementDecoder` は 5 つの口（`decode`・`probe_pna`・`probe_animation`・`decode_frames`・`decode_first_frame`）。`decode_frames` は見出しどおりの枚数・寸法でなければ焼く段が失敗として縮める（`animated.rs:60-68`）。`decode_first_frame` は `decode_frames` の 0 番のコマと同じ画素・同じ `has_alpha`。
- 資産の側の縮め方（`shrink`）は `decode_first_frame` → `decode` の順。採寸の読み手の `decode` も同じ順なので、採寸が焼く絵の寸法は資産の 0 番のコマと同じ。APNG の既定の絵が動きに含まれない形でも、`decode_first_frame` を先に使うので同じになる（R2 は設計で閉じ、`rgb.apng` の検体の檻で確かめる）。
- 焼いた表の項目は元の寸法（`AtlasEntry::original`）を持つ。1 枚目が全透明の動く絵で外形が変わり得るかは、要件 2.2 の檻（全コマで測った値との一致）で落とす。

#### 数える口（6.4 の 3・4）

- `load_shell_target`・`build_balloon_target_from_faces`・`load_scope_balloon_definitions`・`resolve_box_markers` はすでに `&impl ElementDecoder` を受ける。`&WicDecoderArm` に固まっているのは `measure.rs` の `build_shell_assets`・`measure_balloon_surface0` と、`assets.rs` の `build_shell_assets`・`build_balloon_assets` だけ。後者 2 つを `&impl ElementDecoder` にしても、`switch_assets.rs` は `&WicDecoderArm` を渡したまま組み立てが通る（変更 0）。
- `measure_bake_count_tests.rs` は `measure` の子なので私有の `measure_with_reader`・`measure_baked` を呼べる。`#[cfg(test)]` なので examples の `#[path]` の制約（`crate::` を使えない）にも当たらず、`crate::emo2_boot::assets` を呼べる（`emo2_boot` は `pub mod assets`）。採寸 → 資産を本番と同じ順で 1 つのテストに通せるので、数える読み手は共有の支えに置かず、このファイルの中だけに置く。
- `build_shell_assets`（資産）は `ghost_root` から `resolve_with_shell` でシェルを引くので、動く絵の検体はゴーストの形に写す（`target\` の下）。
- `boot_shell_tests.rs` は読み込みの `info!`（欄 `recognized`・`shell_dir`）で「採寸と資産の 2 回」を数えていた。採寸を黙らせると 1 行になるので、期待値を 1 つへ直し、採寸のシェルは準備が置く `BootShellChoice`（`ghost_session.rs:120`・`pub(crate)`・起こす処理が取り出す前に読める）で確かめる。

#### 本文の走査（gap R6）

- `measure_template_tests.rs` の `the_measure_side_does_not_parse_the_shell_itself` は `measure.rs` の本文だけを見ている。新しい `measure_first_frame.rs` は解析をしないが、steering `structure.md` の「兄弟の `<stem>_*.rs` も走査に列挙する」に合わせて足す（design D6）。

#### 測り直しの道具（gap R3）

- 9.14 節は `areka-emo-atlas` の例に一時に置いた捨てる測定プログラムで、`load_shell_target` を通さずに焼く入力を作り直していた。本 spec で測りたいのは採寸の関数そのものなので、`measure` の子に一時の `#[ignore]` テストを置けば、私有の `measure_baked`（前＝全コマ）と `measure_with_reader`（後）を同じ組み立ての中で交互に回せる（前の木を別に組まずに済む）。絵の作り手は 9.14 節と同じ（`image` の png・webp を持つ `areka-emo-atlas` の側で作る）。どちらもコミットしない。

#### バルーンの縮退の記録（gap R4）

- 通しの檻は、どのバルーンでも必ず出る記録（系列の解決が完了した `info!`・透過の宣言の `info!`）で数えるので、emo2 のバルーンで縮退が起きるかどうかに左右されない。縮退の `warn!` そのものを黙らせる働きは、相方側の系列を持たない検体（`animated-playback` のバルーン）で関数の段に固定する。R4 は設計の段では調べ切らず、要らなくなった。

### 7.3 設計の合成（design-synthesis）

- **一般化**: 要件 3.1 と 3.2 は「読み込みの権威が記録を出す道を選べること」の 2 つの場合。3 つの入口（シェル・系列の解決・透過の宣言）に同じ 1 つの切り替え `LoadRecords` を通す。中で呼ぶ私有の関数（`list_file_names`・`enumerate_file_names`・`read_descript_layer`・`read_use_self_alpha`）も同じ値を受ける。
- **作るか借りるか**: 読み手は `ElementDecoder` を包むだけ（9.14 節の「前」と同じ手）。テストの読み手も `MemoryDecoder`・`WicDecoderArm` を包むだけ。記録の数えは既存の `log_capture_kit`、通しは既存の `SwitchRig`。新しい依存は 0。
- **削ったもの**: 上限を渡す入口（B-2）・焼いた物を運ぶ置き場（案 A）・`build_boot_assets_with_decoder` のような新しい入口（`build_shell_assets`・`build_balloon_assets` を直接呼べば足りる）・共有のテストの支えへの数える読み手（使うのは 1 ファイル）・`placement/mod.rs` の `mod` の行（`measure` の子にした）。

### 7.4 選ばなかった案

| 案 | 何か | 選ばなかった理由 |
|----|------|-----------------|
| 案 S | 採寸で記録を出し、資産の組み立てを黙らせる | 着替えと共有の `build_*_assets` に兄弟か引数が要る・記録が表示の絵の事実でなくなる・`kero.*` の無いゴーストの scope 1 の記録が 0 行になる |
| 記録の口を `bool` で渡す | `emit: bool` | 呼ぶ所で意味が読めない。2 値の列挙にしても大きさは同じ |
| 記録を出さない入口を公開の列挙で受ける 1 本 | `load_shell_target_with(…, LoadRecords)` を `pub` に | 外のクレートに切り替えの型を見せることになる。呼び手が要るのは「出さない」1 通りだけ |
| `read_use_self_alpha` に `_quiet` の兄弟 | 名前を増やす | `pub(crate)` で呼び手は 3 か所だけ。引数を足して 3 か所を直すほうが小さい |

### 7.5 危うさと手当て

- 新しい読み込みの記録を足す人が `emits()` で囲み忘れると、採寸の道で 2 行に戻る → 通しの檻（記録の行の数）が赤になる。
- 配置の scope の集まりが scope 2 以上へ広がると（`extra-character-windows`）、資産の組み立てが読まない scope のバルーンの記録が 0 行になる → design の Revalidation Triggers に書いた。
- 動く絵の 1 枚目が全透明・寸法の契約の外の絵で、採寸の寸法が全コマの値とずれ得る → 要件 2.2 の檻が落とす。
- `balloon-webp-names` が `balloon.rs` に触る → 着地の順で直列（要件の Adjacent expectations のまま）。

### 7.6 設計の検査の結果（design-review-gate）

- 機械の検査: 要件の番号 29 個（1.1〜1.5・2.1〜2.6・3.1〜3.7・4.1〜4.6・5.1〜5.5）がすべて対応表に在る。境界の 4 節・ファイルの計画に空欄・仮置きなし。部品はすべてファイルに対応する（数える読み手は `measure_bake_count_tests.rs` の中）。
- 判断の検査: 1 回目の直し（数える読み手を共有の支えから検査のファイルの中へ移す・資産の側の検体をゴーストの形へ写すことを書く・根拠の無い道具の記述を消す・examples の制約がテストに当たらないことを書く）の後、2 回目で通った。要件の穴として差し戻すものは無い。設計討議で確かめる候補は D3（descript の範囲）と D4（動く絵の 3 つの `warn!`）。


### 7.7 設計討議の結論（2026-10-31・開発者「承認」）

- 設計の検証（design-validation.md）の重要な問題 1・2 を、要件 3 の例外として受け入れた: (1) 1 枚目が全透明の動く絵で、採寸の道に焼く段の「全透明」の `warn!` が 1 行増える（要件 3.7・design D4）。(2) バルーンの 2 層マージの中で descript が読めない `warn!` は 1 行にならない（要件 3.2・design D3）。
- 根拠: どちらも壊れた絵や特別な絵のときだけ出る記録である。(1) を根本で消すには `areka-emo-atlas` の焼く段に記録を出さない口が要り、同じウェーブの `emo-atlas-palette-alpha` と同じクレートに触れる。(2) は意図して重ねている 2 層マージの中に在る。
- 2 件とも、完了の棚卸で起票する「残る重なり」（焼く段・畳み込みの記録）に入れる。(1) は `MemoryDecoder` の検体で檻に入れる。
- 問題 3（バルーンの檻の数え方）は自明な修正として design.md の結合テスト 1 に書き足した（採寸の scope を `[0, 1]` に揃え、期待値をファイルごとの数の和で書く）。

## 8. 実機の確かめ（要件 5.5）

### 8.1 直す前

- 日時: 2026-10-31（タスク 1.1・排他テストの机を持って 1 体 1 回ずつ）。
- ビルド: ブランチ `52163052`（main `da020c10` に文書だけを足したもの・`crates/` の差分 0）の debug 版。比べのために写しを残した: `target\pm\before\areka.exe`（SHA256 `73E0536775C8695292479D72939C4A57AD221FA3EB0F62782A5BF68FB8DDB0DC`）・`target\pm\before\shiori-host32-helper.exe`（SHA256 `75BD8EC6959976179C2AB461772DCCD4897252D3753CED54490BC8326F49100B`）。
- 画面: 主モニタ DISPLAY1 は 2880x1800・dpi=192（200%）・作業領域 `0,0,2880,1704`。副の DISPLAY2 は dpi=144（150%）で、今回は使っていない。
- 9276 の待受: 起こす前の時点で 0 行（SSP のぱすたは起きていない）。areka の MCP は 2 回とも 127.0.0.1:9801 で待ち受けた。
- 根: `target\pm\root`（`doc/pasta-real-run.md` の 3 節と同じ 5 つのフォルダ）。ぱすたの書庫の sha256 は `4d2b3efa9f3cd561b0dc3ec3e140e4fdd7b73551fd88f8ef9ba986ca1c206eb9`。記憶 `target\pm\profile` は空から始め、ぱすた → emo2 の順に同じ記憶で起こした（5.2 も同じ順で回す）。
- 起こし方: `doc/pasta-real-run.md` の 4 節の塊で、exe を `target\pm\before\areka.exe`、`RUST_LOG=info,areka::boot_resolve=debug`、`AREKA_APP_SMOKE_EXIT_MS=180000`（有界の自動終了）にした。`PASTA_DEBUG` は消した。`--ghost` には絶対パスを渡した。記録は `target\pm\logs\{pasta1,emo2a}.stdout.log`。
- 終わり方: 2 回とも自動終了で閉じた（`ghost shutdown sequence completed`・`session_mark_cleared`）。起こした pid 13512 と 28088 は残っていない。
- 撮った面: `target\pm\shots\before\` に置いた（MCP の `dump_surface` の PNG）。

#### ぱすた（窓際のぱすたさん）

| 項目 | 直す前の値 |
|---|---|
| k₀ | `primary_dpi=192`・`k_shell=2.0`・`k_balloon=2.0` |
| 窓の大きさの行（`placement: k₀ 倍後の物理窓寸で窓を生成する`・1 行） | scope 0: `char_size 1108x1368`・`balloon_size 872x900`。scope 1: `char_size 1108x1368`・`balloon_size 872x900` |
| 位置の記録（`merge_scope restore`） | scope 0: `anchor=Free char=(0,336) balloon_off=(80,116) balloon=(80,452)`。scope 1: `anchor=Free char=(0,336) balloon_off=(-872,0) balloon=(-872,336)` |
| バルーンの補正 | scope 1 の起動時関門で `flipped (-872,336)→(1108,336)`。runtime 関門でも同じ補正が 1 行出た |
| 並べ終えた時点の保存 | scope 0 は `char=(0,336) saved=(0,336) char_w=1108` を書いた。scope 1 は台本が置いた窓なので書かない |
| MCP `get_property` の `currentghost.scope(n).rect` | scope 0: `0,336,1108,1704`。scope 1: `0,336,1108,1704` |
| `dump_surface`（いま映っている面） | scope 0 は面 1002（ボールだけ。初回の台本がクリックを待つので身体は映らない）。554x684・`pasta-s0-shown-1002.png`（SHA256 先頭 `41D32521DBA6E343`）。scope 1 は面 10（窓の枠）。554x684・`pasta-s1-shown-10.png`（`F869B7418E05C096`） |
| `dump_surface`（面 1000 を単独で描いたもの） | 554x684・`pasta-s0-surface1000.png`（`BB2642F198E258C4`）。身体・ボール・× が描かれる |
| 「全透明」の `warn!` | 4 行。`pasta/null.png` が 2 行、`balloonk0.png` が 2 行。動く WebP のものは 0 行（記録の中に `webp` の語は 0 回） |
| WARN 以上のほかの行 | `seriko: mustselect` が 64 行。ERROR は 0 行 |

#### emo2（えも？？）

| 項目 | 直す前の値 |
|---|---|
| k₀ | `primary_dpi=192`・`k_shell=2.0`・`k_balloon=2.0` |
| 窓の大きさの行（1 行） | scope 0: `char_size 868x1374`・`balloon_size 800x448`。scope 1: `char_size 672x800`・`balloon_size 576x406` |
| 位置の記録（`merge_scope restore`） | scope 0: `anchor=Bottom char=(2012,330) balloon_off=(-268,-258) balloon=(1744,72)`。scope 1: `anchor=Bottom char=(1340,904) balloon_off=(292,-150) balloon=(1632,754)` |
| windowposition（2 層マージ） | scope 0: `(266,-129)`。scope 1: `(-190,-75)` |
| 並べ終えた時点の保存 | scope 0: `char=(2064,610) saved=(2446,610) char_w=764`。scope 1: `char=(1392,904) saved=(1728,904) char_w=672` |
| MCP `get_property` の `currentghost.scope(n).rect` | scope 0: `2064,610,2828,1704`。scope 1: `1340,904,2012,1704` |
| `dump_surface`（いま映っている面） | scope 0 は面 1000。382x547・`emo2-s0-shown-1000.png`（`189FB6158126CD52`）。scope 1 は面 2105。336x400・`emo2-s1-shown-2105.png`（`D2234D887690130E`） |
| 「全透明」の `warn!` | 0 行 |
| WARN 以上の行 | 2 行。折返し基準が描画範囲の外に解決された件が 1 行と、自動終了の `force_quit` が 1 行。ERROR は 0 行 |

- 5.2 で比べる所: 窓の大きさの行・`merge_scope restore` の値・保存の値・`rect`・面の番号と PNG の寸法は、そのまま一致を見る。PNG のハッシュは動く絵のコマの位相で変わりうる。ハッシュが違うときは、面 1000 を単独で描いたもの（`pasta-s0-surface1000.png`）を基準にして見比べる。
- 注: emo2 の scope 0 の `rect` と保存の `char_x`（2064・幅 764）は、記録の `char=(2012,330)`（幅 868）と違う。これは並べた後の窓の矩形を読んだ値で、直す前の今の動きのまま控えた。

### 8.2 直した後

- 日時: 2026-10-31（タスク 5.2・排他テストの机を持って 1 体 1 回ずつ）。
- ビルド: `ed7349ad` の debug 版。写しを `target\pm\after\` に残した: `areka.exe`（SHA256 `D683F7068FA8D695C7E7633BA496B2C29858EBB811F4A44C7519AEEF2F0F58B8`）・`shiori-host32-helper.exe`（`75BD8EC6…`・直す前と同じ）。
- 条件は 8.1 と同じ。
  - 主モニタは dpi=192。副モニタはこの時 192 だったが、使っていない。
  - 9276 の待受は 0 件。根も同じ。
  - 記憶は `target\pm\profile` と、根の中のゴーストの記憶 `ghost\master\profile` を空に戻し、ぱすた → emo2 の順に起こした。
  - 起こし方も同じで、exe だけを `target\pm\after\areka.exe` に替えた。
- 記録は `target\pm\logs\{pasta2,emo2b}.stdout.log`、撮った面は `target\pm\shots\after\`。
- 2 回とも自動終了で閉じた（`session_mark_cleared`）。起こした pid 3996 と 25560 は残っていない。

#### 前後の比べ

| ゴースト | 項目 | 直す前（8.1） | 直した後 | 同じか |
|---|---|---|---|---|
| ぱすた | k₀ | 2.0 / 2.0（dpi 192） | 2.0 / 2.0（dpi 192） | 同じ |
| ぱすた | 窓の大きさの行 | 1 行。どちらの scope も char 1108x1368・balloon 872x900 | 1 行。同じ値 | 同じ |
| ぱすた | `merge_scope restore` | scope 0: char (0,336)・balloon (80,452)。scope 1: char (0,336)・balloon (-872,336) | 同じ値 | 同じ |
| ぱすた | バルーンの補正 | boot-gate と runtime-gate で、どちらも (-872,336)→(1108,336) | 同じ | 同じ |
| ぱすた | 並べ終えた時点の保存 | scope 0 は (0,336) を書いた。scope 1 は書かない | 同じ | 同じ |
| ぱすた | MCP の rect | どちらの scope も `0,336,1108,1704` | 同じ | 同じ |
| ぱすた | 面 1000 を単独で描いたもの | 554x684・`BB2642F198E258C4` | 554x684・`BB2642F198E258C4` | 同じ |
| ぱすた | scope 1 に映っている面 10 | 554x684・`F869B7418E05C096` | 554x684・`F869B7418E05C096` | 同じ |
| ぱすた | scope 0 に映っている面 1002 | 554x684・`41D32521DBA6E343` | 554x684・`8DE8A60FBE09ED5D` | 寸法は同じ。ハッシュの違いは動くボールのコマの位相だけ（絵ではきらめきの位置だけが違う） |
| ぱすた | 「全透明」の `warn!` | 4 行（`pasta/null.png` が 2 行・`balloonk0.png` が 2 行） | 4 行（同じ内訳） | 同じ |
| ぱすた | WARN・ERROR の数 | 69・0 | 69・0 | 同じ |
| emo2 | k₀ | 2.0 / 2.0 | 2.0 / 2.0 | 同じ |
| emo2 | 窓の大きさの行 | 1 行。scope 0 が 868x1374 / 800x448、scope 1 が 672x800 / 576x406 | 1 行。同じ値 | 同じ |
| emo2 | windowposition | (266,-129)・(-190,-75) | 同じ | 同じ |
| emo2 | `merge_scope restore` | scope 0: char (2012,330)・balloon (1744,72)。scope 1: char (1340,904)・balloon (1632,754) | 同じ値 | 同じ |
| emo2 | 並べ終えた時点の保存 | scope 0: char 2064・saved 2446。scope 1: char 1392・saved 1728 | 同じ | 同じ |
| emo2 | MCP の rect（scope 0） | `2064,610,2828,1704` | `2064,610,2828,1704` | 同じ |
| emo2 | MCP の rect（scope 1） | `1340,904,2012,1704` | `1392,904,2064,1704` | 違う（下の注） |
| emo2 | 映っている面（面 1000・面 2105） | 382x547・`189FB6158126CD52`、336x400・`D2234D887690130E` | ハッシュも同じ | 同じ |
| emo2 | 「全透明」・WARN・ERROR の数 | 0・2・0 | 0・2・0 | 同じ |

- 結論: 寸法・位置・保存・windowposition・記録の行の数は、2 体ともすべて一致した。違いは次の 2 つで、どちらも直しによる違いとは見ていない。
  1. ぱすたの面 1002 のハッシュ。寸法は同じで、違いは動く絵のコマの位相だけ。
  2. emo2 の scope 1 の MCP rect（1340 → 1392）。1.1 は並べ終わる前（記録の保存の行は 07:12:56）に rect を読んだため、restore の値（`char_x=1340`）が返った見込みが高い。保存の記録は前後どちらの走りも `char_x=1392` で一致している。ただし 1.1 は rect を読んだ時刻を記録していないので、この「読んだ時刻の違い」という説明は確かめていない。
- 動く WebP: ぱすたの「全透明」の `warn!` 4 行は、どれも静止画（`pasta/null.png`・`balloonk0.png`）のもので、ぱすたの動く WebP のものは前後とも 0 行（記録に `webp` の語は 0 回）。ぱすたの動く WebP は 1 枚目が全透明ではなく、要件 3.7 の「採寸の道で `warn!` が 1 行増える」場合には当たらない。

## 9. 測り直し（要件 5.1〜5.4）

- 日時: 2026-10-31（タスク 5.1）。コミット `748e70b9` の上で測った。静かな机（排他テストの机と単独の机）を持ったのは測る 1 回だけで、測り終えてすぐ返した（要件 5.4）。
- 機械と道具: 8.1 節と同じ機械（Windows 11 Pro 10.0.26300）・rustc 1.99.1。release はワークスペースの出荷の形。
- 時間は参考の記録で、合否の線にはしない（要件 5.3）。合否は、焼く回数と読んだコマの数の檻（`measure_bake_count_tests.rs`）で決まる。

### 9.1 測り方

- 捨てる測定を、一時のテスト（`#[ignore]`）として置いた。置き場は `crates/areka/src/placement/measure_perf_tmp_tests.rs` で、`measure.rs` から子のテストとして宣言した。測り終えたら、ファイルも宣言も消した（コミットしていない）。
- 測るための絵（`target\placement-measure\` の下・測り終えて消した）。作り手は `gen_animated_samples` と同じ手（`image` の書き手で 1 コマずつ作って包む）。
  - `big.apng`（11,109,652 バイト）と `big.webp`（8,418,166 バイト）。500×500・RGBA 8 ビット・268 コマで、全コマ全面・置き換え。絵柄は完了 `areka-P0-animated-image-decode` の research.md 9.14 節と同じ型にした（上と左に透明の帯・半透明の地・1 画素ずつ動くぼかした円）。
  - `shell_big` は emo2 の写しで、`shell/master/surface0.png` の中身を `big.apng` に替えた（動く絵 1 つ）。
  - `shell_big4` は、さらに `CityPop/surface0010.png`・`purple/0/base1.png`・`purple/0/base2.png` も替えた（動く絵 4 つ）。
  - 9.14 節と同じく、シェルに入れたのは APNG だけ（`big.webp` は作ったが測りには使っていない）。
- 1 回の起動の形: 採寸 → シェルの資産（`build_shell_assets`）→ バルーンの資産（`build_balloon_assets`）。読み手は 1 つの `WicDecoderArm`。scope は `[0, 1]`、バルーンは emo2-kakukaku、既定の面は組み込みの面。
  - 前: `measure_baked(&arm, …)`（全コマを焼く今の採寸）＋資産。
  - 後: `measure_with_reader(&arm, …)`（採寸の読み手で 1 枚目だけ読む）＋資産。
- 時間は `Instant` で測った。「採寸」は採寸の分だけ、「合計」は採寸から資産の組み立てまで。
- 前と後を 1 組ずつ交互に回し、1 組目は捨てた。数えた組は emo2 が 20 組、`shell_big` と `shell_big4` が 5 組ずつ。表には中央値を書いた。
- 回したコマンド: `cargo test --release -p areka --bin areka perf_tmp_measure -- --ignored --nocapture --test-threads=1`（1 回だけ・118.99 秒・緑）。

### 9.2 結果（中央値・ms）

| シェル | 組 | 前の合計 | 後の合計 | 前の採寸 | 後の採寸 |
| --- | --- | --- | --- | --- | --- |
| emo2 そのまま（動く絵 0） | 20 | 197.0 | 199.1 | 100.6 | 99.2 |
| `shell_big`（動く絵 1 つ） | 5 | 2,427.2 | **1,298.2** | 1,203.4 | **102.3** |
| `shell_big4`（動く絵 4 つ） | 5 | 9,184.1 | **4,690.7** | 4,642.8 | **101.1** |

生の数（合計・ms・組の順）:

- emo2 の前: 238, 277, 447, 436, 178, 167, 180, 183, 190, 200, 189, 186, 188, 188, 194, 201, 201, 205, 238, 214
- emo2 の後: 271, 378, 454, 245, 210, 174, 183, 202, 189, 191, 182, 191, 192, 188, 198, 200, 196, 228, 219, 262
- `shell_big` の前: 2476, 2490, 2262, 2355, 2427
- `shell_big` の後: 1354, 1240, 1224, 1306, 1298
- `shell_big4` の前: 9505, 9566, 9048, 9156, 9184
- `shell_big4` の後: 4746, 4691, 4582, 4647, 4863

### 9.3 読み

- **動く絵を全コマ読む採寸の分は消えた**。後の採寸は、動く絵が 1 つでも 4 つでも約 100 ms で、動く絵 0 の emo2 の採寸（99.2 ms）と同じ。前は、動く絵 1 つあたり約 1.1〜1.2 秒が採寸にかかっていた（`shell_big` で 1,203 ms、`shell_big4` で 4,643 ms）。
- 起動の焼く時間（合計）は、`shell_big` で 2,427 → 1,298 ms、`shell_big4` で 9,184 → 4,691 ms になり、どちらもほぼ半分。残るのは資産の組み立てが全コマを 1 回読む分で、それは要件のとおり残す。
- 前と後の差（`shell_big` で約 1,129 ms、`shell_big4` で約 4,493 ms）は、前の採寸と後の採寸の差（約 1,101 ms・約 4,542 ms）とほぼ同じ。採寸の外の時間は変わっていない。
- emo2（動く絵 0）は前後で差がない（合計 197.0 と 199.1・採寸 100.6 と 99.2）。最初の数組は前後とも揺れが大きいが、中央値はほとんど動いていない。動かない絵だけのシェルで遅くならないことが分かる。
- 設計の見込み（`shell_big` 3.2 → 約 1.7 秒、`shell_big4` 13.2 → 約 6.5 秒）より、前も後も速い。前の数も 9.14 節の 3,244 ms・13,155 ms より小さい（コミットと版の違いで遅い所が直ったか、検体の圧縮の違いによる）。前後の比、つまり採寸の分が消えて合計がほぼ半分になるという形は、見込みどおり。
- メモリの山は今回は測っていない（要件 5 に無い）。
