# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・2026-11-22 に入っている版は 5.3.3・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **areka 本体の残り（起動・起動オプション・ゴーストの会話の場・SHIORI の結線・終了ほか）** を受け持つ。置き場は crate 分けの後の 2 つ＝本体（`crates/areka/`）の残りと、集めた層（`crates/areka-session/`）の大半。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。あわせて、brief に預かった件（`audit-sylphya-actor`・`placement-crate-split`・`audit-placement-core`・`runtime-crate-split`・`audit-emo2-boot-spine`・`audit-emo2-boot-frame`・`audit-areka-input-menu`・`session-module-gather`・棚卸㊷・`dola-cue-test-builder`・`session-crate-split` からの申し送り）を片づける。

## Introduction

この spec は、areka を起こしてから終えるまでの本体の道（起動オプションの読み取り・構成入力の解決・起動するゴーストとバルーンの判断・起動窓と起動の直後の結線・ゴーストの会話の場・SHIORI の結線と試験用の脳・読み直し・同じプロセスでの起こし直し・終了の統合操作・OS のセッションの終了・記憶を書けない失敗の知らせ・読み取り専用の起動・記録の履歴・投げ込み・画面を出さない取り出し）を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議でこの文書に落として直す対象を確定する。

受け持つ範囲は、areka を使う人とゴーストから見た振る舞いを決める。そのため、範囲の外から見た「外への約束」は次のとおりとする（以下「外への約束」）。

- 起動オプション（位置引数・`--ghost`・`--random`・`--option`・`--sstp-listen`・`--help`・`--offline-dump` と `--dump-*` ほか）の読み方（SSP と同じ書き方）。
- SHIORI へ送る起動・終了・切替のイベントとその参照（ukadoc が正典・SSP 互換。きれいに終わらなかった次の起動の `OnBoot` の Ref6＝`halt`・Ref7 を含む）。
- 環境変数の名（`AREKA_` 冠）と意味。
- 利用者へ出す知らせの文（告知の決まった一文・失敗を既定ゴーストの台詞で伝える道・説明書の節の名指し）。
- 走っている印（前回きれいに終わったかの印）の置き場と形。
- ログの行の語と出所（`areka::…`・`areka_session::…`。実機の確かめ・`tools/`・手順書・`get_log` の履歴の取り決め `doc/ssp-mcp/log-convention.md` が字面で読む）。
- `areka-session` の公開の面のうち、範囲の外（本体の `install/`・`update/`・`mcp/`・`roots/`・`menu/`・`runtime_tests/`、`areka-session` の `readme*`・`roots/`・`ghost_switch*`・`shell_balloon_switch*`、`areka-runtime` の口の束、examples、`crates/real-run-kit/` ほか）が使う項目。

受け持つ範囲の大きさ（brief の棚卸㊸の再測定は `main` `312a2f18`。着手のときの枝の頭 `1b3ede54` との差は steering と spec の文書だけで、`git diff --stat` で確かめた。`wc -l` で数えた）:

| 測った時点 | 範囲 | ファイル | 本番 | テスト |
|---|---|---|---|---|
| 起票のとき（`e3c2cbe4`・brief の「Current State」の値） | 本体の残り（crate 分けの前） | 211 | 12,594 行 | 18,655 行 |
| 着手のとき（`1b3ede54`） | 本体の残り（下の Boundary Context の 94 ファイル） | 94 | 22 ファイル・5,665 行 | 72 ファイル・20,400 行 |
| 着手のとき（`1b3ede54`） | `crates/areka-session/` の大半（同 74 ファイル） | 74 | 28 ファイル・6,938 行 | 46 ファイル・11,917 行 |
| 着手のとき（`1b3ede54`） | 合計 | 168 | 50 ファイル・12,603 行 | 118 ファイル・32,317 行 |

テストに数えたのは、名前が `_tests.rs`・`_test_support.rs` で終わるファイルと `tests/` の下のファイル（起票のときと同じ数え方）。本番には `Cargo.toml`・`build.rs`・`tests.manifest` を含めた。合計 168 ファイル・44,920 行は brief の棚卸㊸の値（約 168 ファイル・約 44,900 行）と合う。このほか、仮定 1 で範囲に入れる本体の `examples/`（11 ファイル）・`README.md`・`areka.manifest` の 13 ファイル・2,334 行と、絵の置き場 `shell/`（79 ファイル）・`icon/`（26 ファイル）がある。1,000 行に近いのは `tests/stay_on_top_real_run.rs`（966 行）・`main_session_mark_tests.rs`（959 行）・`areka-session/src/boot_resolve_tests.rs`（873 行）・`areka-session/src/ghost_session.rs`（862 行）・`session_tests/reload_tests.rs`（820 行）。範囲の中に、1 ファイル 1,000 行の番人の上限越えの表（`crates/log-capture-kit/tests/file_length_guard_test.rs`・範囲の外）に載るファイルは 0 本。数え直しはギャップ分析の報告でもう一度行う（要件 1.6）。

この spec はウェーブ C26 で、ほかの 2 本（`audit-areka-install-update`・`audit-emo2-boot-switch`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C26 の行の約束 ⑴〜⑷）を守る（要件 7）。

