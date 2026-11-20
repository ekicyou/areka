# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・2026-11-20 に入っている版は 5.3.3・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の emo2_boot のうち毎フレームの組み立て** を受け持つ。受け持つファイルは、2026-11-17 `runtime-crate-split` の後の置き場で、`crates/areka-runtime/src/` の `frame/`・`frame.rs`・`frame_*.rs`（`frame_test_support.rs`・`frame_shared_test_support.rs`・`frame_balloon_runtime_test_support.rs` などのテストの支えを含む）と、本体の `crates/areka/src/runtime_tests/` の `frame/`・`frame.rs`・`frame_*.rs` と、終わる相の `crates/areka/src/session/ghost_quit.rs`・`ghost_quit_tests.rs`・`ghost_quit_logsink_tests.rs`・`ghost_quit_switch_tests.rs` である。まだ audit をかけていない。

着手のときの測り直し（2026-11-20・`main` `9bf6e32e`・brief と同じく `_tests.rs`・`_test_support.rs`・本体の `runtime_tests/` をテストに数えた・`wc -l`）: 103 ファイル・計 34,968 行・うち本番の名前のファイル 6,332 行・テスト 28,636 行。brief の棚卸㊵の実測（103 ファイル・約 35,000 行・本番 約 6,500・テスト 約 28,500）と合う。brief の「Current State」の「60 ファイル・4,845・21,175」は古い。1,000 行を越えるのは `crates/areka-runtime/src/frame_transition_branch_tests.rs`（1,271 行・1,000 行の番人の上限越えの表に載る）だけで、900 行台が 3 本（`frame_test_support.rs` 955・`frame_attach_tests.rs` 947・`frame_balloon_offset_follow_tests.rs` 947）。数え直しはギャップ分析の報告でもう一度行う（要件 1.2）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。あわせて、brief に預かった件（`runtime-module-untangle`・`audit-wintf-window`・`runtime-crate-split`・`audit-emo2-boot-spine`・`audit-placement-follow-residue` の申し送りと、棚卸㊵で `audit-areka-app` の brief から移った件）を片づける。

## Introduction

この spec は、areka の毎フレームの組み立て（フレームの相の並び・拡大率の相・窓の状態の相・Z 順の取り出し・位置の再配置・バルーンの追随・状態の報告・ゴーストの終わる相ほか）を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

受け持つ範囲は、areka を使う人から見た振る舞い（キャラクターとバルーンの窓の位置・Z 順・拡大率が変わったときの置き直し・ゴーストの終わり方）を毎フレーム決める。そのため、範囲の外から見た「外への約束」は次のとおりとする（以下「外への約束」）。

- ゴーストと SSP 互換の振る舞い（窓の位置と重なり・拡大率の変化のときの置き直し・バルーンの追随・終了と切替のときの SHIORI へのイベント）。ukadoc が正典。
- `areka-runtime` の公開の面（本体・examples・テストの支えが使う項目。`runtime-crate-split` が開けた項目を含む）。
- ログの行の語と target（実機の確かめ・`tools/perf/` の道具・手順書が字面で読む）。

この spec はウェーブ C23 で、ほかの 3 本（`hit-id-pass`・`balloon-flip-ssp-review`・`input-crate-split`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C23 の行の約束 ⑴〜⑹）を守る（要件 7）。

要件の討議（2026-11-20）で決めたことは、末尾の「要件の討議で確定したこと」にまとめた（本文の該当の所に「（確定 N）」と書いた）。指摘の番号はギャップ分析（`research.md`）の 1〜114。

## Boundary Context

