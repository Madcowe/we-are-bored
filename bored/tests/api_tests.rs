use bored::*;
use bored::notice::*;
use bored::url::*;

// ── ProtocolVersion ──

#[test]
fn protocol_version_new_is_latest() {
    let v = ProtocolVersion::new();
    assert_eq!(v.get_version(), 3);
}

#[test]
fn protocol_version_check_valid() {
    for v in [1u64, 2, 3] {
        assert!(ProtocolVersion::check(v).is_ok());
        assert_eq!(ProtocolVersion::check(v).unwrap().get_version(), v);
    }
}

#[test]
fn protocol_version_check_invalid() {
    for v in [0u64, 4, 100, u64::MAX] {
        assert_eq!(ProtocolVersion::check(v), Err(BoredError::InvalidProtocolVersion(v)));
    }
}

#[test]
fn protocol_version_ordering() {
    let v1 = ProtocolVersion::check(1).unwrap();
    let v2 = ProtocolVersion::check(2).unwrap();
    assert!(v1 < v2);
    assert_eq!(v1, v1);
}

// ── Coordinate ──

#[test]
fn coordinate_within() {
    let c = Coordinate { x: 5, y: 5 };
    assert!(c.within(&Coordinate { x: 5, y: 5 }));
    assert!(c.within(&Coordinate { x: 10, y: 10 }));
    assert!(!c.within(&Coordinate { x: 4, y: 5 }));
    assert!(!c.within(&Coordinate { x: 5, y: 4 }));
}

#[test]
fn coordinate_within_zero() {
    let zero = Coordinate { x: 0, y: 0 };
    assert!(zero.within(&Coordinate { x: 0, y: 0 }));
    assert!(zero.within(&Coordinate { x: 1, y: 1 }));
}

#[test]
fn coordinate_add() {
    let a = Coordinate { x: 3, y: 7 };
    let b = Coordinate { x: 2, y: 3 };
    assert_eq!(a.add(&b), Coordinate { x: 5, y: 10 });
}

#[test]
fn coordinate_subtract_no_underflow() {
    // When self < other on an axis, subtact subtracts 0 (keeps self unchanged)
    let a = Coordinate { x: 5, y: 5 };
    let big = Coordinate { x: 10, y: 10 };
    let result = a.subtact(&big);
    assert_eq!(result, Coordinate { x: 5, y: 5 });
}

#[test]
fn coordinate_subtract_normal() {
    let a = Coordinate { x: 10, y: 20 };
    let b = Coordinate { x: 3, y: 5 };
    assert_eq!(a.subtact(&b), Coordinate { x: 7, y: 15 });
}

#[test]
fn coordinate_add_i32_tuple_positive() {
    let c = Coordinate { x: 5, y: 5 };
    assert_eq!(c.add_i32_tuple((3, 7)), Coordinate { x: 8, y: 12 });
}

#[test]
fn coordinate_add_i32_tuple_negative_clamp() {
    let c = Coordinate { x: 2, y: 3 };
    assert_eq!(c.add_i32_tuple((-10, -10)), Coordinate { x: 0, y: 0 });
}

#[test]
fn coordinate_display() {
    let c = Coordinate { x: 42, y: 99 };
    assert_eq!(format!("{}", c), "x: 42 y: 99");
}

// ── Bored creation & basics ──

#[test]
fn bored_create() {
    let b = Bored::create("Test", Coordinate { x: 80, y: 24 });
    assert_eq!(b.get_name(), "Test");
    assert_eq!(b.get_dimensions(), Coordinate { x: 80, y: 24 });
    assert!(b.get_notices().is_empty());
}

#[test]
fn bored_add_notice_in_bounds() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    let n = Notice::create(Coordinate { x: 10, y: 5 });
    assert!(b.add(n, Coordinate { x: 0, y: 0 }).is_ok());
    assert_eq!(b.get_notices().len(), 1);
}

#[test]
fn bored_add_notice_out_of_bounds() {
    let mut b = Bored::create("B", Coordinate { x: 20, y: 10 });
    let n = Notice::create(Coordinate { x: 10, y: 5 });
    assert!(b.add(n, Coordinate { x: 15, y: 8 }).is_err());
}

