//! drain フェーズ＋resnap（[`run_drain_phase`]・[`resnap_shell_targets`] ほか）。

use std::sync::mpsc::Receiver;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::{debug, info, trace, warn};

use areka_emo_compose::ScaleRatio;
use areka_emo_present::{EmoPresenter, PresentCommand, TargetId};
use wintf::ecs::{SizeI, WindowPos};

use crate::placement::balloon_lock::apply_script_balloon_lock;
use crate::placement::chain_finalize::{
    ChainDeferReason, ChainFinalizeStall, ChainFinalized, ChainSlot, ScopeChainState, ScriptPlaced,
    boot_pair_decision, mark_non_positive_widths, moved_default_pos, note_chain_deferral,
};
use crate::placement::chain_realign;
use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
use crate::placement::dpi_sync::{self, HoldSite};
use crate::placement::follow::{
    ClampScene, move_window_to, place_chain_in_work_area, resize_window_to,
};
use crate::placement::persist::persist_unremembered_char_positions;
use crate::placement::reset_balloon_pos::{BalloonResetTrigger, reset_balloon_positions};
use crate::placement::reset_pos::reset_window_positions;
use crate::placement::resolver::{PointPx, SizePx};
use crate::placement::script_alignment::apply_script_alignment;
use crate::placement::spawn::GhostWindows;
use crate::placement::sticky_group_ledger::{
    StickyGroupLedger, apply_reset_sticky, apply_set_sticky,
};

use super::{Emo2Wiring, PositionCue, apply_move_directive, shell_target};

// ---------------------------------------------------------------------------
// drain・text フェーズ＋排他 system（tasks.md task 4.2・design「UI 毎フレーム結線 / frame」の
// Responsibilities フェーズ②（drain）／③（text）・Service Interface・DD-1）
// ---------------------------------------------------------------------------

/// フェーズ②（drain・design「フェーズ②（drain）」・DD-1）: attach 完了後のみ受信済み
/// `PresentCommand` を FIFO で `presenter.apply` へ適用する（画面に載らない途中の `ShowSurface` は畳む）。
///
/// attach 前はチャネルが保留バッファを兼ねる（取りこぼしなし・FIFO）ため **`attached` が立つまで
/// drain しない**。装着後は届いている指令を非ブロックで**先に集めてから**到着順に
/// `presenter.apply(world, cmd)` する（集め方・畳み方は [`drain_present_commands`]）。`apply` は `()` を返し、失敗は
/// `cmd.reply`（本経路は常に `None`＝撃ちっぱなし）経由で、未装着等の異常は presenter 内部で
/// `error!` 済み（log-first）。本フェーズは panic しない（`SurfaceOutput`→UI の非ブロック配送契約）。
///
/// drain は attach 後にのみ走るため `TargetNotAttached` は原理上発生しない（発生＝結線バグとして
/// presenter が `error!`・design「適用時（UI）」）。
///
/// # 窓寸 reconcile は本フェーズの外側にある
///
/// 本フェーズは**指令の適用だけ**を担う。表示成立点の状態照合報告（`take_pending_resize`）を
/// 窓へ反映する `reconcile_reported_sizes` は、呼び出し順の所有者である
/// [`emo2_frame_system`](super::emo2_frame_system) が本フェーズの**後**に直接呼ぶ
/// （`areka-P0-balloon-visibility` design 決定 D5・task 4.3）。バルーン可視性の相を
/// 「全 `\b` 指令の適用後」かつ「表示が積む窓寸要求の消化前」へ挟むための並べ替えであり、
/// 適用と反映の順序（適用 → 反映）は移動前と変わらない。
pub fn run_drain_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    // attach 前はチャネルが保留バッファを兼ねる（取りこぼしなし・FIFO）。装着後のみ drain する（DD-1）。
    if !wiring.attached {
        return;
    }
    // wiring.rx（受信端＝shared 借用）と wiring.presenter（mut 借用）は互いに素なフィールドゆえ両立する。
    // apply は () を返し、失敗は cmd.reply（本経路は常に None）経由。未装着等の異常は presenter 内部で
    // error! 済み（log-first）。撃ちっぱなしの非ブロック配送契約ゆえ本フェーズは panic しない。
    drain_present_commands(&wiring.rx, |cmd| wiring.presenter.apply(world, cmd));
}

/// 1 フレームの drain が受信端から取り出す指令の上限（送り手が受け手より速いときもフレームを終わらせる）。
///
/// 普段は届くのが 1 フレームに数件（動く絵で 1 target あたり毎秒十数件）なので当たらない。
pub(super) const MAX_DRAIN_PER_FRAME: usize = 1024;

