# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **areka 本体のインストール・ネットワーク更新・MCP の受け口・根の一覧・説明書** を受け持つ。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。あわせて、brief に預かった件（`audit-sylphya-actor`・`runtime-module-untangle`・`session-module-gather`・`session-crate-split` の申し送りと、ほかの brief から指される件）を片づける。

## Introduction

この spec は、ゴースト・シェル・バルーン・追加ファイルの `.nar` を入れる手続き、ゴースト・シェル・バルーンのネットワーク更新、MCP のツールの要求を受けて答える口、ベースウェアの根（置き場）の並びとその編集、ゴーストの説明書を開く手続きを ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議でこの文書に落として直す対象を確定する。

受け持つ範囲は、areka を使う人から見た振る舞い（`.nar` のインストールとその確かめ・断り・失敗の伝え方、ネットワーク更新の進み方と結果、MCP のツールの名・引数・答え、置き場の並びとその足し外し、説明書を開くこと）を決める。そのため、範囲の外から見た「外への約束」は次のとおりとする（以下「外への約束」）。

- インストールとネットワーク更新に関わる SHIORI のイベントの名・参照・送る条件、受ける台本のタグとその引数（ukadoc が正典・SSP 互換）。
- MCP のツールの名・引数・答えの形（SSP の MCP と同じ形。`check_script` は areka だけのツール）。
- 置き場の並びを残す利用者のファイルの形と、置き場を決める起動の入力（環境変数・引数）。
- 説明書を開くときのファイルの決め方（ゴーストの descript の `readme` キー）と、OS の既定のアプリで開くこと。
- ログの行の語と出所（`areka::install…`・`areka::update…`・`areka::mcp…`・`areka::roots…`・`areka_session::roots…`・`areka_session::readme…`。実機の確かめ・`tools/`・手順書が字面で読み、記録の分け方〔`areka-session` の `log_history.rs`〕が出所の前方一致で読む）。
- 範囲の中の公開の項目のうち、範囲の外（本体のほかの部分・`areka-session` のほかの部分・`crates/real-run-kit/`・examples・テスト）が使う項目。

受け持つ範囲の大きさ（着手のときの `main` `1b3ede54`。brief の棚卸㊸の再測定〔`312a2f18`〕との差は文書だけ。`wc -l` で数えた）:

| 測った時点 | 範囲 | `.rs` のファイル | 本番 | テスト |
|---|---|---|---|---|
| 起票のとき（`e3c2cbe4`・brief の「Current State」の値） | 本体の `install*`・`update*`・`mcp*`・`roots*`・`readme*` ほか | 107 | 8,922 行 | 21,355 行 |
| 着手のとき（`1b3ede54`） | 本体（`crates/areka/src/` の `install/`・`update/`・`mcp/`・`roots/`・`roots.rs`・`session_tests/readme_body_tests.rs`） | 104 | 42 ファイル・8,417 行 | 62 ファイル・22,689 行 |
| 着手のとき（`1b3ede54`） | `areka-session`（`crates/areka-session/src/` の `roots/`・`readme/`・`readme.rs`・`readme_tests.rs`） | 20 | 9 ファイル・1,898 行 | 11 ファイル・3,149 行 |
| 着手のとき（合計） | | 124 | 51 ファイル・10,315 行 | 73 ファイル・25,838 行 |

テストに数えたのは、名前が `_tests.rs` で終わるファイルと、名前が `_test_support.rs` で終わるファイル（起票のときと同じ数え方）。範囲に `.rs` でないファイルは無い。1,000 行に近いのは `update/worker_tests.rs`（967 行）・`install/startup_tests.rs`（854 行）・`mcp/get_expression_table_tests.rs`（808 行）。数え直しはギャップ分析の報告でもう一度行う（要件 1.6）。

この spec はウェーブ C26 で、ほかの 2 本（`audit-areka-app`・`audit-emo2-boot-switch`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C26 の行の約束 ⑴〜⑷）を守る（要件 7）。

## Boundary Context

- **In scope**:
  - 本体（`crates/areka/src/`）の `install/`・`update/`・`mcp/`・`roots/` の全部と `roots.rs`・`session_tests/readme_body_tests.rs`、`areka-session`（`crates/areka-session/src/`）の `roots/`・`readme/` の全部と `readme.rs`・`readme_tests.rs` に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - brief に預かった件と、ほかの brief からこの spec へ寄せられた件（要件 5）。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 7 の約束に従う）。
