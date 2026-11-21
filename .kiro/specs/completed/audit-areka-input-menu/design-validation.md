# 設計の検証: audit-areka-input-menu

- 検証日: 2026-11-21
- 対象: `design.md`（649 行）・`requirements.md`（確定の節を含む）・`research.md`・steering（`product.md`・`tech.md`・`structure.md`）
- 進め方: `kiro-validate-design`（Analysis → Critical Issues → Strengths → GO/NO-GO）。対話なしで判定だけを出す。

## 設計の総評

77 件の指摘と件 1〜11 が、作業の組 G1〜G16・ファイルごとの表・「壊すと赤になる本番の行」の表で、漏れなく追える形に落ちている。範囲の外で触るファイルは要件 7.4 の挙げたものとちょうど同じで、設計が呼ぶ既存の口はどれも書かれた形のままコードに在る（下の「突き合わせ」）。直すべき所は 2 つある。強調の寄せ（指摘 15）のテストが、寄せでいちばん起きやすい壊れ方（場所の取り違え）を捕まえないこと。指摘 74 を「閉じる」とした扱いが、確定した要件（残す 0 件）と食い違うこと。どちらも設計の討議で直せる大きさで、作りを変える必要はない。

## コードとの突き合わせ（主な主張）

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| `SylphyaPublisher::barrier_within(Duration) -> Result<(), ReplyError>`（期限越えは `error!` 1 件） | `crates/areka-sylphya/src/actor.rs:562` | 在る（書かれた形のまま） |
| `areka_runtime::spine::{join_bounded, SPIN_WAIT, GpuPermit}` | `spine.rs:321-324`（`#[cfg(any(test, feature = "test-support"))]`）・`spine_wait.rs:428` `join_bounded(&str, ActorHandle)` | 在る。`SylphyaParts.handle` は `areka_actor::ActorHandle` で型が合う。依存は増えない |
| `run_balloon_visibility_phase(&mut RuntimeWiring, &mut World)` | `balloon_visibility.rs:502-504`（`test-support` で `pub mod phase`）・`balloon_visibility_phase.rs:72` | 在る。装着前（`balloon_models` が空）でも信号は取り出され、`TalkStarted` が `state.talk_id` を置く（`balloon_visibility_decision.rs:117-118`）。`deadline` は立たないので起こしの旗にも触れない。OPEN QUESTION 5 の道は組める見込み |
| `TextSlotBinding::new(slot, window, scale, surface, image)` | `areka-emo-text/src/actor.rs:72` | 在る（拡大率 2.0 の GPU の World を組める） |
| `areka_placement::spawn::test_support::{two_scope_placements, titles, ghost_window_entities}` | `spawn_test_support.rs:17,48,53` | 在る |
| `wintf::ecs::world::tick_wake::take`・`log_capture_kit::{capture, capture_lines}` | `tick_wake.rs:271`・`capture.rs:174`・`event.rs:165` | 在る |
| `MouseWiring::send_mouse` は私有で、子のモジュール（`drag`）から呼べる | `lib.rs:306` | 在る。`plan_and_send_move`・`send_double_click`・`notify_drag` の記録は名・水準・欄が `send_mouse` と同じ |
| 移す 13 本は `areka-input` を使わない | `spawn_assembly_tests.rs` の `crate::` は 180 行（残す 1 本）だけ。ほかの import は `areka-placement` の依存（`wintf`・`bevy_ecs`・`windows`・`tracing`・`log-capture-kit`）で足りる | `Cargo.toml` を触らずに移せる |
| `settle_box_hover` の 2 本は `areka-runtime` で組める | `areka-runtime` は `areka-emo-compose`・`areka-parsers`・`areka-emo-text` に依存済み | `Cargo.toml` を触らずに移せる |
| 名を変えるテストのファイルの、範囲の外の読み手 | `git grep`: `collision-probe.rs:66`・本体 `app_exit_tests.rs:182`・`doc/COMPAT_ARCHITECTURE.md:412` | 設計の「照合」の表と同じ |
| 出所が変わる記録を出所で絞る読み手 | `git grep` で、`areka_input::balloon::pressed` などを出所に書く読み手は 0 | 議題 5 の前提どおり |
| `WHEEL_PAGESCROLL` は `windows` 0.62.2 に無い | レジストリの `windows-0.62.2` を grep して当たり 0 | 指摘 75 の結論は正しい |
| `world_with_wiring` は `AppExit` を挿す | `pointer_test_support.rs:60` | 指摘 2 の表明の差し替えは落とす力を持つ |

## 重大な問題（3 件まで）