## Boundary Context

- **In scope**:
  - 本体（`crates/areka/`）の次の 94 ファイルに audit をかけて出た指摘の直し:
    - `src/` 直下のファイルのうち `roots.rs` でないもの 28 本（`main.rs`・`main_*_tests.rs`・`file_drop*`・`shiori_host*`・`shiori_session.rs`・`shiori_demo.rs`・`reference_brain.rs`・`shiori_*_e2e_tests.rs`・`perf_thread_report*`・`png_image*`・`thread_roles*`・`tick_gate_config*`・`sample_test_support.rs`・`scope_bind_fixture_test_support.rs`）。
    - `src/cli/`（11 本）・`src/offline_dump/`（11 本）。
    - `src/session_tests/` のうち `readme_body_tests.rs`・`shell_balloon_switch*` でないもの 26 本。
    - `tests/`（16 本）・`Cargo.toml`・`build.rs`。
  - 本体の `examples/`・`README.md`・`areka.manifest`（仮定 1）と、絵の置き場 `shell/`・`icon/`（中身の絵は読まず、置き場・名前・使われ方を見る＝仮定 1）。
  - `crates/areka-session/` のうち、`src/readme.rs`・`src/readme/`・`src/readme_tests.rs`・`src/roots/`・`src/ghost_switch*`・`src/shell_balloon_switch*` でないもの全部（`src/` の 71 本と `Cargo.toml`・`build.rs`・`tests.manifest`）。
  - 直しのために要るテストの追加と直し。
  - brief に預かった件と、ほかの brief からこの spec へ寄せられた件（要件 5）。範囲の外のファイルにある件も、要件 5 に挙げたものは直す。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 7 の約束に従う）。
- **Out of scope**:
  - C26 のほかの 2 本の持ち物（約束 ⑴）:
    - `audit-areka-install-update`＝本体の `install*`・`update*`・`mcp*`・`roots.rs`・`roots/`・`session_tests/readme_body_tests.rs` と、`areka-session` の `roots/`・`readme.rs`・`readme/`・`readme_tests.rs`。
    - `audit-emo2-boot-switch`＝`areka-session` の `ghost_switch*`・`shell_balloon_switch*`、本体の `session_tests/shell_balloon_switch*`・`runtime_tests/balloon_visibility*`、`areka-runtime` の受け持ち（`switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`）。
  - 本体の `src/runtime_tests/`・`src/placement_tests/`・`src/menu/` の見直し（完了した見直しの持ち物か、開発者の決めで外した所）。要件 5 に挙げた件のほかは触らない（仮定 2）。
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。
  - 新しい機能。
  - `vendors/` の下。`crates/dola/`・`crates/wintf*`（約束 ⑷）。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。`session-crate-split` が申し送った `areka-session` の `type_complexity`・`collapsible_if` ほかも同じ。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 2）。
- **Adjacent expectations**:
  - `audit-areka-install-update` は、`NO_BODY` の受け口の差（本体の系の束が据わっていないときの答え方）をそろえるかを決める。直すファイル（`areka-session/src/body_systems.rs`・本体の `session_tests/body_systems_tests.rs`・`areka-session/src/log_history.rs` の `UNKEPT_TARGETS`・`areka-session/src/reload_test_support.rs`）はこの spec が直す（約束 ⑴・要件 5.17）。
  - 本体の `install/`・`update/`・`mcp/`・`roots/` と `areka-session` の `ghost_switch*`・`shell_balloon_switch*`・`readme*`・`roots/` は、`areka-session` の公開の面（`boot_config`・`ghost_session`・`app_exit`・`log_history`・`write_failure` ほか）を使う。この spec は、そのファイルを直さずに済む形でしか公開の項目の名と形を変えない（要件 7.3）。
  - `areka-runtime` は、口の束（`session_hooks`）を通して `areka-session` の `hooks.rs` が据える中身を呼ぶ。口の束の形はこの spec の持ち物でない。
  - v0.2.0 の `dola-interrupt-ticks` ほかは、この見直しの着地を待つ（roadmap の spec 台帳）。この spec の側に約束は無い。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall Boundary Context の In scope に挙げたファイルだけを対象にして `/ponytail:ponytail-audit` をかける
