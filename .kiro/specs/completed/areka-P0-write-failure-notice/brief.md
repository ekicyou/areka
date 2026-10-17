# Brief: areka-P0-write-failure-notice

> 2026-10-10 `/kiro-discovery` で起票。出どころは `areka-P0-winget-manifest-submission` のタスク 3.6（手元の確かめで見つかった件の起票。同 spec の要件 6.3）。根拠の記録は同 spec の `verification/winget-local-check.md`（「機械の全員向けの実測」の 3・4・6・7 と、「見つかった件と起票」の 5・6 行目）。区分 B（バグ）。

## Problem

- **困る人**: areka が書けない場所に置かれている利用者。見つけたのは winget の機械の全員向けの入れ方だが、読み取り専用の媒体・空きの無いディスク・権限の無いフォルダでも同じことが起きる。
- **areka の記憶が書けないとき、areka は利用者へ何も伝えない。** 起動の記録に `ERROR` と `WARN` が残るだけで、ゴーストは普段どおりに立って喋る。その裏で次のことが起きている。
  - 最後に使ったゴーストを覚えられない（次の起動は、前のゴーストで立たない）。
  - 起動中の印を、書くことも消すこともできない。＝前回がきれいに終わらなかったことに気付く仕組みが、黙って働かなくなる（落ちても、次の起動は何事も無かったように立つ）。
- **起動の記録の文が、実際と合わない所が 2 つ在る。**
  - 「最後に使ったものを記憶へ書きました」は、すぐ次の行で書き込みが失敗しているのに出る。
  - 終わるときの警告「次の起動は前回落ちたとして…」は、印がそもそも書けていないときには、そうならない。
- **書けないせいで `.nar` を入れられなかったのに、利用者には「ファイルが壊れてるのかもね。」と伝わる。** 利用者は、壊れていない `.nar` を疑うことになる。本当の理由（アクセスが拒否された場所）は、起動の記録にしか出ない。

## Current State

### コードで確かめたこと（2026-10-10・この枝の木）

- **areka の記憶の保存の失敗**: `crates/areka-sylphya/src/persist/mod.rs` の保存は、書き込みに失敗すると `error!`（`persist commit failed; …`）を残して `PersistOutcome::Degraded` を返す。黙ってはいないが、行き先は記録だけ。
- **起動中の印**: `crates/areka/src/boot_resolve.rs` の `write_session_mark`・`clear_session_mark`。`Degraded` のときは `warn!`（`session_mark_write_degraded`・`session_mark_clear_degraded`）を残すだけで、呼び手へは何も返さない。
  - `clear_session_mark` の警告の文は「きれいに終わったのに、次の起動は前回落ちたとして既定のゴーストで Ref6/7 付きになります」。印が書けていたのに消せなかったときは正しいが、**書くのも失敗していたとき**（今回）は、残る印が無いので当たらない。
- **最後に使ったもの**: 同じファイルの `LastUsed::record`。保存を投函するだけで結果を待たず、すぐに `info!`（`last_used_recorded`・「最後に使ったものを記憶へ書きました」）を残す。＝この文は「書いた」ではなく「書くように頼んだ」時点で出る。
- **インストールの失敗の理由**: `crates/areka/src/install/judge.rs` の `failure_word` が、`areka-nar` の失敗を `OnInstallFailure` の Reference0 の語へ写す。読み書きの失敗（`NarError::Io`）は、書庫が壊れているとき（`CorruptArchive`・`IntegrityMismatch`・`NameUndecodable`）と同じ `extraction` になる。これは完了 `ghost-install` の要件どおりの写し（「読み取り・組み上げ・確定・巻き戻しの I/O の失敗」を `extraction` へ）。
- **利用者へ告げる今の道**:
  - `crates/areka/src/alert.rs` の告知（`AlertScene`）は、起動を止める 4 場面と、SHIORI が動かなくなった場面だけ。書けない場面は無い。
  - 前回がきれいに終わらなかったことは、次の起動の `OnBoot` の Reference6（`halt`）・Reference7（落ちたゴーストの名前）で既定のゴーストへ伝える。この道は印に頼るので、印が書けないと働かない。
  - 完了 `ghost-install` の要件: ゴーストが `OnInstallFailure` に応えなかったとき、areka は代わりの画面や台詞を出さない。

