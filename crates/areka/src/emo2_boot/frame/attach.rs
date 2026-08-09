//! attach 計画＋attach 相（[`AttachPlan`]・[`plan_attachments`]・[`run_attach_phase`] ほか）。

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::world::World;
use tracing::{debug, error, info, warn};

use areka_emo_present::{PresentCommand, TargetId, TextSlotView};
use areka_emo_text::actor::TextLayerRuntime;
use areka_parsers::balloon::BalloonModel;
use areka_sakura::ActorKey;
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

use crate::placement::spawn::GhostWindows;

use super::{
    AuthorDpis, BalloonScopeAssets, BootAssets, Emo2Wiring, ScopeAssets, balloon_target,
    shell_target,
};

/// 窓×資産の scope 突き合わせ結果（DD-12・純関数 [`plan_attachments`] の戻り値）。
///
/// `GhostWindows::scopes()`（`usize`・昇順・**正**）を [`BootAssets`] の資産 scope と照合した
/// 装着計画。三分類は排他:
/// - `items`: 窓と資産の双方が揃った scope の装着項目（`window_scopes` の出現順を保つ）。
/// - `missing_assets`: 窓はあるが対応資産が無い scope（呼び手が `warn!`＋skip＝表示なし縮退）。
/// - `unused_assets`: 資産はあるが窓が無い scope（呼び手が `debug!`＋破棄）。
///
/// 純粋・決定論（GPU 不要）。呼び手（attach フェーズ）は `items.len()` が期待窓数と一致することを
/// 積極 assert し、warn+skip 縮退が scope 導出バグを隠さないことを檻に入れる（DD-12・spine S1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachPlan {
    /// 窓と資産の双方が揃った scope の装着項目（`window_scopes` 出現順）。
    pub items: Vec<PlannedAttach>,
    /// 窓はあるが対応資産が無い scope（`usize` のまま・呼び手が `warn!`＋skip 縮退）。
    pub missing_assets: Vec<usize>,
    /// 資産はあるが窓が無い scope（`u32`・呼び手が `debug!`＋破棄）。
    pub unused_assets: Vec<u32>,
}

/// 1 scope 分の装着計画項目（attach フェーズ＝task 4.1 が消費）。
///
/// scope の shell／balloon 表示対象（`target_map` の正本・DD-3 の `2*scope`／`2*scope+1` 採番）と
/// 初期表示 surface id（DD-9・task 2.6 が [`super::assets::ScopeAssets`] へ焼き込み済み）を運ぶ。
/// `static_binds` は [`BootAssets`] 単一共有ゆえ本項目には複製しない（attach フェーズが
/// `assets.static_binds` を直接読む）。
///
/// `shell_index`／`balloon_index` は attach フェーズが非 Clone な `EmoWorld` を
/// `assets.shells`／`assets.balloons` から添字で move 消費するための添字（DD-12 の突き合わせで
/// 自然に確定する）。当該 scope に balloon 資産が無い場合 `balloon_index` は `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAttach {
    /// 対応 scope 番号（`usize`→`u32` 変換済み）。
    pub scope: u32,
    /// シェル表示対象（`shell_target(scope)`＝偶数・DD-3）。
    pub shell_target: TargetId,
    /// バルーン表示対象（`balloon_target(scope)`＝奇数・DD-3）。
    pub balloon_target: TargetId,
    /// 初期表示 surface id（`ScopeAssets.initial_surface_id`・DD-9）。
    ///
    /// **注記（defect #5・2026-07-13 実機#5）**: attach フェーズはこの値でシェル初回表示を駆動しなく
    /// なった（シェルは最初の `\s` cue まで非表示）。planner の突き合わせ・DD-9 の記録として carry する。
    pub initial_surface_id: u32,
    /// `assets.shells` 内の対応添字（attach フェーズの add 消費用）。
    pub shell_index: usize,
    /// `assets.balloons` 内の対応添字（同 scope・無ければ `None`）。
    pub balloon_index: Option<usize>,
}

