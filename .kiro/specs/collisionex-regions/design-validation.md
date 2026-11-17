# 設計の検証: collisionex-regions

- 対象: `.kiro/specs/collisionex-regions/design.md`（2026-11-17 生成・未承認）
- 照らした資料: `requirements.md`（承認済み）・`research.md`（9・10 節）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C22）
- 照らしたコード: `crates/areka-parsers/src/shell/{model,decode,parse}.rs`・`crates/areka-emo-compose/src/{hit,hit_import,fold,budget,plan_test_support}.rs`・`crates/areka-emo-compose/src/{plan,nesting,outline_walk}.rs`（`surface_rows` の使い手）・`crates/areka-emo-present/src/{presenter_test_support,shell_target,shell_target_element_base_tests}.rs`・同梱の検体 `vendors/sample_ghost/claudia.nar` の `shell/master/surfaces.txt`
- 進め方: 対話なし（設計の討議へ渡す）

## 設計の検証の要約

設計は、読み（`decode.rs` の兄弟の関数）・形の算術（新しい `hit_shape.rs`）・重なり（`hit.rs` の述語の差し替え）・畳み込み（`fold.rs` の `numbers.cuts` の撤去）を既存の層と依存の向きのまま分けており、コードと照らした主張はほぼ正しい。要件の 1〜8 はトレース表で全部に担い手がある。直す所は、要件に無い「予算を重さで数える」変更の波及が境界から漏れていることと、同じ番号の警告の範囲の書き方の食い違いと、`hit-id-pass` への受け渡しの書き方の 3 点で、どれも設計の討議で数行を直せば片づく。

## コードと照らして確かめた主張

| 設計の主張 | 確かめた結果 |
|---|---|
| `collisionex` の行は `strip_prefix("collision")` の残り `ex0` が数字だけでないので記録なしに飛ぶ | 正しい（`decode.rs` 245〜271 行）。新しい枝を先に置けば取り違えない |
| `Collision` の 4 欄を読むのは emo-compose の `hit.rs`・`hit_import.rs` と檻だけ・areka 本体は名前の文字列だけ | 正しい。`Collision {` の直書きは parsers の檻・`plan_test_support.rs`・`presenter_test_support.rs`（524 行の `hit_coll` と 658 行）・`hit_import.rs` の `import` だけ。`crates/areka/` の `Collision {` は `CaseCollision` で別物。`.left` などの読みは `hit.rs` 94 行・`validation_tests.rs` 612 行・`fold_merge_tests.rs` 164 行 |
| 同じ collision番号の前の行を除くのは `Numbers.cuts` と最後の `retain` だけ | 正しい（`fold.rs` 95〜106 行・179〜194 行・291〜297 行）。消せば両方が残る |
| element番号の二重定義の `warn!` は後の行のうち面にすでにある番号を数える形 | 正しい（`fold.rs` 270〜285 行）。ただし element の警告は追記でも出る（`source` を見ない）。collision の警告を波括弧だけにする設計は要件 3.7 どおり |
| `ShellIssue` の出口は `shell_target.rs` の `parse_all` 1 回 | 正しい（`shell_target.rs` 418 行）。`ShellIssue` を別 crate で `match` する所は無い |
| 楕円の判定 `(u·h)² + (v·w)² <= (w·h)²` | 正しい（中心 `(l+r)/2`・半径 `w/2` の式を 2 倍して整数にしたもの）。囲みの中で `u ≤ w`・`v ≤ h` なので、`w·h < 2^64` の間は `u128` で厳密 |
| 円は `i64` の全域で厳密 | 正しい（囲みで `dx`・`dy ≤ radius < 2^63`、二乗の和 `< 2^127`） |
| クローディアは `rect` 78・`ellipse` 32・`polygon` 28、多角形は 5〜7 頂点 | 正しい（検体を開いて数えた）。欄の数は 7・13・15・17 で、末尾の `,` も同じ面の同じ番号も無い＝新しい記録は出ない見込み。`collisionex` を持つ同梱の検体はクローディアだけ |
| 既存の「値にしない」の檻は名前を最後の欄に書いた誤った書式 | 正しい（`decode_tests_lenient_input_tests.rs` 83・226 行・`validation_tests.rs` 553 行）。156 行は未知のブレスの中なので読まれず、変わらない |

