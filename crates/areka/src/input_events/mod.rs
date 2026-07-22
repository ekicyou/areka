//! UI→kanade のマウス入力配信配線（areka-P0-input-events）。
//!
//! キャラ窓のポインタイベントを捉え、当たり判定名を collision-geometry の resolver で解決し、
//! 送出間引き（[`throttle`]）を通して kanade へマウス入力メッセージとして配信する薄い配線層。
//!
//! 本モジュールは現状 [`throttle`]（送出間引きの純粋・決定的判定・task 2.4）のみを収める。
//! per-scope 間引き状態を `HashMap` で保持する `MouseWiring` とポインタハンドラ結線は
//! task 2.6／2.7 で本 mod へ増設される。

pub(crate) mod throttle;

use std::collections::HashMap;
use std::sync::mpsc::Sender;

use areka_emo_present::EmoPresenter;
use areka_kanade::{KanadeMsg, MouseButton, MouseEventKind, MouseInput};

use crate::emo2_boot::hit_region::{HitRegion, resolve_hit_region};
use throttle::{MouseMoveThrottle, plan_mouse_move};

/// UI スレッド所有のマウス入力配信資源（NonSend・DD-IE-9）。
///
/// kanade への投函端（[`Sender`] クローン・1.4）・per-scope 間引き状態・当たり判定 resolver の
/// 供給源（実／mock の差し替えシーム・[`RegionSource`]・1.5）・注入可能な時刻源（`now_ms`）を
/// 1 資源に束ねる。[`Sender`] 単体は `Send` だが、resolver（presenter 読み）と間引き状態が
/// UI スレッド所有ゆえ NonSend 1 個に束ねる（`Emo2Wiring` 前例と同型・順序依存なし self-gating）。
///
/// 本 struct と送出ヘルパは task 2.6 の範囲。ポインタハンドラ（`on_char_pointer_moved` /
/// `on_char_pointer_pressed`）と暫定退避（Ctrl+左ダブルクリック）は task 2.7、`wire_mouse_input`
/// による World 挿入（main.rs 呼出）は task 3.1 で本 mod へ増設される。
///
/// `#[allow(dead_code)]`: 送出ヘルパ群は task 2.7 のハンドラ／3.1 の配線が消費するまで非テスト
/// コードから未参照（throttle.rs が collision-geometry の第一消費者まで携えたのと同型）。
#[allow(dead_code)]
pub(crate) struct MouseWiring {
    /// `GhostRuntime::kanade()` クローン（1.4・std mpsc）。
    sender: Sender<KanadeMsg>,
    /// per-scope 間引き状態（scope→状態）。
    throttle: HashMap<u32, MouseMoveThrottle>,
    /// 実／mock の差し替えシーム（1.5）。
    region_source: RegionSource,
    /// 注入可能 clock（既定: 起動からの経過 ms・単調）。
    now_ms: Box<dyn FnMut() -> u64>,
}

/// 当たり判定名の供給源シーム（実／mock）。
///
/// `#[allow(dead_code)]`: `Presenter` variant の実運用消費は task 2.7 のハンドラ／3.1 の配線
/// （`RegionSource::Presenter` を構築）まで非テストコードから未参照。
#[allow(dead_code)]
pub(crate) enum RegionSource {
    /// 実運用: presenter で `resolve_hit_region` を呼ぶ（1.3）。
    Presenter,
    /// 決定論檻: 固定写像で `HitRegion` を返す（1.5）。
    Mock(fn(u32, i64, i64) -> HitRegion),
}

#[allow(dead_code)]
impl MouseWiring {
    /// 実運用の構築子（既定 clock＝構築時に捕捉した [`Instant`] からの経過 ms）。
    ///
    /// NOTE: 既定 clock の構築は純粋テスト経路の外に置く（テストは [`with_clock`] で決定的 clock を
    /// 注入する）。`Instant::now()` を読むためユニット檻からは使わない。
    ///
    /// [`Instant`]: std::time::Instant
    /// [`with_clock`]: MouseWiring::with_clock
    pub(crate) fn new(sender: Sender<KanadeMsg>, region_source: RegionSource) -> Self {
        let start = std::time::Instant::now();
        Self {
            sender,
            throttle: HashMap::new(),
            region_source,
            now_ms: Box::new(move || start.elapsed().as_millis() as u64),
        }
    }

