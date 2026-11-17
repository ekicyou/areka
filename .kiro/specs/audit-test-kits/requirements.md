# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **試験の道具のクレート（実走・記録の捕まえ・検体・一時フォルダ）** を受け持つ（受け持つファイル＝`crates/real-run-kit/`・`crates/log-capture-kit/`・`crates/sample-ghost-kit/`・`crates/temp-path-kit/` の全部。まだ audit をかけていない）。

着手のときの測り直し（2026-11-17・`main` `5749a4c7`・brief と同じく `_tests.rs`・`tests/`・`_test_support.rs` をテストに数え、`src/bin/` と `examples/` はコードに数えた）: 62 ファイル（`.rs` 58 本・`Cargo.toml` 4 本）＝コード 5,707 行・テスト 11,129 行（`.rs` の計 16,836 行は棚卸㊴の再測定と一致）。内訳は `real-run-kit` 20 ファイル（コード 1,922・テスト 1,661）・`log-capture-kit` 21 ファイル（コード 1,112・テスト 5,537）・`sample-ghost-kit` 18 ファイル（コード 2,520・テスト 3,753）・`temp-path-kit` 3 ファイル（コード 153・テスト 178）。1,000 行に近いのは `crates/log-capture-kit/tests/sample_path_guard_test.rs` 926 行。物差しの版は、いま入っている版をギャップ分析の報告に書く。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。あわせて、brief に預かった 2 件（件 1＝記録の捕まえの手助け `expect_one`・`ExpectField` の写しを 2 か所から消す・件 2＝`crates/log-capture-kit/src/lib.rs` の「消費する N crate」の数の文）を片づける。

## Introduction

この spec は、試験の道具の 4 つのクレートを ponytail の audit で見直し、出た指摘を全部直すためのものである。4 つは、本物の areka を子として立てて MCP で話す実機の試験の窓口 `real-run-kit`、記録の捕まえの硬化とワークスペース全体の見張り（1 ファイル 1,000 行の番人・一時パスと検体の窓口の迂回の検知・`with_default` の迂回の検知・文書の木の番人）を持つ `log-capture-kit`、検体のゴーストとバルーンを名前で引く窓口 `sample-ghost-kit`、テスト用の一時パスの窓口 `temp-path-kit` である。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

