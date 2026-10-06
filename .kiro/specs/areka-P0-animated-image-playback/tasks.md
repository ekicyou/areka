# Implementation Plan

> 用語: 「動く絵」＝動く APNG・動く WebP。「子」＝動く絵 1 つから作る、element を持たず `always` を 1 本だけ持つ部品（design.md の「動く絵の子」・鍵 `PartKey::Film`）。「経過 0」＝時計が無い（コマの欄に何も載っていない）ときに合成が定義から描くコマ。「回数つき」＝繰り返し回数が「合計 N 回」の動く絵、「終わりなし」＝「終わりなし」の動く絵。「出番の世代」＝表示層が外から所有される対象（バルーン）ごとに持ち、隠れていた窓を出すたびに 1 つ進める番号。
>
> 共通の決まり: テストは実装と同じフォルダの兄弟ファイル（`<stem>_*_tests.rs`）に置き、本番ファイルには `#[cfg(test)] #[path = "…"] mod …;` の接続の宣言だけを残す。1 ファイル 1,000 行以下（`looper.rs` が 800 行を超えそうなら、バルーンの面と `refresh` を `looper_stage.rs` へ出す）。記録の無い失敗の経路を作らない。記録は表を作るときだけ（刻みごとの記録 0 本）。時刻は刻みと注入した時計、乱数は `LoopRng` の注入、発行は捕まえるだけの出力で決定論的に回す。`Cargo.toml`・`Cargo.lock` の変更 0・新しい外部クレート 0。
>
> 触らないファイル: `areka-emo-atlas` の全ファイル・`areka-parsers`・`areka-emo-compose` の `fold.rs`・`method.rs`・`atlas_bind.rs`・`boxes.rs`・`hit_import.rs`・`base_image.rs`・`areka-emo-present` の `presenter/show.rs`（`apply_show`）・`cache.rs`（`CAPACITY`）・`crates/areka/src/emo2_boot/` の `spine.rs`・`assets.rs`・`frame.rs`・`frame/wiring.rs`。`mod.rs` の `let clock = TalkClock::new(clock_fn);` の名前・型・持ち主は変えない（`areka-P0-balloon-lifecycle-events` が使う）。`emo2` の照合（焼いた結果・合成の結果・まばたきの決定論テスト）は期待値を 1 本も書き換えない（呼び出しの形の変更に合わせた書き換えは数えない）。

- [x] 1. 着手前の確認と検体
- [x] 1.1 main を取り込み、先に入った `element-base-method` の後の形を引き直して、前の数字を採る
  - `areka-P0-element-base-method` が main に入っていることを確かめ、取り込む（入っていなければ止まって開発者へ知らせる）。`areka-P0-balloon-lifecycle-events` は待たない（2026-10-05 開発者裁定で本 spec が先に入る。`mod.rs` の `clock: TalkClock` の名前・型・持ち主を変えない約束はそのまま）
  - 取り込んだ後のコードで、design.md「触るファイルと並走の重なり」の行（`plan.rs` の `push_static_element_ops`・`flatten_surface`・`flatten_extent`、`mod.rs` の `spawn_seriko(` と `LoopTickerConfig` の所、`hub.rs` の `ShowSurface` の腕、`visibility.rs` の `show_target`）を読み直し、design の前提と食い違う所があれば research.md に書いてから進む
  - 実装の前の HEAD で、`emo2` の 1 コマの時間を `areka-P0-recompose-budget` の測り方で採り、機械と測り方を添えて research.md に記す
  - 完了の姿: 取り込みのコミットがあり、全テストが緑で、research.md に「前の数字」と「引き直した結果（食い違い 0 件、または件数と中身）」が載っている
  - _Requirements: 7.3, 7.7_

- [x] 1.2 試験用のシェルとバルーンの検体を置く
  - `crates/areka-emo-compose/tests/fixtures/animated-playback/` に、シェル（`surfaces.txt` と絵）とバルーン（面の絵）を置く。絵は読み込みの側の検体から `rgb.apng`（終わりなし）・`basic.apng`（合計 2 回・全透明のコマと待ち時間 0 を含む）・`rgb.webp`（終わりなし）・`alpha.webp`（合計 3 回・待ち時間 0 を含む）を写す
  - 中身は少なくとも: element定義に置いた動く絵／`surface<数字>.png` として置いた動く絵／`element0` を持つサーフェスの動く `surface<数字>.png`／同じ絵を置く 2 つのサーフェス／同じ絵を 1 つのサーフェスの 2 か所に置く行／手書きの `always`（一番上と部品）／`bind+always` の行／待ち時間の合計が 0 の手書きの `always`
  - 完了の姿: 検体がリポジトリにあり、シェルは今の読み手と `load_shell_target` で、バルーンの面は `resolve_balloon_faces` で読んで失敗せず、第三者の著作物が 0 件で、ワークスペースの見張りのテストが緑のまま
  - _Requirements: 9.4_

- [x] 2. 合成の側: 子の定義・欄・経過 0・外形
- [x] 2.1 部品の鍵の 2 つ目の種類と、`always` の見分け・経過 0 の求め方を置く
  - `nesting.rs` に `PartKey`（作者のサーフェス／動く絵の子）・`FilmId`・`ElementKind::Film` を足す。作者の欄の読み分け（`element_kind`）は `Film` を返さない
  - `is_always_interval`（`always` の単独・小文字の完全一致）と `rest_index`（待ち時間の累積が 0 の最後の番号）を 1 つずつ置き、合成・見える部品・seriko の表が同じ関数を使える形で公開する
  - `NestReport::from_world` は `ElementKind::Film` を「無い番号」にも「循環」にも数えない
  - 兄弟のテストで: `always` は真、`bind+always`・`always+bind`・`Always`・`always ` は偽／`rest_index` は待ち 0・0・100 で 1、100 始まりで None／`NestReport` が `Film` を数えない（振り分けが `_` の腕なので、コンパイラが漏れを教えない所を檻にする）
  - 完了の姿: 3 つの型と 2 つの関数が公開され、既存の compose のテスト（golden を含む）が期待値を変えずに緑
  - _Requirements: 1.12, 4.8, 4.9_