- **In scope**:
  - 上の受け持つファイル全部に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - brief に預かった件（要件 5）。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3.2 と要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。例: `crates/areka-runtime/src/` の `frame*` でないもの（完了した `audit-emo2-boot-spine` と、`switch_assets*`・`move_cue*`・`change_cue*` ほかを持つ `audit-emo2-boot-switch`）、本体の `session/` の `ghost_quit*` でないもの、`crates/areka-placement/`（完了した `audit-placement-*`）。範囲の外で見つけた指摘は、その範囲の spec の持ち物として残す（要件 7.7・7.8）。
  - 新しい機能。ただし、確定した指摘の直しとして、失敗を記録して止まる道を足すことは新しい機能に数えない。
  - 同じ範囲に触る後の spec の仕事の先回り（`session-module-gather` の `runtime_tests/` の `frame*` と `session/ghost_quit*` のパスの直し・`window-resetwindowpos-hold` の `frame/move_hold.rs` の働き・`balloon-text-visual-tree` の `frame/{attach,wiring,scale_text,drain_resnap}.rs` の働き・`balloon-script-offset-align` の働き）。どれもこの spec の着地を待つ。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - areka 本体（`crates/areka/src/` の `main.rs`・`ghost_session*.rs`・`session/` のほかのファイルほか）と本体のテスト（`runtime_tests/`）は、`areka-runtime` の `frame` の公開の項目を今の名前と形で使っている。公開の形を変える直しは、本体の呼び手を同じ変更で全部直す（要件 7.6）。
  - 実機の確かめ（emo2・pasta の実走のテストと `doc/pasta-real-run.md`）と `tools/perf/` の道具は、受け持つ範囲が出すログの行の語を字面で読む。
  - 配置の crate（`crates/areka-placement/`）は、シェルの切り替えの直後に整合待ちの札（`DpiSyncHold`）のある窓の置き直しを見送り、札が外れるフレームに拡大率の相（`frame/dpi.rs`）の `DpiReproject` がその窓を新しい揃え方で書くことを前提にしている（`audit-placement-follow-residue` の申し送り・要件 5.13）。
  - 配置の crate のテストだけの公開の扱い（`runtime-crate-split` の申し送り ⑸ の表）で「公開のまま」とした項目は本番が使う語で、この spec の直しでも切り替えない。
  - 同じウェーブの `input-crate-split` は本体の `input_events/` を crate `areka-input` へ移し、本体の `main.rs` に `use areka_input as input_events;` の付け替えを残す。`balloon-flip-ssp-review` は配置の crate のバルーンの寄せ（`balloon_limit.rs`・`follow/window_move.rs` ほか）に、`hit-id-pass` は当たりの判定（`crates/areka-emo-compose/`・`crates/areka-emo-present/`・wintf の `layout/hit_region/`・場合により `crates/areka-runtime/src/hit_region.rs`）に触る（要件 7.3）。

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
6. The 本 spec shall 規模の見立て（ギャップ分析の見込み 24〜28 タスク）が 20 を越えても 1 本のまま進め、分けず・削らず、見立ては事実として記録だけする（確定 1）
7. The 本 spec の直し shall 前の決めに触れて見えた 55・64・67・68・72・80・94 番を、決めの狙いを保つ直し方で直し、直さずに残さない（確定 7）

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直して関数や項目の形を変えるとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・テストの支え・examples・fixture・文書を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・テストの支え・examples・fixture・設定・スキル・手順書・steering・文字列での参照と `#[path]` を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない
5. The 本 spec の直し shall 位置の指令の預かりの列（2 番）を、外から見た振る舞い（預かる・順を追い越さない・取りこぼさない）を変えずに、毎フレームの見直しの手間（O(n²)）だけを直す。列が伸び続ける件は要件 7.8 のとおり `window-resetwindowpos-hold` へ回す（確定 2）
6. The 本 spec の直し shall 109 番（壁時計の 30 秒の待ち）をこの spec で直し、同じ行を載せている `areka-test-wait-wording` の brief のその件を「`audit-emo2-boot-frame` で済み」に書き換える（確定 8）
7. The 本 spec の直し shall 33・99 番の本体から `areka-runtime` へのテストの戻し（約 20 本）をこの spec で行い、依存を足さない（確定 9）

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

行番号は brief の棚卸㊵の再測定（`main` `67e0c70e`）のもので、ギャップ分析で今の `main` に当て直す（要件 1.7）。

#### Acceptance Criteria

