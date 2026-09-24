//! 台本と差し替え（design §SwapDriver）。World を書く唯一の書き手。
//!
//! 同じ system を `Update`（配置が決まる前）と `FrameFinalize`（画面への反映の後・tick の記録より前）の
//! 2 段に登録し、台本がどの tick のどの段で動くかを決める。
//!
//! 観測の前には必ず「用意」（観測しない）を置き、窓の `emo-*` 子を全部消して出発の面だけを装着し直し、
//! 最新のフレームの絵と直前の tick の当たり判定・矩形が出発の面に揃うまで待つ。design は「各観測の後に
//! A へ戻す」だが、B→A と A2→A0 は出発が A でないので、戻し先を観測の出発の面にした（戻しと同じ手で
//! B0・A2 へ用意する）。

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info};
use windows::Win32::System::Performance::QueryPerformanceCounter;

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_emo_present::{DEFAULT_AUTHOR_DPI, EmoPresenter, PresentCommand, TargetId};
use wintf::ecs::{FrameCount, GraphicsCore, SizeI, WindowPos, WucGraphicsResource};

use crate::observe::{self, Class, Kind, Observer, PAIR_ASSET, PAIR_FACE};

/// 用意の後、揃ったと見るまでに最低限待つ tick 数（用意の tick の絵が取り込みに届くまで・約 40 ms）。
const PREP_MIN_TICKS: u32 = 5;
/// 用意が揃うのを待つ上限（design §SwapDriver の 60 tick）。
const PREP_GIVE_UP_TICKS: u32 = 60;
/// 用意と差し替えの基準の版が使う表示先 id。
const BASE_TARGET: TargetId = TargetId(1);

// ---------------------------------------------------------------------------
// 台本の語彙
// ---------------------------------------------------------------------------

/// A＝StayseeBalloon・B＝emo2-kakukaku。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Balloon {
    A,
    B,
}

impl Balloon {
    fn face(self) -> Face {
        match self {
            Balloon::A => Face::A0,
            Balloon::B => Face::B0,
        }
    }
}

/// 判別対の要素（A0・A2＝Staysee の面 0・面 2、B0＝kakukaku の面 0）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    A0,
    A2,
    B0,
}

impl Face {
    fn balloon(self) -> Balloon {
        match self {
            Face::A0 | Face::A2 => Balloon::A,
            Face::B0 => Balloon::B,
        }
    }
    fn surface(self) -> u32 {
        match self {
            Face::A0 | Face::B0 => 0,
            Face::A2 => 2,
        }
    }
    /// 判別対の中での P／Q（A0 は両対で P）。
    fn class(self) -> Class {
        match self {
            Face::A0 => Class::P,
            Face::A2 | Face::B0 => Class::Q,
        }
    }
}

/// 差し替えの 3 版（5.1・5.2・5.3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Reattach,
    RemoveThenAttach,
    AttachNewHideOld,
}

impl Method {
    fn name(self) -> &'static str {
        match self {
            Method::Reattach => "reattach",
            Method::RemoveThenAttach => "remove-then-attach",
            Method::AttachNewHideOld => "attach-new-hide-old",
        }
    }
}

/// 差し替えの時点（system を登録した段）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Update,
    FrameFinalize,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::Update => "update",
            Stage::FrameFinalize => "finalize",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// GPU 資源を待って最初の表示（A0）をし、揃うのを待つ。
    Boot,
    /// 観測しない用意（戻し）: 子を全部消して `face` を装着し、`pair` で揃うのを待つ。
    Prep { face: Face, pair: usize },
    Swap {
        method: Method,
        stage: Stage,
        from: Balloon,
        to: Balloon,
    },
    /// `\b[ID]` 相当（同じ装着の面の切り替え・`Update` だけ）。
    FaceSwitch { from: Face, to: Face },
    /// 最終の並べ出しをして窓を消す。
    Done,
}

