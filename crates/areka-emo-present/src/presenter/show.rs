//! `ShowSurface` の適用（`EmoPresenter::apply_show`）——k 導出・引き当て／計画・頁と葉の用意・
//! 装着の遅延生成・節の写し＋配置＋可視化・当たりの描き先の塗り・表示成立点の状態更新を 1 呼び出しで行う単一漏斗。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{DrawPlan, PlanItem};
use wintf::ecs::window::transition_diag;

use crate::draw_nodes::DrawNodes;
use crate::gpu_atlas::GpuAtlas;

use super::scale_change::{AppliedScale, ScaleChange};
use super::timing::{EmitContext, FrameTiming, Stage, compose_key_hash};
use super::transition_record::{SurfaceRecord, SurfaceStage, stamp_of, surface_line};
use super::{
    BindSet, ComposeError, DPI, EmoPresenter, GraphicsCore, PatternState, PresentError,
    PresentOutcome, TargetId, VisibilityOwnership, VisualMount, World, derive_scale,
};

impl EmoPresenter {
    /// `ShowSurface` の適用（キャッシュ引き当て or 計画 → 頁と葉の用意 → 節の写し・配置の書き込み → 可視化 → 当たりの描き先）。
    ///
    /// 結果は戻り値で返す。応答の口へ送るのは指令の入口（`apply`）だけで、ここは送らない。
    ///
    /// 手順:
    /// - 未装着なら `error!` ＋ `Err(TargetNotAttached)`。
    /// - 窓の `DPI` component と target 政策から**この適用に使う k**を導出する（[`derive_scale`]・
    ///   component 不在は `None` のまま渡して k=1.0 への縮退へ落とす）。k はキー要素ではない: 合成入力
    ///   （surface id＋bind 集合＋pattern）が完全一致するヒットなら計画を組み直さないし、k だけが変わった適用も
    ///   ヒットする。
    /// - ミスなら計画を組む（native 原寸）。`SurfaceNotFound` は `error!` ＋表示不変＋`Err`、
    ///   `EmptyComposition` は `warn!` ＋ Hide 縮退＋`Ok`（外形の上限越えの `ExtentTooLarge` も同じ扱い）。
    /// - 頁の絵と計画の葉を用意する（失敗し得る GPU 呼び出しはここだけ・どの状態にも触れる前）。
    ///   ミスなら続けて `cache.insert` へ計画を渡す。
    /// - `mount` 未生成なら遅延生成し、節の写し → 配置（原寸・k）→ 可視化の順に同一呼び出し内で
    ///   書く。拡大は wintf の描画経路（`render_surface` の `SetTransform`）が `Arrangement.scale`＝k で掛け、
    ///   原寸のまとめの節が原寸で描いてから k 倍で重ねるので、この漏斗は k の値で手順を分岐しない。
    ///
    /// # 毎フレーム経路は再利用席の上を走る
    ///
    /// ミス経路の計画は [`FrameBudget`] の
    /// 席か、キャッシュ追い出しエントリの容量回収で回る。定常状態（寸法不変・初回確保後）ではこの
    /// 経路の新規確保は 0 であり、確保が起きた回だけが perf サマリ行の `alloc_*` に現れる。
    ///
    /// [`FrameBudget`]: super::budget::FrameBudget
    ///
    /// # 可視化手順だけが所有権でゲートされる
    ///
    /// [`VisibilityOwnership::External`] の target では、末尾の可視化（`set_visible(true)`／`visible=true`）
    /// と mount 遅延生成の初期可視性だけが変わり、それ以外の全手順は所有権に依らず共通（k 導出・計画・
    /// キャッシュ・節・配置・`applied`／`native_size`／`last_show`／`pending_resize`・
    /// 下の表示の行）。表示成立点が 1 つであることを崩さないための境界であり、「不可視のまま配置先と面が
    /// 確立する」はこの共通部分がそのまま与える。可視化は [`EmoPresenter::show_target`] が同じ漏斗を
    /// 再通過したうえで付与する。
    ///
    /// `reveal` は「呼び手がこの適用の後に窓を見えるようにする」こと。見えていなかった外から所有される
    /// 窓を出し直す [`EmoPresenter::show_target`] だけが真を渡し、表示の行の水準の判定（コマ送りだけの
    /// 適用か）に使う。
    pub(super) fn apply_show(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        reveal: bool,
    ) -> PresentOutcome {
        // 計時は無条件に開始する。ログ設定を参照しないので、計時ログの有効・無効で表示経路が分岐しない。
        // `emit` は `self` を消費するため、以降の early return は計時器を黙って drop する＝成立点の対でしか
        // 行が出ない。
        let mut timing = FrameTiming::start();

        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(
                ?target_id,
                surface_id,
                "apply(ShowSurface): 未装着ターゲット"
            );
            return Err(PresentError::TargetNotAttached(target_id));
        };