1. The 本 spec の直し shall `crates/areka-runtime/src/frame/wiring.rs` 319 行あたりの説明の文の古いパス（`super::super::input_events::balloon`）を、モジュールのパスを書かずに本体の入力の層を名前で指す文に直す（件 1＝`runtime-module-untangle` の申し送り。`input-crate-split` が同じウェーブで `input_events` を別の crate へ移すため、パスで書くとまた古びる）。同じ理由で、295 行あたりの `input_events/mod.rs` の `resolve_hit_owned` と 326 行あたりの `input_events/balloon.rs` の `on_balloon_pointer_moved` ほかを名指す文も、ファイルのパスを書かずに本体の入力の層の関数を名で指す文に直す（ギャップ分析の 37 番・2026-11-20 `input-crate-split` からの頼み）
2. The 本 spec の直し shall `crates/areka-runtime/src/frame_transition_atomicity_tests.rs` 47〜53 行と 527 行が他のファイルを行番号で指す所を、関数や項目の名前で指す形に直す（件 2＝`audit-wintf-window` の件 1）
3. The 本 spec の直し shall `frame/zorder_drain.rs` 52 行・`frame/zorder_descript.rs` 33 行・`frame/zorder_drain_tests.rs` 443 行の「grep 対象が 2 本に割れる」という理由の文を、事実に合う文に直す。事実は、`areka-runtime` の中で記録のマクロを呼ぶと出力先が `areka_runtime::…` になり、サインオフの `RUST_LOG`（`wintf::ecs::window=debug`）の前方一致の絞り込みでは debug の受理の行が点かないこと。だから行の字面と出力先を wintf の 1 か所に保つ（件 3＝`audit-wintf-window` の件 2。同 spec の指摘 34・R2-1 の文「前方一致なので漏れはしない」は wintf の中の 2 つの出力先どうしの話で、`areka-runtime` の側には当てはまらない＝ギャップ分析の 46 番）
4. The 本 spec の直し shall `frame.rs` 96〜134 行・152〜153 行・`frame/wiring.rs` 12〜13 行の `#[allow(unused_imports)]` と、`frame.rs` 171・178・188 行の許可を、`structure.md` の確かめ方（`cargo rustc -p areka-runtime --lib -- --force-warn unused_imports`）で確かめ、まだ要る許可だけを今の事実に合う理由の文つきで残し、要らない許可は外す（件 4＝`runtime-crate-split` の申し送り ⑵）
5. The 本 spec の直し shall `runtime-crate-split` が私有から開けた項目のうちこの範囲の分（brief の申し送り ⑶ の 2.5 の 10 件・4.2 ⑴〜⑷ の項目と渡し口・宣言の印の切り替え）ごとに、`areka-runtime` の外に実在の使い手があるかを確かめ、使い手の無いものは元の私有に戻すか消し、使われない `pub(crate)` の項目は消す。本体のテストだけが使う項目は使い手に数える（件 5・確定 5）
6. The 本 spec の直し shall `runtime-crate-split` がテストを関数の単位で割った後に、同じ名前のテストが `areka-runtime` と本体の両方にあるファイル（`frame/window_state_tests.rs`・`frame/status_report_tests.rs` ほか）を確かめ、同じ中身の写しが残っていれば 1 つにする（件 5 の続き）
7. The 本 spec の直し shall `frame/drain_resnap.rs` 554 行と `frame_chain_finalize_tests.rs` 204 行の「唯一の位置ライター」の文を、今の事実（`move_window_to` は位置の書き手の 1 つ）に合う文に直す（件 6＝`runtime-crate-split` の申し送り ⑷ の 1 つ目）
8. The 本 spec の直し shall `frame_work_area_resnap_hold_tests.rs` 429 行の `#[should_panic(expected = "DpiSyncHold")]` に、見張りが `debug_assertions` だけで鳴る事実に合わせて `#[cfg_attr(not(debug_assertions), ignore = "…")]` を付け、`--release` の合否が変わることを記録に残す（件 7＝同 ⑷ の 2 つ目・確定 4）
9. The 本 spec の直し shall `frame/move_shift.rs` 75 行の注を、面の `move` のずれのドラッグ中・組の相手への見送りが `debug!` で記録される今の事実に合う文に直す（件 8＝同 ⑷ の 3 つ目）
10. If 窓の側から来る scope が `frame/dpi.rs` 413〜425 行あたりで `shell_target`／`balloon_target` へ渡る前に、`audit-emo2-boot-spine` が台本の側（`target_map.rs`）に置いた上限（`i32::MAX`）を越えるとき, then the 本 spec の直し shall その番号を台本の側と同じ扱い（あふれさせず、`warn!` を残して捨てる）にそろえ、直す前にあふれで赤になるテストを立てる（件 9＝`audit-emo2-boot-spine` の申し送りア）
11. The 本 spec の直し shall `crates/areka/src/runtime_tests/frame/window_state_tests.rs` 144 行あたりのテスト関数を、補助を分けて `areka-runtime` の側へ戻す。`areka-runtime` の dev-dependencies は前からあるので依存は足さない（件 10＝同イ）
12. The 本 spec の直し shall 写し 3 つを共通の置き場の側を使う形に寄せる＝`frame/zorder_drain_tests.rs` 465 行の `code_only` を `source_text_test_support::code_only` へ・`frame_chain_finalize_tests.rs` 304 行あたりの私有の `capture_logs` を `frame_test_support::capture_logs` へ・`frame_drain_text_tests.rs` 16／25 行あたりの `controllable_clock`・`set_now` を `talk_clock.rs` の `#[cfg(test)]` の同名の補助へ（件 11＝同オと完了の時の棚卸）
13. The 本 spec の直し shall `frame/drain_resnap_tests.rs` 103 行あたりのコメントの「`load-repro.md` の ⑹」に、持ち主の spec（`areka-P0-ghost-session-test-load-flake` か `areka-test-binary-stability` のうち実際にその ⑹ を持つ方）を名乗らせる（件 12＝同完了の時の棚卸）
14. When ギャップ分析を行うとき, the ギャップ分析 shall 整合待ちの札のある窓を、札が外れるフレームに拡大率の相（`frame/dpi.rs`）の `DpiReproject` が本番の相の並びの中で本当に書くかを確かめ、結果を報告に書く（件 13＝`audit-placement-follow-residue` の申し送り・参照は `.kiro/specs/completed/audit-placement-follow-residue/design.md` の決め 5）
15. The 本 spec の直し shall 札が外れるフレームに、本番の相の並び（拡大率の相の札の付け外しと書き込み）がシェルの切り替えで見送った窓を `DpiReproject` で新しい揃え方に書くことを、決定論のテスト 1 本で縛る（ギャップ分析の 6 番。5.14 の確かめで本番は書くと分かったが、その流れを通るテストが範囲の中に 0 本だった・確定 3）
16. The 本 spec の直し shall 棚卸㊵で `audit-areka-app` の brief からこの範囲へ移った件を直す＝`frame*` に残る spec の名の略称 `atom`（`areka-P0-dpi-transition-atomicity`）を正式の名へ・`frame_diag_route_tests.rs` 19 行あたりがもう無い `areka_placement::test_support::capture_logs` を名指す所・`frame_transition_branch_tests.rs` 501 行と 527〜546 行の「4 つの見送り点」（5 つ目の `HoldSite::PatternMove` の呼び出しの綴りを固定していない）と 545 行の名前の無い「task 6.5」・`frame/drain_resnap.rs` 596 行の「ここ 1 か所だけ」（`reset_pos.rs` も呼ぶ）と 478 行の届かない `ChainDeferReason::DpiSyncHeld` の腕（件 14＝`audit-areka-app` の brief の「2026-11-15 `audit-placement-core` の完了の申し送り」の件 2〔`atom` の分〕・件 3・件 4・件 9）
17. The 本 spec shall 件 1〜14 を片づけたことを、預かった件の出どころ（`runtime-module-untangle`・`audit-wintf-window`・`runtime-crate-split` の申し送り ⑵〜⑷・`audit-emo2-boot-spine` のア・イ・オと完了の時の棚卸・`audit-placement-follow-residue`・`audit-areka-app` の brief の件 2〜4・9）と一緒に記録に残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall 上限越えの表に載っていないファイルに 1,000 行の番人を越えさせず（足すなら新しいファイルへ）、表に載る `frame_transition_branch_tests.rs` を今より長くしない
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録と `Err` の両方で表し、panic は続けられない致命の場合に限る）
3. The 本 spec の直し shall 終了の道（ゴーストの終わる相・アプリの終了・スレッドの後始末を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 順序や状態の問題を 1 フレーム遅らせて逃げず、状態の持ち方を直して同じフレームの中で解く
5. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
6. The 本 spec の直し shall 依存を足さない（C23 の約束 ⑹＝`Cargo.lock` の席は `input-crate-split`。外の crate も、ワークスペースの中の crate への新しい依存も含む）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする
8. The 本 spec の直し shall 外の crate をフォークも `[patch]` もせず、足りない所は公開の型で自分の側に組む

