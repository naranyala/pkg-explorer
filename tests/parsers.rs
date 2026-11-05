use crate::sources::pacman;

#[test]
fn pacman_parser_fixtures() {
    let rows = pacman::parse_package_list(include_str!("fixtures/pacman-q.txt"));
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].name, "base");
    assert_eq!(rows[1].name, "filesystem");
    assert_eq!(rows[2].version, "1:1.3.1-2");
}
