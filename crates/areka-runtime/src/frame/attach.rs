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

use crate::session_hooks::SessionHooks;
use crate::target_map::window_scope;
use crate::{shell_carry, skin_facts_copy};
use areka_placement::descript_window_keys::fallback_candidates;
use areka_placement::spawn::GhostWindows;

use super::balloon_face::{FaceCause, select_balloon_face};
use super::{
    AuthorDpis, BalloonScopeAssets, BootAssets, RuntimeWiring, ScopeAssets, balloon_target,
    shell_target,
};

/// 窓×資産の scope 突き合わせ結果（純関数 [`plan_attachments`] の戻り値）。
///
/// `GhostWindows::scopes()`（`usize`・昇順・**正**）を [`BootAssets`] の資産 scope と照合した
/// 装着計画。三分類は排他:
/// - `items`: 窓と資産の双方が揃った scope の装着項目（`window_scopes` の出現順を保つ）。
/// - `missing_assets`: 窓はあるが対応資産が無い scope（呼び手が `warn!`＋skip＝表示なし縮退）。
/// - `unused_assets`: 資産はあるが窓が無い scope（呼び手が `debug!`＋破棄）。
///
/// 純粋・決定論（GPU 不要）。attach フェーズは件数を `info!` に出すだけで、`items.len()` が期待窓数と
/// 一致することの表明は本体の spine のテストが持つ（warn+skip 縮退が scope 導出バグを隠さないための檻）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachPlan {
    /// 窓と資産の双方が揃った scope の装着項目（`window_scopes` 出現順）。
    pub items: Vec<PlannedAttach>,
    /// 窓はあるが対応資産が無い scope（`usize` のまま・呼び手が `warn!`＋skip 縮退）。
    pub missing_assets: Vec<usize>,
    /// 資産はあるが窓が無い scope（`u32`・呼び手が `debug!`＋破棄）。
    pub unused_assets: Vec<u32>,
}

/// 1 scope 分の装着計画項目（attach フェーズが消費する）。
///
/// scope の shell／balloon 表示対象（`target_map` の正本の `2*scope`／`2*scope+1` 採番）と
/// 初期表示 surface id（[`super::assets::ScopeAssets`] へ焼き込み済み）を運ぶ。
/// `static_binds` は [`BootAssets`] 単一共有ゆえ本項目には複製しない（attach フェーズが
/// `assets.static_binds` を直接読む）。
///
/// `shell_index`／`balloon_index` は attach フェーズが非 Clone な `EmoWorld` を
/// `assets.shells`／`assets.balloons` から添字で move 消費するための添字（窓一覧との突き合わせで
/// 自然に確定する）。当該 scope に balloon 資産が無い場合 `balloon_index` は `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAttach {
    /// 対応 scope 番号（`usize`→`u32` 変換済み）。
    pub scope: u32,
    /// シェル表示対象（`shell_target(scope)`＝偶数）。
    pub shell_target: TargetId,
    /// バルーン表示対象（`balloon_target(scope)`＝奇数）。
    pub balloon_target: TargetId,
    /// 初期表示 surface id（`ScopeAssets.initial_surface_id`）。
    ///
    /// attach フェーズはこの値でシェル初回表示を駆動しない（シェルは最初の `\s` cue まで非表示）。
    /// ghost の読み直しで控えた面が読み直したシェルに無いときの戻り先（descript の既定の面・違えば
    /// 続けて組み込みの既定の面の候補の並び）として使う。
    pub initial_surface_id: u32,
    /// `assets.shells` 内の対応添字（attach フェーズの add 消費用）。
    pub shell_index: usize,
    /// `assets.balloons` 内の対応添字（同 scope・無ければ `None`）。
    pub balloon_index: Option<usize>,
}

