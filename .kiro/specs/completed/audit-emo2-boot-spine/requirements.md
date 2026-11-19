# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の emo2_boot の残り（背骨・消費者の台帳・適合の台本・資産・トークの時計ほか）** を受け持つ。受け持つファイルは、2026-11-17 `runtime-crate-split` の後の置き場で、`crates/areka-runtime/` と `crates/areka/src/runtime_tests/` のうち `audit-emo2-boot-frame`・`audit-emo2-boot-switch` の持ち物でないもの全部（`crates/areka-runtime/` の `Cargo.toml`・`examples/collision-probe*`・`src/placement_tests/` のうち `transition_*` でないものを含む）と、`crates/areka/src/session/` の `hooks.rs`・`reload*`・`runtime_boot*`・`switch_copy*`・`same_folder_pin*`・e2e の 2 本（`script_security_e2e_tests.rs`・`readme_script_e2e_tests.rs`）である。まだ audit をかけていない。

着手のときの測り直し（2026-11-17・`main` `5749a4c7`・brief と同じく `_tests.rs`・`tests/`・`_test_support.rs`・本体の `runtime_tests/` をテストに数え、`.rs` と `Cargo.toml` を数えた）: 125 ファイル・計 32,204 行（`.rs` 124 本・32,140 行と `Cargo.toml` 64 行・`wc -l`）・うち本番の名前のファイル 42 本・9,966 行（examples 8 本・1,092 行を含む）・テスト 82 本・22,174 行。brief の棚卸㊴の実測（124 本・32,140 行）と合う（ギャップ分析 `research.md` の 1.1 で数え直した）。1,000 行に近いのは `spine_conformance_lap_tests.rs`・`spine_conformance_script.rs`・`session/reload_tests.rs`・`assets_tests.rs`・`session/reload.rs`・`lib.rs`（brief の実測）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。あわせて、brief に預かった件（`audit-sylphya-actor` から回された audit #16・`runtime-module-untangle` と `runtime-crate-split` の申し送り）を片づける。

## Introduction

この spec は、areka の実行時の背骨（起動・SHIORI との行き来・トークの時計・消費者の台帳・適合の台本・資産の読み込み・読み直しと切替の cue ほか）と、本体の起動・読み直し・切替の写しの道（`session/` の受け持ちの分）を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

受け持つ範囲は、areka を使う人から見た振る舞い（起動の順・SHIORI へ送るイベントとその順・トークの進み・読み直しと切替の結果・失敗のときのゴーストの台詞）を直接決める。そのため、範囲の外から見た「外への約束」は次のとおりとする（以下「外への約束」）。

- ゴーストと SSP 互換の振る舞い（SHIORI へ送るイベントの名前・順・Reference の中身、トークとさくらスクリプトの進み、読み直し・切替・終了の結果）。ukadoc が正典。
- `areka-runtime` の公開の面（本体・examples・テストの支えが使う項目）。
- ログの行の語と target（実機の確かめ・`tools/perf/` の道具・手順書が字面で読む）。

この spec はウェーブ C22 で、ほかの 5 本（`collisionex-regions`・`audit-placement-follow-residue`・`audit-test-kits`・`audit-ukadoc-survey`・`audit-pilot`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C22 の行の約束 ⑴〜⑸）を守る（要件 7）。

要件の討議で開発者に決めてもらったことは、末尾の「要件の討議で確定したこと」にまとめた（2026-11-17 に全議題を推奨案で確定・番号は議題の番号）。

## Boundary Context