細かい食い違い（討議で 1 行直せばよいもの・重大ではない）:
- 設計は「`shell_target_element_base_tests.rs` のクローディアの檻の『warn 以上 0 行』」を引くが、その檻（`claudia_base_surfaces_are_full_size_without_warnings`）が固定しているのは `undrawn_count == 0` と外形だけで、`warn!` の行数は固定していない。新しい記録が出ないことを決定論で固定するなら、この檻に `warn_lines(&events)` の空を足すか、実機の確かめだけで見る旨に書き直す。
- 既存の `ShellIssue::UnreadableNumber.text` は「飛ばした欄の原文」（`collision*` では見出しの欄 1 つ）で、設計は `collisionex` の行では「欄を `,` で繋いだ行」を入れる。同じ腕で `text` の中身が綴りによって違うので、`model.rs` の説明の 1 行を直す。
- `presenter_test_support.rs` の 658 行を `CollisionShape::Rect(CollisionBounds { .. })` で直書きすると rustfmt で行が増えうる。`hit_coll(0, 4, 0, 8, 4, "Body")` へ替えれば「行を増やさない」が守れる。

## 重大な指摘（3 件）

🔴 **指摘 1: 予算を「重さ」で数える変更の波及が境界とファイルの計画から漏れている**
**Concern**: 設計は `budget::surface_rows` を重さ（多角形は頂点の数）で数え直すと決めるが、`surface_rows` の使い手は `hit_import.rs` だけでなく、合成の走査 `plan.rs`（433 行）・`nesting.rs`（138 行の `surface_rows(m) < VISIT_BUDGET`）・`outline_walk.rs`（146 行）にもある。この 3 つは Modified Files にも Revalidation Triggers にも檻にも出てこない。持ち込みの側も、`regions_of` の持ち越しの切り方（`done.out.drain(..total - carried)`）・`HitMemo::insert` の `list.len()`・覚えを使うときの `budget.take(n + list.len())`・`spent` と `carried` の足し算・`attach_hit_regions_limited` の `imported` が件数で結びついており、一部だけを重さにすると予算が食い違う。
**Impact**: 多角形を持つシェル（クローディアがそう）で、当たり判定と関係の無い合成の走査の予算まで黙って変わる。要件はどこも予算の数え方を求めておらず（OPEN QUESTION 1 の仮定）、要件 6.4（`collision*` だけのシェルは変えない）は守れても、変える範囲が設計の書く境界より広い。
**Suggestion**: 討議で OPEN QUESTION 1 を決める。採るなら、`plan.rs`・`nesting.rs`・`outline_walk.rs` を「振る舞いが変わるファイル」として Boundary と Revalidation Triggers に足し、件数で結びついた所（持ち越しの切り方・`HitMemo`・`take`・`imported`）を一覧にして、重さで切る檻を `row_budget_tests.rs` に置く。走査の予算を変えたくないなら、`surface_rows` は件数のままにし、重さは写しの上限（`body_lines`）と持ち込みの量（`IMPORT_BUDGET`・`HitMemo`）だけに当てる案もある。
**Traceability**: 3.4・3.5・6.4（Boundary Commitments「行の重さの数え方」）
**Evidence**: design.md「HitImport」の Responsibilities（`budget::surface_rows`）・「Modified Files」の `budget.rs`・「Revalidation Triggers」・「Open Questions」1

🔴 **指摘 2: 同じ collision番号の警告の範囲が設計の中で食い違い、要件 8.2 と読み分かれる**
**Concern**: 「Modified Files」は `fold.rs` について「`add_lines`（波括弧）と**新設の面**に collision番号の二重定義の `warn!` を足す」と書くが、「FoldCollisions」と OPEN QUESTION 2 は「同じ波括弧の中だけで番号が重なる行は警告しない」と書く。新設の面（`upsert_surface`）で警告を出せば、それは同じブレスの中の重なりの警告になる。要件の側も、3.7 は「同じサーフェスで同じ番号の当たり判定が重なったとき」と広く、8.2 は「同じ番号を 2 つのブレスと同じブレスの中に書いた並び（両方が残り後の方が手前・警告が 1 行）」と書き、6.4 だけが「2 つ以上のブレスで書いた」ときに限る。SSP の測り（`research.md` 9 節）は 2 つのブレスにまたがる重なりだけで、同じブレスの中は測っていない。
**Impact**: 実装係がどちらの書き方に従うかで、`fold_merge_tests.rs` の期待値と `collision*` だけのシェルの記録（6.4）が変わる。
**Suggestion**: 「Modified Files」の「と新設の面」を消し、FoldCollisions の書き方（波括弧が足す行のうち面にすでにある番号だけ・記述子ごとに 1 行）にそろえる。討議で OPEN QUESTION 2 を推奨案（同じブレスの中は警告しない）で確定し、8.2 の「警告が 1 行」は 2 つのブレスの重なりについての期待値だと檻の名前と説明に書く。同じブレスの中は SSP で測っていないので、§8 には「areka の裁量」として書く。
**Traceability**: 3.3・3.7・6.4・8.2・7.4
**Evidence**: design.md「Modified Files」の `fold.rs` の行・「FoldCollisions」の Responsibilities・「Open Questions」2・「Testing Strategy」の `fold_merge_tests.rs`

