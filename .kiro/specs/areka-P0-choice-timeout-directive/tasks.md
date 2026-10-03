# Implementation Plan

> 設計 `design.md`（2026-10-03）の部品とテスト 4 群・文書の連鎖をタスクへ落とした。規模 S。触らない場所（`crates/areka-kanade/src/`・`crates/areka-parsers/`・`crates/areka-emo-text/`・`crates/areka/src/emo2_boot/`・`crates/areka/src/input_events/`・`crates/areka-talk/`・各 `Cargo.toml`・`Cargo.lock`）は全タスクを通して 0 ファイル。

- [x] 1. (P) 再生層の時刻表が選択待ちの区切りの値で区切りを飛ばしも解きもしないようにし、檻を足す
  - 時刻表の区切りの期限を決める腕のうち、選択待ちの腕だけを「期限なし」に変え、腕の上に理由（値は上位層へ運ぶ指令・再生層は解かない・解けるのは外からの解決だけ）を書く。クリック待ち・時間待ちの腕は触らない
  - 選択待ちの区切りの型の doc 注記に「時間の値は上位層へ運ぶ指令で、時刻表はこれで区切りを飛ばしも解きもしない」を 1〜2 行足す
  - dola の既存の時刻表テストのファイルへ、値 `0.0`・`-0.001` の区切りに着くと止まる檻、値 `0.5` の区切りを値ごと一度に越えても止まる檻、止まった後に時計を大きく進めても解けず外からの解決でだけ後続が配られる檻を足す（設計 Testing Strategy の dola の 1〜4）
  - 足した檻は変更前の腕に戻すと赤になり、変更後は緑。クリック待ち・時間待ちの既存の檻（区切りを越えたら飛ばす・自分で解く・既に過ぎていたら飛ばす）は無改変で緑のまま
  - _Requirements: 1.3, 5.1, 5.2, 5.3, 5.4_
  - _Boundary: dola TimedSchedule_

- [ ] 2. 台本の時間の指定を読み、選択待ちの区切りへ焼く
- [x] 2.1 (P) 時間の欄を秒の指令へ変える純関数を作る
  - 設計の読み方の表（欄なし・空欄→既定、整数→秒、前後の空白・`+`・余分な欄の扱い、正負の桁あふれ→飽和、それ以外→読めない）どおりに、ミリ秒→秒の変換をこの 1 か所に置く。値の正規化はしない
  - 定義の箇所に正典の URL の `// ukadoc:` 行を 1 行置く
  - compile の兄弟テストの新しいファイルに、表の全行と、整数 N ∈ {1, 999, 1000, 1001, 30000, 2147483647} で秒を 1000 倍して丸めると N に戻る檻を書く
  - compile の親ファイルの末尾に、このテストファイルへの接続宣言（`#[cfg(test)] #[path = "compile_choice_timeout_tests.rs"] mod choice_timeout_tests;`）を 1 行足す（2.2 は同じファイルへ追記する）
  - 表の全行の檻が緑で、結果の秒が常に有限であることを檻が確かめている
  - _Requirements: 1.4, 1.5, 2.1, 2.2, 2.3, 6.1, 8.1, 8.2_
  - _Boundary: areka-sakura compile parse_choice_timeout_
- [x] 2.2 汎用コマンドの腕で指定を読み、走査後の選択待ちの区切りへ最後の値を入れる
  - 汎用コマンドの腕の転記はそのままに、その後で名前 `set` かつ先頭の欄 `choicetimeout` のときだけ 2.1 の関数を呼び、走査の局所の値を上書きする（既定・読めない→未指定、秒→その値）。読めないときは語彙 `choice_timeout_unreadable`・欄 `raw` の WARN を 1 行出す（警告文に `\!` を書かない）
  - 走査後の選択待ちの区切りへその値を入れる。選択肢が無ければ区切りを出さず値は捨てる。区切りの注記「本層は値を供給しない」を今の振る舞いへ書き直す
  - 区切りの取り出しの補助関数を既存の私的な置き場から compile のテスト共有の置き場へ移し、既存の腕のテストは import の 1 行だけを変える（`timeout: None` の既存の表明は無改変）
  - 2.1 のテストファイルへ、台本の文字列から区切りの値を固定する檻を足す: 指定なし・`500`・`0`・`-1`・`-5`・省略・空欄・選択肢の後ろ・複数回（後が勝つ・最後が省略なら既定）・`\e`／`\-` の後ろだけ・選択肢の無い台本（全 cue が指定なしの台本と同一）・読めない値（既定・他の cue の列は読める値のときと同一）・読めない→読める・正負の桁あふれ
  - 新しい檻と、指定の無い台本についての既存の `timeout: None` の檻（compile の 7 か所）がすべて緑
  - _Depends: 2.1_
  - _Requirements: 2.4, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 9.1, 9.6, 10.6_

- [ ] 3. 層をまたいで通しの振る舞いを固定する
- [x] 3.1 時計を一度に進めても選択待ちに入ることを再生の駆動で確かめる
  - drive の兄弟テストの新しいファイル（接続宣言を drive の親ファイルに 1 行・本体は無改変）で、選択肢と指定 `500`・`0`・`-1`・省略を持つ台本を流し、時計を区切りの手前から区切り＋値を越えた先まで一度に進める
  - 各値で選択待ちの知らせがちょうど 1 通届き、運ばれる指令がそれぞれ `Some(0.5)`・`Some(0.0)`・`Some(-0.001)`・`None`、表示の終わりが区切りの時刻であること、その後さらに時計を進めても台本が終わらないことを確かめる
  - 新しい檻が緑で、既存の「選択待ちの知らせは 1 回だけ・指令は未指定」の檻も無改変で緑
  - _Depends: 1, 2.2_
  - _Requirements: 5.1, 5.2, 5.3, 9.3, 9.5, 9.6_