/// 固定の台本（5.4: 基準の結果に依らず全版を観測する）。
pub fn script() -> Vec<Step> {
    let mut steps = vec![Step::Boot];
    for method in [
        Method::Reattach,
        Method::RemoveThenAttach,
        Method::AttachNewHideOld,
    ] {
        for stage in [Stage::Update, Stage::FrameFinalize] {
            if steps.len() > 1 {
                steps.push(Step::Prep {
                    face: Face::A0,
                    pair: PAIR_ASSET,
                });
            }
            for (from, to) in [(Balloon::A, Balloon::B), (Balloon::B, Balloon::A)] {
                if from == Balloon::B {
                    steps.push(Step::Prep {
                        face: Face::B0,
                        pair: PAIR_ASSET,
                    });
                }
                steps.push(Step::Swap {
                    method,
                    stage,
                    from,
                    to,
                });
            }
        }
    }
    for (from, to) in [(Face::A0, Face::A2), (Face::A2, Face::A0)] {
        steps.push(Step::Prep {
            face: from,
            pair: PAIR_FACE,
        });
        steps.push(Step::FaceSwitch { from, to });
    }
    steps.push(Step::Done);
    steps
}

/// 観測の行の名前（design §Data Models）。観測しない段は `None`。
pub fn obs_name(step: Step) -> Option<String> {
    match step {
        Step::Swap {
            method,
            stage,
            from,
            to,
        } => Some(format!(
            "{}@{} {from:?}→{to:?}",
            method.name(),
            stage.name()
        )),
        Step::FaceSwitch { from, to } => Some(format!("face-switch@update {from:?}→{to:?}")),
        _ => None,
    }
}

/// その段が動く時点。
pub fn act_stage(step: Step) -> Stage {
    match step {
        Step::Swap { stage, .. } => stage,
        Step::Done => Stage::FrameFinalize,
        Step::Boot | Step::Prep { .. } | Step::FaceSwitch { .. } => Stage::Update,
    }
}

/// 台本が装着に使う資産の組の数（A, B）。`EmoWorld` は `Clone` でないので起動時に作り置く。
pub fn needs(steps: &[Step]) -> (usize, usize) {
    let mut n = (0, 0);
    let mut add = |b: Balloon| match b {
        Balloon::A => n.0 += 1,
        Balloon::B => n.1 += 1,
    };
    for s in steps {
        match *s {
            Step::Boot => add(Balloon::A),
            Step::Prep { face, .. } => add(face.balloon()),
            Step::Swap { to, .. } => add(to),
            Step::FaceSwitch { .. } | Step::Done => {}
        }
    }
    n
}

// ---------------------------------------------------------------------------
// 書き手
// ---------------------------------------------------------------------------

type Set = (EmoWorld, AtlasTable);

#[derive(Clone, Copy, Debug)]
enum Phase {
    /// 今の段の仕事を、その段の時点で行う。
    Act,
    /// 仕事を済ませた tick（用意は揃うのを、観測は閉じるのを `Update` で待つ）。
    Wait(u32),
}

/// 台本・presenter・作り置きの資産（NonSend・UI スレッド専有）。
pub struct Driver {
    presenter: EmoPresenter,
    window: Entity,
    pool_a: Vec<Set>,
    pool_b: Vec<Set>,
    steps: Vec<Step>,
    cursor: usize,
    phase: Phase,
    /// 今の表示先 id（隠すだけの版で動く・用意で 1 に戻る）。
    current_id: TargetId,
    /// 隠すだけの版が次に使う id（単調増加）。
    next_id: u32,
    /// 期待する原寸（A0・A2・B0）。
    sizes: [(u32, u32); 3],
}

impl Driver {
    pub fn new(
        window: Entity,
        steps: Vec<Step>,
        pool_a: Vec<Set>,
        pool_b: Vec<Set>,
        sizes: [(u32, u32); 3],
    ) -> Self {
        Self {
            presenter: EmoPresenter::new(),
            window,
            pool_a,
            pool_b,
            steps,
            cursor: 0,
            phase: Phase::Act,
            current_id: BASE_TARGET,
            next_id: BASE_TARGET.0 + 1,
            sizes,
        }
    }

    fn size(&self, face: Face) -> (u32, u32) {
        self.sizes[face as usize]
    }

    fn step(&mut self, world: &mut World, stage: Stage) {
        let now = world.resource::<FrameCount>().0;
        if let Phase::Wait(since) = self.phase {
            if stage != Stage::Update || !self.finished(world, since, now) {
                return;
            }
            self.cursor += 1;
            self.phase = Phase::Act;
        }
        let Some(&step) = self.steps.get(self.cursor) else {
            return;
        };
        if act_stage(step) != stage {
            return;
        }
        match step {
            Step::Boot => {
                let ready = world.get_resource::<GraphicsCore>().is_some()
                    && world
                        .get_resource::<WucGraphicsResource>()
                        .is_some_and(|r| r.is_valid());
                if !ready {
                    return;
                }
                self.prep(world, Face::A0, PAIR_ASSET, now);
            }
            Step::Prep { face, pair } => self.prep(world, face, pair, now),
            Step::Swap {
                method,
                stage,
                from,
                to,
            } => self.swap(world, step, method, stage, from, to, now),
            Step::FaceSwitch { from, to } => self.face_switch(world, step, from, to, now),
            Step::Done => return self.done(world, now),
        }
        self.phase = Phase::Wait(now);
    }