- [x] 2.2 コマの欄に 3 つの意味と、動く絵の子の欄を持たせる
  - `pattern.rs` に、欄の読み `Cell`（載っていない＝経過 0／サーフェスを指すコマ／絵を指すコマ／消えている）と、`set_blank`・`set_part_blank`・`set_film`・`remove_film`・`cell` と、子の欄・「消えている」の欄を読むだけの走査の口を足す（表示層が回数つきの子の欄を外し、perf の鍵へ混ぜるのに使う）
  - 今の関数（`set`・`remove`・`get`・`iter`・`set_part`・`part_get`・`part`・`clear_parts`・`is_empty`）の署名と `PatternFrame` の欄は変えない。`clear_parts` は子の欄と部品の「消えている」も消す。空の内側の表は持たない
  - 等しさは足した欄も比べる（合成の鍵に自動で入る）
  - 兄弟の `pattern_cell_tests.rs` で: 入れた順に依らず等しい／空の欄は「全部が経過 0」と等しい／「消えている」と「載っていない」が区別される／同じ番号の一番上の欄・部品の欄・子の欄が混ざらない
  - 完了の姿: 4 つの読みが区別され、新しい欄を使わない既存の呼び出しの結果が本 spec の前と同じ
  - _Requirements: 1.10, 4.5, 7.4_

- [x] 2.3 動く絵を子へ分解し、子の定義を面の表に載せる
  - 新しい `film.rs` に `FilmSheet`・`FilmSheets`・`FilmSkip`・`FilmSkipReason` と分解の手順を置き、`EmoWorld::bind_atlas` の中で束縛の次の行に呼ぶ（2 度呼ばれたら `debug_assert!`）。読み口 `film_sheet`・`film_sheets`・`film_skips` を足す
  - 束縛先が動く絵の親である画像の element を見つけ、絵ごとに 1 度だけ検査する（0 番が親自身・コマと待ち時間の数が同じ・全部の原寸が親と同じ・待ち時間の合計が 1 以上）。落ちた絵は分解せず、理由つきで `skipped` に載せる（画像の element のまま＝ 1 枚目の静止画）
  - 通った絵は子の定義を 1 つ作り、置いていた element の種類だけを `ElementKind::Film` に替える（番号・X,Y・描画メソッドはそのまま、束縛は空）。同じ画像は同じ子。経過 0 のコマを分解のときに決めておく
  - 動く絵が 0 で `skipped` も空なら何も載せない。記録はここでは出さない（値で返す）
  - 兄弟の `film_tests.rs` で: 番号・X,Y が保たれる／検査で落ちる 4 通り／`element0` を持つサーフェスの `surface<数字>.png` は分解されない／動く GIF と縮んだ絵は分解されない／同じファイルは同じ子／動く絵が 0 の面の表に何も載らない／1 枚目の待ち時間が 0 の絵の経過 0 は次のコマ／同じ入力から同じ結果
  - 完了の姿: 検体のシェルを読むと、動く絵の element が子を置く element に替わり、`element*` から `film_sheet` が必ず引け、`emo2` の面の表には子の定義が載らない
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.11, 2.5, 3.4, 7.1, 8.2, 8.3_

- [x] 2.4 見える部品に、動く絵の子と `always` の経過 0 の先を数える
  - `SurfaceParts` に `films` と `always_rest` の 2 欄を足し、行を載せる条件にもこの 2 欄を足す（動く絵だけを置いたサーフェスの行が載るように）
  - 見える部品の求め方に、`always` の animation に欄が無ければ経過 0 の先へ進む辺を足す（「消えている」なら進まない）
  - 読み口 `visible_films(top, parts, out)` を足す。`visible_parts` の署名は変えない
  - 兄弟の `nesting_film_tests.rs` で: 見える子（一番上・子・pattern の先）／経過 0 の先が見える部品に入る／「消えている」では入らない／動く絵だけを置いたサーフェスの行が載る
  - 完了の姿: 検体の各サーフェスについて、見える部品と見える子の答えが期待どおりで、動く絵も `always` も無い表の答えが本 spec の前と同じ
  - _Requirements: 1.9, 3.7_

- [x] 2.5 合成で、動く絵の子と `always` の経過 0 を描く
  - `push_static_element_ops` の振り分けに `ElementKind::Film` の腕を足し、`ElementKind::Surface` と同じ場所・同じ順で子を平坦化する。子の平坦化（新しい `plan_always.rs`）は、欄を読んで「絵を指すコマならその絵・載っていなければ経過 0 の絵・消えていれば何も」を 1 枚の命令にし、置いた element の描画メソッドを運ぶ。全透明のコマは命令にしない
  - `flatten_surface` の重ねる対象に、そのサーフェスの `always` の animation を足す。欄が無ければ経過 0 の pattern を今のコマと同じやり方で描き、「消えている」なら描かない。`always` でない animation の扱いと重ねる順は変えない
  - 欄の絵の番号が子のコマに無いときは経過 0 を描いて `debug!`、子の定義が無い `Film` は描かずに `error!`
  - 兄弟の `plan_always_tests.rs` で: 欄が空のとき経過 0 の絵が出る（手書きの一番上・手書きの部品・動く絵の子の 3 通り）／「消えている」では出ない／合計 0 の手書きの `always` は 1 周だけ評価した絵／透明な所から 1 枚目が透けない（コマの絵 1 枚だけが命令になる）／`always` の無いサーフェスの命令列は前と同じバイト
  - 完了の姿: コマの欄が空の `PatternState` で検体のサーフェスを合成すると経過 0 の絵が出て、`emo2` の合成の照合が期待値を変えずに緑
  - _Requirements: 1.4, 3.1, 4.1, 4.5, 4.6, 7.1_

