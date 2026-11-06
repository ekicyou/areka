# ギャップ分析: audit-emo-text-state-actor（文字の層の状態・アクター・選択肢・窓の見直し）

- 測った時点: `main` `73c5b25d`（枝 `claude/kiro-start-audit-emo-text-state-actor-dcff96`・`f7335050`）・2026-11-06。このクレートを最後に変えたのは `0f2f3ecd`（`audit-parsers-sakura`・PR#428）で、棚卸㉟の測り（`18e4cb30`）より前。
- 物差し: `/ponytail:ponytail-audit`（ponytail 5.3.3＝5.3.2 と 1 字も違わない・10 節の 6）。作法のうち 2 つを外した（要件 1.2・1.3）。
  - 「最大 20 件」を外した。指摘は全件を下に並べ、省いた件は無い。
  - 「読まなかった所（Not checked）」を残さない。範囲の 121 ファイルを全部読んだ（3 節）。
- 番号は audit の番号（組をまたいで通し番号）。組は audit の 3 つ（必ず直す・直したほうがよい・できれば）のまま。
- この文書はコードを変えていない。決めごとは要件の討議に回す（11 節）。
- **この回でしたこと**: 1 回目の調べ（範囲の大きさ・守る約束・持ち込みの件・要件 1.5 の前提・タスクの数の見込み）に続けて、2 回目に作法の 2 つを外した audit を範囲の全部にかけ、指摘の全件を 4 節に書いた（兄弟の見直し 7 本と同じ形）。

## 1. 要約

- **何をするところか**: バルーンの文字の層のうち、配置と描画（`audit-emo-text-layout-draw` の持ち物）を除いた部分。台本の指令を受けてスコープごとの表示の状態を持つ（`state*`）、スコープごとの描き手・箱・スクロール・装飾の差し込みを毎フレーム動かす（`actor*`）、選択肢の当たりと強調（`choice*`）、指令の受け口（`sink*`）、描く面と窓の子（`surface*`）、印の面と矢印（`inline_marker*`・`marker_surface*`・`scroll_marker*`）、面ごとの上書き（`balloon_overrides*`）、公開の一覧と番人（`lib.rs`）。
- **想定した負荷**: 1 人の利用者の 1 台の机で、UI スレッドが毎フレーム文字の面を描く。ゴーストの台本とバルーン・シェルの設定ファイルはネットから入れたもので、**信用できない入力**として扱った（壊れた・意地の悪い入力でアプリが落ちたり、メモリを使い果たしたり、記録が止まらなくなったりしないこと）。
- **大きさ**: 121 ファイル（`src/` 68・`tests/` 40・`examples/` 12・`Cargo.toml`）。合わせて 45,385 行（要件の表の コード 10,832 行＋テスト 34,553 行と一致）。テストを除く本番のソースは 19 ファイル・8,731 行。
- **指摘の数**: 必ず直す 4 件・直したほうがよい 18 件・できれば 8 件＝**30 件**（テストの指摘は種類ごとに 1 件へまとめ、場所は全部を挙げた）。
- **見つかった型**: ⑴ 設定ファイルと台本の数字に上限が無く、足し算があふれる・巨大な面を作る・作れないと毎フレーム記録を出す・NaN で落ちる（1〜4）。⑵ 失敗の道で窓の子が残る・同じ警告が何度も出る・状態が消えない（5・7・11）。⑶ 毎フレームの作り直しと大きな確保（8〜10）。⑷ 失敗しえないテスト・名前と違うことを確かめるテスト（13・14）。⑸ テストの部品の写し（15・16）。⑹ 役目を終えたスパイクと古い説明。
- **要件 1.5 の前提（1 つの World に文字の層は 1 つ）**: 今の本番では成り立つが、コードの形では守られておらず、ゴーストの切り替えと 1 本のテスト（`actor_scroll_retain_tests.rs`）が前提を破っている。audit はこれを**指摘 6**（直したほうがよい）とした（6 節）。
- **持ち込みの件（要件 7）**: 今も直っていない（`src/actor_box.rs:236`・`:237`・`:427`）。壊れたら落ちるテストも無い。**指摘 1**（必ず直す）とした（7 節）。
- **大きさの見込み**: 直しは 26〜34 タスクの見込み。要件 8.1 の 20 を越えるが、兄弟と同じく 1 本のまま進める案を推す（9 節・議題 1）。

## 2. 受け持つ範囲（要件 1.1・1.3 の読む対象）

| まとまり | ファイル | 行 |
|---|---|---|
| `src/` の本番のソース | 19（`actor.rs`・`actor_attach.rs`・`actor_box.rs`・`actor_decoration.rs`・`actor_present.rs`・`actor_scroll.rs`・`balloon_overrides.rs`・`choice.rs`・`inline_marker.rs`・`lib.rs`・`marker_surface.rs`・`scroll_marker.rs`・`sink.rs`・`state.rs`・`state_anchor.rs`・`state_decoration.rs`・`state_route.rs`・`surface.rs`・`surface_window_child.rs`） | 8,731 |
| `src/` の `actor_*` のテスト（`actor_test_support.rs` を含む） | 23 | 10,324 |
| `src/` の `state_*` のテスト（`state_test_support.rs` を含む） | 13 | 4,946 |
| `src/` の `choice_*` のテスト | 5 | 2,837 |
| `src/` のそのほかのテスト（`sink_*`・`surface_*`・`balloon_overrides_*`・`inline_marker_*`・`marker_surface_*`・`scroll_marker_*`） | 8 | 2,175 |
| `tests/`（Rust 35・設定の検体 5） | 40 | Rust 14,271 |
| `examples/`（Rust 8・検体 4） | 12 | Rust 2,101 |
| `Cargo.toml` | 1 | — |

- 1 ファイル 1,000 行の上限の近く: `surface.rs` 995・`actor_choice_contract_tests.rs` 991・`choice_tests.rs` 980。ほかは 900 行未満（`actor_region_warn_tests.rs` 894・`actor.rs` 841）。上限を越えるファイルは無い。
- わざと置いた先送りの印（要件 3.1 の 2 種類目の候補）は 4 節の終わりの「決めごととして指摘に数えなかったもの」にまとめた。

## 3. 読んだ範囲（要件 1.3）

範囲の全 121 ファイルを読んだ。下請け 8 人で全行を読み、進行役が指摘の行を開いて確かめた。

- 本番のソース 19 ファイルと `Cargo.toml` は 2 人で全行を読んだ（`actor*`・`balloon_overrides.rs`・`inline_marker.rs`・`marker_surface.rs`・`scroll_marker.rs` の 10 ファイル／`choice.rs`・`lib.rs`・`sink.rs`・`state*`・`surface*` の 9 ファイルと `Cargo.toml`）。テストを読んだ 2 人も、テストの裏付けのために `state*`・`sink.rs`・`surface_window_child.rs`・`balloon_overrides.rs`・`inline_marker.rs`・`marker_surface.rs`・`scroll_marker.rs`・`choice.rs` を全行読み直した。
- テストと `examples/` は 6 人で全行を読んだ（`actor_*` のテストを 2 人・`state_*` とそのほかのテストを 1 人・`choice_*` のテストと `examples/` を 1 人・`tests/` を 2 人）。
- 進行役は、本番のコードの指摘（1〜11・19〜23・28 と 24 の本番の分）と例の指摘 18 は行を開いて確かめた。27 の古い説明は主なもの（`actor.rs:168-169,243-245`・`lib.rs:22-27`・`sink.rs:11`・`state.rs:87`・`surface.rs:7-8`）を開いた。17 はスパイクの頭の説明と、名指す所が完了した spec の文書だけであることを確かめた。テストの指摘は、失敗しえない・名前と違う型の主なもの（`actor_box_tests.rs:226`・`actor_runtime_frame_tests.rs:390-398`・`tests/scale_invariance_test.rs:117-155`・`tests/draw_readback_test.rs:236-248,399-402`・`state_cue_apply_tests.rs:200-229`・`choice_tests.rs:739-750`・`tests/staysee_balloon_fixture/bake.rs:128-140`・`actor_scroll_retain_tests.rs:113-176`）を開いて確かめ、写しの指摘は下請けの grep の結果を採った。テストは走らせていない（失敗しえないことは読みで確かめた）。
- `Cargo.toml` の依存はどれも使われている。説明は 1 か所だけ古い（指摘 27）。
- 設定の検体 9 ファイルはどれも使われている（`tests/fixtures/shell-balloon/` の 2 つは `shell_balloon_fixture_test.rs` と `areka-emo-present` のテストの部品が読む・`tests/fixtures/emo2-choice/` の 3 つは `choice_fixture_test.rs`・`examples/fixtures/emo2-vertical/` の 2 つは `examples/emo-text-layer/fixture.rs` と `tests/vertical_fixture_test.rs`・`tests/pipeline_test.rs`・`examples/fixtures/emo2-vertical-canon/` の 2 つはテストだけが読む）。

## 4. 指摘の一覧

各件の欄: **場所**／**何か**／**困る場面**／**いちばん小さい直し**／**範囲の外への波及**／**残す 3 種類に当たるか**（要件 3.1 の 1＝裁定に反する・2＝わざと置いた先送りの印・3＝SSP 互換・ukadoc 正典）。

### 必ず直す

#### 1. 箱の位置に大きさを足す計算があふれる（`actor_box.rs:236-237,427`・持ち込みの件）

- **何か**: 箱の置き場所の X・Y（`i64`・シェルの element定義の数字）に箱の大きさ（`u32`）を、上限で止めずにそのまま足す。236-237 は表示している箱の当たりの四角、427 は箱がサーフェスの画像からはみ出すかの判断。
- **困る場面**: `elementN,balloon,名前,9223372036854775807,0` と書いたシェルでは、試験の作りは足し算で落ち、出荷の作りは負へ折り返す（236-237 は四角が裏返り、427 は「はみ出していない」と読んで警告が出ない）。壊れたら落ちるテストは無い。
- **いちばん小さい直し**: 3 か所を `saturating_add`。テスト 1〜2 本（7 節）。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。近くの `actor_box.rs:197`（`site.x as f32`）は落ちず、窓の外の位置になるだけで、427 の警告がそれを知らせるので指摘に数えない。

#### 2. 矢印の位置に絵の幅を足す計算があふれる（`marker_surface.rs:137-138`）

- **何か**: 矢印を描く矩形の右端・下端を `i32` のまま `m.at.0 + m.image.width as i32` で足す。同じ矩形を当たり（`:194`）とマスク（`:236`）は `i64` で求めており、同じ規則が 2 通りに書いてある。
- **困る場面**: バルーンに `arrow0.x,2147483647` と書き、文字があふれて矢印が出ると、試験の作りは落ち、出荷の作りは右端が負へ折り返して矩形が裏返る（描いた絵とクリックを受けるマスクが食い違う）。
- **いちばん小さい直し**: `hit()` と同じく `i64` で足す（1〜2 行）。テスト 1 本。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 3. 文字の面・箱の面・印の面の大きさに上限が無く、作れないと毎フレーム作り直して記録を出す（`actor_present.rs:241-252,538-541`・`actor_box.rs:376-386`・`marker_surface.rs:231`・`actor_decoration.rs:205-208`・`scroll_marker.rs:268-273`・`surface.rs:96,99`）

- **何か**: 文字の面の物理寸は `ceil(validrect の寸 × k)`、印の面は `画像の寸 × k`。validrect の右辺・下辺の正の値は画像の大きさで切られない（`region.rs:286-287`・範囲の外）。箱の `size` は正の `u32` なら何でも通る。面を作るのに失敗すると描き手が入らず、次のフレームでまた作ろうとする。
- **困る場面**:
  - 300 画素の絵のバルーンに `validrect.right,20000` と書くと、swap chain を作れず、`error!` が毎フレーム（源と `crates/areka` の `scale_text.rs:302` で 2 行ずつ）プロセスの終わりまで出続ける。
  - `validrect.right,16000`・`validrect.bottom,16000` では作れてしまい、1 枚 約 1 GB の面を 5 枚（swap chain 2・描く面 2・読み戻し 1）確保する。
  - シェルの `balloon.x` の `size,16000,16000` は箱の文字の面で同じことが起き、矢印が出ると印の面と CPU のマスク（`vec![0; w*h*4]`・約 1 GB）も作る。確保に失敗するとプロセスが落ちる。
  - 箱の大きさが 2^31 以上だと `as i32` で負に化け、矢印が負の位置に出る。ちょうど `i32::MIN` では `w - 20` が試験の作りで落ちる。
  - `surface.rs:96,99` の `width * height * 4` は `u32` の掛け算（今は先の swap chain の作成が大きな寸で先に失敗するので届かない）。
- **いちばん小さい直し**: 面の矩形を画像（箱は箱の大きさとシェルの画像の重なり）の矩形で切ってから物理寸へ直す（窓は画像の大きさなので、見える画素は変わらない見込み）。作れなかった場所は入力が変わるまで作り直さず、記録は 1 回。箱の大きさと矢印の広がりは `i64` で読む。掛け算は `usize`。テスト＝大きな validrect・箱の大きさで面が画像の寸を越えない・作れない寸で記録が 1 回。
- **範囲の外への波及**: なし（公開の形は変わらない）。validrect の解決そのもの（`region.rs`）は `audit-emo-text-layout-draw` の持ち物で、こちらは面の大きさを決める所だけを直す。
- **残す 3 種類**: 種類 3 の確かめが要る（validrect が画像の外へ出るときの扱い・ukadoc）。直しの形は議題 5。

#### 4. 選択肢の帯の丈を決める式が、字の大きさが NaN になると落ちる（`choice.rs:137-140`）

- **何か**: `line_box_height.clamp(font_height, upper)` は、下限の `font_height` が NaN だと標準の `f32::clamp` の決まりで panic する。`font_height` は行の矩形の上下の差（`choice.rs:240`）。この関数は選択肢の有無に依らず、毎フレームすべての行で呼ばれる（`actor_present.rs:400` の `choice_ranges` → `line_bands`）。
- **困る場面**: 台本の `\f[height,1e38]`（`look.rs:757` は有限の正なら上限なしで通す・範囲の外）の行を横書きで 4〜5 行重ねると、行の上端・下端がともに無限大になり、差が NaN になって UI スレッドが落ちる。道筋は読みで確かめた（走らせてはいない。描画の側がそれより先に失敗するかは確かめていない）。
- **いちばん小さい直し**: `line_box_height.max(font_height).min(upper)`（有限の値では同じ答え・落ちない）。テスト＝`highlight_band_extent(f32::NAN, 1.0, 1.0)` が落ちない。
- **範囲の外への波及**: 字の大きさに上限が無い根は `look.rs`（`audit-emo-text-layout-draw` の持ち物）＝向こうへ知らせる（10 節の 3）。**残す 3 種類**: 当たらない。

### 直したほうがよい

#### 5. 箱の面を付けた後で描き手の組み立てが失敗すると、窓の子が残り、毎フレーム 1 つずつ増える（`actor_present.rs:301-332`・`actor_decoration.rs:49-73`・`surface_window_child.rs:75-115`）

- **何か**: 箱の場所は `attach_window_child` でシェルの窓に子の entity（swap chain の絵を持つ）を先に作り、その後で `build_actor_render`（`FontCatalog::new`・`ViewboxExecutor::new_shared`・`DWriteMetrics::new_shared`）を組む。後半が `Err` だと面は捨てられるが、`TextSurface` に `Drop` は無いので子の entity は World に残る。
- **困る場面**: 箱が初めて付く時に装置を失う・`CreateTextFormat` が箱のフォントの指定を断るなどで組み立てが失敗し続けると、毎フレーム `emo-text-box-surface` の子が 1 つずつ増え、GPU のメモリと窓の `Children` が際限なく伸びる。
- **いちばん小さい直し**: 面を要らない描き手と計測器を先に組み、面の装着を最後にする（または `Err` の道で `surface.release(world, slot)`）。テスト＝組み立てが失敗する場面で窓の子の数が増えない。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 6. 「1 つの World に文字の層は 1 つ」の前提を形で守っていない（`actor_present.rs:130-160`・`actor_scroll_retain_tests.rs:113-176`・要件 1.5）

- **何か**: 装置が作り直されたかを見る印 `SeenGraphicsCore` を World の資源（World に 1 つ）に置き、説明に「1 つの World に文字の層は 1 つの前提」と書く（6 節）。
- **困る場面**: ⑴ ゴーストの切り替えは同じ World で文字の層を作り直すので、新しい文字の層は前の文字の層が置いた印を引き継ぐ（説明の「初めて呼んだ回は印を控えるだけ」と食い違う・今は描き手が 0 なので害は無い）。⑵ `actor_scroll_retain_tests.rs:114-125` の `oracle_frame` は、動いている文字の層と同じ World に毎フレーム 2 つ目の文字の層を作って提示している＝前提はもうテストで破れている。装置の作り直しが起きると、先に回った方が印を書き換え、後の方は古い装置の描き手を使い続ける。⑶ 2 つ目の文字の層を置く spec（`multi-ghost-runtime`）が来たときに、番人のテストが赤にならない。
- **いちばん小さい直し**: 印を `TextLayerRuntime` の私的な欄（`Option<Tick>`）へ移す（6.4 の甲）。テスト＝同じ World に 2 つの文字の層を置き、装置を作り直すと両方とも描き手を作り直す。
- **範囲の外への波及**: なし（公開の形は変わらない）。**残す 3 種類**: 当たらない。扱いは議題 2。

