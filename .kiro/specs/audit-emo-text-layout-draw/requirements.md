# Requirements Document

## Project Description (Input)

- **誰が困っているか**: areka の開発者。コードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。
- **今の状況**: 全 33 本の見直しのうち、この spec は **文字の層のレイアウトと描画**（`crates/areka-emo-text/src/` の `layout*`・`segment*`・`region*`・`wrap*`・`lookahead*`・`cursor_tag*`・`writing*`・`draw*`・`canvas*`・`viewbox*`・`look*`・`color*`・`underline*`・`place*`）を受け持つ。まだ audit をかけていない。指摘はギャップ分析で洗い出す（起票のときに出さなかったのは、着手までにコードが変わって指摘が古びるため）。ほかに、`audit-parsers-sakura` から 1 件の申し送り（大きな有限の割合の改行を重ねると、配置の位置が無限大や数でない値になる）を受けている。
- **何が変わるべきか**: 受け持つ範囲の audit の指摘が 3 つの組（必ず直す・直したほうがよい・できれば）とも全部直り、申し送りの件も直り、直した後にもう一度かけた audit で新しい指摘が 0 件になり、全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所の 1 件ごとにそれが壊れたら落ちるテストがある。進め方は「原則、ギャップ分析でauditを実施して、設計以降で修正」（2026-10-19 開発者）。正本は `brief.md` と roadmap の「テーマ別の決めごと」の「ponytail による見直し」。

## Introduction

