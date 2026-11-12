# Requirements Document

## Project Description (Input)
開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは、各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **wintf のレイアウトと部品** を受け持つ。受け持つ範囲に audit をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直し、直した後にもう一度 audit をかけて新しい指摘が 0 件で、全体テストが緑になっている状態にする。

## Introduction

この spec は、wintf の `crates/wintf/src/ecs/layout*` と `crates/wintf/src/ecs/widget*` をまとめて見直し、指摘を全部直す。

要件の段では「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身は、ギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直し方は設計で決める。

受け持つ範囲は、2026-11-06 の棚卸㉟（main `18e4cb30`）で測り直した。着手のとき（main `73c5b25d`）も、この範囲のファイルはそのときから変わっていない。

| 測った時点 | ファイル | 本番 | テスト |
|---|---|---|---|
| 起票（main `e3c2cbe4`） | 50 | 9,130 行 | 2,426 行 |
| 棚卸㉟（main `18e4cb30`）・着手（main `73c5b25d`） | 50 | 7,217 行 | 4,251 行 |

（起票のときとは数え方が違う。棚卸㉟からは空行を含め、`_tests.rs`・`tests.rs`・`tests/`・`tests_ex/`・`_test_support.rs` をテストに数えた。合わせて 11,468 行。1,000 行に近いファイルは無く、大きいのは `layout/hit_test/mod.rs` の 632 行・`layout/systems/monitor_systems.rs` の 576 行。）

## Boundary Context

- **In scope**:
  - `crates/wintf/src/ecs/` のうち、名前が `layout` か `widget` で始まるもの全部（着手のときの中身は `layout/` と `widget/` の 2 つのフォルダ）。
  - 受け持つファイルの指摘の直しと、そのためのテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3 と要件 5 の約束に従う）。
- **Out of scope**:
  - 範囲の外の見直し（ほかの 32 本の持ち物）。wintf の `ecs/visual/`・`ecs/graphics/` もこの spec は触らない。
  - 新しい機能。
  - `vendors/`。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
  - `audit-wintf-runtime` がすでに直した件（下の要件 2.5）の数え直し。
- **Adjacent expectations**:
  - `audit-wintf-input`・`audit-wintf-window`（同じ `ecs/` の残り）・完了した `audit-wintf-graphics-world`・`audit-wintf-runtime` とは、範囲が重ならない。
  - 同じウェーブ C18 で並んで走る 7 本とは、触るソースファイルを共有しない約束で並ぶ（roadmap のウェーブの表の約束 ⑴ ⑵ ⑹）。
    - `wintf-visual-tree` が wintf の `ecs/mod.rs`・`ecs/world/mod.rs` を使う。この spec はこの 2 つに触らない。
    - `placement-crate-split` が `crates/areka/` と、その外の `log-capture-kit` のテスト 2 つ・wintf の `ecs/window_proc/` の 2 つ・`tools/perf/fixtures/`・`tools/perf/fixtures-loop/` を書き換える。この spec はこれらに触らない。
    - `Cargo.lock` の席は `placement-crate-split` が使う。この spec は依存を足さない。
    - `audit-emo-text-state-actor`・`audit-emo-text-layout-draw` が `crates/areka-emo-text/` を受け持つ。
  - この spec の着地を待つ後の spec がある（`wintf-d2d-composite-rules`・`hit-id-pass`）。`hit-id-pass` は、今は areka から使われていない色分け画像の当たり判定（`layout/hit_region/` の読み込みの働き）を使い回す見込みで、`wintf-d2d-composite-rules` は `widget/bitmap_source/alpha_mask.rs` の α マスクを置き換えるかを議題に持つ。

## Requirements

### Requirement 1: 受け持つ範囲を確定して全部を見る
**Objective:** As a 開発者, I want 受け持つ範囲のファイルがもれなく見直しの対象になっていること, so that 見直しの抜けやほかの spec との二重の見直しが起きない

#### Acceptance Criteria
1. When ギャップ分析を始めるとき, the 見直しの作業 shall その時点の main で受け持つ範囲のファイルを列挙し、ファイル数と本番・テストの行数を記録に残す。
2. If 着手のときより後に `crates/wintf/src/ecs/` の下へ名前が `layout` か `widget` で始まるファイルやフォルダが足されたなら, the 見直しの作業 shall それを受け持つ範囲に含める。
3. The 見直しの作業 shall 受け持つ範囲のファイルを全部読み、読まなかったファイルを残さない。
4. The 見直しの作業 shall 受け持つ範囲の外のファイルを見直しの対象にしない。

### Requirement 2: 指摘を全件洗い出して記録する
**Objective:** As a 開発者, I want audit の指摘が数の上限なしに全部記録されていること, so that 直す対象を要件の討議で漏れなく確定できる