#### 7. 箱のフォントの警告が、登録し直すたびに出る（`actor_box.rs:102-104,406-410`・`balloon_overrides.rs:68-73,106-113`・`draw.rs:257-273`）

- **何か**: 箱ごとに「1 度だけ」の門（`box_definition_warned`）が抑えるのは、折り返しの基準・無視した書き出し位置・選択肢の飾りの縮退の 3 つの警告だけ。フォントの警告（`font.height,0`・`font.name` が空・`font.bold,xyz` など）は解決の中で出るので、`set_box_layout` で 1 回、`register_box` のたびにもう 1 回出る。
- **困る場面**: シェルの `balloon.x` に `font.height,0` と書くと、読み込みで同じ警告が 2 回出る。その後も、台詞が終わって箱が隠れ（`hide_boxes`）、次の台詞の頭で印が下りて登録し直すたびに、拡大率が変わるたびに、箱が動くサーフェスの切り替えのたびに、また出る。
- **いちばん小さい直し**: 箱の解決を `set_box_layout` で名前ごとに 1 度だけ行って持ち、`register_box` はそれを使って印だけ押す（箱の大きさは置き場所で変わらないので持った値は古びない）。テスト＝登録し直しで WARN が増えない。
- **範囲の外への波及**: なし（`draw.rs`・`balloon_overrides.rs` の警告の文は変えない）。**残す 3 種類**: 当たらない。

#### 8. 何も変わらないフレームでも、配置・選択肢の範囲・キャンバス・装飾を毎回作り直す（`actor_present.rs:354-436,616`）

- **何か**: 最初の台詞の後、`present_frame` はプロセスの終わりまで毎フレーム走る。場所ごとに `layout_styled`・`choice_ranges`・`ContentCanvas::from_layout`・`decorate_canvas` を組み、その後で描き手が「変化なし」と答える。全文の先読みが無いか食い違うとき（`Basis::Arrived`）は、BudouX の区切りも届いた全文に毎フレームかける。
- **困る場面**: 500 字を出し終えたバルーンを 10 秒見せると、60 fps で同じ配置を 600 回、行・字の列・キャンバスの確保ごと作り直す。
- **いちばん小さい直し**: 見える字の数・場所の項目の数と消した回数・強調・手動の位置・配置の入力が前のフレームと同じなら、配置から装飾までを飛ばす。少なくとも `Basis::Arrived` の区切りは場所ごとに項目の数で控える。効き目は `tools/perf/` で測る。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 9. 字が 1 つ出るたびに、箱の当たりのマスクを面の大きさで確保し直す（`surface_window_child.rs:22-37,155-164`）

- **何か**: 字の矩形が変わるたびに `hit_cells_mask` が面の大きさの BGRA を `vec![0; w*h*4]` で作り、`AlphaMask::from_pbgra32` がもう 1 つ確保して全画素を読む。
- **困る場面**: 1600×1200 の箱の面で文字が 1 字ずつ出ると、1 字ごとに 7.7 MB を確保して 0 で埋め、190 万画素を UI スレッドで読む。
- **いちばん小さい直し**: マスクを持ち続け、wintf の `AlphaMask::regenerate_from_pbgra32`（再確保しない口）で作り直す。BGRA の作業場も面に持つ。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 10. 本番が使わない読み戻し用の写し先を、面ごとにずっと持つ（`surface.rs:263-264,375,455-481`）

- **何か**: 面を作るたびに面と同じ大きさの CPU 用の写し先（staging）を作るが、それを使う `read_back` は テストと `examples/` だけが呼ぶ。本番の読み戻し（`crates/areka/src/mcp/dump_balloon.rs:83,90` の `begin_read_back`）は呼び出しごとに自分で写し先を作る。
- **困る場面**: 1600×1200 のバルーンや箱の面 1 つにつき 7.7 MB の GPU のメモリを、使わないまま面の寿命の間持つ。
- **いちばん小さい直し**: `staging` の欄と作成を消し、`read_back` は呼ばれたときに作る（`begin_read_back` と同じ形）。
- **範囲の外への波及**: `read_back` の形は変えない（`crates/areka` のテストが呼ぶ）。**残す 3 種類**: 当たらない。

#### 11. スコープの状態が消えず、台本の `\p[N]` の数だけ増え続ける（`state.rs:437-446,501-568`）

- **何か**: スコープ（`\p[N]`・N は `u32`）ごとの状態は、全消し（`ClearAll`）では中身だけを空にして残し（画面から消すための決め）、消えるのは箱の場所だけ（`set_box_index`）。毎フレームの提示は全部のスコープを回る。
- **困る場面**: `\p[0]a\p[1]a…\p[9999]a` の台本の後、1 万のスコープの状態が残り続け、毎フレーム 1 万回たどり、描き手の無いスコープごとに「未装着」の答えを作る。違う N を使う台本を重ねるほど増える。
- **いちばん小さい直し**: 全消しの時に「前の全消しから中身が空のまま」で面も持たないスコープを外す（画面の消し方は今のまま）。テスト＝多くのスコープの後の全消し 2 回で数が戻る。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 12. 危ない分岐に、壊れたら落ちるテストが無い

- **場所と何か**:
  - `marker_surface.rs:269-280`: unsafe の `CreateBitmap`（`:294-304`）の前の「寸とバイト列が合うか」の確かめを通るテストが 1 本も無い。確かめを消しても全部緑。4×4 の絵に 4 バイトの列なら、確かめが無いと 64 バイトを 4 バイトの列から読む。
  - `choice.rs:692`: `cursor.style,none` の大文字小文字を問わない比べのテストが無い（`choice_style_resolve_tests.rs:63-70` は小文字だけ）。`== "none"` に変えると、`cursor.style,NONE` のバルーンが黙って黒い四角の印に変わる。
- **いちばん小さい直し**: 1 本ずつ足す（前者は実 GPU で短いバイト列を渡して `Err` と印の面が変わらないこと・後者は `"NONE"` を足す）。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 13. 失敗しえないテスト（偽の安心）

- **場所と何か**:
  - `actor_box_tests.rs:226-227`: 「箱を隠す印は `\c` では下ろさない」を確かめるのに、印を立てたのと別のスコープ（"1"）へ `\c` を送る。`actor.rs` の `Clear` の腕に印を下ろす行を足しても緑。
  - `actor_runtime_frame_tests.rs:390-398`: 描画回数の累計が「前以上」かを見る。累計は増えるだけなので、次のフレームで何も描かなくても緑。
  - 拡大率 k ごとの比べ: `tests/scale_invariance_test.rs:64-73,117-155,166-226,338-372,401-450,469-535`・`tests/staysee_balloon_fixture/scale.rs:196-235`・`tests/areka_balloon_fixture/wrapping.rs:158-200,264-305,348-361,409-431`。テスト自身が `TextSlotBinding::new` を作って `binding.image_size` を解決へ渡す＝k が入力に入らず、同じ純関数を同じ値で呼び直すだけ。説明（`scale_invariance_test.rs:117-119`・`scale.rs:194-195`）は「本番が物理寸を渡す形に変われば赤」と書くが、`actor_attach.rs:223,373` を `surface_size` に変えても緑（本番の向きは `tests/attach_wiring_test.rs:580-625` が実の presenter で守っている）。
  - `tests/draw_readback_test.rs:399-402,457-460,570-573,625-628`: 読み戻した列の長さから座標を作る（`:236-248`）ので、「インクが validrect の中」はいつも真。
  - `state_cue_apply_tests.rs:200-229`・`state_reveal_tests.rs:144-174`・`state_clear_count_tests.rs:106-116`: 同じ cue 列を新しい状態 2 つへ流して比べる。状態は純粋なので、`apply_cue` を何もしない形にしても緑。
  - `tests/attach_wiring_test.rs:433-463`: 「継ぎ目」の確かめを、テスト自身が `rt.state()` から `LayoutEngine::layout` を呼んで行う。文字の層が行を何も持たなくても緑。
  - `lib.rs:135-139`: 同じ `parse` を 2 回呼んで比べる（無損失と 2 つ以上に分かれる確かめは本物）。
  - `state_cursor_coord_parse_tests.rs:243-254`: `parse_cursor_coord`（`state.rs:230-275`）は添字も `unwrap` も持たず、落ちようがない。
  - 定数どうしの比べ: `tests/staysee_balloon_fixture/scale.rs:88-103`（1 本まるごと）・`:115,137-140,164,182-185,202,231-234,294-297`・`:158-162,255-259`・`assets.rs:94-98`・`bake.rs:162-166`・`faces.rs:133-140`・`wrapping.rs:307-311`、`tests/areka_balloon_fixture/definition.rs:117`・`wrapping.rs:267,302-304,411,428-430`、`tests/kero_menu_capacity_test.rs:530-533,545-546,583-586,759-763`、`tests/physical_extent_arbitration_test.rs:288-317,361-366,437-455`（決まった範囲を回った回数を数える）、`tests/decoration_readback/vertical.rs` の `VERTICAL_MODES` を自分の字面と比べる所。
  - 画素の数え上げ: `tests/areka_balloon_fixture/bake.rs:117-124,131-135`・`tests/staysee_balloon_fixture/bake.rs:130-137,142-147`。「総数＝矩形の面積」「内訳の和＝総数」は数え方から真。staysee は元の画像の面積と比べる確かめ（areka の `bake.rs:125-130` の形）を持たず、切り詰められた矩形を見逃す。
- **困る場面**: 本物の誤りが入っても緑のまま。数だけ多く、直すときの手間だけが増える。
- **いちばん小さい直し**: 消す。`actor_box_tests.rs:226` は宛先を "0" に。k のテストは k のループと誤った説明を消し、絶対値の確かめ（`scale_invariance_test.rs:237-321,374-394,514-522`）だけ残す。staysee の画素の数え上げは元の画像の面積と比べる形にする。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 14. 名前や説明と違うことを確かめているテスト

- **場所と何か**:
  - `choice_tests.rs:739-770`: 名前は「塗る帯とクリックを受ける帯は同じ列から出る」で、説明は「別々の列を渡す実装に戻すと赤」と書くが、`decorate_canvas` も本番の呼び手も呼ばない。`actor_present.rs:425` に別の帯の列を渡しても緑（本物の突き合わせは `choice_anchor_tests.rs:358`・`choice_decorate_tests.rs:557`）。
  - `choice_tests.rs:646-675`: 名前は「キャンバスの強調の導出と一致」だが `from_layout`・`decorate_canvas` を呼ばず、入力と期待が `:298-323` と同じ。
  - `choice_tests.rs:515-560`: 説明は「独立に計算」だが、期待値を `to_window_physical`（`choice.rs:419-440`）と同じ式の字面で作る。数字を書いた確かめ（`:456,489,565,598`）が同じ範囲を覆う。
  - `actor_route_tests.rs:243-266`: 名前は「`\c` は行き先の場所の強調だけを落とす」だが、箱の場所に強調を入れない。`actor.rs:387` を普通のバルーンだけ落とす形にしても緑。
  - `state_cue_apply_tests.rs:38-51`: 名前は「字の単位は書記素クラスタ」だが、入力（"a"・"あ"・"🦆"）はどれも 1 符号点。`state.rs:512` を `chars()` にしても緑（本物は `state_cluster_tests.rs:36-90`）。
  - `actor_choice_settle_tests.rs:418-423,431`: 「押す判定の間 WARN 0 行」と書くが、`press_hits`（`:368-374`）はテストの中の写しで記録を出さない＝決して落ちない。写しは本物（`crates/areka/src/input_events/balloon.rs:375`・`shell_box.rs:104`）の後の変更にも付いていかない。
  - `actor_runtime_frame_tests.rs:84-87,127-130,168-171,207-210`: `error!` 0 件を、同じ捕まえ方の窓の中に対照の `error!` を置かずに言う（このクレートのほかのテストは 0 件の主張に対照を 1 件置く）。
  - `state_cue_apply_tests.rs:241-255`: `choice_cue` は字数を `chars().count()` で数える。説明は「`cue` と同じ間隔」だが、`cue`（`state_test_support.rs:14`）と `actor_test_support.rs:111` は書記素で数える。修飾付きの絵文字の名札で食い違う（今の呼び手には無い）。
  - `tests/areka_balloon_fixture/definition.rs:253`: 名前は「読み込みで警告なし」だが、スコープ 1 は WARN 2 件を期待する（`:74,250`）。
- **困る場面**: 守っているつもりの分岐が壊れても赤にならない。
- **いちばん小さい直し**: 入力か比べ方を 1〜数行直す（例: 箱の場所にも強調を入れて `\c` の後に消えたかを見る・書記素の入力に結合文字を足す・対照の `error!` を置く・`cluster_count` で数える）か、名前を今の確かめに合わせる。重なるもの（`:646-675`・`:515-560`・`:38-51`）は消す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 15. `src/` のテストの組み立て部品が多くのファイルに写してある

- **場所と何か**（下請けの grep の結果）:
  - 実 GPU の World の組み立て（`CoInitializeEx`・`GraphicsCore`・`WucGraphicsResource`・差し込み口・資源の挿入の 10 行）: 元の `actor_test_support.rs:119-131` の写しが `actor_clear_atomicity_tests.rs:92-102`・`actor_decoration_frame_tests.rs:52-62`・`actor_runtime_frame_tests.rs:259-269,346-356,443-454`・`actor_scale_refresh_tests.rs:623-628`。
  - 箱の表の組み立て（`EmoWorld::build`＋`fold_boxes`＋問題なしの確かめ）が 10 か所（`actor_box_sync_tests.rs:70,608,681`・`actor_box_tests.rs:60`・`actor_region_collapse_tests.rs:99`・`actor_route_tests.rs:40`・`actor_lookahead_tests.rs:40`・`sink_probe_tests.rs:37`・`state_decoration_carry_tests.rs:71`・`state_route_tests.rs:55`）。`state_decoration_carry_tests.rs:90-96` の `name()` と `state_route_tests.rs:69-77` の `boxed()` は名前を引くたびに組み直す。
  - 箱の部品: `layout`（`actor_box_tests.rs:58-63`＝`actor_box_sync_tests.rs:68-73`・`:607-609,680-682` にも中身）・`fake_resolver`（`actor_box_tests.rs:76-83`＝`actor_box_sync_tests.rs:86-93`＝`actor_route_tests.rs:64-71`）・`emote`／`text`（`actor_box_tests.rs:90-96`＝`actor_box_sync_tests.rs:95-101`・`actor_route_tests.rs:73-75`）・`boxed`（`actor_box_tests.rs:66-73`）・`items_at`（`actor_box_tests.rs:116-120`＝`actor_box_sync_tests.rs:186-191`＝`actor_route_tests.rs:84-90`）・`actor_box_present_tests.rs:271-277`（取り込み済みの `name()` を書き直す）。
  - 画素の数え上げ: `actor_choice_contract_tests.rs` の中に 9 通り（`:241-251,376-391,393-408,410-420,589-645,881-885,887-902`・矩形の切り詰めが `:377-380,394-397,589-594,888-891`・`surface().expect().read_back().expect()` の鎖が約 12 回）、`ink_in_band` が `actor_clear_atomicity_tests.rs:195-205` と `actor_scroll_retain_tests.rs:101-111`（同一）。
  - 窓の子の列: `actor_region_collapse_present_tests.rs:52-57`・`actor_region_collapse_tests.rs:481-486`・`actor_scroll_present_tests.rs:102-107`・`marker_surface_tests.rs:50`・`surface_window_child_tests.rs:44`・`actor_box_present_tests.rs:329-334`（中に書いたもの）。
  - 記録の捕まえ方（対照の `error!` つき）: `actor_region_warn_tests.rs:142-157`・`actor_region_collapse_tests.rs:106-130`・`actor_region_collapse_present_tests.rs:61-78`。数の欄の読み取り: `actor_region_warn_tests.rs:160-166`・`actor_region_collapse_tests.rs:132-138`（同一）。`actor_region_warn_tests.rs` は対照の `errors, 1` の確かめを 14 回書く。
  - 警告の文の定数: ROP の縮退（`actor_box_sync_tests.rs:673-674`・`actor_decoration_tests.rs:238`）・折り返しの基準（`actor_box_sync_tests.rs:588`・`actor_region_warn_tests.rs:84`）・無視した書き出し位置（`actor_box_sync_tests.rs:591`・`actor_region_warn_tests.rs:402`）。
  - 指令の部品: `cue`・`choice`・`balloon`（`actor_choice_settle_tests.rs:22-40,57-59`＝`sink_talk_end_tests.rs:27-53`）・`pump_until_idle`（`actor_test_support.rs:43-48`・`sink.rs:312-317`・`sink_talk_end_tests.rs:56-60`）・記録の取り出し（`sink.rs:280,544-563`・`sink_talk_end_tests.rs:63-95`）・`REVEAL_INTERVAL`（`actor_scroll_retain_tests.rs:46` が `actor_test_support.rs:24` の値を写す）。
  - `state_*` の部品: `font`（`state_anchor_tests.rs:405`・`state_decoration_tests.rs:34`・`state_route_tests.rs:404`・`actor_box_tests.rs:98`・`actor_lookahead_tests.rs:77`）・`text`（`state_anchor_tests.rs:31`・`state_decoration_tests.rs:45`・`state_marker_tests.rs:22`）・`applied`（`state_anchor_tests.rs:47`・`state_marker_tests.rs:26`・`choice_anchor_tests.rs:66`・`actor_choice_settle_tests.rs:76`）・`spans`（`state_anchor_tests.rs:56`・`state_marker_tests.rs:44`）・`text`／`at`（`state_place_tests.rs:11,19`・`state_decoration_carry_tests.rs:169,173`）。
  - 配置の部品の書き直し: `scroll_marker_tests.rs:263,266-282,285-297`・`state_marker_tests.rs:118-134`（`layout_test_support.rs` の `IMAGE`・`model_rect`・`broken_lines` と同じ中身）。
  - 選択肢の部品: `region`（`choice_tests.rs:224-235`・`choice_decorate_tests.rs:18-29,495-506`・`choice_glyph_cells_tests.rs:17-33`・`choice_anchor_tests.rs:111-120`）・`seg`（`choice_tests.rs:251-257`＝`choice_decorate_tests.rs:57-63`）・`ChoiceSpan` の組み立て（`choice_tests.rs:30-39`・`choice_decorate_tests.rs:528-535`・`choice_glyph_cells_tests.rs:263-270`）・行の組み立て（`choice_tests.rs:8-27,238-248`・`choice_glyph_cells_tests.rs:36-54`）・同じ「は い ⏎ い い え」の検体（`choice_decorate_tests.rs:517-549`・`choice_glyph_cells_tests.rs:255-289`）。
  - そのほか: 面と合成器の組み立て（`surface_window_child_tests.rs:19-25`・`surface.rs:626`）・窓とマスクの組み立て（`surface_hit_cells_tests.rs:19-29,41-68`・`marker_surface_tests.rs:319-364`）。