この文書は、文字の層のうちレイアウトと描画を受け持つファイルに `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組とも全部直すための要件を定める。要件の段では「何を、どの作法で見直し、何をもって終わりとするか」の**形だけ**を書き、指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書へ落として直す対象を確定する（要件 3）。申し送りの 1 件だけは、望む姿が brief で決まっているので、ここに要件として書く（要件 5）。

受け持つ範囲の大きさ（行数は空行を含む。テストに数えたのは、名前が `_tests.rs` か `_test_support.rs` で終わるファイル）:

| 測った時点 | ファイル | コード | テスト |
|---|---|---|---|
| 起票のとき（main `e3c2cbe4`・brief の値） | 76 | 10,135 行 | 24,371 行 |
| 着手のとき（main `73c5b25d`） | 81（コード 27・テスト 54） | 10,820 行 | 27,643 行 |

いちばん大きいファイルは `draw_format_metrics_tests.rs` の 922 行で、1 ファイル 1,000 行の上限に近い。範囲のファイルに、番人の例外表に載っているものは無い。

## Boundary Context

- **In scope**:
  - 受け持つファイル＝`crates/areka-emo-text/src/` のうち、名前が `layout`・`segment`・`region`・`wrap`・`lookahead`・`cursor_tag`・`writing`・`draw`・`canvas`・`viewbox`・`look`・`color`・`underline`・`place` で始まるファイル全部（コード・テスト・テストの支え）。
  - 受け持つファイルの指摘の直しと、`audit-parsers-sakura` からの申し送りの 1 件の直し。そのためのテストの追加と直し。直しのために要る、範囲の外の呼び出し元の直し（要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外の見直し（ほかの 32 本の見直しの持ち物）。同じクレートのうち `actor*`・`state*`・`choice*`・`sink*`・`surface*`・`lib.rs`・`balloon_overrides*`・`inline_marker*`・`marker_surface*`・`scroll_marker*`・`tests/`・`examples/`・`Cargo.toml` は `audit-emo-text-state-actor` の持ち物。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し（直さずに議題にする）。
- **Adjacent expectations**:
  - 同じウェーブ C18 で `audit-emo-text-state-actor` が同じクレートの別のファイルを受け持って走る。受け持つファイルは重ならない。このクレートに新しいソースファイルを足してよい 1 本だけの席は、C18 ではあちらが使う。
  - 同じウェーブの `wintf-visual-tree` は `ecs::graphics::Visual` の名前を変えず、`ecs::visual` の側を直す約束（変えると、受け持つ `draw_oracle_tests.rs`・`viewbox_draw_test_support.rs` ほかに響く）。
  - 同じウェーブの見直し（`audit-emo-text-layout-draw` を含む 6 本）は、`crates/areka/` と、`placement-crate-split` が触る外のファイルに触らない。そのため、このクレートの公開の口は変えず、足すだけにする（`crates/areka/` の約 60 ファイルがこのクレートを使う）。
  - `audit-parsers-sakura`（完了）: 改行の割合のうち非有限の値だけを既定の 1.0 へ倒し、有限で大きな値は縛らない。残りの守り（位置が非有限にならないこと）は、この spec が配置の側で受け持つ（要件 5）。
  - 直にこの見直しの着地を待つ v0.2.0 の spec は `compose-f16-precision`・`dola-interrupt-ticks` の 2 本。この見直しはそれらの作り直しを先取りしない。
  - 範囲の 5 ファイル（`canvas.rs`・`color.rs`・`draw.rs`・`look.rs`・`region.rs`）に `doc/COMPAT_ARCHITECTURE.md` を名指す説明の行がある。この名前の改名は `doc-compat-architecture-rename` の持ち物で、この見直しは改名しない。
  - `clippy-199-lints` は同じクレートに触るため、見直しのあいだは同じウェーブに並べない。

## Requirements

### Requirement 1: 範囲の確定

**Objective:** As a 開発者, I want 見直しが受け持つファイルを着手のときの実物で確定したい, so that 見直しの漏れも、ほかの見直しとの二重の持ち物も起きない

#### Acceptance Criteria

1. When ギャップ分析を始める場合, the 見直し shall `crates/areka-emo-text/src/` のうち名前が `layout`・`segment`・`region`・`wrap`・`lookahead`・`cursor_tag`・`writing`・`draw`・`canvas`・`viewbox`・`look`・`color`・`underline`・`place` で始まる、git の管理下のファイルを全部数え、その一覧と数（ファイル数・コードの行数・テストの行数）を記録に残す
2. The 見直し shall `audit-emo-text-state-actor` の持ち物（`actor*`・`state*`・`choice*`・`sink*`・`surface*`・`lib.rs`・`balloon_overrides*`・`inline_marker*`・`marker_surface*`・`scroll_marker*`・`tests/`・`examples/`・`Cargo.toml`）と `vendors/` の下のファイルを、受け持つファイルに含めない
3. If 範囲のファイルの一覧が、着手のときより後に（ほかの spec の着地などで）増えたり減ったりした場合, then the 見直し shall 一覧と数を測り直し、増えたファイルも見直しの対象に入れる

### Requirement 2: 指摘の洗い出し

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず読み残さずに洗い出したい, so that 「全部直す」の「全部」が何件かが確かめられる

#### Acceptance Criteria

1. When ギャップ分析を行う場合, the 見直し shall 受け持つ範囲だけを対象にして `/ponytail:ponytail-audit` をかける
2. The 見直し shall audit の作法のうち「最大 20 件」を外し、出た指摘を全件記録して、省いた件を残さない
3. The 見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読んだファイルの数が範囲のファイルの数と一致することを記録に残す
4. The 見直し shall 指摘の一覧をギャップ分析の報告に残し、各指摘に audit の番号をそのまま付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに入るかを書く
5. The 見直し shall ギャップ分析の報告に、3 つの組それぞれの件数を、0 件の組も数字で書く
6. The 見直し shall ponytail のプラグインを利用者全体に入っているまま使い、リポジトリにプラグインのフックやスキルを写さない

### Requirement 3: 直す対象の確定と、直さずに残してよい指摘

**Objective:** As a 開発者, I want 指摘の一覧を要件に落として、直すものと残すものを自分の裁定で確定したい, so that 設計と実装が直す対象をはっきり知り、「直さない」が黙って増えない

#### Acceptance Criteria

1. When 要件の討議を行う場合, the 見直し shall ギャップ分析の指摘を全件この要件の文書へ落とし、各指摘を「直す」か「直さずに残す」のどちらかに確定して、その一覧を設計・タスク・実装の段が従う正本にする
2. The 見直し shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る: ⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印（実在の理由を書いた説明付きで「使われていない」ことを許した語彙の口と、`shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの） ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
3. If 指摘が ⑶ に当たりそうな場合, then the 見直し shall ukadoc で正典を確かめた結果（URL と原文の引用）を添えて、要件の討議の議題にする
4. If 指摘の直しが、このクレートの公開の口のうち `crates/areka/` が使うものを消したり形を変えたりしなければできない場合, then the 見直し shall その指摘を直さずに要件の討議の議題にし、扱い（足すだけの形で直す・後のウェーブへ回す・申し送る）を開発者の裁定で決める
5. When 開発者が指摘を残すと裁定した場合, the 見直し shall その指摘の番号・当たる種類・裁定・理由を spec の記録に残す
6. If 指摘が 3 種類のどれにも当たらず、要件 3.4 の議題で開発者が別の扱いを裁定したものでもない場合, then the 見直し shall その指摘を直す対象から外さない
7. If 直す対象の見立てが 1 spec 20 タスクを越えそうな場合, then the 見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切る案を開発者に出す

