# Requirements Document

## Project Description (Input)
開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。本 spec は全 33 本の見直しのうちの 1 本で、**設定ファイルの読み取りとさくらスクリプト**（`crates/areka-parsers/`・`crates/areka-sakura/` 全部）を受け持つ。受け持つ範囲には、まだ audit をかけていない。

変えたいこと: 受け持つ範囲に audit をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件で、全体テストが緑になっていること。正本は `brief.md` と、roadmap の「テーマ別の決めごと」の「ponytail による見直し」。

## Introduction

本 spec の要件は、brief の進め方（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）に従い、**「受け持つ範囲の audit の指摘を、3 つの組とも全部直す」という形だけ**を書く。指摘の中身はまだ書かない。指摘はギャップ分析で洗い出し、要件の討議で要件に落として直す対象を確定し、設計で指摘ごとの直し方を決め、実装で直す。

受け持つ範囲の大きさ（2026-11-05・main `88eb0a43` で測り直した）: 116 ファイル（Rust 114・`Cargo.toml` 2）・コード 7,842 行・テスト 19,202 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。起票のとき（main `e3c2cbe4`）の 99 ファイル・7,265 行・16,605 行から増えている。1,000 行の番人に近いファイルは `areka-sakura` の `compile_arm_tests.rs`（928）・`drive_lifecycle_tests.rs`（862）・`drive_delivery_tests.rs`（853）と、`areka-parsers` の `balloon/parse_tests.rs`（922）・`balloon/model.rs`（805）。

本 spec はウェーブ C17（2026-11-04 棚卸㉞）で、`doc-tree-tidy` と、見直しの 6 本（`audit-emo-present`・`audit-wintf-graphics-world`・`audit-ghost-catalog`・`audit-wintf-runtime`・`audit-emo-compose`・`audit-seriko`）と並んで走る。v0.2.0 の作り直しと触るファイルが重なるため、作り直しの前に済ませる（v0.2.0 の 8 本が本 spec の着地を待つ）。

## Boundary Context

- **In scope**:
  - `crates/areka-parsers/`・`crates/areka-sakura/` の全部のファイル（本番・テスト・`Cargo.toml`）への audit の指摘の直し。
  - そのためのテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し。今の呼び出し元のクレートは `areka`・`areka-emo-atlas`・`areka-emo-compose`・`areka-emo-present`・`areka-emo-text`・`areka-ghost`・`areka-nar`・`areka-seriko` の 8 つ（2026-11-05 に各 `Cargo.toml` で確かめた）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の持ち物）。呼び出し元の直しで範囲の外のファイルに触っても、そのファイルの指摘は直さない。
  - 新しい機能。
  - `vendors/`。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。audit が同じ所を指したら、その指摘としては直してよい。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 2）。
  - 依存の追加（同じウェーブの約束 ⑶＝`Cargo.lock` の席は空けておく）。
  - `areka-emo-text` へのファイルの追加（同じウェーブの約束 ⑶）。
  - `doc/`・`docs/` を名指す行の追加と変更（同じウェーブの約束 ⑴・要件 4.5）。
- **Adjacent expectations**:
  - 物差しは ponytail のプラグイン 5.3.2 の `/ponytail:ponytail-audit`（利用者全体のプラグイン・リポジトリに写さない）。
  - 同じウェーブの `doc-tree-tidy` は、このウェーブのあいだ `doc/COMPAT_ARCHITECTURE.md` と `doc/ssp-mcp/` の名前を変えない、を当てにする。
  - 同じウェーブの見直しのうち、本 spec の呼び出し元のクレートを受け持つのは `audit-emo-present`（`areka-emo-present`）・`audit-emo-compose`（`areka-emo-compose`）・`audit-seriko`（`areka-seriko`）・`audit-ghost-catalog`（`areka-ghost`・`areka-nar`）。見直しどうしの範囲は重ならない。直しが相手の範囲に及ぶときは、相手のセッションと申し合わせてから触る（要件 4.2）。
  - 同じクレートに触る `clippy-199-lints` と、本体の crate 分けの 6 本は、このウェーブに並ばない。

## Requirements

### Requirement 1: 受け持つ範囲の指摘を全部洗い出す
**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も省かれずに一覧になっていること, so that 直す対象を漏れなく確定できる

#### Acceptance Criteria
1. When ギャップ分析を行うとき, the 本 spec の見直し shall 受け持つ範囲（`crates/areka-parsers/`・`crates/areka-sakura/`）だけを対象に `/ponytail:ponytail-audit` をかける。
2. The 本 spec の見直し shall 受け持つ範囲のファイルを全部読み、audit の「読まなかった所（Not checked）」を残さない。
3. The 本 spec の見直し shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに一覧にする。
4. The 本 spec の見直し shall 指摘の一覧をギャップ分析の報告に残し、各指摘に audit の番号と組（必ず直す・直したほうがよい・できれば）をそのまま付ける。
5. When 要件の討議を行うとき, the 本 spec の見直し shall 指摘の一覧を要件に落とし、直す対象を audit の番号で確定する。
6. If 直す対象が 1 spec 20 タスクを越える見込みのとき, then the 本 spec の見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切り出すことを議題にする。

### Requirement 2: 指摘を 3 つの組とも全部直す
**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 3 つの組とも全部直っていること, so that 設定ファイルの読み取りとさくらスクリプトのコードが同じ物差しで一度きれいになり、v0.2.0 の作り直しをその上で始められる