- **困る場面**: 実 GPU の World の作り方・箱の名前の解き方・警告の文を 1 つ変えると、5〜10 か所を同じように直す。直し漏れた写しは、そのテストだけが古い作りで回る。`actor_choice_contract_tests.rs`（991 行）と `choice_tests.rs`（980 行）は、次の 1 本で 1,000 行の番人を越える。
- **いちばん小さい直し**: 部品を `actor_test_support.rs`・`state_test_support.rs`・`choice.rs` の `#[cfg(test)]` の共有（今の `bands_of` の形）へ 1 つずつ置き、写しを消して引く。画素の数え上げは `count_px(bytes, w, rect, pred)` 1 つ。上限の近い 2 ファイルは写しと 13・14・25 の消しで 100 行前後減る見込み。
- **範囲の外への波及**: `layout_test_support.rs`（`audit-emo-text-layout-draw` の持ち物）の 3 つを `pub(crate)` にするのは向こうと申し合わせる（10 節の 3）。**残す 3 種類**: 当たらない。

#### 16. `tests/` のテストの組み立て部品が多くのファイルに写してある

- **場所と何か**（下請けの grep の結果）:
  - `make_world_with_gpu`（同一・8 か所）: `attach_wiring_test.rs:46`・`box_attach_test.rs:76`・`choice_fixture_test.rs:204`・`decoration_readback/mod.rs:206`・`draw_readback_test.rs:100`・`emo2_fixture_e2e_test.rs:258`・`line_pitch_readback_test.rs:193`・`viewbox_scroll_test.rs:96`。
  - `spawn_reserved_slot`（名前の字だけ違う）: `choice_fixture_test.rs:219`・`decoration_readback/mod.rs:221`・`draw_readback_test.rs:499`・`emo2_fixture_e2e_test.rs:273`・`line_pitch_readback_test.rs:208`・`viewbox_scroll_test.rs:113`。`opaque_count`: `attach_wiring_test.rs:303`・`box_attach_test.rs:214`・`choice_fixture_test.rs:326`・`draw_readback_test.rs:231`・`emo2_fixture_e2e_test.rs:360`・`decoration_readback/mod.rs:200`。読み戻しの包み: `attach_wiring_test.rs:308`・`box_attach_test.rs:218`・`draw_readback_test.rs:251`・`line_pitch_readback_test.rs:302`・`decoration_readback/mod.rs:263-269`。`pump_until_idle`: `attach_wiring_test.rs:296`・`draw_readback_test.rs:224`・`pipeline_test.rs:65`・`viewbox_scroll_test.rs:166`。`cue`（間隔つき）: `pipeline_test.rs:49`・`scale_invariance_test.rs:91`・`draw_readback_test.rs:208`。
  - presenter の検体（`elem`・`shell_of`・`build_target_assets`・`show_surface`）: `attach_wiring_test.rs:63-143` と `draw_readback_test.rs:117-197`（1 バイトの違いだけ）・`box_attach_test.rs:91-158`（ほぼ同じ）。
  - 強調の帯の確かめと PNG の書き出し: `choice_fixture_test.rs:261-354,440-577,595-674` と `emo2_fixture_e2e_test.rs:309-400,519-658,676-755`（`line_pitch_readback_test.rs:314-404,592-709` にも似た部品）。写しはもうずれ始めている（`count_in_rect` の切り詰めが `choice_fixture_test.rs:278` にあり `emo2_fixture_e2e_test.rs:345` に無い）。同じ字の大きさ・帯で `highlight_band_offset` の答えを 2 か所で確かめる。
  - 検体の読み方: `EMO2` の置き場（`emo2_fixture_e2e_test.rs:65-78`・`kero_menu_capacity_test.rs:66-79`・`shipped_fixture_region_test.rs:175-188`）・バルーンのファイルの読み（`emo2_fixture_e2e_test.rs:82-94` だけが UTF-8 で読み、空の確かめも無い。`kero_menu_capacity_test.rs:84-114`・`shipped_fixture_region_test.rs:208-222`・`staysee_balloon_fixture/test_support.rs:138` は本番と同じ ANSI の規則。`shipped_fixture_region_test.rs:203-204` 自身が「本番と違う規則で読むと本番が違っても緑になる」と書く）・`menu.pasta` の本文の取り出し（`emo2_fixture_e2e_test.rs:112-124` と `kero_menu_capacity_test.rs:171-184` が別のやり方）・cue の写し（`emo2_fixture_e2e_test.rs:288`・`kero_menu_capacity_test.rs:188-219`・`staysee_balloon_fixture/script.rs:97-134`）・`png_native_size`（`shipped_fixture_region_test.rs:251`・`areka_balloon_fixture/assets.rs:17-29`・`staysee_balloon_fixture/assets.rs:13-24`）・DirectWrite の工場（`decoration_readback/mod.rs:123`・`kero_menu_capacity_test.rs:225`・`line_pitch_readback_test.rs:222`・`staysee_balloon_fixture/test_support.rs:167`）・本物のフォントの門（`decoration_readback/mod.rs:135` と `kero_menu_capacity_test.rs:234`・`line_pitch_readback_test.rs:239` が別の確かめ方）・`emo2-vertical` の読み（`pipeline_test.rs:466-483`・`vertical_fixture_test.rs:42-60,171-193`）・`read_decoded`（`staysee_balloon_fixture/test_support.rs:138-152`・`choice_fixture_test.rs:51`・`vertical_fixture_test.rs:51`・`pipeline_test.rs:474`）。
  - 2 つのバルーンの検体の組: α の数え上げ（`areka_balloon_fixture/bake.rs:36-137` と `staysee_balloon_fixture/bake.rs:45-149`・約 100 行）・COM の初期化（`areka_balloon_fixture/assets.rs:109-112`・`online.rs:35-38`・`bake.rs:49-54`・`staysee_balloon_fixture/bake.rs:61-66`）・読み込みの記録の確かめ（`areka_balloon_fixture/definition.rs:24,176-191,199-248` と `staysee_balloon_fixture/faces.rs:16,67-79,173-260`）・配置の道（`areka_balloon_fixture/wrapping.rs:116-130,158-231,234-259` と `staysee_balloon_fixture/wrapping.rs:67-81,96-106,117-135,268-294`・`script.rs:480-502`）・`fake_resolver`（`shell_balloon_fixture_test.rs:74-81`）。
  - 同じ期待値を 2 度書く: `staysee_balloon_fixture/bake.rs:42` の `EXPECTED_FACE0_FILES` は `test_support.rs:183` の `SCOPE_FACE0_FILES` の写し（`bake.rs:40-41` 自身が「二重に綴らない」と書く）。`areka_balloon_fixture` は面 0 のファイル名・大きさ・領域を `definition.rs:53-76`・`wrapping.rs:90-104`・`bake.rs:33`・`test_support.rs:60,64` に書く。
- **困る場面**: GPU の World の作り方（COM の初期化・`present_frame` に要る資源）を 1 つ変えると 8 か所を直す。直し漏れはそのテストの実行ファイルだけを壊す。検体の読み方が本番とずれた写しは、本番が違っても緑になる。
- **いちばん小さい直し**: `tests/` に共有の部品のファイルを 1 つ置き、`#[path]` で引く（`decoration_readback_test.rs:44` が前例）。バルーンのファイルは本番と同じ ANSI の規則の 1 つの読み方にする。期待値は `test_support.rs` から引く。
- **範囲の外への波及**: なし（`tests/` の中だけ・`lib.rs` の一覧には載らない）。**残す 3 種類**: 当たらない。

#### 17. 役目を終えた使い捨てのスパイクが残っている（`tests/viewbox_blit_spike.rs`・649 行）

- **何か**: 先頭の説明（`:1-9`）は「使い捨てのスパイク」「GO を確かめた後に本実装が入れば役目を終える」と書く。GO は 2026-07-12 に記録済み（`.kiro/specs/completed/areka-P0-emo-text-viewbox/tasks.md:158`）で、本実装（`ViewboxExecutor`）が在る。中身は `surface.rs`・`draw.rs` の面の作り方・読み戻し・描画を手で写したもの（`:109-314`）。
- **困る場面**: 「ずらした結果＝描き直した結果」は今、本物の `ViewboxExecutor` を通して `viewbox_draw_live_diff_tests.rs` が確かめている。スパイクは行の高さを変える spec のたびに手で直す手間だけを生む（前例 `line-height-canon`）。AA のこぼれ幅の計測（`:525,585-633`）は印字するだけで確かめない。
- **いちばん小さい直し**: ファイルを消す（−649 行）。消す前に `viewbox_draw_live_diff_tests.rs` が同じ不変を全部覆うかを確かめる（進行役は頭の説明だけを読んだ）。
- **範囲の外への波及**: なし（名指すのは完了した spec の文書だけ）。**残す 3 種類**: 当たらない。

#### 18. 例の `--hold` が自動判定を走らせ、目で見る前に自分で閉じる（`examples/emo-text-layer/demo.rs:44-46`・`drive.rs:73-81,327-355`・`examples/emo-text-layer.rs:57,197-238`）

- **何か**: 説明は「`--hold` は PASS/FAIL の自動判定を行わない目視の確かめ」と書くが、`drive_scenario` は `--hold` でも各チェックポイントを確かめ、最初の失敗で `finish()` が窓を閉じ、`main` が FAIL で終わる。
- **困る場面**: 説明（`:57`）は目視を 96 以外の DPI で行えと書き、同じ説明（`:45-46`）は 200% では C3 以降が FAIL すると書く。だから 200% のモニタで `--hold` を走らせると、約 1.1 秒で自分で閉じる。96 DPI でも 1 周目の終わり（約 3.8 秒）より前にダブルクリックで終えると「FAIL」と終了コード 1。
- **いちばん小さい直し**: `--hold` のときはチェックポイントと PASS/FAIL の印字を飛ばす。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 19. 面ごとの上書きを、記録のためだけにもう 1 度通す（`balloon_overrides.rs:78-82,106-113`・`draw.rs:295-304`・`look.rs:235-246`）

- **何か**: 同じ指定の列を `apply_font_tag` へ 2 回通す（1 回目は `probe_all` で記録だけ・2 回目は `look.rs` が答えを捨てて適用）。2 回は同じ順と同じ土台でなければならず、モジュールの説明自身が「ずれやすい」と書く。もう入力が 1 つ違う（試しは土台 `base` を、本番は層の複製を渡す。今それで答えが変わる指定は見つけていない）。
- **困る場面**: 片方だけ順や土台を変えると、記録した警告と実際の見た目が食い違う。
- **いちばん小さい直し**: 適用の側が `FontTagIssue` を返し、記録は 1 度だけにする。`probe_all`・試しの土台・そのための `from_balloon` を消す。
- **範囲の外への波及**: `look.rs`・`draw.rs` は `audit-emo-text-layout-draw` の持ち物＝形を変える前に申し合わせる（10 節の 3）。**残す 3 種類**: 当たらない。

#### 20. 本番の中の写し: 描き手の片付けと、登録の後半（`actor_box.rs:58-63,469-474`・`actor_present.rs:164-177,226-231`・`actor_attach.rs:221-236,356-408,425-427`）

- **何か**: 「印の面を消してから面を外す（または窓の子を消す）」が 5 か所にある（装置の変化の写しの `None` の腕は、`release` が窓の子も消すので要らない）。登録と拡大率の作り直しは「背景を引く → 解決 → 印を押す → 前の飾りの縮退を取る → 登録 → 入力を覚える」を 2 か所に書き、同じ順の注釈と `trace!` の文（`:356-360` と `:386-390`）も写してある。
- **困る場面**: 片付けに 1 つ部品が増えたとき、5 か所のどれかを直し漏れると面か窓の子が残る。登録の順を片方だけ変えると、飾りの警告が片方の道でだけ増える。
- **いちばん小さい直し**: `ActorRender::discard(self, world, slot)` を 1 つと、登録の後半の私的な関数を 1 つ置き、写しを消す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 21. 装置の失敗を記録する小さな関数が 4 か所に写してある（`surface.rs:51-70`・`marker_surface.rs:309-315`・`draw.rs:817,830-836`・`viewbox_draw.rs:318,331-337`）

- **何か**: `windows::core::Error` を `TextLayerError::Device` に直して `error!` する閉包が 4 ファイルにあり、説明は「`surface.rs` と同型」と書く。違うのは記録の文だけ。
- **困る場面**: 記録の欄を 1 つ足すと 4 か所を直す。
- **いちばん小さい直し**: `lib.rs` の `TextLayerError` の隣に `pub(crate)` の `device_err`／`none_err` を 1 つ置き、文は引数で渡す。
- **範囲の外への波及**: `draw.rs`・`viewbox_draw.rs` は `audit-emo-text-layout-draw` の持ち物＝こちらが共有の関数を置き、向こうが写しを消す形で申し合わせる（10 節の 3）。**残す 3 種類**: 当たらない。

#### 22. テストだけが使う公開の包み `handle_text_msg` と、事実と違う説明（`sink.rs:170-191`）

- **何か**: 本番の取り出しは `handle_text_msg_with`（`actor.rs:34,657`）を使う。`handle_text_msg` を呼ぶのは `sink.rs` のテストと `tests/pipeline_test.rs` だけ（`crates/areka` は 0）。説明は「UI ドレインの handler は本関数へ委譲」と書く。
- **困る場面**: 説明を信じてこの包みで取り出しを組むと、トークの終わり・選択待ちの始まり・先渡しが黙って落ちる。
- **いちばん小さい直し**: 消して、テストは `handle_text_msg_with(msg, |c, _| …, |_| Ok(()), |_| Ok(()))` を呼ぶ（または `#[cfg(test)]` にする）。
- **範囲の外への波及**: なし（公開だが `crates/areka` は使わない）。**残す 3 種類**: 当たらない。

### できれば

#### 23. 毎フレームの小さな無駄（`actor_present.rs:215`・`choice.rs:566-594,68-98`・`state.rs:587-591`・`state_anchor.rs:33-37`）