2. The ギャップ分析 shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The ギャップ分析 shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない（`shell/`・`icon/` の絵は中身を読まず、置き場・名前・使われ方と説明の文との食い違いを見る＝仮定 1）
4. The ギャップ分析 shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. The ギャップ分析 shall 要件 5 の預かった件のうち、audit の指摘と重なるものを、どの番号と重なるかを書いて報告に残す
6. The ギャップ分析 shall 着手のときの `main` で範囲のファイル数と行数（本番・テストの別）を数え直し、上の表と食い違えば報告に書く
7. The ギャップ分析 shall brief と本文に書いた行番号を今の `main` に当て直し、動いていれば報告に新しい行番号を書き、もう無い件は「済み」と根拠（grep の式と 0 件）を書く
8. The ギャップ分析 shall 本体に残る SHIORI の試験用の脳（`shiori_host`・`shiori_session`・`shiori_demo`・`reference_brain`・`AREKA_SHIORI_DEMO`）について、本番の道・テスト・文書から実在の使い手があるかを数え、消せるかの判断の材料を報告に残す（brief の Upstream「消せるかはこの spec の audit で決める」・仮定 4）

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 直す対象と、直さずに残す指摘を自分で確定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. When 要件の討議で直す対象が確定したとき, the 本 spec shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか回すか）をこの文書に書き加える
2. If 指摘の直しが次の 3 種類のどれかに当たるとき, then the 本 spec shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの（起動オプションの読み方、SHIORI へ送る起動・終了・切替のイベントの名・参照・送る条件を含む）
3. When 3 種類目に当たるかを判断するとき, the 本 spec shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする。ukadoc で読めない点は、開発者の許しを得て SSP の MCP で測ってから議題にする（要件 8.2）
4. If 指摘の直しが 3 種類のどれにも当たらないとき, then the 本 spec shall その指摘を残すことを議題にせず直す
5. When 開発者が指摘を残すと裁定したとき, the 本 spec shall 裁定とその理由をこの文書に残し、要件 9 の「新しい指摘 0 件」の数から外す
6. The 本 spec shall 確定した指摘と預かった件の直しが 20 タスクを越えても、2 本目の spec に切らずに 1 本のまま進める（見込みは 24〜30 タスク＝ギャップ分析。仮定 8・討議の議題 3）
7. When 直しが外への約束のうち 3 種類目に当たらないもの（環境変数・知らせの文・走っている印・ログの行）を変えるとき, the 本 spec shall 変える所と読み手を議題に挙げて、開発者の確定を得てから直す

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that 本体の起動と終了の道からバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本 spec の直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本 spec の直し shall 要件 2.2 に当たって開発者が残すと裁定した指摘と、要件 7 で後の spec へ回すと裁定した指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出たとき, the 本 spec の直し shall その箇所を使うテスト・テストの支え・検体・examples・呼び出し元・文書まで直し切り、直しの途中の状態を残さない
4. When コード・テスト・検体・依存の行を消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・検体・設定・スキル・手順書・steering・文字列での参照と `#[path]`・マクロや型の出し直しを経た使い方を含む）で使われていないことを確かめる
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

行番号は brief の棚卸㊸の再測定（`main` `312a2f18`）のもので、着手のときの枝の頭 `1b3ede54` でもソースは変わっていない。ギャップ分析で当て直す（要件 1.7）。

#### Acceptance Criteria

