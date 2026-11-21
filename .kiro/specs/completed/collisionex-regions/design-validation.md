# 設計の検証: collisionex-regions（2 回目・決め 7 の後の設計）

- 対象: `.kiro/specs/completed/collisionex-regions/design.md`（2026-11-17・決め 7「形は ID で塗る」の後に作り直した版・未承認）
- 照らした資料: `requirements.md`（承認済み・決め 7 まで）・`research.md`（9〜11 節）・`.kiro/specs/completed/hit-id-pass/brief.md`（最後の節の申し送り）・前回の本書（初版の設計への 3 件の指摘）・steering（`product.md`・`tech.md`・`structure.md` のテストの命名と 1,000 行）
- 照らしたコード: `crates/areka-parsers/src/shell/{model,decode,parse,boxes,cursor}.rs` と檻・`crates/areka-emo-compose/src/{hit,hit_import,fold,world,plan_test_support}.rs` と `{fold_merge,row_budget,log_firing,base_image}_tests.rs`・`crates/areka-emo-present/src/{shell_target,presenter/hit,presenter_test_support,shell_target_test_support,shell_target_element_base_tests}.rs`・同梱の検体 `vendors/sample_ghost/*.nar` の `surfaces.txt`
- 進め方: 対話なし（設計の討議へ渡す）

## 設計の検証の要約

決め 7 で形の内外の判定（CPU の式）と予算の「重さ」をやめたので、設計は「型を形の列挙へ替える・`decode.rs` に兄弟の読みを足す・`hit.rs` の述語を `rect` だけの網羅の `match` に替える・`hit_import.rs` のずらしを形ごとにする・`fold.rs` の `cuts` を消して警告を足す」の 5 か所に縮み、新しい本番のファイルも無い。コードと照らした主張はどれも事実どおりで、要件の 1〜8 の全項目に担い手と檻がある。残る重大な指摘は、同じブレスの中の番号の重なりの警告が承認済みの要件 3.7 の字面と食い違う点と、クローディアの記録の確かめが実機だけに頼っている点の 2 件で、どちらも設計の討議で数行を直せば片づく。

## 前回の指摘の行方

| 前回の指摘 | 今の設計 | 判定 |
|---|---|---|
| 1 予算を「重さ」で数える変更の波及（`plan.rs`・`nesting.rs`・`outline_walk.rs`） | Non-Goals と Out of Boundary で「予算の数え方は変えない・多角形も 1 件 1」とし、`budget.rs`・`plan.rs`・`nesting.rs`・`outline_walk.rs` は「触らないことを確かめるファイル」へ | 問題ごと消えた |
| 2 同じ番号の警告の範囲の食い違い（「新設の面」） | Modified Files の `fold.rs` の行は「波括弧（`Source::Brace`）の行を足すときだけ」へ直り、FoldCollisions と一致した | 解消。ただし同じブレスの中の扱いは OPEN QUESTION 1 として残り、要件 3.7 の字面と合わない（下の指摘 1） |
| 3 `hit-id-pass` への受け渡しが純関数の `contains` と合わない | `hit_shape.rs` は作らず、Data Contracts は「型・並びの位置・ずらしの規則だけを約束し、判定の口の形（画像を渡すか）は `hit-id-pass` が決める・そのとき Revalidation Triggers に当たる」へ | 解消 |
| 細かい食い違い（`UnreadableNumber.text` の説明・`presenter_test_support.rs` 658 行の直書き） | どちらも Modified Files に入った | 解消 |
| 細かい食い違い（クローディアの檻の警告の固定） | research.md 11.5 で「檻は `warn!` の行数を固定していないので壊れない・記録は実機（8.6）で見る」 | 事実は正しい。ただし決定論の固定が無い（下の指摘 2） |

## コードと照らして確かめた主張

