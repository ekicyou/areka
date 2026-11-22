# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・2026-11-22 に入っている版は 5.3.3・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の emo2_boot のうち切り替え・バルーンの見え方・ホバー・移動の cue** を受け持つ。受け持つファイルは、crate 分けが全部着地した後の置き場で、次の 3 つの crate にある（以下「受け持つファイル」）。

- `crates/areka-session/src/` の `ghost_switch*`・`shell_balloon_switch*`（17 ファイル）。
- `crates/areka-runtime/src/` の `switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`（41 ファイル）。
- 本体の `crates/areka/src/runtime_tests/` の `balloon_visibility*`（7 ファイル）と `crates/areka/src/session_tests/` の `shell_balloon_switch*`（3 ファイル）。

まだ audit をかけていない。

着手のときの測り直し（2026-11-22・`main` `1b3ede54`・brief と同じく `_tests.rs`・`_test_support.rs`・本体の `runtime_tests/`・`session_tests/` をテストに数えた・`wc -l`）: 68 ファイル・計 22,708 行・うち本番の名前のファイル 5,931 行・テスト 16,777 行。brief の棚卸㊸の実測（68 ファイル・約 22,700 行）と合う。brief の「Current State」の「50 ファイル・5,672・12,687」は古い。ギャップ分析は、名前が本番の型でも `#[path]` でテストに取り込まれる表 `balloon_visibility_timeout_suppression_cases.rs`（605 行）をテストに数え、本番 16 ファイル・5,326 行・テスト 52 ファイル・17,382 行とした。1,000 行を越えるファイルは無く、受け持つファイルは 1,000 行の番人の上限越えの表に 1 本も載っていない。上限に近いのは `crates/areka-session/src/ghost_switch_tests.rs` 982・`crates/areka-runtime/src/balloon_visibility_tests.rs` 960・`crates/areka-session/src/ghost_switch.rs` 927。数え直しはギャップ分析の報告でもう一度行う（要件 1.2）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。あわせて、brief に預かった件（`runtime-module-untangle`・`runtime-crate-split`・`audit-emo2-boot-frame`・`session-crate-split` の申し送りと、棚卸㊶・㊸でこの範囲へ寄せた件）を片づける。

## Introduction

この spec は、areka のゴーストの切り替え・シェルとバルーンの切り替え（切り替えの素材の組み立てを含む）・バルーンの見え方（出す・消す・時間切れ）・ホバーの注入・`\![move]` と `\![change,…]` の cue を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

受け持つ範囲は、areka を使う人から見た振る舞い（ゴースト・シェル・バルーンの切り替わり方と失敗のときの戻り方・バルーンが出て消える時・マウスのホバーで起きること・台本の `\![move]` で窓が着く位置）を決める。そのため、範囲の外から見た「外への約束」は次のとおりとする（以下「外への約束」）。

- ゴーストと SSP 互換の振る舞い（切り替えのときに SHIORI へ送るイベント・バルーンの時間切れと `OnBalloonTimeout` ほか・ホバーのイベント・`\![move]`・`\![change,…]` の読みと動き）。ukadoc が正典。`doc/COMPAT_ARCHITECTURE.md` の表がこの範囲の関数や定数（`parse_move_directive`・`compute_move_target`・`resolve_frame`・`effective_display_end`・`decide_timeout`・`DEFAULT_BALLOON_TIMEOUT_SECS`・環境変数 `AREKA_BALLOON_TIMEOUT_MS` ほか）を名指して、今の振る舞いを決めごととして記録している。
- `areka-runtime` と `areka-session` の公開の面（本体・examples・テストの支えが使う項目。`runtime-crate-split`・`session-crate-split` が開けた項目と、`test-support` の機能で開けた項目を含む）。
- ログの行の語と target（実機の確かめ・`tools/perf/` の道具・手順書が字面で読む）。

この spec はウェーブ C26 で、ほかの 2 本（`audit-areka-app`・`audit-areka-install-update`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C26 の行の約束 ⑴〜⑷）を守る（要件 7）。