1. When 記憶の置き場は書けるのに中身が使えない（TOML として壊れている・`format-version` が違う・読めない）ために保存をやめた知らせを出すとき, the areka shall 直し方の一文を「置き場を書ける状態にする」でなく「ファイルを直すか、別の名前に変える」向きの文で出し、置き場が書けないときの今の一文は変えない（`areka-session/src/alert.rs:88` の `MEMORY_WRITE_FIX`・2026-11-06 開発者の承認・`audit-sylphya-actor` の設計の討議 議題 1。文の字面は設計で決める＝仮定 9）。理由の見分けは `areka-sylphya` の `PersistFailureCause` に種類を 1 つ足して行い、理由の文字列の中身では見分けない（討議の議題 6）
2. The 本 spec の直し shall ワークスペース全体を歩く見張り（`areka-session/src/session_end_sync_send_tests.rs`・本体の `session_tests/readonly_body_tests.rs`）を、ワークスペース全体の見張りの置き場 `crates/log-capture-kit/tests/` へ移し、走査は `workspace_scan/mod.rs` を使う（`placement-crate-split` の件 1。見張りが見る中身は変えない）
3. The 本 spec の直し shall `areka-session/src/ghost_session.rs:213-214` と `boot_seam.rs:7`・`:136`・`:156` の説明が名指す記憶からの戻しの継ぎ目（`load_restored_state` → `apply_restored_placements`）を、今の継ぎ目の名に直す（`audit-placement-core` の件 5。今の名はギャップ分析で確かめる）
4. The 本 spec の直し shall `areka-session/src/boot_seam_restore_tests.rs:24`（同じ古さが `:11`・`:34`・`:118` にもある）の `plant_minimal_ghost` の説明（消えた関数と「同型」・「`resolve` が成功する」）を、今の事実に合う文に直す（同 件 8）
5. The 本 spec の直し shall `SharedFakeIo` の写し（`areka-session/src/boot_resolve_tests.rs:421-422`・`boot_seam_persist_wiring_tests.rs:13-14`）を `areka_placement::test_support::SharedFakeIo` を使う形に寄せ、本体の `runtime_tests/frame/switch_finish_tests.rs:61` の同じ名の型が写しかを確かめて、写しなら同じく寄せる（同 件 10）
6. The 本 spec の直し shall 本体の `tick_gate_config_producers_tests.rs:68` の `code_only` を `source_text_test_support::code_only` を使う形に寄せる（`audit-emo2-boot-spine` のオ・`audit-emo2-boot-frame` のオ）
7. The 本 spec の直し shall 記憶の書き手の待ち `let _ = parts.handle.join();` の 6 か所（`areka-session/src/boot_resolve_tests.rs` の 482・575・756・804 行、`boot_seam_persist_wiring_tests.rs` の 80・123 行）を、待ちに上限を持ち、書き手の panic を捨てない形（`areka_runtime::spine::join_bounded` と同じ扱い）に直す（`audit-emo2-boot-frame` のコ）
8. The 本 spec の直し shall 本体の `runtime_tests/spine.rs:123` の `capture_logs` の写しと GPU の World の写しを、上流の共通の補助（`areka-runtime` の `frame_test_support::gpu_on` ほか）を使う形に寄せる。共通の補助が本体から読めないときは、その理由を一行で残して写しを残す（同 サ）
9. The 本 spec の直し shall 本体の `menu/ghost_frame_tests.rs:43` の起動の文脈 `BootContext` を組む補助の写しを、`areka-session` の `reload_test_support` に足すゴースト名・バルーン名の引数を取る補助（`boot_context_named`）へ寄せる。ほかの 2 本の持ち物の写しは触らない（同 シ・仮定 6・2026-11-22 設計の討議の議題 2 で「寄せない」を改めた）
10. The 本 spec の直し shall `areka-session/src/app_exit_escape_tests.rs:73` の説明が消したテストの名で指す所を、役目（入力の crate の送出のテスト）で指す形に直し、`ghost_session.rs:821` の結線の説明が挙げる資源を、今の結線が挿す 4 つ（`BalloonWiring`・`BalloonHover`・`ChoiceSelectionInbox`・`ShellBoxHover`）に合わせる（`audit-areka-input-menu` の件 1・件 3）
11. The 本 spec の直し shall 本体の `menu/mod.rs:221-223`・`:254` の説明が呼び手を `ghost_session::register_systems` と書く所を、直すか残すかを見直しで決め、直すなら説明の文だけを直す（`session-module-gather` の件 2。同じ件の `mcp/mod.rs:48` は `audit-areka-install-update` の持ち物なので触らない・仮定 2）
12. The 本 spec の直し shall 本体の `runtime_tests/zorder_wiring_tests.rs` の作業番号の札つきのテスト 2 本（29 行 `t_zwi05_…`・74 行 `t_zwi08_…`）の名から札だけを外し（`the_boot_wires_the_channel_the_sink_and_the_handoff`・`the_entry_point_carries_the_shell_setting_into_the_wiring`）、同じコミットで冒頭の説明（4・5・10・11・13 行）の名指しを `test-name-work-tags` の改名表の新しい名へ直して番号の札を消す（棚卸㊷の申し送り）
13. The 本 spec の直し shall 本体の `session_tests/readonly_run_tests.rs:95`・`readonly_sites_tests.rs:119`・`write_failure_sites_tests.rs:142` の `prop_set_cue` の写しを、`areka-runtime` の `consumer_ledger_test_support::carrier_cue` へ寄せるかを見直しで決め、寄せられるものは寄せる（`dola-cue-test-builder` の申し送り。`areka-runtime`・seriko の側の包みは範囲の外）
14. The 本 spec shall `session-crate-split` が条件なしの `pub` にした項目（`GhostSession::{seriko, loop_ticker}`・`PreparedWindows::spawn`・`reload::drain`・`write_failure::drain_system`・`boot_seam::insert_persist_wiring`）を `pub` のまま残し、残す理由（どれも範囲の外の使い手は本体のテストだけだが、crate の中の本番が使うので `#[cfg]` で消せず、狭めると条件ごとの 2 つの定義か包みが要って行が増える）を記録に残す（`session-crate-split` の申し送り・要件 2.5 の裁定・討議の議題 9）
15. The 本 spec の直し shall 使われなくなった `pub(crate)` の項目を消し（ギャップ分析の時点で 0 件）、条件ごとに 2 つある `BodySystems::of` の定義を `pub fn` 1 つにまとめて私有の `installed_or_none` を消す（同・討議の議題 9）
16. The 本 spec の直し shall 本体の `session_tests/reload_tests.rs` の 2 本に足した `world.insert_resource(crate::BODY_SYSTEMS);` を、共通の足場へ寄せるかを見直しで決める。ほかの 2 本の持ち物の 6 本（`install/desk_tests.rs`・`update/desk_resolve_tests.rs`・`session_tests/shell_balloon_switch_tests.rs`）は触らない（同・約束 ⑴）
17. When `audit-areka-install-update` が `NO_BODY` の受け口の差をそろえると決めたとき, the 本 spec の直し shall その決めに従って `areka-session/src/body_systems.rs`・本体の `session_tests/body_systems_tests.rs`・`areka-session/src/reload_test_support.rs` を直し、向こうが `info!` を足すときは `areka-session/src/log_history.rs` の `UNKEPT_TARGETS` を直す（約束 ⑴・仮定 7）
18. The 本 spec の直し shall `areka-session/build.rs` が本体の `areka.manifest` を読む形にして `areka-session/tests.manifest` を消し、同じ申告の写しを無くす（`session-crate-split` の申し送り・討議の議題 10）
19. The 本 spec の直し shall 本体の `Cargo.toml` の依存の行を、`use` を経ない使い方（マクロ・型の出し直し）まで含めて本体の本番とテストで数え直し、本体が使わなくなった依存の行は外し、残す行の説明（62・66・75 行ほか、使い手が `areka-runtime` へ移って古くなったもの）を今の使い手に合わせる（`runtime-crate-split` の申し送り・約束 ⑵）
20. The 本 spec の直し shall SHIORI の試験用の脳（`shiori_host*`・`shiori_session.rs`・`shiori_demo.rs`・`reference_brain.rs`・`shiori_*_e2e_tests.rs` の 8 ファイル）と、`main.rs` の該当箇所・`AREKA_SHIORI_DEMO`・使い手の無くなる依存 `windows-core` を消し、`.kiro/steering/structure.md` の該当の記述を同じ変更で直す。消す前に、受け口を外から使うテスト（範囲の中の `session_tests/ghost_session_scope_scale_boot_tests.rs` と、範囲の外の `mcp/get_property_tests.rs`）を本番の受け口（`PropGetCueSink`）を通る形に直す。範囲の外の分は `audit-areka-install-update` のセッションと申し合わせて直してもらい、受けられないときは `shiori_host.rs` だけを残して、向こうの着地の後に消す件として申し送る（仮定 4・討議の議題 5。`AREKA_SHIORI_DEMO` が消えるのは要件 2.7 の確定として扱う＝読み手 0 件）
21. The ギャップ分析 shall brief の古い節に書かれ、棚卸㊸の再測定の「残る件」「済んだ件」のどちらにも載らない件（`audit-placement-core` の件 1〔`examples/emo-present/balloon.rs:45` の `read_balloon_offset` が片方だけの `balloon.offsetx`／`offsety` を捨てる〕・件 2・件 3・件 4・件 7・件 8 の `collision-probe/fixture.rs:23`・件 9、`audit-emo2-boot-spine` のキ〔`is_benign_boot_error` の説明〕・ク〔`examples/emo-present/setup.rs:88` の「本番（`build_boot_assets`）」〕）を今の `main` で当て直し、範囲の中にあるものはこの spec で直す。範囲の外にあるものは要件 7.10 で扱う（仮定 10）
22. The 本 spec shall 要件 5.1〜5.21 の片づけを、預かった件の出どころ（brief の節の名）と一緒に記録に残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall 1 ファイル 1,000 行の番人を越えさせず（足すなら新しいファイルへ）、番人の上限越えの表へ新しいファイルを足さない
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録と `Err` の両方で表し、panic は続けられない致命の場合に限る）
3. The 本 spec の直し shall 終了の道（アプリの終了・OS のセッションの終了・ゴーストを降ろすこと・背景の仕事の後始末・スレッドの待ちを含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 順序や状態の問題を 1 フレーム遅らせて逃げず、状態の持ち方を直して同じフレームの中で解く
5. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
6. The 本 spec の直し shall 外の crate への新しい依存を足さない
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする
8. The 本 spec の直し shall 外の crate をフォークも `[patch]` もせず、足りない所は公開の型で自分の側に組む
9. The 本 spec の直し shall 「イベントを投げ、さくらスクリプトが返ったら再生する」仕組みを崩さず、ベースウェアの側に判断やモードを足さない
10. The 本 spec の直し shall 失敗を利用者へ伝える道を、メッセージボックスでなく既定ゴーストの台詞で伝える今の形から外さない（インストールの警告のダイアログは範囲の外）
11. The 本 spec の直し shall コメントの決まり（CLAUDE.md の `## Comments`）に従い、`.kiro` の spec・要件の番号・作業番号をコードとテストの名に書かない