#[test]
fn bored_add_notice_exactly_at_edge() {
    let mut b = Bored::create("B", Coordinate { x: 20, y: 10 });
    let n = Notice::create(Coordinate { x: 10, y: 5 });
    assert!(b.add(n, Coordinate { x: 10, y: 5 }).is_ok());
}

#[test]
fn bored_remove_newest() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    b.add(Notice::create(Coordinate { x: 5, y: 5 }), Coordinate { x: 0, y: 0 }).unwrap();
    b.add(Notice::create(Coordinate { x: 5, y: 5 }), Coordinate { x: 10, y: 0 }).unwrap();
    assert_eq!(b.get_notices().len(), 2);
    b.remove_newest_notice();
    assert_eq!(b.get_notices().len(), 1);
}

#[test]
fn bored_remove_oldest() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    let mut n1 = Notice::create(Coordinate { x: 10, y: 5 });
    n1.write("first").unwrap();
    b.add(n1, Coordinate { x: 0, y: 0 }).unwrap();
    let mut n2 = Notice::create(Coordinate { x: 10, y: 5 });
    n2.write("second").unwrap();
    b.add(n2, Coordinate { x: 20, y: 0 }).unwrap();
    b.remove_oldest_notice();
    assert_eq!(b.get_notices().len(), 1);
    assert_eq!(b.get_notices()[0].get_content(), "second");
}

#[test]
fn bored_remove_on_empty() {
    let mut b = Bored::create("B", Coordinate { x: 10, y: 10 });
    b.remove_newest_notice(); // should not panic
    b.remove_oldest_notice();
}

// ── Pruning ──

#[test]
fn prune_removes_fully_occluded() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    let n1 = Notice::create(Coordinate { x: 5, y: 5 });
    b.add(n1, Coordinate { x: 0, y: 0 }).unwrap();
    // Cover n1 entirely
    let n2 = Notice::create(Coordinate { x: 10, y: 10 });
    b.add(n2, Coordinate { x: 0, y: 0 }).unwrap();
    assert_eq!(b.get_notices().len(), 1);
}

#[test]
fn prune_keeps_partially_visible() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    let n1 = Notice::create(Coordinate { x: 10, y: 10 });
    b.add(n1, Coordinate { x: 0, y: 0 }).unwrap();
    let n2 = Notice::create(Coordinate { x: 10, y: 10 });
    b.add(n2, Coordinate { x: 5, y: 5 }).unwrap();
    assert_eq!(b.get_notices().len(), 2);
}

// ── WhatsOnTheBored ──

#[test]
fn whats_on_empty_bored() {
    let b = Bored::create("B", Coordinate { x: 5, y: 5 });
    let w = WhatsOnTheBored::create(&b);
    assert!(w.get_1d().iter().all(|c| c.is_none()));
}

#[test]
fn whats_on_single_notice() {
    let mut b = Bored::create("B", Coordinate { x: 10, y: 10 });
    b.add(Notice::create(Coordinate { x: 3, y: 3 }), Coordinate { x: 0, y: 0 }).unwrap();
    let w = WhatsOnTheBored::create(&b);
    let flat = w.get_1d();
    assert_eq!(flat[0], Some(0));
    assert_eq!(flat[3], None); // x=3 is outside
}

// ── Cardinal navigation ──

#[test]
fn cardinal_notice_none_when_alone() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    b.add(Notice::create(Coordinate { x: 10, y: 10 }), Coordinate { x: 45, y: 20 }).unwrap();
    assert_eq!(b.get_cardinal_notice(0, Direction::Up), None);
    assert_eq!(b.get_cardinal_notice(0, Direction::Down), None);
    assert_eq!(b.get_cardinal_notice(0, Direction::Left), None);
    assert_eq!(b.get_cardinal_notice(0, Direction::Right), None);
}

#[test]
fn cardinal_notice_finds_neighbor() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    b.add(Notice::create(Coordinate { x: 10, y: 10 }), Coordinate { x: 50, y: 20 }).unwrap();
    b.add(Notice::create(Coordinate { x: 10, y: 10 }), Coordinate { x: 0, y: 0 }).unwrap();
    assert!(b.get_cardinal_notice(0, Direction::Up).is_some());
}

