# 設計の検証: runtime-module-untangle

- 対象: `.kiro/specs/runtime-module-untangle/design.md`（要件 `requirements.md`・研究 `research.md`）
- 測った木: main `4863cb667` のワークツリー（`crates/areka/src/emo2_boot/` の今のコード）
- 進め方: `kiro-validate-design`（Analysis → Critical Issues → Strengths → GO/NO-GO）・対話なし

## Design Review Summary

設計は要件 1〜8 を全部コンポーネントに当て、本体の状態の読み書きを関数のポインタの束 `SessionHooks` に、終わる相を相ごと `session::ghost_quit` へ、起動を `BootRuntimeFn` の引数に、という 3 つの形で `runtime` の本番の外の参照を切っており、どれも同期で 1 フレームも遅らせない。コードと突き合わせた所（口の当たる場所・終わる相の登録・`log_history` の決まり・1,000 行）はほぼ設計どおりで、直すべきは 0 件の数え方の較正と、本番の口の倒れ先の扱いの 2 点だけである。

## コードとの突き合わせ（確かめたこと）

| 確かめたこと | 結果 |
|---|---|
| `runtime` に残る本番のファイルが本体を引く所（`crate::` と `super::` の両方） | `balloon_visibility_phase.rs`（`GhostSlot`・`BalloonWiring`・`ShellBoxHover`）・`frame.rs`（`quit_app`・`ghost_switch`）・`frame/switch.rs`・`frame/{attach,status_report,scale_notice,window_state,wiring}.rs`・`shell_carry.rs`・`change_cue.rs`・`switch_cue.rs`・`shell_balloon_resolve.rs`・`mod.rs`。設計の口・型の下ろし・終わる相の移動・起動の引数で全部に当てがある |
| `SessionHooks` の 10 本の口と、今の `GhostSlot`／`BootContext`／`SwitchInFlight`／`ReloadInFlight` の読み書き | `frame/switch.rs` の `session()`・`set_shell_dir`・`reload::skin_is_reload`＋`skin_applied`・`record_memory`・`BootContext` の書き換え・`ghost_ref_name`・`skin_phase_ended`・`SwitchInFlight`、ほか 6 ファイルの `kanade`／`seriko_sink`／`ghost_folder` と 1 対 1 に当たる。`finish_shell`・`finish_balloon` の段の並びと `warn!` の枝も設計の図どおり |
| 終わる相の登録 | 今は `mod.rs` 727 行目 `add_systems(Update, ghost_quit_system.before(emo2_frame_system))`。`frame.rs` で `ghost_switch`・`SwitchInFlight` を引くのは終わる相の中（234〜239 行目）だけで、相ごと移せば `frame.rs` に残る参照は無い。本番で毎フレームの相を登録する所は `ghost_session::register_systems` の 1 か所だけ（`main.rs` 257 行目） |
| `log_history` の決まりと 4.3 | `areka::install`（Update）・`areka::update`（Update）・`areka::readme`（Status）の決まりが在る。`info!` を持つのは `install_cue.rs` 65 行目だけで、`update_cue.rs`・`readme_cue.rs` は `warn!`・`debug!` だけ（`warn!` は出所によらず誤り・`debug!` は残らない）。`UNKEPT_TARGETS` を `areka::install::install_cue` の 1 行にする設計で足りる。`session` へ移すほかのファイルの出所（`areka::session::{reload, switch_copy, …}`・`areka::session::ghost_quit`）はどの決まりにも当たらず、今と同じく残らない |
| 1,000 行 | `main.rs` 989・`ghost_session.rs` 945・`mod.rs` 833・`ghost_switch.rs` 934・`balloon_visibility_phase.rs` 852・`frame/switch.rs` 703。設計の見込みで超えるファイルは無い |
| 設計が固定に使う既存の試験 | `boots_twice_in_one_process_without_double_registration`・`unrelated_info_is_not_kept`・`rules_table_is_the_designed_fourteen_rows_in_order`・`status_rows_and_areka_is_exact_only`・`frame_schedule_tests` の字面の照合は全部在る |

## Critical Issues

