# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・着手のときは 5.3.3＝audit の作法の `SKILL.md` は 5.3.2 と 1 字も違わない・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **先進坑の実験コード**（`crates/pilot/`）を受け持つ。まだ audit をかけていない。

着手のときの測り直し（2026-11-17・`main` `5749a4c7`・brief と同じく `_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）: `crates/pilot/` のうち `examples/pilot-windows-rs-next/` の外は、git に載ったファイルが 31（`.rs` 22・文書 8・`Cargo.toml` 1）＝コード 7,550 行・テスト 531 行で、起票のときと 1 行も変わっていない。1,000 行を越えているのは `examples/pilot-clickthrough-alpha-toggle/main.rs`（1,006 行・1,000 行の番人の例外表に載っている）、近いのは `examples/pilot-balloon-asset-swap/swap.rs`（928 行）と `observe.rs`（852 行）。11-17 に着地した `pilot-windows-rs-next`（PR#471）が `examples/pilot-windows-rs-next/` に本線から独立したワークスペース（104 ファイル）を足した。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Introduction

この spec は、先進坑の実験コードを置く `crates/pilot/`（空の `lib.rs` と `examples/` の下の先進坑たち＝`pilot-balloon-asset-swap`・`pilot-clickthrough-alpha-toggle`・`pilot-dropfiles-on-wuc-window`・`shiori-host-32`・`wintf-winmsg-executor`・`_template`）を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

`crates/pilot/` は先進坑の決まり（`.kiro/steering/two-tunnel.md`）に縛られる。出荷するクレートはどれも `pilot` に頼らない（葉の隔離）。そのため範囲の外に `pilot` の呼び出し元は無く、直しが範囲の外へ及ぶのは、`pilot` が頼る側のクレートや、`pilot` のファイルを名前で指す文書・テスト（1,000 行の番人の例外表ほか）に限られる見込みである。各先進坑の `README.md`・`REPORT.md` は、測った時のコードについての一次記録（知見）で、先進坑の成果物そのものである。

この spec はウェーブ C22 で、ほかの 5 本（`collisionex-regions`・`audit-placement-follow-residue`・`audit-emo2-boot-spine`・`audit-test-kits`・`audit-ukadoc-survey`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C22 の行の約束 ⑴〜⑸）を守る（要件 7）。

本文の【仮定】は、brief で決まらず要件の討議で開発者に確かめる点である。一覧は末尾の「要件の討議へ持ち越す点」に置いた。

## Boundary Context

- **In scope**:
  - `crates/pilot/` のうち git に載ったファイル全部に audit をかけて出た指摘の直し。【仮定 1】`examples/pilot-windows-rs-next/` の下は除く（要件 1.1）。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の読み手（文書・テスト・設定）の直し（要件 3.2 と要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。範囲の外で見つけた指摘は、その範囲の spec の持ち物として残す（例: `crates/sample-ghost-kit` は同じウェーブの `audit-test-kits` の範囲、`crates/wintf` は完了した wintf の見直しの範囲）。
  - 新しい機能（新しい先進坑・新しい実行例・新しい引数）。ただし、確定した指摘の直しとして、止まる道や記録を足すことは新しい機能に数えない。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件・steering の先進坑の決まり）を覆す直し。直さずに議題にする。
  - 【仮定 1】`examples/pilot-windows-rs-next/` の下（windows-rs の移行の本体が読む一次記録と、全体テストに乗らない独立したワークスペース）。
- **Adjacent expectations**:
  - 出荷するクレートは `pilot` に頼らない。この spec の直しも、その形を崩さない（要件 5.1）。
  - 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）は、`pilot-clickthrough-alpha-toggle/main.rs` を「今も 1,000 行を越えている」例外として名前で持つ。表の項目が越えなくなると番人が赤になる。同じウェーブでこの表と件数の行に触れるのは `audit-test-kits` だけである（要件 7.1〜7.3）。
  - `pilot` の dev 依存の `sample-ghost-kit` は同じウェーブの `audit-test-kits` の範囲にある。その公開の形はこの spec からは変えない（要件 7.4）。
  - windows-rs の移行の本体（まだ起票していない）は `examples/pilot-windows-rs-next/` の一次記録を読む。【仮定 1】のとおり外せば、この spec はそこに触らない。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `crates/pilot/` のうち git に載ったファイルだけを対象にして `/ponytail:ponytail-audit` をかける。【仮定 1】`examples/pilot-windows-rs-next/` の下は対象から外す
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）と、使った物差しの版を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When 指摘の直しが先進坑の一次記録（`README.md`・`REPORT.md` に書いた測った結果）と食い違いを生むとき, the ギャップ分析 shall その指摘にその旨と、食い違う記録の場所を書く（要件 5.3 の議題の材料）
7. When 指摘の直しが要件 7 の約束（1,000 行の番人の例外表・`main.rs` の分割・依存・同じウェーブの相手の範囲）に触れるとき, the ギャップ分析 shall その指摘にその旨を書く（要件 7 の議題の材料）
8. When 指摘を確かめるのに先進坑を実機の画面や実 GPU で動かす必要があるとき, the ギャップ分析 shall その指摘にその旨を書く
9. The ギャップ分析 shall 直したバグ・危ない所を落とすテストが全体テストで実際に走るか（ビルドされるだけで走らないか）を、範囲のテストごとに確かめて報告に書く（要件 4.3 の材料）

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
6. If 直す対象のタスクが 20 を越えそうなとき, then the 要件の討議 shall 指摘の組ごと（または `examples/pilot-windows-rs-next/` の下かどうかの境目）で、同じ範囲の 2 本目の spec へ切り分けることを議題にしてよい
7. 【仮定 2】The 本 spec shall 先進坑の品質の基準が緩くてよいこと（`two-tunnel.md`「品質基準」）を理由に指摘を残さず、整形・命名・テストの厳しさに関わる指摘も 3 つの組のとおりに直す（開発者がこの spec を起票した方針「とにかく指摘事項は全部直す」を先に立てる）

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・読み手・テスト・見本・設定・文書（ファイルを名前で指す 1,000 行の番人の表や手順書を含む）を、範囲の外も含めてツリー全体で探し、全部直し切る。ただし要件 7 の約束に触れるものは要件 7 に従う
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・見本・設定・workflow・スキル・手順書・steering・完了した spec の文書・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない
5. If 直しのために範囲の外のファイルの形を変える必要があるとき, then the 本 spec の直し shall そのファイルの持ち主の見直しの spec が走っていれば、そのセッションと申し合わせてから触る

### Requirement 4: 直したことを確かめる

**Objective:** As a 開発者, I want 直した後に、指摘が残っていないことと壊れていないことを確かめたい, so that 直しが新しい問題を生んでいないと言える

#### Acceptance Criteria

1. When 全部の直しが済んだとき, the 本 spec の検証 shall 同じ範囲（要件 1.1 で決めた範囲）へもう一度 `/ponytail:ponytail-audit` を同じ作法（全件・全部読む）でかけ、新しい指摘が 0 件であることを確かめる
2. If かけ直しで新しい指摘（直しが生んだものを含む）が出たとき, then the 本 spec の検証 shall その指摘も直し、もう一度かけ直す
3. The 本 spec shall 直したバグ・危ない所ごとに、それが壊れたら落ちる決定論のテストを 1 本ずつ持ち、そのテストが全体テストで実際に走る（ビルドされるだけでは足りない）
4. When 実装の最後, the 本 spec の検証 shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）を回して緑を確かめる
5. When 指摘を確かめるのに先進坑を実機の画面で動かすとき, the 本 spec の検証 shall 画面の操作を排他テストの机で順を取って行い、終わったらすぐ机を返す

### Requirement 5: 先進坑の決まりと一次記録を守る

**Objective:** As a 開発者, I want 直しの後も先進坑が先進坑のままでいてほしい, so that いつでも捨てられる形と、測った知見の正しさが残る

#### Acceptance Criteria

1. The 本 spec の直し shall 出荷するクレートが `pilot` に頼らない形（葉の隔離）と、`pilot` の `lib.rs` が空で先進坑のコードが `examples/` の下だけにある形を保つ
2. The 本 spec の直し shall 先進坑のコードを本坑（出荷するクレート）へ写さない
3. 【仮定 3】When 直しが、`README.md`・`REPORT.md` に書いた測った結果のもとになったコードを変えるとき, the 本 spec の直し shall 記録の測った結果は書き換えず、どの指摘の直しでコードが測った後に変わったかを日付つきで記録に書き足す
4. If 指摘が先進坑の実行例をまるごと消すことを求めるとき（ギャップ分析の 27 件には無い＝かけ直しで出たときの決まり）, then the 本 spec shall その先進坑の `README.md`・`REPORT.md`（知見）は残し、コードを消すかどうかを要件の討議で議題にする

### Requirement 6: 守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを破らないでほしい, so that 直しのせいでほかの所に負債が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行を越えさせない。足すなら新しいファイルへ足す（`swap.rs` 928 行・`observe.rs` 852 行は越えさせない）
2. The 本 spec の直し shall 決定論のテストで直しを網羅する
3. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は記録を残してから失敗として返す）
4. The 本 spec の直し shall 終了の道を正規の実装にし、小細工で済ませない
5. The 本 spec の直し shall 実機の根と一時フォルダをワークツリーの `target` の下に置く
6. The 本 spec の直し shall GPU のテストを実 GPU で回す
7. The 本 spec の直し shall 文字化けをコンソールの文字コードを変えて逃げず、読む側を直す

### Requirement 7: 同じウェーブの約束を守る

**Objective:** As a 開発者, I want 並んで走るほかの 5 本とぶつからずに着地してほしい, so that ウェーブ C22 の全部が手戻りなく合流できる

#### Acceptance Criteria

1. The 本 spec の直し shall `crates/log-capture-kit/tests/file_length_guard_test.rs` の上限越えの例外表と件数の行に触らない（同じウェーブで触れるのは `audit-test-kits` だけ・C22 の約束 ⑵）
2. The 本 spec の直し shall `examples/pilot-clickthrough-alpha-toggle/main.rs`（1,006 行・例外表に載っている）を割らない。割ることを求める指摘は、次のウェーブへ申し送る（C22 の約束 ⑵）
3. 【仮定 5】If 直しで `pilot-clickthrough-alpha-toggle/main.rs` が 1,000 行以下になる見込みが出たとき, then the 本 spec shall `audit-test-kits` のセッションと申し合わせて例外表の項目を外してもらうか、行数の減る直しを次のウェーブへ申し送るかを議題にする（表の項目が越えなくなると番人が赤になるため）
4. The 本 spec の直し shall 同じウェーブのほかの spec の範囲のファイル（`crates/sample-ghost-kit` ほか）の公開の形を変えない
5. The 本 spec の直し shall 依存を足さない（`Cargo.lock` の席は空いているが、C22 の約束 ⑸ で依存は足さない）。どうしても要るときは議題にし、足すなら `tech.md` へ登記して開発者の承認を得る
6. 【仮定 6】The 本 spec shall 要件 7.2・7.3 で次のウェーブへ申し送った直しを、直さずに残す 3 種類（要件 2.2）とは別の「申し送り」として記録し、受け取る spec の brief か次の棚卸へ渡す

## 要件の討議へ持ち越す点

brief で決まらず、要件の討議で開発者に確かめる点。どれも本文には推す案を【仮定】として書いた。

1. **`examples/pilot-windows-rs-next/` を範囲から外すか**（要件 1.1・Boundary Context）。【仮定 1】外す＝範囲を「`crates/pilot/` の全部。ただし `examples/pilot-windows-rs-next/` を除く」とする。理由（brief 2026-11-17 の再測定）: ⑴ 本線の member でなく全体テストに乗らない＝「全体テストが緑」で直しを確かめられない ⑵ git の依存を引くので、確かめるたびにその rev を取りに行く ⑶ `README.md`・`REWRITE_TABLE.md`・`UPSTREAM_DRAFT.md`・証跡は、まだ起票していない移行の本体（6〜8 本）が読む一次記録で、直すと測った時のコードと記録がずれる ⑷ 先進坑は使い捨て。入れる場合は指摘が 20 を越えやすく、越えたらこの置き場を同じ範囲の 2 本目に切る（要件 2.6）。
2. **先進坑の「品質の基準は緩くてよい」と「指摘は全部直す」のどちらを先に立てるか**（要件 2.7）。【仮定 2】全部直す（この spec は開発者が見直しの方針で起票した・緩めてよいのは品質で、葉の隔離は崩さない）。
3. **直しが一次記録の測った結果とずれるとき**（要件 5.3）。【仮定 3】測った結果は書き換えず、直しでコードが変わったことを日付と指摘の番号つきで記録に書き足す。
4. ~~**先進坑をまるごと消す指摘が出たとき**（要件 5.4）~~ — ギャップ分析の 27 件に当たるものが無いので議題から外した（要件 5.4 は、かけ直しで出たときの決まりとして残す）。
5. **`pilot-clickthrough-alpha-toggle/main.rs` の行数が減る直し**（要件 7.3）。【仮定 5】1,000 行以下になる見込みなら、`audit-test-kits` のセッションと申し合わせて表から外してもらう。申し合わせがつかなければ、その直しを次のウェーブへ申し送る。
6. **次のウェーブへ申し送る直しの扱い**（要件 7.2・7.6）。brief の「直さずに残してよいのは 3 つだけ」と、ウェーブの約束による申し送りが重なる。【仮定 6】申し送りは 3 種類とは別の記録として残し、受け取る spec の brief か次の棚卸へ渡す（直さないのではなく、直す時期を次のウェーブへ移す）。
