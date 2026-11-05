# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **wintf の ECS の外（刻みの土台・COM・窓の型・API・数の型）** を受け持つ（`crates/wintf/` のうち `src/ecs/` の外のもの全部）。まだ audit をかけていない。指摘はギャップ分析で洗い出す。開発者の方針は「とにかく指摘事項は全部直す」（2026-10-19）で、進め方は「原則、ギャップ分析でauditを実施して、設計以降で修正」（同じ日）。正本は `brief.md` と roadmap の「テーマ別の決めごと」の「ponytail による見直し」。

## Introduction

この spec は、wintf のうち ECS の外にあるコードへ ponytail の audit をかけ、出た指摘を 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。要件の段では「何を、どの作法で見直し、何をもって終わりとするか」の形だけを決める。指摘の中身はギャップ分析で洗い出し、要件の討議でこの文書へ落として直す対象を確定する（下の要件 3）。

着手のとき（main `88eb0a43`）に測り直した大きさ: `crates/wintf/` のうち `src/ecs/` の外は git の管理下に 160 ファイル。Rust のコードは空行を除いて約 12,100 行、テスト（`_tests.rs`・`tests/`・`_test_support.rs`）は約 17,000 行（起票のときは 156 ファイル・コード 12,257 行・テスト 20,138 行＝数え方が違う）。

## Boundary Context

- **In scope**:
  - 受け持つファイル＝`crates/wintf/` のうち `src/ecs/` の外のもの全部（`src/runtime/`・`src/com/`・`src/numerics/`・`win_style*`・`win_state*`・`api*`・`lib.rs`・`test_crash_report.rs`・`tests/`・`examples/`・`examples.manifest`・`build.rs`・`README.md`・`Cargo.toml`）。
  - 受け持つファイルの指摘の直し、そのためのテストの追加と直し、直しのために要る範囲の外の呼び出し元の直し（要件 6 の約束に従う）。
- **Out of scope**:
  - 範囲の外の見直し（ほかの 32 本の見直しの持ち物。`crates/wintf/src/ecs/` は `audit-wintf-graphics-world` ほか wintf の見直しの持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し（直さずに議題にする）。
- **Adjacent expectations**:
  - `wintf-visual-tree`（v0.2.0）: `crates/wintf/src/ecs/visual/` と、そこからだけ使われる `src/numerics/` の働き（`Transform2D`・`Aabb`・`Matrix3x2Ext` ほか）を本番の描画層へ上げる。この spec はそれらを「使われていない」ことを理由に消さない。
  - `wintf-dwmflush-wait-compensation`（版に縛らない・未着手）: `src/runtime/tick_bridge.rs` の vblank のスレッドの空回りと、`Err` の回に起こさない穴を直す。同じファイルを触るので同じウェーブに並べない。
  - `audit-wintf-graphics-world` ほか同じウェーブ C17 の見直し 6 本と `doc-tree-tidy`: 受け持つファイルは重ならない。ウェーブの約束（`doc/`・`docs/` を名指す行を足さず変えない・誰も依存を足さない・emo-text にファイルを足さない）を守る。
  - `clippy-199-lints`: 同じクレートに触る。見直しの段のあいだは並べない。

## Requirements

### Requirement 1: 範囲の確定

**Objective:** As a 開発者, I want 見直しが受け持つファイルを着手のときの実物で確定したい, so that 見直しの漏れも、ほかの見直しとの二重の持ち物も起きない

#### Acceptance Criteria

1. When ギャップ分析を始めるとき, the 見直し shall `crates/wintf/` のうち `src/ecs/` の外にある git の管理下のファイルを全部数え、その一覧と数を記録に残す
2. The 見直し shall `crates/wintf/src/ecs/` の下のファイルと `vendors/` の下のファイルを、受け持つファイルに含めない
3. If 範囲のファイルの一覧が、着手のときより後に（ほかの spec の着地などで）増えたり減ったりしたとき, the 見直し shall 一覧と数を測り直し、増えたファイルも見直しの対象に入れる

### Requirement 2: 指摘の洗い出し

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず読み残さずに洗い出したい, so that 「全部直す」の「全部」が何件かが確かめられる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the 見直し shall 受け持つ範囲だけを対象にして `/ponytail:ponytail-audit` をかける
2. The 見直し shall audit の作法のうち「最大 20 件」を外し、出た指摘を全件記録して、省いた件を残さない
3. The 見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読む
4. The 見直し shall 指摘の一覧をギャップ分析の報告に残し、各指摘に audit の番号をそのまま付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに入るかを書く
5. The 見直し shall ギャップ分析の報告に、3 つの組それぞれの件数を、0 件の組も数字で書く

### Requirement 3: 直す対象の確定

