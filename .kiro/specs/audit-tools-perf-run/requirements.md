# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **性能改善ループの走らせ方（PowerShell）** を受け持つ（受け持つファイル＝`tools/perf/` の `*.ps1`・`*.psm1`。まだ audit をかけていない）。

着手のときの測り直し（2026-11-14・`main` `30a343eb`・brief と同じく `_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）: 8 ファイル＝コード 5,456 行・テスト 0 行・`*.psm1` は 0 ファイル。内訳は `check-quiet.ps1` 662 行・`invoke-cpu-sample.ps1` 606 行・`invoke-followup-checks.ps1` 865 行・`invoke-perf-run.ps1` 926 行・`perf-loop.common.ps1` 678 行・`perf-loop.measure.ps1` 993 行・`perf-loop.ps1` 685 行・`perf-pwsh-child.ps1` 41 行。起票のとき（`main` `e3c2cbe4`）の数と変わらない。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Introduction

この spec は、性能改善ループを走らせる PowerShell の道具（入口 `perf-loop.ps1` とその本体・共通の部分、静かさの確かめ `check-quiet.ps1`、CPU の採り方 `invoke-cpu-sample.ps1`、見た目の追随の検査 `invoke-followup-checks.ps1`、1 本の計測の走行 `invoke-perf-run.ps1`、子の pwsh の呼び方 `perf-pwsh-child.ps1`）を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

これらの道具は開発の道具で、areka の実行時のコードからは参照されない。代わりに、ループを回すスキル `perf-loop-iteration` と役割別のエージェント（`perf-measure`・`perf-implement` ほか）が入口のサブコマンドを呼び、判定と台帳の Python の道具（`audit-tools-perf-judge` の持ち物）が、この範囲の道具が書くファイルと行を読む。また、全体テストの段 `encoding check`（`tools/encoding-check.ps1`）が、この範囲のファイルを構文解析し、子の pwsh の呼び方を実際に呼んで確かめる。そのため、範囲の外から見た「公開の形」は、PowerShell の関数ではなく、サブコマンド・引数・終了コード・端末へ出す結果の行・書き出すファイルとその中の `key=value` の語・定数 `CHECK_ALL` と `SCRIPT_VERSION` である（以下「外への約束」。説明の正本は `tools/perf/README.md`）。

この spec はウェーブ C20 で、ほかの 7 本（`wintf-visual-damage-redraw`・`wintf-winmsg-executor-import`・`audit-placement-core`・`audit-placement-follow`・`audit-wintf-window`・`audit-wintf-input`・`audit-tools-perf-judge`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C20 の行の約束 ⑴〜⑹）を守る（要件 6）。

要件の討議で開発者に決めてもらうことは、次の 5 つである（どれも答えで作業が変わる）。

- 直さずに残す指摘があるか（要件 2.2 の 3 種類）。
- 直したバグ・危ない所のテストをどこで回すか（全体テストに段を足すかを含む・要件 4.2）。
- 外への約束を変えなければ直せない指摘の扱い（要件 6.5）。
- 実際の計測の走行でしか確かめられない直しの扱い（要件 7.3）。
- 直す対象のタスクが 20 を越えそうなときに、2 本目の spec へ切り分けるか（要件 2.6）。

## Boundary Context

- **In scope**:
  - `tools/perf/` の下の `*.ps1`・`*.psm1` 全部（今は直下の 8 ファイル）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元と読み手の直し（要件 3.2 と要件 6 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。範囲の外で見つけた指摘も、その範囲の spec の持ち物として残す（例: `tools/perf/` の Python・検体・`README.md`・`goals/` は `audit-tools-perf-judge` の持ち物、`tools/test-all.ps1`・`tools/encoding-check.ps1`・`tools/load-flake.ps1`・`tools/mouse-input.ps1` は `audit-tools-scripts` の持ち物）。
  - 新しい機能（新しいサブコマンド・新しい検査・新しい計測の段の追加）。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
  - 性能の要約の行を出す側（Rust のクレートの中の記録）の直し。
- **Adjacent expectations**:
  - 判定と台帳の Python の道具（`audit-tools-perf-judge`・C20 で並んで走る）は、この範囲の道具が書くファイル・行・語が今のまま続くことを前提にしている。C20 のあいだ、両方の spec は相手が読む口（引数・終了コード 0〜3・`key=value` の出力の語・`CHECK_ALL`・`SCRIPT_VERSION`）を申し合わせなしに変えない（C20 の約束 ⑷・要件 6.4）。
  - ループを回すスキル `perf-loop-iteration`（とその雛形 `templates/agent-prompts.md`）、役割別のエージェント（`.claude/agents/perf-*.md`）、`CLAUDE.md` の `perf-loop-iteration` の行は、入口のサブコマンドの名前・引数と、末尾の 1 行 `PERF-LOOP RESULT <sub> code=<n> dir=<path>` が今のまま続くことを前提にしている。
  - 全体テストの段 `encoding check` は、この範囲のファイルが端末へ出す文を ASCII だけに保ち、子の pwsh の呼び方が今の名前で呼べることを前提にしている。
  - この範囲の道具は、areka が出す性能の要約の行を読む。C20 のあいだ、`wintf-visual-damage-redraw` はその行の形を変えない（C20 の約束 ⑴）。この spec は、その行の形が今のまま続くことを前提にしてよい。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `tools/perf/` の下の `*.ps1`・`*.psm1` だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘を直すのに外への約束（サブコマンドの名前・引数・終了コード・端末へ出す結果の行の形・書き出すファイルの名前と置き場・その中の `key=value` の語・`CHECK_ALL`・`SCRIPT_VERSION`・`encoding check` の段が読み込む `perf-loop.common.ps1` とその中の `Invoke-PwshChild` の名前と使い方）を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの読み手（Python の道具・スキル・エージェント・`README.md`・`goals/`・`encoding check` の段）に響くかを書く（要件 6.5 の議題の材料）
7. When audit の指摘を確かめるのに実際の計測の走行（areka を立てて測る走行や、画面を操作する検査）が要るとき, the ギャップ分析 shall その指摘にその旨を書く（要件 7.3 の議題の材料）

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 要件の討議 shall ギャップ分析の指摘を全件、3 つの組とも直す対象として要件に落とす
2. The 本 spec shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る
   - ⑴ 開発者の裁定に反する直し
   - ⑵ わざと置いた先送りの印（実在の理由を書いた説明つきで使われていないことを許している語彙の口、および `shortcut:`・`ponytail:` の注釈で、書いた限界が今の負荷を越えていないもの）
   - ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
3. If 指摘が ⑶ に当たるかもしれないとき, then the 要件の討議 shall ukadoc で正典を確かめた結果（出どころと引用）を添えて議題にする
4. If 指摘の直しが開発者の裁定を覆すとき, then the 本 spec shall その指摘を直さず、議題にする
5. The 本 spec shall 直さずに残すと裁定された指摘ごとに、当たる種類と開発者の裁定を記録に残す
6. If 直す対象のタスクが 20 を越えそうなとき, then the 要件の討議 shall 指摘の組ごとに、同じ範囲の 2 本目の spec へ切り分けることを議題にしてよい

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・読み手・テスト・検体・設定・文書を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・検体・設定・スキル・エージェント・手順書・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 全体テストか道具の自分の検査（各道具の `-SelfTest` と `perf-loop.ps1 selftest`）のどちらかで回る形にする。どこで回すか（全体テストに段を足すかを含む）は要件の討議で開発者が決める（今の全体テストは `tools/perf/` の自分の検査を回していない）
3. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる
4. The 本 spec のテスト shall 実際の計測の走行（areka を立てて測る走行）・静かな机・画面の操作に頼らずに回る
5. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
6. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
7. When 既存の自分の検査が直しで古びたとき, the 本 spec の直し shall その検査を直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は、理由を書いた行と 0 でない終了コードの両方で表し、失敗を成功の終了コードで終わらせない）
3. The 本 spec の直し shall 終了の道（立てた子のプロセスの終わらせ方・途中で止めたときの後始末を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 端末へ出す文を ASCII だけに保ち、出す量を増やすときも呼ぶセッションのトークンの負荷を抑える形（数行）にする
6. The 本 spec の直し shall 依存を足さない（C20 の約束 ⑸＝`Cargo.lock` の席は `wintf-winmsg-executor-import` だけ。PowerShell のモジュールや Python のパッケージを新しく要るようにすることも含む）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 6: 範囲の外へ直しが及ぶとき・C20 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る（C20 では `tools/perf/` の Python・検体・`README.md`・`goals/` を受け持つ `audit-tools-perf-judge` がこれに当たる）
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C20 のあいだ, the 本 spec の直し shall C20 のほかの 6 本（`audit-tools-perf-judge` を除く）が触るファイル（ロードマップの C20 の行の「触るファイル」の ①〜⑥。`Cargo.lock`・根の `Cargo.toml`・`tools/crates-io.ps1` を含む）に手を入れない
4. While C20 のあいだ, the 本 spec の直し shall `audit-tools-perf-judge` が読む口（引数・終了コード 0〜3・`key=value` の出力の語・`CHECK_ALL`・`SCRIPT_VERSION`）を、そのセッションとの申し合わせなしに変えない（C20 の約束 ⑷）
5. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、読み手が `audit-tools-perf-judge` の持ち物ならそのセッションと申し合わせ、申し合わせで決まらないとき、または読み手がスキル・エージェントのときは要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・申し合わせたうえで両方で直す・後の spec へ回す）を開発者の裁定で決める
6. When 申し合わせか裁定に従って外への約束を変えるとき, the 本 spec の直し shall 同じ変更の中で、それを前提にしている範囲の外の読み手（Boundary Context の Adjacent expectations に挙げたもの。`audit-tools-perf-judge` の持ち物は申し合わせた分担に従う）を全部直し切る
7. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在することを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
8. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 7: 計測の走行と画面の操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッション, I want この spec の作業のあいだに、重い計測の走行や画面の操作が走らないでほしい, so that 互いの計測やテストの負荷を汚さない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、実際の計測の走行（入口のサブコマンド `measure-baseline`・`rank-run`・`measure-ab`・`followup`・`final` や、それらが呼ぶ走行）を回さない
2. The 本 spec の作業 shall 既にある走行の成果物（`%LOCALAPPDATA%\areka-diag\perf-loop\` の下の各ゴールの基準・周・最終の走行のフォルダ）と、ゴールの台帳と結果（`goals/` の各ゴールが指す spec の下）を、直しを確かめるために書き替えない
3. If 直しが実際の計測の走行でしか確かめられないとき, then the 本 spec shall その直しを決定論のテストで確かめられる形を先に探し、それでも足りない分は要件の討議で議題にし、扱い（静かな机で 1 本だけで回す・ウェーブの外へ回す・確かめを省く）を開発者の裁定で決める

### Requirement 8: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 6.5・6.7 で後へ回すと裁定された指摘は数えない。打ち切りは 8.7）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 道具の自分の検査（`pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest`）も回し、緑で終わったことを記録に残す
6. When 直しを全部終えたとき, the 検証 shall 要件 6.3・6.4 の約束が守られている（6.3 のファイルに差分が無い・6.4 の口が申し合わせた分のほかは変わっていない）ことを、既定のブランチとの差分で確かめて記録に残す
7. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と道具の振る舞い（終了コード・出力の行と語・書き出すファイル・測った値）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・audit-seriko と同じ形）
