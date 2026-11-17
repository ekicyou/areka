# ギャップ分析: collisionex-regions

> 2026-11-17・`/kiro-validate-gap`。入力は `requirements.md`（未承認・要件生成直後）・`brief.md`（「2026-11-17 棚卸㊴の再測定」の節まで）・steering（`structure.md` のテストと 1,000 行の規律・`roadmap.md` のウェーブ C22 の行）。測った基準は作業ツリーの HEAD（`58acf1a38`＝main `5749a4c7` の上に spec の初期化だけ）。本書は情報と選択肢を並べるもので、決めは要件の討議・設計の段に委ねる。

## 1. 分析の要約

- **読み手・型・解決の 3 か所とも「矩形 1 種」を前提に書かれているが、並び（1 本の `Vec<Collision>`）・番号による置き換え（`fold.rs` の `cuts`）・画家の順（逆順走査）・子からの持ち込み（`hit_import.rs`）・記録の出口（`ShellIssue` → `shell_target.rs` の `warn!`）はそのまま使える。** 足りないのは「形」の欄と、形ごとの内外の判定と、形ごとのずらし方と、`collisionex` の行の読みと記録の種類だけ。
- **下流（Reference4・カーソルの形）は名前の文字列しか受け取らない**（`EmoPresenter::hit_region_client` → `ClientHit.region: Option<&str>`）。`hit_region_in` の入出力の形を保てば areka 本体（`crates/areka/`）は 1 行も触らずに要件 6 が満たせる。本体で `areka_parsers::shell::Collision` を組む所は 0 件（`install/judge.rs` の `CaseCollision` は別物）。
- **候補は 3 つ**: A＝既存ファイルへ全部足す／B＝読み・判定ともに新しいファイルへ／C＝読みは `decode.rs` の兄弟の関数として足し、形の判定だけを emo-compose の新しいファイルへ置く（ウェーブ C22 の表の「形の検査の新しいファイル」と一致）。規模は M（8〜11 タスクの見込みのまま）・危険は低〜中。
- **見落としやすい所**: ⑴ 楕円・円の整数の判定は i64 の座標で i128 でも溢れうる（要件 2.10） ⑵ 幅 0 の楕円を式だけで判定すると縦の無限の直線に当たる（仮定 A8 の「線分」にならない） ⑶ 持ち込みの予算と覚え（`IMPORT_BUDGET`・`HitMemo`・`surface_rows`）は領域の「数」で数えるので、頂点の多い多角形の写しが予算の外で膨らむ ⑷ 台帳の `collision*` を `implemented` にすると定義箇所に正典 URL の証拠と報告の作り直しが要る ⑸ `collisionex` を「吸収される」と固定した既存の檻 4 本は書式も誤っており（名前を最後の欄に書いている）、書き直しになる。
- **調べが要る所**: 正典が沈黙する決め（交差する多角形の塗り方・楕円と円の境界・形の名前の大小文字）は、開発者の方針（ukadoc で読めない点は議題の前に SSP の MCP で測る）に照らすと SSP で測れる。測りには毎回開発者の許しと排他テストの机が要る。

## 2. 今の状態（調べた資産）

### 2.1 読み手（`crates/areka-parsers/src/shell/`）

| 資産 | 今の働き | この spec との関係 |
| --- | --- | --- |
| `model.rs:199-231` `Collision { index: u32, left, top, right, bottom: i64, name: CollisionName }` | 矩形だけ。`#[derive(Clone, Debug, PartialEq)]`。`CollisionName` は不透明な NewType | 形の欄が無い。ここが中心の変更点 |
| `model.rs:65-74, 233-246` `Surface.collisions`・`SurfaceAppend.collisions`（どちらも `Vec<Collision>`・出現順） | 1 本の並び | 要件 1.8（`collision*` と `collisionex*` を書いた順に 1 本へ）はこの並びに混ぜれば満たせる |
| `decode.rs:236-272` `decode_collisions` | `collision` の後が数字だけの行を値にする。`collisionex0` は後が `ex0` なので**記録なしに飛ぶ**。欠けた・数でない座標は `field_i64` で 0 | `collisionex` の分岐を `collision` の分岐より先に置く必要がある（`strip_prefix("collision")` は `collisionex` にも当たる） |
| `decode.rs:34-66` `Reading`（見出し・崩れの列） | `unreadable`・`too_wide`・`wait_range` の 3 つの口 | 新しい理由の口を足す場所 |
| `model.rs:303-356` `ShellIssue`（6 種・`#[non_exhaustive]` なし・`severity()`／`message()` の網羅 match は model.rs だけ） | 記録の型 | 種類を足しても crate の外の match は壊れない（`severity`・`message` だけ） |
| `parse.rs` `parse_all` → `ShellText.issues` | 崩れを登場順に返す。記録は出さない | 変更不要 |
| `lexer.rs:191-217` `split_csv` | 欄を `,` で分けて前後の空白を除く。`[...]` の中の `,` は守る。**本体の行の原文と行の番号は捨てる**（`Token::Brace.body: Vec<Vec<String>>`） | 要件 4.9「行の原文を添え」は欄を `,` で繋ぎ直した近似になる（後述 5 の議題 11） |