/// 窓一覧と構築資産を突き合わせ装着計画を返す純関数（DD-12・GPU 不要の決定論単体テスト対象）。
///
/// **`GhostWindows::scopes()`（`window_scopes`・`usize`・正）が唯一の正**（DD-12）。窓一覧を
/// 走査し、各窓 scope を `u32` へ変換（`u32::try_from`）してから `assets.shells` を `scope`
/// フィールドで照合する:
/// - 一致 → `items` へ [`PlannedAttach`] を積む（`window_scopes` の出現順を保つ）。
/// - 不一致、または `u32` に収まらない `usize`（`u32::MAX` 超過＝如何なる資産 scope とも一致し得ない）
///   → 当該窓 scope（`usize`）を `missing_assets` へ。
///
/// 加えて、`assets.shells` の各 scope のうち `window_scopes` に現れないもの（資産あり窓なし）を
/// `unused_assets`（`u32`）へ集める（`u32`→`usize` は無損失ゆえ `as` で比較）。
///
/// # 純粋性
/// 状態・I/O・GPU なし。戻り値は入力順に決定論的（`window_scopes`／`assets.shells` の順序を保つ）。
/// `window_scopes` は `GhostWindows` 契約により一意昇順が期待されるが、本関数は分類の正しさを
/// その前提に依存しない（重複窓は重複項目として忠実に写す）。
///
/// # scope 整合（DD-12）
/// 窓あり資産なしは `warn!`＋skip 縮退、資産あり窓なしは `debug!`＋破棄を呼び手（attach フェーズ）が
/// 行う。呼び手は `items.len()` が期待窓数と一致することを積極 assert し、縮退が scope 導出バグを
/// 隠さないことを担保する（spine S1）。
pub fn plan_attachments(window_scopes: &[usize], assets: &BootAssets) -> AttachPlan {
    let mut items = Vec::new();
    let mut missing_assets = Vec::new();

    // GhostWindows::scopes()（正）を走査し、usize→u32 の吸収をここで一度だけ行う（DD-12）。
    for &window_scope in window_scopes {
        // u32 に収まらない usize は如何なる資産 scope（u32）とも一致し得ない → missing 分類。
        let Ok(scope) = u32::try_from(window_scope) else {
            missing_assets.push(window_scope);
            continue;
        };
        // 資産存在の正は shells の scope フィールド。一致した添字を attach フェーズへ運ぶ。
        match assets.shells.iter().position(|s| s.scope == scope) {
            Some(shell_index) => {
                // balloon 資産は同 scope で引く（build_boot_assets は shell と同 scope 集合で組むが、
                // 万一不揃いなら None として運び、attach フェーズが文字層接続を縮退できるようにする）。
                let balloon_index = assets.balloons.iter().position(|b| b.scope == scope);
                items.push(PlannedAttach {
                    scope,
                    shell_target: shell_target(scope),
                    balloon_target: balloon_target(scope),
                    initial_surface_id: assets.shells[shell_index].initial_surface_id,
                    shell_index,
                    balloon_index,
                });
            }
            // 窓はあるが対応資産が無い → warn!＋skip 縮退の対象（呼び手が観測）。
            None => missing_assets.push(window_scope),
        }
    }

    // 資産あり窓なし: shells の scope で window_scopes に不在のものを集める。
    // u32→usize は無損失（対象プラットフォームの usize は 32bit 以上）ゆえ as で比較する。
    let mut unused_assets = Vec::new();
    for shell in &assets.shells {
        if !window_scopes.contains(&(shell.scope as usize)) {
            unused_assets.push(shell.scope);
        }
    }

    AttachPlan {
        items,
        missing_assets,
        unused_assets,
    }
}