        // 窓 DPI は wintf の `DPI` component から読む。component 不在は `None` のまま [`derive_scale`] へ渡す:
        // ここで 96 を捏造すると k=1.0 への縮退（error! ＋ k=1.0）が「正常系のふり」で通ってしまう。
        let window = target.window;
        let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
        let scale = derive_scale(target.policy, window_dpi);

        // コマ送りだけの適用か。どの状態も上書きする前に決める: 前に成立した面の番号と着せ替えが同じ・
        // 前に適用した拡大率が同じ・見えていなかった窓がこの適用で見えるようにならない、の 3 つが揃うとき
        // 真。シェル（指令で見える窓）はこの漏斗の末尾で見えるようになり、バルーン（外から所有）は呼び手が
        // `reveal` を渡したときだけ見えるようになる。
        let becomes_visible = !target.visible
            && (reveal || matches!(target.ownership, VisibilityOwnership::CommandDriven));
        let frame_step = !becomes_visible
            && target.applied == Some(scale)
            && target
                .last_show
                .as_ref()
                .is_some_and(|(id, b, _)| *id == surface_id && *b == binds);

        // 引き当ては合成入力（id＋binds＋pattern）の完全一致のみヒット。k はキーに参加しない: 計画と面は
        // いずれも原寸で k を含まないので、k だけが変わった適用は同じエントリにヒットし、変換の係数
        // （下の `set_layout`）だけが変わる。pattern は指令が運ぶ現在コマ集合をそのまま透過する。
        //
        // `touch` であって `get` ではない（容量 3 の LRU）。ここが 1 適用に 1 回の引き当て点＝最近使用順を
        // 動かす唯一の場所。`get`（順序を動かさない読み取り）へ替えると置換は挿入順（FIFO）へ静かに退化
        // するが、表示も確保計数も変わらないので、その退化は `presenter_cache_capacity_tests.rs` だけが
        // 捕まえる。
        let cache_hit = target.cache.touch(surface_id, &binds, &pattern);
        timing.mark(Stage::CacheLookup);
        if !cache_hit {
            // 計画は画面の節の材料で書き出しの出どころ（native 原寸・emo-compose は k を知らない）。席を閉包で
            // 借りるのは、計画の大きさが合成器の内側で決まり、伸長の観測を席を使い終えた直後にしか置けないため
            // （`FrameBudget::plan_scratch` の doc）。
            let planned = target.budget.plan_scratch(|plan| {
                target.composer.plan_into(
                    plan,
                    &target.emo_world,
                    &target.atlas,
                    surface_id,
                    &binds,
                    &pattern,
                )
            });
            timing.mark(Stage::Compose);
            match planned {
                Ok(()) => {}
                Err(
                    e @ (ComposeError::EmptyComposition(_) | ComposeError::ExtentTooLarge { .. }),
                ) => {
                    // 全透明退化（外形 0×0）: 許容される正常退化として Hide 縮退＋reply Ok（skip ではない）。
                    // 外形の上限越え（合成が error! 済み・絵は確保されない）も同じく隠す。
                    if matches!(e, ComposeError::EmptyComposition(_)) {
                        tracing::warn!(
                            ?target_id,
                            surface_id,
                            was_visible = target.visible,
                            "apply(ShowSurface): 全透明退化（EmptyComposition）→ Hide 縮退（reply Ok）"
                        );
                    } else {
                        tracing::warn!(
                            ?target_id,
                            surface_id,
                            was_visible = target.visible,
                            error = %e,
                            "apply(ShowSurface): 外形が上限越え（ExtentTooLarge）→ Hide 縮退（reply Ok）"
                        );
                    }
                    if let Some(mount) = target.mount.as_ref() {
                        mount.set_visible(world, false);
                    }
                    target.visible = false;
                    // この縮退は Hide と同じ表示結果ゆえ現サーフェス無し。
                    target.current_surface_id = None;
                    target.failure_note = None;
                    return Ok(());
                }
                Err(e) => {
                    // 解決不能 id（SurfaceNotFound 等）: error! ＋ 表示不変 ＋ reply Err。
                    tracing::error!(
                        ?target_id,
                        surface_id,
                        error = %e,
                        "apply(ShowSurface): 合成失敗 → 表示は適用前のまま（reply Err）"
                    );
                    target.failure_note = None;
                    return Err(PresentError::Compose(e));
                }
            }
        }