### 2.2 合成と当たり判定（`crates/areka-emo-compose/src/`）

| 資産 | 今の働き | この spec との関係 |
| --- | --- | --- |
| `hit.rs:80-97` `hit_region_in(&[Collision], x, y, RegionPriority) -> Option<&str>` | 逆順走査で最初に当たった矩形の名前（閉区間・反転は並べ直さず当たらない） | `find` の述語を「形ごとの内外」へ替えるだけで画家の順は保てる。`region` は述語が偽を返せば「下を続ける」（要件 2.8）になる |
| `hit.rs:143-158` `hit_region_scaled_in` | ÷k してから `hit_region_in` へ委譲 | 変更不要（要件 2.11 は構造的に満たされる） |
| `hit.rs:42-48` `RegionPriority`（`Painter` だけ・match にワイルドカードを置かない規律） | 重なりの規則の型の継ぎ目 | 変更不要。`collision-sort` は読まない（要件 3.6） |
| `hit.rs:160-677` 檻（**同じファイルの中の `mod tests`**） | 矩形の 5 分岐・k=2・k=1.25・反転・極値 | 677 行。新しい檻は structure.md の規律で兄弟ファイルへ置く |
| `hit_import.rs:268-285` `import`（`left..bottom` を飽和で足す） | 子の領域を element定義の X,Y だけずらす | 形ごとのずらし（要件 3.4）に替える。多角形は全頂点・円は中心・`region` は置く位置 |
| `hit_import.rs:261-264` `names_of`（親に直接書いた名前の集合） | 同じ名前の子の領域を持ち込まない | `region` の名前もここに入る（後述 5 の議題 9） |
| `hit_import.rs:43, 116-139` `IMPORT_BUDGET`・`HitMemo`（列の**長さ**で数える） | 持ち込みの量の上限 | 多角形 1 件の重さ（頂点の数）が数えられない（議題 8） |
| `budget.rs:19-32` `surface_rows`（`collisions.len()` を行として数える） | 広がりの予算 | 同上 |
| `fold.rs:94-104, 290-302` 波括弧の同じ collision番号の前の行を `c.index` で除く・追記は重ねて足す | 番号の置き換え | `Collision.index` を `collisionex` にも入れれば、要件 3.2・3.3（仮定 A2）は**コードの変更なしに**満たされる |
| `world.rs:166-174` `EmoWorld::hit_regions` | 持ち込み済みの列か `SurfaceMaster.collisions` | 変更不要 |
| `normalized.rs:79-85` 順序の不変条件（登場順・追記は末尾） | 画家の順の前提 | 変更不要 |

### 2.3 下流・記録の出口・台帳・文書

- **表示の層**: `crates/areka-emo-present/src/presenter/hit.rs`（`hit_region_client` が `debug!` 1 行に k・座標・region を残す＝実機の確かめの grep の的）。`shell_target.rs:471-474` が `ShellIssue` を読み込みのたびに `warn!`／`error!` で出す（新しい種類も自動で出る）。
- **areka 本体**: `input_events/{click,drag,char_watch,hover_region,cursor_answer}.rs` は `hit.region`（文字列）だけを使う。カーソルの形の行（`cursor_pick.rs`・`cursor_answer.rs`）も名前で引く。**変更不要**（要件 6.1〜6.4・7.4・ウェーブ C22 の約束 ⑴）。
- **台帳**: `doc/ukadoc-coverage/ledger/assets.toml` の 7018 行（`collision*`・`degraded`・担当 `areka-P0-shell-parse`）と 7031 行（`collisionex*`・`absent`・担当なし・`introduced = "2.5.19"`）。`cargo test -p ukadoc-survey` が台帳を検査し、`implemented` には定義箇所の正典 URL のコメント（`// ukadoc: https://…#<anchor>:1`）が要る（`ImplementedWithoutEvidence`）。台帳を触ったら `report/assets.md`・`report/summary.md` を作り直す（`DomainReportStale` でバイト一致を検査）。
- **§8**: `doc/COMPAT_ARCHITECTURE.md` 124 行目からの表。関連の前例＝276 行（同じ collision番号の扱い・画家の順と同じ向き）・194／225 行（キーワードの大小文字を問わない前例）・255 行（小文字の完全一致の前例）。
- **既存の檻で `Collision {` を直書きする所**: parsers のテスト 6 本（約 20 か所）・emo-compose の `plan_test_support.rs::coll`・`hit_import.rs::import`・emo-present の `presenter_test_support.rs::hit_coll` と 658 行の 1 か所（ウェーブ表と一致）。`presenter_test_support.rs` は 890 行で 1,000 行の目安に近い。
- **陳腐化する檻**（`collisionex` を「値にしない」と固定している）: `decode_tests_lenient_input_tests.rs` の `collisionex_is_absorbed_but_plain_collision_survives`（76 行〜）・156 行と 226 行の断片・`validation_tests.rs` の 530〜610 行。どれも名前を最後の欄に書いた誤った書式（例 `collisionex0,ellipse,10,20,30,40,Face`）なので、新しい読み手では「形の名前が未知」の記録が出る。書き直しになる。

