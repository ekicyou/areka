//! drain フェーズ＋resnap（[`run_drain_phase`]・[`resnap_shell_targets`] ほか）。

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::{debug, info, warn};

use areka_emo_compose::ScaleRatio;
use areka_emo_present::{EmoPresenter, TargetId};
use wintf::ecs::{SizeI, WindowPos};

use crate::placement::chain_finalize::{
    ChainDeferReason, ChainFinalizeStall, ChainFinalized, ScopeChainState, finalize_chain,
    moved_default_pos, note_chain_deferral,
};
use crate::placement::diag::{DESPAWNED_SKIP_TAG, PlacementRoute};
use crate::placement::follow::{move_window_to, resize_window_to};
use crate::placement::resolver::{PointPx, SizePx};
use crate::placement::spawn::GhostWindows;

use super::{Emo2Wiring, apply_move_directive, shell_target};

// ---------------------------------------------------------------------------
// drain・text フェーズ＋排他 system（tasks.md task 4.2・design「UI 毎フレーム結線 / frame」の
// Responsibilities フェーズ②（drain）／③（text）・Service Interface・DD-1）
// ---------------------------------------------------------------------------

/// フェーズ②（drain・design「フェーズ②（drain）」・DD-1）: attach 完了後のみ受信済み
/// `PresentCommand` を FIFO で全件 `presenter.apply` へ適用する。
///
/// attach 前はチャネルが保留バッファを兼ねる（取りこぼしなし・FIFO）ため **`attached` が立つまで
/// drain しない**。装着後は [`Receiver::try_iter`] で**現時点でキュー済みの指令を非ブロックで
/// FIFO 全件**取り出し、到着順に `presenter.apply(world, cmd)` する。`apply` は `()` を返し、失敗は
/// `cmd.reply`（本経路は常に `None`＝撃ちっぱなし）経由で、未装着等の異常は presenter 内部で
/// `error!` 済み（log-first）。本フェーズは panic しない（`SurfaceOutput`→UI の非ブロック配送契約）。
///
/// drain は attach 後にのみ走るため `TargetNotAttached` は原理上発生しない（発生＝結線バグとして
/// presenter が `error!`・design「適用時（UI）」）。`try_iter` はチャネルが空になるか送信端が全て
/// drop されると尽きる（ブロックしない）。
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
    // try_iter: 現時点でキュー済みの指令を非ブロックで FIFO 全件取り出す（空・全送信端 drop で尽きる）。
    // wiring.rx（受信端＝shared 借用）と wiring.presenter（mut 借用）は互いに素なフィールドゆえ両立する。
    for cmd in wiring.rx.try_iter() {
        // apply は () を返し、失敗は cmd.reply（本経路は常に None）経由。未装着等の異常は presenter 内部で
        // error! 済み（log-first）。撃ちっぱなしの非ブロック配送契約ゆえ本フェーズは panic しない。
        wiring.presenter.apply(world, cmd);
    }
}

