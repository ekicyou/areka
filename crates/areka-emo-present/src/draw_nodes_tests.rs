use super::*;

#[test]
fn each_mix_maps_to_its_d2d_composite_mode() {
    let table = [
        (Mix::SourceOver, D2D1_COMPOSITE_MODE_SOURCE_OVER),
        (Mix::SourceAtop, D2D1_COMPOSITE_MODE_SOURCE_ATOP),
        (Mix::DestinationOver, D2D1_COMPOSITE_MODE_DESTINATION_OVER),
        (Mix::Copy, D2D1_COMPOSITE_MODE_SOURCE_COPY),
        (Mix::DestinationIn, D2D1_COMPOSITE_MODE_DESTINATION_IN),
    ];
    for (mix, mode) in table {
        assert_eq!(mix_mode_of(mix), mode, "{mix:?}");
    }
}
