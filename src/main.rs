use macroquad::prelude::*;

const W: i32 = 32;
const H: i32 = 16;
const D: i32 = 32;

struct World {
    blocks: Vec<u8>,
}

impl World {
    fn new() -> Self {
        let mut blocks = vec![0u8; (W * H * D) as usize];
        for x in 0..W {
            for z in 0..D {
                let h = (6.0 + (x as f32 * 0.3).sin() * 2.0 + (z as f32 * 0.22).cos() * 2.0) as i32;
                for y in 0..h.min(H) {
                    let t = if y == h - 1 { 1 } else if y >= h - 4 { 2 } else { 3 };
                    blocks[Self::idx(x, y, z)] = t;
                }
            }
        }
        World { blocks }
    }

    fn idx(x: i32, y: i32, z: i32) -> usize {
        (x + W * (y + H * z)) as usize
    }

    fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || y < 0 || z < 0 || x >= W || y >= H || z >= D {
            return 0;
        }
        self.blocks[Self::idx(x, y, z)]
    }

    fn set(&mut self, x: i32, y: i32, z: i32, t: u8) {
        if x < 0 || y < 0 || z < 0 || x >= W || y >= H || z >= D {
            return;
        }
        self.blocks[Self::idx(x, y, z)] = t;
    }

    fn solid_at(&self, p: Vec3) -> bool {
        self.get(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32) != 0
    }

    fn exposed(&self, x: i32, y: i32, z: i32) -> bool {
        let n = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
        n.iter().any(|(dx, dy, dz)| {
            let (nx, ny, nz) = (x + dx, y + dy, z + dz);
            if ny >= H {
                return true;
            }
            if nx < 0 || nz < 0 || nx >= W || nz >= D || ny < 0 {
                return false;
            }
            self.get(nx, ny, nz) == 0
        })
    }

    fn raycast(&self, origin: Vec3, dir: Vec3) -> Option<(IVec3, IVec3)> {
        let mut prev = origin.floor().as_ivec3();
        let mut t = 0.0;
        while t < 6.0 {
            let c = (origin + dir * t).floor().as_ivec3();
            if self.get(c.x, c.y, c.z) != 0 {
                return Some((c, prev));
            }
            prev = c;
            t += 0.02;
        }
        None
    }
}

fn color(t: u8) -> Color {
    match t {
        1 => Color::new(0.30, 0.65, 0.25, 1.0),
        2 => Color::new(0.50, 0.35, 0.20, 1.0),
        _ => Color::new(0.50, 0.50, 0.52, 1.0),
    }
}

fn window_conf() -> Conf {
    Conf { window_title: "RustCraft".to_owned(), ..Default::default() }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();
    let mut pos = vec3(16.0, 14.0, 16.0);
    let mut vel_y = 0.0f32;
    let mut on_ground = false;
    let (mut yaw, mut pitch) = (0.0f32, 0.0f32);
    let mut selected: u8 = 1;
    let mut last_mouse: Vec2 = mouse_position().into();
    set_cursor_grab(true);
    show_mouse(false);

    loop {
        let dt = get_frame_time().min(0.05);
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        // Muis kijken
        let m: Vec2 = mouse_position().into();
        let delta = m - last_mouse;
        last_mouse = m;
        yaw += delta.x * 0.003;
        pitch = (pitch - delta.y * 0.003).clamp(-1.5, 1.5);

        let front = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos());
        let flat = vec3(yaw.cos(), 0.0, yaw.sin());
        let right = flat.cross(Vec3::Y).normalize();

        // Bewegen
        let mut mv = Vec3::ZERO;
        if is_key_down(KeyCode::W) { mv += flat; }
        if is_key_down(KeyCode::S) { mv -= flat; }
        if is_key_down(KeyCode::D) { mv += right; }
        if is_key_down(KeyCode::A) { mv -= right; }
        if mv.length() > 0.0 {
            let step = mv.normalize() * 5.0 * dt;
            for axis in [vec3(step.x, 0.0, 0.0), vec3(0.0, 0.0, step.z)] {
                let np = pos + axis;
                let blocked = [0.1, 0.9, 1.7].iter().any(|h| world.solid_at(vec3(np.x, pos.y + h, np.z)));
                if !blocked { pos = np; }
            }
        }

        // Zwaartekracht en springen
        if is_key_pressed(KeyCode::Space) && on_ground {
            vel_y = 8.0;
        }
        vel_y -= 25.0 * dt;
        let ny = pos.y + vel_y * dt;
        on_ground = false;
        if vel_y < 0.0 && world.solid_at(vec3(pos.x, ny, pos.z)) {
            pos.y = ny.floor() + 1.0;
            vel_y = 0.0;
            on_ground = true;
        } else if vel_y > 0.0 && world.solid_at(vec3(pos.x, ny + 1.8, pos.z)) {
            vel_y = 0.0;
        } else {
            pos.y = ny;
        }
        if pos.y < -10.0 {
            pos = vec3(16.0, 14.0, 16.0);
            vel_y = 0.0;
        }

        // Blokken kiezen, breken, plaatsen
        if is_key_pressed(KeyCode::Key1) { selected = 1; }
        if is_key_pressed(KeyCode::Key2) { selected = 2; }
        if is_key_pressed(KeyCode::Key3) { selected = 3; }

        let eye = pos + vec3(0.0, 1.6, 0.0);
        let hit = world.raycast(eye, front);
        if let Some((target, before)) = hit {
            if is_mouse_button_pressed(MouseButton::Left) {
                world.set(target.x, target.y, target.z, 0);
            }
            if is_mouse_button_pressed(MouseButton::Right) {
                let feet = pos.floor().as_ivec3();
                if before != feet && before != feet + IVec3::Y {
                    world.set(before.x, before.y, before.z, selected);
                }
            }
        }

        // Tekenen
        clear_background(Color::new(0.5, 0.75, 1.0, 1.0));
        set_camera(&Camera3D {
            position: eye,
            target: eye + front,
            up: Vec3::Y,
            ..Default::default()
        });
        for x in 0..W {
            for y in 0..H {
                for z in 0..D {
                    let t = world.get(x, y, z);
                    if t != 0 && world.exposed(x, y, z) {
                        let c = vec3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                        draw_cube(c, Vec3::ONE, None, color(t));
                        draw_cube_wires(c, Vec3::ONE, Color::new(0.0, 0.0, 0.0, 0.25));
                    }
                }
            }
        }
        if let Some((t, _)) = hit {
            let c = vec3(t.x as f32 + 0.5, t.y as f32 + 0.5, t.z as f32 + 0.5);
            draw_cube_wires(c, Vec3::splat(1.01), WHITE);
        }

        set_default_camera();
        let (cx, cy) = (screen_width() / 2.0, screen_height() / 2.0);
        draw_line(cx - 8.0, cy, cx + 8.0, cy, 2.0, WHITE);
        draw_line(cx, cy - 8.0, cx, cy + 8.0, 2.0, WHITE);
        draw_rectangle(10.0, 10.0, 28.0, 28.0, color(selected));
        draw_text("WASD lopen | Spatie springen | Links breken | Rechts plaatsen | 1-3 blok | Esc stoppen", 50.0, 30.0, 22.0, WHITE);

        next_frame().await;
    }
}
