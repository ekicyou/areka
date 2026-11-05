# ギャップ分析: audit-emo-compose（面の合成の見直し）

- 測った時点: `main` `88eb0a43`（枝 `claude/kiro-start-audit-emo-compose-cb01ad`・`c2779c8f`）・2026-11-05
- 物差し: `/ponytail:ponytail-audit`（ponytail 5.3.2）。作法のうち 2 つを外した（要件 1.2・1.3）。
  - 「最大 20 件」を外した。指摘は全件を下に並べ、省いた件は無い。
  - 「読まなかった所（Not checked）」を残さない。範囲の 97 ファイルを全部読んだ（下の「読んだ範囲」）。
- 番号は audit の番号（組をまたいで通し番号）。組は audit の 3 つ（必ず直す・直したほうがよい・できれば）のまま。
- この文書はコードを変えていない。決めごとは要件の討議に回す。

## 1. 要約

- **何をするクレートか**: シェルの `surfaces.txt` を読んだ結果（`Shell`）と焼いた絵の表（`AtlasTable`）から、面の 1 枚絵（premultiplied BGRA の `ComposedSurface`）を作る。畳み込み（fold）→ 命令列（plan）→ 画素の転写（execute）の 3 段。ほかに当たり判定の領域引き・入れ子の表・箱（シェル内バルーン）の表・表示の拡大率の数学を持つ。
- **想定した負荷**: 1 人の利用者の 1 台の机で、インストールしたゴーストのシェルを UI スレッドの上で毎フレーム組む。シェルのファイルはネットから入れたもので、**信用できない入力**として扱った（壊れた・意地の悪いシェルでアプリが落ちたり固まったりしないこと）。
- **指摘の数**: 必ず直す 5 件・直したほうがよい 8 件・できれば 7 件＝**20 件**（テストの指摘は種類ごとに 1 件へまとめ、場所は全部を挙げた）。
- **見つかった型**: ⑴ シェルの数字（範囲・座標・入れ子）に上限が無く、壊れたシェル 1 つでメモリを使い果たす・固まる（1〜3）。⑵ 同じ規則が 2 か所で食い違う（4・5・9）。⑶ 写し（重ね方の式・テストの組み立て部品）。⑷ 失敗しえないテスト・名前と違うことを確かめるテスト。⑸ 古くなった説明。
- **大きさ**: 直しは見込みで 22〜26 タスク。要件 7 の 20 を越えるので、組で 2 本に切る案を出す（8 節）。

## 2. 読んだ範囲（要件 1.3）

範囲の全 97 ファイルを読んだ。ソース（`lib.rs`・`error.rs`・`method.rs`・`bind.rs`・`bind_steps.rs`・`composed.rs`・`film.rs`・`nesting.rs`・`normalized.rs`・`pattern.rs`・`pattern_word.rs`・`world.rs`・`atlas_bind.rs`・`base_image.rs`・`boxes.rs`・`fold.rs`・`plan.rs`・`plan_always.rs`・`plan_bounds.rs`・`plan_extent.rs`・`plan_issues.rs`・`blit.rs`・`blit_rect.rs`・`hit.rs`・`hit_import.rs`・`scale.rs`・`log_capture.rs`・`sample_test_support.rs`・`golden_tests.rs`）は進行役が全行を読んだ。テストのファイル 4 組（約 15,800 行）は下請け 4 人が全行を読み、進行役が指摘の行をすべて開いて確かめた。`Cargo.toml`・`tests/surface_nesting_fixture_test.rs`・`tests/fixtures/` の 24 ファイル（`surfaces.txt` 2 つ・`descript.txt` 2 つ・`README.md`・画像）も読んだ。検体のファイルはどれも、どこかのテストか `surfaces.txt` から使われている（使われていない検体は 0）。

## 3. 指摘の一覧

各件の欄: **場所**／**何か**／**困る場面**／**いちばん小さい直し**／**範囲の外への波及**／**残す 3 種類に当たるか**（要件 3.1 の 1＝裁定に反する・2＝わざと置いた先送りの印・3＝SSP 互換・ukadoc 正典）。

### 必ず直す

#### 1. 番号の範囲の展開に上限が無い（`fold.rs:298-346`・`boxes.rs:226,236`）

- **何か**: `surface0-5` のような範囲を、畳み込みが番号の列（`Vec<u32>`）にいったん全部広げる。除外の範囲（`!a-b`）も番号の集合（`BTreeSet`）に全部入れる。箱の畳み込みも同じ関数（`expand_targets`）を通る。
- **困る場面**: `surface.append0-4000000000` と 1 行書いたシェル（打ち間違いでも、わざとでも）を読むと、40 億個の番号の列（約 16 GB）を確保しようとしてアプリが落ちる。`!0-4000000000` も同じ。`surface.append` は「既にある番号」にしか効かないのに、先に全部を広げている。
- **いちばん小さい直し**: ⑴ 追記（`surface.append`）は「今ある番号のうち範囲に入るもの」だけを `SurfaceIndex` から引く（範囲を広げない）。⑵ 除外は範囲のまま「入るか」を比べる（集合に広げない）。⑶ 新設（plain の `surface0-N`）は、範囲がとても大きいときに広げる前に記録（`error!` か `warn!`）を残して打ち切る上限を置く（上限の値は議題）。⑴⑵ は振る舞いを変えない。壊れたら落ちるテスト＝`Range{0,u32::MAX}` の追記・除外が短い時間で終わり、結果が今と同じ。
- **範囲の外への波及**: なし（`expand_targets` は `pub(crate)`・呼び手は `fold.rs` と `boxes.rs` だけ）。
- **残す 3 種類**: ⑶ の上限だけ議題。種類 1（記憶「ゴーストの台本の大きさを甘く見ない・上限で断らず」＝台本の話だが、上限で切る形に近い）と種類 3（SSP が大きな範囲をどう扱うかは ukadoc に書かれていない＝測る）。⑴⑵ は当たらない。

#### 2. 面の外形に上限が無い（`plan_extent.rs:138-142,222-223`・`composed.rs:29-38,96-106`・`blit.rs:76`）

- **何か**: 外形（合成先の絵の幅・高さ）は「置いた位置＋原寸」の最大で決まり、上限が無い。`i64` から `u32` へ `as` で切り詰め、`ComposedSurface` は `width * 4` を `u32` で掛ける。
- **困る場面**: 着せ替えのコマに `animation0.pattern0,overlay,10,100000,100000` と書いたシェルは、外形が約 100000×100000 になり、合成のたびに約 40 GB の確保を試みて落ちる。位置が 2^32 を越えると `as u32` で小さな値に化け、絵が黙って欠ける。幅が 2^30 を越えると `width * 4` が桁あふれ（試験の作りでは panic・出荷の作りでは化けた幅で転写の添字が外れて panic）。
- **いちばん小さい直し**: 外形を飽和で求め、上限（候補: GPU の面の最大寸・`i32` の範囲）を越えたら `error!` を残して `Err`（`ComposeError` に 1 つ足す・`#[non_exhaustive]` なので呼び手は壊れない）。`ComposedSurface` の掛け算は `usize` で行う。テスト＝大きな位置のコマを持つ面が `Err` を返し、確保しない。
- **範囲の外への波及**: `ComposeError` を `match` している範囲の外のテストは `EmptyComposition` だけを名指しで比べる（`areka-emo-present` の `balloon_*_tests.rs` ほか）＝壊れない。`Err` を受けた表示の側の扱いは `areka-emo-present`（並走中の `audit-emo-present` の持ち物）＝新しい `Err` で何をするか（隠す・前の絵のまま）を申し合わせる。
- **残す 3 種類**: 上限の値と「切るか断るか」は種類 1（同上の記憶）・種類 3（SSP が巨大な外形をどう扱うか・測る）の議題。指摘 5 と同じ関数に触る。