- [x] 2.6 外形の計算を移し、動く絵の子と `always` を数える
  - `compute_extent`・`flatten_extent` を中身を変えずに新しい `plan_extent.rs` へ移し、既存の `plan_extent_tests.rs`・`plan_nesting_extent_tests.rs` の接続先だけを替える（移した時点で全テストが緑であることを先にコミットする）
  - 足すのは 2 か所: 画像の element と子の原寸を同じ 1 行で数える／`always` の animation の全部の pattern の先を、着せ替えの pattern0 と同じやり方で数える。着せ替えの pattern0 を「index が最小」で取る今の食い違いは直さない
  - 兄弟の `plan_extent_film_tests.rs` で: 動く絵を置いたサーフェスの外形が同じ寸法の静止画のときと同じ／動く `surface0.png` だけのサーフェスが 0×0 にならない／手書きの `always` は全部の pattern の和集合／コマの欄を替えても外形は同じ
  - 完了の姿: 移した後の差分が「関数の置き場所と 2 か所の追加」だけで、`emo2` の外形が前と同じ値
  - _Requirements: 1.2, 1.7, 6.2, 7.1_

- [x] 2.7 合成がたどった先と見える部品の答えを突き合わせる
  - 今ある `nesting_fixture_tests.rs` の型で、検体の全サーフェスと、欄の組（空・経過 0 と違うコマ・消えている・子のコマ）について、合成が平坦化でたどったサーフェスと子の集合が `visible_parts`＋`visible_films` の答えと一致することを確かめる
  - 完了の姿: 突き合わせのテストが緑で、片方だけを壊す変異（経過 0 の辺を外す等）で赤になることを 1 度確かめてから戻している
  - _Requirements: 1.9, 3.7_

- [ ] 3. seriko の側: 繰り返しの計算・表・時計・バルーンの面
- [x] 3.1 (P) 経過からコマを決める純粋な計算を置く
  - `LoopFrame` に「絵を指すコマの絵の番号」の欄（`picture`）を足し、seriko の中の書き下しを直す（子の形のテストに要るので、表の仕事より先に置く）
  - `timeline.rs` に `lap_of`（何周目か・周の頭からの経過）と `AlwaysView`・`always_at`（今のコマ）を足す。回数つきで経過が「周期 × N」以上なら最後のコマ。最初の待ちの前は、1 周目なら何も出さず、2 周目以降は前の周の最後のコマ。負の番号のコマに居る間は何も出さない。丸めず、状態も乱数も持たない
  - 兄弟の `timeline_repeat_tests.rs` で: 待ち時間どおりの境目／終わりなしの 2 周目と最初の待ちの間／合計 N 回で最後のコマに止まる／待ち時間 0 のコマを飛ばす（`alpha.webp` の 100・0・70）／経過が 1 秒飛んだら 1 秒ぶん進む（手書きの形と子の形の両方）／途中の `-1` で消えて次で戻る／同じ入力で同じ答え／pattern0 の待ちが 0 の手書きの `always` の周の境目（最後のコマは 0 ミリ秒しか出ない帰結を固定）
  - 完了の姿: 上のテストが全部緑で、既存の `frame_at` のテストが期待値を変えずに緑
  - 合成の側（2.x）に依存しないので、2 と並べて進めてよい
  - _Requirements: 1.5, 1.6, 2.1, 2.2, 2.4, 2.6, 2.9, 4.2, 4.5_
  - _Boundary: Repeat_

- [x] 3.2 seriko の表に `always` と子の行を採る
  - `LoopTrigger::Always { period_ms, laps }` を足し、`is_always_interval` が真の animation を回数なしで採る。待ち時間の合計が 0 のものは採らず、サーフェスの番号・animation の番号・理由を `warn!` で 1 回出す。組み合わせとほかの語は今の `debug!` の腕のまま
  - 作者のサーフェスの輪の後に `film_sheets()` から子 1 つにつき `always` を 1 本採る（周期・回数はファイルの値）。`film_skips()` の 1 件ごとに相対パスと理由を `warn!` で 1 回出す
  - 部品の列の読み口を `PartKey` で引けるようにし、門 `is_continuous()`（動く部品が在る、または `always` を 1 本以上採った）を足す。腕を 2 つと決め打つ所（`from_world` の `k == 0` の検査ほか）と、既存テスト `only_random_and_bindrandom_are_recorded_others_debug_logged`（例の `always` を `runonce` へ）・`recorded_anims_satisfy_postconditions` を直す
  - 兄弟の `table_always_tests.rs`・`table_film_tests.rs` で: 完全一致だけ採る／合計 0 の `warn!` が 1 回／子の行の周期・回数・絵を指すコマ／`film_skips` の `warn!` が原因ごとに 1 回／`emo2` の表は `is_continuous()` が今と同じ値
  - 完了の姿: 検体の表に手書きの `always` と子の行が載り、記録が表を作るときだけ原因ごとに 1 回出て、`emo2` の表の中身が前と同じ
  - _Depends: 2.1, 2.3_
  - _Requirements: 2.5, 4.1, 4.6, 4.8, 4.9, 8.1, 8.3, 8.4_

