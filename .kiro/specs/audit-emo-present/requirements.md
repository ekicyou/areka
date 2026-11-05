# Requirements Document

## Project Description (Input)
開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は、ponytail による見直し 33 本のうちの 1 本で、**面とバルーンの提示**（`crates/areka-emo-present/` 全部）を受け持つ。まだ audit をかけておらず、指摘はギャップ分析で洗い出す。直した後は、同じ範囲へもう一度 audit をかけて新しい指摘が 0 件になり、全体テストが緑で、直したバグ・危ない所の 1 つずつに、それが壊れたら落ちるテストがある状態にする（開発者「とにかく指摘事項は全部直す方針です」・brief.md が正本）。

## Introduction

この spec は、`crates/areka-emo-present/` に `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。要件に書くのは「指摘を全部直す」という形と、その守り方だけである。**指摘の中身はここに書かない**。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

**着手のときの測り直し**（2026-11-05・作業場の先頭 `88eb0a43`＝main と同じ。数えた対象は git が持つファイル。`_tests.rs`・`_test_support.rs`・`tests/` をテストに数えた＝brief と同じ数え方）:

| 項目 | 起票のとき（main `e3c2cbe4`） | いま |
|---|---|---|
| ファイル | 73 | 91（Rust 90・`Cargo.toml` 1） |
| コード | 6,146 行 | 26 ファイル・7,300 行 |
| テスト | 21,599 行 | 64 ファイル・25,092 行（どれも `src/` の兄弟ファイル・`tests/` は無い） |
| 1,000 行を越えるファイル | — | 1 本＝`src/cache_tests.rs` 1,301 行（行数の番人の例外表に載っている。番人の自己検査が表から外して試す 1 件でもある） |
| 1,000 行に近い本番ファイル | — | `src/shell_target.rs` 951 行 |
| `doc/`・`docs/` を名指す行 | — | 0 |
| 範囲の外の呼び出し元（このクレートを使うファイル） | — | `crates/areka`（`src/`・`tests/` 77＋実行例 6）・`crates/areka-emo-text`（26＋実行例 2）・`crates/pilot`（実行例 2）・`crates/log-capture-kit`（1） |

## Boundary Context

- **In scope**: `crates/areka-emo-present/` の全部のファイルへの audit の指摘の直し。そのためのテストの追加と直し。直しのために要る範囲の外の呼び出し元・テスト・fixture の直し（要件 7 の約束に従う）。
- **Out of scope**: 範囲の外の見直し（ほかの 32 本の持ち物）。新しい機能。`vendors/`。clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し（直さずに議題にする）。
- **Adjacent expectations**:
  - 同じウェーブ C17 の兄弟（`doc-tree-tidy` と見直し 6 本）は並んで走る。見直しどうしの範囲は重ならない。いま見つかる呼び出し元の 4 つのクレートは、C17 のどの兄弟の範囲でもない。
  - `clippy-199-lints` と本体の crate 分けの 6 本は、このウェーブに並べない（roadmap「ウェーブ編成」C17 の約束 ⑷）。
  - v0.2.0 の `draw-methods-canon`・`collisionex-regions`・`hit-id-pass`・`compose-f16-precision`・`instant-to-interrupt-ticks`・`tick-gate-adoption` は、この spec の着地を待つ（触るファイルが重なる）。この spec はそれらの機能を先取りしない。
  - 1 ファイル 1,000 行の番人とその例外表は `crates/log-capture-kit/` にあり、`audit-test-kits` の範囲である。

## Requirements

### Requirement 1: 指摘の洗い出し
**Objective:** As a 開発者, I want 受け持つ範囲の全部のファイルを同じ物差しで見た指摘の一覧, so that 直す対象を漏れなく確定できる

#### Acceptance Criteria
1. When ギャップ分析を行うとき, the 見直しの作業 shall `crates/areka-emo-present/` だけを対象に `/ponytail:ponytail-audit` をかけ、指摘の一覧をギャップ分析の報告に残す。
2. The 見直しの作業 shall audit の作法の「最大 20 件」を外し、出た指摘を全件載せ、省いた件を残さない。
3. The 見直しの作業 shall 範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない。
4. The 見直しの作業 shall 指摘の番号を audit の番号のまま使い、各指摘に 3 つの組（必ず直す・直したほうがよい・できれば）のどれかを付ける。
5. When 要件の討議を行うとき, the 見直しの作業 shall 指摘の一覧を要件に落とし、直す対象と、要件 3 の例外として直さずに残す候補を確定する。

### Requirement 2: 指摘を全部直す
**Objective:** As a 開発者, I want audit の指摘が 3 つの組とも全部直っていること, so that 面とバルーンの提示の段からバグ・危ない所・要らないコードが消える

#### Acceptance Criteria
1. The 見直しの作業 shall 要件の討議で確定した指摘を、必ず直す・直したほうがよい・できれば の 3 つの組とも全部直す。
2. The 見直しの作業 shall 要件 3 の例外として開発者が裁定した指摘のほかに、直さない指摘を残さない。
3. When 指摘を直すとき, the 見直しの作業 shall ponytail の「いちばん小さな完全な変更」に従い、消せるものは消し、呼び出し元・テスト・fixture まで直し切る。
4. When コードを消すとき, the 見直しの作業 shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
5. When バグや危ない所を直すとき, the 見直しの作業 shall 根の 1 か所を直し、呼び出し元ごとの繕いで済ませない。

### Requirement 3: 直さずに残してよい 3 つの例外
**Objective:** As a 開発者, I want 直さずに残す指摘が決めた 3 種類だけで、どれも私の裁定を経ていること, so that 「全部直す」の方針が黙って崩れない

#### Acceptance Criteria
1. The 見直しの作業 shall 直さずに残す指摘を、次の 3 種類だけに限る。⑴ 開発者の裁定に反する直し。⑵ わざと置いた先送りの印（実在の理由の doc を付けた `#[allow(dead_code)]` の語彙の口と、限界が今の負荷を越えていない `shortcut:`・`ponytail:` の注釈）。⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。
2. When 指摘を直さずに残す候補にするとき, the 見直しの作業 shall それを要件の討議の議題にして、開発者の裁定を仰ぐ。
3. If 指摘が 3 種類目（SSP との互換や ukadoc の正典に関わる振る舞いの変更）に当たる見込みなら, the 見直しの作業 shall 議題にする前に ukadoc で確かめ、出どころを添える。
4. If 直さずに残す候補に開発者の裁定が無いなら, the 見直しの作業 shall その指摘を直す対象として扱う。
5. When 開発者が直さずに残すと裁定したとき, the 見直しの作業 shall 指摘の番号・種類・裁定を spec の記録に残す。