    /// テスト用の構築子（決定的 clock を注入する・純粋檻用）。
    #[cfg(test)]
    fn with_clock(
        sender: Sender<KanadeMsg>,
        region_source: RegionSource,
        now_ms: Box<dyn FnMut() -> u64>,
    ) -> Self {
        Self {
            sender,
            throttle: HashMap::new(),
            region_source,
            now_ms,
        }
    }

    /// (scope, 窓 client 物理 px) → 当たり判定名（DD-IE-10・座標は素通し＝DPI 変換なし）。
    ///
    /// - [`RegionSource::Mock`] → `f(scope, x, y)`（presenter を無視・1.5）。
    /// - [`RegionSource::Presenter`] → `Some(p)` なら `resolve_hit_region(p, scope, x, y)`（1.3）。
    ///   `presenter` 不在（`Emo2Wiring` 未挿入＝boot 前／失敗時）は `HitRegion { scope, region: None }`
    ///   へ正常縮退する（collision-geometry design の消費想定どおり・trace）。
    ///
    /// 座標 `x`/`y` は当該 shell 窓の client 物理 px であり、そのまま resolver へ渡す（DPI 変換しない・
    /// k=1.0 契約＝collision-geometry 4.3 を継承・DD-IE-10）。
    fn resolve_region(
        &self,
        presenter: Option<&EmoPresenter>,
        scope: u32,
        x: i64,
        y: i64,
    ) -> HitRegion {
        match self.region_source {
            RegionSource::Mock(f) => f(scope, x, y),
            RegionSource::Presenter => match presenter {
                Some(p) => resolve_hit_region(p, scope, x, y),
                None => {
                    tracing::trace!(
                        event = "mouse_region_degrade",
                        scope,
                        "Emo2Wiring 不在（boot 前／失敗時）: region None へ正常縮退"
                    );
                    HitRegion { scope, region: None }
                }
            },
        }
    }

    /// per-scope 間引き判定を通し、送出条件成立時のみ `OnMouseMove` 相当を送出する（5.1・DD-IE-5）。
    ///
    /// per-scope の [`MouseMoveThrottle`] を引き（無ければ既定生成）、[`plan_mouse_move`] で
    /// (次状態, 送出可否) を求めて次状態を保存し、送出可否が true のときだけ
    /// `KanadeMsg::Mouse(MouseInput { .., kind: Move })` を [`Sender`] へ送る。
    ///
    /// 座標 `x`/`y` は窓 client 物理 px（DD-IE-10・素通し）。返り値は実際に送出したか。
    /// 送出失敗（kanade 停止後の [`Sender`] エラー）は warn＋no-op（false 返し・log-first）。
    fn plan_and_send_move(
        &mut self,
        scope: u32,
        x: i64,
        y: i64,
        region: Option<String>,
    ) -> bool {
        let now = (self.now_ms)();
        let state = self.throttle.entry(scope).or_default();
        let (next, send) = plan_mouse_move(state, (x, y), &region, now);
        *state = next;

        if !send {
            return false;
        }

        let msg = KanadeMsg::Mouse(MouseInput {
            scope,
            x,
            y,
            region,
            kind: MouseEventKind::Move,
        });
        if self.sender.send(msg).is_err() {
            tracing::warn!(
                event = "mouse_send_failed",
                scope,
                kind = "move",
                "kanade Sender 送出失敗（actor 停止後）: no-op で継続"
            );
            return false;
        }
        true
    }