### 正典（ukadoc）

- `OnInstallFailure`: 「インストールに失敗した際に発生。」Reference0「※失敗理由。」（ukadoc MCP の索引。2026-10-10 に引いた。語の一覧は索引に載っていない）。
- 失敗理由の語の一覧は、同じページの注記に在る。リポジトリの登記（`doc/COMPAT_ARCHITECTURE.md` §8 の「インストールの失敗理由の語の写し」の行と、完了 `ghost-install` の要件の正典の表＝2026-09-28 にページ本体から引いた物）では 6 語: `unlha32`・`extraction`（「解凍失敗（ファイル破損）」）・`invalid type`・`artificial`・`unsupported`・`password`。areka が送るのは `extraction`・`invalid type`・`unsupported` の 3 語。
- ＝「入れ先へ書けない」に当たる語は、この 6 語の中に無い。`extraction` の説明は「ファイル破損」なので、同梱のゴーストの台詞は正典の読みとしては筋が通っている。**要件の段で ukadoc のページ本体を引き直す。**

### 測った事実（記録から。0.0.2・開発機 1 台・機械の全員向けに入れて普段の権限で起動）

- 1 回の起動につき、areka の記憶への書き込みが 3 回失敗した（起こす前に印を書くとき・最後に使ったゴーストを書くとき・終わるときに印を消すとき）。3 回とも「アクセスが拒否されました。 (os error 5)」。
- このことを伝える台詞・イベント・知らせの画面は 0。2 回目の起動は告知を抑える環境変数（`AREKA_NO_ALERT`）を置いていない。
- 2 回目の起動で、「前回はきれいに終わらなかった」の行（`session_mark_found`）は 0 件。1 回目の終わりの警告が言ったようにはならなかった。
- メニューの「インストール…」で `.nar` を 1 つ選ぶと、作業フォルダを作れずに止まり、記録は `install_failed … kind="Io" phase="stage" … word="extraction"`。同梱のゴースト（えも？？）は「えー、なんでなん！？」「ファイルが壊れてるのかもね。」と喋った。えも？？ の辞書は、理由を見ずに 3 つの台詞から 1 つを選ぶ（辞書の頭の注釈「理由を問わず共通トーク」）。
- MCP の `get_log` には、上の失敗の行が載っていた（外の道具からは読める）。

### 分かっていないこと

- 書けない理由がほかの物（ディスクの空きが無い・読み取り専用）のときの記録の出方。測ったのは「アクセスが拒否されました」だけ。
- ゴーストの記憶（ゴーストのフォルダの中）が書けないときに起きること。今回の機械では書けた。
- バルーンの `.nar` と、ゴーストの切り替えでの出方（試していない）。

## Desired Outcome

向きだけを書く。細かい形は要件で決める。

1. areka が記憶を書けないとき、利用者は、書けないことと、どこへ書けないのかを知る。伝える道は、できる限りゴーストの台詞（このプロジェクトの決まり: 失敗はメッセージボックスでなく既定のゴーストの台詞で伝える・記録だけで終わる失敗の道を作らない）。
2. 起動の記録の文が、起きたことと合う（書けていないのに「書きました」と出ない。起きない先を予告しない）。
3. 書けないせいで入れられなかったことが、書庫が壊れていることと見分けられる。

## Approach

