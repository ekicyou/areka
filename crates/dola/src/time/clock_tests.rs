use super::*;

fn os_ticks() -> i64 {
    // SAFETY: 引数もポインタも渡さない読み取りで、呼ぶ側に前提が無い。
    unsafe { QueryUnbiasedInterruptTimePrecise() as i64 }
}

#[test]
fn consecutive_reads_never_go_back() {
    let mut last = now();
    for _ in 0..1000 {
        let next = now();
        assert!(next >= last, "{next:?} < {last:?}");
        last = next;
    }
}

#[test]
fn now_is_on_the_os_scale() {
    let before = os_ticks();
    let t = now().as_ticks();
    let after = os_ticks();
    assert!(before <= t && t <= after, "{before} <= {t} <= {after}");
}