### Requirement 7: 範囲の外へ直しが及ぶとき・C26 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えたとき, the 本 spec の直し shall ツリー全体で呼び出し元を探して、全部直す
2. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
3. While C26 のあいだ, the 本 spec の直し shall `audit-areka-install-update`・`audit-emo2-boot-switch` の持ち物のファイル（Boundary Context の Out of scope）に手を入れず、そのファイルが使う `areka-session` の公開の項目の名と形を変えない（約束 ⑴）
4. While C26 のあいだ, the 本 spec の直し shall `crates/dola/` と `crates/wintf*` に手を入れない（約束 ⑷）
5. Where 直しが `Cargo.toml`・`Cargo.lock` を変えるところでは, the 本 spec の直し shall それを C26 でこの spec だけが持つ席として扱い、外の crate が依存から消えたときは同じ変更で `THIRD-PARTY-NOTICES.md` を作り直す（手で直さない・約束 ⑵）
6. Where 直しが `doc/COMPAT_ARCHITECTURE.md` の §8 に及ぶところでは, the 本 spec の直し shall 既にある別々の行だけを直す（約束 ⑶）
7. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更と要件 5 の件のほかは手を入れない（範囲の外の指摘を見つけても、その範囲の持ち主の件として残す）
8. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める
9. When 外への約束のうちログの行の語か出所を変えると裁定されたとき, the 本 spec の直し shall 同じ変更の中で読み手（実機の確かめ・`tools/`・手順書・`doc/ssp-mcp/log-convention.md` を grep で探したもの）を全部直す
10. If 直しや預かった件が C26 のほかの 2 本の触るファイル、または持ち主のいない範囲の外のファイルに及ぶとき, then the 本 spec shall C26 の 2 本の分は触らずに相手のセッションへ申し送り、持ち主のいない分は要件の討議で扱い（その場で説明の文だけを直す・後の spec へ回す）を決め、申し送った件と相手を記録に残す
11. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在する（`completed/` でない）ことを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
12. The 本 spec shall ギャップ分析で範囲の外に見つけた件を、その範囲の持ち主の brief に件として書き足し、回した件と回し先を記録に残す
13. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、共有の机を汚す操作・外の実物を動かす操作・重い走行が勝手に走らないでほしい, so that ほかの仕事と開発者の机が汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、静かな机を要る計測・画面の操作・開発者の areka や SSP の本物の置き場への設置を回さない
2. When 要件 2.3 のために SSP の MCP で測るとき, the 本 spec の作業 shall 測る前に開発者の許しを得て、読み直し（reload）を打つときは毎回許しを取り、排他テストの机で順を取り、測り終えたらすぐ机を返す
3. Where 実機の確かめ（`tests/` の emo2・pasta の実走・最前面・窓のつながり・起動のインストールの実走ほか）が全体テストの段として既に回っているところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
4. When 重い走行のテストが赤になったとき, the 本 spec の作業 shall 2 回目からは赤が出たテストだけに絞って短く回し、全体の走行を何度も回さない
5. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の窓・偽の SHIORI・偽の時計・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にする
6. If `tests/smoke_boot_loop_exit.rs` の 60 秒の待ちを越える揺れが作業中にまた出たとき, then the 本 spec shall 揺れを記録に残して要件の討議か開発者へ知らせ、この spec の中で `tools/load-flake.ps1` の計測を回さない（仮定 11）

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で残すと裁定された指摘と、要件 7.8・7.11 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 7.3〜7.7 の約束が守られている（C26 のほかの 2 本の持ち物のファイルと `crates/dola/`・`crates/wintf*`・`vendors/` に差分が無い・範囲の外の差分が要件 5 の件と呼び出し元の直しだけ）ことと、外への約束の変わった所が要件 2.7・7.8 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞い（起動オプション・SHIORI へ送るもの・知らせの文・走っている印・ログの行・公開の面）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・`audit-seriko` と同じ形）
7. If かけ直しのたびに同じ所から「必ず直す」が出続けるとき, then the 本 spec shall 打ち切らずに、その所の直しを Fable で総括し直し、その結果を議題として開発者へ出す（2026-11-15 `audit-placement-core` の決め）
8. If かけ直しの指摘が、前の回の直しが呼んだ連鎖（直しが次の指摘を生んだもの）であるとき, then the 本 spec shall 関連する指摘を Fable でまとめて、回をまたいで矛盾の無い形に直す。前の回と関わらない単なる追加の指摘は普通に直して閉じる（2026-11-17 開発者の決め）

