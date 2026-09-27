# Implementation Plan

> 前提: 決定論テスト（1〜2）は x64 だけで回る。触るソースは転記の `decode.rs` とその兄弟テスト 2 本、切替の入口の `ghost_switch.rs` とその兄弟テスト 1 本に閉じる（並走 `session-mark-residue`・`shell-balloon-switch` と重ねない）。`compile.rs`・`change_cue.rs`・字句解析・kanade・`areka-ghost`・`GhostSpec`／`SwitchRequest`／`SwitchVerdict` の形には触らない。各タスクの完了時に `cargo build --workspace --tests`（x64）が通る状態を保ち、`ghost_switch.rs`・`ghost_switch_tests.rs`・`decode.rs` はそれぞれ 1,000 行以下に収める。
>
> 1.1（`areka-parsers`）と 2.1（`areka` の純粋な関数）は別クレート・別ファイルなので並行できる。2.2 以降は `ghost_switch.rs` を共有するので直列。3.1 は台帳を実装済みにし備考に `reason` の語を書くので、証拠の行（1.1）と記録の語彙（2.2）が揃ってから行う。`.kiro/steering/roadmap.md` の完了の印（要件 7.5）は `/kiro-complete` の ROADMAP 更新で付け、タスクの中では二重に書かない。

- [x] 1. 転記: `\+`／`\_+` を角括弧付きの切替要求の別名にする
- [x] 1.1 (P) 裸の `\+`／`\_+` を `\![change,ghost,random]`／`\![change,ghost,sequential]` と同じ値へ写す
  - 転記の裸タグの振り分けに 2 腕を足し、`\+` は「change・ghost・random」、`\_+` は「change・ghost・sequential」の汎用の運び手（既存の受け皿）を返す。意味づけはせず、腕の注釈に「別名の転記」と書く
  - 各腕の直前に正典 URL の証拠の行（`// ukadoc: …#_5c_2b:1`／`#_5c__2b:1`）を置く。冒頭の説明の一覧に 1 行足す
  - 角括弧付きの `\+[…]` の腕は作らない（今日どおり生の綴りになる）。字句解析は触らない
  - 転記の兄弟テストに足す: `\+`・`\_+` の転記結果が角括弧付き形の転記結果と等しい／`\+こんにちは`・`\_+次へ` で直後の本文が残る／`\+[x]` は生の綴り 1 つのまま
  - 角括弧なしで生の綴りになる正典の綴りの一覧から `_+` を外し（12 → 11）、定数と説明文の数を合わせ、`\_+` が本仕様で別名になったと 1 文書く
  - `cargo test -p areka-parsers` が緑で、`\+`／`\_+` が生の綴りに落ちる経路が無い
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 6.4_
  - _Boundary: BareAlias_

- [ ] 2. 切替の入口: 特別な名前の解決と `lastinstalled` の記録
- [ ] 2.1 (P) 特別な名前を目録のフォルダ名へ解く純粋な関数と、解けない理由の語彙を足す
  - 解決の結果（特別な名前ではない／解けた〔フォルダ名と `sequential` の今の位置〕／解けない〔理由〕）と、解けない理由 4 つ（目録が空の `random`・目録が空の `sequential`・記録なし・記録のゴーストが目録に無い）を、`reason` 欄の語と人が読む 1 文を返す形で足す
  - 純粋な関数は目録・今のゴーストのフォルダ名・記録・乱数（関数引数）だけを入力とし、fs・World を読まない。特別な名前は正典の 3 語の完全一致（大文字小文字を区別）
  - `random`＝目録から今のゴーストを除いた候補から乱数で 1 体（候補 0 体で目録ありなら今のゴースト自身・目録が空なら解けない）、`sequential`＝今の位置の次・末尾なら先頭・位置が無ければ先頭（1 体なら自分自身）、`lastinstalled`＝記録のフォルダ名で目録を引く。乱数は `random` で候補が 1 体以上のときだけちょうど 1 回呼ぶ
  - 入口のテストファイルに純粋な関数のテストを足す（design の Unit Tests 1〜9）: 判定の綴り（`Random`・`RANDOM` は特別でない）・`random` の候補と両端・1 体・目録に無い（`None` と `Zed`）・空・`sequential` の次／末尾→先頭／目録に無い／1 体／空・`lastinstalled` の 4 通り・同名のゴーストより優先・理由の語と本文がすべて異なる
  - この段では入口から呼ばない（未使用の警告は次のタスクで消える）。純粋な関数のテストが緑
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.7, 3.1, 3.2, 3.3, 3.4, 3.5, 3.7, 4.3, 4.4, 4.5, 4.7, 5.2, 5.4, 5.5, 6.1, 6.3, 9.1, 9.2, 9.3, 9.4, 9.6, 9.7_
  - _Boundary: SpecialNameResolver_

