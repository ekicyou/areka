# Brief: areka-P0-shell-balloon-frame-align

> 2026-10-04 `/kiro-discovery` で起票（`shell-balloon` の完了の手順の中で、実機の確かめに残った気付きを拾った）。**種別はバグ**（目に付かない 1 フレームのずれと、それと同じ根の `Status` の 1 フレームの欠け・警告の段の行）。本文のソースの指し先は起票時（`b33c00ae`）の実物＝着手時に引き直すこと。

## Problem

- **ゴースト作者**: 箱のあるサーフェスへ切り替えたフレームに `Status` を受け取ると、`balloon(ID群)` から見えているはずのバルーンが 1 フレームだけ欠ける。そのフレームに届いた要求では、見えている箱を前提にした分岐が外れうる。
- **利用者**: 箱の置き場所が違うサーフェスへ替えた瞬間に、新しい絵の上に前の置き場所の文字が 1 フレーム（約 8 ms）残ることがある。目には付かない（2026-10-04 の実機の確かめ・`completed/areka-P0-shell-balloon/real-machine-check.md` の 3.2 と 4.1 の項目 4＝16 回中 2 回）。
- **開発者**: 普通のバルーンを一度出して隠した後に箱だけに文字が出ると、`WARN … event="balloon_status_surface_unknown"` が 1 度出る。設計どおり番号 0 で届いているが、箱を使う普通の使い方で警告の段の行が出るので、本物の警告に紛れる。

## Current State

- 文字の層が台本の `\s` を受け取るのはフレームの外で、置き場所に効くのは次のフレームの `sync_boxes`（`crates/areka-emo-text/src/actor_box.rs`）。絵の差し替え（seriko → `run_drain_phase`）が先に届いたフレームでは、新しい絵の上に前の置き場所の文字が残る。逆の向き（前の絵の上に新しい置き場所の文字）は seriko が別のスレッドなので起こりうる、と設計は書いている（完了 spec の design.md「描画は毎フレーム、…」の段落）。
- `\s` の受け取りで箱の四角の写し（`shown_boxes`）が外れ、次のフレームの提示で戻るまでのあいだ、`Status` の届け（`crates/areka/src/emo2_boot/frame/status_report.rs` の `report_balloons`・`box_showing`）が空を報告する（例 `bindings=[]` → 次のフレームで `BalloonBinding { character_id: 0, balloon_id: 0 }`）。
- 箱だけが見えているとき、番号は `current_surface_id`（取れなければ 0）から取る。普通のバルーンを隠した後は取れないので 0 で届け、`warn!` を 1 度出す。

## Desired Outcome

- 箱の置き場所の付け替えが、`\s` を受け取ったのと同じフレームの描画に効く（1 フレームのずれが 0 になる）。**1 フレーム遅らせて揃える手当ては取らない**（開発者方針「状態の持ち方を変えて 0 フレームで解け」）。
- 箱が見え続けているあいだ、`Status` の `balloon(ID群)` から箱のスコープが 1 フレームも欠けない。
- 箱だけが見えている普通の使い方で、警告の段の行が出ない（番号の持ち方を直すか、水準を見直すかは設計で決める）。
- どれも決定論のテストで、直す前に赤になる形を添える。

## Approach

設計で決める。候補は次のとおり。
- `\s` の受け取りの時点で、置き場所（登録）と `shown_boxes` の写しを同じフレームの中で付け替える（`sync_boxes` を待たない）。
- `Status` の届けを、写しが外れている間は「前のフレームで見えていた組」から作る（ただしこれは遅らせる手当てに近いので、上の案で根から消せるならそちらを採る）。
- 箱だけのときの番号は、普通のバルーンを隠した時点の番号を持ち続ける。

## Scope

- **In**: 箱の置き場所の付け替えの時機、`shown_boxes` の写しの外し方と戻し方、`Status` の `balloon(ID群)` の箱のスコープ、箱だけのときの番号と警告。
- **Out**: 普通のバルーンの置き場所と `Status` の振る舞い（今のまま）。seriko のスレッドの構成。

## Boundary Candidates

- `crates/areka-emo-text/src/actor_box.rs`（`sync_boxes`・`shown_boxes`・`hide_boxes`）と `state_route.rs`（`route_surface`）
- `crates/areka/src/emo2_boot/frame/status_report.rs`（`report_balloons`）と `crates/areka/src/emo2_boot/balloon_visibility_phase.rs`（届けの呼び出し）

## Out of Boundary

- 箱の当たり判定（字の矩形のマスク・完了 spec のタスク 13）。マスクも画素と同じく最大 1 フレーム前の姿が残るが、ここで画素の付け替えが同じフレームになれば一緒に揃う。
- `balloon-element-order`（画像のあいだに箱を挟む重ね順）

## Upstream / Downstream

- **Upstream**: `areka-P0-shell-balloon`（完了）
- **Downstream**: なし

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の気付きの追跡）
- **Adjacent**: 文字とバルーンの列の spec（`balloon-font-file`・`text-typesetting`・`talk-fast-forward` ほか）は `areka-emo-text` を共有するので並べない。`emo2_boot` の結線の列（`shell-companion-balloon`・`balloon-canon-residue`）とも `frame/` を共有しうる。

## Constraints

- 目に付かないので急がない（列の空きで入れる）。
- テストの決定論・1 ファイル 1,000 行未満（`crates/areka/src/emo2_boot/spine.rs` は起票時にちょうど 1,000 行で、足すと番人が赤）。