- **何か**: ⑴ 場所ごとに毎フレーム `ResolvedBalloonText` を丸ごと複製する（フォント名・代わりのフォントの列・2 層の見た目で数回の確保）。借りるだけで足りる。⑵ `decorate_canvas` は行が選択肢を持つかを見る前に字の列を複製する（選択肢 1 つと 50 行なら 49 の複製を捨てる）。⑶ `annotate_lines` は範囲の終わりを過ぎても全部の行を回る。⑷ `\q` と `\_a` のたびに、項目を頭から数えて字の数を出す（同じ数は表示の時刻の列の長さとして持っている・2 か所の写し）。
- **困る場面**: 長い台詞や選択肢の多いメニューで、毎フレームの確保と走査が増える。
- **いちばん小さい直し**: ⑴ 借りる ⑵ 範囲を先に出し、空でないときだけ複製 ⑶ `if line_start >= span_end { break; }` ⑷ `reveal.times().len()`。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 24. 要らない小さなコード

- **場所と何か**:
  - `actor.rs:446-449` `TextLayerRuntime::config()`: 呼び手 0（ツリー全体で `.config()` は 0 件）。
  - `marker_surface.rs:206-219` の `read_back`・`size`・`window_child`: 中の面へ渡すだけで、`marker_surface_tests.rs` だけが呼ぶ（本番は `surface()` を通る）。
  - `choice.rs:694`: `style_has_underline` の腕は `_` の腕と同じ答え（注釈は残せる）。
  - `choice.rs:186` `highlight_band_offset`（公開）: 本番は `band_offset_on_char_box` を通り、呼ぶのはテストだけ（`crates/areka` は 0）。
  - `state.rs:544,564`: 箱の場所の `\c` の回数も数えるが、読むのは普通のバルーンの場所だけ（`state.rs:694-699`）。
  - `actor_decoration_tests.rs:210-211`: 使わない `world` と差し込み口。
  - `actor_runtime_frame_tests.rs:26-29`・`sink.rs:246` の `with_log_cage`: `count_levels` の名前を変えるだけの包み。
  - `examples/emo-text-layer/drive.rs:860-865` `window_probe`: `reveal_probe` から 1 欄を除くだけ。
- **困る場面**: 読む・直す行が増えるだけ。
- **いちばん小さい直し**: 消す・呼び手を直に呼ぶ形にする。`highlight_band_offset` はテストのために残すなら `#[cfg(test)]`。
- **範囲の外への波及**: なし（どれも `crates/areka` は使わない）。**残す 3 種類**: 当たらない。

#### 25. 同じことを 2 度確かめているテスト

- **場所**: `actor_choice_contract_tests.rs:453-457,509-512,689-693`（`== 1` の直後の `< rows.len()`）・`:74-83`（`:559-564` の繰り返し・名前の「cursor の model から」も違う）・`:297-343`（`:547-783` が覆う）・`:10-31`（`:35-50` が覆う）。`actor_clear_atomicity_tests.rs:257-263,349-355,370-373`。`actor_lookahead_tests.rs:739-741`（前の項は後の項の部分）・`:406-409`（同じファイルの `warns()` の中身）・`:671-674`（`Rig::rows` の写し）。`state_route_tests.rs:402-518`（`state_decoration_carry_tests.rs:228-262,324-362,434-459` の写し・`:492-498` だけは箱の行き先で `reset_decoration` を通す唯一の確かめなので carry へ移す）。`state_decoration_tests.rs:485-548`（§7 の 4 本が §1〜§5 の繰り返し）。`choice_decorate_tests.rs:266-284`。`choice_style_resolve_tests.rs:107-162,164-180`（説明の「大文字小文字を問わない」を確かめない）。`choice_tests.rs:695-698,794,830-833,916-920,976-979`。`state_cue_apply_tests.rs:615`・`balloon_overrides_tests.rs:187-192`。`tests/areka_balloon_fixture/assets.rs:124-135` と `online.rs:43-51`・`definition.rs:157-170`・`wrapping.rs:196-206`。`tests/staysee_balloon_fixture/definition.rs:161-171` と `region.rs:195-206,244-249`・`region.rs:139-168` と `wrapping.rs:261-266`・`script.rs:332-337,361-364,469-476`・`scale.rs:207-214,277-290`・`wrapping.rs:202-207`・`region.rs:224-230`・`faces.rs:151-166`。`tests/physical_extent_arbitration_test.rs:216-238` と `:378-393`・`:186-192` と `:280-286`。`tests/decoration_readback_test.rs:101-113` と `decoration_readback/vertical.rs:169-181`。`tests/vertical_fixture_test.rs:116-124,133-140,156-160,275-284`。`tests/viewbox_scroll_test.rs:227-230,332-339`。`tests/shipped_fixture_region_test.rs:287-302` と `tests/kero_menu_capacity_test.rs:815-836`（読む関数が無いファイルで落ちる）・領域の値（`kero_menu_capacity_test.rs:368-387,676-680` と `shipped_fixture_region_test.rs:351-376`）。
- **困る場面**: 振る舞いを変えるとき、同じ期待を何か所も直す。
- **いちばん小さい直し**: 弱い方を消し、固有の 1〜2 行だけを強い方へ移す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 26. テストの余分な仕事（`tests/decoration_readback/mod.rs:242-285`・`tests/decoration_readback_test.rs:122-133,185-196`・`decoration_readback/vertical.rs:190-203`・`tests/shell_balloon_fixture_test.rs:140-155,257`）

- **何か**: 飾りの読み戻しは、飾りの無い同じ絵を鍵ごとに描き直し（書字方向ごとに 7 回・ほかに 3 回）、1 枚ごとに GPU の装置を作る（実行ファイル全体で約 150 回）。シェルの検体のテストは、名前を 1 つ引くたびにファイルの読み・解析・`EmoWorld::build`・`fold_boxes` をやり直す（1 本で 10〜40 回）。`:257` は `:197` の確かめの繰り返し。
- **困る場面**: 実 GPU のテストの走行が重くなる（記憶「高負荷テストを何度も回さない」）。
- **いちばん小さい直し**: 飾りの無い絵は書字方向ごとに 1 度。シェルの表は 1 本のテストで 1 度作って渡す。`:257` を消す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 27. 古くなった説明（コードの説明とテストの説明）

- **コード**: `actor.rs:168-169,243-245`（「本 task では空のまま」＝今は `actor_present.rs:450-475` が毎回埋める）・`lib.rs:22-27`（子の一覧に `state_route`・`state_anchor`・`actor_box`・`actor_scroll`・`marker_surface`・`viewbox_draw_underline`・`surface_window_child` が無い）・`surface.rs:7-8`（「`windows` を触るのは本モジュールと draw のみ」＝`viewbox_draw*`・`marker_surface.rs`・`surface_window_child.rs` も触る）・`sink.rs:11`（送れないときは `error!` だけ＝`Drop`（`:77-79`）は `warn!`）・`state.rs:87`（「両項とも整数 px」＝`\f[height,12.5]` も 0 以上の有限の行間も通る）・`choice.rs:774-777`（「design は部分色の既定を規定しない」＝ukadoc は `cursor.brush.color.*`・`cursor.font.color.*` の既定を 0 と書く。`unwrap_or(0)` は正典どおりで、理由の文だけが古い）・`sink.rs:170-191`（指摘 22）・`Cargo.toml:60`（`areka-emo-atlas` を使うのは `tests/attach_wiring_test.rs` だけ＝今は 11 のテストと例）。
- **テスト**: `actor_test_support.rs:1-4,101`（task 7.2 のテストは今は `actor_runtime_frame_tests.rs`・`runtime_tests` は無い）・`actor_scale_refresh_tests.rs:170-174`（「原寸が導出される」＝今は渡すだけ）・`actor_region_collapse_present_tests.rs:39`（`add_slot` は子を 1 つ足すだけ）・`actor_underline_present_tests.rs:278`（面 2・3 が書いてない）・`state_decoration_reset_tests.rs:18,143`（「旧い既定と同値なら追随」＝今は `set_look_layers` がいつも引き直す・`state_decoration.rs:585-589`）・`surface_window_child_tests.rs:140`（切り詰めは bevy でなく `surface_window_child.rs:97` の自前）・`state_marker_tests.rs:3-4`（読み手は今 `\![*]` を出す・`areka-parsers/src/sakura/decode.rs:213`）・`state_route_tests.rs:6`（部品を使うのは `state_place_tests.rs` だけでない）・`tests/choice_fixture_test.rs:11`（「本テストは 2 本」＝3 本）・`tests/viewbox_scroll_test.rs:74-77,127`（「実測 2」＝上限は 3・「pitch 15」＝14）・`tests/line_pitch_readback_test.rs:636-637`（今は 4 画素上へ開く）・`tests/staysee_balloon_fixture_test.rs:37`（`scale` は DirectWrite を使わない）・`tests/areka_balloon_fixture_test.rs:20-21`（WIC を使うのは `assets` だけでない）・`examples/emo-text-layer.rs:62,73`（「どの DPI でもそのまま PASS」＝同じ説明の `:45-46` は 200% で FAIL と書く）。
- **困る場面**: 説明を信じた読み手が誤った前提で直す（指摘 22 はこの型）。
- **いちばん小さい直し**: 説明を今のコードに合わせる。作業の経緯の文は消す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 28. 純粋層の走査の一覧が説明と合わない（`lib.rs:193-419,427-519`・`lib.rs:14-20`）

- **何か**: 説明は `actor_*` を結線層とするが、走査の一覧（`PURE_SOURCES`）には `actor_present.rs`・`actor_attach.rs`・`actor_scroll.rs` が載り、禁じる字は `windows` の綴りだけ。`actor_present.rs:9` は COM の装置を持つ `wintf::ecs::{GraphicsCore, WucGraphicsResource}` を取り込んで「純粋」として通る。逆に `windows` を使わない `inline_marker_tests.rs`・`underline_tests.rs` は「走査しない」側に載る。
- **困る場面**: `state.rs` に `use wintf::com::…`（D2D・DXGI の包み）を足しても `pure_layer_modules_have_no_windows_imports` は緑。`inline_marker_tests.rs` に `windows` を足しても緑。
- **いちばん小さい直し**: 禁じる字に `wintf::com` を足し（`wintf::ecs` の資源を取り込む `actor_present.rs` は走査しない側へ移すか、`ecs` だけは許すと説明に書く）、2 つのテストのファイルを走査する側へ移す。母数 83 を合わせて直す。
- **範囲の外への波及**: `underline_tests.rs` は `audit-emo-text-layout-draw` の持ち物だが、一覧は `lib.rs`（こちら）にある＝母数の行が動くので申し合わせる（10 節の 2）。**残す 3 種類**: 当たらない。

#### 29. 2 つの例の写しと、記録の無い失敗（`examples/emo-text-layer/fixture.rs:16-57`・`setup.rs:13-17,40-54,98-121`・`drive.rs:88-139,374-376,409-424,868-876`・`examples/emo-text-typewriter-demo.rs:84-110,183-244,306-342`）

- **何か**: 検体の置き場・ファイルの読み・バルーンの読み込み・失敗の打ち切り・面 0 の合成で窓の大きさを出す所・窓を作る所・GPU の準備と装着が、2 つの例に写してある。`drive.rs` の読み戻しの失敗などは `error!` を出さずに失敗の列へ積むだけ（`present_at` の説明は「log-first」）。
- **困る場面**: `sample-ghost-kit` や `ShowSurface` の形が変わると 2 つの例を同じように直す。例の失敗が記録に残らない。
- **いちばん小さい直し**: タイプライターの例が `#[path]` で `fixture`・`setup` を引く。失敗は `error!`＋積む 1 つの関数を通す。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

#### 30. 手書きの「キー,値」の読み（`tests/staysee_balloon_fixture/script.rs:577-588`）

- **何か**: `declared_color` が `key,value` の行を手で読む。同じテストの組（`definition.rs`）は `areka_parsers::kv::parse_kv` を使う。
- **困る場面**: `cursor.brush.color.r ,192`（カンマの前に空白）のような宣言を、本番の読み方と違う読み方で期待値にしてしまう。
- **いちばん小さい直し**: `parse_kv` で読む。
- **範囲の外への波及**: なし。**残す 3 種類**: 当たらない。

### 決めごととして指摘に数えなかったもの（`shortcut:`・`#[allow(dead_code)]`・名指しの継ぎ目）

- `scroll_marker.rs:229` の `shortcut:`（自動の範囲の行数を 1 画面とみなす・行の丈が揃わない台本でずれうる）。普通のバルーンの負荷は越えない＝audit の作法どおり指摘にしない。
- `surface.rs:269-270` の `sprite`（外形を追う将来の操作口）・`:241-243,276-277,436-440` の `fixed_overlay`（`\_b --option=fixed` の固定層の型の口）。どちらも `#[allow(dead_code)]` と理由の説明がある＝種類 2。
- `state_decoration.rs:232-235` の `unowned_vocab`・`:591-624` の `reset_decoration`、`choice.rs:679-680` の `resolve` の使わない引数 `default_font_color`、`actor.rs:269-271`・`actor_box.rs:138-142` の `box_font_dirs`。本番の呼び手は無いが、説明が開いている spec（`anchor-style-canon`・`choice-marker-styling`・`text-align-shadow-canon`・`talk-fast-forward`・`balloon-font-file`）の継ぎ目と名指す＝種類 2 の扱い（議題 6）。
- `actor.rs:597-603` の `set_surface_resolver`・`:613-633` の `arrange_for_test`（`#[cfg(test)]` のテストの継ぎ目）・`state_decoration.rs:228-230` の `look_layers`（テストだけが読む・公開の理由の説明あり）・`actor_decoration.rs:152-153`（直の登録口は次の作り直しで警告が 1 つ多い・説明あり・本番は使わない）。
- `surface.rs:548-598` の `copy_front_to_back_shifted` の `expect`（COM の約束の破れを致命として扱うと説明にある）。
- `state_decoration_reset_tests.rs:540-657`（§13・§14 のソースの字面の確かめ・限界を説明に書いてある）・`tests/kero_menu_capacity_test.rs:260-276` の `LegacyPitchMetrics`（古い行送りの対照・本番に置かない決め）・`tests/physical_extent_arbitration_test.rs` の誤差の数（81/81）の固定（裁定）・`tests/shipped_fixture_region_test.rs:42-50`（書き出し位置の生の値を確かめない決め）・`actor_scale_refresh_tests.rs:281-286`（ほかのテストと分け合う檻と説明にある）・`actor_choice_contract_tests.rs:842-856`（代わりのフォントでは通さない較正）。
- `actor_present.rs:2`・`actor_attach.rs:3` の「足す予定の spec」の見出し（`text-reveal-fade`・`balloon-markers`・`balloon-font-file`＝どれも開いている spec）。

## 5. 範囲の外から使われている所（要件 5.1 の「変えない公開の形」）

このクレートを使う範囲の外のクレートは `crates/areka` の 1 つだけ（`Cargo.toml` で依存するのも `crates/areka` だけ）。`crates/areka/src/` の 62 ファイルが `areka_emo_text` を名指す（要件の「約 60」と合う）。名指しの多いものは次のとおり（数は名指しの回数）。

- 受け持つ範囲の中: `state::TextLayerConfig`（22）・`actor::TextLayerRuntime`（20＋`::new`）・`actor` のモジュールそのもの（17）・`scroll_marker` の `WheelLines`・`StopReason`・`ARROW_STEP_LINES`（計 12）・`state::SpanKind`（5）・`sink::EmoTextSink`（2）・`actor::ResolvedBalloonText`・`actor::HitRectPx`（各 2）・`actor::spawn_emo_text`・`actor::present_frame`・`state::TextItem::Glyph`・`state::SurfaceKeyOutcome`・`surface::PendingReadBack`・`choice::to_window_physical`（各 1）。
- 受け持つ範囲の外（`audit-emo-text-layout-draw` の持ち物）: `place::PlaceKey`（10）・`place::TextPlace`・`look`・`draw::DEFAULT_BALLOON_BACKGROUND`・`writing::WritingMode`・`layout::LineRect`・`color::mix_disabled`・`region`。

`TextLayerRuntime` の公開の関数（`register_actor_view`・`refresh_actor_scale` など）は `crates/areka/src/emo2_boot/` の本番とテストから呼ばれている。直しで名前・引数・戻り値・欄を変えると要件 5.1 に触れる。足すだけなら触れない。

**4 節の指摘の直しで、`crates/areka` が使う公開の形を変えるものは 0 件**。消す公開の項目（指摘 22 の `handle_text_msg`・24 の `config()`・`MarkerSurface` の 3 つ・`highlight_band_offset`）は、どれも `crates/areka` から使われていない（ツリー全体を grep した）。形を残す `read_back` は `crates/areka` のテストが呼ぶ（指摘 10）。

## 6. 要件 1.5: 「1 つの World に文字の層は 1 つ」は今のコードで成り立つか

