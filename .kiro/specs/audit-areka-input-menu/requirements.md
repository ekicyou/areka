# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・2026-11-21 に入っている版は 5.3.3・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **areka の入力の出来事**（2026-11-21 `input-crate-split` の後の置き場の `crates/areka-input/`）を受け持つ。右クリックメニュー（本体の `menu/`）は 2026-11-01 の開発者の決めで範囲から外した（windows-reactor が来れば作り直すので、見直しても捨てることになる）。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。あわせて、brief に預かった件（`collisionex-regions`・`input-crate-split`・`audit-areka-app` の brief からの申し送り）を片づける。

## Introduction

この spec は、ポインタ・ホイール・クリック・ドラッグの入力の出来事を SHIORI のイベントへ運ぶ層（キャラクターの窓とバルーンの窓のポインタの受け手・当たりの領域・カーソルの選び方・選択肢の確定・ユーザーの中断・間引き・ドラッグ）を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議でこの文書に落として直す対象を確定する。

受け持つ範囲は、areka を使う人から見た振る舞い（マウスの移動・クリック・ダブルクリック・ホイール・ドラッグで SHIORI へ送るイベントとその中身、カーソルの形、選択肢を押したときの確定、台本の中断）を決める。そのため、範囲の外から見た「外への約束」は次のとおりとする（以下「外への約束」）。

- SHIORI へ送るマウスのイベントとその参照（ukadoc が正典・SSP 互換）。
- `areka-input` の公開の面のうち、本体（`crates/areka/`）とその試験が使う項目。
- ログの行の語と出所（`areka_input::…`・実機の確かめと手順書が字面で読む）。

受け持つ範囲の大きさ（brief の棚卸㊶の再測定・`main` `31ac3eaf`。着手のときの `main` `f6405e09` は、その後にドキュメントだけが変わった。`wc -l` で数えて合計 15,163 行で一致を確かめた）:

| 測った時点 | 範囲 | `.rs` のファイル | 本番 | テスト |
|---|---|---|---|---|
| 起票のとき（`e3c2cbe4`・brief の「Current State」の値） | 本体の `input_events/`・`menu*`・`file_drop*`・`app_exit_escape_tests.rs` | 60 | 6,072 行 | 12,976 行 |
| 着手のとき（`31ac3eaf`／`f6405e09`） | `crates/areka-input/` | 49 | 19 ファイル・4,851 行 | 30 ファイル・10,312 行 |

テストに数えたのは、名前が `_tests.rs` で終わるファイルと、名前が `_test_support.rs` で終わるファイル（起票のときと同じ数え方）。ほかに `Cargo.toml`（46 行）と `cursor/`（`README.md` 75 行と、カーソルの絵 4 つ＝`hand.cur`・`grip.cur`・`hand.svg`・`grip.svg`）が範囲に入る。1,000 行に近いのは `balloon_pointer_handler_tests.rs`（874 行）・`user_break_tests.rs`（853 行）・`shell_box_handler_tests.rs`（719 行）・`input_events_tests.rs`（699 行）。範囲の中に、1 ファイル 1,000 行の番人の上限越えの表（`crates/log-capture-kit/tests/file_length_guard_test.rs`・範囲の外）に載るファイルは 0 本。数え直しはギャップ分析の報告でもう一度行う（要件 1.6）。

この spec はウェーブ C24 で、ほかの 3 本（`window-resetwindowpos-hold`・`session-module-gather`・`test-crash-report-merge`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C24 の行の約束 ⑴〜⑹。この spec が直接守るのは ⑶ と ⑹）を守る（要件 7）。

## Boundary Context

- **In scope**:
  - `crates/areka-input/` の全部（`src/` の 49 ファイル・`Cargo.toml`・`cursor/`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - brief に預かった件と、ほかの brief からこの spec へ寄せられた件（要件 5）。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 7 の約束に従う）。