- 伝える道は、今在るものから選ぶ。**正典に無い新しいイベントは作らない**（前例: 切替の失敗の知らせは、新しいイベントを作らず、メッセージボックスも出さず、既定のゴーストの `OnBoot` の Reference6・7 に載せた＝`doc/COMPAT_ARCHITECTURE.md` §8 の「切替に失敗したことの知らせ方」の行）。候補の洗い出しと選び方は要件の段で行い、正典に在るイベントで運べるかを ukadoc で確かめる。
- 記録の文の 2 か所は、結果を見てから出す形か、文を事実に合わせる形に直す。
- インストールの失敗の理由は、areka の中では今も見分けが付いている（記録の `kind="Io"`）。ゴーストへ渡す所で 1 語にまとまるので、渡し方を直す。直し方は議題。

## Scope

- **In**:
  - areka の記憶が書けないときの、利用者への知らせ。
  - 起動中の印が書けない・消せないときの扱いと、記録の文。
  - `last_used_recorded` の文。
  - 書けないせいのインストールの失敗を、書庫の破損と見分けて伝えること。
  - 決定論テスト（書けない置き場を偽の保存の口で作る）。
- **Out**:
  - 書ける場所へ置き場を移すこと（`areka-P0-user-data-root`）。
  - 同梱のゴースト（えも？？）の辞書の台詞の直し（辞書の元はこのリポジトリの外。必要なら申し送る）。
  - ネットワーク更新の失敗の語（完了 `network-update` の持ち物）。

## Boundary Candidates

- 保存の結果を呼び手へ返す道（印・最後に使ったもの）。
- 利用者へ伝える道（どのイベント・どの台詞）。
- 記録の文の直し。
- インストールの失敗の理由の渡し方（`failure_word` と、`doc/COMPAT_ARCHITECTURE.md` §8 の登記）。

## Out of Boundary

- 記憶の保存の仕組みそのもの（`areka-sylphya` の読んで重ねて書く形・一時ファイルからの名前替え）。
- `areka-nar` の展開の手続きと、拒否の種類。
- 起動を止める告知の 4 場面と、SHIORI が動かなくなった場面の告知。

## Upstream / Downstream

- **Upstream**: 完了 `ghost-install`（`OnInstallFailure` と失敗理由の語の写し）・完了 `ghost-shell-balloon-switch`（起動中の印）・完了 `session-mark-residue`（印の残りの穴）・完了 `shiori-fault-notice`（失敗を利用者へ告げる道）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `ghost-install` の「I/O の失敗は `extraction`」の写しを見直す）。
- **Adjacent**:
  - `areka-P0-user-data-root`（同じ日に起票。置き場が直れば、winget で入れた形ではこの失敗が起きなくなる。ただし、書き込みはほかの理由でも失敗するので、この spec は置き場の直しを待たない）。
  - `areka-P0-install-live-target-hazards`（`install/` を触る）。
  - `areka-P0-baseware-root-list`（`boot_resolve.rs`・`install/`）。同じ時に走らせるなら、触る関数が重ならないことを照らす。

## Constraints

- メッセージボックスを足さない。正典に無い新しいイベントを作らない。
- 既存のゴーストの辞書が比べる相手は正典の語。Reference0 に正典に無い語を載せると、その語を知らない辞書がどう動くかを先に確かめる。
- 記録の語彙（`event=` の名前）を変えるときは、それを読む道具とテストを一緒に直す。
- 実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。書けない置き場は、機械の全員向けの入れ方を再び行わずに作る（権限を外したフォルダか、偽の保存の口）。
- 区分: B。規模の見込み: S〜M（要件で測る）。要件定義のモデル: Fable を勧める（伝える道と正典の語の裁定）。

## 要件の討議へ持ち込む議題（起票では決めていない）

1. **書けないことを、どの道で利用者へ伝えるか。** 今在る道（既定のゴーストの台詞・`OnBoot` の Reference・告知）のどれに載せるか、正典に在るイベントで運べるものが在るか。いつ伝えるか（起動のたび・最初の 1 回だけ）。
2. **インストールの失敗の理由**: 正典の 6 語に「書けない」に当たる語が無い。正典に無い語を足してよいか（前例: ネットワーク更新では、正典に語の無い輸送の失敗に `dns`・`connect` などを足して `doc/COMPAT_ARCHITECTURE.md` §8 に登記した）、`extraction` のままにして別の道で補うか。
3. **起動中の印が書けないとき**、落ちたことに気付く仕組みが働いていないことを、どう扱うか（知らせるだけか、別の手で補うか）。
4. **同梱のゴーストの辞書**が理由を見て台詞を分けるように、辞書の持ち主へ申し送るか。


## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）
- 前提の変化: 無し（10-10 の起票の後、関係のファイルは変わっていない）。
- 触るファイル（呼び手を数えた）:
  - `crates/areka/src/boot_resolve.rs`（起動中の印を書く・消す関数と、最後に使ったものの記録）。
  - その呼び手の `main.rs`（956 行・上限 1,000）と `emo2_boot/ghost_switch.rs`（902 行・切替の途中の印を書く所）。
  - `install/judge.rs`（失敗理由の語の写し）・要れば `alert.rs` と `crates/areka-sylphya/src/persist/mod.rs`（保存の結果）・`doc/COMPAT_ARCHITECTURE.md` §8。
  - 印のテスト `main_session_mark_tests.rs` は 938 行＝足さずに兄弟のファイルへ置く。
- 規模: 6〜10 タスク（伝える道を今在るものから選ぶ場合）。
- 先に要るもの: 働きの上では無し。ただしカタログの鎖の 4 本と同じ `boot_resolve.rs` を触るので、鎖の席を 1 回使う。
- カタログの鎖の順の案: `baseware-root-list` → `user-data-root` → `ghost-inner-balloon` → `write-failure-notice` → `shell-companion-balloon`＝この spec は 4 番目（区分 A の 3 本の後・区分 C の前）。
  - 触る関数は、`ghost-inner-balloon`・`shell-companion-balloon` が直す所（バルーンを決める鎖）とは別。開発者が「同じファイルでも関数が別ならよい」と決めれば、この 2 本のどちらかと並べられる。
  - `baseware-root-list`・`user-data-root` とは並べない（`main.rs`・`install/`・記憶の置き場が重なる）。
- ファイルの重なり（鎖のほか）: `emily-ghost-verification`（`install/judge.rs`・向こうが触る場合だけ）・`install-live-target-hazards`（`install/judge.rs`）・`tasktray-icon`（`main.rs`）。
- 優先度の区分: B。要件定義のモデル: Fable（どちらも起票のまま）。
- 見つけた穴・古くなった記述: 本文の触る所に、呼び手の `main.rs` と `emo2_boot/ghost_switch.rs` が無い（保存の結果を呼び手へ返すなら、両方に入る）。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）
- 前提の変化: 版の段で **v0.0.3**。`ghost-inner-balloon` は 2026-10-16 に版に縛らない列へ移った＝鎖の順「`user-data-root` → `ghost-inner-balloon` → この spec」のままだと、v0.0.3 の spec が版に縛らない spec を待つ逆向きになる。触るファイルは変わっていない。
- 鎖の中の順の答え: **`user-data-root` のすぐ後（3 本目）でよい**。`ghost-inner-balloon` との重なりは `boot_resolve.rs` だけで、触る関数も別（こちらは印と記録の関数、向こうはバルーンを決める鎖）＝この spec を前に出しても困らない。
- `baseware-root-list` と同時には走らせられない。順を決めるファイル:
  - `main.rs`: 最初のゴーストを起こす所で、印を書く呼び出しの引数が `running_name(&ctx.root, …)`＝向こうが根の並びに変える数行と同じ所。降ろした後の始末の所（`after_run`）も既定のゴーストの場所を根から組む。
  - `boot_resolve.rs`: 印に書く名前を引く `running_name` は、印を消す関数のすぐ下で、目録を根から引く。
  - `emo2_boot/ghost_switch.rs`: 定常に入ったときの印の投函が `running_name(&ctx.root, …)` を呼ぶ。降ろした直後の記録（`write_switch_drop` の呼び出し）もこのファイル。