- [ ] 3.2 警告の記録と、台本から kanade の入口までの値を ghost で確かめる
  - ghost の兄弟テストの新しいファイル（接続宣言を dispatcher の親ファイルに 1 行・本体は無改変）で、記録の捕捉の窓の中で compile を呼び、読めない値 `abc` で WARN `choice_timeout_unreadable`（target は compile）が記録され、正しい値 `500` では同じ語彙が 0 件であることを確かめる
  - 同じファイルで、指定 `1234` の台本を dispatcher に流して注入の時刻を進め、kanade への選択待ちの知らせが表示の終わり `1_350`・指令 `Some(1.234)` で届くことを確かめる。テスト名は `script_choice_timeout_1234_reaches_kanade_with_display_end_and_directive`。doc 注記に対になる kanade のテスト `choice_timeout_directive_1234_fires_exactly_at_display_end_plus_1234` と境の値 `1234` を書く
  - この檻は時計を区切り＋1.234 秒の先まで進めないので 1.1 に依らず緑になる（要件 5.2 の証拠は 1.1 と 3.1 が担う）
  - 2 本の檻が緑
  - _Depends: 2.2_
  - _Requirements: 6.1, 9.2, 9.4, 9.5_
- [ ] 3.3 (P) kanade が指令どおりの期限で時間切れを出すことを外側から確かめる
  - kanade の外側のテストの新しいファイル（選択肢のテストの親ファイルに接続宣言 1 行）で、既存のハーネスに選択待ちの知らせを自前の注入列で投函する（既存の待ちを作る補助関数は指令を未指定に固定するので使わない）
  - 指令 `Some(1.234)` では表示の終わり＋1233 ms で時間切れが出ず、＋1234 ms ちょうどで `OnChoiceTimeout` が出る。指令 `Some(0.0)`・`Some(-0.001)` では既定の 30 秒を越えた＋60,000 ms でも出ない
  - 期限の両側の檻のテスト名は `choice_timeout_directive_1234_fires_exactly_at_display_end_plus_1234`。doc 注記に対になる ghost のテスト `script_choice_timeout_1234_reaches_kanade_with_display_end_and_directive` と境の値 `1234` を書く。`crates/areka-kanade/src/` は 0 ファイル
  - 檻が緑
  - _Requirements: 1.1, 1.2, 1.4, 2.1, 2.2, 9.4, 9.5_
  - _Boundary: areka-kanade tests_

- [ ] 4. 記録を実装に合わせる
- [ ] 4.1 網羅台帳の行を実装済みへ替え、台帳の連鎖を満たす
  - 台帳 `sakura-script.toml` の `\![set,choicetimeout,時間]` の項目を `implemented`・引受先を本 spec・注記を今の振る舞い（読めない値は既定＋WARN）へ
  - `roadmap-draft.md` に本 spec の行を足し、`areka-P0-sakura-time-directives` の受け持ちの数・brief の総数・段階ごとの表・「会話」の節の依存の欄を数え直した値で直し、追加の理由の段落と `snapshot_on` を書く
  - 報告 2 本を生成器（`report`・`report-summary`）で作り直す（手で数を直さない）。同じ C1 の `status-execution-states` が先に `main` へ着地していたら、取り込んでから作り直す
  - `cargo test -p ukadoc-survey` が緑（証拠の行の検査は 2.1 の `// ukadoc:` 行を見る）
  - _Depends: 2.1_
  - _Requirements: 10.1, 10.2, 10.5_
- [ ] 4.2 (P) 正典が黙っている点の裁量と、着地で古くなる記述を書き直す
  - `doc/COMPAT_ARCHITECTURE.md` §8 に裁量の 1 行（最後が勝つ・他の負の値も時間切れなし・空欄は省略と同じ・読めない値は既定＋警告・終わりのタグの後ろは数えない）を足し、「compile 側時間指令 allowlist」の行に `set,choicetimeout` を実際に読むようになったことを書き足す
  - `doc/choice-cascade-compat.md` の行 5b（台本の指定が流れる）と行 5d（着いたときに飛ばす判定も選択の区切りには効かない、と構造にした）を書き直す
  - `briefing-sakura-script.md` の消費側の表・担当の突合表・「語彙の登記」の表の `set,choicetimeout` の行を直し、§8 の行を指す先が合っていることを見る
  - 3 文書の該当の行が実装の振る舞いと一致し、`cargo test -p ukadoc-survey` が緑のまま
  - _Requirements: 10.3, 10.4, 10.6_
  - _Boundary: doc COMPAT_ARCHITECTURE, choice-cascade-compat, briefing_

- [ ] 5. 全体の回帰を確かめる
  - `tools/test-all.ps1` でワークスペース全体を回し、緑であること（クリック待ち・時間待ちの既存の檻・指定の無い台本の既存の檻を含む）
  - `git diff --stat main...HEAD` で、触らない場所と `Cargo.toml`・`Cargo.lock` が 0 ファイルであることを確かめる
  - _Requirements: 5.4, 9.5, 9.6, 10.2_
