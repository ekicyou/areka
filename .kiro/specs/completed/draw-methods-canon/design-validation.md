# 設計の確かめ: draw-methods-canon

- 対象: `.kiro/specs/draw-methods-canon/design.md`（要件 `requirements.md`・`research.md`・steering の `product.md`・`tech.md`・`structure.md` と突き合わせ）
- 手順: `kiro-validate-design`（分析 → 重大な問題 → 良い所 → GO／NO-GO）。対話なしで行い、問いは下の「重大な問題」として設計の討議へ渡す。
- 既存コードとの照合: `crates/areka-emo-compose/src/{plan.rs,plan_always.rs,plan_extent.rs,plan_bounds.rs,nesting.rs,error.rs}`・`crates/wintf/src/ecs/graphics/visual/{draw_content.rs,promotion_shape.rs,draw/diff.rs}`・`crates/wintf/src/ecs/graphics/damage.rs`・`crates/wintf/src/ecs/layout/hit_test/mod.rs`・`crates/areka-emo-present/src/presenter/{hub.rs,snapshot.rs}`・`crates/areka/src/mcp/dump_surface.rs`。

## 設計の確かめの要約

「計画（compose の純粋な値）→ 節（描画の木）→ wintf の 1 つの描き方」の形は、要件 7.8（描き方を 2 つ持たない）と開発者の決め（描画と当たりは GPU・1 フレーム遅らせない）にまっすぐ沿っており、語 → `Mix` → D2D の重ね方の写しも正典と SSP の測りに合っている。wintf の既存の口（`MixMode::Composite` の 13 種・切り抜きの中の重ね方・`SOURCE_OVER` でない件の差分の全面化・`SOURCE_OVER` でない部分木を上げない規則）は設計の前提どおりに在る。一方で、拡大率 k が整数でないときの描き方・pattern定義の `base` の意味を 1 か所に閉じること・自動の上げ下げと「画面と同じ答え」の関係の 3 点が、決めないまま実装に回ると見え方か檻の前提が崩れる。

## 重大な問題

🔴 **重大な問題 1**: k が整数でないときの描き方（R5・OPEN QUESTION 5）を実装の段へ送っている
**Concern**: 今は「原寸で合成してから 1 枚を拡大」だが、設計は element ごとに LINEAR で拡大して重ねる形になる。補間と重ね方は入れ替えられない（補間(A over B) ≠ 補間(A) over 補間(B)）ので、重なる部品の縁で今の答えから外れ、突き合わせた部品の継ぎ目に下の窓が透ける細い線が出うる。`replace` の切り抜き・`reduce` の全面への効き方も、拡大した座標の端数で縁が変わる。
**Impact**: 要件 8.1（`overlay` だけのシェルは今と差 1 以内）は画面に出る絵の約束だが、設計の檻（WARP・`screen_offscreen_gpu_tests.rs`）はどれも k＝1 なので、k＝1.25 の破れは檻をすり抜ける。継ぎ目が見つかったときの手当て（面の根を原寸のまとめにして拡大）は wintf の `VisualGroup` の約束（解像度の指定）を変えるので、タスクの後で出ると wintf の口と差分の範囲の規則を作り直すことになる。
**Suggestion**: 設計の段で描き方を決める。推奨は「`emo-surface` の下を原寸の 1 枚（原寸のまとめ）に描き、k で拡大して入れ物へ置く」で、今の見え方・`dump_surface`（原寸）と画面の拡大前の答えが同じ絵になる。`VisualGroup` に「原寸で描いて拡大する」を約束として足し、k＝1.25 の実 GPU の檻（継ぎ目の無いこと・今との差 1 以内）を足す。element ごとの拡大を選ぶなら、要件 8.1 を k＝1 に限る改訂を討議で決める。
**Traceability**: 8.1, 7.3, 7.8, 9.2
**Evidence**: design.md「GpuAtlas」（葉を LINEAR の `DrawBitmap`）・「Presenter の変更」の mount（`Arrangement{0, k, native}`）・「研究の項目」R5・OPEN QUESTION 5