4 つのクレートはどれもテスト専用（`[dev-dependencies]` からだけ引く）で、areka の実行時のコードには入らない。代わりに、範囲の外の多くのクレートのテストがこの範囲に頼る（2026-11-17 の数え: `log-capture-kit` を 21 crate・`temp-path-kit` を 12 crate・`sample-ghost-kit` を 15 crate〔`real-run-kit` と `log-capture-kit` を含む〕・`real-run-kit` を `areka` が引く）。開発の道具（`tools/sample-fetch.ps1`・`tools/package.ps1`・`tools/load-flake.ps1`・`tools/encoding-check.ps1`・`tools/perf/` の下）、手順書（`doc/testing.md`・`doc/pasta-real-run.md` ほか）、steering（`structure.md`・`tech.md`・`logging.md`・`product.md`）、検体の顔ぶれの正本 `vendors/sample_ghost/README.md` も、この範囲の名前と置き場を書いている。そのため、範囲の外から見た「公開の形」は、各クレートの公開の関数・型・機能（`env-filter`）の名前と振る舞い、実行体（`real-run-fake-areka`・`nar-sample-path`〔引数 `--fetch-list` と終了コード 3 を含む〕）と見本（`fold-samples`）の名前・引数・終了コード・出す行、読む環境変数の名前、置き場の名前（`target\test-roots\`・`<target>\nar-samples\fetched\`・`manual/<検体>/` ほか）、見張りが赤にする決まりとその例外表である（以下「外への約束」）。

この spec はウェーブ C22 で、ほかの 5 本（`collisionex-regions`・`audit-placement-follow-residue`・`audit-emo2-boot-spine`・`audit-ukadoc-survey`・`audit-pilot`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C22 の行の約束 ⑴〜⑸）を守る（要件 7）。

前提の仮置き（要件の討議で確かめる・末尾の「要件の討議への問い」）は、本文の中に「仮置き」と書いた。

## Boundary Context

- **In scope**:
  - `crates/real-run-kit/`・`crates/log-capture-kit/`・`crates/sample-ghost-kit/`・`crates/temp-path-kit/` の全部（`src/`・`src/bin/`・`examples/`・`tests/`・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - brief に預かった件 1（`crates/areka-emo-present/src/scale_tests.rs` と `crates/areka-emo-text/tests/attach_wiring_test.rs` の `expect_one`・`ExpectField` の写しを消して道具の側を使う）と件 2（`crates/log-capture-kit/src/lib.rs` の消費する crate の数の文）。
  - 直しのために要る、範囲の外の呼び出し元と読み手の直し（要件 3.2 と要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。範囲の外で見つけた指摘は、その範囲の spec の持ち物として残す。
  - 新しい機能（新しい窓口・新しい見張り・新しい実行体や引数）。ただし、確定した指摘の直しとして、黙って通る道を止める確かめや後始末を足すことは新しい機能に数えない。
  - `vendors/` の下（`vendors/sample_ghost/` の検体と `README.md` を含む）。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。見張りが赤にする決まり（1 ファイル 1,000 行・一時パスはワークツリーの `target` の下・検体と一時パスは窓口を通す・記録の捕まえは窓口を通す・試験の道具は `[dependencies]` に置かない）は裁定に当たり、緩めない。
  - `input-crate-split` が `input_events/` を移したときに古くなる `crates/log-capture-kit/src/event.rs`・`event_tests.rs` の説明のパス（`input-crate-split` の brief 96 行が持つ）と、`session-module-gather` が移したときに古くなる `real-run-kit` の出所の名指し（`areka::app_exit` ほか・同 brief 48・64・100 行が持つ）の先回りの書き換え。
- **Adjacent expectations**:
  - 範囲の外の各クレートのテスト（上の数え）は、公開の関数・型の名前と振る舞い（捕まえの窓が対照の記録を要求して「出ていない」の主張を 0 件のまま緑にしないこと・一時パスが一意で破棄のときに消えること・検体が名前で引けること・実機の試験が落ちた回の証拠を残すこと）が今のまま続くことを前提にしている。
  - 開発の道具 `tools/sample-fetch.ps1` は、`nar-sample-path --fetch-list` が出す一覧の形と、取り寄せの置き場 `<target>\nar-samples\fetched\<名>.nar`・`<名>.nar.sha256` の名前が今のまま続くことを前提にしている。全体テストの `sample fetch` の段と、ほかの道具（`package.ps1`・`load-flake.ps1`・`encoding-check.ps1`・`tools/perf/`）も、この範囲の名前や置き場を名指ししている。
  - ワークスペースの全部の spec は、`log-capture-kit/tests/` の見張りが今の決まりで赤にし続けることを前提にしている。見張りの例外表（`file_length_guard_test.rs` の上限越えの表と件数の定数 `OVER_LIMIT_ALLOWED_COUNT` ほか）は、表に載ったファイルの持ち主の spec と分け合う。
  - 同じウェーブの 5 本は、この spec が `crates/{real-run-kit,log-capture-kit,sample-ghost-kit,temp-path-kit}/` だけを触り、`file_length_guard_test.rs` の表と数の行を C22 で触るのはこの spec だけであることを前提にしている（C22 の約束 ⑵）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `crates/real-run-kit/`・`crates/log-capture-kit/`・`crates/sample-ghost-kit/`・`crates/temp-path-kit/` の全部だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）と、使った物差しの版を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘を直すのに外への約束を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの読み手（呼び出し元のクレート・開発の道具・手順書・steering・スキル）に響くか、その読み手が C22 のほかの 5 本の触るファイルに当たるかを書く（要件 7.4・7.7 の議題の材料）
7. When audit の指摘を確かめるのに要件 8.1 の操作が要るとき, the ギャップ分析 shall その指摘にその旨を書く（要件 8.3 の議題の材料）
8. The ギャップ分析 shall 件 1 と件 2 について、要件 5 を満たすのに要る事実（件 1 の 2 か所の写しと道具の側の振る舞いの違いの有無、件 2 の今の文と実際の数）を測って報告に書く

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
6. If 直す対象のタスクが 20 を越えそうなとき, then the 要件の討議 shall 指摘の組ごとに、同じ範囲の 2 本目の spec へ切り分けることを議題にしてよい（仮置き: brief の見込みは 20 未満＝1 本のまま）

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・読み手・テスト・見本・設定・文書を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・見本・設定・開発の道具の台本・スキル・エージェント・手順書・steering・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない（確定した指摘の直しとして、黙って通る道を止める確かめや後始末を足すことは新しい機能に数えない）

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 全体テストの中で回る（常時の `cargo test` で回るか、全体テストの既にある段で回る）
3. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる
4. The 本 spec のテスト shall ネット・昇格・画面の操作・静かな机・本物の areka の実機の走り・本物の SSP に頼らずに回る（既にある段 `sample fetch` の取り寄せと、偽の子 `real-run-fake-areka` を使う既にある形の試験を除く）
5. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
6. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
7. When 見張り（`log-capture-kit/tests/` の下）の判定を直すとき, the 本 spec の直し shall その見張りが違反を見つけて赤にすることを示す較正を保ち、較正の無い 0 件で緑にしない
8. When 既存のテストや較正が直しで古びたとき, the 本 spec の直し shall それを直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: brief に預かった 2 件を片づける

**Objective:** As a 開発者, I want 前の棚卸や完了から預かった 2 件が、持ち主のこの spec で片づいてほしい, so that 持ち主の無い宿題が残らない

#### Acceptance Criteria

1. The 本 spec の直し shall `crates/areka-emo-present/src/scale_tests.rs`（`expect_one`）と `crates/areka-emo-text/tests/attach_wiring_test.rs`（`expect_one`・`ExpectField`）が自前で持つ写しを消し、`log-capture-kit` の側のもの（`src/expect.rs`）を使わせる（件 1。どちらのクレートも `log-capture-kit` を開発用の依存に既に持ち、C22 のほかの 5 本の触るファイルに当たらない）
2. When 件 1 の写しを消すとき, the 本 spec の直し shall 呼ぶ側のテストの主張の強さ（ちょうど 1 件であること・欠けた項目を失敗にすること・失敗の文に何を探したかが出ること）を弱めない
3. The `crates/log-capture-kit/src/lib.rs` の説明 shall 実際の消費 crate の数と食い違う数を書かない（件 2。仮置き: 数を書かない文にする＝crate が増えるたびに古びる手書きの数を残さない。2026-11-17 の `main` `5749a4c7` では「21 crate」で実際の 21 と合っている）
4. The 本 spec shall 件 1・件 2 を片づけたことを、預かった brief の件の名前で記録に残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（本番の形の失敗は理由を書いた記録と失敗の戻り値で表す。テストの道具が試験を落とすときは理由を書いた panic の文で表す〔仮置き〕。どちらも失敗を黙って成功にしない）
3. The 本 spec の直し shall 終了の道（立てた子のプロセスの終わらせ方・取っ手の破棄の順・途中で落ちたときの後始末と証拠の残し方を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 道具が端末へ出す文を ASCII だけに保ち、出す量を増やすときも呼ぶセッションのトークンの負荷を抑える形（数行）にする
6. The 本 spec の直し shall 4 つのクレートをテスト専用のまま保つ（どれもほかのクレートの `[dependencies]` に現れない・`log-capture-kit` と `temp-path-kit` は自分の `[dependencies]` にワークスペースの中のクレートを持たない＝leaf のまま。`log-capture-kit` の `[dev-dependencies]` の `sample-ghost-kit` は今の設計のまま）
7. The 本 spec の直し shall 依存を足さない（C22 の約束 ⑸。外のクレート・PowerShell のモジュール・外の道具を新しく要るようにすることを含む）
8. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 7: 範囲の外へ直しが及ぶとき・C22 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C22 のあいだ, the 本 spec の直し shall C22 のほかの 5 本が触るファイル（ロードマップの C22 の行の「触るファイル」の ①②③⑤⑥。`crates/areka-runtime/src/` の本番と本体の `runtime_tests/`・`session/`・`crates/areka-placement/src/` の追従と移り変わり・`crates/areka-parsers/src/shell/`・`crates/areka-emo-compose/src/{hit,hit_import}.rs`・`crates/areka-emo-present/src/presenter_test_support.rs`・`presenter/hit.rs`・`crates/areka-emo-text/src/{actor,actor_test_support}.rs`・`crates/ukadoc-survey/`・`crates/pilot/` を含む）に手を入れない（例外: steering と `doc/COMPAT_ARCHITECTURE.md` §8 の末尾の追記は、重ならない自分の行だけを書く）
4. If 直しが C22 のほかの 5 本の触るファイルに及ぶとき, then the 本 spec shall 外への約束を変えない直し方を先に探し、それでも及ぶなら、そのファイルを触らずに相手のセッションへ申し送るか、相手と申し合わせて触り、どちらにしたかと相手を記録に残す
5. While C22 のあいだ, the 本 spec shall `crates/log-capture-kit/tests/file_length_guard_test.rs` の上限越えの表と件数の定数をこのウェーブで触る唯一の spec として扱い、表に載るファイルの持ち主（`audit-pilot`・`audit-placement-follow-residue`・`audit-emo2-boot-frame`）のファイルを割らない（C22 の約束 ⑵）
6. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める
7. When 裁定に従って外への約束を変えるとき, the 本 spec の直し shall 同じ変更の中で、それを前提にしている範囲の外の読み手（Boundary Context の Adjacent expectations に挙げたもの）を全部直し切る（7.3・7.4 の約束の中で）
8. If 直しが pasta のリポジトリと同じ文で持つスキルの文を変えるとき, then the 本 spec shall 変えた文と理由を記録に残し、pasta の側で同じ文にそろえる件として「ぱすた開発」のセッションへ申し送る
9. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在することを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
10. The 本 spec の直し shall `vendors/` の下に手を入れない
11. The 本 spec の直し shall emo-text に新しいソースファイルを足さない（C22 の約束 ⑸）

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、重い走行・共有の机を変える操作・画面の操作・外の実物を動かす操作が走らないでほしい, so that ほかの仕事の負荷も共有の机も汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、次の操作を回さない: 本物の areka を起こす実機の試験（環境変数で明示したときだけ回る `emo2_real_run` ほか）・負荷の走行（`tools/load-flake.ps1` の本番の走り・`areka-test-binary-stability` の走行）・本物の SSP やその MCP の操作・画面の操作・開発者の机の本物の置き場への書き込み・`vendors/sample_ghost/` の検体の書き替え・全体テストの外での検体の取り寄せ
2. Where 操作が全体テストの中で既に回っているもの（`sample fetch` の取り寄せ・偽の子を使う `real-run-kit` の試験ほか）であるところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
3. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の子 `real-run-fake-areka`・偽の置き場・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にし、扱いを開発者の裁定で決める（仮置き: 決定論のテストで筋まで縛り、効き目は次に本当にその操作をしたときに確かめるものとして、検証の記録に指摘の番号ごとに書き残す＝`audit-tools-scripts`・`audit-tools-perf-run` の裁定と同じ形）
4. If 開発者の裁定で 8.1 の操作を回すとき, then the 本 spec の作業 shall 排他テストの机を借り、1 本ずつ、回す回数と所要時間の見込みを先に示してから回し、終わったらすぐ机を返す

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 7.6・7.9 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 7.3・7.5・7.10 の約束が守られている（該当のファイルに差分が無い・上限越えの表に載るファイルを割っていない）ことと、外への約束の変わった所が要件 7.6 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と道具の振る舞い（公開の関数・型の振る舞い・見張りの判定・実行体の終了コードと出す行・書き出すファイルと置き場）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・audit-seriko と同じ形）

## 要件の討議への問い（仮置き）

要件の段では開発者に問わず、次の点を仮置きにした。要件の討議で確かめる。

1. **件 2 の文の形（要件 5.3）**: 仮置き＝「消費する N crate」の数を書かない文にする。`main` `5749a4c7` では棚卸㊴（PR#473）が「21」へ数え直してあり、実際の 21 と合っているが、C23 の `input-crate-split` がワークスペースにクレートを足すとまた古びうる。数を残すなら、数え直すだけで閉じる。
2. **テストの道具の失敗の表し方（要件 6.2）**: 仮置き＝テスト専用のクレートが試験を落とす道（`real-run-kit` の「試験は `Result` でなく panic で落とす」の設計）は、理由を書いた panic の文をもって「記録のある失敗」と読む。決まり「`error!`＋`Err`」を本番の形の失敗に当てる。
3. **`vendors/sample_ghost/README.md`（Boundary の Out）**: 仮置き＝brief の Out「`vendors/`」に従い触らない。検体の登記の手順や置き場の名前を変える指摘が出て、この README の文が古びるときは、要件の討議で議題にする（`vendors/sample_ghost/` は外のリポジトリでなくこのリポジトリの中に在る）。
4. **steering の古い記述**: `structure.md` の Temp Path Kit 節の Consumers は 9 crate を挙げるが、2026-11-17 の数えでは 12（`areka-placement`・`areka-runtime`・`surface-compare` が無い）。仮置き＝audit が範囲の説明の古びとして指したときに、steering の自分の行として直す（steering は同じウェーブで許す重なり）。指さなかったときは完了の棚卸で拾う。
5. **実機でしか効き目を確かめられない直し（要件 8.3）**: 仮置き＝`real-run-kit` の起動・閉じ方・証拠の直しは、偽の子 `real-run-fake-areka` を使う決定論のテストで筋まで縛り、効き目は次の本当の実機の試験で確かめるものとして番号ごとに記録する（先例 `audit-tools-scripts` の議題 5 と同じ形）。
6. **分け方（要件 2.6）**: 仮置き＝1 本のまま（brief の見込みは 20 未満）。ギャップ分析の指摘の数で見立てが 20 を越えたら議題にする。