        // 頁の上げと葉の記録。失敗し得る GPU 呼び出しはここに集約し、節にも合成メモにも触れる前に置く:
        // ここで失敗すればメモ・節・配置・可視性・`applied`／`native_size`／`last_show`／
        // `pending_resize` の 1 つも動いていない。
        let plan = if cache_hit {
            &target
                .cache
                .get(surface_id, &binds, &pattern)
                .expect("直前に引き当て済み")
                .plan
        } else {
            target.budget.planned()
        };
        prepare_gpu(
            world,
            target.gpu.get_mut(),
            &target.atlas,
            plan,
            cache_hit,
            target_id,
            &mut target.failure_note,
        )?;
        timing.mark(Stage::Upload);

        if !cache_hit {
            // 容量回収は計画と記録の成功後に限る（`take_recycled` の契約）。失敗し得る位置で呼ぶと
            // スロットが空のまま残り、直後の同一入力の適用がヒットせず計画の組み直しへ落ちて「失敗時は表示も
            // キャッシュも適用前のまま」が崩れる。
            let recycled = target.cache.take_recycled();
            // 追い出しエントリの計画の容量を受け取り、席と交代させる（複写も確保も起きない）。回収が
            // 成立しなければ空から始まり、以後の伸長が計数される。
            let mut plan = super::budget::FrameBudget::recycled_plan(recycled);
            target.budget.swap_plan_scratch(&mut plan);
            target
                .cache
                .insert(surface_id, binds.clone(), pattern.clone(), plan);
            // `Stage::Insert` の区間は挿入（スロット置換）と席の交代（O(1) の入れ替え）を含む。旧エントリは
            // `take_recycled` で先に回収済みなので、対の解放 churn はこの区間に無い。
            timing.mark(Stage::Insert);
        }

        // 使えるエントリ（引き当て済み・順序は動かさない `get`）。原寸は計画の外形、物理寸は
        // 丸め権威の式 1 つ（`scaled_extent`）で導く。照会値（`target_physical_size`）と同じ式で同じ値になる。
        let entry = target
            .cache
            .get(surface_id, &binds, &pattern)
            .expect("直前に引き当て済み");
        let native = (entry.plan.extent.w, entry.plan.extent.h);
        let base_surface = entry.plan.base_surface;
        let physical = scale.scaled_extent(native.0, native.1);

        // 装着の遅延生成（初回表示・原寸確定後・純 ECS・失敗経路なし）。`WucGraphicsResource`／
        // `Compositor` は要らない。
        if target.mount.is_none() {
            // 初期可視性は所有権から導出する。`CommandDriven` は可視で構築（この漏斗の末尾で
            // `set_visible(true)` する経路と同値）、`External` は不可視で構築する。「可視で spawn してから
            // 消す」経路を持たないことで、可視状態を component レベルでも一度も経由しないことを構造で
            // 保証する。導出をここで行う（装着側の既定に委ねない）のは、可視性の所有者を知っているのが
            // この層だけだから。
            let initially_visible = matches!(target.ownership, VisibilityOwnership::CommandDriven);
            let mount = VisualMount::attach(world, window, native, scale, initially_visible);
            target.nodes = Some(DrawNodes::new(mount.group()));
            target.mount = Some(mount);
        }