| 設計の主張 | 確かめた結果 |
|---|---|
| `Collision` の 4 欄を直に読む・作るのは parsers の檻・`hit.rs` 94 行・`hit_import.rs` 278〜281 行・`plan_test_support.rs`・`presenter_test_support.rs`（524・658 行）・`validation_tests.rs` 612 行・`fold_merge_tests.rs` 164 行だけ | 正しい。ワークスペース全体を引いて、ほかに `Collision {` の直書きも `.left` などの読みも無い（`crates/areka/` の `Collision {` は `install` と `areka-nar` の別物）。emo-present の本番 `presenter/hit.rs` は `hit_regions` と `hit_region_in` を通すだけ |
| 同じ collision番号の前の行を除くのは `Numbers.cuts` と最後の `retain` だけ | 正しい（`fold.rs` 95〜106・179〜194・291〜297 行） |
| element の二重定義の `warn!` は「足す前の面の番号の集合」で後の行を数える＝同じブレスの中の重なりは数えない | 正しい（`fold.rs` 270〜285 行・`numbers.elements` を伸ばすのは数えた後）。新設の面（`upsert_surface`）は `add_lines` を通らない |
| `ShellIssue` の出口は `shell_target.rs` の `parse_all` 1 回（`records.emits()` の下で `severity()` に従い `warn!`／`error!`） | 正しい（418・469〜476 行）。`boxes.rs`・`cursor.rs` も `parse_all` を呼ぶが射影だけで記録を捨てるので、`ShapesInactive` が二重に出ることは無い。`ShellIssue` を別 crate で `match` する所は無い |
| `collisionex` を持つ同梱の検体はクローディアだけ（138 行＝`rect` 78・`ellipse` 32・`polygon` 28） | 正しい（全 `.nar` を開いて数えた） |
| 同じ番号を 2 つのブレスに書いた同梱の検体は無い＝6.4 の改めで同梱の検体の記録は増えない | 正しい（えみり・クローディア・emo2 ほか全部を引いて 0 件・範囲と除外の記述子も展開して数えた） |
| 既存の檻でクローディアの `warn` 以上を 0 行と固定したものは無い | 正しい。クローディアのシェルを読む檻は `shell_target_element_base_tests.rs` の `claudia_base_surfaces_are_full_size_without_warnings` だけで、固定しているのは外形と `undrawn_count == 0`。`sample-ghost-kit/src/lib_tests.rs` の警告 0 件の檻は `dot_sakura`、`real-run-kit/src/launch_tests.rs` は名前の文字列だけ、`crates/areka/src/boot_*_tests.rs` はバルーンの `claudia` で、シェルを読まない。`ShapesInactive` の 1 行で赤になる既存の檻は無い |
| 既存の「値にしない」の檻は名前を最後の欄に書いた誤った書式 | 正しい（`decode_tests_lenient_input_tests.rs` 83・226 行・`validation_tests.rs` 553 行）。新しい読みでは形の欄が数（`10`・`5`）になり `UnknownShape` の記録が出るので、書き直し（8.8）は必須 |
| 新しい檻の名前 | `hit_order_tests.rs`（`hit` ＋ `order_tests`）・`decode_collisionex_tests.rs`（`decode` ＋ `collisionex_tests`）は structure.md の最長 stem・前向きの衝突禁止のどちらにも当たらない |

細かい食い違い（討議で 1 行直せばよいもの・重大ではない）:
- 4.7 の檻は `parse_all` を通す必要がある（`decode(lex(..))` の道は `Reading::finish` を呼ばないので `ShapesInactive` が出ない）。Testing Strategy の `decode_collisionex_tests.rs` の行に「4.7 と 6.4 は `parse_all` で」と添える。
- `decode_tests_element_collision_tests.rs` 19 行の説明「collisionex はここでは扱わない」は、直書きを書き換えるついでに直す（Modified Files にファイルはもう入っている）。
- `hit-id-pass` の brief の Upstream は「`collisionex` の行を読む読み手と 4 つの図形」のままで、今の契約（5 つの形を型へ運ぶ・`rect` だけが当たる）と合わない。7.6 は討議で書いた節なので、討議のうちに 1 語直す。

## 重大な指摘（2 件）

🔴 **指摘 1: 同じブレスの中の番号の重なりを警告しない仮定が、承認済みの要件 3.7 の字面と食い違う**
**Concern**: FoldCollisions と OPEN QUESTION 1 は「同じ波括弧の中だけで番号が重なる行は警告しない」と置く（element の二重定義の今の数え方と同じで、コードの上では正しい）。一方、要件 3.3 は重なりの場合として「同じブレスの中で 2 度書くとき」を明示し、3.7 は「同じサーフェスで同じ番号の当たり判定が重なったとき」に警告すると書き、例外は追記だけを挙げる。8.2 の「警告が 1 行」も、2 つのブレスと同じブレスの中を混ぜた並びの期待値として書かれている。
**Impact**: 仮定のまま実装すると、要件 3.7 の字面どおりの檻（同じブレスの中の重なりで警告 1 行）とぶつかり、レビューと `/kiro-validate-impl` で要件違反と読まれる。逆に字面に合わせると、element の警告と違う新しい数え方が要り、6.4（`collision*` だけのシェルの記録を変えるのは 2 つ以上のブレスに書いたときだけ）と食い違う。
**Suggestion**: 討議で OPEN QUESTION 1 を推奨案（同じブレスの中は警告しない・element と同じ）で決め、要件 3.7 に「同じブレスの中だけの重なりは警告しない（element番号の二重定義と同じ・SSP では測っていない）」を 1 文足す。8.2 の「警告が 1 行」は後の波括弧の重なりの期待値だと檻の説明に書き、§8 には「areka の裁量」として書く（設計の LedgerAndDocs の §8 の行はすでにそう書いている）。
**Traceability**: 3.3・3.7・6.4・8.2・7.4
**Evidence**: design.md「FoldCollisions」の Responsibilities・「Open Questions」1・「Testing Strategy」の `fold_merge_tests.rs`・`row_budget_tests.rs` の行

