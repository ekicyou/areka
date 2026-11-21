# 設計の検証: input-crate-split

- 対象: `.kiro/specs/input-crate-split/design.md`（2026-11-20・design-generated）
- 照らしたもの: `requirements.md`・`research.md`・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の「本体の crate 分け」の共通の約束と C23 の行）・作業の木のコード（2026-11-20・`c779a4da2`）
- 進め方: kiro-validate-design（design-review.md の 分析 → 重大な指摘 → 良い所 → GO／NO-GO）。対話なしで判定まで行った。

## 1. まとめ

設計は前例（placement-crate-split・runtime-crate-split）の形をそのまま使い、足す仕組みを `InputHooks` 1 つに絞っている。行の番号まで測った主張は、確かめた範囲ですべてコードと合っていた（下の 2 節）。直さないと実装が止まる穴は無い。指摘 3 件は、同じウェーブとの受け渡しの決め方 1 件と、テストの説明と検証の手順の正確さ 2 件で、どれも設計の討議で 1〜2 行足せば閉じられる。**判定は GO**。

## 2. 確かめた事実（コードと照らした結果）

| 設計の主張 | 確かめた結果 |
|---|---|
| `input_events/` は 50 ファイル・移すのは 44（`mod.rs` を含む）・置き直すのは 6 本 | 合う（`ls` で 50。`shortcut{,_tests}.rs`・`file_drop{,_tests,_mime_tests,_wiring_tests}.rs` の 6 本を除くと 44） |
| 本番で本体を引くのは `mod.rs:34`・`shortcut.rs` の `crate::menu` 2 行・`file_drop.rs` の `crate::ghost_session`・`crate::install` だけ | 合う。テストを含めても、`crate::` の本体の項目は 6 本の外では `mod.rs:34` だけ（378 行は説明の文） |
| `shortcut.rs`・`file_drop.rs` は入力のほかのファイルを使わない | 合う（`super::` は自分のテストの中だけ・`crate::input_events` は 0） |
| `mod.rs` の 37 行・174〜200 行・446〜455 行・505 行・543〜553 行 | 合う（37 行の `use areka_runtime::hit_region::{…}`・505 行の `[`quit_app`]`・547 行の `quit_app(world, ExitOrigin::Escape);`） |
| `register_systems` の 67 行が `session::hooks::install`・`ghost_session.rs` 332・335 行が置き直す 2 本の呼び出し・`main.rs` 261 行が唯一の本番の呼び手 | 合う |
| 本番の依存 12 個・開発用の依存 7 個 | 合う（`areka-ghost`・`areka-emo-compose`・`areka-emo-present`・`windows` は本番のファイルが使う。`areka-parsers`・`areka-sylphya`・`areka-seriko`・`areka-emo-atlas` はテストと `*_test_support.rs` だけ。`Win32_UI_WindowsAndMessaging` はワークスペースの表に在る） |
| 本体のテストが使う入力の `#[cfg(test)]` の項目は `flag_rx`・`flag_source_connected` だけ | 合う（`balloon_test_support`・`drag_test_support`・`with_clock`・`settle_box_hover` の本体からの使い手は 0） |
| 強制退避の枝に入るテストは `input_events_tests.rs` の 3 本だけ | 合う（`ctrl_down` を立てるテストはほかに 2 か所あるが、どちらも結線済み・Shift なしで枝に入らない） |
| 約束で触らないファイル（`runtime_tests/`・`session/runtime_boot*`・`session/ghost_quit*`）は出し直しで通る | 合う（`file_drop`・`shortcut` を名指す所は 0。使う項目はどれも設計の公開の一覧に在る） |
| `include_bytes!("../../cursor/…")` → `"../cursor/…"`・`#[path]` の解け方は変わらない | 合う |
| 見張り 4 本は本体と `areka-runtime` の 2 つの `src` を歩き、入力の本番に当たりは無い見込み | 合う（`char_watch.rs` は `tick_wake::arm_deadline` だけで `mark` を使わない＝生産者の表に当たらない。ShellExecute・WinHTTP の綴りは 0） |
| `log_history` の決まりで `areka_input::…` はどれにも当たらない | 合う（前方一致は `::` の区切りで見る・`areka` は完全一致） |
| `register_systems` を素の World で呼ぶテストの前例 | 在る（`ghost_session_reload_tests.rs` 67〜71 行・`ghost_session_restart_tests.rs` 449〜461 行。後者は `SessionHooks` が据わったことを同じテストの中で確かめている） |
| 出所を名前で読む所 4 か所・入力を名指すほかのファイルの一覧 | 合う（`git grep` の結果が設計の「変えるファイル」と一致。`tech.md` は 0） |
| 1,000 行 | 足した後も上限の内（`main.rs` 989→991・`update/worker_tests.rs` 964→数行増） |

