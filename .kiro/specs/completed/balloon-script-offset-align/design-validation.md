# 設計の確かめ: balloon-script-offset-align

- 対象: `design.md`（2026-11-27 生成）・`requirements.md`（承認済み）・`research.md`
- 確かめ方: `kiro-validate-design`（対話なし・判定まで）。設計の主張はコードを読んで突き合わせた。

## 設計の確かめの要約

既存の「受け手の表 → 線 → move drain → placement の当て口」の型と、標準の位置を解く 1 か所（`resolve_standard_scopes`）への重ねで、要件 1〜8 をほぼ漏れなく受けている。主張の要（合流の 1 か所・起動の関門が台本の値を見ないこと・記憶の書き手の場所・`persist.rs` と約束の 5 本に触れないこと）はコードと合っていた。残る重い点は、0 フレームの主張の檻が move drain の 1 呼び出しまでしか無いことと、この spec の本題である「DPI の違う画面へバルーンを出す」形の振る舞いを実機任せにしていることの 2 つで、どちらも設計の討議で詰められる。

## コードとの突き合わせ（主張ごと）

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| 標準の位置の解きは `resolve_standard_scopes` の 1 か所に集まる（シェルの切り替え・面の `windowposition`・バルーンの切り替え・初期化） | 呼び手は `reseed.rs:69`（`apply_shell_descript`）・`face_windowposition.rs:76`・`reset_balloon_pos.rs:61` の 3 つ。バルーンの切り替えは `switch.rs:644` の `finish_balloon` → `balloon_face.rs:62` の `select_balloon_face` → `apply_face_windowposition` を通る。ほかの `resolve_placement` の呼び手は起動（`lib.rs:270`）とキャラ窓だけの並べ直し（`reset_pos.rs:103`）で、バルーンのずらしを解かない | 合っている |
| 合流が `balloon_x_mode` を上書きする（向きは合流の後に重ねる） | `lib.rs:625` の `sc.balloon_x_mode = x_mode` | 合っている |
| 右に出すときの符号の反転と換算が台本の基準点にも効く | `resolver.rs` の P5 が `balloon_offset_descript_x` の 2 倍を引く。`overlay_offset` がこの欄を書くので効く | 合っている |
| 起動の関門は台本の値を見ない | `apply_balloon_limit` の本番の呼び手は `areka-session/src/boot_seam.rs:32` だけ（spawn の前・World なし）。台本の値は保存せず窓ごと消える | 合っている |
| 押し戻しは 3 か所（＋判定し直し） | `balloon_limit.rs:190`・`window_move.rs:1017`・`balloon_drag.rs:454`。判定し直し `rejudge_all_balloons`（`other_chars.rs:62`）は `follow_balloon` → `window_move.rs` の関門を通るので、3 か所の変更で覆われる | 合っている |
| 記憶の書き手はドラッグの離しの 1 か所・分を引いても `persist.rs` に触れない | `balloon_drag.rs:277` の `let persist = offset_tl;` を直すだけで、`balloon_offset_entries`・`persist_entries` は今の口のまま。`persist.rs:556-572` の `merge_scope` は記憶があれば未係留の基準で置き換える（`apply_shell_descript` で分を足す条件 `dpi.is_none()` と合う） | 合っている |
| 約束の試験の支え 5 本 | `follow_test_support.rs`・`persist_finalize_save_tests.rs`・`frame_persist_test_support.rs`・`move_cue_apply_move_tests.rs`・`placement_tests/reset_balloon_pos_tests.rs` はどれも File Structure Plan の変更の列に無い | 合っている |
| move drain と表示の順 | `frame.rs:200` 取り出し → `:208` 可視性 → `:212` 窓寸の照合 → `:221` 再スナップ → `:227` move drain。同じ `frame_system` の中 | 合っている（ただし重い点 1） |
| 行の参照 | 台帳 3043・3088 行目、`COMPAT_ARCHITECTURE.md` 451 行目、`lib.rs:8` の「21 のモジュール」 | 合っている |

## 重い点（Critical Issues）

🔴 **重い点 1: 「同じ巡で書く」の檻が move drain の 1 呼び出しまでで、預かりがバルーンの可視より遅れる道が残る**
- **懸念**: 2 つの変種は `Landed`（表示した上で窓寸の照合まで済んだ）を待って預かる（`drain_resnap.rs:162-189`）。一方バルーンの可視は 9 相で台詞があれば決まり、この待ちを見ない。窓寸の照合が見送られた巡（`drain_resnap.rs:162` のコメントの場面）や、同じスコープの預かった `\![move]` の後ろでは、バルーンが descript の位置で見えてから次の巡以降に台本の位置へ跳ぶ。設計は後者だけを例外に挙げ、前者に触れていない。`frame_balloon_script_drain_tests.rs` は `run_move_drain_with` の 1 回しか見ないので、この食い違いを赤にできない。
- **影響**: ゴールの「フレームを遅らせない」と要件 2.6・2.7・3.8 が、初めの台詞で台本の位置へ出したいゴーストで破れうる。Open Question 6 が「段 4 で目で見る」としているが、決定論で測れる事柄を実機へ回している。
- **勧め**: ⑴ `frame_system` の段で「バルーンの可視が真になる巡には、その窓の `WindowPos` がもう台本の位置にある」を固定するテストを足す（窓寸の照合が見送られる巡を含める）。⑵ 状態の持ち方で解く案を設計で決める。例: 値を Component へ入れるのは預かりに関わらず drain の時点にし、置き直しだけを着地の時に回す（着地の道で Component のあるスコープを解き直す）。または、そのスコープのバルーン側の指令を預かっている間はバルーンの可視を出さない。
- **Traceability**: 2.6・2.7・3.8・8.1（Goals「指令の当ては、指令と同じ巡で画面へ出る」）
- **Evidence**: design.md「System Flows」の「預かり」「同じ巡」・「Testing Strategy」の `frame_balloon_script_drain_tests.rs`・Open Question 6

