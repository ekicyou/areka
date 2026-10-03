# Implementation Plan

- [x] 1. 土台: 前提の確かめと、振る舞いを変えない部品
- [x] 1.1 emo2 の pasta が `OnTranslate` に 204 を返すことを実機で確かめ、実装前の比べる記録を採る
  - 既存の `crates/areka-kanade/tests/kanade/real_helper_test.rs` と同じ補助プロセス経由の読み込みを使う使い捨ての `#[ignore]` テストで、本物の pasta（emo2・絶対パス）へ `OnTranslate` の GET を正典の Reference0〜3 の形で送る。根はワークツリーの `target\` の下に置く
  - 続きを待っているシーンがある状態（雑談の途中）でも 204 で、そのシーンの進み方が変わらないことを見る
  - main の版のまま、emo2 で起動 → 雑談 → 選択肢 → 終了を 1 周し、台詞の記録を `target\` の下に採っておく（6.2 で比べる相手）
  - 結果（応答の状態コードと記録の抜粋・比べる記録の置き場所）を research.md の設計フェーズの記録に追記する。204 でなければ実装を止め、設計へ戻す（この関門は 1.2 以降のすべての前提なので、1.2 以降は 1.1 の後に始める）
  - 完了の状態: research.md に「pasta は `OnTranslate` に 204」の実機の証跡と比べる記録の置き場所があり、確かめのための一時コードはリポジトリに残っていない
  - _Requirements: 8.2_

- [x] 1.2 (P) 線の層に Reference の「欠番の印」を足す
  - 欠番の印（NUL 1 文字）を定め、要求の組み立てがその位置の `ReferenceN:` 行を出さずに番号だけ進めるようにする
  - 印を含まない要求のバイト列は 1 バイトも変わらない
  - 完了の状態: 線の層のテストで「Reference1 の行が無く Reference2・3 の番号が保たれる」「印の無い要求のバイト列が同じ」が緑
  - _Requirements: 3.3, 8.1_
  - _Boundary: AbsentReference_
  - _Depends: 1.1_

- [x] 1.3 (P) 字句解析と同じ規則で環境変数を置き換える関数を足す
  - 字句解析の走査の本体を 1 本にし、既存の字句解析とこの置き換えが同じ走査を使う（規則の写しを作らない）
  - 値を埋めるときは `\` を `\\`、`%` を `\%` にする。値の無い名前と環境変数でない部分は 1 バイトも変えない
  - 既存の字句解析のテストが変わらずに緑
  - 完了の状態: 置き換えのテスト（貪欲な名前 `%usernameabc`・角括弧の中の `%`・`\%`・`\\`・未閉じの `[`・値のエスケープ・値の無い名前）が緑
  - _Requirements: 2.4, 2.5, 8.1_
  - _Boundary: SysVarSubstitution_
  - _Depends: 1.1_

- [x] 1.4 再生側の値の規則で台詞を展開し、読みが変わらないことを照合する関数を足す
  - 値の決め方は再生時の展開と同じ規則（写しに値がある名前はその値・`username` は既定値「ユーザーさん」・それ以外は綴りのまま）
  - 展開の後と前の命令の列を照合し、読みが変わる台詞は展開せずに元のまま返して警告 `sysvar_expand_fallback` を 1 件残す
  - 再生（`compile`）の振る舞いは変えない
  - 完了の状態: `%usernameさん` → `太郎さん`・既定値・`selfname` 等・綴りのまま・読みが変わる 3 つの並びで元の文字列・展開の前後で再生の文字の並びが同じ、のテストが緑
  - _Requirements: 2.2, 2.3, 2.4, 2.5, 8.1_
  - _Depends: 1.3_

- [x] 1.5 (P) 運行に、展開と MAKOTO の鎖を外から渡す口の型を足す
  - 展開の関数と MAKOTO の口（台詞と元のイベントの ID を受け取り台詞を返す）の 2 つを束ねた型と、どちらも台詞をそのまま返す素通しを用意し、クレートの公開面に出す
  - 口の引数は文字列 2 つだけで、DLL・プロセス・文字コードの型を持たない
  - 完了の状態: 素通しが台詞を 1 文字も変えずに返すテストが緑で、運行のクレートの依存（`Cargo.toml`）は変わっていない
  - _Requirements: 7.1, 7.2, 7.4_
  - _Boundary: TranslateSeams_
  - _Depends: 1.1_

- [x] 2. `OnTranslate` の組み立て
- [x] 2.1 元のイベントの型と `OnTranslate` の組み立てを足し、許可表に加える
  - 元のイベント（ID は選択肢の任意名も逐語・Reference の並び）の型を足す
  - Reference0＝展開済みの台詞・Reference1＝欠番の印（線の層との境界から再輸出した名前で使う）・Reference2＝元のイベントの ID・Reference3＝元の Reference をバイト値 1 で連ねたもの（0 個なら空文字列）で組み立てる
  - 許可表に `OnTranslate` を ukadoc の URL の注記つきで足し、冒頭の Reference 表に 1 行足す。許可表の数の固定を 45 → 46 にする
  - 網羅の台帳の `OnTranslate` を実装済み・担当は本 spec に改め、台帳の検査が緑
  - 完了の状態: Reference0〜3 のテスト（Reference が 0 個・1 個・複数・空文字を含む・選択肢の任意名）と添字 1 が欠番の印であること、許可表の数 46 が緑
  - _Requirements: 3.2, 3.3, 3.4, 3.7, 8.1_
  - _Depends: 1.2_

- [ ] 3. 運行表の翻訳の帳簿と殻の実行
- [x] 3.1 応答の読みと元のイベントの控えを運行表に足す（まだ翻訳は始めない）
  - 運行の状態に「翻訳の待ち」と「応答を待っている GET の元のイベント」の 2 欄を足す。一括の最後の往復が GET ならその元のイベントを控え、NOTIFY・降ろす往復なら空にし、SHIORI の応答の入力で 1 回だけ取り出す
  - 運行表の翻訳のモジュール（`schedule/translate.rs`）を新しく作り、翻訳の待ち・殻への依頼（台詞・元のイベント・Status）・殻が戻す結果の 3 つの型をここに置く（3.2・3.3 はこの型を使う）
  - `OnTranslate` の生の結果を読む純粋な関数を、design の表どおり（置換・空・204・エラー応答・輸送路の失敗・GET では起きない結果）に足し、どの行も元のイベントの ID つきで 1 件記録する
  - 運行の状態を `..` なしの構造体リテラルで組む既存テスト（約 21 か所・`steady_flow_tests.rs` は 3 か所で 925 → 931 行）に新しい 2 欄の初期値を書き足す。それ以外の既存テストは変えない
  - この時点で再生の始まり方は今日と同じ
  - 完了の状態: 応答の読みの表の全行と記録の語彙・レベルのテスト、元のイベントの控えのテストが緑
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.7, 8.1, 8.4_

- [x] 3.2 殻が翻訳の依頼を実行する関数を足す
  - 順序は「展開 → `OnTranslate` の組み立て → 生の往復 → 応答の読み → MAKOTO の口」。Status は依頼に載った値を使う
  - 今日の往復の関数を「検査して送って生の結果を返す部分」と「エラー応答を 204 へ写す部分」に分け、既存の呼び手の振る舞いは変えない。エラー応答の警告は応答の読みの 1 件だけにする
  - 送る 1 か所で、Reference の値が欠番の印と同じ位置（`OnTranslate` の添字 1 以外）を空文字に替えて `reference_absent_marker_replaced` を 1 件残す
  - 完了の状態: 依頼を直接渡すテストで、偽の SHIORI が Reference0 に `太郎さん` を受け取る・応答の行列ごとの返り値・MAKOTO の口が応答の種類に依らず（台詞, 元のイベントの ID）で 1 回呼ばれる・印の置き換えと警告、が緑
  - _Requirements: 2.1, 2.2, 4.1, 4.4, 4.5, 7.3, 8.1_
  - _Depends: 1.5, 2.1_

- [x] 3.3 翻訳の行動と結果の入力を、運行表と殻の駆動に通す（捕まえる規則はまだ働かせない）
  - 翻訳の行動と翻訳の結果の入力を足し、結果の入力の腕で預けた一括の台詞だけを差し替えて返す（起動の記録の後ろ書きは触らない）。控え（再生中の台詞・切替の台詞）を `talk_id` の一致で最終の台詞に書き換える。切替の `OnClose` の台詞では切替の台詞を書き換えない
  - 輸送路の失敗では `translate_failed` を残して預けた一括を捨て、既存の故障の遷移へ。切替の送り出しなら切替の台詞を空にする。帳簿の無い結果は `translate_done_unexpected` を残して捨てる
  - 殻の駆動は、入れ直すものが SHIORI の応答か翻訳の結果かで入力を選び、停止の原因と引き継ぎの控えを両方の枝で同じに取る。口つきの起動の関数を足し、既存の起動の関数は素通しを渡す薄い包みにする
  - 既存テスト用に「翻訳の行動が出たら、結果を台詞そのままで入れ直して続きを返す」補助（`schedule/translate_test_support.rs`）を作っておく（3.4 で使う）
  - 完了の状態: 帳簿を直接置いた状態からの結果の入力のテスト（差し替え・控えの書き換え・故障・帳簿なし）が緑で、既存のテストは変更なしで緑
  - _Requirements: 1.4, 4.5, 6.1, 6.2, 8.4_

- [x] 3.4 運行表の出口で SHIORI の台詞の再生開始を捕まえ、既存のテストの期待を直す
  - `step` の出口で、条件 4 つ（SHIORI の応答で台詞あり・1 文字以上・一括に再生の開始・元のイベントが `OnTranslate` でない）を満たす一括を順序を保って帳簿へ預け、翻訳の行動 1 つに替える。Status は捕まえた時点の状態から導く
  - 空の台詞は `translate_skipped_empty`、`OnTranslate` の応答は `translate_skipped_self`、元のイベントが分からないときは `translate_source_missing` を残して今日どおり再生する
  - 既存の運行表・殻・結合のテストのうち台詞を返すものの期待に `OnTranslate` を書き足す（3.3 の補助を使う）。行数の上限に近いテストのファイル（`actor_tests.rs` 970 行・`steady_flow_tests.rs` 3.1 の後で 931 行）は書き足す前に行数を見積もり、あふれる分は新しい兄弟ファイルへ置く
  - この時点から kanade を使うすべてのものが `OnTranslate` を送るので、同じタスクの中でテスト用の偽の SHIORI 2 つ（`crates/areka/src/emo2_boot/spine.rs` と `crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の `ScriptedShioriBackend`。台本に無い GET で panic する）に「`OnTranslate` は既定で 204」を足す（呼び出しの記録には残す・`spine.rs` は 998 行のまま行数を増やさない）。areka-ghost と areka 本体の既存テストのうち台詞を返す台本を持つものの期待の列に `OnTranslate` を書き足す
  - 完了の状態: 運行のクレート・areka-ghost・areka 本体の全テストが緑で、許可表に無い ID を送らないテストも緑。emo2 の e2e の表示される台詞が本 spec の前と同じ。どのテストのファイルも 1,000 行未満
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.6, 4.6, 5.1, 5.2, 5.6, 8.1, 8.2, 8.4_

- [x] 3.5 5 種類の経路ごとの通過と控えの書き換えをテストで固定する
  - 表の 8 か所の腕を 1 本ずつ、最上位の `step` に元のイベントの台詞を入れて一括が翻訳の行動だけになり、結果を入れると預けた一括（起動は再生の開始＋バージョンの通知、選択の連鎖は選択の解決＋再生の開始の順）が返ることを確かめる。捕まえる規則を外すとどれも赤になる
  - areka が作る起動の記録だけの台詞・0 文字の台詞・外から頼まれた `OnTranslate` の応答が翻訳されないこと
  - 再生を始めた後の相・期限・台詞の切れ目の見張りの結果が翻訳なしと同じこと。印のイベントの台詞で見張りが印の台詞を追うこと
  - 結果の後に `OnChoiceTimeout` の Reference0 と停止通知の切替の中身が最終の台詞になること
  - 完了の状態: 経路のテストのファイルが緑で、出口の規則の呼び出しを外すと経路の 8 本すべてが赤になることを一度確かめた
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 3.1, 4.6, 5.5, 6.1, 6.2, 8.1_

- [ ] 3.6 待ちの間の入力・往復の数・Status を殻の結合テストで固定する
  - `OnTranslate` で止まる偽の SHIORI を使い、待ちの間にマウス・毎秒の時刻・終了の要求・切替の要求・外からの依頼を送り、止めている間は SHIORI へ何も送られず、放した後に再生の開始が先に出て、その後で各入力が今日の「再生中」の規則で処理されること
  - 台詞 1 つにつき `OnTranslate` が 1 回だけ・204 の元のイベントと空の 200 では 0 回であること
  - `OnTranslate` の Status が再生を始める時点の状態（`talking` を含む）で届き、見出しが他の GET と同じこと。輸送路の失敗で止まるとき停止の原因が今日の故障と同じこと
  - 完了の状態: 殻の結合テストのファイルが緑
  - _Requirements: 3.6, 4.5, 5.3, 5.4, 5.6, 8.1_

- [ ] 4. ゴーストの結線
- [ ] 4.1 写しの源から展開の関数を組む結線を足す
  - 写しの源から展開の関数を組む関数と、注入された 1 つの源を翻訳用と再生用の 2 つに分ける関数を足す
  - 翻訳用の sylphya の読み口は、再生用の固定の記録を出さず、別の `debug!`（`translate snapshot from sylphya reader`）を出す。排他が壊れた源は `translate_snapshot_poisoned` を残して空の写しで進む
  - 完了の状態: 分けた 2 つの源が同じ値を返す・翻訳用の読み口が再生用の記録を出さない・排他が壊れたときの記録、のテストが緑
  - _Requirements: 2.3, 8.4_
  - _Depends: 1.4, 1.5_

- [ ] 4.2 ゴーストの起動で口つきの運行を使う
  - 写しの源の解決を運行の起動の前へ移し、口つきの起動の関数に展開の関数と素通しの MAKOTO を渡す
  - 完了の状態: areka-ghost の全テストが緑で、ゴーストの起動の経路が口つきの起動の関数を通る
  - _Requirements: 1.1, 2.3, 7.2_
  - _Depends: 3.4, 4.1_

- [ ] 4.3 本物の結線で展開・残った綴り・切替の送り先をテストで固定する
  - `username` を持つゴーストの台詞 `%usernameさん` が `OnTranslate` に `太郎さん` で届き、204 のとき再生側の文字の並びが今日と同じこと
  - 偽の SHIORI が `OnTranslate` に `%usernameさん` を含む台詞を 200 で返したとき、再生側の文字の並びが `太郎さん` になること
  - 切替で、前のゴーストの送り出しの台詞の `OnTranslate` が前のゴーストの SHIORI にだけ届き、次のゴーストの挨拶の `OnTranslate` が次のゴーストの SHIORI にだけ届くこと
  - 置き先は `crates/areka-ghost` の結線の近くの新しい兄弟のテストファイル（例: `translate_wiring_e2e_tests.rs`）とし、既存のテストのファイルへは書き足さない（1,000 行の上限）
  - 完了の状態: 3 つのテストが緑
  - _Requirements: 2.2, 2.5, 2.6, 3.5, 8.1_

- [ ] 5. 正典の沈黙する箇所の登記
  - `doc/COMPAT_ARCHITECTURE.md` §8 に要件 6.3 の 6 点と設計の裁量 7〜11 を足し、既存の行「`OnGhostChanged` の Ref1 に何を載せるか」を「翻訳した後の、実際に表示した台詞を載せる」に書き換え、「`%username` 既定値」の行に翻訳の前の展開も同じ定義点を使うことを書き足す
  - 完了の状態: §8 に 11 点がそろい、`OnGhostChanged` の Ref1 の行は書き換えた 1 行だけで、文書を読む検査（ある場合）が緑
  - _Requirements: 6.3, 6.4_

- [ ] 6. 全体の確かめ
- [ ] 6.1 ワークスペース全体のテストを通す
  - `pwsh -NoProfile -File tools/test-all.ps1` を通し、ファイルの長さの検査も含めて緑にする
  - 完了の状態: 全体のテストの実行の記録が緑
  - _Requirements: 8.3_

- [ ] 6.2 emo2 の実機で翻訳を挟んでも表示が同じことを確かめる
  - emo2（pasta）で起動 → 雑談 → 選択肢 → 終了を 1 周する。`RUST_LOG` に `kanade=trace` を開け、根はワークツリーの `target\` の下に置く
  - `translate_reply` が `kind=no_content` で台詞ごとに 1 件出ること、台詞の記録が 1.1 で採った本 spec の前の記録と同じこと、記録の無い分岐が無いことを確かめる
  - 完了の状態: 実機の記録の抜粋で上の 3 点が確かめられ、台詞の記録（表示した台詞の並び）の差が 0（`translate_reply` と `OnTranslate` の `shiori_request` の行が増えるのは差に数えない）
  - _Requirements: 8.2, 8.4_

## Implementation Notes
- 1.1: emo2 の pasta は `OnTranslate` に 204（雑談の途中でも続きは変わらない）。比べる記録は `target\translate-baseline\`（`talk-record.txt`）。今の版は台詞の本文を記録に残さないので、6.2 は「どのイベントが台詞を返しどの順で再生が始まったか」の並びと画面の目視で比べる。Ctrl＋ダブルクリックの終了は 09-19 に撤去済みで、終了は右クリックのメニューの「終了」。`cargo build` の後は 32bit の helper を `target\debug\` へ写し直してから起動する
- 1.2〜1.5: clippy は HEAD の時点で既に赤（`areka-kanade/src/shiori/real.rs` の `collapsible_if`・`dola` の 21 件・`areka-sakura/src/compile_arm_tests.rs` の doc の字下げ・`shiori-host32-host/tests` の `drop_non_drop`）。担当は `clippy-199-lints` で、本 spec では触らない。自分が触ったファイルに指摘が無いことだけを見る。`areka-sakura` には `log-capture-kit` が無いので、記録の数はテストの中の小さな `tracing::Subscriber` で数える
- 2.1: 台帳 `shiori.toml` の `OnTranslate` の note は最終形の振る舞いで書いた。根拠の場所は今は `events.rs`・`shiori3.rs` だけなので、3.x で `schedule/translate.rs`・`actor_translate.rs` ができたら根拠の行に書き足す。台帳を変えたら `briefing.md` の barrier の数・`roadmap-draft.md` の `owner_count` も合わせ、`report/*.md` は手でなく `cargo run -p ukadoc-survey -- report`／`-- report-summary` で作り直す（改行だけ変わった報告ファイルはコミットに含めない）
- 3.1: `translate::before` は `Option<SourceEvent>` を返し、`after` は `replied` を取らない形で入れた（使い手がまだ無いため）。3.4 で設計の `Replied` の形に広げ、`after` の控えの決まりに「最後の往復が `Action::Translate` なら空にする」を足す。輸送路の失敗は `read_reply` では記録せず、`on_done` の `translate_failed`（3.3）が受け持つ。`State` を `..` なしで組む既存テストは 14 か所で、`steady_flow_tests.rs` は `..base_state()` なので触っておらず 925 行のまま（3.4 の行数の見積もりは 925 から）
- 3.2: `lib.rs` の `mod actor_translate;` に一時の `#[allow(dead_code)]` がある。3.3 で `execute_actions` の腕から `run_translate` を呼んだら外す。送出は `round_trip_raw`（検査・印の置き換え・`shiori_request`・往復）と、エラー応答を写す `round_trip_request` の 2 段。偽の SHIORI のスレッドは送り手を落としてから join しないと `recv` で止まる
- 3.3: 殻の本番の関数は `drive_translating`・`execute_batch` の名前になり、元の `drive`・`execute_actions` は素通しの口を渡す `#[cfg(test)]` の包みとして残した（既存テストを変えないため）。`actor.rs` は 876 行で、3.4 以降で殻に書き足す余地は 120 行ほど。翻訳の失敗で切替の台詞は `None`（下流は `unwrap_or_default()` で空文字）。設計の `translate_tests.rs` の行「選択の連鎖の後でも故障になること（論点 10）」はまだ無いので、3.5 で足す
- 3.4: `crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` は本 spec の前から 1,129 行で長さの検査の例外表に載っている。期待の列に `OnTranslate` を 1 つ足して 1,135 行（検査は例外のファイルの行数を固定しないので緑・例外表は触らない）。`State` を手で組み `reply_source: None` のまま SHIORI の応答を入れる既存テストは、構造上は起きない `translate_source_missing`（error）の枝を通って今日どおり再生する（触っていない）。主な経路の通過は 3.5 で固定する。3.5 では「204 の応答の `step` で生まれる起動の記録だけの台詞（`StartTalk` あり）を捕まえない」も固定する（3.4 の `no_content_reply_is_not_captured` は `StartTalk` の無い一括しか見ていない）。4.2 で本物の展開を渡すと、`spine_conformance_script.rs` の `translated()` と areka-ghost の `expected_translate` の Reference0（今は展開の前の台詞）が変わる
- 3.5: 「翻訳なしと同じ」の比べる相手 `translate_path_tests.rs` の `step_untranslated` は `step` の本体から `translate::before`／`after` を除いた写し。`step` の順序を変えたらこの写しも直す。停止通知の切替の中身は源（`State::change.script` が `Unloading` まで最終の台詞のまま）までしか見ていないので、停止通知そのものは 3.6 の殻の結合テストで確かめる。論点 10 のテストは `translate_path_tests.rs` に置いた