        // 節 → 配置 → 可視化（同一呼び出し内＝原子入替）。可視化手順だけが所有権で
        // ゲートされる: `External` の target では表示状態の確立（計画・節・配置・k・面 id・
        // 再表示入力・窓寸 reconcile 要求）は全て完了し、可視性の付与だけを行わない。面切替もループ由来の
        // 反復指令も、経路を問わず可視状態を変えない。
        let visualize = matches!(target.ownership, VisibilityOwnership::CommandDriven);

        // 状態照合の前値導出。前値を上書きする前に前回表示の原寸と物理寸を組み立てる。物理寸は契約式
        // `applied.scaled_extent(native_size)` に従う: 別フィールドで物理寸を二重に持つと更新点が 2 つに
        // なり、片方だけが書かれる欠陥を招く。
        //
        // `resized`（遷移観測）は原寸の外形が前回表示から変わったか。k だけの変化は false で、
        // `size_changed`（物理寸の変化・窓寸 reconcile の材料）とは別の述語である。
        //
        // 前値なし（初回表示）は `None` ≠ `Some(..)` なので必ず差分扱いになる。窓は起動時 k₀ 見積もり寸で
        // 生成されており実窓 DPI 由来の k と一致する保証がないため、初回を黙らせると k₀ 補正が永久に走らない。
        let resized = target.native_size != Some(native);
        let prev_physical = target
            .applied
            .zip(target.native_size)
            .map(|(k, (nw, nh))| k.scaled_extent(nw, nh));
        let size_changed = prev_physical != Some(physical);

        // 遷移観測: サーフェス更新の記録。寸が動いた回だけ、かつ観測が有効なときだけ組む。前置ガードが
        // 行の組立より外側に無いと、既定 OFF の運転でも毎フレーム `String` を確保して定常状態の
        // アロケーション 0 が壊れる。発行は `debug!` なのでガードの有無で出力は変わらず、この退行を
        // 捕まえるのは `transition_record_tests.rs` の本文走査だけである。
        let observe_surface = (size_changed || resized) && transition_diag::is_enabled();
        if observe_surface {
            transition_diag::emit_line(&surface_line(&SurfaceRecord {
                // 刻印は World 資源から組む。本経路は `&mut World` を持つ観測点であり、
                // スレッド局所ミラーは World を借りられない点の専用口である。
                stamp: stamp_of(world),
                stage: SurfaceStage::Upload,
                target_id,
                size: Some(physical),
                resized: Some(resized),
                reason: None,
            }));
        }

        let mount = target.mount.as_ref().expect("直上で生成済み");
        // 節: 同じ位置の項が等しければ触らない（同一エントリの再適用で版を上げない）。葉は上で記録済み。
        let gpu = target.gpu.get_mut().as_mut().expect("直前に用意した");
        let nodes = target.nodes.as_mut().expect("装着と一緒に作る");
        let rebuilt = nodes.sync(world, gpu, &entry.plan)?;
        target.budget.note_rebuilds(rebuilt);
        let plan_items = u32::try_from(entry.plan.items.len()).unwrap_or(u32::MAX);
        let plan_groups = entry
            .plan
            .items
            .iter()
            .filter(|item| matches!(item, PlanItem::GroupBegin { .. }))
            .count() as u32;
        timing.mark(Stage::Nodes);
        // 配置（原寸・k）: 同値なら書かない。bounds（＝当たりの α を読む座標の基準・物理寸）は wintf が
        // `GlobalArrangement` へ導く。可視性に依らず常に合わせる——不可視中の更新は安全であり
        // （`HitTest::none()` 中は入れ物の α が判定に使われない）、ここで揃えておくことが「不可視期間中に
        // 生じた変化を取りこぼさない」の土台になる。
        mount.set_layout(world, native, scale);
        target.shown_base = Some(base_surface);
        if visualize {
            mount.set_visible(world, true);
            target.visible = true;
        }
        // 可視化後の記録。節・配置を揃え終えた点＝「描画内容がこの寸で見えるようになった」
        // 瞬間で、判定側はこの行の `t_us` と当該窓の窓書込の `t_us` の差（`visualize_to_write_us`）で
        // 「窓矩形と描画内容が食い違う区間」を測る。条件は upload の記録と同じ 1 つの札を使う
        // （2 度評価すると片方だけが変わる形を作れる）。
        if observe_surface {
            transition_diag::emit_line(&surface_line(&SurfaceRecord {
                stamp: stamp_of(world),
                stage: SurfaceStage::Visualize,
                target_id,
                size: Some(physical),
                // 段階の意味を持たない 2 フィールドは番兵で残す（落とさない）。
                resized: None,
                reason: None,
            }));
        }
        // 表示確立＝この id が現サーフェス（全透明でも成立・α 非依存の単一真実源）。
        target.current_surface_id = Some(surface_id);
        target.paint_hit(
            world.get_resource::<GraphicsCore>(),
            target_id,
            surface_id,
            base_surface,
            native,
        );
        // ここが**表示成立点**＝ k・native 原寸・再表示入力の唯一の更新点。
        // 手前の失敗経路はすべて early return 済みゆえ、失敗時は前 k・前表示が保たれる。

