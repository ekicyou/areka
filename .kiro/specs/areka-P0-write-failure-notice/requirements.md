# Requirements Document

> 本文の実測は **2026-10-16・本ブランチ**（main `0bb51d42`＝棚卸㉖のコミット。ソースは `4cf510c1`〔`balloon-text-bugs` の着地〕から不変）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 要件 9 は brief の議題 4 件に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。要件 1〜8 のうち、議題の答えで形が変わる所には「（議題 N）」と書いた。そこは向きだけを確定し、具体の選び方は要件 9 に置く。

## Project Description (Input)

**誰の何が困っているか**: areka が書けない場所に置かれている利用者。見つけたのは winget の「機械の全員向け」の入れ方（`areka-P0-winget-manifest-submission` のタスク 3.6・`verification/winget-local-check.md`）だが、読み取り専用の媒体・空きの無いディスク・権限の無いフォルダでも同じことが起きる。

**今の状態**: areka が自分の記憶を書けないとき、起動の記録に `ERROR` と `WARN` が残るだけで、利用者には何も伝わらない。ゴーストは普段どおりに立って喋るが、その裏で ⑴ 最後に使ったゴーストを覚えられない、⑵ 起動中の印を書くことも消すこともできず、前回がきれいに終わらなかったことに気付く仕組みが黙って働かなくなる。起動の記録の文にも、書けていないのに「書きました」と出る所と、起きない先（「次の起動は前回落ちたとして…」）を予告する所がある。さらに、書けないせいで `.nar` を入れられなかったのに、ゴーストへは書庫が壊れているときと同じ失敗理由 `extraction` が渡り、同梱のゴーストは「ファイルが壊れてるのかもね。」と喋る。

**何を変えるか**: ⑴ areka が記憶を書けないとき、利用者は「書けないこと」「どこへ書けないのか」「そのせいで何が覚えられないのか」を、メッセージボックスでなくゴーストの台詞の道で知る。⑵ 起動の記録の文が起きたことと合う。⑶ 書けないせいで入れられなかったことが、書庫の破損と見分けられる形でゴーストへ渡る。正典に無い新しいイベントは作らない。

> 起票: 2026-10-10 `/kiro-discovery`（区分 B＝バグ）。測り直し: 棚卸㉔（10-11）・棚卸㉕（10-16）・棚卸㉖（10-16）。ウェーブ C8 で `areka-P0-user-data-root` と同じウェーブに並ぶ（触らないファイルの約束は「Boundary Context」）。

## Introduction

### 誰が困っているか

- **areka のフォルダが書けない場所に在る利用者**（winget の機械の全員向けの入れ方で `C:\Program Files\…` に入った人・読み取り専用の媒体から動かす人・空きの無いディスクの人）: ゴーストは普通に動くので、何かがおかしいことに気付けない。閉じて開くと前と違うゴーストが立ち、落ちても次の起動は何事も無かったように立つ。
- **`.nar` を入れようとした同じ利用者**: 書けないせいで入らなかったのに「ファイルが壊れてるのかもね。」と言われ、壊れていない `.nar` を疑う。本当の理由（アクセスが拒否された場所）は起動の記録にしか出ない。

### いま何が起きているか（2026-10-10 の実測・2026-10-16 のコードの読み直し）

測った形（0.0.2・開発機 1 台・機械の全員向けに入れて普段の権限で起動）:

- 1 回の起動につき、areka の記憶への書き込みが 3 回失敗した（起こす前に印を書くとき・最後に使ったゴーストを書くとき・終わるときに印を消すとき）。3 回とも「アクセスが拒否されました。 (os error 5)」。
- これを伝える台詞・イベント・知らせの画面は 0。
- 2 回目の起動で「前回はきれいに終わらなかった」の行（`session_mark_found`）は 0 件。1 回目の終わりの警告が予告したことは起きなかった。
- メニューの「インストール…」で `.nar` を選ぶと、作業フォルダを作れずに止まり、記録は `install_failed … kind="Io" phase="stage" … word="extraction"`。同梱のゴースト（emo2）は「えー、なんでなん！？」「ファイルが壊れてるのかもね。」と喋った（emo2 の辞書は理由を見ずに 3 つの台詞から 1 つを選ぶ）。

コードで確かめた、areka が自分で書く記憶と、失敗したときの今の扱い:

