# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（起票のときは 5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **配置の crate（`areka-placement`）の残り（窓の出し方・設定・保存・重なりの順・DPI ほか）** を受け持つ（受け持つファイル＝`crates/areka-placement/` のうち `audit-placement-follow` の持ち物〔`src/follow*`・`src/transition_*`〕でないもの全部〔`Cargo.toml`・`examples/` を含む〕と、`crates/areka/src/placement_tests/` のうち `transition_*` でないもの〔`mod.rs` を含む〕）。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。

あわせて、`placement-crate-split` からの申し送り（⑴〜⑷）と `audit-emo-atlas` からの申し送り（件 1＝環境変数で赤になるテスト）、2026-11-14 棚卸㊲の再測定で見つかった古い名前（`emo2_boot`・`Emo2Wiring`）の説明を、この spec で片づける。

## Introduction

この文書は、配置の crate の核（窓の出し方・設定・保存・重なりの順・DPI ほか）と、それを確かめる本体の `placement_tests/` の一部に `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直すための要件を定める。要件の段では「全部直す」という形だけを書き、audit の指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書に落として直す対象を確定する。

申し送りの件は中身が決まっているので、要件 2 に外から見た結果として書く。直し方は要件の討議と設計で決める。

受け持つ範囲の大きさ:

| 測った時点 | ファイル | コード | テスト |
|---|---|---|---|
| 起票のとき（`e3c2cbe4`・旧 `crates/areka/src/placement/`・brief の値） | 73 | 7,661 行 | 20,167 行 |
| `placement-crate-split` の段 5（`dfc9974f`・brief の値） | 114 | 10,459 行 | 27,424 行 |
| 要件の段（`30a343eb`・`.rs` だけを数えた） | 114（うち `.rs` 113） | 10,413 行 | 27,424 行 |

テストに数えたのは、名前が `_tests.rs`・`_test_support.rs` で終わるファイル、`test_support.rs`・`test_crash_report.rs`、`crates/areka/src/placement_tests/` の全部。ファイルの数には `Cargo.toml` を含む。1,000 行に近いのは `src/measure_tests.rs`（999 行）・`src/persist_restore_tests.rs`（939 行）・`src/persist.rs`（928 行）・`src/lib.rs`（918 行）。範囲の中に、1 ファイル 1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`・範囲の外）に載るファイルは 0 本（載る配置のファイルは、どれも `audit-placement-follow` の持ち物）。ギャップ分析で `main` を取り込んで測り直す（要件 1.2）。

この spec はウェーブ C20 で、ほかの 7 本（`wintf-visual-damage-redraw`・`wintf-winmsg-executor-import`・`audit-placement-follow`・`audit-wintf-window`・`audit-wintf-input`・`audit-tools-perf-run`・`audit-tools-perf-judge`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C20 の行の ⑴〜⑹）を守る（要件 8）。

## Boundary Context

- **In scope**:
  - `crates/areka-placement/` のうち `src/follow*`・`src/transition_*` でない全ファイル（本番のコード・テスト・テストの支え・`examples/`・`Cargo.toml`）と、`crates/areka/src/placement_tests/` のうち `transition_*` でない 9 ファイル（`mod.rs` を含む）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 7 と要件 8 の約束に従う。C20 のあいだは要件 8 で触れない所がある）。
  - 申し送りの件（要件 2）。
