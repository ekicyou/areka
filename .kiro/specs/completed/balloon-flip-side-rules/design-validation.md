# 設計の確かめ: balloon-flip-side-rules

- 日付: 2026-11-16
- 対象: `design.md`（2026-11-16T13:00 生成）・`requirements.md`（承認済み）・`research.md`
- 進め方: `kiro-validate-design`（Analysis → Critical Issues → Strengths → GO/NO-GO）。対話なしで判定まで出す。

## まとめ

判定の核（`place_balloon_in_work_area`）に段を 1 つ足し、2 つの関門が「判定に入れるキャラ窓の矩形の列」を渡すだけ、という形は今の「純関数の核と薄い関門」をそのまま広げており、境界（`crates/areka/src/runtime/` に触れない）も守れている。コードで確かめた前提（`ChainFinalized` の挿入の時点・`FrameFinalize` の書き込みが同じ巡で流れること・`\s[-1]` が子の `Visual` だけを不可視にすること・P5 の式）はどれも設計の書きどおりだった。残る懸念は、判定し直しの場面を起動の並べ直しだけに絞ったために同じ型の取りこぼしが残る点と、判定し直しの系に 6.4 の檻が無い点の 2 つで、どちらも設計の話し合いで閉じられる大きさ。

## コードで確かめたこと

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| `ChainFinalized` は起動の並べ直しの書き込み（`move_window_to`）の後に入る | `crates/areka/src/runtime/frame/drain_resnap.rs` 560〜593 行（`move_window_to` の繰り返し → 既定位置の更新 → `world.insert_resource(ChainFinalized)`） | そのとおり。挿入はここ 1 か所だけ |
| `frame_system` は `Update`・`FrameFinalize` は同じ巡の最後の段・窓の指令は巡の後に流れる | `runtime/mod.rs` 743 行（`add_systems(Update, frame_system…)`）・`wintf/src/ecs/world/mod.rs` 596〜620 行（13 段の順）・`wintf/src/runtime/tick_bridge.rs` 252 行（`try_tick_world` の後に `flush_window_pos_commands`） | そのとおり。1 フレーム遅らせない |
| `ChainFinalized` は窓の一式ごとに 1 度（終了・入れ直しで外れる） | `crates/areka/src/app_exit.rs` 203 行（`close_windows_for_restart`） | そのとおり。SHIORI だけの読み直しでは外れない（＝判定し直しも走らない・問題なし） |
| 表示の有無は子の `Visual.is_visible`・`\s[-1]` は子を不可視にする | `areka-emo-present/src/mount.rs`（`emo-surface` と `emo-text-layer-slot` を `ChildOf(window)` で装着・`set_visible` が両方を切り替える）・`presenter/hub.rs` 240 行（`apply_hide`） | そのとおり。ほかに窓の子になるのは箱の文字の面（`areka-emo-text/src/surface_window_child.rs`・`Visual::default()`＝可視）で、こちらは絵の切り替えと非表示で箱の同期が外す（`actor_box.rs` 160〜163 行） |
| 判定の呼び手は 2 か所 | `balloon_limit.rs` 273 行（起動の関門）・`follow/window_move.rs` 1077 行付近（書き込みの関門） | そのとおり |
| P5 は左右とも `ox` を右向きに足す・`ox` は descript の分と `windowposition.x` の分の和 | `resolver.rs` 354〜391 行・`lib.rs` `scale_then_merge`（換算 → `windowposition` の合流の順） | そのとおり |
| `ScopeConfig` を全欄列挙で組むのは `resolve_scope` と `resolver_balloon_keyword_tests.rs` だけ | `ScopeConfig {` の全出現（ほかは `..ScopeConfig::default()`） | そのとおり |
| 登録の 1 行は `runtime/` の外 | `crates/areka/src/ghost_session.rs` 67〜100 行（`balloon_syncscale::register(world)` が末尾） | そのとおり。`runtime/zorder_wiring_tests.rs` が `include_str!("../ghost_session.rs")` で字面を読むが、重なりの結線の行を探すだけで、末尾に 1 行足しても影響しない |
| `window_move.rs` は 1,000 行の例外表 | `crates/log-capture-kit/tests/file_length_guard_test.rs` 67 行（パスで載る・行数の上限は持たない） | そのとおり。数行の増分は通る |

## 重大な指摘

### 重大な指摘 1: 判定し直しを起動の並べ直しだけに絞ると、シェル位置初期化と拡大率の遷移の後に同じ重なりが残る