- **Out of scope**:
  - 本体（`crates/areka/`）のファイル全部。名前や中身が入力に関わっても、次のファイルは範囲の外（C24 の約束 ⑶）:
    - `crates/areka/src/file_drop.rs`・`file_drop_*tests.rs`・`app_exit_escape_tests.rs`（棚卸㊶で `audit-areka-app` の範囲へ寄せた）。
    - `crates/areka/src/menu/` の全部（`menu/shortcut.rs`・`shortcut_tests.rs` を含む）。2026-11-01 の開発者の決め（右クリックメニューを見直しの範囲から外す）による。見直すかどうかは開発者が決める（OPEN QUESTION 2）。
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 2）。
- **Adjacent expectations**:
  - 本体は `use areka_input as input_events;` で入力の crate を出し直して使っている（`structure.md` の「Input Crate」）。同じウェーブの `session-module-gather` が、この出し直しを外して本体の 26 の呼び手を `areka_input::…` へ書き換える。この spec は、その呼び手が使う項目（`MouseWiring`・`user_break::drain_no_user_break_signals`〔本体の `runtime_tests/frame_shell_box_arrow_tests.rs`〕・`attach_*`・`register_*` ほか）の形を変えない（要件 7.3）。
  - `areka-input` は `areka-runtime` の `crash_report_test_support`（落ちの報告の据え付け）を `src/crash_report_tests.rs` から使う。同じウェーブの `test-crash-report-merge` はこの落ちの報告の写し 3 本をまとめる spec で、その brief は「`areka-input` は落ちの報告を使わない」と書いている。事実と違うので、要件の討議で扱いを決める（OPEN QUESTION 1）。
  - `areka-runtime`・`areka-placement` は、`crates/areka-runtime/examples/collision-probe.rs`・`collision-probe/setup.rs` の説明の文を直すほかは触らない（要件 5.6・C24 の約束 ⑶）。
  - v0.2.0 の `collisionex-regions` の後継と `instant-to-interrupt-ticks`・`dola-interrupt-ticks` は、この見直しと触るファイルが重なり、この見直しの着地を待つ（roadmap の spec 台帳）。この spec の側に約束は無い。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `crates/areka-input/` の全部（`src/`・`Cargo.toml`・`cursor/`）だけを対象にして `/ponytail:ponytail-audit` をかける
2. The ギャップ分析 shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The ギャップ分析 shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない（`cursor/` の絵は中身を読まず、置き場・名前・使われ方と `README.md` の記述との食い違いを見る＝仮定 3）
4. The ギャップ分析 shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. The ギャップ分析 shall 要件 5 の預かった件のうち、audit の指摘と重なるものを、どの番号と重なるかを書いて報告に残す
6. The ギャップ分析 shall 着手のときの `main` で範囲のファイル数と行数（本番・テストの別）を数え直し、上の表と食い違えば報告に書く
7. The ギャップ分析 shall brief と本文に書いた行番号を今の `main` に当て直し、動いていれば報告に新しい行番号を書く

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 直す対象と、直さずに残す指摘を自分で確定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. When 要件の討議で直す対象が確定したとき, the 本 spec shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか回すか）をこの文書に書き加える
2. If 指摘の直しが次の 3 種類のどれかに当たるとき, then the 本 spec shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの（SHIORI へ送るマウスのイベントの名・参照・送る条件を含む）
3. When 3 種類目に当たるかを判断するとき, the 本 spec shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする。ukadoc で読めない点は、開発者の許しを得て SSP の MCP で測ってから議題にする（要件 8.2）
4. If 指摘の直しが 3 種類のどれにも当たらないとき, then the 本 spec shall その指摘を残すことを議題にせず直す
5. When 開発者が指摘を残すと裁定したとき, the 本 spec shall 裁定とその理由をこの文書に残し、要件 9 の「新しい指摘 0 件」の数から外す
6. If 確定した指摘の直しが 20 タスクを越えそうなとき, then the 本 spec shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切ることを議題にする（見込みは 10〜16 タスク＝棚卸㊶。仮定 5）

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that 入力の出来事のコードからバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本 spec の直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本 spec の直し shall 要件 2.2 に当たって開発者が残すと裁定した指摘と、要件 7 で後の spec へ回すと裁定した指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出たとき, the 本 spec の直し shall その箇所を使うテスト・検体・呼び出し元まで直し切り、直しの途中の状態を残さない
4. When コード・テスト・検体を消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・検体・設定・文字列での参照を含む）で使われていないことを確かめる
5. The 本 spec の直し shall 直し方を ponytail の「いちばん小さな完全な変更」に従って選び、消せるものは消す

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本以上持たせる
2. When バグを直すとき, the 本 spec の直し shall 直す前にそのバグで赤になるテストを立て、直した後に同じテストが緑になることを確かめる
3. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる（時刻の長さや走る順の偶然に頼らない）
4. The 本 spec のテスト shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）の中で回る
5. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
6. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
7. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない・本質の変わったテストを黙って消さない）