🔴 **Critical Issue 1**: 0 件の数え方（式 2・式 3）が宣言の深さを取り違えている
**Concern**: 設計は「本番のファイルは `#[path]` で深さが変わるものが無い」と書くが、今の木では `balloon_visibility.rs` が `balloon_visibility_phase.rs`・`balloon_visibility_decision.rs`・`balloon_visibility_wait.rs` を、`move_cue.rs` が `move_cue_args.rs`・`move_cue_geometry.rs`・`move_cue_place.rs` を `#[path]` で子として宣言しており、どれも直下に置かれた深さ 2 の本番のファイルである。今の木に式 2 を当てると、`balloon_visibility_phase.rs` 47・48 行目の `super::super::frame`・`super::super::target_map`（`runtime` の中を指す）が 2 行出る。逆に、これらのファイルの `super::super::super::`（`runtime` の外へ抜ける）は式 3（`frame/` だけを見る）に拾われない。
**Impact**: 完了のときの式 2 が 0 行にならず、設計の「残った行は全部 `#[cfg(test)]` の中」の説明も当てはまらないので、要件 3.3 の証跡が書けない。直した後にこのファイルへ外へ抜ける参照が入っても黙って通る。
**Suggestion**: 本番のファイルの集合を宣言の深さで 2 つに分ける（深さ 1＝`mod.rs` が宣言するもの、深さ 2＝`frame/*.rs` と上の 6 ファイル）。式 2 は深さ 1 の集合に、式 3 は深さ 2 の集合に当てる。着手の測りで `mod` と `#[path]` の宣言の一覧からこの 2 つの集合を作り、今の木に式を当てて「外へ抜ける行だけが出る」ことを 1 度確かめてから 0 件を書く。
**Traceability**: 3.1, 3.3, 3.4, 3.5
**Evidence**: design.md「Testing Strategy → 外の参照の数え方（BoundaryCount）」・research.md 9.10 節

🔴 **Critical Issue 2**: 本番の口の倒れ先 `FALLBACK` が試験で踏まれず、据え忘れが黙って縮退する
**Concern**: `#[cfg(test)]` の `FALLBACK` を `session::hooks::HOOKS` にするので、本番の `FALLBACK`（`None`・偽・空の文字・何もしない）は `#[cfg(not(test))]` の中だけにあり、どの試験でも動かない。さらに、口が据わっていないときに `SessionHooks::of` が何も記録しないので、`ghost_switch_in_flight`（偽）・`skin_phase_ended`（何もしない）・`ghost_folder`（`None` → `-`）は記録なしで今と違う答えに倒れる。
**Impact**: 決定論の試験網羅の必達と「ログ無し失敗経路の禁止」に当たる。今は据える所が 1 か所なので起きないが、`runtime-crate-split` で crate の境目ができた後に据え忘れると、切替の予約や読み直しの終わりが記録なしで飛ぶ。
**Suggestion**: 状態の無いときの答えを `cfg` なしの定数（例 `NO_BODY`）として置き、本番の `FALLBACK` はそれを指す。`SessionHooks::of` は口が無いと `error!` を 1 度出してから `NO_BODY` を返す。`NO_BODY` の 10 本の答えは、既存の試験（`boots_twice_in_one_process_without_double_registration` ほか）に assert を足して固定する（試験の関数は足さない・要件 8.1 の数は変えない）。
**Traceability**: 3.7, 3.8, 7.2
**Evidence**: design.md「Components and Interfaces → SessionHooks」の Service Interface と Implementation Notes の Risks

## Design Strengths

- **順と時刻を構造で守っている**: 終わる相を相ごと移し、`ghost_quit_system.before(frame_system)` の登録と呼ぶ順をそのまま残すので、終わる段取りの順と 1 フレームの遅れの無さが、知らせの線ではなく登録の形で決まる。口もどれも同じ関数の中の同期の呼び出しで、`finish_shell`・`finish_balloon` の段の並びと記録の文が `runtime` に残る。
- **測った事実で要件の当てはめを直している**: `shell_carry` の門だけを呼び手へ出す決め、`update_cue`・`readme_cue` は記録の分け方が変わらず `install_cue` だけを「残さない出所」に入れる決めは、どれもコードと一致しており、要件 2.1・4.3 の原則どおりに作業の量を最小にしている。

## Final Assessment

- **Decision**: GO
- **Rationale**: 依存の向き・振る舞いを変えない仕組み・要件の全項目への当てはめに根本の食い違いは無く、2 つの指摘はどちらも数え方と倒れ先の局所の直しで、設計の形（口の束・終わる相の移動・起動の引数）を変えずに直せる。
- **Next Steps**:
  1. 設計の議論（`/kiro-design-discussion runtime-module-untangle`）で Critical Issue 1・2 を議題にし、design.md の BoundaryCount 節と SessionHooks 節を直す。
  2. 直した後に `/kiro-spec-tasks runtime-module-untangle` でタスクを作る。着手のタスク（S0）に、宣言の深さで分けた本番のファイルの集合を作って式を較正する手順を入れる。
