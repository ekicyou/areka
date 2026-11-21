# 設計の検証: window-resetwindowpos-hold

- 日付: 2026-11-21
- 対象: `design.md`（`538e3499b`）・`requirements.md`・`research.md`
- 方式: `kiro-validate-design`（対話なし・報告をこのファイルへ残す）

## Design Review Summary

今ある「読み口越しに表示の事実を引き、預かりの判定で使う」形の延長で、新しい相・チャネル・資源を作らず、表示層を一次情報源に保ったまま並べ直しの表示待ちを足す、筋の良い設計である。コードと照らした主張（R1 の結論・`show.rs` の早期復帰の網羅・`Blocked.all` と「前に預かった移動を捨てる」腕の巡をまたぐ噛み合い）はいずれも成り立つ。残る論点は 3 つで、どれも設計の話し合いで決められ、作りの組み直しは要らない。

## コードとの照合（確かめた主張）

1. **R1（確定は先頭の窓を動かさない）: 成り立つ。** `resolve_chain`（`chain_finalize.rs` 421 行〜）は先頭のスロットでは `prev` が `None` なので式を当てず、今の x を基準にするだけである。`finalize_chain_once_with` が先頭を動かす道は `place_chain_in_work_area` の作業領域への寄せ戻しだけで、本当の幅が仮の幅より狭いときは寄せ戻しが起きない。再スナップが下端の中央を保つなら、先頭の右端は `R − (w0p − w0)/2` に残る。隙間 `(w0p−w0)/2 + (w1p−w1)/2` は確定が `JoinLeft` で消すので、赤の檻 (b)「確定が窓を動かさない」は直す前に確実に赤になる。フレームの順（`frame.rs` 216〜229 行: 再スナップ → 位置の指令の相 → ずれ → 確定）も設計のとおりである。
2. **`applied` が `None` の間だけ `never_shown` を書く: `show.rs` の早期復帰を全部覆う。** 未装着（target が無く欄も無い）・`GraphicsCore` 不在（`Err(Device)`）・`record_display` の失敗（`Err`）・`EmptyComposition`／`ExtentTooLarge`（`Ok` のまま隠す）・合成の失敗（`Err(Compose)`）の 5 つで、`Ok` かつ `applied == None` で返るのは隠す縮退の腕だけである。`show_or_hold` の預かりの腕は `last_show` が `Some`（＝`applied` が `Some`）のときだけ通るので混ざらない。実装の注意: `show_or_hold` は今は結果を返信へ渡して捨て、`apply_replace` は `let _ = self.apply_show(…)` で捨てているので、両方とも結果を受け取って欄を書く形へ改める必要がある（設計の「3 腕で書く」の中身）。
3. **`Blocked.all` と捨てる腕の巡をまたぐ噛み合い: 正しい。** 巡の頭で `held` は `mem::take` で空になり、前の巡から預かった並べ直しは列の先頭で判定されるので、その時点の `held` に移動は無く、並べ直しの後に預かった移動は捨てられない（2.3・2.4）。預かった並べ直しの後にもう 1 本の並べ直しが届いた巡は、間の移動だけを `move_hold_superseded` で捨て、2 本の並べ直しを順に預かる（結果は順のまま当てたのと同じ）。細かな点として、後の並べ直しは `blocked` の腕を通るので `reset_window_pos_held` でなく `move_hold_behind` を残す（5.1 の「待っているスコープ」は前の並べ直しの行で読めるので害は無い）。

## Critical Issues

🔴 **Critical Issue 1**: 表示しないと分かった窓への移動の扱い（枝 A／B）が未決のまま
**Concern**: 4.3 は SSP の測りを「設計の段」で行うと決めているが、設計は【SSP の測りの後に決める】と仮の既定（枝 B）を置いたままで、測りは済んでいない。
**Impact**: 枝で `move_hold_unshown` の `handling`・檻 5 の期待・§8 の書き足しが変わり、タスクを確定できない。
**Suggestion**: 設計の話し合いの中で排他テストの机を取って `research.md` §10 の手順で測り、片方の枝に決めて design.md と `research.md` に版・手順・結果を書いてから `/kiro-spec-tasks` へ進む。
**Traceability**: 4.1・4.3・5.4・7.6
**Evidence**: design.md「MoveHold 判定の規則 2」・「Open Questions 2」・「Implementation Notes」

