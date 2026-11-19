# 設計の確かめ: audit-placement-follow-residue

> 2026-11-17 `kiro-validate-design`（対話なしで実行）。読んだもの: `spec.json`（言語 ja・phase design-generated）・`requirements.md`・`design.md`・`research.md`・steering の `product.md`・`tech.md`・`structure.md`・`roadmap.md` の C22 の行と同じウェーブの約束。測ったツリーは枝 `claude/kiro-start-audit-placement-follow-residue-68f4ba` の頭（`75a2fc545`）。

## 設計の確かめの要旨

設計は要件 1〜10 の件をすべて部品と流れに割り当てていて、要件の対応表に抜けは無い。設計の要になる主張を 5 つ、コードで確かめた。4 つはそのとおりで、残る 1 つ（記憶へ 1 回）は振る舞いは正しいが、檻と文への波及が設計の書き方より広い。ほかに `drag_follow.rs` の分け方の「呼び合いは一方向」という主張が 1 か所違う。どちらも直す所がはっきりしていて、作りを変える話ではないので、実装へ進めてよい。

### コードで確かめた主張

| 主張 | 結果 | 根拠 |
|---|---|---|
| 決め 1: `ChainRule::from_terms` の `#[cfg(test)]` を外せば記憶の戻しで鎖が組める | そのとおり | `chain_rule.rs:154-157` は `Self::sorted(scopes, true)` だけ。`prev(ChainView::Saved, n)` は列の中の「n より下で鎖に入る一番近いスコープ」を返す（`:192-198`）。`ScopePlacement::anchor` は resolver が `Anchor::from_alignment(&sc.alignment)` で出した値（`resolver.rs:289`）で、印の保存の `ChainRule::of` と同じ出し方。`reseed.rs:70` の呼び手は 1 スコープなので組は出来ない。G1 の grep は今 `chain_finalize.rs`・`chain_realign.rs`・`chain_rule.rs`・`persist_save.rs`・`resolver.rs` だけで、設計の予想どおり |
| 決め 2b: `on_char_drag_end` には始めの位置が出せないときの縮退が既にある | そのとおり | `drag_follow.rs:176-190`（`policy_mapped_position` が `None` なら `WindowPos.position` で確定と保存）。wintf の守りも既にある: `DragState::Dragging` の `move_window` は `ctx.move_window && ctx.initial_window_pos.is_some()`（`ecs/drag/state/mod.rs:260`）なので、位置の無いドラッグでは wndproc が窓を動かさない |
| 決め 5（D3）: 札が外れるフレームに拡大率の相が入れ直した揃え方で置く | そのとおり | 札のある窓は `Changed<DPI>` が無くても対象に入る（`frame/dpi.rs:355-363`）。札を外すのは `apply_dpi_phase_gate` だけ（`dpi_sync.rs:226-256`）で、通ったフレームに `reconcile_window_size` か `reproject_char_window_at_current_size` が `resize_window_to` を通す。揃え方（`Anchored`）は `reanchor_char_windows` より前に `apply_shell_descript` の `replace_anchor` が入れ替える（`reseed.rs:80`・`frame/switch.rs:564-573`）ので、見送っても新しい揃え方は失われない。`WorkAreaResnap` が同じ「解除フレームの 1 本へ合流させる」形を既に使っている（`dpi_sync.rs:265-276`）。`AnchorChange` と `DpiReproject` はどちらも `is_system_reanchor` の側（`diag.rs:315`）で、経路ごとの扱いの差は無い |
| 決め 7: 束を 1 つにしても書く印は同じ | 集まりは同じ | `send_entries` の外す印（`clamped_old_memory`）は「束が位置を書かないスコープ」なので、束を大きくしても外れる印は増えない。触れる組（`written` と `next(Saved, k)`）の和も同じ。値の違いは設計が書いたとおり（相手だけが触れる組の印を読む時点が、動かした窓の当て直しの後になる） |
| 要件 2.6: `DraggingState` を組む所・読む所の一覧 | 揃っている | `git grep -n "DraggingState *{"` の 11 か所と `initial_window_pos` の読み手（wintf・配置・`areka-runtime`・本体）が設計の「164」の一覧と合う。`balloon_lock_drag_tests.rs:313`・`dispatch_order_tests.rs:216`・`clickthrough/controller_tests.rs:26` は `WindowDragContext`／`DragState` の欄で、型は変わらない |

## 重大な指摘

### 重大な指摘 1: 記憶へ 1 回にしたときの、今の檻と書き手の一覧への波及が設計に無い