/// [`run_drain_phase`] の本体（適用を閉包で受ける形・GPU 無しの檻へ入れるためのシーム）。
///
/// 1. 届いている指令を**適用より先に**上限 [`MAX_DRAIN_PER_FRAME`] まで集める。`try_iter` を回しながら
///    適用すると、送り手（seriko）が送り続けるあいだ尽きず、適用 1 回が送りの間隔より重いと
///    フレームが終わらない（rerun2 の固まり・task 5.6）。上限を超えた分は捨てずにチャネルに残り、
///    次のフレームが到着順に取り出す（当たったら `warn!`）。
/// 2. 同じ target へ続く `reply` の無い `ShowSurface` は、後ろに同じ target の `ShowSurface` が
///    あれば画面に載らないので最後の 1 件だけ適用する（今のコマを出す）。`ShowSurface` 以外の指令は
///    区切りで、それをまたいで畳まない。`reply` 付きの指令は畳まない。
pub(super) fn drain_present_commands(
    rx: &Receiver<PresentCommand>,
    mut apply: impl FnMut(PresentCommand),
) {
    let cmds: Vec<PresentCommand> = rx.try_iter().take(MAX_DRAIN_PER_FRAME).collect();
    if cmds.len() == MAX_DRAIN_PER_FRAME {
        warn!(
            cap = MAX_DRAIN_PER_FRAME,
            "drain: 1 フレームに取り出す上限に達した（残りは捨てずに次のフレームで到着順に適用する）"
        );
    }
    // 後ろから見て、区切りまでのあいだに後の ShowSurface を持つ target を控える。
    let mut kept = Vec::with_capacity(cmds.len());
    let mut shown_later: Vec<TargetId> = Vec::new();
    let mut superseded = 0usize;
    for cmd in cmds.into_iter().rev() {
        if let PresentCommand::ShowSurface { target, reply, .. } = &cmd {
            if shown_later.contains(target) {
                if reply.is_none() {
                    superseded += 1;
                    continue;
                }
            } else {
                shown_later.push(*target);
            }
        } else {
            shown_later.clear();
        }
        kept.push(cmd);
    }
    if superseded > 0 {
        trace!(
            superseded,
            "drain: 同じ target へ続く ShowSurface を最後の 1 件に畳んだ（途中のコマは画面に載らない）"
        );
    }
    for cmd in kept.into_iter().rev() {
        apply(cmd);
    }
}

/// move drain フェーズ（`\![move]` の末端結線・design「frame 相で drain→`apply_move_directive`」・
/// R5.1/5.3/5.5/R6・task 9.2）: talk スレッド（`MoveCueSink`）から mpsc で届いた [`PositionCue`]
/// を非ブロックで FIFO 全件 drain し、UI スレッド上で種類ごとに適用する——`Move` は
/// [`apply_move_directive`]、`ResetWindowPos` は [`reset_window_positions`]（`OnResetWindowPos` は
/// 送らない・areka-P0-move-canon 要件 8.7）、吸着・組を作る・組を解く・バルーン位置初期化は
/// `placement` の各関数（areka-P0-window-link-directives）。
///
/// `PresentBridge`（[`run_drain_phase`]）と同型の跨ぎパターンだが、ゲートは `attached`（GPU）でなく
/// **`GhostWindows` の存在**である——move は GPU 表示層でなくキャラ窓 entity（`GhostWindows` が spawn 時
/// に生成）へ作用するため、GPU attach を待つ必要がない。`GhostWindows` 未挿入の間はチャネルが保留
/// バッファを兼ね（[`Receiver::try_iter`] を呼ばず取りこぼさない）、窓が生成された最初のフレームで
/// 一括適用する（OnFirstBoot の位置調整を早期に取りこぼさないための buffering・present drain の
/// 「attach 前は保留」と同じ意図）。
///
/// 各指令の適用は [`apply_move_directive`]／[`reset_window_positions`] が完結させる: 窓/`WindowPos`/
/// 作業領域/主モニタ/並べ直しの材料の不在はいずれも同関数内で `warn!`＋`false`（log-first・非 panic・R5.5）ゆえ、本フェーズは
/// 戻り値を捨てて次 directive へ進む（1 件の縮退が他 directive・talk を巻き込まない）。`try_iter` は
/// チャネルが空か全送信端 drop で尽きる（ブロックしない・empty/disconnected でも panic しない）。
pub fn run_move_drain_phase(wiring: &Emo2Wiring, world: &mut World) {
    // GhostWindows 未挿入の間はチャネルが保留バッファを兼ねる（try_iter を呼ばず取りこぼさない）。
    // 窓生成後の最初のフレームで一括適用する（OnFirstBoot 移動の早期取りこぼし防止・present drain と同意図）。
    if world.get_resource::<GhostWindows>().is_none() {
        return;
    }
    // try_iter: 現時点でキュー済みの PositionCue を非ブロックで FIFO 全件取り出す（空・全送信端 drop で尽きる）。
    // wiring.move_rx（shared 借用）と world（mut 借用・別オブジェクト）は互いに素ゆえ両立する。
    for cue in wiring.move_rx.try_iter() {
        let directive = match cue {
            PositionCue::Move(directive) => directive,
            // 並べ直しは拡大率の値を読まない（今の窓の大きさがすでに今の拡大率で描いた大きさ・
            // research §13 議題 1）。失敗は関数内で warn! 済みゆえ戻り値を捨てて次へ進む。
            PositionCue::ResetWindowPos => {
                reset_window_positions(world);
                continue;
            }
            // 吸着・組・バルーン位置初期化（areka-P0-window-link-directives）。記録は各関数の中ゆえ
            // 戻り値を捨てる。台本の並び（組を作ってから `\![move]`）はこの 1 本の線の順で保たれる。
            PositionCue::SetAlignment { scope, word } => {
                apply_script_alignment(world, scope, &word);
                continue;
            }
            PositionCue::SetSticky { tokens } => {
                apply_set_sticky(world, &tokens);
                continue;
            }
            PositionCue::ResetSticky => {
                apply_reset_sticky(world);
                continue;
            }
            PositionCue::ResetBalloonPos => {
                reset_balloon_positions(world, BalloonResetTrigger::Script);
                continue;
            }
            // 台本の lock・unlock（descript-window-keys・要件 5.1・5.2）。記録は関数の中。
            PositionCue::BalloonMoveLock { scope, locked } => {
                apply_script_balloon_lock(world, scope, locked);
                continue;
            }
        };
        // 台本のオフセットは**作者基準 px**ゆえ、対象スコープのシェルへ実適用中の k で物理 px へ
        // 換算する（`move_cue_geometry.rs` の `compute_move_target`・`windowposition.x/y` と同じ写像）。
        // 真実源は表示層の `applied_ratio`＝「いま画面に載っている絵に実際に掛かった k」であり、
        // 導出しただけで適用に失敗した k は漏れてこない。k はこの 1 か所からだけ取り、モニタの DPI から
        // 計算し直さない（将来の利用者の倍率もこの値に含めれば追従する・research §13 議題 1）。
        //
        // 表示未成立（初回 ShowSurface 前）・未登録 target は `None`。このとき恒等へ縮退する——
        // まだ拡大が掛かっていない状態であり、従来（k 非適用）と同じ値になる安全側の既定である。
        let k = wiring
            .presenter
            .applied_ratio(shell_target(directive.scope))
            .unwrap_or_else(|| {
                // 件 5 の測り（MoveMeasure・要件 6.1）: 表示の前に指令が当たったかを記録で読む。
                info!(
                    event = "move_ratio_unset",
                    scope = directive.scope,
                    "表示の前なので \\![move] を拡大率 1 で当てる"
                );
                ScaleRatio::ONE
            });
        // 適用の全縮退（窓・作業領域・主モニタの不在）は apply_move_directive 内で warn!＋false 済み
        // （log-first・R5.5）。戻り値は捨てて次 directive へ進む（1 件の縮退で talk を殺さない・非 panic）。
        apply_move_directive(world, &directive, k);
    }
}

