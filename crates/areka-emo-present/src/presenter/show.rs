//! `ShowSurface` の適用（`EmoPresenter::apply_show`）——k 導出・引き当て／合成＋表示記録・装着の遅延生成・
//! 表示記録＋配置＋マスク同期＋可視化・表示成立点の状態更新を 1 呼び出しで行う単一漏斗。

use wintf::ecs::window::transition_diag;

use crate::display::record_display;

use super::scale_change::{AppliedScale, ScaleChange};
use super::timing::{EmitContext, FrameTiming, Stage, compose_key_hash};
use super::transition_record::{SurfaceRecord, SurfaceStage, stamp_of, surface_line};
use super::{
    AlphaMaskResource, BindSet, ComposeError, DPI, EmoPresenter, GraphicsCore, PatternState,
    PresentError, PresentOutcome, Recompose, TargetId, VisibilityOwnership, VisualMount, World,
    derive_scale,
};

impl EmoPresenter {
    /// `ShowSurface` の適用（キャッシュ引き当て or 合成＋表示記録 → 表示記録・配置の書き込み → マスク同期 → 可視化）。
    ///
    /// 結果は戻り値で返す。応答の口へ送るのは指令の入口（`apply`）だけで、ここは送らない。
    ///
    /// 手順:
    /// - 未装着なら `error!` ＋ `Err(TargetNotAttached)`。
    /// - 窓の `DPI` component と target 政策から**この適用に使う k**を導出する（[`derive_scale`]・
    ///   component 不在は `None` のまま渡して k=1.0 への縮退へ落とす）。k はキー要素ではない: 合成入力
    ///   （surface id＋bind 集合＋pattern）が完全一致するヒットなら再合成しないし、k だけが変わった適用も
    ///   ヒットする。
    /// - ミスなら合成する（native 原寸）。`SurfaceNotFound` は `error!` ＋表示不変＋`Err`、
    ///   `EmptyComposition` は `warn!` ＋ Hide 縮退＋`Ok`（外形の上限越えの `ExtentTooLarge` も同じ扱い）。
    ///   `Ok` なら原寸バイトから表示記録（閉じた `GraphicsCommandList`）を起こし、マスクを原寸バイトから
    ///   1 回だけ生成して `cache.insert` へ面・マスク・記録の三つ組で渡す。
    /// - `mount` 未生成ならエントリの表示記録込みで遅延生成し、表示記録 → 配置（原寸・k）→ マスク →
    ///   可視化の順に同一呼び出し内で書く。拡大は wintf の描画経路（`render_surface` の `SetTransform`）が
    ///   `Arrangement.scale`＝k で掛けるので、この漏斗は k の値で手順を分岐しない。
    ///
    /// # 毎フレーム経路は再利用席の上を走る
    ///
    /// ミス経路のバッファ（native 合成先・表示バッファ・当たり判定マスク）はいずれも [`FrameBudget`] の
    /// 席か、キャッシュ追い出しエントリの容量回収で回る。定常状態（寸法不変・初回確保後）ではこの
    /// 経路の新規確保は 0 であり、確保が起きた回だけが perf サマリ行の `alloc_*` に現れる。
    ///
    /// [`FrameBudget`]: super::budget::FrameBudget
    ///
    /// # 可視化手順だけが所有権でゲートされる
    ///
    /// [`VisibilityOwnership::External`] の target では、末尾の可視化（`set_visible(true)`／`visible=true`）
    /// と mount 遅延生成の初期可視性だけが変わり、それ以外の全手順は所有権に依らず共通（k 導出・合成・
    /// キャッシュ・表示記録・配置・マスク・`applied`／`native_size`／`last_show`／`pending_resize`・
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