🔴 **重い点 2: 本題の形（バルーンだけが DPI の違う画面へ移る）の振る舞いを決めず、実機の段 4 任せにしている**
- **懸念**: バルーン窓は自分の DPI で大きさを変える（`frame/dpi.rs` の DPI の相は `BalloonWindowMarker` の窓も扱う）。変わった大きさの書き込みも `window_move.rs` の関門を通り、`limit_work_area` は変わった後の矩形の中心で画面を選び直す。左に出すバルーンは追従の値が `−幅 + ずらし` で、幅が変わると本体との隙間が変わる（Risks 2）。大きくなって中心が本体の画面へ戻れば本体の画面へ押し込まれ、作者が置いた画面から外れる（Risks 1）。設計は「決定論のテストでは窓の大きさが DPI で変わらない」としてどちらも段 4 で見るが、placement の World のテストでは、2 つの画面の `MonitorSnapshot` でバルーン窓の寸だけを書き換えて関門を通せば測れる。
- **影響**: 要件 6.1・6.2・9.2 の受け入れそのものが実装の後まで分からない。揺れたら選び方を「前に選んだ画面を優先する」へ変える（Open Question 4）なら、`limit_work_area` の形（状態を持つかどうか）が実装の後に変わる。
- **勧め**: ⑴ 「バルーンの寸だけが変わった書き込み」を `balloon_limit_monitor_tests.rs` に足す（小さくなる・大きくなる・境に近い形。選ぶ画面が変わらないか、変わるならどう変わるかを固定）。⑵ 左に出すバルーンの隙間を、今のまま受け入れるのか、右端を本体へ保つよう直すのかを設計で決め、決めを §8 の行に書く（要件の決め直しが要るなら討議で 9.4 に回す）。⑶ 揺れの手当てを入れないなら、入れないと決めた理由（押し込んだ結果が選んだ画面に収まる・その画面の DPI で 1 回で止まる）をテストで示す。
- **Traceability**: 6.1・6.2・6.5・9.2・9.4
- **Evidence**: design.md「Risks」1・2・「Open Questions」4・「Testing Strategy」の「実機（9.2）」

## 設計の強み

1. **重ねる段の選び方が既存の式をそのまま効かせている**: ずらしを換算の前（`scale_then_merge` の前）、向きを合流の後に入れることで、倍率の換算・右側の符号の反転・`windowposition` の調整量・キーワードの再導出・DPI の追随が新しい式なしで台本の値にも効く。台本の値の無いスコープは素通しで、今の置き場所を 1 ビットも変えない（要件 4.4・6.7）。
2. **「台本の値の分」を使う時点で 2 回解いて出す決め**: 分を持ち回らないので、倍率やバルーンの大きさが変わっても古くならず、記憶の書き手（`balloon_drag.rs` の 1 行）と記憶から戻す所（`apply_shell_descript`）を直すだけで `persist.rs` と約束の支えに触れずに済んでいる（要件 5.8・8.4・ウェーブ C28 の約束）。

## 重くはないが討議で触れてよい点

- `window_move.rs` の「差し引きの行を増やさない」: 関門はキャラ窓の矩形（台本の値なし用）とバルーンの矩形の両方を組み、`is_script_placed` を引き、`Clamp` の行に 2 つの欄を足すので、「数行を 1 回の呼び出しへ替える」だけでは増える見込み。キャラ窓を引く段から作業領域を選ぶ段まで（未解決の `warn!` を含む）を `balloon_limit.rs` の 1 関数へ移すと確実に減らせる。
- 分が解けないとき（`ScriptShare::Unresolved`）に記憶を書かない決めは、利用者のドラッグを次の起動へ持ち越さない。今の窓の組では起きにくいが、§8 か Error Handling の表に「書かない」理由を残すとよい。
- `none` の側は標準のキャラ窓の位置とその画面の作業領域で決まるので、キャラ窓が別の画面へ移った後に分を解くと、表示中の側と違う側で解く場面が理論上ある（`none` のときだけ・狭い）。テストに 1 本あると安心。

## 判定

**GO**（重い点 1・2 を設計の討議で決めることを条件に）

- **理由**: 構造の食い違いは無く、合流の 1 か所・起動の関門・記憶の書き手・約束のファイルの主張はコードで裏が取れた。重い点は檻の足りなさと本題の形の決め残しで、どちらも設計の枠（`limit_work_area`・預かり・当て口）の中で直せ、作り直しは要らない。
- **次の手**: 設計の討議（`kiro-design-discussion`）で重い点 1・2 と Open Questions 1〜6 を議題にし、決めを design.md に入れてから `/kiro-spec-tasks balloon-script-offset-align` へ進む。