## 要件の討議で確定したこと（2026-11-22・推奨どおり一括承認）

確定した指摘の一覧（要件 2.1）は、ギャップ分析（`research.md`）の 4 節の 97 件（必ず直す 2・直したほうがよい 74・できれば 21）全部を直す対象とする。開発者が残すと裁定した指摘は 0 件。下の議題の答えは、指摘の直し方の向きを決める。

| 議題 | 確定 |
|---|---|
| 1 範囲の縁 | 本体の `examples/`・`README.md`・`areka.manifest`・`shell/`・`icon/` を範囲に入れる（仮定 1 のとおり） |
| 2 見直さない所 | 本体の `runtime_tests/`・`placement_tests/`・`menu/`（`menu/shortcut.rs` を含む）は見直さず、要件 5 の件の行だけを触る（仮定 2 のとおり） |
| 3 規模 | 2 本目に切らず 1 本で進める（要件 2.6） |
| 4 直し方 | 今あるファイルの中で直すのを基本にし、移すのは全体を歩く見張り（議題 11）・本体の部品を使わないテスト（議題 12）・番人に迫るファイルだけ（ギャップ分析の案 C） |
| 5 SHIORI の試験用の脳 | 消す（要件 5.20）。`areka-ghost` の `InProcHost` が `GetProperty` を sylphya に委ねない件は範囲の外として起票の候補に回す |
| 6 記憶のファイルが使えない場面の一文 | `PersistFailureCause` に種類を 1 つ足して出し分ける（要件 5.1）。`areka-sylphya` への変更は直しに要る範囲の外の変更として扱う（要件 7.7） |
| 7 `--offline-dump` の巨大な下地（指摘 2） | 本体の `offline_dump` の側で、下地の幅か高さが合成と同じ上限を越えたら `error!` を出して全部の面を失敗として返す。`areka-emo-compose` の `shell_bounds` には上限を入れない。終了コードは設計で ukadoc に当てる |
| 8 ログの語（指摘 4・95） | 変える（`areka-session` の行の `[main]` を `[session_mark]`・`[boot_seam]` に、起動の倒れ先の記録に `event` 欄を足す）。読み手は要件 7.9 のとおり同じ変更で直す（要件 2.7 の確定） |
| 9 開けた `pub` 6 項目・`BodySystems::of` | `pub` のまま残す・`BodySystems::of` は `pub fn` 1 つ（要件 5.14・5.15） |
| 10 マニフェストの写し | `areka-session/build.rs` が本体の `areka.manifest` を読み、`tests.manifest` を消す（要件 5.18） |
| 11 全体を歩く見張り | `crates/log-capture-kit/tests/` へ移して `workspace_scan` を使う。歩く範囲は `crates/**` に広がるが、今の当たりの増えは 0 件なので要件 5.2 の「見る中身は変えない」の中とする。`readonly_body_tests.rs` の全体を歩かないテストは本体に残す |
| 12 本体の部品を使わないテスト（指摘 30・46・91） | `areka-session` の同じ口のテストへ移す |
| 13 `BootContext` の補助の写し | 寄せずに理由を 1 行残す（要件 5.9）→ 設計の討議（議題 2）で、名の引数を取る補助を足して寄せる、に改めた |
| 14 `NO_BODY` の受け口 | 仮定 7 のとおり、`audit-areka-install-update` の決めを伝言で受けてから直す。決まるまで、落ちる力の無い表明（指摘 17）は消すだけにする |
| 15 使われない絵 77 個（指摘 76） | 消す。`shell/base.png` と `shell/org/2.zip` は残す。説明の文（`icon/README.md`・`doc/pasta-profile.md`）は指摘 97 で合わせる |
| 16 `examples/mock-shell.rs`（指摘 75） | 消す。手本として名指す `menu-thumbnail-preview` の brief へ、git の履歴の場所を申し送る |
| 17 持ち主のいない範囲の外の古い説明（ギャップ分析の 7 節） | コメント・doc の文だけのものは、この spec がその場で直す（要件 5.21・7.10）。コードの直し（`areka-placement/src/persist_finalize_save_tests.rs:106` の `join`）とログの語（`areka-emo-present/src/shell_target.rs:494`）は直さずに覚え書きとして回す。C26 のほかの 2 本の分は相手のセッションへ申し送る |
| 18 先送りの印 | `perf_thread_report.rs:224` の `delta` と `offline_dump/render.rs:47-50` の `RenderError::Write` の中身は先送りの印として残す（要件 2.2 の 2 種類目）。事実と違う `DemoError::Timeout` の印は議題 5 で消える |