#### 3. 入れ子の広がりと深さに上限が無い（`plan.rs:346-466`・`plan_extent.rs:163-285`・`plan_bounds.rs:57-118`・`hit_import.rs:55-87`・`nesting.rs:218-295`）

- **何か**: 入れ子（element定義・pattern定義でサーフェスを置く）は、祖先の列で循環だけを止める。同じ子を何度もたどること（菱形）と、深い鎖には上限が無く、どれも再帰で書かれている。
- **困る場面**: ⑴ `surface1` が `surface2` を 2 か所に置き、`surface2` が `surface3` を 2 か所に置き……を 30 段重ねたシェルは、合成のたびに 2^30（約 10 億）個の命令を作り、UI スレッドが固まる（外形・シェルの外形・当たり判定の持ち込みも同じく指数で増える）。⑵ `surface0` → `surface1` → … → `surface100000` の鎖は、再帰が 10 万段になってスタックがあふれ、アプリが落ちる。
- **いちばん小さい直し**: 1 回の合成（外形・命令・持ち込みの各走査）に「たどる節の数」の予算と深さの上限を置き、越えたら `error!` を 1 行残してその枝を打ち切る（部分の結果は今の循環の扱いと同じ形）。テスト＝菱形 30 段と深い鎖のシェルが短い時間で終わり、記録が出る。
- **範囲の外への波及**: なし（`visible_parts` は `areka-seriko` が呼ぶが、形は変わらない）。
- **残す 3 種類**: 上限の値は種類 1（同上）の議題。`hit_import.rs:53`・`nesting.rs:401` の `ponytail:` 注釈（「読み込み 1 回だけなので」）は種類 2 に当たる。普通のシェルの負荷は越えていないが、壊れたシェルでは越える＝この直しの予算でまとめて覆うかを議題にする。

#### 4. 「見える部品」の表と合成が `bind+random` で食い違う（`nesting.rs:97-98,134-143`・`bind_steps.rs:157-166`）

- **何か**: 入れ子の表（`NestTable`）は、有効な `bind+random` の着せ替えにコマが無いとき、その pattern0 の先を「見える部品」に数える（説明は「合成と同じ辺」）。ところが合成は、2026-10-31 の `animated-surface-log-flood` 件 26 の直しから、コマの無い `bind+random` を何も描かない。直したときに表の側を揃えなかった。
- **困る場面**: `surface100` に `animation5.interval,bind+random,2`・`animation5.pattern0,overlay,40,...` があり、着せ替え 5 が有効でコマが無いと、画面には 40 が出ていないのに `visible_parts` は 40 を返す。seriko は 40 の部品のアニメーションを毎刻み評価し、境界で抽選する（乱数を使い、動く絵の子も「見える」に数える）。テストは両側が逆の答えを固定している（`nesting_visible_tests.rs:291-335` は見える・`plan_bind_random_tests.rs:86-88` と `plan_ops_tests.rs:984-985` は描かない）。両者が一致することを確かめるテスト（`nesting_visible_tests.rs:694-728`）は、この入力を 1 つも持たないので見逃している。
- **いちばん小さい直し**: `nesting.rs:138` で `bind_targets` に足すのを純 `bind` だけにする。`bind_steps.rs` の `bind+random` の手順は、そうすると誰も描かないので作らない（`push_bind_contribution` の早い戻りもまとめられる）。テスト＝`nesting_visible_tests.rs:331,334,335` の期待を直し、一致のテスト（694-728）の入力に「pattern0 を持つ `bind+random`」を足す（直す前は落ちる）。
- **範囲の外への波及**: `crates/areka-seriko/src/parts_tests.rs:395-460` が「40 が見える扱い」を前提に乱数の呼び出し回数を固定している＝期待が変わる。**並走中の `audit-seriko` の範囲**なので、触る前にそのセッションと申し合わせる（要件 5.2）。
- **残す 3 種類**: 種類 1 には当たらない見込み（件 26 の開発者の決め「止めた重ね絵を残さない」に表を揃える直し）。ただし seriko のテストの説明が「設計どおりの既知の振る舞い」と書いているので、討議で確かめる。

#### 5. 外形が画像の element定義の X,Y を数えない（`plan_extent.rs:207-223`・`plan.rs:225-229`）

- **何か**: 命令は画像を「置いた位置＋element定義の X,Y」に置くのに、外形は「置いた位置＋原寸」だけを数え、element定義の X,Y を足さない（`plan_bounds.rs:91` のシェルの外形は足している）。
- **困る場面**: `element1,overlay,arm.png,150,0` で土台（幅 150）の右にはみ出して置いた腕は、外形が 150 のままなので黙って切られる。記録も出ない。テスト `plan_extent_film_tests.rs:121,129` は今の（切る）振る舞いを固定している。
- **いちばん小さい直し**: `flatten_extent` の画像の枝に element定義の X,Y を足す（1 行）。テスト＝はみ出す element定義の外形。
- **範囲の外への波及**: 窓の大きさが変わるシェルが出る（`areka-emo-present`・`crates/areka/src/placement/` は外形を受け取るだけ）。
- **残す 3 種類**: **種類 1 の議題**。この件は開いている spec `extent-element-offset`（区分 B・「和集合に揃えるか、ベースで切ると明文化して記録を出すか」の開発者の裁定待ち）が持つ。本 spec で直すか、あちらに任せて残すかを決める。種類 3（ukadoc の外形の定義）も絡む。

### 直したほうがよい

#### 6. サーフェスを受ける当たり判定の 2 関数は本番で使われず、持ち込んだ領域を素通りする（`hit.rs:79-86,157-165`・`lib.rs:72`・`hit_in_tests.rs:97-127`）

- **何か**: `hit_region`・`hit_region_scaled` は `SurfaceMaster.collisions`（転記のままの列）で引く。本番（`areka-emo-present` の `presenter/hit.rs`）は子から持ち込んだ領域を含む `EmoWorld::hit_regions` を `_in` の形へ渡しており、2 関数の本番の呼び手は 0（ツリー全体を grep した）。
- **困る場面**: 新しい呼び手がこの名前の短い方を使うと、入れ子のサーフェスで子の当たり判定が効かない。`hit_in_tests.rs:97-127` は「同じ列を受けた 2 つの形が同じ答え」を確かめるが、片方がもう片方を呼ぶだけなので失敗しえない。
- **いちばん小さい直し**: 2 関数と `lib.rs` の再公開を消す。`hit.rs` のテストは `&m.collisions` を `_in` へ渡す形に直す。`hit_in_tests.rs` は答えを固定した 1 本（130-154）だけ残す。`normalized.rs:83` の説明のリンクを `hit_region_in` へ。
- **範囲の外への波及**: コードの呼び手は 0。`areka-emo-present/src/presenter_read_accessor_tests.rs:373,383` の説明の文が古い名前を書いている＝`audit-emo-present` の持ち物（直さなくても壊れない・伝えるだけ）。
- **残す 3 種類**: 当たらない。

#### 7. 重ね方の式が 2 か所に写してあり、ふつうの道は画面の外の画素も 1 つずつ回る（`blit.rs:19-33,84-161`・`blit_rect.rs:25-36,84-99,176-218`）

