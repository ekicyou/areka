# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **kanade（ゴーストの段取りの役者）** を受け持つ（受け持つファイル＝`crates/areka-kanade/` 全部。2026-11-06 の測り直しで 146 ファイル・コード約 11,200 行・テスト約 36,100 行。まだ audit をかけていない）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件。全体テストが緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。あわせて、`audit-ghost-catalog` から申し送られた件 1（今の clippy で `-D warnings` が赤）を、振る舞いを変えずに直す。

## Introduction

この spec は、kanade（ゴーストの起動・ふだん・終了の段取りを決める役者）のクレート `crates/areka-kanade/` を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

あわせて、`audit-ghost-catalog` の完了の棚卸で申し送られた件 1（`cargo clippy -p areka-kanade --all-targets --no-deps -- -D warnings` が、本番のコードで `collapsible_if` を 1 件、テストで `type_complexity` を 5 件出して赤になる。2026-11-06 のギャップ分析で回した実測では、テストの 5 件は `type_complexity` 4 件と本番と同じ `collapsible_if` の重なり 1 件）を、この spec で直す（要件 7）。

この spec はウェーブ C18 で、ほかの 7 本（`wintf-visual-tree`・`placement-crate-split`・`audit-wintf-layout-widget`・`audit-dola`・`audit-emo-text-state-actor`・`audit-emo-text-layout-draw`・`audit-sylphya-actor`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C18 の行）を守る（要件 6）。待つ spec は無い（前に待っていた `mcp-reload` は 2026-10-22 に着地済み）。

## Boundary Context

- **In scope**:
  - `crates/areka-kanade/` の全ファイル（本番のコード・テスト・テストの道具・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 6 の約束に従う）。
  - `audit-ghost-catalog` から申し送られた件 1（kanade の clippy の `-D warnings` の赤）の直し（要件 7）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直しのうち、件 1 の外のもの（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - kanade を呼ぶのは `crates/areka`（139 ファイル）と `crates/areka-ghost`（30 ファイル）である。この spec は、この 2 つが使っている kanade の公開の口の形を変えない（足すのはよい・要件 9.9 の 2 か所だけは呼び手ごと変える）。`crates/areka/` は同じウェーブの `placement-crate-split` が書き換えるので、この spec は触らない（要件 9.9 の直しに要る 3 つのファイルだけは、同 spec と申し合わせて触る＝要件 6.1）。
  - kanade が使う `areka-talk`（`SecurityLevel`・`StartTalk`・`ScriptOrigin`）と `areka-actor`（`reply_channel`・`ReplySender`・`spawn_ui`・`ActorHandle`）の形は、同じウェーブの `audit-sylphya-actor` が変えない。この spec も、この 2 つのクレートの今ある公開の口の形を変えない（足すのはよい）。
  - kanade は dola・wintf に依らない。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `crates/areka-kanade/` だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数を測り直す
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（範囲を分けて読んだ結果をまとめた後の、3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く

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
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・fixture・設定を、範囲の外も含めてツリー全体で探し、全部直し切る（範囲の外へ及ぶときは要件 6 の約束に従う）
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる
3. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
4. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
5. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ。上限の近くにあるファイル＝`src/actor_tests.rs`・`src/schedule/steady.rs`・`src/schedule/events.rs`・`src/schedule/steady_flow_tests.rs`・`tests/kanade/steady_test.rs` には特に気を付ける）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録とエラーの戻り値の両方で表す）
3. The 本 spec の直し shall 終了の道を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 依存を足さず、`Cargo.lock` を変えない（C18 で `Cargo.lock` に触るのは `placement-crate-split` だけ）
6. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 6: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが、kanade の呼び手や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. The 本 spec の直し shall `crates/areka/` の下のファイルに手を入れない（同じウェーブの `placement-crate-split` が書き換えるため）。ただし要件 9.9 の直しに要る `crates/areka/src/input_events/`（`user_break.rs` とその呼び手のテスト）・`crates/areka/src/log_history.rs`・`crates/areka/src/mcp/sakurascript_log_tests.rs` は除く（2026-11-06 `placement-crate-split` と申し合わせ済み＝同 spec はこれらのファイルを動かさず、名前も変えず、書き換えるのは `crate::placement::` を名指す行だけ）。足す行に `crate::placement::` を書くとき、または直しが `placement` を名指すファイルへ及ぶときは、触る前に `placement-crate-split` のセッションへ知らせる
2. The 本 spec の直し shall `placement-crate-split` が触る `crates/areka/` の外のファイル（`log-capture-kit` のテスト 2 つ・wintf の `ecs/window_proc/` の 2 つ・`tools/perf/fixtures/`・`tools/perf/fixtures-loop/`）に手を入れない
3. The 本 spec の直し shall `crates/areka` と `crates/areka-ghost` が使っている kanade の公開の口（型・関数・定数）の名前・引数・戻り値・公開の有無を変えない。ただし要件 9.9 の 2 か所（`KanadeMsg::UserBreak` に欄を足す・`resource_username` を消す）は除き、呼び手を全部直す（新しい口を足すのはよい）
4. The 本 spec の直し shall `areka-talk` と `areka-actor` の今ある公開の口の形を変えない（新しい口を足すのはよい）
5. If 確定した指摘の直しが、要件 6.1〜6.4 のどれかを破らないと成り立たないとき, then the 本 spec shall その直しを行わず、議題にして開発者の裁定を仰ぐ
6. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
7. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）
8. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 7: 申し送りの clippy の赤を直す