🔴 **指摘 3: `hit-id-pass` への受け渡しの書き方が、純関数の `hit_shape::contains` と合わない**
**Concern**: 「Data Contracts & Integration」は「`hit-id-pass` は `hit_shape::contains` の `Region` の腕を書き換えて働きを足す（読み手は触らない）」と書く。一方で `hit_shape.rs` は「`std` と parsers の型だけ・I/O なし・割り当てなし」の純関数で、`contains(shape, x, y)` には画像の画素を渡す口が無い。`region` の働き（指定色の画素を当たりにする）には画像（またはそれから作った当たりの面）が要るので、`hit-id-pass` は `contains`・`hit_region_in`・`hit_region_scaled_in` の入出力の形を変えるか、`Collision` の外に面ごとの当たりの面を持つ必要がある。これは設計が Revalidation Triggers に挙げる「`hit_region_in` の入出力の形を変えたとき」に当たる。
**Impact**: 受け渡しの約束が「腕を 1 つ書き換えるだけ」と読めると、`hit-id-pass` の見積もりと境界（emo-present の `presenter/hit.rs` まで届くか）を誤る。この仕様の型（`CollisionRegion { file, red, green, blue, invert, x, y }`）そのものは要件 5.1〜5.3 を満たしており、直すのは約束の書き方だけ。
**Suggestion**: 「Data Contracts & Integration」を「`hit-id-pass` は `CollisionRegion` の欄と並びの中の位置（重なりの順）と `(x, y)`（親の面の座標での画像の左上）を読む。画像から作る当たりの面の持ち方と、判定の口の形（画像を渡すかどうか）は `hit-id-pass` が決め、そのときこの設計の Revalidation Triggers に当たる」に書き直す。あわせて、`(x, y)` が表示の拡大率の戻し（2.11）の後のサーフェスの座標であることを 1 行添える。
**Traceability**: 5.1・5.2・5.3・2.8（Boundary「Out of Boundary」の `hit-id-pass`）
**Evidence**: design.md「Data Contracts & Integration」・「HitShape」の Responsibilities と Invariants・「Revalidation Triggers」

## 設計の強み

1. **層と依存の向きを崩さずに、変える所を 1 か所ずつに絞っている**。幾何は `hit_shape.rs` だけ、重なりは `hit.rs` の述語 1 つ、同じ番号の扱いは `fold.rs` の `cuts` の撤去、読みは `decode.rs` の兄弟の関数で、`hit_region_in` の入出力・emo-present・areka 本体は変えない。コードと照らした主張（4 欄の読み手・`cuts` の所在・出口の 1 か所・クローディアの行の数）はどれも事実どおりで、`#[non_exhaustive]` を付けずに網羅の `match` を保つ判断も既存の `ShellIssue`・`RegionPriority` の前例にそろっている。
2. **判定を整数と検査つきの算術だけで決め、境界と崩れた入力の結果を全部決めている**。楕円・円の式は検算して正しく、溢れ・反転・負の半径・`region` はどれも「当たらない」という決まった結果に落ちる。読みの判定の順を図で固定し 1 行 1 件の記録にしたので、決定論の檻が理由を名指しで固定できる（要件 2.9・2.10・4.1〜4.10）。

## 最終の判定

- **判定: GO**
- **理由**: 既存の構造との衝突は無く、要件の全項目に担い手と檻がある。3 件の指摘はどれも書き方と範囲の明示の直しで、アーキテクチャを組み替える必要は無い。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion collisionex-regions`）で、指摘 1（OPEN QUESTION 1 と波及するファイル）・指摘 2（OPEN QUESTION 2 と「新設の面」の削除）・指摘 3（受け渡しの書き直し）を議題にし、残りの OPEN QUESTION 3〜6 とあわせて決める。
  2. 「細かい食い違い」の 3 行（クローディアの檻の警告の固定・`UnreadableNumber.text` の説明・`hit_coll` の使い回し）を設計に足す。
  3. 設計の承認の後、`/kiro-spec-tasks collisionex-regions` でタスクを作る。