### Requirement 5: brief に預かった件を片づける

**Objective:** As a 開発者, I want 前の spec やほかの brief から預かった件が、持ち主のこの spec で片づいてほしい, so that 持ち主の無い宿題が残らない

行番号は brief の棚卸㊶の再測定（`main` `31ac3eaf`）と着手のときの `main` `f6405e09` で確かめたもの。ギャップ分析で当て直す（要件 1.7）。

#### Acceptance Criteria

1. The 本 spec の直し shall `crates/areka-input/src/lib.rs` 508 行の「The Hand／collisionex は実装しない」を、`collisionex-regions` の着地の後の事実に合う文に直すか、理由を持たない文なら消す（件 1＝`collisionex-regions` の申し送り・棚卸㊶の ⑵）
2. The 本 spec の直し shall `crates/areka-input/src/lib.rs` の冒頭の説明ほか、本体の中にあった頃の書き方のまま残る説明を、入力の crate の今の事実に合う文に直す（件 2＝`input-crate-split` の申し送り）
3. The 本 spec の直し shall 強制退避の枝へ落ちない側のテスト 3 本（`input_events_tests.rs` の `handler_wired_ctrl_left_double_click_…`・`handler_left_double_click_without_ctrl_…`、`input_events_menu_tests.rs` の `wired_ctrl_left_double_click_…_partner_window`）の「窓が消えて件数が落ちる」という説明を、今の事実（入力の crate のテストの World には口が据わらず、合否は `try_recv` の表明が保つ）に合う文に直す（件 3＝同）
4. The 本 spec の直し shall `input_events_tests.rs` 362 行の「強制退避は下の 2 本（結線前・Ctrl+Shift）が持つ。」を、テストが本体の `app_exit_escape_tests.rs` へ移った今の事実に合う文に直すか、パスを書かずに指す形に直す（件 4＝同。本体のファイルの名は `session-module-gather` が動かしうるので、パスで書かない＝仮定 6）
5. The 本 spec の直し shall `spawn_assembly_tests.rs` 170〜181 行あたりの「入力の側」「input_events の檻」の言い方を、入力の crate の今の名で言う文に直す（件 5＝同）
6. The 本 spec の直し shall `crates/areka-runtime/examples/collision-probe.rs` 63 行あたりと `collision-probe/setup.rs` 75〜76 行の「`pub(crate)`・内部が `crate::` パスを使う」という理由の文を、`attach_char_pointer_handlers` が `pub` になった今の事実に合う文に直す（件 6＝同。範囲の外のファイルだが、C24 の約束 ⑶ で説明の文に限って触ってよい）
7. The 本 spec の直し shall `cargo doc -p areka-input --no-deps` の警告（brief の値で 20 件＝私有の項目へのリンク 19 件・`MouseWiring::with_clock` の解けないリンク 1 件）を 0 件にする（件 7＝同）
8. The 本 spec の直し shall `input-crate-split` が開けた項目（9 モジュールと `pub mod hooks`・根の 8 項目・子の 18 項目・メソッド 7 つ・`test-support` の機能で開けた `pointer_test_support`・`MouseWiring::with_clock`・`user_break` の `flag_rx`・`flag_source_connected`）ごとに、`areka-input` の外に実在の使い手があるかを確かめ、使い手の無いものは狭めるか消す。本体のテストだけが使う項目は使い手に数え、本体が使う項目は狭めない（件 8＝同・C24 の約束 ⑶・仮定 4）
9. The 本 spec の直し shall `crates/areka-input/src/user_break.rs` 201 行の `crates/areka/src/readme.rs` のパスを、パスを書かずに本体の関数を名で指す形に直す（件 9＝C24 の約束 ⑶。`session-module-gather` が本体のファイルを移すため）
10. The 本 spec の直し shall `crates/areka-input/src/choice_drain.rs` 253 行あたりの私有の `capture_logs` の写しを、範囲の中か上流にある共通の捕捉の補助を使う形に寄せる（件 10＝`audit-areka-app` の brief の「`audit-emo2-boot-frame` からの申し送り」のサ・棚卸㊶で持ち主へ寄せた。範囲の中には `balloon_test_support.rs` 185 行に同じ形の `capture_logs` がある）
11. The 本 spec の直し shall `crates/areka-input/src/drag_test_support.rs` 283 行の `let _ = self.parts.handle.join();` を、待ちに上限を持ち、書き手の panic を捨てない形（`areka_runtime::spine::join_bounded` と同じ扱い）に直す（件 11＝`audit-areka-app` の brief の同じ申し送りのコ・仮定 7）
12. The 本 spec shall 件 1〜11 を片づけたことを、預かった件の出どころ（`collisionex-regions`・`input-crate-split` の申し送り・`audit-areka-app` の brief のサ・コ・C24 の約束 ⑶）と一緒に記録に残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall 1 ファイル 1,000 行の番人を越えさせず（足すなら新しいファイルへ）、番人の上限越えの表へ新しいファイルを足さない
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録と `Err` の両方で表し、panic は続けられない致命の場合に限る）
3. The 本 spec の直し shall 終了の道（台本の中断・アプリの終了・スレッドの後始末を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 順序や状態の問題を 1 フレーム遅らせて逃げず、状態の持ち方を直して同じフレームの中で解く
5. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
6. The 本 spec の直し shall 依存を足さない（C24 の約束 ⑹＝`Cargo.lock` の席は `test-crash-report-merge`。外の crate も、ワークスペースの中の crate への新しい依存も含む）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする
8. The 本 spec の直し shall 外の crate をフォークも `[patch]` もせず、足りない所は公開の型で自分の側に組む
9. The 本 spec の直し shall 当たり判定の絵を GPU で解く今の形を、CPU で解く形へ戻さない
10. The 本 spec の直し shall 「イベントを投げ、さくらスクリプトが返ったら再生する」仕組みを崩さず、ベースウェアの側に判断やモードを足さない