設計の討議（2026-11-22・推奨どおり一括承認）で、並走の兄弟から受けた次の 7 件を確定した指摘の一覧に足した（どれも範囲の中のファイル）。直し方は design.md の「兄弟から受けた件」。

- S1 `reload_test_support` にゴースト名・バルーン名の引数を取る `boot_context_named` を足す（`audit-areka-install-update` の依頼）
- S2 `alert.rs` に「この走行で告知を抑えるか」を答える公開の関数 `alerts_suppressed()` を 1 つ置く（同）
- S3 `alert.rs:150` の `ExeLocationUnavailable` の欄を読まない形にする（同）
- S4 `log_history.rs` の `snapshot` が排他を握ったまま深く写すのを、排他の外で写す形にする（同）
- S5 `png_image_tests.rs` の `wic_decode_rgba` を `pub(crate)` で共有する（同）
- S6 `mcp/sakurascript_log_tests.rs` の中身を `log_history` のテストへ移す（同。向こうがファイルを消す）
- S8 `reload.rs` の更新中・インストール中の断りを MCP からの要求だけに当て、台本の `\![reload,…]` は通す（`audit-emo2-boot-switch` の知らせ）

仮定 1〜12 は、上の確定に合わせて全部そのまま確定した（仮定 4 は「消す」、仮定 5 は議題 9 の「`pub` のまま残す」、仮定 8 は「1 本で進める」に決まった）。

## 仮定（2026-11-22 の要件の討議で確定）

brief で決まらなかった点は、次の仮定で書いた（本文の「仮定 N」）。どれも要件の討議で確定した（上の表）。

