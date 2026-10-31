# 設計の検証: seriko-pattern-calls

- 検証日: 2026-10-30
- 入力: `spec.json`（language: ja）・`requirements.md`・`design.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`）
- 突き合わせたコード（読んで確かめた所）: 行数（`looper.rs` 924・`table.rs` 901・`parts.rs` 893・`actor.rs` 843・`shell_target.rs` 865 は設計の数のとおり）、`plan.rs` の着せ替えの pattern0 だけの静的な経路と合成のたびの `ComposeMethod::from_name`（`plan.rs` 509〜535 行）、`is_bind_animation` の D9-3 の門、`world.rs` の `targets_animation_id`、`manifest.rs` の 7 語の写し、`timeline.rs` の `frame_at`、`DisplayCommand`（`#[non_exhaustive]` なし）と `PresentCommand`（あり）、`PlacementRoute` の網羅 `match`（`is_system_reanchor`・`deferral_covers_route`）、`enqueue_window_set_pos`、`project_anchor`、`guard_visibility`／`apply_visibility_guard`、`follow_balloon`、`on_balloon_drag_end`、1,000 行の見張りの例外表

## 設計の検証のまとめ

設計は、8 語を「描く語」から切り離して読み込みのときに 1 度だけ解き、seriko の再生・合成・置き方へ役目を分けて運ぶ形で、今の層と依存の向きに沿っている。コードの名前・行数・今の規則についての主張は、ほぼすべてコードのとおりだった。ただし `move` の置き方の段に、要件 7.8 の例（`move,0,10,0,9999` が作業領域の中へ戻る）を今の部品の組み合わせでは満たせない所が 1 つあり、バルーンのずれにも閉じていない経路がある。どちらも設計の議論で直せる範囲で、骨格の作り直しは要らない。

## 重大な指摘（3 件）

### 🔴 指摘 1: 設計が呼ぶ「見える範囲の守り」は Y を動かさないので、`move,0,10,0,9999` が作業領域の中へ戻らない

- **問題**: 設計は、ずらした後に `project_anchor` と `apply_visibility_guard` を当てれば「作業領域の中に置く決まり」を満たすとしている。しかしコードでは、`guard_visibility` は「Y は一切変更しない（Y は射影 T の所有）」と決めてあり、X だけを引き戻す（`placement/follow/visibility.rs` 59〜67・100〜103 行）。`project_anchor` も `Free`・`Left`・`Right` では Y をそのまま返す（`placement/follow/anchor.rs`）。だから下の端に揃えない置き方（実機の確かめで使う形）では、y に 9999×k を足した窓は画面の下へ出たままになり、見えなくなる。
- **影響**: 要件 7.8 の例と、設計のテスト（`move_shift_tests.rs` の「`move,0,10,0,9999` の形が作業領域の中へ戻ること」）と、実機の確かめの 3 番（「最後の `9999` は作業領域の中へ戻る」）が、書いた部品のままでは通らない。実装の段で初めて気づくと、Y の扱いを誰が持つか（射影 T の決めごと D6）にかかる判断を、場当たりで決めることになる。
- **直し方**: 今ある「作業領域の中に置く」部品を名指しして使う。候補は `placement/follow/work_area_clamp.rs` が使う `balloon_limit::clamp_rect_to_work_area`（左辺・上辺を優先して窓の矩形を作業領域の中へ入れる式）。`apply_char_shift` の手順 3 を「`project_anchor` → 作業領域の中への寄せ（`clamp_rect_to_work_area`）→ `apply_visibility_guard`」にし、記録の語（`[work-area-clamp]` の場面）を足すかどうかも決める。見える範囲の守りの「Y は動かさない」の決めごとは変えない。
- **要件との対応**: 7.8・9.4（3 番）
- **設計の該当箇所**: 「MoveShiftPlacement」の「キャラ窓への当て方」手順 3、「Testing Strategy」の `move_shift_tests.rs` と「実機」の 3 番

### 🔴 指摘 2: バルーンのずれが、バルーンのドラッグと保存で二重に数えられる