**結論: 今の本番では成り立つ。** ただしコードの形で守られてはおらず、守っているのは本番の組み立て方だけ。ゴーストの切り替えとテスト 1 本が前提を破っており、audit はこれを指摘 6 とした。

### 6.1 印の置き場と使い方

- `crates/areka-emo-text/src/actor_present.rs:130-134`: 印 `SeenGraphicsCore(Tick)` は World の資源（`#[derive(Resource)]`）で、説明に「World ごとに 1 つなので、1 つの World に文字の層は 1 つの前提（本番はゴーストの World ごとに 1 つ）」とある。私的な型で、ほかのファイルは使っていない。
- `actor_present.rs:140-160`（`drop_renders_after_device_change`）: `GraphicsCore` の変わった時刻を、World の印と比べる。新しければ印を今の時刻に書き換え（`:153-154`）、文字の層の描き手を全部捨てる。印が無い初めての回は印を置くだけで戻る（`:155-157`）。
- 呼ばれるのは `present_frame` の頭の 1 か所だけ（`actor_present.rs:63`）。

### 6.2 本番で文字の層が 1 つになる理由

- 本番で `TextLayerRuntime::new` を呼ぶのは `crates/areka/src/emo2_boot/mod.rs:475`（`wire_emo2_boot` の手順 2）だけ。もう 1 つの `crates/areka/src/emo2_boot/spine.rs:666` はテストの足場で、自分の `World` を作る（`spine.rs:440` の `make_world_with_gpu` の中の `:450`）。残りは全部テスト・テストの部品・`examples/`。
- できた文字の層は `Emo2Wiring` に入り（`mod.rs:633-641`）、`Emo2Wiring` は World の NonSend の置き場に載る（`mod.rs:631`）。この置き場は型ごとに 1 つで、入れ直すと前のものを置き換える。
- `present_frame` を本番で呼ぶのは `crates/areka/src/emo2_boot/frame/scale_text.rs:302` だけで、その `Emo2Wiring` の文字の層を渡す。
- `areka-emo-text` の中のテストと `tests/` を全部読んだ結果、2 つの文字の層を同じ World で回すのは `actor_scroll_retain_tests.rs:114-125` の `oracle_frame` だけ（動いている文字の層と同じ World に、毎フレームまっさらな文字の層を作って 1 フレーム提示する・`:140,174`）。装置の作り直しが起きないので今は緑。ほかのテストは 1 つの World に 1 つの文字の層。

### 6.3 前提が崩れる所・崩れかける所

1. **ゴーストの切り替え**: 切り替えは同じ World で起動し直す（`crates/areka/src/emo2_boot/ghost_switch.rs:850` → `ghost_session.rs:817` の `boot_ghost_strict` → `ghost_session.rs:863` の `wire_emo2_boot`）。新しい文字の層ができても、World の印 `SeenGraphicsCore` は前の文字の層が置いたまま残る。新しい文字の層の初めての回は「印が有る」側の道を通り、装置が変わっていれば描き手を捨てる。このときまだ描き手は 0 なので、今は害が無い。印の説明（「最初に呼んだ回は印を控えるだけ」）とは食い違う。
2. **1 つの World に 2 つの文字の層**: 今の本番には無いが、テスト 1 本（6.2）が既にしている。もし装置が作り直されると、先に回った文字の層が印を書き換え、後の文字の層は「変わっていない」と読んで古い装置の描き手を使い続ける。複数のゴーストを同時に動かす `multi-ghost-runtime`（v0.1.0 の余力の枠・時期は未定）が同じ World に 2 つ目を置く形を取ると、ここで崩れる。
3. 型で守っていないので、崩れても番人のテストは赤にならない。

### 6.4 直し方の候補（指摘 6・議題 2）

- **甲（推す）**: 印を World の資源から `TextLayerRuntime` の私的な欄（`Option<Tick>`）へ移す。文字の層ごとに自分の見た時刻を持つので、前提そのものが要らなくなる。切り替えの後も「初めての回は控えるだけ」が説明どおりになる。公開の形は変わらない（`crates/areka` に触れない）。差分は数行と、壊れたら落ちるテスト 1 本（同じ World に 2 つの文字の層を置き、装置を作り直すと両方とも描き手を作り直す）。
- **乙**: 今の形のまま、説明に「本番で 1 つになる理由」（6.2）と切り替えの残り（6.3 の 1）を書き足す。コードは変えない。2 つ目の文字の層を置く spec が来たときに直す。テストの `oracle_frame` は別の World へ移す。
- **丙**: 2 つ目の文字の層が来たら `debug_assert!` で落とす。落ちる道を足すので、記録の無い失敗の道を作らない決まり（要件 6.4）と合わない。推さない。

## 7. 要件 7（持ち込みの件・指摘 1）: 箱の位置の足し算があふれる

### 7.1 今のコード

- `src/actor_box.rs:236-237`（`refresh_shown_boxes`・箱の当たりの矩形）: `right: at(site.x + i64::from(def.size.0))`・`bottom: at(site.y + i64::from(def.size.1))`。`site.x`・`site.y` は `i64`（`crates/areka-emo-compose/src/boxes.rs:54-55` の `BoxPlacement`）、大きさは `u32`。
- `src/actor_box.rs:427`（箱がサーフェスの画像からはみ出すかの判断）: `site.x < 0 || site.y < 0 || site.x + w > sw || site.y + h > sh`。負の値は `||` の手前の比べで先に真になるので、あふれるのは大きな正の値のときだけ。
- `elementN,balloon,名前,9223372036854775807,0` では、debug は足し算で panic し、release は負へ折り返す（236 は矩形が裏返る・427 は「はみ出していない」と誤って読み、`warn!` が出ない）。
- 壊れたら落ちるテストは無い（このクレートに `i64::MAX`・`9223372036854775807` を使うテストは 0）。
- 兄弟の直し: `audit-emo-compose` は同じ形を `saturating_add` で直した（`.kiro/specs/completed/audit-emo-compose/research.md` 15 節の 31・テストは `crates/areka-emo-compose/src/plan_nesting_tests.rs:402` から）。

### 7.2 直し

- 3 か所とも `saturating_add`（要件 7.1 の「上限で止める・折り返さない」）。427 は `site.x.saturating_add(w) > sw` で、上限で止まった値はサーフェスの幅より大きいので「はみ出す」と正しく読み、`warn!` が出る。
- テスト: 兄弟のテストの形に倣い、`actor_box_tests.rs`（251 行）か `actor_box_present_tests.rs`（345 行）に、`i64::MAX` の置き場所で panic せず、矩形が裏返らず、はみ出しの `warn!` が 1 回出ることを確かめる 1〜2 本を足す。どちらのファイルも上限まで余裕がある。
- 近くの `actor_box.rs:197` の `box_origin`（`site.x as f32`）: audit は指摘に数えなかった（指摘 1 の欄）。panic はせず、窓の外の意味の無い位置になるだけで、その場面では 427 の警告が出る。

## 8. 要件と手持ちの対応（Requirement-to-Asset Map）

| 要件 | 手持ち | ギャップ |
|---|---|---|
| 1 洗い出し | 範囲の 121 ファイルを全部読んだ（3 節）・指摘 30 件（4 節）・要件 1.5 の答え（6 節） | なし（要件の討議で直す対象を確定する） |
| 2 全部直す | 指摘ごとの いちばん小さい直し（4 節） | **Missing**: 直し・テスト（指摘 1〜5・11・12 は壊れたら落ちるテストから） |
| 3 残す裁定 | 決めごととして数えなかったもの（4 節の終わり）・種類 3 の確かめの要る指摘 3 | **Unknown**: validrect が画像の外へ出るときの ukadoc の扱い（議題 5） |
| 4 直した後 | 全体テスト `tools/test-all.ps1`・`log-capture-kit`（`warn!` を数える）・実 GPU のテストの足場（`actor_test_support.rs` の `com_world` など） | **Missing**: 直した後の audit のかけ直し |
| 5 範囲の外 | 範囲の外から使われる公開の形の一覧と、直しで変わるもの 0 件（5 節） | **Constraint**: `crates/areka/` に触らない・公開の形を変えない（ウェーブ C18 の約束 ⑴）・`audit-emo-text-layout-draw` の持ち物に及ぶ 4・15・19・21・28 は申し合わせ（10 節の 3） |
| 6 守る決まり | 依存の追加は要らない（どの直しも標準の関数と既にある依存だけ） | **Constraint**: `surface.rs` が 995 行＝指摘 3・10 の直しで行が増えれば新しいファイルが要る（10 節の 2） |
| 7 持ち込み | 3 か所の場所と直し方（7 節） | **Missing**: テスト |
| 8 大きさ | 指摘 30 件・兄弟の実績（9 節） | 直しは 26〜34 タスクの見込み（議題 1） |

## 9. タスクの数の見込みと切り方（要件 8.1）

| 兄弟の見直し | 範囲の大きさ | 指摘の数 | タスク | 切ったか |
|---|---|---|---|---|
| `audit-emo-compose` | 97 ファイル | 20（1 回目） | 40 | 切らない（要件の討議で「1 本のまま」） |
| `audit-parsers-sakura` | 116 ファイル・コード 7,842・テスト 19,202 | 93 | 27 | 切らない |
| `audit-wintf-runtime` | 160 ファイル | 35 | 28 | 切らない（開発者「1 本のまま」） |
| `audit-wintf-graphics-world` | — | — | 23 | 切らない |
| `audit-seriko` | 60 ファイル前後 | 少ない（バグ 0） | 18 | 切らない |
| **この spec** | 121 ファイル・コード 10,832・テスト 34,553 | **30** | 26〜34（見込み） | — |

- 見込みの内訳: 必ず直す 4 件で 4〜5 タスク（どれもテスト 1 本つき）。本番のコードの直したほうがよい 5〜11・19〜22 の 11 件で 10〜12 タスク（8 は計測つき）。テストの 12〜16 で 5〜7 タスク（13・14 は消しが主・15・16 は部品のまとめを先に）。17・18 で 1〜2 タスク。できれば 8 件で 3〜5 タスク。かけ直しと全体テストで 2〜3 タスク。
- 要件 8.1 の 20 を越える。案: **切らない**（推す）。兄弟は 20 を越えても 1 本のまま進め、開発者もそれを選んだ。切ると同じクレートの同じファイル（`actor_present.rs`・`actor_box.rs`・`choice.rs` と多くのテスト）に 2 本が順に触ることになり、`lib.rs` の 1 本だけの席（新しいファイル・母数 83）の取り合いも生まれる。
- 切るなら（ほかの案）: **組で切る**＝1 本目は必ず直す 1〜4 と本番のコードの直したほうがよい（5〜11・17〜22）・2 本目は テストの直し（12〜16）とできれば（23〜30）。前後で同じテストのファイルに触るので、2 本目は 1 本目の着地の後。

## 10. 守る約束と気をつける所

1. **`crates/areka/` に触らない・公開の形を変えない**（ウェーブ C18 の約束 ⑴）: 4 節の直しで変わる公開の形は 0 件（5 節）。テストの部品の形（`state_test_support.rs`・`actor_test_support.rs`）は `#[cfg(test)]` の中なので、`crates/areka` のテストからは見えない（変えてよい）。
2. **新しいソースファイルの席**: このウェーブで emo-text にファイルを足せるのはこの spec だけ（約束 ⑷）。足したら `lib.rs` の `PURE_SOURCES`（`lib.rs:193`・母数 83 を `lib.rs:502` で固定）か `SOURCES_OUTSIDE_THE_PURE_SCAN`（`lib.rs:427`）に載せる（要件 6.3）。指摘 28 は一覧そのものを動かす。`audit-emo-text-layout-draw` はファイルを足さないが、消すことはありうる＝母数の行が動くときは申し合わせる。`surface.rs`（995 行）は指摘 3・10 で行が増えれば新しいファイルが要る（`staging` を消す 10 で減る分と相殺の見込み）。足したファイルが `.kiro/steering/structure.md` の emo-text の主要ファイルの一覧（337 行から）に当たるなら一覧も直す。`structure.md` は同じウェーブの `placement-crate-split` も直す（別の節）＝触る前にそのセッションへ知らせる。`tests/` に置く共有の部品（指摘 16）は `lib.rs` の一覧に載らない。
3. **`audit-emo-text-layout-draw` との境**: 受け持つファイルの重なりは 0。こちらの範囲から向こうの関数を呼ぶ所（例: `LayoutEngine::visible_window` を `scroll_marker.rs:227` が呼ぶ）と、向こうがこちらの型を使う所は、形を変える前に申し合わせる（要件 5.4）。この audit で申し合わせが要るもの: ⑴ 指摘 4 の根（`look.rs:757` の `\f[height]` に上限が無い）を向こうへ知らせる（こちらは `choice.rs` の式だけを直す） ⑵ 指摘 15 で `layout_test_support.rs` の 3 つの部品を `pub(crate)` にする ⑶ 指摘 19 で `look.rs`・`draw.rs` の上書きの適用が問題の列を返す形にする ⑷ 指摘 21 で `draw.rs`・`viewbox_draw.rs` の `device_err` の写しを消す（共有の関数は `lib.rs` に置く） ⑸ 指摘 28 の `underline_tests.rs` の一覧の移し。向こうの持ち込み（`layout_scan.rs` の改行の送りに有限かの検査が無い件）は向こうの持ち物で、こちらは触らない。
4. **同じウェーブの相手が変えない形**: `ecs::graphics::Visual`・`VisualGraphics`（`wintf-visual-tree`・この範囲の約 20 ファイルが使う）、`Arrangement`・`HitTest`・`AlphaMaskResource`・`LayoutScale`・`AlphaMask`（`audit-wintf-layout-widget`・`surface.rs`・`surface_window_child.rs`・`marker_surface.rs` が使う。指摘 9 は既にある `regenerate_from_pbgra32` を使うだけ）、`dola::cue::CueSink`（`audit-dola`・`sink.rs`）、`areka-actor` の公開の形（`audit-sylphya-actor`）。こちらはそれらの形を前提にしてよい。
5. **`doc/COMPAT_ARCHITECTURE.md`**: この範囲のコメントがこの文書を名指す。改名の spec（`doc-compat-architecture-rename`）は同じウェーブに無い＝名指しはそのままにし、名前を変えない。
6. **物差しの版**: 利用者全体に入っている ponytail は 2026-11-06 に 5.3.3 へ上がった（`~/.claude/plugins/installed_plugins.json`）。`skills/ponytail-audit/SKILL.md` は 5.3.2 と 1 字も違わない（比べて確かめた）＝要件の「5.3.2」の物差しと同じ。外す作法の 2 つ（「最大 20 件」＝同ファイル 84 行・「Not checked」＝同 98 行）も同じ場所にある。
7. **実 GPU のテスト**: `actor_*_present_tests.rs`・`marker_surface_tests.rs`・`surface_*_tests.rs`・`tests/*_readback_test.rs` などは実 GPU で回る。直しの確かめで重い走行を何度も回さず、2 回目からは赤が出たテストに絞る。

## 11. 決めごと（要件の討議へ）

1. **2 本に切るか**（要件 8.1・答えで作業が変わる）: 推す＝**切らない**（9 節）。指摘は 30 件・見込みは 26〜34 タスク。切るなら 9 節の組の案。
2. **要件 1.5 の前提の扱い（指摘 6）**（答えで作業が変わる・小さい）: 推す＝**6.4 の甲**（印を文字の層の私的な欄へ移し、前提を要らなくする・公開の形は変わらない・テスト 1 本）。乙（説明だけ直す）を選べば、`actor_scroll_retain_tests.rs` の `oracle_frame` を別の World へ移し、`multi-ghost-runtime` の brief に「印を文字の層ごとにする」を申し送る。
3. **持ち込みの件の直す範囲（指摘 1）**（答えで作業が少し変わる）: 推す＝**要件 7.2 の 3 か所（236・237・427）を `saturating_add` で直し、テストを 1〜2 本**。`actor_box.rs:197` の大きな値の f32 は audit が指摘に数えなかったので直さない。
4. **公開の形を変えないと直せない指摘が出たとき**（要件 5.2）: 今の 30 件には無い（5 節）。推す＝**直しやかけ直しで出たときは「足すだけ」の形で直す**（古い口を残して新しい口を足し、呼び手の書き換えは `crates/areka` の見直しの spec へ申し送る）。足すだけでも直らないものは、残すか後の spec へ回すかを 1 件ずつ議題にする。
5. **面の大きさの上限の形（指摘 3）**（答えで作業が変わる・種類 3 の確かめ）: 推す＝**面の矩形を画像の矩形（箱は箱の大きさとシェルの画像の重なり）で切り、作れなかった場所は入力が変わるまで作り直さず記録を 1 回にする**。窓は画像の大きさなので見える画素は変わらない見込み。ただし validrect が画像の外へ出るときの扱いを ukadoc で引いて確かめてから決める（読めなければ SSP の MCP で測る）。ほかの案: 面の寸を装置の上限（16384）で止めるだけ（約 1 GB の面を作る場面は残る）。
6. **決めごととして数えなかったものを残すか**（種類 2・答えで作業が変わる）: 推す＝**残す**（4 節の終わりの一覧）。`#[allow(dead_code)]` と理由の説明を持つ口・`shortcut:` の注釈・開いている spec を名指す継ぎ目で、どれも今の負荷で限界を越えていない。継ぎ目のうち `#[allow(dead_code)]` の印を持たないもの（`unowned_vocab`・`reset_decoration`・`default_font_color`・`box_font_dirs`）は、名指す spec が閉じたら消すと説明に書き足すだけにする。