- [x] 3.3 部品の時計に `always` と回数つきの決まりを入れる
  - 先に、時計の入れ物の鍵を（スコープ, 面の種類）、中の鍵を（`PartKey`, animation の番号）に替えるだけの変更を、既存テストが緑のままでコミットする。評価する部品は見える部品＋見える子
  - 着せ替えの番人 `gate` を 3 通り（抽選・`always`・対象外）にする。`always` は乱数を引かず、時計が無ければ渡された時刻で作り、`always_at` の答えを欄へ書く（経過 0 と同じなら載せない・絵を指すなら `set_film`・何も出さないなら、経過 0 のコマが在るときだけ「消えている」）
  - 回数つきの時計は、評価のとき見えていなければ捨てる。入れ物 1 つの回数つきの時計を全部捨てる口（面が隠れたとき用）を `parts.rs` に置く。この口を呼ぶのは後のタスク（`\s[-1]` は 3.4、`\b[-1]` は 3.6、窓が閉じた知らせと新しい出番は 5.1）。終わりなしの時計は捨てない。抽選の animation の決まりは変えない。生まれた・捨てたは `debug!`
  - 兄弟の `parts_film_tests.rs`・`parts_always_tests.rs` で: 見えた時刻で時計が生まれる／同じ絵を置く別のサーフェスへ替えても続き／置いていないサーフェスへ行って戻っても進んでいる／回数つきは見えなくなると捨てられ戻ると頭から／続けて見えていれば始め直さず、止まっていれば止まったまま／スコープごと・面の種類ごとに別／子では巻き戻らない／経過 0 と同じコマは欄に載らない
  - 完了の姿: 上のテストが緑で、既存の部品の時計のテスト（`looper_parts_emo2_tests.rs` を含む）が期待値を変えずに緑
  - _Requirements: 2.3, 2.7, 2.8, 3.1, 3.2, 3.3, 3.4, 3.5, 4.4, 6.6, 7.4_

- [x] 3.4 一番上の `always`・出来事の直後の評価・表の差し替えを進行に入れる
  - 進行の面ごとの処理に、一番上の `always` の再生（無ければ作り、`always_at` で欄を置き、末尾でも負の番号でも捨てない）を足す。抽選の輪は変えず、`Always` は乱数を引く前に飛ばす
  - 飛ばす条件に「一番上に `always` が無い」を足し、`is_continuous()` が偽の表では今と同じ行だけを通す
  - `refresh_parts` を `refresh(scope, slot, at_ms)` へ広げ、面の切り替え・着せ替えの変化の直後に呼ぶ（見えている `always` の時計を `at_ms` で作り、回数つきの見えなくなった時計を捨て、欄を作り直して `commit_pattern`）
  - 一番上が隠れた（`\s[-1]`）ときは、3.3 の口でシェルの面の回数つきの時計を捨てる。表の差し替えでは、その面の種類の時計を捨てる。出来事の時刻は刻みの単調性の番人（`last_seen`）に入れない
  - 兄弟の `looper_always_tests.rs` で: 一番上の `always` が抽選を待たず始まり、切り替えで頭から／`always` を足しても同じ乱数の列で `random` の発火の時刻が変わらない／`\s[A]`→`\s[B]`→`\s[A]` が刻みの間に続いても回数つきが頭から／表の差し替えで捨てる
  - 完了の姿: 上のテストが緑で、`spine_seriko_loop_tests.rs` ほか既存の seriko の決定論テストが期待値を変えずに緑
  - _Requirements: 3.6, 4.1, 4.3, 4.7, 7.2_

- [x] 3.5 時計つきの起動で、出来事の時刻から時計を始める
  - `SerikoClock` と `spawn_seriko_clocked` を足し、今の `spawn_seriko` は署名を変えずに時計なしでそこへ委ねる
  - 台本の合図（`\s`・`\b`・着せ替え）を処理するとき時計を読み、その時刻で `refresh` する。時計が無ければ直前の刻みの時刻を使う。経過は「今の時刻 − 開始の時刻」を 0 で止めて求める
  - 兄弟のテストで: 時計を注入すると、時計の開始が合図を処理した時刻になる（刻みの時刻ではない）／刻みの時刻が開始より僅かに前でも経過が負にならない／時計なしの起動は今と同じ指令の列
  - 完了の姿: 時計つき・時計なしの両方の起動で上のテストが緑で、`spine.rs` に変更が 0
  - _Requirements: 1.6, 2.9, 3.1_

- [ ] 3.6 バルーンの面でも部品と `always` を回す
  - 知らせの値 `StageNote::Balloon { scope, open, face, generation }` を定義する（運ぶ口と世代の扱いは 5.1）
  - `ScopeStates` にバルーンの窓（開いているか）と面の番号の覚えを足し、`note_stage`・`stage_slots` を置く。`\b[-1]` のときは 3.3 の口でバルーンの面の回数つきの時計を捨てる。バルーンの面の番号は、`\b[番号]` を受けていればその番号、受けていなければ知らせの面（古い知らせで上書きしない）
  - 進行の対象を `stage_slots` にし、部品の欄の作り直しを面の種類を問わず行う。窓が閉じている間は評価しても時計を作らない
  - `commit_pattern` のバルーンの腕は同じ面の番号を使う。`shown_slots`・`apply`・`apply_balloon`・着せ替えの決まりは変えない
  - 兄弟の `state_stage_tests.rs`・`looper_balloon_tests.rs` で: `\b` を受けていないスコープは知らせの面を使い、受けた後は使わない／窓が開いている面で子の時計が生まれる／閉じている間は時計が生まれず、終わりなしの時計だけが進む／シェルとバルーンの時計が混ざらない
  - 完了の姿: 上のテストが緑で、`emo2` のバルーンの指令の列が前と同じ
  - _Requirements: 6.1, 6.3, 6.6, 6.7_

