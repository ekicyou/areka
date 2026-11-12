# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **MCP の受け口と中継・ネットワーク更新のクレート** を受け持つ（受け持つファイル＝`crates/areka-mcp/`・`crates/areka-mcp-bridge/`・`crates/areka-update/` 全部。2026-11-13 棚卸㊱の測り直し〔main `069f1a48`〕で 100 ファイル＝`areka-mcp` 44・`areka-mcp-bridge` 26・`areka-update` 30、空行を除いてコード 5,808 行・テスト 12,390 行。900 行を越えるファイルは無い。指摘はギャップ分析で audit をかけて洗い出す）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件。全体テストが緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Introduction

この spec は、MCP の受け口（`areka-mcp`）・MCP の中継（`areka-mcp-bridge`）・ネットワーク更新のエンジン（`areka-update`）の 3 つのクレートを、ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

ただ 1 件だけ、audit を待たずに直す対象が決まっている。完了した `audit-ghost-catalog` から申し送られた件 1（空・相対の `AREKA_PROFILE_DIR` で、中継の控えの置き場が作業フォルダからの相対になる）である（要件 3.5）。

この spec はウェーブ C19 で、ほかの 7 本（`wintf-d2d-composite-rules`・`dola-yaml-backend`・`runtime-module-untangle`・`audit-shiori-host32`・`audit-emo-atlas`・`emo-text-reveal-incremental`・`audit-impl-watch`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C19 の行）を守る（要件 6）。本体 `crates/areka` は `areka_mcp`・`areka_update` を `mcp/`（約 30 ファイル）・`update/`・`cli/listen.rs`・`install/fetch_url.rs`・`main.rs`・`log_history.rs` から使い、同じウェーブの `runtime-module-untangle` がそのうち `mcp/` と `update/` を書き換える。そのため、C19 のあいだは 3 つのクレートの公開の形を変えず、足すだけにし、本体には触らない。

## Boundary Context

- **In scope**:
  - `crates/areka-mcp/`・`crates/areka-mcp-bridge/`・`crates/areka-update/` の全ファイル（本番のコード・テスト・テストの道具・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - `audit-ghost-catalog` から申し送られた件 1 の直し（要件 3.5）。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る範囲の外の最小の変更（要件 6 の約束の中で）。例: 件 1 で `crates/areka-user-data/src/lib.rs` の判定の関数を範囲の外から呼べるようにする 1 行（持ち主の `audit-ghost-catalog` は完了している）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。範囲の外で見つけた指摘も、その範囲の spec の持ち物として残す（例: 本体の `crates/areka/src/mcp/`・`update/` は `audit-areka-install-update` の持ち物）。
  - 新しい機能。
  - 依存の版上げ（`rmcp` ほかの版を上げるのは `/update` の仕事）。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
  - v0.2.0 で受け持つクレートに手を入れる spec（`mcp-set-property` ほか）の仕事。それらはこの spec の着地を待つ。この spec は先取りしない。
- **Adjacent expectations**:
  - 受け持つクレートを呼ぶ範囲の外のコード（本体 `crates/areka` ほか）は、ほかの見直しの spec と本体の crate 分けの spec の持ち物である。C19 のあいだ、この spec はそれらを書き換えずに済む形でだけ直す（要件 6）。
  - C19 で並ぶほかの 7 本とは、受け持つファイルが重ならない（棚卸㊱の測り）。
  - `crates/areka/src/log_history_convention_tests.rs` は `areka-update` の `src` を文字として読み、ログの出どころ（`areka_update::…` の名）を確かめている。この spec はそのテストを書き換えずに済む形でだけ、ログの出し方を直す（要件 6.3）。
  - SSP の MCP と同じ道具の形（MCP の受け口）と、ukadoc のネットワーク更新の正典（更新のエンジン）は外から見た約束である。それに関わる振る舞いを変える直しは、ukadoc で確かめてから議題にする（要件 2.3）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall 受け持つ 3 つのクレートだけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数を測り直す
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（範囲を分けて読んだ結果をまとめた後の、3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. The ギャップ分析 shall 各指摘について、直すと 3 つのクレートの公開の形（範囲の外から使われている名前・型・関数の形）が変わるか、範囲の外のファイルに手が及ぶかを報告に書く
7. The ギャップ分析 shall `audit-ghost-catalog` から申し送られた件 1 を、audit が出したかどうかにかかわらず、指摘の一覧に入れる

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 要件の討議 shall ギャップ分析の指摘を全件、3 つの組とも直す対象として要件に落とす
2. The 本 spec shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る
   - ⑴ 開発者の裁定に反する直し
   - ⑵ わざと置いた先送りの印（実在の理由を書いた説明つきで使われていないことを許している語彙の口、および `shortcut:`・`ponytail:` の注釈で、書いた限界が今の負荷を越えていないもの）
   - ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの（MCP の道具の形・ネットワーク更新の手順とイベントほか）
   - 要件 6.4・6.5 で後の spec へ回すと裁定された指摘は、直さずに残すものには当たらない（回し先の spec が直す）