起票の段で決め切れず要件の討議へ持ち越す問いは、末尾の「要件の討議へ持ち越す問い」に、草案で置いた仮定と一緒にまとめた（本文の該当の所に「（仮定・問い N）」と書いた）。

## Boundary Context

- **In scope**:
  - 上の受け持つファイル全部に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - brief に預かった件（要件 5）。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3.2 と要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。例: `crates/areka-session/src/` の `ghost_switch*`・`shell_balloon_switch*` でないもの（`audit-areka-app`・`audit-areka-install-update` の持ち物）、`crates/areka-runtime/src/` の受け持ちでないもの（完了した `audit-emo2-boot-spine`・`audit-emo2-boot-frame` の範囲）、本体の `runtime_tests/`・`session_tests/` の受け持ちでないもの、`crates/areka-placement/`（完了した `audit-placement-*`）。範囲の外で見つけた指摘は、その範囲の spec の持ち物として残す（要件 7.7・7.8）。
  - 新しい機能。ただし、確定した指摘の直しとして、失敗を記録して止まる道を足すことは新しい機能に数えない。
  - 同じ範囲に触る後の spec の仕事の先回り（`balloon-script-offset-align` の `move_cue*` の働き・`dola-interrupt-ticks` の時刻の型と合図の付け替え・`doc-compat-architecture-rename` の文書の名の変更・`ghost-switch-balloon-fallback` の切り替えのバルーンの決め方の変更）。どれもこの spec の着地を待つ。
  - 範囲をまたいで起動の文脈 `BootContext` を組む補助の写し 5 か所を 1 つにまとめること（`audit-areka-app` の brief の「シ」・C26 の 3 本の着地の後の仕事）。この spec は自分の範囲の 1 か所だけを片づける（要件 5.8・仮定・問い 5）。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - areka 本体（`crates/areka/src/` の `main.rs`・本体の `BODY_SYSTEMS` ほか）と `areka-session` の範囲の外のファイル（`ghost_session.rs`・`boot_*`・`reload*` ほか）は、受け持つファイルの公開の項目を今の名前と形で使っている。公開の形を変える直しは、呼び手を同じ変更で全部直す（要件 7.6）。呼び手のファイルが同じウェーブのほかの 2 本の持ち物なら、要件 7.1 のとおり申し合わせてから触る。
  - 実機の確かめ（emo2・pasta の実走のテストと `doc/pasta-real-run.md`）と `tools/perf/` の道具は、受け持つ範囲が出すログの行の語を字面で読む。
  - 移動の cue のテストは、配置の crate のテストの支え（`areka_placement::test_support::give_fake_handles` ほか）を使ってよい。配置の crate の中身は変えない。
  - 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の上限越えの表に、受け持つファイルは載っていない。この spec の直しは表に入るファイルを作らない（要件 6.1）。
  - 番人の crate（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit` ほか）は `test-support` の機能に入れられない（`session-crate-split` の決め D8 ⑵）。テストの支えをまとめる直しはこの決めの中で行う（要件 5.9・仮定・問い 3）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall Project Description の受け持つファイルだけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの一覧・数・行数（本番のコードとテストを分けて）と、使った物差しの版を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘を直すのに外への約束を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの読み手（ゴーストと SSP 互換の振る舞い・`doc/COMPAT_ARCHITECTURE.md` の行・本体と `areka-session` の呼び手・ログの行を読む実機の確かめと道具）に響くかを書く（要件 2.3・7.6 の議題の材料）
7. The ギャップ分析 shall 要件 5 の各件について、今の `main` で事実を確かめ直した結果（場所・行・今も当てはまるか）を報告に書く
8. If 要件 5 の件が audit の指摘と同じ所を指すとき, then the ギャップ分析 shall 両方を 1 つの指摘として番号を 1 つにまとめ、どの件と同じかを書く

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
6. The 本 spec shall 規模の見立て（棚卸㊸の見込み 15〜24 タスク・ギャップ分析の見立て 17〜22 タスク）が 20 を越えても 1 本のまま進め、分けず・削らず、見立ては事実として記録だけする（仮定・問い 1）

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

**Objective:** As a 開発者, I want 前の spec から預かった件と、棚卸でこの範囲へ寄せた件が、持ち主のこの spec で片づいてほしい, so that 持ち主の無い宿題が残らない

行番号は brief の棚卸㊸の再測定（`main` `312a2f18`）のもので、2026-11-22 の `main` `1b3ede54` でも同じ行にあることを確かめた。ギャップ分析で今の `main` に当て直す（要件 1.7）。

#### Acceptance Criteria

1. The 本 spec の直し shall `crates/areka-session/src/shell_balloon_switch.rs` 37〜40 行の `areka_runtime::skin_switch` の出し直しを、設計の 5 型（`SkinKind`・`SkinOrigin`・`SkinRequest`・`SkinSpec`・`SkinVerdict`）だけに絞り、残りの 5 項目（`SkinRequestRaw`・`SkinSwitchInFlight`・`SkinSwitchStage`・`skin_ref_name`・`skin_ref_path`）を引くテストと呼び手を `areka_runtime::skin_switch` から直に引く形に直す（件 1＝`runtime-module-untangle` の申し送り。5 項目のうち本番の呼び手が出し直しの道を使うものがあれば、その項目は残し、理由を記録する）
2. The 本 spec の直し shall `session-crate-split` が条件つきで開けた項目（同じファイル 43 行の `#[cfg(any(test, feature = "test-support"))]` の `SwapFinish` の出し直しと、モジュールごと開けた `ghost_switch_test_support`）ごとに、今の使い手を確かめ、使い手の無い開け方は閉じ、要る開け方は今の事実に合う理由の文で残す（件 2・仮定・問い 6）
3. The 本 spec の直し shall `runtime-crate-split` が私有から開けた項目のうちこの範囲の分（brief の申し送り ⑶ の 2.5 の 26 件〔`switch_assets.rs` は brief の 4 件に、分ける前に `pub(crate)` だった `SwapPayload`・`SwapSlot` を足した 6 件を数える〕・4.2 ⑴〜⑷ の項目と渡し口・宣言の印の切り替え）ごとに、`areka-runtime` の外に実在の使い手があるかを確かめ、使い手の無いものは元の私有に戻すか消し、使われない `pub(crate)` の項目は消す。本体のテストだけが使う項目は使い手に数える（件 3・仮定・問い 2）
4. The 本 spec の直し shall 本体の `crates/areka/src/runtime_tests/balloon_visibility_phase_tests.rs` のテスト関数 2 本（`audit-emo2-boot-spine` の申し送りウの 2 本）を、補助ごと `areka-runtime` の側へ戻し、146 行の `world_at` の写しをやめて `crates/areka-runtime/src/balloon_visibility_phase_test_support.rs` 38 行の `world_at` を使う形に寄せる。依存は足さない（件 4＝`runtime-crate-split` の申し送り ⑶ と `audit-emo2-boot-spine` の申し送りウ）
5. The 本 spec の直し shall 移動の cue のテストの偽の HWND の輪の写し（`crates/areka-runtime/src/move_cue_apply_move_tests.rs` 44 行の `attach_fake_handles`・`move_cue_place_tests.rs` の同じ形）を、`areka_placement::test_support::give_fake_handles` を使う形に寄せ、偽の記憶の書き手の写し `SharedFakeIo`（`move_cue_apply_move_sticky_tests.rs` 200 行・`move_cue_place_tests.rs` 321 行）を 1 つにする（件 5＝`audit-emo2-boot-frame` の申し送りキと棚卸㊸の件 10）
6. If `crates/areka-runtime/src/balloon_visibility_phase.rs` 464 行で窓の側から来る scope が窓の scope の上限（`crates/areka-runtime/src/target_map.rs` の `window_scope` が置く `i32::MAX`）を越えるとき, then the 本 spec の直し shall その番号をあふれさせず、`warn!` を残して読み飛ばす（`window_scope` を使う形にそろえる）。直す前にあふれで赤になるテストを立てる（件 6＝`audit-emo2-boot-frame` の申し送りク）
7. The 本 spec の直し shall `crates/areka-runtime/src/move_cue_apply_move_sticky_tests.rs` 255 行と `move_cue_place_tests.rs` 386 行の `let _ = parts.handle.join();` を、`areka_runtime::spine::join_bounded` で待ち、記憶の書き手の panic を捨てない形に直す（件 7＝`audit-emo2-boot-frame` の申し送りコの `areka-runtime` の分）
8. The 本 spec の直し shall `crates/areka-session/src/ghost_switch_tests.rs` 84 行の `boot_context` の写しを、同じ範囲の `ghost_switch_test_support.rs` へ寄せる（件 8＝棚卸㊶の「シ」のこの範囲の分。範囲をまたぐ 5 か所のまとめは Out of scope・仮定・問い 5）
9. The 本 spec の直し shall `add_rpost_shell` の 2 つの写し（`crates/areka-session/src/shell_balloon_switch_session_lap_tests.rs` 59 行・本体の `crates/areka/src/session_tests/shell_balloon_switch_session_update_tests.rs` 56 行）を、番人の crate を `test-support` に入れず、依存を足さずに 1 つにする。それができないと分かったときは、写しを残す理由を今の事実に合う文で両方に残し、要件の討議の議題にする（件 9＝`session-crate-split` の申し送り・棚卸㊸で持ち主のこの spec へ寄せた・仮定・問い 3）
10. The 本 spec の直し shall 本体の `crates/areka/src/session_tests/shell_balloon_switch_tests.rs` の 3 本（288・407・463 行）が `world.insert_resource(crate::BODY_SYSTEMS);` の 1 行を手で足している形を確かめ、テストの足場が本体の組を据える形にまとめられるならまとめ、まとめられないなら理由を記録して残す。どちらでも表明は変えない（件 10＝`session-crate-split` の決め D13 の申し送り・棚卸㊸で持ち主のこの spec へ寄せた・仮定・問い 4）
11. The 本 spec shall 件 1〜10 を片づけたことを、預かった件の出どころ（`runtime-module-untangle`・`runtime-crate-split` の申し送り ⑶・`audit-emo2-boot-spine` のウ・`audit-emo2-boot-frame` のキ・ク・コ・`session-crate-split` の申し送り・棚卸㊶の「シ」・棚卸㊸の件 10）と一緒に記録に残す
12. The 本 spec shall brief の申し送りのうち、着手の前に済んだと分かっている件（`audit-emo2-boot-spine` のオ＝`balloon_visibility_phase_zorder_chain_tests.rs` の `code_only` の写しは `source_text_test_support::code_only` を使う形になった）を、ギャップ分析で今の `main` で済んだままであることを確かめて記録だけ残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルにも 1,000 行の番人を越えさせない（足すなら新しいファイルへ。とくに上限に近い `ghost_switch_tests.rs` 982・`balloon_visibility_tests.rs` 960・`ghost_switch.rs` 927 に足すときは、先に分けるか新しいファイルへ置く）。上限越えの表に新しいファイルを入れない
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録と `Err` の両方で表し、panic は続けられない致命の場合に限る）
3. The 本 spec の直し shall 終了の道（切り替えの途中の打ち切り・ゴーストの終わり・スレッドの後始末を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 順序や状態の問題を 1 フレーム遅らせて逃げず、状態の持ち方を直して同じフレームの中で解く
5. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
6. The 本 spec の直し shall 依存を足さない（C26 の約束 ⑵＝`Cargo.toml`・`Cargo.lock` は `audit-areka-app` だけ。外の crate も、ワークスペースの中の crate への新しい依存も、機能の組み替えも含む）
7. If 直しが新しい依存か `Cargo.toml` の変更なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案（要るときだけ）を添えて開発者の承認を仰ぐ議題にする
8. The 本 spec の直し shall 外の crate をフォークも `[patch]` もせず、足りない所は公開の型で自分の側に組む