| 書く物 | 定義点 | 書き込みの結果を見るか | 失敗したときの今の扱い |
|---|---|---|---|
| 起動中の印を書く（最初のゴーストを起こす前） | `crates/areka/src/boot_resolve.rs` の `write_session_mark` | 見る | `warn!(session_mark_write_degraded)`。呼び手へは返さない |
| 切替で降ろした直後の記憶（最後のゴースト＝既定・印＝切替先） | 同 `write_switch_drop`（呼び手 `crates/areka/src/emo2_boot/ghost_switch.rs` の `switch_to`） | 見る | `warn!(switch_drop_record_degraded)`。呼び手へは返さない |
| 起動中の印を消す（きれいな終わり） | 同 `clear_session_mark`（呼び手 `crates/areka/src/main.rs` の `settle_session_mark`） | 見る | `warn!(session_mark_clear_degraded)`。文は「次の起動は前回落ちたとして既定のゴーストで Ref6/7 付きになります」＝**印がもともと書けていなかったときは当たらない** |
| 印を残す（きれいに終わらなかった） | `main.rs` の `settle_session_mark` | （書かない） | `info!(session_mark_kept)` の文「次の起動は既定のゴーストで Ref6/7 付き」＝**印がもともと書けていなかったときは当たらない** |
| 最後に使ったゴースト・バルーン・シェル（定常に入ったとき） | `boot_resolve.rs` の `LastUsed::record`（呼び手 `ghost_switch.rs` の `record_steady_memory` ほか） | **見ない**（書き手へ投函するだけ） | 投函の直後に `info!(last_used_recorded)`「最後に使ったものを記憶へ書きました」＝**書けていなくても出る** |
| 定常に入ったゴーストの名前を印へ | `ghost_switch.rs` の `record_steady_memory` | **見ない** | `info!(session_mark_steady)`「…起動中の印へ投函した」＝文は「投函した」で事実と合う。失敗は areka の側の記録に出ない |
| 最後のシェルだけ／最後のバルーンだけ（差し替えの完了） | 同 `record_last_shell`・`record_last_balloon` | **見ない** | 投函の直後に `info!(last_shell_recorded)`・`info!(last_balloon_recorded)`「…記憶へ書きました」＝**書けていなくても出る** |
| 入れたバルーンの記憶 | `crates/areka/src/install/desk.rs` の `remember_balloon` | **見ない** | 投函の直後に `info!(install_balloon_remembered)`「…入れたバルーンへ書き換えました」＝**書けていなくても出る** |
| 窓の位置の記憶・ゴーストのプロパティの記憶 | `crates/areka/src/placement/persist.rs`・`crates/areka-ghost/src/prop_sink.rs` の投函 | **見ない** | brief の測り直しには載っていない投函の口（要件の段で数えた）。失敗は `persist commit failed` の記録だけ |
| 置き場の並び（メニューで根を足す・外す） | `crates/areka/src/roots/edit.rs` の `apply` | ゴーストが居ないときだけ見る／ゴーストが動いている間は**見ない** | ゴーストが居ないときは `error!(roots_memory_write_failed)`。動いている間は投函だけ（`baseware-root-list` 要件 8.5 が「利用者への知らせはこの spec の持ち物」と書いた） |
| 記憶の書き込みそのもの | `crates/areka-sylphya/src/persist/mod.rs` の `save_scope` | — | `error!`（`persist commit failed; …`・置き場のパスと OS の理由つき）を残して `PersistOutcome::Degraded` を返す。`Degraded` は中身を持たない（パスも理由も呼び手へ渡らない）。書き手へ投函する口 `SylphyaPublisher::persist_put` は結果の返り口を持たない（書き手の中には結果を返す道が在るが、外から頼む口が無い） |