### Requirement 7: 範囲の外へ直しが及ぶとき・C24 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えたとき, the 本 spec の直し shall ツリー全体で呼び出し元を探して、全部直す
2. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
3. While C24 のあいだ, the 本 spec の直し shall 本体（`crates/areka/`）のファイルに手を入れず、本体（本番とテスト）・`crates/real-run-kit/`・examples が使う `areka-input` の公開の項目の名と形を変えない（C24 の約束 ⑶）
4. While C24 のあいだ, the 本 spec の直し shall `crates/areka-runtime/` と `crates/areka-placement/` に、要件 5.6 の説明の文のほかは手を入れない
5. While C24 のあいだ, the 本 spec の直し shall `Cargo.lock` と `crates/log-capture-kit/` に手を入れない
6. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない（範囲の外の指摘を見つけても、その範囲の持ち主の件として残す）
7. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める
8. When 外への約束のうちログの行の語か出所を変えると裁定されたとき, the 本 spec の直し shall 同じ変更の中で読み手（実機の確かめ・`tools/`・手順書を grep で探したもの）を全部直す
9. If 直しが C24 のほかの 3 本の触るファイルに及ぶとき, then the 本 spec shall そのファイルを触らずに、相手のセッションへ申し送り、申し送った件と相手を記録に残す
10. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在する（`completed/` でない）ことを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
11. The 本 spec shall ギャップ分析で範囲の外に見つけた件を、その範囲の持ち主の brief に件として書き足し、回した件と回し先を記録に残す
12. When `test-crash-report-merge` が `areka_runtime::crash_report_test_support` の置き場や名を変えるとき, the 本 spec shall `crates/areka-input/src/crash_report_tests.rs` の使い方を、要件の討議で決めた分担に従って追従させる（仮定 1・OPEN QUESTION 1）
13. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、共有の机を汚す操作・外の実物を動かす操作・重い走行が勝手に走らないでほしい, so that ほかの仕事と開発者の机が汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、静かな机を要る計測・画面の操作・開発者の areka や SSP の本物の置き場への設置を回さない
2. When 要件 2.3 のために SSP の MCP で測るとき, the 本 spec の作業 shall 測る前に開発者の許しを得て、読み直し（reload）を打つときは毎回許しを取り、排他テストの机で順を取り、測り終えたらすぐ机を返す
3. Where 実機の確かめ（emo2・pasta の実走のテスト・入力先の実機のテスト）が全体テストの段として既に回っているところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
4. When 重い走行のテストが赤になったとき, the 本 spec の作業 shall 2 回目からは赤が出たテストだけに絞って短く回し、全体の走行を何度も回さない
5. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の窓・偽のポインタの状態・偽の時計・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にする

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で残すと裁定された指摘と、要件 7.7・7.10 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 7.3〜7.5 と 6.6 の約束が守られている（本体のファイルに差分が無い・`areka-runtime` と `areka-placement` の差分が要件 5.6 の説明の文だけ・`Cargo.lock` と `log-capture-kit` に差分が無い）ことと、外への約束の変わった所が要件 7.7 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞い（SHIORI へ送るもの・カーソル・ログの行・公開の面）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・`audit-seriko` と同じ形）
7. If かけ直しのたびに同じ所から「必ず直す」が出続けるとき, then the 本 spec shall 打ち切らずに、その所の直しを Fable で総括し直し、その結果を議題として開発者へ出す（2026-11-15 `audit-placement-core` の決め）
8. If かけ直しの指摘が、前の回の直しが呼んだ連鎖（直しが次の指摘を生んだもの）であるとき, then the 本 spec shall 関連する指摘を Fable でまとめて、回をまたいで矛盾の無い形に直す。前の回と関わらない単なる追加の指摘は普通に直して閉じる（2026-11-17 開発者の決め）

