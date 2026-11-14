# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **配置の crate（`areka-placement`）のうち追従と移り変わりの判定**（`crates/areka-placement/src/` の `follow*` と `transition_*`、本体 `crates/areka/src/placement_tests/` の `transition_*`）を受け持つ。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。`placement-crate-split` からの申し送り ⑴〜⑶ と、2026-11-14 棚卸㊲の再測定で見つかった古い名前も、ギャップ分析で判断する。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。

## Introduction

この文書は、配置の crate のうち追従と移り変わりの判定に `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直すための要件を定める。要件の段では「全部直す」という形だけを書き、指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書に落として直す対象を確定する。

受け持つ範囲の大きさ（着手のときの測り直し・`main` `30a343eb`）:

| 測った時点 | ファイル | コード | テスト |
|---|---|---|---|
| 起票のとき（`e3c2cbe4`・brief の値・`crates/areka/src/placement/` の下） | 47 | 7,154 行 | 19,569 行 |
| `placement-crate-split` の段 5（`dfc9974f`・brief の値） | 56 | 5,778 行 | 24,099 行 |
| 棚卸㊲の再測定（`df6f4e03`・brief の値） | 56 | — | — |
| 着手のとき（`30a343eb`） | 56 | 5,778 行 | 24,099 行 |

56 ファイルの内訳は、`crates/areka-placement/src/` の 41（`follow.rs`・`follow/` の下の 10・`transition_diag.rs` の本番 12 と、`follow_*_tests.rs`・`follow_test_support.rs`・`transition_diag_tests.rs` のテスト 29）と、本体 `crates/areka/src/placement_tests/` の `transition_*` の 15（テスト専用の判定器 `transition_judge*` とその檻・`transition_signoff*`）。テストに数えたのは、名前が `_tests.rs` か `_test_support.rs` で終わるファイルと、`placement_tests/` の下の全部。範囲の中は棚卸㊲の再測定（`df6f4e03`）から変わっていない。

1,000 行の上限を越えるファイルは 3 本＝`follow/window_move.rs`（1,231 行）・`placement_tests/transition_judge_tests.rs`（1,039 行）・`placement_tests/transition_judge_verdict_tests.rs`（1,037 行）。どれも番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`・範囲の外）に載っている。上限に近いのは `placement_tests/transition_judge.rs`（999 行）と `follow/drag_follow.rs`（978 行）。

## Boundary Context

- **In scope**:
  - 上の 56 ファイルに audit をかけて出た指摘の直し。
  - `placement-crate-split` からの申し送り ⑴〜⑶（brief の同名の節）と、受け持ちの 9 ファイルの説明に残る旧い名前 `emo2_boot`・`Emo2Wiring` の判断と直し（要件 2）。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 6 と要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。同じ crate の核（`crates/areka-placement/` の残りのファイルと本体 `placement_tests/` の残り 9 ファイル）は `audit-placement-core` の持ち物。
  - wintf の型（`ecs/window/monitor.rs` の `Monitor` ほか）。同じ C20 の `audit-wintf-window` の持ち物（要件 7.4）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 4）。
- **Adjacent expectations**:
  - 受け持つ範囲の公開の項目（申し送り ⑶ の再公開ほか）は、範囲の外の次の所が使う（着手のときの測り）: 同じ crate の核のファイル（`audit-placement-core` の範囲・参照は両向きに多い）、本体 `crates/areka/` の `runtime/`・`input_events/`・`mcp/`・`session/`・`main.rs`・`app_exit.rs` ほかのテストと `examples/collision-probe`、`crates/areka-placement/examples/window-placement.rs`。
  - テスト専用の判定器 `transition_judge*` を引くのは、本体の `runtime/frame_transition_atomicity_tests.rs` と `runtime/frame_transition_branch_tests.rs`。判定器の署名を変えると `runtime/` に触る。`runtime/frame_transition_branch_tests.rs` は番人の例外表に載り、番人の自己検査が表から外して試す 1 件でもある。
  - C21 の `runtime-crate-split` が、`transition_judge*` とそのテストを `areka-runtime` へ移す。この spec はその前に着地する。
  - 番人の例外表の持ち物は `audit-test-kits`（見込みは C22 で、この spec の後）。
  - C20 で並ぶほかの 7 本との約束は要件 7 に書く。

## Requirements

### Requirement 1: 指摘の洗い出し

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行う, the 本見直し shall 受け持つ 56 ファイル（Introduction の内訳）だけを対象にして `/ponytail:ponytail-audit` をかける
2. The 本見直し shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The 本見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない
4. The 本見直し shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. When 要件の討議で直す対象が確定した, the 本見直し shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか）をこの文書に書き加える

