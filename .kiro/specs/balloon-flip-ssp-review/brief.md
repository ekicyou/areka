# Brief: balloon-flip-ssp-review

> 2026-11-16 `balloon-flip-side-rules` の要件の段で、SSP の測り（`.kiro/specs/completed/balloon-flip-side-rules/research.md` 9 節）の結果を受けて `/kiro-discovery` で起票した。開発者の裁定は「そのまま＋起票」。`balloon-flip-side-rules` の決めごとは今のまま進め、並べ替えと押し込みを残すかどうかの見直しはこの spec が引き取る。区分 **A**（開発者が起票を頼んだ）。

## Problem

- 2026-10-30 の `window-work-area-clamp` は、SSP と同じ振る舞いにする、という裁定で、バルーンの並べ替えを入れた（要件 5.3〜5.8）。キャラ窓が作業領域に収まっていてバルーンだけが横にはみ出すとき、バルーンをキャラ窓の反対の側へ移す、という決まりである。
  - 裁定の根拠は、SSP 2.9.26 のぱすたさんの形だった。窓が画面の左端にあり、相方のバルーンが窓の右隣に出ていた（`completed/window-work-area-clamp/research.md` 13 節）。
- 2026-11-16 に SSP 2.9.34 で測ると、SSP はバルーンを並べ替えなかった。
  - 測った形: えも2DEBUG・バルーン `kakukaku for emo-gs`。キャラ窓は作業領域に収まり、バルーンだけが横に 63px・103px はみ出す形。
  - 窓の動かし方: `\![move]` で寄せた場合と、マウスのドラッグで寄せた場合の両方。
  - 結果: どちらも、バルーンを並べ替えも押し込みもしなかった。バルーンは画面の端で切れたまま出た（画面を撮って確かめた）。窓のほうは、ドラッグの後に作業領域の下端へ押し上げた。
- 同じ日にぱすたさんも測った。相方のバルーンは、窓の左に 845px の余地がある位置でも右隣に出ていた。つまり 10-30 の根拠は、並べ替えではなく相方のもともとの側だった疑いがある。
- areka はバルーンの押し込みもしている（完了 `areka-P0-windowposition-limit`）。ukadoc の `windowposition.limit` は既定 1 で、「バルーンを強制的に画面内に維持するかどうか。1で維持」と書く（https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.limit）。ところが SSP 2.9.34 は、上の形でバルーンを画面の中へ戻さなかった。押し込みは ukadoc には合っているが、SSP とは違う。

## Current State

- 並べ替え: `crates/areka-placement/src/balloon_limit.rs` の `place_balloon_in_work_area`（起動の関門 `apply_balloon_limit_one` と、書き込みの関門 `follow/window_move.rs` の `apply_balloon_limit_gate` の 2 か所から呼ぶ）。記録は `[balloon-limit] Clamp … how="flipped"`／`"pushed"`。
- 押し込み: 同じファイルの `limit_correction`（`windowposition.limit` が 1 のとき）。
- `balloon-flip-side-rules` は、並べ替えの上に 2 つの直しを乗せる（並べ替えた先がほかのキャラ窓に重なるときは押し込みへ倒す・右に出すバルーンの横のずらしの向き）。決めごとは、どちらも areka の裁量として COMPAT §8 に書く。
- `doc/COMPAT_ARCHITECTURE.md` §8 の「【上書き】バルーンだけが作業領域からはみ出すとき」の行は、10-30 の読み（SSP は並べ替える）を前提にしている。

## Desired Outcome

- バルーンだけが横にはみ出すときの areka の決まりを、SSP の振る舞いを測ったうえで開発者が決め、その通りに動く。案は次の 3 つ。
  - ⑴ 今のまま（並べ替え→入らなければ押し込み）とし、areka の裁量として §8 に残す。
  - ⑵ 並べ替えをやめて押し込みだけにする（ukadoc の `windowposition.limit` どおり）。
  - ⑶ SSP と同じく、はみ出しを許す。
- §8 の行を、測った事実に合わせて書き直す。

## Approach

- 要件の段の最初に、`balloon-flip-side-rules` で測れなかった場面を SSP で測る。測る前には開発者の許しを取り、同じウェーブのセッションへ知らせる。
  - はみ出しの大きい形（バルーンの大半が作業領域の外へ出る形）
  - 起動の直後（記憶の位置が画面の端のとき）
  - 面の切り替えでバルーンの大きさが変わるとき
  - `windowposition.limit,1` を明示に書いたバルーン（入っているバルーンに無ければ、測れなかったと残す）
- ukadoc と SSP が食い違うと測りで確かめたら（`windowposition.limit` の既定 1「バルーンを強制的に画面内に維持」と、SSP がはみ出したバルーンを戻さないこと）、開発者が ukadoc の製作元へ報告する（2026-11-16 開発者「ukadocとSSPが異なるならukadoc製作元に報告しておく」）。報告の文案（測った版・検体・手順・値・画面の画像）はこの spec が用意する。
- 測った結果で、上の 3 案から開発者が裁定する。⑵ か ⑶ なら、`balloon-flip-side-rules` が並べ替えの上に足した段（重なりの判定・映しの式）の扱いもここで決める。

## Scope

- **In**: 並べ替えと押し込みの決まりの見直し・SSP の測り・§8 の行・決めた決まりの決定論のテスト。
- **Out**: キャラ窓の鎖の寄せ（`follow/work_area_clamp.rs`）・キャラ窓を作業領域へ戻す決まり・バルーンの縦の置き方・キーワードの横の位置の式。

## Boundary Candidates

- 並べ替えと押し込みの判定（`balloon_limit.rs` の `place_balloon_in_work_area`・`limit_correction`）
- §8 の行と、完了した spec の決まりを改める記録