- `user-data-root` とも同じウェーブに置けない（`main.rs`・`boot_resolve.rs`）が、触る所は別（向こうは記憶の置き場を決める `boot_config.rs` の `default_app_profile_dir` と、`main.rs` の起動の文脈を組む 1 行）。開発者が「同じファイルでも関数が別ならよい」と決めれば並べられる。働きの上では待たない（ただ、知らせに出す「書けない場所」は `user-data-root` の後の置き場で確かめる方が手戻りが無い）。
- 触るファイル: 棚卸㉔のまま（`boot_resolve.rs` 609・`main.rs` 956・`ghost_switch.rs` 902・`install/judge.rs` 246・要れば `alert.rs` 241 と `crates/areka-sylphya/src/persist/mod.rs` 403・COMPAT §8）。印のテスト `main_session_mark_tests.rs` は 938 行＝兄弟のファイルへ。
- 規模: 6〜10 タスク（変わらず）。
- 同じウェーブに置けない相手（24 本の中）: `baseware-root-list`・`user-data-root`・`multi-ghost-runtime`（`main.rs`・`boot_resolve.rs`・`ghost_switch.rs`）・`default-balloon-selfmade`（`boot_resolve.rs`）・`property-name-case-fold`（保存の結果を返す形で `persist/mod.rs` に触るときだけ）・`emily-ghost-verification`（向こうが `install/judge.rs` に触るときだけ）。重なり 0: `app-icon`・`balloon-text-bugs`・`winget-release-automation`。
- 優先度の区分: B（バグ。書けないことが利用者に伝わらない）。
- 要件定義のモデル: Opus に改める。伝える道は「切替の失敗の知らせ」（`OnBoot` の Reference6・7）、失敗の語は「ネットワーク更新の輸送の語の足し方」という先例が決める。新しい境界・並行の保証・骨格の分かれ目に当たらない。
- 分割の案: 切らない。
- 見つけた穴・古くなった記述: roadmap の「版をまたぐ依存の確かめ（2026-10-15）」の行と「棚卸㉔で列に足したこと」の鎖の順は、`ghost-inner-balloon` を前に置いたまま（直しは棚卸の報告へ）。本文の触る所に `running_name` の呼び手が無い。

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）
- 前提の変化: `baseware-root-list` が着地した（PR#317）。印に書く名前を引く `running_name` は根を受け取らない形になった（`crates/areka/src/boot_resolve.rs` 491〜501 行・決まったゴーストだけを受け取る）＝棚卸㉕の「`running_name(&ctx.root, …)` の呼び手で順が決まる」は消えた。呼び手は `main.rs` 477 行・`emo2_boot/ghost_switch.rs` 638 行・`emo2_boot/same_folder_pin.rs` 32 行で、この spec は名前の引き方に触らない。
- 書けない場所が増えた（`baseware-root-list` の要件 8.5 が「利用者への知らせはこの spec の持ち物」と書いた）:
  - 置き場の並びを記憶へ書けないとき（`crates/areka/src/roots/edit.rs` の `apply` 186〜194 行・記録 `roots_memory_write_failed`。ゴーストが動いている間は書き手へ投函するだけで結果を見ない 178〜180 行）。
  - 見つかる根が 1 つも無くて入れられないとき（`install/destination.rs` の `install_into` 99〜127 行）も、読み書きの失敗として「ファイル破損」の語 `extraction` になる。
