# 設計の検証: talk-script-metadata

- 対象: `design.md`（e61536276 の時点）・`requirements.md`・`research.md`・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`）
- 進め方: `kiro-validate-design`（Analysis → Critical Issues → Strengths → GO/NO-GO）。対話なしで判定まで出す。

## 設計の検証の要約

付帯情報を kanade の `step` の出口で押し、`begin_talk` でトークの複製ごとに届け、開く系の受け口を共通の決まりに乗せる筋は、今のコードの通り道にそのまま乗っていて、要件 1〜7 のすべてに行き先がある。コードで確かめた主張（下の「コードで確かめたこと」）はおおむね正しい。直すべき点は 2 つで、どちらも設計の議論で決めて design.md を直せば済み、筋は変わらない。

## コードで確かめたこと

- **`step` を出る `StartTalk` は全部押される**: 本番で `StartTalk` を作るのは `areka-kanade/src/schedule/` の中だけで（`mod.rs:147` の `start_talk`・`boot.rs:262`／`282`・`change.rs:263`／`416`・`close.rs:73`。`close.rs:279` と `prop_sink.rs:355` はテストの中）、`actor.rs:488` の `execute_batch` は `step` の返す一括を写すだけ。
  - 翻訳に預けた一括: `sstp::on_send` は腕の中で `translate::defer` に預け、`translate::after` の `capture` は出口で預ける。どちらも `translate::on_done`（`mod.rs:290` の `Input::TranslateDone` の腕）が `deferred` を `route` の戻りとして返すので、その `step` の出口で `stamp_runner` を通る。`on_done` は `script` と `origin` だけを書き換え、`meta` に触れないので送り手は保たれる（要件 2.4）。輸送路の失敗では一括を捨てるので押す相手が無い。
  - 後書きだけの起動のトーク（`boot.rs:282`・空の台本＋`first_boot_epilogue`）も `route` から出るので押される。
- **`KanadeConfig` は `runtime.rs` に触らずに置き場を運べる**: `resolve_kanade_config`（`config.rs:24`）は `..KanadeConfig::new(…)` の形で組み、`runtime.rs:578` はその結果を受けて他の欄（`first_boot`・`choice_wake`・`boot_origin`・`shell_folder` ほか）だけを書き換える。テストの `KanadeConfig { … }` も 2 か所とも `..KanadeConfig::new` の形なので、欄を足しても壊れない。
- **`begin_talk` の引数を足しても一括の実装は壊れない**: `impl<T: CueSink + Clone + Send + 'static> BootCueSink for T`（`sink.rs:42`）は `clone_box` だけを実装し、`begin_talk` は既定のまま。自分で実装しているのは `BalloonLifecycleSink`（`talk_lifecycle.rs:278`）・`NoUserBreakCueSink`（`user_break_cue.rs:128`）とテストの受け口だけ。
- **依存の向き**: `areka-kanade` は `areka-talk` を本番で引くので `KanadeConfig.ghost_place: Option<GhostPlace>` は置ける。`areka-session` は `areka-ghost` を本番で引き、`areka-talk`・`areka-sakura` は引かないので、`areka_ghost::sink` の再輸出は要る。`areka-ghost` の dev 依存には `temp-path-kit`・`log-capture-kit` がある。
- **空のパス**: `destination.rs` は `file`・`explorer`・`editor` と `\j[file:…]` の空の引数を読みの段で断るので、`Unresolved::Empty` が新しく効くのは環境変数が空に展開された `\![open,file]` だけ（要件 6.5 の「唯一の違い」と合う）。

## Critical Issues

🔴 **Critical Issue 1**: 共通の決まり `talk_path` を `areka-ghost` に置く理由が事実と合わない
**Concern**: 設計の議論の 3 と research.md の「決め 4」は「付帯情報を受ける受け手は必ず `BootCueSink`（`areka-ghost`）を実装するので届く」とする。だが下流の `balloon-inline-image`（`\_b`）・`balloon-font-file`（`\f`）の受け手になる文字の層 `areka-emo-text` の本番の依存は `areka-sakura`・`dola`・`areka-actor`・`areka-parsers`・`areka-emo-present`・`areka-emo-compose` だけで、`areka-ghost` を引かない（今の文字の受け口は一括の実装に乗っている）。
**Impact**: 置き場を `areka-ghost` に固めると、文字の層は共通の決まりを呼ぶだけのために `areka-ghost`（kanade・sylphya・SHIORI の結線を抱える最上位の結線の層）への依存を足すか、`areka-runtime` で包んで解いた後のパスを渡す形に縛られる。この spec は下流 4 本が乗る土台なので、後から置き場を動かすと下流がまとめて動く。
**Suggestion**: `resolve`・`SearchOrder`・`Unresolved` を `areka-sakura`（例 `areka_sakura::talk_path`。`areka-talk` を引き、`areka-emo-text`・`areka-ghost`・`areka-runtime` のどれからも既に届く）へ置く。fs の読みは `Path::exists` だけで、呼ぶのは UI スレッドの外という約束はそのまま。`areka-session` へは今の設計どおり `areka_ghost` の再輸出で届ける。`areka-ghost` に残すなら、理由を「文字の層は `areka-runtime` で包む」と書き直し、下流の形を縛ることを境界の節に明記する。
**Traceability**: 要件 5.1（解き方は 1 つ）・Boundary Context の Adjacent expectations（下流がこの解き方を使う）
**Evidence**: design.md「Allowed Dependencies」・「Revalidation Triggers」・「設計の議論へ渡すこと」の 3／research.md の決め 4（147 行目は「`areka-emo-text` は `areka-runtime` に依存しない」と書いている）