- **Out of scope**:
  - 本体と `areka-session` のうち上に挙げなかったファイル全部。名前や中身がインストール・更新・説明書に関わっても、次のファイルは範囲の外:
    - `crates/areka-session/src/readme_script_e2e_tests.rs`（名前は `readme` で始まるが `audit-areka-app` の持ち物）。
    - 本体の `main.rs`（`BODY_SYSTEMS` を含む）・`file_drop*`・`menu/`・`session_tests/` のうち `readme_body_tests.rs` でないもの、`areka-session` の `body_systems*`・`log_history*`・`reload*`（`reload_test_support.rs` を含む）・`Cargo.toml`（`audit-areka-app` の持ち物）。
    - `areka-session` の `ghost_switch*`・`shell_balloon_switch*`、本体の `session_tests/shell_balloon_switch*`（`audit-emo2-boot-switch` の持ち物）。
  - `.nar` の容れ物・更新の取得・MCP の通信の crate（`crates/areka-nar/`・`crates/areka-update/`・`crates/areka-mcp/`・`crates/areka-mcp-bridge/`）。見直しは済んでいる（`audit-mcp-update-crates`）。
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 2）。
- **Adjacent expectations**:
  - 範囲の外から範囲の中を呼ぶ所（本体の `main.rs` の `BODY_SYSTEMS`・`file_drop.rs`・`menu/` からインストール・更新・説明書の受付を呼ぶ所、`areka-session` の起動の層から根の並びを引く所）は、`audit-areka-app` の持ち物。この spec の直しがその形を変えるときは要件 7.2 に従う。
  - 記録の分け方（`areka-session/src/log_history.rs` の `RULES`・`UNKEPT_TARGETS`）と、本体が無いときの受け口（`areka-session/src/body_systems.rs` の `NO_BODY`）は `audit-areka-app` の持ち物。この spec はその扱いを決め、本文は向こうが直す（要件 5.2・5.8）。
  - `doc-compat-architecture-rename`（`install/judge.rs` が名指す `doc/COMPAT_ARCHITECTURE.md` の名を変える spec）・`balloon-text-visual-tree`（`mcp/dump_balloon.rs` に触る spec）・`dola-interrupt-ticks`（範囲の `install_cue*`・`update_cue*`・`readme_cue*`・`mcp/dump_*_gpu_*` が使う dola の時刻と合図を変える spec）は、この見直しの着地を待つ。この spec の側に約束は無い（要件 7.4 の名指しの文書の名を変えないことのほか）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall Boundary Context の In scope に挙げた 124 ファイルだけを対象にして `/ponytail:ponytail-audit` をかける
2. The ギャップ分析 shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The ギャップ分析 shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない
4. The ギャップ分析 shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. The ギャップ分析 shall 要件 5 の預かった件のうち、audit の指摘と重なるものを、どの番号と重なるかを書いて報告に残す
6. The ギャップ分析 shall 着手のときの `main` で範囲のファイル数と行数（crate ごと・本番とテストの別）を数え直し、Introduction の表と食い違えば報告に書く
7. The ギャップ分析 shall brief と本文に書いた行番号を今の `main` に当て直し、動いていれば報告に新しい行番号を書く

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 直す対象と、直さずに残す指摘を自分で確定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. When 要件の討議で直す対象が確定したとき, the 本 spec shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか回すか）をこの文書に書き加える
2. If 指摘の直しが次の 3 種類のどれかに当たるとき, then the 本 spec shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの（インストール・ネットワーク更新の SHIORI のイベントの名・参照・送る条件、受ける台本のタグ、失敗理由の語、MCP のツールの名・引数・答えの形を含む）
3. When 3 種類目に当たるかを判断するとき, the 本 spec shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする。ukadoc で読めない点は、開発者の許しを得て SSP の MCP で測ってから議題にする（要件 8.2）
4. If 指摘の直しが 3 種類のどれにも当たらないとき, then the 本 spec shall その指摘を残すことを議題にせず直す
5. When 開発者が指摘を残すと裁定したとき, the 本 spec shall 裁定とその理由をこの文書に残し、要件 9 の「新しい指摘 0 件」の数から外す
6. The 本 spec shall 確定した指摘の直しのタスクの数が 20 を越える見込みになっても、規模だけを理由に 2 本目の spec へ切る案を推さない（見込みは 18〜28 タスク＝棚卸㊸）【仮置き（議題 6）: brief の「20 タスクを越えそうなら切ってよい」と、後の開発者の決め「20 タスクは discovery の分け方の目安で spec の上限ではない」（2026-11-16）が食い違うので、後の決めに従って 1 本のまま進める形で書いた】

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that インストール・更新・MCP・根・説明書のコードからバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本 spec の直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本 spec の直し shall 要件 2.2 に当たって開発者が残すと裁定した指摘と、要件 7 で後の spec や持ち主へ回すと裁定した指摘のほかは、直さずに残さない
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
5. Where テストが実機の根・置き場・取得したファイル・一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
6. Where テストが GPU を使うところでは（`mcp/dump_*_gpu_*` ほか）, the 本 spec のテスト shall 実 GPU で回す
7. Where テストがネットワーク更新や URL からの取得を試すところでは, the 本 spec のテスト shall 外のホストへつながず、同じ機械の中の偽の相手か偽の取得の口で試す
8. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない・本質の変わったテストを黙って消さない）

