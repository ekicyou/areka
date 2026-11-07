# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **文字の層の状態・アクター・選択肢・窓**（`crates/areka-emo-text/` のうち `audit-emo-text-layout-draw` の持ち物でないもの全部）を受け持つ。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。

あわせて、`audit-emo-compose` の完了の棚卸（2026-11-06）から持ち込まれた件（箱の位置に大きさを足すと整数があふれる）を、この spec の audit の指摘と一緒に直す。

## Introduction

この文書は、文字の層の状態・アクター・選択肢・窓のファイルに `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直すための要件を定める。要件の段では「全部直す」という形だけを書き、指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書に落として直す対象を確定する。持ち込みの件だけは、中身が起票のときに分かっているので要件 7 に書く。

受け持つ範囲の大きさ（着手のときの測り直し・`main` `73c5b25d`）:

| 測った時点 | ファイル | うち Rust のファイル | コード | テスト |
|---|---|---|---|---|
| 起票のとき（`e3c2cbe4`・brief の値） | 102 | — | 9,170 行 | 29,332 行 |
| 着手のとき（`73c5b25d`） | 121 | 111 | 10,832 行 | 34,553 行 |

テストに数えたのは、名前が `_tests.rs` で終わるファイル・`tests/` の下のファイル・名前が `_test_support.rs` で終わるファイル。ファイルの数には `Cargo.toml`・`examples/` の 12 ファイル（検体 4 つを含む）・`tests/fixtures/` の下の 5 ファイル（設定の検体）を含む。棚卸㉟の測り（`18e4cb30`）の後、このクレートは変わっていない。1 ファイル 1,000 行の上限の近くにあるのは `src/surface.rs`（995 行）・`src/actor_choice_contract_tests.rs`（991 行）・`src/choice_tests.rs`（980 行）の 3 つ。

この spec はウェーブ C18 で、ほかの 7 本（`wintf-visual-tree`・`placement-crate-split`・`audit-wintf-layout-widget`・`audit-dola`・`audit-emo-text-layout-draw`・`audit-kanade`・`audit-sylphya-actor`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C18 の行）を守る。

## Boundary Context

- **In scope**:
  - `crates/areka-emo-text/` のうち、`audit-emo-text-layout-draw` の持ち物（`src/` の `layout*`・`segment*`・`region*`・`wrap*`・`lookahead*`・`cursor_tag*`・`writing*`・`draw*`・`canvas*`・`viewbox*`・`look*`・`color*`・`underline*`・`place*`）でない全ファイル。すなわち `src/` の `state*`・`actor*`・`choice*`・`sink*`・`surface*`・`balloon_overrides*`・`inline_marker*`・`marker_surface*`・`scroll_marker*`・`lib.rs`、`tests/` の全部、`examples/` の全部、`Cargo.toml`。
  - その指摘の直しと、そのためのテストの追加と直し。
  - 持ち込みの件（要件 7）の直しと、そのテスト。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 5 の約束に従う）。
- **Out of scope**: 範囲の外の見直し（ほかの 32 本の見直しの持ち物。同じクレートの `audit-emo-text-layout-draw` の持ち物を含む）。新しい機能。`vendors/`。clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。
- **Adjacent expectations**:
  - このクレートを使う範囲の外のクレートは、着手のときに `crates/areka` の 1 つ（呼び手のファイルは約 60）。そのファイルの多くは、同じウェーブの `placement-crate-split` が書き換える。
  - 同じウェーブの相手には、次の形を変えないことを期待する（ウェーブ C18 の約束）。`wintf-visual-tree` は `ecs::graphics::Visual`・`VisualGraphics` の名前を変えない。`audit-wintf-layout-widget` は `Arrangement`・`HitTest`・`AlphaMaskResource`・`LayoutScale`・`AlphaMask` の形を変えない。`audit-dola` は `dola::cue::CueSink` の形を変えない。`audit-sylphya-actor` は `areka-actor` の公開の形を足すだけにする。
  - `audit-emo-text-layout-draw` とは同じクレートの別のファイルを受け持つ。互いの関数の形を、相談なしに変えない。
  - このクレートに新しいソースファイルを足してよいのは、このウェーブではこの spec だけ（`lib.rs` の純粋な層の一覧の席）。
  - 範囲の中に、1 ファイル 1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`・範囲の外）に載っているファイルは、着手のときに無い。

