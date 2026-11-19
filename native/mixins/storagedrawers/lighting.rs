// AO weights and light blending follow Horizon 9f42064 mesher::lighting,
// which implements Minecraft BlockModelLighter. World samples are supplied
// by the adapter so a framed block can supply Java's dynamic shade brightness.
use horizon_assets::model::Direction as D;
use horizon_client::models::catalog::QuadShape;
#[derive(Clone,Copy)]
pub(super) struct Sample{pub light:u32,pub shade:f32,pub permeable:bool,pub solid:bool}
fn normal(face:D)->[i32;3]{match face{D::Down=>[0,-1,0],D::Up=>[0,1,0],D::North=>[0,0,-1],D::South=>[0,0,1],D::West=>[-1,0,0],D::East=>[1,0,0]}}
fn add(a:[i32;3],b:[i32;3])->[i32;3]{std::array::from_fn(|i|a[i]+b[i])}
pub(super) fn smooth_blend(mut a: u32, mut b: u32, mut c: u32, center: u32) -> u32 {
    if center >> 20 & 15 > 2 || center >> 4 & 15 > 2 {
        for n in [&mut a, &mut b, &mut c] {
            if *n == 0 {
                *n = center;
            } else if *n >> 20 & 15 == 0 {
                *n |= center & 0x00FF_0000;
            }
        }
    }
    (a + b + c + center) >> 2 & 0x00FF_00FF
}
/// `LightCoordsUtil.smoothWeightedBlend` (Java float sums and int narrowing).
fn weighted_blend(c: [u32; 4], w: [f32; 4]) -> u32 {
    let channel = |shift: u32| {
        let v = |i: usize| (c[i] >> shift & 0xFF) as f32 * w[i];
        (v(0) + v(1) + v(2) + v(3)) as i32 as u32 & 0xFF
    };
    channel(0) | channel(16) << 16
}
// SizeInfo indices into the face shape.
const DOWN: u8 = 0;
const UP: u8 = 1;
const NORTH: u8 = 2;
const SOUTH: u8 = 3;
const WEST: u8 = 4;
const EAST: u8 = 5;
const FLIP_DOWN: u8 = 6;
const FLIP_UP: u8 = 7;
const FLIP_NORTH: u8 = 8;
const FLIP_SOUTH: u8 = 9;
const FLIP_WEST: u8 = 10;
const FLIP_EAST: u8 = 11;
struct Adjacency {
    corners: [D; 4],
    /// Face-shape factor pairs for each vertex's four corner weights.
    weights: [[u8; 8]; 4],
    /// Output vertex for each of the four computed corners.
    remap: [usize; 4],
}
const fn adjacency(face: D) -> Adjacency {
    use D::*;
    match face {
        Down => Adjacency {
            corners: [West, East, North, South],
            weights: [
                [
                    FLIP_WEST, SOUTH, FLIP_WEST, FLIP_SOUTH, WEST, FLIP_SOUTH, WEST, SOUTH,
                ],
                [
                    FLIP_WEST, NORTH, FLIP_WEST, FLIP_NORTH, WEST, FLIP_NORTH, WEST, NORTH,
                ],
                [
                    FLIP_EAST, NORTH, FLIP_EAST, FLIP_NORTH, EAST, FLIP_NORTH, EAST, NORTH,
                ],
                [
                    FLIP_EAST, SOUTH, FLIP_EAST, FLIP_SOUTH, EAST, FLIP_SOUTH, EAST, SOUTH,
                ],
            ],
            remap: [0, 1, 2, 3],
        },
        Up => Adjacency {
            corners: [East, West, North, South],
            weights: [
                [
                    EAST, SOUTH, EAST, FLIP_SOUTH, FLIP_EAST, FLIP_SOUTH, FLIP_EAST, SOUTH,
                ],
                [
                    EAST, NORTH, EAST, FLIP_NORTH, FLIP_EAST, FLIP_NORTH, FLIP_EAST, NORTH,
                ],
                [
                    WEST, NORTH, WEST, FLIP_NORTH, FLIP_WEST, FLIP_NORTH, FLIP_WEST, NORTH,
                ],
                [
                    WEST, SOUTH, WEST, FLIP_SOUTH, FLIP_WEST, FLIP_SOUTH, FLIP_WEST, SOUTH,
                ],
            ],
            remap: [2, 3, 0, 1],
        },
        North => Adjacency {
            corners: [Up, Down, East, West],
            weights: [
                [UP, FLIP_WEST, UP, WEST, FLIP_UP, WEST, FLIP_UP, FLIP_WEST],
                [UP, FLIP_EAST, UP, EAST, FLIP_UP, EAST, FLIP_UP, FLIP_EAST],
                [
                    DOWN, FLIP_EAST, DOWN, EAST, FLIP_DOWN, EAST, FLIP_DOWN, FLIP_EAST,
                ],
                [
                    DOWN, FLIP_WEST, DOWN, WEST, FLIP_DOWN, WEST, FLIP_DOWN, FLIP_WEST,
                ],
            ],
            remap: [3, 0, 1, 2],
        },
        South => Adjacency {
            corners: [West, East, Down, Up],
            weights: [
                [UP, FLIP_WEST, FLIP_UP, FLIP_WEST, FLIP_UP, WEST, UP, WEST],
                [
                    DOWN, FLIP_WEST, FLIP_DOWN, FLIP_WEST, FLIP_DOWN, WEST, DOWN, WEST,
                ],
                [
                    DOWN, FLIP_EAST, FLIP_DOWN, FLIP_EAST, FLIP_DOWN, EAST, DOWN, EAST,
                ],
                [UP, FLIP_EAST, FLIP_UP, FLIP_EAST, FLIP_UP, EAST, UP, EAST],
            ],
            remap: [0, 1, 2, 3],
        },
        West => Adjacency {
            corners: [Up, Down, North, South],
            weights: [
                [
                    UP, SOUTH, UP, FLIP_SOUTH, FLIP_UP, FLIP_SOUTH, FLIP_UP, SOUTH,
                ],
                [
                    UP, NORTH, UP, FLIP_NORTH, FLIP_UP, FLIP_NORTH, FLIP_UP, NORTH,
                ],
                [
                    DOWN, NORTH, DOWN, FLIP_NORTH, FLIP_DOWN, FLIP_NORTH, FLIP_DOWN, NORTH,
                ],
                [
                    DOWN, SOUTH, DOWN, FLIP_SOUTH, FLIP_DOWN, FLIP_SOUTH, FLIP_DOWN, SOUTH,
                ],
            ],
            remap: [3, 0, 1, 2],
        },
        East => Adjacency {
            corners: [Down, Up, North, South],
            weights: [
                [
                    FLIP_DOWN, SOUTH, FLIP_DOWN, FLIP_SOUTH, DOWN, FLIP_SOUTH, DOWN, SOUTH,
                ],
                [
                    FLIP_DOWN, NORTH, FLIP_DOWN, FLIP_NORTH, DOWN, FLIP_NORTH, DOWN, NORTH,
                ],
                [
                    FLIP_UP, NORTH, FLIP_UP, FLIP_NORTH, UP, FLIP_NORTH, UP, NORTH,
                ],
                [
                    FLIP_UP, SOUTH, FLIP_UP, FLIP_SOUTH, UP, FLIP_SOUTH, UP, SOUTH,
                ],
            ],
            remap: [1, 2, 3, 0],
        },
    }
}
const ADJACENCY: [Adjacency; 6] = [
    adjacency(D::Down),
    adjacency(D::Up),
    adjacency(D::North),
    adjacency(D::South),
    adjacency(D::West),
    adjacency(D::East),
];
pub(super) fn quad(shape:&QuadShape,face:D,own:Sample,sample:impl Fn([i32;3])->Sample)->([f32;4],[u32;4]){
    let direction=normal(face);let adjacency=&ADJACENCY[face as usize];
    let base=if shape.cubic{direction}else{[0;3]};
    let corner=|i:usize|add(base,normal(adjacency.corners[i]));
    let fetch=|p|{let s=sample(p);(s.light,s.shade)};
    let (light0,shade0)=fetch(corner(0));let (light1,shade1)=fetch(corner(1));
    let (light2,shade2)=fetch(corner(2));let (light3,shade3)=fetch(corner(3));
    let permeable=|i|sample(add(corner(i),direction)).permeable;
    let diagonal=|a,b|if permeable(a)||permeable(b){fetch(add(corner(a),normal(adjacency.corners[b])))}else{(light0,shade0)};
    let (light02,shade02)=diagonal(0,2);let (light03,shade03)=diagonal(0,3);
    let (light12,shade12)=diagonal(1,2);let (light13,shade13)=diagonal(1,3);
    let next=sample(direction);
    let light_center=if shape.cubic||!next.solid{next.light}else{own.light};
    let shade_center=if shape.cubic{sample(base).shade}else{own.shade};
    let ao = [
        (shade3 + shade0 + shade03 + shade_center) * 0.25,
        (shade2 + shade0 + shade02 + shade_center) * 0.25,
        (shade2 + shade1 + shade12 + shade_center) * 0.25,
        (shade3 + shade1 + shade13 + shade_center) * 0.25,
    ];
    let blended = [
        smooth_blend(light3, light0, light03, light_center),
        smooth_blend(light2, light0, light02, light_center),
        smooth_blend(light2, light1, light12, light_center),
        smooth_blend(light3, light1, light13, light_center),
    ];
    let mut brightness = [0f32; 4];
    let mut light = [0u32; 4];
    for (k, &vertex) in adjacency.remap.iter().enumerate() {
        if shape.partial {
            let f = |i: usize| shape.bounds[adjacency.weights[k][i] as usize];
            let w = [f(0) * f(1), f(2) * f(3), f(4) * f(5), f(6) * f(7)];
            brightness[vertex] =
                (ao[0] * w[0] + ao[1] * w[1] + ao[2] * w[2] + ao[3] * w[3]).clamp(0.0, 1.0);
            light[vertex] = weighted_blend(blended, w);
        } else {
            brightness[vertex] = ao[k];
            light[vertex] = blended[k];
        }
    }
    (brightness, light)
}