// ── Upper-left most ──

#[test]
fn upper_left_most_empty() {
    let b = Bored::create("B", Coordinate { x: 10, y: 10 });
    assert_eq!(b.get_upper_left_most_notice(), None);
}

#[test]
fn upper_left_most_picks_closest_to_origin() {
    let mut b = Bored::create("B", Coordinate { x: 100, y: 50 });
    b.add(Notice::create(Coordinate { x: 5, y: 5 }), Coordinate { x: 50, y: 20 }).unwrap();
    b.add(Notice::create(Coordinate { x: 5, y: 5 }), Coordinate { x: 1, y: 1 }).unwrap();
    assert_eq!(b.get_upper_left_most_notice(), Some(1));
}

// ── Notice ──

#[test]
fn notice_new_defaults() {
    let n = Notice::new();
    assert_eq!(n.get_top_left(), Coordinate { x: 0, y: 0 });
    assert_eq!(n.get_dimensions(), Coordinate { x: 60, y: 18 });
    assert_eq!(n.get_content(), "");
}

#[test]
fn notice_create_custom_size() {
    let n = Notice::create(Coordinate { x: 20, y: 10 });
    assert_eq!(n.get_dimensions(), Coordinate { x: 20, y: 10 });
}

#[test]
fn notice_text_width_height() {
    let n = Notice::create(Coordinate { x: 10, y: 8 });
    assert_eq!(n.get_text_width(), 8);
    assert_eq!(n.get_text_height(), 6);
}

#[test]
fn notice_text_width_tiny() {
    let n = Notice::create(Coordinate { x: 2, y: 2 });
    assert_eq!(n.get_text_width(), 0);
    assert_eq!(n.get_text_height(), 0);
}

#[test]
fn notice_max_chars() {
    let n = Notice::create(Coordinate { x: 3, y: 3 });
    assert_eq!(n.get_max_chars(), 1);
    let n2 = Notice::create(Coordinate { x: 2, y: 2 });
    assert_eq!(n2.get_max_chars(), 0);
}

#[test]
fn notice_max_lines() {
    let n = Notice::create(Coordinate { x: 10, y: 5 });
    assert_eq!(n.get_max_lines(), 3);
}

#[test]
fn notice_write_ok() {
    let mut n = Notice::create(Coordinate { x: 12, y: 3 });
    assert!(n.write("I am BORED").is_ok());
    assert_eq!(n.get_content(), "I am BORED");
}

#[test]
fn notice_write_too_much() {
    let mut n = Notice::create(Coordinate { x: 5, y: 3 });
    assert_eq!(n.write("This is way too much text"), Err(BoredError::TooMuchText));
}

#[test]
fn notice_write_with_hyperlink() {
    let mut n = Notice::new();
    assert!(n.write("Click [here](https://example.com) now").is_ok());
    assert_eq!(n.get_content(), "Click [here](https://example.com) now");
}

#[test]
fn notice_relocate() {
    let b = Bored::create("B", Coordinate { x: 100, y: 50 });
    let mut n = Notice::create(Coordinate { x: 10, y: 5 });
    assert!(n.relocate(&b, Coordinate { x: 5, y: 5 }).is_ok());
    assert_eq!(n.get_top_left(), Coordinate { x: 5, y: 5 });
}

#[test]
fn notice_relocate_out_of_bounds() {
    let b = Bored::create("B", Coordinate { x: 20, y: 10 });
    let mut n = Notice::create(Coordinate { x: 10, y: 5 });
    assert!(n.relocate(&b, Coordinate { x: 15, y: 8 }).is_err());
}

#[test]
fn notice_id_roundtrip() {
    let mut n = Notice::new();
    assert_eq!(n.get_notice_id(), "");
    n.set_notice_id("test-id-123".to_string());
    assert_eq!(n.get_notice_id(), "test-id-123");
}

// ── Hyperlinks ──

#[test]
fn get_hyperlinks_none() {
    let links = get_hyperlinks("No links here").unwrap();
    assert!(links.is_empty());
}

