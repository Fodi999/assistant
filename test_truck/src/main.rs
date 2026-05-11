use truck_modeling::*;
use truck_meshalgo::tessellation::*;
use std::fs::File;

fn main() {
    let p0 = builder::vertex(Point3::new(-1.0, -1.0, -1.0));
    let p1 = builder::vertex(Point3::new(1.0, -1.0, -1.0));
    let wire = builder::line(&p0, &p1);
    let face = builder::tsweep(&wire, Vector3::new(0.0, 2.0, 0.0));
    let solid = builder::tsweep(&face, Vector3::new(0.0, 0.0, 2.0));
    
    let mut mesh = solid.triangulation(0.01).to_polygon();
}