    /// `OnMouseDoubleClick` 相当を無条件送出する（間引きなし・1.2/3.x）。
    ///
    /// クリックは間引き対象外ゆえ throttle を通さず即送出する。座標 `x`/`y` は窓 client 物理 px
    /// （DD-IE-10・素通し）。送出失敗は warn＋no-op（log-first）。
    fn send_double_click(
        &mut self,
        scope: u32,
        x: i64,
        y: i64,
        region: Option<String>,
        button: MouseButton,
    ) {
        let msg = KanadeMsg::Mouse(MouseInput {
            scope,
            x,
            y,
            region,
            kind: MouseEventKind::DoubleClick { button },
        });
        if self.sender.send(msg).is_err() {
            tracing::warn!(
                event = "mouse_send_failed",
                scope,
                kind = "double_click",
                "kanade Sender 送出失敗（actor 停止後）: no-op で継続"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_kanade::{KanadeMsg, MouseButton, MouseEventKind};
    use crate::emo2_boot::hit_region::HitRegion;
    use std::sync::mpsc;

    /// 単調増加する注入 clock を作る（毎呼出で +step ms）。
    fn stepping_clock(start: u64, step: u64) -> Box<dyn FnMut() -> u64> {
        let mut t = start;
        Box::new(move || {
            let now = t;
            t += step;
            now
        })
    }

    /// RED（mock-seam send・1.5）: Mock resolver＋注入 clock で単体から配信し、受信側で内容を観測する。
    #[test]
    fn mock_seam_plan_and_send_move_observed() {
        let (tx, rx) = mpsc::channel::<KanadeMsg>();
        let mut wiring = MouseWiring::with_clock(
            tx,
            RegionSource::Mock(|_, _, _| HitRegion {
                scope: 0,
                region: Some("Head".to_string()),
            }),
            stepping_clock(1000, 1000),
        );

        // Mock は presenter を無視して固定写像を返す。
        let hit = wiring.resolve_region(None, 0, 10, 20);
        assert_eq!(hit.region, Some("Head".to_string()), "Mock は固定 region を返す");

        // 初回送出（moved=first_send）。
        let sent = wiring.plan_and_send_move(0, 10, 20, hit.region.clone());
        assert!(sent, "初回移動は送出される");

        let msg = rx.try_recv().expect("KanadeMsg が届くべき");
        match msg {
            KanadeMsg::Mouse(m) => {
                assert_eq!(m.scope, 0);
                assert_eq!(m.x, 10);
                assert_eq!(m.y, 20);
                assert_eq!(m.region, Some("Head".to_string()));
                assert_eq!(m.kind, MouseEventKind::Move);
            }
            _ => panic!("Mouse(Move) を期待"),
        }
        assert!(rx.try_recv().is_err(), "1 件のみ送出されるべき");
    }

    /// 間引き統合（5.1）: 同一 pos 再送は hover 抑制で送出されない（moved=false）。
    #[test]
    fn throttle_suppresses_same_position_hover() {
        let (tx, rx) = mpsc::channel::<KanadeMsg>();
        let mut wiring = MouseWiring::with_clock(
            tx,
            RegionSource::Mock(|_, _, _| HitRegion {
                scope: 0,
                region: Some("Head".to_string()),
            }),
            // 大きく進む clock でも位置不変なら送出されないことを見る。
            stepping_clock(1000, 10_000),
        );

        // 初回は送出。
        assert!(wiring.plan_and_send_move(0, 10, 20, Some("Head".to_string())));
        rx.try_recv().expect("初回は届く");

        // 同一 pos（moved=false）: 間隔が幾ら経っても hover 抑制で送出しない。
        assert!(
            !wiring.plan_and_send_move(0, 10, 20, Some("Head".to_string())),
            "同一 pos は送出しない（hover 抑制）"
        );
        assert!(rx.try_recv().is_err(), "抑制時は何も届かない");
    }

    /// 間引き統合（5.1）: 移動＋同一 region＋間隔未経過は抑制され、何も送出されない。
    #[test]
    fn throttle_suppresses_move_same_region_within_interval() {
        let (tx, rx) = mpsc::channel::<KanadeMsg>();
        // clock: 1回目=1000, 2回目=1050（+50ms < 100ms 間隔）。
        let mut wiring = MouseWiring::with_clock(
            tx,
            RegionSource::Mock(|_, _, _| HitRegion {
                scope: 0,
                region: Some("Head".to_string()),
            }),
            stepping_clock(1000, 50),
        );

        // 初回送出（now=1000）。
        assert!(wiring.plan_and_send_move(0, 10, 20, Some("Head".to_string())));
        rx.try_recv().expect("初回は届く");

        // 移動・同一 region・間隔未経過（now=1050, delta=50 < 100）: 抑制。
        assert!(
            !wiring.plan_and_send_move(0, 11, 20, Some("Head".to_string())),
            "移動＋同一 region＋間隔未経過は抑制"
        );
        assert!(rx.try_recv().is_err(), "抑制時は何も届かない");
    }

    /// 左ダブルクリックは間引きなしで無条件送出され、内容（kind=DoubleClick{Left}）が観測できる（1.2/3.3）。
    #[test]
    fn double_click_left_sends_unconditionally() {
        let (tx, rx) = mpsc::channel::<KanadeMsg>();
        let mut wiring = MouseWiring::with_clock(
            tx,
            RegionSource::Mock(|_, _, _| HitRegion { scope: 0, region: None }),
            stepping_clock(1000, 1000),
        );

        wiring.send_double_click(0, 5, 6, Some("Head".to_string()), MouseButton::Left);
        match rx.try_recv().expect("dblclick が届くべき") {
            KanadeMsg::Mouse(m) => {
                assert_eq!(m.scope, 0);
                assert_eq!(m.x, 5);
                assert_eq!(m.y, 6);
                assert_eq!(m.region, Some("Head".to_string()));
                assert_eq!(m.kind, MouseEventKind::DoubleClick { button: MouseButton::Left });
            }
            _ => panic!("Mouse(DoubleClick) を期待"),
        }

        // クリックは throttle を通さない: 同一座標で連続送出しても届く。
        wiring.send_double_click(0, 5, 6, Some("Head".to_string()), MouseButton::Left);
        assert!(rx.try_recv().is_ok(), "クリックは間引かれず 2 回目も届く");
    }

    /// 右ダブルクリックも同様に無条件送出され、button=Right が観測できる（3.3）。
    #[test]
    fn double_click_right_sends_with_right_button() {
        let (tx, rx) = mpsc::channel::<KanadeMsg>();
        let mut wiring = MouseWiring::with_clock(
            tx,
            RegionSource::Mock(|_, _, _| HitRegion { scope: 0, region: None }),
            stepping_clock(1000, 1000),
        );

        wiring.send_double_click(1, 7, 8, None, MouseButton::Right);
        match rx.try_recv().expect("dblclick が届くべき") {
            KanadeMsg::Mouse(m) => {
                assert_eq!(m.scope, 1);
                assert_eq!(m.x, 7);
                assert_eq!(m.y, 8);
                assert_eq!(m.region, None);
                assert_eq!(m.kind, MouseEventKind::DoubleClick { button: MouseButton::Right });
            }
            _ => panic!("Mouse(DoubleClick) を期待"),
        }
    }

    /// presenter 不在の正常縮退（1.3・DD-IE-9）: `RegionSource::Presenter` で presenter=None なら
    /// `HitRegion { region: None }` を返し panic しない。
    #[test]
    fn presenter_absent_degrades_to_region_none() {
        let (tx, _rx) = mpsc::channel::<KanadeMsg>();
        let wiring = MouseWiring::with_clock(
            tx,
            RegionSource::Presenter,
            stepping_clock(1000, 1000),
        );

        let hit = wiring.resolve_region(None, 3, 100, 200);
        assert_eq!(
            hit,
            HitRegion { scope: 3, region: None },
            "Emo2Wiring 不在は region None へ正常縮退（scope はそのまま反映）"
        );
    }
}