- **何か**: premultiplied SourceOver の式（`div255`・`source_over_channel`）と、1 命令を転写する二重ループが `blit.rs` と `blit_rect.rs` に別々にある。`blit.rs` は転写元の全画素を回ってから 1 画素ずつ外へ出たかを見る。`blit_rect.rs` は先に矩形で切ってから回る。
- **困る場面**: 式を片方だけ直すと、いちからの合成と矩形の重ね直しで画素が食い違う（今はテストが 2 つの道の一致を確かめているので気付けるが、直す場所が 2 つある）。大きな部品を大きく外へはみ出して置くと、見えない画素の分も毎フレーム回る。
- **いちばん小さい直し**: 「1 命令を矩形で切って重ねる」関数を 1 つにし、`blit::execute` は「消す＋全命令を外形の矩形で切って重ねる」、`execute_reusing` は「写す＋違う矩形で切って重ねる」にする。式は 1 か所。
- **範囲の外への波及**: なし（どちらも `pub(crate)`）。
- **残す 3 種類**: 当たらない（`Overlay` 以外を矩形の道に入れない決めはそのまま残す）。

#### 8. 毎フレームの余分な仕事と確保（`lib.rs:92-94` の「確保なし」の約束と食い違う）（`plan.rs:143-144,388,454-455,425`・`plan_extent.rs:124-143,257-259`・`bind_steps.rs:97-99`・`method.rs:134-136`・`pattern_word.rs:352-353`・`world.rs:312-318`）

- **何か**: ⑴ 外形は「束縛のあと変わらない静的な量」なのに、合成のたびに全部をたどり直す。⑵ その中で着せ替えのコマ 1 つごとに描画メソッドの語を 2 回正規化し、毎回文字列を 2 つずつ確保する（`targets_animation_id`・`is_move_name`）。⑶ 静的な element の並びは畳み込みで整列済み（不変条件）なのに、毎回添字の `Vec` を作って並べ直す。⑷ 段ごとに `merged_ids`・`claims`・`done`・`expanded`・`stack` を確保する。⑸ `animation_sort()` は値を複製する（`Other(String)` なら確保）。
- **困る場面**: 動く絵のコマ替えのたびに合成するシェルで、確保と文字列の処理が毎フレーム走る。`lib.rs`・`plan.rs` の説明は「定常状態で確保なし（要件 10.3）」と書いており、読み手を誤らせる。
- **いちばん小さい直し**: ⑴ 外形を `bind_atlas` の直後に面ごとに 1 度求めて面の表のリソースに置き（`BindSteps` と同じ形・合成器は結果を持たない約束のまま）、`build_plan` は引くだけにする。これで ⑵ も毎フレームから消える。⑶ 並べ直しを消す。⑷⑸ は計測で効くと分かったものだけ合成器の作業場へ移す。説明を事実に合わせる。
- **範囲の外への波及**: なし（`compute_extent` は `pub(crate)`・`shell_bounds` は読む側の形が変わらない）。
- **残す 3 種類**: 当たらない。研究: 効き目は `tools/perf/` の計測で確かめる（10 節）。

#### 9. 追記（`surface.append`）と波括弧の足し合わせで、同じ番号の行の扱いが違う（`fold.rs:136-162` と `fold.rs:221-229,261-282`・`boxes.rs:231` と `boxes.rs:238`）

- **何か**: 同じ番号の 2 つ目の `surface*`ブレスは「同じ element番号・collision番号・pattern番号の前の行を除いてから足す（後勝ち）」。`surface.append*` は element・collision を重複のまま足し、animation は丸ごと置き換える。箱の畳み込みも同じ違いを持つ。
- **困る場面**: `surface0{element0,overlay,a.png,0,0}` の後の `surface.append0{element0,overlay,b.png,0,0}` は a と b を両方重ねて描く。同じ 2 行目を `surface0{...}` で書くと b だけになる。`surface.append0` で `animation1.pattern3` だけを足すと、animation1 の pattern0〜2 が消える。
- **いちばん小さい直し**: 追記も波括弧と同じ `merge_brace` を通す（1 本にまとめる）か、違いを規則として説明に書き残す。
- **範囲の外への波及**: なし。
- **残す 3 種類**: **種類 3 の議題**。ukadoc（<https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/#caption_surface.append>）の原文は「surface.appendによる定義は、『既に（surfaceブレス、またはsuface*.pngの存在によって）ある定義』に対してのみ付け加えられる。」とだけ書き、同じ番号の行の勝ち負けは書いていない。SSP の MCP で測ってから決める（記憶「ukadoc で読めない点は議題の前に SSP の MCP で測る」）。animation の丸ごと置き換えは今 `warn!` で「de-facto」と書いてある。

#### 10. 失敗しえないテスト（偽の安心）

- **場所と何か**（どれも開いて確かめた）:
  - `hit_in_tests.rs:97-127`: 片方がもう片方を呼ぶだけの 2 つの形を比べる（指摘 6）。
  - `plan_extent_tests.rs:107-125`: `BindSet` を作って `_b` に置くだけで誰にも渡さず、同じ呼び出しを 3 回比べる。「有効な着せ替えに依らない」は確かめていない。
  - `scale_ratio_tests.rs:124,131,201,595,638-641`（同じ形がほかにも数か所）: 同じ純関数を同じ値で 2 回呼んで比べる。
  - `pattern_parts_tests.rs:24`: 既定値どうしを比べる。`plan_extent_tests.rs:393-394`: 直前で 300×300 と確かめた後に「0 でない」を確かめる。
  - `composer_tests.rs:638-646` `default_equals_new`: 空の `Vec` の長さどうしを比べる（`base_ops` は見ない）。
  - `golden_tests_surface0_base_tests.rs:142-200`: 「違う画像なら違う」を、大きさの違う画像で確かめる＝長さだけで決まり、画素を 1 つも写さなくても通る。
  - `golden_tests_trim_equivalence_tests.rs:213-261`・`composer_tests.rs:727-774`: 手で作った誤りの値が自分と違うこと・前のテストと同じことを確かめるだけ。
- **困る場面**: 本物の誤りが入っても緑のまま。数だけ多く、直すときの手間だけが増える。
- **いちばん小さい直し**: 消す。`plan_extent_tests.rs:107-125` だけは `build_plan` に別々の `BindSet` を渡して外形が同じことを確かめる形に直す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 11. 名前や説明と違うことを確かめているテスト

- **場所と何か**:
  - `fold_tests.rs:240-254`: 「2 → 1 の順に作る」と書くが、両方が在ることと数しか見ない。
  - `bind_steps_tests.rs:78-89`: 「`bind+random` に `insert` は入らない」の唯一のテストだが、`insert` は pattern2 に在り、pattern0 だけを残す絞りで先に消える。`bind_steps.rs:68` の `if pure` を消しても緑。
  - `plan_ops_tests.rs:894-925`: 「pattern0 の無い着せ替えは描かない（RED 核）」だが `bind+random` を使うので、件 26 の早い戻りでいつも空になり、守るはずの絞りに届かない。`plan_ops_tests.rs:931` の名前（`..._composes_only_index0`）は自分の期待（純 `bind` は全部のコマ）と逆。
  - `plan_bind_tests.rs:287-295`: 「合成を繰り返しても記録は 1 度」を確かめるはずが、毎回面の表を作り直している。
  - `plan_bounds_tests.rs:70-81`: 合成が `Err` の面を黙って飛ばすので、全部の面が `Err` なら `plan_bounds_tests.rs:198-221` の 2 本は何も確かめずに通る。
  - `golden_tests_surface1000_bind_tests.rs:50-59`: 「pattern0 が在る」を `min_by_key(index)`（番号の最も小さいもの）で見る＝pattern0 が無くても通る。
  - `nesting_kind_tests.rs:118-119`: 引けなかった記録は `debug!` なのに `WARN` の無いことを見る。
  - `plan_extent_film_tests.rs:182-200`: `bind+random` の pattern0 が他のコマの和の内側にあり、「最も小さい番号を数える」は確かめていない。
  - `composer_tests.rs:617-636`: 「毎フレームの道で `resolve` を呼ばない」を `plan.rs`・`blit.rs`・`lib.rs` でしか見ず、`plan_extent.rs`・`plan_always.rs`・`blit_rect.rs` を見ない。
  - `film_tests.rs:444-464`: GIF の半分は静止画を `.gif` の名前で入れているだけ。