    /// 今の段が済んだか（`Update` で調べる）。
    fn finished(&self, world: &World, since: u32, now: u32) -> bool {
        match self.steps[self.cursor] {
            Step::Boot => self.prep_settled(world, Face::A0, PAIR_ASSET, since, now),
            Step::Prep { face, pair } => self.prep_settled(world, face, pair, since, now),
            Step::Swap { .. } | Step::FaceSwitch { .. } => world.resource::<Observer>().is_closed(),
            Step::Done => false,
        }
    }

    /// 用意（観測しない）: 子を全部消して `face` だけを `BASE_TARGET` に装着し直し、窓寸を合わせる。
    fn prep(&mut self, world: &mut World, face: Face, pair: usize, now: u32) {
        world.resource_mut::<Observer>().active = pair;
        let removed = despawn_mounts(world, self.window);
        self.current_id = BASE_TARGET;
        let r = self
            .attach(world, BASE_TARGET, face.balloon())
            .and_then(|()| self.show(world, BASE_TARGET, face.surface()));
        self.fit(world, BASE_TARGET);
        match r {
            Ok(()) => info!(tick = now, ?face, removed, "用意（観測しない）"),
            Err(e) => error!(tick = now, ?face, removed, error = %e, "用意に失敗"),
        }
    }

