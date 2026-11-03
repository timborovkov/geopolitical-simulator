use sim_core::*;
#[test]
fn strict_gregorian_input_roundtrips_the_campaign_calendar() {
    for day in (0..=MAX_DAY).step_by(17) {
        assert_eq!(parse_date(&date(day)).unwrap(), day);
    }
    assert_eq!(parse_date("2000-02-29").unwrap(), 59);
    assert_eq!(parse_date("2100-01-01").unwrap(), MAX_DAY);
    for bad in [
        "2001-02-29",
        "2000-04-31",
        "2000-00-01",
        "1999-12-31",
        "2100-01-02",
        "2000-1-01",
        "2000-01-00",
        "éééé-01-01",
        "",
    ] {
        assert!(parse_date(bad).is_err(), "{bad}");
    }
}