記憶の置き場: アプリの記憶は `AREKA_PROFILE_DIR`、無ければ `<areka.exe のフォルダ>\profile\areka\sylphya.toml`。ゴーストごとの記憶は `<ゴーストのフォルダ>\ghost\master\profile\areka\` の下。

インストールの失敗理由（`crates/areka/src/install/judge.rs` の `failure_word`）: 読み書きの失敗（`NarError::Io`）は、作業フォルダや入れ先へ書けないときも、書庫が壊れているとき（`CorruptArchive`・`IntegrityMismatch`・`NameUndecodable`）と同じ `extraction` になる。見つかる根が 1 つも無くて入れられないとき（`crates/areka/src/install/destination.rs` の `install_into`）も `extraction`。これは完了 `ghost-install` の要件 5.2（「読み取り・組み上げ・確定・巻き戻しの I/O の失敗」を `extraction` へ）どおりの写し。

利用者へ告げる今の道:

- 告知（`crates/areka/src/alert.rs` の `AlertScene`）はメッセージボックスで、起動を止める 4 場面と SHIORI が動かなくなった場面だけ。書けない場面は無く、足さない（開発者裁定 2026-09-26「メッセージボックスはデスクトップマスコットとして無粋」・告知はアプリが終わるときの最後の手段）。
- 前回がきれいに終わらなかったことは、次の起動で既定のゴーストへ `OnBoot` の Reference6＝`halt`・Reference7＝落ちたゴーストの名前で伝える。この道は印に頼るので、印が書けないと働かない。
- 完了 `ghost-install` の要件: ゴーストが `OnInstallFailure` に応えなかったとき、areka は代わりの画面や台詞を出さない。

### 正典（ukadoc）の位置づけ

2026-10-16 に ukadoc のページ本体（`https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html`）を引き直した。

- `OnInstallFailure`: 「インストールに失敗した際に発生。」Reference0「※失敗理由。」（同ページ `#OnInstallFailure`）。
- 同ページの注記「インストールの失敗理由」は 6 語（逐語）: `unlha32`「lzh形式ファイルを展開するdllのロードに失敗した」・`extraction`「解凍失敗（ファイル破損）」・`invalid type`「install.txtに不備がある」・`artificial`「ユーザがインストールを中断した（※SSPのみ）」・`unsupported`「その他サポートしていないアーカイブ」・`password`「パスワードつきアーカイブでパスワード誤りがあった(SSP 2.5.09以降)」。リポジトリの登記（`doc/COMPAT_ARCHITECTURE.md` §8「インストールの失敗理由の語の写し」・完了 `ghost-install` の正典の表）と一致した。**「入れ先へ書けない」に当たる語はこの 6 語に無い。**
- 同ページの注記「ネットワーク失敗理由」（`OnUpdateFailure` ほか）には、書けない失敗の語が 2 つ在る（逐語）: `fileio`「※SSPのみ。容量不足による更新対象への書き込み失敗。」・`readonly`「※SSPのみ。読み取り専用フラグによる更新対象への書き込み失敗。ファイルやフォルダに読み取り専用フラグがついていると発生。」。areka のネットワーク更新は、手元の作業フォルダや確定の書き込みの失敗を既に `fileio` で送っている（`crates/areka/src/update/refs.rs` の `failure_word`・完了 `network-update` 要件 4.2）。
- `OnBoot`: Reference6「※MATERIA、SSPのみ　前回の処理中で落ちた時にhalt。」・Reference7「※MATERIA、SSPのみ　前回の処理中で落ちたゴースト名 。」（同ページ `#OnBoot`）。値は `halt` しか定義されていない。
- **ベースウェアが自分の記憶を書けないことをゴーストへ知らせるイベントは、ukadoc に無い**（2026-10-16 に `shiori_event` の分類を「書き込み」「エラー」「失敗理由」で引いた。在るのは個々の機能〔メールの確認・HTTP・圧縮・解凍・更新など〕の失敗イベントだけ）。＝知らせの道（議題 1）は正典が沈黙する所で、選んだ形は `doc/COMPAT_ARCHITECTURE.md` §8 に areka 裁量として登記する。

### 何を変えるか

- areka が自分の記憶を書けなかったことが、どの書き込みでも（結果を見ずに投函している書き込みも含めて）知らせの道へ届く。記録だけで終わる失敗の道を 0 にする。
- 利用者へは、メッセージボックスでなく、ゴーストの台詞の道で伝える。正典に無い新しいイベントは作らない（道の具体は議題 1）。
- 起動の記録の文を、起きたことに合わせる（書けていないのに「書きました」と出ない・起きない先を予告しない）。
- 書けないせいで入れられなかったときは、`extraction` と別の語をゴーストへ渡す（語の具体は議題 2）。

## Boundary Context