### Requirement 4: 直しを守るテスト
**Objective:** As a 開発者, I want 直したバグ・危ない所が戻ってきたら落ちるテスト, so that 直しが後の変更で黙って崩れない

#### Acceptance Criteria
1. When バグまたは危ない所の指摘を直すとき, the 見直しの作業 shall その直しが壊れたら落ちるテストを 1 本ずつ置く。
2. The 見直しの作業 shall 足すテストを決定論で書き、同じ入力で毎回同じ結果にする。
3. Where テストが GPU を使うとき, the 見直しの作業 shall そのテストを実 GPU で回す。
4. Where テストが実機の根や一時フォルダを使うとき, the 見直しの作業 shall それらをワークツリーの `target` の下に置く。
5. When 直しを終えたとき, the 見直しの作業 shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）を回して緑を確かめる。
6. If 文字化けが出たなら, the 見直しの作業 shall コンソールの文字コードを変えて逃げず、読む側を直す。

### Requirement 5: もう一度かけて 0 件
**Objective:** As a 開発者, I want 直した後の同じ範囲に audit の新しい指摘が無いこと, so that 直しが新しい問題を生んでいないと分かる

#### Acceptance Criteria
1. When 全部の直しを終えたとき, the 見直しの作業 shall 同じ範囲（`crates/areka-emo-present/`）へ要件 1 と同じ作法で `/ponytail:ponytail-audit` をかけ直す。
2. If かけ直しで新しい指摘が出たなら, the 見直しの作業 shall それも直し、かけ直しで新しい指摘が 0 件になるまで続ける。
3. The 見直しの作業 shall 要件 3 で直さずに残すと裁定した指摘だけを、かけ直しの結果に残してよいものとして扱う。
4. The 見直しの作業 shall かけ直しの結果（新しい指摘 0 件と、残した指摘の番号）を検証の記録に残す。

### Requirement 6: 守る決まり
**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないこと, so that 見直しが別の負債を生まない

#### Acceptance Criteria
1. The 見直しの作業 shall どのファイルも 1,000 行の番人を越えさせず、足すコードやテストが上限を越えそうなら新しいファイルへ置く。
2. The 見直しの作業 shall 番人の例外表に載っている `src/cache_tests.rs` に行を足さない。
3. The 見直しの作業 shall 記録の無い失敗の道を作らず、失敗は記録を残したうえで呼び出し元へ返す。
4. The 見直しの作業 shall 終了の道を正規の実装にし、小細工で済ませない。
5. The 見直しの作業 shall 依存を足さず、`Cargo.lock` を変えない（ウェーブ C17 の約束 ⑶）。
6. If 依存を足さなければ直せない指摘が出たなら, the 見直しの作業 shall 依存を足さず、その指摘を要件の討議の議題にする。

### Requirement 7: 範囲の外への及び方とウェーブの約束
**Objective:** As a 開発者, I want 範囲の外の呼び出し元への直しが漏れず、並んで走るセッションとぶつからないこと, so that 見直しが他の作業を壊さない

#### Acceptance Criteria
1. When 直しで関数や型の形が変わるとき, the 見直しの作業 shall ツリー全体で呼び出し元を探し、範囲の外のものも全部直す。
2. If 直しが及ぶ範囲の外のファイルを、ほかの見直しの spec の走っているセッションが受け持っているなら, the 見直しの作業 shall そのセッションと申し合わせてから触る。
3. The 見直しの作業 shall `doc/`・`docs/` を名指す行を足さず、変えない（ウェーブ C17 の約束 ⑴）。
4. The 見直しの作業 shall `crates/areka-emo-text/` に新しいファイルを足さない（ウェーブ C17 の約束 ⑶）。
5. If 直しが `src/cache_tests.rs` を分けるなど、範囲の外の番人の例外表の変更を要するなら, the 見直しの作業 shall その表の持ち主の範囲として扱い、要件 7.2 と同じく申し合わせてから触る。
6. The 見直しの作業 shall 新しい機能を足さず、範囲の外の見直し・`vendors/`・clippy の指摘の機械的な直しに手を出さない。

### Requirement 8: タスクの大きさ
**Objective:** As a 開発者, I want 1 本の spec が 20 タスクを越えないこと, so that 実装と確かめが 1 本の中で回し切れる

#### Acceptance Criteria
1. The 見直しの作業 shall 1 本の spec のタスクを 20 以下にする。
2. If 確定した指摘を直すタスクが 20 を越えそうなら, the 見直しの作業 shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切り出すことを議題にする。
3. The 見直しの作業 shall タスクを、指摘の組（必ず直す・直したほうがよい・できれば）ごと、またはファイルのまとまりごとに切る。
4. The 見直しの作業 shall 最後のタスクを、要件 5 のかけ直しと要件 4.5 の全体テストを行う検証のタスクにする。