/// 窓一覧と構築資産を突き合わせ装着計画を返す純関数（GPU 不要の決定論単体テスト対象）。
///
/// **`GhostWindows::scopes()`（`window_scopes`・`usize`・正）が唯一の正**。窓一覧を
/// 走査し、各窓 scope を [`window_scope`] で scope 番号へ直してから `assets.shells` を `scope`
/// フィールドで照合する:
/// - 一致 → `items` へ [`PlannedAttach`] を積む（`window_scopes` の出現順を保つ）。
/// - 不一致、または上限（`i32::MAX`）を越える `usize`（表示対象を採番できない）
///   → 当該窓 scope（`usize`）を `missing_assets` へ。
///
/// 加えて、`assets.shells` の各 scope のうち `window_scopes` に現れないもの（資産あり窓なし）を
/// `unused_assets`（`u32`）へ集める（`u32`→`usize` は無損失ゆえ `as` で比較）。
///
/// # 純粋性
/// 状態・I/O・GPU なし。戻り値は入力順に決定論的（`window_scopes`／`assets.shells` の順序を保つ）。
/// `window_scopes` は `GhostWindows` 契約により一意昇順が期待されるが、本関数は分類の正しさを
/// その前提に依存しない（重複窓は重複項目として忠実に写す）。
pub fn plan_attachments(window_scopes: &[usize], assets: &BootAssets) -> AttachPlan {
    let mut items = Vec::new();
    let mut missing_assets = Vec::new();

    for &scope_usize in window_scopes {
        let Some(scope) = window_scope(scope_usize) else {
            missing_assets.push(scope_usize);
            continue;
        };
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
            None => missing_assets.push(scope_usize),
        }
    }

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