- [ ] 4. 表示層: 隠れている間の先送りと出番の世代
- [ ] 4.1 (P) perf の記録の鍵に、部品の欄・子の欄・「消えている」を混ぜる
  - `timing.rs` の `compose_key_hash` が、一番上の欄に加えて部品の欄・子の欄・「消えている」も混ぜる（2.2 の走査の口を使う）
  - `timing_tests.rs` の鍵の檻に、部品の欄だけ・子の欄だけ・「消えている」だけが違う 3 通りの弁別を足す
  - 完了の姿: 3 通りで鍵が違い、一番上の欄だけの `PatternState` の鍵が前と同じ値
  - seriko の側（3）とは触るクレートが違い、並べて進めてよい
  - _Depends: 2.2_
  - _Requirements: 7.5_
  - _Boundary: 表示層の perf の記録_

- [ ] 4.2 隠れているバルーンへの指令はコマを預かるだけにする
  - `PresentTarget` に「出すときに使うコマ」の欄を 1 つ足す。`hub.rs` の `ShowSurface` の腕で、外から所有され・見えておらず・面が確立済みで番号が同じ・着せ替えが同じ、の 4 つが揃えば欄に置いて成功の応答を 1 回返して戻る（`apply_show` を呼ばない）。1 つでも偽なら今までどおり通し、成立したら欄を空にする
  - `show_target` は「預かったコマ、無ければ `last_show` のコマ」から回数つきの子の欄を外して通し直す（外す関数は `visibility.rs` に 1 つ置く）。成立したら欄を空にし、失敗したら残す。対象の差し替え・`Hide` の後の別の面の確立では欄を空にする。`last_show` の意味と `apply_show` は変えない
  - 兄弟の `presenter_film_tests.rs` で: 隠れている対象へコマだけ違う指令を 3 回送ると合成 0 回で、`show_target` の後の絵が最後の指令のコマ／その後も `last_shown` が成立済みの絵を返し `read_back` が失敗しない／回数つきが止まった状態で隠し、戻しの指令なしで出すと最初の絵が経過 0（終わりなしの欄は外れない）／先送りの縁（面の番号が違う・着せ替えが違う・見えている・シェル）は今までどおり合成される／応答が必ず 1 回／コマを替えても文字のスロットと対象の寸法が同じ／透ける形が表示中のコマに従い、作者の当たり判定の矩形は同じ／静止画だけのバルーンの合成の回数が前と同じ
  - 完了の姿: 上のテストが緑で、`show.rs` の差分が 0
  - 合成の側の欄と子の定義（2.2・2.3）だけに依存する（seriko の側には依存しない）
  - _Depends: 2.2, 2.3_
  - _Requirements: 1.8, 1.13, 2.3, 3.3, 3.7, 6.2, 6.4, 6.5_

- [ ] 4.3 出番の世代と、合図が追い付くまでの欄の外し
  - `PresentTarget` に出番の世代と追い付いた世代を足す（どちらも 0 始まり・対象の差し替えで引き継ぐ）。`show_target` が見えていなかった外から所有される対象を見えるようにしたとき出番の世代を 1 つ進める
  - `PresentCommand` に `StageAck { target, generation }` を 1 種足す。腕は追い付いた世代を大きい方にするだけ（応答なし）。出番の世代より大きい番号・未装着の対象は `debug!` で捨てる
  - `ShowSurface` の腕で、預かるかの判定の前に、外から所有され・追い付いた世代が出番の世代より小さいなら、指令のコマから回数つきの子の欄を外す（4.2 と同じ関数）。指令そのものは捨てない
  - 読み口 `stage_generation(target)` を `read.rs` に足す
  - 兄弟のテストで: 出すたびに世代が 1 つ進み、見えている対象への `show_target` では進まない／大きすぎる合図・未装着の合図は無視／追い付いた後の指令はそのまま通る
  - 完了の姿: 上のテストが緑で、シェルの対象への指令の扱いが前と同じ（世代を持たない）
  - _Requirements: 2.3_

- [ ] 4.4 閉じる → 出すの競り合いを、着く順の全部の並びで固定する
  - 兄弟の `presenter_stage_tests.rs` で、回数つきの絵がコマ k に居る状態から「隠す → `show_target`」を行い、①古い指令（コマ k）②閉じた知らせで出た戻しの指令（経過 0）③`StageAck` ④新しい指令（コマ 1）を、③が④より前という制約の下で取りうる全部の並び（①②は③の前後どこでも・0〜2 件）で流す
  - どの並びでも「④が着くまでの全部の合成が経過 0 の絵」「④の後はコマ 1」になること、終わりなしの絵のコマが 1 度も外されないこと、古い世代の合図・出番より大きい合図が無視されること、面の番号を替える古い指令が捨てられずに効くことを確かめる
  - 完了の姿: 並びを数え上げるテストが緑で、並びの数がテストの中で明示の値として確かめられている
  - _Requirements: 2.3, 3.3_