### Requirement 7: 範囲の外へ直しが及ぶとき・C23 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C23 のあいだ, the 本 spec の直し shall C23 のほかの 3 本の触るファイル（ロードマップの C23 の行の「触るファイル」の ①②④）に手を入れない。とくに `crates/areka-placement/` の全部・`crates/areka-runtime/src/hit_region.rs`・本体の `input_events/`・`main.rs`・`ghost_session.rs`・`ghost_session_runtime.rs`・`app_exit.rs`・`menu/`・`placement_tests/spawn_assembly_tests.rs`・`Cargo.toml`・`Cargo.lock`・`crates/log-capture-kit/src/event.rs` とその試験・`doc/COMPAT_ARCHITECTURE.md` §8 に手を入れない（確定 6）
4. Where C23 で `crates/log-capture-kit/tests/file_length_guard_test.rs` の上限越えの表に触ってよいのはこの spec だけであるところでは, the 本 spec の直し shall 表と件数の行を、この範囲のファイルが表に入る・出るときに限って直す（C23 の約束 ⑸）
5. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める
6. When 外への約束のうち `areka-runtime` の公開の面かログの行の語を変えると裁定されたとき, the 本 spec の直し shall 同じ変更の中で読み手（本体の呼び手とテスト・実機の確かめ・`tools/perf/`・手順書を grep で探したもの）を全部直す
7. If 直しが C23 のほかの 3 本の触るファイルに及ぶとき, then the 本 spec shall そのファイルを触らずに、相手のセッションへ申し送り、申し送った件と相手を記録に残す
8. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在する（`completed/` でない）ことを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
9. The 本 spec の直し shall `vendors/` の下に手を入れない
10. The 本 spec shall ギャップ分析の 6 節で範囲の外に見つけた件を、次の持ち主の brief に件として書き足し、回した件と回し先を記録に残す（確定 6）＝ア（窓の scope を作る入口の上限）は `extra-character-windows`・イ〜カ（`areka-placement` の写しと古い注）は `test-crash-report-merge`・キ・ク（`move_cue*`・`balloon_visibility_phase.rs`）は `audit-emo2-boot-switch`・オ（本体の分）・コ・サ・シは `audit-areka-app`・スは `areka-test-wait-wording`。2 番の列が伸び続ける件は `window-resetwindowpos-hold`（確定 2）。セ（`spine_wait.rs` の注）は設計で直しに要る変更として触るか回すかを決める

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、共有の机を汚す操作・外の実物を動かす操作・重い走行が勝手に走らないでほしい, so that ほかの仕事と開発者の机が汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、静かな机を要る計測・画面の操作・開発者の areka や SSP の本物の置き場への設置を回さない
2. When 要件 2.3 のために SSP の MCP で測るとき, the 本 spec の作業 shall 測る前に開発者の許しを得て、読み直し（reload）を打つときは毎回許しを取り、排他テストの机で順を取り、測り終えたらすぐ机を返す（C23 の `balloon-flip-ssp-review` も同じ机で SSP を測るので、順を取らずに測らない）
3. Where 実機の確かめ（emo2・pasta の実走のテスト）が全体テストの段として既に回っているところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
4. When 重い走行のテストが赤になったとき, the 本 spec の作業 shall 2 回目からは赤が出たテストだけに絞って短く回し、全体の走行を何度も回さない
5. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の窓・偽のモニター・偽の時計・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にする

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 7.5・7.8 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 7.3・7.4・6.6 の約束が守られている（約束のファイルに差分が無い・`Cargo.lock` に差分が無い・上限越えの表の差分がこの範囲のファイルの出入りだけ）ことと、外への約束の変わった所が要件 7.5・7.6 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞い（ゴーストに見える振る舞い・SHIORI へ送るもの・ログの行・公開の面）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・`audit-seriko` と同じ形）
7. If かけ直しのたびに同じ所から「必ず直す」が出続けるとき, then the 本 spec shall 打ち切らずに、その所の直しを Fable で総括し直し、その結果を議題として開発者へ出す（2026-11-15 `audit-placement-core` の決め）
8. If かけ直しの指摘が、前の回の直しが呼んだ連鎖（直しが次の指摘を生んだもの）であるとき, then the 本 spec shall 関連する指摘を Fable でまとめて、回をまたいで矛盾の無い形に直す。前の回と関わらない単なる追加の指摘は普通に直して閉じる（2026-11-17 開発者の決め）