#### Acceptance Criteria
1. When ギャップ分析を行うとき, the 見直しの作業 shall 受け持つ範囲だけを対象に `/ponytail:ponytail-audit` をかける。
2. The 見直しの作業 shall audit の「最大 20 件」の上限を外し、出た指摘を全件記録して、省いた指摘を残さない。
3. The 見直しの作業 shall audit の「読まなかった所（Not checked）」を残さない。
4. The 見直しの作業 shall 指摘の一覧をギャップ分析の報告に残し、各指摘に audit の番号と組（必ず直す・直したほうがよい・できれば）をそのまま付ける。
5. The 見直しの作業 shall `audit-wintf-runtime` がこの範囲で直し済みの件（`Percent` を 100 で割る直し・`FlexContainer` と `FlexItem` の削除・`Opacity` の削除・親の無い根の bounds の伸ばし方・taffy のノードを消すときの失敗の捨て方）を、指摘として数え直さない。
6. When ギャップ分析を行うとき, the 見直しの作業 shall `Arrangement` を足したときのフックが、生成のときに一緒に入れた `GlobalArrangement` を既定値で上書きする件（`audit-wintf-runtime` からの申し送り）を、指摘にするかどうかを決め、決めた結果と理由を報告に残す。
7. When ギャップ分析で `widget/bitmap_source/systems.rs`・`widget/shapes/rectangle.rs`・`layout/systems/window_pos_systems.rs` を見るとき, the 見直しの作業 shall `audit-wintf-graphics-world` が入れた直しの後の形を対象にする。
8. When 要件の討議を行うとき, the 見直しの作業 shall 指摘の一覧を直す対象として要件に落とし、直す対象を確定する。

### Requirement 3: 指摘を 3 つの組とも全部直す
**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも全部直っていること, so that 受け持つ範囲にバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria
1. The 見直しの作業 shall 要件の討議で直す対象と確定した指摘を、必ず直す・直したほうがよい・できれば の 3 つの組とも全部直す。
2. When バグか危ない所の指摘を直したとき, the 見直しの作業 shall その直しが元に戻ったら落ちるテストを、指摘 1 件につき 1 本以上置く。
3. When コードやファイルを消す前に, the 見直しの作業 shall ツリー全体（テスト・fixture・例・設定・文字列での参照を含む）で使われていないことを確かめる。
4. When 直しで関数や型の形が変わるとき, the 見直しの作業 shall ツリー全体で呼び出し元を探し、要件 5 で触らないと決めた所を除いて、範囲の外のものも含めて全部直す。
5. If 直しが及ぶ範囲の外のファイルが、走っているほかの spec の持ち物なら, the 見直しの作業 shall そのセッションと申し合わせてから触る。
6. The 見直しの作業 shall 指摘の直しのほかに新しい機能を足さない。

### Requirement 4: 直さずに残す指摘を裁定に回す
**Objective:** As a 開発者, I want 直さずに残す指摘が決められた種類だけで、どれも自分の裁定を経ていること, so that 「全部直す」の方針が黙って崩れない

#### Acceptance Criteria
1. The 見直しの作業 shall 直さずに残してよい指摘を、次の 3 種類だけに限る。⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印（実在の理由の doc を付けた `#[allow(dead_code)]` の語彙の口と、限界が今の負荷を越えていない `shortcut:`・`ponytail:` の注釈） ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。
2. When 指摘が直さずに残す 3 種類のどれかに当たるとき, the 見直しの作業 shall その指摘を要件の討議の議題にし、開発者の裁定を得てから扱いを決める。
3. If 指摘が SSP との互換や ukadoc の正典に関わる振る舞いの変更なら, the 見直しの作業 shall 議題にする前に ukadoc で確かめ、出典を添える。
4. If audit が、後の spec が使い回す見込みの働き（`layout/hit_region/` の色分け画像の当たり判定の読み込み・`widget/bitmap_source/alpha_mask.rs` の α マスク）を「使われていない」「消せる」と指したなら, the 見直しの作業 shall 消さずに、`hit-id-pass`・`wintf-d2d-composite-rules` へ申し送る案として要件の討議で開発者に確かめる。
5. If 直しが要件 5 の触らない約束に当たり、このウェーブでは直せないなら, the 見直しの作業 shall その指摘を要件の討議の議題にし、相手の着地の後へ回すか、相手のセッションと申し合わせて行を分けるか、理由を書いて残すかの裁定を得る。
6. The 見直しの作業 shall 直さずに残すと裁定された指摘と後へ回すと裁定された指摘を、理由と裁定とともに記録に残す。

### Requirement 5: 直しのあいだ守る決まりと並走の約束
**Objective:** As a 開発者, I want 直しがプロジェクトの決まりと同じウェーブの約束を破らないこと, so that 見直しが新しい問題やほかのセッションとのぶつかりを生まない