### 2.4 実データ（同梱の検体）

- 同梱の `.nar` 10 本と取り寄せの `pasta-in-windows` を走査した結果、`collisionex` を持つのは**クローディアだけ**（138 行＝`rect` 78・`ellipse` 32・`polygon` 28・`circle`／`region` 0）。`collision*` の行は 0。名前は `Head`・`Face`・`Bust`・`Hair`・`Foot`・`Fan`・`Skirt`・`Body`・`Tray`。
- `rect`・`ellipse` の行は全部ちょうど 4 座標（欄 7）。`polygon` は 5〜7 頂点で座標は全部偶数個。大文字の形の名前・余分な座標・欠けた座標は無い＝クローディアの読み込みで新しい記録は出ない見込み（`shell_target_element_base_tests.rs` のクローディアの檻の「warn 以上 0 行」は崩れない見込み）。
- 実機の手順の雛形は完了 `areka-P0-mouse-drag-events` の `verification/real-machine.md`（根は `<ワークツリー>\target\mde-signoff\root`・`real-machine-run.ps1 -Prepare`・`RUST_LOG=info,areka=debug,kanade=trace,…`）。

## 3. 要件と資産の対応（ギャップの印: Missing＝無い／Unknown＝調べが要る／Constraint＝制約）

| 要件 | 使える資産 | ギャップ |
| --- | --- | --- |
| 1.1〜1.5 形ごとの読み | `decode_collisions`・`field_string` | **Missing**: `collisionex` の分岐・形の語の見分け・形ごとの欄の数の検査 |
| 1.6 名前は書いたまま | `CollisionName` | なし（空の名前も今と同じに運ぶ） |
| 1.7 形の名前の大小文字 | §8 の大小文字の前例（194・225 行） | **Unknown**（A5・SSP で測れる） |
| 1.8 1 本の並び | `Vec<Collision>` | **Missing**: 形の欄（型の変更） |
| 1.9 既存の `collision*` | `decode_collisions` | **Constraint**: 欠けた座標を 0 にする今の振る舞いを保つ（`collisionex` だけ行を捨てる＝2 つの規則が並ぶ） |
| 2.1〜2.5 形ごとの内外 | `hit_region_in` の述語 | **Missing**: 楕円・円・多角形（偶奇＋辺の上）の整数の判定 |
| 2.6〜2.7・A8 反転・負の半径・大きさ 0 | 矩形の「並べ直さない」前例 | **Missing**: 幅 0 の楕円は式だけだと縦の直線全体に当たる＝囲みの長方形の内外を先に見る必要 |
| 2.8 `region` は当たらず下を続ける | 逆順の `find` | 述語が偽を返すだけ |
| 2.9〜2.10 決定論・極値 | i64 の座標・`unscale_coord` の飽和 | **Missing／Constraint**: 楕円の `a²b²`・円の `dx²+dy²` は i128 でも溢れうる（議題 5） |
| 2.11 表示の拡大率 | `hit_region_scaled_in` | なし（委譲で保たれる） |
| 3.1〜3.3 重なり・追記・番号の置き換え | 逆順走査・`fold.rs` の `cuts` | なし（`index` を共有すれば A2 は自動で満たされる） |
| 3.4〜3.5 子からの持ち込み | `hit_import::import`・`names_of` | **Missing**: 形ごとのずらし。**Unknown**: `region` の名前で子を落とすか（議題 9） |
| 3.6 `collision-sort` | `RegionPriority::Painter` | なし |
| 4.1〜4.10 記録 | `Reading`・`ShellIssue`・`shell_target.rs` の出口 | **Missing**: 理由つきの記録の種類。**Constraint**: 行の原文は lexer で失われている（議題 11）。**Unknown**: `region` の欄が 6 つ以上のときの扱いが要件に無い |
| 5.1〜5.3 `region` の欄 | — | **Missing**: 型の欄。**Unknown**: 子から持ち込んだ `region` の「置く位置」を持つ欄（構文に無い・議題 3） |
| 6.1〜6.4 下流 | `ClientHit.region`・本体の `input_events` | なし（本体に触らない） |
| 7.1〜7.2 台帳 | `assets.toml` 2 行・ukadoc-survey の検査 | **Constraint**: `implemented` は正典 URL の証拠が要る・報告 2 本の作り直し（議題 13） |
| 7.3 §8 | 表の既存の行 | **Missing**: 新しい行（正典の URL と逐語の引用） |
| 8.1〜8.4 決定論の檻 | `hit.rs` の檻の作り（5 分岐・ハードコードの期待値） | **Missing**: 新しい兄弟の檻。陳腐化する檻 4 本の書き直し |
| 8.5〜8.7 実機 | mouse-drag-events の手順・`hit_region_client` の `debug!`・排他テストの机 | **Constraint**: 画面の操作は排他テストの机・SendInput。`circle`・`region` はクローディアに無い＝決定論だけ |