- **Out of scope**:
  - `audit-placement-follow` の持ち物（`crates/areka-placement/src/` の `follow.rs`・`follow/` の下・`follow_*`・`transition_*` と、`crates/areka/src/placement_tests/transition_*`）。
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 5）。
- **Adjacent expectations**:
  - この crate を使う範囲の外のコードは、本体 `crates/areka/`（`runtime`・`input_events`・`menu`・`session` ほか）と wintf の 2 か所（`ecs/window_proc/lifecycle.rs` が `diag::DESPAWNED_SKIP_TAG` の綴りを、`ecs/window_proc/window_pos_tests.rs` がログの出所 `areka_placement::diag` を名指す）。wintf の 2 ファイルは同じ C20 の `audit-wintf-window` の持ち物。
  - `audit-placement-follow` とは受け持つファイルが重ならないが、参照は両向きに多い（`placement_tests/mod.rs` が相手の `transition_judge`・`transition_judge_offset` を宣言する・相手の `follow/offset_space` が根の `scale_signed` を引く）。互いの持ち物の関数の形は相談なしに変えない（要件 8）。
  - wintf の `MonitorRecord`（`ecs/window/transition_diag.rs`）と矩形の型は `audit-wintf-window` の持ち物。この spec は wintf の型へ手を入れない（要件 8）。
  - C21 の `runtime-crate-split` が、受け持ちの `placement_tests/reset_balloon_pos_tests.rs` を `areka-runtime` へ移す。この spec はその前に着地する。
  - `clippy-199-lints` は同じ crate に触る。本 spec が audit の指摘として clippy の指摘と同じ所を直したときは、`clippy-199-lints` が直し直さなくてよいように記録を残す（要件 4.4）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行う, the 本見直し shall 受け持つ 114 ファイル（Boundary Context の In scope）だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the 本見直し shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）を測り直し、使った ponytail のプラグインの版とあわせて報告に書く
3. The 本見直し shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
4. The 本見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない
5. The 本見直し shall 各指摘に通し番号（範囲を分けて読んだ結果をまとめた後の、3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘が要件 2 の申し送りの件と同じ所を指す, the 本見直し shall その指摘と件の対応を報告に書き、同じ直しを 2 回数えない
7. When audit の指摘を直すのに要件 8 の約束（公開の形は足すだけ・相手の持ち物の関数の形を変えない・wintf の型に手を入れない・依存を足さない）に触れる, the 本見直し shall その指摘にその旨と、範囲の外のどの呼び出し元やどの spec に響くかを書く（要件 8.6 の議題の材料）

### Requirement 2: 申し送りの件を片づける

**Objective:** As a 開発者, I want ほかの spec から申し送られた件が、この spec の中で確かに片づいていること, so that 配置の crate に古い説明や環境で揺れるテストが残らない

#### Acceptance Criteria

1. The 本見直し shall `placement-crate-split` の申し送り ⑵ に挙がった `#[allow]` 属性と説明のうち、理由の文が今の事実（配置は lib の crate・examples は `#[path]` でソースを取り込まない）に合わないものを、属性が要らなければ属性ごと消し、要るなら今の事実に合う理由の文に直す
2. The 本見直し shall 申し送り ⑵ で「古くなっていない」とされた `#[allow(dead_code)]` について、理由が今も事実かを確かめ直し、事実でなくなっていれば 2.1 と同じに扱う
3. The 本見直し shall 申し送り ⑶ に挙がった、外から使うために開けた項目と、テストの支えのために開けた項目のそれぞれについて、範囲の外の使い手をツリー全体で確かめ、使い手の無い公開を指摘として一覧に入れる（直し方は要件 8.2 と 8.6 に従う）
4. The 本見直し shall `lib.rs` の頭に書かれた依存の向きの規約と、実際のモジュールどうしの参照（申し送り ⑷ の、規約が名指さない参照と輪を含む）が食い違わないようにする（規約の文を直すか、参照を直すかは設計で決める）
5. Where 申し送り ⑷ の根の `scale_signed` の形を変える, the 本見直し shall `audit-placement-follow` のセッションと申し合わせてから変える
6. Where 申し送り ⑴ の wintf との重なり（`diag.rs` の `MonitorRecord`・`resolver.rs` の `RectPx`）を audit が指摘する, the 本見直し shall その指摘を要件 8.3 と 8.6 に従って扱う
7. The 本見直し shall `measure_first_frame_tests.rs` の `fade_in_animation_all_transparent_warns_zero_lines_on_measure_bake_and_raw_bake` を、動く絵の上限の環境変数（`AREKA_ANIMATED_IMAGE_MAX_FRAMES`・`AREKA_ANIMATED_IMAGE_MAX_PIXELS`）を立てて走らせても、立てずに走らせても、同じ結果になる形に直す（`audit-emo-atlas` の件 1）
8. When 2.7 を直した, the 本見直し shall `crates/areka/src/placement_tests/measure_bake_log_tests.rs` と `measure_bake_count_tests.rs`（ギャップ分析で、こちらも同じ環境変数で赤になると分かった＝指摘 5）も、同じ環境変数を立てて走らせたときに結果が変わらないことを確かめ、変わるなら同じに直す
9. The 本見直し shall 受け持つファイルの説明のうち今のコードを指す所に残る古い名前（`emo2_boot`・`Emo2Wiring`・要件の段の `30a343eb` で 13 ファイル・27 行）を今の名前（`runtime` ほか）に直し、直したファイルと行の数を `runtime-crate-split` の brief へ申し送る（同 spec が直す数を減らすため。同 brief の「`areka-placement` の 4 行は棚卸㊲で直した」の記述と数が合わなければ、その記述も正す）