// ---------------------------------------------------------------------------
// resnap シーム（tasks.md task 3.2・design「統合シーム（emo2_boot frame.rs）>
// resnap_shell_targets / resnap_from_sizes」・Req1.3/3.1/3.2/4.1/4.3/4.5・DD-2/DD-5）
// ---------------------------------------------------------------------------

/// 合成寸法列を受け、shell サーフェス寸が変わった scope の char 窓のみ [`resize_window_to`] を
/// 駆動する純粋判定部（headless テスト対象・GPU 不要・design「resnap_from_sizes」・
/// Req1.3/3.1/3.4/4.5）。
///
/// [`GhostWindows`] Resource を world から取得（未挿入は no-op＝Preconditions）。各
/// `(scope, shown_size)` について:
/// - `char_window(scope)` が `None`（未知 scope）→ skip（再適用対象の char 窓が無い）。
/// - **非正寸**（`w <= 0 || h <= 0`）→ skip（Req3.4 の防御・[`resize_window_to`] と二重防波堤）。
/// - char 窓 `WindowPos.size` と `SizeI::new(w, h)` が**異なるときのみ** [`resize_window_to`] を
///   呼ぶ（同寸は no-op＝冗長駆動回避・Req3.1 べき等）。
///
/// **balloon 窓には一切触れない**（scope→`char_window` 写像のみ・Req4.5/DD-5）。判定・反映は
/// World 操作に閉じ GPU を要しない（GPU 結合は薄い [`resnap_shell_targets`] が担う）。
pub(super) fn resnap_from_sizes(world: &mut World, sizes: impl Iterator<Item = (usize, SizePx)>) {
    // GhostWindows（scope→窓 entity の正本）。未挿入は no-op（Preconditions・Req4.5）。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return;
    };
    // scope→char_window を先に解決して collect し、world の不変借用を後段の &mut ループへ跨がせない
    // （借用衝突回避）。未知 scope・非正寸はここで弾く（Req3.4・二重防波堤）。
    let mut targets: Vec<(Entity, SizePx)> = Vec::new();
    for (scope, shown_size) in sizes {
        let Some(char_window) = ghost_windows.char_window(scope) else {
            // 未知 scope（GhostWindows に無い）→ skip（char 窓が無ければ再適用対象なし）。
            continue;
        };
        if shown_size.w <= 0 || shown_size.h <= 0 {
            debug!(
                scope,
                ?shown_size,
                "resnap: 非正寸のため skip（Req3.4・二重防波堤）"
            );
            continue;
        }
        targets.push((char_window, shown_size));
    }
    // 反映: char 窓 WindowPos.size と異なるときのみ resize_window_to を駆動（同寸は非発火・Req3.1）。
    for (char_window, shown_size) in targets {
        let current = world.get::<WindowPos>(char_window).and_then(|wp| wp.size);
        if current == Some(SizeI::new(shown_size.w, shown_size.h)) {
            // 同寸＝冗長駆動を避ける（Req3.1 べき等・正常系ゆえ静穏に skip）。
            continue;
        }
        // 異寸のみ: 新寸で T 再適用→一度書き→随伴（resize_window_to が単一ライター・Req1.3）。
        // 経路タグは Resnap（毎フレーム再スナップ・Req 1.2／task 1.4）。
        resize_window_to(world, char_window, shown_size, PlacementRoute::Resnap);
    }
}