### Requirement 5: brief に預かった件を片づける

**Objective:** As a 開発者, I want 前の spec やほかの brief から預かった件が、持ち主のこの spec で片づいてほしい, so that 持ち主の無い宿題が残らない

行番号は brief の棚卸㊸の再測定（`main` `312a2f18`）と着手のときの `main` `1b3ede54` で確かめたもの。ギャップ分析で当て直す（要件 1.7）。

#### Acceptance Criteria

1. The 本 spec の直し shall `crates/areka/src/install/desk.rs` 381 行の `sylphya_publisher().barrier()`（期限の無い待ちを UI スレッドから呼ぶ）を、期限つきの待ち（`areka-sylphya` の `barrier_within`）に乗り換え、期限が切れたときは記録を残して続ける形に直す（件 1＝`audit-sylphya-actor` の audit #4・2026-11-06 開発者の承認）
2. When 直しで `update_cue`（出所 `areka::update…`）か `readme_cue`（出所 `areka_session::readme…`）に `info!` の記録を足すとき, the 本 spec shall その記録を残すか残さないか（`UNKEPT_TARGETS` へ足すか）を決め、`log_history.rs` の本文の直しは持ち主の `audit-areka-app` へ申し送る（件 2＝`runtime-module-untangle` の申し送り・棚卸㊸で出所を `areka_session::readme` に読み替え）【仮置き（議題 2）: 決めが無いあいだは、今と同じ記録の分け方を保つ（足した `info!` の出所を `UNKEPT_TARGETS` へ足す）形で書いた】
3. The 本 spec の直し shall `crates/areka/src/install/mod.rs` 132 行・`crates/areka/src/update/mod.rs` 224 行・`crates/areka/src/mcp/mod.rs` 48 行の、呼び手を `ghost_session::register_systems` と書く説明を、今の事実（名目上の渡し手は本体の `main.rs` の `BODY_SYSTEMS`）に合う文に直すか、呼び手を書かない形に直す（件 3＝`session-module-gather` の申し送りの件 2。`mcp/mod.rs` 48 行は `audit-areka-app` の brief の件 2 に挙がるが、ファイルの持ち主はこの spec）【仮置き（議題 7）: 持ち主のこの spec が直し、`audit-areka-app` のセッションへ直したことを知らせる形で書いた】
4. The 本 spec の直し shall `crates/areka/src/install/names_tests.rs` 29 行の `let _ = parts.handle.join();` を、待ちに上限を持ち、書き手の panic を捨てない形（`areka_runtime::spine::join_bounded` と同じ扱い）に直す（件 4＝棚卸㊶でほかの brief から寄せられた件）
5. The 本 spec の直し shall `crates/areka-session/src/readme/readme_cue_tests.rs` 24 行の私有の `capture_logs` の写しを、範囲の中か上流にある共通の捕捉の補助を使う形に寄せる（件 5＝同）
6. The 本 spec の直し shall `crates/areka/src/install/desk_tests.rs` 730 行と `crates/areka-session/src/readme/opener_submit_tests.rs` 31 行の `boot_context` の写しを、持ち主の違うファイル（`areka-session/src/reload_test_support.rs`・`reload.rs`・`lib.rs`・`Cargo.toml`）に手を入れずに減らせる所まで減らし、残る写しと寄せ先の候補（`reload_test_support.rs` 17 行）を記録に残す（件 6＝同。1 か所へまとめるのは C26 の 3 本の着地の後）【仮置き（議題 4）: 3 本の着地の後にまとめる仕事の持ち主が決まっていないので、この spec は記録に残すまでとし、持ち主（既にある spec の brief への書き足しか、`/kiro-discovery` での起票か）は討議で決める形で書いた】
7. The 本 spec の直し shall `world.insert_resource(crate::BODY_SYSTEMS);` の 1 行を足した 3 本のテスト（`install/desk_tests.rs` 385 行・554 行、`update/desk_resolve_tests.rs` 558 行）について、共通の足場へ寄せるかを決め、寄せるなら寄せる（件 7＝`session-crate-split` の申し送り・C26 の約束 ⑴ で持ち主のこの spec へ寄せた）【仮置き（議題 5）: 寄せ先の足場が範囲の中にあれば寄せ、範囲の外（持ち主の違う足場）にしか無ければ 1 行を残して理由を記録する形で書いた】
8. The 本 spec shall `NO_BODY`（`areka-session/src/body_systems.rs`）の受け口と、本体が「窓口が無い」ときに使う受け口（受信端の無い送出端を持つ `InstallCueSink`・`url` の腕で取得のスレッドを起こす）の差について、そろえるか・どちらへそろえるかを要件の討議で決め、直す本文（`areka-session/src/body_systems.rs`・本体の `session_tests/body_systems_tests.rs`）は持ち主の `audit-areka-app` へ申し送る（件 8＝`session-crate-split` の申し送り・C26 の約束 ⑴）【仮置き（議題 1）: ギャップ分析がこの差を読み、そろえるか否かと向きを推奨つきで議題に出す形で書いた。本番の道ではこの差を通らない（結線の誤りとテストの足場だけ）】
9. The 本 spec shall 件 1〜8 を片づけたことを、預かった件の出どころ（`audit-sylphya-actor`・`runtime-module-untangle`・`session-module-gather`・`session-crate-split`・棚卸㊶㊸）と、申し送った相手と一緒に記録に残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall 1 ファイル 1,000 行の番人を越えさせず（足すなら新しいファイルへ）、番人の上限越えの表へ新しいファイルを足さない
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録と `Err` の両方で表し、panic は続けられない致命の場合に限る）
3. The 本 spec の直し shall 終了の道（インストール・更新・フォルダ選び・説明書を開くスレッドの後始末と、アプリの終了の途中の受け口の始末を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall UI スレッドを、期限の無い待ち・取得・書き込みで塞がない（重い仕事はスレッドの外へ出す今の形を崩さない）
5. The 本 spec の直し shall 信頼の境界での確かめ（`.nar` の中身のパスと名前、取得したファイルと更新の一覧、URL、MCP の要求の引数、利用者の置き場のファイル）を緩めない
6. The 本 spec の直し shall 失敗を利用者へ伝える道を、メッセージボックスでなくゴーストの台詞（SHIORI のイベント）で伝える今の形から外さない（インストールの警告の既にある例外のほかに、新しいメッセージボックスを足さない）
7. The 本 spec の直し shall 順序や状態の問題を 1 フレーム遅らせて逃げず、状態の持ち方を直して同じフレームの中で解く
8. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
9. The 本 spec の直し shall 依存を足さない（C26 の約束 ⑵＝`Cargo.toml`・`Cargo.lock` の席は `audit-areka-app`。外の crate も、ワークスペースの中の crate への新しい依存も、`features` の足し外しも含む）
10. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする
11. The 本 spec の直し shall 外の crate をフォークも `[patch]` もせず、足りない所は公開の型で自分の側に組む
12. The 本 spec の直し shall 外のアプリや受け口を自前の設定で写さず、OS の既定のアプリ・受け口で開く今の形（説明書・フォルダ・URL）を保つ
13. The 本 spec の直し shall 「イベントを投げ、さくらスクリプトが返ったら再生する」仕組みを崩さず、ベースウェアの側に判断やモードを足さない