### Requirement 7: 範囲の外へ直しが及ぶとき・C26 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C26 のあいだ, the 本 spec の直し shall ほかの 2 本（`audit-areka-app`・`audit-areka-install-update`）の brief の「受け持つファイル」に載るファイルの持ち物の件を直さず、持ち主へ申し送る（C26 の約束 ⑴）
4. While C26 のあいだ, the 本 spec の直し shall `Cargo.toml`・`Cargo.lock`・`crates/dola/`・`wintf` に手を入れない（C26 の約束 ⑵・⑷）
5. Where 直しで `doc/COMPAT_ARCHITECTURE.md` の名指し（§8 を含む）を直すところでは, the 本 spec の直し shall 既にある行の、この範囲の関数や定数を名指す字面だけを直し、行を足さない（C26 の約束 ⑶）
6. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める
7. When 外への約束のうち `areka-runtime`・`areka-session` の公開の面かログの行の語を変えると裁定されたとき, the 本 spec の直し shall 同じ変更の中で読み手（本体の呼び手とテスト・`areka-session` の呼び手・実機の確かめ・`tools/perf/`・手順書・`doc/COMPAT_ARCHITECTURE.md` を grep で探したもの）を全部直す
8. If 直しが C26 のほかの 2 本の持ち物のファイルに及び、相手のセッションと申し合わせても触れないとき, then the 本 spec shall そのファイルを触らずに、相手のセッションへ申し送り、申し送った件と相手を記録に残す
9. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在する（`completed/` でない）ことを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
10. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、共有の机を汚す操作・外の実物を動かす操作・重い走行が勝手に走らないでほしい, so that ほかの仕事と開発者の机が汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、静かな机を要る計測・画面の操作・開発者の areka や SSP の本物の置き場への設置を回さない
2. When 要件 2.3 のために SSP の MCP で測るとき, the 本 spec の作業 shall 測る前に開発者の許しを得て、読み直し（reload）を打つときは毎回許しを取り、排他テストの机で順を取り、測り終えたらすぐ机を返す
3. Where 実機の確かめ（emo2・pasta の実走のテスト）が全体テストの段として既に回っているところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
4. When 重い走行のテストが赤になったとき, the 本 spec の作業 shall 2 回目からは赤が出たテストだけに絞って短く回し、全体の走行を何度も回さない
5. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の窓・偽の HWND・偽の時計・偽の SHIORI・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にする

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 7.6・7.9 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 6.1・6.6・7.3〜7.5 の約束が守られている（`Cargo.toml`・`Cargo.lock`・`crates/dola/`・`wintf` に差分が無い・上限越えの表に差分が無い・`doc/COMPAT_ARCHITECTURE.md` の差分が既にある行の字面だけ・ほかの 2 本の持ち物のファイルの差分が申し合わせた分だけ）ことと、外への約束の変わった所が要件 7.6・7.7 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞い（ゴーストに見える振る舞い・SHIORI へ送るもの・ログの行・公開の面）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・`audit-seriko` と同じ形）
7. If かけ直しのたびに同じ所から「必ず直す」が出続けるとき, then the 本 spec shall 打ち切らずに、その所の直しを Fable で総括し直し、その結果を議題として開発者へ出す（2026-11-15 `audit-placement-core` の決め）
8. If かけ直しの指摘が、前の回の直しが呼んだ連鎖（直しが次の指摘を生んだもの）であるとき, then the 本 spec shall 関連する指摘を Fable でまとめて、回をまたいで矛盾の無い形に直す。前の回と関わらない単なる追加の指摘は普通に直して閉じる（2026-11-17 開発者の決め）