### Requirement 4: 指摘を全部直す

**Objective:** As a 開発者, I want 「直す」と確定した指摘を 3 つの組とも全部直したい, so that 受け持つ範囲にバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードが残らない

#### Acceptance Criteria

1. The 見直し shall 「直す」と確定した指摘を、必ず直す・直したほうがよい・できれば の 3 つの組とも全部直す
2. The 見直し shall 各指摘を、その指摘を解く最も小さな完全な変更で直し、消せるものは消し、直しが壊す呼び出し元・テスト・テストの支え・設定まで直し切る
3. When コード・型・ファイル・設定を消す場合, the 見直し shall その前に、テスト・テストの支え・設定・文字列での参照を含むツリー全体で使われていないことを確かめ、その確かめ方と結果を記録に残す
4. The 見直し shall 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直しをしない

### Requirement 5: 申し送りの件（大きな有限の割合の改行で位置が壊れる）

**Objective:** As a 開発者, I want `\n[3e38]` のような大きな有限の割合の改行を重ねても文字の配置の位置が壊れないようにしたい, so that ゴーストの台本の書き損じひとつで吹き出しの文字の位置が無限大や数でない値にならない

#### Acceptance Criteria

1. The 見直し shall `audit-parsers-sakura` からの申し送りの件（同 spec の `research.md` §13.4 の R17）を、ギャップ分析の指摘の一覧に出どころ付きで載せ、直す対象に入れる
2. If 改行の割合が大きな有限の値で、改行の送りを位置に足すと位置が有限の数でなくなる場合, then the 文字の配置 shall 位置を有限の数のまま保ち、その送りを警告の記録（`warn!`）に残す
3. If 改行を重ねて割合の和が有限の数でなくなる場合, then the 文字の配置 shall 要件 5.2 と同じく位置を有限の数のまま保ち、警告の記録に残す
4. While 改行の割合と送りの結果が有限の数に収まっている間, the 文字の配置 shall 今と同じ位置に文字を置く（ふつうの大きさの割合の改行の振る舞いを変えない）
5. The 見直し shall 要件 5.2・5.3 の守りが外れたら落ちる、決定論のテストを 1 本以上持つ

### Requirement 6: 直しが守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないようにしたい, so that 見直しが新しい傷を作らない

#### Acceptance Criteria

1. The 見直し shall 本番とテストのどのファイルも 1 ファイル 1,000 行の上限（番人のテスト）を越えさせない
2. The 見直し shall 失敗を記録なしに捨てる道を作らず、失敗の道ではエラーの記録（`error!`）を残したうえで呼び出し元へ失敗（`Err`）を返す
3. The 見直し shall 終了の道を正規の手順のまま保ち、その場しのぎの細工で終わらせる道を作らない
4. Where 直しのテストが実機の根や一時フォルダを使う場合, the 見直し shall それらをワークツリーの `target` の下に置く
5. Where 直しのテストが GPU を使う場合, the 見直し shall 実際の GPU で回し、ソフトウェアの描き手へ置き換えない
6. If 文字化けに出会った場合, then the 見直し shall コンソールの文字コードを変えて逃げず、読む側を直す
7. The 見直し shall 依存を足さず、`Cargo.lock` を変えない（ウェーブ C18 の約束）
8. If 依存を足さなければ直せない指摘が出た場合, then the 見直し shall その指摘を直さずに議題にし、足すと決まったら `tech.md` へ登記して開発者の承認を得る