- [ ] 5. 窓の知らせと結線（seriko・表示層・`areka` をつなぐ）
- [ ] 5.1 seriko が窓の知らせを受け、新しい出番では合図を先に出し、合図を表示層へ写す
  - 統合のタスク（seriko の `actor.rs`・`output.rs` と、`areka` の `adapter.rs` を同じコミットで触る。`map_display_command` は網羅の `match` なので、`DisplayCommand` に腕を足した時点で写す腕が無いとビルドが止まる）
  - `SerikoMsg::Stage { note, at_ms }`・`SerikoSink::send_stage` を足す（受け手が消えた後は `debug!`）。`output.rs` の `DisplayCommand` に `StageAck { scope, generation }` を 1 種だけ足す。今ある 5 種の欄は変えない
  - `Stage` を受けたら順に: 世代が覚えている値より新しければ `StageAck` を先に 1 件出し、3.3 の口でそのスコープのバルーンの面の回数つきの時計を全部捨てる → `note_stage` → `refresh(scope, Slot::Balloon, at_ms)` → 返った指令を出す。どれも今の単一の発行点から出す。閉じた知らせでも回数つきの時計を捨てる
  - `adapter.rs` の `map_display_command` に、`DisplayCommand::StageAck` を `PresentCommand::StageAck`（バルーンの対象）へ写す腕を 1 つ足す
  - 兄弟の `actor_stage_tests.rs` で: 世代が進んだ知らせで出力が「`StageAck` → （在れば）`ShowBalloon`」の順／閉じた知らせを受けずに世代だけ進んでも、回数つきの時計が捨てられ知らせの時刻で作り直される／同じ世代では合図が出ない／知らせだけでは `HideBalloon`・`Hide` を出さない／知らせの無い流れ（`emo2`・シェルだけ）では `StageAck` が 0 件。`adapter.rs` の兄弟のテストで `StageAck` がバルーンの対象へ写る
  - 完了の姿: ワークスペースがビルドされ上のテストが緑で、出口が 1 つのまま（`emit_display` 以外から指令を出す行が 0）
  - _Depends: 3.6, 4.3_
  - _Requirements: 2.3, 6.1, 6.3, 6.7_

- [ ] 5.2 時計を 1 つ作って刻みと seriko へ渡す
  - `mod.rs` で新しい時計 `seriko_clock` を 1 つ作り、刻みの起動（`LoopTickerConfig::clock`）と `spawn_seriko_clocked` の両方へ同じものを渡す。`let clock = TalkClock::new(clock_fn);` は名前・型・持ち主を変えない
  - 完了の姿: `areka` がビルドされ、`spine.rs` の差分が 0 で、起動の既存テストが緑。刻みと seriko が同じ時計の値を読むことを兄弟のテスト 1 本で確かめる
  - _Depends: 3.5_
  - _Requirements: 1.6, 2.9_

- [ ] 5.3 届けの相から、窓の見える・見えないと出番の世代を seriko へ知らせる
  - `run_status_report_phase` の中、文字の層を借りる手前で、装着済みのバルーンのスコープ（昇順）ごとに `target_visible`・`current_surface_id`・`stage_generation` を読み、前に知らせた組と違うときだけ `send_stage` を呼ぶ
  - `BalloonStatusLedger` に「スコープ → 前に知らせた組」の欄を足す。置き場のゴーストが居ないフレームは台帳を変えずに見送る
  - 兄弟の `status_report_stage_tests.rs` で: 前と同じなら送らない／文字の層を借りられないフレームでも送る／開いたままでも世代が進んでいれば送る（同じフレームの中の隠して出し直し）／ゴーストが居ないフレームは台帳が変わらない
  - 完了の姿: 観測の列から送る知らせの列が期待どおりで、`emo2` の届けの相の既存テストが緑
  - _Depends: 4.3, 5.1_
  - _Requirements: 2.3, 6.1_

- [ ] 5.4 本物の読み手から合成まで、シェルとバルーンを 2 形式で通す
  - 兄弟の `film_playback_e2e_tests.rs` で、検体のシェルを `load_shell_target` で読み、`build_world` → `AnimationTable::from_world` → 時計つきの seriko（出力は捕まえるだけ）→ `\s[0]` → 刻みと進め、出た `PatternState` を合成して、決め手の画素が検体のコマと一致することを APNG と動く WebP の両方で確かめる。最初の指令の絵が経過 0 のコマであることも確かめる
  - バルーンは `resolve_balloon_faces` → `build_balloon_target_from_faces` で同じことをする
  - `map_display_command` を通して、閉じる → 出すの競り合いを 1 本踏む（指令の道が順を保つことの固定）。隠れているバルーンで `dump_balloon` が失敗しないことを 1 本確かめる
  - 完了の姿: E2E が 2 形式・シェルとバルーンの 4 通りで緑
  - _Requirements: 1.1, 1.2, 2.3, 6.1, 6.7, 9.1_

- [ ] 6. 確かめ
- [ ] 6.1 合成し直す回数を数えるテストで、動かないシェルが変わらないことを固定する
  - 動く絵も `always` も無い表（`emo2` の表を含む）で、同じ刻みと乱数の列に対して出る指令の列が今の期待値のまま（既存の `spine_seriko_loop_tests.rs`・`looper_parts_emo2_tests.rs` を書き換えずに通す）
  - 動く絵 1 つの表で、待ち時間 100 ミリ秒のコマに 16 ミリ秒の刻みを 7 回与えて指令が 1 件だけ／面の表示の指令の直後の最初の刻みで指令が 0 件
  - 全テストを回し、時計・Windows の拡張機能・ネットワークに依らないこと（注入の時計と乱数だけ）を確かめる
  - 完了の姿: 上のテストを含むワークスペースの全テストが緑で、`emo2` の照合の期待値の差分が 0
  - _Requirements: 7.1, 7.2, 7.4, 7.7, 9.2, 9.3_