- **In scope**:
  - 上の受け持つファイル全部に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - brief に預かった件（要件 5）。うち audit #16 は、範囲の外の `crates/areka-actor/src/{ui,lib}.rs`・`crates/areka-actor/tests/` のうち `spawn_ui` を呼ぶもの・`crates/areka-emo-text/src/{actor,actor_test_support}.rs` に及ぶ。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3.2 と要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。例: `crates/areka-runtime/src/frame/`・`frame*.rs`・本体の `runtime_tests/frame*`・`session/ghost_quit*`（`audit-emo2-boot-frame`）、`switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`・`session/ghost_switch*`・`session/shell_balloon_switch*`（`audit-emo2-boot-switch`）、`crates/areka-runtime/src/placement_tests/transition_*`（`audit-placement-follow-residue`）、install・update・readme の cue（`audit-areka-install-update`）。範囲の外で見つけた指摘は、その範囲の spec の持ち物として残す。
  - 新しい機能。ただし、確定した指摘の直しとして、失敗を記録して止まる道を足すことは新しい機能に数えない。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - areka 本体（`crates/areka/src/` の `main.rs`・`session/` のほかのファイル・`mcp/` ほか）は、`areka-runtime` の公開の項目を今の名前と形で使っている。公開の形を変える直しは、本体の呼び手を同じ変更で全部直す（要件 7.6）。
  - 実機の確かめ（emo2・pasta の実走のテストと `doc/pasta-real-run.md`）と `tools/perf/` の道具は、受け持つ範囲が出すログの行の語を字面で読む。
  - `areka-actor` の `spawn_ui` の呼び手は、`areka-runtime` の `lib.rs`（`BootWiringError::SpawnUi`）・`areka-emo-text` の `actor.rs`・`areka-actor` のテストである。audit #16 の直しはこれを全部直す。
  - 同じウェーブの `audit-test-kits` は番人の crate（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`・`real-run-kit`）を受け持ち、`audit-placement-follow-residue` は `placement_tests/transition_*` を受け持つ（要件 7.3〜7.5）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall Project Description の受け持つファイルだけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの一覧・数・行数（本番のコードとテストを分けて）と、使った物差しの版を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘を直すのに外への約束を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの読み手（ゴーストと SSP 互換の振る舞い・本体の呼び手・ログの行を読む実機の確かめと道具）に響くかを書く（要件 2.3・7.6 の議題の材料）
7. The ギャップ分析 shall 要件 5 の各件について、今の `main` で事実を確かめ直した結果（場所・行・今も当てはまるか）を報告に書く

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 要件の討議 shall ギャップ分析の指摘を全件、3 つの組とも直す対象として要件に落とす
2. The 本 spec shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る
   - ⑴ 開発者の裁定に反する直し
   - ⑵ わざと置いた先送りの印（実在の理由を書いた説明つきで使われていないことを許している語彙の口〔`#[allow(dead_code)]`〕、および `shortcut:`・`ponytail:` の注釈で、書いた限界が今の負荷を越えていないもの）
   - ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
3. If 指摘が ⑶ に当たるかもしれないとき, then the 要件の討議 shall ukadoc で正典を確かめた結果（出どころの URL と引用）を添えて議題にし、ukadoc で読めない点は SSP の MCP で測った結果を添える（測りは要件 8.2 に従う）
4. If 指摘の直しが開発者の裁定を覆すとき, then the 本 spec shall その指摘を直さず、議題にする
5. The 本 spec shall 直さずに残すと裁定された指摘ごとに、当たる種類と開発者の裁定を記録に残す
6. The 本 spec shall 規模の見立て（ギャップ分析の見込み 20〜24 タスク・手間 L）が大きくても 1 本のまま進め、分けず・削らず、見立ては事実として記録だけする（2026-11-16 開発者「一旦specを切り出した後にあと出して分割を提案されるのは困る」。brief の「20 を越えたら 2 本目へ切ってよい」より後の決め）

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直して関数や項目の形を変えるとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・テストの支え・examples・fixture・文書を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・テストの支え・examples・fixture・設定・スキル・手順書・steering・文字列での参照と `#[path]` を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. When バグを直すとき, the 本 spec の直し shall 直す前にそのバグで赤になるテストを立て、直した後に同じテストが緑になることを確かめる
3. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる（時刻の長さや走る順の偶然に頼らない）
4. The 本 spec のテスト shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）の中で回る
5. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
6. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
7. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない・本質の変わったテストを黙って消さない）