## 要件の討議で確定したこと

2026-11-20 の要件の討議で、開発者が 9 つの議題を全部推奨案で承認した（「推奨どおり承認」）。番号は本文の「（確定 N）」。

1. **規模（要件 2.6）**: ギャップ分析の見込みは 24〜28 タスク。1 本のまま進め、見立ては記録だけする（2026-11-16 の決め・前例 `audit-emo2-boot-spine` の 23 タスク）。
2. **預かりの列（2 番・要件 3.5）**: この spec では振る舞いを変えずに、見直しの手間だけを直す（集合にして O(n²) を消す）。前の指令を捨てる案と上限で落とす案は取らない。理由は 2 つ。`window-work-area-clamp` の決め（2026-10-29「待つ間は順のまま預かり、取りこぼさない」）に反する。また、C23 の兄弟が触る `doc/COMPAT_ARCHITECTURE.md` §8 の行の書き換えと SSP の測りが要る。面を出さない窓への指令で列が伸び続ける件は、同じ `frame/move_hold.rs` を触る `window-resetwindowpos-hold` の brief へ申し送る（要件 7.8・7.10）。
3. **件 13 のテスト（6 番・要件 5.15）**: ギャップ分析で、本番の相の並びは札の外れるフレームにその窓を書くと分かった。けれども、その流れを通る決定論のテストは範囲の中に 0 本だった。1 本足す。
4. **`--release` の印（92 番・要件 5.8）**: 付ける。`--release` でテストを回す道（全体テスト・CI・道具）は今は無い（ギャップ分析の 3 節）。
5. **使い手の数え方（要件 5.5）**: 本体のテストだけが使う項目は使い手に数える（`audit-emo2-boot-spine` の議題 4 と同じ）。この数え方で、私有へ戻すのは `StageObservation` の 1 つ、狭めるのは 7 件（114 番）。
6. **C23 と範囲の外の件（要件 7.3・7.10）**: C23 のあいだ `crates/areka-placement/` には手を入れない。範囲の外で見つけた件は、要件 7.10 の持ち主の brief へ書き足す。どの持ち主も未着手の spec である（2026-11-20 に `completed/` でないことを確かめた）。
7. **前の決めに触れて見えた 7 件（要件 2.7）**: 55・64・67・68・72・80・94 番は直す。直し方はどれも決めの狙いを保つ。
   - 55: assert を足すだけ。
   - 64: 実 GPU のテストが常に回る。
   - 67: 網羅の檻を強める。
   - 68: 受容した残余の名を残して 1 本にする。
   - 72: `areka-P0-dpi-transition-atomicity` の要件 7.5 は「x64 の決定論テストとして走る」だけで、見張りの形を名指さない。
   - 80: 陽性の対を 1 本残す。
   - 94: 3 水準を 1 本で回す。
   - 直さずに残す指摘は 0 件。
8. **109 番の持ち主（要件 3.6）**: この spec で直し、`areka-test-wait-wording` の brief の同じ件を「済み」に書き換える。
9. **33・99 番の戻し（要件 3.7）**: この spec で行う。依存は足さない。`session-module-gather` の手間を減らす方向で、後の spec の先回りには当たらない。