- **問題**: 設計はバルーンのずれを「キャラ窓 ＋ 相対 offset ＋ バルーンのずれ」で当てる。キャラ窓の側は、ドラッグの終わりでずれを 0 にし、保存ではずれを引く手当てがある。しかしバルーン窓の側には同じ手当てが無い。コードの `on_balloon_drag_end`（`placement/follow/drag_follow.rs` 657 行〜）は、offset を「バルーンの今の位置 − キャラ窓の位置」から作って保存する。ずれが当たったままのバルーンを利用者が動かすと、ずれを含んだ offset が保存され、`follow_balloon` がその上にずれをもう 1 度足す。
- **影響**: バルーンの面の `move`（要件 3.8 で同じ規則を当てると決めた所）を持つバルーンで、ドラッグの後にバルーンがずれの分だけ余計に離れ、その値が起動をまたいで残る。
- **直し方**: キャラ窓と同じ形にそろえる。`on_balloon_drag_end`（とドラッグ中に offset を書く所）で、バルーン窓の `MoveShiftApplied.applied` を 0 にするか、offset を作る前に `applied` を引く。どちらにするかを決め、`move_shift_tests.rs` に「バルーンのドラッグの後のずれ」と「バルーンの offset の保存値がずれを含まない」を足す。
- **要件との対応**: 3.8・7.1・7.6
- **設計の該当箇所**: 「MoveShiftPlacement」の「バルーン窓への当て方」と「他の書き手との関係」

### 🔴 指摘 3: ファイルの計画に、実在しない置き場と 1,000 行の上限に近いファイルの見落としがある

- **問題**: 2 点ある。(a) File Structure Plan は `placement/visibility.rs` を直すとしているが、そのファイルは無い。見える範囲の守りは `placement/follow/visibility.rs` にある（ただし経路の判定は `is_system_reanchor` へ委ねているので、実際に直すのは `diag.rs` の網羅 `match` だけで足りる見込み）。(b) 直すファイルのうち `placement/follow/drag_follow.rs` は 967 行、`placement/persist.rs` は 928 行で、1,000 行の見張り（`log-capture-kit/tests/file_length_guard_test.rs`）の例外表には `window_move.rs` しか載っていない。設計は上限の手当てを seriko の 4 ファイルにしか書いておらず、`drag_follow.rs` には指摘 2 の直しも乗る。
- **影響**: タスクの分け方が実在しないファイルを指し、`drag_follow.rs` が上限を超えれば全体テストの見張りが赤になる。
- **直し方**: (a) の置き場を `placement/follow/visibility.rs` に直すか、「`diag.rs` の `is_system_reanchor` に腕を足すだけ（visibility.rs は委ねているので直さない）」と書き換える。(b) `drag_follow.rs` と `persist.rs` に足す行は呼び出し 1〜2 行だけにし、ずれを引く本体は新しい `placement/follow/move_shift.rs` に置く、と設計に明記する。
- **要件との対応**: 7.1・7.8・9.5（既存のテストを通す）
- **設計の該当箇所**: 「File Structure Plan」の `areka/src/placement/` の行と、「実装の注」の上限の項

## 設計の良い所

1. **語の表を 1 か所に置き、読み込みのときに 1 度だけ解く形**。`pattern_word`／`call_targets` を唯一の正本にし、seriko の表・`BindSteps`・入れ子の辺・外形がその答えだけを使う。合成のたびの `from_name` の `warn!`（`plan.rs` 535 行）が構造の上で消え、5.4・5.5 を「記録を出す所が表を作る所しか無い」ことで満たしている。
2. **時刻と順の扱いが今の作りに素直に乗っている**。過去の時刻に始まった再生を入れる先例（`Armed::poll`・`fire_top_triggers`）に呼び出しの開始を重ね、`run_calls` を再生中の番号の写しの前に置いて 1 フレームの遅れを出さない。ずれを `PatternState` の外に持つことで、書き出しにずれが写らない（9.6・M8）ことと合成の覚えの鍵を守ることを、同時に構造で満たしている。

## 最終判定

- **判定: GO（条件つき）**
- **理由**: 層の境目・依存の向き・新しい依存なし・ログのない失敗の経路なし、の steering の決めに沿っており、要件 1〜11 のすべてに部品と口が対応している。3 件の指摘は、どれも `move` の置き方の段の中で閉じる直しで、骨格（語の表・働きのコマ・`CallPlays`・`BindSteps`・`MoveShift` の伝達）は変わらない。
- **次の手順**:
  1. 設計の議論（`/kiro-design-discussion seriko-pattern-calls`）で指摘 1〜3 を議題にし、決めを `design.md` に写す。特に指摘 1 は、Y を寄せる部品を名指しして決める。
  2. その後 `/kiro-spec-tasks seriko-pattern-calls` でタスクを作る。

## 細かな気づき（議題にしなくてよい）

- 「`SerikoLoopConfig { … }` を書く 23 か所」は、文字列で数えると 31 か所に当たる（構造体の定義・doc・`impl` の中を含む）。機械的な直しなので、数の違いはタスクの段で数え直せば足りる。
- 要件 3.6 の括弧書き（「この spec の前と同じ時刻に同じアニメーションが抽選で始まる」）を、設計は「呼び出しを持たないシェルでは同じ」と読んでいる。呼び出しが相手を再生中にすれば抽選から外れるのは今の規則どおりなので、読みとして筋が通っている。§8 に書く予定にも入っている。
