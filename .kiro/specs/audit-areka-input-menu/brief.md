# Brief: audit-areka-input-menu

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の入力の出来事と右クリックメニュー** を受け持つ。

## Current State

- 受け持つファイル: `crates/areka-input/`（`src/`・`cursor/`・`Cargo.toml`）・本体の `menu*`（`menu/shortcut.rs`・`shortcut_tests.rs` を含む）・本体の `file_drop.rs`・`file_drop_*tests.rs`・`app_exit_escape_tests.rs`。`shortcut.rs`・`file_drop.rs` を範囲に残すかは、この spec の要件の段が決める。
- 大きさ: 60 ファイル・コード 6,072 行・テスト 12,976 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
- まだ audit をかけていない。指摘はギャップ分析で洗い出す（起票のときに出さなかったのは、着手までにコードが変わって指摘が古びるため）。

## Desired Outcome

- 受け持つ範囲に `/ponytail:ponytail-audit` をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。
- 直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。
- 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑。直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Approach

`/kiro-start` の各段で、次のように進める（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

- **要件**: 「受け持つ範囲の audit の指摘を、3 つの組とも全部直す」という形だけを書く。指摘の中身はまだ書かない。
- **ギャップ分析**: 受け持つ範囲だけを対象にして `/ponytail:ponytail-audit` をかける。指摘の一覧はギャップ分析の報告に残す。番号は audit の番号をそのまま使う。audit の作法のうち 2 つは外す。
  - 「最大 20 件」は外す。全件を出し、省いた件を残さない。
  - 「読まなかった所（Not checked）」は残さない。範囲のファイルは全部読む。
- **要件の討議**: 指摘の一覧を要件に落として、直す対象を確定する。直さずに残す 3 種類（「Constraints」）は、ここで議題にする。
- **設計**: 指摘ごとの直し方を決める（何を消すか・どこにまとめるか・範囲の外の呼び出し元への影響）。直し方は ponytail の「いちばん小さな完全な変更」に従う。消せるものは消し、呼び出し元・テスト・fixture まで直し切る。
- **タスク・実装**: 直す。最後に同じ範囲へ audit をかけ直して、0 件を確かめる（検証の段のタスク）。

## Scope

- **In**: 受け持つファイルの指摘の直し・そのためのテストの追加と直し・直しのために要る範囲の外の呼び出し元の直し（下の「Constraints」の約束に従う）。
- **Out**: 範囲の外の見直し（ほかの 32 本の持ち物）・新しい機能・`vendors/`・clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。

## Boundary Candidates

- 指摘の組（必ず直す・直したほうがよい・できれば）ごと、またはファイルのまとまりごとにタスクを切る。

## Out of Boundary

- ほかの 32 本の範囲のファイル（直しが及ぶときの約束は「Constraints」）。
- 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。

## Upstream / Downstream

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`input-crate-split`（2026-10-28 `/kiro-discovery`・本体の crate 分けの 4 本目）。入力の出来事はその着地で `crates/areka-input/src/` へ移り、`shortcut.rs`（`crates/areka/src/menu/shortcut.rs`）と `file_drop.rs`（`crates/areka/src/file_drop.rs`）は本体に残り、menu も本体に残る。同 spec がこの brief の「受け持つファイル」を移し先に書き換えた。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `clippy-199-lints`（同じクレートに触る）。同じ範囲に触る未着手の spec は、着手のときに roadmap の spec 台帳で確かめる。

## Constraints

- **範囲の外へ直しが及ぶとき**: 関数の形を変えたら、ツリー全体で呼び出し元を探して全部直す。相手の範囲の見直しの spec が走っていれば、そのセッションと申し合わせてから触る。
- **直さずに残してよいのは次の 3 つだけ**。どれも要件の討議で開発者の裁定を仰ぐ。
  1. 開発者の裁定に反する直し。
  2. わざと置いた先送りの印。`#[allow(dead_code)]` に実在の理由の doc を付けた語彙の口（実装規律「語彙完備・配線ゼロの追跡」）と、`shortcut:`・`ponytail:` の注釈で限界が今の負荷を越えていないものがこれに当たる。
  3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。ukadoc で確かめてから決める。