**Objective:** As a 開発者, I want kanade の clippy を `-D warnings` で緑にしたい, so that 警告を誤りとして扱う確かめで kanade が赤にならない

#### Acceptance Criteria

1. When 直しを全部終えたとき, the `cargo clippy -p areka-kanade --all-targets --no-deps -- -D warnings` shall 警告 0 件で通る（申し送りの `collapsible_if` 1 件〔`src/shiori/real.rs:331`〕と `type_complexity` 4 件〔`src/actor_online_tests.rs` 21 行目の `spawn_with` の戻りの組ほか・申し送りの「5 件」の残り 1 件は `collapsible_if` の重なり〕を含む）
2. The 本 spec の clippy の直し shall 本番のコードとテストの振る舞いを変えない（直す前と同じテストを同じ結果で通す）

### Requirement 8: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘は数えない）
3. If かけ直しを重ねても新しい指摘が 0 件に収束しないとき, then the 検証 shall 打ち切る前に、各回の指摘の数と組の内訳を添えて開発者に知らせ、裁定を仰ぐ
4. The 検証 shall かけ直した audit の結果（新しい指摘が 0 件であること、または要件 8.3 の裁定）を記録に残す
5. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる

### Requirement 9: 要件の討議で確定した直す対象と残す指摘（2026-11-06・8 議題を推奨案で確定・議題 7 は討議のうえ開発者「承認」）

**Objective:** As a 開発者, I want 討議で決めた直す対象と、残すと裁定した指摘と、議題ごとの直し方の決めを 1 か所で引きたい, so that 設計とタスクが決めから外れない

指摘の番号は `research.md` §3 の通し番号（#1〜#96）。議題 7（#6・#62・#92）は `placement-crate-split` と申し合わせたうえで、推奨案（この spec で直す）を開発者が承認した。

#### Acceptance Criteria

