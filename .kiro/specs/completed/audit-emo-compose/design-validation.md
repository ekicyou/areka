# 設計の確かめ: audit-emo-compose（面の合成の見直し）

- 確かめた時点: 枝 `claude/kiro-start-audit-emo-compose-cb01ad`（`51f1c770`）・2026-11-05
- 読んだもの: `spec.json`・`requirements.md`（末尾の「要件の討議で確定した事項」を含む）・`design.md`・`research.md`・`brief.md`・`.kiro/steering/` の `product.md`・`tech.md`・`structure.md`
- コードで確かめたもの: `crates/areka-emo-compose/src/` の `bind_steps.rs`・`nesting.rs`・`plan.rs`・`plan_bounds.rs`・`plan_issues.rs`・`hit_import.rs`・`fold.rs`・`boxes.rs`・`world.rs` と、テストの `boxes_tests.rs`・`base_image_tests.rs`・`fold_tests.rs`。範囲の外は `crates/wintf/src/ecs/graphics/core.rs`・`crates/areka-emo-present/src/display.rs`・`crates/areka-emo-present/src/shell_target.rs`・`crates/log-capture-kit/tests/file_length_guard_test.rs`
- この確かめはコードも設計も変えていない。

## 要約

設計は 18 件の直しを今のファイルと層の形の中に収めていて、依存の向き・「合成器は絵を持たない」約束・`#[non_exhaustive]` の列挙も守っている。コードと照らした主な主張（指摘 4 の根が 1 か所であること、上限 16,384 の定数がツリーに無いこと、`SurfaceIndex` の外の使い手が 0 であること、再帰の走査が 6 か所あること）は合っていた。ただし、広がりの予算をかける範囲と、箱の追記の範囲引きの波及の 2 点は、実装の前に設計の討議で決め直したほうがよい。

## コードと照らした結果

| 設計の主張 | 結果 |
|---|---|
| 指摘 4 の根は `BindSteps::build` の 1 か所（`bind+random` にも pattern0 の手順を作り、`NestTable::from_world` がそれを「描く」辺に数える） | 合っている。`bind_steps.rs:44-53` が pattern0 の手順を作り、`nesting.rs:138` だけがそれを数える。合成は `bind_steps.rs:159-166` の早い戻りで使わない。`claim_from` は `Insert` しか見ず、`bind+random` の手順には `Insert` が入らない（`if pure`）ので、手順を消しても差し込みの数えは変わらない |
| `MAX_EXTENT` 16,384 と同じ値の定数はツリーに無い | 合っている（`16_384`・`16384`・`D3D11_REQ_TEXTURE2D`・`GetMaximumBitmapSize` はどれもヒットしない）。`create_device_3d` は機能水準の列を `None` にしてハードウェアで機器を作る。`display.rs:181` が `CreateBitmap` で原寸のまま GPU へ上げる。ただし「既定の並び＝11_1 から」は事実と違う。`None` のときの既定の並びは 11_0 から始まり、11_1 は入らない。どちらも一辺の最大は 16,384 なので値は変わらない。説明の 1 行を直すだけでよい |
| `SurfaceIndex` の、このクレートの外の使い手は 0 | 合っている。中の引きは `surface`・`hit_regions`・`surface_and_binding`・`attach_hit_regions`・`fold_append` で、毎フレーム通るのは入った面 1 つごとに 1〜2 回。`BTreeMap` に替えても引き 1 回が O(log 面の数) になるだけ |
| 入れ子の再帰は 6 か所（`plan.rs` の鎖・`flatten_extent`・`plan_bounds::walk`・`regions_of`・`nesting` の `walk`／`enter`・`claim_from`） | 合っている。ほかに再帰は無い。`reaches` はもともと反復で、`seen` の集合で各面を 1 度しか訪れない |
| 見える部品の走査は各面を 1 度しか訪れない（予算は要らない） | 合っている（`out` を訪れた印に使う・`nesting.rs:318-324`） |
| 命令列を明示のスタックにしても命令の並びは同じ | 筋は通る。段を作る処理（`push_static_element_ops` と層 (ii)）は子の結果を読まないので、子へ入る所に印を置いて後でたどれば並びは同じになる。祖先の確かめも、印を取り出す時点の祖先の列が再帰の入口の時点と同じになる |
| `hit_region`・`hit_region_scaled` の本番の呼び手は 0 | 合っている（範囲の外のヒットは、別のモジュール名 `emo2_boot::hit_region` と `resolve_hit_region` だけ） |
| 外形の表を `bind_atlas` で作る | 筋は通る。`bind_atlas` は面の表 1 つにつき 1 度だけ呼ぶ決まり（`world.rs:236-243`）。束縛が無ければ今も `surface_and_binding` が `None` を返して外形は 0×0。`shell_bounds` の唯一の本番の呼び手（`offline_dump/mod.rs:218`）は束縛の後に呼ぶ |