## Requirements

### Requirement 1: 指摘の洗い出し

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行う, the 本見直し shall Boundary Context の「In scope」に挙げた受け持つファイルだけを対象にして `/ponytail:ponytail-audit` をかける
2. The 本見直し shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The 本見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない
4. The 本見直し shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. The 本見直し shall 棚卸㉟の後に足された所（装置の作り直しを見て描き手を捨てる所と、その印の説明「1 つの World に文字の層は 1 つ」の前提）も audit の対象にし、その前提が今のコードで成り立つかを報告に書く
6. When 要件の討議で直す対象が確定した, the 本見直し shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか）をこの文書に書き加える

### Requirement 2: 指摘を全部直す

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that 文字の層の状態・アクター・選択肢・窓のコードからバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本見直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本見直し shall 要件 3 の 3 種類のどれかに当たり、開発者が残すと裁定した指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出た, the 本見直し shall その箇所を使うテスト・検体・呼び出し元まで直し切り、直しの途中の状態を残さない

### Requirement 3: 直さずに残す指摘は開発者の裁定で決める

**Objective:** As a 開発者, I want 直さずに残す指摘を自分で裁定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. If 指摘の直しが次の 3 種類のどれかに当たる, the 本見直し shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. When 3 種類目に当たるかを判断する, the 本見直し shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする
3. If 指摘の直しが 3 種類のどれにも当たらない, the 本見直し shall その指摘を残すことを議題にせず直す
4. When 開発者が指摘を残すと裁定した, the 本見直し shall 裁定とその理由をこの文書に残し、要件 4 の「新しい指摘 0 件」の数から外す

### Requirement 4: 直した後の確かめ

**Objective:** As a 開発者, I want 直しが新しい問題を生んでいないことを確かめられること, so that 見直しが終わったと安心して言える

#### Acceptance Criteria

1. When 確定した指摘を全部直し終えた, the 本見直し shall 同じ範囲に、要件 1 と同じ作法（件数の上限なし・全部読む）で `/ponytail:ponytail-audit` をもう一度かける
2. If もう一度かけた audit が新しい指摘（直しが生んだ指摘を含む）を出した, the 本見直し shall その指摘も直し、新しい指摘が 0 件になるまで audit をかけ直す
3. If 5 回目のかけ直しでも新しい指摘が 0 件にならず、その回の「必ず直す」と本番の振る舞いに関わる指摘がどちらも 0 件である, the 本見直し shall その回の指摘を全部直したうえで、かけ直しを区切ることを開発者に一言知らせてから区切り、検証の記録に「5 回目で区切った・残りは直し済み」と書く（2026-11-06 `audit-seriko` での開発者の決め）
4. When 見直しを終える, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる
5. The 本見直し shall 直したバグと危ない所の 1 件ずつに、その直しが壊れたら落ちるテストを 1 本以上置く
6. The 本見直し shall 足したテストと直したテストを、毎回同じ結果になる（決定論の）テストにする

### Requirement 5: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが安全に、並んで走るほかの spec とぶつからずに入ること, so that ほかのクレートが壊れず、並行作業が止まらない

#### Acceptance Criteria