        // 引き当ては合成入力（id＋binds＋pattern）の完全一致のみヒット。k はキーに参加しない: 面・マスク・
        // 表示記録はいずれも原寸で k を含まないので、k だけが変わった適用は同じエントリにヒットし、変換の
        // 係数（下の `set_layout`）だけが変わる。pattern は指令が運ぶ現在コマ集合をそのまま透過する。
        //
        // `touch` であって `get` ではない（容量 3 の LRU）。ここが 1 適用に 1 回の引き当て点＝最近使用順を
        // 動かす唯一の場所。`get`（順序を動かさない読み取り）へ替えると置換は挿入順（FIFO）へ静かに退化
        // するが、表示バイトも確保計数も変わらないので、その退化は `presenter_cache_capacity_tests.rs` だけが
        // 捕まえる。
        let cache_hit = target.cache.touch(surface_id, &binds, &pattern);
        timing.mark(Stage::CacheLookup);
        // 合成の仕方（控えに外れた回だけ意味を持つ・重ね直しの範囲の行が読む）。
        let mut recompose = Recompose::Full;
        if !cache_hit {
            // 下敷きの印は合成の**前に**偽にする。この適用が合成の後で失敗して戻っても、
            // 合成器の前の命令列（`base_ops`）は控えの前の絵と食い違っているので、次は必ずいちからになる。
            let partial_base = std::mem::replace(&mut target.partial_base, false);

            // 合成先は [`FrameBudget`] の常設席で、`compose_into` が容量を再利用するので外形の変わらない
            // 反復では 1 バイトも確保しない。席を閉包で借りるのは、合成外形が `compose_into` の内側で決まり、
            // 伸長の観測を席を使い終えた直後にしか置けないため（`FrameBudget::native_scratch` の doc）。
            //
            // 合成結果を先に束縛するのは、計時点 `Stage::Compose` を合成の直後に置くため。
            //
            // 動きの無い面（`pattern` が空）はいちから合成する。動きのある面は下敷き形の口を呼び、印が真で
            // 前に成立した面の絵が控えに在る（`get`＝順序を動かさない）ときだけその絵を下敷きに渡す。
            // 印が真なら合成器の前の命令列はその絵を作った命令列である。
            let base = if partial_base {
                target
                    .last_show
                    .as_ref()
                    .and_then(|(id, b, p)| target.cache.get(*id, b, p))
                    .map(|e| &e.composed)
            } else {
                None
            };
            let composed = target.budget.native_scratch(|scratch| {
                // 合成は常に native 原寸（emo-compose は k を知らない）。外形は席そのものが持ち、
                // エントリの `composed` 外形として下流が読む。
                if pattern.is_empty() {
                    target
                        .composer
                        .compose_into(
                            scratch,
                            &target.emo_world,
                            &target.atlas,
                            surface_id,
                            &binds,
                            &pattern,
                        )
                        .map(|()| Recompose::Full)
                } else {
                    target.composer.compose_into_reusing(
                        scratch,
                        base,
                        &target.emo_world,
                        &target.atlas,
                        surface_id,
                        &binds,
                        &pattern,
                    )
                }
            });
            timing.mark(Stage::Compose);
            match composed {
                Ok(how) => {
                    recompose = how;
                    // 表示の記録: 合成した原寸バイトから閉じたコマンドリストを起こし、下の `insert` で
                    // エントリへ束ねる。記録は k を含まない。
                    //
                    // 失敗し得る GPU 呼び出しはここに集約し、回収（`take_recycled`）より手前に置くことで失敗時の
                    // 前状態を保つ: ここで失敗すればメモ・表示記録・配置・マスク・可視性・`applied`／
                    // `native_size`／`last_show`／`pending_resize` の 1 つも動いていない。記録元は合成先席
                    // そのもの（下の交代より前なので、まだ原寸が入っている）。
                    let display_list = {
                        let Some(dc) = world
                            .get_resource::<GraphicsCore>()
                            .and_then(|gfx| gfx.device_context())
                        else {
                            // 同じ原因の続きは 2 回目から DEBUG。原因は返す `Device` の値と同じ。
                            const NO_GFX: (&str, i32) = ("GraphicsCore resource", 0);
                            if target.failure_note == Some(NO_GFX) {
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
                            target.failure_note = Some(NO_GFX);
                            return Err(PresentError::Device {
                                hresult: NO_GFX.1,
                                context: NO_GFX.0,
                            });
                        };
                        // 席は読むだけだが、貸し出し口は 1 つしかない（`native_scratch`）。伸長は
                        // 起きないため確保の計数も動かない。
                        let repeat = target.failure_note;
                        match target
                            .budget
                            .native_scratch(|scratch| record_display(dc, scratch, repeat))
                        {
                            Ok(list) => list,
                            // record_display は内部で記録済み（command.rs の device_err・同じ原因の続きは debug!）。
                            Err(e) => {
                                if let PresentError::Device { hresult, context } = &e {
                                    target.failure_note = Some((*context, *hresult));
                                }
                                return Err(e);
                            }
                        }
                    };
                    // `Stage::Upload` の区間＝原寸 D2D bitmap の生成＋描画命令の記録＋Close。
                    timing.mark(Stage::Upload);
                    // 容量回収は合成・記録の成功後に限る（`take_recycled` の契約）。失敗し得る位置で呼ぶと
                    // スロットが空のまま残り、直後の同一入力の適用がヒットせず再合成へ落ちて「失敗時は表示も
                    // キャッシュも適用前のまま」が崩れる。
                    let recycled = target.cache.take_recycled();
                    // 追い出しエントリの表示バッファ容量を受け取り、束ねられていたマスクは輪番の空きスロットとして
                    // 下の `regenerate_mask` へ回す。回収が成立しなければ空バッファから始まり、以後の伸長が
                    // 計数される。
                    let (mut display, retired_mask) =
                        super::budget::FrameBudget::display_buffer(recycled);
                    // 合成先席と表示バッファを交代させる（複写も確保も起きない）。k の値に依らず常にこの経路で、
                    // 交代後は回収した容量が合成先席に入り、次の適用はそちらへ合成する。
                    target.budget.swap_native_scratch(&mut display);
                    // マスクはこの適用で 1 回だけ原寸バイトから生成する。生成点は `cache.insert` の外＝予算シームの
                    // 輪番で、`retired_mask`（直前の適用で表示に使ったマスク）が次の空きスロットへ回り、代わりに
                    // 前々回のマスク（下流が既に手放して単独所有）が再生成先として取り出される。挿入は表示バッファと
                    // マスクを同時に受け取るので対は崩れない。÷k は wintf の `alpha_mask_hit` が物理寸の境界に
                    // 対する比例写像で 1 回だけ掛ける。
                    let mask = target.budget.regenerate_mask(
                        retired_mask,
                        display.bytes(),
                        display.width(),
                        display.height(),
                        display.stride(),
                    );
                    // 原寸の外形は `composed` そのものが持つ（別フィールドで二重に持たない）。
                    target.cache.insert(
                        surface_id,
                        binds.clone(),
                        pattern.clone(),
                        display,
                        mask,
                        display_list,
                    );
                    // `Stage::MaskGen` の区間はマスク生成＋挿入全体（スロット置換）と席の交代（O(1) の入れ替え）を
                    // 含む。旧エントリは `take_recycled` で先に回収済みなので、対の解放 churn はこの区間に無い。
                    timing.mark(Stage::MaskGen);
                }
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

        // 使えるエントリ（引き当て済み・順序は動かさない `get`）。原寸はエントリの `composed` 外形、物理寸は
        // 丸め権威の式 1 つ（`scaled_extent`）で導く。照会値（`target_physical_size`）と同じ式で同じ値になる。
        let entry = target
            .cache
            .get(surface_id, &binds, &pattern)
            .expect("直前に引き当て済み");
        let native = (entry.composed.width(), entry.composed.height());
        let physical = scale.scaled_extent(native.0, native.1);

        // 装着の遅延生成（初回表示・原寸確定後）。surface entity はエントリの表示記録込みで
        // spawn する（純 ECS・失敗経路なし）。`WucGraphicsResource`／`Compositor` は要らない。
        if target.mount.is_none() {
            // 初期可視性は所有権から導出する。`CommandDriven` は可視で構築（この漏斗の末尾で
            // `set_visible(true)` する経路と同値）、`External` は不可視で構築する。「可視で spawn してから
            // 消す」経路を持たないことで、可視状態を component レベルでも一度も経由しないことを構造で
            // 保証する。導出をここで行う（装着側の既定に委ねない）のは、可視性の所有者を知っているのが
            // この層だけだから。
            let initially_visible = matches!(target.ownership, VisibilityOwnership::CommandDriven);
            target.mount = Some(VisualMount::attach(
                world,
                window,
                native,
                scale,
                &entry.display,
                initially_visible,
            ));
        }

        // 表示記録 → 配置 → マスク同期 → 可視化（同一呼び出し内＝原子入替）。可視化手順だけが所有権で
        // ゲートされる: `External` の target では表示状態の確立（合成・表示記録・配置・マスク・k・面 id・
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
        // 表示記録: 値が異なるときだけ挿す（同一エントリの再適用で `Changed` を立てない）。
        mount.set_display(world, &entry.display);
        // 配置（原寸・k）: 同値なら書かない。bounds（＝αマスク座標基準・物理寸）は wintf が
        // `GlobalArrangement` へ導く。可視性に依らず常に合わせる——不可視中の更新は安全であり
        // （`HitTest::none()` 中はマスクが判定に使われない）、ここで揃えておくことが「不可視期間中に
        // 生じた変化を取りこぼさない」の土台になる。
        mount.set_layout(world, native, scale);
        target.shown_mask = Some(entry.mask.clone());
        if let Some(mut mask_res) = world.get_mut::<AlphaMaskResource>(mount.surface_entity()) {
            // 表示記録と同一の原寸 bytes 由来のマスクを hit-test へ供給する。供給は共有参照の受け渡し
            // （`set_shared`）＝`Arc` の参照カウント増のみで、実体の複製は起きない。
            mask_res.set_shared(entry.mask.clone());
        } else {
            tracing::warn!(
                ?target_id,
                entity = ?mount.surface_entity(),
                "apply(ShowSurface): surface entity に AlphaMaskResource が無い（当たり判定は矩形/前状態）"
            );
        }
        if visualize {
            mount.set_visible(world, true);
            target.visible = true;
        }
        // 可視化後の記録。表示記録・配置・マスクを揃え終えた点＝「描画内容がこの寸で見えるようになった」
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
        // いま表示に使ったエントリ由来の原寸をそのまま写す。合成した回か否かで分岐させると、「insert 済みの
        // まま失敗 → 後からヒットで成立」の経路で照会値が画面と乖離する。容量 3 ではヒットしたエントリが
        // 直前の挿入とは限らないので、エントリの `composed` 外形から読む。
        target.native_size = Some(native);
        // 合成キーの安定ハッシュ（perf サマリ行の `key_hash`）は `last_show` への move の直前に取る。全段の
        // `mark` が済んだ後なので、この走査（借用のみ・確保なし）は段別所要へ混入せず `t_total_us` にだけ含まれる。
        let key_hash = compose_key_hash(surface_id, &binds, &pattern);
        // 下敷きの印: 控えに外れ、動きのある面（下敷き形の口で合成した）ときだけ真。
        // このとき合成器の前の命令列は、いま控えへ入れた `last_show` の鍵の絵を作った命令列である。
        target.partial_base = !cache_hit && !pattern.is_empty();
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
        // 重ね直しの範囲の行: 控えに外れて成立した適用だけ、表示の行の直前に 1 行。表示の行と perf の行の
        // あいだには何も挟まない。
        if !cache_hit {
            let (kind, (rect_x, rect_y, rect_w, rect_h)) = match recompose {
                Recompose::Full => ("full", (0, 0, native.0, native.1)),
                Recompose::Partial { x, y, w, h } => ("partial", (x, y, w, h)),
            };
            tracing::debug!(
                ?target_id,
                surface_id,
                recompose = kind,
                rect_x,
                rect_y,
                rect_w,
                rect_h,
                "apply(ShowSurface): 重ね直しの範囲"
            );
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
            },
            allocs,
        );
        Ok(())
    }
}