- **困る場面**: 守っているつもりの分岐が壊れても赤にならない（指摘 4 の食い違いもこの型で見逃された）。
- **いちばん小さい直し**: 各テストの入力か比べ方を 1〜数行直す（例: 順序は `assert_eq!(expand_targets(..), vec![2, 1])`・`if pure` は pattern0 に `insert` を置いた `bind+random`・`plan_bounds` は `expect` か件数の確かめ・走査の表に 3 ファイルを足す）。`plan_ops_tests.rs:894-925` は消すか純 `bind` に直す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 12. テストの組み立て部品が多くのファイルに写してある

- **場所と何か**（下請けの報告を進行役が grep と `diff` で確かめた）:
  - 面の組み立て（`elem`・`surface`・`surface_with_anims`・`shell_of`・`bake_atlas` ほか）: `plan_test_support.rs:14-117` が `plan` の中だけの非公開の部品なので、`composer_tests.rs:22-77`・`log_firing_tests.rs:36-133`・`atlas_bind.rs:76-150`・`film_tests.rs:22-51`・`fold_tests.rs:40-71`・`base_image_tests.rs:16-47` に写しがある。`bake_atlas` は `nesting_kind_tests.rs:139-170` と `nesting_visible_tests.rs:484-515` が 1 バイトも違わない。
  - コマ 1 つ（`frame`）が 9 か所（`nesting_visible_tests.rs:42`・`pattern_cell_tests.rs:10`・`plan_always_tests.rs:212`・`nesting_film_tests.rs:104`・`pattern.rs:236`・`plan_bind_tests.rs:72`・`plan_nesting_tests.rs:285`・`plan_nesting_extent_tests.rs:135`・`pattern_parts_tests.rs:10`）。
  - 領域 1 つ（`coll`）が `hit.rs:202`・`hit_in_tests.rs:16`・`hit_import_tests.rs:29`・`fold_tests.rs:74`。1 画素の頁（`page_1px`・`single_entry_table`）が `blit.rs:176,188` と `log_firing_tests.rs:333,343`。`solid` が `film_tests.rs:54` と `plan_extent_film_tests.rs:36`（同一）。`px` が `blit.rs:214` と `composer_tests.rs:125`。`png_size` が `golden_tests_frame_extent_tests.rs:41-53` と `nesting_fixture_tests.rs:41-51`。大きさ指定の焼き（`plan_extent_tests.rs:43-65`・`plan_nesting_extent_tests.rs:36-63`・`plan_bounds_tests.rs:26-67`）。動く絵の焼き（`plan_always_tests.rs:72-146`・`plan_extent_film_tests.rs:47-111`）。記録の読み取り（`plan_nesting_tests.rs:88-108`・`nesting_visible_tests.rs:790-808`）。命令の名前引き（`plan_nesting_tests.rs:50-81`・`plan_bind_tests.rs:38-66`）。
  - `ops_to_paths`（`plan_ops_tests.rs:262-271`）の中身が `plan_ops_tests.rs:44-52,86-89`・`plan_bind_random_tests.rs:64-66`・`plan_bind_tests.rs:56-60`・`plan_nesting_tests.rs:71-75` に書き直してある。`plan_ops_tests.rs` の `derive_ops(...)` の 9 行の塊が 22 回・`push_static_element_ops(...)` の 11 行の塊が 8 回。
  - `bind_anim_xy`（`plan_ops_tests.rs:615-617`）は `bind_anim` をそのまま呼ぶだけ（説明「x,y=0 固定を拡張」は誤り）。
- **困る場面**: 面の型に欄を 1 つ足すたびに 10 か所近くを直す。写しの一部だけが古くなる（`plan_test_support.rs` の `bind_anim` は無い面 999999 を指す pattern5 を持つが、写しは持たない）。
- **いちばん小さい直し**: `plan_test_support.rs` をクレートの直下の `#[cfg(test)] mod`（`sample_test_support` と並べる）に上げて `pub(crate)` にし、写しを消して引く。`frame`・`coll`・頁の部品も同じ所へ。`derive_ops` の塊は `fn ops(..) -> Vec<BlitOp>` 1 つにする（約 200 行減る見込み）。
- **範囲の外への波及**: なし（テストの中だけ）。**残す 3 種類**: 当たらない。

#### 13. 1,000 行を越えるテストのファイル（`plan_ops_tests.rs`・1,468 行）