## 4. 実装の候補

### 共通（どの候補でも同じ）

- 型: `Collision` の `left/top/right/bottom` を形の列挙（例 `shape: CollisionShape { Rect{left,top,right,bottom} | Ellipse{…} | Circle{cx,cy,r} | Polygon(Vec<(i64,i64)>) | Region{file,r,g,b,invert,…} }`）へ置き換えるのが素直。`index` と `name` は残す（`fold.rs` と `names_of` はそのまま動く）。檻の作り手を減らすため `Collision::rect(index,l,t,r,b,name)` のような作り方を 1 つ用意すると、`coll`・`hit_coll` と parsers のテストの直書き約 20 か所の書き換えが 1 行ずつになる。
- 退けた形: `collisionex` を別の `Vec` に分ける形は、要件 1.8・3.1（書いた順の 1 本の並び・混ぜた重なり）を満たすために並びの索引（`DefRef` のような）がもう 1 つ要り、`fold.rs`・`hit_import.rs`・`hit.rs` の全部が 2 本を扱うことになる。
- 本体・依存・emo-text: どの候補も areka 本体・`Cargo.lock`・emo-text に触らない（C22 の約束 ⑴⑸）。`log-capture-kit` の上限越えの表にも触らない（約束 ⑵＝どのファイルも 1,000 行を越えない見込み: `decode.rs` 680→約 800・`hit.rs` 677→約 700）。

### 候補 A: 既存ファイルへ足す

- 読み: `decode.rs` の `decode_collisions` に `collisionex` の分岐と形ごとの欄の検査を足す。判定: `hit.rs` の述語を `match` に広げ、楕円・円・多角形の判定も `hit.rs` に置く。ずらし: `hit_import.rs::import` を `match` に。
- ✅ 新しいファイル 0（ウェーブ表の「触るファイル」に完全に収まる）・差分が最小。
- ❌ `hit.rs` は「純関数の照合」と 500 行の檻を抱えており、形の算術（i128・偶奇・辺の上）が混ざると読みにくい。新しい檻は兄弟ファイルへ置く規律なので、どのみち檻のファイルは増える。

### 候補 B: 読み・判定ともに新しいファイルへ

- 読み: `shell/collision_ex.rs`（`collisionex` の 1 行 → `Result<Collision, 理由>`）。判定: `hit_shape.rs`（形と点 → 内外）。ずらし: 形の型のメソッド（`model.rs`）か `hit_shape.rs` の関数。それぞれに兄弟の `_tests.rs`。
- ✅ 責務がはっきりし、檻を形ごとに閉じて書ける。`hit-id-pass` が後で `region` を足すときの的が見つけやすい。
- ❌ parsers 側の新しいファイルはウェーブ表の「触るファイル」（parsers は `{model,decode}.rs` だけ）の外＝同じウェーブの 5 本とは重ならないが、表の記述と食い違う。ファイルの数が増える。

### 候補 C（混成）: 読みは `decode.rs`、形の判定だけ emo-compose の新しいファイルへ

- 読み: `decode.rs` に兄弟の関数（`decode_collisionex_row` のような）を足し、`decode_collisions` から呼ぶ（`decode_elements`・`decode_animations` と同じ並び）。判定: 新しい `crates/areka-emo-compose/src/hit_shape.rs`（`contains(&CollisionShape, x, y) -> bool`）と兄弟の `hit_shape_tests.rs`。`hit.rs` は述語を `hit_shape::contains` に替えるだけ。ずらし: 形の型に `translated(dx, dy)` を持たせるか `hit_shape.rs` に置く。
- ✅ ウェーブ表の「形の検査の新しいファイル」と一致・`hit.rs` の契約と既存の檻は無傷・読み手は今の流儀のまま。
- ❌ 読みの規則（欄の数・数の読み・`region` の欄）が `decode.rs` に約 120 行増える。ずらしの置き場（parsers の型のメソッドか emo-compose の関数か）は決めが要る。

## 5. 設計の決めが要る項目（要件の討議へ）

