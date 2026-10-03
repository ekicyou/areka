# 設計レビュー: areka-P0-status-execution-states

- 実施日: 2026-10-03
- 対象: `design.md`（2026-10-03 生成）・`requirements.md`（確定）・`research.md`
- やり方: 設計が「こうなっている」と書いたコードの箇所を、実際のソースを読んで突き合わせた（kanade の `status.rs`・`actor.rs`・`schedule/{mod,boot,change,steady}.rs`・`msg.rs`、areka の `update/{worker,desk,procedure}.rs`・`install/fetch_url.rs`・`input_events/user_break.rs`・`emo2_boot/user_break_cue.rs`・`emo2_boot/balloon_visibility_phase.rs`・`emo2_boot/frame/wiring.rs`・`emo2_boot/ghost_switch.rs`、areka-sakura の `drive.rs`、areka-ghost の `dispatcher.rs`、ukadoc-survey の `spec_checks.rs`、台帳とロードマップの原稿）。
- 判断の物差し: 既存の作りとの整合、C1 ウェーブで触らない約束のファイル、開発者方針（1 フレーム遅らせる解を取らない・失敗の経路は必ず記録する・判断の分岐は決定論のテストで押さえる・差分は小さく・既存の部品を使い回す）。

## レビューの要約

設計は、`Status` の組み立て側（`status.rs` の語彙・書式・導出表）を変えずに材料の欄 3 本を足し、出どころ 3 つから kanade へ届ける道を「ゴーストごとの持ち物が変化を押し込む（nouserbreak・balloon）」と「プロセスに 1 つの数を殻が読む（online）」の 2 種類に絞っている。コードの突き合わせでは、要となる主張（旗の下ろし方の是正が `dispatcher` の「閉じて合流してから複製」の順序に乗ること・`GhostSlot` の入れ替えが同期であること・一周の照合が置き場を据えていないこと・`run_order(&dyn UpdatePorts)` が `Sync` を求めないこと・`standard_started` が承諾待ちの後に呼ばれること）はいずれも実物と一致した。触らない約束のファイルにも及んでいない。

残る問題は、テストの決定論に関わるもの 1 件（プロセス全体の数をテストが立てると、同じテスト実行ファイルの中で並走する他のテストの `Status` の期待値が揺れる）と、online の説明図が実際の読み直しの時機と食い違っている 1 件である。どちらも作りの骨格を変えずに直せる。

## 重大な問題（最大 3 件）

### 🔴 重大な問題 1: プロセスに 1 つの「通信中の数」を立てるテストが、同じ実行ファイルの他のテストを揺らす

- **懸念**: 殻（`actor.rs`）は**プロセスに 1 つの** `online::PROCESS` を毎メッセージ読む。設計のテスト方針は「持ち手（更新・取得）は数を引数で受け、兄弟テストは自分の `static` を渡す」「`PROCESS` を立てるテストは『載っている』だけを見る」であるが、これは **`PROCESS` を立てたテストと同じ実行ファイルの中で並走している他のテスト**を守らない。`cargo test` は 1 つの実行ファイルの中のテストを並列に走らせるので、`tests/kanade/external_status_test.rs`（`tests/kanade.rs` に束ねられる）が guard を持っている間、同じ実行ファイルの `choice_test_choosing_status_tests.rs`・`steady_test.rs` などが起こした kanade の殻も `PROCESS` を読み、`Some("talking")`／`None` を期待する記録の照合が `talking,online` で赤になる。areka-ghost の `tests/ghost` も同じで、`real_pasta_test.rs`（env があるときだけ走る）が guard を持つ間、同じ実行ファイルの `recorder.rs` 経由の 9 件の `Status` 完全一致（例 `recorder.get("OnBoot", …, Some("talking"))`）が揺れる。
- **影響**: 要件 8.1・8.2 の決定論のテストが、実行の並び方しだいで赤になる（開発者方針「決定論的テスト網羅は必達」「16 並列の負荷で揺れを確かめよ」に反する）。
- **提案**: 殻が読む数を**差し替えられる形**にする。最小の形は `KanadeConfig` に `online: &'static OnlineCounter` を 1 本足し、`KanadeConfig::new` が `&online::PROCESS` を既定で入れる（既存の呼び手は無改変・結線のファイルに触らない）。殻は `config.online.is_online()` を読む。kanade のハーネスは関数内の `static` を渡して「載っている」「載っていない」の両方を決定論で見られる。実ゴーストを通す `real_pasta_test.rs` だけは `PROCESS` を使わざるを得ないなら、その 1 件を **別の実行ファイル**（`tests/` 直下の独立した `.rs`）に置き、`Status` の完全一致を持つテストと同じ実行ファイルに同居させない。
- **対応する要件**: 8.1・8.2・8.4（検証）、2.5（通信していなければ載せない）
- **設計書の該当箇所**: 「online の数（`online.rs`）と殻の同期」の「テストの決定論」、「Testing Strategy › Integration Tests」、「Risks」の 2 点目

### 🔴 重大な問題 2: online の説明図が、更新の「読み直し」の時機と食い違っている