### Requirement 5: brief に預かった件を片づける

**Objective:** As a 開発者, I want 前の spec から預かった件が、持ち主のこの spec で片づいてほしい, so that 持ち主の無い宿題が残らない

#### Acceptance Criteria

1. The 本 spec の直し shall `areka_actor::spawn_ui` の戻り値から使われないエラー型（`UiSpawnError`）を外し、呼び手を全部直す。呼び手は `spawn_ui(`・`spawn_emo_text(`・`UiSpawnError` の grep で 4 つの crate・23 ファイル（`areka-actor` 4・`areka-emo-text` 13〔examples の 1 本を含む〕・`areka-runtime` 2〔`lib.rs` の `BootWiringError::SpawnUi` とそれを説明する文を含む〕・本体 4。一覧は `research.md` の 3 節）と、`spawn_ui` の `Err` に触れる説明の文である。本体の 1 本（`crates/areka/src/input_events/balloon_choice_settle_tests.rs`）は範囲の外（`audit-areka-input-menu` の持ち物）だが、持ち主のセッションは走っていないので、要件 3.2・7.1 のとおり申し合わせなしで直す（件 1＝`audit-sylphya-actor` の audit #16・2026-11-06 開発者の承認・ギャップ分析の 4 番）
2. The 本 spec の直し shall 揺れるテスト `spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits`（今は `crates/areka/src/runtime_tests/spine_third_scope_talk_tests.rs`）の赤の原因（待ちの形の前提）を突き止めて記録に残し、テストの本質（第三のスコープのトークが最後まで流れ、別れの挨拶で終わること）を外さずに直す（件 2・ギャップ分析の 2 番で根の形が分かった＝終わりの条件が `TalkEnded` だけを待ち、別のスレッドの seriko から届く目印の面 2100 を待っていない。直す前に、テスト専用の門つきの出力で目印の面を止めて毎回赤になる決定論のテストを立て、直した後に緑を確かめる＝要件 4.2。その赤が組み立てで立たないと分かったときは、設計の段で議題に戻す）
3. The 本 spec の直し shall `crates/areka-runtime/src/assets.rs` の `build_boot_assets` の `#[allow(dead_code)]` がまだ要るかを確かめ、要らなければ外し、要るなら理由の文を今の事実に合わせる（件 3）
4. The 本 spec の直し shall `crates/areka-runtime/src/hit_region.rs` の頭の説明の「`crate::` フリー規律」を、今の事実（examples の `#[path]` の取り込みが残っているか・`is_benign_boot_error` への本体の呼び出しが残っているか）を確かめたうえで、事実に合う文に直す（件 4）
5. The 本 spec の直し shall `runtime-crate-split` が私有から開けた項目（brief の申し送り ⑶ の 2.5・3.2・4.2 の分）ごとに、`areka-runtime` の外に実在の使い手があるかを確かめ、使い手の無いものは元の私有に戻すか消し、使われない `pub(crate)` の項目は消す（件 5・使い手の無いものは戻す・消す。議題 4 で確定）
6. The 本 spec の直し shall 番人の crate を引く補助を使うために本体に残した 17 本のテスト関数のうち、この spec の持ち物のファイルにある 14 本（`switch_cue_tests.rs` 5・`shell_balloon_resolve_tests.rs` 3・`shell_carry_tests.rs` 6）を `areka-runtime` の側へ戻す。番人の 3 つの crate は `areka-runtime` の dev-dependencies に前から入っているので、依存は足さない（`research.md` の 3 節）。ほかの spec の持ち物のファイルにある 3 本（`balloon_visibility_phase_tests.rs` の 2 本＝`audit-emo2-boot-switch`・`runtime_tests/frame/window_state_tests.rs` の 1 本＝`audit-emo2-boot-frame`）は触らず、要件 7.8 のとおり持ち主の brief へ申し送る（件 6）
7. The 本 spec shall `crates/areka-runtime/src/placement_tests/transition_judge.rs` の古い説明（47〜50 行）を自分では直さない。持ち主の `audit-placement-follow-residue` の brief（`brief.md` 109 行）に既に書かれていることを、ギャップ分析で確かめ済みとして記録に残す（件 7）
8. The 本 spec shall 本体の `runtime_tests/balloon_visibility_phase_tests.rs` の `world_at` の写しを自分では直さない。両方のファイルが `audit-emo2-boot-switch` の持ち物で、同 brief（98 行）に既に書かれていることを、ギャップ分析で確かめ済みとして記録に残す（件 8）
9. The 本 spec shall 件 1〜8 を片づけたことを、預かった brief の件の出どころ（`audit-sylphya-actor` の議題 3・`runtime-module-untangle` の申し送り・`runtime-crate-split` の申し送り ⑵ ⑶）と一緒に記録に残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録と `Err` の両方で表し、panic は続けられない致命の場合に限る）
3. The 本 spec の直し shall 終了の道（ゴーストの終了・アプリの終了・スレッドの後始末を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 順序や状態の問題を 1 フレーム遅らせて逃げず、状態の持ち方を直して同じフレームの中で解く
5. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
6. The 本 spec の直し shall 依存を足さない（C22 の約束 ⑸＝`Cargo.lock` の席は空き。外の crate も、ワークスペースの中の crate への新しい依存も含む）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする
8. The 本 spec の直し shall 外の crate をフォークも `[patch]` もせず、足りない所は公開の型で自分の側に組む