1. **型の形**: `Collision` の 4 欄を形の列挙へ置き換えるか、4 欄を残して形の欄を足すか（後者は矩形の値が二重になる）。檻の作り方（`Collision::rect` など）を用意するか。
2. **形の列挙に `#[non_exhaustive]` を付けるか**: structure.md の規律（公開 enum は `#[non_exhaustive]`）に従うと、別 crate の emo-compose の `match` にワイルドカードの腕が要り、形を足したときに判定の漏れがコンパイルで見つからない（`RegionPriority` と同じ理由）。`ShellIssue` は「付けない」前例（`model.rs:304`）。
3. **`region` の「置く位置」の欄**（要件 3.4）: 構文には無い欄（読みでは 0,0）。子から持ち込むとずれる。`hit-id-pass` との受け渡しの契約として型に持たせるか。
4. **由来（`collision` か `collisionex` か）を型に残すか**: 要件は区別を求めない。将来の `collision-sort` の忠実化や台帳・§8 の説明では役に立つが、今は使い手が居ない。
5. **整数の算術の範囲**（要件 2.10）: 楕円の判定 `(2x−l−r)²·h² + (2y−t−b)²·w² ≤ w²·h²` は i64 の座標で i128 も溢れうる。円の `dx²+dy²` も同じ。案＝⑴ 読みで座標を i32 の範囲に限る（越えたら行を使わず記録）⑵ 判定で `checked_*` が溢れたら「当たらない」⑶ 任意の大きさの数。決まった結果を返すならどれでもよいが、境界の振る舞いが変わる。
6. **幅か高さが 0 の楕円**（仮定 A8）: 式だけでは `x = l` の縦の直線の全体（`t..b` の外も）に当たる。囲みの長方形の閉区間を先に見る規則を明記するか。
7. **多角形の規則の細部**: 偶奇の規則（仮定 A3）と「辺の上は当たる」を合わせると、交差する多角形の内側を横切る辺の上も当たる。頂点が 3 未満相当（重なった頂点・一直線）の扱い。頂点の数の上限（予算）を設けるか。
8. **持ち込みの予算の数え方**: `IMPORT_BUDGET`・`HitMemo`・`surface_rows` は領域の数で数える。頂点の多い多角形が入れ子で何度も写されると予算の外で膨らむ。頂点の数で重みを付けるか、今のまま（行の数）でよいか。
9. **`region` と名前の規則**（要件 3.5）: 親に同じ名前の `region` があると、子の同じ名前の形を持ち込まない（今の規則をそのまま当てると、当たらない `region` が子の当たる形を消す）。`hit-id-pass` が働きを足すまでの間の振る舞いとしてそれでよいか。
10. **同じブレスの中の同じ番号**: クローディアには無いが、`collision0` と `collisionex0` を同じブレスに書くと、今の `fold.rs` は同じブレスの中では両方を残す（置き換えは後のブレスだけ）。仮定 A2 はブレスをまたぐ置き換えだけを言うので、同じブレスの中は今のまま（両方残り後の方が手前）でよいかを明記するか。
11. **記録の型と「行の原文」**（要件 4.9）: `ShellIssue` に理由つきの種類を足すか（例「使えない collisionex の行」＋理由の語・「まだ働かない region」）、`UnreadableNumber` を流用するか。行の原文は lexer が捨てているので、欄を `,` で繋ぎ直した近似（前後の空白は失われる）で足りるか、lexer から原文を運ぶか（lexer はウェーブ表の触るファイルの外）。
12. **新しいファイルの置き場**: 候補 A／B／C（4 章）。parsers 側に新しいファイルを置くならウェーブ表の記述との食い違いを roadmap の持ち主へ知らせる。
13. **台帳の状態**（仮定 A7）: `collision*` を `implemented` にするなら、定義箇所に正典 URL の証拠コメントが要る（`ImplementedWithoutEvidence`）。一方で画家の順は正典（先に書いた方が上位）と逆向きなので、台帳の語の定義（`degraded`＝「動くが正典どおりではない」）に照らすと `degraded` のままが筋とも読める。`collisionex*` の担当は `collisionex-regions`。どちらも `report/assets.md`・`report/summary.md` の作り直しが要る。
14. **正典が沈黙する所を SSP で測るか**: 開発者の方針は「ukadoc で読めない点は議題の前に SSP の MCP で測る」。測れる所＝交差する多角形の塗り方（A3）・楕円と円の境界（A8）・形の名前の大小文字（A5）・`collision` と `collisionex` の番号の並び（A2）。SSP の重なりの順（A1）は正典が書いているので測らなくてよい。測りには開発者の許しと排他テストの机と SendInput が要る。測らずに areka 裁量として §8 に書く道もある。
15. **`region` の記録の粒度**（仮定 A6）: 行ごと・読み込みのたびか、シェルごとに 1 件か。
16. **要件の穴**: ⑴ `region` の行に 6 つ以上の欄があるとき（要件 4.5 は `rect`・`ellipse`・`circle` だけを言う） ⑵ 欄の前後の空白（lexer が除くので要件の追記は不要の見込み） ⑶ 空の名前の当たり（今の矩形と同じく `Some("")` が返り、Reference4 は空になる）。

## 6. 規模と危険

- **規模: M**（3〜7 日・8〜11 タスクの見込みのまま）。既存の流儀（`decode_*` の兄弟の関数・`ShellIssue`・逆順の `find`・持ち込みのずらし）に沿って足すだけで、新しい依存も新しい仕組みも無い。檻の書き換え（直書き約 20 か所＋陳腐化 4 本）と台帳・§8・実機が仕事の半分を占める。
- **危険: 低〜中**。低の理由＝下流の契約（名前の文字列）を変えず、本体に触らず、同梱の検体ではクローディアしか振る舞いが変わらない。中の理由＝整数の算術の境界（議題 5・6・7）と予算の数え方（議題 8）は書き方を誤ると黙って別の所に当たり、檻を丁寧に作らないと見つからない。