- 「書きました」と先に言う記録は `last_used_recorded` のほかに 3 つ在る: `record_last_shell`（`boot_resolve.rs` 559〜563 行）・`record_last_balloon`（574〜578 行）・入れたバルーンの記憶（`install/desk.rs` の `remember_balloon` 394〜398 行）。どれも投函だけで結果を待たない。
- 印の書き損じの記録 `session_mark_write_degraded` の文は直っている（436 行）。直っていないのは消し損じ `clear_session_mark` の文（484 行）。
- 触るファイルと関数:
  - `boot_resolve.rs` の 391〜579 行（`save_app`・`write_session_mark`・`write_switch_drop`・`clear_session_mark`・`LastUsed::record`・`record_last_shell`・`record_last_balloon`）。
  - `main.rs` の `boot_first_ghost`（463〜506 行）・`settle_session_mark`（693〜714 行）・要れば `session_mark_verdict`（648〜689 行）。知らせを起動のイベントに載せるなら `first_boot_origin`（449〜454 行）とその呼び出し（306〜313 行）。
  - `emo2_boot/ghost_switch.rs` の `record_steady_memory`（604〜648 行）と `switch_to` の書き込み（671〜680 行）。
  - `install/judge.rs` の失敗の語（80〜128 行）・`roots/edit.rs` の `apply`・`install/desk.rs` の `remember_balloon`・要れば `crates/areka-sylphya/src/actor.rs`（書き手から結果を返す口は 83・686 行に在る）と `alert.rs`（267 行）・`doc/COMPAT_ARCHITECTURE.md` の §8。
- 行数: `main.rs` は 971 行（956 から増えた）。知らせの配線を `main.rs` に足すと 1,000 を超えうる（印の判定の枝と起動の関数で 10〜30 行）＝新しいファイルか `boot_resolve.rs`（615 行）へ置く。`ghost_switch.rs` は 917 行。印のテスト `main_session_mark_tests.rs`（941 行）には足さず兄弟のファイルへ。
- 規模: 7〜11 タスク（並びの記憶と 3 つの記録の文が増えた分）。分割の案: 切らない。
- 先に要るもの: 無し＝今の版の先頭。
- `user-data-root` と同じファイルを触るか: 確実に重なるファイルは 0。重なりうるのは `boot_resolve.rs`（向こうが議題 7 で `ghost_roots` 352〜357 行を触るときだけ）・`roots/edit.rs`（向こうが書ける根を守るときの `decide_remove` 67〜89 行）・`main.rs`（向こうが置き場を `main` の中で 1 度だけ決めるときの 171〜231 行）。どれもこちらとは関数が別。
- ほかの重なり: `multi-ghost-runtime`（`main.rs`・`boot_resolve.rs`・`ghost_switch.rs`）・`mcp-reload`（`ghost_switch.rs`）・`tasktray-icon`（`main.rs`）・`default-balloon-selfmade`（`boot_resolve.rs` 32 行の定数）・`install-live-target-hazards` と `emily-ghost-verification`（`install/judge.rs` を触るときだけ）・`property-name-case-fold`（保存の口を変えるときだけ）。
- 人の手: 要らない（書けない置き場は `target\` の下のフォルダか偽の保存の口で作る）。静かな机も前後の全体テストの比べも要らない。依存も足さない。
- 優先度の区分: B（書けないことが利用者に伝わらない）。要件定義のモデル: Opus（伝える道も失敗の語の足し方も前例が決める）。
- 古くなった記述（ロードマップ）: 「次のウェーブの候補」の「`running_name` の呼び手 3 か所」と、「棚卸㉔で列に足したこと」の「`main.rs`（956 行）」。

## 2026-10-16 棚卸㉖のウェーブの約束（C8・`user-data-root` と同じウェーブ）
- `user-data-root` と同じウェーブに並べる。向こうは `main.rs`・`boot_resolve.rs`・`roots/edit.rs` に触らない約束をした（どうしても `boot_resolve.rs` の `ghost_roots` に触るなら、こちらの着地の後に main を取り込んでから）。
- この spec が触らないもの: `crates/areka/src/boot_config.rs`・`crates/areka/src/roots/{mod,resolve}.rs`・`crates/areka-update/`・`crates/areka-mcp-bridge/`（向こうの持ち物）。
- `main.rs` は 971 行。知らせの結線を足すなら新しいファイルか `boot_resolve.rs` へ置き、`main.rs` は呼び出しの行だけにする。