- **何か**: 着手の前から番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs:63`・範囲の外）に載っている。指摘 11・12 の直しで中を触る。
- **困る場面**: 例外が残る限り、このファイルは際限なく伸びられる。
- **いちばん小さい直し**: テーマで 2 つに分ける（1〜855 行＝静的・着せ替え・循環の命令／857〜1468 行＝コマの合流の門を新しい `plan_frame_merge_tests.rs` へ）。どちらも 1,000 行の下。例外表から 1 行を消す（要件 6.1 は「足さない」で、消すのは妨げない）。
- **範囲の外への波及**: 例外表の 1 行（`log-capture-kit` のテスト）＝直しに要る最小の箇所（要件 5.4）。
- **残す 3 種類**: 当たらない。

### できれば

#### 14. 同じことを 2 度確かめているテスト

- **場所**: `blit_rect_tests.rs:286-291` と `:119-129`（下敷きなし → いちから）。`fold_tests.rs:193-221` と `fold_merge_tests.rs:109-169`（数の確かめだけが固有）。`plan_ops_tests.rs:1238-1271` と `:474`（入力・期待が同じ）。`golden_tests_determinism_budget_tests.rs:68-79`（同じ 4 つを 2 度）・`:93-133`（`compose` は `compose_into` の包み）・`:224-278`（容量の確かめの再掲）。`golden_tests_surface1000_bind_tests.rs:89-94,178-181`。`nesting_fixture_tests.rs:268-288` と `hit_import_tests.rs:249-270`。`pattern_cell_tests.rs:86,120,137,154` と `pattern_parts_tests.rs:43,75,100`・`pattern.rs` の `eq_is_insertion_order_stable`。`plan_extent_tests.rs:493-521` と `:525-578`。`plan_nesting_tests.rs:222-262` の報告の部分と `nesting_visible_tests.rs:814-837`。`scale_ratio_tests.rs:114` と `:440`・`:220` と `:336`。`blit_rect_tests.rs:395-405` と `composer_tests.rs:617-636`（走査の確かめ）。
- **困る場面**: 振る舞いを変えるとき、同じ期待を何か所も直す。
- **いちばん小さい直し**: 弱い方を消し、固有の 1〜2 行だけを強い方へ移す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 15. 古くなった説明（コードの説明とテストの説明）

- **コード**: `plan.rs:8-36`（「本 task」「後続 task」の作業の経緯）・`plan.rs:96-114,275-284`（「後続 task が分類を担う」＝もう `build_plan` がある）・`plan.rs:73` と `plan_extent.rs:119-120,147-150`（「全 bind animation の pattern0」＝今は純 `bind` と `always` は全部のコマ）・`lib.rs:92-94` と `plan.rs:33-36`（確保なし＝指摘 8）・`method.rs:7`（未実装は「warn 以上」＝今は `debug!`）・`world.rs:28`（「本 task で定義し task 4 が挿入」）・`world.rs:328-336`（`world()`・`world_mut()` は「本 spec では未使用」＝`hit_import.rs:43` ほかが使う）・`scale.rs:369-373`（「`num`／`den` の口は新設しない」＝`parts()` が `scale.rs:245` に在る）・`nesting.rs:97-98,137`（「合成と同じ辺」＝指摘 4）・`fold.rs:34-35`。
- **テスト**: `log_firing_tests.rs:1,6,11-12,140,158-159,175-178,199,214,230-231,361,401`・`plan_ops_tests.rs:857-864,887-892,919,922,993-998,1031-1036,1077,1093-1097,1276-1279,1317-1324,1359-1364`・`nesting_report_tests.rs:78`（WARN → 今は DEBUG・「bind 種」→ 今は `bind+random` だけ）・`golden_tests_blink_static_tests.rs:22-28,59-63,136-141`・`golden_tests_frame_extent_tests.rs:12-13,208`・`golden_tests_test_support.rs:50-51`・`golden_tests_surface1000_bind_tests.rs:66-67`・`plan_extent_tests.rs:12-13,104,442`・`plan_bind_random_tests.rs:5-6`・`nesting_visible_tests.rs:1-6,291-292`・`plan_test_support.rs:43-47,62`（pattern5 は「使わない」＝今は純 `bind` が全部たどる）・`fold_tests.rs:12,50`・`composer_tests.rs:563-564,613`・`plan_ops_tests.rs:242`・`blit_transparent_alpha_tests.rs:3-5`・`tests/fixtures/animated-playback/README.md:27`（`descript.txt` は `charset,UTF-8` も持つ）。
- **困る場面**: 説明を信じた読み手が誤った前提で直す（指摘 4 はこの型で残った）。
- **いちばん小さい直し**: 説明を今のコードに合わせる。作業の経緯の段落は消す。
- **範囲の外への波及**: なし（`doc/`・`docs/` のパスを名指す行は 0 のまま・要件 6.9）。**残す 3 種類**: 当たらない。

#### 16. 要らない小さなコード

- **場所と何か**: `fold.rs:36-42` `fold_shell`（テストだけが呼ぶ包み・`expect(dead_code)`）＝テストは `fold_shell_with(.., Records::Emit)` を呼べばよい（23 か所）。`composer_tests.rs:648-650` `_type_anchor`（使わない取り込みを黙らせるだけ）。`plan_ops_tests.rs:551` `let _ = &atlas;`。`plan_ops_tests.rs:615-617` `bind_anim_xy`（指摘 12）。`scale_test_support.rs`（定数 1 つだけの 2 行のモジュール＝`scale_ratio_tests.rs` の頭へ）。`lib.rs:124-130` `Composer::new` の手書き（`Default` を導いているので `Self::default()` で足りる）。
- **困る場面**: 読む・直す行が増えるだけ。
- **いちばん小さい直し**: 消す・1 行にする。
- **範囲の外への波及**: `Composer::new` は範囲の外（`areka-emo-present`・`crates/areka/examples`）も呼ぶ＝名前は残すので壊れない。**残す 3 種類**: `fold_shell` の `expect(dead_code)` は「語彙の口」ではないので種類 2 には当たらない。

#### 17. 面の新設が 2 か所に写してある（`base_image.rs:75-83`・`fold.rs:388-403`）

- **何か**: 「`SurfaceId` と `SurfaceMaster` で entity を作り `SurfaceIndex` に載せる」2 行が、土台の絵の決定と畳み込みの `upsert_surface` に別々にある。
- **困る場面**: 面の entity に部品を 1 つ足すとき、片方だけ直すと「画像だけで在る面」だけ欠ける。
- **いちばん小さい直し**: `upsert_surface` を `pub(crate)` にして `base_image.rs` から呼ぶ。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 18. `ComposedSurface` の `stride` 欄は常に `width * 4`（`composed.rs:19,30,97,115`）

- **何か**: 欄として持ち、3 か所で `width * 4` を入れ直している。
- **困る場面**: 入れ直しを 1 か所忘れると、幅と行の長さが食い違う。
- **いちばん小さい直し**: 欄を消し、`stride()` が `width * 4` を返す（公開の形は同じ）。指摘 2 の掛け算の直しと一緒に。
- **範囲の外への波及**: なし（`stride()` の形は変えない）。**残す 3 種類**: 当たらない。

#### 19. 転写元の範囲外読みは守らず落ちる（頁の欠けは守るのに）（`blit.rs:140-147`・`blit_rect.rs:203-215`）

- **何か**: 頁が無ければ `warn!` で飛ばすのに、`uv_rect` が頁の外へはみ出していると添字が外れて panic する。`AtlasTable::new`（公開）は `uv_rect` と頁の大きさを照らさない。
- **困る場面**: 焼きの誤りや手で組んだ表で uv が 1 画素はみ出すと、UI スレッドで落ちる。
- **いちばん小さい直し**: 命令ごとに 1 度だけ「uv が頁に収まるか」を見て、外なら `warn!` で飛ばす（頁の欠けと同じ扱い）。指摘 7 の 1 本にした関数の頭に置く。
- **範囲の外への波及**: なし。表の不変条件そのものは `areka-emo-atlas`（`audit-emo-atlas` の範囲）＝あちらで表を作るときに確かめる直しと重ねるかは申し合わせ。**残す 3 種類**: 当たらない。

#### 20. 本番ではいつも 1 つの値しか通らない型の口（`plan.rs:61-69` の `BlitOp.method`・`blit.rs:89-97`・`normalized.rs:17-54` の `Transform.linear`・`hit.rs:42-48` の `RegionPriority`）

- **何か**: 畳み込みが element の描画メソッドをいつも `Overlay` にし、コマの `Overlay` 以外は命令の前に落とすので、`blit.rs` の「未実装の命令を飛ばす」枝は本番で通らない。`Transform` の 2×2 の部分はいつも単位行列。`RegionPriority` は 1 つだけの値。
- **困る場面**: 今は無い。読む・試す行が増えるだけ。
- **いちばん小さい直し**: 消すなら各 1〜数十行（`method` 欄・`linear` 欄・`RegionPriority` の引数）。
- **範囲の外への波及**: `RegionPriority` は `areka-emo-present/src/presenter/hit.rs` が渡している＝消すならそちらも直す。
- **残す 3 種類**: **種類 2 の議題（残す案を推す材料が強い）**。記憶「合成メソッドは D2D が支える全部に対応できるよう wintf も含めて拡張する予定」と、開いている spec `draw-methods-canon`・`draw-methods-blend`・`element-clipping-option` がこの口を使う。`RegionPriority` は SSP の `collision-sort` を忠実に解かない意図的な逸脱の口（種類 3 も絡む）。

### 決めごととして指摘に数えなかったもの（`shortcut:`・`ponytail:` の注釈）

- `nesting.rs:401`（`reaches` を辺 1 本ごとに全体へ）・`hit_import.rs:53`（根ごとに子をたどり直す）。どちらも「読み込み 1 回だけ」の限界を名指ししており、普通のシェルの負荷は越えない＝audit の作法どおり指摘にしない。ただし壊れたシェルでは指摘 3 の型で越えるので、指摘 3 の議題で扱う。

## 4. 要件と手持ちの対応（Requirement-to-Asset Map）

| 要件 | 手持ち | ギャップ |
|---|---|---|
| 1 洗い出し | 本文書（20 件・全ファイル読了） | なし（要件 1.5 の書き足しは討議の後） |
| 2 全部直す | 各指摘の直しは既存の関数の中で閉じる | **Constraint**: 4 は `areka-seriko` のテスト・2 は `areka-emo-present` の扱いに及ぶ |
| 3 残す裁定 | 議題の候補は 1⑶・2・3・5・9・20（3 節の各「残す 3 種類」） | **Unknown**: 9 は SSP の測りが要る（ukadoc に書いていない） |
| 4 直した後 | 全体テスト `tools/test-all.ps1`・`log-capture-kit`・`MemoryDecoder`（決定論の土台は揃っている） | **Missing**: 1〜4・19 の「壊れたら落ちるテスト」は今 0 本 |
| 5 範囲の外 | grep で呼び手を確かめ済み（各件の欄） | **Constraint**: `audit-seriko`・`audit-emo-present` と申し合わせ（4・2・6）。`audit-emo-atlas` と 19 |
| 6 守る決まり | 依存の追加は不要（全件 std と既存の依存で直る）・`Cargo.lock` は変わらない | 13 で例外表の 1 行を消す（足さない） |
| 7 大きさ | — | **Constraint**: 見込み 22〜26 タスク＝20 を越える（8 節） |

## 5. 直し方の選択肢

- **案 A（今のファイルの中で直す）**: 全件を今のファイルの中で直す。新しいファイルは `plan_frame_merge_tests.rs`（13）と、組み立て部品をクレートの直下へ上げる 1 つ（12）だけ。外形の前計算（8）は `BindSteps` と同じ形のリソースを `plan_extent.rs` の中に足す。
  - ✅ 構造を変えない・差分が小さい。❌ `plan.rs`（707 行）・`plan_extent.rs` に予算の引数が増える。
- **案 B（走査の文脈を 1 つの型にまとめてから直す）**: `flatten_surface`・`flatten_extent`・`walk`・`regions_of` の引数（9 本前後）を、既にある `Level` に似た文脈の型にまとめ、予算（3）と前計算（8）をその型に持たせる。
  - ✅ `too_many_arguments` の許しが消え、予算の置き場所が 1 つになる。❌ 差分が大きく、並走の `draw-methods-canon` ほか（`plan.rs` を触る列）と重なりやすい。
- **案 C（混ぜる）**: 必ず直すの 1〜4 は案 A で先に入れ（テストつき）、8 の前計算と 3 の予算の置き場所は設計の段で案 B の型を小さく入れるかを決める。

## 6. 手間と危うさ

- **手間**: M（3〜7 日）。直しの 1 件ずつは小さいが、テストの整理（10〜14）の行数が多い。
- **危うさ**: Medium。上限の値（1〜3）と外形（5）・追記の規則（9）は開発者の裁定と SSP の測りが要る。4 は並走の `audit-seriko` のテストに触る。ほかは既存の型の中の直しで Low。

## 7. 設計の段への申し送り

- 推す並び: 必ず直す（1〜5）→ 直したほうがよい（6〜13）→ できれば（14〜20）。テストの整理（12）は 10・11・13・14 の前に置くと、写しを直す手間が 1 度で済む。
- 決めること: 上限の値と「切る／断る」（1〜3）・新しい `ComposeError` を表示の側がどう扱うか（2）・外形の規則（5・`extent-element-offset` との持ち分け）・追記の規則（9）・型の口を残すか（20）。

## 8. タスクの数の見込みと切り方（要件 7.1）

| 組 | 指摘 | タスクの見込み |
|---|---|---|
| 必ず直す | 1〜5 | 7〜9（各件に直し＋壊れたら落ちるテスト・2 と 4 は申し合わせを含む） |
| 直したほうがよい | 6〜13 | 9〜11（12 は 2〜3 タスク・13 は 1） |
| できれば | 14〜20 | 4〜5 |
| 確かめ | — | 2（audit のかけ直し・全体テスト） |
| 計 | 20 件 | **22〜27** |

20 を越えるので、要件 7.1 のとおり組で切る案を出す。

- **切り方 甲（推す）**: 本 spec＝必ず直す＋直したほうがよい（1〜13・約 18〜22）。2 本目（同じ範囲・例 `audit-emo-compose-2`）＝できれば（14〜20・約 6〜7）。5 を `extent-element-offset` に任せる裁定なら本 spec は 20 の内に収まる見込み。
- **切り方 乙**: 本 spec＝コードの直し（1〜9・16〜20）。2 本目＝テストの整理（10〜15）。テストの整理だけで 1 本になり、ほかの spec と重ならない。
- **切らない**: 5 を `extent-element-offset` へ、20 を残す裁定にすれば、約 19〜22 で 1 本に収まる見込み。

## 9. 並走の兄弟（ウェーブ C17）に及ぶ所

- `audit-seriko`（`crates/areka-seriko/`）: 指摘 4 の直しで `src/parts_tests.rs:395-460` の期待（40 が見える扱い・乱数の呼び出し回数）が変わる。`src/parts.rs` のコードは変えない見込み。触る前に申し合わせる。
- `audit-emo-present`（`crates/areka-emo-present/`）: 指摘 2 の新しい `Err` の扱い（コードの変更が要るかは申し合わせ）。指摘 6 で古くなる説明の 2 行（`presenter_read_accessor_tests.rs:373,383`・直さなくても壊れない）。指摘 20 で `RegionPriority` を消す裁定なら `presenter/hit.rs`。
- `audit-parsers-sakura`（`crates/areka-parsers/`・`crates/areka-sakura/`・2026-11-05 の申し合わせ）: そちらの指摘 6 が巨大な範囲の上限を読む側でかける＝指摘 1⑶（新設の範囲の上限）はそちらへ任せ、fold では上限をかけない（議題 1 の裁定しだいで見直す）。fold に残すのは 1⑴⑵（振る舞いを変えない）。そちらの 76（`ShellBoxes` → `Vec<BoxDefinition>`）が `boxes.rs:226`・`:236` で指摘 1 と同じ行に触り、36・38 でこのクレートのテストの期待と字面を直す＝後から着いた側が rebase で合わせる。指摘 12 でテストの組み立て部品の置き場が動くことは伝えた。**確定（同日・そちらの要件 7.6・7.10）**: 読む側で範囲の「幅」に上限 8193（ukadoc caption_standarddef「surfaceID は 0 から 8192 まで」の数）をかけ、越えたら `error!` でその記述子だけ捨ててシェルは全体として読む（番号そのものは縛らない）。**注（2026-11-05・開発者の問いで ukadoc を引き直した）**: 原文は「なお、MATERIAではsurfaceIDにつかえる数字は0から8192までという制限がある。」（<https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/#caption_standarddef>）＝MATERIA だけの制限で、ukadoc の一般の決まりは「割り振りは基本的に自由」。SSP 向けの実在のシェルは 8193 以上の番号を使う（`C:\wintools\ssp\ghost\` の `Yada-Yada-Yada` の `surface11000`、`ponyaduma` の 9121、`emo` の 8210 を pattern が参照）＝番号の上限としては使えない（幅の上限の数として借りるのは parsers の持ち物）。幅が 1,000 を越える範囲は 27 体のどれにも無かった。fold の 1⑴⑵ はこちら、`boxes.rs:226`・`:236` とテストの部品の移動は後から着いた側が合わせる。
- そのほかの範囲の外: 指摘 13 の例外表の 1 行（`crates/log-capture-kit/tests/file_length_guard_test.rs:63`）。`crates/areka-emo-text/` には触らない（要件 6.10）。

## 10. 調べが要る所（Research Needed）

1. SSP で、`surface.append*` が同じ element番号・collision番号・animation番号の行を書いたときの勝ち負け（指摘 9・SSP の MCP で測る）。
2. SSP で、巨大な範囲（`surface0-100000` 程度）・巨大な位置（外形 10 万画素）・深い入れ子を持つシェルがどうなるか（指摘 1〜3 の上限の値の手がかり）。
3. ukadoc の外形の定義（指摘 5・`extent-element-offset` の議題 1 と同じ）。
4. 指摘 8 の前計算の効き目（`tools/perf/perf-loop.ps1` で、動く絵のあるシェルの合成の時間と確保の回数を前後で測る）。

## 11. 設計の段の調べと決め（2026-11-05・`/kiro-spec-design -y`）

- 使ったもの: `kiro-spec-design` の決まり（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と設計のひな形。ponytail（いちばん小さな完全な直し）の物差し。発見の段は「既存の仕組みの拡張」として軽い型（light）で行い、下請けは使わず進行役がコードを読んだ。
- 測った時点: 枝 `claude/kiro-start-audit-emo-compose-cb01ad`（`56720f79`）。`88eb0a43` から後にこのクレートと `log-capture-kit` への変更は 0（`git log` で確かめた）＝3 節の行の番号は今も使える。ただし次の行は 3 節の番号がずれていた（設計ではこちらを使う）。
  - `plan_extent.rs`（202 行のファイル）: 外形の `as u32` は 45〜46 行、画像の原寸の枝は 116〜128 行、着せ替えの id の列は 136〜143 行、描画メソッドの語の確かめは 162〜165 行、再帰は 68〜190 行（3 節の 138-142・222-223・163-285・207-223・257-259 は誤り）。指摘 5 の「element定義の X,Y を足さない」は 127〜128 行。
  - ほか（`fold.rs`・`boxes.rs`・`composed.rs`・`blit.rs`・`blit_rect.rs`・`plan.rs`・`plan_bounds.rs`・`hit_import.rs`・`nesting.rs`・`bind_steps.rs`・`method.rs`・`world.rs`）は 3 節の番号どおり。

### 11.1 調べたこと

| 問い | 分かったこと | 設計への効き目 |
|---|---|---|
| GPU の面の最大寸の定数がツリーに在るか | 無い（`16384`・`MAX_TEXTURE`・`D3D11_REQ_*`・`GetMaximumBitmapSize` を grep して 0 件）。GPU の機器は `crates/wintf/src/ecs/graphics/core.rs` の `create_device_3d` が機能水準を指定せずハードウェアで作る。合成の絵は `areka-emo-present/src/display.rs` が `CreateBitmap` で原寸のまま上げる | 上限は機能水準 11 の一辺 16,384 を定数で写す（このクレートは `windows` に依存しない）。議題 2 |
| 広がりの予算に使える元からの頭打ち | 見つからない。近いもの: `areka-emo-atlas/src/limits.rs` の動く絵の画素の上限（2^28 画素）・`pack.rs` の頁 2048・`transition_judge_verdict.rs` の 1 フレーム 16,667 µs。ukadoc にも入れ子の深さ・数の決まりは無い（`insert` は「insert先でのinsert（入れ子）も可能」と書く＝https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/#insert ） | 予算の値は議題 1。推す案は 1 フレームの長さから測って決める |
| シェルの大きさから予算を導けるか | 面の数 × 参照の行の数は、30 段の菱形（31 × 60＝1,860）は切るが、ダミーの面を 10 万足すと 10^10 に膨らむ。読み込みの 2 か所（`reaches`・`regions_of`）は根ごと・辺ごとに全体をたどるので、長い鎖（10 万）で二乗＝10^10 になり、この予算では覆えない | 議題 1 の案イの欠点として書いた |
| 外形を前計算すると読み込みが重くなるか | 前計算は全部の面を根に外形をたどる＝ `shell_bounds` が今すでに行っている形と同じ。長い鎖では二乗になるが、表 1 回の予算の範囲に入れるので覆える | 外形の表は `bind_atlas` の最後で作り、予算は表 1 回の範囲 |
| 指摘 4 の直しの最小の場所 | 食い違いの根は `BindSteps::build` が `bind+random` に pattern0 の手順を作ること。`NestTable` はその手順を読むだけ・合成は早い戻りで使わない | `bind_steps.rs` の 1 か所を直せば `nesting.rs` のコードは変わらない（説明だけ） |
| 追記の範囲引きに要る形 | `SurfaceIndex` は `HashMap`（範囲引きができない）。このクレートの外の使い手は 0（grep）。`SurfaceImages` は `BTreeMap` | `SurfaceIndex` を `BTreeMap` に替える（議題 4） |
| 範囲の外で「未存在」の記録を数えるテスト | `crates/areka/src/placement/measure_bake_log_tests.rs` は単一の番号の追記だけを使う | 範囲の中の無い番号の `warn!` を出さなくしても数は変わらない（議題 5） |
| `ComposeError` を名指しで比べる範囲の外 | `EmptyComposition` だけ（`areka-emo-present/src/balloon.rs` ほか・`crates/areka/src/emo2_boot/assets.rs`・`offline_dump_tests.rs`・examples） | 足しても壊れない |
| 指摘 4 で期待が変わる seriko のテスト | `areka-seriko/src/parts_tests.rs` の `bind+random` を持つテストは 245・400〜460・576・777 行の付近（3 節の 395〜460 より広い） | `audit-seriko` との申し合わせの範囲を広げて書いた |
| 入れ子の再帰の在りか | 命令列（`plan.rs` の `flatten_surface` ⇄ `push_static_element_ops` ⇄ `push_contribution` ⇄ `bind_steps::push_bind_contribution`・`Level::flatten`）・外形・シェルの外形・当たり判定の持ち込み・見える部品・`claim_from` の 6 か所。`reaches` はもとから反復 | 6 か所を明示のスタックに。予算は広がりが指数になりうる 5 つの範囲（見える部品は各面 1 度なので要らない） |

### 11.2 合わせ方（design-synthesis）

- **まとめ（一般化）**: 深さと広がりの 2 つの危なさは 6 か所の走査で同じ形をしている。予算は 1 つの型（`budget.rs` の `Budget`）にまとめ、6 か所が同じ口で数える。走査そのものは、たどる辺と積み荷が違うので 1 つにまとめない。
- **作るか借りるか**: 依存を足さない約束なので、明示のスタックは `Vec` で書く（`stacker` などのクレートで深い再帰を逃がす手は取らない＝依存が増え、広がりも防げない）。上限の値は `windows` の定数を引かず写す（依存を増やさない）。
- **削る**: 当たり判定の 2 関数・`fold_shell`・`scale_test_support.rs`・`stride` 欄・`blit_rect.rs` の式の写し・`merge_animations`（指摘 9 で SSP が波括弧と同じなら）を消す。`PlanScratch` 以外の新しい型は足さない。外形の表は `BindSteps` と同じ形のリソースで、合成器は絵を持たない約束のまま。
- **案の選び方**: ギャップ分析 5 節の案 C。命令列の走査だけ作業場を 1 つの型にまとめる（明示のスタックにすると、まとめないと引数が増えるため）。

### 11.3 危うさ

- 命令列の走査を明示のスタックにすると、命令の並びを崩す恐れがある。今の命令列のテスト（`plan_ops_tests.rs`・`plan_nesting_tests.rs`・`plan_bind_tests.rs`・golden）を形を変えずに先に緑にしておき、直しの前後で同じ並びを確かめる。
- 1 つの段の中の `contains` の二乗（animation の数）は指摘に無い。かけ直しの audit が指したら直す（要件 4.2）。
- 予算の値は議題 1 の決めを待つ。決まるまで 3 の予算のタスクは値を置けない（走査の明示のスタックは先に入れられる）。
- 指摘 9 は開発者の許可が要る測りを待つ。許可が出なければ規則を書き残す形で閉じる。

---
読まなかった所は無い（範囲の 97 ファイルを全部読んだ）。テストもビルドも走らせていない（「今のテストは緑」は読みからの前提）。

## 12. 指摘 9 の SSP の測り（2026-11-05・設計の討議の中で）

開発者「都度計るのは困る。計りはまとめるか、今行え」を受けて、設計の段でまとめて測った。

- 測った SSP: `SSP/2.9.29 (20261103-7)`（`C:\wintools\ssp\ssp.exe --ghost emo`）。ゴースト `emo` に試しのシェル `areka_append_probe` を一時に置き、`\![change,shell,…]` で切り替え、`dump_surface`（面を 1 枚だけ初めの状態で描く）の画素で読んだ。終わったら元のシェルへ戻し、試しのシェルを消し、SSP を降ろした。
- 試しのシェル: 土台 `gray.png`（40×40 の灰）・`red.png`（20×20）・`green.png`（20×20）・`blue.png`（10×10）。descript に `seriko.use_self_alpha,1`（無いと左上の色が抜き色になり全部透明になった）と `sakura.bindgroup1.default,1`。

| # | 書いたもの | SSP の結果（画素） | SSP の記録 | 今の areka |
|---|---|---|---|---|
| 0 | `surface0{element0 灰, element1 赤 0,0}` ＋ `surface.append0{element1 青 20,20}` | 赤と青の**両方**を描く | Warning「Surface=0/Element=1 二重定義されています」 | 両方を描く（同じ） |
| 2 | `surface2{element0 灰, element1 赤}` ＋ 2 つ目の `surface2{element1 青 20,20}` | 赤と青の**両方**を描く | Warning「Surface=2/Element=1 二重定義されています」 | 後の行だけ（前の element1 を除く）＝**違う** |
| 1 | `surface1{animation1 bind・pattern0 赤・pattern1 緑}` ＋ `surface.append1{animation1.pattern2 青}` | 赤・緑だけ。追記の pattern2 は**描かない** | 記録なし | animation1 を丸ごと置き換える（pattern2 だけ）＋`warn!`＝**違う** |
| 3 | `surface3{animation1 bind・pattern0 赤}` ＋ `surface.append3{animation1.interval,bind・pattern1 緑}` | 赤だけ。追記の animation1 は**丸ごと捨てる** | **Error**「Surface=3/Animation=1 二重定義されています」 | 丸ごと置き換える（緑だけ）＝**違う** |
| 5 | `surface5{animation1 bind・pattern0 赤}` ＋ `surface.append5{animation1.pattern0 青}`（同じ pattern番号） | 赤だけ（追記を捨てる） | 記録なし | 丸ごと置き換える（青だけ）＝**違う** |
| 6 | `surface6{animation1 bind・pattern0 赤・pattern1 緑}` ＋ 2 つ目の `surface6{animation1.pattern2 青}` | 赤・緑だけ（後のブレスの行を捨てる） | 記録なし | pattern番号ごとに足す（赤・緑・青）＝**違う** |
| 8 | `surface8{element0 灰}` ＋ 2 つ目の `surface8{animation1 bind・pattern0 青}`（新しい番号） | 青を描く（足す） | 記録なし | 足す（同じ） |
| 9 | `surface9{element0 灰}` ＋ `surface.append9{animation1 bind・pattern0 青}`（新しい番号） | 青を描く（足す） | 記録なし | 足す（同じ） |
| 4・7 | `animation2` ＋ `sakura.bindgroup2.default,1` | 1 つのブレスでも描かない（7）＝bindgroup 2 の既定が効かない形だった | — | 測りに使わない（4 は 9 で測り直した） |

- **読み**: SSP 2.9.29 は、追記と 2 つ目の波括弧を同じ規則で扱う。⑴ element は同じ番号でも両方を足す（描く順は書いた順・警告）。⑵ animation は、すでにある番号の行（interval でも pattern でも）を後から書いても捨てる（**先勝ち**）。interval を書き直したときだけ Error を出す。⑶ 無い番号の animation は足す。
- **測れなかったもの**: collision（当たり判定）。画素に出ず、画面のクリック無しで読む口が MCP に無い。要件の討議の決めにより、collision は今のまま規則を説明に書き残す。
- **実在のシェルでの当たり**（`C:\wintools\ssp\ghost\` の 6 体を数えた・使い捨ての数え上げ）: 波括弧の同じ番号の element・animation の書き直しは `emily4`・`emo`・`claudia`・`R_POST_and_KOMAINU`・`dot_sakura`・`pasta-in-windows` のどれも 0 件。追記の同じ番号の animation は `emo` に 1 件（今 areka は丸ごと置き換え、SSP は捨てる＝見た目が違う）。
- **完了した spec との関わり**: 波括弧の「同じ名前の行は後勝ち」は `areka-P0-emily-ghost-verification`（2026-10-17・開発者の決め「足し合わせる」を受けた設計とタスク 6.1・`fold_merge_tests.rs` の `same_named_line_in_a_later_brace_wins`）で入った。「足し合わせる」は SSP と同じだが、「後勝ち」は SSP と違う＝SSP に合わせるなら、完了した spec の規則（種類 1）を変える。設計の討議の議題にする。

## 13. 面の番号の上限の SSP の測り（2026-11-05・開発者「SSPを確認してみて」）

- 測った SSP: `SSP/2.9.29`（`--ghost emo`・試しのシェル `areka_id_probe`・`dump_surface` に番号を渡して描けるかで読んだ）。終わったら元のシェルへ戻し、試しのシェルを消し、SSP を降ろした。

| 回 | 書いた面の番号 | SSP の結果 |
|---|---|---|
| 1 | 0・10・8192・8193・65535・65536・999999・1000000・2147483647・**2147483648・4294967295・4294967296・99999999999**、範囲 `surface3000000-3000002`・`surface.append2000000-2000002` | シェルを読む途中で**落ちた**（「Uncontinuable Exception Raised」・`STACKTRC.TXT`＝`ACCESS_VIOLATION` の書き込み・`SPSeriko::ProcessMultipleSurfaceArray` ← `ProcessMultipleSurfaceDef` ← `LoadSurfacePropertiesImpl`・落ちる前に約 1.5 GB を確保・スタックに文字列 `4294967295`） |
| 2 | 0・10・8193・65536・999999・1000000・9999999・100000000・**2147483647** | 落ちない。どの番号も `dump_surface` で描けた（比べに定義していない 12345 は「No such surface ID」） |
| 3 | 範囲 `surface3000000-3000002` と `surface.append3000000-3000002` だけ | 落ちない。3000001 を描けた |

- **読み**: SSP は面の番号を符号つき 32 ビット（0〜2,147,483,647）で受ける。2^31 以上の番号（回 1 の 4 つのどれか・切り分けていない）を書くと、読み込みで落ちる。100 万台・300 万台の範囲は普通に受ける。
- **測らなかったもの**: とても幅の広い範囲（例 `surface0-100000000`）を SSP がどう扱うか（固まる・落ちるおそれがあり、利用者の SSP で試すのは控えた）。
- 後始末: SSP が `C:\wintools\ssp\STACKTRC.TXT`（2026-11-05 20:07・約 100 KB）を残した。中に環境変数（認証の値を含む）が入っているので、消すかは開発者に任せる。
