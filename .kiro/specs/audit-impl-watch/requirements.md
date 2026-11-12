# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **机の貸し出しのアプリ（impl-watch）** を受け持つ（受け持つファイル＝`crates/areka-impl-watch/` 全部。まだ audit をかけていない）。

着手のときの測り直し（2026-11-13・`main` `4863cb66`・brief と同じく `_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）: 40 ファイル＝コード 12 ファイル 3,759 行・テスト 27 ファイル 11,945 行・`Cargo.toml` 1 ファイル 25 行。起票のとき（`main` `e3c2cbe4`）の数と変わらない。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Introduction

この spec は、並んで走る実装セッションに 3 種類の机（実装の机・排他テストの机・マージの机）を貸し出すコンソールアプリ `areka-impl-watch` のクレート `crates/areka-impl-watch/` を、ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

このアプリは開発の道具で、areka の実行時のコードからは参照されない。代わりに、並んで走る各セッションが、置き場所（環境変数 `AREKA_IMPL_WATCH_HOME`）に置いた実行ファイルをスキルから直に呼んでいる。そのため、範囲の外から見た「公開の形」は、Rust の型や関数ではなく、コマンド・引数・終了コード・端末へ出す文・状態ファイルである（以下「外への約束」。正本は `doc/impl-watch.md`）。

この spec はウェーブ C19 で、ほかの 7 本（`wintf-d2d-composite-rules`・`dola-yaml-backend`・`runtime-module-untangle`・`audit-mcp-update-crates`・`audit-shiori-host32`・`audit-emo-atlas`・`emo-text-reveal-incremental`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C19 の行の約束 ⑴〜⑷）を守る（要件 6）。

## Boundary Context

- **In scope**:
  - `crates/areka-impl-watch/` の全ファイル（本番のコード・テスト・テストの道具・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し（常時のテストと、本物の実行ファイルを通す実機テスト `tests/real.rs` の両方）。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3.2 と要件 6 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。範囲の外で見つけた指摘も、その範囲の spec の持ち物として残す（例: 置き方の道具 `tools/impl-watch-install.ps1` は `audit-tools-scripts` の持ち物）。
  - 新しい機能（新しい机・新しいコマンド・机の決まりの追加）。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。机の決まり（`doc/impl-watch.md` の 1 節の「決まり」1〜11）は、完了した `areka-P0-impl-watch`・`areka-P0-impl-watch-test-desk`・`areka-P0-impl-watch-skills`・`areka-P0-impl-watch-residue` の要件で決まったものなので、これに当たる。
- **Adjacent expectations**:
  - 実行ファイルを呼ぶ範囲の外のもの（スキル `kiro-impl`・`kiro-complete`・`kiro-next`・`release`・`update`・`kiro-watch-clear`、手順書 `doc/impl-watch.md`、置き方の道具 `tools/impl-watch-install.ps1`、steering の `structure.md` の Impl Watch の節）は、外への約束が今のまま続くことを前提にしている。C19 のあいだ、この spec は外への約束を足すことのほかは変えない（要件 6.4）。
  - 走っているほかのセッションは、置き場所に置いた実行ファイルと状態ファイルを今も使っている。この spec の作業は、それらを入れ替えも書き替えもしない（要件 7）。
  - C19 で並ぶほかの 7 本とは、受け持つファイルが重ならない（ロードマップの C19 の行＝⑧ は impl-watch の道具だけを触る）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `crates/areka-impl-watch/` だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘を直すのに外への約束（コマンド・引数・終了コード・端末へ出す文の形・`--help` の今ある行・状態ファイルの形式とその版・置き場所の下のファイルの名前）を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの呼び出し元（スキル・手順書・置き方の道具・steering）に響くかを書く（要件 6.5 の議題の材料）

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 要件の討議 shall ギャップ分析の指摘を全件、3 つの組とも直す対象として要件に落とす
2. The 本 spec shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る
   - ⑴ 開発者の裁定に反する直し（机の決まりを変える直しを含む）
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
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・fixture・設定・文書を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・fixture・設定・スキル・手順書・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる
3. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
4. Where テストが実行ファイルを子プロセスに立てるところでは, the 本 spec のテスト shall 置き場所（`AREKA_IMPL_WATCH_HOME`）をワークツリーの `target` の下の Windows 形の絶対パスで必ず上書きし、開発者の機械に設定された本物の置き場所に触れない
5. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
6. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録とエラーの戻り値の両方で表す。判断の核は記録を綴らない今の作りのまま、記録は口の側で出す）
3. The 本 spec の直し shall 終了の道（待ち・見張りの終わり方を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 端末へ出す文を ASCII だけに保ち、出す量を増やすときも呼ぶセッションのトークンの負荷を抑える形（数行）にする
6. The 本 spec の直し shall 依存を足さない（C19 の約束 ⑷＝`Cargo.lock` の席は `dola-yaml-backend` だけ）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 6: 範囲の外へ直しが及ぶとき・C19 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C19 のあいだ, the 本 spec の直し shall 次のファイルに手を入れない: `crates/areka/` の全部（C19 の約束 ⑴）・`Cargo.lock`（約束 ⑷）・`runtime-module-untangle` が本体の外で書き換えるファイル（`crates/wintf/src/test_crash_report.rs`・`crates/log-capture-kit` の番人 2 つ＝`file_length_guard_test.rs`・`workspace_scan_test.rs`・`tools/load-flake.ps1`・`doc/ssp-mcp/log-convention.md`・`doc/COMPAT_ARCHITECTURE.md`・`doc/emo2-conformance-scope.md`）
4. While C19 のあいだ, the 本 spec の直し shall 外への約束（要件 1.6 に挙げたもの）を、足すことのほかは変えない（C19 の約束 ⑴「自分のクレートの公開面は足すだけ」）
5. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・C19 のほかの 7 本の着地を待って本 spec の中で直す・後の spec へ回す）を開発者の裁定で決める
6. When 裁定に従って外への約束を変えるとき, the 本 spec の直し shall 同じ変更の中で、それを前提にしている範囲の外のもの（Boundary Context の Adjacent expectations に挙げた呼び出し元＝スキル・`doc/impl-watch.md`・`tools/impl-watch-install.ps1`・steering の `structure.md`、およびスキルの中にある手順書の写し）を全部直し切る
7. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在することを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
8. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 7: 開発者の机で動いているものに触れない

**Objective:** As a 並んで走るほかのセッション, I want この spec の作業のあいだも、置き場所の実行ファイルと状態ファイルが今のまま動いてほしい, so that 見直しの途中の実行ファイルで机の記録が壊れたり、待ち・見張りが終わったりしない

#### Acceptance Criteria

1. The 本 spec の作業 shall 置き場所（`AREKA_IMPL_WATCH_HOME`）の実行ファイルを、ワークツリーで組んだもので入れ替えない
2. The 本 spec の作業 shall 置き場所の状態ファイル・ロック・居る印・ログ・`status.md` を、机のコマンドを正規に呼ぶことのほかで書き替えない（直しを確かめるために手で走らせるときも、置き場所をワークツリーの `target` の下へ上書きする）
3. If 直しが状態ファイルの形式の版を上げるとき, then the 本 spec shall 要件 6.5 の議題にする（版が上がる入れ替えでは、置き方の道具が `clear` を呼び、走っているすべての待ち・見張りが終わるため）

### Requirement 8: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 6.5・6.7 で後へ回すと裁定された指摘は数えない。打ち切りは 8.7）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 本物の実行ファイルを通す実機テスト（`cargo test -p areka-impl-watch --test real -- --ignored`。全体テストには入らない）も回し、緑で終わったことを記録に残す
6. When 直しを全部終えたとき, the 検証 shall 要件 6.3・6.4 の約束が守られている（6.3 のファイルに差分が無い・外への約束が足すことのほかは変わっていない）ことを、既定のブランチとの差分で確かめて記録に残す
7. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と実行ファイルの振る舞いに関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・audit-seriko と同じ形）