    /// 用意が揃った: 最新のフレームの絵・直前の tick の当たり判定が `face`・矩形が原寸。
    /// 同じ絵へ戻る用意では新しいフレームが来ないことがあるので「最新のフレーム」で見る（design）。
    fn prep_settled(&self, world: &World, face: Face, pair: usize, since: u32, now: u32) -> bool {
        if now < since + PREP_MIN_TICKS {
            return false;
        }
        let obs = world.resource::<Observer>();
        let s = observe::lock(&obs.shared);
        let want = face.class();
        let pic = s.frames.last().map(|f| f.picture);
        let tick = s.ticks.last().copied();
        let ok = pic == Some(want)
            && tick.is_some_and(|t| {
                t.tick > since
                    && t.pair == pair
                    && t.hit == want
                    && observe::rect_size(&t.rect) == self.size(face)
            });
        if ok {
            debug!(tick = now, ?face, waited = now - since, "用意が揃った");
            return true;
        }
        if now >= since + PREP_GIVE_UP_TICKS {
            error!(
                tick = now,
                ?face,
                ?pic,
                hit = ?tick.map(|t| t.hit),
                rect = ?tick.map(|t| t.rect),
                "用意が {PREP_GIVE_UP_TICKS} tick で揃わない — 次へ進む（次の観測の直前のフレームに写る）"
            );
            return true;
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn swap(
        &mut self,
        world: &mut World,
        step: Step,
        method: Method,
        stage: Stage,
        from: Balloon,
        to: Balloon,
        now: u32,
    ) {
        let name = obs_name(step).expect("差し替えは観測");
        info!(
            tick = now,
            qpc = qpc(),
            version = method.name(),
            stage = stage.name(),
            ?from,
            ?to,
            "swap: 要求"
        );
        world.resource_mut::<Observer>().open(
            PAIR_ASSET,
            Kind::Swap,
            &name,
            from.face().class(),
            to.face().class(),
            now,
        );
        let r = match method {
            // 基準（5.1）: 同じ id へ再登録するだけ（古い子は残る）。
            Method::Reattach => self
                .attach(world, self.current_id, to)
                .and_then(|()| self.show(world, self.current_id, 0)),
            // 本命（5.2）: 窓のバルーンの子を名前で探して消し、同じ呼び出しの中で再登録・表示（5.7）。
            Method::RemoveThenAttach => {
                let removed = despawn_mounts(world, self.window);
                self.attach(world, self.current_id, to)
                    .and_then(|()| self.show(world, self.current_id, 0))
                    .and_then(|()| match removed {
                        2 => Ok(()),
                        n => Err(format!("消した子が 2 でなく {n}")),
                    })
            }
            // 隠すだけ（5.3）: 別の id で装着・表示し、古い id を隠す。
            Method::AttachNewHideOld => {
                let (old, new) = (self.current_id, TargetId(self.next_id));
                self.next_id += 1;
                self.current_id = new;
                self.attach(world, new, to)
                    .and_then(|()| self.show(world, new, 0))
                    .map(|()| self.hide(world, old))
            }
        };
        self.fit(world, self.current_id);
        self.after_change(&name, to.face());
        if let Err(e) = r {
            error!(%name, error = %e, "差し替えに失敗 — この観測は測れない");
            world.resource_mut::<Observer>().abort("差し替えに失敗");
        }
    }

    fn face_switch(&mut self, world: &mut World, step: Step, from: Face, to: Face, now: u32) {
        let name = obs_name(step).expect("面の切り替えは観測");
        info!(
            tick = now,
            qpc = qpc(),
            version = "face-switch",
            stage = Stage::Update.name(),
            ?from,
            ?to,
            "swap: 要求"
        );
        world.resource_mut::<Observer>().open(
            PAIR_FACE,
            Kind::FaceSwitch,
            &name,
            from.class(),
            to.class(),
            now,
        );
        let r = self.show(world, self.current_id, to.surface());
        self.fit(world, self.current_id);
        self.after_change(&name, to);
        if let Err(e) = r {
            error!(%name, error = %e, "面の切り替えに失敗 — この観測は測れない");
            world.resource_mut::<Observer>().abort("面の切り替えに失敗");
        }
    }

    /// 台本の終わり（6.4）: 最終の並べ出し → 終了の理由 → 窓を消す（`run()` が戻る）。
    fn done(&mut self, world: &mut World, now: u32) {
        let verdict = world.resource_mut::<Observer>().summarize(false, now);
        let reason = crate::completed_reason(&verdict);
        world.resource_mut::<crate::Run>().exit = Some(reason);
        info!(
            ?reason,
            "終了: 台本を終えた（上限時間を待たない）— 窓を消す"
        );
        world.despawn(self.window);
        self.cursor += 1;
    }

    /// 差し替えの直後の今の面と物理寸を、到達先の原寸と比べる（design §SwapDriver Validation）。
    fn after_change(&self, name: &str, to: Face) {
        let id = self.current_id;
        let surface = self.presenter.current_surface_id(id);
        let size = self.presenter.target_physical_size(id);
        let want = self.size(to);
        debug!(%name, ?id, ?surface, ?size, ?want, "差し替えの直後の今の面と物理寸");
        if surface != Some(to.surface()) || size != Some(want) {
            error!(%name, ?id, ?surface, ?size, ?want, "差し替えの直後の面・物理寸が到達先と違う");
        }
    }

    fn attach(&mut self, world: &mut World, id: TargetId, b: Balloon) -> Result<(), String> {
        let pool = match b {
            Balloon::A => &mut self.pool_a,
            Balloon::B => &mut self.pool_b,
        };
        let (w, a) = pool
            .pop()
            .ok_or_else(|| format!("{b:?} の作り置きの資産が尽きた"))?;
        self.presenter
            .attach_target(world, id, self.window, w, a, DEFAULT_AUTHOR_DPI)
            .map_err(|e| format!("attach_target({id:?}, {b:?}) が失敗: {e}"))
    }

    /// `ShowSurface`。`apply` は戻り値を持たないので、今の面が要求どおりになったかで成否を見る。
    fn show(&mut self, world: &mut World, id: TargetId, surface_id: u32) -> Result<(), String> {
        self.presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target: id,
                surface_id,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: None,
            },
        );
        match self.presenter.current_surface_id(id) {
            Some(s) if s == surface_id => Ok(()),
            got => Err(format!(
                "ShowSurface({id:?}, {surface_id}) の後の今の面が {got:?}"
            )),
        }
    }

    fn hide(&mut self, world: &mut World, id: TargetId) {
        self.presenter.apply(
            world,
            PresentCommand::Hide {
                target: id,
                reply: None,
            },
        );
    }