## 要件の討議へ持ち越す問い

起票の段で brief から決め切れなかったもの。草案は各問いの「仮定」で書いた。要件の討議で確定したら、ここを「確定したこと」に書き換える。

1. **規模が 20 を越えたときの分け方（要件 2.6）**: brief の Constraints と棚卸㊸の再測定は「20 を越えたら指摘の組で同じ範囲の 2 本目へ切る」と書く。一方で 2026-11-16 の開発者の決め（spec を切り出した後に分割を後出しで提案しない・20 タスクは discovery の分け方の目安で spec の上限ではない）があり、同じ型の `audit-emo2-boot-spine` は 23 タスクで 1 本のまま着地した。**仮定**: 後の決めと前例に従い、1 本のまま進めて見立てを記録だけする。
2. **「使い手」の数え方（要件 5.3）**: `runtime-crate-split` が開けた項目のうち、本体のテストだけが使う項目を使い手に数えるか。**仮定**: `audit-emo2-boot-spine` の確定（本体のテストだけが使う項目は使い手に数えて戻さず、外に使い手の無い項目だけ戻す）と同じにする。要件 5.4 で本体のテストを `areka-runtime` へ戻した後は、戻したテストだけが使っていた項目は使い手を失うので、戻した後の姿で数える。
3. **`add_rpost_shell` の写しを 1 つにできないとき（要件 5.9）**: 写しは、`test-support` の機能に番人の crate（`sample-ghost-kit`）を入れられないため（`session-crate-split` の決め D8 ⑵）に残った。1 つにする形（たとえば検体の根を呼び手が取って渡し、シェルを写す 9 行だけを番人に頼らない支えへ置く）が依存も `Cargo.toml` も変えずに成り立つかは、設計で確かめるまで分からない。**仮定**: 依存と `Cargo.toml` を変えない形を先に探し、成り立てば 1 つにする。成り立たなければ、決め D8 ⑵ を覆す直しは種類 ⑴（開発者の裁定に反する直し）に当たるとして写しを理由の文つきで残し、議題にする。
4. **`BODY_SYSTEMS` の 1 行を手で足した 3 本の「直す」の中身（要件 5.10）**: 申し送りは「足したテストの一覧」で、直し方を決めていない。1 行は `session-crate-split` の決め D13 がわざと置いた形（表明を変えないための最小の 1 行）で、`update_busy`・`install_busy` の照会が真のときの断りを見るにはこの 2 ファイルが本体の組を要る。**仮定**: 同じ 1 行が同じファイルの 3 本に写っている形を、テストの足場（本体の `switch_rig` か、同じファイルの小さな補助）が据える形にまとめる。足場の側でまとめると本体の組を要らないほかのテストの振る舞いが変わるなら、まとめずに理由を記録して残す。
5. **`boot_context` の写しの寄せ方（要件 5.8・Out of scope）**: この spec の brief は「同じ範囲の `ghost_switch_test_support.rs` へ寄せる」と書き、`audit-areka-app` の brief は「シの写し（範囲をまたぐ 5 か所）をまとめるのは 3 本の着地の後」と書く。**仮定**: この spec は自分の範囲の 1 か所（`ghost_switch_tests.rs` 84 行）を自分の範囲のテストの支えへ寄せるだけにし、範囲をまたぐ 5 か所の 1 つへのまとめは C26 の後の仕事として触らない。ほかの範囲の写しが `ghost_switch_test_support.rs` の補助を使えるようになることは、ギャップ分析の報告に書いて持ち主へ渡す材料にするだけ。
6. **条件つきで開けた `SwapFinish` の出し直しと `ghost_switch_test_support` の「直す」の中身（要件 5.2）**: brief は「この範囲」と書くだけで、閉じるのか、理由を直すのかを決めていない。**仮定**: 要件 5.1 と同じ物差し（テストは `areka_runtime::skin_switch` から直に引く・使い手の無い開け方は閉じる）を当てる。`ghost_switch_test_support` は本体のテストと `areka-session` の範囲の外のテストが使う（`session-crate-split` の数えで、足場を引くファイルは範囲の中を含めて 31）ので、モジュールの開け方は残し、中の項目のうち使い手の無いものだけ閉じる。