### Requirement 7: 範囲の外へ直しが及ぶとき・C22 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C22 のあいだ, the 本 spec の直し shall `crates/areka-runtime/src/frame/`・`frame*.rs` と `crates/areka-placement/` に手を入れず、emo-text に新しいソースファイルを足さない（C22 の約束 ⑷ ⑸）
4. While C22 のあいだ, the 本 spec の直し shall `crates/log-capture-kit/tests/file_length_guard_test.rs` に手を入れず、その上限越えの表に載るファイルを割らない（C22 の約束 ⑵）
5. While C22 のあいだ, the 本 spec の直し shall `crates/areka-runtime/src/placement_tests/mod.rs` を持ち物として扱い、`placement_tests/transition_*` の中身に手を入れない（C22 の約束 ⑶）
6. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める（議題 4 で確定: `areka-runtime` の公開の面とログの行の語は、読み手を同じ変更で全部直す。ゴーストと SSP 互換の振る舞いが変わる指摘は 0 件）
7. If 直しが C22 のほかの 5 本の触るファイル（ロードマップの C22 の行の「触るファイル」の ①②④⑤⑥）に及ぶとき, then the 本 spec shall そのファイルを触らずに、相手のセッションへ申し送り、申し送った件と相手を記録に残す
8. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在する（`completed/` でない）ことを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
9. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、共有の机を汚す操作・外の実物を動かす操作・重い走行が勝手に走らないでほしい, so that ほかの仕事と開発者の机が汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、静かな机を要る計測・画面の操作・開発者の areka や SSP の本物の置き場への設置を回さない
2. When 要件 2.3 のために SSP の MCP で測るとき, the 本 spec の作業 shall 測る前に開発者の許しを得て、読み直し（reload）を打つときは毎回許しを取り、排他テストの机で順を取り、測り終えたらすぐ机を返す
3. Where 実機の確かめ（emo2・pasta の実走のテスト）が全体テストの段として既に回っているところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
4. When 重い走行のテストが赤になったとき, the 本 spec の作業 shall 2 回目からは赤が出たテストだけに絞って短く回し、全体の走行を何度も回さない
5. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の SHIORI・偽の時計・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にする（決定論のテストで筋まで縛り、効き目は次の実機の確かめで見るものとして、検証の記録に指摘の番号ごとに書き残す。ギャップ分析の時点では、8.1 の操作でしか確かめられない指摘は 0 件＝`research.md` の 9 節）

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 7.6・7.8 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 7.3〜7.5 の約束が守られている（約束のファイルに差分が無い・emo-text に新しいソースファイルが無い・`Cargo.lock` に差分が無い）ことと、外への約束の変わった所が要件 7.6 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞い（ゴーストに見える振る舞い・SHIORI へ送るもの・ログの行・公開の面）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・`audit-seriko` と同じ形）
7. If かけ直しのたびに同じ所から「必ず直す」が出続けるとき, then the 本 spec shall 打ち切らずに、その所の直しを Fable で総括し直し、その結果を議題として開発者へ出す（2026-11-15 `audit-placement-core` の決め）

