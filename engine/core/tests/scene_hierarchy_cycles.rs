use hezhou_core::Scene;

#[test]
fn scene_rejects_self_and_ancestor_cycles() {
    let mut scene = Scene::new();
    let a = scene.create_entity();
    let b = scene.create_entity();
    let c = scene.create_entity();

    scene.set_parent(b, Some(a));
    scene.set_parent(c, Some(b));

    // 自环
    scene.set_parent(a, Some(a));
    assert_eq!(scene.get_parent(a), None);

    // c 的父是 b，b 的父是 a；把 a 挂到 c 下会形成 a→c→b→a。
    scene.set_parent(a, Some(c));
    assert_eq!(scene.get_parent(a), None);

    // 合法层级保持不变
    assert_eq!(scene.get_parent(b), Some(a));
    assert_eq!(scene.get_parent(c), Some(b));
}