🔴 **Critical Issue 2**: 触るファイルの並びに漏れがあり、依存の向きの 1 行が今のコードと違う
**Concern**: 「Modified Files」は `begin_talk` を直に呼ぶテストを `dispatcher_begin_talk_tests.rs` だけとするが、ほかに `areka-runtime/src/balloon_visibility_choice_wait_tests.rs`・`script_restriction_tests.rs`・`talk_lifecycle_signals_tests.rs`・`user_break_cue_origin_tests.rs` と `crates/areka/src/runtime_tests/frame_balloon_timeout_notice_e2e_tests.rs` が 2 引数で呼んでいる。`ShownTalk { … }` を直に組むテスト（`balloon_events_done_tests.rs`・`balloon_events_tests.rs`・`steady_choice_script_tests.rs`）にも `sender` が要る。また「依存の向き」の行は `areka-talk → areka-sakura::contract → areka-kanade` とするが、`areka-kanade` は `areka-sakura` を引かず `areka-talk` を直に引く。
**Impact**: コンパイラが全部見つけるので壊れはしないが、tasks.md の `_Boundary:_` と、同じウェーブ（`sylphya-sqlite-kvs`・`talk-thread-count-growth`）との「触るファイル」の突き合わせがこの並びを元にするため、実装の途中で範囲の外に見えるファイルを触ることになる。依存の向きの行は「違反はレビューで誤り」とされる規則なので、誤った向きが正の基準になる。
**Suggestion**: 上の 8 ファイルを「Modified Files」のテストの段に足し、依存の向きの行を `areka-talk → {areka-sakura, areka-kanade} → areka-ghost → areka-runtime／areka-session` に直す。
**Traceability**: 要件 7.1（決定論のテスト）・Boundary Context（同じウェーブの約束）
**Evidence**: design.md「File Structure Plan」の Modified Files 最後の項・「Architecture Pattern & Boundary Map」の依存の向きの行

## Design Strengths

- **出口で一括に押す形**: 実行するゴーストを `talk_origin::stamp` と同じ規律で `step` の出口に置くので、台本を作る腕が増えても漏れず、翻訳に預けた一括も翻訳の結果の `step` で自然に押される。`ScriptOrigin` の形と値に触らないので、出どころを比べる今のテストと判定が動かない。
- **受け手の境界を複製に合わせた**: `ReadmeCueSink` を `NoUserBreakCueSink` と同じく自分で `BootCueSink` を実装させ、`clone_box` で控えを既定へ戻すので、「複製＝トークの境界」が型の側で保たれ、前のトークの付帯情報を持ち越さない（要件 4.2）。opener も要求に載った実行するゴーストで文脈を組むので、UI スレッドで `GhostSlot` を引く時点のずれが消える（要件 3.4・6.2）。

## 設計の議論へ渡す仮定についての所見（判定には入れない）

- 議論の 1（`TalkSender::Runner`）: 送り手の名は `sender_name()` で要件 2.1 を満たし、同じ名のゴーストが 2 体居ても取り違えない。運行表の腕へ `KanadeConfig` を通さずに済むので、推奨案として妥当。
- 議論の 2（`ScriptOrigin` を `TalkMeta` に入れない）・議論の 4（置き場を絶対にできないとき `warn!` の上で `None`）: どちらも要件の確定と合い、問題は無い。

## Final Assessment

- **Decision**: GO
- **Rationale**: 今のアーキテクチャの通り道（出口の規則・`begin_talk`・受け口の自己選別）に沿っていて、要件のすべてに実装の道がある。指摘の 2 つはどちらも置き場と書き漏れの直しで、設計の議論で決めて design.md に入れれば済む。
- **Next Steps**: 設計の議論（`kiro-design-discussion`）で Critical Issue 1（`talk_path` の置き場）と 2（触るファイルの並びと依存の向き）を議題にし、design.md を直してから `/kiro-spec-tasks talk-script-metadata` へ進む。