/// drain 後に shell サーフェス寸法の変化を検知し、変化した char 窓のみアンカー再適用を駆動する
/// 薄いアダプタ（GPU 結合の thin wiring・`presenter` を read-only 消費・design
/// 「resnap_shell_targets」・Req3.2/4.1/4.5）。
///
/// [`GhostWindows`] を取得し `scopes()` を回す。各 scope について
/// **`presenter.text_slot_view(shell_target(scope))`**（**`balloon_target` は読まない**＝shell
/// 限定駆動・Req4.5/DD-5）を引き、`None`（初回 `ShowSurface` 前＝未表示）は skip。
/// `surface_size() -> (u32, u32)`（emo-present 適用点の実寸・Req4.1）を `i32::try_from` で
/// [`SizePx`] 化し、**変換失敗・0** は skip（Req3.4。`try_from(0)=Ok(0)` ゆえ 0 を明示的に弾く）。
/// 得た `(scope, SizePx)` 列を [`resnap_from_sizes`] へ渡す——**presenter 借用を解いてから**
/// （先に `Vec` へ collect してから world を mut 借用・借用衝突回避）。
///
/// 未表示 target・未装着 presenter は全 scope skip（no-op・panic しない）。`GhostWindows` 未挿入
/// でも安全（`resnap_from_sizes` が no-op）。
pub(super) fn resnap_shell_targets(presenter: &EmoPresenter, world: &mut World) {
    resnap_with(presenter, world)
}

/// 表示中 target の**物理寸**（k 倍後）だけを引く最小シーム（[`EmoPresenter::target_physical_size`]
/// の抽象）。
///
/// `EmoPresenter` から `Some` を得るには実 GPU で `ShowSurface` を完了させた装着済み target が
/// 要る。ゆえに「resnap が **どの `TargetId` を読むか**」（shell か balloon か）は、素の
/// `EmoPresenter::new()` を渡す存在チェックでは**全 target が `None` に潰れて観測できない**——
/// `shell_target`→`balloon_target` の 1 トークン変異が檻をすり抜けていた実際の穴である
/// （2026-07-30 是正。それ以前は「コードレビューで足りる」と散文で断っていた）。
///
/// 本トレイトは兄弟の [`ScaleReportSource`] と同型の意図を持つ: **frame 側の結線**を GPU 無しの
/// 決定論檻へ入れるためのシーム（D9 の振り分け基準 (a)＝判断分岐は in-crate 純テスト）。
pub(super) trait PhysicalSizeSource {
    /// 表示中なら適用済み k を掛けた物理寸を返す。未装着・未表示は `None`。
    fn physical_size(&self, target: TargetId) -> Option<(u32, u32)>;
}

impl PhysicalSizeSource for EmoPresenter {
    fn physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
        self.target_physical_size(target)
    }
}