### Requirement 2: 申し送りと古い名前の判断

**Objective:** As a 開発者, I want `placement-crate-split` が直さずに残した所と古くなった名前が、audit が指したかどうかにかかわらず判断されていること, so that crate を分けたときの持ち越しが見直しの後に残らない

#### Acceptance Criteria

1. When ギャップ分析を行う, the 本見直し shall brief の申し送り ⑴〜⑶ の項目を 1 つずつ、行の番号を今のコードで確かめ直したうえで、「直す」「理由が今も事実なので残す」「要件 7 の約束のために C20 のうちには直せない」のどれに当たるかを判断し、ギャップ分析の報告に残す
2. The 本見直し shall 申し送り ⑵ の `#[allow]` と説明のうち、理由の文が今の事実（配置が lib の crate であること・examples が `#[path]` でソースを取り込まないこと）に合わないものを、そのまま残さない
3. When 申し送り ⑵ の `#[allow]` を判断する, the 本見直し shall 属性そのものが今も要るか（lib の公開の項目に lint が鳴るか）を確かめ、要らなければ属性を消し、要るなら今の事実に合う理由へ書き直す
4. If 申し送り ⑵ の属性が語彙の口（実装規律「語彙完備・配線ゼロの追跡」）を表している, the 本見直し shall 属性を消すかどうかと語彙の口そのものを残すかどうかを分けて判断し、語彙の口を消す直しは要件 4 に従う
5. The 本見直し shall 申し送り ⑵ のうち brief が「古くなっていない」とした `#[allow]`（`follow.rs` の 108 行・123 行と `follow/window_move.rs` の 449 行）を、理由が今も事実であることを確かめたうえで、直す対象に数えない
6. The 本見直し shall 申し送り ⑶ の公開の項目を 1 つずつ、範囲の外から使われているかで判断し、要らない公開を一覧にする（公開を狭める直しは要件 7.1 と要件 7.3 に従う）
7. The 本見直し shall 申し送り ⑴ の wintf との重なり（`follow/work_area.rs` の `MonitorSnapshot` が包む `Monitor`）を判断し、wintf の型に寄せる直しは要件 7.4 に従う
8. The 本見直し shall 受け持ちのファイルの説明に残る旧い名前 `emo2_boot`・`Emo2Wiring`（着手のときに 9 ファイル）を、直すかどうか判断する
9. If 旧い名前 `emo2_boot`・`Emo2Wiring` を直した, the 本見直し shall 直したことを `runtime-crate-split` の brief に申し送る（roadmap の C19 の約束 ⑵ でその spec が直す予定だった分の手を減らすため）

### Requirement 3: 指摘を全部直す

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that 追従と移り変わりの判定のコードからバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本見直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘と、要件 2 で「直す」と判断した項目を全部直す
2. The 本見直し shall 要件 4 の 3 種類のどれかに当たって開発者が残すと裁定した指摘と、要件 7.3 で開発者が後へ回すと裁定した指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出た, the 本見直し shall その箇所を使うテスト・fixture・呼び出し元まで直し切り、直しの途中の状態を残さない

### Requirement 4: 直さずに残す指摘は開発者の裁定で決める

**Objective:** As a 開発者, I want 直さずに残す指摘を自分で裁定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. If 指摘の直しが次の 3 種類のどれかに当たる, the 本見直し shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. When 3 種類目に当たるかを判断する, the 本見直し shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする
3. If 指摘の直しが 3 種類のどれにも当たらない, the 本見直し shall その指摘を残すことを議題にせず直す
4. When 開発者が指摘を残すと裁定した, the 本見直し shall 裁定とその理由をこの文書に残し、要件 5 の「新しい指摘 0 件」の数から外す

### Requirement 5: 直した後の確かめ

**Objective:** As a 開発者, I want 直しが新しい問題を生んでいないことを確かめられること, so that 見直しが終わったと安心して言える

#### Acceptance Criteria

1. When 確定した指摘を全部直し終えた, the 本見直し shall 同じ範囲に、要件 1 と同じ作法（件数の上限なし・全部読む）で `/ponytail:ponytail-audit` をもう一度かける
2. If もう一度かけた audit が新しい指摘（直しが生んだ指摘を含む）を出した, the 本見直し shall その指摘も直し、新しい指摘が 0 件になるまで audit をかけ直す
3. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無い, the 本見直し shall 開発者に一言知らせたうえで、5 回目の指摘を直したところでかけ直しを打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書く（2026-11-06 `audit-seriko` で開発者が承認した形）
4. The 本見直し shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を検証の記録に残す
5. When 見直しを終える, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる
6. The 本見直し shall 直したバグと危ない所の 1 件ずつに、その直しが壊れたら落ちるテストを 1 本以上置く
7. The 本見直し shall 足したテストと直したテストを、毎回同じ結果になる（決定論の）テストにする