## 12. 手間と危うさ

- **手間**: L（1〜2 週）。指摘 30 件のうち本番のコードの直しが 19 件、テストの直しは消しと部品のまとめが主。読む量が 45,000 行あり、テストの直しの量で決まる。持ち込みの件・指摘 2・4・6 は S。
- **危うさ**: Medium。`crates/areka` の公開の形を変えられない（今の直しは変えずに済む）・`surface.rs` が上限の 5 行手前・実 GPU のテストが多く走行が重い・指摘 3 は見え方に触れうる（議題 5）・指摘 8 の作り直しの省きは「変化なし」の条件を取り違えると古い字を見せる（計測とテストで確かめる）。持ち込みの件と指摘 2・4・6 は既存の形の中の直しで Low。

## 13. 直し方の選択肢（指摘が出た後の全体の進め方）

- **案 A（今のファイルの中で直す）**: 直しは今のファイルの中で閉じ、新しいファイルは上限を越えるときだけ（`surface.rs` など）。✅ 差分が小さく、`audit-emo-text-layout-draw` と重ならない。❌ 上限の近い 3 ファイルを直すと、結局ファイルを分けることになる。
- **案 B（テストの部品を先にまとめてから直す）**: 部品（指摘 15・16）を先に 1 か所へまとめ、その後で個々のテスト（13・14・25）を直す。✅ 写しを直す手間が 1 度で済む（兄弟の `audit-emo-compose` の 12 を先に置いた並びと同じ）。❌ 1 タスク目の差分が大きい。
- **案 C（混ぜる・推す）**: 必ず直す 1〜4 と本番のコードの直したほうがよい（5〜11・19〜22）を案 A で先に入れ、テストの整理（12〜16）は案 B の並びで後に置き、できれば（23〜30）を最後に。指摘の一覧が出たので、この並びを設計の段の出発点にする。

## 14. 調べが要る所（Research Needed）

1. validrect が画像の外へ出るときの扱い（指摘 3・議題 5・要件 3.2）。ukadoc で引き、読めなければ SSP の MCP で測る。
2. 指摘 17 のスパイクを消す前に、`viewbox_draw_live_diff_tests.rs` が同じ不変を全部覆うか（進行役は頭の説明だけを読んだ）。
3. 指摘 4 の道筋（大きな `\f[height]` で行の位置が無限大になる前に、配置・DirectWrite の側が止めるか）。直しは道筋に依らないが、テストの入力を決めるときに確かめる。
4. 指摘 8 の効き目（`tools/perf/` の計測）。
5. `multi-ghost-runtime` が 2 つ目のゴーストを同じ World に置く形か（6.3 の 2・その spec の brief で確かめる。議題 2 で甲を取れば要らない）。

## 15. 設計の段の調べ（2026-11-06・`/kiro-spec-design -y`）

- **調べの種類**: 軽い調べ（既にある仕組みの直し・新しい依存なし・外の資料の調べは要らない）。下請け 3 人で ⑴ 指摘 17 のスパイクと live-diff の照らし合わせ ⑵ 指摘 8 の「変化なし」の条件と指摘 23 ⑶ 指摘 3・5・9・10・2 の直しの形と `surface.rs` の行の見込みを読み、進行役が指摘 1・6・7・11・19〜22・24・28 の行を開いた。コードは変えていない・テストは走らせていない。
- **使ったスキルと手引き**: `kiro-spec-design`（同梱の `design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）。形は完了した兄弟 `audit-emo-compose` の design.md に倣った。

### 15.1 14 節の調べの答え

1. **validrect が画像の外へ出るとき（14 節の 1）**: 要件の討議で ukadoc を引いて済み（`requirements.md` の「面の大きさの上限」）。設計の段で分かったこと: 面の原点は `region.left/top` で、`canvas.rs:341`・`choice.rs:336,421,467`・`viewbox_draw_render.rs:375-404`・印の描き方がこれを前提にする。右と下は描く式に触らずに切れる（描き手は `surface.size()` から計画する・`viewbox_draw_render.rs:70`）が、左と上を切るには原点のずれを向こうの持ち物まで通す必要がある。そこで右と下だけを切り、左と上の大きな負の値は面の大きさの上限（一辺 16,384・面積 2^25・兄弟 `audit-emo-compose` の `MAX_EXTENT`・`MAX_AREA` と同じ）で止める形を推す（design.md の設計の議題 1）。
2. **指摘 17 のスパイク（14 節の 2）**: スパイクの確かめ 7 つを照らした。⑴ 自分の読み戻しの `RowPitch >= stride`（`:299`）＝スパイク自身の写しの確かめ ⑵ 比べる所にインクがある（`:556`）＝`viewbox_draw_live_diff_tests.rs:203` ほか ⑶ A に描いて B へずらす＝B に直に描く（許し 4 画素・`:560`）＝live-diff の `checkpoint` が面の全体を `assert_eq!`（`:216`・許しなし）で、本物の `ViewboxExecutor` を通して比べる（横書き `:377`・縦書き右から `:384`・スパイクに無い縦書き左から `:393`・`viewbox_draw_scroll_retain_tests.rs:322`） ⑷ 全角と半角の複数行＝live-diff は全角の 10・20 px・比例の書体（`:481`）・絵文字（`:540`）・検体（`:617`）。半角の英数字をずらして比べる場面だけは無いが、ずらしの不変は字に依らない ⑸ AA のこぼれ幅（`:525,585-633`）＝印字だけ・値は `viewbox_dirty_tests.rs:428` が固定 ⑹ 計測した行にインク＝スパイク自身の確かめ ⑺ GO／NO-GO の印字。**消しても本番を守る確かめは失われない**。`Cargo.toml` に `[[test]]` は無い。古くなる説明は `viewbox.rs:333` と `viewbox_draw_live_diff_tests.rs:455`（どちらも向こうの持ち物・説明だけ）。
3. **指摘 4 の道筋（14 節の 3）**: 直し（`max`／`min`）は道筋に依らず、テストは純関数を NaN で呼ぶので、道筋は確かめないまま進める。根（`look.rs:757`）は向こうへ知らせる。
4. **指摘 8 の効き目（14 節の 4）**: `present_frame` だけを狙う計測の場面は `tools/perf/` に無い。全体の計測（`tools/perf/invoke-perf-run.ps1 -Profile short`）で前後を 1 回だけ測って記録に残す（効かなくても戻さない）。「変化なし」の条件は 15.2。
5. **`multi-ghost-runtime` の形（14 節の 5）**: 指摘 6 で印を文字の層ごとにするので要らない。

### 15.2 指摘 8 の「変化なし」の条件（下請けの読み）

- 飛ばす所（`actor_present.rs:354-426`）が読む値: 届いた状態の `items`（消すまで増えるだけ・`clear_content` でだけ戻る）・`reveal.visible(t)`・`choices`（`\_a` の開きは項目を増やさない・`state_anchor.rs:39`）・字の見た目の列・`current_look()`（`\f`・`set_look_layers`・`set_box_traits`・`copy_script` で項目を増やさずに変わる）・`settled`（強調の絞りだけに効く）・`layout_input`（登録し直しで描き手を捨てずに替わる・`actor_attach.rs:181`・`actor_box.rs:452`）・全文の先読み（入れ直し・`forget_boxes`・`\c` で区間の番号が進む）・`choice_hover`・計測器（`ActorRender` の寿命の間変わらない）。
- 版の数（revision）は今どこにも無い。使える単調な値は場所ごとの消した回数（`state.rs:440`）・項目の数・選択肢の数・`settled`。
- 先読みの入れ直しを呼ぶのは `actor.rs:433,609,626`・`actor_box.rs:79` の 4 か所だけ＝文字の層の側に数を 1 つ置けば `lookahead.rs`（向こうの持ち物）を変えずに済む。
- 鍵の形は design.md の「配置の覚え」。届いた字の区切りの覚えは `(消した回数, 項目の数)` で足りる（項目は消すまで増えるだけ・表を替えると箱の描き手は捨てられる）。
- 指摘 23: 4 つとも行を開いて確かめた（`actor_present.rs:215`・`choice.rs:568-594`・`choice.rs:79-95`・`state.rs:587-591`／`state_anchor.rs:33-37`）。直しの形は design.md。

### 15.3 指摘 3・5・9・10・2 の読み

- 箱の場所の `binding.image_size` は箱の大きさで、シェルの画像の大きさは `DesiredBox.surface_size`（`actor_box.rs:286,386`）にしか無く、はみ出しの警告に使った後で捨てられる＝文字の層に覚える欄が要る。
- 作れない面の記録は源（`surface.rs`・`marker_surface.rs` の `device_err`）と `crates/areka` の `scale_text.rs:302` の 2 か所で出る。覚えで 2 回目から `Ok(false)` を返せば、`crates/areka` を変えずに両方とも 1 回で止まる。
- 指摘 5: `TextSurface` に `Drop` は無い（`Drop` は `sink.rs` だけ）。`build_actor_render` の失敗しうる 4 つ（`dwrite_factory`・`FontCatalog::new`・`ViewboxExecutor::new_shared`・`DWriteMetrics::new_shared`）はどれも面を要らない＝装着を最後へ回せる。呼び手は 1 か所。
- 指摘 9: `wintf/src/ecs/widget/bitmap_source/alpha_mask.rs:100` に `pub fn regenerate_from_pbgra32(&mut self, pixels: &[u8], width: u32, height: u32, stride: u32)`。`AlphaMaskResource` には `set`・`set_shared(Arc)`・`mask()` だけ＝`areka-emo-present` の `presenter/budget.rs:320-345` の 2 枚を入れ替える形に倣う。
- 指摘 10: `read_back` の呼び手はテストと例と `crates/areka` のテスト 2 本（`spine_talk_close_tests.rs:28`・`dump_balloon_gpu_tests.rs:57`）だけ。本番の `dump_balloon.rs:83,90` は `begin_read_back`（その場で写し先を作る）。
- `surface.rs` の行: 10 で約 −4・3 の掛け算の書き直しは 0・9 は `surface_window_child.rs` 側＝約 991 行で新しいファイルは要らない。
- 指摘 2: `marker_surface.rs:137-138` の `i32` の足し算を確かめた（絵の幅の `as i32` も負に化けうる）。

### 15.4 進行役が開いた行

- 指摘 1: `actor_box.rs:236-237,427` を確かめた。
- 指摘 6: 印は `actor_present.rs:130-160` の私的な資源で、使うのは `drop_renders_after_device_change` だけ。
- 指摘 7: `set_box_layout` は `ResolvedFont::resolve_with_background` で見た目の 2 層だけを解き（`actor_box.rs:102-104`）、`register_box` は登録のたびに `ResolvedBalloonText::resolve_with_background` で全部を解く（`:406-410`）＝警告は表を入れたときに 1 回、登録し直しのたびにもう 1 回。解決は拡大率に依らない。
- 指摘 11: 提示の走査は「中身があるか面を持つ」場所だけを描く（`actor_present.rs:65-79`）。`ActorTextState` は `PartialEq` を持つ。登録で入る見た目の 2 層（`set_look_layers`・`state_decoration.rs:585-589`）は場所の中身に載るので、場所を外す条件は「作りたてと同じ」にする（見た目を入れたスコープは外れない）。消した回数 `clears` は `crates/areka` の `balloon_visibility.rs` が `clear_count` の変化で見る＝外さない。
- 指摘 19: 本番の適用（`look.rs:235-246` の `apply_overrides`）は答えを捨て、記録は `balloon_overrides.rs` の試しの通し（`probe_all`）だけが出す。`balloon_overrides::overrides` の呼び手は `draw.rs:304` だけ（`crates/areka` は 0）。試しの土台（`draw.rs:295-302`）はほかに使い道が無い。
- 指摘 21: 写しは `surface.rs:51,61`・`marker_surface.rs:309`・`draw.rs:817,830`・`viewbox_draw.rs:318,331`。emo-text に記録の文を数えるテストは無い（同じ文を数えるのは `areka-emo-present` の別の文）。
- 指摘 22・24: `handle_text_msg`・`highlight_band_offset`・`config()`・`balloon_overrides` は `crates/areka` から使われていない（grep し直した）。
- 指摘 28: 走査の一覧の中に `wintf::com` を取り込むファイルは 0。`wintf::` を取り込むのは `actor_present.rs:9` の `wintf::ecs::{GraphicsCore, WucGraphicsResource}` だけ。`inline_marker_tests.rs`・`underline_tests.rs` は `windows` を取り込まない。

### 15.5 まとめの見方（design-synthesis）

- **広げて 1 つにしたもの**: 「作れない面・上限を越える面は入力が変わるまで作り直さず記録 1 回」を、文字の面と印の面で同じ形（物理寸の覚え）にした。面の大きさの上限の値は兄弟の合成の上限と同じにした。
- **作るか借りるか**: マスクの使い回しは wintf の既にある口と `areka-emo-present` の形を借りた。新しい依存は無い。
- **削ったもの**: 指摘 8 の計測の場面は作らない（全体の計測で 1 回）。指摘 21 の記録の文は引数にせず 1 つに揃えた（呼び出しの箇所を変えずに済む）。指摘 11 の「前の全消しから空のまま」の印は持たず、「作りたてと同じ」の比べで済ませた（新しい欄が要らない）。左と上の切りは上限で覆い、描く式に原点のずれを通さない（設計の議題 1）。
- **新しいファイル**: `src/actor_surface_limit_tests.rs`・`src/actor_frame_memo_tests.rs`・`src/choice_test_support.rs`・`tests/support/mod.rs`。本番のソースは足さない。

## 16. 設計の討議（2026-11-06）

- 設計の検証は GO（`design-validation.md`）。大事な問題 2 件は自明な直しとして design.md に入れた: ⑴ 作れなかった面の覚えは、入力で答えが決まる失敗（上限越え・その寸での作成の失敗）だけを覚え、資源（Compositor・`GraphicsCore`）が無いか無効な一時の失敗は覚えずに毎フレーム試し直す。`WucGraphicsResource` が作り直されたときも覚えを消す。⑵ 配置の覚えの鍵から `#[cfg(test)]` の `lookahead.number` を外し、消した回数と先読みの入れ直しの数で足りる理由を書いた。
- 設計の議題 5 件は、開発者「承認」で全部推奨案のとおりに決めた（design.md の終わりの節）。

## 17. 指摘 8 の効き目の計測（実装の段・タスク 3.8 と 3.10）

全体の計測（`tools/perf/invoke-perf-run.ps1 -Profile short -Build release -AutoQuiet`・emo2 の検体・32bit の SHIORI ヘルパーあり）を、配置の覚えを入れる前と後に 1 回ずつ回す。効かなくても直しは戻さない（design.md「配置の覚え」）。判定は `judge-perf.py --mode verdict --build release --emit-metrics`。

| 項目 | 前（3.8・2026-11-06・`f9a8bf04`） | 後（3.10・2026-11-06・`44a19223` に 3.10 の直しを足した作業ツリー） |
|---|---|---|
| 出力先 | `target\perf\before-3.8`（このワークツリーの下） | `target\perf\after-3.10`（このワークツリーの下） |
| 判定 | 不合格（終了コード 1・判定式 ⑵ と ⑷a） | 不合格（終了コード 1・判定式 ⑵ と ⑷a） |
| 定常のアイドル CPU の平均（1 コア換算） | 10.69%（上限 3.0%） | 16.06%（上限 3.0%） |
| CPU の p50／p95／最大 | 10.87%／23.35%／26.44% | 15.53%／32.62%／34.21% |
| 発話の頂の CPU | 26.44% | 32.62% |
| 進行境界スキップ（定常／全区間） | 31 件／39 件（全部 loop_ticker） | 26 件／29 件（全部 loop_ticker） |
| コマ適用間隔の p95 | 161.613ms（109 本・上限 172.5ms） | 171.574ms（118 本・上限 172.5ms） |
| 表示用バッファの新規確保 | 0 件 | 0 件 |
| 定常の適用の回数・長さ | 255 回・359.9 秒（CPU の点 24） | 317 回・359.3 秒（CPU の点 24） |