🔴 **指摘 2: クローディアの `ShapesInactive` の記録（数 32・28）が実機の確かめだけに頼り、既存の檻の名前と説明は着地の後に事実と合わなくなる**
**Concern**: 8.6 の「読み込みの記録にまだ働かない形の数が出る」を、設計は実機の確かめだけで見る（RealMachineCheck・research.md 11.5）。決定論の側は `decode_collisionex_tests.rs` の作り物の入力だけで、実の検体での数（範囲の見出し・追記を含むクローディアの 138 行が楕円 32・多角形 28 と数えられること）はどこにも固定されない。また、クローディアを実際に読む唯一の檻 `claudia_base_surfaces_are_full_size_without_warnings` は、名前も説明（「本 spec の警告は 0 件」）も「警告なし」を謳うが、着地の後はクローディアの読み込みで `warn!` が 1 行出る。
**Impact**: 数え方の誤り（追記や範囲の見出しで数え損ねる・二重に数える）が実機の段まで見つからず、実機の確かめの後に退行しても赤にならない。檻の名前が事実と食い違ったまま残る。
**Suggestion**: 同じ檻（既存の `capture` と `warn_lines` がそのまま使える）に「`warn` 以上はちょうど 1 行で、`ShapesInactive` の文言に楕円 32・多角形 28・円 0・region 0 が載る」の検査を 3〜4 行足し、名前か説明を「描けない行は無い・まだ働かない形の記録だけが出る」へ直す。`shell_target_element_base_tests.rs` を Modified Files と 8.6 のトレースに足す。実機の確かめ（8.5・8.6）はそのまま残す（静的の証跡と実機の証跡の二本立て）。
**Traceability**: 4.7・8.3・8.6
**Evidence**: design.md「RealMachineCheck」・「Testing Strategy」の Unit Tests の `decode_collisionex_tests.rs` の行・「Modified Files」（`shell_target_element_base_tests.rs` が無い）・research.md 11.5 の最後の項

## 設計の強み

1. **決め 7 を受けて、変える所を最小の 5 か所に絞りきっている**。幾何のコードも新しい本番のファイルも予算の変更も無く、`hit_region_in` の入出力・emo-present・areka 本体も変えない。`hits` と `shifted` をワイルドカードなしの網羅の `match` にし、`CollisionShape` に `#[non_exhaustive]` を付けないので、`hit-id-pass` が偽の腕を替える的は `hits` の 1 か所に見え、形を足せばコンパイルで漏れが見つかる。主張はワークスペース全体と同梱の検体で確かめて、どれも事実どおりだった。
2. **`hit-id-pass` との境界を「型・並びの位置・ずらしの規則」だけに引き直した**。判定の口の形と、境界・大きさ 0・反転の決めを相手の設計へ明示的に渡し、その変更が自分の Revalidation Triggers に当たると書いたので、前回の指摘 3 の誤読の余地が無くなった。brief の申し送り（巻き数の規則・凸包ではなく辺どおり・型の契約）とも一致している。

## 最終の判定

- **判定: GO**
- **理由**: 既存の構造との衝突は無く、前回の 3 件は解消か問題ごと消えた。残る 2 件は要件の 1 文の追記と既存の檻への数行の検査で、アーキテクチャを組み替える必要は無い。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion collisionex-regions`）で、指摘 1（OPEN QUESTION 1 と要件 3.7 の 1 文）・指摘 2（クローディアの檻の検査と名前）を議題にし、残りの OPEN QUESTION 2〜4 とあわせて決める。
  2. 「細かい食い違い」の 3 行（4.7 の檻は `parse_all` で・`decode_tests_element_collision_tests.rs` の説明・`hit-id-pass` の brief の「4 つの図形」）を設計と brief に足す。
  3. 設計の承認の後、`/kiro-spec-tasks collisionex-regions` でタスクを作る。