1. **範囲の縁（本体の examples ほか）**: brief は「`crates/areka/` と `crates/areka-session/` のうち、ほかの見直しの持ち物でないもの全部」と書くが、棚卸㊸の 94 ファイルの数には本体の `examples/`（11 本）・`README.md`・`areka.manifest`・`icon/`・`shell/` が入っていない。ほかに持ち主の見直しは見当たらない（`audit-emo2-boot-spine` が持ったのは `areka-runtime` の `examples/`）。仮定＝範囲に入れる（13 ファイル・2,334 行が足される）。`shell/`（brief が名指す）・`icon/` の絵は中身を読まず、置き場・名前・使われ方を見る。
2. **本体の `runtime_tests/`・`placement_tests/`・`menu/`**: 94 ファイルに入らず、完了した見直しの持ち物か、2026-11-01 の開発者の決めで見直しから外した所（`menu/`）。仮定＝見直さない。要件 5 に挙げた件（`runtime_tests/zorder_wiring_tests.rs`・`runtime_tests/spine.rs`・`runtime_tests/frame/switch_finish_tests.rs`・`menu/ghost_frame_tests.rs`・`menu/mod.rs` の説明）だけを触る。棚卸㊸の裁定 8 ⑸「本体の `menu/shortcut.rs` を見直しの範囲に入れるか（推す＝入れない）」も、この仮定（入れない）で書いた。
3. **外への約束の範囲**: 仮定＝起動オプション・SHIORI のイベント・環境変数・知らせの文・走っている印・ログの行・`areka-session` の公開の面の 7 つ。うち SSP 互換と ukadoc の正典に当たるのは起動オプションと SHIORI のイベントで、ほかは要件 2.7 で確定を得てから変える。
4. **SHIORI の試験用の脳（要件 1.8・5.20）**: brief は「消せるかはこの spec の audit で決める」と書く。消すと開発者が使う `AREKA_SHIORI_DEMO` と、`structure.md` の「SHIORI 契約チェーン e2e も同居」の記述と、e2e のテスト 3 本（`shiori_e2e_tests.rs`・`shiori_lifecycle_e2e_tests.rs`・`shiori_reference_e2e_tests.rs`）の扱いが変わる。SSP 互換・ukadoc の正典には当たらない。仮定＝audit の材料を見て議題にし、開発者の決めで消すか残す。
5. **`session-crate-split` が開けた `pub`（要件 5.14）**: 仮定＝本番の使い手が範囲の外に無い項目は `test-support` の前例どおり条件の下へ戻す。`session-crate-split` が据えた口と `test-support` の前例の中で絞るだけなので、Opus で足りる（棚卸㊸の裁定 7）。
6. **起動の文脈の補助の写し（要件 5.9）**: 棚卸㊸は「シの写しをまとめるのは 3 本の着地の後」と書く。写しは 5 か所のうち、この spec の分が `menu/ghost_frame_tests.rs:43`、`audit-areka-install-update` の分が `install/desk_tests.rs:730`・`areka-session/src/readme/opener_submit_tests.rs:31`、`audit-emo2-boot-switch` の分が `areka-session/src/ghost_switch_tests.rs:84`（`session_tests/reload_tests.rs` の分は済み）。仮定＝この spec は自分の 1 か所だけを、既にある `areka-session` の `reload_test_support::boot_context` へ寄せる（本体の `menu/` から読めない、または形が合わないときは寄せずに理由を残す）。5 か所を 1 つの補助へまとめ直すのは 3 本の着地の後（この spec ではしない）。
7. **`NO_BODY` の受け口の差（要件 5.17）**: そろえるかは `audit-areka-install-update` が決め、直すファイルはこの spec が持つ。仮定＝向こうの要件の討議か設計の決めを、セッションの伝言で受け取ってから直す。この spec の実装までに決まらなければ、決まった時点で直すタスクを最後に置き、着地までに決まらなければ向こうへ件として返す。
8. **規模（要件 2.6）**: 見込みは 22〜34 タスク（棚卸㊸）。20 を越える見込みが高い。仮定＝ギャップ分析の指摘の数を見て、要件の討議で指摘の組ごとに同じ範囲の 2 本目を切るかを決める（今は切らない・brief の Constraints のとおり）。
9. **知らせの一文（要件 5.1）**: 仮定＝出し分けの条件（理由が「中身を消さないように書くのをやめました」の場面）と文の向きだけを要件で決め、字面は設計で決める。説明書（`dist/README.txt`）の節の名指しは変えない。
10. **古い節の件で、今は範囲の外にあるもの（要件 5.21・7.10）**: 件 3・件 4・件 8 の `collision-probe/fixture.rs:23`・件 9 は、crate 分けで `areka-runtime`・`areka-placement` へ移り、持ち主の見直しは完了している（C26 のほかの 2 本の持ち物でもない）。件 2（`scg`・`atom`）は今の `areka`・`areka-session`・`areka-runtime` の `src/` で 0 件。仮定＝ギャップ分析で当て直し、まだ残る分は要件の討議で「この spec がその場で説明の文だけを直す」か「後の spec へ回す」かを決める。
11. **`smoke_boot_loop_exit` の揺れ（要件 8.6）**: `dola-cue-test-builder` が、混んだ機械の 1 回で 60 秒の待ちを越えて落ちたのを観測した（回し直しは緑）。仮定＝この spec では測らない（測ることが本体の仕事はウェーブに並べない）。audit がこのテストの待ち方を指したときは、その指摘として直す。
12. **`session-crate-split` が残した文書の食い違い**: `doc/COMPAT_ARCHITECTURE.md` の `change_tests.rs` ほか 3 件・`doc/ukadoc-coverage/briefing-assets.md`（`areka-parsers` の消えたファイルの行番号）・`doc/ideas/novel-redesign-2026-11-20/05-stage.md:55`（`areka-emo-compose` の消えたファイル）は、この範囲のコードを指さない。仮定＝範囲の外として扱い、要件 7.12 で持ち主の件か覚え書きとして回す。ただし `doc/COMPAT_ARCHITECTURE.md` §8 のうち、この範囲のテストの名を指す既にある行は、名を直したときに同じ変更で直す（約束 ⑶）。