## 要件の討議で確定したこと（2026-11-17・全議題を推奨案で確定）

ギャップ分析（`research.md`）の後に、答えで作業が変わるものだけを議題にし、開発者が 2026-11-17 に全議題を推奨案で承認した（「推奨どおり承認」・番号はギャップ分析の指摘の番号）。起票のときの問いのうち、分け方（要件 2.6）・件 1 の申し合わせ（要件 5.1）・件 6 の依存（要件 5.6）・件 7 と件 8（要件 5.7・5.8）・揺れるテストの確かめ方（要件 5.2）・実機でしか確かめられない直し（要件 8.5）は、ギャップ分析で分かった事実と開発者の既にある決めで閉じ、本文へ書き込んだ。

1. **35 番（自前の 30 秒の輪 10 か所）**: 確定＝`wait_until` の形（進みの数で締め切りを延ばす形）へそろえて直す。完了した spec の決め R7.9（「ハイブリッドは各呼び出し点の自前の輪」）は、この承認で改めた（`drive_talk` は既にこの形）。
2. **25 番（余韻の 300 反復）**: 確定＝直さずに残す（要件 2.2 の ⑴・完了した spec の決め R2.9「製品のコードへ観測の口を作らない」）。今の形と根拠の文を残す。
3. **81・82 番（一周の台本の決め）**: 確定＝81 番（毎秒の変化通知の応答本数）は直さずに残す（⑴・task 2.2 の「段の上限だけから導き、駆動器から借りない」規則・余った応答は記録に残らず害が無い）。82 番（採取時刻の自己検査）は検査を残したまま（design D3）、中の `Vec` をやめて `any` 1 回にする形で直す。
4. **外への約束を変える直し（4・36・48・62・71 番・要件 7.6）**: 確定＝`areka-runtime` の公開の面とログの行の語は、同じ変更の中で読み手を全部直す（ログの語は、実機の確かめ・`tools/perf/`・手順書を grep で探す）。62 番は送りの失敗だけ新しい event 名へ分ける。runtime-crate-split が開けた項目のうち、本体のテストだけが使う 62 項目は使い手に数えて戻さず、外に使い手の無い 5 項目（36 番）だけ戻す。ゴーストと SSP 互換の振る舞い（(a)）が変わる指摘は 0 件。
5. **範囲の外への申し送り（`research.md` の 6 節のア〜ケ・要件 7.7・7.8）**: 確定＝走っている `audit-ukadoc-survey` へのケは、そのセッションへ伝えて記録に残す。走っていない持ち主は、実在する（`completed/` でない）ことを確かめてから brief へ書き足す（2026-11-17 に済み＝ア・イ・オの 1 つ＝`audit-emo2-boot-frame`、ウ・オの 1 つ＝`audit-emo2-boot-switch`、オの 1 つ・キ・ク＝`audit-areka-app`）。持ち主の見直しが完了済みのエ（落ちの報告の写し 3 本・まとめるには新しい依存が要る）と、97 番の着地を待つカ（`carrier_cue` の写し 15 個）は、`/kiro-complete` の未解決の棚卸で扱う件として検証の記録に残す。