🔴 **Critical Issue 2**: 「表示した」の判定が窓の大きさの着地を見ていない
**Concern**: `Settle::Shown` は `applied_ratio` が `Some` だけで決まるが、並べ直しの本体は `WindowPos.size` を読む。窓の大きさの照合（`reconcile_reported_sizes`）は整合待ちの札（`DpiSyncHold`）のある窓へは書かずに見送るので、表示が成り立った巡に窓の大きさがまだ仮のままの形がありうる。起動の確定はこれを `ResnapNotLanded` で待っている。
**Impact**: その巡に預かりを解くと、並べ直しは仮の大きさで当たり、直したい崩れ（先頭のずれ・確定の動き直し）が戻る。決定論の足場は窓の大きさをその場で揃えるので、檻では見えない。
**Suggestion**: 並べ直しの待ちの `Shown` を「表示が成り立ち、かつ窓の大きさが表示の大きさに着地した」とし、起動の確定（`collect_chain_states`）と同じ材料で読む。足さないなら、着地が同じ巡に必ず起きる理由を設計に書き、整合待ちの札のある形を檻に 1 本足す。
**Traceability**: 1.2・1.5・1.7・6.2
**Evidence**: design.md「スコープの表示の決まり」（`Shown` は `applied_ratio` が `Some`）・「今のコードの読み」

🔴 **Critical Issue 3**: 面の指令が一度も来ない窓で列全体が止まる
**Concern**: 窓はあるのに `\s` が一度も届かないスコープは `Pending` のままで、預かった並べ直しが `Blocked.all` を立て続けるので、表示済みの窓への後の指令まで際限なく預かられる。今の `move_hold.rs` の説明が「捨てないと列全体が詰まる」と退けた形を、並べ直しで作り直すことになる。
**Impact**: 3.4 の字面（表示しない窓 1 つで、表示済みの窓への後の指令を際限なく預からない）に背き、今は当たっている並べ直しと移動が止まる後退になりうる。申し送りの件（`audit-emo2-boot-frame`）は、表示しない窓が実在することを示している。
**Suggestion**: 起動のときに areka が窓ごとに最初の `ShowSurface` を必ず出すか（既定の面の表示）をコードで確かめて、出すなら不変条件として設計に書く。出さないなら、3.4 の読みを設計の話し合いの議題に上げ、開発者の決めをもらう。そのとき時間の上限・イベントで決める形は議題 2 の決めで取れないので、表示層で決まる事実の範囲で解く。
**Traceability**: 3.1・3.4・6.1
**Evidence**: design.md「MoveHold の Risks」・「Open Questions 1・3」

## Design Strengths

1. **事実の持ち主が 1 つ**: 「最初の面を表示しないまま決まった」を表示層の `apply` の結果だけから書き、位置の側は毎巡プルで読む。第 2 の帳簿を作らず、公開の入口 `run_move_drain_with` の形も `ShownRatioSource` の既定の実装で保つので、C24 の約束と依存の向きを崩さない。
2. **赤の檻が根に当たる**: R1 をコードの読みで追い、檻を「再スナップ → 位置の指令の相 → 起動の確定」まで通して、先頭の右端と確定が動かないことを見る形にした。隙間 0 だけを見る檻では今のコードでも緑になる点を見抜いている。

## Final Assessment

- **Decision**: **GO**
- **Rationale**: 作りは今の預かりの延長で、依存の向き・相の順・公開の形を変えずに要件を覆い、コードとの照合で主な主張が成り立った。3 つの論点はどれも設計の話し合いで決められ、作りの組み直しは要らない。
- **Next Steps**:
  1. 設計の話し合い（`/kiro-design-discussion window-resetwindowpos-hold`）で Critical Issue 1〜3 を議題にする。Issue 1 は SSP の測りを済ませて枝を決め、Issue 2 は待ちの判定に着地を入れるかを決め、Issue 3 は起動の既定の面の表示を確かめて 3.4 の読みを決める。
  2. design.md へ反映した後に `/kiro-spec-tasks window-resetwindowpos-hold` へ進む。