/// attach フェーズ（高々 1 回）。
///
/// ゲート（`GhostWindows` Resource ＋ `GraphicsCore::is_valid()` ＋ `WucGraphicsResource::is_valid()`）成立
/// フレームで純関数 [`plan_attachments`]を確定し、計画項目ごとに shell／balloon target を
/// 装着する。**バルーンのみ**面 0 の `ShowSurface` を駆動して文字層スロットを取得するが、その手前で
/// 可視性を外部所有へ移すため画面には出ない——「不可視のままの確立」であり、可視化はバルーン可視性
/// 制御が可視コンテンツの配置を契機に別途付与する。**シェルは `ShowSurface` そのものを発行せず**最初のさくらスクリプト `\s` cue まで
/// 非表示を保つ。
/// 資産は `Option::take` で高々 1 回消費し、ゲート不成立では消費せず
/// 次フレーム再試行へ委ねる（表示なし縮退・hang しない）。窓あり資産なしは `warn!`＋skip、資産あり
/// 窓なしは `debug!`＋破棄で log-first に観測し、計画件数と実装着件数を `info!` に列挙する（spine が
/// 件数一致を積極 assert）。個別の attach 失敗・窓欠落は `error!`／`warn!`＋継続であり
/// panic しない——1 scope の失敗は他 scope を巻き込まない。
///
/// `apply` は同期実行のため balloon の `text_slot_view` は同一フレームで
/// `Some` になるのが正常経路。万一 `None`（上流の遅延化）なら接続せず `unconnected_text` に
/// 控え、繋ぎ直しは配置先が取れた最初のフレームの
/// [`run_text_scale_phase`](super::run_text_scale_phase) が行う（[`connect_balloon_text`]）。
pub fn run_attach_phase(wiring: &mut RuntimeWiring, world: &mut World) {
    // 装着済みなら以降 no-op（装着後の remove/insert churn を避ける）。
    if wiring.attached {
        return;
    }

    // ゲートが欠けるか無効な間は資産を消費せず attached も立てず、次フレーム再試行へ委ねる（hang しない）。
    let gate_ready = world.get_resource::<GhostWindows>().is_some()
        && world
            .get_resource::<GraphicsCore>()
            .is_some_and(GraphicsCore::is_valid)
        && world
            .get_resource::<WucGraphicsResource>()
            .map(|r| r.is_valid())
            .unwrap_or(false);
    if !gate_ready {
        return;
    }

    // GhostWindows は Clone（小さな Entity 写像）。apply が `&mut World` を要するため、
    // 窓写像を先に clone して world の不変借用をループへ跨がせない（借用衝突回避）。
    let ghost_windows = world
        .get_resource::<GhostWindows>()
        .expect("ゲートで存在確認済み")
        .clone();
    let window_scopes: Vec<usize> = ghost_windows.scopes().collect();

    // 資産は高々 1 回消費（ゲート成立後にのみ take）。既に None（二重 attach の異常）なら log-first で
    // 観測して打ち切る（panic しない・attached を立てて以降の空回りを止める）。
    let Some(assets) = wiring.assets.take() else {
        warn!(
            "runtime attach: ゲート成立だが BootAssets が既に消費済み（想定外）→ 装着せず打ち切り"
        );
        wiring.attached = true;
        return;
    };

    let plan = plan_attachments(&window_scopes, &assets);

    if !plan.missing_assets.is_empty() {
        warn!(
            missing_scopes = ?plan.missing_assets,
            "runtime attach: 窓はあるが対応資産が無い scope（表示なし縮退・skip）"
        );
    }
    if !plan.unused_assets.is_empty() {
        debug!(
            unused_scopes = ?plan.unused_assets,
            "runtime attach: 資産はあるが窓が無い scope（破棄）"
        );
    }

    // 非 Clone な `EmoWorld` を計画の添字で個別に move 消費するため Option 包みにする（take で 1 回）。
    let BootAssets {
        shells,
        balloons,
        // resolver は attach では未使用（seriko へは `wire_runtime` が手渡す）。
        resolver: _,
        // static_binds はシェルの初回表示には使わない（初回表示は最初の `\s` cue が駆動する
        // Show{shell,id,binds=static_binds} に載る・起動時オンの bindgroup default は seriko が保持）。
        // ghost の読み直しで控えた着せ替えを、読み直したシェルの既定との差として名前で出し直すのにだけ
        // 使う。
        static_binds,
        // bind_resolver の写しも同じく読み直しの着せ替えの名前を引くのにだけ使う（解決は seriko の写し）。
        bind_resolver,
        // loop_tables は attach では未使用（SERIKO ループ表は spawn_seriko の actor 構築が手渡す）。
        loop_tables: _,
        // author_dpi: descript 宣言由来の原稿 DPI を attach 時の target 政策として
        // 供給する。shell と balloon で別宣言ゆえ引き当てを取り違え
        // ないよう [`AuthorDpis`] へ束ねる（下の `attach_target` 呼び 2 箇所が `for_target` で引く）。
        shell_author_dpi,
        balloon_author_dpi,
        boxes,
        cursors,
    } = assets;
    // 箱の束: 置き場所の表・`\s` の解決の閉包・フォントを探す場所の順を文字の層へ渡す。
    // 全スコープで共有する 1 束ゆえループの前に 1 度。ゴーストの切替は文字の層ごと作り直し、
    // この相がもう 1 度走って新しいシェルの束を渡す。
    boxes.hand_to(&mut wiring.runtime.borrow_mut(), world);
    // シェルのカーソルも同じく新しいシェルの分へ入れ替える。
    cursors.hand_to(world);
    let author_dpis = AuthorDpis {
        shell: shell_author_dpi,
        balloon: balloon_author_dpi,
    };
    let mut shells: Vec<_> = shells.into_iter().map(Some).collect();
    // 文字層へ渡すバルーン定義は各 scope 自身の [`BalloonScopeAssets::definitions`] の面 0（scope 別 2 層マージ
    // 済み）。World／アトラスと**同一の資産 1 件から**取り出すため、ある scope のバルーンが
    // 別 scope の系列由来の定義で駆動される取り違えが構造的に起こり得ない。
    let mut balloons: Vec<_> = balloons.into_iter().map(Some).collect();

    let planned_count = plan.items.len();
    let mut attached_count = 0usize;
    // ghost の読み直しの前に控えた面と着せ替え（読み直しの起こし直しのときだけ在る）。装着
    // できたシェルごとに、読み直したシェルで出す面と着せ替えを決めておき、ループの後で seriko へ送る。
    let carry = wiring.shell_carry.take();
    let mut carried_shows = Vec::new();
    let mut carried_binds = Vec::new();

    for item in &plan.items {
        let scope = item.scope;

        // --- shell target: char_window → attach_target（EmoWorld を move）。初回 ShowSurface は
        //     発行しない: シェルは最初の `\s` cue まで非表示・target のみ生成する。 ---
        let Some(shell_window) = ghost_windows.char_window(scope as usize) else {
            error!(
                scope,
                "runtime attach: char_window が無い（GhostWindows 不整合）→ この scope を skip"
            );
            continue;
        };
        let Some(shell_assets) = shells.get_mut(item.shell_index).and_then(|s| s.take()) else {
            error!(
                scope,
                shell_index = item.shell_index,
                "runtime attach: shell 資産の添字が空（二重消費？）→ skip"
            );
            continue;
        };
        let ScopeAssets {
            emo_world: shell_world,
            atlas: shell_atlas,
            ..
        } = shell_assets;
        // 控えた面が読み直したシェルに在るかは、装着で move する前の World で引く。
        let carried = carry.as_ref().and_then(|c| c.surface(scope)).map(|id| {
            shell_carry::decide(
                id,
                &fallback_candidates(scope as usize, item.initial_surface_id),
                |s| shell_world.surface(s).is_some(),
            )
        });

        wiring.presenter.attach_target(
            item.shell_target,
            shell_window,
            shell_world,
            shell_atlas,
            // author_dpi は attach 対象 target と同じ式で引く（`item.shell_target` を両方に書く）＝
            // shell/balloon の取り違えが 1 行の中で目視可能になる（両者 u16 で型は守ってくれない）。
            author_dpis.for_target(item, item.shell_target),
        );
        // シェルは初回 ShowSurface を attach で発行しない。SSP 互換の既定は「シェル表示なし（surface -1）」
        // であり、attach 時に surface0/surface10 を焼き付けるとゴースト起動の一瞬に規定面がちらつく。
        // 初回シェル表示は、最初の `\s[N]` cue が seriko→PresentBridge→drain 経路で運ぶ ShowSurface が
        // 駆動する（起動時オンの bindgroup default は seriko 保持の static_binds が Show に載る）。
        // シェル target の装着成功を計上（planned==attached の assert 用・balloon と対で 1 scope）。
        attached_count += 1;
        carried_shows.extend(carried.map(|show| (scope, show)));
        if let Some(binds) = carry.as_ref().and_then(|c| c.binds(scope)) {
            let key = ActorKey::from(scope.to_string());
            let decided = shell_carry::decide_binds(
                scope,
                binds,
                carry
                    .as_ref()
                    .map_or(&static_binds, |c| c.old_static())
                    .for_scope(&key),
                static_binds.for_scope(&key),
                &bind_resolver,
            );
            carried_binds.push((scope, decided));
        }

        // --- balloon target（同 scope の資産がある場合）: attach → 可視性を外部所有へ →
        //     不可視のままの確立（ShowSurface 面0・default）→ text_slot_view → register_actor_view ---
        let Some(balloon_index) = item.balloon_index else {
            warn!(
                scope,
                "runtime attach: 同 scope の balloon 資産が無い（balloon_index None）→ 文字層接続なし"
            );
            continue;
        };
        let Some(balloon_window) = ghost_windows.balloon_window(scope as usize) else {
            warn!(
                scope,
                "runtime attach: balloon_window が無い（GhostWindows 不整合）→ バルーン装着を skip"
            );
            continue;
        };
        // 当該 scope の資産 1 件を take で消費する（World／アトラス／定義は同一資産から取り出す
        // ＝別 scope の系列由来の定義が混ざり得ない）。
        let Some(BalloonScopeAssets {
            emo_world: balloon_world,
            atlas: balloon_atlas,
            definitions: balloon_definitions,
            background_color,
            name: balloon_name,
            ..
        }) = balloons.get_mut(balloon_index).and_then(|b| b.take())
        else {
            error!(
                scope,
                balloon_index,
                "runtime attach: balloon 資産の添字が空（二重消費？）→ バルーン装着を skip"
            );
            continue;
        };
        wiring.presenter.attach_target(
            item.balloon_target,
            balloon_window,
            balloon_world,
            balloon_atlas,
            // shell 側と同型: attach 対象 target と同じ式（`item.balloon_target`）で引き当てる。
            author_dpis.for_target(item, item.balloon_target),
        );
        // バルーンの可視性は可視性制御ただ一つが所有する。装着直後・初回指令の**前**に外部所有へ移す——後に置くと直下の
        // ShowSurface が先に可視化してしまい、起動時の不可視が成立しない。
        // 未装着 target のみが失敗し得る（直上で attach_target 済みゆえ通常は起こらない）が、
        // 沈黙させると以降のバルーンが理由の記録なく不可視で放置されるため error!＋skip する
        // （log-first。他 scope は巻き込まない）。
        if let Err(e) = wiring.presenter.set_visibility_ownership(
            item.balloon_target,
            areka_emo_present::presenter::VisibilityOwnership::External,
        ) {
            error!(
                scope,
                error = %e,
                "runtime attach: バルーン可視性の外部所有への設定に失敗（log-first・このバルーンを skip）"
            );
            continue;
        }
        // バルーンの表示確立は面 0・bind なし・pattern なし（初回枠）。上で外部所有へ移してあるため
        // 画面には出ず、文字の配置先と面だけが立つ。初回枠は SERIKO ループ非駆動ゆえ空 pattern。
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
        // apply は同期ゆえ同一フレームで text_slot_view が Some になるのが正常経路。
        // None（上流の遅延化）は接続せず控え、繋ぎ直しは文字の拡大率の相が行う。
        let view = wiring.presenter.text_slot_view(item.balloon_target);
        // 再追従（[`run_text_scale_phase`]）は**同一の写像**で actor を引く——
        // ここと式が食い違うと、再追従が別 actor を作って文字だけ旧 k のまま残る。
        let actor = ActorKey::from(scope.to_string());
        // 普通のバルーンの名前（フォルダ名）は装着の**前**に渡す——後だと、領域が
        // 同じままの再追従では警告が出ず、名前は次に値が新しく決まった登録からしか効かない。
        wiring
            .runtime
            .borrow_mut()
            .set_balloon_label(&actor, balloon_name);
        // 束を入れ、面 0 の定義を今の定義にする（唯一の書き手・矢印の絵と windowposition も届く）。
        // 文字層へ渡すのと再追従が使うのは書き手が入れた**同一の** `balloon_models[scope]`
        //——装着が `text_slot_view` None で次フレームへ委ねられた場合でも
        // モデル自体は有効ゆえ、接続成否に関わらず入れる（繋ぎ直しもこのモデルを使う）。
        wiring
            .balloon_definitions
            .insert(scope, balloon_definitions);
        select_balloon_face(wiring, world, scope, 0, FaceCause::Attach);
        if !connect_balloon_text(
            &wiring.runtime,
            view,
            actor,
            &wiring.balloon_models[&scope],
            // 面 0 の原点画素——無効表示の文字色の混色の相手。
            background_color,
        ) {
            wiring.unconnected_text.insert(scope, background_color);
        }
    }

    info!(
        planned = planned_count,
        attached = attached_count,
        missing = plan.missing_assets.len(),
        unused = plan.unused_assets.len(),
        "runtime attach: 装着計画を実行（planned＝計画件数・attached＝実装着件数）"
    );

    // ghost の読み直しの前の面と着せ替えを、台本の `\s[面]`・`\![bind,…]` と同じ道で seriko へ出し直す
    // （target は装着済み）。
    shell_carry::replay(world, &carried_shows, &carried_binds);

    // strict の判定に使う今のシェルとバルーンの事実を、装着した表示の層から入れる。
    // 置き場のゴーストが無ければ判定もしない。
    if let Some(sink) = (SessionHooks::of(world).seriko_sink)(world) {
        let scopes = window_scopes.iter().filter_map(|&s| window_scope(s));
        sink.set_facts(skin_facts_copy::copy(wiring.presenter(), scopes));
    }

    wiring.attached = true;
}