#### Acceptance Criteria
1. The 本 spec の直し shall 要件の討議で確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。
2. The 本 spec の直し shall 直さずに残す指摘を、次の 3 種類に当たり、かつ要件の討議で開発者が残すと裁定したものだけにする: ① 開発者の裁定に反する直し ② わざと置いた先送りの印（実在の理由の説明を付けた `#[allow(dead_code)]` の語彙の口＝実装規律「語彙完備・配線ゼロの追跡」、および `shortcut:`・`ponytail:` の注釈で、限界が今の負荷を越えていないもの） ③ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。
3. If ある指摘が 2.2 の 3 種類のどれかに当たるとき, then the 本 spec の見直し shall その指摘を要件の討議の議題にし、推奨案を付けて開発者の裁定を仰ぐ（audit 系の議題は推奨案のまま承認してよい・2026-11-04 開発者「audit系は推奨案でよい」）。
4. When ある指摘が 2.2 の ③ に当たる見込みのとき, the 本 spec の見直し shall 議題にする前に ukadoc で正典を確かめ、確かめた項目（URL と原文の引用）を議題に添える。
5. The 本 spec の見直し shall brief か roadmap がほかの spec に割り当てた指摘を除き、指摘を範囲の外として落とさない。
6. The 本 spec の直し shall 指摘ごとに、その指摘を解く最も小さな完全な変更を採り、消せるものは消し、直しが及ぶ呼び出し元・テスト・fixture まで直し切る。
7. When 直しがコードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
8. The 本 spec の直し shall 外から見た振る舞いの変化を、要件の討議で確定した指摘の直しの分だけにする。

### Requirement 3: 直しを壊れたら落ちるテストで守る
**Objective:** As a 開発者, I want 直したバグと危ない所が、後の変更で壊れたらテストで分かること, so that 直しが黙って元に戻らない

#### Acceptance Criteria
1. When バグか危ない所の指摘を直したとき, the 本 spec の直し shall 指摘 1 件につき、その直しが壊れたら落ちるテストを 1 本以上添える。
2. The 本 spec で足したテストと直したテスト shall 決定論で回る（同じ入力で毎回同じ結果になる）。
3. When 直しが既にあるテストの前提を変えるとき, the 本 spec の直し shall そのテストを新しい振る舞いに合わせて直し、確かめの中身とテストの本数を、消した機能の分のほかは減らさない。

### Requirement 4: 範囲の境界と同じウェーブの約束を守る
**Objective:** As a 開発者と同じウェーブで並ぶほかのセッション, I want 本 spec の直しが受け持つ範囲と約束の外へはみ出さないこと, so that 並べて走る 7 本の仕事が互いに壊れない

#### Acceptance Criteria
1. When 直しで関数や型の形（引数・戻り値・公開の有無・名前）を変えるとき, the 本 spec の直し shall ツリー全体で呼び出し元を探し、範囲の外のものも含めて全部直す。
2. If 直しが同じウェーブで走っているほかの見直しの spec の受け持つファイルに及ぶとき, then the 本 spec の直し shall 相手のセッションと申し合わせてから触る。
3. While 呼び出し元の直しのために範囲の外のファイルを触るとき, the 本 spec の直し shall そのファイルの、本 spec の指摘の直しに要らない変更をしない。
4. The 本 spec の直し shall 依存を足さず、`areka-emo-text` にファイルを足さない。
5. The 本 spec の直し shall `doc/`・`docs/` を名指す行を足さず、今ある 3 行（`areka-sakura` の `strict.rs` の 5・15 行目＝`doc/ssp-mcp/areka-tools.md`、`sysvar.rs` の 44 行目＝`doc/COMPAT_ARCHITECTURE.md`、いずれも 2026-11-05 の行番号）の名指しを変えない。
6. If ある指摘の直しが 4.5 の 3 行の名指しを変えずには済まないとき, then the 本 spec の見直し shall その指摘を要件の討議の議題にする。
7. The 本 spec の直し shall `vendors/` に触らず、新しい機能を足さず、clippy の指摘を機械的には直さない（audit が同じ所を指した指摘は、その指摘として直す）。

### Requirement 5: 直しの中でもプロジェクトの決まりを守る
**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないこと, so that 見直しが新しい問題を持ち込まない

#### Acceptance Criteria
1. The 本 spec が触る本番とテストのファイル shall 1 ファイル 1,000 行の番人を越えない。足す分で越えるときは、新しいファイルへ置く。
2. If 直した処理が失敗するとき, then the 処理 shall 記録を残してから失敗を返す（記録の無い失敗の道を作らない）。
3. The 本 spec が触る終了の道 shall 正規の実装にする（小細工で済ませない）。
4. Where テストが実機の根か一時フォルダを使うとき, the テスト shall それをワークツリーの `target` の下に置く。
5. Where テストが GPU を使うとき, the テスト shall 実 GPU で回す。
6. If 文字化けに当たったとき, then the 本 spec の直し shall コンソールの文字コードを変えて逃げず、読む側を直す。

### Requirement 6: 仕上げに 0 件と全体テストの緑を確かめる
**Objective:** As a 開発者, I want 直した後の範囲に新しい指摘が無く、全体テストが緑であることを確かめた記録, so that 見直しが完了したと判断できる

#### Acceptance Criteria
1. When 確定した指摘を全部直し終えたとき, the 本 spec の見直し shall 同じ範囲へ、要件 1 と同じ作法（全件・全部読む）でもう一度 `/ponytail:ponytail-audit` をかける。
2. If かけ直しで新しい指摘が出たとき（直しが生んだ指摘を含む）, then the 本 spec の見直し shall 要件 2 と同じ扱いでそれも直し、かけ直しを繰り返す。
3. The 本 spec の見直し shall かけ直しで新しい指摘が 0 件であること（要件 2.2 で残すと裁定した指摘を除く）を、件数 0 と明示して検証の記録に残す。
4. When 完了を判定するとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑である。