### Requirement 7: 範囲の外へ直しが及ぶとき・C26 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えたとき, the 本 spec の直し shall ツリー全体で呼び出し元を探して、全部直す
2. If 直しが C26 のほかの 2 本（`audit-areka-app`・`audit-emo2-boot-switch`）の持ち物のファイルに及ぶとき, then the 本 spec shall そのファイルを触らずに、相手のセッションへ申し送り、申し送った件と相手を記録に残す（C26 の約束 ⑴）【仮置き（議題 3）: 相手の spec が先に着地して走っていないときは、`main` を取り込んだうえでこの spec が自分で直し、直したことを記録に残す形で書いた】
3. The 本 spec の直し shall `Cargo.toml`（本体・`areka-session` を含むどの crate のものも）と `Cargo.lock` に手を入れない（C26 の約束 ⑵）
4. The 本 spec の直し shall `doc/COMPAT_ARCHITECTURE.md` の §8 には既にある別々の行のほかに手を入れず、範囲の中のコードが名指す文書の名とパス（`install/judge.rs` が名指す `doc/COMPAT_ARCHITECTURE.md` を含む）を変えない（C26 の約束 ⑶・`doc-compat-architecture-rename` が同じウェーブに置けない理由）
5. The 本 spec の直し shall `crates/dola/` と `crates/wintf/` に手を入れない（C26 の約束 ⑷）
6. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない（範囲の外の指摘を見つけても、その範囲の持ち主の件として残す）
7. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める
8. When 外への約束のうちログの行の語か出所を変えると裁定されたとき, the 本 spec の直し shall 同じ変更の中で読み手（実機の確かめ・`tools/`・手順書を grep で探したもの）を全部直し、記録の分け方（`log_history.rs`）の直しが要るときは持ち主へ申し送る（要件 7.2）
9. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在する（`completed/` でない）ことを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
10. The 本 spec shall ギャップ分析で範囲の外に見つけた件を、その範囲の持ち主の brief に件として書き足し、回した件と回し先を記録に残す。持ち主が C26 のほかの 2 本のときは、相手のセッションへも知らせる
11. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、共有の机を汚す操作・外の実物を動かす操作・重い走行が勝手に走らないでほしい, so that ほかの仕事と開発者の机が汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、静かな机を要る計測・画面の操作・開発者の areka や SSP の本物の置き場へのインストールや更新・外のホストからの取得・開発者が動かしている areka や SSP の MCP への要求を回さない
2. When 要件 2.3 のために SSP の MCP で測るとき, the 本 spec の作業 shall 測る前に開発者の許しを得て、読み直し（reload）を打つときは毎回許しを取り、排他テストの机で順を取り、測り終えたらすぐ机を返す
3. Where 実機の確かめ（emo2・pasta の実走のテストほか）が全体テストの段として既に回っているところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
4. When 重い走行のテストが赤になったとき, the 本 spec の作業 shall 2 回目からは赤が出たテストだけに絞って短く回し、全体の走行を何度も回さない
5. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の取得の口・偽の窓口・偽の時計・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にする

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で残すと裁定された指摘と、要件 7.7・7.9 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 7.2〜7.5 と 6.9 の約束が守られている（C26 のほかの 2 本の持ち物のファイルの差分が要件 7.2 の仮置きの分だけ・`Cargo.toml` と `Cargo.lock` に差分が無い・`crates/dola/` と `crates/wintf/` に差分が無い・名指しの文書の名が変わっていない）ことと、外への約束の変わった所が要件 7.7 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞い（SHIORI へ送るもの・インストールや更新の結果・MCP の答え・置き場のファイル・ログの行・公開の面）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・`audit-seriko` と同じ形）
7. If かけ直しのたびに同じ所から「必ず直す」が出続けるとき, then the 本 spec shall 打ち切らずに、その所の直しを Fable で総括し直し、その結果を議題として開発者へ出す（2026-11-15 `audit-placement-core` の決め）
8. If かけ直しの指摘が、前の回の直しが呼んだ連鎖（直しが次の指摘を生んだもの）であるとき, then the 本 spec shall 関連する指摘を Fable でまとめて、回をまたいで矛盾の無い形に直す。前の回と関わらない単なる追加の指摘は普通に直して閉じる（2026-11-17 開発者の決め）

## 要件の討議へ持ち越す点（仮置き）

本文の【仮置き（議題 N）】の印と対応する。どれも、決めが変わればその要件の文を書き換える。

1. `NO_BODY` と本体の「窓口が無い」ときの受け口の差を、そろえるか・どちらへそろえるか（要件 5.8）。
2. `update_cue`・`readme_cue` に `info!` を足すときに、記録を残すか（`UNKEPT_TARGETS` へ足すか）（要件 5.2）。
3. 申し送る相手の spec が先に着地したとき、この spec が自分で直してよいか（要件 7.2）。
4. `boot_context` の写しを 1 か所へまとめる仕事（C26 の 3 本の着地の後）の持ち主（要件 5.6）。
5. `BODY_SYSTEMS` の 1 行を足した 3 本のテストの寄せ先（要件 5.7）。
6. 20 タスクを越える見込みのときに 2 本目へ切るか（brief の文と 2026-11-16 の決めの食い違い・要件 2.6）。
7. `audit-areka-app` の brief に挙がる `mcp/mod.rs` 48 行の説明を、ファイルの持ち主のこの spec が直すこと（要件 5.3）。