1. The 本 spec shall `research.md` §3 の 96 件のうち、96 件を全部直す対象とし、3 つの組（Must fix 2・Should fix 51・Nice to have 43 のうち該当分）とも直す。#63 は 9.6 のとおり一部を残す
2. When #1 を直すとき, the kanade shall 外から来た台本の CR・LF・NUL を、SEND の入口（`schedule/sstp.rs` の `on_send`）で取り除いてから受け付ける。返事の語 `SstpSendOutcome` には値を足さない。SSP は生の改行を含む台本を断らず、改行が抜けた形で再生した（`research.md` §10）。送る点で全ての ID と Reference を検査する二段目の守りと、それに当たったときの扱いは設計で決める
3. When #3 を直すとき, the kanade shall 終了の保留中・切替の保留中に来た `RaiseEvent` と property の返しを SHIORI へ送らず、`warn!` を記録して捨てる（マウス・SSTP・台詞の切れ目と同じ決まり）。SSP では測っていない（測るには開発者の SSP のゴーストを閉じる操作が要るため）
4. When #5 を直すとき, the kanade shall 「台本があるか」の判定を 1 つの関数にまとめて 3 か所から使い、空白だけの台本も台本として扱う（SSP は空白だけの台本を再生して話し中のトークを置き換えた＝`research.md` §10）
5. When #7 を直すとき, the kanade shall 選択待ちの時間切れを期限ちょうどに出す（毎秒の刻みまで待たない）。完了 `areka-P0-choice-select-events` の設計「Tick で判定」は、大原則「時刻は正確に扱う」（2026-10-05）に合わせて改める。直しが `crates/areka-ghost` の刻みに及んでよい（持ち主は着地済み・C18 に居ない）。起こし方は設計で決める
6. The 本 spec shall #63 のうち、5 状態（`Minimizing`・`Induction`・`Passive`・`TimeCritical`・`Opening`）の語彙を、わざと置いた先送りの印（⑵・`status-execution-states` 要件 6.1・6.2 の「語彙に留める」）として残す（要件 2.5 の記録）。空の `OpeningKinds` から `opening()` が出るバグは直す
7. The 本 spec の直し shall 完了した spec の細かい決め（テストを対で置く・trace で観測する・証跡の形など）に触れる #4・#8・#14・#25・#28・#50・#51・#52・#53・#67・#68・#74・#79・#87・#88 を直す。決めの趣旨は変えない（例 #79 の 30,000 の値は残し、テストをまとめるだけ）
8. When #18（要件 7）を直すとき, the 本 spec の直し shall 本番のコードの赤に隠れていた結合テストの 11 件（`manual_contains` 7・`doc_lazy_continuation` 3・`drain_collect` 1）も直し、要件 7.1 の「警告 0 件」を結合テストまで含めて満たす
9. The 本 spec の直し shall 要件 6.1・6.3 の例外として、次の 3 件をこの spec で直す（`placement-crate-split` の着地を待たない。同 spec の着地は早くても 1〜2 週先で、触る行は重ならない）
   - #6: 利用者の中断の知らせ `KanadeMsg::UserBreak` に、押されたバルーンのトークの番号を載せる欄を足し、番号が今のトークと合うときだけ止める（合わなければ表 C）。表示の側から番号を運ぶ道がどこまで及ぶかは設計で測る（`send_break` は今 `scope` しか持たない）。呼び手＝`crates/areka/src/input_events/user_break.rs` と、`UserBreak` を組む・比べるテスト（`user_break_tests.rs`・`shell_box_handler_tests.rs` ほか）
   - #62: 素通しの `resource_username` を消し、呼び手（`crates/areka-ghost/tests/ghost/spine_e2e_test.rs` と注の `sylphya_integration_test.rs`）を `resource_get("username", ..)` に直す
   - #92: `crates/areka/src/log_history.rs` の `TARGET_SCRIPT` を `areka_kanade::script_log::TARGET` に替え、2 つの文字列の一致を縛るテスト（`crates/areka/src/mcp/sakurascript_log_tests.rs`）を消す
10. The 本 spec shall 1 本の spec のまま進め、タスクは指摘の組ごとでなくファイルのまとまりごとに束ねる。If タスクの段で 20 を大きく越えるとき, then the 本 spec shall 残る Nice to have を同じ範囲の 2 本目の spec へ回す
