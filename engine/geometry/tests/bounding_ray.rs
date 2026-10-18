use hezhou_geometry::BoundingBox;
use nalgebra::Vector3;

#[test]
fn ray_hits_aabb_and_misses_outside() {
    let bbox = BoundingBox::new(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(1.0, 1.0, 1.0));
    let t = bbox.intersects_ray(&Vector3::new(0.0, 0.0, 5.0), &Vector3::new(0.0, 0.0, -1.0));
    assert!(t.is_some());
    assert!((t.unwrap() - 4.0).abs() < 0.001);
    assert!(bbox.intersects_ray(&Vector3::new(0.0, 5.0, 5.0), &Vector3::new(0.0, 0.0, -1.0)).is_none());
}

#[test]
fn ray_inside_starts_at_zero() {
    let bbox = BoundingBox::new(Vector3::new(-1.0, -1.0, -1.0), Vector3::new(1.0, 1.0, 1.0));
    let t = bbox.intersects_ray(&Vector3::new(0.0, 0.0, 0.0), &Vector3::new(0.0, 0.0, 1.0));
    assert_eq!(t, Some(0.0));
}