- [ ] 2.2 `lastinstalled` の記録と書く口を足し、入口に解決の段を組み込む
  - プロセスの中だけの記録（フォルダ名 1 つの `Resource`）と書く口（置き換えて `info!(last_installed_recorded)` を 1 件）を足す。書く口には `#[allow(dead_code)]` を付けて本番の呼び手 `areka-P0-ghost-install` を注釈し、ファイル・記憶へは書かない
  - 記録の型 `LastInstalledGhost` と書く口 `record_last_installed` の 2 つの名前は design の「設計で決めたこと」のまま変えない（`ghost-install` の brief がこの名前で書く）
  - 入口の中身を「乱数を関数引数で受ける版」へ移し、今の入口はそこへ本番の乱数を渡す薄い皮にする（呼び手・メニューは変えない）
  - 目録を読んだ直後・名指しの突き合わせの前に解決の段を置く（名前の要求だけ・フォルダの要求は素通し）。予約・文脈の判定の順は変えない。記録は読むだけで消さない
  - 解けたら `info!(ghost_switch_resolved)` を名前・切替先・位置つきで 1 件残してフォルダの名指しへ読み替え、以降は今日の経路のまま。解けなければ `warn!(ghost_switch_unknown)` を `reason` と理由ごとの本文つきで 1 件残して「該当なし」を返す
  - 既存の名指しの該当なしの `warn!` に `reason = "name"` を足し、入口の説明の「該当なし」を「切替先が決まらない」へ改める。ファイル冒頭の説明に解決の段を 1 文足す
  - 既存テスト `unknown_names_warn_once_and_send_nothing` を `Nobody` だけに縮めて `reason = "name"` を確かめ、既存の入口テストがすべて緑のまま
  - _Requirements: 2.5, 2.6, 3.6, 4.1, 4.2, 4.6, 4.8, 5.1, 5.3, 5.5, 5.6, 5.7, 6.1, 6.2, 6.5, 9.5_
  - _Boundary: SwitchEntry, LastInstalledRecord_

- [ ] 2.3 入口の統合テストで解決の分岐を固定する
  - 既存の道具立て（実 fs の目録 `A`／`B`・今のゴースト `A` の World・送出の読み出し・ログの捕捉）で design の Integration Tests 1〜6 を足す
  - `random`・`sequential` が入口で切替先へ解けて kanade への要求 1 件（切替先・出どころ・`raise_event` の真偽の両方がそのまま）・`ghost_switch_resolved` 1 件・`ghost_switch_requested` 1 件／今 `B` の `sequential` は先頭へ
  - 目録が空の根で `random`／`sequential` が「該当なし」・送出 0・予約なし・`ghost_switch_unknown` 1 件で `reason` がそれぞれの語
  - `lastinstalled`: 記録なし／記録 `B` へ受理（`raise_event` 付きも）／2 回書いたら最後の 1 件／消えたゴースト／受理後も記録が残る／記録が今のゴースト自身でも受理。`last_installed_recorded` が書いた回数だけ残る
  - 切替中の 2 通目の `random` は解決せず `Busy`（`ghost_switch_resolved` 0 件）／メニューのフォルダ名 `random` は解決に掛からず `reason = "name"`
  - `cargo test -p areka` の入口テストが緑で、`ghost_switch_tests.rs` が 1,000 行以下（超えるなら design へ戻る）
  - _Requirements: 2.1, 2.5, 2.7, 3.1, 3.2, 3.5, 3.6, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 5.1, 5.3, 5.6, 5.7, 6.1, 6.2, 6.3, 6.6_