- **懸念**: 設計の図「online: 数の増減と殻の読み取り」は、読み直し（同じゴーストへの切替）で新しい kanade が起動し OnInitialize〜OnBoot に `online` が載った**後**に `run_order` を抜けて数が戻る、と描いている。実物は `update/procedure.rs` の `run_order` が全対象を終えてから `ports.request_reload(…)` を頼み、**その直後に `return end`** する。頼みは UI スレッドが後で（窓を閉じる待ちを挟んで）行うので、`spawn_worker` の閉包が `drop(ports)` した時点で数は 0 に戻り、読み直しで起きる新しいゴーストの起動の各段には、ほぼ常に `online` は**載らない**。要件 2.6 の「通信が続いている場合」の条件に照らせば、読み直しの後に通信は無いのでこの振る舞い自体は正しいが、設計の記述がそれと逆の期待を与える。
- **影響**: 実装者や実機サインオフが「読み直し後の OnBoot に online が載るはず」と探して空振りする。要件 2.6 の本番で実在する場面は「URL の取得が続いている間に利用者がゴーストを切り替えた」（取得スレッドは `fetch_and_send` を抜けるまで数を持つ）であり、設計はそれを明示していない。
- **提案**: 図と「online: 数の増減と殻の読み取り」の説明を実物に合わせる——⑴ 更新の `online` は `standard_started` から `run_order` を抜けるまでで、読み直しの頼みの直後に終わる（読み直し後の起動には載らない）、⑵ 要件 2.6 の本番の場面は URL 取得と切替の重なりである、⑶ 要件 2.6 の決定論のテストは「数を立てたまま kanade を起こし、`Boot` の最初のリクエストに載る」で固定する（これは設計の作りのまま成立する）。読み直しの間も `online` を保ちたいなら、guard を `OrderDone` でなく読み直しの完了まで持ち越す別の作りになるので、要らないことを明記して閉じるのがよい。
- **対応する要件**: 2.3・2.6・5.1
- **設計書の該当箇所**: 「System Flows › online: 数の増減と殻の読み取り」（sequenceDiagram と箇条書き 2 点目）、「Requirements Traceability」の 2.6 行

### 🔴 重大な問題 3: なし

設計の他の主張はコードと一致しており、作りの骨格に関わる問題は上の 2 件に留まる。

## 設計の強み

1. **旗の下ろし方の是正が根本から**: 「前のトークの旗が次のトークの立ち上がりまで残る」区間を、kanade 側の先送りや遅らせでなく、トークごとの受け口の複製の `Drop` で `TalkEnded` を流す形で**構造として消している**。`areka-ghost/src/dispatcher.rs` の `on_start` が `close_active_if_any()`（Close→join）の後に `clone_box` する順序、`areka-sakura/src/drive.rs` の自然終端（`settle_after_tick` で player を落とす）と中断（`on_close` で phase を Idle へ差し替えて落とす）のどちらでも複製が落ちることを確かめた。残る遅れは UI→kanade の運搬だけで、`talking` と同じ種類である。
2. **既存の形の使い回しと、失敗の経路の記録**: 届けは既存の `UserBreakWiring.kanade`（nouserbreak）と `GhostSlot` → `GhostSession::kanade()`（balloon）から引き、新しい線も新しい crate 依存も作らない。`KanadeMsg` の変種は 1 つ（`ExecutionState`）にまとめ、`derive` は純関数のまま。送出の失敗は `error!`（`no_user_break_send_failed`・`balloon_status_send_failed`）、番号が取れない縮退は scope ごとに 1 回の `warn!`、置き場が空のときは `debug!` と、黙る経路が無い。`online` の数は RAII なので、取得口を作れない・落とせない・送れないのどの `return` でも戻る（`fetch_and_send` の 3 つの早期 `return` を確認）。C1 の約束のファイルには 1 つも触れない計画になっている（`balloon_visibility_phase.rs` は子で、`frame.rs` の `mod` 1 行は許される側）。

## 最終判断

- **判断: GO**
- **理由**: 要件 1〜8 の対応が設計に揃い、コードの突き合わせで骨格の主張は一致した。重大な問題 2 件はいずれも「数の読み口を差し替え可能にする」「図と説明を実物に合わせる」という局所の修正で、形（案 C・混成）を変えずに閉じられる。設計ディスカッションでこの 2 件を決めてから `/kiro-spec-tasks` へ進むのが妥当である。
- **次の手順**: 設計ディスカッションで重大な問題 1・2 を決着 → `design.md` を直す → `/kiro-spec-tasks areka-P0-status-execution-states`。

## 設計ディスカッションで触れたい小さな点

1. **ファイルの綴り**: 「File Structure Plan」の `crates/areka-kanade/tests/kanade/kanade.rs` は実在しない。束ねの入口は `crates/areka-kanade/tests/kanade.rs`（`#[path = "kanade/…"]` の列）である。
2. **手書きの数**: 「持ち主の記録」の `sakura-time-directives` の `owner_count` 「5→6」は、`doc/ukadoc-coverage/roadmap-draft.md` の実物が **11** なので 11→12 が正しい（本 spec の 4→3 は実物と一致）。数は `cargo test -p ukadoc-survey` が判定するので実装で捕まるが、設計の文面は直しておく。
3. **nouserbreak の残る遅れの明記**: 要件ディスカッションの記録は「食い違いを許す解は取らない」と書いている。設計は「残るのは UI→kanade の運搬の間だけ・`talking` と同じ種類」と説明しているので、これを**受け入れた残差**として要件 3.4 の行に一言添えておくと、実機サインオフで「載っているのに中断できる」を探す範囲が定まる（`no_user_break_changed` と `shiori_request` の時刻を並べる、と設計の 9.6 節にあるとおり）。
4. **`dispatcher` の所在**: 設計本文は「`dispatcher` は…」とだけ書き、研究は `dispatcher.rs` と書いている。実物は `crates/areka-ghost/src/dispatcher.rs`（areka-sakura ではない）。Supporting References に 1 行足すと実装者が迷わない。
5. **`balloon_visibility_phase.rs` の `use` 行**: 相の終わりで `status_report::report_balloons` を呼ぶため、冒頭の `use super::super::frame::{Emo2Wiring, resolve_talk_time};` に名前を足すことになる。研究 4.3 節が注意するとおり `emo-text-file-split` と同じ行で文字の上の衝突が起きうるので、併合のときに見るべき 1 行として控えておく。