### Requirement 6: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが安全に、並んで走るほかの spec とぶつからずに入ること, so that ほかのクレートが壊れず、並行作業が止まらない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えた, the 本見直し shall ツリー全体で呼び出し元を探して、全部直す
2. If 直しが及ぶ呼び出し元の範囲を受け持つ spec のセッションが走っている, the 本見直し shall そのセッションと申し合わせてから、その呼び出し元に触る
3. When コード・テスト・fixture を消す, the 本見直し shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる
4. The 本見直し shall 範囲の外のファイルの、直しに要らない箇所を直さない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）

### Requirement 7: C20 の並走の約束

**Objective:** As a 開発者, I want 見直しの直しが同じウェーブ C20 のほかの spec の邪魔をしないこと, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While C20 のあいだ, the 本見直し shall `areka-placement` の公開の形（crate の外から呼べる型・関数・欄の名前と形）を、足すことのほかは変えない（C20 の約束 ⑵）
2. While C20 のあいだ, the 本見直し shall `audit-placement-core` の受け持つファイルが使う項目の形（署名）を足すことのほかは変えず、相手の受け持つファイルの関数の形を変えない（C20 の約束 ⑵＝両向きに呼び合う）
3. If 確定した指摘が要件 7.1 か要件 7.2 の形を変えずには直せない, the 本見直し shall その指摘だけを直さずに議題にし、「相手のセッション（`audit-placement-core`、または使い手の範囲を受け持つ spec）と申し合わせて C20 のうちに直す」か「後の spec へ申し送る」かを開発者の裁定で決める
4. While C20 のあいだ, the 本見直し shall wintf の型と wintf のファイルに手を入れず、受け持ちの型を wintf の型に寄せる直しが要るときは、`audit-wintf-window` のセッションと申し合わせてから行う（C20 の約束 ⑶）
5. The 本見直し shall 依存を足さず、`Cargo.lock` を変えない（C20 の約束 ⑸＝`Cargo.lock` の席は `wintf-winmsg-executor-import`）
6. If 直しに新しい依存が要ると分かった, the 本見直し shall 依存を足さずに開発者へ議題として出す（足すなら `tech.md` への登記と開発者の承認が要る）
7. The 本見直し shall `vendors/` の下に手を入れない

### Requirement 8: 守る決まり

**Objective:** As a 開発者, I want 見直しの直しがプロジェクトの決まりを守ること, so that 直しが別の決まりを破らない

#### Acceptance Criteria

1. The 本見直し shall 1 ファイル 1,000 行の番人を赤にせず、直しで行が上限を越えるときは新しいファイルへ分け、番人の例外表へ新しいファイルを足さない
2. The 本見直し shall 番人の例外表に載っている受け持ちの 3 本（`follow/window_move.rs`・`placement_tests/transition_judge_tests.rs`・`placement_tests/transition_judge_verdict_tests.rs`）に行を足さず、足す分は新しいファイルへ置く
3. If 直しで例外表に載っているファイルが分けられて 1,000 行の上限の下に収まった, the 本見直し shall そのファイルの行を例外表と件数の定数から消す（直しに要る最小の範囲の外の変更として行う。表の持ち物の `audit-test-kits` はこの spec の後に着手する）
4. The 本見直し shall 記録の無い失敗の道を作らず、失敗は記録（`error!`）を残したうえで呼び出し元へ失敗（`Err`）として返す
5. The 本見直し shall 終了の道を正規の実装にし、終了を早めるための小細工を入れない
6. The 本見直し shall 実機の根と一時フォルダを、ワークツリーの `target` の下に置く
7. The 本見直し shall GPU のテストを実 GPU で回す
8. If 文字化けが起きた, the 本見直し shall コンソールの文字コードを変えて逃げず、読む側を直す

### Requirement 9: spec の大きさ

**Objective:** As a 開発者, I want 1 本の spec が大きくなりすぎないこと, so that 1 本ずつ確かめて着地できる

#### Acceptance Criteria

1. If 確定した指摘の直しが 20 タスクを越えそうになった, the 本見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切ることを議題にする