- **中身**: 設計は `follow_sticky_follow_drag_tests.rs` について「呼びの 8 か所は戻りを使わない形のまま・回数の檻を足す」と書くが、`finish_char_drag` を直に呼ぶ檻のうち 3 本（`finish_writes_partner_persists_logs_and_clears_session`・`finish_persists_partner_position_without_its_pattern_shift`・`finish_persists_partner_in_descript_form_not_current_anchor`、147・203・240 行）は、呼んだ後に `rig.saved()` で相手の位置が記憶に入ったことを確かめている。`finish_char_drag` が書かなくなると、この 3 本は落ちる。158・277・354 行の `let ((), events) = capture(..)` は戻りの型が変わるのでビルドが通らない。あわせて、記憶へ書く呼び手を並べた文（`persist.rs` の頭の doc 9〜12 行「書く呼び手は次の 4 か所」に `finish_char_drag` がある・`persist_save.rs:33` の `PersistWiring` の doc「書く呼び手 4 か所……3 か所は `persist_entries`」・`drag_follow.rs:135` の「`finish_char_drag` が……記憶へ書き」）が古くなるが、Modified Files に載っていない。
- **影響**: この 3 本は要件 5.2（束の中身＝相手は面の move のずれを引いた元の位置を descript の向きの形で）を今固めている唯一の檻。作り直し方を決めずにタスクへ渡すと、檻を消すか弱める直しになりやすく、要件 5.2 の確かめが抜ける。文が残ると要件 7.1（消えた名前・今の道と違う説明）に新しい件を足すことになる。
- **直し方**: 設計の「記憶へ 1 回」の Modified Files に、⑴ 3 本の檻は `finish_char_drag` の戻りの束を見る形に書き直す（`rig.saved()` でなく戻り値の `WindowPos` の鍵と値・「動かした窓の鍵は入らない」も戻り値で）、⑵ `capture` の型の 3 か所、⑶ `persist.rs` 9〜12 行・`persist_save.rs:33`・`drag_follow.rs:135` の書き手の一覧（4 か所 → 3 か所、組の相手はキャラ窓のドラッグの確定の束に入る）を足す。回数の檻（`commit_count` が 1）は `on_char_drag_end` を通す形で別に置く。
- **対応する要件**: 5.1・5.2・5.3・7.1・9.5
- **設計の場所**: 「File Structure Plan > Modified Files > 記憶へ 1 回（要件 5）」・「DragEndPersist」

### 重大な指摘 2: `drag_follow.rs` を分ける段の「呼び合いは一方向」が 1 か所違う

- **中身**: 決め 4 は「塊のあいだの呼び合いは一方向（⑴⑵ は ⑶ を呼ばない）」として 503〜1000 行を `follow/balloon_drag.rs` へ移すと書く。ところが移す塊の中の `known_position`（550 行）を、残す側の `guard_balloon_position`（488 行）が呼んでいる。移す側でも 565・733 行が呼ぶ。
- **影響**: 要件 3.2 の「分けるだけの段で、今のテストがそのまま通る」は守れるが、設計どおりに移すとビルドが通らず、置き場（どちらのファイルに置くか）と見える範囲（`pub(super)` か）をタスクの中で決めることになる。分ける段は 164 と番号の直しの前に置く順の頭（作業の順 1）なので、ここで迷うと後ろが全部待つ。
- **直し方**: 決め 4 に「`known_position` は `drag_follow.rs` に残して `pub(super)` にし、`balloon_drag.rs` は `super::drag_follow::known_position` で引く」（または逆）を 1 行足す。`CW_USEDEFAULT` の `use` も置いた側へ。行の見込み（約 500／約 510）はほぼ変わらない。
- **対応する要件**: 3.1・3.2・3.3
- **設計の場所**: 「決め」の表の 4・「File Structure Plan > Modified Files > 1,000 行（要件 3）」

## 設計のよい所

1. **4 回目（`AnchorChange`）の直し方 D3 が、今ある仕組みにそのまま乗っている。** 札を外すのは拡大率の相だけ・外れるフレームに札のある窓が必ず対象に入る・揃え方は前もって入れ替わっている、の 3 つがコードで成り立っていて、見張りの表（`deferral_covers_route` と檻の `EXPECTED`）を 1 行も変えずに「見送りが覆う経路」を本当にする。作業領域の再スナップが同じ形を既に使っているので、新しい決まりを足さない。設計の段で一時の試しを回し、見張りが `route=Some(AnchorChange)` で鳴ることを確かめてから決めている点もよい（要件 6.1）。
2. **164 を型で持たせ、「在る＝ドラッグ中」の意味を変えない。** `DraggingState.initial_window_pos` を `Option` にし、`None` でも挿す形なので、在る無しだけを見る詰め直し・寄せ・面の move のずれ・予約の 8 か所は触らずに済み、(0,0) との取り違えはビルドが止める。前例（`WindowDragContext.initial_window_pos: Option<Point>`）と同じ形で、wintf の中の守り（`move_window && is_some()`）とも合う。公開の段（wintf 0.2.0 の壊れる変更）への影響も要件 2.7 どおり書いてある。

## 結論

- **判定: GO**
- **理由**: 鎖の決まり・wintf のドラッグ・拡大率の相と札のどれとも作りがぶつからず、要の主張はコードで確かめられた。上の 2 つは設計の Modified Files と決め 4 に数行足せば閉じる直しで、作り直しは要らない。
- **次の手**:
  1. 設計の討議で、重大な指摘 1・2 と、設計の「未決」1〜6 をまとめて決める。
  2. 決めたことを `design.md` に入れてから `/kiro-spec-tasks audit-placement-follow-residue` でタスクを作る。

### 補足（重大ではないもの）

- 未決 6 の「C22 の触るファイルの列の外」には、ほかに `crates/areka-placement/src/lib.rs`（要件 4.6 の「根を引く子」の文）・`persist_save.rs`（指摘 1 の doc）・`spawn_follow_pipeline_tests.rs`・`balloon_lock_drag_tests.rs`（358・426 の写し）が入る。どれも `areka-placement` の中で、同じウェーブに触る相手はいない（約束 ⑷）ので、並べて書いておけば足りる。