- **In scope**:
  - areka が自分で書く記憶（アプリの記憶＝起動中の印・最後に使ったゴースト・置き場の並び／ゴーストごとの記憶のうち areka が書く物＝最後のシェル・最後のバルーン・入れたバルーン・窓の位置・ゴーストのプロパティ）が書けないときの、利用者への知らせ。
  - 起動中の印が書けない・消せないときの扱いと、記録の文。
  - 「書きました」と先に言う記録 4 つ（`last_used_recorded`・`last_shell_recorded`・`last_balloon_recorded`・`install_balloon_remembered`）と、印の 2 つの文（`session_mark_clear_degraded`・`session_mark_kept`）。
  - `.nar` のインストールで、入れ先（作業フォルダ・入れる先の置き場）へ書けない失敗と、入れる先の根が 1 つも無い失敗を、書庫の破損と見分けてゴーストへ渡すこと（`doc/COMPAT_ARCHITECTURE.md` §8 の登記の改め）。
  - 決定論テスト（書けない置き場を偽の保存の口か、ワークツリーの `target\` の下の書けないフォルダで作る）。
- **Out of scope**:
  - 書ける場所へ置き場を移すこと（`areka-P0-user-data-root` の持ち物）。
  - 同梱のゴースト（emo2）の辞書の台詞の直し（辞書の元はこのリポジトリの外。申し送りは議題 4）。
  - ネットワーク更新の失敗の語（完了 `network-update` の持ち物。既に `fileio` を送っている）。
  - SHIORI（ゴーストの辞書）が自分で書くセーブファイルの失敗（SHIORI の持ち物）。
  - 記憶の保存の仕組みそのもの（`areka-sylphya` の読んで重ねて書く形・一時ファイルからの名前替え）。
  - `areka-nar` の展開の手続きと、拒否の種類。`.nar` 自体を読めない失敗の語（今日どおり `extraction`）。
  - 起動を止める告知の 4 場面と、SHIORI が動かなくなった場面の告知。
- **Adjacent expectations**:
  - `areka-P0-user-data-root`（ウェーブ C8 で同じウェーブ）: 置き場が書ける場所へ移れば winget の入れ方ではこの失敗は起きなくなるが、書き込みはほかの理由（空き・読み取り専用・権限）でも失敗するので、この spec は向こうを待たない。知らせに出す「どこへ書けないか」は、そのときの記憶の置き場をそのまま出す（置き場の決め方には触れない）。
  - ウェーブの約束（棚卸㉖）: この spec は `crates/areka/src/boot_config.rs`・`crates/areka/src/roots/{mod,resolve}.rs`・`crates/areka-update/`・`crates/areka-mcp-bridge/` に触らない。`crates/areka/src/main.rs`（971 行・上限 1,000）には呼び出しの行だけを足し、知らせの結線は別のファイルへ置く。
  - 完了 `ghost-install`: `OnInstallFailure` の Reference0 は語 1 つだけ・Reference1 以降を足さない・ゴーストが応えなければ代わりの画面も台詞も出さない、は変えない。改めるのは「書けない I/O の失敗を `extraction` へ写す」の 1 行だけ。
  - 完了 `ghost-shell-balloon-switch`・`session-mark-residue`: 印を書く・消す時点と、きれいな終わりの判定（`main.rs` の `session_mark_verdict`）は変えない。argv で始まったプロセスは印を読まず書かない、も変えない。

## Requirements

### Requirement 1: areka が書く記憶の失敗は、どれも知らせの道へ届く

**Objective:** As a 書けない場所で areka を動かしている利用者, I want areka が記憶を書けなかったことを、どの書き込みでも取りこぼさずに拾うこと, so that 記録だけで終わる失敗が 1 つも残らない

#### Acceptance Criteria

1. If areka がアプリの記憶へ書けない（起動中の印を書く・降ろした直後の記憶を書く・定常に入ったゴーストの名前を印へ書く・印を消す・最後に使ったゴーストを書く・置き場の並びを書く）, then the areka shall その失敗を、今日どおり記録（error か warn）に残したうえで、要件 2 の知らせの道へ渡す。
2. If areka がゴーストごとの記憶へ書けない（最後に使ったバルーン・シェル・最後のシェルだけ・最後のバルーンだけ・入れたバルーン・窓の位置・ゴーストのプロパティ）, then the areka shall 要件 1.1 と同じく、記録に残したうえで知らせの道へ渡す。
3. While ゴーストが動いていて、書き込みを書き手へ投函するだけで結果を待たない形で書いている, when その書き込みが失敗する, the areka shall その失敗を知らせの道へ届ける（投函の形を残すか結果を待つ形にするかは設計で決める。どちらでも、失敗が知らせに届かない書き込みは 0）。
4. The areka shall 失敗の記録に、書けなかった記憶の置き場（フォルダ）と、OS が返した理由（例: 「アクセスが拒否されました。 (os error 5)」）と、何が覚えられなかったか（印・最後のゴースト・置き場の並び など）を載せる。
5. The areka shall 書けなかったときも、その回の働きは今日どおり続ける（ゴーストは立ち続け、置き場の並びはその回に効き、切替・差し替えは進む）。書けないことを理由に起動を止めない・終わらせない。

### Requirement 2: 書けないことを、ゴーストの台詞の道で利用者へ伝える

**Objective:** As a 書けない場所で areka を動かしている利用者, I want 書けないことと、どこへ書けないのかと、そのせいで何が覚えられないのかを知ること, so that 置き場を直すか、書ける場所へ移すかを自分で決められる

#### Acceptance Criteria

1. When 要件 1 の失敗が起きる, the areka shall 利用者が、⑴ areka が記憶を書けないこと、⑵ 書けない置き場（フォルダのパス）、⑶ そのせいで覚えられないこと（次の起動で前のゴーストで立たない・落ちても次の起動で気付けない・置き場の並びが次の起動に残らない、のうち当たる物）を知れる形で伝える。
2. The areka shall 知らせを、ゴーストの台詞の道で伝える（具体の道は議題 1）。メッセージボックスを足さない（`crates/areka/src/alert.rs` の告知の場面を増やさない）。
3. The areka shall 正典（ukadoc）に無い新しい SHIORI イベントを作らない（0 件）。正典に在るイベント・Reference を使うときは、その Reference の正典の意味から外れた値を載せない。
4. When 同じ起動の中で書けない失敗が何度も起きる, the areka shall 知らせを利用者が読める回数に抑える（同じ置き場への同じ理由の失敗を、書き込みのたびに喋らせない。回数とまとめ方は議題 1）。
5. While 告知を抑える環境変数 `AREKA_NO_ALERT` が設定されている, the areka shall 書けない知らせの扱いを議題 1 の答えに従って決める（知らせがゴーストの台詞の道ならこの変数の対象外＝今日の告知だけを抑える意味を変えない、を推奨）。
6. The areka shall 選んだ知らせの形を、正典の沈黙を埋める areka 裁量として `doc/COMPAT_ARCHITECTURE.md` §8 に、ukadoc の URL と逐語の引用つきで登記する。
7. If 知らせを渡す先のゴーストが知らせに応えない（台詞を返さない）, then the areka shall 知らせを諦めた旨を記録に残す（記録だけで消える道を作らないため、応えなかったことは記録で見える）。代わりの画面は出さない。

### Requirement 3: 起動中の印が働いていないことを扱う

**Objective:** As a 書けない場所で areka を動かしている利用者, I want 落ちたことに気付く仕組みが今は働いていないと知ること, so that 次の起動で何も言われないことを「前回はきれいに終わった」と取り違えない

#### Acceptance Criteria

1. If 起動中の印を書けない（最初のゴーストを起こす前・切替で降ろした直後のどちらでも）, then the areka shall その回は「落ちても次の起動で気付けない」ことを、要件 2 の知らせに含める。
2. If 起動中の印を消せない, and 記憶にはその回か前回の印が残っている, then the areka shall 「きれいに終わったのに、次の起動は前回落ちたとして既定のゴーストで起きる」ことを、記録と知らせに含める。
3. If 起動中の印を消せない, and 記憶に残る印が無い（その回の印がもともと書けていなかった）, then the areka shall 次の起動で既定のゴーストに切り替わる・Ref6/7 が付く、とは予告しない。
4. If 前回の印が記憶に残っていて、その回に消すことも書き替えることもできない, then the areka shall 次の起動のたびに同じ「前回落ちた」が繰り返されうることを、知らせに含める（起動のたびに `halt` で起きるのは今日の判定どおりで、変えない）。
5. The areka shall 印が書けないときの補い（印の代わりの手で落ちたことに気付く仕組み）を、議題 3 の答えに従う（推奨は「知らせるだけ・補いは作らない」）。
6. The areka shall きれいな終わりの判定（`main.rs` の `session_mark_verdict`）の理由と順序、印を書く・消す時点、argv で始まったプロセスは印を読まず書かないこと、を変えない。

### Requirement 4: 起動の記録の文が、起きたことと合う

**Objective:** As a 起動の記録を読む開発者と、MCP の `get_log` で記録を読む外の道具, I want 記録の文が実際に起きたことと一致すること, so that 記録を読んで原因を取り違えない

#### Acceptance Criteria

1. The areka shall 「記憶へ書きました」の意味の記録（`last_used_recorded`・`last_shell_recorded`・`last_balloon_recorded`・`install_balloon_remembered`）を、書き込みが成り立ったときだけ出す。書けなかったときに「書きました」の記録は 0 件。（結果を見てから出す形か、投函の時点の文を「書くように頼みました」に改めて結果を別の行で出す形かは設計で決める。どちらでも、書けなかったときは失敗の記録が出る。）
2. The areka shall 印を消せなかったときの記録（`session_mark_clear_degraded`）の文を、要件 3.2・3.3 の区別に合わせる（残る印が在るときだけ「次の起動は前回落ちたとして既定のゴーストで起きる」と書く）。
3. The areka shall 印を残すときの記録（`session_mark_kept`）の文を、印がその回に書けていたときだけ「次の起動は既定のゴーストで Ref6/7 付き」と書き、書けていなかったときはそう予告しない。
4. When 記録の語彙（`event=` の名前・欄）を変える, the areka shall それを読むテストと文書を同じ変更で直す（読み手が古い名前を探して空振りする所を 0 にする）。読み手は設計の段で grep で数え直す。2026-10-16 の時点で、要件 4.1〜4.3 の 6 つの名前を読むのは、本体 3（`boot_resolve.rs`・`main.rs`・`install/desk.rs`）・テスト 10（`boot_resolve_tests.rs`・`main_session_mark_tests.rs`・`main_session_mark_fallback_tests.rs`・`install/desk_record_tests.rs`・`emo2_boot/frame/switch_finish_tests.rs`・`shell_balloon_switch_session_{lap,balloon,abort}_tests.rs`・`session_end_tests.rs`・`session_end_deadline_tests.rs`）・生成物の文書 1（`doc/roadmap-overview-2026-10-16.html`）。`tools/` の下に読み手は 0。このうちテストの幾つかは「書きました」の行を「投函が起きた印」として数えているので、文や出す時点を変えるときは、その判定の意味も置き換える。
5. The areka shall 今日の記録の行のうち、事実と合っている物（`session_mark_write_degraded`・`switch_drop_record_degraded`・`session_mark_steady`・`roots_memory_write_failed`・`persist commit failed`）の名前を変えない（文に要件 1.4 の欄を足すのは可）。

### Requirement 5: 書けないせいのインストールの失敗を、書庫の破損と見分けて伝える

**Objective:** As a 書けない場所で `.nar` を入れようとした利用者, I want 入らなかった理由が「書けない」ことだとゴーストに分かること, so that 壊れていない `.nar` を疑わずに済む

#### Acceptance Criteria

1. If `.nar` のインストールが、作業フォルダを作れない・入れる先の置き場へ書けない・確定や巻き戻しで書けない、のどれかで止まる, then the areka shall `OnInstallFailure` の Reference0 に、書庫の破損（`extraction`）と別の語を 1 つ載せる（語の具体は議題 2）。
2. If 入れる先の根（ゴーストやバルーンの置き場）が 1 つも見つからずに入れられない, then the areka shall 要件 5.1 と同じ語を載せる（「書庫が壊れている」とは伝えない）。
3. The areka shall 書庫が壊れている・中身の整合が合わない・名前を読めない（`CorruptArchive`・`IntegrityMismatch`・`NameUndecodable`）と、`.nar` 自体を読めない失敗は、今日どおり `extraction` を送る。
4. The areka shall `OnInstallFailure` の Reference0 を語 1 つだけにし、Reference1 以降を足さない（完了 `ghost-install` 要件 5.1 のまま）。ゴーストが応えなければ代わりの画面も台詞も出さない（同じく今日どおり）。
5. The areka shall 失敗理由の写しを、ワイルドカードの腕を置かない網羅の形のまま保つ（失敗の種類が増えたらビルドが止まる）。`invalid type`・`unsupported` の写しは変えない。
6. The areka shall 改めた写しを `doc/COMPAT_ARCHITECTURE.md` §8 の「インストールの失敗理由の語の写し」の行に登記し直す（正典の語の出どころ＝ukadoc の URL と逐語の引用・正典に無い語を使うならその旨）。
7. The areka shall 記録 `install_failed` の欄（`kind`・`phase`・`word`）を残し、`word` は新しい語を出す。
8. If インストールの失敗が areka の記憶の書き込みの失敗でもある（入れたバルーンの記憶を書けない など）, then the areka shall 要件 1・2 の知らせの道にも渡す。

### Requirement 6: 既存の振る舞いと境界を守る

**Objective:** As a 既存の利用者と既存のゴーストの作者, I want 書ける場所で動かしているときの振る舞いが 1 つも変わらないこと, so that この直しで普段の動きが崩れない

#### Acceptance Criteria

1. While areka の記憶がすべて書ける, the areka shall 書けない知らせを 0 回にし、起動・切替・差し替え・終了・インストールの振る舞い（送るイベント・Reference・台詞・記録の行の数）を今日と同じにする（要件 4 で文を直す行は、書けたときの文が今日と同じ意味であること）。
2. The areka shall メッセージボックスを足さない（0 個）。起動を止める告知の 4 場面と SHIORI が動かなくなった場面の告知の文面・出る条件を変えない。
3. The areka shall 記憶の保存の仕組み（読んで重ねて書く形・一時ファイルからの名前替え）と `areka-nar` の展開の手続きを変えない。
4. The areka shall ウェーブ C8 の約束どおり、`crates/areka/src/boot_config.rs`・`crates/areka/src/roots/{mod,resolve}.rs`・`crates/areka-update/`・`crates/areka-mcp-bridge/` に触らず、`crates/areka/src/main.rs` を 1,000 行以下に保つ。
5. The areka shall ネットワーク更新の失敗の語（完了 `network-update` の写し）を変えない。

### Requirement 7: 決定論テストと実機での確かめ

**Objective:** As a 開発者, I want 書けない場面のすべてが決定論のテストで固定されていること, so that 後で誰かが知らせの道や記録の文を壊したらテストが赤くなる

#### Acceptance Criteria

1. The areka shall 要件 1.1・1.2 の書き込みの 1 つずつについて、書けない置き場（偽の保存の口、またはワークツリーの `target\` の下で書き込みを拒むフォルダ）で、失敗が知らせの道へ届くことを判定するテストを持つ（判定した書き込みの数が、要件 1 の表の書き込みの数と等しいことも判定する）。
2. The areka shall 要件 3.1〜3.4 の 4 つの形（印を書けない・消せずに残る印が在る・消せずに残る印が無い・前回の印が消せずに残る）を、それぞれテストで判定する。
3. The areka shall 要件 4.1〜4.3 の記録の文を、書けたときと書けなかったときの両方でテストで判定する（書けなかったときに「書きました」が 0 件）。
4. The areka shall 要件 5.1・5.2 の失敗が新しい語を、要件 5.3 の失敗が `extraction` を送ることを、失敗の種類の 1 つずつについてテストで判定する。
5. The areka shall 要件 6.1（書けるときは知らせが 0 回）をテストで判定する。
6. The areka shall テストの根・検体・一時フォルダをワークツリーの `target\` の下だけに置き、機械の全員向けのインストールを再び行わない。印のテスト `crates/areka/src/main_session_mark_tests.rs`（941 行）には足さず、兄弟のファイルへ置く。
7. When 実装が終わる, the areka shall 書けないアプリの記憶の置き場（`target\` の下で書き込みを拒むフォルダ）で emo2 を 1 回起動して終える実機の確かめを、有界の自動終了で行い、知らせが利用者に見える形で出たこと（台詞の文か画面の写し）と、要件 4 の記録の文を記録に残す。人の手は要らない。

### Requirement 8: 同梱のゴーストへの申し送り

**Objective:** As a 同梱のゴースト emo2 を使う利用者, I want 書けないせいで入らなかったときに emo2 が「壊れてるかも」と言わないこと, so that 本当の理由に近い台詞を聞ける

#### Acceptance Criteria

1. The areka shall emo2 の辞書を、この spec の中で書き換えない（辞書の元はリポジトリの外。リポジトリには配布形の `vendors/sample_ghost/emo2.nar` だけが在り、`dic/install.pasta` の `OnInstallFailure` は「理由を問わず共通トーク」の 3 場面）。
2. When 要件 5 の新しい語と、要件 2 の知らせの形が決まる, the areka shall emo2 の辞書の持ち主へ、届くようになった語と知らせの形と、理由ごとに台詞を分けてほしいことを申し送る（議題 4。申し送りの置き場は議題 4 の答えに従う）。
3. The areka shall 申し送りが届くまでの間も、要件 5 の語は送る（辞書が知らない語は辞書の既定の台詞へ落ちるだけで、今日より悪くならない。ギャップ分析 `research.md` §5.3 で検体の 5 体〔emo2・claudia・konnoyayame・emily4・R_POST_and_KOMAINU〕を読み、知らない語で台詞が出なくなる辞書は 0 と確かめた）。

### Requirement 9: 裁定（暫定・要件ディスカッションで確定）

**Objective:** As a 開発者, I want brief の議題 4 件に推奨案が付いていること, so that 要件ディスカッションで一括して決められる

#### Acceptance Criteria

1. **議題 1（知らせの道・いつ伝えるか）**: The areka shall 次の候補から 1 つを選んで要件 2 を満たす。
   - 候補 (a) **areka 自身の文を、今立っているゴーストのバルーンに出す**（SHIORI へイベントを送らず、ベースウェアが組んだ台本を表示する。新しいイベントは作らない。文面は areka が持つ）。
   - 候補 (b) **既定のゴーストの `OnBoot` の Reference に載せる**（切替の失敗の知らせの前例と同じ器。ただし Reference6 の正典の値は `halt` だけで、「書けない」を載せると正典の意味から外れる＝要件 2.3 に触れる）。
   - 候補 (c) **知らせない（記録と MCP の `get_log` だけ）**＝brief の目指す姿 1 に反する。
   - **推奨: (a)**。理由: 正典に器が無い（「正典の位置づけ」）。areka がゴーストへ送れるイベントは正典の固定の名前の許可の一覧（`crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS`）に限られ、「書けない」を正典の意味のまま運べる Reference は `OnInstallFailure`・`OnUpdateFailure` の Reference0 だけ（どちらもその機能の失敗のときにしか送れない）。正典の意味を曲げずに起動中の書き込みの失敗を伝えられるのは、イベントを介さずベースウェアの文として出す形だけ。第三者のゴーストの辞書に頼らず、どのゴーストでも同じに伝わる。時期は「1 回の起動につき最初の失敗のときに 1 回」（同じ置き場の続く失敗はまとめる＝要件 2.4）。起動の直後の失敗（印を書けない）は、ゴーストが最初に喋り終えた後に出す。`AREKA_NO_ALERT` の対象外（要件 2.5）。
   - (a) の注意（討議で必ず確かめる）: ⑴ areka には今、ベースウェアが組んだ文をバルーンに出す道が無い（新しく作る。イベントではないので「新しいイベントを作らない」には触れない）。⑵ 完了 `ghost-install` は「ゴーストが応えなかったとき、areka は代わりの台詞を出さない」と決めており、09-26 の裁定は「失敗はゴーストの台詞で伝える」。(a) は「ゴーストの口を借りて areka の言葉を出す」形なので、この 2 つの線の内か外かを開発者が決める。外なら (b) の変形（正典の意味から外れない載せ方が在るか）を設計の段で探すか、(c) を採って brief の目指す姿 1 を改める。
   - (b) の補足: 既定のゴースト emo2 の辞書は、Reference6 が `halt` で Reference7 が空でないときだけ `起動halt` の台詞を選び、ほかの Reference は読まない（`vendors/sample_ghost/emo2.nar` の `ghost/master/scripts/pasta/shiori/event/boot.lua`）。＝(b) で「書けない」を載せても、今の emo2 は喋らない。
   - 未確定: (a) の文面と、出す時点（`OnBoot` の台詞の後か、すぐか）。(a) を採るなら「ゴーストが応えない」（要件 2.7）は起きない。
2. **議題 2（インストールの失敗の語）**: The areka shall 要件 5.1 の語を次の候補から選ぶ。
   - 候補 (a) **`fileio`**（ukadoc の「ネットワーク失敗理由」の語・「容量不足による更新対象への書き込み失敗。」。areka のネットワーク更新が手元の書き込みの失敗に既に送っている語）。
   - 候補 (b) **`readonly`**（同じ一覧の「読み取り専用フラグによる更新対象への書き込み失敗。」）。権限の拒否・空き不足には字面が合わない。
   - 候補 (c) **areka の新しい語**（ネットワーク更新で `dns`・`connect` を足した前例の形）。
   - 候補 (d) **`extraction` のまま**、別の道（議題 1 の知らせ）で補う。
   - **推奨: (a) `fileio`**。理由: 正典の語の中で「書けない」を言う語で、areka の中でネットワーク更新と同じ失敗に同じ語を使える。インストールの一覧の外の語なので、辞書が知らなければ既定の台詞へ落ちる（今日の「壊れてるかも」より悪くならない）。検体の 5 体の辞書で確かめ済み（`research.md` §5.3: emo2 は今日と同じ・emily4 は誤った「壊れていると思うから取り直して」が消える・YAYA と里々の 3 体は語をそのまま見せる）。登記は §8 の行で「インストールの一覧に語が無いので、同じページのネットワーク失敗理由の `fileio` を借りる」と書く。
3. **議題 3（印が書けないときの扱い）**: The areka shall 要件 3.5 を次のどちらかで満たす。
   - 候補 (a) **知らせるだけ**（要件 3.1〜3.4）。補いの仕組みは作らない。
   - 候補 (b) 印の代わりの置き場（一時フォルダなど）へ書いて補う。
   - **推奨: (a)**。理由: (b) は記憶の置き場の決め方に触れ、`areka-P0-user-data-root` の持ち物と重なる（Out of scope）。書けない置き場を直す・移すのは利用者が知らせを見て行う。
4. **議題 4（同梱のゴーストの辞書への申し送り）**: The areka shall 要件 8.2 の申し送りを、次のどちらかで行う。
   - 候補 (a) **申し送る**（`.kiro/steering/roadmap.md` の申し送りの行と、この spec の完了の報告に、emo2 の辞書の持ち主＝pasta の側への頼みとして書く）。
   - 候補 (b) 申し送らない。
   - **推奨: (a)**。理由: `.nar` の失敗の語を分けても、emo2 が理由を見ない限り利用者に届く台詞は変わらない。
