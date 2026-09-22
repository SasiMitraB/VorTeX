use gpui::AssetSource;
use vortex::icons::{Assets, IconName};

#[test]
fn every_embedded_icon_is_an_svg() {
    let paths = Assets.list("icons/").unwrap();
    assert!(!paths.is_empty());
    for path in paths {
        let bytes = Assets.load(&path).unwrap().expect("listed icon should load");
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.trim_start().starts_with("<svg"), "{path} is not an SVG");
        assert!(text.contains("viewBox"), "{path} has no viewBox");
    }
}

#[test]
fn icon_names_resolve_to_embedded_files() {
    for name in [IconName::Folder, IconName::FileCode, IconName::Play, IconName::X] {
        assert!(Assets.load(name.path()).unwrap().is_some(), "{:?} missing", name);
    }
    assert!(Assets.load("icons/does-not-exist.svg").unwrap().is_none());
}