/// バルーン文字層スロットの接続判断（`None` 分岐を headless に切り出した補助）。
///
/// `text_slot_view` が `Some` なら [`TextLayerRuntime::register_actor_view`] で actor を登録し
/// `true` を返す。`None`（初回 `ShowSurface` 未合流＝上流の遅延化）なら
/// 登録せず `warn!` して `false` を返す（panic しない）。繋ぎ直しは文字の拡大率の相
/// （[`run_text_scale_phase`](super::run_text_scale_phase)）が配置先の取れた最初のフレームで行う。
/// 登録判断を純結線として切り出すことで、GPU 不要の headless 単体テストが None 経路を檻に入れられる。
///
/// `background` は当該 scope のバルーン面 0 の原点画素（[`BalloonScopeAssets::background_color`]）。
/// 無効表示（`\f[disable]`）の文字色を導く混色の相手であり、**装着の前**に渡す。順序は
/// `frame_attach_tests.rs` の `connect_balloon_text_resolves_text_with_the_background_at_attach_time`（実 GPU）が見張る。
pub(super) fn connect_balloon_text(
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    view: Option<TextSlotView>,
    actor: ActorKey,
    model: &BalloonModel,
    background: (u8, u8, u8),
) -> bool {
    match view {
        Some(view) => {
            let mut rt = runtime.borrow_mut();
            // 背景色は**装着の前**に渡す——装着の時点で覚えている値が無効表示
            // （`\f[disable]`）の 2 層に焼かれるため、後から入れても既装着スコープには効かず
            // 次の再追従まで効き目が遅れる（`set_balloon_background` の doc が定める契約）。
            rt.set_balloon_background(actor.clone(), background);
            rt.register_actor_view(actor, &view, model);
            true
        }
        None => {
            warn!(
                actor = %actor.as_str(),
                "runtime attach: text_slot_view が None（初回 ShowSurface 未合流）→ 文字層接続を次フレームへ委ねる（R4.2）"
            );
            false
        }
    }
}
