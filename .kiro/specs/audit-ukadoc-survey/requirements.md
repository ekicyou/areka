# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **ukadoc の網羅の調べの道具（ukadoc-survey）** を受け持つ（受け持つファイル＝`crates/ukadoc-survey/` 全部。まだ audit をかけていない）。

着手のときの測り直し（2026-11-17・`main` `5749a4c7`・brief と同じく `_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）: 96 ファイル＝コード 43 ファイル 9,099 行・テスト 52 ファイル 24,988 行・`Cargo.toml` 1 ファイル 25 行。起票のとき（`main` `e3c2cbe4`）の数と変わらない。物差しの ponytail は今入っている 5.3.3 を使う（brief の 5.3.2 から上がった・audit の作法は同じ）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Introduction

この spec は、ukadoc（SSP 公式仕様書）の項目について「正典の写し（カタログ）」と「areka の判定（台帳）」を建て、その整合を常時走るテストで守る調べの道具 `ukadoc-survey`（`crates/ukadoc-survey/`）を、ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

この道具は開発の道具で、areka の実行時のコードからは 1 行も参照されない。代わりに、範囲の外から次の形で使われている。そのため、範囲の外から見た「公開の形」は、Rust の型や関数ではなく、次のもの（以下「外への約束」）である。

- 副手続き 9 つ（`catalog`・`ledger-init`・`report`・`report-summary`・`check`・`evidence`・`candidates`・`diff`・`priority-apply`。どれも引数を取らない）と、その終了コード・端末へ出す文（手引きは `doc/ukadoc-coverage/README.md`）。
- 環境変数 `AREKA_UKADOC_SNAPSHOT`（スナップショットの場所）。
- 道具が読み書きする `doc/ukadoc-coverage/` の下のファイル（カタログ・台帳・報告など）の形式と、同じ入力から書き出す中身。台帳の項目形式・状態の語彙・ドメインの分割は、完了した `areka-P0-ukadoc-survey-toolkit` の要件 2.1・2.2・2.6 と付録 A で凍結されている。
- 常時走る整合テスト（`cargo test -p ukadoc-survey`）が何を見て落ちるか。スキル `kiro-next`（Step 8 でこのテストを回す・新しい spec のフォルダを作っても赤にならない前提）と `kiro-complete`（`.kiro/specs/` の直下の数え直しのため、テストの最中にフォルダを増減しない手順）は、この振る舞いを前提にしている。

この spec はウェーブ C22 で、ほかの 5 本（`collisionex-regions`・`audit-placement-follow-residue`・`audit-emo2-boot-spine`・`audit-test-kits`・`audit-pilot`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C22 の行の約束 ⑴〜⑸）を守る（要件 6）。

## Boundary Context

- **In scope**:
  - `crates/ukadoc-survey/` の全ファイル（本番のコード・テスト・テストの道具・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し（単体のテストと、`tests/` の整合テスト・副手続きの出力のテストの両方）。
  - 直しのために要る、範囲の外の呼び出し元・文書の直し（要件 3.2 と要件 6 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。範囲の外で見つけた指摘も、その範囲の spec の持ち物として残す。
  - 新しい機能（新しい副手続き・新しい検査・台帳の新しい欄）。
  - カタログの作り直しや台帳の中身の更新（`ukadoc-catalog-refresh` の持ち物）。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。台帳の形式の凍結（`areka-P0-ukadoc-survey-toolkit` の付録 A ほか）はこれに当たる。
- **Adjacent expectations**:
  - 道具を呼ぶ・道具の振る舞いを前提にする範囲の外のもの（手引き `doc/ukadoc-coverage/README.md`、スキル `kiro-next`・`kiro-complete`、steering の `workflow.md`・`structure.md` の ukadoc Survey Toolkit の節、`doc/ukadoc-coverage/` の下のデータ）は、外への約束が今のまま続くことを前提にしている。C22 のあいだ、この spec は外への約束を変えない（要件 6.4）。
  - 同じクレートに触る未着手の `clippy-199-lints`・`ukadoc-catalog-refresh`、`crates/ukadoc-survey/src/evidence/candidates.rs` の説明の中のパスを直す見込みの `input-crate-split`、台帳の根拠の欄の値に触る見込みの `doc-compat-architecture-rename` は、この spec の着地の後に走る。この spec は、それらの brief に書かれた前提（副手続きの名前・台帳の欄の読み方）を壊さない。
  - C22 で並ぶほかの 5 本とは、受け持つファイルが重ならない（ロードマップの C22 の行＝⑤ は `crates/ukadoc-survey/` だけを触る）。ただし `collisionex-regions` は `doc/ukadoc-coverage/ledger/assets.toml` の 2 行を書き替え、この道具の整合テストがそれを読む。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `crates/ukadoc-survey/` だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘を直すのに外への約束（副手続きの名前と引数・終了コード・端末へ出す文の形・環境変数・`doc/ukadoc-coverage/` の下のファイルの形式と同じ入力から書き出す中身・整合テストが落とす条件）を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの呼び出し元（手引き・スキル・steering・データ）に響くかを書く（要件 6.5 の議題の材料）

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 要件の討議 shall ギャップ分析の指摘を全件、3 つの組とも直す対象として要件に落とす
2. The 本 spec shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る
   - ⑴ 開発者の裁定に反する直し（台帳の形式の凍結を変える直しを含む）
   - ⑵ わざと置いた先送りの印（実在の理由を書いた説明つきで使われていないことを許している語彙の口、および `shortcut:`・`ponytail:` の注釈で、書いた限界が今の負荷を越えていないもの）
   - ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの（カタログに写す正典の項目の読み方・数え方を変える直しを含む）
3. If 指摘が ⑶ に当たるかもしれないとき, then the 要件の討議 shall ukadoc で正典を確かめた結果（出どころと引用）を添えて議題にする
4. If 指摘の直しが開発者の裁定を覆すとき, then the 本 spec shall その指摘を直さず、議題にする
5. The 本 spec shall 直さずに残すと裁定された指摘ごとに、当たる種類と開発者の裁定を記録に残す

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す。直す対象を 1 本の spec の中で直し切り、件数を理由に 2 本目の spec へ分けない（2026-11-16 開発者の決め＝一度切り出した spec は規模を理由に後から分けない。brief の最後の行「20 タスクを越えそうなら 2 本目として切ってよい」はこの決めに置き換わる）
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・fixture・設定・文書を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・fixture・設定・スキル・手引き・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる（ほかのセッションが `.kiro/specs/` や `doc/ukadoc-coverage/` を同じ時に書き替えても、直しが新しい揺れを生まない）
3. Where テストが一時フォルダや副手続きを走らせる根を使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置き、ワークツリーの外の `doc/ukadoc-coverage/` や開発者の機械のスナップショットを書き替えない
4. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す（着手のときの範囲には GPU を使うテストは無い）
5. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない。整合テストが落とす条件を弱めて緑にしない）

### Requirement 5: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は記録とエラーの戻り値の両方で表し、副手続きは失敗を 0 でない終了コードと端末への文で知らせる）
3. The 本 spec の直し shall 終了の道を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 依存を足さない（C22 の約束 ⑸＝`Cargo.lock` に触らない）
6. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 6: 範囲の外へ直しが及ぶとき・C22 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C22 のあいだ, the 本 spec の直し shall 次のファイルに手を入れない: `crates/log-capture-kit/tests/file_length_guard_test.rs`（C22 の約束 ⑵＝`audit-test-kits` だけが触る）・`Cargo.lock`（約束 ⑸）・C22 のほかの 5 本が触るファイル（ロードマップの C22 の行の「触るファイル」①〜④⑥。`doc/ukadoc-coverage/ledger/assets.toml` を含む）
4. While C22 のあいだ, the 本 spec の直し shall 外への約束（要件 1.6 に挙げたもの）を変えない。特に、今の `doc/ukadoc-coverage/` の入力から副手続きが書き出すファイルの中身を 1 バイトも変えない（前提＝未決の問い 2）
5. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・C22 のほかの 5 本の着地を待って本 spec の中で直す・後の spec へ回す）を開発者の裁定で決める
6. When 裁定に従って外への約束を変えるとき, the 本 spec の直し shall 同じ変更の中で、それを前提にしている範囲の外のもの（Boundary Context の Adjacent expectations に挙げた手引き・スキル・steering・データ）を全部直し切る
7. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在することを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
8. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 7: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 6.5・6.7 で後へ回すと裁定された指摘は数えない。打ち切りは 7.6）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 6.3・6.4 の約束が守られている（6.3 のファイルに差分が無い・外への約束が変わっていない・今の入力から副手続きが書き出す中身が `main` の道具と同じ）ことを、既定のブランチとの差分と、副手続きの出力の突き合わせで確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と道具の振る舞い（副手続きの出力・整合テストの判定）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・audit-seriko と同じ形）