#[test]
fn get_hyperlinks_single() {
    let links = get_hyperlinks("Click [here](http://x.com)").unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].get_text(), "here");
    assert_eq!(links[0].get_link(), "http://x.com");
}

#[test]
fn get_hyperlinks_multiple() {
    let links = get_hyperlinks("[a](u1) and [b](u2)").unwrap();
    assert_eq!(links.len(), 2);
}

#[test]
fn get_display_strips_markdown() {
    let content = "See [link](http://x.com) here";
    let d = get_display(content, get_hyperlinks(content).unwrap());
    assert_eq!(d.get_display_text(), "See link here");
}

#[test]
fn get_display_preserves_non_link_brackets() {
    let content = "a [] () [link](url) b";
    let d = get_display(content, get_hyperlinks(content).unwrap());
    assert_eq!(d.get_display_text(), "a [] () link b");
}

#[test]
fn hyperlink_url_too_long() {
    let long_url = "x".repeat(MAX_URL_LENGTH + 1);
    let result = Hyperlink::create("text", (0, 4), &long_url, (6, 6 + long_url.len()));
    assert_eq!(result, Err(BoredError::URLTooLong));
}

#[test]
fn hyperlink_url_at_max() {
    let url = "x".repeat(MAX_URL_LENGTH);
    assert!(Hyperlink::create("t", (0, 1), &url, (3, 3 + url.len())).is_ok());
}

// ── Display ──

#[test]
fn display_decrement_locations() {
    let d = Display::new();
    d.get_hyperlink_locations(); // just coverage
    assert_eq!(d.get_display_text(), "");
}

// ── remove_tail_link ──

#[test]
fn remove_tail_link_no_link() {
    let mut n = Notice::create(Coordinate { x: 20, y: 5 });
    n.write("hello world").unwrap();
    assert_eq!(n.remove_tail_link().unwrap(), false);
    assert_eq!(n.get_content(), "hello world");
}

#[test]
fn remove_tail_link_with_link() {
    let mut n = Notice::create(Coordinate { x: 40, y: 5 });
    n.write("go [here](http://x.com)").unwrap();
    assert_eq!(n.remove_tail_link().unwrap(), true);
    assert_eq!(n.get_content(), "go ");
}

// ── NoticeHyperlinkMap ──

#[test]
fn notice_hyperlink_map_no_links() {
    let mut n = Notice::create(Coordinate { x: 10, y: 5 });
    n.write("hello").unwrap();
    let map = NoticeHyperlinkMap::create(&n).unwrap();
    let flat: Vec<_> = map.get_map().into_iter().flatten().collect();
    assert!(flat.iter().all(|c| c.is_none()));
}

// ── BoredAddress ──

#[test]
fn bored_address_new_is_topic() {
    let addr = BoredAddress::new();
    let display = format!("{}", addr);
    assert!(display.starts_with("bored://bored."));
}

#[test]
fn bored_address_from_string_with_prefix() {
    let addr = BoredAddress::from_string("bored://bored.test-id").unwrap();
    assert_eq!(addr.get_topic(), "bored.test-id");
}

#[test]
fn bored_address_from_string_without_prefix() {
    let addr = BoredAddress::from_string("bored.my-topic").unwrap();
    assert_eq!(addr.get_topic(), "bored.my-topic");
}

#[test]
fn bored_address_derived_name() {
    let addr = BoredAddress::from_string("genesis").unwrap();
    assert_eq!(addr.get_topic(), "bored.genesis");
}

#[test]
fn bored_address_empty_is_error() {
    assert!(BoredAddress::from_string("").is_err());
}

#[test]
fn bored_address_display_roundtrip() {
    let addr = BoredAddress::from_string("bored.roundtrip").unwrap();
    let s = format!("{}", addr);
    let addr2 = BoredAddress::from_string(&s).unwrap();
    assert_eq!(addr, addr2);
}

// ── URL ──

#[test]
fn url_bored_net() {
    let url = URL::from_string("bored://bored.test".to_string()).unwrap();
    assert_eq!(url, URL::BoredNet(BoredAddress::Topic("bored.test".to_string())));
}