- **守る決まり**:
  - 1 ファイル 1,000 行の番人を越えない。足すなら新しいファイルへ。
  - 決定論のテストで網羅する。
  - 記録の無い失敗の道を作らない（`error!`＋`Err`）。
  - 終了の道は正規の実装にする。
  - 実機の根と一時フォルダは、ワークツリーの `target` の下に置く。
  - GPU のテストは実 GPU で回す。
  - 文字化けをコンソールの文字コードで逃げない。
  - 依存を足すなら `tech.md` へ登記して、開発者の承認を得る。
- 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
- 1 spec 20 タスクを越えそうなら、要件の討議で、指摘の組ごとに同じ範囲の 2 本目として切ってよい。

## 2026-11-01 棚卸㉝の後の開発者の決め

2026-11-01 開発者の承認（棚卸㉝の後・ツールチップは「境界領域です。リアクター描画にした方が良い可能性がある。WinUIで描画できる要素ですからね」）＝範囲から右クリックメニュー（`menu/`）の部分を外し、入力の出来事だけにする。メニューはリアクターが来れば作り直すので、見直しても捨てることになる。外した部分は windows-reactor 待ちの段の申し送り。範囲のファイルの一覧は要件の段で直す。

## 2026-11-17 棚卸㊴の申し送り（`collisionex-regions` から）

- `crates/areka/src/input_events/mod.rs` 518 行の説明「The Hand／collisionex は実装しない（7.4）」は、`collisionex-regions`（C22）が着地すると古くなる。`collisionex-regions` は areka 本体のファイルに触らない約束で C22 に入ったので、この 1 行はこの spec が直す（`input-crate-split` の後なら新しい crate の側の同じ行）。

## `input-crate-split` からの申し送り（2026-11-21 着地）

- 受け持つファイルは `crates/areka-input/src/` へ移った（中身は変えていない・パスとモジュールの名だけ書き換えた）。上の「見つけた穴」⑶ の `readme.rs:93` は `crates/areka-input/src/choice_drain.rs` へ書き換え済み。
- 古い説明（移したファイルのコメントの中身は変えていないので、見直しで拾う）:
  - `crates/areka-input/src/lib.rs` の冒頭の説明ほか、本体の中にあった頃の書き方の説明（「The Hand／collisionex」ほか）。
  - 「強制退避の枝へ落ちない」側のテスト 3 本（`input_events_tests.rs` の `handler_wired_ctrl_left_double_click_…`・`handler_left_double_click_without_ctrl_…`、`input_events_menu_tests.rs` の `wired_ctrl_left_double_click_…_partner_window`）の「窓が消えて件数が落ちる」という説明。入力の crate のテストの World には口が据わらないので、今は事実でない（合否の力は `try_recv` の表明が保つ）。
  - `input_events_tests.rs` の「強制退避は下の 2 本（結線前・Ctrl+Shift）が持つ。」は、その 3 本を本体の `app_exit_escape_tests.rs` へ移したので何も指していない。
  - `spawn_assembly_tests.rs` の 176〜181 行の「入力の側」の言い方は本体の中にあった頃のまま。
  - `crates/areka-runtime/examples/collision-probe.rs`・`collision-probe/setup.rs` の「`pub(crate)`・内部が `crate::` パスを使う」という理由の文は古い（`attach_char_pointer_handlers` は `pub` になった）。
  - `cargo doc -p areka-input --no-deps` の警告 20 件（移したコメントの「private item へのリンク」19 件・`MouseWiring::with_clock` の解けないリンク 1 件）。
- 開けた項目（狭められるかの見直しの的）: 9 モジュールと `pub mod hooks`、根の 8 項目・子の 18 項目・メソッド 7 つ、それに本体の `runtime_tests/frame_shell_box_arrow_tests.rs` が使う `user_break::drain_no_user_break_signals`。`test-support` の機能で開けたのは `pointer_test_support`・`MouseWiring::with_clock`・`user_break` の `flag_rx`・`flag_source_connected`。
- 参照: `.kiro/specs/completed/input-crate-split/design.md`・`verification.md` の「完了の申し送り」⑶。