/// move drain フェーズ（`\![move]` の末端結線・design「frame 相で drain→`apply_move_directive`」・
/// R5.1/5.3/5.5/R6・task 9.2）: talk スレッド（`MoveCueSink`）から mpsc で届いた [`MoveDirective`]
/// を非ブロックで FIFO 全件 drain し、UI スレッド上で [`apply_move_directive`] へ適用する。
///
/// `PresentBridge`（[`run_drain_phase`]）と同型の跨ぎパターンだが、ゲートは `attached`（GPU）でなく
/// **`GhostWindows` の存在**である——move は GPU 表示層でなくキャラ窓 entity（`GhostWindows` が spawn 時
/// に生成）へ作用するため、GPU attach を待つ必要がない。`GhostWindows` 未挿入の間はチャネルが保留
/// バッファを兼ね（[`Receiver::try_iter`] を呼ばず取りこぼさない）、窓が生成された最初のフレームで
/// 一括適用する（OnFirstBoot の位置調整を早期に取りこぼさないための buffering・present drain の
/// 「attach 前は保留」と同じ意図）。
///
/// 各 directive の適用は [`apply_move_directive`] が完結させる: 非スコープ基準・窓/`WindowPos` 不在・
/// 座標算出不能はいずれも同関数内で `warn!`＋`false`（log-first・非 panic・R5.5）ゆえ、本フェーズは
/// 戻り値を捨てて次 directive へ進む（1 件の縮退が他 directive・talk を巻き込まない）。`try_iter` は
/// チャネルが空か全送信端 drop で尽きる（ブロックしない・empty/disconnected でも panic しない）。
pub fn run_move_drain_phase(wiring: &Emo2Wiring, world: &mut World) {
    // GhostWindows 未挿入の間はチャネルが保留バッファを兼ねる（try_iter を呼ばず取りこぼさない）。
    // 窓生成後の最初のフレームで一括適用する（OnFirstBoot 移動の早期取りこぼし防止・present drain と同意図）。
    if world.get_resource::<GhostWindows>().is_none() {
        return;
    }
    // try_iter: 現時点でキュー済みの MoveDirective を非ブロックで FIFO 全件取り出す（空・全送信端 drop で尽きる）。
    // wiring.move_rx（shared 借用）と world（mut 借用・別オブジェクト）は互いに素ゆえ両立する。
    for directive in wiring.move_rx.try_iter() {
        // 台本のオフセットは**作者基準 px**ゆえ、対象スコープのシェルへ実適用中の k で物理 px へ
        // 換算する（`resolve_move_target_position` の doc・`windowposition.x/y` と同じ写像）。
        // 真実源は表示層の `applied_ratio`＝「いま画面に載っている絵に実際に掛かった k」であり、
        // 導出しただけで適用に失敗した k は漏れてこない。
        //
        // 表示未成立（初回 ShowSurface 前）・未登録 target は `None`。このとき恒等へ縮退する——
        // まだ拡大が掛かっていない状態であり、従来（k 非適用）と同じ値になる安全側の既定である。
        let k = wiring
            .presenter
            .applied_ratio(shell_target(directive.scope))
            .unwrap_or(ScaleRatio::ONE);
        // 適用の全縮退（非スコープ基準・窓不在・算出不能）は apply_move_directive 内で warn!＋false 済み
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
            debug!(scope, ?shown_size, "resnap: 非正寸のため skip（Req3.4・二重防波堤）");
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
/// 毎フレームの見送りは無音のままだが、有界の待ち（[`CHAIN_FINALIZE_STALL_FRAMES`]）を超えても
/// 未確定なら [`defer_chain_finalize`] が**一度だけ**理由つきの `warn!` を出す。正常な待ちと
/// 永久に条件が揃わない停滞を、事後にログから判別できるようにするため。
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
            defer_chain_finalize(world, reason);
            return;
        }
    };

    // ここで world の shared 借用は終わり、mut 借用して反映へ渡す。
    let moves = finalize_chain(&states);
    for m in &moves {
        let Some(&(_, entity, pos)) = targets.iter().find(|(s, _, _)| *s == m.scope) else {
            continue;
        };
        info!(
            scope = m.scope,
            from_x = pos.x,
            to_x = m.new_x,
            "chain_finalize: 実表示寸で連鎖を再解決（初期配置の確定・scg 7.1）"
        );
        // 反映は move_window_to のみ（唯一の位置ライター・バルーン随伴 offset 維持を内包）。
        // Y は現在値を据え置く（下端吸着は各窓の再アンカーが既に保っている・7.2）。
        move_window_to(world, entity, m.new_x, pos.y);
    }

    // 台帳の既定位置を確定値へ揃える（以後の「既定配置のまま」判定が確定後を基準に働く）。
    if !moves.is_empty()
        && let Some(mut ghost_windows) = world.get_resource_mut::<GhostWindows>()
    {
        for m in &moves {
            if let Some(&(_, _, pos)) = targets.iter().find(|(s, _, _)| *s == m.scope) {
                ghost_windows.set_default_char_pos(m.scope, moved_default_pos(pos, m.new_x));
            }
        }
    }

    world.insert_resource(ChainFinalized);
    debug!(
        scopes = states.len(),
        moved = moves.len(),
        "chain_finalize: 初期配置を確定（以後のサーフェス切替では駆動しない・scg 7.4）"
    );
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
        // `None` は「既定配置ではない」（保存位置の復元）。判定側が常に対象外として扱う
        // ため、ここでは打ち切らずそのまま渡す（scg 7.3）。
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