#### Acceptance Criteria
1. The 見直しの作業 shall どのファイルも 1,000 行を越えさせず、足す中身が越えさせるなら新しいファイルへ置く。
2. The 見直しの作業 shall 直しのテストを、毎回同じ結果になるテストで書く。
3. If 直しが失敗の道を持つなら, the 見直しの作業 shall その失敗をエラーのログに残し、呼び出し元へ失敗として返して、記録の無い失敗の道を作らない。
4. When 直しが終了の道に触れるとき, the 見直しの作業 shall 終了の道を正規の手順のまま保ち、その場しのぎの近道で置き換えない。
5. The 見直しの作業 shall 実機の根と一時フォルダを、ワークツリーの `target` の下だけに置く。
6. The 見直しの作業 shall GPU を使うテストを、実際の GPU で回す。
7. If 文字化けが出たなら, the 見直しの作業 shall コンソールの文字コードを変えて逃げず、読む側を直す。
8. If 直しに新しい依存が要るなら, the 見直しの作業 shall 依存を足さずに止め、開発者の承認を仰ぐ（同じウェーブでは `Cargo.lock` の席は `placement-crate-split` のもの。承認されたら `tech.md` へ登記する）。
9. The 見直しの作業 shall 同じウェーブのあいだ、wintf の `ecs/mod.rs`・`ecs/world/mod.rs` に触らない。
10. The 見直しの作業 shall wintf の `ecs/visual/`・`ecs/graphics/` に触らない。
11. The 見直しの作業 shall 同じウェーブのあいだ、`crates/areka/` と、`placement-crate-split` が書き換える範囲の外のファイル（`log-capture-kit` のテスト 2 つ・wintf の `ecs/window_proc/` の 2 つ・`tools/perf/fixtures/`・`tools/perf/fixtures-loop/`）に触らない。
12. If 直しが、`crates/areka/` の呼び出し元が使う公開の形を変えるものなら, the 見直しの作業 shall 同じウェーブのあいだはその形を変えずに保って足すだけにし、足すだけで直せないなら要件 4.5 の議題に回す。

### Requirement 6: 直し終わりを確かめる
**Objective:** As a 開発者, I want 直し終わりが新しい audit と全体テストで確かめられていること, so that 「全部直った」をあとから疑わずに済む

#### Acceptance Criteria
1. When 直す対象を全部直し終えたとき, the 見直しの作業 shall 同じ受け持つ範囲へもう一度 `/ponytail:ponytail-audit` をかける。
2. If かけ直しの audit で新しい指摘（直しが生んだ指摘を含む）が出たなら, the 見直しの作業 shall それも直すか、要件 4 の種類に当たるなら裁定に回し、もう一度 audit をかける。
3. The 見直しの作業 shall 最後の audit で新しい指摘が 0 件であることを、0 件と明記して記録に残す。
4. If かけ直しが 5 回目になっても新しい指摘が 0 件にならないなら, the 見直しの作業 shall 打ち切る前に開発者へ知らせ、5 回目で「必ず直す」の指摘と本番の振る舞いに関わる指摘がどちらも 0 件のときに限り、5 回目の指摘を直したところで打ち切る案を示す（2026-11-06 `audit-seriko` で開発者が承認した形）。
5. Where 要件 6.4 の打ち切りが開発者に承認されたなら, the 見直しの作業 shall 検証の記録に「5 回目で打ち切った・残りは直し済み」と書く。
6. The 見直しの作業 shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる。
7. If 直す対象の指摘が多く、タスクが 20 を越えそうなら, the 見直しの作業 shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切る案を開発者に示す。

## 指摘の扱い（2026-11-06 要件の討議で確定）

要件 2.8 に従い、ギャップ分析（`research.md`）の指摘 37 件を、直す対象として確定した。開発者「承認」（全議題を推奨案で確定）。番号は audit の番号のまま。