/// attach フェーズ（高々 1 回・design「フェーズ①（attach）」）をテスト駆動口として実装する。
///
/// ゲート（`GhostWindows` Resource ＋ `GraphicsCore` ＋ `WucGraphicsResource::is_valid()`）成立
/// フレームで純関数 [`plan_attachments`]（DD-12）を確定し、計画項目ごとに shell／balloon target を
/// 装着する。**バルーンのみ**初回表示（面0）を駆動して文字層スロットを取得し、**シェルは初回表示を
/// 発行せず**最初のさくらスクリプト `\s` cue まで非表示を保つ（defect #5・2026-07-13 実機#5）。
/// 資産は `Option::take` で高々 1 回消費し、ゲート不成立では消費せず
/// 次フレーム再試行へ委ねる（表示なし縮退・hang しない）。窓あり資産なしは `warn!`＋skip、資産あり
/// 窓なしは `debug!`＋破棄で log-first に観測し、計画件数と実装着件数を `info!` に列挙する（spine が
/// 件数一致を積極 assert・DD-12）。個別の attach 失敗・窓欠落は `error!`／`warn!`＋継続であり
/// panic しない（log-first・R7.3）——1 scope の失敗は他 scope を巻き込まない。
///
/// donor（`examples/emo-present.rs::boot_present_system`）の attach 駆動を、複数 scope × `GhostWindows`
/// 由来の窓解決へ一般化したもの。`apply` は同期実行のため balloon の `text_slot_view` は同一フレームで
/// `Some` になるのが正常経路（DD-4）。万一 `None`（上流の遅延化）なら接続せず次フレーム再試行に委ねる
/// （R4.2・[`connect_balloon_text`]）。
pub fn run_attach_phase(wiring: &mut Emo2Wiring, world: &mut World) {
    // 高々 1 回: 装着済みなら以降 no-op（装着後の remove/insert churn を避ける donor 慣行）。
    if wiring.attached {
        return;
    }

    // ゲート: GhostWindows Resource ＋ GPU 資源（GraphicsCore ＋ WucGraphicsResource::is_valid）。
    // いずれか欠ける間は資産を消費せず attached も立てず、次フレーム再試行へ委ねる（hang しない）。
    let gate_ready = world.get_resource::<GhostWindows>().is_some()
        && world.get_resource::<GraphicsCore>().is_some()
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !gate_ready {
        return;
    }

    // GhostWindows は Clone（小さな Entity 写像）。attach_target/apply が `&mut World` を要するため、
    // 窓写像を先に clone して world の不変借用をループへ跨がせない（借用衝突回避）。
    let ghost_windows = world
        .get_resource::<GhostWindows>()
        .expect("ゲートで存在確認済み")
        .clone();
    let window_scopes: Vec<usize> = ghost_windows.scopes().collect();

    // 資産は高々 1 回消費（ゲート成立後にのみ take）。既に None（二重 attach の異常）なら log-first で
    // 観測して打ち切る（panic しない・attached を立てて以降の空回りを止める）。
    let Some(assets) = wiring.assets.take() else {
        warn!("emo2 attach: ゲート成立だが BootAssets が既に消費済み（想定外）→ 装着せず打ち切り");
        wiring.attached = true;
        return;
    };

    // DD-12: 窓一覧（正）× 資産の突き合わせ（純関数・GPU 不要）。
    let plan = plan_attachments(&window_scopes, &assets);

    // log-first の縮退観測: 窓あり資産なし＝warn!（表示なし縮退・skip）・資産あり窓なし＝debug!（破棄）。
    if !plan.missing_assets.is_empty() {
        warn!(
            missing_scopes = ?plan.missing_assets,
            "emo2 attach: 窓はあるが対応資産が無い scope（表示なし縮退・skip）"
        );
    }
    if !plan.unused_assets.is_empty() {
        debug!(
            unused_scopes = ?plan.unused_assets,
            "emo2 attach: 資産はあるが窓が無い scope（破棄）"
        );
    }

    // 非 Clone な `EmoWorld` を計画の添字で個別に move 消費するため Option 包みにする（take で 1 回）。
    let BootAssets {
        shells,
        balloons,
        // resolver は attach では未使用（seriko へは wire_emo2_boot=task 5.1 が手渡す）。
        resolver: _,
        // static_binds は attach では未使用（defect #5・2026-07-13 実機#5）: シェル初回表示を attach で
        // 焼き付けなくなったため。起動時オンの bindgroup default は seriko が保持し（spawn_seriko へ
        // 手渡し済み）、最初の `\s` cue が駆動する Show{shell,id,binds=static_binds} に載って表示層へ届く。
        static_binds: _,
        // bind_resolver は attach では未使用（seriko の actor へは task 7.2 が手渡す）。
        bind_resolver: _,
        // loop_tables は attach では未使用（SERIKO ループ表は spawn_seriko の actor 構築＝task 9.2 が
        // 手渡す）。attach 相はループを駆動しないため破棄する。
        loop_tables: _,
        // author_dpi（D1・Req1.1）: descript 宣言由来の原稿 DPI を attach 時の target 政策として
        // 供給する（emo-dpi-scaling task 4.2）。shell と balloon で別宣言ゆえ引き当てを取り違え
        // ないよう [`AuthorDpis`] へ束ねる（下の `attach_target` 呼び 2 箇所が `for_target` で引く）。
        shell_author_dpi,
        balloon_author_dpi,
    } = assets;
    let author_dpis = AuthorDpis {
        shell: shell_author_dpi,
        balloon: balloon_author_dpi,
    };
    let mut shells: Vec<_> = shells.into_iter().map(Some).collect();
    // 文字層へ渡すバルーン定義は各 scope 自身の [`BalloonScopeAssets::model`]（scope 別 2 層マージ
    // 済み・Req 2.1）。World／アトラスと**同一の資産 1 件から**取り出すため、ある scope のバルーンが
    // 別 scope の系列由来の定義で駆動される取り違えが構造的に起こり得ない（Req 4.1）。
    let mut balloons: Vec<_> = balloons.into_iter().map(Some).collect();

    let planned_count = plan.items.len();
    let mut attached_count = 0usize;

    for item in &plan.items {
        let scope = item.scope;

        // --- shell target: char_window → attach_target（EmoWorld を move）。初回 ShowSurface は
        //     発行しない（defect #5）: シェルは最初の `\s` cue まで非表示・target のみ生成する。 ---
        let Some(shell_window) = ghost_windows.char_window(scope as usize) else {
            error!(scope, "emo2 attach: char_window が無い（GhostWindows 不整合）→ この scope を skip");
            continue;
        };
        let Some(shell_assets) = shells.get_mut(item.shell_index).and_then(|s| s.take()) else {
            error!(
                scope,
                shell_index = item.shell_index,
                "emo2 attach: shell 資産の添字が空（二重消費？）→ skip"
            );
            continue;
        };
        let ScopeAssets {
            emo_world: shell_world,
            atlas: shell_atlas,
            ..
        } = shell_assets;

        if let Err(e) = wiring.presenter.attach_target(
            world,
            item.shell_target,
            shell_window,
            shell_world,
            shell_atlas,
            // author_dpi は attach 対象 target と同じ式で引く（`item.shell_target` を両方に書く）＝
            // shell/balloon の取り違えが 1 行の中で目視可能になる（両者 u16 で型は守ってくれない）。
            author_dpis.for_target(item, item.shell_target),
        ) {
            error!(scope, error = %e, "emo2 attach: シェル target の attach に失敗（log-first・継続）");
            continue;
        }
        // シェルは初回 ShowSurface を attach で発行しない（defect #5・2026-07-13 実機#5）。SSP 互換の
        // 既定は「シェル表示なし（surface -1）」であり、attach 時に surface0/surface10 を焼き付けると
        // ゴースト起動の一瞬に規定面がちらつく（実機#5 の欠陥）。初回シェル表示は、最初のさくら
        // スクリプト `\s[N]` cue が seriko→PresentBridge→drain 経路で運ぶ ShowSurface が駆動する
        // （起動時オンの bindgroup default は seriko 保持の static_binds が Show に載る）。上の
        // attach_target で target 自体は生成済みゆえ、後続の `\s`-driven ShowSurface はこの
        // shell_target へ適用できる（emo2 murasaki は `\s[1000]`／kero は `\s[通常]` を OnBoot で
        // 発行するため、talk 開始直後にシェルは表示される）。
        // シェル target の装着成功を計上（DD-12 の planned==attached 積極 assert 用・balloon と対で 1 scope）。
        attached_count += 1;

        // --- balloon target（同 scope の資産がある場合）: attach → 初回 ShowSurface（面0・default）
        //     → text_slot_view → register_actor_view ---
        let Some(balloon_index) = item.balloon_index else {
            warn!(scope, "emo2 attach: 同 scope の balloon 資産が無い（DD-12 balloon_index None）→ 文字層接続なし");
            continue;
        };
        let Some(balloon_window) = ghost_windows.balloon_window(scope as usize) else {
            warn!(scope, "emo2 attach: balloon_window が無い（GhostWindows 不整合）→ バルーン装着を skip");
            continue;
        };
        // 当該 scope の資産 1 件を take で消費する（World／アトラス／定義は同一資産から取り出す
        // ＝別 scope の系列由来の定義が混ざり得ない・Req 4.1）。
        let Some(BalloonScopeAssets {
            emo_world: balloon_world,
            atlas: balloon_atlas,
            model: balloon_model,
            ..
        }) = balloons.get_mut(balloon_index).and_then(|b| b.take())
        else {
            error!(
                scope,
                balloon_index,
                "emo2 attach: balloon 資産の添字が空（二重消費？）→ バルーン装着を skip"
            );
            continue;
        };
        if let Err(e) = wiring.presenter.attach_target(
            world,
            item.balloon_target,
            balloon_window,
            balloon_world,
            balloon_atlas,
            // shell 側と同型: attach 対象 target と同じ式（`item.balloon_target`）で引き当てる。
            author_dpis.for_target(item, item.balloon_target),
        ) {
            error!(scope, error = %e, "emo2 attach: バルーン target の attach に失敗（log-first・継続）");
            continue;
        }
        // バルーン初回表示は面 0・bind なし・pattern なし（DD-9・R4.1 の「初回サーフェス表示＝
        // バルーン枠表示」）。初回枠は SERIKO ループ非駆動ゆえ空 pattern＝拡張前と観測等価（R5.4）。
        wiring.presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: item.balloon_target,
                surface_id: 0,
                binds: areka_emo_compose::BindSet::default(),
                pattern: areka_emo_compose::PatternState::default(),
                reply: None,
            },
        );
        // apply は同期ゆえ同一フレームで text_slot_view が Some になるのが正常経路（DD-4）。
        // None（上流の遅延化）は接続せず次フレーム再試行へ委ねる（R4.2）。
        let view = wiring.presenter.text_slot_view(item.balloon_target);
        connect_balloon_text(
            &wiring.runtime,
            view,
            // 再追従（[`run_text_scale_phase`]）は**同一の写像**で actor を引く——
            // ここと式が食い違うと、再追従が別 actor を作って文字だけ旧 k のまま残る。
            ActorKey::from(scope.to_string()),
            &balloon_model,
        );
        // 文字層 k 再追従（D11-3・R8.1）の再利用源: **いま文字層へ渡したのと同一の**モデルを
        // scope キーで記憶する（借用→move の 1 値ゆえ二つの供給先が別値になり得ない——別値になると
        // 「装着時と再追従時で別定義」という静かな食い違いが生まれる・R4.1/4.2）。文字層スケール相
        // （[`run_text_scale_phase`]）はこれを再利用して binding を組み直す（再パースしない）。
        // 装着が `text_slot_view` None で次フレームへ委ねられた場合でもモデル自体は有効ゆえ、
        // 接続成否に関わらず記憶する（再追従は未登録 actor を静穏 skip する・7.1 の契約）。
        wiring.balloon_models.insert(scope, balloon_model);
    }

    info!(
        planned = planned_count,
        attached = attached_count,
        missing = plan.missing_assets.len(),
        unused = plan.unused_assets.len(),
        "emo2 attach: 装着計画を実行（planned＝計画件数・attached＝実装着件数）"
    );

    // ゲートを通過した attach 試行の完了。以降は no-op（高々 1 回）。
    wiring.attached = true;
}

/// バルーン文字層スロットの接続判断（R4.2 の None 分岐を headless に切り出した補助・DD-4）。
///
/// `text_slot_view` が `Some` なら [`TextLayerRuntime::register_actor_view`] で actor を登録し
/// `true` を返す。`None`（初回 `ShowSurface` 未合流＝上流の遅延化・Revalidation Trigger）なら
/// 登録せず `warn!` して `false` を返し、接続を次フレーム再試行へ委ねる（panic しない・R4.2）。
/// 登録判断を純結線として切り出すことで、GPU 不要の headless 単体テストが None 経路を檻に入れられる。
pub(super) fn connect_balloon_text(
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    view: Option<TextSlotView>,
    actor: ActorKey,
    model: &BalloonModel,
) -> bool {
    match view {
        Some(view) => {
            runtime.borrow_mut().register_actor_view(actor, &view, model);
            true
        }
        None => {
            warn!(
                actor = %actor.as_str(),
                "emo2 attach: text_slot_view が None（初回 ShowSurface 未合流）→ 文字層接続を次フレームへ委ねる（R4.2）"
            );
            false
        }
    }
}
