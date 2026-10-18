use hezhou_physics::CharacterController;

#[test]
fn character_falls_under_gravity_and_lands() {
    let mut c = CharacterController::new(0.0, 5.0, 0.0);
    for _ in 0..300 { c.step(1.0 / 60.0, 0.0, 0.0, false, 0.0); }
    assert!((c.position.y - 0.0).abs() < 0.01);
    assert!(c.is_grounded());
}

#[test]
fn character_jumps_and_returns_to_ground() {
    let mut c = CharacterController::new(0.0, 0.0, 0.0);
    c.step(1.0 / 60.0, 0.0, 0.0, false, 0.0);
    c.step(1.0 / 60.0, 0.0, 0.0, true, 0.0);
    assert!(c.position.y > 0.01);
    assert!(!c.is_grounded());
    for _ in 0..300 { c.step(1.0 / 60.0, 0.0, 0.0, false, 0.0); }
    assert!(c.is_grounded());
}

#[test]
fn character_moves_horizontally() {
    let mut c = CharacterController::new(0.0, 0.0, 0.0);
    c.step(0.5, 1.0, 0.0, false, 0.0);
    assert!((c.position.x - 2.5).abs() < 0.01);
}