- [ ] 3. 台帳と文書を実装に合わせる
- [ ] 3.1 網羅台帳・ロードマップ草案を改め、生成物を作り直す
  - `sakura-script.toml` の `\+`・`\_+` を実装済み・持ち主＝本仕様にし、`same-feature` で `\![change,ghost,…]` を指し返す。備考の「壊れ方」を実装後の振る舞い（何へ解かれるか・解けないときの `reason`）へ書き換え、`\_+` の「所有先未定」の行を消す
  - 同台帳の `\![change,ghost,…]` の備考の「持ち場で…切り替えない」の 1 文を「解決済み（本仕様）」へ、`shiori.toml` の `OnGhostChanging`／`OnGhostChanged` の同じ文も同じ文面へ改める。シェル・バルーンの行には触らない
  - `roadmap-draft.md` に本仕様の `[[spec]]`（宛先 2 件・段 B・束「切替」・ウェーブ B4-②）を足し、`[briefs].count` を 1 増やし、追加の段落と段階の表の「切替」の行に本仕様を足す
  - 生成器（`ukadoc-survey` の `report`・`report-summary`）で生成物 3 本を作り直し、手では直さない
  - `briefing.md` と台帳の状態分布の突き合わせの検査が赤なら、生成器・数え直しの出力で数を合わせる（手で推測しない）
  - `cargo test -p ukadoc-survey` が緑（生成物が台帳より古い・要約が台帳と違う・状態分布、の検査を含む）
  - _Depends: 1.1, 2.2_
  - _Requirements: 7.1, 7.2, 7.3, 7.6, 7.8_

- [ ] 3.2 互換の設計文書と手書きの説明書に裁定と実装を記す
  - `COMPAT_ARCHITECTURE.md` §8 に裁定 1〜7（並び・目録に無いとき・1 体だけ・消えた記録・記録の寿命・同名より優先・一様な選択）を 1 行ずつ足し、「角括弧なし `\_` タグ」の行の `\_+` の所有先を本仕様へ改める
  - `briefing-sakura-script.md` の `\+`・`\_+` の「未対応」を実装後の振る舞いへ、`\_+` の「無所有一覧で裁定」を本仕様へ、`\![change,ghost,…]` の行の「持ち場」の 1 文を台帳と同じ「解決済み」の文面へ改める
  - 3 つの文書で本仕様の持ち場を「未解決」と書く文が 0 件（`grep` で確かめる）で、`cargo test -p ukadoc-survey` が緑のまま
  - _Depends: 2.2, 3.1_
  - _Requirements: 7.4, 7.7, 7.9, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7_

- [ ] 4. 全体の回帰と実機サインオフ
- [ ] 4.1 ワークスペース全体のテストを回す
  - 先に `cargo fmt --all` をかけてから、`tools/test-all.ps1`（fmt 検査・i686 の成果物を含む全体）を 1 回回し、1 本も落ちない
  - 触った 3 つのソースファイルがそれぞれ 1,000 行以下、`compile.rs`・`change_cue.rs`・字句解析・kanade に差分が無い（`git diff --stat` で確かめる）
  - _Requirements: 5.3, 6.3, 6.4_

- [ ] 4.2 実機で `\+` を言う検体から他のゴーストへ替わるのを見て、記録を残す
  - 完了 `ghost-shell-balloon-switch` の `signoff.md` の手順で、里々の検体の丸ごとの複製の `OnBoot` を `\+` 1 行に差し替え、目録に emo2 と並べる。先に `kanade=trace` の応答の生文字列で里々が `\+` を素通しすることを確かめる
  - `AREKA_PROFILE_DIR` を空のフォルダ・`RUST_LOG=info,areka=debug,kanade=trace`・有界の自動終了・絶対パスで起動し、`ghost_switch_resolved`（`name=random`）→ `ghost_switch_requested`（`raise_event=false`）→ 切替の完了までのログを grep し、組み立ての「無視」の `debug!` に `\+` が出ないことを確かめる
  - 結果を spec の下の `signoff.md` に残し、`lastinstalled` の実機確認と書く口の 2 つの名前（記録の型・書く口）を `ghost-install` へ申し送ると明記する
  - `.kiro/steering/roadmap.md` の本仕様の行は、このタスクでは触らず `/kiro-complete` の ROADMAP 更新で完了へ改める（二重に書かない）
  - `signoff.md` に期待したログの行がそろい、利用者から見て検体が消えて emo2 が現れた
  - _Requirements: 7.5, 8.1, 8.2, 8.3_