3. If 指摘が ⑶ に当たるかもしれないとき, then the 要件の討議 shall ukadoc で正典を確かめた結果（出どころと引用）を添えて議題にする
4. If 指摘の直しが開発者の裁定を覆すとき, then the 本 spec shall その指摘を直さず、議題にする
5. The 本 spec shall 直さずに残すと裁定された指摘ごとに、当たる種類と開発者の裁定を記録に残す
6. If 直す対象のタスクが 20 を越えそうなとき, then the 要件の討議 shall 指摘の組ごとに、同じ範囲の 2 本目の spec へ切り分けることを議題にしてよい

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・fixture・設定を、範囲の外も含めてツリー全体で探し、全部直し切る（範囲の外へ及ぶときは要件 6 の約束に従う）
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない
5. If 環境変数 `AREKA_PROFILE_DIR` の値が空か相対のパスのとき, then the MCP の中継 shall その値を控えの置き場に使わず、areka 本体と同じ決まり（`error!` を記録して捨て、値が無いときと同じ次の決めへ進む）で置き場を決める（`audit-ghost-catalog` からの件 1）

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる（外のネットワークに出ない）
3. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
4. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
5. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録とエラーの戻り値の両方で表す）
3. The 本 spec の直し shall 終了の道を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 依存を足さず、依存の版も変えない（C19 では `Cargo.lock` に触ってよいのは `dola-yaml-backend` だけ）
6. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 6: 範囲の外へ直しが及ぶとき（C19 の並走の約束）

**Objective:** As a 開発者, I want 範囲の外への直しが、同じウェーブで並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While ウェーブ C19 が走っているあいだ, the 本 spec の直し shall `areka-mcp`・`areka-mcp-bridge`・`areka-update` の公開の形（範囲の外から使われている名前・型・関数の形）を変えず、消さない（足すだけはよい）
2. While ウェーブ C19 が走っているあいだ, the 本 spec の直し shall `crates/areka/` と、同じウェーブのほかの 7 本が触るファイル（ロードマップの「ウェーブ編成」の C19 の行に挙げたもの）に手を入れない
3. The 本 spec の直し shall `crates/areka/src/log_history_convention_tests.rs` が `areka-update` の `src` から読んで確かめているログの出どころ（`areka_update::…` の名）を、そのテストが書き換えなしで緑のまま通る形に保つ
4. If 確定した指摘が 6.1〜6.3 の約束を破らないと直せないとき, then the 本 spec shall 約束を破って直さず、要件の討議でその指摘を議題にし、扱い（約束の中で直す形・`runtime-module-untangle` ほかのセッションとの申し合わせ・後のウェーブで直す受け持ちの spec）を開発者の裁定で決める
5. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在することを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
6. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
7. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない（件 1 の `crates/areka-user-data/src/lib.rs` は、判定の関数を範囲の外から呼べるようにする変更か、同じ判定を写すかのどちらか＝設計で決める）
8. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 7: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 6.4・6.5 で後の spec へ回すと裁定された指摘は数えない）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 6.1〜6.3・5.5 の約束が守られている（3 つのクレートの公開の形が消えも変わりもしていない・`crates/areka/` ほか 6.2 のファイルに差分が無い・`Cargo.lock` に差分が無い・6.3 のテストが書き換えなしで緑）ことを、既定のブランチとの差分で確かめて記録に残す