#[test]
fn url_clearnet_https() {
    let url = URL::from_string("https://example.com".to_string()).unwrap();
    assert_eq!(url, URL::ClearNet("https://example.com".to_string()));
}

#[test]
fn url_clearnet_http() {
    let url = URL::from_string("http://example.com".to_string()).unwrap();
    assert_eq!(url, URL::ClearNet("http://example.com".to_string()));
}

#[test]
fn url_bored_app() {
    let url = URL::from_string("app://about".to_string()).unwrap();
    assert_eq!(url, URL::BoredApp("about".to_string()));
}

#[test]
fn url_empty_is_error() {
    assert!(URL::from_string("".to_string()).is_err());
}

#[test]
fn url_short_bored_name() {
    let url = URL::from_string("hi".to_string()).unwrap();
    match url {
        URL::BoredNet(_) => {},
        _ => panic!("Expected BoredNet"),
    }
}

// ── Serialization ──

#[test]
fn bored_json_roundtrip() {
    let mut b = Bored::create("Roundtrip", Coordinate { x: 80, y: 24 });
    let mut n = Notice::create(Coordinate { x: 20, y: 5 });
    n.write("Hello world").unwrap();
    b.add(n, Coordinate { x: 5, y: 3 }).unwrap();
    let json = serde_json::to_string(&b).unwrap();
    let b2: Bored = serde_json::from_str(&json).unwrap();
    assert_eq!(b, b2);
}

#[test]
fn notice_json_roundtrip() {
    let mut n = Notice::create(Coordinate { x: 15, y: 6 });
    n.write("test [link](url)").unwrap();
    let json = serde_json::to_string(&n).unwrap();
    let n2: Notice = serde_json::from_str(&json).unwrap();
    assert_eq!(n, n2);
}

#[test]
fn bored_address_json_roundtrip() {
    let addr = BoredAddress::new();
    let json = serde_json::to_string(&addr).unwrap();
    let addr2: BoredAddress = serde_json::from_str(&json).unwrap();
    assert_eq!(addr, addr2);
}

// ── Error Display ──

#[test]
fn error_display_messages() {
    let e = BoredError::InvalidProtocolVersion(99);
    assert!(format!("{}", e).contains("99"));
    let e = BoredError::TooMuchText;
    assert!(format!("{}", e).contains("Too much text"));
    let e = BoredError::NoBored;
    assert!(!format!("{}", e).is_empty());
}

// ── Edge cases ──

#[test]
fn bored_many_notices() {
    let mut b = Bored::create("Big", Coordinate { x: 200, y: 200 });
    for i in 0..20u16 {
        let n = Notice::create(Coordinate { x: 5, y: 5 });
        b.add(n, Coordinate { x: i * 8, y: 0 }).unwrap();
    }
    assert!(b.get_notices().len() <= 20);
    assert!(b.get_upper_left_most_notice().is_some());
}

#[test]
fn notice_multiline_write() {
    let mut n = Notice::create(Coordinate { x: 20, y: 8 });
    assert!(n.write("Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6").is_ok());
}

#[test]
fn bored_hyperlink_map_basic() {
    let mut b = Bored::create("H", Coordinate { x: 30, y: 15 });
    let mut n = Notice::create(Coordinate { x: 20, y: 5 });
    n.write("[click](http://example.com)").unwrap();
    b.add(n, Coordinate { x: 0, y: 0 }).unwrap();
    let map = BoredHyperlinkMap::create(&b).unwrap();
    let flat: Vec<_> = map.get_map().into_iter().flatten().collect();
    assert!(flat.iter().any(|c| c.is_some()));
}

// ── BoredError conversions ──

#[test]
fn error_from_serde_json() {
    let bad: Result<Bored, _> = serde_json::from_str("not json");
    let e: BoredError = bad.unwrap_err().into();
    match e {
        BoredError::JSONError(_) => {},
        _ => panic!("Expected JSONError"),
    }
}

#[test]
fn error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
    let e: BoredError = io_err.into();
    match e {
        BoredError::IOError(msg) => assert!(msg.contains("gone")),
        _ => panic!("Expected IOError"),
    }
}