- 前の値の不合格は、この直しより前から持っている状態の記録で、ここで直すものではない。
- 後の値（3.10）: 配置の覚え（3.8）と届いた字の区切りの覚え（3.10）を入れた release で、前と同じ根・同じ形で 1 回回した。定常のアイドル CPU の平均は 10.69% から 16.06% に上がったが、定常の適用の回数も 255 回から 317 回に違っており、1 回ずつの計測ではこの差が直しによるものとは言えない。
- 効かなくても直しは戻さない（design.md「配置の覚え」）ので、直しはそのまま残す。
- 前も後も判定式 ⑵ と ⑷a で不合格で、これはこの spec の直しより前から持っている状態（3.8 の記録と同じ）。

## 18. 例の `--hold` を 200% の画面で手で走らせた記録（実装の段・タスク 5.7）

例の直し（4.6・5.2・5.5・5.7）が済んだ作業ツリーで、排他テストの机を取って 1 回走らせた（design.md の議題 3・案ア）。

| 項目 | 結果 |
|---|---|
| 日時 | 2026-11-07 00:24:35（日本時間）起動・記録の時刻は UTC で 2026-11-06T15:24:35Z から |
| コマンド | このワークツリーの `target\debug\examples\emo-text-layer.exe --hold`（`cargo build -p areka-emo-text --example emo-text-layer` でビルドした物を直に起こした・pid を取るため。`__COMPAT_LAYER` は付けない） |
| 画面 | 主のモニタが 200%（dpi=192）。窓の位置 (320,160)・(320,416) はどちらもこのモニタの上 |
| DPI の行 | `実モニタ DPI と合成スケール … balloon="sakura" dpi=192 scale_k=2.0`・同じく `balloon="kero" dpi=192 scale_k=2.0`。ShowSurface の行は両方の target で `k=2.0 window_dpi=Some((192, 192))` |
| 自分で閉じないこと | 「hold — talk をループ再生（balloon 上ダブルクリックで終了）」の行が約 3.8 秒おきに 50 回（15:24:39Z〜15:27:46Z・約 3 分）。記録の最後の行は 15:27:47Z（進行役が止める直前）。この間、窓は自分で閉じなかった |
| 判定の印字 | `PASS`／`FAIL` の行は 0 件（`--hold` は判定を飛ばす・指摘 18） |
| 記録の段 | ERROR は 0 件。WARN は文節の折り返しの「先渡しが無い——届いた字だけで区切って配置する」（`areka_emo_text::lookahead`・\0 と \1 の行の頭ごと）だけ |
| 閉じ方 | バルーンのダブルクリックで閉じる所は試せなかった。押す点 (520,272) の窓の持ち主が例のプロセスであることは確かめたが、隔離された shell からの `SetCursorPos` が拒まれ（err=5）、隔離の外での打ち直しは自動モードの見張りが拒んだ。進行役が、そのプロセスがこのワークツリーの `emo-text-layer.exe` であることを確かめてから止めた（`Stop-Process`）。そのため終了コードは取れていない |

- 閉じ方の道（ダブルクリックで窓を消し、`run()` が戻る）はこの spec で変えていない（4.6 は判定を飛ばすだけで、閉じ方は今のまま・要件 6.5）。
- 記録の出力: このワークツリーの `target\hold-run\out.txt`。

## 19. かけ直しの audit（6.1）

### 1 回目

- 測った時点: 枝 `claude/kiro-start-audit-emo-text-state-actor-dcff96` の `c81ca034`（5.7 まで済み）・2026-11-07。
- 物差し: `/ponytail:ponytail-audit`（ponytail 5.3.3・Skill の道具で読み込んだ）。1 回目の audit と同じく作法の 2 つを外した（件数の上限なし・読まなかった所を残さない）。
- 範囲: Boundary Commitments の This Spec Owns のファイル＝**125 ファイル**（`src/` 71＝本番 19・テストと部品 52、`tests/` 41（Rust 36・設定の検体 5）、`examples/` 12（Rust 8・検体 4）、`Cargo.toml`）。`src/` の `layout*`・`segment*`・`region*`・`wrap*`・`lookahead*`・`cursor_tag*`・`writing*`・`draw*`・`canvas*`・`viewbox*`・`look*`・`color*`・`underline*`・`place*` は `audit-emo-text-layout-draw` の持ち物なので外した。1 回目の 121 ファイルに、足したファイル 5 つ（`actor_surface_limit_tests.rs`・`actor_frame_memo_tests.rs`・`choice_test_support.rs`・`tests/support/mod.rs`・`tests/support/fixture.rs`）を足し、消したスパイク 1 つを引いた。
- 読み方: 下請け 8 人が受け持ちのファイルを全行読み（本番 2 人・`src/` のテスト 4 人（例は `choice_*` のテストと同じ 1 人）・`tests/` 2 人）、進行役が本番の指摘（R1-1・R1-2）と例の指摘（R1-9・R1-10・R1-11）の行を開いて確かめた。テストは走らせていない（落ちえないことは読みで確かめた）。
- 前回の直しの確かめ: 指摘 1〜5・7〜10・12・13・20〜23・27・28 の直しは受け持ちのファイルの中で正しく入っている（下請けが本番の道と突き合わせた）。直し残しは R1-7（指摘 15）・R1-8（指摘 16）・R1-16 の `choice.rs`（指摘 24 の後の説明）・R1-5 の `faces.rs`（指摘 13）・R1-19（指摘 27 の「作業の経緯の文は消す」）。新しいテスト `actor_surface_limit_tests.rs`・`actor_frame_memo_tests.rs` は、直しを戻すと赤になることを読みで確かめた（欠けは R1-27 の 1 つ）。
- 頼まれた点: (a) `surface_window_child.rs` の `HitMaskCache` のマスクの欄が無い道は、本番では届かない（マスクは `box_hit_components`（`surface_window_child.rs:165`）で entity を作るときに必ず入り、`AlphaMaskResource` を外すコードはツリーに 0 件・`set_hit_cells` は変化のあったフレームだけ呼ばれる）＝届く場面が無いので指摘にしない。(b) は R1-3、(c)(d) は R1-19 に入れた。
- ukadoc: SSP 互換・ukadoc の正典に当たる指摘は 0 件（確かめが要るものが無かった）。
- **数**: 必ず直す 1・直したほうがよい 13・できれば 15＝**29 件**。本番の振る舞いに関わるもの 4 件（R1-1・R1-2・R1-23・R1-24）。公開の形を変えないと直せないもの 0 件。開発者の裁定に回すもの 2 件（R1-23・R1-24）。