## 3. 重大な指摘（3 件）

### 指摘 1: `hit-id-pass` との受け渡しが、同じウェーブの約束 ⑶ と噛み合っていない

**懸念**: 設計は「`hit-id-pass` が `resolve_hit_region` の形を変えたら、後に着地する側が `crates/areka-input/src/lib.rs` で直す」とする（見直しの引き金・危うさ 1・OPEN QUESTIONS 6）。しかし roadmap の C23 の約束 ⑶ は `hit-id-pass` に「本体に触らない」を課している。呼び手は今は本体の `input_events/mod.rs`（37 行の `use`・`MouseWiring::resolve_region`・`resolve_hit_owned`）にあるので、

- `hit-id-pass` が先に着地して形を変えると、本体の呼び手に触る＝⑶ に当たる。
- この仕様が先に着地すると、呼び手は新しい crate に在り、⑶ がそこへの手入れを許すかが書かれていない（`areka-input` は「本体」か）。

**影響**: どちらが先に着地しても、後の側が実装の段で「約束に当たるか」で止まりうる。設計の計画（入口の行の中身を変えない・行の単位で書き換える）そのものは正しく、変えるのは受け渡しの決めだけ。

**提案**: 設計の討議で `hit-id-pass` のセッションに相談し（持ち場の事実は持ち主へ先に聞く）、次のどちらかを「同じウェーブとの重なり」に 1 行で書く。
1. `hit-id-pass` は `resolve_hit_region` と `HitRegion` の形を保つ（入口の呼び手に触らない）。
2. 形を変えるなら、呼び手の 3 か所の追従は `hit-id-pass` が行い、⑶ の「本体」に `crates/areka-input/` を含めない読みを両方の spec に明記する（この仕様が先に着地した場合）。この仕様が後なら、rebase のときにこの仕様が新しい置き場で直す。

**関わる所**: 設計「見直しの引き金」3 つ目・「同じウェーブとの重なり」の `hit-id-pass` の項・「申し送りと危うさ」の危うさ 1・OPEN QUESTIONS 6。

### 指摘 2: 入力に残るテストのうち 3 本の「非空虚性」の説明が、移した後に事実でなくなる

**懸念**: 入力の crate のテストの実行ファイルには口（`InputHooks`）が据わらない。そのため、誤って強制退避の枝へ落ちても、D2 の倒れ方（`error!(input_hooks_missing)`＋終わりの指示・窓は閉じない）になる。ところが入力に残る次のテストは、「強制退避の腕へ落ちれば窓が消えて件数の表明が落ちる」を自分の守りの理由に挙げている。

- `input_events_tests.rs` の `handler_wired_ctrl_left_double_click_sends_no_close_request_and_delivers_the_double_click`（433〜435 行の「# 非空虚性」）
- `input_events_menu_tests.rs` の `wired_ctrl_left_double_click_sends_no_close_request_on_the_partner_window`（161〜162 行）
- `input_events_tests.rs` の `handler_left_double_click_without_ctrl_does_not_despawn_and_sends`（「Ctrl なしは despawn しない」の表明）

**影響**: 合否の力は保たれる。どのテストも `rx.try_recv()` の表明を持ち、枝へ落ちれば送出が無いので赤になる（実害は 0）。ただし、窓の件数の表明はその枝を見分けなくなり、説明の文は古くなる。設計の「入力のほかのテストで強制退避の枝に入るものは 0」は正しいが、「入らないことを何が見張るか」が移す前と変わることが書かれていない。

**提案**（テストの中身は変えない・要件 8.1 の範囲）:
1. Handover ⑶（`audit-areka-input-menu` へ申し送る古い説明の一覧）に、上の 3 本の非空虚性の文を足す。
2. 設計の「今の形」の強制退避の段落に 1 行足す: 「入力に残るテストが強制退避の枝へ誤って落ちたことを捕まえるのは `try_recv` の表明で、窓の件数の表明は移した後はその枝を見分けない」。タスクの段のレビューの観点に回す。