**Objective:** As a 開発者, I want 指摘の一覧を要件に落として、直すものと残すものを自分の裁定で確定したい, so that 設計と実装が直す対象をはっきり知っている

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 見直し shall ギャップ分析の指摘を全件この要件の文書へ落とし、各指摘を「直す」か「直さずに残す」のどちらかに確定する
2. The 見直し shall 直さずに残してよい指摘を、次の 3 種類だけに限り、どれも開発者の裁定を得てから残す: ⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印（実在の理由の doc を付けた `#[allow(dead_code)]` の語彙の口と、`shortcut:`・`ponytail:` の注釈のうち限界が今の負荷を越えていないもの） ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
3. If 指摘が ⑶ に当たりそうなとき, the 見直し shall ukadoc で確かめた結果を添えて開発者の裁定を仰ぐ
4. If audit が `crates/wintf/src/numerics/` のうち `wintf-visual-tree` が本番へ上げる働きを「使われていない」「`allow(dead_code)`」と指したとき, the 見直し shall それを消さず、⑵ として `wintf-visual-tree` へ申し送ることを要件の討議で開発者に確かめる
5. If 直す指摘が `wintf-visual-tree` の brief の「直すべき傷」1〜8 と重なるとき, the 見直し shall その旨をこの spec の記録に残し、あちらのギャップ分析が二重に数えないようにする
6. If 指摘が `wintf-dwmflush-wait-compensation` の直す中身（vblank のスレッドの空回り・`Err` の回に起こさない穴・`AsyncTickTask` の注記の食い違い）と重なるとき, the 見直し shall その旨をこの spec の記録に残し、どちらの spec で直すかを要件の討議で開発者に確かめる
7. If 直す対象の見立てが 1 spec 20 タスクを越えそうなとき, the 見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切る案を開発者に出す

### Requirement 4: 指摘を全部直す

**Objective:** As a 開発者, I want 「直す」と確定した指摘を 3 つの組とも全部直したい, so that 受け持つ範囲にバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードが残らない

#### Acceptance Criteria

1. The 見直し shall 「直す」と確定した指摘を、必ず直す・直したほうがよい・できれば の 3 つの組とも全部直す
2. The 見直し shall 各指摘を、その指摘を解く最も小さな完全な変更で直し、消せるものは消し、直しが壊す呼び出し元・テスト・fixture・設定まで直し切る
3. When コードや設定を消すとき, the 見直し shall その前に、テスト・fixture・設定・文字列での参照を含むツリー全体で使われていないことを確かめ、確かめた結果を記録に残す
4. If 直しが wintf の公開の口（クレートの外から使える型・関数・定数）を消したり形を変えたりするとき, the 見直し shall 変わった公開の口の一覧を記録に残し、次の版の知らせに書けるようにする
5. The 見直し shall 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直しをしない

### Requirement 5: 直しが守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないようにしたい, so that 見直しが新しい傷を作らない

#### Acceptance Criteria

1. The 見直し shall 本番のファイルとテストのファイルのどちらも、1 ファイル 1,000 行の番人を越えさせず、足すものが入りきらないときは新しいファイルへ分ける
2. The 見直し shall 失敗を記録なしに捨てる道を作らず、失敗の道ではエラーの記録を残したうえで呼び出し元へ失敗を返す
3. The 見直し shall 終了の道を正規の手順のまま保ち、小細工で終わらせる道を作らない
4. Where 直しのテストが実機の根や一時フォルダを使うとき, the 見直し shall それらをワークツリーの `target` の下に置く
5. Where 直しのテストが GPU を使うとき, the 見直し shall 実 GPU で回し、ソフトウェアの描き手へ置き換えない
6. If 文字化けに出会ったとき, the 見直し shall コンソールの文字コードを変えて逃げず、読む側を直す
7. While ウェーブ C17 が走っているあいだ, the 見直し shall 依存を足さない（足す必要が出たら直さずに議題にし、足すと決まったら `tech.md` へ登記して開発者の承認を得る）

### Requirement 6: 範囲の外へ及ぶ直し

**Objective:** As a 開発者, I want 範囲の外の呼び出し元へ直しが及んでも、ほかの見直しや同じウェーブの仕事と衝突しないようにしたい, so that 並べて走らせている spec がお互いを壊さない

#### Acceptance Criteria

1. When 直しが関数や型の形を変えるとき, the 見直し shall ツリー全体で呼び出し元を探し、全部直す
2. If 直す呼び出し元がほかの見直しの spec の受け持つファイルにあり、その spec が走っているとき, the 見直し shall そのセッションと申し合わせてから触る
3. The 見直し shall 受け持つファイルで `doc/`・`docs/` を名指す行を足さず、変えない（同じウェーブの `doc-tree-tidy` との約束）
4. The 見直し shall emo-text のクレートに新しいソースファイルを足さない（同じウェーブの約束）
5. The 見直し shall `vendors/` の下を変えない

### Requirement 7: 壊れたら落ちるテスト

**Objective:** As a 開発者, I want 直したバグと危ない所が、また壊れたら必ず気付けるようにしたい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 見直し shall 直したバグと危ない所の指摘ごとに、その直しが壊れたら落ちるテストを 1 本以上持つ
2. The 見直し shall それらのテストを、実行のたびに同じ答えになる決定論のテストにする
3. When 指摘を全部直し終えたとき, the 見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）を緑にする

### Requirement 8: かけ直しで 0 件を確かめる

**Objective:** As a 開発者, I want 直した後の範囲にもう一度 audit をかけて、指摘が残っていないことを確かめたい, so that 「全部直した」を報告ではなく測った結果で言える

#### Acceptance Criteria

1. When 指摘を全部直し終えたとき, the 見直し shall 要件 1 で確定した範囲（その後の増減を含む）へ、要件 2 と同じ作法（全件・全部読む）で `/ponytail:ponytail-audit` をかけ直す
2. If かけ直しで新しい指摘（直しが生んだ指摘を含む）が出たとき, the 見直し shall それも要件 3〜7 と同じ扱いで直し、要件 3 で「直さずに残す」と裁定された指摘を除いて 0 件になるまでかけ直す
3. The 見直し shall かけ直しの記録に、読んだファイルの数が範囲のファイルの数と一致することと、3 つの組それぞれの件数（0 件も数字で）と、要件 3 で「直さずに残す」と裁定された指摘の番号を書く