## 2026-11-21 棚卸㊶の再測定（main `31ac3eaf`）
- **前提の変化**: `input-crate-split`（11-21・PR#486）が着地し、待つ相手は無くなった。`crates/areka/src/input_events/` は無くなり、入力の出来事は `crates/areka-input/` へ移った。`file_drop.rs` は本体の `crates/areka/src/file_drop.rs`（349 行・テスト 3 本 858 行）に、`shortcut.rs` は本体の `crates/areka/src/menu/shortcut.rs`（92 行・`shortcut_tests.rs` 262 行）に残った。`hit-id-pass`（PR#487）は `areka-input` に触っていない。
- **範囲（今の main）**: `crates/areka-input/` の `.rs` 49 ファイル 15,163 行（本番 19・4,851 行／テスト 30・10,312 行）と `Cargo.toml`・`cursor/`（`README.md` と絵 4 つ）。長いのは `lib.rs` 610・`choice_drain.rs` 545・`balloon_pointer_handler_tests.rs` 874・`user_break_tests.rs` 853。
- **範囲から外すもの（C24 に置く条件）**: 本体の `file_drop.rs`・`file_drop_*tests.rs`・`menu/shortcut*.rs`・`app_exit_escape_tests.rs`。どれも `session-module-gather` が触る（`file_drop.rs:15`・`file_drop_wiring_tests.rs:17`・`shortcut_tests.rs:16` の `crate::ghost_session` のパス・`app_exit_escape_tests.rs` は `app_exit` と一緒に移る）。外した `file_drop.rs`・`file_drop_*tests.rs`・`app_exit_escape_tests.rs` は `audit-areka-app` の受け持つファイル（本体の残り）へ移した（棚卸㊶）。`menu/shortcut*.rs` は 2026-11-01 の決め（右クリックメニューを見直しの範囲から外す）の `menu/` へ移ったので見直さない（見直すなら開発者が決める）。
- **同じウェーブの相手との約束（C24）**: 本体（`crates/areka/`）のファイルに触らない＝`areka-input` の公開の形を狭める直しは、本体が使う項目（`MouseWiring`・`user_break::drain_no_user_break_signals`〔`runtime_tests/frame_shell_box_arrow_tests.rs`〕・`attach_*`・`register_*` ほか、`session-module-gather` が書き換える 26 ファイルの呼び手）を除く。`user_break.rs:201` の `crates/areka/src/readme.rs` はパスを書かない形に直す（向こうがファイルを移すため）。`areka-runtime`・`areka-placement` は `examples/collision-probe*` の説明のほか触らない。これで C24 のほかの 3 本と 0。
- **規模**: 指摘の数で決まる（範囲は `audit-emo2-boot-frame` の約 4 割＝10〜16 の見込み・20 を越えたら指摘の組で同じ範囲の 2 本目へ）。
- **先に要るもの**: 無い（✅ `input-crate-split`）。
- **同じウェーブに置けない相手**: 範囲に本体の 4 つを残すなら `session-module-gather`・`session-crate-split`。
- **優先度の区分**: A（開発者「とにかく指摘事項は全部直す方針です」）。
- **要件定義のモデル**: Opus（見直しの型どおり）。
- **分割の案**: 無し（指摘の数しだいで要件の討議で切る）。
- **見つけた穴・古くなった記述**: ⑴ Current State の「本体の `menu*`」は 2026-11-01 の決め（menu を外す）と食い違う。「60 ファイル・6,072 行・12,976 行」も古い（上の数）。⑵ `crates/areka-input/src/lib.rs:508`「The Hand／collisionex は実装しない」は `collisionex-regions` の着地で誤り（11-17 の申し送り）。⑶ 外した `file_drop.rs`・`shortcut.rs` の預け先（上・棚卸㊶で決めた）。⑷ `audit-areka-app` の申し送りサ（`audit-emo2-boot-frame` の 6 節）の `capture_logs` の写し `crates/areka-input/src/choice_drain.rs` はこの spec が直す（棚卸㊶で持ち主へ寄せた）。