🔴 **重大な問題 2**: pattern定義の `base` の意味が `plan_base` の 1 か所に閉じていない
**Concern**: 設計は `is_implemented` を `Base` で真にし、`base` の置き換え（XY を無視・層 (i) を置き換え）を `plan_base.rs` の「ルートの今のコマと着せ替えの pattern0」だけで扱う。ところが同じ門を通るほかの歩き方は、`base` を「(x, y) に入る overlay」として扱ったままになる。`plan.rs:623` の `always` の経過 0 の pattern（`always_rest_target` を通って `level.enter(p.surface_id, p.x, p.y)`）・入れ子の面のコマ（`push_direct` は入れ子の段でも同じ枝）・`nesting.rs:196`・`:308` の見える部品・`plan_bounds.rs:154` のシェルの外形（門が真になって `pattern.x, pattern.y` で数える）・`plan_extent.rs:208`（`pattern_word` が `Draw` なので今も `pattern.x, pattern.y` で数える）。
**Impact**: `always` の `base` と入れ子の面の `base` は置き換えでなく重ねで描かれ（4.1・4.2 の破れ）、XY が 0 でない `base` では外形と描いた位置が食い違う。XY が負なら外形が描いた絵より狭くなり、置き換えた絵が切れる。「描けるか」と意味の答えが歩き方ごとに分かれ、6.1 の「1 か所」に反する。
**Suggestion**: `base` の XY を 0 にし、置き換えの印を付けるのを読みの後の 1 か所（`fold.rs` の pattern の正規化か、`plan_base` の判定を共有する関数）で済ませ、`plan_extent`・`plan_bounds`・`nesting`・`plan_always` がその値を読むだけにする（`plan_extent.rs` の本体を触らずに済む）。`always` の `base` と入れ子の面の `base` の答え（置き換えるのはどの面の層 (i) か）を規則に足し、計画の値のテストで固める。
**Traceability**: 4.1, 4.2, 4.5, 3.5, 6.1
**Evidence**: design.md「PlanBuilder と DrawPlan」の規則（`base` の置き換え）・「Modified Files」の `nesting.rs`・`plan_always.rs`・`plan_bounds.rs`（「門は同じ 1 関数のまま（答えが広がる）」）・Requirements Traceability 3.5

🔴 **重大な問題 3**: 自動の上げ下げと「画面と同じ答え」の関係が決まっていない
**Concern**: wintf は、後に描く中身と重ならず `SOURCE_OVER` だけの部分木を別の描画面へ自動で上げる（`promotion_shape.rs`）。最後に描くまばたきのコマは上がりうる。設計の「SurfaceAlpha」は上がった入れ物も読んで α を `1-(1-a)(1-b)` で重ねるとする一方、「Performance & Scalability」は「上げは無くなる」とし、`screen_offscreen_gpu_tests.rs` は `emo-surface` の入れ物 1 枚（`read_all`）と `last_shown` のバイト一致を檻にしている。
**Impact**: 上がると `emo-surface` の入れ物にまばたきが入らず、7.3・9.3 の一致の檻が落ちるか、上がらない形でしか確かめられない。当たりの α は重ねで補えても、範囲だけ＝全部の一致・所要の測り（8.2）の前提が上がるかどうかで変わる。
**Suggestion**: どちらかに決めて書く。⑴ `emo-surface` の下は上げない（重大な問題 1 の原寸のまとめにすれば `ancestor_layer` と同じ扱いで自然に上がらない）。⑵ 上げてよいとし、「画面の答え」を入れ物を重ねた値と定めて、一致の檻と当たりの読みを同じ定義にそろえる。Performance の文と SurfaceAlpha の文を同じ答えにそろえる。
**Traceability**: 7.1, 7.3, 7.7, 8.2, 9.3
**Evidence**: design.md「SurfaceAlpha」・「Performance & Scalability」・「GPU-presence Tests」の `screen_offscreen_gpu_tests.rs`

## 設計の良い所

- **1 つの計画から 1 つの描き方へ**: `DrawPlan` を compose の純粋な値にし、画面・`dump_surface`・`--offline-dump`・WARP の檻が同じ `DrawNodes` の写しと wintf の同じ描き方を通る。compose は wintf を持たないまま、「描けるか」（`is_implemented`）・語 → `Mix`（`mix_of`）・`Mix` → D2D（`mix_mode_of`）がそれぞれ 1 か所で、後の `draw-methods-blend` が読み手に触らずに乗れる。
- **既存の土台の正しい読み**: 重ね方の写し（`SOURCE_ATOP`・`DESTINATION_OVER`・`DESTINATION_IN`・切り抜きの中の `SOURCE_COPY`）が正典と `research.md` §10 の測りに合い、wintf の既存の約束（`SOURCE_OVER` でない件は切り抜きまで広がる・差分は全面・その部分木は上げない）とも合っている。`ComposeError` は `#[non_exhaustive]` なので `Render` を足しても `dump_surface.rs` を触らずに済み、`EmoPresenter` は `!Send` なので `last_shown` の遅い満たし（`OnceCell`）も型の上で成り立つ。取り下げと移しの表もファイルごとに理由がある。

## 最終の判断

- **判断**: GO
- **理由**: 層と依存の向き・要件の全部への対応・実装の道筋ははっきりしており、根本の衝突は無い。上の 3 点はどれも設計の中の決め（描き方の約束・`base` の正規化の置き場・上げ下げの扱い）で閉じられ、作り直しを要しない。
- **次の手順**: 3 点を設計の討議（`/kiro-design-discussion draw-methods-canon`）で決め、design.md に反映してから `/kiro-spec-tasks draw-methods-canon` へ進む。