/// [`resnap_shell_targets`] の本体（[`PhysicalSizeSource`] 越しに寸を引く形へ一般化したもの）。
///
/// 本番経路は `resnap_shell_targets` が**本体を持たずここへ委譲する**だけである——実装を 2 つに
/// 割らないことが要点で、fake 相手の檻が「本番も同じ判断をしている」ことを担保する
/// （実装が分岐していると fake は緑のまま本番だけ壊れ得る）。
///
/// # 破棄済み窓の打ち切り（要件 6.2/6.3・design D8 消費側）
///
/// scope ループの**冒頭**で char 窓 entity の存在を確認し、既に despawn 済みなら
/// `debug!`（[`DESPAWNED_SKIP_TAG`]）で当該 scope を打ち切って**他 scope は処理し切る**
/// （終了処理の正常系ゆえ警告以上を出さない）。寸の問い合わせより手前に置くのは、
/// 破棄済み窓のために表示側へ問い合わせる意味が無いためである。
pub(super) fn resnap_with<S: PhysicalSizeSource + ?Sized>(source: &S, world: &mut World) {
    // scope 識別は GhostWindows 経由（Req4.5）。未挿入は shell 寸を引く対象が無い＝no-op。
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return;
    };
    // presenter 借用を解いてから resnap_from_sizes（&mut World）を呼ぶため、先に collect する。
    let mut sizes: Vec<(usize, SizePx)> = Vec::new();
    for scope in ghost_windows.scopes() {
        // 存在確認（要件 6.2/6.3・design D8 消費側）: レジストリが指す char 窓が既に
        // despawn 済みなら **正常終了系**として debug で打ち切り、**他 scope は処理し切る**。
        // 寸の問い合わせより手前に置く——破棄済みの窓のために表示側へ問い合わせる意味が
        // 無いうえ、素通りさせると下流 `resize_window_to` が破棄済み窓ぶん呼ばれる。
        if let Some(char_window) = ghost_windows.char_window(scope)
            && world.get_entity(char_window).is_err()
        {
            debug!(
                scope,
                entity = ?char_window,
                "{DESPAWNED_SKIP_TAG} resnap: char 窓 entity が破棄済み（despawn）→ 本 scope を正常系として打ち切り（他 scope は継続）"
            );
            continue;
        }
        // 整合ゲート（設計 C5・要件 5.8）: 待ち札のある窓へはこの経路からも書かない。
        // 見送った寸は持ち越し用に溜めない——本関数は毎フレーム**表示側の現物理寸を読み直す**
        // ので、待ちが解けたフレームに同じ食い違いがそのまま再び見える（拡大率の相が先に
        // 1 本書けば、そこで一致してべき等 skip で抜ける）。
        if let Some(char_window) = ghost_windows.char_window(scope)
            && dpi_sync::defers_window_write(world, char_window, HoldSite::Resnap)
        {
            continue;
        }
        // shell target（偶数=2*scope）のみを読む（balloon_target は読まない＝shell 限定・Req4.5）。
        // 窓 client に合わせるべき寸は **物理寸**（k 倍後）であって native 原寸ではない。両者を
        // 選べる `text_slot_view()`（`surface_size()`／`physical_size()` が隣り合う）ではなく、
        // **物理寸だけを返す** `target_physical_size` を引く——消費点に取り違えの選択肢を残さない
        // （native で駆動すると k≠1 で DPI 相 reconcile と同一フレーム内で綱引きになり窓が原寸へ
        // 引き戻される）。丸めは presenter 側が権威 `scaled_extent` で確定済みゆえ通貨変換のみ行う。
        // 未表示（初回 ShowSurface 前）・未装着は `None` → skip（no-op・遅延化への防御）。
        // shell/balloon の取り違えは `resnap_reads_shell_targets_only_and_ignores_balloon_geometry`
        // と `resnap_queries_shell_targets_only` が排他的に殺す（2026-07-30 実測）。
        let Some((w, h)) = source.physical_size(shell_target(scope as u32)) else {
            continue;
        };
        // (u32,u32)→i32 変換失敗は skip（Req3.4）。
        let (Ok(w), Ok(h)) = (i32::try_from(w), i32::try_from(h)) else {
            debug!(
                scope,
                w, h, "resnap: 物理寸の i32 変換に失敗 → skip（Req3.4）"
            );
            continue;
        };
        // 0 は skip（try_from(0)=Ok(0) ゆえ明示的に弾く・Req3.4）。負値は u32 起点ゆえ生じない。
        if w == 0 || h == 0 {
            debug!(
                scope,
                "resnap: 物理寸が 0 → skip（Req3.4・try_from(0)=Ok を明示的に弾く）"
            );
            continue;
        }
        sizes.push((scope, SizePx { w, h }));
    }
    // ここで presenter／ghost_windows 借用は終わり、world を mut 借用して判定・反映へ渡す。
    resnap_from_sizes(world, sizes.into_iter());
}

// ---------------------------------------------------------------------------
// 実表示寸での連鎖再解決（scg 要件 7・design C6）
// ---------------------------------------------------------------------------

/// 初期配置を**実表示寸**で一度きり確定させる（要件 7.1/7.4）。
///
/// 本番経路は [`resnap_shell_targets`] と同じく本体を持たず [`finalize_chain_once_with`] へ
/// 委譲する（実装を 2 つに割らない＝fake 相手の檻が本番の判断を担保する）。
pub(super) fn finalize_chain_once(presenter: &EmoPresenter, world: &mut World) {
    finalize_chain_once_with(presenter, world)
}