### Requirement 7: 範囲の外へ及ぶ直しと、同じウェーブの約束

**Objective:** As a 開発者, I want 範囲の外の呼び出し元へ直しが及んでも、ほかの見直しや同じウェーブの仕事と衝突しないようにしたい, so that 並べて走らせている spec がお互いを壊さない

#### Acceptance Criteria

1. When 直しが関数や型の形を変える場合, the 見直し shall ツリー全体で呼び出し元を探し、要件 7.6 の約束の内で全部直す
2. If 直す呼び出し元がほかの見直しの spec の受け持つファイルにあり、その spec が走っている場合, then the 見直し shall そのセッションと申し合わせてから触る
3. The 見直し shall `crates/areka-emo-text/src/` に新しいファイルを足さず、`lib.rs` に触らない（`lib.rs` と新しいファイルの席は `audit-emo-text-state-actor` の持ち物）
4. The 見直し shall 直したバグ・危ない所の壊れたら落ちるテストを、受け持つ範囲の今ある兄弟のテストのファイルへ足す
5. If テストを足すと兄弟のテストのファイルが 1,000 行を越える場合, then the 見直し shall 新しいファイルの席の持ち主（`audit-emo-text-state-actor`）と置き場を申し合わせてから足す
6. The 見直し shall `crates/areka/` の下のファイルに触らず、このクレートの公開の口のうち `crates/areka/` が使うものを消したり形を変えたりしない（足すだけにする）
7. When 直しが `actor*`・`state*` のファイルから呼ばれる関数の形を変える場合, the 見直し shall その前に `audit-emo-text-state-actor` のセッションと申し合わせる
8. If 同じウェーブの `wintf-visual-tree` の改名が、受け持つ `draw_oracle_tests.rs`・`viewbox_draw_test_support.rs` が使う `ecs::graphics::Visual` の側に及びそうな場合, then the 見直し shall あちらに `ecs::visual` の側を改名してもらうよう申し合わせる
9. The 見直し shall 範囲の外のファイルでは、直しのために要る呼び出し元の直しだけを行い、そのファイルの別の問題は直さない
10. Where audit の指摘が clippy の指摘と同じ所を指す場合, the 見直し shall その所を audit の指摘として直し、それ以外の clippy の指摘の機械的な直しは行わない
11. The 見直し shall `vendors/` の下を変えない

### Requirement 8: 壊れたら落ちるテストと全体テスト

**Objective:** As a 開発者, I want 直したバグと危ない所が、また壊れたら必ず気付けるようにしたい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 見直し shall 直したバグと危ない所の指摘ごとに、その直しが元に戻ったら落ちるテストを 1 本以上持つ
2. The 見直し shall 追加・変更するテストを決定論のテストにし、実行の時刻・順序・外の状態によって結果が変わらないようにする
3. When 指摘を全部直し終えた場合, the 見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる

### Requirement 9: かけ直しで 0 件を確かめる

**Objective:** As a 開発者, I want 直した後の範囲にもう一度 audit をかけて、指摘が残っていないことを確かめたい, so that 「全部直した」を報告ではなく測った結果で言える

#### Acceptance Criteria

1. When 指摘を全部直し終えた場合, the 見直し shall 要件 1 で確定した範囲（その後の増減を含む）へ、要件 2 と同じ作法（全件を出す・全部読む）で `/ponytail:ponytail-audit` をかけ直す
2. If かけ直しで新しい指摘（直しが生んだ指摘を含む）が出た場合, then the 見直し shall それも要件 3〜8 と同じ扱いで直し、要件 3 で残すと裁定された指摘を除いて 0 件になるまでかけ直す
3. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無い場合, then the 見直し shall 開発者に知らせたうえで、5 回目の指摘を直したところで打ち切ってよい
4. The 見直し shall かけ直しの記録に、読んだファイルの数が範囲のファイルの数と一致することと、各回の 3 つの組それぞれの件数（0 件も数字で）と、要件 3 で残すと裁定された指摘の番号と、打ち切ったときはその回と理由を書く