## 仮定と要件の討議への持ち越し

brief で決まらなかった点は、次の仮定で書いた。どれも要件の討議の議題にする（本文の「仮定 N」）。

1. **落ちの報告の分担（要件 7.12・OPEN QUESTION 1）**: `crates/areka-input/src/crash_report_tests.rs` は `areka_runtime::crash_report_test_support::{install, installed}` を使う。けれども `test-crash-report-merge` の brief は使い手を 3 crate と本体の `runtime_tests/mod.rs` だけとし、「`areka-input` は落ちの報告を使わない」と書いている。同 spec は C24 の約束 ⑷ で `crates/areka-input/` に触らない。仮定＝後から着地する側が追従する（先に向こうが着地したら、この spec が rebase して `crash_report_tests.rs` の参照を新しい置き場へ機械的に付け替える。こちらが先なら、向こうが付け替えるか、`areka-runtime` に出し直しを残す）。この食い違いは、要件の討議の前に `test-crash-report-merge` のセッションへ知らせる。
2. **右クリックメニューと spec の名（OPEN QUESTION 2）**: spec の名には「menu」が残るが、2026-11-01 の決めで `crates/areka/src/menu/`（`menu/shortcut*.rs` を含む）は範囲の外。仮定＝見直さない。入力の crate の中でメニューの側から呼ばれる口（`lib.rs` の右ダブルクリックの預かりの取り出しと送出・右クリックされた窓のスコープ・`send_close_request`、それを確かめる `input_events_menu_tests.rs` ほか）は `crates/areka-input/` のファイルなので範囲に入れる。ただし本体のメニューが使う項目なので、名と形は変えない（要件 7.3）。
3. **カーソルの絵（要件 1.3）**: 仮定＝`cursor/` の 4 つの絵は中身を読まない（audit が読める形でない）。置き場・名前・使われ方と `README.md` の記述との食い違いを見る。
4. **使い手の数え方（要件 5.8）**: 仮定＝本体のテストだけが使う項目も使い手に数える（`audit-emo2-boot-frame` の確定 5 と同じ）。`session-module-gather` が本体の 26 の呼び手を書き換えても、使う項目の集合は変わらない前提で数える。
5. **規模（要件 2.6）**: 仮定＝1 本のまま進める（見込み 10〜16 タスク）。タスクを作って 20 を越えたら、そのとき 2 本目へ切ることを議題にし直す。
6. **本体のファイルを名指す説明（要件 5.4・5.9）**: 仮定＝本体のファイルのパスを書かず、関数やテストの名で指す（`session-module-gather` が本体のファイルを `session/` へ移すため）。
7. **`drag_test_support.rs` の待ち（要件 5.11）**: この件はこの spec の brief には無く、`audit-areka-app` の brief の申し送り（コ）が「`audit-areka-input-menu` の持ち物」と書いている。仮定＝引き受ける。`join_bounded` は `areka-runtime` の公開の関数で、`areka-input` は前から `areka-runtime` に依存しているので依存は増えない。待つ相手の型が合わないときの直し方は設計で決める。