/// [`finalize_chain_once`] の本体（[`PhysicalSizeSource`] 越しに寸を引く形へ一般化したもの）。
///
/// # 駆動条件（すべて満たしたフレームで 1 度だけ走る）
///
/// - [`ChainFinalized`] 未挿入（＝まだ確定していない・7.4）。
/// - [`GhostWindows`] が在り、スコープが 1 つ以上ある。
/// - **全スコープ**について実表示寸が引け、かつ char 窓の `WindowPos` の寸が**それと一致**して
///   いる。一致しない間は [`resnap_from_sizes`] の再適用が未 landing＝位置が確定していないため
///   見送る（次フレームで再挑戦する）。この条件により「resize の反映を待たずに古い位置で連鎖を
///   解く」取り違えが構造的に起こらない。
///
/// いずれか 1 つでも欠ける scope があれば**確定させずに戻る**（部分適用しない）。窓破棄後の
/// フレームでも `WindowPos` 不在で見送るだけで、panic もログ汚染もしない。
///
/// # 見送りの可観測性（scg 6.5）
///
/// 見送りの理由は「ゴースト待ち」と「areka 自身の待ち」に仕分ける。仕分けは理由の全列挙子を
/// 書いた `match` で決める（理由が増えたら、足した側がコンパイルエラーでどちらかを選ぶ）。
///
/// - **ゴースト待ち**（窓が無い・スコープが無い・まだ一度も表示されていない）は数えない。
///   `trace!` を 1 行残して戻るだけで、[`defer_chain_finalize`] を呼ばない（窓を閉じた後に
///   数の記録が作り直されることも無い）。この待ちの長さはゴーストの台本しだい（最初の `\s`
///   をいつ出すか）で決まり、巡でも秒でも決まった数で区切る限り、どこかのゴーストの正常な
///   待ちで鳴ってしまうため。「まだ一度も表示されていない」には表示側がまだ装着されていない
///   巡も含まれる（走査からは見分けられない）。装着の失敗は装着の側が自分で記録する。
/// - **areka 自身の待ち**（表示された後の処理が着地しない）は今日どおり [`defer_chain_finalize`]
///   へ渡す。有界の待ち（[`CHAIN_FINALIZE_STALL_FRAMES`]）を超えても未確定なら**一度だけ**
///   理由つきの `warn!` を出し、正常な待ちと永久に条件が揃わない停滞をログから見分けられる
///   ようにする。
///
/// 走査はスコープの昇順で最初に躓いた理由 1 つで打ち切るので、仕分けもその 1 つで決まる。
/// 同じ巡に areka 自身の理由と未表示が重なっていれば、先に躓いたスコープの側で決まる。
///
/// [`CHAIN_FINALIZE_STALL_FRAMES`]: crate::placement::chain_finalize::CHAIN_FINALIZE_STALL_FRAMES
pub(super) fn finalize_chain_once_with<S: PhysicalSizeSource + ?Sized>(
    source: &S,
    world: &mut World,
) {
    // 一度きり（7.4）。確定後のサーフェス切替では駆動しない＝会話中に位置が動かない。
    if world.get_resource::<ChainFinalized>().is_some() {
        return;
    }
    let (states, targets) = match collect_chain_states(source, world) {
        Ok(scan) => scan,
        Err(reason) => {
            match reason {
                // ゴースト待ち: 数えない（長さはゴーストの台本しだい）。
                ChainDeferReason::NoGhostWindows
                | ChainDeferReason::NoScopes
                | ChainDeferReason::NotShownYet { .. } => {
                    trace!(
                        scope = ?reason.scope(),
                        reason = %reason,
                        "chain_finalize: ゴースト待ちのため見送りを数えない（窓が無い・まだ一度も表示されていない）"
                    );
                }
                // areka 自身の待ち: 今日どおり数える。
                ChainDeferReason::NoCharWindow { .. }
                | ChainDeferReason::UnusableShownSize { .. }
                | ChainDeferReason::NoWindowPos { .. }
                | ChainDeferReason::IncompleteWindowPos { .. }
                | ChainDeferReason::ResnapNotLanded { .. }
                | ChainDeferReason::DpiSyncHeld { .. } => defer_chain_finalize(world, reason),
            }
            return;
        }
    };

    // 組ごとの決め（dpi-realign-remembered-chain design BootRemembered）。記憶に位置の無い
    // スコープ（既定の位置がある）は今の並べ直しのまま: 既定のままなら左へ付け、既定から動かされたか
    // 台本の `\![move]` が置いた窓（x が既定のままでも・scg 7.3）は動かさない。記憶から戻した
    // スコープ（既定の位置が無い）は「台本の印 → 組の窓 → 覚えた印」の順で決める（6.3〜6.5）。
    // 基準は前の窓の解いた後の x（resolve_chain）。
    let ledger = world.get_resource::<StickyGroupLedger>();
    let ghost_windows = world.get_resource::<GhostWindows>();
    let mut slots: Vec<ChainSlot> = states
        .iter()
        .zip(&targets)
        .map(|(s, &(_, entity, _))| ChainSlot {
            scope: s.scope,
            current_x: s.current_x,
            width: s.width,
            decision: boot_pair_decision(
                s,
                world.get::<ScriptPlaced>(entity).is_some(),
                ledger.is_some_and(|l| l.group_of(s.scope as u32).is_some()),
                ghost_windows.and_then(|gw| gw.remembered_adjacency(s.scope)),
            ),
        })
        .collect();

    // ここで world の shared 借用は終わり、mut 借用して反映へ渡す。
    mark_non_positive_widths(&mut slots);
    // 解いた鎖を基準の窓のモニタの作業領域の中に置く（window-work-area-clamp 要件 1・2.2）。
    // 場面は記録の語だけ: 既定の位置が有れば起動の並べ直し、無ければ記憶からの隣り合わせ。
    let has_default = |scope: usize| {
        states
            .iter()
            .any(|s| s.scope == scope && s.default_x.is_some())
    };
    let moves: Vec<_> = place_chain_in_work_area(world, &slots, &targets, |scope| {
        if has_default(scope) {
            ClampScene::BootRepack
        } else {
            ClampScene::RememberedJoin
        }
    })
    .into_iter()
    .filter(|p| p.to != p.from)
    .collect();
    // 記憶から戻した組は、動かさない組も含めて組ごとに 1 行残す（7.1）。
    for (i, (slot, state)) in slots.iter().zip(&states).enumerate().skip(1) {
        if state.default_x.is_some() {
            continue;
        }
        let decision = slot.decision.as_str();
        let to_x = moves
            .iter()
            .find(|m| m.scope == slot.scope)
            .map_or(slot.current_x, |m| m.to.x);
        info!(
            prev_scope = slots[i - 1].scope,
            scope = slot.scope,
            decision,
            from_x = slot.current_x,
            to_x,
            dx = to_x.saturating_sub(slot.current_x),
            "chain_finalize: 記憶から戻した組の判断を下した（覚えた印で今の幅に隣り合わせる・要件 6.3・7.1）"
        );
    }
    for m in &moves {
        let Some(&(_, entity, pos)) = targets.iter().find(|(s, _, _)| *s == m.scope) else {
            continue;
        };
        // 記憶に位置の無い組は今の行のまま（記憶から戻した組は上の組の行が出ている）。
        if has_default(m.scope) {
            info!(
                scope = m.scope,
                from_x = pos.x,
                to_x = m.to.x,
                "chain_finalize: 実表示寸で連鎖を再解決（初期配置の確定・scg 7.1）"
            );
        }
        // 反映は move_window_to のみ（唯一の位置ライター・バルーン随伴 offset 維持を内包）。
        // Y は寄せが縦に収めたときだけ変わる（それ以外は今の Y＝下端吸着は再アンカーが保つ・7.2）。
        move_window_to(world, entity, m.to.x, m.to.y);
    }

    // 台帳の既定位置を確定値へ揃える（以後の「既定配置のまま」判定が確定後を基準に働く）。
    //
    // ここを明示的に書くのは、上の反映が `move_window_to`＝`PlacementRoute::MoveCue`
    // ＝**明示操作**の経路を通るからである。単一の窓書込口の追随規則（atom D9／D16）は
    // システム由来の再アンカーにしか効かないので、両者は重ならない（二重に書かない）。
    //
    // Y は今の窓の Y でなく**元の既定位置の Y** を載せる（寄せが縦に収めたときだけ寄せた後の Y・
    // window-work-area-clamp 要件 7.5）。台本の `\![move]` が置いた窓は印
    // （`ScriptPlaced`）で並べ直しから外れるので、ここへ来るのは台本の置いていない窓だけで、普段は
    // 今の Y と既定の Y が等しく結果は変わらない。今の Y を載せる形にすると、既定と違う Y が既定へ
    // 入り得て、並べ終えた時点の保存の「動かされていない」の判定を化かす
    // （areka-P0-char-position-save-on-exit 要件 1.7）。
    if !moves.is_empty()
        && let Some(mut ghost_windows) = world.get_resource_mut::<GhostWindows>()
    {
        for m in &moves {
            // 既定の位置が無いのは記憶から戻した窓を覚えた印で隣り合わせた組。既定の位置を
            // 付けないので、並べ終えた時点の保存はこの窓の位置を書かない（6.6）。
            let Some(old_default) = ghost_windows.default_char_pos(m.scope) else {
                debug!(
                    scope = m.scope,
                    "chain_finalize: 記憶から戻した窓を隣り合わせたので既定の位置を付けない（記憶に書かない・6.6）"
                );
                continue;
            };
            let mut pos = moved_default_pos(old_default, m.to.x);
            if m.to.y != m.from.y {
                pos.y = m.to.y;
            }
            ghost_windows.set_default_char_pos(m.scope, pos);
        }
    }

    world.insert_resource(ChainFinalized);
    // 記憶に位置が無い窓の位置をここで書く（areka-P0-char-position-save-on-exit 要件 1.1）。
    // 印の内側に置くので同じ窓の一式で 1 度だけ通り、並べ終える前に終わった回は書かない
    // （1.6）。動かすスコープが 0 件の回でも呼ぶ。本番の呼び手はここ 1 か所だけ（3.1・3.2）。
    persist_unremembered_char_positions(world);
    debug!(
        scopes = states.len(),
        moved = moves.len(),
        "chain_finalize: 初期配置を確定（以後のサーフェス切替では駆動しない・scg 7.4）"
    );
}