    /// `take_pending_resize` → `WindowPos` で窓寸を絵に合わせる（位置は固定のまま）。
    fn fit(&mut self, world: &mut World, id: TargetId) {
        let Some((w, h)) = self.presenter.take_pending_resize(id) else {
            return;
        };
        let size = SizeI {
            width: w as i32,
            height: h as i32,
        };
        match world.get_mut::<WindowPos>(self.window) {
            Some(mut wp) => {
                if wp.size != Some(size) {
                    wp.size = Some(size);
                    debug!(w, h, "窓寸を絵に合わせた");
                }
            }
            None => error!(window = ?self.window, "窓に WindowPos が無い — 窓寸を合わせられない"),
        }
    }
}

/// 窓の子のうち present の装着（`emo-surface`・`emo-text-layer-slot`）を消す。消した数を返す。
///
/// presenter の内部の物を外から消している（5.7）。本坑では present に正規の片付けの口が要る。
fn despawn_mounts(world: &mut World, window: Entity) -> usize {
    let mounts: Vec<Entity> = world
        .get::<Children>(window)
        .map(|c| {
            c.iter()
                .filter(|&e| {
                    world.get::<Name>(e).is_some_and(|n| {
                        matches!(n.as_str(), "emo-surface" | "emo-text-layer-slot")
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    for &e in &mounts {
        world.despawn(e);
    }
    mounts.len()
}

fn qpc() -> i64 {
    let mut v = 0i64;
    // SAFETY: 出力先はローカル変数。失敗は 0 のまま（ログの参考値）。
    let _ = unsafe { QueryPerformanceCounter(&mut v) };
    v
}

/// 登録用: 段を閉じ込めた排他 system。上限時間で終わった後は何もしない。
pub fn swap_system_for(stage: Stage) -> impl FnMut(&mut World) {
    move |world: &mut World| {
        if world.resource::<crate::Run>().exit.is_some() {
            return;
        }
        let Some(mut d) = world.remove_non_send::<Driver>() else {
            return;
        };
        d.step(world, stage);
        world.insert_non_send(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_observes_six_versions_both_ways_then_two_face_switches() {
        let steps = script();
        let names: Vec<String> = steps.iter().filter_map(|s| obs_name(*s)).collect();
        let mut want = Vec::new();
        for m in ["reattach", "remove-then-attach", "attach-new-hide-old"] {
            for st in ["update", "finalize"] {
                want.push(format!("{m}@{st} A→B"));
                want.push(format!("{m}@{st} B→A"));
            }
        }
        want.push("face-switch@update A0→A2".to_string());
        want.push("face-switch@update A2→A0".to_string());
        assert_eq!(names, want);
        assert_eq!(steps.first(), Some(&Step::Boot));
        assert_eq!(steps.last(), Some(&Step::Done));
    }

    #[test]
    fn every_observation_starts_from_a_settled_prep_of_its_from_face() {
        let steps = script();
        for w in steps.windows(2) {
            let start = match w[1] {
                Step::Swap { from, .. } => from.face(),
                Step::FaceSwitch { from, .. } => from,
                _ => continue,
            };
            let pair = if matches!(w[1], Step::FaceSwitch { .. }) {
                PAIR_FACE
            } else {
                PAIR_ASSET
            };
            match w[0] {
                Step::Boot => assert_eq!((start, pair), (Face::A0, PAIR_ASSET)),
                Step::Prep { face, pair: p } => assert_eq!((face, p), (start, pair), "{:?}", w[1]),
                other => panic!("観測の前が戻しでない: {other:?} → {:?}", w[1]),
            }
        }
    }

    #[test]
    fn each_step_acts_at_its_stage() {
        let swap = |stage| Step::Swap {
            method: Method::Reattach,
            stage,
            from: Balloon::A,
            to: Balloon::B,
        };
        assert_eq!(act_stage(swap(Stage::Update)), Stage::Update);
        assert_eq!(act_stage(swap(Stage::FrameFinalize)), Stage::FrameFinalize);
        let face = Step::FaceSwitch {
            from: Face::A0,
            to: Face::A2,
        };
        assert_eq!(act_stage(face), Stage::Update);
        assert_eq!(act_stage(Step::Boot), Stage::Update);
        assert_eq!(
            act_stage(Step::Prep {
                face: Face::B0,
                pair: PAIR_ASSET
            }),
            Stage::Update
        );
        assert_eq!(act_stage(Step::Done), Stage::FrameFinalize);
    }

    #[test]
    fn prebuilt_asset_sets_cover_every_attach_in_the_script() {
        // A: 起動 1・B→A 6・版の間の戻し A0 5・面の用意 A0／A2 2。B: A→B 6・用意 B0 6。
        assert_eq!(needs(&script()), (14, 12));
    }
}