## Out of Boundary

- `balloon-flip-side-rules` が足す、重なりの判定と右の横のずらしの向き（その spec が持つ。この spec は結論しだいで扱いを決めるだけ）
- 利用者がバルーンをドラッグして離したときの補正

## Upstream / Downstream

- **Upstream**: `balloon-flip-side-rules`（測りの事実と、並べ替えの上に足した段。先に着地させる）。完了 `window-work-area-clamp`（要件 5.3〜5.8・`research.md` 13 節）・完了 `areka-P0-windowposition-limit`（押し込み）。
- **Downstream**: `extra-character-windows`（3 体目の窓の並べ替え）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の決まりの見直し。完了 spec の本文は書き換えず、§8 に改めた事実を足す）。
- **Adjacent**: `balloon-flip-side-rules`（同じ `balloon_limit.rs`＝同じウェーブに置かない）・`audit-placement-follow-residue`（`balloon_limit.rs`）・`balloon-canon-residue`。

## Constraints

- SSP を測るときは毎回開発者の許しを取る。画面の入力は排他テストの机で順番を取る。試験の根・検体はワークツリーの `target\` の下だけに置く。
- ゴーストの名前や初回の起動を見る分岐を入れない。
- 失敗の道は必ず記録する（`error!`＋`Err`）。記録の語は今の `[balloon-limit] Clamp` の欄で足す。

- **優先度の区分**: A（開発者が起票を頼んだ）。
- **版**: v0.2.0（`balloon-flip-side-rules` の後）。
- **要件定義のモデル**: Opus（SSP の測りと、今ある決まりを残すかの裁定。新しい境界や並行の設計には当たらない）。
- **規模**: S〜M（⑴ なら記録と §8 だけ。⑵ なら並べ替えを外し、檻を書き換えるので 4〜8 タスク）。
- **要件の段の議題**: 上の 3 案のどれにするか。

## 2026-11-17 棚卸㊴の再測定（main 7f41b53e）
- **前提の変化**: 待っていた `balloon-flip-side-rules` は 11-17 に着地した（PR#469）。`place_balloon_in_work_area`（`balloon_limit.rs` 172 行）は判定に入れるキャラ窓の列 `chars` を受け、映した先がほかの窓に重なれば `BalloonPlace::PushedOverlap` で押し込みへ倒す。記録 `[balloon-limit] Clamp` に欄 `overlap_scope` が付いた。書き込みの関門は `follow/window_move.rs` の `apply_balloon_limit_gate`（984 行・列は 1078 行の `shown_char_rects`）。起動の並べ直しと鎖の寄せの後に全部のバルーンを判定し直す系（新しい `follow/other_chars.rs`・登録は本体の `ghost_session.rs` 100 行・呼び手は `reset_pos.rs` 134 行と `chain_realign.rs` 427 行）も入った。
- **古くなった記述**: Current State の「§8 の行は 10-30 の読み（SSP は並べ替える）を前提にしている」は古い。`balloon-flip-side-rules` が §8 の「【上書き】バルーンだけが作業領域からはみ出すとき」の行（`doc/COMPAT_ARCHITECTURE.md` 431 行）に、2.9.34 の測りと、見直しをこの spec が持つことを書き足した。同じ節の「乗せる」は「乗せた」と読む。測っていない場面（はみ出しの大きい形・起動の直後・面の切り替え）は同 spec の `research.md` 9.3 のとおり今も測っていない。
- **触るファイル**: 案 ⑴＝§8 の行だけ（記録を足すなら `balloon_limit.rs`）。案 ⑵・⑶＝`crates/areka-placement/src/balloon_limit.rs`（360 行）・`balloon_limit_flip_tests.rs`（580 行）・`follow/window_move.rs`（1,234 行・上限の例外表に載る）と §8 の行。重なりの判定と判定し直しの系も外すなら、さらに `follow/other_chars.rs`・`follow_other_chars_tests.rs`・`follow.rs`・`reset_pos.rs`・`chain_realign.rs`・本体の `crates/areka/src/ghost_session.rs`（登録の 1 行）と、`resolver_resolve_balloon_tests.rs`・`balloon_offset_supply_tests.rs` ほかの檻。
- **規模**: S〜M（⑴ 2〜3・⑵ 4〜8・⑶ で判定し直しの系も外すと 6〜10 タスク）。20 は越えない。
- **先に要るもの**: 働きの依存は無し。ファイルでは `audit-placement-follow-residue`（`balloon_limit.rs` の 491・`balloon_limit_flip_tests.rs` の `point_of` の写し・`window_move.rs` の文・`follow*` の範囲に入る `other_chars.rs`）を待つ＝roadmap の「v0.2.0 の作り直しと触るファイルが重なる見直しは先に済ませる」。要件の段の最初に SSP の測りがある（毎回開発者の許し・画面は排他テストの机で順を取る・同じウェーブのセッションへ知らせる）。
- **同じウェーブに置けない相手**: `audit-placement-follow-residue`。案 ⑵・⑶ で判定し直しの登録を外すなら `session-module-gather`（`ghost_session.rs`）とも重なる。`balloon-text-visual-tree` とは、箱の文字の面の名前を `other_chars.rs` が読む所でつながる（両方が触るなら順を決める）。`window-resetwindowpos-hold` は `reset_pos.rs` を読むだけなので 0。
- **優先度の区分**: A（開発者が起票を頼んだ・2026-11-16「そのまま＋起票」）。版は v0.2.0。
- **要件定義のモデル**: Opus（SSP の測りと、今ある決まりを残すかの裁定。新しい境目・並行の設計・骨格の分かれ目に当たらない）。
- **分割の案**: 無し。