// ---------------------------------------------------------------------------
// DPI 遷移後の連鎖再解決（atom 設計 C4・要件 6.1／6.2／6.3／6.6）
// ---------------------------------------------------------------------------

/// 遷移後の連鎖の解き直しを 1 回試みる（`emo2_frame_system` が連鎖確定の**直後**に呼ぶ）。
///
/// 本番経路は [`finalize_chain_once`] と同じく本体を持たず
/// [`realign_chain_once_with_source`] へ委譲する（実装を 2 つに割らない）。
pub(super) fn realign_chain_once(presenter: &EmoPresenter, world: &mut World) {
    realign_chain_once_with_source(presenter, world)
}

/// [`realign_chain_once`] の本体（[`PhysicalSizeSource`] 越しに寸を引く形へ一般化したもの）。
///
/// 判断そのもの（武装しているか・待ち札が残っていないか・解き直して武装を解く）は
/// [`crate::placement::chain_realign`] が持つ。ここが供給するのは走査だけである——
/// 「全スコープの窓寸が実表示寸へ揃ったか」の判定を起動時確定と**同一の実装**
/// （[`collect_chain_states`]）で行うために、走査を関数として渡す。
///
/// 走査を配置層へ移さないのは、表示側の寸の引き口（[`PhysicalSizeSource`]）と target 採番
/// （`shell_target`）が表示層の語彙だからである（配置層はどちらも知らない）。
pub(super) fn realign_chain_once_with_source<S: PhysicalSizeSource + ?Sized>(
    source: &S,
    world: &mut World,
) {
    chain_realign::realign_chain_once_with(world, |world| collect_chain_states(source, world));
}