開発者の決めとの照らし合わせ:

- 指摘 5 と 20 には触らない（Non-Goals）。外形の走査を明示のスタックにしても、element定義の X,Y を足す直しは入れない。`plan_extent_film_tests.rs:121,129` の期待も変えない。
- 指摘 1⑶ は `audit-parsers-sakura` に任せている（新設の `expand_targets` は範囲を広げたまま）。
- 上限の決め方: 外形は元からある頭打ち（GPU の面の最大寸）、深さは上限なし（明示のスタック）、広がりは枝だけを捨てて `error!` を出し、シェルは全体として読む。ここまでは討議の決めどおり。予算の値は議題 1 として開かれている（下の問題 1 も参照）。
- 依存を足さない・`Cargo.lock` を変えない・`doc/`・`docs/` の行を変えない・`crates/areka-emo-text/` に新しいファイルを足さない。どれも守っている。`tests/fixtures/animated-playback/README.md:27` の直しはパスを名指さない。

## 重大な問題

### 問題 1: 読み込みの走査の予算を「シェル全体の合計」でかけると、上限の根拠と範囲が合わない

- **何が問題か**: 予算の値は「1 フレーム（16.667 ms）の中で回りきれる面の数」から決める案（議題 1・案ア）。ところが同じ 1 つの値を、外形の表・シェルの外形・当たり判定の持ち込み・到達の確かめでは「全部の面の合計（全部の `reaches` の合計）」にかけている（System Flows の予算の表）。この合計は 1 フレームの仕事ではないうえ、壊れていなくても面の数に比例して伸びる。シェルが大きいほど、壊れていないのに打ち切られる見込みが増える。つまり「普通のシェルならこの範囲に収まる」という見立てに戻ってしまう。そのうえ `reaches` は `seen` の集合を持つ反復なので、菱形で指数に増えることはもともと無い。
- **影響**: 面の多い正しいシェルで、外形の表や当たり判定の持ち込みが途中で切れ、絵が欠ける・子の当たり判定が効かない、ということが起こりうる。もう 1 つの影響として、`build_extents` と `build_plan` で打ち切る位置がずれると、命令が外形の外に出て黙って切られる。
- **直し方の案**: 読み込みの走査の予算は「根の面 1 つにつき 1 つ」にする。そうすれば、その根を 1 回合成するときと同じ範囲・同じ打ち切り位置になる（外形の表と命令列の打ち切り位置も揃う）。`reaches` には予算を通さず、「`seen` で 1 回ごとに線形」と説明に書く（`ponytail:` の注釈は残す）。決定論のテストの形（2^30 の菱形は切れる・検体と golden は切れない）はそのまま使える。
- **要件との対応**: 要件 2.1・4.4、「要件の討議で確定した事項」の「上限の決め方」（元からある頭打ちだけから決める・越えた枝だけを捨てる）
- **設計の箇所**: 「System Flows」の「広がりの予算（指摘 3）」の表、「明示のスタックの走査と予算」の「ほかの 5 か所」、「設計の議題」1

### 問題 2: 箱の追記を範囲引きにすると、箱の報告（`BoxIssue::AppendTargetMissing`）が変わり、今あるテストが落ちる