🔴 **重大な問題 1: 強調の寄せ（指摘 15）で起きやすい「場所の取り違え」を、どのテストも捕まえない**
**問題**: `apply_hover` は、普通のバルーンの `inject_choice_hover(&actor, v)` と箱の `inject_choice_hover_at(&box_place(..), v)` を 1 本の `inject_choice_hover_at(place, v)` にまとめる。いちばん起きやすい壊れ方は、呼び手が違う `PlaceKey` を渡すこと。ところが、今の箱のテスト（`shell_box_handler_tests.rs` の `hover_injections`・`ShellBoxHover::injected`）が見る値は、呼び手か `apply_hover` が書くもので、文字の層へどの場所で注入したかには関わらない。OPEN QUESTION 3 の案（在しない ordinal を注入して `areka_emo_text::actor` の DEBUG を 1 行見る）も、場所を問わず出る行を見ている（DEBUG の欄は `actor` だけで、箱と普通のバルーンは同じ actor になる）。
**影響**: 寄せの後に箱の強調が普通のバルーンへ当たる（またはその逆の）壊れ方が起きても、テストは緑のまま。要件 4.1 の「壊れたら落ちる」を、寄せそのものについて満たさない。
**提案**: 同じ DEBUG を、在るか無いかで分かれる形で使う。普通のバルーンにだけ ordinal 0 の選択肢を置いた runtime（`runtime_with_active_choice`）に ordinal 0 を注入し、普通のバルーンの場所なら DEBUG が 0 行、箱の場所なら 1 行になることを 1 本で見る。両方の向きを見るので、取り違えはどちらの向きでも赤になる。照合は文の字面でなく、出所 `areka_emo_text::actor`・水準 DEBUG・欄 `ordinal` で行う。範囲の外の公開の面は増えない（OPEN QUESTION 3 の前者の案のまま、落とす力だけを足す）。
**対応する要件**: 3.1・4.1・4.2（指摘 3・15）
**設計の該当箇所**: 「`apply_hover`（指摘 15・3）」・「指摘ごとの直し」の 3・15 の行・Testing Strategy の「`apply_hover` の注入の到達」の行・OPEN QUESTION 3

🔴 **重大な問題 2: 指摘 74 を「変えずに閉じる」とした扱いが、確定した要件（残す 0 件）と食い違い、かけ直しの終わりの条件を濁す**
**問題**: requirements.md の確定の節は「直す対象は指摘 1〜77 の全部」「残すと裁定した指摘は 0 件」と書いている。設計は指摘 74 を「変えない・理由を記録して閉じる。かけ直しで再び出たら、要件 2.5 の残すの裁定にする」としている。直さずに閉じることは、要件 2.5 の「残す」そのものである。しかもこの指摘は audit 自身が「今のままでよい」と書いたものなので、かけ直しでまた出る見込みが高い。裁定を要件 9.2 のかけ直しの輪の中まで先送りすると、「新しい指摘 0 件」の数え方が回の途中で変わる。
**影響**: 要件 3.2・2.5 と記録が食い違い、要件 9.2・9.3 の判定（0 件か、打ち切りか）がぶれる。
**提案**: 設計の討議で、指摘 74 を要件 2.5 の「残す」として今裁定してもらう。理由は「鍵の型 `(u32, String, CursorScene)` は `areka-runtime` の持ち物で要件 7.4 の外・`WM_SETCURSOR` ごとに 1 回の複製は想定の負荷で問題にならない」。裁定と理由は requirements.md の確定の節に書き足し、9.2 の数から外す。指摘 75 は「確かめたら定数が無かった」ので直しが要らないと確定したもので、残すには当たらない。記録の上で 74 と分けて書く。
**対応する要件**: 2.5・3.2・9.2・9.3
**設計の該当箇所**: 「指摘ごとの直し」の 74 の行・Requirements Traceability の 2.5 の行・OPEN QUESTION 4

## 設計のよい所

1. **すべてコードに根を持っている**: 寄せの 3 か所・移す 3 組・待ちの上限・トークの番号の置き方は、どれも既存の口を書かれた形のまま使う。`Cargo.toml`・`lib.rs`・本体のファイルを触らずに成り立つことを、上の突き合わせで確かめた。要件 7.3〜7.5 と 6.6 を守る道は、組み方の段から閉じている。
2. **落とす力を本番の行で書いている**: Testing Strategy の表は、テストごとに「どの本番の行を壊すと赤になるか」を書いている（例: `balloon_moved.rs:120` の `.map(|i| rows[i].ordinal)`・`char_watch.rs:250-254`・`click.rs:145-152`）。字の大きさに依る前提（拡大率 2.0・折り返し）も、表明してから進む形にして、黙って空振りしないようにしてある。要件 4.1〜4.3 を確かめられる形で持っている。

## 最終の判定

- **判定**: **GO**
- **理由**: 作りの食い違い・依存の向きの破れ・範囲の外への越境は無く、77 件と件 1〜11 への道も具体的に書かれている。重大な問題 2 件は、テストを 1 本強めることと、裁定を 1 件書き足すことで閉じる。設計の作りを変える必要はない。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-areka-input-menu`）で、重大な問題 1（OPEN QUESTION 3 の案に、在るか無いかで分かれる形の確かめを足す）と、重大な問題 2（指摘 74 を残すと今裁定する）を、OPEN QUESTION 1〜9 と一緒に確定する。
  2. 確定した分を design.md と requirements.md の確定の節に反映し、`/kiro-spec-tasks audit-areka-input-menu` でタスクを作る（見込み 16・20 を越えたら要件 2.6 の議題）。