/// 確定の走査結果——判定へ渡す状態列と、反映へ渡す `(scope, char 窓 entity, 現在位置)` の列。
///
/// 現在位置を持ち回るのは Y を据え置くため（7.2）。
type ChainScan = (Vec<ScopeChainState>, Vec<(usize, Entity, PointPx)>);

/// 確定に要る入力を集める。駆動条件を 1 つでも欠くスコープがあればその場で打ち切り、
/// **見送った理由**を返す（部分適用しない）。
fn collect_chain_states<S: PhysicalSizeSource + ?Sized>(
    source: &S,
    world: &World,
) -> Result<ChainScan, ChainDeferReason> {
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return Err(ChainDeferReason::NoGhostWindows);
    };

    let mut states: Vec<ScopeChainState> = Vec::new();
    let mut targets: Vec<(usize, Entity, PointPx)> = Vec::new();

    for scope in ghost_windows.scopes() {
        let Some(char_window) = ghost_windows.char_window(scope) else {
            return Err(ChainDeferReason::NoCharWindow { scope });
        };
        // `None` は「既定配置ではない」（記憶から戻したスコープ）。ここでは打ち切らずそのまま渡す。
        // 起動の並べ直しは `None` のスコープを覚えた印で決め（`left`／`right` は今の幅で隣り合わせ、
        // `none`・印なし・組の窓・台本の印は動かさない）、拡大率の変化の後の詰め直しはこの値を
        // 使わず変わる直前の隣り合いで決める（dpi-realign-remembered-chain 要件 1・6.3〜6.5）。
        //
        // 読む値は「起動時の既定」ではなく**「最後にシステムが置いた既定位置」**である
        // （atom D9／D16）。システム由来の再アンカーでは単一の窓書込口が同じ量を書込先へ運ぶ。
        let default_x = ghost_windows.default_char_pos(scope).map(|p| p.x);
        // 表示未成立（初回 ShowSurface 前）は実表示寸が未確定＝まだ確定できない。
        let Some((w, h)) = source.physical_size(shell_target(scope as u32)) else {
            return Err(ChainDeferReason::NotShownYet { scope });
        };
        let (Ok(iw), Ok(ih)) = (i32::try_from(w), i32::try_from(h)) else {
            return Err(ChainDeferReason::UnusableShownSize { scope, w, h });
        };
        if iw <= 0 || ih <= 0 {
            return Err(ChainDeferReason::UnusableShownSize { scope, w, h });
        }
        let Some(wp) = world.get::<WindowPos>(char_window) else {
            return Err(ChainDeferReason::NoWindowPos { scope });
        };
        let (Some(pos), Some(size)) = (wp.position, wp.size) else {
            return Err(ChainDeferReason::IncompleteWindowPos { scope });
        };
        // 再アンカーが未 landing＝位置が確定していない。次フレームへ送る。
        if size.width != iw || size.height != ih {
            return Err(ChainDeferReason::ResnapNotLanded {
                scope,
                shown: (iw, ih),
                window: (size.width, size.height),
            });
        }
        states.push(ScopeChainState {
            scope,
            current_x: pos.x,
            width: iw,
            default_x,
        });
        targets.push((scope, char_window, PointPx { x: pos.x, y: pos.y }));
    }

    if states.is_empty() {
        // 窓がまだ生えていない。確定させずに次フレームへ。
        return Err(ChainDeferReason::NoScopes);
    }
    Ok((states, targets))
}

/// 見送りを 1 フレームぶん記録し、有界の待ちを超えていれば**一度だけ**診断を出す（scg 6.5）。
///
/// 呼び手（[`finalize_chain_once_with`]）は **areka 自身の理由**だけを渡す。ゴースト待ち
/// （窓が無い・スコープが無い・まだ一度も表示されていない）はここへ来ないので、数は「areka 自身の理由で
/// 見送った巡の数」になる。
///
/// 正常な起動ではログを 1 行も増やさない（確定は閾値の遥か手前で起きる）。報告後は数えも報せも
/// しないため、停滞し続けても出力は 1 行のまま。
fn defer_chain_finalize(world: &mut World, reason: ChainDeferReason) {
    world.init_resource::<ChainFinalizeStall>();
    let mut stall = world.resource_mut::<ChainFinalizeStall>();
    if !note_chain_deferral(&mut stall) {
        return;
    }
    let deferrals = stall.deferrals;
    warn!(
        deferrals,
        // 構造化フィールドの `scope` は grep 用（本文の Display は人が読む側）。
        scope = ?reason.scope(),
        reason = %reason,
        "chain_finalize: 初期配置の確定が続けて見送られている（隣接が崩れたままの可能性・scg 7.1/6.5）"
    );
}

#[cfg(test)]
#[path = "drain_resnap_tests.rs"]
mod tests;