- [ ] 6.2 1 コマの時間と合成の回数を測って残す
  - `emo2` の 1 コマの時間を、1.1 と同じ機械・同じ測り方で採り、前の数字と並べる
  - 検体の動く絵 1 つを表示しているシェルで、合成し直す回数と 1 コマの時間を測る（コマが 3 枚以下と 4 枚以上の両方）。16 ミリ秒に収まらなければ数字と対処を開発者へ報告する
  - 合成の結果を覚える席の数は変えない（変更 0 と記す）
  - 完了の姿: research.md に前後の数字・動く絵の数字・席の数の変更 0 が載っている
  - _Requirements: 7.3, 7.5, 7.6_

- [ ] 6.3 実機で 2 形式を確かめる
  - 検体のシェルとバルーンを `emo2` の写しに足したゴーストを、ワークツリーの `target\` の下に作って起こす（絶対パス・`RUST_LOG` は判定の分かれ目の `debug!` まで開ける・有界の自動終了）
  - APNG と動く WebP のそれぞれで、自動アニメーション・`always`・サーフェスを切り替えても途切れないこと・バルーンの面が動くこと・隠れている間に合成が走らないこと・バルーンが出た瞬間のコマを、記録と `mcp-dump-images` の読み戻しで確かめる。`emo2` そのままの見た目が変わらないことも確かめる
  - 実機で areka の未対応のためにうまくいかなかった件は、範囲外でもすべて `/kiro-discovery` で起票する
  - 完了の姿: research.md に 2 形式それぞれの結果（記録の抜き書きと読み戻しの絵の確かめ）が載っている
  - _Requirements: 9.5, 10.6_

- [ ] 7. 記録と申し送り
- [ ] 7.1 (P) 網羅台帳と対応表を、着地した振る舞いに合わせる
  - `assets.toml` の `always` の項を「単独の `always` は駆動する。組み合わせは駆動しない」へ、`element*` の項に自動アニメーションが動くこと・動く GIF は非対応のまま・`--clipping` を付けても動く絵として読むこと（`areka-P0-element-clipping-option` が着地するまで）を書く。`import` の項は触らない
  - `doc/COMPAT_ARCHITECTURE.md` §8 に 1 節: design.md「文書の更新」に挙げた全項（繰り返し回数と始め直し・待ち時間・巻き戻さない・途中の終わりのコマと合計 0・経過 0 と外形・バルーンの面と完了 spec の上書き・16 ミリ秒の刻み・pattern0 の待ち 0・スコープごとの記録・出したときの 1 枚目）
  - 完了の姿: 台帳の検査（`ukadoc-survey` ほか）が緑で、§8 の節に design.md の全項が載っている
  - _Requirements: 10.1, 10.2, 10.3, 10.4_
  - _Boundary: 文書_

- [ ] 7.2 後続 3 本の申し送りと roadmap を実物と照らす
  - `areka-P0-seriko-trigger-intervals`・`areka-P0-animated-image-import`・`areka-P0-extent-element-offset` の brief の申し送りの節を、実装した型・関数・ファイルの名前と照らして書き直す
  - roadmap の `seriko-trigger-intervals` の行が「優先（高）」で `always` を含む組み合わせが載っていること、本 spec の行の触るファイルと規模が実物と合うことを確かめる
  - 実装の途中で見つけた範囲外の問題がすべて `/kiro-discovery` で起票され roadmap に載っていることを確かめる
  - 完了の姿: 3 本の brief の申し送りと実物の食い違いが 0 件で、`import` の扱いの差分が 0（読み手・`method.rs` に変更なし）
  - _Requirements: 5.1, 5.2, 10.5, 10.6, 10.7_

## Implementation Notes
- 1.2: 検体は `tests/fixtures/animated-playback/{shell,balloon}/`。面とバルーンの面は `.png` の名前しか拾わないので、`surface1.png`＝`basic.apng`・`surface2.png`／`balloons0.png`＝`rgb.apng`・`balloons1.png`＝`alpha.webp` の中身の写し。`rgb.*` は α を持たず左上の白が抜き色になる＝決め手の画素は写し元 `crates/areka-emo-atlas/src/testdata/animated/` の README の表を正本に選ぶ（5.4）
- 並走（10-06 調べ）: `areka-P0-self-alpha-declaration`（セッションなし・tasks 生成済み）は `build_balloon_target_from_faces` に値を 1 つ足し、`build_shell_target*` の直呼びに `UseSelfAlpha::On` を足す。後から main に入る側が、1.2 の `shell_target_animated_fixture_tests.rs` と 5.4 の E2E の呼び出しを合わせる。`mcp-get-status` は `frame/status_report.rs` を読むだけの見込み（問い合わせ中）
- 7.1: 台帳を書き替えたら `doc/ukadoc-coverage/report/summary.md` は手で直さず `ukadoc-survey` の report と report-summary を回し直して作る（並走の choice-script-prefix・element-base-method も同じ表を作り直すので、後から入る側が回し直す）
- 1.1（範囲外・完了時に起票）: `emo2` の 1 コマの時間が 8 月の draw-load-parity（p50 2.8 ms・p95 26.5 ms）より桁で遅い（10-06 main で p50 69 ms・p95 605 ms・catch-up 96→252）。差のほとんどが `show.rs` の最後の `mark(MaskGen)` から `emit` までの、どの段にも入らない区間。本 spec の前からの問題。6.2 の前後比較は `perf-loop.ps1 prepare-ab`／`measure-ab` の交互取得を必須にする
- 2.1: `FilmId` は `nesting.rs` に置いた（2.3 で `film.rs` を作っても 2 つ目を作らない）。`plan.rs` の `ElementKind::Film(_)` は網羅のための仮の腕（全部を `debug!` で飛ばす）＝2.3 から 2.5 の間は本番で届く。2.5 で design の誤りの表どおり（子の定義が無いときだけ `error!`）に置き換え、そのレビューで確かめる
- 2.2: 走査の口は `cells()`（一番上も含む「載っていない」以外の全部）。4.1 は `iter()` を先に混ぜ、一番上でない欄と一番上の「消えている」を在るときだけ足して、一番上だけの鍵を前と同じ値に保つ。子の欄は「消えている」を持てない（`films: BTreeMap<FilmId,u32>`）＝3.3 で子の `always_at` が「何も出さない」を返す場面があれば欄の形を見直す
- 2.3: `FilmId` は親の `ElementId` の値。経過 0 は `rest_index(once(0).chain(delays[..n-1]))`（pattern i の待ち＝コマ i−1 の待ち時間）。`surface<数字>.png` は `apply_base_images` が足す層 0 の画像の element として同じ分解に乗る。2.5 までは検体の子は `plan.rs` の仮の腕で描かれない
- 範囲外（完了時に確かめる）: `cargo clippy -p areka-emo-compose --all-targets -- -D warnings` が既存のテスト 5 か所の `chunks_exact`（`blit_transparent_alpha_tests.rs`・`golden_tests_surface1000_bind_tests.rs`・`golden_tests_test_support.rs`・`composer_tests.rs` ×2）で赤。ツールチェーンの新しい警告。`clippy-199-lints` の持ち物か確かめて起票
- 2.4: 経過 0 の pattern の求め方 `rest_pattern(anim)` は `nesting.rs` に置いた（`pub(crate)`・index の昇順に並べて `rest_index`）。2.5 の `plan_always.rs` はこれを呼び 2 つ目を作らない。seriko（3.2）は `table.rs` の同じ並べ方の待ち時間に公開の `rest_index` を当てる＝並べ方を変えるなら両方を揃える。`always_rest` の先はまだ `has_animated_parts` に数えない（3.2 の `is_continuous()` の仕事）
- 2.5: 仮の `Film` の腕は `plan_always::push_film_op` に置き換えた（子の定義が無いときだけ `error!`）。経過 0 の pattern の描画メソッドが動かないときは `debug!`（周ごとに巡るので `warn!` だと刻みごとの記録になる）。動く絵だけのサーフェス（検体の 1 番）は 2.6 まで外形 0×0＝`build_plan` が `EmptyComposition`
- 2.6: `plan_extent.rs` は `plan` の子のモジュール（`#[path]`・`pub(crate) use extent::compute_extent`）。移動は `d7755372`。`always` の外形は負の番号・7 語・`move`（ukadoc「サーフェスIDは無視される」）を数えない。見える部品（2.4）は `is_implemented_name` で `move` を既に外している
- 3.1: `current_frame_index` は `timeline.rs`（`pub(crate)`）へ移し、`frame_at` もこれを通る（Repeat は末端・`looper` を参照しない）。`always_at(…, 0)` は合成の `rest_index` と一致（テストで固定）＝3.3 の「経過 0 と同じなら欄に載せない」の前提。絵を指すコマは `Nothing` にならない。seriko の clippy の赤 2 件（`actor.rs` の large_enum_variant・`looper.rs` の collapsible_if）と `dola` の 21 件は前からのもの（完了時に起票の対象）
- 3.2: 子の行は animation 0・`LoopFrame { surface_id: -1, picture: Some(ElementId の値) }`（`FilmSheet.frames` の値をそのまま）＝3.3 は `set_film(film, frame.picture)` と書けば合成の `push_film_op` と一致する。`from_world` は `from_world_and_films(world, sheets, skips)` へ委ねる（seriko は atlas に依らないのでテストは手で組んだ子の定義）。`has_animated_parts` は `always_rest` の先と子の行を持つサーフェスも数える。looper の抽選は `Always` を乱数の前で飛ばし、parts の `gate` は `Always` に `None`（再生は 3.3・3.4）
- 3.3: 鍵の付け替えは `59b03474`。`PartClocks::drop_finite(scope, slot, table)` は本番で未使用のため `#[cfg_attr(not(test), allow(dead_code))]`＝3.4（`\s[-1]`）・3.6（`\b[-1]`）・5.1 で呼んだら外す。`peek`（`refresh_parts`）は `always` の時計を作らない＝出来事の時刻で作るのは 3.4 の `refresh`。`rebuild` が見えている部品の「消えている」を落としていた不具合を直した
- 範囲外（完了時に確かめる）: `rebuild` は `clear_parts` 直後の 1 回目に、経過 0 の辺・着せ替えの辺の先の部品も評価するので、外側が別のコマに居る刻みでも見えていない部品の抽選が回ることがある（`surface-element-nesting` からの性質・引く乱数の数は本 spec の前と同じ）
- 3.4: `refresh_parts` は `LoopRuntime::refresh(scope, slot, at_ms: Option<u64>, states)`（`None` なら直前の刻みの時刻・`last_seen` に入れない）。`\s`・着せ替え・`\b` の 3 か所から呼ぶ。`\b[-1]` → `drop_finite(Balloon)` の道は既に通した＝3.6 で 2 本目を作らない。`drop_finite` の `allow(dead_code)` は外した。表の差し替えは `clear(slot)` でその面の種類の時計だけ捨てる。バルーンの面の部品の欄の作り直しはまだシェルの面だけ（3.6 で外す）
- 3.5: `pub type SerikoClock = Arc<dyn Fn() -> u64 + Send + Sync>`・`spawn_seriko_clocked(..., Option<SerikoClock>)`。時計は `LoopRuntime` が持ち（`with_clock`・`event_ms()`）、合図を処理するたびに読む。5.2 は同じ `Arc` を `Box::new(move || MonotonicMs(c()))` と包んで `LoopTickerConfig::clock` へ渡せば単位が揃う