        // 状態照合＝窓寸 reconcile 要求の生成。判定材料（`prev_physical`／`size_changed`）は上で導出済み。
        // 要求を積むのはここ（表示成立が確定した後・`applied`／`native_size` の上書きより手前）。
        if size_changed {
            // 差分あり＝呼び手（frame drain フェーズ）へ新物理寸を報告する。同寸のときは**何も触らない**
            // ——`None` を書き戻すと未消費の要求を殺してしまう（取りこぼしを作らない・べき等）。
            target.pending_resize = Some(physical);
        }

        target.applied = Some(scale);
        // 倍率の記録は `applied` を書くこの点だけで積む（装着ごとの起点と比べる）。
        let now = AppliedScale {
            ratio: scale,
            dpi_x: window_dpi.map(|(x, _)| x),
        };
        if target.scale_baseline != Some(now) {
            self.scale_changes.push(ScaleChange {
                target: target_id,
                before: target.scale_baseline,
                after: now,
            });
            target.scale_baseline = Some(now);
        }
        // いま表示に使ったエントリ由来の原寸をそのまま写す。計画を組んだ回か否かで分岐させると、「insert 済みの
        // まま失敗 → 後からヒットで成立」の経路で照会値が画面と乖離する。容量 3 ではヒットしたエントリが
        // 直前の挿入とは限らないので、エントリの計画の外形から読む。
        target.native_size = Some(native);
        // 合成キーの安定ハッシュ（perf サマリ行の `key_hash`）は `last_show` への move の直前に取る。全段の
        // `mark` が済んだ後なので、この走査（借用のみ・確保なし）は段別所要へ混入せず `t_total_us` にだけ含まれる。
        let key_hash = compose_key_hash(surface_id, &binds, &pattern);
        // 成立をはさんだので、次の装置の失敗は新しい続き＝ERROR。
        target.failure_note = None;
        target.last_show = Some((surface_id, binds, pattern));

        // 表示成立点の観測ログ。最初の表示と状態の替わりは INFO、コマ送りだけは DEBUG。実機サインオフは
        // `RUST_LOG` を grep して面・着せ替え・拡大率・窓の大きさの替わりと隠した後の出し直しをここで読み、
        // コマ送りだけの適用は `areka_emo_present=debug` で同じ文言・同じ欄のまま 1 適用 1 行で読める。
        // k 導出値（`k`・`k_ratio`）と適用寸（`native_*`・`scaled_*`＝物理寸）が揃うことで、2 水準
        // （125%/200%）の実行が「異なる物理寸で描かれた」ことをログだけで決定論的に判定できる。欄の並びは
        // このマクロ 1 か所に置く（水準は字面で渡す決まりのため）。
        macro_rules! show_line {
            ($level:expr) => {
                tracing::event!(
                    $level,
                    ?target_id,
                    surface_id,
                    cache_hit,
                    // k の有理表現（既約 num/den）。`ScaleRatio` の num/den は非公開ゆえ `Debug` で出す。
                    k_ratio = ?scale,
                    k = scale.as_f32(),
                    author_dpi = target.policy.author_dpi(),
                    // `None` は k=1.0 への縮退（DPI component 不在 → k=1.0）そのものなので潰さずに出す。
                    window_dpi = ?window_dpi,
                    native_w = native.0,
                    native_h = native.1,
                    scaled_w = physical.0,
                    scaled_h = physical.1,
                    // 今回の表示成立が窓寸 reconcile 要求を積んだか（状態照合の観測点）。
                    size_changed,
                    "apply(ShowSurface): 表示・マスクを更新"
                )
            };
        }
        if frame_step && !size_changed {
            show_line!(tracing::Level::DEBUG);
        } else {
            show_line!(tracing::Level::INFO);
        }