### Requirement 3: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行う, the 本見直し shall ギャップ分析の指摘の一覧を、直す対象としてこの文書に落とす
2. When 要件の討議で直す対象が確定した, the 本見直し shall 確定した指摘の一覧（通し番号・組・一言・直すか裁定で残すか）をこの文書に書き加える
3. The 本見直し shall 直さずに残す候補（要件 5 の 3 種類に当たるもの）と、要件 8.6 で並走の約束に触れる指摘を、要件の討議の議題にして開発者の裁定を仰ぐ

### Requirement 4: 指摘を全部直す

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that 配置の crate の核からバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本見直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本見直し shall 要件 5 の 3 種類のどれかに当たって開発者が残すと裁定した指摘と、要件 8.6 で開発者がほかの spec へ回すと裁定した指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出た, the 本見直し shall その箇所を使うテスト・fixture・examples・呼び出し元まで直し切り、直しの途中の状態を残さない
4. When audit の指摘として clippy の指摘と同じ所を直した, the 本見直し shall どこを直したかを `clippy-199-lints` の brief に記録する

### Requirement 5: 直さずに残す指摘は開発者の裁定で決める

**Objective:** As a 開発者, I want 直さずに残す指摘を自分で裁定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. If 指摘の直しが次の 3 種類のどれかに当たる, the 本見直し shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞い（窓の出る位置・重なりの順・保存して次に起動したときの位置・DPI が変わったときの見え方ほか）が変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. When 3 種類目に当たるかを判断する, the 本見直し shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする
3. If 指摘の直しが 3 種類のどれにも当たらない, the 本見直し shall その指摘を残すことを議題にせず直す
4. When 開発者が指摘を残すと裁定した, the 本見直し shall 裁定とその理由をこの文書に残し、要件 6 の「新しい指摘 0 件」の数から外す

### Requirement 6: 直した後の確かめ

**Objective:** As a 開発者, I want 直しが新しい問題を生んでいないことを確かめられること, so that 見直しが終わったと安心して言える

#### Acceptance Criteria

1. When 確定した指摘を全部直し終えた, the 本見直し shall 同じ範囲に、要件 1 と同じ作法（件数の上限なし・全部読む）で `/ponytail:ponytail-audit` をもう一度かける
2. If もう一度かけた audit が新しい指摘（直しが生んだ指摘を含む）を出した, the 本見直し shall その指摘も直し、新しい指摘が 0 件になるまで audit をかけ直す
3. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無い, the 本見直し shall 開発者に一言知らせたうえで、5 回目の指摘を直したところでかけ直しを打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書く（2026-11-06 `audit-seriko` で開発者が承認した形）
4. The 本見直し shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を検証の記録に残す
5. When 見直しを終える, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる
6. The 本見直し shall 直したバグと危ない所の 1 件ずつに、その直しが壊れたら落ちるテストを 1 本以上置く
7. The 本見直し shall 足したテストと直したテストを、毎回同じ結果になる（決定論の）テストにし、環境変数や走らせる順に結果が左右されない形にする