| 番号 | 組 | 指摘（一行） | 扱い |
|---|---|---|---|
| 1 | 必ず直す | 木に入った後で `BoxStyle` を付けた部品が taffy の木につながらない・外しても古い寸法が残る | 直す |
| 2 | 必ず直す | 子を消しても残った兄弟の配置を計算し直さない | 直す。直しは係の中だけで行い、`ecs/world/mod.rs` の係の並びは変えない（要件 5.9） |
| 3 | 必ず直す | `Typewriter` のトークを入れ替えても前のトークの文と時間表が使われる・外しても最後の絵が残る | 直す |
| 4 | 必ず直す | 最後の文字の後ろの待ちと知らせが捨てられる | 直す |
| 5 | 必ず直す | 知らせの宛先が消えていると UI スレッドが panic で落ちる | 直す |
| 6 | 必ず直す | 文字を 3 通りに数え、サロゲートの対や結合文字で隠す範囲と待ちの位置がずれる | 直す |
| 7 | 必ず直す | `BitmapSource` を外す・付け直すと古い画像と古い当たりのマスクが残る | 直す（議題 3）。外すときは `BitmapSourceResource` だけを外し、`HitTest` は残す（利用者が入れたものかもしれない・マスクが無ければ当たりは矩形に戻る） |
| 8 | 必ず直す | 空のトークの背景を描き直さない | 直す |
| 9〜13 | 直したほうがよい | 記録の無い失敗・恒久の失敗の毎回のやり直し・UI スレッドでの 2 度の解読・`Typewriter` の毎回の描き直し・`Arrangement` のフックが束で渡した `GlobalArrangement` を上書き | 直す |
| 14 | 直したほうがよい | 当たり判定の規則が 2 か所に写してあり、名前付きの領域で答えが食い違う | 直す（議題 2）。1 つにまとめ、`NamedRegions` は α のマスクを見ず、領域の表だけで形を決める |
| 15 | 直したほうがよい | 色分け画像の領域が 256 色を越えると番号が回る | 直す（議題 4） |
| 16 | 直したほうがよい | 色分け画像の透明な画素が「黒」の領域として当たる・半透明の画素の色がずれる | **直さずに `hit-id-pass` へ申し送る**（議題 4・要件 4.1 の ⑶ として裁定済み）。SSP の `collisionex` の `region` での透明な画素の扱いは ukadoc に書いてないため、SSP の MCP で測ることも含めて、この読み込みを使い回すあちらで決める |
| 17〜18 | 直したほうがよい | 色分け画像のパスの文字化け・読み込みのテストが無い | 直す（議題 4） |
| 19〜22 | 直したほうがよい | 向きの設定の写し・モニタの `BoxStyle` の写し・描く骨組みの 5 重の写し・要らない `unsafe impl Send/Sync` | 直す |
| 23 | 直したほうがよい | 使われていないコード | 直す（議題 5・6）。公開の型と関数も消してよい（版はまだ 0.x・使い手はこのリポジトリの中だけ・areka の参照 0）。`#[deprecated]` の `colors` も消す。`ColorMapDef`・`ColorMapping` は消すが、wintf の `serde` 機能は残す（`Cargo.lock` に触らない・`Shape`・`ShapeRegion` がまだ使う） |
| 24 | 直したほうがよい | 書くだけで誰も読まない `TextLayoutResource`・`TextLayoutMetrics` | 直す（議題 5）。型ごと消す |
| 25 | 直したほうがよい | `PhysicalPoint` の名前が別々の 2 つの型を指す | 直す。`ecs/mod.rs` には触らない（要件 5.9） |
| 26〜27 | できれば | taffy のテストのためだけの逆引きの表・`BitmapSource` のフックの二重の挿入 | 直す |
| 28 | できれば | `BitmapSource.path` を書き換えても何も起きない | 直す（議題 8）。欄を読み取り専用にする（`path()` を足し欄を非公開に）。書き換えで読み直す形は新しい機能に当たるので取らない（要件 3.6） |
| 29〜32 | できれば | 古い説明・毎回の小さな無駄・何もしないコード・前景色の式の写し | 直す |
| 33 | できれば | 値の型に `Component` を付けている | 直す（議題 5）。derive を外してよい |
| 34 | できれば | ログを 1 行出すだけの外しのフック | 直す |
| 35 | できれば | 型の作りや定数を確かめるだけのテスト | 消す（議題 7）。網羅は判断の分かれ目の網羅で、derive の動きや定数は対象にしない（`audit-wintf-graphics-world` の議題 8 と同じ答え） |
| 36〜37 | できれば | 領域の表が無いとマウスが動くたびに警告・テストの COM の初期化の失敗を捨てる | 直す |

- **進め方**（議題 1）: この spec 1 本で直す。2 本目には切らない（要件 6.7 の案は示し、退けた）。見立ては 19〜24 タスク。同じファイルに閉じる直しを 1 タスクにまとめて数を抑える（前例の `audit-wintf-graphics-world` と同じ）。
- **範囲の外のテストと例の書き換え**: `crates/wintf/tests/`・`crates/wintf/examples/` のうち、この spec の直しが及ぶもの（13・14・24・25・26・33 番の呼び手）は、本番の直しと一緒にこの spec で書き換えてよい。持ち主の `audit-wintf-runtime` は完了しているので、要件 3.5 の申し合わせは要らない。
- **申し送り**: 16 番は `hit-id-pass` の brief の末尾へ書き足した。`monitor_systems_transition_tests.rs` が `ecs/world/mod.rs` の字面を見張っている件は、同じウェーブの `wintf-visual-tree` 向けに、ウェーブの共有の覚え書きへ書いた。