        // perf サマリ行。上の表示の行と同一適用の対として隣接して出る別行で、ここが唯一の emit 点＝
        // 早期復帰の経路では 1 行も出ない。`take_delta` は増分を取り出して器を 0 に戻す。早期復帰で
        // 取り出されなかった増分は次の適用へ持ち越される（budget.rs の契約）ので、確保は成立した次の行に
        // 現れ、累積（`cumulative`）は常に厳密である。
        let allocs = target.budget.take_delta();
        timing.emit(
            &EmitContext {
                target_id,
                surface_id,
                cache_hit,
                key_hash,
                // 遷移観測の刻印そのものから読む。供給元が 2 つになると
                // 「perf 行と surface 行が同一フレームで突合できる」が静かに壊れる。
                frame: stamp_of(world).frame,
                plan_items,
                plan_groups,
            },
            allocs,
        );
        Ok(())
    }
}

/// 頁の絵と計画の葉を用意する。装置が替わっていたら新しい装置で控えを作り、計画の葉を全部記録できて
/// から前の控えと替える（失敗すれば前の控えのまま・節の葉の差し替えは写しの段）。
///
/// 装置が無いとき、合成メモに当たった適用は控えの葉のまま節を写す（その計画の葉はその控えで記録
/// 済み）。外れた適用は表示を記録できない。
/// `note` は同じ原因の装置の失敗の続きを `debug!` にまとめる控え（失敗の原因を書き、成立点が空にする）。
fn prepare_gpu(
    world: &mut World,
    gpu: &mut Option<GpuAtlas>,
    atlas: &AtlasTable,
    plan: &DrawPlan,
    cache_hit: bool,
    target_id: TargetId,
    note: &mut Option<(&'static str, i32)>,
) -> Result<(), PresentError> {
    let Some(device) = world
        .get_resource::<GraphicsCore>()
        .and_then(|gfx| gfx.d2d_device())
        .cloned()
    else {
        if cache_hit && gpu.is_some() {
            return Ok(());
        }
        // 同じ原因の続きは 2 回目から DEBUG。原因は返す `Device` の値と同じ。
        const NO_GFX: (&str, i32) = ("GraphicsCore resource", 0);
        if *note == Some(NO_GFX) {
            tracing::debug!(
                ?target_id,
                "apply(ShowSurface): GraphicsCore 不在または DeviceContext 不在（表示を記録できない）"
            );
        } else {
            tracing::error!(
                ?target_id,
                "apply(ShowSurface): GraphicsCore 不在または DeviceContext 不在（表示を記録できない）"
            );
        }
        *note = Some(NO_GFX);
        return Err(PresentError::Device {
            hresult: NO_GFX.1,
            context: NO_GFX.0,
        });
    };
    let repeat = *note;
    let remember = |note: &mut Option<(&'static str, i32)>, e: PresentError| {
        if let PresentError::Device { hresult, context } = &e {
            *note = Some((*context, *hresult));
        }
        e
    };
    if let Some(gpu) = gpu.as_mut().filter(|g| *g.device() == device) {
        return gpu.prepare(plan, repeat).map_err(|e| remember(note, e));
    }
    let mut fresh = GpuAtlas::new_with(&device, atlas, repeat).map_err(|e| remember(note, e))?;
    fresh.prepare(plan, repeat).map_err(|e| remember(note, e))?;
    *gpu = Some(fresh);
    Ok(())
}