- **何が問題か**: 設計は、指摘 1⑴ の範囲引きで消えるのを、畳み込みの `warn!`（「surface.append 対象 id が未存在」）だけとしている。しかし箱の `distribute`（`boxes.rs:236-245`）は、範囲の中の無い番号ごとに `BoxIssue::AppendTargetMissing` を報告に積む。この報告は `crates/areka-emo-present/src/shell_target.rs:804` で `warn!` になる。範囲を `existing` から引く形にすると、範囲の中の無い番号の報告が消える。今あるテスト `boxes_tests.rs:775-792`（`surface.append1-5,!3` で面 5 の報告を期待する）は落ちる。設計の「事前・事後の条件」と、要件の討議の決め（⑴⑵ は振る舞いを変えない）のどちらにも、この変化は書かれていない。直すテストの一覧にも、範囲の外の受け手にも載っていない。
- **影響**: 実装の段で、決めていない振る舞いの変化（箱の報告と、表示の側の警告の行）を、そのとき作業している人がその場で判断することになる。`audit-emo-present` との申し合わせにもこの件が入っていない。
- **直し方の案**: 議題 5 を箱にも広げて、どちらかに決める。(a) 範囲の中の無い番号は箱でも報告しない。`boxes_tests.rs:775-792` の期待を直し、`shell_target.rs:804` の警告が減ることを `audit-emo-present` に伝える。(b) 範囲 1 つにつき「届かなかった番号があった」を 1 件だけ報告する形に変える（報告の型が変わる）。推すのは (a)（畳み込みの側と同じ扱いになり、型も変わらない）。
- **要件との対応**: 要件 2.3・5.1・5.2、「確定した指摘の一覧」の 1（⑴⑵ は振る舞いを変えない）
- **設計の箇所**: 「追記の範囲引き（指摘 1⑴⑵）」の「事前・事後の条件」と「波及」、「並走の兄弟との申し合わせ」、「設計の議題」5

### 問題 3: 1,000 行の番人の見通しが `plan.rs` しか見ておらず、上限に近いテストのファイルへの足し分が見積もられていない

- **何が問題か**: 要件 6.1 の対応欄は `plan.rs`（706 行）だけを確かめている。しかし、新しく足すテストの置き場の中には、上限に近いファイルがある。`fold_tests.rs` は 976 行で、1⑴⑵ の壊れたら落ちるテストを足し、さらに指摘 16 で 23 か所の `fold_shell(..)` を `fold_shell_with(.., Records::Emit)` に書き換える（rustfmt が行を折れば行が増える）。ほかにも `composer_tests.rs`（897 行）・`nesting_visible_tests.rs`（837 行・深さと指摘 4 のテストを足す）・`blit.rs`（782 行・`overlay_op` と指摘 19 のテストが入る）がある。File Structure Plan の新しいファイルは `budget.rs` と `plan_frame_merge_tests.rs` の 2 つだけ。
- **影響**: 実装の途中で番人が赤になり、決めていない分け方（新しいファイル）がその場で足される。指摘 12 の写しの削除で減る行もあるので、越えるかどうかは足し引きしだい。
- **直し方の案**: タスクの生成の前に、置き場ごとに「今の行数＋足す分－写しの削除で減る分」を見積もり、950 行を越えそうなファイル（少なくとも `fold_tests.rs`）は新しい兄弟のテストファイル（例: `fold_range_tests.rs`）に置くと File Structure Plan に書いておく。例外表には足さない（要件 6.1）。
- **要件との対応**: 要件 6.1
- **設計の箇所**: 「Requirements Traceability」の 6.1、「File Structure Plan」、「Testing Strategy」の「壊れたら落ちるテスト」の表

## 設計の良いところ

1. **直しを今の形の中に閉じ込めている**: 新しく足す型は `Budget`・`PlanScratch`・`SurfaceExtents`・`ExtentTooLarge` の 4 つだけ。どれも、今ある形（読み込みの値は面の表のリソース、`BindSteps` と同じ置き方、合成器は作業場だけを持つ）に従っている。案 B（走査の文脈をまとめる型）を命令列の作業場だけに絞った理由も筋が通っている。
2. **主張がコードの根拠つきで、波及を先に洗い出している**: 指摘 4 の根を 1 か所に絞った見立て、`hit_region` の呼び手 0、`SurfaceIndex` の外の使い手 0、`ComposeError` の名指しの比べが `EmptyComposition` だけであること、は確かめた範囲でどれも正しかった。並走の兄弟との申し合わせも、相手と時期まで決めてある。

## 判定

**GO**（上の 3 点を設計の討議で決めてから、タスクの生成へ進む）

- **理由**: 層の形や依存とぶつかる所は無く、要件と開発者の決め（指摘 5・20 は残す、1⑶ は任せる、依存と `Cargo.lock` に触らない、`doc/` の行と `areka-emo-text` に触らない）も守っている。3 点はどれも、設計の 1〜数行の決め直しか、タスクを作る前の見積もりで片づく。
- **次にすること**:
  1. 設計の討議（`/kiro-design-discussion audit-emo-compose`）で、問題 1 を議題 1 と一緒に、問題 2 を議題 5 と一緒に決め、問題 3 の置き場を File Structure Plan に書き足す。あわせて、機能水準の既定の並びの説明（11_0 から）を直す。
  2. そのあとで `/kiro-spec-tasks audit-emo-compose` を回す。