1. The 本見直し shall `crates/areka/` のファイルを書き換えず、`crates/areka/` の呼び出し元が使っている公開の形（型・関数・欄の名前と形）を変えない（ウェーブ C18 の約束）
2. If 指摘の直しが `crates/areka/` の呼び出し元の書き換えなしには成り立たない, the 本見直し shall その直しをこのウェーブでは入れず、残す（理由を書く）か後の spec へ回すかを要件の討議の議題にする
3. When 直しで範囲の中の関数や型の形を変えた, the 本見直し shall ツリー全体で呼び出し元を探して、全部直す
4. If 直しが `audit-emo-text-layout-draw` の受け持つファイルの関数の形、またはそのファイルが呼ぶこの範囲の関数の形を変える, the 本見直し shall `audit-emo-text-layout-draw` のセッションと申し合わせてから変える
5. If 直しが及ぶ範囲の外のファイルを受け持つ見直しの spec が走っている, the 本見直し shall そのセッションと申し合わせてから、そのファイルに触る
6. When コード・テスト・検体を消す, the 本見直し shall 消す前に、ツリー全体（テスト・検体・設定・文字列での参照を含む）で使われていないことを確かめる
7. The 本見直し shall 範囲の外のファイルの、直しに要らない箇所を直さない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）

### Requirement 6: 守る決まり

**Objective:** As a 開発者, I want 見直しの直しがプロジェクトの決まりとウェーブの約束を守ること, so that 直しが別の決まりを破らず、同じウェーブのほかの spec の邪魔をしない

#### Acceptance Criteria

1. The 本見直し shall 1 ファイル 1,000 行の番人を赤にせず、直しで行が上限を越えるときは新しいファイルへ分ける
2. The 本見直し shall 1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`）と一時の入口の番人（`crates/log-capture-kit/tests/temp_path_guard_test.rs`）に触らない（同じウェーブの `placement-crate-split` が触る）
3. When 新しいソースファイルを足した, the 本見直し shall そのファイルを `lib.rs` の純粋な層の一覧か、その走査の外の一覧のどちらかに載せ、一覧の番人を緑に保つ
4. The 本見直し shall 記録の無い失敗の道を作らず、失敗は記録（`error!`）を残したうえで呼び出し元へ失敗（`Err`）として返す
5. The 本見直し shall 終了の道を正規の実装にし、終了を早めるための小細工を入れない
6. The 本見直し shall 実機の根と一時フォルダを、ワークツリーの `target` の下に置く
7. The 本見直し shall GPU のテストを実 GPU で回す
8. If 文字化けが起きた, the 本見直し shall コンソールの文字コードを変えて逃げず、読む側を直す
9. The 本見直し shall 依存を足さず、`Cargo.lock` を変えない（ウェーブ C18 の約束）
10. If 直しに新しい依存が要ると分かった, the 本見直し shall 依存を足さずに開発者へ議題として出す（足すなら `tech.md` への登記と開発者の承認が要る）
11. The 本見直し shall `vendors/` の下に手を入れない

### Requirement 7: 持ち込みの件＝箱の位置の足し算があふれる

**Objective:** As a 開発者, I want 設定ファイルに極端な数を書かれても文字の層が落ちず、箱の位置が狂わないこと, so that 壊れた設定のゴーストやバルーンでも areka が止まらない

#### Acceptance Criteria

1. If 箱の位置に大きさを足した値が、位置を表す整数の上限を越える（例: `elementN,balloon,名前,9223372036854775807,0`）, the 本見直し shall どのビルドでも途中で落ちず、値を折り返さずに上限で止める
2. The 本見直し shall 箱の当たりの矩形を作る所と、箱がバルーンの面からはみ出すかを判断する所の、両方の足し算に要件 7.1 を当てる
3. The 本見直し shall 要件 7.1 の直しが壊れたら落ちるテストを 1 本以上置く
4. If audit が同じ所を指摘した, the 本見直し shall その指摘と持ち込みの件を 1 つの直しとして扱い、二重に直さない

### Requirement 8: spec の大きさ

**Objective:** As a 開発者, I want 1 本の spec が大きくなりすぎないこと, so that 1 本ずつ確かめて着地できる

#### Acceptance Criteria

1. If 確定した指摘の直しが 20 タスクを越えそうになった, the 本見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切ることを議題にする

## 要件の討議で確定した事項（2026-11-06・全議題を推奨案で確定）

開発者「承認」（2026-11-04 の決め「audit 系は推奨案でよい」にも沿う）。指摘の番号と組は `research.md` 4 節の audit の番号のまま（要件 1.4・1.6）。

### 確定した指摘の一覧

| 番号 | 組 | 扱い |
|---|---|---|
| 1 | 必ず直す | 直す（持ち込みの件＝要件 7 と 1 つの直し）。`actor_box.rs` の 236・237・427 の 3 か所を上限で止める足し算にし、テストを 1〜2 本置く。197 の `site.x as f32` は落ちず、427 の警告が知らせるので直さない（議題 3） |
| 2 | 必ず直す | 直す（矢印の矩形を当たり・マスクと同じく `i64` で求める） |
| 3 | 必ず直す | 直す（下の「面の大きさの上限」・議題 5） |
| 4 | 必ず直す | 直す（`choice.rs` の式を NaN で落ちない形に）。字の大きさに上限が無い根（`look.rs`）は `audit-emo-text-layout-draw` へ知らせる |
| 5 | 直したほうがよい | 直す |
| 6 | 直したほうがよい | 直す（議題 2）。装置の作り直しを見る印を World の資源から文字の層の私的な欄へ移し、「1 つの World に文字の層は 1 つ」の前提を要らなくする。公開の形は変えない。同じ World に 2 つの文字の層を置いて装置を作り直すと、両方とも描き手を作り直すことを確かめるテストを置く |
| 7〜22 | 直したほうがよい | 直す |
| 23〜30 | できれば | 直す |

残すと裁定した指摘は無い（要件 3.4 で数から外すものは 0 件）。

### 決めごととして指摘に数えなかったもの（議題 6・種類 2）

`research.md` 4 節の終わりの一覧（`#[allow(dead_code)]` と理由の説明を持つ口・`shortcut:` の注釈・開いている spec を名指す継ぎ目など）は残す。どれも書かれた限界が今の負荷を越えていない。継ぎ目のうち `#[allow(dead_code)]` の印を持たない 4 つ（`state_decoration.rs` の `unowned_vocab`・`reset_decoration`、`choice.rs` の `resolve` の引数 `default_font_color`、`actor.rs`・`actor_box.rs` の `box_font_dirs`）は、「名指す spec が閉じたら消す」と説明に書き足すだけにする。