入力のテストの World に記録だけの口を据える案は、要件 5.1（本体の部品を偽物に替えない）とテストの中身を変えることに当たるので推さない。

### 指摘 3: 落ちの報告の確かめの grep が、手順どおりに数えると外れる

**懸念**: TestBinaryGuard の確かめは「初期化子の `static` が `#[cfg(test)]` の下にしか無いことを grep で示す（`crates/areka-runtime/src/crash_report_test_support.rs`・`crates/areka/src/runtime_tests/mod.rs`・`crates/areka-input/src/crash_report_tests.rs` の 3 か所だけ）」とする。しかし `.CRT$XCU` の初期化子は、`crates/areka-placement/src/test_crash_report.rs:128` と `crates/wintf/src/test_crash_report.rs:128` にも在る（それぞれの写し・統合は `test-crash-report-merge` の持ち物）。また `runtime_tests/mod.rs` の `static` は自分では `#[cfg(test)]` を持たず、`main.rs` 52 行の `#[cfg(test)] mod runtime_tests;` の下で `cfg(test)` になる。

**影響**: 検証の段で針を `.CRT$XCU` にすると 5 か所が出て「3 か所だけ」が外れ、実装係が手順の読みで迷う。行の単位の grep では「`#[cfg(test)]` の下」を示せない。

**提案**: 確かめの言い方を次へ直す。「`areka_runtime::crash_report_test_support::install` を指す初期化子は 3 か所で、どれも宣言の連なりが `#[cfg(test)]` の下（`crash_report_test_support.rs` は自分の `#[cfg(test)]`・`runtime_tests/mod.rs` は `main.rs` 52 行・`crash_report_tests.rs` は `lib.rs` の `#[cfg(test)]` の宣言）。placement と wintf の写しはこの仕様の外」。

## 4. 良い所

1. **測りに裏打ちされている**: 移すファイル・呼び手の行・公開にする項目・依存の 12 個・`#[cfg(test)]` の使い手・見張り 4 本・出所を名前で読む 4 か所が、行の番号まで作業の木で測ってあり、確かめた範囲で全部合っていた。迷いうる所（`#[path]` の解け方・出し直しで解けるか・resolver 2 で機能が実機の exe にも効くこと・`EnvFilter` の前方一致）も、根拠つきで先に潰してある。
2. **足す仕組みが最小で、前例と同じ向き**: 強制退避の向きの逆転は、`SessionHooks` と同じ「型は使い手の crate・中身と据え付けは本体」の口の束 1 つで済ませ、分岐の条件・記録の本文・順は 1 字も変えない。強制退避のテスト 3 本は本物の `quit_app` を見たまま本体へ移し、小道具は `test-support` で開けて写しを作らない。見張り 4 本の根に `areka-input` を足す D11 は、「分ける前に本体の `src` が覆っていた範囲」が黙って縮むのを先回りで防いでいる。

## 5. 補足（重大ではない・討議で採るかを決めればよい）

- 新しいテスト `register_systems_installs_the_escape_hook` の前半（口が据わる）は、前例どおり `ghost_session_restart_tests.rs` の `boots_twice_in_one_process_without_double_registration`（`SessionHooks` を同じ所で確かめている・457〜461 行）に表明 1 行を足す形でも示せる。後半（結線前の Ctrl+左ダブルクリックで窓が消える）は `escape_works_without_mouse_wiring` と重なる。どちらの形でも要件 4.6 は満たすので、多重集合の差を 1 減らしたいかで選べばよい。
- `SessionHooks::of` は口が無いときの `error!` を `#[cfg(not(test))]` で抑えている。D2 は常に記録する形で、`hooks_tests.rs` がそれを捕まえる。前例と違うことは意図どおりで、そのことを D2 の行に 1 語添えると読み手が迷わない。

## 6. 判定

**GO**

- 理由: 依存の向き・公開面・テストの置き場・ログの出所・検証の手順が要件 1〜8 にすべて対応し、主張は確かめた範囲でコードと合う。指摘 3 件は、受け渡しの決め 1 件と文の正確さ 2 件で、設計の構造を変えない。
- 次の一手: 設計の討議（kiro-design-discussion）で、指摘 1〜3 と OPEN QUESTIONS 1〜7 を議題にする（指摘 1 は OPEN QUESTIONS 6 と合わせて、`hit-id-pass` のセッションの返事を先に取る）。決めを design.md に入れた後、`/kiro-spec-tasks input-crate-split` へ進む。