| 番号 | 組 | 種類 | 場所 | 何か・困る場面 | いちばん小さい直し | 本番の振る舞い |
|---|---|---|---|---|---|---|
| R1-1 | 必ず直す | 直す | `marker_surface.rs:147-151`・`actor_present.rs:592,688` | 矢印の絵を GPU へ移す `marker_bitmap` が失敗しても覚えない。`drawn`・`scale` は成功したときだけ変わるので `is_drawn` が偽のまま、毎フレーム描き直す。ネットから入れたバルーンの矢印の PNG の一辺が 16,384 を越える（例 20000×4）と D2D の `CreateBitmap` が毎フレーム失敗し、`error!` が毎フレーム 1 件出て `present_frame` も毎フレーム `Err`（指摘 3 の印の面の上限は面の寸を見るだけで絵の寸は見ない・静止画のデコードに一辺の上限は無い） | 失敗した（`shown`, k）を `MarkerSurface` に覚えて `is_drawn` が「済み」と扱う、または `marker_bitmap` の手前で絵の一辺を `MAX_SURFACE_SIDE` で切り `markers_failed` と同じ形で覚える。テスト 1 本 | 変わる（記録の量） |
| R1-2 | 直したほうがよい | 直す | `actor_attach.rs:413-416` | 再追従（k が変わる）の潰れていない道で、描き手を `surfaces.remove` で捨てるだけで差し込み口から外さない。差し込み口の `VisualGraphics` に前の sprite が残る。k=1 で validrect の幅 9,000 のバルーンを 200% の画面へ移すと、新しい物理寸 18,000 が上限を越えて面を作らず、前の寸の古い字が映ったまま新しい台詞が出ない | 捨てる描き手を `retired_markers` と同じ形で控え、次の提示の頭で `discard(world, Some(slot))` する。テスト 1 本 | 変わる（まれ） |
| R1-3 | 直したほうがよい | 直す | `tests/areka_balloon_fixture/definition.rs:214-229` | `use_self_alpha_is_declared_on` が `capture` の窓で WARN/ERROR 0 件と言うが、発行点が生きている対照が無い（`log-capture-kit/src/lib.rs:76-79` の決まりに反する）。呼び先が `load_balloon_use_self_alpha_quiet` に替わっても緑 | 同じ窓の `areka_emo_present::self_alpha` の `info!`（`self_alpha.rs:43-51`）がちょうど 1 件で `kind=balloon`・`treatment=1` であることを対照に足し、説明を直す | 変わらない |
| R1-4 | 直したほうがよい | 直す | `actor_lookahead_tests.rs:604-614,626-633`・`actor_lookahead_shapes_tests.rs:381,421-427`・`actor_scale_refresh_tests.rs:394-406,466-476,526-536,569-579`・`state_decoration_reset_tests.rs:391,419`（と説明 `:8-9`） | 0 件の主張の窓にも同じテストのほかの窓にも対照が無い（`scale_refresh` は対照が別の窓）。捕捉が働かなくても緑。`state_decoration_reset_tests.rs:8-9` の「同じ窓に出る側の対照を置いた」は事実と違う | `capture` を `test_support::capture_checked` に替える（`reset` の 2 つは `:369` と同じ形の出る側を 1 件置く） | 変わらない |
| R1-5 | 直したほうがよい | 直す | `tests/staysee_balloon_fixture/faces.rs:47-54,100-116,133`・`tests/pipeline_test.rs:226-227,256-259,486-494`・`tests/vertical_fixture_test.rs:151-162`・`state_decoration_tests.rs:441-453`・`choice_tests.rs:584-600`・`state_decoration_reset_tests.rs:187,233-250` | 落ちえないテスト。`faces.rs` は定数どうしを比べるループ（指摘 13 の直し残し）。`pipeline` は同じ cue 列を新しい純粋な状態 2 つへ流して比べる。`vertical_fixture` の空でない確かめは `read_fixture_bytes` が先に落とす。`carrier_with_another_name_is_ignored` は `font_tag_tokens` の名前の確かめを外しても緑。`choice_tests` は期待値を本番と同じ式で組み、帯が 0 本でも緑。`reset` の warn 0 は記録の道の無い関数に対するもので、`:233-250` は `:169-188` と同じ | `faces` の関数と定数と呼び出しを消す。`pipeline` の `*_b` の側を消す。`vertical_fixture` のテストを消す。`state_decoration_tests` は `before = state.clone()` と全体の一致にする。`choice_tests` は数で書く。`reset` の `:187` の確かめと `:233-250` を消す | 変わらない |
| R1-6 | 直したほうがよい | 直す | `tests/staysee_balloon_fixture/scale.rs:45-50,86-115`・`tests/choice_fixture_test.rs:180-188`・`state_route_tests.rs:180-190`・`actor_region_warn_tests.rs:448-454`・`actor_route_tests.rs:215-216` | 名前・説明と違うことを確かめる、または弱い。`scale.rs` は本番に起きない入力（k 倍した寸）を解決へ渡す（同じファイルの `:14-19` と食い違う）。`choice_fixture` は「台本と注入の対応」と言いつつ選択肢の開きの数だけを比べ、台本の選択肢を `うん,OnYes` に替えても緑。`state_route` は Unresolved に変える閉包を通らない。`region_warn` は scope 0／1 のどちらのラベルも受け、ラベルの取り違えで緑。`actor_route` は前提（消す前に強調がある）を確かめない | `scale.rs` のテストと補助を消す。`choice_fixture` は `command_cues` に通して (id, text) の列を比べる。`state_route` は名前を直す。`region_warn` は期待のラベルを引数に。`actor_route` は前提の assert を 1 つ | 変わらない |
| R1-7 | 直したほうがよい | 直す | 読み戻しの鎖 `rt.surface(..).expect().read_back().expect()`: `actor_clear_atomicity_tests.rs:64-71,144-151,210-214,227-231,280-284,300-304,330-334`・`actor_region_collapse_present_tests.rs:108-112`・`actor_runtime_frame_tests.rs:300-307,469-476`・`actor_scale_refresh_tests.rs:269-273,638-642`・`actor_scroll_present_tests.rs:388-392,395-399`・`actor_scroll_retain_tests.rs:113-116,150-154,187-191`／`ceil(size×k)`: `actor_surface_limit_tests.rs:49-54`・`actor_underline_present_tests.rs:194-199`・`actor_scroll_retain_tests.rs:49-54,142-143,244-245`・`actor_scroll_present_tests.rs:428-431`／`make_dispatcher_and_compositor`: `surface_window_child_tests.rs:20-26`・`surface.rs:626`（兄弟の `draw_oracle_tests.rs:43`・`viewbox_draw_test_support.rs:38` にも）／窓とマスクの組み立て: `surface_hit_cells_tests.rs:19-29,41-68`・`marker_surface_tests.rs:347-393`／`NO_OFFSET`: `choice_tests.rs:203`・`choice_decorate_tests.rs:44`／包み: `actor_decoration_frame_tests.rs:24-37`・`actor_frame_memo_tests.rs:99-111`・`choice_style_resolve_tests.rs:13-21`・`choice_test_support.rs:26-34`（`model_rect` で書ける）・`choice_anchor_tests.rs:66-74`（`spans`）・`state_decoration_reset_tests.rs:253-258`（`warns`）・`state_route_tests.rs:392-412` | `src/` のテストの部品の写し（指摘 15 の直し残し）。部品の 1 つを直しても写しが古いまま残る | 既にある部品（`test_support::read_back`・`box_in` など）を呼ぶ。無いもの（`physical(size, k)`・`make_dispatcher_and_compositor`・窓とマスク・`NO_OFFSET`）は `#[cfg(test)] pub(crate)` で 1 つ置く（兄弟の 2 か所は申し合わせ） | 変わらない |
| R1-8 | 直したほうがよい | 直す | `tests/line_pitch_readback_test.rs:99-132,288-321,552-630`・`tests/shell_balloon_fixture_test.rs:44-53,99-106`・`TalkCue` の手組み（`tests/box_attach_test.rs:98-105`・`tests/decoration_readback/mod.rs:358-365`・`tests/line_pitch_readback_test.rs:201-230`・`tests/choice_fixture_test.rs:201-221`）・BalloonModel の幾何の組み立て（`tests/pipeline_test.rs:168`・`tests/scale_invariance_test.rs:40`・`tests/viewbox_scroll_test.rs:98`・`tests/draw_readback_test.rs:77`・`tests/attach_wiring_test.rs:171`）・`tests/staysee_balloon_fixture/test_support.rs:137-139`（`merged_model` の写し・`resolve_faces` は `bake.rs` だけが使う）・`tests/staysee_balloon_fixture/region.rs:100`（`example_fixture_dir`）・`tests/staysee_balloon_fixture/wrapping.rs:405-419`（`glyph_items`） | `tests/` の部品の写し（指摘 16 の直し残し）。`line_pitch` の `ink_extent`・`ink_count`・2 つの定数・帯の確かめは `support` の `y_span_where`・`count_in_rect`・`assert_hover_band` の写しで、共有の帯の確かめを締めても届かない（今も ordinal 1 の上端を見ない）。`shell_balloon` の `fixture_text` は Utf8 で読み（本番と `support` は Ansi）、`support/fixture.rs:81-84` の決まりに反する | `support` の部品を呼び、写しを消す。幾何の組み立ては `support` に 1 つ置く | 変わらない |
| R1-9 | 直したほうがよい | 直す | `examples/emo-text-layer/fixture.rs:26-30` | 例のバルーンを `DefaultEncoding::Utf8` で読む（本番 `areka-emo-present/src/balloon.rs:508` と `tests/support/fixture.rs:86` は Ansi）。説明は「tests と同じ規約」と書くが違う。`charset` の行の無い上書き層（例の `balloons0s.txt` も無い）に CP932 の字があると、例だけが本番と違う model を観測する | `Ansi` に変える（説明はそのまま正しくなる） | 変わらない |
| R1-10 | 直したほうがよい | 直す | `examples/emo-text-layer/drive.rs:240-247` | `--hold` でも 30 秒の watchdog が `fail`＋`finish` を走らせる。hold で drain が止まると窓が自分で閉じ、終了コードは 0（「`--hold` は自分で閉じない」に反する） | watchdog を `!demo.hold` のときだけ効かせ、hold では `error!` を 1 回だけ出す | 変わらない |
| R1-11 | 直したほうがよい | 直す | `examples/emo-text-layer/drive.rs:208-212`・`examples/emo-text-layer.rs:215-218` | `--hold` で装着に失敗すると `fail_attach` が窓を閉じ、`main` は hold なら判定を見ずに `Ok(())`＝終了コード 0。タイプライターの例は同じ失敗で `setup_abort`（終了コード 1）なので 2 つの例で違う | `fail_attach` を `setup_abort` に替える（hold でないときの出力は同じ） | 変わらない |
| R1-12 | 直したほうがよい | 直す | `examples/emo-text-layer/drive.rs:281,291`・`examples/emo-text-typewriter-demo.rs:301` | `present_frame` の `Err` を `warn!` だけで流す（例の失敗は `error!` を残す決まりから外れる）。hold とタイプライターの例で装置が失われ毎フレーム失敗しても `error!` が 0 行 | 最初の 1 回だけ `error!` にする | 変わらない |
| R1-13 | 直したほうがよい | 直す | `examples/emo-text-layer/drive.rs:350-352` | `read_back().ok()` が `TextLayerError` を捨てる。読み戻しの失敗が「readback 不能」とだけ記録され、HRESULT も「面が無い」か「読み戻しの失敗」かも残らない | 原因を `fail` の文に含める | 変わらない |
| R1-14 | 直したほうがよい | 直す | `examples/emo-text-layer/drive.rs:138-145` | DPI の記録の繰り返しで kero の行にも sakura の `view0.scale()` を書く。sakura の窓が 100%・kero が 150% の画面にあると、kero の行は dpi=144 なのに scale_k=1.0 | 繰り返しの組に各窓の view を入れる | 変わらない |
| R1-15 | できれば | 直す | `state_cue_apply_tests.rs:72-75,239-258,281-284,302-307,497-498`・`choice_style_resolve_tests.rs:120-142`（`:198-215` と）・`actor_tests.rs:6-22`・`actor_clear_atomicity_tests.rs:47-53,287-296,335-338`・`actor_choice_contract_tests.rs:225`・`actor_inline_marker_tests.rs:129-135`・`choice_tests.rs:266`・`choice_decorate_tests.rs:185-239,742-745`・`choice_anchor_tests.rs:258-265`・`state_anchor_tests.rs:613-616`・`state_cluster_tests.rs:114`・`state_marker_tests.rs:94`・`state_place_tests.rs:160-163`・`state_reveal_tests.rs:98-107`・`inline_marker_tests.rs:21-22`・`scroll_marker_tests.rs:18-21`・`tests/vertical_fixture_test.rs:186-198`・`tests/viewbox_scroll_test.rs:230-233`・`tests/staysee_balloon_fixture/region.rs:303-307,324-330`・`tests/staysee_balloon_fixture/wrapping.rs:381-387`・`tests/staysee_balloon_fixture/bake.rs:56-70`・`tests/physical_extent_arbitration_test.rs:194-205,369-375`（81/81 の固定には触れない）・`tests/decoration_readback_test.rs:101-113` | 同じことを 2 度確かめる、または直前の完全一致から導ける確かめ | 弱い方を消す（`decoration_readback` の表の数は `const _: () = assert!(..)` に） | 変わらない |
| R1-16 | できれば | 直す | `sink.rs:46,285-287`（`GhostBootOptions.text_sink`＝今は `sinks`）・`state_decoration.rs:53`（`lexer_tests.rs::quoted_arg_protects_comma` は無い・本物は `quoted_and_unquoted_args_mixed`）・`choice.rs:680,703,757-759`（指摘 24 の後も「`resolve` と同じ `style_has_underline`」）・`lib.rs:14-15`（純粋層の一覧に `choice`・`balloon_overrides`・`inline_marker` などが無い）・`lib.rs:286`（「14 本」の下に 23 件）・`surface.rs:719-726,773`（`source_tex`＝今は `sources[front]`・「上流描画（task 6）不在」は誤り）・`actor_box.rs:41`（「tasks.md Implementation Notes 5.4」は今の spec の 5.4 を指して見え、行き先を誤る・本物は `completed/areka-P0-shell-balloon/tasks.md:289`） | 本番のソースの古い説明・事実と違う説明 | 今のコードに合わせる | 変わらない |
| R1-17 | できれば | 直す | `sink_talk_end_tests.rs:7,97,99,116,118`（「2 つの受け口の形」・「4 つ目の受け口」＝受け口は 3 つ）・`actor_choice_contract_tests.rs:35,45,50`（「8.1 では常に空」）・`:126`（`line_bands` は `choice.rs`）・`actor_decoration_tests.rs:5-7`（「COM は要らない」＝最後の 1 本は実 GPU）・`actor_box_tests.rs:4`（「World は空でよい」）・`actor_lookahead_shapes_tests.rs:4,5,46,48,123,160`（「検査 8」が `actor_lookahead_tests.rs:636-688` と重なる）・`actor_scroll_present_tests.rs:379-380,393`（「描画資源だけを捨てて」＝R1-28）・`tests/shipped_fixture_region_test.rs:46,345`（「6 本」＝5 本）・`tests/fixtures/emo2-choice/menu.txt:17-18`（ordinal 0＝実は 1・Invert は描かない）・`tests/decoration_readback/vertical.rs:31,121`（登記は済み・`doc/COMPAT_ARCHITECTURE.md:250` の値も古い＝持ち主へ）・`tests/draw_readback_test.rs:19-21,30`（「draw.rs 6.3 檻が正典」＝今は `draw_oracle_tests.rs`・`canvas.rs:695`）・`tests/staysee_balloon_fixture/script.rs:22`（「α 後」）・`tests/support/mod.rs:439`（今は `viewbox_draw_png_dump_tests.rs`）・`choice_anchor_tests.rs:307`（「research.md 21 節」は辿れない）・`tests/staysee_balloon_fixture_test.rs:71-108`（`tests/` の中の `#[cfg(test)]` 9 個は何もしない） | テストの古い説明・事実と違う説明 | 今のコードに合わせる | 変わらない |
| R1-18 | できれば | 直す | `examples/emo-text-layer/setup.rs:127-128`・`examples/emo-text-typewriter-demo.rs:260`（「k=1.0 恒常」＝200% で 2.0・18 節）・`examples/emo-text-layer.rs:78-79`（「上流が k≠1 を供給し次第」）・`examples/emo-text-layer/drive.rs:17-18`（`WS_EX_TRANSPARENT` は付けない）・`examples/emo-text-layer/verdict.rs:49-50`（`vertical_lr` の腕が無い）・`examples/emo-text-layer/scenario.rs:84-86`（見出しの名が違う）・`examples/emo-text-layer.rs:201`（`--hold` でも「自動・約 4.5 秒」と出す） | 例の古い説明 | 今のコードに合わせる | 変わらない |
| R1-19 | できれば | 直す | 作業の経緯の札・文（「task N」「タスク N」の字だけで 131 か所・58 ファイル）。spec の名の無い番号だけの札: `actor.rs:539,570`・`actor_attach.rs:152,184`・`actor_decoration.rs:42`・`actor_present.rs:83,404,744`・`marker_surface.rs:11`・`choice.rs:331`・`sink.rs:37,39,69,125,130,159,245,287`・`lib.rs:101,104,123,344,440,538`・`surface.rs:527-530,652,833`・`Cargo.toml:36`・`actor_box_tests.rs:1,128`・`actor_box_sync_tests.rs:1,6,476,633`・`actor_box_present_tests.rs:1,6,108-109,158-159,196-197`・`actor_choice_contract_tests.rs:162,252,379,381,392,399,451`・`actor_clear_atomicity_tests.rs:9,180,184,298`・`actor_decoration_frame_tests.rs:1`・`actor_decoration_tests.rs:1,257`・`actor_runtime_frame_tests.rs:352,403,438`・`actor_scale_refresh_tests.rs:17,23,283-284`・`actor_route_tests.rs:1`・`actor_tests.rs:121`・`actor_surface_limit_tests.rs:432`・`actor_region_collapse_tests.rs:19-20,414`・`choice_decorate_tests.rs:1,571,709`・`choice_tests.rs:187,832`・`choice_style_resolve_tests.rs:1`・`choice_glyph_cells_tests.rs:1`・`state_decoration_tests.rs:2,15`・`state_decoration_reset_tests.rs:2,6,516,630`・`state_cue_apply_tests.rs:419`・`state_decoration_carry_tests.rs:133`・`state_place_tests.rs:97`・`sink_talk_end_tests.rs:155`・`surface_hit_cells_tests.rs:1`・`surface_window_child_tests.rs:1,81`・`scroll_marker_tests.rs:254`・`marker_surface_tests.rs:340`・`tests/attach_wiring_test.rs:1,66,137,404,521,525,529-533,562`・`tests/box_attach_test.rs:1,8,9,15,215`・`tests/choice_fixture_test.rs:1,132`・`tests/fixtures/emo2-choice/descript-cursor.txt:2,5`・`descript-plain.txt:2`・`menu.txt:1`・`tests/draw_readback_test.rs:1,17,154,346,348`・`tests/emo2_fixture_e2e_test.rs:1,6,7`・`tests/pipeline_test.rs:1,450,452`・`tests/scale_invariance_test.rs:1,11,17`・`tests/shell_balloon_fixture_test.rs:1`・`tests/vertical_fixture_test.rs:1,42,55`・`tests/viewbox_scroll_test.rs:1,13,141`（頼まれた (c)）・`examples/emo-text-layer.rs:65,191`（`println` で画面に「（task 9.2）」が出る）・`examples/emo-text-layer/fixture.rs:21`・`examples/emo-text-layer/drive.rs:103`。経緯を語る文: `actor_scroll_retain_tests.rs:15,19,227`（「HEAD で緑」）・`actor_tests.rs:24,44`・`actor_underline_present_tests.rs:603`・`state_test_support.rs:6-7`・`state_reveal_tests.rs:33,176-178`・`state_cue_apply_tests.rs:340,352,373,414,544,549`・`state_cursor_coord_parse_tests.rs:129-131`・`tests/shipped_fixture_region_test.rs:57-117`（付録は `completed/areka-P0-balloon-vertical-canon/tasks.md` の付録 A の古い写し）・`examples/emo-text-layer/scenario.rs:12,14,16,25,79-80,88-92` | 指摘 27 の「作業の経緯の文は消す」の直し残し。spec の名の無い番号は、どの spec の tasks.md かを読み手が辿れない（`actor_box.rs:41` は誤った先へ導く・R1-16）。この spec の「指摘 N」も spec の名が無いので、completed へ移すと別の audit の番号と見分けにくい。spec の名と番号を書いた札（`script-small-tags task 6.3`・`balloon-scroll-arrows task 4.3` など）は数えない | 番号だけの札は消して要件の番号と理由を残す（または spec の名を足す）。経緯の文と付録は消す（付録は正本を指す 1 行に） | 変わらない（`emo-text-layer.rs:191` は例の印字だけ） |
| R1-20 | できれば | 直す | `actor_underline_present_tests.rs:250,280`（撮るたびに `com_world()`＝装置を 1 本で 24 回・縦書きで 12 回）・`tests/decoration_readback/mod.rs:196-218`（撮るたびに `make_world_with_gpu()`・横 22 回・縦 73 回）・`actor_lookahead_tests.rs:660,668`（配置を 2 回組む）・`actor_box_tests.rs:122-124`（持っている名前を表を畳み直して引く）・`tests/areka_balloon_fixture/wrapping.rs:250-263`（本番の式の写しと配置の 4 回の組み直し） | テストの余分な仕事 | World を 1 つ組んで予約スロットだけ毎回足す（`actor_surface_limit_tests` の形）・取った値を使い回す・ループを消す | 変わらない |
| R1-21 | できれば | 直す | `lib.rs:521-528` | 純粋層の禁じる字の検査は `use wintf::{com::dxgi::X, ecs::Y};` を通す（`wintf::com` も `windows::` も字として現れない）。今の走査面 88 ファイルに `com::` は 0 件 | 禁じる字に `"com::"` を足す | 変わらない |
| R1-22 | できれば | 直す | `surface.rs:568-569` | `(-dx).max(0)`・`(-dy).max(0)` は `i32::MIN` で桁があふれる（隣の `:571` は `saturating_sub`）。2^31 画素のスクロールが要るので実際には届かない | `dx.saturating_neg().max(0)`（dy も） | 実際には変わらない |
| R1-23 | できれば | 開発者の裁定 | `balloon_overrides.rs:59-66` | `disable.font.color` の成分不足の `warn!` が解き直しのたびに出る（前の値と比べる門が無い）。面を切り替えるたびに同じ警告がもう 1 件（毎フレームではない）。`look.rs`・`draw.rs` の書体の警告も同じ種類で兄弟の持ち物 | 前の値と比べ、新しく決まったときだけ出す。`real-run-log-noise` が範囲に入れなかったのかを確かめてから | 変わる（記録だけ） |
| R1-24 | できれば | 開発者の裁定 | `state.rs:445-447`（`clears`）・`state_route.rs` の `routes` | 指摘 11 の直しで `actors` は外れるが、`clears` はわざと残し（`crates/areka` の `balloon_visibility` が `clear_count` の変化を見る）、`routes` もスコープの数だけ増え `set_box_index` でも外れない。台本のスコープの番号を台詞ごとに変えるゴーストで、使ったスコープ 1 つにつき約 100 バイトが残る | 残すのは決めたことなので、裁定に回すか記録に残す（直すなら `clear_count` の読み手と申し合わせる） | 変わる（直すなら） |
| R1-25 | できれば | 直す | `actor_box_sync_tests.rs:186-194,293-297,444-448,469-473`・`actor_decoration_tests.rs:287-291,297-301,308-312`・`state_cue_apply_tests.rs:350,415,621`・`state_route_tests.rs:230,270`・`sink_talk_end_tests.rs:45-49,207` | 0 件の主張の窓に対照が無い（同じテストの前の窓で捕捉が働くと分かっているので R1-4 より軽い） | 窓を `capture_checked` にする | 変わらない |
| R1-26 | できれば | 直す | `tests/support/mod.rs`・`tests/support/fixture.rs` の `elem`・`shell_of`・`gradient_image`・`count_in_rect`・`y_span_where`・`emo2_folder`・`dwrite_factory`・`BALLOON_LOG_TARGET`・`AlphaCensus` | `support` の中でしか使わない `pub`（使われない部品は 0）。R1-8 で `count_in_rect`・`y_span_where` を外から使うならそれは残る | R1-8 の後で、外から使わないものを私有にする | 変わらない |
| R1-27 | できれば | 直す | `actor_surface_limit_tests.rs`（文字の面の覚えの鍵の WUC の部分・`actor_present.rs:264-271`） | 文字の面の `attempt` から WUC の資源の変わった時刻を外しても全部緑（WUC の作り直しで試し直すことは印の面（`:349-382`）でしか確かめていない） | `remembered_surface_failure_is_retried…` に `WucGraphicsResource` の `set_changed()` の段を足し、記録が 1 件増えることを見る | 変わらない |
| R1-28 | できれば | 直す | `actor_scroll_present_tests.rs:379-380,393` | 全部描き直しと比べる部品が `rt.surfaces.remove(place)` で印の面を控えずに捨て、比べるたびに窓の子が 2 つ残る（説明は「描画資源だけを捨てて」） | `retired_markers` に控える（R1-17 の説明と一緒に） | 変わらない |
| R1-29 | できれば | 直す | `examples/emo-text-layer/scenario.rs:22-26`・`examples/emo-text-typewriter-demo.rs:68-70` | 挨拶の 3 行 `LINE1`〜`LINE3` が 2 つの例に同じ字で写してある（`fixture`・`setup` は `#[path]` で引けている） | タイプライターの例が `scenario` から引くか、`fixture.rs` へ移す | 変わらない |

- 指摘にしなかったもの: (a) の `HitMaskCache`（上の「頼まれた点」）・`scroll_marker.rs:229` の `shortcut:`・`tests/kero_menu_capacity_test.rs:260-276`・`tests/shipped_fixture_region_test.rs:42-50`・`tests/physical_extent_arbitration_test.rs` の 81/81・`tests/shell_balloon_fixture_test.rs` の `fake_resolver`・`state_decoration_tests.rs:407,421` の `cursorbrush`（兄弟の 11.4）・`tests/areka_balloon_fixture/definition.rs:127-131` の表の数（重なりを防ぐ意図が説明にある）・`Cargo.toml` の description の spec 名（ほかのクレートと同じ出自の札）。
- 範囲の外で見えたもの（知らせるだけ）: 兄弟の `draw_oracle_tests.rs:43`・`viewbox_draw_test_support.rs:38` の写し（R1-7）・`state_decoration_reset_tests.rs:483` は範囲の中で `state_decoration.rs:53` と同じ誤った名（R1-16 と一緒に直す）・`doc/COMPAT_ARCHITECTURE.md:250` の古い値（R1-17）。