- **懸念**: 前提 A5 は判定し直しを `ChainFinalized` の挿入だけにした。ところがシェル位置初期化（`reset_pos::reset_window_positions`・110 行）も、拡大率の遷移の後の解き直し（`chain_realign.rs`・393 行の `place_chain_in_work_area(…, ClampScene::DpiRealign)`）も、起動の並べ直しと同じく「スコープを 1 つずつ動かし、自分のバルーンだけを随伴させる」。本体を先に動かすと、本体のバルーンは相方が動く前の矩形で判定され、相方が本体の右へ来た後も並べ替えた位置のまま残る。要件 2.10 が直す型の取りこぼしが、利用者がメニューから選べる操作でそのまま起きる。
- **影響**: 要件 2 の目的（相方の絵がバルーンに隠れない）が、起動の直後だけ守られ、シェル位置初期化の後と、モニタをまたいだ後には守られない。どちらも `areka-placement` の中の手続きなので、`runtime/` の約束とは関係なく直せる。
- **提案**: 判定し直しの本体を `fn rejudge_all_balloons(world: &mut World)` の素の関数にし、⑴ `ChainFinalized` の系（今の設計） ⑵ `reset_window_positions` の書き込みの後 ⑶ `chain_realign` の書き込みの後、の 3 か所から呼ぶ。要件 2.7・2.10 の「起動の並べ直し」を「鎖の寄せ・並べ直し（起動・シェル位置初期化・拡大率の遷移の後）」へ広げるかを話し合いで決める。広げないなら、取りこぼしを `doc/COMPAT_ARCHITECTURE.md` §8 と追跡の起票（`/kiro-discovery`）に残す。
- **出どころ**: 要件 2.7・2.10・決めごと 2
- **根拠**: design.md「前提と未決の論点」A5・「System Flows／起動の並べ直しの後の判定し直し」

### 重大な指摘 2: 判定し直しの系に「動かない」の檻が無く、守りを外した書き込みを全スコープへ出す

- **懸念**: 判定し直しの系は全スコープのバルーンを `BalloonFollowTrigger::Placement(PlacementRoute::MoveCue)`（可視性の守りを当てない引き金）で随伴させ直す。テストの方針の 6.4 の見張り（Integration 5）は既存の檻が緑のままであることに頼るが、既存の檻はどれも `ghost_session::register_systems` を通らないので、この系を 1 度も回さない。「重ならず横のずらしが 0 のバルーンは 1 ピクセルも動かない」が判定し直しの道では檻に入っていない。加えて `windowposition.limit,0` のバルーンは関門が素通しするので、直前の書き込みが可視性の守りで寄せた位置だった場合、守りなしの位置へ書き戻す余地がある。
- **影響**: 要件 6.4（配布版のぱすた・emo2 の見た目を保つ）が判定し直しの道で黙って崩れても気付けない。
- **提案**: ⑴ 判定し直しの対象を `BalloonLimit(true)` のバルーンに絞る（limit 0 は要件 5.6 で並べ替えないので、判定し直す意味が無い・1 行の絞り込み） ⑵ `follow_other_chars_tests.rs` に「重ならない形で `ChainFinalized` を入れて系を回しても、全スコープのバルーンの `WindowPos` が前とビット同一で、`[balloon-limit] Clamp` の行が増えない」の檻を足す。
- **出どころ**: 要件 6.4・2.7・6.2
- **根拠**: design.md「RejudgeSystem」・「Testing Strategy／Integration Tests」5

## 気づいた点（重大ではない）

- **A2（表示の読み方）**: 「子の `Visual` のどれかが可視なら表示している」は、今の窓の子（`emo-surface`・`emo-text-layer-slot`・箱の文字の面）では成り立つ。箱の文字の面は `Visual::default()`（可視）で切り替わらないが、非表示のときは箱の同期が外す。将来ほかの子が窓に付くと黙って「表示している」に倒れるので、`shown_char_rects` の説明に「どの子を見ているか」を 1 行書いておくとよい。
- **`doc/COMPAT_ARCHITECTURE.md`**: 同じウェーブの `runtime-crate-split` もこの文書のパスを書き換える（同 brief 123 行）。§8 の 1 行どうしなので競合は小さいが、「Revalidation Triggers」に載せておくと後の rebase で迷わない。

## よいところ

1. **規則を核 1 本に置き、自分の窓を列から外さない形にした**。要件 2.4 の「自分の窓と重なる部分は数えない」を交わりの内包で書いたので、呼び手は全スコープの矩形をそのまま渡せばよく、2 つの関門に分岐が増えない。3 体目の窓（`extra-character-windows`）にも規則の変更なしで効く。
2. **件 2 を合流の前の値で解いた**。`balloon_offset_descript_x` を別の欄で運び、P5 の右の腕で `ox − 2d` にするだけで、`windowposition.x` の分を反転しない（ukadoc の 2 つの記述の違い）ことと、3.3 の一致（p＝0 で R − d）が式で示されている。`balloon_offset` を P5 の結果から組む事後条件のおかげで、追従・基準対・シェルの差し替え・シェル位置初期化が式を通るだけで揃う。

## 判定

**GO**

- **理由**: 境界・依存の向き・記録の語の約束を崩す点は無く、前提はコードで確かめられた。重大な指摘 2 つは、判定し直しの範囲（要件の読みの話し合い）と檻の追加で、どちらも設計の形を変えずに閉じられる。
- **次の手順**: 設計の話し合い（`kiro-design-discussion`）で重大な指摘 1（判定し直しの場面を広げるか・広げないなら §8 と起票に残すか）と重大な指摘 2（limit 1 への絞り込みと 6.4 の檻）を決め、design.md に反映してから `/kiro-spec-tasks balloon-flip-side-rules` へ進む。