## 7. 設計の段への申し送り

- **推す方向（決めではない）**: 候補 C。型は `Collision { index, shape, name }`（議題 1）・形の列挙は `#[non_exhaustive]` を付けない側で網羅の match を保つ（議題 2・`ShellIssue` の前例）・整数の判定は i128 ＋溢れたら当たらない、または読みで範囲を限る（議題 5）・幅 0 の楕円は囲みの長方形を先に見る（議題 6）。
- **調べること（Research Needed）**:
  - SSP の測り（議題 14）: 交差する多角形の中・楕円の周の上・大文字の形の名前・`collision0` を後のブレスの `collisionex0` が置き換えるか。測りの前にウェーブの兄弟へ知らせる・`ssp_shiori_log.txt` の差分で Reference4 を読む。
  - ukadoc の `collisionex*` の節（[descript_shell_surfaces#collisionex-star](https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#collisionex-star)）と台帳の id（`collisionex_2a_2cID_2c_30bf_30a4_30d7_2c_5ea7_6a191_2c_5ea7_6a192...:1`）の正典 URL の綴り（証拠コメントの書き方・README 3 章）。
  - `cargo test -p ukadoc-survey` の所見のうち、`owner` に spec 名を書くときの検査（実在する spec か）・同じウェーブの `audit-ukadoc-survey` が検査の側を変えないか（触るのは `crates/ukadoc-survey/` だけで台帳には触らない見込み）。
  - 実機（要件 8.5〜8.7）: クローディアの `surface0` で矩形（`Head` など）・楕円（`Foot`）・多角形（`Fan`）のそれぞれに当たる画面の座標を、表示の拡大率込みで先に決めておく（SendInput の的）。

## 8. 要件の討議での振り分け（2026-11-17）

- **要件へ直した（自明）**: 5 章の 6（楕円は囲みの長方形の中を先に見る＝要件 2.2）・10（同じブレスの中の同じ番号は両方残す＝要件 3.3）・11 の「行の原文」（欄を `,` で繋いだもの＝要件 4.9）・16 ⑴（`region` の余分な欄＝要件 4.5）・陳腐化する檻 4 本の書き直し（要件 8.8）。
- **設計の段へ回す**: 1（型の形）・2（`#[non_exhaustive]`）・3（`region` の置く位置の欄）・4（由来を型に残すか）・5（整数の算術の範囲）・7（多角形の細部）・8（持ち込みの予算の数え方）・11 の記録の型・12（新しいファイルの置き場）。
- **開発者の議題へ**: 9（`region` と名前の規則）・13（台帳の状態）・14（SSP で測るか）・15（`region` の記録の粒度）と、要件の仮定 A1（重なりの順）・A4（読めない行の線引き）。

## 9. SSP の測り（2026-11-17・要件の討議の決めで設計の前に）

- **環境**: SSP（`C:\wintools\ssp`）。開発者の許しを得て、排他テストの机を取って測った。測りに使ったのは `．さくら`（`dot_sakura`・「SHIORI呼出ログ」が有効）で、`えも2DEBUG` の横へ `\![call,ghost,．さくら]` で呼んだ。試しのシェル `shell\collisionex-probe\`（300×300 の不透明な画像・`seriko.use_self_alpha,1`・面 0〜99 に同じ当たり判定）を置き、`\![change,shell,collisionex-probe]` に着替えさせた。点は `SendInput`（絶対座標・仮想デスクトップ）でマウスを動かし、ゴーストの `ghost\master\ssp_shiori_log.txt` に増えた `OnMouseMove` の Reference4 を読んだ。表示の拡大率は 2（画像の 1 点＝画面の 2 点）。試しのシェルの本文は `evidence/ssp-probe-surfaces.txt`。
- **後片付け**: `．さくら` を `master` のシェルへ戻してから閉じ、試しのシェルのフォルダを消した。途中でクローディアでも試した（同じ形のフォルダを置いて消した・シェルは `Master` へ戻して閉じた）。`えも2DEBUG` には何もしていない。

| 点（画像の座標） | 置いた当たり判定 | Reference4 |
| --- | --- | --- |
| (100,100) 五芒星の中心（巻き数 2） | `collisionex1,Star,polygon,100,20,147,165,24,75,176,75,53,165` | `Star` |
| (100,40)・(40,80)・(60,80) 星の腕（巻き数 1） | 同上 | `Star` |
| (100,150)・(45,110) 凸包の中で星の外 | 同上 | 空 |
| (20,20) 星の外 | — | 空 |
| (215,60) | `collisionex2,UpEllipse,Ellipse,200,20,280,100` | `UpEllipse` |
| (215,150) | `collisionex3,UpRect,RECT,200,120,280,180` | `UpRect` |
| (50,240) | 前のブレスの `collision4,10,200,90,280,OldRect` | `OldRect` |
| (150,240) | 後のブレスの `collisionex4,NewRect,rect,110,200,190,280` | `NewRect` |
| (215,210) | 前のブレスの `collision5,200,190,230,230,Old5` | `Old5` |
| (215,260) | 後のブレスの `collision5,200,240,230,280,New5` | `New5` |
| (5,290)・(290,290) | — | 空 |

- SSP のエラーログ: 番号 4・5 のどちらにも「shell\collisionex-probe\surfaces.txt:Line=17:Surface=0/Collision=4 二重定義されています。 (…Line=12)」の形の警告が面ごとに出た。形の名前の大小文字・`polygon` の交差には何も出なかった。
- **読み**: ⑴ `collision*` と `collisionex*` の番号は 1 つの並び（A2）。SSP は重複を警告して両方を残し、どちらも当たる＝areka の既存の決め（後のブレスが前の同じ collision番号の行を除く・§8・`fold.rs` の `numbers.cuts`）とずれていた。開発者の裁定（2026-11-17）でこの仕様の中で SSP に合わせる（`fold.rs` の collision の足し方を element と同じ「両方を足して警告」へ）。⑵ 交差する多角形は巻き数の規則（A3＝偶奇から改めた）。⑶ 形の名前は大小文字を区別しない（A5）。
- **測らなかった所**: 楕円・円の周の上と大きさ 0 の形（A8＝areka の裁量）・`circle`・`region`。

## 10. 設計の段の調べと決め（2026-11-17・`/kiro-spec-design -y`）

### 10.1 要約

- **調べの種類**: 軽い調べ（既存の仕組みの拡張）。新しい依存・外の API は無いので、Web の調べはしなかった。読んだのは steering（`product.md`・`tech.md`・`structure.md` のテストの命名と 1,000 行・`roadmap.md` の C22 の行・`logging.md`）、設計の雛形、本書 1〜9 節、`brief.md`、`evidence/ssp-probe-surfaces.txt`、コード（`shell/{model,decode,parse,lexer,mod}.rs`・emo-compose の `{hit,hit_import,fold,budget,lib}.rs` と檻・emo-present の `shell_target.rs`・`presenter_test_support.rs`）。
- **使った指針**: 設計の決めは「全部を直す最小の変更」（ponytail の指針）に寄せた。抽象・作り方の関数・由来の欄など、今の要件が使わないものは足さない。
- **主な所見**: ⑴ `Collision` の 4 欄を読むのは emo-compose の `hit.rs`・`hit_import.rs` と檻だけで、areka 本体・emo-present の本番は名前の文字列しか読まない（本体に触らずに済む） ⑵ 同じ collision番号の前の行を除くのは `fold.rs` の `Numbers.cuts` と最後の `retain` だけ（消せば両方が残る） ⑶ element番号の二重定義の `warn!` は「後の波括弧が足す行のうち、面にすでにある番号の行」を数える形で、同じブレスの中の重なりは警告していない ⑷ 予算（`VISIT_BUDGET`・`IMPORT_BUDGET`・`HitMemo`・`MAX_COPIED_LINES`）は当たり判定を 1 件 1 で数えており、頂点の多い多角形を写すと量が頂点の数だけ膨らむ ⑸ 字句の段は末尾の空の欄も 1 つの欄として返す。

### 10.2 決め（設計の段へ回された項目・8 節）

| 項目（5 節の番号） | 決め | 理由 |
| --- | --- | --- |
| 1 型の形 | `Collision { index, shape: CollisionShape, name }`。形は `Rect(CollisionBounds)`・`Ellipse(CollisionBounds)`・`Circle { x, y, radius }`・`Polygon(Vec<(i64, i64)>)`・`Region(CollisionRegion)` | 4 欄を残すと矩形の値が二重になる。矩形と楕円は同じ 4 欄なので同じ構造体を使う。檻の作り方の関数は足さない（直書きは約 20 か所で、`coll`・`hit_coll` の中身を直せば呼び手は直らなくてよい） |
| 2 `#[non_exhaustive]` | 付けない（`CollisionShape`・`CollisionExFault` とも） | 5 つの形は ukadoc が閉じて定める集合で拡張の継ぎ目ではない。emo-compose の `match` をワイルドカードなしの網羅に保ち、`hit-id-pass` などが腕を変えたときに判定とずらしの漏れをコンパイルで見つける（`ShellIssue` の前例） |
| 3 `region` の置く位置 | `CollisionRegion { file, red, green, blue, invert, x, y }` の `x`・`y`。読みでは 0,0、持ち込みで element定義の X,Y だけずれる | 要件 3.4 が「`region` は置く位置」をずらすと言う。`hit-id-pass` との受け渡しの契約 |
| 4 由来を型に残すか | 残さない | 使い手が居ない（重なりは区別しない）。要るなら `collision-sort` を忠実にする仕様で足す（設計の OPEN QUESTION 6） |
| 5 整数の算術の範囲 | 浮動小数を使わず、検査つきの `i128`／`u128`。溢れたら当たらない。円は `i64` の全域で厳密、楕円は幅と高さがおよそ 2^32 未満、多角形は座標の差がおよそ 2^63 未満で厳密 | 要件 2.10 は「異常終了せず決まった結果」。読みで座標を `i32` に限る案は、持ち込みのずらしで範囲を越えうるので退けた。256 ビットの掛け算の自作は実のシェルに無い大きさのためのコードになるので退けた（OPEN QUESTION 3） |
| 7 多角形の細部 | 巻き数の規則（SSP の測り）＋辺の上は当たる（外積 0 かつ辺の囲みの中）。頂点の重なり・一直線は面積 0 で辺の上だけ当たる。頂点の数に上限は設けず、重さで予算に数える | 正典に上限が無い。頂点の数の上限より、予算を重さで数える方が今の予算の保証（指摘 79・129・130）をそのまま多角形へ広げられる |
| 8 予算の数え方 | 当たり判定の重さ＝多角形は頂点の数（最小 1）・ほかは 1。`budget::weight`・`weights` を足し、`surface_rows`・`body_lines`・持ち込みの量・持ち越し・`HitMemo` を重さで数える。予算の値は変えない | 件数のままだと、範囲の見出し（最大 8,193 面）や入れ子の持ち込み（2^20 件）で同じ多角形の頂点が写され、メモリと点 1 つの判定の仕事が頂点の数だけ膨らむ。矩形・楕円・円・`region` だけのシェルは重さ＝件数で振る舞いが変わらない（OPEN QUESTION 1） |
| 11 記録の型 | `ShellIssue::CollisionEx { heading, text, fault: CollisionExFault }` と `ShellIssue::RegionInactive { heading, text, count }`。番号が読めない行は既存の `UnreadableNumber`（`text` に欄を `,` で繋いだ行） | 理由の語を型で持つと `message()` が理由ごとに書き分けられ、檻が理由を名指しで固定できる。欄の名前 `text` は既存の `UnreadableNumber.text` に合わせた（既存の `line` は行の番号） |
| 12 ファイルの置き場 | 候補 C。読みは `decode.rs` の兄弟の関数 `decode_collisionex_row`、形の算術は新しい `crates/areka-emo-compose/src/hit_shape.rs` | ウェーブ表の「形の検査の新しいファイル」と一致。`hit.rs` は述語を替えるだけで、入出力の契約と中の檻は無傷 |

- **同じ collision番号の警告**（要件 3.7）: element番号の二重定義と同じ仕組み（`add_lines` が後の波括弧の行を足すとき、面にすでにある番号の行を数え、記述子ごとに 1 行）。同じ波括弧の中だけの重なりは警告しない（OPEN QUESTION 2・要件 6.4 との整合）。
- **`region` の記録**（要件 4.7）: `Reading` に数と最初の行を控え、`parse_all` の輪の後の `reading.finish()` で 1 件だけ積む。記録の列の最後に入る。
- **判定の順**（読み）: 形 → 座標の数 → 多角形の奇数 → 使う欄の整数 → 多すぎる欄。`region` は欄の検査を 1 つにまとめる。1 行に複数の理由が当てはまっても記録は 1 件。

### 10.3 合成の見方（generalization・build vs adopt・simplification）

- **まとめた所**: 要件 2（内外）と 3.4（ずらし）は「形の算術」という 1 つの責務なので、`hit_shape.rs` の 2 つの関数（`contains`・`translated`）にまとめた。`hit.rs` と `hit_import.rs` は形を知らない。
- **作るか借りるか**: 点の内外（閉じた楕円・円・巻き数の多角形）は標準の整数の算術で数十行に収まり、ワークスペースの依存に幾何の crate は無い。依存を足さない約束（C22 ⑸）もあるので自作する。
- **削った所**: 由来の欄・檻の作り方の関数・頂点の共有（`Arc`）・同じブレスの中の警告の新しい関数・`hit.rs` の口の形の変更・字句の段から原文を運ぶ変更は入れない。`presenter/hit.rs` と areka 本体は変更なし。

### 10.4 危険と手当て

- 頂点の重さで数えると持ち込みの持ち越しの切り方（列の前から落とす）が件数から重さに変わる＝`row_budget_tests.rs` に重さの檻を足して固定する。
- 同じ collision番号の前の行を除かなくなるので、番号を書き直す前提のシェルでは前の行も当たり、警告が増える＝SSP と同じ振る舞い（開発者の裁定 2026-11-17）。§8 の【上書き】の行を書き直す。
- 台帳の報告（`report/assets.md`・`summary.md`）は機械で作り直すので、同じウェーブの `audit-ukadoc-survey`（`crates/ukadoc-survey/` だけを触る）が報告の作り方を変えると、着地の順で作り直しが要る。