### 面の大きさの上限（指摘 3・議題 5）

- 文字の面・箱の面・印の面の矩形を、画像の矩形（箱は箱の大きさとシェルの画像の重なり）で切ってから物理寸へ直す。窓は画像の大きさなので、見える画素は変わらない。
- 面を作れなかった場所は、入力が変わるまで作り直さず、`error!` は 1 回にする。
- 箱の大きさと矢印の広がりは `i64` で読み、面の大きさの掛け算は `usize` で行う。
- 種類 3（SSP 互換・ukadoc の正典）には当たらない。ukadoc は validrect を「テキスト描画範囲の右位置。」（https://ukatech.shillest.net/ukadoc-spec/descript/balloon/#validrect.right ）などと定めるだけで、画像の外へ出たときの扱いを定めない（要件 3.2 の確かめ・2026-11-06）。

### 公開の形を変えないと直せない指摘が後から出たとき（議題 4・要件 5.2）

着手のときの 30 件には無い。直しやかけ直しで出たときは、古い口を残して新しい口を足すだけの形で直し、呼び手の書き換えは `crates/areka` の見直しの spec へ申し送る。足すだけで直らないものは、残すか後の spec へ回すかを 1 件ずつ議題にする。

### spec の大きさ（議題 1・要件 8）

切らずに 1 本で進める（見込み 26〜34 タスク）。兄弟の見直しも 20 を越えて 1 本のまま進めた。切ると同じファイルに 2 本が順に触り、`lib.rs` の新しいファイルの席の取り合いも生まれる。