### Requirement 7: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが安全に、並んで走るほかの spec とぶつからずに入ること, so that ほかのクレートが壊れず、並行作業が止まらない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えた, the 本見直し shall ツリー全体で呼び出し元を探して、全部直す
2. If 直しが及ぶ呼び出し元の範囲を受け持つ spec のセッションが走っている, the 本見直し shall そのセッションと申し合わせてから、その呼び出し元に触る
3. When コード・テスト・fixture を消す, the 本見直し shall 消す前に、ツリー全体（テスト・fixture・設定・examples・文字列での参照を含む）で使われていないことを確かめる
4. The 本見直し shall 範囲の外のファイルの、直しに要らない箇所を直さない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残し、持ち主の spec が実在することを確かめたうえでその brief へ申し送る）

### Requirement 8: C20 の並走の約束

**Objective:** As a 開発者, I want 見直しの直しが同じウェーブ C20 のほかの spec の邪魔をしないこと, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. The 本見直し shall `audit-placement-follow` の持ち物のファイル（Boundary Context の Out of scope の 1 つ目）に手を入れず、相手のファイルが使う項目の名前と形を、足すことのほかは相談なしに変えない（C20 の約束 ⑵）
2. While C20 のあいだ, the 本見直し shall `areka-placement` の公開の形（範囲の外から呼べるモジュール・型・関数・定数・欄の名前と形）を、足すことと要件 8.6 で開発者が認めた例外のほかは変えない（C20 の約束 ⑵）
3. The 本見直し shall wintf の型と、wintf が名指す記録の綴りと出所（`DESPAWNED_SKIP_TAG` の綴り・ログの出所 `areka_placement::diag` の置き場）を変えない（C20 の約束 ⑶）
4. The 本見直し shall 依存を足さず、`Cargo.lock` を変えない（C20 の約束 ⑸＝`Cargo.lock` の席は `wintf-winmsg-executor-import`）
5. If 直しに新しい依存が要ると分かった, the 本見直し shall 依存を足さずに開発者へ議題として出す（足すなら `tech.md` への登記と開発者の承認が要る）
6. If 確定した指摘が 8.1〜8.4 の約束に触れずには直せない, the 本見直し shall その指摘だけを直さずに議題にし、「相手のセッション（`audit-placement-follow`・`audit-wintf-window` ほか、触れる相手）と申し合わせて C20 のうちに直す」か「実在を確かめた持ち主の spec へ回し、その brief へ申し送る」かを開発者の裁定で決める
7. Where 8.6 で C20 のうちに直すと裁定した, the 本見直し shall 直す前にツリー全体で範囲の外の使い手を確かめ直し、響く相手のセッションへ変える名前を知らせてから直す
8. The 本見直し shall `vendors/` の下に手を入れない

### Requirement 9: 守る決まり

**Objective:** As a 開発者, I want 見直しの直しがプロジェクトの決まりを守ること, so that 直しが別の決まりを破らない

#### Acceptance Criteria

1. The 本見直し shall 1 ファイル 1,000 行の番人を赤にせず、直しで行が上限を越えるときは新しいファイルへ分け、番人の例外表へ新しいファイルを足さない
2. The 本見直し shall 記録の無い失敗の道を作らず、失敗は記録（`error!`）を残したうえで呼び出し元へ失敗（`Err`）として返す
3. The 本見直し shall 終了の道を正規の実装にし、終了を早めるための小細工を入れない
4. The 本見直し shall 実機の根と一時フォルダを、ワークツリーの `target` の下に置く
5. The 本見直し shall GPU のテストを実 GPU で回す
6. If 文字化けが起きた, the 本見直し shall コンソールの文字コードを変えて逃げず、読む側を直す

### Requirement 10: spec の大きさ

**Objective:** As a 開発者, I want 1 本の spec が大きくなりすぎないこと, so that 1 本ずつ確かめて着地できる

#### Acceptance Criteria

1. If 確定した指摘の直しが 20 タスクを越えそうになった, the 本見直し shall 要件の討議で、指摘の組ごと（またはファイルのまとまりごと）に同じ範囲の 2 本目の spec として切ることを議題にする
2. When 2 本目の spec を切ると裁定された, the 本見直し shall 2 本目へ移す指摘の通し番号をこの文書と 2 本目の brief の両方に書き、どちらにも載らない指摘を残さない
